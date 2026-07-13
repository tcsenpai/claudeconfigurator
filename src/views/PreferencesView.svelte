<script lang="ts">
  import { appConfig, saveAppConfig } from "../lib/appConfig.svelte";
  import BackupSection from "../lib/BackupSection.svelte";

  let saved = $state(false);

  // Edit delay in seconds; store as ms.
  const delaySec = $derived(Math.round(appConfig.autosave_delay_ms / 100) / 10);

  async function persist() {
    await saveAppConfig();
    saved = true;
    setTimeout(() => (saved = false), 1200);
  }

  function setAutosave(on: boolean) { appConfig.autosave = on; persist(); }
  function setDelay(sec: number) {
    if (!Number.isFinite(sec) || sec < 1) sec = 1;
    appConfig.autosave_delay_ms = Math.round(sec * 1000);
    persist();
  }
</script>

<div class="wrap">
  <h2>Preferences {#if saved}<span class="saved">saved</span>{/if}</h2>

  <section>
    <h3>Autosave</h3>
    <label class="row">
      <span class="l">Autosave edits</span>
      <label class="switch">
        <input type="checkbox" checked={appConfig.autosave}
          onchange={(e) => setAutosave(e.currentTarget.checked)} />
        <span class="slider"></span>
      </label>
    </label>
    <p class="help">
      When on, a changed file is saved automatically after a period of
      inactivity. A backup is kept before every save. Off by default because
      this edits your live Claude config.
    </p>

    <label class="row" class:disabled={!appConfig.autosave}>
      <span class="l">Delay (seconds)</span>
      <input type="number" min="1" step="0.5" value={delaySec}
        disabled={!appConfig.autosave}
        onchange={(e) => setDelay(e.currentTarget.valueAsNumber)} />
    </label>
  </section>

  <BackupSection />
</div>

<style>
  .wrap { height: 100%; overflow-y: auto; padding: 14px 18px; }
  h2 { font-size: 15px; margin: 0 0 12px; }
  .saved { font-size: 11px; color: var(--accent); margin-left: 8px; }
  section { margin-bottom: 18px; max-width: 560px; }
  h3 { font-size: 12px; color: var(--accent); text-transform: uppercase; letter-spacing: 0.5px;
    margin: 0 0 8px; border-bottom: 1px solid var(--border); padding-bottom: 4px; }
  .row { display: flex; align-items: center; gap: 12px; padding: 6px 0; }
  .row.disabled { opacity: 0.5; }
  .l { flex: 1; font-size: 13px; }
  .row input[type="number"] { width: 90px; }
  .help { font-size: 11px; color: var(--fg-dim); line-height: 1.4; margin: 2px 0 8px; }

  .switch { position: relative; display: inline-block; width: 34px; height: 18px; }
  .switch input { opacity: 0; width: 0; height: 0; }
  .slider { position: absolute; inset: 0; cursor: pointer; background: #444; border-radius: 18px; transition: 0.15s; }
  .slider::before { content: ""; position: absolute; height: 14px; width: 14px; left: 2px; bottom: 2px;
    background: #ccc; border-radius: 50%; transition: 0.15s; }
  .switch input:checked + .slider { background: var(--accent); }
  .switch input:checked + .slider::before { transform: translateX(16px); background: #fff; }
</style>
