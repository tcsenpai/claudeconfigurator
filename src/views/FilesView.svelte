<script lang="ts">
  import { listRootMd, deleteEntry, type FileItem } from "../lib/api";
  import DocEditor from "../lib/DocEditor.svelte";
  import AddDialog from "../lib/AddDialog.svelte";
  import ConfirmDialog from "../lib/ConfirmDialog.svelte";
  import { nav, follow } from "../lib/nav.svelte";

  let items = $state<FileItem[]>([]);
  let selected = $state<string | null>(null);
  let adding = $state(false);
  let pendingDelete = $state<FileItem | null>(null);

  function reload() { return listRootMd().then((f) => (items = f)); }
  $effect(() => { reload(); });

  // Honor a cross-view open request targeting a root file.
  $effect(() => {
    if (nav.request && !nav.request.includes("/")) {
      selected = nav.request;
      nav.request = null;
    }
  });

  async function onCreated(path: string) {
    adding = false;
    await reload();
    selected = path;
  }

  async function doDelete(path: string, deleteBackups = false) {
    pendingDelete = null;
    await deleteEntry(path, deleteBackups);
    if (selected === path) selected = null;
    await reload();
  }
</script>

<div class="split">
  <div class="list">
    <div class="toolbar">
      <span class="label">Files</span>
      <button class="add" title="New file" onclick={() => (adding = true)}>+</button>
    </div>
    <ul>
      {#each items as it (it.path)}
        <li class:active={selected === it.path}>
          <button class="name" onclick={() => (selected = it.path)}>{it.name}</button>
          <button class="del" title="Delete {it.name}" onclick={() => (pendingDelete = it)}>×</button>
        </li>
      {/each}
    </ul>
  </div>
  <div class="detail">
    <DocEditor
      path={selected}
      onFollow={follow}
      onDelete={selected ? (delBackups) => doDelete(selected!, delBackups) : null}
    />
  </div>
</div>

{#if adding}
  <AddDialog kind="file" onClose={() => (adding = false)} onCreated={onCreated} />
{/if}

{#if pendingDelete}
  <ConfirmDialog
    message={`Delete ${pendingDelete.path}?`}
    checkboxLabel="Delete backup history as well"
    onCancel={() => (pendingDelete = null)}
    onConfirm={(delBackups) => doDelete(pendingDelete!.path, delBackups)}
  />
{/if}

<style>
  .split { display: grid; grid-template-columns: 200px 1fr; height: 100%; }
  .list { display: flex; flex-direction: column; border-right: 1px solid var(--border); min-height: 0; }
  .list ul { list-style: none; margin: 0; padding: 0 6px 6px; overflow-y: auto; }
  .toolbar { display: flex; align-items: center; gap: 4px; padding: 6px; }
  .toolbar .label { flex: 1; font-size: 12px; color: var(--fg-dim); }
  .add { padding: 0 10px; font-size: 15px; line-height: 1; }
  .list li {
    display: flex; align-items: stretch; border-radius: 4px; overflow: hidden;
  }
  .list li:hover { background: var(--bg-hover); }
  .list li.active { background: var(--bg-hover); }
  .list li .name {
    flex: 1; min-width: 0; text-align: left; background: none; border: none; padding: 5px 8px;
    font-family: ui-monospace, monospace; font-size: 12px; cursor: pointer;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .list li.active .name { color: var(--accent); }
  .del {
    background: none; border: none; color: var(--fg-dim); padding: 0 8px; font-size: 15px; line-height: 1;
  }
  .del:hover { background: var(--warn); color: #fff; }
  .detail { min-width: 0; min-height: 0; overflow: hidden; }
</style>
