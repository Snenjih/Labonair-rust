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
| Pane | A region of the workspace that contains tabs and has one focused active tab. Panes may be split, resized, merged, and restored. |
| Split | A workspace layout operation that divides a pane and places or moves tabs between the resulting panes. |
| Dock | A resizable side or bottom container hosted by Workspace. |
| Panel | A feature-owned view hosted in a dock. The feature owns its content and actions; Workspace owns placement and focus. |
| Command | A stable, searchable action identity contributed by the module that owns the behavior. A key binding is one way to invoke it. |
| Setting | A typed user or project value with an owner, scope, default, validation rule, persistence behavior, and visible effect. |
| Extension | An independently installed package that adds a declared capability under explicit trust, permission, version, and recovery rules. |
| DiffView | A shared view that presents a typed comparison supplied by a producer. The producer retains domain actions such as staging or saving. |
| Evidence record | A capture, test, reproducible interaction, or measurement tied to exact builds and conditions. Source inspection alone is structural evidence. |

The feature inventory must use these terms consistently. If an observed
workflow has no equivalent Labonair term, add a proposed term here before
writing its implementation packet.
