# ADR-0007: Fehlererkennung — ein Kern besteht nur, wenn er getestet wurde

- **Status:** Vorgeschlagen
- **Datum:** 2026-11-04

## Kontext

Vier unabhängige Signale entscheiden über „durchgefallen": Meldung der Engine, Prozess verschwunden, Machine Check
(WHEA), Kern ohne Last. Die Durchsicht von 1.1.2 fand in dreien davon einen Fehler, bei dem ein Kern **bestand, ohne
getestet worden zu sein**: Fehlerzeilen wurden über die Lesegrenze hinaus als gelesen markiert; ein abgelehnter Tiefwert
wurde als bestanden gebucht; ein Machine Check wurde über eine ungemessene Annahme (APIC-Id = Prozessornummer) einem
anderen Kern zugeschrieben.

## Entscheidung

1. **Logdateien nur zeilenweise vollständig** (`LogTail`): Nichts hinter dem letzten Zeilenumbruch wird gelesen, bevor
   dahinter ein Umbruch steht. (I2)
2. **Ein Wert gilt als angelegt, wenn er zurückgelesen wurde** (jenseits von −30). Wird er abgelehnt: Suche endet beim
   letzten bewährten Wert mit Sicherheitsabstand, ohne bewährten Wert wird der Kern übersprungen, getestet wird nie „was
   gerade anliegt". (I1)
3. **Machine Checks über die gemessene APIC-Tabelle** (CPUID Blatt 0xB, je logischem Prozessor mit festgenageltem Thread).
   **Neu gegenüber C#: Ist die Zuordnung unklar, zählt der Fehler gegen den getesteten Kern**, nicht dagegen. Ein
   zu Unrecht belasteter Kern kostet einen Durchlauf, ein zu Unrecht freigesprochener Kern kostet Vertrauen. (I4)
4. **WHEA schwerer Fehler (Ereignis 18) wird gezielt abgefragt**, nicht aus den letzten N Einträgen aller Art gefiltert.
5. **Wiederaufnahme unterscheidet Absturz und geschlossenes Programm** über die Bootzeit (I9).
6. **Der Temperatur-Notschutz ist nie still inaktiv:** ohne Messwert gibt es eine Warnung; unplausible Werte werden
   verworfen, nicht geglaubt (I6).
7. **Die Einordnung eines Worker-Fehlers** („Prozess weg", „Kern ohne Last") ist einstellbar; Rechenfehler und Machine Check
   werden **nie** herausgefiltert.

## Alternativen

- Zeilen wie bisher bis zum Dateiende lesen. Verworfen: Fehlalarm unmöglich, Fehlbestehen möglich.
- Unklare WHEA-Zuordnung verwerfen (heutiges Verhalten). Verworfen, siehe Punkt 3.

## Folgen

- Mehr Durchläufe wiederholen, wenn die APIC-Tabelle nicht lesbar ist. Das ist gewollt.
- Der Engine-Adapter (Prime95, y-cruncher) bleibt klein: er liefert Zeilen, `pbo-core` entscheidet.

## Prüfung

`LogTailTests`, `RunnerSafetyTests`, `ApicTopologyTests`, `InterruptedRunTests` werden Vektoren. Jeder Vektor muss gegen eine
absichtlich kaputte Implementierung scheitern ([ADR-0012](0012-pruefstrategie.md)).
