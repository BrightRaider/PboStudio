# ADR-0014: Eigenständig zuerst, zusätzlich schnell in ein Gastgeberprogramm umbaubar

- **Status:** Vorgeschlagen — Wunsch des Eigentümers: PboStudio soll sich später mit wenig Arbeit als Modul (nachladbare DLL)
  in ein Gastgeberprogramm einbinden lassen, und es muss **auch eigenständig funktionieren**
- **Datum:** 2026-11-05

## Kontext

Plugin-Systeme nach dem Muster von OBS oder VST haben typischerweise vier Eigenschaften, auf die sich PboStudio einstellt:

1. **Die DLL-Grenze ist klein und stabil:** eine C-Funktionstabelle, über die Nachrichten laufen. Zeichenketten werden geliehen,
   **kein Speicher überquert die Grenze**. Neue Fähigkeiten sind neue Nachrichten, keine neuen Strukturen.
2. **Die Oberfläche ist Beschreibung, nicht Code:** Der Gastgeber zeichnet Seiten, die das Modul beschreibt und an seine
   Einstellungen bindet. Dazu kommen Statuszeilen, Hinweise und Dialoge.
3. **Der Gastgeber läuft oft ohne Administratorrechte.**
4. **Module werden beim Start geladen und im Betrieb nicht ersetzt.**

**Dieses Repository kennt keinen bestimmten Gastgeber.** Es enthält keine Namen, Funktionen, Nachrichten oder Dateipfade eines
solchen Programms und keinen Code, der von einem abhängt. Es stellt nur sicher, dass sich `pbo-app` **ohne Umbau des Kerns**
in ein Modul einbinden lässt. Das Modul selbst, der **Adapter** zur Schnittstelle des Gastgebers, entsteht außerhalb dieses
Repositorys.

Folgerung: PboStudio erfindet keine eigene Gastschnittstelle. Es ist eine **Bibliothek**, die das Modul des Gastgebers
statisch einbindet. Dann gibt es keine ABI-Frage zwischen PboStudio und dem Modul (gleiche Toolchain), und die Grenze nach
außen ist die des Gastgebers.

Zwei Dinge stehen einer Einbettung sonst im Weg:

- **Rechte:** Der SMU-Zugriff braucht Administratorrechte, ein Modul läuft mit den Rechten des Gastgebers.
- **Lizenz:** Der Hardwareteil ist aus GPL-3.0-Quellen abgeleitet (unten).

## Eigenständig zuerst

**`pbo` muss ohne jeden Gastgeber vollständig funktionieren.** Die Einbettung ist ein zusätzlicher Verwendungszweck, kein Teil
des Programms:

- **Eine Datei.** `pbo.exe` ist das Produkt ([ADR-0010](0010-verteilung.md)): eine EXE, nichts wird entpackt, keine zweite
  Datei muss daneben liegen. Sie hat zwei Rollen: die Oberfläche, und mit `--helper` den Helfer mit Rechten.
- **Zwei Betriebsarten, derselbe Code.**
  - **(A) Ein Prozess mit Rechten**, die Hardware im selben Prozess, wie in 1.x (Manifest „als Administrator"). Der einfachste
    Weg und der, den ein Einsteiger sieht.
  - **(B) Oberfläche ohne Rechte, Helfer mit Rechten**, verbunden über eine Pipe. Das ist der Weg für einen Gastgeber und für
    die Absturzbehandlung nach der Anmeldung.

  Die Anwendungsschicht kennt nur ein **Backend-Merkmal**. Ob dahinter ein Kanal im selben Prozess oder die Pipe zum Helfer
  steckt, merken weder Oberfläche noch Kern. **Gebaut wird (A) zuerst** (Phasen 2 bis 5), (B) kommt in Phase 6 dazu, ohne
  dass sich Oberfläche oder Kern ändern.
- **Die Verdrahtung liegt in den Programmen, nicht in `pbo-app`.** `pbo-app` hängt nur von `pbo-core` und dessen Merkmalen ab.
  Das Programm `pbo` steckt `pbo-smu`, `pbo-engines`, `pbo-os` und `pbo-store` an; ein Gast steckt stattdessen den Client für
  den Helfer an. So bleiben die GPL-abgeleiteten Teile aus dem Gast heraus.
- **Prüfstein:** Alle Gates bis G6 laufen ohne Gast. Das CI baut und testet den Workspace ohne jeden Gast.

## Entscheidung

1. **`pbo-app` ist als Bibliothek einbindbar.** Ein Gast baut sein Modul als eigenes Crate (`cdylib`) und bindet `pbo-app`
   statisch ein. Der **Adapter**, die Brücke zwischen den Nachrichten des Gastgebers und den Befehlen und Ereignissen von
   `pbo-app`, gehört dem Gast und liegt nicht in diesem Repository.
2. **Der Einbettungsvertrag von `pbo-app`.** Damit der Adapter klein bleibt und der Umbau schnell geht, bietet `pbo-app`:
   - **Serialisierbare Typen** für Plan, Befehl, Ereignis und Zustand (JSON), mit `schema_version`.
   - Eine **flache Darstellung des Plans** als Schlüssel und Wert, weil Gastgeber Einstellungen oft so speichern und Seiten
     daran binden (`spec/ux.md`).
   - Eine **Statuszeile** und eine **Fortschrittsangabe** (Phase, Kern, Restzeit) als einfache, übersetzte Werte, die ein Gast
     in eine Zeile, einen Hinweis oder eine Überlagerung setzen kann.
   - **Texte nach Schlüssel**, die Sprache wird übergeben ([ADR-0011](0011-texte-und-sprachen.md)).
   - **Keine Prozess-Globals:** kein `static mut`, keine globale Protokollierung, kein `process::exit`; Datenordner, Uhr,
     Protokollsenke und Sprache werden **hineingereicht**.
   - **Threads nur hinter einem Handle**, das beim `Drop` aufräumt und zurücksetzt ([ADR-0006](0006-prozesssteuerung.md)).
     Eine DLL, die mit laufenden Threads entladen oder beendet wird, bringt den Gastgeber zum Absturz.
   - **Fehler als Werte.** `pbo-app` ruft kein `panic!` für Erwartbares auf. Das Abfangen von Panics an der Grenze macht der
     Adapter.
   - **Alles, was die Oberfläche kann, geht über Befehle.** Ein Gast hat keine Oberfläche von PboStudio.
3. **Ein neutraler Beispielgast** in `examples/embed` zeigt den Weg: ein kleines `cdylib` mit **eigener** Funktionstabelle und
   JSON-Nachrichten, mit den Namen von PboStudio (`pbo_…`), nicht denen eines Gastgebers. Er ist die Vorlage für den Umbau:
   Ein Gast ersetzt dort die Namen und Nachrichtenformen durch die seines Gastgebers und lässt den Rest stehen. Die Vorlage
   enthält **nichts**, was die Schnittstelle eines bestimmten Gastgebers abbildet.
4. **Umfang im Gast.** Eine Einstellungsseite für den Plan (Kerne, Tests, Dauer), Knöpfe für Start und Stopp, eine Statuszeile,
   Hinweise bei Ende, Absturz und Abbruch. **Kernliste mit Live-Werten, Graph und Verlauf bleiben im Programm `pbo`**, im Gast
   steht ein Knopf „PboStudio öffnen". Ein Gastgeber braucht dafür keine neuen Oberflächenelemente.
5. **Die Rechte liegen im Helfer.** Der Gast spricht über die Pipe mit ihm, mit denselben Nachrichten wie die Oberfläche von
   `pbo`. Der Gastgeber braucht keine Administratorrechte. Der Helfer wird erhöht gestartet (UAC) und nach einem Absturz bei
   der **Anmeldung** wieder angestoßen (`spec/ux.md`). Er besitzt **Plan und Zustand** und läuft weiter, wenn der Gastgeber
   sich beendet oder neu startet. **Ein Neustart des Gastgebers beendet also keinen laufenden Test.** Der Helfer nimmt **mehrere Fronten gleichzeitig** an,
   etwa das Fenster von `pbo` und das Modul im Gastgeber. Alle sehen denselben Zustand, Befehle werden nacheinander
   verarbeitet, und eine Front kann jederzeit wegbrechen und wiederkommen.
6. **Der Gast arbeitet mit einem installierten PboStudio.** Der Helfer ist dann `pbo.exe --helper` aus der normalen
   Installation, der Gast liefert **keinen** Hardwarecode mit, und fehlt PboStudio, zeigt er einen Knopf zum Herunterladen. So
   landet kein GPL-Code im Aktualisierungsweg des Gastgebers.
7. **Die Schnittstelle von `pbo-app` ist klein und versioniert** (`api_version`). Eine Änderung braucht einen ADR.

## Lizenz (nicht entschieden, vor einer Einbindung zu klären)

Das Repository steht unter GPL-3.0. Der Eigentümer ist der **einzige Autor** im Git-Verlauf, und es gibt keine Pull-Requests
Dritter; der eigene Code (`src/`) gehört ihm allein. **Nicht** ihm gehört, was aus ZenStates-Core stammt (GPL-3.0, anderer
Urheber): `external/` und die daraus abgeleiteten Hardwaredaten.

- **Eigener Code im Gast:** `pbo-core` und `pbo-app` darf der Eigentümer zusätzlich unter anderen Bedingungen nutzen.
  **Voraussetzung:** Bis dahin nimmt PboStudio keine Beiträge Dritter in diese Crates auf, oder nur mit einer Vereinbarung, die
  das erlaubt (Contributor License Agreement). Sonst bleibt der Beitrag GPL.
- **Abgeleitete Hardwaredaten** liegen nur im Helfer (`pbo-smu`, `data/families.toml`, [ADR-0005](0005-smu-zugriff.md)),
  einem getrennten Programm, das nur über die Pipe spricht. Das ist der übliche Weg, GPL-Teile abzutrennen. **Ob das im
  Einzelfall trägt, ist eine Rechtsfrage.**
- Liefert der Gastgeber den Helfer **nicht** mit (Punkt 6), gibt er keinen GPL-Code weiter.
- Alternativ `families.toml` aus eigener Lesung der Hardware neu belegen. Aufwand und Reichweite offen.

Das ist **keine Rechtsberatung** und vor der Einbindung fachkundig zu klären. Der Aufbau hält alle Wege offen, weil der
GPL-Teil getrennt ist.

## Alternativen

- **Eine eigene Gastschnittstelle (DLL) in PboStudio.** Verworfen: Der Gastgeber hat schon eine, eine zweite wäre doppelte
  Arbeit und eine zweite Wahrheit.
- **Seitenbeschreibung als Schnittstelle von PboStudio.** Verworfen: Der Gast bildet auf die Elemente seines Gastgebers ab, das
  ist seine Arbeit.
- **Hardware in der Gast-DLL.** Verworfen: braucht Rechte im Gastgeber und zieht den GPL-Teil ins Modul.
- **Rust-Typen über die DLL-Grenze.** Verworfen: keine stabile ABI.

## Folgen

- Mehr Arbeit am Anfang: das Protokoll des Helfers, der UAC-Start und eine abgesicherte Verbindung (nur der gleiche Nutzer,
  begrenzte Befehle, keine beliebigen Pfade als Argument).
- Die Arbeit des Gasts liegt außerhalb dieses Repositorys: Seiten und Schlüssel, Aktionen, Statuszeile, Hinweise.
- Gewinn: Gastgeber und Oberfläche brauchen keine Rechte, die Angriffsfläche schrumpft, das Programm wirkt weniger gefährlich,
  und ein Neustart des Gastgebers stört keinen Lauf. Die Absturzbehandlung wird einfacher, weil der Helfer den Zustand hält
  ([ADR-0008](0008-persistenz-und-wiederaufnahme.md), `spec/ux.md`).
- Zusatz zu [ADR-0006](0006-prozesssteuerung.md): Die Wächter (`Drop`) leben im Helfer.

## Risiken und Offenlegung

Das Programm ändert Spannungskurven und nutzt einen Kernel-Treiber. Das gilt für `pbo` ebenso wie für ein Modul in einem
Gastgeber, der ein größeres Publikum erreicht, etwa Spieler. Der Grundsatz dafür lautet: **Alles wird einmal klar und
öffentlich gesagt, und die Entscheidung liegt beim Nutzer.** Kein wiederkehrender Warnhinweis, keine Bevormundung.

- **Anti-Cheat.** Nicht systematisch untersucht, aber es liegt eine Beobachtung vor. Der Treiber (PawnIO, signiert, LGPL-2.1,
  [ADR-0005](0005-smu-zugriff.md)) ist ein allgemeiner Hardwarezugriff. Er startet nicht beim Systemstart, sondern **bei
  Bedarf** (Starttyp „bei Bedarf") und bleibt nach dem ersten Zugriff bis zum Neustart geladen.
  **Beobachtung des Eigentümers (2026-11-05):** Auf seinem Spielerechner laufen Treiber und Spiele ohne Beanstandung. Dort war
  PawnIO geladen, und unter anderem Riot Vanguard und Easy Anti-Cheat sind installiert. Das ist **eine Beobachtung auf einem
  Rechner**: Gespielt wurden nach Angabe des Eigentümers unter anderem ARC Raiders und League of Legends. Wie lange der
  Treiber währenddessen geladen war, ist nicht erfasst, und für andere Anti-Cheat-Systeme, Versionen und Rechner sagt sie
  nichts. PboStudio **verspricht keine Verträglichkeit** und sagt öffentlich,
  was geprüft ist und was nicht. Wer das Modul lädt oder `pbo` startet, entscheidet das selbst. Das Modul wird nicht
  vorausgesetzt, ein Gastgeber muss es nicht immer laden.
- **Instabilität und Abstürze** gehören zum Zweck des Programms. Die Schutzmechanismen stehen in den Invarianten (Rücklesen,
  Zurücknehmen, Absturzbehandlung). Erklärt wird es einmal, im Einrichtungsassistenten und in der README, nicht bei jedem Start.
- **Mehr Nutzer, mehr Rückfragen.** Dagegen helfen eine genaue öffentliche Dokumentation, `pbo diag` ([ADR-0012](0012-pruefstrategie.md))
  als Datei, die man anhängt, und der sichtbare Prüfstatus je Prozessorfamilie ([ADR-0005](0005-smu-zugriff.md)).
- **Inhalt der Offenlegung** (Abschnitt „Sicherheit und Verträglichkeit" in der README): was der Treiber ist und woher er
  kommt (Starttyp „bei Bedarf", bis zum Neustart geladen), welche Rechte das Programm braucht und warum, dass Werte live geschrieben und nach einem Neustart verworfen werden,
  was an echter Hardware geprüft ist und was nicht, und was nicht untersucht wurde (Anti-Cheat).

## Prüfung

Gate **G6** bekommt einen Punkt: Der Abschnitt „Sicherheit und Verträglichkeit" steht in der README, **bevor** die erste
Rust-Fassung oder ein Modul veröffentlicht wird.

Gate **G5** bekommt fünf Punkte:

- **`pbo.exe` läuft eigenständig**, in beiden Betriebsarten, ohne irgendeinen Gast. Ein CI-Lauf baut und testet den Workspace
  **ohne** Gast.
- **Der neutrale Beispielgast** (`examples/embed`) wird gebaut und von einem Testprogramm geladen. Es prüft: Befehle, Ereignisse
  und Statuszeile laufen; **Zerstören beendet alle Threads**; ein Panic im Adapter wirft den Lader nicht um.
- Ein **Neustarttest:** Der Gastgeber beendet sich mitten im Lauf (simuliert), der Test läuft im Helfer weiter, und der neue
  Gast findet den Zustand wieder.
- Eine **CI-Prüfung:** In `pbo-core` und `pbo-app` kein `static mut`, kein `process::exit`, keine Initialisierung globaler
  Protokollierung.
- Eine **Prüfung auf Durchsickern:** Vor jedem Commit sucht eine lokale Prüfung (Pre-Commit-Hook beim Eigentümer) im
  Änderungsstand nach Namen und Pfaden, die auf einen bestimmten Gastgeber hindeuten, und bricht ab, wenn sie etwas findet.
  Die **Namensliste liegt nicht im Repository**, sondern nur beim Eigentümer, denn sie wäre selbst ein Verrat.
