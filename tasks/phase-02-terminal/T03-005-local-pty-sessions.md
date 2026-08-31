# T03-005: Lokale PTY-Sessions und Multi-Tab-Terminal

## Status
⏳ Pending

## Phase
2 — Terminal-Engine

## Abhängigkeiten
T03-001 (alacritty_terminal einbinden)
T03-002 (GPUI-Terminal-Renderer)
T03-003 (Tastatur- und Maus-Mapping)

## Ziel
Lokale PTY-Sessions als vollwertige, mehrfach gleichzeitig laufende Terminal-Instanzen innerhalb der App-Management verwaltbar und darstellbar machen. Dazu gehören das Erstellen neuer lokaler Terminal-Sessions (mit wählbarer Shell), das Verwalten mehrerer gleichzeitig laufender Sessions, das saubere Beenden sowie das Ändern der Shell-Auswahl pro Tab. Das entspricht der heutigen "lokal-Terminal"-Tab-Funktionalität von Labonair.

## Kontext
In Labonair gibt es zwei Arten von Terminal-Sessions: lokale- und SSH-Sessions (SSH folgt in Phase 6). Lokale Sessions starten eine Shell (zsh/bash/fish nach Wahl des Benutzers) auf derselben Maschine, über ein PTY. Der Benutzer kann mehrere Terminal-Tabs öffnen, jeder mit eigener Session und Shell-Einstellung.

Wichtig ist die zugrunde liegende Terminologie: Die Terminal-Logik (T03-001–004) ist die "Engine" pro Session; das Tab-System (Phase 3) verwaltet die Sichtbarkeit und das Layout. Diese Task sorgt dafür, dass die "Engine" als verwaltete, mehrfach instanziierbare Session funktioniert — mehrere Sessions können leben, während der Benutzer nur eine sieht.

Dazu gehört auch das Verhalten beim Tab-Schließen: Eine laufende Shell mit laufendem Prozess soll korrekt beendet (SIGTERM/SIGHUP) und das PTY geschlossen werden, ohne zu hängen.

## Anweisungen zur Umsetzung

1. **Session-Registry aufbauen.** Ein Modul, das alle laufenden lokalen Terminal-Sessions verwaltet:
   - Eine eindeutige Session-ID pro Session.
   - Abbild von ID → Session-Handels (Engine + Metadaten + Zustand).
   - Methoden zum Erstellen, Abrufen, Beenden einer Session.
   - Thread-sicheren Zugriff, da Sessions asynchron laufen.

2. **Neue Session erstellen.** Implementiere das Anlegen einer neuen lokalen Session:
   - Shell-Auswahl (Standard-Shell oder eine vom Benutzer konfigurierte bzw. gewählte Shell).
   - Start im aktuellen Arbeitsverzeichnis (das aus der CWD des Vorgänger-Kontexts stammt, ggf. aus T03-004).
   - Initiale Terminal-Dimensionen (an die Tab-Größe angepasst).
   - Optionaler Befehl, der beim Start ausgeführt wird (analog zu "Startbefehl" / "Startup-Snippet").

3. **Mehrere gleichzeitige Sessions.** Verifiziere, dass mehrere lokale Sessions parallel laufen können, ohne sich gegenseitig zu beeinflussen. Jede Session hat eigene Engine, eigenen PTY, eigene Farb-/Metadaten-Ausgabe. Das während Wechsel zwischen Tabs (Sichtbarkeit) soll die Session im Hintergrund weiterlaufen (Prozess-Aktivität fortsetzen) — nicht pausiert.

4. **Session beenden.** Implementiere das saubere Beenden einer Session:
   - Bei laufendem Vordergrund-Prozess ggf. freundliche Aufforderung (analog dem KeepTerminal-Verhalten) oder direktes Beenden.
   - SIGTERM/SIGHUP senden, um die Shell zu beenden.
   - PTY schließen und Ressourcen (Reader/Absender) freigeben.
   - Hängende Prozesse (Vordergrund-Job) erkennen und sicherstellen, dass das Beenden zuverlässig passiert.

5. **Re-Init/Restart-Verhalten.** Für den Fall, dass eine Session beendet wurde (Shell-Exit) soll das Terminal-UI entscheiden können, ob eine neue Session im selben Tab gestartet wird (analog dem KeepTerminal-Click-um-neu-Zu-Startezu-Verhalten).

6. **Shell-Wechsel pro Session.** Erlaube es, die beim Start verwendete Shell zu konfigurieren (falls die App dies pro Tab unterstützt) — die Shell-Auswahl soll bei der Session-Erstellung mitgetragen werden.

7. **Verbinden mit dem Tab-System.** Sorge für eine saubere Trennlinie und Schnittstellen-API zur späteren Phase-3-Tab-Verwaltung: Das Tab-System ruft die Session-Registry auf, um eine Session zu erstellen/abzurufen/zu beenden und um die Sichtbarkeit einer zugeordneten View zu steuern (ohne die Session-Logik selbst zu besitzen).

8. **Tests schreiben.** Erstelle Tests, die:
   - Das Erstellen mehrerer paralleler lokaler Sessions verifizieren.
   - Das korrekte Beenden (inkl. Beenden mit Vordergrund-Prozess) verifizieren.
   - Das Ausführen eines Befehls beim Start verifizieren.
   - Das Weiterlaufen im Hintergrund bei Tab-Wechsel verifizieren.

## Akzeptanzkriterien

- [ ] Ein Session-Registry-Modul verwaltet beliebig viele gleichzeitige lokale Sessions.
- [ ] Eine neue Session startet die gewählte Shell im aktuellen Arbeitsverzeichnis.
- [ ] Mehrere Sessions laufen parallel und unabhängig voneinander; Tab-Wechsel pausiert nicht die Prozesse.
- [ ] Das Beenden ist sauber: kein Hängen, keine Zombie-Prozesse, Ressourcen freigegeben.
- [ ] Beendete Sessions zeigen ein sinnvolles "Shell beendet"-Verhalten und erlauben Neustart im selben Tab.
- [ ] Die Schnittstelle zum Tab-System (Phase 3) ist klar und dokumentiert.
- [ ] Alle Tests laufen grün.

## Notizen

- Diese Task legt die Grundlage für die phasenweise Tab-Verwaltung; die visuelle Tab-Leiste und das Split-Layout folgen in Phase 3.
- Das "KeepTerminal"-Verhalten von Labonair (am unteren zuletzt-Exit-screen mit hint zu klicken um neue Shell zu öffnen) sollte hier vorbereitet werden.
- Prozessbeendigung sorgfältig: Vordergrund-Prozesse (z.B. laufendes vim) zu erkennen und zu signalisieren ist wichtig, damit kein Prozess hängen bleibt.

## Warnungen

- ⚠️ Hängende Prozesse (Vordergrund-Job) dürfen blockieren — Vordergrund-Erkennung und korrektes Signalisieren (SIGTERM und ggf. SIGKILL-Timeouts) einbauen.
- ⚠️ Das gleichzeitige Lesen mehrerer PTYs darf sich nicht gegenseitig blockieren — jeder Lese-Thread/Stream unabhängig verwalten.
- ⚠️ Beim Startbefehl: Abwägen, ob der Befehl interaktiv (Shell wechselt nicht) oder einmalig (Shell läuft danach weiter) ausgeführt wird — je nach Labelonair-Verhalten.

## Weiterführende Tasks

- Phase 3: Tab-System & Workspace-Layout
