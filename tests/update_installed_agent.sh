#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
"$cargo_bin" build --release --quiet --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
scratch=$(mktemp -d /tmp/seamlesscontrol-update-XXXXXX)
agent="$scratch/bin/seamlesscontrold"
server_pid=
cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill -INT "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT
mkdir -m 700 -p "$scratch/bin" "$scratch/config" "$scratch/run"
install -m 755 "$repo_dir/agent/target/release/seamlesscontrold" "$agent"
old_inode=$(stat -c '%i' "$agent")

port=$((49000 + RANDOM % 8000))
XDG_CONFIG_HOME="$scratch/config" XDG_RUNTIME_DIR="$scratch/run" \
  "$agent" serve "127.0.0.1:$port" >"$scratch/server.log" 2>&1 &
server_pid=$!
for _ in {1..100}; do
  if XDG_RUNTIME_DIR="$scratch/run" "$agent" status >/dev/null 2>&1; then break; fi
  if ! kill -0 "$server_pid" 2>/dev/null; then
    cat "$scratch/server.log" >&2
    exit 1
  fi
  sleep 0.05
done
XDG_RUNTIME_DIR="$scratch/run" "$agent" status >/dev/null
identity="$scratch/config/seamlesscontrol/identity"
before=$(sha256sum "$identity" | cut -d ' ' -f1)

XDG_BIN_HOME="$scratch/bin" bash "$repo_dir/packaging/install-agent.sh" >"$scratch/install.log" 2>&1
new_inode=$(stat -c '%i' "$agent")
[[ "$new_inode" != "$old_inode" ]]
[[ "$(sha256sum "$agent" | cut -d ' ' -f1)" == \
   "$(sha256sum "$repo_dir/agent/target/release/seamlesscontrold" | cut -d ' ' -f1)" ]]
[[ "$(sha256sum "$identity" | cut -d ' ' -f1)" == "$before" ]]
kill -0 "$server_pid"
XDG_RUNTIME_DIR="$scratch/run" "$agent" status >/dev/null
if compgen -G "$scratch/bin/.seamlesscontrold.*" >/dev/null; then
  printf 'Quedó un archivo temporal del instalador\n' >&2
  exit 1
fi

kill -INT "$server_pid"
wait "$server_pid"
server_pid=
XDG_CONFIG_HOME="$scratch/config" XDG_RUNTIME_DIR="$scratch/run" \
  "$agent" serve "127.0.0.1:$port" >"$scratch/restarted.log" 2>&1 &
server_pid=$!
for _ in {1..100}; do
  if XDG_RUNTIME_DIR="$scratch/run" "$agent" status >/dev/null 2>&1; then break; fi
  sleep 0.05
done
XDG_RUNTIME_DIR="$scratch/run" "$agent" status >/dev/null
[[ "$(sha256sum "$identity" | cut -d ' ' -f1)" == "$before" ]]
printf 'Actualización atómica con receptor activo e identidad conservada: correcto\n'
