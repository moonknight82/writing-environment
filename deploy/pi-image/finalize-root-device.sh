#!/usr/bin/env bash
set -euo pipefail

image="${1:-}"
[[ -f "$image" ]] || { printf 'Raw Raspberry Pi image not found: %s\n' "$image" >&2; exit 2; }

for command_name in blkid losetup mount mountpoint python3 sfdisk umount; do
  command -v "$command_name" >/dev/null 2>&1 || {
    printf 'Required image finalization tool is missing: %s\n' "$command_name" >&2
    exit 1
  }
done

partition_data="$({ sfdisk --json "$image"; } | python3 -c '
import json, sys
table = json.load(sys.stdin)["partitiontable"]
parts = table["partitions"]
if len(parts) != 2:
    raise SystemExit("expected exactly two partitions")
sector = int(table.get("sectorsize", 512))
print(sector, int(parts[0]["start"]), int(parts[0]["size"]), int(parts[1]["start"]), int(parts[1]["size"]))
')"
read -r sector_size boot_start boot_size root_start root_size <<<"$partition_data"

boot_loop=""
root_loop=""
boot_mount="$(mktemp -d)"
root_mount="$(mktemp -d)"

cleanup() {
  mountpoint -q "$root_mount" && umount "$root_mount" || true
  mountpoint -q "$boot_mount" && umount "$boot_mount" || true
  [[ -z "$root_loop" ]] || losetup -d "$root_loop" 2>/dev/null || true
  [[ -z "$boot_loop" ]] || losetup -d "$boot_loop" 2>/dev/null || true
  rmdir "$root_mount" "$boot_mount" 2>/dev/null || true
}
trap cleanup EXIT

boot_loop="$(losetup --find --show \
  --offset "$((boot_start * sector_size))" \
  --sizelimit "$((boot_size * sector_size))" \
  "$image")"
root_loop="$(losetup --find --show \
  --offset "$((root_start * sector_size))" \
  --sizelimit "$((root_size * sector_size))" \
  "$image")"

boot_uuid="$(blkid -s UUID -o value "$boot_loop")"
root_uuid="$(blkid -s UUID -o value "$root_loop")"
[[ -n "$boot_uuid" && -n "$root_uuid" ]] || {
  printf 'Unable to read the boot and root filesystem UUIDs.\n' >&2
  exit 1
}

mount "$boot_loop" "$boot_mount"
mount "$root_loop" "$root_mount"

cmdline="$boot_mount/cmdline.txt"
fstab="$root_mount/etc/fstab"
[[ -f "$cmdline" && -f "$fstab" ]] || {
  printf 'The image does not contain the expected Raspberry Pi boot files.\n' >&2
  exit 1
}

if grep -q 'root=/dev/disk/by-slot/system' "$cmdline"; then
  sed -i "s#root=/dev/disk/by-slot/system#root=UUID=$root_uuid#" "$cmdline"
elif ! grep -q "root=UUID=$root_uuid" "$cmdline"; then
  printf 'Refusing to replace an unexpected root= value in cmdline.txt.\n' >&2
  exit 1
fi

if grep -q '^/dev/disk/by-slot/system[[:space:]]' "$fstab"; then
  sed -i "s#^/dev/disk/by-slot/system#UUID=$root_uuid#" "$fstab"
elif ! grep -q "^UUID=$root_uuid[[:space:]]" "$fstab"; then
  printf 'Refusing to replace an unexpected root entry in fstab.\n' >&2
  exit 1
fi

if grep -q '^/dev/disk/by-slot/boot[[:space:]]' "$fstab"; then
  sed -i "s#^/dev/disk/by-slot/boot#UUID=$boot_uuid#" "$fstab"
elif ! grep -q "^UUID=$boot_uuid[[:space:]]" "$fstab"; then
  printf 'Refusing to replace an unexpected boot entry in fstab.\n' >&2
  exit 1
fi

sync
printf 'Pinned root filesystem to UUID=%s and boot filesystem to UUID=%s.\n' \
  "$root_uuid" "$boot_uuid"
