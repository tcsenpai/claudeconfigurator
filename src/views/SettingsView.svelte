<script lang="ts">
  import { readFile, writeRaw } from "../lib/api";
  import JsonNode from "../lib/JsonNode.svelte";

  const PATH = "settings.json";

  // Single source of truth. Both Form and Raw edit `data`; switching modes
  // carries unsaved edits because neither reloads from disk.
  let data = $state<unknown>(null);
  let mode = $state<"form" | "raw">("form");
  let dirty = $state(false);
  let saving = $state(false);
  let error = $state("");

  // Raw-mode text buffer + its own parse error (kept separate so a temporarily
  // invalid edit doesn't corrupt `data`).
  let rawText = $state("");
  let rawError = $state("");

  $effect(() => {
    readFile(PATH)
      .then((d) => { data = JSON.parse(d.raw); error = ""; })
      .catch((e) => (error = String(e)));
  });

  function onRoot(v: unknown) { data = v; dirty = true; }

  function enterRaw() {
    rawText = JSON.stringify(data, null, 2);
    rawError = "";
    mode = "raw";
  }

  function enterForm() {
    // Commit the raw buffer into `data` before showing the form.
    if (!syncRaw()) return; // stay in raw if it doesn't parse
    mode = "form";
  }

  // Parse rawText into data. Returns true on success.
  function syncRaw(): boolean {
    try {
      data = JSON.parse(rawText);
      rawError = "";
      dirty = true;
      return true;
    } catch (e) {
      rawError = String(e);
      return false;
    }
  }

  function onRawInput(text: string) {
    rawText = text;
    syncRaw(); // keep `data` live when valid; surface error when not
  }

  const canSave = $derived(dirty && !saving && (mode === "form" || !rawError));

  async function save() {
    if (!canSave) return;
    if (mode === "raw" && !syncRaw()) return;
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
      <button class:on={mode === "form"} onclick={enterForm}>Form</button>
      <button class:on={mode === "raw"} onclick={enterRaw}>Raw</button>
      <button onclick={save} disabled={!canSave}>{saving ? "Saving…" : "Save"}</button>
    </div>
  </div>

  {#if error}<div class="err">{error}</div>{/if}

  {#if data === null}
    <div class="empty">Loading…</div>
  {:else if mode === "form"}
    <div class="form"><JsonNode value={data} onChange={onRoot} /></div>
  {:else}
    <div class="raw">
      {#if rawError}<div class="err">{rawError}</div>{/if}
      <textarea
        spellcheck="false"
        value={rawText}
        oninput={(e) => onRawInput(e.currentTarget.value)}
      ></textarea>
    </div>
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
  .raw { flex: 1; display: flex; flex-direction: column; min-height: 0; }
  .raw textarea {
    flex: 1; resize: none; border: none; border-radius: 0; padding: 10px 12px;
    font-family: ui-monospace, monospace; font-size: 12px; line-height: 1.5;
  }
  .raw textarea:focus { outline: none; }
  .empty { color: var(--fg-dim); padding: 10px; }
  .err { padding: 8px 10px; background: var(--warn); color: #fff; font-size: 12px; }
</style>
