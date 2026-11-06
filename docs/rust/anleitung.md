# Anleitung: PboStudio 1.2.0-beta.2 (Rust)

Für den, der das Programm benutzt. Wie es gebaut ist und was geprüft wurde, steht in [README.md](README.md); jeder Befehl
mit seinen Schaltern in [`rust/README.md`](../../rust/README.md).

## Was es tut

Es sucht für jeden Kern eines AMD-Ryzen-Prozessors den tiefsten Wert des **Curve Optimizers**, der unter Last noch fehlerfrei
rechnet. Dafür belastet es einen Kern nach dem anderen mit Prime95 oder y-cruncher, schreibt den Wert, wartet, ob ein
Rechenfehler oder ein Machine Check (WHEA) kommt, und geht Schritt für Schritt weiter. Geprüft ist es an einem Ryzen 7 5800X3D
(Zen 3); **Zen 4 und Zen 5 sind nicht gemessen** und im Programm als „ungeprüft" gekennzeichnet.

## Einrichten (einmal)

1. **Administratorrechte:** Die Prozessorregister sind nur damit erreichbar. Das Fenster fragt beim Start einmal danach (Windows zeigt die Abfrage) und läuft dann mit den Rechten weiter, ohne dass noch einmal gefragt wird. Wird abgelehnt, öffnet es ohne Rechte und bietet an, einen Helfer mit Rechten zu starten oder sich neu zu starten; mit der Umgebungsvariable `PBO_NO_ELEVATE` fragt es gar nicht. Die Befehle der Kommandozeile fragen nicht von selbst: dort das Terminal als Administrator öffnen.
2. **Treiber:** `pbo setup pawnio` startet den Installer des Treibers PawnIO (unverändert, geprüft gegen die Prüfsumme dieser
   Fassung; Windows fragt nach Erlaubnis).
3. **Testprogramme:** `pbo setup all` holt Prime95 und y-cruncher von der Seite des Herstellers (nur https, Prüfsumme muss
   stimmen) in den Ordner `engines` neben dem Programm. Ohne Netz: `pbo setup prime95 --from ARCHIV`. Ein anderer Ordner:
   Umgebungsvariable `PBO_ENGINES`.
4. **Prüfen:** `pbo ready` sagt, was für eine Nacht ohne Aufsicht noch fehlt (Anmeldeaufgabe, automatische Anmeldung, Ruhezustand,
   Speicherplatz, Temperatur). `pbo diag` schreibt `pbo-diag.txt`, hilfreich, wenn etwas nicht erkannt wird.

## Das Fenster

`pbo` ohne Befehl öffnet es. Oben die Messwerte mit ihren Grenzen; links die Kerne (Auswahl, Wert mit Pfeilen, Tiefe, was gehalten
hat und was durchgefallen ist); rechts drei Reiter.

- **Auto:** Programm starten, *Auto*, **Start**, mehr ist nicht nötig. Oben die drei Stufen *Schnell*, *Ausgewogen*, *Gründlich*
  (die Länge der Tests), jede mit der Zeit im ungünstigsten Fall und einer Zeile, was sie bedeutet. Darunter die drei Schritte
  von Auto und der Knopf. Alles andere ist **vorgewählt, passend zu deinem Prozessor**. Wer selbst eingreifen will, klickt unten
  auf **Feineinstellung**; der Knopf verschwindet, und es erscheinen drei Zeilen (mit **Zurück** oben rechts schließt man sie wieder): **Schritt** (wie weit der Wert pro Test sinkt,
  1 bis 5 Zähler), **Sicherheit** (der Abstand, den der Kern am Ende zu dem Wert behält, der noch hielt, 1 bis 5) und **Start bei**
  (wo die Suche bei einem Kern ohne Wert beginnt: −10, −15 oder −20; ohne Wirkung, solange jeder Kern schon einen Wert hat). Dazu **Nach Absturz** (*Warten* oder *Weitermachen*, siehe "Über Nacht").
  Vorgewählt sind *Schnell*, Schritt 5, Sicherheit 3 und der Start des Prozessors (−20 bei X3D, sonst −15); bei *Gründlich* ist
  der Schritt 1. Die Zeiten rechnen mit deiner Wahl. Auto läuft in drei Schritten: prüfen, was die aktuellen Werte halten; jeden
  Kern absenken, bis er nachgibt; das Ergebnis unter jeder Lastart beweisen. Es geht in **beide Richtungen**: ein Kern, der auf
  einem durchgefallenen Wert steht, wird wieder hinauf gesucht. Man kann jederzeit anhalten und später an derselben Stelle
  fortsetzen.
- **Tests:** vierzehn fertige Testlisten (Empfohlen, Schnellcheck, Grenzwert, Volllast, Über Nacht, y-cruncher, Micro-Bursts …), je mit Zeit und Programm.
- **Erweitert:** was getestet wird (Programm, Befehlssatz, FFT-Bereich auch frei, Zeit auch „Auto", Runden, Threads, die
  y-cruncher-Algorithmen) und wie es läuft (Lastwechsel *Pausen* oder *Schübe*, Reihenfolge der Kerne, Suche grob oder fein,
  Verhalten bei einem Fehler, Ton). Darunter steht dasselbe als Befehlszeile; *Speichern* und *Laden* sichern den Plan als Datei.

Unten: **Live anwenden** schreibt die geänderten Werte jetzt (bis zum Neustart). **Alle auf das Limit** trägt für jeden Kern die
tiefste Grenze ein und **Werte vom Start** das, was beim Öffnen galt; geschrieben wird erst mit *Live anwenden*. **BIOS-Liste** zeigt die Werte zum Eintragen im UEFI ([siehe unten](#werte-ins-bios-eintragen)); am Ende eines Laufs steht der Knopf
**Ins BIOS eintragen …** neben „Fertig" und öffnet dieselbe Liste. **Bei jedem Windows-Start** sichert die Werte, die gerade
gelten, und legt eine geplante Aufgabe an (`pbo resume`), die sie bei der Anmeldung schreibt; war ein Lauf unterbrochen, meldet
sie das und schreibt statt dessen die bewährten Werte.

## Wie lange dauert Auto, und warum

Die Zahl auf den drei Kacheln ist der **ungünstigste Fall** für die gewählten Kerne und besteht aus drei Teilen:

| Phase | Was | Wie viel |
|---|---|---|
| 1 Prüfen | jeder Kern mit **zwei** Tests bei dem Wert, den er gerade hält | Kerne × 2 × Minuten (Schnell 2, Ausgewogen 3, Gründlich 6) |
| 2 Suche | nur Kerne, die noch tiefer können (mehr als der Sicherheitsabstand vom Limit entfernt) | je Kern: (Weg bis zum Limit ÷ Schritt + 1) Tests × Minuten (3, 5, 6) |
| 3 Beweis | jeder Kern mit **vier** Lasten (Prime95 SSE, AVX, AVX2, y-cruncher) | Kerne × 4 × Minuten (3, 6, 10) |

- **Phase 1 und 3 laufen immer ganz durch**, solange kein Kern durchfällt. Sie hängen nicht davon ab, wie tief ein Kern schon steht.
  Ein Kern auf −30 spart nur die Suche; er braucht den Beweis eher mehr, weil nichts anderes zeigt, dass er hält.
- **Phase 2 ist wirklich nur „höchstens":** gerechnet wird, dass jeder suchende Kern bis zum Limit sinkt. Ein Kern, der noch keinen Wert hält, beginnt beim gewählten Start (−20 bei X3D, sonst −15), nicht bei 0. Ein Kern, der bei −20
  durchfällt, hört dort auf. Auch ein Kern, der durchfällt, wird nicht bis nach oben zurückgesucht, sondern geht auf den letzten
  Wert, der hielt, plus Sicherheitsabstand.
- Bei **acht Kernen** und Schnell sind Phase 1 und 3 zusammen **2,1 Stunden**, bei Ausgewogen 4, bei Gründlich 6,9. Mehr als diese
  Zeit lässt sich mit Auto nicht sparen; ist man schon dort, bleibt nur ein anderer Test (Reiter *Tests*).
- Ändert man **Schritt** oder **Sicherheit**, rechnen die Kacheln neu.

## Der schnellste Weg zu einem guten Ergebnis

Nichts davon ist Handarbeit: **Auto starten und die Vorauswahl lassen.** Sie ist schon der schnelle Weg, passend zum Prozessor:
*Schnell*, Schritt 5, Sicherheit 3, Start bei −20 (X3D) oder −15 (alle anderen). Wer sie ändern will, öffnet die *Feineinstellung*; man muss aber nicht.

Es spart an drei Stellen:

- **Der Startwert.** Ein Kern, der noch keinen Wert hält, beginnt nicht bei 0, sondern beim gewählten Start. Die Zeitanzeige
  rechnet damit. Ein tieferer Start spart im Normalfall Tests; im ungünstigsten Fall (der Kern fällt gleich beim ersten durch und
  muss ganz hinauf) kostet er etwas mehr, deshalb zeigt die Zeit für −20 eine Spur mehr als für −15.
- **Fällt ein Kern durch, kümmert sich Auto darum.** Er geht auf den letzten Wert zurück, der hielt, plus Sicherheitsabstand;
  hat noch nichts gehalten, sucht Auto ihn Schritt für Schritt hinauf. Mit *Fortsetzen* läuft es dort weiter, wo es stand.
- **Der Schritt:** 5 statt 3 spart rund ein Fünftel der Gesamtzeit, und der Sicherheitsabstand fängt ab, dass er gröber ist.

Zeit im ungünstigsten Fall mit der Vorauswahl, acht Kerne, an denen noch nie etwas eingestellt wurde:

| | Schnell | Ausgewogen | Gründlich (Schritt 1) |
|---|---|---|---|
| X3D (Start −20) | **4,1 h** | 7,3 h | 23,7 h |
| alle anderen (Start −15) | **3,7 h** | 6,7 h | 19,7 h |

Davon sind **2,1 Stunden fest** (Phase 1 und 3 bei Schnell); die Suche in Phase 2 ist der Rest und nur der ungünstigste Fall: jeder
Kern hört an seiner Grenze auf.

**Danach:**

1. **Erst benutzen, dann festschreiben.** Ein paar Stunden bis Tage mit dem, was du wirklich tust (Spiele, Arbeit). Erst danach
   ins BIOS eintragen oder „Bei jedem Windows-Start" einschalten. *Werte vom Start* bleiben als Rückweg.
2. **Stürzt ein Kern im Alltag ab,** weiß das Programm es nicht von selbst. `pbo whea` zeigt, welchen Kern Windows nennt. Wähle
   diesen Kern aus und prüfe ihn im Reiter *Tests* mit *Micro-Bursts* (leichte, wechselnde Last findet Kerne, die unter Volllast
   halten und im Alltag nicht). Fällt er durch, wird der Wert vermerkt, und *Fortsetzen* in Auto setzt den Kern neu. Es muss kein
   Wert von Hand geändert werden.
3. **Gründlich oder Schritt 1 nur, wenn sich die letzten ein, zwei Zähler lohnen:** es kostet ein Mehrfaches, der Gewinn je Zähler
   wird kleiner und die Absturzgefahr größer.

Was **nicht** hilft: ein kürzerer Test als 2 bis 3 Minuten (er findet instabile Kerne nicht zuverlässig) und das Weglassen der
Prüfung unter echter Last.

## Werte ins BIOS eintragen

Die Werte, die das Programm schreibt, gelten bis zum Neustart. Dauerhaft sind sie nur im UEFI (oder mit der Anmeldeaufgabe).
Öffne dazu die **BIOS-Liste** (unten) oder, am Ende eines Laufs, **Ins BIOS eintragen …**:

1. In das UEFI starten (meist mit der Taste Entf beim Hochfahren).
2. *Advanced*, *AMD Overclocking*, *Precision Boost Overdrive*, *Curve Optimizer* (die Namen sind je nach Mainboard-Hersteller
   etwas anders).
3. *Curve Optimizer* auf **Per Core** stellen.
4. Für jeden Kern **Sign: Negative** und **Magnitude** gleich der Zahl ohne Minus (−27 wird 27). Die Liste im Programm schreibt
   dazu „(Negativ, 27)".
5. Speichern und neu starten. Im Programm bei **Werte vom Start** ansehen: Es müssen dieselben Zahlen bei denselben Kernen
   stehen. **Wie die Kerne gezählt werden, kann je Mainboard anders sein**; stimmt es nicht, sind zwei Kerne vertauscht.

Das UEFI nimmt Werte nur in dem Bereich an, den der Prozessor erlaubt (bei Zen 3 bis −30).

## Über Nacht und ohne Fenster

- **`pbo auto`** macht auf der Befehlszeile dasselbe wie Auto im Fenster: prüfen, suchen, beweisen, eine Phase nach der anderen, bis
  die Kampagne fertig ist (`--intensity fast|normal|thorough`, dazu `--step`, `--guardband`, `--start`, `--cores`). Es muss
  als Administrator laufen wie alle Befehle.
- **Selbst weitermachen nach einem Absturz** ist ausgeschaltet, solange man es nicht einschaltet: im Fenster in der
  Feineinstellung *Nach Absturz: Weitermachen*, oder einmal `pbo autostart on --continue` (beides legt die Anmeldeaufgabe an).
  Dann geschieht nach einem Absturz Folgendes: der Rechner startet, bei der Anmeldung kommen die **bewährten Werte** zurück, das
  Programm sagt, welcher Kern abgestürzt ist, und **nach einer Minute** macht Auto dort weiter, wo es stand. Die Minute lässt sich mit
  `pbo resume --cancel` beenden. **Höchstens dreimal hintereinander:** ein Kern, der den Rechner immer wieder abstürzen lässt,
  soll keine Endlosschleife auslösen; danach wartet das Programm auf dich (`pbo whea` und `pbo knowledge` zeigen, was los war).
- Damit das Ganze nachts ohne dich geht, muss der Rechner sich nach dem Neustart **selbst anmelden** und nicht schlafen gehen:
  `pbo ready` prüft das (Anmeldeaufgabe, automatische Anmeldung, Ruhezustand, Speicherplatz, Temperatur).
- **Was an der Hardware noch nicht gesehen ist:** das Weitermachen nach einem *echten* Absturz. Die Teile (Werte zurück, Zähler,
  Wartezeit, Auto ohne Fenster) sind gebaut und geprüft, aber ein echter Absturz mit Neustart steht noch aus.
- **`pbo report`** schreibt eine Datei `pbo-report.txt` neben das Programm: Fassung, was der Prozessor hält, was das Programm sich
  gemerkt hat, die Machine Checks der letzten 90 Tage, wie Auto gestartet wurde und das Ende des Protokolls. Sie ist für
  Fehlermeldungen gedacht; sieh sie dir an, bevor du sie anhängst (sie nennt Prozessor und BIOS-Version).
- Kleinigkeiten für Tests: `--pause N` (Sekunden Ruhe zwischen zwei Kernen, damit der nächste kühl startet), `--yc-tests A,B,C`
  und `--yc-memory MB` (die Tests von y-cruncher und sein Speicher je logischem Prozessor).

## Was das Programm auf den Prozessor schreibt

Nur die Curve-Optimizer-Werte der Kerne, jeden mit **Rücklesen**; nie einen Wert über 0; bei Zen 3 nie tiefer als −30. Bricht man
einen Lauf ab, stellt das Programm die Werte wieder her. Ein Test der gehaltenen Werte (`pbo run`) schreibt nichts.

## Nach einem Absturz

Ein Absturz mitten im Lauf ist ein erwartetes Ergebnis, kein Fehler des Programms. Bei der Anmeldung (wenn die Aufgabe angelegt ist, sonst beim
nächsten Lauf oder mit `pbo resume`) vermerkt das Programm den Wert, der lief, als durchgefallen, schreibt die bewährten Werte zurück
(`pbo resume`) und nennt die Machine Checks, die Windows für den Absturz aufgezeichnet hat, mit dem Kern, den jeder nennt.
`pbo whea` listet die ganze Vorgeschichte, auch Abschalter ohne Machine Check; es sagt auch, wenn das Systemprotokoll geleert wurde.

## Dateien

Alles liegt im Ordner `runs\` neben dem Programm: `core_knowledge.json` (was je Kern gehalten hat und durchgefallen ist, mit der
BIOS-Version), `co_saved.json` (die gesicherten Werte), `settings.json` und `logs\` (ein Protokoll je Tag, der Tag endet um
06:00; **nichts wird von selbst gelöscht**). `pbo import` übernimmt, was eine 1.x-Fassung hinterlassen hat.

## Grenzen, ehrlich

- **Kein Fehler wurde bisher an echter Hardware erzwungen**: die Erkennung ist mit dem echten Prime95 und einer eingefügten
  Fehlerzeile geprüft und mit einem Eintrag in einem eigenen Protokoll, nicht mit einem echten Ausfall eines Kerns.
- Ein Test, der besteht, beweist nur, dass es **in diesem Test** hielt. Ein Spiel mit leichter, wechselnder Last kann einen
  Kern anders fordern; deshalb gibt es Lastwechsel und das Sicherheitsabstand-Maß („Abstand", Voreinstellung 3 Schritte über dem,
  was gehalten hat).
- Das Programm ist **nicht signiert**. `pbo version` und `pbo hash DATEI` zeigen die Prüfsumme, mit der man ein Original von einer
  Kopie unterscheidet.
