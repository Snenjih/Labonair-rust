import type { FontContext } from "../types";

export const BUNDLED_FONTS = ["Inter Variable", "JetBrains Mono"] as const;

const BUNDLED_STACKS: Record<string, string> = {
  "Inter Variable": '"Inter Variable", sans-serif',
  "JetBrains Mono": '"JetBrains Mono", SFMono-Regular, Menlo, monospace',
};

/** CSS-reserved generic font keywords — never valid as a distinct font name. */
export const RESERVED_FONT_NAMES = [
  "serif",
  "sans-serif",
  "monospace",
  "cursive",
  "fantasy",
  "system-ui",
  "ui-serif",
  "ui-sans-serif",
  "ui-monospace",
  "ui-rounded",
  "inherit",
  "initial",
  "unset",
  "revert",
  "revert-layer",
];

/** Escapes a name for use inside a double-quoted CSS string — backslashes
 *  and quotes must be escaped or the produced font-family value (and every
 *  declaration after it, since this feeds a CSS custom property) breaks.
 *  Font names shouldn't contain these in practice, but a custom font's label
 *  is free-text the user typed, so this can't be assumed. */
function escapeCssString(name: string): string {
  return name.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

/** Builds the full CSS font-family value to persist when the user picks `name`
 *  from the picker. Bundled names resolve to their known fallback stack; any
 *  other name (system or custom) gets a generic fallback appended based on
 *  `context`. Existing stored values (built before this system existed, or
 *  the legacy bare `"system-ui"` default) are consumed as-is by callers via
 *  `primaryFontFamilyName` — this function is only used going forward, on
 *  selection, so it never needs to parse/migrate old values. */
export function buildFontFamilyValue(name: string, context: FontContext): string {
  const bundled = BUNDLED_STACKS[name];
  if (bundled) return bundled;
  const needsQuotes = /[^A-Za-z0-9-]/.test(name);
  const quoted = needsQuotes ? `"${escapeCssString(name)}"` : name;
  return context === "monospace" ? `${quoted}, monospace` : `${quoted}, sans-serif`;
}

/** Extracts the primary family name from a stored CSS font-family stack, for
 *  matching against picker candidates. Handles quoted/unquoted first token
 *  and degrades gracefully (returns the input unchanged) for values that
 *  don't parse as expected — e.g. leftover free-text from the old Input. */
export function primaryFontFamilyName(stack: string): string {
  const first = stack.split(",")[0]?.trim() ?? stack;
  return first.replace(/^["']|["']$/g, "");
}

/** Case-insensitive equality check used for collision validation. */
export function namesEqual(a: string, b: string): boolean {
  return a.trim().toLowerCase() === b.trim().toLowerCase();
}
