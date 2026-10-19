//! **Viability, in steps, against constants of the run** (plan §6.1).
//!
//! Each step is worth more than everything below it, so selection always has
//! something to climb -- the ant rig's foraging score had no slope until it
//! was built this way (`dead-ends.md`'s forage-rung entry). Every threshold is
//! a [`Constants`] field, measured on the shipped species and written in the
//! run's settings line, **never read from the child's genes**: a progress
//! fraction whose denominator a child can lower is a score it can cheat
//! (review F12).

use super::arena::Census;

/// The run's yardsticks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constants {
    /// Shoot cells a plant needs to count as established.
    pub n_est: u32,
    /// Established seedlings the top step stops paying for. Capped, because a
    /// count with no ceiling brings back the many-tiny-seeds cheat.
    pub k_seed: u32,
    /// **The number a plant cannot trade away**: shoot cells the founder must
    /// hold at its first flower. Below it, the flowering step and every step
    /// above it pay half -- otherwise the cheapest climb is a dwarf that
    /// flowers at once, which the review's probe found in one gene
    /// (`after_metamers` 3). 0 turns it off.
    pub size_floor: u32,
}

/// The score of one bed. Germinated 1, established 2, flowered 3, fruited 4,
/// up to 5 with `k_seed` seedlings established; inside each step, a fraction
/// of the way to the next.
pub fn bed_score(c: &Census, k: &Constants) -> f32 {
    if !c.germinated {
        return 0.0;
    }
    let frac = |num: u32, den: u32| if den == 0 { 0.0 } else { (num as f32 / den as f32).min(0.999) };
    if c.peak_shoot < k.n_est {
        return 1.0 + frac(c.peak_shoot, k.n_est);
    }
    if c.first_flower.is_none() {
        // Established, growing towards a flower: paid by size against the
        // floor a flowering plant must reach, never against the child's own
        // flowering threshold.
        return 2.0 + frac(c.peak_shoot, k.size_floor.max(k.n_est));
    }
    let above = if !c.fruited { 3.0 } else { 4.0 + (c.seedlings_established.min(k.k_seed) as f32 / k.k_seed.max(1) as f32) };
    if k.size_floor > 0 && c.shoot_at_first_flower < k.size_floor {
        2.0 + (above - 2.0) * 0.5
    } else {
        above
    }
}

/// Mean and minimum of a genome's bed scores.
pub fn fold(scores: &[f32]) -> (f32, f32) {
    if scores.is_empty() {
        return (0.0, 0.0);
    }
    let mean = scores.iter().sum::<f32>() / scores.len() as f32;
    let min = scores.iter().copied().fold(f32::INFINITY, f32::min);
    (mean, min)
}

/// **The bar is a paired difference** (review F10): per layout, `a - b`.
/// Returns (mean difference, layouts where `a` is ahead, layouts behind, a
/// rough 95% half-width `2 sd / sqrt(n)`).
pub fn paired(a: &[f32], b: &[f32]) -> (f32, usize, usize, f32) {
    let d: Vec<f32> = a.iter().zip(b).map(|(x, y)| x - y).collect();
    if d.is_empty() {
        return (0.0, 0, 0, 0.0);
    }
    let n = d.len() as f32;
    let mean = d.iter().sum::<f32>() / n;
    let var = d.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / (n - 1.0).max(1.0);
    let ahead = d.iter().filter(|&&x| x > 0.0).count();
    let behind = d.iter().filter(|&&x| x < 0.0).count();
    (mean, ahead, behind, 2.0 * var.sqrt() / n.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k() -> Constants {
        Constants { n_est: 20, k_seed: 4, size_floor: 60 }
    }

    fn census(shoot: u32, flower: Option<(u64, u32)>, fruited: bool, seedlings: u32) -> Census {
        Census {
            germinated: true,
            peak_shoot: shoot,
            first_flower: flower.map(|f| f.0),
            shoot_at_first_flower: flower.map_or(0, |f| f.1),
            fruited,
            seedlings_established: seedlings,
            ..Census::default()
        }
    }

    #[test]
    fn every_step_outweighs_everything_below_it() {
        let k = k();
        let steps = [
            bed_score(&Census::default(), &k),
            bed_score(&census(19, None, false, 0), &k),
            bed_score(&census(10_000, None, false, 0), &k),
            bed_score(&census(100, Some((5_000, 80)), false, 0), &k),
            bed_score(&census(100, Some((5_000, 80)), true, 0), &k),
            bed_score(&census(100, Some((5_000, 80)), true, 99), &k),
        ];
        for w in steps.windows(2) {
            assert!(w[1] > w[0], "{steps:?}");
        }
        assert_eq!(steps[5], 5.0, "the top step is capped at k_seed");
    }

    #[test]
    fn a_dwarf_that_flowers_at_once_pays_half_above_establishment() {
        let k = k();
        let tall = bed_score(&census(100, Some((5_000, 80)), true, 4), &k);
        let dwarf = bed_score(&census(100, Some((1_000, 25)), true, 4), &k);
        assert_eq!(tall, 5.0);
        assert_eq!(dwarf, 3.5);
        // ...and still above any plant that never flowered at all.
        assert!(dwarf > bed_score(&census(10_000, None, false, 0), &k));
    }

    #[test]
    fn the_paired_bar_counts_sides() {
        let (mean, ahead, behind, _) = paired(&[3.0, 2.0, 5.0], &[2.0, 2.0, 4.0]);
        assert!((mean - 2.0 / 3.0).abs() < 1e-6);
        assert_eq!((ahead, behind), (2, 0));
    }
}
