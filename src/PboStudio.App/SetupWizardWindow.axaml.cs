using System.Diagnostics;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Media;
using Avalonia.Threading;
using PboStudio.Core;

namespace PboStudio.App;

public partial class SetupWizardWindow : Window
{
    private readonly SmuService _smu;
    private CancellationTokenSource? _downloadCts;
    private bool _busy;

    public SetupWizardWindow() : this(new SmuService()) { }

    public SetupWizardWindow(SmuService smu)
    {
        InitializeComponent();
        _smu = smu;
        ApplyLocalization();
        // Was synchronous in the constructor: file probing plus an SMU round trip before the
        // window could paint.
        _ = RefreshStatusAsync();
    }

    private void ApplyLocalization()
    {
        Title = "PboStudio – " + LocalizationService.Get("WizardTitle");
        HeaderTitle.Text = LocalizationService.Get("WizardTitle");
        HeaderSubtitle.Text = LocalizationService.Get("WizardSubtitle");

        PawnIoTitle.Text = LocalizationService.Get("WizardPawnIo");
        PawnIoWhat.Text = LocalizationService.Get("WizardPawnIoWhat");
        PawnIoAdminNote.Text = LocalizationService.Get("WizardPawnIoNeedsAdmin");
        InstallPawnIoButton.Content = LocalizationService.Get("WizardInstall");
        PawnIoWebButton.Content = LocalizationService.Get("WizardWebsite");

        Prime95Title.Text = LocalizationService.Get("WizardPrime95");
        Prime95What.Text = LocalizationService.Get("WizardPrime95What");
        DownloadPrime95Button.Content = LocalizationService.Get("WizardDownload");

        YCruncherTitle.Text = LocalizationService.Get("WizardYCruncher");
        YCruncherWhat.Text = LocalizationService.Get("WizardYCruncherWhat");
        DownloadYCruncherButton.Content = LocalizationService.Get("WizardDownload");

        InstallAllButton.Content = LocalizationService.Get("WizardInstallAll");
        RecheckButton.Content = LocalizationService.Get("WizardRecheck");
        CloseButton.Content = LocalizationService.Get("Close");
        CancelDownloadButton.Content = LocalizationService.Get("WizardCancel");
        OpenDownloadPageButton.Content = LocalizationService.Get("WizardOpenPage");
    }

    // ══════════════════════════════════════════════════════════════
    // Status
    // ══════════════════════════════════════════════════════════════

    private async Task RefreshStatusAsync()
    {
        var status = await Task.Run(() => DependencyService.CheckDependencies(AppContext.BaseDirectory, _smu));

        PawnIoStatusText.Text = status.PawnIoAvailable
            ? LocalizationService.Get("WizardPawnIoOk")
            : $"❌ {status.PawnIoStatus}";
        PawnIoStatusText.Foreground = StatusBrush(status.PawnIoAvailable);
        // Was the one card whose button never switched off once its component was present.
        InstallPawnIoButton.IsEnabled = !status.PawnIoAvailable && !_busy;

        Prime95StatusText.Text = status.Prime95Available
            ? $"{LocalizationService.Get("WizardInstalled")} — {status.Prime95Path}"
            : LocalizationService.Get("WizardNotFound");
        Prime95StatusText.Foreground = StatusBrush(status.Prime95Available);
        DownloadPrime95Button.IsEnabled = !status.Prime95Available && !_busy;

        YCruncherStatusText.Text = status.YCruncherAvailable
            ? $"{LocalizationService.Get("WizardInstalled")} — {status.YCruncherPath}"
            : LocalizationService.Get("WizardNotFound");
        YCruncherStatusText.Foreground = StatusBrush(status.YCruncherAvailable);
        DownloadYCruncherButton.IsEnabled = !status.YCruncherAvailable && !_busy;

        int missing = (status.PawnIoAvailable ? 0 : 1)
                    + (status.Prime95Available ? 0 : 1)
                    + (status.YCruncherAvailable ? 0 : 1);

        SummaryText.Text = missing == 0
            ? LocalizationService.Get("WizardReady")
            : string.Format(LocalizationService.Get("WizardMissing"), missing);
        SummaryText.Foreground = SolidColorBrush.Parse(missing == 0 ? "#34D399" : "#FBBF24");
        SummaryBanner.Classes.Set("success", missing == 0);
        SummaryBanner.Classes.Set("warning", missing != 0);

        // Downloadable components only; PawnIO runs its own elevated installer.
        InstallAllButton.IsEnabled = !_busy && (!status.Prime95Available || !status.YCruncherAvailable);
        RecheckButton.IsEnabled = !_busy;
    }

    private static IBrush StatusBrush(bool ok) => SolidColorBrush.Parse(ok ? "#34D399" : "#F87171");

    private void SetBusy(bool busy)
    {
        _busy = busy;
        DownloadProgressPanel.IsVisible = busy;
        InstallPawnIoButton.IsEnabled = !busy;
        DownloadPrime95Button.IsEnabled = !busy;
        DownloadYCruncherButton.IsEnabled = !busy;
        InstallAllButton.IsEnabled = !busy;
        RecheckButton.IsEnabled = !busy;
    }

    // ══════════════════════════════════════════════════════════════
    // Actions
    // ══════════════════════════════════════════════════════════════

    private async void OnRecheck(object? sender, RoutedEventArgs e) => await RefreshStatusAsync();

    private async void OnDownloadPrime95(object? sender, RoutedEventArgs e)
    {
        await RunDownloadAsync(DependencyService.Prime95);
        await RefreshStatusAsync();
    }

    private async void OnDownloadYCruncher(object? sender, RoutedEventArgs e)
    {
        await RunDownloadAsync(DependencyService.YCruncher);
        await RefreshStatusAsync();
    }

    /// <summary>One click for the common first-run case instead of three separate ones.</summary>
    private async void OnInstallAll(object? sender, RoutedEventArgs e)
    {
        var status = await Task.Run(() => DependencyService.CheckDependencies(AppContext.BaseDirectory, _smu));

        if (!status.Prime95Available)
            await RunDownloadAsync(DependencyService.Prime95);

        if (!status.YCruncherAvailable && _downloadCts?.IsCancellationRequested != true)
            await RunDownloadAsync(DependencyService.YCruncher);

        await RefreshStatusAsync();
    }

    private EngineSource? _lastFailedSource;

    private async Task RunDownloadAsync(EngineSource source)
    {
        string name = source.Name;
        _downloadCts = new CancellationTokenSource();
        SetBusy(true);
        OpenDownloadPageButton.IsVisible = false;
        DownloadProgressBar.Value = 0;
        DownloadProgressBar.IsIndeterminate = false;
        ProgressLabelText.Foreground = SolidColorBrush.Parse("#38BDF8");

        try
        {
            await DependencyService.DownloadEngineAsync(AppContext.BaseDirectory, source, (pct, status) =>
            {
                Dispatcher.UIThread.Post(() =>
                {
                    ProgressLabelText.Text = $"{name}: {status}";
                    if (pct >= 0)
                    {
                        DownloadProgressBar.IsIndeterminate = false;
                        DownloadProgressBar.Value = pct;
                    }
                    else
                    {
                        DownloadProgressBar.IsIndeterminate = true;
                    }
                });
            }, _downloadCts.Token);
        }
        catch (OperationCanceledException)
        {
            ProgressLabelText.Text = LocalizationService.Pick($"{name}: abgebrochen.", $"{name}: cancelled.");
        }
        catch (Exception ex)
        {
            ProgressLabelText.Foreground = SolidColorBrush.Parse("#F87171");
            string targetDir = DependencyService.TargetDirectoryFor(AppContext.BaseDirectory, source);
            ProgressLabelText.Text = LocalizationService.Pick(
                $"{name}: {ex.Message}\nDu kannst das Archiv von Hand laden und nach „{targetDir}“ entpacken.",
                $"{name}: {ex.Message}\nYou can download the archive by hand and extract it to “{targetDir}”.");

            // A dead end is the one outcome the setup must never produce: offer the page.
            _lastFailedSource = source;
            OpenDownloadPageButton.IsVisible = true;

            SetBusy(false);
            DownloadProgressPanel.IsVisible = true;
            return;
        }
        finally
        {
            _downloadCts?.Dispose();
            _downloadCts = null;
        }

        SetBusy(false);
    }

    private void OnCancelDownload(object? sender, RoutedEventArgs e) => _downloadCts?.Cancel();

    private void OnOpenDownloadPage(object? sender, RoutedEventArgs e)
    {
        string url = _lastFailedSource?.PageUrl ?? DependencyService.Prime95.PageUrl;
        try { Process.Start(new ProcessStartInfo(url) { UseShellExecute = true }); } catch { }
    }

    private async void OnDownloadPawnIo(object? sender, RoutedEventArgs e)
    {
        SetBusy(true);
        DownloadProgressBar.IsIndeterminate = true;
        CancelDownloadButton.IsEnabled = false;
        ProgressLabelText.Foreground = SolidColorBrush.Parse("#38BDF8");

        try
        {
            await DependencyService.DownloadAndInstallPawnIoAsync(msg =>
                Dispatcher.UIThread.Post(() => ProgressLabelText.Text = msg));

            ProgressLabelText.Text = LocalizationService.Pick(
                "Installer gestartet. Nach Abschluss auf „Erneut prüfen“ klicken — für vollen SMU-Zugriff ist ein Neustart von PboStudio nötig.",
                "Installer started. Click “Check again” once it finishes — PboStudio must be restarted for full SMU access.");
        }
        catch (Exception ex)
        {
            ProgressLabelText.Foreground = SolidColorBrush.Parse("#F87171");
            ProgressLabelText.Text = LocalizationService.Pick(
                $"Fehler beim Start des PawnIO-Installers: {ex.Message}",
                $"Error starting the PawnIO installer: {ex.Message}");
        }
        finally
        {
            SetBusy(false);
            CancelDownloadButton.IsEnabled = true;
            DownloadProgressBar.IsIndeterminate = false;
            DownloadProgressPanel.IsVisible = true;
            await RefreshStatusAsync();
        }
    }

    private void OnOpenPawnIoWeb(object? sender, RoutedEventArgs e)
    {
        try { Process.Start(new ProcessStartInfo("https://pawnio.eu/") { UseShellExecute = true }); }
        catch { }
    }

    private void OnClose(object? sender, RoutedEventArgs e) => Close();
}
