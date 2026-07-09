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
