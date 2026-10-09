# 1. Verwendete Referenzquellen

Die zentrale Zed-Referenz ist:

- [Zed Settings Window](/Users/niklas/Developer/active/Labonair/Labonair-rust/zed-refrence/zed/crates/settings_ui/src/settings_ui.rs)
- [Zed Settings Components](/Users/niklas/Developer/active/Labonair/Labonair-rust/zed-refrence/zed/crates/settings_ui/src/components)
- [Zed TreeViewItem](/Users/niklas/Developer/active/Labonair/Labonair-rust/zed-refrence/zed/crates/ui/src/components/tree_view_item.rs)
- [Zed DropdownMenu](/Users/niklas/Developer/active/Labonair/Labonair-rust/zed-refrence/zed/crates/ui/src/components/dropdown_menu.rs)
- [Zed Settings Input Field](/Users/niklas/Developer/active/Labonair/Labonair-rust/zed-refrence/zed/crates/settings_ui/src/components/input_field.rs)
- [Zed Settings Number Field](/Users/niklas/Developer/active/Labonair/Labonair-rust/zed-refrence/zed/crates/settings_ui/src/components/number_field.rs)

Labonair:

- [SettingsView](/Users/niklas/Developer/active/Labonair/Labonair-rust/crates/settings-ui/src/view.rs)
- [Generic Settings Renderer](/Users/niklas/Developer/active/Labonair/Labonair-rust/crates/settings-ui/src/panes/generic.rs)
- [Settings Pages](/Users/niklas/Developer/active/Labonair/Labonair-rust/crates/settings-ui/src/pages.rs)
- [Settings Schema](/Users/niklas/Developer/active/Labonair/Labonair-rust/crates/settings-ui/src/schema.rs)
- [Settings Window](/Users/niklas/Developer/active/Labonair/Labonair-rust/crates/settings-ui/src/window.rs)
- [UI-Kit Palette](/Users/niklas/Developer/active/Labonair/Labonair-rust/crates/ui-kit/src/palette.rs)
- [UI-Kit Tree Rows](/Users/niklas/Developer/active/Labonair/Labonair-rust/crates/ui-kit/src/tree_row.rs)
- [UI-Kit Number Field](/Users/niklas/Developer/active/Labonair/Labonair-rust/crates/ui-kit/src/number_field.rs)

Normative Labonair-Verträge:

- [Settings Contract](/Users/niklas/Developer/active/Labonair/Labonair-rust/docs/settings.md)
- [Settings Guidelines](/Users/niklas/Developer/active/Labonair/Labonair-rust/docs/settings-guidelines.md)
- [Settings Inventory](/Users/niklas/Developer/active/Labonair/Labonair-rust/docs/settings-inventory.md)
- [Design System](/Users/niklas/Developer/active/Labonair/Labonair-rust/docs/design-system.md)
- [Visual Verification](/Users/niklas/Developer/active/Labonair/Labonair-rust/docs/visual-verification.md)
- [Product Surface Acceptance](/Users/niklas/Developer/active/Labonair/Labonair-rust/docs/audits/product-surface-acceptance.md)

---

# 2. Zeds Settings UI im Detail

## 2.1 Gesamtstruktur

Zed verwendet folgende Struktur:

```text
Settings Window
├── linke Navigation
│   ├── Suchfeld
│   ├── hierarchischer Tree
│   │   ├── Kategorie
│   │   └── Abschnitt
│   └── Fokus-Navigation-Footer
└── rechte Inhaltsseite
    ├── Seitenkopf
    │   ├── Settings-Datei/Scope
    │   └── Edit-in-JSON-Aktion
    ├── Bereichstitel
    ├── Abschnittsüberschriften
    └── virtuelle Liste von Setting-Zeilen
```

Die wichtigsten Referenzwerte:

| Bereich | Zed |
|---|---:|
| Sidebar-Breite | 226 px |
| minimale Inhaltsbreite | 400 px |
| Navigation-Zeile | 28 px |
| Suchfeld | 28 px hoch |
| Inhaltsabstand oben | 24 px |
| Input-Höhe | 32 px |
| Abschnittsüberschrift | kleiner, gedämpfter Text plus Divider |
| Setting-Zeile | linker Informationsteil plus rechter Control-Bereich |
| linke Beschreibungsspalte | maximal ungefähr zwei Drittel |
| lange Listen | virtualisiert |
| Footer der Navigation | ungefähr 32 px |

Die Zed-Navigation ist semantisch ein Tree:

- `Tree` als Containerrolle;
- `TreeItem` für Kategorien und Untereinträge;
- `aria-level`;
- `aria-expanded`;
- `aria-selected`;
- sichtbarer Fokusrahmen;
- Expand/Collapse per Tastatur;
- Fokuswechsel zwischen Navigation und Inhalt;
- automatisches Scrollen des fokussierten Eintrags.

Labonair verwendet aktuell eine optisch ähnliche, aber technisch einfachere Handkomposition aus `div`-Elementen.

---

## 2.2 Zeds linke Navigation

Zeds Navigation besitzt fünf wichtige Eigenschaften.

### 1. Keine zusätzliche dritte Navigationsebene

Zed verwendet:

```text
Kategorie
└── Abschnitt oder Unterseite
```

Die Kategorie ist der Root-Eintrag. Abschnittsüberschriften darunter sind entweder direkte Sprungziele oder Unterseiten.

Labonair besitzt zusätzlich die statischen Gruppen:

```text
GENERAL
WORK
CONNECTIONS
```

Diese Gruppen sind zwar nicht interaktiv, bilden aber optisch eine zusätzliche Hierarchie. Für vollständige Zed-Parität empfehle ich, diese Großbuchstaben-Gruppen langfristig zu entfernen. Die sieben Settings-Kategorien reichen aus.

Ziel:

```text
General
  Appearance
  Startup
  Session Restore
  Updates

Appearance
  Typography
  Density & Motion
  Layout
  Zen Mode

Terminal
  Shell
  Font
  Cursor
  Scrolling
```

Das entspricht besser dem Zed-Modell und spart vertikalen Platz.

### 2. Root-Zeilen besitzen echte Disclosure-Steuerung

Zed:

- Chevron in eigener Spalte;
- ganze Zeile fokussierbar;
- Klick auf Kategorie öffnet die Kategorie;
- Chevron expandiert oder kollabiert;
- Doppelklick kann die Expansion verändern;
- aktive Kategorie erhält eine Selection-Fläche;
- Fokus erhält einen separaten Fokusrahmen.

Labonair hat bereits Chevron und Expansion, aber:

- keine vollständige Tree-Semantik;
- keinen eigenständigen Fokuszustand;
- keine vollständige Tastatursteuerung;
- Expansion wird beim Öffnen jedes Mal zurückgesetzt;
- die Navigation wird nicht virtualisiert.

### 3. Suchmodus expandiert passende Kategorien automatisch

Zed:

- Suchanfrage filtert die sichtbaren Navigationseinträge;
- passende Root-Kategorien werden automatisch geöffnet;
- leere Abschnittseinträge verschwinden;
- Treffer werden in den Inhaltsbereich gespiegelt;
- Fokus kann zwischen Suchfeld, Navigation und Inhalt wechseln.

Labonair besitzt bereits einen guten globalen Suchindex und Search-Jump-Mechanismus. Dieser sollte erhalten bleiben. Die fehlenden Punkte liegen hauptsächlich in:

- Fokuswechsel;
- Tree-Semantik;
- No-Results-Layout;
- Scroll- und Virtualisierungsverhalten.

### 4. Fokus-Footer

Zed zeigt unten in der Sidebar eine kleine Tastaturhilfe, beispielsweise:

```text
⌘?  Focus Content
```

Labonair besitzt bereits die UI-Kit-Komponente [`keybinding_hint`](\/Users/niklas/Developer/active/Labonair/Labonair-rust/crates/ui-kit/src/kbd.rs), nutzt sie aber in der Settings-Navigation noch nicht.

Dieser Footer sollte übernommen werden.

### 5. Navigation ist eine fokussierbare Gruppe

Zed unterstützt:

- Focus Search;
- Focus Navbar;
- Focus Content;
- Next/Previous Navigation Entry;
- Next/Previous Root Entry;
- First/Last Entry;
- Expand;
- Collapse;
- Tab-Navigation zwischen Regionen.

Labonair sollte dieselben Verhaltensziele abbilden, aber über Labonair-eigene Commands und Keymap-Verträge.

---

## 2.3 Zeds Inhaltsbereich

Zed verwendet keine Karten pro Setting.

Die Struktur ist:

```text
Seitenkopf
  Section Header
    Setting Row
    Setting Row
  Section Header
    Setting Row
    Setting Row
```

Eine Setting-Zeile besitzt:

```text
┌────────────────────────────────────────────────────────────┐
│ Titel                         Control                       │
│ Beschreibung                 Reset / Scope / Value        │
└────────────────────────────────────────────────────────────┘
```

Wichtige Layoutregeln:

- linke Beschreibungsspalte bekommt `max-width`;
- rechter Control-Bereich bleibt kompakt;
- Beschreibung darf umbrechen;
- Controls werden nicht durch lange Texte verdrängt;
- Zeilen besitzen keine Kartenrundung;
- Divider erscheinen nur zwischen tatsächlichen Einträgen;
- nach dem letzten Eintrag eines Abschnitts gibt es mehr Abstand;
- Abschnittsüberschriften sind klein, gedämpft und durch eine horizontale Linie ergänzt;
- Inhaltslisten sind scrollbar und bei großen Listen virtualisiert.

Labonair ist schon näher an diesem Modell als an der alten `reference-src`-Oberfläche:

- keine Karten pro Zeile;
- Zeilen mit Bottom-Divider;
- getrennte linke und rechte Bereiche;
- wiederverwendete Controls;
- generierte Felder.

Die wichtigsten Unterschiede:

| Bereich | Labonair aktuell | Ziel |
|---|---|---|
| Content-Inset | 24 px an allen Seiten | Zed-artige kleine horizontale Einrückung, größerer oberer Abstand |
| Section Header | 15 px semibold, ohne Divider | kleiner, gedämpfter Text plus Divider |
| Row-Abstand | `py(14)` | typisierte Zed-Abstände, ungefähr `pt16/pb16` |
| linke Spalte | flexibel | maximal ungefähr zwei Drittel |
| letzte Zeile | normaler Abstand | zusätzlicher Abschnittsabstand |
| lange Liste | vollständig aufgebaut | bounded/virtualized |
| semantische Rollen | kaum vorhanden | Group, Heading, Label, Field |
| Search Empty State | einfacher Text oben | zentrierter Empty State |

---

## 2.4 Zeds Seitenkopf und Scope-Modell

Zed besitzt im Seitenkopf einen Settings-Datei-Switcher:

```text
User Settings
Project Settings
Server Settings
...
```

Das ist kein rein visueller Zusatz. Zed macht damit sichtbar, welche Datei bzw. Ebene gerade bearbeitet wird.

Labonair verwendet aktuell:

- einen effektiven gemergten Wert;
- `Default`, `User` oder `Project` als Origin Badge;
- einen Reset-Button;
- immer einen User-Layer-Schreibpfad.

Das ist derzeit funktional problematisch:

1. Ein Wert kann sichtbar aus dem Projekt stammen.
2. Eine Änderung wird trotzdem in den User-Layer geschrieben.
3. Der Nutzer erhält keine klare Scope-Auswahl.
4. Reset schreibt aktuell den Default-Wert in den User-Layer, statt den Override zu entfernen.

Der aktuelle Code dokumentiert diese Einschränkung ausdrücklich in [`SettingsView::reset_field`](\/Users/niklas/Developer/active/Labonair/Labonair-rust/crates/settings-ui/src/view.rs).

Für Labonair sollte Zeds Dateiauswahl nicht wörtlich kopiert werden. Stattdessen sollte ein Labonair-spezifischer Scope-Switcher entstehen:

```text
Scope: User ▾
```

Bei aktivem Projekt:

```text
Scope: User ▾      Scope: Project
```

Empfohlenes Verhalten:

- `Default`: kein Override aktiv;
- `User`: Änderung wird in `config.json` geschrieben;
- `Project`: Änderung wird in `.labonair/settings.json` geschrieben;
- `Project` ohne aktives Projekt: nicht auswählbar;
- ein Projekt-Override wird als `Project` angezeigt;
- Reset entfernt den Override der aktuell ausgewählten Ebene;
- `Edit in config.json` öffnet die zur Ebene passende Datei;
- niemals stillschweigend in den falschen Layer schreiben.

Das ist eine notwendige funktionale Voraussetzung für echte Zed-Parität.

---

## 2.5 Zeds Setting-Zeilen

Zeds Setting-Row enthält:

- Titel;
- Beschreibung;
- optional Reset;
- optional „Modified in …“;
- rechter Control;
- optional ein Link-Icon für den JSON-Pfad;
- Hover-, Fokus- und Disabled-Zustände.

Labonair besitzt bereits:

- Titel;
- Beschreibung;
- Hint;
- Unit;
- `Default`/`User`/`Project` Badge;
- Reset-Icon;
- rechten Control.

Das Labonair-Origin-Badge sollte beibehalten werden, weil es zum Settings-Vertrag gehört. Die Anzeige sollte aber Zed-ähnlicher werden:

```text
Font size          User  ↺
Editor font size
```

oder bei Projektwerten:

```text
Font size          Project  ↺
Editor font size
```

Nicht übernommen werden sollte Zeds `zed://settings/...`-Link, solange es dafür keinen Labonair-Command- oder URI-Vertrag gibt. Das wäre eine zusätzliche Funktion und keine reine UI-Anpassung.

---

# 3. Abgleich mit dem aktuellen Labonair-Stand

## Bereits gut gelöst

Labonair besitzt bereits eine solide Basis:

- dediziertes natives Settings-Fenster;
- typed `SettingsContent`;
- generierte Feldoberfläche;
- zentrale `SettingsStore`-Layer;
- globaler Search-Index;
- Deep-Linking auf Kategorie und Abschnitt;
- UI-Kit-basierte Buttons, Inputs, Selects, Number Fields und Switches;
- systemweite Font-Ermittlung außerhalb des UI-Threads;
- anchored Dropdowns außerhalb des Scroll-Clips;
- globale Notifications für Speicher- und Diagnosefehler;
- keine Hosts-, Themes-, Icon-Themes- oder Shortcuts-Management-Seiten;
- separate Theme-Ownership;
- getrennte General-, Appearance-, Terminal-, Editor-, File-Manager-, Workspace- und Connections-Bereiche.

Das ist architektonisch bereits näher am Ziel als eine komplette Neuentwicklung.

## Die wichtigsten Lücken

### P0: Scope und Reset

Das ist zuerst zu lösen, weil sonst die Oberfläche falsche Erwartungen erzeugt.

- Schreiben in User oder Project muss explizit sein.
- Reset muss Overrides entfernen.
- Project-Werte dürfen nicht still in User geschrieben werden.
- Origin Badge und aktiver Scope müssen konsistent sein.

### P0: Navigation und Fokus

- echte Tree-Semantik;
- Fokusrahmen;
- Fokuswechsel zwischen Search, Nav und Content;
- Expand/Collapse per Tastatur;
- Footer mit Fokus-Hinweis;
- stabile Expansion statt Reset bei jedem Öffnen;
- uniform/virtualized Navigation.

### P0: Content-Geometrie

- Sidebar von 208 auf 226 px angleichen;
- Content-Inset neu vermessen;
- Section Header mit Divider;
- linke Textspalte begrenzen;
- Controls rechts stabil halten;
- Zed-artige letzte-Row-Abstände;
- horizontales Overflow bei schmalem Fenster vermeiden.

### P0: Interaktive Controls

Der aktuelle UI-Kit-Stand ist visuell brauchbar, aber nicht vollständig Zed-ähnlich.

#### Textfelder

Aktuell zeigt Labonair meistens eine klickbare Darstellung und erzeugt erst beim Bearbeiten ein echtes Feld.

Ziel:

- echtes Input-Feld direkt sichtbar;
- 32 px Höhe;
- native Auswahl, Clipboard, IME und Undo;
- Enter bestätigt;
- Escape verwirft;
- Blur bestätigt;
- Focus-Ring;
- definierte Placeholder-Darstellung;
- persistente Feldidentität beim Scrollen.

#### Selects

Der aktuelle `select_trigger` ist ein tokengebundenes `div` mit Click-Handler.

Ziel:

- `ComboBox`-Semantik;
- `aria-label`;
- `aria-value`;
- `aria-expanded`;
- Tab-Fokus;
- Pfeiltasten;
- Enter/Space öffnet;
- Escape schließt;
- ausgewählter Eintrag wird sichtbar markiert;
- Popover bleibt am Trigger verankert und klappt am Fensterrand korrekt um.

#### Number Fields

Das aktuelle Labonair-NumberField besitzt:

- Minus;
- Wert;
- Plus;
- optionalen Track;
- Clamping.

Zed besitzt zusätzlich:

- Read/Edit-Modus;
- direkten Wert-Edit;
- Tastatursteuerung;
- Spinbutton-Semantik;
- Fokus-Ring;
- Modifier-basierte Schritte;
- Accessible Increment/Decrement;
- Reset-Integration.

Empfehlung:

- für Settings standardmäßig keinen Slider-Track verwenden;
- Track nur bei echten Slider-Semantiken;
- Settings-Zahlen als direkte Spinbox darstellen;
- Integer und Float identisch behandeln;
- Schrittweiten, Min/Max und Dezimalstellen aus `FieldControl` ableiten;
- Wert bei Blur validieren und clampen.

#### Switches

- UI-Kit-Komponente beibehalten;
- Focus-visible-State sicherstellen;
- `aria-checked`;
- disabled und error definieren;
- keine direkte Abhängigkeit von nicht synchronisierten globalen `gpui-component`-Tokens.

#### Font Picker

Die Font-Liste ist potentiell lang und asynchron.

Benötigt werden:

```text
Loading
Ready
Empty
Error
```

Fehler werden weiterhin global in Notifications publiziert. Loading darf nicht wie eine leere Font-Liste aussehen.

#### SFTP Columns

Die Spaltenverwaltung bleibt eine zulässige Custom-Interaktion, weil sie nicht mit einem normalen Feld ausreichend beschrieben ist.

Sie benötigt zusätzlich:

- Tab-Fokus für jede Checkbox;
- fokussierbare Up/Down-Buttons;
- Disabled-Zustände;
- tastaturbasierte Reorder-Funktion;
- klare Screenreader-Beschriftung.

---

# 4. Zielbild für Labonair

```text
Native Settings Window
├── OS titlebar
└── Settings Surface
    ├── Settings Navigation
    │   ├── Search field
    │   ├── Tree navigation
    │   │   ├── General
    │   │   │   ├── Appearance
    │   │   │   ├── Startup
    │   │   │   └── Updates
    │   │   ├── Appearance
    │   │   ├── Terminal
    │   │   ├── Editor
    │   │   ├── File Manager
    │   │   ├── Workspace
    │   │   └── Connections
    │   └── Focus navigation hint
    └── Settings Content
        ├── Scope selector + config action
        ├── Page title
        ├── Search results or generated page
        ├── Section header
        ├── Setting field row
        │   ├── label
        │   ├── description
        │   ├── origin badge
        │   ├── reset button
        │   └── control
        └── anchored popover layer
```

Empfohlene Geometrie:

| Element | Ziel |
|---|---:|
| Settings Sidebar | 226 px |
| Settings Window Minimum | bestehende 760 × 480 beibehalten |
| nominale Breite | bestehende 1040 px zunächst beibehalten |
| Navigation Row | 28 px, density-aware |
| Search Field | 28 px |
| Content Top Padding | 24 px |
| Page Header | kompakter als aktuelle 44 px, nach Screenshot-Baseline festlegen |
| Setting Input | 32 px |
| Section Header | kleiner Muted-Text, Divider |
| Setting Row | links max. 2/3, rechts stabiler Control-Bereich |
| Focus Indicator | 1–2 px, nicht durch Density skalieren |

Wichtig: Zed verwendet `pt_10()` auf macOS, weil die Zed-Navigation in einer clientseitig dekorierten Window-Struktur direkt unter den Traffic Lights liegt. Labonair nutzt ein natives GPUI-Fenster mit eigener Titlebar-Konfiguration. Dieser 40-px-Abstand darf deshalb nicht blind übernommen werden. Das muss anhand eines nativen Screenshots entschieden werden.

---

# 5. Komponentenplan

## 5.1 Bestehende Komponenten weiterverwenden

Keine neuen lokalen Varianten erzeugen für:

- Buttons;
- Icon Buttons;
- Checkboxes;
- Switches;
- Inputs;
- Select Popovers;
- Tooltips;
- Badges;
- Dividers;
- Keyboard Hints;
- Tree Rows.

Bestehende Kandidaten:

- `Palette`
- `Density`
- `TreeRow`
- `keybinding_hint`
- `field_input`
- `button`
- `select_popover`
- `checkbox`
- `number_field`
- `Switch`
- `Divider`

## 5.2 UI-Kit gezielt erweitern

### `TreeRow`

Erweiterungen:

- `role(TreeItem)`;
- `aria-level`;
- `aria-expanded`;
- `aria-selected`;
- FocusHandle;
- Focus-visible border;
- separate selected/focused/hover states;
- Root- und Child-Variante;
- Chevron-Hitbox;
- optional disabled state.

Keine Zed-Strukturen kopieren. Die bestehende Labonair-`TreeRow`-API bleibt die Grundlage.

### `NumberField`

Erweiterungen:

- Edit-Modus;
- direkte Texteingabe;
- `role(SpinButton)`;
- Min/Max/Step-Accessibility;
- Increment/Decrement-Actions;
- Modifier-Schritte;
- optional Reset;
- `track(false)` als Settings-Standard;
- validierte Commit-Callbacks.

### `select_trigger`

Erweiterungen:

- ComboBox-Rolle;
- Tab-Index;
- `aria-label`;
- `aria-description`;
- `aria-value`;
- `aria-expanded`;
- Focus-visible-State;
- Open/Close per Tastatur.

### generischer Section Header

Falls der vorhandene `list_header` nicht ausreicht:

```text
SectionHeader
├── muted label
├── optional icon
└── horizontal divider
```

Dieser bleibt generisch und wird nicht als Zed-spezifischer Component-Port bezeichnet.

## 5.3 Settings-UI-interne Komponenten

Diese gehören in `crates/settings-ui`, nicht zwingend in das UI-Kit:

- `SettingsNavigation`
- `SettingsPageHeader`
- `SettingsScopeSelector`
- `SettingsFieldRow`
- `SettingsOriginBadge`
- `SettingsSearchResults`
- `SettingsFontPickerState`
- `SettingsContentList`
- `SettingsDiagnosticState`

Sie dürfen nur UI-Kit-Komponenten und Design Tokens kompositionieren.

---

# 6. Umfangreicher Umsetzungsplan

## Phase 0: Baseline und Vertragsprüfung

Ziel: Vor der visuellen Änderung sicherstellen, dass die angezeigten Felder tatsächlich dem aktuellen Settings-Vertrag entsprechen.

Arbeiten:

1. native Settings-Fenster mit `cargo run -p labonair` starten;
2. Screenshots der aktuellen Oberfläche erstellen;
3. dieselben Zustände in der Zed-Referenz analysieren;
4. `schema.rs`, `pages.rs`, `settings-content` und `settings-inventory.md` vergleichen;
5. veraltete oder widersprüchliche Felder klassifizieren;
6. Scope- und Reset-Verhalten dokumentieren;
7. Settings-spezifische Findings an R07-001 anhängen.

Wichtig: In der aktuellen `schema.rs` existieren mehrere Felder, die die normative Inventory als `Remove` markiert, beispielsweise verschiedene Editor- und Auto-Save-Felder. Diese Diskrepanz muss vor der finalen UI-Abnahme geklärt werden. Sie darf nicht durch bloßes Styling verborgen werden.

Abnahme:

- jede sichtbare Field Definition besitzt Consumer, Default, Scope, Validation und Reset-Verhalten;
- keine Management-Kategorie wird hinzugefügt;
- keine Referenzdatei wird verändert.

---

## Phase 1: Scope-, Layer- und Reset-Vertrag

Betroffene Bereiche:

- `crates/settings/src/store.rs`
- `crates/settings-ui/src/view.rs`
- `crates/settings-ui/src/schema.rs`
- `crates/settings-ui/src/tests.rs`
- `docs/settings.md`
- `docs/settings-guidelines.md`

Arbeiten:

1. aktiven Schreib-Scope definieren;
2. User- und Project-Schreiben trennen;
3. `clear_user_override(path)` ergänzen;
4. Project-Override-Entfernung ergänzen;
5. effektiven Wert und tatsächliche Zielschicht unterscheiden;
6. Origin-Badge aus dem effektiven Wert ableiten;
7. Reset als Löschen des Overrides implementieren;
8. Speicherfehler weiterhin über Notifications publizieren;
9. bei aktivem Project-Scope das Projekt-JSON öffnen;
10. keine stillen User-Layer-Schreibvorgänge bei Projektwerten.

Tests:

- User-Wert wird nur in User-Datei geschrieben;
- Project-Wert wird nur in Project-Datei geschrieben;
- Reset entfernt den Override vollständig;
- nach Reset wird `Default` angezeigt;
- Project-Override bleibt wirksam, wenn nur der User-Wert geändert wird;
- ein Project-Override kann nicht versehentlich in User umgewandelt werden.

---

## Phase 2: Layout-Tokens und Window-Geometrie

Betroffene Dateien:

- `crates/settings-ui/src/window.rs`
- `crates/settings-ui/src/view.rs`
- `crates/ui-kit/src/density.rs`
- gegebenenfalls Theme-Token-Dateien

Arbeiten:

1. Sidebar von 208 auf 226 px umstellen;
2. Settings-spezifische Maße benennen;
3. keine neuen Inline-Spacing-Werte verteilen;
4. Header-Höhe anhand nativer Baseline überprüfen;
5. Traffic-Light-Kompensation überprüfen und wahrscheinlich reduzieren;
6. Content-Inset nach Zed-Modell neu strukturieren;
7. 760 × 480 Narrow Mode testen;
8. 1040-px-Normalmodus gegen Long Descriptions testen;
9. Dark/Light/Compact/Default/Comfortable vermessen.

Zielstruktur:

```text
Content Root
├── top padding: 24
├── page header: inner padding 8
├── section header: inner padding 8
└── field row: inner padding 8
```

Nicht übernehmen:

- Zeds clientseitige Titlebar-Top-Padding, wenn Labonair bereits native OS-Traffic-Lights verwendet;
- Zeds exakte Farben;
- harte Zed-Theme-Referenzen.

---

## Phase 3: Navigation neu aufbauen

Betroffene Dateien:

- `crates/settings-ui/src/view.rs`
- `crates/ui-kit/src/tree_row.rs`
- gegebenenfalls neue lokale Datei `crates/settings-ui/src/navigation.rs`

Arbeiten:

1. statische `GENERAL`, `WORK`, `CONNECTIONS`-Bänder entfernen oder auf rein visuelle Übergangsphase begrenzen;
2. sieben Root-Kategorien als Tree rendern;
3. Sections als Child-Einträge rendern;
4. `TreeRow` für Root/Child verwenden;
5. `uniform_list` für Navigation einsetzen;
6. `role(Tree)` und `TreeItem` ergänzen;
7. Expansion nicht bei jedem Öffnen blind löschen;
8. aktive Kategorie automatisch expandieren;
9. Suchanfragen öffnen passende Kategorien;
10. Fokuszustände getrennt von Selection rendern;
11. Footer mit `keybinding_hint` hinzufügen;
12. Fokuswechsel zwischen Navigation und Inhalt implementieren;
13. Pfeiltasten und Expand/Collapse ergänzen;
14. Search-Focus-Command integrieren.

Abnahme:

- gesamte Navigation per Tastatur bedienbar;
- Fokus ist jederzeit sichtbar;
- aktive Kategorie und fokussierte Kategorie sind unterscheidbar;
- Abschnittsklick scrollt korrekt;
- Suchtreffer expandieren den passenden Root;
- Navigation bleibt bei langen Listen stabil.

---

## Phase 4: Inhaltslayout und virtuelle Liste

Betroffene Dateien:

- `crates/settings-ui/src/view.rs`
- `crates/settings-ui/src/panes/generic.rs`
- `crates/settings-ui/src/pages.rs`
- gegebenenfalls `crates/settings-ui/src/search.rs`

Arbeiten:

1. `render_generated_body` auf eine bounded/virtualized Liste umstellen;
2. stabile Item-Indizes für Section und Field definieren;
3. Search-Jump an virtuelle Zeilen anbinden;
4. `ScrollHandle` und Virtualisierung synchronisieren;
5. `SettingsSectionHeader` mit Divider rendern;
6. Field Rows mit Zed-artigen Abständen rendern;
7. linke Spalte auf ungefähr zwei Drittel begrenzen;
8. Controls rechts nicht schrumpfen lassen;
9. letzte Zeile eines Abschnitts mit größerem Bottom-Abstand versehen;
10. Section-Anchor-Scrolling beibehalten;
11. Search Empty State zentrieren;
12. bei keiner Suchanfrage die normale Kategorieansicht zeigen;
13. keine Karten pro Setting einführen.

Ziel:

```text
Page Title

Section Header ───────────────────────────

Setting Row                         Control
Setting Row                         Control

Section Header ───────────────────────────
```

---

## Phase 5: Textfelder und Selects

### Textfelder

Betroffene Dateien:

- `crates/ui-kit/src/text_field.rs`
- `crates/settings-ui/src/panes/generic.rs`
- `crates/settings-ui/src/view.rs`

Arbeiten:

1. Textfeld direkt als echtes Input rendern;
2. Lazy-Fake-Darstellung entfernen oder nur für nicht editierbare Preview-Zustände verwenden;
3. per-field Input-State stabil halten;
4. Enter/Blur committen;
5. Escape revertieren;
6. Focus-Ring ergänzen;
7. Placeholder `(default)` klar darstellen;
8. Feldwert nicht durch Repaint überschreiben, wenn der Nutzer gerade editiert;
9. Invalid-State über Border/Ring anzeigen;
10. operative Fehler global melden.

### Selects

Arbeiten:

1. `select_trigger` auf ComboBox-Semantik erweitern;
2. per Tastatur öffnen;
3. Pfeiltasten innerhalb der Liste;
4. Enter übernimmt Auswahl;
5. Escape schließt;
6. aktuelles Element sichtbar markieren;
7. Popover am Trigger verankern;
8. Position bei unteren Fensterrändern testen;
9. lange Optionslisten scrollen oder virtualisieren;
10. Font Picker als spezialisierte, aber UI-Kit-basierte Variante ausführen.

---

## Phase 6: Number Fields und Spezialfelder

### Numeric Settings

Arbeiten:

1. Track im Settings-Kontext deaktivieren;
2. Minus, Wert und Plus als eine zusammenhängende Spinbox darstellen;
3. Klick auf Wert öffnet Edit-Modus;
4. Pfeil hoch/runter ändern den Wert;
5. Shift/Alt-Schritte definieren;
6. Min/Max clampen;
7. Dezimaldarstellung stabilisieren;
8. `aria-valuemin`, `aria-valuemax`, `aria-valuenow`, `aria-valuestep`;
9. Reset integrieren;
10. Disabled-at-boundary korrekt rendern.

### SFTP Columns

Arbeiten:

1. Checkbox-Zustand fokussierbar machen;
2. Up/Down-Buttons mit Labels versehen;
3. Reorder per Tastatur;
4. Disabled-Zustände anzeigen;
5. Spaltenreihenfolge nach jeder Änderung sofort speichern;
6. `sftpColumns` weiterhin als einziges kanonisches Feld verwenden.

---

## Phase 7: Seitenkopf, Scope Selector und About-Bereich

### Seitenkopf

Empfohlene Struktur:

```text
[Scope: User ▾]                     [Edit in config.json]
```

Bei einer Subpage:

```text
[←] Terminal / Advanced              [Edit in config.json]
```

Die vorhandene Breadcrumb-Funktion kann bleiben, sollte aber in eine kompaktere Zed-artige Seite integriert werden.

### About Hero

Der aktuelle About-Hero ist stark von `reference-src` geprägt:

- große Karte;
- Labonair-Logo;
- Check-for-Updates-Button;
- GitHub-/Website-/Report-Links.

In der untersuchten Zed-Settings-Oberfläche gibt es keinen vergleichbaren großen Labonair-About-Card-Block.

Für vollständige Zed-Parität empfehle ich:

- den großen Hero zu entfernen;
- Version und Update-Verhalten als normale General-Settings oder kompakte Kopfzeile darzustellen;
- keine große visuelle Karte oberhalb der Settings-Liste zu behalten.

Falls der Hero produktseitig gewünscht ist, sollte er als bewusste Labonair-Ausnahme dokumentiert werden. Er darf nicht als Teil der Zed-Parität gelten.

---

## Phase 8: States, Accessibility und Motion

Für jedes relevante Control müssen folgende Zustände überprüft werden:

- normal;
- hover;
- pressed;
- selected;
- focused;
- disabled;
- error;
- loading;
- empty.

Settings-spezifisch:

### Search

- kein Suchtext;
- Treffer;
- keine Treffer;
- Tastaturauswahl;
- Escape löscht zuerst die Suche;
- zweites Escape schließt das Fenster.

### Font Picker

- Fonts werden geladen;
- Font-Liste ist bereit;
- Font-Liste ist leer;
- Font-Scan schlägt fehl;
- Fehler erscheint in Notifications;
- Selector bleibt nicht dauerhaft scheinbar leer.

### Settings-Datei

- gültige User-Datei;
- gültige Project-Datei;
- malformed JSON;
- Schreibfehler;
- Migration-Warnung;
- unbekannter Schlüssel.

Es soll keine zweite Inline-Fehleroberfläche entstehen, wenn derselbe Fehler bereits im Notification Center angezeigt wird.

### Reduced Motion

- Dropdown-Fade und Highlight-Animation respektieren `reduceMotion`;
- kein animierter Pulse bei reduziertem Motion-Modus;
- Fokusindikatoren bleiben immer sofort sichtbar.

---

# 7. Empfohlene Dateistruktur der Umsetzung

| Datei | Aufgabe |
|---|---|
| `crates/settings-ui/src/window.rs` | Window-Größe und Window-Optionen |
| `crates/settings-ui/src/view.rs` | Settings-Surface, Header, Fokus, Scope |
| `crates/settings-ui/src/navigation.rs` | optionaler lokaler Navigationsrenderer |
| `crates/settings-ui/src/panes/generic.rs` | Field Rows und Controls |
| `crates/settings-ui/src/pages.rs` | reine Kategorie-/Abschnittsplatzierung |
| `crates/settings-ui/src/schema.rs` | typed Field Metadata |
| `crates/settings-ui/src/search.rs` | Search-Index und Treffer |
| `crates/settings-ui/src/tests.rs` | UI-/Scope-/Reachability-Tests |
| `crates/settings/src/store.rs` | Override-Löschen und Layer-spezifisches Schreiben |
| `crates/ui-kit/src/tree_row.rs` | Tree-Semantik und Fokus |
| `crates/ui-kit/src/number_field.rs` | Editierbare Spinbox |
| `crates/ui-kit/src/select.rs` | ComboBox-/Focus-Verhalten |
| `crates/ui-kit/src/text_field.rs` | native Settings-Inputs |
| `crates/ui-kit/src/density.rs` | benannte Layout-Metriken |
| `docs/settings.md` | Scope-/Reset-Vertrag |
| `docs/design-system.md` | neue UI-Kit-Metriken und States |
| `docs/audits/product-surface-acceptance.md` | visuelle Abnahmematrix |
| `tasks/rework/` | bounded follow-up nach R07-001 |

---

# 8. Visuelle Abnahmematrix

Die bestehende Matrix in R07-001 ist aktuell noch offen. Für Settings müssen mindestens diese Zustände geprüft werden:

| Zustand | Erwartung |
|---|---|
| Normal | General, Appearance, Terminal und Editor sauber gerendert |
| Narrow | 760 × 480 ohne horizontales Überlaufen |
| Focused | Search, Navigation, Select, Text, Number und Switch besitzen sichtbaren Fokus |
| Empty | keine Suchtreffer zeigen zentrierten Empty State |
| Loading | Font Picker zeigt Loading statt leerer Auswahl |
| Error | Save-/Parse-Fehler gehen in Notifications, kein doppeltes Inline-Fehlersystem |
| Long list | Editor und Terminal bleiben scroll- und fokusstabil |
| Overlay | Select-Popover bleibt am Trigger und wird nicht abgeschnitten |
| Scope | Default/User/Project korrekt sichtbar |
| Reset | aktiver Override wird wirklich entfernt |
| Dark | Kontrast und Divider klar |
| Light | Border und Muted Text nicht zu schwach |
| Compact | Controls bleiben bedienbar |
| Comfortable | Zeilen wachsen konsistent |
| Reduced motion | keine unnötigen Animationen |

Die Prüfung muss mit dem nativen Rust-Prozess erfolgen, nicht mit der Legacy-Tauri-Anwendung. Die vorhandene Dokumentation weist bereits darauf hin, dass die vollständige visuelle Matrix wegen macOS-Screen-Recording-Berechtigungen bisher nicht vollständig bestätigt werden konnte.

---

# 9. Reihenfolge im aktiven Rework-Queue

Die aktuelle Queue besitzt mit R07-001 weiterhin die früheste unvollständige Aufgabe.

Darum sollte die Umsetzung so eingeordnet werden:

1. Settings-Baseline und Abweichungen an R07-001 dokumentieren.
2. Die visuelle Settings-Matrix dort vervollständigen, soweit der aktuelle Stand dies erlaubt.
3. Den Scope-/Reset-Konflikt als bounded Follow-up erfassen.
4. Danach den eigentlichen Settings-Parity-Slice als neue geordnete Rework-Aufgabe aufnehmen.
5. Nicht direkt mit R09 oder einem parallelen Architekturpfad beginnen.
6. Erst nach Contract, Store und UI-Kit-Primitiven die SettingsView umbauen.

Die tatsächliche Implementierungsreihenfolge muss lauten:

```text
Settings-Vertrag
→ Layer/Reset
→ UI-Kit-Komponenten
→ Settings-Navigation
→ Content-Liste
→ Field Controls
→ States/Accessibility
→ visuelle Abnahme
→ Dokumentations- und Queue-Abschluss
```

## Definition of Done

Die Settings-Oberfläche ist erst fertig, wenn:

- alle sieben Labonair-Kategorien korrekt dargestellt werden;
- keine verbotenen Management-Kategorien existieren;
- Navigation und Fields vollständig per Tastatur funktionieren;
- User- und Project-Scope eindeutig sind;
- Reset Overrides entfernt;
- Selects, Inputs und Number Fields echte native Interaktionen besitzen;
- lange Listen stabil bleiben;
- alle UI-Kit-Regeln eingehalten werden;
- keine Zed-Implementierung kopiert wurde;
- die Zed-artige visuelle Hierarchie erreicht ist;
- die vollständige native Zustandsmatrix dokumentiert ist;
- `cargo fmt --check`;
- `cargo check --workspace --all-targets`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo test --workspace`;
- Dokumentations- und Rework-Queue-Prüfungen erfolgreich sind.
