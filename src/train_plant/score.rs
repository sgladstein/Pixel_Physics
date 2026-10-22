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

/// **Rung 2's score: juvenile growth, in steps** (plant-community plan
/// §6.2). Not germinated 0; germinated 1 plus the way to establishment;
/// established 2 plus its shoot cells at the end of the rung against
/// `reference` -- the shipped species' mean in the same station, measured
/// once at the start of the run, so a child cannot move its own yardstick.
/// Size is paid on a log scale, `ln(1 + end/ref) / ln(1 + GROW_CAP)`, so
/// the reference scores 0.32 of the step, twice it 0.5, and the step is
/// full only at `GROW_CAP` times it. It was a straight line capped at twice
/// the reference, and the first shrub re-fit (2026-10-21, bright + dim)
/// had its every child on the cap from generation 6: its champion grew
/// 488 cells against a bright yardstick of 244 and 377 against a dim one
/// of 111, so the trainer could no longer tell children apart. A ceiling
/// is still wanted -- it keeps a runaway grower from buying the whole
/// score -- but it belongs where the bed, not the scale, runs out.
pub const GROW_CAP: f32 = 8.0;

/// **Rung 4's score: hold ground, and let the neighbour live** (plan §5,
/// "others do not pay for it"; §6.2 rung 4). The founder's own life
/// ([`sized_score`] against the control's size), less whatever it cost the
/// shipped neighbour beside it below `y.nb_score` -- that neighbour's mean
/// next to the shipped species of the founder's role, so a candidate is
/// charged only for harm past what its role already does. One step of the
/// neighbour's life lost is one step of the founder's own. A neighbour that
/// does better earns nothing: helping is not this rung's job. Graded rather
/// than a floor that rejects, so a candidate that crowds its neighbour a
/// little loses a little. Never below 0.
///
/// **And the whole score shrinks once the neighbour is below
/// [`NEIGHBOUR_FLOOR`] of its usual size.** The difference alone was too
/// weak (2026-10-22, `Reports/plant-community-2026-10-20/m2/hold/`): on the
/// log scale a founder that doubles itself gains about what it loses for
/// halving its neighbour, so the rung-2 re-fits -- which take a shrub beside
/// them from 367 cells to 21 -- already beat shipped here, and twelve
/// generations of rung-4 selection from them moved nothing. Below the floor
/// the score is multiplied by `kept / floor`, so it is still graded: a
/// neighbour kept at a quarter of its size halves the score, one smothered
/// to a twentieth all but zeroes it.
pub fn hold_score(c: &Census, k: &Constants, y: &Yardstick) -> f32 {
    let own = sized_score(c, k, y.own_end);
    let nb = c.neighbour.as_deref().map_or(y.nb_score, |n| sized_score(n, k, y.nb_end));
    (own - (y.nb_score - nb).max(0.0)).max(0.0) * neighbour_kept(c, y)
}

/// The share of its usual size below which a rung-4 neighbour's loss
/// scales the founder's whole score ([`hold_score`]).
pub const NEIGHBOUR_FLOOR: f32 = 0.5;

/// `min(1, kept / NEIGHBOUR_FLOOR)`, where `kept` is the neighbour's end
/// shoot against `y.nb_end`; 1 with no neighbour.
pub fn neighbour_kept(c: &Census, y: &Yardstick) -> f32 {
    c.neighbour.as_deref().map_or(1.0, |n| {
        let kept = n.end_shoot as f32 / y.nb_end.max(1.0);
        (kept / NEIGHBOUR_FLOOR).min(1.0)
    })
}

pub fn grow_score(c: &Census, n_est: u32, reference: f32) -> f32 {
    if !c.germinated {
        return 0.0;
    }
    if c.established_at.is_none() {
        return 1.0 + if n_est == 0 { 0.0 } else { (c.peak_shoot as f32 / n_est as f32).min(0.999) };
    }
    2.0 + size_frac(c.end_shoot, reference)
}

/// Shoot cells against a reference, on [`grow_score`]'s log scale, in [0, 1).
pub fn size_frac(end_shoot: u32, reference: f32) -> f32 {
    let rel = end_shoot as f32 / reference.max(1.0);
    ((1.0 + rel).ln() / (1.0 + GROW_CAP).ln()).min(0.999)
}

/// **A whole life that pays size while it has not flowered**: [`bed_score`],
/// except that an established plant with no flower yet is paid its end shoot
/// against `end_ref` on [`grow_score`]'s scale instead of its peak against
/// `size_floor`. Measured on rung 4 (64 wide, 40,000 frames, 8 layouts): the
/// tree and the creeper never flower in that window, so `bed_score` put
/// every one of them at 2.999 -- 55 cells and 1,900 read alike, and a
/// perennial could be ranked only by what it did to its neighbour. The end,
/// not the peak, so a plant that dies scores its death.
pub fn sized_score(c: &Census, k: &Constants, end_ref: f32) -> f32 {
    if !c.germinated || c.peak_shoot < k.n_est || c.first_flower.is_some() {
        return bed_score(c, k);
    }
    2.0 + size_frac(c.end_shoot, end_ref)
}

/// What rung 4 scores against, per bed kind (station and neighbour),
/// measured on the control: its own end shoot, its neighbour's end shoot
/// beside it, and that neighbour's [`sized_score`] there.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Yardstick {
    pub own_end: f32,
    pub nb_end: f32,
    pub nb_score: f32,
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
    fn the_grow_score_steps_and_pays_size_against_the_reference() {
        let est = |end: u32| Census { germinated: true, peak_shoot: end.max(20), established_at: Some(1_000), end_shoot: end, ..Census::default() };
        let unsprouted = Census::default();
        let small = Census { germinated: true, peak_shoot: 10, ..Census::default() };
        let order = [grow_score(&unsprouted, 20, 100.0), grow_score(&small, 20, 100.0), grow_score(&est(50), 20, 100.0), grow_score(&est(100), 20, 100.0), grow_score(&est(150), 20, 100.0)];
        for w in order.windows(2) {
            assert!(w[1] > w[0], "{order:?}");
        }
        assert!((grow_score(&est(200), 20, 100.0) - 2.5).abs() < 1e-6, "twice the reference scores half way up its step");
        assert!(grow_score(&est(700), 20, 100.0) > grow_score(&est(400), 20, 100.0), "still telling growers apart well past twice the reference");
        assert!(grow_score(&est(10_000), 20, 100.0) < 3.0, "capped below the next step");
    }

    #[test]
    fn holding_ground_is_charged_for_the_neighbour_it_costs() {
        let k = Constants { n_est: 20, k_seed: 24, size_floor: 55 };
        let life = |flowered: bool| Census {
            germinated: true,
            peak_shoot: 200,
            established_at: Some(1_000),
            first_flower: flowered.then_some(5_000),
            shoot_at_first_flower: 100,
            end_shoot: 200,
            ..Census::default()
        };
        let beside = |nb: Census| Census { neighbour: Some(Box::new(nb)), ..life(true) };
        let y = Yardstick { own_end: 200.0, nb_end: 200.0, nb_score: bed_score(&life(true), &k) };
        let alone = bed_score(&life(true), &k);
        assert_eq!(hold_score(&beside(life(true)), &k, &y), alone, "a neighbour as well off as beside the shipped species costs nothing");
        let stunted = hold_score(&beside(life(false)), &k, &y);
        let killed = hold_score(&beside(Census::default()), &k, &y);
        assert!(alone > stunted && stunted > killed, "graded by how much of its life the neighbour lost: {alone} {stunted} {killed}");
        assert!(killed >= 0.0);
        assert_eq!(hold_score(&beside(life(true)), &k, &Yardstick { nb_score: y.nb_score - 1.0, ..y }), alone, "a neighbour doing better earns nothing");
        // The floor: a neighbour as well off in score but smothered in size
        // scales the whole score, graded below the floor.
        let shrunk = |end: u32| hold_score(&beside(Census { end_shoot: end, ..life(true) }), &k, &y);
        assert_eq!(shrunk(100), alone, "at the floor nothing is taken");
        assert!(shrunk(100) > shrunk(50) && shrunk(50) > shrunk(10) && shrunk(10) > 0.0, "graded below it");
        assert!((shrunk(50) - alone / 2.0).abs() < 1e-5, "a quarter of its size halves the score");
        // A perennial that has not flowered is paid its size, not pinned at
        // the top of the step as `bed_score` pins it.
        let tree = |end: u32| Census { end_shoot: end, peak_shoot: end, ..life(false) };
        assert_eq!(bed_score(&tree(500), &k), bed_score(&tree(2_000), &k), "the defect: bed_score reads them alike");
        assert!(sized_score(&tree(2_000), &k, 1_000.0) > sized_score(&tree(500), &k, 1_000.0));
    }

    #[test]
    fn the_paired_bar_counts_sides() {
        let (mean, ahead, behind, _) = paired(&[3.0, 2.0, 5.0], &[2.0, 2.0, 4.0]);
        assert!((mean - 2.0 / 3.0).abs() < 1e-6);
        assert_eq!((ahead, behind), (2, 0));
    }
}
