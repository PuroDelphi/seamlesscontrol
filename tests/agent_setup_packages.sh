#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
if [[ ! -x "$repo_dir/agent/target/release/seamlesscontrold" ]]; then
  cargo build --release --quiet --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
fi
scratch=$(mktemp -d /tmp/seamlesscontrol-setup-XXXXXX)
trap 'rm -rf -- "$scratch"' EXIT
fake_home="$scratch/home"
mkdir -p "$fake_home" "$scratch/mock" "$scratch/run"
custom_unit="$fake_home/.config/systemd/user/seamlesscontrol-sender.service"
mkdir -p "$(dirname "$custom_unit")"
printf '[Unit]\nDescription=User custom service\n' >"$custom_unit"
for command in dirname install mktemp rm mv chmod touch grep cmp; do
  ln -s "$(command -v "$command")" "$scratch/mock/$command"
done
cat >"$scratch/mock/cargo" <<'MOCK'
#!/usr/bin/bash
if [[ $1 == --version ]]; then printf 'cargo test stub\n'; exit 0; fi
[[ $1 == build ]]
MOCK
cat >"$scratch/mock/pacman" <<'MOCK'
#!/usr/bin/bash
[[ $1 == -Q ]] || exit 2
grep -Fxq "$2" "$SC_PACKAGE_DB"
MOCK
cat >"$scratch/mock/omarchy" <<'MOCK'
#!/usr/bin/bash
printf '%s\n' "$*" >> "$SC_OMARCHY_LOG"
if [[ $1 == pkg && $2 == add ]]; then
  shift 2
  printf '%s\n' "$@" >> "$SC_PACKAGE_DB"
fi
MOCK
cat >"$scratch/mock/systemctl" <<'MOCK'
#!/usr/bin/bash
if [[ $1 == is-active ]]; then exit 0; fi
MOCK
chmod 755 "$scratch/mock"/{cargo,pacman,omarchy,systemctl}
: > "$scratch/packages"

for _ in 1 2; do
  HOME="$fake_home" XDG_CONFIG_HOME="$fake_home/.config" \
    XDG_STATE_HOME="$fake_home/.local/state" XDG_RUNTIME_DIR="$scratch/run" \
    SC_PACKAGE_DB="$scratch/packages" SC_OMARCHY_LOG="$scratch/omarchy.log" \
    PATH="$scratch/mock" /usr/bin/bash "$repo_dir/packaging/install-agent.sh" >/dev/null
done
marker="$fake_home/.local/state/seamlesscontrol/installed-packages"
[[ $(sort "$marker") == $'avahi\nwl-clipboard' ]]
[[ -x "$fake_home/.local/bin/seamlesscontrold" ]]
[[ $(cat "$custom_unit") == $'[Unit]\nDescription=User custom service' ]]

HOME="$fake_home" XDG_CONFIG_HOME="$fake_home/.config" \
  XDG_STATE_HOME="$fake_home/.local/state" XDG_RUNTIME_DIR="$scratch/run" \
  SC_PACKAGE_DB="$scratch/packages" SC_OMARCHY_LOG="$scratch/omarchy.log" \
  PATH="$scratch/mock" /usr/bin/bash "$repo_dir/packaging/uninstall-agent.sh" --remove-deps >/dev/null
[[ ! -e "$marker" && ! -e "$fake_home/.local/bin/seamlesscontrold" ]]
[[ $(cat "$custom_unit") == $'[Unit]\nDescription=User custom service' ]]
[[ $(tail -1 "$scratch/omarchy.log") == 'pkg drop avahi wl-clipboard' ]]
printf 'Only packages first installed by SeamlessControl are tracked and removed: correct.\n'
