#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'EOF'
Usage: ./apply-appliance-update.sh [PI_APP_ARCHIVE]

Update an existing Writing Environment Raspberry Pi appliance in place. The
update installs the current ARM64 app, replaces the panel with the Super+Space
system drawer, applies the US-International cedilla keyboard default, disables
session blanking, and installs the quiet boot splash.

Projects, rclone credentials, and user preferences are preserved. Reboot after
the update completes.
EOF
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
esac
[[ $# -le 1 ]] || { usage >&2; exit 2; }
[[ "$(uname -s)" == "Linux" && "$(uname -m)" == "aarch64" ]] || {
  printf 'This updater must be run on the 64-bit Raspberry Pi.\n' >&2
  exit 1
}
command -v sudo >/dev/null 2>&1 || { printf 'sudo is required.\n' >&2; exit 1; }

if [[ -d "$script_dir/../../deploy/pi-image/rootfs" ]]; then
  appliance_rootfs="$(cd "$script_dir/../../deploy/pi-image/rootfs" && pwd)"
elif [[ -d "$script_dir/appliance-rootfs" ]]; then
  appliance_rootfs="$script_dir/appliance-rootfs"
else
  printf 'The appliance-rootfs assets are missing beside the updater.\n' >&2
  exit 1
fi
readonly appliance_rootfs

archive="${1:-}"
if [[ -z "$archive" ]]; then
  archive="$(find "$script_dir" -maxdepth 1 -type f -name 'writing-environment-pi-arm64-*.tar.gz' -print | LC_ALL=C sort | tail -n 1)"
fi
[[ -f "$archive" ]] || { printf 'Pi application archive not found.\n' >&2; exit 1; }

if [[ -f "$script_dir/SHA256SUMS" ]]; then
  (
    cd "$script_dir"
    sha256sum -c SHA256SUMS
  )
fi

work_dir="$(mktemp -d /tmp/writing-environment-update.XXXXXX)"
cleanup() {
  case "$work_dir" in
    /tmp/writing-environment-update.*) rm -rf -- "$work_dir" ;;
  esac
}
trap cleanup EXIT

tar -xzf "$archive" -C "$work_dir"
installer="$(find "$work_dir" -mindepth 2 -maxdepth 2 -type f -name install.sh -print -quit)"
[[ -x "$installer" ]] || { printf 'The application archive has no executable installer.\n' >&2; exit 1; }
"$installer" --preserve-autostart

sudo apt-get update
sudo apt-get install -y \
  blueman \
  bluez \
  file \
  fuzzel \
  librsvg2-bin \
  locales \
  pi-bluetooth \
  plymouth \
  plymouth-themes \
  procps \
  raindrop \
  rasputin \
  rc-gui \
  rpcc \
  surf \
  wlopm \
  wlr-randr \
  xdg-utils

sudo install -m 0755 \
  "$appliance_rootfs/usr/local/bin/writing-environment-session-setup" \
  "$appliance_rootfs/usr/local/bin/writing-environment-system-menu" \
  /usr/local/bin/
sudo install -d -m 0755 /etc/writing-environment
sudo install -m 0644 \
  "$appliance_rootfs/etc/writing-environment/fuzzel.ini" \
  /etc/writing-environment/fuzzel.ini

timestamp="$(date +%Y%m%d-%H%M%S)"
labwc_dir="${XDG_CONFIG_HOME:-$HOME/.config}/labwc"
autostart_file="$labwc_dir/autostart"
mkdir -p "$labwc_dir"
[[ ! -f "$autostart_file" ]] || cp -p "$autostart_file" "$autostart_file.before-appliance-update-$timestamp"
autostart_tmp="$(mktemp "$labwc_dir/autostart.XXXXXX")"
awk '
  {
    normalized = $0
    sub(/^[[:space:]]+/, "", normalized)
    sub(/[[:space:]]+$/, "", normalized)
  }
  normalized ~ /^\/usr\/bin\/lwrespawn \/usr\/bin\/(pcmanfm-pi|wf-panel-pi)/ { next }
  normalized == "/usr/bin/kanshi &" { next }
  normalized == "/usr/bin/lxsession-xdg-autostart" { next }
  normalized ~ /^\/usr\/local\/bin\/writing-environment-(panel-menu|session-setup)/ { next }
  normalized == "/usr/local/bin/writing-environment &" { next }
  { print }
  END {
    print "/usr/local/bin/writing-environment-session-setup"
    print "/usr/local/bin/writing-environment &"
  }
' "$autostart_file" 2>/dev/null >"$autostart_tmp" || {
  printf '/usr/local/bin/writing-environment-session-setup\n/usr/local/bin/writing-environment &\n' >"$autostart_tmp"
}
chmod 0644 "$autostart_tmp"
mv "$autostart_tmp" "$autostart_file"

patch_labwc_shortcuts() {
  local target_file="$1" scope="$2" target_tmp
  target_tmp="$(mktemp /tmp/writing-environment-labwc.XXXXXX)"
  awk '
    /<!-- writing-environment-(panel-menu|system-menu|settings) -->/ { skipping = 1; next }
    skipping && /<\/keybind>/ { skipping = 0; next }
    skipping { next }
    /<\/keyboard>/ && !inserted {
      print "    <!-- writing-environment-system-menu -->"
      print "    <keybind key=\"W-Space\">"
      print "      <action name=\"Execute\" command=\"writing-environment-system-menu\" />"
      print "    </keybind>"
      print "    <!-- writing-environment-settings -->"
      print "    <keybind key=\"C-W-s\">"
      print "      <action name=\"Execute\" command=\"rpcc\" />"
      print "    </keybind>"
      inserted = 1
    }
    { print }
  ' "$target_file" >"$target_tmp"

  if ! grep -q 'key="W-Space"' "$target_tmp"; then
    local fallback_tmp
    fallback_tmp="$(mktemp /tmp/writing-environment-labwc-fallback.XXXXXX)"
    awk '
      /<\/(labwc_config|openbox_config)>/ && !inserted {
        print "  <keyboard>"
        print "    <!-- writing-environment-system-menu -->"
        print "    <keybind key=\"W-Space\">"
        print "      <action name=\"Execute\" command=\"writing-environment-system-menu\" />"
        print "    </keybind>"
        print "    <!-- writing-environment-settings -->"
        print "    <keybind key=\"C-W-s\">"
        print "      <action name=\"Execute\" command=\"rpcc\" />"
        print "    </keybind>"
        print "  </keyboard>"
        inserted = 1
      }
      { print }
    ' "$target_tmp" >"$fallback_tmp"
    mv "$fallback_tmp" "$target_tmp"
  fi

  grep -q 'key="W-Space"' "$target_tmp" || {
    printf 'Could not add Super+Space to %s.\n' "$target_file" >&2
    exit 1
  }
  if [[ "$scope" == "system" ]]; then
    sudo cp -p "$target_file" "$target_file.before-appliance-update-$timestamp"
    sudo install -m 0644 "$target_tmp" "$target_file"
  else
    cp -p "$target_file" "$target_file.before-appliance-update-$timestamp"
    install -m 0644 "$target_tmp" "$target_file"
  fi
}

system_rc_file="/etc/xdg/labwc/rc.xml"
[[ ! -f "$system_rc_file" ]] || patch_labwc_shortcuts "$system_rc_file" system
user_rc_file="$labwc_dir/rc.xml"
[[ ! -f "$user_rc_file" ]] || patch_labwc_shortcuts "$user_rc_file" user

environment_file="$labwc_dir/environment"
[[ ! -f "$environment_file" ]] || cp -p "$environment_file" "$environment_file.before-appliance-update-$timestamp"
environment_tmp="$(mktemp "$labwc_dir/environment.XXXXXX")"
awk '
  /^[[:space:]]*(export[[:space:]]+)?(BROWSER|XKB_DEFAULT_MODEL|XKB_DEFAULT_LAYOUT|XKB_DEFAULT_VARIANT)[[:space:]]*=/ { next }
  { print }
  END {
    print "BROWSER=/usr/bin/surf"
    print "XKB_DEFAULT_MODEL=pc105"
    print "XKB_DEFAULT_LAYOUT=us"
    print "XKB_DEFAULT_VARIANT=intl"
  }
' "$environment_file" 2>/dev/null >"$environment_tmp" || true
chmod 0644 "$environment_tmp"
mv "$environment_tmp" "$environment_file"

compose_file="$HOME/.XCompose"
if [[ ! -f "$compose_file" ]]; then
  install -m 0644 "$appliance_rootfs/etc/skel/.XCompose" "$compose_file"
elif ! grep -q '<dead_acute> <c>.*ccedilla' "$compose_file"; then
  cp -p "$compose_file" "$compose_file.before-appliance-update-$timestamp"
  printf '\n# Writing Environment Portuguese cedilla\n<dead_acute> <c> : "ç" ccedilla\n<dead_acute> <C> : "Ç" Ccedilla\n' >>"$compose_file"
fi

sudo install -d -m 0755 /usr/local/share/applications
sudo install -m 0644 \
  "$appliance_rootfs/usr/local/share/applications/writing-environment-browser.desktop" \
  /usr/local/share/applications/writing-environment-browser.desktop
xdg-mime default writing-environment-browser.desktop text/html
xdg-mime default writing-environment-browser.desktop x-scheme-handler/http
xdg-mime default writing-environment-browser.desktop x-scheme-handler/https

localsend_deb="$(find "$script_dir" -maxdepth 1 -type f -name 'LocalSend-*-linux-arm-64.deb' -print -quit)"
if [[ -f "$localsend_deb" ]]; then
  [[ "$(dpkg-deb -f "$localsend_deb" Architecture)" == "arm64" ]] || {
    printf 'The bundled LocalSend package is not Linux ARM64.\n' >&2
    exit 1
  }
  sudo apt-get install -y "$localsend_deb"
fi

bluetooth_plugin="$script_dir/librpcc_writing-environment-bluetooth.so"
if [[ -f "$bluetooth_plugin" ]]; then
  file "$bluetooth_plugin" | grep -qE 'ELF 64-bit.*(ARM aarch64|aarch64)' || {
    printf 'The bundled Control Centre Bluetooth plugin is not Linux ARM64.\n' >&2
    exit 1
  }
  sudo install -d -m 0755 /usr/lib/aarch64-linux-gnu/rpcc
  sudo install -m 0644 "$bluetooth_plugin" \
    /usr/lib/aarch64-linux-gnu/rpcc/librpcc_writing-environment-bluetooth.so
fi

mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/autostart"
install -m 0644 /dev/stdin "${XDG_CONFIG_HOME:-$HOME/.config}/autostart/blueman.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Blueman Applet
Hidden=true
EOF

sudo install -d -m 0755 /usr/share/plymouth/themes/writing-environment
sudo install -m 0644 \
  "$appliance_rootfs/usr/share/plymouth/themes/writing-environment/writing-environment.plymouth" \
  "$appliance_rootfs/usr/share/plymouth/themes/writing-environment/writing-environment.script" \
  /usr/share/plymouth/themes/writing-environment/
rsvg-convert -w 512 -h 512 \
  /usr/local/share/icons/hicolor/scalable/apps/writing-environment.svg \
  | sudo tee /usr/share/plymouth/themes/writing-environment/logo.png >/dev/null
sudo plymouth-set-default-theme -R writing-environment

boot_dir=/boot/firmware
[[ -d "$boot_dir" ]] || boot_dir=/boot
sudo python3 - "$boot_dir/cmdline.txt" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
arguments = path.read_text(encoding="utf-8").split()
arguments = [argument for argument in arguments if argument != "console=tty1"]
for argument in (
    "quiet", "splash", "logo.nologo", "vt.global_cursor_default=0",
    "systemd.show_status=auto", "rd.systemd.show_status=auto",
    "plymouth.ignore-serial-consoles",
):
    if argument not in arguments:
        arguments.append(argument)
path.write_text(" ".join(arguments) + "\n", encoding="utf-8")
PY
if sudo grep -q '^disable_splash=' "$boot_dir/config.txt"; then
  sudo sed -i 's/^disable_splash=.*/disable_splash=1/' "$boot_dir/config.txt"
else
  printf '\n[all]\ndisable_splash=1\n' | sudo tee -a "$boot_dir/config.txt" >/dev/null
fi

sudo systemctl enable --now bluetooth.service
if systemctl list-unit-files hciuart.service --no-legend 2>/dev/null | grep -q '^hciuart.service'; then
  sudo systemctl enable --now hciuart.service
fi
sudo rfkill unblock bluetooth 2>/dev/null || true

current_locale="$(sed -n 's/^LANG=//p' /etc/locale.conf 2>/dev/null | head -n 1)"
case "$current_locale" in
  [a-z][a-z]_[A-Z][A-Z].UTF-8) locale_name="$current_locale" ;;
  *) locale_name="en_US.UTF-8" ;;
esac
sudo localedef -i "${locale_name%%.*}" -f UTF-8 "$locale_name"
printf 'LANG=%s\n' "$locale_name" | sudo tee /etc/locale.conf /etc/default/locale >/dev/null

pkill -f '/usr/bin/lwrespawn /usr/bin/wf-panel-pi' 2>/dev/null || true
pkill -x wf-panel-pi 2>/dev/null || true
if pgrep -x labwc >/dev/null 2>&1; then
  labwc --reconfigure || true
fi

printf '\nUpdate complete. Reboot the Pi with: sudo reboot\n'
