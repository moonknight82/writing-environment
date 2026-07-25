Turns rclone's excessive-deletion safety abort into a clear, persistent recovery state. If more than 25% of tracked files appear missing locally or remotely, the app confirms that no changes were made, switches off automatic sync for that project, and identifies which side needs recovery.

The recovery action restores only files missing from the guarded side using rclone's `--ignore-existing` behavior, then resumes normal conflict-preserving bisync. Existing files are never overwritten, no deletion is forced, and automatic sync remains off until the writer deliberately re-enables it.

Signed desktop release for macOS Apple Silicon, Linux amd64, and Raspberry Pi ARM64.

This is a personal project under active development. Back up important writing and review the release notes before updating.
