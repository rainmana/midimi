#!/usr/bin/env bash
set -euo pipefail
DEST="assets/soundfonts/GeneralUser-GS.sf2"
URL="https://github.com/mrbumpy409/GeneralUser-GS/raw/main/GeneralUser-GS.sf2"
mkdir -p assets/soundfonts
if [ -f "$DEST" ]; then echo "soundfont already present"; exit 0; fi
echo "Fetching GeneralUser GS (~31MB)…"
curl -L --fail -o "$DEST" "$URL"
echo "Saved $DEST"
