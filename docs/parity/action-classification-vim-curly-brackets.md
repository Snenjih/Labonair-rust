# Vim CurlyBrackets Action Classification

**Status:** Static Vim and Helix object routes, delimiter-range behavior, and Labonair owner gap classified; platform dispatch and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns text-object ranges, selection, and edits; Keymap owns the shared Vim/Helix profile; Settings owns Vim and Helix mode values.

## Evidence boundary

The pinned Linux action register contains `vim::CurlyBrackets` with an optional `opening` argument. The shared `vim.json` profile binds `{`, `}`, and `shift-b` in the broad `vim_operator == a || vim_operator == i || vim_operator == cs || vim_operator == helix_next || vim_operator == helix_previous` context. `{` supplies `opening: true`; the other two use the default. No Linux, macOS, or Windows platform-default map binds this Vim action. Static registration and profile data do not prove keyboard dispatch.

The later exact `vim_operator == helix_next` and `vim_operator == helix_previous` blocks bind `shift-b` to Pane navigation. Under the pinned keymap specificity and declaration-order rules, those rows override the broad CurlyBrackets candidate in those two directional contexts. The alias remains available in Vim inside/around and `cs` contexts. The separate `b` alias remains Parentheses.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-CurlyBrackets](registered-actions-unbound.md) | Registered object action. Vim uses it for inside/around curly pairs and as the old-pair selector for `cs`; Helix uses it for current and directional object selection. | `labonair-editor` has a `%` matching-bracket motion for `{}` but no inside/around curly text object, operator-object route, surround-change pair selection, or Helix object mode. |
| [KEYBIND-VIM-HELIX-e72a89c89d55](keymap-overlay-requirements.md) | Shared `{` binding with `opening: true` in Vim object, `cs`, and Helix directional contexts. | No shared Vim/Helix profile row exists in Labonair. |
| [KEYBIND-VIM-HELIX-db7eb0fe9d0f](keymap-overlay-requirements.md) | Shared `}` binding with the default `opening` value in the same broad context. | No shared Vim/Helix profile row exists in Labonair. |
| [KEYBIND-VIM-HELIX-27588cb974e8](keymap-overlay-requirements.md) | Shared `shift-b` curly alias in the broad context; later exact Helix next/previous contexts dispatch to Pane navigation instead. | No shared Vim/Helix profile or Helix directional mode exists in Labonair. |
| [SET-keymap-vim-mode and SET-keymap-helix-mode](settings-requirements.md) | These mode values gate the profile and route object actions through Vim or Helix operator state; there is no CurlyBrackets-specific setting. | Labonair's Vim support is partial and it has no Helix mode setting or profile. |

## Pinned behavior

In Vim, `i{` and `a{` select the inner or delimiter-inclusive range. An operator such as change, delete, or yank can consume that range; Visual mode expands the selection. The same object action is accepted after `cs` as the old surrounding pair selector. A generic action invocation without a pending object or surround operator does not establish a useful selection workflow.

In Helix mode, `m i {` and `m a {` select the current inside/around curly object. The next and previous object operators use `] {` and `[ {` and route through the same curly boundary object with around selection. The broad `shift-b` alias is overridden for those directional operators by the later exact Pane bindings, so `{` and `}` are the remaining listed curly routes there. This precedence is static source interpretation and still needs dispatch verification.

The Vim range implementation finds curly pairs by literal brace scanning rather than a syntax tree. The scanner tracks nested `{` and `}` pairs, ignores a brace preceded by a backslash, and searches across lines. Braces inside strings or comments can therefore participate when they are not escaped. The inside range of a multiline pair trims outer whitespace only when the enclosed content has non-whitespace; the around range includes the braces and does not absorb adjacent whitespace. Missing pairs yield no object range. The range path accepts an operator count but does not widen the curly selection by that count.

The Helix adapter maps this action to `ImmediateBoundary::CurlyBrackets` and uses its own current/next/previous range methods. The Vim scanner fixtures do not establish the Helix boundary search behavior; that remains a separate runtime acceptance item.

The focused pinned Vim fixture `test_multiline_surrounding_character_objects` uses nested braces in a multiline function. It checks `v i {` from inside the nested block and from positions outside the inner pair, and checks `c i {` for an empty multiline inner range. The shared `test_change_surrounding_character_objects` and `test_delete_surrounding_character_objects` fixtures iterate only double quotes and parentheses, so they do not establish curly change/delete coverage. No dedicated Helix CurlyBrackets fixture or fixture for the `shift-b` Pane override was found. These tests were inspected, not run.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns a future shared profile route, and Settings owns the existing Vim mode value and any future Helix mode value. No new dependency, notification, event, registry, or shell-wide conditional is needed for the action itself.

Labonair's Editor has a `%` motion that matches parentheses, square brackets, and braces. That is a cursor motion, not an inside/around object. No local `CurlyBrackets` object, operator-pending object dispatch, `cs` pair selection, Helix object route, or shared profile entry was found. The local motion therefore does not satisfy this workflow.

## Acceptance still required

- Verify Vim `i{` and `a{` from inside, on, and outside nested pairs; then exercise change, delete, yank, and Visual expansion with both delimiters and nested multiline content.
- Verify `cs{` old-pair selection and its subsequent new-pair input, including missing pairs, malformed nesting, escaped braces, braces in strings/comments, and cursor positions on either delimiter.
- Verify empty, whitespace-only, and non-whitespace multiline interiors; confirm the inner trimming and around whitespace rules, count behavior, cursor/selection direction, undo grouping, and no-range failure behavior.
- Verify Helix current, next, and previous object selection, mode transitions, `opening` direction, and the exact `shift-b` override against the pinned runtime. Confirm `{`/`}` still reach CurlyBrackets in the directional contexts.
- Dispatch all three shared bindings on macOS, Linux, and Windows with Vim/Helix enabled, each relevant base profile, and user overrides. Verify conflicting bindings and context exclusions separately.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, mode/operator context, theme, scale, cursor position, and selected range recorded.
- Implement the text object and operator consumers in `labonair-editor`, the shared profile route in `labonair-keymap`, and a typed Helix mode contract only if accepted. Preserve `%` as the separate matching-bracket motion.

## Source records inspected

- Vim action, object range, marker scanning, and focused tests: `zed-refrence/zed/crates/vim/src/object.rs`.
- Operator, Visual, and `cs` object handling: `zed-refrence/zed/crates/vim/src/normal.rs`.
- Helix boundary conversion and current/next/previous selection: `zed-refrence/zed/crates/vim/src/helix/object.rs` and `zed-refrence/zed/crates/vim/src/helix/select.rs`.
- Shared object bindings and later exact `shift-b` overrides: `zed-refrence/zed/assets/keymaps/vim.json`.
- Local `%` match-bracket motion and delimiter matching: `crates/editor/src/vim.rs` and `crates/editor/src/editing.rs`.
- Linux Vim action registration: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
