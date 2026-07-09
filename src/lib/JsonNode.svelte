<script lang="ts">
  import Self from "./JsonNode.svelte";

  // A recursive editor for arbitrary JSON. `value` is any JSON value; `onChange`
  // bubbles a replacement value up to the parent, which rebuilds its own value.
  interface Props {
    value: unknown;
    label?: string;
    onChange: (v: unknown) => void;
    onRemove?: () => void;
  }
  let { value, label, onChange, onRemove }: Props = $props();

  type Kind = "object" | "array" | "string" | "number" | "boolean" | "null";
  const kind = (v: unknown): Kind =>
    v === null ? "null"
    : Array.isArray(v) ? "array"
    : typeof v === "object" ? "object"
    : (typeof v as Kind);

  const k = $derived(kind(value));

  // --- object helpers ---
  const entries = $derived(k === "object" ? Object.entries(value as Record<string, unknown>) : []);
  function setKey(key: string, v: unknown) {
    onChange({ ...(value as object), [key]: v });
  }
  function removeKey(key: string) {
    const next = { ...(value as Record<string, unknown>) };
    delete next[key];
    onChange(next);
  }
  let newKey = $state("");
  function addKey() {
    if (!newKey.trim()) return;
    setKey(newKey.trim(), "");
    newKey = "";
  }

  // --- array helpers ---
  const items = $derived(k === "array" ? (value as unknown[]) : []);
  function setItem(i: number, v: unknown) {
    const next = [...(value as unknown[])];
    next[i] = v;
    onChange(next);
  }
  function removeItem(i: number) {
    onChange((value as unknown[]).filter((_, j) => j !== i));
  }
  function addItem() {
    onChange([...(value as unknown[]), ""]);
  }

  // Change a leaf's type via the dropdown (helps build nested structures).
  function retype(next: Kind) {
    const map: Record<Kind, unknown> = {
      object: {}, array: [], string: "", number: 0, boolean: false, null: null,
    };
    onChange(map[next]);
  }
</script>

<div class="node">
  <div class="head">
    {#if label !== undefined}<span class="lbl">{label}</span>{/if}
    {#if k === "string"}
      <input value={value as string} oninput={(e) => onChange(e.currentTarget.value)} />
    {:else if k === "number"}
      <input type="number" value={value as number}
        oninput={(e) => onChange(e.currentTarget.valueAsNumber)} />
    {:else if k === "boolean"}
      <input type="checkbox" checked={value as boolean}
        onchange={(e) => onChange(e.currentTarget.checked)} />
    {:else if k === "null"}
      <span class="null">null</span>
    {:else}
      <span class="badge">{k}{k === "array" ? ` [${items.length}]` : ` {${entries.length}}`}</span>
    {/if}

    <select class="type" value={k} onchange={(e) => retype(e.currentTarget.value as Kind)}>
      {#each ["string", "number", "boolean", "null", "object", "array"] as t}
        <option value={t}>{t}</option>
      {/each}
    </select>
    {#if onRemove}<button class="x" onclick={onRemove} title="remove">×</button>{/if}
  </div>

  {#if k === "object"}
    <div class="children">
      {#each entries as [key, v] (key)}
        <Self value={v} label={key} onChange={(nv) => setKey(key, nv)} onRemove={() => removeKey(key)} />
      {/each}
      <div class="add">
        <input placeholder="new key" bind:value={newKey}
          onkeydown={(e) => e.key === "Enter" && addKey()} />
        <button onclick={addKey}>+ key</button>
      </div>
    </div>
  {:else if k === "array"}
    <div class="children">
      {#each items as v, i (i)}
        <Self value={v} label={`[${i}]`} onChange={(nv) => setItem(i, nv)} onRemove={() => removeItem(i)} />
      {/each}
      <button class="additem" onclick={addItem}>+ item</button>
    </div>
  {/if}
</div>

<style>
  .node { border-left: 1px solid var(--border); padding-left: 8px; margin: 2px 0; }
  .head { display: flex; align-items: center; gap: 6px; min-height: 24px; }
  .lbl { color: var(--fg-dim); font-family: ui-monospace, monospace; font-size: 12px; min-width: 90px; }
  .badge { font-size: 11px; color: var(--fg-dim); }
  .null { color: var(--fg-dim); font-style: italic; }
  .type { font-size: 11px; background: var(--bg); color: var(--fg-dim); border: 1px solid var(--border); border-radius: 3px; }
  .x { padding: 0 6px; color: var(--warn); background: none; border: none; font-size: 15px; }
  .children { margin-left: 8px; }
  .add { display: flex; gap: 4px; margin: 4px 0; }
  .additem { font-size: 11px; }
  input:not([type]) { flex: 1; min-width: 0; }
</style>
