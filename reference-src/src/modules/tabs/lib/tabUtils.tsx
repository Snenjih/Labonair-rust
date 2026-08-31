import { fileIconUrl } from "@/modules/explorer/lib/iconResolver";
import type { ConnectionEntry, ConnectionStatus } from "@/modules/hosts/store/connectionStatusStore";
import { useHostsStore } from "@/modules/hosts/store/hostsStore";
import type { Host } from "@/modules/hosts/types";
import type { SidebarTabInfoType } from "@/modules/settings/store";
import type { TransferJob } from "@/modules/sftp/store/transferStore";
import { getLocalOpenedAt } from "@/modules/terminal/lib/terminalSessionRegistry";
import {
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
} from "@/components/ui/dropdown-menu";
import {
  CloudServerIcon,
  ComputerTerminal02Icon,
  Folder01Icon,
  Folder02Icon,
  GitBranchIcon,
  GitCompareIcon,
  Globe02Icon,
  Home03Icon,
  PencilEdit02Icon,
  TerminalIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { Tab, WorkspaceTab } from "../types";

// --- Shared hook ---

export function useRecentHosts(limit = 5) {
  const hosts = useHostsStore((s) => s.hosts);
  return [...hosts].sort((a, b) => (b.last_connected_at ?? 0) - (a.last_connected_at ?? 0)).slice(0, limit);
}

// --- Shared label function ---

export function labelFor(t: Tab): string {
  if (t.kind === "editor") return t.title;
  if (t.kind === "preview") return t.title;
  if (t.kind === "ai-diff") return t.title;
  if (t.kind === "home") return t.title;
  if (t.kind === "sftp") return t.title;
  if (t.kind === "git-graph") return t.title;
  if (t.kind === "git-diff") return t.title;
  if (t.kind === "commit-diff") return t.title;
  const wt = t as WorkspaceTab;
  if (wt.customTitle) return wt.customTitle;
  const activeSession = wt.sessions[wt.activePaneId];
  if (activeSession?.processTitle) return activeSession.processTitle;
  if (activeSession?.kind === "local" && activeSession.cwd) {
    const parts = activeSession.cwd.split("/").filter(Boolean);
    return parts.length ? parts[parts.length - 1] : "/";
  }
  return wt.title;
}

// --- Sidebar tab info line ---

export type SidebarInfoSegment =
  | { type: "path" | "host" | "uptime" | "transfer"; text: string }
  | { type: "connection"; text: string; status: ConnectionStatus }
  | { type: "busy"; text: string };

const CONNECTION_STATUS_LABELS: Record<ConnectionStatus, string> = {
  connecting: "Connecting…",
  connected: "Connected",
  error: "Error",
};

function hostLabelFor(hostId: string, hosts: Host[]): string {
  return hosts.find((h) => h.id === hostId)?.name ?? hostId;
}

/** Resolves the remote host label for a tab, if it's backed by one — shared
 *  by the info-line's "Host" segment and the folder-grouping "Remote" badge.
 *  Prefers a live `ConnectionEntry.hostLabel` (snapshotted at connect time,
 *  survives host deletion) over a fresh `hosts` lookup, matching each
 *  kind's existing connection-store key (`activePaneId` for workspace
 *  panes, `String(tab.id)` for SFTP tabs). Returns `undefined` for local
 *  tabs and kinds with no host concept at all (`ai-diff`, `home`). */
export function remoteHostLabelFor(
  t: Tab,
  hosts: Host[],
  connections: Record<string, ConnectionEntry>,
): string | undefined {
  if (t.kind === "workspace") {
    const wt = t as WorkspaceTab;
    const activeSession = wt.sessions[wt.activePaneId];
    const entry = connections[wt.activePaneId];
    if (entry) return entry.hostLabel;
    if (activeSession?.quickConnect) {
      return `${activeSession.quickConnect.username}@${activeSession.quickConnect.hostAddress}`;
    }
    return undefined;
  }
  if (t.kind === "editor" || t.kind === "preview") {
    return t.remoteHostId ? hostLabelFor(t.remoteHostId, hosts) : undefined;
  }
  if (t.kind === "sftp") {
    const entry = connections[String(t.id)];
    return entry ? entry.hostLabel : hostLabelFor(t.hostId, hosts);
  }
  if (t.kind === "git-graph" || t.kind === "git-diff" || t.kind === "commit-diff") {
    return t.hostId ? hostLabelFor(t.hostId, hosts) : undefined;
  }
  return undefined;
}

/** Compact "connected for" duration, e.g. "just now" / "5m" / "1h 12m" —
 *  granularity matches what's useful in a narrow sidebar row, not precise
 *  uptime tracking. */
function formatUptime(connectedAtMs: number, nowMs: number): string {
  const elapsed = nowMs - connectedAtMs;
  if (elapsed < 60_000) return "just now";
  if (elapsed < 3_600_000) return `${Math.floor(elapsed / 60_000)}m`;
  const hours = Math.floor(elapsed / 3_600_000);
  const mins = Math.floor((elapsed % 3_600_000) / 60_000);
  return mins ? `${hours}h ${mins}m` : `${hours}h`;
}

/** Resolves the user-selected sidebar info types into displayable segments
 *  for one tab. `hosts`/`connections`/`transferJobs`/`isBusy`/`nowMs` are
 *  passed in (rather than read via hooks here) so this stays a plain
 *  function — the calling component owns the store subscriptions and the
 *  uptime ticker. Returns `[]` whenever a selected type doesn't apply to
 *  this tab's kind/state, letting the caller silently omit the segment (or
 *  the whole second line) with no placeholder. */
export function sidebarInfoLineFor(
  t: Tab,
  selected: SidebarTabInfoType[],
  hosts: Host[],
  connections: Record<string, ConnectionEntry>,
  transferJobs: TransferJob[],
  isBusy: boolean,
  nowMs: number,
): SidebarInfoSegment[] {
  if (selected.length === 0) return [];

  const segments: SidebarInfoSegment[] = [];

  const addPath = (text: string | undefined) => {
    if (text) segments.push({ type: "path", text });
  };
  const addHost = (text: string | undefined) => {
    if (text) segments.push({ type: "host", text });
  };
  const addConnection = (entry: ConnectionEntry | undefined) => {
    if (entry)
      segments.push({
        type: "connection",
        text: CONNECTION_STATUS_LABELS[entry.status],
        status: entry.status,
      });
  };
  const addUptime = (entry: ConnectionEntry | undefined) => {
    if (entry?.status === "connected")
      segments.push({ type: "uptime", text: formatUptime(entry.connectedAt, nowMs) });
  };
  const addTransfer = (sessionId: string) => {
    const job = transferJobs.find((j) => j.session_id === sessionId && j.status === "running");
    if (!job) return;
    const pct = job.bytes_total > 0 ? Math.round((job.bytes_transferred / job.bytes_total) * 100) : 0;
    segments.push({ type: "transfer", text: `${job.direction === "download" ? "⬇" : "⬆"} ${pct}%` });
  };
  const addBusy = () => {
    if (isBusy) segments.push({ type: "busy", text: "Running…" });
  };

  for (const type of selected) {
    if (type === "host") {
      addHost(remoteHostLabelFor(t, hosts, connections));
      continue;
    }

    if (t.kind === "workspace") {
      const wt = t as WorkspaceTab;
      const activeSession = wt.sessions[wt.activePaneId];
      const entry = connections[wt.activePaneId];
      if (type === "path") addPath(activeSession?.cwd);
      else if (type === "connection") addConnection(entry);
      else if (type === "uptime") {
        if (activeSession?.kind === "local") {
          const openedAt = getLocalOpenedAt(wt.activePaneId);
          if (openedAt) segments.push({ type: "uptime", text: formatUptime(openedAt, nowMs) });
        } else {
          addUptime(entry);
        }
      } else if (type === "busy") addBusy();
    } else if (t.kind === "editor") {
      if (type === "path") addPath(t.remoteHostId ? (t.remotePath ?? t.path) : t.path);
    } else if (t.kind === "ai-diff") {
      if (type === "path") addPath(t.path);
    } else if (t.kind === "sftp") {
      const entry = connections[String(t.id)];
      if (type === "path") addPath(t.remotePath);
      else if (type === "connection") addConnection(entry);
      else if (type === "uptime") addUptime(entry);
      else if (type === "transfer") addTransfer(String(t.id));
    } else if (t.kind === "git-graph" || t.kind === "git-diff" || t.kind === "commit-diff") {
      if (type === "path") addPath(t.kind === "git-diff" ? t.repoRoot : t.repositoryPath);
    }
    // "preview"/"home" have no path segment to contribute; "host" is
    // already handled above for every kind that supports it.
  }

  return segments;
}

// --- Shared plural kind label (for "Close all …" menu items) ---

export function pluralLabelFor(kind: Tab["kind"]): string {
  switch (kind) {
    case "workspace":
      return "Terminals";
    case "editor":
      return "Editors";
    case "preview":
      return "Previews";
    case "ai-diff":
      return "AI Diffs";
    case "home":
      return "Home Tabs";
    case "sftp":
      return "SFTP Tabs";
    case "git-graph":
      return "Git Graphs";
    case "git-diff":
      return "Git Diffs";
    case "commit-diff":
      return "Commit Diffs";
  }
}

// --- Shared icon component ---

export function TabIconFor({ tab, active, size = 14 }: { tab: Tab; active: boolean; size?: number }) {
  if (tab.kind === "editor") {
    const url = fileIconUrl(tab.title);
    return url ? (
      <img src={url} alt="" decoding="sync" className="shrink-0" style={{ width: size, height: size }} />
    ) : null;
  }
  if (tab.kind === "preview") {
    return <HugeiconsIcon icon={Globe02Icon} size={size} strokeWidth={1.75} className="shrink-0" />;
  }
  if (tab.kind === "ai-diff") {
    return (
      <HugeiconsIcon icon={GitCompareIcon} size={size} strokeWidth={1.75} className="shrink-0 text-warning" />
    );
  }
  if (tab.kind === "home") {
    return <HugeiconsIcon icon={Home03Icon} size={size} strokeWidth={1.75} className="shrink-0" />;
  }
  if (tab.kind === "sftp") {
    return <HugeiconsIcon icon={CloudServerIcon} size={size} strokeWidth={1.75} className="shrink-0" />;
  }
  if (tab.kind === "git-graph") {
    return <HugeiconsIcon icon={GitBranchIcon} size={size} strokeWidth={1.75} className="shrink-0" />;
  }
  if (tab.kind === "git-diff") {
    return (
      <HugeiconsIcon
        icon={GitCompareIcon}
        size={size}
        strokeWidth={1.75}
        className="shrink-0 text-modified"
      />
    );
  }
  if (tab.kind === "commit-diff") {
    return (
      <HugeiconsIcon icon={GitCompareIcon} size={size} strokeWidth={1.75} className="shrink-0 text-info" />
    );
  }
  if (tab.kind === "workspace") {
    const wt = tab as WorkspaceTab;
    const activeSession = wt.sessions[wt.activePaneId];
    const icon = activeSession?.kind === "ssh" ? ComputerTerminal02Icon : TerminalIcon;
    return <HugeiconsIcon icon={icon} size={size} strokeWidth={1.75} className="shrink-0" />;
  }
  return (
    <HugeiconsIcon
      icon={active ? Folder02Icon : Folder01Icon}
      size={size}
      strokeWidth={2}
      className="shrink-0"
    />
  );
}

// --- Shared new-tab dropdown items ---

interface NewTabDropdownItemsProps {
  onNew: () => void;
  onNewPreview: () => void;
  onNewEditor: () => void;
  onNewSsh: (hostId: string, title: string) => void;
  onNewSftp: (hostId: string, title: string) => void;
  onOpenHostManager: () => void;
  onNewGitGraph?: () => void;
}

export function NewTabDropdownItems({
  onNew,
  onNewPreview,
  onNewEditor,
  onNewSsh,
  onNewSftp,
  onOpenHostManager,
  onNewGitGraph,
}: NewTabDropdownItemsProps) {
  const recentHosts = useRecentHosts();
  return (
    <>
      <DropdownMenuItem onSelect={() => onNew()}>
        <HugeiconsIcon icon={TerminalIcon} size={14} strokeWidth={1.75} />
        <span className="flex-1">Terminal</span>
        <span className="text-xs text-muted-foreground">⌘T</span>
      </DropdownMenuItem>
      <DropdownMenuItem onSelect={() => onNewEditor()}>
        <HugeiconsIcon icon={PencilEdit02Icon} size={14} strokeWidth={1.75} />
        <span className="flex-1">Editor</span>
        <span className="text-xs text-muted-foreground">⌘E</span>
      </DropdownMenuItem>
      <DropdownMenuItem onSelect={() => onNewPreview()}>
        <HugeiconsIcon icon={Globe02Icon} size={14} strokeWidth={1.75} />
        <span className="flex-1">Preview</span>
        <span className="text-xs text-muted-foreground">⌘P</span>
      </DropdownMenuItem>
      {onNewGitGraph && (
        <DropdownMenuItem onSelect={onNewGitGraph}>
          <HugeiconsIcon icon={GitBranchIcon} size={14} strokeWidth={1.75} />
          <span className="flex-1">Git Graph</span>
        </DropdownMenuItem>
      )}
      <DropdownMenuSeparator />
      <DropdownMenuSub>
        <DropdownMenuSubTrigger>
          <HugeiconsIcon icon={ComputerTerminal02Icon} size={14} strokeWidth={1.75} />
          <span className="flex-1">SSH</span>
        </DropdownMenuSubTrigger>
        <DropdownMenuSubContent className="min-w-48">
          {recentHosts.length === 0 ? (
            <DropdownMenuItem disabled>
              <span>No hosts yet</span>
            </DropdownMenuItem>
          ) : (
            recentHosts.map((host) => (
              <DropdownMenuItem key={host.id} onSelect={() => onNewSsh(host.id, host.name)}>
                <span className="flex-1 truncate">{host.name}</span>
                <span className="ml-2 text-xs text-muted-foreground truncate max-w-28">
                  {host.host_address}
                </span>
              </DropdownMenuItem>
            ))
          )}
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={onOpenHostManager}>
            <span>All hosts...</span>
          </DropdownMenuItem>
        </DropdownMenuSubContent>
      </DropdownMenuSub>
      <DropdownMenuSub>
        <DropdownMenuSubTrigger>
          <HugeiconsIcon icon={CloudServerIcon} size={14} strokeWidth={1.75} />
          <span className="flex-1">SFTP</span>
        </DropdownMenuSubTrigger>
        <DropdownMenuSubContent className="min-w-48">
          {recentHosts.length === 0 ? (
            <DropdownMenuItem disabled>
              <span>No hosts yet</span>
            </DropdownMenuItem>
          ) : (
            recentHosts.map((host) => (
              <DropdownMenuItem key={host.id} onSelect={() => onNewSftp(host.id, host.name)}>
                <span className="flex-1 truncate">{host.name}</span>
                <span className="ml-2 text-xs text-muted-foreground truncate max-w-28">
                  {host.host_address}
                </span>
              </DropdownMenuItem>
            ))
          )}
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={onOpenHostManager}>
            <span>All hosts...</span>
          </DropdownMenuItem>
        </DropdownMenuSubContent>
      </DropdownMenuSub>
    </>
  );
}
