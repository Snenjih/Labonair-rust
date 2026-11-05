# Editor Go to Line Action Classification

**Status:** Static semantic and Labonair-owner crosswalk for one default-macOS
action; pinned runtime, native interaction, and paired visual acceptance remain
pending  
**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`  
**Owner:** Editor navigation state; Workspace composes the current Editor view
and command-palette route

## Evidence boundary

This packet states the user outcome behind the pinned `go_to_line::Toggle`
action and compares it with Labonair's current Editor contract and route. The
[action register](action-requirements.md) retains the exact default key and
context. The pinned action catalog and source inspection support the static
classification, but do not prove macOS dispatch or rendered behavior.

The record is `Source checked`, not `Runtime observed`. The current host is
Linux without `DISPLAY` or `WAYLAND_DISPLAY`; no pinned macOS dispatch, native
Labonair interaction, or paired capture was available. Provenance remains the
same-session independent redevelopment documented in the [parity README](README.md#provenance-and-source-boundary).

## Owner and workflow contract

| Contract field | Current Labonair record |
|---|---|
| Owner module and canonical crate | Editor — `labonair-editor` owns document positions and caret state. The current interactive Editor view adapter is hosted by `labonair-workspace`; Workspace composes the view and routes typed actions. |
| Canonical user entry point | No direct `go_to_line::Toggle` route is registered. Outline symbol results can select a known line through the command palette, but that does not accept an arbitrary line or column. |
| Public operation boundary | Editor `Position`, `EditorView::goto_line`/`goto_position`, and the typed palette `SubmenuAction::GoToLine(usize)`. The current submenu payload carries a line only. |
| Registry coverage | The pinned action is absent as a direct Labonair Editor command/keymap descriptor. The Editor provider builds `GoToLine` actions only for Outline symbol rows; the Workspace palette handler applies that typed line jump. |
| State and persistence | The Editor document owns caret position. A go-to-line query and preview are transient in the target workflow; Labonair has no query state or navigation history to persist for this action. |
| Settings and migration | No action-specific setting or persisted value is present or added by this inventory. |
| Notifications and failure | The current line jump clamps a requested line to the document's last line. The pinned prompt's invalid-input and dismissal behavior still needs runtime acceptance; no notification route is added here. |
| UI-kit and focus | Labonair has no direct go-to-line prompt or focus route. A future prompt must use shared UI-kit input and preserve the Editor's focus and caret lifecycle. |

## Required states, input, and persistence checks

The pinned default binding is `Ctrl+G` in the full Editor context. Acceptance
must verify context exclusion, input focus, preview, confirmation, and cancel
against the pinned runtime and Labonair build. The following target states are
still unverified at runtime:

- **Input:** empty query, one-based absolute line, optional colon-separated
  character column, positive/negative relative line offset, malformed line,
  malformed column, and whitespace around the query.
- **Document bounds:** empty and one-line documents, first and last lines,
  out-of-range line/column, Unicode text, and multi-buffer/excerpt positions.
- **Preview and commit:** preview the requested line without moving the caret;
  confirm moves the caret and focuses the Editor; cancel closes the prompt,
  clears preview, restores scroll, and leaves the caret unchanged.
- **Surface state:** Editor absent, Editor outside the full mode, modal already
  open, focus transferred elsewhere, narrow window, and prompt recovery after
  invalid input.
- **Persistence:** query, preview, and prompt state remain transient; the
  resulting caret position follows the Editor document's existing lifecycle.

## Per-action semantic crosswalk

`Partial` means Labonair has a related operation but lacks the pinned entry
point, input, or state. These are static classifications, not runtime
acceptance.

| Requirement | Labonair owner / canonical crate | Pinned user goal, stated independently | Labonair static route, coverage, and gap |
|---|---|---|---|
| [`ACT-MACOS-go_to_line-Toggle`](action-requirements.md) | Editor (`labonair-editor`); Workspace (`labonair-workspace`) composes the view | Open a compact prompt for the active Editor and move its caret to an absolute line, optional character column, or relative line offset. | **Partial.** `EditorView::goto_line` can move to a clamped line and `goto_position` can apply a line/column, but no direct prompt or `go_to_line::Toggle` command is registered. Outline symbol rows can jump to a known line through `SubmenuAction::GoToLine(usize)`. Evidence: [`command_provider.rs`](../../crates/editor/src/command_provider.rs), [`workspace.rs`](../../crates/workspace/src/workspace.rs), [`editor.rs`](../../crates/workspace/src/views/editor.rs). |

## Existing test traceability

| Related local evidence | Requirement IDs | Coverage limit |
|---|---|---|
| Outline symbol snapshots produce typed line-jump actions; Workspace applies the line to the active Editor. | `ACT-MACOS-go_to_line-Toggle` | This is a different, line-only route. No action-specific local regression test or pinned-key dispatch test was found for the direct prompt. |

No local action-specific test, native interaction, or paired visual artifact
is recorded for this ID. Tests in the pinned source do not count as Labonair
coverage or runtime observation.

## Next work

Continue classifying the remaining default-macOS action families and the
registered-but-unbound catalog. Keep Editor caret ownership with Editor and
use Workspace for composition; do not treat an Outline result as the missing
arbitrary line/column workflow.
