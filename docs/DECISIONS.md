# Design Decisions

Architecture Decision Records (ADRs) for midimi v1. Each entry: context, decision, consequence.

---

## ADR-1: Turso (embedded SQLite) as a cache-only local database

**Context.** midimi needs to persist a small amount of user state: the recent-files list, registered soundfonts, and key-value settings (theme). The app must work fully offline with no network dependency. The official `libsql` crate targets the cloud-hosted Turso service; the newer `turso` crate (v0.6) wraps the same engine but runs entirely in-process.

**Decision.** Use the `turso` crate (`turso = "0.6.1"`) with `Builder::new_local(path)` to open an embedded SQLite file in the Tauri app-data directory (`midimi.db`). The database holds three tables (`library`, `soundfonts`, `settings`) and is treated as a **rebuildable cache**: the filesystem is the source of truth. If the DB file is deleted it is recreated from scratch on the next launch with no data loss to the user's MIDI files or soundfonts.

**Consequence.** No network calls. Startup is synchronous (`block_on`). The `turso` crate is in beta; using an in-process SQLite wrapper that is rebuildable means any instability produces a degraded-but-functional state (missing recent list) rather than data corruption. Accepting beta risk is appropriate here.

---

## ADR-2: Rust-side synthesis (rustysynth) over Web Audio

**Context.** MIDI playback could be implemented in the browser via the Web Audio API's MIDI synthesizer pathway, or server-side in Rust. The v1 spec includes a future WAV/MP3 offline export feature (v1.1).

**Decision.** Synthesize in Rust using `rustysynth` (`MidiFileSequencer` + `Synthesizer`). The render thread produces raw PCM which goes directly to the system audio device via `cpal`.

**Consequence.** Authoritative timing: the sequencer's sample counter is the single clock; there is no browser event loop jitter between note scheduling and audio output. Offline export (`render_offline` in `audio.rs`) is trivial because the same render path runs headlessly without a cpal stream. Web Audio is entirely absent from the stack, which also removes the Web Audio context lifecycle (autoplay policy, context suspend/resume) as a failure mode.

---

## ADR-3: Dedicated render thread + lock-free rtrb ring buffer

**Context.** `cpal` delivers audio in a real-time callback on an OS audio thread. Locking a mutex inside that callback risks priority inversion and audio glitches (dropout, clicks). rustysynth's `MidiFileSequencer::render` is not realtime-safe if called directly from the callback.

**Decision.** Split synthesis into a dedicated **render thread** that pushes PCM into a lock-free `rtrb::RingBuffer<f32>`. The cpal callback only calls `consumer.pop()`, which never blocks or allocates. Buffer capacity is 8 × 1 024 stereo frames; underruns produce silence.

**Consequence.** The cpal callback is safe for realtime scheduling. The `cpal::Stream` returned by `build_output_stream` must be `Send` to store inside Tauri's managed state (`AppState`); cpal 0.18's `Stream: Send` bound is load-bearing here — earlier versions required an `unsafe` workaround. Dropping `_stream` stops audio, so it is kept alive in `AudioEngine` for the application lifetime.

---

## ADR-4: Seek implemented as replay-from-0 + render-and-discard

**Context.** Users need to seek to arbitrary positions during playback. rustysynth 1.3.6 has **no seek API** — `MidiFileSequencer` can only replay from the beginning.

**Decision.** On `Cmd::Seek(target)`, call `sequencer.play(midi, false)` to reset to position 0, then call `sequencer.render(...)` in a tight loop (discarding the output) until `get_position() >= target`. This runs on the render thread (not the cpal callback) and is safe to do synchronously.

**Consequence.** Seek cost is O(target × sample_rate / block_size) render calls. For a 5-minute file seeking to the midpoint, this is roughly 2.5 × 60 × 44 100 / 1 024 ≈ 6 500 iterations — fast enough in practice but noticeable on very long files at high sample rates. Document this if rustysynth ever adds a real seek API; until then this is the only option.

---

## ADR-5: Per-track mute/solo deferred to v1.1

**Context.** The v1 spec lists mute and solo as desired transport controls. `rustysynth::MidiFileSequencer` drives a `Synthesizer` that plays all MIDI channels simultaneously. There is no API to silence individual channels during playback.

**Decision.** Defer per-track mute/solo to v1.1. The v1 track panel shows track names and structure only. Implementing mute/solo requires replacing `MidiFileSequencer` with a hand-written event scheduler that calls `Synthesizer::process_midi_message` per event and skips muted tracks.

**Consequence.** v1 ships without mute/solo. Users can see track names and counts but cannot isolate individual parts. The workaround (open a DAW for isolation) is acceptable for the v1 scope. The `TrackList` component is already structured to add checkboxes once the scheduler is in place.

---

## ADR-6: GeneralUser GS bundled as a Tauri resource

**Context.** MIDI playback requires a General MIDI soundfont. Shipping no soundfont means the user must source their own on first launch — a poor out-of-box experience.

**Decision.** Bundle **GeneralUser GS v2.0** by S. Christian Collins. License: permissive, Apache-2-compatible (see `assets/soundfonts/GeneralUser-GS-LICENSE.txt`). The `.sf2` file (~31 MB) is gitignored and fetched by `scripts/fetch-soundfont.sh` at dev/CI time, then declared as a Tauri resource in `tauri.conf.json`. The license text is vendored at `assets/soundfonts/GeneralUser-GS-LICENSE.txt`.

**Consequence.** Users get working audio on first launch with no configuration. The fetch script (`curl` from GitHub releases) must run before `npm run tauri dev` or `npm run tauri build`. CI (`build.yml`) runs the script as the first build step. The `.sf2` is not committed to git, keeping the repository small.

---

## ADR-7: Svelte 5 + raw Canvas 2D for the visualization layer

**Context.** The primary UI surface is a full-screen animation driven by a 60 Hz `requestAnimationFrame` loop with per-frame DOM writes. Reactive UI frameworks with virtual DOM diffing (React, Vue) incur reconciliation overhead that fights a tight animation loop. The visualization plugin contract requires a direct reference to `HTMLCanvasElement`.

**Decision.** Use **Svelte 5** (compile-time reactivity, no VDOM) for the application shell and **raw Canvas 2D** (`CanvasRenderingContext2D`) for visualization. Svelte's `$effect` and `$props` runes manage the canvas lifecycle (setup, resize, teardown) without fighting the rAF loop.

**Consequence.** Minimal runtime overhead. The visualization contract is `Canvas 2D` — it is available everywhere and requires no GPU setup. **WebGL** is the documented upgrade path for the first real external visualization plugin; the `Visualization` interface abstracts the canvas element, so a WebGL plugin can acquire a `webgl2` context in `setup()` without any other code change.

---

## ADR-8: SvelteKit + adapter-static (SPA, SSR disabled) instead of plain Svelte

**Context.** The `create-tauri-app` `svelte-ts` template scaffolds a SvelteKit project. An alternative would be plain Vite + Svelte (no router framework).

**Decision.** Keep SvelteKit with `@sveltejs/adapter-static` and `export const ssr = false` in `src/routes/+layout.ts`. The adapter writes a fully static SPA bundle to `build/`, which Tauri serves from disk. The SvelteKit file layout (`src/routes/+page.svelte`, components in `src/lib/`) matches the official Tauri + Svelte documentation.

**Consequence.** v1 has only one route, so SvelteKit's router is invisible. If the app grows multi-page (settings page, plugin manager) the router is already in place. The SSR output (`src-tauri/src/lib.rs` knows nothing about it) is simply discarded by Tauri. adapter-static produces deterministic file URLs that Tauri's asset protocol can resolve without a dev server in production builds.

---

## Known follow-up: render_offline reverb tail clipping

`audio::render_offline` stops rendering when `sequencer.end_of_sequence()` returns true, which happens as soon as the last MIDI event is processed. The synthesizer's internal reverb/chorus tail continues for several seconds after the last note but is not captured. Before `render_offline` becomes the WAV/MP3 export path in v1.1, add a fixed tail-drain loop (e.g., render an additional 3 seconds of silence through the synthesizer) to capture the reverb decay.
