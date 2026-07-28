# Architecture

## Delivery sequence

1. Validate the interaction design as a lightweight Svelte frontend.
2. Add a Rust/Tauri backend for filesystem access, recovery, indexing, and export.
3. Benchmark the complete application on a Raspberry Pi 4 with 2 GB. (Completed for the current interface.)
4. Replace the frontend with GTK4 only if the Pi benchmark reveals unacceptable WebKitGTK latency or memory use.
5. Add conservative bidirectional synchronization. (Completed for the current application.)
6. Package the proven application in a minimal Labwc-based Raspberry Pi OS image. (Completed.)
7. Add a rebuildable SQLite FTS index and validate 10,000-sheet libraries on the 2 GB Raspberry Pi. (Current milestone.)
8. Add portable DOCX and PDF export without changing Markdown's role as the authoritative manuscript format.

## Application boundaries

```text
Svelte interface
    |
Tauri command boundary
    |
    +-- Markdown library service
    +-- Atomic persistence and snapshots
    +-- SQLite FTS search index
    +-- Export service
    +-- Synchronization coordinator
```

The interface never treats its in-memory editor state or SQLite as the authoritative manuscript. The persistence service owns file writes and reports whether content is dirty, saved locally, or synchronized remotely.

The editor keeps the keystroke path separate from whole-document metrics. Input updates the native
textarea and schedules autosave immediately, while word/session counts are coalesced after a short
typing pause. Writing Focus uses three persistent overlay text nodes (before, active, and after),
scans only the current paragraph for its boundary, and refreshes at most once per animation frame.
Scrollbar geometry is remeasured only when layout settings or the window size change, not after
every character. Native persistence—including History snapshots, atomic replacement, and index
maintenance—runs on a blocking worker rather than the application event thread. Filesystem events
caused by a local save remain observable, but their library reconciliation is coalesced until local
editing and the save transaction are idle; external changes are still protected by the expected
disk-version check at the next save.

The desktop application maintains a rebuildable SQLite FTS index under its application-data directory, isolated by canonical project path. Project opening reconciles file modification metadata incrementally, saved sheets update only their own index entry, and search reads from FTS. A missing, incompatible, or corrupt index is rebuilt from Markdown; if indexing remains unavailable, the application falls back to bounded direct scans. SQLite is always derived state and never becomes authoritative manuscript storage.

Each open desktop project also has one native recursive filesystem watcher. Markdown and group-folder events are coalesced after a quiet period before the index and visible library are reconciled. Unrelated assets and hidden synchronization markers do not trigger refresh work. An externally changed active sheet is reread only when the editor is clean. If the editor is dirty, autosave compares against the exact last-known disk contents and stops on a mismatch; the writer can then keep the disk version and preserve the local draft as a separate conflict sheet.

The interface keeps a persistent registry of every project the writer has opened, including a stable local identifier, explicit open/closed state, favorite state, last-opened time, and last selected sheet. Open projects remain in the sidebar while the writer switches among them; closed non-favorites remain registered but hidden. The active project's navigation tree is derived from complete Markdown relative paths, so nested folders remain visible without requiring proprietary project metadata. A project that joins universal sync receives one optional hidden UUID marker so another installation can derive the same remote destination. Removing or corrupting that marker never prevents Markdown discovery or editing. Cross-project moves create, flush, and byte-verify the destination before removing the source.

The universal Inbox is a separate ordinary Markdown root at `Documents/Writing Environment/Inbox`.
It is always addressable from the sidebar and becomes the default workspace when no project is
restored. It shares project reading, indexing, autosave, History, and external-change behavior but is
not inserted into the project registry. Moving an Inbox sheet to an open project writes and flushes a
temporary destination file, atomically places it without overwriting an existing path, verifies the
bytes, and only then removes the Inbox source.

Trash remains partitioned outside manuscript folders by canonical origin path, but the interface
aggregates every registered project and Inbox partition into one static recovery view. Origin
identity is attached while listing rather than written into legacy Trash items. A restore uses the
original relative path when its origin exists; an unavailable origin can be restored to Inbox under
a collision-safe name. Permanent deletion validates the complete selected set of partitions before
removing any item.

Markdown files are authoritative; the application database is only a rebuildable search index. Opening a project recursively discovers readable `.md` files without requiring Writing Environment frontmatter or IDs. The shared reader accepts UTF-8, UTF-16, UTF-32, and Windows-1252 text and normalizes legacy encodings to UTF-8 only when the writer saves an edit. Known non-document artifacts such as macOS AppleDouble `._*.md` files are ignored. A bounded retry handles cloud clients that briefly expose an incomplete multibyte write.

## Synchronization boundary

The app coordinates `rclone bisync`; it does not implement Dropbox OAuth or embed cloud credentials. Provider configuration remains in rclone's user configuration. The interface stores one universal remote root and inclusion state for Inbox and registered projects, then runs each included location sequentially. Remote paths are deterministic (`Inbox/` and `Projects/<stable-project-id>/`), while each local-root/remote pair still receives an isolated work directory under the app-data folder. Bisync retains a 25% maximum-deletion guard. If that guard fires, the app persists a paused state for that location and turns off universal automatic sync. Recovery copies only missing files toward the guarded side with `--ignore-existing`, then resumes normal conflict-preserving bisync; it never applies `--force` or an automatic authoritative resync. Before an established sync, an unreadable local `.md` file can be restored from a readable same-path remote copy only after its exact original bytes are saved under the location's isolated recovery data.

First-time setup lists every uninitialized included location in a separate confirmation. Each destination must be empty before its local contents are copied. Existing per-project preferences migrate as preserved legacy destinations with automatic sync disabled; the app does not upload, relocate, or delete files during migration. Recurring runs use an access marker, resilient recovery, a deletion ceiling, and non-overlapping local and remote backup directories. When both sides change the same sheet, both versions are retained with `writing-environment-conflict` in the Markdown filename so they remain visible in the sheet list.

## Durability rules

- Save to a temporary file in the destination directory.
- Flush and replace the destination atomically where the filesystem permits it.
- Never truncate the only known-good copy in place.
- Preserve recovery snapshots outside the synchronized library tree.
- Expose recovery snapshots through History without allowing preview or restore identifiers to escape their sheet-specific directory.
- Snapshot the current sheet before restoring an older revision so restoration remains reversible.
- Move deleted sheets to an application trash directory.
- Keep stable document identifiers across renames and moves.
- Treat synchronization conflicts as additional versions, never as permission to discard one side.
- Refuse an autosave when the on-disk sheet no longer matches the version originally loaded; preserve both versions through an explicit local conflict copy.

## Raspberry Pi 4 considerations

- Build and test for ARM64 on Raspberry Pi OS.
- Use system WebKitGTK rather than bundling a browser engine.
- Avoid background polling when filesystem notifications or timers can be coalesced.
- Debounce indexing and synchronization separately from local saving.
- Keep themes CSS-variable-based so switching themes does not rebuild the component tree.
- Ship fonts locally and use a small, curated font set.
