# Vim Class Text Object Classification

**Status:** Static action, grammar range behavior, shared profile routes, and local owner gap classified; runtime and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns object/operator state, selections, and modal transitions; Language owns grammar text-object ranges; Keymap owns shared mode and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers vim::Class. It has no binding in the macOS, Linux, or Windows platform-default maps. The shared vim.json layer maps c to Class in Vim inside, around, and surround-change contexts. Under Helix mode, t maps to Class (Type) for current and next/previous type objects. Helix c selects the current Comment object in inside/around contexts; in next/previous contexts, later specific c bindings resolve to NextComment/PreviousComment. The broad Class and Comment rows also declare those directional contexts, but the later specific rows win by the pinned resolver's context-depth and declaration-order precedence. These declarations are shared profile data, not evidence of active profile loading or key dispatch on any platform.

Registered Vim actions also have a focused-Editor Command Palette route when Vim/Helix is enabled. Class depends on pending text-object/operator context to select or act on a range; invoking the action without that context does not provide a standalone class navigation operation. Palette discovery does not establish successful selection or operator behavior.

Class text ranges are syntax-backed. The generic text-object engine requests the language's InsideClass or AroundClass capture at the cursor. The bundled query files provide captures for C, C++, CSS, Go, JavaScript, Markdown, Python, Rust, TypeScript, and TSX; the Proto extension also provides class captures. Grammar definitions determine what counts as a class-like range. The action does not promise literal class-declaration behavior in every language.

The language test fixture for a Rust impl block returns the function range for InsideClass and the full impl block for AroundClass. It demonstrates grammar-defined semantics, not direct Vim dispatch. The Helix fixture exercises the t-as-Type mapping, including selection of a Rust struct. No focused Vim ac or ic dispatch fixture was found. Source fixtures were inspected, not run.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-Class](registered-actions-unbound.md) | Registered unit action mapped to Object::Class, using grammar captures for inside and around ranges. | Editor owns the capability, but Labonair has no syntax-backed text-object action or range. Its class symbol support is for symbol discovery. |
| [KEYBIND-VIM-HELIX-5f44df1dbbb2](keymap-overlay-requirements.md) | Shared c binding to Class in Vim a, i, and cs contexts. Its declared Helix next/previous candidates are overridden by later specific NextComment/PreviousComment bindings; no platform-default binding. | Labonair has no shared Vim Class profile binding. |
| [KEYBIND-VIM-HELIX-f021a4fe05af](keymap-overlay-requirements.md) | Shared t binding for Helix Type in a, i, and next/previous object contexts when Helix mode is enabled. Helix c selects the current Comment object in a/i contexts and resolves to directional comment actions in next/previous contexts. | Labonair has no Helix mode or Type text-object route. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate the shared layer and Vim palette namespace; there is no Class-specific setting. | Labonair has only a partial Editor-owned Vim preference and no Helix profile layer. |

## Pinned behavior

The Vim object action dispatches through the current Vim mode. In Normal and Helix Normal it uses the normal-object path; in Visual, Visual Line, Visual Block, and Helix Select it uses the visual-object path. Insert and Replace modes ignore the object action. In Vim, the normal path requires a pending text-object operator, such as inside, around, or a surround-change operation. The visual path likewise requires an active object operator. With no such context, Class does not perform a standalone selection.

For an inside request, the range helper first seeks the smallest matching InsideClass capture at the cursor. If that capture is absent, it finds the smallest surrounding AroundClass capture and searches within it for an inside capture; if no non-empty inner capture exists, it falls back to the full surrounding class capture. Around requests select the smallest matching AroundClass capture. When no matching surrounding capture exists, no range is returned.

Class is treated as multiline and as an object that expands in both directions. Vim inside selection targets Visual mode; around selection targets Visual Line mode. The Class range branch does not use the text-object count, so a count does not widen the syntax capture. Operator behavior when no capture is found, including resulting mode and selection, remains a runtime acceptance question.

In Helix, t selects the current Type object, while ]t and [t route through the next/previous object handlers with around enabled. Those handlers use object-specific Helix ranges when available. The pinned fixture confirms the t mapping selects a Rust struct and separately confirms that c means Comment in Helix. For directional selectors, the later c rows resolve to NextComment/PreviousComment; t remains the Type selector. Current, next, and previous Type behavior still depends on grammar queries and runtime acceptance.

Class text-object selection is distinct from symbol discovery or navigation to a class declaration. The action changes selection or supplies a range to the pending operator; it does not itself jump to a symbol.

## Labonair owner and current route

The canonical behavior owner is labonair-editor. Language grammar integration remains within that capability boundary; labonair-keymap owns future mode profiles and user overrides, Settings owns Vim/Helix mode values, and the existing Command Palette remains the discovery surface. No Class-specific setting, notification, event, registry, shell-wide conditional, or new UI control is needed.

Labonair's Editor has class symbol information in its symbol-discovery path, but crates/editor has no Vim Class text-object range, operator integration, typed Class action, Helix mode, shared profile route, or palette descriptor. Symbol discovery must not be counted as text-object coverage. This inventory records the owner gap without changing runtime code.

Object selection and pending operator state are transient Editor state. Language query definitions provide the ranges; no separate Class persistence is needed.

## Acceptance still required

- Verify the shared c route in Vim inside/around and surround-change contexts, including excluded contexts. Verify Helix t as Type and c as the current Comment object, plus ]t/[t and ]c/[c directional behavior. Confirm c resolves to NextComment/PreviousComment in directional contexts because those exact rows are declared later.
- Verify focused-Editor palette discovery and invocation with and without pending object/operator context. Capture selection, cursor, mode, focus, and any operator effect.
- Exercise nested class-like captures, cursors on boundaries, missing inner captures, missing around captures, empty captures, counts, multiple selections, and each supported grammar. Confirm no-range behavior for each caller.
- Compare Vim inside and around behavior separately from Helix current, next, and previous Type selection. Include representative class, struct, impl, interface, and grammar-specific ranges without generalizing across languages.
- Dispatch the shared bindings separately on macOS, Linux, and Windows, including user overrides and mode transitions.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, mode, selection, cursor, viewport, theme, and scale recorded.
- Implement the syntax-backed action and range adapter in labonair-editor; add shared profile contributions through labonair-keymap and mode values through Settings only when the owner contracts are established.

## Source records inspected

- Class action registration, mode dispatch, Object::Class range, and capture selection: zed-refrence/zed/crates/vim/src/object.rs.
- Normal and visual object/operator handling: zed-refrence/zed/crates/vim/src/normal.rs and zed-refrence/zed/crates/vim/src/visual.rs.
- Helix current, next, and previous object selection: zed-refrence/zed/crates/vim/src/helix/select.rs and zed-refrence/zed/crates/vim/src/helix/object.rs.
- Shared c/t and Helix directional bindings: zed-refrence/zed/assets/keymaps/vim.json.
- Rust InsideClass/AroundClass fixture: zed-refrence/zed/crates/language/src/buffer_tests.rs.
- Helix Type and Comment fixture: zed-refrence/zed/crates/vim/src/helix/object.rs.
- Bundled text-object query files and Proto extension: zed-refrence/zed/crates/grammars/src and zed-refrence/zed/extensions/proto/languages/proto/textobjects.scm.
- Linux action registration evidence: docs/parity/evidence/zed-actions-3569541-linux-x86_64.json.
- Labonair Editor Vim and symbol-discovery paths: crates/editor/src/vim.rs and crates/editor/src/symbols.rs.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
