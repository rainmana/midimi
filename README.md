# midimi

midimi is a MIDI sequencer and player built with Tauri v2 + Svelte 5. It plays back MIDI files using a bundled soundfont (GeneralUser GS), visualizes tracks, and lets you edit note data — all in a native desktop app.

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+
- A soundfont (fetched automatically — see below)

## Setup

```bash
# 1. Fetch the bundled soundfont (~31 MB, one-time)
./scripts/fetch-soundfont.sh

# 2. Install JS dependencies
npm install

# 3. Launch in dev mode
npm run tauri dev
```

## Building

```bash
npm run tauri build
```

## Third-party assets

**GeneralUser GS v2.0** — soundfont by S. Christian Collins  
License: `assets/soundfonts/GeneralUser-GS-LICENSE.txt`  
Source: https://github.com/mrbumpy409/GeneralUser-GS
