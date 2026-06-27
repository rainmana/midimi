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
          const ev: NoteEvent = { track: nn.track, channel: nn.channel, note: nn.note, velocity: nn.velocity };
          v.onNoteOn(ev);
          active.push({ ev, end: nn.start_sec + nn.dur_sec });
          ptr++;
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
