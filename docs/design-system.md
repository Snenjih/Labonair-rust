# Labonair Design System

**Status:** Normative
**Version:** 2

## Visual objective

Labonair adopts **Ciel's design system** (clean-room: measured values and rules,
no source copied): calm, dense, precise, fast. Surfaces separate by **fill
contrast**, not by borders and shadows. The pinned Zed build remains the
behavioral reference for parity; Ciel is the visual reference. Labonair owns
its implementation, tokens, and identity.

Core rules (Ciel `DESIGN.md` §2, §6–§10):

- **Two surface levels.** Chrome (`surface.toolbar/title_bar/status_bar`,
  `sidebar.background`) holds the titlebar, statusbar, and docks; the main
  area is `core.background`. Inside the main area, separate by spacing and at
  most a `core.card`/`core.muted` fill — never border *and* shadow together,
  never a card in a card.
- **Only floating layers carry elevation:** menus, dropdowns, popovers,
  tooltips use a 1px `border.variant` hairline plus the popover shadow;
  dialogs use a 1px `border.strong` hairline and no shadow.
- **One active state.** Selected/active rows, tabs, nav items, and toggles use
  `surface.sunken` plus a semibold label. Hover is `core.accent`. No accent
  tint, no side stripe, no underline. The blue `core.primary` accent is for
  links, toggled markers, and genuine "changed" highlights only.
- **The primary action is neutral.** `ButtonVariant::Action` uses
  `surface.action` (dark in light themes, light in dark themes), once per view.
- **Every button has chrome** (fill or outline) except breadcrumb-style
  navigation text; icon-only buttons may be ghost.

## Token ownership

All colors, spacing, radii, typography, shadows, focus states, and semantic status colors come from the theme/design-token layer. Feature crates consume tokens; they do not invent global values.

Reference values from `reference-src` may be used as input during migration, but new values must be recorded in the token source rather than copied into views.

The token source is the theme/design-token implementation and its focused
tests. A view may not introduce a global color, spacing, radius, typography,
shadow, or state value inline. If a new visual value is genuinely required,
extend the token source first and record the reason in the owning task.

## Surfaces and the optional translucent material

By default every surface is opaque and the window uses
`WindowBackgroundAppearance::Opaque`. The setting
`appearance.translucentSurfaces` (Settings › Appearance › Density & Motion,
default off) switches the main and Settings windows to GPUI's native blurred
material and paints surfaces with the `labonair-theme-tokens::VibrancyLayer`
opacities below. Toggling it applies live to every open window
(`labonair_theme_ui::window_background`).

| Surface role | Glassy | Image + Classic | Image + Glassy | Use |
|---|---:|---:|---:|---|
| Canvas | 0.92 | 0.78 | 0.60 | Workspace, editor and Settings content background |
| Toolbar | 0.82 | 0.78 | 0.60 | Titlebar and statusbar chrome |
| Sidebar | 0.86 | 0.78 | 0.60 | Navigation rails and docks |
| Terminal | 1.00 | 0.74 | 0.55 | Terminal surface fill (its own opacity setting still applies) |
| Card | 0.94 | 0.86 | 0.74 | Raised panels and contained surfaces |
| Control | 0.94 | 0.88 | 0.80 | Input surfaces |
| Popover | 0.97 | 0.97 | 0.94 | Menus, dropdowns, and floating cards |
| Divider | 0.56 | 0.56 | 0.50 | Structural borders and hairlines |

`appearance.translucentSurfaces` is the Glassy/Classic interface style (Glassy =
on). While a background image covers the window the shell sets
`ThemeMetrics::background_active`, and `labonair-theme-tokens::SurfaceMaterial`
(`Opaque`, `Glassy`, `ImageClassic`, `ImageGlassy`) selects the column above.
The image layer is the first child of the shell root, beneath all chrome; the
surfaces above are translucent so it reads through. The image opacity is the
slider mapped through `slider^0.6` (default 30% renders at about 0.49), and the
tint opacity is clamped when the tint colour has too little contrast against the
UI foreground. Popovers stay near opaque so menus are readable on any image.

`VibrancyLayer::resolve(color, material)` is the only entry point (a plain
`bool` converts to `Opaque`/`Glassy`); with an opaque material it returns the
theme color untouched. Read the material through `UiTheme::surface_material()`
or `ThemeStore::surface_material()`, never from `translucent_surfaces` alone.
Keep the `gpui-component::Root` clear only while Glassy; Classic (with or
without an image) paints `core.background` there, under the image layer.
Surfaces with their own opaque renderer (terminal cell backgrounds) must
consult `ThemeStore::terminal_background()` / the material to stay translucent
while an image shows.

### Ciel token map

| Ciel token | Labonair token |
|---|---|
| `--surface` | `core.background`, `core.popover` |
| `--surface-chrome` | `surface.toolbar/title_bar/status_bar`, `sidebar.background` |
| `--surface-muted` | `core.card`, `core.muted`, `core.secondary` |
| `--surface-hover` | `core.accent` |
| `--surface-hover-strong` | `surface.hover_strong` |
| `--surface-sunken` (selected) | `surface.sunken` |
| `--line`, `--line-strong`, `--line-subtle` | `core.border`, `border.strong`, `border.subtle` |
| `--input` / `--field-bg` / `--field-ring` | `core.input` / `Palette::field_bg` / `border.focused` |
| `--accent` | `core.primary` |
| `--action-surface` family | `surface.action`, `action_foreground`, `action_hover`, `action_active` |
| `--radius-sm/md/lg/xl/3xl/bubble/modal` | `RadiusScale` `sm/md/lg/xl/xl2/xl3/xl4` = 4/8/12/14/20/22/30 |
| `--dur-fast/base/slow` | `Animation` 120/160/200 ms |

Built-in themes are `Theme::light()` (warm neutral) and `Theme::dark()`
(neutral dark); the embedded `labonair.json` is regenerated from them
(`REGEN_BUILTIN_THEME=1 cargo test -p labonair-theme builtin_json`).


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

Buttons share Ciel's control scale: 16 (inline), 24 (`Xs`), 28 (`Default`,
the minimum click target), 32 (`Sm`), and 36 (`Lg`, the standard form button)
logical pixels at the reference UI font size, with 12/13/13/14 px labels and
medium weight. Their dimensions scale with the active UI font and density, and
their corners use the `md` control radius (8). The default appearance is
subtle (ghost); filled (`secondary`), outlined, outlined-ghost, error-tinted
(`destructive`), the neutral `Action` primary, link, and transparent
treatments are explicit choices. Controls in one row share one height. Enabled controls define
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

Horizontal titlebar tabs use a flat, borderless 40-pixel row (the titlebar
height) with content-sized widths, muted inactive text, a hover fill, and an
active surface that joins the workspace below. The bar and inactive tabs use the toolbar
surface; the active tab uses the application background. These are existing
theme roles captured by `Palette`; the tab view adds no literal colors. The
horizontal close action appears on tab hover. Vertical tabs retain their
full-row selected fill, rounded shape, and always-available close action.

Reusable controls expose a real disabled state: preserve their normal geometry,
remove activation and keyboard focus, and use the shared disabled treatment.
The select trigger and text-field surface have disabled variants alongside
buttons, checkboxes, segmented controls, and numeric fields.
Text-field frames provide shared compact (28-pixel) and standard (36-pixel)
sizes with the `md` radius, an `input` border, the `field_bg` fill, no hover
chrome, and a `field_ring` border on focus. Select triggers are 32 pixels.
Menus use 28-pixel rows, the `lg` radius, a hairline, and the popover shadow;
group labels are uppercase 12 px semibold. Their geometry scales with UI font size and density. Standard settings
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

## Motion

`ui-kit/src/animation.rs` is the one motion engine. `Palette::motion`
(`MotionPrefs`) carries the theme durations (`dur_fast` 120 ms, `dur_base`
160 ms, `dur_slow` 200 ms), the `ease_premium` / `ease_soft` curves, and a
`MotionLevel`:

| Level | Behaviour |
|---|---|
| `Full` | fades, slides and size reveals |
| `Reduced` | fades only; offsets and reveals snap to their end state |
| `Off` | every animation collapses to a 10 microsecond floor (instant) |

GPUI divides elapsed time by the animation duration every frame, so no
duration is ever zero: all durations go through `MotionPrefs::resolve`. The
level is chosen in a single function (`animation::motion_level`), which maps
the resolved `ThemeMetrics::animations` (`appearance.animations`) to the level.

GPUI 0.2.2 has no transforms and no hover transitions, so the engine animates
only opacity and layout properties. The `MotionExt` element methods are
`motion_fade_in`, `motion_slide_in_y` (a `top` offset lerp), `motion_reveal`
(width or height lerp, clipped) and `motion_state_fade` (the state is part of
the animation id, so it restarts on every flip). Hover stays instant. Animation
ids must be stable between frames; changing the id restarts the animation.

### Exit state

Popovers, context menus, popover menus, select lists and dialogs can fade out
before they unmount. The owner keeps a `Presence` (open, closing, closed) next
to its open state, calls `request_close(now, &c.motion)` from every dismissal
path, `tick(now, &c.motion)` once per render, and builds the surface while
`is_mounted()` using the `*_with_exit` variant with `closing = is_closing()`.
The exit fade is `dur_fast`; repeated close requests (Esc spam) do not restart
it, re-opening cancels it, `MotionLevel::Off` closes at once, and focus returns
only when `tick` reports `PresenceStep::JustClosed`. A closing surface swallows
pointer input so it cannot fire a second action. The original functions
(`popover`, `context_menu`, `popover_menu`, `select_popover`, `modal_overlay`)
are unchanged and unmount instantly.

## Pane chrome

`pane_bar` and `pane_frame` (`ui-kit/src/pane_bar.rs`) are generic: they know
nothing about tabs or pane kinds. The bar is 28 px (`Density::pane_bar_height`)
with an icon, a title, an optional leading slot (status marker) and trailing
slot (path, cwd), and four optional actions: split horizontal (new pane to the
right), split vertical (new pane below), zoom (toggle, restore glyph while
zoomed) and close, each with a tooltip, an enabled flag and a callback.
`can_split(false)` disables both split actions and swaps their tooltip. The
whole bar is the drag handle: `render()` returns a `Stateful<Div>` and the
caller attaches `on_drag`; action buttons swallow mouse-down so pressing one
never starts a drag. Double-clicking the bar calls the zoom hook. The active
bar uses the sunken fill and a semibold title; inactive bars use hover accent
and show their actions on hover only. A `line_subtle` hairline separates bar
and body.

`PaneLayoutStyle` mirrors `appearance.paneLayout`. Comfortable panes float with
`PANE_GAP` (6 px, density scaled) between them in a rounded (`radius.lg`) frame
with a 1 px `line_subtle` ring (`line_strong` while focused). Compact panes
have no gap, no rounding and no ring; the dividers belong to the tiling
container's resize handles.

## Settings widgets

Settings pages group rows into cards (`settings_group`): a `radius.lg` card on
the `card` fill with `line_subtle` hairlines between rows and an optional
caption above. `settings_row` places a title and description on the left and a
control on the right with an optional reset slot (minimum height 44 px, scaled
by `Palette::control_space`). `slider_row` is a row whose control is the shared
slider plus a fixed-width value read-out. `category_chip` is the 22 px rounded
colour badge before a category; its fill derives from a `ChipHue` and the
light/dark mode and keeps at least 3:1 contrast with its white glyph.

`preview_tile_group` renders radio-style tiles with a mini-window mock drawn
from theme colours (`TileMock`): colour scheme (auto shows half light, half
dark), interface style (classic opaque, glassy translucent), pane layout
(comfortable, compact) and theme (light and dark halves of one theme). The
selected tile has a 2 px `primary` ring and a bold label; the ring is a
constant border so selection never shifts layout. Arrow keys move and select
(Left/Up previous, Right/Down next, wrapping), Enter and Space activate, and
the owner keeps the selected value.

## Floating tabs

`TabStyle::Floating` (default `Classic`) renders tab items as 28 px blocks with
`radius.md`: selected is the sunken fill, a 1 px `line_subtle` ring and a
semibold label; unselected hover is the accent fill; the close button appears
on hover only. Blocks keep a constant 1 px border so selection does not shift
layout. `floating_tab_strip` is the container (40 px bar, 6 px padding, 4 px
gap horizontally; a column with 2 px gaps vertically), `tab_group_gap` replaces
the pinned/unpinned divider with a larger gap, and `tab_drop_marker` is the
absolutely positioned 2 px `primary` insertion bar (no border, no layout
shift).

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
