# Release process

Writing Environment publishes signed application updates from the public GitHub repository. Source pushes run verification only; a matching semantic-version tag publishes a release.

## One-time repository setup

The updater public key is committed in `src-tauri/tauri.conf.json`. Its private counterpart must never be committed. Store the complete private-key text as the GitHub Actions repository secret `TAURI_SIGNING_PRIVATE_KEY`, and keep a separate offline backup. Losing this key prevents already-installed copies from trusting future updates.

The current macOS package uses ad-hoc Apple code signing for personal testing. Tauri's updater signature verifies update authenticity, but it does not replace Apple Developer ID signing or notarization for general distribution.

## Publish

1. Make sure `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` contain the same version.
2. Update the release notes in `.github/workflows/release.yml` for the version being published.
3. Run `pnpm install --frozen-lockfile`, `pnpm build`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
4. Commit the release changes.
5. Create and push a matching tag, such as `v0.3.1`.
6. Watch the **Publish signed desktop release** workflow.
7. Confirm the GitHub Release contains `latest.json`, signed macOS and Debian updater artifacts, and the Raspberry Pi manual bootstrap archive with its checksum and signature.
8. Test **Writer (Aa) → Application updates → Check for Updates…** on each installed platform before relying on automatic checks.

GitHub's public ARM64 runner creates the Raspberry Pi Debian package natively. The manual Pi archive contains the executable extracted from that Debian bundle, allowing the bootstrap installation to identify future updates as Debian packages.

## Safety model

- Update checks use HTTPS and a static GitHub Release manifest.
- Tauri requires a valid embedded-key signature before installing an artifact.
- The application saves the active sheet before download or installation.
- Installation and restart always require an explicit click.
- Linux Debian updates request authorization through the system privilege dialog.
- Raspberry Pi OS and appliance-shell updates are never included in an app-only update.
