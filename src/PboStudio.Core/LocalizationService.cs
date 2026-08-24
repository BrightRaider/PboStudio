namespace PboStudio.Core;

public enum Language
{
    English,
    German
}

public static class LocalizationService
{
    public static Language CurrentLanguage { get; set; } = Language.English;

    public static bool IsGerman => CurrentLanguage == Language.German;

    /// <summary>
    /// Inline pair for strings that only appear once. Keys in <see cref="Get"/> are for text
    /// that XAML references or that more than one call site needs.
    /// </summary>
    public static string Pick(string german, string english) => IsGerman ? german : english;

    public static string Get(string key)
    {
        bool isDe = IsGerman;
        return key switch
        {
            // ---- Header ----
            "SystemAuditTooltip" => isDe ? "Klicken zum Öffnen/Schließen der System-Check Details" : "Click to toggle System Health Audit details",
            "DriverManager" => isDe ? "⚙️ Treiber & Engines" : "⚙️ Driver & Engines",
            "DriverManagerTooltip" => isDe ? "Öffnet die Verwaltung für PawnIO-Treiber, Prime95 und y-cruncher" : "Opens the manager for the PawnIO driver, Prime95 and y-cruncher",
            "LanguageTooltip" => isDe ? "Sprache umschalten (Deutsch / Englisch) · Strg+L" : "Switch UI language (German / English) · Ctrl+L",
            "WatchdogActive" => isDe ? "🛡️ WÄCHTER: Überwachung aktiv" : "🛡️ WATCHDOG: Monitoring active",
            "WatchdogReset" => isDe ? "🛡️ WÄCHTER: Überwachung aktiv (zurückgesetzt)" : "🛡️ WATCHDOG: Monitoring active (reset)",
            "WatchdogTooltip" => isDe
                ? "WHEA-Wächter: liest das Windows-Ereignisprotokoll auf korrigierte Hardware-Fehler (Machine Check Exceptions). Solche Fehler sind das früheste Anzeichen für ein zu aggressives Curve-Optimizer-Offset — oft lange bevor ein Test abstürzt."
                : "WHEA watchdog: reads the Windows event log for corrected hardware errors (machine check exceptions). These are the earliest sign of a too-aggressive Curve Optimizer offset — often long before a test crashes.",
            "SystemScoreTooltip" => isDe
                ? "Plattform-Check: prüft, ob EXPO/XMP aktiv und das BIOS aktuell ist. Beides beeinflusst die PBO-Stabilität, bevor überhaupt ein CO-Wert im Spiel ist."
                : "Platform check: verifies that EXPO/XMP is enabled and the BIOS is current. Both affect PBO stability before any CO value is involved.",
            "SystemOk" => isDe ? "System OK" : "System OK",
            "SystemHintOne" => isDe ? "Hinweis" : "hint",
            "SystemHintMany" => isDe ? "Hinweise" : "hints",
            "SystemChecking" => isDe ? "Prüfe System…" : "Checking system…",
            "SystemAuditTitle" => isDe ? "Plattform-Check" : "Platform check",
            "AuditCheckedPoints" => isDe ? "Geprüfte Punkte:" : "Checked items:",

            // ---- Live metrics ----
            "MetricBoost" => isDe ? "TAKT" : "CLOCK",
            "MetricTemp" => isDe ? "TEMP" : "TEMP",
            "MetricNoTelemetry" => isDe ? "Keine Telemetrie — PawnIO-Treiber nicht aktiv" : "No telemetry — PawnIO driver not active",
            "MetricLoading" => isDe ? "Telemetrie wird geladen…" : "Loading telemetry…",
            "TooltipPpt" => isDe
                ? "PPT (Package Power Tracking): die gesamte Leistungsaufnahme des CPU-Pakets in Watt. Erreicht der Ist-Wert das Limit, drosselt PBO den Takt."
                : "PPT (Package Power Tracking): total CPU package power draw in watts. When the current value reaches the limit, PBO throttles the clock.",
            "TooltipTdc" => isDe
                ? "TDC (Thermal Design Current): Dauerstrom aus den Spannungswandlern in Ampere, thermisch begrenzt. Wird meist bei längerer Volllast zum Limit."
                : "TDC (Thermal Design Current): sustained current from the VRMs in amps, thermally limited. Usually becomes the limit under prolonged full load.",
            "TooltipEdc" => isDe
                ? "EDC (Electrical Design Current): Spitzenstrom in Ampere. Wird bei kurzen Lastspitzen und wenigen aktiven Kernen zuerst erreicht."
                : "EDC (Electrical Design Current): peak current in amps. Reached first during short load spikes with few active cores.",
            "TooltipBoost" => isDe
                ? "Höchster aktuell anliegender Boost-Takt über alle Kerne. Ein negativer CO-Wert erlaubt bei gleicher Spannung mehr Takt — steigt dieser Wert, wirkt dein Offset."
                : "Highest boost clock currently applied across all cores. A negative CO value allows more clock at the same voltage — if this rises, your offset is working.",
            "TooltipTempNow" => isDe
                ? "Aktuelle CPU-Temperatur gegen das Drossel-Limit (Tjmax). Das Notfall-Limit für den Testabbruch stellst du im Tab „System“ ein."
                : "Current CPU temperature against the throttle limit (Tjmax). The emergency cut-off for aborting a test is set in the “System” tab.",
            "TooltipScalar" => isDe
                ? "PBO-Skalar: erlaubt der CPU, höhere Spannungen länger zu halten. Höhere Werte bedeuten mehr Boost, aber auch mehr Verschleiß."
                : "PBO scalar: allows the CPU to sustain higher voltages longer. Higher values mean more boost, but also more degradation.",

            // ---- Recommendation card ----
            "SmartRecommendationTitle" => isDe ? "SMART-EMPFEHLUNG (CPU & WHEA)" : "SMART RECOMMENDATION (CPU & WHEA)",
            "CoarseMode" => isDe ? "⏩ Grob (+4)" : "⏩ Coarse (+4)",
            "FineMode" => isDe ? "🎯 Fein (+2)" : "🎯 Fine (+2)",
            "CoarseFineTooltip" => isDe
                ? "Wie stark ein Kern nach einem WHEA-Fehler entschärft wird. Grob (+4) findet schnell einen stabilen Wert, Fein (+2) holt das letzte Quäntchen heraus, braucht aber mehr Durchläufe."
                : "How far a core is backed off after a WHEA error. Coarse (+4) finds a stable value quickly, Fine (+2) squeezes out the last bit but needs more passes.",
            "ApplyValues" => isDe ? "⚡ Werte übernehmen" : "⚡ Apply values",
            "ApplyValuesTooltip" => isDe ? "Trägt die empfohlenen Korrekturwerte in die Tabelle ein (schreibt sie noch nicht in die CPU)" : "Writes the recommended values into the table (does not yet write them to the CPU)",
            "CopyBios" => isDe ? "📋 BIOS-Liste" : "📋 BIOS list",
            "ResetWhea" => isDe ? "🧹 WHEA zurücksetzen" : "🧹 Reset WHEA",
            "ResetWheaTooltip" => isDe
                ? "Setzt den Fehlerzähler auf null. Alle bisherigen WHEA-Einträge werden ab jetzt ignoriert — sinnvoll, nachdem du neue CO-Werte übernommen hast."
                : "Resets the error counter to zero. All prior WHEA entries are ignored from now on — useful after you have applied new CO values.",
            "RecommendedProfile" => isDe ? "Empfohlenes Profil" : "Recommended profile",
            "UseProfile" => isDe ? "Profil laden" : "Load profile",

            // ---- Core table ----
            "AllCores" => isDe ? "ALLE KERNE" : "ALL CORES",
            "AllCoresTooltip" => isDe ? "Alle Kerne für den nächsten Testlauf aus- oder abwählen" : "Select or deselect all cores for the next run",
            "ColumnCore" => isDe ? "KERN" : "CORE",
            "CurveOpt" => isDe ? "CURVE OPTIMIZER" : "CURVE OPTIMIZER",
            "ColumnCoTooltip" => isDe
                ? "Curve-Optimizer-Offset in Zählschritten. Negativ = weniger Spannung (kühler, mehr Boost, riskanter). Im BIOS trägst du das getrennt ein: Vorzeichen „Negative“ und den Betrag ohne Minus, also -20 hier = Negative / 20 im BIOS."
                : "Curve Optimizer offset in counts. Negative = less voltage (cooler, more boost, riskier). In the BIOS you enter this separately: sign “Negative” plus the magnitude without the minus, so -20 here = Negative / 20 in the BIOS.",
            "TestStatus" => isDe ? "STATUS" : "STATUS",
            "StatusReady" => isDe ? "Bereit" : "Ready",
            "StatusWaiting" => isDe ? "wartet" : "waiting",
            "StatusRunning" => isDe ? "läuft" : "running",
            "StatusPassed" => isDe ? "bestanden" : "passed",
            "StatusFailed" => isDe ? "FEHLER" : "FAILED",
            "StatusSkipped" => isDe ? "übersprungen" : "skipped",
            "StatusLocked" => isDe ? "Limit" : "limit",
            "SetAllMax" => isDe ? "⚡ Alle auf" : "⚡ All to",
            "SetAllMaxTooltip" => isDe
                ? "Setzt alle ausgewählten Kerne auf den maximalen Negativ-Wert, den dieser Prozessor zulässt. Das ist der absolute Grenzwert — Abstürze sind dabei die Regel, nicht die Ausnahme."
                : "Sets all selected cores to the maximum negative value this processor allows. That is the absolute limit — crashes are the rule there, not the exception.",
            "SetAllReset" => isDe ? "🔄 BIOS-Werte" : "🔄 BIOS values",
            "SetAllResetTooltip" => isDe
                ? "Setzt die ausgewählten Kerne in der Tabelle auf die Werte zurück, die beim Programmstart aus der CPU gelesen wurden — also auf deine BIOS-Einstellung. Die CPU selbst ändert sich erst mit „⚡ Live anwenden“; ohne das bleiben die zuletzt geschriebenen Werte bis zum Neustart aktiv."
                : "Returns the selected cores in the table to the values read from the CPU at startup, i.e. your BIOS setting. The CPU itself only changes once you press “⚡ Apply live”; until then the values last written stay active until reboot.",
            "CoreTooltip" => isDe ? "Diesen Kern im nächsten Lauf testen" : "Test this core in the next run",
            "GoldCoreTooltip" => isDe
                ? "🥇 Bester Kern dieser CPU. Der Prozessor meldet für ihn den höchsten CPPC-Leistungswert ({0}) — Windows wählt ihn für einzelne Threads zuerst und er boostet am höchsten. Erfahrungsgemäß verträgt er 3–5 Punkte weniger Undervolt als Standardkerne."
                : "🥇 Best core on this CPU. The processor reports the highest CPPC performance value for it ({0}) — Windows picks it first for single threads and it boosts highest. Typically tolerates 3–5 points less undervolt than standard cores.",
            "SilverCoreTooltip" => isDe
                ? "🥈 Zweitbester Kern (CPPC-Leistungswert {0}). Boostet ebenfalls überdurchschnittlich hoch und reagiert empfindlicher auf aggressive CO-Werte."
                : "🥈 Second-best core (CPPC performance value {0}). Also boosts above average and reacts more sensitively to aggressive CO values.",
            "LockedTooltip" => isDe
                ? "🔒 Vom Auto-Tuner fixiert: Dieser Kern ist an seiner Stabilitätsgrenze angekommen und wird nicht weiter abgesenkt. Wert von Hand ändern hebt die Sperre auf."
                : "🔒 Locked by the auto-tuner: this core has reached its stability limit and will not be lowered further. Editing the value by hand releases the lock.",
            "CcdTooltip" => isDe ? "Core Complex Die — die physische Chiplet-Gruppe, auf der dieser Kern sitzt" : "Core Complex Die — the physical chiplet group this core sits on",
            "SelectCcdTooltip" => isDe ? "Nur die Kerne dieses Chiplets auswählen" : "Select only the cores on this chiplet",
            "DeltaTooltip" => isDe ? "Abweichung vom BIOS-Ausgangswert" : "Difference from the BIOS starting value",

            // ---- Test controls ----
            "TestControls" => isDe ? "TEST-STEUERUNG" : "TEST CONTROLS",
            "TestProfile" => isDe ? "Test-Profil" : "Test profile",
            "AutoTunerTitle" => isDe ? "AUTO-TUNER (vollautomatisch)" : "AUTO-TUNER (automated)",
            "AutoTunerTooltip" => isDe
                ? "Der Auto-Tuner senkt jeden Kern schrittweise ab, testet nach jedem Schritt und fixiert den Kern (🔒) beim ersten Fehler auf dem letzten stabilen Wert. Grob springt in -3er-Schritten und ist schnell, Fein geht in -1er-Schritten und findet das echte Maximum."
                : "The auto-tuner lowers each core step by step, tests after every step, and locks the core (🔒) at the last stable value on the first error. Coarse moves in -3 steps and is fast, Fine moves in -1 steps and finds the true maximum.",
            "AutoTunerDisabled" => isDe ? "Deaktiviert (manuell)" : "Disabled (manual)",
            "AutoTunerCoarse" => isDe ? "Grob (-3 Schritte — Schnellsuche)" : "Coarse (-3 steps — fast search)",
            "AutoTunerFine" => isDe ? "Fein (-1 Schritt — Feinabstimmung)" : "Fine (-1 step — precision tuning)",
            "AutoTunerProgress" => isDe ? "Absenk-Fortschritt" : "Step-down progress",
            "AutoTunerNeedsSmu" => isDe
                ? "Der Auto-Tuner kann ohne PawnIO keine Curve-Optimizer-Werte setzen und würde nur die BIOS-Werte messen. Bitte zuerst den PawnIO-Treiber installieren."
                : "Without PawnIO the auto-tuner cannot write Curve Optimizer values and would only measure the BIOS settings. Install the PawnIO driver first.",
            "MarginsNotApplied" => isDe
                ? "Hinweis: {0} Kern(e) haben in der Tabelle andere Werte als die CPU. Ohne PawnIO können sie nicht gesetzt werden — getestet werden die aktuell anliegenden BIOS-Werte."
                : "Note: {0} core(s) hold different values in the table than the CPU does. Without PawnIO they cannot be applied — the run tests the BIOS values currently in effect.",

            "StartTest" => isDe ? "🚀 Test starten" : "🚀 Start test",
            "StartAutoTuner" => isDe ? "🤖 Auto-Tuner starten" : "🤖 Start auto-tuner",
            "StopTest" => isDe ? "⏹ Test abbrechen" : "⏹ Cancel test",
            "StartTooltip" => isDe ? "Startet oder stoppt den Testlauf · F5" : "Starts or stops the test run · F5",
            "SetupRequired" => isDe ? "⚠️ Setup erforderlich" : "⚠️ Setup required",
            "SetupRequiredTooltip" => isDe ? "Es fehlen noch Komponenten. Öffne den Treiber- & Engines-Manager." : "Components are still missing. Open the driver & engines manager.",
            "TestRunning" => isDe ? "TEST LÄUFT" : "TEST RUNNING",

            // ---- Apply section ----
            "ApplySectionTitle" => isDe ? "CO-WERTE ANWENDEN" : "APPLY CO VALUES",
            "ApplyLive" => isDe ? "⚡ Live anwenden" : "⚡ Apply live",
            "ApplyLiveHint" => isDe ? "wirkt sofort · weg nach Neustart" : "takes effect at once · gone after reboot",
            "ApplyLiveTooltip" => isDe
                ? "Schreibt die geänderten Werte direkt in die SMU-Register der CPU. Wirkt sofort und ohne Neustart — geht aber beim nächsten Hochfahren verloren, weil das BIOS seine eigenen Werte neu setzt. · Strg+S"
                : "Writes the changed values straight into the CPU's SMU registers. Takes effect immediately without a reboot — but is lost on the next boot, because the BIOS reapplies its own values. · Ctrl+S",
            "ApplyBiosHint" => isDe ? "dauerhaft · Handarbeit im UEFI" : "permanent · manual entry in UEFI",
            "ApplyBiosTooltip" => isDe
                ? "Kopiert alle Werte formatiert in die Zwischenablage, damit du sie im UEFI unter AMD Overclocking → Curve Optimizer eintragen kannst. Das ist der einzige wirklich dauerhafte Weg. · Strg+C"
                : "Copies all values to the clipboard in a readable format so you can enter them in the UEFI under AMD Overclocking → Curve Optimizer. This is the only truly permanent route. · Ctrl+C",
            "AutostartBoot" => isDe ? "Bei jedem Windows-Start" : "On every Windows start",
            "ApplyAutostartHint" => isDe ? "dauerhaft · geplante Aufgabe · Adminrechte" : "permanent · scheduled task · needs admin",
            "AutostartTooltip" => isDe
                ? "Legt die geplante Windows-Aufgabe „PboStudioWatchdog“ an, die deine gespeicherten CO-Werte nach jedem Start im Hintergrund neu setzt. Bequemer als der BIOS-Eintrag, wirkt aber erst ein paar Sekunden nach dem Anmelden — und braucht Administratorrechte."
                : "Creates the scheduled Windows task “PboStudioWatchdog”, which reapplies your saved CO values in the background after every boot. More convenient than the BIOS route, but only takes effect a few seconds after login — and requires administrator rights.",

            // ---- Engine tab ----
            "MinPerCore" => isDe ? "Min. / Kern" : "Min / core",
            "MinPerCoreTooltip" => isDe ? "Wie lange jeder einzelne Kern pro Durchgang belastet wird" : "How long each individual core is stressed per pass",
            "Passes" => isDe ? "Durchgänge" : "Passes",
            "PassesTooltip" => isDe ? "Wie oft die komplette Kernliste durchlaufen wird. Ein Kern besteht oft den ersten Durchgang und fällt im dritten durch." : "How many times the full core list is worked through. A core often passes the first pass and fails on the third.",
            "StressEngine" => isDe ? "Testprogramm" : "Stress engine",
            "StressEngineTooltip" => isDe ? "Prime95 belastet mit klassischen FFT-Berechnungen, y-cruncher mit Pi-Berechnungen und deutlich mehr Cache- und Speicherdruck." : "Prime95 stresses with classic FFT math, y-cruncher with pi computations and considerably more cache and memory pressure.",
            "ThreadsPerCore" => isDe ? "Threads pro Kern" : "Threads per core",
            "ThreadsTooltip" => isDe
                ? "SMT (Simultaneous Multithreading) ist AMDs Hyperthreading: jeder physische Kern führt zwei Threads aus. Ein Thread lässt den Kern höher boosten und findet Fehler bei Höchsttakt, zwei Threads erzeugen mehr Hitze und Stromlast."
                : "SMT (simultaneous multithreading) is AMD's hyper-threading: each physical core runs two threads. One thread lets the core boost higher and finds errors at peak clock, two threads generate more heat and current load.",
            "InstructionSet" => isDe ? "Befehlssatz" : "Instruction set",
            "InstructionSetTooltip" => isDe
                ? "Welche Recheneinheiten belastet werden. SSE lässt den höchsten Takt zu und findet Instabilität bei niedriger Spannung — dafür ist es beim Curve Optimizer das Mittel der Wahl. AVX/AVX2/AVX-512 ziehen deutlich mehr Strom und decken andere Fehler auf."
                : "Which execution units get stressed. SSE allows the highest clock and finds instability at low voltage — which is why it is the tool of choice for Curve Optimizer work. AVX/AVX2/AVX-512 draw far more current and expose different faults.",
            "FftRange" => isDe ? "FFT-Bereich" : "FFT range",
            "FftCustom" => isDe ? "Eigener Bereich…" : "Custom range…",
            "FftMin" => isDe ? "von (K)" : "from (K)",
            "FftMax" => isDe ? "bis (K)" : "to (K)",
            "FftCustomTooltip" => isDe
                ? "Eigener FFT-Bereich in K. Prime95 arbeitet die Größen zwischen diesen Grenzen ab. Ein enger Bereich um eine Größe, die einen Kern zuvor zum Absturz gebracht hat, reproduziert den Fehler deutlich schneller als ein voller Durchlauf."
                : "Custom FFT range in K. Prime95 works through the sizes between these bounds. A narrow range around a size that previously crashed a core reproduces the fault far faster than a full sweep.",
            "FftTooltip" => isDe
                ? "FFT (Fast Fourier Transform) ist die Rechenaufgabe, mit der Prime95 belastet. Kleine FFTs passen in den Cache und erzeugen maximale Hitze im Kern; große FFTs gehen in den Arbeitsspeicher und prüfen zusätzlich Speichercontroller und Infinity Fabric."
                : "FFT (fast Fourier transform) is the workload Prime95 uses. Small FFTs fit in cache and generate maximum in-core heat; large FFTs reach into main memory and additionally test the memory controller and Infinity Fabric.",
            "CoreOrder" => isDe ? "Kern-Reihenfolge" : "Core order",
            "CoreOrderTooltip" => isDe ? "„Abwechselnd“ springt zwischen den Chiplets hin und her, damit ein Kern zwischen zwei Läufen abkühlen kann." : "“Alternate” hops between chiplets so a core can cool down between two runs.",
            "OrderCustom" => isDe ? "Eigene Reihenfolge…" : "Custom order…",
            "CustomOrderLabel" => isDe ? "Reihenfolge (Kern-Nummern)" : "Order (core numbers)",
            "CustomOrderTooltip" => isDe
                ? "Kern-Nummern in der gewünschten Reihenfolge, getrennt durch Komma oder Leerzeichen — zum Beispiel „0, 4, 1, 5“. Nicht genannte Kerne werden übersprungen. Praktisch, um verdächtige Kerne zuerst und mehrfach zu prüfen."
                : "Core numbers in the order you want them, separated by commas or spaces — for example “0, 4, 1, 5”. Cores you leave out are skipped. Handy for testing suspect cores first, and more than once.",
            "CustomOrderEmpty" => isDe ? "Noch keine gültige Reihenfolge eingegeben." : "No valid order entered yet.",
            "CustomOrderInvalid" => isDe ? "Ungültig: {0}" : "Not valid: {0}",
            "CustomOrderPreview" => isDe ? "Läuft als: {0}" : "Runs as: {0}",
            "CustomOrderNeeded" => isDe
                ? "Es ist „Eigene Reihenfolge“ gewählt, aber keine gültige Kern-Reihenfolge eingetragen."
                : "“Custom order” is selected but no valid core order has been entered.",
            "StopOnError" => isDe ? "Beim ersten Fehler anhalten" : "Stop on first error",
            "StopOnErrorTooltip" => isDe ? "Bricht den gesamten Lauf ab, sobald ein Kern durchfällt — statt die restlichen Kerne weiterzutesten." : "Aborts the whole run as soon as one core fails — instead of continuing with the remaining cores.",
            "PauseInterval" => isDe ? "Pause alle (Sek.)" : "Pause every (s)",
            "PauseDuration" => isDe ? "Pause-Dauer (Sek.)" : "Pause for (s)",
            "TransientPauseTooltip" => isDe
                ? "Transient-Pause: der Test wird kurz angehalten, damit der Kern in den Leerlauf fällt und danach wieder hochboostet. Genau dieser Lastwechsel (Vdroop) bringt grenzwertige CO-Werte zum Absturz — ein Dauerlauf mit konstanter Last tut das oft nicht."
                : "Transient pause: the test is briefly suspended so the core drops to idle and then boosts back up. Exactly this load transition (Vdroop) is what makes borderline CO values crash — a steady constant load often does not.",
            "YcAlgo" => isDe ? "y-cruncher Algorithmen" : "y-cruncher algorithms",
            "YcAlgoTooltip" => isDe ? "Welche Rechenkerne von y-cruncher gefahren werden. Das CO-Preset enthält genau die Algorithmen, die erfahrungsgemäß am zuverlässigsten instabile Kerne aufdecken." : "Which y-cruncher workloads run. The CO preset contains exactly the algorithms that experience shows expose unstable cores most reliably.",

            // ---- System tab ----
            "SafetyTempLimit" => isDe ? "Notfall-Temperaturlimit (°C)" : "Emergency temperature limit (°C)",
            "SafetyTempTooltip" => isDe ? "Übersteigt die CPU-Temperatur diesen Wert, wird der Testlauf sofort abgebrochen. Bleibt auch während eines Laufs änderbar." : "If the CPU temperature exceeds this value the run is aborted immediately. Stays editable during a run.",
            "PostTestAction" => isDe ? "Aktion nach Testende" : "Post-test action",
            "ActionDoNothing" => isDe ? "Nichts tun (PC eingeschaltet lassen)" : "Do nothing (keep PC running)",
            "ActionSleep" => isDe ? "🌙 Energiesparmodus (Standby)" : "🌙 Sleep (standby)",
            "ActionShutdown" => isDe ? "🔌 PC herunterfahren" : "🔌 Shut down PC",
            "WebhookLabel" => isDe ? "Discord-Webhook (optional)" : "Discord webhook (optional)",
            "WebhookTooltip" => isDe ? "Schickt Start, Kernfehler und Endergebnis als Nachricht in einen Discord-Kanal — praktisch für Läufe über Nacht." : "Sends start, core failures and the final result as messages into a Discord channel — handy for overnight runs.",

            // ---- Workspace tabs ----
            "TabGraph" => isDe ? "📊 Live-Telemetrie" : "📊 Live telemetry",
            "TabLog" => isDe ? "📜 Protokoll" : "📜 Log",
            "TabSetup" => isDe ? "🎯 Setup" : "🎯 Setup",
            "TabEngine" => isDe ? "⚡ Engine" : "⚡ Engine",
            "TabSystem" => isDe ? "🛡️ System" : "🛡️ System",
            "CopyLog" => isDe ? "Kopieren" : "Copy",
            "ClearLog" => isDe ? "Leeren" : "Clear",
            "OpenLogFolder" => isDe ? "Ordner" : "Folder",
            "OpenLogFolderTooltip" => isDe
                ? "Öffnet den Ordner mit den Protokolldateien. Jeder Programmstart schreibt eine eigene Datei, die laufend mitgeschrieben wird — sie überlebt also auch einen Absturz oder Bluescreen."
                : "Opens the folder holding the log files. Every program start writes its own file continuously, so it survives a crash or bluescreen.",
            "Shortcuts" => isDe
                ? "⌨️ F5 Start/Stop · Strg+S Live · Strg+C BIOS · Strg+L Sprache"
                : "⌨️ F5 start/stop · Ctrl+S live · Ctrl+C BIOS · Ctrl+L language",

            // ---- Results ----
            "ResultPassTitle" => isDe ? "✓ Bestanden — kein Kern ist durchgefallen." : "✓ Passed — no core failed.",
            "ResultPassText" => isDe
                ? "Für ein wirklich belastbares Ergebnis lass als Nächstes das Profil „Absicherung – Alles über Nacht“ laufen und danach einmal „Kleine FFTs“. Zwei verschiedene Lastarten zu bestehen sagt deutlich mehr aus als eine."
                : "For a genuinely dependable result, run the “Overnight Thorough Validation” profile next, then “Small FFTs” once. Passing two different load types says considerably more than passing one.",
            "ResultFailOne" => isDe ? "Ein Kern ist durchgefallen." : "One core failed.",
            "ResultFailMany" => isDe ? "{0} Kerne sind durchgefallen." : "{0} cores failed.",
            "ResultFailAdvice" => isDe
                ? "Die Vorschläge gehen bewusst {0} Schritte zurück, nicht einen: Ein Wert, der knapp durchfällt, ist auch knapp darüber noch nicht sicher. Nach dem Übernehmen die betroffenen Kerne erneut testen — sie sind dafür bereits allein angehakt, das dauert Minuten statt Stunden."
                : "The suggestions deliberately back off {0} steps, not one: a value that just barely fails is not safe just barely above it either. After applying, re-test the affected cores — they are already selected on their own, which takes minutes instead of hours.",
            "ReasonCalc" => isDe ? "hat falsch gerechnet" : "produced a wrong result",
            "ReasonMachineCheck" => isDe ? "hat einen Hardware-Fehler gemeldet" : "reported a hardware error",
            "ReasonProcessGone" => isDe ? "hat den Test abstürzen lassen" : "crashed the test process",
            "ReasonHang" => isDe ? "ist unter Last eingeschlafen" : "stopped responding under load",
            "ResultAccept" => isDe ? "✓ Empfohlene Werte übernehmen" : "✓ Apply recommended values",
            "ResultExport" => isDe ? "📄 Bericht exportieren" : "📄 Export report",
            "ResultExportTooltip" => isDe ? "Erzeugt einen HTML-Stabilitätsbericht mit allen Werten, Fehlern und Laufzeiten" : "Creates an HTML stability report with all values, errors and runtimes",
            "Close" => isDe ? "Schließen" : "Close",
            "BiosCopiedTitle" => isDe ? "📋 BIOS-Werte kopiert" : "📋 BIOS values copied",
            "BiosCopiedHint" => isDe
                ? "Diese Werte trägst du im UEFI unter AMD Overclocking → Curve Optimizer → Per Core ein. Dort gibst du Vorzeichen und Betrag getrennt an: „Negative“ plus die Zahl ohne Minuszeichen."
                : "Enter these values in the UEFI under AMD Overclocking → Curve Optimizer → Per Core. There you specify sign and magnitude separately: “Negative” plus the number without the minus sign.",

            // ---- Recovery banner ----
            "RecoveryTitle" => isDe ? "Unerwarteter System-Neustart erkannt" : "Unexpected system restart detected",
            "RecoveryResume" => isDe ? "✓ Wert korrigieren & fortsetzen" : "✓ Correct value & resume",
            "RecoveryDismiss" => isDe ? "Verwerfen" : "Dismiss",

            // ---- Confirmations ----
            "ConfirmTitle" => isDe ? "Bist du sicher?" : "Are you sure?",
            "ConfirmAllMax" => isDe
                ? "Setzt alle {0} ausgewählten Kerne auf {1} — den absoluten Grenzwert dieses Prozessors.\n\nDieser Wert ist auf so gut wie keiner CPU stabil. Rechne mit Abstürzen, Bluescreens und einem abgebrochenen Testlauf. Als Ausgangspunkt für den Auto-Tuner ist das in Ordnung, als Dauereinstellung nicht."
                : "Sets all {0} selected cores to {1} — the absolute limit of this processor.\n\nThis value is stable on almost no CPU. Expect crashes, bluescreens and an aborted run. As a starting point for the auto-tuner that is fine; as a permanent setting it is not.",
            "ConfirmYes" => isDe ? "Trotzdem setzen" : "Set anyway",
            "ConfirmNo" => isDe ? "Abbrechen" : "Cancel",

            // ---- Setup wizard ----
            "WizardTitle" => isDe ? "Treiber & Stress-Engines" : "Driver & stress engines",
            "WizardSubtitle" => isDe
                ? "PboStudio braucht drei Bausteine. Fehlt einer, kann kein Test starten."
                : "PboStudio needs three building blocks. If one is missing, no test can start.",
            "WizardPawnIo" => isDe ? "PawnIO Kernel-Treiber" : "PawnIO kernel driver",
            "WizardPawnIoWhat" => isDe
                ? "Der einzige Weg, Curve-Optimizer-Werte im laufenden Betrieb zu lesen und zu schreiben. PawnIO ist ein quelloffener, Microsoft-signierter Treiber, der Ring-0-Zugriffe auf klar umrissene Befehle beschränkt — anders als die üblichen Tuning-Treiber, die vollen Speicherzugriff freigeben. Ohne ihn laufen Stresstests weiterhin, nur das Setzen der Werte nicht."
                : "The only way to read and write Curve Optimizer values while Windows is running. PawnIO is an open-source, Microsoft-signed driver that restricts ring-0 access to a narrow set of commands — unlike the usual tuning drivers, which expose full memory access. Without it, stress tests still run; only writing values does not.",
            "WizardPawnIoNeedsAdmin" => isDe ? "Die Installation öffnet ein Administrator-Fenster von Windows. Danach ist ein Neustart von PboStudio nötig." : "Installation opens a Windows administrator prompt. PboStudio needs to be restarted afterwards.",
            "WizardPrime95" => isDe ? "Prime95" : "Prime95",
            "WizardPrime95What" => isDe
                ? "Der Standard-Stresstest für Curve Optimizer. Rechnet FFTs und meldet sofort, wenn ein Kern falsch rechnet — das feinste verfügbare Instabilitäts-Signal."
                : "The standard stress test for Curve Optimizer work. Computes FFTs and reports immediately when a core calculates incorrectly — the finest available instability signal.",
            "WizardYCruncher" => isDe ? "y-cruncher" : "y-cruncher",
            "WizardYCruncherWhat" => isDe
                ? "Zweite Meinung mit anderem Lastprofil: viel Cache- und Speicherdruck. Findet Fehler, die Prime95 durchgehen lässt — deshalb bestehen beide zusammen deutlich mehr als eines allein."
                : "A second opinion with a different load profile: heavy cache and memory pressure. Catches faults Prime95 lets through — which is why passing both means considerably more than passing one.",
            "WizardInstall" => isDe ? "Installieren" : "Install",
            "WizardDownload" => isDe ? "Herunterladen" : "Download",
            "WizardWebsite" => isDe ? "Website" : "Website",
            "WizardInstallAll" => isDe ? "⬇ Alles Fehlende installieren" : "⬇ Install everything missing",
            "WizardRecheck" => isDe ? "🔄 Erneut prüfen" : "🔄 Check again",
            "WizardReady" => isDe ? "✅ Alles bereit — du kannst testen." : "✅ All set — you are ready to test.",
            "WizardMissing" => isDe ? "⚠️ Es fehlen noch {0} Komponente(n)." : "⚠️ {0} component(s) still missing.",
            "WizardInstalled" => isDe ? "✅ Installiert" : "✅ Installed",
            "WizardNotFound" => isDe ? "❌ Nicht gefunden — kann automatisch heruntergeladen werden." : "❌ Not found — can be downloaded automatically.",
            "WizardPawnIoOk" => isDe ? "✅ Treiber aktiv, SMU erreichbar." : "✅ Driver active, SMU reachable.",
            "WizardCancel" => isDe ? "Abbrechen" : "Cancel",

            // ---- Graph ----
            "GraphClock" => isDe ? "Takt" : "Clock",
            "GraphTemp" => isDe ? "Temperatur" : "Temperature",
            "GraphPower" => isDe ? "Leistung" : "Power",
            "GraphNoData" => isDe ? "Noch keine Messwerte" : "No samples yet",
            "GraphLimit" => isDe ? "Limit" : "limit",
            "GraphNow" => isDe ? "jetzt" : "now",

            _ => key
        };
    }
}
