#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
"$cargo_bin" build --quiet --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
agent="$repo_dir/agent/target/debug/seamlesscontrold"
scratch=$(mktemp -d /tmp/seamlesscontrol-roaming-XXXXXX)
server_pid=
client_pid=
proxy_pid=
cleanup() {
  if [[ -n "$client_pid" ]]; then kill "$client_pid" 2>/dev/null || true; fi
  if [[ -n "$proxy_pid" ]]; then kill "$proxy_pid" 2>/dev/null || true; fi
  if [[ -n "$server_pid" ]]; then kill -INT "$server_pid" 2>/dev/null || true; fi
  if [[ -n "$client_pid" ]]; then wait "$client_pid" 2>/dev/null || true; fi
  if [[ -n "$proxy_pid" ]]; then wait "$proxy_pid" 2>/dev/null || true; fi
  if [[ -n "$server_pid" ]]; then wait "$server_pid" 2>/dev/null || true; fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT

mkdir -m 700 -p "$scratch/server/run" "$scratch/client/run"
port=$((49000 + RANDOM % 8000))
proxy_port=$((49000 + RANDOM % 8000))
server_address="127.0.0.1:$port"
new_address="127.0.0.3:$proxy_port"

XDG_CONFIG_HOME="$scratch/server/config" XDG_RUNTIME_DIR="$scratch/server/run" \
  "$agent" serve "$server_address" >"$scratch/server.log" 2>&1 &
server_pid=$!
for _ in {1..100}; do
  if XDG_RUNTIME_DIR="$scratch/server/run" "$agent" status >/dev/null 2>&1; then break; fi
  sleep 0.05
done
XDG_RUNTIME_DIR="$scratch/server/run" "$agent" status >/dev/null

XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
  "$agent" pair "$server_address" >"$scratch/client.log" 2>&1 &
client_pid=$!
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

XDG_CONFIG_HOME="$scratch/client/config" "$agent" topology set 127.0.0.1 1 0 >/dev/null
XDG_CONFIG_HOME="$scratch/server/config" "$agent" topology set 127.0.0.1 1 0 >/dev/null
server_key=$(<"$scratch/client/config/seamlesscontrol/peers/127.0.0.1")
client_key=$(<"$scratch/server/config/seamlesscontrol/peers/127.0.0.1")

python3 -u "$repo_dir/tests/roaming_proxy.py" "$proxy_port" "$port" >"$scratch/proxy.log" 2>&1 &
proxy_pid=$!
for _ in {1..100}; do
  if [[ -s "$scratch/proxy.log" ]]; then break; fi
  if ! kill -0 "$proxy_pid" 2>/dev/null; then cat "$scratch/proxy.log" >&2; exit 1; fi
  sleep 0.05
done
[[ "$(<"$scratch/proxy.log")" == READY ]]

timeout 15 env XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
  "$agent" pair "$new_address" >"$scratch/roaming.log" 2>&1
wait "$proxy_pid"
proxy_pid=

[[ ! -e "$scratch/client/config/seamlesscontrol/peers/127.0.0.1" ]]
[[ ! -e "$scratch/server/config/seamlesscontrol/peers/127.0.0.1" ]]
[[ "$(<"$scratch/client/config/seamlesscontrol/peers/127.0.0.3")" == "$server_key" ]]
[[ "$(<"$scratch/server/config/seamlesscontrol/peers/127.0.0.2")" == "$client_key" ]]
client_topology=$(XDG_CONFIG_HOME="$scratch/client/config" "$agent" topology)
server_topology=$(XDG_CONFIG_HOME="$scratch/server/config" "$agent" topology)
[[ "$client_topology" == *$'SLOT\t127.0.0.3\t1\t0'* ]]
[[ "$server_topology" == *$'SLOT\t127.0.0.2\t1\t0'* ]]
[[ "$client_topology" != *$'SLOT\t127.0.0.1'* ]]
[[ "$server_topology" != *$'SLOT\t127.0.0.1'* ]]
printf 'Cambio de IP autenticado en ambos extremos y cuadrícula conservada: correcto\n'
