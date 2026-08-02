#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/../.." && pwd)"
cd "$repo_root"

package_version="$(node -p "require('./package.json').version")"
tauri_version="$(node -p "require('./src-tauri/tauri.conf.json').version")"
cargo_version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' src-tauri/Cargo.toml | head -1)"
lock_version="$(awk '
  $0 == "name = \"writing-environment\"" { found = 1; next }
  found && /^version = / { gsub(/^version = \"|\"$/, ""); print; exit }
' src-tauri/Cargo.lock)"

for version_entry in \
  "src-tauri/tauri.conf.json:$tauri_version" \
  "src-tauri/Cargo.toml:$cargo_version" \
  "src-tauri/Cargo.lock:$lock_version"; do
  version_file="${version_entry%%:*}"
  version_value="${version_entry#*:}"
  if [[ "$version_value" != "$package_version" ]]; then
    printf 'Version mismatch: package.json is %s but %s is %s.\n' \
      "$package_version" "$version_file" "$version_value" >&2
    exit 1
  fi
done

if [[ -n "${RELEASE_TAG:-}" && "$RELEASE_TAG" != "v${package_version}" ]]; then
  printf 'Release tag %s does not match application version v%s.\n' \
    "$RELEASE_TAG" "$package_version" >&2
  exit 1
fi

if ! grep -Fq "Version ${package_version}" .github/release-notes.md; then
  printf 'Release notes do not identify Version %s.\n' "$package_version" >&2
  exit 1
fi

printf 'Release metadata is consistent for v%s.\n' "$package_version"
