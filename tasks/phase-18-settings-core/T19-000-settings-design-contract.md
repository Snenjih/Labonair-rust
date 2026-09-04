# T19-000: Settings-Design-Kontrakt festschreiben

## Status
✅ Done

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T16-007 (`labonair-settings-ui` extrahiert)

## Ziel
Bevor der Zed-Style-Umbau der Settings beginnt (T19-001 ff.), einen **schriftlichen,
verbindlichen Design-Kontrakt** für alle Settings-Seiten festlegen — damit die
Settings nicht wieder driften wie in der Tauri-Version (verstreut, uneinheitliches
UI je Kategorie, Optionen nur im UI ohne Modell). Der Kontrakt ist reine
Dokumentation + eine `CLAUDE.md`-Regel; **kein Code**. Er ist die Zielvorgabe,
gegen die T19-001–T19-010 implementieren.

## Kontext
- `docs/architecture.md §8.3` fasst den Kontrakt bereits an — diese Task macht
  daraus das eigenständige, ausführliche Dokument und verankert es normativ.
- Heutiger Ist-Zustand (das Problem): `crates/ui/src/settings.rs` (212 KB) —
  flache `CATEGORIES` (10), handgepflegte `FIELDS`-Tabelle (131 Einträge,
  driftet gegen ~170 `Preferences`-Felder), `SECTION_GROUPS`; Sonder-Panes
  (Theme-Grid, Shortcuts-Capture, AI-Provider, MCP) mit je eigenem UI-Stil.
- Zed-Vorbild: `zed-refrence/zed/crates/settings_ui/src/settings_ui.rs` +
  `page_data.rs` (deklarative Seiten aus `SettingField` + Renderer-Registry).
- Referenz-Verhalten (Navigation/Copy): `reference-src/src/modules/settings/`
  (`definitions.ts` — Kategorie/Sektion-Struktur, Beschreibungstexte).

## Anweisungen zur Umsetzung
1. **`docs/settings-guidelines.md` anlegen** — der Kontrakt, mit diesen
   verbindlichen Punkten:
   1. **Ein Navigations-Modell.** Links: Top-Level-Kategorien (feste
      Reihenfolge). Rechts: die gewählte Kategorie als Seite mit
      **aufklappbaren Abschnitts-Überschriften** (`SectionHeader`,
      Disclosure) + Scroll-Spy-Sprungmarken. Große Kategorien dürfen
      **Unter-Seiten** haben (`SubPageLink`, mit Zurück-Pfeil). Keine
      Kategorie weicht von diesem Muster ab.
   2. **Jede Einstellung ist ein typisiertes Feld** im `SettingsContent`-Baum
      (T19-001) mit Metadaten (`title`, `description`, `unit?`, `range?`,
      `placeholder?`, `requires_restart?`). **Keine Einstellung existiert nur
      im UI.** Kein paralleles `FIELDS`-Array.
   3. **Die Feld-UI wird aus dem Rust-Typ generiert** (T19-004): `bool` →
      Switch, `enum` → Dropdown (menschenlesbare Labels), Zahl → NumberField
      (mit `range`/`step` aus Metadata), `String` → TextInput; Spezialtypen
      über Metadata-Marker. **Keine handgebauten Toggles/Zeilen** für simple
      Feldtypen.
   4. **Custom-Panes** sind ein **sanktionierter erstklassiger Pfad**, kein
      Hack: `SettingsPage { kind: Custom(render_fn) }`, registrierbar auch als
      **Top-Level-Kategorie** (wie „Themes", künftig „Hosts", „Shortcuts",
      „AI", „MCP"). Sie rendern **im Standard-Seiten-Chrome** (gleicher Kopf,
      gleiche Suche, gleiche Herkunfts-Badges) — nur der Inhaltsbereich ist
      custom. Ein neuer Custom-Top-Level = eine Registrierungszeile, kein
      Eingriff in die Feld-Registry.
   5. **Herkunft + Reset.** Jedes gerenderte Feld zeigt dezent seine wirksame
      Schicht (Standard / Benutzer / Projekt, `SettingsStore::source_of`) und
      bietet bei ≠ Standard „auf Standard zurücksetzen".
   6. **Suche global.** Jede Seite (auch Custom-Panes) speist durchsuchbare
      Stichwörter; die globale Suche (T19-007) findet jedes Feld.
   7. **Deep-Links.** Jede Kategorie **und** jeder Abschnitt hat einen
      stabilen Slug; `settings://<kategorie>/<abschnitt>` springt dorthin
      (Command-Palette, „Herkunft anzeigen", `menu.rs`).
   8. **Copy-Regeln.** Labels im Satzstil, kurz. Beschreibungen imperativ,
      ein Satz. Einheiten ins Feld/Metadata, nicht ins Label. Keine
      Redundanz Label ↔ Beschreibung. Englisch (Language Protocol).
   9. **Kein Fenster-Wildwuchs.** Ein Settings-Fenster, ein Store, ein
      Navigations-Baum. Neue Bereiche hängen sich als Kategorie/Abschnitt
      ein, nicht als eigenes Fenster/Overlay.
2. **`CLAUDE.md` (Repo-Root)** — unter „## Critical Rules" **Regel 9**
   ergänzen (Regel 8 = Layout-Vertrag aus T18-007):
   > 9. **Settings-Design-Kontrakt einhalten** — jede Einstellung ist ein
   > typisiertes `SettingsContent`-Feld mit generierter UI; ein
   > Navigations-Modell (Kategorie → Abschnitt → optionale Unter-Seite);
   > Custom-Panes nur für echte Nicht-Feld-UIs und immer im Standard-Chrome.
   > Details + Abweichungs-Prozess: `docs/settings-guidelines.md`.
3. **`docs/architecture.md §8.3`** — auf `docs/settings-guidelines.md`
   verweisen (kurz, nicht duplizieren).
4. **`tasks/ROADMAP.md`** — in der Phase-18-Tabelle T19-000 als erste Zeile
   führen; im Rework-Erfolgskriterium 25 einen Halbsatz „…gemäß
   `docs/settings-guidelines.md`" ergänzen.
5. **`handshake.md`** — Eintrag, dass der Kontrakt ab jetzt normativ ist.

## Akzeptanzkriterien
- [x] `docs/settings-guidelines.md` existiert mit den 9 Punkten, jeweils
      konkret genug, um in einem Review „verstößt gegen Punkt X" sagen zu
      können.
- [x] `CLAUDE.md` hat Critical Rule 9 (Settings-Kontrakt) mit Verweis.
- [x] `docs/architecture.md §8.3` verweist auf das Dokument (bereits vorab so
      geschrieben; verifiziert, keine Änderung nötig).
- [x] `ROADMAP.md` listet T19-000 als erste Phase-18-Task (bereits vorab so
      geschrieben; verifiziert, keine Änderung nötig). Erfolgskriterium 25
      enthält bereits den Halbsatz „…gemäß `docs/settings-guidelines.md`".
- [x] Reines Doku-Change: `git diff --stat` (für die committeten Dateien)
      zeigt nur `.md`-Dateien.
- [x] Gates unverändert grün (keine Code-Änderung): `cargo fmt --check`,
      `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Dies ist eine **Doku-Task** und braucht keinen Nutzer-Sicht-Test der App.
  Der Wert liegt darin, dass T19-001–T19-010 (und alle künftigen
  Settings-PRs) ein verbindliches Ziel haben.
- Bewusst **vor** T19-001 (deshalb Nummer `T19-000`): der `SettingsContent`-
  Baum soll schon mit dem Kontrakt im Kopf geschnitten werden.

## Warnungen
- ⚠️ Nicht überspezifizieren: der Kontrakt legt **Regeln** fest, nicht die
  Pixel. Konkrete Widgets/Abstände bleiben Sache von T19-004 + Nutzer-
  Sichtprüfung.
- ⚠️ „Custom-Top-Level-Kategorie" ist die einzige erlaubte Sonderform — der
  Kontrakt muss klar sagen, dass das **kein** Freibrief für eigenes
  Seiten-Chrome ist.

## Weiterführende Tasks
- [T19-001: `labonair-settings-content` — typisierter Baum](./T19-001-settings-content-tree.md)
- [T19-004: Settings-UI aus dem Modell generieren](./T19-004-generated-settings-ui.md)
- [T19-010: Settings › Hosts](./T19-010-hosts-settings-category.md)
