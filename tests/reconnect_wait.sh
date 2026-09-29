#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
"$cargo_bin" build --quiet --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
agent="$repo_dir/agent/target/debug/seamlesscontrold"
scratch=$(mktemp -d /tmp/seamlesscontrol-retry-XXXXXX)
sender_pid=
cleanup() {
  if [[ -n "$sender_pid" ]]; then
    kill -INT "$sender_pid" 2>/dev/null || true
    wait "$sender_pid" 2>/dev/null || true
  fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT

mkdir -m 700 -p "$scratch/run"
port=$((48000 + RANDOM % 10000))
XDG_CONFIG_HOME="$scratch/config" XDG_RUNTIME_DIR="$scratch/run" \
  "$agent" connect "127.0.0.1:$port" right >"$scratch/sender.log" 2>&1 &
sender_pid=$!

status=
for _ in {1..100}; do
  status=$(XDG_RUNTIME_DIR="$scratch/run" "$agent" status 2>/dev/null || true)
  if [[ "$status" == *$'\treconnecting\t'* ]]; then break; fi
  sleep 0.05
done
[[ "$status" == *$'\treconnecting\t'* ]]
sleep 1.3
kill -0 "$sender_pid"
retry_count=$(awk '/Conexión interrumpida/ { count++ } END { print count+0 }' "$scratch/sender.log")
(( retry_count >= 2 ))
kill -INT "$sender_pid"
wait "$sender_pid"
sender_pid=
[[ ! -e "$scratch/run/seamlesscontrol/control.sock" ]]
printf 'Reintento sin receptor y salida limpia: correcto\n'
