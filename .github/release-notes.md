Version 0.7.1 turns formatted Markdown Preview into a live companion to the
editor instead of replacing the writing surface. Desktop windows divide the
available writing area into equal editor and preview panes with independent
scrolling. Narrow windows retain both surfaces in a stacked split, and Writing
Focus remains available while Preview is active.

Linux now detects NVIDIA graphics hardware before WebKitGTK starts and selects
its shared-memory renderer when necessary. This avoids DRM/GBM permission
failures seen with some NVIDIA driver and compositor combinations while leaving
Intel and AMD rendering unchanged. An explicit environment setting can still
override the automatic choice.

Linux x86-64 releases now publish both the native Debian package and a portable
AppImage intended for Fedora and other distributions. The release workflow
verifies both artifact formats before publication.

This is a personal project under active development. Back up important writing
and review the release notes before updating.
