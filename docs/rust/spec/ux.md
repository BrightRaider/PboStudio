# Bedienkonzept — einfach für jeden, und der Profi kommt mit

Status: **Entwurf zur Durchsicht**, 2026-11-04. Gehört zur Spezifikation ([ADR-0002](../adr/0002-spezifikation-zuerst.md)),
setzt [ADR-0004](../adr/0004-zuerst-ohne-fenster.md) voraus und bestimmt [ADR-0009](../adr/0009-oberflaeche.md) und
[ADR-0011](../adr/0011-texte-und-sprachen.md).

## Anlass

Drei Beobachtungen aus der Benutzung von 1.1.3, die eine gemeinsame Ursache haben:

- Auto testet alle Kerne, auch wenn man nur einen anhakt. Es überschreibt die Auswahl vor jeder Phase.
- Die Marken „stürzte ab bei …" sind immer sichtbar, der Knopf zum Löschen ist nur bei einem bestimmten Tuner-Modus da.
- Im Tests-Tab wählt man **ein** fertiges Paket aus einer Liste, nicht beliebig viele Tests.

Ursache: Die drei Tabs bearbeiten denselben **versteckten Zustand** (Kerne, Profil, Tuner-Modus, Durchgänge), und was auf
dem Bildschirm steht, ist nicht das, was gleich läuft. Mit weiteren Reitern, Karten oder Texten lässt sich das nicht
beheben, es braucht ein anderes Modell.

## Leitgedanke: ein Plan, drei Tiefen

Es gibt **nicht** ein Anfängerprogramm und ein Profiprogramm, und es gibt keinen „Expertenmodus"-Schalter, der das Verhalten
ändert. Es gibt **einen Plan**. Die Tiefe bestimmt nur, wie viel davon man sieht und anfasst. Wer tiefer geht, verliert
nichts und muss nichts neu lernen: Jede Profi-Einstellung ist die Verfeinerung eines Wertes, den der Anfänger schon als
Empfehlung hatte.

## Der Plan

| Teil | Inhalt | Standard |
|---|---|---|
| **Kerne** | welche Kerne, mit ihrem Wert | alle |
| **Tests** | eine **Liste** von Bausteinen in Reihenfolge (Testprogramm, Modus, Anteil der Zeit), beliebig viele | Empfehlung für den Prozessor |
| **Dauer** | pro Kern, Durchgänge | aus der Schätzung |
| **Suche** | aus, grob oder fein; Sicherheitsabstand | Empfehlung |
| **Bei Fehler** | Kern zurücknehmen und weitermachen, oder anhalten | zurücknehmen |
| **Grenzen** | Temperatur, Hardwarefehler, tiefster Wert | sicher |

Der Plan ist eine **lesbare Datei**. Oberfläche und Befehlszeile (`pbo`, [ADR-0004](../adr/0004-zuerst-ohne-fenster.md))
benutzen dieselbe Datei und denselben Kern.

**Auto ist kein eigener Zustand des Programms, sondern ein Verfahren, das den Plan vorausfüllt.** Die drei Phasen
(prüfen, absenken, absichern) sind drei Schritte eines Plans. Daraus folgen Regeln:

1. **Was angezeigt wird, läuft.** Auto zeigt den Plan vor dem Start. Ein Test vergleicht angezeigten und ausgeführten Plan.
2. **Die Empfehlung ist ein Standardwert, nie ein Zwang.** Ändert man etwas, steht dort „angepasst", mit „zurück auf
   Empfehlung" pro Zeile und für alles.
3. **Auto hält sich an die gewählten Kerne**, in allen Phasen. Nichts angehakt heißt alle.
4. **Nichts überschreibt eine Eingabe ohne sichtbaren Hinweis.** Es gibt keinen Zustand, den nur ein Tab kennt.

## Die drei Tiefen

Ein Bildschirm, kein Reiter pro Tiefe.

**Tiefe 1: Einfach.** Eine Entscheidung.

```
  AMD Ryzen 7 5800X3D · 8 Kerne
  Plan: alle Kerne · 3 Phasen · etwa 1 Tag                 [ Anpassen ▾ ]

                        [  Start  ]

  Die Maschine startet dabei eventuell neu. Das ist gewollt, es macht danach weiter.
```

**Tiefe 2: Anpassen.** Der Plan, aufgeklappt. Hier landet, wer nur einen Kern nachtesten oder mehrere Tests
kombinieren will.

```
  Kerne    [x]0  [x]1  [ ]2  [ ]3  ...      jeweils mit Wert, ▲▼
  Tests    1  Prime95 SSE        40 %   [ ⋮ ]
           2  y-cruncher         60 %   [ ⋮ ]       [ + Test ]    Vorlagen ▾
  Dauer    10 min pro Kern · 3 Durchgänge
  Suche    Aus · Grob · [Fein]    Abstand +3
  Bei Fehler  [Kern zurücknehmen]

  2 Werte angepasst                                     [ Zurück auf Empfehlung ]
```

Die heutigen Pakete werden zu **Vorlagen**: Sie füllen die Testliste vor, man kann sie danach ändern.

**Tiefe 3: Experte.** Alles andere, an der Stelle, an die es gehört: je Test FFT-Bereich, AVX-Modus, Threads, Speicher,
y-cruncher-Algorithmen; Grenzen und Schwellen; Webhook; Plan als Datei öffnen, speichern, teilen; Protokoll.

## Was die Tiefen zusammenhält

1. **Dieselben Fakten, andere Auflösung.** Der Anfänger liest „Kern 3 ist stabil bei −27", der Profi schaltet auf Rohwerte
   (Offset, Testlauf, Meldung) um. Beide sehen dasselbe Ergebnis, verschieden genau.
2. **„Warum?" gehört zu jedem Kern.** Eingeklappt: der Verlauf, was wo durchgefallen ist, mit welchem Test, wann. Dort
   steht auch **Vergessen** (heute im Tuner-Panel versteckt) und der Stand der Hardwarefehler (heute ein eigener Knopf).
3. **Ein Ergebnis ist kein Ende.** Nach Auto: „Ins BIOS übernehmen" für den Anfänger, „Als Plan öffnen" für den Profi, der
   damit weitermacht.
4. **Grenzen hebt man einzeln und mit Bestätigung an**, nicht über einen globalen Schalter.
5. **Dateien statt Geheimnisse:** Plan, Ergebnis und Gedächtnis sind lesbare Dateien ([ADR-0008](../adr/0008-persistenz-und-wiederaufnahme.md)).

## Vier Bausteine, die einen Lauf verlässlich machen

Sie ergänzen den Plan an den Stellen, an denen ein Lauf heute still scheitert oder nichts wert ist.

### 1. Bereitschaftsprüfung: „Bereit für die Nacht?"

Vor einem langen Lauf, auf Wunsch und automatisch ab einer einstellbaren Dauer. Eine Liste mit Ampel, die **nichts
ändert**, sondern sagt, was fehlt und was zu tun ist:

- **Autostart-Aufgabe vorhanden?** Heute läuft sie bei der **Anmeldung** (`ONLOGON`) und wendet nur das gespeicherte Profil
  an (`--apply-saved`). Nach einem Absturz bleibt der Rechner am Anmeldebildschirm stehen, wenn kein Auto-Login
  eingerichtet ist, und nichts setzt den Lauf fort. Die Prüfung nennt das beim Namen.
- Schlafmodus und Ruhezustand, Neustart durch Windows-Updates, Speicherplatz, Temperaturreserve.
- Das Programm richtet **Auto-Login nicht selbst ein**: Es ist eine Sicherheitseinstellung. Es weist darauf hin und erklärt es.
- **Der Auslöser bleibt die Anmeldung.** Ein Start schon vor der Anmeldung, mit höchsten Rechten und ohne angemeldeten Nutzer,
  wirkt zu Recht gefährlich und wird nicht voreingestellt. Wenn er überhaupt angeboten wird, dann nur nach einer
  **ausdrücklichen Nachfrage vorher**, die die Folgen nennt, und nie als Standard.

### 2. Selbsttest: „Funktioniert es auf diesem Rechner?"

Ein Knopf, etwa zwei Minuten, ohne Dauerlast. Er liest Prozessor, Kernzuordnung (Fuse-Maske, APIC) und prüft die Messwerte
auf Plausibilität (Temperatur, Leistung, Takt). Er schreibt einen Curve-Optimizer-Wert auf einen Kern, liest ihn zurück und
stellt den alten wieder her, auch jenseits von −30. Das Ergebnis ist eine Zeile je Prüfung (bestanden, durchgefallen, nicht
geprüft) und eine Datei zum Teilen (`pbo diag`, [ADR-0012](../adr/0012-pruefstrategie.md)), samt Prüfstatus der
Prozessorfamilie ([ADR-0005](../adr/0005-smu-zugriff.md)).

Zweck: **Vor einer Nacht erfahren, ob das Programm auf diesem Rechner verlässlich arbeitet**, besonders für Familien, die
nicht auf Hardware geprüft sind (Zen 4, Zen 5). Der alte Wert wird in jedem Fall zurückgeschrieben
([ADR-0006](../adr/0006-prozesssteuerung.md)).

### 3. Nur ungestörte Durchläufe zählen

Ein Durchlauf gilt nur dann als bestanden, wenn er **ungestört** war. Der Lauf misst die Last anderer Prozesse und der
anderen Kerne mit. „Fremd" heißt: alles außer dem Testprogramm, das dieses Programm gestartet hat, und dem Programm selbst.
Der Durchlauf ist **„gestört"**, sobald **eine** dieser Bedingungen erfüllt ist (Startwerte, einstellbar in Tiefe 3):

| Messgröße | Schwelle |
|---|---|
| fremde Rechenzeit auf dem geprüften Kern (beide Threads) | mehr als **5 %** der Dauer des Durchlaufs |
| fremde Last aller anderen Kerne, gleitender Mittelwert über 10 s | über **15 %** |
| ein einzelner anderer Kern, gleitender Mittelwert über 10 s | über **40 %** |

*Herleitung.* Gemessen auf einem Ryzen 7 5800X3D (16 logische Prozessoren), 30 Sekunden normaler Alltag mit Browser,
Entwicklungsumgebung und Sprachchat, im Sekundentakt: Gesamtlast im Mittel **6,0 %**, höchster Sekundenwert **15,3 %**; je
logischem Prozessor im Mittel 1,8 bis 17,2 %, einzelne Sekunden bis 93 %. Die Schwellen liegen mit Abstand über diesem
Alltag und gelten für **gleitende Mittelwerte**, nicht für Spitzen, damit ein kurzer Ausschlag nichts stört. Ein Spiel, das
einzelne Kerne dauerhaft auslastet, sollte darüber liegen; **das ist nicht gemessen**, es lief kein Spiel. Es ist eine
Messung auf einem Rechner: Die Zahlen sind Annahmen mit Abstand, keine gemessenen Grenzen. Deshalb schreibt jeder Durchlauf
die gemessene Fremdlast in das Protokoll, damit sie mit Rückmeldungen von Testern nachgestellt werden können.

Folgen eines gestörten Durchlaufs:

- Er zählt weder als bestanden noch als durchgefallen, erscheint so im Verlauf des Kerns und wird wiederholt.
- Nach **drei gestörten Durchläufen hintereinander** pausiert der Lauf, wartet, bis der Rechner frei ist, und meldet es,
  statt Durchläufe zu verbrennen.
- Ein **Fehler** in einem gestörten Durchlauf bleibt ein Fehler, denn er ist echt. Er wird als „unter fremder Last"
  gekennzeichnet, weil die Ursache unklar ist.
- Tiefe 1 sagt es in einem Satz: „Der Rechner war nicht frei, dieser Durchlauf zählt nicht."

Anlass: ein Test, während man spielt. Takt, Temperatur und Last weichen vom geplanten Zustand ab, und ein Bestehen sagt dann
kaum etwas ([ADR-0007](../adr/0007-fehlererkennung.md)). Heute erkennt das Programm das nicht.

### 4. BIOS-Abgleich

Nach „Ins BIOS übernehmen" merkt sich das Programm das Ergebnis. Beim nächsten Start liest es die Werte, **bevor** das
gespeicherte Profil angewendet wird, und vergleicht: „Das BIOS stimmt mit dem Ergebnis überein", oder eine Liste der Kerne,
die abweichen, mit beiden Werten. Ein Vertipper im UEFI fällt sonst nie auf. Nach einem BIOS-Update steht ein Hinweis, dass
die Werte neu zu prüfen sind.

## Absturz und Weitermachen

Aus Issue 3 (rensus898), als Teil des Plans und nicht als Einzelkorrekturen ([ADR-0008](../adr/0008-persistenz-und-wiederaufnahme.md)):

- Nach einem Absturz schreibt das Programm den **bewährten Wert jedes Kerns** zurück, bevor es weitermacht. Der abgestürzte
  Kern wird zurückgenommen. Heute startet es mit den BIOS-Werten.
- Ein Absturz, den das Programm nur aus dem Windows-Log erschließt, wird **ohne Klick** vermerkt. Heute wird er nur angeboten.
- Ein abgestürzter Kern bleibt aus den Suchschritten draußen und kommt erst beim Absichern wieder dazu.
- **Automatisch weitermachen** nach einem Zähler, der sich abbrechen lässt, mit Obergrenze für Fortsetzungen hintereinander,
  damit ein Kern, der den Rechner immer wieder umwirft, keine Endlosschleife auslöst. Danach: sicherer Endzustand und Meldung.
- Ein Protokoll pro Tag, die Nacht gehört zum ersten Tag, nichts wird automatisch gelöscht.

Die Zahlen (Zähler, Obergrenze, Takt der Messwerttabelle) sind offen und gehen an rensus898 zur Rückmeldung.

## Was jede Gruppe bekommt

- **Einsteiger:** eine Entscheidung bis zum Start, Sätze statt Kürzel, Restzeit, ein Neustart mitten im Lauf ist kein Problem.
- **Profi:** beliebige Testlisten in freier Reihenfolge, jeder Kern einzeln, der Plan als Datei und per Befehlszeile,
  Rohwerte, der volle Verlauf je Kern. Offen: CoreCycler-Konfigurationen lesen (siehe Fragen).

## Wohin die heutigen Teile gehen

| Heute | Künftig |
|---|---|
| Auto-Tab | Tiefe 1 und die Vorbelegung des Plans |
| Tests-Tab, Liste fertiger Pakete | Vorlagen für die Testliste (Tiefe 2) |
| Advanced-Tab | Tiefe 3 |
| Kernliste | Teil des Plans (Kerne) und des Ergebnisses; Details je Kern |
| Gedächtnis zurücksetzen, WHEA zurücksetzen | Details je Kern |
| Autostart-Profil, BIOS-Liste | Aktionen am Ergebnis, dazu BIOS-Abgleich |
| Autostart-Aufgabe (bei Anmeldung) | Bereitschaftsprüfung und Weitermachen nach einem Absturz |
| Einrichtungsassistent | zusätzlich der Selbsttest |

## Offene Fragen

1. **Reihenfolge der Tests:** alle Tests nacheinander auf einem Kern, dann der nächste Kern (so arbeitet CoreCycler), oder
   jeder Test über alle Kerne? Vorschlag: pro Kern, als Option umschaltbar.
2. **Auto als Reiter oder als Zustand des Plans?** Vorschlag: kein Reiter, der Plan zeigt „Empfehlung" als Zustand.
3. **Format der Plan-Datei:** TOML, weil von Hand lesbar und kommentierbar, die 1.x-Dateien sind JSON.
4. **CoreCycler-Konfiguration lesen?** Es würde Profis den Wechsel leicht machen. Umfang ungeklärt.
5. **Wie viel Fachsprache in Tiefe 1?** Vorschlag: „Offset" und „Kern" ja, „WHEA", „APIC", „FFT" erst ab Tiefe 2.
6. ~~Start-Auslöser nach einem Absturz~~ **Entschieden: Anmeldung.** Systemstart höchstens als Angebot mit ausdrücklicher
   Nachfrage vorher, nie als Standard. Ohne Auto-Login (das das Programm nicht einrichtet) bleibt der Rechner nach einem
   Absturz am Anmeldebildschirm stehen. Die Bereitschaftsprüfung sagt das vorher.
7. ~~Schwelle für „gestört"~~ **Startwerte festgelegt** (5 % / 15 % / 40 %, Herleitung oben). Zu prüfen bleibt, ob ein Spiel
   darüber liegt, und wie die Werte auf anderen Rechnern ausfallen. Das Protokoll liefert dafür die Messwerte.
8. **Zahlen für das Weitermachen und die Messwerttabelle** (Zähler, Obergrenze, Takt): Rückmeldung von rensus898 abwarten.

## Prüfstein

- **Neuling:** startet ohne Hilfe mit einer Entscheidung einen sinnvollen Lauf (Gate G5, [Plan](../README.md)).
- **Profi:** bildet eine bestehende CoreCycler-Konfiguration vollständig als Plan ab und teilt einen Plan als Datei.
- **Beide:** angezeigter und ausgeführter Plan sind gleich, automatisch geprüft.
- **Bereitschaft:** Ohne eingerichtetes Weitermachen warnt die Prüfung vor einem langen Lauf, und der Hinweis nennt den
  Anmeldebildschirm.
- **Selbsttest:** Er läuft auf jeder Familie, stellt den alten Wert immer wieder her (auch bei Abbruch) und erzeugt eine Datei.
- **Gestört:** Ein Durchlauf mit simulierter Fremdlast zählt nicht als bestanden, ein Fehler darin bleibt ein Fehler.
- **BIOS-Abgleich:** Er liest vor dem Anwenden des Profils und meldet jede Abweichung mit beiden Werten.
