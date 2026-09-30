#!/usr/bin/env bash
set -euo pipefail

install_dir=${XDG_BIN_HOME:-"$HOME/.local/bin"}
agent="$install_dir/seamlesscontrold"

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
    if [[ -e "$unit_dir/$unit" || -L "$unit_dir/$unit" ]]; then
      systemctl --user disable --now "$unit"
      installed_units=true
    fi
  done
  if [[ "$installed_units" == true ]]; then
    for unit in "${units[@]}"; do
      rm -f -- "$unit_dir/$unit"
    done
    systemctl --user daemon-reload
  fi
fi

if [[ -x "$agent" ]] && "$agent" status >/dev/null 2>&1; then
  printf 'El agente aún está activo fuera de esas unidades. Termine la sesión desde el panel o terminal y repita la desinstalación.\n' >&2
  exit 1
fi
rm -f -- "$agent"
printf 'Agente retirado. La identidad, los pares y la cuadrícula permanecen en %s/seamlesscontrol/.\n' \
  "${XDG_CONFIG_HOME:-"$HOME/.config"}"
printf 'Retire el widget aparte con: omarchy plugin remove seamlesscontrol.control\n'
