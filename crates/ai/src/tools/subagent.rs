//! Sub-agent registry + runner interface (T11-004).
//!
//! Port of `reference-src/src/modules/ai/agents/registry.ts` +
//! `runSubagent.ts`. A sub-agent is an isolated, **read-only** helper with a
//! fresh message history and a restricted toolset. The concrete runner (which
//! actually calls a model) is injected so this crate stays free of an event
//! loop; tests use [`NoopSubagentRunner`].

use super::registry::ToolContext;

/// A sub-agent kind the main agent can spawn.
pub struct SubagentDef {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// Whitelisted tool names — read-only, no `run_subagent` (no recursion).
    pub tools: &'static [&'static str],
    pub system_prompt: &'static str,
}

const READ_ONLY_TOOLS: &[&str] = &["read_file", "list_directory", "grep", "glob"];

/// The full sub-agent catalog.
pub static SUBAGENTS: &[SubagentDef] = &[
    SubagentDef {
        id: "explore",
        label: "Explore",
        description: "Read-only codebase explorer. Locates files, traces references, summarizes architecture.",
        tools: READ_ONLY_TOOLS,
        system_prompt: "You are an exploration subagent. Answer the spawn question by READING the codebase only — no edits, no commands. Use grep/glob/list_directory/read_file. Be terse. Return a concise summary (file paths, key findings, line numbers). Stop as soon as you can answer.",
    },
    SubagentDef {
        id: "code-review",
        label: "Code review",
        description: "Reviews changed code for correctness, architecture, performance, security.",
        tools: READ_ONLY_TOOLS,
        system_prompt: "You are a code-review subagent. Report only ACTIONABLE findings: correctness bugs, architecture violations, performance issues, security risks. Skip style. Format: \"[MUST/SHOULD/NIT] file:line — issue → fix\". If nothing is wrong, say \"Looks good.\"",
    },
    SubagentDef {
        id: "security",
        label: "Security review",
        description: "Audits code/configuration for security risks (auth, injection, secrets, etc).",
        tools: READ_ONLY_TOOLS,
        system_prompt: "You are a security-review subagent. Scan for: injection (SQL, shell, path), auth/authz bypass, secret leakage, missing validation at trust boundaries, unsafe deserialization, weak crypto. Report concrete findings with file:line and severity. Be conservative. If nothing is wrong, say \"No security issues found.\"",
    },
    SubagentDef {
        id: "general",
        label: "General research",
        description: "General-purpose worker for multi-step research questions spanning many files.",
        tools: READ_ONLY_TOOLS,
        system_prompt: "You are a general-purpose research subagent. Answer the spawn question by reading the codebase. Don't speculate — verify. Return a tight summary with the evidence you used (paths, line numbers).",
    },
];

/// Max agentic steps a sub-agent may take.
pub const SUBAGENT_MAX_STEPS: usize = 12;

/// Result of a completed sub-agent run.
#[derive(Debug, Clone, PartialEq)]
pub struct SubagentReport {
    pub summary: String,
    pub steps: usize,
}

pub fn find_subagent(id: &str) -> Option<&'static SubagentDef> {
    SUBAGENTS.iter().find(|s| s.id == id)
}

/// Injected strategy that actually runs a sub-agent (calls a model in a bounded
/// loop). Kept as a trait so `crates/ai` doesn't own a runtime.
pub trait SubagentRunner: Send + Sync {
    fn run(
        &self,
        kind: &str,
        prompt: &str,
        ctx: &mut ToolContext,
    ) -> Result<SubagentReport, String>;
}

/// No-op runner for tests / headless builds (no model available).
pub struct NoopSubagentRunner;

impl SubagentRunner for NoopSubagentRunner {
    fn run(
        &self,
        kind: &str,
        _prompt: &str,
        _ctx: &mut ToolContext,
    ) -> Result<SubagentReport, String> {
        find_subagent(kind).ok_or_else(|| format!("unknown subagent type: {kind}"))?;
        Ok(SubagentReport {
            summary: "noop".to_string(),
            steps: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_read_only_and_complete() {
        assert_eq!(SUBAGENTS.len(), 4);
        for s in SUBAGENTS {
            assert_eq!(s.tools, READ_ONLY_TOOLS);
            assert!(!s.tools.contains(&"write_file"));
            assert!(!s.tools.contains(&"run_subagent"));
        }
        assert!(find_subagent("explore").is_some());
        assert!(find_subagent("nope").is_none());
    }
}
