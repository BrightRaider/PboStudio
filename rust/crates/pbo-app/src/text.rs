//! Every sentence the window shows, in both languages (ADR-0011).
//!
//! A key is a type: a missing key is a compile error, and every key has an English and a German text or there is no
//! entry. Texts with numbers are formats that take the numbers as arguments, never sentences that guess a size.
//! The tone is the signature's: short, quiet, no exclamation marks.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    De,
}

/// Declares every key once, with its two texts, and builds the type, the lookup and the list of all keys from it.
macro_rules! texts {
    ($($key:ident => ($en:expr, $de:expr),)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Key {
            $($key,)*
        }

        impl Key {
            /// Every key, for the test that checks them all.
            pub const ALL: &'static [Key] = &[$(Key::$key,)*];
        }

        pub fn tr(lang: Lang, key: Key) -> &'static str {
            let (en, de) = match key {
                $(Key::$key => ($en, $de),)*
            };
            match lang {
                Lang::En => en,
                Lang::De => de,
            }
        }
    };
}

texts! {
    TabPlan => ("Plan", "Plan"),
    TabMeasurements => ("Measurements", "Messwerte"),
    TabLog => ("Log", "Protokoll"),
    TabInfo => ("Processor", "Prozessor"),
    Start => ("Start", "Start"),
    Stop => ("Stop", "Anhalten"),
    Adjust => ("Adjust", "Anpassen"),
    BackToRecommendation => ("Back to the recommendation", "Zurück auf Empfehlung"),
    Dismiss => ("Done", "Fertig"),
    NotElevatedTitle => ("Administrator rights needed", "Administratorrechte nötig"),
    NotElevatedBody => ("The processor registers can only be reached from an elevated program. Nothing has been read or changed.", "Die Prozessorregister sind nur aus einem Programm mit erhöhten Rechten erreichbar. Es wurde nichts gelesen oder geändert."),
    RestartElevated => ("Restart as administrator", "Als Administrator neu starten"),
    NoEngineTitle => ("No test program found", "Kein Testprogramm gefunden"),
    NoEngineBody => ("Put Prime95 into an engines folder next to this program (engines\\prime95\\prime95.exe).", "Prime95 in einen Ordner engines neben dieses Programm legen (engines\\prime95\\prime95.exe)."),
    CheckedOnHardware => ("checked on hardware", "an Hardware geprüft"),
    NotCheckedOnHardware => ("not checked on hardware", "nicht an Hardware geprüft"),
    NotCheckedHint => ("Nobody has run this program on this processor family yet. Before anything is written, the write path tests itself: each core's own value is written back and read.", "Auf dieser Prozessorfamilie hat das noch niemand ausprobiert. Bevor etwas geschrieben wird, prüft sich der Schreibweg selbst: Der Wert jedes Kerns wird zurückgeschrieben und gelesen."),
    RestartHint => ("The machine may restart during this. That is intended; it carries on afterwards.", "Der Rechner startet dabei eventuell neu. Das ist gewollt, danach macht es weiter."),
    CoresLabel => ("Cores", "Kerne"),
    EngineLabel => ("Test program", "Testprogramm"),
    SecondsLabel => ("Seconds per test", "Sekunden pro Test"),
    SearchLabel => ("Search", "Suche"),
    SearchOff => ("Off", "Aus"),
    SearchCoarse => ("Coarse", "Grob"),
    SearchFine => ("Fine", "Fein"),
    HeadroomLabel => ("Headroom", "Abstand"),
    BothThreads => ("Both threads of a core", "Beide Threads eines Kerns"),
    CoreList => ("Cores and their values", "Kerne und ihre Werte"),
    CoreHeld => ("holds", "hält"),
    CoreBestHeld => ("best that held", "bester, der hielt"),
    CoreFailedAt => ("failed at", "fiel durch bei"),
    CoreNothingKnown => ("nothing tested yet", "noch nichts getestet"),
    Temperature => ("Temperature", "Temperatur"),
    Power => ("Power", "Leistung"),
    CoreVoltage => ("Core voltage", "Kernspannung"),
    NotAvailable => ("n/a", "n/v"),
    StateReady => ("Ready", "Bereit"),
    StatePreparing => ("Preparing", "Vorbereitung"),
    StateStopping => ("Stopping, putting the values back", "Wird beendet, die Werte werden zurückgesetzt"),
    StateFinished => ("Finished", "Fertig"),
    StateStopped => ("Stopped", "Angehalten"),
    StateLimit => ("Stopped at a limit", "An einer Grenze angehalten"),
    StateInconclusive => ("Stopped: a test did not run", "Angehalten: ein Test lief nicht"),
    SaveAsLogonProfile => ("Keep these values for logon", "Diese Werte für die Anmeldung behalten"),
    Saved => ("Saved", "Gespeichert"),
    TheseCommandLine => ("The same on the command line", "Dasselbe in der Befehlszeile"),
    LogEmpty => ("Nothing has been logged in this session.", "In dieser Sitzung wurde noch nichts protokolliert."),
    InfoProcessor => ("Processor", "Prozessor"),
    InfoBios => ("BIOS", "BIOS"),
    InfoWritePath => ("Write path", "Schreibweg"),
    InfoRunSelftest => ("Test the write path", "Schreibweg prüfen"),
    InfoSelftestOk => ("Works: every core's value was written back and read.", "Funktioniert: Der Wert jedes Kerns wurde zurückgeschrieben und gelesen."),
    StartHelper => ("Start the helper", "Helfer starten"),
    HelperHint => ("The window stays without administrator rights. A small helper with those rights does the work and keeps a run going if this window is closed.", "Das Fenster bleibt ohne Administratorrechte. Ein kleiner Helfer mit diesen Rechten erledigt die Arbeit und lässt einen Lauf weiterlaufen, wenn dieses Fenster geschlossen wird."),
    HelperWaiting => ("Waiting for the helper", "Warte auf den Helfer"),
    ReadyTitle => ("Ready for the night?", "Bereit für die Nacht?"),
    ReadyLogonTask => ("Logon task", "Anmelde-Aufgabe"),
    ReadyLogonTaskHint => ("Without it nothing carries on after a crash. Turn it on with: pbo autostart on", "Ohne sie macht nach einem Absturz nichts weiter. Einrichten mit: pbo autostart on"),
    ReadyAutoLogin => ("Automatic sign-in", "Automatische Anmeldung"),
    ReadyAutoLoginHint => ("Without automatic sign-in the machine stops at the logon screen after a crash and the task never runs. The program does not set this up: it is a security setting that is yours to decide.", "Ohne automatische Anmeldung bleibt der Rechner nach einem Absturz am Anmeldebildschirm stehen, und die Aufgabe läuft nie. Das Programm richtet das nicht ein: Es ist eine Sicherheitseinstellung, die Sie entscheiden."),
    ReadySleep => ("Sleep on mains power", "Ruhezustand am Netzteil"),
    ReadySleepHint => ("The program holds the machine awake while a run is going, but a sleep that begins before the run starts or after it ends is not covered.", "Das Programm hält den Rechner während eines Laufs wach, aber ein Ruhezustand vor dem Start oder nach dem Ende ist nicht abgedeckt."),
    ReadyDisk => ("Free disk space", "Freier Platz"),
    ReadyDiskHint => ("Less than 500 MB are free on the drive of the log folder.", "Auf dem Laufwerk des Protokollordners sind weniger als 500 MB frei."),
    ReadyTemperature => ("Temperature headroom", "Abstand zur Temperaturgrenze"),
    ReadyTemperatureHint => ("At idle the processor is already close to its limit. A long run will be throttled or stopped.", "Im Leerlauf ist der Prozessor schon nah an der Grenze. Ein langer Lauf wird gedrosselt oder angehalten."),
    ReadyUnknown => ("could not be read", "konnte nicht gelesen werden"),
    ReadyCheck => ("Check", "Prüfen"),
    LearnedTitle => ("What the search has learned", "Was die Suche gelernt hat"),
    SetupTitle => ("Set up", "Einrichten"),
    SetupInstall => ("Install", "Installieren"),
    SetupPresent => ("present", "vorhanden"),
    SetupMissing => ("missing", "fehlt"),
    SetupDriver => ("Driver (PawnIO)", "Treiber (PawnIO)"),
    SetupDriverUnknown => ("cannot be checked without administrator rights", "ohne Administratorrechte nicht prüfbar"),
    AutoStart => ("Start at", "Start bei"),
    AutoStartNoEffect => ("No effect now: every core already holds a value.", "Jetzt ohne Wirkung: alle Kerne haben schon einen Wert."),
    AutoMore => ("Fine-tuning", "Feineinstellung"),
    AutoLess => ("Back", "Zurück"),
    AutoStartTip => ("How deep the search of a core that holds nothing yet begins. Most cores clear it; one that does not is taken back up.", "Wie tief die Suche bei einem Kern beginnt, der noch nichts hält. Die meisten Kerne halten ihn; einer, der ihn nicht hält, wird wieder hinaufgesucht."),
    AutoStep => ("Step", "Schritt"),
    AutoStepTip => ("How far the value moves with each test, in counts. Smaller steps find the limit more exactly and take more tests.", "Wie weit der Wert mit jedem Test sinkt, in Zählern. Kleinere Schritte treffen die Grenze genauer und brauchen mehr Tests."),
    AutoMargin => ("Safety", "Sicherheit"),
    AutoMarginTip => ("Counts kept between the deepest value that held and the one that is set at the end.", "Zähler Abstand zwischen dem tiefsten Wert, der hielt, und dem, der am Ende gesetzt wird."),
    NoCoreChosen => ("Choose at least one core.", "Mindestens einen Kern wählen."),
    SetupDone => ("Done: set up", "Fertig: eingerichtet"),
    CoreSet => ("Set", "Setzen"),
    SetupLogonTask => ("Logon task", "Anmelde-Aufgabe"),
    SetupRemove => ("Remove", "Entfernen"),
    InterruptedTitle => ("The last run did not end", "Der letzte Lauf ist nicht zu Ende gegangen"),
    InterruptedBody => ("The program was closed or the machine crashed in the middle of a test. The values the cores hold may be test values. This puts every core back to what it was proven to hold, and notes a crash against the core that was under test.", "Das Programm wurde geschlossen oder der Rechner ist mitten in einem Test abgestürzt. Die Werte der Kerne können Testwerte sein. Das schreibt jeden Kern auf seinen bewährten Wert zurück und vermerkt einen Absturz beim getesteten Kern."),
    InterruptedRestore => ("Put the proven values back", "Bewährte Werte zurückschreiben"),
    PlanExplainSearch => ("Start looks for the deepest Curve Optimizer value each core keeps stable: it tests one core at a time and steps the value down until the core fails, then settles on the last value that held, with some headroom. Values are applied live and are gone after a restart; nothing is written to the BIOS.", "Start sucht für jeden Kern den tiefsten Curve-Optimizer-Wert, der stabil bleibt: Ein Kern nach dem anderen wird getestet und der Wert Schritt für Schritt gesenkt, bis der Kern durchfällt. Dann gilt der letzte Wert, der hielt, mit etwas Abstand. Die Werte wirken sofort und sind nach einem Neustart weg; ins BIOS wird nichts geschrieben."),
    PlanExplainCheck => ("Start tests each core at the value it holds now and changes nothing.", "Start testet jeden Kern bei dem Wert, den er jetzt hat, und ändert nichts."),
    ValueUnderTest => ("Value under test", "Getesteter Wert"),
    Restored => ("Values put back", "Werte zurückgeschrieben"),
    SetupFailed => ("Set up failed; see the log tab", "Einrichten fehlgeschlagen; siehe Protokoll"),
    PlanSaveShort => ("Save", "Speichern"),
    PlanLoadShort => ("Load", "Laden"),
    PlanSave => ("Save plan file", "Plan als Datei speichern"),
    PlanLoad => ("Load plan file", "Plan aus Datei laden"),
    PlanSaved => (r"Plan saved to runs\plan.toml", r"Plan in runs\plan.toml gespeichert"),
    PlanLoaded => ("Plan loaded", "Plan geladen"),
    PlanLoadFailed => ("The plan file could not be read", "Die Plan-Datei konnte nicht gelesen werden"),
    InfoSelftestFailed => ("Does not work on this processor. Nothing is written.", "Funktioniert auf diesem Prozessor nicht. Es wird nichts geschrieben."),
    Phases => ("Phases:", "Phasen:"),
    IntensityTitle => ("How thorough should Auto be?", "Wie gründlich soll Auto sein?"),
    IntensityFast => ("Fast", "Schnell"),
    IntensityNormal => ("Balanced", "Ausgewogen"),
    IntensityThorough => ("Thorough", "Gründlich"),
    IntensityFastHint => ("Short tests", "Kurze Tests"),
    IntensityNormalHint => ("Longer tests", "Längere Tests"),
    IntensityThoroughHint => ("Single steps, long tests", "Einzelne Schritte, lange Tests"),
    IntensityNote => ("Worst case: every core walks its whole way. Most stop much earlier.", "Ungünstigster Fall: Jeder Kern geht seinen ganzen Weg. Die meisten hören viel früher auf."),
    PhaseDone => ("done", "erledigt"),
    ColCore => ("Core", "Kern"),
    ColDepth => ("Depth", "Tiefe"),
    LegendHolds => ("holds now", "hält jetzt"),
    TestsTakes => ("Takes about", "Dauert etwa"),
    TestsOnCores => ("on the chosen cores", "auf den gewählten Kernen"),
    SystemTitle => ("System", "System"),
    AutoIsTwoWay => ("Auto goes both ways: a core that holds too much is taken back up.", "Auto geht in beide Richtungen: Ein Kern, der zu viel hält, wird wieder zurückgenommen."),
    LabelClock => ("CLOCK", "TAKT"),
    LabelVcore => ("VCORE", "VCORE"),
    LabelTemp => ("TEMP", "TEMP"),
    LabelPower => ("PPT", "PPT"),
    LabelTdc => ("TDC", "TDC"),
    LabelEdc => ("EDC", "EDC"),
    WatchdogActive => ("WATCHDOG: monitoring active", "WÄCHTER: Überwachung aktiv"),
    WatchdogReady => ("WATCHDOG: ready", "WÄCHTER: bereit"),
    WatchdogNoHelper => ("WATCHDOG: no processor access", "WÄCHTER: kein Prozessorzugriff"),
    SystemOk => ("System OK", "System in Ordnung"),
    SystemCheck => ("Check the setup", "Einrichtung prüfen"),
    DriverEngines => ("Driver & Engines", "Treiber & Engines"),
    AllCores => ("All cores", "Alle Kerne"),
    CurveOptimizer => ("Curve Optimizer", "Curve Optimizer"),
    AllToDeepest => ("All to the limit", "Alle auf das Limit"),
    BiosValues => ("Values from start", "Werte vom Start"),
    AtChipLimit => ("at the chip limit", "am Chip-Limit"),
    StateTestingNow => ("Testing", "Wird getestet"),
    ApplyCo => ("Apply CO values", "CO-Werte anwenden"),
    ApplyLive => ("Apply live", "Live anwenden"),
    ApplyLiveHint => ("takes effect at once · gone after a restart", "wirkt sofort · nach Neustart weg"),
    BiosList => ("BIOS list", "BIOS-Liste"),
    BiosListHint => ("permanent · enter by hand in the UEFI", "dauerhaft · von Hand im UEFI eintragen"),
    EveryStart => ("On every Windows start", "Bei jedem Windows-Start"),
    EveryStartHint => ("permanent · scheduled task · needs administrator rights", "dauerhaft · geplante Aufgabe · braucht Administratorrechte"),
    TabAuto => ("Auto", "Auto"),
    TabTests => ("Tests", "Tests"),
    TabAdvanced => ("Advanced", "Erweitert"),
    CampaignReady => ("Ready to start", "Bereit zum Start"),
    CampaignInProgress => ("Campaign still in progress", "Kampagne läuft noch"),
    CampaignDone => ("All three steps are done", "Alle drei Schritte sind erledigt"),
    CampaignSetup => ("Set up first", "Erst einrichten"),
    CampaignSetupBody => ("The driver or a test program is missing. Open Driver & Engines.", "Treiber oder Testprogramm fehlt. Treiber & Engines öffnen."),
    CampaignResume => ("Interrupted: nothing is lost, it carries on from the same place.", "Unterbrochen: Nichts geht verloren, es geht an derselben Stelle weiter."),
    CampaignFresh => ("Three steps find the best value of every core and prove it. The first one only checks what the current values hold.", "Drei Schritte finden den besten Wert jedes Kerns und beweisen ihn. Der erste prüft nur, was die aktuellen Werte halten."),
    Step1 => ("Check what the current values hold", "Prüfen, was die aktuellen Werte halten"),
    Step2 => ("Lower each core until it gives", "Jeden Kern absenken, bis er nachgibt"),
    Step3 => ("Prove the result under every load type", "Das Ergebnis unter jeder Lastart beweisen"),
    StopAnyTime => ("You can stop any time and carry on later: it resumes from the same place.", "Sie können jederzeit anhalten und später weitermachen: Es setzt an derselben Stelle fort."),
    Continue => ("Continue", "Fortsetzen"),
    RunThisTest => ("Run this test", "Diesen Test ausführen"),
    TestsIntro => ("Ready-made test lists. They test at the values the cores hold now and change nothing.", "Fertige Testlisten. Sie testen bei den Werten, die die Kerne jetzt haben, und ändern nichts."),
    Recommended => ("recommended", "empfohlen"),
    AdvPageTest => ("Test", "Test"),
    AdvPageRun => ("Run", "Ablauf"),
    AdvOnError => ("On an error", "Bei einem Fehler"),
    OnErrorContinue => ("Go on", "Weiter"),
    OnErrorSkip => ("Skip core", "Kern aus"),
    OnErrorStop => ("Stop", "Anhalten"),
    OnErrorTip => ("What a test of the held values does when a core fails: test it again next round, leave it out of the later rounds, or end the run. A search is not affected: failing is how it finds the edge.", "Was ein Test bei den gehaltenen Werten tut, wenn ein Kern durchfällt: ihn in der nächsten Runde noch einmal testen, ihn in den späteren Runden auslassen oder den Lauf beenden. Eine Suche ist nicht betroffen: Durchfallen ist ihr Weg zur Grenze."),
    AdvSound => ("Sound at an error and at the end", "Ton bei Fehler und am Ende"),
    SoundTip => ("The sound of the system: one when a core fails in a test of the held values, one when the run is over.", "Der Systemton: einer, wenn ein Kern bei einem Test der gehaltenen Werte durchfällt, einer, wenn der Lauf zu Ende ist."),
    OrderPairs => ("Pairs", "Paare"),
    SecondsAuto => ("Auto", "Auto"),
    SecondsAutoTip => ("Until the test program has been through its whole work list once, but at most an hour.", "Bis das Testprogramm seine ganze Arbeitsliste einmal durchlaufen hat, höchstens eine Stunde."),
    FftCustom => ("Custom", "Eigener"),
    FftFrom => ("from", "von"),
    FftTo => ("to", "bis"),
    AdvYcSet => ("Algorithms", "Algorithmen"),
    YcSetCurve => ("Curve Optimizer", "Curve Optimizer"),
    YcSetFast => ("Quick", "Schnell"),
    AdvRounds => ("Rounds", "Runden"),
    AdvTransient => ("Load changes", "Lastwechsel"),
    AdvOrder => ("Core order", "Kernreihenfolge"),
    TransientOff => ("Off", "Aus"),
    TransientPauses => ("Pauses", "Pausen"),
    TransientBursts => ("Bursts", "Schübe"),
    TransientTip => ("The test is frozen for a moment (pauses) or in fractions of a second (bursts), so the core drops to idle and has to boost back up. That change of load finds a Curve Optimizer value that steady load never does.", "Der Test wird kurz angehalten (Pausen) oder im Sekundenbruchteil (Schübe), damit der Kern in den Leerlauf fällt und wieder hochtakten muss. Dieser Lastwechsel findet einen Curve-Optimizer-Wert, den gleichmäßige Last nie findet."),
    OrderSequential => ("In order", "Fortlaufend"),
    OrderAlternate => ("Alternating", "Abwechselnd"),
    OrderRandom => ("Random", "Zufällig"),
    OrderTip => ("Alternating jumps between the two halves of the cores, so neighbours are never tested back to back and the heat of one does not soak into the next. Pairs hands the load from every core to every other, which takes many tests.", "Abwechselnd springt zwischen den beiden Hälften der Kerne, damit Nachbarn nie hintereinander getestet werden und die Wärme des einen nicht in den nächsten zieht. Paare gibt die Last von jedem Kern an jeden anderen weiter, das dauert viele Tests."),
    AdvMode => ("Instructions", "Befehlssatz"),
    AdvThreads => ("Both threads", "Beide Threads"),
    ModeLabel => ("Prime95 instruction set", "Prime95-Befehlssatz"),
    FftLabel => ("FFT range", "FFT-Bereich"),
    ForgetMemory => ("Forget what was learned", "Gelerntes vergessen"),
    Forgotten => ("The memory of the search was cleared", "Das Gedächtnis der Suche wurde gelöscht"),
    LiveTelemetry => ("Live telemetry", "Live-Messwerte"),
    LogButton => ("Log", "Protokoll"),
    Shortcuts => ("F5 start/stop · Ctrl+S apply live · Ctrl+L language", "F5 Start/Stopp · Strg+S live anwenden · Strg+L Sprache"),
    BiosTitle => ("Values for the BIOS", "Werte für das BIOS"),
    BiosBody => (
        "How to enter them:
1. Restart into the UEFI (usually the Del key at start-up).
2. Advanced, AMD Overclocking, Precision Boost Overdrive, Curve Optimizer (the names differ a little between board makers).
3. Curve Optimizer: Per Core.
4. For each core, Sign: Negative, Magnitude: the number without the minus (-27 becomes 27).
5. Save and restart. Come back here and look at Values from start: the numbers must be the ones listed, on the same cores.
The values in the UEFI survive a restart; the live values do not. How the cores are counted can differ between boards, so check that core 0 here is core 0 there.",
        "So geht es:
1. In das UEFI neu starten (meist mit der Taste Entf beim Hochfahren).
2. Advanced, AMD Overclocking, Precision Boost Overdrive, Curve Optimizer (die Namen sind je nach Mainboard-Hersteller etwas anders).
3. Curve Optimizer: Per Core.
4. Für jeden Kern Sign: Negative, Magnitude: die Zahl ohne Minus (-27 wird 27).
5. Speichern und neu starten. Hier \"Werte vom Start\" ansehen: die Zahlen müssen die der Liste sein, bei denselben Kernen.
Die Werte im UEFI überstehen einen Neustart, die Live-Werte nicht. Wie die Kerne gezählt werden, kann je Mainboard anders sein; prüfe, dass Kern 0 hier auch Kern 0 dort ist."
    ),
    BiosNegative => ("Negative", "Negativ"),
    EnterInBios => ("Enter in the BIOS …", "Ins BIOS eintragen …"),
    EnterInBiosHint => ("These values are gone after a restart unless they are entered in the UEFI or kept for logon.", "Nach einem Neustart sind diese Werte weg, wenn sie nicht im UEFI eingetragen oder für die Anmeldung behalten werden."),
    Copy => ("Copy", "Kopieren"),
    Copied => ("Copied", "Kopiert"),
    Close => ("Close", "Schließen"),
    SelectedCores => ("cores selected", "Kerne gewählt"),
    PendingApplied => ("Values applied", "Werte angewendet"),
    NothingToApply => ("Nothing to apply: no value was changed", "Nichts anzuwenden: Es wurde kein Wert geändert"),
    LogOlder => ("Earlier today", "Früher heute"),
    HealthTip => ("Driver, test programs and the logon task", "Treiber, Testprogramme und Anmelde-Aufgabe"),
    RunningTitle => ("Running", "Läuft"),
    StartedToast => ("Started", "Gestartet"),
    PlanCustomRun => ("Run this plan", "Diesen Plan ausführen"),
}

/// `Plan: all cores, about 26:05 at most` as a format, not a guess.
pub fn plan_sentence(lang: Lang, core_count: usize, all: bool, longest_seconds: u64) -> String {
    let time = pbo_core::duration::hours(longest_seconds as i64);
    match (lang, all) {
        (Lang::En, true) => format!("Plan: all {core_count} cores, {time} at most"),
        (Lang::En, false) => format!("Plan: {core_count} selected core(s), {time} at most"),
        (Lang::De, true) => format!("Plan: alle {core_count} Kerne, höchstens {time}"),
        (Lang::De, false) => format!("Plan: {core_count} gewählte(r) Kern(e), höchstens {time}"),
    }
}

pub fn adjusted(lang: Lang, parts: usize) -> String {
    match lang {
        Lang::En => format!("{parts} part(s) adjusted"),
        Lang::De => format!("{parts} Teil(e) angepasst"),
    }
}

/// What to say after a value was written and read back.
pub fn core_set_result(lang: Lang, core: usize, asked: i32, held: i32) -> String {
    match (lang, asked == held) {
        (Lang::En, true) => format!("Core {core}: now {held}"),
        (Lang::En, false) => format!("Core {core}: asked for {asked}, the processor keeps {held}"),
        (Lang::De, true) => format!("Kern {core}: jetzt {held}"),
        (Lang::De, false) => format!("Kern {core}: verlangt {asked}, der Prozessor behält {held}"),
    }
}

/// "Testing core 4 (step 2)". A run that is not part of the campaign (a test list, a plan) has no step: phase 0.
pub fn progress(lang: Lang, phase: u32, core: u32) -> String {
    match (lang, phase) {
        (Lang::En, 0) => format!("Testing core {core}"),
        (Lang::De, 0) => format!("Kern {core} wird getestet"),
        (Lang::En, _) => format!("Testing core {core} (step {phase})"),
        (Lang::De, _) => format!("Kern {core} wird getestet (Schritt {phase})"),
    }
}

/// A span of minutes in words: "9 h 20 min", "45 min".
pub fn span(lang: Lang, minutes: f64) -> String {
    let total = minutes.max(0.0).round() as i64;
    let (h, m) = (total / 60, total % 60);
    let (hour, min) = match lang {
        Lang::En => ("h", "min"),
        Lang::De => ("Std.", "Min."),
    };
    match (h, m) {
        // Nothing to do is said in hours, like the long spans are: "0 h".
        (0, 0) => format!("0 {hour}"),
        (0, m) => format!("{m} {min}"),
        (h, 0) => format!("{h} {hour}"),
        (h, m) => format!("{h} {hour} {m} {min}"),
    }
}

/// A span for a small place: "3:52 h" from an hour on, "45 min" below.
pub fn span_short(minutes: f64) -> String {
    let total = minutes.max(0.0).round() as i64;
    if total == 0 {
        "0 h".to_string()
    } else if total < 60 {
        format!("{total} min")
    } else {
        format!("{}:{:02} h", total / 60, total % 60)
    }
}

/// "up to 9 h 20 min": a worst case, said as one.
pub fn up_to(lang: Lang, minutes: f64) -> String {
    match lang {
        Lang::En => format!("up to {}", span(lang, minutes)),
        Lang::De => format!("bis zu {}", span(lang, minutes)),
    }
}

/// "3 changes": what is waiting to be applied.
pub fn pending(lang: Lang, n: usize) -> String {
    match (lang, n) {
        (Lang::En, 1) => "1 change".to_string(),
        (Lang::En, n) => format!("{n} changes"),
        (Lang::De, 1) => "1 Änderung".to_string(),
        (Lang::De, n) => format!("{n} Änderungen"),
    }
}

/// The name of a ready-made test list short enough for a button.
pub fn profile_short(lang: Lang, id: &str) -> &'static str {
    let (en, de) = match id {
        "RecommendedCombo" => ("Recommended", "Empfohlen"),
        "HeavyFfts" => ("Heavy FFTs", "Heavy FFTs"),
        "FullPrime95" => ("Full Prime95", "Prime95 komplett"),
        "QuickCheck" => ("Quick check", "Schnellcheck"),
        "BreakingPoint" => ("Breaking point", "Grenzwert"),
        "HeavyLoad" => ("Heavy load", "Volllast"),
        "YCruncherCo" => ("y-cruncher CO", "y-cruncher CO"),
        "YCruncherAll" => ("y-cruncher all", "y-cruncher alle"),
        "Step1Sse" => ("Step 1 · SSE", "Schritt 1 · SSE"),
        "Step2Avx2" => ("Step 2 · AVX2", "Schritt 2 · AVX2"),
        "SmallFfts" => ("Small FFTs", "Kleine FFTs"),
        "Overnight" => ("Overnight", "Über Nacht"),
        "MemoryFabric" => ("Memory & Fabric", "Speicher & Fabric"),
        "MicroBurst" => ("Micro-bursts", "Micro-Bursts"),
        _ => ("?", "?"),
    };
    match lang {
        Lang::En => en,
        Lang::De => de,
    }
}

/// "About 20:42 hrs left (from phase 2 of 3)", the time as a format that takes the number.
pub fn time_left(lang: Lang, hours: &str, phase: i32) -> String {
    match lang {
        Lang::En => format!("About {hours} hrs left (from phase {phase} of 3)"),
        Lang::De => format!("Etwa {hours} Std. übrig (ab Phase {phase} von 3)"),
    }
}

/// The name and a short explanation of a ready-made test list.
pub fn profile_text(lang: Lang, id: &str) -> (&'static str, &'static str) {
    let (en, de): ((&str, &str), (&str, &str)) = match id {
        "RecommendedCombo" => (
            ("Recommended: Prime95 SSE and y-cruncher", "Two different kinds of load say more than one: Prime95 for peak boost, then y-cruncher for heavy vector and cache load."),
            ("Empfohlen: Prime95 SSE und y-cruncher", "Zwei verschiedene Lastarten sagen mehr als eine: Prime95 für den Höchsttakt, danach y-cruncher für schwere Vektor- und Cache-Last."),
        ),
        "HeavyFfts" => (
            ("Heavy FFTs: the CoreCycler classic", "The in-cache FFT sizes that expose an undervolted core fastest. One engine, so cores that lock early are not wasted."),
            ("Heavy FFTs: der CoreCycler-Klassiker", "Die FFT-Größen im Cache, die einen unterversorgten Kern am schnellsten entlarven. Ein Testprogramm, damit früh eingerastete Kerne nicht verschwendet werden."),
        ),
        "FullPrime95" => (
            ("Full run: Prime95 SSE and AVX2", "Both instruction sets, one after the other. Prime95 only."),
            ("Komplett: Prime95 SSE und AVX2", "Beide Befehlssätze nacheinander. Nur Prime95."),
        ),
        "QuickCheck" => (
            ("Quick check (3 min per core)", "Half an hour for eight cores, for the obvious outliers."),
            ("Schnellcheck (3 Min. je Kern)", "Eine halbe Stunde für acht Kerne, für die offensichtlichen Ausreißer."),
        ),
        "BreakingPoint" => (
            ("Breaking point: hard load transitions", "AVX2 on two threads, interrupted every five seconds so the core drops to idle and boosts back up."),
            ("Grenzwert: harte Lastwechsel", "AVX2 auf zwei Threads, alle fünf Sekunden unterbrochen, damit der Kern in den Leerlauf fällt und wieder hochtaktet."),
        ),
        "HeavyLoad" => (
            ("Heavy load: cooling and PBO limits", "Steady AVX2 load on two threads: shows what the cooler and the power limits do."),
            ("Volllast: Kühlung und PBO-Grenzen", "Gleichmäßige AVX2-Last auf zwei Threads: zeigt, was Kühler und Leistungsgrenzen tun."),
        ),
        "YCruncherCo" => (
            ("y-cruncher: Curve Optimizer", "The algorithms most often credited with catching Curve Optimizer instability that Prime95 runs past."),
            ("y-cruncher: Curve Optimizer", "Die Algorithmen, die am häufigsten Curve-Optimizer-Instabilität finden, die Prime95 übersieht."),
        ),
        "YCruncherAll" => (
            ("y-cruncher: all algorithms", "Every algorithm y-cruncher offers, twice."),
            ("y-cruncher: alle Algorithmen", "Jeder Algorithmus, den y-cruncher anbietet, zweimal."),
        ),
        "Step1Sse" => (
            ("Step 1: SSE (start here)", "The gentlest instruction set: peak boost and the voltage droop limits."),
            ("Schritt 1: SSE (hier anfangen)", "Der mildeste Befehlssatz: Höchsttakt und die Grenzen des Spannungseinbruchs."),
        ),
        "Step2Avx2" => (
            ("Step 2: AVX2 (after that)", "Heavier load, more heat, a lower clock: the step after SSE."),
            ("Schritt 2: AVX2 (danach)", "Mehr Last, mehr Wärme, niedrigerer Takt: der Schritt nach SSE."),
        ),
        "SmallFfts" => (
            ("Small FFTs (fast discovery)", "The smallest FFT sizes: the quickest way to find the first weak core."),
            ("Kleine FFTs (Schnellsuche)", "Die kleinsten FFT-Größen: der schnellste Weg zum ersten schwachen Kern."),
        ),
        "Overnight" => (
            ("Overnight: thorough validation", "All FFT sizes with three instruction sets, then y-cruncher. The proof before the values go into the BIOS."),
            ("Über Nacht: gründliche Absicherung", "Alle FFT-Größen mit drei Befehlssätzen, danach y-cruncher. Der Beweis, bevor die Werte ins BIOS gehen."),
        ),
        "MemoryFabric" => (
            ("Memory controller and Infinity Fabric", "Large FFT sizes that pull in the memory controller as well."),
            ("Speichercontroller und Infinity Fabric", "Große FFT-Größen, die auch den Speichercontroller beanspruchen."),
        ),
        "MicroBurst" => (
            ("Micro-bursts: game-style load changes", "Load switched on and off in fractions of a second, hundreds of times a minute, the way a game engine does."),
            ("Micro-Bursts: Lastwechsel wie im Spiel", "Last, die im Sekundenbruchteil an- und abgeschaltet wird, hunderte Male pro Minute, wie eine Spiele-Engine."),
        ),
        _ => return ("?", ""),
    };
    match lang {
        Lang::En => en,
        Lang::De => de,
    }
}

/// "6 min × 3 · 1 thread · Prime95 SSE → y-cruncher", the numbers of a profile in one line.
pub fn profile_meta(lang: Lang, p: &pbo_core::profiles::Profile) -> String {
    use pbo_core::profiles::Engine;
    let legs: Vec<String> = p
        .legs
        .iter()
        .map(|l| match l.engine {
            Engine::Prime95 => format!("Prime95 {}", l.mode.name().to_uppercase()),
            Engine::YCruncher => "y-cruncher".to_string(),
        })
        .collect();
    let threads = match (lang, p.threads) {
        (Lang::En, 1) => "1 thread".to_string(),
        (Lang::En, n) => format!("{n} threads"),
        (Lang::De, 1) => "1 Thread".to_string(),
        (Lang::De, n) => format!("{n} Threads"),
    };
    let per_core = match lang {
        Lang::En => "min/core",
        Lang::De => "Min./Kern",
    };
    use pbo_core::transient::Transient;
    let transient = match p.transient_mode() {
        Transient::Off => String::new(),
        Transient::Bursts => match lang {
            Lang::En => " · micro-bursts".to_string(),
            Lang::De => " · Micro-Bursts".to_string(),
        },
        Transient::Periodic { every_s, .. } => match lang {
            Lang::En => format!(" · pause every {every_s} s"),
            Lang::De => format!(" · Pause alle {every_s} s"),
        },
    };
    format!(
        "{} {per_core} × {} · {threads} · {}{transient}",
        p.minutes_per_core,
        p.iterations,
        legs.join(" → ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_text_is_empty_and_no_translation_is_a_copy_by_accident() {
        // Where English and German are the same on purpose: names, abbreviations, one word that is shared.
        let allow_equal = [
            Key::TabPlan,
            Key::Start,
            Key::InfoBios,
            Key::NotAvailable,
            Key::LabelTemp,
            Key::LabelVcore,
            Key::LabelPower,
            Key::LabelTdc,
            Key::LabelEdc,
            Key::CurveOptimizer,
            Key::TabAuto,
            Key::TabTests,
            Key::SystemOk,
            Key::SystemTitle,
            Key::AdvPageTest,
            Key::SecondsAuto,
            Key::YcSetCurve,
        ];
        for k in Key::ALL {
            let (en, de) = (tr(Lang::En, *k), tr(Lang::De, *k));
            assert!(!en.is_empty() && !de.is_empty(), "{k:?}");
            assert!(
                en != de || allow_equal.contains(k),
                "{k:?} has the same text in both languages: {en}"
            );
            assert!(
                !en.contains('!') && !de.contains('!'),
                "{k:?}: no exclamation marks"
            );
        }
    }

    #[test]
    fn the_plan_sentence_carries_its_numbers() {
        assert_eq!(
            plan_sentence(Lang::En, 8, true, 3 * 3600 + 5 * 60),
            "Plan: all 8 cores, 3:05 at most"
        );
        assert_eq!(
            plan_sentence(Lang::De, 2, false, 600),
            "Plan: 2 gewählte(r) Kern(e), höchstens 0:10"
        );
    }
}

#[cfg(test)]
mod span_tests {
    use super::*;

    #[test]
    fn a_span_is_said_in_hours_and_minutes() {
        assert_eq!(span(Lang::En, 560.0), "9 h 20 min");
        assert_eq!(span(Lang::De, 560.0), "9 Std. 20 Min.");
        assert_eq!(span(Lang::En, 45.0), "45 min");
        assert_eq!(span(Lang::En, 120.0), "2 h");
        assert_eq!(span(Lang::En, 0.4), "0 h");
        assert_eq!(span(Lang::De, 0.0), "0 Std.");
        assert_eq!(span_short(0.0), "0 h");
        assert_eq!(span_short(45.0), "45 min");
        assert_eq!(up_to(Lang::De, 90.0), "bis zu 1 Std. 30 Min.");
    }

    #[test]
    fn a_run_outside_the_campaign_has_no_step() {
        assert_eq!(progress(Lang::En, 0, 4), "Testing core 4");
        assert_eq!(progress(Lang::De, 0, 4), "Kern 4 wird getestet");
        assert_eq!(progress(Lang::En, 2, 4), "Testing core 4 (step 2)");
    }

    #[test]
    fn what_is_pending_has_a_singular() {
        assert_eq!(pending(Lang::En, 1), "1 change");
        assert_eq!(pending(Lang::De, 3), "3 Änderungen");
    }

    #[test]
    fn every_test_list_has_a_short_name() {
        for p in pbo_core::profiles::all() {
            assert_ne!(profile_short(Lang::En, p.id), "?", "{}", p.id);
            assert!(
                profile_short(Lang::En, p.id).chars().count() <= 18,
                "{}",
                p.id
            );
            assert!(
                profile_short(Lang::De, p.id).chars().count() <= 18,
                "{}",
                p.id
            );
        }
    }
}
