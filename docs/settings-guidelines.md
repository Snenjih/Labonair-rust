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

10. **Dense native navigation.** The Settings rail is one UI-kit tree over the
    owner-backed pages in the [Settings crosswalk](parity/settings-crosswalk.md).
    Root selection and disclosure are separate actions; section children are
    scroll anchors, not management pages. Pages start collapsed; selecting a
    root does not disclose its section anchors. Search results and deep links
    expand the page when they need to reveal a specific section.
11. **Explicit persistence scope.** The header's User/Project selector is the
    only scope choice for a Settings edit. A Project choice is available only
    with an active project; the Settings owner enforces the project whitelist,
    rejects malformed project JSON, and persists sparse overrides.
12. **Native editing and long-list behavior.** Text and numeric values use
    real UI-kit `InputState` editors when activated. Selects support focused
    keyboard opening and arrow/Enter selection. Field content uses GPUI's
    variable-height virtualized list; no page may eagerly build an unbounded
    number of rows.
13. **Native keyboard and accessibility fallback.** Interactive Settings
    controls expose visible focus and keyboard activation/navigation wherever
    GPUI 0.2.2 permits it. That GPUI version does not expose native
    accessibility roles or ARIA-style names/values; do not describe this as
    screen-reader parity. Revisit semantic roles and announcements when the
    native accessibility API becomes available.

14. **Unavailable project values are visibly inert.** In Project scope, fields
    outside the project-settings whitelist keep their displayed value and
    explanatory description, but their control cannot be focused or activated.
    Commit or cancel an active edit against its original scope before switching
    the write target; never defer a predictable whitelist failure until after
    the user attempts to change a value.

15. **Navigation is independent of persistence.** A page descriptor owns its
    stable key, title, slug, field placements, and persisted-group fallbacks.
    Rows and search routes resolve by full JSON field path. One field has one
    canonical Settings editor; explicit cross-page placement wins over a
    persisted-group fallback. New fields remain reachable through their
    owning page's fallback until their deliberate placement is recorded.

16. **Modified values identify their source.** Show the reset action beside the
    setting title and identify whether the effective override comes from User
    or Project settings. Reset clears that layer's sparse override so the next
    lower-precedence value takes effect.

17. **Setting links resolve to fields.** A JSON-backed row may copy a
    `labonair://settings/<json-path>` link. Opening a known link selects the
    owning page, scrolls to the field, and briefly highlights it; an unknown
    path must not open an unrelated Settings page.
