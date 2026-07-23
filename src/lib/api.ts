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

// The bundled JSON Schema for settings.json (loose shape; we read a subset).
export interface JsonSchema {
  properties?: Record<string, SchemaProp>;
}
export interface SchemaProp {
  type?: string | string[];
  description?: string;
  enum?: unknown[];
  default?: unknown;
  minimum?: number;
  maximum?: number;
  items?: { type?: string };
}
export const settingsSchema = () => invoke<JsonSchema>("settings_schema");

// --- app preferences (the app's own config, not ~/.claude) ---
export interface AppConfig {
  autosave: boolean;
  autosave_delay_ms: number;
}
export const appConfigGet = () => invoke<AppConfig>("app_config_get");
export const appConfigSet = (config: AppConfig) => invoke<void>("app_config_set", { config });

// --- MCP servers (from ~/.claude.json) ---
export interface McpServer {
  name: string;
  enabled: boolean;
  config: Record<string, unknown>;
}
export const mcpList = () => invoke<McpServer[]>("mcp_list");
export const mcpUpsert = (name: string, config: Record<string, unknown>, enabled: boolean) =>
  invoke<void>("mcp_upsert", { name, config, enabled });
export const mcpRemove = (name: string) => invoke<void>("mcp_remove", { name });
export const mcpSetEnabled = (name: string, enabled: boolean) =>
  invoke<void>("mcp_set_enabled", { name, enabled });

// --- config bundle (export / restore) ---
export interface SecretHit { location: string }
export interface RestorePreview {
  version: number;
  created_at: string;
  redacted: boolean;
  redactions: string[];
  conflicts: string[];
  new_files: string[];
  plugin_install_cmds: string[];
}
export const bundleScanSecrets = () => invoke<SecretHit[]>("bundle_scan_secrets");
export const bundleExport = (dest: string, redact: boolean, timestamp: string) =>
  invoke<void>("bundle_export", { dest, redact, timestamp });
export const bundlePreview = (archive: string) => invoke<RestorePreview>("bundle_preview", { archive });
export const bundleRestore = (
  archive: string, mode: "replace" | "merge", replaceFiles: string[], timestamp: string,
) => invoke<void>("bundle_restore", { archive, mode, replaceFiles, timestamp });

// --- scope (global vs project) ---
export interface ScopeInfo { kind: "global" | "project"; label: string; path: string }
export interface OpenResult { status: "opened" | "no-claude"; info: ScopeInfo | null }
export const scopeGet = () => invoke<ScopeInfo>("scope_get");
export const scopeSetGlobal = () => invoke<ScopeInfo>("scope_set_global");
export const scopeOpenProject = (path: string) => invoke<OpenResult>("scope_open_project", { path });
export const scopeCreateClaude = (path: string) => invoke<ScopeInfo>("scope_create_claude", { path });

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
export const deleteEntry = (path: string, deleteBackups = false) => invoke<void>("delete_entry", { path, deleteBackups });

// --- graph ---
export interface GraphNode { id: string; kind: string }
export interface GraphEdge { from: string; to: string; kind: string }
export interface Graph { nodes: GraphNode[]; edges: GraphEdge[] }
export const graphData = () => invoke<Graph>("graph_data");

// --- backup history ---
export interface BackupInfo {
  index: number;
  size: number;
  modified_ms: number;
}
export const backupList = (path: string) => invoke<BackupInfo[]>("backup_list", { path });
export const backupRead = (path: string, index: number) => invoke<string>("backup_read", { path, index });
export const backupRestore = (path: string, index: number) => invoke<string>("backup_restore", { path, index });

