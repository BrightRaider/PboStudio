# ADR-0009: Oberfläche — dieselbe Handschrift wie die anderen Programme des Eigentümers, ohne deren Quelltext

- **Status:** Entschieden vom Eigentümer. Die Oberfläche der anderen Programme wird 1:1 übernommen, als eine Art Signatur;
  deren Quelltext darf nicht ins öffentliche Repository, Farben und Maße schon.
- **Datum:** 2026-11-04

## Ziel

Es geht um eine **wiedererkennbare Handschrift**: Wer ein Programm des Eigentümers öffnet, soll es am Aussehen erkennen. Das
Aussehen besteht aus Werten und Regeln, nicht aus Quelltext:

- Farben (dunkel und hell) mit **Bernstein-Akzent** (`#F5A623` dunkel, `#E0900C` hell);
- Schrift: Segoe UI und Segoe Fluent Icons **aus Windows selbst**, nichts eingebettet;
- Rundungen (8 und 10), Abstände, Textgrößen (14 / 12,5 / 22);
- Aufbau aus **Karten und Zeilen** (Beschriftung, Erklärung als Tooltip, Bedienelement rechts), Reiter oben, ruhige Flächen.

Funktionsfarben (bestanden, Gefahr, Info) und die PboStudio-eigenen Bausteine dürfen dazukommen, solange sie sich in diese
Sprache einfügen.

## Grenze

Der Quelltext der anderen Programme des Eigentümers ist **privat und bleibt es**. Daraus folgt:

1. **Nichts davon wird kopiert, auch nicht abgetippt.** `pbo-ui` ist neu geschrieben, gegen eine **Design-Beschreibung**
   (`docs/design.md`, öffentlich): Farbwerte, Schrift, Rundungen, Abstände, Größen, Verhalten der Bausteine in Worten und
   Bildern. Die Beschreibung ist die Quelle der Wahrheit für das Aussehen; sie hält beide Programme zusammen, ohne dass eines
   vom anderen abhängt.
2. **Kein gemeinsames Crate.** Ein `ui-kit`, das beide benutzen, müsste öffentlich sein, und damit auch der Code, der heute
   privat ist. Die Gefahr, dass beide Programme auseinanderlaufen, ist der Preis; sie wird über die Design-Beschreibung und
   eine Vorschau mit Bildvergleich begrenzt (Punkt 6).
3. **Dieses Verzeichnis nennt keine Interna des privaten Projekts** (Dateien, Typen, Schnittstellen). Es beschreibt nur, was
   ein Nutzer sieht, und die öffentlich bekannten Zahlen.

## Entscheidung

1. **Toolkit:** egui/eframe **0.29.1** (OpenGL über `glow`), `winit` 0.30. Die Version wird festgenagelt und nur bewusst
   gewechselt ([ADR-0014](0014-einbettbar.md): ein Gastgeber, der den Baustein direkt einbindet, braucht dieselbe).
2. **Schriftdarstellung:** Heller Text auf dunklem Grund kann mit dem Standard-Renderer zu kräftig oder zu dünn wirken. Ein
   angepasster egui-Renderer (fremder Code, MIT oder Apache-2.0, mit einer kleinen Änderung an der Berechnung der
   Glyphendeckung) darf im öffentlichen Repository stehen, mit dem Lizenztext des Originals und einem Vermerk zur Änderung.
   **Zuerst ohne die Änderung bauen** und im Vergleich entscheiden, ob sie sichtbar nötig ist.
3. **Aufbau:** Die Oberfläche trifft **keine Entscheidungen** ([ADR-0004](0004-zuerst-ohne-fenster.md)). Sie liest einen
   Zustand von `pbo-app` und gibt Absichten zurück; Seiten und Texte sind Daten, Zustände lassen sich ohne Hardware
   darstellen.
4. **Was PboStudio zusätzlich braucht:**
   - die **Kernliste** (Zeile mit Zahlenstepper, Hinweis-Chip, Zustand) und die **Auswahl pro Chiplet**;
   - die **drei Reiter** Auto / Tests / Erweitert als Segmentleiste;
   - der **Telemetrie-Graph** (in C# 316 Zeilen eigenes Zeichnen, hier ein Painter-Widget);
   - **Toast**, **Fortschritt mit Restzeit**, **Protokoll-Leiste**, **Einrichtungsassistent** (Treiber, Engines);
   - **Funktionsfarben:** *bestanden*, *Gefahr*, *Info*, neben dem Akzent, als Ergänzung der Palette.
5. **Der Akzent wird Bernstein.** Das ist eine sichtbare Änderung gegenüber 1.x und **gewollt**: Der Akzent ist der
   erkennbarste Teil der Signatur. Grün bleibt die Farbe für „bestanden".
6. **Oberflächentests:** Jeder Zustand aus [ADR-0004](0004-zuerst-ohne-fenster.md) bekommt eine **Vorschau ohne die
   Anwendung** und ein Bildschirmfoto, das gegen eine Referenz verglichen wird. Das ersetzt das, was in dieser Sitzung von
   Hand per PowerShell-Bildschirmausschnitt ging.

## Alternativen

- **Code aus den privaten Programmen des Eigentümers in das öffentliche Repository kopieren.** Vom Eigentümer ausgeschlossen.
- **Gemeinsames öffentliches `ui-kit`.** Siehe Grenze, Punkt 2: würde privaten Code öffentlich machen.
- **Slint, Tauri/WebView2, iced.** Nicht wegen Schwächen verworfen, sondern weil dieselbe Handschrift mit egui am
  geradlinigsten entsteht und der Eigentümer sie dort schon hat.
- **Avalonia behalten.** Verworfen ([ADR-0001](0001-neubau-in-rust.md)).

## Folgen

- Das Aussehen ist als Entscheidung fertig; die Arbeit liegt in den neuen Bausteinen und in einer genauen Design-Beschreibung.
- Neu schreiben heißt, dass Feinheiten (Abstände, Zustände beim Überfahren, Fokus) einmal **nachgemessen** werden müssen,
  nicht nachgebaut aus dem Gedächtnis. Das geschieht an Bildschirmfotos der anderen Programme, nicht an deren Quelltext.
- egui ist ein Sofortmodus-Toolkit: Die Kernliste mit 16 bis 32 Zeilen und ein Graph, der alle 2 s Daten bekommt, sind
  unkritisch. Ein sehr großer Text (das Protokoll) muss virtualisiert werden.
- Barrierefreiheit ist in egui schwächer als in nativen Toolkits. Die Avalonia-Oberfläche lieferte der Automation auch
  keine Elemente, es ist also kein Rückschritt.
- **Offen:** Wie der Bernstein-Akzent neben den Phasenfarben (Grün/Blau/Violett) wirkt, zeigt erst eine Vorschau. Stören die
  Phasenfarben die Signatur, werden sie zurückgenommen (Form oder Symbol statt Farbe), nicht der Akzent.

## Prüfung

Gate **G5:** Eine Person, die das Programm nie gesehen hat, startet ohne Hilfe einen sinnvollen Lauf; jeder Zustand hat eine
Vorschau mit Referenzbild; `pbo-ui` hängt nur von `pbo-app` ab; im Repository liegt kein Quelltext aus dem privaten Projekt.
