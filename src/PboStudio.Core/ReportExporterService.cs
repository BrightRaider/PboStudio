using System.Text;

namespace PboStudio.Core;

public static class ReportExporterService
{
    public static string GenerateHtmlReport(
        string cpuName,
        IReadOnlyDictionary<int, int> coreMargins,
        IReadOnlyList<WheaEvent> wheaEvents,
        string testedProfile,
        TimeSpan totalDuration,
        int failureCount)
    {
        var sb = new StringBuilder();
        sb.AppendLine("<!DOCTYPE html>");
        sb.AppendLine("<html lang=\"de\">");
        sb.AppendLine("<head>");
        sb.AppendLine("  <meta charset=\"UTF-8\">");
        sb.AppendLine($" <title>PboStudio Stabilitäts-Report - {cpuName}</title>");
        sb.AppendLine("  <style>");
        sb.AppendLine("    body { font-family: 'Segoe UI', Tahoma, sans-serif; background: #0B0D12; color: #E2E8F0; padding: 24px; }");
        sb.AppendLine("    .card { background: #141720; border: 1px solid #222838; border-radius: 8px; padding: 20px; margin-bottom: 16px; }");
        sb.AppendLine("    h1 { color: #10B981; margin-top: 0; }");
        sb.AppendLine("    .badge { display: inline-block; padding: 4px 10px; border-radius: 12px; font-weight: bold; font-size: 12px; }");
        sb.AppendLine("    .pass { background: #064E3B; color: #34D399; }");
        sb.AppendLine("    .fail { background: #7F1D1D; color: #F87171; }");
        sb.AppendLine("    table { width: 100%; border-collapse: collapse; margin-top: 12px; }");
        sb.AppendLine("    th, td { text-align: left; padding: 8px 12px; border-bottom: 1px solid #222838; }");
        sb.AppendLine("    th { background: #1A1E2B; color: #94A3B8; }");
        sb.AppendLine("  </style>");
        sb.AppendLine("</head>");
        sb.AppendLine("<body>");

        sb.AppendLine("  <div class=\"card\">");
        sb.AppendLine("    <h1>⚡ PboStudio Curve Optimizer Stabilitäts-Zertifikat</h1>");
        sb.AppendLine($"   <p><strong>Prozessor:</strong> {cpuName}</p>");
        sb.AppendLine($"   <p><strong>Test-Profil:</strong> {testedProfile}</p>");
        sb.AppendLine($"   <p><strong>Gesamtdauer:</strong> {totalDuration:h\\:mm\\:ss} Std</p>");
        sb.AppendLine($"   <p><strong>Ergebnis:</strong> {(failureCount == 0 ? "<span class=\"badge pass\">BESTANDEN (STABIL)</span>" : $"<span class=\"badge fail\">{failureCount} FEHLER ERKANNT</span>")}</p>");
        sb.AppendLine($"   <p><small>Erstellt am: {DateTime.Now:dd.MM.yyyy HH:mm:ss} mit PboStudio v2.0</small></p>");
        sb.AppendLine("  </div>");

        sb.AppendLine("  <div class=\"card\">");
        sb.AppendLine("    <h2>🎯 Per-Core Curve Optimizer Konfiguration</h2>");
        sb.AppendLine("    <table>");
        sb.AppendLine("      <thead><tr><th>Kern</th><th>Curve Optimizer Margin</th><th>Status</th></tr></thead>");
        sb.AppendLine("      <tbody>");
        foreach (var (core, margin) in coreMargins.OrderBy(k => k.Key))
        {
            sb.AppendLine($"        <tr><td>Kern {core}</td><td><strong>{margin}</strong></td><td><span class=\"badge pass\">Aktiv</span></td></tr>");
        }
        sb.AppendLine("      </tbody>");
        sb.AppendLine("    </table>");
        sb.AppendLine("  </div>");

        if (wheaEvents.Count > 0)
        {
            sb.AppendLine("  <div class=\"card\">");
            sb.AppendLine($"   <h2>⚠️ Erfasste WHEA Hardware-Fehler ({wheaEvents.Count})</h2>");
            sb.AppendLine("    <ul>");
            foreach (var evt in wheaEvents.Take(10))
            {
                sb.AppendLine($"     <li>{evt.Time:dd.MM.yyyy HH:mm} - APIC ID {evt.ApicId}: WHEA Event {evt.EventId}</li>");
            }
            sb.AppendLine("    </ul>");
            sb.AppendLine("  </div>");
        }

        sb.AppendLine("</body>");
        sb.AppendLine("</html>");

        return sb.ToString();
    }

    public static string SaveReport(
        string workRoot,
        string cpuName,
        IReadOnlyDictionary<int, int> coreMargins,
        IReadOnlyList<WheaEvent> wheaEvents,
        string testedProfile,
        TimeSpan totalDuration,
        int failureCount)
    {
        string html = GenerateHtmlReport(cpuName, coreMargins, wheaEvents, testedProfile, totalDuration, failureCount);
        string reportPath = Path.Combine(workRoot, $"PboStudio_Report_{DateTime.Now:yyyyMMdd_HHmmss}.html");
        Directory.CreateDirectory(workRoot);
        File.WriteAllText(reportPath, html, Encoding.UTF8);
        return reportPath;
    }
}
