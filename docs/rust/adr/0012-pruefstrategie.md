# ADR-0012: Prüfstrategie

- **Status:** Vorgeschlagen
- **Datum:** 2026-11-04

## Kontext

Das Programm entscheidet über Stabilität von Hardware, aber nur ein Gerät steht zur Verfügung (Zen 3, 8 Kerne). In 1.1.x
sind zwei Fehler an Prozessoren aufgetaucht, die nicht zur Verfügung standen: Telemetrie auf Zen 4 und eine Kernzuordnung, die
auf 12-Kern-Teilen falsch gewesen wäre. Beides lag in Annahmen, die auf dem einen Gerät zufällig stimmten. Ein Test, der den
alten Fehler nicht findet, beweist nichts; in dieser Sitzung wurden mehrere Tests erst durch den Vergleich mit dem **alten**
Code nützlich (und einer war ohne diesen Vergleich wirkungslos).

## Entscheidung

Fünf Ebenen, von billig nach teuer:

1. **Einheitentests** im Kern, ohne Hardware.
2. **Testvektoren** ([ADR-0002](0002-spezifikation-zuerst.md)): dieselben Ein- und Ausgaben für C# und Rust.
3. **Abgespielte Gespräche** für die SMU-Schicht ([ADR-0005](0005-smu-zugriff.md)): aufgezeichnet auf der 5800X3D, in Rust
   wiederholt. Sie prüfen die Rust-Seite gegen die laufende C#-Seite, nicht gegen Annahmen.
4. **Oberflächenvorschau** ([ADR-0009](0009-oberflaeche.md)): jeder Zustand ein Bild; Abweichung ist ein Fehler.
5. **Hardwareprobe** an den Gates G3 und G4: ein vollständiger Schnelltest, eine Absturzprobe (Prozess beenden, Neustart),
   ein Parken und Zurücksetzen. Pro Familie ein Prüfstatus (`hardware` / `source` / `none`).

**Regel:** Jede Invariante (I1–I14) hat einen Test, der **nachweislich gegen eine absichtlich kaputte Implementierung
scheitert.** Der Nachweis steht in der Beschreibung der Änderung.

**Diagnose für Nutzer:** `pbo diag` schreibt eine Datei, die ein Nutzer anhängt: Version, Prozessor, BIOS, Kernzuordnung,
APIC-Tabelle, PM-Tabelle und Prüfstatus der Familie, Log. Die Reporter aus den Issues haben genau das gebraucht, und es
sollte ein Befehl sein, nicht eine Anleitung.

## Alternativen

- Nur Einheitentests. Verworfen: Sie haben die Hardwarefehler nicht gefunden.
- Nur Hardwareproben. Verworfen: zu langsam und auf ein Gerät beschränkt.

## Folgen

- Das **Tester-Programm** wird Teil des Produkts: eine Matrix (Familie × Prüfstatus) und ein offener Aufruf in den Release-Notes.
- Ein Release nennt, was **nicht** geprüft wurde. In 1.1.1 und 1.1.2 war das eine Stärke und bleibt es.

## Prüfung

Die Matrix ist veröffentlicht und stimmt mit `families.toml` überein; jedes Gate hat eine Checkliste mit Ergebnis.
