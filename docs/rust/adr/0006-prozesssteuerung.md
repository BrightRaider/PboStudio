# ADR-0006: Prozesssteuerung und „immer zurücksetzen"

- **Status:** Vorgeschlagen
- **Datum:** 2026-11-04

## Kontext

Das Programm startet die Testprogramme selbst, nagelt sie auf einen Kern, unterbricht ihre Last und kann dabei andere Kerne
parken. Die Mechanik in C# ist richtig, aber ihr **Zurücksetzen hing an Disziplin**: Es lief nur am normalen Ende der
Schleife. Abbruch, Temperaturnotschutz, ein Engine-Fehler und das Schließen des Fensters ließen die geparkten Kerne auf 0
(behoben in 1.1.2). Ein Beenden des Programms von außen kann auch das nicht abfangen.

## Entscheidung

1. **Start wie bisher:** `CreateProcess` angehalten, Affinität setzen, in ein **Job-Objekt** mit Affinitätsgrenze und
   `KILL_ON_JOB_CLOSE` stecken, dann erst fortsetzen. Ein Prozess kann seine Affinität selbst zurückweiten (y-cruncher
   tut es); das Job-Objekt kann er nicht umgehen.
2. **Anhalten und Fortsetzen über `NtSuspendProcess` / `NtResumeProcess`**; Mikro-Lastwechsel laufen auf eigener
   Uhr unterhalb der Überwachungstaktung und setzen im Abschluss immer fort.
3. **Zurücksetzen ist eine Eigenschaft des Typs:** `ParkedCores` (Wächter, `Drop` setzt zurück), `Session` (`Drop` beendet den
   Prozess), `Pulser` (`Drop` bricht ab und setzt fort). Es gibt keinen Weg, einen Kern zu parken, ohne dass ein Wächter
   existiert.
4. **`panic = "unwind"`** im Laufzeitprofil. Das übliche `panic = "abort"` spart etwas Größe, überspringt aber `Drop` und
   würde Punkt 3 aushebeln. Alternativ ein Panic-Haken, der zurücksetzt, bevor er abbricht. *Das ist ein Zielkonflikt mit
   [ADR-0010](0010-verteilung.md) und wird dort gemessen, nicht geraten.*
5. **Beenden von außen:** Vor dem Parken schreibt das Programm die Absicht atomar nach `runs/parked.json` (Kerne und
   Werte). Beim nächsten Start wird sie, falls der Rechner nicht neu gestartet wurde, zurückgenommen. Das schließt die Lücke,
   die ein `Drop` nicht schließen kann, und ist neu gegenüber C#.

## Alternativen

- Zurücksetzen per Konvention, wie heute. Verworfen: Es ist genau der Fehler, der gefunden wurde.
- Kerne nie parken. Verworfen: Die Zuordnung eines Fehlers zum getesteten Kern ist der Zweck des Parkens.

## Folgen

- Mehr Typen, dafür kein unbewachter Pfad.
- `panic = "unwind"` kostet einige hundert KB.

## Prüfung

Die C#-Tests `RunnerSafetyTests` werden Vektoren. Zusätzlich: ein Test, der den Lauf mit einem Panic im Engine-Adapter
abbricht und prüft, dass die Kerne zurückgesetzt sind; ein Test, der `parked.json` nach einem simulierten Beenden von
außen abarbeitet.
