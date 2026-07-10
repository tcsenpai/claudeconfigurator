<script lang="ts">
  import { mcpList, mcpUpsert, mcpRemove, mcpSetEnabled, type McpServer } from "../lib/api";
  import JsonNode from "../lib/JsonNode.svelte";
  import ConfirmDialog from "../lib/ConfirmDialog.svelte";

  let servers = $state<McpServer[]>([]);
  let error = $state("");
  let filter = $state("");

  // Editor dialog state (add or edit one server).
  let editing = $state<{ name: string; config: Record<string, unknown>; isNew: boolean } | null>(null);
  let editName = $state("");
  let pendingDelete = $state<string | null>(null);

  async function load() {
    try { servers = await mcpList(); error = ""; }
    catch (e) { error = String(e); }
  }
  $effect(() => { load(); });

  const shown = $derived(
    filter ? servers.filter((s) => s.name.toLowerCase().includes(filter.toLowerCase())) : servers,
  );

  function summary(cfg: Record<string, unknown>): string {
    if (typeof cfg.url === "string") return `${cfg.type ?? "http"} · ${cfg.url}`;
    const args = Array.isArray(cfg.args) ? " " + cfg.args.join(" ") : "";
    return `${cfg.command ?? "?"}${args}`;
  }

  function openNew() {
    editName = "";
    editing = { name: "", config: { command: "", args: [] }, isNew: true };
  }
  function openEdit(s: McpServer) {
    editName = s.name;
    editing = { name: s.name, config: structuredClone(s.config), isNew: false };
  }

  async function saveEdit() {
    if (!editing) return;
    const name = editName.trim();
    if (!name) { error = "name required"; return; }
    const enabled = editing.isNew ? true : servers.find((s) => s.name === editing!.name)?.enabled ?? true;
    try {
      // If renamed, drop the old entry.
      if (!editing.isNew && editing.name !== name) await mcpRemove(editing.name);
      await mcpUpsert(name, editing.config, enabled);
      editing = null;
      await load();
    } catch (e) { error = String(e); }
  }

  async function toggle(s: McpServer) {
    try { await mcpSetEnabled(s.name, !s.enabled); await load(); }
    catch (e) { error = String(e); }
  }
  async function doDelete(name: string) {
    pendingDelete = null;
    try { await mcpRemove(name); await load(); }
    catch (e) { error = String(e); }
  }
</script>

<div class="wrap">
  <div class="bar">
    <input class="filter" bind:value={filter} placeholder="Filter…" />
    <button class="add" onclick={openNew}>+ Add server</button>
  </div>
  {#if error}<div class="err">{error}</div>{/if}

  <div class="list">
    {#each shown as s (s.name)}
      <div class="card" class:off={!s.enabled}>
        <label class="switch" title={s.enabled ? "enabled" : "disabled"}>
          <input type="checkbox" checked={s.enabled} onchange={() => toggle(s)} />
          <span class="slider"></span>
        </label>
        <div class="meta">
          <div class="name">{s.name}</div>
          <div class="sum">{summary(s.config)}</div>
        </div>
        <button onclick={() => openEdit(s)}>Edit</button>
        <button class="rm" onclick={() => (pendingDelete = s.name)}>Remove</button>
      </div>
    {/each}
    {#if !shown.length}<div class="empty">No MCP servers.</div>{/if}
  </div>
</div>

{#if editing}
  <div class="backdrop" onclick={(e) => e.target === e.currentTarget && (editing = null)} role="presentation">
    <div class="dialog" role="dialog" aria-modal="true" tabindex="-1">
      <div class="head">{editing.isNew ? "Add" : "Edit"} MCP server</div>
      <label class="field"><span>Name</span>
        <input bind:value={editName} placeholder="my-server" /></label>
      <div class="cfg">
        <div class="cfglabel">Config (stdio: command/args/env — remote: type/url/headers)</div>
        <JsonNode value={editing.config} onChange={(v) => (editing!.config = v as Record<string, unknown>)} />
      </div>
      <div class="actions">
        <button onclick={() => (editing = null)}>Cancel</button>
        <button class="primary" onclick={saveEdit}>Save</button>
      </div>
    </div>
  </div>
{/if}

{#if pendingDelete}
  <ConfirmDialog
    message={`Remove MCP server "${pendingDelete}"? ~/.claude.json is backed up first.`}
    onCancel={() => (pendingDelete = null)}
    onConfirm={() => doDelete(pendingDelete!)}
  />
{/if}

<style>
  .wrap { display: flex; flex-direction: column; height: 100%; }
  .bar { display: flex; gap: 8px; padding: 8px 12px; border-bottom: 1px solid var(--border); background: var(--bg-alt); }
  .filter { flex: 1; }
  .err { padding: 8px 12px; background: var(--warn); color: #fff; font-size: 12px; }
  .list { flex: 1; overflow-y: auto; padding: 10px 12px; display: flex; flex-direction: column; gap: 6px; }
  .card {
    display: flex; align-items: center; gap: 12px; background: var(--bg-alt);
    border: 1px solid var(--border); border-radius: 6px; padding: 8px 12px;
  }
  .card.off { opacity: 0.55; }
  .meta { flex: 1; min-width: 0; }
  .name { font-size: 12px; font-family: ui-monospace, monospace; }
  .sum { font-size: 11px; color: var(--fg-dim); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .rm { color: var(--warn); border-color: var(--warn); }
  .empty { color: var(--fg-dim); padding: 10px; }

  .backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 100; }
  .dialog { width: 460px; max-height: 80vh; overflow-y: auto; background: var(--bg); border: 1px solid var(--border); border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .head { font-size: 13px; color: var(--accent); }
  .field { display: grid; grid-template-columns: 60px 1fr; align-items: center; gap: 8px; }
  .field span { color: var(--fg-dim); font-size: 12px; }
  .cfglabel { font-size: 11px; color: var(--fg-dim); margin-bottom: 4px; }
  .cfg { border-top: 1px solid var(--border); padding-top: 8px; }
  .actions { display: flex; justify-content: flex-end; gap: 6px; }
  .actions .primary { border-color: var(--accent); color: var(--accent); }

  .switch { position: relative; display: inline-block; width: 34px; height: 18px; flex-shrink: 0; }
  .switch input { opacity: 0; width: 0; height: 0; }
  .slider { position: absolute; inset: 0; cursor: pointer; background: #444; border-radius: 18px; transition: 0.15s; }
  .slider::before { content: ""; position: absolute; height: 14px; width: 14px; left: 2px; bottom: 2px; background: #ccc; border-radius: 50%; transition: 0.15s; }
  .switch input:checked + .slider { background: var(--accent); }
  .switch input:checked + .slider::before { transform: translateX(16px); background: #fff; }
</style>
