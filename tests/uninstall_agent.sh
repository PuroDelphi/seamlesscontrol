#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
"$cargo_bin" build --release --quiet --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
scratch=$(mktemp -d /tmp/seamlesscontrol-uninstall-XXXXXX)
server_pid=
cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill -INT "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT

fake_home="$scratch/home"
bin="$fake_home/.local/bin/seamlesscontrold"
unit_dir="$fake_home/.config/systemd/user"
plugin_dir="$fake_home/.config/omarchy/plugins/seamlesscontrol.control"
mkdir -m 700 -p "$fake_home/.local/bin" "$unit_dir" "$plugin_dir" "$scratch/run" "$scratch/mock"
install -m 755 "$repo_dir/agent/target/release/seamlesscontrold" "$bin"
cp "$repo_dir"/packaging/seamlesscontrol-*.service "$unit_dir/"
printf 'widget conservado\n' >"$plugin_dir/marker"
cat >"$scratch/mock/systemctl" <<'MOCK'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$SC_SYSTEMCTL_LOG"
MOCK
chmod 755 "$scratch/mock/systemctl"

port=$((49000 + RANDOM % 8000))
HOME="$fake_home" XDG_CONFIG_HOME="$fake_home/.config" XDG_RUNTIME_DIR="$scratch/run" \
  "$bin" serve "127.0.0.1:$port" >"$scratch/server.log" 2>&1 &
server_pid=$!
for _ in {1..100}; do
  if XDG_RUNTIME_DIR="$scratch/run" "$bin" status >/dev/null 2>&1; then break; fi
  sleep 0.05
done
XDG_RUNTIME_DIR="$scratch/run" "$bin" status >/dev/null
identity="$fake_home/.config/seamlesscontrol/identity"
before=$(sha256sum "$identity" | cut -d ' ' -f1)

if HOME="$fake_home" XDG_CONFIG_HOME="$fake_home/.config" XDG_RUNTIME_DIR="$scratch/run" \
  SC_SYSTEMCTL_LOG="$scratch/systemctl.log" PATH="$scratch/mock:$PATH" \
  bash "$repo_dir/packaging/uninstall-agent.sh" >"$scratch/blocked.log" 2>&1; then
  printf 'El desinstalador quitó un agente activo\n' >&2
  exit 1
fi
[[ -x "$bin" ]]
[[ -f "$plugin_dir/marker" ]]
[[ "$(sha256sum "$identity" | cut -d ' ' -f1)" == "$before" ]]

kill -INT "$server_pid"
wait "$server_pid"
server_pid=
HOME="$fake_home" XDG_CONFIG_HOME="$fake_home/.config" XDG_RUNTIME_DIR="$scratch/run" \
  SC_SYSTEMCTL_LOG="$scratch/systemctl.log" PATH="$scratch/mock:$PATH" \
  bash "$repo_dir/packaging/uninstall-agent.sh" >"$scratch/uninstall.log" 2>&1
[[ ! -e "$bin" ]]
[[ -z "$(find "$unit_dir" -maxdepth 1 -name 'seamlesscontrol-*.service' -print)" ]]
[[ -f "$plugin_dir/marker" ]]
[[ "$(sha256sum "$identity" | cut -d ' ' -f1)" == "$before" ]]
[[ $(wc -l <"$scratch/systemctl.log") -eq 5 ]]

custom_bin="$scratch/custom-bin"
mkdir -m 700 "$custom_bin"
install -m 755 "$repo_dir/agent/target/release/seamlesscontrold" "$custom_bin/seamlesscontrold"
HOME="$fake_home" XDG_CONFIG_HOME="$fake_home/.config" XDG_RUNTIME_DIR="$scratch/run" \
  XDG_BIN_HOME="$custom_bin" SC_SYSTEMCTL_LOG="$scratch/systemctl.log" \
  PATH="$scratch/mock:$PATH" bash "$repo_dir/packaging/uninstall-agent.sh" \
  >"$scratch/custom-uninstall.log" 2>&1
[[ ! -e "$custom_bin/seamlesscontrold" ]]
[[ $(wc -l <"$scratch/systemctl.log") -eq 5 ]]
[[ "$(sha256sum "$identity" | cut -d ' ' -f1)" == "$before" ]]
printf 'Retirada segura de unidades y binario; identidad y widget conservados: correcto\n'
