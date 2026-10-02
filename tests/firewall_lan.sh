#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT

mkdir -p "$scratch/bin"
cat >"$scratch/bin/seamlesscontrold" <<'EOF'
#!/usr/bin/env bash
[[ $1 == local-address && $2 == 47832 ]]
printf '192.168.1.25:47832\n'
EOF
cat >"$scratch/bin/ip" <<'EOF'
#!/usr/bin/env bash
case "$*" in
  '-4 -o addr show scope global') printf '2: wlo1 inet 192.168.1.25/24 brd 192.168.1.255 scope global\n' ;;
  '-4 route show dev wlo1 scope link') printf '192.168.1.0/24 proto kernel src 192.168.1.25\n' ;;
  *) exit 1 ;;
esac
EOF
cat >"$scratch/bin/ufw" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
cat >"$scratch/bin/sudo" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$FIREWALL_TEST_LOG"
EOF
cat >"$scratch/bin/pkexec" <<'EOF'
#!/usr/bin/env bash
printf 'POLKIT %s\n' "$*" >>"$FIREWALL_TEST_LOG"
EOF
chmod +x "$scratch/bin/"*
export PATH="$scratch/bin:$PATH"
export FIREWALL_TEST_LOG="$scratch/commands"
unset DISPLAY WAYLAND_DISPLAY

output=$(bash "$repo_dir/packaging/firewall-lan.sh" show)
[[ $output == *'ufw allow in on wlo1 from 192.168.1.0/24 to 192.168.1.25 port 47832 proto tcp comment seamlesscontrol'* ]]
printf 'n\n' | bash "$repo_dir/packaging/firewall-lan.sh" allow >/dev/null
[[ ! -e $FIREWALL_TEST_LOG ]]
printf 's\n' | bash "$repo_dir/packaging/firewall-lan.sh" allow >/dev/null
rg -q 'ufw allow in on wlo1 from 192.168.1.0/24 to 192.168.1.25 port 47832 proto tcp comment seamlesscontrol' "$FIREWALL_TEST_LOG"
printf 's\n' | bash "$repo_dir/packaging/firewall-lan.sh" remove >/dev/null
rg -q 'ufw delete allow in on wlo1 from 192.168.1.0/24 to 192.168.1.25 port 47832 proto tcp comment seamlesscontrol' "$FIREWALL_TEST_LOG"
WAYLAND_DISPLAY=wayland-1 bash "$repo_dir/packaging/firewall-lan.sh" allow 47832 --yes </dev/null >/dev/null
rg -q 'POLKIT .*/ufw allow in on wlo1 from 192.168.1.0/24 to 192.168.1.25 port 47832 proto tcp comment seamlesscontrol' "$FIREWALL_TEST_LOG"
printf 'Confirmación y límites de la regla UFW: correcto.\n'
