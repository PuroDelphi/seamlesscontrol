#!/usr/bin/env bash
# Local capture -> Noise -> authenticated receiver check. The receiver counts
# event types and never creates virtual input, so both identities can share one desktop.
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
"$cargo_bin" build --quiet --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
if [[ ${1:-} == --synthetic ]]; then
  "$cargo_bin" build --quiet --manifest-path "$repo_dir/agent/Cargo.toml" --bin virtual-input-probe
elif [[ $# -ne 0 ]]; then
  printf 'Uso: bash tests/integrated_capture_local.sh [--synthetic]\n' >&2
  exit 2
fi

agent="$repo_dir/agent/target/debug/seamlesscontrold"
runtime=${XDG_RUNTIME_DIR:?Ejecute la prueba dentro de la sesión Omarchy}
if [[ -e "$runtime/seamlesscontrol/control.sock" ]]; then
  printf 'Ya hay un agente local o un socket de control. Ciérrelo antes de la prueba.\n' >&2
  exit 1
fi
scratch=$(mktemp -d /tmp/seamlesscontrol-integrated-XXXXXX)
server_pid=
client_pid=
cleanup() {
  local result=$?
  if [[ -n "$client_pid" ]]; then kill -INT "$client_pid" 2>/dev/null || true; fi
  if [[ -n "$server_pid" ]]; then kill -INT "$server_pid" 2>/dev/null || true; fi
  if [[ -n "$client_pid" ]]; then wait "$client_pid" 2>/dev/null || true; fi
  if [[ -n "$server_pid" ]]; then wait "$server_pid" 2>/dev/null || true; fi
  if ((result != 0)); then
    printf '\nRegistro del emisor:\n' >&2
    cat "$scratch/client.log" >&2 2>/dev/null || true
    printf '\nRegistro del receptor:\n' >&2
    cat "$scratch/server.log" >&2 2>/dev/null || true
  fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT
mkdir -m 700 -p "$scratch/server/run" "$scratch/client/config"
port=$((48000 + RANDOM % 10000))
address="127.0.0.1:$port"

SEAMLESSCONTROL_TEST_OBSERVE=1 XDG_CONFIG_HOME="$scratch/server/config" \
  XDG_RUNTIME_DIR="$scratch/server/run" "$agent" serve "$address" \
  >"$scratch/server.log" 2>&1 &
server_pid=$!
for _ in {1..100}; do
  if XDG_RUNTIME_DIR="$scratch/server/run" "$agent" status >/dev/null 2>&1; then break; fi
  sleep 0.05
done
XDG_RUNTIME_DIR="$scratch/server/run" "$agent" status >/dev/null

XDG_CONFIG_HOME="$scratch/client/config" "$agent" pair "$address" \
  >"$scratch/client.log" 2>&1 &
client_pid=$!
server_status=
client_status=
for _ in {1..100}; do
  server_status=$(XDG_RUNTIME_DIR="$scratch/server/run" "$agent" status 2>/dev/null || true)
  client_status=$("$agent" status 2>/dev/null || true)
  if [[ "$server_status" == *$'\tpairing\t'* && "$client_status" == *$'\tpairing\t'* ]]; then break; fi
  sleep 0.05
done
[[ "$server_status" == *$'\tpairing\t'* && "$client_status" == *$'\tpairing\t'* ]]
server_code=$(printf '%s\n' "$server_status" | cut -f6)
client_code=$(printf '%s\n' "$client_status" | cut -f6)
[[ "$server_code" =~ ^[0-9]{6}$ && "$server_code" == "$client_code" ]]
XDG_RUNTIME_DIR="$scratch/server/run" "$agent" approve "$server_code" >/dev/null
"$agent" approve "$client_code" >/dev/null
wait "$client_pid"
client_pid=

XDG_CONFIG_HOME="$scratch/client/config" "$agent" connect "$address" right \
  >"$scratch/client.log" 2>&1 &
client_pid=$!
client_status=
printf 'Esperando el portal; cuando aparezca la captura, cruce el borde derecho.\n'
for _ in {1..150}; do
  client_status=$("$agent" status 2>/dev/null || true)
  if [[ "$client_status" == *$'\tready\t'* || "$client_status" == *$'\tcontrolling\t'* ]]; then break; fi
  if ! kill -0 "$client_pid" 2>/dev/null; then break; fi
  sleep 0.2
done
[[ "$client_status" == *$'\tready\t'* || "$client_status" == *$'\tcontrolling\t'* ]]
printf 'Captura lista: cruce el borde derecho y mueva el ratón durante la prueba.\n'
if [[ ${1:-} == --synthetic ]]; then
  "$repo_dir/agent/target/debug/virtual-input-probe" --edge-test
  sleep 3
else
  sleep 20
fi
kill -INT "$client_pid"
wait "$client_pid"
client_pid=
for _ in {1..100}; do
  if rg -q '^OBSERVED\t' "$scratch/server.log"; then break; fi
  sleep 0.05
done
line=$(rg '^OBSERVED\t' "$scratch/server.log" | tail -n 1)
printf '%s\n' "$line"
begin=$(printf '%s\n' "$line" | sed -n 's/.*begin=\([0-9]*\).*/\1/p')
motion=$(printf '%s\n' "$line" | sed -n 's/.*motion=\([0-9]*\).*/\1/p')
((begin > 0 && motion > 0))
if [[ ${1:-} != --synthetic ]]; then
  keys=$(printf '%s\n' "$line" | sed -n 's/.*keys=\([0-9]*\).*/\1/p')
  buttons=$(printf '%s\n' "$line" | sed -n 's/.*buttons=\([0-9]*\).*/\1/p')
  ((keys > 0 && buttons > 0))
fi
printf 'Captura EIS, canal Noise y recepción autenticada: correcto (sin inyección).\n'
