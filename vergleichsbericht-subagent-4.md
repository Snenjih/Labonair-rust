# Vergleichsbericht — Subagent 4

Scope: AI chat, Host Manager, CWD breadcrumbs, SSH connecting screen, shared design/UI primitives.
Method: frozen reference in `reference-src/` vs. Rust/GPUI port in `crates/`. Analysis only, nothing modified.

All paths absolute. `file:line` given for both sides.

---

## 0. Executive summary

| Area | Verdict | Severity |
|---|---|---|
| Shared UI primitives (Button/Input/Select/…) | No shared component layer exists at all; every view hand-rolls its own `btn`. Root cause of most other divergences. | **Critical** |
| CWD breadcrumb | ~5 % ported. Plain non-interactive text. No home-collapse, no click-to-cd, no dropdown, no context menu, no file mode, no remote. `pathUtils.ts` + test not ported. | **Critical** |
| SSH "connecting" screen | Not built. Reference is a 5-state machine + 4-stage progress + live log screen; port writes `"Connecting…"` into the PTY feed and shows one generic modal. `connectionStatusStore` not ported. | **Critical** |
| Host Manager | Wrong information architecture (single scroll list + modal form vs. master/detail + side panel + 4 tabs). ~9 form fields missing, auth-method taxonomy wrong, no autosave, no Test Connection, no grid/list/sort/ping/DnD. | **High** |
| AI panel | Backend (`crates/ai`) is largely complete; the **UI** is a single monolithic view missing ModelPicker, AgentSwitcher, slash-commands, directives, @-file picker, QueueStrip, TodoStrip, PlanDiffReview, voice, context pills, real tool chips, and a real text input. | **High** |

---

## 1. AI panel

### 1a. Reference structure

- Panel body: `reference-src/src/modules/ai/components/AiMiniWindow.tsx:160` (`Body`) composes, top→bottom:
  - `Header` (`AiMiniWindow.tsx:240`) → identity pill containing `AgentSwitcher` + `ModelPicker grouped` + `ContextIndicator`, plus `BoundTabBadge`.
  - `PlanModeStrip` (`AiMiniWindow.tsx:205`) — amber "Plan mode · N queued / Exit".
  - `AiChatView` (`reference-src/src/modules/ai/components/AiChat.tsx:53`) — the message list only.
  - `QueueStrip` (`reference-src/src/modules/ai/components/QueueStrip.tsx:9`) — numbered queued follow-ups, "sends when AI finishes", per-item cancel.
  - `TodoStrip` (`reference-src/src/modules/ai/components/TodoStrip.tsx:14`) — `Progress` bar + completed/total + per-todo checkbox list.
  - `PlanDiffReview` (`AiMiniWindow.tsx:155`).
- Composer: `reference-src/src/modules/ai/components/AiInputBar.tsx:94`
  - `ContextPillsRow` (`AiInputBar.tsx:622`) — bordered pills: shell/ssh name, `~/…/cwd` (`shortCwd`), git branch. Shown in both modes.
  - `ChipsRow` (`AiInputBar.tsx:652`) — animated chips for picked commands (`#name`), directives (`#handle`), file/image/selection attachments with per-chip remove.
  - AI ⇄ Shell mode toggle (`AiInputBar.tsx:147`) when `terminalComposerEnabled` and a session is active.
  - `+` attach button → hidden `<input type=file multiple accept=…>` (`AiInputBar.tsx:369`, `ACCEPTED_FILES`).
  - Mic button → `useWhisperRecording` (`reference-src/src/modules/ai/hooks/useWhisperRecording.ts`), states recording / transcribing, needs OpenAI key.
  - `<textarea>` with autoresize (`AiInputBar.tsx:775`), placeholder `"Ask Labonair anything · @ files · # directives"`.
  - `#` trigger → `detectDirectiveTrigger` (`AiInputBar.tsx:52`) → `DirectivePickerContent` (commands + user directives, arrow/Tab/Enter/Esc nav).
  - `@` trigger → `detectFileTrigger` (`AiInputBar.tsx:70`) → debounced `fs_search` invoke → `FilePickerContent`, directory drill-down.
  - Enter=send, Shift+Enter=newline, ⌘/Ctrl+Enter while busy = enqueue (`AiInputBar.tsx:476`), "⌘↵ to queue a follow-up" hint + "N queued" badge.
  - `AgentSwitcher` (`reference-src/src/modules/ai/components/AgentSwitcher.tsx:34`) — built-in + custom agents, per-agent icon, "Manage agents" → settings.
  - Stop button while busy, else Send (disabled unless `canSend`).
  - `AiInputBarConnect` (`AiInputBar.tsx:783`) — "Connect any AI provider…" banner + "Add API key" button when no provider configured.
- `ModelPicker` (`reference-src/src/modules/ai/components/ModelPicker.tsx:320`) — `Popover`, 460 px:
  - search box (name/provider/capability/instance), refresh button (spins while fetching).
  - tabs All / Favorites / Recent (+ recent count badge) (`ModelPicker.tsx:604`).
  - left provider rail (`ProviderButton`, `ModelPicker.tsx:119`) — configured providers only, per-provider icon, loading dot, "no key" warning dot, animated active indicator.
  - `ModelRow` (`ModelPicker.tsx:169`) — favorite star, provider icon, label + hint, three `CapabilityGroup` bar meters (Intelligence/Speed/Cost), "no tools" warning badge, selected tick, `opacity-40` when no key.
  - `SkeletonRow` / `ErrorRow` per instance; empty state → "Add a provider in Settings".
- Message rendering (`AiChat.tsx`):
  - Empty state: `ConversationEmptyState` title "Ask Labonair anything" (`AiChat.tsx:67`).
  - User message: parses `LABONAIR_CMD_RE` (`reference-src/src/modules/ai/lib/slashCommands.ts:47`) → `CommandSnippet` chip + remaining prose (`AiChat.tsx:114`).
  - Part types (`AiChat.tsx:159`): `text` → `MessageResponse` (streaming markdown), `reasoning` → `Reasoning`/`ReasoningTrigger`/`ReasoningContent` collapsible (`reference-src/src/components/ai-elements/reasoning.tsx`), `dynamic-tool` / `tool-*` → `RenderedTool`.
  - `RenderedTool` (`AiChat.tsx:179`): `state === "approval-requested"` → `AiToolApproval` card; else `ToolCallChip`.
  - `ToolCallChip` (`reference-src/src/modules/ai/components/ToolCallChip.tsx:264`): per-tool `TOOL_META` icon+label (17 tools), status dot/icon (pending pulse / success tick / error+denied cross), `buildSummary` per tool (filename, truncated command + `exit N`, "N matches in M files", byte counts…), `EXPANDABLE_TOOLS` set → collapsible `ExpandedDetail` (stdout/stderr/exit, file preview first 30 lines, dir listing, grep/glob hits, truncation notices).
  - "Thinking…" `Spinner` row while busy and last msg is user; error banner with Dismiss; `ConversationScrollButton`.
- Status bar entry: `reference-src/src/modules/statusbar/AiTools.tsx:29` — "Open AI Agent ⌘I" pill ⇄ inline `ModelSelector` + mic + submit.
- Slash commands: `/init` (writes LABONAIR.md), `/plan` (plan mode) (`slashCommands.ts:33`).
- Sessions: `reference-src/src/modules/ai/lib/sessions.ts`, `store/chatStore.ts` — favorites, recents, per-session queues, `agentMeta.step`.

### 1b. Rust port current state

- `crates/ui/src/ai_chat.rs` — one file, one `AiChatView` (`ai_chat.rs:510`) rendering header + messages + composer. Backing store `AiChatStore` (`ai_chat.rs:23`) wraps `labonair_ai::SessionStore` and drives send→stream→tool-dispatch.
- Header (`ai_chat.rs:672`): session-title dropdown (`render_session_menu`, `ai_chat.rs:780`), a model "chip" that **cycles to the next model in `MODELS` on click** (`cycle_model`, `ai_chat.rs:603`), a `+` new-session button. No search, tabs, favorites, recents, providers, capability meters, key state.
- Messages (`ai_chat.rs:844`):
  - Empty state text "Ask Labonair anything" (`ai_chat.rs:862`) — matches copy.
  - User bubble (`render_user_message`, `ai_chat.rs:915`): `split_context_blocks` (`ai_chat.rs:1493`) extracts `<selection>/<file>/<image>` labels into `📎 label` lines + prose. **No `<labonair-command>` / CommandSnippet handling.**
  - Assistant (`render_assistant_message`, `ai_chat.rs:950`): custom markdown (`crate::markdown::parse_markdown`) with `render_block` (headings, paragraphs, quotes, rules, bullets, ordered, tables, fenced code with tree-sitter highlight + Copy). Reasoning = one collapsible "▸ Thinking" box (`ai_chat.rs:972`), plain text only. Error box.
  - Tool call (`render_tool_call`, `ai_chat.rs:1050`): monospace tool name + status word (`streaming…`/`awaiting approval`/`done`/`rejected`), raw truncated args (600 chars), raw truncated result, inline Approve/Reject. **No icons, no per-tool label, no summary, no expandable structured detail, no status dots.**
  - Streaming indicator is a header status line only (`run_status_label`, `ai_chat.rs:455`) — no "Thinking…" spinner row, no `ConversationScrollButton`.
- Composer (`render_composer`, `ai_chat.rs:1329`): a `div` with `track_focus` + `on_key_down` that pushes chars one at a time (`on_composer_key`, `ai_chat.rs:612`). Handles Enter/Shift+Enter/Backspace/Esc. Attachment chips with remove (`ai_chat.rs:1347`). Hint text "Enter to send · Shift+Enter for newline". Stop/Send.
  - **Not a real text field**: no caret, no text selection, no mouse cursor positioning, no paste/clipboard-in, no IME, backspace only pops the last char, no undo.
  - **Missing entirely**: ContextPillsRow, ChipsRow for directives/commands, AI⇄Shell toggle, `+` file attach, mic/whisper, `#` directive picker, `@` file picker, ⌘↵ enqueue + queue hint, `AgentSwitcher`, `AiInputBarConnect` banner.
- No `QueueStrip`, `TodoStrip`, `PlanDiffReview`, `PlanModeStrip`, `BoundTabBadge`, `ContextIndicator` equivalents.
- `crates/ui/src/agent_access.rs` is a store-only mirror (`AgentAccessStore`, `agent_access.rs:35`) — no UI, consistent with reference (menu-driven).
- Model catalog `crates/ai/src/config.rs:174` `ModelInfo` has `tags` but **no `capabilities` (intelligence/speed/cost)** the reference `ModelRow` renders.
- Backend coverage is good: `crates/ai/src/tools/registry.rs` advertises the full tool set incl. `run_subagent`; sessions/instances/providers/secret-store all present.

### 1c. Fix recommendations (AI panel)

1. Split `AiChatView` into `AiChatHeader`, `AiMessageList`, `AiComposer`, `QueueStrip`, `TodoStrip` sub-views mirroring `AiMiniWindow.tsx` composition.
2. Build a real text-input primitive first (see §5) and rebuild the composer on it; then port `#`/`@`/mode-toggle/attach/mic/enqueue.
3. Port `ModelPicker` as a GPUI popover: search + All/Favorites/Recent tabs + provider rail + capability meters. Add `capabilities` to `ModelInfo`. Replace the click-to-cycle chip.
4. Port `ToolCallChip`: `TOOL_META` icon/label table, status dot/icon, `buildSummary`, `ExpandedDetail` per tool with the same collapsible bodies.
5. Port `CommandSnippet` + `LABONAIR_CMD_RE` parsing for user messages; port `/init` and `/plan` slash commands + `PlanModeStrip`.
6. Add `QueueStrip` (per-session queue in store) and `TodoStrip` (`TodoStore` already exists in `crates/ai`) with a progress bar.
7. Port `AgentSwitcher` (custom agents live in `crates/ai` `SUBAGENTS`/agents store).
8. Add the "no provider configured" connect banner and the status-bar `AiTools` pill (⌘I).
9. Reasoning: render markdown inside `ReasoningContent`, auto-expand while streaming then collapse (reference behavior).

---

## 2. Host Manager

### 2a. Reference structure

- `reference-src/src/modules/hosts/components/HomeDashboard.tsx:227` — **master/detail**:
  - Left pane: Row 1 search input (`HomeDashboard.tsx:454`) with placeholder "Find a host or type user@hostname to quick-connect…"; parses `user@host:port` → `quickConnectMatch` (`HomeDashboard.tsx:349`) → "Quick Connect" suggestion card (`HomeDashboard.tsx:721`).
  - Row 2 actions toolbar (`HomeDashboard.tsx:468`): split **NEW HOST / NEW CREDENTIAL** button + dropdown (New Host / New Credential / New Group / Import SSH Config / Export SSH Config); grid/list layout toggle; sort dropdown (Last Connected / A–Z / Z–A, with tick); vertical separator; **Hosts / Credentials** view toggle.
  - Groups chip row (`HomeDashboard.tsx:686`): `GroupCard` per group with host count, inline rename, delete-confirm; inline "Group name…" input; horizontal scroll; click filters.
  - Content: skeletons while loading; `EmptyState` "No hosts yet / Add First Host" (`HomeDashboard.tsx:113`); grid (`repeat(auto-fill, minmax(260*cardScale, 1fr))`) of `HostCard` or list of `HostListItem`; **DnD reorder** via `@dnd-kit` (only in Last-Connected order, `HomeDashboard.tsx:403`); per-host ping status `online|offline|checking` (`startPingWorker`, `HomeDashboard.tsx:293`); multi-select (⌘-toggle / shift-range).
  - Auto-refresh 30 s + refetch on window focus.
- Right side-panel 340 px (`HomeDashboard.tsx:930`): `HostFormPanel` (keyed by hostId → full remount per host) or `CredentialFormPanel`.
- `reference-src/src/modules/hosts/components/HostFormPanel.tsx:222`:
  - Header: `HostIconPicker` (`HostFormPanel.tsx:486`), "NEW HOST" / "HOST DETAILS" eyebrow, inline editable name, `SaveStatusIcon` (idle→saving spinner→success/error flash, `HostFormPanel.tsx:186`), options dropdown (Connect SSH / Open SFTP / Test Connection / Duplicate / Delete Host…), close.
  - **Debounced autosave** (1 s, `HostFormPanel.tsx:316`); flush-on-host-switch/unmount (`HostFormPanel.tsx:337`); `MIN_SAVING_DISPLAY_MS` 700.
  - **Test Connection** → `ssh_test_connection` invoke, handles `success` / `unknown_host_key` / `host_key_changed` (`HostFormPanel.tsx:414`).
  - Duplicate-name soft warning (`HostFormPanel.tsx:477`).
  - 4 tabs: **General / SSH / SFTP / Tunnels** (`HostFormPanel.tsx:550`).
  - Full field list (`FormState`, `HostFormPanel.tsx:57`):

| Field | Tab | Control | Rust port? |
|---|---|---|---|
| `name` (Display Name) | General + header | text | ✅ (`name`) |
| `host_address` | General | text | ✅ (`address`) |
| `port` | General | number, w-20 | ✅ (`port`) |
| `username` | General | text | ✅ (`username`) |
| `auth_method` | General | 4 segmented buttons: **Password / SSH Key / Credential / None** | ⚠️ Rust has **Password / SSH Key / Agent / None** — "Credential" renamed to "Agent", semantics diverge |
| `private_key_path` | General (auth=key) | text `~/.ssh/id_rsa` | ✅ (`key_path`) |
| `password` | General (auth=password) | password + "Stored securely in local encrypted store" | ⚠️ present but rendered as an editable bullet-string (`hosts.rs:1515`) |
| `credential_id` | General (auth=credential) | `<select>` of credentials + "+ Create new credential" link + empty hint | ⚠️ Rust shows a "Credential: X" cycle button **always**, not gated on auth method, no create link |
| `group_id` | General | `<select>` None + groups (with icon) | ⚠️ "Group: X" cycle button |
| `pin_to_top` | General | toggle "Always show this host first" | ❌ **missing** |
| `jump_host_id` | General | `<select>` None + other hosts `name (addr:port)` + hint | ⚠️ "Route: X" cycle button (no addr:port in label) |
| `notes` | General | textarea "Notes / Runbook" | ❌ **missing** |
| `icon` | header | `HostIconPicker` (symbol/shape/os/number icon sets) | ❌ **missing** |
| `default_path_ssh` | SSH | text + "runs `cd <path>`" hint | ✅ (`default_path`, labelled "Start directory") |
| `keep_alive_interval` | SSH | number "Keep-Alive Interval (s)" | ❌ **missing** |
| `keep_alive_tries` | SSH | number "Max Tries" | ❌ **missing** |
| `sudo_password` | SSH | password "Sudo Password Autofill", "(set)" placeholder, Keychain hint | ❌ **missing** |
| `block_agent_access` | SSH | toggle "Block AI Agent Access" + explanation | ✅ (`block_agent_access`, as a button) |
| `startup_snippet_id` | SSH | `<select>` None + snippets | ❌ **missing** |
| `startup_snippet_mode` | SSH | Execute / Inject segmented + per-mode hint | ❌ **missing** |
| `default_path_sftp` | SFTP | text "/var/www" + hint | ❌ **missing** |
| `tunnels[]` | Tunnels | local_port → remote_host : remote_port rows, add/remove, empty state | ✅ (`render_tunnels_section`, `hosts.rs:1297`) |
| — | — | — | ➕ Rust adds a **"Tags (comma separated)"** field not in the reference form (`hosts.rs:1531`) |

- New-host mode: "Add Host" button (`HostFormPanel.tsx:769`); edit mode has no save button (autosave + header dropdown).
- Delete → `AlertDialog` confirm (`HostFormPanel.tsx:1083`).
- Other reference components: `HostCard`, `HostListItem`, `GroupCard`, `CredentialCard`, `CredentialListItem`, `CredentialFormPanel`, `HostInspector`, `HostAvatar`, `HostIconGlyph`, `SshConfigImportDialog`, icon sets in `lib/icons/`.
- `connectionStatusStore` (`reference-src/src/modules/hosts/store/connectionStatusStore.ts:36`) — see §4.

### 2b. Rust port current state

- `crates/ui/src/hosts.rs` — `HostManagerView` (`hosts.rs` ~`363`). `render` (`hosts.rs:2123`):
  - Single vertically-scrolling column: toolbar (title "Hosts", New Host, New Group inline input, Credentials, Import SSH config, Export SSH config), optional Active-tunnels panel, then **group blocks** (ungrouped first, then each group) — `render_group_block` (`hosts.rs:1146`), each a list of `render_host_row` (`hosts.rs:1052`).
  - Host row: status glyph, `name`, `user@addr:port · <status>`, buttons Connect / SFTP / Edit / Duplicate / Delete.
  - No search, no quick-connect parsing, no grid/list toggle, no sort, no Hosts/Credentials view toggle (credentials are a separate modal `render_credentials`, `hosts.rs:1668`), no group filter chips (groups are static sections), no ping worker / online-offline status, no DnD reorder, no multi-select, no card scale, no auto-refresh/focus-refetch, no skeletons, no empty-state art.
- Form is a **centered modal overlay** (`render_form`, `hosts.rs:1387`), single flat scroll (no tabs), fields as listed above. `HostForm` struct `hosts.rs:212`.
  - Selects are click-to-cycle buttons (`hosts.rs:1543` credential, `:1564` group, `:1590` jump).
  - Save is an explicit button (`hosts.rs:1660`) → `submit_form`; **no autosave, no save-status indicator**.
  - **No Test Connection, no Duplicate/Connect/Delete in a header menu, no icon picker, no duplicate-name warning.**
- `AuthMethod` enum `hosts.rs:78` — `Password / Key / Agent / None`.

### 2c. Fix recommendations (Host Manager)

1. Rebuild as master/detail: left list pane + a persistent right side-panel (not a modal). Key the panel by host id.
2. Add the actions toolbar: search (with `user@host:port` quick-connect parsing + suggestion card), split NEW button + dropdown, grid/list toggle, sort dropdown, Hosts/Credentials view toggle.
3. Group filter chip row with `GroupCard` (count / rename / delete-confirm) replacing static sections.
4. Port the 4-tab form (General/SSH/SFTP/Tunnels) with **all** missing fields: `pin_to_top`, `sudo_password`, `keep_alive_interval`, `keep_alive_tries`, `default_path_sftp`, `startup_snippet_id` + `startup_snippet_mode`, `notes`, `icon` (+ `HostIconPicker` + icon sets).
5. Rename `AuthMethod::Agent` → `Credential`; only show the credential picker when `auth == Credential`; add "+ Create new credential" link. Use real `<select>`-style dropdowns (see §5).
6. Add debounced autosave (1 s) + `SaveStatusIcon`, flush-on-switch; header options dropdown with **Test Connection** (`ssh_test_connection` result variants), Duplicate, Connect SSH, Open SFTP, Delete.
7. Add ping worker → per-host online/offline/checking dot; DnD reorder (Last-Connected order only); auto-refresh + refetch on focus.
8. Remove the extra "Tags" field from the form (or hide it) to match the reference form surface.

---

## 3. CWD breadcrumbs

### 3a. Reference

- `reference-src/src/modules/statusbar/CwdBreadcrumb.tsx:72` + `reference-src/src/modules/statusbar/lib/pathUtils.ts` + `.test.ts`.
- Behaviors:
  1. **Home collapse**: `segmentsFromCwd(cwd, home)` (`pathUtils.ts:17`) — if cwd is under `home`, first segment is `~` / "Home" with a home icon; else first segment is `/`.
  2. **Directory mode** (`CwdBreadcrumb.tsx:131`): parent segments + a distinct current segment.
  3. **File mode** (`CwdBreadcrumb.tsx:74`, when `filePath` set): dir segments navigate, filename is a non-interactive `BreadcrumbPage` leaf.
  4. **Each parent segment** = a `Badge variant="outline"` button; click → `onCd(seg.fullPath)` (`CwdBreadcrumb.tsx:264`).
  5. **Current segment** = `CurrentSegmentDropdown` (`CwdBreadcrumb.tsx:327`): opens a `DropdownMenu`, lazily `provider.readDir(path)`, lists child directories, click → `onCd(child)`; "Loading…" / "No subfolders" / error states.
  6. **Collapsed middle segments** on narrow widths (`CollapsedSegments`, `CwdBreadcrumb.tsx:396`): a `…` dropdown listing the hidden segments (`md:hidden` / `max-md:hidden` responsive swap).
  7. **Per-segment context menu** (`SegmentExtraActions`, `CwdBreadcrumb.tsx:195`): Copy absolute path, Copy relative path (`relativePath`, `pathUtils.ts:1`), Open in current terminal, Open in new terminal (`onCdInNewTab`), Bookmark / Remove bookmark (when `bookmarksEnabled`), Reference in AI chat (dispatches `labonair:ai-attach-file`). Merged into `BarItemContextMenu` (bar-item reposition/hide) via its `extra` slot.
  8. **Remote target** (`BreadcrumbRemoteTarget = {hostId, sessionId}`): `resolveProvider` (`CwdBreadcrumb.tsx:57`) picks local vs. remote FS provider so the dropdown browses through the *same* SSH session the explorer tree uses. Covered by `CwdBreadcrumb.test.ts`.
  9. Separators are chevron SVGs (`BreadcrumbSeparator`, `[&>svg]:size-3`).
- Rendered from `reference-src/src/modules/statusbar/StatusBar.tsx`.

### 3b. Rust port

- `crates/ui/src/app_shell.rs:1186` inside `render_statusbar`. `cwd.as_deref().map(display_path)` → split on `/`, filter empty, render each as a `div` (last = `fg`, rest = `muted`) joined by `"/"` text. Leading `/` shown if absolute. Falls back to the tab label when no cwd.
- That's the entire implementation. **Not interactive.**
- Missing: home/`~` collapse, click-to-cd, current-segment subdirectory dropdown, `…` collapse, right-click context menu (all 6 actions), file mode, remote provider support, chevron separators, bookmarks integration.
- `pathUtils.ts` (`relativePath`, `segmentsFromCwd`, `Segment`) and `CwdBreadcrumb.test.ts` (`resolveProvider`) **not ported**.
- Also architecturally different: reference lives in a dedicated `statusbar/` module with `BarItemContextMenu` (reposition/hide bar items); the port has no bar-item system.

### 3c. Fix recommendations

1. Create `crates/ui/src/breadcrumb.rs` (or a `statusbar` module). Port `pathUtils` verbatim (`segmentsFromCwd`, `relativePath`) + its unit test.
2. Render segments as clickable pill buttons → emit a `Cd(path)` event the workspace already handles (`connect`/`cd` plumbing exists in `workspace.rs`).
3. Current-segment dropdown: reuse the explorer's directory-listing path (local + `russh-sftp`) keyed by the active pane's session, matching `resolveProvider`.
4. Per-segment right-click context menu with the 6 actions; wire "Reference in AI chat" to `AiChatView::attach_file` (already exists, `ai_chat.rs:567`) and bookmarks to `crates/ui/src/bookmarks.rs`.
5. Add file-mode (editor tab active → dir segments + filename leaf).
6. Narrow-width `…` collapse.

---

## 4. SSH "connecting" screen

### 4a. Reference

- `reference-src/src/modules/terminal/SshLoadingScreen.tsx:57` — a **full-pane** component mounted in place of the terminal until connected.
- State machine `Status` (`SshLoadingScreen.tsx:30`): `quick_connect_password | connecting | waiting_trust | waiting_auth | waiting_passphrase | error`.
- `connecting` view (`SshLoadingScreen.tsx:316`): spinner + **4-stage progress indicator** — SSH: `TCP Connect → Handshake → Auth → Shell`; SFTP: `… → SFTP` (`SSH_STAGES`/`SFTP_STAGES`, `:38`). Current stage derived from streamed log lines by `detectStage` (`:41`). Cancel button.
- Live **"Connection Log"** panel (`SshLoadingScreen.tsx:562`): pulsing dot, monospaced streamed `ssh_connect_log` lines, auto-scroll.
- `quick_connect_password` (`:266`): identity line `user@host:port`, password field, Connect / Cancel.
- `waiting_trust` (`:376`): unknown vs. **mismatch (MITM)** styling, fingerprint (MD5) block, "Trust & Connect" / "Accept anyway" / Abort → `ssh_trust_host`.
- `waiting_auth` (`:435`): prompt message, password field, 2FA-aware (`is_2fa` from `auth_required`); on success saves the entered password to keychain (`secrets_set`, `:209`).
- `waiting_passphrase` (`:480`): encrypted-key passphrase, Unlock / Cancel.
- `error` (`:533`): message + **Retry** / Close.
- On `session_established` (`:205`): saves password if user fixed auth, starts tunnels (`ssh_start_tunnels`), mirrors `last_connected_at` into the hosts store, calls `onConnected`.
- Events consumed: `ssh_connect_log`, `known_hosts_warning`, `auth_required`, `passphrase_required`, `session_established`.
- Animated card transitions (`AnimatePresence mode="wait"`).
- `connectionStatusStore.ts:36` — per-session `ConnectionEntry { sessionId, hostId, kind: terminal|sftp, status: connecting|connected|error, error, jumpHostName, hostLabel (snapshot), workspaceTabId/paneId/sftpTabId, connectedAt }`; `upsert` / `setStatus` / `remove`. Consumed by the status bar and command palette (jump-host badge, reconnect targets).

### 4b. Rust port

- No dedicated screen. `crates/ui/src/workspace.rs:1962` `connect_host` writes `b"Connecting\xe2\x80\xa6\r\n"` into the `RemoteFeed` (`:2004`) and runs `ssh_connect` (`spawn_ssh_connect`, `:2056`).
- Errors: printed as red ANSI text into the terminal feed (`workspace.rs:2104`), then `HostStatus::Failed`.
- Prompts: a single generic modal `render_ssh_prompt` (`workspace.rs:2401`) handling three `SshPrompt` variants — `Trust { host, fingerprint, mismatch }`, `Password { message, is_2fa }`, `Passphrase` — with a title, a text body (bullets for typed chars), Cancel / OK. Uses `crate::theme::modal_scrim()`. No animation.
- Status tracking: `HostStatus { Idle, Connecting, Connected, Failed }` (`crates/ui/src/hosts.rs:31`), surfaced only as a glyph/word in the host row. No `sessionId → entry` map, no `error`, no `jumpHostName`, no `kind`, no nav metadata.
- Missing: the connecting screen itself, 4-stage progress, connection-log surface, quick-connect password screen, distinct unknown-vs-mismatch trust styling beyond the title, Retry button, save-password-to-keychain-on-fix, `last_connected_at` mirror on connect, SFTP-specific stages, `connectionStatusStore` equivalent (→ jump-host badge / palette reconnect).
- 2FA: only reflected in the modal title (`workspace.rs:2438`), no distinct field/help.

### 4c. Fix recommendations

1. Add `crates/ui/src/ssh_loading.rs` — a full-pane view the SSH pane shows until `session_established`, with the 5-state machine and 4-stage progress bar (derive stage from the `ssh_connect_log` stream the backend already emits — check `AppEvent`/`RemoteFeed` for a log channel; add one if absent).
2. Render the live connection log panel from that stream.
3. Add the quick-connect password state (the port supports quick connect via `connect_host` variants — check `spawn_ssh_connect` args).
4. Trust card: distinct MITM styling + fingerprint block; error state: Retry (re-invoke `spawn_ssh_connect`) + Close.
5. On success: `secrets_set` the fixed password, `ssh_start_tunnels`, write `last_connected_at` into the hosts store.
6. Introduce a `ConnectionStatusStore` GPUI entity mirroring `connectionStatusStore.ts` (per-session status/error/jump-host/kind) and consume it in the status bar + command palette.

---

## 5. Shared design / UI primitives

### 5a. Reference

`reference-src/src/components/ui/` — a full primitive set, all token-driven (`cn` + Tailwind + oklch tokens from `globals.css`):

- **`button.tsx`**: `buttonVariants` (cva).
  - variants: `default` (bg-primary), `outline`, `secondary`, `ghost`, `destructive` (destructive/10 tint), `link`.
  - sizes: `default` (h-9), `xs` (h-6 text-xs), `sm` (h-8), `lg` (h-10), `icon` (size-9), `icon-xs` (size-6), `icon-sm` (size-8), `icon-lg` (size-10).
  - base: `rounded-4xl` (pill), `border border-transparent`, `focus-visible:ring-3 ring-ring/30`, `active:not-aria-[haspopup]:translate-y-px`, `disabled:opacity-50 pointer-events-none`, `aria-invalid:*`, auto icon sizing, `has-data-[icon=inline-end/start]` padding.
  - `asChild` via Radix `Slot`; `data-variant` / `data-size` attributes.
- **`input.tsx`**: `h-9 rounded-3xl bg-input/50`, focus ring, `disabled:*`, `aria-invalid:*`, file-input styling.
- Plus: `select.tsx`, `dropdown-menu.tsx`, `dialog.tsx`, `alert-dialog.tsx`, `popover.tsx`, `hover-card.tsx`, `tooltip.tsx`, `tabs.tsx`, `badge.tsx`, `label.tsx`, `switch.tsx`, `checkbox.tsx`, `radio-group.tsx`, `slider.tsx`, `progress.tsx`, `spinner.tsx`, `skeleton.tsx`, `separator.tsx`, `kbd.tsx`, `command.tsx`, `context-menu.tsx`, `menubar.tsx`, `breadcrumb.tsx`, `collapsible.tsx`, `scroll-area.tsx`, `resizable.tsx`, `sheet.tsx`, `card.tsx`, `alert.tsx`, `toggle.tsx` / `toggle-group.tsx`, `button-group.tsx`, `input-group.tsx`, `item.tsx`, `empty.tsx`.
- Every feature component imports these — one consistent visual language (pill buttons, rounded-3xl inputs, one ring treatment, one disabled treatment).

### 5b. Rust port

- **No shared component module.** `crates/ui/src/lib.rs:5` module list has no `widgets`/`components`/`ui`. `gpui-component` (named in `CLAUDE.md` as the intended primitive lib) is **not a dependency of `crates/ui`**.
- Each view hand-rolls its own button helper, all slightly different:
  - `hosts.rs:1025` `fn btn(id, label, p, primary)` → `px_2 py_1 rounded_md text_xs`, primary = `bg(accent)` + `hover opacity 0.85`, secondary = `border` + `hover bg(border)`.
  - `git.rs:1465` `fn tool_btn(...)`.
  - `settings.rs:2303` `fn step_btn(...)`.
  - `workspace.rs:2493`/`2506` — inline ad-hoc button divs for the SSH prompt.
  - `ai_chat.rs` — inline `div().px_2p5().py_1().rounded_sm()...` for Send/Stop/Approve/Reject (yet another style).
- No `rounded-4xl` pill buttons anywhere; the port uses `rounded_md` / `rounded_sm` universally → **direct visual divergence from the reference** (violates Critical Rule #3 "UI values come from the reference").
- No shared text `Input`: every "field" is a focus-tracking `div` with `on_key_down` char-pushing (`hosts.rs` `labelled_field` `:1221`, `ai_chat.rs` composer, `workspace.rs` prompt buffers, host-manager group draft, etc.). No caret, selection, paste, IME, undo, mouse positioning. Passwords rendered as an editable string of `•`.
- No `Select` / `DropdownMenu` primitive — selects are click-to-cycle buttons (host form) or bespoke menus (`ai_chat.rs render_session_menu`, `hosts.rs`).
- No `Tooltip`, `Tabs`, `Switch` (toggles are hand-drawn `div`s), `Badge`, `Kbd`, `Popover`, `Progress`, `Skeleton`, `Spinner`, `Breadcrumb`, `ContextMenu` primitives. `crate::theme::modal_scrim()` is the only shared bit of chrome.
- `crates/theme/src/tokens.rs` has the oklch tokens, but nothing maps them to a variant system.

### 5c. Fix recommendations

1. Create `crates/ui/src/widgets/` with at minimum: `button.rs`, `text_input.rs`, `select.rs`, `dropdown_menu.rs`, `tooltip.rs`, `tabs.rs`, `switch.rs`, `checkbox.rs`, `badge.rs`, `kbd.rs`, `progress.rs`, `spinner.rs`, `skeleton.rs`, `context_menu.rs`, `popover.rs`, `dialog.rs` (+ `alert_dialog.rs`).
2. `Button`: an enum `ButtonVariant { Default, Outline, Secondary, Ghost, Destructive, Link }` × `ButtonSize { Default, Xs, Sm, Lg, Icon, IconXs, IconSm, IconLg }`, driven off `ThemeStore` tokens, `rounded-4xl` radius, the reference's focus-ring / disabled / active-translate treatment. Replace all `btn`/`tool_btn`/`step_btn`/inline buttons.
3. `TextInput`: adopt `gpui-component`'s `Input`/`TextInput` (add the dependency the project already declares intent for) or GPUI's editor primitive — a real field with caret/selection/paste/IME. Rebuild the AI composer, all host-form fields, SSH prompts, search boxes on it.
4. `Select`/`DropdownMenu`: a real popover list; replace every click-to-cycle button and bespoke menu.
5. Pull radii/spacing/ring values 1:1 from `reference-src/src/components/ui/*` + `globals.css` (Critical Rule #3).

---

## 6. Prioritized fix list

| # | Area | Item | Effort | Priority |
|---|---|---|---|---|
| 1 | §5 | `TextInput` primitive (real caret/selection/paste/IME) | L | **P0 — blocks 2, 3, 6, 7** |
| 2 | §5 | `Button` variant/size system; replace all ad-hoc `btn` helpers; `rounded-4xl` | M | **P0** |
| 3 | §5 | `Select` / `DropdownMenu` / `Tooltip` / `Switch` / `Tabs` / `Badge` / `Progress` / `Spinner` / `Skeleton` primitives | L | **P0** |
| 4 | §3 | Port `pathUtils` (+test); interactive breadcrumb: home-collapse, click-to-cd, current-segment subdir dropdown, `…` collapse | M | **P1** |
| 5 | §3 | Breadcrumb per-segment context menu (copy abs/rel, open in current/new terminal, bookmark, reference in AI chat); file mode; remote provider | M | **P1** |
| 6 | §4 | `ssh_loading.rs` full-pane connecting screen: 5-state machine + 4-stage progress + live connection-log panel | L | **P1** |
| 7 | §4 | `ConnectionStatusStore` entity (per-session status/error/jump-host/kind) + status-bar/palette consumers; Retry; save-password-on-fix; `last_connected_at` mirror | M | **P1** |
| 8 | §2 | Host Manager → master/detail + persistent side panel; actions toolbar (search + quick-connect, split NEW, grid/list, sort, Hosts/Credentials toggle); group filter chips | L | **P1** |
| 9 | §2 | Host form → 4 tabs + missing fields: `pin_to_top`, `sudo_password`, `keep_alive_interval`, `keep_alive_tries`, `default_path_sftp`, `startup_snippet_id`+`mode`, `notes`, `icon` (+ `HostIconPicker`) | L | **P1** |
| 10 | §2 | Fix auth taxonomy (`Agent`→`Credential`, gate credential picker on it, "+ Create credential"); debounced autosave + `SaveStatusIcon`; header dropdown with **Test Connection** / Duplicate / Connect / SFTP / Delete; drop stray "Tags" field | M | **P1** |
| 11 | §1 | AI composer rebuild on new `TextInput`: `#` directives, `@` file picker, `+` attach, mic/whisper, AI⇄Shell toggle, ⌘↵ enqueue + hint, `ContextPillsRow`, `ChipsRow`, connect banner | L | **P1** |
| 12 | §1 | `ToolCallChip` parity: per-tool icon/label, status dot, `buildSummary`, expandable `ExpandedDetail` bodies | M | **P2** |
| 13 | §1 | `ModelPicker` popover: search + All/Favorites/Recent tabs + provider rail + capability meters; add `capabilities` to `ModelInfo`; drop click-to-cycle | M | **P2** |
| 14 | §1 | `QueueStrip`, `TodoStrip` (+ progress bar), `PlanModeStrip`/`PlanDiffReview`, `AgentSwitcher` | M | **P2** |
| 15 | §1 | `CommandSnippet` + `LABONAIR_CMD_RE` in user messages; `/init` + `/plan` slash commands; markdown-in-reasoning + auto-expand-while-streaming | S | **P2** |
| 16 | §2 | Per-host ping status (online/offline/checking), DnD reorder, auto-refresh + focus-refetch, skeletons, empty-state art, multi-select | M | **P3** |
| 17 | §1 | Status-bar `AiTools` pill (⌘I) + inline model/mic/submit | S | **P3** |
| 18 | §2 | Grid/list `HostCard` vs `HostListItem`, `GroupCard` rename/delete-confirm/count, credential cards/list, card-scale | M | **P3** |
