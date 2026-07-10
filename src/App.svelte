<script lang="ts">
  import { views } from "./views/registry";
  import { nav } from "./lib/nav.svelte";
  import { loadAppConfig } from "./lib/appConfig.svelte";

  loadAppConfig();

  const active = $derived(views.find((v) => v.id === nav.view) ?? views[0]);
  const Active = $derived(active.component);
</script>

<div class="app">
  <nav class="sidebar">
    <div class="brand">.claude</div>
    {#each views as v (v.id)}
      <button class:active={v.id === nav.view} onclick={() => (nav.view = v.id)}>{v.label}</button>
    {/each}
  </nav>
  <main>
    {#key active.id}
      <Active />
    {/key}
  </main>
</div>

<style>
  .app { display: grid; grid-template-columns: 130px 1fr; height: 100vh; }
  .sidebar {
    display: flex; flex-direction: column; gap: 2px; padding: 8px 6px;
    background: var(--bg-alt); border-right: 1px solid var(--border);
  }
  .brand {
    font-family: ui-monospace, monospace; color: var(--accent); font-size: 13px;
    padding: 4px 8px 10px; letter-spacing: 0.5px;
  }
  .sidebar button {
    text-align: left; background: none; border: none; padding: 6px 8px; border-radius: 5px; color: var(--fg-dim);
  }
  .sidebar button:hover { background: var(--bg-hover); color: var(--fg); }
  .sidebar button.active { background: var(--bg-hover); color: var(--accent); }
  /* min-height: 0 lets this grid item shrink to the 100vh row instead of
     growing to fit content, so inner overflow-y: auto panes actually scroll. */
  main { min-width: 0; min-height: 0; overflow: hidden; }
</style>
