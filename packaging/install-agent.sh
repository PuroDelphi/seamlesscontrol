#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cargo_bin=${CARGO:-cargo}
install_dir=${XDG_BIN_HOME:-"$HOME/.local/bin"}

"$cargo_bin" build --release --manifest-path "$repo_dir/agent/Cargo.toml" --bin seamlesscontrold
install -d -m 755 "$install_dir"
install -m 755 "$repo_dir/agent/target/release/seamlesscontrold" "$install_dir/seamlesscontrold"

if [[ "$install_dir" == "$HOME/.local/bin" ]]; then
  unit_dir="$HOME/.config/systemd/user"
  install -d -m 755 "$unit_dir"
  install -m 644 "$repo_dir/packaging/seamlesscontrol-receiver.service" "$unit_dir/"
  install -m 644 "$repo_dir/packaging/seamlesscontrol-sender.service" "$unit_dir/"
  install -m 644 "$repo_dir/packaging/seamlesscontrol-mesh.service" "$unit_dir/"
  systemctl --user daemon-reload
  printf 'Unidades de usuario instaladas, sin habilitar. Configure las direcciones y el puerto antes de activarlas.\n'
fi

printf 'Agente instalado en %s/seamlesscontrold\n' "$install_dir"
printf 'Añada ese directorio a PATH antes de abrir el widget.\n'
