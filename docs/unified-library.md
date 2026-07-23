# Unified library model

## Decision

Writing Environment will present one calm, application-wide library while preserving ordinary Markdown project folders underneath. The interface owns navigation, Inbox, aggregate Trash, and sync configuration; each project remains an independent folder that can be opened without the application.

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
- Favorited projects remain visible when closed.
- The active project remains visible even when it is not favorited.
- Closing a project unloads its sheets, watcher, search context, and active sync state. It never deletes or moves the project folder.
- Right-clicking a project offers Open/Close and Add/Remove from Favorites.
- Project folders are shown as a nested tree using their real relative paths. The current one-level group summary will not flatten deeper folders.

Until Inbox is implemented, a desktop session with no active project shows a genuinely empty workspace. It must never fall back to the editable browser-prototype library.

## Inbox

Inbox is an application-managed, ordinary Markdown folder in a visible user-writable location. It stores loose sheets that have not yet been assigned to a project and participates in universal search and sync.

Moving an Inbox sheet into a project uses a safe cross-volume operation: copy to a temporary destination, flush and verify the complete Markdown file, atomically place it at the destination, then remove the Inbox copy. A failure leaves at least one complete copy.

## Universal Trash

Trash is one aggregate view, not one undifferentiated deletion folder. Items remain partitioned by origin project or Inbox in application data. Each item records its origin identity and relative path.

- Restore returns an item to its original location when that project is available.
- If the original project is unavailable, the writer may restore to Inbox.
- Empty Trash can remove all items or only items from a selected origin after explicit confirmation.
- History and Trash remain local by default and are not part of normal manuscript sync.

## Universal sync

The user configures a remote root such as `dropbox:Writing Environment` once. The remote layout is deterministic:

```text
Writing Environment/
  Inbox/
  Projects/
    <stable-project-id>-<safe-project-name>/
```

Projects receive a stable identifier so two folders with the same display name cannot collide. The application schedules isolated project sync jobs sequentially and reports one aggregate status: up to date, working, conflicts preserved, or needs attention.

Each project has a **Sync this project** toggle. Enabling universal sync does not silently upload an existing project: its first remote initialization still requires confirmation and an empty destination. A preference may control whether projects added later are included by default.

Existing project-based rclone profiles will be migrated, not discarded. The migration records their remote locations and preserves their current conflict archives until the new universal root is confirmed.

## Delivery stages

1. Correct empty/restored workspace state and add Close Project.
2. Add a persistent project registry and nested folder tree.
3. Add the universal Inbox and safe Move to Project.
4. Aggregate existing origin-partitioned Trash into one view.
5. Add universal sync configuration, per-project inclusion, migration, and aggregate status.
6. Run multi-project conflict, recovery, 10,000-sheet, and physical Raspberry Pi 4 tests.

