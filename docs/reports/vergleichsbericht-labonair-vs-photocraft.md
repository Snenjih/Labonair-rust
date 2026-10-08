# Vergleichsbericht — Labonair-rust und PhotoCraft

Erstellt: 2026-10-08
Status: Beratender Vergleichsbericht; keine normative Architekturentscheidung
Vergleichsstände: Labonair-rust `master` bei `8d1de8a`, PhotoCraft `main` bei
`5896f0b`
Lokale Vergleichsquelle: `../photocraft/`

## 0. Zweck und Abgrenzung

Dieser Bericht vergleicht die Dokumentations-, Agenten-, Architektur-,
Build-, Test-, Release-, Funktions- und Menüorganisation von Labonair-rust
mit PhotoCraft. Die beiden Produkte haben unterschiedliche Ziele:

- **Labonair** ist ein nativer GPUI-Dev-Op-Arbeitsplatz für lokale und
  entfernte Terminal-, SSH-, SFTP-, Editor- und Git-Workflows.
- **PhotoCraft** ist ein nativer Rust-Bildeditor mit Photoshop-Kompatibilität,
  Dateiformat-/Rendering-Fokus, CLI, Control Protocol, MCP und WebAssembly-
  Ziel.

Die Produktfunktionen sind deshalb nicht 1:1 vergleichbar. Übertragbar sind
vor allem die Organisationsmuster: maschinenprüfbare Verträge, eindeutige
Quellen, generierte Nachweise, Agenten-Onboarding, Funktionsinventare,
End-to-End-Akzeptanz und reproduzierbare CI-Gates. Photoshop-Menüs,
WebAssembly oder Bildformat-Corpora sind keine automatische Zielarchitektur
für Labonair.

Die Untersuchung war read-only. Die Arbeitsbäume waren bei der Prüfung ohne
angezeigte Git-Änderungen. Es wurde kein vollständiger Workspace-Build
ausgeführt; die im Abschnitt 2 genannten vorhandenen Prüfungen wurden
ausgeführt und ihr Ergebnis ist ausdrücklich als Evidenz dieses Berichts
markiert.

## 1. Kurzfazit

Labonair hat derzeit die bessere konzeptionelle Governance für Ownership und
Architektur. Die Trennung zwischen normativem Ziel, aktuellem Audit, ADR,
aktiver Rework-Queue, Bericht und Archiv ist sauber und sollte erhalten
bleiben. Besonders stark sind:

- eine zentrale Dokumentationsautorität in `docs/README.md`;
- `documentation-governance.md` mit Dokumentklassen, Autoritätsreihenfolge
  und Archivregeln;
- die Capability-Matrix mit Owner, Contract-Crate, Entry Point und Ist-Status;
- die boundary-first-Regel in `feature-lifecycle.md`;
- owner-registrierte Commands, Panels, Status-Items, Themes, Hosts,
  Notifications und Transfers;
- die explizite Trennung von Zielarchitektur und unvollständiger Migration.

PhotoCraft ist derzeit bei der ausführbaren Evidenz und beim Agentenfluss
stärker. Besonders wertvoll sind:

- ein fünfminütiger Agenten-Orientierungsweg in `AGENTS.md`;
- ein engine-zentriertes Command-Modell mit stabiler ID, Parametern,
  Enablement und Ausführung;
- ein maschinell erzwungenes Dependency-Layer-Modell;
- generierte Menü-Parität, Scorecard und Corpus-/Performance-Nachweise;
- eine CI, die neben Rust-Gates auch Corpus, WASM, Layering, Scorecard,
  Plattformen und Packaging prüft;
- eine getrennte, ausführliche Automation- und Security-Dokumentation;
- ein `book/` als navigierbarer Einstieg für verschiedene Zielgruppen.

Der wichtigste aktuelle Labonair-Befund ist kein Dokumentationsdetail,
sondern ein gebrochener Architektur-Gate: `scripts/check-crate-deps.sh`
meldet zwei nicht erlaubte Abhängigkeiten von
`labonair-panel-explorer` auf `labonair-sftp` und `labonair-ssh`. Dadurch ist
der dokumentierte Anspruch „Dependency-Richtung wird in CI erzwungen“ im
aktuellen Stand nicht grün. Die Kanten müssen entweder entfernt und durch den
Explorer-Host-Vertrag ersetzt oder bewusst als zeitlich begrenzte Migration
mit Owner, Consumer und Removal Condition dokumentiert werden.

Die strategische Empfehlung lautet:

> Labonairs Ownership- und Autoritätsmodell bleibt die Basis. PhotoCrafts
> maschinenlesbare Evidence-Schicht wird darübergelegt: ein deklarativer
> Architekturvertrag, generierte Capability-/Surface-/Menu-/Scorecard-Seiten,
> ein Agenten-Handbuch, ein dokumentiertes Automation-/Security-Modell und
> verpflichtende CI-Gates.

## 2. Verifizierter Ausgangsstand

### 2.1 Ausgeführte Prüfungen

| Projekt | Prüfung | Ergebnis | Bedeutung |
|---|---|---|---|
| Labonair | `python3 scripts/check_documentation.py` | bestanden; 13 normative Dokumente und 94 Markdown-Dateien geprüft | Lokale Markdown-Links, normative Metadaten und bekannte veraltete Control-Marker sind konsistent. |
| Labonair | `python3 scripts/check_rework_queue.py` | bestanden; 43 Tasks, aktiv: `R07-001-product-surface-acceptance.md` | Sequenz und genau ein aktiver frühester unvollständiger Task sind konsistent. |
| Labonair | `scripts/check-crate-deps.sh` | **fehlgeschlagen; 2 Verstöße** | `panel-explorer → sftp` und `panel-explorer → ssh` stehen im Manifest, aber nicht in der Allowlist. |
| PhotoCraft | `cargo xtask layers` | bestanden; 28 Workspace-Pakete, keine Layer-Verstöße | Die Layer-Tabelle in `xtask/src/layers.rs` stimmt mit Cargo Metadata überein. |
| PhotoCraft | `cargo xtask scorecard --check` | bestanden | `docs/scorecard.md` entspricht den registrierten Quellen. |
| beide | `git diff --check` | bestanden | Keine Whitespace-Fehler im geprüften Arbeitsbaum. |

Die vollständigen Labonair-Gates aus `AGENTS.md` — Format, Check, Clippy und
Workspace-Tests — wurden für diesen Bericht nicht erneut als Gesamtbuild
gestartet. Ein grüner Dokumentations- oder Layer-Gate ist kein Beweis für
funktionale oder visuelle Vollständigkeit.

### 2.2 Größenordnung und Struktur

| Dimension | Labonair-rust | PhotoCraft | Einordnung |
|---|---:|---:|---|
| Cargo-Manifeste unter `crates/` bzw. `apps/` | 57 | 27 plus `xtask` (28 durch `cargo xtask layers`) | Labonair zerlegt nach Produkt-Ownership und Integrationsgrenzen; PhotoCraft nach Engine-Layern und Apps. |
| Dokumente unter `docs/` | 46 Dateien im Inventar | 47 Dateien im Inventar | Ähnliche Detailmenge, aber unterschiedliche Verteilung. |
| Zusätzliche Handbuchseiten | kein mdBook | 35 Seiten unter `book/src/` | PhotoCraft hat eine zusätzliche Zielgruppen-/Navigationsschicht. |
| Aktive Implementierungsqueue | 43 Rework-Tasks, sequenziell | Roadmap + Issue-/Scorecard-Quellen | Labonair ist stärker migrationsorientiert; PhotoCraft stärker metrikenorientiert. |
| Generierter Funktionsnachweis | kein vergleichbarer vollständiger Generator | `docs/parity.md`, aktuell 627/627 Menüeinträge live | PhotoCraft macht Command-/Menü-Abdeckung unmittelbar prüfbar. |
| Generierter Qualitätsnachweis | Architekturgraph und Prüfskripte vorhanden | `docs/scorecard.md`, Layer-/Corpus-/Perf-/Paritätsquellen | Labonair braucht eine eigene, produktgerechte Scorecard. |

Die Zahlen sind Inventarwerte, keine Qualitätswertung. Mehr Crates oder mehr
Dokumentseiten sind nur dann ein Vorteil, wenn Owner, Quelle und Gate klar
bleiben.

### 2.3 Dokumentationsdrift, die bereits sichtbar ist

PhotoCraft zeigt zugleich, warum generierte Nachweise allein nicht genügen:
`docs/parity.md` meldet am geprüften Stand 627/627 live, während `docs/roadmap.md`
noch ältere Werte wie 532/625 beziehungsweise 626/626 enthält. Das ist ein
konkreter Drift zwischen generiertem Ist-Nachweis und manuell gepflegter
Roadmap-Erzählung.

Die Konsequenz für Labonair ist nicht, weniger zu dokumentieren, sondern
Zahlen und Zustände zu klassifizieren:

- **Generated fact** — aus aktuellem Quellcode erzeugt;
- **Measured evidence** — aus einem konkreten Lauf, Corpus oder Screenshot;
- **Normative target** — gewünschter Zustand, noch kein Ist-Nachweis;
- **Narrative context** — Erklärung, Entscheidung oder historische Analyse.

Jede Zahl in Roadmaps und Berichten sollte auf eine dieser Klassen und, wo
möglich, auf eine Quelle, einen Commit und einen Befehl verweisen.

## 3. Dokumentationsarchitektur im direkten Vergleich

### 3.1 Labonair: Stärken

Labonair hat mit `docs/documentation-governance.md` ein ungewöhnlich klares
Autoritätsmodell:

| Klasse | Ort | Funktion |
|---|---|---|
| Binding instructions | `AGENTS.md` | Repositoryweite Umsetzungs- und Verifikationsregeln |
| Normativer Vertrag | `docs/*.md` | Produkt-, Architektur-, Modul-, Registry-, Settings- und Lifecycle-Regeln |
| Entscheidung | `docs/adr/` | Akzeptierte, begründete Architekturentscheidungen |
| Ist-Evidenz | `docs/audits/` | Belegt den gegenwärtigen Quellbaum und offene Migrationen |
| Forschung | `docs/reports/` | Vergleiche und Untersuchungen ohne Autorität über das Ziel |
| Ausführung | `tasks/rework/` | Aktive, geordnete Umsetzung |
| Historie | `tasks/archive/`, `docs/archive/` | Nicht mehr aktive Pläne und alte Verträge |
| Vorschlag | `ideas/` | Noch nicht bindende Produkt- oder Architekturideen |
| Kontinuität | `memory/` | Handshake, Bugs und Lernnotizen |

Die Trennung von `Target` und `Current implementation` in
`docs/capabilities.md` verhindert einen häufigen Agentenfehler: Eine schöne
Zielbeschreibung wird nicht fälschlich als implementierte Funktion gelesen.
Auch `docs/README.md` definiert eine klare Autoritätsreihenfolge.

### 3.2 PhotoCraft: Stärken

PhotoCraft beginnt mit `AGENTS.md` und führt Agenten in fünf Minuten zu:

1. Architektur und Crate-Map;
2. Entwicklung und Ausführung;
3. Contribution-Regeln und Command-Checklist;
4. Control Protocol;
5. UI-Design;
6. Roadmap, Paritätsliste und Scorecard;
7. Test-Corpora.

Das `book/src/README.md` erweitert diesen Ablauf nach Zielgruppe: Nutzer,
Contributors, Architekturarbeit, Automation und Security. Die Buchstruktur
gibt den Agenten einen sichtbaren Lesepfad, ohne `docs/` als Detailquelle zu
duplizieren. Das ist ein sehr gutes Muster für die Auffindbarkeit.

PhotoCrafts Scorecard ist als Quellcode-Modell organisiert: TOML-Checklisten,
Performance-Budgets, Baseline, Corpus-Floors und Quellbaum-Audits werden zu
`docs/scorecard.md` generiert. CI prüft die Reproduzierbarkeit. Ein Eintrag
hat ID, Ziel, Status, Issue und Notiz; dadurch ist „unvollständig“ nicht nur
eine freie Formulierung.

### 3.3 Verbesserung für Labonair

Labonair sollte seine vorhandene Governance nicht durch eine zweite parallele
Dokumentationswelt ersetzen. Empfohlen ist eine dreistufige Ergänzung:

1. **Agenten-Handbuch als Einstieg:** Ein kurzes `docs/agents/README.md` oder
   ein lokales mdBook verweist auf die bestehenden normativen Quellen und
   ordnet sie nach Änderungsart.
2. **Maschinenlesbare Quellen:** Architektur-, Capability-, Surface- und
   Qualitätsdaten werden aus kleinen TOML/JSON-Dateien oder registrierten
   Rust-Descriptors erzeugt.
3. **Generierte Ansichten:** `docs/capabilities.md`, ein Menu-/Surface-
   Katalog, ein Architekturgraph und eine Scorecard werden aus diesen Quellen
   aktualisiert und in CI auf Drift geprüft.

Damit bleiben die langen, begründeten Verträge lesbar, während Statuszahlen,
Owner und Abdeckungslisten nicht manuell auseinanderlaufen.

## 4. Agenten-Dokumentation und Arbeitsfluss

### 4.1 Was PhotoCraft besser operationalisiert

PhotoCrafts `AGENTS.md` enthält neben Architekturregeln konkrete Agenten-
Sicherheits- und Abschlussregeln:

- „Everything is a command“: neue Nutzerfunktion zuerst als Command-Spec;
- keine Panics, kein `unsafe` außerhalb einer expliziten Ausnahme;
- input-derived numbers gelten als feindlich;
- Tests sind Teil jeder Änderung;
- UI-Änderungen benötigen visuelle Evidenz;
- WASM darf nicht versehentlich brechen;
- klare Arbeitsauswahl über Roadmap, Scorecard, Parity und Devlog;
- Parallel-Agenten bekommen eigene Target-Verzeichnisse und Dateigrenzen;
- vor dem Abschluss existiert eine konkrete, konditionale Prüfliste.

Labonair hat die wichtigen Architekturregeln bereits, aber der Agentenfluss
ist stärker auf den Architekturumbau als auf jede einzelne Funktionsänderung
optimiert. Es fehlt ein kompakter „Wenn du X änderst, lies Y und führe Z aus“-
Index.

### 4.2 Empfohlenes Labonair-Agenten-Handbuch

`docs/agents/README.md` sollte ein einziger, kurzer Einstieg werden und auf
keine neue Autorität neben `AGENTS.md` verweisen. Der Inhalt sollte mindestens
diese Tabelle enthalten:

| Änderungsart | Zuerst lesen | Pflichtquellen | Pflichtprüfungen |
|---|---|---|---|
| neues Capability | `product.md`, `capabilities.md`, `modules.md`, `feature-lifecycle.md` | Owner, Contract, Entry Point, Registry, Settings, Persistence, Notifications, UI-kit | Contract-Tests, Dependency-Gate, Capability-Drift |
| Crate-/Boundary-Änderung | `architecture.md`, `repository-layout.md`, ADRs, Inventory | erlaubte Kante, Removal Condition, Graph | Cargo Metadata, Dependency-Verifier, Graph-Check |
| Command/Keymap/Palette | `registries.md`, `product.md`, CommandId-Vertrag | Owner, ID, Kontext, Default-Binding, Menü-/Palettepfad | Registry-Duplikate, Entry-Point-Äquivalenz, Keymap-Tests |
| UI/Layout | `design-system.md`, `visual-verification.md`, Surface-Katalog | normal, narrow, focused, empty, loading, error | Native Screenshot/Fixture, visuelle Acceptance-Matrix |
| Settings | `settings.md`, `settings-guidelines.md`, `settings-inventory.md` | Wert, Scope, Default, Migration, Consumer | Schema-/Consumer-Audit, Migrationstests |
| SSH/SFTP/Secrets/MCP | Security- und Automation-Handbuch | Trust Boundary, Grant, Timeout, Redaction, Error path | Negative Tests, Fuzz/Property-Tests, Protocol smoke |
| Dokumentation/Repository-Control | `documentation-governance.md` | Authority, Status, Version, Source-of-truth | Docs checker, link checker, generated-doc check |

Zusätzlich sollte der Einstieg je einen „Start“, „Währenddessen“ und
„Abschluss“-Block mit konkreten Commands enthalten. `CLAUDE.md` kann dann
weiterhin nur ein kurzer Adapter auf `AGENTS.md` bleiben; seine wiederholten
Umgebungsangaben sollten bereinigt und, falls sie nicht mehr gelten, entfernt
werden.

### 4.3 Session-Kontinuität

PhotoCraft nennt einen Devlog als Arbeitsübergabe, der aktuelle Baum enthält
aber keinen sichtbaren `log/`- oder `plan/`-Bestand. Labonair hat dafür
`handshake.md` und `memory/`, was grundsätzlich besser zu seiner
Repository-Governance passt. Es sollte jedoch klarer getrennt werden:

- `handshake.md`: nur aktueller Session-Zustand und nächster sicherer Schritt;
- `memory/bugs_and_fixes.md`: dauerhafte, nicht offensichtliche Erkenntnisse;
- ein optionaler `memory/devlog/`-Eintrag: messbare Fortschritte und offene
  Validierung;
- aktive Architekturarbeit ausschließlich in `tasks/rework/`.

Ein Agent sollte nicht in mehreren konkurrierenden Notizsystemen nach dem
aktuellen Task suchen müssen.

## 5. Architektur und Infrastruktur

### 5.1 Unterschiedliche, jeweils passende Architekturmodelle

**Labonair** organisiert nach Produktfähigkeit und Ownership. Ein Capability-
Owner besitzt Domain-State, Behavior, UI, Persistence, Commands,
Notifications und Tests; Geschwister-Crates bilden echte UI-, Storage- oder
Integration-Grenzen. Der Shell-/App-Root komponiert registrierte Beiträge.

**PhotoCraft** organisiert nach Engine-Layern:

```text
L0 Grundlagen / standalone Formate
L1 Dokumentmodell
L2 Operationen, Paint, Text, Vector
L3 Compose, GPU, natives Format
L4 I/O und Plugins
L5 Command Engine
L6 UI und Automation
Apps / xtask als Composition- und Tool-Schicht
```

PhotoCraft registriert diese Tabelle im ausführbaren `xtask` und verbietet
Aufwärts- und Seitwärtsabhängigkeiten. Labonairs Python-Allowlist prüft die
konkreten internen Kanten ebenfalls streng, ist aber stärker handcodiert und
enthält viele Migrationskommentare.

Für Labonair ist das Capability-Modell das bessere Produktmodell. Die
ausführbare Layer-Idee sollte trotzdem übernommen werden, aber als
Ownership-/Boundary-Modell statt als künstliche L0-L6-Kopie.

### 5.2 Empfohlener deklarativer Architekturvertrag

Eine neue Quelle, zum Beispiel `architecture/graph.toml`, sollte pro
Workspace-Crate erfassen:

```toml
[[crate]]
name = "labonair-panel-explorer"
owner = "explorer"
role = "ui"
contract = "labonair-explorer-host"
allowed_dependencies = [
  "labonair-explorer-host",
  "labonair-filesystem",
  "labonair-notifications",
  "labonair-panel",
  "labonair-settings",
  "labonair-theme",
  "labonair-ui-kit",
]
status = "partial"
removal_condition = "remote directory operations use ExplorerHost"
```

Ein `cargo xtask architecture` oder `scripts/check_architecture.py` erzeugt
daraus:

- die Allowed-Edges für den Verifier;
- einen aktuellen Crate-Graph mit Owner- und Rollenfarben;
- eine Tabelle „Crate → Owner → Contract → Consumer“;
- die Liste unregistrierter Crates;
- die Liste von `partial`/`transitional` Kanten;
- eine Driftprüfung zwischen Cargo Metadata und Dokumentation.

Die Quelle darf nicht die gesamte Capability-Matrix duplizieren. Sie sollte
nur maschinenprüfbare Struktur tragen; die erklärende Begründung bleibt in
`architecture.md` und den Audits.

### 5.3 Aktueller konkreter Boundary-Befund

`crates/panel-explorer/Cargo.toml` hängt direkt von
`labonair-sftp` und `labonair-ssh` ab. Der Quellcode verwendet dort
`SftpSessionService`, `SftpBrowserService` und `SshSessionId` für entfernte
Verzeichnisse. Das kollidiert mit der dokumentierten R07-004-Aussage, dass
der Explorer seine Workspace-Interaktion nur über
`labonair-explorer-host` erledigt.

Empfohlene Zielentscheidung:

1. Der Explorer besitzt weiterhin die Tree- und View-Logik.
2. Der Explorer-Host-Vertrag erhält eine klar benannte Remote-Browser-
   Capability oder einen separaten, explorer-eigenen `RemoteFileService`-
   Vertrag.
3. `labonair-sftp`/`labonair-sftp-ssh` implementieren diesen Vertrag als
   Adapter; `panel-explorer` kennt weder SSH-Session-IDs noch SFTP-Transport.
4. Die Kante wird aus `panel-explorer/Cargo.toml` entfernt.
5. Audit, Capability-Matrix und Verifier werden gemeinsam aktualisiert.

Wenn diese Entfernung noch nicht möglich ist, muss die Kante explizit als
`Partial` in `remaining-boundaries.md` mit Consumer, Owner, Task und
Entfernungsbedingung erscheinen. Ein stilles Abweichen vom Verifier ist die
schlechteste Variante, weil es CI und Dokumentation gleichzeitig unglaubwürdig
macht.

## 6. Funktions- und Capability-Management

### 6.1 Labonairs vorhandene Stärke

`docs/capabilities.md` ist bereits die richtige fachliche Mitte. Jede Zeile
enthält Disposition, Owner, Contract-Crate, UI-Crate, Storage/Integration,
Entry Point und aktuellen Implementierungsstand. Auch bewusst entfernte,
verschobene oder deferred Oberflächen sind dokumentiert.

Das ist für Labonairs Produkt besser als eine einfache Feature-Liste, weil
„Hosts“, „Themes“, „Settings“, „Notifications“ und „Command Palette“ jeweils
auch Ownership- und Surface-Fragen haben.

### 6.2 Was PhotoCraft zusätzlich nachweist

PhotoCraft macht die Funktionseinheit selbst maschinenlesbar: eine
`CommandSpec` trägt ID, Label, Menüpfad, Shortcut, Parameterbeschreibung,
Enablement und `run`. Engine, UI, CLI, Control Protocol und MCP verwenden
dieselbe Registry. Die Paritätsdatei beantwortet automatisch „ist dieser
Menüpunkt an einen Command angeschlossen?“. Die Scorecard ergänzt „ist das
Verhalten wirklich getestet, gemessen oder nur verdrahtet?“.

Für Labonair sollten nicht alle Aktionen in eine stringbasierte Remote-API
gezwungen werden. Aber jeder sichtbare, relevante Workflow sollte einen
maschinenlesbaren Descriptor besitzen.

### 6.3 Empfohlenes Capability-Schema

Als Quelle für eine generierte Capability-/Surface-Scorecard sollte jeder
Eintrag mindestens diese Felder haben:

| Feld | Zweck |
|---|---|
| `id` | stabiler Capability- oder Workflow-Identifier |
| `owner` / `crate` | eindeutige Ownership |
| `disposition` | keep, redesign, defer, remove |
| `entry_points` | Palette, global menu, panel, statusbar, standalone, MCP |
| `commands` | IDs, Kontext, Default-Binding, Enablement |
| `contracts` | öffentliche Typen, Traits und Events |
| `state` | Owner und Lifecycle-Zustände |
| `persistence` | Speicherort, Schema, Migration, Secret-Anteil |
| `settings` | Werte, Scope, Default und Consumer |
| `notifications` | passive Meldung, Detail, Action, Dedupe |
| `ui_components` | UI-kit-Primitives und feature-owned Views |
| `tests` | Unit, contract, integration, workflow, negative |
| `visual_evidence` | Fixture/Screenshot/Acceptance-State |
| `automation` | MCP/agent support and restrictions |
| `status` | target, partial, wired, tested, visually_verified, shipped |
| `last_verified` / `source_commit` | zeitliche Nachvollziehbarkeit |

Aus diesen Daten können `docs/capabilities.md`, `docs/surfaces.md` und ein
Agenten-Index erzeugt werden. Die aktuelle Capability-Matrix bleibt als
lesbare normative Sicht bestehen, erhält aber ein „Generated from“-Header,
falls sie tatsächlich generiert wird.

### 6.4 Workflow-Akzeptanz statt nur Crate-Akzeptanz

Labonairs Funktionsnachweis sollte nicht bei „Crate kompiliert“ enden. Für
jede Kernfähigkeit sollten wenige realistische Tasks definiert werden:

- lokales Projekt öffnen, Terminal und Editor starten;
- standalone Terminal ohne Projekt öffnen;
- Host auswählen, SSH verbinden, SFTP öffnen;
- Datei remote durchsuchen, übertragen, Fortschritt und Fehler sehen;
- Git-Diff öffnen und zur Editorstelle zurückkehren;
- Theme previewen und bestätigen, ohne Settings als Managementfläche zu
  verwenden;
- Keymap ändern, Konflikt erkennen und Command über Palette ausführen;
- Notification öffnen, Detail/Action ausführen und Dedupe prüfen;
- MCP-Grant erteilen, Command sichtbar im Terminal ausführen, Grant entziehen;
- App neu starten und Workspace-/Settings-/Session-Grenzen prüfen.

Diese Szenarien sollten als kleine workflow fixtures mit Status geführt werden:
`planned`, `contract`, `implemented`, `automated`, `visual_verified`,
`regression_locked`. Das ist die Labonair-Entsprechung zu PhotoCrafts
Scorecard und Agent-Tasks, ohne dessen Bildeditor-Anforderungen zu kopieren.

## 7. Command-, Menü- und Entry-Point-Organisation

### 7.1 PhotoCrafts Muster

PhotoCraft hat eine große Photoshop-Menüstruktur in
`crates/ui-egui/src/menu_catalog.rs`. Engine-Command-Specs werden aus vielen
fachlichen Modulen gesammelt. `menus.rs` bildet UI-Level-Commands und
plattformnahe Dialog-/Panel-Aktionen ab; `is_live` und `docs/parity.md`
prüfen, ob die Katalogeinträge einen ausführbaren Weg haben.

Das ist stark für ein Produkt, dessen erklärtes Ziel „Photoshop-Menü- und
Command-Parität“ ist. Es hat aber auch eine erkennbare Restkomplexität: Die
UI besitzt weiterhin eine große Dispatch-Funktion für UI-only Verhalten.
Labonair sollte daher das Prinzip der Descriptor-/Coverage-Prüfung übernehmen,
nicht eine zentrale UI-Dispatch-Funktion.

### 7.2 Labonairs Ziel und verbleibende Gefahr

Labonairs normative Verträge verlangen bereits:

- eine zentrale `CommandId`;
- owner-registrierte Command-Provider;
- typed dynamic submenu snapshots;
- Keymap-Auflösung über den Command-Vertrag;
- globale Navigation über Titlebar-Menü und Palette;
- keine parallelen Shell-Feature-Tabellen.

Das ist architektonisch die richtige Richtung. In der nativen Adapter-Schicht
existieren aber weiterhin GPUI-Action-Definitionen und Mapping-Registrierungen
in `crates/shell/src/menu.rs` und `crates/shell/src/commands.rs`. Diese sind
für das native Plattform-Adapterproblem teilweise legitim; sie dürfen jedoch
nicht erneut zu einer zweiten fachlichen Quelle werden.

### 7.3 Empfohlener Menu-/Surface-Katalog

Labonair braucht eine normative, aber möglichst generierte Datei wie
`docs/surface-catalog.md` oder `docs/menu-catalog.md`. Pro sichtbarem Eintrag
sollte sie zeigen:

| Information | Beispielhafte Frage |
|---|---|
| Surface | Titlebar, global menu, palette, workspace, dock, statusbar, overlay |
| Owner | Welche Capability registriert den Eintrag? |
| Stable ID | Welche `CommandId` ist kanonisch? |
| Label / path | Wie erscheint er im Menü oder Picker? |
| Context | Empty, standalone, project, terminal, editor, SSH, SFTP, modal? |
| Availability | Wann ist er aktiviert oder warum disabled? |
| Alternate entries | Welche anderen Wege delegieren auf denselben Command? |
| Keymap | Default, user override, conflict, unbound |
| Persistence | Ändert die Aktion Session, Settings, DB oder Remote State? |
| Notification | Welche Erfolg-/Fehler-/Action-Meldung entsteht? |
| Evidence | Contract test, workflow test, screenshot, manual acceptance |

Der Generator sollte mindestens folgende Fehler finden:

- doppelte sichtbare IDs oder Labels in einem Kontext;
- Menüeintrag ohne Owner oder ohne kanonischen Command;
- Palette-Eintrag ohne ausführbaren Handler;
- Default-Binding ohne Command oder Command ohne dokumentierten Kontext;
- Shell-eigene Feature-Action ohne ADR/Adapter-Markierung;
- registrierte, aber dauerhaft no-opende Aktionen;
- Surface ohne normal/narrow/focused/empty/loading/error-Evidenz;
- Settings- oder Notification-Duplikate außerhalb des Owners.

### 7.4 Konkreter Menüaufbau als Produktvertrag

Der Katalog sollte die bereits beschlossene Labonair-Struktur sichtbar machen,
ohne neue permanente Chrome zu erfinden:

```text
Titlebar
├── Tabs
└── ein globaler Menü-Button

Workspace
├── aktive Tabs
├── rekursive Split-Panes
└── Empty / Standalone / Project states

Docks
└── registrierte Panels aus Capability-Beiträgen

Statusbar
├── links: Panel-/Dock-Steuerung
└── rechts: globale Informationen, Transfers, Notifications

Overlay
├── Command Palette
├── Dialogs und focused pickers
└── Statusbar Notification Dropdown als einzige globale Meldungsfläche
```

Die konkreten Commands für Hosts, Themes, Icon Themes, Keymap, Settings,
Transfers und Notifications werden jeweils über den Owner-Katalog verlinkt.
Damit kann ein Agent den Menüaufbau verstehen, ohne `shell/src` durchsuchen
zu müssen.

## 8. Settings, Persistence und Datenlebenszyklus

### 8.1 Vergleich

Labonair hat mit `settings.md`, `settings-guidelines.md` und
`settings-inventory.md` ein deutlich besseres Ownership-Ziel als eine
klassische „alle Konfiguration in Settings“-Struktur. Hosts, Themes, Keymap,
Transfers und Notifications sind bewusst aus der Settings-Managementfläche
herausgenommen.

PhotoCraft ergänzt dazu einen automatisch ermittelten Audit für Einstellungen,
die kein Consumer liest. Das liefert eine konkrete Zahl für tote Preferences
und verhindert, dass ein Feld nur deshalb als fertig gilt, weil es im JSON
existiert.

### 8.2 Verbesserungen

Für Labonair sollte die Settings-Quelle folgende Informationen generieren oder
prüfen:

- Feld-ID und typisierter Datentyp;
- Default und zulässige Wertebereiche;
- User-/Project-/Workspace-Scope;
- Consumer-Crates und konkrete Consumer-Symbole;
- Speicherung und Migration;
- Secret-/Path-/Remote-Risiko;
- UI-Field und Hilfe-/Fehlertext;
- letzter Consumer-Test;
- Status `used`, `migration_only`, `deprecated`, `dead`.

Der Check sollte fehlschlagen, wenn:

- ein dokumentiertes Feld keinen Consumer und keine Migration besitzt;
- ein Consumer ein Feld liest, das nicht im Inventory steht;
- eine Managementfläche eines fremden Owners in Settings auftaucht;
- ein Default im Dokument und im Code auseinanderläuft;
- ein Projektwert eine globale oder geheime Einstellung unzulässig überschreibt.

Session-, Workspace-, SQLite-, Keyring-, Transfer- und Update-Daten sollten
zusätzlich in einem `docs/data-lifecycle.md` beschrieben werden. Für jeden
Datentyp gehören dort Owner, Speicherort, Lebensdauer, Migration,
Crash-/Corruption-Verhalten und Secret-Policy hinein.

## 9. Automation, MCP und Security

### 9.1 PhotoCrafts Vorsprung

PhotoCraft dokumentiert Automation als eigenes System:

- CLI und Headless-Ausführung;
- JSON-lines Control Protocol;
- Live-Bridge zur laufenden Desktop-App;
- MCP im Headless- und Bridge-Modus;
- Token, Loopback und Rooted Read/Write Permissions;
- Request-, Reply-, Connection-, Batch- und Preview-Limits;
- explizite Security-Lücken und zukünftige Hardening-Punkte;
- Agent-Workflow-Tests.

Das ist nicht nur API-Dokumentation, sondern ein Sicherheits- und
Betriebsvertrag.

### 9.2 Labonairs bestehender Implementierungsstand

Labonair hat einen substanziellen `labonair-mcp-server` mit Grants für
Terminaltabs, sichtbarer Command-Ausführung, Output-Lesen, Keystrokes,
Tab-Öffnen/-Schließen, Host-Blockierung und Activity Events. Diese Regeln
stehen derzeit vor allem im Quellcode und in Tool-Beschreibungen. In der
zentralen Dokumentationsnavigation fehlt eine gleichwertige Automation-
und Security-Handbuchstruktur.

Dadurch kann ein Agent zwar über MCP arbeiten, aber ein Dokumentationsagent
erkennt schwerer:

- welche Tools garantiert vorhanden sind;
- welche Aktionen nur bei explizitem Grant erlaubt sind;
- wie SSH-, Host-Key-, Secret- und SFTP-Grenzen wirken;
- welche Pfade und Payloads begrenzt werden;
- welche Fehler transient, user-actionable oder fatal sind;
- ob eine UI-Funktion auch automatisierbar ist.

### 9.3 Empfohlene Dokumente und Gates

Neue, nicht-normative oder normative Teilbereiche unter `docs/automation/`
und `docs/security/` sollten mindestens enthalten:

1. `overview.md` — Agenten- und Integrationsmodell;
2. `mcp.md` — Tool-Katalog, Parameter, Grant-Lifecycle, Beispiele;
3. `automation-security.md` — Token, loopback, capabilities, revocation,
   redaction, path/host policy;
4. `limits.md` — timeouts, output caps, connection/batch limits;
5. `test-matrix.md` — every documented tool has positive and negative tests;
6. `threat-model.md` — remote host, malicious terminal output, hostile
   config/database, compromised MCP client, secrets and updates.

Der Tool-Katalog sollte aus Rust-Metadaten oder Contract-Tests erzeugt werden.
Manuelle Beispiele dürfen ergänzt werden, sollen aber als Beispiel markiert
sein und nicht die einzige Quelle des API-Umfangs darstellen.

Zusätzliche sinnvolle Gates:

- kein Tool ohne stabile ID, Beschreibung, Parameter- und Fehlervertrag;
- jeder Grant kann widerrufen werden und wird bei Ausführung erneut geprüft;
- keine Secrets in Logs, Notifications, SQLite, Screenshots oder Testfixtures;
- negative Tests für fehlende Grants, blockierte Hosts, falsche Session-Typen,
  Zeitüberschreitungen, zu große Output- oder Batch-Payloads;
- Property-/Fuzz-Tests für JSON, Session-Snapshots, Host-Konfigurationen,
  Settings-Edits und MCP-Parameter;
- ein Audit-Event enthält nur redigierte Metadaten, nicht Terminalinhalte,
  Tokens oder Passwörter.

PhotoCrafts Security-Dokumente benennen bewusst auch nicht implementierte
Schutzmaßnahmen. Dieses Statusverhalten sollte Labonair übernehmen: Eine
Security-Idee wird als `Implemented`, `Known limitation`, `Proposed` oder
`Future hardening` gekennzeichnet und nie durch eine Designbeschreibung als
fertig ausgegeben.

## 10. Tests, Evidenz und Qualität

### 10.1 Was Labonair bereits hat

Labonair besitzt:

- umfangreiche crate-lokale Tests;
- Architektur- und Queue-Gates;
- einen detaillierten visuellen Verifikationsvertrag;
- `scripts/screenshot.sh` mit PID- und Binary-Prüfung gegen die alte Tauri-
  App;
- `testing-plan.md` mit Editor-, Workspace-, Settings-, Notification- und
  Fehlerzuständen;
- Performance-Dokumentation mit explizitem Hinweis, dass keine Messung als
  vorhanden ausgegeben werden darf, solange sie nicht gelaufen ist.

Die genaue Prozessidentifikation im Screenshot-Script ist für Labonair sogar
spezifischer als ein generischer Screenshot-Workflow, weil der alte und der
neue App-Bundle-Name kollidieren können.

### 10.2 Was PhotoCraft zusätzlich systematisiert

PhotoCraft trennt und automatisiert:

- normale Workspace-Tests;
- adversarial `panic_hunt` für Commands;
- reale Datei-Corpora mit Pins, Hashes und Floors;
- CPU/GPU-Oracle-Vergleiche;
- Performance-Budgets und Baselines;
- Paritätsgenerator;
- UI-Snapshots und Control-Channel-Screenshots;
- manuelle Fuzz-Ausführung und Crash-Artefakte;
- Scorecard-Check in CI.

### 10.3 Labonair-Scorecard

Labonair sollte eine eigene `quality/`- oder `scorecard/`-Quelle erhalten,
nicht PhotoCrafts Bildeditor-Kategorien kopieren. Sinnvolle Bereiche sind:

| Bereich | Mess-/Nachweisbeispiele |
|---|---|
| Architecture | unregistered crate, forbidden edge, transitional edge age, owner coverage |
| Commands | descriptor coverage, duplicate IDs, route equivalence, disabled reason |
| Surfaces | panel/status/palette/menu registration, canonical owner, visual states |
| Workspace | empty, standalone, project, SSH, SFTP, session restore |
| Settings | used/dead fields, scope, migration, schema diagnostics |
| Remote safety | grants, host blocking, auth failure, timeout, output limits |
| Notifications | error migration, deduplication, action routing, no-toast invariant |
| Reliability | no predictable unwrap, cancellation, corrupt persistence, panic boundary |
| Performance | startup, terminal throughput, editor latency, large directory, Git, SFTP transfer |
| Distribution | bundle structure, signature, update verification, supported targets |
| Documentation | links, authority metadata, source commit, generated outputs, stale claims |

Ein Scorecard-Eintrag sollte `id`, `target`, `status`, `owner`, `evidence`,
`command`, `last_verified`, `issue` und `note` besitzen. Die Statuswerte
`done`, `partial`, `missing`, `not_measurable`, `deferred` reichen aus. Ein
`partial`-Eintrag braucht immer eine Erklärung, was noch fehlt.

### 10.4 Performance

Labonairs `docs/performance.md` ist als Messmethode gut, aber die Werte werden
noch nicht so reproduzierbar als Baseline/Gate verwaltet wie bei PhotoCraft.
Empfehlung:

- `perf/budgets.toml` mit Szenarien und Zielwerten;
- `perf/baseline.json` mit Maschine, OS, Commit und Messbedingungen;
- `cargo xtask perf --quick` für lokale Regressionen;
- ein Nightly- oder manuell startbares Performance-Workflow;
- keine harten Gates für noch nicht messbare Szenarien, aber sichtbare
  `not_measurable`-Einträge;
- getrennte Budgets für GPUI-Foreground, I/O, Worker und End-to-End-Latenz.

Für Labonair wären als erste Szenarien sinnvoll: Cold Start bis interaktiv,
Terminal-Output-Throughput, Editor-Typing auf großer Datei, Explorer auf
5.000 Einträgen, Git-Status auf großem Repository, SFTP-Listing und Transfer,
MCP-Command-Latenz und Session-Restore.

## 11. CI-, Release- und Infrastrukturvergleich

### 11.1 Labonair aktuell

Die Labonair-CI enthält:

- Dependency-Verifier;
- Documentation-Governance- und Rework-Queue-Check;
- macOS mit fmt/check/clippy/test;
- Linux nur als informational `continue-on-error` Check;
- macOS-Release mit Bundle, Smoke-Test und optionalem Signieren/
  Notarisieren.

Das passt zum ausdrücklich macOS-first-Produktstatus, ist aber als
Qualitätsschranke schwächer als der Anspruch der Architekturverträge:

- Der Dependency-Gate ist aktuell rot.
- Linux kann rot sein, ohne den Workflow zu blockieren.
- Es gibt keinen mdBook-/Dokumentationsbuild.
- Es gibt keinen generierten Capability-/Menu-/Surface-/Scorecard-Check.
- Es gibt keinen WASM- oder zweiten Desktop-Plattform-Gate.
- Es gibt keine reguläre Corpus-/Fuzz-/Performance-Infrastruktur für die
  remote- und persistenzkritischen Grenzen.
- Release ist auf macOS begrenzt und hat weniger Artefakt-/Plattform-
  Verifikation als PhotoCraft.

### 11.2 PhotoCraft aktuell

PhotoCraft prüft in CI und Zusatzworkflows:

- Linux/macOS/Windows abhängig von Änderungsbereich;
- Tests mit und ohne optionale Features sowie mit eingebetteten Fonts;
- Layering via `cargo xtask layers`;
- Scorecard-Drift via `cargo xtask scorecard --check`;
- reale Corpus-Tests mit gepinnten Quellen und Hashes;
- WASM-Checks;
- manuell auslösbare Fuzz-Jobs mit Crash-Artefakten;
- mdBook-Build;
- Packaging-Lint;
- Performance-Nightly mit Baseline und Budgets;
- Multi-Plattform-Release für macOS, Windows, Linux, Flatpak, FreeBSD und
  Web mit Artefaktprüfung.

Auch hier gibt es offene Punkte — zum Beispiel nicht vorhandene
`cargo-audit`-/`cargo-deny`- und SBOM-Gates —, aber sie sind in der
Security-Dokumentation ausdrücklich als nicht implementiert markiert. Dieses
ehrliche Muster ist für Labonair wichtiger als eine möglichst lange CI-Liste.

### 11.3 Priorisierte CI-Verbesserungen für Labonair

**P0 — Verlässlichkeit der vorhandenen Ansprüche**

1. Die zwei Explorer-Kanten auflösen oder explizit als Migration registrieren.
2. Linux-Check aus `continue-on-error` herausnehmen, sobald die Plattform-
   abhängigkeiten stabil sind; bis dahin als bewusstes `informational` mit
   Issue und Frist dokumentieren.
3. Cargo-Kommandos in CI und Agentenhandbuch vereinheitlichen und, wo der
   Lockfile-Vertrag gilt, `--locked` verwenden.
4. Einen einzigen `cargo xtask ci`- oder `scripts/ci.sh`-Entry-Point schaffen,
   der die Reihenfolge und Zusammenfassung der Gates ausgibt.

**P1 — Drift und Evidence**

5. `docs --check`: lokale Links, Status/Version, Report-Index, orphaned docs,
   generated-file freshness und stale markers.
6. `architecture --check`: Cargo Metadata gegen deklarative Owner-/Edge-Quelle.
7. `capabilities --check`: jede Capability besitzt Owner, Entry Point und
   Status; jedes Produkt-Crate ist einer Capability oder Foundation-Rolle
   zugeordnet.
8. `surfaces --check`: jeder Command, Panel, Status-Item und Notification-
   Entry stammt aus einem Owner und hat eine gültige ID.
9. `scorecard --check`: generierte Qualitätsübersicht.
10. Dokumentationsbuild als eigener CI-Job; ein mdBook ist optional, aber die
    Navigation sollte CI-geschützt sein.

**P2 — Produktreife**

11. Cross-platform Builds auf Linux und später Windows nach Produktentscheidung.
12. Dependency-Advisory-, Lizenz- und optional SBOM-Prüfungen.
13. Parser-/Protocol-Fuzzing und reproduzierbare negative Fixtures.
14. Performance-Nightly und signierte, verifizierte Release-Artefakte.

## 12. Release, Betrieb und Wiederherstellung

PhotoCraft dokumentiert nicht nur das Bauen, sondern auch Installationsformate,
Portable Mode, Fonts, MIME-Registrierung, Signaturen, Notarisierung,
Gatekeeper, Flatpak-Sandbox, Web-Limits und die Trennung von Secrets im
Release-Environment.

Labonair sollte seine vorhandene `docs/RELEASE.md` in eine stärker
operationalisierte Struktur erweitern:

- `docs/release/overview.md`: unterstützte Ziele und Artefakte;
- `docs/release/build.md`: reproduzierbarer lokaler Build;
- `docs/release/signing.md`: Signatur-/Notarisierungs-/Update-Kette;
- `docs/release/verify.md`: Checks, die ein Nutzer-/CI-Artefakt bestehen muss;
- `docs/release/rollback.md`: fehlerhaftes Update, beschädigte Session,
  Settings-Recovery, Migration-Rollback;
- `docs/release/data.md`: welche lokalen Daten beim Deinstallieren bleiben;
- `docs/release/support-matrix.md`: tatsächlich getestete OS-/GPUI-/SSH-
  Kombinationen.

Der Update-Mechanismus sollte als Supply-Chain betrachtet werden: Manifest,
Signatur, Download, temporäre Datei, Hash-/Signaturprüfung, atomare
Installation und Rollback müssen in einem Ablaufdiagramm zusammengehören.

## 13. Konkreter Zielaufbau für die Dokumentation

Der folgende Aufbau nutzt Labonairs bestehende Autoritätsregeln und ergänzt
PhotoCrafts Auffindbarkeit und Evidence. Die Namen sind Vorschläge, keine
sofortige Verpflichtung, bevor sie in `docs/documentation-governance.md`
aufgenommen werden.

```text
AGENTS.md                         # bindende Kurzregeln
CLAUDE.md                         # kurzer Adapter, keine zweite Autorität
README.md                         # Produkt- und Start-Einstieg

docs/
├── README.md                     # Autoritäts- und Navigationsindex
├── agents/README.md              # Lesepfad und Change-Matrix für Agents
├── product.md                    # normativer Produktvertrag
├── architecture.md               # Laufzeit- und Layer-/Boundary-Regeln
├── repository-layout.md          # Platzierung
├── modules.md                    # Ownership-/Crate-Regeln
├── registries.md                 # typed registries
├── capabilities.md               # generierte/kanonische Capability-Sicht
├── surfaces.md                   # Surface-/Entry-Point-Katalog
├── menu.md                       # Menü-/Palette-/Keymap-Modell
├── workspace-model.md            # Project/Standalone/remote
├── settings.md                   # Settings-Werte und Scope
├── settings-inventory.md         # Consumer-/Migration-Audit
├── design-system.md              # Tokens und UI-kit
├── feature-lifecycle.md          # Boundary-first workflow
├── development.md                # build/run/debug/agent driving
├── testing.md                    # Testmatrix und Gates
├── performance.md                # Messmethodik
├── data-lifecycle.md             # DB, JSON, keyring, session, secrets
├── automation/
│   ├── overview.md
│   ├── mcp.md
│   ├── limits.md
│   └── test-matrix.md
├── security/
│   ├── overview.md
│   ├── threat-model.md
│   ├── trust-boundaries.md
│   └── vulnerability-reporting.md
├── release/
│   ├── overview.md
│   ├── verify.md
│   └── rollback.md
├── architecture/graph.toml      # maschinenlesbare Crate-/Owner-Quelle
├── scorecard/*.toml              # Evidence-Quellen
├── assets/                       # generierte Graphen und Screenshots
├── adr/                          # akzeptierte Entscheidungen
├── audits/                       # aktueller Ist-Stand
├── reports/                      # beratende Vergleiche/Forschung
└── archive/                      # superseded, nicht aktiv

tasks/rework/                     # einzige aktive Architekturqueue
memory/                           # Continuity und Lessons
scripts/ oder xtask/              # Checker und Generatoren
```

Eine neue Datei erhält vor dem Commit eine Klasse, einen Owner, eine Quelle
und eine Ablageentscheidung. Der Report-Index bleibt bewusst beratend und
darf keinen normativen Text wiederholen.

## 14. Empfohlene Umsetzungsreihenfolge

Die Reihenfolge respektiert Labonairs aktive Queue. Sie soll R07-001 nicht
umgehen und keinen späteren Rework-Task vorzeitig starten.

### Phase A — P0: Vertrauensbasis schließen

1. `panel-explorer → sftp/ssh` fachlich auflösen oder als dokumentierte,
   befristete Boundary-Migration einordnen.
2. `R07-001` um den vollständigen Dependency-/Documentation-/Surface-
   Acceptance-Nachweis ergänzen, falls die Kante in diesen Scope gehört.
3. Agenten-Lesepfad und Change-Matrix als `docs/agents/README.md` anlegen.
4. `docs/development.md` und `docs/testing.md` als praktische, nicht
   konkurrierende Einstiegspunkte erstellen oder die Informationen in
   vorhandenen Verträgen verlinken.
5. CI-Command-Sammlung vereinheitlichen und die tatsächlichen Statuswerte
   (`required`, `informational`, `blocked`, `not-measurable`) sichtbar machen.

### Phase B — P1: Maschinenlesbare Organisation

6. Architekturmanifest plus Generator/Verifier für Owner, Rolle, Contract und
   erlaubte Kanten.
7. Capability-Descriptor und generierte Capability-/Surface-Sicht.
8. Menu-/Command-Katalog aus `CommandId`-/Provider-Metadaten.
9. Settings-Consumer-/Dead-Field-Audit.
10. Scorecard-Quellen und generierte `docs/scorecard.md`.
11. `reports/README.md` um einen datierten Index mit Source-Commit,
    Scope, Status und „not authority“-Hinweis erweitern.

### Phase C — P1/P2: Funktionale Nachweise

12. Kern-Workflow-Suite für Terminal, Workspace, SSH, SFTP, Transfers, Editor,
    Git, Settings, Keymap, Themes, Notifications und MCP.
13. Automation-/Security-Handbuch und negative Grant-/Path-/Secret-Tests.
14. Visual Surface Matrix mit nativen Screenshots und Artefaktverweisen.
15. Performance-Budgets und Baseline für die Labonair-spezifischen Hotpaths.
16. Fuzz-/Property-Tests für Settings, Sessions, Hostdaten und MCP-Payloads.

### Phase D — Produktreife nach Entscheidung

17. Linux-/Windows-Support nur mit dokumentiertem Support-Matrix- und
    Release-Gate.
18. Release-Signatur, Update-Verification, Rollback und Recovery-Evidenz.
19. Optionales mdBook oder gleichwertige statische Navigation, wenn die
    Dokumentationsmenge die vorhandene Markdown-Navigation übersteigt.

## 15. Was ausdrücklich nicht übernommen werden sollte

1. **Keine PhotoCraft-Menüstruktur:** Labonair braucht keine zehn Photoshop-
   Hauptmenüs oder 627 Menü-Paritätsitems.
2. **Keine zentrale String-Dispatch-Gottklasse:** PhotoCrafts UI-only
   Dispatch ist eine verständliche Restlösung für sein Produkt; Labonairs
   typed owner registry ist für die gewünschte Modularchitektur passender.
3. **Keine Bildeditor-Scorecard:** Nur das Datenformat — ID, Ziel, Status,
   Evidenz, Owner, Befehl, Datum — ist übertragbar.
4. **Keine WASM-/Web-Zielpflicht:** PhotoCrafts Web-Build folgt seinem Produkt;
   Labonair bleibt nativ, solange kein Produktentscheid etwas anderes sagt.
5. **Keine automatische Ausweitung des Funktionsumfangs:** PhotoCraft zeigt
   viele Paritätslücken; daraus folgt nicht, dass Labonair Marketplace,
   Remote-Themes, Extension Hosting oder neue permanente Chrome-Flächen
   braucht.
6. **Keine Vermischung von Target und Ist:** Labonairs getrennte normative
   Verträge und Audits sind hier klarer als PhotoCrafts teilweise gemischte
   Architektur-Narrative.

## 16. Abschlussbewertung nach Priorität

| Priorität | Befund | Empfohlene Entscheidung |
|---|---|---|
| P0 | Dependency-Gate von Labonair rot an zwei Explorer-Kanten | Sofort innerhalb der aktiven Acceptance klären; keine neue Featurearbeit darauf aufbauen. |
| P0 | Agenten kennen Regeln, aber nicht immer den kürzesten Lesepfad | `docs/agents/README.md` mit Change-Matrix und Abschluss-Gates ergänzen. |
| P0 | CI prüft Dokumentlinks, aber nicht Capability-/Menu-/Surface-Drift | Kleine, deterministische Generator-/Check-Schicht ergänzen. |
| P1 | Capability-Matrix ist lesbar, aber überwiegend manuell | Descriptoren/Scorecard-Quellen einführen und Ansichten generieren. |
| P1 | Menu-/Entry-Point-Vertrag ist normativ beschrieben, aber nicht als vollständiger Katalog sichtbar | `menu.md`/`surfaces.md` mit Owner, ID, Kontext und Evidenz erzeugen. |
| P1 | MCP/Remote-/Secret-Verhalten ist implementiert, aber nicht als vollständiges Handbuch auffindbar | Automation- und Security-Dokumente samt Negativtests ergänzen. |
| P1 | Visuelle Regeln sind stark, Evidence bleibt task-/manuell gebunden | Surface-Fixtures, Screenshot-Index und Zustandsmatrix standardisieren. |
| P2 | Performance ist methodisch dokumentiert, aber nicht als Budget-/Baseline-Gate | PhotoCrafts Scorecard-/Baseline-Idee produktgerecht übernehmen. |
| P2 | Release- und Support-Matrix ist macOS-lastig | Plattformen, Signaturen, Recovery und Supportgrenzen explizit machen. |
| P2 | PhotoCraft hat selbst sichtbaren Roadmap-/Paritätsdrift | Generated-vs-narrative Statusklassen in Labonair verbindlich machen. |

## 17. Schluss

Labonair braucht keinen Architektur-Neustart. Der wichtigste Teil — Owner,
Contracts, Registries, normative Verträge, Audits und eine geordnete
Migrationsqueue — ist bereits besser vorbereitet als bei vielen vergleichbaren
Rust-Anwendungen. Der nächste Reifegrad ist die Übersetzung dieses Wissens in
reproduzierbare Evidence:

```text
Normativer Vertrag
        ↓
Owner-/Boundary-Descriptor
        ↓
Command-/Surface-/Capability-Registrierung
        ↓
generierte Dokumentation und Scorecard
        ↓
Contract-, Workflow-, Visual-, Security- und Performance-Evidence
        ↓
CI-Gate und datierter Handoff
```

Wenn diese Kette geschlossen ist, können Menschen und AI-Agenten gleichzeitig
beantworten: Was soll existieren? Wem gehört es? Wo ist der Einstieg? Welche
Abhängigkeiten sind erlaubt? Was ist wirklich implementiert? Wie wurde es
geprüft? Und welcher nächste Task ist nachweisbar der richtige?
