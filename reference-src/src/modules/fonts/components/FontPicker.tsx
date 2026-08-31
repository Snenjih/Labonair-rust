import { Add01Icon, ArrowDown01Icon, Delete01Icon, FolderOpenIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { handleApiError } from "@/lib/errors";
import { cn } from "@/lib/utils";
import { isLabonairError } from "@/types";
import {
  BUNDLED_FONTS,
  buildFontFamilyValue,
  namesEqual,
  primaryFontFamilyName,
  RESERVED_FONT_NAMES,
} from "../lib/fontFamily";
import { useCustomFontsStore } from "../store/customFontsStore";
import { useSystemFontsStore } from "../store/systemFontsStore";
import type { CustomFontInfo, FontContext } from "../types";

type FontPickerProps = {
  /** Full CSS font-family stack, as stored in the preference. */
  value: string;
  onChange: (value: string) => void;
  context: FontContext;
  className?: string;
};

export function FontPicker({ value, onChange, context, className }: FontPickerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [mode, setMode] = useState<"list" | "add">("list");

  const families = useSystemFontsStore((s) => s.families);
  const systemLoading = useSystemFontsStore((s) => s.loading);
  const systemHydrate = useSystemFontsStore((s) => s.hydrate);
  const customFonts = useCustomFontsStore((s) => s.fonts);
  const importFont = useCustomFontsStore((s) => s.importFont);
  const deleteFont = useCustomFontsStore((s) => s.deleteFont);

  const currentName = primaryFontFamilyName(value);

  function handleOpenChange(next: boolean) {
    setIsOpen(next);
    if (next) {
      // A user opening the picker is a legitimate reason to no longer defer
      // the system-font scan — no-op if it already ran.
      void systemHydrate();
      setMode("list");
    }
  }

  function pick(name: string) {
    onChange(buildFontFamilyValue(name, context));
    setIsOpen(false);
  }

  return (
    <Popover open={isOpen} onOpenChange={handleOpenChange}>
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="outline"
          size="sm"
          className={cn("h-7 justify-between gap-2 px-2.5 text-[11.5px] font-normal", className)}
        >
          <span className="truncate" style={{ fontFamily: value }}>
            {currentName}
          </span>
          <HugeiconsIcon
            icon={ArrowDown01Icon}
            strokeWidth={2}
            className="size-3.5 shrink-0 text-muted-foreground"
          />
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-72 gap-0 overflow-hidden p-0">
        {mode === "list" ? (
          <FontListView
            currentName={currentName}
            families={families}
            systemLoading={systemLoading}
            customFonts={customFonts}
            onPick={pick}
            onDelete={(filename) => void deleteFont(filename)}
            onAddClick={() => setMode("add")}
          />
        ) : (
          <AddCustomFontForm
            existingNames={[...BUNDLED_FONTS, ...families, ...customFonts.map((f) => f.label)]}
            onImport={importFont}
            onDone={(name) => pick(name)}
            onCancel={() => setMode("list")}
          />
        )}
      </PopoverContent>
    </Popover>
  );
}

type FontListViewProps = {
  currentName: string;
  families: string[];
  systemLoading: boolean;
  customFonts: CustomFontInfo[];
  onPick: (name: string) => void;
  onDelete: (filename: string) => void;
  onAddClick: () => void;
};

function FontListView({
  currentName,
  families,
  systemLoading,
  customFonts,
  onPick,
  onDelete,
  onAddClick,
}: FontListViewProps) {
  return (
    <div className="flex flex-col">
      <Command className="rounded-none bg-transparent p-0">
        <CommandInput placeholder="Search fonts…" />
        <CommandList className="max-h-64">
          <CommandEmpty>No fonts found.</CommandEmpty>
          {customFonts.length > 0 && (
            <CommandGroup heading="Custom">
              {customFonts.map((f) => (
                <FontRow
                  key={f.filename}
                  name={f.label}
                  selected={f.label === currentName}
                  onSelect={() => onPick(f.label)}
                  onDelete={() => onDelete(f.filename)}
                />
              ))}
            </CommandGroup>
          )}
          <CommandGroup heading="Bundled">
            {BUNDLED_FONTS.map((name) => (
              <FontRow key={name} name={name} selected={name === currentName} onSelect={() => onPick(name)} />
            ))}
          </CommandGroup>
          <CommandGroup heading="System">
            {systemLoading && families.length === 0 && (
              <div className="px-3 py-2 text-[11px] text-muted-foreground">Scanning system fonts…</div>
            )}
            {families.map((name) => (
              <FontRow key={name} name={name} selected={name === currentName} onSelect={() => onPick(name)} />
            ))}
          </CommandGroup>
        </CommandList>
      </Command>
      {/* Fixed footer, deliberately outside CommandList so search/scroll
          never hides or filters it away. */}
      <button
        type="button"
        onClick={onAddClick}
        className="flex shrink-0 items-center gap-2 border-t border-border/60 px-3 py-2 text-[11.5px] font-medium text-muted-foreground transition-colors hover:bg-accent/50 hover:text-foreground"
      >
        <HugeiconsIcon icon={Add01Icon} size={13} strokeWidth={1.75} />
        Add custom font…
      </button>
    </div>
  );
}

function FontRow({
  name,
  selected,
  onSelect,
  onDelete,
}: {
  name: string;
  selected: boolean;
  onSelect: () => void;
  onDelete?: () => void;
}) {
  return (
    <CommandItem
      value={name}
      data-checked={selected || undefined}
      onSelect={onSelect}
      className="group/font-row justify-between"
    >
      <span className="truncate" style={{ fontFamily: name }}>
        {name}
      </span>
      {onDelete && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          title={`Delete "${name}"`}
          className="ml-auto shrink-0 rounded p-0.5 text-muted-foreground opacity-0 transition-opacity hover:text-destructive group-hover/font-row:opacity-100"
        >
          <HugeiconsIcon icon={Delete01Icon} size={12} strokeWidth={1.75} />
        </button>
      )}
    </CommandItem>
  );
}

type AddCustomFontFormProps = {
  existingNames: string[];
  onImport: (sourcePath: string, label: string) => Promise<CustomFontInfo>;
  onDone: (name: string) => void;
  onCancel: () => void;
};

function AddCustomFontForm({ existingNames, onImport, onDone, onCancel }: AddCustomFontFormProps) {
  const [sourcePath, setSourcePath] = useState<string | null>(null);
  const [label, setLabel] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function handlePickFile() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Font", extensions: ["ttf", "otf", "woff", "woff2"] }],
      });
      if (!selected || typeof selected !== "string") return;
      setSourcePath(selected);
      const base = selected.split(/[/\\]/).pop() ?? selected;
      setLabel(base.replace(/\.[^.]+$/, ""));
      setError(null);
    } catch (e) {
      handleApiError(e, "Failed to open file picker", "Fonts");
    }
  }

  function validate(name: string): string | null {
    const trimmed = name.trim();
    if (!trimmed) return "Enter a name for this font.";
    if (trimmed.length > 80) return "Name must be 80 characters or fewer.";
    if (/["\\]/.test(trimmed)) {
      return "Font name can't contain quote or backslash characters.";
    }
    if (RESERVED_FONT_NAMES.some((r) => namesEqual(r, trimmed))) {
      return `"${trimmed}" is a reserved keyword — choose another name.`;
    }
    if (existingNames.some((n) => namesEqual(n, trimmed))) {
      return `A font named "${trimmed}" already exists — choose a different name.`;
    }
    return null;
  }

  async function handleSave() {
    if (!sourcePath) return;
    const validationError = validate(label);
    if (validationError) {
      setError(validationError);
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const info = await onImport(sourcePath, label.trim());
      onDone(info.label);
    } catch (e) {
      setError(isLabonairError(e) ? e.message : String(e));
      handleApiError(e, "Font import failed", "Fonts");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="flex flex-col gap-3 p-3">
      <span className="text-[11.5px] font-medium">Add custom font</span>
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="h-7 justify-start gap-2 text-[11.5px] font-normal"
        onClick={() => void handlePickFile()}
      >
        <HugeiconsIcon
          icon={FolderOpenIcon}
          size={13}
          strokeWidth={1.75}
          className="shrink-0 text-muted-foreground"
        />
        <span className="truncate">
          {sourcePath ? (sourcePath.split(/[/\\]/).pop() ?? sourcePath) : "Choose file…"}
        </span>
      </Button>
      <div className="flex flex-col gap-1">
        <label className="text-[10.5px] text-muted-foreground" htmlFor="font-picker-label">
          Name
        </label>
        <Input
          id="font-picker-label"
          value={label}
          onChange={(e) => {
            setLabel(e.target.value);
            setError(null);
          }}
          placeholder="My Font"
          className="h-7 text-[11.5px]"
          disabled={!sourcePath}
        />
      </div>
      {error && <p className="text-[10.5px] text-destructive">{error}</p>}
      <div className="flex justify-end gap-2 pt-1">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 text-[11.5px]"
          onClick={onCancel}
          disabled={saving}
        >
          Cancel
        </Button>
        <Button
          type="button"
          size="sm"
          className="h-7 text-[11.5px]"
          onClick={() => void handleSave()}
          disabled={!sourcePath || saving}
        >
          {saving ? "Importing…" : "Save"}
        </Button>
      </div>
    </div>
  );
}
