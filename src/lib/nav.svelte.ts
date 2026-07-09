// Tiny cross-view navigation store. Following an @-ref sets `request`; the
// target view picks it up, opens the file, and clears it.

interface NavState {
  view: string;
  /** A path (relative to ~/.claude) a view should open when it becomes active. */
  request: string | null;
}

export const nav = $state<NavState>({ view: "files", request: null });

/** Map a relative path to the view that owns it. */
export function viewForPath(path: string): string {
  if (path.startsWith("skills/")) return "skills";
  if (path.startsWith("commands/")) return "commands";
  if (path.startsWith("agents/")) return "agents";
  if (path === "settings.json") return "settings";
  return "files";
}

/** Follow a resolved @-ref: switch view + queue the file to open. */
export function follow(path: string) {
  nav.view = viewForPath(path);
  nav.request = path;
}
