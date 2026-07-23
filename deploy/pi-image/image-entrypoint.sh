#!/usr/bin/env bash
set -euo pipefail

: "${WRITER_PASSWORD_HASH:?A hashed writer password is required.}"
: "${IMAGE_LOCALE:=en_US.UTF-8}"
: "${IMAGE_KEYBOARD_KEYMAP:=us}"
: "${IMAGE_KEYBOARD_LAYOUT:=English (US)}"
: "${IMAGE_TIMEZONE:=America/Sao_Paulo}"
: "${IMAGE_WIFI_COUNTRY:=BR}"
[[ -d /source/config && -d /source/layer ]] || {
  printf 'Prepared image source is missing under /source.\n' >&2
  exit 1
}
mkdir -p /output

./rpi-image-gen build -S /source -c writing-environment.yaml -- \
  "IGconf_device_user1passhash=$WRITER_PASSWORD_HASH" \
  "IGconf_locale_default=$IMAGE_LOCALE" \
  "IGconf_locale_keyboard_keymap=$IMAGE_KEYBOARD_KEYMAP" \
  "IGconf_locale_keyboard_layout=$IMAGE_KEYBOARD_LAYOUT" \
  "IGconf_locale_timezone=$IMAGE_TIMEZONE" \
  "IGconf_ieee80211_regdom=$IMAGE_WIFI_COUNTRY"

image="$(find work -type f -name 'writing-environment-pi4.img' -print -quit)"
[[ -n "$image" ]] || {
  printf 'rpi-image-gen finished without producing the expected image.\n' >&2
  exit 1
}

/usr/local/sbin/writing-environment-finalize-root-device "$image"
cp "$image" /output/writing-environment-pi4.img
xz -T0 -6 --keep /output/writing-environment-pi4.img
cd /output
sha256sum writing-environment-pi4.img writing-environment-pi4.img.xz >SHA256SUMS
printf 'Image artifacts written to /output.\n'
