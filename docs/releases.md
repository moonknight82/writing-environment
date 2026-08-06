# Release process

Writing Environment publishes signed application updates from the public GitHub repository. Source pushes run verification only; a matching semantic-version tag builds a release. The release remains a private draft until every signed desktop artifact, `latest.json`, Raspberry Pi appliance package, and APT deployment has completed, then becomes public in one final step. This prevents an incomplete release from becoming GitHub's `latest` release before its updater manifest exists.

## One-time repository setup

The updater public key is committed in `src-tauri/tauri.conf.json`. Its private counterpart must never be committed. Store the complete private-key text as the GitHub Actions repository secret `TAURI_SIGNING_PRIVATE_KEY`, and keep a separate offline backup. Losing this key prevents already-installed copies from trusting future updates.

The Raspberry Pi APT archive uses a separate OpenPGP key. Its public half is committed as `deploy/apt-repository/writing-environment-archive-keyring.asc`; its private half is stored in the GitHub Actions secret `APT_SIGNING_PRIVATE_KEY` and in the ignored local `.apt-signing` directory. The release-only fingerprint is `6B9286D479AB874C435A3EADEF2E16B217210ECB`. Keep a secure offline backup: installed Pis require a package signed by the current key to trust a future replacement.

The current macOS package uses ad-hoc Apple code signing for personal testing. Tauri's updater signature verifies update authenticity, but it does not replace Apple Developer ID signing or notarization for general distribution.

## Publish

1. Run `scripts/release/verify.sh` to confirm `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, and the release notes agree.
2. Update `.github/release-notes.md` for the version being published.
3. Run `pnpm install --frozen-lockfile`, `pnpm build`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
4. Commit the release changes.
5. Create and push a matching tag, such as `v0.3.1`.
6. Watch the **Publish signed desktop release** workflow. The draft release becomes public only after every required job succeeds.
7. Confirm the GitHub Release contains `latest.json`, the signed Linux amd64 `.deb` and AppImage artifacts, the signed macOS artifact, the Raspberry Pi manual archive, and the four appliance/repository packages.
8. Confirm the Pages deployment publishes a valid signed APT repository at `https://moonknight82.github.io/writing-environment/apt`.
9. Test both **Writer (Aa) → Application updates → Check for Updates…** and **Super+Space → Updates** before relying on automatic checks.

GitHub's public ARM64 runner creates the Raspberry Pi Debian package natively. After the desktop release jobs finish, a second ARM64 job builds the appliance packages, signs standard Debian repository metadata, uploads the bootstrap packages to the GitHub Release, and deploys the static repository through GitHub Pages.

## Safety model

- Update checks use HTTPS and a static GitHub Release manifest.
- Tauri requires a valid embedded-key signature before installing an artifact.
- The application saves the active sheet before download or installation.
- Installation and restart always require an explicit click.
- Linux Debian updates request authorization through the system privilege dialog.
- The APT signing key is distinct from the Tauri updater key and is scoped to the separate Writing Environment source with `Signed-By`.
- APT pinning gives the custom repository normal priority only for packages named `writing-environment*`; Raspberry Pi OS remains authoritative for the rest of the system.
- The system drawer always asks before running `apt full-upgrade`; no appliance package reboots the Pi automatically.
