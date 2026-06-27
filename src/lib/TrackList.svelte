<script lang="ts">
  import type { MidiData } from './types';
  let { midi }: { midi: MidiData | null } = $props();
  const HUES = [190, 280, 320, 50, 150, 220, 0, 100];
  const counts = $derived.by(() => {
    const c: Record<number, number> = {};
    midi?.notes.forEach((n) => (c[n.track] = (c[n.track] ?? 0) + 1));
    return c;
  });
</script>

{#if midi}
<div class="panel">
  <h3>Tracks</h3>
  {#each midi.tracks as tr}
    {#if (counts[tr.index] ?? 0) > 0}
      <div class="row">
        <span class="dot" style="background: hsl({HUES[tr.index % HUES.length]} 90% 62%)"></span>
        <span class="name">{tr.name ?? `Track ${tr.index + 1}`}</span>
        <span class="count">{counts[tr.index]}</span>
      </div>
    {/if}
  {/each}
</div>
{/if}

<style>
  .row { display: flex; align-items: center; gap: 8px; padding: 3px 0; font-size: 13px; }
  .dot { width: 10px; height: 10px; border-radius: 50%; box-shadow: 0 0 8px currentColor; flex: none; }
  .name { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .count { color: var(--muted); font-size: 11px; font-variant-numeric: tabular-nums; }
</style>
