# T15-005: Auto-Updater (macOS)

## Status
✅ Done

## Phase
14 — Testing & Polish

## Abhängigkeiten
T15-004 (Verpackung & Release)

## Ziel
Parität zu `reference-src/src/modules/updater/` + `tauri-plugin-updater`: die App prüft auf
Updates, lädt sie herunter, verifiziert die Signatur und installiert — mit einem
Update-Dialog wie im Original. Da Tauri wegfällt, wird der Mechanismus neu gebaut
(macOS: **Sparkle** oder ein äquivalenter signierter Appcast-Flow).

## Kontext
Referenz:
- `reference-src/src/modules/updater/` — Update-Dialog (verfügbar / lädt / bereit / Fehler),
  Auto-Check-Intervall, „später"/„jetzt neu starten".
- `reference-src/src-tauri/` — `tauri-plugin-updater`-Konfiguration (Endpoint, Public Key,
  Signaturprüfung), `.github/workflows/release.yml` (jetzt unter `reference-src/.github/…`).
- Homebrew-Cask des Originals (separates Repo) als alternativer Distributionsweg.

## Anweisungen
1. Entscheidung dokumentieren: **Sparkle** (bewährt, macOS-nativ, signierter Appcast) vs.
   eigener minimaler Updater (GitHub-Releases-JSON + Ed25519-Signatur + In-Place-Replace).
   Empfehlung: Sparkle für macOS.
2. Update-Feed/Appcast bereitstellen: Release-Artefakt (aus T15-004: signiertes/notarisiertes
   `.app` im `.dmg` oder `.zip`) + Appcast-XML mit Version, URL, Signatur, Release-Notes.
3. In die App einbauen: Auto-Check beim Start + periodisch, „Nach Updates suchen…" aus dem
   macOS-Menü (T04-005) und aus Settings.
4. Update-Dialog nativ in GPUI nachbauen (Zustände: verfügbar / Download läuft (Fortschritt) /
   installationsbereit / Fehler), Texte/Buttons aus dem Original.
5. Release-CI (`.github/workflows/release.yml` neu): Tag → Build → Sign → Notarize → Appcast
   aktualisieren. Abstimmen mit dem Homebrew-Cask-Weg.
6. Fehlerfälle über das Notification-System (T04-004).

## Akzeptanzkriterien
- [x] App erkennt eine neuere Version über den Appcast/Feed — `fetch_manifest` +
      `UpdateManifest::available()` (`SemVer` + platform artifact).
- [x] Download + Signaturprüfung + Installation + Neustart implementiert —
      `download_update` (streamed, progress) → `verify_update` (minisign) →
      `apply_macos_update` (atomic bundle-swap + rollback) → `relaunch`.
      Unit-getestet (Signatur positiv/negativ, Bundle-Swap). *Ende-zu-Ende mit
      echter signierter Test-Version nicht in dieser Umgebung ausführbar
      (keine Developer-ID / kein Release) — dokumentiert in `docs/RELEASE.md`.*
- [x] „Nach Updates suchen…" aus App-Menü **und** Command-Palette löst den
      manuellen Check aus (`AppShell::act_check_for_updates`); die
      `checkForUpdates`-Einstellung steuert den Auto-Check beim Start.
- [x] Update-Dialog zeigt verfügbar / Download (Fortschritt) / bereit / Fehler;
      Texte + Button-Labels 1:1 aus `reference-src/.../UpdaterDialog.tsx`.
- [x] Ungültige/fehlende Signatur wird abgelehnt — negativ getestet
      (`tampered_artifact_is_rejected`, `wrong_key_is_rejected`,
      `missing_signature_or_key_is_an_error`,
      `placeholder_public_key_refuses_everything`).
- [x] Release-CI: `scripts/package-macos.sh` erzeugt `*.app.tar.gz` +
      `latest.json` (+ minisign-Signatur wenn Key gesetzt);
      `.github/workflows/release.yml` signiert/notarisiert (Secrets) und lädt
      beide hoch. *Nicht in dieser Umgebung ausgeführt.*
- [x] `cargo check` + `clippy --workspace --all-targets -- -D warnings` +
      `cargo test --workspace` + `cargo fmt --all --check` grün.

## Notizen
- Sparkle-Anbindung aus Rust: via `objc2`/Framework-Bindings oder ein kleines Swift-Shim.
  Zed nutzt einen eigenen Auto-Updater — dessen Ansatz als Referenz ansehen.
- Linux-Update (AppImage/Flatpak/Repo) ist „später", kein Blocker.

## Warnungen
- ⚠️ Code-Signing + Notarization (Apple Developer ID) ist Pflicht, sonst blockt Gatekeeper das Update.
- ⚠️ Signaturprüfung des Update-Pakets niemals überspringen — Sicherheitskritisch.
