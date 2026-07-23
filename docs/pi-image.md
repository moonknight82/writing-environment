# Dedicated Raspberry Pi image

Writing Environment can be packaged as a complete, flashable Raspberry Pi 4 image. It is based on 64-bit Raspberry Pi OS Lite (Debian Trixie), adds only Raspberry Pi's minimal Labwc desktop, and opens the writer automatically after boot.

## What the desktop looks like

The first boot lands directly in Writing Environment's distraction-free, borderless presentation view. PiXonyx Dark Mode is the default across window decorations, GTK applications, menus, and the panel. Press `Ctrl+Tab` to show the complete bottom panel and press it again to hide the panel. The shortcut also starts the panel if it is not already running. Press `Ctrl+Super+S` to open **Control Centre** directly, or open it from the Raspberry Pi system menu. Use F11 or the toolbar's presentation button to return to a normal resizable window.

Control Centre keeps the desktop escape hatches in one place:

- Appearance, desktop, theme, and panel settings;
- display arrangement, scale, and orientation;
- keyboard layout, repeat delay, and repeat rate;
- language, locale, timezone, and Wi-Fi regulatory country;
- Bluetooth device pairing and connection management.

The Bluetooth page opens the full device manager while the existing panel icon remains available for quick connections. The redundant Blueman tray applet is disabled so only one Bluetooth icon appears. Advanced Raspberry Pi configuration remains available from Terminal with `sudo raspi-config`.

`wf-panel-pi` implements pointer-edge reveal through a private Wayfire hotspot protocol. Labwc does not implement that protocol, so the image uses a compositor-level keyboard shortcut to toggle the panel's `autohide` state directly. This keeps the lighter Labwc desktop while preserving quick access to network, Bluetooth, audio, files, Terminal, updates, and power controls. The panel resets to hidden at login.

The bottom panel exposes only the useful escape hatches:

- Writing Environment, Files, LocalSend, and Terminal launchers;
- running-window list and notification tray;
- network, Bluetooth pairing, audio, removable-drive, update, clock, and power controls;
- Raspberry Pi Control Centre for appearance, display, keyboard, localisation, and Bluetooth settings.

The image includes `surf`, a deliberately minimal browser for rclone OAuth and occasional links. It reuses the WebKitGTK engine already required by Writing Environment instead of adding Chromium or another large browser stack. The image also includes the official LocalSend ARM64 package for direct transfers on the local network; LocalSend is installed as a normal Debian package and does not require snapd.

The image does not install an office suite, media player, IDE, games, education software, remote-desktop server, or the Recommended Software catalog. It remains a normal recoverable Linux desktop rather than a locked kiosk. The included browser is intended as a lightweight utility rather than a full daily-browsing environment.

## Build on an Apple Silicon Mac

Requirements:

- Apple Silicon Mac;
- Docker Desktop running;
- a current Writing Environment Pi ARM64 application artifact under `artifacts/pi-arm64`, or its path supplied explicitly;
- enough free disk space for the container cache and uncompressed image.

Run:

```sh
scripts/pi-image/build-on-mac.sh
```

The script asks privately for a password for the `writer` recovery account. The password is hashed immediately and plaintext is not written to disk. It then:

1. verifies the application archive and ARM64 executable;
2. downloads the pinned rclone ARM64 release and verifies its SHA-256 checksum;
3. downloads the pinned official LocalSend ARM64 Debian package and verifies its SHA-256 checksum;
4. builds the pinned official Raspberry Pi `rpi-image-gen` revision in an ARM64 container;
5. produces `writing-environment-pi4.img`, a compressed `.img.xz`, and `SHA256SUMS` under `artifacts/pi-image/mac-*`.

To select a particular application build:

```sh
scripts/pi-image/build-on-mac.sh --artifact /absolute/path/to/writing-environment-pi-arm64-version.tar.gz
```

For unattended trusted automation, pass a SHA-512 crypt hash beginning with `$6$` through `WRITING_ENVIRONMENT_IMAGE_PASSWORD_HASH` or `--password-hash`. Do not put a plaintext password in shell history.

For a test or distributable image with the known initial password `writer`, use:

```sh
scripts/pi-image/build-on-mac.sh --default-password
```

After booting, open Terminal and run `writing-environment-change-password` (or simply `passwd`) to replace it. The default password is convenient for first access, not suitable for a Pi exposed to untrusted users or a network service.

The personal defaults are English (US), a US keyboard map, São Paulo time, and Brazil's Wi-Fi regulatory region. Override any of them when building; for example, a Brazilian keyboard can be selected with:

```sh
scripts/pi-image/build-on-mac.sh \
  --keyboard-keymap br \
  --keyboard-layout 'Portuguese (Brazil)'
```

The matching `--locale`, `--timezone`, and `--wifi-country` options are shown by `scripts/pi-image/build-on-mac.sh --help`.

The container runs privileged because image assembly needs loop devices, mounts, and container capabilities. The builder is pinned to one upstream commit so a later upstream change cannot silently alter a release.

## Flash and start

Verify the compressed image before flashing:

```sh
cd artifacts/pi-image/mac-YYYYMMDD-HHMMSS
shasum -a 256 -c SHA256SUMS
```

Use Raspberry Pi Imager's **Use custom** action to select `writing-environment-pi4.img.xz`, then write it to the target microSD card or USB SSD. Do not use Imager OS customization to replace the image's account or desktop configuration.

The final image pins its boot and root filesystems by their unique filesystem UUIDs. This works from a microSD card, USB storage, or NVMe and avoids relying on Raspberry Pi's `/dev/disk/by-slot` storage aliases.

At first boot, the system expands the root filesystem once and may reboot automatically. It then logs in as `writer` and opens Writing Environment. Use the network panel control to join Wi-Fi, then configure optional Dropbox or another provider from Terminal with:

```sh
rclone config
```

Provider credentials are created on the Pi and are never embedded in the image produced on the Mac.
The minimal browser opens automatically when rclone needs an OAuth login. LocalSend is available from the bottom panel for transfers between the Pi and other devices on the same network.

## Update the application without reflashing

Copy a newer Pi ARM64 application `.tar.gz` to the Pi and run:

```sh
writing-environment-update /path/to/writing-environment-pi-arm64-version.tar.gz
```

The updater validates archive paths and delegates checksum, architecture, library, and hardware checks to the artifact's installer. It preserves the boot launch configuration and does not touch projects, Trash, History, rclone credentials, or preferences.

Desktop-shell releases can also include an `writing-environment-pi4-*-update.tar.gz` bundle. This updates the app and the appliance session without reflashing, preserving the same user data and rclone credentials:

```sh
tar -xzf writing-environment-pi4-*-update.tar.gz
cd writing-environment-pi4-*-update
./apply-appliance-update.sh
sudo reboot
```

The updater backs up the previous user autostart and system panel configuration before replacing the managed settings. It also installs the complete Control Centre, the `Ctrl+Super+S` shortcut, the minimal OAuth browser, and LocalSend without touching writer data. Existing Chromium, snapd, and Snap packages are left untouched.

Raspberry Pi OS system updates remain available through the panel updater or `sudo apt update && sudo apt full-upgrade` in Terminal.

## Recovery

- Press `Alt+F4` to close the writer and expose the desktop.
- Open Terminal from the panel for diagnostics or manual updates.
- Press `Ctrl+Super+S` to open Control Centre even while the panel is hidden.
- The `writer` password chosen during the image build enables `sudo` recovery. Images built with `--default-password` initially use `writer`; change it with `writing-environment-change-password`.
- SSH is disabled by default; enable it explicitly through Raspberry Pi Configuration only if remote administration is needed.
- Projects live in ordinary Markdown folders; `/home/writer/Writing` is prepared as a convenient local default.
- Autostart configuration is under `/home/writer/.config/labwc/autostart`.

The image does not contain manuscript data. Rebuilding or reflashing an image erases the target disk, so copy unsynchronized projects elsewhere first.

## Reproducibility and pinned inputs

The image source lives under `deploy/pi-image`, while `scripts/pi-image/prepare-source.sh` produces a self-contained source tree for the builder. The current release pins:

- Raspberry Pi `rpi-image-gen` by Git commit;
- Raspberry Pi OS Trixie through the upstream configuration;
- rclone by version and SHA-256 checksum;
- LocalSend's official ARM64 Debian package by version and SHA-256 checksum;
- the Writing Environment application through the selected checksummed artifact.

The image itself intentionally follows Raspberry Pi OS security and package updates after installation rather than freezing the running system forever.
