Moves native autosave persistence off the interactive application thread on macOS, Linux amd64, and Raspberry Pi. History snapshots, atomic file replacement, and SQLite index maintenance now run on a blocking worker after 1.2 seconds of typing inactivity.

Filesystem notifications caused by the app's own save are coalesced until editing and saving are idle, preventing a save from immediately triggering a full library reconciliation. The status bar now distinguishes unsaved typing from an active disk save.

History behavior, atomic writes, external-change and disk-conflict protection, universal sync, Inbox, and Trash remain unchanged.

Signed desktop release for macOS Apple Silicon, Linux amd64, and Raspberry Pi ARM64.

This is a personal project under active development. Back up important writing and review the release notes before updating.
