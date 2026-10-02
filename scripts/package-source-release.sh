#!/usr/bin/env bash
set -euo pipefail

tag=${1:?Usage: package-source-release.sh vX.Y.Z [OUTPUT_DIR]}
if [[ ! $tag =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'Expected a version tag such as v0.19.6\n' >&2
  exit 2
fi

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
output_dir=${2:-"$repo_dir/dist"}
mkdir -p -- "$output_dir"
output_dir=$(cd "$output_dir" && pwd)
commit=$(git -C "$repo_dir" rev-parse --verify "refs/tags/$tag^{commit}")
version=$(git -C "$repo_dir" show "$commit:manifest.json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["version"])')
if [[ $tag != "v$version" ]]; then
  printf 'Tag %s does not match manifest version %s\n' "$tag" "$version" >&2
  exit 1
fi

name="seamlesscontrol-$version-source.tar.gz"
git -C "$repo_dir" archive --format=tar --prefix="seamlesscontrol-$version/" "$commit" | gzip -n > "$output_dir/$name"
(
  cd "$output_dir"
  sha256sum "$name" > "$name.sha256"
  sha256sum --check "$name.sha256"
)
printf 'Commit: %s\nArchive: %s\nChecksum: %s\n' "$commit" "$output_dir/$name" "$output_dir/$name.sha256"
