<script lang="ts">
  import * as ipc from '$lib/ipc';
  import VizCanvas from '$lib/VizCanvas.svelte';
  import type { MidiData, Playhead } from '$lib/types';

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
