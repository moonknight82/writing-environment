Fixes the Raspberry Pi appliance's Display and Control Centre behavior. Display
now opens a dedicated screen-layout window instead of calling a standalone
command that Raspberry Pi OS does not ship.

The writer remains visually fullscreen, but now uses a borderless maximized
Linux window so Control Centre dialogs and other system utilities can appear
above it. This prevents settings actions from seeming to freeze when a dialog
opens. System-drawer launches are also recorded in
`~/.cache/writing-environment/launcher.log` for diagnostics.

The signed APT repository continues to update the application and appliance
shell together with Raspberry Pi OS through the Super+Space Updates action.
Fresh flashable images include this fix.

Signed desktop release for macOS Apple Silicon, Linux amd64, and Raspberry Pi
ARM64.

This is a personal project under active development. Back up important writing
and review the release notes before updating.
