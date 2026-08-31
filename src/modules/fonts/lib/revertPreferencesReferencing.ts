import { useNotificationStore } from "@/modules/notifications/store/useNotificationStore";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  DEFAULT_PREFERENCES,
  setAppFontFamily,
  setEditorFontFamily,
  setTerminalFontFamily,
} from "@/modules/settings/store";
import { primaryFontFamilyName } from "./fontFamily";

const CHECKS = [
  { key: "appFontFamily", label: "UI font", setter: setAppFontFamily } as const,
  { key: "terminalFontFamily", label: "Terminal font", setter: setTerminalFontFamily } as const,
  { key: "editorFontFamily", label: "Editor font", setter: setEditorFontFamily } as const,
];

/** Called whenever a custom font is deleted (locally or via cross-window
 *  sync). Any of the 3 font preferences that referenced the deleted font by
 *  name is reset to its default, with a notification — otherwise the
 *  terminal/editor/UI would silently keep pointing at a font-family string
 *  with no backing FontFace, falling back to whatever the browser picks. */
export function revertPreferencesReferencing(deletedLabel: string): void {
  const prefs = usePreferencesStore.getState();
  for (const { key, label, setter } of CHECKS) {
    const current = prefs[key];
    if (primaryFontFamilyName(current) !== deletedLabel) continue;
    void setter(DEFAULT_PREFERENCES[key]);
    useNotificationStore.getState().addActionResultNotification({
      type: "warning",
      title: "Font removed",
      message: `"${deletedLabel}" was deleted and is no longer available. ${label} was reset to its default.`,
      source: "Fonts",
    });
  }
}
