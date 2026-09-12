# R09-004 — Editor E3: display map, search, folding, and navigation

## Status

⏳ Planned

## Owner

- Module: editor
- Capability-matrix row: [Editor](../../docs/capabilities.md)
- Composition entry point: Editor-owned view boundary hosted by the Workspace Editor tab

## Dependencies

- R09-001-editor-e0-text-model-contract
- R09-003-editor-e2-editing-and-splits
- R09-002-editor-e1-file-lifecycle
- R07-001-product-surface-acceptance must be complete.

## Goal

Replace the fixed character-grid renderer and line-only search with a
viewport-aware display model that supports wrapping, horizontal scrolling,
folds, gutters, minimap/sticky context where enabled, fast project search
handoff, and precise navigation.

## Scope

- In scope: DisplaySnapshot/display transformations, viewport and scroll
  management, soft wrap, horizontal scroll, folds, indent guides, line
  numbers, selection/caret rendering, minimap and sticky context, bracket
  matching, in-buffer literal/regex/multiline search and replace, project
  search handoff, outline/symbol navigation, and accessible focus states.
- Out of scope: LSP-provided symbols/diagnostics/completion, Git gutter
  state, remote search, and changes to the canonical Project Diff surface.

## Contracts and ownership

- Public domain values: DisplaySnapshot, DisplayPoint, DisplayRow, FoldRange,
  SearchQuery, SearchMatch, SearchOptions, SearchResult, OutlineItem, and
  typed DisplayEvent/SearchEvent in labonair-editor.
- Service traits or typed events: a narrow ProjectSearchProvider contract for
  filesystem-backed search results; labonair-filesystem remains the search
  implementation owner and Editor owns query/result presentation.
- Registry contributions: Editor contributes commands for Find, Replace,
  Find Next/Previous, Go To Line, Go To Symbol, Fold/Unfold, Fold All,
  Unfold All, Toggle Word Wrap, Toggle Minimap, and Toggle Sticky Context.
- UI surface: Editor tab/standalone Editor overlay and in-editor panels.
  Project search results remain the existing workspace/search surface, and
  Project Diff remains the only canonical review surface.
- Shared UI-kit components: search fields, lists, rows, popovers, dialogs,
  scroll containers, menus, tabs, badges, focus indicators, and keyboard
  hints. Editor text, gutter, fold markers, and minimap are feature-owned
  compositions using theme tokens.

## Commands and keymap

Use canonical command identities and Editor context. Find/replace must expose
case sensitivity, whole word, regex, multiline, preserve-case, and wrap
behavior as typed SearchOptions; no hard-coded whole-word or literal-only
fallback remains. Invalid query syntax is local field validation. Project
search invokes the filesystem provider through a typed contract rather than
reading the filesystem on the GPUI thread.

## Settings

These are proposed value settings and become binding only when implemented
with a Settings inventory row:

| Key | Type / default | Scope | Migration and test |
|---|---|---|---|
| editorBracketMatching | bool / true | Global + safe Project | Promote retained legacy input; validate bool, reset to true, test live rendering |
| editorIndentationGuides | bool / true | Global + safe Project | Promote retained legacy input; validate bool, reset to true, test display snapshot |
| editorStickyScroll | bool / false | Global + safe Project | New key; default false, unknown-field safe migration, test narrow/empty states |
| editorMinimap | bool / false | Global + safe Project | New key; default false, unknown-field safe migration, test performance and toggle |
| editorRenderWhitespace | enum / none | Global + safe Project | New key; validate enum, default none, test token rendering |
| editorScrollBeyondLastLine | bool / true | Global + safe Project | New key; default true, validate bool, test viewport bounds |

The existing editor font, line height, tab size, word wrap, line-number,
relative-number, indentation, Vim, and editor-theme values remain unchanged.
Each proposed field must be added to Settings content, scope/whitelist,
generated UI, migration classification, and focused consumer tests in the
same implementation change.

## Persistence and migration

- Storage: DisplayMap, folds, search query, and scroll are live Editor view
  state. Durable folds, selections, and scroll offsets are serialized in E6.
- Migration: old line-index state is clamped to the new buffer revision; a
  missing or invalid fold range is ignored without altering file contents.
- Compatibility: the current literal search adapter remains only while all
  callers use SearchQuery/SearchOptions. Remove it when Workspace and the
  command palette consume the typed contract.

## Notifications

Search has no passive notifications. Filesystem project-search failures and
limit/recovery states publish structured notifications with details and
deduplication metadata. Invalid regex input remains inline field validation
because the user must correct it; it is not a toast.

## Dependency restrictions

- Keep display, folding, search presentation, and Editor commands in the
  Editor owner.
- Use a direct typed filesystem search contract for the current provider; do
  not create a general search registry for one provider.
- Do not add dependencies from Editor to Workspace, Shell, or Project Diff.
- A new parsing/search dependency needs a license and performance rationale,
  focused tests, and dependency-verifier review.

## Implementation plan

1. Define display-coordinate mappings and viewport snapshots over anchors →
   verify wrapped, folded, long-line, Unicode, and scroll-boundary tests.
2. Implement folds, gutters, guides, minimap/sticky context, bracket matching,
   and focus states → verify normal, narrow, focused, empty, loading, and
   long-file visual states.
3. Implement typed in-buffer and project search with replacement and result
   limits → verify literal, regex, multiline, case, whole-word, wrap,
   replacement, invalid-query, and filesystem-limit tests.
4. Replace the old renderer/search adapter and register commands → verify
   palette/keymap integration, source audit, and removal of hard-coded options.

## Acceptance criteria

- [ ] Rendering uses display snapshots and remains responsive for long files;
      no foreground I/O or full-document reparsing occurs on every keystroke.
- [ ] Wrapping, horizontal scroll, folds, gutters, bracket matching, and
      configured minimap/sticky context behave consistently with selections.
- [ ] Search and replace support the typed options and project-search limits.
- [ ] Outline navigation exposes ranges and hierarchy where the parser
      provides them, with a usable empty state.
- [ ] Proposed settings have complete owner, type, default, scope, migration,
      reset, validation, UI, and consumer-test records.
- [ ] Focused core, UI, search, visual, registry, and filesystem tests pass.
- [ ] Full repository, dependency, documentation, queue, and diff checks pass.

## Removal / exit condition

The task is complete when the fixed character-grid renderer, line-only search,
hard-coded search options, and old fold/navigation paths are removed, and
Editor is the sole owner of display/search presentation.

## Notes and follow-ups

E4 adds semantic tokens and local LSP decorations to the same display
contract. E5 adds Git hunk decorations without making Project Diff a second
Editor implementation.

## Implementation evidence (E3 core)

- Added the Editor-owned `DisplayMap`/`DisplaySnapshot` contract with soft
  wrapping, folded ranges, viewport and scroll clamping, logical/display
  coordinate mapping, line-number modes, indentation guides, and
  whitespace-aware ranges.
- Reworked search around typed query/options and match ranges, including
  regex, multiline, whole-word, case handling, wrap navigation, and
  transaction-backed replace-all. Unicode and CRLF mappings are covered by
  focused tests.
- The Workspace adapter consumes the display/search contracts; it does not
  create a second document or split/search model. E4 decorations are projected
  through the same display snapshot.
- Verification: 155 `labonair-editor` unit tests plus 10 LSP/runtime integration/
  fixture tests, 140 Workspace tests, formatting,
  workspace check, Clippy, dependency, queue, documentation, and diff checks
  pass in the current worktree. Native visual acceptance remains under
  R07-001 because the Labonair application surface is not available to the
  current UI automation session.
- The task remains formally `Planned` while R07-001 is open; this evidence
  records the implemented core and does not claim the visual matrix is done.

## Implementation evidence (display consumers)

The native adapter now consumes the configured editor font family, font size,
line height, rulers, minimap, sticky symbol context, semantic-token ranges,
cursor blink preference, and optional Outline panel. Outline entries navigate
through the Editor document contract, while the minimap and sticky context
remain editor-local overlays. Cursor/selection status is also rendered only
when the corresponding settings are enabled.

The existing Cmd+F overlay now forwards a typed Editor search query with
smart-case, regex, whole-word, multiline, and wrap options. It also exposes
Editor-only Replace One and Replace All actions, including capture-group
replacement, validation errors, and disabled states for read-only or
unavailable targets. Workspace remains a composition adapter and does not own
matches or replacement transactions.

The bounded project-search slice adds the Editor-owned
`ProjectSearchQuery`/`ProjectSearchHit`/`ProjectSearchResult` and
generation-checked `ProjectSearchSession` contract. Workspace maps the
existing `fs_grep` response through a blocking adapter, preserves line/text
metadata and truncation, rejects unsafe result paths, and hosts the existing
Cmd+F File/Project scope with keyboard result navigation. Project search does
not expose multiline mode because `fs_grep` is line-oriented.

The Open File slice adds the Editor-owned `FileFinderQuery`,
`FileFinderCandidate`, `FileFinderResult`, and generation-safe
`FileFinderSession`. Workspace lists only local files below the explicit
workspace root through the existing `fs_glob` primitive on a blocking runtime,
canonicalizes every candidate, and re-validates the selected path before
calling `Workspace::open_file`. The existing ModalLayer hosts the keyboard-first
view; `Cmd+Shift+O`, the palette row, native File menu, and keymap all resolve
through `CommandId::OpenFile`.

Verification in the current worktree: 155 `labonair-editor` unit tests plus 10
LSP/runtime integration/fixture tests pass; 140 `labonair-workspace` tests pass,
including focused Finder ranking/session, adapter, and command-ownership tests.
The requested Clippy and final diff checks are run after this documentation
update. Native visual acceptance remains under R07-001 because the Labonair
application surface is not available to the current UI automation session.
