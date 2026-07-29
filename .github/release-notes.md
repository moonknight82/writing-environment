Introduces the signed Writing Environment APT repository for Raspberry Pi OS
Trixie. The application, minimal Labwc/Fuzzel appliance shell, keyboard
defaults, settings integration, and Plymouth boot theme now install as normal
Debian packages while the official Raspberry Pi OS repositories remain active.

The Super+Space Updates action now upgrades both Raspberry Pi OS and Writing
Environment packages through one confirmed `apt full-upgrade`. Fresh appliance
images include the repository key and source from first boot; existing systems
can install the repository bootstrap package once and use APT thereafter.

The dedicated Raspberry Pi image now opens Writing Environment in native
fullscreen, removes the persistent panel, keeps the Fuzzel status drawer and
Control Centre available, prevents blanking during writing sessions, defaults
to the US-International cedilla behavior, and displays the quiet Writing
Environment splash during boot.

Rust builds performed directly on a 2 GB Raspberry Pi are limited to one Cargo
job by default to avoid out-of-memory kills.

Signed desktop release for macOS Apple Silicon, Linux amd64, and Raspberry Pi
ARM64.

This is a personal project under active development. Back up important writing
and review the release notes before updating.
