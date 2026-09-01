//! Agent tool-execution loop + approval orchestration (T11-004).
//!
//! After a model turn produces tool calls, [`ToolTurn::begin`] auto-executes
//! the read-only ones and holds the mutating ones for user approval. The UI
//! calls [`ToolTurn::resolve`] as the user approves/rejects each card; once
//! [`ToolTurn::is_complete`] is true, [`ToolTurn::into_messages`] yields the
//! `Role::Tool` result messages to append to the history and re-send — that is
//! how the run "continues automatically" after approval.

use serde_json::Value;

use super::registry::{ToolContext, ToolRegistry};
use crate::message::{ChatMessage, ToolCall};

/// One executed tool call's outcome.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub name: String,
    pub output: Value,
    pub is_error: bool,
}

impl ToolResult {
    fn to_message(&self) -> ChatMessage {
        let text = serde_json::to_string(&self.output).unwrap_or_else(|_| "{}".to_string());
        ChatMessage::tool_result(self.tool_call_id.clone(), text)
    }
}

fn is_error_output(v: &Value) -> bool {
    v.get("error").is_some()
}

/// The set of tool calls from a single assistant turn, tracked through
/// approval + execution.
pub struct ToolTurn {
    done: Vec<ToolResult>,
    pending: Vec<ToolCall>,
}

impl ToolTurn {
    /// Execute all auto-approved calls now; queue the approval-gated ones.
    pub fn begin(registry: &ToolRegistry, calls: &[ToolCall], ctx: &mut ToolContext) -> Self {
        let mut done = Vec::new();
        let mut pending = Vec::new();
        for call in calls {
            match registry.get(&call.name) {
                None => done.push(ToolResult {
                    tool_call_id: call.id.clone(),
                    name: call.name.clone(),
                    output: serde_json::json!({ "error": format!("unknown tool: {}", call.name) }),
                    is_error: true,
                }),
                Some(tool) if tool.needs_approval() => pending.push(call.clone()),
                Some(tool) => {
                    let args = parse_args(&call.arguments);
                    let output = tool.run(args, ctx);
                    let is_error = is_error_output(&output);
                    done.push(ToolResult {
                        tool_call_id: call.id.clone(),
                        name: call.name.clone(),
                        output,
                        is_error,
                    });
                }
            }
        }
        ToolTurn { done, pending }
    }

    /// Tool-call ids still awaiting an approve/reject decision.
    pub fn pending_ids(&self) -> Vec<String> {
        self.pending.iter().map(|c| c.id.clone()).collect()
    }

    pub fn is_complete(&self) -> bool {
        self.pending.is_empty()
    }

    /// Look up a still-pending call by id.
    pub fn pending_call(&self, id: &str) -> Option<&ToolCall> {
        self.pending.iter().find(|c| c.id == id)
    }

    /// Resolve one pending approval. `approved == false` records a clean
    /// rejection result without executing. Returns the produced [`ToolResult`].
    pub fn resolve(
        &mut self,
        registry: &ToolRegistry,
        id: &str,
        approved: bool,
        ctx: &mut ToolContext,
    ) -> Result<ToolResult, String> {
        let idx = self
            .pending
            .iter()
            .position(|c| c.id == id)
            .ok_or_else(|| format!("no pending tool call {id}"))?;
        let call = self.pending.remove(idx);

        let result = if !approved {
            ToolResult {
                tool_call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({ "error": "Rejected by user.", "rejected": true }),
                is_error: true,
            }
        } else {
            let tool = registry
                .get(&call.name)
                .ok_or_else(|| format!("unknown tool: {}", call.name))?;
            let output = tool.run(parse_args(&call.arguments), ctx);
            let is_error = is_error_output(&output);
            ToolResult {
                tool_call_id: call.id.clone(),
                name: call.name.clone(),
                output,
                is_error,
            }
        };
        self.done.push(result.clone());
        Ok(result)
    }

    /// All results so far, in call order.
    pub fn results(&self) -> &[ToolResult] {
        &self.done
    }

    /// The `Role::Tool` messages to append to the conversation and re-send to
    /// the model. Only meaningful once [`ToolTurn::is_complete`].
    pub fn into_messages(self) -> Vec<ChatMessage> {
        self.done.iter().map(ToolResult::to_message).collect()
    }
}

fn parse_args(raw: &str) -> Value {
    if raw.trim().is_empty() {
        return Value::Object(Default::default());
    }
    serde_json::from_str(raw).unwrap_or(Value::Object(Default::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::host::ScratchHost;
    use crate::tools::live_bridge::StaticLiveBridge;
    use crate::tools::subagent::NoopSubagentRunner;
    use crate::tools::todos::TodoStore;
    use std::sync::{Arc, Mutex};

    fn ctx(dir: &std::path::Path) -> ToolContext {
        let live = StaticLiveBridge {
            cwd: Some(dir.to_string_lossy().to_string()),
            workspace_root: Some(dir.to_string_lossy().to_string()),
            ..Default::default()
        };
        ToolContext::new(
            "s",
            Arc::new(live),
            Arc::new(ScratchHost::default()),
            Arc::new(Mutex::new(TodoStore::load(dir.join("t.json")))),
            Arc::new(NoopSubagentRunner),
        )
    }

    fn scratch() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("run-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn call(id: &str, name: &str, args: Value) -> ToolCall {
        ToolCall {
            id: id.into(),
            name: name.into(),
            arguments: args.to_string(),
        }
    }

    #[test]
    fn read_only_calls_auto_execute() {
        let d = scratch();
        std::fs::write(d.join("x.txt"), "content").unwrap();
        let reg = ToolRegistry::builtin();
        let mut c = ctx(&d);
        let turn = ToolTurn::begin(
            &reg,
            &[call(
                "1",
                "read_file",
                serde_json::json!({ "path": "x.txt" }),
            )],
            &mut c,
        );
        assert!(turn.is_complete());
        assert!(!turn.results()[0].is_error);
        let msgs = turn.into_messages();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].content.contains("content"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn mutating_call_pauses_then_continues_after_approval() {
        let d = scratch();
        let reg = ToolRegistry::builtin();
        let mut c = ctx(&d);
        let mut turn = ToolTurn::begin(
            &reg,
            &[call(
                "c1",
                "run_command",
                serde_json::json!({ "command": "echo continued" }),
            )],
            &mut c,
        );
        assert!(!turn.is_complete());
        assert_eq!(turn.pending_ids(), vec!["c1".to_string()]);

        let r = turn.resolve(&reg, "c1", true, &mut c).unwrap();
        assert_eq!(r.output["stdout"], "continued\n");
        assert!(turn.is_complete());
        let msgs = turn.into_messages();
        assert!(msgs[0].content.contains("continued"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn rejection_records_clean_result_without_executing() {
        let d = scratch();
        let reg = ToolRegistry::builtin();
        let mut c = ctx(&d);
        let mut turn = ToolTurn::begin(
            &reg,
            &[call(
                "c1",
                "write_file",
                serde_json::json!({ "path": "danger.txt", "content": "x" }),
            )],
            &mut c,
        );
        let r = turn.resolve(&reg, "c1", false, &mut c).unwrap();
        assert!(r.is_error);
        assert_eq!(r.output["rejected"], true);
        assert!(!d.join("danger.txt").exists());
        assert!(turn.is_complete());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unknown_tool_yields_error_result() {
        let d = scratch();
        let reg = ToolRegistry::builtin();
        let mut c = ctx(&d);
        let turn = ToolTurn::begin(
            &reg,
            &[call("1", "no_such_tool", serde_json::json!({}))],
            &mut c,
        );
        assert!(turn.is_complete());
        assert!(turn.results()[0].is_error);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn mixed_turn_executes_readonly_and_holds_mutating() {
        let d = scratch();
        std::fs::write(d.join("a.txt"), "aaa").unwrap();
        let reg = ToolRegistry::builtin();
        let mut c = ctx(&d);
        let turn = ToolTurn::begin(
            &reg,
            &[
                call("r", "read_file", serde_json::json!({ "path": "a.txt" })),
                call(
                    "w",
                    "write_file",
                    serde_json::json!({ "path": "b.txt", "content": "b" }),
                ),
            ],
            &mut c,
        );
        assert_eq!(turn.results().len(), 1);
        assert_eq!(turn.pending_ids(), vec!["w".to_string()]);
        let _ = std::fs::remove_dir_all(&d);
    }
}
