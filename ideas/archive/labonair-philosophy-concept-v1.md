# Labonair Philosophy & Evolution Concept

> **Status:** Draft / Ideen-Konzept
> **Autor:** Niklas + Claude Code
> **Erstellt:** 2026-09-02
> **Gilt ab:** Nach Erreichen der Feature-Parität (Phase 15 abgeschlossen)

Dieses Dokument beschreibt die strategische Weiterentwicklung von Labonair nach dem Erreichen der Feature-Parität mit der Referenz-App. Grundlage ist die **Zed-Philosophie**: so simple und performant wie möglich, maximale User-Effizienz, keine überflüssige UI, feste Orte für bestimmte Funktionen — aber Modularität und Individualisierung für Personalisierung.

---

## Leitprinzipien (Zed-Philosophie angewandt auf Labonair)

1. **Simple by default** — So wenig UI-Surface wie nötig. Alles was nicht aktiv gebraucht wird, verschwindet.
2. **Fast always** — KeineOperation blockiert. Alles async, alles non-blocking.
3. **Intentional placement** — Jede Funktion hat EINEN klaren Ort. Keine versteckten Menüs, keine redundanten Zugangswege.
4. **Progressive disclosure** — Basic-Nutzer sehen nur das Nötige. Power-User finden erweiterte Features auf Anfrage.
5. **Keyboard-first** — Alles per Keyboard erreichbar. Mouse ist optional, nie Required.
6. **Modular, not monolithic** — Features sind unabhängig nutzbar, aber folgen einem einheitlichen Pattern.

---

## 1. Host Manager → Command Palette Integration

### Ausgangslage
Der Host Manager ist aktuell ein eigenes Panel/Modal mit vollständiger CRUD-Oberfläche (Hinzufügen, Bearbeiten, Löschen, Import/Export, Verbinden). Er ist untukgebracht, aber für den daily-use Overhead: der User muss ein separates Panel öffnen, navigieren, und schließen.

### Ziel
Der Host Manager wird in die **Command Palette** integriert — der zentralen Navigations-Hub der App. Der Host Manager als separates Panel wird aufgelöst. Die Palette wird zum Hauptort für alle Host-bezogenen Aktionen.

### Veränderungen

#### A) Command Palette erweitern
Die Palette bekommt eine neue Kategorie `"Hosts"` mit folgenden Einträgen:

| Eintrag | Aktion | Verfügbare Aliase |
|---|---|---|
| `Connect to host...` | Zeigt Host-Liste, einlicken = verbinden | `ssh`, `connect` |
| `Disconnect host` | Trennt aktive Verbindung | `disconnect` |
| `Add new host` | Öffnet Host-Editor Modal | `new-host`, `add-host` |
| `Edit host...` | Zeigt Host-Liste, einlicken = bearbeiten | `edit-host` |
| `Remove host...` | Host löschen (mit Bestätigung) | `rm-host`, `delete-host` |
| `Import hosts...` | SSH-Config Import | `import` |
| `Export hosts...` | Hosts exportieren | `export` |
| `Host settings...` | Host-spezifische Optionen (SSH-Keys, Jump-Hosts) | — |

#### B) Hotkey zum direkten Host-Connect
Ein dedizierter Hotkey (z.B. `Cmd+Shift+S` oder konfigurierbar) öffnet die Palette **direkt im Host-Connect-Modus** — keine manuelle Suche nötig. Zeigt sofort die Host-Liste.

#### C) Host-Editor Modal (statt Panel)
Für CRUD-Operationen die mehr Platz brauchen (SSH-Config-Details, Jump-Host-Chain, Key-Auswahl) — ein **kompaktes Modal**, erreichbar über Palette-Eintrag. Kein permanentes Panel mehr.

### User-Flow (vorher/nachher)

```
VORHER:
  Panel öffnen → Host suchen → Klick auf "Connect" → Panel schließen
  
NACHHER:
  Cmd+Shift+S → Host-Name tippen → Enter → Verbunden
  (0 Klicks, 2 Tastendrücke)
```

### Technische Anforderungen
- Command Palette bekommt eine `HostCategory` mit dynamischen Einträgen (nur verbundene Hosts bei "Disconnect")
- Host-Editor wird als eigenes Modal (wie Settings) implementiert
- `HostManagerView` wird aufgelöst — nur noch das Modal + Palette-Einträge bleiben
- Hotkey `Cmd+Shift+S` → `palette.run("connect")` oder `palette.run("Connect to host...")`

---

## 2. Kein Pflicht-Tab (Empty State)

### Ausgangslage
Aktuell muss mindestens ein Tab/Pane immer offen sein (Terminal, Host-Manager, oder Editor). Der User kann die App nicht "leer" lassen. Das erzeugt unnötigen Anfangs-Overhead.

### Ziel
Die App darf **vollständig leer** sein — kein Tab, kein Terminal, kein Panel. Nur die Hintergrundfarbe. Der User entscheidet aktiv, was er öffnen möchte.

### Veränderungen

#### A) Empty State UI
Wenn keine Tabs offen sind:
```
┌─────────────────────────────────────────┐
│                                         │
│                                         │
│         [App-Hintergrundfarbe]          │
│                                         │
│    Double-click to open a terminal      │
│    or press Cmd+N for a new tab         │
│                                         │
│                                         │
└─────────────────────────────────────────┘
```
- Kein Welcome-Screen, kein Onboarding — einfach leer
- Subtiler Hinweis (niedrige Opazität, verschwindet nach dem ersten Tab-Öffnen)
- Doppelklick auf leere Fläche → lokales Terminal öffnen

#### B) Workspace-Änderungen
- `Workspace` erlaubt `tabs.len() == 0` (kein Mindest-Tab)
- Tab-Leiste bleibt sichtbar aber leer (kein "+" Zwang)
- Statusbar zeigt "No active sessions" oder ist ebenfalls leer
- Header bleibt sichtbar (App-Name, CMDPalette-Zugang, etc.)

#### C) Session-Restore
- Wenn bei letzter Sitzung alles geschlossen war → App startet leer
- Kein erzwungenes "Fallback-Terminal"

#### D) Schnellzugriff
- `Doppelklick` auf leere Workspace-Fläche → `open_local_terminal()`
- `Cmd+T` oder `Cmd+N` → neues lokales Terminal
- Alle bestehenden Shortcuts funktionieren weiter (Cmd+1-9 für Tabs, etc.)

### Technische Anforderungen
- `Workspace::new` akzeptiert leere Tab-Liste
- `render` rendert Empty-State wenn `tabs.is_empty()`
- `AppShell` rendert kein Fallback-Terminal bei leerer Sitzung
- Session-Restore: `RestoreAction::Empty` (oder einfach kein Snapshot)
- Doppelklick-Handler auf Workspace-Body

---

## 3. Projekt-System (Bookmarks → Projects)

### Ausgangslage
Das aktuelle Bookmark-System (`PathBookmark`) speichert Ordner-Pfade mit optionalen Labels und Host-Zuordnung. Es ist passiv — ein Lesezeichen sagt nicht "was gehört zusammen".

### Ziel
Ein **projekt-orientiertes System** das:
- Verzeichnisse als Projekte definiert
- Projekte automatisch von freien Terminals unterscheidet
- Tabs/Sessions einem Projekt zuordnet
- Projekt-Wechsel die zugehörigen Tabs fokussiert/öffnet

### Veränderungen

#### A) Projekt-Datenmodell
```rust
struct Project {
    id: Uuid,
    name: String,                    // "Labonair", "mein-backend", etc.
    root_path: PathBuf,              // lokaler Root
    host_id: Option<Uuid>,           // Remote-Host (None = lokal)
    created_at: DateTime<Utc>,
    last_used_at: DateTime<Utc>,
    associated_tabs: Vec<TabId>,     // welche Tabs gehören zum Projekt
    git_branch: Option<String>,      //letzter bekannter Branch (Info)
}
```

#### B) Projekt- vs. Frei-Terminal
| Eigenschaft | Projekt-Terminal | Frei-Terminal |
|---|---|---|
| Wurde gestartet aus | Projekt-Kontext (Project-Panel oder Projektauswahl) | Doppelklick, Cmd+T, Command Palette |
| Arbeitet in | `project.root_path` (oder Remote-Pfad) | Beliebiger Pfad |
| Erscheint in | Projekt-Tab-Gruppe | "Unassociated"-Bereich |
| Wird geschlossen bei | Projekt-Wechsel (optional) | App-Schließen |
| Name | Projekt-Name + "Terminal" | "Terminal 1", "Terminal 2", etc. |

#### C) Projekt-Panel / -Overlay
Ein **Project-Overlay** (ähnlich wie Bookmarks, aber mächtiger):
```
┌─ Projects ──────────────────────────┐
│                                     │
│  ● Labonair (local)                 │
│    └── Terminal (bash)  [active]    │
│    └── Editor: src/main.rs          │
│                                     │
│  ○ backend-api (ssh: production)    │
│    (not active)                     │
│                                     │
│  ○ mein-spiel (local)              │
│    └── Terminal (zsh)               │
│                                     │
│  ── Unassociated ─────────────────  │
│    Terminal 3 (bash)  [active]      │
│                                     │
│  [+ New Project]                    │
└─────────────────────────────────────┘
```
- Klick auf Projekt → fokussiert/öffnet zugehörige Tabs
- Rechtsklick → Rename, Remove, Set Root, Associate Tab
- **"New Project"** = Pfad auswählen + Name vergeben

#### D) Projekt-Erkennung (Auto-Detection)
Automatische Heuristik um Projekte von freien Terminals zu unterscheiden:

| Signal | Projekt | Frei |
|---|---|---|
| Pfad enthält `.git/` | Wahrscheinlich Projekt | — |
| Pfad enthält `package.json`/`Cargo.toml`/`go.mod` | Wahrscheinlich Projekt | — |
| Tab wurde aus Project-Overlay gestartet | Definitiv Projekt | — |
| Tab läuft in `~/` oder `/tmp` | — | Wahrscheinlich frei |
| Tab wurde per Doppelklick/Cmd+T geöffnet | — | Definitiv frei |

**Projekt-Vorschlag:** Wenn ein neues Terminal in einem Verzeichnis mit `.git` oder Manifest-Datei geöffnet wird → Mini-Toast: "This looks like a project. Create project 'my-backend'?" mit [Yes] [No] [Always for this path].

#### E) Migration bestehender Bookmarks
- `PathBookmark` → `Project` (automatisch, einmigration)
- Bestehende Bookmarks werden als Projekte importiert
- `bookmarks.json` → `projects.json`

### Technische Anforderungen
- Neues Modul `crates/backend/src/modules/projects/`
- `ProjectStore` (JSON-persistiert in `~/.config/labonair/projects.json`)
- `ProjectOverlay` (GPUI Entity, Bookmarks-Pattern)
- Tab-spezifische Projekt-Zuordnung (Tab merkt sich `project_id`)
- Explorer bekommt "Create Project from here" Kontextmenü
- Command Palette bekommt `"Switch to project..."` Eintrag
- Session-Restore: Projekt-Zuordnung wird mitgespeichert
- Migration: `bookmarks.json` → `projects.json` (einmalig, mit Backup)

---

## 4. AI überall (Inline + Tab)

### Ausgangslage
Die AI ist aktuell in einem Sidebar/Fenster gebunden. Der User muss dorthin navigieren, eine Frage stellen, und zurückkehren. Das ist ein Unterbrechungs-Flow.

### Ziel
AI ist ein **überall verfügbares Werkzeug**, kein Ort. Der User stellt Fragen wo er gerade ist — im Terminal, im Editor, im Composer — und bekommt Antworten inline ohne den Kontext zu verlieren.

### Die drei AI-Zugangswege

```
                    ┌─────────────────────┐
                    │    AI ACCESS MODEL   │
                    └─────────┬───────────┘
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
     ┌────────────┐  ┌────────────────┐  ┌──────────┐
     │  INLINE    │  │  CONTEXT MENU  │  │ AI TAB   │
     │  @"frage"  │  │  "Ask AI..."   │  │ (full)   │
     └────────────┘  └────────────────┘  └──────────┘
      Composer/       Editor/Terminal/     Komplexe
      Terminal        Explorer Selection   AI-Arbeit
```

### 4A) Inline AI — `@"frage"` Syntax

**Wo:** Im Composer (Chat-Input) und im Terminal-Input.
**Wie:** Der User tippt `@"Wie starte ich nginx?"` — das System erkennt die Syntax und verarbeitet die Frage.

**Zwei Antwort-Typen:**

| Typ | Erkennung | Aktion |
|---|---|---|
| **Message** | Frage endet mit `?` oder ist kein Command | Inline-Antwort wird unter der Eingabe angezeigt |
| **Command** | Frage beginnt mit "how to", "wie kann ich", "give me", "show me" + terminale Syntax erkannt | Command wird in die Eingabezeile eingefügt, User muss nur Enter drücken |

**Beispiele:**
```
User tippt:  @"Wie starte ich den nginx service?"
→ Message:   "sudo systemctl start nginx"
→ [Enter zum Ausführen]

User tippt:  @"Zeig mir alle laufenden Docker Container"
→ Command:   docker ps
→ [Enter zum Ausführen]

User tippt:  @"Was macht diese Funktion in Zeile 42?"
→ Message:   (Erklärung der Funktion)
→ [Kein Command, nur Info]
```

**UI-Flow:**
```
┌─ Composer ─────────────────────────────────┐
│ @"Wie starte ich nginx?"                   │
├────────────────────────────────────────────┤
│ 💡 sudo systemctl start nginx         [▶] │  ← Command-Vorschlag
│    [Enter to execute] [Tab to edit]        │
└────────────────────────────────────────────┘
```

Oder bei Message-Typ:
```
┌─ Composer ─────────────────────────────────┐
│ @"Was macht diese Funktion?"               │
├────────────────────────────────────────────┤
│ Diese Funktion initialisiert den DB-Pool   │
│ mit max. 10 Verbindungen und einem         │
│ Timeout von 30 Sekunden.                   │
└────────────────────────────────────────────┘
```

### 4B) Context Menu AI

**Wo:** Rechtsklick-Menu im Editor, Terminal, Explorer.
**Wie:** Auswahl → "Ask AI..." → Mini-Popup.

**Editor-Kontext:**
```
┌─ Editor Kontextmenu ──────────┐
│  Copy                        │
│  Paste                       │
│  ─────────────────────────── │
│  Ask AI: "Explain this code" │  ← neue Option
│  Ask AI: "Write tests for"   │
│  Ask AI: "Refactor this"     │
└──────────────────────────────┘
```

**Terminal-Kontext:**
```
┌─ Terminal Kontextmenu ─────────────┐
│  Copy Selection                   │
│  Paste                            │
│  ──────────────────────────────── │
│  Ask AI: "Explain this output"    │
│  Ask AI: "Fix this error"         │
│  Ask AI: "What does this mean?"   │
└───────────────────────────────────┘
```

**Mini-Popup (gemeinsam für alle Context-Aktionen):**
```
┌─ AI Quick Question ──────────────────────┐
│                                          │
│  Explain this code:                      │
│  ```rust                                 │
│  fn init_pool(max: u32) -> Pool {        │
│      // ...                              │
│  }                                       │
│  ```                                     │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │ Antwort wird hier angezeigt...     │  │
│  └────────────────────────────────────┘  │
│                                          │
│  [Copy] [Insert as Comment] [Close]      │
└──────────────────────────────────────────┘
```

### 4C) AI Tab (Vollständige AI-Arbeit)

**Wo:** Als normaler Tab (neben Terminal, Editor, etc.).
**Wie:** `Cmd+Shift+A` oder Command Palette "Open AI Tab".

**Design:**
```
┌─ AI Tab ────────────────────────────────────────────────┐
│  [Chat]  [Composer]  [CIEL]              ● AI Tab       │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─ Messages ─────────────────────────────────────────┐ │
│  │ User: Erkläre mir die Architektur von Labonair    │ │
│  │ AI:  Labonair besteht aus 5 Hauptmodulen...       │ │
│  │                                                    │ │
│  │ User: Schreibe eine Funktion die X macht          │ │
│  │ AI:  ```rust                                      │ │
│  │      fn do_x() { ... }                            │ │
│  │      ```                                          │ │
│  │      [Copy] [Insert in Editor] [Run]              │ │
│  └────────────────────────────────────────────────────┘ │
│                                                         │
│  ┌─ Input ───────────────────────────────────────────┐  │
│  │ Frag mich anything...                    [Send]   │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Sub-Tabs im AI Tab:**
- **Chat** — Freie Konversation, Fragen, Erklärungen
- **Composer** — Code-Generierung mit Vorschau & Insert
- **CIEL** — (Optional) Agentic Companion, wenn integriert

**Context-Awareness:** Der AI Tab kennt den aktiven Kontext:
- Wenn Terminal aktiv → kann letzte Befehle/Ausgabe als Context senden
- Wenn Editor aktiv → kann aktuelle Datei/Selektion als Context senden
- Wenn Explorer aktiv → kann ausgewählte Datei als Context senden

### 4D) Composer-Überarbeitung

**Der AI Input-Button wird entfernt.** Die AI ist jetzt überall — kein dedizierter Button nötig.

**Neue Composer-Features:**
- `@"frage"` Syntax (siehe 4A) — erkennt AI-Anfragen automatisch
- `@dateiname` für Datei-Referenz (Context)
- `@host:name` für Remote-Kontext
- Normale Nachrichten bleiben unverändert

### 4E) AI-Nachrichten-Typen

| Typ | UI | Nutzung |
|---|---|---|
| `Message` | Inline-Text (Mini-Popup oder Tab) | Erklärungen, Fragen, Informationen |
| `Command` | Inline-Code mit "Execute"-Button | Terminal-Befehle, Skripte, Codestücke |

Der AI erkennt automatisch den Typ:
- Frage mit `?` oder "was/wie/warum" → Message
- Frage mit "zeig/gib/mach/run" + terminale Keywords → Command
- Unklar → Message (Default)

### Technische Anforderungen
- **AI Provider Layer** (`crates/ai/`) — erweitert um inline-message und command-generation
- **Composer-Syntax-Parser** — erkennt `@"..."` und `@datei` Patterns
- **Mini-Popup Component** — GPUI Entity für Inline-AI-Antworten
- **Context-Menu AI** — Erweiterung der bestehenden Kontextmenüs
- **AI Tab** — neuer Tab-Typ im Workspace
- **CIEL-Integration** —Brücke zum externen CIEL-Projekt (API-Definition pending)
- **Message Queue** — AI-Antworten werden asynchron geladen, UI bleibt responsive

---

## 5. Optimierungen, Settings, Editor, GitHub & CLI

### 5A) Settings-Überarbeitung

**Ziel:** Settings sollen übersichtlicher, durchsuchbarer und logischer kategorisiert sein.

**Veränderungen:**
- **Kategorien klarer strukturieren** ( Appearance | Terminal | Editor | AI | Hosts | Projects | Shortcuts | Advanced )
- **Suchfunktion** in Settings (filtert alle Felder)
- **Import/Export** der gesamten Settings-Konfiguration
- **Reset to Defaults** pro Kategorie (nicht nur global)
- **Live-Preview** wo möglich (Theme-Wechsel, Font-Größe, etc.)
- **Zusammenfassung** am Anfang (welche Settings wurden geändert vs. Default)

### 5B) Editor-Verbesserungen

**Ziel:** Editor-Qualität schrittweise auf Zed-Niveau bringen.

**Priorisierte Verbesserungen:**
1. **Multi-Cursor** — `Alt+Click` für zusätzliche Cursors, `Cmd+D` für Next-Occurrence
2. **Better Search & Replace** — Regex-Support, Multi-File Search, Replace Preview
3. **Minimap** (optional) — Überblick über die gesamte Datei
4. **Code-Folding** — Baumstruktur-basiertes Falten via TreeSitter
5. **Bracket-Matching** visuell verbessert (aktuell basisch)
6. **Auto-Indent** verbessert (kontextsabhängig)
7. **LSP-Integration** — Completion, Go-to-Definition, Find-References (wenn CIEL-Integration publiziert wird)

### 5C) GitHub-Funktionen

**Ziel:** GitHub-Operationen direkt aus Labonair, ohne Terminal.

**Features:**
- **PR-Review** — PRs anzeigen, Comments lesen/schreiben, Approve/Request Changes
- **Issues** — Issues anzeigen, kommentieren, Labels setzen, Close/Open
- **Actions** — CI/CD Runs anzeigen, Logs lesen, Re-Trigger
- **Releases** — Releases anzeigen, Notes lesen, Assets herunterladen
- **Blame** — Editor-Integration: Zeile → Git-Blame (wer hat diese Zeile geändert?)
- **Code-Review-Modus** — Editor zeigt PR-Diffs inline (wie Zed's multibuffer)

**Integration mit bestehendem Git-Panel:**
```
┌─ Git Panel ──────────────────┐
│  Changes                     │
│  Branches                    │
│  Stash                       │
│  ─────────────────────────── │
│  GitHub                      │  ← neue Sektion
│    Pull Requests (3)         │
│    Issues (12)               │
│    Actions (2 running)       │
└──────────────────────────────┘
```

### 5D) Labonair CLI

**Ziel:** Labonair über das Terminal steuern — für Power-User und Automation.

**Befehle:**
```bash
# App steuern
labonair open                    # App starten
labonair close                   # App schließen
labonair focus                   # App fokussieren

# Sessions
labonair ssh <host>              # SSH-Verbindung öffnen
labonair terminal                # Neues lokales Terminal
labonair terminal --project <p>  # Terminal in Projekt-Kontext

# Projects
labonair project list            # Alle Projekte anzeigen
labonair project switch <name>   # Projekt wechseln
labonair project create <path>   # Neues Projekt erstellen

# Scripting
labonair exec <command>          # Befehl ausführen (ohne UI)
labonair batch <script.json>     # Batch-Operationen

# Info
labonair status                  # App-Status (Verbundene Hosts, etc.)
labonair config show             # Aktuelle Konfiguration
labonair --version               # Version
```

**Implementierung:**
- CLI Binary: `crates/cli/` (eigenes Crate)
- Kommunikation mit laufender App via Unix Socket / Named Pipe
- Fallback: Wenn App nicht läuft → starten mit den gegebenen Argumenten
- JSON-Output für Scripting (`--json` Flag)

### 5E) Allgemeine Kleinoptimierungen

| Bereich | Optimierung |
|---|---|
| **Tab-Schließen** | `Cmd+W` schließt Tab, nicht App (aktuell manchmal App) |
| **Tab-Benennung** | Automatisch aus Dateiname/Host/Pfad, editierbar per Doppelklick |
| **Notification-System** | Toasts mit Undo-Funktion wo möglich |
| **Clipboard** | `Cmd+Shift+C` = Copy without formatting |
| **File-Associations** | Dateien aus Explorer im passenden Tab öffnen (Editor vs. Preview) |
| **Keyboard-Overlay** | `Cmd+/` zeigt alle Shortcuts als Overlay |
| **Idle-Dimming** | Tabs die lange nicht genutzt wurden, werden visuell zurückgenommen |
| **Theme-Auswahl** | Quick-Theme-Switcher im Header (Zed-Style) |

---

## Umsetzungs-Reihenfolge (nach Feature-Parität)

```
Phase 16: Empty State + Kein Pflicht-Tab
           (klein, sofort nutzbar, keine Breaking Changes)
           
Phase 17: Command Palette Überarbeitung + Host Manager Integration
           (größere UI-Änderung, aber klar strukturiert)
           
Phase 18: Projekt-System (Bookmarks → Projects)
           (Datenmodell-Änderung, Migration, UI)

Phase 19: AI everywhere (@" composer + context-menu + AI tab)
           (größte Änderung, multi-step)

Phase 20: Settings/Editor/GitHub polish + CLI
           (Hausaufgaben, iterativ)
```

---

## Offene Fragen

1. **CIEL-Integration:** Wie tight soll die Kopplung sein? Eigener Subprocess? Library? Nur API?
2. **CLI Socket:** Unix Socket (macOS/Linux) oder Named Pipe (Windows-kompatibel)?
3. **Projekt-Auto-Detection:** Soll die App aktiv im Hintergrund scannen oder nur bei Tab-Öffnung?
4. **AI Provider:** Soll die inline-AI dieselben Provider nutzen wie der AI Tab? (Ja, wahrscheinlich)
5. **Backwards-Kompatibilität:** Soll `bookmarks.json` beibehalten werden oder migriert?
6. **Multi-Window:** Wann wird Multi-Window unterstützt? (Relevant für CLI-Kommunikation)
