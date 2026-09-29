#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
"$cargo_bin" build --quiet --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
agent="$repo_dir/agent/target/debug/seamlesscontrold"
scratch=$(mktemp -d /tmp/seamlesscontrol-pair-XXXXXX)
server_pid=
client_pid=
cleanup() {
  if [[ -n "$client_pid" ]]; then kill "$client_pid" 2>/dev/null || true; fi
  if [[ -n "$server_pid" ]]; then kill -INT "$server_pid" 2>/dev/null || true; fi
  if [[ -n "$client_pid" ]]; then wait "$client_pid" 2>/dev/null || true; fi
  if [[ -n "$server_pid" ]]; then wait "$server_pid" 2>/dev/null || true; fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT

mkdir -m 700 -p "$scratch/server/run" "$scratch/client/run"
port=$((48000 + RANDOM % 10000))
address="127.0.0.1:$port"
XDG_CONFIG_HOME="$scratch/server/config" XDG_RUNTIME_DIR="$scratch/server/run" \
  "$agent" serve "$address" >"$scratch/server.log" 2>&1 &
server_pid=$!

for _ in {1..100}; do
  if XDG_RUNTIME_DIR="$scratch/server/run" "$agent" status >/dev/null 2>&1; then break; fi
  sleep 0.05
done
XDG_RUNTIME_DIR="$scratch/server/run" "$agent" status >/dev/null

XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
  "$agent" pair "$address" >"$scratch/client.log" 2>&1 &
client_pid=$!

server_status=
client_status=
for _ in {1..100}; do
  server_status=$(XDG_RUNTIME_DIR="$scratch/server/run" "$agent" status 2>/dev/null || true)
  client_status=$(XDG_RUNTIME_DIR="$scratch/client/run" "$agent" status 2>/dev/null || true)
  if [[ "$server_status" == *$'\tpairing\t'* && "$client_status" == *$'\tpairing\t'* ]]; then break; fi
  sleep 0.05
done
[[ "$server_status" == *$'\tpairing\t'* && "$client_status" == *$'\tpairing\t'* ]]
server_code=$(printf '%s\n' "$server_status" | cut -f6)
client_code=$(printf '%s\n' "$client_status" | cut -f6)
[[ "$server_code" =~ ^[0-9]{6}$ && "$server_code" == "$client_code" ]]

XDG_RUNTIME_DIR="$scratch/server/run" "$agent" approve "$server_code" >/dev/null
XDG_RUNTIME_DIR="$scratch/client/run" "$agent" approve "$client_code" >/dev/null
wait "$client_pid"
client_pid=
[[ -f "$scratch/server/config/seamlesscontrol/peers/127.0.0.1" ]]
[[ -f "$scratch/client/config/seamlesscontrol/peers/127.0.0.1" ]]

XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
  "$agent" pair "$address" >"$scratch/reconnect.log" 2>&1
reconnect_message=$(<"$scratch/reconnect.log")
[[ "$reconnect_message" == *'emparejado sin iniciar la captura'* ]]

XDG_RUNTIME_DIR="$scratch/server/run" "$agent" revoke 127.0.0.1 >/dev/null
[[ ! -e "$scratch/server/config/seamlesscontrol/peers/127.0.0.1" ]]
compgen -G "$scratch/server/config/seamlesscontrol/revoked/*" >/dev/null
[[ -z "$(XDG_CONFIG_HOME="$scratch/server/config" "$agent" peers)" ]]
if XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
  "$agent" pair "$address" >"$scratch/revoked.log" 2>&1; then
  printf 'Un par revocado pudo reconectar\n' >&2
  exit 1
fi
revoked_message=$(<"$scratch/revoked.log")
[[ "$revoked_message" == *'PeerRejected'* ]]

kill -INT "$server_pid"
wait "$server_pid"
server_pid=
[[ ! -e "$scratch/server/run/seamlesscontrol/control.sock" ]]
printf 'Emparejamiento, revocación y limpieza del servicio: correcto\n'
