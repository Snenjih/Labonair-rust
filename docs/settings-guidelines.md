# Settings UI Guidelines

**Status:** Normative
**Version:** 1
**Related:** [`settings.md`](settings.md), [`design-system.md`](design-system.md)

These rules define how the Settings surface may evolve. Settings is a value
editor, not a general application-management window.

1. **One navigation model.** Use category → section → optional detail page.
   Do not add a second sidebar, modal taxonomy, or feature-specific navigation
   inside Settings.
2. **Typed fields first.** Every ordinary setting is represented by a typed
   field with a stable key, default, scope, validation, description, and reset
   behavior.
3. **No management pages.** Hosts, credentials, themes, icon themes, keymaps,
   notifications, transfers, and command registration are opened through their
   own canonical surfaces.
4. **Persisted selections are not ownership.** A selected theme ID or similar
   value may be persisted by the settings storage layer while the owning module
   controls its registry and user flow.
5. **Custom panes are exceptional.** Use a custom pane only when the interaction
   cannot be expressed as fields. It must keep the standard Settings chrome and
   UI-kit components.
6. **No duplicate controls.** A value has one canonical editor. Other surfaces
   may preview or invoke an action but must not create a second settings form.
7. **Notifications are global.** Validation and operational failures that need
   user attention enter the notification registry. Do not add feature-local
   toast or inline error systems.
8. **Scope is explicit.** Document whether a field is global, user, profile,
   project, or workspace scoped. Do not silently write project data into user
   settings.
9. **Removal is complete.** Removing a field requires checking defaults,
   migrations, serialization, UI metadata, command IDs, tests, and documentation.
