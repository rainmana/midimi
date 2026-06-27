# midimi — Visualization Gallery + Melody's Path — Design Spec

- **Status:** Approved (owner: "keep cruising")
- **Date:** 2026-06-27
- **Tracks issues:** [#2](https://github.com/rainmana/midimi/issues/2) (gallery foundation), [#3](https://github.com/rainmana/midimi/issues/3) (Melody's Path)
- **Roadmap:** sub-project 1 of the Visualization Gallery (the four structure-revealing vises, #3–#6)

---

## 1. Vision & Scope

Two deliverables in one slice:
1. **Visualization Gallery foundation** — let midimi hold *many* visualizations and switch between them from the UI, with the choice persisted. cosmic-aurora becomes gallery item #1.
2. **Melody's Path** — the first structure-revealing visualization: a *scrolling* per-track melodic contour that shows the relative rise/fall/leap of the music as it plays.

This is **frontend-only**. The note timeline already arrives from the backend via `open_midi` (`MidiData.notes`); no Rust changes.

## 2. Goals

- Switch visualizations from a picker; the selection persists across launches.
- Melody's Path renders each track's melodic contour scrolling past a fixed playhead, with big interval leaps emphasized and notes blooming as they're played.
- Lay the **foundation the other three vises reuse**: the timeline-aware contract, the registry/switcher, a shared geometry/analysis helper module, and a vitest test bed.

## 3. Non-Goals (separate issues)

- Harmonic Web / Song Architecture / Tension & Release (#4 / #5 / #6) — each its own spec.
- WebGL rendering + user plugin loading (#12).
- Zoom / horizontal scrub of the window; algorithmic "which track is the melody" extraction (we show all tracks).

## 4. Architecture

### 4.1 The one architectural change — timeline-aware contract
Today a `Visualization` only hears notes *as they fire* (`onNoteOn`/`onNoteOff`) — fine for ambient cosmic-aurora, but a *scrolling* view must draw notes that have **not played yet** (the path approaching from the right). So extend the contract with one optional method:

```ts
loadTimeline?(notes: Note[], durationSec: number): void;
```

`VizCanvas` calls it whenever a song loads **and** whenever the active viz changes. cosmic-aurora ignores it; Melody's Path (and the future three vises) use it. This is the keystone — every structure-revealing viz needs the whole timeline, not just live events.

### 4.2 Registry + switching
- `src/lib/visualizations/registry.ts` exports `VIZZES: { id: string; name: string; create(): Visualization }[]` = `[cosmic-aurora, melody-path]`.
- `VizCanvas` gains an `vizId` prop. When it changes: `teardown()` the old viz → `create()`+`setup()` the new one → `resize()` → re-feed `loadTimeline(currentNotes, duration)`. The once-only RAF loop (which must **not** be re-created per frame — preserve the existing effect structure) keeps calling the active viz's `onFrame`.
- `src/lib/VizPicker.svelte` (mirrors `ThemePicker`): lists `VIZZES`, selecting one updates state + `setSetting('viz', id)`. `+page.svelte` restores the `viz` setting on startup (default `cosmic-aurora`) alongside the theme.

### 4.3 Shared helpers (the toolkit the gallery is built on)
`src/lib/visualizations/geometry.ts` — pure functions, unit-tested:
- `pitchToY(note: number, top: number, bottom: number, minPitch = 21, maxPitch = 108): number` — map MIDI pitch to a y within `[top, bottom]` (higher pitch → smaller y), clamped.
- `windowSlice(sorted: Note[], t0: number, t1: number): Note[]` — notes whose `start_sec` falls in `[t0, t1]` (the visible window).
- `leapIntensity(prevNote: number, nextNote: number): number` — `0..1` emphasis for an interval, ramping with semitone distance (e.g. an octave → ~1).

## 5. Melody's Path component (`melody-path.ts`)

Implements `Visualization` + `loadTimeline`.

- **State:** on `loadTimeline`, group notes by track, each sorted by `start_sec`.
- **Layout:** fixed playhead at `PLAYHEAD_X = 0.30 * width`. Visible window = `[playhead − PAST, playhead + FUTURE]` with `PAST = 3s`, `FUTURE = 7s`. `pxPerSec = width / (PAST + FUTURE)`. Future scrolls in from the right and flows left past the playhead.
- **`onFrame(playhead, level, bands)`:** clear with a translucent dark trail; for each track, `windowSlice` its notes, map `x = PLAYHEAD_X + (note.start_sec − playhead) * pxPerSec`, `y = pitchToY(note.note, …)`; draw a glowing polyline in the track's cosmic hue (reuse `TRACK_HUES`) with note-dots at vertices. **Leaps emphasized:** each segment's brightness/width scales with `leapIntensity` so a dramatic jump pops. Ambient glow scales with `level`/`bands`. Draw the fixed playhead line + faint octave gridlines.
- **`onNoteOn(note)`:** spawn a bloom at the note's current screen position (it's crossing the playhead) — reuse cosmic-aurora's radial bloom, scaled by velocity. Blooms fade over ~1s.
- **`resize` / dpr:** same HiDPI handling as cosmic-aurora.

Shared aesthetic helpers (`TRACK_HUES`, the radial-bloom draw) are extracted from cosmic-aurora into a tiny module both import (or duplicated if extraction churns cosmic-aurora too much — prefer extraction).

## 6. Data Flow

`open_midi` → `MidiData` → `+page.svelte` (`midi` state) → `VizCanvas` calls active viz's `loadTimeline(midi.notes, midi.duration_sec)`. The `"playhead"` event (~60 Hz) → `VizCanvas` RAF loop → active viz `onFrame(playhead, level, bands)`. Note on/off still derived in `VizCanvas` from `(timeline + playhead)` as today.

## 7. Persistence

A `viz` setting (string id, default `cosmic-aurora`), written via `set_setting` and restored on startup in the `+page.svelte` effect that already restores `theme` and `recent`.

## 8. Testing

- **vitest** (new dev dep + `"test": "vitest run"` script): unit-test `geometry.ts` — `pitchToY` is monotonic and clamps to bounds; `windowSlice` returns only in-range notes (boundary-inclusive as specified); `leapIntensity` ~0 for a step and high for an octave.
- **Interactive (owner):** `npm run tauri dev` → load a MIDI → open the viz picker → switch to Melody's Path → confirm the contour scrolls past the playhead, leaps visibly pop, notes bloom as they cross, and switching back to Cosmic Aurora works. Switching persists across restart.

## 9. Files

**New:** `src/lib/visualizations/registry.ts`, `src/lib/visualizations/melody-path.ts`, `src/lib/visualizations/geometry.ts`, `src/lib/visualizations/geometry.test.ts`, `src/lib/VizPicker.svelte`.
**Modified:** `src/lib/visualizations/types.ts` (+`loadTimeline?`), `src/lib/VizCanvas.svelte` (active-viz swap + feed timeline), `src/routes/+page.svelte` (mount VizPicker + restore `viz`), `src/lib/visualizations/cosmic-aurora.ts` (export shared bloom/hues if extracted), `package.json`/`vite.config.js` (vitest).
**Backend:** none.

## 10. Risks & Mitigations

| Risk | Mitigation |
|---|---|
| The viz swap touches `VizCanvas` — the single place driving all vises | Preserve the once-only RAF `$effect` structure (the prior fix); swap only the viz object, not the loop. |
| vitest + SvelteKit/Vite config friction | vitest reads the existing `vite.config.js`; add a minimal `test` block. Keep tests on pure `geometry.ts` (no DOM). |
| Scrolling perf on dense MIDI | `windowSlice` bounds per-frame work to the visible ~10s window; notes pre-sorted per track. |
| Contract `loadTimeline` optional vs required | Optional (`?`) so cosmic-aurora and any future event-only viz need not implement it. |

## 11. Locked Decisions

Scrolling window (not whole-piece) · per-track ribbons (no melody extraction) · playhead at 30% · window 3s past / 7s future · leaps emphasized · reuse cosmic-aurora glow/hues · frontend-only · vitest for geometry helpers.
