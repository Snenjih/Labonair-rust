# Font-Size Decrease Action Classification

**Status:** Static action, scope, and Settings requirements; executable routes,
native dispatch, and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Themes (`labonair-theme`) owns live application typography and
font-size adjustment. Settings owns configured values. Editor and Terminal
consume buffer-size typography, with a terminal-specific override.

## Evidence boundary

This packet independently states the user-visible effects and current
Labonair gaps. Static inspection establishes the default actions, their
contexts and arguments, configured values, and current routes; it does not
prove native dispatch or rendered size changes. Same-session research and
packet authoring are independent redevelopment, not separated-room clean-room
work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Native macOS runtime and paired capture access are unavailable in this session.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner boundary | Themes owns the app-wide UI and shared buffer typography state and adjustment. Settings (`labonair-settings`) stores configured values; `labonair-theme-ui` applies them through the ThemeStore. Editor and Terminal consume the shared buffer size; Terminal may use its own configured size. Editor owns the conditional mouse-wheel input and its Editor settings gate, and requests the same transient Theme-owned buffer adjustment. Keep action behavior out of Settings and the shell. |
| Buffer action entry | [`ACT-MACOS-zed-DecreaseBufferFontSize`](action-requirements.md) is bound globally to `Command+-` on macOS and `Ctrl+-` on Linux/Windows, with `persist:false`. The application View menu's Zoom Out item invokes the same decrease. In an eligible Editor, holding the primary modifier (Command on macOS; Control on Linux/Windows) while scrolling the mouse wheel also adjusts buffer size when `editor.mouse_wheel_zoom` is enabled. Wheel adjustment is transient and does not use the action's `persist` argument. |
| UI action entry | [`ACT-MACOS-zed-DecreaseUiFontSize`](action-requirements.md) uses the same key only in the more-specific `Onboarding` and `Welcome` contexts, also with `persist:false`. There it overrides the global buffer action. It has no mouse control. |
| Adjustment | Each action reduces its own effective font size by 1 px and clamps it to the inclusive 6–100 px range. The UI-font action changes the font scale used throughout the application UI and all open windows, though its default key route exists only in Onboarding and Welcome. The buffer action and eligible wheel gesture change the shared buffer text size; Terminal inherits it unless its own font-size setting overrides it. |
| Transient argument | `persist:false` is the default action argument and the value in the default keymap and View menu. It adjusts the current runtime size, refreshes open windows, and does not write Settings. The adjustment lasts until app restart or a change to its configured setting, which clears the transient adjustment. At the lower bound, further decreases keep the size at the bound. |
| Persistent argument | `persist:true` adjusts the user's configured value instead, clamped to the supported range. The buffer action writes `theme.buffer_font_size`; the UI action writes `theme.ui_font_size`. The JetBrains profile provides an Editor-context `Ctrl+<` buffer-decrease binding with `persist:true`; profile and user overrides must preserve the argument's effect. |
| Context precedence | Outside Onboarding and Welcome, the global decrease key affects buffer text. Inside those contexts, the UI-font binding wins and changes the app UI size instead. Verify focus transitions, overlays, modal surfaces, and alternate profiles before claiming dispatch parity. |
| Settings and persistence | The pinned settings inventory links [`SET-appearance-buffer-font-size`](settings-requirements.md), default 15 and user-scoped, [`SET-appearance-ui-font-size`](settings-requirements.md), default 16 and user-scoped, and [`SET-editor-mouse-wheel-zoom`](settings-requirements.md), false by default. The wheel gesture requires this setting and an Editor instance that permits zoom; embedded editors may disable it. [`SET-terminal-terminal-font-size`](settings-requirements.md) is unset by default and inherits the buffer size. Persistent actions change the matching user setting; transient actions and wheel adjustments restore the configured base after restart. Settings changes clear the relevant transient adjustment. |
| Related actions | The companion [font-size increase and reset packet](action-classification-font-size-increase-reset.md) classifies the four bound Increase/Reset requirements and the View-menu-only Reset All action. |
| Labonair Settings gap | Labonair retains font-size settings but no working decrease action, mouse-wheel zoom setting, or gesture route. R08-002 removed Zoom In/Out/Reset menu, palette, and default-keymap routes because they had no handler and no product decision. The active parity goal now explicitly requires real font-size adjustment; that historical removal is not evidence the workflows are out of scope. Do not restore the old no-op command IDs. |
| Current value model | Local Settings contains UI and shared buffer sizes under Appearance, plus separate Editor and Terminal sizes. Its Appearance defaults are 13, Editor defaults to 13, and Terminal defaults to 14; the Settings UI allows 9–24 for the Appearance sizes. The Theme bridge currently feeds editor and Appearance typography into distinct runtime structures. Resolve their effective precedence and choose one canonical buffer value before implementation; do not add another field or silently merge values. |
| Events and registries | Provide two typed Theme-owned actions with the platform defaults and contexts above. A transient change refreshes all open windows; a persistent change flows through the Settings value owner and live ThemeStore update. Editor routes the enabled primary-modifier wheel gesture through the typed buffer adjustment boundary. One Theme action contribution is sufficient; do not add a generic registry. |
| Notifications and failure | Successful transient adjustment needs no notification. A persisted change must report failure through the notification center without presenting an unsaved value as committed, and leave the action retryable. Settings edits or reset clear a stale transient override. Runtime save failure and recovery have not been observed in this session. |
| UI kit | Keep the existing Settings controls and application menu on shared UI-kit contracts. Use the existing Keymap surface for discoverability and overrides. Do not add feature-local +/- buttons for these two actions. Shared typography and spacing must respond together when the UI font size changes. |

## Action requirements and current route

| Requirement ID | Binding, context, and argument | User-visible outcome | Current Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-zed-DecreaseBufferFontSize`](action-requirements.md) | macOS `Command+-`; Linux/Windows `Ctrl+-`; global; `persist:false` | Decrease shared buffer text by 1 px. An eligible Editor also decreases it on a negative mouse-wheel delta with the primary modifier held and `editor.mouse_wheel_zoom` enabled. The setting is the Terminal fallback when its specific size is unset. | Settings values feed Theme typography, but no decrease action, View-menu route, mouse-wheel setting, or gesture exists. R08-002 removed the previous no-op Zoom commands. |
| [`ACT-MACOS-zed-DecreaseUiFontSize`](action-requirements.md) | macOS `Command+-`; Linux/Windows `Ctrl+-`; `Onboarding` and `Welcome`; `persist:false` | Decrease application UI text/scale by 1 px; this scoped action shadows the global buffer decrease. | UI font size is configurable in Settings, but no executable context-specific decrease action exists. |

The same key therefore has two outcomes based on the active context. Buffer
decrease is global and appears as View > Zoom Out; UI decrease is limited to
Onboarding and Welcome. The default argument is transient for both. A selected
JetBrains profile adds an Editor-context buffer shortcut that persists its
change. Separately, the optional primary-modifier wheel gesture changes buffer
size only over Editor instances that permit it. Increase and reset behavior is
specified in the companion packet.

## Required user-flow and state acceptance

- Press the default decrease key in a normal Editor, Terminal with no specific
  font size, Terminal with its own size configured, Welcome, and Onboarding.
  Verify the target value and visible text/geometry change in each context.
- Confirm the Welcome/Onboarding action shadows the global buffer decrease and
  changes UI scale across all open windows. Check focus changes, popovers,
  dialogs, and more-specific user bindings.
- Decrease repeatedly through the lower bound; verify 1 px steps, no value
  below 6 px, no overflow at limits, and unchanged line-height ratios unless
  their own setting requires otherwise.
- With `editor.mouse_wheel_zoom` false, verify primary-modifier scrolling does
  not zoom and normal wheel scrolling still scrolls the document. Enable it and
  verify positive/negative wheel deltas increase/decrease size by one pixel in
  eligible Editors, while embedded Editors that disable zoom continue scrolling.
  Check Command on macOS and Control on Linux/Windows.
- Compare the configured value before and after default `persist:false`, app
  restart, Settings edits, and the separately bound reset actions. Verify
  transient adjustments return to the configured value and setting edits clear
  them.
- Use the JetBrains profile and a user binding with `persist:true`. Verify
  one-step change writes the correct user-scoped Setting, respects the 100 px
  upper/lower bounds, reloads, and reports write failures accurately.
- Compare the pinned defaults (buffer 15, UI 16) with Labonair's current
  Appearance/Editor/Terminal paths. Resolve which local setting supplies the
  effective buffer size before claiming parity; preserve an explicit Terminal
  override.
- Check default bindings and context precedence on macOS, Linux, and Windows,
  including keymap profiles and user overrides. Confirm the buffer action is
  discoverable from the intended View menu and Keymap surfaces. Verify the
  mouse-wheel setting is user-scoped, defaults false, and updates live.
- Capture paired native macOS states for default and minimum buffer size, UI
  scale in multiple windows, Terminal inheritance/override, Welcome/Onboarding
  context routing, disabled/enabled Command+wheel behavior, and narrow-window
  reflow. Record exact builds, OS, bounds,
  theme, scale, focus/context, values, profile, and action argument.

## Acceptance evidence still required

Static source and settings inventories establish the action bindings,
arguments, target scopes, conditional wheel gesture, limits, and current
Settings/action gap only. Verify
the actual action dispatch, visible font changes, transient restart behavior,
persistent writes, settings-change reset, context precedence, failure recovery,
and paired captures on the pinned build and Labonair. No action or visual
parity claim is made from the existing static font values or the historical
R08-002 no-op command removal.
