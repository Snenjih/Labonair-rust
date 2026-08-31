# T11-006: MCP-Bridge — Grants-UI & Settings

## Status
⏳ Pending

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
- [ ] Tab-Kontextmenü togglet den Grant; Badge erscheint/verschwindet entsprechend
- [ ] Badge-Popover listet freigegebene Tabs + Einzel-Widerruf funktioniert
- [ ] Settings: Enable/Regenerate/Port/Timeout/Auto-Revoke wirken (Port-Wechsel startet Listener neu)
- [ ] Setup-Befehl zeigt korrekten Port; Copy funktioniert
- [ ] Per-Host-Block: gesetzt → Tab-Toggle disabled, laufender Grant sofort widerrufen
- [ ] `mcpNotifyOnActivity` an → Toast bei Agent-Aktivität; aus → still
- [ ] Preference-Werte überleben Neustart und werden an `McpState` gepusht
- [ ] `cargo check` + `clippy -- -D warnings` + `cargo test` grün

## Notizen
- Badge-Layout am `JumpHostDropdown` des Originals orientieren (gleiche Popover/Pill-Optik).
- Grant-Store spiegelt lokal + pusht nach Rust (wie `agentAccessStore.ts`).

## Warnungen
- ⚠️ Grant-Enforcement niemals nur clientseitig — die echte Prüfung ist im Server (T11-005),
  die UI ist nur Komfort.
