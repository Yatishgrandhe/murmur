#!/usr/bin/env bash
# Build Murmur, sign it, install it to /Applications and (re)start it.
#
# Signing with your own Apple Development certificate (if you have one) keeps macOS's
# Microphone / Accessibility permissions across rebuilds. Ad-hoc signatures change on every
# build, and macOS then silently stops honoring the permission you granted.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

npx tauri build --bundles app
APP=src-tauri/target/release/bundle/macos/Murmur.app

IDENTITY=$(security find-identity -v -p codesigning | awk -F'"' '/Apple Development|Developer ID Application/ {print $2; exit}')
if [ -n "${IDENTITY}" ]; then
  echo "Signing with: ${IDENTITY}"
  codesign --force --deep --sign "${IDENTITY}" "$APP"
else
  echo "No developer certificate found; keeping the ad-hoc signature (re-grant permissions after each rebuild)."
fi

pkill -f "Murmur.app/Contents/MacOS/murmur" || true
sleep 1
ditto "$APP" /Applications/Murmur.app
open -a /Applications/Murmur.app
echo "Murmur installed and running. Tap ⌥ Space, speak, and pause."
