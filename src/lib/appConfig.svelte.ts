// Reactive store for the app's own preferences (autosave, etc). Loaded once at
// startup; `save()` persists to disk via Rust. Separate from ~/.claude.

import { appConfigGet, appConfigSet, type AppConfig } from "./api";

export const appConfig = $state<AppConfig>({ autosave: false, autosave_delay_ms: 5000 });

let loaded = false;

export async function loadAppConfig() {
  if (loaded) return;
  loaded = true;
  try {
    const c = await appConfigGet();
    appConfig.autosave = c.autosave;
    appConfig.autosave_delay_ms = c.autosave_delay_ms;
  } catch {
    /* keep defaults */
  }
}

export function saveAppConfig() {
  return appConfigSet({ ...appConfig });
}
