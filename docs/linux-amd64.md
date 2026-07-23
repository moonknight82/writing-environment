# Linux amd64 port

Writing Environment supports 64-bit Intel/AMD Linux as a first-class desktop target. The native installer currently targets Debian 12, Ubuntu 22.04, and newer Debian-family releases. A portable AppImage build path is included for testing on other x86-64 distributions.

The editor, project format, themes, focus controls, goals, autosave, History, Trash, sync, and recovery behavior are shared with the Raspberry Pi and macOS builds. Projects remain ordinary folders containing Markdown files.

Editor text size, line height, and sheet width are local visual preferences available in the Writer (`Aa`) panel; they do not modify Markdown contents.

## Install from source on Debian or Ubuntu

Open a terminal in the project and run:

```sh
scripts/linux-amd64/install.sh
```

This validates the x86-64 host, installs WebKitGTK/Tauri build dependencies plus English and Brazilian Portuguese spelling dictionaries, builds an optimized native executable, and installs the program under `/opt/writing-environment`. It also creates a desktop application entry and icon.

Autostart is disabled by default on a general-purpose desktop. Enable it explicitly with:

```sh
scripts/linux-amd64/install.sh --autostart
```

The XDG autostart entry works with GNOME, KDE Plasma, Cinnamon, Xfce, and other standards-compliant desktop environments. Unlike the Raspberry Pi appliance session, the amd64 app opens as an ordinary resizable window. Set `WRITING_ENVIRONMENT_APPLIANCE=1` in `~/.config/writing-environment/environment` if you prefer maximized startup.

Update or safely uninstall with:

```sh
scripts/linux-amd64/update.sh
scripts/linux-amd64/uninstall.sh
```

Uninstall removes only the program and managed launch entries. Markdown projects, recovery snapshots, Trash data, user configuration, packages, Node.js, and Rust are preserved.

## Build portable packages

The reproducible package builder uses a Debian 12 amd64 container, even when invoked on an ARM Mac or ARM Linux host:

```sh
scripts/linux-amd64/build-packages.sh
```

It exports an amd64 `.deb` and `SHA256SUMS` under a new timestamped `artifacts/linux-amd64/` directory. This also works under amd64 emulation on an ARM development machine.

On an Apple Silicon or Intel Mac with Docker Desktop running, use the purpose-named wrapper:

```sh
scripts/linux-amd64/build-on-mac.sh
```

It builds the current checkout as an x86-64 `.deb`, verifies that a package was exported, and writes `SHA256SUMS` beside it.

On a native x86-64 Docker host, build both portable formats with:

```sh
scripts/linux-amd64/build-packages.sh --with-appimage
```

Tauri's `linuxdeploy` AppImage wrapper launches nested x86-64 helper processes and is not reliable under ARM/QEMU emulation. The application and `.deb` compile correctly under emulation; AppImage packaging is therefore restricted to a native amd64 host instead of silently producing an unverified artifact.

The Debian 12 baseline is deliberate. Linux binaries inherit a minimum glibc version from their build host, and Tauri recommends building on the oldest supported system that provides WebKitGTK 4.1. Debian 12 and Ubuntu 22.04 meet that requirement.

To run the AppImage:

```sh
chmod +x Writing\ Environment_0.3.0_amd64.AppImage
./Writing\ Environment_0.3.0_amd64.AppImage
```

The exact generated filename may vary slightly with the Tauri bundler version. The AppImage is unsigned in this development milestone, so verify it against `SHA256SUMS` after transferring it.

Install the Debian package with:

```sh
sudo apt install './Writing Environment_0.3.0_amd64.deb'
```

## Compatibility baseline

| Component | Baseline |
| --- | --- |
| CPU | x86-64 / amd64 |
| Native installer | Debian 12, Ubuntu 22.04, or newer |
| Portable builder | Debian 12 amd64 container |
| Display | X11 or Wayland desktop supported by GTK/WebKitGTK |
| Web runtime | WebKitGTK 4.1 |
| Project files | UTF-8 Markdown folders |

The app does not load fonts or interface assets from the network. Editing and saving remain functional offline. Optional cloud synchronization uses a separately configured rclone 1.66-or-newer installation; provider credentials remain outside Writing Environment.
