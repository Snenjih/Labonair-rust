# Labonair Design System

**Status:** Normative

## Visual objective

Labonair should feel dense, calm, precise, and fast. The visual system uses a restrained hierarchy rather than decorative chrome. Zed is a behavioral reference, while Labonair's own tokens and product needs remain authoritative.

## Token ownership

All colors, spacing, radii, typography, shadows, focus states, and semantic status colors come from the theme/design-token layer. Feature crates consume tokens; they do not invent global values.

Reference values from `reference-src` may be used as input during migration, but new values must be recorded in the token source rather than copied into views.

The token source is the theme/design-token implementation and its focused
tests. A view may not introduce a global color, spacing, radius, typography,
shadow, or state value inline. If a new visual value is genuinely required,
extend the token source first and record the reason in the owning task.

## UI-kit requirement

The following must come from `labonair-ui-kit`:

- buttons and icon buttons;
- text fields and search fields;
- lists and list rows;
- dropdowns, menus, and context menus;
- popovers and dialogs;
- tabs and tab controls;
- badges, indicators, disclosure rows, tooltips, and keyboard hints;
- standard dividers, scroll containers, and empty/loading states.

Feature modules may compose these primitives and provide domain content. They may not create local variants with different padding, colors, hover behavior, or typography without extending the shared component.

## Interaction states

Every interactive component must define normal, hover, pressed, selected, focused, disabled, and error states where applicable. Keyboard focus must remain visually distinguishable.

## Layout rules

- Use a consistent spacing scale.
- Keep permanent chrome minimal.
- Align popovers to their triggering element in the same window coordinate space.
- Use progressive disclosure for secondary actions.
- Lists must define selection, focus, empty, loading, and error behavior.
- Long lists must use bounded or virtualized rendering where measurement requires it.

## Review rule

A UI change is incomplete until it has been checked at normal, narrow, focused, empty, loading, and error states. Screenshots or a visual comparison are required for shell and shared-component changes.

Exceptions to the UI-kit rule require all of: a documented reason, a
composition of existing tokens, an owner, and a follow-up decision about
whether the pattern belongs in `labonair-ui-kit`. One-off local styling is not
an exception.
