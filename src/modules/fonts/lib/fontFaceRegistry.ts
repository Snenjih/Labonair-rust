import { invoke } from "@tauri-apps/api/core";
import type { CustomFontInfo } from "../types";

// document.fonts is per-document — this registry (and its Map) is therefore
// implicitly per-window, which is correct: each window's bootstrap path
// registers its own FontFace objects into its own document.
const registry = new Map<string, FontFace>();

/** Loads and registers a custom font's FontFace into this document. Throws
 *  if the file is corrupt/unparseable (caught by callers to trigger a
 *  rollback via `font_delete`) — this is the second, authoritative
 *  validation layer beyond the Rust-side magic-byte sniff. No-op if this
 *  document already has this font registered.
 *
 *  Delivered as a base64 data URL (`font_read_data_url`), not `convertFileSrc`
 *  — the Font Loading API's `FontFace.load()` performs a CORS-checked fetch
 *  of its source, unlike passive `<img>`/`<iframe src>` references, and
 *  asset:// URLs fail that check in WKWebView with a generic "NetworkError".
 *  A data: URL never triggers a network fetch, so it can't fail that way. */
export async function registerCustomFontFace(font: CustomFontInfo): Promise<void> {
  if (registry.has(font.filename)) return;
  const dataUrl = await invoke<string>("font_read_data_url", { filename: font.filename });
  const face = new FontFace(font.label, `url("${dataUrl}")`);
  await face.load();
  document.fonts.add(face);
  registry.set(font.filename, face);
}

export function unregisterCustomFontFace(filename: string): void {
  const face = registry.get(filename);
  if (!face) return;
  document.fonts.delete(face);
  registry.delete(filename);
}
