# Font-Size Increase and Reset Action Classification

**Status:** Static action, scope, and Settings requirements; executable routes,
native dispatch, and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Themes (`labonair-theme`) owns live application, shared buffer,
and Agent typography state. Settings owns configured values. Editor,
Terminal, and Agent UI consume their respective typography scopes.

## Evidence boundary

This packet independently states the user-visible increase and reset goals,
including the distinct four-scope Reset All workflow. Static action, keymap,
menu, and Settings inspection establishes the recorded inputs and effects;
it does not establish native dispatch, rendered changes, persistence recovery,
or visual parity. Same-session research and packet authoring are independent
redevelopment, not separated-room clean-room work, as recorded in the
[parity README](README.md#provenance-and-source-boundary). Native macOS runtime
and paired capture access are unavailable in this session.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner boundary | Themes owns live UI, shared buffer, and Agent typography state and adjustment. Settings (`labonair-settings`) stores configured values; `labonair-theme-ui` applies them through ThemeStore. Editor and Terminal consume shared buffer typography; Terminal may override its size. Agent UI consumes distinct Agent UI and buffer scopes. Keymap owns binding and context resolution. Keep behavior out of Settings and the shell. |
| Buffer increase entry | [`ACT-MACOS-zed-IncreaseBufferFontSize`](action-requirements.md) is global on `cmd-=` / `cmd-+` for macOS and `ctrl-=` / `ctrl-+` for Linux; Windows uses `ctrl-=` / `ctrl-shift-=`. Default arguments are `persist:false`. View > Zoom In also invokes the buffer action. The macOS and Linux JetBrains profile adds Editor-context `ctrl->` with `persist:true`. |
| UI increase entry | [`ACT-MACOS-zed-IncreaseUiFontSize`](action-requirements.md) uses the matching platform key in the more-specific `Onboarding` and `Welcome` contexts, with `persist:false`. There it shadows the global buffer action and increases app UI typography. |
| Buffer reset entry | [`ACT-MACOS-zed-ResetBufferFontSize`](action-requirements.md) is bound globally to `cmd-0` on macOS and `ctrl-0` on Linux/Windows. View > Reset Zoom also invokes it. Default argument is `persist:false`. |
| UI reset entry | [`ACT-MACOS-zed-ResetUiFontSize`](action-requirements.md) uses `cmd-0` / `ctrl-0` only in `Onboarding` and `Welcome`, where it shadows the global buffer reset. Default argument is `persist:false`. |
| Reset All entry | Registered action [`ACT-REGISTRY-LINUX-zed-ResetAllZoom`](registered-actions-unbound.md) has no default-keymap binding; View > Reset All Zoom invokes it with `persist:false`. It resets UI, shared buffer, Agent UI, and Agent buffer typography. Git Commit and Markdown Preview size overrides are separate scopes and are not included. |
| Adjustment and reset | Increase changes the chosen effective size by 1 px and clamps it to 6–100 px. A transient increase adds or updates a runtime override and refreshes open windows without editing Settings. A transient reset removes that scope's runtime override and returns to its current configured base, which may be a customized value. Repeated increases stop at 100 px. A persistent buffer increase writes `theme.buffer_font_size`; a persistent UI increase writes `theme.ui_font_size`. Persistent reset clears the matching override and applies its configured fallback. |
| Context precedence | Outside Welcome and Onboarding, the global increase/reset key changes buffer typography. Inside those more-specific contexts, the UI action wins and changes app UI typography. Check focused view, overlays, modal surfaces, profile layers, and user overrides before claiming dispatch parity. |
| Settings and persistence | Pinned defaults are buffer 15 px, UI 16 px, Agent UI unset/inherited from UI, and Agent buffer 12 px. Terminal inherits the shared buffer size unless its own size is configured. The local Settings model has duplicate `appearance.appFontSize`, `appearance.bufferFontSize`, `editor.editorFontSize`, and terminal font paths, with different defaults/ranges and a ThemeStore bridge. Reconcile the canonical Labonair values and the distinct Agent scopes before implementation; do not add another size field or collapse the explicit Terminal override. |
| Related actions | The separate [font-size decrease packet](action-classification-font-size-decrease.md) records the paired buffer/UI decreases and conditional Editor wheel gesture. The four bound increase/reset IDs and menu-only Reset All are listed below and in the parity action catalogs. |
| Labonair action gap | Labonair exposes configurable typography values but has no working increase or reset action, View Zoom menu route, or matching typed Theme action. R08-002 removed old no-op Zoom menu, palette, and default-keymap routes. The active parity goal now requires real behavior; do not restore those dead `CommandId::Zoom*` paths. |
| Current value model | Labonair Appearance UI/shared buffer values default to 13 px; Editor uses a distinct 13 px value and Terminal has a distinct 14 px default. Settings UI constrains Appearance values to 9–24. The Theme bridge feeds separate runtime structures. Resolve effective precedence and choose one canonical buffer value before implementation. |
| Events and registries | Provide typed Themes-owned increase and reset operations. Route global versus Welcome/Onboarding bindings through Keymap; expose View menu contributions through the existing global-menu registry. Persistent changes use Settings' value owner. Do not add a general font-action registry for one owning module. |
| Notifications and failure | Transient changes perform no I/O. A persistent settings failure must leave the displayed value truthful and provide recovery through the notification center; successful adjustments need no toast. Pinned write failure behavior and Labonair recovery remain unverified. |
| UI kit | Reuse the existing application menu and UI-kit-backed Settings fields. Do not add local +/- controls as substitutes for the actions. UI-font changes must update shared typography/spacing consistently. |

## Action requirements and current route

| Requirement ID | Binding, context, and argument | User-visible outcome | Current Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-zed-IncreaseBufferFontSize`](action-requirements.md) | macOS `cmd-=` and `cmd-+`; Linux `ctrl-=` and `ctrl-+`; Windows `ctrl-=` and `ctrl-shift-=`; global; default `persist:false`. JetBrains Editor `ctrl->` uses `persist:true`. | Increase shared Editor/Terminal buffer text by 1 px, clamped at 100 px. | Settings feed typography but no executable increase action or View > Zoom In route exists. R08-002 removed the old no-op Zoom path. |
| [`ACT-MACOS-zed-IncreaseUiFontSize`](action-requirements.md) | Matching platform `cmd`/`ctrl` equals/plus keys in `Onboarding` and `Welcome`; `persist:false`. | Increase app UI typography by 1 px; this more-specific action shadows global buffer increase. | UI font is configurable, but no executable context-specific increase action exists. |
| [`ACT-MACOS-zed-ResetBufferFontSize`](action-requirements.md) | macOS `cmd-0`; Linux/Windows `ctrl-0`; global; default `persist:false`. View > Reset Zoom uses the same argument. | Restore the configured shared buffer base by clearing the transient override. A persistent invocation clears the configured buffer override. | No executable reset action or View > Reset Zoom route exists. The old no-op Zoom route remains removed. |
| [`ACT-MACOS-zed-ResetUiFontSize`](action-requirements.md) | `cmd-0` / `ctrl-0` in `Onboarding` and `Welcome`; default `persist:false`. | Restore configured UI typography; the scoped action shadows global buffer reset. | UI font is configurable, but no executable context-specific reset action exists. |
| [`ACT-REGISTRY-LINUX-zed-ResetAllZoom`](registered-actions-unbound.md) | No default key binding; View > Reset All Zoom invokes `persist:false`. The action schema also supports `persist:true`. | Restore UI, shared buffer, Agent UI, and Agent buffer configured sizes. Leave Git Commit and Markdown Preview scopes unchanged. | No Reset All action handler or global View-menu route exists in Labonair. |

## Required user-flow and state acceptance

- In a normal Editor and Terminal, press the global increase and reset keys;
  verify a one-pixel increase, configured-base restoration, and terminal
  inheritance versus an explicit Terminal size.
- In Welcome and Onboarding, use the same keys and confirm UI typography wins
  over the global buffer action. Verify scale across open windows and inspect
  focus transitions, modal surfaces, and user/keymap profile overrides.
- Increase repeatedly to the 100 px upper bound. Confirm no value exceeds the
  bound, no-op-at-bound state is stable, and Settings remains unchanged for
  default `persist:false` routes.
- Change configured Settings while a transient override is active; verify the
  transient value clears and reset restores the latest configured base, not a
  hard-coded default. Restart and verify transient values are not persisted.
- Use the JetBrains Editor binding and a user binding with `persist:true`;
  verify the correct user-scoped value changes, reloads, respects the clamp,
  and reports a failed write without displaying it as committed.
- Set distinct UI, buffer, Agent UI, and Agent buffer sizes; run Reset All.
  Verify all four fallback values apply, including Agent UI inheritance and
  Agent buffer's configured default, while Git Commit and Markdown Preview
  sizes are unaffected.
- Confirm the View menu exposes Zoom In, Reset Zoom, and Reset All Zoom with
  their distinct scopes and transient defaults. Confirm Reset All has no
  default key binding and remains separately discoverable as a menu action.
- Compare pinned buffer/UI/Agent defaults with Labonair's duplicate font
  values. Resolve the canonical local value path and preserve Terminal's
  explicit override before claiming parity.
- Capture paired native macOS states for increase, each reset, Reset All,
  Welcome/Onboarding precedence, Terminal inheritance/override, all four
  typography scopes, configured/transient/persistent settings, and narrow
  layout. Record exact builds, OS, bounds, theme, scale, focus/context, values,
  keymap profile, and action argument.

## Acceptance evidence still required

Static inventories establish the recorded bindings, context split, arguments,
menu routes, font scopes, and local action/value gap only. Verify native
dispatch, visible changes, configured-base restoration, transient lifetime,
persistent writes and failure recovery, all four Reset All scopes, and paired
macOS captures on the pinned build and Labonair. The action rows remain
`Pending` for runtime acceptance; static typography values and R08-002's
historical no-op removal are not parity evidence.
