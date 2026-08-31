# T01-003: Referenz-Kopie verifizieren & Projekt-Doku

## Status
✅ Done

## Phase
0 — Projekt-Setup & Grundgerüst

## Abhängigkeiten
T01-001 (Cargo Workspace)

## Ziel
Sicherstellen, dass die eingefrorene Referenz-Kopie `reference-src/` vollständig und unangetastet
im Repo liegt, und die Projekt-Doku (README) auf den Fork-Charakter ausrichten. **Es wird KEIN
Symlink und KEINE externe Anbindung erstellt** — dieser Fork ist vollständig standalone.

## Kontext
Labonair-rust ist ein **Hard Fork**. Der Original-Webapp-Source wurde einmalig nach
`reference-src/` kopiert. Ab jetzt gibt es keine Verbindung mehr zum Original-Repo — weder
Symlink, Submodul noch Pfad-Abhängigkeit. Alle Tasks lesen ausschließlich aus `reference-src/`.

## Anweisungen

### 1. Referenz-Kopie prüfen

Verifiziere, dass folgende Pfade unter `reference-src/` existieren und lesbar sind:

- `reference-src/src/styles/globals.css` — Design-Tokens (oklch)
- `reference-src/src/modules/` — die Frontend-Feature-Module (Verhaltens-Referenz)
- `reference-src/src-tauri/src/modules/` — die Rust-Backend-Module (Port-Vorlage), u.a.:
  `ssh`, `sftp`, `git`, `fs`, `pty`, `hosts`, `credentials`, `snippets`, `secrets.rs`,
  `themes`, `backgrounds`, `scrollback`, `settings`, `shell`, `terminal_exec`, `mcp`,
  `fonts`, `dock_menu.rs`, `menu_sync.rs`, `errors.rs`
- `reference-src/src-tauri/Cargo.toml` — maßgebliche Crate-Versionen (russh 0.62.2, russh-sftp 2.3.0,
  rusqlite 0.40, portable-pty 0.9, kein git2 → git-CLI)
- `reference-src/src-tauri/src/modules/pty/scripts/` — Shell-Init-Skripte (zshrc.zsh, bashrc.bash)

Falls etwas fehlt: stoppen und melden — die Referenz ist die einzige Quelle.

### 2. README.md schreiben

Erstelle `README.md` im Repo-Stamm:

```markdown
# Labonair-rust

Native Rust/GPUI-Rewrite von Labonair (vormals Tauri v2 + React 19) — ein **Hard Fork**.
Vollständig standalone, keine Verbindung zum Original-Repo.

## Referenz
`reference-src/` ist eine **eingefrorene, read-only Kopie** des Original-Webapps.
Sie ist die einzige Quelle für Design-Werte, Feature-Verhalten und zu portierende
Backend-Logik. Niemals editieren, niemals extern verlinken.

## Ziel
Volle Feature-Parität — alles was Labonair heute kann, läuft am Ende in purem Rust.

## Status
Siehe [tasks/ROADMAP.md](./tasks/ROADMAP.md) und [handshake.md](./handshake.md).

## Entwicklung
    cargo run      # App starten
    cargo check    # Kompilieren ohne Run
    cargo clippy --all-targets -- -D warnings
    cargo test
```

### 3. .gitignore prüfen

`.gitignore` ist bereits gesetzt (Rust `/target`, `.claude/`, OS-Kram). Sicherstellen, dass
`reference-src/` **nicht** ignoriert wird (es ist getrackt) und kein `/reference`-Symlink-Eintrag
mehr nötig ist.

## Akzeptanzkriterien

- [ ] Alle unter Anweisung 1 gelisteten Referenz-Pfade existieren in `reference-src/`
- [ ] `README.md` existiert im Repo-Stamm und beschreibt den Fork-/Referenz-Charakter
- [ ] `.gitignore` ignoriert `reference-src/` NICHT; kein externer Symlink im Repo
- [ ] `git grep -n "\.\./Labonair"` liefert 0 Treffer (keine Alt-Pfade mehr)
- [ ] `cargo check` weiterhin grün

## Notizen

- **Desktop-Only**: kein Web-Output.
- `reference-src/` wird mitgecheckt in den Remote — bewusst, es ist Teil des Forks.

## Warnungen

- ⚠️ **Kein Symlink, kein Submodul, keine Pfad-Dependency** zu einem externen Labonair-Repo.
  Der User will die Trennung explizit (Hard Fork).
- ⚠️ **`reference-src/` niemals ändern** — read-only Referenz.

## Weiterführende Tasks

- Keine direkten Nachfolger. Projekt kann mit Phase 1 fortfahren.
