# Vim CountCommand Action Classification

**Status:** Static count context, range-prefilled Command Palette route, and local owner gap classified; profile and end-to-end acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns Vim count/operator state; Keymap owns the shared Vim profile; Command Palette owns the modal query and command results; Settings owns Vim mode values.

## Evidence boundary

The pinned Linux action inventory registers `vim::CountCommand`. The shared Vim/Helix profile binds `:` to it only in `VimControl && VimCount`; the Vim context adds `VimCount` while a pre-operator count is pending or an operator is active with a post-operator count. This is not a general colon binding. No platform-default map binds the Vim action, and the shared profile declaration does not prove runtime dispatch.

The action consumes the pending count and forced-motion flag, computes a current-line range query, and passes that query to the Command Palette toggle. The Vim command interceptor parses ranged commands from the palette query. This action does not execute a Vim command itself. When Vim Mode is enabled, the generic palette filter can expose registered Vim actions, but selecting CountCommand while that palette is open reaches the toggle's close branch instead of seeding a new query; this is a static candidate route, not a useful confirmed workflow. If another modal cannot be dismissed or the window has no focused view, the action does not open the palette.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-CountCommand](registered-actions-unbound.md) | Registered unit action that consumes count/forced-motion state and opens the Command Palette with a line-range query; the palette-listed candidate closes an already-open palette. | Labonair has no CountCommand action, Vim Ex range parser/interceptor, or range-prefilled palette route. |
| [KEYBIND-VIM-HELIX-734cca507839](keymap-overlay-requirements.md) | Shared `:` binding in `VimControl && VimCount`, reached after a count before an operator or after an operator. | Labonair supports counted Vim motions but has no count-aware colon action or corresponding shared profile entry. |
| [SET-keymap-vim-mode](settings-requirements.md) | Vim Mode gates the shared profile and registers the Vim command interceptor/palette namespace; no CountCommand-specific setting exists. | Labonair has partial Vim support and no Vim Ex command layer. |

## Pinned behavior

`Vim::take_count` multiplies the pending pre-operator and post-operator counts when both exist and consumes the count state. CountCommand defaults a missing count to one. A count of zero or one seeds `.`; a larger count `n` seeds `.,.+(n-1)`, so count three seeds `.,.+2` for the current line and the next two lines. It also consumes the forced-motion flag.

The action requires an available Workspace and calls `CommandPalette::toggle` with the range string. When opening succeeds, the palette initializes its query from that string. With Vim enabled, its global command interceptor parses the range prefix and offers matching Vim commands; a focused Editor command such as `d`, `sort`, or another range-capable command then operates on the selected lines. Existing `test_command_ranges` exercises explicit ranged commands, but no focused fixture was found for the `n:` action seeding and executing the range. Source and fixtures were inspected, not run.

The action does not itself edit the buffer or persist the range. The selected Vim command owns any later buffer or workspace effects. Count state is transient and consumed before the palette opens; the palette query is transient as well.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor` for Vim count state, `labonair-keymap` for the future shared profile, and `labonair-command-palette` for query UI and command results. Settings owns the existing Vim mode value. No new notification, event, registry, dependency, shell-wide conditional, or permanent shell surface is needed.

Labonair's Editor supports counted motions but has no `CountCommand`, Vim Ex range parser/interceptor, or route that opens the Command Palette with a line range. Its Command Palette opens with an empty query and exposes registered command descriptors; its query setter is private, and no Vim command interceptor was found in the workspace. The generic `Cmd+P` command search is not the counted Vim command workflow.

## Acceptance still required

- Verify the `:` route with a pending count before an operator and after an operator. Confirm the context does not capture ordinary uncounted `:` or the separate Visual `:` route.
- Verify whether CountCommand is discoverable in the Vim Command Palette and confirm selecting it while the palette is open closes the palette without seeding the range.
- Verify count one, counts greater than one, and count zero produce the expected range strings. Test combined pre/post counts, very large counts, forced-motion reset, pending-operator state, and repeated/cancelled invocations.
- Verify `3:` opens the palette with `.,.+2`, retains focus, and offers range-capable commands. Execute representative delete, sort, join, goto, and buffer commands; confirm the affected lines, cursor, selection, undo grouping, and any workspace side effects.
- Verify palette toggle behavior when the palette is already open, another dismissible modal is active, a non-dismissible modal is active, the Workspace is missing, or the window has no focused view.
- Verify command query editing, acceptance, cancellation, focus restoration, and error handling after a seeded range; ensure the count has been consumed exactly once.
- Dispatch the shared `:` binding on macOS, Linux, and Windows with Vim Mode and user overrides. Capture the count context, palette query, Editor selection, and platform/profile details.
- Capture paired native macOS Keymap, Command Palette, and Editor evidence with platform, profile, count/operator context, theme, and scale recorded.
- Implement the typed action in `labonair-editor`, add only the justified profile route through `labonair-keymap`, and extend the existing Command Palette's typed command contract if the full Vim range workflow is accepted.

## Source records inspected

- CountCommand action and Vim command range parser/interceptor: `zed-refrence/zed/crates/vim/src/command.rs`.
- Pre/post count, forced-motion state, and `VimCount` context: `zed-refrence/zed/crates/vim/src/vim.rs` and `zed-refrence/zed/crates/vim/src/state.rs`.
- Shared count-colon profile row: `zed-refrence/zed/assets/keymaps/vim.json`.
- Command Palette toggle, initial query, and global command-interceptor path: `zed-refrence/zed/crates/command_palette/src/command_palette.rs` and `zed-refrence/zed/crates/command_palette_hooks/src/command_palette_hooks.rs`.
- Explicit range command fixtures: `zed-refrence/zed/crates/vim/src/command.rs`.
- Local Vim count parser and generic Command Palette open/query behavior: `crates/editor/src/vim.rs` and `crates/command-palette/src/palette.rs`.
- Linux Vim action registration evidence: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
