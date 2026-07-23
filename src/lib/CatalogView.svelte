<script lang="ts">
  import { catalog, deleteEntry, type Entry, type Kind } from "./api";
  import DocEditor from "./DocEditor.svelte";
  import AddDialog from "./AddDialog.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import { nav, follow } from "./nav.svelte";

  interface Props {
    kind: "skills" | "commands" | "agents";
    /** Show a namespace group column (commands). */
    grouped?: boolean;
  }
  let { kind, grouped = false }: Props = $props();

  let entries = $state<Entry[]>([]);
  let selected = $state<string | null>(null);
  let filter = $state("");
  let adding = $state(false);
  let pendingDelete = $state<Entry | null>(null);

  // Plural view kind -> singular create kind.
  const addKind = $derived<Kind>(
    kind === "skills" ? "skill" : kind === "commands" ? "command" : "agent",
  );

  // What to actually delete: a skill is a folder (skills/<name>), everything
  // else is the file itself.
  function deleteTarget(e: Entry): string {
    if (kind === "skills") return e.path.replace(/\/SKILL\.md$/, "");
    return e.path;
  }

  function reload() { return catalog().then((c) => (entries = c[kind])); }
  $effect(() => { reload(); });

  async function onCreated(path: string) {
    adding = false;
    await reload();
    selected = path;
  }

  async function doDelete(e: Entry, deleteBackups = false) {
    pendingDelete = null;
    await deleteEntry(deleteTarget(e), deleteBackups);
    if (selected === e.path) selected = null;
    await reload();
  }

  const selectedEntry = $derived(entries.find((e) => e.path === selected) ?? null);

  // Cross-view open request for a path this view owns.
  $effect(() => {
    if (nav.request && nav.request.startsWith(kind + "/")) {
      selected = nav.request;
      nav.request = null;
    }
  });

  const shown = $derived(
    filter
      ? entries.filter((e) =>
          (e.name + " " + e.description + " " + e.group).toLowerCase().includes(filter.toLowerCase()),
        )
      : entries,
  );

  function label(e: Entry) {
    return grouped && e.group ? `${e.group}/${e.name}` : e.name;
  }
</script>

<div class="split">
  <div class="list">
    <div class="toolbar">
      <input class="filter" bind:value={filter} placeholder="Filter…" />
      <button class="add" title="New {addKind}" onclick={() => (adding = true)}>+</button>
    </div>
    <div class="cards">
      {#each shown as e (e.path)}
        <div class="row" class:active={selected === e.path}>
          <button class="card" onclick={() => (selected = e.path)}>
            <div class="name">{label(e)}</div>
            {#if e.description}<div class="desc">{e.description}</div>{/if}
          </button>
          <button class="del" title="Delete {e.name}" onclick={() => (pendingDelete = e)}>×</button>
        </div>
      {/each}
    </div>
  </div>
  <div class="detail">
    <DocEditor
      path={selected}
      onFollow={follow}
      onDelete={selectedEntry ? (delBackups) => doDelete(selectedEntry, delBackups) : null}
    />
  </div>
</div>

{#if adding}
  <AddDialog kind={addKind} onClose={() => (adding = false)} onCreated={onCreated} />
{/if}

{#if pendingDelete}
  <ConfirmDialog
    message={`Delete ${deleteTarget(pendingDelete)}?`}
    checkboxLabel="Delete backup history as well"
    onCancel={() => (pendingDelete = null)}
    onConfirm={(delBackups) => doDelete(pendingDelete!, delBackups)}
  />
{/if}

<style>
  .split { display: grid; grid-template-columns: 260px 1fr; height: 100%; }
  .list { display: flex; flex-direction: column; border-right: 1px solid var(--border); min-height: 0; }
  .toolbar { display: flex; gap: 4px; margin: 6px; }
  .filter { flex: 1; }
  .add { padding: 0 10px; font-size: 15px; line-height: 1; }
  .cards { overflow-y: auto; padding: 0 6px 6px; display: flex; flex-direction: column; gap: 4px; }
  .row {
    display: flex; align-items: stretch; flex-shrink: 0; background: var(--bg-alt);
    border: 1px solid var(--border); border-radius: 5px; overflow: hidden;
  }
  .row:hover { background: var(--bg-hover); }
  .row.active { border-color: var(--accent); }
  .card {
    flex: 1; min-width: 0; text-align: left; background: none; border: none; border-radius: 0;
    padding: 6px 8px; cursor: pointer;
  }
  .del {
    background: none; border: none; border-radius: 0; color: var(--fg-dim);
    padding: 0 9px; font-size: 15px; line-height: 1;
  }
  .del:hover { background: var(--warn); color: #fff; }
  .name { font-size: 12px; font-family: ui-monospace, monospace; color: var(--fg); }
  .desc { font-size: 11px; color: var(--fg-dim); margin-top: 2px; line-height: 1.35;
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  /* min-height: 0 lets the grid item shrink to the row so the editor's
     internal scroller gets a bounded height instead of growing to content. */
  .detail { min-width: 0; min-height: 0; overflow: hidden; }
</style>
