import { useEffect } from "react";
import { useSystemFontsStore } from "../store/systemFontsStore";

/** Triggers the (potentially expensive, hundreds-of-files) system font scan
 *  at the lowest possible priority: gated on the same condition that makes
 *  the main window visible (`prefsHydrated && sessionRestored`), then a
 *  short settle delay so scheduling doesn't compete with the window
 *  show/compositing frame, then a generous `requestIdleCallback` timeout
 *  (longer than the other idle-deferred stores in useAppBootstrap.ts) so it
 *  fires on genuine idle far more often than on the timeout fallback. */
export function useDeferredSystemFontHydration(prefsHydrated: boolean, sessionRestored: boolean): void {
  useEffect(() => {
    if (!prefsHydrated || !sessionRestored) return;

    let idleId: number | undefined;
    let fallbackId: ReturnType<typeof setTimeout> | undefined;

    const settleId = setTimeout(() => {
      const cb = () => void useSystemFontsStore.getState().hydrate();
      if (typeof requestIdleCallback !== "undefined") {
        idleId = requestIdleCallback(cb, { timeout: 4000 });
      } else {
        fallbackId = setTimeout(cb, 2500);
      }
    }, 300);

    return () => {
      clearTimeout(settleId);
      if (idleId !== undefined) cancelIdleCallback(idleId);
      if (fallbackId !== undefined) clearTimeout(fallbackId);
    };
  }, [prefsHydrated, sessionRestored]);
}
