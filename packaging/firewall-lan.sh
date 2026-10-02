#!/usr/bin/env bash
set -euo pipefail

mode=${1:-allow}
port=${2:-47832}
if [[ $mode != allow && $mode != remove && $mode != show ]] ||
   [[ ! $port =~ ^[0-9]+$ ]] || ((port < 1 || port > 65535)); then
  printf 'Uso: %s [allow|remove|show] [PUERTO]\n' "$0" >&2
  exit 2
fi

for program in seamlesscontrold ip ufw; do
  if ! command -v "$program" >/dev/null 2>&1; then
    printf 'Falta %s; instale primero el agente y el cortafuegos que utilice este equipo.\n' "$program" >&2
    exit 1
  fi
done

address=$(seamlesscontrold local-address "$port")
local_ip=${address%:*}
if [[ ! $local_ip =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'No se pudo determinar una dirección IPv4 LAN del receptor.\n' >&2
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
  printf 'No se encontró la interfaz de %s.\n' "$local_ip" >&2
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
  printf 'No se encontró la subred conectada de %s.\n' "$interface" >&2
  exit 1
fi

rule=(allow in on "$interface" from "$network" to "$local_ip" port "$port" proto tcp comment seamlesscontrol)
if [[ $mode == remove ]]; then
  rule=(delete "${rule[@]}")
fi
printf 'Regla UFW propuesta: '
printf '%q ' ufw "${rule[@]}"
printf '\n'
if [[ $mode == show ]]; then
  exit 0
fi

if [[ $mode == remove ]]; then
  prompt='¿Autoriza retirar esta regla de SeamlessControl? [s/N] '
else
  prompt='¿Autoriza añadir esta regla solo para la red local? [s/N] '
fi
printf '%s' "$prompt"
read -r answer || exit 1
case ${answer,,} in
  s|si|sí|y|yes) ;;
  *) printf 'Sin cambios en el cortafuegos.\n'; exit 0 ;;
esac

if command -v pkexec >/dev/null 2>&1 && [[ -n ${WAYLAND_DISPLAY:-}${DISPLAY:-} ]]; then
  pkexec "$(command -v ufw)" "${rule[@]}"
else
  sudo "$(command -v ufw)" "${rule[@]}"
fi
printf 'Regla %s. Compruebe su estado con sudo ufw status.\n' "$([[ $mode == remove ]] && printf retirada || printf aplicada)"
