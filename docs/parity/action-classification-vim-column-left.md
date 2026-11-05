# Vim ColumnLeft Action Classification

**Status:** Static action, shared binding, viewport/cursor behavior, and local owner gap classified; native and cross-platform acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns viewport scrolling and cursor/selection updates; Keymap owns shared mode and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers vim::ColumnLeft. It has no binding in the macOS, Linux, or Windows platform-default maps. The shared vim.json layer binds z h under VimControl && !menu. VimControl is present in Normal, Visual, Operator, Helix Normal, and Helix Select contexts; it is absent while the keymap is in a waiting operator context and while a menu is active. The binding is shared profile data, not evidence of active profile loading or dispatch on any platform.

Registered Vim actions also have a focused-Editor Command Palette route when Vim/Helix is enabled. Palette discovery does not establish the current editor's horizontal-scroll configuration, resulting viewport, or cursor behavior.

The action uses the Editor's screen-scroll path. A Vim count becomes a signed horizontal column amount; with no count, the action scrolls left by one column. The scroll path also consumes forced-motion state and exits temporary Normal mode. It does not edit buffer text.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ColumnLeft](registered-actions-unbound.md) | Registered unit action that scrolls the focused Editor left by a counted number of columns, defaulting to one. | Labonair has no Vim action for counted horizontal scrolling. Its EditorViewSession declares scroll_left, but no connected interactive route was found. |
| [KEYBIND-VIM-HELIX-6a1f8219a616](keymap-overlay-requirements.md) | Shared z h binding in VimControl && !menu, covering Vim Normal/Visual/Operator and Helix Normal/Select; no platform-default binding. | Labonair has no shared Vim/Helix profile or z h action route. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate the shared layer and Vim palette namespace. Soft-wrap and preferred-line-length settings affect horizontal display columns. | Labonair has a partial Editor-owned Vim preference and no Helix mode/profile layer. |

## Pinned behavior

ColumnLeft consumes the Vim count, using one when there is no explicit count, and sends ScrollAmount::Column(-count) to the shared Vim scroll path. It also consumes forced-motion state and exits temporary Normal mode. The scroll operation has no text-edit side effect.

For the main Editor viewport path, the handler first records whether the newest selection is on screen, then scrolls the viewport. If the selection was already off screen, it leaves its cursor/selection position unchanged. If it was on screen, the scroll path adjusts the selection head only when needed to keep it within the newly visible columns. A cursor that remains visible does not move. A cursor pushed outside the visible range is clipped to the nearest visible column. Visual selections keep their anchor while their head is adjusted; Visual Block mode uses its dedicated block-motion path. Horizontal scrolling clears the vertical/horizontal selection goal.

The shared horizontal-scroll fixture uses a constrained 12-column view with nowrap and exercises z h and z l. After scrolling right by half a page, z h leaves the cursor in place because it remains visible. Starting from the left edge, z l scrolls one column right and moves the cursor to keep it visible. The fixture demonstrates the shared key route and cursor clipping, but it does not cover ColumnLeft directly through the Command Palette or every Editor layout and mode.

The viewport's left edge and available horizontal extent constrain the scroll position. Gutter margin, short lines, soft wrapping, bounded preferred line length, hover scrolling, and viewport size can affect the visible result and need separate acceptance. The pinned behavior does not change file contents.

## Labonair owner and current route

The canonical behavior owner is labonair-editor. Keymap owns future shared mode bindings and user overrides, Settings owns Vim/Helix mode values, and the existing Command Palette remains the discovery surface. No ColumnLeft-specific setting, notification, event, registry, dependency, shell-wide conditional, or new UI control is needed.

Labonair's Editor has no typed ColumnLeft action or shared Vim/Helix keymap route. Its EditorViewSession includes a scroll_left field with a default value, but a repository search found no runtime read/write path beyond the field declaration and defaults. That field is not interactive horizontal-scroll coverage.

The Editor owns viewport and selection state. If the stored scroll_left value is wired in a future session workflow, that persistence behavior should remain in the Editor owner and be accepted separately from this action.

## Acceptance still required

- Verify z h in Vim Normal, Visual, Operator, and Helix Normal/Select, and excluded waiting/menu contexts. Dispatch the shared binding separately on macOS, Linux, and Windows, including user overrides.
- Exercise no count and explicit counts, left-edge clamping, maximum horizontal offset, empty/short lines, long lines, cursor/selection on and off screen, multiple selections, Visual, Visual Block, Helix Select, and temporary Normal mode.
- Confirm that viewport movement changes by display columns while buffer text remains unchanged. Verify the cursor stays still when visible and clips to the visible column edge only when required.
- Test no-wrap, soft-wrap, bounded preferred line length, gutter margin, varying Editor widths, scrollbar extents, hover scrolling, and layouts with no visible-column measurement.
- Verify focused-Editor palette discovery and invocation. Capture viewport, selection/cursor, mode, focus, and scrollLeft session save/restore separately.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, mode, wrapping, viewport dimensions, horizontal offset, cursor, theme, and scale recorded.
- Implement horizontal scrolling and cursor clipping within labonair-editor; connect session persistence only when its lifecycle contract is established, and add the shared binding through labonair-keymap.

## Source records inspected

- ColumnLeft action registration and counted scroll setup: zed-refrence/zed/crates/vim/src/normal/scroll.rs.
- Shared scroll/editor cursor-clipping algorithm and horizontal-scroll fixture: zed-refrence/zed/crates/vim/src/normal/scroll.rs.
- Horizontal viewport movement and bounds: zed-refrence/zed/crates/editor/src/scroll.rs.
- Shared z h keymap row: zed-refrence/zed/assets/keymaps/vim.json.
- Linux action registration evidence: docs/parity/evidence/zed-actions-3569541-linux-x86_64.json.
- Labonair EditorViewSession scroll_left declaration/defaults: crates/editor/src/session.rs.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
