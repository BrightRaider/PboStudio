# ADR-0013: Lizenz und Herkunft

- **Status:** Vorgeschlagen — braucht eine Bestätigung des Eigentümers
- **Datum:** 2026-11-04

## Kontext

Das Repository steht unter **GPL-3.0**. Einbezogen sind: ZenStates-Core (GPL-3.0, wird upstream von irusanov gepflegt, mit
lokalen Änderungen als `external/zenstates-local-patches.diff`), die signierten PawnIO-Module (**LGPL-2.1**, unverändert,
Release 0.2.4), Prime95 und y-cruncher (Programme Dritter, nur im Offline-Paket mitgeliefert) und künftig egui (MIT oder
Apache-2.0). Das Aussehen der anderen Programme des Eigentümers wird als Werte und Beschreibung übernommen, nicht als Quelltext.

Der Neubau übernimmt **Daten und Wissen**, nicht Quelltext, aus ZenStates-Core: Befehls-IDs, Tabellenlayouts, Kernlayout und
Kommentare.

## Entscheidung

1. **PboStudio bleibt unter GPL-3.0.**
2. **Aus ZenStates-Core abgeleitete Daten** (`families.toml`) tragen **Herkunftsvermerk** (Datei, Zeile, Version) und einen
   Hinweis auf die Lizenz. Es wird kein Quelltext übersetzt, den man nicht ohnehin unter GPL-3.0 weitergeben dürfte; die Daten
   sind es ohnehin.
3. **PawnIO-Module** werden unverändert und mit Lizenztext (`COPYING`) und Quellverweis mitgeliefert. Sie sind Daten, die der
   Treiber lädt; es gibt keine Verlinkung mit dem Programm.
4. **Quelltext aus den privaten Programmen des Eigentümers kommt nicht in dieses Repository** (seine Entscheidung). Farben, Maße und Schrift sind
   Werte und stehen in `docs/design.md`; `pbo-ui` wird neu geschrieben ([ADR-0009](0009-oberflaeche.md)). Der egui-Renderer
   mit der Glättung (falls gebraucht) ist fremder Code und bekommt seinen Originaltext und einen Vermerk zur Änderung.
5. **Abhängigkeiten werden prüfbar:** `cargo deny` (Lizenzen) und eine erzeugte Liste Dritter im Release.
6. **Engines im Offline-Paket:** Die Weitergabebedingungen von Prime95 und y-cruncher werden geprüft und im Repository
   dokumentiert, bevor die Rust-Fassung sie weiter mitliefert. (Heute liegt `license.txt` von Prime95 bei, eine Prüfung ist nicht
   dokumentiert.)

## Alternativen

- Andere Lizenz für die Rust-Fassung. Verworfen: Ableitung aus GPL-3.0-Material.
- Engines nie mitliefern. Möglich, aber das Offline-Paket ist ein Nutzerwunsch.

## Folgen

- Herkunft ist ein Teil der Daten, nicht eine Fußnote.

## Prüfung

`cargo deny check` grün; jede Zeile in `families.toml` hat ein `source`; das Release enthält die Lizenztexte.
