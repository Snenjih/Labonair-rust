# T16-002: `labonair-gpui-ext` + `labonair-ui-kit` (Skeleton)

## Status
✅ Done

## Phase
15 — Crate-Zerlegung & Fundament

## Abhängigkeiten
T16-001 (ADR & Ziel-Crate-Graph)

## Ziel
Die zwei Fundament-Crates anlegen, auf denen alle weiteren aufbauen:
`labonair-gpui-ext` (geteilte Prelude + GPUI-Helfer) und `labonair-ui-kit`
(Design-System). In dieser Task ziehen die bestehenden Primitives aus
`crates/ui/src/components/` in `labonair-ui-kit` um und alle Aufrufstellen
werden umgebogen. **Null Verhaltensänderung** — reiner Move + Re-Export.

## Kontext
- Heute: `crates/ui/src/components/{mod.rs,button.rs,context_menu.rs,icon.rs,text_field.rs}`
  (~880 Z.). `mod.rs` re-exportiert zusätzlich
  `gpui_component::{badge::Badge, switch::Switch, tooltip::Tooltip}`.
- Aufrufstellen: `grep -rn 'crate::components' crates/ui/src` — überall
  (`hosts.rs`, `ai_chat.rs`, `explorer.rs`, `settings.rs`, `app_shell.rs`, …).
- Zed-Vorbild: `zed-refrence/zed/crates/ui/` (Design-System-Crate mit
  `prelude.rs`, `styles/`, `traits/`, `components/`), `zed-refrence/zed/crates/ui/src/prelude.rs`.
- Wiederkehrende GPUI-Importblöcke im Port: `use gpui::{div, px, App, ...}` +
  `use gpui::prelude::FluentBuilder` in fast jeder `crates/ui/src/*.rs`.

## Anweisungen zur Umsetzung
1. **`crates/gpui-ext/` anlegen** (`labonair-gpui-ext`):
   - `src/gpui_ext.rs` als Lib-Root (`[lib] path` in `Cargo.toml` explizit).
   - `pub mod prelude` mit den im Port wiederkehrenden Re-Exports:
     `pub use gpui::{prelude::*, div, px, rems, App, AppContext, Context,
     Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
     SharedString, Styled, Window, ...}` — die Menge aus einem `grep` über die
     tatsächlichen `use gpui::{…}`-Zeilen ableiten, nicht raten.
   - Kleine Helfer-Traits, die heute mehrfach ad-hoc existieren (z.B.
     `.when_hovered(...)`-artige Kürzel, falls vorhanden) — nur was real
     doppelt ist, nichts Spekulatives.
   - Dependencies: `gpui`, `gpui-component` (nur wenn ein Helfer es braucht).
2. **`crates/ui-kit/` anlegen** (`labonair-ui-kit`):
   - `src/ui_kit.rs` Lib-Root.
   - `crates/ui/src/components/{button,context_menu,icon,text_field}.rs` **1:1**
     hierher verschieben (`git mv`), Modulpfade anpassen.
   - `pub use` derselben Symbole wie heute `components/mod.rs`
     (`button`, `ButtonSize`, `ButtonVariant`, `DISABLED_OPACITY`,
     `context_menu`, `MenuClick`, `MenuItem`, `file_icon`, `folder_icon`,
     `IconName`, `field_input`, `text_field`, `InputEvent`, `InputState`) +
     die drei `gpui-component`-Re-Exports (`Badge`, `Switch`, `Tooltip`).
   - Theme-Zugriff: die Primitives lesen heute `crate::theme::ThemeStore` /
     `active_theme`. In `ui-kit` auf `labonair_theme`/`labonair_ui`-Theme
     umstellen — falls `ThemeStore` noch in `crates/ui` liegt, vorerst
     `labonair-ui-kit` → `labonair-theme` zeigen lassen und den `ThemeStore`
     mitnehmen **falls** er reiner Token-Zugriff ist; sonst dünnen
     `Theme`-Trait in `ui-kit` definieren, den `crates/ui` implementiert.
     Entscheidung in `docs/architecture.md` (T16-001 §3) nachziehen.
   - Dependencies: `gpui`, `gpui-component`, `labonair-theme`,
     `labonair-gpui-ext`.
3. **Workspace-`Cargo.toml`**: beide neuen Crates zu `members` hinzufügen,
   Versionen/Deps in `[workspace.dependencies]` pflegen.
4. **`crates/ui` umstellen**: `mod components;` entfernen, stattdessen
   `pub use labonair_ui_kit as components;` **oder** alle `crate::components::`
   → `labonair_ui_kit::` ersetzen (Zweiteres bevorzugt, sauberer; per
   `grep` + gezieltem Ersetzen). `crates/ui/Cargo.toml` bekommt
   `labonair-ui-kit` + `labonair-gpui-ext` als Deps.
5. **Alle Aufrufstellen** in `crates/ui/src/*.rs` und (falls vorhanden) in
   `crates/app` anpassen.
6. **Verifizieren**: `cargo run` zeigt exakt dieselbe UI wie vorher (Buttons,
   Kontextmenüs, Icons, Textfelder unverändert).

## Akzeptanzkriterien
- [ ] `crates/gpui-ext/` und `crates/ui-kit/` existieren, sind
      Workspace-Members, mit explizitem `[lib] path`.
- [ ] `labonair-ui-kit` re-exportiert exakt die heutige Symbol-Menge aus
      `components/mod.rs` (kein Symbol verloren, keins hinzugefügt).
- [ ] `crates/ui/src/components/` ist entfernt; alle Aufrufstellen zeigen auf
      `labonair_ui_kit::…`.
- [ ] `cargo run` — UI visuell identisch zu vor der Task (manueller
      Screenshot-Vergleich Buttons/Menüs/Icons/Inputs).
- [ ] Keine neue `clippy`-Warnung; bestehende Tests der Primitives laufen im
      neuen Crate.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Diese Task ist bewusst „langweilig": kein Feature, kein Redesign. Der
  Primitive-**Ausbau** kommt erst in T20-001.
- Wenn der `ThemeStore` schwer zu lösen ist, ist ein `ui-kit::Theme`-Trait der
  Standardweg (Zed macht es via `ui`-Crate + `theme`-Crate genauso getrennt).

## Warnungen
- ⚠️ `git mv` statt Copy+Delete, damit die History erhalten bleibt.
- ⚠️ Reihenfolge im Workspace: `gpui-ext` vor `ui-kit` vor `ui` — sonst
  Auflösungsfehler beim ersten `cargo check`.
- ⚠️ Kein „während wir dabei sind"-Refactor an den Primitives selbst.

## Weiterführende Tasks
- [T16-003: `labonair-notifications` extrahieren](./T16-003-extract-notifications-crate.md)
- [T16-004: `labonair-command-palette` extrahieren](./T16-004-extract-command-palette-crate.md)
