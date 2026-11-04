namespace PboStudio.Core;

/// <summary>
/// How deep a Curve Optimizer value this particular machine accepts.
///
/// 1.1.0 said it was -30 on every AMD desktop part. That was stated as fact and not sourced, and
/// a report from a Ryzen 9 7900X3D / 7950X3D owner contradicts it: -40 entered in the BIOS and
/// used, and -30 as the stopping point of this program. There is no AMD document that settles it
/// either way. The vendored ZenStates library says the range is -30 to 30 before Zen 4 and -50 to
/// 50 on Zen 4 and newer, in a comment on the commands this program sends - the best source
/// available, not a specification, and not yet checked on a Zen 4 or Zen 5 part here.
///
/// So the range is no longer a constant. It is what the standard says as a floor, widened by
/// what the processor itself demonstrates: a value it reports having, a value the person
/// explicitly asks for, and narrowed again if a write turns out not to stick.
/// </summary>
public static class CurveOptimizerRange
{
    /// <summary>Accepted by every Ryzen that has the feature. The default, and the safe one.</summary>
    public const int Standard = -30;

    /// <summary>
    /// The deepest value anything is known to claim. A bound on typing, not a promise: nothing
    /// deeper has ever been reported, and a value below it is a typo.
    /// </summary>
    public const int Widest = -50;

    /// <param name="modelLimit">What this processor family is known to accept.</param>
    /// <param name="userLimit">
    /// The deepest value the person wants searched, or null for the processor's own default. It works
    /// in both directions within the standard and the widest: deeper than the default for a board
    /// that accepts more, shallower for somebody who wants a Ryzen 7000 or 9000 held at the standard -30.
    /// Nothing shallower than -30 is honoured - there is no reason to refuse a value every part takes.
    /// </param>
    /// <param name="readFromHardware">
    /// Values the processor reported for its cores at start-up. A value it holds is a value it
    /// accepts, which is evidence no setting can outweigh - and without it a core the BIOS set
    /// to -40 is displayed as -30, looks edited, and "Apply live" writes -30 back.
    /// </param>
    /// <param name="appliedFloor">
    /// The value the processor actually applied when a deeper one was requested and not kept, or
    /// null. It pulls the range back: asking for -45 and getting -40 means the limit is -40.
    /// </param>
    public static int Effective(
        int modelLimit,
        int? userLimit,
        IEnumerable<int> readFromHardware,
        int? appliedFloor = null)
    {
        int limit = Math.Max(Widest, Math.Min(Standard, modelLimit));

        if (userLimit is { } asked)
            limit = Math.Clamp(asked, Widest, Standard);

        foreach (int held in readFromHardware)
            if (held < limit)
                limit = Math.Max(Widest, held);

        // A write that did not stick is the processor saying where its limit is.
        if (appliedFloor is { } applied && applied > limit && applied <= Standard)
            limit = applied;

        return limit;
    }

    /// <summary>
    /// Whether a write beyond the standard range was kept. Compared only there: inside it, the
    /// write path is verified on hardware and a mismatch more likely means a read that raced the
    /// write than a refused value.
    /// </summary>
    public static bool WasKept(int requested, int? readBack) =>
        requested >= Standard || readBack is null || readBack == requested;
}
