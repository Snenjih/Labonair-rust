# Vim BackQuotes Text Object Classification

**Status:** Static action, shared binding, text-object, surround, and Helix routes classified; duplicate-listener effect and native acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Editor owns text-object ranges, selections, and edits; Keymap owns profile and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers `vim::BackQuotes`. It has no
binding in the macOS, Linux, or Windows platform-default maps. The shared
`vim.json` profile contains one active backtick binding for Vim object,
change-surround, and Helix directional contexts. Registered Vim actions also
have a static focused-Editor Command Palette route while Vim/Helix is enabled.
This establishes registration and profile data; macOS registration, active
profile loading, native dispatch, and text-object interaction remain pending.

The dedicated BackQuotes action is separate from `vim::AnyQuotes`, which
searches single, double, and backtick delimiters, and `vim::MiniQuotes`, which
has its own active `q` binding and fallback behavior. The shared backtick key
does not imply that those actions are interchangeable.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-BackQuotes](registered-actions-unbound.md) | Registered action for a backtick text object; it acts under pending Vim object/surround or Helix directional state. Plain Normal-mode palette invocation has no pending range and is a no-op. | `labonair-editor` has no BackQuotes object, backtick range scanner, or surround operator route. |
| [KEYBIND-VIM-HELIX-9f3ee8930dea](keymap-overlay-requirements.md) | The shared layer binds backtick in `vim_operator == a || vim_operator == i || vim_operator == cs || vim_operator == helix_next || vim_operator == helix_previous`; it has no platform-default binding. | Labonair Keymap has no shared Vim/Helix profile layer. Linux and Windows dispatch remain separate runtime acceptance checks. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate the shared layer and Vim palette namespace; there is no BackQuotes-specific setting. | Labonair has partial Editor-owned Vim mode and no Helix mode or matching profile loader. |

## Pinned behavior

`Object::BackQuotes` searches for backtick markers in the current line. Its
range implementation calls the shared `surrounding_markers` scanner with
backtick as both opening and closing marker and disables cross-line search.
The scanner ignores a marker immediately preceded by `\`; it does not parse
language string syntax or calculate escape parity. An unmatched pair yields
no range and no error message.

Inside selection excludes both delimiters and keeps the body as written.
Around selection includes both delimiters and may absorb adjacent horizontal
whitespace after the closing delimiter. If there is no trailing whitespace,
it extends before the opening delimiter instead. Newlines are never included
by this whitespace expansion. An empty pair can produce an empty inside
range. The text-object range branch does not use its numeric `times` argument.

The object is single-line, expands selection in both directions, and targets
Visual mode (preserving Visual Block when invoked there). In Vim, the shared
key acts after inside/around object operators. In `cs` change-surround state,
BackQuotes identifies and anchors the old backtick pair; the user then supplies
the replacement delimiter through the separate change-surround step. A
missing valid old pair leaves no replacement anchors.

The Helix adapter maps BackQuotes to `ImmediateBoundary::BackQuotes`, which
supports current, next, and previous range methods. The active keymap contexts
use only Helix next/previous for this action. Boundary scans remain literal
character scans and do not consult a language quote parser.

The focused Editor's Vim action namespace exposes the action to the Command
Palette when Vim/Helix is enabled. In ordinary Normal mode without a pending
object, change-surround, or Helix directional operator, the object handler has
no work to perform. The palette entry is not a separate command that chooses
an arbitrary quote pair.

### Duplicate handler registration in the pinned source

`object::register` contains two identical `Vim::action` registrations for
`&BackQuotes`. Each call uses `Editor::register_action`, which allocates a
distinct action subscription and installs a Bubble-phase listener for that
action type. The source therefore defines two listeners; the user-visible
effect across the different operator and selection modes has not been
confirmed. No handler was removed or changed in the frozen reference. Resolve
and characterize this anomaly before treating a single dispatch as accepted
behavior.

The inspected `test_anyquotes_object` fixture includes backtick cases, but it
binds and exercises `vim::AnyQuotes`, not `vim::BackQuotes`. The surrounding
character fixture table omits backticks, and no focused BackQuotes or Helix
BackQuotes test was found. Source fixtures were inspected but not run.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; Keymap owns optional
profile/user bindings and Settings owns mode values. The existing Command
Palette remains the discovery surface. No event, new registry, setting,
notification, shell condition, or persistent selection state is required.

`crates/editor/src/vim.rs` implements basic Vim modes and motion operators but
has no text-object state, BackQuotes action, backtick scanner, `cs` workflow,
or Helix directional selector. Local Editor, Keymap, and Settings sources
contain no BackQuotes implementation or active shared mode layer. The
canonical local Editor action and range boundary must be designed with the
text model and Vim/Helix owner contract; the pinned duplicate registration is
an evidence item, not a requirement to create duplicate local handlers.

No dedicated local setting is needed for quote selection. Markers and
selection are transient; edits performed by a change/delete operator continue
through the Editor's normal document transaction and persistence lifecycle.

## Acceptance still required

- Verify the shared backtick binding in its Vim `a`, `i`, `cs`, Helix next, and
  Helix previous contexts, and confirm it does not dispatch in excluded
  contexts. Verify mode-off behavior and custom-binding conflict/save/reload.
- Repeat action dispatch separately on macOS, Linux, and Windows. Confirm
  focused-Editor palette visibility and that plain Normal-mode invocation
  without pending object state leaves selection and text unchanged.
- Check inside/around results at every delimiter position, with empty bodies,
  escaped delimiters, multiple backtick pairs, missing opening/closing
  markers, adjacent horizontal whitespace, line breaks, and long lines.
- Verify Vim change/delete/yank and Visual-object use, plus `cs` old-pair
  selection and replacement. Confirm Vim quote, AnyQuotes, and MiniQuotes
  routes remain distinct.
- Verify Helix current/next/previous adapter behavior and selection movement.
- Instrument or otherwise observe one dispatched BackQuotes action in each
  relevant mode to determine the effect of the two registered listeners.
  Record this as a pinned-source anomaly until its effect is verified.
- Capture paired native macOS Editor and Keymap evidence with platform,
  profile, mode setting, pending operator, selection, viewport, theme, and
  scale recorded.
- Implement and test only through `labonair-editor` and the canonical Keymap
  profile contract after the owner text-object and Helix-selection contracts
  are available.

## Source records inspected

- Action declaration, registrations, object flags, range dispatch, marker
  scanner, and fixtures: `zed-refrence/zed/crates/vim/src/object.rs`.
- Registered handler subscription and Vim action namespace:
  `zed-refrence/zed/crates/vim/src/vim.rs` and
  `zed-refrence/zed/crates/editor/src/editor.rs`.
- Normal object and change-surround routing:
  `zed-refrence/zed/crates/vim/src/normal.rs` and
  `zed-refrence/zed/crates/vim/src/surrounds.rs`.
- Helix BackQuotes adapter and boundary scanner:
  `zed-refrence/zed/crates/vim/src/helix/object.rs` and
  `zed-refrence/zed/crates/vim/src/helix/boundary.rs`.
- Shared keymap and Linux action registration evidence:
  `zed-refrence/zed/assets/keymaps/vim.json` and
  `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Labonair owner audit: `crates/editor/src/vim.rs`,
  `crates/editor/src/editing.rs`, `crates/keymap/src`, and
  `crates/settings/src`.

Same-session pinned-source research and independently written requirements
are redevelopment, not separated-room clean-room implementation. This packet
records static evidence and does not claim runtime acceptance.
