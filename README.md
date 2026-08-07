# Writing Environment

A distraction-free, local-first Markdown writing environment designed first for a Raspberry Pi 4 with 2 GB of RAM.

> **Personal-project status:** Writing Environment is currently a personal, experimental project shared publicly for testing and learning. It is not a supported commercial product, has no guaranteed release schedule, and should not be treated as the only copy of important writing.

The application is portable across Raspberry Pi, Linux amd64, and macOS. Its dedicated Raspberry Pi image uses Raspberry Pi OS Lite and a minimal Labwc session to create a calm writing appliance without locking away essential system controls.

## Current milestone

The v0.1 desktop vertical slice and dedicated Raspberry Pi appliance image are complete. Version 0.2 established large-library performance and reliability, v0.3 added signed application updates, v0.4 unified Inbox and project workflows, v0.5 completed portable document export, and v0.6 makes navigation and multi-sheet organization practical across larger libraries. The repository contains:

- three-pane library, sheet, and editor layout;
- focus mode and independently collapsible sidebars;
- caret-aware paragraph and sentence writing focus, with an Off mode;
- a dedicated Projects area with persistent recent projects and pinning;
- a permanent Inbox of ordinary Markdown files for writing before choosing a project;
- native Markdown-folder selection;
- a live split Markdown Preview beside the writable editor, with sanitized rendering for headings, emphasis, lists, quotations, code, and tables;
- recursive Markdown discovery and metadata extraction;
- sheet creation, rename, duplication, folder moves, and explicit multi-selection for batch move, export, or Trash actions;
- safe Inbox-to-project moves that atomically place and verify the destination before removing the source;
- universal, origin-labelled Trash stored outside manuscripts, with original-location and Inbox-fallback restoration plus scoped Empty Trash confirmation;
- full-text library search through a rebuildable, project-local SQLite FTS index, with direct Markdown scanning as a safe fallback;
- global search across Inbox and every open project, plus a keyboard-first Quick Switcher with local recent and favorite sheets;
- coalesced native project-folder notifications for Markdown changes made by sync tools, file managers, or other editors;
- guarded autosave that detects concurrent disk changes and offers to keep both versions instead of overwriting either one;
- optional reopening of the last project and sheet at launch;
- debounced local autosave using atomic file replacement;
- optional provider-neutral universal two-way sync through rclone, including Dropbox remotes;
- one remote root with Inbox/project inclusion controls, aggregate status, and isolated per-location safety profiles;
- manual sync plus opt-in sequential automatic sync after saves and every five minutes;
- empty-remote initialization, access checks, bounded deletion, and conflict-copy preservation;
- recovery snapshots stored outside the selected library;
- a sheet History browser with timestamped previews, word deltas, and reversible restore;
- current-sheet, folder, and complete-project DOCX, PDF, and EPUB export through a native Save dialog, with ordering, metadata, title-page, section-break, and reusable-preset controls;
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

Version 0.3.2 corrects desktop workspace restoration after updating. With no project open, the desktop app now stays genuinely empty; disabling last-workspace reopening clears the saved workspace immediately; only the active project and favorites remain in the sidebar; and the active project can be unloaded with **Right-click → Close Project**.

Version 0.4.0 begins the unified-library milestone. Opened projects are retained in a persistent registry, the active project expands into its real nested folder tree, parent folders include descendant sheets, and sheet creation or moves can target safe nested paths such as `Research/Locations`.

Version 0.4.1 makes open projects explicit and durable. Switching projects no longer removes the previous project from the sidebar, each project remembers its last sheet, and **Move to another project or folder** can transfer a sheet safely between any two open projects. A Markdown file with invalid UTF-8 bytes is skipped with a precise warning instead of preventing the rest of its project from opening; the unreadable file is never rewritten or deleted.

Version 0.4.2 turns rclone's excessive-deletion abort into a guided recovery state. When more than 25% of tracked files appear missing on either side, sync makes no changes, disables automatic sync for that project, and offers to restore only missing files from the complete side. Recovery uses `--ignore-existing`: it never overwrites an existing file, never bypasses the deletion limit with `--force`, and leaves automatic sync off until the writer deliberately re-enables it.

Version 0.4.3 establishes **files over app** as a compatibility rule. Every readable Markdown file in an opened folder or any nested subfolder is a sheet, whether it was created by Writing Environment, Obsidian, another editor, or a cloud client. UTF-8 (with or without a byte-order mark), UTF-16, UTF-32, and Windows-1252 text are decoded; editing a legacy-encoded file safely normalizes it to UTF-8. macOS AppleDouble `._*.md` metadata is ignored rather than mistaken for prose. Reads retry briefly across cloud-client replacement boundaries, and sync can replace an unreadable local `.md` file with its valid remote counterpart only after preserving the original bytes in app recovery data.

Version 0.4.4 completes the unified-library interface. Inbox is a permanent editable Markdown workspace, Trash aggregates recoverable sheets from Inbox and every registered project, and sheets can move safely between open projects. Sync now uses one universal rclone root with separate Inbox/project inclusion controls, sequential isolated jobs, aggregate status, explicit empty-destination initialization, and stable cross-device project identities. Existing project sync profiles remain at their previous remote locations with automatic sync disabled until the writer deliberately chooses how to proceed.

Version 0.4.5 removes whole-document work from the editor's keystroke path. Word and session counts are coalesced after a short typing pause, while Writing Focus keeps persistent overlay nodes, scans only the active paragraph for boundaries, and remeasures scrollbar geometry only after an actual layout change. Autosave timing and disk-conflict protection are unchanged.

Version 0.4.6 moves native autosave persistence off the interactive application thread. History snapshots, atomic file replacement, and search-index maintenance run on a blocking worker after 1.2 seconds of typing inactivity. Filesystem notifications caused by the app's own save are coalesced until editing and saving are idle, while external-change and disk-conflict protection remain active.

Version 0.5.0 introduces one-way document export for the current in-memory sheet. The Export menu creates editable DOCX, fixed-layout Letter PDF, or standards-compliant reflowable EPUB files without saving formatting back into Markdown. PDF uses locally bundled Source Serif 4 fonts; EPUB packages self-contained XHTML, navigation, metadata, and CSS.

Version 0.5.1 completes the export milestone with selected-folder and complete-project assembly. Writers can choose alphabetical or creation-date ordering, a title page, section page breaks, title/author/language metadata, and reusable export presets. All three formats omit YAML front matter, avoid duplicating matching opening titles, and write atomically beside the chosen destination.

Version 0.5.5 adds a safe formatted Markdown preview and makes an empty Inbox explicitly non-editable until a sheet is created. It also prevents an incomplete GitHub Release from becoming the updater's latest release before its signed update manifest is available. Version 0.5.4 removes Raspberry Pi's incompatible Desktop/Taskbar appearance plugin from the panel-free appliance Control Centre. Version 0.5.3 made the writer visually fullscreen while allowing Raspberry Pi settings windows and dialogs to rise normally, and added a dedicated Display Settings launcher. The signed APT repository introduced in 0.5.2 updates the application, Fuzzel/Labwc shell, keyboard defaults, settings integration, and Plymouth theme alongside the normal Raspberry Pi OS packages.

Version 0.6.0 adds global, origin-labelled search across Inbox and every open project; a keyboard-first Quick Switcher with local recent and favorite sheets; and an explicit multi-selection mode for moving, trashing, or exporting several sheets. These navigation preferences remain local interface state and never add metadata to Markdown folders. Batch file mutations retain the existing per-file atomic safety rules and report partial progress if an item prevents the batch from continuing.

Version 0.7.0 gives toolbar, sort, project, and sheet-action popovers consistent desktop behavior. Menus dismiss on outside clicks, Escape, or Tab; only one remains open; and keyboard users can navigate with arrows, Home/End, Enter/Space, and open project context menus with Shift+F10 or the Menu key. Focus returns to the originating control after Escape, with a quiet visible focus treatment inside menus.

Version 0.7.1 keeps the editor writable while formatted Markdown Preview is active, using equal side-by-side panes on desktop and a stacked split on narrow windows. It also adds an automatic WebKitGTK rendering fallback for NVIDIA Linux systems and publishes a portable Linux x86-64 AppImage alongside the Debian package.

Version 0.7.2 supersedes the source-only v0.7.1 tag after a GitHub Actions outage prevented that release from building. Linux x86-64 releases now include a Fedora RPM in addition to the Debian package and portable AppImage.

Update artifacts and `latest.json` are published through [GitHub Releases](https://github.com/moonknight82/writing-environment/releases). Every application update is verified with Tauri's embedded public signing key before installation. The private key is held only in the repository's GitHub Actions secrets. macOS uses a signed Tauri application archive, Linux amd64 publishes Debian and AppImage formats, and Raspberry Pi uses signed Debian packages.

The in-app updater remains application-only on desktop systems. On the Raspberry Pi appliance, the recommended Super+Space **Updates** action uses the signed APT repositories and can update both Raspberry Pi OS and the appliance packages. Neither path modifies projects, History, Trash, rclone credentials, or application preferences.

The repeatable 10,000-sheet release benchmark currently measures an approximately 5.4-second one-time index build, sub-millisecond indexed search, and an approximately 40 ms unchanged-library refresh on the Apple Silicon development host. The physical Raspberry Pi 4 with 2 GB remains the release baseline.

The plain browser build retains a safe prototype library and stores its sample draft in browser storage. Real folder access is enabled only inside the Tauri desktop application.

Remote credentials are not stored by Writing Environment. Configure a provider once with `rclone config`, then use Sync to choose one universal remote root and include Inbox or individual projects. Every new destination requires explicit confirmation and must be empty; automatic sync remains off until all included locations initialize successfully. Existing project sync settings are preserved at their old locations with automatic sync disabled, and are never moved merely by opening the new interface.

History keeps the newest 30 pre-save versions for each sheet outside the synchronized project. Restoring a revision snapshots the current sheet first, so a restore can itself be undone.

The editor toolbar's **Export** menu writes the current in-memory sheet, an explicit sheet selection, a selected folder, or the complete active project as DOCX, PDF, or EPUB without changing its Markdown source. Multi-sheet exports can be ordered alphabetically or by creation date, add a title page and author byline, and begin each sheet on a new page or EPUB chapter. Title, author, language, scope, ordering, and layout choices can be saved as reusable presets. DOCX and PDF use a Letter manuscript profile with one-inch margins, 12 pt double-spaced body paragraphs, half-inch first-line indents, and dedicated treatments for titles, headings, quotations, lists, code, and scene breaks. EPUB uses the same parsed structure in reflowable XHTML with local CSS, metadata, and navigation. YAML front matter is omitted, and a leading `#` heading that matches the sheet title is not duplicated. Export writes through a temporary file and atomically replaces the chosen destination. Image embedding and Markdown tables remain outside this release.

PDF export embeds [Source Serif 4](https://github.com/adobe-fonts/source-serif), distributed under the SIL Open Font License 1.1. The bundled license is kept in `src-tauri/assets/fonts/LICENSE.md`.

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

The Mac build now exports both the manual archive and the ARM64 application `.deb`. To build a complete flashable Raspberry Pi OS image around the newest Debian package and generate the matching signed APT release:

```sh
scripts/pi-image/build-on-mac.sh
```

The image boots directly into Writing Environment in a borderless maximized presentation view. Press `Super+Space` to open a curated Fuzzel system drawer above the writer; it reports current Wi-Fi, Bluetooth, update, and Pi power status and provides Files, Terminal, Browser, LocalSend, settings, display, restart, and shutdown actions. F11 or the presentation toolbar button returns to a normal window. Labwc remains the lightweight compositor, while the persistent desktop panel and desktop icons are not started. Raspberry Pi Control Centre provides the native configuration tools, and advanced administration remains available through `sudo raspi-config`. See the [dedicated Pi image guide](docs/pi-image.md).

The public APT repository is served from `https://moonknight82.github.io/writing-environment/apt`. Its source file is additive: Debian and Raspberry Pi repositories remain responsible for the operating system, kernel, firmware, and ordinary packages.

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

Use `--with-portable` on a native x86-64 Docker host to create `.deb`, AppImage, and RPM packages. Tagged GitHub releases publish all three formats: `.deb` for Debian/Ubuntu, RPM for Fedora, and AppImage as the portable fallback for other x86-64 distributions. The former `--with-appimage` option remains available as an alias.

See the [Linux amd64 guide](docs/linux-amd64.md) for compatibility, packaging, update, and uninstall details.

On Apple Silicon macOS, the signed local release script is:

```sh
scripts/macos/build.sh
```

Release bundles require the updater signing key through `TAURI_SIGNING_PRIVATE_KEY`; the local macOS and Docker build scripts load it from `~/.tauri/writing-environment.key` by default. Ordinary development with `pnpm tauri dev` does not require the private key.

## Publishing a release

The public release workflow builds macOS Apple Silicon, Linux amd64, and Raspberry Pi ARM64 packages. To publish, update the version consistently in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, commit it, then push the matching tag—for example `v0.5.0`. GitHub Actions tests, builds, signs, and publishes the packages and updater manifest. See [Release process](docs/releases.md) for signing-key recovery and release checks.

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
- [Unified library model](docs/unified-library.md)
- [Release process](docs/releases.md)
