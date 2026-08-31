import { useEffect } from "react";
import { usePreferencesStore } from "@/modules/settings/preferences";

/** Applies appFontFamily/appFontSize/appLineHeight to real runtime CSS custom
 *  properties on <html>. `appFontFamily` is stored as a ready-to-use CSS
 *  font-family stack (same convention as terminalFontFamily/editorFontFamily)
 *  and is applied as-is — it is never re-built here.
 *
 *  Must be called from every window (main + settings) — document.documentElement
 *  is per-document, and the Settings window's own chrome should reflect the
 *  chosen UI font while it's being picked. */
export function useTypographyEngine(): void {
  const fontFamily = usePreferencesStore((s) => s.appFontFamily);
  const fontSize = usePreferencesStore((s) => s.appFontSize);
  const lineHeight = usePreferencesStore((s) => s.appLineHeight);

  useEffect(() => {
    document.documentElement.style.setProperty("--app-font-family", fontFamily);
  }, [fontFamily]);

  useEffect(() => {
    document.documentElement.style.setProperty("--app-font-size", `${fontSize}px`);
    document.documentElement.style.fontSize = `${fontSize}px`;
  }, [fontSize]);

  useEffect(() => {
    document.documentElement.style.setProperty("--app-line-height", String(lineHeight));
    document.documentElement.style.lineHeight = String(lineHeight);
  }, [lineHeight]);
}
