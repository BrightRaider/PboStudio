# ADR-0011: Texte und Sprachen

- **Status:** Vorgeschlagen
- **Datum:** 2026-11-04

## Kontext

1.x hat Deutsch und Englisch über eine Zeichenketten-Tabelle (590 Zeilen), die per Schlüssel abgefragt wird. Gefundene
Fehlerklassen: ein Schlüssel, der nur im Code vorkommt und in der Tabelle fehlt (zeigt dann den Schlüssel); eine „Übersetzung",
die mit dem Schlüssel identisch ist; ein Text, der eine Zahl **errät** („eine Nacht bis ein Wochenende"), wo man sie rechnen
kann; Absätze, die eine bereits getroffene Entscheidung erklären, wo ein Tooltip reicht. Ein Teil davon war nur durch
Test-Hilfskonstrukte aufzuspüren.

## Entscheidung

1. **Schlüssel sind ein Typ.** Ein fehlender Schlüssel ist ein **Übersetzungsfehler zur Bauzeit**, kein Laufzeitfehler. Entweder
   über eine Aufzählung oder eine Makro-Tabelle, die beide Sprachen verlangt.
2. **Beide Sprachen sind Pflicht pro Eintrag.** Eine Allow-Liste benennt, wo Deutsch und Englisch gleich sind (STATUS, TEMP).
3. **Texte mit Zahlen sind Formate, nicht Sätze:** Dauer, Wert, Kern kommen als Argumente. Keine geratenen Größen im Text.
4. **UX-Regeln aus 1.1 sind Teil der Spezifikation** (`spec/ux.md`): eine Empfehlung statt vieler Optionen; Erklärungen stehen im
   Tooltip, die Fakten auf dem Bildschirm; keine Fachbegriffe, wo ein Satz reicht.
5. **Ton wie in der Signatur** ([ADR-0009](0009-oberflaeche.md)): kurz, ruhig, ohne Ausrufezeichen. Das wird in der
   Design-Beschreibung festgehalten, bevor Texte entstehen.

## Alternativen

- Fluent oder gettext. Verworfen für den Anfang: zwei Sprachen und wenige Formen; ein Typ reicht und ist am strengsten.

## Folgen

- Das Übersetzen eines Textes ist nicht mehr zu vergessen, auch nicht beim Entfernen eines Elements.

## Prüfung

Ein Test, der jeden Schlüssel in beiden Sprachen verlangt; keiner, der den Schlüssel selbst als Text liefert.
