using System.Security.Cryptography;

namespace PboStudio.Core;

/// <summary>
/// What this program will fetch and run, and from where.
///
/// It runs as administrator and starts what it downloads: two stress engines that execute with
/// those rights for hours, and the installer of a <em>kernel driver</em>. Three gaps in how that
/// was done:
///
///  - A plain-http address stood in the y-cruncher mirror list, and the page it scans for the
///    current version was fetched over http too. Anything on the path could substitute the archive
///    and it would have been run elevated.
///  - A link found on that page was followed whatever host it pointed at.
///  - The PawnIO installer was written under a fixed name in the temp folder and then started with
///    elevation, with nothing checking that the bytes were the ones that were meant.
///
/// The installer has a known-good SHA-256, which GitHub publishes for the release asset and which
/// matches the copy shipped in the offline bundle byte for byte. The engines are version-pinned
/// files on vendor servers that change; for those the line drawn is https, and the vendor's own
/// domain.
/// </summary>
public static class DownloadPolicy
{
    /// <summary>PawnIO_setup.exe 2.2.0, as GitHub lists it for the release asset.</summary>
    public const string PawnIoInstallerSha256 =
        "1f519a22e47187f70a1379a48ca604981c4fcf694f4e65b734aaa74a9fba3032";

    /// <summary>
    /// Whether to fetch <paramref name="url"/> for <paramref name="allowedHosts"/>: https only, and
    /// only on a domain the vendor already uses. A link discovered on a page is not trusted because
    /// the page named it.
    /// </summary>
    public static bool IsAllowed(string url, IEnumerable<string> allowedHosts)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri)) return false;
        if (uri.Scheme != Uri.UriSchemeHttps) return false;

        string root = RegistrableRoot(uri.Host);
        return allowedHosts.Any(h => string.Equals(RegistrableRoot(h), root, StringComparison.OrdinalIgnoreCase));
    }

    /// <summary>
    /// The last two labels of a host: "cdn.numberworld.org" and "www.numberworld.org" are the same
    /// owner, "numberworld.org.evil.com" and "evilnumberworld.org" are not. Good enough for the few
    /// vendor domains involved; it is not a general public-suffix lookup and does not pretend to be.
    /// </summary>
    public static string RegistrableRoot(string host)
    {
        var labels = host.Trim('.').ToLowerInvariant().Split('.', StringSplitOptions.RemoveEmptyEntries);
        return labels.Length <= 2 ? string.Join('.', labels) : string.Join('.', labels[^2..]);
    }

    /// <summary>Whether the file at <paramref name="path"/> hashes to <paramref name="expectedSha256"/>.</summary>
    public static bool Sha256Matches(string path, string expectedSha256)
    {
        try
        {
            using var stream = File.OpenRead(path);
            string actual = Convert.ToHexString(SHA256.HashData(stream));
            return string.Equals(actual, expectedSha256, StringComparison.OrdinalIgnoreCase);
        }
        catch
        {
            return false;
        }
    }
}
