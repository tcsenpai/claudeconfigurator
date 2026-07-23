<script lang="ts">
  interface Props {
    message: string;
    confirmLabel?: string;
    checkboxLabel?: string;
    onConfirm: (checked: boolean) => void;
    onCancel: () => void;
  }
  let { message, confirmLabel = "Delete", checkboxLabel, onConfirm, onCancel }: Props = $props();
  let checked = $state(false);
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onCancel()} />
<div class="backdrop" onclick={(e) => e.target === e.currentTarget && onCancel()} role="presentation">
  <div class="dialog" role="dialog" aria-modal="true" tabindex="-1">
    <div class="msg">{message}</div>
    {#if checkboxLabel}
      <label class="chk-row">
        <input type="checkbox" bind:checked={checked} />
        <span>{checkboxLabel}</span>
      </label>
    {/if}
    <div class="actions">
      <button onclick={onCancel}>Cancel</button>
      <button class="danger" onclick={() => onConfirm(checked)}>{confirmLabel}</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed; inset: 0; background: rgba(0, 0, 0, 0.5);
    display: flex; align-items: center; justify-content: center; z-index: 100;
  }
  .dialog {
    width: 320px; background: var(--bg); border: 1px solid var(--border);
    border-radius: 8px; padding: 16px; display: flex; flex-direction: column; gap: 14px;
  }
  .msg { font-size: 13px; line-height: 1.4; }
  .chk-row { display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--fg-dim); cursor: pointer; }
  .chk-row input { cursor: pointer; }
  .actions { display: flex; justify-content: flex-end; gap: 6px; }
  .danger { background: var(--warn); border-color: var(--warn); color: #fff; }
  .danger:hover { filter: brightness(1.1); }
</style>
