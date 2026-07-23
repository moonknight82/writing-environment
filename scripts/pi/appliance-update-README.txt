Writing Environment Raspberry Pi appliance update
=================================================

Copy all files from this kit into one folder on the Raspberry Pi, open Terminal
in that folder, and run:

  chmod +x apply-appliance-update.sh
  ./apply-appliance-update.sh ./writing-environment-pi-arm64-*.tar.gz
  sudo reboot

The updater verifies the bundled checksums and installs the ARM64 app, installs
the Labwc Ctrl+Tab panel toggle, adds the Ctrl+Super+S Control Centre shortcut,
and installs the Appearance, Display, Keyboard, Localisation, and Bluetooth
settings. It also installs a minimal browser for rclone authentication and the
official LocalSend ARM64 package, then adds LocalSend to the bottom panel.
Snapd is not required. Projects, History, Trash, rclone credentials, and
application preferences are preserved.

After reboot, press Ctrl+Tab to show the complete bottom panel. Press Ctrl+Tab
again to hide it. Pointer-edge reveal is not supported by Labwc because the
Raspberry Pi panel implements that gesture through a Wayfire-only protocol.

Press Ctrl+Super+S to open Control Centre from anywhere on the desktop.

The updater leaves any existing Chromium, snapd, or Snap edition of LocalSend
untouched. Remove those separately only after confirming the new browser and
direct LocalSend installation work for you.
