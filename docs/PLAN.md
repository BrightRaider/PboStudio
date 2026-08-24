# PboStudio — Plan

## Ziel

Was CoreCycler kann, in einer bedienbaren Oberfläche, plus einen Curve-Optimizer-Editor,
der die Werte live liest und schreibt. Eine EXE, herunterladen und starten. Sie führt ein
verständliches Protokoll und sagt am Ende, welchen Wert man nehmen sollte.

Kein Fork von CoreCycler und kein übernommener Code — nur derselbe Zweck.

## Was das Tool können muss

### Testen

- Engines: **Prime95** und **y-cruncher**. AIDA64 fällt raus (kostenpflichtig, Lizenz nötig),
  Linpack ist ein Kandidat für später.
- Prime95: Befehlssatz (SSE / AVX / AVX2 / AVX512) und FFT-Bereich wählbar, inklusive
  eigener Min/Max-Grenzen.
- y-cruncher: Algorithmenauswahl (BKT, SFT, SNT, SVT, FFT, N63, VT3 …), Testdauer, Speicher.
- Last auf **einen physischen Kern** festnageln, wahlweise ein oder zwei Threads.
- Laufzeit pro Kern, Anzahl Durchläufe, Kerne überspringen, Reihenfolge (der Reihe nach,
  abwechselnd, zufällig, eigene Liste).
- Zwischen den Kernen pausieren, Testprogramm pro Kern neu starten.
- Last periodisch kurz aussetzen — findet Fehler, die nur bei Lastwechseln auftreten.

### Fehler erkennen

Ein Kern gilt als durchgefallen bei:

- Rechenfehler, den die Engine selbst meldet (Prime95 `results.txt`, y-cruncher-Ausgabe)
- abgestürztem oder verschwundenem Testprozess
- **WHEA**-Einträgen im Windows-Ereignisprotokoll (Warnungen zählbar auf Wunsch)
- Kern läuft plötzlich im Leerlauf, obwohl er unter Last stehen sollte

Reaktion konfigurierbar: sofort anhalten, Kern künftig überspringen, Signalton, Fenster blinken.

### Curve Optimizer und PBO

- **Lesen:** CO-Wert je Kern, PBO-Grenzen (PPT/TDC/EDC), Scalar, Boost-Override.
- **Schreiben:** CO je Kern und für alle Kerne. PBO-Grenzen dort, wo die CPU es zulässt —
  bei X3D-Modellen sind sie fest verdrahtet, das zeigt die Oberfläche dann ehrlich an.
- **Wichtig:** Über die SMU gesetzte Werte gelten nur bis zum Neustart. Das Tool bietet
  daher an, sie beim Anmelden automatisch wieder zu setzen — und zeigt am Ende eine
  Übersicht zum Abtippen ins BIOS.

#### Gemessen am 5800X3D (Vermeer, SMU 00384C00), nicht vermutet

Schreiben und Zurücklesen funktionieren: `-5` gesetzt, `-5` gelesen, wieder auf `0`.

Aber der SMU-Lesebefehl meldet **ausschließlich, was zur Laufzeit über die SMU geschrieben
wurde** — nicht das, was AGESA beim Booten aus den BIOS-Einstellungen angewandt hat. Ein
Kern, an dem das Tool noch nichts gesetzt hat, liest `0`, egal was im BIOS steht.

Daraus folgt für die Oberfläche:

- Nie `0` anzeigen, wo in Wahrheit „unbekannt" gilt. Ein Kern ohne eigene Schreiboperation
  bekommt `—`, nicht `0`.
- Beim Start erklären: entweder CO im BIOS auf 0 stellen und alles hier machen, oder die
  BIOS-Werte einmalig als Startwerte eintragen.
- Vor dem ersten Schreiben warnen, dass ein gesetzter Wert die BIOS-Einstellung dieses
  Kerns bis zum Neustart ersetzt.

Ebenfalls gemessen: PBO-Grenzen (PPT/TDC/EDC) antworten beim X3D nicht — gefused, wie
erwartet. Scalar und Boost-Takt (4550 MHz) kommen korrekt.

#### Auslesen funktioniert — die Nullen kamen vom BIOS

Erster Messwert war `0` auf allen Kernen, obwohl im BIOS `-20 / -25 / -30` gesetzt waren.
Die naheliegende Erklärung — `Zen3Settings` erbt von `Zen2Settings` einen Lesebefehl, der
dort als „Not sure" markiert ist — war **falsch**.

Tatsächliche Ursache: Das Board hatte die Curve-Optimizer-Werte angewandt, ohne sie je an
die SMU zu übergeben. In diesem Zustand sieht sie kein Programm unter Windows, auch Ryzen
Master nicht. Behoben durch: im BIOS deaktivieren, speichern, neu starten, wieder
aktivieren, speichern, neu starten. Danach liest das Tool exakt `-20 / -25 / -30 …`.

Konsequenz für die Oberfläche: Lesen wird uneingeschränkt genutzt. Melden aber **alle**
Kerne `0`, ist das mehrdeutig — entweder ist nichts gesetzt, oder das Board steckt im
beschriebenen Zustand. Statt einer selbstbewussten Null zeigt die App dann den Hinweis mit
der Abfolge zum Beheben.

Lehre fürs Projekt: Ein plausibler Code-Fund ist kein Beweis. Erst der Gegentest an echter
Hardware entscheidet.

### Automatik

Auf Wunsch sucht das Tool die Werte selbst: Startwerte übernehmen (oder die aktuellen
auslesen), bei Fehler den Wert eines Kerns um einen Schritt entschärfen, denselben Kern
erneut testen, bis er hält oder die Grenze erreicht ist.

Weil dabei zwangsläufig Abstürze auftreten — das ist die Methode, nicht ein Fehler:

- Fortschritt nach **jedem** Schritt auf Platte, Schreibcache leeren
- nach einem Absturz automatisch weitermachen (geplanter Task beim Anmelden), der
  Wert, bei dem es krachte, gilt als zu scharf
- vorher **Wiederherstellungspunkt** anbieten

### Protokoll und Empfehlung

- Lesbares Protokoll: welcher Kern, wie lange, womit, was passiert ist.
- Verlaufsansicht je Kern statt Textwüste.
- Auswertung am Ende: pro Kern der letzte stabile Wert, plus Empfehlung mit
  Sicherheitsabstand — ein Wert, der im Test gerade so hält, hält im Alltag nicht.

## Voraussetzungen

Nicht verhandelbar, weil Ring-0:

- **Administratorrechte**, bei jedem Start.
- **PawnIO** — signierter Treiber, den die App beim ersten Start selbst installieren kann.
  Bewusst nicht WinRing0 oder inpoutx64: unsigniert und von Defender als
  Sicherheitsrisiko gemeldet.
- Curve Optimizer nur auf AMD Ryzen. Testen geht auf jeder CPU.

Alles Weitere holt sich die App selbst: die Engines werden beim ersten Start
heruntergeladen, nicht mitgeliefert.

## Reihenfolge

**0 — Beweis.** PboProbe elevated laufen lassen: liest sie die CO-Werte des 5800X3D?
Danach ein vorsichtiger Schreibversuch. Ohne das ist alles Weitere Spekulation.

**1 — Testmaschine.** Prozesssteuerung, Kern-Anheftung, Rotation, Fehlererkennung, Protokoll.
Das ist das Herzstück und der größte Brocken. Läuft ohne Oberfläche, gegen die Konsole prüfbar.

**2 — SMU-Schicht.** CO und PBO lesen und schreiben, mit Grenzen und Rücksetzknopf.

**3 — Oberfläche.** Avalonia. Kerntabelle mit CO-Wert und Status, Teststeuerung, Live-Protokoll.

**4 — Automatik.** Suchschleife, Absturzsicherung, Fortsetzen, Wiederherstellungspunkt.

**5 — Auslieferung.** Erststart-Assistent (Treiber, Engines), Native AOT, eine EXE.
Ohne Signatur meldet sich SmartScreen — bekannt aus BrightRaider.

## Offen

- Name. `PboStudio` ist ein Platzhalter.
- Der 5800X3D taugt nur begrenzt als Prüfstand: CO schreiben geht, PBO-Grenzen sind gefused.
  Für den Rest brauchen wir Rückmeldung von fremder Hardware.
