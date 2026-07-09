// Intent-based grouping of settings.json top-level keys, so the Settings tab
// reads like a Preferences window instead of a JSON dump. Keys not listed here
// fall into an "Other" group. `tab` points complex objects at their dedicated
// editor instead of rendering them inline.

export interface GroupDef {
  id: string;
  title: string;
  keys: string[];
}

// Ordered sections. A key may appear in only one group.
export const GROUPS: GroupDef[] = [
  {
    id: "model",
    title: "Model & behavior",
    keys: [
      "model", "effortLevel", "fastMode", "fastModePerSessionOptIn",
      "alwaysThinkingEnabled", "autoMemoryEnabled", "language",
      "skillListingBudgetFraction",
    ],
  },
  {
    id: "safety",
    title: "Permissions & safety",
    keys: [
      "permissions", "skipDangerousModePermissionPrompt", "disableAllHooks",
      "allowManagedHooksOnly", "allowManagedPermissionRulesOnly",
    ],
  },
  {
    id: "env",
    title: "Environment",
    keys: ["env", "apiKeyHelper", "awsAuthRefresh", "awsCredentialExport"],
  },
  {
    id: "git",
    title: "Git & attribution",
    keys: [
      "includeCoAuthoredBy", "includeGitInstructions", "respectGitignore",
      "attribution", "cleanupPeriodDays",
    ],
  },
  {
    id: "appearance",
    title: "Appearance & status",
    keys: ["statusLine", "fileSuggestion", "autoUpdatesChannel"],
  },
  {
    id: "extensions",
    title: "Plugins & marketplaces",
    keys: ["enabledPlugins", "extraKnownMarketplaces", "strictKnownMarketplaces"],
  },
];

// Keys that have their own dedicated tab; render a pointer instead of an editor.
export const TAB_KEYS: Record<string, string> = {
  hooks: "Hooks",
  enabledPlugins: "Plugins",
  extraKnownMarketplaces: "Plugins",
};

const grouped = new Set(GROUPS.flatMap((g) => g.keys));

/** Assign every present key to a group; unknown keys go to "Other". */
export function assignGroups(keys: string[]): { def: GroupDef; keys: string[] }[] {
  const out = GROUPS.map((def) => ({ def, keys: keys.filter((k) => def.keys.includes(k)) }));
  const other = keys.filter((k) => !grouped.has(k) && k !== "$schema");
  if (other.length) out.push({ def: { id: "other", title: "Other", keys: other }, keys: other });
  return out.filter((g) => g.keys.length > 0);
}
