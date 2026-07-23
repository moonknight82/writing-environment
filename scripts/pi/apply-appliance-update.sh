#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly script_dir

usage() {
  cat <<'EOF'
Usage: ./apply-appliance-update.sh [PI_APP_ARCHIVE]

Update an existing Writing Environment Raspberry Pi image in place. The script
installs the bundled/current ARM64 app, removes duplicate Labwc startup
commands, installs the reliable Ctrl+Tab panel toggle, adds Ctrl+Super+S for
Control Centre, installs the complete writing-focused settings surface, adds a
minimal OAuth browser, and installs the bundled official LocalSend ARM64 package.

The update preserves projects, rclone credentials, and user preferences. Reboot
after it completes.
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

archive="${1:-}"
if [[ -z "$archive" ]]; then
  archive="$(find "$script_dir" -maxdepth 1 -type f -name 'writing-environment-pi-arm64-*.tar.gz' -print | LC_ALL=C sort | tail -n 1)"
fi
[[ -f "$archive" ]] || { printf 'Pi application archive not found.\n' >&2; exit 1; }

if [[ -f "$script_dir/SHA256SUMS" ]]; then
  command -v sha256sum >/dev/null 2>&1 || { printf 'sha256sum is required.\n' >&2; exit 1; }
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

timestamp="$(date +%Y%m%d-%H%M%S)"
labwc_dir="${XDG_CONFIG_HOME:-$HOME/.config}/labwc"
autostart_file="$labwc_dir/autostart"
mkdir -p "$labwc_dir"
if [[ -f "$autostart_file" ]]; then
  cp -p "$autostart_file" "$autostart_file.before-appliance-update-$timestamp"
fi
autostart_tmp="$(mktemp "$labwc_dir/autostart.XXXXXX")"
awk '
  {
    normalized = $0
    sub(/^[[:space:]]+/, "", normalized)
    sub(/[[:space:]]+$/, "", normalized)
  }
  normalized == "/usr/bin/lwrespawn /usr/bin/pcmanfm-pi &" { next }
  normalized == "/usr/bin/lwrespawn /usr/bin/wf-panel-pi &" { next }
  normalized == "/usr/bin/kanshi &" { next }
  normalized == "/usr/bin/lxsession-xdg-autostart" { next }
  normalized == "/usr/local/bin/writing-environment-panel-menu --hide &" {
    if (panel_hide_seen) next
    panel_hide_seen = 1
  }
  normalized == "/usr/local/bin/writing-environment &" {
    if (app_seen) next
    app_seen = 1
  }
  { print }
  END {
    if (!panel_hide_seen) print "/usr/local/bin/writing-environment-panel-menu --hide &"
    if (!app_seen) print "/usr/local/bin/writing-environment &"
  }
' "$autostart_file" 2>/dev/null >"$autostart_tmp" || {
  printf '/usr/local/bin/writing-environment &\n' >"$autostart_tmp"
}
chmod 0644 "$autostart_tmp"
mv "$autostart_tmp" "$autostart_file"

patch_panel_config() {
  local target_file="$1"
  local install_scope="$2"
  local target_tmp
  target_tmp="$(mktemp /tmp/writing-environment-panel.XXXXXX)"
  awk '
    BEGIN {
      wanted["autohide"] = "true"
      wanted["autohide_duration"] = "180"
      wanted["edge_offset"] = "20"
      wanted["layer"] = "overlay"
      wanted["minimal_height"] = "40"
      wanted["position"] = "bottom"
    }
    {
      key = $0
      sub(/=.*/, "", key)
      if (key == "launchers") {
        value = $0
        sub(/^[^=]*=/, "", value)
        if (value !~ /(^|[[:space:]])localsend_app([[:space:]]|$)/) {
          if (value != "") value = value " "
          value = value "localsend_app"
        }
        print "launchers=" value
        launchers_seen = 1
        next
      }
      if (key in wanted) {
        if (!seen[key]) print key "=" wanted[key]
        seen[key] = 1
        next
      }
      print
    }
    END {
      for (key in wanted) if (!seen[key]) print key "=" wanted[key]
      if (!launchers_seen) print "launchers=writing-environment pcmanfm localsend_app x-terminal-emulator"
    }
  ' "$target_file" >"$target_tmp"

  if [[ "$install_scope" == "system" ]]; then
    sudo cp -p "$target_file" "$target_file.before-appliance-update-$timestamp"
    sudo install -m 0644 "$target_tmp" "$target_file"
  else
    cp -p "$target_file" "$target_file.before-appliance-update-$timestamp"
    install -m 0644 "$target_tmp" "$target_file"
  fi
}

system_panel_file="/etc/xdg/wf-panel-pi/wf-panel-pi.ini"
[[ -f "$system_panel_file" ]] || { printf 'wf-panel-pi configuration is missing.\n' >&2; exit 1; }
patch_panel_config "$system_panel_file" system

user_panel_file="${XDG_CONFIG_HOME:-$HOME/.config}/wf-panel-pi/wf-panel-pi.ini"
if [[ -f "$user_panel_file" ]]; then
  patch_panel_config "$user_panel_file" user
fi

sudo install -m 0755 /dev/stdin /usr/local/bin/writing-environment-panel-menu <<'EOF'
#!/bin/sh
set -eu
panel=/usr/bin/wf-panel-pi
config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
config_dir="$config_home/wf-panel-pi"
config_file="$config_dir/wf-panel-pi.ini"
requested="${1:-toggle}"
case "$requested" in
  toggle|--hide) ;;
  *) printf 'Usage: writing-environment-panel-menu [--hide]\n' >&2; exit 2 ;;
esac
mkdir -p "$config_dir"
[ -f "$config_file" ] || touch "$config_file"
current="$(awk '
  /^\[panel\][[:space:]]*$/ { in_panel = 1; next }
  /^\[/ { in_panel = 0 }
  in_panel && /^[[:space:]]*autohide[[:space:]]*=/ {
    value = $0
    sub(/^[^=]*=[[:space:]]*/, "", value)
    print value
    exit
  }
' "$config_file" 2>/dev/null || true)"
if [ "$requested" = "--hide" ]; then
  next=true
elif [ "$current" = "false" ]; then
  next=true
else
  next=false
fi
config_tmp="$(mktemp "$config_dir/wf-panel-pi.ini.XXXXXX")"
cleanup() {
  rm -f -- "$config_tmp"
}
trap cleanup EXIT HUP INT TERM
awk -v value="$next" '
  BEGIN { in_panel = 0; panel_seen = 0; written = 0 }
  /^\[[^]]+\][[:space:]]*$/ {
    if (in_panel && !written) {
      print "autohide=" value
      written = 1
    }
    in_panel = ($0 ~ /^\[panel\][[:space:]]*$/)
    if (in_panel) panel_seen = 1
    print
    next
  }
  in_panel && /^[[:space:]]*autohide[[:space:]]*=/ {
    if (!written) print "autohide=" value
    written = 1
    next
  }
  { print }
  END {
    if (in_panel && !written) {
      print "autohide=" value
    } else if (!panel_seen) {
      if (NR > 0) print ""
      print "[panel]"
      print "autohide=" value
    }
  }
' "$config_file" 2>/dev/null >"$config_tmp"
chmod 0644 "$config_tmp"
mv "$config_tmp" "$config_file"
trap - EXIT HUP INT TERM
if [ "$requested" != "--hide" ] && ! pgrep -x wf-panel-pi >/dev/null 2>&1; then
  nohup "$panel" >/tmp/writing-environment-panel.log 2>&1 &
fi
exit 0
EOF

patch_labwc_shortcut() {
  local target_file="$1"
  local install_scope="$2"
  local target_tmp
  target_tmp="$(mktemp /tmp/writing-environment-labwc-rc.XXXXXX)"
  awk '
    /<!-- writing-environment-(panel-menu|settings) -->/ { skipping = 1; next }
    skipping && /<\/keybind>/ { skipping = 0; next }
    skipping { next }
    /<\/keyboard>/ && !inserted {
      print "    <!-- writing-environment-panel-menu -->"
      print "    <keybind key=\"C-Tab\">"
      print "      <action name=\"Execute\" command=\"writing-environment-panel-menu\" />"
      print "    </keybind>"
      print "    <!-- writing-environment-settings -->"
      print "    <keybind key=\"C-W-s\">"
      print "      <action name=\"Execute\" command=\"rpcc\" />"
      print "    </keybind>"
      inserted = 1
    }
    { print }
  ' "$target_file" >"$target_tmp"

  if ! grep -q 'key="C-Tab"' "$target_tmp" || ! grep -q 'key="C-W-s"' "$target_tmp"; then
    local fallback_tmp
    fallback_tmp="$(mktemp /tmp/writing-environment-labwc-rc-fallback.XXXXXX)"
    awk '
      /<\/(labwc_config|openbox_config)>/ && !inserted {
        print "  <keyboard>"
        print "    <!-- writing-environment-panel-menu -->"
        print "    <keybind key=\"C-Tab\">"
        print "      <action name=\"Execute\" command=\"writing-environment-panel-menu\" />"
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

  if { ! grep -q 'key="C-Tab"' "$target_tmp" || ! grep -q 'key="C-W-s"' "$target_tmp"; } \
      && [[ "$install_scope" == "user" ]]; then
    printf '%s\n' \
      '<?xml version="1.0"?>' \
      '<labwc_config>' \
      '  <keyboard>' \
      '    <!-- writing-environment-panel-menu -->' \
      '    <keybind key="C-Tab">' \
      '      <action name="Execute" command="writing-environment-panel-menu" />' \
      '    </keybind>' \
      '    <!-- writing-environment-settings -->' \
      '    <keybind key="C-W-s">' \
      '      <action name="Execute" command="rpcc" />' \
      '    </keybind>' \
      '  </keyboard>' \
      '</labwc_config>' >"$target_tmp"
  fi

  if ! grep -q 'key="C-Tab"' "$target_tmp" || ! grep -q 'key="C-W-s"' "$target_tmp"; then
    printf 'Could not add the managed desktop shortcuts to %s.\n' "$target_file" >&2
    exit 1
  fi

  if [[ "$install_scope" == "system" ]]; then
    sudo cp -p "$target_file" "$target_file.before-appliance-update-$timestamp"
    sudo install -m 0644 "$target_tmp" "$target_file"
  else
    cp -p "$target_file" "$target_file.before-appliance-update-$timestamp"
    install -m 0644 "$target_tmp" "$target_file"
  fi
}

system_rc_file="/etc/xdg/labwc/rc.xml"
if [[ -f "$system_rc_file" ]]; then
  patch_labwc_shortcut "$system_rc_file" system
fi

user_rc_file="$labwc_dir/rc.xml"
if [[ -f "$user_rc_file" ]]; then
  patch_labwc_shortcut "$user_rc_file" user
fi

if pgrep -x labwc >/dev/null 2>&1; then
  labwc --reconfigure || true
fi

sudo apt-get update
sudo apt-get install -y \
  blueman \
  bluez \
  file \
  locales \
  pi-bluetooth \
  pipanel \
  raindrop \
  rasputin \
  rc-gui \
  rpcc \
  surf \
  wfplug-bluetooth \
  wlr-randr \
  xdg-utils

localsend_deb="$(find "$script_dir" -maxdepth 1 -type f -name 'LocalSend-*-linux-arm-64.deb' -print -quit)"
if [[ -f "$localsend_deb" ]]; then
  [[ "$(dpkg-deb -f "$localsend_deb" Architecture)" == "arm64" ]] || {
    printf 'The bundled LocalSend package is not Linux ARM64.\n' >&2
    exit 1
  }
  sudo apt-get install -y "$localsend_deb"
else
  printf 'Warning: the official LocalSend ARM64 package was not included; continuing without it.\n' >&2
fi

sudo install -d -m 0755 /usr/local/share/applications
sudo install -m 0644 /dev/stdin /usr/local/share/applications/writing-environment-browser.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=Web Browser
GenericName=Minimal Web Browser
Comment=Open web pages and authenticate cloud accounts
Exec=surf %u
Icon=web-browser
Terminal=false
Categories=Network;WebBrowser;
MimeType=text/html;x-scheme-handler/http;x-scheme-handler/https;
StartupNotify=true
EOF

xdg-mime default writing-environment-browser.desktop text/html
xdg-mime default writing-environment-browser.desktop x-scheme-handler/http
xdg-mime default writing-environment-browser.desktop x-scheme-handler/https

environment_file="$labwc_dir/environment"
if [[ -f "$environment_file" ]]; then
  cp -p "$environment_file" "$environment_file.before-appliance-update-$timestamp"
fi
environment_tmp="$(mktemp "$labwc_dir/environment.XXXXXX")"
awk '
  /^[[:space:]]*(export[[:space:]]+)?BROWSER[[:space:]]*=/ { next }
  { print }
  END { print "BROWSER=/usr/bin/surf" }
' "$environment_file" 2>/dev/null >"$environment_tmp" || printf 'BROWSER=/usr/bin/surf\n' >"$environment_tmp"
chmod 0644 "$environment_tmp"
mv "$environment_tmp" "$environment_file"

bluetooth_plugin="$script_dir/librpcc_writing-environment-bluetooth.so"
if [[ -f "$bluetooth_plugin" ]]; then
  file "$bluetooth_plugin" | grep -qE 'ELF 64-bit.*(ARM aarch64|aarch64)' || {
    printf 'The bundled Control Centre Bluetooth plugin is not Linux ARM64.\n' >&2
    exit 1
  }
  sudo install -d -m 0755 /usr/lib/aarch64-linux-gnu/rpcc
  sudo install -m 0644 \
    "$bluetooth_plugin" \
    /usr/lib/aarch64-linux-gnu/rpcc/librpcc_writing-environment-bluetooth.so
else
  printf 'Warning: the Control Centre Bluetooth plugin was not included; continuing without that tab.\n' >&2
fi

mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/autostart"
install -m 0644 /dev/stdin "${XDG_CONFIG_HOME:-$HOME/.config}/autostart/blueman.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Blueman Applet
Hidden=true
EOF

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
printf 'LANG=%s\n' "$locale_name" | sudo tee /etc/locale.conf >/dev/null
printf 'LANG=%s\n' "$locale_name" | sudo tee /etc/default/locale >/dev/null

printf '\nUpdate complete. Reboot the Pi with: sudo reboot\n'
