<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { scope, switchGlobal, openProject, createClaude } from "./scope.svelte";
  import { nav } from "./nav.svelte";

  let menuOpen = $state(false);
  let error = $state("");
  let confirmCreate = $state<string | null>(null);

  async function pickProject() {
    menuOpen = false; error = "";
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir !== "string") return;
    try {
      const status = await openProject(dir);
      if (status === "no-claude") confirmCreate = dir;
      else resetView();
    } catch (e) { error = String(e); }
  }

  async function doCreate() {
    const dir = confirmCreate!;
    confirmCreate = null;
    try { await createClaude(dir); resetView(); }
    catch (e) { error = String(e); }
  }

  async function goGlobal() {
    menuOpen = false; error = "";
    try { await switchGlobal(); resetView(); }
    catch (e) { error = String(e); }
  }

  // After a scope switch, land on a sane tab (Files works in every scope).
  function resetView() { nav.view = "files"; }
</script>

<div class="scope">
  <button class="current" onclick={() => (menuOpen = !menuOpen)} title={scope.info.path}>
    <span class="dot" class:project={scope.info.kind === "project"}></span>
    <span class="label">{scope.info.label}</span>
    <span class="chev">▾</span>
  </button>
  {#if menuOpen}
    <div class="menu">
      <button onclick={goGlobal}>Global (~/.claude)</button>
      <button onclick={pickProject}>Open project…</button>
    </div>
  {/if}
  {#if error}<div class="err">{error}</div>{/if}
</div>

{#if confirmCreate}
  <div class="backdrop" role="presentation">
    <div class="dialog" role="dialog" aria-modal="true" tabindex="-1">
      <p>No <code>.claude</code> folder here. Create one and edit this project's config?</p>
      <div class="path">{confirmCreate}</div>
      <div class="actions">
        <button onclick={() => (confirmCreate = null)}>Cancel</button>
        <button class="primary" onclick={doCreate}>Create .claude</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .scope { position: relative; padding: 6px; border-bottom: 1px solid var(--border); }
  .current {
    width: 100%; display: flex; align-items: center; gap: 6px; text-align: left;
    background: var(--bg); border: 1px solid var(--border); border-radius: 5px; padding: 5px 7px;
  }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--fg-dim); flex-shrink: 0; }
  .dot.project { background: var(--accent); }
  .label { flex: 1; min-width: 0; font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .chev { color: var(--fg-dim); font-size: 10px; }
  .menu {
    position: absolute; left: 6px; right: 6px; top: 100%; z-index: 20; margin-top: 2px;
    background: var(--bg); border: 1px solid var(--border); border-radius: 5px; overflow: hidden;
    display: flex; flex-direction: column;
  }
  .menu button { text-align: left; background: none; border: none; padding: 6px 8px; font-size: 12px; }
  .menu button:hover { background: var(--bg-hover); }
  .err { font-size: 10px; color: var(--warn); margin-top: 4px; }

  .backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 100; }
  .dialog { width: 380px; background: var(--bg); border: 1px solid var(--border); border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .dialog p { margin: 0; font-size: 13px; }
  .path { font-size: 11px; color: var(--fg-dim); font-family: ui-monospace, monospace; word-break: break-all; }
  .actions { display: flex; justify-content: flex-end; gap: 6px; }
  .actions .primary { border-color: var(--accent); color: var(--accent); }
</style>
