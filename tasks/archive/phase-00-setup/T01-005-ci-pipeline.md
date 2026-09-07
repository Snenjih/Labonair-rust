# T01-005: CI-Pipeline (cargo check/clippy/test/fmt)

## Status
✅ Done

## Phase
0 — Projekt-Setup & Grundgerüst

## Abhängigkeiten
T01-001 (Cargo Workspace)

## Ziel
Eine GitHub-Actions-Pipeline, die bei jedem Push/PR auf `master` `cargo fmt --check`,
`cargo check`, `cargo clippy --all-targets -- -D warnings` und `cargo test` auf macOS ausführt.
Ersetzt die alten pnpm/vitest/react-Workflows (liegen jetzt unter
`reference-src/.github/workflows/`, nur zur Referenz).

## Kontext
Die alten Workflows (`ci.yml`, `codeql.yml`, `react-doctor.yml`, `release.yml`, `labeler.yml`,
`dependabot-audit.yml`) sind auf das Web-Toolchain zugeschnitten und wurden nach
`reference-src/.github/workflows/` verschoben. `.github/` im Repo-Stamm enthält noch die
Issue-/PR-Templates, CODEOWNERS, dependabot.yml, labeler.yml, release.yml — diese ggf. auf
Rust anpassen oder entfernen.

## Anweisungen
1. Neuer Workflow `.github/workflows/ci.yml`:
   - Trigger: `push` auf `master`, `pull_request`.
   - Runner: `macos-latest`.
   - Rust via `dtolnay/rust-toolchain@stable` + `Swatinem/rust-cache`.
   - Steps: `cargo fmt --all --check` · `cargo check --workspace --all-targets` ·
     `cargo clippy --workspace --all-targets -- -D warnings` · `cargo test --workspace`.
   - GPUI baut auf macOS gegen Metal — prüfen, ob zusätzliche System-Deps nötig sind
     (Xcode Command Line Tools sind auf `macos-latest` vorhanden).
2. `dependabot.yml` auf `package-ecosystem: cargo` umstellen (statt npm).
3. `.github/workflows/react-doctor.yml`-Äquivalent NICHT nachbauen.
4. `CODEOWNERS`, `labeler.yml`, `release.yml` auf die neue `crates/`-Struktur anpassen oder
   als „später" markieren (Release-Workflow gehört zu T15-004/T15-005).
5. Ein Linux-Job (`ubuntu-latest`, `cargo check` only) kann als `continue-on-error` ergänzt
   werden — Linux ist „später", soll aber nicht komplett brechen.

## Akzeptanzkriterien
- [x] `.github/workflows/ci.yml` existiert und triggert auf push/PR
- [ ] CI läuft grün auf einem Test-PR (fmt + check + clippy + test) — pending push (nur auf Nutzer-Wunsch)
- [x] `dependabot.yml` nutzt `cargo` (Workspace-Root `/`, kein npm mehr)
- [x] Keine verbleibenden pnpm/react-Workflows im Repo-Stamm-`.github/` (alte Workflows nur unter `reference-src/.github/workflows/`)
- [x] `cargo fmt --check` und `cargo clippy -- -D warnings` lokal grün

## Notizen
- Caching ist wichtig — GPUI + alacritty + russh sind große Dependency-Bäume.
- `build-grammars`-Feature (TreeSitter) braucht ggf. einen eigenen, selteneren Job.

## Warnungen
- ⚠️ macOS-Runner-Minuten zählen 10×. Job schlank halten (ein Job, gutes Caching).
- ⚠️ Erste CI-Runs können durch GPUI-Kompilierzeit lange dauern — Timeout großzügig setzen.
