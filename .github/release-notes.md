Removes Raspberry Pi's Desktop/Taskbar appearance plugin from the panel-free
Writing Environment appliance. Its controls target PCManFM's desktop and
wf-panel-pi, which the appliance deliberately does not run.

Toggling options such as **Show Home Folder** previously launched
`pcmanfm --reconfigure` synchronously. With no existing desktop process, that
command remained open and blocked Control Centre. The incompatible Desktop,
Taskbar, Appearance, and Defaults pages are now absent; Screens, mouse and
keyboard, Raspberry Pi system configuration, and Bluetooth remain available.

The signed APT repository continues to update the application and appliance
shell together with Raspberry Pi OS through the Super+Space Updates action.
Fresh flashable images include this fix.

Signed desktop release for macOS Apple Silicon, Linux amd64, and Raspberry Pi
ARM64.

This is a personal project under active development. Back up important writing
and review the release notes before updating.
