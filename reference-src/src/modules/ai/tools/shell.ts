import { tool } from "ai";
import { z } from "zod";
import { invoke } from "@tauri-apps/api/core";
import { useNotificationStore } from "@/modules/notifications/store/useNotificationStore";
import { usePreferencesStore } from "@/modules/settings/preferences";
import { native } from "../lib/native";
import { resolveBoundTerminal, resolveTerminalTarget } from "../lib/resolveTerminalTarget";
import { checkShellCommand } from "../lib/security";
import type { ToolContext } from "./context";

function truncate(s: string, max: number): string {
  return s.length > max ? `${s.slice(0, max - 1)}…` : s;
}

/** Surfaces a notification for a command that ran without any visible
 *  terminal tab (headless `bash_run_headless`, or a `bash_background`
 *  spawn) — gated on the user's preference, so headless/background AI
 *  activity is never *completely* invisible unless they opt out. A plain
 *  synchronous call, not a Tauri event round-trip: the whole AI tool loop
 *  already runs in this renderer, unlike the MCP bridge's `mcp_activity`
 *  event, which genuinely crosses a process boundary. */
function notifyHeadlessCommand(action: "bash_run_headless" | "bash_background", command: string): void {
  if (!usePreferencesStore.getState().aiNotifyOnHeadlessCommand) return;
  useNotificationStore.getState().addNotification({
    type: "info",
    title:
      action === "bash_background" ? "Agent started a background process" : "Agent ran a headless command",
    message: truncate(command, 120),
    source: "AI",
  });
}

/**
 * Per-session lazy shell-session id. The agent gets one persistent shell per
 * chat session, so cwd survives across tool calls (cd, mkdir+cd, etc).
 * Stored as Promise<number> so concurrent first calls share one creation.
 * Backs `bash_run_headless` only — `bash_run` (visible/tab-bound) runs
 * inside the real terminal's own shell instead, see
 * `src/modules/ai/lib/resolveTerminalTarget.ts`.
 */
const sessionShells = new Map<string, Promise<number>>();

async function isShellAlive(shellId: number): Promise<boolean> {
  try {
    const r = await native.shellSessionRun(shellId, "echo __nx_ok__", 3);
    return r.exit_code === 0 && r.stdout.includes("__nx_ok__");
  } catch {
    return false;
  }
}

async function getSessionShell(sessionId: string, cwd: string | null): Promise<number> {
  const existing = sessionShells.get(sessionId);
  if (existing !== undefined) {
    try {
      const shellId = await existing;
      if (await isShellAlive(shellId)) return shellId;
    } catch {
      // Previous creation failed; fall through
    }
    sessionShells.delete(sessionId);
  }
  const p = native.shellSessionOpen(cwd);
  sessionShells.set(sessionId, p);
  return p;
}

/**
 * Removes a session's cached shell entry and closes the backing Rust shell
 * process (`shell_session_close`). Previously only cleared the JS-side map,
 * leaving the actual spawned shell process running server-side for the rest
 * of the app's lifetime — every AI session that ever called `bash_run` leaked
 * one. Safe to call for a session that never opened a shell (nothing to await).
 */
export async function clearSessionShell(sessionId: string): Promise<void> {
  const pending = sessionShells.get(sessionId);
  sessionShells.delete(sessionId);
  if (!pending) return;
  try {
    const shellId = await pending;
    await native.shellSessionClose(shellId);
  } catch {
    // Shell was never actually created (creation promise rejected) or is
    // already gone — nothing to close.
  }
}

export function buildShellTools(ctx: ToolContext) {
  return {
    bash_run: tool({
      description:
        "Run a shell command visibly inside a real terminal tab — indistinguishable from the user typing it themselves. This is the default way to execute anything the user should be able to watch (debugging, builds, tests, fixes). " +
        "Default (no `target`): uses this chat's bound terminal — picked once (the active tab, or the last open one, or a freshly opened one if none exists) and then reused for every later `bash_run` call in this conversation, even if the user switches which tab is focused, until you pass a different `target` or the bound tab gets closed. " +
        '`target`: "current" = rebind to whichever terminal tab is focused right now, "new" = open a fresh local terminal tab and bind to it, or a 1-based terminal-tab index to rebind to a specific open tab. ' +
        "Returns merged `output` (stdout+stderr interleaved, since this runs in a real terminal, not a separate pipe) and `exit_code`. If the command doesn't finish within `timeout_secs`, returns `still_running: true` with the output so far — use `bash_check_output` to keep watching it, or `bash_send_keys` to answer a prompt (e.g. sudo password) or send Ctrl+C. " +
        "NEVER invoke interactive tools (vim, less, top) — they will hang. Asks for user approval.",
      inputSchema: z.object({
        command: z.string(),
        target: z.union([z.literal("current"), z.literal("new"), z.number().int().min(1)]).optional(),
        timeout_secs: z.number().int().min(1).max(300).optional(),
      }),
      needsApproval: true,
      execute: async ({ command, target, timeout_secs }) => {
        const safety = checkShellCommand(command);
        if (!safety.ok) return { error: safety.reason };
        const sid = ctx.getSessionId();
        if (!sid) return { error: "no active chat session" };

        const resolved = await resolveTerminalTarget(sid, target);
        if ("error" in resolved) return { error: resolved.error };

        try {
          const r = await native.terminalExecRunCommand({
            kind: resolved.kind,
            sessionId: resolved.kind === "ssh" ? resolved.paneId : undefined,
            localPtyId: resolved.localPtyId,
            command,
            timeoutMs: (timeout_secs ?? 30) * 1000,
          });
          return {
            command,
            output: r.output,
            exit_code: r.exit_code,
            still_running: r.still_running,
            tab: resolved.label,
          };
        } catch (e) {
          return { error: String(e) };
        }
      },
    }),

    bash_check_output: tool({
      description:
        "Peek at new output from the terminal this chat is bound to, without running anything — use after a `bash_run` call returned `still_running: true`, to see progress or check whether it has since finished (`exit_code` becomes non-null once it does). Waits up to `wait_ms` (default 1000) for new output before returning. Does not include output from before this call was made. Auto-executes (read-only).",
      inputSchema: z.object({
        wait_ms: z.number().int().min(1).max(30_000).optional(),
      }),
      execute: async ({ wait_ms }) => {
        const sid = ctx.getSessionId();
        if (!sid) return { error: "no active chat session" };

        const resolved = await resolveBoundTerminal(sid);
        if ("error" in resolved) return { error: resolved.error };

        try {
          const r = await native.terminalExecPeekOutput({
            kind: resolved.kind,
            sessionId: resolved.kind === "ssh" ? resolved.paneId : undefined,
            localPtyId: resolved.localPtyId,
            waitMs: wait_ms,
          });
          return {
            output: r.output,
            exit_code: r.exit_code,
            still_running: r.still_running,
            tab: resolved.label,
          };
        } catch (e) {
          return { error: String(e) };
        }
      },
    }),

    bash_send_keys: tool({
      description:
        "Send raw keystrokes to the terminal this chat is bound to — use to answer an interactive prompt a `bash_run` command is waiting on (e.g. a sudo password, a y/n confirmation) or to send a control character (e.g. the two characters \\u0003 for Ctrl+C to interrupt a stuck command). Does not wait for a response — call `bash_check_output` afterward to see the result. Asks for user approval.",
      inputSchema: z.object({
        data: z.string(),
      }),
      needsApproval: true,
      execute: async ({ data }) => {
        const sid = ctx.getSessionId();
        if (!sid) return { error: "no active chat session" };

        const resolved = await resolveBoundTerminal(sid);
        if ("error" in resolved) return { error: resolved.error };

        try {
          await native.terminalExecSendKeys({
            kind: resolved.kind,
            sessionId: resolved.kind === "ssh" ? resolved.paneId : undefined,
            localPtyId: resolved.localPtyId,
            data,
          });
          return { ok: true, tab: resolved.label };
        } catch (e) {
          return { error: String(e) };
        }
      },
    }),

    bash_run_headless: tool({
      description:
        "Run a foreground shell command WITHOUT a visible terminal tab — in this session's persistent hidden agent shell. cwd persists across calls (so `cd foo` then `bash_run_headless pwd` works). Prefer `bash_run` for anything the user should be able to watch; use this only when you deliberately don't want the command to appear in any visible terminal tab. For long-running or daemon processes, use `bash_background` instead. NEVER invoke interactive tools (vim, less, top) — they will hang. Asks for user approval.",
      inputSchema: z.object({
        command: z.string(),
        timeout_secs: z.number().int().min(1).max(300).optional(),
      }),
      needsApproval: true,
      execute: async ({ command, timeout_secs }) => {
        const safety = checkShellCommand(command);
        if (!safety.ok) return { error: safety.reason };

        // Route through the active SSH session when user is in a remote tab.
        const sshTabId = ctx.getActiveSshTabId();
        if (sshTabId) {
          try {
            const r = await invoke<{ stdout: string; stderr: string; exit_code: number }>(
              "ssh_exec_command",
              { sessionId: sshTabId, command },
            );
            notifyHeadlessCommand("bash_run_headless", command);
            return {
              command,
              stdout: r.stdout,
              stderr: r.stderr,
              exit_code: r.exit_code,
              remote: true,
            };
          } catch (e) {
            return { error: String(e) };
          }
        }

        const sid = ctx.getSessionId();
        if (!sid) return { error: "no active chat session" };
        try {
          const shellId = await getSessionShell(sid, ctx.getCwd());
          const r = await native.shellSessionRun(shellId, command, timeout_secs);
          notifyHeadlessCommand("bash_run_headless", command);
          return {
            command,
            stdout: r.stdout,
            stderr: r.stderr,
            exit_code: r.exit_code,
            timed_out: r.timed_out,
            truncated: r.truncated,
            cwd_after: r.cwd_after,
          };
        } catch (e) {
          return { error: String(e) };
        }
      },
    }),

    bash_background: tool({
      description:
        "Spawn a long-running background process (e.g. `pnpm dev`, `cargo watch`, log tailers). Returns a handle; use `bash_logs` to read its output and `bash_kill` to stop it. Output is captured into a 4MB ring buffer. Always headless (no visible terminal tab). Asks for user approval.",
      inputSchema: z.object({
        command: z.string(),
        cwd: z.string().nullable().optional(),
      }),
      needsApproval: true,
      execute: async ({ command, cwd }) => {
        const safety = checkShellCommand(command);
        if (!safety.ok) return { error: safety.reason };
        const effectiveCwd = cwd ?? ctx.getCwd();
        try {
          const handle = await native.shellBgSpawn(command, effectiveCwd);
          notifyHeadlessCommand("bash_background", command);
          return { handle, command, cwd: effectiveCwd, ok: true };
        } catch (e) {
          return { error: String(e) };
        }
      },
    }),

    bash_logs: tool({
      description:
        "Read accumulated logs from a `bash_background` process. Pass `since_offset` from the previous response's `next_offset` to tail incrementally. `dropped` reports bytes evicted by the ring buffer.",
      inputSchema: z.object({
        handle: z.number().int(),
        since_offset: z.number().int().optional(),
      }),
      execute: async ({ handle, since_offset }) => {
        try {
          const r = await native.shellBgLogs(handle, since_offset);
          return r;
        } catch (e) {
          return { error: String(e) };
        }
      },
    }),

    bash_list: tool({
      description:
        "List all background processes spawned by `bash_background` in this app — running and exited. **Always call this BEFORE spawning a new long-running process** (especially dev servers like `pnpm dev`, `next dev`, `vite`) to avoid duplicates. If a matching process is already running, reuse it (call `open_preview` again instead of respawning). Auto-executes.",
      inputSchema: z.object({}),
      execute: async () => {
        try {
          const list = await native.shellBgList();
          return { processes: list };
        } catch (e) {
          return { error: String(e) };
        }
      },
    }),

    bash_kill: tool({
      description:
        "Terminate a `bash_background` process by handle. Idempotent — kills nothing if the handle is unknown or already exited.",
      inputSchema: z.object({ handle: z.number().int() }),
      execute: async ({ handle }) => {
        try {
          await native.shellBgKill(handle);
          return { handle, ok: true };
        } catch (e) {
          return { error: String(e) };
        }
      },
    }),
  } as const;
}
