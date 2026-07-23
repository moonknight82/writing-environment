# Raspberry Pi 4 deployment

The hardware baseline is a Raspberry Pi 4 running the 64-bit Raspberry Pi OS release based on Debian Trixie. The application can be built natively on the Pi or inside a Debian ARM64 container on an Apple Silicon Mac. Both routes produce the same self-contained artifact format and install a direct release binary instead of an AppImage.

## Recommended initial system

- Raspberry Pi 4 with 2 GB RAM or more (4 GB leaves more headroom)
- 64-bit Raspberry Pi OS with Desktop
- Official power supply
- Active or well-ventilated cooling
- USB 3 SSD when available; a good microSD card is acceptable for initial testing
- 1920×1080 display for the first layout and responsiveness pass

The normal Desktop image is the easiest base while application diagnostics are still active. The installer also supports Raspberry Pi OS Lite: if Labwc is absent, it installs `rpd-wayland-core`, `rpd-theme`, and `rpd-preferences` to add Raspberry Pi's minimal Wayland desktop.

Do not use the 32-bit Raspberry Pi OS image. The installer checks for `aarch64` and Raspberry Pi 4 hardware before modifying the machine.

For the appliance-style experience, build the [dedicated Raspberry Pi image](pi-image.md). It starts from Lite, defaults the desktop to Dark Mode, keeps Light Mode available through Appearance Settings, and omits the general-purpose application bundles.

## Install on the Pi

Copy or clone this repository onto the Pi, open a terminal in it, and run:

```sh
scripts/pi/install.sh
```

The script:

1. verifies the Pi model, ARM64 architecture, and Debian-family OS;
2. installs the Raspberry Pi/Tauri build dependencies;
3. installs pnpm and the stable Rust toolchain if they are missing;
4. runs the locked frontend install and a native optimized Tauri build;
5. installs the binary under `/opt/writing-environment`;
6. installs a launcher, application-menu entry, and scalable icon;
7. adds one marked launch command to the user's Labwc autostart file.

The application opens in borderless maximized presentation mode at the next graphical login. It is not a locked kiosk: F11 returns it to a normal resizable window, while `Alt+F4`, Terminal, and the Raspberry Pi desktop remain available for recovery and diagnostics. Writing Environment's own distraction-free control can hide its sidebars while writing.

Useful installation variants:

```sh
# Install without opening the app automatically at login
scripts/pi/install.sh --no-autostart

# Rebuild after changing source code, preserving the current autostart choice
scripts/pi/update.sh

# Fast-forward a clean Git checkout before rebuilding
scripts/pi/update.sh --pull
```

The initial build can take several minutes on a Pi 4. Later builds reuse Rust and pnpm caches.

## Configure optional cloud sync

The installer also adds English and Brazilian Portuguese Hunspell dictionaries and initializes WebKitGTK with both languages for native check-as-you-type spelling. Spell checking is enabled by default in the app; automatic correction remains opt-in in the Writer (`Aa`) panel.

The same Writer panel controls editor-only text size, line height, and sheet width. These visual preferences never alter the Markdown files.

The normal installer adds a compatible current rclone release from rclone's official installer, but leaves it unconfigured. Raspberry Pi OS/Debian's packaged rclone is currently too old for the conflict and backup safeguards used here. Run this once as the same desktop user who launches Writing Environment:

```sh
rclone config
rclone listremotes
```

Create a Dropbox remote or another rclone-supported remote. Writing Environment never reads or stores the provider password or OAuth token itself; those remain in rclone's user configuration. A Mac-built application artifact also does not contain or transfer credentials.

Then open the project in Writing Environment and choose **Sync**:

1. select the configured remote;
2. choose a new, empty remote folder;
3. select **Initialize sync**;
4. optionally enable **Automatic sync** after initialization succeeds.

Automatic sync runs after a settled local save and every five minutes while the app is open. It is off by default. The editor remains local-first when the network is unavailable, and a sync error does not block local saving.

Each synchronized project contains a hidden `.writing-environment-sync` access marker. Recovery snapshots and Trash remain outside the project and are not uploaded. Replaced versions are retained under the app-data sync profile and a sibling remote folder ending in `.writing-environment-archive`. Simultaneous edits are preserved as additional Markdown files containing `writing-environment-conflict` in their names.

## Build on an Apple Silicon Mac

Install and start Docker Desktop, then run this from the project root:

```sh
scripts/pi/build-on-mac.sh
```

The build runs in a native Linux ARM64 container and uses Debian Bookworm as a conservative compatibility baseline. Its output is written under a timestamped `artifacts/pi-arm64/mac-*` directory. The directory contains a `.tar.gz` artifact and its SHA-256 checksum.

Copy only the `.tar.gz` file to the Pi, then install it without compiling:

```sh
tar -xzf writing-environment-pi-arm64-*.tar.gz
cd writing-environment-pi-arm64-*
./install.sh
```

For an existing installation, preserve its current autostart choice:

```sh
./install.sh --skip-packages --preserve-autostart
```

The first Docker build downloads the Linux toolchain and dependencies. Docker cache mounts retain the Rust registry, Git dependencies, and compiled target files for faster subsequent builds.

## Build an artifact on the Pi

To create the identical distributable format natively without installing it immediately:

```sh
scripts/pi/build-on-pi.sh
```

After the initial dependency setup, use `--skip-packages` for later builds:

```sh
scripts/pi/build-on-pi.sh --skip-packages
```

Native artifacts are written under timestamped `artifacts/pi-arm64/pi-*` directories. Extract and run their included `install.sh` in the same way as a Mac-built artifact.

Every prebuilt installer verifies the archive's internal SHA-256 manifest, confirms that the executable is Linux ARM64, checks dynamic-library availability, and refuses unsupported hardware unless explicitly allowed. Project folders, recovery snapshots, Trash, and existing preferences are never included in or removed by an artifact installation.

## What is installed

| Purpose | Location |
| --- | --- |
| Release executable | `/usr/bin/writing-environment` (`/opt/writing-environment/writing-environment` remains a compatibility fallback for older images) |
| Single-instance launcher | `/usr/local/bin/writing-environment` |
| Desktop entry | `/usr/local/share/applications/writing-environment.desktop` |
| Icon | `/usr/local/share/icons/hicolor/scalable/apps/writing-environment.svg` |
| Optional session overrides | `~/.config/writing-environment/environment` |
| Managed launch block | `~/.config/labwc/autostart` |

The launcher uses a per-user lock so a menu click cannot create a second copy when the boot-launched app is already running. It sets `WRITING_ENVIRONMENT_APPLIANCE=1`, which asks the app to maximize its main window. To use a normal-sized startup window, uncomment this line in `~/.config/writing-environment/environment`:

```sh
WRITING_ENVIRONMENT_APPLIANCE=0
```

The launcher sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` by default. Physical Pi 4 testing under Labwc/Wayland showed a corrupted first frame through WebKitGTK's DMA-BUF renderer; minimizing and restoring the window forced a clean repaint. The shared-memory fallback starts cleanly. To retest the hardware renderer after a future WebKitGTK or Mesa update, add this to the same environment file:

```sh
WEBKIT_DISABLE_DMABUF_RENDERER=0
```

## Uninstall safely

```sh
scripts/pi/uninstall.sh
```

Uninstall removes only the installed executable, launcher, desktop entry, icon, and the marked Labwc autostart block. It deliberately preserves:

- every selected project and Markdown document;
- recovery snapshots and Trash data under the application's user-data directory;
- `~/.config/writing-environment`;
- Debian packages, Node.js, pnpm, and Rust.

This means reinstalling cannot destroy a manuscript and uninstall does not unexpectedly remove shared development tools.

## Physical Pi performance pass

Generate a disposable test project containing 1,000 small sheets and one roughly 100,000-word sheet:

```sh
scripts/pi/generate-test-library.sh
```

Open the printed folder path as a project in Writing Environment. Then capture a system and idle-process report:

```sh
scripts/pi/benchmark.sh --launch
```

Reports are written under `reports/pi/` and ignored by Git. Each report records the OS, kernel, architecture, memory, storage, Pi thermal/throttling state, executable size, and application process RSS/CPU, followed by the manual acceptance checklist.

The current provisional gate is:

- ordinary typing has no visible input lag;
- project navigation stays responsive with 1,000 Markdown files;
- a 100,000-word sheet remains editable;
- idle resident memory remains below 350 MB;
- editing and saving continue with networking disconnected;
- `vcgencmd get_throttled` remains `0x0` after the stress pass.

If ordinary typing misses that gate, the WebKitGTK interface should be profiled before more major features are added. The Rust persistence layer and portable Markdown project format can remain unchanged if a GTK4 interface later becomes necessary.

## Current data-safety behavior

- The selected project remains the source of truth.
- Only existing `.md` files inside that project can be read or saved.
- Relative paths containing parent traversal are rejected.
- Each save snapshots the previous file under the application's user-data directory.
- The History button lists those snapshots, previews their Markdown and word delta, and restores them without overwriting the current version's recovery copy.
- Snapshots stay outside the selected project so cloud synchronization does not recursively synchronize recovery history.
- Each sheet retains the newest 30 recovery snapshots.
- Saving writes and flushes a temporary file in the manuscript directory before replacing the original.
- The original file permissions are applied to the replacement.

- Optional remote synchronization is coordinated through rclone and remains separate from local saving.
- The first remote folder must be empty; the app will not automatically merge an unknown pre-existing remote tree.
- Automatic sync is opt-in and can be disabled without affecting local autosave.
- Sync conflicts are preserved as additional visible Markdown files instead of choosing a winner.
