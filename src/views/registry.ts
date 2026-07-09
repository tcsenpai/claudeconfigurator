import type { Component } from "svelte";
import FilesView from "./FilesView.svelte";
import SkillsView from "./SkillsView.svelte";
import CommandsView from "./CommandsView.svelte";
import AgentsView from "./AgentsView.svelte";
import SettingsView from "./SettingsView.svelte";

export interface View {
  id: string;
  label: string;
  component: Component;
}

// The extensibility contract: add a module = add a folder + one line here.
export const views: View[] = [
  { id: "files", label: "Files", component: FilesView },
  { id: "skills", label: "Skills", component: SkillsView },
  { id: "commands", label: "Commands", component: CommandsView },
  { id: "agents", label: "Agents", component: AgentsView },
  { id: "settings", label: "Settings", component: SettingsView },
];
