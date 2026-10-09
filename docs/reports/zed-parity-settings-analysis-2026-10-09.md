# R07-000 Settings Baseline — Research-Side Findings

**Status:** Partial source and history review; no visual parity claim
**Owner:** Product architecture
**Baseline date:** 2026-10-09

This is research-side material. It is not an implementation brief. A future
implementation checkout must omit this report and the Zed gitlink. Observable
requirements must be rewritten independently in the parity packet.

## Reference acquisition

| Item | Verified value | Evidence status |
|---|---|---|
| Parent gitlink | `zed-refrence/zed` → `3569541038dd51524b03998ba4d38d253cb54f80` | Source checked |
| Zed commit date | 2026-09-04 | Source checked |
| Local branch | `main` | Source checked |
| Tag `nightly-9` | No local tag points at the pinned commit | Source checked; label unverified |
| Zed submodule remote | `https://github.com/zed-industries/zed.git` | Source checked |
| Zed working tree | Clean | Source checked |
| Parent `.gitmodules` | Mapping added in the current working tree; clean-clone check awaits a committed parent revision | Pending |
| Labonair parent revision | `c42d33fae184b98c90516d747bfbb4844eb21992` | Source checked |
| Host OS | macOS `26.7.1` | Environment checked |
| Pinned Zed executable | `target/debug/zed` built successfully from the pinned clean checkout with `mise exec cmake@4.4.3 -- cargo build --locked -p zed`; the linker emitted an `__eh_frame` size warning | Build verified; native UI not inspected |
| Installed Zed application | `/Applications/Zed.app` reports version `1.23.2`, build `20261007.180437`; its source commit was not established and it is not accepted as the pinned runtime | Source checked |
| Labonair executable | `target/debug/labonair` exists; exact build-to-source correspondence is not recorded | Pending |
| Window bounds, theme, scale, fonts | Not captured | Pending |

The pinned executable now builds locally, but the current computer-use session
reports no application surfaces (`apps: []`); its documented native entry
points fail at runtime (`cua.getApp is not a function` and
`cua.computer` is undefined).
The existing native acceptance log records that macOS denied Screen Recording
to the runner. Neither Settings window was therefore inspected in this
session. A matching baseline still requires native access to the pinned Zed
build and the Labonair build from a recorded revision, matching macOS window
bounds/theme/scale/fonts, and inspected captures from both windows.
The workspace host operator owns restoring native-window access and the
Screen Recording grant for the runner.

## Pinned Settings navigation

The pinned Settings surface declares these top-level pages in this order:

1. General
2. Appearance
3. Keymap
4. Editor
5. Languages & Tools
6. Search & Files
7. Window & Layout
8. Panels
9. Debugger
10. Terminal
11. Version Control
12. Collaboration
13. AI
14. Network
15. Developer

The navigation also contains section entries under each page. Some workflows
open dynamic pages for providers, agents, MCP servers, skills, tool
permissions, sandboxing, audio devices, edit prediction setup, and feature
flags. `Developer` conditionally exposes feature flags when overrides are
enabled. Search, page focus, file/scope switching, nested pages, and keyboard
traversal still require runtime inspection and a complete interaction record.

## Representative category: Editor

This is a source-derived text inventory, not a complete field crosswalk. The
page includes grouped controls for autosave, which-key, multibuffer, cursor,
gutter, scrolling, scrollbars, minimap, search, hover, signature help, Vim,
toolbars, and drag/drop behavior. Examples of visible setting rows are:

| Visible label | Setting key | Control inferred from the declared value | Scope in page declaration | Evidence |
|---|---|---|---|---|
| Auto Save Mode | `autosave$` | Variant selector; changing to delayed save reveals a numeric delay field | User | Source checked |
| Delay (milliseconds) | `autosave.after_delay.milliseconds` | Numeric field | User | Source checked |
| Show Which-key Menu | `which_key.enabled` | Boolean control | User | Source checked |
| Menu Delay | `which_key.delay_ms` | Numeric field | User | Source checked |
| Double Click In Multibuffer | `double_click_in_multibuffer` | Enum selector | User | Source checked |
| Expand Excerpt Lines | `expand_excerpt_lines` | Numeric field | User | Source checked |
| Excerpt Context Lines | `excerpt_context_lines` | Numeric field | User | Source checked |

The pinned Editor page declares 70 setting rows. Each is available in the User
settings file. This index records the visible label and key; per-row defaults,
effects, and control variants still need to be joined from the settings schema
and verified in the rendered page.

| Visible label | Setting key |
|---|---|
| Auto Save Mode | `autosave$` |
| Delay (milliseconds) | `autosave.after_delay.milliseconds` |
| Show Which-key Menu | `which_key.enabled` |
| Menu Delay | `which_key.delay_ms` |
| Double Click In Multibuffer | `double_click_in_multibuffer` |
| Expand Excerpt Lines | `expand_excerpt_lines` |
| Excerpt Context Lines | `excerpt_context_lines` |
| Expand Outlines With Depth | `outline_panel.expand_outlines_with_depth` |
| Diff View Style | `diff_view_style` |
| Minimum Split Diff Width | `minimum_split_diff_width` |
| Scroll Beyond Last Line | `scroll_beyond_last_line` |
| Vertical Scroll Margin | `vertical_scroll_margin` |
| Horizontal Scroll Margin | `horizontal_scroll_margin` |
| Scroll Sensitivity | `scroll_sensitivity` |
| Mouse Wheel Zoom | `mouse_wheel_zoom` |
| Fast Scroll Sensitivity | `fast_scroll_sensitivity` |
| Autoscroll On Clicks | `autoscroll_on_clicks` |
| Sticky Scroll | `sticky_scroll.enabled` |
| Auto Signature Help | `auto_signature_help` |
| Show Signature Help After Edits | `show_signature_help_after_edits` |
| Snippet Sort Order | `snippet_sort_order` |
| Enabled | `hover_popover_enabled` |
| Delay | `hover_popover_delay` |
| Sticky | `hover_popover_sticky` |
| Hiding Delay | `hover_popover_hiding_delay` |
| Enabled | `drag_and_drop_selection.enabled` |
| Delay | `drag_and_drop_selection.delay` |
| Show Line Numbers | `gutter.line_numbers` |
| Relative Line Numbers | `relative_line_numbers` |
| Show Runnables | `gutter.runnables` |
| Show Breakpoints | `gutter.breakpoints` |
| Show Bookmarks | `gutter.bookmarks` |
| Show Folds | `gutter.folds` |
| Min Line Number Digits | `gutter.min_line_number_digits` |
| Git Gutter Width | `gutter.git_gutter_width$` |
| Custom Width | `gutter.git_gutter_width` |
| Inline Code Actions | `inline_code_actions` |
| Show | `scrollbar` |
| Cursors | `scrollbar.cursors` |
| Git Diff | `scrollbar.git_diff` |
| Search Results | `scrollbar.search_results` |
| Selected Text | `scrollbar.selected_text` |
| Selected Symbol | `scrollbar.selected_symbol` |
| Diagnostics | `scrollbar.diagnostics` |
| Horizontal Scrollbar | `scrollbar.axes.horizontal` |
| Vertical Scrollbar | `scrollbar.axes.vertical` |
| Show | `minimap.show` |
| Display In | `minimap.display_in` |
| Thumb | `minimap.thumb` |
| Thumb Border | `minimap.thumb_border` |
| Current Line Highlight | `minimap.current_line_highlight` |
| Max Width Columns | `minimap.max_width_columns` |
| Breadcrumbs | `toolbar.breadcrumbs` |
| Quick Actions | `toolbar.quick_actions` |
| Selections Menu | `toolbar.selections_menu` |
| Agent Review | `toolbar.agent_review` |
| Code Actions | `toolbar.code_actions` |
| Default Mode | `vim.default_mode` |
| Toggle Relative Line Numbers | `vim.toggle_relative_line_numbers` |
| Use System Clipboard | `vim.use_system_clipboard` |
| Use Smartcase Find | `vim.use_smartcase_find` |
| Global Substitution Default | `vim.gdefault` |
| Highlight on Yank Duration | `vim.highlight_on_yank_duration` |
| Regex Search | `vim.use_regex_search` |
| Show Edit Predictions in Normal Mode | `vim.show_edit_predictions_in_normal_mode` |
| Cursor Shape - Normal Mode | `vim.cursor_shape.normal` |
| Cursor Shape - Insert Mode | `vim.cursor_shape.insert` |
| Cursor Shape - Replace Mode | `vim.cursor_shape.replace` |
| Cursor Shape - Visual Mode | `vim.cursor_shape.visual` |
| Custom Digraphs | `vim.custom_digraphs` |

The Settings window declares a 226 `px` navigation rail and a 400 `px` minimum
content width, so the declared minimum width is 626 `px`. Its minimum height
is 240 `px`. The initial window
bounds are scaled with the UI font size. These are source-declared layout
values; they do not substitute for matching rendered measurements on macOS.

Focusable navigation entries have keyboard tab stops. Focusing a navigation
entry opens its page when no nested page is active. The source review has not
yet confirmed those behaviors in a running window. The pinned source also
declares these keyboard paths:

- Search accepts a query, filters visible navigation entries and page items,
  and exposes a clear control when the query is non-empty.
- A navigation entry can be expanded or collapsed; focus on a root category
  opens it, and focus on a section can return to its containing category.
- The Search-to-Navigation and Navigation-to-Content focus actions are
  explicit. Next/previous focus moves through visible controls, and file/scope
  choices can be focused and changed by keyboard.
- Switching User/Project/Server files rebuilds the available page set. A
  nested page remains open when that page supports the selected file scope.

The complete key-by-key traversal, focused styling, every field's defaults,
validation and reset behavior, and the rendered narrow-window/error states
remain to be recorded. The Editor category text inventory is present, but the
representative category is still incomplete.

## Four Settings-history candidates

The four exact commits named in the active task are present in the Labonair
Git object database. They are Settings-related commits, but the available
evidence does not prove they correspond one-to-one with four separately
reported visual attempts.

| Candidate | Date | Verified code change | Matching Settings artifact | Cause of visual miss |
|---|---|---|---|---|
| `3aa2327604eb3cea488a0530d647511afe64f7df` | 2026-09-25 | Search deletion handling accepts both `backspace` and `delete` when the Settings card has focus | None found | Unknown |
| `f197d32910a5ff2e53960c7c9d16eb0e7111c06f` | 2026-09-25 | Removed a 640 logical-pixel content cap so Settings rows fill available width | None found | Unknown |
| `4cca14a5685198de2faa45a20fc272e03f839143` | 2026-09-25 | Revised generated field rows, control rendering, search placement, and anchored submenu behavior | None found | Unknown |
| `c9075e958deb26d9af2e4dfce170f22ed5d26306` | 2026-10-09 | Large Settings UI and project-scope rework across Settings UI, schema/store, and UI-kit controls | None found | Unknown |

No Zed Settings capture or Labonair Settings capture is present under `shots/`.
The only tracked image is `shots/labonair.png` (4024×2474, SHA-256
`3e1d406ad59be309c16c599f77303d3bcccedfe7a45b0296e21ef334d69edcc4`); it
shows the SFTP surface and is not Settings evidence. Earlier commit code
changes explain what was attempted, but they do not establish why the rendered
result missed the visual target.

## Public documentation review

The public docs pages were checked on 2026-10-09. They are discovery inputs;
they do not replace the pinned runtime inventory.

| Page | Pinned-tree observation | Drift status |
|---|---|---|
| UI/UX checklist | No matching checklist page exists in the pinned documentation tree; the current public page provides the broad acceptance checklist | Public-doc-only; runtime relationship unverified |
| Development glossary | A glossary exists in the pinned docs and current public docs; exact semantic changes have not been reviewed | Pending |
| All Settings | A Settings reference exists in the pinned docs and current public docs; the complete field-level delta has not been computed | Pending |
| All Actions | The pinned `all-actions.md` is only a short generated-content stub, while the current public action reference contains the expanded action catalog | Public-doc/runtime extraction required |
| Language Extensions | Current public reference exists; matching pinned-runtime support and catalog changes remain to be checked | Pending |
| Extension Capabilities | Current public reference documents capability grants; the pinned extension API and grant list still need a field-by-field comparison | Pending |

Source links consulted: [UI/UX checklist](https://zed.dev/docs/development/ui-checklist),
[development glossary](https://zed.dev/docs/development/glossary),
[all settings](https://zed.dev/docs/reference/all-settings),
[all actions](https://zed.dev/docs/all-actions),
[language extensions](https://zed.dev/docs/extensions/languages), and
[extension capabilities](https://zed.dev/docs/extensions/capabilities).

## Review items that remain open

- Build or otherwise identify an executable matching the pinned Zed commit.
- Capture Settings in both apps under the same recorded macOS conditions.
- Complete the Settings field/control/default/scope/effect crosswalk.
- Match the four candidate commits to the reported attempts and recover their
  original viewport/platform artifacts if they exist.
- Compare each public docs page to the pinned runtime and record exact drift.
- Record the following review items without claiming legal clearance:

| Review item | Required question | Review owner |
|---|---|---|
| Code/license provenance | Which implementation and distribution obligations apply to the pinned reference and any independent Labonair work? | Product/legal reviewer |
| Branding and original assets | Which names, icons, artwork, and bundled assets may Labonair use or must independently replace? | Product/legal reviewer |
| Extension content and distribution | What review, permission, signing, update, and removal rules apply to third-party extension packages? | Product/legal + Security |
| Hosted services and accounts | Which identity, model, billing, telemetry, and organization data flows are in scope and under what user consent? | Product + Security |
| Remote and collaboration services | What trust grants, host boundaries, retention, and failure disclosures are required? | Security + capability owners |
| Platform packaging and updates | Which signing, notarization, update, rollback, and support obligations apply on each platform? | Release owner |
