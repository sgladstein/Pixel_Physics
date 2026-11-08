//! Rules of Seep, each checked against a case where it must fire and, where
//! it matters, a paired case where it must not.

use super::*;

fn quiet(seed: u64) -> Game {
    let mut g = Game::new(seed);
    // Close the cracks so a test sees only the gas it put there.
    for c in g.cracks.iter_mut() {
        c.opens_at = u32::MAX;
    }
    g
}

fn fill(g: &mut Game, x0: i32, x1: i32, y0: i32, y1: i32, c: f32) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            if g.air.is_open(x, y) {
                g.air.c[idx(x as usize, y as usize)] = c;
            }
        }
    }
}

fn sum(g: &Game, x0: i32, x1: i32, y0: i32, y1: i32) -> f32 {
    let mut s = 0.0;
    for y in y0..=y1 {
        for x in x0..=x1 {
            s += g.air.at(x, y);
        }
    }
    s
}

/// The control: a mine left alone loses the upper lamp, and inside the
/// brief's "about 45 seconds".
#[test]
fn doing_nothing_loses_the_upper_lamp() {
    let mut g = Game::new(1);
    for _ in 0..60 * TICKS_PER_SECOND {
        g.step(&[]);
    }
    let a = &g.lamps[0];
    assert_eq!(a.state, LampState::Blown, "lamp A after 60 s of nothing: {:?}", a.state);
    assert!(a.blown_at.unwrap() < 50 * TICKS_PER_SECOND, "blown at {:?}", a.blown_at);
}

/// Spark in the pink band burns it; the same spark in purple fizzles and
/// says so. Paired on the same cell.
#[test]
fn only_pink_burns() {
    let mut pink = quiet(1);
    fill(&mut pink, 60, 90, 118, 128, 0.12);
    pink.act(Act::Spark { x: 75, y: 124 });
    for _ in 0..30 {
        pink.step(&[]);
    }
    assert!(pink.counts.burnt > 5.0, "pink: burnt {}", pink.counts.burnt);
    assert_eq!(pink.counts.fizzles, 0);

    let mut purple = quiet(1);
    fill(&mut purple, 60, 90, 118, 128, 0.5);
    purple.act(Act::Spark { x: 75, y: 124 });
    for _ in 0..30 {
        purple.step(&[]);
    }
    assert!(purple.counts.burnt < 0.5, "purple: burnt {}", purple.counts.burnt);
    assert_eq!(purple.counts.fizzles, 1);
    assert!(purple.floaters.iter().any(|f| f.text == "TOO THICK"));
}

/// A flame runs through pink and stops at purple: a purple core inside a
/// pink skin keeps most of its gas.
#[test]
fn flame_stops_at_the_thick_core() {
    let mut g = quiet(1);
    fill(&mut g, 55, 95, 116, 130, 0.12);
    fill(&mut g, 68, 82, 119, 127, 0.6);
    let core0 = sum(&g, 68, 82, 119, 127);
    g.act(Act::Spark { x: 57, y: 129 });
    for _ in 0..40 {
        g.step(&[]);
    }
    assert!(g.counts.burnt > 5.0, "the skin burnt {}", g.counts.burnt);
    let core1 = sum(&g, 66, 84, 117, 129);
    assert!(core1 > 0.6 * core0, "core {core0} -> {core1}");
}

/// Damp air beside wet rock will not light.
#[test]
fn damp_stops_flame() {
    let mut g = quiet(1);
    fill(&mut g, 60, 90, 118, 128, 0.12);
    assert!(g.ignitable(75, 124));
    g.act(Act::Damp { x: 75, y: 124 });
    assert!(!g.ignitable(75, 124));
}

/// A fed tube is wet and will not light; a starving one is dry and burns
/// like a fuse whatever the gas. Paired on one tube.
#[test]
fn fed_tubes_hold_starving_tubes_burn() {
    let mut g = quiet(1);
    let cells: Vec<(i32, i32)> = (60..100).map(|x| (x, (115..=145).rev().find(|&y| g.air.is_open(x, y)).unwrap())).collect();
    let (x, y) = cells[5];
    let e = g.mould.test_tube(cells);
    g.mould.edges[e].fed = 1.0;
    g.mould.edges[e].d = 2.0;
    assert!(!g.ignitable(x, y), "a fed tube lit");
    g.mould.edges[e].fed = 0.0;
    g.mould.edges[e].d = 0.3;
    assert!(g.ignitable(x, y), "a starving tube would not light");
    g.fire[idx(x as usize, y as usize)] = BURN_TICKS;
    for _ in 0..240 {
        g.step(&[]);
    }
    assert!(g.mould.counts.tubes_burnt >= 1, "the dry tube did not burn");
}

/// Blow moves gas: a pocket blown left ends up further left than the same
/// pocket left alone.
#[test]
fn blow_pushes_gas() {
    let centre = |g: &Game| {
        let (mut s, mut m) = (0.0, 0.0);
        for y in 100..H {
            for x in 0..W {
                let c = g.air.at(x as i32, y as i32);
                s += c * x as f32;
                m += c;
            }
        }
        s / m.max(1e-6)
    };
    let mut still = quiet(1);
    let mut blown = quiet(1);
    for g in [&mut still, &mut blown] {
        fill(g, 120, 140, 114, 122, 0.12);
    }
    for t in 0..120 {
        still.step(&[]);
        let acts = if t < 60 { vec![Act::Blow { x: 140.0 - t as f32 * 0.5, y: 118.0, dx: -1.0, dy: 0.0 }] } else { vec![] };
        blown.step(&acts);
    }
    let (a, b) = (centre(&still), centre(&blown));
    assert!(b < a - 8.0, "left alone {a}, blown {b}");
}

/// A mould led to the stall by a damp road up the sump wall and along the
/// roof caps its seep and clears lamp B; the same mould left alone has not
/// reached it in the same time. (Until 2026-11-08 this test led the mould
/// along the floor to crack 1.)
#[test]
fn a_damp_road_leads_the_mould_to_the_stall() {
    let run = |road: bool| {
        let mut g = Game::new(2);
        for t in 0..40 * TICKS_PER_SECOND {
            let mut acts = Vec::new();
            let road = if road { super::script::stall_road(&g) } else { Vec::new() };
            if t >= 30 && (t as usize - 30) < road.len() {
                let (x, y) = road[t as usize - 30];
                acts.push(Act::Damp { x, y });
            }
            g.step(&acts);
        }
        (g.cracks[2].cap_ema, g.counts.choked_ticks[1])
    };
    let (cap_road, choke_road) = run(true);
    let (cap_none, choke_none) = run(false);
    eprintln!("road: cap {cap_road:.2} choked {choke_road}; alone: cap {cap_none:.2} choked {choke_none}");
    assert!(cap_road > 0.4, "road: cap {cap_road}");
    assert!(cap_none < 0.1, "alone: cap {cap_none}");
    assert!(choke_road + 5 * TICKS_PER_SECOND < choke_none, "lamp B choked {choke_road} ticks led, {choke_none} alone");
}

/// Same seed, same acts, same mine.
#[test]
fn deterministic() {
    let run = || {
        let mut g = Game::new(3);
        for t in 0..900 {
            let acts = if t % 50 == 0 { vec![Act::Damp { x: 60, y: 139 }, Act::Spark { x: 180, y: 105 }] } else { vec![] };
            g.step(&acts);
        }
        (g.air.total(), g.mould.counts.eaten, g.counts.burnt, g.mould.tube_cells())
    };
    assert_eq!(run(), run());
}

/// The stall's seep chokes lamp B: left alone for two minutes it stops lamp
/// B's digging for more than a third of them; the same mine with that one
/// seep shut barely touches it. Paired on one seed.
#[test]
fn the_stall_seep_chokes_lamp_b() {
    let run = |seep: bool| {
        let mut g = Game::new(1);
        if !seep {
            g.cracks[2].opens_at = u32::MAX;
        }
        for _ in 0..120 * TICKS_PER_SECOND {
            g.step(&[]);
        }
        g.counts.choked_ticks[1]
    };
    let (with, without) = (run(true), run(false));
    assert!(with > 45 * TICKS_PER_SECOND, "seep open: lamp B choked {with} ticks");
    assert!(without < 5 * TICKS_PER_SECOND, "seep shut: lamp B choked {without} ticks");
}

/// A safety lamp never lights the gas round it; a naked lamp in the same
/// gas does. Paired on the two lamps of one mine.
#[test]
fn a_safety_lamp_never_lights_the_gas() {
    let burnt = |k: usize| {
        let mut g = quiet(1);
        let (x, y) = (g.lamps[k].x, g.lamps[k].y);
        fill(&mut g, x - 6, x + 6, y - 4, y + 4, 0.11);
        for _ in 0..20 {
            g.step(&[]);
        }
        g.counts.burnt
    };
    assert!(burnt(0) > 2.0, "naked lamp A: burnt {}", burnt(0));
    assert!(burnt(1) < 0.01, "safety lamp B: burnt {}", burnt(1));
}

/// One snip on the core does nothing, and a mould snipped everywhere else
/// grows back from the core. The playtest's case: one curious Cut beside
/// the sump at 20 s killed the mould for the whole level.
#[test]
fn the_core_cannot_be_snipped_and_regrows() {
    let mut g = Game::new(1);
    for _ in 0..20 * TICKS_PER_SECOND {
        g.step(&[]);
    }
    let core = (g.mould.nodes[g.mould.core].x, g.mould.nodes[g.mould.core].y);
    let before = g.mould.live_tubes();
    assert!(before > 0, "no mould to snip at 20 s");
    g.act(Act::Cut { x: core.0 + 1, y: core.1 });
    assert_eq!(g.counts.cuts_hit, 0, "a snip on the core cut something");
    // Now cut everything that can be cut, and kill every tip.
    let cells: Vec<(i32, i32)> = g.mould.edges.iter().filter(|e| e.alive).flat_map(|e| e.cells.clone()).collect();
    for (x, y) in cells {
        g.meters[3] = 1.0;
        g.act(Act::Cut { x, y });
    }
    for t in g.mould.tips.iter_mut() {
        t.alive = false;
    }
    let left = g.mould.live_tubes();
    for _ in 0..40 * TICKS_PER_SECOND {
        g.step(&[]);
    }
    let regrown = g.mould.live_tubes();
    assert!(regrown > left + 2, "after the cuts {left} tubes, 40 s later {regrown}");
}

/// Stars are carts hauled.
#[test]
fn stars_are_carts() {
    let mut g = quiet(1);
    for (c, want) in [(0.0, 0), (39.9, 0), (40.0, 1), (64.0, 1), (65.0, 2), (85.0, 3), (96.0, 3)] {
        g.lamps[0].carts = c;
        g.lamps[1].carts = 0.0;
        assert_eq!(g.stars(), want, "{c} carts");
    }
}

/// Crack 2 hisses before it opens, so it can be planned for.
#[test]
fn crack_2_hisses_first() {
    let mut g = Game::new(1);
    let at = g.cracks[1].opens_at - HISS_LEAD;
    while g.tick <= at {
        g.step(&[]);
    }
    assert!(g.floaters.iter().any(|f| f.text == "HISS"), "no hiss at {} s", g.tick / TICKS_PER_SECOND);
}



/// Cut's job: a dry tube burning toward a lamp sets off the fuse alarm
/// and singes the lamp; the same tube snipped ahead of the flame stops it.
/// Paired on one tube. Before the fuse ran cell by cell (2026-11-08) the
/// flame died at the first junction and this failed with no singe.
#[test]
fn snipping_a_fuse_saves_the_lamp() {
    let run = |snip: bool| {
        let mut g = quiet(1);
        let (lx, ly) = (g.lamps[0].x, g.lamps[0].y);
        let mut firsts = Vec::new();
        for k in 0..8 {
            let cells: Vec<(i32, i32)> = (0..5).map(|i| (lx - 44 + k * 5 + i, ly + 1)).collect();
            firsts.push(cells[0]);
            let e = g.mould.test_tube(cells);
            g.mould.edges[e].fed = 0.0;
            g.mould.edges[e].d = 0.3;
        }
        let (fx, fy) = firsts[0];
        g.fire[idx(fx as usize, fy as usize)] = BURN_TICKS;
        for t in 0..600 {
            let acts = if snip && t == 5 { vec![Act::Cut { x: lx - 22, y: ly + 1 }] } else { vec![] };
            g.step(&acts);
        }
        (g.lamps[0].singes, g.counts.fuse_alarms)
    };
    let (singes_left, alarms_left) = run(false);
    let (singes_cut, _) = run(true);
    assert!(singes_left >= 1 && alarms_left >= 1, "left alone: singes {singes_left} alarms {alarms_left}");
    assert_eq!(singes_cut, 0, "snipped: the lamp was still singed");
}

