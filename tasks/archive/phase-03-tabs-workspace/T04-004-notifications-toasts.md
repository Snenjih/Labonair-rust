# T04-004: Notifications / Toast-System

## Status
✅ Done

## Phase
3 — App-Shell, Tab-System & Workspace-Layout

## Abhängigkeiten
T04-003 (App-Shell)

## Ziel
Ein app-weites Notification-/Toast-System als Ersatz für `reference-src/src/modules/notifications/`.
Andere Module (SSH-Fehler, Transfer-Ergebnisse, Update-Hinweise, MCP-Aktivität …) melden darüber
Erfolg/Info/Warnung/Fehler. Viele spätere Tasks setzen dieses System voraus.

## Kontext
Referenz: `reference-src/src/modules/notifications/` (Toast-Komponente, Store, Auto-Dismiss,
Severity-Stufen, evtl. Aktions-Buttons). Design (Farben pro Severity, Radius, Schatten,
Position, Animation) aus `reference-src/src/styles/globals.css` + der Komponente selbst.

## Anweisungen
1. `crates/ui` bekommt ein `notifications`-Modul: ein GPUI-Entity/Model `NotificationCenter`
   mit `push(Notification { severity, title, body, actions, timeout })`, `dismiss(id)`.
2. Severity-Stufen 1:1: `info | success | warning | error` (Namen aus dem Original prüfen).
3. Toast-Overlay im `AppShell` rendern (oben/rechts oder wie im Original), gestapelt,
   Auto-Dismiss nach Timeout, manuelles Schließen, optionale Aktions-Buttons.
4. Ein globaler, leicht erreichbarer Zugang (z.B. `cx.global::<NotificationCenter>()` oder ein
   `Handle`, das durch den Shell-Coordinator gereicht wird) — API-Ansatz in gpui verifizieren.
5. Fehler-Konvention: Backend-Funktionen geben `Result<_, String>` zurück (Critical Rule 6);
   der aufrufende UI-Code entscheidet, ob daraus ein Error-Toast wird. Helfer bereitstellen
   (`notify_err(result)`).
6. Animation (Ein-/Ausblenden, Slide) an das Original angleichen.

## Akzeptanzkriterien
- [ ] `NotificationCenter.push(...)` zeigt einen Toast im korrekten Design
- [ ] Alle 4 Severity-Stufen mit korrekten Farben/Icons
- [ ] Auto-Dismiss + manuelles Schließen + Stapelung funktionieren
- [ ] Aktions-Button in einem Toast löst Callback aus
- [ ] Von einem beliebigen Modul aus erreichbar (Demo: Fake-Fehler beim Start zeigt Toast)
- [ ] `cargo check` + `clippy -- -D warnings` + `cargo test` (Store-Logik unit-getestet) grün

## Notizen
- Timeout pro Severity unterschiedlich (Error bleibt länger/manuell) — Original-Werte übernehmen.
- Nicht überdesignen: eine Queue, ein Overlay, klare API.

## Warnungen
- ⚠️ Toasts dürfen Klicks auf die dahinterliegende UI nicht blockieren (pointer-events nur auf
  dem Toast selbst).
