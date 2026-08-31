import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import { handleApiError } from "@/lib/errors";

type SystemFontsState = {
  hydrated: boolean;
  loading: boolean;
  families: string[];
  /** Idempotent — safe to call from multiple windows/components. The actual
   *  scan runs at most once per process (Rust side also caches via OnceLock),
   *  regardless of how many times this is invoked. */
  hydrate: () => Promise<void>;
};

// Module-level guard (not a `hydrated` state flag) so concurrent callers
// within the same window before the first hydrate() resolves still only
// trigger one invoke — mirrors useAgentsStore's pattern.
let initialized = false;

export const useSystemFontsStore = create<SystemFontsState>((set) => ({
  hydrated: false,
  loading: false,
  families: [],
  hydrate: async () => {
    if (initialized) return;
    initialized = true;
    set({ loading: true });
    try {
      const families = await invoke<string[]>("fonts_list_system");
      set({ families, hydrated: true, loading: false });
    } catch (e) {
      // Fail open — the picker stays usable with Bundled + Custom groups
      // even if system font enumeration errors out for some reason.
      handleApiError(e, "Failed to scan system fonts", "Fonts");
      set({ hydrated: true, loading: false });
    }
  },
}));
