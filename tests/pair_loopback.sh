#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
"$cargo_bin" build --quiet --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
agent="$repo_dir/agent/target/debug/seamlesscontrold"
scratch=$(mktemp -d /tmp/seamlesscontrol-pair-XXXXXX)
server_pid=
client_pid=
slow_pid=
cleanup() {
  if [[ -n "$slow_pid" ]]; then kill "$slow_pid" 2>/dev/null || true; fi
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

if XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
  "$agent" latency "$address" >"$scratch/unpaired-latency.log" 2>&1; then
  printf 'La medición de latencia aceptó un equipo sin emparejar\n' >&2
  exit 1
fi
[[ ! -e "$scratch/client/config/seamlesscontrol/peers/127.0.0.1" ]]

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

XDG_CONFIG_HOME="$scratch/client/config" "$agent" topology set 127.0.0.1 1 0 >/dev/null
topology=$(XDG_CONFIG_HOME="$scratch/client/config" "$agent" topology)
[[ "$topology" == *$'SLOT\tlocal\t0\t0'* ]]
[[ "$topology" == *$'SLOT\t127.0.0.1\t1\t0'* ]]
route=$(XDG_CONFIG_HOME="$scratch/client/config" "$agent" topology route 127.0.0.1)
[[ "$route" == $'HOP\t1\t127.0.0.1\tright' ]]
XDG_CONFIG_HOME="$scratch/client/config" "$agent" topology set local 1 0 >/dev/null
topology=$(XDG_CONFIG_HOME="$scratch/client/config" "$agent" topology)
[[ "$topology" == *$'SLOT\tlocal\t1\t0'* ]]
[[ "$topology" == *$'SLOT\t127.0.0.1\t0\t0'* ]]
XDG_CONFIG_HOME="$scratch/client/config" "$agent" topology set local 0 0 >/dev/null

# An unauthenticated socket may be slow; it must not block another paired
# peer's encrypted diagnostic session.
bash -c 'exec 9<>/dev/tcp/127.0.0.1/$1; sleep 5' _ "$port" &
slow_pid=$!
sleep 0.2
latency_output=$(timeout 2 env XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
  "$agent" latency "$address")
kill "$slow_pid" 2>/dev/null || true
wait "$slow_pid" 2>/dev/null || true
slow_pid=
latency_line=$(printf '%s\n' "$latency_output" | awk -F '\t' '$1 == "LATENCY" { print }')
IFS=$'\t' read -r label count minimum p50 p95 maximum <<<"$latency_line"
[[ "$label" == LATENCY && "$count" == 20 ]]
[[ "$minimum" =~ ^[0-9]+$ && "$p50" =~ ^[0-9]+$ && "$p95" =~ ^[0-9]+$ && "$maximum" =~ ^[0-9]+$ ]]
(( minimum <= p50 && p50 <= p95 && p95 <= maximum ))

XDG_CONFIG_HOME="$scratch/server/config" "$agent" topology set 127.0.0.1 1 0 >/dev/null
XDG_CONFIG_HOME="$scratch/server/config" XDG_RUNTIME_DIR="$scratch/server/run" \
  "$agent" revoke 127.0.0.1 >/dev/null
[[ ! -e "$scratch/server/config/seamlesscontrol/peers/127.0.0.1" ]]
[[ "$(XDG_CONFIG_HOME="$scratch/server/config" "$agent" topology)" != *$'SLOT\t127.0.0.1'* ]]
compgen -G "$scratch/server/config/seamlesscontrol/revoked/*" >/dev/null
[[ -z "$(XDG_CONFIG_HOME="$scratch/server/config" "$agent" peers)" ]]
if XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
  "$agent" latency "$address" >"$scratch/revoked.log" 2>&1; then
  printf 'Un par revocado pudo conectarse sin nueva aprobación\n' >&2
  exit 1
fi

pair_again() {
  XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
    "$agent" pair "$address" >"$scratch/reapproved.log" 2>&1 &
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
}

pair_again
[[ -z "$(find "$scratch/server/config/seamlesscontrol/revoked" -type f -print -quit)" ]]
XDG_CONFIG_HOME="$scratch/server/config" XDG_RUNTIME_DIR="$scratch/server/run" \
  "$agent" revoke 127.0.0.1 >/dev/null
XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
  "$agent" revoke 127.0.0.1 >/dev/null
pair_again
[[ -z "$(find "$scratch/server/config/seamlesscontrol/revoked" -type f -print -quit)" ]]
[[ -z "$(find "$scratch/client/config/seamlesscontrol/revoked" -type f -print -quit)" ]]

kill -INT "$server_pid"
wait "$server_pid"
server_pid=
[[ ! -e "$scratch/server/run/seamlesscontrol/control.sock" ]]
printf 'Emparejamiento, revocación, nueva aprobación y limpieza del servicio: correcto\n'
