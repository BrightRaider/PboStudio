# Vergleich mit CoreCycler

Stand 2026-11-05, gegen die Voreinstellungsdatei des öffentlichen Projekts (63 Einstellungen in zehn Abschnitten) und gegen
die C#-Fassung 1.1.3. „da" heißt: im Rust-Fenster oder in der Befehlszeile vorhanden. Es ist kein Code übernommen; verglichen
wurde, was ein Werkzeug dieser Art können muss.

Das Fenster legt die Einstellungen auf zwei Seiten des Reiters Erweitert: **Test** (was getestet wird: Programm, Befehlssatz oder
Algorithmen, FFT-Bereich, Zeit, Runden, Threads) und **Ablauf** (wie es läuft: Lastwechsel, Reihenfolge der Kerne, Suche,
Verhalten bei einem Fehler, Ton).

## Was da ist

| CoreCycler | Rust-Fassung |
|---|---|
| Testprogramm Prime95, y-cruncher | da |
| Zeit je Kern | da: 30 s bis 30 min als Auswahl, in der Plan-Datei beliebig, und **„Auto"**: bis das Testprogramm seine Arbeitsliste einmal durchlaufen hat, höchstens eine Stunde (`--seconds auto`). **An Kern 7 belegt seit der Reparatur** (Prime95 las die Einstellungen nicht, siehe `docs/rust/README.md`, E5): „Auto" endet mit Prime95 von selbst; das Fenster bietet es als Zeit „Auto" an, in der Befehlszeile ist es `--seconds auto` |
| Last zwischendurch anhalten (`suspendPeriodically`, Dauer, Abstand) | da: Lastwechsel „Pausen" (Voreinstellung eine Sekunde alle 30 Sekunden) und „Schübe" (300 ms Last, 100 ms Leerlauf). Die Testlisten bringen ihre Werte mit |
| Reihenfolge der Kerne | da: fortlaufend, abwechselnd (erste und zweite Hälfte im Wechsel), zufällig und **Paare** (die Last wird von jedem Kern an jeden anderen übergeben) |
| Threads je Kern | da („Beide Threads") |
| Anzahl der Durchläufe | da als Runden (1 bis 50) |
| Kerne auslassen | da, als Auswahl der Kerne |
| Test je Kern neu starten | da, immer |
| Bei einem Fehler: Kern auslassen (`skipCoreOnError`), alles anhalten (`stopOnError`) | da als „Bei einem Fehler": Weiter, Kern aus (Voreinstellung wie bei CoreCycler), Anhalten. Gilt für Tests der gehaltenen Werte; eine Suche wird davon nicht berührt, denn Durchfallen ist ihr Weg zur Grenze |
| Piepton und Blinken bei Fehler (`beepOnError`, `flashOnError`) | da: der Systemton bei einem Fehler (nur bei Tests der gehaltenen Werte) und am Ende des Laufs, und die Taskleiste des Fensters meldet das Ende (bei einem Fehler dringlicher). Ein Schalter „Ton bei Fehler und am Ende", in der Befehlszeile `--quiet` |
| WHEA-Fehler im Ereignisprotokoll suchen, Warnung 19 mitzählen | da; ein Ereignis ohne Prozessor wird dem getesteten Kern angelastet, ein Ereignis eines anderen Kerns nicht |
| Prime95: Befehlssatz, FFT-Bereich, freier Bereich | da: SSE, AVX, AVX2, AVX-512; acht Bereiche und **ein eigener Bereich** (kleinste und größte Größe in K, 4 bis 65536) |
| y-cruncher: Algorithmen | da als Wahl zwischen zwei Listen: die fünf für den Curve Optimizer (Voreinstellung) und die drei der Schnellsuche. Gestartet wie bei CoreCycler aus einer Einstellungsdatei (`LogicalCores`, 64 MiB je Thread), denn nur so läuft es auf dem einen Kern mit einem Thread |
| Automatische Anpassung des Curve Optimizers bei Fehler | da, als Suche je Kern, **in beide Richtungen** (ein zu tiefer Wert wird wieder hinaufgesucht) |
| Startwerte, Schrittweite, Wiederholung des Kerns nach Fehler | da (aktuelle Werte, grob 3 / fein 1, die Suche prüft jeden Wert) |
| Weitermachen nach einem Absturz | da: Aufgabe bei der Anmeldung und ein Knopf; der Absturz wird dem Kern angelastet, der unter Test stand |
| Protokolldatei | da (ein Protokoll je Tag, endet um 06:00) |
| Pause zwischen den Kernen (`delayBetweenCores`) | da: `--pause N` Sekunden Ruhe zwischen zwei Kernen, damit der nächste kühl startet |
| y-cruncher: freie Liste der Tests, Speicher | da: `--yc-tests A,B,C` (die Namen von y-cruncher, geprüft) und `--yc-memory MB` je logischem Prozessor |
| Ohne Fenster laufen lassen | da: `pbo auto` fährt die drei Phasen von Auto auf der Befehlszeile, eine nach der anderen, bis die Kampagne fertig ist |
| Weitermachen nach einem Absturz **von allein** | da, auf Wunsch: `pbo autostart on --continue` (oder im Fenster, Feineinstellung, *Nach Absturz: Weitermachen*): bei der Anmeldung kommen die bewährten Werte zurück, dann macht Auto nach einer Minute Wartezeit (die man mit `pbo resume --cancel` beendet) weiter, höchstens dreimal hintereinander |
| Eine Datei für Fehlermeldungen | `pbo report` schreibt `pbo-report.txt`: Fassung, Werte, Gedächtnis, Machine Checks, Start von Auto, Ende des Protokolls |
| Prüfung auf neuere Fassung | da (`pbo update`, ersetzt nie von selbst) |

## Absichtlich anders

- **Aida64 und Linpack** gibt es nicht und sind nicht geplant: Prime95 und y-cruncher decken den Curve-Optimizer-Fall ab.
- **Der Wert wird nie über 0 gesetzt** (`maxValue` darf bei CoreCycler positiv sein).
- **Weitermachen nach einem Absturz ist eine Wahl** (Feineinstellung, *Nach Absturz*; Voreinstellung *Warten*): die Werte kommen
  bei der Anmeldung in jedem Fall zurück, ein Test läuft aber nur dann von allein weiter, wenn man es eingeschaltet hat, nach einer
  Wartezeit, die man beenden kann, und höchstens dreimal hintereinander (ein Kern, der den Rechner immer wieder abstürzen lässt,
  soll keine Endlosschleife auslösen).
- **Kein Eintrag im Windows-Ereignisprotokoll** und **kein Wiederherstellungspunkt**: das Protokoll steht neben dem Programm,
  und ein Wiederherstellungspunkt ist nichts, was ein Programm ungefragt anlegen sollte.
- **Keine Einstellung der Priorität des Testprogramms**, kein Wechsel der Anhaltemethode: eine Voreinstellung, die sich bewährt hat.
- **Obergrenze 50 Runden** (CoreCycler lässt 10 000 zu); wer mehr will, startet noch einmal.
- **Der Ton gilt nicht für Fehler in einer Suche**: dort ist Durchfallen der Normalfall, und ein Ton bei jedem wäre Lärm.

## Was noch fehlt

1. **Eine Nachricht aufs Telefon** (nur die C#-Fassung kann einen Webhook anstoßen, CoreCycler hat nichts dergleichen): zurückgestellt. Der Systemton und die Taskleiste decken ab, was CoreCycler bietet.
2. **Aida64 und Linpack** als Testprogramme: nicht geplant (siehe oben).
3. **Verzögerung der ersten Fehlerprüfung**: eine Feinheit, die ein Neustart des Testprogramms je Kern ohnehin erledigt.

## Was an echter Hardware gesehen ist

Gesehen an Kern 7 der 5800X3D ([Beleg](../evidence/e5-5800x3d-load-changes.txt)): die **Pausen** und die **Schübe** (mit Prime95 und mit
y-cruncher), der eigene FFT-Bereich in der Einstellungsdatei, und seit der Reparatur des Prime95-Starts **„Auto" im Fenster** (an
Kern 1, ohne Eingriff, beide Phasen). **Nicht belegt:** der Ton (ob er zu hören war, ist offen), das Verhalten bei einem
**echten** Fehler (die Erkennung ist mit dem echten Prime95 und einer eingefügten Fehlerzeile geprüft, nicht mit einem echten
Fehler; Kern 0 hielt −30 zwei Minuten, mit Prime95 und mit y-cruncher, ohne Fehler) und ein Lauf über alle acht Kerne.
Gebaut und getestet sind alle (Zustandslogik,
Pulser mit einem Schalter, der jederzeit freigegeben wird, Plan-Datei, Befehlszeile, Vektoren), aber diese **nicht an der
5800X3D gesehen**. Das Einfrieren und Freigeben selbst ist an einem echten Prozess geprüft (Rechenprozess, ohne Rechte), nicht
mit Prime95 oder y-cruncher. Das steht in `docs/rust/README.md`, bis es geprüft ist.
