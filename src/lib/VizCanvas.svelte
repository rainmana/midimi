<script lang="ts">
  import { createCosmicAurora } from './visualizations/cosmic-aurora';
  import type { MidiData, Playhead } from './types';
  import type { NoteEvent } from './visualizations/types';

  let { midi, head }: { midi: MidiData | null; head: Playhead | null } = $props();

  let host: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  const viz = createCosmicAurora();
  let raf = 0;
  let ptr = 0;
  let active: { ev: NoteEvent; end: number }[] = [];
  let lastTime = 0;

  function resetSchedule() { for (const a of active) viz.onNoteOff(a.ev); ptr = 0; active = []; lastTime = 0; }

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
          const ev: NoteEvent = { track: n.track, channel: n.channel, note: n.note, velocity: n.velocity };
          viz.onNoteOn(ev);
          active.push({ ev, end: n.start_sec + n.dur_sec });
          ptr++;
        }
        for (let i = active.length - 1; i >= 0; i--) {
          if (active[i].end <= time) {
            viz.onNoteOff(active[i].ev);
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
