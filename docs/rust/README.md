# PboStudio in Rust — Plan

Status: **Vorschlag**, Stand 2026-11-04. Jede Entscheidung steht als ADR in [`adr/`](adr/).
Die Rust-Fassung zählt als 1.x weiter (kein 2.0, Entscheidung des Eigentümers); die C#-Linie läuft bis zur Parität
weiter und bekommt ab Gate G3 nur noch Fehlerkorrekturen.

## Die Frage

> Wechsel zu Rust — oder nur die Funktionen aufschreiben und dort von vorn beginnen?

**Empfehlung: Spezifikation zuerst, dann Neubau in Rust. Den C#-Code nicht übersetzen, aber seine
Prüfungen mitnehmen.** „Funktionen aufschreiben" allein reicht nicht, denn die Funktionen sind das
Einfache. Verloren ginge, *was schiefgehen kann*: in der Durchsicht von 1.1.2 lag fast alles Wichtige
in Fehlern, die nirgends als Funktion standen — ein Kern, der besteht, ohne getestet worden zu sein;
eine Fehlerzeile, die über die Lesegrenze fällt; Kerne, die nach einem Abbruch auf 0 bleiben. Das
steht heute nur in Tests und Kommentaren. Die Spezifikation besteht deshalb aus **ausführbaren
Testvektoren**, nicht aus Text ([ADR-0002](adr/0002-spezifikation-zuerst.md)).

### Drei Wege im Vergleich

| | A. C# weiter, aufräumen + NativeAOT | B. Zeile für Zeile nach Rust | **C. Spezifikation, dann Neubau** |
|---|---|---|---|
| Größe / Speicher / Start | teilweise; Avalonia und Reflection-Bindings bremsen AOT | ja | **ja** |
| Fehlerklasse „Logik im Fenster" | bleibt, bis `MainWindow` zerlegt ist | wandert mit nach Rust | **fällt weg, weil die Grenze von Anfang an da ist** |
| Regressionsrisiko | klein | mittel | **klein, wenn die Vektoren vorher stehen** |
| Aufwand | klein | groß | groß |
| Hardwarewissen (Zen 4/5) | bleibt, Bibliothek wird upstream gepflegt | geht verloren, wird übersetzt | **geht verloren, wird gezielt neu belegt** ([ADR-0005](adr/0005-smu-zugriff.md)) |

Warum nicht A: Es ist der billigste Weg und der, den ich wählen würde, wenn Größe und Start keine Rolle
spielten. Sie spielen eine Rolle, und es gibt Belege dafür aus einem eigenen Projekt: Ein anderes Programm des
Eigentümers ist mit dem Wechsel auf Rust von **72 auf 24 MB Arbeitsspeicher**, von **71 auf 14 MB** auf der Platte und auf
**0,2 s Start** gekommen (App 5,9 MB). PboStudio liegt heute bei **45 MB Datei und etwa 200 MB
Arbeitsspeicher**. Dazu kommt, dass ein selbstentpackendes .NET-Paket mit Treiber und
Administratorrechten genau das Profil ist, bei dem Virenscanner anspringen (bei einem kleinen, unsignierten Programm des
Eigentümers: 2 von 69 Engines).

Warum nicht B: 60 % des Anwendungscodes (6 700 von 11 000 Zeilen) sind Avalonia-spezifisch und müssen
ohnehin neu geschrieben werden. 4 250 davon sind **eine** Datei, in der Logik und Oberfläche vermischt
sind. In ihr lagen die meisten Fehler dieser Durchsicht (Doppelstart, Wiederherstellungsknopf,
Einstellungen, Schließen mitten im Lauf). Eine Übersetzung nähme diese Struktur mit.

### Was gegen den Wechsel spricht

Ehrlich gesagt ist **die größte Gefahr nicht die Sprache, sondern das Hardwarewissen.** Ihr einziges
Testgerät ist eine Zen-3-CPU. Die C#-Bibliothek trägt das Wissen für Zen 4 und 5 (Mailbox-Adressen,
Befehle, Tabellenlayouts), und das wird upstream gepflegt. Ein Wechsel behebt dieses Problem nicht, er
verschärft es, denn wir übernehmen die Pflege. Der 1.1.1-Fehler (Temperatur 0 auf Zen 4) war genau diese
Klasse. Der Plan begegnet dem mit einem Prototyp **vor** jeder Zusage (Gate G0), mit datengetriebenen
Familientabellen samt Herkunft und Prüfstatus und mit den Testern, die sich in den Issues schon
angeboten haben.

## Ziele

1. **CoreCycler in schön, einfach, besser.** Die Produktentscheidungen aus 1.1 bleiben das Produkt:
   drei Tabs (Auto / Tests / Erweitert), eine Laufzeit über dem Startknopf, Erklärtexte in Tooltips, eine
   Empfehlung statt vierzehn Profilen. Sie werden in Phase 1 als `docs/spec/ux.md` festgehalten.
2. **„Bestanden" heißt „getestet".** Die Invarianten unten sind der eigentliche Wert.
3. **Messbar klein.** Zielwerte, abgeleitet von dem anderen Rust-Programm des Eigentümers, nicht versprochen: EXE ≤ 10 MB,
   Arbeitsspeicher im Leerlauf ≤ 60 MB, bereit in ≤ 1 s.
4. **Auch ohne Fenster nutzbar** (CLI) — für Tests, für Fortgeschrittene und für Fehlerberichte.
5. **Ehrlich über Prüfstatus:** pro Prozessorfamilie sichtbar, ob sie an echter Hardware geprüft wurde.

**Nicht-Ziele:** neue Funktionen vor der Parität; Linux; Aida64 (kostenpflichtig); ein eigener Treiber.

## Invarianten, die der Neubau halten muss

Aus der Durchsicht von 1.1.2. Jede hat heute einen C#-Test und bekommt einen Testvektor (Phase 1).

| # | Invariante | Heute geprüft durch |
|---|---|---|
| I1 | Ein Kern besteht nur, wenn der getestete Wert nachweislich anlag. Jenseits −30 wird zurückgelesen; abgelehnt ⇒ letzter bewährter Wert, sonst überspringen. | `RunnerSafetyTests` |
| I2 | Fehlerzeilen werden nur vollständig gelesen. | `LogTailTests` |
| I3 | Kernadressierung kommt aus der Fuse-Maske. Passt sie nicht zu Windows, wird das gemeldet, nicht geraten. | `SmuCoreMapTests` |
| I4 | Ein Machine Check wird mit der **gemessenen** APIC-Tabelle zugeordnet. Unklar ⇒ zählt gegen den getesteten Kern (neu: heute umgekehrt). | `ApicTopologyTests` |
| I5 | Geparkte Kerne werden immer zurückgesetzt, wie auch immer der Lauf endet. In Rust: Drop-Guard. | `RunnerSafetyTests` |
| I6 | Der Temperatur-Notschutz ist nie still inaktiv. | `PmTableLayoutTests` |
| I7 | Angezeigt wird, was der Prozessor hält, nicht was getippt wurde. | Apply-live-Pfad |
| I8 | Downloads nur über https von der Herstellerdomain; der Treiber-Installer gegen einen festen SHA-256. | `DownloadPolicyTests` |
| I9 | Zustandsdateien werden atomar geschrieben; ein Absturz des Rechners ist nicht dasselbe wie ein geschlossenes Programm (Bootzeit). | `InterruptedRunTests` |
| I10 | Zeiten kennen Tage (kein Umbruch bei 24 h). | `DurationFormatTests` |
| I11 | Schätzungen sind Obergrenzen und werden aus dem Zustand gerechnet. | `CampaignEstimatorTests` |
| I12 | Nichts Unbekanntes erscheint als Messwert: unbekannte Tabelle ⇒ „n/a", nie 0. | `PmTableLayoutTests` |
| I13 | Der Auto-Durchlauf endet (Stillstandserkennung, Abbruchkriterium). | `CampaignSimulationTests` |
| I14 | Tiefe = Standard ∪ Beleg des Prozessors ∪ Nutzerwunsch; zurückgeholt, wenn ein Schreiben nicht hält; pro BIOS gemerkt. | `CurveOptimizerRangeTests` |

## Phasen und Ausstiegskriterien

Aufwand nur relativ (S/M/L), bewusst ohne Termine. Jede Phase endet an einem **Gate**: Besteht es nicht,
wird angehalten und neu entschieden, nicht weitergebaut.

| Phase | Inhalt | Aufwand | Gate |
|---|---|---|---|
| **0. Prototyp** | Rust-Programm auf der 5800X3D: PawnIO öffnen, Module laden; CO aller Kerne lesen; PM-Tabelle (Temperatur/PPT); Topologie und APIC-Tabelle; Kindprozess im Job-Objekt mit Affinität starten und die Auslastung messen. | S | **G0:** gelesene Werte *identisch* zur C#-Ausgabe (CO −20/−25/−30/−30/−28/−30/−30/−30, Telemetrie, 16 APIC-Ids), Datei < 3 MB. **Nicht bestanden ⇒ Plan wird zu Weg A.** |
| **1. Spezifikation** | Die 453 Testfälle und die Invarianten werden zu sprachneutralen Testvektoren (JSON). C# führt sie aus. UX-Spezifikation schreiben. | M | **G1:** C# besteht alle Vektoren; Lückenliste der Invarianten ohne Vektor ist leer. |
| **2. Kern** | `pbo-core` rein (ohne Betriebssystem): Empfehlung, Auto-Durchlauf, Schätzer, Tuner, Gedächtnis, Tiefe, Runner-Zustandsmaschine mit Attrappen-Engine; Dateiformate kompatibel zu 1.x. | M | **G2:** alle Vektoren in Rust grün; ein simulierter Durchlauf endet wie in C#. |
| **3. Windows-Schicht** | Prozess/Job/Affinität, Engines, Log-Lesen, WHEA, Topologie, Downloads, CLI. | L | **G3:** ein vollständiger Schnelltest per CLI auf der 5800X3D mit gleichem Ergebnis wie 1.x, plus Absturzprobe (Prozess beenden ⇒ korrekte Wiederaufnahme). **Ab hier ist C# 1.x nur noch Fehlerkorrekturen.** |
| **4. SMU** | CO lesen/schreiben mit Rücklesen, PM-Tabellen pro Familie datengetrieben, Prüfstatus. | L (Risiko) | **G4:** Zen 3 geprüft (Ihr Gerät); Zen 4 durch einen Tester **oder** als „ungeprüft" ausgeliefert und so beschriftet. |
| **5. Oberfläche** | Die drei Tabs, Kernliste, Telemetrie, Assistent. Toolkit nach [ADR-0009](adr/0009-oberflaeche.md). | L | **G5:** jemand, der das Programm nie gesehen hat, startet ohne Hilfe einen sinnvollen Lauf. |
| **6. Verteilung** | Eine EXE, Aktualisierung, Engines als geprüfte Module, Import von `runs/`. Beta der Rust-Fassung als eigener Kanal. | M | **G6:** Update von der C#- auf die Rust-Fassung behält Einstellungen, Gedächtnis und laufenden Durchlauf. |

## Risiken

| Risiko | Wirkung | Gegenmaßnahme |
|---|---|---|
| Hardwarewissen Zen 4/5 geht verloren | falsche Telemetrie, falsche Kernzuordnung | Prototyp G0; datengetriebene Tabellen mit Herkunft; Prüfstatus sichtbar; Tester aus den Issues |
| Regression „bestanden, ohne getestet" | der schlimmste Fehler, den das Programm haben kann | Invarianten als Vektoren *vor* dem Neubau; Hardwareprobe an Gate G3 |
| Zwei Codebasen | doppelte Arbeit, Auseinanderlaufen | C# ab G3 eingefroren; gemeinsame Vektoren halten beide ehrlich |
| egui hat keine fertige Kernliste mit Zahlenfeldern und keinen Graphen | neue Bausteine nötig, schwächere Barrierefreiheit | Bausteine früh in Phase 5 mit Vorschau bauen ([ADR-0009](adr/0009-oberflaeche.md)); Aussehen selbst ist schon fertig |
| Virenscanner | Fehlalarme bei Treiber + Administrator | kleine EXE, **keine Code-Signatur** (Entscheidung des Eigentümers), Prüfsummen im Release, Einreichung bei Anbietern |
| Lizenz | abgeleitete Daten aus GPL-Quellen | GPL-3.0 beibehalten, Herkunft vermerken ([ADR-0013](adr/0013-lizenz-und-herkunft.md)) |

## Parität: was in der Rust-Fassung vorhanden sein muss

Gegenüber CoreCycler und gegenüber PboStudio 1.1.2. Nichts davon ist neu; es ist die Messlatte.

- **Engines:** Prime95 (SSE/AVX/AVX2/AVX-512, alle FFT-Bereiche, eigener Bereich, Speicher), y-cruncher
  (Algorithmen, Dauer, Speicher, Binärdatei). Last auf einen Kern festgenagelt, 1 oder 2 Threads, ein
  Thread über beide SMT-Geschwister.
- **Ablauf:** Laufzeit pro Kern (auch automatisch), Durchgänge, Reihenfolge (der Reihe nach, abwechselnd,
  zufällig, eigene Liste, Kernpaare), Pausen, Mikro-Lastwechsel, nur den getesteten Kern belasten,
  Priorität, Wiederaufnahme.
- **Fehlererkennung:** Rechenfehler der Engine, Prozess weg, Machine Check (WHEA), Kern ohne Last;
  Temperatur-Notschutz; Absturzwiederherstellung.
- **Tuner:** Suche mit Sicherheitsabstand, Gedächtnis pro Kern, Tiefe nach Beleg, Auto-Durchlauf in drei
  Phasen, Schätzung der Dauer.
- **Werte:** live lesen und schreiben mit Rücklesen, BIOS-Liste, Autostart-Profil mit Prüfung.
- **Drumherum:** Plattformprüfung, Wiederherstellungspunkt, Webhook, Protokoll, Bericht, Deutsch/Englisch.

## Offene Fragen

1. **~~Welches Oberflächen-Toolkit?~~ Beantwortet:** egui/eframe 0.29.1 mit demselben Aussehen wie die anderen Programme des Eigentümers, als wiedererkennbare Handschrift, aber **ohne** Quelltext aus deren privaten Projekten:
   Farben, Schrift und Maße kommen als Werte in eine Design-Beschreibung, `pbo-ui` wird neu geschrieben
   ([ADR-0009](adr/0009-oberflaeche.md), [ADR-0013](adr/0013-lizenz-und-herkunft.md)).
2. **~~Code-Signatur?~~ Entschieden: keine.** Die Folge ist bekannt: Ein Programm, das einen Kernel-Treiber lädt und
   Administratorrechte verlangt, wird bei SmartScreen und Virenscannern weiter auffallen. Dagegen bleiben
   Prüfsummen im Release, Einreichung bei Anbietern und ein Abschnitt „Sicherheit" ([ADR-0010](adr/0010-verteilung.md)).
3. **Zen 4 und Zen 5:** Wer testet? Die beiden Reporter haben sich angeboten.
4. **~~Versionierung?~~ Entschieden: 1.x weiterzählen**, kein 2.0.
