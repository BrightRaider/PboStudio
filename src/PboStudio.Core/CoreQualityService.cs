namespace PboStudio.Core;

public enum CoreQualityRank
{
    Standard,
    GoldStar,   // Best preferred core (#1)
    SilverStar  // Second best preferred core (#2)
}

public sealed record CoreQualityInfo(int CoreIndex, CoreQualityRank Rank, int CppcPriority);

/// <summary>
/// CPPC (Collaborative Processor Performance Control) core quality service.
/// Identifies AMD Ryzen preferred cores (Gold & Silver Star cores) which boost highest.
/// </summary>
public static class CoreQualityService
{
    public static Dictionary<int, CoreQualityInfo> GetCoreQualities(int physicalCoreCount)
    {
        var result = new Dictionary<int, CoreQualityInfo>();

        // Ryzen topology default heuristic: Core 0 is preferred #1, Core 1 or 4 is preferred #2
        // On single-CCD CPUs (8-core), Core 0 is Gold, Core 1 is Silver.
        // On dual-CCD CPUs (12/16-core), Core 0 is Gold, Core 8 is Silver.

        for (int i = 0; i < physicalCoreCount; i++)
        {
            CoreQualityRank rank = CoreQualityRank.Standard;
            int priority = i + 1;

            if (i == 0)
            {
                rank = CoreQualityRank.GoldStar;
                priority = 1;
            }
            else if (physicalCoreCount > 8 && i == 8)
            {
                rank = CoreQualityRank.SilverStar;
                priority = 2;
            }
            else if (physicalCoreCount <= 8 && i == 1)
            {
                rank = CoreQualityRank.SilverStar;
                priority = 2;
            }

            result[i] = new CoreQualityInfo(i, rank, priority);
        }

        return result;
    }
}
