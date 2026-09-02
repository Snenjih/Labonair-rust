//! AI agents — named instruction presets the AI panel's AgentSwitcher picks
//! between (port of `reference-src/src/modules/ai/lib/agents.ts` +
//! `store/agentsStore.ts`).
//!
//! Built-in agents are hard-coded; user agents + the active id persist in a
//! plain `labonair-agents.json` object in the config dir (the web app used a
//! Tauri `LazyStore`). A corrupt file is treated as "no custom agents".

use serde::{Deserialize, Serialize};

use crate::modules::fs::paths::config_dir;

const AGENTS_FILE: &str = "labonair-agents.json";

/// A named instruction preset. `built_in` agents cannot be edited or removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub instructions: String,
    /// One of `coder`/`architect`/`reviewer`/`security`/`designer`/`spark`.
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub built_in: bool,
}

/// The hard-coded agents (verbatim ids + names from the reference; instruction
/// text trimmed to the first line for the port — the full prompts live in the
/// reference and can be pasted back if needed).
pub fn builtin_agents() -> Vec<Agent> {
    let a = |id: &str, name: &str, desc: &str, icon: &str, instr: &str| Agent {
        id: id.to_string(),
        name: name.to_string(),
        description: desc.to_string(),
        instructions: instr.to_string(),
        icon: icon.to_string(),
        built_in: true,
    };
    vec![
        a(
            "builtin:coder",
            "Coder",
            "General-purpose coding assistant. Writes, edits, and runs.",
            "coder",
            "You are an expert software engineer pair-programming inside the user's terminal. Read files before editing them. Prefer the smallest correct change. Run the project's checks after non-trivial edits.",
        ),
        a(
            "builtin:architect",
            "Architect",
            "Design and tradeoffs. Plans before code.",
            "architect",
            "You are a senior software architect. Restate the problem, surface 2-3 viable approaches with real tradeoffs, recommend one with reasoning, and call out risks. Output: Problem / Options / Recommendation / Risks / Next steps.",
        ),
        a(
            "builtin:reviewer",
            "Code Reviewer",
            "Reviews diffs for correctness, perf, security.",
            "reviewer",
            "You are a meticulous code reviewer. Focus on logic errors, edge cases, race conditions, layer violations, perf cliffs, security. Skip formatting nits. Output: [MUST/SHOULD/NIT] file:line - issue -> fix.",
        ),
        a(
            "builtin:security",
            "Security",
            "Threat-models changes and flags vulns.",
            "security",
            "You are an application-security engineer. Threat-model the change. Look for input validation, authn/authz bypass, secret exposure, SSRF, path traversal, injection, deserialization, dependency CVEs. For each finding: severity, exploit sketch, fix.",
        ),
        a(
            "builtin:designer",
            "Designer",
            "UI/UX critique and refinement.",
            "designer",
            "You are a senior product designer with a taste for restrained, modern UI. Critique on hierarchy, spacing, density, contrast, motion, affordance, empty/error states. Propose concrete changes.",
        ),
    ]
}

/// The default active agent id.
pub fn default_active_id() -> String {
    "builtin:coder".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentsFile {
    #[serde(default)]
    custom_agents: Vec<Agent>,
    #[serde(default)]
    active_agent_id: Option<String>,
}

/// Everything the store needs on hydrate.
#[derive(Debug, Clone)]
pub struct LoadedAgents {
    pub custom: Vec<Agent>,
    pub active_id: String,
}

fn agents_path() -> std::path::PathBuf {
    config_dir().join(AGENTS_FILE)
}

fn load_from(path: &std::path::Path) -> LoadedAgents {
    let file = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<AgentsFile>(&raw).ok())
        .unwrap_or_default();
    LoadedAgents {
        custom: file
            .custom_agents
            .into_iter()
            .map(|mut a| {
                a.built_in = false;
                a
            })
            .collect(),
        active_id: file.active_agent_id.unwrap_or_else(default_active_id),
    }
}

fn save_to(path: &std::path::Path, custom: &[Agent], active_id: &str) -> Result<(), String> {
    let file = AgentsFile {
        custom_agents: custom.to_vec(),
        active_agent_id: Some(active_id.to_string()),
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// Load custom agents + the active id from the config dir.
pub fn load() -> LoadedAgents {
    load_from(&agents_path())
}

/// Persist custom agents + the active id.
pub fn save(custom: &[Agent], active_id: &str) -> Result<(), String> {
    save_to(&agents_path(), custom, active_id)
}

/// Insert-or-replace a custom agent by id (built-ins are ignored). Pure.
pub fn upsert(current: &[Agent], agent: Agent) -> Vec<Agent> {
    if agent.built_in {
        return current.to_vec();
    }
    let mut next: Vec<Agent> = current
        .iter()
        .filter(|a| a.id != agent.id)
        .cloned()
        .collect();
    next.push(agent);
    next
}

/// Remove a custom agent by id. Pure.
pub fn remove(current: &[Agent], id: &str) -> Vec<Agent> {
    current.iter().filter(|a| a.id != id).cloned().collect()
}

/// A fresh custom-agent id.
pub fn new_agent_id() -> String {
    format!(
        "agent-{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(id: &str) -> Agent {
        Agent {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            instructions: String::new(),
            icon: "coder".to_string(),
            built_in: false,
        }
    }

    #[test]
    fn builtins_are_stable_and_first_is_the_default() {
        let b = builtin_agents();
        assert_eq!(b.len(), 5);
        assert!(b.iter().all(|a| a.built_in));
        assert_eq!(b[0].id, default_active_id());
    }

    #[test]
    fn upsert_inserts_then_replaces_and_ignores_builtins() {
        let list = upsert(&[], agent("a"));
        assert_eq!(list.len(), 1);
        let mut a2 = agent("a");
        a2.name = "renamed".into();
        let list = upsert(&list, a2);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "renamed");
        let mut builtin = agent("b");
        builtin.built_in = true;
        assert_eq!(upsert(&list, builtin).len(), 1);
    }

    #[test]
    fn remove_drops_by_id() {
        let list = vec![agent("a"), agent("b")];
        assert_eq!(remove(&list, "a"), vec![agent("b")]);
    }

    #[test]
    fn round_trips_through_a_file() {
        let dir = std::env::temp_dir().join(format!("labonair-agents-test-{}", new_agent_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.json");
        save_to(&path, &[agent("x")], "agent-x").unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded.custom.len(), 1);
        assert_eq!(loaded.active_id, "agent-x");
        std::fs::remove_dir_all(&dir).ok();
    }
}
