namespace PboStudio.Core;

/// <param name="Margins">What may be written: only cores that exist, only values in range.</param>
/// <param name="Problems">What was left out and why, for the log. Empty when everything was usable.</param>
public sealed record SavedProfileCheck(
    IReadOnlyDictionary<int, int> Margins,
    IReadOnlyList<string> Problems,
    bool WrongProcessor);

/// <summary>
/// Decides what of a saved profile it is safe to write back at Windows start.
///
/// The autostart task wrote the saved values into the processor at every logon, unexamined. Two
/// ways that goes wrong without anybody noticing:
///
///  - The profile belongs to another processor. Swapping a 5800X3D for a 7800X3D leaves eight cores
///    and an eight-entry file, and every old value is applied to a chip it was never tested on.
///  - A value is outside what any processor accepts - a damaged file, or a figure the processor
///    clamps. It was sent to the SMU as it stood.
/// </summary>
public static class SavedProfileValidator
{
    public static SavedProfileCheck Check(
        SavedCoProfile profile, string currentCpuName, int coreCount, int deepestAllowed, int shallowestAllowed = 30)
    {
        var problems = new List<string>();

        if (!string.Equals(profile.CpuName?.Trim(), currentCpuName?.Trim(), StringComparison.OrdinalIgnoreCase))
        {
            problems.Add($"saved for \"{profile.CpuName}\", this is \"{currentCpuName}\"");
            return new SavedProfileCheck(new Dictionary<int, int>(), problems, WrongProcessor: true);
        }

        var usable = new Dictionary<int, int>();
        foreach (var (core, value) in profile.CoreMargins.OrderBy(kv => kv.Key))
        {
            if (core < 0 || core >= coreCount)
                problems.Add($"core {core} does not exist on this processor");
            else if (value < deepestAllowed || value > shallowestAllowed)
                problems.Add($"core {core}: {value} is outside {deepestAllowed}..{shallowestAllowed}");
            else
                usable[core] = value;
        }

        return new SavedProfileCheck(usable, problems, WrongProcessor: false);
    }
}
