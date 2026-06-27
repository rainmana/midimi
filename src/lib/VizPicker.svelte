<script lang="ts">
  import { VIZZES } from './visualizations/registry';
  import * as ipc from './ipc';
  let { current = $bindable('cosmic-aurora') }: { current?: string } = $props();
  async function choose(id: string) { current = id; await ipc.setSetting('viz', id); }
</script>

<div class="panel">
  <h3>Visualization</h3>
  {#each VIZZES as v}
    <button class:active={current === v.id} onclick={() => choose(v.id)}>{v.name}</button>
  {/each}
</div>

<style>
  button { display: block; width: 100%; text-align: left; margin-bottom: 4px; font-size: 13px; }
  button.active { border-color: var(--accent); color: var(--accent); }
</style>
