import { invoke } from "@tauri-apps/api/core";

// Mirrors src-tauri/src/frontmatter.rs `Field`.
export type Field =
  | { kind: "scalar"; key: string; value: string }
  | { kind: "list"; key: string; value: string[] }
  | { kind: "raw"; key: string; value: string };

export interface FileItem { name: string; path: string }
export interface FileDoc {
  path: string;
  fields: Field[];
  body: string;
  raw: string;
  dir: string;
}
export interface Entry {
  name: string;
  description: string;
  path: string;
  group: string;
}
export interface Catalog { skills: Entry[]; commands: Entry[]; agents: Entry[] }
export interface Ref { start: number; end: number; token: string; target: string | null }

export const listRootMd = () => invoke<FileItem[]>("list_root_md");
export const readFile = (path: string) => invoke<FileDoc>("read_file", { path });
export const writeFile = (path: string, fields: Field[], body: string, validateJson = false) =>
  invoke<void>("write_file", { path, fields, body, validateJson });
export const writeRaw = (path: string, content: string, validateJson = false) =>
  invoke<void>("write_raw", { path, content, validateJson });
export const catalog = () => invoke<Catalog>("catalog");
export const scanRefs = (body: string, dir: string) =>
  invoke<Ref[]>("scan_refs", { body, dir });

// --- settings (structured) ---
export const settingsGet = <T = unknown>(key: string) => invoke<T>("settings_get", { key });
export const settingsSet = (key: string, value: unknown) =>
  invoke<void>("settings_set", { key, value });

// --- plugins ---
export interface Plugin {
  id: string;
  name: string;
  marketplace: string;
  version: string;
  enabled: boolean;
}
export interface Marketplace { name: string; repo: string; source: string }
export interface PluginData { plugins: Plugin[]; marketplaces: Marketplace[] }

export const pluginsList = () => invoke<PluginData>("plugins_list");
export const pluginSetEnabled = (id: string, enabled: boolean) =>
  invoke<void>("plugin_set_enabled", { id, enabled });
export const pluginInstall = (id: string) => invoke<string>("plugin_install", { id });
export const pluginRemove = (id: string) => invoke<string>("plugin_remove", { id });
export const marketplaceAdd = (repo: string) => invoke<string>("marketplace_add", { repo });

// --- create / import ---
export type Kind = "skill" | "command" | "agent" | "file";

export const createEntry = (kind: Kind, name: string, namespace?: string) =>
  invoke<string>("create_entry", { kind, name, namespace: namespace || null });
export const importFile = (kind: Kind, name: string, src: string, namespace?: string) =>
  invoke<string>("import_file", { kind, name, src, namespace: namespace || null });
export const importSkillDir = (name: string, src: string) =>
  invoke<string>("import_skill_dir", { name, src });
export const deleteEntry = (path: string) => invoke<void>("delete_entry", { path });
