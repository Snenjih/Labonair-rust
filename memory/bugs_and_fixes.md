# Bugs, fixes, and non-obvious constraints

Older entries preserve the state of the code when each issue was recorded.
When an API was later renamed or removed, the current implementation and
normative documentation take precedence over the historical symbol name.

## 2026-09-10 — Keymap surface UX rework: GPUI / jsonc-parser constraints

**`uniform_list` requires a uniform item height.** It measures item 0 and
reuses that height for every row. The keymap tab now mixes section-header rows
and command rows in one `uniform_list`, so both are forced to the same fixed
height (`ROW_HEIGHT == SECTION_HEIGHT`, wrapped in `div().h(..).w_full()`).
A shorter header row silently overlapped the command rows below it.

**Capturing a keystroke without triggering its action needs
`App::intercept_keystrokes`, not `on_key_down`.** Interceptors run in
`Window::dispatch_key_event` *before* `dispatch_key` (action matching);
`cx.stop_propagation()` inside the interceptor prevents action dispatch. A
plain `on_key_down` handler runs after action matching for global bindings and
cannot suppress them. `gpui-component`'s `Input` also binds `up`/`down`/`enter`
as actions in its own `"Input"` key context, so an ancestor `on_key_down`
never sees those keys while an `Input` is focused — the keymap tab hand-rolls
its filter string via a single view-level `on_key_down` router instead (same
pattern as `HostManagerView::on_search_key`).

**`gpui::Keystroke`'s `Display` renders platform glyphs (`⌘⇧P`), not a
parseable binding string.** `labonair-keymap-ui::keystroke::to_binding_string`
emits `cmd-shift-p` form in `labonair_keymap::normalize_single_keystroke` rank
order (ctrl, alt, shift, cmd) so the result is already normalized and
round-trips through `Keystroke::parse`. Modifier-only keystrokes
(`key == "cmd"` etc., which GPUI synthesizes on a lone-modifier tap) are
dropped during capture.

**`jsonc-parser` 0.23 exposes byte ranges on every AST node** via
`parse_to_ast` + `Ranged` (`ObjectProp.range`, `Object.range`, `Array.elements`).
`file::remove_user_binding_override` uses this for a surgical text edit
(delete matched property spans + one adjacent comma, drop a block that would
be left empty), then re-parses and returns `Err` if the result would not
round-trip rather than writing a broken file. `serde_json::Value` (via
`parse_to_serde_value`) has no ranges — that path is only for validation.

## 2026-09-10 — Terminal settings: dead `terminalCursorBlink`, stale opacity, 7 unwired fields

**Symptom:** `terminalCursorBlink` had a Settings row + a `Toggle: Terminal
Cursor Blink` palette command and was threaded into `alacritty_terminal`'s
`default_cursor_style.blinking`, but nothing ever blinked — `RenderableCursor`
dropped the `blinking` bit and `AlacEvent::CursorBlinkingChange` was ignored
"Not needed by the current renderer". `terminalOpacity` was only read in
`TerminalView::render` with no `SettingsStore` observer, so it only updated on
the next unrelated repaint. `terminalScrollback` / `terminalCursorStyle` were
spawn-only. `settings-inventory.md` marked `terminalFontWeight`,
`terminalCursorBlinkInterval`, `terminalWordSeparator`,
`terminalScrollSensitivity`, `terminalFastScrollModifier`,
`confirmCloseTerminalTab` (and, contradictorily, `terminalLineHeight`) as
**Remove**.

**Fix (user-approved promotion to Keep):**
* `RenderableCursor` gained `blinking`, populated from `Term::cursor_style().blinking`
  (respects DECSCUSR). `TerminalView` drives a blink `Task` off
  `terminalCursorBlinkInterval`, holds the cursor solid for one interval after
  key/output (`activity_at`), and only blinks while focused.
* `EmulatorConfig` gained `word_separators`; `TerminalEmulator::apply_config`
  calls `Term::set_options(Config)` — the clean live re-tune path for
  scrollback depth / default cursor style / `semantic_escape_chars`.
* `TerminalEmulator::select_word_at` uses `SelectionType::Semantic` +
  `include_all()` (reads `semantic_escape_chars`); wired to double-click
  (`MouseDownEvent.click_count >= 2`).
* Wheel handler multiplies the step by `terminalScrollSensitivity` and ×5 when
  `terminalFastScrollModifier` is held.
* `Workspace::reapply_terminal_settings` (called from the shell's
  `SettingsStore` observer) pushes `apply_runtime_config` into every open pane
  and `cx.notify()`s each `TerminalView` so opacity / blink-interval are live.
* `confirmCloseTerminalTab` gates `Workspace::request_close` for a
  `TabKind::Workspace` tab with a `SessionStatus::Running` local pane.
* `terminalLineHeight` / `terminalFontWeight` flow through
  `theme_ui::font_overrides_from_settings` → `FontOverrides` (new
  `terminal_line_height` read + `terminal_weight`), applied live by the
  Settings UI's existing `apply_prefs_to_theme` call.

**Alacritty 0.24.2 APIs used:** `Term::set_options(Config)` (updates
`grid.update_history`, `default_cursor_style`, `semantic_escape_chars`),
`Term::cursor_style() -> CursorStyle { shape, blinking }`,
`Selection::new(SelectionType::Semantic, point, side)` + `include_all()`.

## 2026-09-09 — `tabsLocation == "sidebar"` did nothing (no Tabs panel existed)

**Symptom:** setting Appearance → "Tab bar location" to *Sidebar* made the
titlebar tab strip vanish with no replacement. `titlebar.rs` already gated the
strip on `tabs_location() == "sidebar"` (`let tabs = (!tabs_in_sidebar).then(...)`)
but there was no panel rendering the tabs anywhere — the "Tabs sidebar panel"
the comments referenced was never built.

**Fix:** added `TabsPanel` (`crates/workspace/src/tabs_panel.rs`), a thin
`labonair_panel::Panel` wrapper owned by `labonair-workspace` (the tab owner).
It renders `Workspace::render_tab_list_vertical` — a column variant of
`render_tab_bar`; `render_tab` grew a `sidebar: bool` for full-width rows,
click-anchored context/new-tab menus (instead of the `TITLEBAR_OFFSET`
y-anchor) and a top-edge drop indicator. New `PanelIcon::Tabs` variant in the
`labonair-panel` contract → `IconName::Tab` in `dock_status_item.rs`.

**Non-obvious wiring:**
- The panel entity is created in `shell/src/bootstrap.rs` (like the other
  panel owners) but `labonair_workspace::tabs_panel::tabs_panel_registration`
  builds the contribution. `Workspace` keeps the handle (`tabs_panel` field,
  set in `init_docks`) so it can re-dock the panel at runtime.
- `init_docks` special-cases `name == "tabs"`: it is only auto-placed if a
  persisted layout already had it; otherwise presence is decided by
  `Workspace::sync_tabs_in_sidebar`, called once after `init_docks` and from a
  `SettingsStore` observer in `bootstrap.rs` on every settings change (early
  returns when dock membership already matches the setting).
- Precedent for a chrome view calling `workspace.update(cx, |w, cx|
  w.render_*(cx))` inside its own `render`: `titlebar.rs` already does exactly
  this with `render_tab_bar`.

## 2026-09-07 — Background presentation moved behind `BackgroundHost` (R07-005)

**Change:** `labonair-workspace` (`Workspace` and `TerminalView`) used to hold
`Entity<BackgroundStore>` directly and call `.read(cx).layer(scope)` plus
`cx.observe(&background, ...)`. It now holds a
`labonair_background_host::BackgroundHost` — a `Rc<dyn Fn(&App, LayerScope) ->
Option<AnyElement>>` layer callback plus an `Entity<BackgroundPulse>` (a
zero-sized entity `labonair-background::host()` bumps via `cx.observe` on the
real store whenever it notifies). Call sites become
`self.background.layer(cx, scope)` and `cx.observe(background.pulse(), ...)`.
`LayerScope` itself moved to the new `labonair-background-host` leaf crate;
`labonair-background` re-exports it so existing call sites (`BackgroundStore::layer`,
`app_shell.rs`'s `LayerScope::App`) keep compiling unchanged.

## 2026-09-07 — `scripts/screenshot.sh` rejects a `./`-relative launch

**Finding:** The helper validates the target PID's command path by prefixing a
non-absolute path with `$REPO_ROOT/` and string-comparing it to
`$REPO_ROOT/target/debug/labonair`. Launching the binary as
`./target/debug/labonair` yields `ps` command `./target/debug/labonair`, which
becomes `$REPO_ROOT/./target/debug/labonair` — the embedded `/./` fails the
exact match and the capture is refused as "not the native Rust executable".

**Workaround:** launch the native app by its absolute path
(`/Users/.../target/debug/labonair`) or via `cargo run -p labonair`; both
produce a command token the validator normalizes. macOS Screen Recording
permission for the runner is otherwise in place as of this date — captures
succeed once the PID check passes.

## 2026-09-07 — Explorer no longer holds `Entity<Workspace>` (R07-004)

**Change:** `labonair-panel-explorer` used to store the workspace entity and
call `Workspace::{open_file,new_terminal_tab_in,open_preview,active_file_path}`
plus re-export `labonair_workspace::drag`. It now holds a
`labonair_explorer_host::ExplorerHost` (four `Rc<dyn Fn>` callbacks) built by
the shell. Auto-reveal lost its `cx.observe(&workspace)`; the composition root
calls the new public `ExplorerView::notify_active_file_changed` from the
existing workspace observer instead. `DraggedPaths` / `quote_paths` /
`shell_quote` / `is_previewable` / `PREVIEW_EXTENSIONS` moved from
`labonair-workspace` to the new leaf crate `labonair-explorer-host`;
`workspace/src/drag.rs` is deleted and `views/{terminal,preview}.rs` import
from `labonair-explorer-host`.

## 2026-09-07 — Normative status metadata may be qualified

**Finding:** The documentation governance checker initially required the
exact value `Normative`, but current contracts intentionally use qualified
values such as `Normative target architecture` and `Normative ownership map`.

**Resolution:** The checker accepts a `Normative` prefix while still rejecting
supporting, working, or missing status metadata.

## 2026-09-06 — Keep project-settings loading out of render

**Finding:** `Workspace::render` synchronized the project-settings layer. Even
with a cached root comparison, that path could load/reload a project file and
publish notifications as a render side effect.

**Resolution:** Project settings are synchronized from the typed
`WorkspaceTransition` boundary and from the explicit project-settings refresh
command only. Rejected-key notifications are deduplicated there as well. The
pure `WorkspaceContext` owns transition mutation and no-op detection; the GPUI
workspace only coordinates the resulting side effects.

## 2026-09-06 — Command action names belong to the command ID

**Finding:** The shell lifecycle-registry test initially called
`action_name()` on `CommandDescriptor`, but canonical action-name conversion
is implemented on `CommandId` in `labonair-command-palette-core`.

**Resolution:** The test now resolves `descriptor.id.action_name()`, keeping
the assertion on the same typed identity used by the registry and keymap
compatibility layer.

## 2026-09-06 — Empty-state shortcut must use the registered palette binding

**Finding:** The native Empty workspace surface displayed `⌘K` for Commands,
but the canonical `CommandPalette` shortcut registry defines `⌘P`.

**Resolution:** The workspace hint now displays `⌘P`, keeping the visible
shortcut and the keymap registry consistent.

## 2026-09-06 — Smoke launch must prove the native process stays alive

**Finding:** The optional GUI smoke path started the bundled executable
directly, but only waited and then killed its PID. A binary that exited during
startup could therefore be reported as launched successfully.

**Resolution:** The smoke test now opens the `.app` with `open -n -W` using
its absolute repository path, resolves the resulting exact Rust executable
PID, checks that PID with `ps`, and stops that PID directly.

The first run after this check was added exposed an additional native launch
failure: the release executable stayed alive for the first second but then
terminated with `Abort trap: 6` before the five-second interval ended. The
smoke test now checks both ends of the interval, so this remains a real launch
blocker rather than a false green result.

The macOS diagnostic report identified the failure in `NSApplication` and
LaunchServices (`_RegisterApplication` / `GetCurrentProcess`) while the
process has bundle ID `com.labonair.rust`; it is not a legacy-app collision.
That applied to direct executable invocation; opening the exact `.app` path
through LaunchServices is the reliable macOS form.

The reliable macOS launch form is `open -n -W` with the absolute `.app` path.
Launching the executable directly can initialize AppKit without a visible
window in this runner, while the exact bundle launch produces a PID-scoped
window that `scripts/screenshot.sh` can verify.

## 2026-09-06 — Never launch visual checks through the shared app name

**Finding:** A generic `open -a Labonair` launch can resolve the installed
legacy Tauri application because it shares the native app's display name.

**Resolution:** The release smoke test and manual verification open the
absolute Rust bundle path and then target its exact executable PID. Release
documentation uses that path or `cargo run -p labonair`; the generic
application-name launch is explicitly forbidden.

## 2026-09-06 — Project identity must drive Explorer and Git roots

**Finding:** Selecting a project updated `WorkspaceIdentity` and project
settings, but shell synchronization still derived Explorer and Git roots only
from the active terminal cwd. A project with no terminal therefore continued
to show the home directory, and a terminal `cd` could move project surfaces.

**Resolution:** `labonair-shell::bootstrap` now resolves an explicit project
root first. Only standalone workspaces fall back to the active terminal cwd;
the Explorer may then fall back to the home directory, while Git remains
unscoped without a project or terminal. Pure resolver tests cover project
precedence and standalone fallbacks.

## 2026-09-06 — Keep the native bundle distinct from the legacy app

**Finding:** The native Rust bundle and the installed legacy Tauri bundle both
used `com.labonair.app`. macOS LaunchServices could therefore resolve the
wrong application identity while developing or opening the bundle.

**Resolution:** The native package now uses the distinct identifier
`com.labonair.rust` consistently in `crates/app/Cargo.toml`, the macOS
`Info.plist`, entitlements, release documentation, and bundle smoke test.
The old app remains a separate legacy process and cannot satisfy the native
bundle checks.

**Verification:** `scripts/smoke-test.sh` rebuilt the bundle, verified the
identifier and structure, and passed all three Rust smoke tests. Launching the
bundle with `open` from this restricted shell still returns macOS
LaunchServices error `-10827`; no screenshot from that attempt is accepted as
GUI evidence. The failure is an environment/launch-path issue still requiring
manual confirmation outside this shell.

## 2026-09-06 — Replace workspace shell callbacks with typed events

**Finding:** `Workspace` stored an `open_hosts_hook` supplied by the shell and
invoked it from feature UI. This made workspace behavior depend on a shell
callback and hid the actual cross-surface navigation contract.

**Resolution:** `WorkspaceEvent::OpenHosts` is now emitted by the workspace;
`labonair-shell` subscribes at composition time and opens the canonical Hosts
palette page. The workspace no longer stores or invokes a shell hook.

**Non-obvious constraint:** Project identity must not be inferred from the
active terminal's current working directory. Standalone terminals can have a
cwd, so `WorkspaceContext` starts standalone and changes to `Project` only
through an explicit project-opening transition.

**Build note:** The first compile after this change failed because the shell's
internal `workspace` re-export exposed only `Workspace`, not the new
`WorkspaceEvent`. Re-exporting the event from `crates/shell/src/shell.rs`
restored the intended composition-root boundary.

**Environment note:** `cargo run` reaches the running native binary, but the
available desktop UI surface exposes no Labonair window in this session. A
system screenshot showed only the Codex desktop, so the R02-002 visual check
cannot be treated as passed from this environment.

## 2026-09-06 — Remove the stale shortcuts menu path

**Finding:** The native Window menu still exposed `Keyboard Shortcuts` through
an unregistered legacy action, while the current product surface is the
canonical keymap flow.

**Resolution:** The menu now exposes `Open Keymap (JSON)` and maps the old
`settings::OpenShortcuts` action name to the existing `OpenKeymapJson` action.
This preserves old user keymap files without keeping a duplicate Settings or
Shortcuts surface.

## 2026-09-06 — Make visual screenshots fail closed

**Finding:** `scripts/screenshot.sh` initially identified windows only by the
generic owner name `Labonair`. Because `/Applications/Labonair.app` is the
legacy Tauri app, a visual check could inspect that app instead of the Rust
binary. The earlier full-display fallback had the same class of risk.

**Resolution:** The script now resolves `target/debug/labonair`, accepts an
optional Rust PID, filters CoreGraphics windows by that exact process PID, and
exits with an error when no matching layer-0 window is present. Only a
screenshot targeted at the Rust process is accepted as visual evidence.

The helper also recognizes the packaged Rust executable under
`target/release/bundle/macos/Labonair.app/Contents/MacOS/labonair`. Launching
the bundle was necessary for reliable macOS window visibility in the visual
check; the installed legacy app remains a separate process and is never a
valid target.

## 2026-09-06 — Keep the active rework queue authoritative

**Finding:** `R01-001` was still marked in progress even though later R01
tasks were marked done and `R02-002` was active. The queue README therefore
disagreed with the earliest incomplete task.

**Resolution:** Closed `R01-001` at its actual foundation scope, documented
remaining backend adapters as later migrations, listed every rework task in
order, and added `scripts/check_rework_queue.py` to enforce one active task
and earliest-incomplete ordering.

## 2026-09-06 — Make repository placement and feature changes explicit

**Finding:** The architecture rules described ownership and dependency
direction, but contributors still had to infer where capability files belonged
and which design/registry/persistence checks preceded implementation.

**Resolution:** Added normative `docs/repository-layout.md` and
`docs/feature-lifecycle.md`, linked from the repository instructions and docs
index. The lifecycle now requires classification, ownership, public contracts,
UI-kit and user-message decisions, migration order, removal checks, and the
full verification gates before a task is complete.

## 2026-09-06 — GPUI path picker and explicit project settings identity

**Finding:** GPUI 0.2.2 exposes the native folder picker through
`App::prompt_for_paths(PathPromptOptions)`, returning a oneshot receiver. The
first implementation attempted to resolve the method through
`BorrowMut<App>` on `Context<AppShell>`, which was ambiguous.

**Resolution:** Call `prompt_for_paths` directly through `Context`'s `Deref`
to `App`, then handle the asynchronous result in the shell composition root.
The selected directory is applied through
`Workspace::apply_transition(WorkspaceTransition::OpenProject { root })`.

**Important behavior:** Project settings must be keyed by explicit workspace
identity. The active terminal's cwd is only terminal metadata and must never
activate or replace the project settings layer.

**Build note:** `WorkspaceIdentity::project_root()` returns `Option<&Path>`;
when a owned path is needed, use `map(Path::to_path_buf)` rather than
`cloned()`, because the latter is not available on this `Option` shape.

## 2026-09-06 — Icon-theme preview must be a separate transient layer

**Finding:** The command palette already previewed app themes, but icon themes
had only an active persisted ID. Applying a highlighted icon theme directly
would make scrolling change user settings before confirmation.

**Resolution:** `ThemeStore` now keeps an optional, non-persisted icon-theme
preview ID. `icon_theme()` resolves the preview before the active ID,
`cancel_icon_theme_preview` restores the active selection, and activation
clears the preview. The palette emits separate preview events for app and icon
theme pages so changing pages cannot leave the other preview layer active.

**Non-obvious constraint:** The Explorer already reads `ThemeStore::icon_theme`
for every render, so the preview becomes visible through the existing icon
rendering path without a second UI or icon registry.

## 2026-09-06 — GPUI global-menu events must leave the titlebar

**Finding:** Calling Settings or workspace actions directly from the titlebar
would make permanent shell chrome own feature behavior and would prevent the
same entry points from being reused by other global surfaces.

**Resolution:** The titlebar emits a small `TitlebarEvent`; bootstrap subscribes
to it and delegates to Settings, Workspace keymap opening, or a Command Palette
page. The shared `popover_menu` remains the only menu implementation.

**Non-obvious constraint:** `MouseDownEvent::position` is window-space, so the
global menu anchor must retain the event's x coordinate and use the titlebar
bottom as y. Passing a local element coordinate reproduces the old top-left /
opposite-side placement bug.

## 2026-09-06 — Host UI needs shared capability state, not the backend facade

**Finding:** Removing `labonair-backend` from `labonair-hosts-ui` required the
UI to receive the same database and secret cache that the application already
owns. Constructing a second `SecretsState` would have split the cache and could
have produced inconsistent secret reads.

**Resolution:** `AppInner.secrets` is now an `Arc<SecretsState>`, and Workspace
injects the shared database, secret handle, data directory, and a typed
`HostEventHandler`. The callback preserves MCP grant revocation when a host is
blocked without exposing the broad application facade to the UI crate.

## 2026-09-06 — Settings migrations must not recreate removed capability areas

**Finding:** Removing `mcp`, `personalization`, `hosts`, and `keymap` from
`SettingsContent` exposed that the v1→v2 migrator still generated those areas.
This caused missing-key panics in migration fixtures and would have recreated
the ownership problem on every migrated installation.

**Resolution:** The migrator now writes only the seven canonical
`SettingsContent` areas, leaves standalone MCP preferences untouched for the
MCP owner, treats host-manager preferences as explicitly skipped, and keeps
host-store migration as a separate named legacy step. Migration fixtures now
assert that removed areas are absent while real value overrides still survive.

**Non-obvious constraint:** `settings-content/src/hosts.rs`, `mcp.rs`, and
`personalization.rs` remain public only as migration wire types. They must not
be added back to `SettingsContent` or the generated Settings schema.

## 2026-09-06 — Command metadata must be shared with keymap and palette

**Finding:** The command palette had a static presentation table while the
shell kept a second behaviour/context registry. This allowed commands to be
visible in one surface but absent from keymap discovery or dispatch.

**Resolution:** Added the UI-free `labonair-command-palette-core` registry and
descriptor/provider contracts. The palette now adapts injected snapshots,
the shell keeps only execution closures, and keymap loading resolves default
shortcuts through the same descriptor registry. The old palette `COMMANDS`
table and its shortcut lookup path were removed.

**Non-obvious constraint:** The keymap core must not depend on GPUI. GPUI's
`KeybindDisplay` remains a presentation adapter in the command-palette crate,
while keymap tests validate binding shape without importing GPUI.

## 2026-09-06 — Full AI test suite needs local listener permission

**Finding:** The first sandboxed `cargo test --workspace` run failed only in
the pre-existing AI end-to-end streaming test because its local HTTP fixture
could not bind a listener and returned `Operation not permitted`.

**Resolution:** Reran the unchanged workspace test suite with the approved
local socket permission. All workspace tests passed; this is an environment
constraint, not a product or SSH/SFTP failure.

## 2026-09-06 — SFTP view must close SSH when subsystem setup fails

**Finding:** The first injected SFTP connection path could leave the newly
authenticated SSH session registered if opening the SFTP subsystem failed.

**Resolution:** The view now disconnects the SSH session on SFTP-open failure,
while normal tab retirement closes the SFTP handle before disconnecting SSH.

## 2026-09-06 — SFTP must not own host authentication

**Finding:** The first draft of the SFTP service accepted `host_id`, password,
database, and secret-resolution concerns. The SSH/SFTP audit showed that this
would merely recreate the backend facade at a new crate boundary and would
make the SFTP contract responsible for authentication.

**Resolution:** Split the contract into `SftpSessionService` and
`SftpBrowserService`. SSH creates the authenticated session; SFTP receives an
opaque `SftpSessionHandle` and exposes only remote filesystem operations. The
backend adapter temporarily maps that handle to the existing session registry
while the remaining consumers migrate.

## 2026-09-06 — GPUI checks require the local macOS compiler cache

**Finding:** A sandboxed GPUI check failed while Metal shader compilation tried
to write Clang modules below the user's local cache directory.

**Resolution:** The affected GPUI checks were rerun with the approved local
compiler-cache access. Pure SSH/SFTP/backend checks compile without that
requirement.

## 2026-09-06 — Settings management categories must be removed as a UI boundary

**Finding:** Removing `Themes`, `Hosts`, and `Shortcuts` from `AREAS` alone
left dead custom panes, field-registry entries, deep-link tests, and a host
settings callback. That would make the old ownership model appear removed
while still retaining unreachable Settings behavior.

**Resolution:** Removed the three navigation entries and their Settings-only
panes, removed host/keymap management rows from the Settings field registry,
and kept the persisted `SettingsContent.hosts`/`keymap` values only as an
explicit migration-compatibility layer. `Open Hosts` now targets the command
palette Hosts page; host management surface wiring is left to the Hosts
capability instead of keeping a broken `settings://hosts` deep link.

## 2026-09-06 — Historical architecture documents must be visibly superseded

**Finding:** Several reports and early ADRs remained in active-looking paths
and still described Settings-host management, toasts, and the former task
tree. The current normative docs were correct, but readers could not reliably
distinguish historical evidence from project rules.

**Resolution:** Added explicit superseded markers, introduced the normative
capability matrix and Settings UI guidelines, documented the full transitional
edge families, and added a rework task template. The old reports remain
available for traceability but cannot override `docs/` contracts.

## 2026-09-06 — SCM and Project Diff need one source-control capability

**Finding:** `panel-scm` and the workspace Project Diff still imported the
backend facade directly for Git values and every local/remote operation. A
graph-only contract was not sufficient because staging, commits, branches,
tags, stashes, and synchronization were still coupled to backend internals.

**Resolution:** Expanded `labonair-git` with the UI-free Git value types and
`GitService` operation contract. `BackendGitService` now adapts the existing
executor, while both UI surfaces receive only `Arc<dyn GitService>` from
composition. The dependency verifier now rejects the old backend edges.

## 2026-09-06 — SSH snippet execution needs an injected capability contract

**Finding:** After local snippet execution moved into `labonair-snippets`, the
panel still called the backend's russh function directly and translated raw
`AppEvent` payloads back into UI events. That preserved the old backend/UI
coupling and made the local and SSH execution paths structurally different.

**Resolution:** Added shared `SnippetRunEvent` values and the asynchronous
`SshCommandExecutor` contract to `labonair-snippets`. The backend now exposes
`BackendSshExecutor` as an adapter that owns russh session/channel details.
`SnippetsView` receives the trait object during composition and consumes typed
events through its existing local channel. The raw event wrapper remains only
as a compatibility seam for older callers.

**Non-obvious constraint:** `SnippetRunState` must be shared by the injected
adapter and cancellation calls, so `AppInner::snippet_run` is an `Arc` rather
than an inline state value. This keeps cancellation independent from the UI
and avoids copying the in-flight channel registry.

## 2026-09-06 — Capability panels need cloneable infrastructure handles

**Finding:** `panel-snippets` had already stopped using backend execution
functions, but its Cargo dependency and every CRUD operation still pulled in
the whole `Backend` facade solely to reach the shared SQLite connection.

**Resolution:** `labonair-persistence::Database` now owns an `Arc<Mutex<_>>`
and implements `Clone`. The panel receives that narrow handle directly, while
the shell injects the concrete SSH executor separately. The dependency
allow-list now rejects a backend edge for the snippets panel.

## 2026-09-06 — Git graph UI should depend on a graph-specific contract

**Finding:** `panel-git-graph` used the broad backend facade for repository
detection, log loading, commit details, and branch actions. That made a pure
graph view know about both local and SSH transport state.

**Resolution:** Added the UI-free `labonair-git` crate with `CommitInfo`,
`GitFuture`, and `GitGraphService`. `BackendGitGraphService` adapts the
existing backend Git executor, while the graph view receives an
`Arc<dyn GitGraphService>` from composition. The panel no longer depends on
`labonair-backend`; its existing UI and generation-guard behavior is unchanged.

## 2026-09-06 — Local snippet execution belongs in the snippets capability

**Finding:** Local snippet execution was implemented in
`backend::modules::snippets::exec`, even though it only needed a shell,
working directory, process cancellation, and output events. Passing the whole
backend `App` into that code coupled a local feature to unrelated SSH, MCP,
and persistence state.

**Resolution:** Added `labonair_snippets::exec` with `LocalRunRegistry`,
typed `LocalRunEvent` values, and a caller-provided event sink. The snippet
panel now runs local silent snippets directly through that API and cancels
them through its own registry. The backend retains only the SSH execution
adapter until a narrow SSH session contract is available.

**Compatibility note:** SSH output still uses the existing app event bus
while its transport contract is extracted. Local output no longer crosses
that stringly-typed bus.

## 2026-09-06 — Notification lifecycle must be UI-free

**Finding:** `labonair-notifications` combined notification data, GPUI entity
state, toast rendering, severity-specific timers, callback actions, and an
error preference gate. That made the notification system a presentation
implementation instead of a reusable cross-module registry.

**Resolution:** Added `labonair-notifications-core` with typed notification
kinds, stable registry IDs, retained records, read state, details, action
metadata, bounded history, and explicit deduplication. The GPUI crate now only
adapts the registry and keeps temporary callback compatibility. Notifications
are retained for the statusbar dropdown; no timer or toast layer remains.

**Verification detail:** The statusbar uses `overflow_y_scroll` for the full
history and toggles record details on row activation. The unread badge is
driven by registry read state rather than total retained records.

## 2026-09-06 — Remove dead error-notification setting with the toast path

**Finding:** `notify_on_errors` was exposed by Settings but no longer had a
valid product behavior once all messages were required to reach the central
notification dropdown.

**Resolution:** Removed the field from the typed settings content, legacy
preferences projection, migration mapping, and settings UI. Existing JSON
remains forward-compatible because unknown removed fields are ignored during
deserialization.

## 2026-09-06 — Shared database lifecycle is infrastructure, not host logic

**Finding:** The existing `HostsDb` wrapper initialized one SQLite database
containing hosts, credentials, and snippets. Treating that wrapper as the
host module's store would keep unrelated persistence concerns coupled.

**Resolution:** Created `labonair-persistence` for the shared SQLite
connection, schema creation, and idempotent migrations. The backend keeps only
the compatibility alias `HostsDb` and the old initialization path for now;
feature-specific query ownership remains the next migration step.

## 2026-09-06 — Host read and group operations can move without App state

**Finding:** Host listing, ordering, and group CRUD only need the shared
database and do not need secrets, MCP grants, or GPUI state.

**Resolution:** Added `labonair_hosts::store` and migrated host manager and
snippet consumers to use it directly. Secret-bearing create/update/delete and
duplicate operations remain in the backend adapter until their secret and MCP
event contracts are explicit.

## 2026-09-06 — Host write ownership moved behind typed requests

**Finding:** Keeping the old host write implementation in `backend` would
leave the host capability dependent on the application facade even after its
domain and read paths were extracted.

**Resolution:** Moved create, update, duplicate, delete, and sudo-password
storage into `labonair_hosts::store` using `HostCreateRequest` and
`HostUpdateRequest`. `HostEvent::AgentAccessBlocked` is the only callback
contract; the backend adapter remains responsible for inspecting MCP grants
and emitting `AppEvent::McpGrantExpired`. Added a direct store test proving
secrets do not enter the `Host` read model.

## 2026-09-06 — Credential capability has the same backend facade pattern

**Finding:** Credential metadata, secret storage, host references, and SSH
keypair generation were all implemented in `backend::modules::credentials`.
The module only needed database, secrets, and an explicit data directory; it
did not need `App` or UI state.

**Resolution:** Extracted the implementation into
`labonair-credentials`, including the russh-compatible keypair tests. The
backend module now only adapts the old App-based signatures and resolves the
data directory at the composition boundary.

## 2026-09-06 — Snippet persistence is independent from process execution

**Finding:** Snippet models and CRUD queries lived beside local/SSH execution
and therefore forced the entire backend module into every snippet UI caller.

**Resolution:** Created `labonair-snippets` with domain models and the SQLite
store, migrated `panel-snippets` to consume it directly, and retained only
the execution functions in the backend until process/session contracts are
extracted.

## 2026-09-06 — Host domain models must be separated before host persistence

**Finding:** `backend::modules::hosts` combined serializable host models,
SQLite persistence, secret access, and App/MCP behavior. Moving the whole
module would have created another capability-owned crate with the same global
coupling.

**Resolution:** Extracted only `Host`, `Group`, and `ReorderItem` into the
UI-/backend-free `labonair-hosts` contract crate. `HostsDb` and App-bound
operations remain explicitly transitional in `labonair-backend` until a
narrow host store/service API and typed MCP-revocation callback are defined.
Feature crates now import the domain models directly.

## 2026-09-06 — Structured errors extracted as a shared contract

**Finding:** `LabonairError` was implemented in `backend`, so any future
capability crate that needed consistent classification or recovery metadata
would have to depend on the backend facade.

**Resolution:** Created `labonair-errors` and moved the catalog, conversions,
classification rules, serialization, and tests there. `backend::modules::errors`
now only re-exports it, preserving existing internal call sites while making
the contract available to standalone capability services.

## 2026-09-06 — Secret storage extracted without retaining App ownership

**Finding:** The secret store used `backend::App` only to resolve its data
directory, which made encryption/cache logic appear to belong to the backend
and forced unrelated callers through that facade.

**Resolution:** Created `labonair-secrets` with `SecretsState::new(data_dir)`;
the crate now owns path resolution from its state, plain/encrypted migration,
cache, and secret operations. At the time, `backend::modules::secrets` was a
compatibility adapter until SSH, Hosts, and MCP consumed the service directly;
that adapter was later removed once the direct migration completed.
Added isolated plain-store and encryption-toggle round-trip tests.

## 2026-09-06 — Interrupted Cargo rebuild can leave stale artifact locks

**Finding:** After `cargo clean` during a full-disk condition, an interrupted
parallel build left missing intermediate proc-macro artifacts and stale Cargo
processes holding the artifact lock. This produced misleading `syn`/`serde`
compile errors and prevented a second clean.

**Resolution:** Confirmed the affected processes and stopped only those build
PIDs, recreated the regenerable target directories, and reran the changed
crate tests serially. The focused `labonair-filesystem` and
`labonair-backend` suites passed; the full workspace had already passed before
the final watcher-only contract change, while workspace check and Clippy also
passed after it.

## 2026-09-06 — First platform boundary extracted from backend

**Finding:** Local file access, traversal, mutation, path resolution, and
search were implemented under `labonair-backend`, making feature crates reach
through the backend facade for a shared platform capability.

**Resolution:** Created `labonair-filesystem` as a UI-free workspace crate,
moved the filesystem services, watcher implementation, and tests there,
centralized home-path expansion, and kept only a small application-event
adapter in the backend as an explicit transitional seam. Feature crates now
consume the filesystem crate directly; the dependency verifier tracks the new
platform edge.

## 2026-09-06 — Workspace AI integration test needs loopback permission

**Finding:** The first sandboxed `cargo test --workspace` run failed only at
`client::tests::end_to_end_streams_openai_sse_over_http` because the test binds
a local TCP listener and the sandbox returned `Operation not permitted`.

**Resolution:** Re-ran the unchanged workspace test with approved loopback
network permission; all workspace tests passed. This is an environment
constraint, not a product or documentation regression.

## 2026-09-06 — Architecture reset supersedes the port-era planning contract

**Finding:** The former `AGENTS.md`, architecture document, settings contract,
roadmap, and reports still treated feature parity, a toast layer, Hosts in
Settings, and a broad backend as active decisions. They conflicted with the
new Dev-Op workspace direction.

**Resolution:** Added the v2 product, architecture, module, registry,
design-system, workspace, settings, and rework-roadmap contracts under
`docs/`; reduced `AGENTS.md` and `CLAUDE.md` to current rules; archived the
former architecture and idea documents; moved reports under `docs/reports/`;
and marked superseded tasks/ADRs as historical. The target is capability-owned
modules with explicit composition, typed cross-module contracts, and shared
UI-kit primitives.

## 2026-09-06 — Dependency gate must distinguish target and migration edges

**Finding:** `scripts/check_crate_deps.py` still encoded the v1 crate graph and
failed on four already-known transitional edges, while its success message
claimed that the old architecture was fully satisfied.

**Resolution:** Updated the checker to use the v2 migration inventory, keep the
four transitional edges explicit, reject every untracked edge/cycle, and report
that a green result means no untracked violation rather than migration
completion. The remaining edges are now removable one by one as contracts move.

## 2026-09-05 — GPUI panic: "hover style already set" when `.hover()` applied twice

**Context:** `ui-kit::ListItem` lets call sites override a row's background via
`.extra(..)`, and the host/SCM rows wanted a custom hover fill (accent-border /
border) distinct from `ListItem`'s default `selected_fill` bg-tint hover.

**Finding:** A `Stateful` element may only receive `.hover(..)` once. Calling it
inside `.extra(..)` after `ListItem` had already applied its own default hover
(`row.hover(|s| s.bg(selected_fill))` when `!selected`) panics at render time
with "hover style already set".

**Fix:** Added `ListItem::hover_style(f)` (crates/ui-kit/src/list.rs) — a builder
slot that replaces the default hover fill and is applied in `IntoElement` before
`.extra(..)`, so call sites never call `.hover(..)` twice. Migrated
`hosts-ui` (accent-border hover) and `panel-scm` file rows (border hover, accent
selected) onto it.

**Reference:** confirmed via the panic message at runtime; the constraint is
visible in the Zed `gpui` element state (one `hover` slot per element).

## 2026-09-05 — Zed UI source cannot be copied into the Apache-2.0 project

**Context:** A source-level comparison was made for Zed's dock/status bar,
Project Panel, Git Panel, and shared UI primitives.

**Finding:** The inspected Zed crates `workspace`, `project_panel`, `git_ui`,
and `ui` each declare `GPL-3.0-or-later`. Labonair's root license is
Apache-2.0.

**Resolution:** Treat Zed as a behavioral and architectural reference. Specify
observable interaction outcomes and independently implement them over
Labonair's existing panel, dock, workspace, and UI-kit APIs. Do not copy or
closely translate Zed function bodies, type layouts, comments, or algorithms
unless the project first makes an explicit, reviewed licensing decision.

**Reference:** `docs/ui-comparison-zed-sidebar-status-bar.md` section 2.
## 2026-09-06 — Transfer capability extraction and wire compatibility

**Decision:** Transfer lifecycle state and the queue UI now belong to the
`transfers` module. `labonair-transfers` is UI-free and receives legacy worker
events only through the backend adapter; `labonair-transfers-ui` owns the
statusbar queue. Workspace owns only the typed enqueue request and the
tab-local SFTP refresh callback. Transfer history remains in memory until a
separate persistence decision; retry remains a separate typed worker feature.

**Bug found:** The first typed `TransferStatus` enum omitted the existing
`serde(rename_all = "snake_case")` contract. The decoder therefore rejected
worker payloads such as `"status": "running"` even though compilation passed.

**Fix:** Restored the wire-format attribute in `crates/transfers/src/lib.rs`
and added a decoder regression test. The focused transfer tests and full
workspace gates now pass.
# 2026-09-06 — Capability extraction must move persistence with the UI state

Moving `BackgroundStore` out of `labonair-workspace` while leaving
`backend::modules::backgrounds` in place would have preserved a split owner:
GPUI state in one feature crate and persistence in the broad backend facade.
The resolution was to move both the GPUI store and the background persistence
module into `labonair-background`, replacing the backend path with the
filesystem platform service. `settings-ui` now has no workspace dependency;
the remaining workspace-to-background edge is only app composition and is
explicitly tracked for a later synchronization/standalone-surface task.

## 2026-09-06 — Notification statusbar presentation belongs to Notifications

**Finding:** The retained notification registry lived in the Notifications
capability, but the global statusbar item and dropdown were still implemented
in `shell/src/status_items.rs`. This made the shell the accidental owner of
notification rendering and allowed the capability boundary to drift.

**Resolution:** Moved `NotificationsStatusItem` into
`crates/notifications/src/status_item.rs`. The item owns the statusbar bell,
scrollable dropdown, expansion/read state, clear-all behavior, and retained
action affordances. The shell now only constructs and registers the public
capability item.

**Non-obvious constraint:** The panel `StatusItem` contract is intentionally
UI-agnostic, so the notifications UI may depend on `labonair-panel`,
`labonair-theme`, and `labonair-ui-kit`; those shared contracts must not depend
back on Notifications. The existing callback-backed action adapter remains
temporary until producers publish stable command IDs through the command
registry.

## 2026-09-06 — Operation errors must have one retained surface

**Finding:** Several active views kept raw operation errors in banners, rows,
or connection cards even after the statusbar notification registry became the
canonical message surface. This caused duplicate messages and inconsistent
error details.

**Resolution:** Migrated Explorer, SFTP, Preview, Git Graph, SCM, Hosts, SSH
connection setup, editor operations, and Project Diff to structured
notifications with neutral summaries, expandable details, source IDs, and
deduplication keys. Internal error values remain only for retry/control flow.
The transfer conflict/file-error modal, SFTP permission validation, and editor
external-change prompt are explicitly retained because they require an
immediate user decision; passive list/card error text was removed.

**Verification note:** The AI HTTP end-to-end test and Settings file-watcher
test require OS listener/file-event permissions. They failed inside the
sandbox with `Operation not permitted` / a missing rename event and passed
when rerun individually with those permissions enabled.

## 2026-09-07 — Avoid eager indexing when deriving optional contexts

**Bug found:** The shell command adapter used
`(contexts.len() == 1).then_some(contexts[0])`. `bool::then_some` evaluates
its argument eagerly, so global commands with an empty context slice panicked
even though the condition was false.

**Fix:** Use `first().copied().filter(...)` so an empty context list remains a
valid global command. The command-provider equality tests caught the issue
before the change was accepted.

## 2026-09-07 — Register identity dependencies in the boundary verifier

**Bug found:** Moving the editor's `SearchFocus` identity into its owner
provider added a direct dependency on `labonair-interaction-contracts`, but the
dependency verifier still treated the editor as a one-edge engine crate.

**Fix:** Added the intentional editor-to-interaction-contracts edge to
`scripts/check_crate_deps.py`. The graph remains acyclic and the editor stays
UI-free.

## 2026-09-07 — Preserve defaults when privatizing legacy editor migration data

**Bug found:** Moving the historical `EditorPrefs` wire type into the private
Settings migrator initially used derived defaults. That changed `hlsearch`,
`incsearch`, and `smartcase` from their legacy `true` defaults to `false`, so
the sparsifier emitted false values as user overrides and two migration tests
failed.

**Fix:** Restored the exact legacy `Default` implementation inside
`migrate_v2`. The standalone editor adapter was then deleted while the wire
shape remains available only to the one-time migration.

## 2026-09-07 — Normalize injected terminal receiver errors

**Bug found:** The MCP command-capture loop combines SSH broadcast receivers
and the injected local-terminal receiver in one async branch. SSH returned a
`broadcast::RecvError` while the capability contract returned `String`, so
the branches had incompatible result types.

**Fix:** Convert the SSH receiver error to `String` at the MCP adapter
boundary. The command loop now consumes both transports through the same
`Result<Vec<u8>, String>` shape without knowing either receiver
implementation.

## 2026-09-07 — Add direct SQLite dependency to MCP server integration

**Bug found:** After moving the concrete MCP server from `labonair-backend`,
the new `labonair-mcp-server` crate failed to compile because its state and
server code still use `rusqlite::params!` directly.

**Fix:** Declare `rusqlite` in the MCP integration crate. The backend no
longer needs that dependency after the MCP module extraction.

## 2026-09-07 — Pass all owner entities when composing commands

**Bug found:** Extending `compose_builtin_commands` with an optional Hosts
entity left the no-context test helper calling it with only the existing
workspace and updater arguments. Rust reported a missing third argument.

**Fix:** Pass `None` for Hosts in the metadata-only composition path. The
runtime bootstrap path supplies the real `HostManagerView` entity.

## 2026-09-07 — Name the injected keymap raw-file callback type

**Bug found:** Clippy rejected the new Keymap-UI command registration because
the `Rc<dyn Fn(&mut Window, &mut App)>` parameter triggered
`clippy::type_complexity` under the repository's `-D warnings` policy.

**Fix:** Introduced the public `OpenRawHandler` type alias in the Keymap-UI
command provider and used it at the registration boundary.

## 2026-09-07 — Update the metadata-only command registry test

**Bug found:** After moving Workspace-owned command handlers out of the shell
table, the shell test still expected `run_for` to return handlers for
`OpenProject` and `ReturnToStandalone`. The test helper intentionally builds
metadata without a live Workspace entity, so that expectation described the
removed compatibility path.

**Fix:** Assert that the metadata-only registry has no shell fallback for
those commands. Runtime bootstrap supplies the owner handlers through the
Workspace entity registration.

## 2026-09-07 — Pass the new Hosts picker callback through composition

**Build failure:** After moving the connection command registrations into
Hosts-UI, the metadata-only shell composition helper still supplied the old
number of optional owner callbacks. Rust reported the missing final argument
for the Hosts picker handler.

**Fix:** Added the explicit `None` to the metadata-only path and passed the
typed Hosts picker callback from bootstrap in the runtime path. The callback
only opens the existing canonical palette page; host selection and transport
intent remain owned by Hosts-UI and its typed picker snapshots.

## 2026-09-07 — Import `AppContext` for owner status registration

**Build failure:** Moving notification status-item construction into the
Notifications crate caused `App::new` to be unavailable at the new
registration boundary, because the extension trait was not imported there.

**Fix:** Imported GPUI's `AppContext` in the Notifications status-item module.
The shell no longer needs that construction helper or the corresponding
status-item trait imports.

## 2026-09-07 — Native screenshot capture denied by macOS

**Observation:** The native Rust executable started successfully and its
window was found by PID, but `screencapture` returned a permission denial.

**Disposition:** Keep the R07-001 visual matrix pending and record the exact
process/window evidence. A future visual acceptance run needs Screen Recording
permission for the runner; no screenshot of another application may be used.

## 2026-09-07 — Screenshot validator rejected `cargo run` process path

**Bug:** `scripts/screenshot.sh` compared the absolute repository binary path
only. `cargo run -p labonair` exposed the same native executable as the
relative command `target/debug/labonair`, so the validator rejected the
correct PID before it could inspect the window.

**Fix:** Normalize the executable token from `ps` against the repository root
before comparing it with the approved debug and packaged Rust binary paths.
The retry confirmed the correct native window and reached `screencapture`; the
remaining failure is the genuine macOS Screen Recording permission denial.
