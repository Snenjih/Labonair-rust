# T15-003: Cross-Platform- und Performance-Optimierung

## Status
⏳ Pending

## Phase
14 — Testing & Polish

## Abhängigkeiten
Phase 2 (Terminal), 3 (Layout), sowie Kern-Subsysteme
(ggf. macOS zuerst, Linux später)

## Ziel
Die App auf den Zielplattformen (prioritär macOS, später Linux) zuverlässig laufen lassen und die Performance zu optimieren — schneller Start, flüssiges Rendering, niedriger Speicherverbrauch — ohne Funktions- oder Designverlust. Dazu gehören Plattform-spezifische Feinschliffe (Fensterverhalten, DPI, native Menü) sowie Performance-Profile und -Verbesserungen an heißen Pfaden.

## Kontext
Reflexion der zugrunde liegenden Architektur: Die App ist eine reine GPUI-Native-App (kein WebView, kein IPC/J-SON-Serialisieren), was von Haus aus weniger Overhead bedeutet als die Tauri/React-Version. Dennoch gibt es heiße Pfade, die optimiert werden müssen (Terminal-Rendering, große Listend chirurgien, Git-Status-Polling, AI-Streaming-Rendering), sowie plattformspezifische Feinheiten.

Plattform-Zielsetzung laut Projekt: ErstmacOS, später Linux. Windows ist (jetzt) nicht Ziel. Performance ist das erklärte Motiv hinter der Portierung — dieser Task stellt sicher, dass der Performance-Vorteil real spürbar wird.

## Anweisungen zur Umsetzung

1. **Perf-Baseline aufstellen.** Miss und dokumentiere die aktuelle Baseline:
   - Startzeit (bis Fenster sichtbar + interaktionsbereit).
   - Speicherverbrauch (idle und mit mehreren Terminals/Editor-Tabs).
   - Frame-Drops / Rendering-Zeit bei Terminal-Ausgabe (viele Zeilen), virtuellen Listen, Git-Graph mit vielen Commits.
   - Scroll-Leistung in Terminal, Explorer, SFTP.

2. **Startzeit optimieren.** Reduziere die Zeit bis zur Interaktivität:
   - Lazy-Loadung von Modulen/Grammatiken (nicht alles gleich beim Start laden).
   - Asynchrone Initialisierung von Backend-Komponenten (DB, SSH-Infrastruktur) ohne Blockierung des UI.
   - Effizientes Fenster-Erstellen und Theme-Ablösen.

3. **Terminal-Rendering optimieren.** Sorge für flüssiges Terminal-Scrolling und -Update bei hoher Ausgabe:
   - Damage-Tracking (nur geänderte Bereiche neu zeichnen) konsequent nutzen.
   - Font-/Glyph-Caching effizient; minimale Neuberechnung von Textläufen.
   - Wiederverwendbare Render-Puffer statt Neuzuordnung pro Frame.

4. **Listen-/Graph-Performance.** Optimiere die virtualisierten Ansichten:
   - Explorer/SFTP/Git-List: nur sichtbare Zeilen rendern, effiziente Item-Recycling.
   - Git-Graph: nur sichtbare Commit-Zeilen zeichnen, Layout vorab berechnen.

5. **Netzwerk-/Polling-Optimierung.** Optimiere Background-Aktualisierungen:
   - Git-Status-Polling: bedarfsgerecht, mit Generation-Guards, nicht übermäßig häufig.
   - SFTP/FS-Aktualität: Event-basiert statt Polling wo möglich.
   - AI-Streaming: inkrementelles Rendering (aus T11-003), keine Komplett-Neuberechnung.

6. **Speicher-Lebenszyklus.** Kontrolliere den Speicherverbrauch:
   - Scrollback- und Puffer-Begrenzungen.
   - Freigeben von Ressourcen beim Schließen von Sessions/Tabs.
   - Kein unbegrenztes Caching von Terminal-Ausgaben/Datei-Editionen.

7. **MacOS-Feinschliff.** Sorge für natives macOS-Verhalten:
   - Native Fensterleiste/-titel, Dock-Verhalten, ggf. native Menü (Kürzel-Sync aus T12-002/T13-004).
   - Retina/DPI-korrekte Darstellung.
   - App-Name/Icon, Autostart soweit vorhanden.

8. **Linux-Basis (später).** Bereite die Linux-Unterstützung vor (renderer wgpu/Vulkan), auch wenn primär macOS. Das Layout/Rendern sollte plattformunabhängig bleiben; nur die platformspezifische Integration kapseln.

9. **Perf-Regressionstests.** Sofern automatisierbar (z.B. Frame-Zeiten in CI/Flamegraph), Perf-Tests anlegen oder zumindest manuelle Kriterien dokumentieren, um Regressionen zu vermeiden.

## Akzeptanzkriterien

- [ ] Die App startet schnell (deutlich schneller als die Tauri-Version) und ist zügig interaktiv.
- [ ] Terminal-Ausgabe-Scrolling ist flüssig (kein spürbares Lag bei hoher Ausgabe).
- [ ] Explorer/SFTP/Git-Graph scrollen flüssig bei großen Datenmengen.
- [ ] Git-Status aktualisiert sich effizient (kein Over-Polling).
- [ ] AI-Streaming-Rendering ist flüssig (inkrementell).
- [ ] Speicherverbrauch bleibt kontrolliert beim Öffnen/Schließen vieler Sessions/Tabs.
- [ ] macOS-natives Verhalten (Leiste, DPI, ggf. Menü) funktioniert.
- [ ] Linux-Grundlagen sind vorbereitet/beherrschbar (später Ziel).
- [ ] Die Performance ist messbar schneller/besser als die Referenz-App (wo relevant).

## Notizen

- Performance ist das Motiv der Portierung — diese Phase ist wichtiger als der bloße "Nice-to-have"-Polish.
- Fokus auf die spürbarsten Pfade (Start, Terminal-Rendering, Scroll, große Listen), nicht auf Mikro-Optimierung intransparenter Stellen.

## Warnungen

- ⚠️ Vorzeitige Optimierung vermeiden — zuerst messen (Profiler), dann nur die belegten Engpässe optimieren.
- ⚠️ Keine Optimierung auf Kosten der Design-Parität oder Korrektheit.
- ⚠️ Frame-Timing auf macOS nicht übermachen im Perf-Test-Umfeld (durchgehend vs. Folien-Beschleunigung).

## Weiterführende Tasks

- [T15-004: Verpackung & Release](./T15-004-packaging-release.md)
