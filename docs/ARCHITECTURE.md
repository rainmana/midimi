# Architecture

midimi is a native desktop MIDI player built with **Tauri v2** (Rust backend) and **SvelteKit + Svelte 5** (frontend SPA). This document describes the real data flows and module responsibilities as implemented in v1.

---

## Process model

```
┌─────────────────────────────────────────────────────────┐
│  WebView  (SvelteKit SPA, adapter-static, SSR off)      │
│  src/routes/+page.svelte · src/lib/{ipc,VizCanvas,…}   │
└───────────────┬──────────────────────────────┬──────────┘
                │  invoke (IPC)                │  event ("playhead")
┌───────────────▼──────────────────────────────▼──────────┐
│  Tauri core (Rust, src-tauri/src/)                       │
│  lib.rs — setup + 60 Hz emit loop                        │
│  commands.rs — #[tauri::command] handlers                │
│  midi.rs — parse_midi (note timeline)                    │
│  audio.rs — AudioEngine + render thread + rtrb buffer    │
│  analysis.rs — RMS + FFT band analyzer                   │
│  db.rs — Turso (embedded SQLite) cache                   │
└──────────────────────────────────────────────────────────┘
```

---

## Data flow: open → audio → canvas

### 1. MIDI parse → note timeline

`commands::open_midi` reads the file bytes and calls `midi::parse_midi`. The parser (`midly` crate) does two passes:

1. **Pass 1** — build a global tempo map (absolute tick → µs/quarter) across all tracks.
2. **Pass 2** — convert every NoteOn/NoteOff pair into a `Note { track, channel, note, start_sec, dur_sec, velocity }` using the tempo map. Track names are extracted from `TrackName` meta events.

The result is `MidiData { title, duration_sec, tracks: Vec<TrackInfo>, notes: Vec<Note> }`. This is serialized over IPC and stored in the frontend as the note timeline.

Simultaneously, `commands::open_midi` constructs a `rustysynth::MidiFile` (the sequencer's own parse) and sends a `Cmd::Load` message to the render thread.

### 2. Render thread → ring buffer → cpal audio output

`AudioEngine::new()` sets up three concurrent pieces:

- **cpal stream** (real-time thread) — calls `consumer.pop()` in its callback; underruns produce silence. The callback never locks, allocates, or blocks.
- **rtrb ring buffer** — lock-free single-producer single-consumer channel between the render thread and the cpal callback. The buffer holds 8 × 1024 stereo frames.
- **render thread** — a plain `std::thread` that owns the `MidiFileSequencer` (rustysynth) and the `BandAnalyzer`. It:
  1. Drains control commands from a `std::sync::mpsc::Receiver` (non-blocking `try_recv`).
  2. If `playing && ring buffer has room`, calls `sequencer.render(&mut left, &mut right)` for 1 024 frames, interleaves them into the ring buffer, runs the FFT analyzer, computes RMS, then writes a `Snapshot` into the shared `Arc<Mutex<Snapshot>>`.
  3. If idle, sleeps 2 ms to avoid busy-spinning.

Control commands (`Cmd::Load / Play / Pause / Seek / SetSpeed / SetVolume`) are sent from Tauri command handlers via the `mpsc::Sender` stored in `AudioEngine`.

### 3. Single-clock principle

`sequencer.get_position()` (rustysynth) is the **only** clock. It advances strictly with rendered samples and is written into `Snapshot::position_sec` each render block. The frontend reads this position via the `"playhead"` event — it does not maintain its own timer. This eliminates drift between audio and visualization.

### 4. 60 Hz playhead event → VizCanvas

A dedicated OS thread in `lib.rs::setup` wakes every 16 ms, locks the shared `Snapshot`, and emits a `"playhead"` Tauri event with:

```json
{ "time": f64, "duration": f64, "level": f32, "bands": [f32; 16], "playing": bool }
```

`src/lib/ipc.ts::listenPlayhead` subscribes to this event and updates `head: Playhead` state in `+page.svelte`, which flows as a reactive prop into `VizCanvas`.

### 5. VizCanvas: note scheduling + animation loop

`VizCanvas.svelte` runs a `requestAnimationFrame` loop. Each frame it:

1. Advances a note pointer (`ptr`) through `midi.notes` until `notes[ptr].start_sec > head.time`, calling `viz.onNoteOn(ev)` for each newly active note.
2. Evicts expired notes from the `active` list, calling `viz.onNoteOff(ev)`.
3. Calls `viz.onFrame(head.time, head.level, head.bands)` to render the frame.

On seek detected as `time < lastTime − 0.05`, the scheduler resets: all active notes receive `onNoteOff` and `ptr` resets to 0.

---

## Module map

### Backend (`src-tauri/src/`)

| Module | Responsibility |
|---|---|
| `lib.rs` | Tauri app setup, resource resolution, DB init, soundfont registration, 60 Hz emit loop |
| `commands.rs` | All `#[tauri::command]` handlers; `AppState` struct holding `Mutex<AudioEngine>`, `turso::Connection`, and current MIDI/SF arcs |
| `midi.rs` | `parse_midi(bytes) → MidiData`; global tempo-map construction; two-pass note extraction |
| `audio.rs` | `AudioEngine` (command sender + shared snapshot + cpal stream + render thread join handle); `render_loop` (sequencer + ring buffer fill + analysis); `render_offline` (for future WAV/MP3 export) |
| `analysis.rs` | `BandAnalyzer` (Hann-windowed FFT, 16 log-spaced bands, 40 Hz – Nyquist); `rms()` |
| `db.rs` | Turso connection; `library`, `soundfonts`, `settings` tables; upsert/query helpers |

### Frontend (`src/`)

| Path | Responsibility |
|---|---|
| `src/routes/+page.svelte` | Root page; holds `midi`, `head`, `recent`, `theme` state; wires up playhead listener and startup effects |
| `src/lib/ipc.ts` | Typed wrappers for every `invoke` call and the `listenPlayhead` event subscription |
| `src/lib/types.ts` | Shared TS interfaces: `Note`, `TrackInfo`, `MidiData`, `Playhead`, `SoundfontRow`, `LibraryRow`, `Setting` |
| `src/lib/theme.ts` | Three built-in themes (Cosmic, Nebula Rose, Abyss); `applyTheme(id)` sets CSS custom properties |
| `src/lib/VizCanvas.svelte` | Canvas host; note scheduler; delegates rendering to the active `Visualization` plugin |
| `src/lib/Transport.svelte` | Play/pause/seek scrubber, tempo slider, volume slider |
| `src/lib/TrackList.svelte` | Track names display (structure view; mute/solo deferred to v1.1) |
| `src/lib/SoundfontPicker.svelte` | Load SF2 from disk or select a registered soundfont |
| `src/lib/ThemePicker.svelte` | Theme switcher; persists selection via `set_setting` |
| `src/lib/visualizations/types.ts` | `Visualization` interface (plugin contract) and `NoteEvent` type |
| `src/lib/visualizations/cosmic-aurora.ts` | Bundled "Cosmic Aurora" visualization: aurora ribbons + note orbs on Canvas 2D |

---

## Visualization plugin contract

Any object implementing `Visualization` (`src/lib/visualizations/types.ts`) can be dropped into `VizCanvas`:

```ts
interface Visualization {
  id: string;
  name: string;
  setup(canvas: HTMLCanvasElement): void;   // called once; acquire ctx here
  onNoteOn(note: NoteEvent): void;          // fired when sequencer clock reaches note.start_sec
  onNoteOff(note: NoteEvent): void;         // fired when clock passes note.start_sec + note.dur_sec
  onFrame(playhead: number, level: number, bands: number[]): void; // called every rAF (~60 Hz)
  resize(width: number, height: number): void;  // called on mount and on ResizeObserver
  teardown(): void;                         // called when VizCanvas unmounts; free GPU resources
}
```

`NoteEvent` carries `{ track, channel, note, velocity }`. The `note` field is a MIDI pitch number (21–108 covers standard piano range).

This is the extension seam: a future plugin loader will discover and instantiate user-supplied `Visualization` objects without any other code change. WebGL is the documented upgrade path for the first real external plugin.

---

## Analysis pipeline

Each 1 024-frame render block:

1. Left and right channels are averaged to mono.
2. A Hann window is applied.
3. A forward real FFT (rustfft, planned once at startup) produces 512 complex bins.
4. Bins are summed into 16 logarithmically-spaced bands (40 Hz – Nyquist). Each band value is normalized to ~0–1.
5. RMS of the left channel alone is the `level` scalar.

Both `bands` and `level` are written into the shared `Snapshot` and emitted with every `"playhead"` event.

---

## Persistence (Turso / embedded SQLite)

Three tables, schema created on startup:

| Table | Key | Purpose |
|---|---|---|
| `library` | `path UNIQUE` | Recently opened files (max 20 shown), ordered by `last_opened_at` |
| `soundfonts` | `path UNIQUE` | Registered soundfonts; `is_builtin=1` for GeneralUser GS |
| `settings` | `key PRIMARY KEY` | Key-value store; currently used for `theme` |

The database is a pure cache. The filesystem is the source of truth; the DB can be deleted and will be rebuilt on next open.
