<script lang="ts">
  import {
    pluginsList, pluginSetEnabled, pluginInstall, pluginRemove, marketplaceAdd,
    type PluginData,
  } from "../lib/api";

  let data = $state<PluginData>({ plugins: [], marketplaces: [] });
  let busy = $state(false);
  let msg = $state("");
  let installId = $state("");
  let marketRepo = $state("");
  let filter = $state("");

  async function load() {
    data = await pluginsList();
  }
  $effect(() => { load(); });

  const shown = $derived(
    filter
      ? data.plugins.filter((p) => p.id.toLowerCase().includes(filter.toLowerCase()))
      : data.plugins,
  );

  async function toggle(id: string, enabled: boolean) {
    try { await pluginSetEnabled(id, enabled); await load(); }
    catch (e) { msg = String(e); }
  }

  async function run(action: () => Promise<string>, ok: string) {
    busy = true; msg = "";
    try { const out = await action(); msg = out.trim() || ok; await load(); }
    catch (e) { msg = "error: " + String(e); }
    finally { busy = false; }
  }

  const doInstall = () => installId.trim() &&
    run(() => pluginInstall(installId.trim()), "installed").then(() => (installId = ""));
  const doAddMarket = () => marketRepo.trim() &&
    run(() => marketplaceAdd(marketRepo.trim()), "marketplace added").then(() => (marketRepo = ""));
</script>

<div class="wrap">
  <div class="scroll">
    <section>
      <h3>Install</h3>
      <div class="row">
        <input bind:value={installId} placeholder="name@marketplace" disabled={busy}
          onkeydown={(e) => e.key === "Enter" && doInstall()} />
        <button onclick={doInstall} disabled={busy || !installId.trim()}>Install</button>
      </div>
      <div class="row">
        <input bind:value={marketRepo} placeholder="add marketplace: owner/repo" disabled={busy}
          onkeydown={(e) => e.key === "Enter" && doAddMarket()} />
        <button onclick={doAddMarket} disabled={busy || !marketRepo.trim()}>Add marketplace</button>
      </div>
      {#if msg}<pre class="msg">{msg}</pre>{/if}
    </section>

    <section>
      <h3>Installed ({data.plugins.length})</h3>
      <input class="filter" bind:value={filter} placeholder="Filter…" />
      <div class="plugins">
        {#each shown as p (p.id)}
          <div class="plugin">
            <label class="tgl">
              <input type="checkbox" checked={p.enabled} onchange={(e) => toggle(p.id, e.currentTarget.checked)} />
            </label>
            <div class="meta">
              <div class="name">{p.name}<span class="mk">@{p.marketplace}</span></div>
              <div class="ver">{p.version}</div>
            </div>
            <button class="rm" disabled={busy} onclick={() => run(() => pluginRemove(p.id), "removed")}>Remove</button>
          </div>
        {/each}
      </div>
    </section>

    <section>
      <h3>Marketplaces ({data.marketplaces.length})</h3>
      <div class="markets">
        {#each data.marketplaces as m (m.name)}
          <div class="market"><span class="mname">{m.name}</span><span class="repo">{m.source}:{m.repo}</span></div>
        {/each}
      </div>
    </section>
  </div>
</div>

<style>
  .wrap { height: 100%; }
  .scroll { height: 100%; overflow-y: auto; padding: 12px 16px; }
  section { margin-bottom: 18px; }
  h3 { font-size: 12px; color: var(--fg-dim); text-transform: uppercase; letter-spacing: 0.5px; margin: 0 0 6px; }
  .row { display: flex; gap: 6px; margin-bottom: 6px; }
  .row input { flex: 1; font-family: ui-monospace, monospace; font-size: 12px; }
  .filter { width: 100%; margin-bottom: 6px; }
  .msg { background: var(--bg-alt); border: 1px solid var(--border); border-radius: 5px; padding: 6px 8px;
    font-size: 11px; white-space: pre-wrap; margin: 4px 0 0; max-height: 140px; overflow: auto; }
  .plugins, .markets { display: flex; flex-direction: column; gap: 4px; }
  .plugin { display: flex; align-items: center; gap: 10px; background: var(--bg-alt);
    border: 1px solid var(--border); border-radius: 6px; padding: 6px 10px; }
  .meta { flex: 1; min-width: 0; }
  .name { font-size: 12px; font-family: ui-monospace, monospace; }
  .mk { color: var(--fg-dim); }
  .ver { font-size: 11px; color: var(--fg-dim); }
  .rm { font-size: 11px; color: var(--warn); }
  .market { display: flex; justify-content: space-between; font-size: 12px; padding: 4px 10px;
    border: 1px solid var(--border); border-radius: 6px; }
  .mname { font-family: ui-monospace, monospace; }
  .repo { color: var(--fg-dim); }
</style>
