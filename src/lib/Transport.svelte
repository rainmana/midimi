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
