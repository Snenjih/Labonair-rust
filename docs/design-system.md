# Labonair Design System

**Status:** Normative
**Version:** 2

## Visual objective

Labonair should feel dense, calm, precise, and fast. The visual system uses a restrained hierarchy rather than decorative chrome. The pinned Zed build is the observable UI/UX target for clean-room parity; Labonair continues to own its implementation, tokens, and feature contracts.

## Token ownership

All colors, spacing, radii, typography, shadows, focus states, and semantic status colors come from the theme/design-token layer. Feature crates consume tokens; they do not invent global values.

Reference values from `reference-src` may be used as input during migration, but new values must be recorded in the token source rather than copied into views.

The token source is the theme/design-token implementation and its focused
tests. A view may not introduce a global color, spacing, radius, typography,
shadow, or state value inline. If a new visual value is genuinely required,
extend the token source first and record the reason in the owning task.

## Default typography

The default typography follows the frozen reference application:

- UI chrome: `Inter Variable`, 13 px, line-height `1.5`.
- Terminal: `JetBrains Mono` with `SFMono-Regular`, `Menlo`, and `monospace`
  fallbacks; 14 px, normal weight, letter-spacing `0`, line-height `1.05`.
- Editor: the same monospace stack at 13 px with line-height `1.55`.

Terminal weight remains a value setting with Normal, Medium, and Bold options.
Font smoothing is not a persisted setting; native GPUI text rasterization owns
that platform-specific behavior.

Tokens are semantic rather than feature-specific: a feature consumes, for
example, surface, text, focus, selected, danger, and spacing roles instead of
defining a `host-row-blue` or `terminal-padding` value. Color-theme and
icon-theme registries provide the active values; feature views do not select
or persist themes themselves.

## UI-kit requirement

The following must come from `labonair-ui-kit`:

- buttons and icon buttons;
- text fields and search fields;
- multiline text-area frames for growing composition inputs;
- search clear actions with the shared keyboard focus and tooltip treatment;
- lists and list rows;
- dropdowns, menus, and context menus;
- popovers and dialogs;
- tabs and tab controls;
- continuous numeric sliders;
- badges, indicators, disclosure rows, tooltips, and keyboard hints;
- standard dividers, scroll containers, and empty/loading states.

Centered modal overlays use `modal_overlay`; owner-rendered dialog contents use
`dialog_surface`. The shared frame supplies the fixed scrim, centered placement,
card colors, border, radius, spacing, and shadow. Feature owners retain dialog
content, focus entry and return, dismissal, validation, and action order. A
dialog placed beside a trigger or at a viewport edge uses `dialog_surface`
inside its owner-provided positioner rather than introducing another card style.

Buttons share one compact size scale: 16, 18, 22, 28, and 32 logical pixels
at the reference UI font size. Their dimensions scale with the active UI font
and density, and their corners use the small radius token. The default
appearance is subtle; filled, outlined, outlined-ghost, error-tinted, link,
and transparent treatments are explicit choices. Enabled controls define
hover, pressed, and visible focus states. Disabled controls preserve
their geometry, lose their Tab stop, and use disabled foreground treatment.

Icon buttons use the same shared state model. The UI-kit builder supports wide
and square geometry, caller-owned selected and disabled state, an alternate
selected glyph, a status indicator, opacity, and a tooltip. Icon-only actions
provide a short tooltip label. The glyph can be sized independently while
remaining scaled with the active font and density; indicators can carry a
border token to stay distinct from the button surface. GPUI 0.2.2 does not expose modality-specific
focus or native accessibility labels; visible focus and tooltips are the
available cues.

Workspace and docked tab rows use the UI-kit `tab_item` control. It provides
horizontal and vertical geometry, selected, dirty, busy, peek, focused, and disabled
states, a shared close button, and Enter/Space plus orientation-aware arrow
activation. Workspace keeps tab ordering, drag/drop, persistence, rename, and
close decisions in its owner; those behaviors extend the shared row instead
of replacing its styling.

Reusable controls expose a real disabled state: preserve their normal geometry,
remove activation and keyboard focus, and use the shared disabled treatment.
The select trigger and text-field surface have disabled variants alongside
buttons, checkboxes, segmented controls, and numeric fields.
Text-field frames provide shared compact (24-pixel) and standard (32-pixel)
sizes. Their geometry scales with UI font size and density. Standard settings
text inputs have a 256-pixel minimum width; the UI kit owns padding, border,
radius, fill, and focus treatment.

Render an unavailable action with `button_disabled`; it keeps button geometry
while removing hover paint, pointer affordance, and the Tab stop. Do not attach
an activation handler to it.

Feature modules may compose these primitives and provide domain content. They may not create local variants with different padding, colors, hover behavior, or typography without extending the shared component.

The Settings surface specifically composes `TreeRow`, `NumberField`, the
shared search-field chrome, `text_field_surface`, text and icon buttons,
segmented controls, select trigger/popovers, `field_input`, badges, and
`keybinding_hint`.
`NumberField` has no filled slider track in Settings: it is a compact minus /
editable value / plus control. Numeric editing uses a native `InputState` and
commits only validated, clamped values.

Feature-owned editable text uses the shared `InputState` editor inside a
UI-kit field frame. The owner keeps the value, validation, and commit rules;
view-level key handlers do not emulate text editing or caret movement.

Settings value and owner-action rows share one section layout. Each row uses
16 logical pixels of top and bottom padding; the final row in a section uses
40 pixels below. Draw a divider only between rows in the same section. Apply
these dimensions through `Palette::control_space` so UI font size and density
scale the row geometry together.

Continuous ranges use the shared `Slider` surface. Feature owners retain its
`SliderState`, persistence, units, and update behavior. Pair pointer dragging
with a keyboard-operable numeric control when the range must also support
precise keyboard adjustment.

Select popovers use a caller-owned GPUI `ListState` and render option rows
virtually, with a 320 logical-pixel maximum viewport. While open, Settings
supports arrow navigation, Home/End, Page Up/Down, Enter/Space to choose, and
Escape to dismiss; moving the highlight scrolls the selected row into view.
Anchored rich-content popovers and menu cards invoke their owner-provided
dismissal callback on Escape while popup content has keyboard focus, and stop
propagation so the same key does not activate a second dismissal path. The
Settings select remains controlled by the Settings view, which owns its
keyboard highlight and Escape behavior.

All anchored menus and popovers use the shared UI-kit positioner. It measures
the rendered panel, flips each axis toward the side with more available room,
clamps the result inside an 8 logical-pixel viewport inset, and limits the
panel to the usable viewport size. Apply this behavior to pointer menus,
trigger selects, and rich-content popovers; feature owners provide the anchor
and content, not separate placement math.

## Interaction states

Every interactive component must define normal, hover, pressed, selected, focused, disabled, and error states where applicable. Keyboard focus must remain visually distinguishable.

GPUI 0.2.2 does not expose native accessibility roles or ARIA-style
name/value/expanded properties. Components must still provide visible focus
and keyboard behavior where possible; this is a documented limitation, not
screen-reader parity. Reassess it when GPUI exposes a native semantics API.

GPUI 0.2.2 also does not distinguish keyboard focus from pointer focus in its
style API. Shared controls keep a visible focus treatment for both input
paths; modality-specific focus styling remains pending a GPUI capability.

## Layout rules

- Use a consistent spacing scale.
- Keep permanent chrome minimal.
- Align popovers to their triggering element in the same window coordinate space.
- Use progressive disclosure for secondary actions.
- Lists must define selection, focus, empty, loading, and error behavior.
- Long lists must use a bounded, scrollable, or virtualized container.
- User-facing error explanations belong in the notification dropdown. A local
  error state may still prevent an invalid action or render an invalid control,
  but it must not create a second inline error message for the same failure.

## Surface ownership

Feature views own composition and domain content. The UI kit owns the
interaction mechanics and visual variants of reusable controls. The shell
owns only placement of permanent zones and overlay anchors. A dropdown or
popover must be positioned from the triggering element's bounds in the same
window coordinate space; it must never choose a fixed opposite-side location.

## Review rule

A UI change is incomplete until it has been checked at normal, narrow, focused, empty, loading, and error states. Screenshots or a visual comparison are required for shell and shared-component changes.

Exceptions to the UI-kit rule require all of: a documented reason, a
composition of existing tokens, an owner, and a follow-up decision about
whether the pattern belongs in `labonair-ui-kit`. One-off local styling is not
an exception.
