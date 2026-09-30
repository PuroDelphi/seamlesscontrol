#!/usr/bin/env bash
# Pair two temporary identities, then send a four-pixel encrypted movement to
# the normal receiver on this desktop. No input capture session is opened.
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
"$cargo_bin" build --quiet --manifest-path "$repo_dir/agent/Cargo.toml" \
  --bin seamlesscontrold --example injection_local
agent="$repo_dir/agent/target/debug/seamlesscontrold"
probe="$repo_dir/agent/target/debug/examples/injection_local"
runtime=${XDG_RUNTIME_DIR:?Ejecute la prueba dentro de la sesión Omarchy}
if [[ -e "$runtime/seamlesscontrol/control.sock" ]]; then
  printf 'Ya hay un agente local o un socket de control. Ciérrelo antes de la prueba.\n' >&2
  exit 1
fi
scratch=$(mktemp -d /tmp/seamlesscontrol-injection-XXXXXX)
server_pid=
client_pid=
cleanup() {
  local result=$?
  if [[ -n "$client_pid" ]]; then kill "$client_pid" 2>/dev/null || true; fi
  if [[ -n "$server_pid" ]]; then kill -INT "$server_pid" 2>/dev/null || true; fi
  if [[ -n "$client_pid" ]]; then wait "$client_pid" 2>/dev/null || true; fi
  if [[ -n "$server_pid" ]]; then wait "$server_pid" 2>/dev/null || true; fi
  if ((result != 0)); then
    printf '\nRegistro del emisor de prueba:\n' >&2
    cat "$scratch/client.log" >&2 2>/dev/null || true
    printf '\nRegistro del receptor:\n' >&2
    cat "$scratch/server.log" >&2 2>/dev/null || true
  fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT
mkdir -m 700 -p "$scratch/client/run"
port=$((48000 + RANDOM % 10000))
address="127.0.0.1:$port"

XDG_CONFIG_HOME="$scratch/server/config" "$agent" serve "$address" \
  >"$scratch/server.log" 2>&1 &
server_pid=$!
for _ in {1..100}; do
  if "$agent" status >/dev/null 2>&1; then break; fi
  sleep 0.05
done
"$agent" status >/dev/null

XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
  "$agent" pair "$address" >"$scratch/client.log" 2>&1 &
client_pid=$!
server_status=
client_status=
for _ in {1..100}; do
  server_status=$("$agent" status 2>/dev/null || true)
  client_status=$(XDG_RUNTIME_DIR="$scratch/client/run" "$agent" status 2>/dev/null || true)
  if [[ "$server_status" == *$'\tpairing\t'* && "$client_status" == *$'\tpairing\t'* ]]; then break; fi
  sleep 0.05
done
[[ "$server_status" == *$'\tpairing\t'* && "$client_status" == *$'\tpairing\t'* ]]
server_code=$(printf '%s\n' "$server_status" | cut -f6)
client_code=$(printf '%s\n' "$client_status" | cut -f6)
[[ "$server_code" =~ ^[0-9]{6}$ && "$server_code" == "$client_code" ]]
"$agent" approve "$server_code" >/dev/null
XDG_RUNTIME_DIR="$scratch/client/run" "$agent" approve "$client_code" >/dev/null
wait "$client_pid"
client_pid=

"$probe" "$address" "$scratch/client/config/seamlesscontrol" \
  >"$scratch/client.log" 2>&1
cat "$scratch/client.log"
rg -q '^INJECTION\t' "$scratch/client.log"
printf 'Recepción Noise e inyección virtual con cursor restaurado: correcto.\n'
