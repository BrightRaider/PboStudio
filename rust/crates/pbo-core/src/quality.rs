//! Which cores the processor itself rates highest (CPPC "highest performance": from the processor, or as Windows ranks
//! the cores from the same firmware values).
//!
//! This used to be a hard-coded guess (core 0 is gold, core 8 or 1 is silver) shown as if it had been measured; on
//! plenty of Ryzen parts the preferred core is neither. When the values are unavailable there is no ranking at all
//! rather than an invented one. The ranking feeds the extra headroom the tuner keeps on the cores that boost furthest
//! (`tuner::PREFERRED_CORE_EXTRA_GUARDBAND`). Vectors: `spec/vectors/core-quality.json`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    Standard,
    /// The highest performance value the processor reports.
    GoldStar,
    /// The second highest.
    SilverStar,
}

impl Rank {
    pub fn name(self) -> &'static str {
        match self {
            Rank::Standard => "Standard",
            Rank::GoldStar => "GoldStar",
            Rank::SilverStar => "SilverStar",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quality {
    pub core: usize,
    pub rank: Rank,
    /// The raw value the core reports, or 0 when unknown.
    pub cppc: i32,
}

pub fn core_qualities(core_count: usize, cppc: Option<&[i32]>) -> Vec<Quality> {
    let mut result: Vec<Quality> = (0..core_count)
        .map(|core| Quality {
            core,
            rank: Rank::Standard,
            cppc: 0,
        })
        .collect();
    let Some(cppc) = cppc else {
        return result;
    };

    // A zero means the MSR read for that core failed; it is not a low score.
    let scored: Vec<(usize, i32)> = (0..core_count)
        .filter(|i| cppc.get(*i).is_some_and(|v| *v > 0))
        .map(|i| (i, cppc[i]))
        .collect();
    for (core, score) in &scored {
        result[*core].cppc = *score;
    }

    // Nothing to rank: too few readings, or firmware reporting one value for every core.
    if scored.len() < 2 {
        return result;
    }
    let mut distinct: Vec<i32> = scored.iter().map(|s| s.1).collect();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() < 2 {
        return result;
    }

    // Ties go to the lower core index, matching how Windows breaks them.
    let mut ordered = scored;
    ordered.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    result[ordered[0].0].rank = Rank::GoldStar;
    result[ordered[1].0].rank = Rank::SilverStar;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_readings_is_no_ranking() {
        assert!(core_qualities(4, None)
            .iter()
            .all(|q| q.rank == Rank::Standard && q.cppc == 0));
    }

    #[test]
    fn the_two_highest_are_gold_and_silver_with_ties_to_the_lower_core() {
        let q = core_qualities(4, Some(&[200, 230, 230, 210]));
        assert_eq!(q[1].rank, Rank::GoldStar);
        assert_eq!(q[2].rank, Rank::SilverStar);
        assert_eq!(q[0].rank, Rank::Standard);
    }

    #[test]
    fn a_failed_read_is_not_a_low_score_and_one_value_for_all_ranks_nobody() {
        let q = core_qualities(3, Some(&[0, 200, 200]));
        assert!(q.iter().all(|q| q.rank == Rank::Standard));
        assert_eq!(q[0].cppc, 0);
        assert_eq!(q[1].cppc, 200);
    }
}
