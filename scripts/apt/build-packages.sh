#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repo_root="$(cd "$script_dir/../.." && pwd)"
readonly templates="$repo_root/deploy/apt-repository/packages"

app_deb=""
settings_plugin=""
display_settings=""
public_key=""
output=""
revision="1"

usage() {
  cat <<'EOF'
Usage: scripts/apt/build-packages.sh OPTIONS

Build the Writing Environment appliance Debian packages.

Required options:
  --app-deb FILE          Tauri writing-environment ARM64 Debian package
  --settings-plugin FILE  ARM64 Raspberry Pi Control Centre plugin
  --display-settings FILE ARM64 standalone Display Settings launcher
  --public-key FILE       ASCII-armored APT repository public key
  --output DIRECTORY      New output directory for all Debian packages

Optional:
  --revision NUMBER       Debian packaging revision (default: 1)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --app-deb) app_deb="${2:-}"; shift 2 ;;
    --settings-plugin) settings_plugin="${2:-}"; shift 2 ;;
    --display-settings) display_settings="${2:-}"; shift 2 ;;
    --public-key) public_key="${2:-}"; shift 2 ;;
    --output) output="${2:-}"; shift 2 ;;
    --revision) revision="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
  esac
done

[[ -f "$app_deb" ]] || { printf 'Application Debian package not found: %s\n' "$app_deb" >&2; exit 1; }
[[ -f "$settings_plugin" ]] || { printf 'Settings plugin not found: %s\n' "$settings_plugin" >&2; exit 1; }
[[ -f "$display_settings" ]] || { printf 'Display Settings launcher not found: %s\n' "$display_settings" >&2; exit 1; }
[[ -f "$public_key" ]] || { printf 'APT public key not found: %s\n' "$public_key" >&2; exit 1; }
[[ -n "$output" ]] || { printf -- '--output is required.\n' >&2; exit 2; }
[[ "$revision" =~ ^[1-9][0-9]*$ ]] || { printf 'Invalid Debian revision: %s\n' "$revision" >&2; exit 2; }
[[ ! -e "$output" ]] || { printf 'Refusing to overwrite existing output: %s\n' "$output" >&2; exit 1; }

for command_name in dpkg-deb file gpg install sed; do
  command -v "$command_name" >/dev/null 2>&1 || {
    printf 'Required package-build tool is missing: %s\n' "$command_name" >&2
    exit 1
  }
done

app_package="$(dpkg-deb -f "$app_deb" Package)"
app_version="$(dpkg-deb -f "$app_deb" Version)"
app_architecture="$(dpkg-deb -f "$app_deb" Architecture)"
[[ "$app_package" == "writing-environment" ]] || { printf 'Unexpected app package: %s\n' "$app_package" >&2; exit 1; }
[[ "$app_architecture" == "arm64" ]] || { printf 'Expected an ARM64 app package, found %s.\n' "$app_architecture" >&2; exit 1; }
[[ "$app_version" =~ ^[0-9]+.[0-9]+.[0-9]+([+~.-][A-Za-z0-9.+~-]+)?$ ]] || {
  printf 'Unexpected application version: %s\n' "$app_version" >&2
  exit 1
}
file "$settings_plugin" | grep -qE 'ELF 64-bit.*(ARM aarch64|aarch64)' || {
  printf 'The settings plugin is not Linux ARM64.\n' >&2
  exit 1
}
file "$display_settings" | grep -qE 'ELF 64-bit.*(ARM aarch64|aarch64)' || {
  printf 'The Display Settings launcher is not Linux ARM64.\n' >&2
  exit 1
}

readonly package_version="${app_version}-${revision}"
readonly staging="$(mktemp -d /tmp/writing-environment-debian.XXXXXX)"
trap 'rm -rf -- "$staging"' EXIT
mkdir -p "$output"

render_control() {
  local package_name="$1" package_root="$2"
  sed \
    -e "s/@PACKAGE_VERSION@/$package_version/g" \
    -e "s/@APP_VERSION@/$app_version/g" \
    "$templates/$package_name.control" >"$package_root/DEBIAN/control"
}

build_package() {
  local package_name="$1"
  local architecture="$2"
  local package_root="$staging/$package_name"
  local package_file="$output/${package_name}_${package_version}_${architecture}.deb"
  render_control "$package_name" "$package_root"
  chmod 0755 "$package_root/DEBIAN"
  find "$package_root/DEBIAN" -maxdepth 1 -type f -name 'post*' -exec chmod 0755 {} +
  dpkg-deb --build --root-owner-group "$package_root" "$package_file" >/dev/null
  printf 'Built %s\n' "$package_file"
}

shell_root="$staging/writing-environment-shell"
mkdir -p \
  "$shell_root/DEBIAN" \
  "$shell_root/etc/dconf/db/local.d" \
  "$shell_root/etc/writing-environment" \
  "$shell_root/usr/lib/aarch64-linux-gnu/rpcc" \
  "$shell_root/usr/local/bin" \
  "$shell_root/usr/local/share/applications" \
  "$shell_root/usr/local/share/icons/hicolor/scalable/apps"

install -m 0644 "$templates/writing-environment-shell.conffiles" "$shell_root/DEBIAN/conffiles"
install -m 0755 "$templates/writing-environment-shell.postinst" "$shell_root/DEBIAN/postinst"
install -m 0755 "$templates/writing-environment-shell.postrm" "$shell_root/DEBIAN/postrm"
install -m 0644 "$repo_root/deploy/pi-image/rootfs/etc/dconf/db/local.d/00-writing-environment" "$shell_root/etc/dconf/db/local.d/00-writing-environment"
install -m 0644 "$repo_root/deploy/pi-image/rootfs/etc/writing-environment/fuzzel.ini" "$shell_root/etc/writing-environment/fuzzel.ini"
install -m 0755 "$repo_root/deploy/pi/writing-environment" "$shell_root/usr/local/bin/writing-environment"
install -m 0755 "$repo_root/deploy/pi-image/rootfs/usr/local/bin/writing-environment-change-password" "$shell_root/usr/local/bin/writing-environment-change-password"
install -m 0755 "$repo_root/deploy/pi-image/rootfs/usr/local/bin/writing-environment-session-setup" "$shell_root/usr/local/bin/writing-environment-session-setup"
install -m 0755 "$repo_root/deploy/pi-image/rootfs/usr/local/bin/writing-environment-system-menu" "$shell_root/usr/local/bin/writing-environment-system-menu"
install -m 0755 "$repo_root/deploy/pi-image/rootfs/usr/local/bin/writing-environment-update" "$shell_root/usr/local/bin/writing-environment-update"
install -m 0755 "$display_settings" "$shell_root/usr/local/bin/writing-environment-display-settings"
install -m 0644 "$repo_root/deploy/pi/writing-environment.desktop" "$shell_root/usr/local/share/applications/writing-environment.desktop"
install -m 0644 "$repo_root/deploy/pi-image/rootfs/usr/local/share/applications/writing-environment-browser.desktop" "$shell_root/usr/local/share/applications/writing-environment-browser.desktop"
install -m 0644 "$repo_root/design/app-icon.svg" "$shell_root/usr/local/share/icons/hicolor/scalable/apps/writing-environment.svg"
install -m 0644 "$settings_plugin" "$shell_root/usr/lib/aarch64-linux-gnu/rpcc/librpcc_writing-environment-bluetooth.so"
build_package writing-environment-shell arm64

boot_root="$staging/writing-environment-boot-theme"
mkdir -p "$boot_root/DEBIAN" "$boot_root/usr/share/plymouth/themes/writing-environment"
install -m 0755 "$templates/writing-environment-boot-theme.postinst" "$boot_root/DEBIAN/postinst"
install -m 0644 \
  "$repo_root/deploy/pi-image/rootfs/usr/share/plymouth/themes/writing-environment/writing-environment.plymouth" \
  "$repo_root/deploy/pi-image/rootfs/usr/share/plymouth/themes/writing-environment/writing-environment.script" \
  "$boot_root/usr/share/plymouth/themes/writing-environment/"
install -m 0644 "$repo_root/src-tauri/icons/icon.png" "$boot_root/usr/share/plymouth/themes/writing-environment/logo.png"
build_package writing-environment-boot-theme all

repository_root="$staging/writing-environment-repository"
mkdir -p \
  "$repository_root/DEBIAN" \
  "$repository_root/etc/apt/preferences.d" \
  "$repository_root/etc/apt/sources.list.d" \
  "$repository_root/usr/share/keyrings"
install -m 0644 "$templates/writing-environment-repository.conffiles" "$repository_root/DEBIAN/conffiles"
install -m 0644 "$repo_root/deploy/apt-repository/writing-environment.sources" "$repository_root/etc/apt/sources.list.d/writing-environment.sources"
install -m 0644 "$repo_root/deploy/apt-repository/writing-environment.pref" "$repository_root/etc/apt/preferences.d/writing-environment"
gpg --batch --yes --dearmor \
  --output "$repository_root/usr/share/keyrings/writing-environment-archive-keyring.gpg" \
  "$public_key"
build_package writing-environment-repository all

appliance_root="$staging/writing-environment-appliance"
mkdir -p "$appliance_root/DEBIAN" "$appliance_root/usr/share/doc/writing-environment-appliance"
install -m 0644 /dev/null "$appliance_root/usr/share/doc/writing-environment-appliance/.keep"
build_package writing-environment-appliance all

install -m 0644 "$app_deb" "$output/writing-environment_${app_version}_arm64.deb"

for package_file in "$output"/*.deb; do
  dpkg-deb --info "$package_file" >/dev/null
done

printf 'Writing Environment Debian package set written to:\n%s\n' "$output"
