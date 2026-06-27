# midimi v1 (Vertical Slice) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a dark-native Tauri desktop app that opens a `.mid` file, plays it through a bundled (swappable) SoundFont, and reacts with a magical cosmic/aurora visualization — with recent files, soundfonts, and settings persisted in a local embedded Turso DB.

**Architecture:** A Tauri v2 shell. The Rust backend parses MIDI (`midly`), synthesizes audio (`rustysynth`) on a dedicated render thread that pushes interleaved f32 into a lock-free ring buffer (`rtrb`) consumed by a `cpal` output stream, computes a cheap RMS + FFT band vector (`rustfft`) per block, and persists cache data in `turso`. A single playback clock (`MidiFileSequencer::get_position()`) is the source of time; the backend emits `{time, level, bands, playing}` at ~60 Hz. The Svelte 5 frontend renders one full-bleed `<canvas>` driven by that feed, deriving note-on/off events from the timeline it receives once at load.

**Tech Stack:** Rust + Tauri v2; `midly`, `rustysynth`, `cpal`, `rtrb`, `rustfft`, `turso`, `tokio`, `serde`. Frontend: Svelte 5 + TypeScript + Vite; `@tauri-apps/api`, `@tauri-apps/plugin-dialog`. SoundFont: GeneralUser GS v2.0.3.

## Global Constraints

These apply to **every** task. Exact, verified values (June 2026):

- **Rust deps** (`src-tauri/Cargo.toml`): `tauri = { version = "2", features = [] }`, `tauri-plugin-dialog = "2"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `midly = "0.5"`, `rustysynth = "1.3.6"`, `cpal = "0.18"`, `rtrb = "0.3"`, `rustfft = "6.4"`, `turso = "0.6.1"`, `tokio = { version = "1", features = ["full"] }`. Build-dep: `tauri-build = { version = "2", features = [] }`.
- **JS deps** (`package.json`): `@tauri-apps/api@^2`, `@tauri-apps/plugin-dialog@^2`; dev: `@tauri-apps/cli@^2`, `svelte@^5`, `@sveltejs/vite-plugin-svelte@^5`, `vite@^6`, `typescript@^5`, `svelte-check@^4`.
- **Tauri v2 layout:** entry is `src-tauri/src/lib.rs` exposing `pub fn run()`, called by a thin `src-tauri/src/main.rs`. Commands registered in ONE `tauri::generate_handler![...]`.
- **`emit` is on the `Emitter` trait; `manage`/`state` on `Manager`** — `use tauri::{Emitter, Manager};`.
- **Never hold a `std::sync::MutexGuard` across `.await`** in an async command. Lock + drop in a sync scope, then await.
- **`cpal` 0.18:** `build_output_stream(config, data_cb, err_cb, None)` takes `config` BY VALUE; `config.channels` is `u16`, `config.sample_rate` is `u32` (plain, not newtypes). The `Stream` stops when dropped — keep it alive in state.
- **`rustysynth` 1.3.6:** `render(&mut left, &mut right)` is PLANAR (separate buffers, NOT interleaved); both slices must be equal length. `MidiFileSequencer` has **NO seek** — implement seek by `play()` then render-and-discard to the target time. `set_speed(ratio)` is the tempo multiplier (panics if negative). Sequencer consumes the `Synthesizer` and only lends it back immutably.
- **`turso` 0.6.1:** `Builder::new_local(path).build().await` (async) → `db.connect()` (SYNC) → `Connection` (Clone + Send + Sync). `execute`/`query` are async; `Rows::next().await?` is a manual cursor. No-param calls need an explicit `()`; a single positional param needs a 1-tuple `("x",)`. Values are `Null|Integer(i64)|Real(f64)|Text(String)|Blob`. It is BETA — cache tables only.
- **SoundFont:** GeneralUser GS v2.0.3 (`GeneralUser-GS.sf2`, ~30.8 MB). Fetched by script (gitignored), bundled as a Tauri resource, loaded at runtime. License text vendored at `assets/soundfonts/GeneralUser-GS-LICENSE.txt`.
- **License:** Apache-2.0 for app code. **Sample rate:** build the synth at the cpal device's sample rate (no resampling). **Audio channels:** assume stereo (2) output. **Commit** after every task's final passing step.

---

## File Structure

```
midimi/
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs            # thin: calls midimi_lib::run()
│  │  ├─ lib.rs             # Tauri builder, command registration, state, 60Hz emit loop
│  │  ├─ commands.rs        # #[tauri::command] fns (IPC surface)
│  │  ├─ midi.rs            # parse_midi(bytes) -> MidiData  (pure, TDD)
│  │  ├─ analysis.rs        # BandAnalyzer + rms (pure, TDD)
│  │  ├─ audio.rs           # AudioEngine: render thread + ring buffer + cpal
│  │  └─ db.rs              # turso open + cache CRUD (TDD via :memory:)
│  ├─ capabilities/default.json   # + "dialog:default"
│  ├─ tauri.conf.json       # + bundle.resources for the soundfont
│  ├─ build.rs
│  └─ Cargo.toml
├─ src/
│  ├─ main.ts               # mounts App.svelte
│  ├─ App.svelte            # layout + top-level state wiring
│  ├─ app.css               # theme CSS variables + base dark styles
│  ├─ lib/
│  │  ├─ types.ts           # MidiData/Note/TrackInfo/Playhead mirrors
│  │  ├─ ipc.ts             # typed invoke wrappers + listenPlayhead
│  │  ├─ theme.ts           # theme registry (CSS var sets)
│  │  ├─ VizCanvas.svelte   # canvas host: RAF loop, derives note on/off
│  │  ├─ Transport.svelte   # play/pause/seek/tempo/volume
│  │  ├─ TrackList.svelte   # per-track name + mute/solo
│  │  ├─ SoundfontPicker.svelte
│  │  ├─ ThemePicker.svelte
│  │  └─ visualizations/
│  │     ├─ types.ts        # Visualization contract + NoteEvent
│  │     └─ cosmic-aurora.ts# the v1 built-in visualization
├─ assets/soundfonts/
│  ├─ GeneralUser-GS.sf2    # gitignored, fetched by script
│  └─ GeneralUser-GS-LICENSE.txt
├─ scripts/fetch-soundfont.sh
├─ .github/workflows/build.yml
├─ docs/{ARCHITECTURE.md,DECISIONS.md,superpowers/...}
├─ LICENSE  (Apache-2.0)
└─ README.md
```

---

## Task 0: Scaffold, dependencies, soundfont, license, CI

**Files:**
- Create: whole project via `create-tauri-app`, then `src-tauri/Cargo.toml`, `package.json`, `scripts/fetch-soundfont.sh`, `assets/soundfonts/GeneralUser-GS-LICENSE.txt`, `LICENSE`, `README.md`, `.github/workflows/build.yml`, `docs/ARCHITECTURE.md`, `docs/DECISIONS.md`
- Modify: `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `.gitignore`

**Interfaces:**
- Consumes: nothing (greenfield; the repo already has `docs/`, `.gitignore`, `LICENSE`-less root).
- Produces: a launchable Tauri app and all dependencies the later tasks import.

- [ ] **Step 1: Scaffold the app into the existing repo**

The repo root `midimi/` already exists with `docs/` and `.serena/`. Scaffold into a temp dir and move the generated files in (avoids create-tauri-app refusing a non-empty dir):

```bash
cd /Users/alec/Development/midimi
npm create tauri-app@latest _scaffold -- --template svelte-ts --manager npm
# Move generated files up into the repo root (keep existing docs/.git):
rsync -a _scaffold/ ./ && rm -rf _scaffold
```

If `--manager`/`--template` flags error on your CLI version, run `npm create tauri-app@latest _scaffold` interactively and choose: TypeScript → npm → **Svelte** → **TypeScript** (NOT SvelteKit).

- [ ] **Step 2: Pin dependencies**

Replace `src-tauri/Cargo.toml` `[dependencies]` (keep the generated `[package]`, `[build-dependencies]`, `[lib]` name = `midimi_lib`, and `[[bin]]`):

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
midly = "0.5"
rustysynth = "1.3.6"
cpal = "0.18"
rtrb = "0.3"
rustfft = "6.4"
turso = "0.6.1"
tokio = { version = "1", features = ["full"] }
```

Add the dialog plugin + api to JS:

```bash
npm install @tauri-apps/plugin-dialog
```

- [ ] **Step 3: Soundfont fetch script + vendored license + gitignore**

`scripts/fetch-soundfont.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
DEST="assets/soundfonts/GeneralUser-GS.sf2"
URL="https://github.com/mrbumpy409/GeneralUser-GS/raw/main/GeneralUser-GS.sf2"
mkdir -p assets/soundfonts
if [ -f "$DEST" ]; then echo "soundfont already present"; exit 0; fi
echo "Fetching GeneralUser GS (~31MB)…"
curl -L --fail -o "$DEST" "$URL"
echo "Saved $DEST"
```

Make it executable and run it. Create `assets/soundfonts/GeneralUser-GS-LICENSE.txt` containing the verbatim GeneralUser GS v2.0 license text (copy from https://github.com/mrbumpy409/GeneralUser-GS `documentation/LICENSE.txt`). Append to `.gitignore`:

```
# Bundled soundfont binary (fetched via scripts/fetch-soundfont.sh)
assets/soundfonts/*.sf2
```

```bash
chmod +x scripts/fetch-soundfont.sh && ./scripts/fetch-soundfont.sh
```

- [ ] **Step 4: Bundle the soundfont as a Tauri resource + dialog permission**

In `src-tauri/tauri.conf.json`, under `bundle`, add the resource and ensure a product name:

```json
"bundle": {
  "active": true,
  "targets": "all",
  "resources": { "../assets/soundfonts/GeneralUser-GS.sf2": "soundfonts/GeneralUser-GS.sf2" },
  "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.icns", "icons/icon.ico"]
}
```

In `src-tauri/capabilities/default.json`, add `"dialog:default"` to the `permissions` array.

- [ ] **Step 5: Apache-2.0 LICENSE, README, CI, docs stubs**

Create `LICENSE` with the full Apache-2.0 text (https://www.apache.org/licenses/LICENSE-2.0.txt). Create `README.md` with: what midimi is, prerequisites (Rust, Node, `./scripts/fetch-soundfont.sh`), `npm install`, `npm run tauri dev`, and a "Third-party assets" section crediting GeneralUser GS. Create `.github/workflows/build.yml`:

```yaml
name: build
on: { push: { branches: [main] }, pull_request: {} }
jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        os: [macos-latest, ubuntu-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - name: Linux deps
        if: matrix.os == 'ubuntu-latest'
        run: sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libasound2-dev build-essential libgtk-3-dev librsvg2-dev
      - run: bash scripts/fetch-soundfont.sh
      - run: npm install
      - uses: tauri-apps/tauri-action@v0
```

Create `docs/ARCHITECTURE.md` and `docs/DECISIONS.md` with a one-line intro each (filled in Task 10).

- [ ] **Step 6: Verify it launches, then commit**

Run: `npm run tauri dev`
Expected: the default Tauri+Svelte window opens. Close it.

```bash
git add -A && git commit -m "chore: scaffold Tauri v2 + Svelte 5, deps, soundfont, license, CI"
```

---

## Task 1: MIDI parsing → note timeline (`midi.rs`, TDD)

**Files:**
- Create: `src-tauri/src/midi.rs`
- Test: inline `#[cfg(test)]` module in `midi.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod midi;`)

**Interfaces:**
- Consumes: `midly = "0.5"`.
- Produces:
  - `pub struct Note { pub track: usize, pub channel: u8, pub note: u8, pub start_sec: f64, pub dur_sec: f64, pub velocity: u8 }` (derives `Serialize, Clone`)
  - `pub struct TrackInfo { pub index: usize, pub name: Option<String> }` (derives `Serialize, Clone`)
  - `pub struct MidiData { pub title: Option<String>, pub duration_sec: f64, pub tracks: Vec<TrackInfo>, pub notes: Vec<Note> }` (derives `Serialize, Clone`)
  - `pub fn parse_midi(bytes: &[u8]) -> Result<MidiData, String>`

- [ ] **Step 1: Write the failing test**

Add to `src-tauri/src/midi.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Minimal SMF: format 0, 1 track, 96 ticks/quarter, default tempo (120 BPM => quarter = 0.5s).
    // Events: NoteOn ch0 key60 vel100 @t0 ; NoteOff ch0 key60 @ +96 ticks ; EndOfTrack.
    fn tiny_midi() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"MThd");
        v.extend_from_slice(&[0, 0, 0, 6, 0, 0, 0, 1, 0, 96]); // len=6, fmt=0, ntrks=1, div=96
        v.extend_from_slice(b"MTrk");
        let track: [u8; 12] = [
            0x00, 0x90, 0x3C, 0x64, // dt0  NoteOn  ch0 key60 vel100
            0x60, 0x80, 0x3C, 0x00, // dt96 NoteOff ch0 key60 vel0
            0x00, 0xFF, 0x2F, 0x00, // dt0  EndOfTrack
        ];
        v.extend_from_slice(&(track.len() as u32).to_be_bytes());
        v.extend_from_slice(&track);
        v
    }

    #[test]
    fn parses_one_note_with_correct_timing() {
        let data = tiny_midi();
        let md = parse_midi(&data).expect("should parse");
        assert_eq!(md.tracks.len(), 1);
        assert_eq!(md.notes.len(), 1, "exactly one note");
        let n = &md.notes[0];
        assert_eq!(n.note, 60);
        assert_eq!(n.velocity, 100);
        assert!((n.start_sec - 0.0).abs() < 1e-6, "starts at 0");
        assert!((n.dur_sec - 0.5).abs() < 1e-3, "96 ticks @120BPM = 0.5s, got {}", n.dur_sec);
        assert!((md.duration_sec - 0.5).abs() < 1e-3);
    }

    #[test]
    fn note_on_velocity_zero_is_note_off() {
        // NoteOn vel0 should close the note, not open a second.
        let mut v = Vec::new();
        v.extend_from_slice(b"MThd");
        v.extend_from_slice(&[0, 0, 0, 6, 0, 0, 0, 1, 0, 96]);
        v.extend_from_slice(b"MTrk");
        let track: [u8; 12] = [
            0x00, 0x90, 0x40, 0x50, // NoteOn key64 vel80
            0x30, 0x90, 0x40, 0x00, // NoteOn key64 vel0  == NoteOff (dt48)
            0x00, 0xFF, 0x2F, 0x00,
        ];
        v.extend_from_slice(&(track.len() as u32).to_be_bytes());
        v.extend_from_slice(&track);
        let md = parse_midi(&v).unwrap();
        assert_eq!(md.notes.len(), 1);
        assert!((md.notes[0].dur_sec - 0.25).abs() < 1e-3);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd src-tauri && cargo test midi::`
Expected: FAIL — `parse_midi` not found / `mod midi` not declared.

- [ ] **Step 3: Implement `parse_midi`**

Put this ABOVE the test module in `src-tauri/src/midi.rs`:

```rust
use serde::Serialize;
use std::collections::HashMap;
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

#[derive(Serialize, Clone, Debug)]
pub struct Note {
    pub track: usize,
    pub channel: u8,
    pub note: u8,
    pub start_sec: f64,
    pub dur_sec: f64,
    pub velocity: u8,
}

#[derive(Serialize, Clone, Debug)]
pub struct TrackInfo {
    pub index: usize,
    pub name: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct MidiData {
    pub title: Option<String>,
    pub duration_sec: f64,
    pub tracks: Vec<TrackInfo>,
    pub notes: Vec<Note>,
}

// Convert an absolute tick to seconds using a sorted (tick, us_per_quarter) tempo map.
// ponytail: linear scan over tempo changes; real files have a handful. Switch to a
// cumulative prefix + binary search only if a file ever ships thousands of tempo events.
fn tick_to_sec(tick: u64, tempo_map: &[(u64, f64)], ticks_per_beat: f64) -> f64 {
    let mut sec = 0.0_f64;
    let mut last_tick = 0_u64;
    let mut us = 500_000.0_f64; // default 120 BPM until the first Set Tempo
    for &(t, u) in tempo_map.iter() {
        if t >= tick {
            break;
        }
        sec += (t - last_tick) as f64 * (us / 1_000_000.0) / ticks_per_beat;
        last_tick = t;
        us = u;
    }
    sec + (tick - last_tick) as f64 * (us / 1_000_000.0) / ticks_per_beat
}

pub fn parse_midi(bytes: &[u8]) -> Result<MidiData, String> {
    let smf = Smf::parse(bytes).map_err(|e| format!("invalid MIDI: {e}"))?;

    // Metrical => ticks-per-quarter + a tempo map. SMPTE => fixed ticks-per-second.
    let (ticks_per_beat, smpte_tps) = match smf.header.timing {
        Timing::Metrical(tpb) => (tpb.as_int() as f64, None),
        Timing::Timecode(fps, sub) => (1.0, Some(fps.as_f32() as f64 * sub as f64)),
    };

    // Pass 1: build a GLOBAL tempo map (absolute tick -> us/quarter) across all tracks.
    // In format-1 files all tracks share one tick clock; Set Tempo usually lives in track 0
    // but governs every track, so we hoist it to be global.
    let mut tempo_map: Vec<(u64, f64)> = Vec::new();
    if smpte_tps.is_none() {
        for track in smf.tracks.iter() {
            let mut abs: u64 = 0;
            for ev in track.iter() {
                abs += ev.delta.as_int() as u64;
                if let TrackEventKind::Meta(MetaMessage::Tempo(us)) = ev.kind {
                    tempo_map.push((abs, us.as_int() as f64));
                }
            }
        }
        tempo_map.sort_by_key(|&(t, _)| t);
        tempo_map.dedup_by_key(|&mut (t, _)| t);
    }

    let to_sec = |tick: u64| -> f64 {
        match smpte_tps {
            Some(tps) => tick as f64 / tps,
            None => tick_to_sec(tick, &tempo_map, ticks_per_beat),
        }
    };

    // Pass 2: notes + track names.
    let mut notes = Vec::new();
    let mut tracks = Vec::with_capacity(smf.tracks.len());
    for (track_index, track) in smf.tracks.iter().enumerate() {
        let mut abs: u64 = 0;
        let mut name: Option<String> = None;
        let mut open: HashMap<(u8, u8), (f64, u8)> = HashMap::new();
        for ev in track.iter() {
            abs += ev.delta.as_int() as u64;
            match ev.kind {
                TrackEventKind::Meta(MetaMessage::TrackName(n)) => {
                    name = Some(String::from_utf8_lossy(n).into_owned());
                }
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    match message {
                        MidiMessage::NoteOn { key, vel } => {
                            let (k, v) = (key.as_int(), vel.as_int());
                            if v == 0 {
                                if let Some((start, vel0)) = open.remove(&(ch, k)) {
                                    notes.push(Note { track: track_index, channel: ch, note: k,
                                        start_sec: start, dur_sec: to_sec(abs) - start, velocity: vel0 });
                                }
                            } else {
                                open.insert((ch, k), (to_sec(abs), v));
                            }
                        }
                        MidiMessage::NoteOff { key, .. } => {
                            let k = key.as_int();
                            if let Some((start, vel0)) = open.remove(&(ch, k)) {
                                notes.push(Note { track: track_index, channel: ch, note: k,
                                    start_sec: start, dur_sec: to_sec(abs) - start, velocity: vel0 });
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        // Close any notes left hanging at the track's final tick.
        let end = to_sec(abs);
        for ((ch, k), (start, vel0)) in open.drain() {
            notes.push(Note { track: track_index, channel: ch, note: k,
                start_sec: start, dur_sec: (end - start).max(0.0), velocity: vel0 });
        }
        tracks.push(TrackInfo { index: track_index, name });
    }

    notes.sort_by(|a, b| a.start_sec.partial_cmp(&b.start_sec).unwrap_or(std::cmp::Ordering::Equal));
    let duration_sec = notes.iter().map(|n| n.start_sec + n.dur_sec).fold(0.0_f64, f64::max);
    let title = tracks.iter().find_map(|t| t.name.clone());

    Ok(MidiData { title, duration_sec, tracks, notes })
}
```

Add `mod midi;` to `src-tauri/src/lib.rs` (near the top).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd src-tauri && cargo test midi::`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(midi): parse SMF into a flat note timeline with global tempo map"
```

---

## Task 2: Audio analysis — FFT bands + RMS (`analysis.rs`, TDD)

**Files:**
- Create: `src-tauri/src/analysis.rs`
- Test: inline `#[cfg(test)]` in `analysis.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod analysis;`)

**Interfaces:**
- Consumes: `rustfft = "6.4"`.
- Produces:
  - `pub struct BandAnalyzer { /* private */ }`
  - `pub fn BandAnalyzer::new(fft_size: usize, n_bands: usize, sample_rate: f32) -> BandAnalyzer`
  - `pub fn BandAnalyzer::analyze(&mut self, samples: &[f32]) -> Vec<f32>` (len == `n_bands`, ~0..1)
  - `pub fn rms(samples: &[f32]) -> f32`

- [ ] **Step 1: Write the failing test**

Add to `src-tauri/src/analysis.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    #[test]
    fn loud_band_matches_input_frequency() {
        let sr = 44_100.0_f32;
        let n = 1024;
        let n_bands = 8;
        // Pure tone at 2000 Hz.
        let freq = 2000.0_f32;
        let samples: Vec<f32> = (0..n).map(|i| (TAU * freq * i as f32 / sr).sin()).collect();
        let mut a = BandAnalyzer::new(n, n_bands, sr);
        let bands = a.analyze(&samples);
        assert_eq!(bands.len(), n_bands);
        // The hottest band should be the one whose frequency range contains 2000 Hz,
        // and it should clearly dominate a low band.
        let max_idx = bands.iter().enumerate().max_by(|x, y| x.1.partial_cmp(y.1).unwrap()).unwrap().0;
        assert!(max_idx >= 3, "2kHz should land in an upper band, got band {max_idx}");
        assert!(bands[max_idx] > bands[0] * 5.0, "tone band must dominate the lowest band");
    }

    #[test]
    fn rms_of_silence_is_zero_and_signal_is_positive() {
        assert!(rms(&[0.0; 256]) < 1e-9);
        assert!(rms(&[0.5; 256]) > 0.4);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd src-tauri && cargo test analysis::`
Expected: FAIL — `BandAnalyzer`/`rms` not found.

- [ ] **Step 3: Implement the analyzer**

Put above the test module:

```rust
use std::sync::Arc;
use rustfft::{num_complex::Complex, Fft, FftPlanner};

pub struct BandAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buffer: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    band_edges: Vec<usize>,
}

impl BandAnalyzer {
    pub fn new(fft_size: usize, n_bands: usize, sample_rate: f32) -> Self {
        // Plan ONCE; re-planning per frame is the dominant perf footgun.
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let denom = (fft_size - 1) as f32;
        let window: Vec<f32> = (0..fft_size)
            .map(|n| 0.5 * (1.0 - (std::f32::consts::TAU * n as f32 / denom).cos()))
            .collect();
        let scratch = vec![Complex { re: 0.0, im: 0.0 }; fft.get_inplace_scratch_len()];

        // Log-spaced band edges in bin indices over the usable 1..=N/2 range.
        let nyq_bin = fft_size / 2;
        let bin_hz = sample_rate / fft_size as f32;
        let f_min = 40.0_f32;
        let f_max = sample_rate * 0.5;
        let mut band_edges = Vec::with_capacity(n_bands + 1);
        for b in 0..=n_bands {
            let frac = b as f32 / n_bands as f32;
            let f = f_min * (f_max / f_min).powf(frac);
            band_edges.push(((f / bin_hz).round() as usize).clamp(1, nyq_bin));
        }
        Self {
            fft,
            window,
            buffer: vec![Complex { re: 0.0, im: 0.0 }; fft_size],
            scratch,
            band_edges,
        }
    }

    /// `samples` must be >= fft_size long. Returns one magnitude per band (~0..1).
    pub fn analyze(&mut self, samples: &[f32]) -> Vec<f32> {
        let n = self.buffer.len();
        for i in 0..n {
            self.buffer[i] = Complex { re: samples[i] * self.window[i], im: 0.0 };
        }
        self.fft.process_with_scratch(&mut self.buffer, &mut self.scratch);
        let norm = 1.0 / (n as f32 * 0.5);
        let mut out = Vec::with_capacity(self.band_edges.len() - 1);
        for w in self.band_edges.windows(2) {
            let lo = w[0];
            let hi = w[1].max(w[0] + 1);
            let mut sum = 0.0_f32;
            for bin in lo..hi {
                sum += self.buffer[bin].norm();
            }
            out.push(((sum / (hi - lo) as f32) * norm).min(1.0));
        }
        out
    }
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}
```

Add `mod analysis;` to `src-tauri/src/lib.rs`.

- [ ] **Step 4: Run to verify pass**

Run: `cd src-tauri && cargo test analysis::`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(analysis): log-spaced FFT band analyzer + RMS for viz glow"
```

---

## Task 3: Local Turso cache layer (`db.rs`, TDD)

**Files:**
- Create: `src-tauri/src/db.rs`
- Test: inline `#[cfg(test)]` in `db.rs` (uses `:memory:`)
- Modify: `src-tauri/src/lib.rs` (add `mod db;`)

**Interfaces:**
- Consumes: `turso = "0.6.1"`, `tokio` (tests).
- Produces (all `async`, all take `&turso::Connection`):
  - `pub async fn open(path: &str) -> Result<turso::Connection, String>` — opens + runs schema.
  - `pub struct LibraryRow { pub id: i64, pub path: String, pub title: Option<String>, pub duration_sec: f64 }` (`Serialize, Clone`)
  - `pub struct SoundfontRow { pub id: i64, pub path: String, pub name: String, pub is_builtin: bool }` (`Serialize, Clone`)
  - `pub struct Setting { pub key: String, pub value: String }` (`Serialize, Clone`)
  - `pub async fn upsert_recent(conn, path, title: Option<&str>, duration_sec, now_unix: i64) -> Result<(), String>`
  - `pub async fn list_recent(conn, limit: i64) -> Result<Vec<LibraryRow>, String>`
  - `pub async fn register_soundfont(conn, path, name, is_builtin: bool) -> Result<SoundfontRow, String>`
  - `pub async fn list_soundfonts(conn) -> Result<Vec<SoundfontRow>, String>`
  - `pub async fn get_setting(conn, key) -> Result<Option<String>, String>`
  - `pub async fn set_setting(conn, key, value) -> Result<(), String>`
  - `pub async fn list_settings(conn) -> Result<Vec<Setting>, String>`

- [ ] **Step 1: Write the failing test**

Add to `src-tauri/src/db.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn settings_roundtrip() {
        let conn = open(":memory:").await.unwrap();
        assert_eq!(get_setting(&conn, "theme").await.unwrap(), None);
        set_setting(&conn, "theme", "cosmic").await.unwrap();
        set_setting(&conn, "theme", "aurora").await.unwrap(); // upsert overwrites
        assert_eq!(get_setting(&conn, "theme").await.unwrap(), Some("aurora".to_string()));
        assert_eq!(list_settings(&conn).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn recent_is_ordered_and_deduped() {
        let conn = open(":memory:").await.unwrap();
        upsert_recent(&conn, "/a.mid", Some("A"), 1.0, 100).await.unwrap();
        upsert_recent(&conn, "/b.mid", Some("B"), 2.0, 200).await.unwrap();
        upsert_recent(&conn, "/a.mid", Some("A"), 1.0, 300).await.unwrap(); // re-open A, newest
        let rows = list_recent(&conn, 10).await.unwrap();
        assert_eq!(rows.len(), 2, "path is unique");
        assert_eq!(rows[0].path, "/a.mid", "most-recent first");
        assert_eq!(rows[1].path, "/b.mid");
    }

    #[tokio::test]
    async fn soundfont_register_and_list() {
        let conn = open(":memory:").await.unwrap();
        let row = register_soundfont(&conn, "/gm.sf2", "GeneralUser GS", true).await.unwrap();
        assert!(row.id > 0);
        assert!(row.is_builtin);
        let all = list_soundfonts(&conn).await.unwrap();
        assert_eq!(all.len(), 1);
        // Re-register same path is idempotent (no duplicate).
        register_soundfont(&conn, "/gm.sf2", "GeneralUser GS", true).await.unwrap();
        assert_eq!(list_soundfonts(&conn).await.unwrap().len(), 1);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd src-tauri && cargo test db::`
Expected: FAIL — `open`/functions not found.

- [ ] **Step 3: Implement the DB layer**

Put above the tests:

```rust
use serde::Serialize;
use turso::{Builder, Value};

#[derive(Serialize, Clone, Debug)]
pub struct LibraryRow { pub id: i64, pub path: String, pub title: Option<String>, pub duration_sec: f64 }
#[derive(Serialize, Clone, Debug)]
pub struct SoundfontRow { pub id: i64, pub path: String, pub name: String, pub is_builtin: bool }
#[derive(Serialize, Clone, Debug)]
pub struct Setting { pub key: String, pub value: String }

pub async fn open(path: &str) -> Result<turso::Connection, String> {
    let db = Builder::new_local(path).build().await.map_err(|e| e.to_string())?;
    let conn = db.connect().map_err(|e| e.to_string())?; // connect() is SYNC
    for stmt in [
        "CREATE TABLE IF NOT EXISTS library (id INTEGER PRIMARY KEY, path TEXT UNIQUE NOT NULL, title TEXT, duration_sec REAL, last_opened_at INTEGER NOT NULL)",
        "CREATE TABLE IF NOT EXISTS soundfonts (id INTEGER PRIMARY KEY, path TEXT UNIQUE NOT NULL, name TEXT NOT NULL, is_builtin INTEGER NOT NULL DEFAULT 0)",
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
    ] {
        conn.execute(stmt, ()).await.map_err(|e| e.to_string())?;
    }
    Ok(conn)
}

fn opt_text(v: Value) -> Option<String> {
    match v { Value::Text(s) => Some(s), _ => None }
}

pub async fn upsert_recent(conn: &turso::Connection, path: &str, title: Option<&str>, duration_sec: f64, now_unix: i64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO library (path, title, duration_sec, last_opened_at) VALUES (?1, ?2, ?3, ?4)\n         ON CONFLICT(path) DO UPDATE SET title=?2, duration_sec=?3, last_opened_at=?4",
        (path, title, duration_sec, now_unix),
    ).await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn list_recent(conn: &turso::Connection, limit: i64) -> Result<Vec<LibraryRow>, String> {
    let mut rows = conn.query(
        "SELECT id, path, title, duration_sec FROM library ORDER BY last_opened_at DESC LIMIT ?1",
        (limit,),
    ).await.map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(LibraryRow {
            id: row.get::<i64>(0).map_err(|e| e.to_string())?,
            path: row.get::<String>(1).map_err(|e| e.to_string())?,
            title: opt_text(row.get_value(2).map_err(|e| e.to_string())?),
            duration_sec: row.get::<f64>(3).map_err(|e| e.to_string())?,
        });
    }
    Ok(out)
}

pub async fn register_soundfont(conn: &turso::Connection, path: &str, name: &str, is_builtin: bool) -> Result<SoundfontRow, String> {
    conn.execute(
        "INSERT INTO soundfonts (path, name, is_builtin) VALUES (?1, ?2, ?3)\n         ON CONFLICT(path) DO UPDATE SET name=?2, is_builtin=?3",
        (path, name, is_builtin as i64),
    ).await.map_err(|e| e.to_string())?;
    let mut rows = conn.query("SELECT id, path, name, is_builtin FROM soundfonts WHERE path=?1", (path,))
        .await.map_err(|e| e.to_string())?;
    let row = rows.next().await.map_err(|e| e.to_string())?.ok_or("soundfont insert vanished")?;
    Ok(SoundfontRow {
        id: row.get::<i64>(0).map_err(|e| e.to_string())?,
        path: row.get::<String>(1).map_err(|e| e.to_string())?,
        name: row.get::<String>(2).map_err(|e| e.to_string())?,
        is_builtin: row.get::<i64>(3).map_err(|e| e.to_string())? != 0,
    })
}

pub async fn list_soundfonts(conn: &turso::Connection) -> Result<Vec<SoundfontRow>, String> {
    let mut rows = conn.query("SELECT id, path, name, is_builtin FROM soundfonts ORDER BY is_builtin DESC, name ASC", ())
        .await.map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(SoundfontRow {
            id: row.get::<i64>(0).map_err(|e| e.to_string())?,
            path: row.get::<String>(1).map_err(|e| e.to_string())?,
            name: row.get::<String>(2).map_err(|e| e.to_string())?,
            is_builtin: row.get::<i64>(3).map_err(|e| e.to_string())? != 0,
        });
    }
    Ok(out)
}

pub async fn get_setting(conn: &turso::Connection, key: &str) -> Result<Option<String>, String> {
    let mut rows = conn.query("SELECT value FROM settings WHERE key=?1", (key,)).await.map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(row) => Ok(Some(row.get::<String>(0).map_err(|e| e.to_string())?)),
        None => Ok(None),
    }
}

pub async fn set_setting(conn: &turso::Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2",
        (key, value),
    ).await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn list_settings(conn: &turso::Connection) -> Result<Vec<Setting>, String> {
    let mut rows = conn.query("SELECT key, value FROM settings", ()).await.map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(Setting {
            key: row.get::<String>(0).map_err(|e| e.to_string())?,
            value: row.get::<String>(1).map_err(|e| e.to_string())?,
        });
    }
    Ok(out)
}
```

Add `mod db;` to `src-tauri/src/lib.rs`.

- [ ] **Step 4: Run to verify pass**

Run: `cd src-tauri && cargo test db::`
Expected: PASS (3 tests). If `turso` reports a missing SQL feature for `ON CONFLICT`, fall back to a `SELECT`-then-`INSERT or UPDATE` in `upsert_recent`/`set_setting`/`register_soundfont` (same signatures) — but `ON CONFLICT` on a UNIQUE/PK column is supported in 0.6.1.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(db): local Turso cache layer for library, soundfonts, settings"
```

---

## Task 4: Audio engine — render thread, ring buffer, cpal, offline render (`audio.rs`)

**Files:**
- Create: `src-tauri/src/audio.rs`
- Test: inline `#[cfg(test)]` in `audio.rs` (covers the offline render core; live streaming is a manual smoke check in Task 7)
- Modify: `src-tauri/src/lib.rs` (add `mod audio;`)

**Interfaces:**
- Consumes: `crate::analysis::{BandAnalyzer, rms}`; `rustysynth`, `cpal`, `rtrb`.
- Produces:
  - `pub const N_BANDS: usize = 16;`
  - `pub struct Snapshot { pub position_sec: f64, pub duration_sec: f64, pub level: f32, pub bands: Vec<f32>, pub playing: bool }` (derives `Clone, Default`)
  - `pub struct AudioEngine { /* private */ }` with: `pub fn new() -> Result<AudioEngine, String>`, `pub fn shared(&self) -> Arc<Mutex<Snapshot>>`, `pub fn load(&self, sf: Arc<SoundFont>, midi: Arc<MidiFile>, duration: f64)`, `pub fn play(&self)`, `pub fn pause(&self)`, `pub fn seek(&self, sec: f64)`, `pub fn set_speed(&self, ratio: f64)`, `pub fn set_volume(&self, v: f32)`
  - `pub fn render_offline(sf: &Arc<SoundFont>, midi: &Arc<MidiFile>, sample_rate: i32) -> Vec<f32>` (interleaved stereo; also the future WAV/MP3 export seam)

- [ ] **Step 1: Write the failing test (offline render core)**

Add to `src-tauri/src/audio.rs`. It uses the fetched soundfont; if absent it skips (CI fetches it before tests):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use rustysynth::{MidiFile, SoundFont};

    fn tiny_midi() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"MThd");
        v.extend_from_slice(&[0, 0, 0, 6, 0, 0, 0, 1, 0, 96]);
        v.extend_from_slice(b"MTrk");
        let track: [u8; 12] = [0x00,0x90,0x3C,0x64, 0x60,0x80,0x3C,0x00, 0x00,0xFF,0x2F,0x00];
        v.extend_from_slice(&(track.len() as u32).to_be_bytes());
        v.extend_from_slice(&track);
        v
    }

    #[test]
    fn offline_render_produces_audible_stereo() {
        let sf_path = "../assets/soundfonts/GeneralUser-GS.sf2";
        if !std::path::Path::new(sf_path).exists() {
            eprintln!("skipping offline_render test: run ./scripts/fetch-soundfont.sh first");
            return;
        }
        let sf_bytes = std::fs::read(sf_path).unwrap();
        let sf = Arc::new(SoundFont::new(&mut std::io::Cursor::new(sf_bytes)).unwrap());
        let midi = Arc::new(MidiFile::new(&mut std::io::Cursor::new(tiny_midi())).unwrap());
        let pcm = render_offline(&sf, &midi, 44_100);
        // ~0.5s of stereo at 44.1k ≈ 44100 interleaved samples (allow generous slack for release tail).
        assert!(pcm.len() >= 20_000, "expected a meaningful number of samples, got {}", pcm.len());
        assert!(pcm.len() % 2 == 0, "interleaved stereo => even length");
        let peak = pcm.iter().fold(0.0_f32, |m, &s| m.max(s.abs()));
        assert!(peak > 0.001, "rendered audio should not be silent, peak={peak}");
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd src-tauri && cargo test audio::`
Expected: FAIL — `render_offline` not found (compile error).

- [ ] **Step 3: Implement the engine + offline render**

Put above the tests:

```rust
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Producer, RingBuffer};
use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};

use crate::analysis::{rms, BandAnalyzer};

pub const N_BANDS: usize = 16;
const FFT_SIZE: usize = 1024;
const BLOCK: usize = 1024; // frames per render iteration (FFT_SIZE == BLOCK keeps analysis simple)

#[derive(Clone, Default)]
pub struct Snapshot {
    pub position_sec: f64,
    pub duration_sec: f64,
    pub level: f32,
    pub bands: Vec<f32>,
    pub playing: bool,
}

enum Cmd {
    Load { sf: Arc<SoundFont>, midi: Arc<MidiFile>, duration: f64 },
    Play,
    Pause,
    Seek(f64),
    SetSpeed(f64),
    SetVolume(f32),
}

pub struct AudioEngine {
    tx: Sender<Cmd>,
    snapshot: Arc<Mutex<Snapshot>>,
    _stream: cpal::Stream, // kept alive; dropping it stops audio
    _render: JoinHandle<()>,
}

impl AudioEngine {
    pub fn new() -> Result<AudioEngine, String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("no audio output device")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        if supported.sample_format() != cpal::SampleFormat::F32 {
            return Err(format!("unsupported sample format {:?} (expected f32)", supported.sample_format()));
        }
        let sample_rate: u32 = supported.sample_rate();
        // ponytail: force stereo; desktop default outputs are ~always f32 stereo. Revisit if a
        // device rejects this config.
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let (producer, mut consumer) = RingBuffer::<f32>::new(BLOCK * 2 * 8);
        let err_fn = |e: cpal::Error| eprintln!("audio stream error: {e}");
        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Real-time thread: only pop. Underrun => silence, never block/allocate/lock.
                    for s in data.iter_mut() {
                        *s = consumer.pop().unwrap_or(0.0);
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;

        let snapshot = Arc::new(Mutex::new(Snapshot { bands: vec![0.0; N_BANDS], ..Default::default() }));
        let (tx, rx) = channel::<Cmd>();
        let snap2 = snapshot.clone();
        let render = std::thread::spawn(move || render_loop(rx, producer, snap2, sample_rate));

        Ok(AudioEngine { tx, snapshot, _stream: stream, _render: render })
    }

    pub fn shared(&self) -> Arc<Mutex<Snapshot>> { self.snapshot.clone() }
    pub fn load(&self, sf: Arc<SoundFont>, midi: Arc<MidiFile>, duration: f64) { let _ = self.tx.send(Cmd::Load { sf, midi, duration }); }
    pub fn play(&self) { let _ = self.tx.send(Cmd::Play); }
    pub fn pause(&self) { let _ = self.tx.send(Cmd::Pause); }
    pub fn seek(&self, sec: f64) { let _ = self.tx.send(Cmd::Seek(sec)); }
    pub fn set_speed(&self, ratio: f64) { let _ = self.tx.send(Cmd::SetSpeed(ratio)); }
    pub fn set_volume(&self, v: f32) { let _ = self.tx.send(Cmd::SetVolume(v)); }
}

fn render_loop(rx: Receiver<Cmd>, mut producer: Producer<f32>, snapshot: Arc<Mutex<Snapshot>>, sample_rate: u32) {
    let mut analyzer = BandAnalyzer::new(FFT_SIZE, N_BANDS, sample_rate as f32);
    let mut seq: Option<MidiFileSequencer> = None;
    let mut midi: Option<Arc<MidiFile>> = None;
    let mut duration = 0.0_f64;
    let mut playing = false;
    let mut volume = 1.0_f32;

    let mut left = vec![0.0_f32; BLOCK];
    let mut right = vec![0.0_f32; BLOCK];
    let mut mono = vec![0.0_f32; FFT_SIZE];

    loop {
        // Drain control messages (non-blocking).
        loop {
            match rx.try_recv() {
                Ok(Cmd::Load { sf, midi: m, duration: d }) => {
                    let settings = SynthesizerSettings::new(sample_rate as i32);
                    match Synthesizer::new(&sf, &settings) {
                        Ok(synth) => {
                            let mut s = MidiFileSequencer::new(synth);
                            s.play(&m, false);
                            seq = Some(s);
                            midi = Some(m);
                            duration = d;
                            playing = true; // auto-play on open
                        }
                        Err(e) => eprintln!("synthesizer init failed: {e}"),
                    }
                }
                Ok(Cmd::Play) => playing = true,
                Ok(Cmd::Pause) => playing = false,
                Ok(Cmd::SetVolume(v)) => volume = v.clamp(0.0, 2.0),
                Ok(Cmd::SetSpeed(r)) => { if let Some(s) = seq.as_mut() { s.set_speed(r.max(0.0)); } }
                Ok(Cmd::Seek(target)) => {
                    if let (Some(s), Some(m)) = (seq.as_mut(), midi.as_ref()) {
                        // rustysynth has NO seek: replay from 0 then render-and-discard to target.
                        s.play(m, false);
                        while s.get_position() < target && !s.end_of_sequence() {
                            s.render(&mut left, &mut right);
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return, // engine dropped
            }
        }

        let can_render = playing && seq.is_some() && producer.slots() >= BLOCK * 2;
        if can_render {
            let s = seq.as_mut().unwrap();
            s.render(&mut left, &mut right); // planar L/R
            for i in 0..BLOCK {
                let _ = producer.push(left[i] * volume);
                let _ = producer.push(right[i] * volume);
            }
            for i in 0..FFT_SIZE { mono[i] = 0.5 * (left[i] + right[i]); }
            let bands = analyzer.analyze(&mono);
            let level = rms(&left);
            let pos = s.get_position();
            if s.end_of_sequence() { playing = false; }
            if let Ok(mut snap) = snapshot.lock() {
                snap.position_sec = pos;
                snap.duration_sec = duration;
                snap.level = level;
                snap.bands = bands;
                snap.playing = playing;
            }
        } else {
            if let Ok(mut snap) = snapshot.lock() { snap.playing = playing; }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

/// Render an entire sequence offline to interleaved stereo f32. Used by the test now and by
/// WAV/MP3 export later (feed the result to `hound` / an MP3 encoder).
pub fn render_offline(sf: &Arc<SoundFont>, midi: &Arc<MidiFile>, sample_rate: i32) -> Vec<f32> {
    let settings = SynthesizerSettings::new(sample_rate);
    let synth = Synthesizer::new(sf, &settings).expect("synthesizer");
    let mut seq = MidiFileSequencer::new(synth);
    seq.play(midi, false);
    let mut left = vec![0.0_f32; 4096];
    let mut right = vec![0.0_f32; 4096];
    let mut out = Vec::new();
    let cap_secs = midi.get_length() + 5.0; // safety bound
    while !seq.end_of_sequence() {
        seq.render(&mut left, &mut right);
        for i in 0..left.len() { out.push(left[i]); out.push(right[i]); }
        if out.len() as f64 / 2.0 / sample_rate as f64 > cap_secs { break; }
    }
    out
}
```

Add `mod audio;` to `src-tauri/src/lib.rs`.

- [ ] **Step 4: Run to verify pass**

Run: `cd src-tauri && cargo test audio::`
Expected: PASS (1 test; or a printed skip line if the soundfont isn't fetched — in that case run `./scripts/fetch-soundfont.sh` from the repo root and re-run, and confirm it PASSES).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(audio): rustysynth render thread + rtrb ring buffer + cpal output; offline render core"
```

---

## Task 5: Tauri command surface + state wiring + 60 Hz emit loop (`commands.rs`, `lib.rs`)

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs` (full `run()` + `mod commands;`), `src-tauri/src/main.rs` (keep generated `fn main(){ <lib>::run() }`)

**Interfaces:**
- Consumes: `crate::audio::AudioEngine`, `crate::db`, `crate::midi::{parse_midi, MidiData}`.
- Produces:
  - `pub struct AppState { pub engine: Mutex<AudioEngine>, pub db: turso::Connection, pub current_sf: Mutex<Option<Arc<SoundFont>>>, pub current_midi: Mutex<Option<Arc<MidiFile>>>, pub builtin_sf_path: String }`
  - Commands: `open_midi(path)->MidiData`, `play()`, `pause()`, `seek(seconds)`, `set_tempo(ratio)`, `set_volume(volume)`, `load_soundfont(path)->SoundfontRow`, `set_soundfont(id)`, `list_soundfonts()->Vec<SoundfontRow>`, `list_recent()->Vec<LibraryRow>`, `get_settings()->Vec<Setting>`, `set_setting(key,value)`
  - Event emitted: `"playhead"` → `{ time, duration, level, bands, playing }`

- [ ] **Step 1: Write `commands.rs`**

```rust
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rustysynth::{MidiFile, SoundFont};
use tauri::State;

use crate::audio::AudioEngine;
use crate::db;
use crate::midi::{parse_midi, MidiData};

pub struct AppState {
    pub engine: Mutex<AudioEngine>,
    pub db: turso::Connection,
    pub current_sf: Mutex<Option<Arc<SoundFont>>>,
    pub current_midi: Mutex<Option<Arc<MidiFile>>>,
    pub builtin_sf_path: String,
}

fn now_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn load_sf_arc(path: &str) -> Result<Arc<SoundFont>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let sf = SoundFont::new(&mut std::io::Cursor::new(bytes)).map_err(|e| format!("soundfont load: {e:?}"))?;
    Ok(Arc::new(sf))
}

// Swap the active soundfont and (if a song is loaded) restart it through the new sound.
// ponytail v1: restarts from 0 on soundfont change; carry-position is a later nicety.
fn apply_soundfont(state: &State<'_, AppState>, sf: Arc<SoundFont>) -> Result<(), String> {
    *state.current_sf.lock().map_err(|e| e.to_string())? = Some(sf.clone());
    let midi = state.current_midi.lock().map_err(|e| e.to_string())?.clone();
    if let Some(midi) = midi {
        let dur = midi.get_length();
        state.engine.lock().map_err(|e| e.to_string())?.load(sf, midi, dur);
    }
    Ok(())
}

#[tauri::command]
pub async fn open_midi(state: State<'_, AppState>, path: String) -> Result<MidiData, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let md = parse_midi(&bytes)?;
    let midi = Arc::new(MidiFile::new(&mut std::io::Cursor::new(bytes)).map_err(|e| format!("midi load: {e:?}"))?);

    // Ensure a soundfont is loaded (builtin by default). Lock + drop before any await.
    let sf = {
        let mut cur = state.current_sf.lock().map_err(|e| e.to_string())?;
        if cur.is_none() {
            *cur = Some(load_sf_arc(&state.builtin_sf_path)?);
        }
        cur.as_ref().unwrap().clone()
    };
    { state.engine.lock().map_err(|e| e.to_string())?.load(sf, midi.clone(), md.duration_sec); }
    *state.current_midi.lock().map_err(|e| e.to_string())? = Some(midi);

    db::upsert_recent(&state.db, &path, md.title.as_deref(), md.duration_sec, now_unix()).await?;
    Ok(md)
}

#[tauri::command]
pub fn play(state: State<'_, AppState>) -> Result<(), String> { state.engine.lock().map_err(|e| e.to_string())?.play(); Ok(()) }
#[tauri::command]
pub fn pause(state: State<'_, AppState>) -> Result<(), String> { state.engine.lock().map_err(|e| e.to_string())?.pause(); Ok(()) }
#[tauri::command]
pub fn seek(state: State<'_, AppState>, seconds: f64) -> Result<(), String> { state.engine.lock().map_err(|e| e.to_string())?.seek(seconds); Ok(()) }
#[tauri::command]
pub fn set_tempo(state: State<'_, AppState>, ratio: f64) -> Result<(), String> { state.engine.lock().map_err(|e| e.to_string())?.set_speed(ratio); Ok(()) }
#[tauri::command]
pub fn set_volume(state: State<'_, AppState>, volume: f64) -> Result<(), String> { state.engine.lock().map_err(|e| e.to_string())?.set_volume(volume as f32); Ok(()) }

#[tauri::command]
pub async fn load_soundfont(state: State<'_, AppState>, path: String) -> Result<db::SoundfontRow, String> {
    let sf = load_sf_arc(&path)?;
    apply_soundfont(&state, sf)?;
    let name = std::path::Path::new(&path).file_stem().and_then(|s| s.to_str()).unwrap_or("SoundFont").to_string();
    db::register_soundfont(&state.db, &path, &name, false).await
}

#[tauri::command]
pub async fn set_soundfont(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let row = db::list_soundfonts(&state.db).await?.into_iter().find(|s| s.id == id).ok_or("unknown soundfont id")?;
    let sf = load_sf_arc(&row.path)?;
    apply_soundfont(&state, sf)
}

#[tauri::command]
pub async fn list_soundfonts(state: State<'_, AppState>) -> Result<Vec<db::SoundfontRow>, String> { db::list_soundfonts(&state.db).await }
#[tauri::command]
pub async fn list_recent(state: State<'_, AppState>) -> Result<Vec<db::LibraryRow>, String> { db::list_recent(&state.db, 20).await }
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Vec<db::Setting>, String> { db::list_settings(&state.db).await }
#[tauri::command]
pub async fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> { db::set_setting(&state.db, &key, &value).await }
```

- [ ] **Step 2: Write `lib.rs` `run()`**

Replace the body of `src-tauri/src/lib.rs` (keep the generated module/lib name) with:

```rust
mod analysis;
mod audio;
mod commands;
mod db;
mod midi;

use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager};

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Resolve the bundled soundfont (Tauri resource).
            let sf_path = app
                .path()
                .resolve("soundfonts/GeneralUser-GS.sf2", tauri::path::BaseDirectory::Resource)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .into_owned();

            // Open the local Turso cache in the app data dir (one-time blocking init).
            let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&data_dir).ok();
            let db_file = data_dir.join("midimi.db").to_string_lossy().into_owned();
            let conn = tauri::async_runtime::block_on(async { db::open(&db_file).await })
                .map_err(|e| e.to_string())?;

            // Register the builtin soundfont (best effort).
            {
                let conn = conn.clone();
                let p = sf_path.clone();
                tauri::async_runtime::block_on(async move {
                    let _ = db::register_soundfont(&conn, &p, "GeneralUser GS", true).await;
                });
            }

            // Audio engine + its shared snapshot.
            let engine = audio::AudioEngine::new().map_err(|e| e.to_string())?;
            let shared = engine.shared();

            app.manage(AppState {
                engine: Mutex::new(engine),
                db: conn,
                current_sf: Mutex::new(None),
                current_midi: Mutex::new(None),
                builtin_sf_path: sf_path,
            });

            // 60 Hz playhead emit loop.
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                let snap = shared.lock().ok().map(|s| s.clone());
                if let Some(s) = snap {
                    let _ = handle.emit(
                        "playhead",
                        serde_json::json!({
                            "time": s.position_sec,
                            "duration": s.duration_sec,
                            "level": s.level,
                            "bands": s.bands,
                            "playing": s.playing,
                        }),
                    );
                }
                std::thread::sleep(Duration::from_millis(16));
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_midi,
            commands::play,
            commands::pause,
            commands::seek,
            commands::set_tempo,
            commands::set_volume,
            commands::load_soundfont,
            commands::set_soundfont,
            commands::list_soundfonts,
            commands::list_recent,
            commands::get_settings,
            commands::set_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running midimi");
}
```

Keep `src-tauri/src/main.rs` as generated (it calls `<crate>_lib::run()`).

- [ ] **Step 3: Verify it compiles and unit tests still pass**

Run: `cd src-tauri && cargo build && cargo test`
Expected: build succeeds; all prior tests (midi/analysis/db/audio) PASS. (First real audible playback is verified in Task 7, once the frontend can trigger `open_midi`.)

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(ipc): Tauri command surface, managed state, 60Hz playhead emit loop"
```

---

## Task 6: Frontend types, IPC client, first audible playback (`types.ts`, `ipc.ts`, `App.svelte`)

**Files:**
- Create: `src/lib/types.ts`, `src/lib/ipc.ts`
- Modify: `src/App.svelte` (replace the generated demo)

**Interfaces:**
- Consumes: backend commands + `"playhead"` event from Task 5.
- Produces (TS mirrors of the Rust serde types — names MUST match the JSON):
  - `types.ts`: `Note`, `TrackInfo`, `MidiData`, `Playhead`, `SoundfontRow`, `LibraryRow`, `Setting`
  - `ipc.ts`: `openMidi`, `play`, `pause`, `seek`, `setTempo`, `setVolume`, `loadSoundfont`, `setSoundfont`, `listSoundfonts`, `listRecent`, `getSettings`, `setSetting`, `listenPlayhead`, `pickMidi`, `pickSoundfont`

- [ ] **Step 1: Write `src/lib/types.ts`**

```ts
export interface Note { track: number; channel: number; note: number; start_sec: number; dur_sec: number; velocity: number; }
export interface TrackInfo { index: number; name: string | null; }
export interface MidiData { title: string | null; duration_sec: number; tracks: TrackInfo[]; notes: Note[]; }
export interface Playhead { time: number; duration: number; level: number; bands: number[]; playing: boolean; }
export interface SoundfontRow { id: number; path: string; name: string; is_builtin: boolean; }
export interface LibraryRow { id: number; path: string; title: string | null; duration_sec: number; }
export interface Setting { key: string; value: string; }
```

- [ ] **Step 2: Write `src/lib/ipc.ts`**

```ts
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { MidiData, Playhead, SoundfontRow, LibraryRow, Setting } from './types';

export const openMidi = (path: string) => invoke<MidiData>('open_midi', { path });
export const play = () => invoke('play');
export const pause = () => invoke('pause');
export const seek = (seconds: number) => invoke('seek', { seconds });
export const setTempo = (ratio: number) => invoke('set_tempo', { ratio });
export const setVolume = (volume: number) => invoke('set_volume', { volume });
export const loadSoundfont = (path: string) => invoke<SoundfontRow>('load_soundfont', { path });
export const setSoundfont = (id: number) => invoke('set_soundfont', { id });
export const listSoundfonts = () => invoke<SoundfontRow[]>('list_soundfonts');
export const listRecent = () => invoke<LibraryRow[]>('list_recent');
export const getSettings = () => invoke<Setting[]>('get_settings');
export const setSetting = (key: string, value: string) => invoke('set_setting', { key, value });

export const listenPlayhead = (cb: (p: Playhead) => void): Promise<UnlistenFn> =>
  listen<Playhead>('playhead', (e) => cb(e.payload));

export async function pickMidi(): Promise<string | null> {
  const r = await open({ multiple: false, filters: [{ name: 'MIDI', extensions: ['mid', 'midi'] }] });
  return typeof r === 'string' ? r : null;
}
export async function pickSoundfont(): Promise<string | null> {
  const r = await open({ multiple: false, filters: [{ name: 'SoundFont', extensions: ['sf2'] }] });
  return typeof r === 'string' ? r : null;
}
```

- [ ] **Step 3: Replace `src/App.svelte` with a minimal harness**

```svelte
<script lang="ts">
  import * as ipc from './lib/ipc';
  import type { MidiData, Playhead } from './lib/types';

  let midi = $state<MidiData | null>(null);
  let head = $state<Playhead | null>(null);

  $effect(() => {
    let un: (() => void) | undefined;
    ipc.listenPlayhead((p) => (head = p)).then((f) => (un = f));
    return () => un?.();
  });

  async function openFile() {
    const path = await ipc.pickMidi();
    if (!path) return;
    midi = await ipc.openMidi(path); // auto-plays
    console.log('loaded', midi.title, midi.notes.length, 'notes');
  }
</script>

<main>
  <button onclick={openFile}>Open MIDI</button>
  <button onclick={() => ipc.play()}>Play</button>
  <button onclick={() => ipc.pause()}>Pause</button>
  {#if midi}<p>{midi.title ?? 'Untitled'} — {midi.notes.length} notes — {midi.duration_sec.toFixed(1)}s</p>{/if}
  {#if head}<p>t={head.time.toFixed(2)} / {head.duration.toFixed(2)} · level={head.level.toFixed(3)} · {head.playing ? '▶' : '⏸'}</p>{/if}
</main>
```

- [ ] **Step 4: Smoke test — first sound (manual)**

Run: `npm run tauri dev`. Click **Open MIDI**, pick any `.mid` file (use one you have, or download a public-domain MIDI).
Expected: you **hear** the song play through GeneralUser GS; the `t=…` line advances ~60×/sec and `level` moves with the music; the console logs the note count. This verifies the entire backend pipeline (parse → synth → ring → cpal → emit) end to end.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(frontend): typed IPC client + minimal playback harness (first sound)"
```

---

## Task 7: Visualization engine + cosmic-aurora (`visualizations/*`, `VizCanvas.svelte`)

**Files:**
- Create: `src/lib/visualizations/types.ts`, `src/lib/visualizations/cosmic-aurora.ts`, `src/lib/VizCanvas.svelte`
- Modify: `src/App.svelte` (mount the canvas full-bleed behind the controls)

**Interfaces:**
- Consumes: `MidiData`, `Playhead` from `types.ts`.
- Produces:
  - `visualizations/types.ts`: `NoteEvent`, `Visualization` (the plugin contract)
  - `cosmic-aurora.ts`: `createCosmicAurora(): Visualization`
  - `VizCanvas.svelte`: a `<canvas>` host with props `{ midi: MidiData | null, head: Playhead | null }` that runs the RAF loop and DERIVES note-on/off from `(timeline + playhead)`.

- [ ] **Step 1: Write the visualization contract `src/lib/visualizations/types.ts`**

```ts
export interface NoteEvent { track: number; channel: number; note: number; velocity: number; }

// The plugin seam: future user visualizations implement this same interface.
export interface Visualization {
  id: string;
  name: string;
  setup(canvas: HTMLCanvasElement): void;
  onNoteOn(note: NoteEvent): void;
  onNoteOff(note: NoteEvent): void;
  onFrame(playhead: number, level: number, bands: number[]): void;
  resize(width: number, height: number): void;
  teardown(): void;
}
```

- [ ] **Step 2: Write `src/lib/visualizations/cosmic-aurora.ts`**

```ts
import type { Visualization, NoteEvent } from './types';

interface Orb { x: number; y: number; vx: number; vy: number; r: number; life: number; maxLife: number; hue: number; }

const TRACK_HUES = [190, 280, 320, 50, 150, 220, 0, 100];

export function createCosmicAurora(): Visualization {
  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let w = 0, h = 0, dpr = 1;
  let orbs: Orb[] = [];
  let t = 0;
  let smoothLevel = 0;
  let bandsSmooth: number[] = [];

  const pitchToX = (note: number) => {
    const lo = 21, hi = 108;
    const f = Math.max(0, Math.min(1, (note - lo) / (hi - lo)));
    return f * w;
  };

  return {
    id: 'cosmic-aurora',
    name: 'Cosmic Aurora',
    setup(c) { canvas = c; ctx = c.getContext('2d')!; },
    resize(width, height) {
      dpr = window.devicePixelRatio || 1;
      w = width; h = height;
      canvas.width = Math.floor(width * dpr);
      canvas.height = Math.floor(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    },
    onNoteOn(n: NoteEvent) {
      const hue = TRACK_HUES[n.track % TRACK_HUES.length];
      const vel = n.velocity / 127;
      orbs.push({
        x: pitchToX(n.note),
        y: h * (0.25 + 0.5 * (1 - n.note / 127)) + (Math.random() - 0.5) * 40,
        vx: (Math.random() - 0.5) * 12,
        vy: -8 - vel * 22,
        r: 6 + vel * 26,
        life: 0,
        maxLife: 1.1 + vel * 1.4,
        hue,
      });
      if (orbs.length > 600) orbs.splice(0, orbs.length - 600);
    },
    onNoteOff() {},
    onFrame(_playhead, level, bands) {
      t += 1 / 60;
      smoothLevel += (level - smoothLevel) * 0.2;
      if (bandsSmooth.length !== bands.length) bandsSmooth = bands.slice();
      for (let i = 0; i < bands.length; i++) bandsSmooth[i] += (bands[i] - bandsSmooth[i]) * 0.25;

      // Trail (slight persistence => motion blur).
      ctx.globalCompositeOperation = 'source-over';
      ctx.fillStyle = 'rgba(5, 4, 12, 0.28)';
      ctx.fillRect(0, 0, w, h);

      // Aurora ribbons (additive).
      ctx.globalCompositeOperation = 'lighter';
      for (let r = 0; r < 3; r++) {
        const baseHue = 170 + r * 50 + Math.sin(t * 0.1 + r) * 20;
        const amp = h * (0.06 + 0.10 * (bandsSmooth[r * 3] ?? smoothLevel));
        const yBase = h * (0.30 + r * 0.14);
        ctx.beginPath();
        for (let x = 0; x <= w; x += 12) {
          const y = yBase
            + Math.sin(x * 0.006 + t * (0.4 + r * 0.2)) * amp
            + Math.sin(x * 0.013 - t * 0.7) * amp * 0.5;
          x === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
        }
        const grad = ctx.createLinearGradient(0, yBase - amp, 0, yBase + amp);
        grad.addColorStop(0, `hsla(${baseHue}, 90%, 65%, 0)`);
        grad.addColorStop(0.5, `hsla(${baseHue}, 90%, 65%, ${0.10 + 0.25 * smoothLevel})`);
        grad.addColorStop(1, `hsla(${baseHue}, 90%, 65%, 0)`);
        ctx.strokeStyle = grad;
        ctx.lineWidth = 26 + 60 * smoothLevel;
        ctx.stroke();
      }

      // Note orbs (additive glow + bright core).
      for (let i = orbs.length - 1; i >= 0; i--) {
        const o = orbs[i];
        o.life += 1 / 60;
        if (o.life > o.maxLife) { orbs.splice(i, 1); continue; }
        o.x += o.vx / 60;
        o.y += o.vy / 60;
        o.vy += 6 / 60;
        const k = 1 - o.life / o.maxLife;
        const rr = o.r * (0.6 + 0.4 * k) * (1 + 0.6 * smoothLevel);
        const g = ctx.createRadialGradient(o.x, o.y, 0, o.x, o.y, rr * 3);
        g.addColorStop(0, `hsla(${o.hue}, 100%, 75%, ${0.9 * k})`);
        g.addColorStop(0.3, `hsla(${o.hue}, 100%, 60%, ${0.35 * k})`);
        g.addColorStop(1, `hsla(${o.hue}, 100%, 50%, 0)`);
        ctx.fillStyle = g;
        ctx.beginPath();
        ctx.arc(o.x, o.y, rr * 3, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = `hsla(${o.hue}, 100%, 92%, ${k})`;
        ctx.beginPath();
        ctx.arc(o.x, o.y, rr * 0.5, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.globalCompositeOperation = 'source-over';
    },
    teardown() { orbs = []; },
  };
}
```

- [ ] **Step 3: Write `src/lib/VizCanvas.svelte`**

```svelte
<script lang="ts">
  import { createCosmicAurora } from './visualizations/cosmic-aurora';
  import type { MidiData, Playhead } from './types';

  let { midi, head }: { midi: MidiData | null; head: Playhead | null } = $props();

  let host: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  const viz = createCosmicAurora();
  let raf = 0;
  let ptr = 0;
  let active: { idx: number; end: number }[] = [];
  let lastTime = 0;

  function resetSchedule() { ptr = 0; active = []; lastTime = 0; }

  // Reset the note scheduler whenever a new song loads.
  $effect(() => { midi; resetSchedule(); });

  $effect(() => {
    viz.setup(canvas);
    const ro = new ResizeObserver(() => viz.resize(host.clientWidth, host.clientHeight));
    ro.observe(host);
    viz.resize(host.clientWidth, host.clientHeight);

    const loop = () => {
      // Read the latest props each frame (these reads are NOT tracked as effect deps).
      const m = midi;
      const time = head?.time ?? 0;
      const level = head?.level ?? 0;
      const bands = head?.bands ?? [];

      if (m) {
        if (time < lastTime - 0.05) resetSchedule(); // seek / restart => re-derive
        const notes = m.notes;
        while (ptr < notes.length && notes[ptr].start_sec <= time) {
          const n = notes[ptr];
          viz.onNoteOn({ track: n.track, channel: n.channel, note: n.note, velocity: n.velocity });
          active.push({ idx: ptr, end: n.start_sec + n.dur_sec });
          ptr++;
        }
        for (let i = active.length - 1; i >= 0; i--) {
          if (active[i].end <= time) {
            const n = notes[active[i].idx];
            viz.onNoteOff({ track: n.track, channel: n.channel, note: n.note, velocity: n.velocity });
            active.splice(i, 1);
          }
        }
      }
      lastTime = time;
      viz.onFrame(time, level, bands);
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => { cancelAnimationFrame(raf); ro.disconnect(); viz.teardown(); };
  });
</script>

<div class="viz" bind:this={host}><canvas bind:this={canvas}></canvas></div>

<style>
  .viz { position: absolute; inset: 0; overflow: hidden; background: #05040c; }
  canvas { display: block; width: 100%; height: 100%; }
</style>
```

If the setup `$effect` re-runs on every `head` change (it must NOT), wrap the prop reads in the loop with Svelte's `untrack` — but because they are read inside `loop` (called later via RAF), they are not synchronous deps and the effect runs once. Verify in Step 5 that it does not re-create the loop each frame.

- [ ] **Step 4: Mount the canvas in `src/App.svelte`**

```svelte
<script lang="ts">
  import * as ipc from './lib/ipc';
  import VizCanvas from './lib/VizCanvas.svelte';
  import type { MidiData, Playhead } from './lib/types';

  let midi = $state<MidiData | null>(null);
  let head = $state<Playhead | null>(null);

  $effect(() => {
    let un: (() => void) | undefined;
    ipc.listenPlayhead((p) => (head = p)).then((f) => (un = f));
    return () => un?.();
  });

  async function openFile() {
    const path = await ipc.pickMidi();
    if (!path) return;
    midi = await ipc.openMidi(path);
  }
</script>

<VizCanvas {midi} {head} />
<div class="controls">
  <button onclick={openFile}>Open MIDI</button>
  <button onclick={() => ipc.play()}>Play</button>
  <button onclick={() => ipc.pause()}>Pause</button>
  {#if midi}<span>{midi.title ?? 'Untitled'} · {head ? head.time.toFixed(1) : '0.0'}s</span>{/if}
</div>

<style>
  :global(body) { margin: 0; overflow: hidden; background: #05040c; color: #dfe4ff; font-family: ui-sans-serif, system-ui, sans-serif; }
  .controls { position: fixed; left: 16px; bottom: 16px; display: flex; gap: 8px; align-items: center; z-index: 10; }
  button { background: #15102e; color: #cdd6ff; border: 1px solid #3a2f66; border-radius: 10px; padding: 8px 14px; cursor: pointer; }
  button:hover { background: #1e1640; }
  span { opacity: 0.8; }
</style>
```

- [ ] **Step 5: Magic check (manual)**

Run: `npm run tauri dev`. Open a busy `.mid` and press play.
Expected: the screen fills with the deep-space aurora; **glowing note orbs bloom on each note**, drift upward, and fade; aurora ribbons breathe with the music's loudness. Open DevTools and confirm the RAF loop is NOT being re-created every frame (no repeated "setup" — add a temporary `console.log('setup')` in the effect if unsure, then remove it).

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(viz): cosmic-aurora visualization + canvas host deriving note events from the timeline"
```

---

## Task 8: Theming + full UI (transport, panels) + startup persistence

**Files:**
- Create: `src/lib/theme.ts`, `src/app.css`, `src/lib/Transport.svelte`, `src/lib/TrackList.svelte`, `src/lib/SoundfontPicker.svelte`, `src/lib/ThemePicker.svelte`
- Modify: `src/main.ts` (import `./app.css`), `src/App.svelte` (full layout + startup reads)

**Interfaces:**
- Consumes: all of `ipc.ts`, `types.ts`, `VizCanvas.svelte`.
- Produces: `theme.ts` → `interface Theme`, `THEMES: Theme[]`, `applyTheme(id: string): string`. Components above. Final `App.svelte` layout.
- **Scope note (verified constraint):** per-track **mute/solo is deferred** — `MidiFileSequencer` owns the synth immutably and plays all channels, so live muting needs the future manual scheduler. `TrackList` is a structure view (name · color · note count). Seek is fast-forward (rustysynth has no seek). Both recorded in `DECISIONS.md` (Task 10).

- [ ] **Step 1: `src/lib/theme.ts`**

```ts
export interface Theme { id: string; name: string; vars: Record<string, string>; }

export const THEMES: Theme[] = [
  { id: 'cosmic', name: 'Cosmic', vars: { '--bg':'#05040c','--surface':'#0b0922cc','--border':'#2a2350','--text':'#dfe4ff','--muted':'#8b86b8','--accent':'#19f0c8','--accent2':'#d36bff' } },
  { id: 'nebula', name: 'Nebula Rose', vars: { '--bg':'#0a0510','--surface':'#1a0f24cc','--border':'#43275c','--text':'#ffe9f6','--muted':'#b58fb0','--accent':'#ff6bd0','--accent2':'#8a6bff' } },
  { id: 'abyss', name: 'Abyss', vars: { '--bg':'#02060a','--surface':'#06131ccc','--border':'#143041','--text':'#d6f7ff','--muted':'#6f97a6','--accent':'#2bd6ff','--accent2':'#19f0c8' } },
];

export function applyTheme(id: string): string {
  const t = THEMES.find((x) => x.id === id) ?? THEMES[0];
  for (const [k, v] of Object.entries(t.vars)) document.documentElement.style.setProperty(k, v);
  return t.id;
}
```

- [ ] **Step 2: `src/app.css` (global chrome) + import it**

```css
:root { --bg:#05040c; --surface:#0b0922cc; --border:#2a2350; --text:#dfe4ff; --muted:#8b86b8; --accent:#19f0c8; --accent2:#d36bff; }
* { box-sizing: border-box; }
body { margin: 0; overflow: hidden; background: var(--bg); color: var(--text); font-family: ui-sans-serif, system-ui, -apple-system, sans-serif; }
.panel { background: var(--surface); backdrop-filter: blur(12px); border: 1px solid var(--border); border-radius: 14px; padding: 12px 14px; }
.panel h3 { margin: 0 0 8px; font-size: 12px; text-transform: uppercase; letter-spacing: 1px; color: var(--muted); font-weight: 600; }
button { font: inherit; color: var(--text); background: var(--surface); border: 1px solid var(--border); border-radius: 10px; padding: 7px 12px; cursor: pointer; transition: background .15s, border-color .15s; }
button:hover { border-color: var(--accent); }
.muted { color: var(--muted); font-size: 12px; }
input[type=range] { accent-color: var(--accent); }
```

In `src/main.ts`, add `import './app.css';` at the top (keep the existing `mount(App, …)`).

- [ ] **Step 3: `src/lib/Transport.svelte`**

```svelte
<script lang="ts">
  import * as ipc from './ipc';
  import type { MidiData, Playhead } from './types';
  let { midi, head }: { midi: MidiData | null; head: Playhead | null } = $props();

  let seeking = $state(false);
  let seekPos = $state(0);
  let tempo = $state(1);
  let volume = $state(1);

  const dur = $derived(head?.duration || midi?.duration_sec || 0);
  const pos = $derived(seeking ? seekPos : (head?.time ?? 0));
  const playing = $derived(head?.playing ?? false);
  const fmt = (s: number) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;
</script>

<div class="bar">
  <button class="play" onclick={() => (playing ? ipc.pause() : ipc.play())} disabled={!midi}>{playing ? '⏸' : '▶'}</button>
  <span class="t">{fmt(pos)}</span>
  <input class="seek" type="range" min="0" max={dur || 1} step="0.01" value={pos}
    disabled={!midi}
    oninput={(e) => { seeking = true; seekPos = +e.currentTarget.value; }}
    onchange={(e) => { ipc.seek(+e.currentTarget.value); seeking = false; }} />
  <span class="t">{fmt(dur)}</span>
  <label>speed <input type="range" min="0.5" max="2" step="0.05" bind:value={tempo} onchange={() => ipc.setTempo(tempo)} /> {tempo.toFixed(2)}×</label>
  <label>vol <input type="range" min="0" max="1.5" step="0.01" bind:value={volume} onchange={() => ipc.setVolume(volume)} /></label>
</div>

<style>
  .bar { position: fixed; left: 16px; right: 16px; bottom: 16px; z-index: 10;
    display: flex; align-items: center; gap: 12px; padding: 10px 16px;
    background: var(--surface); backdrop-filter: blur(12px); border: 1px solid var(--border); border-radius: 16px; }
  .play { width: 42px; height: 42px; border-radius: 50%; font-size: 16px; }
  .seek { flex: 1; }
  .t { font-variant-numeric: tabular-nums; color: var(--muted); font-size: 12px; min-width: 36px; }
  label { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--muted); }
  label input { width: 84px; }
</style>
```

- [ ] **Step 4: `src/lib/TrackList.svelte`**

```svelte
<script lang="ts">
  import type { MidiData } from './types';
  let { midi }: { midi: MidiData | null } = $props();
  const HUES = [190, 280, 320, 50, 150, 220, 0, 100];
  const counts = $derived.by(() => {
    const c: Record<number, number> = {};
    midi?.notes.forEach((n) => (c[n.track] = (c[n.track] ?? 0) + 1));
    return c;
  });
</script>

{#if midi}
<div class="panel">
  <h3>Tracks</h3>
  {#each midi.tracks as tr}
    {#if (counts[tr.index] ?? 0) > 0}
      <div class="row">
        <span class="dot" style="background: hsl({HUES[tr.index % HUES.length]} 90% 62%)"></span>
        <span class="name">{tr.name ?? `Track ${tr.index + 1}`}</span>
        <span class="count">{counts[tr.index]}</span>
      </div>
    {/if}
  {/each}
</div>
{/if}

<style>
  .row { display: flex; align-items: center; gap: 8px; padding: 3px 0; font-size: 13px; }
  .dot { width: 10px; height: 10px; border-radius: 50%; box-shadow: 0 0 8px currentColor; flex: none; }
  .name { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .count { color: var(--muted); font-size: 11px; font-variant-numeric: tabular-nums; }
</style>
```

- [ ] **Step 5: `src/lib/SoundfontPicker.svelte`**

```svelte
<script lang="ts">
  import * as ipc from './ipc';
  import type { SoundfontRow } from './types';
  let fonts = $state<SoundfontRow[]>([]);
  let current = $state<number | null>(null);

  async function refresh() {
    fonts = await ipc.listSoundfonts();
    if (current == null && fonts[0]) current = fonts[0].id;
  }
  $effect(() => { refresh(); });

  async function choose(id: number) { current = id; await ipc.setSoundfont(id); }
  async function add() {
    const p = await ipc.pickSoundfont();
    if (!p) return;
    const row = await ipc.loadSoundfont(p);
    await refresh();
    current = row.id;
  }
</script>

<div class="panel">
  <h3>SoundFont</h3>
  <select value={current} onchange={(e) => choose(+e.currentTarget.value)}>
    {#each fonts as f}<option value={f.id}>{f.name}{f.is_builtin ? ' ★' : ''}</option>{/each}
  </select>
  <button onclick={add}>Load .sf2…</button>
</div>

<style>
  select { width: 100%; margin-bottom: 8px; background: var(--bg); color: var(--text); border: 1px solid var(--border); border-radius: 8px; padding: 6px; }
  button { width: 100%; }
</style>
```

- [ ] **Step 6: `src/lib/ThemePicker.svelte`**

```svelte
<script lang="ts">
  import { THEMES, applyTheme } from './theme';
  import * as ipc from './ipc';
  let { current = $bindable('cosmic') }: { current?: string } = $props();
  async function choose(id: string) { current = applyTheme(id); await ipc.setSetting('theme', id); }
</script>

<div class="panel">
  <h3>Theme</h3>
  <div class="row">
    {#each THEMES as t}
      <button class:active={current === t.id} onclick={() => choose(t.id)}>{t.name}</button>
    {/each}
  </div>
</div>

<style>
  .row { display: flex; flex-wrap: wrap; gap: 6px; }
  button { font-size: 12px; padding: 6px 10px; }
  button.active { border-color: var(--accent); color: var(--accent); }
</style>
```

- [ ] **Step 7: Final `src/App.svelte` (layout + startup persistence)**

```svelte
<script lang="ts">
  import * as ipc from './lib/ipc';
  import VizCanvas from './lib/VizCanvas.svelte';
  import Transport from './lib/Transport.svelte';
  import TrackList from './lib/TrackList.svelte';
  import SoundfontPicker from './lib/SoundfontPicker.svelte';
  import ThemePicker from './lib/ThemePicker.svelte';
  import { applyTheme } from './lib/theme';
  import type { MidiData, Playhead, LibraryRow } from './lib/types';

  let midi = $state<MidiData | null>(null);
  let head = $state<Playhead | null>(null);
  let recent = $state<LibraryRow[]>([]);
  let theme = $state('cosmic');
  let panelsOpen = $state(true);

  $effect(() => {
    let un: (() => void) | undefined;
    ipc.listenPlayhead((p) => (head = p)).then((f) => (un = f));
    return () => un?.();
  });

  // Startup: restore theme + recent list.
  $effect(() => {
    (async () => {
      const settings = await ipc.getSettings();
      theme = applyTheme(settings.find((s) => s.key === 'theme')?.value ?? 'cosmic');
      recent = await ipc.listRecent();
    })();
  });

  async function openPath(path: string) { midi = await ipc.openMidi(path); recent = await ipc.listRecent(); }
  async function openFile() { const p = await ipc.pickMidi(); if (p) await openPath(p); }
</script>

<VizCanvas {midi} {head} />

<header class="topbar">
  <div class="brand"><span class="bdot"></span> midimi</div>
  <div class="now">{midi?.title ?? (midi ? 'Untitled' : 'Open a MIDI to begin')}</div>
  <button class="ghost" onclick={() => (panelsOpen = !panelsOpen)} title="Toggle panels">{panelsOpen ? '⟩' : '⟨'}</button>
</header>

{#if panelsOpen}
<aside class="panels">
  <button class="open" onclick={openFile}>＋ Open MIDI…</button>
  <TrackList {midi} />
  <SoundfontPicker />
  <ThemePicker bind:current={theme} />
  <div class="panel">
    <h3>Recent</h3>
    {#each recent as r}
      <button class="recent" onclick={() => openPath(r.path)}>{r.title ?? r.path.split('/').pop()}</button>
    {/each}
    {#if recent.length === 0}<p class="muted">Nothing yet.</p>{/if}
  </div>
</aside>
{/if}

<Transport {midi} {head} />

<style>
  .topbar { position: fixed; top: 14px; left: 16px; right: 16px; z-index: 10; display: flex; align-items: center; gap: 14px; pointer-events: none; }
  .brand { display: flex; align-items: center; gap: 8px; font-weight: 600; letter-spacing: 1px; font-size: 18px; }
  .bdot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 12px var(--accent); }
  .now { flex: 1; color: var(--muted); font-size: 13px; }
  .ghost { pointer-events: auto; background: transparent; border: none; color: var(--muted); }
  .panels { position: fixed; top: 56px; right: 16px; bottom: 84px; width: 250px; z-index: 10;
    display: flex; flex-direction: column; gap: 12px; overflow-y: auto; }
  .open { width: 100%; padding: 10px; font-weight: 600; }
  .recent { width: 100%; text-align: left; margin-bottom: 4px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 12px; }
</style>
```

- [ ] **Step 8: Verify the full UI + persistence (manual)**

Run: `npm run tauri dev`.
Expected: dark cosmic UI; open a MIDI → it plays, aurora reacts, transport works (play/pause, scrub seeks, speed + volume sliders respond), track list shows colored tracks with note counts, soundfont dropdown swaps the sound, theme buttons restyle the chrome instantly. **Close and reopen the app** → the chosen theme persists and the file appears under **Recent** (click it to reopen). `npm run check` (svelte-check) passes with no type errors.

- [ ] **Step 9: Commit**

```bash
git add -A && git commit -m "feat(ui): theming + transport + panels + recent; restore theme/recent on startup"
```

---

## Task 9: Acceptance pass + optional demo track

**Files:**
- Optional create: `scripts/make-demo.mjs`, `assets/demo/scale.mid` (+ bundle as resource), first-run auto-open in `src/App.svelte`

**Interfaces:**
- Consumes: the whole app. Produces: a verified v1 + (optional) first-run delight.

- [ ] **Step 1: Run the full test + type suite**

Run: `cd src-tauri && cargo test && cd .. && npm run check`
Expected: all Rust tests PASS; svelte-check reports 0 errors.

- [ ] **Step 2: Walk the v1 acceptance checklist (manual)**

With `npm run tauri dev`, confirm each vertical-slice criterion:
1. Drag-drop OR file-picker opens a `.mid`. (File-picker is wired; drag-drop is a nice-to-have — if not added, note it.)
2. It plays through the bundled GeneralUser GS soundfont.
3. The cosmic/aurora visualization reacts in real time.
4. Transport: play/pause, seek (fast-forward), speed, volume all work.
5. Track list shows tracks (name · color · count).
6. SoundFont picker swaps instruments; Theme picker restyles.
7. Recent files + theme persist across restarts.

Fix any criterion that fails before proceeding. Each fix is its own small commit.

- [ ] **Step 3 (optional): Bundle a public-domain demo track + first-run auto-open**

`scripts/make-demo.mjs` (no deps — writes a C-major scale, a public-domain melody):

```js
import { writeFileSync, mkdirSync } from 'node:fs';
mkdirSync('assets/demo', { recursive: true });
const tpqn = 96, notes = [60,62,64,65,67,69,71,72];
const tb = []; // track bytes
const push = (...b) => tb.push(...b);
const vlq = (n) => (n < 128 ? [n] : [0x80 | (n >> 7), n & 0x7f]); // n < 16384 here
for (const n of notes) { push(0x00, 0x90, n, 0x64); push(...vlq(tpqn), 0x80, n, 0x00); }
push(0x00, 0xff, 0x2f, 0x00);
const len = tb.length;
const header = [0x4d,0x54,0x68,0x64, 0,0,0,6, 0,0, 0,1, (tpqn>>8)&0xff, tpqn&0xff];
const trk = [0x4d,0x54,0x72,0x6b, (len>>24)&0xff,(len>>16)&0xff,(len>>8)&0xff,len&0xff, ...tb];
writeFileSync('assets/demo/scale.mid', Buffer.from([...header, ...trk]));
console.log('wrote assets/demo/scale.mid');
```

Run `node scripts/make-demo.mjs`. Add `"../assets/demo/scale.mid": "demo/scale.mid"` to `tauri.conf.json` `bundle.resources`. Add a backend command `demo_path() -> String` returning the resolved resource path (mirror `builtin_sf_path` resolution), expose it in `ipc.ts` as `demoPath()`, and in `App.svelte`'s startup effect, if `recent.length === 0`, `await openPath(await ipc.demoPath())`. This gives first-run users instant sound + visuals.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "test: v1 acceptance pass" # + "feat: bundled demo track + first-run auto-open" if Step 3 done
```

---

## Task 10: Documentation + decisions log + CI finalize

**Files:**
- Modify: `docs/ARCHITECTURE.md`, `docs/DECISIONS.md`, `README.md`, `.github/workflows/build.yml`

**Interfaces:** Consumes the finished app. Produces the living docs the project requires.

- [ ] **Step 1: `docs/ARCHITECTURE.md`**

Write the real architecture: the data flow (parse → render thread → ring buffer → cpal; 60 Hz snapshot → `playhead` event → canvas), the module map (`midi`/`analysis`/`audio`/`db`/`commands`/`lib` and the frontend `ipc`/`VizCanvas`/`visualizations`), the single-clock principle, and the Visualization plugin contract (with the `setup/onNoteOn/onNoteOff/onFrame/resize/teardown` signature) as the extension seam.

- [ ] **Step 2: `docs/DECISIONS.md` (ADR log)**

Record, each as a short ADR (context → decision → consequence), at minimum:
1. **Turso (the Rust rewrite) embedded, cache-only** — beta engine acceptable because the filesystem is the source of truth.
2. **Rust-side synthesis (rustysynth) over Web Audio** — makes offline export trivial and gives authoritative timing.
3. **Render thread + rtrb ring buffer** — never lock the synth in the cpal callback (glitch-free audio).
4. **Seek = replay + fast-forward** — rustysynth 1.3.6 has no seek API; documented O(target) cost.
5. **Mute/solo deferred to a manual scheduler (v1.1)** — `MidiFileSequencer` owns the synth immutably and plays all channels, so live per-track control isn't possible on this path.
6. **GeneralUser GS bundled** — permissive, Apache-2-compatible; fetched by script, bundled as a resource, license vendored.
7. **Svelte 5 + raw Canvas 2D** — minimal runtime, no VDOM fighting the animation loop; WebGL is the documented upgrade path.

- [ ] **Step 3: `README.md` finalize**

Ensure it covers: what midimi is, screenshot placeholder, prerequisites (Rust stable, Node 20+), **`./scripts/fetch-soundfont.sh`**, `npm install`, `npm run tauri dev`, `npm run tauri build`, the Apache-2.0 license, and a **Third-party assets** section crediting *GeneralUser GS by S. Christian Collins* with a pointer to `assets/soundfonts/GeneralUser-GS-LICENSE.txt`.

- [ ] **Step 4: Confirm CI builds locally where possible**

Run: `npm run tauri build` (debug or release) on your machine to confirm the bundle includes the soundfont resource and launches. The `.github/workflows/build.yml` from Task 0 stays inert until a remote is added; sanity-check its YAML with `yamllint` or by eye.

- [ ] **Step 5: Commit + tag v1**

```bash
git add -A && git commit -m "docs: architecture, decision log, README; finalize CI"
git tag -a v0.1.0 -m "midimi v1 vertical slice"
```

---

## Self-Review

Reviewed the plan against `docs/superpowers/specs/2026-06-26-midimi-design.md` with fresh eyes:

**1. Spec coverage.**
- Open/play MIDI (T1, T4–T6) ✓ · swappable soundfonts (T5, T8) ✓ · cosmic/aurora viz (T7) ✓ · transport play/pause/seek/tempo/volume (T8) ✓ (seek is fast-forward — verified `rustysynth` limitation) · soundfont + theme pickers (T8) ✓ · persistence: recent/soundfonts/settings (T3, T5, T8) ✓ · Visualization plugin seam (T7) ✓ · Apache-2.0 + soundfont license (T0) ✓ · CI (T0, T10) ✓ · living docs (T10) ✓.
- **GAP — per-track mute/solo:** the spec's §4 item 5 says "mute, solo." Verification proved this is unachievable on the `MidiFileSequencer` path (it owns the synth immutably, plays all channels). The plan **defers functional mute/solo to v1.1** (needs a manual `process_midi_message` scheduler) and ships the track list as a structure view. **Recommended:** update spec §4 to mark mute/solo as v1.1, or add a manual-scheduler task to v1. Flagged for the owner.

**2. Placeholder scan.** No `TBD`/`TODO`/"add error handling"/"similar to Task N" — every code step contains complete, runnable code or exact commands. Deferred features (export, plugin loading, mute/solo) are explicitly out-of-scope with rationale, not silent gaps.

**3. Type consistency (checked across tasks).**
- `playhead` JSON `{time,duration,level,bands,playing}` (Rust, T5) ≡ `Playhead` (TS, T6) ✓.
- `MidiData{title,duration_sec,tracks,notes}` / `Note{track,channel,note,start_sec,dur_sec,velocity}` identical Rust↔TS ✓.
- `SoundfontRow{id,path,name,is_builtin}`, `LibraryRow{id,path,title,duration_sec}`, `Setting{key,value}` identical ✓.
- Command names + arg keys match `ipc.ts` wrappers exactly: `open_midi(path)`, `seek(seconds)`, `set_tempo(ratio)`, `set_volume(volume)`, `set_soundfont(id)`, `set_setting(key,value)`, `load_soundfont(path)` ✓.
- `AudioEngine` method names (`load/play/pause/seek/set_speed/set_volume/shared`) consistent across T4 (def) and T5 (use) ✓.
- `BandAnalyzer::new/analyze`, `rms`, `render_offline`, `parse_midi`, `MidiData` — all referenced with the signatures they were defined with ✓.

No issues requiring inline fixes beyond the documented mute/solo deferral, which is a spec decision for the owner, not a plan defect.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-27-midimi-v1-vertical-slice.md`. Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration (REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`).
2. **Inline Execution** — execute tasks in this session with checkpoints for review (REQUIRED SUB-SKILL: `superpowers:executing-plans`).
