//! How deep a Curve Optimizer value this machine accepts (invariant I14).
//!
//! The range is no longer a constant. It is the standard as a floor, widened by what the processor itself
//! demonstrates (a value it reports holding) and by what the person asks for, and narrowed again when a
//! write turns out not to stick. Vectors: `spec/vectors/curve-optimizer-range.json`.

/// Accepted by every Ryzen that has the feature. The default, and the safe one.
pub const STANDARD: i32 = -30;

/// The deepest value anything is known to claim. A bound on typing, not a promise.
pub const WIDEST: i32 = -50;

/// The depth to search to.
///
/// * `model_limit`: what this processor family is known to start out accepting.
/// * `user_limit`: the deepest value the person wants searched, or `None` for the processor's own default.
///   It works in both directions within the standard and the widest. Nothing shallower than the standard is
///   honoured: there is no reason to refuse a value every part takes.
/// * `read_from_hardware`: values the processor reported for its cores. A value it holds is a value it
///   accepts, evidence no setting can outweigh.
/// * `applied_floor`: the value the processor actually applied when a deeper one was requested and not kept.
///   It pulls the range back: asking for -45 and getting -40 means the limit is -40.
pub fn effective(
    model_limit: i32,
    user_limit: Option<i32>,
    read_from_hardware: &[i32],
    applied_floor: Option<i32>,
) -> i32 {
    let mut limit = model_limit.clamp(WIDEST, STANDARD);

    if let Some(asked) = user_limit {
        limit = asked.clamp(WIDEST, STANDARD);
    }

    for &held in read_from_hardware {
        if held < limit {
            limit = held.max(WIDEST);
        }
    }

    // A write that did not stick is the processor saying where its limit is.
    if let Some(applied) = applied_floor {
        if applied > limit && applied <= STANDARD {
            limit = applied;
        }
    }

    limit
}

/// Whether a write beyond the standard range was kept. Compared only there: inside it, the write path is
/// verified on hardware and a mismatch more likely means a read that raced the write than a refused value.
pub fn was_kept(requested: i32, read_back: Option<i32>) -> bool {
    requested >= STANDARD || read_back.is_none() || read_back == Some(requested)
}
