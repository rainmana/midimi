# Visualization Gallery + Melody's Path — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans (or subagent-driven-development). Steps use checkbox (`- [ ]`).

**Goal:** Add a switchable visualization gallery and the first structure-revealing visualization, Melody's Path (a scrolling per-track melodic contour).

**Architecture:** Frontend-only (the note timeline already arrives via `open_midi`). Extend the `Visualization` contract with optional `loadTimeline`; a registry + `VizCanvas` swap + `VizPicker` enable switching; `melody-path.ts` renders the contour. Pure-TS geometry helpers are vitest-tested.

**Tech Stack:** Svelte 5 + TypeScript + Canvas 2D; vitest.

## Global Constraints

- SvelteKit project: components in `src/lib/`, imported via `$lib/…`; app root `src/routes/+page.svelte`; `ssr=false`.
- Preserve VizCanvas's discipline: read `head` ONLY inside the RAF loop (untracked); read `vizId`/`midi` synchronously in the effect (tracked) so the loop rebuilds on switch/song-change but NOT every frame.
- Locked viz decisions: scrolling window, playhead at `0.30 * width`, window `PAST=3s` / `FUTURE=7s`, per-track ribbons (`TRACK_HUES`), leaps emphasized, reuse cosmic-aurora glow.
- Persist the active viz as a `viz` setting (default `cosmic-aurora`); restore on startup.

---

## Task 1: Geometry helpers + vitest + contract extension (TDD)

**Files:**
- Create: `src/lib/visualizations/geometry.ts`, `src/lib/visualizations/geometry.test.ts`, `vitest.config.ts`
- Modify: `src/lib/visualizations/types.ts` (+`loadTimeline?`), `package.json` (vitest dep + `test` script)

**Interfaces — Produces:** `pitchToY(note, top, bottom, minPitch?, maxPitch?) -> number`; `windowSlice(sorted: Note[], t0, t1) -> Note[]`; `leapIntensity(prev, next) -> number (0..1)`; `Visualization.loadTimeline?(notes: Note[], durationSec: number)`.

- [ ] **Step 1: Install vitest + script**

`npm i -D vitest`, then add to `package.json` scripts: `"test": "vitest run"`.

- [ ] **Step 2: `vitest.config.ts`** (isolated from the SvelteKit Vite config — pure-TS tests)

```ts
import { defineConfig } from 'vitest/config';
export default defineConfig({ test: { include: ['src/**/*.test.ts'], environment: 'node' } });
```

- [ ] **Step 3: Write the failing test** — `src/lib/visualizations/geometry.test.ts`

```ts
import { describe, it, expect } from 'vitest';
import { pitchToY, windowSlice, leapIntensity } from './geometry';
import type { Note } from '../types';

const n = (note: number, start_sec: number): Note => ({ track: 0, channel: 0, note, start_sec, dur_sec: 0.5, velocity: 100 });

describe('pitchToY', () => {
  it('maps higher pitch nearer the top and clamps', () => {
    expect(pitchToY(108, 0, 100)).toBeCloseTo(0);
    expect(pitchToY(21, 0, 100)).toBeCloseTo(100);
    expect(pitchToY(200, 0, 100)).toBeCloseTo(0);
    expect(pitchToY(0, 0, 100)).toBeCloseTo(100);
    expect(pitchToY(72, 0, 100)).toBeLessThan(pitchToY(60, 0, 100));
  });
});
describe('windowSlice', () => {
  it('returns only notes within the inclusive window', () => {
    const notes = [n(60, 0), n(62, 2), n(64, 5), n(65, 9)];
    expect(windowSlice(notes, 1, 6).map((x) => x.note)).toEqual([62, 64]);
  });
});
describe('leapIntensity', () => {
  it('is low for a step and ~1 for an octave', () => {
    expect(leapIntensity(60, 62)).toBeLessThan(0.3);
    expect(leapIntensity(60, 72)).toBeCloseTo(1);
    expect(leapIntensity(60, 90)).toBeCloseTo(1);
  });
});
```

- [ ] **Step 4: Run → FAIL** — `npm test` (geometry.ts missing).

- [ ] **Step 5: Implement** — `src/lib/visualizations/geometry.ts`

```ts
import type { Note } from '../types';

export function pitchToY(note: number, top: number, bottom: number, minPitch = 21, maxPitch = 108): number {
  const f = Math.max(0, Math.min(1, (note - minPitch) / (maxPitch - minPitch)));
  return bottom - f * (bottom - top); // higher pitch -> nearer top
}
export function windowSlice(sorted: Note[], t0: number, t1: number): Note[] {
  return sorted.filter((nn) => nn.start_sec >= t0 && nn.start_sec <= t1);
}
export function leapIntensity(prevNote: number, nextNote: number): number {
  return Math.max(0, Math.min(1, Math.abs(nextNote - prevNote) / 12));
}
```

- [ ] **Step 6: Extend the contract** — in `src/lib/visualizations/types.ts`, add `import type { Note } from '../types';` at top and add to the `Visualization` interface:

```ts
  /** Optional: full note timeline on song load (for structure-revealing vises). */
  loadTimeline?(notes: Note[], durationSec: number): void;
```

- [ ] **Step 7: Run → PASS + typecheck** — `npm test` (3 pass) and `npm run check` (0 errors).

- [ ] **Step 8: Commit** — `git add -A && git commit -m "feat(viz): geometry helpers + loadTimeline contract + vitest"`

---

## Task 2: Gallery foundation — registry, VizCanvas swap, VizPicker, persistence

**Files:**
- Create: `src/lib/visualizations/registry.ts`, `src/lib/VizPicker.svelte`
- Modify: `src/lib/VizCanvas.svelte` (vizId prop + single-effect swap), `src/routes/+page.svelte` (mount picker + restore `viz`)

**Interfaces — Consumes:** `createViz` (registry). **Produces:** `VIZZES`, `createViz(id)`; `VizCanvas` prop `vizId`.

- [ ] **Step 1: `registry.ts`** (cosmic-aurora only for now; Task 3 adds melody-path)

```ts
import type { Visualization } from './types';
import { createCosmicAurora } from './cosmic-aurora';

export interface VizEntry { id: string; name: string; create: () => Visualization; }
export const VIZZES: VizEntry[] = [
  { id: 'cosmic-aurora', name: 'Cosmic Aurora', create: createCosmicAurora },
];
export function createViz(id: string): Visualization {
  return (VIZZES.find((v) => v.id === id) ?? VIZZES[0]).create();
}
```

- [ ] **Step 2: Rewrite `src/lib/VizCanvas.svelte`** to a single (vizId, midi)-driven effect (head read only inside the loop):

```svelte
<script lang="ts">
  import { createViz } from './visualizations/registry';
  import type { MidiData, Playhead } from './types';
  import type { NoteEvent } from './visualizations/types';

  let { midi, head, vizId = 'cosmic-aurora' }:
    { midi: MidiData | null; head: Playhead | null; vizId?: string } = $props();

  let host: HTMLDivElement;
  let canvas: HTMLCanvasElement;

  // Rebuild the viz + RAF loop on viz switch OR new song. `head` is read only
  // inside `loop`, so it is NOT a dependency (no per-frame rebuild).
  $effect(() => {
    const id = vizId;
    const m = midi;
    const v = createViz(id);
    v.setup(canvas);
    v.loadTimeline?.(m?.notes ?? [], m?.duration_sec ?? 0);
    v.resize(host.clientWidth, host.clientHeight);
    const ro = new ResizeObserver(() => v.resize(host.clientWidth, host.clientHeight));
    ro.observe(host);

    let ptr = 0;
    let active: { ev: NoteEvent; end: number }[] = [];
    let lastTime = 0;
    let raf = 0;
    const loop = () => {
      const time = head?.time ?? 0;
      const level = head?.level ?? 0;
      const bands = head?.bands ?? [];
      if (m) {
        if (time < lastTime - 0.05) { for (const a of active) v.onNoteOff(a.ev); ptr = 0; active = []; }
        const notes = m.notes;
        while (ptr < notes.length && notes[ptr].start_sec <= time) {
          const nn = notes[ptr];
          const ev = { track: nn.track, channel: nn.channel, note: nn.note, velocity: nn.velocity };
          v.onNoteOn(ev); active.push({ ev, end: nn.start_sec + nn.dur_sec }); ptr++;
        }
        for (let i = active.length - 1; i >= 0; i--) {
          if (active[i].end <= time) { v.onNoteOff(active[i].ev); active.splice(i, 1); }
        }
      }
      lastTime = time;
      v.onFrame(time, level, bands);
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => { cancelAnimationFrame(raf); ro.disconnect(); v.teardown(); };
  });
</script>

<div class="viz" bind:this={host}><canvas bind:this={canvas}></canvas></div>

<style>
  .viz { position: absolute; inset: 0; overflow: hidden; background: #05040c; }
  canvas { display: block; width: 100%; height: 100%; }
</style>
```

- [ ] **Step 3: `src/lib/VizPicker.svelte`**

```svelte
<script lang="ts">
  import { VIZZES } from './visualizations/registry';
  import * as ipc from './ipc';
  let { current = $bindable('cosmic-aurora') }: { current?: string } = $props();
  async function choose(id: string) { current = id; await ipc.setSetting('viz', id); }
</script>

<div class="panel">
  <h3>Visualization</h3>
  {#each VIZZES as v}
    <button class:active={current === v.id} onclick={() => choose(v.id)}>{v.name}</button>
  {/each}
</div>

<style>
  button { display: block; width: 100%; text-align: left; margin-bottom: 4px; font-size: 13px; }
  button.active { border-color: var(--accent); color: var(--accent); }
</style>
```

- [ ] **Step 4: Wire `src/routes/+page.svelte`** — add `import VizPicker from '$lib/VizPicker.svelte';`, a `let vizId = $state('cosmic-aurora');`, pass `vizId` to `<VizCanvas {midi} {head} {vizId} />`, mount `<VizPicker bind:current={vizId} />` in the `<aside class="panels">` stack (e.g. above ThemePicker), and in the startup settings `$effect` set `vizId = settings.find((s) => s.key === 'viz')?.value ?? 'cosmic-aurora';`.

- [ ] **Step 5: Gate + commit** — `npm run check` (0 errors) and `npm run build` (succeeds). Commit: `git add -A && git commit -m "feat(viz): visualization gallery (registry, switcher, persistence)"`. Manual (owner) later: picker shows Cosmic Aurora; switching persists across restart.

---

## Task 3: Melody's Path visualization

**Files:**
- Create: `src/lib/visualizations/melody-path.ts`
- Modify: `src/lib/visualizations/cosmic-aurora.ts` (export `TRACK_HUES`), `src/lib/visualizations/registry.ts` (register melody-path)

**Interfaces — Consumes:** `pitchToY/windowSlice/leapIntensity` (geometry), `TRACK_HUES` (cosmic-aurora), `Visualization`/`NoteEvent`. **Produces:** `createMelodyPath(): Visualization`.

- [ ] **Step 1: Export the shared hues** — in `cosmic-aurora.ts` change `const TRACK_HUES = [...]` to `export const TRACK_HUES = [...]` (no other change).

- [ ] **Step 2: `src/lib/visualizations/melody-path.ts`**

```ts
import type { Visualization, NoteEvent } from './types';
import type { Note } from '../types';
import { pitchToY, windowSlice, leapIntensity } from './geometry';
import { TRACK_HUES } from './cosmic-aurora';

const PAST = 3, FUTURE = 7, PLAYHEAD_FRAC = 0.30;

interface Bloom { x: number; y: number; hue: number; life: number; r: number; }

export function createMelodyPath(): Visualization {
  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let w = 0, h = 0, dpr = 1;
  let byTrack: Note[][] = [];
  let blooms: Bloom[] = [];
  let smoothLevel = 0;

  const noteY = (note: number) => pitchToY(note, h * 0.12, h * 0.88);

  return {
    id: 'melody-path',
    name: "Melody's Path",
    setup(c) { canvas = c; ctx = c.getContext('2d')!; },
    resize(width, height) {
      dpr = window.devicePixelRatio || 1; w = width; h = height;
      canvas.width = Math.floor(width * dpr); canvas.height = Math.floor(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    },
    loadTimeline(notes: Note[]) {
      byTrack = [];
      for (const nn of notes) (byTrack[nn.track] ??= []).push(nn);
      for (const t of byTrack) if (t) t.sort((a, b) => a.start_sec - b.start_sec);
      blooms = [];
    },
    onNoteOn(n: NoteEvent) {
      blooms.push({ x: PLAYHEAD_FRAC * w, y: noteY(n.note), hue: TRACK_HUES[n.track % TRACK_HUES.length], life: 0, r: 10 + (n.velocity / 127) * 26 });
    },
    onNoteOff() {},
    onFrame(playhead, level, _bands) {
      smoothLevel += (level - smoothLevel) * 0.2;
      const playheadX = PLAYHEAD_FRAC * w;
      const pxPerSec = w / (PAST + FUTURE);
      const t0 = playhead - PAST, t1 = playhead + FUTURE;

      ctx.globalCompositeOperation = 'source-over';
      ctx.fillStyle = 'rgba(5, 4, 12, 0.30)';
      ctx.fillRect(0, 0, w, h);

      ctx.strokeStyle = 'rgba(120,110,170,0.12)'; ctx.lineWidth = 1;
      for (const p of [36, 48, 60, 72, 84, 96]) {
        const y = noteY(p); ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke();
      }

      ctx.globalCompositeOperation = 'lighter';
      ctx.lineCap = 'round';
      for (let ti = 0; ti < byTrack.length; ti++) {
        const track = byTrack[ti];
        if (!track || track.length === 0) continue;
        const hue = TRACK_HUES[ti % TRACK_HUES.length];
        const vis = windowSlice(track, t0, t1);
        for (let i = 0; i < vis.length; i++) {
          const a = vis[i];
          const ax = playheadX + (a.start_sec - playhead) * pxPerSec;
          const ay = noteY(a.note);
          if (i > 0) {
            const b = vis[i - 1];
            const bx = playheadX + (b.start_sec - playhead) * pxPerSec;
            const by = noteY(b.note);
            const leap = leapIntensity(b.note, a.note);
            ctx.strokeStyle = `hsla(${hue}, 95%, ${60 + leap * 25}%, ${0.35 + leap * 0.5})`;
            ctx.lineWidth = (2 + leap * 4) * (1 + 0.5 * smoothLevel);
            ctx.beginPath(); ctx.moveTo(bx, by); ctx.lineTo(ax, ay); ctx.stroke();
          }
          ctx.fillStyle = `hsla(${hue}, 90%, 72%, 0.9)`;
          ctx.beginPath(); ctx.arc(ax, ay, 2.5, 0, Math.PI * 2); ctx.fill();
        }
      }

      for (let i = blooms.length - 1; i >= 0; i--) {
        const o = blooms[i];
        o.life += 1 / 60;
        if (o.life > 1) { blooms.splice(i, 1); continue; }
        const k = 1 - o.life;
        const rr = o.r * (1 + 0.6 * smoothLevel);
        const g = ctx.createRadialGradient(o.x, o.y, 0, o.x, o.y, rr * 2.5);
        g.addColorStop(0, `hsla(${o.hue}, 100%, 82%, ${0.9 * k})`);
        g.addColorStop(1, `hsla(${o.hue}, 100%, 60%, 0)`);
        ctx.fillStyle = g;
        ctx.beginPath(); ctx.arc(o.x, o.y, rr * 2.5, 0, Math.PI * 2); ctx.fill();
      }

      ctx.globalCompositeOperation = 'source-over';
      ctx.strokeStyle = 'rgba(159, 252, 255, 0.85)'; ctx.lineWidth = 1.5;
      ctx.beginPath(); ctx.moveTo(playheadX, 0); ctx.lineTo(playheadX, h); ctx.stroke();
    },
    teardown() { byTrack = []; blooms = []; },
  };
}
```

- [ ] **Step 3: Register it** — in `registry.ts` add `import { createMelodyPath } from './melody-path';` and append `{ id: 'melody-path', name: "Melody's Path", create: createMelodyPath }` to `VIZZES`.

- [ ] **Step 4: Gate + commit** — `npm run check` (0 errors), `npm run build` (succeeds), `npm test` (still green). Commit: `git add -A && git commit -m "feat(viz): Melody's Path scrolling contour visualization"`. Manual (owner): load a MIDI → switch to Melody's Path → contour scrolls past the playhead, leaps pop, notes bloom on crossing.

---

## Self-Review

- **Spec coverage:** gallery foundation (registry/VizCanvas/VizPicker/persistence) → Task 2; `loadTimeline` contract + geometry + vitest → Task 1; Melody's Path scrolling contour + leaps + bloom → Task 3; persistence of `viz` → Task 2 Step 4. cosmic-aurora unaffected (only `TRACK_HUES` exported). ✓
- **Placeholder scan:** none — every step has complete code/commands.
- **Type consistency:** `createViz`/`VIZZES` (registry) used by VizCanvas + VizPicker; `loadTimeline?(notes, durationSec)` defined in Task 1, called in Task 2 (VizCanvas) and implemented in Task 3; `TRACK_HUES` exported in Task 3 Step 1 before melody-path imports it; `pitchToY/windowSlice/leapIntensity` signatures consistent across Task 1 and Task 3. ✓
