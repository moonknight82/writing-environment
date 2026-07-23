# Product brief

## Purpose

Create a calm, reliable writing instrument centered on portable Markdown files. It should feel purpose-built when running on a Raspberry Pi, while leaving every manuscript readable without the application.

## Target hardware

The baseline target is a Raspberry Pi 4 with 2 GB of RAM running 64-bit Raspberry Pi OS and Labwc/Wayland. A 4 GB model provides extra headroom but is not required.

The application must be tested on that device before a feature is considered complete. Faster development computers and newer Raspberry Pi models are secondary targets, not the performance baseline.

## Product principles

1. Local writing never depends on a network connection.
2. No normal failure may silently destroy or overwrite manuscript text.
3. Markdown files are authoritative; databases and indexes are rebuildable.
4. System controls are available but remain out of sight during writing.
5. Themes may change the atmosphere, not the meaning or location of controls.
6. Typing responsiveness takes priority over animation and decorative effects.
7. The application remains useful even if cloud synchronization is never configured.

## First vertical slice

- Library groups and sheets
- Recent and pinned project folders in a dedicated navigation area
- Markdown editor
- Automatic local saving
- Atomic file replacement
- Local revision snapshots
- Trash and recovery
- Full-text search
- Word count
- Focus and typewriter modes
- Caret-aware paragraph and sentence focus
- Light, warm, dark, and Old Terminal visual themes
- Persistent writer line-height control
- Persistent session word goal with live progress
- Visible local-save and synchronization state

## Performance budgets

Initial budgets to validate on the Raspberry Pi 4:

- no perceptible input lag during ordinary Markdown editing;
- application idle memory target below 350 MB;
- editor remains responsive with a 100,000-word sheet;
- library navigation remains responsive with 10,000 sheets;
- animations limited to opacity and transforms, normally 180 ms or less;
- no backdrop blur, animated backgrounds, or continuously running decorative effects.

These are engineering targets and may be tightened after measurement on the physical device.

## Dedicated OS environment

- Raspberry Pi OS Lite (Debian Trixie, ARM64) base
- Minimal Raspberry Pi Labwc desktop, with no general-purpose application bundles
- PiXonyx Dark Mode by default and PiXtrix Light Mode available through Appearance Settings
- Automatic graphical login and borderless presentation-mode writer launch
- Essential escape hatches: Files, Terminal, network, Bluetooth, audio, removable media, updater, power, and system preferences

## Deferred work

- DOCX, PDF, and EPUB export
- Plugin marketplace
- Collaboration
- Mobile applications
