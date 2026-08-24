using System.Net.Http.Json;
using System.Text;

namespace PboStudio.Core;

/// <summary>
/// Sends optional Discord webhook notifications when tests start, fail, or complete.
/// </summary>
public static class NotificationService
{
    private static readonly HttpClient HttpClient = new();

    public static async Task<bool> SendWebhookAsync(string webhookUrl, string title, string description, int colorHex = 0x2D6A4F)
    {
        if (string.IsNullOrWhiteSpace(webhookUrl) || !Uri.TryCreate(webhookUrl, UriKind.Absolute, out _))
            return false;

        try
        {
            var payload = new
            {
                embeds = new[]
                {
                    new
                    {
                        title = title,
                        description = description,
                        color = colorHex,
                        timestamp = DateTime.UtcNow.ToString("o"),
                        footer = new { text = "PboStudio CoreCycler" }
                    }
                }
            };

            var response = await HttpClient.PostAsJsonAsync(webhookUrl, payload);
            return response.IsSuccessStatusCode;
        }
        catch
        {
            return false;
        }
    }

    public static Task<bool> SendRunStartedAsync(string webhookUrl, string cpuName, int coreCount, TimeSpan runtimePerCore) =>
        SendWebhookAsync(
            webhookUrl,
            "🚀 PboStudio: Test gestartet",
            $"**CPU:** {cpuName}\n**Kerne:** {coreCount} ausgewählt\n**Dauer pro Kern:** {runtimePerCore.TotalMinutes:F0} Min.",
            0x3498DB // Blue
        );

    public static Task<bool> SendCoreFailedAsync(string webhookUrl, int coreIndex, string failureReason, int currentMargin, int suggestedMargin) =>
        SendWebhookAsync(
            webhookUrl,
            $"⚠️ PboStudio: Fehler auf Kern {coreIndex}",
            $"**Grund:** {failureReason}\n**Aktueller Wert:** {currentMargin}\n**Empfohlen:** {suggestedMargin}",
            0xE74C3C // Red
        );

    public static Task<bool> SendRunFinishedAsync(string webhookUrl, int failedCores, TimeSpan duration)
    {
        if (failedCores == 0)
        {
            return SendWebhookAsync(
                webhookUrl,
                "✅ PboStudio: Test erfolgreich beendet",
                $"Alle Kerne haben den Test bestanden!\n**Gesamtdauer:** {duration:h\\:mm\\:ss}",
                0x2ECC71 // Green
            );
        }

        return SendWebhookAsync(
            webhookUrl,
            $"❌ PboStudio: Test beendet ({failedCores} Fehler)",
            $"{failedCores} Kern(e) sind durchgefallen. Empfehlungen in der App beachten.\n**Gesamtdauer:** {duration:h\\:mm\\:ss}",
            0xE74C3C // Red
        );
    }
}
