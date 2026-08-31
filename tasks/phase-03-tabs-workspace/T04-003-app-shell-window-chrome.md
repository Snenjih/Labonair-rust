# T04-003: App-Shell & Fensterchrome

## Status
⏳ Pending

## Phase
3 — App-Shell, Tab-System & Workspace-Layout

## Abhängigkeiten
T04-001 (Tab-Leiste), T04-002 (Split-Pane-Layout)

## Ziel
Das Gesamt-Gerüst der App: das `App.tsx`-Äquivalent als Root-Coordinator, der Header-Bar,
Content-Bereich (Tabs/Workspace), Sidebar-Container und Status-Bar zu einem Fenster komponiert —
mit nativem macOS-Titlebar-Verhalten. Ab hier hat `cargo run` das echte App-Layout.

## Kontext
Referenz-Module:
- `reference-src/src/modules/header/` — obere Leiste, Inline-Suche (`SearchInline`, adaptiv
  Terminal vs. Editor).
- `reference-src/src/modules/statusbar/` — untere Leiste: cwd-Breadcrumb, AI-Tools-Indikator,
  Jump-Host-/Live-Connection-Badges.
- `reference-src/src/App.tsx` (bzw. `AppShell`) — verdrahtet die Module, bleibt Coordinator,
  kein Feature-Host.
- Layout/Spacing/Farben: `reference-src/src/styles/globals.css`.
- Fensteroptionen: `reference-src/src-tauri/` (`tauri.conf.json`, window-state Plugin).

## Anweisungen
1. `crates/app` (oder `crates/ui`) bekommt ein `AppShell`-Root-View (GPUI `Render`), das
   hält: `header`, `sidebar`, `main` (Tabs+Workspace aus T04-001/002), `statusbar`.
2. Header nachbauen: Titel/Breadcrumb, Aktionsbuttons, Inline-Suche. Verhalten von
   `SearchInline`/`SearchTarget` übernehmen (Suche zielt je nach aktivem Tab auf Terminal
   oder Editor).
3. Statusbar nachbauen: cwd-Breadcrumb, Verbindungs-/Jump-Host-Badges, AI-Indikator.
   Datenquellen sind teils noch nicht da (SSH/AI) → als optionale Slots bauen, die leer
   bleiben bis die jeweiligen Phasen liefern.
4. Sidebar-Container: linke Leiste mit umschaltbaren Panels (Explorer/SFTP/Source-Control/
   Git-Graph/AI kommen später) — hier nur der Container + Umschalt-Leiste, Panels als Slots.
5. Fenster/Chrome: native macOS-Titlebar (traffic lights), korrekte Mindestgröße,
   Fenster-Position/-Größe merken (window-state-Äquivalent → Preferences oder eigener
   kleiner State, überschneidet sich mit T14-001).
6. Root-View ist reiner Coordinator: kein Feature-Code direkt darin, nur Komposition + das
   Durchreichen von Shared-State/Callbacks.

## Akzeptanzkriterien
- [ ] `cargo run` zeigt das vollständige App-Layout (Header + Sidebar-Leiste + Tabs/Workspace + Statusbar)
- [ ] Design (Höhen, Paddings, Farben, Trennlinien) 1:1 zu reference-src
- [ ] Native macOS-Titlebar; Fenstergröße/-position überleben Neustart
- [ ] Inline-Suche im Header funktioniert gegen das aktive Terminal
- [ ] Statusbar zeigt cwd korrekt; leere Slots crashen nicht
- [ ] `AppShell` enthält keinen Feature-spezifischen Code (nur Komposition)
- [ ] `cargo check` + `clippy -- -D warnings` + `cargo test` grün

## Notizen
- Sidebar-Panel-Registry so bauen, dass spätere Phasen nur ein Panel registrieren müssen.
- Überlappung mit T14-001 (Session-Persistenz) bewusst — Fenster-State hier minimal halten,
  T14-001 baut darauf auf.

## Warnungen
- ⚠️ GPUI-Fenster-/Titlebar-API (`WindowOptions`, `titlebar`, traffic-light-Position) in
  gpui-Source verifizieren — Zed macht das, Signatur dort abschauen.
