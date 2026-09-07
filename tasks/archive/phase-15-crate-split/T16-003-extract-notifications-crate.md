# T16-003: `labonair-notifications` extrahieren

## Status
✅ Done

## Phase
15 — Crate-Zerlegung & Fundament

## Abhängigkeiten
T16-002 (`labonair-gpui-ext` + `labonair-ui-kit`)

## Ziel
Das Notifications-/Toast-System aus dem `ui`-Monolithen in einen eigenen Crate
`labonair-notifications` lösen, damit sowohl `labonair-shell` als auch die
künftigen Panel-Crates Toasts auslösen können, ohne von `crates/ui` abzuhängen.
Reiner Move, keine Verhaltensänderung.

## Kontext
- Heute: `crates/ui/src/notifications.rs` — `NotificationCenter` (GPUI-Entity),
  `Notification` (`info`/`error`/…), `render_overlay(...)`, `init_notifications`,
  `notification_center()` (globaler Zugriff). Verwendet in `app_shell.rs`
  (`render_overlay`, Demo-Toast in `AppShell::new`), `settings.rs`
  (`Notification`), evtl. weiteren Views.
- Aufrufmuster prüfen: `grep -rn 'notifications::\|NotificationCenter\|Notification::' crates/ui/src`.
- Zed-Vorbild: `zed-refrence/zed/crates/notifications/` und
  `zed-refrence/zed/crates/workspace/src/{notifications.rs,toast_layer.rs}` —
  Toast-Layer ist bei Zed Teil des Workspace, das eigentliche
  Notification-Model ein eigener Crate. Wir folgen der Crate-Trennung.

## Anweisungen zur Umsetzung
1. **`crates/notifications/` anlegen** (`labonair-notifications`,
   `src/notifications.rs` Lib-Root, `[lib] path` explizit).
2. `crates/ui/src/notifications.rs` per `git mv` hierher; Modulaufteilung
   optional (`model.rs` + `overlay.rs`) nur wenn die Datei > ~300 Z. ist.
3. Öffentliche API unverändert lassen:
   `NotificationCenter`, `Notification`, `render_overlay`,
   `init_notifications`, `notification_center` (bzw. deren aktuelle Namen).
4. Dependencies: `gpui`, `labonair-ui-kit` (für Icon/Button/Toast-Styling),
   `labonair-theme`, `labonair-gpui-ext`. **Keine** Abhängigkeit zurück auf
   `crates/ui`.
5. Workspace-`Cargo.toml`: Member + `[workspace.dependencies]`-Eintrag.
6. `crates/ui`: `mod notifications;` entfernen, `labonair-notifications` als
   Dep; alle `crate::notifications::` → `labonair_notifications::`.
   `crates/app/src/main.rs` ruft `init_notifications` — Import dort anpassen.
7. `cargo run`: Der Debug-Startup-Toast erscheint wie bisher, Fehler-Toasts
   (z.B. Settings-Fehler) unverändert.

## Akzeptanzkriterien
- [ ] `crates/notifications/` ist eigener Workspace-Member; `crates/ui` hat
      keine `notifications.rs` mehr.
- [ ] Öffentliche Symbole identisch benannt; keine Aufrufstelle inhaltlich
      geändert (nur Import-Pfad).
- [ ] `labonair-notifications` hängt nicht (direkt oder transitiv) von
      `labonair-ui` ab (`cargo tree -p labonair-notifications` prüfen).
- [ ] `cargo run`: Startup-Toast + mindestens ein ausgelöster Fehler-Toast
      verhalten sich unverändert (Position, Auto-Dismiss, Styling).
- [ ] Bestehende Notification-Tests laufen im neuen Crate.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Der *Toast-Layer* (wo die Toasts im Fenster gerendert werden) zieht später in
  T17-005 in `labonair-workspace` — hier bleibt `render_overlay` noch so, wie
  es `AppShell` heute aufruft.

## Warnungen
- ⚠️ `notification_center()` ist vermutlich ein globaler Zugriff
  (`cx.global`/`try_global`) — sicherstellen, dass die Global-Registrierung
  (`init_notifications`) weiter genau einmal in `main.rs` passiert.

## Weiterführende Tasks
- [T16-004: `labonair-command-palette` extrahieren](./T16-004-extract-command-palette-crate.md)
