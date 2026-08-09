#!/bin/bash
# Rebuild and reinstall apexshot deb package
# Usage: bash reinstall-dev-deb.sh

set -e

cd "$(dirname "$0")"

echo "Building release binary..."
cargo build --release

echo "Staging capture helper..."
cp target/release/apexshot-capture packaging/deb/apexshot-capture

echo "Building .deb package..."
cargo deb --no-build

echo "Removing old package..."
sudo dpkg -r apexshot 2>/dev/null || true

DEB="$(ls -1t target/debian/apexshot_*.deb 2>/dev/null | head -n1 || true)"
if [[ -z "$DEB" ]]; then
  echo "Error: no apexshot_*.deb found under target/debian/" >&2
  exit 1
fi

echo "Installing new .deb: $DEB"
sudo dpkg -i "$DEB"

echo "Done."
