# T19-010: Settings › Hosts — Host- & Credential-Verwaltung als Top-Level-Kategorie

## Status
✅ Done

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T19-004 (generierte Settings-UI + Custom-Top-Level-Pfad), T16-008 (`labonair-hosts-ui`), T19-001 (`hosts: HostsContent` + `AREAS`-Eintrag), T17-009 (interimer `TabKind::Hosts`)

## Ziel
Die Host-/Credential-Verwaltung bekommt ihren **endgültigen** Ort: eine
erstklassige Top-Level-Settings-Kategorie **„Hosts"** (Peer von „Themes").
Damit entfällt der interime `TabKind::Hosts`-Tab (aus T17-009) — der
Host-Manager ist dann weder Tab noch Panel. Verbinden läuft ausschließlich über
die Command-Palette-`Page::Hosts` (`Enter` = SSH, `Shift+Enter` = SFTP, seit
T16-007), das `＋▾`-Titlebar-Menü und die native Menüleiste; **Verwalten**
(anlegen / bearbeiten / löschen / duplizieren, Credentials, Jump-Hosts,
Tunnel, SSH-Config-Import/Export, Verfügbarkeits-Polling) läuft über diese
Settings-Kategorie. Grundlage: `docs/architecture.md §8.1`,
`docs/settings-guidelines.md` Punkt 4.

## Kontext
- `labonair-hosts-ui` (seit T16-008): `HostManagerView` + Host-Formular +
  `ssh_connection`-Editor. Deps nur `labonair-backend` + `ui-kit` + `theme` +
  `notifications`; Tab-Öffnen über hereingereichte Callbacks. **Kein**
  `impl Panel`.
- `labonair-settings-ui` (seit T16-007): `panes/`-Ordner für Custom-Panes;
  seit T19-004 der `AREAS`-getriebene Kategorie-/Seiten-Aufbau mit
  `SettingsPage { kind: Custom(render_fn) }` und Standard-Seiten-Chrome
  (Kopf, Suche, Herkunfts-Badges).
- `SettingsContent.hosts: HostsContent` (T19-001): nicht-geheime Host-Felder +
  `credential_ref` (Keyring-Referenz, **nie** Secrets in JSON).
- Backend: `labonair-backend::modules::{ssh,hosts}` + `keyring` — das Anlegen/
  Ändern eines Hosts schreibt die nicht-geheimen Felder in den User-Layer von
  `settings.json` (über `SettingsStore`/surgical write, T19-005) **und** das
  Secret in den OS-Keychain.
- Entfallende Zwischenstufe: `TabKind::Hosts` + `open_host_manager` +
  `CommandId::OpenHostManager` (aus T17-009) → hier ersetzt durch
  „Open Host Settings" (`open_settings_window(Some("hosts"))`).
- Referenz-Verhalten der Verwaltung: `reference-src/src/modules/hosts/`
  (Formularfelder, Gruppen, Jump-Host-Auswahl, Tunnel-Zeilen, Import/Export).

## Anweisungen zur Umsetzung
1. **`labonair-settings-ui` → Dep auf `labonair-hosts-ui`** (Abhängigkeitsregel
   9, `docs/architecture.md §3` — erlaubte Kante). `crates/settings-ui/panes/
   hosts.rs` als `SettingsPage { kind: Custom(render_hosts_pane) }`, verdrahtet
   in den `AREAS`-Eintrag `hosts` (Slug `hosts`, `kind = Custom`).
2. **Pane-Aufbau** (im Standard-Seiten-Chrome — Kopf/Suche/Badges kommen vom
   generischen Rahmen, nur der Body ist custom), Abschnitte mit Deep-Link-Slugs:
   - `hosts/list` — Host-Liste (Name, `user@addr:port`, Gruppe, „zuletzt
     verbunden"), mit Aktionen **Neu / Bearbeiten / Duplizieren / Löschen**.
     Auswahl öffnet den Editor rechts bzw. als `SubPageLink`
     (`hosts/edit`).
   - `hosts/edit` (Unter-Seite) — das Host-Formular aus `labonair-hosts-ui`:
     Verbindungsfelder, Auth-Methode (Passwort / Key / Agent), Key-Pfad,
     **Credential** (Eingabe schreibt in den Keychain, Anzeige nur „gesetzt/
     nicht gesetzt"), Jump-Host-Auswahl (Referenz auf anderen Host), Tunnel-
     Liste (lokal/remote, Port-Paare).
   - `hosts/ssh-config` — SSH-Config **Import** (Datei wählen → Vorschau →
     übernehmen) + **Export** (aktuelle Hosts → `~/.ssh/config`-Fragment).
     Port von T07-003.
   - `hosts/availability` — Polling an/aus, Intervall (generierte Felder aus
     `connections`/`hosts` — dürfen als normale `SettingField` im Custom-Body
     mitgerendert werden).
3. **Schreibpfad**: Host anlegen/ändern ⇒ genau **eine** Funktion
   (`hosts_ui::apply_host_change`) schreibt die nicht-geheimen Felder über
   `SettingsStore` in den User-Layer (`hosts.entries`) **und** das Secret in
   den Keychain, setzt `credential_ref`. Kein zweiter Schreibpfad. Live:
   `SettingsStore` benachrichtigt → Command-Palette-`Page::Hosts` +
   `＋▾`-Menü sehen den neuen Host sofort.
4. **`TabKind::Hosts` entfernen** (aus T17-009): Variante + `open_host_manager`
   + `CommandId::OpenHostManager` → ersetzen durch `OpenHostSettings`
   (`open_settings_window(Some("hosts"))`). Native Menüleiste (`menu.rs`) +
   `＋▾`-Submenü „Alle Hosts…" zeigen jetzt auf die Settings-Kategorie.
   `Cmd+Shift+N` bleibt die Palette-`Page::Hosts` (Verbinden), **nicht** die
   Settings.
5. **Connect-Callbacks**: `render_hosts_pane` bekommt `on_open_ssh` /
   `on_open_sftp` von der Shell (die die Tab-Erzeugung im `Workspace`
   auslösen), damit man aus der Liste auch direkt verbinden kann („Verbinden"-
   Button pro Zeile). Keine `settings-ui → workspace`-Kante — Callbacks werden
   beim `open_settings_window` durchgereicht (bestehende `SettingsDeps`).
6. **Tests**:
   - `apply_host_change` schreibt nicht-geheime Felder in `settings.json`
     (User-Layer) und **kein** Secret; `credential_ref` gesetzt.
   - Nach `apply_host_change` liefert die Palette-Hostliste (gleiche
     `known_hosts`-Quelle) den neuen/geänderten Host.
   - Deep-Link `settings://hosts` und `settings://hosts/ssh-config` landen auf
     der richtigen Stelle.
   - `TabKind::Hosts` existiert nicht mehr; `OpenHostSettings` öffnet das
     Settings-Fenster auf „Hosts".
7. `cargo run`: Settings → Hosts → neuen Host anlegen (mit Passwort) →
   speichern → Command-Palette (`Cmd+Shift+N`) zeigt ihn → `Enter` verbindet,
   `Shift+Enter` öffnet SFTP; Host bearbeiten/löschen wirkt live; SSH-Config
   importieren; Jump-Host + Tunnel setzen und verbinden; kein Host-Tab mehr
   öffenbar (nur noch Settings).

## Akzeptanzkriterien
- [x] Top-Level-Kategorie „Hosts" (Slug `hosts`, `kind = Custom`) im
      Settings-Fenster, im Standard-Seiten-Chrome; Abschnitte
      `list` / `edit` / `ssh-config` / `availability` mit Deep-Links.
- [x] Host CRUD + Credentials (Keychain) + Jump-Hosts + Tunnel +
      SSH-Config-Import/Export funktionieren; **ein** Schreibpfad
      (`apply_host_change`); Secrets nie in `settings.json`.
- [x] Änderungen wirken live auf Command-Palette-`Page::Hosts` und das
      `＋▾`-Menü (kein Neustart).
- [x] `TabKind::Hosts`, `open_host_manager`, `CommandId::OpenHostManager`
      existieren nicht mehr; `OpenHostSettings` / Menü / `＋▾` „Alle Hosts…"
      öffnen Settings › Hosts. `Cmd+Shift+N` bleibt der Verbinden-Pfad.
- [x] `cargo tree -p labonair-settings-ui` hat die Kante zu
      `labonair-hosts-ui`; `labonair-hosts-ui` hat **keine** Kante zu
      `labonair-workspace`/`-shell`/`-panel*`.
- [x] Der Host-Manager ist nirgends mehr Tab oder Dock-Panel.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Das ist der Abschluss von Thema 2: nach dieser Task existiert „der
  Host-Manager" nur noch als (a) Palette-Verbinden-Seite und (b)
  Settings-Verwaltungs-Kategorie — genau die gewünschte Zweiteilung.
- Das Host-Formular ist **eine** Komponente aus `labonair-hosts-ui`, hier nur
  eingebettet — nicht neu bauen. Falls T17-009 sie schon für den interimen
  Tab genutzt hat: identische Komponente, nur anderer Einbettungsort.
- Verfügbarkeits-/Polling-Felder dürfen als generierte `SettingField` im
  Custom-Body sitzen — Custom-Body heißt nicht „keine generierten Felder".

## Warnungen
- ⚠️ Secrets: Die Credential-Eingabe darf den Wert **nie** in ein
  `SettingsContent`-Feld oder eine Log-Zeile schreiben — direkt in den
  Keychain, nur `credential_ref` zurück. Review-Punkt.
- ⚠️ Migration: bestehende Hosts liegen heute in SQLite
  (`rusqlite`, `backend::modules::hosts`). Beim ersten Start nach dieser Task
  die SQLite-Hosts **einmalig** nach `settings.json` (`hosts.entries`) +
  Keychain übernehmen; SQLite-Tabelle danach ignorieren (nicht löschen —
  Rückweg offen halten). Migrationsschritt in `T19-009` (Settings-Migrator)
  koordinieren oder hier als einmalige Hydrate implementieren, im
  Doc-Kommentar festhalten.
- ⚠️ `settings.json` mit vielen Hosts kann groß werden — `hosts.entries` als
  Array ist ok, aber die surgical-write-Performance (T19-005) im Blick behalten.

## Weiterführende Tasks
- [T19-007: Globale Settings-Suche](./T19-007-global-settings-search.md)
- [T19-009: Settings-Migrator](./T19-009-settings-migrator.md)
