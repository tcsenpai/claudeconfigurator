<script lang="ts">
  import { listRootMd, type FileItem } from "../lib/api";
  import DocEditor from "../lib/DocEditor.svelte";
  import AddDialog from "../lib/AddDialog.svelte";
  import { nav, follow } from "../lib/nav.svelte";

  let items = $state<FileItem[]>([]);
  let selected = $state<string | null>(null);
  let adding = $state(false);

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
          <button onclick={() => (selected = it.path)}>{it.name}</button>
        </li>
      {/each}
    </ul>
  </div>
  <div class="detail">
    <DocEditor path={selected} onFollow={follow} />
  </div>
</div>

{#if adding}
  <AddDialog kind="file" onClose={() => (adding = false)} onCreated={onCreated} />
{/if}

<style>
  .split { display: grid; grid-template-columns: 200px 1fr; height: 100%; }
  .list { display: flex; flex-direction: column; border-right: 1px solid var(--border); min-height: 0; }
  .list ul { list-style: none; margin: 0; padding: 0 6px 6px; overflow-y: auto; }
  .toolbar { display: flex; align-items: center; gap: 4px; padding: 6px; }
  .toolbar .label { flex: 1; font-size: 12px; color: var(--fg-dim); }
  .add { padding: 0 10px; font-size: 15px; line-height: 1; }
  .list li button {
    width: 100%; text-align: left; background: none; border: none; padding: 5px 8px; border-radius: 4px;
    font-family: ui-monospace, monospace; font-size: 12px;
  }
  .list li.active button { background: var(--bg-hover); color: var(--accent); }
  .detail { min-width: 0; }
</style>
