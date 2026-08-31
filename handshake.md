# Handshake — Session State (Labonair-rust Port)

Authored by: GPUI-native port of Labonair (formerly Tauri v2 + React 19 → now pure Rust/GPUI).

> This file is the authoritative continuity doc for the **port** project. This is a **hard fork** — fully standalone, no link/symlink/submodule to any external Labonair repo. The old web-app source is a frozen read-only copy at `reference-src/` inside this repo and is the only reference. Do not mistake the old git history/tech for the current target.

## Last Session: 2026-09-01 (T03-002 — GPUI terminal cell renderer)

### What Was Done
- **T03-002 ✅ Done.** The terminal is now a visible, interactive GPUI surface.
  - **`crates/terminal/src/render.rs`** (new, no GPUI) — pure prep helpers:
    - `batch_runs(&RenderableScreen) -> Vec<StyledRun>`: collapses the per-cell grid into per-row runs of identical `RunStyle` (fg/bg/bold/italic/underline/strikeout/hidden) on contiguous columns; a column gap always breaks a run so each run is positioned by `start_col` alone. Trailing blank cells share the default style and stay in one run (correct terminal behavior).
    - `grid_size(w_px, h_px, cell_w, cell_h) -> (cols, rows)`: floor division, min 1×1.
    - `RunStyle`, `StyledRun { line, start_col, text, style }` (+ `width()`).
    - 4 unit tests (run merge, style-change split w/ palette colors, bold breaks run, grid math).
  - **`crates/terminal/src/engine.rs`** — `RenderableScreen` gained `selection: Vec<SelectionSpan>` (per-visible-row spans, end-exclusive), populated in `render()` from `term.renderable_content().selection` (alacritty already resolves the `SelectionRange`; handles block vs linewise, clamps to `columns`). New `TerminalEmulator::set_selection((i32,usize),(i32,usize))` / `clear_selection()` (uses `alacritty_terminal::selection::{Selection,SelectionType}` + `index::Side`) — the mouse task (T03-003) will drive these; exposed now so the renderer can already draw selections. +1 engine test.
  - **`crates/terminal/src/lib.rs`** — re-exports `render::*`, `SelectionSpan`, and `alacritty_terminal::vte::ansi::{CursorShape, Rgb}`.
  - **`crates/ui/src/terminal.rs`** (new) — `TerminalView`, a GPUI entity (`Render` + `Focusable`):
    - `new(theme, window, cx)` spawns a local `TerminalSession` (80×24), focuses itself, `cx.observe`s the `ThemeStore` to re-color the running shell + repaint, and starts a poll `Task` (`cx.spawn` async closure + `cx.background_executor().timer(16ms)`) that drains `session.drain_events()` and only `cx.notify()`s when there were events; the loop stops on `Exit`/`ChildExit` or when the entity is dropped. Spawn failure is stored as `Err(String)` and rendered as a message instead of crashing.
    - `render()` derives cell metrics from the theme: `cell_w = text_system().ch_advance(resolve_font(terminal_font), font_size)`, `cell_h = ceil(font_size * terminal_line_height)`. Fits the grid to `window.viewport_size()` via `grid_size` and calls `session.resize(TermDimensions{..})` only when `(cols,rows)` changed. Paints a `relative` `size_full` container (bg = theme `terminal.background`) with: one absolutely-positioned `div` per `StyledRun` (`.left/.top` from `start_col*cell_w` / `line*cell_h`, `.w(run_width*cell_w)`, `.h(cell_h)`, `.bg(run.style.bg)`, `.text_color`, `.text_size(font_size)`, `.line_height(cell_h)`, `.whitespace_nowrap()`, bold→cloned `Font` with `weight = BOLD`, italic/underline/line_through toggles, hidden→fg=bg); selection spans as translucent overlays (`terminal.selection` @ `selection_alpha`); and a cursor overlay honoring `CursorShape` (Block/HollowBlock = 55%-alpha fill so the glyph shows through, Beam = 2px, Underline = 2px at cell bottom, Hidden/scrolled-out = none).
    - Input: `.on_scroll_wheel` → `Scroll::Delta(lines)` (`ScrollDelta::Lines` used directly, `Pixels` divided by `cell_h`); `.on_key_down` → `keystroke_to_bytes()` then `session.write()` + `Scroll::Bottom` snap; `.on_mouse_down(Left)` refocuses. `keystroke_to_bytes` is the minimal map (ctrl+letter→control byte, named keys enter/tab/backspace/arrows/home/end/delete→sequences, `key_char` passthrough, alt→ESC prefix). Full mapping is T03-003.
    - 4 plain `#[test]`s (ctrl-C, named keys, printable+alt, `to_hsla` black/white).
  - **`crates/ui/Cargo.toml`** — added `labonair-terminal` path dep (no cycle: terminal only deps theme). `crates/ui/src/lib.rs` re-exports `TerminalView`.
  - **`crates/app/src/main.rs`** — `Root` now holds `Entity<TerminalView>` (created with `window` in `Root::new`) and renders it full-window over the theme background; dropped the T02 swatch/sample demo. `Root::new` signature gained `&mut Window`.
- **GPUI API used (gpui 0.2.2, verified in source):** `App::text_system() -> &Arc<TextSystem>`; `TextSystem::{resolve_font(&Font)->FontId, ch_advance(FontId,Pixels)->Result<Pixels>}`; `f32::from(Pixels)` (the `.0` field is private in 0.2.2 — use the `From` impls); `Window::{viewport_size()->Size<Pixels>, focus(&FocusHandle)}`; `App::focus_handle()`; `Context::spawn(async move |WeakEntity<T>, &mut AsyncApp| ...)` (stable async closures) + `AsyncApp::background_executor()` + `BackgroundExecutor::timer(Duration)`; `WeakEntity::update(cx, |&mut T, &mut Context<T>|) -> Result<R>`; `div().id(_)` → `Stateful<Div>`, so a `Render` fn mixing an early `Div` return with a final `Stateful<Div>` must unify via `.into_any_element()` (keep the signature `-> impl IntoElement`, NOT `-> AnyElement`, or clippy's `refining_impl_trait` fires); `InteractiveElement::{track_focus, key_context, on_key_down, on_scroll_wheel, on_mouse_down}`; `Styled::{absolute, relative, whitespace_nowrap, italic, underline, line_through, line_height, text_size, font}`; `ScrollDelta::{Lines(Point<f32>), Pixels(Point<Pixels>)}`; `gpui::Rgba{r,g,b,a: f32}.into() -> Hsla`.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Test counts: 122 backend, 1 app_state, 22 theme, **13 ui (+4)**, **28 terminal (+5)**. Not yet visually run — user should `cargo run` and confirm a live shell renders, is typable, scrolls, and resizes with the window.

### Current State
- Branch `master`, 12 unpushed commits + this one. `crates/ui` owns the GPUI terminal element; `crates/terminal` owns batching + selection snapshot. App root shows one interactive terminal.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) still garbles Next Task Protocol steps 1 & 3 — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### Known limitations (for next tasks, not blockers)
- Resize is driven by `window.viewport_size()` (terminal assumed full-window). Once the app shell / tabs exist (Phase 03) the view must measure its own element bounds instead.
- The renderer repaints the whole grid on any wakeup (no per-region damage tracking). Fine at 80×24–ish; revisit under T15-003 if scrolling large output stutters.
- Selection can be *drawn* but nothing *creates* one yet — `set_selection` is wired for T03-003 (mouse drag-select).
- Wide/CJK cells: the engine already drops `WIDE_CHAR_SPACER` cells; the wide glyph occupies one run cell but is positioned for a single column (double-width advance handled by GPUI text shaping, not by the cell grid). Verify under T03-003/T15-001.

### What's Next
- **T03-003** `tasks/phase-02-terminal/T03-003-keyboard-mouse-mapping.md` — full keyboard + mouse mapping (modifiers, mouse reporting, drag selection, copy/paste, bracketed paste). Deps: T03-002 (done).
- **T02-006** (terminal background images) is now unblocked (needed the renderer element).

### Blockers
- None.

---

## Session: 2026-08-31 (T03-001 — alacritty_terminal engine integration)

### Task selection note
- **T02-006** (terminal background images) was the next roadmap task, but its `## Abhängigkeiten` list `T02-002 (Theme-Store), Phase 2 (Terminal-Renderer — für die tatsächliche Darstellung)` and two of its acceptance criteria require a live terminal element to render into. Phase 02 did not exist. Per the session directive, **T02-006 was left untouched** (still `⏳ Pending`) and **T03-001** was done instead. Return to T02-006 after the Phase 02 renderer (T03-002) lands.

### What Was Done
- **T03-001 ✅ Done.** `crates/terminal` now embeds `alacritty_terminal` 0.24.2 as the emulation core — render-free logic only, no GPUI.
  - **`crates/terminal/src/engine.rs`** (new) — `TerminalEmulator`: wraps `alacritty_terminal::Term<EventProxy>` + `vte::ansi::Processor` (fed byte-by-byte; vte 0.13.1 `advance(&mut handler, u8)`). `feed(&[u8]) -> Vec<TerminalEvent>` runs the parser and returns a trailing `Wakeup` (Term never emits Wakeup itself — that was Alacritty's event-loop's job) plus any `Cwd` recovered by an OSC-7 sniffer that scans the raw stream *before* the parser (`alacritty_terminal` 0.24 ignores OSC 7; OSC 133 is deferred to T03-004). `render() -> RenderableScreen` walks `term.renderable_content().display_iter` and resolves every `vte::ansi::Color` (Named/Indexed/Spec, incl. DIM group + INVERSE swap) to a concrete `Rgb` via `TerminalColors` (T02-004) — never an Alacritty default. `TermDimensions` implements `alacritty_terminal::grid::Dimensions` (own type, not the crate's `term::test::TermSize`). `resize()`, `scroll(Scroll)`, `is_alt_screen()` (TermMode::ALT_SCREEN), `history_len()`, `set_colors()`, `take_pty_output()` (drains DA/DSR replies the `EventProxy` buffers). Config: `scrolling_history = 10_000`.
  - **`EventProxy`** impl `alacritty_terminal::event::EventListener` — maps `Event::{Wakeup,Title,ResetTitle,Bell,Exit,ChildExit,MouseCursorDirty}` onto a `std::sync::mpsc::Sender<TerminalEvent>`; `PtyWrite` bytes are appended to a shared `Arc<Mutex<Vec<u8>>>` (drained by the session's I/O thread, not sent to the UI). Clipboard/Color/CursorBlink/TextAreaSize events dropped. `Clone + Send` so `Term` can move to the reader thread.
  - **`crates/terminal/src/session.rs`** (new) — `TerminalSession::spawn(colors, dims, SessionOptions)`: opens a PTY via `portable-pty` `native_pty_system()`, `CommandBuilder` (default shell = `$SHELL` → `/bin/zsh`; env `TERM=xterm-256color`, `COLORTERM=truecolor`, `TERM_PROGRAM=Labonair`, `LABONAIR_TERMINAL=1` — mirrors `reference-src/.../pty/shell_init.rs`). A dedicated `labonair-pty-reader` OS thread does blocking `reader.read()`, locks the emulator, `feed()`s, writes back any `take_pty_output()` replies, forwards events over the channel; on EOF sends `TerminalEvent::Exit`. UI-facing API: `write()`, `resize(&mut)` (both PTY + grid), `scroll()`, `set_colors()`, `render()`, `with_emulator()`, `drain_events()`, `recv_event_timeout()`, `shell_pid()`. `Drop` kills the child and joins the thread.
  - **`crates/terminal/examples/headless_dump.rs`** (new) — spawns a real shell, runs a colored `printf`, prints the grid to stdout with no GUI. Verified manually: prompt + `GREETING from alacritty_terminal` render correctly.
  - **`crates/terminal/src/lib.rs`** re-exports `engine::*`, `session::*`, and `alacritty_terminal::grid::Scroll`.
- **No new deps** — `alacritty_terminal`, `portable-pty`, `tokio` were already in `crates/terminal/Cargo.toml`; `labonair-theme` path dep already present (T02-004). `alacritty_terminal`'s own `tty`/`event_loop` modules are **not** used — PTY I/O is portable-pty + our thread (per the architecture doc + task note).
- **Tests:** 23 in `crates/terminal` (was 9). `engine::tests` (11): plain-text, SGR color→theme palette, cursor-move CSI, Wakeup on output, OSC 0 title event, OSC 7 cwd, resize, scrollback accumulate/scroll (`Scroll::Top`/`Bottom`), alt-screen `?1049h/l`, INVERSE fg/bg swap. `session::tests` (4, spawn real `/bin/sh`): runs a command & reads it back, ANSI color from shell output resolves to theme green, `stty size` reflects a resize, `exit` produces an Exit/ChildExit event.
- **GPUI API used:** none (this task is renderer-free).
- **alacritty_terminal 0.24.2 API (verified in `~/.cargo/registry/src/.../alacritty_terminal-0.24.2/src`):** `Term::new<D: Dimensions>(Config, &D, T)`, `Term::resize<S: Dimensions>(S)`, `term.renderable_content() -> RenderableContent { display_iter: GridIterator<Indexed<&Cell>>, display_offset, cursor: RenderableCursor { point: Point<Line,Column>, shape }, colors, mode }`, `term.grid().history_size()`, `term.scroll_display(Scroll)`, `term.mode().contains(TermMode::ALT_SCREEN)`. `Cell { c, fg, bg, flags: Flags }`; `Flags::{BOLD,ITALIC,DIM,INVERSE,STRIKEOUT,HIDDEN,ALL_UNDERLINES,WIDE_CHAR_SPACER,LEADING_WIDE_CHAR_SPACER}`. `event::{Event, EventListener, WindowSize}`. VTE parser = `alacritty_terminal::vte::ansi::Processor` (vte **0.13.1**, `advance(&mut H: Handler, byte: u8)` — single byte, not a slice). `NamedColor` discriminants: system 0–15 contiguous, `Foreground=256`, then Background/Cursor/Dim*/Bright*/Dim* — `named as usize` arithmetic is safe for the Bright/Dim ranges.

### Current State
- Branch `master`, 11 unpushed commits + this one. `crates/terminal` owns the emulation core + PTY sessions; nothing renders it yet.
- Pre-existing uncommitted `CLAUDE.md` edit (not mine) still garbles Next Task Protocol steps 1 & 3 — left untouched & excluded from the commit, flag to user.
- `reference-src/` untouched.

### What's Next
- **T03-002** `tasks/phase-02-terminal/T03-002-gpui-terminal-renderer.md` — GPUI cell renderer consuming `engine::RenderableScreen`. Deps: T03-001 (done) + T02-004 (done).
- **T02-006** (terminal background images) is still pending and now only blocked on the renderer element from T03-002.

### Blockers
- None.

---

## Session: 2026-08-31 (T02-005 — font handling & bundling)

### What Was Done
- **T02-005 ✅ Done.** Bundled the reference app's font families natively and wired them into GPUI's text system.
  - **`crates/theme/assets/fonts/`** (new) — SIL OFL 1.1 font files committed into the repo: `InterVariable.ttf` + `InterVariable-Italic.ttf` (rsms Inter 4.1), `JetBrainsMono-{Regular,Medium,Bold,Italic,BoldItalic}.ttf` (JetBrainsMono 2.304). Family `name`-table values verified with fonttools: `"Inter Variable"` and `"JetBrains Mono"` (Medium carries typographic-family id16 `"JetBrains Mono"` so weight selection works). `LICENSE` + `Inter-OFL.txt` + `JetBrainsMono-OFL.txt` document redistribution rights. SF Mono / Menlo are runtime fallbacks only, never bundled.
  - **`crates/theme/src/fonts.rs`** (new module, `pub mod fonts`) — `embedded_fonts() -> Vec<Cow<'static,[u8]>>` via `include_bytes!` (7 files), plus `UI_FONT_FAMILY`/`MONO_FONT_FAMILY` consts and `UI_FONT_FALLBACKS` (`[".SystemUIFont","sans-serif"]`) / `MONO_FONT_FALLBACKS` (`["SFMono-Regular","Menlo","monospace"]` — mirrors the reference CSS stack). lib.rs re-exports all. 2 tests (all 7 assets are valid TrueType sfnt & >10KB; family-name stability).
  - **`crates/theme/src/tokens.rs`** — `Typography` gained font fields matching `preferencesStore` defaults: `ui_font_fallback`, `buffer_font_family` + `buffer_font_size` (13), `terminal_font_family`, `terminal_font_size` (14), `terminal_line_height` (1.05), `terminal_letter_spacing` (0), `terminal_font_weight` (new `MonoFontWeight` enum Normal/Medium/Bold), `mono_font_fallback`, `font_ligatures` (true — reference always loads xterm `LigaturesAddon`). `Typography::default()` fills them from `crate::fonts` consts; `Theme::light()/dark()` unchanged (both use the default). Existing `app_*` fields untouched.
  - **`crates/ui/src/theme.rs`** — `ThemeStore` accessors: `ui_font()` / `buffer_font()` / `terminal_font()` -> `gpui::Font` (family + `FontFallbacks::from_fonts` + weight; mono honors `font_ligatures` via `FontFeatures::disable_ligatures()`), `terminal_font_size()`, `terminal_line_height()`. New free fn **`init_fonts(cx: &App)`** = `cx.text_system().add_fonts(labonair_theme::embedded_fonts())` (non-fatal on error). lib.rs re-exports `init_fonts`. 1 new `#[gpui::test]`.
  - **`crates/app/src/main.rs`** — `labonair_ui::init_fonts(cx)` first thing in `Application::run`; `Root::render` now sets `.font(ui_font)` on the root and renders a mono/ligature sample line (`-> => != >= …`) in `terminal_font` for visual parity checks.
- **GPUI API used (gpui 0.2.2, verified in source):** `TextSystem::add_fonts(Vec<Cow<'static,[u8]>>)` (mac impl uses `CGFont::from_data_provider` → needs ttf/otf, **not** woff2/ttc); `App::text_system() -> &Arc<TextSystem>`; `gpui::font(family)` builder + `Font { features, fallbacks, weight, style, .. }`; `FontFallbacks::from_fonts(Vec<String>)` / `.fallback_list()`; `FontFeatures::disable_ligatures()` (sets `calt=0`) / `.is_calt_enabled()`; `FontWeight::{NORMAL,MEDIUM,BOLD}`; `Styled::font(Font)`.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 122 backend, 1 app_state, **22 theme (+2)**, **9 ui (+1)**, 9 terminal. Not yet visually run — user should `cargo run` to confirm UI renders in Inter and the sample line shows JetBrains Mono ligatures.

### Current State
- Branch `master`, 10 unpushed commits + this one. `crates/theme` owns the font assets + `Typography` font fields; `crates/ui` exposes `gpui::Font` accessors + `init_fonts`.
- Pre-existing uncommitted `CLAUDE.md` edit (not mine) still garbles Next Task Protocol steps 1 & 3 — left untouched & excluded from the commit, flag to user.
- `reference-src/` untouched.

### What's Next
- **T02-006** `tasks/phase-01-theme/T02-006-terminal-background-images.md` — terminal background images. Deps: T02-002 (done) + Phase 2 (terminal engine, not yet started) — **may be blocked** until Phase 02 exists; check the task file's dependencies before starting.

### Blockers
- T02-006 lists a "Phase 2" dependency — the terminal engine (T03-*) does not exist yet. Next session must check whether T02-006 can proceed or whether Phase 02 comes first.

---

## Session: 2026-08-31 (T02-004 — terminal ANSI palette → engine bridge)

### What Was Done
- **T02-004 ✅ Done.** Built the theme → terminal-engine color bridge in **`crates/terminal/src/palette.rs`** (new module; `crates/terminal` gained a `labonair-theme` path dep).
  - **`Rgb`** is re-used straight from `alacritty_terminal::vte::ansi::Rgb` (`{r,g,b: u8}`) so Phase 02 integration is friction-free. `alacritty_terminal` re-exports `vte`, so the path is `alacritty_terminal::vte::ansi::{NamedColor, Rgb}` and `alacritty_terminal::term::color::Colors`.
  - **`TerminalColors`** (`Debug/Clone/Copy/PartialEq`, not `Eq` — carries an `f32`): `background`, `foreground`, `bright_foreground`, `dim_foreground`, `cursor`, `cursor_text` (= bg, xterm `cursorAccent`), `selection` (alpha stripped) + `selection_alpha: f32`, and `normal`/`bright`/`dim: [Rgb; 8]`. `from_theme(&Theme)` / `from_palette(&TerminalPalette)`. Row order = black,red,green,yellow,blue,magenta,cyan,white.
  - **`TerminalColors::ansi256(u8) -> Rgb`** — standard xterm 256 scheme: 0–7 normal, 8–15 bright, 16–231 6×6×6 cube (`cube_axis(n) = 0 if n==0 else 55+n*40`), 232–255 grayscale (`8 + 10*(i-232)`). Cube/ramp are derived, not from globals.css (per task note).
  - **`TerminalColors::to_alacritty_colors() -> Colors`** — fills all 256 ANSI slots + `Foreground`/`Background`/`Cursor` + `BrightForeground` (267) + `DimForeground` (268) + the `DimBlack..DimWhite` group (259–266). This is what a Phase-02 session hands the engine.
  - **`ansi_self_test() -> String`** — ANSI escape dump (16 system + 216 cube + 24 grayscale + SGR attrs incl. `\x1b[2mdim`) for visual parity checks against Labonair (`ls`/Vim/Git).
  - Conversion reuses `labonair_theme::to_rgb8` (same rounding as theme export) → "< 1/255" is exact by construction.
  - `crates/terminal/src/lib.rs` re-exports `TerminalColors`, `ansi_self_test`.
  - **9 tests** in `palette::tests`: three-rows-present, named-range lookup, cube+grayscale xterm scheme (16→black, 231→white, 196→red, 46→green, 232→#080808, 255→#eeeeee), exact conversion for all 8 normal colors, dark values vs globals.css (`--terminal-yellow`==primary #E6B450, `--terminal-red`==destructive #F26D78, bright white/fg ≈ #FFFFFF), light≠dark, `selection_alpha`==0.13, alacritty `Colors` list fully filled, self-test covers all 256 indices.
- **Deferred to Phase 02** (explicitly this task's "Weiterführende Tasks — Phase 2: Terminal-Engine übernimmt die Paletten-Integration in der Praxis"): binding `TerminalColors` into a live PTY session, and the `cx.observe(&theme_store)` re-color-on-theme-switch hook. The bridge + resolution + self-test are complete and green now.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` — all green. Counts: 122 backend, 1 app_state, 20 theme, 8 ui, **9 terminal (new)**.

### Current State
- Branch `master`, 9 unpushed commits + this one. `crates/terminal` now owns the palette bridge; `crates/theme` untouched.
- Uncommitted pre-existing `CLAUDE.md` edit (not mine) still garbles Next Task Protocol steps 1 & 3 — left untouched, flag to user.
- `reference-src/` untouched.

### What's Next
- **T02-005** `tasks/phase-01-theme/T02-005-font-loading.md` — font handling / bundling (GPUI). Dep (T01-001) satisfied.
- Phase 02 (T03-001) will consume `TerminalColors` when it integrates alacritty_terminal.

### Blockers
- None.

---

## Session: 2026-08-31 (T02-003 — theme import/export for user themes)

### What Was Done
- **T02-003 ✅ Done** (functional layer; settings-UI wiring deferred to T13-002 — no shell/settings surface exists in Phase 1).
- **`crates/theme/src/import.rs`** (new) — the JSON ⇆ typed `Theme` conversion layer:
  - `ThemeFile` / `ThemeFileVariant` serde structs, Labonair-compatible (`name`/`author`/`author_url`/`version`/`description` + `variants: BTreeMap<String, {mode,label,colors}>`). `from_json` / `to_json` / `validate` (name non-empty, ≥1 `light` + ≥1 `dark` variant — matches backend `themes::Theme::validate`).
  - `Theme::from_theme_file(&ThemeFile, dark: bool) -> Result<(Theme, Vec<String>), String>` — resolves the mode's variant, starts from the built-in `Theme::light()/dark()` default, overlays every recognised token. Unknown token names + unparseable colors become **warnings**, never errors; omitted tokens keep their default. Color parsing reuses `color::parse_color` (hex/rgb/oklch/transparent).
  - `Theme::to_theme_file(name, author) -> ThemeFile` — serializes all `COLOR_TOKENS` to `#rrggbb(aa)` via new `color::to_hex`, writes the same set into both a `light` and `dark` variant so export always re-imports cleanly.
  - `pub const COLOR_TOKENS: &[&str]` — the 72 canonical dot-notation keys (core, sidebar.*, surface, border.*, status, cursor/selection, terminal.* + terminal.ansi.{,bright_,dim_}*). `set_token`/`get_token` are inverses; `set_token` also accepts a few legacy aliases (`terminal_red`, `card-foreground`, …).
  - lib.rs re-exports `ThemeFile`, `ThemeFileVariant`, `COLOR_TOKENS`, `to_hex`. Module-level doc block documents the full JSON schema.
  - 5 tests: parse+convert matching variant, bad/unknown → warnings not errors, validate rejects missing-mode/name, export round-trips (all 72 tokens within ±1/255), every canonical token is settable.
- **`crates/ui/src/theme.rs`** — `ThemeStore` gained `custom_file: Option<ThemeFile>` alongside `custom: Option<Theme>`:
  - `import_theme_file(ThemeFile, cx) -> Result<Vec<String>, String>` — validates, resolves for the current mode, stores both, `notify()`, returns warnings. Invalid file → `Err`, nothing activated.
  - `clear_custom_theme(cx)`, `active_theme_file(name) -> ThemeFile` (export).
  - `reresolve_custom()` re-derives `custom` from `custom_file` when the resolved mode changes — wired into `set_preference` + `set_system_appearance`. `set_custom_theme` now also clears `custom_file`.
  - lib.rs re-exports `ThemeFile` / `ThemeFileVariant`. 3 new `#[gpui::test]`s (import activates + follows mode switch + clear; invalid rejected without activating; export→import round-trip).
- **`crates/backend/src/modules/themes/mod.rs`** — persistence layer was already ported in T01-002 (file-based `config_dir()/themes/*.json`: `themes_get_all`, `theme_import`, `theme_export`, `theme_delete`, `theme_create`, download/index). Added `fn is_protected(id)` helper (used by `theme_delete`) + 3 tests (bundled default parses+validates, validate needs both variants, `default` id is protected).
  - NB: the T02-003 task text claims Labonair stores themes "in einer Datenbank" — the reference (`reference-src/src-tauri/src/modules/themes/mod.rs`) actually uses JSON files in a themes dir. Followed the reference.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` — all green. Counts now: 122 backend, 1 app_state, 20 theme, 8 ui.

### Current State
- Branch `master`, 8 unpushed commits + this one. Theme crate now owns import/export conversion; `ThemeStore` can activate imported themes and follows mode switches for them.
- Uncommitted pre-existing `CLAUDE.md` edit (not mine) still garbles Next Task Protocol steps 1 & 3 — left untouched, flag to user.
- `reference-src/` untouched.

### What's Next
- **T02-004** `tasks/phase-01-theme/T02-004-terminal-palette.md` — integrate the terminal ANSI palette into the theme. Deps (T02-001) satisfied.
- Later: **T13-002** must wire `import_theme_file` / `active_theme_file` / backend `themes_*` into the Appearance settings pane (the deferred criterion 7 of T02-003).

### Blockers
- None.

---

## Session: 2026-08-31 (T02-002 — runtime theme provider + store)

### What Was Done
- **T02-002 ✅ Done.** Added the runtime theme layer in `crates/ui/src/theme.rs`.
  - `ThemeStore` — GPUI entity, the single source of truth for the active theme. Holds `preference: ThemePreference` (System/Light/Dark), `system_mode: ThemeMode` (resolved from `WindowAppearance` via `ThemeMode::from_appearance`, Vibrant* mapped to Light/Dark), the default `light`/`dark` `Theme`s built once from `labonair-theme` (never recomputed), and `custom: Option<Theme>` for imported themes (T02-003).
  - `theme(&self) -> &Theme` returns the custom theme if set, else the default for the resolved mode — cheap, no alloc. `mode()` resolves preference against `system_mode`.
  - Mutators each guard on equality and only `cx.notify()` on real change: `set_preference`, `set_system_appearance` (only re-renders when preference == System), `set_custom_theme(Option<Theme>)`.
  - Convenience accessors: `background/foreground/card/muted/muted_foreground/border/primary/accent -> Hsla`, `radius() -> RadiusScale`, `shadows() -> &Shadows`, `animation() -> &Animation`.
  - Global: `GlobalTheme(pub Entity<ThemeStore>)` + `init(appearance, &mut App) -> Entity<ThemeStore>` (creates entity + `set_global`), `theme_store(&App)`, `active_theme(&App) -> &Theme`.
  - 5 `#[gpui::test]`s (preference switching, system-appearance follow, custom override+fallback, accessor parity, global access). `crates/ui/Cargo.toml` gained `[dev-dependencies] gpui = { features = ["test-support"] }`.
  - `crates/app/src/main.rs`: window init calls `labonair_ui::init_theme(window.appearance(), cx)`, wires `window.observe_window_appearance(..)` → `store.set_system_appearance(..)`. `Root` view now holds `Entity<ThemeStore>`, `cx.observe`s it for re-render, and renders sample swatches (primary/accent/muted/destructive/border) + a card surface using live theme colors.
- **Verified:** `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (119 backend + 1 app_state + 15 theme + 5 ui) — all green. `cargo build --bin labonair` OK. Not yet visually run — user should `cargo run` to confirm swatches + system dark/light.

### Current State
- Branch `master`, 7 unpushed commits + this one. `crates/ui` now owns the theme provider; `crates/theme` unchanged.
- GPUI API used: `Window::observe_window_appearance(FnMut(&mut Window, &mut App)) -> Subscription` (1 arg, no cx), `App::window_appearance()` / `Window::appearance() -> WindowAppearance` (gpui 0.2.2). `#[gpui::test]` needs `gpui` dev-dep feature `test-support`.
- Uncommitted `CLAUDE.md` edit (pre-existing, not mine) still garbles Next Task Protocol steps 1 & 3 (status vocabulary removed, "to ." dangling). Left untouched — flag to user.
- `reference-src/` untouched.

### What's Next
- **T02-003** `tasks/phase-01-theme/T02-003-theme-import-export.md` — theme import/export for user themes. Deps (T02-001, T02-002) satisfied.

### Blockers
- None.

---

## Session: 2026-08-31 (T02-001 — extract theme tokens from globals.css)

### What Was Done
- **T02-001 ✅ Done.** Transcribed every design token from `reference-src/src/styles/globals.css` into typed Rust in `crates/theme/`.
  - `crates/theme/src/color.rs`: Oklch→sRGB→`gpui::Hsla` conversion via the `palette` crate (`oklch(l%, c, h)` / `oklch_a(..., alpha)`), `transparent()`, `to_rgb8()`, and a `parse_color()` accepting `oklch(...)`, `#rgb/#rgba/#rrggbb/#rrggbbaa`, `rgb()/rgba()` (space or comma sep, `/ alpha` as number or %), and `transparent` — for later user-theme import (T02-003).
  - `crates/theme/src/tokens.rs`: `Theme` struct with named typed fields for every category — `CoreColors` (incl. `charts: [Hsla;5]`), `SidebarColors`, `SurfaceColors`, `BorderVariants`, `StatusColors`, `InteractionColors`, `TerminalPalette` (`AnsiColors` ×3 for normal/bright/dim + bg/fg/bright_fg/dim_fg + cursor/selection), `RadiusScale` (base 5px from `0.3125rem`@16px, sm..xl4 via the `calc()` multipliers, `window`=12px), `Shadows` (`Vec<ShadowLayer>{x,y,blur,spread,color}` per tier — row/popover/modal), `Animation` (`Duration` 160/240/320ms + two `CubicBezier`), `Typography` (Inter Variable, 13px, 1.5).
  - `Theme::light()` / `Theme::dark()` factory methods with values transcribed 1:1 from the `:root` / `.dark` blocks. `var(--x)` refs resolved to their concrete values; `color-mix()` only appears in the non-token `.themed-scrollbar` rule so nothing to resolve.
  - 15 unit tests: oklch extremes/grayscale/stability, design-hex spot checks (#E6B450, #F26D78, #3C3C3C, #2B2B2B within ±3/255), parse round-trips, radius multipliers, shadow layer counts, animation durations, terminal palette distinctness.
  - Deps: `palette = "0.7"` added to workspace + `crates/theme/Cargo.toml`.
- **Verified:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (119 backend + 1 app_state + 15 theme) — all green.

### Current State
- Branch `master`, 6 unpushed commits + this one. `crates/theme` is the design-token source of truth; not yet consumed by any UI.
- Uncommitted `CLAUDE.md` edit (pre-existing, not mine) garbles the Next Task Protocol step 1/3 wording (status vocabulary removed, "to ." dangling). Left untouched — flag to user.
- `reference-src/` untouched.

### What's Next
- **T02-002** `tasks/phase-01-theme/T02-002-theme-provider-store.md` — runtime theme provider + store (GPUI global). Deps (T02-001) satisfied.

### Blockers
- None.

---

## Session: 2026-08-31 (T01-005 — CI pipeline)

### What Was Done
- **T01-005 ✅ Done.** Added `.github/workflows/ci.yml`: triggers on push to `master` + all PRs, `concurrency` cancel-in-progress, `RUSTFLAGS: -D warnings`.
  - `macos-latest` job (timeout 60m): `dtolnay/rust-toolchain@stable` (rustfmt+clippy) + `Swatinem/rust-cache@v2` → `cargo fmt --all --check` · `cargo check --workspace --all-targets` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo test --workspace`.
  - `ubuntu-latest` job: `cargo check` only, `continue-on-error: true` (Linux is "later", must not block).
- `.github/dependabot.yml`: dropped the npm ecosystem + the stale `/src-tauri` cargo dir; now one `cargo` entry at workspace root `/` + `github-actions`.
- `.github/labeler.yml`: rewritten from `src/**` / `src-tauri/**` globs to the `crates/**` structure (backend, ui, theme, terminal, editor, ai, ssh/sftp, hosts, git, snippets, ci, dependencies).
- `.github/release.yml`: changelog categories updated (added Git, renamed Frontend→UI/Theme, `frontend`→`ui` label).
- Old web workflows remain only under `reference-src/.github/workflows/` (untouched, reference only). `CODEOWNERS` unchanged (`* @Snenjih` still correct).
- **Verified locally:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (119 backend + 1 app_state) — all green. No Rust code touched.

### Current State
- Branch `master`, 6 unpushed commits + this one. CI config in place but **not yet run on GitHub** (push only on user request) — acceptance criterion "CI grün auf Test-PR" still open.
- `reference-src/` untouched.

### What's Next
- **Phase 00 complete.** Next is **T02-001** `tasks/phase-01-theme/T02-001-*.md` — extract design tokens from `reference-src/src/styles/globals.css`. Dep (T01-001) satisfied.

### Blockers
- None.

---

## Session: 2026-08-31 (T01-004 — typed event system + logging)

### What Was Done
- **T01-004 ✅ Done.** Built a typed routing layer on top of the existing string `EventBus` from T01-002 — additive, no churn to the ~40 ported `app.emit("name", json)` call sites.
- `crates/backend/src/events.rs`: renamed the transport struct `AppEvent` → **`RawEvent`** (name + `serde_json::Value`, still what the broadcast channel carries). Added the typed **`AppEvent`** enum (variants: Transfer{Progress,Completed}, FileConflict, Ssh{SessionEstablished,AuthRequired,PassphraseRequired,KnownHostsWarning,ConnectionLost}, DirChanged, MenuActivated, Mcp{OpenTabRequest,CloseTabRequest,GrantExpired}). `AppEvent::event_name()` maps each variant → the exact string the existing call site emits; `AppEvent::from_raw(&RawEvent) -> Option<AppEvent>` decodes by name + serde (tolerates extra payload fields via `#[serde(default)]` on Options, ignores unknowns).
- `EventBus::emit_event(AppEvent)` — serializes externally-tagged, unwraps to the flat field object so typed + string emitters share one wire shape. `App::emit_event()` forwards to it.
- **One call site converted to typed** (acceptance criterion): `modules/fs/watcher.rs` `fs:dir-changed` → `AppEvent::DirChanged` (dropped the local `DirChangedPayload` struct).
- `lib.rs` now re-exports `RawEvent` alongside `AppEvent`.
- `crates/app/src/main.rs`: `init_logging()` with `EnvFilter` (`warn,labonair=debug,labonair_backend=debug`, `RUST_LOG`-overridable, ANSI+target). `main()` now builds a `tokio::runtime::Runtime`, constructs `Backend::new(dirs::data_dir()/labonair)`, calls `spawn_workers()`, and `spawn_event_logger()` subscribes to the bus and logs every event (typed via `from_raw`, else trace). Runtime is `mem::forget`-kept for process lifetime. Then the GPUI window opens as before.
- Workspace `tracing-subscriber` gained `features = ["env-filter"]`; `crates/app` gained `dirs`.
- 3 unit tests in `events.rs` (typed round-trip, extra-field decode, unknown-name → None).
- **Verified:** `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (119 backend + 1 app_state) — all green. `cargo build --bin labonair` OK.

### Current State
- Branch `master`, 4 unpushed commits + this one. Backend event layer is typed; app wires bus + logging.
- **Not yet visually run** — user should `cargo run` to confirm the startup log line prints and the window still opens.
- `reference-src/` untouched.

### What's Next
- **T01-005** `tasks/phase-00-setup/T01-005-ci-pipeline.md` — GitHub Actions: cargo check/clippy/test/fmt on macOS. Deps (T01-001) satisfied. Last task in phase 00.

### Blockers
- None.

---

## Session: 2026-08-31 (T01-003 — verify reference-src + project docs)

### What Was Done
- **T01-003 ✅ Done.** Verified all reference paths under `reference-src/` exist and are readable: `src/styles/globals.css`, `src/modules/` (23 feature modules), `src-tauri/src/modules/` (all listed backend modules incl. `secrets.rs`, `dock_menu.rs`, `menu_sync.rs`, `errors.rs`), `src-tauri/Cargo.toml`, `src-tauri/src/modules/pty/scripts/` (zshrc/bashrc + z* init scripts). Nothing missing.
- **README.md** already existed (from T01-001) and already describes the hard-fork / read-only `reference-src/` character. Added a "Goal" (full feature parity, web-preview → native markdown) and "Status" (links to ROADMAP + handshake) section.
- **.gitignore** confirmed: `reference-src/` is NOT ignored (tracked); no external symlink entry.
- Reworded 3 historical lines in `handshake.md` that still contained the literal old `../Labonair` path so `git grep -n "\.\./Labonair"` is now 0 hits.
- **Verified:** `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` (117 tests) — all green.

### Current State
- Branch `master`, 3 unpushed commits + this one. No code changes — docs only.
- `reference-src/` untouched.

### What's Next
- **T01-004** `tasks/phase-00-setup/T01-004-event-system.md` — typed event routing/logging on top of `crate::events`. Deps (T01-001, T01-002) satisfied.
- Then T01-005 (CI).

### Blockers
- None.

---

## Session: 2026-08-31 (T01-002 — full backend port from reference-src, Tauri stripped)

### What Was Done
- **T01-002 ✅ Done.** Ported the entire Rust backend from `reference-src/src-tauri/src/modules/` into `crates/backend/src/modules/` (18 modules: ssh, sftp, git, fs, pty, hosts, credentials, snippets, secrets, shell, themes, backgrounds, fonts, scrollback, terminal_exec, settings, mcp, errors). ~16k LOC, ~150 `#[tauri::command]` fns.
- **Approach: verbatim copy + mechanical Tauri strip** (perl passes in `/tmp/detauri.pl`) rather than hand-retyping — preserves logic 1:1. Transformations: drop `#[tauri::command]`; `tauri::State<'_,T>` → `&T`; `tauri::AppHandle` → `crate::App`; `tauri::ipc::Channel<T>` → `crate::events::EventChannel<T>`; `use tauri::*` removed; `.inner().clone()` → `.clone()`; `tauri::async_runtime::spawn` → `tokio::spawn`; `app.state::<T>()` → `&app.<field>` (5 sites, hand-fixed); `app.emit(...)` → `crate::App::emit`.
- **New infra:** `crates/backend/src/events.rs` (`EventBus` broadcast + `EventChannel<T>` point-to-point sink — forward-compatible with T01-004), `crates/backend/src/app.rs` (`App`/`AppState` = `Arc<AppInner>` holding every sub-state + `EventBus`; `App::new(&Path)` opens SQLite + builds all state; `App::spawn_workers()` starts the SFTP transfer worker + MCP auto-revoke sweeper). `lib.rs` re-exports `App`, `AppState`, `AppError` (= `LabonairError`, `From` impls extended with `serde_json::Error` + `String`), `AppResult`.
- **Hand-rewritten:** `settings/mod.rs` (`tauri_plugin_store` → plain atomic JSON read/merge/write against `config_dir()/labonair-settings.json`). `modules/mod.rs` (dropped `dock_menu`/`menu_sync` → deferred to T04-005).
- **Deps added to `crates/backend/Cargo.toml`:** russh(+rsa), russh-sftp, rusqlite, reqwest(rustls), portable-pty, tokio-util, thiserror, log, aes-gcm, rand, md5, openssl(vendored), base64, ignore, grep-regex, grep-searcher, globset, notify, notify-debouncer-mini, flate2, vte 0.15, rmcp 2.2, axum 0.8, schemars, fontdb, socket2, libc, dirs, uuid. (No git2 — git via CLI. No keyring — secrets use aes-gcm local store.)
- **One pre-existing broken test fixed:** `git::tests::branches_parses_current_and_upstream_tracking` used `|` separators but `parse_branches` splits on `\0` (matches `BRANCH_FORMAT` `%00`) — test data corrected, no logic change.
- **Verified:** `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` — all green. 116 backend unit tests + `tests/app_state.rs` (AppState init acceptance test) pass.

### Current State
- Branch `master`, 2 unpushed commits ahead + this one. `crates/backend` fully ported and green. `crates/app` still just the empty GPUI window (does not wire `AppState` yet — that's later phases).
- `reference-src/` untouched.

### What's Next
- **T01-003** `tasks/phase-00-setup/T01-003-reference-symlink.md` — verify `reference-src/` is intact + write project README/.gitignore notes. Deps (T01-001) satisfied.
- Then T01-004 (event system — build typed routing on top of `crate::events`), T01-005 (CI).

### Blockers
- None. Note for later: SSH/SFTP/git/MCP got a *mechanical* Tauri strip — runtime behaviour needs the user's testing once a UI wires them (per the project's KI-builds / user-tests workflow).

---

## Session: 2026-08-31 (T01-001 — Cargo workspace scaffolded, empty GPUI window runs)

### What Was Done
- **T01-001 ✅ Done.** Scaffolded the cargo workspace: root `Cargo.toml` (resolver 2, `[workspace.dependencies]`) + 7 crates under `crates/`: `app` (bin `labonair`), `ui`, `theme`, `terminal`, `editor`, `backend`, `ai`. Placeholder `lib.rs` per crate (doc comment only — **not** the `pub mod ...;` stubs the task listed, since those module files don't exist yet and would break `cargo build`; later phases add them).
- **`crates/app/src/main.rs`** — GPUI entry: `Application::new().run(...)`, `Bounds::centered`, `cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), .. }, |_, cx| cx.new(|_| Root))`, `cx.activate(true)`. `Root` is a `Render` view: full-size `div` bg `0x1a1b26`, child text "Labonair-rust — ready for development" in `0xc0caf5`. `tracing_subscriber::fmt::init()`.
- **Deps corrected vs task file**: `gpui = "0.2.2"` (real crates.io crate; `gpui-component` is a **separate** crate `0.5.1`, NOT a gpui feature — deferred to T04+). Dropped `alacritty_config_derive`, `fontdb`, `rsa` feature on russh (not needed yet). russh resolved to 0.62.7, russh-sftp 2.4.0.
- **Metal Toolchain installed** — `cargo build` of gpui failed with "missing Metal Toolchain"; fixed via `xcodebuild -downloadComponent MetalToolchain` (688 MB, one-time, machine-level).
- Root `README.md` added (workspace layout + commands + Metal Toolchain note).
- Verified: `cargo check`, `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` all green. `cargo run` opens the window — **user visually confirmed** "ready for development" text renders.

### Current State
- Branch `master` (still 1 unpushed planning commit ahead + this one). `Cargo.lock` committed.
- Working GPUI window. `reference-src/` untouched.

### What's Next
- **T01-002** `tasks/phase-00-setup/T01-002-extract-backend-logic.md` — port backend modules from `reference-src/src-tauri/src/modules/` into `crates/backend/`, stripping Tauri/IPC. Deps (T01-001) satisfied.

### Blockers
- None.

---

## Session: 2026-08-31 (Planning correction: hard-fork restructure, reference-src, roadmap gaps closed)

### What Was Done
- **Hard fork clarified & enforced.** User: Labonair-rust must be fully decoupled — no symlink/submodule/path-dependency to the original repo. Memory written (`memory/hard-fork-reference-src.md`).
- **Moved the copied web app into `reference-src/`** (`git mv` of `src/`, `src-tauri/`, `docs/`, `scripts/`, all web configs, `CHANGELOG/CONTRIBUTING/SECURITY/README`, and the old `.github/workflows/`). Repo root is now clean: `CLAUDE.md`, `handshake.md`, `tasks/`, `reference-src/`, `LICENSE`, `.github/` (templates only), `.gitignore`.
- **Swept old external relative paths → `reference-src`** across CLAUDE.md, handshake.md, ROADMAP.md, and all task files (0 remaining hits for the old parent-dir path).
- **`.gitignore` rewritten** for Rust (`/target`, `.claude/`, `session-*.md`); `reference-src/` is tracked.
- **T01-001 deps corrected** to match `reference-src/src-tauri/Cargo.toml`: `russh 0.62.2` (ring), `russh-sftp 2.3.0`, `rusqlite 0.40`, `portable-pty 0.9`; **removed `git2`** — git runs via the `git` CLI (local + remote-over-SSH), same as the original. Same fix applied in T01-002 and T09-001.
- **T01-003 rewritten** — no longer a symlink-to-parent-repo task; now "verify reference-src + write README + .gitignore". Standalone.
- **Roadmap gaps closed** (user wants full feature parity, nothing out of scope). Added tasks:
  - `T01-005` CI pipeline (cargo check/clippy/test/fmt on macOS)
  - `T02-005` font handling/bundling · `T02-006` terminal background images
  - `T04-003` app-shell & window chrome (header/statusbar/sidebar/root coordinator) · `T04-004` notifications/toasts · `T04-005` native macOS menus (app menu + dock menu)
  - `T11-005` MCP bridge server · `T11-006` MCP bridge UI/grants
  - `T15-005` auto-updater (Sparkle) · `T15-006` feature-parity acceptance checklist
  - ROADMAP: added "Feature-Parität" section + success criteria 16–21. Only accepted deviation: web-preview tab → native markdown + open-in-system-browser (GPUI has no WebView).

### Current State
- Branch `master`. This planning-correction commit is **local, not yet pushed** (push when user asks).
- `reference-src/` holds the full frozen original. No cargo workspace scaffolded yet.
- Roadmap now has **55 task files** across 15 phases + expanded ROADMAP.md.

### What's Next
1. Execute **`tasks/phase-00-setup/T01-001-setup-cargo-workspace.md`** per the Next Task Protocol: scaffold the cargo workspace (7 crates), empty GPUI window, `cargo check`/`clippy`/`run` green → mark `✅ Done` → update handshake → commit.
2. Then T01-002 (extract backend from `reference-src/`) → T01-003 (verify reference-src + README) → T01-004 (events/logging) → T01-005 (CI).

### Blockers
- None. T01-001 has no dependencies.

---

## Session: 2026-08-31 (Roadmap complete + repo created + CLAUDE.md Next Task Protocol)

### What Was Done
- **Roadmap built out fully.** `tasks/ROADMAP.md` defines the 15-phase port (Setup → Theme → Terminal → Tabs → Explorer → Editor → SSH → SFTP → Git → Git-Graph → AI → Snippets/Palette → Settings → Session → Testing/Polish). 43 task files exist under `tasks/phase-*/` (each is instruction-only: context, goal, instructions, acceptance criteria, notes, warnings — no code). Naming: `T{NN}-{OOO}.md` where NN = phase 01–15, OOO = task number (files currently use the sequential scheme, e.g. `T01-001-setup-cargo-workspace.md`, `T13-001-settings-preferences.md`, `T15-004-packaging-release.md`). Next Task Protocol in CLAUDE.md consumes these in order.
- **Initial commit** `ead881a` committed the full repo (including the copied original source, kept as reference).
- **GitHub repo created & linked**: `gh repo create Labonair-rust --public --source=. --remote=origin --push` → remote `origin` set, `master` tracked to `origin/master`, URL https://github.com/Snenjih/Labonair-rust
- **CLAUDE.md rewritten** for the port (was stale Tauri/web content). Now documents: Rust/GPUI commands (`cargo check/build/run/clippy/test/fmt`), the single-binary GPUI architecture, Critical Rules (no web tech, reference-only original, GPU API must be source-verified), **Next Task Protocol** (the core task-by-task workflow), session start/end protocols, bug-memory rule, reference usage, language protocol.
- **handshake.md reset** for the port project (this file).

### Current State
- Branch `master`, repo pushed to `origin/master`.
- **First implementation task** (`T01-001-setup-cargo-workspace.md`) is the next to work — nothing implemented yet (only roadmap + config docs). `cargo` project has NOT yet been scaffolded.
- The original source files (src/, src-tauri/, docs/, package.json, etc. from the clone) still sit in the repo as an untracked-from-the-web-app reference. **These are reference artifacts for the port to read, NOT the app to extend.** Future phases will replace the web-tech portions.

### What's Next
1. Execute **T01-001-setup-cargo-workspace.md** per the Next Task Protocol in CLAUDE.md: scaffold the cargo workspace, set it up, verify (`cargo check`), mark `✅ Done`, update this handshake, commit.
2. Then T01-002 → T01-003 (reference symlink → `reference/`) → T01-004, and continue through the phases in order.

### Blockers
- None. Next task is dependency-clean (T01-001 has no dependencies).

---

## Previous Session: (none — port infrastructure only)

Project start. Created repo clone, initialized git, wrote the full task roadmap + task files. No application code written yet.
