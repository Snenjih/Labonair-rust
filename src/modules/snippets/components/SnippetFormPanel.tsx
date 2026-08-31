import {
  ArrowLeft01Icon,
  ComputerIcon,
  Copy01Icon,
  Delete02Icon,
  Logout01Icon,
  MoreHorizontalIcon,
  ServerStack01Icon,
  SlidersHorizontalIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useState } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { useHostsStore } from "@/modules/hosts/store/hostsStore";
import { useCommandSnippetsStore } from "../store/commandSnippetsStore";
import type { CommandSnippet, SnippetExecMode, SnippetTarget } from "../types";

interface Props {
  snippetId: string | null;
  onClose: () => void;
}

interface FormState {
  name: string;
  description: string;
  command: string;
  target: SnippetTarget;
  hostId: string;
  defaultExecMode: SnippetExecMode;
  workingDir: string;
  groupId: string;
}

const EMPTY_FORM: FormState = {
  name: "",
  description: "",
  command: "",
  target: "local",
  hostId: "",
  defaultExecMode: "terminal",
  workingDir: "",
  groupId: "",
};

function snippetToForm(s: CommandSnippet): FormState {
  return {
    name: s.name,
    description: s.description ?? "",
    command: s.command,
    target: s.target as SnippetTarget,
    hostId: s.hostId ?? "",
    defaultExecMode: s.defaultExecMode as SnippetExecMode,
    workingDir: s.workingDir ?? "",
    groupId: s.groupId ?? "",
  };
}

const EXEC_MODES: Array<{
  value: SnippetExecMode;
  icon: typeof Logout01Icon;
  label: string;
  description: string;
}> = [
  {
    value: "terminal",
    icon: Logout01Icon,
    label: "Terminal",
    description: "Opens a new terminal tab and runs the command.",
  },
  {
    value: "silent",
    icon: SlidersHorizontalIcon,
    label: "Silent",
    description: "Runs in background, output visible in log drawer.",
  },
  {
    value: "inject",
    icon: ComputerIcon,
    label: "Inject",
    description: "Pastes command into the active terminal without running.",
  },
];

function FormSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="space-y-3 rounded-lg border border-border bg-card p-4">
      <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">{title}</p>
      {children}
    </section>
  );
}

function FieldRow({ label, children, hint }: { label: string; children: React.ReactNode; hint?: string }) {
  return (
    <div className="space-y-1.5">
      <Label className="text-xs text-muted-foreground">{label}</Label>
      {children}
      {hint && <p className="text-[11px] leading-relaxed text-muted-foreground/70">{hint}</p>}
    </div>
  );
}

export function SnippetFormPanel({ snippetId, onClose }: Props) {
  const snippets = useCommandSnippetsStore((s) => s.snippets);
  const groups = useCommandSnippetsStore((s) => s.groups);
  const createSnippet = useCommandSnippetsStore((s) => s.createSnippet);
  const updateSnippet = useCommandSnippetsStore((s) => s.updateSnippet);
  const deleteSnippet = useCommandSnippetsStore((s) => s.deleteSnippet);
  const hosts = useHostsStore((s) => s.hosts);

  const isNew = snippetId === null;
  const existing = snippetId ? snippets.find((s) => s.id === snippetId) : null;

  const [form, setForm] = useState<FormState>(existing ? snippetToForm(existing) : EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);

  useEffect(() => {
    setForm(existing ? snippetToForm(existing) : EMPTY_FORM);
  }, [snippetId]);

  function set<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((f) => ({ ...f, [key]: value }));
  }

  async function handleSave() {
    if (!form.name.trim() || !form.command.trim()) return;
    setSaving(true);
    try {
      const payload = {
        name: form.name.trim(),
        description: form.description.trim() || null,
        command: form.command,
        target: form.target,
        hostId: form.target === "ssh" && form.hostId ? form.hostId : null,
        defaultExecMode: form.defaultExecMode,
        workingDir: form.workingDir.trim() || null,
        groupId: form.groupId || null,
        tags: null,
        sortOrder: existing?.sortOrder ?? 0,
      };
      if (isNew) {
        await createSnippet(payload);
      } else {
        await updateSnippet(snippetId!, payload);
      }
      onClose();
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!snippetId) return;
    await deleteSnippet(snippetId);
    onClose();
  }

  async function handleDuplicate() {
    if (!existing) return;
    await createSnippet({
      ...existing,
      name: `${existing.name} (copy)`,
      sortOrder: existing.sortOrder + 1,
    });
    onClose();
  }

  const activeExecMode = EXEC_MODES.find((m) => m.value === form.defaultExecMode);

  return (
    <div className="flex h-full flex-col bg-background">
      {/* Header */}
      <div className="flex h-9 shrink-0 items-center gap-2 border-b border-border px-2">
        <button
          onClick={onClose}
          className="flex h-6 w-6 shrink-0 items-center justify-center rounded text-muted-foreground/60 transition-colors hover:bg-muted hover:text-foreground"
        >
          <HugeiconsIcon icon={ArrowLeft01Icon} size={13} strokeWidth={2} />
        </button>
        <span className="min-w-0 flex-1 truncate text-sm font-semibold text-foreground">
          {isNew ? "New Snippet" : "Edit Snippet"}
        </span>
        {!isNew && (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon-xs"
                title="Options"
                className="text-muted-foreground hover:text-foreground"
              >
                <HugeiconsIcon icon={MoreHorizontalIcon} size={14} strokeWidth={2} />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-44">
              <DropdownMenuItem onClick={() => void handleDuplicate()}>
                <HugeiconsIcon icon={Copy01Icon} size={13} strokeWidth={1.5} className="mr-2" />
                Duplicate
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                className="text-destructive focus:text-destructive"
                onClick={() => setDeleteOpen(true)}
              >
                <HugeiconsIcon icon={Delete02Icon} size={13} strokeWidth={1.5} className="mr-2" />
                Delete snippet…
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>

      {/* Scrollable form body */}
      <div className="min-h-0 flex-1 overflow-y-auto themed-scrollbar">
        <div className="space-y-4 px-3 py-3">
          {/* ── General ── */}
          <FormSection title="General">
            <FieldRow label="Name">
              <Input
                value={form.name}
                onChange={(e) => set("name", e.target.value)}
                placeholder="e.g. Deploy to production"
                className="h-8 bg-background text-sm"
              />
            </FieldRow>

            <FieldRow label="Description">
              <Input
                value={form.description}
                onChange={(e) => set("description", e.target.value)}
                placeholder="Optional short description"
                className="h-8 bg-background text-sm"
              />
            </FieldRow>

            <FieldRow label="Group">
              <Select
                value={form.groupId || "none"}
                onValueChange={(v) => set("groupId", v === "none" ? "" : v)}
              >
                <SelectTrigger className="h-8 bg-background text-sm">
                  <SelectValue placeholder="No group" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">No group</SelectItem>
                  {groups.map((g) => (
                    <SelectItem key={g.id} value={g.id}>
                      {g.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </FieldRow>
          </FormSection>

          {/* ── Command ── */}
          <FormSection title="Command">
            <Textarea
              value={form.command}
              onChange={(e) => set("command", e.target.value)}
              placeholder="Enter command or script…"
              className="min-h-[100px] resize-y border-border bg-background font-mono text-xs leading-relaxed"
              spellCheck={false}
            />
          </FormSection>

          {/* ── Execution ── */}
          <FormSection title="Execution">
            {/* Target toggle */}
            <FieldRow label="Target">
              <div className="flex gap-1.5">
                {(["local", "ssh"] as const).map((t) => (
                  <button
                    key={t}
                    type="button"
                    onClick={() => set("target", t)}
                    className={cn(
                      "flex flex-1 items-center justify-center gap-1.5 rounded-md border py-1.5 text-xs font-medium transition-all",
                      form.target === t
                        ? "border-primary bg-primary text-primary-foreground"
                        : "border-border bg-background text-muted-foreground hover:bg-accent",
                    )}
                  >
                    <HugeiconsIcon
                      icon={t === "local" ? ComputerIcon : ServerStack01Icon}
                      size={11}
                      strokeWidth={1.5}
                    />
                    {t === "local" ? "Local" : "SSH"}
                  </button>
                ))}
              </div>
            </FieldRow>

            {form.target === "ssh" && (
              <FieldRow label="Host">
                <Select
                  value={form.hostId || "ask"}
                  onValueChange={(v) => set("hostId", v === "ask" ? "" : v)}
                >
                  <SelectTrigger className="h-8 bg-background text-sm">
                    <SelectValue placeholder="Ask at runtime" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="ask">Ask at runtime</SelectItem>
                    {hosts.map((h) => (
                      <SelectItem key={h.id} value={h.id}>
                        {h.name} ({h.host_address})
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </FieldRow>
            )}

            {/* Exec mode — 3-way button group */}
            <FieldRow label="Default Mode" hint={activeExecMode?.description}>
              <div className="flex gap-1.5">
                {EXEC_MODES.map(({ value, icon, label }) => (
                  <button
                    key={value}
                    type="button"
                    onClick={() => set("defaultExecMode", value)}
                    className={cn(
                      "flex flex-1 flex-col items-center gap-1 rounded-md border px-1 py-1.5 transition-all",
                      form.defaultExecMode === value
                        ? "border-primary bg-primary text-primary-foreground"
                        : "border-border bg-background text-muted-foreground hover:bg-accent",
                    )}
                  >
                    <HugeiconsIcon icon={icon} size={12} strokeWidth={1.5} />
                    <span className="text-[10px] font-medium">{label}</span>
                  </button>
                ))}
              </div>
            </FieldRow>

            {form.target === "local" && (
              <FieldRow label="Working Dir">
                <Input
                  value={form.workingDir}
                  onChange={(e) => set("workingDir", e.target.value)}
                  placeholder="Inherit from terminal"
                  className="h-8 bg-background font-mono text-xs"
                />
              </FieldRow>
            )}
          </FormSection>
        </div>
      </div>

      {/* Sticky footer */}
      <div className="flex shrink-0 items-center justify-end gap-2 border-t border-border px-3 py-2">
        <Button
          variant="ghost"
          size="sm"
          className="text-muted-foreground hover:text-foreground"
          onClick={onClose}
        >
          Cancel
        </Button>
        <Button size="sm" disabled={saving || !form.name.trim() || !form.command.trim()} onClick={handleSave}>
          {saving ? "Saving…" : isNew ? "Create" : "Save changes"}
        </Button>
      </div>

      {/* Delete confirmation */}
      <AlertDialog open={deleteOpen} onOpenChange={setDeleteOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete snippet?</AlertDialogTitle>
            <AlertDialogDescription>"{existing?.name}" will be permanently deleted.</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
