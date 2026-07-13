<script lang="ts">
  import { save, open } from "@tauri-apps/plugin-dialog";
  import {
    bundleScanSecrets, bundleExport, bundlePreview, bundleRestore,
    type SecretHit, type RestorePreview,
  } from "./api";

  let busy = $state(false);
  let msg = $state("");

  // Export pre-flight state.
  let secrets = $state<SecretHit[] | null>(null);
  let pendingExportPath = $state<string | null>(null);

  // Restore state.
  let preview = $state<RestorePreview | null>(null);
  let archivePath = $state<string | null>(null);
  let mode = $state<"replace" | "merge">("replace");
  let replaceSet = $state<Set<string>>(new Set());

  const ts = () => {
    // Local timestamp for filenames + snapshot labels.
    const d = new Date();
    const p = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}${p(d.getSeconds())}`;
  };

  // ---- Export ----
  async function startExport() {
    msg = "";
    const dest = await save({
      defaultPath: `claude-config-${ts()}.tar.gz`,
      filters: [{ name: "Archive", extensions: ["tar.gz", "tgz"] }],
    });
    if (!dest) return;
    pendingExportPath = dest;
    try { secrets = await bundleScanSecrets(); }
    catch (e) { msg = String(e); }
  }

  async function doExport(redact: boolean) {
    if (!pendingExportPath) return;
    busy = true; msg = "";
    try {
      await bundleExport(pendingExportPath, redact, ts());
      msg = `Exported to ${pendingExportPath}${redact ? " (secrets redacted)" : ""}`;
    } catch (e) { msg = "error: " + String(e); }
    finally { busy = false; secrets = null; pendingExportPath = null; }
  }

  // ---- Restore ----
  async function startRestore() {
    msg = "";
    const picked = await open({
      multiple: false,
      filters: [{ name: "Archive", extensions: ["tar.gz", "tgz", "gz"] }],
    });
    if (typeof picked !== "string") return;
    archivePath = picked;
    busy = true;
    try {
      preview = await bundlePreview(picked);
      mode = "replace";
      replaceSet = new Set(preview.conflicts); // default: replace all conflicts
    } catch (e) { msg = "error: " + String(e); preview = null; }
    finally { busy = false; }
  }

  function toggleConflict(f: string) {
    const next = new Set(replaceSet);
    next.has(f) ? next.delete(f) : next.add(f);
    replaceSet = next;
  }

  async function doRestore() {
    if (!archivePath || !preview) return;
    busy = true; msg = "";
    try {
      const replaceFiles = mode === "merge" ? [...replaceSet] : [];
      await bundleRestore(archivePath, mode, replaceFiles, ts());
      const cmds = preview.plugin_install_cmds;
      msg = `Restored. Current config backed up first.` +
        (preview.redactions.length
          ? ` Redacted values (${preview.redactions.join(", ")}) were kept if you already had them, otherwise left unset — set them if missing.`
          : "") +
        (cmds.length ? `\nReinstall plugins:\n${cmds.join("\n")}` : "");
    } catch (e) { msg = "error: " + String(e); }
    finally { busy = false; preview = null; archivePath = null; }
  }
</script>

<section>
  <h3>Backup &amp; restore</h3>
  <p class="help">
    Export the whole Claude configuration (instruction files, skills, commands,
    agents, settings, MCP servers, and a plugins list) as a portable archive, or
    restore one. Runtime caches and session history are not included.
  </p>
  <div class="btns">
    <button onclick={startExport} disabled={busy}>Export configuration…</button>
    <button onclick={startRestore} disabled={busy}>Restore configuration…</button>
  </div>
  {#if msg}<pre class="msg">{msg}</pre>{/if}
</section>

<!-- Export secret pre-flight -->
{#if secrets !== null}
  <div class="backdrop" role="presentation">
    <div class="dialog" role="dialog" aria-modal="true" tabindex="-1">
      <div class="head">Export configuration</div>
      {#if secrets.length}
        <p>{secrets.length} secret-like value(s) found:</p>
        <ul class="secrets">{#each secrets as s}<li>{s.location}</li>{/each}</ul>
      {:else}
        <p>No secrets found in known locations (settings/env, MCP env/headers/args/url). Other places aren't scanned — review before sharing.</p>
      {/if}
      <div class="actions">
        <button onclick={() => { secrets = null; pendingExportPath = null; }}>Cancel</button>
        {#if secrets.length}<button onclick={() => doExport(false)}>Keep secrets</button>{/if}
        <button class="primary" onclick={() => doExport(true)}>
          {secrets.length ? "Redact & export" : "Export"}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Restore preview + route -->
{#if preview}
  <div class="backdrop" role="presentation">
    <div class="dialog" role="dialog" aria-modal="true" tabindex="-1">
      <div class="head">Restore configuration</div>
      <p class="sub">
        Bundle from {preview.created_at || "unknown"}
        {#if preview.redacted}· secrets were redacted{/if}
      </p>
      <div class="modes">
        <label><input type="radio" bind:group={mode} value="replace" /> Overlay (overwrite matching files, keep the rest)</label>
        <label><input type="radio" bind:group={mode} value="merge" /> Merge (choose which conflicts to overwrite)</label>
      </div>
      <p class="sub hint">Neither mode deletes local files or MCP servers absent from the bundle. In Merge mode, existing MCP servers are left as-is; only new ones are added. Current config is backed up first.</p>

      {#if preview.new_files.length}
        <p class="sub">{preview.new_files.length} new file(s) will be added.</p>
      {/if}

      {#if mode === "merge" && preview.conflicts.length}
        <p class="sub">Conflicts — check the ones to overwrite:</p>
        <ul class="conflicts">
          {#each preview.conflicts as f}
            <li><label><input type="checkbox" checked={replaceSet.has(f)} onchange={() => toggleConflict(f)} /> {f}</label></li>
          {/each}
        </ul>
      {:else if preview.conflicts.length}
        <p class="sub">{preview.conflicts.length} existing file(s) will be overwritten.</p>
      {/if}

      <div class="actions">
        <button onclick={() => { preview = null; archivePath = null; }}>Cancel</button>
        <button class="primary" onclick={doRestore} disabled={busy}>Restore</button>
      </div>
    </div>
  </div>
{/if}

<style>
  section { margin-bottom: 18px; max-width: 560px; }
  h3 { font-size: 12px; color: var(--accent); text-transform: uppercase; letter-spacing: 0.5px;
    margin: 0 0 8px; border-bottom: 1px solid var(--border); padding-bottom: 4px; }
  .help { font-size: 11px; color: var(--fg-dim); line-height: 1.4; margin: 0 0 10px; }
  .btns { display: flex; gap: 8px; }
  .msg { background: var(--bg-alt); border: 1px solid var(--border); border-radius: 5px; padding: 8px;
    font-size: 11px; white-space: pre-wrap; margin: 10px 0 0; max-height: 180px; overflow: auto; }

  .backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 100; }
  .dialog { width: 440px; max-height: 80vh; overflow-y: auto; background: var(--bg); border: 1px solid var(--border);
    border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .head { font-size: 13px; color: var(--accent); }
  .sub { font-size: 11px; color: var(--fg-dim); margin: 0; }
  .secrets, .conflicts { margin: 0; padding-left: 16px; font-size: 11px; font-family: ui-monospace, monospace;
    max-height: 200px; overflow-y: auto; }
  .conflicts { list-style: none; padding-left: 0; }
  .conflicts label, .modes label { display: flex; gap: 6px; align-items: center; font-size: 12px; }
  .modes { display: flex; flex-direction: column; gap: 4px; }
  .actions { display: flex; justify-content: flex-end; gap: 6px; margin-top: 4px; }
  .actions .primary { border-color: var(--accent); color: var(--accent); }
</style>
