<script lang="ts">
  import { readFile, writeFile, type Field, type FileDoc } from "./api";
  import { marked } from "marked";
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
  let preview = $state(false);

  const canPreview = $derived(lang === "markdown");
  const html = $derived(preview ? marked.parse(body, { async: false }) as string : "");

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
      <div class="actions">
        {#if canPreview}
          <button class="toggle" class:on={preview} onclick={() => (preview = !preview)}>
            {preview ? "Source" : "Preview"}
          </button>
        {/if}
        <button onclick={save} disabled={!dirty || saving}>{saving ? "Saving…" : "Save"}</button>
      </div>
    </div>
    <FrontmatterEditor {fields} onChange={onFields} />
    <div class="body">
      {#if preview}
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        <div class="md">{@html html}</div>
      {:else}
        {#key doc.path}
          <EditorPane doc={body} dir={doc.dir} {lang} {onFollow} onChange={onBody} />
        {/key}
      {/if}
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
  .actions { display: flex; gap: 6px; }
  .toggle.on { border-color: var(--accent); color: var(--accent); }
  .body { flex: 1; min-height: 0; }
  .md {
    height: 100%; overflow-y: auto; padding: 12px 20px; line-height: 1.55;
  }
  .md :global(h1), .md :global(h2), .md :global(h3) { line-height: 1.25; }
  .md :global(h1) { font-size: 1.5em; border-bottom: 1px solid var(--border); padding-bottom: 0.2em; }
  .md :global(h2) { font-size: 1.25em; border-bottom: 1px solid var(--border); padding-bottom: 0.2em; }
  .md :global(code) { background: var(--bg-alt); padding: 1px 4px; border-radius: 3px; font-size: 0.9em; }
  .md :global(pre) { background: var(--bg-alt); padding: 10px; border-radius: 6px; overflow-x: auto; }
  .md :global(pre code) { background: none; padding: 0; }
  .md :global(a) { color: var(--accent); }
  .md :global(table) { border-collapse: collapse; }
  .md :global(th), .md :global(td) { border: 1px solid var(--border); padding: 3px 8px; }
  .md :global(blockquote) { border-left: 3px solid var(--border); margin: 0; padding-left: 12px; color: var(--fg-dim); }
  .empty { padding: 20px; color: var(--fg-dim); }
  .err { padding: 8px 10px; background: var(--warn); color: #fff; font-size: 12px; }
</style>
