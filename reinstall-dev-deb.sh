#!/bin/bash
# Rebuild and reinstall apexshot deb package
# Usage: bash reinstall-dev-deb.sh

set -e

cd "$(dirname "$0")"

echo "Building release binary..."
cargo build --release

echo "Staging capture helper..."
cp target/release/apexshot-capture packaging/deb/apexshot-capture
cmp target/release/apexshot-capture packaging/deb/apexshot-capture

echo "Building .deb package..."
cargo deb --no-build

apexshot_is_running() {
  pgrep -x apexshot >/dev/null 2>&1 \
    || pgrep -x apexshot-captur >/dev/null 2>&1 \
    || pgrep -x apexshot-capture >/dev/null 2>&1
}

wait_for_apexshot_exit() {
  local attempts="$1"
  local attempt
  for ((attempt = 0; attempt < attempts; attempt++)); do
    if ! apexshot_is_running; then
      return 0
    fi
    sleep 0.25
  done
  return 1
}

echo "Stopping running ApexShot processes..."
pkill -x apexshot 2>/dev/null || true
# Linux truncates the helper's process name to 15 characters.
pkill -x apexshot-captur 2>/dev/null || true
pkill -x apexshot-capture 2>/dev/null || true
if ! wait_for_apexshot_exit 20; then
  echo "ApexShot did not exit after 5 seconds; forcing shutdown..."
  pkill -9 -x apexshot 2>/dev/null || true
  pkill -9 -x apexshot-captur 2>/dev/null || true
  pkill -9 -x apexshot-capture 2>/dev/null || true
  if ! wait_for_apexshot_exit 8; then
    echo "Error: ApexShot processes did not stop" >&2
    ps -eo pid,comm,args | grep -E '[a]pexshot(-captur(e)?)?' >&2 || true
    exit 1
  fi
fi

echo "Removing old package..."
sudo dpkg -r apexshot 2>/dev/null || true

DEB="$(ls -1t target/debian/apexshot_*.deb 2>/dev/null | head -n1 || true)"
if [[ -z "$DEB" ]]; then
  echo "Error: no apexshot_*.deb found under target/debian/" >&2
  exit 1
fi

echo "Installing new .deb: $DEB"
sudo dpkg -i "$DEB"

echo "Verifying installed binaries..."
cmp target/release/apexshot /usr/bin/apexshot
cmp target/release/apexshot-capture /usr/bin/apexshot-capture

echo "Done. Start ApexShot to launch the freshly installed build."
