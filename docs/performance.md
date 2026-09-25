# Performance & Cross-Platform Baseline

**Status:** Supporting baseline; historical measurements may reference the pre-reset implementation.

Reference doc for the "performance is the motive of the port" goal. Captures
the measurement method, the target envelope, the manual regression checklist,
and an inventory of the hot-path guards already in the code so a future change
can be judged against them.

> Scope note: per the task warning, this pass is **measure-first**. The
> GPUI-native architecture (no WebView, no IPC, no JSON round-trips) already
> removes the structural overhead that made the Tauri/React build slow. This
> document is the yard-stick for the native hot-path guards and does not add
> speculative micro-optimisation.

## 0. Hot-path changes implemented in the native port

The current performance pass removed the following foreground work without
changing the user-facing contracts:

- Terminal PTY notifications use a bounded queue and coalesced wakeups. Views
  wait for session notifications instead of polling every 16 ms. Agent output
  is copied only while an agent subscriber is present.
- Terminal search marks its scrollback index dirty while output is arriving.
  Navigation refreshes immediately; render-time highlight refresh is rate
  limited to avoid rescanning scrollback for every PTY chunk.
- Settings UI writes are committed to the in-memory store immediately and
  serialized by one FIFO writer thread. User/project watcher parsing also runs
  on the background executor; only applying an already-parsed result reaches
  the GPUI thread.
- Editor display layouts and document symbols are cached by buffer revision and
  relevant configuration. Repaints copy only the visible display rows.
- Periodic session snapshot JSON, scrollback compression, scrollback file I/O,
  and window-geometry writes run off the GPUI thread. The explicit quit path
  remains synchronous so the final session state is not lost during shutdown.

These are source-level guarantees. No release-build frame-time or RSS number
is claimed until the graphical measurement procedure below has been run on a
macOS host.

## 1. Baseline measurement method

All numbers are taken on the primary target (macOS, Apple Silicon, release
build: `cargo run --release -p labonair`). Record them in the table when a machine is
available; the method is what matters for regression comparison.

| Metric | How to measure |
|---|---|
| Cold start → window visible | wall-clock from process spawn to first paint (`tracing` `info!("Labonair-rust starting")` → first `render`). |
| Start → interactive | first keypress in a terminal echoes. |
| Idle RSS | `footprint` / Activity Monitor, 1 Home tab, 30 s after launch. |
| RSS, 6 terminals + 3 editor tabs | same, after opening that many tabs and running `yes | head -c 5M` in one. |
| Terminal throughput | `time seq 1 200000` inside a pane; watch for frame hitching. |
| Large-list scroll | Explorer on a 5k-entry dir, SFTP on a large remote dir, Git-Graph on a 5k-commit repo — flick-scroll, look for dropped frames. |
| Git status cadence | `tracing` at `debug` — confirm one `git_get_workspace_state` per `POLL_INTERVAL`, not a pile-up. |
| AI streaming | stream a long completion, confirm incremental token append (no full re-layout per chunk). |

### Recorded runs

| Date | Machine | Cold start | Interactive | Idle RSS | Heavy RSS | Notes |
|---|---|---|---|---|---|---|
| _pending_ | | | | | | fill on a release build |

The Tauri/React reference for comparison: WebView process + renderer + Node
sidecar, multi-hundred-ms cold start, ~250–400 MB idle with one webview.
Any Rust number materially worse than that is a regression to investigate.

## 2. Target envelope (macOS, release)

- Cold start to visible: **< 400 ms**, to interactive: **< 700 ms**.
- Idle RSS with one tab: **< 150 MB**.
- Terminal output: no visible hitching at 200k lines; scrollback capped
  (`terminalScrollback` pref, default from `preferences.rs`).
- 5k-row lists scroll at display refresh rate (Git-Graph and SFTP row rendering
  are `uniform_list` virtualised; directory enumeration remains a separate
  measurement).
- Git status: exactly one poll per interval (2 s local, ×N remote), skipped
  when there is no repo root.

## 3. Manual regression checklist (run before a release)

- [ ] Cold start feels instant vs. the reference (side-by-side if possible).
- [ ] Type into a fresh local terminal immediately after launch — no lag.
- [ ] `seq 1 200000` in a pane — scrollback stays smooth, memory settles
      after it finishes (buffer is bounded, not retained unbounded).
- [ ] Open/close ~20 terminal + editor tabs in a loop — RSS returns close to
      baseline (sessions/among `panes` map are dropped on close).
- [ ] Explorer + SFTP on a large directory — scroll is smooth and the
      virtualised row surface stays bounded; record directory-load latency
      separately from first paint.
- [ ] Git-Graph on a large repo — only visible rows render; scroll is smooth.
- [ ] Git panel: with `RUST_LOG=labonair=debug`, confirm the poll cadence and
      that switching away from a repo stops useful work.
- [ ] AI chat: stream a long answer — text appends incrementally, the window
      stays responsive.
- [ ] Retina: 1× and 2× displays both render crisp text; drag the window
      between displays with different scale factors.
- [ ] `prefers-reduced-motion` / the "Reduce motion" setting collapses the
      tab-entrance animation (mirrors the reference `0.01ms` clamp).

## 4. Hot-path guard inventory (already in the code)

| Path | Guard | Location |
|---|---|---|
| Startup | backend workers `spawn_workers()` + event logger are `tokio::spawn`; the window opens without waiting on them. SQLite open is the only sync step and is cheap. | `crates/app/src/main.rs` |
| TreeSitter grammars | behind the `build-grammars` feature / loaded lazily, not at boot. | editor crate |
| Fonts | bundled assets registered once at `init_fonts`. | `crates/theme/src/fonts.rs` |
| Terminal render | The PTY reader feeds the emulator on its own thread; the event-driven view drains bounded/coalesced notifications and repaints on output instead of maintaining a 16 ms timer. | `crates/workspace/src/views/terminal.rs`, `crates/terminal/` |
| Terminal agent tap | PTY bytes are copied into the broadcast tap only when at least one agent subscriber exists. | `crates/terminal/src/session.rs` |
| Terminal search | Full scrollback matching is deferred and render-time refresh is rate limited while output is active. | `crates/terminal/src/engine.rs` |
| Settings persistence | Typed changes update memory immediately; surgical JSONC writes are serialized on one writer thread and flushed at shutdown. | `crates/settings/src/store.rs`, `crates/settings-ui/src/view.rs` |
| Settings watchers | File reads, JSONC parsing, schema validation, and project filtering run in the background; GPUI only applies the loaded layer. | `crates/settings/src/watch.rs`, `crates/settings/src/store.rs` |
| Editor layout/symbols | Display coordinates and document symbols are reused for a buffer revision; paints clone only visible rows. | `crates/editor/src/display.rs`, `crates/workspace/src/views/editor.rs` |
| Explorer | `generation` counter discards stale async dir reads; bounded rendering and lazy expansion. | `crates/panel-explorer/src/panel_explorer.rs` |
| SFTP list | `uniform_list` bounds row-element construction and remote listing is asynchronous; directory enumeration itself is measured separately. | `crates/workspace/src/views/sftp.rs` |
| Git-Graph | row list virtualised with `uniform_list` — only visible commit rows build elements. | `crates/panel-git-graph/src/panel_git_graph.rs` |
| Git status poll | refresh guards prevent overlap and stale results. | `crates/panel-scm/src/panel_scm.rs` |
| Session sync | Workspace metadata is batched and shell metadata is read under one session access; periodic session serialization and scrollback persistence are background work. | `crates/workspace/src/workspace.rs` |
| AI streaming | frontend AI is currently paused; backend streaming remains available for the future rebuild. | `crates/ai/` |

### Follow-ups deliberately deferred (need a profiler + a real workload)

- Streaming/paging directory enumeration for Explorer/SFTP. The SFTP row
  surface is virtualized, but the current local and remote loaders still
  collect and sort a directory before the first list is shown.
- Pausing the Git status poll while the panel is off-screen or the window is
  unfocused — would need a visibility signal from `AppShell`. Low payoff at a
  2 s interval with the existing guards; revisit if profiling shows it.
- Glyph-run caching in the terminal renderer beyond what GPUI's text system
  already caches.

## 5. Cross-platform notes

**macOS (primary)**

- Native title bar: `TitlebarOptions { appears_transparent: false }` in
  `main.rs` — standard traffic-light chrome, no custom drag region.
- Menus: native `cx.set_menus` (App menu bar + Dock menu) from
  `crates/shell/src/menu.rs` — no in-window menu rendering to pay for.
- DPI: GPUI's Metal renderer is scale-factor aware; all sizes are logical
  `px(..)` so Retina is automatic. Terminal cell metrics are derived from
  `text_system().ch_advance` at the current scale.
- Window bounds are persisted/restored (`window_state`).

**Linux (later)**

- Keep it buildable: no macOS-only APIs leak outside `main.rs`'s window
  setup and `menu.rs`. GPUI selects the Vulkan/Blade renderer on Linux; the
  view layer is renderer-agnostic (logical `px`, theme tokens, no platform
  branches in feature view code).
- Open items for the Linux pass: file-dialog / open-in-browser shims,
  keychain backend (`keyring` already abstracts this), font fallback list.

## 6. Visual-parity items closed here (D1–D6 from T15-001)

- **D1** — canonical interaction fills: `ThemeStore::hover_fill()` = `accent`
  (neutral). Selection is now brand-tinted: `selected_fill()` = `primary` at
  0.16 alpha and `selected_accent()` = solid `primary` for the 2px active
  bar/border (`Palette::selected_fill` / `selected_accent` mirror these).
  Hover deliberately stays neutral. Applied to the active editor tab, Explorer
  / tree rows, command-palette selection, the Settings sidebar's active
  category (+ left accent bar), and `icon_toggle_button`'s pressed state
  (dock rail, notification bell, snippet toggles).
- **D2** — `ThemeStore::scrollbar_thumb()` / `scrollbar_thumb_hover()` =
  foreground at 22% → 34% alpha, `SCROLLBAR_SIZE = 10.0` — matches
  `.themed-scrollbar`. (Wire into a scrollbar widget when one is adopted.)
- **D3** — terminal cursor proportions: beam 1 px, underline 1 px,
  `HollowBlock` renders as a 1 px outline (xterm `cursorInactiveStyle:
  "outline"`), focused block keeps the translucent fill.
- **D4** — tab-entrance animation: `CubicBezier::eval` drives a GPUI
  `with_animation` opacity fade over `--dur-base` with `--ease-premium`;
  `TAB_IN_FROM_SCALE` constant records the reference `scale(0.86)` (GPUI 0.2.2
  `Div` has no scale transform, so opacity-only). Reduce-motion clamps the
  duration to ~0 like the reference `0.01ms` rule.
- **D5** — `theme::menu_metrics` constants (container `p-1.5`, item `px-3
  py-2`, `gap-2.5`, popover `p-4`) from the reference `dropdown-menu` /
  `command` / `popover` components; command-palette row padding aligned to
  `ITEM_PAD_X`.
- **D6** — `community_theme_partial_import_round_trips_visually` test: a
  partial community theme applies only its tokens, leaves the rest on the
  default, and survives export → re-import with no channel drift.
