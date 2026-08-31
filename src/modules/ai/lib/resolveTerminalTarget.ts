import { useTabsStore, type WorkspaceTab } from "@/modules/tabs";
import { getLocalPtyId } from "@/modules/terminal/lib/terminalSessionRegistry";
import { useChatStore } from "../store/chatStore";

export type BashRunTarget = "current" | "new" | number | undefined;

export type ResolvedTerminalTarget = {
  tabId: number;
  paneId: string;
  kind: "local" | "ssh";
  /** Only present for `kind === "local"`. */
  localPtyId?: number;
  label: string;
};

const NEW_TAB_PTY_POLL_INTERVAL_MS = 50;
const NEW_TAB_PTY_POLL_TIMEOUT_MS = 3000;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function workspaceTabs(): WorkspaceTab[] {
  return useTabsStore.getState().tabs.filter((t): t is WorkspaceTab => t.kind === "workspace");
}

function labelFor(tab: WorkspaceTab): string {
  return tab.customTitle ?? tab.title ?? "Terminal";
}

/** Polls for a just-opened local tab's numeric pty id — `setLocalPtyId` is
 *  only populated once `openPty()` resolves asynchronously (see
 *  `terminalSessionRegistry.ts`), so it isn't available the instant
 *  `newTab()` returns. */
async function waitForLocalPtyId(paneId: string): Promise<number | { error: string }> {
  const deadline = Date.now() + NEW_TAB_PTY_POLL_TIMEOUT_MS;
  for (;;) {
    const id = getLocalPtyId(paneId);
    if (id !== undefined) return id;
    if (Date.now() >= deadline) {
      return { error: "timed out waiting for the new terminal's shell to start" };
    }
    await sleep(NEW_TAB_PTY_POLL_INTERVAL_MS);
  }
}

/** Resolves a specific (tab, pane) pair into an executable target. For local
 *  panes, `pollForPty` controls whether to wait out an in-flight pty spawn
 *  (only appropriate right after opening a brand-new tab) or fail fast
 *  (an already-open tab should already have a pty id — a missing one there
 *  is a real, if rare, transient state worth surfacing rather than masking). */
async function resolvePane(
  tab: WorkspaceTab,
  paneId: string,
  opts: { pollForPty: boolean },
): Promise<ResolvedTerminalTarget | { error: string }> {
  const session = tab.sessions[paneId];
  if (!session) return { error: `terminal pane no longer exists in tab "${labelFor(tab)}"` };

  if (session.kind === "ssh") {
    return { tabId: tab.id, paneId, kind: "ssh", label: labelFor(tab) };
  }

  let localPtyId = getLocalPtyId(paneId);
  if (localPtyId === undefined) {
    if (!opts.pollForPty) {
      return {
        error: `terminal "${labelFor(tab)}"'s shell hasn't finished starting yet — try again shortly`,
      };
    }
    const polled = await waitForLocalPtyId(paneId);
    if (typeof polled !== "number") return polled;
    localPtyId = polled;
  }
  return { tabId: tab.id, paneId, kind: "local", localPtyId, label: labelFor(tab) };
}

function resolveActivePane(
  tab: WorkspaceTab,
  opts: { pollForPty: boolean },
): Promise<ResolvedTerminalTarget | { error: string }> {
  return resolvePane(tab, tab.activePaneId, opts);
}

async function openNewTabAndResolve(): Promise<ResolvedTerminalTarget | { error: string }> {
  const tabId = useTabsStore.getState().newTab();
  const tab = useTabsStore.getState().tabs.find((t) => t.id === tabId) as WorkspaceTab | undefined;
  if (!tab) return { error: "failed to open a new terminal tab" };
  return resolveActivePane(tab, { pollForPty: true });
}

async function resolveDefault(chatSessionId: string): Promise<ResolvedTerminalTarget | { error: string }> {
  const bound = useChatStore.getState().boundTabs[chatSessionId];
  if (bound) {
    const tab = useTabsStore.getState().tabs.find((t) => t.id === bound.tabId);
    const stillValid = tab && tab.kind === "workspace" && !!tab.sessions[bound.paneId];
    if (stillValid) {
      return resolvePane(tab as WorkspaceTab, bound.paneId, { pollForPty: false });
    }
    // Bound tab/pane was closed — drop the stale binding and fall through
    // to a fresh resolution below, rather than erroring.
    useChatStore.getState().clearBoundTab(chatSessionId);
  }

  const { activeId, tabs } = useTabsStore.getState();
  const active = tabs.find((t) => t.id === activeId);
  if (active && active.kind === "workspace") {
    return resolveActivePane(active, { pollForPty: false });
  }

  // No terminal tab focused — fall back to the last open, non-cold terminal
  // tab (same backward-iteration heuristic `useAiLiveBridge.ts`'s `getCwd()`
  // already uses, generalized to any terminal kind, not just local).
  const candidates = workspaceTabs().filter((t) => !t.cold);
  const fallback = candidates[candidates.length - 1];
  if (fallback) {
    return resolveActivePane(fallback, { pollForPty: false });
  }

  // No terminal tab exists at all — open one.
  return openNewTabAndResolve();
}

/**
 * Resolves a `bash_run`/`bash_check_output`/`bash_send_keys` `target` into a
 * concrete terminal to execute against, and — on success — (re)binds the
 * chat session to it so subsequent calls without an explicit `target` reuse
 * the same terminal (sticky), regardless of which tab the user has focused
 * in the UI.
 */
export async function resolveTerminalTarget(
  chatSessionId: string,
  target: BashRunTarget,
): Promise<ResolvedTerminalTarget | { error: string }> {
  let resolved: ResolvedTerminalTarget | { error: string };

  if (target === "current") {
    const { activeId, tabs } = useTabsStore.getState();
    const active = tabs.find((t) => t.id === activeId);
    if (active?.kind !== "workspace") {
      resolved = { error: "No active terminal. Switch to a terminal tab first, or use target 'new'." };
    } else {
      resolved = await resolveActivePane(active, { pollForPty: false });
    }
  } else if (target === "new") {
    resolved = await openNewTabAndResolve();
  } else if (typeof target === "number") {
    const tabs = workspaceTabs();
    const tab = tabs[target - 1];
    if (!tab) {
      const available = tabs.map((t, i) => `${i + 1}: ${labelFor(t)}`).join(", ");
      resolved = { error: `No terminal at index ${target}. Available: ${available || "none"}` };
    } else {
      resolved = await resolveActivePane(tab, { pollForPty: false });
    }
  } else {
    resolved = await resolveDefault(chatSessionId);
  }

  if (!("error" in resolved)) {
    useChatStore.getState().setBoundTab(chatSessionId, { tabId: resolved.tabId, paneId: resolved.paneId });
  }
  return resolved;
}

/** Resolves against the chat session's existing bound tab only — no
 *  fallback/auto-open. Used by `bash_check_output`/`bash_send_keys`, which
 *  only make sense as a follow-up to a prior `bash_run` call. */
export async function resolveBoundTerminal(
  chatSessionId: string,
): Promise<ResolvedTerminalTarget | { error: string }> {
  const bound = useChatStore.getState().boundTabs[chatSessionId];
  if (!bound) {
    return { error: "no bound terminal for this chat yet — call bash_run first." };
  }
  const tab = useTabsStore.getState().tabs.find((t) => t.id === bound.tabId);
  if (tab?.kind !== "workspace" || !tab.sessions[bound.paneId]) {
    useChatStore.getState().clearBoundTab(chatSessionId);
    return { error: "the bound terminal was closed — call bash_run again to rebind." };
  }
  return resolvePane(tab, bound.paneId, { pollForPty: false });
}
