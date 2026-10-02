#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
install_dir=${XDG_BIN_HOME:-"$HOME/.local/bin"}
agent="$install_dir/seamlesscontrold"
state_dir=${XDG_STATE_HOME:-"$HOME/.local/state"}/seamlesscontrol
installed_packages_file="$state_dir/installed-packages"
say() {
  if [[ ${SEAMLESSCONTROL_LANG:-en} == es ]]; then printf '%s\n' "$2"; else printf '%s\n' "$1"; fi
}
remove_deps=false
if [[ ${1:-} == --remove-deps ]]; then
  remove_deps=true
elif [[ $# -gt 0 ]]; then
  say "Usage: $0 [--remove-deps]" "Uso: $0 [--remove-deps]" >&2
  exit 2
fi

if [[ "$install_dir" == "$HOME/.local/bin" ]]; then
  unit_dir="$HOME/.config/systemd/user"
  units=(
    seamlesscontrol-receiver.service
    seamlesscontrol-receiver-auto.service
    seamlesscontrol-sender.service
    seamlesscontrol-mesh.service
  )
  installed_units=false
  for unit in "${units[@]}"; do
    target="$unit_dir/$unit"
    if [[ -f "$target" && ! -L "$target" ]] && cmp -s -- "$repo_dir/packaging/$unit" "$target"; then
      systemctl --user disable --now "$unit"
      rm -f -- "$target"
      installed_units=true
    elif [[ -e "$target" || -L "$target" ]]; then
      say "Preserving modified service configuration: $target" "Se conserva la configuración modificada del servicio: $target"
    fi
  done
  if [[ "$installed_units" == true ]]; then
    systemctl --user daemon-reload
  fi
fi

agent_running() {
  if command -v pgrep >/dev/null 2>&1 && command -v ps >/dev/null 2>&1; then
    while IFS= read -r pid; do
      args=$(ps -p "$pid" -o args= 2>/dev/null || true)
      if [[ $args =~ (^|/)seamlesscontrold[[:space:]]+(serve|serve-auto|connect|mesh|pair|send-file|receive-file|receive-file-auto|receive-file-ui|receive-file-auto-ui)([[:space:]]|$) ]]; then
        return 0
      fi
    done < <(pgrep -u "$(id -u)" -f '[s]eamlesscontrold' || true)
  fi
  [[ -x "$agent" ]] && "$agent" status >/dev/null 2>&1
}
if agent_running; then
  say 'The agent is still running outside those services. Stop it in the panel or terminal, then retry.' 'El agente aún está activo fuera de esas unidades. Termine la sesión desde el panel o terminal y repita la desinstalación.' >&2
  exit 1
fi
rm -f -- "$agent"
if [[ $remove_deps == true && -f "$installed_packages_file" ]]; then
  packages=()
  while IFS= read -r package; do
    case "$package" in rust|avahi|wl-clipboard) packages+=("$package") ;; esac
  done < "$installed_packages_file"
  if ((${#packages[@]})); then
    say "Removing only packages installed by SeamlessControl: ${packages[*]}" "Retirando solo las dependencias que instaló SeamlessControl: ${packages[*]}"
    omarchy pkg drop "${packages[@]}"
  fi
  rm -f -- "$installed_packages_file"
fi
say "Agent removed. Identity, peers and layout remain in ${XDG_CONFIG_HOME:-"$HOME/.config"}/seamlesscontrol/." "Agente retirado. La identidad, los pares y la cuadrícula permanecen en ${XDG_CONFIG_HOME:-"$HOME/.config"}/seamlesscontrol/."
say 'Remove the widget separately with: omarchy plugin remove seamlesscontrol.control' 'Retire el widget aparte con: omarchy plugin remove seamlesscontrol.control'
