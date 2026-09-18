using System.Text.Json;

namespace PboStudio.Core;

/// <summary>
/// Everything the user configures that is not a Curve Optimizer value. Those live in
/// co_saved.json because the autostart task reads them; this is the rest of the window.
/// </summary>
public sealed class AppSettings
{
    /// <summary>Not called "Language": that would shadow the enum of the same name.</summary>
    public string UiLanguage { get; set; } = nameof(PboStudio.Core.Language.English);

    // Test tab
    /// <summary>
    /// Which profile, by identity. The old <c>ProfileIndex</c> pointed into the profile list,
    /// which is now filtered down to five entries by default — an index into it no longer means
    /// the same profile from one session to the next.
    /// </summary>
    public string ProfileId { get; set; } = "";

    /// <summary>Whether the profile list shows all fourteen rather than the five on the path.</summary>
    public bool ShowAllProfiles { get; set; }

    public int AutoTunerMode { get; set; }

    // Engine tab
    public int MinutesPerCore { get; set; } = 6;
    public int Iterations { get; set; } = 3;
    public int EngineIndex { get; set; }
    public int ThreadsIndex { get; set; }
    public int ModeIndex { get; set; }
    public int FftIndex { get; set; }
    public int CustomFftMin { get; set; } = 4;
    public int CustomFftMax { get; set; } = 32;
    public int YcAlgoIndex { get; set; }
    public int PauseInterval { get; set; } = 30;
    public int PauseDuration { get; set; } = 1;
    public int OrderIndex { get; set; } = 1;
    public string CustomOrder { get; set; } = "";
    public bool StopOnError { get; set; }
    public bool SkipCoreOnError { get; set; } = true;

    /// <summary>CoreCycler's treatThreadErrorsAsRealErrors.</summary>
    public bool WorkerFaultsAreErrors { get; set; } = true;

    /// <summary>CoreCycler's flashOnError.</summary>
    public bool FlashOnError { get; set; } = true;

    /// <summary>CoreCycler's stressTestProgramPriority. 0 low, 1 normal, 2 high.</summary>
    public int StressPriority { get; set; } = 1;

    /// <summary>Prime95's TortureMem in MB. 0 keeps the whole test in cache.</summary>
    public int PrimeMemoryMb { get; set; }
    public int DelayBetweenCores { get; set; } = 2;

    // System tab
    public bool AutoRuntime { get; set; }
    public int AutoRuntimeCap { get; set; } = 60;
    public bool SpreadSmt { get; set; }
    public bool IsolateTestedCore { get; set; }
    public int YcSeconds { get; set; } = 60;
    public int YcMemoryIndex { get; set; } = 1;
    public int YcBinaryIndex { get; set; }
    public string YcCustomAlgorithms { get; set; } = "";

    public int MaxTemp { get; set; } = 90;
    public bool TreatWheaWarningAsError { get; set; } = true;

    /// <summary>Defaults to on: the restore point is cheap and the failure mode it covers is not.</summary>
    public bool CreateRestorePoint { get; set; } = true;

    /// <summary>Points of headroom between the measured boundary and the locked value.</summary>
    public int Guardband { get; set; } = AutoTunerService.DefaultGuardband;
    public int PostTestAction { get; set; }
    public string WebhookUrl { get; set; } = "";

    // Window
    /// <summary>Whether the graph/log dock was left collapsed. Worth keeping: on a dense
    /// setup it is a permanent preference, not a per-session one.</summary>
    public bool BottomDockCollapsed { get; set; }

    public double WindowWidth { get; set; }
    public double WindowHeight { get; set; }
}

/// <summary>
/// Reads and writes <see cref="AppSettings"/>. Nothing here is allowed to throw: losing a
/// preferences file must never stop the program from starting.
/// </summary>
public static class SettingsService
{
    private const string FileName = "settings.json";

    private static readonly JsonSerializerOptions Options = new() { WriteIndented = true };

    private static string PathFor(string workRoot) => Path.Combine(workRoot, FileName);

    public static AppSettings Load(string workRoot)
    {
        try
        {
            string path = PathFor(workRoot);
            if (!File.Exists(path)) return new AppSettings();

            return JsonSerializer.Deserialize<AppSettings>(File.ReadAllBytes(path), Options)
                   ?? new AppSettings();
        }
        catch
        {
            // Corrupt or unreadable: fall back to defaults rather than refusing to start.
            return new AppSettings();
        }
    }

    public static void Save(string workRoot, AppSettings settings)
    {
        try
        {
            Directory.CreateDirectory(workRoot);
            File.WriteAllBytes(PathFor(workRoot), JsonSerializer.SerializeToUtf8Bytes(settings, Options));
        }
        catch
        {
        }
    }
}
