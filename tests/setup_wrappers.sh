#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
scratch=$(mktemp -d /tmp/seamlesscontrol-setup-XXXXXX)
trap 'rm -rf -- "$scratch"' EXIT
mkdir -m 700 -p "$scratch/home" "$scratch/mock"

cat >"$scratch/mock/omarchy" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'omarchy %s\n' "$*" >>"$SC_SETUP_LOG"
plugin="$HOME/.config/omarchy/plugins/seamlesscontrol.control"
case "$1 $2" in
  'plugin add')
    mkdir -p "$plugin/packaging"
    git -C "$plugin" init -q
    git -C "$plugin" remote add origin https://github.com/PuroDelphi/seamlesscontrol.git
    cat >"$plugin/packaging/install-agent.sh" <<'SCRIPT'
#!/usr/bin/env bash
printf 'agent-install\n' >>"$SC_SETUP_LOG"
SCRIPT
    cat >"$plugin/packaging/uninstall-agent.sh" <<'SCRIPT'
#!/usr/bin/env bash
if [[ ${SC_FAIL_UNINSTALL:-0} == 1 ]]; then exit 1; fi
printf 'agent-uninstall\n' >>"$SC_SETUP_LOG"
SCRIPT
    ;;
  'plugin remove') rm -rf -- "$plugin" ;;
esac
MOCK
chmod 755 "$scratch/mock/omarchy"

env HOME="$scratch/home" PATH="$scratch/mock:$PATH" SC_SETUP_LOG="$scratch/actions" \
  bash "$repo_dir/packaging/bootstrap.sh" >"$scratch/first.out"
env HOME="$scratch/home" PATH="$scratch/mock:$PATH" SC_SETUP_LOG="$scratch/actions" \
  bash "$repo_dir/packaging/bootstrap.sh" >"$scratch/second.out"
if env HOME="$scratch/home" PATH="$scratch/mock:$PATH" SC_SETUP_LOG="$scratch/actions" \
  SC_FAIL_UNINSTALL=1 bash "$repo_dir/packaging/remove.sh" >"$scratch/blocked.out" 2>&1; then
  printf 'La retirada eliminó el plugin pese al fallo del agente\n' >&2
  exit 1
fi
[[ -d "$scratch/home/.config/omarchy/plugins/seamlesscontrol.control" ]]
env HOME="$scratch/home" PATH="$scratch/mock:$PATH" SC_SETUP_LOG="$scratch/actions" \
  bash "$repo_dir/packaging/remove.sh" >"$scratch/remove.out"

expected=$(cat <<'EXPECTED'
omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable --yes
agent-install
omarchy plugin update seamlesscontrol.control --yes
omarchy plugin enable seamlesscontrol.control
agent-install
agent-uninstall
omarchy plugin remove seamlesscontrol.control --yes
EXPECTED
)
[[ "$(cat "$scratch/actions")" == "$expected" ]]
[[ ! -e "$scratch/home/.config/omarchy/plugins/seamlesscontrol.control" ]]
printf 'Instalación, actualización y retirada con un comando cada una: correcto\n'
