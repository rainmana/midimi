# midimi — Design Spec

- **Status:** Approved for v1 planning
- **Date:** 2026-06-26
- **Owner:** w.alec.akin@gmail.com
- **License:** Apache-2.0 (application code); bundled soundfonts retain their own licenses

---

## 1. Vision

**midimi** is a dark-mode-native desktop app that lets *non-musicians* play, explore, and feel
MIDI music. You drag in a `.mid` file, it plays through rich soundfonts, and the screen comes
alive with a magical, music-reactive visualization. It is meant to feel less like a DAW and more
like a window into the music.

One sentence: **"A magical music box for MIDI."**

## 2. Goals & Non-Goals

### Goals (v1)
- Open and play any standard `.mid` / `.midi` file with zero musical knowledge required.
- Render audio in Rust using swappable SoundFont (`.sf2`) instruments — ship one, load more.
- A genuinely beautiful, real-time, audio-reactive **cosmic/aurora** visualization.
- Sleek, dark, customizable UI (themes via CSS variables).
- Persist the user's recent files, registered soundfonts, and settings locally.

### Explicit Non-Goals (v1 — seams cut, features deferred)
- MP3 / WAV / MusicXML export (designed, not built).
- Plugin *loading* (the visualization *contract* exists; loading third-party code does not).
- MIDI *editing*.
- Any cloud, account, telemetry, or network feature.
- Mobile (Tauri v2 leaves the door open; not a v1 target).

### Design tenets (ponytail)
1. Stdlib / native platform first (CSS variables for theming, filesystem as source of truth).
2. No speculative abstraction — build the seam only where v1 actually crosses it.
3. Pure-Rust, permissively-licensed dependencies preferred.
4. The shortest path that is also *correct on edge cases* wins.

## 3. Target User

Primary: a curious non-musician who has a MIDI file (a game rip, a classical piece, a meme song)
and wants to *hear it well and watch it*. They do not know what a "channel" or "program change"
is and should never need to. Secondary: hobbyists who want to audition soundfonts.

## 4. v1 Scope — The Vertical Slice

The single, fully-polished path:

1. **Open** a `.mid` (drag-drop, file picker, or a bundled demo track).
2. It **plays** through the bundled General MIDI soundfont; the user can **swap** to any loaded `.sf2`.
3. The **cosmic/aurora canvas** reacts in real time to the music.
4. **Transport**: play / pause / seek / tempo (playback-rate) / master volume.
5. **Track list**: per-track name, mute, solo.
6. **Soundfont picker** and **theme picker**.
7. **Persistence**: recent files, registered soundfonts, and settings survive restarts.

If a user can do those seven things and it feels magical, v1 is done.

## 5. Architecture

```
┌────────────────────────── Tauri v2 App ──────────────────────────┐
│                                                                   │
│  Frontend (WebView)              │   Backend (Rust core)          │
│  Svelte 5 + TS + Vite            │                                │
│  ┌────────────────────────┐      │   ┌─────────────────────────┐  │
│  │ UI shell (transport,    │ IPC  │   │ commands.rs (invoke)    │  │
│  │ track list, pickers)    │◀────▶│   │  open_midi/play/pause…  │  │
│  ├────────────────────────┤ events│  ├─────────────────────────┤  │
│  │ Visualization engine    │◀─────│   │ audio/  (synth+output)  │  │
│  │  <canvas> 2D, plugin    │ 60Hz │   │  midly → timeline       │  │
│  │  contract               │ feed │   │  rustysynth → samples   │  │
│  └────────────────────────┘      │   │  cpal → speakers        │  │
│                                  │   │  rustfft → bands        │  │
│                                  │   ├─────────────────────────┤  │
│                                  │   │ db/ (turso, local file) │  │
│                                  │   │ files/ (sf2 + midi scan)│  │
│                                  │   └─────────────────────────┘  │
└───────────────────────────────────────────────────────────────────┘
```

### 5.1 Audio pipeline (all Rust)
- **Parse:** `midly` reads the `.mid` → a flat **note timeline** (`{track, channel, note,
  start_sec, dur_sec, velocity}`) + metadata (title, tempo map, track names, duration).
- **Synthesize:** `rustysynth` loads the `.sf2` and the `MidiFile`; its `MidiFileSequencer`
  renders interleaved stereo `f32` samples on demand.
- **Output:** `cpal` opens the default output stream; its callback pulls samples from the
  sequencer.
- **Clock:** a single `AtomicU64` **playhead** (samples rendered) is the only source of time.
  Everything — UI position, visuals, seek — derives from it. Audio and visuals cannot drift.
- **Analysis (for visuals):** in/after the render callback, compute a cheap **RMS level** and an
  N-band magnitude vector via `rustfft` over the most recent window. Shipped with the playhead.

### 5.2 Frontend ↔ backend contract
- **Commands (frontend → Rust, `invoke`):** `open_midi(path)`, `play()`, `pause()`, `stop()`,
  `seek(sec)`, `set_tempo(ratio)`, `set_volume(v)`, `set_track_mute(i,bool)`,
  `set_track_solo(i,bool)`, `load_soundfont(path)`, `set_soundfont(id)`, `list_soundfonts()`,
  `list_recent()`, `get_settings()`, `set_setting(key,val)`.
- **On load:** `open_midi` returns the full note timeline + metadata (one transfer).
- **Events (Rust → frontend, ~60 Hz):** `playhead` event = `{ time_sec, level, bands[],
  is_playing }`. Discrete `note_on` / `note_off` events are derived frontend-side from
  (timeline + playhead) to avoid event spam; the contract still exposes them to visualizations.

### 5.3 Data model (local Turso)
A single local file `midimi.db` opened with `turso::Builder::new_local(path)`. **Cache only** —
every row is rebuildable by re-scanning the filesystem, so the beta engine carries no risk of
real data loss.

```sql
CREATE TABLE IF NOT EXISTS library (
  id INTEGER PRIMARY KEY,
  path TEXT UNIQUE NOT NULL,
  title TEXT,
  duration_sec REAL,
  last_opened_at INTEGER NOT NULL          -- unix seconds
);
CREATE TABLE IF NOT EXISTS soundfonts (
  id INTEGER PRIMARY KEY,
  path TEXT UNIQUE NOT NULL,
  name TEXT NOT NULL,
  is_builtin INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL                       -- JSON-encoded scalar
);
```

### 5.4 Visualization engine + plugin seam
The canvas is a single full-bleed `<canvas>` (2D context, additive `globalCompositeOperation =
'lighter'`). A **Visualization** is an object implementing one contract:

```ts
interface Visualization {
  id: string;
  name: string;
  setup(canvas: HTMLCanvasElement): void;
  onNoteOn(note: NoteEvent): void;
  onNoteOff(note: NoteEvent): void;
  onFrame(playhead: number, level: number, bands: number[]): void;
  resize(w: number, h: number): void;
  teardown(): void;
}
```

v1 ships **one** built-in visualization, `cosmic-aurora` (aurora backdrop + note orbs that bloom,
drift, and fade; glow rides `level`/`bands`). Future user plugins are JS modules in a `plugins/`
folder exporting the same contract; **loading them is deferred** (no SDK, no sandbox in v1).
WebGL/shader visuals are the documented upgrade path and the natural first *real* plugin.

### 5.5 Theming
Themes are sets of **CSS custom properties** (`--bg`, `--surface`, `--accent`, `--glow`, the
aurora ramp, etc.). The visualization reads the same vars so the canvas matches the chrome.
v1 ships a few cosmic variants; the active theme persists in `settings`. Editing the vars is the
entire customization surface — no theming engine needed.

## 6. Deferred Features — Designed Seams

| Feature | Approach when built | Difficulty |
|---|---|---|
| **WAV export** | Offline-render the whole sequence with `rustysynth` → write with `hound`. | Easy |
| **MP3 export** | Render → encode with `mp3lame-encoder` (LAME). | Easy-ish (C dep) |
| **MusicXML export** | MIDI→notation: quantize to a grid, infer key/time sig, separate voices. Best-effort, lowest priority. | **Hard** |
| **Plugin loading** | Load JS modules from `plugins/` against the Visualization contract; later a backend plugin surface. | Medium; needs sandbox story |
| **MIDI editing** | Mutate the parsed timeline; re-emit; write back via `midly`. | Medium-Hard |

The export functions are designed as one shared offline-render core (`render_to_pcm`) with
per-format encoders hung off it, so adding a format is adding an encoder, not a pipeline.

## 7. Licensing & Assets

- **App code:** Apache-2.0 (`LICENSE`, SPDX headers where useful, `NOTICE`).
- **Bundled soundfont:** ships one small, permissively-licensed General MIDI `.sf2` under
  `assets/soundfonts/`, with its **own** license file alongside it (a bundled asset's license is
  independent of the app's). Candidate fonts are evaluated for redistribution terms at build time;
  the chosen font's license is vendored verbatim. This is a checklist item, not an assumption.
- **Dependencies:** prefer MIT/Apache/BSD pure-Rust crates (`midly`, `rustysynth`, `cpal`,
  `rustfft`, `turso`, `hound`).

## 8. CI/CD

A GitHub Actions workflow (`.github/workflows/build.yml`) using `tauri-apps/tauri-action` with a
macOS / Windows / Linux matrix. Committed now, **inert until a remote exists** — costs nothing to
have ready, matches the "plan for CI/CD" requirement.

## 9. Repository Layout (target)

```
midimi/
├─ src-tauri/            # Rust backend
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ commands.rs
│  │  ├─ audio/          # synth, sequencer, output, analysis
│  │  ├─ db.rs           # turso
│  │  └─ files.rs        # midi/sf2 scanning
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ src/                  # Svelte 5 + TS frontend
│  ├─ lib/visualizations/cosmic-aurora.ts
│  ├─ lib/theme.ts
│  └─ App.svelte
├─ assets/soundfonts/    # bundled .sf2 + its LICENSE
├─ docs/                 # ARCHITECTURE.md, DECISIONS.md, specs/
├─ .github/workflows/build.yml
├─ LICENSE (Apache-2.0)
└─ README.md
```

## 10. Risks & Mitigations

| Risk | Mitigation |
|---|---|
| `turso` engine is BETA | DB is cache-only and rebuildable; filesystem is source of truth. |
| Audio glitches in `cpal` callback | Keep the callback lock-free (atomics + ringbuffer); do FFT off the hot path. |
| Visual/audio drift | Single atomic playhead clock; visuals never keep their own time. |
| Soundfont licensing | Vendor the font's own license; verify redistribution terms before bundling. |
| `rustysynth` sequencer tempo/seek limits | If its sequencer can't seek/rate cleanly, drive note scheduling from the `midly` timeline and feed `rustysynth` raw note events. (Fallback noted, not pre-built.) |

## 11. Roadmap (post-v1, indicative)

1. **v1** — vertical slice (this spec).
2. **v1.1** — WAV + MP3 export; a second built-in visualization (WebGL shader).
3. **v1.2** — plugin *loading* for visualizations; theme editor UI.
4. **v2** — MIDI editing; MusicXML export; optional Turso cloud sync for the library.

## 12. Open Questions

None blocking v1. The soundfont selection (which specific `.sf2` to bundle) is resolved during
implementation against redistribution terms.
