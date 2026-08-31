import {
  ArrowDown01Icon,
  ComputerIcon,
  Copy01Icon,
  Delete02Icon,
  Edit01Icon,
  Logout01Icon,
  PlayIcon,
  ServerStack01Icon,
  SlidersHorizontalIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { CommandSnippet, SnippetExecMode } from "../types";

function modeLabel(mode: SnippetExecMode): string {
  switch (mode) {
    case "terminal":
      return "Terminal";
    case "silent":
      return "Silent";
    case "inject":
      return "Inject";
  }
}

interface Props {
  snippet: CommandSnippet;
  hostName?: string;
  groupColor?: string | null;
  onRun: (snippet: CommandSnippet, mode?: SnippetExecMode) => void;
  onEdit: (snippet: CommandSnippet) => void;
  onDuplicate: (snippet: CommandSnippet) => void;
  onDelete: (snippet: CommandSnippet) => void;
}

export function SnippetItem({ snippet, hostName, groupColor, onRun, onEdit, onDuplicate, onDelete }: Props) {
  const isSSH = snippet.target === "ssh";
  // groupColor comes from user-defined group data (intentional arbitrary color).
  // Defaults use chart tokens: chart-2 (blue) for SSH, chart-5 (purple) for local.
  const accentColor = groupColor ?? (isSSH ? "var(--chart-2)" : "var(--chart-5)");
  const preview = snippet.description?.trim() || snippet.command.split("\n")[0];

  // Carried by the icon chip's tooltip so the distinction survives even when the
  // badge itself is hidden by the container query at narrow panel widths.
  const hostWarning =
    isSSH && !hostName
      ? snippet.hostId
        ? "This snippet's target host no longer exists"
        : "Prompts for a host each time it runs"
      : undefined;

  async function copyCommand() {
    await navigator.clipboard.writeText(snippet.command);
  }

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div className="@container relative cursor-default rounded-md border border-border bg-card shadow-row transition-colors hover:bg-accent/40">
          <div className="flex flex-col gap-2 px-2.5 py-2.5">
            {/* Title row */}
            <div className="flex items-center gap-2">
              <div
                className="flex size-6 shrink-0 items-center justify-center rounded-md"
                style={{ background: `color-mix(in srgb, ${accentColor} 16%, transparent)` }}
                title={hostWarning}
              >
                <HugeiconsIcon
                  icon={isSSH ? ServerStack01Icon : ComputerIcon}
                  size={13}
                  strokeWidth={1.75}
                  style={{ color: accentColor }}
                />
              </div>
              <span className="min-w-0 flex-1 truncate text-[13px] font-semibold leading-snug tracking-[-0.01em] text-foreground">
                {snippet.name}
              </span>
              {isSSH && hostName && (
                <Badge variant="secondary" className="hidden shrink-0 @[180px]:inline-flex">
                  <span className="max-w-24 truncate">{hostName}</span>
                </Badge>
              )}
              {isSSH && !hostName && (
                <Badge variant="warning" className="hidden shrink-0 @[180px]:inline-flex" title={hostWarning}>
                  {snippet.hostId ? "Host missing" : "Ask at runtime"}
                </Badge>
              )}
            </div>

            {/* Command preview — hidden below 220px to keep the action row reachable */}
            <div className="hidden overflow-hidden rounded border border-border/60 bg-muted/30 px-2 py-1 @[220px]:block">
              <p className="truncate font-mono text-[10px] leading-relaxed text-muted-foreground">
                {preview}
              </p>
            </div>

            {/* Footer: run + actions — always visible, never hover-gated */}
            <div className="flex items-center gap-1.5">
              <div className="flex h-6 shrink-0">
                <Button
                  variant="secondary"
                  size="xs"
                  className="h-6 gap-1 rounded-r-none px-2 text-[10px] font-semibold tracking-wide"
                  title={`Runs: ${modeLabel(snippet.defaultExecMode)}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    onRun(snippet);
                  }}
                >
                  <HugeiconsIcon icon={PlayIcon} size={10} strokeWidth={2.5} />
                  <span className="hidden @[190px]:inline">RUN</span>
                </Button>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <button
                      type="button"
                      title="Choose run mode"
                      onClick={(e) => e.stopPropagation()}
                      className="hidden w-4 items-center justify-center rounded-r-md border-l border-border bg-secondary text-secondary-foreground outline-none transition-colors hover:bg-secondary/80 focus-visible:ring-1 focus-visible:ring-ring/50 @[160px]:flex"
                    >
                      <HugeiconsIcon icon={ArrowDown01Icon} size={8} strokeWidth={2.5} />
                    </button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="start" className="w-44">
                    <DropdownMenuItem onClick={() => onRun(snippet, "terminal")}>
                      <HugeiconsIcon icon={Logout01Icon} size={13} strokeWidth={1.5} className="mr-2" />
                      Run in Terminal
                    </DropdownMenuItem>
                    <DropdownMenuItem onClick={() => onRun(snippet, "silent")}>
                      <HugeiconsIcon
                        icon={SlidersHorizontalIcon}
                        size={13}
                        strokeWidth={1.5}
                        className="mr-2"
                      />
                      Run Silently (log)
                    </DropdownMenuItem>
                    <DropdownMenuItem onClick={() => onRun(snippet, "inject")}>
                      <HugeiconsIcon icon={PlayIcon} size={13} strokeWidth={1.5} className="mr-2" />
                      Inject into Terminal
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>

              <div className="flex-1" />

              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="icon-xs"
                  title="Copy command"
                  className="hidden text-muted-foreground hover:text-foreground @[190px]:flex"
                  onClick={(e) => {
                    e.stopPropagation();
                    void copyCommand();
                  }}
                >
                  <HugeiconsIcon icon={Copy01Icon} size={11} strokeWidth={1.5} />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  title="Edit"
                  className="text-muted-foreground hover:text-foreground"
                  onClick={(e) => {
                    e.stopPropagation();
                    onEdit(snippet);
                  }}
                >
                  <HugeiconsIcon icon={Edit01Icon} size={11} strokeWidth={1.5} />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  title="Delete"
                  className="text-muted-foreground hover:bg-destructive/15 hover:text-destructive"
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete(snippet);
                  }}
                >
                  <HugeiconsIcon icon={Delete02Icon} size={11} strokeWidth={1.5} />
                </Button>
              </div>
            </div>
          </div>
        </div>
      </ContextMenuTrigger>

      <ContextMenuContent className="w-52">
        <ContextMenuItem onClick={() => onRun(snippet, "terminal")}>
          <HugeiconsIcon icon={Logout01Icon} size={13} strokeWidth={1.5} className="mr-2" />
          Run in Terminal
        </ContextMenuItem>
        <ContextMenuItem onClick={() => onRun(snippet, "silent")}>
          <HugeiconsIcon icon={SlidersHorizontalIcon} size={13} strokeWidth={1.5} className="mr-2" />
          Run Silently (log)
        </ContextMenuItem>
        <ContextMenuItem onClick={() => onRun(snippet, "inject")}>
          <HugeiconsIcon icon={PlayIcon} size={13} strokeWidth={1.5} className="mr-2" />
          Inject into Terminal
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem onClick={copyCommand}>
          <HugeiconsIcon icon={Copy01Icon} size={13} strokeWidth={1.5} className="mr-2" />
          Copy Command
        </ContextMenuItem>
        <ContextMenuItem onClick={() => onEdit(snippet)}>
          <HugeiconsIcon icon={Edit01Icon} size={13} strokeWidth={1.5} className="mr-2" />
          Edit
        </ContextMenuItem>
        <ContextMenuItem onClick={() => onDuplicate(snippet)}>
          <HugeiconsIcon icon={Copy01Icon} size={13} strokeWidth={1.5} className="mr-2" />
          Duplicate
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem
          className="text-destructive focus:text-destructive"
          onClick={() => onDelete(snippet)}
        >
          <HugeiconsIcon icon={Delete02Icon} size={13} strokeWidth={1.5} className="mr-2" />
          Delete
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
