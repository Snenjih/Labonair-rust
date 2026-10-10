# Zed-Editor ↔ Labonair: Funktionsvergleich

**Stand:** 10. Oktober 2026 · **Status:** laufende Bestandsaufnahme, noch nicht vollständig

**Zed-Baseline:** `zed-refrence/zed` bei `3569541038dd51524b03998ba4d38d253cb54f80`

**Erste Abnahmeplattform:** macOS
**Labonair-Owner:** Editor-Kern `labonair-editor`; GPUI-Oberfläche aktuell in `labonair-workspace`; globale Tabs/Splits im Workspace; Editor-Einstellungen in `labonair-settings`.

Dieser Bericht vergleicht sichtbare Editor-Funktionen und deren Nutzerabläufe. Er beschreibt Zed unabhängig und enthält keine übernommenen Zed-Implementierungen. Zed-Quelldateien wurden nur als Forschungsquelle für die festgehaltene Baseline gelesen. Implementierungsarbeiten dürfen sich auf die unabhängig formulierten Labonair-Verträge und die Host-Codebasis stützen, nicht auf die Zed-Quelldetails.

## Aussagekraft und Statusbegriffe

Der lokale Zed-Checkout stimmt mit dem oben genannten Commit überein. Sein Quellbaum und Labonairs Quellbaum wurden untersucht. Die aktuelle öffentliche Zed-Dokumentation wurde ergänzend am 10.10.2026 geprüft; sie kann neuer als die gepinnte Referenz sein und wird deshalb nicht automatisch zum Zielstand. Weder ein Laufzeitvergleich der gepinnten Zed-Version noch ein gepaarter nativer UI-Aufnahmevergleich ist in dieser Bestandsaufnahme belegt. **Quellcode-Präsenz beweist noch keine vollständige Bedienbarkeit oder visuelle Gleichwertigkeit.**

| Kennzeichnung | Bedeutung |
|---|---|
| **Vorhanden im Code** | Labonair enthält passenden Modell- oder UI-Code. Laufzeit und visuelle Gleichwertigkeit bleiben offen. |
| **Teilweise** | Eine Untermenge existiert; Zeds Umfang, Integration, Zustände oder Verhalten fehlen beziehungsweise sind nicht belegt. |
| **Lücke** | In den untersuchten Labonair-Ownern wurde kein gleichwertiger Ablauf gefunden. |
| **Nicht belegt** | Die Recherche reicht nicht für eine belastbare Einordnung. |
| **Zed-Quelle** | `P` = gepinnter Zed-Quellbaum geprüft; `D` = aktuelle öffentliche Zed-Dokumentation. Ein `D`-Link allein beweist keine Übereinstimmung mit dem gepinnten Commit. |

Labonair-Pfade in Tabellen zeigen die untersuchten Belege. Ein Feature mit vorhandenem Einstellungsfeld, aber ohne bestätigte Anwendung im Editor, ist nicht als umgesetzt gewertet. Die Zuordnung **Lücke** bedeutet „im aktuellen Quellstand nicht gefunden“, nicht „vom Produkt ausgeschlossen“.

## 1. Editorrahmen, Tabs, Kopfzeile und Status

Zed trennt Fenster-Titelzeile, Pane-Tab-Leiste, Editor-Toolbar und Statusleiste. Die Toolbar liegt unter den Tabs. Die folgenden Toolbar-Elemente lassen sich getrennt anzeigen: Breadcrumbs, Quick Actions, Selections Menu, Agent Review und Code Actions. Die Tab-Leiste besitzt eigene Sichtbarkeits- und Navigationsoptionen. [Zed Editor- und Tab-Einstellungen](https://zed.dev/docs/reference/all-settings) · [Visuelle Editor-Anpassung](https://zed.dev/docs/visual-customization)

Der bereitgestellte, eng zugeschnittene Tab-Bar-Screenshot zeigt zwei Navigationspfeile und vier Tabs (`untitled`, `Cargo.lock`, `CLAUDE.md`, `lib.rs`); `lib.rs` ist kursiv und aktiv dargestellt. In diesem Bild sind keine Dateiglyphen oder Close-Schaltflächen sichtbar. Fenstergröße, Theme, Skalierung und Pointerposition sind nicht dokumentiert; das Einzelbild belegt deshalb nur diesen Zustand, keine Laufzeit- oder Paritätsabnahme.

Der aktuelle Worktree enthält uncommittete Änderungen an Shell-Titelzeile, Workspace-Tabstrip und UI-Kit-Tabdarstellung. Die folgenden Labonair-Einträge beziehen sich auf diesen Quellstand, nicht allein auf den letzten Commit. Die Codeänderungen belegen noch keine gerenderte Gleichheit; die gepaarten nativen UI-Zustände sind weiterhin Pending.

| Zed-Funktion | Labonair-Gegenstück und Status | Nutzen | Verbundene Systeme und Verhalten |
|---|---|---|---|
| Fenster-Titelzeile mit Projekt-/Fensteraktionen | **Teilweise.** Die Labonair-Titelzeile ist globale Shell-Fläche, kein Editor-Owner; im aktuellen Worktree ist sie 32 px hoch und hält Tabstrip, Space-/Neutabwahl sowie genau ein globales Menü auseinander. `crates/shell/src/titlebar.rs`, `crates/workspace/src/workspace.rs`. | Projekt- und Fensteraktionen ohne Platz im Textbereich zu verbrauchen. | App-Fenster, globales Menü, Workspace, OS-Titelleiste. Dies ist von der Editor-Toolbar zu unterscheiden. |
| Tab-Leiste je Pane mit aktiver Editor-Datei | **Teilweise, bewusst anders angeordnet.** Labonair verwendet die gemeinsame Workspace-Tab-Leiste für Editor, Terminal, Diff, Vorschau und weitere View-Arten; siehe `crates/workspace/src/tabs.rs`, `crates/workspace/src/pane.rs`. | Zwischen Dateien und anderen Werkzeugen wechseln und den Editor-Kontext pro Pane sehen. | Workspace besitzt Tab-Identität, Aktivierung, Dirty-/Busy-Status und Schließen. Die universelle Labonair-Tab-Leiste ist eine dokumentierte Produktentscheidung, keine pixelgenaue Zed-Chrome-Kopie. |
| Tab-Chrome: inhaltsbasierte Breite, 32-px-Leiste, schmale Trenner, aktiver Tab schließt bündig an den Editor an; horizontales Scrollen ohne Tab-Kompression | **Im aktuellen Worktree visuell angeglichen, Runtime offen.** Labonair setzt Titelbar und horizontalen Tabstrip auf 32 px, nutzt flache Tabs mit Nullabstand, Theme-Fläche, 1-px-Trennern, bis zu 240 px Tabbreite und nicht schrumpfenden Scrollinhalten. Der aktive Tab nutzt die Workspace-Hintergrundfläche. `crates/shell/src/titlebar.rs`, `crates/workspace/src/workspace.rs`, `crates/ui-kit/src/tab.rs`, `crates/ui-kit/src/palette.rs`. | Aktive Datei schneller erkennen und bei vielen offenen Dateien navigieren, ohne Titel auf gleich schmale Spalten zu pressen. | Zed-Tab-Bar- und Tab-Item-Chrome; Labonair Shell-Titelzeile, Theme-/Density-Tokens, UI-Kit Tab-Komponente und `overflow_x_scroll`. Source-Layout passt in den untersuchten Parametern; gepaarter nativer Bildvergleich fehlt. |
| Kurzlebiger Preview-Tab aus dem Projektbaum; beim nächsten Öffnen ersetzt, durch Pin/Doppelklick dauerhaft | **Teilweise.** Labonair besitzt Preview-Tabs, aber der konkrete Projektbaum-Workflow und alle Promote-/Pin-Zustände sind hier nicht als Zed-gleichwertig belegt. | Dateien rasch ansehen, ohne für jede Inspektion einen permanenten Tab anzulegen. | Project Panel, Workspace-Tab-Lifecycle, Dirty-Status. Zeds Details siehe [Preview Tabs](https://zed.dev/docs/reference/all-settings) und [Project Panel](https://zed.dev/docs/project-panel). |
| Tab-Aktionen und Tab-Zustand: Schließen, Aktivierung nach Close, Dateisymbol, Git- und Diagnosefarbe, Close-Button nur bei Hover | **Teilweise.** Horizontal ausgeblendetes Leading-Glyph für Editor-Tabs; andere View-Arten behalten ihr Glyph. Dirty-/Peek-Zustand ist sichtbar, das Close-X erscheint bei Hover und hat Tooltip/Keyboard-Aktivierung. Zed-artige Konfiguration von Close-Position, Icons, Statusfarben und Aktivierung nach Close ist nicht belegt. | Geöffnete Datei, Änderung und Fehler schnell erkennen; aktive Datei beim Schließen sinnvoll auswählen. | Tab-Owner, Git, Diagnostik, Icon-Theme; Zed-Optionen u. a. `tabs.close_position`, `show_close_button`, `activate_on_close`, `file_icons`, `git_status`, `show_diagnostics`. |
| Tab-Klick, Doppelklick/Preview-Promotion, Mittelklick, Drag/Reorder und Rechtsklickmenü | **Teilweise.** Labonair aktiviert per Klick, Enter oder Space, schließt per X/Mittelklick, reordert per Drag und bietet Rename/Duplicate/Keep Tab Open sowie Close/Close Others/Left/Right/Clean/All/by kind. Die neuen Batch-Schließpfade sind auf die Space des angeklickten Tabs begrenzt und sequenzieren Dirty-Bestätigungen. Ein Doppelklick-Promote ist nicht belegt. | Tabs schnell aktivieren, sortieren, temporäre Tabs behalten und mehrere Tabs mit passenden Dirty-Prüfungen schließen. | Workspace-Tab-Lifecycle, Preview/Dirty-State, Drag-Reorder, Context Menu, Space-Zuordnung und Close-Confirmation-Queue. Codepfade geändert; tatsächlicher Pointer-/Tastaturablauf noch nicht visuell bestätigt. |
| Gepinnte Tabs und Read-Only-Status | **Andere Semantik.** Labonair hat `Keep Tab Open` für Preview-Tabs, aber keinen separaten Pin-Bereich oder Pin-/Unpin-Status. Read-only-Dateien werden im Editorstatus erkannt; im Tabmenü ist kein Umschalten zwischen schreibgeschützt und editierbar belegt. | Gepinnte Dateien dauerhaft sichtbar halten und das versehentliche Schließen verhindern; schreibgeschützte Tabs gezielt gegen Eingaben sperren oder wieder freigeben. | Zed bietet `Pin Tab`/`Unpin Tab` im Rechtsklickmenü und Pin statt Close als Tab-Endaktion; ein Mittelklick schließt keinen gepinnten Tab. `Make Tab Read-Only`/`Make Tab Editable` erscheint nur, wenn der Item-Typ die Umschaltung erlaubt. Labonairs Preview-Promotion und unveränderlicher Dateizustand erfüllen andere Zwecke. `zed-refrence/zed/crates/workspace/src/pane.rs`, `crates/workspace/src/workspace.rs`, `crates/editor/src/lifecycle.rs`. |
| Dateibezogene Tab-Kontextaktionen | **Nicht im aktuellen Workspace-Tabmenü belegt.** Das Labonair-Menü bietet generische Tab-Aktionen; Pfadkopie, Project-Panel-Reveal, Datei-Manager-Reveal, Git-Permalink oder Terminal im Verzeichnis der Datei sind dort nicht vorhanden. | Pfade teilen, Dateien im Projektbaum/Finder lokalisieren, einen stabilen Git-Link erzeugen oder direkt aus dem Datei-Kontext im passenden Verzeichnis arbeiten. | Zed zeigt diese Aktionen nur für passende file-backed Items und Fähigkeiten: Copy Path; Copy Relative Path bei verfügbarem Worktree-Pfad; Open/Copy File Permalink bei Git-Repository; Reveal In Project Panel bei sichtbarem Worktree; lokales Reveal In File Manager und Open in Terminal im Elternverzeichnis. Items können eigene weitere Aktionen beisteuern. `zed-refrence/zed/crates/workspace/src/pane.rs`, `crates/workspace/src/workspace.rs`. |
| Pane-Navigationshistorie und Ein-/Ausblenden der Tab-Bar-Controls | **Sichtbare Controls hinzugefügt; andere Semantik.** Im aktuellen Worktree stehen Previous/Next-Pfeile vor dem Tabstrip und sind bei höchstens einem Tab im aktiven Space deaktiviert. Sie traversieren Nachbartabs (`cycle`), während Zeds Back/Forward die zwei History-Stacks des fokussierten Panes navigieren und unabhängig leer sein können. Für Labonair ist kein eigener Sichtbarkeitsschalter im Editor-Settings-Typ belegt. | Zed-Pfeile rufen besuchte Editororte zurück/vor; Labonair-Pfeile wechseln aktiv zwischen Workspace-Tabs. | Workspace-Tab-Traversal ist nicht pane-lokale Cursor-/Navigationshistorie; Zed `tab_bar.show_nav_history_buttons`, `tab_bar.show_tab_bar_buttons`. Keine gleichwertige Historyfunktion im Labonair-Tabstrip belegt. |
| Editor-Toolbar ein-/ausblendbar und nur bei sichtbaren Elementen gerendert | **Teilweise.** Labonair zeichnet eine Editor-Breadcrumb-Zeile, aber keine belegte funktionsgleiche Toolbar mit einzeln schaltbaren Elementen. | Vertikalen Platz freigeben und Aktionen nahe am Editor halten. | Zed `toolbar.breadcrumbs`, `quick_actions`, `selections_menu`, `agent_review`, `code_actions`. |
| Breadcrumbs aus Dateipfad und Syntax-/Symbolkontext; Klick öffnet Navigation | **Vorhanden im Code, einfacher.** Labonair zeigt Pfad und das Symbol unter dem Cursor in `crates/workspace/src/views/editor.rs` und `crates/editor/src/breadcrumbs.rs`. | Orientierung in verschachtelten Projekten und großen Dateien. | Datei-Lifecycle, Dokument-Symbole, Tree-sitter/LSP, Go-to-Line. Labonair belegt hier Pfad plus aktuelles Symbol, keine gesamte Zed-Menü- und Dropdown-Interaktion. |
| Quick Actions in der Toolbar, kontextabhängige Editor-Aktionen | **Teilweise.** Viele Aktionen sind als Command-Palette-Commands verfügbar; ein passender Toolbar-Bereich ist nicht belegt. | Häufige Aktionen mit Sichtbarkeit und Kontext platzieren. | Command Registry, Editor, Language Server, Git und Toolbar-Einstellungen. |
| Selections Menu: Mehrfachauswahl- und Auswahlaktionen in der Toolbar | **Lücke in der Toolbar; teilweise im Command-System.** Labonair hat Mehrfachauswahl und einzelne Befehle, aber kein entsprechendes Menü belegt. | Auswahlbefehle auffindbar machen, ohne alle Shortcuts zu kennen. | Mehrfachauswahlmodell, Editor-Commands, Keymap und Toolbar. |
| Inline-Code-Action-Schaltfläche bzw. Code-Actions-Schalter in der Toolbar | **Teilweise.** Code-Action-Requests und eine Ergebnis-UI sind im Editor vorhanden; derselbe Gutter-/Toolbar-Einstieg ist nicht belegt. | Schnellreparaturen und Refactorings direkt an der betroffenen Zeile finden. | LSP, Diagnostics, Gutter, Toolbar- und Inline-Code-Actions-Einstellungen. |
| Agent Review-Schaltflächen in Editor-Toolbar | **Lücke beziehungsweise nicht belegt.** | KI-generierte Änderungen lokal im Editor prüfen und zum Agenten zurückgeben. | Agent-Panel, Review-Markierungen, Diff und Editor-Toolbar. Agentenfunktionen sind ein eigener großer Folgeabschnitt, nicht durch normales LSP abgedeckt. |
| Editor-Statusleiste mit Sprache, Cursorposition, Zeilenende, Encoding, LSP-Zustand und mehrstufiger Shortcut-Anzeige | **Teilweise.** Labonair zeigt Cursorposition, optional Auswahlstatistik, Sprache und Zeilenzahl in einer 24-px-Editorzeile. `crates/workspace/src/views/editor.rs`; `EditorContent`-Felder in `crates/settings-content/src/editor.rs`. Zeilenende-/Encoding-Auswahl und ausstehende Tastenanzeige fehlen dort. | Aktuelle Datei-/Cursorparameter lesen oder direkt ändern; auch in komplexen Eingabezuständen Orientierung behalten. | Sprache-/Zeilenenden-/Encoding-Selektoren, LSP-Status, Keymap-Runtime, Settings, globaler Statusbar. Zed-Optionen siehe `status_bar.*`, `global_lsp_settings.button`. |
| Lokaler Language-Service-Status | **Vorhanden als Banner, nicht als Statusleisten-Menü.** `EditorView::render_language_service_status` zeigt Start, laufenden Prozess, Absturz/Fallback und nicht unterstützte Sprache. | Serverstatus und Fallback-Verhalten unmittelbar erkennen. | Explizit injizierter lokaler LSP-Prozess, Runtime-Status und Editor-Banner; kein gleichwertiger Statusbar-Schalter/Service-Menü belegt. |
| Editor-spezifische Menü- und Rechtsklickaktionen | **Teilweise.** Labonair enthält Text- und Git-Gutter-Kontextmenüs in `crates/workspace/src/views/editor.rs`; Aktionsabdeckung und Menüzustände sind nicht vollständig gegen Zed inventarisiert. | Aktionen am Cursor und an Änderungen erreichen, ohne die Auswahl zu verlieren. | UI-Kit-Menüs, Editor-Commands, Git, Language Server; Fokus- und Tastaturbedienung muss je Menü geprüft werden. |
| Leerer, ladender, fehlender und fehlerhafter Editorzustand | **Labonair-Zustände im Code detailliert; UI-Parität offen.** Zed-Quellbelege für New/Present/Deleted/Historic, ReadOnly und fehlgeschlagenes Öffnen stehen im Lifecycle-Audit unter Abschnitt 2; die genaue gepinnte Leer-/Fehlerdarstellung muss noch zur Laufzeit geprüft werden. Labonair zeigt Empty-/Loading-/Missing-/Error-/Recovery-Meldungen in `crates/editor/src/lifecycle.rs` und `crates/workspace/src/views/editor.rs`. | Eindeutige nächste Schritte, wenn noch kein Inhalt da ist oder eine Datei nicht verfügbar ist. | Filesystem, Dateirechte, externe Änderungen, Speichern/Wiederherstellen, Benachrichtigungen. `zed-refrence/zed/crates/language/src/buffer.rs`, `crates/workspace/src/invalid_item_view.rs`. |
| Projektbaum-Auswahl öffnet Datei nur vorläufig; Tippen pinnt Editor-Tab | **Teilweise.** Labonair hat Preview-View und Tabs, aber die Zustandsübergänge aus Zeds Projektbaum sind nicht bestätigt. | Erst inspizieren und nur bei Bearbeitung den permanenten Workspace-Platz verwenden. | Project Panel, Tab-Owner, Dirty-Übergang und Save Policy. |
| Kontextabhängiger Focus und Tastaturpfade zwischen Toolbar, Editor und Popup | **Nicht belegt als vollständiger Paritätsablauf.** Labonair nutzt UI-Kit-Fokuskomponenten, aber kein gepaarter Focus- und Tastaturpfad wurde hier aufgezeichnet. | Editor ohne Maus bedienen und nach Popups an die erwartete Stelle zurückkehren. | GPUI focus handles, UI kit, Keymap-Kontexte, Menüs/Dialogs, Screenreader/IME. |

### Editor-Toolbar: Quick Actions, Menüs und bedingte Beiträge

Zeds Editor-Toolbar wird pro Pane aus Beiträgen zusammengesetzt. Beiträge können links, rechts, in einem sekundären Bereich oder gar nicht erscheinen; wenn alle Beiträge verborgen sind, rendert Zed keine Toolbar-Zeile. Die Quick-Action-Leiste selbst liegt rechts und folgt dem aktiven Pane-Inhalt. Ihr globaler Schalter `toolbar.quick_actions` blendet die Leiste aus, während eigene Schalter und Fähigkeiten weitere Controls unabhängig steuern. Neben dieser allgemeinen Leiste registrieren Diagnostik-, Git-Diff-, Agent-Review-, Such-, Sprachwerkzeug- und Preview-Owner eigene Beiträge (`zed-refrence/zed/crates/workspace/src/toolbar.rs`, `zed-refrence/zed/crates/zed/src/zed.rs`).

| Zed-Control | Zweck und Anzeige-/Interaktionsbedingungen | Labonair-Gegenstück und Unterschied |
|---|---|---|
| Breadcrumbs (`toolbar.breadcrumbs=true`) | Zeigt die vom aktiven Pane-Item gelieferten Pfad-/Symbolsegmente; lange Hierarchien werden auf höchstens zwölf Segmente mit mittigem `…` gekürzt und horizontal scrollbar gehalten. Bei einem Editor öffnet Klick auf die Breadcrumb-Fläche die Symbol Outline; der Tooltip nennt die Aktion und den Shortcut. Rechtsklick kopiert bei einem Projektpfad den absoluten Dateipfad. `ToggleBreadcrumb` kann die Anzeige für den Editor umschalten, ohne den globalen Settingswert zu ändern; Updates der Breadcrumb-Daten kommen ereignisbasiert vom aktiven Item. [Zed-Quellen: Breadcrumb-Owner](zed-refrence/zed/crates/breadcrumbs/src/breadcrumbs.rs), [Editor-Renderer](zed-refrence/zed/crates/editor/src/element.rs). | Labonair rendert derzeit eine 30-px-Zeile in `crates/workspace/src/views/editor.rs`: leerer Puffer, Laden, fehlende Datei und Ladefehler erhalten eigene Beschriftungen; im bereiten Zustand erscheinen ein bis zu 96 Zeichen gekürzter Pfad und das Symbol an der Cursorzeile. Der Symbol-Eintrag springt zur Zeile, der Pfad zeigt den vollständigen Pfad als Tooltip. Pfad und Symbolhierarchie sind keine gemeinsamen Breadcrumb-Outline-Aktion; Kopieren per Rechtsklick und ein Toolbar-Sichtbarkeitsschalter sind im untersuchten Editor-Settingspfad nicht belegt. `crates/editor/src/breadcrumbs.rs`. |
| Buffer Search | Öffnet die Suche im aktiven Einzelpuffer und zeigt den aktiven Zustand als Toggle-Button. Die Leiste erscheint nur bei einem unterstützten Editor-Item; Suchfeld, Trefferzustand und Toolbar-Knopf gehören zusammen. | Labonair hat Buffer- und Projektsuche sowie Suchaktionen, aber keine belegte Suchleiste als Beitrag einer Zed-artigen Editor-Toolbar. `crates/workspace/src/search_overlay.rs`, `crates/workspace/src/workspace.rs`, `crates/editor/src/search.rs`. |
| Preview | Bei Markdown-, SVG- und tabellarischen Daten-Dateien wird ein Eye-Button zum Öffnen der passenden Vorschau angeboten. Alt-Klick öffnet die Vorschau im benachbarten Pane. Nicht unterstützte oder andere Inhalte erhalten diesen Beitrag nicht. | Labonair kennt Preview-Tabs und Vorschauansichten, aber dieselbe kontextuelle Schaltfläche mit belegtem Alt-Klick-/Pane-Verhalten ist nicht nachgewiesen. Die übrige Preview-Tab-Zuordnung steht in der Tabelle oben. |
| REPL / Kernel | Nur wenn Jupyter-REPL aktiviert ist, der Editor zu einem lokalen Projekt gehört und die Sprache eine Session unterstützt. Je nach Zustand öffnet der Knopf Setup oder Kernel-Auswahl; bei aktiver Session zeigt das Menü Kernel-/Sprachname, Status, Run Selection/Run Line, Interrupt, Clear Outputs sowie Session-Aktionen. | Labonairs Terminal und Prozess-Owner sind nicht dasselbe wie ein editorgebundener Notebook-Kernel-/REPL-Workflow. Ein entsprechender Editor-Toolbar-Beitrag ist nicht belegt. |
| Inline Assist | Startet Zeds Inline-Assistant aus dem Editor; der Button erscheint nur, wenn Agent aktiviert und der Agent-Button in den AI-Settings eingeschaltet ist. | Labonairs AI-/Agent-Oberflächen sind separat zu prüfen; eine belegte entsprechende Inline-Assist-Schaltfläche im Editor ist im aktuellen Worktree nicht gefunden. |
| Selections Menu (`toolbar.selections_menu=true`) | Das Cursor-/Auswahlmenü enthält Select All, Select Next Occurrence, Expand/Shrink Selection, Cursor Above/Below, Go to Symbol, Go to Line/Column, Next/Previous Problem, Next/Previous Hunk sowie Move Line Up/Down und Duplicate Selection. „Add to Agent Thread“ fehlt bei deaktivierter AI und ist ohne nichtleere Auswahl deaktiviert; Hunk-Navigation ist ohne Diff-Hunks deaktiviert. | Labonair enthält Auswahl- und Navigationsbefehle, aber keine entsprechende gruppierte Toolbar-Popover-Fläche. Befehle einzeln im Command-System bedeuten keine Gleichheit von Auffindbarkeit, Verfügbarkeitshinweisen oder Menüinhalt. |
| Editor Controls (`Filter`-Menü in Quick Actions) | Fähigkeiten des aktiven Editors bestimmen Einträge: Inlay Hints/Inline Values nur bei Inlay-Hint-Support, Semantic Highlights bei Semantic Tokens, Code Lens bei Support, Minimap bei Unterstützung und Edit Predictions nur bei Provider. Weitere Toggles schalten Diagnostics, Inline Diagnostics, Line Numbers, Selection Menu, Auto Signature Help, Inline/Column Git Blame und Diff Against Default Branch. Vim und Helix sind alternative Modi; Wechsel schaltet den jeweils anderen Modus aus. | Labonair hat Einstellungen und einzelne Aktionen für Teile dieser Features, aber kein entsprechendes zentrales, fähigkeitsabhängiges Editor-Controls-Popover. Pro-Einstellung vorhandene Controls sind kein Beleg für denselben Laufzeit-Einstieg. |
| Code Actions (`toolbar.code_actions=false`) | Der Toolbar-Button erscheint nur, wenn der Editor mindestens einen Code-Action-Provider hat und die Einstellung aktiviert ist. Ohne für die aktuelle Auswahl verfügbare Aktionen ist er deaktiviert; sonst öffnet er die Aktionsliste. Das ist vom gesonderten `editor.inline_code_actions`-Schalter für eine Inline-Schaltfläche am Zeilen-Gutter zu unterscheiden. | Labonair kann LSP-Code-Actions anfordern und anwenden; mehrere Ergebnisse erscheinen in der Editor-Overlay-UI. Im aktuellen Worktree ist weder der Quick-Action-Button noch ein Code-Action-Toolbar-/Gutter-Einstieg belegt. `crates/workspace/src/views/editor.rs`; die Ergebnis-UI allein belegt keine Einstiegsgleichheit. |
| Agent Review (`toolbar.agent_review=true`) | Gehört zum Agent-Diff-Review-Owner, nicht zum allgemeinen Quick-Action-Bar-Render. Im Editor erscheint es bei aktivem Review-Zustand mit Previous/Next Hunk, Reject All, Keep All und Review All Files. Für ein Agent-Diff-Pane erscheinen Sammelaktionen nur, wenn Änderungen vorliegen; bei noch laufender Edit-Tool-Nutzung zeigt der Beitrag einen Spinner. Ohne Review-/Diff-Kontext bleibt er verborgen. | Ein lokaler, gleichwertiger Review-Owner mit hunkweiser Keep/Reject-Steuerung und Review-All-Files-Einstieg ist in den untersuchten Editor-/Workspace-Crates nicht belegt. Das ist eine Agent-Diff-Funktion und nicht mit gewöhnlicher Git-Diff-Navigation gleichzusetzen. |
| Diagnostik-, Project-Search-, Git-Diff-, Sprachwerkzeug-, Logs-, Bild- und Migrationsbeiträge | Diese spezialisierten Toolbar-Items werden von ihren jeweiligen Ownern registriert und abhängig vom aktiven Pane-Item, Prozess-/Diagnostikzustand oder einem speziellen Diff-/View-Kontext links, rechts, sekundär oder verborgen platziert. Sie erweitern die sichtbare Toolbar situationsbezogen; sie sind keine feste Folge allgemeiner Editor-Buttons. | Labonair besitzt mehrere entsprechende Owner und Ansichten, aber die Zed-Beiträge sind nicht als ein globales Toolbar-Menü zu verstehen. Für jeden lokalen Beitrag bleiben Sichtbarkeit, Pane-Zuordnung, Tastaturzugriff und Zustandsübergänge separat zu vergleichen. |

Diese Aufschlüsselung ergänzt die fünf statischen Zed-Toolbar-Settings in der Control-Inventartabelle. Sie erklärt außerdem, weshalb einzelne lokale Commands oder LSP-Ergebnisse nicht automatisch die Toolbar-Funktion abdecken: Die Zed-Leiste verbindet Owner-Auswahl, Fähigkeiten, Einstellungen, Zustandsfeedback und Popup-Aktionen.

### Titelleisten-Controls, Bedingungen und Nutzen

Die folgenden Controls sind Teil von Zeds Fenster-/Workspace-Titelleiste, nicht der Editor-Toolbar. Ihre Schalter und Defaults sind in der 206-Control-Tabelle bei Window & Layout erfasst; diese Tabelle ergänzt den konkreten Zweck und die Zustandsbedingungen. Zed zeigt Elemente abhängig von Plattform, Projekt, Git-Worktree, Login, Verbindung, Call und eingeschaltetem Onboarding-Banner.

| Zed-Titelleisten-Element (Setting/Default oder Bedingung) | Nutzerzweck und verbundene Systeme | Labonair-Gegenstück und Unterschied |
|---|---|---|
| [Application Menu](zed-refrence/zed/crates/title_bar/src/application_menu.rs) (`title_bar.show_menus=false` auf der macOS-Baseline) | Menüaktionen und Einstellungen aus der Titelleiste erreichen. Die ausgeklappte Menüleiste ist plattform-/buildabhängig; auf macOS bleibt das Menü kompakt statt alle Menülabels als Reihe zu zeigen. | Ein einzelner globaler `⋯`-Button öffnet Settings, Keymap, Themes, Icon Themes und Hosts. Er ist ein lokaler App-Menü-Einstieg, keine Menüleiste mit Zeds Projekt-/Account-Bedienelementen. `crates/shell/src/titlebar.rs`. |
| Projekt-Host (`title_bar.show_project_items=true`, wenn ein geteilter/entfernter Host vorliegt) | Anzeigen, wer ein geteiltes Projekt hostet; Klick kann dem Host folgen. Remote-Verbindung, User/Peer-ID und Collaboration-Call sind beteiligt. | Hosts verwaltet SSH/SFTP-Verbindungen; ein geteilter Zed-Projekt-Host und Follow-Teilnehmer-Workflow sind nicht belegt. `crates/hosts-ui/src/hosts.rs`. |
| Projektname / Recent Projects (`title_bar.show_project_items=true`) | Aktives Projekt erkennen und den Recent-Projects-Picker öffnen; Anzeige wird bei langen Namen gekürzt und kann bei mehreren Worktrees einen Pickerpfeil zeigen. | Space-/Projekt-Auswahl und Recent Projects existieren im Workspace; Zeds Projektgruppen- und Multi-Workspace-Verhalten ist nicht als gleichwertige Titelleistenaktion bestätigt. `crates/workspace/src/workspace.rs`. |
| Worktree-Picker (`title_bar.show_worktree_name=true`) | Zwischen Git-Worktrees eines Projekts wechseln und laufenden Erstellungs-/Wechselzustand sehen. | Die lokale Space-Auswahl wechselt Workspaces/Projektwurzeln, nicht nachgewiesen Git-Worktrees innerhalb eines Projekts. Kein entsprechender Titelleisten-Picker belegt. |
| Branch-Picker (`title_bar.show_branch_name=true`; Status-Icon standardmäßig aus) | Branch erkennen; Branch-Menü bietet Branch-/Stash-Aktionen. Bei Detached HEAD wird „Create Branch“ angeboten. `show_branch_status_icon` ersetzt das neutrale Branchsymbol durch Git-Statusindikatoren. | Git-/SCM-Owner hat Branch-Aktionen, aber kein Branch-Button mit Statusanzeige in der lokalen Titelleiste. Der Zed-Statussymbolschalter ist ein eigener Pfad neben dem Branch-Namen. |
| Restricted Mode (`TrustedWorktrees` enthält ein eingeschränktes Projekt) | Sichtbar warnen, dass Project Trust Funktionen einschränkt; Klick öffnet den Sicherheits-/Trust-Dialog. | Lokale AI-Pfadprüfungen und Host-Agent-Access-Schalter sind eigene Sicherheitsebenen; dieselbe Worktree-Trust-Titelleiste ist nicht belegt. |
| Onboarding-Banner (`title_bar.show_onboarding_banner=true`, Banner vorhanden) | Neue Features oder Einstiegsinformationen in der Titelleiste anzeigen; per User-Setting ausblendbar. | Statusbar-Notification-Dropdown ist der lokale Benachrichtigungseinstieg; ein persistenter Onboarding-Banner in der Titelzeile ist nicht belegt. |
| Collaborator-Liste (nur mit eingeloggtem User, Peer-ID und aktivem Call-Room) | Eigene und entfernte Teilnehmer, Sprech-/Mute- und Follow-Zustand sowie Cursorfarben sehen; Klick folgt einem Teilnehmer oder beendet Follow. | Kein Echtzeit-Collaboration-/Call-Room- und Teilnehmer-Follow-Owner gefunden. |
| Call-Controls (nur in einem Call-Room) | Call verlassen, Audio-/Screen-Sharing steuern und Verbindungsqualität mit Latenz-/Jitter-/Packet-Loss-Werten untersuchen. | Kein Call-, Mikrofon-, Screen-Sharing- oder Call-Quality-System in Labonair belegt; Geräte-Setup-Lücke ist auch in der dynamischen Settings-Tabelle verzeichnet. |
| Verbindungsstatus und Update-Hinweis (nur bei Reconnect-/Upgrade-Zustand) | Verbindungsverlust anzeigen; bei erforderlichem Upgrade Update starten oder nach abgeschlossenem Update neu laden. | Remote-Host-Status wird in dessen Workflow verwaltet; ein gleichwertiger Collaboration-/Update-Status in der Editor-Titelleiste ist nicht belegt. |
| Sign In (`title_bar.show_sign_in=true`, aber nur ausgeloggt/Auth-Fehler) | Anmeldung an Zed Hosted Services starten. Während Authentifizierung zeigt die Leiste einen animierten Status. | Kein Benutzerkonto-/Zed-Service-Login in der lokalen Titelzeile; Host-Credentials sind getrennte Verbindungsdaten. |
| User Menu und Avatar (`show_user_menu=true`, `show_user_picture=true`) | Benutzer-/Organisationsprofil, Kontoaktionen und Updates öffnen; Avatar wird nur bei eingeloggtem User und aktiviertem Bildschalter gezeigt, sonst Chevron. | Der lokale globale `⋯`-Menübutton führt zu Produktflächen und Hosts, nicht zu Zed-Account/Organisation/Plan. |
| Linux Window-Button-Layout (`title_bar.button_layout`; Custom-Layout nur bei Custom-Variante) | Native Close/Minimize/Maximize-Buttons gemäß Plattformstandard, Standard- oder GNOME-artiger Custom-Reihenfolge anordnen. Auf macOS greift diese Titlebar-Option nicht als Custom-Linux-Layout. | macOS-Traffic-Lights sind in Labonair als OS-Inset berücksichtigt; benutzerdefiniertes Linux-Fensterbuttonlayout ist nicht als lokale Settingsoption belegt. |

Die Zed-Settings-UI liefert für Projekt-, Worktree-, Branch-, Banner-, Login-, User-, Menü- und Linux-Button-Layout die bereits aufgeführten Defaults. Collaborators, Call-Controls, Restricted Mode, Verbindungsstatus und Update-Hinweise sind dagegen bedingte Laufzeitelemente; ihr Fehlen im Settings-Control-Zähler bedeutet nicht, dass Zed sie nicht besitzt.

## 2. Textmodell, Eingabe und Editieraktionen

Zeds Aktionen umfassen einfache Textbewegung ebenso wie Syntaxbaum-Auswahl, Mehrfachcursor, Edit Predictions, Snippets und mehrere modale Bedienweisen. Die öffentliche Action-Referenz ist dynamisch und deutlich größer als eine Shortlist: [Zed All Actions](https://zed.dev/docs/all-actions). Dieser Bericht inventarisiert alle 585 Actions mit Default-Bindung in der gepinnten macOS-Keymap; die Laufzeit-Routen und semantische Parität jeder einzelnen Action sowie die ungebundene Actionregistry bleiben gesonderte offene Arbeit.

| Zed-Funktion | Labonair-Gegenstück und Status | Nutzen | Verbundene Systeme und Verhalten |
|---|---|---|---|
| Ein Dokument mit Dirty-Zustand, Undo/Redo und stabilen Cursor-/Ankerpositionen | **Vorhanden im Code.** `crates/editor/src/buffer.rs`, `document.rs`. | Änderungen effizient bearbeiten, rückgängig machen und bei Folgeänderungen Auswahlpositionen erhalten. | Ropetextmodell, Transaktionen, Save-Baseline, Syntax/Search-Invaliderung. |
| Datei öffnen, leere Datei, ungespeicherte Änderungen speichern und Datei schließen | **Vorhanden im Code, Ablaufparität teilweise.** `crates/editor/src/lifecycle.rs`, `crates/workspace/src/views/editor.rs`. | Vollständiger Grundzyklus für Quell- und Textdateien. | Filesystem async I/O, Workspace-Tab-Dirty-Policy, Wiederherstellung und externe Änderungen. |
| Zeilen-/Zeichenbewegung, Auswahl, ganze Zeile, Wort und Subwort | **Teilweise bis vorhanden im Code.** `Motion` und Editieroperationen in `crates/editor/src/document.rs`, `editing.rs`; keine vollständige Zed-Action-Parität behauptet. | Präzise Navigation und Textauswahl ohne Maus. | Unicode-Spalten, Soft Wrap, Mehrfachcursor, Basis-Keymap und OS-Layout. |
| Mehrfachcursor per zusätzlicher Position, nächste/alle Vorkommen auswählen | **Vorhanden im Code.** `EditIntent::AddCursor`, `SelectNextOccurrence`, `SelectAllOccurrences`; Commands in `crates/editor/src/command_provider.rs`. | Wiederholte Stellen gleichzeitig ändern und identische Namen umformen. | SelectionSet, Search, Undo-Transaktionen und Keymap. |
| Mehrfachcursor via Alt-Klick/-Drag, beliebige vertikale Cursor und Maus-Auswahl | **Teilweise/nicht belegt.** Das Modell unterstützt mehrere Selektionen; für Zed gleiche Mausgesten wurde kein Nachweis gefunden. | Nicht zusammenhängende Spalten und Textregionen direkt markieren. | GPUI Mouse Events, Caret-Hit-Testing, Auswahlmodell, IME. |
| Auswahl nach größerem/kleinerem Syntaxknoten erweitern oder verkleinern | **Teilweise.** Labonair hat `ExpandSelection`/`ShrinkSelection`; im untersuchten Editor ist keine vollständige Tree-sitter-Node-Auswahl wie in Zed belegt. | Klammerausdruck, Argument, Funktion oder umgebenden Block mit wenigen Tasten auswählen. | Syntaxbaum und sprachspezifische Tree-sitter-Queries. |
| Editierbewegung am AST: Syntaxknoten, Symbol, Klammerbereich; fold/unfold nach Struktur | **Teilweise.** Labonair erhält Symbole, faltet symbolbasierte Bereiche und kann Klammern matchen; Zeds umfassende Syntaxbaum-Aktionen sind nicht belegt. | Code strukturbezogen navigieren und große Dateien komprimieren. | Tree-sitter-Queries, Symbolquelle, Faltranges, Language Extensions. |
| Zeilen duplizieren/verschieben, indent/outdent, Kommentar umschalten, Transpose | **Vorhanden im Code, Sprachparität teilweise.** `EditIntent` und Editor-Commands decken diese Familien ab. | Häufige Umbauten ohne externe Werkzeuge ausführen. | Editor-Settings (Tab-/Einrückungsgröße, Kommentarpräfixe), Language-Metadaten, Mehrfachauswahl. Labonairs Command-Routing nutzt aktuell ein festes Zwei-Leerzeichen-Indent. |
| Auto-Indent, Klammerpaare schließen/überspringen, Auswahl um Klammern legen, Tag-Autoclose und List-Fortsetzung | **Teilweise.** Auto-Indent und `TypeBracket` existieren im Editiermodell; vollständige sprachspezifische Auto-close/Surround/List-Aktionen wurden nicht belegt. | Schreibaufwand sparen und Syntaxbegrenzer korrekt halten. | Grammatik/Sprachkonfiguration, Editor-Settings, Auswahl und Editor-Input. |
| Ausschneiden/Kopieren/Einfügen mit mehreren Carets, Clipboard-Historie/Kill Ring und Character Palette | **Teilweise.** Standardbearbeitung vorhanden; Zed-spezifische Kill-Ring-/Character-Palette-Aktionsfamilie und Mehrfachcursor-Clipboardverhalten nicht vollständig belegt. | Tastaturorientierte Textmanipulation und Wiederverwendung ausgeschnittener Bereiche. | Plattform-Clipboard, GPUI Input, SelectionSet und OS-Tastaturbelegung. |
| Snippets mit Variablen, Auswahlersetzung und navigierbaren Tabstopps | **Teilweise.** Labonair besitzt ein separates Snippets-Feature; enger Editor-Completion-/Snippet-Tabstop-Ablauf hier nicht belegt. | Wiederkehrenden Code mit editierbaren Platzhaltern einsetzen. | Snippets-Owner, Completion-Menü, Datei-/Spracheinstellung und Editor-Fokus. |
| Edit Predictions: mehrzeilige/mehrteilige KI-Vorschläge, einzeln oder abschnittsweise annehmen/ablehnen | **Lücke im Editor-Vergleich.** Labonair hat AI-Funktionen, aber kein hier belegtes Zeta-/Inline-Prediction-Gegenstück. | Häufige nächste Änderungen als Vorschlag erhalten, ohne einen Chat zu öffnen. | Vorhersagemodell/Provider, Cursor-/Bufferkontext, Privacy/Telemetry, Accept/Reject-Keybinds. [Zed Completions](https://zed.dev/docs/completions) |
| Rewrap, Join Lines, Absatz- und Textflussaktionen | **Teilweise/nicht belegt.** `JoinLines` gibt es in Labonairs Vim-Pfad (`J`, `crates/editor/src/vim.rs`); ein allgemeiner Editor-Command dafür ist nicht belegt. Eine Rewrap-Implementierung wurde nicht gefunden. | Kommentare und Textblöcke an verfügbare Zeilenbreite anpassen. | Editor-Breite, Sprache/Kommentarpräfixe, Wrap-Settings. |
| Vertikales/Horizontales Scrollen, Soft Wrap, Scroll-Margen, scroll beyond last line, schneller Alt-Scroll | **Teilweise.** Labonair hat Zeilen-/Spaltenoffset, Word Wrap, Scrollbars und einen Beyond-Last-Line-Wert; keine gleichwertige vollständige Scroll-/Sensitivity-Konfiguration belegt. | Lange Zeilen und große Dokumente kontrollierbar navigieren. | DisplayMap/Viewport, Fontmetrics, Scrollbar, Wrap Guides und UI-Settings. |
| Automatischer und manueller Dokument-Fold, Fold-Stufen, rekursives Auf-/Zuklappen | **Teilweise.** Symbolbasierte Foldbereiche und UI-Marker sind vorhanden; Aktionstiefe und LSP-Folding fehlen beziehungsweise sind nicht belegt. | Struktur großer Quelldateien auf kompakte Ausschnitte reduzieren. | Symbolcache, Syntaxbaum, LSP-Folding-Ranges, gespeicherter Editorzustand. |
| Sticky Scroll / umgebenden Symbolkontext beim Scrollen oben halten | **Teilweise.** Labonair kennt eine `editor_sticky_context`-Option und rendert ein Sticky-Context-Label; identische verschachtelte Zeilen-/Blockdarstellung ist nicht belegt. | Funktions-/Typkontext behalten, obwohl die Deklaration aus dem Viewport scrollt. | Syntaxsymbole, Fold- und DisplayMap, Editor-Settings. |
| Konfigurierbare Schriftfamilie, Größe, Line Height und OpenType-Merkmale | **Teilweise.** Labonair hat Schriftfamilie, Größe und Zeilenhöhe; OpenType-Font-Features pro Buffer sind nicht belegt. | Editor an Lesbarkeit, Dichte, Font und Anzeige anpassen. | Theme Tokens, Plattformfonts, GPUI Text System, User-/Projekt-Settings. Zed `buffer_font_*`. |
| Zeilennummern, relative Nummerierung, Gutter, aktuelle Zeile, Whitespace, Tab-/Indent-Guides, Line Rulers | **Teilweise.** Labonair besitzt Zeilennummern/relative Nummerierung, Current-Line-Highlight, Rulers und ein sichtbares Whitespace-Control. `editor_indentation_guides` ist typisiert und wird an den Display-Pfad übergeben, aber aus der Settings-UI gefiltert; visuelle Gleichheit ist nicht geprüft. | Positionen nachvollziehen, Einrückungen und unsichtbare Zeichen prüfen. | Gutterbreite, Vim `number`/`relativenumber`, Fontgröße, DisplayMap und Settings. |
| Minimap mit Viewport-Thumb, Markierungen, Diagnosen, Git- und Suchpositionen | **Teilweise.** Labonair hat eine Editor-Minimap-Option und Renderer; Zed-artige Thumb-Modi und Overlay-/Markierungsoptionen sind nicht belegt. | Große Datei über Überblicksbild schnell traversieren. | Buffer-Viewport, Search, Diagnostics, Git, Scrollbar und Editorlayout. |
| Cursorform, Cursorblinken, Cursorposition- und Auswahlstatistik | **Vorhanden im Code, Optionen teilweise.** Labonair hat Bar/Block/Underline, Blink und Blinkintervall plus optionale Ln/Col- und Auswahlstatistik. | Sichtbarkeit und Positionsfeedback personalisieren. | GPUI Focus, Settings, Vim-Modus und Statuszeile. |
| Drag-and-drop Auswahl/Textbewegung, Mausrad-Zoom, Mehrfachcursor-Maussteuerung | **Nicht belegt als vollständige Interaktionsfamilie.** | Maus-/Trackpad-Workflows und Anzeigeanpassung unterstützen. | GPUI Pointer-/Scroll-Events, Selection State, Zoom- und Drag-Settings. |
| Großdatei-/generierte-Datei-Fallback ohne UI-Blockierung | **Teilweise.** Labonair setzt eine Syntaxhighlighting-Grenze von 2 MiB und eine konfigurierbare Dateigrößengrenze; Zeds übrige Large-file-Fallbacks/Performance sind nicht abgeglichen. | Minifizierte oder sehr große Dateien öffnen, ohne den UI-Thread unbrauchbar zu machen. | Async Filesystem, Syntaxparser, LSP-Anfragegrenzen, Messfixtures. |

### Datei- und Bufferzustände: Öffnen, Änderungen, Reload und Fehler

Die folgenden Einträge vergleichen den konkreten Lifecycle hinter dem Editorstatus. Die Zed-Baseline ist hier Quellcode des gepinnten Commits; daraus lässt sich die Zustandslogik belegen, aber kein pixelgenauer Empty-/Error-Screen des gebauten Zed-Binaries.

| Zed-Funktion/Zustand | Labonair-Gegenstück und Status | Nutzen | Verbundene Systeme und Verhalten |
|---|---|---|---|
| Neuer/untitled/leerer Buffer, Dirty-Punkt und Save-Baseline | Zed trennt `DiskState::New`, `Present`, `Deleted` und `Historic` (historische Buffer, etwa aus einer Versionskontrolle). Ein leerer neuer Buffer gilt nicht als dirty; bei einem New- oder Deleted-File wird Dirty erst ausgewiesen, wenn der Inhalt nicht leer ist und ungespeicherte Edits vorliegen. Für Zeds leeren Buffer habe ich in den geprüften Editorquellen keinen mit Labonairs Text „Empty file · start typing“ vergleichbaren Placeholder gefunden; die sichtbare Leerfläche bleibt Laufzeitprüfung. Labonair beginnt mit `FileState::New`, rendert diesen Placeholder bei leerem Dokument und wechselt nach Eingabe in Dirty. | Datei-/Tabzustand korrekt erkennen; leere neue Datei nicht mit ungespeicherter Arbeit verwechseln und unmittelbar wissen, wie man beginnt. | Zed `Buffer::is_dirty`, Disk-State, Saved Version und Tabindikator. Labonair `Document`, `FileLifecycle`, Revision und Textflächen-Overlay. `zed-refrence/zed/crates/language/src/buffer.rs`, `crates/workspace/src/pane.rs`, `crates/workspace/src/views/editor.rs`. |
| Datei laden, Binary/ungeeignete Datei und Open-Fehler | Beim fehlgeschlagenen `ProjectItem::try_open` kann der Editor ein `InvalidItemView` anzeigen: „Could not open file“, Root-Cause-Fehlertext und bei lokalen Dateien „Open in Default App“. Ein Binary-Byteinhalt wird im Textbuffer-Reload als nicht unterstütztes Binary abgelehnt und läuft in den Fehlerpfad; Zeds konkrete Darstellung vor/nach Tab-Erzeugung muss am Binary beobachtet werden. Eine allgemeine Größenobergrenze ist in diesem Lifecycle-Audit nicht festgestellt. Labonair zeigt während des asynchronen Lesens einen zentrierten Loading-Status; Binary und TooLarge erhalten eigene Warnflächen ohne Textbuffer, Missing/Error zeigen Details und „Retry“. Ein Open-Fehler wird zusätzlich als Notification gemeldet. | Unlesbare Dateien von leerem Inhalt unterscheiden, Ursache zeigen und einen plausiblen nächsten Schritt anbieten. | Zed `ProjectItem::for_broken_project_item`, `InvalidItemView`, Buffer-Decoding. Labonair Background Executor, Filesystem Reader, Lifecycle-Resolver, Notification Center und Statusbanner. `zed-refrence/zed/crates/workspace/src/item.rs`, `crates/workspace/src/invalid_item_view.rs`, `crates/language/src/buffer.rs`, `crates/editor/src/lifecycle.rs`, `crates/workspace/src/views/editor.rs`. |
| Read-only und temporär gesperrte Tabs | Zed kennt `Capability::ReadWrite`, `Read` (mutable Buffer, aktuell auf read-only geschaltet) und `ReadOnly` (unveränderlicher Buffer). Für `Read` zeigt das Tabmenü einen Lock-Toggle; festes `ReadOnly` zeigt einen deaktivierten Lock-Eintrag. Labonair speichert `can_read`/`can_edit`/`can_save` als Datei-Capabilities und verwendet `FileState::ReadOnly`: Inhalt bleibt sichtbar, Editieren und Save sind gesperrt. Ein entsprechender Editor-Tab-Toggle zwischen editierbar und temporär read-only wurde nicht gefunden. | Text sicher prüfen oder eine schreibbare Datei gezielt vor Änderungen schützen, ohne die Datei zu schließen. | Zed Buffer Capability, Pane-Kontextmenü, Save-Policy und Input-Gating. Labonair Filesystem-Capability-Snapshot und Editor-Input/Save-Gates. `zed-refrence/zed/crates/language/src/buffer.rs`, `crates/workspace/src/pane.rs`, `crates/editor/src/lifecycle.rs`, `crates/workspace/src/views/editor.rs`. |
| Externe Dateiänderung: sauberer Buffer | Zed erhält eine Änderung am zugrunde liegenden File; bei sauberem Buffer und erneut vorhandenem File erzeugt `BufferEvent::ReloadNeeded`, worauf `Project` den Buffer automatisch neu lädt (außer im Collaboration-Sonderfall). Labonair prüft Dateiidentität beim Aktivieren eines Editor-Tabs erneut; eine abweichende Identität bei sauberem Buffer führt zum asynchronen Reload. | Editorinhalt mit Änderungen außerhalb von Labonair synchron halten, ohne eine saubere Ansicht unnötig als Konflikt zu behandeln. | Zed Worktree/File-Events, Buffer-Disk-State, Project Reload und Buffer-Versionen. Labonair `ActiveTabChanged`, Background-Stat/Read, Dateigröße/mtime/Inhaltsidentität und Syntax-/LSP-Neusync. `zed-refrence/zed/crates/language/src/buffer.rs`, `crates/project/src/project.rs`, `crates/workspace/src/workspace.rs`, `crates/workspace/src/views/editor.rs`. |
| Externe Änderung bei Dirty-Buffer, überschreiben oder verwerfen | Zed bestimmt Konflikt aus geändertem Disk-mtime und ungespeicherten Edits bzw. einem gesetzten Konfliktflag; Tabindikator wechselt auf Warning-Farbe. Im Pane-Savepfad hat der Konfliktzweig die Auswahl „Overwrite / Discard Edits / Cancel“; für ein als Konflikt behandeltes, gelöschtes Singleton-File enthält derselbe Zweig „Save / Close / Cancel“. `editor::ReloadFile` ist eine separate Editoraktion. Labonair speichert erwartete Identität aus Pfad, Größe, mtime und Hash; abweichende Identität führt zu `Conflict` und einem Banner mit „Reload (discard my changes)“ oder „Keep mine“. Der reguläre Workspace-Save nutzt `SaveIntent::Normal` und bricht bei externer Abweichung ab. `SaveIntent::OverwriteExternal` existiert im Lifecycle-Modell, aber ich fand im Workspace-Savepfad keinen aufrufenden UI-/Commandpfad dafür. | Eigene und externe Änderungen ohne stilles Überschreiben zusammenführen oder eine bewusste Entscheidung zum Ersetzen treffen. | Zed Conflict-Marker, Save-/Close-Prompt, Buffer-Versionen und File-Disk-State. Labonair typed lifecycle, Identity-Preflight, Recovery-/Conflict-Banner und Notification. `zed-refrence/zed/crates/language/src/buffer.rs`, `crates/workspace/src/pane.rs`, `crates/editor/src/actions.rs`, `crates/editor/src/lifecycle.rs`, `crates/workspace/src/views/editor.rs`. |
| Encoding, BOM und Zeilenenden als Datei-/Editorzustand | Zed-Buffer hält Encoding und Zeilenende getrennt; Reload kann mit erzwungenem Encoding erfolgen. Statusbar-Buttons für aktives Encoding und Zeilenenden sind einstellbar (`status_bar.active_encoding_button`, `line_endings_button`), und der Buffer kann sein Zeilenende umstellen. Labonair akzeptiert derzeit UTF-8 mit optionalem UTF-8-BOM; LF/CRLF/CR werden erkannt und intern normalisiert. Mixed-EOL wird beim Schreiben als LF ausgegeben. Die Info im Lifecycle-Modell ist vorhanden, aber Editorstatusbar-Selector und Encodingwechsel fehlen. | Dateien aus unterschiedlichen Toolchains korrekt lesen und ihre Formatierungsmetadaten beim Save erhalten oder gezielt umstellen. | Zed `encoding_rs`, Buffer-Reload/Serialize, LineEnding und Statusbar. Labonair Filesystem Snapshot, UTF-8/BOM Decode, Canonical LF und Save-Serializer; für gemischte Zeilenenden geht die einzelne pro-Zeile-Information verloren. `zed-refrence/zed/crates/language/src/buffer.rs`, `crates/workspace/src/workspace_settings.rs`, `crates/filesystem/src/file.rs`, `crates/editor/src/lifecycle.rs`. |

## 3. Suche, Navigation und Multi-Buffer

Zed baut viele Resultate als **Multi-Buffer**: eine bearbeitbare Datei-übergreifende Ansicht aus Ausschnitten für Projektsuche, Diagnosen, Referenzen, Git-Diffs und Refactors. Labonairs aktueller Crosswalk markiert diesen Editorbereich als fehlend oder ungeprüft. [Zed Multi-Buffers](https://zed.dev/docs/multibuffers) · [Finding & Navigating](https://zed.dev/docs/finding-navigating) · [Outline Panel](https://zed.dev/docs/outline-panel)

| Zed-Funktion | Labonair-Gegenstück und Status | Nutzen | Verbundene Systeme und Verhalten |
|---|---|---|---|
| In-Datei-Suche, nächster/vorheriger Treffer, um den Cursor oder Auswahlbereich suchen | **Vorhanden im Code, UI teilweise.** `crates/editor/src/search.rs`, `EditorView::find` und `crates/workspace/src/search_overlay.rs`. | Begriffe und wiederkehrende Stellen in einer Datei auffinden. | Search Overlay, Unicode-sichere Ranges, Auswahl, Regex, Wrap und Cursorfokus. |
| In-Datei Replace Next / Replace All mit Regexgruppen und Undo-Transaktion | **Vorhanden im Code.** `replace_one`, `replace_all_checked`; Editor-Aktionen verwenden Dokumenttransaktionen. Vergleich von Tastatur-, Fehler- und Fokuszuständen mit Zed offen. | Sichere Umbenennung/Änderung wiederholter Textstellen. | Regex Engine, Capture-Expansion, Auswahl, Undo, fehlerhafte Query und Trefferzählung. |
| Sucheinstellungen: Regex, Whole Word, Case Sensitive, Smartcase, Suchtreffer im Scrollbar und bei Tippen starten | **Teilweise.** Labonair unterstützt Literal/Regex/Whole Word/Case/Smartcase im Suchmodell; jede Zed-Suchoption plus scrollbar-decorations nicht als gleichwertige Settings belegt. | Suchverhalten an Code- und Textkonventionen anpassen. | Buffer- und Projektsuche, Mehrfachauswahl, Scrollbar, Keymap. Zed `search.*`, `use_smartcase_search`. |
| Projektweite Suche über Suchwurzeln, Ausschlüsse und Git-ignore-Optionen | **Teilweise.** `ProjectSearchSession`, `ProjectSearchQuery` und Workspace-Projektsuche existieren. Filterumfang und vollständige Multi-Buffer-Ausgabe sind offen. | Verwendungen und Text über ein Projekt hinweg auffinden. | Filesystem-Index, Project Roots, Ignore-Regeln, Abbruch, Ergebnisstatus und Search Overlay. |
| Projektsuche und projektweites Replace in einer bearbeitbaren Trefferansicht | **Teilweise.** Labonair führt asynchrone Projektsuche aus und zeigt Trefferlisten mit Navigation zur Fundstelle. Das Replace-Feld bleibt sichtbar, Replace/All sind im Projektmodus aber deaktiviert; dateiübergreifende Änderungen in einer editierbaren Multi-Buffer-Ansicht sind nicht belegt. | Projektweite Treffer vor Änderungen prüfen und Ersetzungen kontrolliert über mehrere Dateien anwenden. | Project Search Session, Workspace-Root/Filesystem, Ergebnisliste und Navigation existieren; Cross-file Replace, Preview/Edit, Dirty-/Save-Status pro Datei und Fehler-/Konfliktauflösung fehlen. |
| Suchresultate bleiben als separate Tabs erhalten, Filterzustände verwalten und zwischen Feld/Ergebnis fokussieren | **Teilweise.** Projektsuche kann Ergebnisse modellieren und Auswahl bewegen; persistente getrennte Such-Multi-Buffers und Filter-UI nicht belegt. | Mehrere Suchen nebeneinander untersuchen und Ergebnislisten schnell bedienen. | Workspace Tabs, Search Session, Project Roots, Keyboard Context. |
| File Finder mit fuzzy Pfadsuche und Vorschau-/Open-Zustand | **Teilweise.** Labonair besitzt `crates/editor/src/file_finder.rs` und einen Editor-Open-File-Befehl; Zed-artige Projektindex-, Preview- und Picker-Interaktionen nicht vollständig belegt. | Datei schnell anhand eines Teilpfads öffnen. | Workspace Roots, Filesystem, Preview Tab, Dateisuche und Keymap. |
| Go to Line/Column und Statusbar-Cursoraktion | **Teilweise.** Cursorposition und `goto_line` sind in der Editor-UI vorhanden; konfigurierbarer Zed-Statusleisten-Button inkl. Line/Column-Popup ist nicht belegt. | Direkt zu präziser Codeposition springen. | Editorstatus, Textposition, Zeilenende, Statusbar-Konfiguration. |
| Outline/Fuzzy Go to Symbol in aktueller Datei; Projekt-Symbolsuche | **Teilweise.** Labonair erzeugt Document Symbols und Command-Palette-Outline; ein dauerhaftes, automatisch mitlaufendes Outline Panel und Projektindex-Symbole sind nicht belegt. | Deklarationen finden und File-Struktur überblicken. | Tree-sitter oder LSP `documentSymbol`, Fuzzy Search, Outline Panel, Multi-Buffer. |
| Go to Definition/Declaration/Type Definition/Implementation sowie Peek/Split-Ziel | **Teilweise.** Labonair routet Definition, Declaration, Implementation und Peek; Type Definition und alle Zed-Resultatansichten sind nicht belegt. | Quellbeziehungen verfolgen, ohne Namen händisch zu suchen. | LSP, Zielauswahl, Navigation History, Pane Split und Multi-Buffer. |
| Find All References in editierbarer Multi-Buffer-Ansicht | **Teilweise.** Labonair kann References über die Language-Service-Grenze anfragen; Ref-Resultat-Multi-Buffer/Outline-Panel fehlen beziehungsweise sind nicht belegt. | Alle Verwendungen prüfen und gruppenweise editieren. | LSP References, Excerpt-Lifecycle, Multi-Buffer, Outline. |
| Projekt-/Datei-Navigationshistorie und zurück/vorwärts je Pane | **Teilweise.** `crates/editor/src/history.rs` existiert; vollständiger Pane-lokaler Sprungverlauf mit Fokusregeln muss abgeglichen werden. | Von Definition/Referenz zur ursprünglichen Arbeitsstelle zurückkehren. | Workspace Pane-Fokus, Cursor/Scrollposition pro Dokument, History-Persistenz. |
| Call Hierarchy, eingehende/ausgehende Aufrufer | **Lücke im untersuchten Labonair-Editor.** | Aufrufbeziehungen ohne manuelle globale Textsuche erkunden. | LSP Call Hierarchy, Picker, Navigation, Projektindex und Einstellungen. |
| Document Links, Hover-Links und direkte Sprünge | **Nicht belegt als vollständiger Ablauf.** | Dokumentations- und Quelllinks im Code entdecken und öffnen. | LSP `documentLink`, Hover Popup, URI-/Workspace-Sicherheit, externes Browsen. |
| Multi-Buffer für Diagnostics/References/Search/Git-Diff/Rename, mit kontextreichen Ausschnitten und gemeinsamer Speicherung | **Lücke bzw. ausdrücklich fehlend im aktuellen Crosswalk.** | Änderungen im Zusammenhang betrachten und gezielt über mehrere Stellen bearbeiten. | Eigener Editor-Multi-Buffer-Owner, stabile Excerpts, Cross-file edit/write, Undo und Dateisystemkonflikte. |
| Outline Panel synchronisiert mit Symbolen eines Multi-Buffers | **Lücke als gemeinsamer Bereich.** | Große Ergebnislisten nach Datei, Ordner und Symbol strukturieren. | Outline Registry/Panel, Multi-Buffer-Excerpt-Mapping, Focus und Navigation. |

### Suchleisten, Filterzustände und Replace-Bedienung

Zed hat zwei zusammenhängende, aber getrennte Suchoberflächen: Buffer Search als dynamischer Editor-Toolbar-Beitrag und Project Search als Workspace-View mit Multi-Buffer-Ergebnissen. Die UI hält Abfrage, Optionen, Trefferstatus und Ersatztext zusammen und markiert laufende bzw. ungültige Abfragen. Labonairs `Cmd-F` ist dagegen ein einzelnes Modal-Overlay, das zwischen Datei und Projekt umschaltet.

| Zed-Ablauf/Control | Labonair-Gegenstück und Status | Nutzen | Zustände, Optionen und verbundene Systeme |
|---|---|---|---|
| Buffer Search-Leiste am aktiven Editor | **Teilweise.** Labonair öffnet per `Cmd-F` ein Overlay statt einer Leiste im Pane-Toolbar-Bereich. Es zeigt Datei-/Projekt-Scope, Trefferzähler und Vor/Zurück-Aktionen. | Suche bleibt am bearbeiteten Dokument und der Trefferstatus ist unmittelbar sichtbar. | Zed zeigt Suchfeld, Case/Whole-Word/Regex-Toggles, Selection-Limit, Zurück/Vor, Zähler und optional Select All Matches. Schmale Zustände können Zähler/Controls reduzieren; Such- und Replace-Zeile sind getrennt fokussierbar. `zed-refrence/zed/crates/search/src/buffer_search.rs`, `crates/workspace/src/search_overlay.rs`. |
| Suche innerhalb der aktuellen Auswahl und Abfrage-Vorbelegung | **Teilweise, anderes Seed-Verhalten.** Labonair übernimmt eine nichtleere einzeilige Auswahl; sonst wird die letzte Suchabfrage wiederverwendet. Eine Auswahlbegrenzung oder Suche nach dem Wort unter dem Cursor ist in der Overlay-UI nicht belegt. | Einen Begriff schnell aus dem Text übernehmen und Vorkommen auf einen markierten Bereich beschränken. | Zed `ToggleSelection` begrenzt Suchtreffer auf den aktiven Auswahlbereich. `seed_search_query_from_cursor` hat Default `always` (Wort unter Cursor), Varianten `selection` und `never`; Selection-Toggle und Seed sind getrennte Einstellungen/Verhalten. `zed-refrence/zed/crates/search/src/buffer_search.rs`, `zed-refrence/zed/crates/editor/src/items.rs`, `crates/workspace/src/search_overlay.rs`, `crates/workspace/src/views/editor.rs`. |
| Project Search: Includes/Excludes, offene Dateien, Git-ignore und Suchbestätigung | **Teilweise.** Labonair-Projektsuche unterstützt asynchrone Suche und Case/Smartcase, Whole Word und Regex im Query-Modell. Das Overlay bietet keine Include-/Exclude-Pfadfelder, keinen „Only Search Open Files“-Schalter und keinen `include_ignored`-Toggle. | Große Projekte auf bestimmte Verzeichnisse/Dateitypen begrenzen oder nur geladene Dateien durchsuchen. | Zed zeigt Include-/Exclude-Globfelder; Filter lassen sich ein-/ausblenden; `Only Search Open Files` und `Include Ignored` sind zusätzliche Projektsuch-Controls. `search.search_on_type=true` führt die Suche beim Tippen aus; bei `false` startet Enter. `zed-refrence/zed/crates/search/src/project_search.rs`, `crates/workspace/src/search_overlay.rs`, `crates/editor/src/search.rs`. |
| Projekttreffer in Multi-Buffer gruppieren, einklappen und navigieren | **Teilweise, Ergebnisansicht fehlt.** Labonair zeigt Treffer in einer begrenzten scrollbaren Liste im Modal; Klick/Pfeile ändern die Auswahl, Enter öffnet die Datei am erfassten Treffer über den sicheren Workspace-Dateipfad. Eine editierbare, nach Datei gegliederte Trefferansicht fehlt. | Treffer umgebend lesen, Gruppen einklappen und mehrere Fundstellen bearbeiten, ohne das Suchpanel zu verlassen. | Zed benutzt einen ProjectSearchView-Multi-Buffer, markiert aktiven und übrige Treffer verschieden und bietet Expand/Collapse All. Labonair hält ProjectSearch-Hits asynchron in `ProjectSearchSession`; Standardlimit 200, Maximalwert 2.000, Ergebnis-Overlay mit Status/Fehler und Auswahlindex. `crates/workspace/src/search_overlay.rs`, `crates/workspace/src/workspace.rs`. |
| Replace Next/All in Datei und Projekt | **Teilweise.** In der aktuellen Datei sind Replace/All nur bei Treffern, gültiger Suche und editierbarem Editor aktiv; Projekt-Scope zeigt die Buttons deaktiviert. `replace_all_checked` baut Edits in Quellreihenfolge und der Dokumentowner wendet sie als eine Undo-Transaktion an. | Mehrere Stellen sicher ändern und eine Aktion bei Bedarf rückgängig machen; projektweite Ersetzungen vor Anwendung prüfen. | Zed blendet per `ToggleReplace` eine zweite Zeile mit Ersatzfeld, Replace Next und Replace All ein. Buffer-Replacement nutzt den aktiven Match bzw. alle Buffer-Treffer. Project Search ersetzt in der Multi-Buffer-Ergebnisansicht; ist eine Suche noch offen, wartet ein ausgelöster Replace-Auftrag auf den bestätigten aktuellen Suchlauf. Regex-Ersetzungsgruppen werden durch den Such-/Editorpfad aufgelöst. `zed-refrence/zed/crates/search/src/buffer_search.rs`, `zed-refrence/zed/crates/search/src/project_search.rs`, `crates/workspace/src/search_overlay.rs`, `crates/editor/src/search.rs`. |
| Case-Modus, Smartcase und Suchstart | **Abweichender Default und UI.** Labonair zeigt einen Case-Schalter zum Wechsel zwischen Smart, Sensitive und Insensitive und sucht bei Texteingabe sofort; ein `search_on_type`-Setting ist in dieser Oberfläche nicht belegt. | Gewohnte Groß-/Kleinschreibung nutzen und zwischen sofortiger Rückmeldung und bestätigter Suche wählen. | Zed Default: Case-insensitive, Whole Word aus, Regex aus, Git-ignore-Dateien ausgeschlossen, `search_on_type=true`, `seed_search_query_from_cursor=always`, `use_smartcase_search=false`. Labonair `SearchOptions` startet mit Smartcase an, Wrap an, Whole Word/Regex/Multiline aus. Zeds Settings sind unter `search.*`, `editor.search.search_on_type`, `seed_search_query_from_cursor` und `use_smartcase_search` erfasst; lokale Optionen stehen in `crates/editor/src/search.rs`. |

## 4. Sprache, Syntax, LSP und Formatierung

Zed verbindet Tree-sitter-Grammatiken und Queries für Syntax-/Strukturarbeit mit LSP für semantische Funktionen. Language Extensions ergänzen Sprachmetadaten, Grammatik, Queries und Serverkonfiguration. Zeds offizielle Liste ist dynamisch; nicht jedes Element ist im gepinnten Commit zwingend identisch. [Language Extensions](https://zed.dev/docs/extensions/languages) · [Configuring Languages](https://zed.dev/docs/configuring-languages) · [Supported Languages](https://zed.dev/docs/languages)

| Zed-Funktion | Labonair-Gegenstück und Status | Nutzen | Verbundene Systeme und Verhalten |
|---|---|---|---|
| Sprachdetektion, Dateitypregeln und Auswahl der Sprache | **Teilweise.** Labonair erkennt rund 22 benannte Sprachfamilien aus Pfad/Endung in `crates/editor/src/language.rs`; frei konfigurierbare Globs, First-Line-Muster und ein Sprachwähler sind nicht gleichwertig belegt. | Richtige Highlight-, Editier-, Kommentar- und Tooling-Regeln pro Datei aktivieren. | Workspace File Open, Settings, Spracherweiterungen und Language Server. |
| Tree-sitter-Syntaxhighlighting, konfigurierbare Capture-Klassen und Theme-Farben | **Vorhanden im Code mit begrenzter Abdeckung.** `crates/editor/src/syntax.rs`, `crates/workspace/src/syntax_theme.rs`; Grammatikbundles für Rust, JSON, TOML, YAML, Python, JavaScript/TypeScript, Go, C/C++, CSS, HTML, Java und Shell. Markdown, SQL, Ruby, PHP, XML, Swift und Kotlin werden erkannt, haben in diesem Bundle aber keine Grammatik. | Tokens, Bezeichner und Sprachstruktur visuell unterscheiden. | Tree-sitter grammars/queries, Theme Tokens, Syntax Theme Selector, große Datei-Fallbacks. Zed-Erweiterungen liefern viel breitere Grammar-/Query-Abdeckung. |
| Syntax Queries für Bracket-Matching, Rainbow Brackets, Autoindent, Fold, Symbol, Injection und syntaktische Auswahl | **Teilweise.** Labonair hat Syntaxhighlighter, Klammer-/Fold-/Symbolteile; Queries für alle Zed-Funktionen, Codeinjections und Sprach-Overrides sind nicht belegt. | Sprache direkt in Bearbeitung, Navigation, Faltung und Formatsteuerung nutzbar machen. | Grammatikpakete, Query Catalog, Language Extensions, Parsercache und Einstellungen. |
| Semantische LSP-Tokens mit Tree-sitter kombiniert oder als vollständige Ersetzung | **Teilweise.** Labonair hat Semantic-Token-Daten und Decorations; Zeds `off`/`combined`/`full`, Prioritäten und Erweiterungsregeln sind nicht belegt. | Unterschiedliche Bezeichner semantisch korrekt statt allein syntaktisch einfärben. | LSP `semanticTokens`, Tree-sitter Theme und `global_lsp_settings.semantic_token_rules`. |
| Dynamische Language Extensions für Grammatik, Server, Snippets, Debuggerkennung und Settings | **Lücke im untersuchten Produktpfad.** Keine Extensions-Owner/Installer-Funktion in Labonairs Editor gefunden. | Sprachabdeckung unabhängig vom App-Release erweitern und Tools bereitstellen. | Extensions Catalog, Download/Update, Permissions/Trust, Language Server, Grammar-Revisions, Offline-/Fehlerpfad. |
| Language Server entdecken, automatisch installieren/starten, priorisieren, deaktivieren und konfigurieren | **Teilweise, grundlegend anders.** `crates/editor/src/lsp_process.rs`, `runtime.rs`, `language_services.rs` bieten Local-Process/LSP-Protokoll, typed Requests und Statusprojektion. Runtime ist standardmäßig deaktiviert; Prozesse werden explizit injiziert. Automatischer Download, Extension-Adapter und Server-Priorisierung fehlen. | Sprachintelligenz mit vorhandenen Projektwerkzeugen nutzen und den Server bei Fehlern verwalten. | Editor Owner, Tokio/Prozess I/O, LSP Registry, Capability-Status, Settings und Trust. Zeds Setup siehe [Configuring Languages](https://zed.dev/docs/configuring-languages). |
| LSP Completion, Dokumentation, Snippets und Vorschlagsannahme | **Teilweise.** Completion-Request und UI sind in `language_services.rs` und `views/editor.rs`; Completion-Schema, Snippettabstop und komplette Suggestion-Interaktionen sind nicht belegt. | Bezeichner und APIs passend zu Projekt-/Typkontext vervollständigen. | LSP `textDocument/completion`, UI-Kandidaten, Snippets, Fokus, Debounce, Cancellation. |
| Signaturhilfe und Parameterinformationen im Call-Site-Kontext | **Teilweise, in Code vorhanden.** Labonair hat SignatureHelp-Requests, Statuszustände und Renderer; Server-/Popup-Feature-Parität bleibt offen. | Parameter einer Funktion beim Schreiben sehen. | LSP `signatureHelp`, Cursorposition/Bufferrevision, Popup-Navigation und Settings. |
| Hover mit Typ-/Dokumentationstext, Diagnose- und Link-Popover | **Teilweise.** LSP Hover-Request und Tooltip-Renderer sind vorhanden; vollständige Markdown-/Link-/Codeblockdarstellung nicht belegt. | Symboldefinitionen verstehen, ohne die Datei zu verlassen. | LSP, Hover Links, Tooltip-/Popover-Positionierung und Syntaxhighlighting. |
| Diagnostics: Push/Pull, Unterstreichung, Hover, Severityfilter, Error Lens, Scrollbar, Project Panel und Tabs | **Teilweise, auf das aktive Dokument begrenzt.** Labonair verarbeitet LSP-Push-Diagnosen und begrenzte Syntaxdiagnosen, verwirft Ergebnisse mit veralteter Dokumentrevision und zeichnet Bereichs-Unterstreichungen. Es gibt Commands zum Zählen und Navigieren. Severityfilter, Diagnose-Hover, Error-Lens-Zeilen, Diagnosemarker in Scrollbar/Tabs/Explorer sowie ein Projekt-Diagnosefenster sind nicht belegt; Pull-Requests an LSP-Server fehlen. | Probleme während der Bearbeitung erkennen, erklären, priorisieren und über Datei- und Projektgrenzen verfolgen. | LSP, Inline Popover, Scrollbar, Statusbar, Tab-/Explorer-Indikatoren, Multi-Buffer und Workspace-Diagnoseansicht. [Zed Diagnostics](https://zed.dev/docs/diagnostics) |
| Code Actions, Quick Fixes und Multi-file-Refactors mit Vorschau/Undo | **Teilweise.** CodeAction-Request und UI, Rename und Apply-Edits sind vorhanden; breit angelegte LSP Workspace Edits als reviewbarer Multi-Buffer plus Dateisystem-Undo sind nicht belegt. | Diagnosen beheben und strukturierte Umformungen kontrolliert anwenden. | LSP WorkspaceEdit, Diagnostics, Multi-Buffer, Undo, File Lifecycle. |
| Rename Symbol über Datei- und Projektgrenzen, Vorschau in Multi-Buffer | **Teilweise.** Rename-Request und Edit-UI sind im Editor implementiert; Zeds reviewbarer Cross-file-Speicher-/Abbruchablauf benötigt Multi-Buffer und ist nicht belegt. | Konsistente Bezeichneränderung ohne Treffer zu übersehen. | LSP `rename`, Filesystem, Multi-Buffer, Undo, Konflikte. |
| Go-to-Definition, References, Implementation, Type Definition, Workspace Symbols, Call Hierarchy | **Teilweise.** Ein Teil der Requests und Commands ist vorhanden; Call Hierarchy/Type Definition/Workspace Symbol Index und Zed Multi-Buffer-Resultate fehlen oder sind nicht belegt. | Struktur des Projekts über Datei- und Symbolgrenzen navigieren. | LSP-Capabilities, Ergebnis-Picker, Workspace Index, Navigation History. |
| Formatting des Dokuments und der Auswahl, Organize Imports und Format-on-Save | **Teilweise.** Labonair hat Formatting-, Format-Selection- und Organize-Imports-Routen. Format-on-save wird im Save-Pfad ausgelöst, sein typisiertes Feld ist jedoch aus der nativen Settings-UI gefiltert. Auswahlformatierung und pro Sprache externe Formatterkonfiguration sind nicht vollständig belegt. | Einheitlichen Stil und Imports automatisiert pflegen. | LSP oder externe Tools, Language Settings, Save-/Undo-/Diff Lifecycle. |
| Linter und Diagnoseintegration | **Teilweise.** Diagnostics können über Local Language Services kommen; installierbare konfigurierbare Linter-Erweiterungen und Save-Tasks sind nicht belegt. | Stil-/Qualitätsfehler jenseits Syntaxfehler erkennen. | Sprache, LSP, Tasks, Statusbar und Erweiterungs-Registry. |
| Inlay Hints für Parameter, Typen und Servermetadaten | **Lücke im untersuchten Editor-Modell.** Kein eigener Inlay-Hint-Werttyp oder Renderer wurde gefunden. | Typ-/Argumenthinweise inline anzeigen, ohne Code zu verändern. | LSP `inlayHint`, Position/Revision, Theme, Debounce und Anzeige-Settings. |
| Code Lens, Referenzzählungen, Implementierungen und Metadaten über Code | **Lücke im untersuchten Editor-Modell.** | Zusätzliche Aktions-/Beziehungsinfos an Deklarationen anzeigen. | LSP `codeLens`, Commands, Inline Blocks und Refresh. |
| Document Colors / eingebettete Farben, z. B. CSS-Farbvorschauen | **Lücke im untersuchten Editor-Modell.** | Farbwerte visuell auswählen und verstehen. | LSP `documentColor`, Color Picker, Color Theme und Text Editing. |
| Code folding ranges aus LSP mit Tree-sitter-/Indent-Fallback | **Teilweise.** Labonair faltet lokale Symbolbereiche; eigener `foldingRange`-LSP Request/Abgleich ist nicht belegt. | Sprachserver kann sprachgenaue Foldgrenzen liefern, Grammatik bleibt fallback. | LSP, Tree-sitter, FoldMap und Settings. |
| Language Server status/menu, Start/Stop/Restart, Timeout und Serverkonfiguration | **Teilweise.** Labonair hat Start-/Stop-/Restart-Commands, Prozessstatus und Fehleranzeige; automatischer Lifecycle/Settingseditor sind nicht belegt. | LSP-Ausfälle erkennen und beheben, ohne Editor neu zu starten. | Process Owner, JSON-RPC, Notifications, Settings, Cancellation und Secret-Redaction. |
| Workspace-spezifische Settings, per-Sprache Overrides und projektbezogene Toolchainwahl | **Teilweise.** Labonair unterstützt Layered User-/Project-Settings, aber Feldumfang, `.editorconfig`, Sprach-Overrides und Toolchain Picker fehlen oder sind nicht belegt. | Konventionen pro Projekt/Sprache statt appweit erzwingen. | Settings Store, Project Trust, EditorConfig, Language Registry und Toolchains. |

### Diagnosen als Ablauf: Empfang, Darstellung, Navigation und Projektansicht

Die Diagnoseanzeige in Zed besteht aus getrennten Oberflächen. `diagnostics_max_severity` filtert die Anzeige im Editor, nicht den Empfang von Diagnosen oder die Projektansicht. Scrollbar, Inline-Zeilenhinweise, Tabs, Project Panel und die Projektansicht besitzen zusätzliche, voneinander unabhängige Controls. Die folgenden Details beziehen sich auf den gepinnten Quellstand.

| Schritt oder Fläche | Zed im gepinnten Stand | Labonair im aktuellen Worktree | Nutzerwirkung und verbundene Systeme |
|---|---|---|---|
| Ergebnisse empfangen und einer Dokumentrevision zuordnen | LSP-Push-Diagnosen werden im Projekt gespeichert; optionales LSP-Pull ist standardmäßig aktiv und wartet 50 ms. `diagnostics_max_severity: all` filtert nur Editor-Rendering; laut Setting-Kommentar laufen Abruf und Projekt-Diagnoseansicht weiter. | Ein injizierter LSP-Prozess nimmt `textDocument/publishDiagnostics` an und speichert Diagnosen je URI. Eine mit Versionsnummer versehene Nachricht wird verworfen, wenn sie nicht zur aktuellen Dokumentversion gehört. Nach asynchroner Dokument-Synchronisierung fragt der Workspace den Diagnosestand ab; `LanguageServiceState` übernimmt Ergebnisse nur bei passender Revision und Generation. Der eingebaute Syntaxprovider prüft derzeit Klammerpaare und in JSON nicht geschlossene Zeichenketten. | Fehler dürfen nicht aus einer älteren Datei- oder Serverantwort in eine neuere Ansicht wandern. Zed ergänzt Push um konfigurierbares Pull; Labonairs Prozesspfad verwendet Push, eine automatische Serverinstallation/-zuordnung bleibt außerhalb des Vergleichs. [`lsp_process.rs`](crates/editor/src/lsp_process.rs), [`language_services.rs`](crates/editor/src/language_services.rs), [`views/editor.rs`](crates/workspace/src/views/editor.rs). |
| Diagnosebereiche im Editor und Nachrichtendetails | Zed zeichnet Diagnosebereiche und kann beim Hover eine Markdown-Erklärung mit Quelle/Code, verwandten Diagnosen und Kopieraktion zeigen. Die Diagnoseblöcke verwenden nach Severity getönte Hintergründe und Rahmen; Links können innerhalb der Diagnosegruppe navigieren. | Labonair projiziert Diagnosebereiche über den Display-Map und zeichnet eine ein Pixel hohe Akzent-Unterstreichung. Sie unterscheidet in der aktuellen Render-Schleife Error, Warning, Information und Hint nicht nach Farbe oder Stil. Die Nachricht wird nicht als Hover-Popover am Bereich angezeigt. | Die Unterstreichung lokalisiert den Fehler; Zeds Hover und Diagnoseblöcke erklären ihn dort, wo er auftritt. Lokale Meldungen sind über den separaten Show-Diagnostics-Command nur als Anzahl sichtbar. [`diagnostics.rs`](zed-refrence/zed/crates/editor/src/diagnostics.rs), [`diagnostic_renderer.rs`](zed-refrence/zed/crates/diagnostics/src/diagnostic_renderer.rs), [`views/editor.rs`](crates/workspace/src/views/editor.rs). |
| Inline-Meldung am Zeilenende (Error Lens) | Optionaler Text hinter der Quellzeile; standardmäßig aus. Defaultwerte: 150 ms Aktualisierungsverzögerung, vier Em-Abstände, Mindestspalte 0 und Severity aus dem Editorfilter übernehmen. | Kein Diagnose-Nachrichtentext am Zeilenende und kein entsprechendes Sichtbarkeits-, Abstands- oder Verzögerungssetting gefunden. | Kurze Erklärung lesen, ohne den Zeilenbereich zu überfahren oder ein Panel zu öffnen. Zed-Settings liegen unter `diagnostics.inline.*`. |
| Scrollbar-, Tab- und Dateibaumindikatoren | Scrollbar-Diagnosemarker zeigen standardmäßig alle Severities. Der Project Panel markiert Dateien mit Fehlern und Warnungen (`project_panel.show_diagnostics: all`); Tabs sind standardmäßig aus und benötigen außerdem aktive Dateiglyphen (`tabs.show_diagnostics: off`). Beide Flächen haben eigene Einstellungen. | Ein `editor_diagnostics`-Schalter steuert die Diagnosepräsentation im Editor. Diagnosemarker in Workspace-Tabs, Explorer und Editor-Scrollbar wurden im untersuchten lokalen Pfad nicht gefunden. | Fehlerdateien erkennen, bevor sie geöffnet werden, und ihre Position in einer langen Datei überblicken. [`default.json`](zed-refrence/zed/assets/settings/default.json), [`pane.rs`](zed-refrence/zed/crates/workspace/src/pane.rs). |
| Diagnose unter dem Cursor, nächste und vorige Diagnose | `F8` und `Shift-F8` springen vor/zurück; das Defaultargument schließt Hint bis Error ein. Zed kann die Diagnose am Cursor aktivieren und ihre Detailblöcke entfalten; Filterargumente erlauben andere Severitybereiche. | Die Palette registriert `Show Diagnostics`, `Next Diagnostic` und `Previous Diagnostic`, aber kein entsprechendes Default-Keybinding wurde im lokalen Editor-Provider gefunden. Vor/zurück läuft zyklisch durch die Diagnosebereiche des aktiven Dokuments und setzt die Einfügemarke an deren Anfang. Es gibt keinen Severityparameter. | Ein kurzer Sprung hält den Schreibkontext; projektweite Navigation läuft separat über die Ergebnisansicht. [`default-macos.json`](zed-refrence/zed/assets/keymaps/default-macos.json), [`command_provider.rs`](crates/editor/src/command_provider.rs), [`views/editor.rs`](crates/workspace/src/views/editor.rs). |
| Projektstatus und persistenter Einstieg | Der Statusleistenindikator zeigt einen Check bei null Fehlern/Warnungen oder getrennte Error-/Warning-Zähler. Er kann zusätzlich die aktuelle Diagnose als anklickbaren Text zeigen. Klick öffnet Project Diagnostics; `diagnostics.button` ist standardmäßig aktiviert. | `Show Diagnostics` veröffentlicht im Notification Center eine Infozeile mit der Anzahl für das aktuelle Dokument oder „No diagnostics for the current document.“ Es gibt keinen persistenten Diagnosezähler in der lokalen Statusleiste. | Projektzustand erkennen, ohne eine bestimmte Datei zu öffnen, und eine aktive Meldung direkt anspringen. [`items.rs`](zed-refrence/zed/crates/diagnostics/src/items.rs), [`views/editor.rs`](crates/workspace/src/views/editor.rs). |
| Projektweite und dateibezogene Ergebnisansicht | `Cmd-Shift-M` öffnet bzw. fokussiert ein Project-Diagnostics-Workspace-Item. Es hält Diagnosen aus mehreren Buffern in einem Multi-Buffer mit Excerpts und Diagnosegruppen. Warnungen lassen sich ein-/ausschalten; `Ctrl-R` aktualisiert oder stoppt den laufenden Refresh. Ein Empty State unterscheidet „No problems in workspace“ von „No errors in workspace“ und kann vorhandene Warnungen einblenden. Für eine einzelne Datei gibt es zusätzlich einen Buffer-Diagnostics-Owner. | Keine Projekt- oder Einzeldatei-Diagnoseansicht gefunden. Der lokale Show-Command zählt Ergebnisse, Next/Previous bewegt den Cursor; es gibt keine Liste, Excerpts, gruppierte Meldungen, Warnungsfilter oder manuellen Diagnose-Refresh. | Fehler über Dateien vergleichen, zwischen Fundstellen springen und aus einem gemeinsamen Ergebnisfenster zurück in den Buffer navigieren. Zeds Ansicht bezieht Projektereignisse ein und aktualisiert Excerpts asynchron; lokale Diagnosen bleiben am jeweils aktiven Dokument. [`diagnostics.rs`](zed-refrence/zed/crates/diagnostics/src/diagnostics.rs), [`buffer_diagnostics.rs`](zed-refrence/zed/crates/diagnostics/src/buffer_diagnostics.rs). |
| Filtereinstellungen und Reichweite | Defaults: Editor und Scrollbar zeigen alle Severity-Stufen; Projektansicht nimmt Warnungen auf; Inline-Zeilenhinweise sind aus. `diagnostics_max_severity` beeinflusst nicht den Datenabruf. Projekt Panel (`all`) und Tabs (`off`) weichen bewusst voneinander ab. | `editor_diagnostics` ist standardmäßig `true` und bietet einen binären Präsentationsschalter. Separater Severity-, Warnungs-, Scrollbar- und Projektansichtsfilter ist nicht belegt. | Einzelne Oberfläche ohne ungewollten Verlust von Diagnosedaten anpassen. Diagnoseerhebung, Editoranzeige, Dateiindikatoren und Projektliste sind getrennte Zustände, keine einzelne globale Ein/Aus-Option. [`default.json`](zed-refrence/zed/assets/settings/default.json), [`editor.rs`](crates/settings-content/src/editor.rs), [`schema.rs`](crates/settings-ui/src/schema.rs). |

## 5. Git, Änderungsgutter, Diff und Review

Zed verbindet Editor-Gutter und Inline Blame mit Git Panel, Datei-/Projekt-Diff, File History, Branch/Worktree und Staging einzelner Hunks. Diffansichten sind Unified oder Side-by-Side und können Ausschnitte bearbeiten. [Zed Git](https://zed.dev/docs/git)

| Zed-Funktion | Labonair-Gegenstück und Status | Nutzen | Verbundene Systeme und Verhalten |
|---|---|---|---|
| Editor-Gutter markiert Added/Modified/Deleted im Vergleich zu Git-Base | **Vorhanden im Code.** Labonair berechnet revision-bound Git-Hunks und rendert sie am Zeilengutter. `crates/editor/src/git.rs`, `views/editor.rs`. | Lokale, noch nicht gespeicherte bzw. nicht committete Codeänderung in Kontext sehen. | Git Owner/Bridge, Filesystem Snapshot, Bufferrevision, diff base, Theme. |
| Hunk-Kontextmenü: Stage, Unstage, Discard mit Vorschau/Bestätigung und Project Diff öffnen | **Teilweise bis vorhanden im Code.** Labonair hat hunk-spezifische Actions, Discard-Bestätigung und Git Context Menu; vollständige parity in jedem Git-Zustand ist nicht durch Runtime belegt. | Änderung aus dem Editor heraus reviewen und gezielt übernehmen/verwerfen. | `labonair-git`, Git Bridge, Patch/Hunk IDs, Notification/Confirmation, Project Diff. |
| Nächste/vorherige Git-Änderung und Hunk ein-/ausklappen | **Teilweise.** Editor-Commands für Next/Previous Change und Project Diff existieren; vollständige Zed-Expand/Collapse-Auswahlfamilie nicht belegt. | Schnell von einem Änderungsblock zum nächsten springen. | Git-Gutter Snapshot, scroll/cursor, Diff decorations, Keymap. |
| Inline Blame für aktive Zeile mit Verzögerung oder Statusbar-Position | **Lücke bzw. nicht belegt.** `git_inline_decorations_for_view` ist vorhanden, aber die Zed-Blame-Infos samt Autor/Commit/Settings sind im untersuchten Editor nicht belegt. | Wer eine Zeile geändert hat, ohne File History oder Terminal öffnen zu müssen. | Git blame, Cursor idle timer, Statusbar/Inline decoration, Commit Summary. |
| Stage/Unstage hunks und staged/unstaged visual style | **Teilweise.** Hunk Actions unterstützen Stage/Unstage; die volle Zed-Hunk-/Statusdarstellung und `git.hunk_style` fehlt beziehungsweise ist nicht als gleichwertig bestätigt. | Teilweise Änderung eines Files sicher in Commit aufnehmen. | Git index, Stage patch API, Gutter/Theme. |
| File Diff, Project Diff, Branch Diff, Stash Diff und File History | **Teilweise.** Labonair hat `crates/workspace/src/views/project_diff.rs`, Git Graph und Diff-Modell; die gemeinsame Zed-artige History-/Stash-/Branch-Diff-Ansicht und universelle DiffView sind nicht vollständig belegt. | Änderungen an Arbeitsbaum, Branches oder Historie an einem Ort vergleichen. | Git, Git Graph, Multi-Buffer, DiffView-Owner, Workspace Tabs. |
| Unified oder Side-by-Side Diffview mit synchronisierter Navigation und Word Diff | **Teilweise.** Diff-/Word-Diff-Modelle existieren; die vollständigen Zed Layout-/Wechsel-/Scroll-Verhaltensweisen und gemeinsamen Git-Producer-Verträge sind offen. | Kleinänderungen inline oder größere Strukturdifferenzen nebeneinander lesen. | DiffView, Textlayout, Scroll Sync, Theme, Git producer. |
| Multi-Buffer Diff-Hunks sind bearbeitbar; einzelne Dateien können gespeichert werden | **Lücke im Multi-Buffer-Sinn.** Labonair hat Projekt-Diff-Fläche, aber keinen bestätigten, cross-file editierbaren Multi-Buffer-Contract. | Änderungen direkt am Vergleich korrigieren und speichern. | Shared DiffView, BufferExcerpt, WorkspaceEdit, File Lifecycle und Undo. |
| Git-Statusfarben in Tabs/Projektbaum, Statusleiste und Diff-Gutter | **Teilweise.** Git-Gutter in Editor und dedizierte Source Control/Git Graph Oberflächen existieren; Tabstatus-/Gutter-Detailabgleich nicht belegt. | Status erkennen, auch wenn betroffene Datei gerade nicht geöffnet ist. | Git registry/events, Workspace Tabs, Explorer, statusbar und settings. |

## 6. Editorbezogene Vorschau, Read-only und Dateitypen

„Preview“ bezeichnet bei Zed verschiedene Dinge: flüchtige Datei-Tabs aus dem Project Panel, Markdown/SVG-Renderings, Bildansicht, Theme-Preview und sprach-/frameworkspezifische Tooling-Vorschauen. Diese Bereiche sind daher in den Tabellen getrennt.

| Zed-Funktion | Labonair-Gegenstück und Status | Nutzen | Verbundene Systeme und Verhalten |
|---|---|---|---|
| Markdown Preview öffnen, fortlaufend mit Editor synchronisieren, Preview neben Quelle öffnen | **Teilweise.** Labonair rendert Markdown im Preview-View; gekoppelte Source-Following- und Side-by-Side-Preview-Aktionen sind nicht belegt. | Formatierte Darstellung prüfen, während der Quelltext offen bleibt. | Markdown Parser/Renderer, Editor Buffer, Workspace Split, Preview Tab. Zed Actions: `markdown::OpenPreview`, `OpenFollowingPreview`, `OpenPreviewToTheSide`. |
| Markdown Vorschau: Codeblöcke, Links, Listen, Tabellen, Bilder, Anker und Scrollnavigation | **Teilweise, mit kleinerem Renderer.** `crates/workspace/src/markdown.rs` erkennt Überschriften, Absätze, Fence-Code, Bullet-/Nummernlisten, Zitate, Trennlinien, Pipe-Tabellen und Inline-Links. `views/preview.rs` rendert Code monospaced, aber ohne Syntax-Highlighting; Inline-Inhalte werden zu einfachem Text abgeflacht, Linkziele sind nicht interaktiv. Bildblöcke, Heading-Anker und Navigation zwischen Quelle und Vorschau sind nicht implementiert. | Dokumentation in ähnlicher Form lesen wie publiziert. | Markdown parser, URI handling, syntax grammars, theme, scroll/link routing. [Zed Markdown](https://zed.dev/docs/languages/markdown) |
| SVG Preview mit Following und Side-by-Side Actions | **Lücke als native Labonair-Preview.** Labonair behandelt SVG aktuell als XML Text/öffnet nicht-native Formate extern gemäß Preview-Implementierungsnotiz. | Vektorgrafik ohne externe Anwendung prüfen. | SVG Renderer, Filesystem, Workspace Preview, Sicherheitsgrenze für eingebettete Inhalte. Zeds Actions listet `svg::OpenPreview*`. |
| Image Viewer für Rasterbilder mit Dateimetadaten/Einheiten und geeigneter Darstellung | **Teilweise.** Labonair rendert PNG/JPEG/GIF/WebP/BMP/ICO nativ via GPUI Image; Zeds Viewer-Tools, Zoom-/Pan-/Metadatenzustände und vollständige Formate sind nicht abgeglichen. | Grafiken und Bildassets direkt im Workspace ansehen. | GPUI Image Decoder, Filesystem, Tab/Preview und `image_viewer.unit`. |
| HTML-/Browser- oder komponentenspezifische Live-Preview | **Lücke beziehungsweise nicht belegt.** Labonairs Preview öffnet HTML und Remote URLs im Systembrowser; eingebettete Zed-Komponenten-/Frontend-Preview wird nicht als equivalent gefunden. | UI/Komponente oder Webansicht in der Nähe des Quellcodes betrachten. | Web Runtime, Dev Server, Framework/Language Extension, Terminal Tasks, Split Pane, Port-/Trust-Grenzen. |
| Theme-/Icon-Theme-Live-Preview direkt im Editor | **Teilweise.** Theme-Owner und Editorpalette sind vorhanden; Theme selector kann Theme wechseln, aber Zeds vollständiger Live-Preview-/Builder-Workflow wurde nicht verglichen. | Farb-/Syntaxänderung vor der dauerhaften Auswahl visuell bewerten. | Theme Store/Extensions, Syntax Theme, Icon Theme, Editor und alle anderen Oberflächen. |
| Emmet-Abbreviation mit Live-Expansion, Inline Input, Enter anwenden, Escape abbrechen | **Lücke im untersuchten Editor.** | HTML/CSS-Struktur aus Kurzsyntax erzeugen. | Emmet Language Extension, Syntax/Selection, Preview Input, Fehlerzustand. [Zed Emmet](https://zed.dev/docs/languages/emmet) |
| Tool-spezifische Vorschau für Farben, Links, Hover, Code Actions und Completion | **Teilweise.** Hover, Completions und Actions existieren als Editoroberfläche; breite Preview-Anbieter, Popover-Positionierung und Langformate müssen separat verglichen werden. | Ergebnisse prüfen, ehe sie übernommen oder extern geöffnet werden. | LSP, UI-Kit Popover, Theme, Viewport collision, focus/dismissal. |
| Nicht-Text-/Binärdatei und unsupported preview: Zustand, read-only, öffnen extern, Fehlertext | **Teilweise.** Labonair File Lifecycle kann uneditierbare/fehlgeschlagene Datei modellieren; vollständige Typabdeckung und Preview/Editor-Wechsel sind nicht belegt. | Datei sicher erkennen, anzeigen oder gezielt an eine andere Anwendung übergeben. | Filesystem MIME/identity, Preview Resolver, OS-Handler, Editor Save Policy. |

### Preview-Einstieg, Bindung und Zustände

Zeds „Preview“ sind eigenständige Workspace-Items mit je eigener Integration; Labonairs Explorer-Aktion öffnet dagegen einen universellen `TabKind::Preview`. Das bedeutet, dass „Datei vorübergehend im Tab öffnen“ und „Dateiinhalt als Vorschau rendern“ nicht denselben Ablauf bezeichnen.

| Ablauf | Gepinnter Zed | Aktueller Labonair-Worktree |
|---|---|---|
| Vorschau aus Editor-Toolbar | Zeds Quick Action Bar prüft den aktiven Inhalt der jeweiligen Pane und zeigt das Auge nur für Markdown, SVG und CSV/TSV/SSV/PSV. Klick öffnet in derselben Pane; Alt-Klick öffnet nebenan. Das ist pane-lokal und zielt auf genau das Item der Toolbar. `zed-refrence/zed/crates/zed/src/zed/quick_action_bar/preview.rs`. | Der Editor zeigt im untersuchten Pfad keine entsprechende Preview-Augenaktion. Der Explorer-Kontext bietet „Preview“ nur für als previewable erkannte Pfade. `crates/panel-explorer/src/panel_explorer.rs`, `crates/workspace/src/views/preview.rs`. |
| Markdown: normal, neben Quelle und automatisch | `markdown::OpenPreview` bindet eine gerenderte Ansicht an den aktiven Markdown-Editor; Änderungen und Dateiwechsel aktualisieren sie. `OpenPreviewToTheSide` öffnet dieselbe Quelle in einer Nachbar-Pane. `OpenFollowingPreview` erzeugt eine Follow-Ansicht, die dem zuletzt aktiven Markdown-Editor folgt. macOS-Standard: `Cmd-Shift-V` für normal, `Cmd-K V` für nebenan; Follow hat in der geprüften Standard-Keymap keine Bindung. Automatisches Öffnen beim Laden eines Markdown-Files ist über `markdown_preview.open_markdown_files_in_preview` abschaltbar und standardmäßig `false`. `zed-refrence/zed/crates/markdown_preview/src/markdown_preview_view.rs`, `markdown_preview_settings.rs`, `assets/keymaps/default-macos.json`, `assets/settings/default.json`. | Explorer „Preview“ ruft `Workspace::open_preview` auf, aktiviert bzw. aktualisiert den bestehenden Preview-Tab und wechselt dessen Adresse. Der Tab hat keine Bindung an einen Editor-Buffer, folgt nicht Cursor oder Dateiwechsel und öffnet nicht automatisch eine zweite Pane. `crates/panel-explorer/src/panel_explorer.rs`, `crates/workspace/src/workspace.rs`, `crates/workspace/src/views/preview.rs`. |
| Markdown: Breite, Darstellung und Synchronisation | `markdown_preview` kennt Schriftgröße/-familie, Code-Schriftfamilie, `open_markdown_files_in_preview`, `limit_content_width` und `max_width`; gepinnte Defaults begrenzen die Breite auf 800 px. Das normale Preview folgt Änderungen des gebundenen Buffers; Auswahl-/Cursorpositionen können zur passenden Stelle im Preview scrollen. Follow bindet die Vorschau bei aktivem Item-Wechsel erneut. `zed-refrence/zed/assets/settings/default.json`, `crates/markdown_preview/src/markdown_preview_view.rs`. | `PreviewView::resolve` liest die Datei und übergibt sie an denselben kleinen Blockparser, der auch für AI-Streaming-Antworten gedacht ist. Es gibt keine Bufferrevision, Source-Selection, Follow-Modus, Breitenbegrenzungseinstellung oder Markdown-Preview-Einstellungen. Die Kopfzeile zeigt Adresse, „Reload“ und bei nichtleerer Adresse „Open externally“. `crates/workspace/src/markdown.rs`, `crates/workspace/src/views/preview.rs`. |
| SVG und Tabellendaten | Markdown-, SVG- und Tabellendaten-Previews haben jeweils `OpenPreview`/`OpenPreviewToTheSide` und denselben Quick-Action-Einstieg. SVG hat zusätzlich `OpenFollowingPreview`; die Follow-Ansicht reagiert auf das aktive Workspace-Item. Die geprüfte macOS-Keymap bindet normales und seitliches Öffnen für SVG und CSV-artige Dateien, aber nicht Follow. SVG wird im Preview als gerenderte Vektorgrafik dargestellt; Tabellendaten haben einen eigenen Preview-Owner. `zed-refrence/zed/crates/svg_preview/src/svg_preview_view.rs`, `crates/tabular_data_preview/src/tabular_data_preview.rs`, `assets/keymaps/default-macos.json`. | SVG, CSV-artige Inhalte und HTML werden durch den allgemeinen Resolver nicht als native Grafik-/Datentabellen-Ansicht gerendert. SVG und HTML zeigen einen externen Fallback; Explorer-Preview für Remote-Projektpfade meldet „unsupported“. Die vorhandene HTML-/URL-Aktion bietet den Systembrowser, keine eingebettete Webansicht. `crates/workspace/src/views/preview.rs`, `crates/panel-explorer/src/panel_explorer.rs`. |
| Rasterbild: Zoom, Pan, Metadaten und Einheiten | Zed hat ein eigenes Image Viewer-Item. Toolbar-Aktionen zoomen hinein/heraus, zeigen den editierbaren Zoom-Prozentsatz, passen ins Fenster; Rechtsklick auf Zoom setzt auf 100 %. Scrollrad schwenkt, modifiziertes Scrollrad und Pinch zoomen um den Pointer; Linksklick oder mittlere Taste ziehen das Bild. Weitere Actions sind `ResetZoom` und `ZoomToActualSize`. macOS bindet `Cmd-=`, `Cmd-+`, `Cmd--`, `Cmd-0`, `Cmd-1` und `Cmd-Shift-0` für Zoom/Reset/Originalgröße/Fit. Die Bildinfo zeigt Pixelmaße, Dateigröße, Farbkanäle/Bits pro Pixel und Format; `image_viewer.unit` ist standardmäßig binary, optional decimal. `zed-refrence/zed/crates/image_viewer/src/image_viewer.rs`, `image_info.rs`, `image_viewer_settings.rs`, `assets/keymaps/default-macos.json`, `assets/settings/default.json`. | `PreviewView` zeigt PNG/JPEG/GIF/WebP/BMP/ICO mit `object_fit: contain`. Eine Zoom-, Pan-, Originalgrößen-, Fit- oder Metadaten-/Einheitensteuerung ist in diesem View nicht vorhanden. `crates/workspace/src/views/preview.rs`. |
| Leerer, nicht unterstützter und fehlerhafter Inhalt | Vorschau-Item und Einstellungen sind typ-spezifisch; in den geprüften Markdown-/SVG-Abläufen wird das Markdown-Rendering im Hintergrund aktualisiert bzw. SVG in einem Background-Task gerendert. | Leere Adresse zeigt „No address“ und den Hinweis, eine Datei aus dem Explorer zu öffnen. Nicht unterstützte Dateien, Remote-URLs und PDF/SVG/HTML zeigen einen erklärenden externen Fallback mit Browser-/Systemaktion; Lesefehler zeigen „Preview unavailable“ und eine Notification mit Details. Der Resolver hat keinen sichtbaren Ladezustand. `crates/workspace/src/views/preview.rs`, `crates/workspace/src/workspace.rs`. |

## 7. Modale Modi, Keymap, Commands und Shortcuts

Zed macht fast jede Funktion als Action verfügbar. Die Keymap unterstützt Plattformprofile (Zed, VS Code, Atom, Emacs Beta, JetBrains, Sublime, TextMate, Cursor, None), kontextsensitive Bindings, Sequenzen, Prioritätsregeln, Konfliktdiagnose und Keymap-Editor. [Key Bindings](https://zed.dev/docs/key-bindings) · [All Actions](https://zed.dev/docs/all-actions)

| Zed-Funktion | Labonair-Gegenstück und Status | Nutzen | Verbundene Systeme und Verhalten |
|---|---|---|---|
| Editieren im nichtmodalen Standardmodus | **Vorhanden im Code.** | Standard-Eingabe mit textorientierten Shortcuts. | Editor Keymap, GPUI Input, Focus Context. |
| Vim-Modus mit Normal/Insert/Visual/Visual Line, Counted Motions, Operatoren, Registers, `/`/`?`, Ex-Kommandos | **Teilweise.** `crates/editor/src/vim.rs` dokumentiert und implementiert eine kompakte Vim-Untermenge inklusive `:w`, `:q`, `:wq`, `:e`, `:noh`, `:s`, `:set`; Zeds vollständige Vim-Integration und Action compatibility sind nicht erreicht. | Bestehende Vim-Muskelgedächtnis und modale Navigation nutzen. | Editor Settings, Cursor/Selection, Regex-Kompatibilität, Save/Close Bridge. [Zed Vim Mode](https://zed.dev/docs/vim) |
| Helix Mode mit selections-first/modal actions | **Lücke.** In Labonair wurde kein Helix-Modus gefunden. | Helix-Nutzer mit eigener Modalität und Auswahl-vor-Aktion arbeiten lassen. | Editor Mode State, Keymap Contexts, Selection Model und Command Palette. |
| Plattform-/Editor-Keymap-Profile und Wechsel zur Laufzeit | **Teilweise.** Labonair hat keymap owner, default Mac/Linux Ressourcen und Konfiguration; die Zed-Palette an kompatiblen Profiles und UI-Auswahl ist nicht belegt. | Bekannte Shortcuts aus anderem Editor übernehmen. | Keymap schema, OS, Settings UI, conflict resolution. |
| Keymap Editor mit Suche, Actionliste, „Change/Add Binding“, direkter JSON-Datei und Vorschau | **Teilweise.** Labonair besitzt Keymap UI/Manager; vollständiger Zed-Editorfluss samt konflikt- und kontextsensitivem Actionkatalog nicht belegt. | Shortcuts finden und ändern, ohne JSON von Hand zu pflegen. | Command Registry, Keymap parser, Key sequence runtime, Settings. |
| Kontextsensitive Bindings, mehrstufige Key-Sequenzen, Priorität und Timeout bei Präfix-Konflikt | **Teilweise.** `crates/keymap/src/runtime.rs` und `crates/keymap/src/file.rs` existieren; Verhalten gegen Zeds Kontextbaum, Präfix-Verzögerung und Konfliktvisualisierung ist nicht verglichen. | Gleiche Taste in Editor, Picker oder Terminal passend zur aktiven Fläche nutzen. | Focus tree, Context expression parser, timeout/cancel, pending-key indicator. |
| Which-key Popup, pending-keystrokes Anzeige, „Open Key Context View“ | **Lücke/nicht belegt.** | Multi-Stroke-Tastenkürzel auffindbar machen und aktive Kontexte nachvollziehen. | Keymap Runtime, Tooltip/Popup, Statusbar, Context Inspector. |
| Key Recording / Tastensequenz als Actions aufnehmen und senden | **Lücke/nicht belegt.** | Wiederkehrende Editierfolgen an eine Aktion oder ein Binding koppeln. | Keymap Editor, simulated input, sequence limit, asynchronous command boundaries. |
| Alle Editoraktionen auffindbar über Palette, Bindungen dokumentiert je macOS/Windows/Linux | **Teilweise.** Labonair besitzt EditorCommandProvider und Command Palette Registry. Die 585 in der gepinnten macOS-Keymap gebundenen Actions sind hier inventarisiert, aber nur teilweise einzeln zu lokalen Routen zugeordnet; ungebundene Registry-Actions und andere Plattformprofile bleiben offen. | Funktionen über Suche finden, selbst wenn Shortcut unbekannt ist. | Command Palette, CommandId, command contexts, Keymap und feature owner. |

### Umfang der gepinnten macOS-Actions

Im gepinnten `assets/keymaps/default-macos.json` stehen **164 verschiedene `editor::`-Actionnamen**, **14 `search::`**, **32 `git::`**, **13 `markdown::`**, **2 `svg::`**, **22 `pane::`** und **42 `workspace::`**-Actionnamen über alle Tastaturkontexte hinweg. Das zählt eindeutige Bezeichner mit mindestens einer Default-Bindung in dieser Datei; es ist weder die gesamte Zed-Actionregistry noch ein Laufzeittest. Die drei Kernindizes führen 289 Actionnamen aus sieben Namespaces mit den gepinnten Bindungen und Kontexten auf; ein Ergänzungsindex erfasst die übrigen 296 Namen aus 61 Namespaces und macht die gebundene Default-macOS-Keymap vollständig. Labonairs `EditorCommandProvider` deklariert 39 Palette-Beschreibungen, davon 36 ausführbare `Editor*`-Command-IDs. Textbewegung und Standardbearbeitung laufen zusätzlich über den GPUI-Editorpfad und sind in dieser Palette-Zahl nicht enthalten. Die Zahlen zeigen daher Rechercheumfang, keinen direkten Verhältnis- oder Paritätswert.

| Gepinnte Action-Familie (Beispiele aus macOS-Keymap) | Labonair-Gegenstück und Status | Nutzerzweck / verbundene Systeme |
|---|---|---|
| Bewegen, Auswählen und Löschen: `Move*`, `Select*`, `Delete*`, `Backspace`, Wort-/Subwort-/Absatz- und Seitenbewegung | **Teilweise.** `Motion` und Editieroperationen decken Grundbewegung/-auswahl ab; exakte Actionvarianten und Defaultbindungen sind nicht vollständig abgeglichen. | Cursor präzise ohne Maus positionieren; Text zu Wort, Zeile, Absatz oder Seite auswählen. DisplayMap, Unicode, Soft Wrap und Plattform-Keymap. |
| Mehrfachauswahl: `AddSelectionAbove/Below`, `SelectNext/Previous`, `SelectAllMatches`, `UndoSelection/RedoSelection` | **Teilweise.** Labonair unterstützt zusätzliche Cursor, nächstes/all Vorkommen und Auswahltransaktionen; „voriges Vorkommen“ und die vollständige Zed-Auswahl-/Undo-Familie sind nicht belegt. | Wiederholte Stellen gleichzeitig ändern und Auswahlzustände gezielt zurücknehmen. SelectionSet, Suchtreffer und Undo-Verlauf. |
| Struktur- und Zeilenbearbeitung: `SelectLarger/SmallerSyntaxNode`, `MoveLine*`, `DuplicateLine*`, `JoinLines`, `Rewrap`, Kommentar- und Einrückaktionen | **Teilweise.** Labonair hat expand/shrink, Zeilen duplizieren/bewegen, Vim-`J` zum Verbinden sowie Transpose-/Indent-/Kommentaroperationen; AST-Auswahl, allgemeiner Join-Command, Rewrap und Sprach-/Einrückungsparität offen. | Code und Text strukturiert anpassen. Tree-sitter, Language-Metadaten, Mehrfachcursor und Editor-Settings. |
| Ein-/Ausgabe und Zwischenablage: `Newline*`, `Cut/Copy/Paste`, `KillRingCut/Yank`, `ShowCharacterPalette` | **Teilweise.** Standard-Textbearbeitung ist Teil des Editorpfads; Kill Ring und Character Palette wurden nicht gefunden. | Text effizient eingeben, entfernen und wiederverwenden. GPUI Input, OS-Clipboard, IME und Mehrfachauswahl. |
| Vorschläge und modale Eingaben: Completion, Word Completion, Snippet-Tabstops, Signature Help, Edit Prediction und Accept Next Word/Line | **Teilweise.** Completion/Signature Help sind vorhanden; Word-Fallback, Tabstop-Navigation und Inline Edit Predictions sind nicht belegt. | API-Vorschläge annehmen, Parameter ansehen oder größere Vorschläge abschnittsweise prüfen. LSP, Snippets, Fokus, Vim-Kontext und Prediction-Provider. |
| Sprachaktionen: Definition/Declaration/Type Definition/Implementation, Peek/Split, References, Rename, Hover, Diagnostics, Format und Organize Imports | **Teilweise.** Ein Teil dieser Requests/Commands ist im Editor; Type Definition, Referenz-Multi-Buffer, alle Splitziele und Ergebnisabläufe fehlen oder sind nicht belegt. | Codebeziehungen folgen, Probleme beheben und Dokumente automatisiert formatieren. LSP, WorkspaceEdit, Navigation und File Lifecycle. |
| Struktur- und Anzeigeaktionen: Foldstufen/Rekursion, Fold All, Soft Wrap, Line Numbers, Inlay Hints, Cursorzentrierung | **Teilweise.** Symbolfaltung, Soft Wrap, Zeilennummern und Scrollen existieren; vollständige Fold-Aktionsfamilie und Inlay Hints fehlen. | Große Dateien fokussiert lesen und zusätzliche Typ-/Parameterhinweise anzeigen. Syntax-/LSP-Queries, DisplayMap, Minimap und Settings. |
| Suchaktionen: Match vor/zurück, Replace Next/All, Case, Whole Word, Regex, Selection und Ignored Files | **Teilweise.** Editor-Suche, Replace und mehrere Suchoptionen sind vorhanden; effektive globale Bindings, Ignore-Filter und Projekt-Replace sind nicht gleichwertig. | Schneller lokal und projektweit suchen und Treffer ändern. Search Overlay, Projektwurzeln, Ignore-Regeln und Buffertransaktionen. |
| Git- und Diffaktionen: Hunk vor/zurück, Änderungen, Stage/Unstage, Restore, Commit, Stash, Blame, Branch-/Remoteoperationen | **Teilweise, über mehrere Owner.** Editor bietet Git-Änderungsnavigation und Diff-Übergang; Stage/Commit/Stash/Blame sind dem Git-Modul zugeordnet und in diesem Editor-Command-Provider nicht zu zählen. | Änderungen im Kontext prüfen und Repositoryzustand bearbeiten. Git owner, Diff, Workspace Tabs und Notification Center. |
| Markdown-/SVG-Preview und Preview-Navigation | **Teilweise.** Markdown Preview hat eine eigene View; Following-/Side-by-Side-Parität ist offen. SVG öffnet Labonair extern statt als native Preview. | Formatierte Dokumentation oder Vektorgrafiken direkt im Workspace prüfen. Preview Resolver, Markdown Renderer, Workspace Split und OS-Dateihandler. |
| Excerpts und editierbare Multi-Buffers: `OpenSelectionsInMultibuffer`, `OpenExcerpts`, `ExpandExcerpts`, `OpenExcerptsSplit` | **Lücke im gleichwertigen Editor-Multi-Buffer.** Projektsuche zeigt Treffer und öffnet sie im regulären Editor; gemeinsame bearbeitbare Ausschnitte sind nicht belegt. | Treffer und Referenzen dateiübergreifend in einem editierbaren Arbeitsbereich vergleichen. Excerpt-Mapping, unabhängige Bufferrevisionen, Save/Undo und Konfliktbehandlung. |

### Gepinnte macOS-Keymap: alle gebundenen editor::-Actions

Diese Tabelle listet alle 164 eindeutigen editor::-Actionnamen aus der gepinnten default-macos.json mit sämtlichen Tastenfolgen und Kontextausdrücken. Die letzte Spalte ordnet jede Action der Labonair-Familie in der vorstehenden Crosswalk-Tabelle zu; sie behauptet keine 1:1-Semantik oder identische Bindung. Ein Kontext begrenzt die Bindung auf genau diesen UI-Zustand. Die Action-Bezeichner und Bindings stammen aus der Keymap, nicht aus einer Beobachtung eines laufenden Zed-Builds.

| Zed Action-ID | Gepinnter macOS-Default und Kontext | Labonair-Crosswalk-Familie |
|---|---|---|
| `editor::AcceptEditPrediction` | ⌥Tab (`Editor && edit_prediction`)<br>Tab (`Editor && edit_prediction && edit_prediction_mode == eager && !showing_completions`) | Vorschläge und modale Eingaben |
| `editor::AcceptNextLineEditPrediction` | ⌃⌘↓ (`Editor && edit_prediction`) | Vorschläge und modale Eingaben |
| `editor::AcceptNextWordEditPrediction` | ⌃⌘→ (`Editor && edit_prediction`) | Vorschläge und modale Eingaben |
| `editor::AddSelectionAbove` | ⌘⌃P (`Editor`)<br>⌘⌥↑ (`Editor`) | Mehrfachauswahl |
| `editor::AddSelectionBelow` | ⌘⌃N (`Editor`)<br>⌘⌥↓ (`Editor`) | Mehrfachauswahl |
| `editor::Backspace` | ⇧⌫ (`Editor`)<br>⌃H (`Editor`)<br>⌫ (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::Backtab` | ⇧Tab (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::BlameHover` | ⌘K ⌘B (`Editor`) | Git-/Diff-/Debuggeraktionen |
| `editor::Cancel` | Esc (`Editor`) | Vorschläge und modale Eingaben |
| `editor::ComposeCompletion` | Tab (`Editor && showing_completions`) | Vorschläge und modale Eingaben |
| `editor::ConfirmCodeAction` | ↩ (`Editor && showing_code_actions`) | Vorschläge und modale Eingaben |
| `editor::ConfirmCompletion` | ↩ (`Editor && showing_completions`) | Vorschläge und modale Eingaben |
| `editor::ConfirmCompletionReplace` | ⇧↩ (`Editor && showing_completions`) | Vorschläge und modale Eingaben |
| `editor::ConfirmRename` | ↩ (`Editor && renaming`) | Vorschläge und modale Eingaben |
| `editor::ContextMenuFirst` | PageUp (`Editor && (showing_code_actions \|\| showing_completions)`) | Vorschläge und modale Eingaben |
| `editor::ContextMenuLast` | PageDown (`Editor && (showing_code_actions \|\| showing_completions)`) | Vorschläge und modale Eingaben |
| `editor::ContextMenuNext` | ↓ (`Editor && (showing_code_actions \|\| showing_completions)`)<br>⌃N (`Editor && (showing_code_actions \|\| showing_completions)`) | Vorschläge und modale Eingaben |
| `editor::ContextMenuPrevious` | ↑ (`Editor && (showing_code_actions \|\| showing_completions)`)<br>⌃P (`Editor && (showing_code_actions \|\| showing_completions)`) | Vorschläge und modale Eingaben |
| `editor::Copy` | ⌘C (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::CopyPath` | ⌘K P (`Editor`) | Zusätzliche Editor-/OS-Aktionen |
| `editor::Cut` | ⌘X (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::Delete` | ⌃D (`Editor`)<br>⌦ (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::DeleteLine` | ⌘⇧K (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::DeleteToBeginningOfLine` | ⌘⌫ (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::DeleteToEndOfLine` | ⌘⌦ (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::DeleteToNextSubwordEnd` | ⌃⌥⌦ (`Editor`)<br>⌃⌥D (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::DeleteToNextWordEnd` | ⌥⌦ (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::DeleteToPreviousSubwordStart` | ⌃⌥⌫ (`Editor`)<br>⌃⌥H (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::DeleteToPreviousWordStart` | ⌥⌫ (`Editor`)<br>⌃W (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::DisplayCursorNames` | ⌃⌘C (`<global>`) | Zusätzliche Editor-/OS-Aktionen |
| `editor::DuplicateLineDown` | ⌥⇧↓ (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::DuplicateLineUp` | ⌥⇧↑ (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::EditLogBreakpoint` | ⇧F9 (`Editor`) | Git-/Diff-/Debuggeraktionen |
| `editor::ExpandAllDiffHunks` | ⌘" (`Editor`) | Git-/Diff-/Debuggeraktionen |
| `editor::ExpandExcerpts` | ⇧↩ (`!AcpThread > Editor && mode == full`) | Excerpts und Multi-Buffers |
| `editor::FindAllReferences` | ⌥⇧F12 (`Editor`) | Sprachaktionen |
| `editor::Fold` | ⌥⌘[ (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::FoldAll` | ⌘K ⌘0 (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::FoldAtLevel_1` | ⌘K ⌘1 (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::FoldAtLevel_2` | ⌘K ⌘2 (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::FoldAtLevel_3` | ⌘K ⌘3 (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::FoldAtLevel_4` | ⌘K ⌘4 (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::FoldAtLevel_5` | ⌘K ⌘5 (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::FoldAtLevel_6` | ⌘K ⌘6 (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::FoldAtLevel_7` | ⌘K ⌘7 (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::FoldAtLevel_8` | ⌘K ⌘8 (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::FoldAtLevel_9` | ⌘K ⌘9 (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::FoldRecursive` | ⌘K ⌘[ (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::Format` | ⌘⇧I (`Editor`) | Sprachaktionen |
| `editor::GoToDeclaration` | ⌃F12 (`Editor`) | Sprachaktionen |
| `editor::GoToDeclarationSplit` | ⌥⌃F12 (`Editor`) | Sprachaktionen |
| `editor::GoToDefinition` | F12 (`Editor`) | Sprachaktionen |
| `editor::GoToDefinitionSplit` | ⌥F12 (`Editor`) | Sprachaktionen |
| `editor::GoToDiagnostic` | F8 (`Editor`) | Sprachaktionen |
| `editor::GoToHunk` | ⌘F8 (`!AcpThread > Editor && mode == full`) | Git-/Diff-/Debuggeraktionen |
| `editor::GoToImplementation` | ⇧F12 (`Editor`) | Sprachaktionen |
| `editor::GoToNextChange` | ⌘⇧⌥⌫ (`Editor && mode == full`) | Git-/Diff-/Debuggeraktionen |
| `editor::GoToPreviousChange` | ⌘⇧⌫ (`Editor && mode == full`) | Git-/Diff-/Debuggeraktionen |
| `editor::GoToPreviousDiagnostic` | ⇧F8 (`Editor`) | Sprachaktionen |
| `editor::GoToPreviousHunk` | ⌘⇧F8 (`!AcpThread > Editor && mode == full`) | Git-/Diff-/Debuggeraktionen |
| `editor::GoToTypeDefinition` | ⌘F12 (`Editor`) | Sprachaktionen |
| `editor::GoToTypeDefinitionSplit` | ⌥⌘F12 (`Editor`) | Sprachaktionen |
| `editor::Hover` | ⌘K ⌘I (`Editor`) | Sprachaktionen |
| `editor::Indent` | ⌘] (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::JoinLines` | ⌃J (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::KillRingCut` | ⌃K (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::KillRingYank` | ⌃Y (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::LineDown` | ⌃PageDown (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::LineUp` | ⌃PageUp (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveDown` | ↓ (`Editor`)<br>⌃N (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveLeft` | ⌃B (`Editor`)<br>← (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveLineDown` | ⌥↓ (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::MoveLineUp` | ⌥↑ (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::MovePageDown` | PageDown (`Editor`)<br>⌃V (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MovePageUp` | PageUp (`Editor`)<br>⌃⇧V (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveRight` | ⌃F (`Editor`)<br>→ (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveToBeginning` | ⌘↑ (`Editor`)<br>⌘Home (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveToBeginningOfLine` | ⌘← (`Editor`)<br>⌃A (`Editor`)<br>Home (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveToEnclosingBracket` | ⌘\| (`Editor`)<br>⌃M (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveToEnd` | ⌘↓ (`Editor`)<br>⌘End (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveToEndOfLine` | ⌘→ (`Editor`)<br>⌃E (`Editor`)<br>End (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveToEndOfParagraph` | ⌃↓ (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveToNextSubwordEnd` | ⌃⌥→ (`Editor`)<br>⌃⌥F (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveToNextWordEnd` | ⌥→ (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveToPreviousSubwordStart` | ⌃⌥← (`Editor`)<br>⌃⌥B (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveToPreviousWordStart` | ⌥← (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveToStartOfExcerpt` | ⌘↑ (`Editor && multibuffer`) | Excerpts und Multi-Buffers |
| `editor::MoveToStartOfNextExcerpt` | ⌘↓ (`Editor && multibuffer`) | Excerpts und Multi-Buffers |
| `editor::MoveToStartOfParagraph` | ⌃↑ (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::MoveUp` | ↑ (`Editor`)<br>⌃P (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::Newline` | ⇧↩ (`Editor && mode == full`)<br>↩ (`Editor && mode == full`)<br>⌃↩ (`Editor && mode == auto_height`)<br>⇧↩ (`Editor && mode == auto_height`)<br>⌥↩ (`AgentFeedbackMessageEditor > Editor`)<br>↩ (`AcpThread > Editor && use_modifier_to_send`)<br>⌃↩ (`BufferSearchBar && !in_replace > Editor`)<br>⌃↩ (`BufferSearchBar \|\| ProjectSearchBar`)<br>⌃↩ (`ProjectSearchBar && !in_replace > Editor`)<br>↩ (`CommitEditor > Editor`)<br>↩ (`GitCommit > Editor && mode == auto_height`)<br>↩ (`ConfigureContextServerModal > Editor`)<br>↩ (`NotebookEditor > Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::NewlineAbove` | ⌘⇧↩ (`Editor && mode == full`) | Ein-/Ausgabe und Zwischenablage |
| `editor::NewlineBelow` | ⌘↩ (`Editor && mode == full`)<br>⌃⇧↩ (`Editor && mode == auto_height`) | Ein-/Ausgabe und Zwischenablage |
| `editor::NextEditPrediction` | ⌥Tab (`Editor && mode == full && edit_prediction`) | Vorschläge und modale Eingaben |
| `editor::NextSnippetTabstop` | Tab (`Editor && in_snippet && has_next_tabstop && !showing_completions`) | Vorschläge und modale Eingaben |
| `editor::OpenExcerpts` | ⌥↩ (`AcpThread > Editor && mode == full`)<br>⌥↩ (`!AcpThread > Editor && mode == full`)<br>⌥↩ (`OutlinePanel && not_editing`) | Excerpts und Multi-Buffers |
| `editor::OpenExcerptsSplit` | ⌘⌥↩ (`!AcpThread > Editor && mode == full`)<br>⌘⌥↩ (`OutlinePanel && not_editing`) | Excerpts und Multi-Buffers |
| `editor::OpenSelectionsInMultibuffer` | ⌥↩ (`Editor && mode == full`) | Excerpts und Multi-Buffers |
| `editor::OrganizeImports` | ⌥⇧O (`Editor`) | Sprachaktionen |
| `editor::Outdent` | ⌘[ (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::PageDown` | ⌘PageDown (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::PageUp` | ⌘PageUp (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::Paste` | ⌘V (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::PreviousEditPrediction` | ⌥⇧Tab (`Editor && mode == full && edit_prediction`) | Vorschläge und modale Eingaben |
| `editor::PreviousSnippetTabstop` | ⇧Tab (`Editor && in_snippet && has_previous_tabstop && !showing_completions`) | Vorschläge und modale Eingaben |
| `editor::Redo` | ⌘⇧Z (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::RedoSelection` | ⌘⇧U (`Editor`) | Mehrfachauswahl |
| `editor::Rename` | F2 (`Editor`) | Sprachaktionen |
| `editor::RevealInFileManager` | ⌘K R (`Editor`)<br>⌘K R (`ImageViewer`) | Zusätzliche Editor-/OS-Aktionen |
| `editor::Rewrap` | ⌘K ⌘Q (`Editor`)<br>⌘K Q (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::ScrollCursorCenter` | ⌃L (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::SelectAll` | ⌘A (`Editor`)<br>⌘A (`Terminal`) | Bewegen, Auswählen und Löschen |
| `editor::SelectAllMatches` | ⌘⇧L (`Editor`)<br>⌘F2 (`Editor`) | Mehrfachauswahl |
| `editor::SelectDown` | ⇧↓ (`Editor`)<br>⌃⇧N (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectEnclosingSymbol` | ⌘⌥E (`Editor && mode == full`) | Struktur- und Zeilenbearbeitung |
| `editor::SelectLargerSyntaxNode` | ⌘⌃→ (`Editor`)<br>⌃⇧→ (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::SelectLeft` | ⇧← (`Editor`)<br>⌃⇧B (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectLine` | ⌘L (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectNext` | ⌘D (`Editor`)<br>⌘K ⌘D (`Editor`) | Mehrfachauswahl |
| `editor::SelectNextSyntaxNode` | ⌘⌃↓ (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::SelectPageDown` | ⇧PageDown (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectPageUp` | ⇧PageUp (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectPrevious` | ⌃⌘D (`Editor`)<br>⌘K ⌃⌘D (`Editor`) | Mehrfachauswahl |
| `editor::SelectPreviousSyntaxNode` | ⌘⌃↑ (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::SelectRight` | ⇧→ (`Editor`)<br>⌃⇧F (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectSmallerSyntaxNode` | ⌘⌃← (`Editor`)<br>⌃⇧← (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::SelectToBeginning` | ⌘⇧↑ (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectToBeginningOfLine` | ⌘⇧← (`Editor`)<br>⇧Home (`Editor`)<br>⌃⇧A (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectToEnd` | ⌘⇧↓ (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectToEndOfLine` | ⌘⇧→ (`Editor`)<br>⇧End (`Editor`)<br>⌃⇧E (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectToEndOfParagraph` | ⌃⇧↓ (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectToNextSubwordEnd` | ⌃⌥⇧→ (`Editor`)<br>⌃⌥⇧F (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectToNextWordEnd` | ⌥⇧→ (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectToPreviousSubwordStart` | ⌃⌥⇧← (`Editor`)<br>⌃⌥⇧B (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectToPreviousWordStart` | ⌥⇧← (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectToStartOfExcerpt` | ⌘⇧↑ (`Editor && multibuffer`) | Excerpts und Multi-Buffers |
| `editor::SelectToStartOfNextExcerpt` | ⌘⇧↓ (`Editor && multibuffer`) | Excerpts und Multi-Buffers |
| `editor::SelectToStartOfParagraph` | ⌃⇧↑ (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::SelectUp` | ⇧↑ (`Editor`)<br>⌃⇧P (`Editor`) | Bewegen, Auswählen und Löschen |
| `editor::ShowCharacterPalette` | ⌃⌘Space (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::ShowCompletions` | ⌃Space (`Editor`) | Vorschläge und modale Eingaben |
| `editor::ShowEditPrediction` | ⌥Tab (`Editor && !edit_prediction`) | Vorschläge und modale Eingaben |
| `editor::ShowSignatureHelp` | ⌘I (`Editor`) | Vorschläge und modale Eingaben |
| `editor::ShowWordCompletions` | ⌃⇧Space (`Editor`) | Vorschläge und modale Eingaben |
| `editor::SignatureHelpNext` | ↓ (`Editor && showing_signature_help && !showing_completions`) | Vorschläge und modale Eingaben |
| `editor::SignatureHelpPrevious` | ↑ (`Editor && showing_signature_help && !showing_completions`) | Vorschläge und modale Eingaben |
| `editor::Tab` | Tab (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::ToggleBlockComments` | ⌘K ⌘/ (`Editor`)<br>⇧⌥A (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::ToggleBreakpoint` | F9 (`Editor`) | Git-/Diff-/Debuggeraktionen |
| `editor::ToggleCodeActions` | ⌘. (`Editor`) | Sprachaktionen |
| `editor::ToggleComments` | ⌘/ (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::ToggleEditPrediction` | ⌃⌘E (`Editor`) | Vorschläge und modale Eingaben |
| `editor::ToggleFold` | ⌘K ⌘L (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::ToggleFoldAll` | ⌘⇧↩ (`BufferSearchBar`) | Struktur- und Anzeigeaktionen |
| `editor::ToggleInlayHints` | ⌃: (`!AcpThread > Editor && mode == full`) | Struktur- und Anzeigeaktionen |
| `editor::ToggleLineNumbers` | ⌘; (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::ToggleSelectedDiffHunks` | ⌘' (`Editor`) | Git-/Diff-/Debuggeraktionen |
| `editor::ToggleSoftWrap` | ⌘K Z (`Editor && mode == full`) | Struktur- und Anzeigeaktionen |
| `editor::Transpose` | ⌃T (`Editor`) | Struktur- und Zeilenbearbeitung |
| `editor::Undo` | ⌘Z (`Editor`) | Ein-/Ausgabe und Zwischenablage |
| `editor::UndoSelection` | ⌘U (`Editor`) | Mehrfachauswahl |
| `editor::UnfoldAll` | ⌘K ⌘J (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::UnfoldLines` | ⌥⌘] (`Editor`) | Struktur- und Anzeigeaktionen |
| `editor::UnfoldRecursive` | ⌘K ⌘] (`Editor`) | Struktur- und Anzeigeaktionen |

Die Sammelfamilie **Zusätzliche Editor-/OS-Aktionen** umfasst DisplayCursorNames, CopyPath und RevealInFileManager; ihr nächster lokaler Owner und dessen effektive Bindung sind noch separat zu verifizieren. Debugger- und Git-Diffaktionen sind dem verbundenen System zugeordnet, weil sie nicht alle dem Editor-Owner gehören.

### Verbundene Zed-Keymap-Namespaces: Search, Git, Markdown und SVG

Diese ergänzende Tabelle erfasst die übrigen 61 in der macOS-Keymap gebundenen Actions, die direkt mit Editor-Suche, Repositoryarbeit und Vorschau zusammenhängen. Bindung und Kontext stammen aus derselben gepinnten Keymap; die letzte Spalte verweist auf den lokalen Owner und die Crosswalk-Familie oben. Das dokumentiert den Einstieg, keine Gleichheit der gesamten Arbeitsabläufe.

| Zed Action-ID | Gepinnter macOS-Default und Kontext | Labonair-Owner / Crosswalk |
|---|---|---|
| `git::Amend` | ⌘⇧↩ (`GitDiff > Editor`)<br>⌘⇧↩ (`CommitEditor > Editor`)<br>⌘⇧↩ (`GitPanel`)<br>⌘⇧↩ (`GitCommit > Editor && mode == auto_height`) | Git- und Diffaktionen / Git-Owner |
| `git::ApplyCurrentStash` | ⌃Space (`StashDiff > Editor`) | Git- und Diffaktionen / Git-Owner |
| `git::Blame` | ⌘⌥G B (`Editor`) | Git- und Diffaktionen / Git-Owner |
| `git::Cancel` | Esc (`GitPanel && CommitEditor`) | Git- und Diffaktionen / Git-Owner |
| `git::Commit` | ⌘↩ (`GitDiff > Editor`)<br>⌘↩ (`CommitEditor > Editor`)<br>⌘↩ (`GitPanel`)<br>⌘↩ (`GitCommit > Editor && mode == auto_height`) | Git- und Diffaktionen / Git-Owner |
| `git::Diff` | ⇧⌃D (`AcpThread > Editor`)<br>⌃G D (`GitPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::DropCurrentStash` | ⌃⇧⌫ (`StashDiff > Editor`) | Git- und Diffaktionen / Git-Owner |
| `git::ExpandCommitEditor` | ⇧Esc (`CommitEditor > Editor`) | Git- und Diffaktionen / Git-Owner |
| `git::Fetch` | ⌃G ⌃G (`GitPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::ForcePush` | ⌃G ⇧↑ (`GitPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::GenerateCommitMessage` | ⌥Tab (`CommitEditor > Editor`)<br>⌥Tab (`GitCommit > Editor && mode == auto_height`) | Git- und Diffaktionen / Git-Owner |
| `git::OpenModifiedFiles` | ⌘⌥G M (`Editor`) | Git- und Diffaktionen / Git-Owner |
| `git::PopCurrentStash` | ⌃⇧Space (`StashDiff > Editor`) | Git- und Diffaktionen / Git-Owner |
| `git::Pull` | ⌃G ↓ (`GitPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::PullRebase` | ⌃G ⇧↓ (`GitPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::Push` | ⌃G ↑ (`GitPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::Restore` | ⌘⌥Z (`Editor && !agent_diff && !AgentPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::RestoreAndNext` | ⌘⌥Z (`GitDiff > Editor`) | Git- und Diffaktionen / Git-Owner |
| `git::RestoreFile` | ⌫ (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br>⌦ (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br>⌘⌫ (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br>⌘⌦ (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git- und Diffaktionen / Git-Owner |
| `git::RestoreTrackedFiles` | ⌃G ⌫ (`GitPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::ReviewDiff` | ⌘⌥G R (`Editor`) | Git- und Diffaktionen / Git-Owner |
| `git::StageAll` | ⌘⌃Y (`GitDiff > Editor`)<br>⌘⌃Y (`GitPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::StageAndNext` | ⌘Y (`Editor && !agent_diff && !AgentPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::StageFile` | ⌘Y (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git- und Diffaktionen / Git-Owner |
| `git::StageRange` | ⇧Space (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git- und Diffaktionen / Git-Owner |
| `git::ToggleFillCommitEditor` | ⌥⇧Esc (`CommitEditor > Editor`) | Git- und Diffaktionen / Git-Owner |
| `git::ToggleStaged` | ⌘⌥Y (`Editor && !agent_diff && !AgentPanel`)<br>⌘⌥Y (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br>Space (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git- und Diffaktionen / Git-Owner |
| `git::TrashUntrackedFiles` | ⌃G ⇧⌫ (`GitPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::UnstageAll` | ⌘⌃⇧Y (`GitDiff > Editor`)<br>⌘⌃⇧Y (`GitPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::UnstageAndNext` | ⌘⇧Y (`Editor && !agent_diff && !AgentPanel`) | Git- und Diffaktionen / Git-Owner |
| `git::UnstageFile` | ⌘⇧Y (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git- und Diffaktionen / Git-Owner |
| `git::Worktree` | ⌘⌃W (`Workspace`) | Git- und Diffaktionen / Git-Owner |
| `markdown::CloseAndReturnToEditor` | ⌘⇧V (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::Copy` | ⌘C (`Markdown`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::CopyAsMarkdown` | ⌘C (`AgentPanel > Markdown`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::OpenPreview` | ⌘⇧V (`Editor && extension == md`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::OpenPreviewToTheSide` | ⌘K V (`Editor && extension == md`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::ScrollDown` | ↓ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::ScrollDownByItem` | ⌥↓ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::ScrollPageDown` | PageDown (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::ScrollPageUp` | PageUp (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::ScrollToBottom` | ⌘↓ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::ScrollToTop` | ⌘↑ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::ScrollUp` | ↑ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner |
| `markdown::ScrollUpByItem` | ⌥↑ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner |
| `search::FocusSearch` | ⌘F (`AcpThreadSearchBar`)<br>⌘F (`BufferSearchBar`)<br>⌘⇧F (`ProjectSearchBar`)<br>⌘F (`KeymapEditor`)<br>⌘F (`SettingsWindow`) | Suchaktionen / Search-Owner |
| `search::NextHistoryQuery` | ↓ (`BufferSearchBar && !in_replace > Editor`)<br>↓ (`ProjectSearchBar > Editor`) | Suchaktionen / Search-Owner |
| `search::PreviousHistoryQuery` | ↑ (`BufferSearchBar && !in_replace > Editor`)<br>↑ (`ProjectSearchBar > Editor`) | Suchaktionen / Search-Owner |
| `search::ReplaceAll` | ⌘↩ (`BufferSearchBar && in_replace > Editor`)<br>⌘↩ (`ProjectSearchBar && in_replace > Editor`) | Suchaktionen / Search-Owner |
| `search::ReplaceNext` | ↩ (`BufferSearchBar && in_replace > Editor`)<br>↩ (`ProjectSearchBar && in_replace > Editor`) | Suchaktionen / Search-Owner |
| `search::SelectAllMatches` | ⌥↩ (`BufferSearchBar`)<br>⌥↩ (`Pane`) | Suchaktionen / Search-Owner |
| `search::SelectNextMatch` | ↩ (`BufferSearchBar`)<br>⌘G (`Pane`) | Suchaktionen / Search-Owner |
| `search::SelectPreviousMatch` | ⇧↩ (`BufferSearchBar`)<br>⇧↩ (`BufferSearchBar && !in_replace > Editor`)<br>⌘⇧G (`Pane`) | Suchaktionen / Search-Owner |
| `search::ToggleCaseSensitive` | ⌥⌘C (`AcpThread`)<br>⌥⌘C (`Pane`) | Suchaktionen / Search-Owner |
| `search::ToggleIncludeIgnored` | ⌘⇧I (`FileFinder \|\| (FileFinder > Picker > Editor)`) | Suchaktionen / Search-Owner |
| `search::ToggleRegex` | ⌥⌘X (`AcpThread`)<br>⌥⌘G (`ProjectSearchBar`)<br>⌥⌘X (`ProjectSearchBar`)<br>⌥⌘G (`ProjectSearchView`)<br>⌥⌘X (`ProjectSearchView`)<br>⌥⌘X (`Pane`) | Suchaktionen / Search-Owner |
| `search::ToggleReplace` | ⌘⇧H (`ProjectSearchBar`)<br>⌘⇧H (`ProjectSearchView`)<br>⌘⇧H (`Pane`) | Suchaktionen / Search-Owner |
| `search::ToggleSelection` | ⌘⌥L (`BufferSearchBar`)<br>⌘⌥L (`Pane`) | Suchaktionen / Search-Owner |
| `search::ToggleWholeWord` | ⌥⌘W (`AcpThread`)<br>⌥⌘W (`Pane`) | Suchaktionen / Search-Owner |
| `svg::OpenPreview` | ⌘⇧V (`Editor && extension == svg`) | Markdown-/SVG-Preview / SVG derzeit extern |
| `svg::OpenPreviewToTheSide` | ⌘K V (`Editor && extension == svg`) | Markdown-/SVG-Preview / SVG derzeit extern |

### Gepinnte macOS-Keymap: Pane- und Workspace-Actions

Die folgenden **64 eindeutigen `pane::`- und `workspace::`-Actions** sind in `default-macos.json` gebunden (22 Pane-, 42 Workspace-Actions). Die Actionargumente aus dem Quellformat stehen als `args=…` am Binding. Kontextausdrücke sind Quellwerte; ein Binding ist nur in diesem UI-Kontext aktiv. Die letzte Spalte vergleicht mit Labonairs registrierten Commands und Default-Keymap, nicht nur mit dem Editor-Provider.

| Zed Action-ID | Gepinnter macOS-Default und Kontext | Labonair-Crosswalk |
|---|---|---|
| `pane::ActivateItem` | `ctrl-1 args=0` (`Pane`)<br>`ctrl-2 args=1` (`Pane`)<br>`ctrl-3 args=2` (`Pane`)<br>`ctrl-4 args=3` (`Pane`)<br>`ctrl-5 args=4` (`Pane`)<br>`ctrl-6 args=5` (`Pane`)<br>`ctrl-7 args=6` (`Pane`)<br>`ctrl-8 args=7` (`Pane`)<br>`ctrl-9 args=8` (`Pane`) | Ähnlich sind Labonairs tab::Select1–Select9 (⌘1–⌘9), aber sie wählen globale Workspace-Tabs. Zed wählt ein Item im aktiven Pane (⌃1–⌃9). |
| `pane::ActivateLastItem` | `ctrl-0` (`Pane`) | Kein entsprechender Last-Item-Befehl in der lokalen Command Registry oder macOS-Keymap gefunden. |
| `pane::ActivateNextItem` | `alt-cmd-right` (`Pane`)<br>`cmd-}` (`Pane`)<br>`ctrl-tab` (`RunModal`) | Labonair tab::Next nutzt ebenfalls ⌃Tab; es wechselt jedoch Shell-Tabs statt Items innerhalb eines Zed-Panes. |
| `pane::ActivatePreviousItem` | `alt-cmd-left` (`Pane`)<br>`cmd-{` (`Pane`)<br>`ctrl-shift-tab` (`RunModal`) | Labonair tab::Prev nutzt ebenfalls ⇧⌃Tab; es wechselt Shell-Tabs statt Items innerhalb eines Zed-Panes. |
| `pane::CloseActiveItem` | `cmd-w args={"close_pinned":false}` (`Pane`) | Labonair tab::Close (⌘W) schließt den aktiven Workspace-Tab; Zeds Pane-Aktion gilt für das aktive Pane-Item und kann andere View-Arten umfassen. |
| `pane::CloseAllItems` | `cmd-k w args={"close_pinned":false}` (`Pane`) | Kein Befehl zum Schließen aller Items des aktiven Panes gefunden. |
| `pane::CloseCleanItems` | `cmd-k u args={"close_pinned":false}` (`Pane`) | Kein Befehl zum Schließen nur sauberer (unveränderter) Items gefunden. |
| `pane::CloseItemsToTheLeft` | `cmd-k e args={"close_pinned":false}` (`Pane`) | Kein Befehl zum Schließen der Pane-Items links des aktiven Items gefunden. |
| `pane::CloseItemsToTheRight` | `cmd-k t args={"close_pinned":false}` (`Pane`) | Kein Befehl zum Schließen der Pane-Items rechts des aktiven Items gefunden. |
| `pane::CloseOtherItems` | `alt-cmd-t args={"close_pinned":false}` (`Pane`) | Nächster Treffer ist tab::CloseOthers. Der lokale Befehl hat keinen gepinnten-Tab-Zustand; exakte Schließ- und Dirty-Tab-Semantik bleibt abzugleichen. |
| `pane::DeploySearch` | `cmd-shift-f` (`Pane`)<br>`cmd-shift-f` (`Workspace`)<br>`cmd-shift-h args={"replace_enabled":true}` (`Workspace`) | Labonairs search::Toggle / Find öffnet die aktuelle Suche mit ⌘F. Zeds ⌘⇧F und replace_enabled-Konfiguration eröffnen einen anderen Scope und Zustand. |
| `pane::GoBack` | `ctrl--` (`AcpThread`)<br>`ctrl--` (`ThreadHistory`)<br>`ctrl--` (`Pane`) | Kein Editor-/Pane-Navigationsverlauf-Befehl in der lokalen Keymap gefunden. |
| `pane::GoForward` | `ctrl-_` (`Pane`) | Kein Editor-/Pane-Vorwärtsverlauf-Befehl in der lokalen Keymap gefunden. |
| `pane::ReopenClosedItem` | `cmd-shift-t` (`Workspace`) | Kein zuletzt geschlossenes Item erneut öffnen. Sitzungs-Wiederherstellung für Tabs ist kein gleicher Undo-Schließvorgang. |
| `pane::RevealInProjectPanel` | `cmd-shift-e` (`!AcpThread > Editor && mode == full`) | sidebar::Toggle (⌘B) zeigt/versteckt den lokalen Datei-Explorer; eine Aktion zum ausgewählten Editorfile im Explorer aufklappen/markieren wurde nicht gefunden. |
| `pane::SplitDown` | `cmd-k down` (`Pane`)<br>`ctrl-alt-down` (`Terminal`) | Lokales pane::SplitDown (⌘⇧D) ist im Terminal-Kontext verfügbar; EditorSplitDown teilt die interne Editorgruppe. Zed bietet Pane-Splits als Workspace-Aktion. |
| `pane::SplitLeft` | `cmd-k left` (`Pane`)<br>`ctrl-alt-left` (`Terminal`) | Kein linker Workspace-Pane-Split-Befehl gefunden. |
| `pane::SplitRight` | `cmd-\` (`Editor`)<br>`cmd-k right` (`Pane`)<br>`ctrl-alt-right` (`Terminal`)<br>`cmd-d` (`Terminal`) | Lokales pane::SplitRight (⌘D) ist im Terminal-Kontext verfügbar; EditorSplitRight teilt die interne Editorgruppe (⌘⌥]). Die Oberflächen und Scopes unterscheiden sich. |
| `pane::SplitUp` | `cmd-k up` (`Pane`)<br>`ctrl-alt-up` (`Terminal`) | Kein oberer Workspace-Pane-Split-Befehl gefunden. |
| `pane::SwapItemLeft` | `ctrl-shift-pageup` (`Pane`) | Keine Tastaturaktion zum Umordnen eines Pane-Items links gefunden; visuelle Drag-Interaktion ist hier nicht bewertet. |
| `pane::SwapItemRight` | `ctrl-shift-pagedown` (`Pane`) | Keine Tastaturaktion zum Umordnen eines Pane-Items rechts gefunden; visuelle Drag-Interaktion ist hier nicht bewertet. |
| `pane::TogglePinTab` | `cmd-k shift-enter` (`Pane`) | Kein Pin-/Unpin-Tab-Zustand oder zugehöriger lokaler Command gefunden. |
| `workspace::ActivatePane` | `cmd-1 args=0` (`Workspace`)<br>`cmd-2 args=1` (`Workspace`)<br>`cmd-3 args=2` (`Workspace`)<br>`cmd-4 args=3` (`Workspace`)<br>`cmd-5 args=4` (`Workspace`)<br>`cmd-6 args=5` (`Workspace`)<br>`cmd-7 args=6` (`Workspace`)<br>`cmd-8 args=7` (`Workspace`)<br>`cmd-9 args=8` (`Workspace`) | Labonair cmd-1…cmd-9 wählt Workspace-Tabs (tab::Select1–Select9); Zed cmd-1…cmd-9 aktiviert das jeweilige Pane. Gleiches Shortcutmuster, anderer Container. |
| `workspace::ActivatePaneDown` | `cmd-k cmd-down` (`Workspace`) | Lokales pane::FocusNext (⌘]) rotiert durch Panes; eine richtungsbezogene Aktivierung nach unten fehlt. |
| `workspace::ActivatePaneLeft` | `cmd-k cmd-left` (`Workspace`) | Lokales pane::FocusNext (⌘]) rotiert durch Panes; eine richtungsbezogene Aktivierung nach links fehlt. |
| `workspace::ActivatePaneRight` | `cmd-k cmd-right` (`Workspace`) | Lokales pane::FocusNext (⌘]) rotiert durch Panes; eine richtungsbezogene Aktivierung nach rechts fehlt. |
| `workspace::ActivatePaneUp` | `cmd-k cmd-up` (`Workspace`) | Lokales pane::FocusNext (⌘]) rotiert durch Panes; eine richtungsbezogene Aktivierung nach oben fehlt. |
| `workspace::AddFolderToProject` | `cmd-shift-a` (`RecentProjects \|\| (RecentProjects > Picker > Editor)`) | Kein Add-Folder-to-Project-Befehl in der lokalen Command Registry gefunden; Projekt öffnen und Standalone-Modus wechseln sind andere Abläufe. |
| `workspace::CloseActiveDock` | `cmd-w` (`Workspace`) | ⌘W ist lokal tab::Close; sidebar::Toggle schaltet nur den Datei-Explorer. Kein allgemeiner Close-Active-Dock-Befehl gefunden. |
| `workspace::CloseAllItemsAndPanes` | `cmd-k cmd-w` (`Pane`) | Kein Workspace-Befehl zum Schließen aller Items und Panes gefunden. |
| `workspace::CloseInactiveTabsAndPanes` | `ctrl-alt-cmd-w` (`Pane`) | tab::CloseOthers schließt andere Tabs; ein gemeinsames Schließen inaktiver Tabs und Panes ist nicht belegt. |
| `workspace::CloseWindow` | `cmd-shift-w` (`<global>`)<br>`cmd-w` (`SettingsWindow`)<br>`escape` (`SettingsWindow`)<br>`cmd-w` (`SkillCreator`)<br>`cmd-w` (`SkillCreator > Editor`) | Kein entsprechendes lokales Window-Close-Keybinding in der User-Keymap. Native Fensterschließung und ⌘Q (App beenden) sind andere Aktionen. |
| `workspace::CopyPath` | `cmd-alt-c` (`OutlinePanel && not_editing`)<br>`cmd-alt-c` (`ProjectPanel`)<br>`cmd-alt-c` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Kein Copy-Path-Command oder Keybinding im lokalen Workspace-/Command-Palette-Owner gefunden. |
| `workspace::CopyRelativePath` | `alt-cmd-shift-c` (`OutlinePanel && not_editing`)<br>`alt-cmd-shift-c` (`ProjectPanel`)<br>`alt-cmd-shift-c` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Kein Copy-Relative-Path-Command oder Keybinding im lokalen Workspace-/Command-Palette-Owner gefunden. |
| `workspace::DecreaseActiveDockSize` | `ctrl-alt-- args={"px":0}` (`Workspace`) | Kein Dock-Größenbefehl in der lokalen Command Registry oder Keymap gefunden. |
| `workspace::DecreaseOpenDocksSize` | `ctrl-alt-_ args={"px":0}` (`Workspace`) | Kein gemeinsamer Größenbefehl für geöffnete Docks in der lokalen Command Registry oder Keymap gefunden. |
| `workspace::FocusNextPart` | `f6` (`Workspace`)<br>`cmd-f6` (`Workspace`) | pane::FocusNext rotiert nur durch Workspace-Panes; Fokusnavigation über Editor, Panels und Statusbar (⌘F6/F6) ist nicht belegt. |
| `workspace::FocusPreviousPart` | `shift-f6` (`Workspace`) | Kein Fokus-zurück-über-Fensterbereiche-Befehl gefunden. |
| `workspace::FollowNextCollaborator` | `ctrl-alt-cmd-f` (`<global>`) | Kein Collaborator-Follow-Befehl oder kollaborativer Workspace-Command gefunden. |
| `workspace::IncreaseActiveDockSize` | `ctrl-alt-= args={"px":0}` (`Workspace`) | Kein Dock-Größenbefehl in der lokalen Command Registry oder Keymap gefunden. |
| `workspace::IncreaseOpenDocksSize` | `ctrl-alt-+ args={"px":0}` (`Workspace`) | Kein gemeinsamer Größenbefehl für geöffnete Docks in der lokalen Command Registry oder Keymap gefunden. |
| `workspace::NewFile` | `cmd-n` (`Workspace && !Terminal`)<br>`cmd-n` (`Welcome`) | Der lokale Explorer bietet „New File“ im Kontextmenü (`crates/panel-explorer/src/panel_explorer.rs`); ein Palette-Command oder Default-Keybinding dafür fehlt. Zed bietet zusätzlich den direkten Workspace-Shortcut cmd-n. |
| `workspace::NewTerminal` | `ctrl-~` (`Workspace`)<br>`cmd-n` (`Terminal`) | NewTerminalTab (⌘T) erstellt lokal einen Terminal-Workspace-Tab; Zed cmd-n ist im Terminal-Kontext und erzeugt ein neues Terminal-Item. |
| `workspace::NewWindow` | `cmd-shift-n` (`Workspace`) | Kein lokaler New-Window-Command oder anpassbares Keybinding gefunden. |
| `workspace::Open` | `cmd-o` (`<global>`) | OpenFile (⌘⇧O) bietet lokale Dateiöffnung; Zed workspace::Open ist auf ⌘O gebunden. Gleicher Grundworkflow, anderer Shortcut und Action-Owner. |
| `workspace::OpenWithSystem` | `ctrl-shift-enter` (`ProjectPanel`)<br>`ctrl-shift-enter` (`InvalidBuffer`) | Kein Dateimanager-Open-With-System-Command gefunden. Die Preview-Ansicht kann bestimmte URLs extern öffnen, das ist kein ProjectPanel-Datei-Workflow. |
| `workspace::ReopenLastPicker` | `cmd-k cmd-p` (`Workspace`) | Kein Reopen-Last-Picker-Befehl gefunden; ⌘P toggelt lokal die Command Palette, nicht den zuletzt verwendeten Picker. |
| `workspace::ResetActiveDockSize` | `ctrl-alt-0` (`Workspace`) | Kein Reset für die Größe eines aktiven Docks gefunden. |
| `workspace::ResetOpenDocksSize` | `ctrl-alt-)` (`Workspace`) | Kein Reset für die Größen aller offenen Docks gefunden. |
| `workspace::Save` | `cmd-s` (`Workspace`) | tab::Save (⌘S) ist der direkte lokale Save-Einstieg; Laufzeit-/Dirty-Buffer-Abdeckung bleibt UI-seitig zu vergleichen. |
| `workspace::SaveAll` | `cmd-alt-s` (`Workspace`) | Kein Save-All-Befehl oder Keybinding gefunden. |
| `workspace::SaveAs` | `cmd-shift-s` (`Workspace`) | Kein Save-As-Befehl oder Keybinding gefunden. |
| `workspace::SaveWithoutFormat` | `cmd-k s` (`Workspace`) | Kein Save-Without-Format-Befehl gefunden; lokales Save hat keinen getrennten Formatting-Bypass-Shortcut. |
| `workspace::SwapPaneDown` | `cmd-k shift-down` (`Workspace`) | Keine Tastaturaktion zum Umordnen von Workspace-Panes nach unten gefunden. |
| `workspace::SwapPaneLeft` | `cmd-k shift-left` (`Workspace`) | Keine Tastaturaktion zum Umordnen von Workspace-Panes nach links gefunden. |
| `workspace::SwapPaneRight` | `cmd-k shift-right` (`Workspace`) | Keine Tastaturaktion zum Umordnen von Workspace-Panes nach rechts gefunden. |
| `workspace::SwapPaneUp` | `cmd-k shift-up` (`Workspace`) | Keine Tastaturaktion zum Umordnen von Workspace-Panes nach oben gefunden. |
| `workspace::ToggleAllDocks` | `alt-cmd-y` (`Workspace`) | Kein Toggle für alle Docks in der lokalen Command Registry oder Keymap gefunden. |
| `workspace::ToggleBottomDock` | `cmd-j` (`Workspace`) | Kein Bottom-Dock-Toggle-Keybinding gefunden; lokale Panels werden über eigene Oberflächen/Statusbar kontrolliert. |
| `workspace::ToggleLeftDock` | `cmd-b` (`Workspace`) | sidebar::Toggle (⌘B) schaltet den Datei-Explorer links; die lokale Sidebar ist nicht deckungsgleich mit Zeds frei belegbarem Left Dock. |
| `workspace::ToggleRightDock` | `cmd-alt-b` (`Workspace`)<br>`cmd-r` (`Workspace`) | Kein Right-Dock-Toggle-Befehl oder Keybinding gefunden. |
| `workspace::ToggleWorktreeSecurity` | `ctrl-cmd-s` (`<global>`) | Kein Worktree-Security-Toggle in der lokalen Command Registry oder Keymap gefunden. |
| `workspace::ToggleZoom` | `shift-escape` (`<global>`) | Ähnlich sind view::ToggleZenMode (⌘⇧Z) und der debug-only Dock-Zoom. Kein regulärer Zoom des aktiven Workspace-Panes ist belegt. |
| `workspace::Unfollow` | `escape` (`Workspace`) | Kein Collaboration-Unfollow-Command gefunden. |


### Vollständige übrige gepinnte macOS-Keymap: 296 Actions

Die vorigen drei Tabellen erfassen 289 Actionnamen aus sieben Editor-, Such-, Git-, Preview-, Pane- und Workspace-Namespaces. Diese Ergänzung erfasst die **übrigen 296 eindeutigen gebundenen Actionnamen aus 61 Namespaces**; zusammen sind damit alle **585** namespace-qualifizierten Actionnamen der gepinnten `default-macos.json` inventarisiert. Das ist eine vollständige Liste der in dieser Datei gebundenen Actions, nicht der gesamten Zed-Actionregistry, und beweist weder Laufzeitverfügbarkeit noch Parität. Schlüssel und Kontextausdrücke entsprechen den JSONC-Quellwerten; `args=…` bewahrt Actionargumente.

Die erste Tabelle ordnet die zusätzlichen Namespaces dem nächstliegenden Labonair-Owner zu. Der folgende Index listet danach jeden einzelnen Bezeichner und alle Bindings auf. Das Vergleichsurteil ist auf System-/Owner-Ebene; eine genaue semantische Route bleibt für jeden Eintrag zu verifizieren.

| Verbundener Bereich | Zed Namespaces | Gebundene Actions | Labonair Owner und Crosswalk-Status |
|---|---|---:|---|
| Projekt-, Datei-, Suche- und Sprachtools | `buffer_search`, `call_hierarchy`, `diagnostics`, `encoding_selector`, `file_finder`, `go_to_line`, `language_selector`, `lsp_tool`, `multi_workspace`, `outline`, `outline_panel`, `project_panel`, `project_search`, `project_symbols`, `projects`, `recent_projects`, `text_finder` | 53 | Lokale Owners: Workspace, Panel Explorer, Search und Editor/Language Services. Datei-Explorer, Buffer-Suche, Projekt-Suche, Outline und Diagnostics besitzen Teilflächen; Projekt-Symbolsuche, Zeds Pickerzustände, dynamische Language Extension Auswahl, Encoding Selector und Call Hierarchy sind nicht als gleichwertige Gesamtabläufe belegt. |
| Git und Change Review | `branch_picker`, `branches`, `git_graph`, `git_panel`, `git_picker`, `stash_picker`, `worktree_picker` | 24 | Lokale Owners: Git, Panel SCM, Panel Git Graph und Workspace/Hosts. Status, Branchwechsel, Graph und Diff sind vorhanden; Zed-Pickerzustände und alle Stash-/Worktree-/Branch-Verläufe bleiben einzeln zu vergleichen. |
| Terminal, Tasks, Debug und REPL | `console`, `debug_panel`, `debugger`, `dev`, `new_process_modal`, `repl`, `task`, `terminal`, `terminal_panel`, `variable_list` | 54 | Lokale Owners: Terminal und Workspace. Terminal-Tab, Shell-Prozess und Panel sind vorhanden; strukturierte Task-Ausführung, DAP-Debugger, Variablen/Watch, REPL und Debugger-Launch-Konfiguration fehlen oder sind nicht gleichwertig belegt. `dev::` enthält auch Entwickler-/Overlay- und Edit-Prediction-History-Actions. |
| AI und Zusammenarbeit | `agent`, `agents_sidebar`, `assistant`, `channel_modal`, `collab_panel`, `inline_assistant`, `skill_creator`, `zeta` | 78 | Lokale Owners: AI/MCP und Workspace. Labonair hat AI-/MCP-Flächen; vollständige Agent Session, Toolfreigabe im Editor, Inline Edit Prediction/Review, Skills und Echtzeit-Collaboration sind keine bestätigte Zed-Parität. |
| Dokumentansichten, Editor-Konfiguration und Keymap | `edit_prediction`, `image_viewer`, `keymap_editor`, `keystroke_input`, `notebook`, `picker`, `settings_editor`, `settings_profile_selector`, `tab_switcher`, `tabular_data`, `theme`, `theme_selector`, `toolchain`, `zed` | 71 | Lokale Owners: Editor, Workspace Preview, Settings, Theme und Keymap. Native Text-/Bildpreview, Settings UI/JSON, Theme- und Keymap-Editor sind vorhanden; Notebook/Kernel und vollständige Profile-, Extension-, Toolchain-, Prediction- und Keymap-Kontext-Workflows bleiben offen. `zed::` enthält zusätzlich OS-/Fensteraktionen. |
| App-Menü, Start und transiente Oberflächen | `command_palette`, `menu`, `onboarding`, `toast`, `welcome` | 16 | Lokale Owners: Command Palette, Shell-Menü, Welcome/Workspace und Notification Center. Palette und App-Einstiege existieren; Zed-Menü-/Pickerinteraktion, Onboarding, Toast-Action und Welcome-Abläufe sind nicht als Editor-Gleichheit bewertet. |

#### Projekt-, Datei-, Suche- und Sprachtools

| Zed Action-ID | Gepinnter macOS-Default und Kontext |
|---|---|
| `buffer_search::Deploy` | `cmd-f` (`Editor && mode == full`)<br>`cmd-alt-l args={"selection_search_enabled":true}` (`Editor && mode == full`)<br>`cmd-f` (`Terminal`)<br>`cmd-f` (`MarkdownPreview`) |
| `buffer_search::Dismiss` | `escape` (`BufferSearchBar`) |
| `buffer_search::FocusEditor` | `tab` (`BufferSearchBar`) |
| `buffer_search::UseSelectionForFind` | `cmd-e` (`Editor && mode == full`) |
| `call_hierarchy::ShowIncomingCalls` | `cmd-k cmd-h` (`Editor`) |
| `call_hierarchy::ToggleDirection` | `cmd-k cmd-h` (`CallHierarchyPicker > Picker > Editor`) |
| `diagnostics::Deploy` | `cmd-shift-m` (`Workspace`) |
| `diagnostics::ToggleDiagnosticsRefresh` | `ctrl-r` (`Diagnostics`) |
| `encoding_selector::Toggle` | `cmd-k n` (`Workspace`) |
| `file_finder::Toggle` | `cmd-p` (`Workspace`) |
| `go_to_line::Toggle` | `ctrl-g` (`Editor && mode == full`) |
| `language_selector::Toggle` | `cmd-k m` (`Workspace`) |
| `lsp_tool::ToggleMenu` | `ctrl-cmd-l` (`<global>`) |
| `multi_workspace::FocusWorkspaceSidebar` | `cmd-alt-;` (`Workspace`) |
| `multi_workspace::ToggleWorkspaceSidebar` | `cmd-alt-j` (`Workspace`) |
| `outline::Toggle` | `cmd-shift-o` (`BufferSearchBar`)<br>`cmd-shift-o` (`Editor && mode == full`) |
| `outline_panel::CollapseSelectedEntry` | `left` (`OutlinePanel && not_editing`) |
| `outline_panel::ExpandSelectedEntry` | `right` (`OutlinePanel && not_editing`) |
| `outline_panel::OpenSelectedEntry` | `space` (`OutlinePanel && not_editing`) |
| `outline_panel::RevealInFileManager` | `alt-cmd-r` (`OutlinePanel && not_editing`) |
| `outline_panel::ToggleFocus` | `cmd-shift-b` (`Workspace`) |
| `project_panel::CollapseAllEntries` | `cmd-left` (`ProjectPanel`) |
| `project_panel::CollapseSelectedEntry` | `left` (`ProjectPanel`) |
| `project_panel::CompareMarkedFiles` | `alt-d` (`ProjectPanel`) |
| `project_panel::Copy` | `cmd-c` (`ProjectPanel`) |
| `project_panel::Cut` | `cmd-x` (`ProjectPanel`) |
| `project_panel::Delete` | `cmd-delete args={"skip_prompt":false}` (`ProjectPanel`)<br>`cmd-alt-backspace args={"skip_prompt":false}` (`ProjectPanel`) |
| `project_panel::Duplicate` | `cmd-d` (`ProjectPanel`) |
| `project_panel::ExpandAllEntries` | `cmd-right` (`ProjectPanel`) |
| `project_panel::ExpandSelectedEntry` | `right` (`ProjectPanel`) |
| `project_panel::NewDirectory` | `alt-cmd-n` (`ProjectPanel`) |
| `project_panel::NewFile` | `cmd-n` (`ProjectPanel`) |
| `project_panel::NewSearchInDirectory` | `cmd-alt-shift-f` (`ProjectPanel`) |
| `project_panel::Open` | `space` (`ProjectPanel && not_editing`) |
| `project_panel::Paste` | `cmd-v` (`ProjectPanel`) |
| `project_panel::Redo` | `cmd-shift-z` (`ProjectPanel`) |
| `project_panel::Rename` | `enter` (`ProjectPanel`)<br>`f2` (`ProjectPanel`) |
| `project_panel::RevealInFileManager` | `alt-cmd-r` (`ProjectPanel`) |
| `project_panel::ToggleFocus` | `cmd-shift-e` (`AgentPanel`)<br>`cmd-shift-e` (`Workspace`) |
| `project_panel::Trash` | `backspace args={"skip_prompt":false}` (`ProjectPanel`)<br>`delete args={"skip_prompt":false}` (`ProjectPanel`)<br>`cmd-backspace args={"skip_prompt":true}` (`ProjectPanel`) |
| `project_panel::Undo` | `cmd-z` (`ProjectPanel`) |
| `project_search::OpenTextFinder` | `alt-cmd-f` (`ProjectSearchBar`)<br>`alt-cmd-f` (`ProjectSearchView`) |
| `project_search::SearchInNew` | `cmd-enter` (`ProjectSearchBar && !in_replace`) |
| `project_search::ToggleAllSearchResults` | `cmd-shift-enter` (`ProjectSearchBar`)<br>`cmd-shift-enter` (`ProjectSearchView`) |
| `project_search::ToggleFilters` | `cmd-shift-j` (`ProjectSearchBar`)<br>`cmd-shift-j` (`ProjectSearchView`) |
| `project_search::ToggleFocus` | `escape` (`ProjectSearchBar`)<br>`escape` (`ProjectSearchView`)<br>`cmd-f` (`Pane`) |
| `project_symbols::Toggle` | `cmd-t` (`Workspace`) |
| `projects::OpenRecent` | `alt-cmd-o` (`Workspace`)<br>`ctrl-r` (`Workspace`) |
| `projects::OpenRemote` | `ctrl-cmd-o args={"from_existing_connection":false}` (`Workspace`)<br>`ctrl-cmd-shift-o args={"from_existing_connection":true}` (`Workspace`) |
| `recent_projects::AddToWorkspace` | `cmd-shift-enter` (`RecentProjects \|\| (RecentProjects > Picker > Editor)`) |
| `recent_projects::RemoveSelected` | `shift-backspace` (`RecentProjects \|\| (RecentProjects > Picker > Editor)`) |
| `recent_projects::ToggleActionsMenu` | `cmd-k` (`RecentProjects \|\| (RecentProjects > Picker > Editor)`) |
| `text_finder::Toggle` | `alt-cmd-f` (`Editor && mode == full`)<br>`alt-cmd-f` (`BufferSearchBar`) |

#### Git und Change Review

| Zed Action-ID | Gepinnter macOS-Default und Kontext |
|---|---|
| `branch_picker::CycleBranchFilter` | `cmd-shift-i` (`GitBranchSelector \|\| (GitBranchSelector > Picker > Editor)`) |
| `branch_picker::DeleteBranch` | `cmd-shift-backspace` (`GitBranchSelector \|\| (GitBranchSelector > Picker > Editor)`) |
| `branch_picker::ForceDeleteBranch` | `cmd-alt-shift-backspace` (`GitBranchSelector \|\| (GitBranchSelector > Picker > Editor)`) |
| `branch_picker::ToggleFilterMenu` | `cmd-k` (`GitBranchSelector \|\| (GitBranchSelector > Picker > Editor)`) |
| `branches::OpenRecent` | `cmd-ctrl-b` (`Workspace`) |
| `git_graph::FocusNextTabStop` | `tab` (`GitGraph`)<br>`tab` (`GitGraphSearchBar > Editor`) |
| `git_graph::FocusPreviousTabStop` | `shift-tab` (`GitGraph`)<br>`shift-tab` (`GitGraphSearchBar > Editor`) |
| `git_panel::ActivateChangesTab` | `cmd-1` (`GitPanel`) |
| `git_panel::ActivateHistoryTab` | `cmd-2` (`GitPanel`) |
| `git_panel::CollapseSelectedEntry` | `left` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) |
| `git_panel::ExpandSelectedEntry` | `right` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) |
| `git_panel::FirstEntry` | `cmd-up` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) |
| `git_panel::FocusChanges` | `tab` (`CommitEditor > Editor`)<br>`shift-tab` (`CommitEditor > Editor`)<br>`alt-up` (`CommitEditor > Editor`) |
| `git_panel::FocusEditor` | `alt-down` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br>`tab` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br>`shift-tab` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) |
| `git_panel::LastEntry` | `cmd-down` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) |
| `git_panel::NextEntry` | `down` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) |
| `git_panel::PreviousEntry` | `up` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) |
| `git_panel::ToggleFocus` | `ctrl-shift-g` (`Workspace`) |
| `git_picker::ActivateBranchesTab` | `cmd-1` (`GitPicker`) |
| `git_picker::ActivateStashTab` | `cmd-2` (`GitPicker`) |
| `stash_picker::DropStashItem` | `ctrl-shift-backspace` (`StashList \|\| (StashList > Picker > Editor)`) |
| `stash_picker::ShowStashItem` | `ctrl-shift-v` (`StashList \|\| (StashList > Picker > Editor)`) |
| `worktree_picker::DeleteWorktree` | `cmd-shift-backspace` (`WorktreePicker \|\| (WorktreePicker > Picker > Editor)`) |
| `worktree_picker::ForceDeleteWorktree` | `cmd-alt-shift-backspace` (`WorktreePicker \|\| (WorktreePicker > Picker > Editor)`) |

#### Terminal, Tasks, Debug und REPL

| Zed Action-ID | Gepinnter macOS-Default und Kontext |
|---|---|
| `console::WatchExpression` | `alt-enter` (`DebugConsole > Editor`) |
| `debug_panel::ToggleFocus` | `cmd-shift-d` (`Workspace`) |
| `debugger::Continue` | `f5` (`Workspace && debugger_stopped`) |
| `debugger::NextBreakpointProperty` | `right` (`BreakpointList`) |
| `debugger::Pause` | `f6` (`Workspace && debugger_session`) |
| `debugger::PreviousBreakpointProperty` | `left` (`BreakpointList`) |
| `debugger::Rerun` | `f5` (`Workspace`) |
| `debugger::RerunSession` | `shift-cmd-f5` (`Workspace && debugger_session`) |
| `debugger::Start` | `f4` (`<global>`) |
| `debugger::StepInto` | `f11` (`Workspace && debugger_stopped`)<br>`ctrl-f11` (`Workspace && debugger_stopped`) |
| `debugger::StepOut` | `shift-f11` (`Workspace && debugger_stopped`) |
| `debugger::StepOver` | `f7` (`Workspace && debugger_stopped`)<br>`f10` (`Workspace && debugger_stopped`) |
| `debugger::Stop` | `shift-f5` (`Workspace && debugger_session`) |
| `debugger::ToggleEnableBreakpoint` | `space` (`BreakpointList`) |
| `debugger::ToggleExpandItem` | `shift-alt-escape` (`DebugPanel`) |
| `debugger::ToggleSessionPicker` | `cmd-i` (`DebugPanel`) |
| `debugger::ToggleThreadPicker` | `cmd-t` (`DebugPanel`) |
| `debugger::UnsetBreakpoint` | `backspace` (`BreakpointList`) |
| `dev::EditPredictionContextGoBack` | `alt-left` (`EditPredictionContext > Editor`) |
| `dev::EditPredictionContextGoForward` | `alt-right` (`EditPredictionContext > Editor`) |
| `dev::ResetFrameOverlayStats` | `ctrl-alt-shift-o` (`<global>`) |
| `dev::ToggleFpsOverlay` | `ctrl-alt-shift-p` (`<global>`) |
| `dev::ToggleInspector` | `cmd-alt-i` (`<global>`) |
| `new_process_modal::ActivateAttachTab` | `cmd-3` (`RunModal`) |
| `new_process_modal::ActivateDebugTab` | `cmd-2` (`RunModal`) |
| `new_process_modal::ActivateLaunchTab` | `cmd-4` (`RunModal`) |
| `new_process_modal::ActivateTaskTab` | `cmd-1` (`RunModal`) |
| `repl::Run` | `ctrl-shift-enter` (`Editor && jupyter`) |
| `repl::RunInPlace` | `ctrl-alt-enter` (`Editor && jupyter`) |
| `task::Rerun` | `cmd-alt-r args={"reevaluate_context":false}` (`Workspace && !Terminal`) |
| `task::Spawn` | `cmd-shift-r` (`Workspace && !Terminal`)<br>`ctrl-alt-shift-r args={"reveal_target":"center"}` (`Workspace && !Terminal`) |
| `terminal::Clear` | `cmd-k` (`Terminal`) |
| `terminal::Copy` | `cmd-c` (`Terminal`) |
| `terminal::Paste` | `cmd-v` (`Terminal`) |
| `terminal::PasteText` | `ctrl-cmd-v` (`Terminal`) |
| `terminal::RerunTask` | `cmd-alt-r` (`Terminal`) |
| `terminal::ScrollLineDown` | `shift-down` (`Terminal`) |
| `terminal::ScrollLineUp` | `shift-up` (`Terminal`) |
| `terminal::ScrollPageDown` | `shift-pagedown` (`Terminal`)<br>`cmd-down` (`Terminal`) |
| `terminal::ScrollPageUp` | `shift-pageup` (`Terminal`)<br>`cmd-up` (`Terminal`) |
| `terminal::ScrollToBottom` | `shift-end` (`Terminal`)<br>`cmd-end` (`Terminal`) |
| `terminal::ScrollToTop` | `shift-home` (`Terminal`)<br>`cmd-home` (`Terminal`) |
| `terminal::SendKeystroke` | `cmd-backspace args="ctrl-u"` (`Terminal`)<br>`cmd-delete args="ctrl-k"` (`Terminal`)<br>`cmd-right args="ctrl-e"` (`Terminal`)<br>`cmd-left args="ctrl-a"` (`Terminal`)<br>`up args="up"` (`Terminal`)<br>`pageup args="pageup"` (`Terminal`)<br>`down args="down"` (`Terminal`)<br>`pagedown args="pagedown"` (`Terminal`)<br>`escape args="escape"` (`Terminal`)<br>`enter args="enter"` (`Terminal`)<br>`ctrl-c args="ctrl-c"` (`Terminal`)<br>`ctrl-r args="ctrl-r"` (`Terminal`)<br>`ctrl-backspace args="ctrl-w"` (`Terminal`) |
| `terminal::SendText` | `alt-delete args="\u001bd"` (`Terminal`)<br>`alt-left args="\u001bb"` (`Terminal`)<br>`alt-right args="\u001bf"` (`Terminal`)<br>`alt-b args="\u001bb"` (`Terminal`)<br>`alt-f args="\u001bf"` (`Terminal`)<br>`ctrl-delete args="\u001b[3;5~"` (`Terminal`) |
| `terminal::ShowCharacterPalette` | `ctrl-cmd-space` (`Terminal`) |
| `terminal::ToggleViMode` | `ctrl-shift-space` (`Terminal`) |
| `terminal_panel::Toggle` | `ctrl-`` (`Workspace`) |
| `variable_list::AddWatch` | `alt-enter` (`VariableList`) |
| `variable_list::CollapseSelectedEntry` | `left` (`VariableList`) |
| `variable_list::CopyVariableName` | `cmd-alt-c` (`VariableList`) |
| `variable_list::CopyVariableValue` | `cmd-c` (`VariableList`) |
| `variable_list::EditVariable` | `enter` (`VariableList`) |
| `variable_list::ExpandSelectedEntry` | `right` (`VariableList`) |
| `variable_list::RemoveWatch` | `delete` (`VariableList`)<br>`backspace` (`VariableList`) |

#### AI und Zusammenarbeit

| Zed Action-ID | Gepinnter macOS-Default und Kontext |
|---|---|
| `agent::AddSelectionToThread` | `cmd->` (`Editor && mode == full`)<br>`cmd->` (`AcpThread`)<br>`cmd->` (`Terminal`) |
| `agent::AllowAlways` | `cmd-alt-y` (`AcpThread`) |
| `agent::AllowOnce` | `cmd-y` (`AcpThread`) |
| `agent::ArchiveSelectedThread` | `backspace` (`ThreadsArchiveView`)<br>`shift-backspace` (`ThreadsSidebar`) |
| `agent::Chat` | `enter` (`AcpThread > Editor && !use_modifier_to_send`)<br>`cmd-enter` (`AcpThread > Editor && use_modifier_to_send`) |
| `agent::ChatWithFollow` | `cmd-enter` (`AcpThread > Editor`) |
| `agent::ClearMessageQueue` | `cmd-alt-backspace` (`AcpThread > Editor`) |
| `agent::CycleFavoriteModels` | `alt-tab` (`AcpThread`)<br>`alt-tab` (`AcpThread > Editor`)<br>`alt-tab` (`InlineAssistant > Editor`) |
| `agent::CycleModeSelector` | `shift-tab` (`AcpThread`)<br>`shift-tab` (`AcpThread > Editor`) |
| `agent::CycleNextInlineAssist` | `ctrl-]` (`InlineAssistant > Editor`) |
| `agent::CyclePreviousInlineAssist` | `ctrl-[` (`InlineAssistant > Editor`) |
| `agent::CycleThinkingEffort` | `ctrl-'` (`AcpThread > Editor`) |
| `agent::DismissThreadSearch` | `escape` (`AcpThreadSearchBar`) |
| `agent::EditFirstQueuedMessage` | `cmd-ctrl-e` (`AcpThread > Editor`) |
| `agent::ExpandMessageEditor` | `shift-alt-escape` (`AcpThread`) |
| `agent::Keep` | `cmd-y` (`AgentDiff`)<br>`cmd-alt-y` (`AgentDiff`)<br>`cmd-y` (`Editor && editor_agent_diff`)<br>`cmd-alt-y` (`Editor && editor_agent_diff`) |
| `agent::KeepAll` | `shift-alt-y` (`AgentDiff`)<br>`shift-alt-y` (`Editor && editor_agent_diff`)<br>`shift-alt-y` (`AcpThread > Editor`) |
| `agent::ManageProfiles` | `cmd-alt-p` (`AcpThread`) |
| `agent::ManageSkills` | `cmd-alt-l` (`AcpThread`) |
| `agent::NewThread` | `cmd-n` (`AgentPanel`)<br>`cmd-n` (`AcpThread`)<br>`cmd-n` (`AgentPanel > Terminal`) |
| `agent::OpenAddContextMenu` | `ctrl-;` (`AcpThread > Editor`) |
| `agent::OpenAgentDiff` | `shift-ctrl-r` (`Editor && editor_agent_diff`)<br>`shift-ctrl-r` (`AcpThread > Editor`) |
| `agent::OpenPermissionDropdown` | `cmd-alt-a` (`AcpThread`) |
| `agent::OpenSettings` | `cmd-alt-c` (`AgentPanel`) |
| `agent::PasteRaw` | `cmd-shift-v` (`AcpThread > Editor`) |
| `agent::Reject` | `cmd-alt-z` (`AgentDiff`)<br>`cmd-alt-z` (`Editor && editor_agent_diff`) |
| `agent::RejectAll` | `shift-alt-z` (`AgentDiff`)<br>`shift-alt-z` (`Editor && editor_agent_diff`)<br>`shift-alt-z` (`AcpThread > Editor`) |
| `agent::RejectOnce` | `cmd-alt-z` (`AcpThread`) |
| `agent::RemoveFirstQueuedMessage` | `cmd-shift-backspace` (`AcpThread > Editor`) |
| `agent::RemoveSelectedThread` | `shift-backspace` (`ThreadHistory > Editor`)<br>`cmd-shift-backspace` (`ThreadsSidebar`) |
| `agent::RenameSelectedThread` | `shift-r` (`ThreadsSidebar && not_searching`) |
| `agent::ScrollOutputLineDown` | `down` (`AcpThread`)<br>`ctrl-alt-down` (`AcpThread`)<br>`ctrl-alt-down` (`AcpThread > Editor`) |
| `agent::ScrollOutputLineUp` | `up` (`AcpThread`)<br>`ctrl-alt-up` (`AcpThread`)<br>`ctrl-alt-up` (`AcpThread > Editor`) |
| `agent::ScrollOutputPageDown` | `pagedown` (`AcpThread`)<br>`ctrl-pagedown` (`AcpThread`)<br>`ctrl-pagedown` (`AcpThread > Editor`)<br>`pagedown` (`AcpThread > Editor && end_of_input`)<br>`ctrl-pagedown` (`AcpThread > Editor && end_of_input`) |
| `agent::ScrollOutputPageUp` | `pageup` (`AcpThread`)<br>`ctrl-pageup` (`AcpThread`)<br>`ctrl-pageup` (`AcpThread > Editor`)<br>`pageup` (`AcpThread > Editor && start_of_input`)<br>`ctrl-pageup` (`AcpThread > Editor && start_of_input`) |
| `agent::ScrollOutputToBottom` | `end` (`AcpThread`)<br>`ctrl-end` (`AcpThread`)<br>`ctrl-end` (`AcpThread > Editor`)<br>`ctrl-end` (`AcpThread > Editor && end_of_input`) |
| `agent::ScrollOutputToNextMessage` | `shift-pagedown` (`AcpThread`)<br>`ctrl-alt-pagedown` (`AcpThread`)<br>`ctrl-alt-pagedown` (`AcpThread > Editor`) |
| `agent::ScrollOutputToPreviousMessage` | `shift-pageup` (`AcpThread`)<br>`ctrl-alt-pageup` (`AcpThread`)<br>`ctrl-alt-pageup` (`AcpThread > Editor`) |
| `agent::ScrollOutputToTop` | `home` (`AcpThread`)<br>`ctrl-home` (`AcpThread`)<br>`ctrl-home` (`AcpThread > Editor`)<br>`ctrl-home` (`AcpThread > Editor && start_of_input`) |
| `agent::SelectNextThreadMatch` | `cmd-g` (`AcpThread`)<br>`enter` (`AcpThreadSearchBar`) |
| `agent::SelectPreviousThreadMatch` | `cmd-shift-g` (`AcpThread`)<br>`shift-enter` (`AcpThreadSearchBar`)<br>`shift-enter` (`AcpThreadSearchBar > Editor`) |
| `agent::SendImmediately` | `cmd-shift-enter` (`AcpThread > Editor`) |
| `agent::SendNextQueuedMessage` | `cmd-shift-alt-enter` (`AcpThread > Editor`) |
| `agent::ToggleFastMode` | `cmd-alt-.` (`AcpThread > Editor`) |
| `agent::ToggleFocus` | `cmd-?` (`Workspace`) |
| `agent::ToggleModelSelector` | `cmd-alt-/` (`AcpThread`)<br>`cmd-alt-/` (`InlineAssistant > Editor`) |
| `agent::ToggleNewThreadMenu` | `cmd-alt-shift-n` (`AgentPanel`) |
| `agent::ToggleOptionsMenu` | `cmd-alt-m` (`AgentPanel`) |
| `agent::ToggleProfileSelector` | `cmd-i` (`AcpThread`)<br>`cmd-i` (`AcpThread > Editor`) |
| `agent::ToggleSearch` | `cmd-f` (`AcpThread`)<br>`cmd-f` (`AcpThread > Editor`)<br>`cmd-f` (`AgentPanel > Terminal`) |
| `agent::ToggleSteerFirstQueuedMessage` | `cmd-ctrl-s` (`AcpThread > Editor`) |
| `agent::ToggleThinkingEffortMenu` | `cmd-alt-'` (`AcpThread > Editor`) |
| `agent::ToggleThinkingMode` | `cmd-alt-k` (`AcpThread > Editor`) |
| `agent::UndoLastReject` | `shift-alt-u` (`AcpThread > Editor`) |
| `agents_sidebar::FocusSidebarFilter` | `cmd-f` (`ThreadsSidebar`) |
| `agents_sidebar::NewThreadInGroup` | `cmd-n` (`ThreadsSidebar`) |
| `agents_sidebar::ToggleThreadHistory` | `cmd-g` (`ThreadsSidebar`) |
| `agents_sidebar::ToggleThreadSwitcher` | `ctrl-tab` (`AgentPanel`)<br>`ctrl-shift-tab args={"select_last":true}` (`AgentPanel`)<br>`ctrl-tab` (`ThreadsSidebar`)<br>`ctrl-shift-tab args={"select_last":true}` (`ThreadsSidebar`)<br>`ctrl-tab` (`ThreadSwitcher`)<br>`ctrl-shift-tab args={"select_last":true}` (`ThreadSwitcher`) |
| `assistant::InlineAssist` | `ctrl-enter` (`!AcpThread > Editor && mode == full`)<br>`ctrl-enter` (`Terminal`) |
| `channel_modal::ToggleMode` | `tab` (`ChannelModal`)<br>`tab` (`ChannelModal > Picker > Editor`) |
| `collab_panel::InsertSpace` | `space` (`(CollabPanel && editing) > Editor`) |
| `collab_panel::MoveChannelDown` | `alt-down` (`CollabPanel`) |
| `collab_panel::MoveChannelUp` | `alt-up` (`CollabPanel`) |
| `collab_panel::OpenSelectedChannelNotes` | `alt-enter` (`CollabPanel`) |
| `collab_panel::Remove` | `ctrl-backspace` (`CollabPanel && not_editing`) |
| `collab_panel::ToggleFocus` | `cmd-shift-c` (`<global>`) |
| `collab_panel::ToggleSelectedChannelFavorite` | `shift-enter` (`CollabPanel`) |
| `inline_assistant::ThumbsDownResult` | `cmd-shift-backspace` (`InlineAssistant > Editor`) |
| `inline_assistant::ThumbsUpResult` | `cmd-shift-enter` (`InlineAssistant > Editor`) |
| `skill_creator::FocusNextField` | `tab` (`SkillCreator`)<br>`tab` (`SkillCreator > Editor`) |
| `skill_creator::FocusPreviousField` | `shift-tab` (`SkillCreator`)<br>`shift-tab` (`SkillCreator > Editor`) |
| `skill_creator::SaveSkill` | `cmd-enter` (`SkillCreator`)<br>`cmd-enter` (`SkillCreator > Editor`) |
| `zeta::FocusPredictions` | `escape` (`RatePredictionsModal > Editor`) |
| `zeta::NextEdit` | `shift-down` (`RatePredictionsModal`) |
| `zeta::PreviewPrediction` | `right` (`RatePredictionsModal`) |
| `zeta::PreviousEdit` | `shift-up` (`RatePredictionsModal`) |
| `zeta::ThumbsDownActivePrediction` | `cmd-shift-backspace` (`RatePredictionsModal`)<br>`cmd-shift-backspace` (`RatePredictionsModal > Editor`) |
| `zeta::ThumbsUpActivePrediction` | `cmd-shift-enter` (`RatePredictionsModal`)<br>`cmd-shift-enter` (`RatePredictionsModal > Editor`) |

#### Dokumentansichten, Editor-Konfiguration und Keymap

| Zed Action-ID | Gepinnter macOS-Default und Kontext |
|---|---|
| `edit_prediction::RatePredictions` | `ctrl-cmd-z` (`<global>`) |
| `edit_prediction::ToggleMenu` | `ctrl-cmd-i` (`<global>`) |
| `image_viewer::FitToView` | `cmd-shift-0` (`ImageViewer`) |
| `image_viewer::ResetZoom` | `cmd-0` (`ImageViewer`) |
| `image_viewer::ZoomIn` | `cmd-=` (`ImageViewer`)<br>`cmd-+` (`ImageViewer`) |
| `image_viewer::ZoomOut` | `cmd--` (`ImageViewer`) |
| `image_viewer::ZoomToActualSize` | `cmd-1` (`ImageViewer`) |
| `keymap_editor::CopyAction` | `cmd-c` (`KeymapEditor`) |
| `keymap_editor::CopyContext` | `cmd-shift-c` (`KeymapEditor`) |
| `keymap_editor::CreateBinding` | `alt-enter` (`KeymapEditor`) |
| `keymap_editor::EditBinding` | `enter` (`KeymapEditor`) |
| `keymap_editor::OpenCreateKeybindingModal` | `cmd-k` (`KeymapEditor`) |
| `keymap_editor::ShowMatchingKeybinds` | `cmd-t` (`KeymapEditor`) |
| `keymap_editor::ToggleConflictFilter` | `cmd-alt-c` (`KeymapEditor`) |
| `keymap_editor::ToggleKeystrokeSearch` | `cmd-alt-f` (`KeymapEditor`)<br>`cmd-alt-f` (`KeymapEditor > BufferSearchBar`) |
| `keystroke_input::ClearKeystrokes` | `delete` (`KeystrokeInput`) |
| `keystroke_input::StartRecording` | `enter` (`KeystrokeInput`) |
| `keystroke_input::StopRecording` | `escape escape escape` (`KeystrokeInput`) |
| `notebook::AddCodeBlock` | `cmd-m` (`NotebookEditor`)<br>`cmd-m` (`NotebookEditor > Editor`) |
| `notebook::AddMarkdownBlock` | `cmd-shift-m` (`NotebookEditor`)<br>`cmd-shift-m` (`NotebookEditor > Editor`) |
| `notebook::DeleteCell` | `d d` (`NotebookEditor && notebook_mode == command`)<br>`backspace` (`NotebookEditor && notebook_mode == command`) |
| `notebook::EnterCommandMode` | `escape` (`NotebookEditor > Editor`) |
| `notebook::EnterEditMode` | `enter` (`NotebookEditor && notebook_mode == command`) |
| `notebook::InterruptKernel` | `cmd-c` (`NotebookEditor`) |
| `notebook::MoveCellDown` | `alt-down` (`NotebookEditor`)<br>`alt-down` (`NotebookEditor > Editor`) |
| `notebook::MoveCellUp` | `alt-up` (`NotebookEditor`)<br>`alt-up` (`NotebookEditor > Editor`) |
| `notebook::RestartKernel` | `cmd-shift-r` (`NotebookEditor`)<br>`cmd-shift-r` (`NotebookEditor > Editor`) |
| `notebook::Run` | `cmd-enter` (`NotebookEditor`)<br>`cmd-enter` (`NotebookEditor > Editor`) |
| `notebook::RunAll` | `cmd-shift-enter` (`NotebookEditor`)<br>`cmd-shift-enter` (`NotebookEditor > Editor`) |
| `notebook::RunAndAdvance` | `shift-enter` (`NotebookEditor`)<br>`shift-enter` (`NotebookEditor > Editor`) |
| `picker::ConfirmCompletion` | `tab` (`Picker > Editor`) |
| `picker::ConfirmInput` | `alt-enter args={"secondary":false}` (`Picker > Editor`)<br>`cmd-alt-enter args={"secondary":true}` (`Picker > Editor`) |
| `settings_editor::CollapseNavEntry` | `left` (`SettingsWindow > NavigationMenu`) |
| `settings_editor::ExpandNavEntry` | `right` (`SettingsWindow > NavigationMenu`) |
| `settings_editor::FocusFile` | `ctrl-1 args=0` (`SettingsWindow`)<br>`ctrl-2 args=1` (`SettingsWindow`)<br>`ctrl-3 args=2` (`SettingsWindow`)<br>`ctrl-4 args=3` (`SettingsWindow`)<br>`ctrl-5 args=4` (`SettingsWindow`)<br>`ctrl-6 args=5` (`SettingsWindow`)<br>`ctrl-7 args=6` (`SettingsWindow`)<br>`ctrl-8 args=7` (`SettingsWindow`)<br>`ctrl-9 args=8` (`SettingsWindow`)<br>`ctrl-0 args=9` (`SettingsWindow`) |
| `settings_editor::FocusFirstNavEntry` | `home` (`SettingsWindow > NavigationMenu`) |
| `settings_editor::FocusLastNavEntry` | `end` (`SettingsWindow > NavigationMenu`) |
| `settings_editor::FocusNextFile` | `cmd-}` (`SettingsWindow`) |
| `settings_editor::FocusNextNavEntry` | `down` (`SettingsWindow > NavigationMenu`)<br>`tab` (`SettingsWindow > NavigationMenu`) |
| `settings_editor::FocusNextRootNavEntry` | `pagedown` (`SettingsWindow > NavigationMenu`) |
| `settings_editor::FocusPreviousFile` | `cmd-{` (`SettingsWindow`) |
| `settings_editor::FocusPreviousNavEntry` | `up` (`SettingsWindow > NavigationMenu`)<br>`shift-tab` (`SettingsWindow > NavigationMenu`) |
| `settings_editor::FocusPreviousRootNavEntry` | `pageup` (`SettingsWindow > NavigationMenu`) |
| `settings_editor::Minimize` | `cmd-m` (`SettingsWindow`) |
| `settings_editor::OpenCurrentFile` | `cmd-,` (`SettingsWindow`) |
| `settings_editor::ToggleFocusNav` | `left` (`SettingsWindow`)<br>`cmd-shift-e` (`SettingsWindow`) |
| `settings_profile_selector::Toggle` | `ctrl-alt-cmd-p` (`Workspace`) |
| `tab_switcher::CloseSelectedItem` | `ctrl-backspace` (`TabSwitcher`) |
| `tab_switcher::Toggle` | `ctrl-shift-tab args={"select_last":true}` (`Workspace`)<br>`ctrl-tab` (`Workspace`) |
| `tabular_data::OpenPreview` | `cmd-shift-v` (`Editor && (extension == csv \|\| extension == tsv \|\| extension == ssv \|\| extension == psv)`) |
| `tabular_data::OpenPreviewToTheSide` | `cmd-k v` (`Editor && (extension == csv \|\| extension == tsv \|\| extension == ssv \|\| extension == psv)`) |
| `theme::ToggleMode` | `cmd-k cmd-shift-t` (`Workspace`) |
| `theme_selector::Toggle` | `cmd-k cmd-t` (`Workspace`) |
| `toolchain::AddToolchain` | `cmd-k cmd-m` (`Workspace`)<br>`cmd-shift-a` (`ToolchainSelector`) |
| `zed::DecreaseBufferFontSize` | `cmd-- args={"persist":false}` (`<global>`) |
| `zed::DecreaseUiFontSize` | `cmd-- args={"persist":false}` (`Onboarding`)<br>`cmd-- args={"persist":false}` (`Welcome`) |
| `zed::Extensions` | `cmd-shift-x` (`Workspace`) |
| `zed::Hide` | `cmd-h` (`<global>`) |
| `zed::HideOthers` | `alt-cmd-h` (`<global>`) |
| `zed::IncreaseBufferFontSize` | `cmd-= args={"persist":false}` (`<global>`)<br>`cmd-+ args={"persist":false}` (`<global>`) |
| `zed::IncreaseUiFontSize` | `cmd-= args={"persist":false}` (`Onboarding`)<br>`cmd-+ args={"persist":false}` (`Onboarding`)<br>`cmd-= args={"persist":false}` (`Welcome`)<br>`cmd-+ args={"persist":false}` (`Welcome`) |
| `zed::Minimize` | `cmd-m` (`<global>`) |
| `zed::OpenKeymap` | `cmd-k cmd-s` (`Workspace`) |
| `zed::OpenKeymapFile` | `cmd-e` (`KeymapEditor`) |
| `zed::OpenSettings` | `cmd-,` (`!SettingsWindow`) |
| `zed::OpenSettingsFile` | `cmd-alt-,` (`<global>`) |
| `zed::OpenWorktreeSetupTasks` | `cmd-shift-c` (`WorktreePicker \|\| (WorktreePicker > Picker > Editor)`) |
| `zed::Quit` | `cmd-q` (`<global>`) |
| `zed::ResetBufferFontSize` | `cmd-0 args={"persist":false}` (`<global>`) |
| `zed::ResetUiFontSize` | `cmd-0 args={"persist":false}` (`Onboarding`)<br>`cmd-0 args={"persist":false}` (`Welcome`) |
| `zed::ToggleFullScreen` | `fn-f` (`Workspace`)<br>`ctrl-cmd-f` (`Workspace`) |

#### App-Menü, Start und transiente Oberflächen

| Zed Action-ID | Gepinnter macOS-Default und Kontext |
|---|---|
| `command_palette::Toggle` | `cmd-shift-p` (`Workspace`) |
| `menu::Cancel` | `cmd-escape` (`<global>`)<br>`ctrl-escape` (`<global>`)<br>`ctrl-c` (`<global>`)<br>`escape` (`<global>`)<br>`escape` (`AgentFeedbackMessageEditor > Editor`)<br>`escape` (`OutlinePanel && not_editing`)<br>`escape` (`ProjectPanel`)<br>`escape` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br>`escape` (`GitCommit > Editor && mode == auto_height`)<br>`escape` (`Picker > Editor`)<br>`escape` (`ZedPredictModal`)<br>`escape` (`ConfigureContextServerModal > Editor`)<br>`escape` (`OnboardingAiConfigurationModal`)<br>`escape` (`KeybindEditorModal`) |
| `menu::Confirm` | `enter` (`<global>`)<br>`enter` (`AgentFeedbackMessageEditor > Editor`)<br>`cmd-enter` (`AcpThread > ModeSelector`)<br>`enter` (`ThreadsSidebar`)<br>`space` (`ThreadsSidebar && not_searching`)<br>`enter` (`Editor && inline_input`)<br>`enter` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br>`space` (`CollabPanel && not_editing`)<br>`cmd-enter` (`ConfigureContextServerModal > Editor`)<br>`enter` (`DebugConsole > Editor`)<br>`cmd-enter` (`KeybindEditorModal`) |
| `menu::Restart` | `alt-shift-enter` (`<global>`) |
| `menu::SecondaryConfirm` | `ctrl-enter` (`<global>`)<br>`cmd-enter` (`<global>`) |
| `menu::SelectChild` | `right` (`menu`)<br>`right` (`ThreadsSidebar`)<br>`cmd-k right` (`CallHierarchyPicker > Picker > Editor`) |
| `menu::SelectFirst` | `home` (`<global>`)<br>`shift-pageup` (`<global>`)<br>`pageup` (`<global>`)<br>`cmd-up` (`<global>`) |
| `menu::SelectLast` | `end` (`<global>`)<br>`shift-pagedown` (`<global>`)<br>`pagedown` (`<global>`)<br>`cmd-down` (`<global>`) |
| `menu::SelectNext` | `tab` (`<global>`)<br>`ctrl-n` (`<global>`)<br>`down` (`<global>`)<br>`right` (`Prompt`)<br>`l` (`Prompt`)<br>`shift-down` (`OutlinePanel && not_editing`)<br>`shift-down` (`ProjectPanel`)<br>`down` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br>`shift-down` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br>`down` (`Picker > Editor`)<br>`ctrl-down` (`TabSwitcher`)<br>`down` (`KeybindEditorModal > Editor`)<br>`down` (`NotebookEditor && notebook_mode == command`) |
| `menu::SelectParent` | `left` (`menu`)<br>`left` (`ThreadsSidebar`)<br>`cmd-k left` (`CallHierarchyPicker > Picker > Editor`) |
| `menu::SelectPrevious` | `shift-tab` (`<global>`)<br>`ctrl-p` (`<global>`)<br>`up` (`<global>`)<br>`left` (`Prompt`)<br>`h` (`Prompt`)<br>`shift-up` (`OutlinePanel && not_editing`)<br>`shift-up` (`ProjectPanel`)<br>`up` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br>`shift-up` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br>`up` (`Picker > Editor`)<br>`ctrl-shift-tab` (`TabSwitcher`)<br>`ctrl-up` (`TabSwitcher`)<br>`up` (`KeybindEditorModal > Editor`)<br>`up` (`NotebookEditor && notebook_mode == command`) |
| `onboarding::Finish` | `cmd-enter` (`Onboarding`) |
| `onboarding::OpenAccount` | `alt-shift-a` (`Onboarding`) |
| `onboarding::SignIn` | `alt-tab` (`Onboarding`) |
| `toast::RunAction` | `alt-shift-enter` (`Workspace`) |
| `welcome::OpenRecentProject` | `cmd-1 args=0` (`Welcome`)<br>`cmd-2 args=1` (`Welcome`)<br>`cmd-3 args=2` (`Welcome`)<br>`cmd-4 args=3` (`Welcome`)<br>`cmd-5 args=4` (`Welcome`) |

### Labonair Command-Palette-Beschreibungen aus dem Editor-Owner

Der folgende Bestand ist exakt der 39 Einträge umfassende Rückgabewert von `EditorCommandProvider::commands()` im aktuellen Quellstand: 3 allgemeine Editor-Einstiege und 36 ausführbare Editor-Commands. Die Shortcuts sind die dort deklarierten macOS-Bindings. `—` bedeutet **kein Default-Binding in dieser Provider-Metadatei**; es schließt eine effektive Bindung aus einer anderen Keymap-Schicht nicht aus. Der Bestand umfasst nicht jede native Cursor-/Textbewegung des Editor-Widgets.

| Command-ID | Angezeigter Befehl | Provider-Default (macOS) | Route / Wirkung |
|---|---|---|---|
| `OpenFile` | Open File… | `⌘⇧O` | Datei öffnen |
| `Find` | Find in Current Pane | `⌘F` | Such-Overlay im aktiven Pane |
| `GoToSymbol` | Go to Symbol… | — | Outline-/Symbol-Untermenü |
| `EditorAddCursor` | Add Cursor | `⌘⌥↓` | Bearbeitung: Cursor hinzufügen |
| `EditorSelectNextOccurrence` | Select Next Occurrence | `⌘D` | Bearbeitung: nächste Fundstelle auswählen |
| `EditorSelectAllOccurrences` | Select All Occurrences | — | Bearbeitung: alle Fundstellen auswählen |
| `EditorExpandSelection` | Expand Selection | `⌃⇧→` | Bearbeitung: Auswahl erweitern |
| `EditorShrinkSelection` | Shrink Selection | `⌃⇧←` | Bearbeitung: Auswahl verkleinern |
| `EditorDuplicateLines` | Duplicate Lines | `⌘⇧D` | Bearbeitung: Zeilen duplizieren |
| `EditorMoveLines` | Move Lines | — | Bearbeitung: Zeilen nach unten bewegen |
| `EditorIndent` | Indent | `⌘]` | Bearbeitung: zwei Leerzeichen einrücken |
| `EditorOutdent` | Outdent | `⌘[` | Bearbeitung: zwei Leerzeichen ausrücken |
| `EditorToggleComment` | Toggle Comment | `⌘/` | Bearbeitung: Kommentar umschalten |
| `EditorTranspose` | Transpose Characters | — | Bearbeitung: Zeichen vertauschen |
| `EditorSplitRight` | Split Editor Right | `⌘⌥]` | Editorgruppe rechts teilen |
| `EditorSplitDown` | Split Editor Down | `⌘⌥⇧D` | Editorgruppe unten teilen |
| `EditorFocusNextGroup` | Focus Next Editor Group | `⌘⌥→` | Nächste Editorgruppe fokussieren |
| `EditorFocusPreviousGroup` | Focus Previous Editor Group | `⌘⌥←` | Vorige Editorgruppe fokussieren |
| `EditorCloseGroup` | Close Editor Group | — | Editorgruppe schließen |
| `EditorCloseOtherGroups` | Close Other Editor Groups | — | Andere Editorgruppen schließen |
| `EditorTriggerCompletion` | Trigger Completion | `⌃Space` | Language Service: Completion anfragen |
| `EditorGoToDefinition` | Go to Definition | `F12` | Language Service: Definition öffnen |
| `EditorGoToDeclaration` | Go to Declaration | — | Language Service: Deklaration öffnen |
| `EditorGoToReferences` | Go to References | — | Language Service: Referenzen anfragen |
| `EditorGoToImplementation` | Go to Implementation | — | Language Service: Implementierung öffnen |
| `EditorPeekDefinition` | Peek Definition | — | Language Service: Definition in Peek-UI |
| `EditorRenameSymbol` | Rename Symbol | `F2` | Language Service: Rename anfragen/anwenden |
| `EditorCodeAction` | Code Action | `⌘.` | Language Service: Code Actions anzeigen |
| `EditorFormatDocument` | Format Document | `⌥⇧F` | Language Service: Dokument formatieren |
| `EditorFormatSelection` | Format Selection | — | Language Service: Auswahl formatieren |
| `EditorOrganizeImports` | Organize Imports | — | Language Service: Imports ordnen |
| `EditorRestartLanguageServer` | Restart Language Server | — | Language Service: Prozess neustarten |
| `EditorStopLanguageServer` | Stop Language Server | — | Language Service: Prozess stoppen |
| `EditorShowDiagnostics` | Show Diagnostics | — | Language Service: Diagnosen anzeigen |
| `EditorNextDiagnostic` | Next Diagnostic | — | Language Service: nächste Diagnose |
| `EditorPreviousDiagnostic` | Previous Diagnostic | — | Language Service: vorige Diagnose |
| `EditorNextGitChange` | Next Git Change | — | Git: nächste Editoränderung |
| `EditorPreviousGitChange` | Previous Git Change | — | Git: vorige Editoränderung |
| `EditorOpenProjectDiff` | Open Project Diff | — | Git/Workspace: Project Diff öffnen |

### Command-Route-Crosswalk für die 39 Editor-Provider-Einträge

Diese Zuordnung nimmt pro Labonair-Provider-Eintrag die nächstliegende gepinnte Zed-Action bzw. den Workflow. **Nächste Action** bedeutet Namens-/Absichtsnähe, keine bestätigte Verhaltensparität. Alle 585 gebundenen Actions sind in den vorangehenden Tabellen inventarisiert; lokale Laufzeit-Routen sind für die übrigen Namespaces noch nicht einzeln crosswalked. Ungebundene Registry-Actions, Menu-Commands ohne Keymap-Bindung und andere Plattformprofile bleiben ebenfalls offen.

| Labonair-Command | Nächste gepinnte Zed-Action / Workflow | Crosswalk-Befund |
|---|---|---|
| OpenFile | workspace::Open / File Finder | workspace::Open öffnet eine Datei oder ein Verzeichnis (⌘O); Labonair OpenFile öffnet eine Datei (⌘⇧O). Zeds ⌘P File Finder ist ein separater Picker-Einstieg. |
| Find | search::FocusSearch | Buffer-Suche ist ein gemeinsamer Einstieg; Zeds Action unterstützt auch Projekt-/andere Suchkontexte. |
| GoToSymbol | Outline-/Symbol-Navigation | Zed bindet outline::Toggle im Editor (⌘⇧O); Labonair öffnet ein Outline-Untermenü aus der Palette. Der Zed-Outline-Panel-Workflow ist kein direkt gleiches Command. |
| EditorAddCursor | editor::AddSelectionBelow / editor::AddSelectionAbove | Zed trennt oberhalb und unterhalb. Labonairs Workspace-Handler übergibt die aktuelle Cursorposition an Document::add_cursor; bei einem üblichen kollabierten Primärcaret erkennt die Methode die schon vorhandene Auswahl und gibt unchanged zurück. Der Provider-Command ist in diesem Zustand daher ein statischer No-Op; auch die Zed-Platzierung oberhalb fehlt. |
| EditorSelectNextOccurrence | editor::SelectNext | Gleiche Grundabsicht und ⌘D-Binding; Semantik der Startauswahl und Wiederholungsgrenzen noch offen. |
| EditorSelectAllOccurrences | editor::SelectAllMatches | Gleiche Grundabsicht; Zed hat im Editor-Kontext ⌘⇧L, für Labonair ist im Provider kein Default-Binding deklariert. |
| EditorExpandSelection | editor::SelectLargerSyntaxNode | Name und Shortcuts liegen nahe; Labonairs Auswahl ist nicht als gleichwertige Tree-sitter-Knotenauswahl belegt. |
| EditorShrinkSelection | editor::SelectSmallerSyntaxNode | Gleiches Gegenstück und derselbe Vorbehalt zur AST-/Syntax-Node-Semantik. |
| EditorDuplicateLines | editor::DuplicateLineDown / editor::DuplicateLineUp | Gleiche Zeilenaktion; Zed trennt Richtung und bindet ⌥⇧↓/↑, Labonair dupliziert über einen Command mit ⌘⇧D. |
| EditorMoveLines | editor::MoveLineDown / editor::MoveLineUp | Labonair-Route ist fest auf down: true; eine Aufwärtsroute fehlt in diesem Provider. |
| EditorIndent | editor::Indent | Gleiche Aktion und ⌘]; Labonair übergibt fest zwei Leerzeichen statt die Settings-/Sprach-Einrückung zu verwenden. |
| EditorOutdent | editor::Outdent | Gleiche Aktion und ⌘[; auch hier ist der lokale Indent-Schritt fest auf zwei Leerzeichen gesetzt. |
| EditorToggleComment | editor::ToggleComments | Gleiche Action-Absicht und ⌘/; sprachspezifische Kommentarpräfixe sind nicht gleichwertig belegt. |
| EditorTranspose | editor::Transpose | Gleiche Aktion; Labonair-Provider deklariert kein Default-Binding, Zed bindet ⌃T. |
| EditorSplitRight | pane::SplitRight | Lokale Route ist EditorCommandRoute::Split(Right) und editorgruppen-intern. Zed pane::SplitRight teilt einen Workspace-Pane; lokal gibt es zusätzlich pane::SplitRight im Terminal-Kontext. |
| EditorSplitDown | pane::SplitDown | Lokale Route ist EditorCommandRoute::Split(Down) und editorgruppen-intern. Zed pane::SplitDown teilt einen Workspace-Pane; lokal gibt es zusätzlich pane::SplitDown im Terminal-Kontext. |
| EditorFocusNextGroup | Workspace-Pane-/Editorgruppen-Fokus vorwärts | Lokale Editorgruppe; Zeds workspace::ActivatePane* navigiert räumlich durch Workspace-Panes, während Labonair pane::FocusNext zyklisch zwischen Panes wechselt. |
| EditorFocusPreviousGroup | Workspace-Pane-/Editorgruppen-Fokus rückwärts | Lokale Editorgruppe; Zed bietet zusätzlich richtungsbezogene workspace::ActivatePane*-Actions, lokal ist nur zyklisches pane::FocusNext gebunden. |
| EditorCloseGroup | Workspace-Pane schließen | Zed pane::CloseActiveItem schließt ein Pane-Item; es entspricht nicht direkt dem lokalen Editorgruppen-Close. |
| EditorCloseOtherGroups | Andere Workspace-Panes schließen | Zed workspace::CloseInactiveTabsAndPanes schließt inaktive Tabs und Panes, nicht exakt andere Editorgruppen; Dirty-Tab-Verhalten bleibt zu vergleichen. |
| EditorTriggerCompletion | editor::ShowCompletions | Gleiche Grundaktion und ⌃Space; Completion-Popup und LSP-/Snippetumfang bleiben separat zu prüfen. |
| EditorGoToDefinition | editor::GoToDefinition | Gleiche Action und F12; Zielauswahl, History und Multi-Buffer-Verhalten nicht gleichgesetzt. |
| EditorGoToDeclaration | editor::GoToDeclaration | Gleiche Action; Zed bindet ⌃F12, Labonair hat im Provider kein Default-Binding. |
| EditorGoToReferences | editor::FindAllReferences | Gleiche Request-Familie; Zeds ⌥⇧F12 und Multi-Buffer-Ergebnisworkflow fehlen als bestätigte Parität. |
| EditorGoToImplementation | editor::GoToImplementation | Gleiche Action und Zed-Binding ⇧F12; lokale Ergebnisansicht bleibt eingeschränkt. |
| EditorPeekDefinition | Definition-Preview / editor::GoToDefinition-nahe Navigation | Labonair-Route öffnet eine Peek-UI; in der gebundenen Actionliste gibt es kein direktes Peek-Gegenstück. Split-Actions sind kein Peek-Ersatz. |
| EditorRenameSymbol | editor::Rename | Gleiche Action und F2; Cross-file Vorschau/Mehrdatei-Undo bleibt offen. |
| EditorCodeAction | editor::ToggleCodeActions | Gleiche Grundabsicht und ⌘.; konkrete Picker-/Quick-Fix- und Gutterintegration noch offen. |
| EditorFormatDocument | editor::Format | Gleiche Grundabsicht; Labonair-Provider bindet ⌥⇧F, Zed bindet ⌘⇧I. |
| EditorFormatSelection | editor::Format mit Auswahl | Kein getrenntes Zed-Keymap-Gegenstück in der Actionliste; Auswahlformatierung hängt von Ziel-/Serverunterstützung ab. |
| EditorOrganizeImports | editor::OrganizeImports | Gleiche Action-Absicht; Zed ⌥⇧O, Labonair ohne Provider-Default-Binding. |
| EditorRestartLanguageServer | Language-Server-Status-/Restart-Workflow | Kein gebundener direkter Treffer in der 585-Actionliste; Statusmenü-/Command-Aktionen gesondert inventarisieren. |
| EditorStopLanguageServer | Language-Server-Status-/Stop-Workflow | Kein gebundener direkter Treffer in der 585-Actionliste; Zeds Scope und Bestätigung noch offen. |
| EditorShowDiagnostics | Diagnostics-/Problems-Ansicht; am nächsten editor::GoToDiagnostic | Zed-Keymap belegt Navigation zu Diagnosen (F8), nicht dasselbe wie Labonairs Show-Diagnostics-Request/Ansicht. |
| EditorNextDiagnostic | editor::GoToDiagnostic | Nächste Diagnose ist Zeds F8-Workflow; gleiche Navigation im lokalen Editor als Command, aber ohne Provider-Default. |
| EditorPreviousDiagnostic | editor::GoToPreviousDiagnostic | Gleiche Action-Absicht; Zed ⇧F8, lokale Provider-Metadaten ohne Default. |
| EditorNextGitChange | editor::GoToNextChange | Gleiche Editor-Git-Change-Navigation und ⌘⇧⌥⌫; Hunk-/Cursorlandung im Laufzeitvergleich offen. |
| EditorPreviousGitChange | editor::GoToPreviousChange | Gleiche Action-Absicht und ⌘⇧⌫; Laufzeit-/Hunksemantik offen. |
| EditorOpenProjectDiff | git::OpenModifiedFiles / git::ReviewDiff | Ähnliche Git-Review-Einstiege, aber kein nachgewiesenes 1:1-Gegenstück zum lokalen Project-Diff-Open. |

### Ausgewählte Zed-Editor-Shortcuts und Labonair-Gegenstücke (macOS)

Die Zed-Spalte nennt repräsentative Defaults aus gepinntem `default-macos.json`; die vollständige Actionliste ist bewusst nicht auf diese Auswahl reduziert. Labonairs Bindings unten stammen aus `crates/editor/src/command_provider.rs` oder aus Command-/Keymap-Einträgen; „nicht gebunden“ bedeutet, dass in der untersuchten Editor-Command-Metadatei kein Default-Binding steht. Ein Zed-Shortcut ist keine Zusage, dass derselbe Tastenpfad in Labonair so gerendert oder kollisionsfrei ist.

| Aufgabe | Zed macOS Default | Labonair macOS Gegenstück | Vergleich |
|---|---|---|---|
| In Datei suchen / ersetzen | `⌘F`; Replace im Find UI | `⌘F` Find Overlay; Replace Actions via Editor | Funktion vorhanden; genaue UI-/Replace-Shortcutparität offen. |
| Projektweite Suche / Replace | `⌘⇧F`; Replace `⌘⇧H` | Projektmodus im Search Overlay | Projektsuche und Treffer-Navigation vorhanden; Replace-Steuerelemente sind dort deaktiviert, Cross-file Replace fehlt. |
| Datei öffnen / Datei-Symbol | `⌘P` File Finder; `⌘⇧O` Symbol/Outline in aktuellen Docs (Actionbelegung prüfen) | `⌘⇧O` `Open File…`; `Go to Symbol…` ohne deklarierte Standardtaste | Signifikante Default-Shortcutabweichung bzw. potenzieller Konflikt; kein stilles Alias annehmen. |
| Definition / Rename | `F12`; `F2` | `F12`; `F2` | Gleicher Hauptshortcut, Zielansicht/LSP-Kapazität teilweise. |
| Code Action | `⌘.` | `⌘.` | Binding und Grundfunktion vorhanden, Gutter/Quick-Fix-Umfang offen. |
| Completion / Signature Help | `⌃Space`; `⌘I` | `⌃Space`; Signature Help kein belegter Default | Completion-Binding vorhanden; Interfaceumfang teilweise. |
| Nächste/all Matches selektieren | `⌘D`; alle `⌘⇧L` | `⌘D`; Select All Occurrences ohne deklarierten Default | Kernaktion vorhanden; alle-occurrences Keybind fehlt in Metadaten. |
| Cursor über/unter aktueller Position | `⌘⌥↑` / `⌘⌥↓` | `⌘⌥↓` für Add Cursor | Add-Cursor unterhalb vorhanden; oberhalb und Aliasprüfung offen. |
| Auswahl syntaxbasiert vergrößern/verkleinern | `⌘⌃→` / `⌘⌃←` (VS Code-Style `⌃⇧→/←`) | `⌃⇧→/←` | Beide Bindings vorhanden, jedoch muss geprüft werden, ob die Labonair-Auswahl ebenso AST-basiert ist. |
| Zeile duplizieren / bewegen | `⌥⇧↑/↓` duplizieren; `⌥↑/↓` bewegen | `⌘⇧D` duplizieren; `Move Lines` ohne Default | Funktion vorhanden; Shortcut weicht ab und Bewegungsrichtung im Command-Mapping ist aktuell festgelegt. |
| Kommentar / Einrücken | `⌘/`; `⌘[` / `⌘]` | `⌘/`; `⌘[` / `⌘]` | Gleiche Defaults; sprachabhängige Kommentarmetadaten/Fest-Indent prüfen. |
| Fold/Unfold | `⌘K ⌘L`, `⌥⌘[`/`]`, `⌘K`-Stufen | Fold Marker und Commands, kein gleiches vollständiges Shortcutset belegt | Funktion teilweise vorhanden, Keymap-/LSP-Faltung offen. |
| Projekt- und Buffer-Symbolnavigation | Datei `⌘⇧O`; Projekt `⌘T`; Outline Panel `⌘⇧B` | Go-to-Symbol Palette-Submenu, keine Standardbindung; symbolbasierte Outline-Fläche | Einzeldatei-Outline vorhanden; Projekt Index/Panel fehlen. |
| Diagnose vor/zurück | `F8` / `⇧F8` | Next/Previous Diagnostic Commands; keine Standardbindung belegt | Navigation modelliert; Quick Navigation/Popover- und Projektansicht offen. |
| Preview Markdown/SVG | Markdown `⌘⇧V`, Side `⌘K V`; SVG Action vorhanden | Preview View öffnet Markdown; kein Zed-Gleiches Side/Following-Binding | Native Renderbasis teilweise, synchronisierte Split-Aktionen fehlen. |
| Split Pane | `⌘K` dann Pfeilrichtung; Pane-Fokus Sequenzen | Editor-intern `⌘⌥]`, `⌘⌥⇧D`, `⌘⌥←/→`; Workspace besitzt zusätzlich allgemeine Splits | Andere Gruppierungsebenen und Hotkeys; Detail in Abschnitt 8. |
| Format / Organize Imports | `⌘⇧I`; `⌥⇧O` | Format Document `⌥⇧F`; Organize Imports kein deklariertes Binding | Grundaktionen vorhanden; Defaults abweichend. |
| Nächster/vorheriger Git-Change | `⌘⇧⌫` / `⌘⇧⌥⌫` | Editor Commands ohne Default-Binding belegt | Git-Hunkmodell vorhanden, Keymap fehlt. |

**Zuordnungshinweis:** Zeds `⌘P` File Finder, `⌘⇧O` Outline und `⌘⇧P` Command Palette sind aktueller öffentlicher Standard. Der lokale Labonair Editor-Command-Provider belegt für Open File dagegen `⌘⇧O`. Vor Schlussfolgerungen über tatsächlich aktive Labonair-Bindings muss daher die effektive Keymap samt globaler und feature-spezifischer Override-Priorität gesammelt werden.

## 8. Splits, Tabs und Workspace-Kompatibilität

Zeds Splits sind Pane-Gruppen innerhalb eines Workspaces: Jede Pane kann Tabs und verschiedene View-Arten enthalten, wird unabhängig fokussiert und in der Größe verändert. Labonair hat derzeit keinen gemeinsamen Splitbaum für alle Workspace-Views. `WorkspaceLayout` gilt für `TabKind::Workspace`-Terminaltabs; `Cmd-D` und `Cmd-Shift-D` öffnen dort ein weiteres Terminal im Arbeitsverzeichnis der aktiven Pane. Der Editor besitzt daneben `EditorSplitTree`, rendert aber nur die aktive Editorgruppe interaktiv und die übrigen als schreibgeschützte Spiegel desselben Dokument-Snapshots. Editor-, SSH- und andere Tab-Arten sind damit nicht automatisch Teil des Terminal-Pane-Baums. Das Paritätsdokument beschreibt einen gemeinsamen Tab/Split-Vertrag für Editor, Terminal, SCM, Git Graph, Diff, Settings und andere Views als Zielzustand, nicht als bereits erreichte Laufzeitparität.

| Zed-Funktion | Labonair-Gegenstück und Status | Nutzen | Verbundene Systeme und Verhalten |
|---|---|---|---|
| Pane nach links/rechts/oben/unten teilen, fokussieren, anpassen und Größen zurücksetzen | **Teilweise, mit engerem Laufzeitumfang.** Zeds Pane-Baum unterstützt vier Richtungen, verschachtelte Achsen, unabhängigen Fokus, Ziehen von Teilungsgrenzen und `ResetPaneSizes` zum gleichen Verteilen der Größen bei erhaltener Baumform. Labonairs Workspace-Pane-Modell unterstützt vier Richtungen und veränderliche Ratios; die registrierten Workspace-Befehle bieten aktuell aber nur Right/Down und sind an Terminal-Kontext gebunden. Der Editor-Splitbaum kann Right/Down teilen und Ratios ziehen; ein Equalize/Reset-Command ist dort nicht belegt. | Mehrere Dateien oder Werkzeuge nebeneinander lesen und den Platz zwischen ihnen passend verteilen. | Zed `PaneGroup`, Pane-Fokusaktionen, `ResetPaneSizes`, Workspace-Session; Labonair `WorkspaceLayout`, Terminal-Session, UI Divider und separate `EditorSplitTree`. `zed-refrence/zed/crates/workspace/src/pane_group.rs`, `crates/workspace/src/workspace.rs`, `crates/workspace/src/pane_group.rs`, `crates/workspace/src/views/editor.rs`. |
| Split-Modi: Pane klonen, leere Pane erzeugen oder aktive Datei verschieben | **Teilweise und anders.** Zed hat pro Richtungsaktion `ClonePane` als Standard sowie `EmptyPane` und `MovePane`; Klonen ist an `can_split`/`clone_on_split` des Items gebunden. Verschieben setzt das aktive Item in die neue Pane um. Hat die Quellpane höchstens ein Item, fällt MovePane auf eine leere Pane in Gegenrichtung zurück. Labonairs Workspace-Split erzeugt im Terminalkontext ein neues Terminal im selben Arbeitsverzeichnis; `EditorSplitTree` erstellt eine Gruppe für denselben Editor und kennt keine Datei-Klon-/Verschiebemodi. | Gezielt dieselbe Ansicht duplizieren, einen leeren Arbeitsbereich vorbereiten oder einen Tab aus der bisherigen Pane herauslösen. | Zed `pane::Split*`-Actions plus `SplitMode`; item-spezifisches Klonen, aktiver Tab und Pane-Events. Labonair Terminal-Session-/PTY-Erzeugung und Editorgruppen. `zed-refrence/zed/crates/workspace/src/pane.rs`, `zed-refrence/zed/crates/workspace/src/workspace.rs`, `crates/workspace/src/workspace.rs`, `crates/editor/src/splits.rs`. |
| Eigenständige Tabs, aktive Items, Fokus und Navigationshistorie pro Pane | **Teilweise.** Zeds jede Pane hat ihre eigene Tab-Liste, aktives Item und fokussierte Navigation; beim Klonen wird Pane-History geforkt. Labonair-Terminal-Panes haben jeweils eine Prozess-/Scrollback-Session, aber die Editor-Splitgruppen besitzen weder eigene Dateien/Tabs noch unabhängige Editoren: Nur die aktive Gruppe zeigt den interaktiven Editor, andere zeigen denselben Snapshot als schreibgeschützte Spiegel. | In jeder Pane einen anderen Arbeitskontext halten und zwischen diesen Kontexten samt Cursor-/Navigationszustand wechseln. | Zed Pane/Item-Owner, Focus Handle, pane-lokale Back/Forward-History; Labonair Workspace-Tablayout, Session-IDs, Editor-Dokument-Owner und `render_split_mirror`. `zed-refrence/zed/crates/workspace/src/pane.rs`, `zed-refrence/zed/crates/workspace/src/workspace.rs`, `crates/workspace/src/workspace.rs`, `crates/workspace/src/views/editor.rs`. |
| Gleiche Pane-Struktur für Editor, Terminal, Diff, Preview, Settings und andere Workspace-Views | **Lücke im aktuellen Laufzeitvertrag.** `WorkspaceLayout` wird nur verwendet, wenn der aktive Tab `TabKind::Workspace` ist; die aktuell registrierten Split-Befehle haben `Terminal`-Kontext. Der Editor hält seine eigene Split-Ansicht, während andere View-Arten nicht als Kinder desselben Pane-Baums belegt sind. | Zwischen Tools wechseln können, ohne Split-, Fokus- oder Größenverhalten neu lernen zu müssen. | Zed Pane nimmt unterschiedliche `ItemHandle`-Typen samt deren Split-/Clone-Verhalten auf; Labonair müsste Editor, Terminal, SCM, Git Graph, Diff, Settings und weitere Views in einem ausführbaren Owner-Vertrag zusammenführen. |
| Tabs und Dateien per Drag-and-drop in Pane oder Splitkante bewegen | **Nicht belegt als Zed-äquivalenter Ablauf.** Labonair-Tabs können in ihrer Tab-Leiste sortiert werden; `WorkspaceLayout` enthält laut Quellkommentar noch keine Bounds-/Drag-Drop-Zuordnung zum Pane-Ziel, und ein tabartübergreifendes Drop in Splitkanten wurde nicht gefunden. | Tabs mit Maus an ein Pane-Ziel verschieben, dort ablegen oder beim Ziehen direkt eine Split-Richtung wählen. | Zed erkennt Drag-Ziele auf Tabs und Pane-Flächen, zeigt Richtungs-/Drop-Zustände und übergibt Items, Projekt-Auswahlen oder externe Pfade an den Ziel-Owner. `zed-refrence/zed/crates/workspace/src/pane.rs`, `crates/workspace/src/pane_group.rs`. |
| Pane verbinden, alle Pane-Gruppen schließen oder Items seitlich tauschen | **Teilweise.** Labonair kann aktive Terminal-Panes schließen; das letzte Pane schließt den Workspace-Tab. `EditorSplitTree` kann die Gruppe schließen und die Struktur kollabieren. Pane-weite Join-All-/Join-Into-Next- und tabübergreifende Item-Tauschaktionen sind in den untersuchten Command-Providern nicht belegt. | Splitlayouts schnell vereinfachen und die Arbeitsdateien ohne erneutes Öffnen in benachbarte Panes verlagern. | Zed `JoinIntoNext`, `JoinAll`, `SwapItemLeft/Right`, Pane-Entfernung und Fokuswahl; Labonair Close-Policy, PTY-Lifecycle und Editor SplitTree. |
| Split-/Tab-Session mit Ratios, Aktivtab, Pane-Fokus und Datei-View-State wiederherstellen | **Teilweise.** Labonair speichert Terminal-Workspace-Layout und Editor-Splitbaum/aktive Gruppe; die Editorgruppen teilen jedoch Dokument und View-Snapshot. Ein gemeinsamer wiederhergestellter Pane-Baum über Editor-, Terminal-, Diff- und weitere Views ist nicht belegt. Zed-Parität für alle View- und Recoveryzustände bleibt offen. | Nach Neustart dieselbe Aufteilung, aktive Datei und Arbeitsposition fortsetzen. | Workspace-Session, Pane-Ratio/Tab-ID, Editor-Snapshots, Owner-Payload-Versionierung, fehlende Dateien und Wiederherstellungsfehler. |
| Pane schließen mit Dirty-/Busy-Owner-Prüfung und sicherer Auswahl der verbleibenden Pane | **Teilweise, in getrennten Ownern.** Workspace-Tab-Schließen beachtet Editor-Dirty-Status; Terminal-Pane-Schließen beendet die zugehörige Session; Editor-Splitgruppen teilen dasselbe Dokument und schließen nur die Gruppe. Eine einheitliche Schließbestätigung für jede View-Art ist nicht belegt. | Keine Änderungen verlieren oder laufende Werkzeuge unbeabsichtigt abbrechen; nach dem Schließen den passenden Fokus behalten. | Zed Pane-/Item-Close-Policies, Save-Intent, Preview-/Pin-Status und History; Labonair TabStore, PTY-Ende, Editor-Dokumentzustand und Notification/Dialog-Gates. |

## 9. Editornahe Einstellungsfamilien

Zeds Settings-Referenz dokumentiert sehr viele globale und pro Sprache überschreibbare Werte. Die folgende Familienmatrix fasst zentrale Bereiche zusammen; darunter steht die vollständige Feldliste der gepinnten Zed-Editor-Settingsseite. Andere Zed-Seiten wie Status Bar, Git und Language Settings enthalten zusätzliche editorrelevante Werte. Labonairs Editor-Settings sind als typed `EditorContent` unter `crates/settings-content/src/editor.rs` definiert; die UI-Felder werden zusätzlich unter `crates/settings-ui/src/schema.rs` registriert. Der Settings-Capability-Katalog markiert `editor` derzeit als **partial**. [Alle Zed Settings](https://zed.dev/docs/reference/all-settings) · [Spracheinstellungen](https://zed.dev/docs/configuring-languages)

| Zed-Einstellungsfamilie / Beispiele | Labonair Gegenstück und Status | Zweck/Nutzen | Abhängigkeiten und markierte Lücke |
|---|---|---|---|
| Editor-Schrift: `buffer_font_family`, `buffer_font_size`, `buffer_font_weight`, `buffer_font_features`, Zeilenhöhe | **Teilweise.** `editorFontFamily`, `editorFontSize`, `editorLineHeight`; OpenType-Merkmale und Gewicht nicht belegt. | Lesbarkeit, Dichte, Codefont und visuelles Raster festlegen. | UI-/Bufferfont getrennt, Font-Fallbacks, Plattformunterstützung und Live-Reload. |
| Tabs: Closeposition, Close-Button-Policy, Tab Icons, Tab Git Status, Tab Diagnostics, Aktivierung bei Schließen | **Teilweise.** Globaler Tab Owner bietet gemeinsame Tabdarstellung; dieselben Editor-Tab-spezifischen Optionen nicht gefunden. | Pane-Tab-Chrome konfigurieren und Dirty-/Statusnavigation bestimmen. | Workspace-Tab-Bar, Git/Diagnostics Events und responsive Layouts. |
| Tab Bar: sichtbar, Navigation History Buttons, Tab Bar Buttons | **Teilweise.** Keine Zed-gleiche Editor-pane Tab-Leiste; Labonair verwendet universelle Shell-Tab-Leiste. | Mehr Platz gewinnen oder panebezogene Navigation zeigen. | Explizite universelle Labonair-Tabentscheidung. |
| Toolbar-Elemente: `breadcrumbs`, `quick_actions`, `selections_menu`, `agent_review`, `code_actions` | **Teilweise.** Breadcrumb vorhanden; separate Quick Actions-/Selections-/Review-/CodeAction-Schalter nicht belegt. | Kopfzeile an Arbeitsablauf und Fensterbreite anpassen. | UI Layout, LSP, Agent Review, Workspace Scope. |
| Status Bar: active file, active language, cursor position, line endings, encoding, pending-keystrokes | **Teilweise.** Sprache, Cursorposition, Zeilenanzahl und Auswahlstatistik teils vorhanden; aktive Datei/Encoding/EOL/Keystroke UI nicht belegt. | Zustandsanzeige und direkte Editier-/Selector-Shortcuts. | Editor Status row vs. global Workspace Statusbar; File metadata und Keymap. |
| Gutter/Layout: line numbers, relative numbers, indent guides, rulers, current-line highlight, scrollbar axes/margins | **Teilweise.** Zeilennummern, relative Nummern, Rulers, Current Line Option; Nutzung von Indent Guides und Achsen/Margen nicht vollständig belegt. | Code orientieren, Einrückungen prüfen, Position im Dokument behalten. | Gutter Rendering, DisplayMap, Scroll Metrics und Vim Optionen. |
| Minimap und Scrollbar overlays: Visibility/Thumb/Border, cursor, Git, search, selected text/symbol, diagnostic severity | **Teilweise.** Minimap Setting/Renderer und Scrollbars sind vorhanden; Anzeigevarianten/Overlays nicht gleichwertig belegt. | Dichte Navigation und Position von Treffern/Fehlern/Änderungen. | Git, Search, LSP, minimap renderer und viewport geometry. |
| Wrap/Scroll: `soft_wrap`, `preferred_line_length`, `show_wrap_guides`, `scroll_beyond_last_line`, scroll sensitivities, margins | **Teilweise.** `editorWordWrap`, `editorScrollBeyondLastLine`, DisplayMap und Scrollbars vorhanden; übrige Optionen nicht belegt. | Lange Zeilen und Cursorzentrierung pro Projekt/Stil anpassen. | Language-specific settings, viewport width, font size und wrapping. |
| Einrückung: `tab_size`, `hard_tabs`, autoindent, wrap/indent rules, Kommentar-/List continuation | **Teilweise.** `editorTabSize`/`editorIndentWithTabs`; Labonair Keymap router verwendet an einigen Stellen festen 2-space unit. Sprach-/Projektspezifische Einrückungsregeln sind nicht gleichwertig belegt. | Formatkonvention pro Sprache/Projekt einhalten. | `.editorconfig`, Language configs, auto-indent und formatting. |
| Unsichtbares: `show_whitespaces` all/selection/none/boundary, konfigurierbare Map | **Teilweise.** `editorWhitespace` Wert existiert; sämtliche Modi/Zeichen und sichtbare Anwendung nicht abgeglichen. | Tabs/Spaces und trailing whitespace sichtbar prüfen. | DisplayMap, theme tokens, Editor Settings. |
| Klammern/Autoclose/Surround: Autoclose pair list, auto-surround, `always_treat_brackets_as_autoclosed`, rainbow bracket | **Teilweise.** `editorBracketMatching` plus `TypeBracket`-Intent vorhanden; Zed-Einstellungsvarianten und Tests aller Klammerzustände fehlen. | Paarzeichen, Auswahl und Klammernavigation automatisieren. | Syntax/Language, Selection, input character, editor settings. |
| Suche: Regex, Case, Whole Word, Ignored Files, Center On Match, Search on Type, Smartcase | **Teilweise.** Search-Modell unterstützt wesentliche Optionsfelder; vollständige Projektfilter-/Scrollbar-/persistent Settings-Matrix nicht nachgewiesen. | Schnelles lokales und globales Finden mit gewünschtem Trefferverhalten. | Search Overlay, Project Index, Keymap, scrollbar, ignore engine. |
| Completions/Snippets: auto-show, docs, word fallback, sorting, insert mode, debounce, snippet Tabstop, signature help | **Teilweise.** Completion und Debounce Settings/Renderer existieren; LSP/word fallback und Zed Sortier-/Akzeptanzoptionen nicht belegt. | Vorschläge passend und ohne unnötige Ablenkung anzeigen. | LSP, Snippets, editor input, key context. |
| Format/Speichern: `format_on_save`, `ensure_final_newline_on_save`, `remove_trailing_whitespace_on_save`, `line_ending`, `auto_save` | **Teilweise.** Labonairs Save-Pfad liest Format-on-save, Auto-save, Delay, Whitespace-Trim und Final-Newline aus typisierten Feldern; diese Felder sind derzeit aus der nativen Settings-UI gefiltert. EOL-/Encoding-Controls und alle Zed-Formatierungsmodi fehlen oder sind nicht belegt. | Datei-Konventionen einhalten und Änderungen dauerhaft speichern. | File lifecycle, formatter process, filesystem atomicity and prompts. |
| Diagnostics: Severity, inline/Error Lens, code actions button, `code_lens`, semantic tokens and `inlay_hints` | **Teilweise.** `editorDiagnostics`, `editorSemanticTokens`, completion/hover Flags; Inline/Error lens, code lens, inlay hints fehlen im untersuchten Settings/API-Modell. | Fehlerumfang und inline Mehrinformationen steuern. | LSP capabilities, diagnostics UI, Gutter, status and extension config. |
| Git: gutter mode/debounce/width, inline blame timing/location/padding/summary, diff base, hunk style, branch picker | **Teilweise.** `editorGitGutter`, `editorGitWordDiff`, Git hunk API; Blame, diff-base customization und voller Style-Katalog nicht belegt. | Statusänderung/Autor und Vergleichsbasis pro Editor steuern. | Git owner, branch default, commit history, editor decorations. |
| Folding/Outline/Breadcrumb/Symbolquelle: tree-sitter vs LSP, fold ranges, symbol source, Sticky Scroll | **Teilweise.** `editorStickyContext`, `editorShowOutline` und Symbols existieren; LSP-/Tree-sitter-Wechsel und voll konfigurierbare Symbolquelle nicht belegt. | Navigationsdarstellung nach Qualität der Language Extension wählen. | Tree-sitter queries, LSP, Editor Toolbar/Outline Panel. |
| Spracheinstellungen: per-language `formatter`, server order, `enable_language_server`, tab size, hard tabs, preferred length, soft wrap, file type patterns | **Lücke/teilweise.** Labonair hat erkennbare statische Languages und Project settings; vollständige Language Settings Registry und Extension precedence nicht gefunden. | Sprache und Tooling je Projekt ohne globalen Nebeneffekt konfigurieren. | Settings Store, project trust, Language Extension, server manager. |
| Theme Overrides / Syntax Theme und user styles | **Teilweise.** Labonair Themes und Editor Syntax Theme-Auswahl sind vorhanden; Zed-artige tokenbezogene Overrides/Erweiterungen sind nicht belegt. | Tokenfarben für persönlichen Arbeitsstil oder Teamkonvention festlegen. | Theme Owner, Syntax captures, dark/light, extension gallery. |
| Editor-/Pane-Defaults: `restore_on_file_reopen`, `double_click_in_multibuffer`, Preview Tabs, per-pane display | **Teilweise.** Workspace Session/Persistence existiert; einzelne Zed-View- und Preview-Tab-Felder sind nicht alle abgebildet. | Cursor-/Tabzustand restaurieren und Vorschauverhalten pro Workspace einstellen. | Workspace Session, Editor state, preview tab, settings schema. |

### Gepinnte Zed-Editor-Settingsseite: vollständige Feldliste

Die gepinnte Settingsseite deklariert die folgenden **70 sichtbaren Editor-Einstellungen**; laut Seitendeklaration sind alle im User-Settings-File verfügbar. Bezeichnungen und deklarierte Pfadkennungen stammen aus `zed-refrence/zed/crates/settings_ui/src/page_data.rs`, die Renderer-Zuordnung aus `crates/settings_ui/src/settings_ui.rs` im gepinnten Quellbaum und die Defaultwerte aus `assets/settings/default.json`. `Toggle`, `Dropdown` und `Zahlenfeld` bezeichnen den registrierten Control-Renderer; die Zeile für Custom Digraphs nutzt den JSON-Editor-Fallback. `—` bedeutet: im aktuellen Labonair-Editor-Typ kein direktes Gegenstück gefunden. Bedingte Varianten sind markiert. Labonairs lokale UI- und Laufzeitbefunde folgen nach der Tabelle.

| Zed UI-Bezeichnung | Zed UI-Control | Gepinnte `page_data`-Pfadkennung | Nächstes Labonair-Editorfeld / Befund | Gepinnter Default | Wirkung in Zed |
|---|---|---|---|---|---|
| Auto Save Mode | Dropdown; Auswahl mit bedingtem Unterfeld | `autosave$` | `editor_auto_save` (aus der Labonair-Settings-UI gefiltert; nur Ein/Aus; kein gleicher Modusumfang) | off | Legt fest, ob Speichern manuell, nach Inaktivität oder bei Fenster-/Fokuswechsel erfolgt. |
| Delay (milliseconds) | Zahlenfeld; nur bei after_delay | `autosave.after_delay.milliseconds` | `editor_auto_save_delay` (aus der Labonair-Settings-UI gefiltert; Laufzeitwert wird für den Timer gelesen) | Nur bei after_delay; Defaultmodus off, daher kein aktiver Millisekundenwert | Wartezeit ohne Eingabe vor dem automatischen Speichern; nur im Modus after_delay aktiv. |
| Show Which-key Menu | Toggle | `which_key.enabled` | — | false | Zeigt passende Folgebindungen, während ein mehrstufiges Tastenkürzel offen ist; die Pending-Key-Anzeige bleibt auch ohne Menü sichtbar. |
| Menu Delay | Zahlenfeld | `which_key.delay_ms` | — | 1000 ms | Zeit bis zum Einblenden des Which-key-Menüs. |
| Double Click In Multibuffer | Dropdown | `double_click_in_multibuffer` | — | select | Bestimmt, ob Doppelklick in einem Excerpt ein Wort auswählt oder den Ausschnitt als neuen Buffer öffnet. |
| Expand Excerpt Lines | Zahlenfeld | `expand_excerpt_lines` | — | 5 | Standardzahl der Zeilen, um die ein Multibuffer-Excerpt erweitert wird. |
| Excerpt Context Lines | Zahlenfeld | `excerpt_context_lines` | — | 2 | Standardzahl der Kontextzeilen um Multibuffer-Excerpts. |
| Expand Outlines With Depth | Zahlenfeld | `outline_panel.expand_outlines_with_depth` | `editor_show_outline` (aus der Labonair-Settings-UI gefiltert; schaltet nur die lokale Outline-Darstellung, keine Tiefe) | 100 | Tiefe, bis zu der Outline-Einträge der aktuellen Datei standardmäßig aufgeklappt sind. |
| Diff View Style | Dropdown | `diff_view_style` | — | split | Darstellungsmodus für Editor-Diffs, etwa Split oder Unified. |
| Minimum Split Diff Width | Zahlenfeld | `minimum_split_diff_width` | — | 100 | Unterhalb dieser Editorbreite wechselt Split Diff automatisch auf Unified; 0 schaltet den automatischen Wechsel aus. |
| Scroll Beyond Last Line | Dropdown | `scroll_beyond_last_line` | `editor_scroll_beyond_last_line` | one_page | Bestimmt, wie weit der Viewport unter das Dateiende scrollen darf. |
| Vertical Scroll Margin | Zahlenfeld | `vertical_scroll_margin` | — | 3 | Hält beim automatischen Cursor-Scroll eine Zahl an Zeilen oberhalb und unterhalb des Cursors sichtbar. |
| Horizontal Scroll Margin | Zahlenfeld | `horizontal_scroll_margin` | — | 5 | Hält beim Mausrad-Scroll Zeichen links und rechts des Cursors im Sichtbereich. |
| Scroll Sensitivity | Zahlenfeld | `scroll_sensitivity` | — | 1.0 | Multipliziert horizontale und vertikale Scrollbewegung. |
| Mouse Wheel Zoom | Toggle | `mouse_wheel_zoom` | — | false | Erlaubt Font-Zoom mit Mausrad plus Plattform-Hauptmodifier. |
| Fast Scroll Sensitivity | Zahlenfeld | `fast_scroll_sensitivity` | — | 4.0 | Scrollfaktor bei schnellem Scrollen mit Alt/Option. |
| Autoscroll On Clicks | Toggle | `autoscroll_on_clicks` | — | false | Erlaubt Scrollen, wenn nahe am Rand der sichtbaren Textfläche geklickt wird. |
| Sticky Scroll | Toggle | `sticky_scroll.enabled` | `editor_sticky_context` (ähnlicher Zweck, anderer Umfang) | false | Hält den umgebenden Scope oben im Editor sichtbar. |
| Auto Signature Help | Toggle | `auto_signature_help` | — | false | Zeigt automatisch ein Signatur-Popup während der Eingabe in Funktionsaufrufen. |
| Show Signature Help After Edits | Toggle | `show_signature_help_after_edits` | — | false | Zeigt Signaturhilfe nach Completion oder eingefügtem Klammerpaar. |
| Snippet Sort Order | Dropdown | `snippet_sort_order` | — | inline | Ordnet Snippets relativ zu anderen Completion-Kandidaten. |
| Enabled | Toggle | `hover_popover_enabled` | `editor_hover` | true | Schaltet das Informations-Popup beim Hover über Editor-Symbolen. |
| Delay | Zahlenfeld | `hover_popover_delay` | — | 300 ms | Wartezeit vor dem Anzeigen des Hover-Popups. |
| Sticky | Toggle | `hover_popover_sticky` | — | true | Erlaubt, mit dem Mauszeiger zum Popup zu wechseln und dessen Inhalt zu benutzen. |
| Hiding Delay | Zahlenfeld | `hover_popover_hiding_delay` | — | 300 ms | Wartezeit bis zum Ausblenden nach Verlassen des Hover-Ziels. |
| Enabled | Toggle | `drag_and_drop_selection.enabled` | — | true | Schaltet Drag-and-drop-Auswahl im Textbuffer ein oder aus. |
| Delay | Zahlenfeld | `drag_and_drop_selection.delay` | — | 300 ms | Wartezeit, bis Ziehen als Drag-and-drop und nicht als neue Textauswahl behandelt wird. |
| Show Line Numbers | Toggle | `gutter.line_numbers` | `editor_line_numbers` | true | Blendet Zeilennummern im Gutter ein oder aus. |
| Relative Line Numbers | Dropdown | `relative_line_numbers` | `editor_relative_line_numbers` | disabled | Wählt absolute, relative oder auch auf umgebrochenen Zeilen relative Nummerierung. |
| Show Runnables | Toggle | `gutter.runnables` | — | true | Blendet Run-Schaltflächen für ausführbare Elemente im Gutter ein oder aus. |
| Show Breakpoints | Toggle | `gutter.breakpoints` | — | true | Blendet Breakpoint-Marker im Gutter ein oder aus. |
| Show Bookmarks | Toggle | `gutter.bookmarks` | — | true | Blendet Bookmark-Marker im Gutter ein oder aus. |
| Show Folds | Toggle | `gutter.folds` | — | true | Blendet Faltsteuerungen im Gutter ein oder aus. |
| Min Line Number Digits | Zahlenfeld | `gutter.min_line_number_digits` | — | 4 | Reserviert mindestens so viele Zeichenplätze für die Zeilennummer im Gutter. |
| Git Gutter Width | Dropdown | `gutter.git_gutter_width$` | `editor_git_gutter` (nur Ein/Aus, keine Breite) | default | Wählt Standardbreite (fontskaliert) oder benutzerdefinierte Breite für Git-Diff-Marker. |
| Custom Width | Zahlenfeld; nur im Custom-Modus | `gutter.git_gutter_width` | `editor_git_gutter` (nur Ein/Aus, keine benutzerdefinierte Breite) | Nur im Modus custom; dort im Default nicht gesetzt | Legt bei gewählter Custom-Variante die Pixelbreite der Git-Diff-Marker fest. |
| Inline Code Actions | Toggle | `inline_code_actions` | — | true | Blendet die Code-Action-Schaltfläche am Zeilenanfang ein oder aus. |
| Show | Dropdown | `scrollbar` | — | auto | Wählt, wann die Editor-Scrollbar erscheint: automatisch, nach System, immer oder nie. |
| Cursors | Toggle | `scrollbar.cursors` | — | true | Zeigt Cursorpositionen in der Scrollbar. |
| Git Diff | Toggle | `scrollbar.git_diff` | — | true | Zeigt Git-Diff-Marker in der Scrollbar. |
| Search Results | Toggle | `scrollbar.search_results` | — | true | Zeigt Buffer-Suchtreffer in der Scrollbar. |
| Selected Text | Toggle | `scrollbar.selected_text` | — | true | Zeigt Vorkommen des ausgewählten Textes in der Scrollbar. |
| Selected Symbol | Toggle | `scrollbar.selected_symbol` | — | true | Zeigt Vorkommen des ausgewählten Symbols in der Scrollbar. |
| Diagnostics | Dropdown | `scrollbar.diagnostics` | — | all | Filtert Scrollbar-Diagnosemarker nach Severity. |
| Horizontal Scrollbar | Toggle | `scrollbar.axes.horizontal` | — | true | Erlaubt oder unterbindet die horizontale Scrollbar unabhängig. |
| Vertical Scrollbar | Toggle | `scrollbar.axes.vertical` | — | true | Erlaubt oder unterbindet die vertikale Scrollbar unabhängig. |
| Show | Dropdown | `minimap.show` | `editor_minimap` | never | Wählt automatische, dauerhafte oder keine Minimap-Anzeige. |
| Display In | Dropdown | `minimap.display_in` | — | active_editor | Zeigt die Minimap nur im fokussierten oder in allen Editoren. |
| Thumb | Dropdown | `minimap.thumb` | — | always | Zeigt den Minimap-Viewport-Thumb bei Hover oder dauerhaft. |
| Thumb Border | Dropdown | `minimap.thumb_border` | — | left_open | Wählt die Randseiten am Minimap-Viewport-Thumb. |
| Current Line Highlight | Dropdown | `minimap.current_line_highlight` | `editor_highlight_current_line` (Editorzeile, nicht Minimap-Overlay) | null (erbt Editorzeile) | Wählt, ob die aktuelle Zeile in der Minimap markiert oder die Editor-Einstellung geerbt wird. |
| Max Width Columns | Zahlenfeld | `minimap.max_width_columns` | — | 80 | Begrenzt die Minimapbreite in Textspalten. |
| Breadcrumbs | Toggle | `toolbar.breadcrumbs` | Pfad-/Symbol-Breadcrumbs gerendert, kein typisiertes Schaltfeld gefunden | true | Blendet Breadcrumbs in der Editor-Toolbar ein oder aus. |
| Quick Actions | Toggle | `toolbar.quick_actions` | — | true | Blendet Quick-Action-Schaltflächen in der Editor-Toolbar ein oder aus. |
| Selections Menu | Toggle | `toolbar.selections_menu` | — | true | Blendet das Selections-Menü in der Editor-Toolbar ein oder aus. |
| Agent Review | Toggle | `toolbar.agent_review` | — | true | Blendet Agent-Review-Schaltflächen in der Editor-Toolbar ein oder aus. |
| Code Actions | Toggle | `toolbar.code_actions` | — | false | Blendet Code-Action-Schaltflächen in der Editor-Toolbar ein oder aus. |
| Default Mode | Dropdown | `vim.default_mode` | `editor_vim_mode` (Vim an/aus, nicht gleiche Modusauswahl) | normal | Wählt den Startmodus von Vim. |
| Toggle Relative Line Numbers | Toggle | `vim.toggle_relative_line_numbers` | `editor_relative_line_numbers` (kein Vim-spezifischer Umschaltwert) | false | Erlaubt das Umschalten relativer Zeilennummern in Vim. |
| Use System Clipboard | Dropdown | `vim.use_system_clipboard` | — | always | Legt fest, wann Vim das System-Clipboard verwendet. |
| Use Smartcase Find | Toggle | `vim.use_smartcase_find` | `vim_smartcase` (Vim-Einstellung; Suchverhalten nicht vollständig abgeglichen) | false | Schaltet Smartcase für die Vim-Suche ein oder aus. |
| Global Substitution Default | Toggle | `vim.gdefault` | — | false | Lässt :substitute standardmäßig alle Zeilentreffer ersetzen; der g-Flag kehrt das Verhalten um. |
| Highlight on Yank Duration | Zahlenfeld | `vim.highlight_on_yank_duration` | — | 200 ms | Dauer der visuellen Markierung nach Vim-Yank. |
| Regex Search | Toggle | `vim.use_regex_search` | — | true | Wählt Regex als Standard für Vim-Suche. |
| Show Edit Predictions in Normal Mode | Toggle | `vim.show_edit_predictions_in_normal_mode` | — | false | Erlaubt Edit Predictions auch im Normalmodus; standardmäßig werden sie in Insert/Replace angeboten. |
| Cursor Shape - Normal Mode | Dropdown | `vim.cursor_shape.normal` | `editor_cursor_style` (eine Form für den Editor, nicht je Modus) | block | Wählt Cursorform im Vim-Normalmodus. |
| Cursor Shape - Insert Mode | Dropdown | `vim.cursor_shape.insert` | `editor_cursor_style` (keine modusspezifische Form) | inherit | Wählt Cursorform im Vim-Insertmodus oder erbt die Editorform. |
| Cursor Shape - Replace Mode | Dropdown | `vim.cursor_shape.replace` | `editor_cursor_style` (keine modusspezifische Form) | underline | Wählt Cursorform im Vim-Replace-Modus. |
| Cursor Shape - Visual Mode | Dropdown | `vim.cursor_shape.visual` | `editor_cursor_style` (keine modusspezifische Form) | block | Wählt Cursorform im Vim-Visual-Modus. |
| Custom Digraphs | Kein natives Control; öffnet settings.json | `vim.custom_digraphs` | — | {} | Definiert benutzerdefinierte Vim-Digraph-Abbildungen. |

### Weitere editorbezogene Zed-Settings außerhalb der 70 Editor-Seitenfelder

Die vorangehenden 70 Zeilen sind die Controls von Zeds eigener Editor-Seite. `EditorSettingsContent` enthält zusätzlich direkte Editorwerte, die auf den Seiten **Appearance**, **Search & Files**, **Languages & Tools** und **General** auftauchen oder nur über die Settings-Datei konfiguriert werden. Die folgende Matrix ergänzt diese Werte; `search` und `jupyter` sind bis zu ihren konkreten Schlüsseln aufgefächert. UI-Controltypen und Pfadkennungen stammen aus der Settings-UI. Die Defaultspalte gibt den gepinnten `default.json`-Wert wieder. Zed flacht `EditorSettingsContent` im serialisierten `SettingsContent` ab: Manche `page_data`-Kennungen tragen deshalb ein internes `editor.`-Präfix, das im ausgelieferten `default.json` nicht Teil des Schlüssels ist.

| Zed `page_data`-Pfadkennung | Zed Settings-Oberfläche / Control | Gepinnter `default.json`-Wert | Zweck in Zed | Nächstes Labonair-Feld / Befund und verbundene Systeme |
|---|---|---|---|---|
| `cursor_blink` | Appearance · Toggle | true | Blinken des Editor-Cursors ein- oder ausschalten. | `editorCursorBlink` ist ein sichtbares Labonair-Control und wird vom Editor-Caret gelesen. |
| `cursor_animation.enabled` | Appearance · Toggle | false | Cursorbewegungen weich animieren. | Keine separate Cursorbewegungs-Animation im Editor-Settings-Typ gefunden. |
| `cursor_shape` | Appearance · Dropdown | bar | Cursorform für den Editor wählen (`bar`, `block`, `underline`, `hollow`). | `editorCursorStyle` ist sichtbar; Labonair hat Bar/Block/Underline, aber keinen Hollow-Wert. |
| `current_line_highlight` | Appearance · Dropdown | all | Umfang der Hervorhebung der aktuellen Zeile wählen. | `editorHighlightCurrentLine` ist ein sichtbarer Ein/Aus-Schalter; keine Auswahl der Zed-Varianten. |
| `selection_highlight` | Appearance · Toggle | true | Alle Vorkommen des aktuell ausgewählten Texts hervorheben. | Kein gleiches persistentes Feld gefunden; Suchtreffer-Markierung ist ein anderer Zustand. Editor-Display und Search. |
| `rounded_selection` | Appearance · Toggle | true | Abgerundete Ecken für Textauswahl verwenden. | Kein entsprechendes Editor-Settings-Feld; Form der Auswahl ist nicht separat einstellbar. |
| `minimum_contrast_for_highlights` | Appearance · Zahlenfeld | 45 | Mindestkontrast für Text über Highlight-Hintergründen nach APCA festlegen; 0 deaktiviert Anpassung. | Kein automatischer Mindestkontrastwert im Labonair Editor-/Theme-Vertrag gefunden. Themefarben und Text-Highlights. |
| `multi_cursor_modifier` | Appearance · Dropdown | alt | Maustaste wählen, mit der zusätzliche Cursor erstellt werden. | `EditorAddCursor` existiert als Command; ein konfigurierbarer Mausmodifier ist nicht belegt. Editor Pointer-Events und SelectionSet. |
| `lsp_highlight_debounce` | Languages & Tools · Zahlenfeld (ms) | 75 | Verzögerung vor der LSP-Abfrage für Highlights am aktuellen Cursorort. | Kein gleiches Highlight-Debounce-Feld; LSP, Cursorposition und Symbol-Highlights. |
| `search.button` | Search & Files · Toggle | true | Projektsuche-Schaltfläche in der Statusleiste anzeigen. | Search Overlay und `⌘F`/Projektsuche sind vorhanden; kein gleiches Statusleisten-Control belegt. Search-Owner und Statusbar. |
| `search.whole_word` | Search & Files · Toggle | false | Whole-Word als Standard für neue Suchen wählen. | Search Overlay hat Whole-Word-Option; kein persistentes Standardfeld im Labonair-Settingsmodell. |
| `search.case_sensitive` | Search & Files · Toggle | false | Groß-/Kleinschreibung als Suchstandard wählen. | Search Overlay kann Case umschalten; kein persistentes Standardfeld im Labonair-Settingsmodell. |
| `use_smartcase_search` | Search & Files · Toggle | false | Case-Sensitive automatisch aktivieren, wenn die Query Großbuchstaben enthält. | `SearchOptions` unterstützt Smartcase (im Editor-Modell standardmäßig aktiv); `vimSmartcase` gilt für Vim-Suche, keine allgemeine persistente Zed-gleiche Einstellung. |
| `search.include_ignored` | Search & Files · Toggle | false | Ignorierte Dateien standardmäßig in Projektsuchergebnissen einschließen. | Kein gleiches Include-Ignored-Control oder -Setting im Project Search belegt. Project roots und Ignore-Regeln. |
| `search.regex` | Search & Files · Toggle | false | Regex als Suchstandard für neue Suchvorgänge wählen. | Regex-Suche ist im Search Overlay vorhanden; kein persistentes Standardfeld im Labonair-Settingsmodell. |
| `search_wrap` | Search & Files · Toggle | true | Am Ende der Trefferliste mit der Suche am anderen Ende fortfahren. | Kein gleiches Wrap-Setting; Treffer-Navigation im Search Overlay ist vorhanden, deren Gleichheit zum Zed-Wrap-Verhalten bleibt offen. |
| `editor.search.center_on_match` | Search & Files · Toggle | false | Aktuellen Suchtreffer im Editor zentrieren. | Kein entsprechendes persistentes Setting; Search Overlay navigiert zu Treffern, genaue Scrollposition ist nicht gleichgesetzt. |
| `editor.search.search_on_type` | Search & Files · Toggle | true | Projektsuche beim Tippen aktualisieren, ohne Enter abzuwarten. | Projektquery aktualisiert asynchron; kein Control für Enter-vs.-Tippen gefunden. Project Search und Index. |
| `seed_search_query_from_cursor` | Search & Files · Dropdown | always | Bestimmen, wann Text unter dem Cursor als neue Suchquery eingesetzt wird. | Kein persistentes Seed-Verhalten im Labonair-Editor-Settingsmodell gefunden. Selection, Cursor und Search Overlay. |
| `redact_private_values` | General · Toggle | false | Werte in als privat markierten Dateien visuell verbergen; Originalwerte bleiben kopierbar. | Keine private-file/secret-redaction Darstellung im Editor gefunden. Private-file rules und Text rendering. |
| `middle_click_paste` | Languages & Tools · Language Settings · Toggle; deklarierter Pfad `languages.$(language).editor.middle_click_paste`, Pick/Write greifen aber auf `settings_content.editor.middle_click_paste` zu | true (Linux) | Mittelklick-Einfügen auf Linux aktivieren. | Kein entsprechender plattformabhängiger Editorwert gefunden. Die Zed-Pick/Write-Funktionen adressieren ein globales Editorfeld, obwohl der JSON-Pfad sprachspezifisch aussieht; tatsächliche Persistenz/Scope an der Settings-Laufzeit offen. Linux Pointer-Events und OS-Clipboard. |
| `language_detection` | Languages & Tools · Toggle | true | Sprache eines unbenannten Buffers anhand seines Inhalts erkennen; explizit ausgewählte Sprache beibehalten. | Labonair erkennt statische Sprachfamilien vor allem über Pfad/Dateiendung; keine gleichwertige Untitled-Buffer-Option. Language Resolver. |
| `go_to_definition_fallback` | Languages & Tools · Dropdown | find_all_references | Bei leerer Definition-Antwort eine konfigurierte Folgeaktion verwenden; Default ist Find All References. | Definition und References sind Commands; kein konfigurierbarer Fallback belegt. LSP Requests und Navigation. |
| `go_to_definition_scroll_strategy` | Languages & Tools · Dropdown | center | Zielposition bei Definition-/Referenznavigation in den Viewport bringen. | Kein scroll strategy Setting; genaue Zielzentrierung nicht als User-Option belegt. LSP Navigation und Editor Viewport. |
| `lsp_results_location` | Languages & Tools · Dropdown | multi_buffer | Mehrere Ergebnisse von Definition/Implementation/References im Multi-Buffer oder an einem anderen Ergebnisziel zeigen. | Referenzen können angefragt werden; gemeinsamer Multi-Buffer-Ergebnis-View fehlt. LSP, Workspace und Multi-Buffer. |
| `diagnostics_max_severity` | Languages & Tools · Dropdown | all | Diagnosedarstellung nach Severity filtern. | `editorDiagnostics` schaltet Diagnosen global ein/aus; keine Severity-Auswahl gefunden. LSP und Editor Decorations. |
| `code_lens` | Languages & Tools · Dropdown; globales EditorSettingsContent-Feld, kein Sprach-Override | off | LSP Code Lens ausblenden, inline anzeigen oder im Code-Action-Menü anbieten. | Kein CodeLens-Typ, -Renderer oder Settingsfeld gefunden. LSP CodeLens und Editor Decorations. |
| `lsp_document_colors` | Languages & Tools · Dropdown | inlay | LSP Document Colors inline bzw. als Inlay-Preview darstellen. | Kein Document-Color-Werttyp oder Renderer gefunden. LSP `documentColor` und Theme. |
| `lsp_document_links` | Kein Settings-Control in page_data.rs gefunden | true | LSP-Dokumentlinks abfragen und im Editor anzeigen. | Kein Document-Link-Provider, Renderer oder Link-Open-Flow gefunden. LSP und externe URI-Aktionen. |
| `jupyter.enabled` | Kein Settings-Control in page_data.rs gefunden | true | Jupyter-Editor-/Kernelintegration aktivieren. | Kein Jupyter-REPL- oder Kernel-Owner gefunden. Kernel lifecycle, Notebook Views und Execution. |
| `jupyter.kernel_selections` | Kein Settings-Control in page_data.rs gefunden | `{}` | Standardkernel pro Sprache auswählen. | Keine Kernelregistrierung oder Sprach-/Kernelzuordnung im Labonair Editor gefunden. |
| `completion_menu_scrollbar` | Languages & Tools · Completion · Dropdown; UI-Pfadkennung `editor.completion_menu_scrollbar`, Root-Schlüssel im Paketdefault, User-Scope | never | Sichtbarkeit der Completion-Menü-Scrollbar wählen. | Kein separates Scrollbar-Setting für Completion-Menüs gefunden. Completion Popup und UI geometry. |
| `completion_detail_alignment` | Languages & Tools · Completion · Dropdown; UI-Pfadkennung `editor.completion_detail_alignment`, Root-Schlüssel im Paketdefault, User-Scope | left | Detailtext in Completion-Einträgen links oder rechts ausrichten. | Completion-Details/Arten werden im lokalen Popup gerendert, aber Ausrichtung ist nicht konfigurierbar. LSP Completion und Popup. |
| `completion_menu_item_kind` | Languages & Tools · Completion · Dropdown; UI-Pfadkennung `editor.completion_menu_item_kind`, Root-Schlüssel im Paketdefault, User-Scope | off | Completion-Item-Kategorien gar nicht oder als farbige Symbol-Badges zeigen. | Labonair rendert Textlabels für Completion-Kinds; keine Badge-Sichtbarkeit/-Darstellung als Settingswert gefunden. |

Die `EditorSettingsContent`-Kommentare nennen für `diagnostics_max_severity` den Default `warning`, während der gepinnte `default.json`-Wert `all` lautet. Die Tabelle verwendet den ausgelieferten JSON-Wert; die Abweichung zwischen Schema-Kommentar und Paketdefault ist damit sichtbar.

**Labonair: Settings-UI und Laufzeit:** `EditorContent` enthält 40 typisierte Editorwerte. 28 werden in den nativen Editor-, Language-Services- und Git-Settingsgruppen angeboten; `crates/settings-ui/src/schema.rs` filtert 12 weitere ausdrücklich aus `all_fields()`. Der Schema-Kommentar nennt diese zwölf kompatibilitätsbedingt und ohne Laufzeitconsumer. Die aktuelle Quellverwendung weicht davon ab: elf werden im Workspace-Editor gelesen. Nur für `editorBracketMatching` fand sich kein Aufruf des zugehörigen Settings-Getters; die getrennte Bracket-Matching-Funktion und Vim-`%`-Bewegung belegen keine Wirkung dieses Schalters.

| Labonair JSON-Feld | Native Settings-UI | Im aktuellen Code beobachtete Verwendung |
|---|---|---|
| `editorIndentationGuides` | Gefiltert | `EditorView::display_settings()` liest den Wert; der Display-Pfad zeichnet Indent Guides. |
| `editorBracketMatching` | Gefiltert | Getter vorhanden, aber kein Aufruf des Getters gefunden; Matching-Hilfsfunktion ist nicht an diesen Schalter gebunden. |
| `editorFormatOnSave` | Gefiltert | Save-Pfad fordert bei aktiviertem Wert Language-Service-Formatting vor dem Schreiben an. |
| `editorAutoSave` | Gefiltert | Dirty-Dateien mit Pfad erhalten einen verzögerten Save-Task nach Edits. |
| `editorAutoSaveDelay` | Gefiltert | Der Wert steuert die Wartezeit des Auto-save-Timers. |
| `editorTrimTrailingWhitespace` | Gefiltert | Wird in die Save Policy übergeben und dort als Save-Normalisierung verwendet. |
| `editorInsertFinalNewline` | Gefiltert | Wird in die Save Policy übergeben und dort als Save-Normalisierung verwendet. |
| `editorShowCursorPosition` | Gefiltert | Steuert, ob Zeile und Spalte in der Editor-Statuszeile gerendert werden. |
| `editorShowSelectionStats` | Gefiltert | Steuert die Anzeige von Auswahlzahl und Zeichenanzahl in der Editor-Statuszeile. |
| `editorShowOutline` | Gefiltert | Steuert die lokale Dokument-Outline neben dem Buffer. |
| `editorAutocompleteDebounceMs` | Gefiltert | Verzögert den nächsten lokalen Completion-Request. |
| `editorMaxFileSizeMb` | Gefiltert | Wird als Byte-Limit beim Laden einer Editor-Datei übergeben. |

Damit ist „typisiert“, „in der nativen Settings-UI sichtbar“ und „im Editor-Laufzeitpfad verwendet“ getrennt erfasst. Die Filterliste und die elf konkreten Verbraucher stehen im aktuellen Quellbaum nebeneinander; ihr Widerspruch mit dem Kommentar in `schema.rs` bleibt eine dokumentierte Befundlage und wurde in diesem Vergleich nicht im Produktcode geändert.

**Weitere typisierte Labonair Editor-Settings:** Schriftfamilie/-größe/-höhe, Tabgröße/Spaces-Tabs, Wrap, absolute/relative Zeilennummern, Whitespace, Minimap, Rulers, Beyond-EOF Scroll, Sticky Context, Diagnostics, Semantic Tokens, Git Gutter/Word Diff, Completion, Hover, Vim, Editor Theme, Vim hlsearch/incsearch/smartcase, Current-Line Highlight, Cursor Blink/Intervall/Form. Ob sichtbare Controls und Laufzeitverhalten Zed gleichwertig sind, bleibt pro Feld separat zu prüfen.

### Zed: Per-Language-Settings und ausgelieferte Sprachprofile

Zeds `AllLanguageSettingsContent` verbindet globale Language-Defaults, eine Map nach Sprachname und optionale Datei-/Sprachzuordnungen. Der gepinnte Typ `LanguageSettingsContent` hat 43 direkte Optionen; die Tabelle fasst jede davon einschließlich der verschachtelten Optionen auf. Die Settings-Seite **Languages & Tools** erstellt dynamische Unterseiten für verfügbare Sprachen. Jede Unterseite kombiniert Editor-, Language-Service- und Edit-Prediction-Werte. Für die meisten sprachspezifischen Felder liest die UI erst den Sprach-Override und fällt dann auf `defaults` zurück; beim Schreiben wird der aktiven Sprache zugeordnet. Die Einträge sind überwiegend für User- und Project-Dateien freigegeben. Die Standarddatei enthält zusätzlich 44 konkrete Einträge unter `languages`.

Die UI macht die meisten skalaren/Enum-Felder editierbar. In `page_data.rs` sind diese strukturierten Felder ausdrücklich mit `SettingField::unimplemented()` markiert: `wrap_guides`, `formatter`, `code_actions_on_format`, `whitespace_map.space/tab`, `inlay_hints.toggle_on_modifiers_press`, `tasks.variables`, `debuggers`, `language_servers` und `prettier.plugins/options`. Der Settings-Renderer zeigt dafür eine Schaltfläche **Edit in settings.json** statt eines nativen Feld-Controls; sie sind damit JSON-konfigurierbar, aber nicht direkt im Formular editierbar. `middle_click_paste` ist ein prüfenswerter Sonderfall: Die Zeile verwendet einen sprachspezifisch aussehenden `json_path`, aber ihre Pick-/Write-Funktionen greifen auf das globale `editor.middle_click_paste` zu. Die drei Completion-Layoutwerte aus der vorherigen Tabelle haben ebenfalls globale `editor.*`-Pfade und User-Scope, obwohl ihre Controls im Language-Settingsdatenpfad deklariert sind.

| Zed-Sprachoptionen und gepinnte Werte aus `default.json` | Zweck | Nächstes Labonair-Feld und Laufzeitbefund |
|---|---|---|
| `tab_size=4`; `hard_tabs=false`; `soft_wrap=none`; `preferred_line_length=80`; `show_wrap_guides=true`; `wrap_guides=[]` | Tabbreite, Tabs-vs.-Spaces, Wrap-Modus und Wrap-Spalten steuern. | `editorTabSize`, `editorIndentWithTabs` und `editorWordWrap` sind globale Werte. Wrap ist dort nur boolesch; Zeilenlänge und Wrap-Guides sind nicht sprachspezifisch einstellbar. |
| `indent_guides.enabled=true`; `line_width=1`; `active_line_width=1`; `coloring=fixed`; `background_coloring=disabled` | Indent-Guides pro Sprache ein-/ausschalten und Linienbreite sowie Färbung wählen. | `editorIndentationGuides` ist ein globaler Ein/Aus-Wert. Breite und Farbmodus fehlen. |
| `format_on_save=off` im Paket; `remove_trailing_whitespace_on_save=true`; `ensure_final_newline_on_save=true`; `line_ending=detect` | Formatierung, Whitespace-/Final-Newline-Normalisierung und Zeilenenden je Sprache bestimmen. | Labonair hat globale `editorFormatOnSave`, `editorTrimTrailingWhitespace` und `editorInsertFinalNewline`; ihre gepinnten Defaults sind jeweils false. Eine Zeilenende-Strategie pro Sprache fehlt. `LanguageSettingsContent` dokumentiert für `format_on_save` dagegen Default `on`; das Paket setzt global `off`. |
| `formatter=auto`; `prettier.allowed=false`; `prettier.parser=""`; `prettier.plugins=[]`; `prettier.options` (freie Formatter-Options-Map) | Formatterkette/Formatterauswahl und Prettier-Parser, Plugins und Optionen je Sprache steuern. | Labonair kann den injizierten Language-Service fürs Formatieren anfragen. Eine per-Sprache Formatterwahl, Prettier-Integration und Options-Map sind nicht belegt. |
| `jsx_tag_auto_close.enabled=true`; `use_on_type_format=true`; `code_actions_on_format={}` | JSX-Tags schließen und On-Type-Formatierung bzw. Code Actions beim Formatieren aktivieren. | Kein JSX-Tag-Autoclose- oder On-Type-Format-Setting gefunden. Ein expliziter Organize-Imports-Command existiert, aber keine pro-Sprache Code-Action-on-format-Zuordnung. |
| `enable_language_server=true`; `language_servers=["..."]`; `semantic_tokens=off`; `document_folding_ranges=off`; `document_symbols=off`; `linked_edits=true` | LSP je Sprache aktivieren und Provider auswählen; Semantikfarben, Folding, Symbolquelle und verknüpfte Edits wählen. In Zeds Serverliste bedeutet `...` die übrigen registrierten Server; `!name` schließt einen aus. | Labonair hat 22 statische `Language`-Varianten und eine Provider-/Prozess-Registry. Serverprozesse werden explizit injiziert; es gibt keine Settings-Map zur Serverauswahl je Sprache. Semantic Tokens sind nur global schaltbar. LSP-Folding-Ranges können im Servicevertrag vorkommen, die sichtbare lokale Faltung wird aber aus Symbolen erzeugt; eine Providerwahl fehlt. Document Symbols stammen im lokalen Editor aus sprachspezifischen Parsern. Linked Edits sind nicht belegt. |
| `allow_rewrap=in_comments`; `show_edit_predictions=true`; `edit_predictions_disabled_in=[]` | Rewrap auf Kommentar, Auswahl oder beliebigen Text begrenzen; Edit Predictions global je Sprache oder Scope verbergen. | Kein Rewrap-Command oder edit-prediction-Provider im Editor gefunden. Vim `J` ist Join Lines und kein Rewrap-Gegenstück. |
| `show_whitespaces=selection`; `whitespace_map.space=•`; `whitespace_map.tab=→` | Sichtbarkeit von Whitespace und die gezeichneten Space-/Tab-Zeichen bestimmen. | `editorWhitespace` steuert die lokale Anzeige global. Pro-Sprache-Override und Zeichenabbildung fehlen. |
| `extend_comment_on_newline=true`; `extend_list_on_newline=true`; `indent_list_on_tab=true` | Kommentarpräfixe bzw. Markdown-Listen beim Zeilenumbruch fortsetzen und Listeneinträge per Tab einrücken. | Keine entsprechenden sprach-/Markdown-spezifischen Editieroptionen oder Fortsetzungsaktionen im Editor-Owner gefunden. |
| `inlay_hints.enabled=false`; `show_value_hints=true`; `show_type_hints=true`; `show_parameter_hints=true`; `show_other_hints=true`; `show_background=false`; `edit_debounce_ms=700`; `scroll_debounce_ms=50`; `toggle_on_modifiers_press`: alle Modifier false im Paket (Schema-Kommentar: null) | LSP-Inlay-Hints und Hint-Arten, Hintergrund, Abruf-Debounce sowie Modifier-Verhalten konfigurieren. | Inlay-Hints-Request, Inline-Renderer und Settingswerte wurden im Labonair-Editor nicht gefunden. |
| `use_autoclose=true`; `use_auto_surround=true`; `always_treat_brackets_as_autoclosed=false` | Paarzeichen einfügen/ausgewählten Text umschließen und Skip-/Delete-Verhalten autogeschlossener Zeichen festlegen. | Bracket-Matching und Vim-Paarbewegung sind vorhanden; automatische Paar-Eingabe, Surround und diese Regeln sind nicht belegt. |
| `auto_indent=syntax_aware`; `auto_indent_on_paste=true` | Syntaxbewusste, beibehaltene oder deaktivierte Auto-Einrückung sowie Paste-Reindent wählen. | Es gibt eine kleine `auto_indent`-Hilfsfunktion, die anhand der vorherigen Zeile und öffnender Klammern eine Einrückung berechnet. Kein persistentes Modus-Setting und kein entsprechender Paste-Schalter gefunden. |
| `tasks.variables={}`; `tasks.enabled=true`; `tasks.prefer_lsp=true` | Sprachbezogene Taskvariablen und die Verwendung von LSP-Tasks priorisieren oder deaktivieren. | Terminal und Befehlsausführung sind eigenständige Flächen; keine pro-Sprache Taskkonfiguration im Editor-/Settingsmodell belegt. |
| `show_completions_on_input=true`; `show_completion_documentation=true`; `completions.words=fallback`; `words_min_length=3`; `lsp=true`; `lsp_fetch_timeout_ms=0`; `lsp_insert_mode=replace_suffix` | Automatische Vorschläge/Dokumentation und Wörter- bzw. LSP-Quellen, Mindestlänge, Timeout und Einfügemodus je Sprache konfigurieren. | `editorCompletion` und `editorAutocompleteDebounceMs` sind global. Der Editor verarbeitet Completion-Ergebnisse und Dokumentation, hat aber keine gleichwertigen sprachbezogenen Quell-, Timeout-, Insert- oder Dokumentations-Schalter. |
| `debuggers=[]`; `word_diff_enabled=true`; `colorize_brackets=false` | Bevorzugte Debugger pro Sprache, Wort-Diff und verschachtelte Bracket-Farbgebung steuern. | Debugger-Owner/-Workflow fehlt. `editorGitWordDiff` ist ein globaler Git-Diff-Schalter; Labonairs `editorBracketMatching` ist ein separates, aktuell nicht aufgerufenes Setting und kein Farbmodus. |

Zeds Settings-Merge überträgt globale Language-Defaults auf Spracheinträge und wendet danach explizite Einstellungen des jeweiligen Sprachblocks an. Vorhandene `language_servers`-Listen eines Sprachblocks bleiben beim globalen Merge erhalten; ein expliziter Sprachwert kann damit die globale Liste gezielt übersteuern. Das unterscheidet sich von Labonairs Settings-Layern: `SettingsLayer::Language(String)` ist im Store kommentiert als Platzhalter ohne Language-Resolver. Außerhalb von Layer-Form-/Reihenfolgetests fand ich weder einen Loader noch einen Workspace-Editor-Verbraucher dieses Layers. Sein Payload ist außerdem das normale `SettingsContent`, keine Map von Sprachname auf Sprachoptionen. Die Projekt-Whitelist kann ausgewählte globale `editor.*`-Werte erlauben, aber keine Spracheinträge.

Die vollständigen, expliziten Sprachblöcke im gepinnten Paketdefault sind:

| Sprache | Gepinnte Overrides in `default.json` |
|---|---|
| Astro | `{"format_on_save":"on","language_servers":["astro-language-server","..."],"prettier":{"allowed":true,"plugins":["prettier-plugin-astro"]}}` |
| Blade | `{"prettier":{"allowed":true}}` |
| C | `{"use_on_type_format":false,"prettier":{"allowed":false}}` |
| C++ | `{"use_on_type_format":false,"prettier":{"allowed":false}}` |
| CSharp | `{"language_servers":["roslyn","!csharp-ls","!omnisharp","..."]}` |
| CSS | `{"prettier":{"allowed":true}}` |
| Dart | `{"format_on_save":"on","tab_size":2}` |
| Diff | `{"show_edit_predictions":false,"remove_trailing_whitespace_on_save":false,"ensure_final_newline_on_save":false}` |
| EEx | `{"format_on_save":"on","language_servers":["elixir-ls","!expert","!dexter","!next-ls","!lexical","..."]}` |
| Elixir | `{"format_on_save":"on","language_servers":["elixir-ls","!expert","!dexter","!next-ls","!lexical","!emmet-language-server","..."]}` |
| Elm | `{"format_on_save":"on","tab_size":4}` |
| Erlang | `{"language_servers":["erlang-ls","!elp","..."]}` |
| Git Commit | `{"allow_rewrap":"anywhere","soft_wrap":"editor_width","preferred_line_length":72}` |
| Go | `{"format_on_save":"on","hard_tabs":true,"code_actions_on_format":{"source.organizeImports":true},"debuggers":["Delve"]}` |
| GraphQL | `{"format_on_save":"on","prettier":{"allowed":true}}` |
| HEEx | `{"format_on_save":"on","language_servers":["elixir-ls","!expert","!dexter","!next-ls","!lexical","..."]}` |
| HTML | `{"prettier":{"allowed":true}}` |
| HTML+ERB | `{"language_servers":["herb","!ruby-lsp","..."]}` |
| Java | `{"prettier":{"allowed":true,"plugins":["prettier-plugin-java"]}}` |
| JavaScript | `{"language_servers":["!typescript-language-server","vtsls","..."],"prettier":{"allowed":true}}` |
| JSON | `{"prettier":{"allowed":true}}` |
| JSONC | `{"prettier":{"allowed":true}}` |
| JS+ERB | `{"language_servers":["!ruby-lsp","..."]}` |
| Kotlin | `{"format_on_save":"on","language_servers":["!kotlin-language-server","kotlin-lsp","..."]}` |
| LaTeX | `{"formatter":"language_server","language_servers":["texlab","..."],"prettier":{"allowed":true,"plugins":["prettier-plugin-latex"]}}` |
| Markdown | `{"use_on_type_format":false,"remove_trailing_whitespace_on_save":false,"allow_rewrap":"anywhere","soft_wrap":"editor_width","completions":{"words":"disabled"},"prettier":{"allowed":true}}` |
| PHP | `{"language_servers":["phpactor","!intelephense","!phptools","!phpantom","..."],"prettier":{"allowed":true,"plugins":["@prettier/plugin-php"],"parser":"php"}}` |
| Plain Text | `{"allow_rewrap":"anywhere","soft_wrap":"editor_width","completions":{"words":"disabled"}}` |
| Proto | `{"language_servers":["buf","!protols","!protobuf-language-server","..."]}` |
| Python | `{"code_actions_on_format":{"source.organizeImports.ruff":true},"formatter":{"language_server":{"name":"ruff"}},"debuggers":["Debugpy"],"language_servers":["basedpyright","ruff","!ty","!pyrefly","!pyright","!pylsp","..."]}` |
| Ruby | `{"language_servers":["solargraph","!ruby-lsp","!rubocop","!sorbet","!steep","!kanayago","!fuzzy-ruby-server","..."]}` |
| Rust | `{"format_on_save":"on","debuggers":["CodeLLDB"]}` |
| SCSS | `{"prettier":{"allowed":true}}` |
| Starlark | `{"format_on_save":"on","language_servers":["starpls","!buck2-lsp","!tilt","..."]}` |
| Svelte | `{"language_servers":["svelte-language-server","..."],"prettier":{"allowed":true,"plugins":["prettier-plugin-svelte"]}}` |
| TSX | `{"language_servers":["!typescript-language-server","vtsls","..."],"prettier":{"allowed":true}}` |
| Twig | `{"prettier":{"allowed":true}}` |
| TypeScript | `{"language_servers":["!typescript-language-server","vtsls","..."],"prettier":{"allowed":true}}` |
| SystemVerilog | `{"language_servers":["slang","!verible","!veridian","!svls","..."],"use_on_type_format":false}` |
| Vue.js | `{"language_servers":["vue-language-server","vtsls","..."],"prettier":{"allowed":true}}` |
| XML | `{"prettier":{"allowed":true,"plugins":["@prettier/plugin-xml"]}}` |
| YAML | `{"prettier":{"allowed":true}}` |
| YAML+ERB | `{"language_servers":["!ruby-lsp","..."]}` |
| Zig | `{"format_on_save":"on","language_servers":["zls","..."]}` |

Die Overrides sind die tatsächlich ausgelieferte Map, nicht eine Behauptung, dass Labonair dieselben Sprachprofile übernehmen sollte. Die Referenz kann sich zwischen Revisionen ändern; der hier verwendete Snapshot ist der oben gepinnte Commit.

### Verbundene Settings-Seiten: 206 weitere Controls

Zusätzlich zur Editor-Seite und den bereits aufgefächerten `EditorSettingsContent`- und `LanguageSettingsContent`-Feldern inventarisiert diese Tabelle **206 Controls** aus **Window & Layout, Panels, Search & Files, Version Control, Debugger, Terminal und AI**. Zehn bereits im direkten Editor-Crosswalk enthaltene Search-/Statusleisten-Pfade sind hier nicht doppelt gelistet. Control-Titel und Pfadkennung stammen aus der gepinnten `page_data.rs`; Defaults stammen aus dem gepinnten `default.json`. `JSON settings editor fallback` bezeichnet `SettingField::unimplemented()` und führt in der Settings UI zu „Edit in settings.json“. `not set` bedeutet: kein Wert im ausgelieferten Default-JSON; ein Schema-Inherit ist eigens vermerkt.

| Labonair-Owner / Crosswalk-Familie | Befund |
|---|---|
| Workspace tabs / previews / layout | `crates/workspace` and the universal tab owner exist; local settings cover tab placement, session restore, and selected layout behavior. Per-pane tab appearance, Zed preview promotion rules, tab caps, navigation-history buttons, and all split-direction defaults are not equivalent settings. |
| Shell titlebar | Labonair has a native titlebar and menu; individual branch/worktree/user/menu visibility fields are not a one-to-one local configuration surface. |
| Statusbar / panel visibility | Local statusbar item placement and panel visibility are configurable in `personalization`; Zed-specific active-file, encoding, EOL, pending-keystroke, diagnostics, search, and debugger indicators have no matching complete field set. |
| Project Explorer / file manager | `panel-explorer` and local file-manager settings cover hidden entries, indentation, sticky ancestors, auto-reveal, single-child folding, and Git decorations. Zed scan inclusion/exclusion precedence, Gitignore hiding, panel-specific badges/sort/dock settings, and preview-open policies are not fully matched. |
| Search, File Finder, and indexing | Local editor search/file finding exists, but its Settings model has no persisted Zed-equivalent defaults for ignored-file strategy, path-finder modal sizing, file scan depth, symlink scanning, inclusion/exclusion rules, or reopen/delete policies. |
| Git / Editor gutter / SCM | Local Editor settings have Boolean Git gutter and word-diff controls and the Git/SCM owners exist. Blame placement/delay, hunk style, diff base, path/sort/group style, and panel-specific setting controls are not matched as a complete set. |
| Terminal / workspace process | Local terminal settings cover shell, fonts, cursor, scrollback, copy, bell, and environment. Dock placement, working-directory policy, panel sizing, toolbar, scrollbar and all key defaults still need field-level runtime checks. |
| Debugger | No local DAP debugger capability was found; Zeds breakpoint persistence, stepping, timeout, log, dock, and panel controls therefore have no direct owner equivalent. |
| AI / Agent / Edit Prediction | Labonair AI/MCP settings exist, but they do not reproduce Zed's agent panel, thread, context, prediction, tool-permission, and per-profile settings model. |

| Zed Settings-Seite | Control-Titel | `page_data`-Pfadkennung | Gepinnter Default | Scope | Labonair-Familie |
|---|---|---|---|---|---|
| Window & Layout | Project Panel Button (Settings control) | `project_panel.button` | true | `USER` | Statusbar / panel visibility |
| Window & Layout | Active Language Button (Settings control) | `status_bar.active_language_button` | true | `USER` | Statusbar / panel visibility |
| Window & Layout | Active Encoding Button (Settings control) | `status_bar.active_encoding_button` | "non_utf8" | `USER` | Statusbar / panel visibility |
| Window & Layout | Cursor Position Button (Settings control) | `status_bar.cursor_position_button` | true | `USER` | Statusbar / panel visibility |
| Window & Layout | Line Endings Button (Settings control) | `status_bar.line_endings_button` | false | `USER` | Statusbar / panel visibility |
| Window & Layout | Pending Keystrokes Indicator (Settings control) | `status_bar.pending_keystrokes_indicator` | true | `USER` | Statusbar / panel visibility |
| Window & Layout | Terminal Button (Settings control) | `terminal.button` | true | `USER` | Statusbar / panel visibility |
| Window & Layout | Diagnostics Button (Settings control) | `diagnostics.button` | true | `USER` | Statusbar / panel visibility |
| Window & Layout | Debugger Button (Settings control) | `debugger.button` | true | `USER` | Statusbar / panel visibility |
| Window & Layout | Active File Name (Settings control) | `status_bar.show_active_file` | false | `USER` | Statusbar / panel visibility |
| Window & Layout | Show Branch Status Icon (Settings control) | `title_bar.show_branch_status_icon` | false | `USER` | Shell titlebar |
| Window & Layout | Show Branch Name (Settings control) | `title_bar.show_branch_name` | true | `USER` | Shell titlebar |
| Window & Layout | Show Worktree Name (Settings control) | `title_bar.show_worktree_name` | true | `USER` | Shell titlebar |
| Window & Layout | Show Project Items (Settings control) | `title_bar.show_project_items` | true | `USER` | Shell titlebar |
| Window & Layout | Show Onboarding Banner (Settings control) | `title_bar.show_onboarding_banner` | true | `USER` | Shell titlebar |
| Window & Layout | Show Sign In (Settings control) | `title_bar.show_sign_in` | true | `USER` | Shell titlebar |
| Window & Layout | Show User Menu (Settings control) | `title_bar.show_user_menu` | true | `USER` | Shell titlebar |
| Window & Layout | Show User Picture (Settings control) | `title_bar.show_user_picture` | true | `USER` | Shell titlebar |
| Window & Layout | Show Menus (Settings control) | `title_bar.show_menus` | false | `USER` | Shell titlebar |
| Window & Layout | Button Layout (Settings control) | `title_bar.button_layout$` | "platform_default" | `USER` | Shell titlebar |
| Window & Layout | Custom Button Layout (Settings control) | `title_bar.button_layout` | "platform_default" | `USER` | Shell titlebar |
| Window & Layout | Show Tab Bar (Settings control) | `tab_bar.show` | true | `USER` | Workspace tabs / previews |
| Window & Layout | Show Git Status In Tabs (Settings control) | `tabs.git_status` | false | `USER` | Workspace tabs / previews |
| Window & Layout | Show File Icons In Tabs (Settings control) | `tabs.file_icons` | false | `USER` | Workspace tabs / previews |
| Window & Layout | Tab Close Position (Settings control) | `tabs.close_position` | "right" | `USER` | Workspace tabs / previews |
| Window & Layout | Maximum Tabs (JSON settings editor fallback) | `max_tabs` | `null` (unlimited) | `USER` | Workspace tabs / previews |
| Window & Layout | Show Navigation History Buttons (Settings control) | `tab_bar.show_nav_history_buttons` | true | `USER` | Workspace tabs / previews |
| Window & Layout | Show Tab Bar Buttons (Settings control) | `tab_bar.show_tab_bar_buttons` | true | `USER` | Workspace tabs / previews |
| Window & Layout | Pinned Tabs Layout (Settings control) | `tab_bar.show_pinned_tabs_in_separate_row` | false | `USER` | Workspace tabs / previews |
| Window & Layout | Activate On Close (Settings control) | `tabs.activate_on_close` | "history" | `USER` | Workspace tabs / previews |
| Window & Layout | Tab Show Diagnostics (Settings control) | `tabs.show_diagnostics` | "off" | `USER` | Workspace tabs / previews |
| Window & Layout | Show Close Button (Settings control) | `tabs.show_close_button` | "hover" | `USER` | Workspace tabs / previews |
| Window & Layout | Preview Tabs Enabled (Settings control) | `preview_tabs.enabled` | true | `USER` | Workspace tabs / previews |
| Window & Layout | Enable Preview From Project Panel (Settings control) | `preview_tabs.enable_preview_from_project_panel` | true | `USER` | Workspace tabs / previews |
| Window & Layout | Enable Preview From File Finder (Settings control) | `preview_tabs.enable_preview_from_file_finder` | false | `USER` | Workspace tabs / previews |
| Window & Layout | Enable Preview From Multibuffer (Settings control) | `preview_tabs.enable_preview_from_multibuffer` | true | `USER` | Workspace tabs / previews |
| Window & Layout | Enable Preview Multibuffer From Code Navigation (Settings control) | `preview_tabs.enable_preview_multibuffer_from_code_navigation` | false | `USER` | Workspace tabs / previews |
| Window & Layout | Enable Preview File From Code Navigation (Settings control) | `preview_tabs.enable_preview_file_from_code_navigation` | true | `USER` | Workspace tabs / previews |
| Window & Layout | Enable Keep Preview On Code Navigation (Settings control) | `preview_tabs.enable_keep_preview_on_code_navigation` | false | `USER` | Workspace tabs / previews |
| Window & Layout | Bottom Dock Layout (Settings control) | `bottom_dock_layout` | "contained" | `USER` | Workspace layout |
| Window & Layout | Centered Layout Left Padding (Settings control) | `centered_layout.left_padding` | 0.2 | `USER` | Workspace layout |
| Window & Layout | Centered Layout Right Padding (Settings control) | `centered_layout.right_padding` | 0.2 | `USER` | Workspace layout |
| Window & Layout | Focus Follows Mouse (Settings control) | `focus_follows_mouse.enabled` | false | `USER` | Workspace layout |
| Window & Layout | Focus Follows Mouse Debounce ms (Settings control) | `focus_follows_mouse.debounce_ms` | 250 | `USER` | Workspace layout |
| Window & Layout | Use System Window Tabs (Settings control) | `use_system_window_tabs` | false | `USER` | Workspace layout |
| Window & Layout | Fullscreen Mode (Settings control) | `fullscreen_mode` | "native" | `USER` | Workspace layout |
| Window & Layout | Window Decorations (Settings control) | `window_decorations` | "client" | `USER` | Workspace layout |
| Window & Layout | Inactive Opacity (Settings control) | `active_pane_modifiers.inactive_opacity` | 1.0 | `USER` | Workspace layout |
| Window & Layout | Border Size (Settings control) | `active_pane_modifiers.border_size` | 0.0 | `USER` | Workspace layout |
| Window & Layout | Zoomed Padding (Settings control) | `zoomed_padding` | true | `USER` | Workspace layout |
| Window & Layout | Close Panel on Toggle (Settings control) | `close_panel_on_toggle` | false | `USER` | Workspace layout |
| Window & Layout | Vertical Split Direction (Settings control) | `pane_split_direction_vertical` | "right" | `USER` | Workspace layout |
| Window & Layout | Horizontal Split Direction (Settings control) | `pane_split_direction_horizontal` | "down" | `USER` | Workspace layout |
| Panels | Project Panel Dock (Settings control) | `project_panel.dock` | "right" | `USER` | Project Explorer / file manager |
| Panels | Project Panel Default Width (Settings control) | `project_panel.default_width` | 240 | `USER` | Project Explorer / file manager |
| Panels | Project Panel Title Tooltips Delay (Settings control) | `project_panel.title_tooltip_delay$` | "default" | `USER` | Project Explorer / file manager |
| Panels | Custom Delay (Settings control) | `project_panel.title_tooltip_delay` | "default" | `USER` | Project Explorer / file manager |
| Panels | Hide .gitignore (Settings control) | `project_panel.hide_gitignore` | false | `USER` | Project Explorer / file manager |
| Panels | Entry Spacing (Settings control) | `project_panel.entry_spacing` | "comfortable" | `USER` | Project Explorer / file manager |
| Panels | File Icons (Settings control) | `project_panel.file_icons` | true | `USER` | Project Explorer / file manager |
| Panels | Folder Indicator (Settings control) | `project_panel.folder_indicator` | "icon" | `USER` | Project Explorer / file manager |
| Panels | Git Status (Settings control) | `project_panel.git_status` | true | `USER` | Project Explorer / file manager |
| Panels | Indent Size (Settings control) | `project_panel.indent_size` | 20 | `USER` | Project Explorer / file manager |
| Panels | Auto Reveal Entries (Settings control) | `project_panel.auto_reveal_entries` | true | `USER` | Project Explorer / file manager |
| Panels | Starts Open (Settings control) | `project_panel.starts_open` | true | `USER` | Project Explorer / file manager |
| Panels | Auto Fold Directories (Settings control) | `project_panel.auto_fold_dirs` | true | `USER` | Project Explorer / file manager |
| Panels | Bold Folder Labels (Settings control) | `project_panel.bold_folder_labels` | false | `USER` | Project Explorer / file manager |
| Panels | Show Scrollbar (Settings control) | `project_panel.scrollbar.show` | `null` (inherits editor scrollbar) | `USER` | Project Explorer / file manager |
| Panels | Horizontal Scroll (Settings control) | `project_panel.scrollbar.horizontal_scroll` | true | `USER` | Project Explorer / file manager |
| Panels | Show Diagnostics (Settings control) | `project_panel.show_diagnostics` | "all" | `USER` | Project Explorer / file manager |
| Panels | Diagnostic Badges (Settings control) | `project_panel.diagnostic_badges` | false | `USER` | Project Explorer / file manager |
| Panels | Git Status Indicator (Settings control) | `project_panel.git_status_indicator` | false | `USER` | Project Explorer / file manager |
| Panels | Sticky Scroll (Settings control) | `project_panel.sticky_scroll` | true | `USER` | Project Explorer / file manager |
| Panels | Show Indent Guides (Settings control) | `project_panel.indent_guides.show` | "always" | `USER` | Project Explorer / file manager |
| Panels | Drag and Drop (Settings control) | `project_panel.drag_and_drop` | true | `USER` | Project Explorer / file manager |
| Panels | Hide Root (Settings control) | `project_panel.hide_root` | false | `USER` | Project Explorer / file manager |
| Panels | Hide Hidden (Settings control) | `project_panel.hide_hidden` | false | `USER` | Project Explorer / file manager |
| Panels | Sort Mode (Settings control) | `project_panel.sort_mode` | "directories_first" | `USER` | Project Explorer / file manager |
| Panels | Sort Order (Settings control) | `project_panel.sort_order` | "default" | `USER` | Project Explorer / file manager |
| Panels | Auto Open Files On Create (Settings control) | `project_panel.auto_open.on_create` | true | `USER` | Project Explorer / file manager |
| Panels | Auto Open Files On Paste (Settings control) | `project_panel.auto_open.on_paste` | true | `USER` | Project Explorer / file manager |
| Panels | Auto Open Files On Drop (Settings control) | `project_panel.auto_open.on_drop` | true | `USER` | Project Explorer / file manager |
| Panels | Hidden Files (JSON settings editor fallback) | `worktree.hidden_files` | ["**/.*"] | `USER` | Project Explorer / file manager |
| Panels | Terminal Dock (Settings control) | `terminal.dock` | "bottom" | `USER` | Terminal |
| Panels | Starts Open (Settings control) | `terminal.starts_open` | false | `USER` | Terminal |
| Panels | Terminal Panel Flexible Sizing (Settings control) | `terminal.flexible` | true | `USER` | Terminal |
| Panels | Show Count Badge (Settings control) | `terminal.show_count_badge` | false | `USER` | Terminal |
| Panels | Outline Panel Button (Settings control) | `outline_panel.button` | true | `USER` | Editor outline panel |
| Panels | Outline Panel Dock (Settings control) | `outline_panel.dock` | "right" | `USER` | Editor outline panel |
| Panels | Outline Panel Default Width (Settings control) | `outline_panel.default_width` | 300 | `USER` | Editor outline panel |
| Panels | File Icons (Settings control) | `outline_panel.file_icons` | true | `USER` | Editor outline panel |
| Panels | Folder Indicator (Settings control) | `outline_panel.folder_indicator` | "icon" | `USER` | Editor outline panel |
| Panels | Git Status (Settings control) | `outline_panel.git_status` | true | `USER` | Editor outline panel |
| Panels | Indent Size (Settings control) | `outline_panel.indent_size` | 20 | `USER` | Editor outline panel |
| Panels | Auto Reveal Entries (Settings control) | `outline_panel.auto_reveal_entries` | true | `USER` | Editor outline panel |
| Panels | Auto Fold Directories (Settings control) | `outline_panel.auto_fold_dirs` | true | `USER` | Editor outline panel |
| Panels | Show Indent Guides (Settings control) | `outline_panel.indent_guides.show` | "always" | `USER` | Editor outline panel |
| Panels | Hide Symbols in Multi-Buffers (Settings control) | `outline_panel.multi_buffer_hide_symbols` | false | `USER` | Editor outline panel |
| Panels | Git Panel Button (Settings control) | `git_panel.button` | true | `USER` | Git panel / SCM |
| Panels | Git Panel Dock (Settings control) | `git_panel.dock` | "right" | `USER` | Git panel / SCM |
| Panels | Starts Open (Settings control) | `git_panel.starts_open` | false | `USER` | Git panel / SCM |
| Panels | Git Panel Default Width (Settings control) | `git_panel.default_width` | 360 | `USER` | Git panel / SCM |
| Panels | Git Panel Status Style (Settings control) | `git_panel.status_style` | "icon" | `USER` | Git panel / SCM |
| Panels | Fallback Branch Name (Settings control) | `git_panel.fallback_branch_name` | "main" | `USER` | Git panel / SCM |
| Panels | Sort By (Settings control) | `git_panel.sort_by` | "path" | `USER` | Git panel / SCM |
| Panels | Group By (Settings control) | `git_panel.group_by` | "status" | `USER` | Git panel / SCM |
| Panels | Collapse Untracked Diff (Settings control) | `git_panel.collapse_untracked_diff` | false | `USER` | Git panel / SCM |
| Panels | Tree View (Settings control) | `git_panel.tree_view` | false | `USER` | Git panel / SCM |
| Panels | File Icons (Settings control) | `git_panel.file_icons` | false | `USER` | Git panel / SCM |
| Panels | Folder Indicator (Settings control) | `git_panel.folder_indicator` | "icon" | `USER` | Git panel / SCM |
| Panels | Diff Stats (Settings control) | `git_panel.diff_stats` | true | `USER` | Git panel / SCM |
| Panels | Primary Click Behavior (Settings control) | `git_panel.entry_primary_click_action` | "project_diff" | `USER` | Git panel / SCM |
| Panels | Show Count Badge (Settings control) | `git_panel.show_count_badge` | false | `USER` | Git panel / SCM |
| Panels | Commit Title Max Length (Settings control) | `git_panel.commit_title_max_length` | 0 | `USER` | Git panel / SCM |
| Panels | Scroll Bar (Settings control) | `git_panel.scrollbar.show` | not set; inherits editor scrollbar configuration | `USER` | Git panel / SCM |
| Panels | Debugger Panel Dock (Settings control) | `debugger.dock` | "bottom" | `USER` | Debugger |
| Panels | Collaboration Panel Button (Settings control) | `collaboration_panel.button` | true | `USER` | Collaboration panel |
| Panels | Collaboration Panel Dock (Settings control) | `collaboration_panel.dock` | "right" | `USER` | Collaboration panel |
| Panels | Collaboration Panel Default Width (Settings control) | `collaboration_panel.dock` | 240 (`default_width`; `page_data` path identifier incorrectly says `collaboration_panel.dock`) | `USER` | Collaboration panel |
| Panels | Agent Panel Button (Settings control) | `agent.button` | true | `USER` | AI / Agent panel |
| Panels | Agent Panel Dock (Settings control) | `agent.dock` | "left" | `USER` | AI / Agent panel |
| Panels | Agent Panel Flexible Sizing (Settings control) | `agent.flexible` | true | `USER` | AI / Agent panel |
| Panels | Agent Panel Default Width (Settings control) | `agent.default_width` | 640 | `USER` | AI / Agent panel |
| Panels | Agent Panel Default Height (Settings control) | `agent.default_height` | 320 | `USER` | AI / Agent panel |
| Panels | Limit Content Width (Settings control) | `agent.limit_content_width` | true | `USER` | AI / Agent panel |
| Panels | Max Content Width (Settings control) | `agent.max_content_width` | 850 | `USER` | AI / Agent panel |
| Search & Files | Include Ignored in Search (Settings control) | `file_finder.include_ignored` | "smart" | `USER` | File Finder / Search |
| Search & Files | File Icons (Settings control) | `file_finder.file_icons` | true | `USER` | File Finder / Search |
| Search & Files | Skip Focus For Active In Search (Settings control) | `file_finder.skip_focus_for_active_in_search` | true | `USER` | File Finder / Search |
| Search & Files | File Scan Exclusions (JSON settings editor fallback) | `file_scan_exclusions` | [11 Werte; siehe default.json] | `USER` | Project indexing / Explorer |
| Search & Files | File Scan Inclusions (JSON settings editor fallback) | `file_scan_inclusions` | [".env*"] | `USER` | Project indexing / Explorer |
| Search & Files | File Scan Depth (Settings control) | `file_scan_depth` | 5 | `USER \| PROJECT` | Project indexing / Explorer |
| Search & Files | Scan Symbolic Links (Settings control) | `scan_symlinks` | "expanded" | `USER` | Project indexing / Explorer |
| Search & Files | Restore File State (Settings control) | `restore_on_file_reopen` | true | `USER` | Workspace file lifecycle |
| Search & Files | Close on File Delete (Settings control) | `close_on_file_delete` | false | `USER` | Workspace file lifecycle |
| Version Control | Disable Git Integration (Settings control) | `git.disable_git` | false | `USER` | Git / Editor gutter / SCM |
| Version Control | Enable Git Status (Settings control) | `git.enable_status` | true | `USER` | Git / Editor gutter / SCM |
| Version Control | Enable Git Diff (Settings control) | `git.enable_diff` | true | `USER` | Git / Editor gutter / SCM |
| Version Control | Visibility (Settings control) | `git.git_gutter` | "tracked_files" | `USER` | Git / Editor gutter / SCM |
| Version Control | Debounce (Settings control) | `git.gutter_debounce` | 0 | `USER` | Git / Editor gutter / SCM |
| Version Control | Enabled (Settings control) | `git.inline_blame.enabled` | true | `USER` | Git / Editor gutter / SCM |
| Version Control | Location (Settings control) | `git.inline_blame.location` | "inline" | `USER` | Git / Editor gutter / SCM |
| Version Control | Delay (Settings control) | `git.inline_blame.delay_ms` | 0 | `USER` | Git / Editor gutter / SCM |
| Version Control | Padding (Settings control) | `git.inline_blame.padding` | 7 | `USER` | Git / Editor gutter / SCM |
| Version Control | Minimum Column (Settings control) | `git.inline_blame.min_column` | 0 | `USER` | Git / Editor gutter / SCM |
| Version Control | Show Commit Summary (Settings control) | `git.inline_blame.show_commit_summary` | false | `USER` | Git / Editor gutter / SCM |
| Version Control | Show Avatar (Settings control) | `git.blame.show_avatar` | true | `USER` | Git / Editor gutter / SCM |
| Version Control | Show Author Name (Settings control) | `git.branch_picker.show_author_name` | true | `USER` | Git / Editor gutter / SCM |
| Version Control | Hunk Style (Settings control) | `git.hunk_style` | "staged_hollow" | `USER` | Git / Editor gutter / SCM |
| Version Control | Diff Base (Settings control) | `git.diff_base` | "head" | `USER` | Git / Editor gutter / SCM |
| Version Control | Path Style (Settings control) | `git.path_style` | "file_name_first" | `USER` | Git / Editor gutter / SCM |
| Version Control | Show Stage/Restore Buttons (Settings control) | `git.show_stage_restore_buttons` | true | `USER` | Git / Editor gutter / SCM |
| Version Control | Show Full File by Default (Settings control) | `git.file_diff.show_full_file` | true | `USER` | Git / Editor gutter / SCM |
| Debugger | Stepping Granularity (Settings control) | `debugger.stepping_granularity` | "line" | `USER` | Debugger |
| Debugger | Save Breakpoints (Settings control) | `debugger.save_breakpoints` | true | `USER` | Debugger |
| Debugger | Timeout (Settings control) | `debugger.timeout` | 2000 | `USER` | Debugger |
| Debugger | Log DAP Communications (Settings control) | `debugger.log_dap_communications` | true | `USER` | Debugger |
| Debugger | Format DAP Log Messages (Settings control) | `debugger.format_dap_log_messages` | true | `USER` | Debugger |
| Terminal | Shell (Settings control) | `terminal.shell$` | "system" | `USER \| PROJECT` | Terminal / workspace process |
| Terminal | Program (Settings control) | `terminal.shell` | "system" | `USER \| PROJECT` | Terminal / workspace process |
| Terminal | Program (Settings control) | `terminal.shell.program` | not set in packaged default.json | `USER \| PROJECT` | Terminal / workspace process |
| Terminal | Arguments (JSON settings editor fallback) | `terminal.shell.args` | not set in packaged default.json | `USER \| PROJECT` | Terminal / workspace process |
| Terminal | Title Override (Settings control) | `terminal.shell.title_override` | not set in packaged default.json | `USER \| PROJECT` | Terminal / workspace process |
| Terminal | Working Directory (Settings control) | `terminal.working_directory$` | "current_project_directory" | `USER \| PROJECT` | Terminal / workspace process |
| Terminal | Directory (Settings control) | `terminal.working_directory.always` | not set in packaged default.json | `USER \| PROJECT` | Terminal / workspace process |
| Terminal | Environment Variables (JSON settings editor fallback) | `terminal.env` | {} | `USER \| PROJECT` | Terminal / workspace process |
| Terminal | Detect Virtual Environment (JSON settings editor fallback) | `terminal.detect_venv` | {1 Schlüssel; siehe default.json} | `USER \| PROJECT` | Terminal / workspace process |
| Terminal | Font Size (Settings control) | `terminal.font_size` | not set; inherits buffer font configuration | `USER` | Terminal / workspace process |
| Terminal | Font Family (Settings control) | `terminal.font_family` | not set; inherits buffer font configuration | `USER` | Terminal / workspace process |
| Terminal | Font Fallbacks (JSON settings editor fallback) | `terminal.font_fallbacks` | not set; inherits buffer font configuration | `USER` | Terminal / workspace process |
| Terminal | Font Weight (Settings control) | `terminal.font_weight` | 400 | `USER` | Terminal / workspace process |
| Terminal | Font Features (JSON settings editor fallback) | `terminal.font_features` | not set in packaged default.json | `USER` | Terminal / workspace process |
| Terminal | Line Height (JSON settings editor fallback) | `terminal.line_height` | "standard" | `USER` | Terminal / workspace process |
| Terminal | Cursor Shape (Settings control) | `terminal.cursor_shape` | "block" | `USER` | Terminal / workspace process |
| Terminal | Cursor Blinking (Settings control) | `terminal.blinking` | "terminal_controlled" | `USER` | Terminal / workspace process |
| Terminal | Alternate Scroll (Settings control) | `terminal.alternate_scroll` | "on" | `USER` | Terminal / workspace process |
| Terminal | Minimum Contrast (Settings control) | `terminal.minimum_contrast` | 45 | `USER` | Terminal / workspace process |
| Terminal | Option As Meta (Settings control) | `terminal.option_as_meta` | false | `USER` | Terminal / workspace process |
| Terminal | Copy On Select (Settings control) | `terminal.copy_on_select` | false | `USER` | Terminal / workspace process |
| Terminal | Keep Selection On Copy (Settings control) | `terminal.keep_selection_on_copy` | true | `USER` | Terminal / workspace process |
| Terminal | Open Links In Mouse Mode (Settings control) | `terminal.open_links_in_mouse_mode` | true | `USER` | Terminal / workspace process |
| Terminal | Audible Bell (Settings control) | `terminal.bell` | "off" | `USER` | Terminal / workspace process |
| Terminal | Default Width (Settings control) | `terminal.default_width` | 640 | `USER` | Terminal / workspace process |
| Terminal | Default Height (Settings control) | `terminal.default_height` | 320 | `USER` | Terminal / workspace process |
| Terminal | Max Scroll History Lines (Settings control) | `terminal.max_scroll_history_lines` | 10000 | `USER` | Terminal / workspace process |
| Terminal | Scroll Multiplier (Settings control) | `terminal.scroll_multiplier` | 1.0 | `USER` | Terminal / workspace process |
| Terminal | Breadcrumbs (Settings control) | `terminal.toolbar.breadcrumbs` | false | `USER` | Terminal / workspace process |
| Terminal | Show Scrollbar (Settings control) | `terminal.scrollbar.show` | `null` (inherits editor scrollbar) | `USER` | Terminal / workspace process |
| AI | Disable AI (Settings control) | `disable_ai` | false | `USER \| PROJECT` | AI / Agent / Edit Prediction |
| AI | Threads Sidebar Side (Settings control) | `agent.sidebar_side` | "left" | `USER` | AI / Agent / Edit Prediction |
| AI | Single File Review (Settings control) | `agent.single_file_review` | false | `USER` | AI / Agent / Edit Prediction |
| AI | Enable Feedback (Settings control) | `agent.enable_feedback` | true | `USER` | AI / Agent / Edit Prediction |
| AI | Notify When Agent Waiting (Settings control) | `agent.notify_when_agent_waiting` | "primary_screen" | `USER` | AI / Agent / Edit Prediction |
| AI | Play Sound When Agent Done (Settings control) | `agent.play_sound_when_agent_done` | "never" | `USER` | AI / Agent / Edit Prediction |
| AI | Expand Edit Card (Settings control) | `agent.expand_edit_card` | true | `USER` | AI / Agent / Edit Prediction |
| AI | Expand Terminal Card (Settings control) | `agent.expand_terminal_card` | true | `USER` | AI / Agent / Edit Prediction |
| AI | Terminal Thread Init Command (Settings control) | `agent.terminal_init_command` | "" | `USER` | AI / Agent / Edit Prediction |
| AI | Thinking Display (Settings control) | `agent.thinking_display` | "auto" | `USER` | AI / Agent / Edit Prediction |
| AI | Cancel Generation On Terminal Stop (Settings control) | `agent.cancel_generation_on_terminal_stop` | true | `USER` | AI / Agent / Edit Prediction |
| AI | Use Modifier To Send (Settings control) | `agent.use_modifier_to_send` | false | `USER` | AI / Agent / Edit Prediction |
| AI | Message Editor Min Lines (Settings control) | `agent.message_editor_min_lines` | 4 | `USER` | AI / Agent / Edit Prediction |
| AI | Show Turn Stats (Settings control) | `agent.show_turn_stats` | false | `USER` | AI / Agent / Edit Prediction |
| AI | Show Merge Conflict Indicator (Settings control) | `agent.show_merge_conflict_indicator` | true | `USER` | AI / Agent / Edit Prediction |
| AI | Auto Compact (Settings control) | `agent.auto_compact.enabled` | true | `USER` | AI / Agent / Edit Prediction |
| AI | Auto Compact Threshold (Settings control) | `agent.auto_compact.threshold` | "90%" | `USER` | AI / Agent / Edit Prediction |
| AI | Display Mode (Settings control) | `edit_prediction.display_mode` | not set in packaged default.json | `USER` | AI / Agent / Edit Prediction |

**Pfadabweichung in der gepinnten Settings UI:** „Collaboration Panel Default Width“ wird von `pick` und `write` auf `collaboration_panel.default_width` gelegt, aber sein `page_data`-Pfadbezeichner lautet fälschlich `collaboration_panel.dock`, derselbe wie beim Dock-Auswahlfeld. Der ausgelieferte Default für `default_width` ist 240 px. Diese Tabelle erhält den Pfadbezeichner aus der UI-Metadatei und den Wert aus dem tatsächlich gelesenen Feld.

### Weitere 72 Controls aus den übrigen statischen Settings-Seiten

Die gepinnte Settings-Navigation hat neben Editor, Languages & Tools, Search & Files, Window & Layout, Panels, Debugger, Terminal, Version Control und AI auch die Seiten **General, Appearance, Keymap, Collaboration, Network und Developer**. Diese Seiten deklarieren zusammen 73 Controls; neun Pfade sind bereits im direkten Editor-Settings-Crosswalk enthalten und hier nicht wiederholt. Die Seite **Languages & Tools** deklariert zehn weitere Controls; zwei sind bereits dort crosswalked, die übrigen acht stehen hier. Die folgenden 72 Zeilen ergänzen somit die bisherigen Tabellen um alle noch nicht einzeln gelisteten Pfade dieser sieben Seiten. Zusammen mit den vorangehenden Tabellen sind jetzt die `SettingItem`-Pfade aller 15 statischen Navigationsseiten indexiert. Dynamische Setup- und Unterseiten sowie Suche, Navigation, Editieren, Zurücksetzen und Laufzeitwirkung der Settings UI müssen noch gesondert untersucht werden. Titel und `page_data`-Pfad stammen aus dem gepinnten `page_data.rs`, Defaults aus `default.json`, soweit dort direkt festgelegt. `not active under … default` bezeichnet einen bedingten Variantenwert, nicht ein fehlendes Settingsfeld; `not set in packaged default.json` bezeichnet einen ausgelassenen JSON-Wert. `JSON settings editor fallback`-Felder werden in der UI nicht als typisiertes Einzelcontrol gerendert.

| Zed Settings-Seite | Control-Titel | `page_data`-Pfadkennung | Gepinnter Default | Scope | Labonair-Familie / Crosswalk |
|---|---|---|---|---|---|
| General | Accessible Mode | `accessible_mode` | false | `USER` | Accessibility: kein gleichwertiger globaler Assistive-Mode-Schalter im lokalen Settingsmodell gefunden. |
| General | When Closing With No Tabs | `when_closing_with_no_tabs` | "platform_default" | `USER` | Shell / Workspace-Lifecycle |
| General | On New Window | `on_new_window` | "launchpad" | `USER` | Workspace- und Session-Lifecycle |
| General | On Last Window Closed | `on_last_window_closed` | "platform_default" | `USER` | Workspace- und Session-Lifecycle |
| General | Use System Path Prompts | `use_system_path_prompts` | true | `USER` | Native Dateiöffnungs- und Speicherdialoge |
| General | Use System Prompts | `use_system_prompts` | true | `USER` | Native Bestätigungsdialoge |
| General | Private Files | `worktree.private_files` | not set in packaged `default.json` | `USER` | Private-file handling; kein entsprechender Owner oder Dateimuster-Settingspfad belegt. |
| General | CLI Default Open Behavior | `cli_default_open_behavior` | "existing_window" | `USER` | CLI / Workspace-Lifecycle |
| General | Reveal If Open | `reveal_if_open` | false | `USER` | Workspace-Tabaktivierung |
| General | Default Open Behavior | `default_open_behavior` | "existing_window" | `USER` | Workspace-Dateiöffnung |
| General | Trust All Projects By Default | `session.trust_all_projects` | not set in packaged `default.json` | `USER` | Projektvertrauen und Security: keine gleichwertige persistente globale Vertrauensoption belegt. |
| General | Restore Unsaved Buffers | `session.restore_unsaved_buffers` | true | `USER` | Workspace-Session-Restore |
| General | Restore On Startup | `restore_on_startup` | "last_session" | `USER` | Workspace-Session-Restore |
| General | Preview Channel | `preview_channel_settings` | not set in packaged `default.json` | `USER` | Release-Kanal / Application Updates |
| General | Settings Profiles | `settings_profiles` | not set in packaged `default.json` | `USER` | Settings-Profile: kein profilbasierter Settings-Owner belegt. |
| General | Telemetry Diagnostics | `telemetry.diagnostics` | true | `USER` | Telemetrie-Datenschutz |
| General | Telemetry Metrics | `telemetry.metrics` | true | `USER` | Telemetrie-Datenschutz |
| General | Anthropic Data Retention | `telemetry.anthropic_retention` | false | `USER` | AI-Datenschutz und Aufbewahrung |
| General | Auto Update | `auto_update` | true | `USER` | Application Updates |
| Appearance | Theme Mode | `theme$` | dynamic (mode=system, light=One Light, dark=One Dark) | `USER` | Theme-System und Appearance-Owner |
| Appearance | Theme Name | `theme` | not active under the dynamic theme default | `USER` | Bedingter statischer Theme-Name; Theme-System |
| Appearance | Mode | `theme.mode` | "system" | `USER` | Theme-System |
| Appearance | Light Theme | `theme.light` | "One Light" | `USER` | Theme-System |
| Appearance | Dark Theme | `theme.dark` | "One Dark" | `USER` | Theme-System |
| Appearance | Icon Theme | `icon_theme$` | static (Zed (Default)) | `USER` | Icon-Theme-System und Dateisymbole |
| Appearance | Icon Theme Name | `icon_theme$string` | Zed (Default), derived from `icon_theme` | `USER` | Bedingter statischer Icon-Theme-Name |
| Appearance | Mode | `icon_theme` | not active under the static icon-theme default | `USER` | Icon-Theme-System |
| Appearance | Light Icon Theme | `icon_theme.light` | not active under the static icon-theme default | `USER` | Icon-Theme-System |
| Appearance | Dark Icon Theme | `icon_theme.dark` | not active under the static icon-theme default | `USER` | Icon-Theme-System |
| Appearance | Font Family | `buffer_font_family` | ".ZedMono" | `USER` | Editor-Typografie |
| Appearance | Font Size | `buffer_font_size` | 15 | `USER` | Editor-Typografie |
| Appearance | Font Weight | `buffer_font_weight` | 400 | `USER` | Editor-Typografie |
| Appearance | Line Height | `buffer_line_height$` | comfortable, derived from `buffer_line_height` | `USER` | Editor-Typografie |
| Appearance | Custom Line Height | `buffer_line_height` | "comfortable" | `USER` | Editor-Typografie |
| Appearance | Font Features | `buffer_font_features` | {} | `USER` | Editor-Typografie |
| Appearance | Font Fallbacks | `buffer_font_fallbacks` | null | `USER` | Editor-Typografie; `null` überlässt Fallbacks der Font-Auflösung. |
| Appearance | Font Family | `ui_font_family` | ".ZedSans" | `USER` | UI-Kit-Typografie |
| Appearance | Font Size | `ui_font_size` | 16 | `USER` | UI-Kit-Typografie |
| Appearance | Font Weight | `ui_font_weight` | 400 | `USER` | UI-Kit-Typografie |
| Appearance | Font Features | `ui_font_features` | {"calt":false} | `USER` | UI-Kit-Typografie |
| Appearance | Font Fallbacks | `ui_font_fallbacks` | null | `USER` | UI-Kit-Typografie; `null` überlässt Fallbacks der Font-Auflösung. |
| Appearance | UI Font Family | `agent_ui_font_family` | null | `USER` | AI / Agent-Typografie; `null` erbt den globalen UI-Font. |
| Appearance | UI Font Size | `agent_ui_font_size` | null | `USER` | AI / Agent-Typografie; `null` erbt die globale UI-Fontgröße. |
| Appearance | Buffer Font Family | `agent_buffer_font_family` | null | `USER` | AI / Agent-Typografie; `null` erbt den globalen Buffer-Font. |
| Appearance | Buffer Font Size | `agent_buffer_font_size` | 12 | `USER` | AI / Agent-Typografie |
| Appearance | Font Family | `markdown_preview.font_family` | null | `USER` | Markdown-Preview-Typografie; `null` nutzt den App-Font. |
| Appearance | Code Font Family | `markdown_preview.code_font_family` | null | `USER` | Markdown-Preview-Code-Typografie; `null` nutzt den App-Font. |
| Appearance | Font Size | `markdown_preview.font_size` | null | `USER` | Markdown-Preview-Typografie; `null` nutzt die Preview-Standardgröße. |
| Appearance | Text Rendering Mode | `text_rendering_mode` | "platform_default" | `USER` | Text-Rendering |
| Appearance | Hide Mouse | `hide_mouse` | "on_typing_and_action" | `USER` | Editor-Pointerverhalten |
| Appearance | Reduce Motion | `reduce_motion` | "off" | `USER` | Accessibility / Animation |
| Appearance | Unnecessary Code Fade | `unnecessary_code_fade` | 0.3 | `USER` | Editor-Syntaxdarstellung |
| Appearance | Show Wrap Guides | `show_wrap_guides` | true | `USER \| PROJECT` | Editor-Layout und Zeilenlängenhinweise |
| Appearance | Wrap Guides | `wrap_guides` | [] | `USER \| PROJECT` | Editor-Layout; im JSON-Editor können benutzerdefinierte Spalten konfiguriert werden. |
| Keymap | Base Keymap | `base_keymap` | "Zed" | `USER` | Keymap-Profile: lokaler Owner vorhanden; Zed-Profilwechsel und Konfliktverhalten nicht gleichwertig belegt. |
| Keymap | Vim Mode | `vim_mode` | false | `USER` | Modal Editing / Vim |
| Keymap | Helix Mode | `helix_mode` | false | `USER` | Modal Editing / Helix: kein lokaler Helix-Modus gefunden. |
| Languages & Tools | File Type Associations (JSON settings editor fallback) | `file_type_associations` | not set in packaged `default.json` | `USER \| PROJECT` | Language registry: lokale Zuordnung ist statisch, keine editierbare User-/Project-Dateitypzuordnung belegt. |
| Languages & Tools | Include Warnings | `diagnostics.include_warnings` | true | `USER` | Diagnostics: globales Ein-/Ausschalten vorhanden; separater Warnungsfilter fehlt. |
| Languages & Tools | Enabled | `diagnostics.inline.enabled` | false | `USER` | Inline-Diagnostics: Dekorationen sind vorhanden, diese Anzeigeoption ist nicht konfigurierbar belegt. |
| Languages & Tools | Update Debounce | `diagnostics.inline.update_debounce_ms` | 150 | `USER` | Inline-Diagnostics: kein entsprechendes Verzögerungsfeld gefunden. |
| Languages & Tools | Padding | `diagnostics.inline.padding` | 4 | `USER` | Inline-Diagnostics: kein entsprechendes Abstandsfeld gefunden. |
| Languages & Tools | Minimum Column | `diagnostics.inline.min_column` | 0 | `USER` | Inline-Diagnostics: kein entsprechender Spaltenwert gefunden. |
| Languages & Tools | Enabled | `diagnostics.lsp_pull_diagnostics.enabled` | true | `USER` | LSP Diagnostics: lokaler Processpfad verarbeitet `publishDiagnostics`; Pull-Diagnostics nicht belegt. |
| Languages & Tools | Debounce | `diagnostics.lsp_pull_diagnostics.debounce_ms` | 50 | `USER` | LSP Diagnostics: kein Pull-Request-Debounce im lokalen Processpfad gefunden. |
| Collaboration | Mute On Join | `calls.mute_on_join` | false | `USER` | Calls und Zusammenarbeit: kein entsprechender Call-Owner belegt. |
| Collaboration | Share On Join | `calls.share_on_join` | false | `USER` | Calls und Zusammenarbeit: kein entsprechender Screen-/Workspace-Share-Workflow belegt. |
| Collaboration | Output Audio Device | `audio.experimental.output_audio_device` | null (system output device) | `USER` | Calls / Audioausgabe: kein entsprechender Call-Audio-Owner belegt. |
| Collaboration | Input Audio Device | `audio.experimental.input_audio_device` | null (system input device) | `USER` | Calls / Audioeingabe: kein entsprechender Call-Audio-Owner belegt. |
| Network | Proxy | `proxy` | "" | `USER` | Netzwerk und hosted services: kein entsprechendes globales Proxy-Setting belegt. |
| Network | Server URL | `server_url` | "https://zed.dev" | `USER` | Netzwerk und hosted services: lokale AI-/MCP-Servereinstellungen sind getrennte Endpunkte. |
| Developer | Performance Profiler | `instrumentation.performance_profiler.enabled` | false | `USER` | Developer Diagnostics: kein entsprechender Profiler-Control-/Trace-Workflow in der Settings UI belegt. |

### Dynamische Settings-Unterseiten und Setup-Flows

Diese Einträge werden aus Settingsseiten oder Aktionen geöffnet und sind keine zusätzlichen Zeilen der statischen Navigationsseiten. Die Tabelle verzeichnet Einstieg, Nutzerworkflow und den nächsten lokalen Owner. Provider- und tool-spezifische Formularfelder, Fehler-/OAuth-Zustände und vollständige Laufzeitinteraktionen sind noch nicht feldweise abgenommen.

| Zed-Unterseite / Einstieg | Nutzerworkflow | Labonair-Gegenstück / Status |
|---|---|---|
| [Languages & Tools → languages.{language}](zed-refrence/zed/crates/settings_ui/src/page_data.rs) | Sprache öffnen und Editor-, Language-Service-, Task-, Formatter- sowie Edit-Prediction-Overrides verwalten. Die 43 LanguageSettingsContent-Felder und 44 gepackten Sprachprofile stehen oben. | Labonair hat statische `LanguageId`- und Service-Registries; dynamische Extension-Discovery und per-Sprache User-/Project-Settings sind nicht belegt. |
| [AI → LLM Providers](zed-refrence/zed/crates/settings_ui/src/pages/llm_providers_page.rs) | Integrierte Modellprovider anzeigen und hinzufügen/konfigurieren; providerabhängige Verbindungs- und Modellwerte pflegen. | Labonair hat AI-Provideradapter und einen Secret Store. Providerkatalog, Einrichtungsfelder und ihre Speicherung sind nicht als gleichwertiger End-to-End-Settings-Flow abgeglichen. |
| [AI → External Agents](zed-refrence/zed/crates/settings_ui/src/pages/external_agents_page.rs) | Externe Agenten über das Agent Client Protocol anzeigen, hinzufügen und entfernen. | Labonair hat interne AI-/Subagent-Tools; eine ACP-Client-Registry mit Agent-Setup und Prozesslebenszyklus ist nicht belegt. |
| [AI → MCP Servers](zed-refrence/zed/crates/settings_ui/src/pages/mcp_servers_page.rs) | Ausgehende Context-Server direkt oder über Extensions verwalten, hinzufügen und konfigurieren; leere Liste und kein aktives Projekt sind eigene Zustände. Die Seite bietet `context_server_timeout=60` Sekunden im User-/Project-Scope. | Labonairs MCP-Server/Agent-Bridge nimmt Anfragen externer Clients an. `max_command_timeout_secs=300` begrenzt einen Agent-Run-Befehl und ist nicht Zeds Timeout für Aufrufe ausgehender Context-Server. Konfigurationsliste und Laufzeit sind verschieden gerichtet. |
| [AI → Skills / Skill Creator](zed-refrence/zed/crates/settings_ui/src/pages/skills_setup.rs) · [Creator](zed-refrence/zed/crates/settings_ui/src/pages/skill_creator.rs) | Agent-Skills global oder je Project-Worktree anzeigen, hinzufügen, bearbeiten und entfernen. | Kein eigener Skill-Katalog, Installations-/Update-Lifecycle oder Project-Worktree-Loader im lokalen Capability-Bestand gefunden. |
| [AI → Sandbox](zed-refrence/zed/crates/settings_ui/src/pages/sandbox_settings.rs) | Erhöhte Terminal-Sandboxrechte prüfen und als dauerhaft promptfrei zulässige Dateisystem-/Netzwerkzugriffe festlegen. | `crates/ai/src/tools/security.rs` blockiert riskante Pfade und ist ausdrücklich eine Schutzschicht, keine Sandbox. Ein OS-Sandbox- und Elevated-Permission-UI ist nicht belegt. |
| [AI → Tool Permissions](zed-refrence/zed/crates/settings_ui/src/pages/tool_permissions_setup.rs) | Regexregeln je Tool-Eingabe automatisch erlauben, ablehnen oder immer bestätigen lassen; konkrete Toolkonfiguration separat öffnen. | Labonair besitzt Bestätigungen für AI-Änderungen/Aktionen und einen Host-Schalter zum Blockieren des Agent-Zugriffs. Ein Regelmodell nach Tool und Eingabe fehlt als gleichwertiger Crosswalk. |
| [Collaboration → Test Audio](zed-refrence/zed/crates/settings_ui/src/pages/audio_input_output_setup.rs) · [Audio Test Window](zed-refrence/zed/crates/settings_ui/src/pages/audio_test_window.rs) | Ein-/Ausgabegeräte wählen und Mikrofon/Lautsprecher testen; die Geräteeinstellungen selbst sind bereits in der Settings-Tabelle aufgeführt. | Kein Calls-/Audio-Owner oder Mikrofon-/Lautsprechertest in Labonair gefunden. |
| [Languages & Tools → Edit Prediction Setup](zed-refrence/zed/crates/settings_ui/src/pages/edit_prediction_provider_setup.rs) | Providerabhängige Edit-Prediction-Modelle und Werte wie API-URL, Modell, Promptformat, Tokenlimit und Debounce konfigurieren; Sprache aktiviert oder deaktiviert Predictions zusätzlich. | Completion und LSP sind lokale Editorfunktionen; Ghost-Text-/Edit-Prediction-Provider und dieser Einrichtungsworkflow sind nicht belegt. |
| [Developer → Feature Flags](zed-refrence/zed/crates/settings_ui/src/pages/feature_flags.rs) | Bedingte, staff-only Overrides zur internen Untersuchung aktivieren; der Einstieg erscheint nur bei freigeschalteten Overrides. | Kein äquivalenter staff-only Feature-Flag-Settings-Flow im lokalen Produkt gefunden. Interne Zed-Flags sind als Referenzverhalten erfasst, aber nicht automatisch als öffentliche Produktanforderung zu behandeln. |

Der MCP-Timeout-Default stammt aus dem Settings-Control `context_server_timeout`; er ist zusätzlich zu den aufgelisteten `SettingItem`-Pfaden der statischen Navigation dokumentiert. Interaktive Abläufe dieser Unterseiten bleiben Teil der offenen Laufzeitabnahme.

## 10. Verbundene Editor-Systeme und deren Nutzen

Diese Systeme liegen teilweise außerhalb des eigentlichen Textwidgets. Sie gehören in den Paritätsumfang, weil Editoraktionen sie auslösen oder ihr Ergebnis inline anzeigen. Die gepinnte Default-macOS-Keymap ist mit allen 585 gebundenen Actions inventarisiert; die systemübergreifenden Laufzeit-Routen und semantischen Entsprechungen sind noch nicht für jede Action einzeln bestätigt.

| Zed verbundenes System | Nutzen für den Editor-Workflow | Labonair-Äquivalent und momentane Einordnung |
|---|---|---|
| Project Panel und Preview Tabs | Dateien suchen, auswählen, als flüchtige Preview öffnen und automatisch zur aktiven Datei springen. | `labonair-panel-explorer`, Filesystem/Workspace und Preview Tabs. Teilweise; Zed-Promote-/Auto-Reveal-/Keyboardzustände offen. |
| Project Search und Multi-Buffer | Mehrere Dateien durchsuchen, Treffer inline bearbeiten und gesammelt speichern. | `crates/editor/src/search.rs`, `crates/workspace/src/project_search.rs`/`search_overlay.rs`. Projektquery teilweise; gemeinsamer Multi-Buffer fehlt. |
| Language Extensions, Grammars, LSP-Installer | Sprachen und Werkzeuge ohne App-Neubau entdecken, installieren und starten. | Kein Extensions Owner im aktuellen Featurebestand gefunden; LSP Process API ist nur Teilgrenze. |
| Diagnostics-/Outline-Panels | Projektweite Ergebnisse sortieren/navigieren und aktiven Symbolbereich synchron halten. | Labonair kann Diagnosen im aktiven Dokument zählen und zyklisch navigieren; Projektansicht, Diagnose-Excerpts und Multi-Buffer-Ersatz sind nicht belegt. Outline ist separat teilweise vorhanden. |
| Git Panel, Git Graph, File History, Stash/Branch Diff | Status, Änderungen und Herkunft direkt aus Editoransicht prüfen. | `labonair-panel-scm`, `labonair-panel-git-graph`, Git und Project Diff existieren; die verbundenen Ablaufdetails bleiben separat abzugleichen. |
| Formatter, Linter, Tasks und Terminal | Code formatieren, prüfen, bauen und starten, ohne Kontextwechsel. | Local terminal und Terminal Tab sind vorhanden; strukturierte Tasks-/Formatter-/Linter-Suche ist nicht äquivalent belegt. |
| Debug Adapter Protocol und Debugger | Breakpoints, Step, Variablen und Stack im Editor-/Projektkontext untersuchen. | Im aktuellen Capability Crosswalk als fehlend/planned Debugger geführt. |
| REPL und Jupyter Kernel | Ausgewählten Code oder Zellen in interaktiver Sitzung ausführen. | Kein REPL-Owner bzw. Kernelworkflow in Labonair gefunden. |
| AI completions / Agent Editing / Agent Review | Inline-Vorschläge, große Änderungen, Review und Toolausführung im Editorworkflow. | Labonair AI/MCP Flächen existieren; Edit Prediction-, Editorreview- und Zed-Agent Tool-Integration ist eigenständige Lücke/Teilparitätsfrage. |
| Keymap Context, Which-Key, Command Palette | Aktionen finden und für aktive Editor-/Popup-/Vim-Kontexte sehen. | Labonair Keymap und Palette vorhanden; exakte Kontext-/Editoraktionenabdeckung offen. |

## 11. Aktuelle Hauptlücken und nächste Recherchearbeit

Der vorliegende Bericht ist ein breit angelegtes Quellcode-/Dokumentationsinventar, noch keine vollständige Action-, Settings- oder Laufzeit-Paritätsabnahme. Aus dem aktuellen Worktree sind besonders diese Unterschiede für den Editorumfang sichtbar:

1. **Multi-Buffer fehlt als belastbarer gemeinsamer Editor-View.** Zed nutzt ihn für Projektsuche, Diagnose, Referenz, Diff und Mehrdatei-Refactor. Labonair besitzt heute Suchergebnisse und Git Diff Views, aber nicht den gemeinsamen Excerpt-/Edit-/Save-Vertrag.
2. **Sprachextensions und automatisch verwaltete Language Server fehlen.** Labonair unterstützt ein begrenztes statisches Sprachset. Ein Local Process/LSP Runtime ist da, wird aber explizit injiziert und ist standardmäßig deaktiviert.
3. **Tab- und Toolbar-Parität ist trotz tieferem Quellinventar weiter offen.** Der aktuelle Worktree setzt Tabstrip-Chrome auf 32 px, inhaltsbasierte Breiten, Trenner, Hover-Close und Editor-Tabs ohne Glyph. Die Pfeile schalten Labonair-Tabs statt Zeds Pane-Navigationshistorie; die Tab-Menüs haben andere Aktionen und scopen Batch-Schließen nach Space. Für die Toolbar sind Zeds Beiträge und ihre Bedingungen nun einzeln dokumentiert; Labonair bietet einzelne Editor- und LSP-Aktionen, aber keinen gleichwertigen per-Pane Quick-Action-, Selections-, Editor-Controls- oder Agent-Review-Vertrag. Code Actions sind als Ergebnis-UI teilweise vorhanden. Encoding/EOL-Controls und Keystroke-Indikatoren bleiben Lücken. Gepaarte native Aufnahmen und Laufzeitinteraktionen fehlen; der Labonair-LSP-Prozessstatus erscheint als Editor-Banner statt Statusleistenmenü.
4. **Labonair verwendet verschiedene Split-Ebenen mit klar begrenztem Umfang.** Der Workspace-Pane-Baum gilt aktuell nur für `TabKind::Workspace`-Terminaltabs; die registrierten Split-Befehle sind an Terminal-Kontext gebunden und erzeugen ein weiteres Terminal im selben Arbeitsverzeichnis. `EditorSplitTree` ist editor-intern und zeigt nicht fokussierte Gruppen als schreibgeschützte Spiegel desselben Dokument-Snapshots. Ein gemeinsamer, unabhängige Editor-/Terminal-/Diff-Views aufnehmender Workspace-Pane-Baum ist nicht belegt.
5. **Diagnosen haben in Labonair einen Dokumentpfad, aber keine Projektansicht.** Der aktuelle Worktree nimmt LSP-Push-Diagnosen an, verwirft Antworten für alte Revisionen und rendert Unterstreichungen; der eingebaute Syntaxprovider deckt Klammerpaare und JSON-Zeichenketten ab. Es gibt Palette-Commands zum Zählen und Navigieren im aktiven Dokument. Zeds severity-getönte Erklärungen/Hover, optionale Error-Lens-Zeilen, Scrollbar-/Tab-/Explorer-Indikatoren, Statusleisten-Zähler und Multi-Buffer-Projektansicht fehlen. Der lokale Diagnose-Schalter ist binär; per-Severity- und Warnungsfilter sind nicht belegt. Inlay Hints, Code Lens, Document Colors, Call Hierarchy, Typdefinition, Workspace Symbols und LSP Folding fehlen oder sind bislang nicht nachgewiesen.
6. **Vorschauen sind nicht vollständig integriert.** Labonair hat einen allgemeinen Preview-Tab mit einfacher Markdown- und Rasterbilddarstellung. Buffergebundene bzw. folgende Markdown-/SVG-Previews, SVG- und Tabellendaten-Renderer, Editor-Augenaktion, Preview neben der Quell-Pane, Bild-Zoom/Metadaten und Emmet fehlen oder sind offen. HTML, SVG, PDF und nicht unterstützte Inhalte gehen an externe Anwendungen; der Preview-Tab zeigt die Zustände ohne Ladeanzeige.
7. **Befehle und Keybindings sind noch keine Paritätsabnahme.** Alle 585 in der gepinnten Default-macOS-Keymap gebundenen Actionnamen, Bindungen und Kontexte sind inventarisiert. Für viele fehlen noch der genaue Labonair-Laufzeitpfad und ein semantischer Vergleich; ungebundene Zed-Registry-Actions, Nutzerprofile und Windows-/Linux-Keymaps sind nicht Teil dieser Zählung.
8. **Visuelle und Laufzeitgleichheit ist unbewiesen.** Für den Vergleich fehlen hier gepaarte Aufnahmen des gepinnten Zed-Builds und des aktuellen Labonair-Builds mit dokumentiertem Fenster, Theme, Skalierung, Fokus, leerem Inhalt, Fehlerfall und Splitzustand.

Für die nächste Erweiterung dieses Berichts sind die noch offenen Arbeitspakete: (a) die lokalen Laufzeit-Routen und semantische Gleichheit für alle 585 gebundenen Actions einzeln klassifizieren; ungebundene Registry-Actions und Plattform-Keymaps zusätzlich inventarisieren; (b) für die erfassten Editor- und verbundenen Settings die exakte lokale Feldzuordnung und gespeicherte/effektive Werte vertiefen und dynamische Provider-, Agent-, MCP-, Skill-, Tool-Permission-, Sandbox-, Audio-, Edit-Prediction- und Feature-Flag-Setups sowie Suche, Navigation, Bearbeiten und Zurücksetzen der Settings UI zur Laufzeit untersuchen; (c) Zed Titelzeile, Pane-Tabs, Toolbar, Statusbar, Empty/Loading/Error und Preview-/Split-Interaktionen an der Referenzlaufzeit beobachten; (d) offene Zed-doc-vs-pin-Abweichungen einzeln markieren; (e) daraus die unabhängige, owner-/workflowbasierte Requirements-Matrix für spätere Implementierung formulieren.

## Quellen und untersuchte Labonair-Flächen

### Zed-Referenzen

- Gepinnter, lokal geprüfter Quellstand: [`zed-refrence/zed`](zed-refrence/zed), Commit `3569541038dd51524b03998ba4d38d253cb54f80` (Forschungsquelle; nicht ändern).
- Gepinnte Editor-Settingsseite und Controls: [`crates/settings_ui/src/page_data.rs`](zed-refrence/zed/crates/settings_ui/src/page_data.rs), Einträge/Bezeichnungen; [`crates/settings_ui/src/settings_ui.rs`](zed-refrence/zed/crates/settings_ui/src/settings_ui.rs), Typ-zu-Renderer-Registrierung und JSON-Editor-Fallback.
- Gepinnte Settings-Schemas und Language-Merge-Regeln: [`crates/settings_content/src/editor.rs`](zed-refrence/zed/crates/settings_content/src/editor.rs), [`crates/settings_content/src/language.rs`](zed-refrence/zed/crates/settings_content/src/language.rs), einschließlich `AllLanguageSettingsContent`, Sprach-Map und `LanguageSettingsContent`.
- Gepinnte Standardwerte: [`assets/settings/default.json`](zed-refrence/zed/assets/settings/default.json), einschließlich bedingter/optional gesetzter Felder.
- Gepinnte macOS-Keymap: [`assets/keymaps/default-macos.json`](zed-refrence/zed/assets/keymaps/default-macos.json); registrierte Base-Keymap-Profile: [`crates/settings/src/base_keymap_setting.rs`](zed-refrence/zed/crates/settings/src/base_keymap_setting.rs).
- Gepinnte Editor-Action-Typen: [`crates/editor/src/actions.rs`](zed-refrence/zed/crates/editor/src/actions.rs), mit ergänzenden Markdown-/Code-Action-Dateien im selben Zed-Editor-Crate.
- Gepinnter Buffer-/Datei-Lifecycle: [`crates/language/src/buffer.rs`](zed-refrence/zed/crates/language/src/buffer.rs), [`crates/project/src/project.rs`](zed-refrence/zed/crates/project/src/project.rs), [`crates/workspace/src/pane.rs`](zed-refrence/zed/crates/workspace/src/pane.rs), [`crates/workspace/src/invalid_item_view.rs`](zed-refrence/zed/crates/workspace/src/invalid_item_view.rs).
- Gepinnte Diagnoseflächen und Renderer: [`crates/editor/src/diagnostics.rs`](zed-refrence/zed/crates/editor/src/diagnostics.rs), [`crates/diagnostics/src/diagnostics.rs`](zed-refrence/zed/crates/diagnostics/src/diagnostics.rs), [`crates/diagnostics/src/buffer_diagnostics.rs`](zed-refrence/zed/crates/diagnostics/src/buffer_diagnostics.rs), [`crates/diagnostics/src/diagnostic_renderer.rs`](zed-refrence/zed/crates/diagnostics/src/diagnostic_renderer.rs), [`crates/diagnostics/src/items.rs`](zed-refrence/zed/crates/diagnostics/src/items.rs), [`crates/workspace/src/pane.rs`](zed-refrence/zed/crates/workspace/src/pane.rs).
- [All Actions](https://zed.dev/docs/all-actions) und [Key Bindings](https://zed.dev/docs/key-bindings): discoverable Actions, kontextbezogene Bindings, Sequenzen und Konfliktregeln.
- [All Settings](https://zed.dev/docs/reference/all-settings), [Visual Customization](https://zed.dev/docs/visual-customization): Editor-, Toolbar-, Tab-, Statusbar-, Scrollbar-, Minimap- und Sprachoptionen.
- [Editing Code](https://zed.dev/docs/editing-code), [Multibuffers](https://zed.dev/docs/multibuffers), [Finding & Navigating](https://zed.dev/docs/finding-navigating), [Outline Panel](https://zed.dev/docs/outline-panel): Editier-, Such-, Ergebnis- und Navigationsflows.
- [Configuring Languages](https://zed.dev/docs/configuring-languages), [Language Extensions](https://zed.dev/docs/extensions/languages), [Supported Languages](https://zed.dev/docs/languages), [Completions](https://zed.dev/docs/completions), [Diagnostics](https://zed.dev/docs/diagnostics): Grammars, LSP, Erweiterungen, Diagnosen und Vorschläge.
- [Git](https://zed.dev/docs/git), [Vim Mode](https://zed.dev/docs/vim), [Markdown](https://zed.dev/docs/languages/markdown), [Project Panel](https://zed.dev/docs/project-panel), [Emmet](https://zed.dev/docs/languages/emmet): Git-/Editor-, Modus- und Preview-Abläufe.
- Öffentliche Dokumentation ist eine volatile Referenz. Für die Paritätsfreigabe sind Verhalten und UI mit dem gepinnten Commit abzugleichen; neuere Settings-/Action-Dokumentation darf den Commit nicht still aktualisieren.

### Labonair-Quellflächen

- Editor Model / Actions / Settings: [`crates/editor/src`](crates/editor/src), [`crates/editor/src/lsp_process.rs`](crates/editor/src/lsp_process.rs), [`crates/editor/src/runtime.rs`](crates/editor/src/runtime.rs), [`crates/settings-content/src/editor.rs`](crates/settings-content/src/editor.rs), [`crates/settings-ui/src/schema.rs`](crates/settings-ui/src/schema.rs), [`crates/editor/src/command_provider.rs`](crates/editor/src/command_provider.rs).
- Editor file lifecycle and storage encoding: [`crates/editor/src/lifecycle.rs`](crates/editor/src/lifecycle.rs), [`crates/editor/src/document.rs`](crates/editor/src/document.rs), [`crates/filesystem/src/file.rs`](crates/filesystem/src/file.rs), [`crates/workspace/src/views/editor.rs`](crates/workspace/src/views/editor.rs), [`crates/workspace/src/workspace.rs`](crates/workspace/src/workspace.rs).
- Language identity, runtime services and settings scopes: [`crates/editor/src/language.rs`](crates/editor/src/language.rs), [`crates/editor/src/language_services.rs`](crates/editor/src/language_services.rs), [`crates/settings-content/src/settings_content.rs`](crates/settings-content/src/settings_content.rs), [`crates/settings/src/store.rs`](crates/settings/src/store.rs), [`crates/settings/src/project.rs`](crates/settings/src/project.rs).
- Workspace appearance and adjacent Settings owners: [`crates/settings-content/src/workspace.rs`](crates/settings-content/src/workspace.rs), [`crates/settings-content/src/file_manager.rs`](crates/settings-content/src/file_manager.rs), [`crates/settings-content/src/terminal.rs`](crates/settings-content/src/terminal.rs), [`crates/settings-content/src/personalization.rs`](crates/settings-content/src/personalization.rs), [`crates/settings-content/src/general.rs`](crates/settings-content/src/general.rs), [`crates/settings-content/src/mcp.rs`](crates/settings-content/src/mcp.rs), [`crates/settings-ui/src/schema.rs`](crates/settings-ui/src/schema.rs).
- AI, MCP, Security und Sprache: [`crates/ai/src/adapters.rs`](crates/ai/src/adapters.rs), [`crates/ai/src/secret_store.rs`](crates/ai/src/secret_store.rs), [`crates/ai/src/tools/security.rs`](crates/ai/src/tools/security.rs), [`crates/ai/src/tools/subagent.rs`](crates/ai/src/tools/subagent.rs), [`crates/mcp-core/src/preferences.rs`](crates/mcp-core/src/preferences.rs), [`crates/mcp-server/src/server.rs`](crates/mcp-server/src/server.rs), [`crates/hosts-ui/src/hosts.rs`](crates/hosts-ui/src/hosts.rs), [`crates/editor/src/language.rs`](crates/editor/src/language.rs).
- Explorer, Editor UI / Search / Preview / Diffs: [`crates/panel-explorer/src/panel_explorer.rs`](crates/panel-explorer/src/panel_explorer.rs), [`crates/workspace/src/views/editor.rs`](crates/workspace/src/views/editor.rs), [`crates/workspace/src/views/preview.rs`](crates/workspace/src/views/preview.rs), [`crates/workspace/src/views/project_diff.rs`](crates/workspace/src/views/project_diff.rs), [`crates/workspace/src/search_overlay.rs`](crates/workspace/src/search_overlay.rs).
- Titlebar and shared tab chrome: [`crates/shell/src/titlebar.rs`](crates/shell/src/titlebar.rs), [`crates/workspace/src/workspace.rs`](crates/workspace/src/workspace.rs), [`crates/ui-kit/src/tab.rs`](crates/ui-kit/src/tab.rs).
- Editor/Workspace contracts and status: [`docs/zed-parity.md`](docs/zed-parity.md), [`docs/parity/feature-crosswalk.md`](docs/parity/feature-crosswalk.md), [`docs/capabilities.md`](docs/capabilities.md), [`docs/settings/catalog.toml`](docs/settings/catalog.toml), [`tasks/rework/R09-001-editor-e0-text-model-contract.md`](tasks/rework/R09-001-editor-e0-text-model-contract.md)–[`R09-007`](tasks/rework/R09-007-editor-e6-persistence-polish-and-p2.md).

**Fortschreibungsregel:** Neue verifizierte Aktion, Einstellung, Zustandsvariante oder Abhängigkeit wird mit Nutzerzweck und Labonair-Owner in die passende Tabelle aufgenommen. `Vorhanden im Code` wird erst nach einem getrennten Laufzeit-/Tastatur-/Visualcheck zu Parität hochgestuft. Der Gesamtvergleich bleibt offen, bis die gepinnte Actionregistry, editorbezogenen und verbundenen Settings, leeren/fehlerhaften Zustände sowie Split- und Preview-Flows gegen die Referenz ausreichend klassifiziert sind.
