# T18-007: Philosophie verankern + Personalisierungs-Settings-Seite

## Status
📋 Geplant

## Phase
17 — Neues Layout & Statusbar-Personalisierung

## Abhängigkeiten
T18-005 (Statusbar-Item-Personalisierung), T18-003 (Statusbar links — Panel-Steuerung)

## Ziel
Zwei Dinge abschließen: (1) die neue Philosophie fest in `ROADMAP.md` +
`CLAUDE.md` verankern, damit alle künftigen Tasks daran ausgerichtet sind;
(2) eine Settings-Seite „Personalisierung", die die per Rechtsklick verstreuten
Optionen bündelt — Statusbar-Layout (welches Item links/rechts/aus) und
Panel-Sichtbarkeit — als überblickbares GUI.

## Kontext
- Philosophie-Text: `bericht-architektur-rework-roadmap.md` §1 + T16-001
  (`docs/architecture.md`).
- `tasks/ROADMAP.md` — Abschnitt „## Vision" + „## Erfolgskriterien".
- `CLAUDE.md` (Repo-Root) — „# Labonair-rust — CLAUDE.md" Kopf,
  „## Critical Rules".
- Settings-UI: `labonair-settings-ui` (nach T16-007). Die generische
  Feld-Rendering-Infrastruktur wird erst in T19-004 Zed-Style; hier reicht
  eine hand-gebaute Pane (wie die heutigen Sonder-Panes Theme/Shortcuts/MCP).
- Daten: `statusBarItemPlacements` (T18-005), Panel-Sichtbarkeits-Blob (neu —
  `panelToggleVisibility` o.ä., in `labonair-settings.json`).

## Anweisungen zur Umsetzung
### Teil A — Philosophie verankern
1. **`tasks/ROADMAP.md`**: „## Vision" um einen Absatz ergänzen — die vier
   Prinzipien aus Bericht §1 (kurz), plus den Satz „Feature-Parität ist das
   Minimum, nicht das Ziel". Den Rework als Phasen 15–21 in der Phasen-
   Übersicht listen (falls T16-001 das nicht schon vollständig getan hat —
   dann nur verifizieren).
2. **`CLAUDE.md`**: unter dem Titel-Block einen kurzen Abschnitt
   „## Philosophie (ab Architektur-Rework)" mit dem Leitsatz + Verweis auf
   `docs/architecture.md`. Die „Critical Rules" **nicht** ändern, nur eine
   Regel ergänzen: „8. **Layout-Vertrag einhalten** — Titlebar nur Tabs + der
   eine Menü-Button; Statusbar = Panel-Steuerung links / Info-Dropdowns
   rechts; Overlays nur über `ModalLayer`/`ToastLayer`. Abweichungen zuerst in
   `docs/architecture.md` begründen."
3. **`handshake.md`**: Eintrag, dass die Philosophie jetzt normativ ist.

### Teil B — Personalisierungs-Seite
4. **Neue Settings-Kategorie „Personalisierung"** in `labonair-settings-ui`
   (`CATEGORIES` erweitern; nach „Appearance & Layout"). Deep-Link-Slug
   `personalization`.
5. **Statusbar-Layout-Editor** in der Pane:
   - Zwei Spalten „Links" / „Rechts", darunter „Ausgeblendet".
   - Jedes verschiebbare `StatusItem` als Chip mit seinem Icon+Titel in der
     Spalte seiner aktuellen `resolve_side`; ausgeblendete unten.
   - Pro Chip: Buttons „← / → / ausblenden / einblenden" (schreiben dasselbe
     `statusBarItemPlacements`-Blob wie das RMB-Menü, gemeinsame Funktion).
   - Optional (nice-to-have, kein Muss): Drag zwischen den Spalten.
   - „Auf Standard zurücksetzen" → Blob leeren.
6. **Panel-Sichtbarkeit** in derselben Pane:
   - Liste aller registrierten Panels mit Switch „im Statusbar-Toggle
     anzeigen". Aus → das Panel taucht nicht in `PanelTogglesStatusItem`
     (T18-003) auf, ist aber weiter per Command-Palette öffenbar.
   - Persistenz-Blob `panelToggleVisibility: { name: bool }`;
     `PanelTogglesStatusItem` liest es.
7. **Live**: Änderungen wirken sofort (die Statusbar re-liest über den
   Tick/Watch aus T18-005), ohne Neustart.
8. `cargo run`: Settings → Personalisierung → ein Info-Item nach links
   schieben → Statusbar aktualisiert sofort; ein Panel-Toggle ausblenden →
   verschwindet aus der Leiste, bleibt in der Palette; „Zurücksetzen" stellt
   Defaults her; alles überlebt Neustart.

## Akzeptanzkriterien
- [ ] `ROADMAP.md` + `CLAUDE.md` enthalten die Philosophie + den Layout-Vertrag
      als normative Aussage; neue Critical Rule 8 ergänzt.
- [ ] Settings-Kategorie „Personalisierung" existiert, per Deep-Link
      (`personalization`) erreichbar.
- [ ] Statusbar-Layout-Editor: Items nach links/rechts/aus, „Zurücksetzen",
      schreibt dasselbe Blob wie das RMB-Menü (gemeinsame Funktion, kein
      Zweit-Pfad).
- [ ] Panel-Sichtbarkeit: Switch je Panel; ausgeschaltet ⇒ kein Toggle, aber
      weiterhin per Command-Palette öffenbar; persistiert.
- [ ] Änderungen wirken live (kein Neustart) und überleben Neustart.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Die Pane ist bewusst hand-gebaut (wie Theme-/Shortcuts-Pane). In T19-004
  wird der generische Teil des Settings-UI Zed-Style — die Personalisierungs-
  Pane bleibt eine Custom-Pane (`SettingsPageItem::Custom`).
- „Zurücksetzen" nur für den Personalisierungs-Bereich, nicht global.

## Warnungen
- ⚠️ Genau **eine** Funktion schreibt `statusBarItemPlacements` — das RMB-Menü
  (T18-005) und diese Pane rufen dieselbe. Kein duplizierter Schreibpfad.
- ⚠️ Panel-Toggle-Sichtbarkeit ≠ Panel-Position. Ausblenden im Toggle
  schließt das Panel nicht und ändert seinen Dock nicht.

## Weiterführende Tasks
- [T19-001: `labonair-settings-content` — typisierter Settings-Baum](../phase-18-settings-core/T19-001-settings-content-tree.md)
- [T19-004: Settings-UI aus Modell generieren](../phase-18-settings-core/T19-004-generated-settings-ui.md)
