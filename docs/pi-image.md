# Dedicated Raspberry Pi image

Writing Environment can be packaged as a complete, flashable Raspberry Pi 4 image. It is based on 64-bit Raspberry Pi OS Lite (Debian Trixie), adds only Raspberry Pi's minimal Labwc compositor and writing utilities, and opens the writer automatically after boot.

## What the desktop looks like

The first boot lands directly in Writing Environment's borderless maximized presentation view. It looks fullscreen while allowing GTK utilities and Control Centre dialogs to rise normally. PiXonyx Dark Mode is the default across the writer, GTK utilities, and the system drawer. Press `Super+Space` to open the Fuzzel drawer above the writer. Press `Ctrl+Super+S` to open **Control Centre** directly. Use F11 or the toolbar's presentation button to return to a normal resizable window.

The drawer is intentionally curated rather than a general Linux application list. Its first four rows report a fresh Wi-Fi snapshot, a fresh Bluetooth snapshot, the cached number of available package updates, and Raspberry Pi power/thermal health. Selecting a status row opens its manager or details. The remaining entries open Writing Environment, Files, Browser, LocalSend, Terminal, Control Centre, display settings, and confirmed recovery/power actions. The update count is refreshed in the background so opening the drawer never waits for APT.

Control Centre keeps the desktop escape hatches in one place:

- Appearance and theme settings;
- display arrangement, scale, and orientation;
- keyboard layout, repeat delay, and repeat rate;
- language, locale, timezone, and Wi-Fi regulatory country;
- Bluetooth device pairing and connection management.

The Bluetooth page and Bluetooth status row open the full device manager. The redundant Blueman tray applet is disabled. Advanced Raspberry Pi configuration remains available from Terminal with `sudo raspi-config`.

The appliance does not start `wf-panel-pi`, desktop icons, a notification tray, or a screen-idle blanker. Labwc owns `Super+Space`, so the drawer remains available even if Writing Environment is closed or needs recovery. Critical power history remains visible in the drawer through `vcgencmd get_throttled`.

The image includes `surf`, a deliberately minimal browser for rclone OAuth and occasional links. It reuses the WebKitGTK engine already required by Writing Environment instead of adding Chromium or another large browser stack. The image also includes the official LocalSend ARM64 package for direct transfers on the local network; LocalSend is installed as a normal Debian package and does not require snapd.

The image does not install an office suite, media player, IDE, games, education software, remote-desktop server, or the Recommended Software catalog. It remains a normal recoverable Linux desktop rather than a locked kiosk. The included browser is intended as a lightweight utility rather than a full daily-browsing environment.

## Build on an Apple Silicon Mac

Requirements:

- Apple Silicon Mac;
- Docker Desktop running;
- a current Writing Environment Pi ARM64 `.deb` under `artifacts/pi-arm64`, or its path supplied explicitly;
- the local APT signing key under `.apt-signing/writing-environment-apt-private.asc` (ignored by Git);
- enough free disk space for the container cache and uncompressed image.

Run:

```sh
scripts/pi-image/build-on-mac.sh
```

The script asks privately for a password for the `writer` recovery account. The password is hashed immediately and plaintext is not written to disk. It then:

1. verifies the ARM64 application package;
2. builds the appliance, boot-theme, and repository Debian packages;
3. creates and verifies a signed static APT repository;
4. downloads the pinned rclone and LocalSend ARM64 releases and verifies their SHA-256 checksums;
5. builds the pinned official Raspberry Pi `rpi-image-gen` revision in an ARM64 container;
6. produces `writing-environment-pi4.img`, a compressed `.img.xz`, `SHA256SUMS`, and the matching APT release under `artifacts/pi-image/mac-*`.

To select a particular application build:

```sh
scripts/pi-image/build-on-mac.sh --app-deb /absolute/path/to/Writing.Environment_version_arm64.deb
```

For unattended trusted automation, pass a SHA-512 crypt hash beginning with `$6$` through `WRITING_ENVIRONMENT_IMAGE_PASSWORD_HASH` or `--password-hash`. Do not put a plaintext password in shell history.

For a test or distributable image with the known initial password `writer`, use:

```sh
scripts/pi-image/build-on-mac.sh --default-password
```

After booting, open Terminal and run `writing-environment-change-password` (or simply `passwd`) to replace it. The default password is convenient for first access, not suitable for a Pi exposed to untrusted users or a network service.

The personal defaults are English (US), a US-International physical keyboard map with dead keys, São Paulo time, and Brazil's Wi-Fi regulatory region. A default Compose override makes apostrophe followed by `c` or `C` produce `ç` or `Ç`, matching macOS behavior instead of producing C-acute. Override the physical layout when building; for example, a Brazilian keyboard can be selected with:

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

At first boot, the system expands the root filesystem once and may reboot automatically. Normal boot shows only the Writing Environment splash before the fullscreen writer appears. The system then logs in as `writer` and opens Writing Environment. Press `Super+Space`, select the Wi-Fi status row, and use the keyboard-oriented NetworkManager screen to join a network. Configure optional Dropbox or another provider from Terminal with:

```sh
rclone config
```

Provider credentials are created on the Pi and are never embedded in the image produced on the Mac.
The minimal browser opens automatically when rclone needs an OAuth login. LocalSend is available from the system drawer for transfers between the Pi and other devices on the same network.

## Update without reflashing

Fresh v0.5.2 images already trust the package-scoped Writing Environment repository. The repository is stored in a separate deb822 source file and does not replace Raspberry Pi OS's Debian or Raspberry Pi sources. From the system drawer, choose **Updates**, or run:

```sh
sudo apt update
sudo apt full-upgrade
```

APT verifies the repository's signed `InRelease` metadata and installs newer versions of the application, appliance shell, and boot theme together with normal Raspberry Pi OS updates. Package pinning allows this repository to provide only names beginning with `writing-environment`; the official repositories remain preferred for every other package.

To migrate an older Writing Environment Pi once v0.5.2 is published:

```sh
curl --proto '=https' --tlsv1.2 -fLO \
  https://github.com/moonknight82/writing-environment/releases/download/v0.5.2/writing-environment-repository_0.5.2-1_all.deb
sudo apt install ./writing-environment-repository_0.5.2-1_all.deb
sudo apt update
sudo apt install writing-environment-appliance
sudo reboot
```

This bootstrap installs only the repository key/source first; the subsequent signed APT transaction installs the appliance packages. Projects, Trash, History, rclone credentials, and preferences remain outside package ownership. The older archive updater remains available as an offline recovery path, but it is no longer the normal update mechanism.

## Recovery

- Press `Alt+F4` to close the writer; `Super+Space` remains available over the plain appliance background.
- Open Terminal from the system drawer for diagnostics or manual updates.
- Press `Ctrl+Super+S` to open Control Centre directly.
- Use **Power & Recovery** in the drawer to restart the writer, restart the Pi, or turn it off after confirmation.
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
- the Writing Environment application through the selected Debian package;
- the appliance package set and APT repository through the dedicated release signing key.

The image itself intentionally follows Raspberry Pi OS security and package updates after installation rather than freezing the running system forever.
