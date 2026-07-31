Adds a formatted Markdown Preview beside the fast source-writing mode. Preview
renders common Markdown structure, including headings, emphasis, quotations,
lists, code, and tables, and sanitizes rendered HTML before showing it.

An empty Inbox now has an explicit, non-editable empty state with a **Create a
sheet** action, so writing is never accidentally detached from a Markdown
file. Switching back to Write preserves the original source and continues to
use the operating system's spelling support.

GitHub Releases now remain drafts while signed desktop artifacts, the updater
manifest, and Raspberry Pi appliance packages are produced. They become public
only once all required pieces are present, preventing an update check from
seeing a release without `latest.json`.

The signed APT repository continues to update the application and appliance
shell together with Raspberry Pi OS through the Super+Space Updates action.
Fresh flashable images include this fix.

Signed desktop release for macOS Apple Silicon, Linux amd64, and Raspberry Pi
ARM64.

This is a personal project under active development. Back up important writing
and review the release notes before updating.
