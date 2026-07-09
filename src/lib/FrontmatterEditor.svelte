<script lang="ts">
  import type { Field } from "./api";

  interface Props {
    fields: Field[];
    onChange: (fields: Field[]) => void;
  }
  let { fields, onChange }: Props = $props();

  function setScalar(i: number, value: string) {
    const next = fields.map((f, j): Field =>
      j === i ? { kind: "scalar", key: f.key, value } : f,
    );
    onChange(next);
  }
  function setList(i: number, value: string[]) {
    const next = fields.map((f, j): Field =>
      j === i ? { kind: "list", key: f.key, value } : f,
    );
    onChange(next);
  }
  // Chip input: comma/enter separated -> string[].
  function listToText(v: string[]) { return v.join(", "); }
  function textToList(t: string) {
    return t.split(",").map((s) => s.trim()).filter(Boolean);
  }
</script>

{#if fields.length}
  <div class="fm">
    {#each fields as f, i (f.key)}
      <label class="row">
        <span class="key">{f.key}</span>
        {#if f.kind === "scalar"}
          <input value={f.value} oninput={(e) => setScalar(i, e.currentTarget.value)} />
        {:else if f.kind === "list"}
          <input
            value={listToText(f.value)}
            oninput={(e) => setList(i, textToList(e.currentTarget.value))}
            placeholder="comma, separated"
          />
        {:else}
          <textarea class="raw" readonly rows={Math.min(4, f.value.split("\n").length)}>{f.value}</textarea>
        {/if}
      </label>
    {/each}
  </div>
{/if}

<style>
  .fm {
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 5px;
    max-height: 40vh;
    overflow-y: auto;
  }
  .row { display: grid; grid-template-columns: 130px 1fr; align-items: center; gap: 8px; }
  .key { color: var(--fg-dim); font-size: 12px; text-align: right; }
  input, .raw { width: 100%; }
  .raw { font-family: ui-monospace, monospace; font-size: 12px; resize: vertical; }
</style>
