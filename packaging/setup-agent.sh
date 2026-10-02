#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
language=${2:-en}
result=0
say() {
  if [[ $language == es ]]; then printf '%s\n' "$2"; else printf '%s\n' "$1"; fi
}
case ${1:-} in
  install)
    say 'SeamlessControl · Install agent and required packages' 'SeamlessControl · Instalar agente y paquetes necesarios'
    if SEAMLESSCONTROL_LANG="$language" bash "$repo_dir/packaging/install-agent.sh"; then
      say 'Installation complete. Return to the SeamlessControl panel.' 'Instalación terminada. Vuelva al panel SeamlessControl.'
    else
      result=1
      say 'Installation failed. Review the error above and try again.' 'La instalación falló. Revise el error anterior e inténtelo de nuevo.' >&2
    fi
    ;;
  remove)
    say 'SeamlessControl · Remove agent and packages installed by this plugin' 'SeamlessControl · Retirar agente y paquetes instalados por este plugin'
    if SEAMLESSCONTROL_LANG="$language" bash "$repo_dir/packaging/uninstall-agent.sh" --remove-deps; then
      say 'Removal complete. The Omarchy widget stays until you remove the plugin.' 'Retirada terminada. El widget permanece hasta que quite el plugin.'
    else
      result=1
      say 'Removal failed. Review the error above and try again.' 'La retirada falló. Revise el error anterior e inténtelo de nuevo.' >&2
    fi
    ;;
  *)
    printf 'Usage: %s install|remove\n' "$0" >&2
    exit 2
    ;;
esac
say 'Press Enter to close this window.' 'Pulse Intro para cerrar esta ventana.'
read -r _ || true
exit "$result"
