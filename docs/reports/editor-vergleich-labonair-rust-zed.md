# Editorvergleich: Labonair Rust und Zed-Referenz

**Stand der Untersuchung:** 11.09.2026
**Labonair Rust:** Commit `d6e6cde55ba05782f588e1ed1e4a5227f3eaf60c`
**Zed-Referenz:** Commit `3569541038dd51524b03998ba4d38d253cb54f80` vom 04.09.2026
**Zusatzreferenz:** `reference-src/` als eingefrorener Vorgänger von Labonair; nur zur Paritätskontrolle, nicht als Architekturvorgabe.

## Kurzfazit

Labonair besitzt bereits einen funktionierenden nativen Editor-Kern: Textbearbeitung, eine Auswahl, Undo/Redo, Dateiladen und atomisches Speichern, Tree-sitter-Syntaxfarben, Soft-Wrap, Suche, ein Outline-/Symbol-Grundgerüst, Vim-Modus, externe Änderungsprüfung und eine GPUI-Ansicht. Der Code ist klein, verständlich und für einen MVP gut testbar.

Im Vergleich zu Zed ist der Editor aber noch ein **einfacher lokaler Texteditor mit Code-Editor-Oberfläche**, während Zed einen vollständigen, sprachbewussten Editor-Stack betreibt. Die größte Lücke liegt nicht bei einzelnen Buttons, sondern im Modell:

1. Labonair bearbeitet einen `Vec<String>`-Puffer mit einem Cursor und Snapshot-History; Zed arbeitet mit Rope-/Anchor-/Transaction-/Snapshot-Abstraktionen.
2. Labonair rendert sichtbare Zeilen direkt; Zed führt Text, Faltungen, Inlays, Tabs, Soft-Wrap, Blöcke, Highlights und Koordinaten über eine `DisplayMap` zusammen.
3. Labonair hat Tree-sitter-Tokenfarben, aber keine Sprachdienste; Zed bindet Language Registry, LSP, Diagnosen, Completion, Hover, Code Actions, Semantic Tokens, Inlays und Folding zusammen.
4. Labonair hat eine lokale Suche im aktiven Puffer; Zed hat eine asynchrone Suche über aktuelle Puffer, Worktrees und Remote-Projekte mit Regex, Ersetzung, Include-/Exclude-Filtern und inkrementellen Ergebnissen.
5. Labonair hat nur zwölf editorbezogene Settings; Zed besitzt eine deutlich feinere Editor-Settings-Schicht für Anzeige, Navigation, Suche, LSP, Completion, Diff, Git, Accessibility und Performance.

Die richtige Konsequenz ist **nicht**, Zed als Ganzes zu kopieren. Labonair sollte Zeds belastbare Konzepte selektiv und unabhängig neu implementieren, in der Reihenfolge:

1. Editor-Vertrag, Rope/Anchors, Transaktionen, mehrere Selektionen und robuster Datei-Lifecycle.
2. Professionelle Kerninteraktionen: Multi-Cursor, Textobjekte, Autoindent, Klammern, Faltungen, horizontales Scrollen, Gutter und konfigurierbare Suche.
3. Projektweite Suche, File Finder, Outline und Navigation.
4. Sprachdienste mit lokalem und remote-fähigem Provider-Vertrag.
5. Inline-Git-/Diagnoseinformationen sowie Completion, Refactoring, Formatting und die fortgeschrittenen UX-Schichten.

Die aktuelle Architekturentscheidung ist dabei grundsätzlich brauchbar: `labonair-editor` besitzt die editorischen Algorithmen, während `workspace/src/views/editor.rs` derzeit den GPUI-Adapter hostet; die Rework-Dokumentation beschreibt genau diese laufende Extraktion.[^arch]

## 1. Untersuchungsrahmen und Bewertungsmaßstab

Verglichen wurden die Quellstrukturen und die vorhandenen Verträge, nicht nur Dateinamen oder Screenshots. Die Aussagen werden getrennt in:

- **Bestand:** im aktuellen Quellstand nachweisbar.
- **Zed-Muster:** im lokalen Zed-Checkout nachweisbar.
- **Bewertung:** daraus abgeleitete Produkt- oder Architekturentscheidung für Labonair.

Die Untersuchung ist ein statischer Source-Audit. Es wurde in diesem Bericht kein Anspruch erhoben, dass jede Zed-Ansicht in einer laufenden Zed-Binary oder jede Labonair-Ansicht in einem manuellen Visual-Check ausgeführt wurde. Für UI-Aufgaben bleiben die im Repository vorgeschriebenen visuellen Prüfungen erforderlich.

### Produktliche Leitplanken

Labonair ist ein nativer, keyboard-first Dev-Op-Arbeitsplatz für lokale und remote Arbeit. Der Editor muss deshalb nicht jede IDE-Funktion von Zed übernehmen. Besonders wertvoll sind Funktionen, die:

- häufige lokale und remote Code-/Konfigurationsänderungen sicher machen,
- Terminal, Explorer, Git, SFTP und Editor verbinden,
- ohne Maus schnell auffindbar sind,
- bei großen oder unzuverlässigen Dateien nicht blockieren,
- verständliche Zustände für Loading, Konflikt, Read-only, Fehler und Remote-Lifecycle zeigen.

Die normative Ownership-Regel bleibt maßgeblich: Ein Feature erhält genau einen Owner und einen kanonischen Einstiegspunkt; `workspace` orchestriert, interpretiert aber nicht dauerhaft editorische Privatdaten. Reports unterstützen diese Regeln, ersetzen sie nicht.[^docs]

## 2. Bestandsaufnahme Labonair Rust

### 2.1 Aktuelles Editor-Modell

Der Kern ist bewusst klein gehalten:

- `TextBuffer` speichert Zeilen in `Vec<String>` und behandelt Spalten als Zeichenpositionen.
- `Document` enthält Buffer, Cursor, optionale Auswahl, Sprache, gespeicherten Text, Disk-mtime, Konfliktflag, Undo-History und Zielspalte für vertikale Bewegung.
- Unterstützte Bewegungen sind links/rechts/hoch/runter, Zeilen-/Dokumentanfang und -ende, einfache Wortbewegung sowie Page Up/Down.
- Es gibt genau einen Cursor und eine optionale lineare Auswahl.
- `History` speichert vollständige Dokument-Snapshots und fasst gleiche Eingaben innerhalb von 600 ms zusammen.

Das ist leicht zu verstehen und ermöglicht gute Unit-Tests. Die gleichen Entscheidungen erzeugen aber die wichtigsten Skalierungsgrenzen: Kopieren kompletter Dokumente für Undo, keine stabilen Anchors bei Einfügungen vor einer Auswahl, keine Transaktionen über mehrere Cursor und keine Darstellungsschicht für virtuelle oder synthetische Textbereiche.[^buffer][^document][^history]

### 2.2 Datei-Lifecycle

Der Dateiadapter hat mehrere gute Sicherheitsentscheidungen:

- Datei-I/O läuft aus dem GPUI-Foreground heraus.
- Standardmäßig werden nur Dateien bis 10 MB geladen.
- Binärdaten und ungültiges UTF-8 werden nicht verlustbehaftet als Text geöffnet.
- Speichern erfolgt über temporäre Nachbardatei, `sync_all` und Rename.
- Externe Änderungen werden über die gespeicherte mtime erkannt; bei sauberem Dokument wird neu geladen, bei Dirty-Dokument wird ein Konfliktzustand angezeigt.

Aktuell fehlen jedoch Encoding-/BOM-Erkennung, Erhalt von CRLF/CR, Dateirechte, gelöschte Dateien als eigener Zustand, explizite Read-only-Fähigkeit, Änderungsprüfung auf Inhaltsbasis und ein Drei-Wege-Konfliktvergleich. Die UI ersetzt Binary- und Too-Large-Zustände derzeit durch einen editierbaren Kommentartext. Das ist als MVP sichtbar, aber riskant: Ein Nutzer kann den Platzhalter wie echten Inhalt behandeln oder versehentlich eine Datei überschreiben.[^file][^editorview]

### 2.3 Syntax, Sprache und Symbole

Labonair unterscheidet 22 `Language`-Varianten über Dateinamen und Endungen. Bundled Tree-sitter-Grammatiken sind aber nur für einen Teil vorhanden: Rust, JSON, TOML, YAML, Python, JavaScript, TypeScript, Go, C, C++, CSS, HTML, Java und Shell. Markdown, SQL, PHP, XML, Ruby, Swift und Kotlin werden zwar erkannt, aber ohne Grammatik dargestellt.[^language]

Der Highlighter ist für den Einstieg sinnvoll optimiert:

- Grammatik-Konfigurationen werden lazy erzeugt und gecacht.
- Es werden nur sichtbare Bereiche plus Randzone verarbeitet.
- Dokumente über 2 MiB werden nicht eingefärbt, um UI-Blockaden zu vermeiden.
- Tree-sitter-Captures werden auf wenige stabile `HighlightKind`-Klassen reduziert.

Die Kehrseite: Der Cache ist kein inkrementeller Syntaxbaum mit edit-basierten Invalidierungsregionen; es gibt keine Semantic Tokens, keine syntaxbasierten Textobjekte, keine Klammerpaare, keine LSP-Symbole und keine sprachspezifische Autoindent-/Injection-Schicht.

Die Symbolerkennung greift auf `tags.scm` zurück, liefert nur Name, grobe Kategorie und Startzeile und unterstützt nur Rust, Python, JavaScript, TypeScript, Go, C, C++, Java. Für viele erkannte Sprachen bleibt die Outline leer. Es gibt keine End-/Body-/Selection-Ranges, keine Symbolhierarchie aus einem LSP, keine fuzzy Suche und keine Ancestor-Kontextzeilen.[^syntax][^symbols]

### 2.4 Suche und Navigation

Der Kernsucher ist literal, zeilenweise und nicht-regulär. Er kann case-sensitive und whole-word, sucht nicht über Zeilen hinweg und ersetzt aktuell alle Treffer back-to-front. Das ist korrekt für einfache Fälle, aber keine professionelle Search API.[^search]

Die zentrale `Cmd+F`-Oberfläche ist ein Workspace-Overlay. Sie entscheidet anhand des aktiven Tabs zwischen Editor und Terminal, besitzt Textfeld, Case-Schalter, Zähler, Vor/Zurück und Schließen. Der Editor übernimmt Treffer und markiert den aktiven Treffer als Auswahl. Whole-word, Regex, Ersetzung, Include-Ignored und Suchmodus sind in diesem Overlay nicht vorhanden.[^searchoverlay][^editorsearch]

Wichtig: Der Filesystem-Crate besitzt bereits eine regex-basierte, gitignore-bewusste Projekt-Grep-Funktion mit Glob-Filtern, Ergebnislimit und Hintergrund-tauglicher Traversierung. Sie ist jedoch noch kein vollständiger editorischer Projekt-Suchworkflow mit Ergebnisbaum, Preview, Ersetzungsplan und Remote-Provider.[^grep]

### 2.5 Vim-Modus

Der Vim-Modus ist für einen produktorientierten Teilumfang überraschend brauchbar: Modi, Zähler, Operatoren, Visual/Visual-Line, Register, Paste, `/`, `?`, `n`, `N` und einige Ex-Kommandos wie `:w`, `:q`, `:wq`, `:e`, `:noh`, `:s`, `:set` sind vorhanden. Der Code ist bewusst als eigene State Machine im Editor-Core organisiert.[^vim]

Er sollte als **unterstützter Subset** kommuniziert werden, nicht als vollständige Vim-Kompatibilität. Für später fehlen unter anderem Makros, Marks, breitere Registersemantik, Textobjekte, `.`-Wiederholung, komplexere Ex-Befehle, Operator-Pending-Sonderfälle, Multi-Cursor-Interaktion und die Kopplung an syntaxbasierte Bewegungen.

### 2.6 GPUI-Ansicht und UI/UX

`workspace/src/views/editor.rs` ist aktuell der sichtbare Editoradapter. Er enthält:

- Zeilennummern-Gutter,
- Caret, lineare Auswahl und aktuelle-Zeile-Band,
- feste Zeichenmetriken,
- Soft-Wrap über Zeichenanzahl,
- manuelles Scrollen über `scroll_top` und Wheel-Events,
- Tree-sitter-Textläufe,
- Clipboard Copy/Cut/Paste,
- Tastaturbearbeitung und Vim-Routing,
- Settings-Schema-Hover für `config.json` und `.labonair/settings.json`,
- Konfliktbanner mit Reload/Keep-Mine,
- Vim-Statuszeile.

Positiv ist die klare Zustandsdarstellung und die spezielle Settings-Hover-Funktion. Die Oberfläche ist aber noch eine direkte Zeilenmalerei: kein horizontales Scrollmodell, kein nativer Scrollbar, keine minimap, keine Faltung, keine Indent-Guides, keine Klammerhervorhebung, kein Inline-Diagnosebereich, keine Inlays, kein Context Menu, keine Completion-Popover, keine Hover-Dokumentation, keine Breadcrumb-/Toolbar-Schicht und keine Gutter-Marker für Git, Breakpoints oder Bookmarks. Die Wrap-Navigation arbeitet mit einer festen Zeichenraster-Annahme und nicht mit einer allgemeinen Display-Koordinatenkarte.[^editorview]

### 2.7 Tabs, Workspace und Nachbarfunktionen

Die Tab-Infrastruktur ist besser als der Editor-Kern selbst:

- `TabKind::Editor` ist ein eigener Tab-Typ mit Dirty- und Peek-Zustand.
- Ein einfacher Klick aus dem Explorer kann ein Peek-Tab wiederverwenden; Bearbeiten oder Doppelklick macht es permanent.
- Unsaved Editor-Tabs erhalten Close-Bestätigung.
- Session-Restore speichert bei Editor-Tabs derzeit im Wesentlichen den lokalen Pfad und öffnet ihn beim Neustart erneut.
- Remote-Dateien werden über eine lokale temporäre Kopie bearbeitet und beim Speichern zurück auf den SSH-Host übertragen.

Die rekursive Split-Tree-Implementierung ist für Workspace-/Terminal-Panes vorhanden. Die Editor-Ansicht wird im Workspace-Renderpfad jedoch direkt als `TabKind::Editor` gerendert; sie ist nicht als editorische Gruppe mit mehreren Editor-Items, Split-Editoren, MultiBuffer-Excerpts oder geteilten Ansichten modelliert. Genau diese Trennung sollte vor einer späteren Editor-Split-Funktion bewusst entschieden werden.[^tabs][^workspace][^panes][^session][^remote]

Explorer und SCM liefern brauchbare Nachbarschaft:

- Der Explorer besitzt lazy Directory-State, Generation Guards gegen verspätete I/O-Ergebnisse, versteckte-Dateien-Schalter, Watcher, Kontextaktionen, Drag/Drop sowie Open/Peek-Verhalten.[^explorer]
- `panel-scm` besitzt Changes/History, Stage/Unstage einzelner Dateien oder Gruppen, Discard, Commit, Branch-/Remote-Aktionen und öffnet den kanonischen Project-Diff.[^scm]
- Der Project-Diff kann Unified/Split darstellen und Hunk-Aktionen ausführen.[^projectdiff]

Die nächste Stufe ist daher nicht ein zweites Git- oder Explorer-System, sondern die editorische Verbindung: aktive Datei im Explorer revealen, Git-Status im Gutter zeigen, Diagnose-/Symbolinformationen am Dateibaum ergänzen und aus Editor-Aktionen in die bestehenden kanonischen Oberflächen navigieren.

## 3. Zed als Referenzmodell

### 3.1 Editor und Rendering

Zed beschreibt den Editor ausdrücklich als Ort, an dem editorische Daten gehalten und dargestellt werden. `EditorElement` ist für das Rendering zuständig; `DisplayMap` ordnet Text in logische Blöcke und hält Transformationsmetadaten für Faltungen, Inlay-Text, Soft-Wrap, Tabs und weitere Darstellungen.[^zededitor]

Der zentrale Unterschied ist die Trennung von:

```text
Quelltext / MultiBuffer
        ↓
Buffer- und Sprachzustand
        ↓
InlayMap → FoldMap → TabMap → WrapMap → BlockMap → Highlights
        ↓
DisplaySnapshot / EditorElement / Gutter / Popovers / Scrollbars
```

Zed kopiert diese Schichten nicht blind bei jeder Eingabe, sondern hält Snapshots und Koordinatenumrechnung pro Layer. Dadurch können virtuelle Inlays, gefaltete Bereiche und Soft-Wrap dargestellt werden, ohne den realen Quelltext zu verändern.[^displaymap]

### 3.2 Buffer und Datei als Produktmodell

Zeds `language::Buffer` ist nicht nur Text. Der Buffer enthält unter anderem Rope-Text, Datei-Handle, Saved-Version und mtime, Transaktionen, Sprache, Syntax-Map, Parse-Status, Diagnosen pro Language Server, Remote-Selektionen, Completion-Trigger, Capability, Conflict, Modeline, Settings, Encoding und BOM. `BufferSnapshot` ist ein immutable, günstig kopierbarer Zustand für Renderer und Hintergrundaufgaben.[^zedbuffer]

Für Labonair ist nicht jedes Feld sofort nötig. Die tragenden Konzepte sind jedoch wichtig:

- **Rope:** Änderungen und Zeilenabfragen ohne vollständiges Dokumentkopieren.
- **Anchor:** Cursor, Auswahl, Diagnostics, Folds und Search-Marker bleiben bei Edits stabil.
- **Transaction:** mehrere Änderungen erscheinen als eine Undo-/Save-/LSP-Einheit.
- **Capability:** Text kann ReadWrite, Read oder ReadOnly sein.
- **File lifecycle:** New, Present, Deleted und historische/vergleichende Quellen sind unterscheidbar.
- **Snapshot:** Renderer und Hintergrunddienste lesen konsistente, immutable Zustände.

### 3.3 Actions und Bedienumfang

Zed registriert eine große Zahl editorischer Actions, statt wichtige Funktionen aus verstreuten Keydown-Sonderfällen zu bauen. Die Action-Liste umfasst unter anderem Bewegungen, Auswahlvarianten, Multi-Selection, Textobjekte, Zeilenoperationen, Faltung, Completion, Signature Help, Hover, Code Actions, LSP-Navigation, Diagnostics, Formatting, Git-Hunks, Bookmarks, Breakpoints, Runnables, Minimap, Inlay Hints, Semantic Highlights und Suche.[^zedactions]

Das Wesentliche für Labonair ist das Muster:

- jede bedeutende Aktion besitzt eine stabile Identität,
- Tastatur, native Menüs und Command Palette rufen denselben Handler auf,
- Actions arbeiten auf dem Editor-Modell statt direkt auf GPUI-Widgets,
- Kontext, Read-only-Zustand und Multi-Cursor werden zentral berücksichtigt.

### 3.4 Gutter, Popovers und Display-Details

Zeds `EditorElement` besitzt separate Layout-/Paint-Pfade für Scrollbars, Minimap, Diff-Hunks, Inline-Diagnosen, Inline-Code-Actions, Blame, Indent-Guides, Bookmarks, Breakpoints, Runnables, Line Numbers, Hover-/Signature-Popovers, Text, Scrollbars und Minimap. Das ist ein Hinweis auf die funktionale Dichte des Editors, nicht die Aufforderung, jede Renderfunktion sofort zu portieren.[^zedelement]

Die UX-Lehre lautet: Der Editor ist ein Informationsraum mit mehreren ruhigen, optionalen Informationskanälen. Gutter, Text, Scrollbar und Popover tragen jeweils unterschiedliche Informationen. Labonair sollte diese Kanäle schrittweise ergänzen, statt zusätzliche dauerhafte Toolbars für jede Funktion einzubauen.

### 3.5 Sprachdienste und Remote

Zeds LSP-Store abstrahiert den Zugriff so, dass der Consumer nicht wissen muss, welcher Language Server dahintersteht. Der Quelltext unterscheidet lokale Lifecycle-Verwaltung, remote Weiterleitung und eine gemeinsame `LspStore`-Schnittstelle.[^lsp]

Der Projekt-/Language-Stack liefert zusätzlich:

- Language Registry und Language Settings,
- Parser/Syntax Maps,
- Completion und Trigger Characters,
- Diagnostics und Related Information,
- Folding Ranges,
- Semantic Tokens,
- Inlay Hints und Code Lens,
- Document Symbols und Links,
- Formatting/Organize Imports,
- Toolchain-/Manifest-/Worktree-Kontext,
- lokale und remote LSP-Adaptionen.

Für Labonair ist das besonders relevant, weil Remote-Editoren schon existieren. Eine lokale Temp-Kopie ist ein guter Fallback für einfache Remote-Dateien, aber eine remote ausgeführte Sprachdienst-Schnittstelle ist die langfristig bessere Lösung für korrekte Projektabhängigkeiten, Include-Pfade, Diagnosen und Definitionen.

### 3.6 Suche und Outline

Zeds Projekt-Suche ist asynchron und liefert entweder vollständige Trefferpositionen oder günstigere Matching-Buffer-Hinweise. Sie unterscheidet lokale Suche, Remote-Suche und nur offene Buffer; die Ergebnisse können gestreamt und begrenzt werden.[^zedprojectsearch]

Die Suchquery unterscheidet Literal- und Regex-Suche, Ersetzung, Whole-Word, Case, Include-Ignored, Include-/Exclude-Pfade und Match-Positionen. Sie unterstützt auch Regex-Replacements mit Capture-Kontext.[^zedsearch]

Zeds Outline-Modell enthält Tiefe, volle Range, Selection-Range, Body-/Annotation-Ranges, Highlight-Ranges und Ancestor-Kontext. Die Suche ist asynchron fuzzy und kann Parent-Kontext vor einem Treffer einblenden.[^zedoutline]

### 3.7 Editor-Settings und Persistence

Zed bündelt Editor-Settings nicht nur als Anzeigepräferenzen. Der Typ enthält unter anderem Cursorform/-animation, Current-Line- und Selection-Highlight, Hover-Verhalten, Toolbar, Scrollbars, Minimap, Gutter, Scroll-Margins, Sticky Scroll, Relative Numbers, Suche, Completion-Menü, Language Detection, Signature Help, LSP-Ergebnisort, Diagnostics-Severity, Code Actions, Drag/Drop, Code Lens, Document Colors/Links und Diff-Verhalten.[^zedsettings]

Die Editor-Persistence speichert neben Pfad, Sprache, Inhalt und mtime auch vertikales/horizontales Scrollen, Selektionen und Folds. Damit öffnet sich ein Editor nach Neustart an einer sinnvollen Stelle, ohne Dirty-/Konfliktinformationen mit der Workspace-Session zu vermischen.[^zedpersistence]

## 4. Vergleichsmatrix

> **Status note (2026-09-12):** The matrix below records the pre-rework
> baseline that motivated the implementation plan. The current implementation
> status is summarized in section 8; the historical baseline is retained so
> the gap analysis remains auditable.

| Bereich | Labonair Rust heute | Zed-Referenz | Bewertung für Labonair |
|---|---|---|---|
| Textmodell | `Vec<String>`, Zeichenpositionen, ein Buffer | Rope, Anchors, Snapshots, Transaktionen, MultiBuffer | **P0 Foundation:** Rope/Anchor/Transaction vor Multi-Cursor, Folding und Inlays |
| Cursor/Auswahl | Ein Cursor, eine lineare Auswahl | Mehrere Selektionen, columnar selection, Selection History, syntax-/word-/subword-basierte Auswahl | **P0:** mehrere stabile Selektionen; **P1:** Syntax- und Spaltenauswahl |
| Undo/Redo | Vollständige Buffer-Snapshots; einfache Coalescing-Regel | Transaktionsbasierte Operationen und Selection-/Buffer-Zustand | **P0:** Edit-Operationen/Transaktionen; keine Snapshot-Kopien pro Zeichen |
| Bewegungen | Grundbewegungen und einfache Wortgrenze | Word/Subword, Paragraph, Syntax Node, Excerpt, Bracket, viele Auswahlvarianten | **P0:** line/word/subword/paragraph/selection; **P1:** Syntax Node/Delimiter |
| Editieroperationen | Insert, Backspace, Delete, Replace-All | Zeilen löschen/duplizieren/verschieben, Join, Sort/Unique, Case, Transpose, Comments, Rewrap, UUID, Tag/Abbreviation | **P0:** Zeilen, Comments, Indent; **P2:** Spezialoperationen selektiv |
| Datei-Encoding | Striktes UTF-8; Binary bei invalidem UTF-8 | Encoding/BOM im Buffer-Modell | **P0:** UTF-8 BOM/CRLF erhalten; **P1:** explizite Encoding-Auswahl |
| Konflikte | mtime, Auto-Reload bei clean, Banner bei dirty | Datei-/Disk-State, Reload-needed, Conflict-Zustand | **P0:** Deleted/Read-only/Conflict als echte Zustände; **P1:** Drei-Wege-Merge |
| Große Dateien | 10-MB-Ladelimit, 2-MB-Syntaxlimit, Platzhalter | Chunk-/Rope-/Snapshot-orientierte Verarbeitung | **P0:** nicht editierbare Large-File-Ansicht plus explizites Öffnen; später streaming/chunked editing |
| Sprache | 22 IDs; nur 14 Grammatikpfade | Language Registry, dynamische Settings, Detection, Syntax/Language Server | **P0:** Registry und Grammar-Metadaten; **P1:** Detection/Language Services |
| Syntax | Tree-sitter, sichtbares Fenster, grobe Tokenklassen | inkrementelle Syntax Maps, semantic tokens, bracket ranges, injections | **P0:** inkrementelle Invalidierung; **P1:** semantic/bracket/injection |
| Suche aktueller Buffer | Literal, zeilenweise, Case/Whole-Word im Kern | Literal/Regex, multiline, Replacement, Search-Highlights, Match-Status | **P0:** Regex/multiline/replace-one/replace-all als Transaktion |
| Projekt-Suche | Filesystem regex backend vorhanden, keine vollständige UI | local/remote/open buffers, streaming, include/exclude, result handles | **P0:** Suchpanel auf vorhandenen `fs_grep`; **P1:** remote/open-buffer parity |
| File Finder | Explorer-Öffnen, kein vollwertiger fuzzy File Finder | Worktree-/Path-/Buffer-Picker und fuzzy Navigation | **P0:** `Open File…`/Recent/Go to Path |
| Symbole/Outline | Tree-sitter tags, Name/Kategorie/Zeile, wenige Sprachen | LSP-/Tree-Symbol, Ranges, Hierarchie, Ancestor-Kontext, fuzzy async | **P0:** Outline-Panel/Popup und echte Ranges; **P1:** LSP-Symbols |
| Folding | nicht vorhanden | Syntax-/LSP-/manuelle Folds, recursive/level/all, Persistenz | **P0.5:** manuelle + syntaxbasierte Folding; Persistenz danach |
| Soft-Wrap | feste Zeichenrasterzeilen | WrapMap mit Display-Koordinaten und Scrollstrategie | **P0.5:** Display-Zeilenmodell; word/character/none; horizontal scroll |
| Tabs/Invisibles | Tabs werden als Text eingegeben; keine sichtbaren Whitespaces | TabMap, Invisibles, Indent Guides, line endings | **P0:** Tab-/Whitespace-/EOL-Anzeige und Indent Guides |
| Completion | keine reguläre Completion | LSP-/Word-/Snippet-Completion, Resolve, Detail-/Scrollbar-Einstellungen | **P1:** LSP Completion + Snippet tabstops |
| Signature Help/Hover | nur Settings-Schema-Hover | LSP Hover, Signature Help, sticky Popovers | **P1:** zuerst Hover, dann Signature Help |
| Diagnostics | keine LSP-Diagnosen | Unterstreichung, Inline-Diagnosen, Gutter, Navigation, Severity-Filter | **P1:** Diagnosemodell + Statusbar/Editor-Integration |
| Code Actions/Refactoring | keine | Quick Fix, Organize Imports, Rename, Definition/References/Implementation | **P1:** Navigation/Rename; **P2:** komplexe Refactorings |
| Formatting | kein aktueller Rust-Formatterpfad | Format on save/manual/range, Format-Transaction | **P1:** providerbasierter Formatter; Format on Save als opt-in |
| Git im Editor | separater Project-Diff; SCM Stage/Unstage | Inline Diff-Hunks, Apply/Restore Hunk, Blame, Review Comments | **P1:** Gutter-Diff + Navigation; Project-Diff bleibt kanonische Review-Fläche |
| Debug/Tasks | keine editorische Integration | Breakpoints, Run/Debug, Runnables, inline values | **P2**, nur bei konkretem Debugger-/Task-Workflow |
| Gutter | Zeilennummern | Nummern, Folds, Git, Diagnostics, Breakpoints, Bookmarks, Runnables | **P0.5:** modularer Gutter; Marker einzeln zuschaltbar |
| Scrollen | manueller Vertical-Scroll, kein Scrollbar-Modell | vertikal/horizontal, Scroll-Margins, Beyond-Last-Line, Scrollbar-Marker | **P0.5:** ScrollManager/Scrollbar; **P2:** Minimap |
| Minimap | nicht vorhanden | konfigurierbare Minimap mit Thumb/Border/Current-Line | **P2**, nach stabiler Display-/Performancebasis |
| Toolbar/Breadcrumbs | keine editorische Toolbar; Statusbar Cursorposition | Breadcrumbs, Quick Actions, Selection Menu, Code Actions | **P1:** kompakte, optionale Editor-Toolbar |
| Mouse UX | Click/Drag-Selection-Grundlage | Drag-and-drop selection, context menu, double-click modes, middle paste | **P0.5:** Context Menu, Click-Zonen, Drag/Drop |
| Keymap | zentrale typed IDs, Editor aktuell nur Find/GoToSymbol | umfangreiche Editor-Actions mit Kontexten und Parametern | **P0:** alle Kernaktionen als stabile Owner-Beiträge |
| Vim | eigener umfangreicher Subset | Zed delegiert Vim an separate Vim-Schicht | **Keep:** Subset testen und an neuen Selection-/Transaction-Kern anbinden |
| Settings | 12 Editorwerte plus Vim-Optionen | breiter EditorSettings-Typ | **P0:** fehlende Basis-Settings; **P1:** per-language/LSP/display |
| Session | Editor-Pfad, aktive Tab-Position | Pfad/Inhalt/Sprache/mtime, Scroll, Selections, Folds in DB | **P1:** Cursor/Scroll/Folds; Dirty-Inhalt nur bewusst und sicher |
| Remote | Temp-Kopie + Upload beim Speichern | lokale und remote Buffer/LSP-Store, RPC | **P0:** Fehler-/Konfliktzustände; **P1:** remote LSP/remote search |
| Accessibility | FocusHandle und Tooltips, GPUI-Rollen begrenzt | keyboard navigation, labelled controls, focus/toolbar patterns | **P0.5:** sichtbarer Keyboard-Fokus, Tooltips, Shortcuts, Kontrast |
| Performance | einfache sichtbare Zeilen, Highlighterfenster | Snapshots, SumTree, chunked maps, async parsing/search | **P0:** messen; **P1:** inkrementeller Parser und virtualisierte Zusatzkanäle |

## 5. Detaillierte Empfehlungen nach Priorität

### P0 — Editor vertrauenswürdig und täglich brauchbar machen

#### P0.1 Editor-Contract und Textmodell

Zuerst sollte `labonair-editor` einen stabilen öffentlichen Editor-Vertrag erhalten, bevor UI-Features die einfache `Document`-API weiter vergrößern. Der Contract sollte mindestens folgende Konzepte abbilden:

```text
BufferId / DocumentId
TextSnapshot
Anchor / AnchorRange
SelectionSet
Edit / Transaction
BufferEvent
FileState
LanguageId
```

Empfohlene Umsetzung:

1. `TextBuffer` schrittweise durch eine Rope-basierte Struktur oder eine gleichwertige, gemessene Textrepräsentation ersetzen.
2. `Position` nur als UI-/API-Koordinate verwenden; intern Offsets und Anchors einsetzen.
3. Eine Änderung als `Transaction` mit Edits, SelectionBefore/After, Source und optionalem Label modellieren.
4. Cursor, Auswahl, Search-Dekoration, Fold, Diagnose und Git-Hunk an Anchors hängen.
5. `TextSnapshot` als read-only Sicht für Rendern, Syntax, Suche und LSP verwenden.
6. Property-Tests für Unicode, CRLF, große Einfügungen, Backspace über Zeilen, Mehrfachauswahl und Undo/Redo ergänzen.

Nicht empfohlen ist ein sofortiges Zed-ähnliches MultiBuffer-/Excerpt-System ohne aktuellen Use Case. Es sollte erst nach stabilen Anchors entstehen, wenn Split-Editoren, Diff-Ansichten oder AI-Kontext tatsächlich mehrere Textbereiche brauchen.

**Akzeptanz:** 100.000-Zeilen-Datei bleibt bei Cursorbewegung und typischer Eingabe responsiv; Undo kopiert nicht für jeden einzelnen Tastendruck den vollständigen Text; jede Selection bleibt bei Edits vor ihr korrekt.

#### P0.2 Datei- und Konfliktzustände

Das bestehende atomische Speichern und die Hintergrund-I/O sollten erhalten bleiben. Ergänzt werden:

- `FileState::New | Present | Deleted | ReadOnly | Binary | TooLarge | Loading | Error | Conflict`;
- Erhalt der erkannten Zeilenenden beim Speichern;
- UTF-8-BOM-Erhalt und später explizite Encoding-Erkennung/-Auswahl;
- Dateiinhalt als Basis-Snapshot neben mtime und Größe;
- Konfliktansicht: Disk-Version, eigene Version, gemeinsame Basis;
- explizite Aktionen `Reload`, `Keep Mine`, `Compare`, `Save As`, `Open Read Only`;
- Autosave `off | afterDelay | onFocusChange`, jeweils mit Konfliktprüfung;
- opt-in `trimTrailingWhitespace`, `insertFinalNewline`, `formatOnSave` als klar getrennte Save-Transforms.

Der Platzhalter für Binary/Too-Large sollte nicht als normales editierbares Dokument in den Buffer geschrieben werden. Besser ist ein read-only Status-View mit Aktionen: „Als Text öffnen“, „Encoding wählen“, „Nur anzeigen“, „Trotzdem öffnen“.

**Akzeptanz:** Kein externer Inhalt wird durch lossy UTF-8-Decoding beschädigt; Dirty-Dateien werden nie still überschrieben; Löschen/Umbenennen auf Disk wird als eigener Zustand sichtbar.

#### P0.3 Multi-Cursor und professionelle Grundedits

Das ist der größte tägliche Funktionssprung nach dem Textmodell:

- mehrere Carets und Selektionen;
- `Select Next Match`, `Select All Matches`, `Add Selection Above/Below`;
- columnar/rectangular selection;
- Copy/Cut/Paste für Selection Sets;
- line start/end, first non-whitespace, page, paragraph, word und subword;
- select line, select to start/end, delete line, delete to end of line;
- duplicate/move line up/down, join lines, transpose;
- indent/outdent/autoindent;
- toggle line/block comment per Sprache;
- matching bracket und select inside/around delimiters;
- tabstops und einfache Snippets.

Jede dieser Aktionen sollte auf dem Editor-Contract arbeiten. Die GPUI-Schicht darf nicht selbst für jeden Cursor Textspans berechnen und direkt in `Document` mutieren.

**Akzeptanz:** dieselbe Aktion funktioniert mit einem und mit vielen Cursors; eine Änderung bildet einen verständlichen Undo-Schritt; Read-only-Buffer blockieren Mutation sichtbar und ohne Notification-Spam.

#### P0.4 Suche, Replace und Navigation

Die vorhandene Suchengine sollte als typed Query ausgebaut werden:

```text
SearchQuery {
    pattern,
    mode: Literal | Regex,
    case_sensitive,
    smart_case,
    whole_word,
    multiline,
    include_ignored,
    scope: CurrentBuffer | OpenBuffers | Project | Selection,
}
```

Dazu gehören:

- Regex und multiline;
- Whole-Word und Smart-Case als echte Settings/Toolbar-Zustände;
- `Replace`, `Replace Next`, `Replace All` mit Preview und einem Undo-Transaction;
- Treffer-Dekorationen für alle Treffer, nicht nur aktive Auswahl;
- Suchhistorie pro Scope und Datei;
- `Esc` schließt das Overlay, ohne Suchzustand des Dokuments unnötig zu verlieren;
- `Go To Line:Column`, `Go To File`, `Go To Symbol`, `Go Back/Forward`;
- projektweite Suche mit Ergebnisbaum `Datei → Zeile → Treffer`, Ergebnislimit, Cancel und Loading/Truncated-State;
- Include-/Exclude-Glob, gitignored-Schalter und remote Suchadapter.

Die vorhandene `labonair-filesystem::fs_grep`-Funktion ist ein guter Traversal-Baustein. Die editorische Query- und Ergebnislogik sollte aber nicht als untypisierte Shell-Sonderbehandlung im Workspace landen.

**Akzeptanz:** `Cmd+F` kann Literal/Regex/Whole-Word/Replace im aktuellen Buffer; `Cmd+Shift+F` liefert Streaming-Projektergebnisse; `Replace All` ist in einem Undo-Schritt rückgängig zu machen.

### P0.5 — Display-Grundlage und UI/UX

#### Display-State statt direkter Zeilenmalerei

Vor Folding, Inlays und Diagnosen sollte Labonair eine kleinere eigene Display-Schicht einführen:

```text
source offset/point
    ↕
display point / visual row
    ↕
layout rectangles / gutter / scrollbar
```

Sie muss zunächst nur Tabs, Zeilenenden, Soft-Wrap, Horizontalscroll und sichtbare Ranges abbilden. Später können Folds, Inlays und Inline-Blöcke als zusätzliche Layer hinzukommen. Das ist die sinnvolle, kleinere Übertragung des Zed-`DisplayMap`-Prinzips.

#### Konkrete UI-Verbesserungen

1. **Gutter:** aktuelle Zeile, absolute/relative Nummern, Fold-Chevrons, Git-Diff-Marker, Diagnostics-Marker, Bookmarks und Breakpoints als unabhängige Slots.
2. **Scrollbars:** vertikal und horizontal, mit optionalen Markern für Search, Diagnostics und Git; Drag-Thumb und `scroll_beyond_last_line`.
3. **Wrap:** `none`, `word`, `character`; korrekte Home/End-/Up/Down-Navigation über Visual Rows.
4. **Whitespace:** Tabs, Spaces, Trailing Whitespace und EOL sichtbar konfigurierbar.
5. **Caret/Selection:** Cursorform, Blink, Current-Line-Highlight, Selection-Highlight, Fokuszustand und Read-only-Zustand.
6. **Klammern:** passende Klammer hervorheben; bei ungültiger Struktur keine falsche Sicherheit suggerieren.
7. **Context Menu:** Copy/Cut/Paste, Select All, Go To, Format, Comment, Fold, Git-Hunk-Aktionen; aus `labonair-ui-kit` zusammensetzen.
8. **Popover:** Completion, Hover, Signature Help und Code Actions müssen eine gemeinsame, keyboard-fähige Popover-Interaktion erhalten.
9. **Toolbar:** optional und kompakt: Sprache, Dirty-Status, Cursor/Selection-Statistik, Format, Outline, Display-Menü. Dauerhafte Controls nur für häufige Aufgaben.
10. **Fokus:** sichtbarer Keyboard-Fokus und Shortcut-Hinweise; Hover ist Ergänzung, nicht alleinige Beschriftung.

Minimap, Sticky Scroll und Inline-Blame sind wertvoll, aber erst sinnvoll, wenn Display-Koordinaten, horizontales Scrollen und große Dateien zuverlässig funktionieren.

**Akzeptanz:** normal, schmal, leer, Loading, Error, Read-only, Dirty, Conflict, Wrapped und sehr lange Zeile sind visuell unterscheidbar; jede neue wiederverwendbare Schaltfläche/List/Menu/Input-Komponente stammt aus `labonair-ui-kit`.

### P1 — Sprachbewusster Editor

#### Language Registry

Aus dem festen `Language`-Enum sollte ein typed Registry-Modell werden, ohne die Ownership zu verwässern. Ein Language Provider beschreibt:

- ID, Label, Dateinamen/Endungen und optional Content Detection;
- Tree-sitter-Grammatik und Highlight Queries;
- Indent Unit, Tab-Regeln und Comment Tokens;
- Injections/Embedded Languages;
- Formatter-/LSP-Provider-IDs;
- Fähigkeiten: Completion, Hover, Diagnostics, Folding, Symbols, Formatting.

Grammatiken sollten weiterhin lazy geladen werden. Language Packs müssen nicht alle in den Prozessstart gezogen werden.

#### LSP-Vertrag

Unter dem Editor-Owner sollte ein kleiner typed Provider-Vertrag entstehen, z. B. mit Operationen für:

- initialize/shutdown/restart/status;
- didOpen/didChange/didSave/didClose;
- completion und completion resolve;
- hover und signature help;
- definition/declaration/type definition/implementation/references;
- rename;
- diagnostics und related information;
- code actions;
- document symbols;
- formatting/range formatting/organize imports;
- folding ranges, semantic tokens, inlay hints, document links/colors.

Der Vertrag sollte die Quelle `Local` oder `Remote` nicht in jedem Editor-Consumer sichtbar machen. Ein lokaler Process Adapter und ein SSH-/Remote Adapter implementieren denselben Contract. Workspace liefert nur Kontext/Host, er besitzt keine LSP-Privatlogik.

Empfohlene Reihenfolge:

1. Server-Lifecycle und Statusmodell.
2. Diagnostics + Gutter + Navigation.
3. Completion + Hover.
4. Definition/References/Rename.
5. Formatting/Code Actions.
6. Folding/Semantic Tokens/Inlays/Code Lens.

#### Diagnostics UX

Diagnosen müssen in drei Ebenen erscheinen können:

- leiser Gutter-Marker und Statusbar-Zähler,
- unterstrichener Bereich im Text,
- Popover/Panel mit Message, Severity, Source, Related Information und Code Action.

Zusätzlich braucht es `Next Diagnostic`, `Previous Diagnostic`, Severity-Filter und einen klaren Zustand für „LSP läuft noch“, „keine Diagnosefähigkeit“ und „Server fehlerhaft“. Inline-Text sollte optional sein und darf bei vielen Diagnosen nicht das Layout unleserlich machen.

### P1 — Git, Diff und Review

Labonair hat bereits eine kanonische Project-Diff-Fläche und SCM-Operationen. Darauf aufbauend:

- inline Git-Gutter: Added/Modified/Deleted;
- `Next Change`, `Previous Change`, `Go To Hunk`;
- Hunk öffnen im Project-Diff;
- Stage/Unstage/Restore Hunk aus dem Gutter-Kontextmenü;
- optional inline blame bei Hover;
- Review-Kommentar nur, wenn der vorhandene AI-/Review-Workflow dafür einen konkreten Consumer besitzt.

Der vollständige Diff-Review bleibt im bestehenden Project-Diff. Eine zweite, eigenständige Diff-Ansicht im Editor würde die normative „eine kanonische Diff-Fläche“-Regel verletzen. Inline-Gutter und Hunk-Aktionen sind Dekoration und Navigation, nicht ein paralleler Review-Owner.

### P1 — Tabs, Splits und Editor-Persistenz

Es sollte eine Entscheidung dokumentiert werden:

- **Variante A:** Editor bleibt eigener Tab und erhält Editor-Splits innerhalb des Editor-Items.
- **Variante B:** Editor wird ein Pane-Content im Workspace-Splitbaum.

Für Labonairs Produkt ist Variante A zunächst risikoärmer: Terminal-Splits und Editor-Tabs bleiben getrennt; der Editor kann intern eigene Gruppen verwalten. Für echte Cross-Feature-Splits, Diff-Excerpts und MultiBuffer ist Variante B langfristig näher an Zed. Diese Entscheidung gehört in ein ADR, bevor UI- und Session-Modelle doppelt gebaut werden.

Unabhängig von der Variante sollten pro Datei gespeichert werden:

- Cursor und Selection Set;
- vertikaler/horizontaler Scroll;
- Sprache/Override;
- Fold-Ranges mit Fingerprint oder Anchor-ähnlicher Stabilisierung;
- optional Search-Zustand;
- kein Dirty-Inhalt ohne explizite Recovery-/Draft-Policy.

### P2 — Selektive fortgeschrittene Funktionen

Diese Funktionen sind sinnvoll, aber nach dem Kern zu priorisieren:

- Minimap;
- Sticky Scroll/Breadcrumbs mit Syntax-/LSP-Symbolen;
- Code Lens und Inlay Hints;
- inline Debug Values, Breakpoints und Runnables;
- Makros und erweiterter Vim-Modus;
- AI Inline Edit Predictions;
- Multibuffer/Excerpts für Search, Diff und AI-Kontext;
- Notebook/Jupyter-Unterstützung;
- kollaborative Remote-Selektionen.

Die Auswahl sollte jeweils einen vorhandenen Labonair-Workflow voraussetzen. Zeds `Jupyter`, Collaboration und Edit Prediction sind keine sinnvollen Foundation-Abhängigkeiten für den ersten professionellen lokalen Editor.

## 6. Einstellungen: Ist-Zustand und empfohlene Zielschicht

### 6.1 Aktuell in Labonair

Der typed `EditorContent`-Bereich enthält derzeit:

```text
editorFontFamily
editorFontSize
editorTabSize
editorWordWrap
editorLineNumbers
editorRelativeLineNumbers
editorIndentWithTabs
vimMode
editorTheme
vimHlsearch
vimIncsearch
vimSmartcase
```

Diese Werte werden bereits live in `EditorView` gelesen; die Settings-UI gruppiert sie in Keybindings, Theme, Font, Behaviour, Indentation und Display.[^labonairsettings]

### 6.2 Zed-relevante Settings-Familien

Zeds EditorSettings zeigt folgende Familien, die für Labonair als Auswahlkatalog dienen können:

| Familie | Kandidaten für Labonair |
|---|---|
| Datei/Lifecycle | `maxFileSize`, Encoding, BOM, EOL, autosave, autosave delay, format on save, trim trailing whitespace, final newline |
| Cursor/Selection | cursor blink, cursor shape, cursor animation, current line highlight, selection highlight, rounded selection, multi-cursor modifier |
| Layout | line height, horizontal/vertical scroll margin, scroll beyond last line, scroll sensitivity, mouse-wheel zoom, sticky scroll |
| Gutter | line numbers, relative numbers, folds, git gutter, diagnostics, bookmarks, breakpoints, runnables |
| Suche | button, whole word, case sensitive, smart case, regex, include ignored, search on type, center on match, wrap, seed from cursor |
| Sprache/LSP | language detection, hover delay/sticky, signature help, diagnostics max severity, result location, code actions, formatting, semantic highlights |
| Inline-Informationen | inlay hints, code lens, document links, document colors, inline diagnostics, inline blame |
| Completion | menu scrollbar, detail alignment, item-kind display, snippet order, trigger behavior |
| Diff/Git | diff view style, minimum split width, show full file, inline hunk controls |
| Accessibility/Privacy | minimum highlight contrast, redact private values, reduce motion, keyboard-focus visibility |
| Advanced | Minimap, Jupyter, multibuffer/excerpt context, drag-and-drop selection, collaboration/AI policies |
```

### 6.3 Stufenplan für Settings

**P0 — sofort wertvoll:**

```text
editorLineHeight
editorAutoSave
editorAutoSaveDelay
editorBracketMatching
editorIndentationGuides
editorTrimTrailingWhitespace
editorInsertFinalNewline
editorFormatOnSave
editorMaxFileSize
editorShowCursorPosition
editorShowSelectionStats
editorShowOutline
searchRegex
searchWholeWord
searchCaseSensitive
searchIncludeIgnored
searchWrap
```

**P1 — nach dem Display-/LSP-Kern:**

```text
editorCursorShape
editorCursorBlink
editorCurrentLineHighlight
editorSelectionHighlight
editorShowScrollbars
editorHorizontalScroll
editorStickyScroll
editorHoverEnabled
editorHoverDelay
editorDiagnostics
editorInlayHints
editorCodeLens
editorSemanticHighlights
editorDocumentLinks
editorDocumentColors
editorMouseWheelZoom
editorScrollSensitivity
```

**P2 — nur bei Bedarf:**

```text
editorMinimap
editorMinimapDisplayIn
editorMinimapThumb
editorCompletionMenuScrollbar
editorCompletionDetailAlignment
editorSnippetSortOrder
editorMultibufferExcerptContext
editorJupyter
editorCollaborationCursors
editorEditPrediction
```

Jedes neue Setting muss einen realen Consumer, Scope und Default besitzen. Display- und Search-Settings gehören in den Editor-Owner; Host-/LSP-Prozesskonfiguration darf nicht in eine allgemeine Settings-Struct ohne Owner wandern. Per-Language-Settings sollten über `LanguageId`-Scope gemergt werden, nicht über verstreute `match`-Blöcke.

## 7. Commands und Keymap

Der Editor registriert aktuell nur `Find` und `GoToSymbol`; `Save`, Tab- und Layoutaktionen kommen aus anderen Ownern.[^editorcommands] Das genügt für den jetzigen Umfang, würde aber bei jedem neuen Feature zu einem UI-Sonderfall führen.

Empfohlene Editor-Beiträge zur zentralen typed Command Registry:

### Navigation und Selection

```text
MoveLeft, MoveRight, MoveUp, MoveDown
MoveToLineStart, MoveToLineEnd, MoveToFirstNonWhitespace
MoveWordLeft, MoveWordRight, MoveSubwordLeft, MoveSubwordRight
MovePageUp, MovePageDown, MoveDocumentStart, MoveDocumentEnd
SelectLeft, SelectRight, SelectUp, SelectDown
SelectLine, SelectAll, SelectWord, SelectNextMatch, SelectAllMatches
AddSelectionAbove, AddSelectionBelow, ToggleColumnarSelection
GoToLine, GoToFile, GoToSymbol, GoBack, GoForward
```

### Editing

```text
Backspace, Delete, DeleteLine, DeleteToEndOfLine
DuplicateLineUp, DuplicateLineDown, MoveLineUp, MoveLineDown
JoinLines, Indent, Outdent, AutoIndent, ToggleComment
Transpose, Rewrap, ConvertCase, InsertSnippet
Undo, Redo, UndoSelection, RedoSelection
```

### Language und Search

```text
Find, FindNext, FindPrevious, Replace, ReplaceNext, ReplaceAll
ShowCompletions, ShowSignatureHelp, Hover, CodeActions
GoToDefinition, GoToDeclaration, GoToImplementation
GoToTypeDefinition, FindReferences, Rename
GoToDiagnostic, GoToNextDiagnostic, GoToPreviousDiagnostic
FormatDocument, FormatSelection, OrganizeImports
```

### Display und Git

```text
ToggleLineNumbers, ToggleRelativeLineNumbers, ToggleWordWrap
ToggleIndentGuides, ToggleWhitespace, ToggleFolding, Fold, UnfoldAll
ToggleMinimap, ToggleStickyScroll, ToggleDiagnostics, ToggleInlayHints
GoToNextChange, GoToPreviousChange, GoToHunk
StageHunk, UnstageHunk, RestoreHunk, ToggleBlame
```

Jede Action braucht Context `Editor`, einen Handler im Editor-Owner und fokussierte Tests. Parametrisierte Actions wie „fold at level“ oder „select next N matches“ sollten typed Payloads erhalten, nicht frei formatierte Strings. Die Palette kann daraus statische Beschreibungen und dynamische Untermenüs generieren.

## 8. Architektur-Zielbild für Labonair

### 8.1 Empfohlene Verantwortungsgrenzen

```text
labonair-editor (Owner)
├── text/buffer: Rope, offsets, anchors, snapshots
├── editing: selections, transactions, history, motions
├── display: visible rows, wraps, folds, tabs, decorations
├── language: language registry, syntax, symbols
├── search: buffer/project query contracts
├── language services: LSP/formatter provider contract
├── git decorations: typed hunk/blame consumer contract
├── commands/keymap contributions
├── editor settings and persistence contract
└── editor-owned view/UI boundary when justified

labonair-filesystem / integration siblings
├── local file read/write/watch
├── project traversal/grep
└── local/remote process and LSP adapters

labonair-workspace (Host/Composition)
├── tab/pane activation
├── open/close/focus routing
├── modal layer hosting
└── typed connections to Explorer, SCM, SSH/SFTP, Notifications
```

Die Rework-Dokumentation stellt bereits fest, dass `labonair-editor` die Algorithmen besitzt und die Workspace-View als GPUI-Adapter fungiert. Das sollte nicht durch eine übereilte Verschiebung in den UI-freien Kern gebrochen werden.[^arch]

Ein optionaler `labonair-editor-ui`-Sibling ist nur dann gerechtfertigt, wenn die GPUI-View wirklich als eigener UI-Boundary benötigt wird. Kein leerer `core`, `api` oder `facade`-Crate nur wegen Namenssymmetrie.

### 8.2 Event-/Contract-Vorschläge

Der minimale typed Event-Satz:

```text
EditorEvent::Edited { document_id, transaction }
EditorEvent::Saved { document_id, path, mtime }
EditorEvent::ReloadNeeded { document_id, disk_state }
EditorEvent::LanguageChanged { document_id, language_id }
EditorEvent::DiagnosticsChanged { document_id, summary }
EditorEvent::SymbolsChanged { document_id }
EditorEvent::SelectionChanged { document_id, summary }
EditorEvent::RequestOpenPath { path, mode }
EditorEvent::RequestOpenProjectDiff { path, hunk }
EditorEvent::RequestNotification { severity, title, details }
```

Der Workspace sollte daraus Tab-Dirty, Fokus und Navigation ableiten. Er sollte nicht den Buffer direkt aus einer `HashMap<u64, Entity<EditorView>>` auslesen, sobald der öffentliche Editor-Snapshot diese Informationen liefern kann.

### 8.3 Persistenz

Editor-Persistenz sollte editor-owned sein; Workspace-Session speichert nur Tab-/Workspace-Identität und referenziert den Editor-Zustand. Vor einer Datenbank ist eine versionierte, atomische JSON-/SQLite-Struktur mit klarer Migration ausreichend. Wichtig ist die Trennung:

- Session: welcher Tab/Workspace ist offen?
- Editor state: wo war der Cursor, Scroll, Fold, Language Override?
- Document content: existiert nur bei bewusstem Recovery-/Draft-Konzept.

## 9. Nachbarfunktionen und UX-Verknüpfung

### Explorer

Beibehalten:

- lazy directories und Generation Guards,
- watcher-basiertes Refresh,
- Peek/Permanent-Open-Semantik,
- Multi-Selection und Drag/Drop.

Ergänzen:

- `Reveal Active File` als Editor → Explorer-Contract;
- Git-Status und Diagnosebadge pro Datei;
- Outline/Symbol-Zähler nur optional, nicht als neue permanente Spalte;
- Keyboard-Navigation und stabile Focus-/Selected-/Marked-Zustände;
- file finder aus derselben Project-/Path-Quelle statt doppelter Traversierung.

Die Explorer-Virtualisierung sollte gegen große Repositories gemessen und nicht nur nach Elementanzahl beurteilt werden. Editor-Projekt-Suche und Explorer müssen dieselben Ignore-/Root-Regeln verwenden.

### SCM und Project Diff

Der SCM-Panel-Workflow `Changes → Stage/Unstage → Project Diff → Commit` ist bereits der richtige kanonische Pfad. Der Editor sollte nur zusätzliche Nähe herstellen:

- modified lines in Gutter;
- Hunk-Navigation;
- Stage/Unstage/Restore über Kontextmenü;
- Konfliktmarker mit direktem Sprung in die Project-Diff-/Merge-Ansicht;
- optional Blame-Popover nach erfolgreicher Git-Abfrage.

Destruktive Aktionen bleiben bestätigt und über die Notification-/Dialogregeln des Produkts geführt; es gibt kein neues Toast-System.

### Terminal und Remote-Dateien

Die vorhandene Remote-Temp-Kopie ist sofort nutzbar und sollte als Fallback erhalten bleiben. Für eine professionelle remote-IDE-Erfahrung braucht Labonair danach:

- Remote-Datei-Statustypen statt nur `(remote)` im Tabtitel;
- Upload-/Download-Fehler als actionable Notification mit Retry;
- externe Remote-Änderung und lokale Dirty-Änderung als Konflikt;
- remote Language Service und remote Project Search, sofern der Host dies unterstützt;
- einheitliche Cancellation bei SSH-Verbindungsabbruch.

### Settings-Hover

Der Schema-Hover über Labonair-Settings ist eine gute eigene Differenzierung. Er sollte als Muster für weitere kontextsensitive Hilfen dienen:

- JSON-Schema-Hover mit Typ, Default und Scope;
- Go-to-setting bzw. Command zum Öffnen des Settings-Eintrags;
- Validierungsdiagnose für unbekannte/ungültige Keys;
- kein generischer Hover-Mechanismus, der jede Datei ohne LSP kostenintensiv parst.

## 10. Konkreter Implementierungsfahrplan

### Phase E0 — Vertrag und Messbasis

- **Owner:** `editor`
- **Eintritt:** aktueller `Document`-/`EditorView`-Workflow bleibt nutzbar.
- **Arbeit:** Editor-Snapshot, Events, FileState, SelectionSet, Command-Liste, Performance-Messungen.
- **Verifikation:** Contract-Tests; Unicode/CRLF/large-file baseline; keine UI-Regression.

### Phase E1 — Buffer/File Foundation

- **Arbeit:** Rope oder gemessene Alternative, Anchors, Transactions, Operation-History, EOL/BOM, Read-only/Deleted/Conflict.
- **Settings:** Max file size, EOL/Encoding UI, Autosave.
- **Persistence:** nur Migration für Cursor-/Scroll-Grundlagen.
- **Verifikation:** property tests, crash-safe save, external-change matrix, 10/100/500-MB read-only tests.

### Phase E2 — Editing Core

- **Arbeit:** Multi-Cursor, Selection Sets, word/subword, line operations, comments, indent, brackets, snippets.
- **Commands:** alle Navigation-/Editing-Actions als Owner-Beiträge.
- **UI:** Context Menu, selection decorations, visible focus.
- **Verifikation:** golden interaction tests für one/many cursors, Vim + normal mode, Undo/Redo.

### Phase E3 — Display und Search

- **Arbeit:** Display rows, horizontal scroll, scrollbar, whitespace, folding, wrapping, Gutter slots; SearchQuery, Replace, project search UI.
- **Integration:** `fs_grep`, Explorer root/ignore rules, Notification Center.
- **Verifikation:** narrow/large/wrapped/folded visual checks; regex replacement; cancellation; truncated results.

### Phase E4 — Language Intelligence

- **Arbeit:** Language Registry, parser lifecycle, LSP provider, diagnostics, hover, completion, symbols, navigation, rename, formatting.
- **Remote:** provider mode local/SSH.
- **Verifikation:** fake LSP tests; server crash/restart; stale-response rejection; diagnostics in clean/dirty/reloaded buffer.

### Phase E5 — Git/Review

- **Arbeit:** inline diff gutter, hunk navigation/actions, blame, conflict bridges to Project Diff.
- **Verifikation:** new/deleted/renamed files, staged vs unstaged, line movement, external Git refresh.

### Phase E6 — Polish/Advanced

- **Arbeit:** Toolbar/Breadcrumbs, Sticky Scroll, Minimap, Code Lens/Inlays, Tasks/Debug, AI inline, collaboration.
- **Verifikation:** feature-specific settings, accessibility/focus, performance under dense decorations.

Für jede Phase muss vor Codebeginn der Lifecycle-Eintrag Owner, Entry Point, Contract, State, Commands, Notifications, Settings, Persistence, UI-kit und Removal Condition beschreiben. Die Phase sollte als Task in `tasks/rework/` oder einer abgeleiteten Implementation-Aufgabe erfasst werden; dieser Bericht allein ändert die normative Roadmap nicht.[^lifecycle]

## 11. Priorisierte Entscheidungsliste

### Einbauen / hohe Priorität

- Rope-/Anchor-/Transaction-Fundament.
- robuste FileStates, Encoding/EOL/BOM, Conflict/Reload/Read-only.
- Multi-Cursor und Selection Sets.
- Zeilen-/Wort-/Subword-/Comment-/Indent-/Bracket-Grundfunktionen.
- Regex-/Multiline-/Replace-Suche.
- Projektweite Suche auf Basis des vorhandenen Filesystem-Backends.
- File Finder, Go-to-Line, Outline-Panel und fuzzy Symbolnavigation.
- Folding, Indent Guides, Whitespaces, horizontales Scrollen, Scrollbars.
- Language Registry und später LSP mit Diagnostics zuerst.
- Git-Gutter und Hunk-Navigation, ohne zweite Diff-Review-Fläche.
- Editor-Command-Beiträge für Kernaktionen.
- Editor-spezifische Persistenz von Cursor/Scroll/Folds.
- Autosave und die aus `reference-src` bekannten Save-Transforms als opt-in.

### Einbauen / mittlere Priorität

- Completion und Snippets.
- Hover, Signature Help, Code Actions, Rename und Formatting.
- Breadcrumbs und kompakte Editor-Toolbar.
- Inline blame, Code Lens und Inlay Hints.
- Remote LSP und Remote Project Search.
- Breakpoints, Runnables, Task-Navigation, wenn Debug-/Task-Owner bereit ist.

### Bewusst später oder nur bei konkretem Workflow

- Minimap.
- Jupyter/Notebook.
- vollständige Vim-Kompatibilität.
- MultiBuffer-/Excerpt-System.
- Kollaboration und Remote-Cursors.
- AI Edit Prediction.
- Zed-spezifische Settings ohne Labonair-Consumer.

## 12. Risiken und Guardrails

### Nicht Zed kopieren

Der Zed-Checkout ist eine Verhaltens- und Architekturquelle. Der Zed-Editor-Crate ist als GPL-3.0-lizenziert ausgewiesen; außerdem verbietet die Labonair-Architektur, Zed-Quelltext als Implementierung zu übernehmen. Es sollten deshalb nur Konzepte, öffentliche Protokollideen und beobachtbare UX-Muster unabhängig nachgebaut werden.[^zedcargo]

### Foundation-Reihenfolge einhalten

Multi-Cursor ohne Anchors, Folding ohne Display-Koordinaten oder LSP-Diagnosen ohne immutable Snapshots führt fast sicher zu Reparaturarbeit. Die Reihenfolge Buffer → Transaction → Display → Language Service ist daher eine technische Abhängigkeit, keine reine Projektmanagement-Präferenz.

### Settings nicht als Feature-Container missbrauchen

Ein Setting beschreibt einen Wert. LSP-Prozess, Host, Theme-Registry, Git-Status und Editor-State bleiben in ihren Ownern. Neue Settings erst hinzufügen, wenn es einen sichtbaren Consumer, Scope, Default, Migration und Test gibt.

### Remote nicht als nachträgliche Sonderbedingung behandeln

Remote-Dateien existieren bereits. Contracts für File-Events, Search und Language Services sollten deshalb von Anfang an `Local` und `Remote` als Adapteroptionen zulassen, ohne dass jeder UI-Handler beide Welten selbst auseinanderhalten muss.

### UI-Polish nicht mit dauerhaftem Chrome verwechseln

Zeds Stärke kommt häufig aus Zustandsklarheit, Fokus, Dichte, Gutter-/Popover-Layern und sauberem Scrollen, nicht aus immer mehr sichtbaren Buttons. Jede neue permanente Shell-Fläche benötigt weiterhin die bestehende ADR-/Product-Surface-Prüfung.

## 13. Ergänzende Paritätskontrolle mit `reference-src`

Der eingefrorene Vorgänger enthält einige Funktionen, die im aktuellen Rust-Editor nicht verloren gehen sollten, obwohl die Umsetzung nicht übernommen werden darf:

- CodeMirror `basicSetup` brachte bereits History, Fold-Gutter, `indentOnInput`, Bracket Matching, Close Brackets, Autocompletion, Active-Line-/Selection-Matches und Search-Keymap.[^refextensions]
- `EditorPane` hatte Settings für Auto-Save und Delay, Bracket Matching, Outline, Format on Save, Indentation Guides, Font/Line Height, Trim Trailing Whitespace und Final Newline.[^refpane]
- Save führte optional Formatierung, Whitespace-Trim und Final-Newline-Transform aus.[^refformat]
- Die Toolbar zeigte Sprache mit Override, Dirty-Status, Cursorposition, Selection Stats sowie Display-/Editing-Schalter für Wrap, Line Numbers, Indent Guides, Outline, Bracket Matching und Format on Save.[^reftoolbar]
- Der Vorgänger hatte ein Outline-Panel mit Symboltyp, Hierarchie, Zeile und leerem Zustand „No symbols found“.[^refoutline]
- Der Language Resolver lud zusätzlich SQL, PHP, XML, Markdown, Ruby, Swift, Kotlin und Dockerfile-Modi; der Formatter unterstützte mehrere JS/TS/JSON/CSS/Markdown/HTML/YAML-Parser.[^reflanguage][^refformatter]

Diese Liste ist für Labonair besonders wichtig: Ein Teil der gewünschten Ziel-Funktionalität ist keine abstrakte Zed-Idee, sondern bereits einmal als Produktworkflow definiert worden. Sie sollte bei der Migration gegen Regressionen geschützt werden, aber mit den aktuellen nativen Ownership-/UI-kit-/Notification-Regeln neu implementiert werden.

## Quellen und Belege

### Labonair Rust und normative Regeln

- [Normative Dokumentübersicht](../../docs/README.md#L1) und [Feature-Lifecycle](../../docs/feature-lifecycle.md#L24).
- [Editor-Capability-Matrix](../../docs/capabilities.md#L20) und [aktueller Editor-/View-Ownership-Audit](../../docs/rework-roadmap.md#L219).
- [TextBuffer](../../crates/editor/src/buffer.rs#L1), [Document](../../crates/editor/src/document.rs#L28), [History](../../crates/editor/src/history.rs#L1).
- [Language-Erkennung und Grammar-Abdeckung](../../crates/editor/src/language.rs#L1), [Tree-sitter-Highlighter](../../crates/editor/src/syntax.rs#L1), [Symbols](../../crates/editor/src/symbols.rs#L1), [Search Core](../../crates/editor/src/search.rs#L1), [Vim](../../crates/editor/src/vim.rs#L1).
- [EditorView als GPUI-Adapter](../../crates/workspace/src/views/editor.rs#L1), [SearchOverlay](../../crates/workspace/src/search_overlay.rs#L1), [Editor-Commands](../../crates/editor/src/command_provider.rs#L1).
- [Dateiladen, Binary/Too-Large und atomisches Speichern](../../crates/filesystem/src/file.rs#L167), [Projekt-Grep](../../crates/filesystem/src/grep.rs#L43).
- [Tab-Modell](../../crates/workspace/src/tabs.rs#L26), [Editor-Tab-Öffnen/Peek](../../crates/workspace/src/workspace.rs#L1447), [Editor-Renderpfad](../../crates/workspace/src/workspace.rs#L4551), [Session-Snapshot](../../crates/workspace/src/session.rs#L31), [Remote-Edit](../../crates/workspace/src/workspace.rs#L2945).
- [Pane-Tree](../../crates/workspace/src/pane_group.rs#L1), [Explorer](../../crates/panel-explorer/src/panel_explorer.rs#L1), [SCM](../../crates/panel-scm/src/panel_scm.rs#L1266), [Project-Diff](../../crates/workspace/src/views/project_diff.rs#L1), [Editor-Settings](../../crates/settings-content/src/editor.rs#L13).

### Zed-Referenz

- [Editor crate overview](../../zed-refrence/zed/crates/editor/src/editor.rs#L1), [Editor state](../../zed-refrence/zed/crates/editor/src/editor.rs#L948), [Editor init/registration](../../zed-refrence/zed/crates/editor/src/editor.rs#L359).
- [DisplayMap model and layers](../../zed-refrence/zed/crates/editor/src/display_map.rs#L1), [DisplayMap responsibilities](../../zed-refrence/zed/crates/editor/src/display_map.rs#L210).
- [Language Buffer](../../zed-refrence/zed/crates/language/src/buffer.rs#L79), [Buffer state](../../zed-refrence/zed/crates/language/src/buffer.rs#L99), [Buffer events](../../zed-refrence/zed/crates/language/src/buffer.rs#L316).
- [Editor actions](../../zed-refrence/zed/crates/editor/src/actions.rs#L1) and [action registration](../../zed-refrence/zed/crates/editor/src/actions.rs#L409).
- [EditorElement](../../zed-refrence/zed/crates/editor/src/element.rs#L248) with [layout/paint responsibilities](../../zed-refrence/zed/crates/editor/src/element.rs#L1374).
- [EditorSettings fields](../../zed-refrence/zed/crates/editor/src/editor_settings.rs#L19) and [SearchSettings](../../zed-refrence/zed/crates/editor/src/editor_settings.rs#L190).
- [LSP Store local/remote/unified model](../../zed-refrence/zed/crates/project/src/lsp_store.rs#L1), [project search](../../zed-refrence/zed/crates/project/src/project_search.rs#L38), [search query](../../zed-refrence/zed/crates/project/src/search.rs#L76), [outline](../../zed-refrence/zed/crates/language/src/outline.rs#L8).
- [Completion](../../zed-refrence/zed/crates/editor/src/completions.rs#L1), [Diagnostics](../../zed-refrence/zed/crates/editor/src/diagnostics.rs#L1), [Folding ranges](../../zed-refrence/zed/crates/editor/src/folding_ranges.rs#L9), [Inlays](../../zed-refrence/zed/crates/editor/src/inlays.rs#L1), [Git/diff/blame/review](../../zed-refrence/zed/crates/editor/src/git.rs#L25), [editor persistence](../../zed-refrence/zed/crates/editor/src/persistence.rs#L20).
- [Zed editor crate manifest and license](../../zed-refrence/zed/crates/editor/Cargo.toml#L1).

### Eingefrorener Labonair-Vorgänger

- [EditorPane settings and lifecycle](../../reference-src/src/modules/editor/EditorPane.tsx#L95), [save transforms/autosave](../../reference-src/src/modules/editor/EditorPane.tsx#L281).
- [CodeMirror shared extensions](../../reference-src/src/modules/editor/lib/extensions.ts#L59), [EditorToolbar](../../reference-src/src/modules/editor/EditorToolbar.tsx#L84), [OutlinePanel](../../reference-src/src/modules/editor/OutlinePanel.tsx#L93).
- [Language resolver](../../reference-src/src/modules/editor/lib/languageResolver.ts#L6), [Formatter](../../reference-src/src/modules/editor/lib/formatter.ts#L1), [Auto-Save](../../reference-src/src/modules/editor/lib/useAutoSave.ts#L1).

[^arch]: [docs/rework-roadmap.md](../../docs/rework-roadmap.md#L219) beschreibt `labonair-editor` als Owner der Editor-Algorithmen und die Workspace-View als GPUI-Adapter; der Audit nennt die Extraktion weiterhin laufend.
[^docs]: [docs/README.md](../../docs/README.md#L53) definiert die Reihenfolge der maßgeblichen Quellen; [repository-layout.md](../../docs/repository-layout.md#L35) definiert Capability- und UI-Sibling-Grenzen.
[^buffer]: [crates/editor/src/buffer.rs](../../crates/editor/src/buffer.rs#L1).
[^document]: [crates/editor/src/document.rs](../../crates/editor/src/document.rs#L28).
[^history]: [crates/editor/src/history.rs](../../crates/editor/src/history.rs#L1).
[^file]: [crates/filesystem/src/file.rs](../../crates/filesystem/src/file.rs#L167).
[^editorview]: [crates/workspace/src/views/editor.rs](../../crates/workspace/src/views/editor.rs#L1).
[^language]: [crates/editor/src/language.rs](../../crates/editor/src/language.rs#L9).
[^syntax]: [crates/editor/src/syntax.rs](../../crates/editor/src/syntax.rs#L1).
[^symbols]: [crates/editor/src/symbols.rs](../../crates/editor/src/symbols.rs#L40).
[^search]: [crates/editor/src/search.rs](../../crates/editor/src/search.rs#L1).
[^searchoverlay]: [crates/workspace/src/search_overlay.rs](../../crates/workspace/src/search_overlay.rs#L1).
[^editorsearch]: [crates/workspace/src/views/editor.rs](../../crates/workspace/src/views/editor.rs#L49).
[^grep]: [crates/filesystem/src/grep.rs](../../crates/filesystem/src/grep.rs#L43).
[^vim]: [crates/editor/src/vim.rs](../../crates/editor/src/vim.rs#L1).
[^tabs]: [crates/workspace/src/tabs.rs](../../crates/workspace/src/tabs.rs#L26).
[^workspace]: [crates/workspace/src/workspace.rs](../../crates/workspace/src/workspace.rs#L4551).
[^panes]: [crates/workspace/src/pane_group.rs](../../crates/workspace/src/pane_group.rs#L1).
[^session]: [crates/workspace/src/session.rs](../../crates/workspace/src/session.rs#L31).
[^remote]: [crates/workspace/src/workspace.rs](../../crates/workspace/src/workspace.rs#L2945).
[^explorer]: [crates/panel-explorer/src/panel_explorer.rs](../../crates/panel-explorer/src/panel_explorer.rs#L1).
[^scm]: [crates/panel-scm/src/panel_scm.rs](../../crates/panel-scm/src/panel_scm.rs#L1266).
[^projectdiff]: [crates/workspace/src/views/project_diff.rs](../../crates/workspace/src/views/project_diff.rs#L1).
[^zededitor]: [Zed crates/editor/src/editor.rs](../../zed-refrence/zed/crates/editor/src/editor.rs#L1).
[^displaymap]: [Zed crates/editor/src/display_map.rs](../../zed-refrence/zed/crates/editor/src/display_map.rs#L1).
[^zedbuffer]: [Zed crates/language/src/buffer.rs](../../zed-refrence/zed/crates/language/src/buffer.rs#L99).
[^zedactions]: [Zed crates/editor/src/actions.rs](../../zed-refrence/zed/crates/editor/src/actions.rs#L409).
[^zedelement]: [Zed crates/editor/src/element.rs](../../zed-refrence/zed/crates/editor/src/element.rs#L1374).
[^lsp]: [Zed crates/project/src/lsp_store.rs](../../zed-refrence/zed/crates/project/src/lsp_store.rs#L1).
[^zedprojectsearch]: [Zed crates/project/src/project_search.rs](../../zed-refrence/zed/crates/project/src/project_search.rs#L38).
[^zedsearch]: [Zed crates/project/src/search.rs](../../zed-refrence/zed/crates/project/src/search.rs#L76).
[^zedoutline]: [Zed crates/language/src/outline.rs](../../zed-refrence/zed/crates/language/src/outline.rs#L8).
[^zedsettings]: [Zed crates/editor/src/editor_settings.rs](../../zed-refrence/zed/crates/editor/src/editor_settings.rs#L19).
[^zedpersistence]: [Zed crates/editor/src/persistence.rs](../../zed-refrence/zed/crates/editor/src/persistence.rs#L20).
[^labonairsettings]: [crates/settings-content/src/editor.rs](../../crates/settings-content/src/editor.rs#L13).
[^editorcommands]: [crates/editor/src/command_provider.rs](../../crates/editor/src/command_provider.rs#L28).
[^lifecycle]: [docs/feature-lifecycle.md](../../docs/feature-lifecycle.md#L24).
[^zedcargo]: [Zed editor Cargo.toml](../../zed-refrence/zed/crates/editor/Cargo.toml#L1).
[^refextensions]: [reference-src Editor extensions](../../reference-src/src/modules/editor/lib/extensions.ts#L59).
[^refpane]: [reference-src EditorPane](../../reference-src/src/modules/editor/EditorPane.tsx#L121).
[^refformat]: [reference-src EditorPane save transforms](../../reference-src/src/modules/editor/EditorPane.tsx#L281).
[^reftoolbar]: [reference-src EditorToolbar](../../reference-src/src/modules/editor/EditorToolbar.tsx#L95).
[^refoutline]: [reference-src OutlinePanel](../../reference-src/src/modules/editor/OutlinePanel.tsx#L93).
[^reflanguage]: [reference-src language resolver](../../reference-src/src/modules/editor/lib/languageResolver.ts#L14).
[^refformatter]: [reference-src formatter](../../reference-src/src/modules/editor/lib/formatter.ts#L8).

## 8. Current implementation status

The bounded native rework has now implemented the foundation and daily editing
workflow described by this report while preserving the documented ownership
rules:

- `labonair-editor` owns the rope buffer, anchors, immutable snapshots,
  transactions, selection sets, history, Vim state, display mapping, folding,
  search/replace, project-search and File Finder contracts, local language
  services, session persistence, and save policies.
- The Workspace view is the GPUI composition adapter. It supplies filesystem
  and Git bridges, renders the Editor surface, and hosts the existing modal
  layer; it does not become a second editor state owner.
- The editor surface now includes multi-selection editing, internal splits,
  soft-wrap, whitespace/rulers/relative numbers, folding, outline, sticky
  context, minimap, scrollbars, diagnostics/semantic decorations, completion,
  signature help, hover, rename/code actions, formatting, recovery, and Git
  gutter/project-diff handoff.
- Search supports literal/regex/multiline/smart-case/whole-word and typed
  replace actions. Project search and Open File use bounded asynchronous
  adapters with cancellation, generation checks, root validation, loading,
  empty, error, and truncated states.
- The latest polish adds truthful path/symbol breadcrumbs with long-path
  truncation, focus/hover/tooltip states, and existing symbol navigation.

The following remain intentionally outside this pass: AI editor features,
remote language services, collaboration, extension/marketplace hosting,
debugger/tasks, remote theme downloads, and provider-backed Inlay Hints or
Code Lens. They require separate product workflows and ownership contracts;
they are documented as deferred rather than represented by fake UI data.

Focused verification recorded by the implementation tasks reached 161 Editor
unit tests, 3 language-runtime tests, 7 LSP-process tests, and 140 Workspace
tests, with formatting, targeted Clippy, and diff checks passing. Native visual
acceptance remains part of R07-001 and is handed off in the repository-root
[`testing-plan.md`](../../testing-plan.md).
