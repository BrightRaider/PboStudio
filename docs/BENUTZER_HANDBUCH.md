# 📘 PboStudio — Das Ultimative Handbuch & Dokumentation

> **PBO Studio**: Die All-in-One Tuning & Stabilitäts-Suite für AMD Ryzen Curve Optimizer, PBO Telemetrie und Per-Core Stresstesting.

---

## 📋 Inhaltsverzeichnis
1. [Über das Projekt](#1-über-das-projekt)
2. [Voraussetzungen & Treiber (PawnIO)](#2-voraussetzungen--treiber-pawnio)
3. [Die Hauptansicht (UI & 3-Tab Architektur)](#3-die-hauptansicht-ui--3-tab-architektur)
4. [Test-Profile & Hybrid-Strategie (Prime95 + y-cruncher)](#4-test-profile--hybrid-strategie-prime95--y-cruncher)
5. [Praxis-Leitfaden: Wie teste ich meine CPU am besten?](#5-praxis-leitfaden-wie-teste-ich-meine-cpu-am-besten)
6. [System Health Audit (EXPO & BIOS Status)](#6-system-health-audit-expo--bios-status)
7. [WHEA Wächter & Smart Empfehlungen (Grob vs. Fein)](#7-whea-wächter--smart-empfehlungen-grob-vs-fein)
8. [Der Auto-Tuner (Top-Down, Adaptive Staging & Limits)](#8-der-auto-tuner-top-down-adaptive-staging--limits)
9. [Werte live anwenden vs. BIOS-Übernahme](#9-werte-live-anwenden-vs-bios-übernahme)
10. [Stabilitäts-Bericht (HTML Report Exporter)](#10-stabilitäts-bericht-html-report-exporter)
11. [Absturzsicherheit & BSOD Recovery](#11-absturzsicherheit--bsod-recovery)
12. [Häufige Fragen (FAQ)](#12-häufige-fragen-faq)

---

## 1. Über das Projekt
**PboStudio** vereint echtes Einzelkern-Stresstesting (bekannt aus CoreCycler) mit der direkten Live-Steuerung des AMD Curve Optimizers über die **SMU (System Management Unit)** in einer einzigen modernen Desktop-Anwendung (.NET 9 / Avalonia UI).

Du musst keine unleserlichen `.ini`-Dateien oder PowerShell-Skripte mehr manuell editieren – Testen, Fehler analysieren, SMU-Register schreiben und BIOS-Listen erstellen geschehen in Echtzeit mit 1 Klick.

---

## 2. Voraussetzungen & Treiber (PawnIO)
- **Betriebssystem**: Windows 10 / 11 (64-Bit)
- **Prozessor**: AMD Ryzen (Zen 2, Zen 3, Zen 4, Zen 5 – inkl. X3D Modelle)
- **Rechte**: Administrator-Rechte (erforderlich für SMU Ring-0 Hardwarekommunikation)
- **Kernel-Treiber**: **PawnIO** (Offiziell signierter Windows-Treiber, kein unsicheres WinRing0/inpoutx64). Über den eingebauten *⚙️ Treiber & Engines Manager* kann PawnIO mit 1 Klick installiert werden.

---

## 3. Die Hauptansicht (UI & 3-Tab Architektur)
Das Hauptfenster ist in übersichtliche Funktionsbereiche gegliedert:

- **Kopfzeile**:
  - **Oben links**: Logo und erkannte CPU. Darunter der `🛡️ WÄCHTER` und die Live-Telemetrie als einzelne Chips (TAKT, PPT, TDC, EDC, TEMP, SCALAR), jeweils mit Auslastungsbalken gegen das Limit.
  - **Oben rechts**: Sprachumschalter, klickbarer Plattform-Check (`🔍 System OK · 95/100`) und `⚙️ Treiber & Engines`.
  - **Setup-Banner**: Fehlen Prime95, y-cruncher oder PawnIO, steht das hier — **bevor** du auf Start drückst. Der Start-Button ist bis dahin deaktiviert.
- **Linke Spalte (Kern-Tabelle & Empfehlungen)**:
  - **Empfehlungskarte**: WHEA-Korrekturvorschläge mit **`⏩ Grob (+4)`** vs. **`🎯 Fein (+2)`**, `⚡ Werte übernehmen`, `🧹 WHEA zurücksetzen` und dem empfohlenen Profil samt `Profil laden`.
  - **Tabelle**: Alle physischen Kerne mit
    - CCD-Chip, sofern die CPU mehr als ein Chiplet hat (aus der Topologie ermittelt, nicht geraten),
    - 🥇/🥈 für die bevorzugten Kerne — **nur** wenn der Prozessor tatsächlich CPPC-Rangwerte meldet; der Tooltip nennt den Messwert,
    - farbcodiertem CO-Wert (grün → gelb → orange → rot, je näher am Chip-Limit),
    - `▲/▼`-Delta gegenüber dem BIOS-Ausgangswert (bernstein = noch nicht angewendet, gedämpft = in der CPU aktiv),
    - Status mit Symbol **und** Farbe (`○ Bereit`, `● läuft`, `✓ bestanden`, `✕ FEHLER`, `🔒 Limit`).
  - **Quick-Actions im Tabellenkopf**:
    - **`[⚡ Alle auf -30]`** (bzw. `-50` bei Zen 5): setzt alle ausgewählten Kerne auf das Chip-Limit. Fragt vorher nach — dieser Wert ist auf kaum einer CPU stabil.
    - **`[🔄 BIOS-Werte]`**: zurück auf die Werte, die beim Programmstart aus der CPU gelesen wurden. Ändert nur die Tabelle; die CPU folgt erst mit `⚡ Live anwenden`.
  - **`CO-WERTE ANWENDEN`** direkt unter der Tabelle: die drei Wege nebeneinander, jeweils mit Kurzerklärung (siehe Kapitel 9).
- **Rechte Spalte (3-Tab Kontrollpanel)**:
  - **Tab `[🎯 Setup]`**: Profilauswahl mit Erklärung, Laufzeit-Abschätzung, Auto-Tuner-Karte mit Absenk-Fortschritt.
  - **Tab `[⚡ Engine]`**: Min/Kern, Durchgänge, Engine-Wahl (Prime95 / y-cruncher), Befehlssatz bis AVX-512, FFT-Bereich **inklusive frei wählbarem Bereich**, Transient-Pause (Intervall/Dauer), Kern-Reihenfolge **inklusive eigener Reihenfolge**, „Beim ersten Fehler anhalten", „Durchgefallenen Kern überspringen" und Pause zwischen Kernen.
  - **Tab `[🛡️ System]`**: Notfall-Temperaturlimit (°C), „WHEA-Warnung als Fehler werten", Aktion nach Testende (Standby/Herunterfahren), Discord-Webhook.
  - **Fortschrittsanzeige** liegt außerhalb der Tabs und bleibt daher in jedem Tab sichtbar.
  - **Sticky Footer**: `🚀 Test starten` / `🤖 Auto-Tuner starten` / `⏹ Test abbrechen`.
- **Untere Workspace-Karte** (Höhe per Trennlinie verstellbar):
  - Tab 1: **`📊 Live-Telemetrie`** — drei getrennte Spuren für Takt, Temperatur und PPT, je mit eigener beschrifteter Skala, Zeitachse über 15 Minuten, eingezeichnetem Notfall-Temperaturlimit und Hover-Tooltip mit echtem Zeitstempel.
  - Tab 2: **`📜 Protokoll`** — nach Schweregrad farbig, mit `Kopieren`, `Ordner` und `Leeren`.

> **Alle Einstellungen bleiben erhalten.** Sprache, Profil, Engine-Parameter, Temperaturlimit, Webhook und Fenstergröße werden in `runs/settings.json` gespeichert und beim nächsten Start wiederhergestellt.

---

## 4. Test-Profile & Hybrid-Strategie (Prime95 + y-cruncher)

PboStudio enthält vorgefertigte, praxiserprobte Test-Profile für jeden Einsatzzweck:

- **🏆 Hybrid Ultimate – Prime95 SSE & y-cruncher (Empfohlen)**:
  - **Phasen**: Phase 1 (Prime95 SSE Huge FFTs) ➔ Phase 2 (y-cruncher CO-Preset: VT3, SFT, SVT, FFT, N63).
  - **Zweck**: Der Gold-Standard! Kombiniert Prime95 SSE für maximale Taktfrequenzen & Vdroop-Limits mit y-cruncher für schwere AVX-Rechenlast und Speichercontroller-Stabilität.
- **⚡ Komplett – Prime95 SSE & AVX2**:
  - **Phasen**: Phase 1 (SSE mit Huge FFTs) + Phase 2 (AVX2 mit Smallest FFTs).
  - **Zweck**: Schneller Klassiker für Alltagstests und Kühlungsgrenzen.
- **🌙 Absicherung über Nacht (Overnight Validation)**:
  - **Phasen**: 3 Phasen über **ALLE FFT-Größen** (4K bis 51200K) für SSE, AVX und AVX2.
  - **Zweck**: 100%ige finale Bestätigung für 24/7-Stabilität vor dem dauerhaften BIOS-Eintrag.

---

## 5. Praxis-Leitfaden: Wie teste ich meine CPU am besten?

Je nachdem, ob deine CPU bereits teilweise optimiert ist oder du ganz neu beginnst, gibt es zwei ideale Vorgehensweisen:

### 🏆 Weg A: Der finale Stabilitäts-Beweis (Bestehende Werte absichern)
*Empfohlen, wenn du deine aktuellen Werte (z. B. 6x `-30` und 2x Zwischenwerte) 1:1 auf Herz und Nieren prüfen willst:*

1. **Auto-Tuner**: Im Tab *[🎯 Setup]* auf **`Deaktiviert (Manuell)`** lassen *(Grünes Status-Badge)*.
2. **Profil**: Wähle **`🏆 Hybrid Ultimate – Prime95 SSE & y-cruncher (Empfohlen)`**.
3. **Kerne**: Alle 8 Kerne ausgewählt lassen.
4. **Klick auf**: **`[🚀 TEST STARTEN]`** *(oder Taste `F5`)*.
5. **Ergebnis**:
   - Bestehen alle Kerne, ist deine Konfiguration **100% rock-solid stabil** für Gaming und 24/7-Betrieb.
   - Falls ein Kern zuckt, meldet sich der **WHEA-Wächter** und schlägt dir sofort die passende Entschärfung (`+4` / `+2`) vor.

---

### 🤖 Weg B: Der Auto-Tuner Turbo (Schnellste Suche & Feintuning)
*Empfohlen, wenn du herausfinden willst, ob die verbleibenden Kerne noch tiefere Werte schaffen:*

1. **Auto-Tuner**: Auf **`Grob (-3 Schritte)`** stellen *(Lila Status-Badge)*.
2. **Profil**: **`🏆 Hybrid Ultimate`** oder **`⚡ Komplett – SSE & AVX2`**.
3. **Klick auf**: **`[🤖 START AUTO-TUNER]`**.
4. **Adaptive Staging am Limit**:
   - Die Kerne, die bereits auf `-30` stehen, werden genau 1x (6 Min) validiert und danach direkt als `🎯 Limit (-30) bestätigt (🔒)` gelockt.
   - Die offenen Kerne werden Schritt für Schritt (`-3`) nach unten getestet.
   - Tritt Instabilität auf, federt der Auto-Tuner automatisch um `+3` zurück und sperrt den Kern (`🔒`).
5. **Dauer**: Bei einem Ryzen 7 5800X3D dauert dieser Gesamtlauf nur **ca. 48 bis 72 Minuten**!

---

## 6. System Health Audit (EXPO & BIOS Status)
Ein Klick auf den Plattform-Check (`🔍 System OK · 95/100`) prüft dein System auf zwei kritische Leistungsfaktoren:
1. **RAM EXPO / XMP Profil**: Liest über WMI die tatsächliche Taktrate ab. Läuft dein RAM nur mit JEDEC Standard-Takt (z.B. 4800 MT/s statt 6000 MT/s), wird eine Warnung ausgegeben.
2. **AGESA BIOS-Datum**: Prüft das Release-Datum deines Mainboard-BIOS. Ist das BIOS älter als 9 Monate, wird ein Update für bessere Ryzen-Stabilität empfohlen.

---

## 7. WHEA Wächter & Smart Empfehlungen (Grob vs. Fein)
Der integrierte Wächter liest in Echtzeit Windows Event Log **WHEA-Fehler 18/19** aus und ordnet sie streng numerisch aufsteigend den physikalischen CPU-Kernen zu (`Kern 0`, `Kern 1`, `Kern 4`).

### Precision Modi:
- **⏩ Grob (+4)**: Erhöht den Voltage-Offset bei WHEA-Fehlern um `+4` Schritte (z.B. `-30 ➔ -26`). Empfohlen, um instabile Kerne sofort abzusichern.
- **🎯 Fein (+2)**: Erhöht den Offset um nur `+2` Schritte (z.B. `-30 ➔ -28`). Für Tuner, die jedes Millivolt herauskitzeln wollen.

---

## 8. Der Auto-Tuner (zweiseitige Suche)

Der Auto-Tuner setzt jeden Wert **selbstständig per SMU, unmittelbar bevor der Kern gemessen wird** — du bestätigst nichts. Jede dieser Schreiboperationen steht im Protokoll.

Ablauf pro Kern:

| Situation | Was passiert |
|---|---|
| Bestanden, oberhalb des Limits | ein Schritt tiefer (`-3` bei Grob, `-1` bei Fein) |
| Bestanden am Chip-Limit | fixiert (🔒) — der Grenzwert ist belegt |
| Durchgefallen, vorher schon einmal bestanden | zurück auf den zuletzt bestandenen Wert und fixiert. Kein erneuter Test nötig: dieser Wert hat bereits einen vollen Durchgang überstanden |
| **Durchgefallen, noch nie bestanden** | Spannung anheben und **erneut testen**, Schritt für Schritt aufwärts, bis ein Wert tatsächlich hält. Erst dieser wird fixiert |
| Durchgefallen bei `0` | Abbruch für diesen Kern mit dem Hinweis, dass die Ursache nicht am Curve Optimizer liegt (RAM/EXPO, Kühlung, BIOS-Version) |

Damit trägt kein fixierter Wert das Etikett „fertig", ohne getestet worden zu sein.

**Zwei Dinge, die du wissen solltest:**

1. **Startest du direkt auf dem Chip-Limit** (z. B. `-30`), findet keine Suche nach unten statt — darunter gibt es nichts. Der Tuner prüft `-30` und arbeitet sich bei einem Fehler aufwärts. Für eine echte Suche startest du mild (z. B. `0` oder `-5`) und lässt ihn absenken.
2. **Die Suche ist durch „Durchgänge" begrenzt.** Von `-30` aus mit Grob und 3 Durchgängen kommst du nur bis `-24`. Reicht das nicht, bleibt der Kern offen statt fixiert. Die Laufzeit-Abschätzung im Setup-Tab rechnet den ungünstigsten Fall mit.

### 🎯 Hardware-Limits je nach CPU-Generation:
- **Ryzen 5000 (Zen 3) & Ryzen 7000 / 8000 (Zen 4):** Standard-Limit = **`-30`**
- **Ryzen 9000 (Zen 5 Granite Ridge):** Erweitertes Limit = **`-50`** (Curve Shaper)

---

## 9. Werte live anwenden vs. BIOS-Übernahme

Alle drei Wege stehen unter der Kern-Tabelle im Block **`CO-WERTE ANWENDEN`** nebeneinander.

### ⚡ Live anwenden (`Strg+S`)
Schreibt die geänderten Werte direkt in die SMU-Register. Wirkt sofort ohne Neustart.
- **Wichtig:** Beim nächsten Hochfahren ist das weg, weil das BIOS beim POST seine eigenen Werte neu setzt. Das Schließen der App ändert dagegen nichts — die Werte bleiben aktiv, solange Windows läuft.

### 🔄 Bei jedem Windows-Start
Das Häkchen im Anwenden-Block legt die geplante Windows-Aufgabe `PboStudioWatchdog` an, die deine gespeicherten Werte nach jedem Start im Hintergrund neu setzt. Bequemer als der BIOS-Eintrag, wirkt aber erst ein paar Sekunden nach dem Anmelden und braucht Administratorrechte. Ein abgeschlossener Auto-Tuner-Lauf aktualisiert dieses Profil automatisch.

### 📋 Permanentes Eintragen im Mainboard-BIOS (`Strg+C`)
Mit Klick auf **`📋 BIOS-Liste`** kopiert PboStudio die formatierten Werte in die Zwischenablage. Das ist der einzige wirklich dauerhafte Weg.
1. Starte den PC neu und drücke `ENTF` / `F2` fürs BIOS.
2. Gehe zu: *Advanced ➔ AMD Overclocking ➔ Precision Boost Overdrive ➔ Curve Optimizer*.
3. Setze *Curve Optimizer* auf **Per Core**, Vorzeichen auf **Negative** und trage deine Beträge ein.
4. Drücke `F10` zum Speichern!

---

## 10. Stabilitäts-Bericht (HTML Report Exporter)
Nach Abschluss eines Testlaufs bietet PboStudio die Funktion **`📄 Report exportieren`**.
Dabei wird eine interaktive, professionelle HTML-Zertifikatsdatei erstellt mit:
- Vollständiger Hardware-Zusammenfassung (CPU, RAM, System Health Score)
- Getesteten Durchgängen, Laufzeiten & Fehlerrate
- Kerntabelle mit final empfohlenen Curve Optimizer Werten zum Ausdrucken oder Teilen.

---

## 11. Absturzsicherheit & BSOD Recovery
Sollte während eines Testlaufs ein Systemabsturz (BSOD) auftreten:
- PboStudio speichert den Testzustand nach jedem Durchgang atomar in `runs/state.json`.
- Nach dem Windows-Neustart erkennt PboStudio den Unterbrechungszustand automatisch und bietet mit einem Klick an: **`✓ Empfohlenen Wert übernehmen & Fortsetzen`**.

---

## 12. Häufige Fragen (FAQ)

**F: Warum braucht PboStudio Admin-Rechte?**  
*A: Die SMU der AMD Ryzen Prozessoren ist ein geschützter Ring-0 Hardwarebereich. Ohne Admin-Rechte kann kein Tool die CO-Werte lesen oder schreiben.*

**F: Testet PboStudio physische oder logische Kerne?**  
*A: Curve Optimizer steuert die Spannungsversorgung pro physischem Kern. PboStudio testet und steuert daher exakt die physischen Kerne (z.B. Core 0 bis Core 7 beim 5800X3D).*

**F: Was unterscheidet PboStudio von CoreCycler?**  
*A: CoreCycler ist ein textbasiertes Skript ohne SMU-Schreibfunktion. PboStudio bietet native Live-SMU-Steuerung, vollautomatisiertes Auto-Tuning, eine moderne 3-Tab UI, WHEA-Wächter in Echtzeit, Hybrid-Testing (Prime95 + y-cruncher) und automatischen Windows-Autostart.*
