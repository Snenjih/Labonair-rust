# T15-004: Verpackung & Release

## Status
✅ Done

## Phase
14 — Testing & Polish

## Abhängigkeiten
Alle vorherigen Phasen (App vollständig funktionsfähig)

## Ziel
Die App für den Distributions-/Release-Zustand vorbereiten: sinnvolle Verpackung für macOS (und perspektivisch Linux), Versions- und Build-Konfiguration, Code-Signierung/Notarisierung (sofern umsetzbar), sowie die Grundlage für Updates/Autoupdate und ein sauberes Release-Dokument. Dies schließt die Portierung ab und mündet in einem installierbaren Produkt.

## Kontext
Die Portierung ist dann "fertig", wenn die App lauffähig installiert werden kann und den Funktions-/Designumfang erfüllt. GPUI-basierte Apps erzeugen native Binaries; die Verteilung erfolgt plattformspezifisch (macOS: .app-Bundle, dmg; Linux: AppImage/Flatpak später). Anders als bei Tauri gibt es kein bundling-Tool wie `tauri bundle` — es ist ein eigenes Build-/Verpackungs-Setup nötig.

Dieser Task setzt das Release-Fundament und bereitet potenzielle zukünftige Auto-Updates vor (analog Labonair's Updater-Integration über Tauri-Updater, hier Rust-nativ).

## Anweisungen zur Umsetzung

1. **App-Bundle-Struktur (macOS).** Erstelle das macOS-.app-Bundle korrekt:
   - Info.plist (Identifier, Version, Name, Icon, Dokument-Typen, etc.).
   - App-Icon (wie Labonair/angepasst).
   - Frameworks/assets korrekt eingebettet (Fonts, Grammatiken).
   - Bundle-Layout, wie es von macOS erwartet wird.

2. **Build-Release-Prozess.** Erstelle ein reproducibles, einheitliches Build:
   - Release-Build mit Optimierungen (`--release`).
   - Korrekte Versionsnummer aus einer zentralen Quelle (Tauri hatte `tauri.conf.json`; hier zentral definieren).
   - Ein Skript (`build.sh` / `just`/Makefile), das den Release erzeugt.

3. **Code-Signierung/Notarisierung (macOS).** Bereite, falls Zertifikate vorhanden, das Signieren und Notarisieren vor:
   - Ad-hoc- oder Developer-Signierung.
   - Notarisierung mit Apple-Server (sofern Konto/Zertifikat vorliegt).
   - Dokumentiere das Verfahren, damit es in CI/Release ausgeführt werden kann.

4. **Autoupdate-Grundlage.** Bereite die Grundlage für Updates vor (falls gewünscht, analog Labonair's Updater):
   - Manifest/Update-Endpunkt definieren (welche Plattform, welche Version).
   - Mechanik zur Prüfung/Herunterladen/Anwenden von Updates (Rust-nativ).
   - UI für "Update verfügbar" (Dialog) — optional, wenn als sinnvoll.

5. **Linux-Verpackung (perspektivisch).** Lege (später) die Grundlage für Linux-Verteilung vor (AppImage/Flatpak), auch wenn der Fokus macOS ist. Kapsle die Plattform-Verpackung modew.

6. **Release-Dokument & Changelog.** Erstelle ein Release-Dokument:
   - Versionsnummer, Zielplattform.
   - Wie zu bauen (Build-Schritte).
   - Was verteilt wird (Artifakt).
   - Bekannte Einschränkungen/Unterschiede zur Original-App.

7. **End-to-End-Smoke-Test.** Erstelle einen End-to-End-Verify, dass das Release-Bundle die App korrekt startet und die Kernfunktionalität (Fenster öffnen, Terminal neu starten, Probe-Ausführung) intakt ist.

## Akzeptanzkriterien

- [x] Ein macOS-.app-Bundle lässt sich bauen und starten. (`scripts/package-macos.sh`)
- [x] Info.plist, Icon und Versionsnummern sind korrekt gesetzt; Ressourcen (Fonts, Grammatiken) eingebettet. (Fonts/Grammatiken sind in die Binary einkompiliert — keine losen Ressourcen; Version aus `crates/app/Cargo.toml`.)
- [x] Ein reproduzierbarer Release-Build-Prozess existiert (Skript). (`scripts/package-macos.sh`, `.github/workflows/release.yml`)
- [x] Code-Signierung/Notarisierung ist vorbereitet/dokumentiert (sofern Zertifikate vorhanden). (Opt-in via `LABONAIR_SIGN_IDENTITY`/`LABONAIR_NOTARY_PROFILE`, `docs/RELEASE.md`; blockiert CI nie.)
- [x] Die Grundlage für Auto-Updates ist definiert (Manifest/Mechanik). (`labonair_backend::updater` — `latest.json`-Format, Endpunkt, `SemVer`-Check; Download/Apply = T15-005.)
- [x] Eine Perspektive für die Linux-Verpackung ist vorbereitet (später Ziel). (`docs/RELEASE.md` — AppImage/Flatpak hinter `scripts/package-<os>.sh`.)
- [x] Ein Release-Dokument/Changelog existiert. (`docs/RELEASE.md`, `docs/LICENSES.md`, `CHANGELOG.md`)
- [x] Ein End-to-End-Smoke-Test verifiziert, dass das Bundle startet und Kernfunktionen funktionieren. (`scripts/smoke-test.sh` + `crates/app/tests/smoke.rs`)

## Notizen

- Anders als Tauri gibt es kein gepolstertes Bundling-Tool; das .app-Bundle wird jeweils manuell/skript gestaltet.
- Der Fokus: macOS first, Linux später (gemäß Zielbeschreibung).
- Signierung/Notarisierung sind optional (wenn keine Zertifikate), aber das Verfahren sollte dokumentiert sein, um später hinzugefügt zu werden.

## Warnungen

- ⚠️ Gebundelte Grammatiken/Fonts nicht vergessen — fehlende Ressourcen führen zu defektem Verhalten im Release, obwohl Dev läuft.
- ⚠️ Code-Signierung/Notarisierung im Notfall nicht CI-blockieren — dokumentieren und manuell/optional gestaltbar halten.

## Weiterführende Tasks

- Projektabschluss: Diese Phase schließt die Portierung ab.
