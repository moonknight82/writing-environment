Writing Environment Pi update 0.5.3
===================================

This source kit updates an existing 64-bit Raspberry Pi 4 appliance. It builds
the current application directly on the Pi, then installs the new minimal
Labwc desktop, Super+Space Fuzzel drawer, US-International cedilla keyboard
behavior, no-blank writing session, and Writing Environment boot splash.

Before starting
---------------

1. Make sure your important manuscripts have finished syncing or are backed up.
2. Connect the Pi to the internet. The first native build downloads Raspberry
   Pi OS packages, JavaScript dependencies, and the Rust toolchain.
3. Run the update as the normal desktop user, not as root and not with sudo.

Copy and verify the kit
-----------------------

Copy both the .tar.gz file and SHA256SUMS from this NAS folder into a local
folder on the Pi, such as Downloads. Building directly on the SMB share is not
recommended.

Open Terminal in the local folder and run:

  sha256sum -c SHA256SUMS
  tar -xzf writing-environment-pi-source-update-0.5.3-20260729.tar.gz
  cd writing-environment-pi-source-update-0.5.3-20260729
  chmod +x scripts/pi/*.sh scripts/pi-image/*.sh deploy/pi-image/*.sh
  ./scripts/pi/update-current-appliance-from-source.sh

The build can take a while on a Raspberry Pi 4. The script will request sudo
only when it needs to install packages or system files. Rust compilation is
limited to one job by default so the build fits the 2 GB Raspberry Pi 4 memory
baseline.

After it reports that the update is complete, reboot:

  sudo reboot

What is preserved
-----------------

The updater preserves Writing Environment projects, History, Trash, rclone
credentials, and application preferences. It also backs up Labwc configuration
files before modifying them.

After reboot
------------

- The Writing Environment opens full screen.
- Super+Space opens the system drawer.
- Ctrl+Super+S opens Control Centre.
- The drawer shows Wi-Fi, Bluetooth, updates, and Pi power status.
- The apostrophe/dead-acute key followed by c or C produces ç or Ç.
- The screen does not blank during the writing session.
- Boot uses the Writing Environment splash instead of console messages.
