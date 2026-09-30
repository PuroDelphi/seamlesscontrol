#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
"$cargo_bin" build --quiet --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
agent="$repo_dir/agent/target/debug/seamlesscontrold"

for program in avahi-browse avahi-publish-service; do
  if ! command -v "$program" >/dev/null; then
    printf 'OMITIDO: falta %s\n' "$program"
    exit 0
  fi
done
if ! avahi-browse --terminate --parsable _seamlesscontrol._tcp >/dev/null 2>&1; then
  printf 'OMITIDO: avahi-daemon no está disponible\n'
  exit 0
fi

scratch=$(mktemp -d /tmp/seamlesscontrol-discovery-XXXXXX)
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

port=$((49000 + RANDOM % 10000))
address=$(
  XDG_CONFIG_HOME="$scratch/config" XDG_RUNTIME_DIR="$scratch/run" \
    "$agent" local-address "$port"
)
ip=${address%:*}
XDG_CONFIG_HOME="$scratch/config" XDG_RUNTIME_DIR="$scratch/run" \
  "$agent" serve-auto "$port" >"$scratch/server.log" 2>&1 &
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

fingerprint=$(awk '/^Identidad local: / { print $3; exit }' "$scratch/server.log")
[[ "$fingerprint" =~ ^[0-9a-f]{64}$ ]]

found=
for _ in {1..40}; do
  found=$(
    "$agent" discover --include-local |
      awk -F '\t' -v ip="$ip" -v port="$port" -v key="$fingerprint" \
        '$1 == "FOUND" && $3 == ip && $4 == port && $5 == key { print; exit }'
  )
  if [[ -n "$found" ]]; then break; fi
  sleep 0.25
done
[[ -n "$found" ]] || {
  printf 'El anuncio del receptor no apareció en Avahi\n' >&2
  cat "$scratch/server.log" >&2
  exit 1
}

normal=$("$agent" discover)
if printf '%s\n' "$normal" |
  awk -F '\t' -v key="$fingerprint" '$1 == "FOUND" && $5 == key { found = 1 } END { exit !found }'; then
  printf 'La búsqueda normal mostró el receptor de este mismo equipo\n' >&2
  exit 1
fi

kill -INT "$server_pid"
wait "$server_pid"
server_pid=

for _ in {1..40}; do
  remaining=$("$agent" discover --include-local)
  if ! printf '%s\n' "$remaining" |
    awk -F '\t' -v key="$fingerprint" '$1 == "FOUND" && $5 == key { found = 1 } END { exit !found }'; then
    printf 'Anuncio, detección, filtro local y retirada mDNS: correcto\n'
    exit 0
  fi
  sleep 0.25
done
printf 'El anuncio mDNS permaneció después de detener el receptor\n' >&2
exit 1
