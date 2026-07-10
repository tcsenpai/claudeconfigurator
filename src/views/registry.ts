import type { Component } from "svelte";
import ClaudeMdView from "./ClaudeMdView.svelte";
import FilesView from "./FilesView.svelte";
import SkillsView from "./SkillsView.svelte";
import CommandsView from "./CommandsView.svelte";
import AgentsView from "./AgentsView.svelte";
import GraphView from "./GraphView.svelte";
import HooksView from "./HooksView.svelte";
import PluginsView from "./PluginsView.svelte";
import SettingsView from "./SettingsView.svelte";
import PreferencesView from "./PreferencesView.svelte";

export interface View {
  id: string;
  label: string;
  component: Component;
}

// The extensibility contract: add a module = add a folder + one line here.
export const views: View[] = [
  { id: "claude", label: "CLAUDE.md", component: ClaudeMdView },
  { id: "files", label: "Files", component: FilesView },
  { id: "skills", label: "Skills", component: SkillsView },
  { id: "commands", label: "Commands", component: CommandsView },
  { id: "agents", label: "Agents", component: AgentsView },
  { id: "graph", label: "Graph", component: GraphView },
  { id: "hooks", label: "Hooks", component: HooksView },
  { id: "plugins", label: "Plugins", component: PluginsView },
  { id: "settings", label: "Settings", component: SettingsView },
  { id: "prefs", label: "⚙ Preferences", component: PreferencesView },
];
