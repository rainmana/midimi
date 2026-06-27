<script lang="ts">
  import * as ipc from './ipc';
  import type { SoundfontRow } from './types';
  let fonts = $state<SoundfontRow[]>([]);
  let current = $state<number | null>(null);

  async function refresh() {
    fonts = await ipc.listSoundfonts();
    if (current == null && fonts[0]) current = fonts[0].id;
  }
  $effect(() => { refresh(); });

  async function choose(id: number) { current = id; await ipc.setSoundfont(id); }
  async function add() {
    const p = await ipc.pickSoundfont();
    if (!p) return;
    const row = await ipc.loadSoundfont(p);
    await refresh();
    current = row.id;
  }
</script>

<div class="panel">
  <h3>SoundFont</h3>
  <select value={current} onchange={(e) => choose(+e.currentTarget.value)}>
    {#each fonts as f}<option value={f.id}>{f.name}{f.is_builtin ? ' ★' : ''}</option>{/each}
  </select>
  <button onclick={add}>Load .sf2…</button>
</div>

<style>
  select { width: 100%; margin-bottom: 8px; background: var(--bg); color: var(--text); border: 1px solid var(--border); border-radius: 8px; padding: 6px; }
  button { width: 100%; }
</style>
