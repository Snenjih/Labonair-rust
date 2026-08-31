import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { isLabonairError } from "@/types";

type HealthState = {
  dead: boolean;
  error: string | null;
  reconnecting: boolean;
  authRequired: boolean;
};

const IDLE: HealthState = { dead: false, error: null, reconnecting: false, authRequired: false };

export interface SftpTabSessionHealth extends HealthState {
  reconnect: () => void;
}

/**
 * Lightweight, read-only health signal for a sidebar tree anchored to an
 * already-open SFTP tab's session (`source: "sftp-tab"`, or a remote
 * editor/preview tab pinned to one) — unlike `useLazyExplorerSession`, this
 * hook doesn't own the session's connect/disconnect lifecycle (the owning
 * tab does that), it only listens for the same `ssh_connection_lost`/
 * `session_established` events so a dead connection surfaces a reconnect
 * affordance here too, instead of leaving the sidebar stuck on an
 * uninteractive inline tree error row with no session-level indication at all.
 */
export function useSftpTabSessionHealth(
  sessionId: string | null,
  hostId: string | null,
): SftpTabSessionHealth {
  const [state, setState] = useState<HealthState>(IDLE);

  useEffect(() => {
    setState(IDLE);
    if (!sessionId) return;

    const unlistenLost = listen<{ session_id: string; reason: string }>("ssh_connection_lost", (e) => {
      if (e.payload.session_id === sessionId) {
        setState({ dead: true, error: e.payload.reason, reconnecting: false, authRequired: false });
      }
    });
    const unlistenEstablished = listen<{ session_id: string }>("session_established", (e) => {
      if (e.payload.session_id === sessionId) {
        setState(IDLE);
      }
    });
    return () => {
      void unlistenLost.then((fn) => fn());
      void unlistenEstablished.then((fn) => fn());
    };
  }, [sessionId]);

  const reconnect = () => {
    if (!sessionId || !hostId) return;
    setState((s) => ({ ...s, reconnecting: true }));
    void invoke("sftp_disconnect", { sessionId })
      .catch(() => {})
      .then(() => invoke("sftp_connect", { sessionId, hostId }))
      .then(() => setState(IDLE))
      .catch((e) => {
        const authRequired = isLabonairError(e) && e.code === "AuthFailed";
        setState({
          dead: true,
          error: isLabonairError(e) ? e.message : String(e),
          reconnecting: false,
          authRequired,
        });
      });
  };

  return { ...state, reconnect };
}
