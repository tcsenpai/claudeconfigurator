<script lang="ts">
  import { readFile, writeRaw, settingsSchema, type JsonSchema } from "../lib/api";
  import SchemaField from "../lib/SchemaField.svelte";
  import JsonNode from "../lib/JsonNode.svelte";
  import { assignGroups, TAB_KEYS } from "../lib/settingsGroups";
  import { nav } from "../lib/nav.svelte";
  import HistoryDialog from "../lib/HistoryDialog.svelte";

  const PATH = "settings.json";

  // Single source of truth. Form and Raw both edit `data`.
  let data = $state<Record<string, unknown> | null>(null);
  let schema = $state<JsonSchema>({});
  let mode = $state<"form" | "raw">("form");
  let dirty = $state(false);
  let saving = $state(false);
  let error = $state("");
  let historyOpen = $state(false);

  let rawText = $state("");
  let rawError = $state("");

  $effect(() => {
    readFile(PATH)
      .then((d) => { data = JSON.parse(d.raw); error = ""; })
      .catch((e) => (error = String(e)));
    settingsSchema().then((s) => (schema = s)).catch(() => (schema = {}));
  });

  const groups = $derived(data ? assignGroups(Object.keys(data)) : []);

  function setKey(key: string, value: unknown) {
    data = { ...(data as object), [key]: value };
    dirty = true;
  }

  function enterRaw() { rawText = JSON.stringify(data, null, 2); rawError = ""; mode = "raw"; }
  function enterForm() { if (syncRaw()) mode = "form"; }

  function syncRaw(): boolean {
    try { data = JSON.parse(rawText); rawError = ""; dirty = true; return true; }
    catch (e) { rawError = String(e); return false; }
  }
  function onRawInput(text: string) { rawText = text; syncRaw(); }

  const canSave = $derived(dirty && !saving && (mode === "form" || !rawError));

  async function save() {
    if (!canSave) return;
    if (mode === "raw" && !syncRaw()) return;
    saving = true; error = "";
    try {
      await writeRaw(PATH, JSON.stringify(data, null, 2) + "\n", true);
      dirty = false;
    } catch (e) { error = String(e); }
    finally { saving = false; }
  }
</script>

<div class="wrap">
  <div class="bar">
    <span class="path">{PATH}{dirty ? " •" : ""}</span>
    <div class="actions">
      <button class:on={mode === "form"} onclick={enterForm}>Form</button>
      <button class:on={mode === "raw"} onclick={enterRaw}>Raw</button>
      <button onclick={() => (historyOpen = true)}>History</button>
      <button onclick={save} disabled={!canSave}>{saving ? "Saving…" : "Save"}</button>
    </div>
  </div>

  {#if error}<div class="err">{error}</div>{/if}

  {#if data === null}
    <div class="empty">Loading…</div>
  {:else if mode === "form"}
    <div class="form">
      {#each groups as g (g.def.id)}
        <section>
          <h3>{g.def.title}</h3>
          {#each g.keys as key (key)}
            {#if TAB_KEYS[key]}
              <div class="tabptr">
                <span class="key">{key}</span>
                <button onclick={() => (nav.view = TAB_KEYS[key].toLowerCase())}>
                  Edit in {TAB_KEYS[key]} tab →
                </button>
              </div>
            {:else}
              <SchemaField
                {key}
                prop={schema.properties?.[key]}
                value={data[key]}
                onChange={(v) => setKey(key, v)}
              />
            {/if}
          {/each}
        </section>
      {/each}
      <!-- feedbackSurveyState and other runtime keys not in any group are
           already covered by the "Other" group via assignGroups. -->
    </div>
  {:else}
    <div class="raw">
      {#if rawError}<div class="err">{rawError}</div>{/if}
      <textarea spellcheck="false" value={rawText}
        oninput={(e) => onRawInput(e.currentTarget.value)}></textarea>
    </div>
  {/if}
</div>

{#if historyOpen && data}
  <HistoryDialog
    path={PATH}
    currentValue={JSON.stringify(data, null, 2)}
    onClose={() => (historyOpen = false)}
    onRestore={(restored) => {
      data = JSON.parse(restored);
      rawText = restored;
      dirty = false;
      historyOpen = false;
    }}
  />
{/if}

<style>
  .wrap { display: flex; flex-direction: column; height: 100%; }
  .bar {
    display: flex; justify-content: space-between; align-items: center;
    padding: 6px 10px; border-bottom: 1px solid var(--border); background: var(--bg-alt);
  }
  .path { color: var(--fg-dim); font-size: 12px; font-family: ui-monospace, monospace; }
  .actions { display: flex; gap: 6px; }
  .actions button.on { border-color: var(--accent); color: var(--accent); }
  .form { flex: 1; overflow-y: auto; padding: 4px 16px 24px; }
  section { margin: 14px 0; }
  section h3 {
    font-size: 12px; color: var(--accent); text-transform: uppercase; letter-spacing: 0.5px;
    margin: 0 0 4px; border-bottom: 1px solid var(--border); padding-bottom: 4px;
  }
  .tabptr {
    display: flex; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 1px solid var(--border);
  }
  .tabptr .key { flex: 1; font-size: 12px; font-family: ui-monospace, monospace; }
  .raw { flex: 1; display: flex; flex-direction: column; min-height: 0; }
  .raw textarea {
    flex: 1; resize: none; border: none; padding: 10px 12px;
    font-family: ui-monospace, monospace; font-size: 12px; line-height: 1.5;
  }
  .raw textarea:focus { outline: none; }
  .empty { color: var(--fg-dim); padding: 10px; }
  .err { padding: 8px 10px; background: var(--warn); color: #fff; font-size: 12px; }
</style>
