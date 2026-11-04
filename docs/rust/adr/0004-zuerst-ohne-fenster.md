# ADR-0004: Zuerst ohne Fenster — Zustandsmaschine im Kern

- **Status:** Vorgeschlagen
- **Datum:** 2026-11-04

## Kontext

Die Fehler, die in 1.1.x am Fenster gefunden wurden, haben einen gemeinsamen Ursprung: Zustand, der nur dort lebt.
Ein zweiter Start neben dem ersten (der Wiederherstellungspunkt dauerte Minuten, und niemand hatte „läuft" markiert); ein
Start, der abgebrochen wurde und trotzdem als Lauf im Durchlauf stand; ein Schließen mitten im Lauf, das die Kerne auf 0
ließ; zwei Abbruch-Knöpfe auf einem Feld. Jeder davon wäre als Zustandsübergang in einem Test aufgefallen.

## Entscheidung

1. **Der Ablauf ist eine Zustandsmaschine im Kern:** Zustände (`Bereit`, `Vorbereitung`, `Läuft(Phase, Kern)`,
   `Wird beendet`, `Fertig`, `Angehalten`) und Übergänge sind Daten. Ungültige Übergänge (Start im Zustand `Vorbereitung`)
   sind im Typ unmöglich oder liefern einen Fehler, aber nie einen zweiten Lauf.
2. **Oberfläche und CLI sind gleichrangige Clients.** Beide senden Befehle und lesen Zustand; keine entscheidet etwas.
3. **Die CLI kommt zuerst** (Phase 3). Alles, was ein Fenster kann, kann die CLI: Auto-Durchlauf, Einzeltest, Werte lesen,
   BIOS-Liste, Bericht. Das macht Fehlerberichte reproduzierbar („führe `pbo auto --dry-run` aus und schicke die Ausgabe").
4. **Die Sichtbarkeit von Bedienelementen ist eine Funktion des Zustands**, nicht Code im Fenster (`AutoPanel.Describe` in C#
   war der erste Schritt dahin und hat sofort einen doppelten Abbruch-Knopf gefunden).

## Alternativen

- Logik in der Oberfläche, wie heute. Verworfen: Ursache der Fehlerklasse.
- Nur ein Fenster, keine CLI. Verworfen: macht Tests und Fehlerberichte teuer.

## Folgen

- Mehr Entwurfsarbeit vor dem ersten Bild. Dafür ist die Oberfläche am Ende dünn und austauschbar.
- Die CLI ist ein zweites Produkt, das gepflegt werden muss; sie ist klein, weil sie nur Befehle weiterreicht.

## Prüfung

Eine Tabelle aller Zustandsübergänge als Test; jedes Sichtbarkeitsmuster der Oberfläche hat einen Zustandstest.
