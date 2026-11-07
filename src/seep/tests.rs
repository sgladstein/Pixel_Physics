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

/// A mould led to the crack by a damp road eats the leak; the same mould
/// left alone eats none of it in the same time.
#[test]
fn a_damp_road_leads_the_mould_to_the_crack() {
    let run = |road: bool| {
        let mut g = Game::new(2);
        for t in 0..75 * TICKS_PER_SECOND {
            let mut acts = Vec::new();
            if road && t < 400 && t % 3 == 0 {
                let x = 44 + (t / 3) as i32;
                if x <= 178 {
                    if let Some(y) = (118..=145).rev().find(|&y| g.air.is_open(x, y)) {
                        acts.push(Act::Damp { x, y });
                    }
                }
            }
            g.step(&acts);
        }
        (g.cracks[0].capped, g.mould.counts.eaten)
    };
    let (cap_road, eat_road) = run(true);
    let (cap_none, eat_none) = run(false);
    assert!(cap_road > 10.0 && eat_road > 50.0, "road: capped {cap_road} eaten {eat_road}");
    assert!(cap_none < 1.0, "alone: capped {cap_none} eaten {eat_none}");
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
