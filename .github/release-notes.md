Version 0.6.0 makes larger Markdown libraries faster to navigate without adding
proprietary metadata to manuscript folders.

Search in the sheet column now spans Inbox and every open project, labels each
result with its origin, and activates the correct library when opened. The new
Command/Control+P Quick Switcher provides keyboard navigation through search
results, recent sheets, and locally stored favorites.

The sheet column also has an explicit selection mode. Writers can select sheets
across folders in the active Inbox or project, then move them, send them to
Trash, or export only that selection as DOCX, PDF, or EPUB. Batch file changes
reuse the existing atomic single-sheet operations. If an item fails, completed
work remains valid, unprocessed sheets stay selected, and the app reports exact
partial progress.

Recent and favorite references are local interface preferences only. They do
not create or modify files inside Markdown projects.

The release workflow now has a single source for release notes, avoiding stale
text from an older release while signed platform artifacts are assembled. The
release remains a private draft until macOS Apple Silicon, Linux amd64,
Raspberry Pi ARM64, the updater manifest, and the signed Raspberry Pi APT
repository are complete.

This is a personal project under active development. Back up important writing
and review the release notes before updating.
