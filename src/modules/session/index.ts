export { captureAndSave, captureSnapshot } from "./capture";
export { restoreIfEnabled, restoreSnapshot, type TabActions } from "./restore";
export {
  cleanupScrollbacks,
  closeDanglingAltScreen,
  saveAllScrollbacks,
  setScrollbackLive,
} from "./scrollback";
export { clearSnapshot, loadSnapshot, saveSnapshot } from "./store";
export type { RestoreResult, SessionSnapshot, TabSnapshot } from "./types";
export type { SessionLifecycleReturn } from "./useSessionLifecycle";
export { useSessionLifecycle } from "./useSessionLifecycle";
