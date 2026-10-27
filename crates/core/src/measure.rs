//! A measurement of one thing against another, on a machine that may be busy.
//!
//! §233. The tree measures *relative* costs — four times the properties must
//! not cost sixteen times the time — and it does so by timing two things and
//! comparing them. **The comparison is where the measurement breaks**, not
//! the code being measured.
//!
//! The shape: each round measures the two costs, and the **fastest round of
//! each side** is kept; the number is one divided by the other. A round that
//! was preempted says nothing about the code, and the fastest round is the
//! least disturbed one.
//!
//! **Measured, on a loaded machine**: the ratio of the two quietest rounds
//! read **4.75 to 5.39** over ten runs, and the true ratio on an idle machine
//! is **5.0**. The two sides are disturbed *together* — the machine takes the
//! same slice out of both — so the ratio holds.
//!
//! **And what does not work**: the middle of the per-round ratios, which
//! looked more principled and is **worse** — measured, it read 4.13 to 8.88
//! over the same ten runs, because one round preempted on the fast side
//! reads far too low and one preempted on the slow side far too high, and
//! the middle of five such rounds is not a middle of anything. It failed the
//! tree's own listing test **20 times in 40 runs**, where the shape here
//! failed it 6.
//!
//! So: **one home for the shape**, in the lowest layer, because the shape was
//! written three times and each of them was a chance to get it wrong.

use std::time::Instant;

/// How long a thing takes, in seconds.
///
/// The timing is the whole of this function, so that every measurement in the
/// tree is timed the same way.
pub fn seconds(f: &mut dyn FnMut()) -> f64 {
    let start = Instant::now();
    f();
    start.elapsed().as_secs_f64()
}

/// The ratio of two costs, each measured in its **fastest round**.
///
/// Each round measures the two costs and reports them as a pair — the fast
/// one first — and this keeps the **fastest** of each side over the rounds and
/// divides the slower by the faster.
///
/// The rounds should be more than one: with one round there is nothing to be
/// robust against, and a preempted round would be the number.
///
/// A round count of **zero** measures nothing, and the number it returns is
/// [`f64::MAX`]: a caller comparing that against a limit fails, which is the
/// honest direction for a measurement that did not happen.
pub fn quietest_ratio(rounds: usize, round: &mut dyn FnMut() -> (f64, f64)) -> f64 {
    if rounds == 0 {
        return f64::MAX;
    }
    let mut fastest = f64::MAX;
    let mut slowest = f64::MAX;
    for _ in 0..rounds {
        let (fast, slow) = round();
        if fast < fastest {
            fastest = fast;
        }
        if slow < slowest {
            slowest = slow;
        }
    }
    slowest / fastest.max(f64::MIN_POSITIVE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two quietest rounds are the number.
    #[test]
    fn the_fastest_round_of_each_side_is_the_number() {
        let mut round = || (1.0, 4.0);
        assert_eq!(quietest_ratio(3, &mut round), 4.0);
    }

    /// A round that was disturbed does not decide the number.
    ///
    /// One round reading far too high — the machine was busy — and one reading
    /// far too low, and the quietest round of each side is still the ratio of
    /// the quiet rounds.
    #[test]
    fn a_disturbed_round_does_not_decide_the_number() {
        let rounds = [(1.0, 4.0), (1.0, 400.0), (100.0, 4.0), (1.0, 4.0), (1.0, 4.0)];
        let mut index = 0usize;
        let mut round = || {
            let pair = rounds[index];
            index += 1;
            pair
        };
        assert_eq!(quietest_ratio(5, &mut round), 4.0);
    }

    /// A measurement that did not happen is not a measurement.
    #[test]
    fn no_rounds_measure_nothing() {
        let mut round = || (1.0, 1.0);
        assert_eq!(quietest_ratio(0, &mut round), f64::MAX);
    }

    /// The timing is the whole of it.
    #[test]
    fn a_thing_that_takes_a_moment_takes_a_moment() {
        let mut work = || {
            let mut sum = 0u64;
            for i in 0..2_000_000u64 {
                sum = sum.wrapping_add(i);
            }
            std::hint::black_box(sum);
        };
        assert!(seconds(&mut work) > 0.0, "the work took no time at all");
    }
}
