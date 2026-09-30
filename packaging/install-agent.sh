#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
install_dir=${XDG_BIN_HOME:-"$HOME/.local/bin"}

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
      printf 'Faltan dependencias (%s) y no se encontró el comando omarchy.\n' "${packages[*]}" >&2
      exit 1
    fi
    printf 'Instalando dependencias de Omarchy: %s\n' "${packages[*]}"
    omarchy pkg add "${packages[@]}"
  fi
  if ! "$cargo_bin" --version >/dev/null 2>&1 && [[ ${CARGO:-} == "" && -x /usr/bin/cargo ]]; then
    cargo_bin=/usr/bin/cargo
  fi
  if ! "$cargo_bin" --version >/dev/null 2>&1; then
    printf 'Cargo no está disponible después de instalar Rust.\n' >&2
    exit 1
  fi
  if ! systemctl is-active --quiet avahi-daemon.service; then
    printf 'Activando Avahi para descubrir equipos en la red local.\n'
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
  printf 'Unidades de usuario instaladas, sin habilitar. receiver-auto usa el puerto 47832 y descubre la IP LAN.\n'
fi

printf 'Agente instalado en %s/seamlesscontrold\n' "$install_dir"
if [[ ":$PATH:" != *":$install_dir:"* ]]; then
  printf 'Añada %s a PATH de la sesión gráfica para que el widget encuentre el agente.\n' "$install_dir"
fi
