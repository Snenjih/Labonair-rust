import { ArrowDown01Icon, ArrowRight01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";
import { sortFileStatuses } from "../lib/fileTree";
import { useSourceControlStore } from "../store/sourceControlStore";
import type { FileStatus } from "../types";
import { FileChangeItem } from "./FileChangeItem";
import { FileTreeList } from "./FileTreeList";

interface ConflictsSectionProps {
  files: FileStatus[];
  onRefresh: () => void;
}

/** A dedicated, visually-distinct section for merge-conflicted files,
 *  rendered above `TrackedSection` so the files needing resolution before a
 *  commit stand out instead of blending into the ordinary Tracked list. */
export function ConflictsSection({ files, onRefresh }: ConflictsSectionProps) {
  const [collapsed, setCollapsed] = useState(false);
  const fileListViewMode = useSourceControlStore((s) => s.fileListViewMode);
  const sortByPath = useSourceControlStore((s) => s.sortByPath);

  if (files.length === 0) return null;

  const sortedFiles = sortFileStatuses(files, sortByPath);

  return (
    <div className="mb-0.5">
      <div className="group/hdr flex h-6 items-center gap-1.5 px-3 transition-colors hover:bg-error/10">
        <button
          type="button"
          className="flex shrink-0 items-center outline-none focus-visible:ring-1 focus-visible:ring-ring/50 rounded"
          onClick={() => setCollapsed((c) => !c)}
        >
          <HugeiconsIcon
            icon={collapsed ? ArrowRight01Icon : ArrowDown01Icon}
            size={8}
            strokeWidth={2.5}
            className="text-error/60 transition-colors group-hover/hdr:text-error"
          />
        </button>

        <button
          type="button"
          className="flex flex-1 items-center gap-1.5 text-left outline-none focus-visible:ring-1 focus-visible:ring-ring/50 rounded"
          onClick={() => setCollapsed((c) => !c)}
        >
          <span className="select-none text-[10px] font-semibold uppercase tracking-widest text-error/70 group-hover/hdr:text-error">
            Merge Conflicts
          </span>
          <span className="font-mono text-[9px] tabular-nums text-error/50">{files.length}</span>
        </button>
      </div>

      {!collapsed &&
        (fileListViewMode === "tree" ? (
          <div className="px-1 pb-0.5">
            <FileTreeList files={sortedFiles} section="unstaged" onRefresh={onRefresh} />
          </div>
        ) : (
          <div className="px-1 pb-0.5">
            {sortedFiles.map((file) => (
              <FileChangeItem
                key={`conflict:${file.path}`}
                file={file}
                section="unstaged"
                onRefresh={onRefresh}
              />
            ))}
          </div>
        ))}
    </div>
  );
}
