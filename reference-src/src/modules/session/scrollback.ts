import { invoke } from "@tauri-apps/api/core";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  commitDormantFlush,
  getAllSessionIds,
  peekDormantAnsi,
} from "@/modules/terminal/lib/terminalSessionRegistry";
import type { TerminalPaneHandle } from "@/modules/terminal/TerminalPane";

function maxScrollbackBytes(): number {
  return usePreferencesStore.getState().scrollbackMaxSizeMb * 1024 * 1024;
}

// Visible marker prepended once `truncateScrollback` has to cut content —
// mirrors DormantRing's OVERFLOW_NOTICE (dormantRing.ts) so an on-disk
// scrollback that got trimmed to fit `maxBytes` reads the same way a
// dropped-while-backgrounded ring does, instead of just silently starting
// mid-stream with no explanation.
const SCROLLBACK_OVERFLOW_NOTICE =
  "\r\n\x1b[0m\x1b[2m[labonair: earlier scrollback was truncated to fit the size limit]\x1b[0m\r\n";

/**
 * Truncates `ansi` from the front (oldest content first) once it exceeds
 * `maxBytes`, keeping the most recent output instead of dropping the whole
 * blob outright — same truncate-with-overflow-notice pattern
 * `DormantRing.push()` uses for its in-memory ring (dormantRing.ts:43-73),
 * applied here to the string about to be persisted to disk. The cut point is
 * advanced to the next line boundary so an ANSI escape sequence never gets
 * split mid-sequence. Shared by both scrollback write paths below so the
 * truncation logic isn't duplicated.
 */
function truncateScrollback(ansi: string, maxBytes: number): string {
  if (ansi.length <= maxBytes) return ansi;
  const budget = maxBytes - SCROLLBACK_OVERFLOW_NOTICE.length;
  if (budget <= 0) return SCROLLBACK_OVERFLOW_NOTICE.slice(0, maxBytes);
  const cutStart = ansi.length - budget;
  const lf = ansi.indexOf("\n", cutStart);
  const start = lf >= 0 ? lf + 1 : cutStart;
  return SCROLLBACK_OVERFLOW_NOTICE + ansi.slice(start);
}

const ALT_SCREEN_ENTER = /\x1b\[\?1049h/g;
const ALT_SCREEN_EXIT = /\x1b\[\?1049l/g;

/**
 * Appended to a replayed scrollback blob only if it ends with a dangling
 * (unmatched) alt-screen entry. SerializeAddon.serialize() (see
 * saveAllScrollbacks below) appends `\x1b[?1049h` iff the buffer was in alt
 * mode (vim/htop/a TUI) when saved — never a matching close — so for a
 * direct save, "1049h present" == "dangling". flushDormantScrollback can
 * combine that with raw dormant-ring bytes that *do* contain a real,
 * balanced 1049h/1049l pair (a TUI opened and closed while the tab was
 * backgrounded), so this counts markers rather than doing a presence check,
 * to avoid a redundant restore on an already-clean blob.
 *
 * This must stay conditional, never unconditional: DECRST 1049 is NOT a
 * true no-op in xterm.js when not needed — it always calls restoreCursor()
 * (see InputHandler.ts case 1049), which jumps the cursor to
 * Buffer.savedX/Y (0,0 by default when no matching 1049h ever ran). An
 * unconditional append would corrupt the far more common non-TUI restore
 * case by yanking the cursor to the top after every replay.
 *
 * Must be baked into the returned string itself (not a live post-hoc check
 * against the bound Terminal) so it self-applies whether the bytes are
 * written straight into a live slot or land in the dormant ring first and
 * get replayed later when the tab becomes visible — see SshTerminalPane.tsx.
 */
export function closeDanglingAltScreen(ansi: string): string {
  const enters = ansi.match(ALT_SCREEN_ENTER)?.length ?? 0;
  const exits = ansi.match(ALT_SCREEN_EXIT)?.length ?? 0;
  return enters > exits ? `${ansi}\x1b[?1049l` : ansi;
}

type ScrollbackLive = {
  getAllTerminalRefs: () => Map<string, TerminalPaneHandle>;
};

let _live: ScrollbackLive | null = null;

export function setScrollbackLive(live: ScrollbackLive): void {
  _live = live;
}

export async function saveAllScrollbacks(sessionIds: string[]): Promise<void> {
  const refs = _live?.getAllTerminalRefs();
  if (!refs) return;
  await Promise.all(
    sessionIds.map(async (sessionId) => {
      const handle = refs.get(sessionId);
      if (!handle) return; // terminal not mounted yet
      const scrollbackLimit = usePreferencesStore.getState().sessionScrollbackLines;
      const ansi = handle.serialize(scrollbackLimit > 0 ? scrollbackLimit : undefined);
      if (!ansi || ansi.trim().length === 0) return; // empty buffer
      const maxBytes = maxScrollbackBytes();
      const truncated = truncateScrollback(ansi, maxBytes);
      try {
        await invoke("scrollback_save", { sessionId, ansi: truncated, maxBytes });
      } catch (e) {
        console.warn("[scrollback] save failed:", sessionId, e);
      }
    }),
  );
}

// Marks the boundary between previously-saved content and a freshly-flushed
// dormant-ring chunk. `buffered` is a raw, unparsed byte tail — splicing it
// straight onto an unrelated serialize() snapshot with no marker makes a
// single garbled cut look like corrupted live rendering. This isn't a fix
// for that garbling itself, just makes each boundary legible.
const DORMANT_FLUSH_SEPARATOR = `\r\n\x1b[2m\x1b[90m${"─".repeat(24)} background output ${"─".repeat(24)}\x1b[0m\r\n\r\n`;

/**
 * Appends a dormant session's buffered output (accumulated in its renderer-
 * pool dormant ring while it had no bound slot — see terminalSessionRegistry.ts)
 * onto its on-disk scrollback file. `scrollback_save` overwrites the whole
 * file, so this loads the existing content first — otherwise quitting while
 * a backgrounded tab is dormant would silently drop everything it buffered.
 * `peekDormantAnsi` only returns bytes new since the last call, so repeated
 * ticks append rather than re-duplicate the whole ring each time.
 * No-op if nothing new was buffered (the common case for a quiet background
 * tab, and always the case for a currently-bound session, whose ring is empty).
 *
 * Known perf limitation (deliberately deferred): this is a full read-modify-
 * write of the whole scrollback file on every 30s tick, so cost scales with
 * the existing file's size rather than just the new delta. An append-only
 * write path would need a new/changed Rust command; left as-is for now since
 * that's a riskier change than this pass's correctness fix (the truncation
 * below) warrants.
 */
export async function flushDormantScrollback(sessionId: string): Promise<void> {
  const buffered = peekDormantAnsi(sessionId);
  if (!buffered) return;
  try {
    const maxBytes = maxScrollbackBytes();
    const existing = (await invoke<string | null>("scrollback_load", { sessionId, maxBytes })) ?? "";
    const combined = existing ? existing + DORMANT_FLUSH_SEPARATOR + buffered : buffered;
    if (combined.trim().length === 0) return;
    const truncated = truncateScrollback(combined, maxBytes);
    await invoke("scrollback_save", { sessionId, ansi: truncated, maxBytes });
    // Only mark these bytes as flushed once they're actually durably saved —
    // an early return above (size cap) or a caught failure below leaves them
    // unflushed so the next tick previews and retries them instead of
    // silently dropping them.
    commitDormantFlush(sessionId);
  } catch (e) {
    console.warn("[scrollback] dormant flush failed:", sessionId, e);
  }
}

/** Flushes every registered session's dormant-ring content — used on app quit
 *  so a backgrounded tab's recent output isn't lost. */
export async function flushAllDormantScrollbacks(): Promise<void> {
  await Promise.all(getAllSessionIds().map((id) => flushDormantScrollback(id)));
}

export async function cleanupScrollbacks(knownSessionIds: string[]): Promise<void> {
  try {
    const retentionDays = usePreferencesStore.getState().scrollbackRetentionDays;
    await invoke("scrollback_cleanup", {
      knownSessionIds,
      maxAgeSecs: retentionDays > 0 ? retentionDays * 86400 : null,
    });
  } catch (e) {
    console.warn("[scrollback] cleanup failed:", e);
  }
}
