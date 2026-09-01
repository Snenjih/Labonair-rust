//! Agent / tool system + live-bridge (T11-004).
//!
//! Gives the AI companion the ability to interact with the app and the system:
//! filesystem tools (read/write/edit/search), shell execution, terminal
//! access, sub-agents and todo management — behind a security layer
//! (approval-gated mutating tools, a deny-list for sensitive paths on *both*
//! read and write) and a lazy live-bridge that exposes the currently active
//! terminal's cwd + buffer to the agent.
//!
//! Pure-Rust port of `reference-src/src/modules/ai/tools/*`,
//! `agents/*` and `lib/{security,todos,useAiLiveBridge}.ts`. Framework-agnostic:
//! the GPUI wiring lives in `labonair-ui`.

pub mod host;
pub mod live_bridge;
pub mod registry;
pub mod run;
pub mod security;
pub mod subagent;
pub mod todos;

pub use host::{
    run_shell_blocking, DirEntry, FileRead, GrepHit, NativeHost, ScratchHost, ShellOutput,
    ToolHost, AI_READ_CAP,
};
pub use live_bridge::{
    resolve_path, terminal_context_block, LiveBridge, NoLiveBridge, StaticLiveBridge,
    TERMINAL_CONTEXT_LINES,
};
pub use registry::{Tool, ToolContext, ToolRegistry};
pub use run::{ToolResult, ToolTurn};
pub use security::{
    check_destructive_command, check_readable, check_readable_resolved, check_shell_command,
    check_writable, check_writable_resolved, SafetyResult,
};
pub use subagent::{
    find_subagent, NoopSubagentRunner, SubagentDef, SubagentReport, SubagentRunner, SUBAGENTS,
    SUBAGENT_MAX_STEPS,
};
pub use todos::{new_todo_id, validate_todos, Todo, TodoStatus, TodoStore};
