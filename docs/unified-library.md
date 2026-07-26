# Unified library model

## Decision

Writing Environment will present one calm, application-wide library while preserving ordinary Markdown project folders underneath. The interface owns navigation, Inbox, aggregate Trash, and sync configuration; each project remains an independent folder that can be opened without the application.

Files outrank application state. Any readable `.md` file in a project or nested folder participates without proprietary metadata, no matter which editor created it. Derived indexes can be discarded and rebuilt; unsupported binary artifacts are never rewritten, and common legacy text encodings remain usable until an explicit edit normalizes them to UTF-8.

Sync is universal in the interface but isolated per project in storage and execution. A writer configures one rclone remote root once, then includes or excludes individual projects. Each project keeps its own sync profile, archive, conflict handling, and deletion boundary.

## Sidebar model

The finished library sidebar is:

```text
Inbox

Projects
  ▾ My Novel
      Draft
      Research
        Locations
      Fragments
  ▸ Essays

Trash
```

- **Inbox** and **Trash** are universal, static destinations.
- Open projects remain visible until the writer explicitly closes them.
- Favorited projects remain visible as shortcuts when closed.
- The active project remains visible even when it is not favorited, and each open project remembers its last selected sheet.
- Closing a project unloads its sheets, watcher, search context, and active sync state. It never deletes or moves the project folder.
- Right-clicking a project offers Open/Close and Add/Remove from Favorites.
- Moving a sheet can target a nested folder in any open project. The verified destination copy is made durable before the source is removed.
- Project folders are shown as a nested tree using their real relative paths. The current one-level group summary will not flatten deeper folders.

When no project is restored, the desktop opens the universal Inbox. It never falls back to the editable browser-prototype library.

## Inbox

Inbox is an application-managed, ordinary Markdown folder in a visible user-writable location. It stores loose sheets that have not yet been assigned to a project, participates in library search, and can be included in the universal sync root.

The implemented desktop location is `Documents/Writing Environment/Inbox`. Inbox opens as the
default non-project workspace, uses the same Markdown reader, autosave, History, search index, and
external-change guard as a project, and never appears in the project registry. It has its own
isolated sync profile under the universal remote root rather than inheriting a project's settings.

Moving an Inbox sheet into a project uses a safe cross-volume operation: copy to a temporary destination, flush and verify the complete Markdown file, atomically place it at the destination, then remove the Inbox copy. A failure leaves at least one complete copy.

## Universal Trash

Trash is one aggregate view, not one undifferentiated deletion folder. Items remain partitioned by origin project or Inbox in application data. Each item records its origin identity and relative path.

- Restore returns an item to its original location when that project is available.
- If the original project is unavailable, the writer may restore to Inbox.
- Empty Trash can remove all items or only items from a selected origin after explicit confirmation.
- History and Trash remain local by default and are not part of normal manuscript sync.

The implemented aggregate view reads the existing path-keyed recovery partitions in place; it does
not migrate, rename, or insert metadata into old trashed manuscripts. Registered closed projects
remain valid origins. When an origin folder is unavailable, restoration is explicitly redirected to
Inbox using a new collision-safe filename. Before permanent emptying, every selected recovery
partition is validated so one malformed origin cannot cause an earlier origin to be partially
emptied.

## Universal sync

The user configures a remote root such as `dropbox:Writing Environment` once. The remote layout is deterministic:

```text
Writing Environment/
  Inbox/
  Projects/
    <stable-project-id>/
```

Projects receive a stable UUID so two folders with the same display name cannot collide and a local rename cannot change the remote destination. The optional `.writing-environment-project-id` marker is created only when a project joins universal sync. It travels with the folder so another installation derives the same remote path, but it is not required to discover, open, or edit any Markdown file. The application schedules isolated project sync jobs sequentially and reports one aggregate status: up to date, working, conflicts preserved, or needs attention.

Inbox and each registered project have an inclusion toggle. Enabling a location does not silently upload it: every new remote destination is listed in a separate confirmation, must be empty, and is initialized only after explicit approval. Automatic sync can be enabled only after every included location is initialized, and jobs run sequentially so one failure can be isolated and reported by origin.

Existing project-based rclone profiles are copied into the universal configuration as preserved legacy locations. Their original remote paths, initialization state, isolated rclone work data, and recovery state remain in place; automatic sync is left off. Moving a legacy project into the new layout is a separate explicit action and first-sync confirmation, and the previous remote folder is not deleted or repurposed.

## Delivery stages

1. [x] Correct empty/restored workspace state and add Close Project.
2. [x] Add a persistent project registry and nested folder tree.
3. [x] Add the universal Inbox and safe Move to Project.
4. [x] Aggregate existing origin-partitioned Trash into one view.
5. [x] Add universal sync configuration, per-project inclusion, migration, and aggregate status.
6. [ ] Run multi-project conflict, recovery, 10,000-sheet, and physical Raspberry Pi 4 tests.
