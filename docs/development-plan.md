# Development plan

## v0.1 — Writing appliance foundation

Completed:

- local-first Markdown projects, sheets, search, History, Trash, autosave, focus controls, goals, themes, and rclone synchronization;
- macOS, Linux amd64, and Raspberry Pi ARM64 application builds;
- a minimal Labwc-based Raspberry Pi 4 image tested on the 2 GB model;
- stable presentation mode, panel access, native spelling support, and clean first-frame rendering on the physical Pi.

The confirmed v0.1 appliance image remains the stable fallback while v0.2 is tested.

## v0.2 — Library scale and reliability

Current work:

- [x] Add a rebuildable SQLite FTS index outside Markdown project folders.
- [x] Incrementally reconcile new, changed, and removed Markdown files when a project reloads.
- [x] Refresh the active sheet's index entry after local autosave.
- [x] Fall back to direct Markdown scans if index creation, repair, or search fails.
- [x] Add correctness tests for changes, removals, saves, multi-term search, and corrupt-index recovery.
- [x] Add a repeatable 10,000-sheet host benchmark.
- [x] Cross-build and package the indexed application on macOS, Linux amd64, and Raspberry Pi ARM64.
- [ ] Smoke-test project opening, saved-text search, and index rebuilding on each packaged platform.
- [ ] Run the cold-index, warm-refresh, search, 100,000-word editing, and memory benchmarks on the physical 2 GB Raspberry Pi.
- [x] Add coalesced external-file notifications so changes made outside the app can refresh without a manual project reload.
- [x] Guard autosave with the last known disk version and preserve local edits as a visible conflict copy when both versions change.
- [x] Expand interrupted-write and synchronization-conflict durability tests, including pre-rename interruption, permission preservation, keep-both conflict copies, isolated sync profiles, access-marker preservation, and complete rclone safety arguments.

Release criteria:

- ordinary typing remains free of perceptible lag;
- indexed search remains effectively immediate with 10,000 sheets;
- unchanged project reopening avoids rereading every manuscript;
- index loss or corruption never affects Markdown contents and repairs automatically;
- the 100,000-word editor and 10,000-sheet library remain usable within the established Pi memory budget.

## v0.3 — Signed distribution and updates

Current work:

- [x] Add a visible manual update check and opt-in/out daily automatic checks.
- [x] Save locally before installation and require confirmation before restarting.
- [x] Verify every downloaded update with a dedicated Tauri signing key.
- [x] Build macOS Apple Silicon, Linux amd64, and Linux ARM64 releases in GitHub Actions.
- [x] Publish release assets and a platform-aware `latest.json` through GitHub Releases.
- [x] Preserve a manual Raspberry Pi bootstrap kit for the first updater-enabled installation.
- [ ] Install v0.3.0 manually and smoke-test the first GitHub-delivered update on each physical platform.
- [ ] Add Apple Developer ID signing and notarization if the project moves beyond personal testing.

Application updates remain separate from Raspberry Pi appliance/OS updates.

## v0.4 — Unified library

Planned in staged, testable increments:

1. [x] Make the unloaded desktop state genuinely empty and add safe project closing.
2. [x] Add a persistent project registry and show real nested project folders.
   Open projects now remain in the sidebar while switching, remember their last sheets, and accept verified cross-project sheet moves.
3. [ ] Add a universal Inbox with safe moves into projects.
4. [ ] Aggregate origin-partitioned Trash into a universal recovery view.
5. [ ] Replace per-project sync configuration in the interface with one universal remote root and per-project inclusion toggles, while retaining isolated sync jobs underneath.
6. [ ] Migrate existing sync preferences without silently uploading or deleting files.

See [Unified library model](unified-library.md) for the storage, recovery, and sync boundaries.

## v0.5 — Export

Planned after the unified library is stable:

1. DOCX export with a small set of predictable manuscript styles.
2. Print-ready PDF export using the same style model.
3. EPUB export after headings, scene breaks, metadata, and asset handling are defined.

Exported files are products of Markdown projects. They do not replace or modify the source manuscripts.

## Later work

- plugin architecture and marketplace;
- collaboration;
- mobile applications.
