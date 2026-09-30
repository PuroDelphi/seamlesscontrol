#!/usr/bin/env bash
# One-command setup for the Omarchy plugin and its companion agent.
set -euo pipefail

plugin_id=seamlesscontrol.control
plugin_url=https://github.com/PuroDelphi/seamlesscontrol.git
plugin_dir="$HOME/.config/omarchy/plugins/$plugin_id"

if ((EUID == 0)); then
  printf 'Instale SeamlessControl desde su usuario Omarchy, sin sudo.\n' >&2
  exit 1
fi
if ! command -v omarchy >/dev/null; then
  printf 'No se encontró Omarchy en PATH.\n' >&2
  exit 1
fi

if [[ -e "$plugin_dir" || -L "$plugin_dir" ]]; then
  if [[ ! -d "$plugin_dir/.git" ]]; then
    printf 'Ya existe %s y no es un plugin Git actualizable.\n' "$plugin_dir" >&2
    exit 1
  fi
  origin=$(git -C "$plugin_dir" config --get remote.origin.url)
  case "$origin" in
    "$plugin_url"|https://github.com/PuroDelphi/seamlesscontrol|git@github.com:PuroDelphi/seamlesscontrol.git)
      ;;
    *)
      printf 'El plugin existente procede de otro repositorio: %s\n' "$origin" >&2
      exit 1
      ;;
  esac
  omarchy plugin update "$plugin_id" --yes
  omarchy plugin enable "$plugin_id"
else
  omarchy plugin add "$plugin_url" --enable --yes
fi

if [[ ! -f "$plugin_dir/packaging/install-agent.sh" ]]; then
  printf 'El plugin instalado no contiene packaging/install-agent.sh.\n' >&2
  exit 1
fi
bash "$plugin_dir/packaging/install-agent.sh"
printf 'SeamlessControl instalado. Abra su panel desde la barra de Omarchy.\n'
