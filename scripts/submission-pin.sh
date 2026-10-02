#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

if ! git diff --quiet || ! git diff --cached --quiet || [[ -n $(git ls-files --others --exclude-standard) ]]; then
  printf '%s\n' 'Commit every release change before preparing a marketplace SHA.' >&2
  exit 1
fi

local_sha=$(git rev-parse HEAD)
remote_sha=$(git ls-remote origin refs/heads/main | awk '{print $1}')
if [[ ! $local_sha =~ ^[0-9a-f]{40}$ || $remote_sha != "$local_sha" ]]; then
  printf '%s\n' 'Local HEAD and the published main branch differ. Push or fetch before submission.' >&2
  exit 1
fi

omarchy plugin validate .
cargo build --release --locked --manifest-path agent/Cargo.toml --bin seamlesscontrold
printf 'Marketplace commit SHA: %s\n' "$local_sha"
printf 'Cargo.lock SHA256: %s\n' "$(sha256sum agent/Cargo.lock | cut -d ' ' -f 1)"
