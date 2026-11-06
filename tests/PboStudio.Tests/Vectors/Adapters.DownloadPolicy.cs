using System.Text.Json;
using PboStudio.Core;

namespace PboStudio.Tests.Vectors;

internal static class DownloadPolicyAdapters
{
    [Vector("download-policy/isAllowed")]
    public static object? IsAllowed(JsonElement i) => DownloadPolicy.IsAllowed(i.Str("url"), i.Strs("allowedHosts"));

    [Vector("download-policy/registrableRoot")]
    public static object? RegistrableRoot(JsonElement i) => DownloadPolicy.RegistrableRoot(i.Str("host"));

    [Vector("download-policy/pinnedInstallerSha256")]
    public static object? Pinned(JsonElement i) => DownloadPolicy.PawnIoInstallerSha256;
}
