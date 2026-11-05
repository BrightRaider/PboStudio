# ADR-0003: Workspace und Abhängigkeitsregel

- **Status:** Vorgeschlagen
- **Datum:** 2026-11-04

## Kontext

In C# liegen Entscheidungslogik, Zustand und Oberfläche in `MainWindow`. Ein Beispiel für die Folge: Die Tiefe, bis zu der
gesucht wird, wurde an zehn Stellen als Konstante gelesen und hing später plötzlich von Hardware und Nutzer ab. Und ein
Start, der abgebrochen wurde, wurde als Lauf verbucht, weil das nur im Fenster sichtbar war.

## Entscheidung

Ein Cargo-Workspace mit einer **Abhängigkeitsregel**: Pfeile zeigen nur nach unten.

```
pbo-ui        ─┐
pbo-cli       ─┴─►  pbo-app   (Anwendungsfälle, Zustandsmaschine, Befehle rein / Zustand raus)
                      │
        ┌─────────────┼─────────────┬───────────────┐
        ▼             ▼             ▼               ▼
   pbo-engines    pbo-smu       pbo-os         pbo-store
   (Prime95,      (CO, PM-      (Prozess, Job, (Dateien,
    y-cruncher)    Tabellen)     WHEA, CPUID)   Migration)
        └─────────────┴──────┬──────┴───────────────┘
                             ▼
                         pbo-core   (rein: kein Betriebssystem, keine Dateien, keine Zeit, kein Zufall)
```

- **`pbo-core`** hat keine Abhängigkeit außer der Standardbibliothek. Zeit und Zufall kommen als Parameter. Alles dort
  ist ohne Hardware testbar und wird von den Testvektoren geprüft.
- **Hardware und Betriebssystem stehen hinter Merkmalen (Traits)** im Kern: `Smu`, `Engine`, `Clock`, `EventSource`. Tests
  setzen Attrappen ein, so wie die C#-Tests es mit `FakeEngine` tun.
- **Die Oberfläche kennt nur `pbo-app`.** Sie sendet Befehle (`Start`, `Stop`, `SetLimit`) und zeigt Zustand. Sie trifft
  keine Entscheidungen.

## Alternativen

- Ein einziges Crate mit Modulen. Verworfen: Die Regel wird nicht vom Compiler durchgesetzt.
- Viele kleine Crates. Verworfen: Aufwand ohne Gewinn; die obige Aufteilung folgt den echten Grenzen.

## Folgen

- Der Compiler setzt durch, was in C# Disziplin war: kein Betriebssystemaufruf im Kern.
- Mehr Dateien am Anfang; dafür ist die Oberfläche austauschbar ([ADR-0009](0009-oberflaeche.md)).
- **Korrektur zur Zeichnung:** `pbo-app` hängt nur von `pbo-core` und dessen Merkmalen ab. Die vier unteren Crates
  (`pbo-engines`, `pbo-smu`, `pbo-os`, `pbo-store`) werden **in den Programmen angesteckt**, nicht in `pbo-app`. Dadurch kann
  ein anderes Programm dieselbe Anwendungsschicht mit anderen Anschlüssen benutzen.
- Für die Einbettung in ein Gastgeberprogramm bindet dessen Modul (DLL, im privaten Projekt) `pbo-app` als Bibliothek ein, mit
  einem Client für den Helfer. `pbo-core` und `pbo-app` enthalten keine GPL-abgeleiteten Teile. **Das Programm `pbo` braucht
  keinen Gast** und funktioniert eigenständig: [ADR-0014](0014-einbettbar.md).

## Prüfung

`cargo tree -p pbo-core` zeigt keine anderen Abhängigkeiten als `std`. Eine Prüfung im CI schlägt fehl, wenn es anders ist.
