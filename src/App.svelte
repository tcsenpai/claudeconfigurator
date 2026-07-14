<script lang="ts">
  import { views } from "./views/registry";
  import { nav } from "./lib/nav.svelte";
  import { loadAppConfig } from "./lib/appConfig.svelte";
  import { scope, loadScope, isProject } from "./lib/scope.svelte";
  import ScopeSelector from "./lib/ScopeSelector.svelte";

  loadAppConfig();
  loadScope();

  // Tabs available in the active scope (project hides global-only views).
  const shown = $derived(isProject() ? views.filter((v) => v.project) : views);

  // If the current tab isn't available in this scope, fall back to the first.
  $effect(() => {
    if (!shown.some((v) => v.id === nav.view)) nav.view = shown[0].id;
  });

  const active = $derived(shown.find((v) => v.id === nav.view) ?? shown[0]);
  const Active = $derived(active.component);
</script>

<div class="app">
  <nav class="sidebar">
    <div class="brand">.claude</div>
    <ScopeSelector />
    <div class="tabs">
      {#each shown as v (v.id)}
        <button class:active={v.id === nav.view} onclick={() => (nav.view = v.id)}>{v.label}</button>
      {/each}
    </div>
  </nav>
  <main>
    <!-- Remount on scope switch (token) so views reload against the new scope. -->
    {#key `${active.id}:${scope.token}`}
      <Active />
    {/key}
  </main>
</div>

<style>
  .app { display: grid; grid-template-columns: 150px 1fr; height: 100vh; }
  .sidebar {
    display: flex; flex-direction: column; gap: 2px; padding: 8px 6px;
    background: var(--bg-alt); border-right: 1px solid var(--border); min-height: 0;
  }
  .brand {
    font-family: ui-monospace, monospace; color: var(--accent); font-size: 13px;
    padding: 4px 8px 8px; letter-spacing: 0.5px;
  }
  .tabs { display: flex; flex-direction: column; gap: 2px; overflow-y: auto; margin-top: 4px; }
  .tabs button {
    text-align: left; background: none; border: none; padding: 6px 8px; border-radius: 5px; color: var(--fg-dim);
  }
  .tabs button:hover { background: var(--bg-hover); color: var(--fg); }
  .tabs button.active { background: var(--bg-hover); color: var(--accent); }
  main { min-width: 0; min-height: 0; overflow: hidden; }
</style>
