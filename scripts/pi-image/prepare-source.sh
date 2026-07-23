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

artifact=""
output=""

usage() {
  cat <<'EOF'
Usage: scripts/pi-image/prepare-source.sh --artifact FILE --output DIRECTORY

Prepare a self-contained rpi-image-gen source directory. The application
artifact must be a Writing Environment Pi ARM64 .tar.gz package.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --artifact) artifact="${2:-}"; shift 2 ;;
    --output) output="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
  esac
done

[[ -f "$artifact" ]] || { printf 'Pi ARM64 artifact not found: %s\n' "$artifact" >&2; exit 1; }
[[ -n "$output" ]] || { printf -- '--output is required.\n' >&2; exit 2; }
[[ ! -e "$output" ]] || { printf 'Refusing to overwrite existing output: %s\n' "$output" >&2; exit 1; }

case "$artifact" in
  *.tar.gz) ;;
  *) printf 'Expected a .tar.gz Pi ARM64 artifact.\n' >&2; exit 2 ;;
esac

temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT

plugin_output="$temporary/settings-plugin"
"$script_dir/build-settings-plugin.sh" --output "$plugin_output"

if tar -tzf "$artifact" | awk '
  /^\// || /(^|\/)\.\.($|\/)/ { unsafe = 1 }
  END { exit unsafe ? 0 : 1 }
'; then
  printf 'The application artifact contains an unsafe path.\n' >&2
  exit 1
fi

artifact_extract="$temporary/application"
mkdir -p "$artifact_extract"
tar -xzf "$artifact" -C "$artifact_extract"
artifact_root="$(find "$artifact_extract" -mindepth 1 -maxdepth 1 -type d -print -quit)"
[[ -n "$artifact_root" && -x "$artifact_root/writing-environment" ]] || {
  printf 'The application artifact does not contain its ARM64 executable.\n' >&2
  exit 1
}
(
  cd "$artifact_root"
  sha256sum -c SHA256SUMS
)
file "$artifact_root/writing-environment" | grep -qE 'ELF 64-bit.*(ARM aarch64|aarch64)' || {
  printf 'The bundled application is not Linux ARM64.\n' >&2
  exit 1
}

mkdir -p "$output/config" "$output/layer" "$output/rootfs"
cp "$repo_root/deploy/pi-image/config/writing-environment.yaml" "$output/config/"
cp "$repo_root/deploy/pi-image/layer/writing-environment-desktop.yaml" "$output/layer/"
cp -a "$repo_root/deploy/pi-image/rootfs/." "$output/rootfs/"

mkdir -p \
  "$output/rootfs/opt/writing-environment" \
  "$output/rootfs/usr/bin" \
  "$output/rootfs/usr/local/bin" \
  "$output/rootfs/usr/local/share/applications" \
  "$output/rootfs/usr/local/share/icons/hicolor/scalable/apps" \
  "$output/rootfs/usr/local/share/writing-environment/packages" \
  "$output/rootfs/usr/lib/aarch64-linux-gnu/rpcc" \
  "$output/rootfs/etc/skel/.config/writing-environment"
install -m 0755 "$artifact_root/writing-environment" "$output/rootfs/usr/bin/writing-environment"
install -m 0755 "$repo_root/deploy/pi/writing-environment" "$output/rootfs/usr/local/bin/writing-environment"
install -m 0644 "$repo_root/deploy/pi/writing-environment.desktop" "$output/rootfs/usr/local/share/applications/writing-environment.desktop"
install -m 0644 "$repo_root/design/app-icon.svg" "$output/rootfs/usr/local/share/icons/hicolor/scalable/apps/writing-environment.svg"
install -m 0644 \
  "$plugin_output/librpcc_writing-environment-bluetooth.so" \
  "$output/rootfs/usr/lib/aarch64-linux-gnu/rpcc/librpcc_writing-environment-bluetooth.so"
install -m 0644 "$repo_root/deploy/pi/environment.example" "$output/rootfs/etc/skel/.config/writing-environment/environment"

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
printf 'app_artifact=%s\n' "$(basename "$artifact")" >"$output/rootfs/usr/share/writing-environment/image-manifest"
printf 'rclone_version=%s\n' "$rclone_version" >>"$output/rootfs/usr/share/writing-environment/image-manifest"
printf 'localsend_version=%s\n' "$localsend_version" >>"$output/rootfs/usr/share/writing-environment/image-manifest"

COPYFILE_DISABLE=1 tar --format=ustar -cf "$output/writing-environment-rootfs.tar" -C "$output/rootfs" .
printf 'Prepared rpi-image-gen source:\n%s\n' "$output"
