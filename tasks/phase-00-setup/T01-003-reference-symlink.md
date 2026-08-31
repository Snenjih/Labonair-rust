# T01-003: Referenz-Symlink zu Labonair erstellen

## Status
⏳ Pending

## Phase
0 — Projekt-Setup & Grundgerüst

## Abhängigkeiten
T01-001 (Cargo Workspace)

## Ziel
Einen Symlink von `reference/` im Labonair-rust-Projekt zu `../Labonair/` erstellen, damit alle Tasks einfachen Zugriff auf das Original haben.

## Anweisungen

### 1. Symlink erstellen

Unter macOS/Linux:

```bash
ln -s ../Labonair reference
```

Die Struktur sollte dann so aussehen:

```
Labonair-rust/
├── reference -> ../Labonair   ← Symlink
├── crates/
├── tasks/
└── ...
```

### 2. README.md erstellen

Erstelle `README.md` im Stammverzeichnis:

```markdown
# Labonair-rust

Native Rust-Portierung von Labonair (Tauri v2 + React → GPUI).

**Referenz**: `reference/` zeigt auf den Original-Source von Labonair.
Alle Design-Werte, funktionale Spezifikationen und Verhaltensweisen werden
aus dem Referenz-Code übernommen und in GPUI übersetzt.

## Status
See [tasks/ROADMAP.md](./tasks/ROADMAP.md)

## Entwicklung
cargo run    # Startet die App
cargo check  # Kompiliert ohne Run
cargo clippy # Lint
```

### 3. .gitignore erstellen

```gitignore
/target
/reference
.DS_Store
```

## Akzeptanzkriterien

- [ ] `reference/` zeigt auf den Labonair-Source
- [ ] README.md existiert mit Projekt-Beschreibung
- [ ] .gitignore schließt `/target` und `/reference` aus
- [ ] `ls reference/src/modules/` zeigt die 23 Module

## Notizen

- **Desktop-Only**: Labonair-rust ist eine Desktop-App. Es gibt kein Web-Output.
- **git clone**: Der `reference/` Ordner wird NICHT in das neue Repo committet (Symlink ist nicht versionierbar sinnvoll).

## Warnungen

- ⚠️ **Nicht versehentlich das Original ändern** — Der Symlink ist read-only Referenz. Keine Änderungen an `../Labonair/` während der Portierung.
- ⚠️ **Kein Git-Submodul**: Der Symlink ist kein Submodul. Bei ordnungsgemäßem Workflow wird er nicht in den Remote gepusht.

## Weiterführende Tasks

- Keine direkten Nachfolger. Projekt kann mit Phase 1 beginnen.
