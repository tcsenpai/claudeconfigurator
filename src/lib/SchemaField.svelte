<script lang="ts">
  import type { SchemaProp } from "./api";
  import JsonNode from "./JsonNode.svelte";

  interface Props {
    key: string;
    prop: SchemaProp | undefined;
    value: unknown;
    onChange: (v: unknown) => void;
  }
  let { key, prop, value, onChange }: Props = $props();

  // Normalize `type` (may be a string or array like ["string","null"]).
  const type = $derived.by(() => {
    const t = prop?.type;
    return Array.isArray(t) ? t.find((x) => x !== "null") : t;
  });

  const isFraction = $derived(
    type === "number" &&
      (prop?.minimum ?? 0) >= 0 &&
      (prop?.maximum ?? 1) <= 1,
  );

  // Which widget to render. Anything we can't confidently type -> JsonNode.
  const widget = $derived.by((): string => {
    if (prop?.enum) return "enum";
    if (type === "boolean") return "bool";
    if (type === "integer") return "int";
    if (type === "number") return isFraction ? "fraction" : "number";
    if (type === "string") return "string";
    if (type === "array" && prop?.items?.type === "string") return "chips";
    return "fallback";
  });

  function chipsToArr(t: string) { return t.split(",").map((s) => s.trim()).filter(Boolean); }
</script>

<div class="field">
  <div class="head">
    <span class="key">{key}</span>
    {#if widget === "bool"}
      <label class="switch">
        <input type="checkbox" checked={value === true}
          onchange={(e) => onChange(e.currentTarget.checked)} />
        <span class="slider"></span>
      </label>
    {:else if widget === "int"}
      <input type="number" step="1" value={value ?? ""}
        oninput={(e) => onChange(e.currentTarget.valueAsNumber)} />
    {:else if widget === "number"}
      <input type="number" step="any" value={value ?? ""}
        oninput={(e) => onChange(e.currentTarget.valueAsNumber)} />
    {:else if widget === "fraction"}
      <span class="range">
        <input type="range" min="0" max="1" step="0.01" value={Number(value) || 0}
          oninput={(e) => onChange(e.currentTarget.valueAsNumber)} />
        <span class="rangeval">{(Number(value) || 0).toFixed(2)}</span>
      </span>
    {:else if widget === "enum"}
      <select value={String(value ?? "")} onchange={(e) => onChange(e.currentTarget.value)}>
        {#each prop?.enum ?? [] as opt}<option value={String(opt)}>{String(opt)}</option>{/each}
      </select>
    {:else if widget === "string"}
      <input value={String(value ?? "")} oninput={(e) => onChange(e.currentTarget.value)} />
    {:else if widget === "chips"}
      <input value={Array.isArray(value) ? value.join(", ") : ""}
        placeholder="comma, separated"
        oninput={(e) => onChange(chipsToArr(e.currentTarget.value))} />
    {/if}
  </div>

  {#if widget === "fallback"}
    <div class="fallback"><JsonNode {value} onChange={onChange} /></div>
  {/if}

  {#if prop?.description}
    <div class="help">{prop.description}</div>
  {/if}
</div>

<style>
  .field { padding: 8px 0; border-bottom: 1px solid var(--border); }
  .head { display: flex; align-items: center; gap: 10px; }
  .key { flex: 1; font-size: 12px; font-family: ui-monospace, monospace; color: var(--fg); }
  .head input:not([type="checkbox"]):not([type="range"]), .head select { width: 190px; }
  .help { font-size: 11px; color: var(--fg-dim); margin-top: 4px; line-height: 1.4; max-width: 640px; }
  .fallback { margin-top: 4px; }
  .range { display: flex; align-items: center; gap: 8px; width: 190px; }
  .range input { flex: 1; }
  .rangeval { font-size: 11px; color: var(--fg-dim); width: 30px; text-align: right; }

  /* toggle switch */
  .switch { position: relative; display: inline-block; width: 34px; height: 18px; }
  .switch input { opacity: 0; width: 0; height: 0; }
  .slider {
    position: absolute; inset: 0; cursor: pointer; background: #444; border-radius: 18px;
    transition: 0.15s;
  }
  .slider::before {
    content: ""; position: absolute; height: 14px; width: 14px; left: 2px; bottom: 2px;
    background: #ccc; border-radius: 50%; transition: 0.15s;
  }
  .switch input:checked + .slider { background: var(--accent); }
  .switch input:checked + .slider::before { transform: translateX(16px); background: #fff; }
</style>
