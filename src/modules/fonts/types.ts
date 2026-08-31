// Field casing mirrors the Rust `CustomFontInfo` struct as-is (no camelCase
// rename on the Rust side), matching the existing `BackgroundInfo` convention.
export type CustomFontInfo = {
  filename: string;
  label: string;
  path: string;
  size_bytes: number;
  imported_at_ms: number;
};

/** "ui" appends a sans-serif fallback, "monospace" appends a monospace one —
 *  used only when building the CSS stack for a freshly-picked font. */
export type FontContext = "ui" | "monospace";

export type FontCandidateGroup = "bundled" | "system" | "custom";
