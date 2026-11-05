# Vim ClearOperators Action Classification

**Status:** Static action, cancellation semantics, six shared bindings, and local owner gap classified; runtime and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns operator stack, count, forced-motion, selected-register, and Helix jump-overlay state; Keymap owns shared mode and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers vim::ClearOperators. It has no binding in the macOS, Linux, or Windows platform-default maps. The shared vim.json layer maps Escape, Ctrl-C, and Ctrl-[ to ClearOperators in both vim_mode == waiting and vim_mode == operator. These are shared mode-profile declarations; the six rows do not establish active profile loading or runtime dispatch on any platform.

Registered Vim actions also have a focused-Editor Command Palette route when Vim/Helix is enabled. The action does not require a pending operator to run. Its common clear routine also consumes any pending count, forced-motion flag, and selected register, then synchronizes the mode context.

The same clear routine handles other cancellation paths. When an unhandled input arrives while an operator is not waiting for more input, the Vim input listener clears the state and immediately records ClearOperators as the action. Focus loss also clears operators through the shared helper, but uses a separate recording path.

No focused ClearOperators test was found in the inspected Vim source. Runtime tests should distinguish the explicit action and six bindings from implicit invalid-input and focus-loss cancellation.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ClearOperators](registered-actions-unbound.md) | Registered unit action that clears pending operator state, count, forced motion, selected register, and an active Helix jump overlay. | Editor owns the behavior, but Labonair has no matching modal operator state or action. |
| [KEYBIND-VIM-HELIX-ff7b5bc901bb](keymap-overlay-requirements.md), [KEYBIND-VIM-HELIX-ec55de25d812](keymap-overlay-requirements.md), and [KEYBIND-VIM-HELIX-0dcd99032537](keymap-overlay-requirements.md) | Shared Ctrl-C, Ctrl-[, and Escape bindings in waiting mode; no platform-default binding. | Labonair has no shared waiting-mode keymap layer. |
| [KEYBIND-VIM-HELIX-6d610abd8246](keymap-overlay-requirements.md), [KEYBIND-VIM-HELIX-5bab71d04374](keymap-overlay-requirements.md), and [KEYBIND-VIM-HELIX-947aa5d493ab](keymap-overlay-requirements.md) | The same three cancellation keys in operator mode; no platform-default binding. | Labonair has no shared operator-mode keymap layer. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate the shared layer and Vim palette namespace; there is no ClearOperators-specific setting. | Labonair has only a partial Editor-owned Vim preference and no Helix mode/profile layer. |

## Pinned behavior

The registered action calls the common clear_operator helper. The helper removes a Helix jump overlay when HelixJump is the active operator, consumes the pending count and forced-motion flag, clears the selected register and entire operator stack, then synchronizes Vim keymap settings. It does not edit buffer text, move the cursor, or directly switch the Editor mode. Synchronization changes the keymap context from waiting/operator back to the underlying mode.

If no operator is active, the helper still consumes any pending count, forced-motion flag, and selected register. An active operator is not required for direct Command Palette invocation. An active Helix jump additionally loses its navigation overlay.

ClearOperators does not clear the VimExchange background highlight. ClearExchange is the separate action that removes that stored exchange range. Clearing an exchange operator through Escape/Ctrl-C/Ctrl-[ can therefore cancel the operator while preserving the exchange highlight for a later exchange.

Escape, Ctrl-C, and Ctrl-[ have the same declared result in both waiting and operator contexts. Waiting covers operators that need a selector after a prefix; operator covers operators waiting for a motion or other continuation. The profile rows do not establish how each platform delivers Escape and control keys, how user overrides compose, or the resulting focus and cursor behavior.

## Labonair owner and current route

The canonical behavior owner is labonair-editor. Keymap owns future shared mode bindings and user overrides, Settings owns Vim/Helix mode values, and the existing Command Palette remains the discovery surface. No ClearOperators-specific setting, notification, event, registry, dependency, shell-wide conditional, or new UI control is needed.

Labonair's Editor and Vim parser do not provide a pending operator stack, Vim count/forced-motion/register state, Helix jump overlays, ClearOperators action, shared mode-profile route, or palette descriptor. The Editor has no local counterpart for this cancellation behavior. This inventory records the owner gap without changing runtime code.

Operator and cancellation state is transient and has no separate persistence. Helix jump overlays are also transient Editor UI state.

## Acceptance still required

- Verify Escape, Ctrl-C, and Ctrl-[ independently in both waiting and operator contexts, including user overrides, on macOS, Linux, and Windows.
- Exercise Vim operators and Helix selector/jump operators. Confirm that clearing resets every operator stack entry, count, forced-motion state, and selected register; confirm that HelixJump overlays are removed.
- Confirm that clearing preserves buffer text, cursor/selection, the underlying Normal/Visual/Helix mode, and a stored VimExchange highlight.
- Invoke ClearOperators from the focused-Editor Command Palette with an active and absent operator, count, forced motion, selected register, Helix jump, and exchange highlight. Record palette close/focus behavior and resulting keymap context.
- Verify implicit clearing on an unhandled key and clearing on focus loss separately from the explicit action. Record macro recording behavior for each route.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, modal mode, pending operator, count, selection, cursor, viewport, theme, and scale recorded.
- Implement the common transient modal state and typed cancellation action inside labonair-editor; contribute shared mode bindings through labonair-keymap when those owner contracts are established.

## Source records inspected

- ClearOperators registration and action handler: zed-refrence/zed/crates/vim/src/vim.rs.
- Operator/waiting keymap context construction, implicit invalid-input clear, focus-loss clear, and clear_operator helper: zed-refrence/zed/crates/vim/src/vim.rs.
- Six shared cancellation key rows: zed-refrence/zed/assets/keymaps/vim.json.
- Separate exchange-highlight cleanup behavior: zed-refrence/zed/crates/vim/src/replace.rs.
- Linux action registration evidence: docs/parity/evidence/zed-actions-3569541-linux-x86_64.json.
- Labonair Editor Vim implementation search: crates/editor/src/vim.rs.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
