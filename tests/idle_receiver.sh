#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
agent=${SEAMLESSCONTROL_AGENT:-"$repo_dir/agent/target/release/seamlesscontrold"}
seconds=${SC_IDLE_SECONDS:-30}
if ! [[ "$seconds" =~ ^[0-9]+$ ]] || (( seconds < 1 || seconds > 300 )); then
  printf 'SC_IDLE_SECONDS debe estar entre 1 y 300\n' >&2
  exit 2
fi
if [[ ! -x "$agent" ]]; then
  printf 'Compile primero el agente release: cargo build --release --manifest-path agent/Cargo.toml --bin seamlesscontrold\n' >&2
  exit 2
fi

scratch=$(mktemp -d /tmp/seamlesscontrol-idle-XXXXXX)
server_pid=
cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill -INT "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT
mkdir -m 700 -p "$scratch/config" "$scratch/run"
port=$((49000 + RANDOM % 8000))
XDG_CONFIG_HOME="$scratch/config" XDG_RUNTIME_DIR="$scratch/run" \
  "$agent" serve-auto "$port" >"$scratch/server.log" 2>&1 &
server_pid=$!

status=
for _ in {1..100}; do
  status=$(XDG_RUNTIME_DIR="$scratch/run" "$agent" status 2>/dev/null || true)
  if [[ "$status" == *$'\tlistening\t'* ]]; then break; fi
  if ! kill -0 "$server_pid" 2>/dev/null; then
    cat "$scratch/server.log" >&2
    exit 1
  fi
  sleep 0.05
done
[[ "$status" == *$'\tlistening\t'* ]]
sleep 0.2

read_ticks() { awk '{print $14+$15}' "/proc/$1/stat"; }
read_rss() { awk '/^VmRSS:/ {print $2}' "/proc/$1/status"; }
read_fds() { find "/proc/$1/fd" -maxdepth 1 -type l | wc -l; }
child_pid=$(pgrep -P "$server_pid" -f avahi-publish-service | head -n 1 || true)
agent_start=$(read_ticks "$server_pid")
agent_rss_start=$(read_rss "$server_pid")
if [[ -n "$child_pid" ]]; then
  child_start=$(read_ticks "$child_pid")
  child_rss_start=$(read_rss "$child_pid")
fi

sleep "$seconds"

hz=$(getconf CLK_TCK)
agent_ticks=$(( $(read_ticks "$server_pid") - agent_start ))
agent_rss_end=$(read_rss "$server_pid")
printf 'Intervalo: %s s · %s ticks/s\n' "$seconds" "$hz"
printf 'Agente: %s ticks CPU · RSS %s → %s KiB · %s descriptores\n' \
  "$agent_ticks" "$agent_rss_start" "$agent_rss_end" "$(read_fds "$server_pid")"
if [[ -n "$child_pid" ]]; then
  child_ticks=$(( $(read_ticks "$child_pid") - child_start ))
  child_rss_end=$(read_rss "$child_pid")
  printf 'Anuncio Avahi: %s ticks CPU · RSS %s → %s KiB · %s descriptores\n' \
    "$child_ticks" "$child_rss_start" "$child_rss_end" "$(read_fds "$child_pid")"
else
  printf 'Anuncio Avahi: no disponible; el daemon compartido no se incluye en esta medición.\n'
fi
