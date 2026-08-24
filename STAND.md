# PboStudio - Projektstand (24. August 2026)

## Übersicht & Zusammenfassung der Funktionen

PboStudio ist ein moderner, performanter Ersatz für CoreCycler in C# (.NET 9) / Avalonia UI für AMD Ryzen Curve Optimizer Tuning.

---

### Fertiggestellte Kernfunktionen & Architektur-Updates:

1. **Low-Level SMU Thread-Safety & Synchronisation (`SmuService.cs`)**:
   - Vollständig thread-sichere Serialisierung aller SMU-Mailbox- und DRAM-PowerTable-Zugriffe (`_smuLock`).
   - Verhindert Kollisionen zwischen dem 2-Sekunden-Telemetrie-Polling und asynchronen Auto-Tuner SMU-Schreibzugriffen.
   - Extraktion des Interfaces **`ISmuService`** und Implementierung von **`MockSmuService`** für 100% isolierte Unit-Tests ohne Hardware-Abhängigkeit.

2. **Native UAC-Sicherheit (`app.manifest`)**:
   - `requestedExecutionLevel` fest auf **`requireAdministrator`** gesetzt.
   - Windows fordert beim Start automatisch die benötigten Ring-0 UAC-Rechte an (Schild-Symbol), wodurch Treiber-Blockaden verhindert werden.

3. **Obsidian Dark Theme als zentrales Design-System (`Styles/Theme.axaml`)**:
   - Alle Farben, Abstände, Radien und Schriftgrößen liegen als Tokens in **einer** Datei; beide Fenster teilen sie sich (vorher pro Fenster dupliziert und bereits auseinandergelaufen).
   - Typo-Skala auf 5 Stufen reduziert (10,5 / 11,5 / 12,5 / 14 / 18), Radien auf 3, Grüntöne auf eine semantische Palette.
   - Inter wird jetzt tatsächlich als UI-Schrift gesetzt; Fluents `SystemAccentColor` ist auf Emerald umgestellt, damit Checkboxen/Fokus nicht mehr Windows-blau sind.
   - **Aufgeräumte 3-Tab Architektur**: `[🎯 Setup]`, `[⚡ Engine]`, `[🛡️ System]` als Segmented Control (`RadioButton`, Keyboard-Navigation inklusive).
   - **Scrollbarer Tab-Body**: Der rechte Bereich konnte seinen eigenen Inhalt bei Standard-Fenstergröße nicht darstellen; Fortschrittsanzeige liegt jetzt außerhalb der Tabs und bleibt in jedem Tab sichtbar.
   - **Sticky Action Footer**: rechts ausschließlich `🚀 Test starten` / `🤖 Auto-Tuner starten` / `⏹ Test abbrechen`.
   - **„CO-Werte anwenden" als ein Block** unter der Kerntabelle: Live-SMU, BIOS-Liste und Windows-Autostart nebeneinander, jeweils mit Kurzerklärung zu Wirkung und Haltbarkeit.
   - **Dynamisches Farbschema**: Grün (#10B981) für statische Stresstests, Lila (#8B5CF6) für den Auto-Tuner.
   - **Multi-CCD Schnellauswahl**: Filter-Buttons werden aus der tatsächlichen Chiplet-Zahl erzeugt (vorher fest zwei, auf Threadripper falsch).
   - **Kerntabelle**: CCD als eigener Chip, Risiko-Farbcodierung des CO-Werts, `▲/▼`-Delta zum Startwert, Fortschrittsbalken bis zum Chip-Limit, Status mit Glyphe **und** Farbe (rot/grün-tauglich).
   - **Quick-Actions**: `[⚡ Alle auf -30]` bzw. `-50` (Label und Wert aus dem echten CPU-Limit, mit Sicherheitsabfrage) und `[🔄 BIOS-Werte]`, das wieder wirklich auf den beim Start gelesenen Wert zurücksetzt.
   - **Sperren während eines Laufs**: alle Parameter, die mitten im Lauf ohnehin wirkungslos wären; das Notfall-Temperaturlimit bleibt bewusst änderbar.
   - Tastatur-Shortcuts: `F5` (Start/Stop), `Ctrl+C` (BIOS Copy), `Ctrl+S` (Live SMU), `Ctrl+L` (DE/EN). `Space` als Start-Trigger wurde entfernt – er löste auf fokussierten ComboBoxen ungewollt Testläufe aus.

3a. **Vollständige Zweisprachigkeit (DE/EN)**:
   - Teststatus, Ergebnis-Panel, Recovery-Banner, Setup-Assistent, Plattform-Check und Protokoll laufen jetzt über `LocalizationService`; vorher waren sie bei englischer Oberfläche deutsch.
   - Sämtliche Tooltips zweisprachig, inklusive Erklärungen zu PPT/TDC/EDC, SMT, FFT, Transient-Pause, CPPC-Kernen und dem Vorzeichen/Betrag-Schema des BIOS.
   - `CpuRecommendationService` erbt die Sprache jetzt aus `LocalizationService` – der Aufrufer hatte das Flag nie gesetzt, wodurch die Empfehlung dauerhaft englisch blieb.

3b. **Live-Telemetrie (`TelemetryGraphControl.cs`)**:
   - Drei getrennte Lanes (Takt / Temperatur / PPT) statt dreier überlagerter Kurven auf einer gemeinsamen normierten Achse.
   - Beschriftete Y-Skalen pro Lane, X-Zeitachse, 15-Minuten-Fenster (vorher 2), rechtsbündig scrollend.
   - Das eingestellte Notfall-Temperaturlimit wird als gestrichelte Linie eingezeichnet.
   - Hover-Tooltip mit echtem Zeitstempel; Leerzustand erklärt fehlende Telemetrie, statt ein schwarzes Rechteck zu zeigen.
   - Kopfzeile zeigt PPT/TDC/EDC/Temp als Chips mit Auslastungsbalken statt einer Monospace-Zeile, die bei vielen Kernen abgeschnitten wurde.

3c. **Behobene Fehler aus dem UI-Review**:
   - `SmuService.MaskFor` teilte durch null, sobald die Topologie 0 Kerne meldete (ohne PawnIO/Admin) – die App stürzte beim Start ab, statt in den reinen Stresstest-Modus zu wechseln.
   - Der Telemetrie-Graph war unsichtbar: als Geschwister im `DockPanel` bekam er implizit `Dock=Left` und damit Breite 0.
   - `AcceptButton` wurde von „BIOS-Liste kopieren" und vom Plattform-Check dauerhaft ausgeblendet und nie wieder eingeblendet – nach einem fehlgeschlagenen Lauf fehlte die Übernahme-Schaltfläche.
   - `SetRunning` überschrieb Beschriftung, Sprache und Auto-Tuner-Zustand des Start-Buttons und setzte einen abweichenden Grünton.
   - `[⚡ All -30]` setzte auf Zen 5 tatsächlich -50; Label und Tooltip widersprachen dem Verhalten.
   - `Ctrl+C` wurde global abgefangen und verhinderte das Kopieren markierter Protokolltexte.
   - Plattform-Check und Abhängigkeitsprüfung liefen synchron im Konstruktor (WMI, Datei-IO, SMU) und verzögerten den ersten Frame; jetzt asynchron nach dem Rendern.
   - Fehlende Komponenten werden **vor** dem Klick auf „Start" als Banner gemeldet, der Start-Button ist bis dahin deaktiviert – vorher scheiterte der Lauf stumm und der Assistent sprang unvermittelt auf.
   - Protokoll hängt Zeilen als `Run`-Inlines an (vorher `Text +=`, also eine vollständige Kopie pro Zeile) und färbt nach Schweregrad.

3d. **Einstellungen, Kernqualität und Werkzeugkette**:
   - **Persistenz**: Sprache, Profil, sämtliche Engine-Parameter, Temperaturlimit, Webhook und Fenstergröße liegen in `runs/settings.json` und werden beim Start wiederhergestellt. Zuvor überlebte nur `co_saved.json` einen Neustart.
   - **Echte CPPC-Rangfolge**: `CoreQualityService` liest die vom Prozessor gemeldeten CPPC-Leistungswerte (MSR `0xC00102B3`, über `ISmuService.CorePerformanceRanking`) statt Kern 0 pauschal als „Gold" zu deklarieren. Meldet die CPU nichts, wird **kein** Abzeichen vergeben — die alte Heuristik zeigte erfundene Daten als Messwert.
   - **Nachgerüstete Schalter**: „Durchgefallenen Kern überspringen", „WHEA-Warnung als Fehler werten" und „Pause zwischen Kernen" existierten nur als fest verdrahtete Vorgaben in `TestPlan` und sind jetzt bedienbar.
   - **Versionskontrolle**: Das Projekt ist ein Git-Repository (Version 0.9.0). Der gepatchte ZenStates-Core-Fork ist vendert, seine Abweichung von Upstream liegt als `external/zenstates-local-patches.diff` bei.
   - **Handbücher** (`docs/BENUTZER_HANDBUCH.md`, `docs/USER_MANUAL.md`) beschreiben wieder die tatsächliche Oberfläche.

4. **Automatisierter Auto-Tuner Regelkreis & Adaptive Staging (`AutoTunerService.cs`)**:
   - **Top-Down & Bottom-Up Suche**: Wähle zwischen Grob (-3 Schritte) und Fein (-1 Schritt).
   - **Hardware- & BIOS-Limit Erkennung**:
     - *Ryzen 5000 (Zen 3) & Ryzen 7000/8000 (Zen 4)*: Standard-Limit fest auf **`-30`** gekoppelt.
     - *Ryzen 9000 (Zen 5)*: Erweitertes Limit bis **`-50`** (Curve Shaper Range).
   - **Zweiseitige Suche (Abstieg *und* Aufstieg)**:
     - *Bestanden, oberhalb des Limits:* nächster Schritt nach unten.
     - *Bestanden am Limit:* fixiert (`🔒`) — der Grenzwert ist belegt.
     - *Durchgefallen mit bekannt gutem Wert:* zurück auf den zuletzt bestandenen Wert und fixiert. Kein erneuter Test nötig, dieser Wert hat bereits einen vollen Slot überstanden.
     - *Durchgefallen ohne je bestandenen Wert* (z. B. Start direkt auf `-30`): Spannung wird angehoben und **erneut getestet**, Schritt für Schritt, bis ein Wert tatsächlich hält. Erst dieser wird fixiert.
     - *Durchgefallen bei `0`:* Lauf für diesen Kern beendet mit dem Hinweis, dass die Ursache nicht am Curve Optimizer liegt (RAM/EXPO, Kühlung, BIOS-Version).
     - Damit trägt kein fixierter Wert mehr das Etikett „fertig", ohne getestet worden zu sein. Zuvor wurde nach einem Fehler pauschal `+3`/`+2` addiert und sofort gesperrt — dieser Wert war eine Vermutung.
   - **Werte werden vor der Messung gesetzt**: `TestPlan.ApplyMargin` schreibt den geplanten Wert per SMU, unmittelbar bevor der Kern belastet wird. Vorher rechnete der Planer mit einem Wert, den die CPU nie hatte.
   - **Status-basierte Restdauer-Berechnung**: Berechnet die echte Maximaldauer in Echtzeit unter Ausschluss gelockter Kerne und basierend auf den Reststufen bis zum Limit.

5. **Multi-Engine Hybrid Pipeline & Stresstest Profile (`TestProfiles.cs`)**:
   - **🏆 Hybrid Ultimate (Prime95 SSE + y-cruncher)**: Führt vollautomatisch nacheinander Prime95 SSE (für Peak-Boost & Vdroop Limits) und y-cruncher CO-Preset (für schwere AVX-Rechenlast & Cache-Stabilität) aus.
   - **Prime95 Thread-Confinement & PinnedProcess (`Prime95Engine.cs`)**:
     - Start im `CREATE_SUSPENDED` Zustand, Einbindung in das `CoreJail` Job Object und Setzen der Affinitätsmaske vor `ResumeThread`.
     - Worker-Threads können zu keinem Zeitpunkt aus der Kern-Affinität ausbrechen.
     - Unterstützung für **AVX-512** (Zen 4 / Zen 5) und y-cruncher Algorithmenprofile (VT3, SFT, SVT, FFT, N63).
     - Konfigurierbare Lastwechsel-Pausen (Intervall/Dauer) für Transienten-/Vdroop-Stresstests.
     - Flexible Kern-Reihenfolge: `Sequential`, `Alternate (Thermal Hopping)`, `Random`.

6. **Speicher- und UI-Stabilität bei Langzeittests**:
   - Ringpuffer-Trimming im Log-Fenster (schützt vor Speicherfragmentierung bei 16+ Stunden Overnight-Runs).
   - 60-Sekunden Telemetriegraph mit dynamischer Y-Achsenskalierung und Mouse-Hover Crosshair mit Tooltip.

7. **Atomare BSOD-Persistenz & Profil-Wiederherstellung (`TestStateService.cs`)**:
   - Atomares Schreiben via `WriteThrough` + erzwungenem Flush + `File.Move(overwrite: true)`.
   - Schützt den Test- und Autostart-Zustand zuverlässig vor Dateikorruption bei abrupten Bluescreens.

8. **Automatisierte Unit-Testsuite (`PboStudio.Tests`)**:
   - **30 automatisierte Tests (100% bestanden)** für Auto-Tuner, SMU-Mocking, CPU-Erkennung, WHEA-Watcher, State-Persistenz, Profile, Stress-Kernel und HTML-Reports.

---

### Verzeichnisstruktur (Clean Release):
- `dist/`: Veröffentlichungsbereite Binaries (`PboStudio.exe`, `engines/`)
- `src/`: Quellcode (`PboStudio.App`, `PboStudio.Core`)
- `tests/`: Testsuite (`PboStudio.Tests`)
- `tools/`: Hilfsskripte
- `docs/`: Dokumentation & Benutzer-Handbuch
