#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
scratch=$(mktemp -d /tmp/seamlesscontrol-setup-result-XXXXXXXX)
trap 'rm -rf -- "$scratch"' EXIT
mkdir -p "$scratch/repo/packaging" "$scratch/home" "$scratch/bin"
cp "$repo_dir/packaging/setup-agent.sh" "$scratch/repo/packaging/setup-agent.sh"
cat > "$scratch/repo/packaging/install-agent.sh" <<'STUB'
#!/usr/bin/env bash
if [[ ${SIMULATE_INSTALL_FAILURE:-0} == 1 ]]; then exit 7; fi
mkdir -p "$XDG_BIN_HOME"
cat > "$XDG_BIN_HOME/seamlesscontrold" <<'AGENT'
#!/usr/bin/env bash
[[ ${1:-} == version ]] && printf '0.24.0\n'
AGENT
chmod 755 "$XDG_BIN_HOME/seamlesscontrold"
STUB
cat > "$scratch/repo/packaging/uninstall-agent.sh" <<'STUB'
#!/usr/bin/env bash
rm -f -- "$XDG_BIN_HOME/seamlesscontrold"
STUB

run_setup() {
  printf '\n' | HOME="$scratch/home" XDG_STATE_HOME="$scratch/state" \
    XDG_BIN_HOME="$scratch/bin" SIMULATE_INSTALL_FAILURE="${SIMULATE_INSTALL_FAILURE:-0}" \
    bash "$scratch/repo/packaging/setup-agent.sh" "$1" en > "$scratch/output" 2>&1
}

run_setup install
IFS=$'\t' read -r status version timestamp < "$scratch/state/seamlesscontrol/setup-result"
[[ $status == installed && $version == 0.24.0 && $timestamp =~ ^[0-9]{13}$ ]]

SIMULATE_INSTALL_FAILURE=1
if run_setup install; then
  printf 'Expected installation failure\n' >&2
  exit 1
fi
IFS=$'\t' read -r status version timestamp < "$scratch/state/seamlesscontrol/setup-result"
[[ $status == failed && $version == - && $timestamp =~ ^[0-9]{13}$ ]]

unset SIMULATE_INSTALL_FAILURE
run_setup remove
IFS=$'\t' read -r status version timestamp < "$scratch/state/seamlesscontrol/setup-result"
[[ $status == removed && $version == - && $timestamp =~ ^[0-9]{13}$ ]]
[[ ! -e "$scratch/bin/seamlesscontrold" ]]
printf 'Setup result records success, failure and removal: correct.\n'
