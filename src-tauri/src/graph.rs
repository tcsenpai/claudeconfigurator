//! Dependency graph of the ~/.claude config: which file references which.
//!
//! Nodes are files under ~/.claude (markdown entries, settings.json, and the
//! hook scripts settings.json invokes). Edges are:
//!   - `ref`: a markdown file's `@`-reference resolving to another file
//!   - `hook`: settings.json invoking a script path in a hook command
//! The frontend renders an ego-graph (one focused node + neighbours) from this.

use crate::index;
use crate::jail;
use crate::refs;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
pub struct Node {
    /// Path relative to ~/.claude (unique id).
    pub id: String,
    /// "claude" | "file" | "skill" | "command" | "agent" | "settings" | "script"
    pub kind: String,
}

#[derive(Serialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    /// "ref" | "hook"
    pub kind: String,
}

#[derive(Serialize, Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[tauri::command]
pub fn graph_data() -> Result<Graph, String> {
    let root = jail::root()?;
    let cat = index::build()?;
    let mut nodes: BTreeSet<(String, String)> = BTreeSet::new(); // (id, kind)
    let mut edges: Vec<Edge> = vec![];

    // Collect the markdown source files: root .md (CLAUDE.md + adjacent) and
    // every skill/command/agent entry.
    let mut md_files: Vec<(String, String)> = vec![];
    for (name, _p) in root_md(&root) {
        let kind = if name == "CLAUDE.md" { "claude" } else { "file" };
        md_files.push((name, kind.to_string()));
    }
    for e in &cat.skills {
        md_files.push((e.path.clone(), "skill".into()));
    }
    for e in &cat.commands {
        md_files.push((e.path.clone(), "command".into()));
    }
    for e in &cat.agents {
        md_files.push((e.path.clone(), "agent".into()));
    }

    // Every source file is a node; scan each for @-refs -> edges.
    for (rel, kind) in &md_files {
        nodes.insert((rel.clone(), kind.clone()));
        let abs = root.join(rel);
        let body = fs::read_to_string(&abs).unwrap_or_default();
        let dir = Path::new(rel).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        if let Ok(found) = refs::scan(&body, &dir, &cat) {
            for r in found {
                if let Some(target) = r.target {
                    // Ensure the target is a node too (it's a real file under root).
                    nodes.insert((target.clone(), kind_of(&target)));
                    edges.push(Edge { from: rel.clone(), to: target, kind: "ref".into() });
                }
            }
        }
    }

    // settings.json hooks -> script paths they invoke.
    if root.join("settings.json").is_file() {
        nodes.insert(("settings.json".into(), "settings".into()));
        for script in hook_scripts(&root) {
            nodes.insert((script.clone(), "script".into()));
            edges.push(Edge { from: "settings.json".into(), to: script, kind: "hook".into() });
        }
    }

    Ok(Graph {
        nodes: nodes.into_iter().map(|(id, kind)| Node { id, kind }).collect(),
        edges,
    })
}

fn kind_of(rel: &str) -> String {
    if rel == "CLAUDE.md" { "claude" }
    else if rel.starts_with("skills/") { "skill" }
    else if rel.starts_with("commands/") { "command" }
    else if rel.starts_with("agents/") { "agent" }
    else if rel == "settings.json" { "settings" }
    else if !rel.contains('/') { "file" }
    else { "script" }
    .to_string()
}

fn root_md(root: &Path) -> Vec<(String, std::path::PathBuf)> {
    fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "md"))
        .map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), p))
        .collect()
}

/// Extract script paths under ~/.claude referenced in hook commands. Matches
/// `$HOME/.claude/...`, `~/.claude/...`, and bare `.claude/...` tokens, keeps
/// those that resolve to an existing file, returns them relative to root.
fn hook_scripts(root: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let raw = match fs::read_to_string(root.join("settings.json")) {
        Ok(s) => s,
        Err(_) => return out,
    };
    let val: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return out,
    };
    let hooks = match val.get("hooks").and_then(|h| h.as_object()) {
        Some(h) => h,
        None => return out,
    };
    for arr in hooks.values() {
        for matcher in arr.as_array().into_iter().flatten() {
            for hook in matcher.get("hooks").and_then(|h| h.as_array()).into_iter().flatten() {
                if let Some(cmd) = hook.get("command").and_then(|c| c.as_str()) {
                    for rel in extract_claude_paths(cmd, root) {
                        out.insert(rel);
                    }
                }
            }
        }
    }
    out
}

/// Pull `.claude/<...>` path tokens out of a shell command and, for those that
/// exist as files, return them relative to root.
fn extract_claude_paths(cmd: &str, root: &Path) -> Vec<String> {
    let mut out = vec![];
    for tok in cmd.split(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ';') {
        if let Some(idx) = tok.find(".claude/") {
            let sub = &tok[idx + ".claude/".len()..];
            if sub.is_empty() {
                continue;
            }
            let abs = root.join(sub);
            if abs.is_file() {
                out.push(sub.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::with_claude;

    #[test]
    fn builds_ref_and_hook_edges() {
        with_claude(|claude| {
            fs::write(claude.join("CLAUDE.md"), "see @RTK.md\n").unwrap();
            fs::write(claude.join("RTK.md"), "x").unwrap();
            fs::create_dir_all(claude.join("hooks")).unwrap();
            fs::write(claude.join("hooks/run.sh"), "echo").unwrap();
            fs::write(
                claude.join("settings.json"),
                r#"{"hooks":{"Stop":[{"matcher":"","hooks":[{"type":"command","command":"$HOME/.claude/hooks/run.sh arg"}]}]}}"#,
            )
            .unwrap();

            let g = graph_data().unwrap();
            let ids: Vec<_> = g.nodes.iter().map(|n| n.id.as_str()).collect();
            assert!(ids.contains(&"CLAUDE.md"));
            assert!(ids.contains(&"RTK.md"));
            assert!(ids.contains(&"settings.json"));
            assert!(ids.contains(&"hooks/run.sh"));

            assert!(g.edges.iter().any(|e| e.from == "CLAUDE.md" && e.to == "RTK.md" && e.kind == "ref"));
            assert!(g.edges.iter().any(|e| e.from == "settings.json" && e.to == "hooks/run.sh" && e.kind == "hook"));
        });
    }
}
