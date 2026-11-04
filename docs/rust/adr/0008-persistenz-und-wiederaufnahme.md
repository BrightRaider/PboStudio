# ADR-0008: Persistenz und Wiederaufnahme

- **Status:** Vorgeschlagen
- **Datum:** 2026-11-04

## Kontext

Der Auto-Durchlauf muss einen Neustart überleben, denn seine zweite Phase sucht gerade den Wert, der die Maschine
umwirft. Dazu schreibt 1.x Dateien nach `runs/`: `settings.json`, `core_knowledge.json`, `campaign.json`, `co_saved.json`,
`whea_seen.json`, `state.json`, `whea_reset.txt`, `logs/`. Nutzer haben diese Dateien, mit Wert: Das Gedächtnis eines
Kerns ist Stunden bis Tage Messarbeit.

## Entscheidung

1. **Die Formate bleiben lesbar.** Die Rust-Fassung liest alle Dateien der C#-Fassung ohne Verlust und schreibt sie zunächst im gleichen
   Format. Jede bekommt ein `schema_version`; ein Format ändert sich nur mit Migration und Test.
2. **Schreiben ist atomar:** in eine Nachbardatei schreiben, auf die Platte zwingen, umbenennen. Ein Absturz mitten im
   Schreiben hinterlässt die alte Datei, nie eine halbe. (C# macht das für Zustand und Profil, nicht für die
   Einstellungen. Das wird einheitlich.)
3. **Ein Lauf, der nicht endete, ist ein Absturz nur, wenn der Rechner danach neu gestartet ist** (I9). Sonst wurde das
   Programm geschlossen, und das wird nicht als instabil im Gedächtnis vermerkt.
4. **Gedächtnis und gelernte Tiefe tragen die BIOS-Version.** Nach einem Firmware-Update werden sie angezeigt, nicht
   stillschweigend angewendet.
5. **Der Absturz-Zustand speichert den Wert, der lief**, nicht den, den der Prozessor nach dem Neustart hält (das war ein
   Fehler im Knopf „Auf −24 entschärfen und fortsetzen").
6. **Einstellungen werden bei jeder Änderung gespeichert**, nicht nur beim Schließen: Ein Absturz ist hier der Normalfall.

## Alternativen

- Eine Datenbank (SQLite). Verworfen: Dateien sind für Nutzer lesbar, kopierbar und in Fehlerberichten beilegbar.
- Neue Formate ohne Migration. Verworfen: vernichtet Messarbeit.

## Folgen

- Testdateien aus echten Läufen werden (bereinigt) zu Fixtures. Die 1.x-Formate sind damit festgeschrieben, solange 1.x
  gelesen werden soll.

## Prüfung

Gate **G6:** Ein Update von der C#- auf die Rust-Fassung behält Einstellungen, Gedächtnis, laufenden Durchlauf und Autostart-Profil.
