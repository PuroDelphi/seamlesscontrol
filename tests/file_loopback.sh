#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
"$cargo_bin" build --quiet --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
agent="$repo_dir/agent/target/debug/seamlesscontrold"
scratch=$(mktemp -d /tmp/seamlesscontrol-file-XXXXXX)
server_pid=
client_pid=
receiver_pid=
sender_pid=
lease_pid=
cleanup() {
  for pid in "$sender_pid" "$receiver_pid" "$client_pid" "$server_pid" "$lease_pid"; do
    if [[ -n "$pid" ]]; then kill "$pid" 2>/dev/null || true; fi
  done
  for pid in "$sender_pid" "$receiver_pid" "$client_pid" "$server_pid" "$lease_pid"; do
    if [[ -n "$pid" ]]; then wait "$pid" 2>/dev/null || true; fi
  done
  rm -rf -- "$scratch"
}
trap cleanup EXIT

mkdir -m 700 -p "$scratch/server/run" "$scratch/client/run" "$scratch/downloads"
port=$((48000 + RANDOM % 10000))
file_port=$((port + 1))
address="127.0.0.1:$port"
file_address="127.0.0.1:$file_port"
XDG_CONFIG_HOME="$scratch/server/config" XDG_RUNTIME_DIR="$scratch/server/run" \
  "$agent" serve "$address" >"$scratch/server.log" 2>&1 &
server_pid=$!
for _ in {1..100}; do
  if XDG_RUNTIME_DIR="$scratch/server/run" "$agent" status >/dev/null 2>&1; then break; fi
  sleep 0.05
done
XDG_RUNTIME_DIR="$scratch/server/run" "$agent" status >/dev/null
XDG_CONFIG_HOME="$scratch/client/config" XDG_RUNTIME_DIR="$scratch/client/run" \
  "$agent" pair "$address" >"$scratch/pair.log" 2>&1 &
client_pid=$!
for _ in {1..100}; do
  server_status=$(XDG_RUNTIME_DIR="$scratch/server/run" "$agent" status 2>/dev/null || true)
  client_status=$(XDG_RUNTIME_DIR="$scratch/client/run" "$agent" status 2>/dev/null || true)
  if [[ "$server_status" == *$'\tpairing\t'* && "$client_status" == *$'\tpairing\t'* ]]; then break; fi
  sleep 0.05
done
[[ "$server_status" == *$'\tpairing\t'* && "$client_status" == *$'\tpairing\t'* ]]
code=$(printf '%s\n' "$server_status" | cut -f6)
[[ "$code" == "$(printf '%s\n' "$client_status" | cut -f6)" ]]
XDG_RUNTIME_DIR="$scratch/server/run" "$agent" approve "$code" >/dev/null
XDG_RUNTIME_DIR="$scratch/client/run" "$agent" approve "$code" >/dev/null
wait "$client_pid"
client_pid=

# The file-protocol fixture simulates both ends of an authenticated active
# control session. The separate active_session unit tests verify lease cleanup.
server_key=$(XDG_CONFIG_HOME="$scratch/client/config" "$agent" peers | awk -F '\t' '$2 == "127.0.0.1" { print $3 }')
client_key=$(XDG_CONFIG_HOME="$scratch/server/config" "$agent" peers | awk -F '\t' '$2 == "127.0.0.1" { print $3 }')
[[ -n "$server_key" && -n "$client_key" ]]
server_lease="$scratch/server/config/seamlesscontrol/active-sessions/$client_key/test.lease"
client_lease="$scratch/client/config/seamlesscontrol/active-sessions/$server_key/test.lease"
mkdir -m 700 -p "${server_lease%/*}" "${client_lease%/*}"
(
  while :; do
    timestamp=$(date +%s%3N)
    printf '127.0.0.1\n%s\n' "$timestamp" > "$server_lease"
    printf '127.0.0.1\n%s\n' "$timestamp" > "$client_lease"
    sleep 0.25
  done
) &
lease_pid=$!
for _ in {1..40}; do
  if [[ -s "$server_lease" && -s "$client_lease" ]]; then break; fi
  sleep 0.05
done

printf 'contenido cifrado y verificado\n' >"$scratch/ejemplo.txt"
mkfifo "$scratch/input"
XDG_CONFIG_HOME="$scratch/server/config" "$agent" receive-file-ui "$file_address" "$scratch/downloads" \
  <"$scratch/input" >"$scratch/receive.log" 2>&1 &
receiver_pid=$!
exec 3>"$scratch/input"
for _ in {1..100}; do
  if [[ -n $(ss -ltnH "( sport = :$file_port )") ]]; then break; fi
  sleep 0.05
done
[[ -n $(ss -ltnH "( sport = :$file_port )") ]]
[[ $(<"$scratch/receive.log") == *$'LISTENING\t127.0.0.1:'* ]]
XDG_CONFIG_HOME="$scratch/client/config" "$agent" send-file "$file_address" "$scratch/ejemplo.txt" \
  >"$scratch/send.log" 2>&1 &
sender_pid=$!
for _ in {1..100}; do
  if [[ $(<"$scratch/receive.log") == *$'OFFER\t'* ]]; then break; fi
  sleep 0.05
done
[[ $(<"$scratch/receive.log") == *$'OFFER\t127.0.0.1\tejemplo.txt\t'* ]]
printf 'SI\n' >&3
wait "$sender_pid"
wait "$receiver_pid"
sender_pid=
receiver_pid=
exec 3>&-
cmp "$scratch/ejemplo.txt" "$scratch/downloads/ejemplo.txt"

printf 'no debe guardarse\n' >"$scratch/rechazado.txt"
XDG_CONFIG_HOME="$scratch/server/config" "$agent" receive-file-ui "$file_address" "$scratch/downloads" \
  <"$scratch/input" >"$scratch/reject.log" 2>&1 &
receiver_pid=$!
exec 3>"$scratch/input"
for _ in {1..100}; do
  if [[ -n $(ss -ltnH "( sport = :$file_port )") ]]; then break; fi
  sleep 0.05
done
[[ -n $(ss -ltnH "( sport = :$file_port )") ]]
XDG_CONFIG_HOME="$scratch/client/config" "$agent" send-file "$file_address" "$scratch/rechazado.txt" \
  >"$scratch/rejected-send.log" 2>&1 &
sender_pid=$!
for _ in {1..100}; do
  if [[ $(<"$scratch/reject.log") == *$'OFFER\t'* ]]; then break; fi
  sleep 0.05
done
[[ $(<"$scratch/reject.log") == *$'OFFER\t127.0.0.1\trechazado.txt\t'* ]]
printf 'NO\n' >&3
if wait "$sender_pid"; then
  printf 'El emisor anunció éxito tras un rechazo\n' >&2
  exit 1
fi
wait "$receiver_pid"
sender_pid=
receiver_pid=
exec 3>&-
[[ ! -e "$scratch/downloads/rechazado.txt" ]]
printf 'Archivo aceptado y rechazo sin escritura: correcto\n'
