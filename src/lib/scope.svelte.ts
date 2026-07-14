// Active editing scope (global ~/.claude vs a project's .claude). The Rust side
// holds the real scope; this store mirrors it for the UI and bumps `token` on
// every change so views re-run their load effects against the new scope.

import { scopeGet, scopeSetGlobal, scopeOpenProject, scopeCreateClaude, type ScopeInfo } from "./api";

export const scope = $state<{ info: ScopeInfo; token: number }>({
  info: { kind: "global", label: "Global (~/.claude)", path: "" },
  token: 0,
});

function apply(info: ScopeInfo) {
  scope.info = info;
  scope.token += 1; // signal views to reload
}

export async function loadScope() {
  try { apply(await scopeGet()); } catch { /* keep default */ }
}

export async function switchGlobal() { apply(await scopeSetGlobal()); }

/** Returns "opened" | "no-claude"; on "no-claude" the caller offers to create. */
export async function openProject(path: string): Promise<"opened" | "no-claude"> {
  const r = await scopeOpenProject(path);
  if (r.status === "opened" && r.info) apply(r.info);
  return r.status;
}

export async function createClaude(path: string) { apply(await scopeCreateClaude(path)); }

export const isProject = () => scope.info.kind === "project";
