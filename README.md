# Writing Environment

A distraction-free, local-first Markdown writing environment designed first for a Raspberry Pi 4 with 2 GB of RAM.

> **Personal-project status:** Writing Environment is currently a personal, experimental project shared publicly for testing and learning. It is not a supported commercial product, has no guaranteed release schedule, and should not be treated as the only copy of important writing.

The application is portable across Raspberry Pi, Linux amd64, and macOS. Its dedicated Raspberry Pi image uses Raspberry Pi OS Lite and a minimal Labwc session to create a calm writing appliance without locking away essential system controls.

## Current milestone

The v0.1 desktop vertical slice and dedicated Raspberry Pi appliance image are complete. Version 0.2 established large-library performance and reliability; v0.3 adds signed application updates and reproducible public releases. The repository contains:

- three-pane library, sheet, and editor layout;
- focus mode and independently collapsible sidebars;
- caret-aware paragraph and sentence writing focus, with an Off mode;
- a dedicated Projects area with persistent recent projects and pinning;
- native Markdown-folder selection;
- recursive Markdown discovery and metadata extraction;
- sheet creation, rename, duplication, and folder moves;
- recoverable Trash stored outside the selected project, with an explicit project-scoped Empty Trash action;
- full-text library search through a rebuildable, project-local SQLite FTS index, with direct Markdown scanning as a safe fallback;
- coalesced native project-folder notifications for Markdown changes made by sync tools, file managers, or other editors;
- guarded autosave that detects concurrent disk changes and offers to keep both versions instead of overwriting either one;
- optional reopening of the last project and sheet at launch;
- debounced local autosave using atomic file replacement;
- optional provider-neutral two-way sync through rclone, including Dropbox remotes;
- manual sync status plus opt-in automatic sync after saves and every five minutes;
- empty-remote initialization, access checks, bounded deletion, and conflict-copy preservation;
- recovery snapshots stored outside the selected library;
- a sheet History browser with timestamped previews, word deltas, and reversible restore;
- path-traversal protection and preserved file permissions;
- persistent editor text-size, line-height, and sheet-width controls;
- native system spell checking with optional automatic correction;
- a persistent session word goal with live net-word progress;
- declarative visual themes with live switching, including Old Terminal;
- responsive behavior for smaller displays;
- no network-loaded fonts or visual assets.

## Application updates

Version 0.3.0 is the updater bootstrap release. Install it manually once on each machine; later releases can be checked from **Writer (Aa) → Application updates → Check for Updates…**. Automatic checks are enabled by default, run at most once per day, and never install or restart without confirmation.

Version 0.3.1 is the first updater-delivered maintenance release. It corrects writing-focus text wrapping on macOS when the editor has a visible scrollbar, including long Markdown paragraphs created on Raspberry Pi.

Update artifacts and `latest.json` are published through [GitHub Releases](https://github.com/moonknight82/writing-environment/releases). Every application update is verified with Tauri's embedded public signing key before installation. The private key is held only in the repository's GitHub Actions secrets. macOS uses a signed Tauri application archive; Linux amd64 and Raspberry Pi use signed Debian packages and request system authorization when installation begins.

Application updates do not modify Raspberry Pi OS, desktop settings, rclone configuration, projects, or recovery data. Appliance-level changes remain separate, deliberate updates or full-image releases.

The repeatable 10,000-sheet release benchmark currently measures an approximately 5.4-second one-time index build, sub-millisecond indexed search, and an approximately 40 ms unchanged-library refresh on the Apple Silicon development host. The physical Raspberry Pi 4 with 2 GB remains the release baseline.

The plain browser build retains a safe prototype library and stores its sample draft in browser storage. Real folder access is enabled only inside the Tauri desktop application.

Remote credentials are not stored by Writing Environment. Configure a provider once with `rclone config`, open a local project, and use the Sync control. The first sync accepts only an empty remote folder; automatic sync remains off until initialization succeeds.

History keeps the newest 30 pre-save versions for each sheet outside the synchronized project. Restoring a revision snapshots the current sheet first, so a restore can itself be undone.

## Run the prototype

Requirements: Node.js 20 or newer and pnpm.

```sh
pnpm install
pnpm dev
```

Create a production frontend build with:

```sh
pnpm build
```

## Run the desktop application

Install the stable Rust toolchain, then run:

```sh
pnpm tauri dev
```

Build the native executable without an installer:

```sh
pnpm tauri build --no-bundle
```

## Install on a Raspberry Pi 4

On a Raspberry Pi 4 running 64-bit Raspberry Pi OS, the deployment kit installs dependencies, builds the native ARM64 application, adds its desktop launcher, and configures a reversible Labwc autostart entry:

```sh
scripts/pi/install.sh
```

Update the installed build from the current checkout with `scripts/pi/update.sh`, or remove the program safely with `scripts/pi/uninstall.sh`. Uninstall never removes Markdown projects, recovery data, or development toolchains. See the [Raspberry Pi 4 deployment guide](docs/raspberry-pi-4.md) for OS preparation, available flags, installed paths, and the physical performance test.

To keep compilation off the Pi, an Apple Silicon Mac with Docker Desktop can produce a checksummed ARM64 artifact:

```sh
scripts/pi/build-on-mac.sh
```

The matching `scripts/pi/build-on-pi.sh` produces the same artifact format natively. Each archive includes its own verified `install.sh`, so deployment to the Pi does not require Rust, Node.js, pnpm, or the source tree.

To build a complete flashable Raspberry Pi OS image around the newest ARM64 artifact:

```sh
scripts/pi-image/build-on-mac.sh
```

The image boots directly into Writing Environment in a distraction-free, borderless presentation mode. Press `Ctrl+Tab` to show or hide the complete bottom panel; F11 or the presentation toolbar button returns to a normal window. Labwc remains the lightweight compositor, but it does not implement the Wayfire-only edge-hotspot protocol used by `wf-panel-pi`, so the image toggles the panel's visibility directly instead of relying on pointer-edge reveal. Raspberry Pi Control Centre provides the Light/Dark appearance controls, while advanced administration remains available through `sudo raspi-config`. The image deliberately includes only the minimal desktop shell, Files, Terminal, appearance controls, Wi-Fi and Bluetooth, audio, removable-media handling, updates, and rclone. See the [dedicated Pi image guide](docs/pi-image.md).

## Install on Linux amd64

The desktop-neutral x86-64 port supports Debian 12, Ubuntu 22.04, and newer Debian-family systems:

```sh
scripts/linux-amd64/install.sh
```

Autostart is opt-in with `--autostart`. A reproducible Debian 12 container produces an amd64 `.deb`, even from an ARM host:

```sh
scripts/linux-amd64/build-packages.sh
```

On macOS, the explicit entry point is:

```sh
scripts/linux-amd64/build-on-mac.sh
```

Use `--with-appimage` on a native x86-64 Docker host to create both `.deb` and AppImage packages.

See the [Linux amd64 guide](docs/linux-amd64.md) for compatibility, packaging, update, and uninstall details.

On Apple Silicon macOS, the signed local release script is:

```sh
scripts/macos/build.sh
```

Release bundles require the updater signing key through `TAURI_SIGNING_PRIVATE_KEY`; the local macOS and Docker build scripts load it from `~/.tauri/writing-environment.key` by default. Ordinary development with `pnpm tauri dev` does not require the private key.

## Publishing a release

The public release workflow builds macOS Apple Silicon, Linux amd64, and Raspberry Pi ARM64 packages. To publish, update the version consistently in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, commit it, then push the matching tag—for example `v0.3.0`. GitHub Actions tests, builds, signs, and publishes the packages and updater manifest. See [Release process](docs/releases.md) for signing-key recovery and release checks.

Native system spelling and grammar checking are enabled by default and can be changed in the Writer (`Aa`) panel. Linux builds initialize WebKitGTK with the bundled English (US) and Brazilian Portuguese Hunspell dictionaries; macOS builds enable WebKit's continuous spelling and grammar services. Writing Focus preserves the operating system's native underlines. Automatic correction remains off by default so the operating system cannot silently rewrite manuscript text unless the writer opts in.

## Project documents

- [Product brief](docs/product-brief.md)
- [Architecture](docs/architecture.md)
- [Markdown library format](docs/file-format.md)
- [Visual theme system](docs/theme-system.md)
- [Raspberry Pi 4 setup](docs/raspberry-pi-4.md)
- [Dedicated Raspberry Pi image](docs/pi-image.md)
- [Linux amd64 port](docs/linux-amd64.md)
- [Development plan](docs/development-plan.md)
- [Release process](docs/releases.md)
