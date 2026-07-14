import type { Component } from "svelte";
import ClaudeMdView from "./ClaudeMdView.svelte";
import FilesView from "./FilesView.svelte";
import SkillsView from "./SkillsView.svelte";
import CommandsView from "./CommandsView.svelte";
import AgentsView from "./AgentsView.svelte";
import GraphView from "./GraphView.svelte";
import HooksView from "./HooksView.svelte";
import PluginsView from "./PluginsView.svelte";
import McpView from "./McpView.svelte";
import SettingsView from "./SettingsView.svelte";
import PreferencesView from "./PreferencesView.svelte";

export interface View {
  id: string;
  label: string;
  component: Component;
  /** Shown in project scope? Global-only features set this false. */
  project?: boolean;
}

// The extensibility contract: add a module = add a folder + one line here.
// `project: false` (or omitted) hides a view when a project scope is active.
export const views: View[] = [
  { id: "claude", label: "CLAUDE.md", component: ClaudeMdView }, // global; project edits it via Files
  { id: "files", label: "Files", component: FilesView, project: true },
  { id: "skills", label: "Skills", component: SkillsView, project: true },
  { id: "commands", label: "Commands", component: CommandsView, project: true },
  { id: "agents", label: "Agents", component: AgentsView, project: true },
  { id: "graph", label: "Graph", component: GraphView },
  { id: "hooks", label: "Hooks", component: HooksView, project: true },
  { id: "plugins", label: "Plugins", component: PluginsView },
  { id: "mcp", label: "MCP", component: McpView, project: true },
  { id: "settings", label: "Settings", component: SettingsView, project: true },
  { id: "prefs", label: "⚙ Preferences", component: PreferencesView },
];
