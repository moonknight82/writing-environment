#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repo_root="$(cd "$script_dir/../.." && pwd)"
readonly rclone_version="1.74.4"
readonly rclone_archive="rclone-v${rclone_version}-linux-arm64.zip"
readonly rclone_sha256="97685285c9ad6a0cf17d5844115d2a67245af6444db672187074bd9c358de419"
readonly localsend_version="1.17.0"
readonly localsend_package="LocalSend-${localsend_version}-linux-arm-64.deb"
readonly localsend_sha256="c2c792aadabeecf864f4105f8b1f8693941bc752fda582f0d2b3794765fcf803"

app_deb=""
packages=""
output=""

usage() {
  cat <<'EOF'
Usage: scripts/pi-image/prepare-source.sh --app-deb FILE --packages DIRECTORY --output DIRECTORY

Prepare a self-contained rpi-image-gen source directory. The application
package and the four appliance packages must be Debian packages built for the
signed Writing Environment APT release.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --app-deb) app_deb="${2:-}"; shift 2 ;;
    --packages) packages="${2:-}"; shift 2 ;;
    --output) output="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
  esac
done

[[ -f "$app_deb" ]] || { printf 'Pi ARM64 Debian package not found: %s\n' "$app_deb" >&2; exit 1; }
[[ -d "$packages" ]] || { printf 'Appliance package directory not found: %s\n' "$packages" >&2; exit 1; }
[[ -n "$output" ]] || { printf -- '--output is required.\n' >&2; exit 2; }
[[ ! -e "$output" ]] || { printf 'Refusing to overwrite existing output: %s\n' "$output" >&2; exit 1; }

temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT

file "$app_deb" | grep -q 'Debian binary package' || {
  printf 'The selected application file is not a Debian package.\n' >&2
  exit 1
}
package_count="$(find "$packages" -maxdepth 1 -type f -name '*.deb' | wc -l | tr -d ' ')"
[[ "$package_count" -ge 5 ]] || {
  printf 'Expected at least five application/appliance packages, found %s.\n' "$package_count" >&2
  exit 1
}

mkdir -p "$output/config" "$output/layer" "$output/rootfs"
cp "$repo_root/deploy/pi-image/config/writing-environment.yaml" "$output/config/"
cp "$repo_root/deploy/pi-image/layer/writing-environment-desktop.yaml" "$output/layer/"
cp -a "$repo_root/deploy/pi-image/rootfs/." "$output/rootfs/"

mkdir -p \
  "$output/rootfs/usr/local/bin" \
  "$output/rootfs/usr/local/share/writing-environment/packages" \
  "$output/rootfs/etc/skel/.config/writing-environment"
for package_file in "$packages"/*.deb; do
  install -m 0644 "$package_file" "$output/rootfs/usr/local/share/writing-environment/packages/"
done

download="$temporary/$rclone_archive"
curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location \
  "https://downloads.rclone.org/v${rclone_version}/$rclone_archive" \
  --output "$download"
printf '%s  %s\n' "$rclone_sha256" "$download" | shasum -a 256 -c -
unzip -q "$download" -d "$temporary/rclone"
rclone_binary="$(find "$temporary/rclone" -type f -name rclone -print -quit)"
[[ -n "$rclone_binary" ]] || { printf 'The verified rclone archive did not contain rclone.\n' >&2; exit 1; }
install -m 0755 "$rclone_binary" "$output/rootfs/usr/local/bin/rclone"

localsend_download="$temporary/$localsend_package"
curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location \
  "https://github.com/localsend/localsend/releases/download/v${localsend_version}/${localsend_package}" \
  --output "$localsend_download"
printf '%s  %s\n' "$localsend_sha256" "$localsend_download" | shasum -a 256 -c -
file "$localsend_download" | grep -q 'Debian binary package' || {
  printf 'The verified LocalSend download is not a Debian package.\n' >&2
  exit 1
}
install -m 0644 \
  "$localsend_download" \
  "$output/rootfs/usr/local/share/writing-environment/packages/localsend-arm64.deb"

mkdir -p "$output/rootfs/usr/share/writing-environment"
printf 'app_package=%s\n' "$(basename "$app_deb")" >"$output/rootfs/usr/share/writing-environment/image-manifest"
printf 'rclone_version=%s\n' "$rclone_version" >>"$output/rootfs/usr/share/writing-environment/image-manifest"
printf 'localsend_version=%s\n' "$localsend_version" >>"$output/rootfs/usr/share/writing-environment/image-manifest"

COPYFILE_DISABLE=1 tar --format=ustar -cf "$output/writing-environment-rootfs.tar" -C "$output/rootfs" .
printf 'Prepared rpi-image-gen source:\n%s\n' "$output"
