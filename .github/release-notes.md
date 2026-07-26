Completes the unified-library interface. Inbox is now a permanent editable Markdown workspace, and Trash presents recoverable sheets from Inbox and every registered project in one origin-labelled view. Sheets can be restored to their original project or safely recovered to Inbox when that project is unavailable.

Sync now uses one universal rclone remote root with separate inclusion controls for Inbox and each project. Locations run sequentially with isolated state, archives, conflict handling, and deletion protection. Every new remote destination requires explicit confirmation and must be empty. A small optional project UUID keeps its destination stable across Mac, Linux, and Raspberry Pi without becoming a requirement for opening Markdown.

Existing project sync settings are preserved at their original remote paths with automatic sync disabled. Installing this release does not silently upload, relocate, or delete project files. Moving a legacy project into the universal layout remains an explicit, separately confirmed action.

Signed desktop release for macOS Apple Silicon, Linux amd64, and Raspberry Pi ARM64.

This is a personal project under active development. Back up important writing and review the release notes before updating.
