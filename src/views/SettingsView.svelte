<script lang="ts">
  import { readFile, writeRaw } from "../lib/api";
  import JsonNode from "../lib/JsonNode.svelte";
  import DocEditor from "../lib/DocEditor.svelte";

  const PATH = "settings.json";
  let data = $state<unknown>(null);
  let mode = $state<"form" | "raw">("form");
  let dirty = $state(false);
  let saving = $state(false);
  let error = $state("");

  // Load once into the form model.
  $effect(() => {
    readFile(PATH)
      .then((d) => { data = JSON.parse(d.raw); error = ""; })
      .catch((e) => (error = String(e)));
  });

  function onRoot(v: unknown) { data = v; dirty = true; }

  async function save() {
    if (saving) return;
    saving = true; error = "";
    try {
      await writeRaw(PATH, JSON.stringify(data, null, 2) + "\n", true);
      dirty = false;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="wrap">
  <div class="bar">
    <span class="path">{PATH}{dirty ? " •" : ""}</span>
    <div class="actions">
      <button class:on={mode === "form"} onclick={() => (mode = "form")}>Form</button>
      <button class:on={mode === "raw"} onclick={() => (mode = "raw")}>Raw</button>
      {#if mode === "form"}
        <button onclick={save} disabled={!dirty || saving}>{saving ? "Saving…" : "Save"}</button>
      {/if}
    </div>
  </div>

  {#if error}<div class="err">{error}</div>{/if}

  {#if mode === "form"}
    <div class="form">
      {#if data !== null}
        <JsonNode value={data} onChange={onRoot} />
      {:else}
        <div class="empty">Loading…</div>
      {/if}
    </div>
  {:else}
    <!-- Raw mode reuses the validated JSON editor. -->
    <div class="raw"><DocEditor path={PATH} lang="json" validateJson /></div>
  {/if}
</div>

<style>
  .wrap { display: flex; flex-direction: column; height: 100%; }
  .bar {
    display: flex; justify-content: space-between; align-items: center;
    padding: 6px 10px; border-bottom: 1px solid var(--border); background: var(--bg-alt);
  }
  .path { color: var(--fg-dim); font-size: 12px; font-family: ui-monospace, monospace; }
  .actions { display: flex; gap: 6px; }
  .actions button.on { border-color: var(--accent); color: var(--accent); }
  .form { flex: 1; overflow-y: auto; padding: 10px 12px; }
  .raw { flex: 1; min-height: 0; }
  .empty { color: var(--fg-dim); padding: 10px; }
  .err { padding: 8px 10px; background: var(--warn); color: #fff; font-size: 12px; }
</style>
