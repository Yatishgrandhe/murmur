#!/usr/bin/env bash
# Downloads the speech model that ships inside Murmur's installer (Whisper tiny.en, q5_1,
# 32 MB) into src-tauri/resources/ and checks its SHA-256. Safe to run again: a verified copy is kept.
set -euo pipefail

NAME="ggml-tiny.en-q5_1.bin"
SHA256="c77c5766f1cef09b6b7d47f21b546cbddd4157886b3b5d6d4f709e91e66c7c2b"
URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/${NAME}"

DIR="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/resources"
OUT="${DIR}/${NAME}"
mkdir -p "$DIR"

sum() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

if [ -f "$OUT" ] && [ "$(sum "$OUT")" = "$SHA256" ]; then
  echo "Speech model already present: $OUT"
  exit 0
fi

echo "Downloading $NAME ..."
curl -fL --retry 3 --retry-delay 2 -o "$OUT.part" "$URL"
GOT="$(sum "$OUT.part")"
if [ "$GOT" != "$SHA256" ]; then
  rm -f "$OUT.part"
  echo "Checksum mismatch for $NAME: got $GOT, expected $SHA256" >&2
  exit 1
fi
mv "$OUT.part" "$OUT"
echo "Speech model ready: $OUT"
