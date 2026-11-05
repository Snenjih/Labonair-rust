# Vim ClearExchange Action Classification

**Status:** Static action, exchange cancellation path, shared binding, and local owner gap classified; direct runtime and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns the exchange range/highlight and Vim operator state; Keymap owns shared mode and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers vim::ClearExchange. It has no binding in the macOS, Linux, or Windows platform-default maps. The shared vim.json layer binds c to ClearExchange only in vim_operator == cx. The sequence begins with cx, where c followed by x starts the Exchange operator; a following motion or text object stores the first range. A later cx followed by a motion or text object exchanges that range. The c key in the cx operator context instead clears the stored exchange state, producing the c x c cancellation route.

The action is registered and participates in the existing focused-Editor Vim action discovery route when Vim/Helix is enabled. Palette invocation does not require the cx keymap context. The handler clears the VimExchange highlight and all pending Vim operator state, so invoking it from another pending operator context can cancel that operator as well. This route and its runtime effect need direct acceptance.

The source fixtures exercise c x c through simulated key sequences. One checks that the active operator is cleared without an exchange range; another first stores a range, invokes c x c, and checks that no background highlights remain. They were inspected but not run. The tests do not establish palette invocation, cursor/mode behavior, or platform dispatch.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ClearExchange](registered-actions-unbound.md) | Registered unit action that clears the VimExchange background highlight and pending Vim operator state. | Editor owns the behavior, but Labonair has no exchange range, clear action, or palette descriptor. |
| [KEYBIND-VIM-HELIX-3f44433d13b1](keymap-overlay-requirements.md) | Shared c binding under vim_operator == cx; no platform-default binding. | Labonair has no exchange operator context or shared cx c route. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate the shared layer and Vim palette namespace; there is no ClearExchange-specific setting. | Labonair has only a partial Editor-owned Vim preference and no Helix mode/profile layer. |

## Pinned behavior

The action handler stops Vim recording, clears Editor background highlights keyed by VimExchange, and then calls the common operator-clear routine. That routine consumes the pending count and forced-motion state, removes the selected register, clears the operator stack, and synchronizes Vim settings. It does not edit the buffer or directly change the mode or cursor. The exchange range is transient Editor highlight state, not a document edit.

Clearing works whether or not an exchange range is stored. With no range, the highlight removal has nothing to remove, but the operator-clear routine still runs. This makes c x c a cancellation route for a pending exchange even before its first range is selected. Because the common routine clears the whole operator stack, a direct palette invocation while a different operator is pending also cancels that state; the focused palette interaction and resulting context require runtime verification.

ClearExchange is distinct from [vim::Exchange](action-classification-vim-exchange.md). Exchange starts or completes the text exchange: the first motion or text object stores and highlights a range, and a later range may exchange text with it. ClearExchange removes the saved range and cancels pending Vim operator state without exchanging or editing text.

## Labonair owner and current route

The canonical behavior owner is labonair-editor. Keymap owns future shared mode bindings and user overrides, Settings owns Vim/Helix mode values, and the existing Command Palette remains the discovery surface. No ClearExchange-specific setting, notification, event, registry, shell-wide conditional, dependency, or new UI control is needed.

Labonair's Editor and Vim parser have no exchange operator, stored exchange highlight/range, ClearExchange action, shared profile route, or palette descriptor. It therefore has no local equivalent for either exchange or its cancellation. This inventory records the owner gap without changing runtime code.

Exchange state is transient Editor state and has no persistence. If implemented later, its range/highlight and cancellation semantics should remain within labonair-editor.

## Acceptance still required

- Verify the shared c x c route after cx with and without a previously stored range. Confirm c x followed by a motion or text object stores the first range, and a second cx range performs the separate exchange operation.
- Confirm ClearExchange removes the VimExchange highlight, clears all operator-stack entries, count, forced-motion state, and selected register, and does not edit text or move the cursor. Confirm the resulting Vim mode.
- Invoke ClearExchange from the focused Editor Command Palette with and without a stored exchange range, and while an unrelated operator is pending. Record whether palette invocation closes or restores focus and verify which operator state is cancelled.
- Exercise repeated clearing, selected register choices, counts, forced motions, multiple/anchored ranges, and edits made while an exchange range is stored.
- Dispatch the shared cx c binding separately on macOS, Linux, and Windows, including user overrides and mode transitions.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, mode, exchange highlight, cursor, viewport, theme, and scale recorded.
- Implement the exchange range, highlight, operator, and typed action inside labonair-editor; expose shared bindings through labonair-keymap when those owner contracts are established.

## Source records inspected

- ClearExchange action registration and handler: zed-refrence/zed/crates/vim/src/vim.rs.
- Exchange range/highlight storage and clearing: zed-refrence/zed/crates/vim/src/replace.rs.
- Operator stack, count, forced-motion, and selected-register clearing: zed-refrence/zed/crates/vim/src/vim.rs.
- Shared Exchange and ClearExchange key contexts: zed-refrence/zed/assets/keymaps/vim.json.
- Clear action fixtures: zed-refrence/zed/crates/vim/src/replace.rs.
- Linux action registration evidence: docs/parity/evidence/zed-actions-3569541-linux-x86_64.json.
- Labonair Editor Vim implementation search: crates/editor/src/vim.rs.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
