using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using PboStudio.Core;

namespace PboStudio.App;

public sealed record TelemetrySample(
    float ClockMhz,
    float TempC,
    float PowerW,
    DateTime Timestamp
);

/// <summary>
/// Three stacked lanes rather than three curves sharing one normalised axis. At the ~50 px
/// per series this control actually gets, overlaid lines with a single combined scale label
/// were not readable: nothing tied a number to a curve.
/// </summary>
public class TelemetryGraphControl : Control
{
    // 15 minutes at one sample every two seconds. The old two-minute window was a keyhole
    // for a run that takes all night.
    private const int MaxSamples = 450;

    private const double AxisStripHeight = 15;
    private const double LanePadding = 3;

    /// <summary>
    /// Band at the top of every lane reserved for its title and scale label. Without it the
    /// plot area started at the very top of the lane and any value near the maximum drew
    /// straight through the text naming it.
    /// </summary>
    private const double LaneHeaderHeight = 13;

    private const double LabelSize = 9.5;
    private const double ScaleLabelSize = 9;

    private readonly List<TelemetrySample> _samples = [];
    private double? _hoverX;

    // Parsed once. These used to be re-parsed on every frame, ten brushes per render.
    private static readonly IBrush BackgroundBrush = SolidColorBrush.Parse("#08090D");
    private static readonly IBrush LaneBrush = SolidColorBrush.Parse("#0B0D12");
    // Was #475569 at 8px: 2.63:1 on this background, i.e. the axis numbers were effectively
    // invisible. #8494A8 clears 5.6:1 and the label size moved up with it.
    private static readonly IBrush TextFaint = SolidColorBrush.Parse("#8494A8");
    private static readonly IBrush TextMuted = SolidColorBrush.Parse("#94A3B8");
    private static readonly IBrush TextBright = SolidColorBrush.Parse("#F8FAFC");
    private static readonly IBrush TooltipBg = SolidColorBrush.Parse("#161B26");

    private static readonly IBrush ClockBrush = SolidColorBrush.Parse("#38BDF8");
    private static readonly IBrush TempBrush = SolidColorBrush.Parse("#F97316");
    private static readonly IBrush PowerBrush = SolidColorBrush.Parse("#22D3EE");

    private static readonly Pen BorderPen = new(SolidColorBrush.Parse("#1C202D"), 1);
    private static readonly Pen GridPen = new(SolidColorBrush.Parse("#161A25"), 1);
    private static readonly Pen SeparatorPen = new(SolidColorBrush.Parse("#12151E"), 1);
    private static readonly Pen CrosshairPen = new(SolidColorBrush.Parse("#475569"), 1, new DashStyle([2, 2], 0));
    private static readonly Pen LimitPen = new(SolidColorBrush.Parse("#EF4444"), 1, new DashStyle([4, 3], 0));

    // Solid / dashed / dotted, so the three lanes stay distinguishable in a screenshot and
    // for the orange-versus-green pairing that trips up deuteranopia.
    private static readonly Pen ClockPen = new(SolidColorBrush.Parse("#38BDF8"), 1.8);
    private static readonly Pen TempPen = new(SolidColorBrush.Parse("#F97316"), 1.8, new DashStyle([5, 2], 0));
    private static readonly Pen PowerPen = new(SolidColorBrush.Parse("#22D3EE"), 1.8, new DashStyle([1.5, 2], 0));

    private static readonly Typeface MonoFace = new("Cascadia Mono, Consolas, monospace");

    private string _labelClock = "CLOCK";
    private string _labelTemp = "TEMP";
    private string _labelPower = "POWER";
    private string _labelNoData = "No samples yet";
    private string _labelLimit = "limit";
    private string _labelNow = "now";

    public TelemetryGraphControl()
    {
        ClipToBounds = true;
    }

    /// <summary>Emergency cut-off from the System tab, drawn into the temperature lane.</summary>
    public double TempLimit { get; set; } = 90;

    /// <summary>Shown instead of an empty black box when there is no telemetry at all.</summary>
    public string? EmptyMessage { get; set; }

    public void ApplyLocalization()
    {
        _labelClock = LocalizationService.Get("GraphClock");
        _labelTemp = LocalizationService.Get("GraphTemp");
        _labelPower = LocalizationService.Get("GraphPower");
        _labelNoData = LocalizationService.Get("GraphNoData");
        _labelLimit = LocalizationService.Get("GraphLimit");
        _labelNow = LocalizationService.Get("GraphNow");
        InvalidateVisual();
    }

    public void AddSample(float clockMhz, float tempC, float powerW)
    {
        _samples.Add(new TelemetrySample(clockMhz, tempC, powerW, DateTime.Now));
        if (_samples.Count > MaxSamples) _samples.RemoveAt(0);
        InvalidateVisual();
    }

    public void Clear()
    {
        _samples.Clear();
        _hoverX = null;
        InvalidateVisual();
    }

    protected override void OnPointerMoved(PointerEventArgs e)
    {
        base.OnPointerMoved(e);
        _hoverX = e.GetPosition(this).X;
        InvalidateVisual();
    }

    protected override void OnPointerExited(PointerEventArgs e)
    {
        base.OnPointerExited(e);
        _hoverX = null;
        InvalidateVisual();
    }

    private static FormattedText Text(string s, double size, IBrush brush) =>
        new(s, System.Globalization.CultureInfo.CurrentCulture, FlowDirection.LeftToRight, MonoFace, size, brush);

    private readonly record struct Lane(
        string Title, IBrush Brush, Pen Pen, double Top, double Height, float Max, string Unit, string Format);

    public override void Render(DrawingContext context)
    {
        base.Render(context);

        double width = Bounds.Width;
        double height = Bounds.Height;
        if (width <= 4 || height <= 4) return;

        context.DrawRectangle(BackgroundBrush, BorderPen, new Rect(0, 0, width, height));

        if (_samples.Count == 0)
        {
            string message = EmptyMessage ?? _labelNoData;
            var empty = Text(message, 11, TextFaint);
            context.DrawText(empty, new Point((width - empty.Width) / 2, (height - empty.Height) / 2));
            return;
        }

        // ── scales ────────────────────────────────────────────────
        float peakClock = _samples.Max(s => s.ClockMhz);
        float peakPower = _samples.Max(s => s.PowerW);

        float maxClock = Math.Max(1000f, (float)Math.Ceiling(Math.Max(peakClock, 4000f) / 500.0) * 500);
        float maxTemp = (float)Math.Max(100.0, Math.Ceiling(TempLimit / 10.0) * 10);
        float maxPower = Math.Max(50f, (float)Math.Ceiling(Math.Max(peakPower, 100f) / 50.0) * 50);

        double laneHeight = (height - AxisStripHeight) / 3.0;

        var lanes = new[]
        {
            new Lane(_labelClock, ClockBrush, ClockPen, 0,                  laneHeight, maxClock, "MHz", "F0"),
            new Lane(_labelTemp,  TempBrush,  TempPen,  laneHeight,         laneHeight, maxTemp,  "°C",  "F0"),
            new Lane(_labelPower, PowerBrush, PowerPen, laneHeight * 2,     laneHeight, maxPower, "W",   "F0"),
        };

        // Right-aligned: the newest sample is always at the right edge and history scrolls
        // left. Drawing from x=0 instead made the "now" label point at empty canvas whenever
        // the 15-minute window was not yet full.
        double stepX = width / (MaxSamples - 1);
        double XFor(int i) => width - (_samples.Count - 1 - i) * stepX;

        for (int laneIndex = 0; laneIndex < lanes.Length; laneIndex++)
        {
            var lane = lanes[laneIndex];
            double plotTop = lane.Top + LaneHeaderHeight;
            double plotHeight = Math.Max(4, lane.Height - LaneHeaderHeight - LanePadding);

            if (laneIndex % 2 == 1)
                context.DrawRectangle(LaneBrush, null, new Rect(0, lane.Top, width, lane.Height));

            if (laneIndex > 0)
                context.DrawLine(SeparatorPen, new Point(0, lane.Top), new Point(width, lane.Top));

            // Mid-scale grid line, and this time the label sits on the line it describes.
            double midY = plotTop + plotHeight / 2;
            context.DrawLine(GridPen, new Point(0, midY), new Point(width, midY));

            // Temperature cut-off: the number that ends a run deserves to be on the chart.
            if (laneIndex == 1 && TempLimit > 0 && TempLimit <= lane.Max)
            {
                double limitY = plotTop + plotHeight - (TempLimit / lane.Max * plotHeight);
                context.DrawLine(LimitPen, new Point(0, limitY), new Point(width, limitY));
                var limitLabel = Text($"{_labelLimit} {TempLimit:F0}°", ScaleLabelSize, SolidColorBrush.Parse("#F87171"));
                context.DrawText(limitLabel, new Point(width - limitLabel.Width - 6, limitY - limitLabel.Height - 1));
            }

            // Series
            Point? previous = null;
            for (int i = 0; i < _samples.Count; i++)
            {
                float raw = laneIndex switch
                {
                    0 => _samples[i].ClockMhz,
                    1 => _samples[i].TempC,
                    _ => _samples[i].PowerW,
                };
                double x = XFor(i);
                double y = plotTop + plotHeight - Math.Clamp(raw / lane.Max, 0, 1) * plotHeight;
                var current = new Point(x, y);
                if (previous.HasValue) context.DrawLine(lane.Pen, previous.Value, current);
                previous = current;
            }

            // Lane title plus the live value, in the series colour so the two are tied together.
            float latest = laneIndex switch
            {
                0 => _samples[^1].ClockMhz,
                1 => _samples[^1].TempC,
                _ => _samples[^1].PowerW,
            };
            var title = Text($"{lane.Title}  {latest.ToString(lane.Format)} {lane.Unit}", LabelSize, lane.Brush);
            context.DrawText(title, new Point(6, lane.Top + 2));

            var scale = Text($"{lane.Max.ToString(lane.Format)}", ScaleLabelSize, TextFaint);
            context.DrawText(scale, new Point(width - scale.Width - 6, lane.Top + 2));

            // The mid-scale label sits inside the plot, on the same edge as the newest sample.
            // A backing plate keeps both readable where they cross.
            var midScale = Text($"{(lane.Max / 2).ToString(lane.Format)}", ScaleLabelSize, TextFaint);
            var midOrigin = new Point(width - midScale.Width - 6, midY - midScale.Height / 2);
            context.DrawRectangle(
                BackgroundBrush, null,
                new Rect(midOrigin.X - 3, midOrigin.Y, midScale.Width + 6, midScale.Height));
            context.DrawText(midScale, midOrigin);
        }

        DrawTimeAxis(context, width, height);

        if (_hoverX.HasValue) DrawHover(context, width, height, stepX, lanes);
    }

    /// <summary>
    /// Sampling interval measured from the timestamps rather than assumed. The old tooltip
    /// hard-coded "two seconds per sample", which silently lies the moment the poll rate moves.
    /// </summary>
    private TimeSpan SampleInterval => _samples.Count > 1
        ? TimeSpan.FromSeconds((_samples[^1].Timestamp - _samples[0].Timestamp).TotalSeconds / (_samples.Count - 1))
        : TimeSpan.FromSeconds(2);

    /// <summary>The X axis had no labels at all; a curve with no time reference says very little.</summary>
    private void DrawTimeAxis(DrawingContext context, double width, double height)
    {
        double y = height - AxisStripHeight + 1;
        var window = SampleInterval * (MaxSamples - 1);

        var marks = new (double X, string Label)[]
        {
            (0, Ago(window)),
            (width / 2, Ago(window / 2)),
            (width, _labelNow),
        };

        foreach (var (x, label) in marks)
        {
            var text = Text(label, ScaleLabelSize, TextFaint);
            double tx = Math.Clamp(x - text.Width / 2, 4, Math.Max(4, width - text.Width - 4));
            context.DrawText(text, new Point(tx, y));
        }
    }

    private static string Ago(TimeSpan span) =>
        span.TotalSeconds < 1 ? "0s"
        : span.TotalMinutes >= 1 ? $"-{span.TotalMinutes:F0}min"
        : $"-{span.TotalSeconds:F0}s";

    private void DrawHover(DrawingContext context, double width, double height, double stepX, Lane[] lanes)
    {
        int hoverIndex = Math.Clamp(
            _samples.Count - 1 - (int)Math.Round((width - _hoverX!.Value) / stepX),
            0, _samples.Count - 1);
        var sample = _samples[hoverIndex];
        double x = width - (_samples.Count - 1 - hoverIndex) * stepX;

        context.DrawLine(CrosshairPen, new Point(x, 0), new Point(x, height - AxisStripHeight));

        for (int laneIndex = 0; laneIndex < lanes.Length; laneIndex++)
        {
            var lane = lanes[laneIndex];
            double plotTop = lane.Top + LaneHeaderHeight;
            double plotHeight = Math.Max(4, lane.Height - LaneHeaderHeight - LanePadding);
            float raw = laneIndex switch
            {
                0 => sample.ClockMhz,
                1 => sample.TempC,
                _ => sample.PowerW,
            };
            double y = plotTop + plotHeight - Math.Clamp(raw / lane.Max, 0, 1) * plotHeight;
            context.DrawEllipse(lane.Brush, null, new Point(x, y), 2.5, 2.5);
        }

        var age = _samples[^1].Timestamp - sample.Timestamp;
        string when = age.TotalSeconds < 1 ? _labelNow : Ago(age);
        var label = Text(
            $"{when}   {sample.ClockMhz:F0} MHz   {sample.TempC:F0} °C   {sample.PowerW:F0} W   ({sample.Timestamp:HH:mm:ss})",
            10, TextBright);

        double boxWidth = label.Width + 14;
        double boxX = Math.Clamp(x + 10, 4, Math.Max(4, width - boxWidth - 4));
        context.DrawRectangle(TooltipBg, new Pen(SolidColorBrush.Parse("#38BDF8"), 1),
            new Rect(boxX, 4, boxWidth, label.Height + 6), 4, 4);
        context.DrawText(label, new Point(boxX + 7, 7));
    }
}
