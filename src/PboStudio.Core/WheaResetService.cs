namespace PboStudio.Core;

public static class WheaResetService
{
    public static string GetResetFilePath(string workRoot) =>
        Path.Combine(workRoot, "whea_reset.txt");

    public static DateTime? GetResetTimestamp(string workRoot)
    {
        string path = GetResetFilePath(workRoot);
        if (!File.Exists(path)) return null;

        try
        {
            string text = File.ReadAllText(path).Trim();
            if (DateTime.TryParse(text, out DateTime dt))
                return dt;
        }
        catch { }
        return null;
    }

    public static void SetResetTimestamp(string workRoot, DateTime timestamp)
    {
        try
        {
            Directory.CreateDirectory(workRoot);
            File.WriteAllText(GetResetFilePath(workRoot), timestamp.ToString("o"));
        }
        catch { }
    }

    public static void ClearResetTimestamp(string workRoot)
    {
        try
        {
            string path = GetResetFilePath(workRoot);
            if (File.Exists(path)) File.Delete(path);
        }
        catch { }
    }
}
