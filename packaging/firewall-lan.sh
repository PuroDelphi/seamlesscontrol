#!/usr/bin/env bash
set -euo pipefail

mode=${1:-allow}
port=${2:-47832}
confirmed_by_panel=${3:-}
say() {
  if [[ ${SEAMLESSCONTROL_LANG:-en} == es ]]; then printf '%s\n' "$2"; else printf '%s\n' "$1"; fi
}
if [[ $mode != allow && $mode != remove && $mode != show ]] ||
   [[ ! $port =~ ^[0-9]+$ ]] || ((port < 1 || port > 65535)) ||
   [[ -n $confirmed_by_panel && $confirmed_by_panel != --yes ]] || (( $# > 3 )); then
  say "Usage: $0 [allow|remove|show] [PORT] [--yes]" "Uso: $0 [allow|remove|show] [PUERTO] [--yes]" >&2
  exit 2
fi

for program in seamlesscontrold ip ufw; do
  if ! command -v "$program" >/dev/null 2>&1; then
    say "Missing $program; install the agent and the firewall used on this computer first." "Falta $program; instale primero el agente y el cortafuegos que utilice este equipo." >&2
    exit 1
  fi
done

address=$(seamlesscontrold local-address "$port")
local_ip=${address%:*}
if [[ ! $local_ip =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  say 'Could not determine a receiver LAN IPv4 address.' 'No se pudo determinar una dirección IPv4 LAN del receptor.' >&2
  exit 1
fi

interface=
while read -r _ candidate family cidr _; do
  if [[ $family == inet && ${cidr%/*} == "$local_ip" ]]; then
    interface=${candidate%%@*}
    break
  fi
done < <(ip -4 -o addr show scope global)
if [[ -z $interface ]]; then
  say "Could not find the interface for $local_ip." "No se encontró la interfaz de $local_ip." >&2
  exit 1
fi

network=
while IFS= read -r route; do
  read -r prefix _ <<<"$route"
  if [[ $prefix =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+/[0-9]+$ &&
        " $route " == *" src $local_ip "* ]]; then
    network=$prefix
    break
  fi
done < <(ip -4 route show dev "$interface" scope link)
if [[ -z $network ]]; then
  say "Could not find the connected subnet for $interface." "No se encontró la subred conectada de $interface." >&2
  exit 1
fi

rule=(allow in on "$interface" from "$network" to "$local_ip" port "$port" proto tcp comment seamlesscontrol)
if [[ $mode == remove ]]; then
  rule=(delete "${rule[@]}")
fi
if [[ ${SEAMLESSCONTROL_LANG:-en} == es ]]; then printf 'Regla UFW propuesta: '; else printf 'Proposed UFW rule: '; fi
printf '%q ' ufw "${rule[@]}"
printf '\n'
if [[ $mode == show ]]; then
  exit 0
fi

if [[ $mode == remove ]]; then
  if [[ ${SEAMLESSCONTROL_LANG:-en} == es ]]; then
    prompt='¿Autoriza retirar esta regla de SeamlessControl? [s/N] '
  else
    prompt='Remove this SeamlessControl rule? [y/N] '
  fi
else
  if [[ ${SEAMLESSCONTROL_LANG:-en} == es ]]; then
    prompt='¿Autoriza añadir esta regla solo para la red local? [s/N] '
  else
    prompt='Add this rule for the local network only? [y/N] '
  fi
fi
printf '%s' "$prompt"
if [[ $confirmed_by_panel == --yes ]]; then
  say 'confirmed in the panel; requesting system authorization.' 'confirmado en el panel; solicitando autorización del sistema.'
else
  read -r answer || exit 1
  case ${answer,,} in
    s|si|sí|y|yes) ;;
    *) say 'No firewall changes made.' 'Sin cambios en el cortafuegos.'; exit 0 ;;
  esac
fi

if command -v pkexec >/dev/null 2>&1 && [[ -n ${WAYLAND_DISPLAY:-}${DISPLAY:-} ]]; then
  pkexec "$(command -v ufw)" "${rule[@]}"
else
  sudo "$(command -v ufw)" "${rule[@]}"
fi
if [[ $mode == remove ]]; then
  say 'Rule removed. Check with sudo ufw status.' 'Regla retirada. Compruebe su estado con sudo ufw status.'
else
  say 'Rule applied. Check with sudo ufw status.' 'Regla aplicada. Compruebe su estado con sudo ufw status.'
fi
