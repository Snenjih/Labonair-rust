# Native Visual Capture Runbook

**Status:** Normative
**Version:** 1
**Owner:** R07 acceptance and surface owners

This is the operator protocol for closing the native visual matrix. It does
not add visual states: [`../product/surfaces.toml`](../product/surfaces.toml)
is the only state authority, and
[`visual-evidence.toml`](visual-evidence.toml) is the only evidence registry.
The runbook explains how to prepare, inspect, capture, and record one state at
a time on a supported macOS host.

## Preconditions

- Use a supported macOS host with a graphical window session.
- Grant Screen Recording to the terminal or runner executing
  [`../../scripts/screenshot.sh`](../../scripts/screenshot.sh). Grant
  Accessibility separately when keyboard or pointer interaction is required.
- Use a disposable application profile or fixture data. Never enter real
  passwords, private keys, bearer tokens, host addresses, or other secrets.
- Build the exact native bundle from the commit being evidenced. Do not use
  `open -a Labonair`, the legacy installed application, a reference renderer,
  or a generic frontmost-window screenshot.
- Capture one catalog state per artifact. If the state cannot be produced or
  cannot be inspected truthfully, leave it `Pending` and record the blocker.

## Capture protocol

1. Check out the commit under review and build the bundle:

   ```text
   scripts/package-macos.sh
   ```

2. Launch the exact executable directly and retain its PID:

   ```text
   BUNDLE_BINARY="$PWD/target/release/bundle/macos/Labonair.app/Contents/MacOS/labonair"
   "$BUNDLE_BINARY" &
   RUST_PID=$!
   ```

   The process must remain alive while the state is prepared. The screenshot
   helper validates the PID and resolves a layer-0 window before capturing.

3. Set the exact catalog identifiers and resolve the deterministic artifact:

   ```text
   SURFACE_ID="workspace.active"
   STATE="home"
   COMMIT="$(git rev-parse HEAD)"
   ARTIFACT="$(python3 scripts/visual_capture_path.py \
     --surface "$SURFACE_ID" --state "$STATE" --commit "$COMMIT")"
   ```

   The path helper rejects unknown states, wrong platform identifiers, invalid
   commit identifiers, and a non-`narrow` viewport for the `narrow window`
   state. Do not rename its output.

4. Prepare the state using the matrix below. Inspect the complete rendered
   surface, including focus, clipping, anchoring, loading/error copy, and
   adjacent shell zones. The preparation action is not evidence by itself.

5. Capture the exact PID:

   ```text
   scripts/screenshot.sh "$ARTIFACT" "$RUST_PID"
   ```

6. Inspect the PNG on the supported host. Reject it when the expected state
   is not visible, another application is present, secrets are visible, or
   the layout is clipped or incorrectly anchored. Keep the artifact below
   `artifacts/visual/` only after inspection.

7. Add a matching `[[capture]]` record to
   [`visual-evidence.toml`](visual-evidence.toml), change exactly that state
   to `Verified`, and retain the artifact in the same commit-bound path. The
   record must include the exact executable path, Rust PID, layer-0 window ID,
   platform, viewport, capture date, and commit. A temporary `/tmp` image,
   terminal log, or process-only launch is not a durable capture.

8. Regenerate and run the scoped gates before moving to the next state:

   ```text
   python3 scripts/gen_visual_evidence.py
   python3 scripts/verify.py --scope visual
   python3 scripts/check_documentation.py
   ```

   At the end of the session run `python3 scripts/verify.py --scope all` and
   update the R07 task handoff with the host, commit, artifact count, and any
   unavailable states.

## State preparation and review matrix

The actions below are bounded preparation guidance, not alternate state
definitions. Use the canonical command/surface owner and a disposable
fixture. If a proposed action does not produce the named state in the actual
application, do not force it with a mock overlay; leave the cell pending and
create a bounded task for the owner.

### `shell.titlebar` — Titlebar

Source: [`../product/surfaces.toml`](../product/surfaces.toml),
[`../../crates/shell/src/titlebar.rs`](../../crates/shell/src/titlebar.rs).

| State | Preparation | Review assertion |
|---|---|---|
| `normal` | Launch with one active workspace tab. | Titlebar, tab strip, global-menu trigger, and window controls render without overlap. |
| `focused tab` | Open at least two tabs and focus one through the canonical tab interaction. | The selected tab has the intended focus/selection treatment and neighboring tabs remain legible. |
| `multiple tabs` | Create multiple terminal/tool tabs through registered commands. | Tab ordering, labels, close affordances, and overflow behavior remain coherent. |
| `narrow window` | Resize the native window to the catalog narrow viewport. | Titlebar content remains usable; no permanent control is clipped or silently duplicated. |
| `zen mode` | Invoke the canonical `ToggleZenMode` command. | Only the documented shell chrome changes and the active content remains usable. |

### `shell.global-menu` — Global Menu

Source: [`../product/menu.toml`](../product/menu.toml),
[`../../crates/shell/src/menu.rs`](../../crates/shell/src/menu.rs).

| State | Preparation | Review assertion |
|---|---|---|
| `closed` | Launch or dismiss the global menu. | The menu is absent and does not reserve an unexplained overlay region. |
| `open` | Activate the titlebar global-menu button. | The menu opens at the trigger, stays inside the viewport, and exposes only catalog entries. |
| `disabled item` | Open the menu in a context with a known unavailable command, such as no project/editor context. | The disabled row is visibly disabled and cannot execute a feature handler. |
| `submenu` | Open a catalog branch such as View or Connections. | The submenu anchors to its parent, flips/clamps correctly, and preserves owner labels/actions. |
| `keyboard navigation` | Focus the menu and navigate with the documented keymap, without relying on pointer hover. | Highlight, submenu traversal, activation, and Escape dismissal are visible and deterministic. |

### `workspace.active` — Active Workspace

Source: [`../product/surfaces.toml`](../product/surfaces.toml),
[`../../crates/workspace/src/workspace.rs`](../../crates/workspace/src/workspace.rs).

| State | Preparation | Review assertion |
|---|---|---|
| `home` | Start with no project selected and the normal home/empty workspace route. | The home state is intentional, unclipped, and exposes the documented entry actions. |
| `project workspace` | Open a disposable local fixture repository through `OpenProject`. | Project identity, active tab, docks, and statusbar agree on the selected root. |
| `standalone` | Use `ReturnToStandalone` or create a standalone terminal/tool tab. | No stale project identity or project-only control remains. |
| `empty pane` | Close the active leaf/tab through the canonical close path while retaining the workspace. | The empty pane state is rendered by Workspace and offers only valid recovery actions. |
| `loading` | Trigger a real asynchronous project/session restore using a local fixture. | Loading indicator and disabled/actionable controls reflect actual in-flight work. |
| `error` | Use a disposable invalid/inaccessible project fixture. | The error is actionable or routed to Notifications; no raw path/secret is leaked. |
| `split panes` | Invoke `SplitRight` and `SplitDown` on a disposable workspace. | Split handles, focus, resizing, and close behavior preserve the layout tree. |

### `workspace.docks` — Registered Docks and Panels

Source: [`../../crates/workspace/src/dock.rs`](../../crates/workspace/src/dock.rs),
[`../../crates/panel/src/dock.rs`](../../crates/panel/src/dock.rs).

| State | Preparation | Review assertion |
|---|---|---|
| `left dock` | Open a registered panel configured for the left dock. | The dock owns placement and the panel uses shared UI-kit controls. |
| `right dock` | Open a registered panel configured for the right dock. | Right-dock width, focus, and close controls remain inside the workspace bounds. |
| `bottom dock` | Open a registered panel configured for the bottom dock. | Bottom-dock resizing does not cover the statusbar or active content. |
| `empty dock` | Close/hide all panels in one dock using canonical controls. | Empty dock chrome is either absent or intentionally represented by the contract. |
| `focused panel` | Focus a panel and then move focus to its adjacent workspace pane. | Focus ring, keyboard routing, and panel actions follow the active owner. |
| `narrow window` | Resize the window to the catalog narrow viewport with a dock open. | Dock collapse/overflow behavior is deterministic and no panel is silently duplicated. |

### `workspace.statusbar` — Statusbar

Source: [`../../crates/workspace/src/status_bar.rs`](../../crates/workspace/src/status_bar.rs),
[`../../crates/workspace/src/status_items.rs`](../../crates/workspace/src/status_items.rs).

| State | Preparation | Review assertion |
|---|---|---|
| `normal` | Launch a normal workspace with standard registered status items. | Left controls and right information items occupy the documented zones. |
| `hidden item` | Hide one optional status item through its canonical owner/settings path. | Only the selected item disappears; shell-wide feature state is not duplicated. |
| `right-click menu` | Open the context menu on a status item. | The menu anchors to the item and contains only valid owner-registered actions. |
| `narrow window` | Resize with status items present. | Items collapse or hide according to the documented rule without clipping. |
| `unread notification badge` | Publish a safe synthetic notification through the notification fixture path. | Badge count/read state is visible and is owned by the notification registry. |

### `overlay.command-palette` — Command Palette

Source: [`../../crates/command-palette/src/palette.rs`](../../crates/command-palette/src/palette.rs),
[`../product/commands.toml`](../product/commands.toml).

| State | Preparation | Review assertion |
|---|---|---|
| `focused` | Invoke `OpenCommandPalette` and leave focus in its input. | Input focus, caret, result ownership, and modal anchoring are clear. |
| `empty` | Enter a query with no matching catalog command. | The empty-result state is explicit and does not invent an executable action. |
| `filtered` | Enter a query matching a known owner command. | Ranking, labels, icons, and keybinding hints match the command catalog. |
| `submenu` | Open an owner-contributed dynamic submenu. | Submenu actions cross the typed palette contract and anchor within the viewport. |
| `keyboard navigation` | Navigate results and submenus with the keymap. | Selection, activation, and dismissal work without pointer-only behavior. |
| `dismissed` | Press Escape or invoke the documented dismissal path. | The overlay and focus trap are removed without leaving stale dimming or state. |

### `workspace.dialogs` — Dialogs and Decision Overlays

Source: [`../../crates/workspace/src/modal_layer.rs`](../../crates/workspace/src/modal_layer.rs),
[`../../crates/shell/src/modals.rs`](../../crates/shell/src/modals.rs).

| State | Preparation | Review assertion |
|---|---|---|
| `open` | Trigger a documented project/settings/feature decision dialog. | Scrim, bounds, title, actions, and focus target are visible and correctly owned. |
| `focused input` | Focus a text/input control inside the dialog. | Keyboard focus is visible and typing does not leak to the underlying workspace. |
| `validation error` | Submit a deliberately invalid value in a disposable fixture. | The field-level correction path is clear and no passive duplicate error surface appears. |
| `confirm/cancel` | Open a decision dialog and inspect both actions without changing user data. | Confirm and cancel have distinct, safe behavior and dismiss/retain state correctly. |
| `loading` | Trigger a real asynchronous dialog-owned operation. | Action controls reflect in-flight state and do not allow duplicate submissions. |
| `failure` | Use a disposable fixture that deterministically fails the operation. | Failure is redacted, actionable, and routed through the documented owner/notification path. |

### `workspace.hosts-tab` — Hosts Management Tab

Source: [`../../crates/hosts-ui/src/hosts.rs`](../../crates/hosts-ui/src/hosts.rs),
[`../security/remote-access.md`](../security/remote-access.md).

| State | Preparation | Review assertion |
|---|---|---|
| `host list` | Open Hosts with a synthetic, non-secret saved-host fixture. | List rows, selection, actions, and empty/loading transitions use the Hosts owner. |
| `empty` | Open Hosts in a disposable data directory with no saved hosts. | Empty actions are explicit and no host/credential from another profile appears. |
| `host form` | Start a new host form without entering real credentials. | Form fields, scopes, and save/cancel actions follow the Hosts contract. |
| `validation error` | Submit an invalid synthetic host/port value. | Validation identifies the field and never writes partial or secret data. |
| `active tunnels` | Use a safe loopback fixture only if the tunnel registry reports an active tunnel. | Tunnel status/actions are owner-registered and do not expose credentials. |
| `loading` | Trigger a real Hosts reload or connection-list operation with fixture data. | Loading state is truthful; if no observable loading phase exists, keep the cell pending. |

### `statusbar.notifications` — Notification Dropdown

Source: [`../../crates/notifications/src/status_item.rs`](../../crates/notifications/src/status_item.rs),
[`../../crates/notifications-core/src/lib.rs`](../../crates/notifications-core/src/lib.rs).

| State | Preparation | Review assertion |
|---|---|---|
| `closed` | Publish a safe fixture notification and leave the dropdown closed. | Only the statusbar badge/item indicates unread state; no toast appears. |
| `unread badge` | Publish one or more safe fixture messages. | Badge count and severity are visible without exposing fixture secrets. |
| `open list` | Activate the notification status item. | Rows use shared UI-kit presentation and list ordering/deduplication is clear. |
| `empty` | Clear the disposable notification registry. | Empty copy is explicit and no stale row remains. |
| `expanded details` | Expand a fixture notification with structured detail text. | Detail expansion is bounded, readable, and does not render hidden payloads. |
| `clear all` | Use the dropdown clear action on fixture messages. | All fixture rows leave the registry and read/badge state updates consistently. |

### `workspace.settings` — Settings Window

Source: [`../../crates/settings-ui/src/window.rs`](../../crates/settings-ui/src/window.rs),
[`../settings/persistence.md`](../settings/persistence.md).

| State | Preparation | Review assertion |
|---|---|---|
| `normal` | Open Settings with a valid disposable settings profile. | Navigation, values, scopes, and shared controls render without capability-owned categories. |
| `scope navigation` | Move between app/workspace/host/session scopes where the catalog permits. | Scope changes are explicit and the effective value source remains understandable. |
| `search/filter` | Search for a known setting and then a no-match query. | Results are bounded, labels/path context are preserved, and no field is invented. |
| `validation error` | Enter an invalid typed value and submit in the disposable profile. | Error state preserves the last valid value and exposes a corrective action. |
| `save failure` | Use a disposable read-only or permission-denied settings fixture. | Write failure is visible, redacted, and does not claim persistence. |

### `workspace.standalone-terminal` — Standalone Terminal

Source: [`../../crates/workspace/src/views/terminal.rs`](../../crates/workspace/src/views/terminal.rs),
[`../../crates/terminal/src`](../../crates/terminal/src).

| State | Preparation | Review assertion |
|---|---|---|
| `ready` | Open a local terminal tab and wait for the prompt. | Terminal input/output, focus, status, and shell integration are ready. |
| `loading` | Capture only if the actual PTY/session startup visibly exposes a loading state. | Loading copy and controls represent real startup; otherwise keep pending. |
| `empty` | Open a terminal state before output or clear it through the canonical command. | Empty presentation is intentional and does not imply a failed process. |
| `process exit` | Run a disposable command such as `exit` in a fixture shell. | Exit state, restart/close actions, and retained output are truthful. |
| `connection failure` | Use a synthetic invalid SSH/connection fixture without credentials. | Failure is redacted and actionable; no real network secret is used. |
| `narrow window` | Resize the native window while the terminal is ready. | Grid, prompt, statusbar, and tab controls resize without clipping or input loss. |

## Recording contract

For each accepted state, the matching `[[capture]]` record must use the exact
path produced by `scripts/visual_capture_path.py` and the exact commit under
review. Keep the state and capture record in the same change. A valid record
has this shape (replace every placeholder with observed values):

```toml
[[capture]]
surface_id = "workspace.active"
state = "home"
status = "Verified"
artifact = "artifacts/visual/workspace-active--home--standard--macos--<commit>.png"
platform = "macos"
viewport = "standard"
binary = "/absolute/path/to/Labonair.app/Contents/MacOS/labonair"
pid = "<observed-rust-pid>"
window = "<observed-layer-0-window-id>"
captured_at = "YYYY-MM-DD"
commit = "<lowercase-commit>"
```

The checker then requires the artifact to exist and rejects a `Verified` or
`Partial` state without its matching capture record. Do not weaken that check
to accommodate an unavailable host; use `Pending` and document the blocker in
the active R07 task instead.
