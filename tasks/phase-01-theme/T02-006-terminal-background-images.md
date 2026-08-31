# T02-006: Terminal-Hintergrundbilder

## Status
⏳ Pending

## Phase
1 — Theme-System & Design-Tokens

## Abhängigkeiten
T02-002 (Theme-Store), Phase 2 (Terminal-Renderer — für die tatsächliche Darstellung)

## Ziel
Volle Parität zum `backgrounds`-Feature des Originals: Benutzer können ein Hintergrundbild
importieren, auswählen, mit Deckkraft/Blur/Anpassung (cover/contain/tile) hinterlegen — hinter
Terminal und/oder App. Bilder werden lokal verwaltet.

## Kontext
Referenz:
- Backend: `reference-src/src-tauri/src/modules/backgrounds/` (Commands `backgrounds_list/import/delete`,
  Speicherort im App-Data-Dir).
- Frontend: `reference-src/src/modules/settings/` → `BackgroundImageLayer` und die Appearance-Section.
- Design-Werte (Overlay-Opacity, Blur-Radius) in `reference-src/src/styles/globals.css`.

## Anweisungen
1. `backgrounds`-Modul nach `crates/backend/src/backgrounds.rs` (oder eigenes Crate-Modul)
   portieren: Bild importieren (Kopie in App-Data-Dir `backgrounds/`), auflisten, löschen.
   Tauri/IPC-Wrapper entfernen → direkte Funktionen.
2. Theme-/Preferences-Objekt erweitern: `background_image: Option<PathBuf>`, `background_opacity`,
   `background_blur`, `background_fit` (Cover/Contain/Tile), `background_target`
   (App / Terminal / beide) — Feldnamen/Defaults aus dem Original übernehmen.
3. GPUI-Rendering: Hintergrundbild als unterste Layer im App-Root bzw. Terminal-Element
   (`img()` / `div().bg(...)` mit Image-Source — API in gpui/gpui-component verifizieren).
   Overlay-Farbe aus Theme darüber legen, damit Kontrast/Lesbarkeit stimmt.
4. Import-Dialog über nativen File-Picker (GPUI `cx.prompt_for_paths` o.ä. — API verifizieren).
5. Settings-UI wird in T13-002 (Appearance) angebunden — hier nur die Datenschicht + Rendering.

## Akzeptanzkriterien
- [ ] Bild importieren/auflisten/löschen funktioniert; Dateien liegen im App-Data-Dir
- [ ] Hintergrundbild rendert hinter Terminal mit korrekter Deckkraft/Blur/Fit
- [ ] Auswahl „App / Terminal / beide" wirkt wie im Original
- [ ] Einstellung überlebt Neustart (Persistenz via Preferences)
- [ ] `cargo check` + `clippy -- -D warnings` + `cargo test` grün

## Notizen
- Bild-Decoding: `image`-Crate; große Bilder ggf. beim Import herunterskalieren.
- Blur ist teuer — prüfen, ob GPUI einen Blur-Filter bietet oder ob vor-geblurrt werden muss.

## Warnungen
- ⚠️ Hintergrundbild darf Terminal-Performance (T15-003) nicht spürbar drücken — einmal
  dekodieren/cachen, nicht pro Frame.
