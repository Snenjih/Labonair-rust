# Handshake — Session State (Labonair-rust Port)

Authored by: GPUI-native port of Labonair (formerly Tauri v2 + React 19 → now pure Rust/GPUI).

> This file is the authoritative continuity doc for the **port** project. This is a **hard fork** — fully standalone, no link/symlink/submodule to any external Labonair repo. The old web-app source is a frozen read-only copy at `reference-src/` inside this repo and is the only reference. Do not mistake the old git history/tech for the current target.

## Last Session: 2026-09-01 (T07-003 — SSH config import/export)

### What Was Done
- **T07-003 ✅ Done.** Backend parse/import/export was already ported in
  `crates/backend/src/modules/ssh/config_parser.rs`; this session hardened it
  and built the UI.
  - **`config_parser.rs`** — parser now recognises `Match` (suppresses key
    capture until the next top-level `Host`) and `Include` (read, not
    followed). New `ImportConflict { Skip, Overwrite, Rename }` enum;
    `import_ssh_config_entries` gained a `conflict: ImportConflict` param and
    now snapshots existing host names → resolves alias collisions per policy
    (Skip = leave + still usable as a ProxyJump target, Overwrite = UPDATE the
    mapped fields in place, Rename = insert as `alias-2`/`-3`/…). Returns the
    ids created **or** overwritten. New `write_ssh_config_export(block, append)`
    — atomic temp+rename write to `~/.ssh/config`, chmod 0600 on unix, `append`
    inserts after existing content; caller gates the non-append path. +4 tests
    (representative parse incl. wildcard/Match/Include/`=`-form/ProxyJump,
    import+mapping+ProxyJump resolution, Skip/Overwrite/Rename, export
    well-formedness + Import→Export→Import round-trip). backend 142→146.
  - **`crates/ui/src/hosts.rs`** — `ImportState`/`ExportState` on
    `HostManagerView`, free fns `cycle_conflict`/`conflict_label`. Toolbar
    gains "Import SSH config" + "Export SSH config". Import modal: async
    `parse_ssh_config_cmd`, per-entry checkbox rows (alias, addr:port, user,
    auth, `via <jump>`, "already exists" marker), select/deselect all, a
    cycling "On conflict: skip/overwrite/rename" button, `N of M selected`
    footer, runs `import_ssh_config_entries` then toast + `reload`. Export
    modal: host checkbox list (all pre-selected), "Copy to clipboard"
    (`cx.write_to_clipboard`) and "Append to ~/.ssh/config"
    (`write_ssh_config_export(_, true)`). New `modal_shell` helper. +2 tests
    (conflict cycle, export preselects all hosts). ui 67→69.
  - Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
    -- -D warnings`, `cargo check --workspace`, `cargo test --workspace` all
    green.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T08-001** — SFTP-Dateibrowser (`tasks/phase-07-sftp/`). Deps T07-001,
  T04-001/2, T05-001 (all done).

### Notes / Quirks (T07-003)
- Conflict policy is **global** for the import batch (one toggle), not
  per-row — matches the task's overwrite/skip/rename wording without a
  per-entry UI. The reference dialog only *warns* on duplicates (by
  `address:port`); this port keys conflicts by **alias/name** and actually
  acts on them.
- Rename collision suffix is `-2`, `-3`, … (there is no UNIQUE constraint on
  `hosts.name`; rename is best-effort cosmetic dedup).
- Overwrite updates only the mapped fields (host_address, port, username,
  auth_method, private_key_path) — never touches group, credential, tags,
  tunnels of the existing row.
- `write_ssh_config_export` is only ever called with `append = true` from the
  UI (the task warns against silent overwrite). The `append = false` branch
  exists but is unused — wire it to an explicit "replace file" action if ever
  needed.
- No test writes to the real `~/.ssh/config`; the file writer is covered by
  the export-text generator + round-trip parse instead.
- IdentityFile → stored as a path on the host (`auth_method = "key"`,
  `private_key_path`), never imported as a credential — matches the task note
  ("kein Klartext von Key-Inhalten").

---

## Previous Session: 2026-09-01 (T07-002 — Jump hosts & tunnels)

### What Was Done
- **T07-002 ✅ Done.** Backend jump-host routing + local-forward tunnels
  already existed (`ssh/client.rs::resolve_jump_host` / `connect_via_jump`,
  `ssh/tunnels.rs`); this task was the UI + wiring.
  - **`crates/backend/src/modules/ssh/tunnels.rs`** — `TunnelConfig` now
    derives `Serialize` + `PartialEq`, `type` defaults to `"local"` on parse
    (round-trips unchanged); dropped the `#[allow(dead_code)]` on `id`/
    `tunnel_type`. `TunnelEntry` gained `configs: Vec<TunnelConfig>`. New
    `pub fn active_tunnels(&TunnelState) -> Vec<ActiveTunnel>` (sorted by
    `(host_id, local_port)`) for the UI panel. +4 tests (JSON shape round-trip,
    `active_tunnels` listing/sort, ref-count-gated shutdown, unknown-host
    no-op). **Only `type:"local"` forwarding is supported — matches the
    reference (`reference-src/.../hosts/types.ts` `type: "local"` only); the
    task's remote/dynamic mention is beyond reference parity and was not
    implemented.**
  - **`crates/ui/src/hosts.rs`** — `HostForm` gains `jump_host: Option<usize>`
    (index into `self.hosts`, never the host being edited) and
    `tunnels: Vec<TunnelDraft>` + a `scratch` fallback string. New
    `HostField::Tunnel{LocalPort,RemoteHost,RemotePort}(usize)` inline-edit
    targets. Free fns `parse_tunnels` / `serialize_tunnels` (drops incomplete
    rows, `type:"local"`). Form now renders a "Jump host (ProxyJump)" cycle
    button + a "Tunnels" section (add / edit 3 fields / remove per row).
    `submit_form` passes `Some(jump_host_id_or_"")` + `Some(tunnels_json)` to
    `hosts_create`/`hosts_update` (`""` clears the jump host). New pub
    `host_name()` / `jump_host_label()` accessors, `ActiveTunnelRow` type +
    `set_active_tunnels()` + an "Active tunnels" panel. `from_host` signature
    gained a `hosts: &[Host]` param. +3 tests.
  - **`crates/ui/src/workspace.rs`** — `ssh_tab_title(host, jump)` helper
    (SSH tab title shows `SSH · host  ⤳ bastion` when routed through a jump
    host). `connect_host` reads the jump label from the host manager for the
    title. On `SshSessionEstablished` → `start_tunnels()` (spawns
    `ssh_start_tunnels`, mirrors the reference's `session_established` hook);
    `retire_tab` SSH teardown also calls `ssh_stop_tunnels`. The 40ms SSH
    poll loop now calls `refresh_active_tunnels()` → pushes `ActiveTunnelRow`s
    into the host manager (deduped, only notifies on change). +1 test.
  - Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
    -- -D warnings`, `cargo check --workspace`, `cargo test --workspace` all
    green. Tests: backend 138→142, ui 63→67.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T07-003** — SSH config import/export (`tasks/phase-06-ssh-ui/
  T07-003-ssh-config-import-export.md`). Dep: T07-001 (done). Backend
  `ssh/config_parser.rs` already exists.

### Notes / Quirks (T07-002)
- **No live jump/tunnel integration test** — same rationale as T07-001 (no
  `sshd` in CI). Routing/tunnel logic is unit-tested at the seams
  (resolution, ref-counting, JSON round-trip, title annotation).
- Tunnel type is **local-forward only** end to end. The `type` field is kept
  in the JSON purely for round-trip fidelity with the reference.
- No standalone "stop this one tunnel" control: tunnels are host-config and
  auto start/stop with the SSH session (backend ref-counts per host). The
  "Active tunnels" panel is read-only status, matching the reference (which
  has no tunnel-management UI at all).
- Multi-hop jump chains are still unsupported (a jump host's own
  `jump_host_id` is never followed — documented in `resolve_jump_host`). The
  form only lets you pick one bastion.
- `HostForm::field_mut` returns `&mut self.scratch` for a stale tunnel-field
  index instead of panicking (indices only ever come from `render`, but the
  guard keeps it total).

---

## Previous Session: 2026-09-01 (T07-001 — Host manager & SSH connection)

### What Was Done
- **T07-001 ✅ Done.** First UI↔backend-wired feature. Host-manager dashboard,
  SSH connect flow, SSH terminal tabs, credential manager.
  - **`crates/terminal/src/session.rs`** — new `SessionAccess` trait (render /
    cwd / metadata / mode_state / selection / scroll / ai_context) impl'd for
    both `TerminalSession` and the new **`RemoteSession`** (transport-backed
    terminal: same `TerminalEmulator`, bytes come from SSH not a local PTY).
    `RemoteSession::new(colors, dims, RemoteWriter, RemoteResizer) -> (Self,
    RemoteFeed)`. `RemoteFeed { feed(&[u8]), mark_disconnected() }` — the SSH
    reader pushes output through `feed`; DA/DSR replies go back out via the
    writer. `RemoteWriter/RemoteResizer = Arc<dyn Fn(..) + Send + Sync>`.
  - **`crates/terminal/src/registry.rs`** — `Slot.session` is now
    `Mutex<SessionBackend>` (`Local(TerminalSession) | Remote(RemoteSession)`);
    all `SessionHandle` methods dispatch. `SessionHandle::with` now hands out
    `&dyn SessionAccess`. New `TerminalRegistry::create_remote(...) ->
    (SessionId, RemoteFeed)`. `restart` errors on remote sessions. +1 test.
  - **`crates/ui/src/hosts.rs`** (new, ~780 lines) — `HostManagerView`
    (`TabKind::Home` content). Groups (ungrouped + named, collapsible,
    create/delete), host rows with live status dot + Connect / Edit /
    Duplicate / Delete, host add/edit form modal (name/address/port/user/
    auth-method[password|key|agent|none]/key-path/password/start-dir/tags +
    credential & group cycle pickers), credential manager modal (list, new
    password/key credential, ed25519 keygen via backend, delete; public key
    shown in a toast). All persistence via `labonair_backend::modules::{hosts,
    credentials}`. Inline text fields use the explorer's focus+key-buffer
    pattern. Emits `HostManagerEvent::Connect(host_id)`. +2 tests.
  - **`crates/ui/src/workspace.rs`** — owns `backend: labonair_backend::App`,
    `tokio: runtime::Handle`, `host_manager`, `ssh_tabs:
    HashMap<SessionId, SshTab>`. Landing tab is now `TabKind::Home` (then a
    terminal tab, still active on start). `connect_host()` builds writer/
    resizer closures that `tokio.spawn` `ssh_pty_write` / `ssh_pty_resize`,
    calls `registry.create_remote`, opens a Workspace tab + `TerminalView`,
    then `spawn_ssh_connect` (calls `ssh_connect`, streams `SshPtyEvent::Data`
    into the `RemoteFeed`). Backend broadcast bus → `std::sync::mpsc` →
    `cx.spawn` 40ms poll → `handle_ssh_event`: `known_hosts_warning` /
    `auth_required` / `passphrase_required` raise an `SshPrompt` modal
    (trust / password / passphrase); `session_established` → status Connected
    + clears prompt; `ssh_connection_lost` → `feed.mark_disconnected()` +
    status Failed. Trust accept → `ssh_trust_host(id, true)`; password/
    passphrase submit → re-run `ssh_connect` with the override (the backend's
    fail-then-re-prompt model). `retire_tab` disconnects the SSH session.
    `pending_connect` drained in `render` (needs `&mut Window`).
  - **`crates/ui/src/app_shell.rs`, `crates/app/src/main.rs`** — thread
    `backend` + `runtime.handle().clone()` (captured before the runtime is
    `mem::forget`-leaked) through `AppShell::new` → `Workspace::new`.
  - **`crates/backend/src/modules/ssh/client.rs`** — +3 tests (trust-host
    oneshot release, unknown session no-panic, connect-to-missing-host errors).
  - Cargo: `crates/ui` gains `tokio` + `uuid` deps.
  - Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
    -- -D warnings`, `cargo check --workspace`, `cargo test --workspace` all
    green. Tests: backend 135→138, terminal 62→63, ui 61→63.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T07-002** — Jump-Hosts & Tunnel (`tasks/phase-06-ssh-ui/T07-002-*`). Dep:
  T07-001 (done). Backend `ssh/tunnels.rs` + jump-host resolution in
  `ssh_connect` already exist; this is largely UI (host-form jump-host picker,
  tunnel editor) + wiring.

### Notes / Quirks (T07-001)
- **No live SSH-server integration test.** Setting up `sshd` in CI is
  platform-fragile; connection-flow error handling + the trust mechanism are
  unit-tested instead. A real end-to-end connect test against a throwaway
  `sshd` is a good follow-up (task lists it under Notizen).
- The UI has **no tokio runtime on the GPUI thread** — `main.rs` leaks the
  runtime and passes a `Handle`. All backend async calls go through
  `tokio.spawn(...)` returning a `JoinHandle`, then `cx.spawn` awaits that
  handle (polling a `JoinHandle` needs no runtime context). This is the
  pattern every future UI↔backend feature should follow.
- `hosts_update` skips a field when its `Option` arg is `None`; there is no
  clean "clear the group" path (passing `Some("")` would leave an orphan FK).
  The form only *sets* a group; credential clearing works (`Some("")`).
- Auth method "agent"/"none" are stored but the backend's agent path only
  triggers under `auth_method == "key"` with a key file — deeper agent-only
  semantics are a backend concern, out of scope here.
- SSH prompt modal is focused from `render` via a `prompt_shown` transition
  guard (the event handler has no `&mut Window`).

---

## Previous Session: 2026-09-01 (T06-004 — Diff view)

### What Was Done
- **T06-004 ✅ Done.** Reusable line-diff core + GPUI diff pane.
  - **`crates/editor/src/diff.rs`** (new, ~530 lines incl. 13 tests) —
    line-based **Myers** O(ND) diff (`myers`, trace + backtrack). Public API:
    `Diff::compute(old, new)` / `compute_with_context(old, new, ctx)` →
    `Diff { lines: Vec<DiffLine>, hunks: Vec<Hunk> }`. `DiffLine { tag:
    ChangeTag (Equal/Delete/Insert), old_line/new_line: Option<usize> (1-based),
    text }`. `Hunk { old_start/old_len/new_start/new_len, lines }` +
    `header()` (`@@ -a,b +c,d @@`) + `change_counts()`. Hunks group changed
    lines whose 3-line context windows touch (`gap > 2*ctx+1` splits).
    `Diff::is_unchanged()` / `stats() -> (ins, del)`. `side_by_side(&Hunk) ->
    Vec<SideRow>` pairs delete-run/insert-run index-wise into
    `RowKind::{Context,Delete,Insert,Replace}` rows with `SideCell { line,
    text }`. Deliberately NOT the textual git-diff format (warning #2 in the
    task) — plain manipulation-friendly line list.
  - **`crates/editor/src/lib.rs`** — `pub mod diff` + re-exports.
  - **`crates/ui/src/diff.rs`** (new, ~430 lines incl. 4 gpui tests) —
    `DiffView` GPUI component. `set_content(old, new, title, cx)` recomputes;
    `DiffLayout::{Unified, Split}` toggle (`toggle_layout` / button / `s` key);
    hunk navigation `next_hunk`/`prev_hunk` (clamped, `j`/`k` / arrows /
    header ↑↓ buttons), active hunk gets a left border in `modified` colour.
    Unified: dual line-number gutter + `+`/`-` sign, row tint from theme
    `success`/`error`. Split: two columns w/ divider, `Replace` rows tinted
    `modified`. Hunk header row uses `info`. Ellipsis divider row for
    skipped context. `on_stage_hunk(Fn(usize, &mut Window, &mut App))` hook
    (type alias `StageHunkFn`) prepared for Phase 8 hunk staging — no
    staging logic here.
  - **`crates/ui/src/theme.rs`** — added `ThemeStore::status_modified()`
    accessor (`status.modified`).
  - **`crates/ui/src/lib.rs`** — `pub mod diff` + `pub use diff::{DiffLayout,
    DiffView}`.
  - Tests: editor 60 (+13), ui 61 (+4). `cargo fmt --all --check`, `cargo
    clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
    all green.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T06-005** (or next lowest-numbered pending in `tasks/phase-05-editor/`;
  T06-001..004 all Done) — check `tasks/ROADMAP.md`.

### Notes / Quirks (T06-004)
- Diff body renders every hunk row directly (no virtualization). Hunks
  already exclude unchanged gaps so it's bounded by change size; the task's
  "consider windowed rendering for huge diffs" note is deferred — wire
  `uniform_list` if a real perf problem shows up with Git/AI diffs.
- The ellipsis divider is informational only (not click-to-expand). Full
  context is available in `Diff::lines` if an expand affordance is wanted
  later.
- `side_by_side` pairs a delete-run with the *immediately following*
  insert-run; a hunk with interleaved equal lines between them produces
  separate single-sided rows, which is correct.
- No tab/AI/Git wiring yet — `DiffView` is standalone and exported; Phase 8
  and Phase 10 call `set_content` + `on_stage_hunk`.

---

## Previous Session: 2026-09-01 (T06-003 — Vim mode)

### What Was Done
- **T06-003 ✅ Done.** Self-contained modal Vim layer over `Document` (no
  external vim crate — the ones that exist are bound to their own buffer
  types; approach documented in the module header).
  - **`crates/editor/src/vim.rs`** (new, ~950 lines incl. 20 tests) —
    `Vim` state machine: `VimMode` (Normal/Insert/Visual/VisualLine/Command),
    `VimOptions` (number/relativenumber/hlsearch/incsearch/smartcase/expandtab/
    tabstop/shiftwidth), `VimKey` input alphabet, `VimResponse { handled, save,
    quit, reload }`. `on_key(&mut Document, VimKey) -> VimResponse`.
    - Motions: h/j/k/l, w/W/b/B/e/E, ge, 0/^/$/gg/G, `{`/`}`, `%` (bracket
      match), f/F/t/T + `;`/`,`, all with counts; combine with operators
      (`d3w`, `2dd`, `dj`, `dt.`).
    - Operators d/c/y + doubled (dd/cc/yy) + D/C/Y; `cw`→`ce` quirk.
    - Edits: x/X (count), r, ~, J, s/S, i/I/a/A/o/O, p/P (charwise +
      linewise register), u, Ctrl-R.
    - Visual + visual-line: motions extend, d/x/c/y/p operate on selection
      (charwise via `doc.anchor`, linewise via a tracked anchor line).
    - Ex line: `:w :q :wq :x :e :noh`, `:s/a/b/[g]` + `:%s/…`, `:set
      [no]opt` / `:set opt=N`. `/` `?` search + `n`/`N` wired to
      `crate::search` (smartcase-aware); `hlsearch` matches exposed on
      `Vim::search_matches`.
    - Undo units: every operator goes through `range_delete` which does
      `set_caret`×2 + `backspace` — `set_caret` breaks history coalescing so
      each command is its own undo step; insert-mode typing stays one step.
  - **`crates/editor/src/lib.rs`** — `pub mod vim` + re-exports (`Vim`,
    `VimKey`, `VimMode`, `VimOptions`, `VimResponse`).
  - **`crates/backend/src/modules/settings/editor.rs`** (new) — `EditorPrefs`
    (vim_mode + the 8 vim options) persisted as the `editor` object inside the
    shared `labonair-settings.json`; `editor_prefs_load()` /
    `editor_prefs_save()` (merge-preserving, atomic tmp+rename). `pub mod
    editor;` added to `settings/mod.rs`. 2 tests.
  - **`crates/ui/src/editor.rs`** — `EditorView.vim: Option<Vim>` built in
    `new()` from `editor_prefs_load()`. `on_key`: after the Cmd/Alt shortcut
    blocks, `vim_key(ks)` translates the keystroke (arrows→hjkl outside insert,
    Ctrl-R→Redo, printable→Char) and `handle_vim()` runs it, then
    bump_syntax/ensure_visible/refresh_matches/emit + acts on save/reload/quit.
    New `EditorEvent::CloseRequested` (from `:q`/`:wq`). Bottom status line
    (`render_vim_status`) shows the mode / live `:`-`/` command line + caret
    pos. Gutter honours `number` / `relativenumber` when vim is on.
  - **`crates/ui/src/workspace.rs`** — `watch_editor` handles
    `EditorEvent::CloseRequested` → `tabs.close(tab_id)` + `retire_tab`.
  - Tests: editor 29→49, backend +2, ui 56→57. `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets -D warnings`, `cargo test
    --workspace`, `cargo check --workspace` all green.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T06-004** — Diff view (`tasks/phase-05-editor/T06-004-diff-view.md`).

### Notes / Quirks (T06-003)
- No Phase 12 settings UI yet: vim mode is toggled only via the persisted
  `editor.vimMode` key in `labonair-settings.json` (read once at
  `EditorView::new`) and at runtime via `:set` for the session. Wire the
  toggle into the Phase 12 Editor settings section when it lands, and call
  `editor_prefs_save` there.
- `r` / `~` currently produce **two** undo steps (delete + insert) because
  `Document` has no in-place replace; acceptable but could be tightened later.
- `:q` close path skips `focus_active` (subscribe closure has no `&Window`),
  so focus isn't moved to the next tab's view after `:q`. Minor.
- hlsearch matches are collected on `Vim::search_matches` but not yet painted
  by the editor renderer — search still moves the caret to matches, which is
  the required integration. Painting them is a small follow-up.
- `Document` caret/selection fields (`cursor`, `anchor`) are `pub`, which the
  vim layer relies on for visual mode + range math.

---

## Previous Session: 2026-09-01 (T06-002 — syntax highlighting & language detection)

### What Was Done
- **T06-002 ✅ Done.** Tree-sitter syntax highlighting + language detection +
  editor colour schemes. No web tech; native `tree-sitter` / `tree-sitter-highlight`.
  - **`crates/editor/Cargo.toml`** — added `tree-sitter 0.25`, `tree-sitter-highlight
    0.25`, and 14 grammar crates: rust, json, python, javascript, typescript,
    go, c, cpp, css, html, bash, toml-ng, yaml, java.
  - **`crates/editor/src/language.rs`** — `Language` gains `Hash`; new variants
    Java/Php/Xml/Ruby/Swift/Kotlin; more filename rules (Containerfile,
    dotfiles, `.dev`), more extensions (java/php/xml/svg/rb/swift/kt/…);
    `Language::has_grammar()`. +2 test assertions blocks.
  - **`crates/editor/src/syntax.rs`** (new) — `SyntaxHighlighter`:
    - `HighlightKind` (20 coarse token classes) + `HIGHLIGHT_NAMES` capture list
      + `capture_kind()` dotted-name → kind mapping.
    - `config(Language) -> Option<&'static HighlightConfiguration>`: lazy per
      language, built on first use, `Box::leak`-ed into a `OnceLock<Mutex<HashMap>>`.
      `build_config` wires each grammar's `HIGHLIGHT(S)_QUERY` (JS uses
      `HIGHLIGHT_QUERY`; TS/C++ concat the JS/C base query; note const-name
      inconsistency across crates). Unsupported langs (plain/md/sql/php/xml/
      ruby/swift/kotlin) → `None` → no spans (default-fg fallback).
    - `update(text, revision, visible_byte_range)`: re-parses only when the
      revision changed or the cached `covered` window no longer contains the
      viewport; keeps spans for `visible ± 32 KiB`; **breaks out of the
      Tree-sitter event stream once past the window**; 2 MiB hard size guard.
    - `line_runs(line, line_start_byte) -> Vec<StyledRun>` splits one line into
      styled/plain runs for the renderer.
    - 7 `#[test]`s (rust/python/json snippet token checks, plain-text = no spans,
      line partition exactness, viewport skips offscreen tail, revision cache reuse).
  - **`crates/editor/src/lib.rs`** — `pub mod syntax` + re-exports.
  - **`crates/ui/src/theme.rs`** — `EditorThemeId` (Auto + 9 named: atomone,
    aura, copilot, github-dark/light, nord, tokyo-night, xcode-dark/light —
    mirrors `reference-src/.../editor/lib/themes.ts`) with `slug`/`from_slug`/
    `ALL`; `ThemeStore.editor_theme` field + `editor_theme()` / `set_editor_theme()`.
  - **`crates/ui/src/syntax_theme.rs`** (new) — `EditorPalette` (one `Hsla` per
    `HighlightKind`). `resolve(id, &ThemeStore)`: `Auto` derives colours from the
    app-theme tokens (primary/accent/status_*/muted_foreground/foreground) so it
    follows light/dark + imported themes; named schemes use fixed `Roles` hex
    palettes. 3 tests (auto tracks app mode, named stable + app-independent,
    every kind resolves).
  - **`crates/ui/src/editor.rs`** — `EditorView` gains `syntax: SyntaxHighlighter`
    + `syntax_rev` (bumped in `edit`/`after_edit`; `resync_syntax()` on load/
    reload sets language + invalidates). `render()`: builds `doc_text` +
    per-visible-line byte offsets, calls `syntax.update(..)` for the visible
    range, resolves `EditorPalette` from `theme.editor_theme()`, and renders
    each line via `gpui::StyledText::with_highlights(Vec<(Range, HighlightStyle)>)`
    (falls back to a plain `div` child when a line has no spans). Theme changes
    already repaint via the existing `cx.observe(&theme)`.
  - **`crates/ui/src/lib.rs`** — `pub mod syntax_theme` + re-exports
    (`EditorPalette`, `EditorThemeId`).
  - Tests: editor 21→29, ui 53→56. `cargo fmt --all --check`, `cargo clippy
    --workspace --all-targets -D warnings`, `cargo test --workspace`, `cargo
    build --bin labonair` all green.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T06-003** — Vim mode (`tasks/phase-05-editor/T06-003-vim-mode.md`). Dep: T06-001.

### Notes / Quirks
- Grammar crates export their highlights query under **inconsistent names**:
  `HIGHLIGHTS_QUERY` (rust/json/python/ts/go/css/html/java/toml-ng/yaml) vs
  `HIGHLIGHT_QUERY` (javascript/c/cpp/bash). Check per crate.
- `tree_sitter_highlight::Language` is a **private** re-export — use
  `tree_sitter::Language` (`tree-sitter` is a direct dep). `LanguageFn` →
  `Language` via `.into()`.
- TS grammar exposes `LANGUAGE_TYPESCRIPT` / `LANGUAGE_TSX` (not `LANGUAGE`);
  its highlights query must be concatenated **after** the JS one.
- `HighlightConfiguration::new` in 0.25 takes `(Language, name: impl Into<String>,
  highlights, injections, locals)` — the `name` param was added in 0.23.
- `tree-sitter-highlight` has no byte-range restriction API; viewport bounding is
  done by breaking the event iterator once `Source.start >= window_end`.
- `StyledText::with_highlights` uses the enclosing element's text style as the
  default run style (delayed); ranges must be char-boundary + sorted + non-overlapping.
- `tree-sitter-md` / markdown grammar not wired (block/inline split API, odd
  const names) — Markdown detected but not highlighted. Same for SQL/PHP/XML/
  Ruby/Swift/Kotlin (no grammar crate added). Easy to add later — one arm in
  `build_config` + one dep.

---

## Prior Session: 2026-09-01 (T06-001 — editor foundation & file open/save)

### What Was Done
- **T06-001 ✅ Done.** Native code-editor foundation. No CodeMirror / web tech;
  a framework-free editing model in the `labonair-editor` crate plus a GPUI
  view in `labonair-ui`.
  - **`crates/editor/`** (was an empty stub) — new modules:
    - `buffer.rs` — `TextBuffer` (`Vec<String>` lines, char-indexed
      `Position`), `insert`/`delete` handle multi-line + Unicode (columns are
      char indices, not bytes), round-trips trailing newline.
    - `history.rs` — `History` undo/redo of full snapshots, coalesces
      same-kind edits within 600ms; `EditKind::Barrier` (paste/reload/replace-
      all) never coalesces; caret moves call `break_coalescing`.
    - `search.rs` — literal (non-regex) `find_all` / `next_match` (wrapping) /
      `replace_all`, case-sensitive + whole-word options, single-line matches.
    - `language.rs` — `Language` enum, extension/filename detection (status
      bar + T06-002 grammar hook prep).
    - `document.rs` — `Document` = buffer + caret + selection + history +
      saved-text baseline (`is_dirty`), `disk_mtime` + `external_change`
      flag, `Motion` enum (word/line/doc/page nav with sticky goal column),
      `insert`/`backspace`/`delete_forward`/`undo`/`redo`/`replace_all`/
      `reload`/`mark_saved`.
    - Cargo.toml: dropped the unused `gpui` dep (core is framework-free).
  - **`crates/backend/src/modules/fs/file.rs`** — added blocking sync fns for
    the editor (run on `cx.background_executor().spawn`): `load_editor_file_sync`
    (`EditorLoad::{Text{content,mtime},Binary,TooLarge}`, 10 MB limit, null-byte
    + strict-UTF-8 sniff — same rules as `fs_read_file`), `save_editor_file_sync`
    (atomic temp+rename, returns new mtime), `file_mtime_sync`. Existing async
    fns untouched.
  - **`crates/ui/src/editor.rs`** (new) — `EditorView` GPUI entity:
    viewport-based line rendering (only visible lines painted) with a
    line-number gutter, caret + selection overlays, current-line highlight;
    keyboard editing via `on_key_down` (printable chars, Enter, Tab→4 spaces,
    Backspace/Delete, arrows/Home/End/PageUp-Down, Alt+arrows = word,
    Cmd+arrows = line/doc, Shift extends selection); Cmd+S save, Cmd+Z/Cmd+Shift+Z
    undo/redo, Cmd+A select-all, Cmd+C/X/V clipboard, Cmd+F find bar.
    Find bar: query + replace fields (Tab toggles), Enter/Shift-Enter next/prev,
    Cmd+Enter replace-all, Cmd+C/Cmd+W toggle case/whole-word, Esc closes.
    External-change: `check_external` re-stats on tab activation — auto-reloads
    if clean, else shows a conflict banner (Reload / Keep mine). Emits
    `EditorEvent::{Changed,Edited}`. All file IO on the background executor.
  - **`crates/ui/src/workspace.rs`** — `editors: HashMap<u64, Entity<EditorView>>`;
    rewrote `open_file(path, peek, window, cx)` with full **peek-tab** semantics
    (single click = peek/reuse the peek tab, double-click or already-open = 
    permanent, first edit un-peeks via `EditorEvent::Edited`); `new_editor_tab`
    (Cmd+E), `save_active` (Cmd+S), `find_in_active_editor` (routes Cmd+F);
    `render_content` renders the `EditorView` for `TabKind::Editor`;
    `focus_active` + `retire_tab` + ActiveTabChanged subscription (fires
    `check_external`) handle editors.
  - **`crates/ui/src/menu.rs`** — new `Save` action, `cmd-s` binding, File ▸ Save
    menu item (binding count test 23→24).
  - **`crates/ui/src/app_shell.rs`** — `act_save`, `act_new_editor_tab` wired as
    root actions; `act_find` now tries the editor find bar first, falls back to
    the terminal/header search.
  - **`crates/ui/src/theme.rs`** — added `ThemeStore::buffer_font_size()`.
  - **`crates/ui/src/explorer.rs`** — `open_file` passes `peek = click_count < 2`.
  - Tests: editor crate 21, ui crate 53 (2 new editor-view tests). Full
    `cargo test --workspace` green; clippy `-D warnings` + `cargo fmt --check`
    clean.

### Current State
- Branch `master`, committed. Pre-existing unrelated `CLAUDE.md` working-tree
  edit deliberately left uncommitted / untouched.

### Next
- **T06-002** — syntax highlighting & language detection (Tree-sitter grammars;
  `Language` enum already in place as the hook point).

### Notes / Quirks
- gpui 0.2.2 `ScrollDelta::Lines(p)` — `p.y` is already `f32` (no `f32::from`);
  `Pixels(p)` needs `f32::from(p.y)`.
- Clippy rejects an inherent method named `from_str` (confusable with
  `FromStr::from_str`) even with `-D warnings` off-by-default lint promoted —
  used `TextBuffer::from_text`.
- `on_click` needs `gpui::StatefulInteractiveElement` in scope + an `.id(...)`
  on the div.

---

## Prior Session: 2026-09-01 (T05-002 — drag-and-drop & advanced file actions)

### What Was Done
- **T05-002 ✅ Done.** Explorer drag-and-drop + copy/cut/paste buffer + OS
  file drop, ported from `reference-src/src/modules/explorer/lib/`
  (`explorerDrag.ts`, `useInternalDrop.ts` `canDropInto`/`resolveDropTarget`,
  `useOsFileDrop.ts`, `useFileTree.movePath`) and backend
  `src-tauri/src/modules/fs/mutate.rs` (`fs_copy_into`).
  - **`crates/backend/src/modules/fs/mutate.rs`** — new blocking sync fns:
    `move_into_sync(src, dest_dir) -> Result<String,String>` (rename into a
    dir; refuses overwrite / move-onto-own-parent / move-into-self-or-
    descendant), `copy_into_sync(&[String], &str)` (extracted core of
    `fs_copy_into`, which now just wraps it in `spawn_blocking`). +2 tests
    (move roundtrip + conflict + into-self + no-op + missing-dest guards;
    copy conflict-rename `name (1).ext` + source-left-in-place + missing src).
    backend tests 131 → 133.
  - **`crates/ui/src/explorer.rs`**:
    - module-level: `pub struct DraggedPaths { paths: Vec<PathBuf> }` (drag
      payload, exported from `lib.rs`), `DragPreview` render entity (pointer
      chip), `ClipOp{Copy,Cut}` + `Clipboard{op,paths}`, `pub fn shell_quote`
      / `pub fn quote_paths` (single-quote wrap unless all-safe chars),
      `can_drop_into(src,dest)` (port of `canDropInto`).
    - `ExplorerView`: `selected: Option<PathBuf>` → `selection: Vec<PathBuf>`
      (Cmd/Shift-click toggles additive; plain click still selects+opens/
      expands). New fields `clipboard: Option<Clipboard>`, `drop_target:
      Option<PathBuf>`.
    - copy/cut/paste: `clip_set`/`clip_clear`/`is_cut`/`paste_into(dir)` +
      `action_paths(path)` (whole selection if `path` is in it, else just it).
      Paste runs `copy_into_sync` (Copy) or a loop of `move_into_sync` (Cut,
      then clears buffer) on the background executor, reloads affected dirs,
      toasts errors. Guards dest against self/descendant.
    - drag: folder + file rows get `.on_drag(DraggedPaths, |_,_,_,cx|
      cx.new(DragPreview))`. Folder rows + the root list container get
      `.on_drag_move::<DragMoveEvent<DraggedPaths>>` (sets `drop_target` for
      the accent highlight), `.on_drop::<DraggedPaths>` → `drop_move` (move,
      `can_drop_into`-filtered), `.on_drop::<ExternalPaths>` → `drop_external`
      (copy_into + success toast). List container drop = into root.
    - terminal drop: `crates/ui/src/terminal.rs` root div gets
      `.on_drop::<DraggedPaths>` → `send_input(quote_paths(&paths) + " ")`.
    - UI: clipboard banner above the list ("N items copied/cut" + Paste +
      Clear), cut rows render at `opacity(0.5)` + error color, drop-target
      folder rows render `bg(accent)`. Context menu gains Copy / Cut /
      Paste (Paste only when buffer non-empty). Explorer-level `on_key`:
      Cmd-C/X/V, Esc clears buffer then selection.
    - +3 pure tests (`can_drop_into` noop/self/descendant, `shell_quote` +
      `quote_paths`, clipboard buffer set/replace/discard). ui tests 48 → 51.
  - **`crates/ui/src/lib.rs`** — `pub use explorer::{DraggedPaths, ExplorerView}`.
- **GPUI API (gpui 0.2.2):** `ExternalPaths(pub(crate) SmallVec<[PathBuf;2]>)`
  with `.paths() -> &[PathBuf]` + blanket `Render`; `InteractiveElement::
  on_drop::<T>` pushes per-TypeId so multiple `.on_drop` with different `T`
  on one element is fine (unlike `.on_drag`, which debug-panics on a second
  call); `ClickEvent::modifiers() -> Modifiers` (`.secondary()` = Cmd on
  mac, `.shift`); `cx.new(...)` inside an `on_drag` constructor closure
  (`&mut App`) needs `use gpui::AppContext`.
- **Deviations / limits:** no name-conflict *dialog* on drop/paste — a move
  onto an existing name fails with an error toast (backend refuses overwrite),
  copy auto-renames `name (1).ext` (reference `fs_copy_into` behavior). No
  chmod/chown: `localFsProvider` in the reference has no chmod/chown (SFTP-
  only — Phase 07), and the task instructions don't mention it, so
  `ChmodChownDialog` is not ported here. `drop_target` highlight clears on
  drop/handler, not on drag-cancel-outside (stale until next notify). Range
  (shift-click) select is treated as additive-toggle, not a true range.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace` (backend 133,
  ui 51), `cargo build --bin labonair` — all green. Not visually run — user
  should `cargo run` and check: drag a file onto a folder → it moves (folder
  highlights while hovering), drag a file onto a terminal pane → its quoted
  path is typed, right-click → Copy/Cut then right-click a folder → Paste,
  Cmd-C/Cmd-X/Cmd-V, cut items dim red, banner shows + Clear works, drop a
  file from Finder onto the tree → it is copied in.

### Current State
- Branch `master`, ~24 unpushed commits + this one.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded.
- `reference-src/` untouched. **Phase 04: T05-001 + T05-002 done.**

### What's Next
- Next roadmap task after T05-002 (see `tasks/ROADMAP.md`) — Phase 04 is
  complete if T05-002 is its last task; otherwise the next `T05-*`. Likely
  Phase 05 (Editor) T06-001.

---

## Session: 2026-09-01 (T05-001 — file tree & explorer basics)

### What Was Done
- **T05-001 ✅ Done.** Sidebar file explorer, ported from
  `reference-src/src/modules/explorer/` (`useLocalExplorerStore` +
  `useFileTree` + `buildTreeRows` + `FileTreeNode`) and the backend
  `src-tauri/src/modules/fs/` (`tree.rs`, `mutate.rs`, `watcher.rs`).
  - **`crates/backend/src/modules/fs/tree.rs`** — added blocking, in-process
    entry point `read_dir_page(path, offset, limit, show_hidden)` (=
    `list_dir_entries_sync` + `paginate_dir_entries`, both made `pub`),
    `pub const DEFAULT_LOCAL_PAGE_LIMIT`, `#[derive(Clone, Debug, ...)]` on
    `EntryKind` / `DirEntry` / `DirReadPage`. +3 tests (dirs-first
    case-insensitive sort, hidden toggle, missing dir → Err not panic).
  - **`crates/backend/src/modules/fs/mutate.rs`** — added sync variants
    `create_file_sync` / `create_dir_sync` / `rename_sync` / `delete_sync`
    (same semantics as the async commands, no `spawn_blocking`). +2 tests
    (create/rename/delete roundtrip incl. conflict refusal, delete-missing
    → Err).
  - **`crates/ui/src/explorer.rs`** (new) — `TreeModel` (pure port of
    `useLocalExplorerStore` + `buildTreeRows`: per-dir node map
    `Loading|Loaded{entries,has_more}|Error`, `generation` stale-guard, lazy
    `needs_load` dedup, `toggle_show_hidden` cache-invalidate-but-keep-expanded,
    `rows()` flatten with pending-create / rename / loading / error / load-more
    rows) + `ExplorerView` GPUI entity wrapping it:
    - `set_root`/`set_root_str` (bumps generation, clears cache, loads root),
      `load_dir` runs `tree::read_dir_page` on `cx.background_executor().spawn`
      and drops the result if `generation` moved,
    - toolbar: new file / new folder / refresh / toggle-hidden / collapse-all,
    - rows in an `overflow_y_scroll` column (chevron + glyph + name, indent by
      depth, selection highlight, gitignored = muted), click folder →
      expand/collapse, click file → `Workspace::open_file` (Editor
      placeholder tab titled with basename, path in `TabData.path`),
    - right-click → context menu (New File/Folder, Rename, Delete, Copy Path,
      Open in Terminal) anchored top-left with a click-catch backdrop,
    - inline text field (hand-rolled like the header search: `key_char` +
      backspace/enter/escape) for create + rename → `mutate::*_sync` off-thread,
      reload parent on success, error toast (`notification_center`) on failure,
    - **embedded watcher**: `notify-debouncer-mini` (300 ms, non-recursive,
      watch-set synced to loaded dirs); debounced callback pushes affected
      parent dirs into an `Arc<Mutex<HashSet>>` drained by a 400 ms
      `cx.spawn` loop that force-reloads any still-loaded dirty dir.
    - delete confirmation modal overlay.
    - 6 pure `#[test]`s on `TreeModel` (flatten+depth+collapse, lazy-load
      dedup, set_root generation+clear, hidden-toggle invalidation,
      watch-target set, glyph map).
  - **`crates/ui/src/workspace.rs`** — added `new_terminal_tab_in(cwd, …)`
    (Explorer "Open in Terminal") and `open_file(path, …)` (Editor placeholder
    tab); `render_content` Editor arm now shows the file path.
  - **`crates/ui/src/app_shell.rs`** — `AppShell` owns
    `explorer: Entity<ExplorerView>`; `render_panel_body` returns the real
    view for `SidebarPanel::Explorer`; a `cx.observe(&workspace)` pushes the
    active terminal cwd (fallback `$HOME`) into `ExplorerView::set_root_str`.
  - **`crates/ui/Cargo.toml`** — `notify = "6"`, `notify-debouncer-mini = "0.4"`.
  - **`crates/ui/src/lib.rs`** — `pub mod explorer;` + `pub use ExplorerView`.
- **GPUI notes:** `overflow_y_scroll` is a `StatefulInteractiveElement` method
  → the scroll container needs an `.id(...)` first. `EditMode` enum gets
  `#[derive(Default)]` + `#[default]` on the unit variant. On macOS the
  `notify` `FsEventWatcher` exposes inherent `watch`/`unwatch` so the
  `notify::Watcher` trait import is unused (removed). `ClipboardItem::new_string`
  for Copy Path.
- **Deviations:** no `@tanstack/react-virtual` windowing (plain scroll column;
  500-entry page cap + lazy load bound the element count); watcher is embedded
  in the entity instead of the backend event bus; file-type icons are a small
  emoji glyph map, not the material-icon-theme port; SFTP/remote scope +
  `remoteScopeCache` LRU are Phase 07, not ported here; chmod/chown dialog and
  DnD are T05-002.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin
  labonair` — all green. Counts: **backend 131 (+5)**, **ui 48 (+6)**. Not
  visually run — user should `cargo run` and check: Explorer panel shows the
  cwd tree, folders expand/collapse, click a file opens an "Editor" tab,
  right-click → New File/Folder/Rename/Delete/Copy Path/Open in Terminal,
  create a file in Finder and watch the tree refresh, toggle-hidden button
  shows/hides dotfiles.

### Current State
- Branch `master`, ~23 unpushed commits + this one.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded.
- `reference-src/` untouched. **Phase 04: T05-001 done, T05-002 next.**

### What's Next
- **T05-002** (`tasks/phase-04-explorer/T05-002-drag-drop-actions.md`) —
  drag-and-drop + advanced file actions (chmod/chown, copy/cut/paste, OS drop).
  Dep: T05-001 (done).

---

## Session: 2026-09-01 (T04-005 — native macOS menus + Dock menu)

### What Was Done
- **T04-005 ✅ Done.** Native macOS menu bar + Dock context menu, ported from
  `reference-src/src-tauri/src/lib.rs` `build_menu`, `modules/dock_menu.rs`,
  `modules/menu_sync.rs`.
  - **`crates/ui/src/menu.rs`** (new):
    - `gpui::actions!(labonair, [...])` — 40 unit-struct actions covering every
      menu entry (File/Edit/View/Terminal/Connections/AI/Window/App).
    - `init(cx: &mut App)`: `cx.bind_keys(bindings())` (23 `KeyBinding`s mirroring
      the reference accelerators — drives both the menu shortcut hint *and*
      dispatch), 8 app-global `cx.on_action` handlers (Quit→`cx.quit()`,
      HideApp/HideOthers/ShowAll→`cx.hide*`, About/OpenSettings/AiSettings/
      CheckForUpdates→info toast placeholders via `notification_center`),
      `cx.set_menus(app_menus())`, `#[cfg(macos)] cx.set_dock_menu(dock_menu())`.
    - `app_menus()` — 8 submenus, structure/order/labels 1:1 with the reference
      (App menu adds Services `os_submenu` + Check for Updates; Edit uses
      `MenuItem::os_action(.., OsAction::{Undo,Redo,Cut,Copy,Paste,SelectAll})`).
      Window submenu named exactly `"Window"` so GPUI wires the live window list.
    - `dock_menu()` — New Terminal Tab / New SSH Connection… / — / Open Host Manager.
    - 3 unit tests (binding count = parse-all-succeed, menu-bar name order,
      dock entry count).
  - **`crates/ui/src/app_shell.rs`** — 12 action handler methods
    (`act_new_terminal_tab`, `act_close_tab`, `act_close_pane`, `act_split_right/
    _down`, `act_find`, `act_toggle_sidebar`, `act_toggle_fullscreen`,
    `act_minimize`, `act_zoom_window`, `act_next_tab`, `act_prev_tab`) registered
    via `.on_action(cx.listener(...))` on the root element. `SplitPaneRight/Down`
    are only registered `when(active_is_terminal)`, `ClosePane` only
    `when(active_has_split)` — so those menu items grey out automatically
    (macOS calls `validate_menu_item` → `is_action_available` against the live
    focus dispatch tree on every menu open; no `set_menus` re-sync needed).
    Removed `AppShell::on_key_down` (Cmd-B / Cmd-F) — now the `ToggleSidebar` /
    `Find` actions.
  - **`crates/ui/src/workspace.rs`** — added thin `pub` wrappers
    (`new_terminal_tab`, `split`, `close_active`, `close_pane`, `cycle`,
    `active_has_split`) so menu + shortcut share one path. Removed the
    Cmd-T/W/D/Shift-D arms from `Workspace::on_key_down` (now actions; the
    Cmd-Shift-`[`/`]` tab-cycle arms stay as an extra binding alongside the new
    `ctrl-tab` / `ctrl-shift-tab`).
  - **`crates/app/src/main.rs`** — `labonair_ui::init_menus(cx)` right after
    `open_window`, before `cx.activate(true)`.
  - **`crates/ui/src/lib.rs`** — `pub mod menu;` + `pub use menu::init as init_menus`.
- **GPUI API used (gpui 0.2.2, verified in source `src/platform/app_menu.rs` +
  `src/platform/mac/platform.rs` + `examples/set_menus.rs`):**
  `App::set_menus(Vec<Menu>)`, `App::set_dock_menu(Vec<MenuItem>)` (mac only),
  `Menu { name: SharedString, items: Vec<MenuItem> }`,
  `MenuItem::{action(name, impl Action), os_action(name, action, OsAction),
  os_submenu(name, SystemMenuType::Services), separator}`,
  `gpui::actions!(namespace, [Name, ...])` (unit structs, derive `gpui::Action`),
  `KeyBinding::new(&str, action, Option<&str>)` (**panics** on bad keystroke —
  `"cmd--"` / `"cmd-="` / `"ctrl-cmd-f"` all parse fine),
  `App::{quit, hide, hide_other_apps, unhide_other_apps}`,
  `Window::{toggle_fullscreen, minimize_window, zoom_window}` (all `&self`).
  Menu enable/disable is driven entirely by `is_action_available` (focus
  dispatch tree) — GPUI's `MenuItem::Action` has **no** `enabled` field and no
  checkmark support, so dynamic checkmarks / recent-hosts / window-list beyond
  what GPUI auto-provides are deferred to their feature phases.
- **Deviations / limits:** items for not-yet-built features (New SSH/SFTP/Preview/
  Editor Tab, Open Host Manager, New SSH Connection, New Quick SSH, New AI
  Session, Ask about Selection, Clear Chat, Toggle AI Panel, Keyboard Shortcuts,
  Zoom In/Out/Reset) have **no handler** → render disabled (per task note);
  their phase adds `.on_action` and they light up. `CloseTab` maps to the
  existing pane-aware `close_active_pane_or_tab` (closes pane if split, else
  tab); `ClosePane` (`cmd-shift-w`) always closes the pane. About / Settings /
  Check for Updates are toast placeholders (Settings = Phase 12, updater =
  T15-005). No `MenuItemRegistry`-style accelerator hot-swap (that's Phase 12
  shortcut config) — bindings are static in `menu::bindings()`.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` (all green; **ui 42, +3**),
  `cargo build --bin labonair` — all green. Not visually run — user should
  `cargo run` and check: full menu bar (Labonair/File/Edit/View/Terminal/
  Connections/AI/Window), Cmd-T / Cmd-W / Cmd-D / Cmd-F / Cmd-B still work via
  the menu path, "Split Pane …" greys out on the Home tab and enables on a
  terminal tab, "Close Pane" greys out until a tab is split, right-click the
  Dock icon → New Terminal Tab works.

### Current State
- Branch `master`, ~22 unpushed commits + this one.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded.
- `reference-src/` untouched. **Phase 03 complete** (T04-001..005 all done).

### What's Next
- **Phase 04 — File-Explorer: T05-001** (`tasks/phase-04-explorer/T05-001-*`) —
  Dateibaum & Explorer-Grundlagen. Dep: T04-002 (done). No blockers.

---

## Session: 2026-09-01 (T04-004 — notifications / toast system)

### What Was Done
- **T04-004 ✅ Done.** App-wide notification/toast system, ported from
  `reference-src/src/modules/notifications/` (`useNotificationStore` +
  `NotificationDropdown`).
  - **`crates/ui/src/notifications.rs`** (new):
    - `NotificationCenter` GPUI entity — `items: Vec<Active>` newest-first, `next_id`,
      `notify_on_errors: bool` (default `true` until T13-001 wires the real preference).
      Ported store logic: **2s spam guard** on `title + body + severity` vs `items[0]`
      (`insert(notif, now, cx)` takes an explicit `now` and is `pub` for deterministic
      tests; `push`/`push_action_result` call it with `Instant::now()`), **100-item cap**
      (`truncate`), **error gate** — `push` drops `Severity::Error` when
      `!notify_on_errors` (ref `addNotification`), `push_action_result` bypasses it
      (ref `addActionResultNotification`). `dismiss(id)`, `clear_all()`, `len`/`is_empty`,
      `snapshots() -> Vec<ToastSnapshot>` (render view), `trigger_action(id, window, cx)`
      (fires the `FnMut` callback once, then removes the toast).
    - `Severity{Info,Success,Warning,Error}` + `default_timeout()` = info 5s / success 4s /
      warning 8s / **error None** (manual dismiss — ref never auto-cleared errors) + `glyph()`
      + `color(&ThemeStore)`.
    - Per-toast **auto-dismiss**: `insert` does `cx.spawn(async move |this, cx| { timer(d).await;
      this.update(..dismiss(id)) })` when a timeout resolves.
    - `Notification` builder (`info/success/warning/error(title, body)` + `.source()` /
      `.timeout()` / `.action()`), `NotificationAction::new(label, impl FnMut(&mut Window, &mut App))`
      (`type ActionCallback = Box<dyn FnMut(..)>`; manual `Debug`).
    - `render_overlay(&Entity<NotificationCenter>, &Entity<ThemeStore>, &mut App) -> Option<AnyElement>`
      — `.absolute().top_4().right_4()` flex-col stack of 360px cards (`bg-card`, 1px border in the
      severity color, `shadow_lg`, glyph + title[semibold] + optional source badge + muted body,
      close ✕, optional accent action button). The container only occupies its own top-right box, so
      clicks elsewhere pass through — **warning satisfied** (only the cards are interactive).
    - Global `GlobalNotificationCenter` + `init(cx)` / `notification_center(cx)`, and the
      `notify_err(title, Result<T, String>, &mut App) -> Option<T>` helper (pushes an error toast via
      the action-result path on `Err`).
    - 8 `#[gpui::test]`s: newest-first + ids, spam guard (block/allow by body/type/time),
      different-titles kept, error gate + action-result bypass, 100 cap, dismiss/clear,
      action label wiring, auto-dismiss after `advance_clock`.
  - **`crates/ui/src/theme.rs`** — new accessors `status_error()` / `status_warning()` /
    `status_info()` / `status_success()` (read `theme().status.*`).
  - **`crates/ui/src/app_shell.rs`** — `AppShell` gained a `notifications: Entity<NotificationCenter>`
    field + `new()` param, observes it, renders `render_overlay(...)` via `.children(toasts)` after the
    background layer. `#[cfg(debug_assertions)]` startup demo toast (acceptance: reachable from anywhere).
  - **`crates/app/src/main.rs`** — `labonair_ui::init_notifications(cx)` before `AppShell::new`.
  - **`crates/ui/src/lib.rs`** — `pub mod notifications;` + re-exports (`init_notifications`,
    `notification_center`, `notify_err`, `Notification`, `NotificationAction`, `NotificationCenter`,
    `Severity`, `GlobalNotificationCenter`).
- **No new deps.** GPUI APIs used were all already known (`cx.spawn` async-closure form,
  `background_executor().timer`, `on_click` needs `StatefulInteractiveElement` in scope, tuple
  `ElementId` `(&str, u64)`, `.children(Option<AnyElement>)`).
- **Deviations / notes:** the reference notifications module is a *persistent dropdown* (bell icon +
  popover list, no auto-dismiss); the task asked for a *toast* system, so this port keeps the store
  semantics 1:1 but renders stacked auto-dismissing toasts instead of the popover. No bell/dropdown
  entry point in the status bar yet (needs the bar-item placement settings from T13 + an icon set) —
  the store keeps the full history regardless, so a dropdown can be added later without touching it.
  `notify_on_errors` defaults to `true` (reference default is `false`, gated by a preference we don't
  have yet). `relativeTime` / "Copy" affordance from the dropdown not ported (toast-inappropriate).
- **Verified:** `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (all suites green; **ui 39, +8**), `cargo fmt --all --check`,
  `cargo build --bin labonair` — all green. Not visually run — user should `cargo run` and check:
  a demo info toast appears top-right on launch and auto-dismisses after ~5s; toasts stack; ✕ closes
  one; clicking through the area where a toast *isn't* still hits the UI behind it.

### Current State
- Branch `master`, ~22 unpushed commits + this one.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded from this commit.
- `reference-src/` untouched.

### What's Next
- **T04-005 — Native macOS menus** (`tasks/phase-03-tabs-workspace/T04-005-native-macos-menus.md`).
  Dep: T04-003 (done). No blockers.

---

## Session: 2026-09-01 (T04-003 — app-shell & window chrome)

### What Was Done
- **T04-003 ✅ Done.** Introduced `AppShell` as the app's root coordinator (ported from `reference-src/src/app/components/AppShell.tsx` + `modules/header/Header.tsx` + `modules/statusbar/StatusBar.tsx`). `cargo run` now shows the full app layout.
  - **`crates/ui/src/app_shell.rs`** (new) — `AppShell` GPUI root view. Pure composition, no feature logic:
    - Owns `theme`, `background`, `workspace: Entity<Workspace>` (creates the `TerminalRegistry` + `Workspace` here now, moved out of the old `Root` in `main.rs`), plus sidebar state (`open`, `width`, `active_panel`) and header search state.
    - **Header** (`HEADER_H = 40`, `bg-toolbar`, `border-b`): ☰ sidebar toggle, "Labonair" title, flex spacer, inline search field (shown on `Cmd-F`), `⋯` menu affordance (placeholder). Left inset `TRAFFIC_LIGHT_INSET = 78` (native titlebar still on, see deviation below).
    - **Body row**: optional left sidebar + `Workspace` (flex_1). Sidebar = a `RAIL_W = 44` panel-switcher rail (`SidebarPanel` enum: Explorer/Snippets/SourceControl/GitGraph/Ai — each a slot rendering "coming in a later phase") + the active panel body + a `cursor_col_resize` edge handle (drag via `on_drag(SidebarResize)` + `on_drag_move::<SidebarResize>` on the body row → `set_sidebar_width`, clamp 180–520). Toggle: ☰ button or `Cmd-B`. Clicking the active panel's rail icon collapses the sidebar.
    - **Status bar** (`STATUS_H = 32`, `bg-status-bar`, `border-t`): left = cwd breadcrumb built from `Workspace::active_cwd` (split on `/`, `~` home substitution via `dirs::home_dir`, last segment = `foreground`, rest muted, `/` separators); falls back to the active tab label when no cwd. Right = `N panes` when >1 + empty slots (connection / jump-host / AI badges land in Phase 06/10).
    - **Inline search**: `Cmd-F` opens a focusable field; typed chars go via `keystroke.key_char` (fallback to key name for single chars), `backspace`/`enter` re-run, `escape` closes + clears + refocuses the pane. Dispatched to `Workspace::search_active` → `TerminalView::search`.
  - **`crates/ui/src/window_state.rs`** (new) — minimal window geometry persistence (`<data_dir>/labonair/window.json`). `load()` on launch (in `main.rs`, feeds `WindowOptions.window_bounds`), `save()` from `AppShell` throttled per render (`SAVE_THROTTLE = 1s`, `bounds_differ` > 2px) **and** on `window.on_window_should_close`. `window_min_size` = 720×480. Overlaps intentionally with T14-001 (session persistence), which will supersede/extend this.
  - **`crates/ui/src/terminal.rs`** — added `TerminalView::search(query, cx) -> bool`: scans the visible screen (`SessionHandle::render().to_text()`) for the first `query` occurrence, selects it via `update_selection((col,row),(col+len,row))`; empty query clears selection. Visible-screen only — a full scrollback find widget with next/prev is a later search-module concern.
  - **`crates/ui/src/workspace.rs`** — **stripped its own chrome** (removed `render_header`/`render_sidebar`/`render_statusbar`, `sidebar_open`/`sidebar_width` fields, `SidebarResize`, `Cmd-B`, `HEADER_H`/`STATUS_H`/`SIDEBAR_*` consts). `Render` is now just tab bar + content + overlays. Added pub accessors for `AppShell`: `active_cwd`, `active_tab_label`, `active_pane_count`, `active_is_terminal`, `search_active`, `focus`.
  - **`crates/app/src/main.rs`** — deleted the old `Root` view; root is now `AppShell::new(...)`. Added `TitlebarOptions { title: "Labonair" }`, `window_min_size`, `window_state::load()` restore.
  - **`crates/ui/Cargo.toml`** — moved `serde_json` to deps, added `dirs`. **`crates/theme` → `crates/ui/src/theme.rs`** — new accessors `toolbar()`, `title_bar()`, `status_bar()`, `sidebar_bg()`, `sidebar_border()`, `sidebar_fg()`.
- **GPUI API used (gpui 0.2.2, verified in source):** `Window::on_window_should_close(cx, Fn(&mut Window, &mut App) -> bool)`; `Window::window_bounds() -> WindowBounds` (`Windowed(Bounds<Pixels>)` variant carries the restore rect); `WindowOptions.titlebar: Option<TitlebarOptions { title, appears_transparent, traffic_light_position }>`, `window_min_size: Option<Size<Pixels>>`. No `observe_window_bounds`/resize hook exists → geometry save is throttled-in-render + on-close. `Keystroke.key_char: Option<String>`.
- **Deviation / limits:** kept the **native macOS titlebar** (`appears_transparent: false`) rather than a custom transparent drag-region titlebar with tabs-in-titlebar (reference default). Traffic lights + native window drag work; the reference's custom-drawn titlebar + `data-tauri-drag-region` + window-move-on-drag was judged out of scope for this task — revisit if design parity demands it. Tab bar stays as its own strip below the header (reference `tabsLocation` supports a "pane" mode, so this is a supported layout, not a regression). Right sidebar still deferred (was already deferred in T04-002).
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (all 14 suites green; ui unchanged at 31 — no new unit tests, the feature is view/window wiring), `cargo build --bin labonair` — all green. Not visually run — user should `cargo run` and check: header + sidebar rail + tabs + statusbar all render; ☰ / `Cmd-B` toggle sidebar; rail icons switch panel; drag sidebar edge to resize; `Cmd-F` opens header search, typing scrolls/highlights a match in the active terminal, `Esc` closes it; statusbar breadcrumb tracks `cd`; resize/move the window, quit, relaunch → geometry restored.

### What's Next
- **T04-004 — Notifications & Toasts** (`tasks/phase-03-tabs-workspace/T04-004-notifications-toasts.md`). No blockers.

---

## Session: 2026-09-01 (T04-002 — split-pane layout & workspace shell)

### What Was Done
- **T04-002 ✅ Done.** Added the split-pane layout tree + full workspace window chrome (pure Rust + GPUI, ported from `reference-src/src/modules/tabs/types.ts` + `store/tabsStore.ts` `splitPane`/`closePane` and `WorkspacePane.tsx`).
  - **`crates/ui/src/pane.rs`** (new) — pure, serde-serializable pane tree:
    - `PaneNode` = `Pane { id }` | `Split { id, axis, ratio, first, second }` (`#[serde(tag="type")]`, axes `horizontal`/`vertical` lowercased). `SplitAxis::Horizontal` = children left→right. `MIN_RATIO = 0.1`.
    - `WorkspaceLayout { root, active }` (no id allocation — the `Workspace` view owns a process-wide `next_pane_id`; `split()`/`new()` take caller-allocated ids so pane ids stay unique across tabs). Ops: `new(first)`, `split(split_id, new_pane, axis)` (replaces active leaf with a 50/50 split, new pane becomes active — mirrors reference `splitPane`), `close(target) -> CloseOutcome::{Closed{new_active}, LastPane, NotFound}` (collapses parent split into sibling; new active = **promoted sibling subtree's first leaf**, matching reference `siblingLeaves[0]`, not the tree's global first leaf), `set_active`, `set_ratio`/`reset_ratio` (clamped `[MIN_RATIO, 1-MIN_RATIO]`), `leaves()`, `len()`.
    - 6 unit tests: split nesting/activation, close→collapse-into-sibling, close keeps a non-removed active valid, ratio clamping + reset, 8-pane deep nest build+teardown stays non-empty/consistent, JSON round-trip.
  - **`crates/ui/src/workspace.rs`** — `Workspace` now owns `layouts: HashMap<tab_id, WorkspaceLayout>` (survives tab switches — pane tree never lost) + `panes: HashMap<PaneId, PaneEntry{session_id, view}>` + `next_pane_id`. Replaced the old one-`TerminalView`-per-tab map.
    - `split_active(axis)` — spawns a new session in the active pane's cwd, `layout.split(...)`, builds a `TerminalView`, focuses the new pane. `close_active_pane` — `layout.close`; `LastPane` → `request_close(tab)`, else `retire_pane` (registry.close). `Cmd-W` = `close_active_pane_or_tab` (pane if split, else whole tab). `retire_tab` tears down **every** pane/session in the closed tab's layout.
    - `render_pane_node` recursively renders the tree: `Split` → flex row/col, `flex_basis(relative(ratio))` first child + `flex_grow` second, a draggable divider (`HANDLE = 6px`, `cursor_col_resize`/`row_resize`); drag handled by `on_drag(PaneResize{split_id})` + `on_drag_move::<DragMoveEvent<PaneResize>>` on the split group — computes new fraction from `ev.bounds` + `ev.event.position` along the axis → `resize_split`. Double-click the divider → `reset_split` (50/50). `Pane` leaf → its `TerminalView` (or empty), 1px border (accent when active+multi, else bg-coloured so no layout shift), click-to-activate.
    - **Workspace hull**: `render_header` (☰ sidebar toggle + "Labonair", `HEADER_H = 36`), body row = optional left sidebar + central column (tab bar over content), `render_statusbar` (active tab label + "· N panes", `STATUS_H = 24`). `render_sidebar` — `card`-bg column (default 260px, clamp 180–520) with an "EXPLORER" header + placeholder + a `cursor_col_resize` edge handle; drag via `on_drag(SidebarResize)` + `on_drag_move::<SidebarResize>` on the body row → `set_sidebar_width` (position.x − row origin.x). Toggle: header button or `Cmd-B`.
    - Keys added to `on_key_down`: `Cmd-D` split horizontal, `Cmd-Shift-D` split vertical, `Cmd-B` toggle sidebar. `Cmd-W` now pane-aware.
  - **`crates/ui/src/terminal.rs`** — `TerminalView` grids to **its own content area** now, not the whole window: new `measured: Option<Size<Pixels>>` field captured every paint by an absolute `canvas(|bounds,_,cx| this.measured = Some(bounds.size) + notify-on-change, |_,_,_,_| {})` child; `render` uses `self.measured.unwrap_or_else(|| window.viewport_size())` for `grid_size`. Needed so split-pane terminals don't oversize.
  - **`crates/ui/Cargo.toml`** — added `serde` (workspace) dep + `serde_json` dev-dep. **`crates/ui/src/lib.rs`** — `pub mod pane;` + re-exports `CloseOutcome, PaneId, PaneNode, SplitAxis, WorkspaceLayout`.
- **GPUI API used (gpui 0.2.2, verified in source):** `gpui::canvas(prepaint: FnOnce(Bounds<Pixels>, &mut Window, &mut App) -> T, paint: FnOnce(Bounds<Pixels>, T, ...))` — `impl Styled for Canvas<T>` so `.absolute().size_full()` chain works; capture `cx.weak_entity()` and `weak.update(cx, ...)` inside the prepaint closure (it gets `&mut App`). `InteractiveElement::on_drag_move::<T>(Fn(&DragMoveEvent<T>, &mut Window, &mut App))` — `DragMoveEvent { event: MouseMoveEvent, bounds: Bounds<Pixels> }`, `.drag(cx) -> &T` reads the active drag value; `cx.listener(...)` adapts directly. `gpui::relative(f32) -> DefiniteLength` for `flex_basis`. `ClickEvent::click_count()` for double-click. Cursor helpers `cursor_col_resize()` / `cursor_row_resize()`. `Pixels` arithmetic: `f32::from(a - b)`.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 126 backend, 1 app_state, 22 theme, **31 ui (+6)**, 62 terminal. Not visually run — user should `cargo run` and check: `Cmd-D`/`Cmd-Shift-D` split the active pane (new shell, same cwd), drag a divider to resize (double-click = 50/50), click a pane to focus it, `Cmd-W` closes the focused pane (or the tab if it's the last pane), `Cmd-B` / the ☰ button toggle the sidebar and its edge is draggable, the status bar shows the pane count, switching tabs keeps each tab's split layout intact.

### Design notes / limits (T04-002)
- Pane content is terminal-only for now; other `TabKind`s still show the "coming in a later phase" placeholder (editor-as-pane-content is Phase 5). Sidebar content is a placeholder (Explorer = Phase 4). Header/statusbar are minimal shells (full chrome = T04-003).
- Layout persistence: `WorkspaceLayout` serializes to JSON (test proves round-trip); wiring it into a session snapshot is Phase 13. The `panes`/`session_id` ↔ pane-id mapping is view-side and not yet serialized.
- Only a left sidebar (task scope); the reference also has a right sidebar — deferred.
- `TabData.session_id` is still set by `open_workspace` for the initial pane (label/compat); teardown now goes through the layout's leaf list, so multi-pane tabs fully clean up.

### What's Next
- **T04-003** `tasks/phase-03-tabs-workspace/T04-003-app-shell-window-chrome.md` — app shell & window chrome. Dep: T04-002 (done).

---

## Previous Session: 2026-09-01 (T04-001 — tab bar & tab management)

### What Was Done
- **T04-001 ✅ Done.** Wired the T03-005 `TerminalRegistry` into the UI and built the tabbed workspace shell (pure Rust + GPUI, ported from `reference-src/src/modules/tabs/`).
  - **`crates/ui/src/tabs.rs`** (new) — the tab data model + `TabStore` GPUI entity:
    - `TabKind` (Home / Workspace / Editor / Preview / AiDiff / Sftp / GitGraph / GitDiff / CommitDiff) with `indicator()` glyph + `default_title()`. `TabData` = flat optional bag (session_id, cwd, process_title, path, host_id, repo_path, url) documented per-kind — deliberately not a per-kind enum so later phases add fields without churn. `Tab { id, kind, title, custom_title, dirty, peek, data }` with `label()` mirroring the reference `labelFor` (custom title → process title → cwd basename → fallback) and `needs_close_confirm()` (dirty editor).
    - `TabStore { tabs: Vec<Tab>, active_id, next_id }`, `EventEmitter<ActiveTabChanged>`. Ops: `open`/`open_workspace`, `set_active`, `cycle(forward)`, `close` (Home + last-tab guarded, returns removed `Tab` so caller tears down its session, active→left-neighbour like reference `closeTab`), `close_others`, `close_by_kind`, `reorder(dragged,target)` (array-move like reference `reorderTabs`), `tabs_by_kind`, field mutators (`set_title`/`set_custom_title`/`set_dirty`/`set_peek`/`set_path`), `sync_workspace_meta`. Every mutation `cx.notify()`s.
    - 7 `#[gpui::test]`s: add/switch/close, close-active→left-neighbour, last-tab+Home unclosable, title/label resolution, reorder, dirty-editor confirm flag, tabs_by_kind + close_by_kind.
  - **`crates/ui/src/workspace.rs`** (new) — `Workspace` GPUI entity: owns `Arc<TerminalRegistry>`, `Entity<TabStore>`, `HashMap<tab_id, Entity<TerminalView>>` (content views kept alive across switches — session lives in the registry regardless).
    - `open_terminal_tab`: inherits cwd from the active terminal's `.cwd()`, `registry.create(...)`, `open_workspace`, builds a `TerminalView` from the `SessionHandle`.
    - Close: `request_close` → dirty editor shows an inline confirm overlay (`confirm_close: Option<u64>`), else `do_close` → `TabStore::close` + `registry.close(session_id)` (no orphaned shells) + drop cached view + `focus_active`.
    - Tab bar render: horizontal `overflow_x_scroll()` strip, per-tab pill (indicator glyph + truncated `label()` + dirty dot + hover close ✕), active tab uses `theme.accent()`, `+` button. Drag-reorder via `on_drag(DraggedTab{id,label})` + `TabDragPreview` render view + `drag_over::<DraggedTab>` left-border highlight (live feedback) + `on_drop` → `TabStore::reorder`. Middle-click closes; right-click opens a context menu (`context_menu: Option<(u64, Point)>`) with Close / Close Others / Close All Of This Type + full-window click-catcher to dismiss.
    - Content area: active Workspace tab → its cached `TerminalView`; every other kind → `placeholder("… — coming in a later phase")`.
    - Keyboard (`on_key_down` on the workspace root, bubbled up from the focused terminal which returns `None` for Cmd combos): Cmd-T new, Cmd-W close (with confirm), Cmd-Shift-] / Cmd-Shift-[ next/prev; `cx.stop_propagation()` on each. Full configurability is Phase 12.
    - `_meta_sync` background task (400 ms) reads each terminal's `cwd()`/`shell_title()` into its tab via `sync_workspace_meta` so tab labels track the shell.
  - **`crates/ui/src/terminal.rs`** — `TerminalView` no longer spawns its own `TerminalSession`; `new()` now takes a `SessionHandle` (registry-backed). All session calls go through `handle` (`handle.with(|s| …)` for scroll/selection/mode/render/metadata, `handle.write`, `handle.resize`, `handle.set_colors`, `handle.drain_events`). Dropped the `Result<TerminalSession,String>` + spawn-failure render branch (registry owns spawn now). Added `handle()`, `focus(&mut Window)`, and a bottom "Shell exited (code) — press ⌘W to close" overlay when `handle.status()` is `Exited`.
  - **`crates/app/src/main.rs`** — `Root` now creates `Arc<TerminalRegistry>` + `Entity<Workspace>` instead of a bare `TerminalView`; renders the workspace full-window under the app-level background layer.
  - **`crates/ui/Cargo.toml`** — added `tracing` (workspace) dep. **`crates/ui/src/lib.rs`** — `pub mod tabs; pub mod workspace;` + re-exports `Tab`, `TabData`, `TabKind`, `TabStore`, `Workspace`.
- **GPUI API used (gpui 0.2.2, verified in source):** `InteractiveElement::on_drag<T,W: Render>(value, Fn(&T, Point<Pixels>, &mut Window, &mut App) -> Entity<W>)` — build the preview with `cx.new(...)` on the `&mut App`; `InteractiveElement::drag_over<S: 'static>(Fn(StyleRefinement, &S, &mut Window, &mut App) -> StyleRefinement)`; `InteractiveElement::on_drop<T: 'static>(Fn(&T, &mut Window, &mut App))` — `cx.listener(...)` matches this signature directly (it returns `impl Fn(&E, &mut Window, &mut App)`); `ElementId: From<(&'static str, u64)>` (tuple ids with a `u64` work — no `usize` cast needed); `Styled`/FluentBuilder `when_some(Option<T>, FnOnce(Self,T)->Self)` needs `use gpui::prelude::FluentBuilder`; `ClickEvent` has **no** `stop_propagation` — use `cx.stop_propagation()`; `cx.new()` in a non-`Context` spot (`&mut App`) needs `use gpui::AppContext`.
- **Design note:** split-pane workspace tabs (multiple sessions per tab, `PaneNode` tree) are **T04-002** — a Workspace tab here owns exactly one session. Tab/Session separation from the reference is kept: closing/switching tabs never pauses a session, the registry is the sole session owner.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 126 backend, 1 app_state, 22 theme, **25 ui (+7)**, 62 terminal. Not visually run — user should `cargo run` and check: `+`/Cmd-T opens a new terminal tab (inherits cwd), tab click/Cmd-Shift-brackets switch, ✕/Cmd-W/middle-click close, drag to reorder, right-click menu, background terminals keep running while another tab is visible, closing a terminal tab kills its shell (no zombie).

### Current State
- Branch `master`, ~18 unpushed commits + this one. `crates/ui` owns the tab model + workspace shell; `TerminalView` is now registry-backed.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### Known limitations (not blockers)
- One session per Workspace tab (splits = T04-002). No tab rename UI yet (`set_custom_title` exists, no inline `<input>` equivalent) — reference has it; deferred with the rest of the tab polish.
- Tab-bar overflow is a plain `overflow_x_scroll` (no sliding-pill animation, no scroll-active-into-view). Context menu is a hand-rolled panel (no `gpui-component` menu primitive in the workspace yet).
- The "shell exited" overlay is informational only — no in-place restart button (registry `SessionHandle::restart` exists; wire a KeepTerminal screen in a later polish pass).
- Editor/Sftp/Git/etc. tab kinds render a placeholder; their content + interactive creation come with their phases.

### What's Next
- **T04-002** `tasks/phase-03-tabs-workspace/T04-002-split-pane-layout-workspace.md` — split-pane layout & workspace (`PaneNode` tree, multiple sessions per tab). Dep: T04-001 (done).

### Blockers
- None.

---

## Session: 2026-09-01 (T02-006 — terminal/app background images)

### What Was Done
- **T02-006 ✅ Done.** Full parity port of the reference `backgrounds` feature (import/list/delete + opacity/blur/tint/fit/target), pure Rust + GPUI.
  - **`crates/backend/src/modules/backgrounds/mod.rs`** — de-Tauri'd: the 5 fns (`backgrounds_dir` now `pub`, `backgrounds_list`, `background_import`, `background_read_data_url`, `background_delete`) changed `async fn` → `fn` (they were only async for Tauri; no `.await` inside, no callers). Added the preferences layer:
    - `BackgroundSettings` (`#[serde(rename_all="camelCase")]`) = reference `preferencesStore` keys `backgroundImage` (filename, `""`=none), `backgroundOpacity` (u8 0–100, def 30), `backgroundBlur` (u8 px 0–100, def 0), `backgroundTintColor` (`#000000`), `backgroundTintOpacity` (u8, def 0) **plus** two keys the pure-Rust renderer needs that the reference did implicitly: `backgroundFit` (`BackgroundFit::{Cover,Contain,Tile}`, def Cover) + `backgroundTarget` (`BackgroundTarget::{Both,App,Terminal}`, def Both). Enums `#[serde(rename_all="lowercase")]`.
    - `background_settings_load()` / `background_settings_save(&BackgroundSettings)` read/merge the shared `config_dir()/labonair-settings.json` blob (same file `super::settings` uses) key-by-key so `barItemPlacements` etc. survive; atomic tmp+rename write; numbers clamped ≤100 on load. Internal `load_from(&Path)`/`save_to(&Path,..)` for tests.
    - 4 new tests (defaults match reference, missing-file → defaults, round-trip + preserves other keys + enum wire form, out-of-range clamp). Uses `std::env::temp_dir()`, not the real config dir.
  - **`crates/ui/src/background.rs`** (new) — `BackgroundStore` GPUI entity + `GlobalBackground` global (`init(cx)` / `background_store(cx)`):
    - Owns `BackgroundSettings` + one decoded `Arc<gpui::Image>`, rebuilt **only** when `(filename, blur)` changes (cache key) — never per frame (task warning). On decode failure it clears the selection and persists that (reference fallback behavior).
    - `load_processed_from(dir,filename,blur)`: `image::load_from_memory` → downscale to `MAX_DIM=2560` (Triangle) → `image::imageops::blur` (pre-blur, since GPUI has **no** blur filter) → re-encode (PNG if source had alpha, else JPEG q85). Fast path: no downscale + no blur → hand GPUI the untouched file bytes with format guessed from extension. AVIF: undecodable by the `image` crate here (no `avif` feature) → only works via the raw fast path (no blur); documented limitation.
    - Mutators `set_image/opacity/blur/tint_color/tint_opacity/fit/target` (persist + rebuild + notify), `import(PathBuf)`, `delete(&str)`, `available()`, `prompt_and_import(cx)` (native picker via `cx.prompt_for_paths(PathPromptOptions{files:true,..})` + `cx.spawn`).
    - `layer(LayerScope::{App,Terminal}) -> Option<AnyElement>`: absolutely-positioned `inset_0` non-interactive overlay, `img(image).object_fit(fit).size_full()`, wrapper `.opacity(backgroundOpacity/100 * 0.5)` (reference `BG_OPACITY_RENDER_FACTOR`), optional tint `div` on top. `App`/`Both` → App scope shows (window-wide, covers terminal too); `Terminal` → Terminal scope shows only. `Tile` → `ObjectFit::Cover` fallback (GPUI has no tiling).
    - 3 tests (fit mapping incl. Tile→Cover, extension→format, generated 3000px image → downscaled + blurred + re-encoded to JPEG).
  - **`crates/ui/Cargo.toml`** — added `image` (workspace) + `labonair-backend` path dep (no cycle; backend doesn't dep ui). **workspace `Cargo.toml`** — added `image = "0.25"` (`default-features=false`, features jpeg/png/gif/webp/bmp; matches gpui's image 0.25.10 in the lockfile).
  - **`crates/ui/src/terminal.rs`** — `TerminalView` gained a `background: Entity<BackgroundStore>` field + `new()` param, `cx.observe(&background)` → repaint, and `.children(self.background.read(cx).layer(LayerScope::Terminal))` on top of the cell/cursor layers.
  - **`crates/app/src/main.rs`** — `Root` holds `Entity<BackgroundStore>` (`labonair_ui::init_background(cx)`), observes it, passes it to `TerminalView::new`, renders `.relative()` + `.children(background.layer(LayerScope::App))` over the theme bg + terminal.
  - **`crates/ui/src/lib.rs`** — re-exports `BackgroundStore`, `BackgroundFit`, `BackgroundTarget`, `LayerScope`, `init as init_background`, `background_store`, `GlobalBackground`.
- **GPUI API used (gpui 0.2.2, verified in source):** `gpui::img(impl Into<ImageSource>) -> Img`; `ImageSource::From<Arc<gpui::Image>>`; `gpui::Image::from_bytes(ImageFormat, Vec<u8>)` (`ImageFormat::{Png,Jpeg,Webp,Gif,Bmp}` — **no Avif variant**); `StyledImage::object_fit(ObjectFit)` (needs the `StyledImage` trait, from `gpui::prelude`; `ObjectFit` has no `PartialEq`/`Debug` → test with `matches!`); `Styled::{opacity(f32), inset_0(), overflow_hidden(), absolute()}` (opacity nests/multiplies via `Window::with_element_opacity`); `App::prompt_for_paths(PathPromptOptions{files,directories,multiple,prompt}) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>>` (on `App`, reachable through `Context` deref); `.children(Option<AnyElement>)` works (Option: IntoIterator).
- **Design note:** the reference wallpaper is one full-window overlay painted *on top* of everything at low opacity (xterm's `<canvas>` is opaque, can't sit behind it). The GPUI port keeps that exact model — overlay on top, `pointer-events`-free, opacity ×0.5 — rather than truly compositing behind the terminal cells (which are also opaque `bg()` divs). "Behind terminal" in criteria == the reference's dimmed-overlay look.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: **126 backend (+4)**, 1 app_state, 22 theme, **18 ui (+3)**, 62 terminal. Not visually run — user should `cargo run`, import an image via a future settings pane (or temporarily call `background_store(cx).update(..set_image..)`), and confirm opacity/blur/fit/target behave.

### Current State
- Branch `master`, ~16 unpushed commits + this one. `crates/ui` now depends on `crates/backend` (first such edge; slower ui compiles). Background layer live in both Root and TerminalView.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### Known limitations (not blockers)
- No settings UI yet — wiring `BackgroundStore` mutators + `prompt_and_import` into the Appearance pane is **T13-002**.
- AVIF images only load when no blur is requested (raw fast path); the `image` crate build here has no `avif` feature and GPUI's `ImageFormat` has no AVIF variant.
- `Tile` fit renders as `Cover` (GPUI `img()` has no tiling). The reference had no tile mode either.
- Re-encode (downscale/blur path) runs synchronously inside the GPUI update that changes the setting — fine for a one-shot user action; move to `spawn_blocking` if it ever stutters.
- Blur uses `image::imageops::blur` (true Gaussian, O(n·r)) applied once at load — not GPUI (no blur filter exists in 0.2.2).

### What's Next
- **T04-001** `tasks/phase-03-tabs-workspace/T04-001-*` — Tab-Leiste & Tab-Verwaltung (wires the T03-005 `TerminalRegistry` into the UI).

### Blockers
- None.

---

## Session: 2026-09-01 (T03-005 — local PTY sessions & multi-tab terminal)

### What Was Done
- **T03-005 ✅ Done.** Multi-session registry + lifecycle for local PTY terminals, ported from `reference-src/src-tauri/src/modules/pty/{mod.rs,session.rs}` (the `PtyState` HashMap + `pty_open`/`pty_close`/`pty_has_foreground_job`).
  - **`crates/terminal/src/registry.rs`** (new) — `TerminalRegistry`: `RwLock<HashMap<SessionId, Arc<Slot>>>` + `AtomicU64` id counter (starts at 1, never reused). `SessionId = u64`. API: `create(colors, dims, options) -> SessionId`, `handle(id) -> Option<SessionHandle>`, `ids()`, `len()`/`is_empty()`, `close(id)` (SIGHUP then drop → SIGKILL+join in `Drop`, returns promptly even with a wedged foreground job), `close_all()`.
  - **`SessionHandle`** — cheap `Clone` (`Arc<Slot>` + id). `write`/`resize`/`set_colors`/`with(|&TerminalSession|…)`/`drain_events`/`status`/`has_foreground_job`/`restart(dims)`. `drain_events` folds `Exit`/`ChildExit` into `SessionStatus::{Running, Exited(i32)}` as a side effect, so the existing UI poll loop gets shell-exit tracking for free. `restart` respawns in place with the stored `SessionOptions` + latest palette, same `SessionId` (→ same tab); errors if still running.
  - **`Slot`** — `Mutex<TerminalSession>` (Mutex only so `restart` can swap the whole value; hot path is `&self` on the inner session, no contention) + `Mutex<TerminalColors>` + `SessionOptions` + `Mutex<SessionStatus>`.
  - **`crates/terminal/src/session.rs`** — `SessionOptions.startup_command: Option<String>` (written to the PTY as input right after spawn, shell stays interactive — PTY buffers it until the shell reads, so ordering vs. shell-init is safe). New `TerminalSession::has_foreground_job()` (`#[cfg(unix)]`: `master.process_group_leader()` != `shell_pid`; `false` elsewhere) and `terminate()` (SIGHUP to `-pid` process group + `pid`; hard kill + join stays in `Drop`, so `drop` right after `terminate` can't hang).
  - **`crates/terminal/src/lib.rs`** — `pub mod registry` + re-export `SessionHandle, SessionId, SessionStatus, TerminalRegistry`.
  - **Deps:** added `libc = "0.2"` (workspace) + `[target.'cfg(unix)'.dependencies] libc` on `labonair-terminal` for SIGHUP. `portable_pty` already pulls libc transitively but doesn't re-export it.
  - **Tab-system seam (Phase 3)** documented in the `registry.rs` module doc: tab system holds `Arc<TerminalRegistry>` + a `SessionId` per local-terminal tab; calls `create` on open (inheriting cwd from the previous tab's `SessionMetadata::cwd`), `handle` for the visible tab's I/O, `status`/`has_foreground_job` for the "shell exited — click to restart" screen + close-confirm prompt, `restart` on click, `close` on tab close. Registry never pauses a session — visibility is purely UI.
  - 5 new registry tests: multiple independent sessions (output isolation), startup command runs, clean close with a live `sleep 300` foreground job (< 3s), background session keeps progressing while only another tab is polled, restart-in-place after `exit 7` (+ restart-on-running errors).
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 122 backend, 1 app_state, 22 theme, 15 ui, **62 terminal (+5)**. Not yet wired into the UI (`crates/ui/terminal.rs` still spawns a single `TerminalSession` directly — that migration is Phase 3 / T04-001). Not visually run.

### Current State
- Branch `master`, ~15 unpushed commits + this one. `crates/terminal` now owns the multi-session registry; UI still single-session until Phase 3.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### What's Next
- **T04-001** `tasks/phase-03-tabs-workspace/T04-001-*` — Tab-Leiste & Tab-Verwaltung. This is where the registry gets wired into the UI (replace the direct `TerminalSession` in `crates/ui/src/terminal.rs`).
- **T02-006** (terminal background images) also still unblocked.

### Blockers
- None.

---

## Session: 2026-09-01 (T03-004 — shell integration & CWD tracking)

### What Was Done
- **T03-004 ✅ Done.** OSC 7 + OSC 133 shell integration and session metadata tracking, ported from `reference-src` (`src-tauri/src/modules/pty/{shell_init.rs,scripts/*}` + `src/modules/terminal/lib/osc-handlers.ts`).
  - **`crates/terminal/src/scripts/`** (new) — verbatim copy of the reference shell-integration rc-files (`zshenv.zsh`, `zprofile.zsh`, `zlogin.zsh`, `zshrc.zsh`, `bashrc.bash`). They emit OSC 7 on every prompt, OSC 133 A/B/C/D around prompt/command/output, and reset OSC 0 at each prompt. Block-mode (`LABONAIR_BLOCKS`) branches kept intact.
  - **`crates/terminal/src/shell_integration.rs`** (new) — `Shell{Zsh,Bash,Other}` + `Shell::from_path`; `configure(&mut CommandBuilder, shell, blocks) -> Shell` sets `TERM/COLORTERM/TERM_PROGRAM=Labonair/LABONAIR_TERMINAL=1` (+ `LABONAIR_BLOCKS`), writes the rc-files atomically (tmp+rename) under `~/.cache/labonair/shell-integration/{zsh,bash}/`, sets `ZDOTDIR`/`LABONAIR_USER_ZDOTDIR` (+ `-l`) for zsh or `--rcfile <path> -i` for bash. Non-fatal on write failure (spawns without integration). No `dirs` dep — uses `std::env::var_os("HOME")`. 3 unit tests.
  - **`crates/terminal/src/engine.rs`** — `OscSniffer` (raw-stream tap, runs before the VTE parser) extended from OSC-7-only to also parse **OSC 133 A/B/C/D** and **OSC 0/1/2** titles. Internal `OscUpdate` enum; `feed()` folds updates into a new `SessionMetadata` and emits `TerminalEvent::{PromptStart,PromptEnd,CommandStart(Option<String>),CommandFinished(Option<i32>)}` (new variants) alongside the existing `Cwd`. OSC 7 now **percent-decoded** (`parse_osc7` mirrors the reference `^file://[^/]*(/.*)$` regex + `percent_decode`/`hex_val` helpers) and **gated**: an OSC 7 emitted while `in_command` (between 133;C and the next A/D) is ignored (untrusted-command-output parity with `registerCwdHandler`). `SessionMetadata{cwd,title,in_command,prompt_phase,last_exit_code,last_command}` + `PromptPhase{Unknown,PromptStart,Prompt,Executing}`. New `TerminalEmulator::{set_initial_cwd, metadata}`. OSC 0/2 title still also flows through alacritty's own `Title` event (sniffer only updates metadata, doesn't double-emit). 8 new engine tests (percent-decode, 133 lifecycle, bare C/D, in-command OSC7 gate, title set/reset, sequences-not-in-grid, initial cwd).
  - **`crates/terminal/src/session.rs`** — `SessionOptions.blocks: bool`; `spawn()` now calls `shell_integration::configure` instead of setting env inline, and seeds the emulator cwd from `options.working_directory` or `std::env::current_dir()`. New `TerminalSession::{metadata() -> SessionMetadata, cwd() -> Option<String>, ai_context(max_lines) -> TerminalContext}`; `TerminalContext{cwd,title,lines}` is the base data-holding for the Phase-10 AI live-context reader. 1 new session test (`/bin/bash` real spawn → `cd /tmp` → asserts OSC 7 tracked cwd + `ai_context`).
  - **`crates/terminal/src/lib.rs`** — `pub mod shell_integration`; re-exports `PromptPhase`, `SessionMetadata`, `TerminalContext`, `Shell`.
  - **`crates/ui/src/terminal.rs`** — `TerminalView::{cwd(), shell_title()}` accessors so the (not-yet-built) status bar / breadcrumb and new-tab logic can read them.
- **Design notes:** OSC 133/7 never reach the visible grid — alacritty's VTE parser consumes every OSC sequence (dispatches to a known set, silently drops the rest); the sniffer is a pure read-only tap on the raw bytes. Kept the sniffer approach from T03-001 rather than a `Term` handler because alacritty 0.24 has no OSC-7/133 hook.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 122 backend, 1 app_state, 22 theme, 15 ui, **57 terminal (+11: +8 engine, +3 shell_integration; the +1 session test replaced none)**. Not yet visually run — user should `cargo run` and check the status data once a status bar exists; for now confirm a zsh/bash shell still starts cleanly with the integration rc-files (no stray `]` / `133` leaking into the prompt, p10k/starship still fine).

### Current State
- Branch `master`, 14 unpushed commits + this one. `crates/terminal` owns shell-integration bootstrap + OSC parsing + `SessionMetadata`; `crates/ui` exposes `cwd()`/`shell_title()`.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) still garbles Next Task Protocol steps 1 & 3 — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### Known limitations (not blockers)
- Block-terminal rendering (the reserved-row prompt + floating block header) is not built — only the `LABONAIR_BLOCKS` env plumbing + `CommandStart` command-text capture. Block UI is later-phase.
- Title still double-sourced (alacritty `Title` event + `SessionMetadata.title`); UI currently uses neither. Consolidate when the tab bar lands (Phase 03).
- `ai_context` returns the visible grid tail only (no scrollback, no prompt/command/output segmentation yet) — enough for Phase 10 to build on.
- OSC 633 (VS Code shell integration) is not parsed — the reference scripts only emit 7 + 133, so there is no path that needs it.

### What's Next
- **T03-005** `tasks/phase-02-terminal/T03-005-local-pty-sessions.md` — local PTY sessions & multi-tab terminal. Deps: T03-001–004 (all done).
- **T02-006** (terminal background images) also still unblocked.

### Blockers
- None.

---

## Session: 2026-09-01 (T03-003 — keyboard & mouse mapping)

### What Was Done
- **T03-003 ✅ Done.** Full GPUI-event → terminal-byte translation.
  - **`crates/terminal/src/input.rs`** (new, no gpui, no alacritty types leaking) — modular, framework-agnostic mappers driven by a `ModeState` snapshot:
    - `KeyInput { key: Key (Char|Named), mods: Modifiers{shift,alt,ctrl,logo}, text: Option<String> }`, `NamedKey` enum (Enter/Tab/Backspace/Escape/Space/arrows/Home/End/PageUp/PageDown/Insert/Delete/Function(u8 1–20)).
    - `key_to_bytes(&KeyInput, &ModeState) -> Option<Vec<u8>>`. Helpers: `char_key_bytes` (Ctrl+letter→C0 0x01–0x1a, Ctrl+symbol `[ \ ] ^ _ @ ?` + digit aliases, alt→ESC prefix, UTF-8 passthrough), `named_key_bytes`, `cursor_key` (DECCKM `ESC O`/`ESC [`, modified → `CSI 1;<mod><final>` with `mod = 1+shift+2*alt+4*ctrl`), `tilde_key` (`CSI n~` / `CSI n;mod~`), `function_key` (F1–F4 SS3 `ESC O P..S` unmodified else CSI; F5–F20 xterm code table 15/17/18/19/20/21/23/24/25/26/28/29/31/32/33/34). Shift+Enter→`ESC CR` (Claude-Code parity, from reference key handler), Shift+Tab→`CSI Z`, Ctrl+Backspace→0x08. Cmd/logo combos return `None` (app shortcuts).
    - `paste_payload(text, bracketed)` — `\r\n`/`\n`→`\r`, bracketed mode wraps in `ESC[200~`/`ESC[201~` and strips any embedded `ESC[201~`.
    - Mouse: `MouseInput{button,kind,col,row,mods}`, `mouse_report(&MouseInput,&ModeState)` → SGR (`CSI < b ; x ; y M|m`) when `sgr_mouse` else legacy `CSI M` with +32 bias; button codes L/M/R=0/1/2, wheel=64/65, +32 motion, +4/+8/+16 shift/alt/ctrl; returns `None` unless the matching mode (click/drag 1002/motion 1003) is active.
    - Wheel: `wheel_action(&WheelInput,&ModeState) -> WheelAction::{Bytes,Scrollback}` — mouse-mode→wheel button reports (one per line step), alt-screen+`alternate_scroll`→arrow keys, else scrollback.
    - 18 unit tests covering every category (basic/ctrl/alt/modified-cursor/function-both-forms/tilde/specials/bracketed-paste/SGR+legacy mouse/drag-mode-gating/wheel×3).
  - **`crates/terminal/src/engine.rs`** — new `ModeState` struct (app_cursor/app_keypad/bracketed_paste/insert/alt_screen/alternate_scroll/mouse_report_click/mouse_drag/mouse_motion/sgr_mouse/utf8_mouse/kitty_keyboard/report_all_keys_as_esc) + `.mouse_reporting()`. `TerminalEmulator::mode_state()` reads `term.mode()` (`TermMode` bitflags). `update_selection_viewport((col,row),(col,row))` — folds in `grid().display_offset()` so a drag stays buffer-anchored, picks anchor/head `Side` from column direction; `selection_text()` → `term.selection_to_string()` (empty→None). +1 engine test would be nice but covered via session.
  - **`crates/terminal/src/session.rs`** — passthroughs: `mode_state()`, `update_selection()`, `clear_selection()`, `selection_text()`.
  - **`crates/terminal/src/lib.rs`** — `pub mod input` + re-exports (`key_to_bytes`, `mouse_report`, `paste_payload`, `wheel_action`, `Key`, `KeyInput`, `Modifiers`, `MouseButton`, `MouseEventKind`, `MouseInput`, `NamedKey`, `WheelAction`, `WheelInput`, `ModeState`).
  - **`crates/ui/src/terminal.rs`** — replaced the minimal `keystroke_to_bytes` with `keystroke_to_input(&Keystroke) -> Option<KeyInput>` (GPUI key-name → `NamedKey`/`Char`, `f1..f20` parse, `key_char` preferred) + `to_term_mods`. `TerminalView` gained `cell_size: (f32,f32)` (stored each render) and `drag_anchor: Option<(usize,usize)>`. New handlers: `on_mouse_down(Left)` → `mouse_report` if mouse mode else start selection (clear old); `on_mouse_move` (pressed Left) → `session.update_selection(anchor, cell_at(pos))`; `on_mouse_up(Left)` → release report or copy-on-select (`copy_selection` → clipboard); `on_mouse_down(Right)` → `paste_from_clipboard` (right-click-pastes parity); `on_scroll_wheel` → `wheel_action` (Bytes vs Scrollback); `on_key_down` → Cmd+C/Cmd+V intercepted for clipboard, else `key_to_bytes(&input, &this.mode())`, write, snap-to-bottom, clear selection. `cell_at(Point<Pixels>)` maps window px → clamped grid cell (assumes terminal origin (0,0) — full-window; revisit when the app shell adds bounds). `mode()` helper reads `session.mode_state()` with default fallback. 3 old tests replaced with 5 (`keystroke_to_input`+`key_to_bytes` round-trips, app-cursor mode, platform-shortcut→None, f5).
- **Kitty keyboard protocol:** deliberately **not** emitted — `ModeState::kitty_keyboard` is observable but our DA responses never advertise support, so shells stay on the legacy sequences (per task warning; revisit if a real Kitty need appears).
- **GPUI API used (gpui 0.2.2):** `InteractiveElement::{on_mouse_move, on_mouse_up}`; `MouseMoveEvent{position, pressed_button: Option<MouseButton>, modifiers}`, `MouseUpEvent{button, position, modifiers}`; `App::{write_to_clipboard(ClipboardItem), read_from_clipboard() -> Option<ClipboardItem>}`, `ClipboardItem::new_string(String)` / `.text() -> Option<String>`; `Keystroke{key: String, modifiers: Modifiers, key_char: Option<String>}`; `gpui::Modifiers{shift,alt,control,platform,function}` (`platform` = Cmd).
- **alacritty_terminal 0.24.2:** `Term::mode() -> &TermMode` (bitflags u32, `APP_CURSOR=1<<1`, `BRACKETED_PASTE=1<<4`, `SGR_MOUSE=1<<5`, `MOUSE_REPORT_CLICK=1<<3`, `MOUSE_DRAG=1<<13`, `MOUSE_MOTION=1<<6`, `ALTERNATE_SCROLL=1<<15`, `KITTY_KEYBOARD_PROTOCOL` = OR of the 5 report flags); `Term::selection_to_string() -> Option<String>`; `Grid::display_offset() -> usize`; `Selection::new(SelectionType::Simple, Point, Side)` + `.update(Point, Side)`, assign to `term.selection`.
- **Verified:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --bin labonair` — all green. Counts: 122 backend, 1 app_state, 22 theme, **15 ui**, **46 terminal (+18)**. Not yet visually run — user should `cargo run` and check: arrows/ctrl-combos/function keys in vim + bash, Cmd+C/Cmd+V, drag-select + copy-on-select, wheel scrollback vs. wheel in `less`/`htop`.

### Current State
- Branch `master`, 13 unpushed commits + this one. `crates/terminal::input` owns the pure mapping; `crates/ui` owns the GPUI event conversion.
- Pre-existing uncommitted `CLAUDE.md` edit (not ours) still garbles Next Task Protocol steps 1 & 3 — left untouched & excluded from this commit, flag to user.
- `reference-src/` untouched.

### Known limitations (not blockers)
- `cell_at` assumes the terminal element sits at window origin (0,0). Correct today (full-window); once tabs/splits land (Phase 03) it must use measured element bounds.
- Selection is `SelectionType::Simple` only — no double-click word / triple-click line select yet (T03-005 or polish).
- Kitty keyboard + OSC 52 clipboard: not implemented (Kitty intentionally; OSC 52 deferred — no shell path needs it yet).
- Numeric keypad application mode (DECKPAM) is tracked in `ModeState.app_keypad` but GPUI doesn't distinguish keypad keys, so no keypad-specific sequences are emitted.

### What's Next
- **T03-004** `tasks/phase-02-terminal/T03-004-shell-integration-cwd.md` — OSC 133 shell integration + CWD tracking. Deps: T03-002 (done).
- **T02-006** (terminal background images) also unblocked.

### Blockers
- None.

---

## Session: 2026-09-01 (T03-002 — GPUI terminal cell renderer)

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
