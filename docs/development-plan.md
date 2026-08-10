# Development plan

## v0.1 — Writing appliance foundation

Completed:

- local-first Markdown projects, sheets, search, History, Trash, autosave, focus controls, goals, themes, and rclone synchronization;
- macOS, Linux amd64, and Raspberry Pi ARM64 application builds;
- a minimal Labwc-based Raspberry Pi 4 image tested on the 2 GB model;
- stable borderless presentation mode, Super+Space system-drawer access, native spelling support, and clean first-frame rendering on the physical Pi.

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
- [x] Enforce files-over-app interoperability: recursively open metadata-free Markdown from other editors, decode common text encodings, ignore macOS metadata sidecars, and preserve unreadable local bytes before remote repair.
- [x] Guard autosave with the last known disk version and preserve local edits as a visible conflict copy when both versions change.
- [x] Expand interrupted-write and synchronization-conflict durability tests, including pre-rename interruption, permission preservation, keep-both conflict copies, isolated sync profiles, access-marker preservation, and complete rclone safety arguments.
- [x] Remove whole-document word counting, focus-overlay reconstruction, and repeated layout measurement from the keystroke path.
- [x] Move native autosave persistence off the application event thread and coalesce self-generated filesystem refreshes until editing and saving are idle.

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
3. [x] Add a universal Inbox with safe moves into projects.
   Inbox is an ordinary visible Markdown folder under the user's Documents directory. It remains
   available without an open project, supports editing, search, History, and external-file refresh,
   and moves sheets into any open project through an atomic, byte-verified destination copy.
4. [x] Aggregate origin-partitioned Trash into a universal recovery view.
   Existing project and Inbox recovery partitions remain intact. The application adds their items
   to one origin-labelled view, restores to the original location when available, offers Inbox as a
   fallback when it is not, and can empty all origins or one selected origin after confirmation.
5. [x] Replace per-project sync configuration in the interface with one universal remote root and per-location inclusion toggles, while retaining isolated sync jobs underneath.
   Inbox and registered projects now derive deterministic destinations from one rclone remote root,
   execute sequentially with separate profiles and deletion guards, and report aggregate status.
   Every new destination requires an explicit first-sync confirmation and must be empty.
6. [x] Migrate existing sync preferences without silently uploading or deleting files.
   Existing initialized project profiles remain at their prior remote paths with automatic sync off.
   A project moves into the universal layout only through an explicit action; its old remote folder
   and recovery data are left untouched. A portable optional project UUID keeps the new destination
   stable across computers without becoming a requirement for opening Markdown.

See [Unified library model](unified-library.md) for the storage, recovery, and sync boundaries.

## v0.5 — Export

Completed:

1. [x] Export the current in-memory sheet to DOCX through a native Save dialog, using a small set of predictable manuscript styles and an atomic destination write.
2. [x] Assemble a folder or complete project into one ordered DOCX, PDF, or EPUB without introducing proprietary manuscript metadata.
3. [x] Add a compact export panel for scope, ordering, title-page, section-break, metadata, and reusable-preset choices.
4. [x] Export a print-ready Letter PDF using the same parsed document and style model, embedded Unicode fonts, and deterministic pagination.
5. [x] Export a valid EPUB 3 publication with reflowable XHTML, metadata, navigation, and local CSS.
6. [x] Embed title, author, and language metadata consistently across all three formats.

Exported files are products of Markdown projects. They do not replace or modify the source manuscripts.

## Editor polish

Current work:

- [x] Replace the writable blank Inbox surface with a clear empty state and explicit Create Sheet action.
- [x] Add a sanitized Markdown preview that renders headings, bold, italics, lists, quotations, code, and tables without changing source files.
- [x] Keep Write mode on the low-latency native textarea so preview parsing does not enter the keystroke path or replace operating-system spell checking.
- [x] Keep GitHub releases private until the updater manifest and all signed platform artifacts exist.
- [ ] Smoke-test Write/Preview switching, empty Inbox behavior, spelling, Writing Focus, and updater checks on macOS, Linux amd64, and the physical 2 GB Raspberry Pi.

## v0.6 — Navigation and organization

Current work:

- [x] Search Inbox and every open project from the sheet list while preserving each result's origin.
- [x] Add a keyboard-first Quick Switcher with Command/Control+P, arrow-key navigation, Enter to open, and Escape to close.
- [x] Add recent and favorite sheets without writing metadata into Markdown project folders.
- [x] Add explicit multi-selection for moving, trashing, and exporting sheets, with visible partial-progress reporting if a batch stops on an error.
- [ ] Run the [v0.6 cross-platform test checklist](v0.6-test-checklist.md) on macOS, Linux amd64, and the physical 2 GB Raspberry Pi.

## v0.7 — Interaction polish and accessibility

Planned:

- [x] Replace independent menu visibility toggles with one transient-popover controller for toolbar, sort, project, and sheet-action menus.
- [x] Close an open popover when the writer clicks outside it, while preserving clicks and text entry inside the popover.
- [x] Close popovers with Escape, return focus to the button that opened them, and prevent more than one transient popover from remaining open.
- [x] Add consistent keyboard navigation and accessible focus treatment for menu items without changing modal-dialog behavior.
- [ ] Keep popovers within the visible window at the Raspberry Pi's minimum supported display size.
- [ ] Smoke-test the interaction model with mouse and keyboard on macOS, Linux amd64, and the physical Raspberry Pi.

## v0.8 — Grammar and style review

Planned in staged, optional increments:

1. [x] Add an on-demand Review mode backed by the LanguageTool HTTP API, with explicit language selection and support for English and Brazilian Portuguese.
2. [x] Keep grammar and style analysis out of the keystroke and autosave paths. Run only after an explicit Check sheet action, discard late results for changed or switched sheets, and never block ordinary writing when the checker is unavailable.
3. [x] Present findings as non-destructive suggestions with category, explanation, and replacements. Apply a replacement only when the checked text still matches the current draft.
4. [x] Mask Markdown syntax, front matter, code, and link destinations before review without shifting UTF-16 source offsets.
5. [ ] Allow individual rules and project vocabulary to be ignored without adding metadata to manuscript files. Individual findings can already be dismissed for the current review.
6. [x] Officially support self-hosted LanguageTool on the writer's computer, NAS, or private server. Test the server without sending manuscript text, reject public unencrypted endpoints, and require an explicit warning acknowledgement for ordinary HTTP across a private network.
7. [x] Provide a reproducible Docker/Portainer deployment built from the checksummed official LanguageTool 6.6 standalone archive for ARM64 and x86 NAS hosts.
8. [ ] Measure review latency and application memory use on the physical 2 GB Raspberry Pi while the LanguageTool service runs on the NAS. Keep the Java service out of the appliance image unless a later benchmark justifies local inclusion.

Write mode remains the low-latency native textarea. Review is an optional layer and never changes Markdown merely because a suggestion was detected. Hosted LanguageTool services are outside the supported v0.8 configuration.

## v0.9 — Scrivener interchange

Planned in two compatible export paths:

1. [ ] Add and document a Scrivener-compatible DOCX preset that exports sheet titles with structural Heading styles, allowing Scrivener's Import and Split command to create one Binder document per sheet.
2. [ ] Verify project, folder, and selected-sheet DOCX imports in current macOS and Windows Scrivener releases, including Unicode text, headings, emphasis, scene breaks, and deterministic sheet order.
3. [ ] Add OPML project export that maps nested Markdown folders and sheets to a Scrivener Binder hierarchy and carries each sheet's text as the corresponding outline item's attached text.
4. [ ] Validate OPML imports with attached text directed to Scrivener's Main Text, and document the required Scrivener import setting.
5. [ ] Keep both exports one-way and non-destructive. Do not generate or modify native `.scriv` project bundles unless Literature & Latte publishes a stable interchange specification.

## Later work

- plugin architecture and marketplace;
- collaboration;
- mobile applications.
