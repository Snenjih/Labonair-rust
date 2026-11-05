# Vim AnyQuotes Text Object Classification

**Status:** Static action, literal matching, and alternate routes classified;
native Vim/Helix acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned Vim owns modal object dispatch; Editor owns ranges, selection,
and edits; Keymap owns profile and user bindings; the Command Palette exposes
registered Editor actions. Labonair's Editor module owns future quote-object
behavior and Keymap owns any optional user binding.

## Evidence boundary

The Linux runtime registers `vim::AnyQuotes`. It has no binding in the
platform-default maps or an active keymap layer. The shared `vim.json` profile
contains a commented `q` candidate for AnyQuotes; the active `q` binding is
`vim::MiniQuotes`. The comment is not an active binding row.

With Vim or Helix enabled and an Editor focused, the pinned Command Palette
exposes the registered Vim action and restores Editor focus before dispatch.
The action also participates in text-object and change-surround flows if a
binding invokes it. Static source confirms these routes; macOS registration,
profile loading, palette invocation, and custom user binding have not been
verified at runtime.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-AnyQuotes](registered-actions-unbound.md) | Editor-owned text object that searches single, double, and backtick quotes; it is available in the focused Editor's Command Palette when Vim/Helix is enabled. A pending object operator, Visual object action, or change-surround flow supplies its use. | `labonair-editor` has no AnyQuotes action or quote range scanner. The typed local palette has no AnyQuotes descriptor. |
| [KMP-MODE-VIM-HELIX](keymap-overlay-requirements.md) | The shared profile's commented `q` candidate is inactive; active `q` invokes MiniQuotes. | Labonair has no Vim text-object command or selected profile overlay; its Vim support is partial. |
| [SET-keymap-vim-mode](settings-requirements.md), [SET-keymap-helix-mode](settings-requirements.md) | User-scoped false-default mode settings control the shared mode layer and palette Vim namespace. They do not activate the commented AnyQuotes binding. | Labonair has no Helix mode or AnyQuotes setting; its Editor-owned Vim preference is a partial counterpart. |

## Pinned behavior

AnyQuotes is a literal delimiter object over three same-character pairs:
single quote (`'`), double quote (`"`), and backtick (`` ` ``). It does not
include vertical bars, which have a separate object, or use language bracket
queries as AnyPair does. For each delimiter type, the scanner searches the
current line and ignores a marker whose immediately preceding character is a
backslash. This is character matching, not string-literal parsing; quote
syntax and language escapes are not interpreted.

The range search first considers enclosing candidates and returns the
smallest. If the cursor is not within one, it falls back to the candidate
nearest the cursor. As a result, delimiters from separate quote literals can
form a pair around intervening text: the pinned fixture places the cursor in
`false string` between `'first'` and `'second'`, and an inside change selects
between the intervening apostrophes. This is an observable lexical behavior,
not a parser-based quote match.

The object is single-line, always expands selection in both directions, and
targets Visual mode. Inside selection excludes the quote markers and keeps
the body whitespace. Around selection includes the markers and the scanner
also absorbs adjacent non-newline whitespace after the closing quote; if no
such trailing whitespace exists, it extends before the opening quote instead.
An empty body produces an empty inside range and needs native characterization
for Visual selection and edit operators. The object range branch ignores its
numeric `times` argument.

The action can supply a source pair to `cs` change-surround: once invoked in a
change-surround context, the handler locates a single, double, or backtick
pair and continues to the replacement delimiter. The pinned unit fixture
manually binds `q` in `a`, `i`, and `cs` contexts and covers single/double/
backtick selection, nested quote types, delete/change inside and around, and
unmatched quote no-ops. Another fixture exercises changing between all three
quote types while retaining the body spacing.

The commented `q` candidate is in a shared profile context containing Vim
inside/around/change-surround and Helix next/previous operators. AnyQuotes has
no `HelixTextObject` adapter. If explicitly bound into Helix next/previous
contexts, the directional selector receives the unsupported-object error and
does not update selection. The active MiniQuotes `q` binding is a separate
action and is not evidence that AnyQuotes is bound.

The Command Palette exposes the action, but invocation in plain Normal mode
without a pending object cannot select a quote pair. A pending object or
Visual path must consume the range. Missing a complete quote pair is silent;
the operator performs no text edit. Line boundaries stop matching and cannot
be crossed by this object.

## Labonair owner and current route

`crates/editor/src/vim.rs` implements basic Editor-owned Vim modes, motions,
and d/c/y motion operators, but has no text-object state, AnyQuotes action, or
quote scanner. `crates/workspace/src/views/editor.rs` forwards input when the
Editor-owned `editor.vimMode` preference is enabled. There is no Helix mode or
selected base-profile overlay loader.

The local command palette consumes typed `CommandId` contributions; neither
Editor nor Keymap registers AnyQuotes. The canonical future behavior belongs
to `labonair-editor`; optional profile/user binding belongs to
`labonair-keymap`, and discoverability uses the existing palette contract.
No new setting, notification, cross-module event, shell route, or selection
persistence is implied.

## Acceptance still required

- Verify the palette entry with Editor focus and Vim/Helix enabled, focus
  restoration, and native action dispatch on macOS. Repeat profile/dispatch
  checks on Linux and Windows.
- Confirm no platform default or active built-in profile binds AnyQuotes;
  active `q` continues to invoke MiniQuotes. Exercise user overrides only
  after their parsing, precedence, conflict, save, and reload behavior is
  available.
- Test single, double, and backtick pairs; nested mixed quote types; escaped
  delimiters and backslash parity; empty/missing/malformed pairs; pairs split
  across lines; cursor positions on markers and in bodies; enclosing-pair
  precedence; nearest-candidate fallback; and the separate-literal case.
- Verify inside and around whitespace boundaries, Visual mode transitions,
  cursor/selection direction, count behavior, pending operator cleanup,
  change/delete/yank and other supported object consumers, undo/redo, and
  registers.
- Exercise `cs` source-pair selection and replacement among all three quote
  families; verify spacing and cursor placement. Confirm a Helix next/previous
  AnyQuotes binding has no selection effect because there is no Helix adapter.
- Capture paired native macOS palette/editor behavior with platform, mode
  setting, profile, viewport, theme, scale, quote text, and selection state.
  Keep the behavior in Editor and binding state in Keymap.

## Source records inspected

- Action declaration, registration, quote-range search, and range mode:
  `zed-refrence/zed/crates/vim/src/object.rs`.
- Quote pair set, change-surround routing, and pinned fixtures:
  `zed-refrence/zed/crates/vim/src/surrounds.rs`.
- Helix text-object adapter coverage:
  `zed-refrence/zed/crates/vim/src/helix/object.rs` and `helix/select.rs`.
- Shared profile candidate and active binding:
  `zed-refrence/zed/assets/keymaps/vim.json`.
- Mode settings and action inventory:
  [`settings-requirements.md`](settings-requirements.md) and
  [`registered-actions-unbound.md`](registered-actions-unbound.md).
- Labonair capability/registry boundaries:
  [`docs/capabilities.md`](../capabilities.md),
  [`docs/registries.md`](../registries.md),
  `crates/editor/src/vim.rs`, and `crates/keymap/src/command_provider.rs`.

Same-session pinned-source research and independently written requirements
are redevelopment, not separated-room clean-room implementation.
