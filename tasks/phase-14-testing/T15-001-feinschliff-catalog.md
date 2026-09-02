# T15-001 — Feinschliff-Katalog (Visual Parity)

Living checklist of design deviations found while comparing the Rust/GPUI port
against the frozen `reference-src/` design spec. Cross-referenced from
`T15-001-visual-parity.md`. Iterative — carried forward into T15-003.

## Method

- Static audit of every `crates/ui/src/*.rs` view against the reference
  Tailwind/shadcn components and `reference-src/src/styles/globals.css`.
- Token authority: `crates/theme/src/tokens.rs` (already a verified 1:1 port of
  the `:root` / `.dark` blocks — spot-checked by `tokens.rs` tests against the
  design-intent hex comments).
- Regression guards added as `cargo test` assertions where a value can be
  checked logically (contrast, fixed shared constants).

## Colors

| # | Area | Deviation | Reference | Fix | Status |
|---|------|-----------|-----------|-----|--------|
| C1 | Modal / dialog / picker backdrops | 6 different hardcoded scrim fills across 8 views (`black/0.4`, `rgba(0x00000099)` ×5, `rgba(0x000000aa)`, `rgba(0x00000080)`, `hsla(…0.5)`) | `dialog.tsx` / `alert-dialog.tsx` / `sheet.tsx` all use a single `bg-black/30` | New `crate::theme::modal_scrim()` → `black @ 0.30`; all 9 overlay sites (command palette, snippets, settings, explorer, transfers, sftp, hosts ×4, workspace prompt + close-confirm) now call it | ✅ |
| C2 | Core / sidebar / surface / border / status / interaction / ANSI tokens | none — transcribed 1:1 from `globals.css`, verified by `theme` crate tests | — | — | ✅ verified |
| C3 | `foreground`/`background` + `terminal` + `muted-foreground` legibility | risk of a future token edit silently breaking contrast | WCAG AA (4.5:1 body, 3:1 muted) | Added `body_text_meets_wcag_aa_contrast` regression test (light + dark) | ✅ |
| C4 | Editor syntax palettes (`syntax_theme.rs`) | 10 named editor themes with literal hex — these are editor color schemes (One Dark, Tokyo Night, GitHub, …), not app-chrome tokens; parity is with the schemes themselves, not `globals.css` | matching upstream schemes | left as-is (correct) | ✅ n/a |
| C5 | `git_graph.rs` lane / avatar palettes, opacity-derived tints | derived from theme tokens via `.opacity(...)`; lane colors are a fixed categorical ramp (same intent as reference graph) | reference git-graph | acceptable — no `globals.css` token for graph lanes | ✅ n/a |

## Radii / Spacing / Shadows / Animation

| # | Area | Finding | Status |
|---|------|---------|--------|
| R1 | Radius scale | `RadiusScale::from_base(5.0)` with the exact `@theme inline` `calc()` multipliers (`0.6 / 0.8 / 1 / 1.4 / 1.8 / 2.2 / 2.6`) + fixed `window: 12px`. Tested. | ✅ verified |
| R2 | Elevation shadows | `row` / `popover` / `modal` layer counts + per-layer x/y/blur/spread/alpha transcribed 1:1 for both variants. Tested. | ✅ verified |
| R3 | Animation timing | `dur-fast/base/slow` = 160/240/320 ms, `ease-premium` / `ease-soft` cubic-beziers 1:1. Tested. | ✅ verified |

## Typography

| # | Finding | Status |
|---|---------|--------|
| T1 | UI font Inter Variable @ 13px / 1.5, buffer JetBrains Mono @ 13px, terminal mono @ 14px / 1.05, ligatures on — matches `--app-font-*` + `preferencesStore` defaults. Bundled assets registered at startup (`init_fonts`). | ✅ verified |
| T2 | Runtime overrides (settings font family/size for app/editor/terminal) wired via `FontOverrides` in T13-003. | ✅ done earlier |

## Deferred to live side-by-side (T15-003)

These need the two apps running next to each other and cannot be resolved by
static audit; tracked here so they are not lost:

- D1 — Hover / focus / active tint *amounts* on buttons, tabs, list rows
  (currently `fg.opacity(0.04–0.05)` hovers; reference uses `accent` / `muted`
  fills). Needs pixel comparison per component.
- D2 — Scrollbar thumb thickness / inset / hover color vs the reference
  `.themed-scrollbar` (10px track, `color-mix(foreground 22% → 34%)`).
- D3 — Terminal cell width/height rounding and cursor block/beam/underline
  proportions vs xterm.
- D4 — Tab-entrance animation curve/scale (`labonair-tab-in`: scale 0.86→1 over
  `--dur-base` `--ease-premium`).
- D5 — Popover/menu padding density (context menus, command palette rows).
- D6 — User-imported theme round-trip visual check (import a community theme,
  compare against the web app with the same file).

## Changes made in this task

- `crates/theme/src/tokens.rs` — `body_text_meets_wcag_aa_contrast` test
  (WCAG relative-luminance helpers + light/dark assertions).
- `crates/ui/src/theme.rs` — `modal_scrim()` shared accessor + test.
- `crates/ui/src/{command_palette,snippets,settings,explorer,transfers,sftp,hosts,workspace}.rs`
  — every modal/overlay backdrop now uses `modal_scrim()` (was 6 divergent
  hardcoded values).
