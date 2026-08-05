Version 0.7.0 makes the writer's transient menus behave consistently with
desktop applications while improving complete keyboard operation.

Toolbar, sorting, project, and sheet-action popovers now share one controller.
Only one can remain open, clicking elsewhere dismisses it, and interactions
inside forms continue normally. Escape closes the active popover and restores
focus to its trigger.

Actual menus now focus their current or first available choice when opened.
Arrow keys move and wrap through choices, Home and End jump to the boundaries,
Enter or Space activates the focused item, and Tab dismisses the menu. Project
context menus can also be opened with Shift+F10 or the keyboard Menu key. A
restrained visible focus treatment makes the current keyboard target clear
without changing modal-dialog behavior.

The Raspberry Pi Docker packager now clears stale bundle output before copying
the ARM64 Debian package, preventing a cached package from another architecture
from appearing in the Pi artifact directory.

This is a personal project under active development. Back up important writing
and review the release notes before updating.
