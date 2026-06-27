<script lang="ts">
  import * as ipc from '$lib/ipc';
  import VizCanvas from '$lib/VizCanvas.svelte';
  import Transport from '$lib/Transport.svelte';
  import TrackList from '$lib/TrackList.svelte';
  import SoundfontPicker from '$lib/SoundfontPicker.svelte';
  import ThemePicker from '$lib/ThemePicker.svelte';
  import VizPicker from '$lib/VizPicker.svelte';
  import { applyTheme } from '$lib/theme';
  import type { MidiData, Playhead, LibraryRow } from '$lib/types';

  let midi = $state<MidiData | null>(null);
  let head = $state<Playhead | null>(null);
  let recent = $state<LibraryRow[]>([]);
  let theme = $state('cosmic');
  let vizId = $state('cosmic-aurora');
  let panelsOpen = $state(true);

  $effect(() => {
    let un: (() => void) | undefined;
    ipc.listenPlayhead((p) => (head = p)).then((f) => (un = f));
    return () => un?.();
  });

  // Startup: restore theme + recent list. Auto-open demo track on first run.
  $effect(() => {
    (async () => {
      const settings = await ipc.getSettings();
      theme = applyTheme(settings.find((s) => s.key === 'theme')?.value ?? 'cosmic');
      vizId = settings.find((s) => s.key === 'viz')?.value ?? 'cosmic-aurora';
      recent = await ipc.listRecent();
      if (recent.length === 0) {
        try { await openPath(await ipc.demoPath()); } catch {}
      }
    })();
  });

  async function openPath(path: string) { midi = await ipc.openMidi(path); recent = await ipc.listRecent(); }
  async function openFile() { const p = await ipc.pickMidi(); if (p) await openPath(p); }
</script>

<VizCanvas {midi} {head} {vizId} />

<header class="topbar">
  <div class="brand"><span class="bdot"></span> midimi</div>
  <div class="now">{midi?.title ?? (midi ? 'Untitled' : 'Open a MIDI to begin')}</div>
  <button class="ghost" onclick={() => (panelsOpen = !panelsOpen)} title="Toggle panels">{panelsOpen ? '⟩' : '⟨'}</button>
</header>

{#if panelsOpen}
<aside class="panels">
  <button class="open" onclick={openFile}>＋ Open MIDI…</button>
  <TrackList {midi} />
  <SoundfontPicker />
  <VizPicker bind:current={vizId} />
  <ThemePicker bind:current={theme} />
  <div class="panel">
    <h3>Recent</h3>
    {#each recent as r}
      <button class="recent" onclick={() => openPath(r.path)}>{r.title ?? r.path.split('/').pop()}</button>
    {/each}
    {#if recent.length === 0}<p class="muted">Nothing yet.</p>{/if}
  </div>
</aside>
{/if}

<Transport {midi} {head} />

<style>
  .topbar { position: fixed; top: 14px; left: 16px; right: 16px; z-index: 10; display: flex; align-items: center; gap: 14px; pointer-events: none; }
  .brand { display: flex; align-items: center; gap: 8px; font-weight: 600; letter-spacing: 1px; font-size: 18px; }
  .bdot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 12px var(--accent); }
  .now { flex: 1; color: var(--muted); font-size: 13px; }
  .ghost { pointer-events: auto; background: transparent; border: none; color: var(--muted); }
  .panels { position: fixed; top: 56px; right: 16px; bottom: 84px; width: 250px; z-index: 10;
    display: flex; flex-direction: column; gap: 12px; overflow-y: auto; }
  .open { width: 100%; padding: 10px; font-weight: 600; }
  .recent { width: 100%; text-align: left; margin-bottom: 4px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 12px; }
</style>
