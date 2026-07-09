<script lang="ts">
  import { catalog, type Entry } from "./api";
  import DocEditor from "./DocEditor.svelte";
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

  $effect(() => { catalog().then((c) => (entries = c[kind])); });

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
    <input class="filter" bind:value={filter} placeholder="Filter…" />
    <div class="cards">
      {#each shown as e (e.path)}
        <button class="card" class:active={selected === e.path} onclick={() => (selected = e.path)}>
          <div class="name">{label(e)}</div>
          {#if e.description}<div class="desc">{e.description}</div>{/if}
        </button>
      {/each}
    </div>
  </div>
  <div class="detail">
    <DocEditor path={selected} onFollow={follow} />
  </div>
</div>

<style>
  .split { display: grid; grid-template-columns: 260px 1fr; height: 100%; }
  .list { display: flex; flex-direction: column; border-right: 1px solid var(--border); min-height: 0; }
  .filter { margin: 6px; }
  .cards { overflow-y: auto; padding: 0 6px 6px; display: flex; flex-direction: column; gap: 4px; }
  .card {
    text-align: left; background: var(--bg-alt); border: 1px solid var(--border); border-radius: 5px;
    padding: 6px 8px; cursor: pointer;
  }
  .card:hover { background: var(--bg-hover); }
  .card.active { border-color: var(--accent); }
  .name { font-size: 12px; font-family: ui-monospace, monospace; color: var(--fg); }
  .desc { font-size: 11px; color: var(--fg-dim); margin-top: 2px; line-height: 1.35;
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .detail { min-width: 0; }
</style>
