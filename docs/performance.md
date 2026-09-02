# Performance & Cross-Platform Baseline (T15-003)

Reference doc for the "performance is the motive of the port" goal. Captures
the measurement method, the target envelope, the manual regression checklist,
and an inventory of the hot-path guards already in the code so a future change
can be judged against them.

> Scope note: per the task warning, this pass is **measure-first**. The
> GPUI-native architecture (no WebView, no IPC, no JSON round-trips) already
> removes the structural overhead that made the Tauri/React build slow, and a
> static review found the known hot paths already guarded (see the inventory
> below). This document is the yard-stick; it does not add speculative
> micro-optimisation.

## 1. Baseline measurement method

All numbers are taken on the primary target (macOS, Apple Silicon, release
build: `cargo run --release`). Record them in the table when a machine is
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
- 5k-row lists scroll at display refresh rate (Git-Graph is `uniform_list`
  virtualised; Explorer/SFTP are page-capped — see inventory).
- Git status: exactly one poll per interval (2 s local, ×N remote), skipped
  when there is no repo root.

## 3. Manual regression checklist (run before a release)

- [ ] Cold start feels instant vs. the reference (side-by-side if possible).
- [ ] Type into a fresh local terminal immediately after launch — no lag.
- [ ] `seq 1 200000` in a pane — scrollback stays smooth, memory settles
      after it finishes (buffer is bounded, not retained unbounded).
- [ ] Open/close ~20 terminal + editor tabs in a loop — RSS returns close to
      baseline (sessions/among `panes` map are dropped on close).
- [ ] Explorer + SFTP on a large directory — scroll is smooth, first page
      appears immediately (500-entry page cap + lazy paging).
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
| Fonts | bundled assets registered once at `init_fonts`. | `crates/ui/src/theme.rs` |
| Terminal render | alacritty computes the renderable diff; GPUI retains the element tree and only repaints on `cx.notify()` after PTY output. Cursor overlay reuses the measured cell metrics. | `crates/ui/src/terminal.rs`, `crates/terminal/` |
| Explorer | `generation` counter discards stale async dir reads; 500-entry page cap + lazy expansion; watcher is `notify-debouncer-mini` at 300 ms, non-recursive, watch-set synced to open dirs. | `crates/ui/src/explorer.rs` |
| SFTP list | same page-capped `overflow_y_scroll` column pattern as Explorer. | `crates/ui/src/sftp.rs` |
| Git-Graph | row list virtualised with `uniform_list` — only visible commit rows build elements. | `crates/ui/src/git_graph.rs` |
| Git status poll | `refreshing` flag prevents overlap; `target_gen` guard drops stale results; no-op when `root` is `None`; interval is 2 s local, `× REMOTE_POLL_MULTIPLIER` for SSH. | `crates/ui/src/git.rs` |
| Session sync | workspace meta (cwd/title) pushed to the tab store on change, not polled. | `crates/ui/src/workspace.rs` |
| AI streaming | incremental markdown append from T11-003; no full recompute per chunk. | `crates/ui/src/ai_chat.rs` |

### Follow-ups deliberately deferred (need a profiler + a real workload)

- Windowing `uniform_list` for Explorer/SFTP (currently page-capped, which
  keeps the element count bounded but not constant). Noted in
  `crates/ui/src/explorer.rs` module docs.
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
  `crates/ui/src/menu.rs` — no in-window menu rendering to pay for.
- DPI: GPUI's Metal renderer is scale-factor aware; all sizes are logical
  `px(..)` so Retina is automatic. Terminal cell metrics are derived from
  `text_system().ch_advance` at the current scale.
- Window bounds are persisted/restored (`window_state`).

**Linux (later)**

- Keep it buildable: no macOS-only APIs leak outside `main.rs`'s window
  setup and `menu.rs`. GPUI selects the Vulkan/Blade renderer on Linux; the
  view layer is renderer-agnostic (logical `px`, theme tokens, no platform
  branches in `crates/ui/src/*` view code).
- Open items for the Linux pass: file-dialog / open-in-browser shims,
  keychain backend (`keyring` already abstracts this), font fallback list.

## 6. Visual-parity items closed here (D1–D6 from T15-001)

- **D1** — canonical interaction fills: `ThemeStore::hover_fill()` = `accent`,
  `selected_fill()` = `muted` (1:1 with the reference `focus:bg-accent` /
  `data-selected:bg-muted`). Command palette rows switched from an ad-hoc
  `accent` selection + `border` hover to `selected_fill()`.
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
