using System.Text.Json;

namespace PboStudio.Core;

/// <summary>What the campaign should do next.</summary>
public enum CampaignVerdict
{
    /// <summary>Nothing can run: the driver or an engine is missing.</summary>
    NeedsSetup,

    /// <summary>Start the run described by <see cref="CampaignDecision.Step"/>.</summary>
    Run,

    /// <summary>Every core sits at a value it has proven under every load type. Nothing left.</summary>
    Done,

    /// <summary>
    /// The same run would be started again with the same cores at the same values. Something is
    /// wrong — a cancelled run, or passes that ran out — and repeating it forever is worse than
    /// stopping and saying so.
    /// </summary>
    Stalled,
}

/// <param name="Step">The run to start. Null unless <see cref="Verdict"/> is Run.</param>
/// <param name="PhaseNumber">1, 2 or 3, for "phase 2 of 3".</param>
public sealed record CampaignDecision(
    CampaignVerdict Verdict,
    NextStep? Step = null,
    int PhaseNumber = 0);

/// <summary>
/// A campaign in progress, as written to disk. It has to survive a reboot, because the phase
/// that searches for each core's limit is the phase whose job is to find the value that takes
/// the machine down.
/// </summary>
public sealed record CampaignState
{
    public bool Active { get; init; }
    public bool Finished { get; init; }
    public DateTime Started { get; init; } = DateTime.Now;
    public DateTime? FinishedAt { get; init; }

    /// <summary>How many runs the campaign has completed, for the progress line.</summary>
    public int RunsDone { get; init; }

    /// <summary>The stage of the run that just finished, as a <see cref="TuningStage"/> name.</summary>
    public string LastStage { get; init; } = "";

    /// <summary>Whether that run produced no failures. The campaign ends on a clean Confirm.</summary>
    public bool LastRunClean { get; init; } = true;

    /// <summary>
    /// Cores and their held values at the start of the last run, as "0:-30,1:-25". A run that
    /// leaves this unchanged has learned nothing, and the next one would learn nothing either.
    /// </summary>
    public string LastSignature { get; init; } = "";

    /// <summary>The values the confirmation run proved. Written only when the campaign finishes.</summary>
    public Dictionary<int, int> FinalMargins { get; init; } = [];

    public bool InProgress => Active && !Finished;
}

/// <summary>
/// One button that runs the whole thing.
///
/// Tuning a Curve Optimizer is three runs, not one: establish what the current values do,
/// search each core that still has room, then prove the result under load types the search
/// never used. Every one of those is hours long, and until now the program asked the reader to
/// come back between them, read a card, and press a button again. That is not a tool that
/// tunes a processor; it is a tool that tells you what to do next.
///
/// The campaign chains them. The decision of what to run is the same one
/// <see cref="NextStepService"/> already makes — this only adds when to stop, how to tell that
/// nothing is moving, and the memory to pick up where it left off after the reboot that the
/// middle phase is designed to cause.
/// </summary>
public static class CampaignService
{
    private static readonly JsonSerializerOptions JsonOpts = new() { WriteIndented = true };

    public static string PathFor(string workRoot) => Path.Combine(workRoot, "campaign.json");

    /// <summary>
    /// The cores and values a run is about to work with. Two identical signatures in a row mean
    /// the run changed nothing, which is the only way this loop can fail to terminate: every
    /// failure raises a core's floor and every pass lowers what it holds, so a run that does
    /// neither will do neither again.
    /// </summary>
    public static string Signature(NextStep step, IReadOnlyDictionary<int, int> margins)
    {
        var cores = step.Cores.Count > 0 ? step.Cores : [.. margins.Keys.Order()];
        return $"{step.Stage}|" + string.Join(",", cores.Select(c => $"{c}:{margins.GetValueOrDefault(c)}"));
    }

    /// <param name="step">What <see cref="NextStepService"/> recommends right now.</param>
    /// <param name="margins">What each core currently holds.</param>
    /// <param name="lastRunClean">Whether the run that just finished produced no failures.</param>
    public static CampaignDecision Decide(
        CampaignState state,
        NextStep step,
        IReadOnlyDictionary<int, int> margins,
        bool lastRunClean)
    {
        if (step.Action == NextStepAction.OpenSetup)
            return new CampaignDecision(CampaignVerdict.NeedsSetup);

        // The confirmation run is the last thing a campaign does, and passing it is the entire
        // point of having run the other two. Nothing after this is worth the reader's evening.
        if (state.LastStage == nameof(TuningStage.Confirm) && lastRunClean && state.RunsDone > 0)
            return new CampaignDecision(CampaignVerdict.Done);

        string signature = Signature(step, margins);
        if (state.RunsDone > 0 && signature == state.LastSignature)
            return new CampaignDecision(CampaignVerdict.Stalled);

        return new CampaignDecision(CampaignVerdict.Run, step, PhaseNumberOf(step.Stage));
    }

    /// <summary>Which of the three phases a stage belongs to, for "phase 2 of 3".</summary>
    public static int PhaseNumberOf(TuningStage stage) => stage switch
    {
        TuningStage.Discover => 1,
        TuningStage.Narrow => 2,
        TuningStage.Confirm => 3,
        _ => 0,
    };

    public const int TotalPhases = 3;

    /// <summary>
    /// Where a core should sit after it failed. Its floor already accounts for the failure —
    /// one point safer than the worst value that ever failed — and the guardband puts it
    /// further still, because the load that broke it is not the hardest load it will ever see.
    /// </summary>
    public static int BackOffTo(CoreKnowledge knowledge, int chipLimit, int guardband) =>
        Math.Min(0, knowledge.FloorFor(chipLimit) + Math.Max(0, guardband));

    // ── persistence ───────────────────────────────────────────────

    public static CampaignState Load(string workRoot)
    {
        try
        {
            string path = PathFor(workRoot);
            if (!File.Exists(path)) return new CampaignState();

            return JsonSerializer.Deserialize<CampaignState>(File.ReadAllBytes(path), JsonOpts)
                ?? new CampaignState();
        }
        catch
        {
            // A campaign that cannot be read is a campaign that never started. Never a reason
            // to refuse to open the window.
            return new CampaignState();
        }
    }

    public static void Save(string workRoot, CampaignState state)
    {
        try
        {
            Directory.CreateDirectory(workRoot);
            string path = PathFor(workRoot);
            string temp = path + $".tmp.{Guid.NewGuid():N}";

            File.WriteAllBytes(temp, JsonSerializer.SerializeToUtf8Bytes(state, JsonOpts));
            File.Move(temp, path, overwrite: true);
        }
        catch { }
    }

    public static void Clear(string workRoot)
    {
        try
        {
            string path = PathFor(workRoot);
            if (File.Exists(path)) File.Delete(path);
        }
        catch { }
    }
}
