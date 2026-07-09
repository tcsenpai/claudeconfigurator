<script lang="ts">
  import { readFile, writeFile, type Field, type FileDoc } from "./api";
  import FrontmatterEditor from "./FrontmatterEditor.svelte";
  import EditorPane from "./EditorPane.svelte";

  interface Props {
    path: string | null;
    lang?: "markdown" | "json";
    validateJson?: boolean;
    onFollow?: (target: string) => void;
  }
  let { path, lang = "markdown", validateJson = false, onFollow = () => {} }: Props = $props();

  let doc = $state<FileDoc | null>(null);
  let fields = $state<Field[]>([]);
  let body = $state("");
  let dirty = $state(false);
  let saving = $state(false);
  let error = $state("");

  // Load whenever `path` changes.
  $effect(() => {
    const p = path;
    doc = null; error = ""; dirty = false;
    if (!p) return;
    readFile(p)
      .then((d) => { doc = d; fields = d.fields; body = d.body; })
      .catch((e) => (error = String(e)));
  });

  function onFields(next: Field[]) { fields = next; dirty = true; }
  function onBody(next: string) { body = next; dirty = true; }

  async function save() {
    if (!path || saving) return;
    saving = true; error = "";
    try {
      await writeFile(path, fields, body, validateJson);
      dirty = false;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

{#if error}
  <div class="err">{error}</div>
{/if}

{#if doc}
  <div class="doc">
    <div class="bar">
      <span class="path">{doc.path}{dirty ? " •" : ""}</span>
      <button onclick={save} disabled={!dirty || saving}>{saving ? "Saving…" : "Save"}</button>
    </div>
    <FrontmatterEditor {fields} onChange={onFields} />
    <div class="body">
      {#key doc.path}
        <EditorPane doc={body} dir={doc.dir} {lang} {onFollow} onChange={onBody} />
      {/key}
    </div>
  </div>
{:else if path}
  <div class="empty">Loading…</div>
{:else}
  <div class="empty">Select a file</div>
{/if}

<style>
  .doc { display: flex; flex-direction: column; height: 100%; }
  .bar {
    display: flex; justify-content: space-between; align-items: center;
    padding: 6px 10px; border-bottom: 1px solid var(--border); background: var(--bg-alt);
  }
  .path { color: var(--fg-dim); font-size: 12px; font-family: ui-monospace, monospace; }
  .body { flex: 1; min-height: 0; }
  .empty { padding: 20px; color: var(--fg-dim); }
  .err { padding: 8px 10px; background: var(--warn); color: #fff; font-size: 12px; }
</style>
