# Vim ConvertToRot47 Action Classification

**Status:** Static printable-ASCII transform, inactive keymap candidates, palette actions, and local owner gap classified; runtime and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns text transforms, selections, and document edits; Keymap owns shared profile and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers <code>vim::ConvertToRot47</code> and the separate <code>vim::PushRot47</code> operator action. Neither has a binding in the macOS, Linux, or Windows platform-default maps. In the shared <code>vim.json</code> file, both ROT47 <code>g ?</code> candidates are comments: the Normal candidate for PushRot47 and the Visual candidate for ConvertToRot47 are inactive. The active <code>g ?</code> rows instead select ROT13. Registered Vim actions have a focused-Editor Command Palette route when Vim/Helix is enabled, so the direct conversion and operator-start actions may be invoked there; palette discoverability and a completed operator workflow still need runtime verification.

No active Helix key binding was found for either ROT47 action. There is no stable KEYBIND requirement ID for the commented candidates.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ConvertToRot47](registered-actions-unbound.md) | Registered unit action that applies printable-ASCII ROT47 to a mode-derived Editor range. | Labonair has no ROT47 transform action, selection route, or palette entry. |
| [ACT-REGISTRY-LINUX-vim-PushRot47](registered-actions-unbound.md) | Separate registered action that pushes the ROT47 operator; no active <code>g ?</code> binding exists. Palette start followed by a motion/object is a static candidate workflow, not runtime-observed. | Labonair has no ROT47 operator or motion route. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate shared profiles and Vim palette discovery; there is no ROT47-specific setting. | Labonair has partial Editor-owned Vim support and no Helix profile. |

## Pinned behavior

The direct action uses the shared <code>manipulate_text</code> path, consuming count and forced-motion state and applying the transform to mode-derived Editor ranges in a transaction. The range, cursor, and resulting-mode behavior matches the [ConvertToLowerCase action](action-classification-vim-convert-to-lower-case.md): Visual ranges collapse to their starts and request Normal; Helix Normal transforms a selection or one character and stays Helix Normal; Helix Select and other direct palette modes request Normal, with Helix-setting and temporary-mode remapping as documented there. In Visual mode the selected range determines the transform; count does not widen it.

For each character, code points U+0021 through U+007E rotate by 47 places through the 94 printable ASCII characters. Space, control characters, DEL, and all non-ASCII scalars remain unchanged. The transform is one-to-one and preserves string length. Unlike ROT13, ROT47 also transforms printable punctuation and digits.

PushRot47 is a different action that pushes <code>Operator::Rot47</code>. The normal operator handler applies the same Editor ROT47 transform to a motion or object range. Its usual shared <code>g ?</code> candidate is commented out; the active Normal <code>g ?</code> route starts PushRot13, and active Visual <code>g ?</code> invokes ConvertToRot13. A palette-started PushRot47 followed by motion/object is statically supported by those action handlers but has no focused fixture or runtime evidence.

The pinned Vim source contains no focused ConvertToRot47 or PushRot47 fixture. The algorithm and operator branches were inspected, not run. There is no direct Helix transform fixture.

## Labonair owner and current route

The canonical behavior owner is <code>labonair-editor</code>; <code>labonair-keymap</code> owns future mode profiles and user overrides, Settings owns Vim/Helix mode values, and the existing Command Palette remains the discovery surface. No ROT47-specific setting, notification, event, registry, dependency, shell-wide conditional, or new UI control is needed.

Labonair's Editor has no ROT47 transform, direct action, palette entry, operator, or shared keymap route. Its cursor-local <code>~</code> action performs a different case toggle.

ROT47 edits remain ordinary Editor document content and use the standard save/undo lifecycle. No transform-specific persistence is required.

## Acceptance still required

- Verify direct palette discovery and invocation of ConvertToRot47 in Normal, Visual, Helix Normal, Helix Select, Insert, Replace, and temporary Normal states. Capture the transformed range, cursor, selection, focus, count, forced-motion state, and resulting mode, including read-only, no-op, and empty-range cases.
- Verify palette invocation of PushRot47 followed by motions and objects, including counts, forced-motion state, cursor restoration, selection behavior, cancellation, undo grouping, and invalid/no-range results.
- Verify the absence of active <code>g ?</code> ROT47 bindings in Vim Normal/Visual and Helix contexts. Confirm <code>g ?</code> continues to dispatch ROT13 in its active contexts and user overrides behave as expected.
- Exercise boundaries around space, U+0021, U+007E, DEL, control characters, non-ASCII scalars, line endings, multi-selections, and large ranges. Confirm printable ASCII rotates by 47 with unchanged length and all other scalars remain unchanged.
- Verify shared profiles and palette discovery separately on macOS, Linux, and Windows, with Vim/Helix settings and user overrides.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, mode, selection, cursor, theme, and scale recorded.
- Implement the typed direct transform and optional operator path within <code>labonair-editor</code>; add only justified profile entry points through <code>labonair-keymap</code> and do not activate the currently commented ROT47 key candidates without product acceptance.

## Source records inspected

- Direct transform and common mode-derived range handling: <code>zed-refrence/zed/crates/vim/src/normal/convert.rs</code>.
- PushRot47 registration and operator push: <code>zed-refrence/zed/crates/vim/src/vim.rs</code>.
- Motion/object conversion dispatch: <code>zed-refrence/zed/crates/vim/src/normal.rs</code>.
- Active ROT13 rows and commented ROT47 candidates: <code>zed-refrence/zed/assets/keymaps/vim.json</code>.
- Editor-level printable ASCII mapping: <code>zed-refrence/zed/crates/editor/src/editor.rs</code>.
- Linux action registration evidence: <code>docs/parity/evidence/zed-actions-3569541-linux-x86_64.json</code>.
- Labonair action search across Editor sources found no ROT47 route: <code>crates/editor/src</code>.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
