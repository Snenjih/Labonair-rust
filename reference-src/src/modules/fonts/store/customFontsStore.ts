import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { handleApiError } from "@/lib/errors";
import { registerCustomFontFace, unregisterCustomFontFace } from "../lib/fontFaceRegistry";
import { revertPreferencesReferencing } from "../lib/revertPreferencesReferencing";
import type { CustomFontInfo } from "../types";

const CHANGED_EVENT = "labonair://fonts-changed";

type CustomFontsState = {
  hydrated: boolean;
  fonts: CustomFontInfo[];
  /** Eager, not idle-deferred — a saved preference may already reference a
   *  custom font, so it must be registered before the UI can render correctly. */
  hydrate: () => Promise<void>;
  importFont: (sourcePath: string, label: string) => Promise<CustomFontInfo>;
  deleteFont: (filename: string) => Promise<void>;
};

async function fetchList(): Promise<CustomFontInfo[]> {
  return invoke<CustomFontInfo[]>("fonts_list_custom");
}

export const useCustomFontsStore = create<CustomFontsState>((set, get) => ({
  hydrated: false,
  fonts: [],

  hydrate: async () => {
    if (get().hydrated) return;
    try {
      const fonts = await fetchList();
      await Promise.all(
        fonts.map((f) =>
          registerCustomFontFace(f).catch((e) =>
            handleApiError(e, `Failed to load custom font "${f.label}"`, "Fonts"),
          ),
        ),
      );
      set({ fonts, hydrated: true });
    } catch (e) {
      handleApiError(e, "Failed to load custom fonts", "Fonts");
      set({ hydrated: true });
      return;
    }

    void listen(CHANGED_EVENT, async () => {
      const fresh = await fetchList();
      const prev = get().fonts;
      const prevByFilename = new Map(prev.map((f) => [f.filename, f]));
      const freshByFilename = new Map(fresh.map((f) => [f.filename, f]));

      const added = fresh.filter((f) => !prevByFilename.has(f.filename));
      await Promise.all(
        added.map((f) =>
          registerCustomFontFace(f).catch((e) =>
            handleApiError(e, `Failed to load custom font "${f.label}"`, "Fonts"),
          ),
        ),
      );
      for (const f of prev) {
        if (!freshByFilename.has(f.filename)) {
          unregisterCustomFontFace(f.filename);
          revertPreferencesReferencing(f.label);
        }
      }
      set({ fonts: fresh });
    });
  },

  importFont: async (sourcePath, label) => {
    const info = await invoke<CustomFontInfo>("font_import", { sourcePath, label });
    try {
      await registerCustomFontFace(info);
    } catch (e) {
      // Rust already persisted the file+manifest entry — roll it back so an
      // unusable, unregisterable font doesn't linger forever in the list.
      await invoke("font_delete", { filename: info.filename }).catch((cleanupErr) =>
        handleApiError(cleanupErr, "Failed to clean up failed font import", "Fonts"),
      );
      throw e;
    }
    set((s) => ({
      fonts: [...s.fonts, info].sort((a, b) => a.label.toLowerCase().localeCompare(b.label.toLowerCase())),
    }));
    void emit(CHANGED_EVENT);
    return info;
  },

  deleteFont: async (filename) => {
    const target = get().fonts.find((f) => f.filename === filename);
    try {
      await invoke("font_delete", { filename });
    } catch (e) {
      handleApiError(e, "Failed to delete font", "Fonts");
      return;
    }
    unregisterCustomFontFace(filename);
    if (target) revertPreferencesReferencing(target.label);
    set((s) => ({ fonts: s.fonts.filter((f) => f.filename !== filename) }));
    void emit(CHANGED_EVENT);
  },
}));
