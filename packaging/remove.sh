#!/usr/bin/env bash
# Remove the agent before Omarchy deletes the plugin checkout containing it.
set -euo pipefail

plugin_id=seamlesscontrol.control
plugin_dir="$HOME/.config/omarchy/plugins/$plugin_id"
if ((EUID == 0)); then
  printf 'Retire SeamlessControl desde su usuario Omarchy, sin sudo.\n' >&2
  exit 1
fi
if ! command -v omarchy >/dev/null; then
  printf 'No se encontró Omarchy en PATH.\n' >&2
  exit 1
fi
if [[ ! -f "$plugin_dir/packaging/uninstall-agent.sh" ]]; then
  printf 'No se encontró el desinstalador en %s.\n' "$plugin_dir" >&2
  exit 1
fi

SEAMLESSCONTROL_REMOVE_PLUGIN=1 bash "$plugin_dir/packaging/uninstall-agent.sh"
exec omarchy plugin remove "$plugin_id" --yes
