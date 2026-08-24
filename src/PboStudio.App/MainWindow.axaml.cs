using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Text;
using Avalonia.Controls;
using Avalonia.Controls.Documents;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media;
using Avalonia.Threading;
using PboStudio.Core;

namespace PboStudio.App;

/// <summary>Where a core stands in the current run. Drives text, glyph and colour together.</summary>
public enum CoreState
{
    Ready,
    Waiting,
    Running,
    Passed,
    Failed,
    Skipped,
    Locked,
}

public sealed class CoreRow : INotifyPropertyChanged
{
    private decimal _margin;
    private bool _selected = true;
    private bool _isLocked;
    private CoreState _state = CoreState.Ready;

    public int Index { get; init; }
    public int TotalCores { get; set; } = 8;

    /// <summary>Total chiplets on this processor, and which one this core sits on. Both come
    /// from <see cref="CoreTopology.ResolveCcds"/> rather than from arithmetic on the index.</summary>
    public int CcdCount { get; set; } = 1;
    public int CcdIndex { get; set; }

    /// <summary>Whether this core takes part in the next run.</summary>
    public bool Selected
    {
        get => _selected;
        set { if (_selected == value) return; _selected = value; Raise(); }
    }

    // ─────────────────────────── identity ───────────────────────────

    public bool HasCcd => CcdCount > 1;
    public string CcdLabel => $"CCD{CcdIndex}";
    public string CcdTooltip => LocalizationService.Get("CcdTooltip");

    /// <summary>Chiplets alternate tint so the eye can group them without reading the label.</summary>
    public IBrush CcdBrush => (CcdIndex % 2) == 0
        ? SolidColorBrush.Parse("#1E3A52")
        : SolidColorBrush.Parse("#3A2C52");

    public string CoreName => LocalizationService.Pick("Kern", "Core") + $" {Index}";

    /// <summary>Fixed so every row lines up; wide enough for "[CCD1] Kern 15 🥇 🔒".</summary>
    public double LabelWidth => HasCcd ? 176 : 128;

    public CoreQualityRank QualityRank { get; set; } = CoreQualityRank.Standard;

    /// <summary>Raw CPPC value behind the badge, so the tooltip can show the evidence.</summary>
    public int CppcPerformance { get; set; }
    public bool HasQuality => QualityRank != CoreQualityRank.Standard;
    public string QualityBadge => QualityRank switch
    {
        CoreQualityRank.GoldStar => "🥇",
        CoreQualityRank.SilverStar => "🥈",
        _ => "",
    };
    public string QualityTooltip => QualityRank switch
    {
        CoreQualityRank.GoldStar => string.Format(LocalizationService.Get("GoldCoreTooltip"), CppcPerformance),
        CoreQualityRank.SilverStar => string.Format(LocalizationService.Get("SilverCoreTooltip"), CppcPerformance),
        _ => "",
    };

    public string SelectTooltip => LocalizationService.Get("CoreTooltip");
    public string MarginTooltip => LocalizationService.Get("ColumnCoTooltip");
    public string DeltaTooltip => DeltaTooltipText;
    public string LockedTooltip => LocalizationService.Get("LockedTooltip");

    public bool IsLocked
    {
        get => _isLocked;
        set
        {
            if (_isLocked == value) return;
            _isLocked = value;
            Raise();
            Raise(nameof(LockedTooltip));
        }
    }

    // ─────────────────────────── value ───────────────────────────

    public decimal MinLimit { get; set; } = -30m;
    public decimal MaxLimit { get; set; } = 30m;
    /// <summary>
    /// What this core held when the program started, i.e. what the BIOS set. Never changes for
    /// the lifetime of the row, so "reset" has something stable to return to.
    /// </summary>
    public decimal BiosBaseline { get; set; }

    /// <summary>
    /// What the CPU currently holds. Moves every time a value is written to the SMU, which is
    /// what makes <see cref="IsChanged"/> mean "edited but not yet applied".
    /// <para>
    /// These two were a single field called Original. Because applying a value overwrote it,
    /// "reset to BIOS values" quietly became "reset to the value I last applied" - and after an
    /// auto-tuner run, which writes every core, it did nothing at all.
    /// </para>
    /// </summary>
    public decimal Applied
    {
        get => _applied;
        set { _applied = value; Raise(); RaiseValueDerived(); }
    }
    private decimal _applied;

    public bool IsChanged => Margin != Applied;

    public decimal Margin
    {
        get => _margin;
        set
        {
            decimal clamped = Math.Clamp(value, MinLimit, MaxLimit);
            if (_margin == clamped) return;
            _margin = clamped;
            Raise();
            RaiseValueDerived();
        }
    }

    /// <summary>How far this core has travelled toward the chip's negative limit, 0..1.</summary>
    public double RiskFraction => _margin >= 0 || MinLimit >= 0
        ? 0
        : Math.Clamp((double)(_margin / MinLimit), 0, 1);

    public IBrush RiskBrush
    {
        get
        {
            if (_margin > 0) return SolidColorBrush.Parse("#38BDF8");   // adding voltage
            if (_margin == 0) return SolidColorBrush.Parse("#222838");  // untouched
            double f = RiskFraction;
            if (f < 0.45) return SolidColorBrush.Parse("#34D399");
            if (f < 0.70) return SolidColorBrush.Parse("#FBBF24");
            if (f < 0.90) return SolidColorBrush.Parse("#F97316");
            return SolidColorBrush.Parse("#F87171");
        }
    }

    public string RiskTooltip => LocalizationService.Pick(
        $"{RiskFraction * 100:F0} % des Weges bis zum Chip-Limit ({MinLimit}) ausgereizt.",
        $"{RiskFraction * 100:F0} % of the way toward the chip limit ({MinLimit}).");

    /// <summary>Distance from the BIOS setting, which is the number a person actually tracks.</summary>
    public string DeltaLabel
    {
        get
        {
            decimal d = Margin - BiosBaseline;
            if (d == 0) return "";
            return d > 0 ? $"▲+{d:0}" : $"▼{d:0}";
        }
    }

    /// <summary>Amber while the edit is only on screen, muted once the CPU actually holds it.</summary>
    public IBrush DeltaBrush => SolidColorBrush.Parse(IsChanged ? "#FBBF24" : "#64748B");

    public string DeltaTooltipText => IsChanged
        ? LocalizationService.Pick(
            $"Noch nicht angewendet. BIOS-Wert war {BiosBaseline:0}, die CPU hält gerade {Applied:0}.",
            $"Not applied yet. The BIOS value was {BiosBaseline:0}; the CPU currently holds {Applied:0}.")
        : LocalizationService.Pick(
            $"Angewendet. BIOS-Wert war {BiosBaseline:0}.",
            $"Applied. The BIOS value was {BiosBaseline:0}.");

    // ─────────────────────────── status ───────────────────────────

    public CoreState State
    {
        get => _state;
        set { _state = value; RaiseStatusDerived(); }
    }

    /// <summary>Margin the auto-tuner settled on, shown inside the Locked pill.</summary>
    public decimal? LockedValue { get; set; }

    public string Status => State switch
    {
        CoreState.Ready => LocalizationService.Get("StatusReady"),
        CoreState.Waiting => LocalizationService.Get("StatusWaiting"),
        CoreState.Running => LocalizationService.Get("StatusRunning"),
        CoreState.Passed => LocalizationService.Get("StatusPassed"),
        CoreState.Failed => LocalizationService.Get("StatusFailed"),
        CoreState.Skipped => LocalizationService.Get("StatusSkipped"),
        CoreState.Locked => $"{LocalizationService.Get("StatusLocked")} ({LockedValue ?? Margin:0})",
        _ => "",
    };

    /// <summary>Shape carries the state as well as colour, for red/green colour blindness.</summary>
    public string StatusGlyph => State switch
    {
        CoreState.Ready => "○",
        CoreState.Waiting => "…",
        CoreState.Running => "●",
        CoreState.Passed => "✓",
        CoreState.Failed => "✕",
        CoreState.Skipped => "–",
        CoreState.Locked => "🔒",
        _ => "",
    };

    public IBrush StatusColor => SolidColorBrush.Parse(State switch
    {
        CoreState.Ready => "#94A3B8",
        CoreState.Waiting => "#94A3B8",
        CoreState.Running => "#38BDF8",
        CoreState.Passed => "#34D399",
        CoreState.Failed => "#F87171",
        CoreState.Skipped => "#64748B",
        CoreState.Locked => "#A78BFA",
        _ => "#94A3B8",
    });

    public IBrush StatusBackground => SolidColorBrush.Parse(State switch
    {
        CoreState.Running => "#0C1924",
        CoreState.Passed => "#0C1A1A",
        CoreState.Failed => "#2E1418",
        CoreState.Locked => "#16122A",
        _ => "#1A1E2B",
    });

    public void RefreshLocalization()
    {
        Raise(nameof(CoreName));
        Raise(nameof(CcdTooltip));
        Raise(nameof(QualityTooltip));
        Raise(nameof(SelectTooltip));
        Raise(nameof(MarginTooltip));
        Raise(nameof(DeltaTooltip));
        Raise(nameof(LockedTooltip));
        Raise(nameof(RiskTooltip));
        RaiseStatusDerived();
    }

    private void RaiseValueDerived()
    {
        Raise(nameof(DeltaBrush));
        Raise(nameof(DeltaTooltip));
        Raise(nameof(RiskFraction));
        Raise(nameof(RiskBrush));
        Raise(nameof(RiskTooltip));
        Raise(nameof(DeltaLabel));
        Raise(nameof(IsChanged));
    }

    private void RaiseStatusDerived()
    {
        Raise(nameof(Status));
        Raise(nameof(StatusGlyph));
        Raise(nameof(StatusColor));
        Raise(nameof(StatusBackground));
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    private void Raise([CallerMemberName] string? name = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}

public enum LogLevel { Info, Success, Warn, Error }

public partial class MainWindow : Window
{
    private readonly ObservableCollection<CoreRow> _rows = [];
    private readonly SmuService _smu = new();
    private readonly IReadOnlyList<PhysicalCore> _cores = CoreTopology.Enumerate();

    private CancellationTokenSource? _runCts;
    private bool _running;
    private bool _suppressSelectAll;

    private readonly DispatcherTimer _clock = new() { Interval = TimeSpan.FromSeconds(1) };
    private DateTime _runStarted;
    private TimeSpan _plannedTotal;
    private string _phaseLabel = "";
    private ActiveRunState? _interruptedState;

    private EngineStatus? _dependencies;
    private readonly CcdLayout _ccdLayout;
    private readonly AppSettings _settings;
    private bool _settingsApplied;
    private readonly StringBuilder _logBuffer = new();
    private readonly StreamWriter? _logFile;
    private readonly string? _logDirectory;

    private const double TelemetryIntervalSeconds = 2.0;

    public MainWindow()
    {
        InitializeComponent();

        // Before anything reads LocalizationService or lays out the window.
        _settings = SettingsService.Load(SettingsRoot);
        LocalizationService.CurrentLanguage =
            _settings.UiLanguage == nameof(Language.German) ? Language.German : Language.English;

        SizeToWorkArea();
        (_logFile, _logDirectory) = OpenSessionLog();
        CoreList.ItemsSource = _rows;

        _ccdLayout = CoreTopology.ResolveCcds(_cores, _smu.ReportedCcdCount);

        BuildCoreRows();
        BuildCcdButtons();
        ApplyLocalization();
        ShowSystemInfo();
        UpdateDurationHint();

        _clock.Tick += (_, _) => UpdateClock();
        StartWatchdog();
        CheckInterruptedRun();

        // After ApplyLocalization, because LoadProfiles resets the profile selection and
        // OnProfileChanged overwrites the engine controls from the preset.
        ApplySettingsToControls();

        AutostartBox.IsChecked = TaskSchedulerService.IsAutostartEnabled();

        LiveTelemetryGraph.TempLimit = (double)(MaxTempBox.Value ?? 90m);

        // Both of these touch WMI / the file system / the SMU. Doing them in the constructor
        // froze the window for up to a second and a half before it ever painted.
        Dispatcher.UIThread.Post(async () =>
        {
            await RunSystemAuditAsync();
            await RefreshDependenciesAsync();
        }, DispatcherPriority.Background);

        // Check if launched via Windows autostart Task (--apply-saved)
        string[] args = Environment.GetCommandLineArgs();
        if (args.Contains("--apply-saved", StringComparer.OrdinalIgnoreCase))
        {
            ApplySavedProfileOnBoot();
        }

        if (_smu.IsAvailable)
        {
            var ticker = new DispatcherTimer { Interval = TimeSpan.FromSeconds(TelemetryIntervalSeconds) };
            ticker.Tick += (_, _) => UpdatePbo();
            ticker.Start();
        }
        else
        {
            SetTelemetryOffline();
        }
    }

    private static string SettingsRoot => Path.Combine(AppContext.BaseDirectory, "runs");

    /// <summary>
    /// Restores the saved configuration. Runs after the profile preset has been applied, so
    /// these values win: a preset is a starting point, not a rule.
    /// </summary>
    private void ApplySettingsToControls()
    {
        var s = _settings;

        if (ProfileBox.ItemCount > 0)
            ProfileBox.SelectedIndex = Math.Clamp(s.ProfileIndex, 0, ProfileBox.ItemCount - 1);

        MinutesBox.Value = s.MinutesPerCore;
        IterationsBox.Value = s.Iterations;
        EngineBox.SelectedIndex = Math.Clamp(s.EngineIndex, 0, 2);
        ThreadsBox.SelectedIndex = Math.Clamp(s.ThreadsIndex, 0, 1);
        ModeBox.SelectedIndex = Math.Clamp(s.ModeIndex, 0, 3);
        FftBox.SelectedIndex = Math.Clamp(s.FftIndex, 0, 8);
        FftMinBox.Value = s.CustomFftMin;
        FftMaxBox.Value = s.CustomFftMax;
        YcAlgoBox.SelectedIndex = Math.Clamp(s.YcAlgoIndex, 0, 2);
        PauseIntervalBox.Value = s.PauseInterval;
        PauseDurationBox.Value = s.PauseDuration;
        OrderBox.SelectedIndex = Math.Clamp(s.OrderIndex, 0, 3);
        CustomOrderBox.Text = s.CustomOrder;
        StopOnErrorBox.IsChecked = s.StopOnError;
        SkipCoreOnErrorBox.IsChecked = s.SkipCoreOnError;
        CoreDelayBox.Value = s.DelayBetweenCores;

        MaxTempBox.Value = s.MaxTemp;
        TreatWheaAsErrorBox.IsChecked = s.TreatWheaWarningAsError;
        PostTestActionBox.SelectedIndex = Math.Clamp(s.PostTestAction, 0, 2);
        WebhookBox.Text = s.WebhookUrl;

        AutoTunerModeBox.SelectedIndex = Math.Clamp(s.AutoTunerMode, 0, 2);

        _settingsApplied = true;
        UpdateDurationHint();
    }

    /// <summary>Captures the current configuration. Cheap enough to call on every change.</summary>
    private void SaveSettings()
    {
        if (!_settingsApplied) return;   // do not persist the half-built state during startup

        var s = _settings;
        s.UiLanguage = LocalizationService.IsGerman ? nameof(Language.German) : nameof(Language.English);

        s.ProfileIndex = ProfileBox.SelectedIndex;
        s.AutoTunerMode = AutoTunerModeBox.SelectedIndex;

        s.MinutesPerCore = (int)(MinutesBox.Value ?? 6);
        s.Iterations = (int)(IterationsBox.Value ?? 3);
        s.EngineIndex = EngineBox.SelectedIndex;
        s.ThreadsIndex = ThreadsBox.SelectedIndex;
        s.ModeIndex = ModeBox.SelectedIndex;
        s.FftIndex = FftBox.SelectedIndex;
        s.CustomFftMin = (int)(FftMinBox.Value ?? 4);
        s.CustomFftMax = (int)(FftMaxBox.Value ?? 32);
        s.YcAlgoIndex = YcAlgoBox.SelectedIndex;
        s.PauseInterval = (int)(PauseIntervalBox.Value ?? 30);
        s.PauseDuration = (int)(PauseDurationBox.Value ?? 1);
        s.OrderIndex = OrderBox.SelectedIndex;
        s.CustomOrder = CustomOrderBox.Text ?? "";
        s.StopOnError = StopOnErrorBox.IsChecked == true;
        s.SkipCoreOnError = SkipCoreOnErrorBox.IsChecked == true;
        s.DelayBetweenCores = (int)(CoreDelayBox.Value ?? 2);

        s.MaxTemp = (int)(MaxTempBox.Value ?? 90);
        s.TreatWheaWarningAsError = TreatWheaAsErrorBox.IsChecked == true;
        s.PostTestAction = PostTestActionBox.SelectedIndex;
        s.WebhookUrl = WebhookBox.Text ?? "";

        s.WindowWidth = Width;
        s.WindowHeight = Height;

        SettingsService.Save(SettingsRoot, s);
    }

    private const int KeepLogFiles = 20;

    /// <summary>
    /// A log that only lives in memory is worth least in the situation it matters most: an
    /// overnight run that ends in a bluescreen. Written through on every line, so the file
    /// survives a hard reset.
    /// </summary>
    private static (StreamWriter? Writer, string? Directory) OpenSessionLog()
    {
        try
        {
            string dir = Path.Combine(AppContext.BaseDirectory, "runs", "logs");
            System.IO.Directory.CreateDirectory(dir);

            foreach (var stale in new DirectoryInfo(dir)
                         .GetFiles("session_*.log")
                         .OrderByDescending(f => f.LastWriteTimeUtc)
                         .Skip(KeepLogFiles))
            {
                try { stale.Delete(); } catch { }
            }

            string path = Path.Combine(dir, $"session_{DateTime.Now:yyyy-MM-dd_HH-mm-ss}.log");
            var writer = new StreamWriter(path, append: true, Encoding.UTF8) { AutoFlush = true };
            return (writer, dir);
        }
        catch
        {
            // Logging must never be the reason the app fails to start.
            return (null, null);
        }
    }

    protected override void OnClosed(EventArgs e)
    {
        SaveSettings();
        _logFile?.Dispose();
        _watchdog?.Dispose();
        base.OnClosed(e);
    }

    private void OnOpenLogFolder(object? sender, RoutedEventArgs e)
    {
        if (_logDirectory is null) return;
        try { Process.Start(new ProcessStartInfo(_logDirectory) { UseShellExecute = true }); } catch { }
    }

    /// <summary>
    /// A fixed 1180x740 was both too tall for 1080p at 150 % scaling and needlessly short on a
    /// large monitor, where the extra height goes straight into visible core rows. Sizes to the
    /// work area instead, in DIPs, leaving room for the taskbar and window chrome.
    /// </summary>
    private void SizeToWorkArea()
    {
        if (Screens.Primary is not { } screen) return;

        double scaling = screen.Scaling > 0 ? screen.Scaling : 1.0;
        double availableWidth = screen.WorkingArea.Width / scaling;
        double availableHeight = screen.WorkingArea.Height / scaling;

        // A remembered size still has to fit the screen it is opening on, which may not be
        // the screen it was saved on.
        double wantWidth = _settings.WindowWidth > 0 ? _settings.WindowWidth : 1180;
        double wantHeight = _settings.WindowHeight > 0 ? _settings.WindowHeight : 900;

        Width = Math.Clamp(wantWidth, MinWidth, Math.Max(MinWidth, availableWidth - 80));
        Height = Math.Clamp(wantHeight, MinHeight, Math.Max(MinHeight, availableHeight - 90));
    }

    // ══════════════════════════════════════════════════════════════
    // Startup / dependencies
    // ══════════════════════════════════════════════════════════════

    /// <summary>
    /// Tells the user what is missing before they press start, rather than letting the run
    /// fail silently and springing a modal window on them afterwards.
    /// </summary>
    private async Task RefreshDependenciesAsync()
    {
        var status = await Task.Run(() => DependencyService.CheckDependencies(AppContext.BaseDirectory, _smu));
        _dependencies = status;

        var missing = new List<string>();
        if (!status.Prime95Available) missing.Add("Prime95");
        if (!status.YCruncherAvailable) missing.Add("y-cruncher");
        if (!status.PawnIoAvailable) missing.Add("PawnIO");

        bool canTest = status.Prime95Available || status.YCruncherAvailable;

        if (missing.Count == 0)
        {
            SetupBanner.IsVisible = false;
        }
        else
        {
            SetupBanner.IsVisible = true;
            string detail = status.PawnIoAvailable
                ? ""
                : LocalizationService.Pick(
                    " Ohne PawnIO laufen Stresstests weiterhin, nur das Setzen der CO-Werte nicht.",
                    " Without PawnIO stress tests still run; only writing CO values does not.");

            SetupBannerText.Text = (canTest
                ? LocalizationService.Pick(
                    $"Es fehlt noch: {string.Join(", ", missing)}.",
                    $"Still missing: {string.Join(", ", missing)}.")
                : LocalizationService.Pick(
                    $"Es fehlt noch: {string.Join(", ", missing)}. Ohne ein Testprogramm kann kein Lauf gestartet werden.",
                    $"Still missing: {string.Join(", ", missing)}. Without a stress engine no run can be started.")) + detail;
            SetupBannerButton.Content = LocalizationService.Get("DriverManager");
        }

        UpdateStartButtonState();
    }

    private void UpdateStartButtonState()
    {
        if (_running) return;

        bool canTest = _dependencies is null || _dependencies.Prime95Available || _dependencies.YCruncherAvailable;

        StartButton.IsEnabled = canTest;
        if (!canTest)
        {
            StartButton.Content = LocalizationService.Get("SetupRequired");
            ToolTip.SetTip(StartButton, LocalizationService.Get("SetupRequiredTooltip"));
            return;
        }

        ToolTip.SetTip(StartButton, LocalizationService.Get("StartTooltip"));
        if (AutoTunerModeBox.SelectedIndex > 0)
        {
            string modeName = AutoTunerModeBox.SelectedIndex == 1
                ? LocalizationService.Pick("Grob -3", "Coarse -3")
                : LocalizationService.Pick("Fein -1", "Fine -1");
            StartButton.Content = $"{LocalizationService.Get("StartAutoTuner")} ({modeName})";
            StartButton.Classes.Set("primary", false);
            StartButton.Classes.Set("auto", true);
        }
        else
        {
            StartButton.Content = LocalizationService.Get("StartTest");
            StartButton.Classes.Set("auto", false);
            StartButton.Classes.Set("primary", true);
        }
    }

    private void ApplySavedProfileOnBoot()
    {
        string workRoot = Path.Combine(AppContext.BaseDirectory, "runs");
        var profile = TaskSchedulerService.LoadSavedProfile(workRoot);
        if (profile == null || !_smu.IsAvailable) return;

        int count = 0;
        foreach (var row in _rows)
        {
            if (profile.CoreMargins.TryGetValue(row.Index, out int val))
            {
                if (_smu.WriteCurveOptimizer(row.Index, val))
                {
                    row.Margin = val;
                    row.Applied = val;
                    count++;
                }
            }
        }
        Log(LocalizationService.Pick(
            $"🚀 Autostart: {count} Curve-Optimizer-Werte aus gespeichertem Profil übernommen.",
            $"🚀 Autostart: applied {count} Curve Optimizer values from the saved profile."), LogLevel.Success);
    }

    // ══════════════════════════════════════════════════════════════
    // Keyboard
    // ══════════════════════════════════════════════════════════════

    private void OnMainWindowKeyDown(object? sender, KeyEventArgs e)
    {
        // A control that already acted on the key owns it. Without this, Space on a focused
        // ComboBox both opened the drop-down and kicked off a multi-hour test run.
        if (e.Handled) return;

        var focused = TopLevel.GetTopLevel(this)?.FocusManager?.GetFocusedElement();
        bool inTextInput = focused is TextBox or ComboBox or NumericUpDown or SelectableTextBlock;

        if (e.Key == Key.F5)
        {
            OnStartStop(sender, new RoutedEventArgs());
            e.Handled = true;
            return;
        }

        if (inTextInput) return;

        if (e.KeyModifiers.HasFlag(KeyModifiers.Control) && e.Key == Key.C)
        {
            OnCopyBiosValues(sender, new RoutedEventArgs());
            e.Handled = true;
        }
        else if (e.KeyModifiers.HasFlag(KeyModifiers.Control) && e.Key == Key.S)
        {
            OnApplyCurveOptimizer(sender, new RoutedEventArgs());
            e.Handled = true;
        }
        else if (e.KeyModifiers.HasFlag(KeyModifiers.Control) && e.Key == Key.L)
        {
            OnToggleLanguage(sender, new RoutedEventArgs());
            e.Handled = true;
        }
    }

    // ══════════════════════════════════════════════════════════════
    // Core rows
    // ══════════════════════════════════════════════════════════════

    private void BuildCoreRows()
    {
        var qualities = CoreQualityService.GetCoreQualities(_cores.Count, _smu.CorePerformanceRanking);
        int maxNegative = AutoTunerService.GetMaxNegativeMargin(_smu.CpuName);

        for (int i = 0; i < _cores.Count; i++)
        {
            var row = new CoreRow
            {
                Index = i,
                TotalCores = _cores.Count,
                CcdCount = _ccdLayout.CcdCount,
                CcdIndex = _ccdLayout.CcdFor(i),
                MinLimit = maxNegative,
                MaxLimit = 30m,
                QualityRank = qualities.TryGetValue(i, out var q) ? q.Rank : CoreQualityRank.Standard,
                CppcPerformance = qualities.TryGetValue(i, out var qp) ? qp.CppcPerformance : 0,
            };
            if (_smu.ReadCurveOptimizer(i) is { } margin)
            {
                row.Margin = margin;
                row.BiosBaseline = margin;
                row.Applied = margin;
            }
            row.PropertyChanged += (_, e) =>
            {
                if (e.PropertyName == nameof(CoreRow.Selected))
                {
                    UpdateSelectAllState();
                    UpdateDurationHint();
                }
                if (e.PropertyName == nameof(CoreRow.Margin))
                {
                    if (row.IsLocked)
                    {
                        row.IsLocked = false;
                        if (row.State == CoreState.Locked) row.State = CoreState.Ready;
                    }
                    UpdateSmartRecommendations();
                    UpdateDurationHint();
                }
            };
            _rows.Add(row);
        }
    }

    /// <summary>One selector per chiplet, generated - not two hard-coded buttons.</summary>
    private void BuildCcdButtons()
    {
        if (!_ccdLayout.IsMultiCcd) return;

        var buttons = new List<Button>();
        for (int c = 0; c < _ccdLayout.CcdCount; c++)
        {
            int ccd = c;
            var button = new Button
            {
                Content = $"CCD{ccd}",
                FontSize = 10,
                Padding = new Avalonia.Thickness(7, 3),
                MinHeight = 22,
            };
            button.Classes.Add("ghost");
            ToolTip.SetTip(button, CcdTooltipFor(ccd));
            button.Click += (_, _) => SelectCcd(ccd);
            buttons.Add(button);
        }
        CcdButtons.ItemsSource = buttons;
    }

    private string CcdTooltipFor(int ccd)
    {
        var cores = _ccdLayout.CoresOn(ccd).ToList();
        string list = cores.Count == 0 ? "" : $" ({string.Join(", ", cores)})";
        return LocalizationService.Get("SelectCcdTooltip") + list;
    }

    private void SelectCcd(int ccd)
    {
        foreach (var row in _rows) row.Selected = row.CcdIndex == ccd;
        UpdateSelectAllState();
        UpdateDurationHint();
    }

    /// <summary>Tri-state so "some cores selected" does not masquerade as "none".</summary>
    private void UpdateSelectAllState()
    {
        if (_suppressSelectAll) return;
        int selected = _rows.Count(r => r.Selected);
        _suppressSelectAll = true;
        SelectAllBox.IsChecked = selected == 0 ? false
            : selected == _rows.Count ? true
            : null;
        _suppressSelectAll = false;
    }

    private void CheckInterruptedRun()
    {
        string workRoot = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "engines", "runs"));
        _interruptedState = TestStateService.DetectInterruptedRun(workRoot);

        if (_interruptedState is null) return;

        int core = _interruptedState.CurrentCore;
        int currentMargin = _interruptedState.CoreMargins.GetValueOrDefault(core, 0);

        RecoveryTitle.Text = LocalizationService.Pick(
            $"Unerwarteter System-Neustart bei Kern {core}",
            $"Unexpected system restart on core {core}");
        RecoveryText.Text = LocalizationService.Pick(
            $"Der letzte Lauf wurde in Durchgang {_interruptedState.CurrentIteration} auf Kern {core} abgebrochen (Curve Optimizer: {currentMargin}). " +
            $"Das ist das typische Bild eines instabilen Kerns.\nEmpfehlung: Den Wert für Kern {core} um 4 Punkte entschärfen ({currentMargin} → {currentMargin + 4}) und den Lauf mit den restlichen Kernen fortsetzen.",
            $"The last run was cut short during pass {_interruptedState.CurrentIteration} on core {core} (Curve Optimizer: {currentMargin}). " +
            $"That is the classic signature of an unstable core.\nRecommendation: back core {core} off by 4 points ({currentMargin} → {currentMargin + 4}) and resume with the remaining cores.");

        RecoveryPanel.IsVisible = true;
        Log(LocalizationService.Pick(
            $"Letzter Testlauf bei Kern {core} durch System-Absturz unterbrochen.",
            $"Last run on core {core} was interrupted by a system crash."), LogLevel.Error);
    }

    private void OnOpenSetupWizard(object? sender, RoutedEventArgs e)
    {
        var wizard = new SetupWizardWindow(_smu);
        wizard.Closed += async (_, _) => await RefreshDependenciesAsync();
        wizard.ShowDialog(this);
    }

    private void OnResumeInterruptedRun(object? sender, RoutedEventArgs e)
    {
        if (_interruptedState is null) return;

        int crashedCore = _interruptedState.CurrentCore;
        int currentMargin = (int)Row(crashedCore).Margin;
        Row(crashedCore).Margin = Math.Min(0, currentMargin + 4);

        var completed = _interruptedState.CompletedCores.ToHashSet();
        foreach (var row in _rows)
            row.Selected = !completed.Contains(row.Index);

        RecoveryPanel.IsVisible = false;
        Log(LocalizationService.Pick(
            $"Empfehlung für Kern {crashedCore} übernommen. Lauf wird für die verbleibenden Kerne fortgesetzt.",
            $"Applied the recommendation for core {crashedCore}. Resuming with the remaining cores."), LogLevel.Success);
        UpdateSelectAllState();
        UpdateDurationHint();

        OnStartStop(sender, e);
    }

    private void OnDismissRecovery(object? sender, RoutedEventArgs e)
    {
        string workRoot = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "engines", "runs"));
        TestStateService.ClearState(workRoot);
        RecoveryPanel.IsVisible = false;
        _interruptedState = null;
    }

    private CpuTuningRecommendation? _currentRecommendation;
    private bool _recommendationFineMode;

    private void OnToggleRecommendationPrecision(object? sender, RoutedEventArgs e)
    {
        _recommendationFineMode = !_recommendationFineMode;
        RecommendationModeButton.Content = LocalizationService.Get(_recommendationFineMode ? "FineMode" : "CoarseMode");
        Log(LocalizationService.Pick(
            $"WHEA-Korrektur umgestellt auf: {(_recommendationFineMode ? "Fein (+2)" : "Grob (+4)")}",
            $"WHEA mitigation switched to: {(_recommendationFineMode ? "Fine (+2)" : "Coarse (+4)")}"));
        UpdateSmartRecommendations();
    }

    // ══════════════════════════════════════════════════════════════
    // Localization
    // ══════════════════════════════════════════════════════════════

    private void OnToggleLanguage(object? sender, RoutedEventArgs e)
    {
        LocalizationService.CurrentLanguage = LocalizationService.CurrentLanguage == Language.English
            ? Language.German
            : Language.English;

        ApplyLocalization();
        SaveSettings();
        Log(LocalizationService.Pick("🌐 Sprache auf Deutsch umgestellt.", "🌐 Language switched to English."));
    }

    private static void Tip(Control c, string key) => ToolTip.SetTip(c, LocalizationService.Get(key));

    private void ApplyLocalization()
    {
        // Header
        LanguageButton.Content = LocalizationService.IsGerman ? "🌐 DE" : "🌐 EN";
        Tip(LanguageButton, "LanguageTooltip");
        Tip(BiosAuditButton, "SystemScoreTooltip");
        SetupWizardButton.Content = LocalizationService.Get("DriverManager");
        Tip(SetupWizardButton, "DriverManagerTooltip");
        Tip(WatchdogBadgeBorder, "WatchdogTooltip");

        BoostLabel.Text = LocalizationService.Get("MetricBoost");
        TempLabel.Text = LocalizationService.Get("MetricTemp");
        Tip(BoostChip, "TooltipBoost");
        Tip(PptChip, "TooltipPpt");
        Tip(TdcChip, "TooltipTdc");
        Tip(EdcChip, "TooltipEdc");
        Tip(TempChip, "TooltipTempNow");
        Tip(ScalarChip, "TooltipScalar");

        ResumeRunButton.Content = LocalizationService.Get("RecoveryResume");
        DismissRecoveryButton.Content = LocalizationService.Get("RecoveryDismiss");

        // Recommendation card
        RecommendationTitleText.Text = LocalizationService.Get("SmartRecommendationTitle");
        RecommendationModeButton.Content = LocalizationService.Get(_recommendationFineMode ? "FineMode" : "CoarseMode");
        Tip(RecommendationModeButton, "CoarseFineTooltip");
        ApplyRecommendationButton.Content = LocalizationService.Get("ApplyValues");
        Tip(ApplyRecommendationButton, "ApplyValuesTooltip");
        ResetWheaButton.Content = LocalizationService.Get("ResetWhea");
        Tip(ResetWheaButton, "ResetWheaTooltip");
        UseRecommendedProfileButton.Content = LocalizationService.Get("UseProfile");

        // Core table
        SelectAllBox.Content = LocalizationService.Get("AllCores");
        Tip(SelectAllBox, "AllCoresTooltip");
        CoHeaderText.Text = LocalizationService.Get("CurveOpt");
        Tip(CoHeaderText, "ColumnCoTooltip");
        SetAllMaxButton.Content = $"{LocalizationService.Get("SetAllMax")} {AutoTunerService.GetMaxNegativeMargin(_smu.CpuName)}";
        Tip(SetAllMaxButton, "SetAllMaxTooltip");
        SetAllOriginalButton.Content = LocalizationService.Get("SetAllReset");
        Tip(SetAllOriginalButton, "SetAllResetTooltip");
        if (CcdButtons.ItemsSource is IEnumerable<Button> ccdButtons)
        {
            int ccd = 0;
            foreach (var b in ccdButtons) ToolTip.SetTip(b, CcdTooltipFor(ccd++));
        }

        // Right panel
        TestControlsTitle.Text = LocalizationService.Get("TestControls");
        RightTabSetupBtn.Content = LocalizationService.Get("TabSetup");
        RightTabEngineBtn.Content = LocalizationService.Get("TabEngine");
        RightTabSystemBtn.Content = LocalizationService.Get("TabSystem");
        TestRunningText.Text = LocalizationService.Get("TestRunning");

        TestProfileLabel.Text = LocalizationService.Get("TestProfile");
        AutoTunerTitleText.Text = LocalizationService.Get("AutoTunerTitle");
        Tip(AutoTunerModeBox, "AutoTunerTooltip");
        AutoTunerOpt0.Content = LocalizationService.Get("AutoTunerDisabled");
        AutoTunerOpt1.Content = LocalizationService.Get("AutoTunerCoarse");
        AutoTunerOpt2.Content = LocalizationService.Get("AutoTunerFine");

        // Apply section
        ApplySectionTitle.Text = LocalizationService.Get("ApplySectionTitle");
        ApplyButton.Content = LocalizationService.Get("ApplyLive");
        Tip(ApplyButton, "ApplyLiveTooltip");
        ApplyLiveHint.Text = LocalizationService.Get("ApplyLiveHint");
        CopyBiosButton.Content = LocalizationService.Get("CopyBios");
        Tip(CopyBiosButton, "ApplyBiosTooltip");
        ApplyBiosHint.Text = LocalizationService.Get("ApplyBiosHint");
        AutostartBox.Content = LocalizationService.Get("AutostartBoot");
        ApplyAutostartHint.Text = LocalizationService.Get("ApplyAutostartHint");
        Tip(AutostartBox, "AutostartTooltip");

        // Engine tab
        MinutesLabel.Text = LocalizationService.Get("MinPerCore");
        Tip(MinutesBox, "MinPerCoreTooltip");
        IterationsLabel.Text = LocalizationService.Get("Passes");
        Tip(IterationsBox, "PassesTooltip");
        EngineLabel.Text = LocalizationService.Get("StressEngine");
        EngineOptAuto.Content = LocalizationService.Get("EngineFollowProfile");
        Tip(EngineBox, "StressEngineTooltip");
        ThreadsLabel.Text = LocalizationService.Get("ThreadsPerCore");
        Tip(ThreadsBox, "ThreadsTooltip");
        ModeLabel.Text = LocalizationService.Get("InstructionSet");
        Tip(ModeBox, "InstructionSetTooltip");
        FftLabel.Text = LocalizationService.Get("FftRange");
        Tip(FftBox, "FftTooltip");
        FftOptCustom.Content = LocalizationService.Get("FftCustom");
        FftMinLabel.Text = LocalizationService.Get("FftMin");
        FftMaxLabel.Text = LocalizationService.Get("FftMax");
        Tip(CustomFftPanel, "FftCustomTooltip");
        OrderLabel.Text = LocalizationService.Get("CoreOrder");
        Tip(OrderBox, "CoreOrderTooltip");
        StopOnErrorBox.Content = LocalizationService.Get("StopOnError");
        Tip(StopOnErrorBox, "StopOnErrorTooltip");
        SkipCoreOnErrorBox.Content = LocalizationService.Get("SkipCoreOnError");
        Tip(SkipCoreOnErrorBox, "SkipCoreOnErrorTooltip");
        CoreDelayLabel.Text = LocalizationService.Get("CoreDelay");
        Tip(CoreDelayBox, "CoreDelayTooltip");
        TreatWheaAsErrorBox.Content = LocalizationService.Get("TreatWheaAsError");
        Tip(TreatWheaAsErrorBox, "TreatWheaAsErrorTooltip");
        PauseIntervalLabel.Text = LocalizationService.Get("PauseInterval");
        Tip(PauseIntervalBox, "TransientPauseTooltip");
        PauseDurationLabel.Text = LocalizationService.Get("PauseDuration");
        Tip(PauseDurationBox, "TransientPauseTooltip");
        YcAlgoLabel.Text = LocalizationService.Get("YcAlgo");
        Tip(YcAlgoBox, "YcAlgoTooltip");
        OrderOpt0.Content = LocalizationService.Pick("Nacheinander", "Sequential");
        OrderOpt1.Content = LocalizationService.Pick("Abwechselnd", "Alternate");
        OrderOpt2.Content = LocalizationService.Pick("Zufällig", "Random");
        OrderOpt3.Content = LocalizationService.Get("OrderCustom");
        CustomOrderLabel.Text = LocalizationService.Get("CustomOrderLabel");
        Tip(CustomOrderBox, "CustomOrderTooltip");
        UpdateCustomOrderHint();
        ThreadOpt0.Content = LocalizationService.Pick("1 Thread", "1 thread");
        ThreadOpt1.Content = LocalizationService.Pick("2 Threads (SMT)", "2 threads (SMT)");

        // System tab
        MaxTempLabel.Text = LocalizationService.Get("SafetyTempLimit");
        Tip(MaxTempBox, "SafetyTempTooltip");
        PostTestActionLabel.Text = LocalizationService.Get("PostTestAction");
        PostOpt0.Content = LocalizationService.Get("ActionDoNothing");
        PostOpt1.Content = LocalizationService.Get("ActionSleep");
        PostOpt2.Content = LocalizationService.Get("ActionShutdown");
        WebhookLabel.Text = LocalizationService.Get("WebhookLabel");
        Tip(WebhookBox, "WebhookTooltip");

        // Bottom workspace
        TabGraphButton.Content = LocalizationService.Get("TabGraph");
        TabLogButton.Content = LocalizationService.Get("TabLog");
        CopyLogButton.Content = LocalizationService.Get("CopyLog");
        ClearLogButton.Content = LocalizationService.Get("ClearLog");
        OpenLogFolderButton.Content = LocalizationService.Get("OpenLogFolder");
        Tip(OpenLogFolderButton, "OpenLogFolderTooltip");
        ShortcutsText.Text = LocalizationService.Get("Shortcuts");

        // Result panel
        AcceptButton.Content = LocalizationService.Get("ResultAccept");
        ExportReportButton.Content = LocalizationService.Get("ResultExport");
        Tip(ExportReportButton, "ResultExportTooltip");
        DismissResultButton.Content = LocalizationService.Get("Close");

        LiveTelemetryGraph.ApplyLocalization();

        foreach (var row in _rows) row.RefreshLocalization();

        LoadProfiles();
        UpdateSmartRecommendations();
        UpdateStartButtonState();
        UpdateDurationHint();
        StartWatchdog();
        RefreshAuditPill();
        _ = RefreshDependenciesAsync();
    }

    // ══════════════════════════════════════════════════════════════
    // Tabs
    // ══════════════════════════════════════════════════════════════

    private void OnRightTabChanged(object? sender, RoutedEventArgs e)
    {
        if (RightTabSetupPanel is null || RightTabEnginePanel is null || RightTabSystemPanel is null) return;
        RightTabSetupPanel.IsVisible = RightTabSetupBtn.IsChecked == true;
        RightTabEnginePanel.IsVisible = RightTabEngineBtn.IsChecked == true;
        RightTabSystemPanel.IsVisible = RightTabSystemBtn.IsChecked == true;
    }

    private void OnBottomTabChanged(object? sender, RoutedEventArgs e)
    {
        if (LiveTelemetryGraph is null || LogContainer is null) return;
        bool log = TabLogButton.IsChecked == true;
        LiveTelemetryGraph.IsVisible = !log;
        LogContainer.IsVisible = log;
        CopyLogButton.IsVisible = log;
        OpenLogFolderButton.IsVisible = log && _logDirectory is not null;
        ClearLogButton.IsVisible = log;
    }

    private void OnAutoTunerModeChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (StartButton is null || AutoTunerModeBox is null) return;
        UpdateStartButtonState();
        UpdateDurationHint();
    }

    // ══════════════════════════════════════════════════════════════
    // Recommendations
    // ══════════════════════════════════════════════════════════════

    private void UpdateSmartRecommendations(IReadOnlyList<WheaEvent>? whea = null)
    {
        string workRoot = Path.Combine(AppContext.BaseDirectory, "runs");
        DateTime? resetTime = WheaResetService.GetResetTimestamp(workRoot);
        whea ??= WheaWatcher.ReadExisting();
        if (resetTime.HasValue)
        {
            whea = whea.Where(e => e.Time > resetTime.Value).ToList();
        }

        var currentMargins = _rows.ToDictionary(r => r.Index, r => (int)r.Margin);
        _currentRecommendation = CpuRecommendationService.GetRecommendation(
            _smu.CpuName, _cores, whea, currentMargins, _recommendationFineMode, LocalizationService.IsGerman);
        RecommendationText.Text = _currentRecommendation.SummaryAdvice;

        // The service already works this out; it used to be computed and thrown away.
        string wanted = _currentRecommendation.RecommendedProfileName;
        var match = FindProfileIndex(wanted);
        if (match >= 0 && ProfileBox.SelectedIndex != match)
        {
            RecommendedProfileRow.IsVisible = true;
            RecommendedProfileText.Text = $"{LocalizationService.Get("RecommendedProfile")}: {ProfileNameAt(match)}";
            _recommendedProfileIndex = match;
        }
        else
        {
            RecommendedProfileRow.IsVisible = false;
            _recommendedProfileIndex = -1;
        }

        if (_currentRecommendation.WheaCorrections.Count > 0)
        {
            WheaRecommendationText.IsVisible = true;
            WheaRecommendationText.Text = string.Join("\n", _currentRecommendation.WheaCorrections);
        }
        else
        {
            WheaRecommendationText.IsVisible = false;
        }
    }

    private int _recommendedProfileIndex = -1;

    /// <summary>
    /// Matches the recommended profile name against the actual list by shared words. The two
    /// are maintained separately and their wording drifts, so an exact or substring compare
    /// silently found nothing and the recommendation row just never appeared.
    /// </summary>
    private int FindProfileIndex(string wanted)
    {
        if (ProfileBox.ItemsSource is not IEnumerable<TestProfile> profiles || string.IsNullOrWhiteSpace(wanted))
            return -1;

        static HashSet<string> Tokens(string s) =>
            [.. s.Split([' ', '-', '–', '(', ')', '&', ',', '.', '/'], StringSplitOptions.RemoveEmptyEntries)
                 .Select(t => new string([.. t.Where(char.IsLetterOrDigit)]).ToLowerInvariant())
                 .Where(t => t.Length > 2)];

        var target = Tokens(wanted);
        if (target.Count == 0) return -1;

        int bestIndex = -1, bestScore = 0;
        var list = profiles.ToList();
        for (int i = 0; i < list.Count; i++)
        {
            int score = Tokens(list[i].Name).Count(target.Contains);
            if (score > bestScore) { bestScore = score; bestIndex = i; }
        }

        // Two shared words is enough to be meaningful and enough to reject a chance hit.
        return bestScore >= 2 ? bestIndex : -1;
    }

    private string ProfileNameAt(int index) =>
        ProfileBox.ItemsSource is IEnumerable<TestProfile> p ? p.ElementAt(index).Name : "";

    private void OnUseRecommendedProfile(object? sender, RoutedEventArgs e)
    {
        if (_recommendedProfileIndex >= 0) ProfileBox.SelectedIndex = _recommendedProfileIndex;
    }

    private void OnApplySmartRecommendation(object? sender, RoutedEventArgs e)
    {
        if (_currentRecommendation is null) return;

        int count = 0;
        foreach (var row in _rows)
        {
            if (_currentRecommendation.SuggestedCoreMargins.TryGetValue(row.Index, out int suggested))
            {
                row.Margin = suggested;
                count++;
            }
        }
        Log(LocalizationService.Pick(
            $"💡 Empfehlung für {_smu.CpuName} auf {count} Kerne angewendet.",
            $"💡 Applied the recommendation for {_smu.CpuName} to {count} cores."), LogLevel.Success);
    }

    private async void OnCopyBiosValues(object? sender, RoutedEventArgs e)
    {
        var lines = _rows.Select(r =>
            $"{r.CoreName}: {r.Margin}  →  BIOS: {(r.Margin < 0 ? "Negative" : r.Margin > 0 ? "Positive" : "Auto")} / {Math.Abs(r.Margin)}");
        string text = LocalizationService.Pick(
            "=== AMD Ryzen Curve Optimizer – BIOS-Werte ===\n",
            "=== AMD Ryzen Curve Optimizer – BIOS values ===\n") +
            $"CPU: {_smu.CpuName}\n" + string.Join("\n", lines);

        if (Clipboard is null) return;

        await Clipboard.SetTextAsync(text);
        Log(LocalizationService.Pick(
            "📋 CO-Werte für das BIOS in die Zwischenablage kopiert.",
            "📋 Copied CO values for the BIOS to the clipboard."), LogLevel.Success);

        ShowDetailPanel(
            LocalizationService.Get("BiosCopiedTitle"),
            text + "\n\n" + LocalizationService.Get("BiosCopiedHint"),
            showAccept: false);
    }

    /// <summary>
    /// Single entry point for the shared detail panel. Every caller states whether the
    /// Accept button belongs, so it can no longer be switched off and left off.
    /// </summary>
    private void ShowDetailPanel(string title, string body, bool showAccept)
    {
        ResultTitle.Text = title;
        ResultText.Text = body;
        AcceptButton.IsVisible = showAccept;
        ExportReportButton.IsVisible = showAccept;
        ResultActions.IsVisible = true;
        ResultPanel.IsVisible = true;
    }

    private void PlayNotificationSound(bool isError)
    {
        try
        {
            if (OperatingSystem.IsWindows())
            {
                if (isError) Console.Beep(500, 350);
                else Console.Beep(1000, 250);
            }
        }
        catch { }
    }

    private void PerformPostTestAction()
    {
        if (PostTestActionBox is null) return;

        int index = PostTestActionBox.SelectedIndex;
        if (index == 1)
        {
            Log(LocalizationService.Pick(
                "🌙 Test beendet: PC wechselt in den Energiesparmodus…",
                "🌙 Test finished: putting the PC into standby…"));
            try { Process.Start("rundll32.exe", "powrprof.dll,SetSuspendState 0,1,0"); } catch { }
        }
        else if (index == 2)
        {
            Log(LocalizationService.Pick(
                "🔌 Test beendet: PC wird in 30 Sekunden heruntergefahren.",
                "🔌 Test finished: shutting the PC down in 30 seconds."));
            try { Process.Start("shutdown.exe", "/s /t 30 /c \"PboStudio test finished.\""); } catch { }
        }
    }

    // ══════════════════════════════════════════════════════════════
    // System audit
    // ══════════════════════════════════════════════════════════════

    private SystemHealthAudit? _audit;

    private async Task RunSystemAuditAsync()
    {
        AuditScoreText.Text = LocalizationService.Get("SystemChecking");
        _audit = await Task.Run(BiosAuditService.RunFullAudit);
        RefreshAuditPill();

        Log(LocalizationService.Pick(
            $"🔍 Plattform-Check: {_audit.OverallScore}/100",
            $"🔍 Platform check: {_audit.OverallScore}/100"));
        foreach (var item in _audit.ActionItems)
            Log("   " + item, item.StartsWith('⚠') ? LogLevel.Warn : LogLevel.Info);
    }

    /// <summary>The pill leads with meaning; the score is the supporting detail, not the headline.</summary>
    private void RefreshAuditPill()
    {
        if (_audit is null) return;

        int problems = _audit.ActionItems.Count(i => i.StartsWith('⚠'));
        AuditScoreText.Text = problems == 0
            ? $"{LocalizationService.Get("SystemOk")} · {_audit.OverallScore}/100"
            : $"{problems} {LocalizationService.Get(problems == 1 ? "SystemHintOne" : "SystemHintMany")} · {_audit.OverallScore}/100";

        AuditScoreText.Foreground = SolidColorBrush.Parse(_audit.OverallScore switch
        {
            >= 90 => "#34D399",
            >= 70 => "#FBBF24",
            _ => "#F87171",
        });
    }

    private void OnShowSystemAuditDetails(object? sender, RoutedEventArgs e)
    {
        if (_audit is null) return;

        // Toggle off if this panel is already showing the audit.
        if (ResultPanel.IsVisible && _detailIsAudit)
        {
            ResultPanel.IsVisible = false;
            _detailIsAudit = false;
            return;
        }

        string body = string.Join("\n• ",
            _audit.ActionItems.Prepend(LocalizationService.Get("AuditCheckedPoints")));
        ShowDetailPanel($"🔍 {LocalizationService.Get("SystemAuditTitle")} — {_audit.OverallScore}/100", body, showAccept: false);
        _detailIsAudit = true;
    }

    private bool _detailIsAudit;

    private void ShowSystemInfo()
    {
        if (!_smu.IsAvailable)
        {
            CpuText.Text = LocalizationService.Pick($"System-CPU ({_cores.Count} Kerne)", $"System CPU ({_cores.Count} cores)");
            SupportText.IsVisible = false;
            ApplyButton.IsEnabled = false;
            Log(LocalizationService.Pick(
                "SMU nicht verfügbar — Stresstests funktionieren, das Setzen von CO-Werten nicht.",
                "SMU unavailable — stress tests work, writing CO values does not."), LogLevel.Warn);
            UpdateSmartRecommendations();
            return;
        }

        CpuText.Text = LocalizationService.Pick($"{_smu.CpuName} ({_cores.Count} Kerne)", $"{_smu.CpuName} ({_cores.Count} cores)");
        UpdatePbo();
        UpdateSmartRecommendations();

        if (_smu.ReadsAllZero())
        {
            SupportText.IsVisible = true;
            SupportText.Text = LocalizationService.Pick(
                "Hinweis: Alle Kerne melden 0. Falls im BIOS CO-Werte gesetzt sind, wurden sie nicht an die SMU übergeben (BIOS-Fix: Curve Optimizer im BIOS aus- und wieder einschalten).",
                "Note: every core reports 0. If CO values are set in the BIOS they were not handed to the SMU (BIOS fix: turn Curve Optimizer off and on again).");
        }

        Log($"{_smu.CpuName}, {_cores.Count} " + LocalizationService.Pick("Kerne erkannt.", "cores detected."), LogLevel.Success);
        LogCcdLayout();
        LogPreferredCores();
    }

    /// <summary>Names the preferred cores and the values they were derived from.</summary>
    private void LogPreferredCores()
    {
        var gold = _rows.FirstOrDefault(r => r.QualityRank == CoreQualityRank.GoldStar);
        var silver = _rows.FirstOrDefault(r => r.QualityRank == CoreQualityRank.SilverStar);

        if (gold is null)
        {
            Log(LocalizationService.Pick(
                "Bevorzugte Kerne: Die CPU meldet keine CPPC-Rangwerte — es werden keine Abzeichen vergeben.",
                "Preferred cores: the CPU reports no CPPC ranking, so no badges are shown."));
            return;
        }

        string silverPart = silver is null
            ? ""
            : LocalizationService.Pick(
                $", 🥈 {silver.CoreName} ({silver.CppcPerformance})",
                $", 🥈 {silver.CoreName} ({silver.CppcPerformance})");

        Log(LocalizationService.Pick(
            $"Bevorzugte Kerne laut CPPC: 🥇 {gold.CoreName} ({gold.CppcPerformance}){silverPart}",
            $"Preferred cores per CPPC: 🥇 {gold.CoreName} ({gold.CppcPerformance}){silverPart}"));
    }

    /// <summary>
    /// States where the grouping came from. If a chiplet split ever looks wrong on an exotic
    /// part, this line is the difference between a guess and a diagnosis.
    /// </summary>
    private void LogCcdLayout()
    {
        if (!_ccdLayout.IsMultiCcd)
        {
            Log(LocalizationService.Pick("Topologie: ein Chiplet (CCD).", "Topology: a single chiplet (CCD)."));
            return;
        }

        string source = _smu.ReportedCcdCount is > 0
            ? LocalizationService.Pick("CCD-Fuse der CPU", "the CPU's CCD fuse")
            : _cores.Select(c => c.Die).Distinct().Count() > 1
                ? LocalizationService.Pick("Die-Angabe von Windows", "the die layout Windows reports")
                : LocalizationService.Pick("L3-Cache-Gruppen", "last-level cache groups");

        string groups = string.Join(" · ", Enumerable.Range(0, _ccdLayout.CcdCount)
            .Select(c => $"CCD{c}: {string.Join(",", _ccdLayout.CoresOn(c))}"));

        Log(LocalizationService.Pick(
            $"Topologie: {_ccdLayout.CcdCount} Chiplets laut {source} — {groups}",
            $"Topology: {_ccdLayout.CcdCount} chiplets per {source} — {groups}"));
    }

    // ══════════════════════════════════════════════════════════════
    // Applying values
    // ══════════════════════════════════════════════════════════════

    private void OnApplyCurveOptimizer(object? sender, RoutedEventArgs e)
    {
        if (!_smu.IsAvailable) return;

        // Only touch cores the user actually edited. Writing every row would silently replace
        // the BIOS setting on cores nobody meant to change.
        var changed = _rows.Where(r => r.IsChanged).ToList();
        if (changed.Count == 0)
        {
            Log(LocalizationService.Pick("Nichts zu tun — kein Wert wurde geändert.", "Nothing to do — no value was changed."));
            return;
        }

        int applied = 0, failed = 0;
        var currentMargins = new Dictionary<int, int>();

        foreach (var row in _rows)
        {
            if (row.IsChanged)
            {
                if (_smu.WriteCurveOptimizer(row.Index, (int)row.Margin))
                {
                    Log($"{row.CoreName}: {row.Applied} → {row.Margin}", LogLevel.Success);
                    row.Applied = row.Margin;
                    applied++;
                }
                else
                {
                    failed++;
                    Log($"{row.CoreName}: " + LocalizationService.Pick("Schreiben abgelehnt.", "write rejected."), LogLevel.Error);
                }
            }
            currentMargins[row.Index] = (int)row.Margin;
        }

        foreach (var row in _rows) row.RefreshLocalization();

        SaveCurrentProfile();

        Log(LocalizationService.Pick(
            $"{applied} Kern(e) live gesetzt" + (failed > 0 ? $", {failed} abgelehnt" : "") + ". Profil unter runs/co_saved.json gespeichert.",
            $"{applied} core(s) applied live" + (failed > 0 ? $", {failed} rejected" : "") + ". Profile saved to runs/co_saved.json."),
            failed > 0 ? LogLevel.Warn : LogLevel.Success);
    }

    /// <summary>
    /// Snapshot for the Windows autostart task. Also written after an auto-tuner run, so a
    /// reboot reapplies the values the tuner settled on rather than an older manual save.
    /// </summary>
    private void SaveCurrentProfile()
    {
        string workRoot = Path.Combine(AppContext.BaseDirectory, "runs");
        var margins = _rows.ToDictionary(r => r.Index, r => (int)r.Margin);
        TaskSchedulerService.SaveProfile(workRoot, _smu.CpuName, margins);
    }

    private void OnToggleAutostart(object? sender, RoutedEventArgs e)
    {
        bool enable = AutostartBox.IsChecked == true;
        string exePath = Process.GetCurrentProcess().MainModule?.FileName
            ?? Path.Combine(AppContext.BaseDirectory, "PboStudio.exe");

        if (TaskSchedulerService.SetAutostart(enable, exePath))
        {
            Log(enable
                ? LocalizationService.Pick(
                    "✓ Windows-Autostart aktiviert (geplante Aufgabe „PboStudioWatchdog“ erstellt).",
                    "✓ Windows autostart enabled (scheduled task “PboStudioWatchdog” created).")
                : LocalizationService.Pick("✓ Windows-Autostart deaktiviert.", "✓ Windows autostart disabled."),
                LogLevel.Success);
        }
        else
        {
            Log(LocalizationService.Pick(
                "⚠️ Die geplante Windows-Aufgabe konnte nicht angepasst werden (Administratorrechte erforderlich).",
                "⚠️ Could not change the scheduled Windows task (administrator rights required)."), LogLevel.Error);
            AutostartBox.IsChecked = !enable;
        }
    }

    /// <summary>
    /// Parses "0, 4, 1, 5" into a run order. Out-of-range entries are reported rather than
    /// silently dropped; repeats are kept, because testing a suspect core twice in one pass is
    /// a legitimate thing to ask for.
    /// </summary>
    private (List<int> Order, List<string> Rejected) ParseCustomOrder()
    {
        var order = new List<int>();
        var rejected = new List<string>();

        char[] separators = [',', ';', ' ', '\t', '\n', '\r', '>'];
        foreach (string token in (CustomOrderBox?.Text ?? "").Split(separators, StringSplitOptions.RemoveEmptyEntries))
        {
            if (int.TryParse(token, out int index) && index >= 0 && index < _cores.Count)
                order.Add(index);
            else
                rejected.Add(token);
        }

        return (order, rejected);
    }

    private void OnOrderChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (CustomOrderPanel is null) return;
        CustomOrderPanel.IsVisible = OrderBox.SelectedIndex == 3;
        UpdateCustomOrderHint();
    }

    private void OnCustomOrderChanged(object? sender, TextChangedEventArgs e) => UpdateCustomOrderHint();

    /// <summary>Shows the order that will actually run, so a typo is visible before the start.</summary>
    private void UpdateCustomOrderHint()
    {
        if (CustomOrderHint is null || CustomOrderBox is null) return;

        var (order, rejected) = ParseCustomOrder();

        if (rejected.Count > 0)
        {
            CustomOrderHint.Text = string.Format(
                LocalizationService.Get("CustomOrderInvalid"), string.Join(", ", rejected));
            CustomOrderHint.Foreground = SolidColorBrush.Parse("#F87171");
        }
        else if (order.Count == 0)
        {
            CustomOrderHint.Text = LocalizationService.Get("CustomOrderEmpty");
            CustomOrderHint.Foreground = SolidColorBrush.Parse("#64748B");
        }
        else
        {
            CustomOrderHint.Text = string.Format(
                LocalizationService.Get("CustomOrderPreview"), string.Join(" \u2192 ", order));
            CustomOrderHint.Foreground = SolidColorBrush.Parse("#34D399");
        }
    }

    private void OnEngineChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (PrimeOptionsPanel is null || YcAlgoPanel is null) return;

        var over = EngineOverride();
        PrimeOptionsPanel.IsVisible = over != EngineKind.YCruncher;
        YcAlgoPanel.IsVisible = over == EngineKind.YCruncher
            || (over is null && ProfileBox?.SelectedItem is TestProfile p && p.Uses(EngineKind.YCruncher));

        UpdateDurationHint();
    }

    // ══════════════════════════════════════════════════════════════
    // Run
    // ══════════════════════════════════════════════════════════════

    private async void OnStartStop(object? sender, RoutedEventArgs e)
    {
        if (_running)
        {
            _runCts?.Cancel();
            Log(LocalizationService.Pick("Abbruch angefordert…", "Cancellation requested…"), LogLevel.Warn);
            return;
        }

        if (!StartButton.IsEnabled) return;

        string engineRoot = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "engines"));

        // Which engines the run needs comes from the plan, not from one dropdown.
        var plannedPhases = BuildPhases();
        bool useYCruncher = plannedPhases.Any(p => p.Engine == EngineKind.YCruncher);
        bool usePrime95 = plannedPhases.Any(p => p.Engine == EngineKind.Prime95);

        string prime95 = Path.Combine(engineRoot, "prime95", "prime95.exe");
        string? ycBinary = null;

        if (useYCruncher)
        {
            string ycRoot = Path.Combine(engineRoot, "ycruncher");
            string? install = Directory.Exists(ycRoot)
                ? Directory.EnumerateDirectories(ycRoot, "y-cruncher*").FirstOrDefault()
                : null;

            ycBinary = install is null ? null : YCruncherEngine.FindBinaryFor(install, DetectGeneration());
            if (ycBinary is null)
            {
                Log(LocalizationService.Pick("y-cruncher nicht gefunden. Setup wird geöffnet…", "y-cruncher not found. Opening setup…"), LogLevel.Error);
                OnOpenSetupWizard(sender, e);
                return;
            }
            Log($"y-cruncher: {Path.GetFileName(ycBinary)}");
        }
        if (usePrime95 && !File.Exists(prime95))
        {
            Log(LocalizationService.Pick($"Prime95 nicht gefunden: {prime95}. Setup wird geöffnet…", $"Prime95 not found: {prime95}. Opening setup…"), LogLevel.Error);
            OnOpenSetupWizard(sender, e);
            return;
        }

        var phases = plannedPhases;

        var skipped = _rows.Where(r => !r.Selected).Select(r => r.Index).ToHashSet();
        if (skipped.Count == _rows.Count)
        {
            Log(LocalizationService.Pick("Kein Kern ausgewählt.", "No core selected."), LogLevel.Warn);
            return;
        }

        var selectedCores = _rows.Where(r => r.Selected).Select(r => r.Index).ToList();
        var coreMargins = _rows.ToDictionary(r => r.Index, r => (int)r.Margin);

        // Refuse to start on an empty custom order rather than falling back to some other
        // order the user did not ask for.
        List<int> customOrder = [];
        if (OrderBox.SelectedIndex == 3)
        {
            customOrder = ParseCustomOrder().Order;
            if (customOrder.Count == 0)
            {
                Log(LocalizationService.Get("CustomOrderNeeded"), LogLevel.Warn);
                RightTabEngineBtn.IsChecked = true;
                CustomOrderBox.Focus();
                return;
            }
        }

        AutoTunerMode? autoTunerMode = AutoTunerModeBox.SelectedIndex switch
        {
            1 => AutoTunerMode.Grob,
            2 => AutoTunerMode.Fein,
            _ => null,
        };

        if (autoTunerMode.HasValue && !_smu.IsAvailable)
        {
            Log(LocalizationService.Get("AutoTunerNeedsSmu"), LogLevel.Error);
            OnOpenSetupWizard(sender, e);
            return;
        }

        // A static run against unapplied edits measures something other than what is on screen.
        int unapplied = _rows.Count(r => r.Selected && r.IsChanged);
        if (unapplied > 0 && !_smu.IsAvailable)
            Log(string.Format(LocalizationService.Get("MarginsNotApplied"), unapplied), LogLevel.Warn);

        // Created once for the whole run so the auto-tuner's memory survives phase changes.
        var tunerState = new Dictionary<int, AutoTunerCoreState>();
        var lockedCores = new HashSet<int>();

        double pauseInterval = (double)(PauseIntervalBox?.Value ?? 30);
        double pauseDuration = (double)(PauseDurationBox?.Value ?? 1);

        int maxNegativeLimit = AutoTunerService.GetMaxNegativeMargin(_smu.CpuName);

        var plan = new TestPlan
        {
            CoresToIgnore = skipped,
            RuntimePerCore = TimeSpan.FromMinutes((double)(MinutesBox.Value ?? 6)),
            Threads = ThreadsBox.SelectedIndex + 1,
            MaxIterations = (int)(IterationsBox.Value ?? 3),
            Order = OrderBox.SelectedIndex switch
            {
                0 => CoreOrder.Sequential,
                2 => CoreOrder.Random,
                3 => CoreOrder.Custom,
                _ => CoreOrder.Alternate,
            },
            CustomOrder = customOrder,
            StopOnError = StopOnErrorBox.IsChecked == true,
            SkipCoreOnError = SkipCoreOnErrorBox.IsChecked == true,
            TreatWheaWarningAsError = TreatWheaAsErrorBox.IsChecked == true,
            DelayBetweenCores = TimeSpan.FromSeconds((double)(CoreDelayBox.Value ?? 2)),
            AutoTuner = autoTunerMode,
            MaxNegativeMargin = maxNegativeLimit,
            InitialCoreMargins = coreMargins,
            ApplyMargin = _smu.IsAvailable ? ApplyMarginToHardware : null,
            TunerState = tunerState,
            LockedCores = lockedCores,
            IsGerman = LocalizationService.IsGerman,
            SuspendPeriodically = pauseInterval > 0 && pauseDuration > 0,
            SuspendEvery = TimeSpan.FromSeconds(pauseInterval),
            SuspendFor = TimeSpan.FromSeconds(pauseDuration),
        };

        // A run is the natural checkpoint: whatever was configured for it is worth keeping.
        SaveSettings();

        _runStarted = DateTime.Now;
        _plannedTotal = EstimateTotalRuntime(autoTunerMode, maxNegativeLimit, phases.Count);
        _phaseLabel = "";
        UpdateClock();
        _clock.Start();

        SetRunning(true);
        ResultPanel.IsVisible = false;
        foreach (var row in _rows)
        {
            row.State = row.Selected ? CoreState.Waiting : CoreState.Skipped;
        }
        Log(LocalizationService.Pick(
            $"Getestet werden: {string.Join(", ", selectedCores)}",
            $"Testing cores: {string.Join(", ", selectedCores)}"));

        string runsDir = Path.Combine(engineRoot, "runs");
        string webhookUrl = WebhookBox.Text?.Trim() ?? "";

        if (!string.IsNullOrEmpty(webhookUrl))
        {
            _ = NotificationService.SendRunStartedAsync(webhookUrl, _smu.CpuName, selectedCores.Count, plan.RuntimePerCore);
        }

        var ycOpts = YcAlgoBox.SelectedIndex switch
        {
            1 => YCruncherOptions.Default,
            2 => new YCruncherOptions(["VT3", "FFT", "N63"]),
            _ => YCruncherOptions.CurveOptimizer,
        };

        _runCts = new CancellationTokenSource();
        var combined = new Dictionary<int, List<Failure>>();
        var completedCores = new List<int>();

        try
        {
            for (int i = 0; i < phases.Count && !_runCts.IsCancellationRequested; i++)
            {
                var phase = phases[i];
                _phaseLabel = phases.Count > 1
                    ? LocalizationService.Pick($"Abschnitt {i + 1} von {phases.Count} · {phase.Label}", $"Phase {i + 1} of {phases.Count} · {phase.Label}")
                    : phase.Label;

                if (phases.Count > 1) Log($"=== {_phaseLabel} ===");

                EngineKind effectiveKind = phase.Engine;
                IStressEngine engine;

                if (effectiveKind == EngineKind.YCruncher)
                {
                    if (ycBinary is null)
                    {
                        string ycRoot = Path.Combine(engineRoot, "ycruncher");
                        string? install = Directory.Exists(ycRoot) ? Directory.EnumerateDirectories(ycRoot, "y-cruncher*").FirstOrDefault() : null;
                        ycBinary = install is null ? null : YCruncherEngine.FindBinaryFor(install, DetectGeneration());
                        if (ycBinary is null)
                        {
                            Log(LocalizationService.Pick("y-cruncher nicht gefunden. Setup wird geöffnet…", "y-cruncher not found. Opening setup…"), LogLevel.Error);
                            OnOpenSetupWizard(sender, e);
                            return;
                        }
                    }
                    engine = new YCruncherEngine(ycBinary, runsDir, phase.YcOptions ?? ycOpts);
                }
                else
                {
                    if (!File.Exists(prime95))
                    {
                        Log(LocalizationService.Pick($"Prime95 nicht gefunden: {prime95}.", $"Prime95 not found: {prime95}."), LogLevel.Error);
                        OnOpenSetupWizard(sender, e);
                        return;
                    }
                    var (fftMin, fftMax) = CustomFftRange();
                    engine = new Prime95Engine(prime95, runsDir,
                        new Prime95Options(phase.Mode, phase.Fft, fftMin, fftMax));
                }

                var runner = new TestRunner(engine, _cores, plan);
                runner.Progress += ev =>
                {
                    if (ev is TestEvent.CoreStarted cs)
                    {
                        var state = new ActiveRunState
                        {
                            IsActive = true,
                            StartTime = _runStarted,
                            CurrentIteration = cs.Iteration,
                            CurrentCore = cs.Core,
                            EngineName = engine.DisplayName,
                            SelectedCores = selectedCores,
                            CompletedCores = [.. completedCores],
                            CoreMargins = coreMargins,
                        };
                        TestStateService.SaveState(runsDir, state);
                    }
                    else if (ev is TestEvent.CorePassed cp)
                    {
                        completedCores.Add(cp.Core);
                    }
                    else if (ev is TestEvent.CoreAutoTuned cat)
                    {
                        coreMargins[cat.Core] = cat.Result.NextMargin;
                    }

                    if (ev is not TestEvent.RunFinished) OnProgress(ev, webhookUrl);
                };

                foreach (var (core, list) in await runner.RunAsync(_runCts.Token))
                {
                    if (!combined.TryGetValue(core, out var all)) combined[core] = all = [];
                    all.AddRange(list);
                }
            }

            TestStateService.ClearState(runsDir);
            if (autoTunerMode.HasValue && _smu.IsAvailable)
                Dispatcher.UIThread.Post(SaveCurrentProfile);
            Dispatcher.UIThread.Post(() => ShowResult(combined, webhookUrl));
        }
        catch (OperationCanceledException)
        {
            TestStateService.ClearState(runsDir);
            Log(LocalizationService.Pick("Test abgebrochen.", "Test cancelled."), LogLevel.Warn);
        }
        catch (Exception ex)
        {
            TestStateService.ClearState(runsDir);
            Log($"{LocalizationService.Pick("Fehler", "Error")}: {ex.Message}", LogLevel.Error);
        }
        finally
        {
            SetRunning(false);
        }
    }

    /// <summary>
    /// Called from the runner's thread just before a core is measured. SmuService serialises
    /// its own access, so the write is safe from here; the row update hops to the UI thread.
    /// </summary>
    private bool ApplyMarginToHardware(int core, int margin)
    {
        bool ok = _smu.WriteCurveOptimizer(core, margin);
        if (!ok) return false;

        Dispatcher.UIThread.Post(() =>
        {
            var row = Row(core);
            row.Margin = margin;
            // The hardware now matches the table, so the row is no longer a pending edit.
            // The BIOS baseline deliberately stays where it was, so "reset" still works.
            row.Applied = margin;
            row.RefreshLocalization();
        });
        return true;
    }

    /// <summary>
    /// The phases a run will actually consist of.
    /// <para>
    /// A profile's first phase is replaced by the Engine tab's own instruction set and FFT
    /// range, while later phases come from the profile verbatim. That is deliberate - the
    /// dropdowns have to do something - but it means a profile whose description promises
    /// "SSE with huge FFTs" can genuinely run small ones. <see cref="PhaseSummary"/> puts the
    /// result on screen so the plan is never implied, only stated.
    /// </para>
    /// </summary>
    /// <summary>
    /// Null means "whatever the profile says", which is the default. Anything else is a
    /// deliberate override by someone who knows they want it.
    /// </summary>
    private EngineKind? EngineOverride() => EngineBox?.SelectedIndex switch
    {
        1 => EngineKind.Prime95,
        2 => EngineKind.YCruncher,
        _ => null,
    };

    /// <summary>
    /// The phases a run will actually consist of.
    /// <para>
    /// The profile owns the plan. An explicit engine override rewrites every phase onto that
    /// engine; the instruction set and FFT range from the Engine tab then apply to the first
    /// Prime95 phase. Previously the engine box silently overrode profiles even when untouched,
    /// so "Recommended - Prime95 SSE &amp; y-cruncher" could run as neither.
    /// </para>
    /// </summary>
    private List<TestPhase> BuildPhases()
    {
        var over = EngineOverride();
        var phases = new List<TestPhase>();

        if (ProfileBox?.SelectedItem is TestProfile profile && profile.Phases.Count > 0)
        {
            phases.AddRange(profile.Phases);
        }
        else
        {
            phases.Add(new TestPhase(SelectedPrimeMode(), SelectedFft(),
                over ?? EngineKind.Prime95));
        }

        if (over is not { } forced) return phases;

        for (int i = 0; i < phases.Count; i++)
        {
            phases[i] = forced == EngineKind.YCruncher
                ? phases[i] with { Engine = EngineKind.YCruncher }
                : new TestPhase(SelectedPrimeMode(), SelectedFft(), EngineKind.Prime95);
        }

        // Forcing Prime95 collapses a chain onto one configuration; keep it to a single phase.
        if (forced == EngineKind.Prime95 && phases.Count > 1) phases.RemoveRange(1, phases.Count - 1);

        return phases;
    }

    /// <summary>One line naming every phase in order, e.g. "1. SSE / Huge  ·  2. y-cruncher".</summary>
    private string PhaseSummary()
    {
        var phases = BuildPhases();

        return string.Join("  ·  ", phases.Select((p, i) =>
        {
            string label = p.Engine == EngineKind.YCruncher
                ? "y-cruncher"
                : $"{p.Mode.ToString().ToUpperInvariant()} / {FftLabelFor(p)}";
            return phases.Count > 1 ? $"{i + 1}. {label}" : label;
        }));
    }

    /// <summary>Shows the real bounds for a custom range rather than the word "Custom".</summary>
    private string FftLabelFor(TestPhase phase)
    {
        if (phase.Fft != FftPreset.Custom) return phase.Fft.ToString();
        var (min, max) = CustomFftRange();
        return $"{min}-{max}K";
    }

    private Prime95Mode SelectedPrimeMode() => ModeBox.SelectedIndex switch
    {
        0 => Prime95Mode.Sse,
        1 => Prime95Mode.Avx,
        2 => Prime95Mode.Avx2,
        _ => Prime95Mode.Avx512,
    };

    private FftPreset SelectedFft() => FftBox.SelectedIndex switch
    {
        0 => FftPreset.Smallest,
        1 => FftPreset.Small,
        2 => FftPreset.Large,
        3 => FftPreset.Huge,
        4 => FftPreset.All,
        5 => FftPreset.Heavy,
        6 => FftPreset.HeavyShort,
        7 => FftPreset.Moderate,
        _ => FftPreset.Custom,
    };

    private static int FftIndexOf(FftPreset preset) => preset switch
    {
        FftPreset.Smallest => 0,
        FftPreset.Small => 1,
        FftPreset.Large => 2,
        FftPreset.Huge => 3,
        FftPreset.All => 4,
        FftPreset.Heavy => 5,
        FftPreset.HeavyShort => 6,
        FftPreset.Moderate => 7,
        _ => 8,
    };

    /// <summary>Only meaningful for <see cref="FftPreset.Custom"/>; ignored otherwise.</summary>
    private (int Min, int Max) CustomFftRange()
    {
        int min = (int)(FftMinBox.Value ?? 4);
        int max = (int)(FftMaxBox.Value ?? 32);
        return min <= max ? (min, max) : (max, min);
    }

    private void OnFftChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (CustomFftPanel is null) return;
        CustomFftPanel.IsVisible = FftBox.SelectedIndex == 8;
    }

    /// <summary>Keeps the two bounds from crossing instead of quietly swapping them later.</summary>
    private void OnCustomFftChanged(object? sender, NumericUpDownValueChangedEventArgs e)
    {
        if (FftMinBox is null || FftMaxBox is null) return;

        if (ReferenceEquals(sender, FftMinBox) && FftMinBox.Value > FftMaxBox.Value)
            FftMaxBox.Value = FftMinBox.Value;
        else if (ReferenceEquals(sender, FftMaxBox) && FftMaxBox.Value < FftMinBox.Value)
            FftMinBox.Value = FftMaxBox.Value;
    }

    /// <summary>Shared by the progress clock and the duration hint so the two cannot disagree.</summary>
    private TimeSpan EstimateTotalRuntime(AutoTunerMode? mode, int maxNegativeLimit, int phaseCount)
    {
        double minutesPerCore = (double)(MinutesBox.Value ?? 6);
        int maxPasses = (int)(IterationsBox.Value ?? 3);

        if (mode.HasValue)
        {
            int totalSlots = _rows
                .Where(r => r.Selected && !r.IsLocked)
                .Sum(r => AutoTunerService.WorstCaseSlots((int)r.Margin, maxNegativeLimit, mode.Value, maxPasses));

            return TimeSpan.FromMinutes(totalSlots * minutesPerCore * phaseCount);
        }

        int selected = _rows.Count(r => r.Selected);
        return TimeSpan.FromMinutes(selected * minutesPerCore * maxPasses * phaseCount);
    }

    private void OnProgress(TestEvent e, string webhookUrl = "") => Dispatcher.UIThread.Post(() =>
    {
        switch (e)
        {
            case TestEvent.RunStarted s:
                Log(LocalizationService.Pick(
                    $"Start: {s.CoreCount} Kerne, je {s.PerCore.TotalMinutes:F0} Minuten.",
                    $"Start: {s.CoreCount} cores, {s.PerCore.TotalMinutes:F0} minutes each."));
                break;

            case TestEvent.IterationStarted s:
                Log(LocalizationService.Pick($"--- Durchgang {s.Iteration} ---", $"--- Pass {s.Iteration} ---"));
                break;

            case TestEvent.CoreStarted s:
                Row(s.Core).State = CoreState.Running;
                Log($"{Row(s.Core).CoreName}: " + LocalizationService.Pick("Test läuft.", "test running."));
                break;

            case TestEvent.CorePassed s:
                Row(s.Core).State = CoreState.Passed;
                Log($"{Row(s.Core).CoreName}: " + LocalizationService.Pick("bestanden.", "passed."), LogLevel.Success);
                break;

            case TestEvent.CoreFailed s:
                Row(s.Core).State = CoreState.Failed;
                Log($"{Row(s.Core).CoreName}: {s.Failure}", LogLevel.Error);
                if (!string.IsNullOrEmpty(webhookUrl))
                {
                    int currentMargin = (int)Row(s.Core).Margin;
                    int suggestedMargin = Math.Min(0, currentMargin + BackOffSteps);
                    _ = NotificationService.SendCoreFailedAsync(webhookUrl, s.Core, s.Failure.ToString(), currentMargin, suggestedMargin);
                }
                break;

            case TestEvent.CoreAutoTuned cat:
                Row(cat.Core).Margin = cat.Result.NextMargin;
                if (_smu.IsAvailable) _smu.WriteCurveOptimizer(cat.Core, cat.Result.NextMargin);
                if (cat.Result.CoreLocked)
                {
                    Row(cat.Core).IsLocked = true;
                    Row(cat.Core).LockedValue = cat.Result.NextMargin;
                    Row(cat.Core).State = CoreState.Locked;
                }
                UpdateAutoTunerProgress();
                Log(cat.Result.Advice);
                break;

            case TestEvent.Info s:
                Log(s.Message);
                break;
        }
    });

    /// <summary>Makes the auto-tuner's search visible instead of leaving it to a paragraph of text.</summary>
    private void UpdateAutoTunerProgress()
    {
        bool active = _running && AutoTunerModeBox.SelectedIndex > 0;
        AutoTunerProgressPanel.IsVisible = active;
        if (!active) return;

        var selected = _rows.Where(r => r.Selected).ToList();
        if (selected.Count == 0) return;

        int locked = selected.Count(r => r.IsLocked);
        AutoTunerProgressBar.Value = (double)locked / selected.Count;
        AutoTunerProgressText.Text = LocalizationService.Pick(
            $"{LocalizationService.Get("AutoTunerProgress")}: {locked} von {selected.Count} Kernen fixiert 🔒",
            $"{LocalizationService.Get("AutoTunerProgress")}: {locked} of {selected.Count} cores locked 🔒");
    }

    /// <summary>Accurate Ryzen CPU generation detection for y-cruncher binary selection.</summary>
    private ZenGeneration DetectGeneration()
    {
        string name = _smu.CpuName;
        if (!name.Contains("Ryzen", StringComparison.OrdinalIgnoreCase))
            return ZenGeneration.Unknown;

        if (name.Contains("9950") || name.Contains("9900") || name.Contains("9800") || name.Contains("9700") || name.Contains("9600"))
            return ZenGeneration.Zen5;
        if (name.Contains("7950") || name.Contains("7900") || name.Contains("7800") || name.Contains("7700") || name.Contains("7600") || name.Contains("7500"))
            return ZenGeneration.Zen4;
        if (name.Contains("5950") || name.Contains("5900") || name.Contains("5800") || name.Contains("5700") || name.Contains("5600") || name.Contains("5500"))
            return ZenGeneration.Zen3;
        if (name.Contains("3950") || name.Contains("3900") || name.Contains("3800") || name.Contains("3700") || name.Contains("3600") || name.Contains("3500"))
            return ZenGeneration.Zen2;
        if (name.Contains("2700") || name.Contains("2600") || name.Contains("1800") || name.Contains("1700") || name.Contains("1600"))
            return ZenGeneration.Zen1;

        return ZenGeneration.Zen4;
    }

    private void LoadProfiles()
    {
        int prevIdx = ProfileBox?.SelectedIndex ?? 0;
        var profiles = TestProfiles.For(_smu.CpuName, isGerman: LocalizationService.IsGerman);
        if (ProfileBox is not null)
        {
            ProfileBox.ItemsSource = profiles;
            ProfileBox.SelectedIndex = Math.Clamp(prevIdx, 0, profiles.Count - 1);
        }
    }

    /// <summary>A profile is just a preset: it fills the controls, it does not lock them.</summary>
    private void OnProfileChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (ProfileBox.SelectedItem is not TestProfile p) return;

        ProfileHelp.Text = p.Explanation;
        ProfileEngineText.Text = p.EngineSummary;
        PauseIntervalBox.Value = p.SuspendEverySeconds;
        PauseDurationBox.Value = p.SuspendForSeconds;
        MinutesBox.Value = p.MinutesPerCore;
        IterationsBox.Value = p.Iterations;
        ThreadsBox.SelectedIndex = p.Threads - 1;
        ModeBox.SelectedIndex = p.Mode switch
        {
            Prime95Mode.Sse => 0,
            Prime95Mode.Avx => 1,
            _ => 2,
        };
        FftBox.SelectedIndex = FftIndexOf(p.Fft);
        OrderBox.SelectedIndex = p.Order switch
        {
            CoreOrder.Sequential => 0,
            CoreOrder.Random => 2,
            _ => 1,
        };

        UpdateDurationHint();
    }

    /// <summary>Total runtime, taking core locks, current margins and the auto-tuner into account.</summary>
    private void UpdateDurationHint()
    {
        if (DurationHint is null || DurationHintBorder is null || DurationHintSub is null) return;

        bool isDe = LocalizationService.IsGerman;
        int selectedCores = _rows.Count(r => r.Selected);
        int phases = ProfileBox?.SelectedItem is TestProfile p ? p.Phases.Count : 1;
        string phaseNote = phases > 1 ? (isDe ? $" × {phases} Abschnitte" : $" × {phases} phases") : "";

        bool isAutoTuner = AutoTunerModeBox?.SelectedIndex > 0;
        if (isAutoTuner)
        {
            var mode = AutoTunerModeBox?.SelectedIndex == 1 ? AutoTunerMode.Grob : AutoTunerMode.Fein;
            string modeDesc = mode == AutoTunerMode.Grob ? (isDe ? "Grob -3" : "Coarse -3") : (isDe ? "Fein -1" : "Fine -1");
            int maxNegativeLimit = AutoTunerService.GetMaxNegativeMargin(_smu.CpuName);

            var activeRows = _rows.Where(r => r.Selected && !r.IsLocked).ToList();
            int lockedCount = _rows.Count(r => r.Selected && r.IsLocked);
            int maxPasses = (int)(IterationsBox.Value ?? 1);

            var total = EstimateTotalRuntime(mode, maxNegativeLimit, phases);

            DurationHintBorder.Classes.Set("success", false);
            DurationHintBorder.Classes.Set("auto", true);
            DurationHint.Foreground = SolidColorBrush.Parse("#C4B5FD");
            DurationHintSub.Foreground = SolidColorBrush.Parse("#A78BFA");

            if (activeRows.Count == 0)
            {
                DurationHint.Text = isDe
                    ? "🎯 Alle ausgewählten Kerne sind fixiert (🔒)"
                    : "🎯 All selected cores are locked (🔒)";
                DurationHintSub.Text = isDe
                    ? "Es gibt nichts mehr abzusenken. Einen Wert von Hand ändern hebt die Sperre wieder auf."
                    : "There is nothing left to lower. Editing a value by hand releases the lock again.";
            }
            else
            {
                string lockedNote = lockedCount > 0 ? (isDe ? $", {lockedCount} 🔒 fixiert" : $", {lockedCount} 🔒 locked") : "";
                DurationHint.Text = isDe
                    ? $"🤖 Auto-Tuner ({modeDesc}): max. {total:h\\:mm} Std. ({activeRows.Count} aktive Kerne{lockedNote})"
                    : $"🤖 Auto-tuner ({modeDesc}): up to {total:h\\:mm} hrs ({activeRows.Count} active cores{lockedNote})";

                DurationHintSub.Text = isDe
                    ? $"Senkt jeden Kern schrittweise bis zum Chip-Limit ({maxNegativeLimit}) ab. Fällt er durch, geht es wieder aufwärts und wird erneut getestet, bis ein Wert hält — erst dieser wird fixiert (🔒). Höchstens {maxPasses} Durchgänge à {MinutesBox.Value} Min; reichen sie nicht, bleibt der Kern offen."
                    : $"Lowers each core step by step toward the chip limit ({maxNegativeLimit}). If it fails, the search moves back up and re-tests until a value holds — only that one gets locked (🔒). At most {maxPasses} passes of {MinutesBox.Value} min; if that is not enough the core stays open.";
            }
        }
        else
        {
            var total = EstimateTotalRuntime(null, 0, phases);

            DurationHintBorder.Classes.Set("auto", false);
            DurationHintBorder.Classes.Set("success", true);
            DurationHint.Foreground = SolidColorBrush.Parse("#34D399");
            DurationHintSub.Foreground = SolidColorBrush.Parse("#6EE7B7");

            DurationHint.Text = total <= TimeSpan.Zero
                ? ""
                : (isDe
                    ? $"⏱️ Statischer Test: {selectedCores} Kerne × {MinutesBox.Value} Min × {IterationsBox.Value} Durchgänge{phaseNote} = {total:h\\:mm} Std."
                    : $"⏱️ Static test: {selectedCores} cores × {MinutesBox.Value} min × {IterationsBox.Value} passes{phaseNote} = {total:h\\:mm} hrs");

            DurationHintSub.Text = isDe
                ? "Prüft die aktuellen CO-Werte auf Stabilität, ohne sie zu verändern."
                : "Tests the current CO values for stability without changing them.";
        }

        if (PhasePlanText is not null)
        {
            PhasePlanText.Text = LocalizationService.Pick("Ablauf: ", "Plan: ") + PhaseSummary();
        }

        UpdateAutoTunerProgress();
    }

    // ══════════════════════════════════════════════════════════════
    // Watchdog
    // ══════════════════════════════════════════════════════════════

    private WheaWatcher? _watchdog;

    private void OnResetWheaBaseline(object? sender, RoutedEventArgs e)
    {
        string workRoot = Path.Combine(AppContext.BaseDirectory, "runs");
        WheaResetService.SetResetTimestamp(workRoot, DateTime.Now);

        Log(LocalizationService.Pick(
            "🧹 WHEA-Protokoll zurückgesetzt. Frühere Hardware-Fehler werden ab jetzt ignoriert.",
            "🧹 WHEA log reset. Past hardware errors are ignored from now on."), LogLevel.Success);
        StartWatchdog();
    }

    private void StartWatchdog()
    {
        string workRoot = Path.Combine(AppContext.BaseDirectory, "runs");
        DateTime? resetTime = WheaResetService.GetResetTimestamp(workRoot);
        bool isDe = LocalizationService.IsGerman;

        var past = WheaWatcher.ReadExisting();
        if (resetTime.HasValue)
        {
            past = past.Where(e => e.Time > resetTime.Value).ToList();
        }

        if (past.Count == 0)
        {
            WatchdogText.Text = LocalizationService.Get(resetTime.HasValue ? "WatchdogReset" : "WatchdogActive");
            WatchdogText.Foreground = SolidColorBrush.Parse("#10B981");
            WatchdogBadgeBorder.Classes.Set("danger", false);
            WatchdogBadgeBorder.Classes.Set("success", true);
            UpdateSmartRecommendations();
        }
        else
        {
            var perCore = past
                .Select(e => e.ApicId is { } id ? WheaWatcher.CoreForApicId(id, _cores) : null)
                .Where(c => c is not null)
                .GroupBy(c => c!.Value)
                .OrderByDescending(g => g.Count())
                .ToList();

            string coreLabel = isDe ? "Kern" : "Core";
            string cores = perCore.Count == 0
                ? ""
                : " (" + string.Join(", ", perCore.Select(g => $"{coreLabel} {g.Key}")) + ")";

            WatchdogText.Text = isDe
                ? $"🛡️ WÄCHTER: ⚠️ {past.Count} HW-Fehler{cores}"
                : $"🛡️ WATCHDOG: ⚠️ {past.Count} HW error(s){cores}";
            WatchdogText.Foreground = SolidColorBrush.Parse("#F87171");
            WatchdogBadgeBorder.Classes.Set("success", false);
            WatchdogBadgeBorder.Classes.Set("danger", true);

            UpdateSmartRecommendations(past);
            foreach (var e in past.Take(5))
                Log(LocalizationService.Pick("Protokoll: ", "Log: ") + Describe(e), LogLevel.Warn);
        }

        _watchdog?.Dispose();
        _watchdog = new WheaWatcher();
        _watchdog.Raised += e => Dispatcher.UIThread.Post(() =>
        {
            WatchdogText.Text = LocalizationService.Pick(
                $"🛡️ WÄCHTER: ⚠️ Soeben ein Hardware-Fehler — {Describe(e)}",
                $"🛡️ WATCHDOG: ⚠️ Hardware error just now — {Describe(e)}");
            WatchdogText.Foreground = SolidColorBrush.Parse("#F87171");
            WatchdogBadgeBorder.Classes.Set("success", false);
            WatchdogBadgeBorder.Classes.Set("danger", true);
            Log(LocalizationService.Pick("ACHTUNG: ", "WARNING: ") + Describe(e), LogLevel.Error);
            UpdateSmartRecommendations(WheaWatcher.ReadExisting());
        });

        try { _watchdog.Start(); }
        catch (Exception ex)
        {
            Log(LocalizationService.Pick($"Überwachung nicht möglich: {ex.Message}", $"Monitoring unavailable: {ex.Message}"), LogLevel.Warn);
        }
    }

    private string Describe(WheaEvent e)
    {
        bool isDe = LocalizationService.IsGerman;
        int? core = e.ApicId is { } id ? WheaWatcher.CoreForApicId(id, _cores) : null;
        string where = core is not null
            ? (isDe ? $"Kern {core}" : $"core {core}")
            : (isDe ? "unbekannter Kern" : "unknown core");
        return isDe
            ? $"{e.Time:dd.MM.yyyy HH:mm} — {where}, korrigierter Hardware-Fehler (WHEA {e.EventId})"
            : $"{e.Time:dd.MM.yyyy HH:mm} — {where}, corrected hardware error (WHEA {e.EventId})";
    }

    // ══════════════════════════════════════════════════════════════
    // Telemetry
    // ══════════════════════════════════════════════════════════════

    private void SetTelemetryOffline()
    {
        foreach (var chip in new Control[] { BoostChip, PptChip, TdcChip, EdcChip, TempChip, ScalarChip })
            chip.IsVisible = false;

        TelemetryOfflineText.IsVisible = true;
        TelemetryOfflineText.Text = LocalizationService.Get("MetricNoTelemetry");
        LiveTelemetryGraph.EmptyMessage = LocalizationService.Get("MetricNoTelemetry");
    }

    /// <summary>Live telemetry from the power table. Limits are static, the values are not.</summary>
    private void UpdatePbo()
    {
        if (_smu.ReadPbo() is not { } pbo) return;

        BoostValue.Text = $"{pbo.BoostMhz} MHz";
        ScalarValue.Text = $"{pbo.Scalar}×";

        if (pbo.HasLimits)
        {
            SetMetric(PptValue, PptBar, pbo.PptNowW, pbo.PptLimitW, "W");
            SetMetric(TdcValue, TdcBar, pbo.TdcNowA, pbo.TdcLimitA, "A");
            SetMetric(EdcValue, EdcBar, pbo.EdcNowA, pbo.EdcLimitA, "A");
            SetMetric(TempValue, TempBar, pbo.TempNowC, pbo.TempLimitC, "°C");
        }
        else
        {
            PptChip.IsVisible = TdcChip.IsVisible = EdcChip.IsVisible = false;
            SetMetric(TempValue, TempBar, pbo.TempNowC, pbo.TempLimitC, "°C");
        }

        LiveTelemetryGraph.AddSample(pbo.BoostMhz, pbo.TempNowC ?? 0f, pbo.PptNowW ?? 0f);

        double maxTemp = (double)(MaxTempBox?.Value ?? 90m);
        LiveTelemetryGraph.TempLimit = maxTemp;

        if (_running && _runCts is not null && !_runCts.IsCancellationRequested && (pbo.TempNowC ?? 0) >= maxTemp)
        {
            Log(LocalizationService.Pick(
                $"🚨 TEMPERATUR-NOTSCHUTZ: {pbo.TempNowC:F0} °C erreicht (Limit {maxTemp} °C). Test wird abgebrochen.",
                $"🚨 TEMPERATURE CUT-OFF: reached {pbo.TempNowC:F0} °C (limit {maxTemp} °C). Aborting the run."), LogLevel.Error);
            PlayNotificationSound(isError: true);
            _runCts.Cancel();
        }
    }

    private const double MetricBarWidth = 72;

    /// <summary>Value plus a proportional bar; colour comes from how close it is to its limit.</summary>
    private static void SetMetric(TextBlock value, Border bar, float? now, float? limit, string unit)
    {
        if (now is null)
        {
            value.Text = "–";
            bar.Width = 0;
            return;
        }

        value.Text = limit is > 0 ? $"{now:F0}/{limit:F0} {unit}" : $"{now:F0} {unit}";

        double ratio = limit is > 0 ? Math.Clamp(now.Value / limit.Value, 0, 1) : 0;
        bar.Width = ratio * MetricBarWidth;
        bar.Background = SolidColorBrush.Parse(ratio switch
        {
            >= 0.97 => "#F87171",
            >= 0.85 => "#FBBF24",
            _ => "#10B981",
        });
        value.Foreground = SolidColorBrush.Parse(ratio >= 0.97 ? "#F87171" : "#CBD5E1");
    }

    // How far to back off a core that failed. One step is false economy: the value that only
    // just fails under test is not safe either, and re-testing costs hours.
    private const int BackOffSteps = 4;

    private readonly Dictionary<int, int> _recommended = [];

    /// <summary>Turns the raw failure list into something a person can act on.</summary>
    private void ShowResult(IReadOnlyDictionary<int, List<Failure>> failures, string webhookUrl = "")
    {
        _recommended.Clear();
        _detailIsAudit = false;
        var totalDuration = DateTime.Now - _runStarted;

        PlayNotificationSound(isError: failures.Count > 0);
        PerformPostTestAction();

        if (!string.IsNullOrEmpty(webhookUrl))
        {
            _ = NotificationService.SendRunFinishedAsync(webhookUrl, failures.Count, totalDuration);
        }

        if (failures.Count == 0)
        {
            ResultPanel.Classes.Set("danger", false);
            ResultPanel.Classes.Set("success", true);
            ResultTitle.Foreground = SolidColorBrush.Parse("#34D399");
            ResultTitle.Text = LocalizationService.Get("ResultPassTitle");
            ResultText.Text = LocalizationService.Get("ResultPassText");
            AcceptButton.IsVisible = false;
            ExportReportButton.IsVisible = true;
            ResultActions.IsVisible = true;
            ResultPanel.IsVisible = true;
            Log(LocalizationService.Pick("Fertig. Kein Kern ist durchgefallen.", "Done. No core failed."), LogLevel.Success);
            return;
        }

        var lines = new List<string>();
        foreach (var (core, list) in failures.OrderBy(f => f.Key))
        {
            int current = (int)Row(core).Margin;
            int suggestion = Math.Min(0, current + BackOffSteps);
            _recommended[core] = suggestion;

            string reason = LocalizationService.Get(list[0].Kind switch
            {
                FailureKind.CalculationError => "ReasonCalc",
                FailureKind.MachineCheck => "ReasonMachineCheck",
                FailureKind.ProcessGone => "ReasonProcessGone",
                _ => "ReasonHang",
            });

            lines.Add($"{Row(core).CoreName} {reason}. {LocalizationService.Pick("Vorschlag", "Suggestion")}: {current} → {suggestion}.");
            Log($"{Row(core).CoreName}: {list.Count} {LocalizationService.Pick("Fehler", "errors")} — {current} → {suggestion}", LogLevel.Error);
        }

        ResultPanel.Classes.Set("success", false);
        ResultPanel.Classes.Set("danger", true);
        ResultTitle.Foreground = SolidColorBrush.Parse("#F87171");
        ResultTitle.Text = failures.Count == 1
            ? LocalizationService.Get("ResultFailOne")
            : string.Format(LocalizationService.Get("ResultFailMany"), failures.Count);

        ResultText.Text = string.Join("\n", lines) + "\n\n" +
            string.Format(LocalizationService.Get("ResultFailAdvice"), BackOffSteps);

        // Was previously left hidden forever after a single click on "Copy BIOS list".
        AcceptButton.IsVisible = true;
        ExportReportButton.IsVisible = true;
        ResultActions.IsVisible = true;
        ResultPanel.IsVisible = true;
    }

    private void OnAcceptRecommendation(object? sender, RoutedEventArgs e)
    {
        foreach (var (core, margin) in _recommended)
            Row(core).Margin = margin;

        Log(LocalizationService.Pick(
            $"{_recommended.Count} Wert(e) übernommen. Jetzt unten rechts „{LocalizationService.Get("ApplyLive")}“ drücken, damit sie wirksam werden.",
            $"Applied {_recommended.Count} value(s). Now press “{LocalizationService.Get("ApplyLive")}” at the bottom right to make them take effect."),
            LogLevel.Success);

        // Selecting exactly the affected cores makes the obvious next step a single click.
        foreach (var row in _rows) row.Selected = _recommended.ContainsKey(row.Index);
        UpdateSelectAllState();
        UpdateDurationHint();

        ResultPanel.IsVisible = false;
    }

    private void OnDismissResult(object? sender, RoutedEventArgs e)
    {
        ResultPanel.IsVisible = false;
        _detailIsAudit = false;
    }

    private void OnExportReport(object? sender, RoutedEventArgs e)
    {
        string workRoot = Path.Combine(AppContext.BaseDirectory, "runs");
        var currentMargins = _rows.ToDictionary(r => r.Index, r => (int)r.Margin);
        var wheaEvents = WheaWatcher.ReadExisting();
        string profileName = ProfileBox.SelectedItem is TestProfile p ? p.Name : "Custom";
        TimeSpan totalDuration = DateTime.Now - _runStarted;

        string reportPath = ReportExporterService.SaveReport(
            workRoot, _smu.CpuName, currentMargins, wheaEvents, profileName, totalDuration, _recommended.Count);

        Log(LocalizationService.Pick($"📄 Bericht exportiert: {reportPath}", $"📄 Report exported: {reportPath}"), LogLevel.Success);
        try { Process.Start(new ProcessStartInfo(reportPath) { UseShellExecute = true }); } catch { }
    }

    /// <summary>
    /// Elapsed and remaining time. The estimate comes from the plan rather than from measured
    /// progress, so it is honest about the schedule but does not account for a run being cut
    /// short - a core that fails early ends its slot sooner than planned.
    /// </summary>
    private void UpdateClock()
    {
        var elapsed = DateTime.Now - _runStarted;
        ElapsedText.Text = $"{elapsed:h\\:mm\\:ss}";

        if (_plannedTotal <= TimeSpan.Zero) return;

        double done = Math.Clamp(elapsed.TotalSeconds / _plannedTotal.TotalSeconds, 0, 1);
        RunProgress.Value = done * 100;

        var left = _plannedTotal - elapsed;
        RemainingText.Text = left > TimeSpan.Zero
            ? LocalizationService.Pick(
                $"noch etwa {left:h\\:mm\\:ss} · fertig gegen {DateTime.Now.Add(left):HH:mm} Uhr",
                $"about {left:h\\:mm\\:ss} left · done around {DateTime.Now.Add(left):HH:mm}")
            : LocalizationService.Pick("gleich fertig", "almost done");

        PhaseText.Text = _phaseLabel;
    }

    private void OnRuntimeChanged(object? sender, NumericUpDownValueChangedEventArgs e) => UpdateDurationHint();

    private void OnToggleAllCores(object? sender, RoutedEventArgs e)
    {
        // The box has already toggled by the time this runs, so just follow it: checked means
        // select everything, unchecked means deselect everything. A partial selection toggles
        // to unchecked first, so the second click always clears.
        bool select = SelectAllBox.IsChecked == true;

        _suppressSelectAll = true;
        foreach (var row in _rows) row.Selected = select;
        _suppressSelectAll = false;

        UpdateDurationHint();
    }

    private async void OnSetAllMax(object? sender, RoutedEventArgs e)
    {
        int maxLimit = AutoTunerService.GetMaxNegativeMargin(_smu.CpuName);
        int affected = _rows.Count(r => r.Selected);
        if (affected == 0) return;

        // One click used to be enough to put every core at the chip limit.
        bool ok = await ConfirmDialog.ShowAsync(
            this,
            string.Format(LocalizationService.Get("ConfirmAllMax"), affected, maxLimit),
            LocalizationService.Get("ConfirmYes"));
        if (!ok) return;

        foreach (var row in _rows.Where(r => r.Selected))
        {
            row.Margin = maxLimit;
            row.IsLocked = false;
        }
        Log(LocalizationService.Pick(
            $"{affected} Kern(e) auf das Chip-Limit {maxLimit} gesetzt.",
            $"Set {affected} core(s) to the chip limit {maxLimit}."), LogLevel.Warn);
        UpdateDurationHint();
    }

    /// <summary>
    /// Back to what the BIOS set, in the table. The CPU only follows once the values are
    /// applied, exactly like every other edit in this column.
    /// </summary>
    private void OnSetAllOriginal(object? sender, RoutedEventArgs e)
    {
        int affected = 0;
        foreach (var row in _rows.Where(r => r.Selected))
        {
            if (row.Margin != row.BiosBaseline) affected++;
            row.Margin = row.BiosBaseline;
            row.IsLocked = false;
            if (row.State == CoreState.Locked) row.State = CoreState.Ready;
        }

        Log(affected == 0
            ? LocalizationService.Pick(
                "Die ausgewählten Kerne stehen bereits auf ihren BIOS-Werten.",
                "The selected cores already hold their BIOS values.")
            : LocalizationService.Pick(
                $"{affected} Kern(e) auf die BIOS-Ausgangswerte zurückgesetzt. Mit „{LocalizationService.Get("ApplyLive")}“ auch in die CPU schreiben.",
                $"Reset {affected} core(s) to their BIOS values. Use “{LocalizationService.Get("ApplyLive")}” to write them to the CPU as well."));

        UpdateDurationHint();
    }

    private CoreRow Row(int index) => _rows[index];

    /// <summary>
    /// Locks down everything that cannot take effect mid-run. Leaving these live meant the UI
    /// accepted changes it then silently ignored. The temperature cut-off stays editable on
    /// purpose - raising a safety limit must never require stopping the run first.
    /// </summary>
    private void SetRunning(bool running)
    {
        _running = running;
        ProgressPanel.IsVisible = running;

        if (!running)
        {
            _clock.Stop();
            Log(LocalizationService.Pick(
                $"Gesamtdauer: {DateTime.Now - _runStarted:h\\:mm\\:ss}",
                $"Total duration: {DateTime.Now - _runStarted:h\\:mm\\:ss}"));
        }

        if (running)
        {
            StartButton.Content = LocalizationService.Get("StopTest");
            StartButton.Classes.Set("primary", false);
            StartButton.Classes.Set("auto", false);
            StartButton.Classes.Set("danger", true);
            StartButton.IsEnabled = true;
        }
        else
        {
            StartButton.Classes.Set("danger", false);
            UpdateStartButtonState();
        }

        ApplyButton.IsEnabled = !running && _smu.IsAvailable;
        CopyBiosButton.IsEnabled = !running;
        ApplyRecommendationButton.IsEnabled = !running;
        SetAllMaxButton.IsEnabled = !running;
        SetAllOriginalButton.IsEnabled = !running;

        foreach (var c in new Control[]
                 {
                     MinutesBox, IterationsBox, ThreadsBox, FftBox, ModeBox, OrderBox, EngineBox,
                     ProfileBox, AutoTunerModeBox, PauseIntervalBox, PauseDurationBox, YcAlgoBox,
                     StopOnErrorBox, SelectAllBox, AutostartBox,
                     FftMinBox, FftMaxBox, CustomOrderBox,
                     SkipCoreOnErrorBox, CoreDelayBox, TreatWheaAsErrorBox,
                 })
        {
            c.IsEnabled = !running;
        }
        if (CcdButtons.ItemsSource is IEnumerable<Button> ccdButtons)
            foreach (var b in ccdButtons) b.IsEnabled = !running;

        UpdateAutoTunerProgress();
    }

    // ══════════════════════════════════════════════════════════════
    // Log
    // ══════════════════════════════════════════════════════════════

    private const int MaxLogLines = 1500;

    /// <summary>
    /// Appends as an inline run rather than concatenating the whole text. The old
    /// <c>Text += …</c> re-copied and re-laid-out the entire buffer on every line, which an
    /// overnight run turns into a real cost. Colour by level so errors do not read as successes.
    /// </summary>
    private void Log(string message, LogLevel level = LogLevel.Info)
    {
        string line = $"[{DateTime.Now:HH:mm:ss}] {message}\n";
        _logBuffer.Append(line);

        try { _logFile?.Write($"[{DateTime.Now:yyyy-MM-dd HH:mm:ss}] [{level,-7}] {message}\n"); }
        catch { }

        if (LogText?.Inlines is not { } inlines) return;

        inlines.Add(new Run(line)
        {
            Foreground = SolidColorBrush.Parse(level switch
            {
                LogLevel.Success => "#34D399",
                LogLevel.Warn => "#FBBF24",
                LogLevel.Error => "#F87171",
                _ => "#94A3B8",
            }),
        });

        while (inlines.Count > MaxLogLines) inlines.RemoveAt(0);

        LogScroller?.ScrollToEnd();
    }

    private async void OnCopyLog(object? sender, RoutedEventArgs e)
    {
        if (Clipboard is null) return;
        await Clipboard.SetTextAsync(_logBuffer.ToString());
        Log(LocalizationService.Pick("📋 Protokoll kopiert.", "📋 Log copied."), LogLevel.Success);
    }

    private void OnClearLog(object? sender, RoutedEventArgs e)
    {
        _logBuffer.Clear();
        LogText?.Inlines?.Clear();
    }
}
