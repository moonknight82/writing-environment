Makes ordinary typing responsive again on macOS, Linux amd64, and Raspberry Pi. Word and session counts no longer rescan and split the complete manuscript on every character; they update after a short typing pause without changing autosave timing.

Writing Focus now keeps three persistent overlay regions instead of rebuilding the complete overlay after every Svelte update. Paragraph boundaries are found by scanning the active paragraph rather than all preceding text, visual refreshes are limited to one per animation frame, and scrollbar geometry is measured only after a real layout change.

Autosave, external-change protection, universal sync, Inbox, and Trash behavior are unchanged from 0.4.4.

Signed desktop release for macOS Apple Silicon, Linux amd64, and Raspberry Pi ARM64.

This is a personal project under active development. Back up important writing and review the release notes before updating.
