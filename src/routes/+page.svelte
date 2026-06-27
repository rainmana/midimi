<script lang="ts">
  import * as ipc from '$lib/ipc';
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
