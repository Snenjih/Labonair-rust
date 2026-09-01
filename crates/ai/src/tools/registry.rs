//! The tool interface + registry + the built-in tool set (T11-004).
//!
//! Port of `reference-src/src/modules/ai/tools/*`. Each [`Tool`] carries its
//! name, model-facing description, JSON-Schema input, an `needs_approval` flag,
//! and its execution logic. [`ToolRegistry::builtin`] assembles the full set
//! the provider advertises to the model.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use super::host::{FileRead, ToolHost, AI_READ_CAP};
use super::live_bridge::{resolve_path, LiveBridge};
use super::security::{
    check_readable, check_readable_resolved, check_shell_command, check_writable_resolved,
};
use super::subagent::{SubagentRunner, SUBAGENTS};
use super::todos::{new_todo_id, Todo, TodoStatus, TodoStore};
use crate::message::ToolDef;

/// Everything a tool needs to run. Rebuilt per agent turn so `read_cache`
/// reflects reads made earlier in the same run.
pub struct ToolContext {
    pub session_id: String,
    pub live: Arc<dyn LiveBridge>,
    pub host: Arc<dyn ToolHost>,
    pub todos: Arc<Mutex<TodoStore>>,
    pub subagents: Arc<dyn SubagentRunner>,
    /// Absolute paths the model has `read_file`'d this session (read-before-edit
    /// invariant). Shared so it survives across approval-gated follow-up turns.
    pub read_cache: Arc<Mutex<HashSet<String>>>,
    pub shell_timeout: Duration,
}

impl ToolContext {
    pub fn new(
        session_id: impl Into<String>,
        live: Arc<dyn LiveBridge>,
        host: Arc<dyn ToolHost>,
        todos: Arc<Mutex<TodoStore>>,
        subagents: Arc<dyn SubagentRunner>,
    ) -> Self {
        ToolContext {
            session_id: session_id.into(),
            live,
            host,
            todos,
            subagents,
            read_cache: Arc::new(Mutex::new(HashSet::new())),
            shell_timeout: Duration::from_secs(30),
        }
    }

    /// Use an externally-owned read cache (shared across turns).
    pub fn with_read_cache(mut self, cache: Arc<Mutex<HashSet<String>>>) -> Self {
        self.read_cache = cache;
        self
    }

    fn cache_has(&self, path: &str) -> bool {
        self.read_cache.lock().unwrap().contains(path)
    }

    fn cache_add(&self, path: String) {
        self.read_cache.lock().unwrap().insert(path);
    }

    fn cwd(&self) -> Option<String> {
        self.live.cwd().or_else(|| self.live.workspace_root())
    }

    fn workspace_root(&self) -> Option<String> {
        self.live.workspace_root().or_else(|| self.live.cwd())
    }
}

fn err(msg: impl Into<String>) -> Value {
    json!({ "error": msg.into() })
}

/// A callable tool advertised to the model.
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> Value;
    /// Mutating tools return `true`: execution pauses for user approval.
    fn needs_approval(&self) -> bool {
        false
    }
    /// Execute. The returned JSON is fed back to the model verbatim; an
    /// `"error"` key signals failure without aborting the run.
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value;

    fn to_def(&self) -> ToolDef {
        ToolDef {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
        }
    }
}

/// The full set of tools handed to the provider.
pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// The complete built-in tool set (FS, search, shell, terminal, todo,
    /// sub-agent).
    pub fn builtin() -> Self {
        ToolRegistry {
            tools: vec![
                Arc::new(ReadFile),
                Arc::new(ListDirectory),
                Arc::new(WriteFile),
                Arc::new(CreateDirectory),
                Arc::new(Edit),
                Arc::new(MultiEdit),
                Arc::new(Grep),
                Arc::new(Glob),
                Arc::new(RunCommand),
                Arc::new(TerminalRead),
                Arc::new(TerminalWrite),
                Arc::new(SuggestCommand),
                Arc::new(TodoWrite),
                Arc::new(RunSubagent),
            ],
        }
    }

    /// A read-only subset (used by sub-agents — no mutation, no recursion).
    pub fn read_only() -> Self {
        ToolRegistry {
            tools: vec![
                Arc::new(ReadFile),
                Arc::new(ListDirectory),
                Arc::new(Grep),
                Arc::new(Glob),
            ],
        }
    }

    pub fn tool_defs(&self) -> Vec<ToolDef> {
        self.tools.iter().map(|t| t.to_def()).collect()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.tools.iter().map(|t| t.name()).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

// ── FS: read ──────────────────────────────────────────────────────────────

struct ReadFile;
impl Tool for ReadFile {
    fn name(&self) -> &'static str {
        "read_file"
    }
    fn description(&self) -> &'static str {
        "Read a UTF-8 text file. Refuses binary, oversized, or sensitive files (.env, keys, credentials). Files larger than 200KB are truncated."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "Absolute path, or relative to the active terminal cwd." } },
            "required": ["path"]
        })
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let Some(path) = args.get("path").and_then(Value::as_str) else {
            return err("missing 'path'");
        };
        let abs = match resolve_path(path, ctx.cwd().as_deref()) {
            Ok(p) => p,
            Err(e) => return err(e),
        };
        if let Err(reason) = check_readable_resolved(&abs) {
            return json!({ "error": reason, "path": abs });
        }
        match ctx.host.read_file(&abs) {
            Ok(FileRead::Text { content, size }) => {
                ctx.cache_add(abs.clone());
                if content.len() > AI_READ_CAP {
                    let truncated: String = content.chars().take(AI_READ_CAP).collect();
                    json!({ "path": abs, "content": truncated, "size": size, "truncated": true })
                } else {
                    json!({ "path": abs, "content": content, "size": size })
                }
            }
            Ok(FileRead::Binary { size }) => {
                json!({ "error": "binary file refused", "path": abs, "size": size })
            }
            Ok(FileRead::TooLarge { size, limit }) => {
                json!({ "error": format!("file too large ({size} bytes, limit {limit})"), "path": abs })
            }
            Err(e) => json!({ "error": e, "path": abs }),
        }
    }
}

struct ListDirectory;
impl Tool for ListDirectory {
    fn name(&self) -> &'static str {
        "list_directory"
    }
    fn description(&self) -> &'static str {
        "List immediate entries (files + directories) in a directory. Hidden entries are omitted."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"]
        })
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let Some(path) = args.get("path").and_then(Value::as_str) else {
            return err("missing 'path'");
        };
        let abs = match resolve_path(path, ctx.cwd().as_deref()) {
            Ok(p) => p,
            Err(e) => return err(e),
        };
        if let Err(reason) = check_readable_resolved(&abs) {
            return json!({ "error": reason, "path": abs });
        }
        match ctx.host.list_dir(&abs) {
            Ok(entries) => json!({
                "path": abs,
                "entries": entries.iter().map(|e| json!({ "name": e.name, "kind": e.kind })).collect::<Vec<_>>()
            }),
            Err(e) => json!({ "error": e, "path": abs }),
        }
    }
}

// ── FS: mutate ────────────────────────────────────────────────────────────

struct WriteFile;
impl Tool for WriteFile {
    fn name(&self) -> &'static str {
        "write_file"
    }
    fn description(&self) -> &'static str {
        "Create or overwrite a file with the given content. Always asks the user before running. Prefer `edit` for in-place changes."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "path": { "type": "string" }, "content": { "type": "string" } },
            "required": ["path", "content"]
        })
    }
    fn needs_approval(&self) -> bool {
        true
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let (Some(path), Some(content)) = (
            args.get("path").and_then(Value::as_str),
            args.get("content").and_then(Value::as_str),
        ) else {
            return err("missing 'path' or 'content'");
        };
        let abs = match resolve_path(path, ctx.cwd().as_deref()) {
            Ok(p) => p,
            Err(e) => return err(e),
        };
        if let Err(reason) = check_writable_resolved(&abs) {
            return json!({ "error": reason, "path": abs });
        }
        match ctx.host.write_file(&abs, content) {
            Ok(()) => {
                ctx.cache_add(abs.clone());
                json!({ "path": abs, "bytesWritten": content.len(), "ok": true })
            }
            Err(e) => json!({ "error": e, "path": abs }),
        }
    }
}

struct CreateDirectory;
impl Tool for CreateDirectory {
    fn name(&self) -> &'static str {
        "create_directory"
    }
    fn description(&self) -> &'static str {
        "Create a directory (and any missing parents). Always asks the user before running."
    }
    fn parameters(&self) -> Value {
        json!({ "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] })
    }
    fn needs_approval(&self) -> bool {
        true
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let Some(path) = args.get("path").and_then(Value::as_str) else {
            return err("missing 'path'");
        };
        let abs = match resolve_path(path, ctx.cwd().as_deref()) {
            Ok(p) => p,
            Err(e) => return err(e),
        };
        if let Err(reason) = check_writable_resolved(&abs) {
            return json!({ "error": reason, "path": abs });
        }
        match ctx.host.create_dir(&abs) {
            Ok(()) => json!({ "path": abs, "ok": true }),
            Err(e) => json!({ "error": e, "path": abs }),
        }
    }
}

fn apply_edits(abs: &str, edits: &[(String, String, bool)], ctx: &mut ToolContext) -> Value {
    let original = match ctx.host.read_file(abs) {
        Ok(FileRead::Text { content, .. }) => content,
        Ok(FileRead::Binary { .. }) => {
            return json!({ "error": "binary file refused", "path": abs })
        }
        Ok(FileRead::TooLarge { size, .. }) => {
            return json!({ "error": format!("file too large ({size} bytes)"), "path": abs })
        }
        Err(e) => return json!({ "error": e, "path": abs }),
    };
    let mut content = original;
    let mut total = 0usize;
    for (old, new, all) in edits {
        if old == new {
            return json!({ "error": "old_string and new_string are identical", "path": abs });
        }
        if old.is_empty() {
            return json!({ "error": "old_string cannot be empty", "path": abs });
        }
        let count = content.matches(old.as_str()).count();
        if count == 0 {
            return json!({ "error": format!("old_string not found: {:?}", truncate(old, 80)), "path": abs });
        }
        if *all {
            content = content.replace(old.as_str(), new);
            total += count;
        } else {
            if count > 1 {
                return json!({ "error": "old_string is not unique. Provide more surrounding context, or set replace_all=true.", "path": abs });
            }
            content = content.replacen(old.as_str(), new, 1);
            total += 1;
        }
    }
    match ctx.host.write_file(abs, &content) {
        Ok(()) => {
            json!({ "ok": true, "replacements": total, "bytesWritten": content.len(), "path": abs })
        }
        Err(e) => json!({ "error": e, "path": abs }),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        s.chars().take(max).collect()
    } else {
        s.to_string()
    }
}

struct Edit;
impl Tool for Edit {
    fn name(&self) -> &'static str {
        "edit"
    }
    fn description(&self) -> &'static str {
        "Replace an exact string in a file. Requires read_file on this path first in the current session. `old_string` must be unique unless `replace_all: true`. Asks for approval."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "old_string": { "type": "string" },
                "new_string": { "type": "string" },
                "replace_all": { "type": "boolean" }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }
    fn needs_approval(&self) -> bool {
        true
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let (Some(path), Some(old), Some(new)) = (
            args.get("path").and_then(Value::as_str),
            args.get("old_string").and_then(Value::as_str),
            args.get("new_string").and_then(Value::as_str),
        ) else {
            return err("missing 'path', 'old_string' or 'new_string'");
        };
        let all = args
            .get("replace_all")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let abs = match resolve_path(path, ctx.cwd().as_deref()) {
            Ok(p) => p,
            Err(e) => return err(e),
        };
        if let Err(reason) = check_writable_resolved(&abs) {
            return json!({ "error": reason, "path": abs });
        }
        if !ctx.cache_has(&abs) {
            return json!({ "error": "must call read_file on this path first (read-before-edit invariant).", "path": abs });
        }
        apply_edits(&abs, &[(old.to_string(), new.to_string(), all)], ctx)
    }
}

struct MultiEdit;
impl Tool for MultiEdit {
    fn name(&self) -> &'static str {
        "multi_edit"
    }
    fn description(&self) -> &'static str {
        "Apply several exact-string replacements to a single file atomically. If any edit's old_string is missing or non-unique, the whole batch aborts before writing. Requires prior read_file. Asks for approval."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "edits": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "type": "object",
                        "properties": {
                            "old_string": { "type": "string" },
                            "new_string": { "type": "string" },
                            "replace_all": { "type": "boolean" }
                        },
                        "required": ["old_string", "new_string"]
                    }
                }
            },
            "required": ["path", "edits"]
        })
    }
    fn needs_approval(&self) -> bool {
        true
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let Some(path) = args.get("path").and_then(Value::as_str) else {
            return err("missing 'path'");
        };
        let Some(raw_edits) = args.get("edits").and_then(Value::as_array) else {
            return err("missing 'edits'");
        };
        let mut edits = Vec::new();
        for e in raw_edits {
            let (Some(old), Some(new)) = (
                e.get("old_string").and_then(Value::as_str),
                e.get("new_string").and_then(Value::as_str),
            ) else {
                return err("each edit needs 'old_string' and 'new_string'");
            };
            edits.push((
                old.to_string(),
                new.to_string(),
                e.get("replace_all")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ));
        }
        if edits.is_empty() {
            return err("'edits' must not be empty");
        }
        let abs = match resolve_path(path, ctx.cwd().as_deref()) {
            Ok(p) => p,
            Err(e) => return err(e),
        };
        if let Err(reason) = check_writable_resolved(&abs) {
            return json!({ "error": reason, "path": abs });
        }
        if !ctx.cache_has(&abs) {
            return json!({ "error": "must call read_file on this path first (read-before-edit invariant).", "path": abs });
        }
        apply_edits(&abs, &edits, ctx)
    }
}

// ── Search ────────────────────────────────────────────────────────────────

fn resolve_root(args: &Value, ctx: &ToolContext) -> Result<String, String> {
    if let Some(r) = args
        .get("root")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        return resolve_path(r, ctx.cwd().as_deref());
    }
    ctx.workspace_root()
        .ok_or_else(|| "no workspace root or active cwd; pass `root` explicitly.".to_string())
}

struct Grep;
impl Tool for Grep {
    fn name(&self) -> &'static str {
        "grep"
    }
    fn description(&self) -> &'static str {
        "Search file contents in the workspace with a regular expression. Honors .gitignore. Returns up to `max_results` (default 200) {path,line,text} hits."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "root": { "type": "string" },
                "glob": { "type": "array", "items": { "type": "string" } },
                "case_insensitive": { "type": "boolean" },
                "max_results": { "type": "integer", "minimum": 1, "maximum": 2000 }
            },
            "required": ["pattern"]
        })
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let Some(pattern) = args.get("pattern").and_then(Value::as_str) else {
            return err("missing 'pattern'");
        };
        let root = match resolve_root(&args, ctx) {
            Ok(r) => r,
            Err(e) => return err(e),
        };
        if let Err(reason) = check_readable(&root) {
            return json!({ "error": reason, "root": root });
        }
        let globs: Vec<String> = args
            .get("glob")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let ci = args
            .get("case_insensitive")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let max = args
            .get("max_results")
            .and_then(Value::as_u64)
            .unwrap_or(200) as usize;
        match ctx.host.grep(pattern, &root, &globs, ci, max) {
            Ok((hits, truncated)) => {
                let hits: Vec<Value> = hits
                    .into_iter()
                    .filter(|h| check_readable(&h.path).is_ok())
                    .map(
                        |h| json!({ "path": h.path, "rel": h.rel, "line": h.line, "text": h.text }),
                    )
                    .collect();
                json!({ "root": root, "hits": hits, "truncated": truncated })
            }
            Err(e) => json!({ "error": e, "root": root }),
        }
    }
}

struct Glob;
impl Tool for Glob {
    fn name(&self) -> &'static str {
        "glob"
    }
    fn description(&self) -> &'static str {
        "Find files by path pattern (gitignore-aware). Patterns use globset syntax: `**/*.rs`, `src/**/test_*.py`. Returns up to `max_results` matches."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "root": { "type": "string" },
                "max_results": { "type": "integer", "minimum": 1, "maximum": 2000 }
            },
            "required": ["pattern"]
        })
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let Some(pattern) = args.get("pattern").and_then(Value::as_str) else {
            return err("missing 'pattern'");
        };
        let root = match resolve_root(&args, ctx) {
            Ok(r) => r,
            Err(e) => return err(e),
        };
        if let Err(reason) = check_readable(&root) {
            return json!({ "error": reason, "root": root });
        }
        let max = args
            .get("max_results")
            .and_then(Value::as_u64)
            .unwrap_or(500) as usize;
        match ctx.host.glob(pattern, &root, max) {
            Ok((hits, truncated)) => {
                let hits: Vec<String> = hits
                    .into_iter()
                    .filter(|p| check_readable(p).is_ok())
                    .collect();
                json!({ "root": root, "hits": hits, "truncated": truncated })
            }
            Err(e) => json!({ "error": e, "root": root }),
        }
    }
}

// ── Shell ─────────────────────────────────────────────────────────────────

struct RunCommand;
impl Tool for RunCommand {
    fn name(&self) -> &'static str {
        "run_command"
    }
    fn description(&self) -> &'static str {
        "Run a shell command in this session's working directory. Returns stdout/stderr and the exit code. Runs async with a timeout. Asks for user approval. NEVER invoke interactive tools (vim, less, top)."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "timeout_secs": { "type": "integer", "minimum": 1, "maximum": 300 }
            },
            "required": ["command"]
        })
    }
    fn needs_approval(&self) -> bool {
        true
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let Some(command) = args.get("command").and_then(Value::as_str) else {
            return err("missing 'command'");
        };
        if let Err(reason) = check_shell_command(command) {
            return err(reason);
        }
        let timeout = args
            .get("timeout_secs")
            .and_then(Value::as_u64)
            .map(Duration::from_secs)
            .unwrap_or(ctx.shell_timeout);
        let cwd = ctx.cwd();
        let out = ctx.host.run_shell(command, cwd.as_deref(), timeout);
        json!({
            "command": command,
            "stdout": out.stdout,
            "stderr": out.stderr,
            "exit_code": out.exit_code,
            "timed_out": out.timed_out,
        })
    }
}

// ── Terminal ──────────────────────────────────────────────────────────────

struct TerminalRead;
impl Tool for TerminalRead {
    fn name(&self) -> &'static str {
        "terminal_read"
    }
    fn description(&self) -> &'static str {
        "Read the current working directory and the last lines of the active terminal's buffer, via the live bridge. Auto-executes (read-only)."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "max_lines": { "type": "integer", "minimum": 1, "maximum": 2000 } }
        })
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let n = args.get("max_lines").and_then(Value::as_u64).unwrap_or(200) as usize;
        json!({
            "cwd": ctx.live.cwd(),
            "buffer": ctx.live.terminal_context(n),
        })
    }
}

struct TerminalWrite;
impl Tool for TerminalWrite {
    fn name(&self) -> &'static str {
        "terminal_write"
    }
    fn description(&self) -> &'static str {
        "Send a command to the active terminal's shell (it executes). Asks for user approval."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "command": { "type": "string" } },
            "required": ["command"]
        })
    }
    fn needs_approval(&self) -> bool {
        true
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let Some(command) = args.get("command").and_then(Value::as_str) else {
            return err("missing 'command'");
        };
        if let Err(reason) = check_shell_command(command) {
            return err(reason);
        }
        if ctx.live.send_to_active_terminal(command) {
            json!({ "command": command, "sent": true })
        } else {
            json!({ "error": "no active terminal to send to", "command": command })
        }
    }
}

struct SuggestCommand;
impl Tool for SuggestCommand {
    fn name(&self) -> &'static str {
        "suggest_command"
    }
    fn description(&self) -> &'static str {
        "Type a single shell command into the user's active terminal at the prompt WITHOUT executing it. Use when the answer to the user's question IS a command."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "explanation": { "type": "string" }
            },
            "required": ["command"]
        })
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let Some(command) = args.get("command").and_then(Value::as_str) else {
            return err("missing 'command'");
        };
        if let Err(reason) = check_shell_command(command) {
            return err(reason);
        }
        let trimmed = command.trim_end_matches('\n');
        if ctx.live.inject_into_active_pty(trimmed) {
            json!({ "command": trimmed, "injected": true, "explanation": args.get("explanation") })
        } else {
            json!({ "error": "no active terminal to inject into", "command": trimmed })
        }
    }
}

// ── Todo ──────────────────────────────────────────────────────────────────

struct TodoWrite;
impl Tool for TodoWrite {
    fn name(&self) -> &'static str {
        "todo_write"
    }
    fn description(&self) -> &'static str {
        "Replace your current task list. Use for any non-trivial multi-step task (>=3 steps). Mark exactly one item in_progress. Always pass the FULL list. Auto-executes."
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "title": { "type": "string" },
                            "description": { "type": "string" },
                            "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
                        },
                        "required": ["title", "status"]
                    }
                }
            },
            "required": ["todos"]
        })
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let Some(raw) = args.get("todos").and_then(Value::as_array) else {
            return err("missing 'todos'");
        };
        let mut todos = Vec::new();
        for t in raw {
            let Some(title) = t.get("title").and_then(Value::as_str) else {
                return err("each todo needs a 'title'");
            };
            let status = match t.get("status").and_then(Value::as_str) {
                Some("pending") => TodoStatus::Pending,
                Some("in_progress") => TodoStatus::InProgress,
                Some("completed") => TodoStatus::Completed,
                _ => return err("each todo needs a valid 'status'"),
            };
            todos.push(Todo {
                id: t
                    .get("id")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .unwrap_or_else(new_todo_id),
                title: title.to_string(),
                description: t
                    .get("description")
                    .and_then(Value::as_str)
                    .map(String::from),
                status,
            });
        }
        let in_progress = todos
            .iter()
            .find(|t| t.status == TodoStatus::InProgress)
            .map(|t| t.title.clone());
        let count = todos.len();
        match ctx.todos.lock().unwrap().set_todos(&ctx.session_id, todos) {
            Ok(()) => json!({ "ok": true, "count": count, "inProgress": in_progress }),
            Err(e) => err(e),
        }
    }
}

// ── Sub-agent ─────────────────────────────────────────────────────────────

struct RunSubagent;
impl Tool for RunSubagent {
    fn name(&self) -> &'static str {
        "run_subagent"
    }
    fn description(&self) -> &'static str {
        "Spawn an isolated read-only subagent with a fresh message history to delegate a self-contained investigation (large search, code review, security audit). Returns a single text summary. Auto-executes."
    }
    fn parameters(&self) -> Value {
        let types: Vec<&str> = SUBAGENTS.iter().map(|s| s.id).collect();
        json!({
            "type": "object",
            "properties": {
                "type": { "type": "string", "enum": types },
                "prompt": { "type": "string", "description": "Self-contained instruction. The subagent has no memory of the conversation." },
                "description": { "type": "string" }
            },
            "required": ["type", "prompt"]
        })
    }
    fn run(&self, args: Value, ctx: &mut ToolContext) -> Value {
        let Some(type_) = args.get("type").and_then(Value::as_str) else {
            return err("missing 'type'");
        };
        let Some(prompt) = args.get("prompt").and_then(Value::as_str) else {
            return err("missing 'prompt'");
        };
        if SUBAGENTS.iter().all(|s| s.id != type_) {
            return err(format!("unknown subagent type: {type_}"));
        }
        let runner = ctx.subagents.clone();
        match runner.run(type_, prompt, ctx) {
            Ok(report) => json!({
                "type": type_,
                "description": args.get("description"),
                "summary": report.summary,
                "steps": report.steps,
            }),
            Err(e) => json!({ "error": e, "type": type_ }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::host::ScratchHost;
    use crate::tools::live_bridge::StaticLiveBridge;
    use crate::tools::subagent::NoopSubagentRunner;

    fn ctx_in(dir: &std::path::Path) -> ToolContext {
        let live = StaticLiveBridge {
            cwd: Some(dir.to_string_lossy().to_string()),
            workspace_root: Some(dir.to_string_lossy().to_string()),
            ..Default::default()
        };
        let todos = TodoStore::load(dir.join("todos.json"));
        ToolContext::new(
            "sess",
            Arc::new(live),
            Arc::new(ScratchHost::default()),
            Arc::new(Mutex::new(todos)),
            Arc::new(NoopSubagentRunner),
        )
    }

    fn scratch() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("tools-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn builtin_registry_advertises_all_tools() {
        let reg = ToolRegistry::builtin();
        let defs = reg.tool_defs();
        assert_eq!(defs.len(), 14);
        for name in [
            "read_file",
            "write_file",
            "edit",
            "grep",
            "glob",
            "run_command",
            "todo_write",
            "run_subagent",
        ] {
            assert!(reg.get(name).is_some(), "missing {name}");
        }
        assert!(reg.get("write_file").unwrap().needs_approval());
        assert!(!reg.get("read_file").unwrap().needs_approval());
    }

    #[test]
    fn read_write_grep_glob_roundtrip() {
        let d = scratch();
        std::fs::write(d.join("a.txt"), "hello\nneedle here\n").unwrap();
        std::fs::write(d.join("b.rs"), "fn main() {}\n").unwrap();
        let mut ctx = ctx_in(&d);

        let reg = ToolRegistry::builtin();
        let r = reg
            .get("read_file")
            .unwrap()
            .run(json!({ "path": "a.txt" }), &mut ctx);
        assert_eq!(r["content"], "hello\nneedle here\n");
        assert!(ctx
            .read_cache
            .lock()
            .unwrap()
            .contains(&d.join("a.txt").to_string_lossy().to_string()));

        let g = reg
            .get("grep")
            .unwrap()
            .run(json!({ "pattern": "needle" }), &mut ctx);
        assert_eq!(g["hits"].as_array().unwrap().len(), 1);

        let gl = reg
            .get("glob")
            .unwrap()
            .run(json!({ "pattern": "*.rs" }), &mut ctx);
        assert_eq!(gl["hits"].as_array().unwrap().len(), 1);

        let w = reg
            .get("write_file")
            .unwrap()
            .run(json!({ "path": "c.txt", "content": "written" }), &mut ctx);
        assert_eq!(w["ok"], true);
        assert_eq!(std::fs::read_to_string(d.join("c.txt")).unwrap(), "written");

        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn deny_list_blocks_read_and_write() {
        let d = scratch();
        std::fs::write(d.join(".env"), "SECRET=1").unwrap();
        let mut ctx = ctx_in(&d);
        let reg = ToolRegistry::builtin();

        let r = reg
            .get("read_file")
            .unwrap()
            .run(json!({ "path": ".env" }), &mut ctx);
        assert!(r.get("error").is_some());
        let w = reg
            .get("write_file")
            .unwrap()
            .run(json!({ "path": ".env", "content": "x" }), &mut ctx);
        assert!(w.get("error").is_some());
        let w2 = reg
            .get("write_file")
            .unwrap()
            .run(json!({ "path": "/etc/passwd", "content": "x" }), &mut ctx);
        assert!(w2.get("error").is_some());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn edit_enforces_read_before_edit() {
        let d = scratch();
        std::fs::write(d.join("f.txt"), "one two three").unwrap();
        let mut ctx = ctx_in(&d);
        let reg = ToolRegistry::builtin();

        let blind = reg.get("edit").unwrap().run(
            json!({ "path": "f.txt", "old_string": "two", "new_string": "2" }),
            &mut ctx,
        );
        assert!(blind["error"]
            .as_str()
            .unwrap()
            .contains("read-before-edit"));

        reg.get("read_file")
            .unwrap()
            .run(json!({ "path": "f.txt" }), &mut ctx);
        let ok = reg.get("edit").unwrap().run(
            json!({ "path": "f.txt", "old_string": "two", "new_string": "2" }),
            &mut ctx,
        );
        assert_eq!(ok["ok"], true);
        assert_eq!(
            std::fs::read_to_string(d.join("f.txt")).unwrap(),
            "one 2 three"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn run_command_executes_and_reports_exit() {
        let d = scratch();
        let mut ctx = ctx_in(&d);
        let reg = ToolRegistry::builtin();
        let r = reg
            .get("run_command")
            .unwrap()
            .run(json!({ "command": "echo hi && exit 3" }), &mut ctx);
        assert_eq!(r["stdout"], "hi\n");
        assert_eq!(r["exit_code"], 3);
        let blocked = reg
            .get("run_command")
            .unwrap()
            .run(json!({ "command": "rm -rf /" }), &mut ctx);
        assert!(blocked.get("error").is_some());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn terminal_read_uses_live_bridge() {
        let d = scratch();
        let live = StaticLiveBridge {
            cwd: Some("/live/cwd".into()),
            terminal_buffer: Some("l1\nl2\nl3".into()),
            ..Default::default()
        };
        let todos = TodoStore::load(d.join("t.json"));
        let mut ctx = ToolContext::new(
            "s",
            Arc::new(live),
            Arc::new(ScratchHost::default()),
            Arc::new(Mutex::new(todos)),
            Arc::new(NoopSubagentRunner),
        );
        let r = ToolRegistry::builtin()
            .get("terminal_read")
            .unwrap()
            .run(json!({ "max_lines": 2 }), &mut ctx);
        assert_eq!(r["cwd"], "/live/cwd");
        assert_eq!(r["buffer"], "l2\nl3");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn todo_write_validates_and_persists() {
        let d = scratch();
        let mut ctx = ctx_in(&d);
        let reg = ToolRegistry::builtin();
        let bad = reg.get("todo_write").unwrap().run(
            json!({ "todos": [
                { "title": "a", "status": "in_progress" },
                { "title": "b", "status": "in_progress" }
            ] }),
            &mut ctx,
        );
        assert!(bad.get("error").is_some());
        let ok = reg.get("todo_write").unwrap().run(
            json!({ "todos": [ { "title": "a", "status": "in_progress" } ] }),
            &mut ctx,
        );
        assert_eq!(ok["count"], 1);
        assert_eq!(ok["inProgress"], "a");
        assert_eq!(ctx.todos.lock().unwrap().get("sess").len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn run_subagent_dispatches_to_runner() {
        let d = scratch();
        let mut ctx = ctx_in(&d);
        let r = ToolRegistry::builtin()
            .get("run_subagent")
            .unwrap()
            .run(json!({ "type": "explore", "prompt": "find X" }), &mut ctx);
        assert_eq!(r["summary"], "noop");
        let bad = ToolRegistry::builtin()
            .get("run_subagent")
            .unwrap()
            .run(json!({ "type": "bogus", "prompt": "x" }), &mut ctx);
        assert!(bad.get("error").is_some());
        let _ = std::fs::remove_dir_all(&d);
    }
}
