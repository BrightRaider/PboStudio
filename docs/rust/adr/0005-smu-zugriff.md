# ADR-0005: SMU-Zugriff — direkt am Treiber, datengetriebene Familientabellen

- **Status:** Vorgeschlagen — Kern von Gate G0 und G4
- **Datum:** 2026-11-04

## Kontext

Die C#-Bibliothek ZenStates-Core (21 000 Zeilen, GPL-3.0) spricht den PawnIO-Treiber **direkt** an: `CreateFile` auf
das Gerät, dann `DeviceIoControl` mit zwei Codes (`0x821 << 2` lädt ein Modul, `0x841 << 2` führt eine Funktion aus,
jeweils mit dem Gerätetyp verknüpft). Es gibt keine zwischengeschaltete DLL. Die Module (`RyzenSMU.bin`,
`AMDFamily17.bin`) sind signierte Binärdateien aus dem offiziellen PawnIO.Modules-Release 0.2.4 (LGPL-2.1).

Gebraucht wird davon nur ein Teil (etwa 3 500 Zeilen in `Cpu.cs`, `RyzenSmu.cs`, `PawnIo.cs`, `SMU.cs`,
`PowerTable.cs`): Prozessorfamilie erkennen, **CO lesen und schreiben**, PM-Tabelle (Version, Inhalt), Topologie aus den
Fuses (`ccdEnableMap`, `coreDisableMap`), PBO-Skalar und Takt.

**Das Wissen steckt in der Bibliothek, auch in ihren Kommentaren.** Ein Beispiel, das ich in der Durchsicht von 1.1.2 erst
beim Lesen des Befehls `SetPsmMarginSingleCore` fand: *„CO margin range from -30 to 30 before Zen 4, and from -50 to 50 on
Zen 4 and newer"*. Das ist keine AMD-Dokumentation, aber die beste verfügbare Quelle, und sie widerspricht dem
ursprünglichen 1.1.0-Stand ebenso wie dem Standardwert für Zen 4 in 1.1.2. Ein Neubau muss solche Stellen **systematisch
einsammeln**, nicht zufällig.

## Entscheidung

1. **Rust spricht den Treiber direkt an** (`windows-sys`, `DeviceIoControl`). Die signierten Module werden unverändert
   mitgeliefert, mit festem SHA-256 und Verweis auf den Release 0.2.4.
2. **`Smu` ist ein Merkmal (Trait)** über einem `Device`-Merkmal. Tests ersetzen das Gerät durch ein **aufgezeichnetes
   Gespräch**: Die C#-Anwendung zeichnet auf der 5800X3D jede Anfrage und Antwort auf; Rust spielt sie ab. So ist die
   SMU-Schicht ohne Hardware prüfbar und gegen die laufende C#-Version abgeglichen.
3. **Familientabelle als Daten, nicht als Code:** `data/families.toml`. Pro Familie: Codename, Mailbox und
   Befehls-IDs für CO lesen/schreiben, Layout der PM-Tabelle (Version → Positionen), Kernlayout, **CO-Tiefe**,
   **`verified`** (`hardware` / `source` / `none`) und **`source`** (Datei und Zeile oder Commit). Ein Eintrag ohne CO-Befehl
   ist ein Schreibverbot, kein Ratespiel.
4. **Die Tabelle wird beim Portieren aus der C#-Bibliothek erzeugt und per Hand geprüft**, einschließlich der Kommentare
   im Quelltext. Jede Zahl behält ihre Herkunft.
5. **Der Prüfstatus ist sichtbar:** im Log, im Bericht und in der Oberfläche. „Ungeprüft" ist ein erlaubter Zustand, „still
   falsch" nicht (Invariante I12).
6. **Rücklesen ist Pflicht** jenseits von −30 (I1), die Kernadressierung kommt aus der Fuse-Maske (I3).

## Alternativen

- **Eine `PawnIOLib.dll` mitliefern und laden.** Zusätzliche Datei, zusätzlicher Angriffsweg, kein Gewinn.
- **Die C#-Bibliothek über FFI aufrufen.** Hebt den Zweck des Wechsels auf (Laufzeit, Größe).
- **Weiter upstream folgen.** Nicht möglich ohne .NET.

## Folgen

- **Wir übernehmen die Pflege für Zen 4 und Zen 5.** Das ist die größte Einzelgefahr des Wechsels ([Plan](../README.md#was-gegen-den-wechsel-spricht)).
- Die Tester aus den Issues sind Teil des Plans, nicht ein Nebeneffekt.
- Neue Prozessoren heißen: eine Zeile in `families.toml`, mit Quelle, zunächst `verified = none`.

## Prüfung

Gate **G0:** Der Prototyp liest auf der 5800X3D CO −20/−25/−30/−30/−28/−30/−30/−30, Temperatur/PPT/TDC/EDC und die 16
APIC-Ids identisch zu C#. Gate **G4:** abgespieltes Gespräch grün; Zen 3 `hardware`; Zen 4 `hardware` durch einen Tester,
sonst ausdrücklich `none`.
