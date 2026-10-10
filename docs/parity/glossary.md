# Labonair Parity Glossary

**Status:** Independent product language for R07-000
**Version:** 1
**Owner:** Product architecture

These definitions describe Labonair's user-visible concepts. They are written
for Labonair and do not transfer reference implementation names or structure.

| Term | Labonair meaning |
|---|---|
| Workspace | The current work context. It may be tied to a project or used temporarily without one. |
| Project | A durable local or remote work root that can own project-scoped settings and session state. |
| Tab | A named, durable view instance managed by Workspace. A tab may display an Editor, Terminal, Settings, DiffView, or another registered view. |
| Tab item | The shared UI control for one tab. It presents the owner-provided icon and label plus selected, dirty, busy, peek, disabled, and close affordances; Workspace owns the instance and its lifecycle. |
| Pane | A region of the workspace that contains tabs and has one focused active tab. Panes may be split, resized, merged, and restored. |
| Split | A workspace layout operation that divides a pane and places or moves tabs between the resulting panes. |
| Dock | A resizable side or bottom container hosted by Workspace. |
| Panel | A feature-owned view hosted in a dock. The feature owns its content and actions; Workspace owns placement and focus. |
| Command | A stable, searchable action identity contributed by the module that owns the behavior. A key binding is one way to invoke it. |
| Setting | A typed user or project value with an owner, scope, default, validation rule, persistence behavior, and visible effect. |
| Setting source | The layer supplying the effective value for a setting, such as its default, User settings, or Project settings. |
| Settings owner action | A registered Settings row that opens a capability's canonical management surface without duplicating its editor, catalog, or persistence. Its stable ID and searchable metadata drive navigation and search. |
| Setting link | A Labonair URL containing a stable JSON field path; opening it selects the owning Settings page and reveals that field. |
| Button | An action control with a label and optional icons. Its appearance expresses emphasis or availability; its owner supplies the action. |
| Icon button | A button whose visible content is an icon. It may be wide or square, selected or disabled, may show a bordered indicator, and has a tooltip naming its action. |
| Text field | An editable single-line value with selection, caret, paste, input-method, focus, and validation behavior. |
| Search field | A text field whose value filters or finds content. Its clear action is part of the same control. |
| Select | A value control with a trigger and a keyboard-navigable option list. The owner supplies options and persists the chosen value. |
| Slider | A continuous range control whose owner supplies the current value, range, units, and update behavior. |
| Menu | A list of actions opened from a trigger or pointer location. Rows define checked, disabled, destructive, nested, and keyboard states where applicable. |
| Popup | A floating surface anchored to an element or pointer location, positioned within its window viewport, and dismissed by its owner. |
| Dialog | A blocking surface for a decision or focused input, with a defined initial focus, dismissal path, and action order. |
| Dialog surface | The UI-kit frame for dialog content. The owning feature supplies the content, focus lifecycle, dismissal, and actions. |
| Focus indicator | The visible treatment that identifies the control receiving keyboard input. |
| Disabled action | An unavailable action that remains understandable, preserves its control geometry, and cannot receive focus or activate. |
| Extension | An independently installed package that adds a declared capability under explicit trust, permission, version, and recovery rules. |
| DiffView | A shared view that presents a typed comparison supplied by a producer. The producer retains domain actions such as staging or saving. |
| Evidence record | A capture, test, reproducible interaction, or measurement tied to exact builds and conditions. Source inspection alone is structural evidence. |

The feature inventory must use these terms consistently. If an observed
workflow has no equivalent Labonair term, add a proposed term here before
writing its implementation packet.
