Fixes project switching so every open project remains in the sidebar until it is explicitly closed. Each project remembers its last selected sheet, and the Move dialog can transfer a sheet to a nested folder in any other open project.

Cross-project moves write and verify a durable destination copy before removing the source. A Markdown file containing invalid UTF-8 bytes no longer prevents its entire project from opening: the application leaves that file untouched, skips it, and names it in a clear warning so it can be converted or restored safely.

Signed desktop release for macOS Apple Silicon, Linux amd64, and Raspberry Pi ARM64.

This is a personal project under active development. Back up important writing and review the release notes before updating.
