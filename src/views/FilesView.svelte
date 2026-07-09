<script lang="ts">
  import { listRootMd, type FileItem } from "../lib/api";
  import DocEditor from "../lib/DocEditor.svelte";
  import { nav, follow } from "../lib/nav.svelte";

  let items = $state<FileItem[]>([]);
  let selected = $state<string | null>(null);

  $effect(() => { listRootMd().then((f) => (items = f)); });

  // Honor a cross-view open request targeting a root file.
  $effect(() => {
    if (nav.request && !nav.request.includes("/")) {
      selected = nav.request;
      nav.request = null;
    }
  });
</script>

<div class="split">
  <ul class="list">
    {#each items as it (it.path)}
      <li class:active={selected === it.path}>
        <button onclick={() => (selected = it.path)}>{it.name}</button>
      </li>
    {/each}
  </ul>
  <div class="detail">
    <DocEditor path={selected} onFollow={follow} />
  </div>
</div>

<style>
  .split { display: grid; grid-template-columns: 200px 1fr; height: 100%; }
  .list { list-style: none; margin: 0; padding: 6px; overflow-y: auto; border-right: 1px solid var(--border); }
  .list li button {
    width: 100%; text-align: left; background: none; border: none; padding: 5px 8px; border-radius: 4px;
    font-family: ui-monospace, monospace; font-size: 12px;
  }
  .list li.active button { background: var(--bg-hover); color: var(--accent); }
  .detail { min-width: 0; }
</style>
