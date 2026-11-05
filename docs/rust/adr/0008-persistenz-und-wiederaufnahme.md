# ADR-0008: Persistenz und Wiederaufnahme

- **Status:** Vorgeschlagen
- **Datum:** 2026-11-04

## Kontext

Der Auto-Durchlauf muss einen Neustart überleben, denn seine zweite Phase sucht gerade den Wert, der die Maschine
umwirft. Dazu schreibt 1.x Dateien nach `runs/`: `settings.json`, `core_knowledge.json`, `campaign.json`, `co_saved.json`,
`whea_seen.json`, `whea_reset.txt`, `logs/`. Der Zustand des laufenden Tests, `state.json`, liegt davon getrennt in
`engines/runs/`; die Rust-Fassung legt beides an einen Ort. Nutzer haben diese Dateien, mit Wert: Das Gedächtnis eines
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
7. **Protokoll: eine Datei pro Tag** (`pbo_JJJJ-MM-TT.log`), nicht eine pro Start und keine endlose. Mehrere Starts am
   selben Tag, auch nach Abstürzen, landen in derselben Datei und sind durch eine Startzeile getrennt.
   **Eine Nacht gehört zum ersten Tag.** Wer abends einen Lauf startet und morgens nachsieht, findet alles in der Datei
   des Abends, nicht zerschnitten bei Mitternacht. Dafür gilt:
   - Der Protokolltag endet **nicht um 00:00, sondern um 06:00 Uhr** (einstellbar). Eine Zeile um 03:00 am 6.11. steht
     in der Datei vom 5.11.
   - **Ein laufender Lauf bleibt in seiner Datei, bis er endet**, auch über die Tagesgrenze hinaus. Ein Lauf, der um 22:00
     beginnt und erst um 08:00 endet, wird nicht um 06:00 zerteilt.
   - Ein Neustart nach einem Absturz in derselben Nacht schreibt in dieselbe Datei weiter.

   Jede Datei beginnt mit einem **Kopf** (Version, Prozessor, BIOS, Stand des Auto-Durchlaufs, Wert je Kern), damit sie für
   sich lesbar ist, auch wenn ein Auto-Durchlauf mehrere Nächte dauert und deshalb mehrere Dateien füllt. Jede Zeile trägt
   die Kennung des Laufs, damit sich ein Durchlauf über mehrere Tage herausfiltern lässt. **Es wird nichts automatisch
   gelöscht**: Jemand will ein Protokoll vielleicht
   Monate oder Jahre später ansehen. Aufräumen gibt es nur als **Einstellung, die aus ist** („Protokolle nach N Tagen
   löschen"). Wird eine Tagesdatei sehr groß, beginnt eine Folgedatei (`pbo_JJJJ-MM-TT_2.log`), es wird nichts
   abgeschnitten. Zeile für Zeile auf die Platte geschrieben, damit sie einen Absturz überlebt. (Anlass: Issue 3 von
   rensus898, Wunsch nach einem durchgehenden Protokoll.)

## Alternativen

- Eine Datenbank (SQLite). Verworfen: Dateien sind für Nutzer lesbar, kopierbar und in Fehlerberichten beilegbar.
- Ein einziges, endloses Protokoll. Verworfen: wächst ohne Grenze und ist für Fehlerberichte zu groß.
- Neue Formate ohne Migration. Verworfen: vernichtet Messarbeit.

## Folgen

- Testdateien aus echten Läufen werden (bereinigt) zu Fixtures. Die 1.x-Formate sind damit festgeschrieben, solange 1.x
  gelesen werden soll.

## Prüfung

Gate **G6:** Ein Update von der C#- auf die Rust-Fassung behält Einstellungen, Gedächtnis, laufenden Durchlauf und Autostart-Profil.
