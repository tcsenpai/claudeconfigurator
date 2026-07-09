<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { createEntry, importFile, importSkillDir, type Kind } from "./api";

  interface Props {
    kind: Kind;
    onClose: () => void;
    onCreated: (path: string) => void;
  }
  let { kind, onClose, onCreated }: Props = $props();

  let mode = $state<"new" | "import">("new");
  let name = $state("");
  let namespace = $state("");
  let src = $state("");
  let importFolder = $state(false); // skills only: import a whole folder
  let busy = $state(false);
  let error = $state("");

  const kindLabel: Record<Kind, string> = {
    skill: "skill", command: "command", agent: "agent", file: "file",
  };
  const showNamespace = $derived(kind === "command");
  const canImportFolder = $derived(kind === "skill");

  // Folder import derives the name from the folder; otherwise a name is needed.
  const needName = $derived(!(mode === "import" && importFolder));
  const ready = $derived(
    !busy &&
    (needName ? !!name.trim() : true) &&
    (mode === "new" || !!src),
  );

  async function pickSource() {
    const folder = importFolder;
    const picked = await open({
      multiple: false,
      directory: folder,
      filters: folder ? undefined : [{ name: "Markdown", extensions: ["md"] }],
    });
    if (typeof picked === "string") {
      src = picked;
      // Default the name from the picked file/folder if empty.
      if (!name.trim()) {
        const base = picked.split("/").pop() ?? "";
        name = base.replace(/\.md$/, "");
      }
    }
  }

  async function submit() {
    if (!ready) return;
    busy = true; error = "";
    try {
      let path: string;
      if (mode === "new") {
        path = await createEntry(kind, name.trim(), namespace.trim() || undefined);
      } else if (importFolder && kind === "skill") {
        path = await importSkillDir(name.trim() || deriveName(src), src);
      } else {
        path = await importFile(kind, name.trim(), src, namespace.trim() || undefined);
      }
      onCreated(path);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function deriveName(p: string) {
    return (p.split("/").pop() ?? "imported").replace(/\.md$/, "");
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onClose()} />
<!-- Close only when the backdrop itself (not a child) is clicked. -->
<div class="backdrop" onclick={(e) => e.target === e.currentTarget && onClose()} role="presentation">
  <div class="dialog" role="dialog" aria-modal="true" tabindex="-1">
    <div class="head">New {kindLabel[kind]}</div>

    <div class="tabs">
      <button class:on={mode === "new"} onclick={() => (mode = "new")}>New</button>
      <button class:on={mode === "import"} onclick={() => (mode = "import")}>Import</button>
    </div>

    {#if mode === "import"}
      {#if canImportFolder}
        <label class="chk">
          <input type="checkbox" bind:checked={importFolder} /> import whole skill folder
        </label>
      {/if}
      <div class="row">
        <button onclick={pickSource}>Choose {importFolder ? "folder" : "file"}…</button>
        <span class="src" title={src}>{src || "no source selected"}</span>
      </div>
    {/if}

    {#if needName}
      <label class="field">
        <span>Name</span>
        <input bind:value={name} placeholder="my-{kindLabel[kind]}"
          onkeydown={(e) => e.key === "Enter" && submit()} />
      </label>
    {/if}

    {#if showNamespace}
      <label class="field">
        <span>Namespace</span>
        <input bind:value={namespace} placeholder="optional (e.g. dev)" />
      </label>
    {/if}

    {#if error}<div class="err">{error}</div>{/if}

    <div class="actions">
      <button onclick={onClose}>Cancel</button>
      <button class="primary" onclick={submit} disabled={!ready}>
        {busy ? "Working…" : mode === "new" ? "Create" : "Import"}
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed; inset: 0; background: rgba(0, 0, 0, 0.5);
    display: flex; align-items: center; justify-content: center; z-index: 100;
  }
  .dialog {
    width: 380px; background: var(--bg); border: 1px solid var(--border);
    border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 10px;
  }
  .head { font-size: 13px; color: var(--accent); }
  .tabs { display: flex; gap: 4px; }
  .tabs button.on { border-color: var(--accent); color: var(--accent); }
  .field { display: grid; grid-template-columns: 80px 1fr; align-items: center; gap: 8px; }
  .field span { color: var(--fg-dim); font-size: 12px; }
  .field input { width: 100%; }
  .row { display: flex; align-items: center; gap: 8px; }
  .src { font-size: 11px; color: var(--fg-dim); font-family: ui-monospace, monospace;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; flex: 1; }
  .chk { font-size: 12px; color: var(--fg-dim); display: flex; gap: 6px; align-items: center; }
  .err { background: var(--warn); color: #fff; font-size: 12px; padding: 6px 8px; border-radius: 4px; }
  .actions { display: flex; justify-content: flex-end; gap: 6px; margin-top: 2px; }
  .actions .primary { border-color: var(--accent); color: var(--accent); }
</style>
