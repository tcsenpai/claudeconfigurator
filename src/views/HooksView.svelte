<script lang="ts">
  import { settingsGet, settingsSet } from "../lib/api";

  // settings.json hooks shape:
  //   { Event: [ { matcher, hooks: [ { type, command?|url?, timeout?, async? } ] } ] }
  interface HookCmd {
    type: "command" | "http";
    command?: string;
    url?: string;
    timeout?: number;
    async?: boolean;
  }
  interface Matcher { matcher: string; hooks: HookCmd[] }
  type Hooks = Record<string, Matcher[]>;

  const EVENTS = [
    "PreToolUse", "PostToolUse", "UserPromptSubmit", "Notification",
    "PermissionRequest", "Stop", "StopFailure", "SubagentStop",
    "PreCompact", "SessionStart", "SessionEnd",
  ];

  let hooks = $state<Hooks>({});
  let dirty = $state(false);
  let saving = $state(false);
  let error = $state("");

  $effect(() => {
    settingsGet<Hooks | null>("hooks")
      .then((h) => { hooks = h ?? {}; error = ""; })
      .catch((e) => (error = String(e)));
  });

  function touch() { dirty = true; }

  function addMatcher(ev: string) {
    hooks[ev] = [...(hooks[ev] ?? []), { matcher: "", hooks: [{ type: "command", command: "" }] }];
    touch();
  }
  function removeMatcher(ev: string, i: number) {
    hooks[ev] = hooks[ev].filter((_, j) => j !== i);
    if (!hooks[ev].length) delete hooks[ev];
    touch();
  }
  function addCmd(m: Matcher) { m.hooks = [...m.hooks, { type: "command", command: "" }]; touch(); }
  function removeCmd(m: Matcher, i: number) { m.hooks = m.hooks.filter((_, j) => j !== i); touch(); }

  async function save() {
    if (saving) return;
    saving = true; error = "";
    try { await settingsSet("hooks", hooks); dirty = false; }
    catch (e) { error = String(e); }
    finally { saving = false; }
  }
</script>

<div class="wrap">
  <div class="bar">
    <span class="title">Hooks{dirty ? " •" : ""}</span>
    <button onclick={save} disabled={!dirty || saving}>{saving ? "Saving…" : "Save"}</button>
  </div>
  {#if error}<div class="err">{error}</div>{/if}

  <div class="scroll">
    {#each EVENTS as ev (ev)}
      {@const list = hooks[ev] ?? []}
      <section>
        <header>
          <span class="ev">{ev}</span>
          <span class="count">{list.length}</span>
          <button class="add" onclick={() => addMatcher(ev)}>+ matcher</button>
        </header>
        {#each list as m, mi (mi)}
          <div class="matcher">
            <div class="mrow">
              <label>matcher <input bind:value={m.matcher} oninput={touch} placeholder="Edit|Write (blank = all)" /></label>
              <button class="x" onclick={() => removeMatcher(ev, mi)}>×</button>
            </div>
            {#each m.hooks as h, hi (hi)}
              <div class="cmd">
                <select bind:value={h.type} onchange={touch}>
                  <option value="command">command</option>
                  <option value="http">http</option>
                </select>
                {#if h.type === "http"}
                  <input class="grow" bind:value={h.url} oninput={touch} placeholder="http://…" />
                {:else}
                  <input class="grow" bind:value={h.command} oninput={touch} placeholder="shell command" />
                {/if}
                <input class="to" type="number" bind:value={h.timeout} oninput={touch} placeholder="timeout" />
                <label class="chk"><input type="checkbox" bind:checked={h.async} onchange={touch} /> async</label>
                <button class="x" onclick={() => removeCmd(m, hi)}>×</button>
              </div>
            {/each}
            <button class="addcmd" onclick={() => addCmd(m)}>+ hook</button>
          </div>
        {/each}
      </section>
    {/each}
  </div>
</div>

<style>
  .wrap { display: flex; flex-direction: column; height: 100%; }
  .bar { display: flex; justify-content: space-between; align-items: center;
    padding: 6px 10px; border-bottom: 1px solid var(--border); background: var(--bg-alt); }
  .title { color: var(--fg-dim); font-size: 12px; }
  .err { padding: 8px 10px; background: var(--warn); color: #fff; font-size: 12px; }
  .scroll { overflow-y: auto; padding: 8px 12px; }
  section { margin-bottom: 12px; }
  header { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .ev { font-family: ui-monospace, monospace; font-size: 12px; color: var(--accent); }
  .count { font-size: 11px; color: var(--fg-dim); }
  .add { font-size: 11px; margin-left: auto; }
  .matcher { border: 1px solid var(--border); border-radius: 6px; padding: 6px 8px; margin: 4px 0; }
  .mrow { display: flex; align-items: center; gap: 8px; }
  .mrow label { flex: 1; display: flex; gap: 6px; align-items: center; font-size: 12px; color: var(--fg-dim); }
  .mrow input { flex: 1; }
  .cmd { display: flex; align-items: center; gap: 6px; margin: 4px 0 4px 12px; }
  .grow { flex: 1; min-width: 0; font-family: ui-monospace, monospace; font-size: 12px; }
  .to { width: 74px; }
  .chk { font-size: 11px; color: var(--fg-dim); display: flex; align-items: center; gap: 3px; }
  .x { color: var(--warn); background: none; border: none; font-size: 15px; padding: 0 6px; }
  .addcmd { font-size: 11px; margin-left: 12px; }
</style>
