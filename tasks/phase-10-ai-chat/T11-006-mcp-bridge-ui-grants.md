# T11-006: MCP-Bridge — Grants-UI & Settings

## Status
✅ Done

## Phase
10 — AI-Chat-System

## Abhängigkeiten
T11-005 (MCP-Bridge Server), T13-001 (Einstellungen-Struktur & Preferences)

## Ziel
Die Benutzeroberfläche der MCP-Bridge: Per-Tab-Opt-in („AI-Agent-Zugriff erlauben"),
Header-Badge mit den freigegebenen Tabs + Einzel-Widerruf, Settings-Bereich (Enable-Toggle,
Setup-Befehl mit Copy, Token neu generieren, Port/Max-Timeout/Auto-Revoke, Per-Host-Block-Liste,
Aktivitäts-Benachrichtigung).

## Kontext
Referenz:
- `reference-src/src/modules/tabs/` → `TabBar` Kontextmenü „Grant AI Agent Access"
  (`ContextMenuCheckboxItem` auf SSH-Workspace-Tabs), `agentAccessStore.ts`.
- `reference-src/src/modules/header/` → `AgentAccessBadge` (Popover/Pill, Liste + Per-Tab-Revoke,
  komplett versteckt wenn nichts freigegeben).
- `reference-src/src/modules/settings/sections/ConnectionsSection.tsx` → „AI Agent Bridge (MCP)":
  Enable-Toggle, `claude mcp add --transport http …` mit Copy, Regenerate-Token, Port/Timeout/
  Auto-Revoke-Felder, `mcpNotifyOnActivity`-Toggle.
- Per-Host-Block: `Host.block_agent_access` (SQLite-Spalte) + Toggle im Host-Formular
  („Agent Access"-Abschnitt). Enforcement 3-fach: Grant-Setzen ablehnen, bei jedem
  `run_command`/`send_keys`/`read_output`/`open_tab` re-prüfen, `hosts_update` widerruft laufende
  Grants sofort wenn Flag gesetzt wird.

## Anweisungen
1. Tab-Kontextmenü-Eintrag „AI-Agent-Zugriff erlauben" (Checkbox) auf SSH- und lokalen
   Terminal-Tabs → ruft die Grant-Funktion aus T11-005.
2. Header-Badge: sichtbar nur wenn ≥1 Grant; Popover listet Tabs, je Zeile Widerruf.
3. Settings-Bereich (in die Struktur aus T13-001 einhängen): Enable, Setup-Befehl (mit dem
   aktuellen Port + „Token wird beim Kopieren nicht angezeigt"-Verhalten wie im Original),
   Regenerate, Port/Max-Timeout/Auto-Revoke-Minuten, `notifyOnActivity`.
4. Per-Host-Block-Toggle im Host-Formular (Phase 06 liefert das Formular — hier nur der
   zusätzliche Abschnitt) + `block_agent_access`-Spalte in der Hosts-Migration.
5. Alle Werte aus den Preferences (T13-001) an den MCP-`McpState` pushen — der Server hat keine
   eigene Persistenz (load-bearing, wie im Original).
6. Aktivitäts-Benachrichtigung: `mcp_activity`-Signal → Toast (T04-004) nur wenn Preference an.
   Fehler laufen über den normalen Fehler-Toast-Pfad, nicht über dieses Signal.

## Akzeptanzkriterien
- [x] Tab-Kontextmenü togglet den Grant; Badge erscheint/verschwindet entsprechend
- [x] Badge-Popover listet freigegebene Tabs + Einzel-Widerruf funktioniert
- [~] Settings: Enable/Regenerate/Port/Timeout/Auto-Revoke wirken — *Settings-**Fenster** gibt es
  noch nicht (T13-001 ⏳). Die Backend-Wirkung (`mcp_set_*`) + Persistenz (`settings::mcp::McpPrefs`)
  + Startup-Push nach `McpState` sind implementiert; die visuellen Regler hängen sich in T13-001 ein.
- [~] Setup-Befehl — dito, gehört ins Settings-Fenster (T13-001)
- [x] Per-Host-Block: Toggle im Host-Formular + `block_agent_access`-Spalte; Grant-Ablehnung +
  Sofort-Widerruf laufender Grants war bereits im Backend (T01-002) implementiert
- [x] `mcpNotifyOnActivity` an → Toast bei Agent-Aktivität; aus → still (`McpActivity`-Handler)
- [x] Preference-Werte überleben Neustart (`labonair-settings.json` → `mcp`) und werden beim
  Start an `McpState` gepusht (`AppShell::new`)
- [x] `cargo check` + `clippy -- -D warnings` + `cargo test` + `fmt --check` grün

> **Teilweise durch T13-001 blockiert:** Das Settings-Fenster/-Pane existiert in der Rust-App noch
> nicht (Phase 12). Alles außer den *sichtbaren* Settings-Reglern (AI Agent Bridge Pane inkl.
> Setup-Command/Copy/Regenerate-Button) ist fertig und getestet. Wenn T13-001 das Settings-Fenster
> baut, muss dort nur noch eine `ConnectionsSection`-Portierung die vorhandenen `mcp_set_*`-Funktionen
> + `McpPrefs`-Load/Save verdrahten (`AgentAccessStore::set_bridge_enabled`/`set_notify_on_activity`
> spiegeln bereits).

## Notizen
- Badge-Layout am `JumpHostDropdown` des Originals orientieren (gleiche Popover/Pill-Optik).
- Grant-Store spiegelt lokal + pusht nach Rust (wie `agentAccessStore.ts`).

## Warnungen
- ⚠️ Grant-Enforcement niemals nur clientseitig — die echte Prüfung ist im Server (T11-005),
  die UI ist nur Komfort.
