<script lang="ts">
  import { onMount } from "svelte";
  import { backupList, backupRead, backupRestore, type BackupInfo } from "./api";

  interface Props {
    path: string;
    currentValue: string;
    onClose: () => void;
    onRestore: (restored: string) => void;
  }
  let { path, currentValue, onClose, onRestore }: Props = $props();

  let backups = $state<BackupInfo[]>([]);
  let selected = $state<BackupInfo | null>(null);
  let backupText = $state("");
  let loading = $state(false);
  let restoring = $state(false);
  let error = $state("");

  interface DiffLine {
    type: "added" | "removed" | "unchanged";
    text: string;
  }

  const diff = $derived.by((): DiffLine[] => {
    if (selected === null || backupText === null) return [];
    
    const oldLines = backupText ? backupText.split(/\r?\n/) : [];
    const newLines = currentValue ? currentValue.split(/\r?\n/) : [];
    const n = oldLines.length;
    const m = newLines.length;

    // Safety guard against massive files to avoid freezing the main thread
    if (n * m > 100000) {
      return [
        { type: "removed", text: `[Backup version - ${oldLines.length} lines]` },
        { type: "added", text: `[Current version - ${newLines.length} lines]` }
      ];
    }

    // Dynamic programming LCS diff
    const dp: number[][] = Array.from({ length: n + 1 }, () => Array(m + 1).fill(0));
    for (let i = 1; i <= n; i++) {
      for (let j = 1; j <= m; j++) {
        if (oldLines[i - 1] === newLines[j - 1]) {
          dp[i][j] = dp[i - 1][j - 1] + 1;
        } else {
          dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
        }
      }
    }

    const result: DiffLine[] = [];
    let i = n, j = m;
    while (i > 0 || j > 0) {
      if (i > 0 && j > 0 && oldLines[i - 1] === newLines[j - 1]) {
        result.push({ type: "unchanged", text: oldLines[i - 1] });
        i--;
        j--;
      } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
        result.push({ type: "added", text: newLines[j - 1] });
        j--;
      } else {
        result.push({ type: "removed", text: oldLines[i - 1] });
        i--;
      }
    }
    return result.reverse();
  });

  async function load() {
    loading = true; error = "";
    try {
      backups = await backupList(path);
      if (backups.length > 0) {
        await select(backups[0]);
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function select(b: BackupInfo) {
    selected = b;
    error = "";
    try {
      backupText = await backupRead(path, b.index);
    } catch (e) {
      error = String(e);
      backupText = "";
    }
  }

  async function restore() {
    if (!selected || restoring) return;
    restoring = true; error = "";
    try {
      const restored = await backupRestore(path, selected.index);
      onRestore(restored);
    } catch (e) {
      error = String(e);
    } finally {
      restoring = false;
    }
  }

  function formatTime(ms: number): string {
    const date = new Date(ms);
    return date.toLocaleString();
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  onMount(() => {
    load();
  });
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onClose()} />

<div class="backdrop" onclick={(e) => e.target === e.currentTarget && onClose()} role="presentation">
  <div class="dialog" role="dialog" aria-modal="true" tabindex="-1">
    <div class="head">
      <span class="title">Backup History</span>
      <span class="file">{path}</span>
    </div>

    {#if error}
      <div class="err">{error}</div>
    {/if}

    <div class="content">
      <div class="sidebar">
        <div class="section-title">Backups</div>
        {#if loading}
          <div class="empty">Loading list…</div>
        {:else if !backups.length}
          <div class="empty">No backups found.</div>
        {:else}
          <div class="list">
            {#each backups as b, i (b.index)}
              <button
                class="item"
                class:active={selected?.index === b.index}
                onclick={() => select(b)}
              >
                <div class="meta-row">
                  <span class="idx">Backup #{i + 1}</span>
                  <span class="size">{formatSize(b.size)}</span>
                </div>
                <div class="time">{formatTime(b.modified_ms)}</div>
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <div class="viewer">
        <div class="section-title">
          <span>Changes (Red = Backup, Green = Current)</span>
        </div>
        <div class="diff-area">
          {#if selected}
            <div class="diff-lines">
              {#each diff as line, i}
                <div class="line {line.type}">
                  <span class="num">{i + 1}</span>
                  <span class="symbol">{line.type === "added" ? "+" : line.type === "removed" ? "-" : " "}</span>
                  <pre class="txt">{line.text || " "}</pre>
                </div>
              {/each}
            </div>
          {:else}
            <div class="empty">Select a backup to view changes.</div>
          {/if}
        </div>
      </div>
    </div>

    <div class="actions">
      <button onclick={onClose}>Cancel</button>
      <button
        class="primary"
        disabled={!selected || restoring}
        onclick={restore}
      >
        {restoring ? "Restoring…" : "Restore Selected Backup"}
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.5);
    display: flex; align-items: center; justify-content: center; z-index: 100;
  }
  .dialog {
    width: 90vw; height: 85vh; max-width: 960px;
    background: var(--bg); border: 1px solid var(--border); border-radius: 8px;
    display: flex; flex-direction: column; overflow: hidden;
  }
  .head {
    padding: 12px 16px; border-bottom: 1px solid var(--border);
    display: flex; justify-content: space-between; align-items: center; background: var(--bg-alt);
  }
  .title { font-size: 14px; font-weight: 600; color: var(--accent); }
  .file { font-size: 12px; font-family: ui-monospace, monospace; color: var(--fg-dim); }
  
  .content { flex: 1; display: flex; min-height: 0; }
  
  .sidebar {
    width: 240px; border-right: 1px solid var(--border);
    display: flex; flex-direction: column; background: var(--bg-alt);
  }
  .section-title {
    font-size: 11px; text-transform: uppercase; letter-spacing: 0.5px;
    color: var(--fg-dim); padding: 8px 12px; border-bottom: 1px solid var(--border);
    display: flex; justify-content: space-between; align-items: center;
  }
  
  .list { flex: 1; overflow-y: auto; display: flex; flex-direction: column; padding: 4px; gap: 4px; }
  .item {
    background: none; border: none; text-align: left; padding: 8px; border-radius: 6px;
    display: flex; flex-direction: column; gap: 3px; cursor: pointer;
  }
  .item:hover { background: var(--bg-hover); }
  .item.active { background: var(--bg-hover); border: 1px solid var(--border); }
  .meta-row { display: flex; justify-content: space-between; font-family: ui-monospace, monospace; font-size: 12px; }
  .idx { color: var(--fg); font-weight: 500; }
  .size { color: var(--fg-dim); }
  .time { font-size: 11px; color: var(--fg-dim); }
  
  .viewer { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .diff-area { flex: 1; overflow: auto; background: var(--bg); font-family: ui-monospace, monospace; font-size: 12px; }
  .diff-lines { display: flex; flex-direction: column; min-height: 100%; }
  
  .line { display: flex; align-items: stretch; line-height: 1.5; white-space: pre-wrap; }
  .line.added { background: rgba(40, 167, 69, 0.15); color: #2beb6e; }
  .line.removed { background: rgba(220, 53, 69, 0.15); color: #ff5f6f; }
  .num { width: 40px; text-align: right; color: var(--fg-dim); padding-right: 10px; user-select: none; border-right: 1px solid var(--border); opacity: 0.5; background: var(--bg-alt); }
  .symbol { width: 20px; text-align: center; user-select: none; opacity: 0.7; }
  .txt { flex: 1; margin: 0; font-family: inherit; font-size: inherit; white-space: pre-wrap; padding-left: 4px; }

  .actions {
    padding: 12px 16px; border-top: 1px solid var(--border); background: var(--bg-alt);
    display: flex; justify-content: flex-end; gap: 8px;
  }
  .actions button.primary { border-color: var(--accent); color: var(--accent); }
  .actions button.primary:disabled { opacity: 0.5; cursor: not-allowed; }
  
  .empty { padding: 20px; color: var(--fg-dim); text-align: center; font-size: 12px; }
  .err { padding: 8px 12px; background: var(--warn); color: #fff; font-size: 12px; }
</style>
