# ADR-0001: Neubau in Rust statt Aufräumen in C#

- **Status:** Bestätigt nach Gate G0 am 2026-11-05 ([Ergebnis](../stages/E0.md)); Rückfallweg A bleibt für spätere Gates
- **Datum:** 2026-11-04

## Kontext

PboStudio 1.1.2 ist eine .NET-9-/Avalonia-Anwendung. Gemessen: 45 MB Datei, etwa 200 MB Arbeitsspeicher im
Leerlauf, selbstentpackend verpackt. Von rund 11 000 eigenen Zeilen sind etwa 6 700 Oberfläche, davon 4 250
in einer einzigen Code-Behind-Datei, in der Logik und Anzeige vermischt sind. Dort lagen die meisten Fehler der
Durchsicht von 1.1.2. Die Hardwareschicht beruht auf einer eingebundenen C#-Bibliothek (ZenStates-Core,
21 000 Zeilen, davon wird ein Bruchteil gebraucht).

Der Eigentümer hat denselben Wechsel bei einem anderen eigenen Programm schon vollzogen: 72 → 24 MB Arbeitsspeicher,
71 → 14 MB Platte, 0,2 s Start, App 5,9 MB. Die Rust-Toolchain (1.99.1) ist auf dieser Maschine installiert.

## Entscheidung

PboStudio wird in **Rust** neu gebaut, **nach einer Spezifikation** ([ADR-0002](0002-spezifikation-zuerst.md)),
nicht als Übersetzung des C#-Codes. Die Rust-Fassung zählt als 1.x weiter (kein 2.0). Die C#-Linie läuft bis zur
Parität weiter und bekommt ab Gate G3 nur noch Fehlerkorrekturen.

## Alternativen

- **A. C# aufräumen, `MainWindow` zerlegen, NativeAOT.** Billiger und mit geringstem Risiko. Bringt die
  Trennung von Logik und Oberfläche, aber nur einen Teil der Größen- und Startgewinne, weil Avalonia mit
  Reflection-Bindings (heute bewusst so) AOT erschwert. *Das ist der Rückfallweg, falls G0 scheitert.*
- **B. Zeile für Zeile nach Rust übersetzen.** Nimmt die Struktur der Code-Behind-Datei mit und liefert
  C#-Aufbau in Rust-Syntax.
- **C. Weiter wie bisher.** Verworfen: Größe und Speicher sind ein erklärtes Ziel.

## Folgen

- Doppelter Pflegeaufwand zwischen G0 und G3, danach Einfrieren der C#-Linie.
- Die Pflege des Hardwarewissens für Zen 4/5 (bisher upstream) geht an uns ([ADR-0005](0005-smu-zugriff.md)).
- Ein kleines Programm ohne Laufzeitpaket; Drop-Wächter machen „immer zurücksetzen" ([ADR-0006](0006-prozesssteuerung.md))
  zur Eigenschaft der Sprache statt zur Disziplin.

## Prüfung

Gate **G0**: Ein Prototyp liest auf der 5800X3D CO-Werte, Telemetrie und Topologie *identisch* zu C#. Scheitert er,
gilt Alternative A.
