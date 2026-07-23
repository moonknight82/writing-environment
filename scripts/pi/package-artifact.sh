#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
readonly script_dir
readonly repo_root
readonly output_dir="${1:-}"
readonly build_id="${2:-$(date +%Y%m%d-%H%M%S)}"
readonly deb_bundle="$(find "$repo_root/src-tauri/target/release/bundle/deb" -maxdepth 1 -type f -name '*.deb' -print -quit 2>/dev/null)"

[[ -n "$output_dir" ]] || { printf 'Usage: scripts/pi/package-artifact.sh OUTPUT_DIR [BUILD_ID]\n' >&2; exit 2; }
[[ "$build_id" =~ ^[A-Za-z0-9._-]+$ ]] || { printf 'BUILD_ID may contain only letters, numbers, dots, underscores, and hyphens.\n' >&2; exit 2; }
[[ "$(uname -s)" == "Linux" ]] || { printf 'Artifacts must be packaged from Linux.\n' >&2; exit 1; }
[[ "$(uname -m)" == "aarch64" ]] || { printf 'Expected an aarch64 build host.\n' >&2; exit 1; }
[[ -f "$deb_bundle" ]] || { printf 'Raspberry Pi Debian bundle not found. Build with --bundles deb first.\n' >&2; exit 1; }
require_command() { command -v "$1" >/dev/null 2>&1 || { printf '%s is required.\n' "$1" >&2; exit 1; }; }
require_command dpkg-deb

version="$(sed -n 's/^[[:space:]]*"version": "\([^"]*\)".*/\1/p' "$repo_root/src-tauri/tauri.conf.json" | head -n 1)"
[[ -n "$version" ]] || { printf 'Could not read the application version.\n' >&2; exit 1; }

readonly artifact_name="writing-environment-pi-arm64-${version}-${build_id}"
readonly archive="$output_dir/$artifact_name.tar.gz"
readonly checksum="$archive.sha256"
[[ ! -e "$archive" && ! -e "$checksum" ]] || { printf 'Refusing to overwrite an existing artifact.\n' >&2; exit 1; }

staging_parent="$(mktemp -d)"
trap 'rm -rf "$staging_parent"' EXIT
staging="$staging_parent/$artifact_name"
mkdir -p "$staging" "$output_dir"

deb_extract="$staging_parent/deb-root"
mkdir -p "$deb_extract"
dpkg-deb -x "$deb_bundle" "$deb_extract"
binary="$deb_extract/usr/bin/writing-environment"
[[ -x "$binary" ]] || { printf 'The Debian bundle does not contain /usr/bin/writing-environment.\n' >&2; exit 1; }
file "$binary" | grep -qE 'ELF 64-bit.*(ARM aarch64|aarch64)' || { printf 'Release binary is not Linux ARM64.\n' >&2; exit 1; }

install -m 755 "$binary" "$staging/writing-environment"
install -m 755 "$repo_root/deploy/pi/writing-environment" "$staging/launcher"
install -m 755 "$repo_root/scripts/pi/install-prebuilt.sh" "$staging/install.sh"
install -m 644 "$repo_root/scripts/pi/lib.sh" "$staging/lib.sh"
install -m 644 "$repo_root/deploy/pi/writing-environment.desktop" "$staging/writing-environment.desktop"
install -m 644 "$repo_root/deploy/pi/environment.example" "$staging/environment.example"
install -m 644 "$repo_root/design/app-icon.svg" "$staging/app-icon.svg"

{
  printf 'format=1\n'
  printf 'version=%s\n' "$version"
  printf 'architecture=aarch64\n'
  printf 'linux_baseline=debian-bookworm\n'
  printf 'build_id=%s\n' "$build_id"
  printf 'update_bundle=deb\n'
} >"$staging/manifest.txt"

(
  cd "$staging"
  checksum_inputs="$staging_parent/checksum-inputs"
  find . -maxdepth 1 -type f ! -name SHA256SUMS -printf '%P\n' | LC_ALL=C sort >"$checksum_inputs"
  xargs sha256sum <"$checksum_inputs" >SHA256SUMS
)

tar -czf "$archive" -C "$staging_parent" "$artifact_name"
(
  cd "$output_dir"
  sha256sum "$(basename "$archive")" >"$(basename "$checksum")"
)

printf 'Raspberry Pi ARM64 artifact written to:\n%s\n%s\n' "$archive" "$checksum"
