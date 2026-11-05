# ADR-0010: Verteilung — eine kleine EXE, geprüfte Module

- **Status:** Vorgeschlagen
- **Datum:** 2026-11-04

## Kontext

PboStudio 1.1.2 ist eine selbstentpackende .NET-Datei von 45 MB, die einen Kernel-Treiber nachlädt und Administratorrechte
verlangt. Ein anderes, bereits nach Rust umgestelltes Programm des Eigentümers ist dagegen eine EXE unter 6 MB, ohne
Installation, in 0,2 s bereit und mit Aktualisierung „mit einem Klick". Dessen Verfahren kann wegen der Trennung der
Repositories ([ADR-0009](0009-oberflaeche.md)) nicht übernommen werden; die Aktualisierung wird für PboStudio neu entworfen.
Als Anforderung bleibt: ein Klick, mit Prüfsumme.

## Entscheidung

1. **Eine EXE, nichts wird entpackt.** Ein Gastgeber ([ADR-0014](0014-einbettbar.md)) bindet `pbo-app` in sein eigenes Modul ein; im
   Release von PboStudio gibt es dafür kein zusätzliches Artefakt, und `pbo.exe` braucht keinen Gast. Zielwerte (abgeleitet, nicht versprochen): ≤ 10 MB, ≤ 60 MB Arbeitsspeicher im
   Leerlauf, bereit in ≤ 1 s. Profil: `lto`, `codegen-units = 1`, `strip`. **`panic` bleibt `unwind`** ([ADR-0006](0006-prozesssteuerung.md));
   die Größe wird mit beiden Einstellungen gemessen, und die Entscheidung folgt der Messung.
2. **Engines (Prime95, y-cruncher) sind Module:** nur über https, nur von der Herstellerdomain, jedes mit festem SHA-256 für die
   festgenagelte Version. Wird eine neuere gefunden, ist sie ein ausdrücklicher Schritt („neue Version gefunden, übernehmen?"),
   nie stillschweigend. (I8)
3. **Der PawnIO-Installer und die Module haben feste Prüfsummen** und werden vor dem Start geprüft, auch die gebündelte Kopie.
4. **Aktualisierung:** eine Prüfung gegen die Release-Seite mit Anzeige und Prüfsumme und einem Klick zum Übernehmen; kein
   stilles Ersetzen eines Programms, das Administratorrechte hat.
5. **Offline-Paket** bleibt als ZIP: Programm, Treiber-Installer, beide Engines. Die Weitergabebedingungen der Engines werden
   vor der Rust-Fassung geprüft und dokumentiert.
6. **Keine Code-Signatur** (Entscheidung des Eigentümers). Die Folge wird in Kauf genommen: Ein Programm mit Treiber und
   Administratorrechten fällt bei SmartScreen und Virenscannern weiter auf (ein vergleichbares kleines, unsigniertes Programm des
   Eigentümers: 2 von 69 Engines; bei PboStudio ist das Profil auffälliger). Dagegen: Prüfsummen im Release, Einreichung der Datei bei
   Microsoft und den Anbietern, die anschlagen, ein Abschnitt „Sicherheit" in der README und eine Zeile in den Release-Notes,
   wie man die Prüfsumme prüft.
7. **Autostart-Aufgabe** zeigt in ihrer Beschreibung, dass sie die EXE mit höchsten Rechten startet: Wer die Datei ersetzen
   kann, hat beim nächsten Anmelden Administratorrechte. Der Installationsort wird geprüft und gewarnt, wenn er für normale
   Nutzer beschreibbar ist.

## Alternativen

- Weiter selbstentpackend verpacken. Verworfen: Größe, Start, Virenscanner.
- Ein Installer (MSI). Verworfen: „herunterladen und starten" ist ein Ziel des ursprünglichen Plans (`docs/PLAN.md`).

## Folgen

- Das Verteilungswerkzeug gehört zum Produkt und wird früh gebaut, nicht zuletzt.

## Prüfung

Gate **G6:** Größe, Speicher und Start gemessen und im Release angegeben; Prüfsummen aller Dateien im Release;
Update von 1.x ohne Verlust.
