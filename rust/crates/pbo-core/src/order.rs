//! The order the cores are tested in.
//!
//! One after the other (0, 1, 2, ...) lets the heat of one core soak into the next one's test. Jumping between the two
//! halves (0, 4, 1, 5, ...) keeps neighbouring cores from running back to back, and a shuffle takes away any
//! pattern at all. The first is what a plan means when it says nothing; the profiles of 1.x mostly alternate.
//! Vectors: `spec/vectors/core-order.json`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoreOrder {
    /// In the order of the numbers.
    #[default]
    Sequential,
    /// From the first half and from the second half in turn, so neighbours are never tested back to back.
    Alternate,
    /// Shuffled, differently every time.
    Random,
    /// Every ordered pair of cores, one after the other: the load is handed from each core to each other core. Some
    /// instabilities only show on that handover, and no order that visits every core once can produce it. It takes
    /// n times (n - 1) times two tests, so it is for small plans.
    Pairs,
}

impl CoreOrder {
    pub const ALL: [CoreOrder; 4] = [
        CoreOrder::Sequential,
        CoreOrder::Alternate,
        CoreOrder::Random,
        CoreOrder::Pairs,
    ];

    /// The name in files and on the command line.
    pub fn id(self) -> &'static str {
        match self {
            CoreOrder::Sequential => "sequential",
            CoreOrder::Alternate => "alternate",
            CoreOrder::Random => "random",
            CoreOrder::Pairs => "pairs",
        }
    }

    pub fn from_id(id: &str) -> Option<CoreOrder> {
        CoreOrder::ALL
            .into_iter()
            .find(|o| o.id().eq_ignore_ascii_case(id.trim()))
    }
}

impl From<crate::profiles::Order> for CoreOrder {
    fn from(order: crate::profiles::Order) -> CoreOrder {
        match order {
            crate::profiles::Order::Sequential => CoreOrder::Sequential,
            crate::profiles::Order::Alternate => CoreOrder::Alternate,
        }
    }
}

/// `cores` in the given order. `seed` only matters for `Random`: the same seed gives the same shuffle.
pub fn arrange(order: CoreOrder, cores: &[usize], seed: u64) -> Vec<usize> {
    match order {
        CoreOrder::Sequential => cores.to_vec(),
        CoreOrder::Alternate => {
            // The first half and the second half in turn. With an odd count the first half is the longer one.
            let half = cores.len().div_ceil(2);
            let mut out = Vec::with_capacity(cores.len());
            for i in 0..half {
                out.push(cores[i]);
                if i + half < cores.len() {
                    out.push(cores[i + half]);
                }
            }
            out
        }
        CoreOrder::Pairs => {
            let mut out = Vec::new();
            for a in cores {
                for b in cores {
                    if a != b {
                        out.push(*a);
                        out.push(*b);
                    }
                }
            }
            // One core has no other to hand over to: it is just tested.
            if out.is_empty() {
                cores.to_vec()
            } else {
                out
            }
        }
        CoreOrder::Random => {
            let mut out = cores.to_vec();
            let mut state = seed | 1;
            // Fisher-Yates with a xorshift: no dependency for a shuffle.
            for i in (1..out.len()).rev() {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let j = (state % (i as u64 + 1)) as usize;
                out.swap(i, j);
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_keeps_the_order_it_was_given() {
        assert_eq!(arrange(CoreOrder::Sequential, &[2, 5, 7], 1), vec![2, 5, 7]);
    }

    #[test]
    fn alternating_jumps_between_the_halves() {
        let eight: Vec<usize> = (0..8).collect();
        assert_eq!(
            arrange(CoreOrder::Alternate, &eight, 0),
            vec![0, 4, 1, 5, 2, 6, 3, 7]
        );
        let five: Vec<usize> = (0..5).collect();
        assert_eq!(arrange(CoreOrder::Alternate, &five, 0), vec![0, 3, 1, 4, 2]);
        assert_eq!(arrange(CoreOrder::Alternate, &[3], 0), vec![3]);
        assert!(arrange(CoreOrder::Alternate, &[], 0).is_empty());
    }

    #[test]
    fn alternating_works_on_the_cores_that_were_chosen_not_on_their_numbers() {
        assert_eq!(
            arrange(CoreOrder::Alternate, &[1, 3, 4, 6], 0),
            vec![1, 4, 3, 6]
        );
    }

    #[test]
    fn pairs_hand_the_load_from_every_core_to_every_other() {
        assert_eq!(
            arrange(CoreOrder::Pairs, &[0, 1, 2], 0),
            vec![0, 1, 0, 2, 1, 0, 1, 2, 2, 0, 2, 1]
        );
        assert_eq!(arrange(CoreOrder::Pairs, &[4], 0), vec![4]);
        assert!(arrange(CoreOrder::Pairs, &[], 0).is_empty());
        let eight: Vec<usize> = (0..8).collect();
        assert_eq!(arrange(CoreOrder::Pairs, &eight, 0).len(), 8 * 7 * 2);
    }

    #[test]
    fn a_shuffle_is_a_permutation_and_the_same_seed_gives_the_same_one() {
        let cores: Vec<usize> = (0..16).collect();
        for seed in [1, 2, 99, 12345, u64::MAX] {
            let a = arrange(CoreOrder::Random, &cores, seed);
            let mut sorted = a.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, cores, "seed {seed}");
            assert_eq!(a, arrange(CoreOrder::Random, &cores, seed));
        }
        assert_ne!(
            arrange(CoreOrder::Random, &cores, 1),
            arrange(CoreOrder::Random, &cores, 2)
        );
    }

    #[test]
    fn the_names_round_trip() {
        for o in CoreOrder::ALL {
            assert_eq!(CoreOrder::from_id(o.id()), Some(o));
        }
        assert_eq!(
            CoreOrder::from_id(" Alternate "),
            Some(CoreOrder::Alternate)
        );
        assert_eq!(CoreOrder::from_id("sideways"), None);
        assert_eq!(CoreOrder::default(), CoreOrder::Sequential);
    }
}
