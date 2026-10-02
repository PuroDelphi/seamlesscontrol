#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
install_dir=${XDG_BIN_HOME:-"$HOME/.local/bin"}
state_dir=${XDG_STATE_HOME:-"$HOME/.local/state"}/seamlesscontrol
installed_packages_file="$state_dir/installed-packages"
say() {
  if [[ ${SEAMLESSCONTROL_LANG:-en} == es ]]; then printf '%s\n' "$2"; else printf '%s\n' "$1"; fi
}

if [[ ${SEAMLESSCONTROL_INSTALL_DEPS:-1} == 1 ]]; then
  packages=()
  if ! "$cargo_bin" --version >/dev/null 2>&1; then packages+=(rust); fi
  if ! command -v avahi-browse >/dev/null || ! command -v avahi-publish-service >/dev/null; then
    packages+=(avahi)
  fi
  if ! command -v wl-copy >/dev/null || ! command -v wl-paste >/dev/null; then
    packages+=(wl-clipboard)
  fi
  if ((${#packages[@]})); then
    if ! command -v omarchy >/dev/null; then
      say "Missing packages (${packages[*]}) and Omarchy was not found." "Faltan dependencias (${packages[*]}) y no se encontró el comando omarchy." >&2
      exit 1
    fi
    say "Installing Omarchy packages: ${packages[*]}" "Instalando dependencias de Omarchy: ${packages[*]}"
    newly_installed=()
    for package in "${packages[@]}"; do
      if ! pacman -Q "$package" >/dev/null 2>&1; then newly_installed+=("$package"); fi
    done
    omarchy pkg add "${packages[@]}"
    if ((${#newly_installed[@]})); then
      install -d -m 700 "$state_dir"
      touch "$installed_packages_file"
      chmod 600 "$installed_packages_file"
      for package in "${newly_installed[@]}"; do
        if pacman -Q "$package" >/dev/null 2>&1 && ! grep -Fxq "$package" "$installed_packages_file"; then
          printf '%s\n' "$package" >> "$installed_packages_file"
        fi
      done
    fi
  fi
  if ! "$cargo_bin" --version >/dev/null 2>&1 && [[ ${CARGO:-} == "" && -x /usr/bin/cargo ]]; then
    cargo_bin=/usr/bin/cargo
  fi
  if ! "$cargo_bin" --version >/dev/null 2>&1; then
    say 'Cargo is unavailable after installing Rust.' 'Cargo no está disponible después de instalar Rust.' >&2
    exit 1
  fi
  if ! systemctl is-active --quiet avahi-daemon.service; then
    say 'Enabling Avahi to discover computers on the local network.' 'Activando Avahi para descubrir equipos en la red local.'
    sudo systemctl enable --now avahi-daemon.service
  fi
fi

"$cargo_bin" build --release --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
install -d -m 755 "$install_dir"
temp_bin=$(mktemp "$install_dir/.seamlesscontrold.XXXXXXXX")
trap 'rm -f -- "$temp_bin"' EXIT
install -m 755 "$repo_dir/agent/target/release/seamlesscontrold" "$temp_bin"
mv -f -- "$temp_bin" "$install_dir/seamlesscontrold"
trap - EXIT

if [[ "$install_dir" == "$HOME/.local/bin" ]]; then
  unit_dir="$HOME/.config/systemd/user"
  install -d -m 755 "$unit_dir"
  install -m 644 "$repo_dir/packaging/seamlesscontrol-receiver.service" "$unit_dir/"
  install -m 644 "$repo_dir/packaging/seamlesscontrol-receiver-auto.service" "$unit_dir/"
  install -m 644 "$repo_dir/packaging/seamlesscontrol-sender.service" "$unit_dir/"
  install -m 644 "$repo_dir/packaging/seamlesscontrol-mesh.service" "$unit_dir/"
  systemctl --user daemon-reload
  say 'User services installed but disabled. receiver-auto uses port 47832 and discovers the LAN IP.' 'Unidades de usuario instaladas, sin habilitar. receiver-auto usa el puerto 47832 y descubre la IP LAN.'
fi

say "Agent installed at $install_dir/seamlesscontrold" "Agente instalado en $install_dir/seamlesscontrold"
if [[ ":$PATH:" != *":$install_dir:"* ]]; then
  say "Add $install_dir to the graphical session PATH so the widget can find the agent." "Añada $install_dir a PATH de la sesión gráfica para que el widget encuentre el agente."
fi
