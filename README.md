<p align="center">
  <img src="assets/branding/midimi-icon.png" alt="midimi" width="168" height="168" />
</p>

<h1 align="center">midimi</h1>

<p align="center"><em>A magical music box for MIDI — play, explore, and <strong>watch</strong> your music.</em></p>

<p align="center">
  <a href="https://github.com/rainmana/midimi/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/rainmana/midimi/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-blue.svg"></a>
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-8a6bff">
  <img alt="Built with" src="https://img.shields.io/badge/built%20with-Tauri%202%20%7C%20Svelte%205-19f0c8">
</p>

midimi is a native desktop MIDI player and visualizer built with Tauri v2 (Rust backend) and SvelteKit + Svelte 5 (frontend). Drop in a `.mid` file and watch it come alive: a full-screen "Cosmic Aurora" visualization reacts to every note in real time, while a bundled General MIDI soundfont renders the audio entirely in Rust — no browser audio engine, no drift. Transport controls let you play, pause, seek, adjust tempo and volume. Recent files and settings persist across launches. Licensed Apache-2.0.

---

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) 20+
- macOS, Linux (with WebKit2GTK + libasound), or Windows

---

## Setup

```bash
# 1. Fetch the bundled soundfont (~31 MB, one-time download)
./scripts/fetch-soundfont.sh

# 2. Install JS dependencies
npm install

# 3. Launch in dev mode (hot-reload frontend, Rust recompiles on change)
npm run tauri dev
```

---

## Build

```bash
npm run tauri build
```

Produces a platform-native installer in `src-tauri/target/release/bundle/`.

---

## Project layout

```
src/                  SvelteKit frontend (SPA, SSR off)
  routes/+page.svelte   Root page — state, playhead listener, file open
  lib/
    ipc.ts              Typed wrappers for every Tauri invoke + event
    types.ts            Shared TS interfaces (Note, MidiData, Playhead, …)
    theme.ts            Three built-in themes (Cosmic, Nebula Rose, Abyss)
    VizCanvas.svelte    Canvas host + note scheduler
    visualizations/
      types.ts          Visualization plugin interface
      cosmic-aurora.ts  Bundled aurora visualization (Canvas 2D)
    Transport.svelte    Play/pause/seek/tempo/volume controls
    TrackList.svelte    Track name list
    SoundfontPicker.svelte  SF2 loader + switcher
    ThemePicker.svelte  Theme selector
src-tauri/src/        Rust backend
  lib.rs              App setup, resource resolution, 60 Hz playhead loop
  commands.rs         Tauri command handlers
  midi.rs             MIDI parser → note timeline
  audio.rs            AudioEngine, render thread, rtrb ring buffer, cpal
  analysis.rs         Hann-windowed FFT, 16 log-spaced bands, RMS
  db.rs               Turso embedded SQLite cache
assets/soundfonts/    GeneralUser GS license
scripts/              fetch-soundfont.sh
docs/
  ARCHITECTURE.md     Data flow, module map, Visualization plugin contract
  DECISIONS.md        ADR log (8 decisions)
```

---

## Docs

- [Architecture](docs/ARCHITECTURE.md) — data flow, module map, Visualization plugin contract
- [Design Decisions](docs/DECISIONS.md) — ADR log

---

## License

Apache-2.0 — see [LICENSE](LICENSE).

---

## Third-party assets

**GeneralUser GS v2.0** by S. Christian Collins  
A permissive General MIDI soundfont bundled with midimi for out-of-box audio playback.  
License: `assets/soundfonts/GeneralUser-GS-LICENSE.txt`  
Source: https://github.com/mrbumpy409/GeneralUser-GS
