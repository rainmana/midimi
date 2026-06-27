<script lang="ts">
  import { THEMES, applyTheme } from './theme';
  import * as ipc from './ipc';
  let { current = $bindable('cosmic') }: { current?: string } = $props();
  async function choose(id: string) { current = applyTheme(id); await ipc.setSetting('theme', current); }
</script>

<div class="panel">
  <h3>Theme</h3>
  <div class="row">
    {#each THEMES as t}
      <button class:active={current === t.id} onclick={() => choose(t.id)}>{t.name}</button>
    {/each}
  </div>
</div>

<style>
  .row { display: flex; flex-wrap: wrap; gap: 6px; }
  button { font-size: 12px; padding: 6px 10px; }
  button.active { border-color: var(--accent); color: var(--accent); }
</style>
