#!/usr/bin/env bash
# Optional diagnostic/fallback bundle for this ARMv7 ADM NAS; no system packages installed.
set -euo pipefail
cd "$(dirname "$0")/.."
api_binary="$PWD/target/armv7-unknown-linux-musleabihf/release/dorm-api"
[[ -x "$api_binary" ]] || { echo 'Run scripts/build-armv7.sh first' >&2; exit 1; }
keyring=/usr/share/keyrings/ubuntu-archive-keyring.gpg
[[ -r "$keyring" ]] || { echo 'This helper requires the Ubuntu archive keyring (tested host: Zorin/Ubuntu)' >&2; exit 1; }
work=$(mktemp -d /tmp/dorm-menu-bundle.XXXXXX)
mkdir -p "$work/apt/lists/partial" "$work/apt/archives/partial" "$work/root" "$work/bundle/runtime/usr/bin" "$work/bundle/bin" "$work/bundle/licenses"
touch "$work/apt/status"
for suite in jammy jammy-updates jammy-security; do
    printf 'deb [arch=armhf signed-by=%s] https://ports.ubuntu.com/ubuntu-ports %s main universe\n' "$keyring" "$suite"
done > "$work/apt/sources.list"
apt_options=(-o APT::Architecture=armhf -o "Dir::Etc::sourcelist=$work/apt/sources.list" -o Dir::Etc::sourceparts=- -o "Dir::State::lists=$work/apt/lists" -o "Dir::State::status=$work/apt/status" -o "Dir::Cache=$work/apt/cache" -o "Dir::Cache::archives=$work/apt/archives" -o Debug::NoLocking=true -o Acquire::Languages=none)
apt-get "${apt_options[@]}" update
apt-get "${apt_options[@]}" --download-only --no-install-recommends -y install poppler-utils:armhf
shopt -s nullglob
for package in "$work"/apt/archives/lib*.deb "$work"/apt/archives/zlib1g*.deb "$work"/apt/archives/poppler-utils*.deb; do
    dpkg-deb -x "$package" "$work/root"
    dpkg-deb -f "$package" Package Version Architecture >> "$work/bundle/poppler-packages.txt"
done
cp -a "$work/root/lib" "$work/bundle/runtime/lib"
cp -a "$work/root/usr/lib" "$work/bundle/runtime/usr/lib"
cp -a "$work/root/usr/bin/pdftotext" "$work/bundle/runtime/usr/bin/"
cp -a "$work/root/usr/share/doc/." "$work/bundle/licenses/"
cp "$api_binary" "$work/bundle/dorm-api"
cat > "$work/bundle/bin/pdftotext" <<'WRAPPER'
#!/bin/sh
bundle=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
exec "$bundle/runtime/lib/arm-linux-gnueabihf/ld-linux-armhf.so.3" --library-path "$bundle/runtime/lib/arm-linux-gnueabihf:$bundle/runtime/usr/lib/arm-linux-gnueabihf" "$bundle/runtime/usr/bin/pdftotext" "$@"
WRAPPER
chmod +x "$work/bundle/bin/pdftotext"
cat > "$work/bundle/run.sh" <<'RUNNER'
#!/bin/sh
set -eu
cd "$(dirname "$0")"
export PATH="$PWD/bin:$PATH"
export DATA_DIR="${DATA_DIR:-$PWD/data}" TOKEN_FILE="${TOKEN_FILE:-$PWD/config/tokens.json}" BIND="${BIND:-127.0.0.1:3000}"
exec ./dorm-api "$@"
RUNNER
chmod +x "$work/bundle/run.sh"
tar -czf target/dorm-menu-nas-bundle.tar.gz -C "$work/bundle" .
printf 'Bundle: %s/target/dorm-menu-nas-bundle.tar.gz\nBuild files and original packages retained in: %s\n' "$PWD" "$work"
