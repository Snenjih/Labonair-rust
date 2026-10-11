# Vim Comment Text Object Classification

**Status:** Static action, syntax range, operator, and shared profile routes classified; runtime dispatch and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns comment range selection, operator state, and comment edits; Language owns grammar comment captures; Keymap owns shared mode and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers <code>vim::Comment</code>. It has no binding in the macOS, Linux, or Windows platform-default maps. The shared <code>vim.json</code> layer binds <code>g c</code> to Comment while a Vim operator is pending, and binds <code>c</code> to Comment for Helix inside/around selection. A broad Helix row also declares <code>c</code> for next/previous operators, but the later, exact directional rows bind those same contexts to <code>vim::NextComment</code> and <code>vim::PreviousComment</code>. The pinned GPUI resolver sorts by context depth and then later declaration order; the exact directional bindings therefore win. These declarations are profile data, not proof of loaded profiles or native dispatch.

The normal-mode <code>g c</code> route to <code>vim::PushToggleComments</code> and visual-mode <code>g c</code> route to <code>vim::ToggleComments</code> are separate comment-editing actions. They do not classify <code>vim::Comment</code>. The pinned <code>test_toggle_comments</code> fixture exercises those toggle routes, including the repeated operator, visual selections, counts, and motion; it is not a direct Comment text-object test.

Registered Vim actions also have a focused-Editor Command Palette route when Vim/Helix is enabled. This is a discovery route, not a standalone comment-navigation operation. Direct invocation, mode behavior, operator state, and resulting range still require runtime acceptance.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-Comment](registered-actions-unbound.md) | Registered unit action mapped to <code>Object::Comment</code>, using grammar captures for inside and around comment ranges. | Editor owns the capability, but Labonair has no syntax-backed Comment text-object action or range. Its selected-line comment toggle is a different workflow. |
| [KEYBIND-VIM-HELIX-720754662b63](keymap-overlay-requirements.md) | Shared <code>g c</code> binding in Vim operator context; Comment supplies a syntax-backed object range to the pending operator. | Labonair has no Vim Comment object or shared mode binding. |
| [KEYBIND-VIM-HELIX-21b18eb55110](keymap-overlay-requirements.md) | Shared Helix <code>c</code> binding selects the current Comment object for inside/around operators. Its directional contexts are superseded by the later exact NextComment/PreviousComment rows. | Labonair has no Helix mode or current Comment object route. |
| [KEYBIND-VIM-HELIX-b69ab2413a28](keymap-overlay-requirements.md) and [KEYBIND-VIM-HELIX-523832efe256](keymap-overlay-requirements.md) | In Helix next/previous selector contexts, <code>c</code> resolves to NextComment/PreviousComment rather than Comment. | Labonair has no directional comment selector route. |
| [KEYBIND-VIM-HELIX-d61dd4a23dcd](keymap-overlay-requirements.md) and [KEYBIND-VIM-HELIX-1b70771bd066](keymap-overlay-requirements.md) | Vim Normal <code>g c</code> starts ToggleComments; Visual <code>g c</code> toggles comments over the selection. These are related comment-edit routes, not Comment text-object bindings. | Labonair offers a selected-line comment toggle, with no Vim operator/profile route. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate the shared layer and Vim palette namespace; there is no Comment-specific setting. | Labonair has only partial Editor-owned Vim support and no Helix profile layer. |

## Pinned behavior

The Comment action maps to <code>Object::Comment</code>. If no Object operator is active, its handler pushes an around Object operator before dispatching through the current Vim mode's normal or visual object path. It is not itself a comment edit. In Vim operator context, the shared <code>g c</code> binding applies the syntax-backed comment object to the pending operator. In Helix, <code>m a c</code> and <code>m i c</code> select the current comment object with around or inside semantics.

The generic text-object helper chooses the smallest matching capture at the cursor. No bundled grammar query defines <code>@comment.inside</code>; when an inside request has no inner capture, the helper falls back to the full surrounding <code>@comment.around</code> capture, when one exists. If there is no surrounding capture, no range is returned. Comment grouping is grammar-specific; line-comment sequences and block comments are not guaranteed to form the same range across languages.

The Helix fixture <code>test_helix_treesitter_object_keys</code> includes <code>m a c</code> selecting a Rust line comment. It confirms the current Comment object route, not directional motion. In the directional contexts, later specific <code>c</code> bindings select the next or previous comment; the broad Comment row does not win there. Vim's normal/visual toggle fixture verifies editing behavior, not this text object. These source fixtures were inspected, not run. No focused direct Vim operator-to-Comment fixture was found.

The action's default around-object setup and generic dispatch leave several direct-invocation edges open. A palette invocation without a pending operator does not establish a standalone navigation operation. Insert/Replace direct invocation appears to push object state before the object dispatcher ignores those modes; treat this as a static inference and verify the resulting state before classifying it as behavior or a defect.

## Labonair owner and current route

The canonical behavior owner is <code>labonair-editor</code>. Language query integration remains within that capability boundary; <code>labonair-keymap</code> owns future mode profiles and user overrides, Settings owns Vim/Helix mode values, and the existing Command Palette remains the discovery surface. No Comment-specific setting, notification, event, registry, dependency, shell-wide conditional, or new UI control is needed.

Labonair's Editor can toggle line comments for selected lines, but it has no syntax-backed comment range, Vim operator-pending Comment action, Helix current/directional comment selection, or corresponding shared profile entries. The line-oriented toggle does not satisfy Comment text-object coverage.

Comment range and pending operator state are transient Editor state. Grammar queries provide captures; text edits made by an outer operator use ordinary document persistence. There is no Comment-specific persistence.

## Acceptance still required

- Verify Vim operator-pending <code>g c</code> as a Comment object with representative pending operators, and distinguish it from Normal/Visual <code>g c</code> comment toggling, including repeated operator, motion, count, and visual-selection behavior.
- Verify Helix current Comment selection with inside and around operators, plus <code>]c</code>/<code>[c</code> and <code>c</code> in directional contexts. Confirm the latter resolve to NextComment/PreviousComment and exercise no-match and boundary behavior.
- Verify direct Command Palette discovery and invocation with no pending operator, with a pending operator, and in Insert/Replace modes. Capture selection, cursor, mode, focus, and pending operator state.
- Exercise grammars with line and block comments, multiple/nested captures, cursor boundaries, absent inner captures, absent surrounding captures, empty captures, multiple selections, and grammar-specific grouping. Confirm no-range behavior for every caller.
- Dispatch the shared profile separately on macOS, Linux, and Windows, including mode transitions and user overrides.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, mode, selection, cursor, viewport, theme, and scale recorded.
- Implement syntax-backed Comment object selection and the required Vim/Helix routes within <code>labonair-editor</code> and <code>labonair-keymap</code>; retain the separate selected-line toggle workflow.

## Source records inspected

- Comment registration, mode dispatch, Object::Comment range selection, and capture fallback: <code>zed-refrence/zed/crates/vim/src/object.rs</code>.
- Normal/visual object and operator handling: <code>zed-refrence/zed/crates/vim/src/normal.rs</code> and <code>zed-refrence/zed/crates/vim/src/visual.rs</code>.
- Helix comment-object fixture and Vim comment-toggle fixture: <code>zed-refrence/zed/crates/vim/src/helix/object.rs</code> and <code>zed-refrence/zed/crates/vim/src/test.rs</code>.
- Shared object, normal toggle, visual toggle, and directional bindings: <code>zed-refrence/zed/assets/keymaps/vim.json</code>.
- Context-depth and declaration-order precedence: <code>zed-refrence/zed/crates/gpui/src/keymap.rs</code> and <code>zed-refrence/zed/crates/gpui/src/keymap/context.rs</code>.
- Comment capture registration and bundled grammar queries: <code>zed-refrence/zed/crates/language_core/src/grammar.rs</code> and the bundled language/extension query files.
- Labonair selected-line comment toggle: <code>crates/editor/src/editing.rs</code> and <code>crates/editor/src/command_provider.rs</code>.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
