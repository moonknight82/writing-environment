#!/usr/bin/env bash
set -euo pipefail

packages=""
private_key=""
output=""

usage() {
  cat <<'EOF'
Usage: scripts/apt/build-repository.sh OPTIONS

Build and sign the static Writing Environment APT repository.

Required options:
  --packages DIRECTORY   Directory containing the release Debian packages
  --private-key FILE     ASCII-armored repository signing private key
  --output DIRECTORY     New output directory; its site/ tree is publishable
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --packages) packages="${2:-}"; shift 2 ;;
    --private-key) private_key="${2:-}"; shift 2 ;;
    --output) output="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
  esac
done

[[ -d "$packages" ]] || { printf 'Package directory not found: %s\n' "$packages" >&2; exit 1; }
[[ -f "$private_key" ]] || { printf 'Repository private key not found: %s\n' "$private_key" >&2; exit 1; }
[[ -n "$output" ]] || { printf -- '--output is required.\n' >&2; exit 2; }
[[ ! -e "$output" ]] || { printf 'Refusing to overwrite existing output: %s\n' "$output" >&2; exit 1; }

for command_name in apt-ftparchive dpkg-scanpackages gpg gpgv gzip sha256sum; do
  command -v "$command_name" >/dev/null 2>&1 || {
    printf 'Required repository tool is missing: %s\n' "$command_name" >&2
    exit 1
  }
done

readonly gpg_home="$(mktemp -d /tmp/writing-environment-apt-gpg.XXXXXX)"
trap 'rm -rf -- "$gpg_home"' EXIT
chmod 0700 "$gpg_home"
gpg --batch --homedir "$gpg_home" --import "$private_key" >/dev/null 2>&1
fingerprint="$(gpg --batch --homedir "$gpg_home" --with-colons --list-secret-keys | awk -F: '$1 == "fpr" { print $10; exit }')"
[[ -n "$fingerprint" ]] || { printf 'The repository signing key contains no secret key.\n' >&2; exit 1; }

readonly site="$output/site"
readonly apt_root="$site/apt"
readonly release_dir="$apt_root/dists/trixie"
readonly binary_dir="$release_dir/main/binary-arm64"
mkdir -p "$apt_root/pool/main" "$binary_dir"

package_count=0
for package_file in "$packages"/*.deb; do
  [[ -f "$package_file" ]] || continue
  install -m 0644 "$package_file" "$apt_root/pool/main/$(basename "$package_file")"
  package_count=$((package_count + 1))
done
[[ "$package_count" -ge 5 ]] || { printf 'Expected at least five Debian packages, found %s.\n' "$package_count" >&2; exit 1; }

(
  cd "$apt_root"
  dpkg-scanpackages --multiversion pool/main /dev/null >dists/trixie/main/binary-arm64/Packages
  gzip -9n -c dists/trixie/main/binary-arm64/Packages >dists/trixie/main/binary-arm64/Packages.gz
  apt-ftparchive \
    -o APT::FTPArchive::Release::Origin='Writing Environment' \
    -o APT::FTPArchive::Release::Label='Writing Environment' \
    -o APT::FTPArchive::Release::Suite='trixie' \
    -o APT::FTPArchive::Release::Codename='trixie' \
    -o APT::FTPArchive::Release::Architectures='arm64' \
    -o APT::FTPArchive::Release::Components='main' \
    -o APT::FTPArchive::Release::Description='Signed Writing Environment appliance packages for Raspberry Pi OS Trixie' \
    release dists/trixie >dists/trixie/Release
)

gpg --batch --yes --homedir "$gpg_home" --local-user "$fingerprint" \
  --digest-algo SHA256 --clearsign \
  --output "$release_dir/InRelease" "$release_dir/Release"
gpg --batch --yes --homedir "$gpg_home" --local-user "$fingerprint" \
  --digest-algo SHA256 --armor --detach-sign \
  --output "$release_dir/Release.gpg" "$release_dir/Release"
gpg --batch --yes --homedir "$gpg_home" --armor --export "$fingerprint" \
  >"$apt_root/writing-environment-archive-keyring.asc"
gpg --batch --yes --dearmor \
  --output "$output/writing-environment-archive-keyring.gpg" \
  "$apt_root/writing-environment-archive-keyring.asc"
gpgv --keyring "$output/writing-environment-archive-keyring.gpg" \
  "$release_dir/Release.gpg" "$release_dir/Release"

install -m 0644 /dev/null "$site/.nojekyll"
printf '%s\n' \
  '<!doctype html><meta charset="utf-8"><title>Writing Environment APT repository</title>' \
  '<h1>Writing Environment APT repository</h1>' \
  '<p>Signed Raspberry Pi OS Trixie packages for the Writing Environment appliance.</p>' \
  >"$site/index.html"
(
  cd "$output"
  find site -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum >SHA256SUMS
)

printf 'Signed APT repository written to:\n%s\nSigning fingerprint: %s\n' "$output" "$fingerprint"
