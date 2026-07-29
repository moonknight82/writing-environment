Writing Environment Raspberry Pi appliance update
=================================================

Copy all files from this kit, including its appliance-rootfs directory, into one
folder on the Raspberry Pi, open Terminal in that folder, and run:

  chmod +x apply-appliance-update.sh
  ./apply-appliance-update.sh ./writing-environment-pi-arm64-*.tar.gz
  sudo reboot

The updater verifies the bundled checksums and installs the ARM64 app, the
Labwc Super+Space system drawer, the Ctrl+Super+S Control Centre shortcut, and
the Appearance, Display, Keyboard, Localisation, and Bluetooth settings. It
also installs a minimal browser, the official LocalSend ARM64 package, and the
quiet Writing Environment boot splash.
Snapd is not required. Projects, History, Trash, rclone credentials, and
application preferences are preserved.

After reboot, press Super+Space to open the system drawer. Its first rows show
Wi-Fi, Bluetooth, available-update, and Raspberry Pi power status. Use the same
drawer to launch utilities or safely restart and turn off the device.

Press Ctrl+Super+S to open Control Centre from anywhere on the desktop.

The updater leaves any existing Chromium, snapd, or Snap edition of LocalSend
untouched. Remove those separately only after confirming the new browser and
direct LocalSend installation work for you.
