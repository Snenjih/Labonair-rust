import { invoke } from "@tauri-apps/api/core";
import { homeDir } from "@tauri-apps/api/path";
import { useEffect, useRef, useState } from "react";
import { handleApiError } from "@/lib/errors";
import { runStoreMigration } from "@/lib/storeMigration";
import { useLayoutEngine } from "@/lib/useLayoutEngine";
import { useTerminalCursorBlinkInterval } from "@/lib/useTerminalCursorBlinkInterval";
import { useThemeEngine } from "@/lib/useThemeEngine";
import { useTypographyEngine } from "@/lib/useTypographyEngine";
import { getAllKeys, hasPersistedModelSelection, type ProviderKeys, useChatStore } from "@/modules/ai";
import { useAgentsStore } from "@/modules/ai/store/agentsStore";
import { useDirectivesStore } from "@/modules/ai/store/directivesStore";
import { useProvidersStore } from "@/modules/ai/store/providersStore";
import { usePathBookmarksStore } from "@/modules/bookmarks/store/pathBookmarksStore";
import { useCustomFontsStore } from "@/modules/fonts";
import { usePreferencesStore } from "@/modules/settings/preferences";
import { onKeysChanged } from "@/modules/settings/store";
import { bootstrapSftpConnectionListener } from "@/modules/sftp/store/sftpStore";
import {
  bootstrapTransferListeners,
  bootstrapTransferSettingsSync,
} from "@/modules/sftp/store/transferStore";
import { buildMenuSyncPayload, useKeybindsStore } from "@/modules/shortcuts";
import { useCommandSnippetsStore } from "@/modules/snippets";

export interface AppBootstrapReturn {
  keysLoaded: boolean;
  apiKeys: ProviderKeys;
  home: string | null;
}

export function useAppBootstrap(): AppBootstrapReturn {
  // Stable store actions — fetched once, never cause re-renders
  const { setApiKeys, setSelectedModelId, hydrateSessions, openPanel } = useChatStore.getState();

  // Reactive selectors
  const apiKeys = useChatStore((s) => s.apiKeys);
  const prefDefaultModel = usePreferencesStore((s) => s.defaultModelId);
  const prefsHydrated = usePreferencesStore((s) => s.hydrated);
  const terminalComposerEnabled = usePreferencesStore((s) => s.terminalComposerEnabled);

  const initPrefs = usePreferencesStore((s) => s.init);
  const initKeybinds = useKeybindsStore((s) => s.init);
  const keybindOverrides = useKeybindsStore((s) => s.overrides);
  const keybindsHydrated = useKeybindsStore((s) => s.hydrated);

  const [keysLoaded, setKeysLoaded] = useState(false);
  const [home, setHome] = useState<string | null>(null);

  // Home directory
  useEffect(() => {
    homeDir()
      .then(setHome)
      .catch(() => setHome(null));
  }, []);

  // API keys loading + live listener
  useEffect(() => {
    let alive = true;
    const reload = () => {
      void getAllKeys().then((keys) => {
        if (!alive) return;
        setApiKeys(keys);
        setKeysLoaded(true);
      });
    };
    reload();
    const unlistenP = onKeysChanged(reload);
    return () => {
      alive = false;
      void unlistenP.then((fn) => fn());
    };
  }, [setApiKeys]);

  // Preferences init
  useEffect(() => {
    void initPrefs();
  }, [initPrefs]);

  // Keybinds init
  useEffect(() => {
    void initKeybinds();
  }, [initKeybinds]);

  // Sync native OS menu accelerators whenever a keybind is overridden/reset,
  // so rebinding in Settings also updates the native menu path instead of
  // only the in-app listener. Waits for `hydrated` so first paint doesn't
  // briefly push an all-defaults payload before real overrides finish
  // loading from disk.
  useEffect(() => {
    if (!keybindsHydrated) return;
    void invoke("menu_sync_accelerators", { updates: buildMenuSyncPayload(keybindOverrides) }).catch(
      (err) => {
        console.error("[menu-sync] failed to sync native menu accelerators", err);
      },
    );
  }, [keybindsHydrated, keybindOverrides]);

  // Theme engine (owns its own effects internally)
  useThemeEngine();

  // Layout engine — applies --radius to <html>
  useLayoutEngine();

  // Typography engine — applies --app-font-family/-size/-line-height to <html>
  useTypographyEngine();

  // Terminal cursor blink (owns its own effects internally)
  useTerminalCursorBlinkInterval();

  // Sync default model from preferences once hydrated. This used to fire
  // unconditionally on every startup, clobbering whatever model the user had
  // actively picked via the ModelPicker (persisted separately) with the
  // "Default model" Settings preference — so the active model silently reset
  // on every relaunch. Now it only seeds `selectedModelId` from the
  // preference when there's no prior explicit pick (fresh install); live
  // edits to the Settings preference while the app is running still sync.
  const defaultModelSeeded = useRef(false);
  useEffect(() => {
    if (!prefsHydrated) return;
    if (!defaultModelSeeded.current) {
      defaultModelSeeded.current = true;
      if (hasPersistedModelSelection()) return;
    }
    setSelectedModelId(prefDefaultModel);
  }, [prefsHydrated, prefDefaultModel, setSelectedModelId]);

  // Seed the docked composer bar open whenever the Shell/Command composer is
  // enabled — it works without any AI provider, so it needs to be visible
  // out of the box. This used to be a permanent visibility bypass in
  // WorkspaceArea (bar could never be closed while the setting was on); now
  // it's just panelOpen's default, so the AI panel bar-item toggle can still
  // close it afterwards like any other closeable surface.
  useEffect(() => {
    if (!prefsHydrated || !terminalComposerEnabled) return;
    openPanel();
  }, [prefsHydrated, terminalComposerEnabled, openPanel]);

  // Run store migration (nexum → labonair) before hydrating sessions
  useEffect(() => {
    void runStoreMigration();
  }, []);

  // Hydrate sessions immediately
  useEffect(() => {
    void hydrateSessions();
  }, [hydrateSessions]);

  // Custom fonts: eager, not idle-deferred — a saved font preference may
  // already reference one, so it must be registered (FontFace) before
  // terminal/editor/UI text can render it correctly. Cheap manifest read,
  // not a filesystem scan, so no startup-latency concern.
  useEffect(() => {
    void useCustomFontsStore.getState().hydrate();
  }, []);

  // Providers store: init once, then reload whenever the settings window changes providers
  useEffect(() => {
    const store = useProvidersStore.getState();
    void store.init();
    let unlisten: (() => void) | null = null;
    void store
      .onProvidersChanged(() => {
        void useProvidersStore.getState().reload();
      })
      .then((fn) => {
        unlisten = fn;
      });
    return () => {
      unlisten?.();
    };
  }, []);

  // Defer non-critical hydrations until the browser is idle
  useEffect(() => {
    const cb = () => {
      void useAgentsStore.getState().hydrate();
      void useDirectivesStore.getState().hydrate();
      void useCommandSnippetsStore.getState().hydrate();
      void usePathBookmarksStore.getState().hydrate();
    };
    if (typeof requestIdleCallback !== "undefined") {
      const id = requestIdleCallback(cb, { timeout: 2000 });
      return () => cancelIdleCallback(id);
    }
    const id = setTimeout(cb, 1000);
    return () => clearTimeout(id);
  }, []);

  // Bootstrap SFTP transfer listeners
  useEffect(() => {
    void bootstrapTransferListeners();
  }, []);

  // Push worker-wide transfer settings (concurrency, chunk size, default
  // conflict policy) to the Rust worker once at startup and on every change.
  useEffect(() => {
    bootstrapTransferSettingsSync();
  }, []);

  // Bootstrap SFTP connection-lost listener (dead sessions → reconnect banner)
  useEffect(() => {
    void bootstrapSftpConnectionListener();
  }, []);

  // Global unhandled error handlers
  useEffect(() => {
    function onUnhandledRejection(e: PromiseRejectionEvent) {
      e.preventDefault();
      handleApiError(e.reason, "Unhandled Error", "System");
    }
    function onError(e: ErrorEvent) {
      handleApiError(e.error ?? e.message, "Runtime Error", "System");
    }
    window.addEventListener("unhandledrejection", onUnhandledRejection);
    window.addEventListener("error", onError);
    return () => {
      window.removeEventListener("unhandledrejection", onUnhandledRejection);
      window.removeEventListener("error", onError);
    };
  }, []);

  return { keysLoaded, apiKeys, home };
}
