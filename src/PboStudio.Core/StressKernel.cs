using System.Numerics;

namespace PboStudio.Core;

/// <summary>
/// The built-in load. Applies a Givens rotation to an L1-resident pair of vectors, over and
/// over. Two properties make it useful for hunting Curve Optimizer instability:
///
///   - It is norm-preserving, so values never drift toward overflow or denormals no matter
///     how long it runs. The arithmetic stays dense multiply-add work at full AVX width,
///     which is what pushes current draw and exposes an undervolted core.
///   - It is bit-deterministic. The same block of passes from the same starting state must
///     produce the same checksum every time. A single flipped bit anywhere in the pipeline
///     changes it, which is how a silent miscalculation gets caught rather than being
///     mistaken for a stable run.
/// </summary>
public sealed class StressKernel
{
    // Sized so both buffers together stay inside a 32 KB L1 data cache.
    private const int Length = 1024;

    private const double C = 0.8;
    private const double S = 0.6;

    private readonly double[] _x = new double[Length];
    private readonly double[] _y = new double[Length];

    public StressKernel() => Reset();

    /// <summary>Restores the deterministic starting state.</summary>
    public void Reset()
    {
        for (int i = 0; i < Length; i++)
        {
            _x[i] = 1.0 + i * (1.0 / Length);
            _y[i] = 2.0 - i * (1.0 / Length);
        }
    }

    /// <summary>
    /// Runs <paramref name="passes"/> rotations from the current state and returns a checksum
    /// of the result. Always call <see cref="Reset"/> first if the checksum is to be compared
    /// against a reference.
    /// </summary>
    public ulong RunBlock(int passes)
    {
        int width = Vector<double>.Count;
        var vc = new Vector<double>(C);
        var vs = new Vector<double>(S);

        var x = _x.AsSpan();
        var y = _y.AsSpan();

        for (int p = 0; p < passes; p++)
        {
            for (int i = 0; i <= Length - width; i += width)
            {
                var vx = new Vector<double>(x.Slice(i, width));
                var vy = new Vector<double>(y.Slice(i, width));

                (vx * vc - vy * vs).CopyTo(x.Slice(i, width));
                (vx * vs + vy * vc).CopyTo(y.Slice(i, width));
            }
        }

        return Checksum();
    }

    private ulong Checksum()
    {
        ulong h = 1469598103934665603UL;
        for (int i = 0; i < Length; i++)
        {
            h = (h ^ (ulong)BitConverter.DoubleToInt64Bits(_x[i])) * 1099511628211UL;
            h = (h ^ (ulong)BitConverter.DoubleToInt64Bits(_y[i])) * 1099511628211UL;
        }
        return h;
    }
}
