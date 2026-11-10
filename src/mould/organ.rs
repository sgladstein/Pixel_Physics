//! **Mould Organ: the network plays a rhythm.** `cargo run --release --bin organ`
//!
//! Brief: `/mnt/project-files/game-design/mould-game-brainstorm-2026-11-08/
//! prototypes/organ.md`. The question is whether steering the network is fun
//! when the reward is a *sound* rather than food joined.
//!
//! One pile is the **heart**; the other three are **bells**. Every beat the
//! heart thumps and a pulse leaves it, running out along the tube at a fixed
//! speed. When the pulse reaches a bell, that bell rings its note. So **tube
//! length is time**: bend a tube with the lamp and its note comes late; wall
//! a tube off with salt and lead it round with oats, and the note goes
//! silent, then comes back late; a loop gives a second route, so a second,
//! quieter note (an echo).
//!
//! **What the probe found the mould does to a rhythm** (`organprobe`, 8
//! seeds; `result.md` in the brief's `organ/` folder): left alone a tube's
//! length does not move at all -- 0 cells over 60 s -- so the beat is rock
//! steady. But the mould's taste for short paths undoes every change once
//! the hand is off it: a lamp push (+79 to +245 ms while held) is gone
//! within 5 s of letting go on 8 of 8 seeds, and a salt-and-oats detour
//! (+262 to +452 ms) withers once its oats are eaten. The lamp is a pitch
//! bend, not a pencil.
//!
//! **Everything about the mould is the mould game's own model**, stepped by
//! the same sub-steps in the same order as `Dish::step` minus `phase_tick`
//! (there is no storm, no late food and no score here). This file is a child
//! of `mould`, which is what lets it call those private sub-steps without
//! making any of them public or changing what they do. The one rule of its
//! own: the heart and the bells never run out of food (`step`).
//!
//! **The pulse is a snapshot.** At each beat the tube length from the heart
//! to every cell is measured once (`heart_geodesic`), and the pulse front
//! then rides that map: a cell is lit while `distance ~ speed x time since
//! the beat`. Arrival times are fixed when the beat fires, so the mould
//! moving under a pulse in flight cannot make a note ring twice or not at
//! all, and the rhythm a player hears is the network as it was on the beat.
//!
//! # Step 0, and why the organ does not use the feeding search's count
//!
//! The brief's first measurement was how much the breadth-first tube
//! distance (four-neighbour steps, as the feeding search counts) from heart
//! to bell wobbles on a settled dish nobody touches. Over 8 joined seeds it
//! was **0 cells** for every bell over 10 s and over 60 s. That was too tidy
//! to believe, and the positive control showed why: a lamp that visibly
//! bowed the high bell's tube by about ten cells moved the step count by 0
//! on 7 of 8 seeds. Four-neighbour steps give every staircase between two
//! points the same length (`dx + dy`), so they cannot see a tube's shape at
//! all. The organ plays by `heart_geodesic` instead, which does see it (the
//! same push: 46-122 ms later at 80 cells/s), and which also read 0 cells of
//! wobble over the same windows. Numbers: `examples/organprobe.rs`, `step0=1`
//! and `bend=1`, and `result.md` in the brief's `organ/` folder.

use super::{cell, disc, idx, Dish, Ground, Params, Pile, Rng, Walker, H, STEPS_PER_SECOND, W};
use std::f32::consts::TAU;

/// Beat period, steps: one pulse every two seconds, the pitch's "every two
/// seconds". The heart thumps on it.
pub const BEAT_STEPS: u64 = 2 * STEPS_PER_SECOND as u64;
/// Pulse speed along tube, cells per second. **50, so one cell is 20 ms**:
/// the settled tubes here are about 35-44, 45-54 and 65-74 cells, which
/// rings the three bells at about 0.8, 1.0 and 1.4 s into a 2 s bar. 80 was
/// tried first: the bend test's lamp push (3-10 cells) was then 46-122 ms,
/// which reads as sloppiness; at 50 the same push is 74-195 ms, around a
/// sixteenth at 120 beats a minute, which reads as swing.
pub const PULSE_SPEED: f32 = 50.0;
/// Farthest a pulse runs, in tube cells: **two bars**. A route longer than
/// one bar rings in the next bar, over the next pulse, rather than falling
/// silent -- a loop's long way round is usually longer than a bar (the low
/// bell's echo through a bridge to the mid bell is about 120 cells).
pub const PULSE_REACH: f32 = 2.0 * PULSE_SPEED * (BEAT_STEPS as f32 / STEPS_PER_SECOND as f32);
/// A second route must be at least this much longer, in cells, to ring as
/// an echo: anything closer is the same tube reached a little further round.
pub const ECHO_GAP: f32 = 12.0;
/// How much the pulse prefers the thick middle of a tube (see `geodesic`).
/// **0: a tube is a shape, not a thickness.** At 3 (the pulse keeping to
/// the thick core, so that a bowed tube's core is followed round its bow)
/// the wobble on a settled dish nobody touched went from 0 cells to 13-17
/// (low), 16-21 (mid) and 19-26 (high) over 10 s -- step 0's own "mush"
/// line -- because the trail inside a tube flickers every step even when
/// its outline holds still.
pub const CORE_WEIGHT: f32 = 0.0;
/// Trail at which a cell counts as the full thickness of a tube.
pub const CORE_TOP: f32 = 20.0;
/// The pulse's lit band, in cells behind its front.
pub const PULSE_BAND: f32 = 9.0;

/// Heart, then the bells, low to high: (x, y, r). Jittered by seed like the
/// mould game's own layout, so paired seeds are different dishes of one shape.
const LAYOUT: [(f32, f32, f32); 4] = [
    (112.0, 82.0, 7.0),  // heart
    (70.0, 50.0, 6.0),   // low bell
    (166.0, 46.0, 6.0),  // mid bell
    (180.0, 128.0, 6.0), // high bell
];

/// The bells' notes, Hz: C3, G3, E4 -- a C-major chord spread out, so any
/// two together sound like they belong and echoes never clash.
pub const NOTES: [f32; 3] = [130.81, 196.00, 329.63];
pub const NOTE_NAMES: [&str; 3] = ["LOW", "MID", "HIGH"];
/// The heart's thump, Hz.
pub const HEART_HZ: f32 = 55.0;
/// Colours: heart, then the bells.
pub const COLOURS: [[u8; 3]; 4] = [[255, 96, 150], [255, 140, 70], [120, 230, 120], [110, 175, 255]];

/// **The target rhythm**, seconds into the bar for each bell: dum (the
/// heart, at 0) ... low ... mid ... high, evenly in the back half. The low
/// bell's natural tube already lands near its tick; the mid and high bells
/// need about 13 and 15-20 more cells of tube, which the mould takes back
/// whenever the hand lets go (see the header), so as built the target is
/// something to hold, not something to build once. Shown by default; `T`
/// hides it. The scripted hand does not aim at it.
pub const TARGET: [f32; 3] = [0.75, 1.25, 1.75];
/// A note within this many milliseconds of its tick is on the beat.
pub const ON_BEAT_MS: f32 = 60.0;

/// Salt lines in an organ dish: the lasting way to lengthen a tube.
pub const ORGAN_SALT: u32 = 3;

// ------------------------------------------------------------------ the dish

/// A new organ dish: the mould game's dish with its piles replaced by a heart
/// and three bells, no late food and no storm, and the inoculum on the heart.
pub fn new_dish(seed: u64) -> Dish {
    // The organ was built and measured on the shared store (step 0: no
    // wobble; the lamp bends, the mould takes it back), before the mould
    // game moved to each piece eating its own food on 2026-11-08. It keeps
    // the store it was measured on until someone re-measures it.
    new_dish_with(
        seed,
        Params {
            pieces: false,
            ..Params::default()
        },
    )
}

pub fn new_dish_with(seed: u64, params: Params) -> Dish {
    let mut d = Dish::with_params(seed, params);
    let mut rng = Rng::new(seed ^ 0x0E6A_2C0F);
    d.piles = LAYOUT
        .iter()
        .enumerate()
        .map(|(k, &(x, y, r))| {
            let (x, y) = if k == 0 {
                (x, y)
            } else {
                let j = 6.0;
                (x + (rng.f32() - 0.5) * 2.0 * j, y + (rng.f32() - 0.5) * 2.0 * j)
            };
            Pile {
                x,
                y,
                r,
                food: super::PILE_FOOD,
                full: super::PILE_FOOD,
                oats: false,
                expires: u64::MAX,
                eaters: 0,
                foe_eaters: 0,
            }
        })
        .collect();
    d.pending.clear();
    d.salt_left = ORGAN_SALT;
    // Move the inoculum onto the heart.
    for w in &d.walkers {
        if let Some(i) = cell(w.x, w.y) {
            d.occ[i] = 0;
        }
    }
    d.walkers.clear();
    let (hx, hy) = (d.piles[0].x, d.piles[0].y);
    let mut placed = 0;
    let mut tries = 0;
    while placed < 400 && tries < 20_000 {
        tries += 1;
        let a = d.rng.f32() * TAU;
        let r = d.rng.f32().sqrt() * 12.0;
        let (x, y) = (hx + a.cos() * r, hy + a.sin() * r);
        if let Some(i) = cell(x, y) {
            if d.ground[i] == Ground::Agar && d.occ[i] == 0 {
                d.occ[i] = 1;
                let heading = d.rng.f32() * TAU;
                d.walkers.push(Walker { slept: u64::MAX, woke: u64::MAX, x, y, heading, food: 0.0 });
                placed += 1;
            }
        }
    }
    d.census();
    d
}

/// One step of the organ's mould: `Dish::step` without the dish clock (no
/// storm, no late food). Oats still come back on the mould game's timer.
/// **The heart and the bells never run out**: a mould game pile is eaten
/// empty in about two and a half minutes, and an instrument that falls
/// silent because its bells were eaten is not the question being asked.
pub fn step(d: &mut Dish) {
    for p in d.piles.iter_mut().filter(|p| !p.oats) {
        p.food = p.full;
    }
    d.update_light();
    d.push_light();
    d.deposit_food();
    d.move_walkers();
    d.diffuse();
    if d.t.is_multiple_of(super::FED_EVERY) {
        d.census();
    }
    d.economy();
    // A snip's wound heals here as in `Dish::step`. Left out, a snip was a
    // permanent wall drawn as plain agar after its 1 s fade, which every
    // tube and so every note routed round (review 1, finding 4: 171 wound
    // cells still 171 after 30 s).
    d.heal_wounds();
    d.expire_oats();
    let sps = STEPS_PER_SECOND as u64;
    if d.t > 0 && d.t.is_multiple_of(super::OATS_REGROW_SECONDS as u64 * sps) && d.oats_left < super::OATS_STOCK {
        d.oats_left += 1;
    }
    if d.lamp.is_some() {
        d.counts.lamp_steps += 1;
    }
    d.t += 1;
}

// ---------------------------------------------------------------- the rulers

/// Tube distance from the heart to every cell, breadth-first along tube as
/// the feeding search runs it (4-neighbour, trail above `tube`), out to
/// `reach`; `u16::MAX` where no tube from the heart reaches. The brief's
/// step 0 ruler; the organ itself plays by `heart_geodesic`.
pub fn heart_distance(d: &Dish, reach: u16, out: &mut Vec<u16>) {
    out.clear();
    out.resize(W * H, u16::MAX);
    let mut queue = std::collections::VecDeque::new();
    let h = &d.piles[0];
    for (ox, oy) in disc(h.r + 1.0) {
        if let Some(i) = cell(h.x + ox, h.y + oy) {
            if d.ground[i] == Ground::Agar && out[i] != 0 {
                out[i] = 0;
                queue.push_back(i);
            }
        }
    }
    let tube = d.params.tube;
    while let Some(i) = queue.pop_front() {
        let dd = out[i];
        if dd >= reach {
            continue;
        }
        let (x, y) = (i % W, i / W);
        for (ok, j) in [
            (x > 0, i.wrapping_sub(1)),
            (x + 1 < W, i + 1),
            (y > 0, i.wrapping_sub(W)),
            (y + 1 < H, i + W),
        ] {
            if ok && d.ground[j] == Ground::Agar && d.trail[j] > tube && out[j] == u16::MAX {
                out[j] = dd + 1;
                queue.push_back(j);
            }
        }
    }
}

/// The nearest step count at a bell's rim, or `None` if no tube reaches it.
pub fn bell_distance(d: &Dish, dist: &[u16], bell: usize) -> Option<u16> {
    let p = &d.piles[bell];
    let mut best = u16::MAX;
    for (ox, oy) in disc(p.r + 1.0) {
        if let Some(i) = cell(p.x + ox, p.y + oy) {
            best = best.min(dist[i]);
        }
    }
    (best != u16::MAX).then_some(best)
}

/// The route a step-count search takes to a bell, heart end first: walked
/// back from the bell's nearest rim cell. Empty if the bell is not reached.
pub fn route_to_bell(d: &Dish, dist: &[u16], bell: usize) -> Vec<usize> {
    let p = &d.piles[bell];
    let mut at = None;
    for (ox, oy) in disc(p.r + 1.0) {
        if let Some(i) = cell(p.x + ox, p.y + oy) {
            if dist[i] != u16::MAX && at.is_none_or(|j: usize| dist[i] < dist[j]) {
                at = Some(i);
            }
        }
    }
    let Some(mut i) = at else { return Vec::new() };
    let mut out = vec![i];
    while dist[i] > 0 {
        let (x, y) = (i % W, i / W);
        let next = [
            (x > 0, i.wrapping_sub(1)),
            (x + 1 < W, i + 1),
            (y > 0, i.wrapping_sub(W)),
            (y + 1 < H, i + W),
        ]
        .into_iter()
        .find(|&(ok, j)| ok && dist[j] == dist[i] - 1)
        .map(|(_, j)| j);
        match next {
            Some(j) => {
                out.push(j);
                i = j;
            }
            None => break,
        }
    }
    out.reverse();
    out
}

/// **Tube length as the eye reads it**: the shortest path from the heart
/// along tube, measured in straight-line cells rather than in grid steps,
/// out to `reach` cells; `f32::INFINITY` where no tube reaches.
///
/// Why not the feeding search's breadth-first count: four-neighbour steps
/// measure every path that only ever heads toward the bell (a staircase) as
/// the same length -- `dx + dy` -- so **a tube bowed sideways inside that
/// box is not one step longer**. Step 0 measured exactly that: zero wobble,
/// and a lamp that visibly bowed the high bell's tube by about ten cells
/// moved the step count by 0 on 7 of 8 seeds (2 cells on the eighth). The
/// count could not wobble because it could not see shape at all.
///
/// So this is Dijkstra over sixteen neighbours (the straight, diagonal and
/// knight's moves, costed 1, sqrt 2 and sqrt 5), each move allowed only if
/// the cells it passes over are tube too, so it cannot cut a corner or hop
/// a gap. Its length for a straight run is within about 3% of the true
/// one at any angle, so bending a tube lengthens it as a rope's would.
pub fn heart_geodesic(d: &Dish, reach: f32, out: &mut Vec<f32>) {
    geodesic(d, reach, CORE_WEIGHT, |_| false, out)
}

/// `heart_geodesic` at a chosen `core` weight (the probe's rulers).
pub fn heart_geodesic_core(d: &Dish, reach: f32, core: f32, out: &mut Vec<f32>) {
    geodesic(d, reach, core, |_| false, out)
}

/// `heart_geodesic` with some cells closed off (`blocked(i)`). A move costs
/// its length times `1 + core * (1 - thickness)`, thickness being the
/// trail at the cell it lands on over `CORE_TOP`, capped at 1: at `core`
/// 0 every tube cell is alike, above it the pulse keeps to the thick middle.
fn geodesic(d: &Dish, reach: f32, core: f32, blocked: impl Fn(usize) -> bool, out: &mut Vec<f32>) {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    out.clear();
    out.resize(W * H, f32::INFINITY);
    let tube = d.params.tube;
    let open = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x as usize >= W || y as usize >= H {
            return false;
        }
        let i = idx(x as usize, y as usize);
        d.ground[i] == Ground::Agar && d.trail[i] > tube && !blocked(i)
    };
    let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
    let h = &d.piles[0];
    for (ox, oy) in disc(h.r + 1.0) {
        if let Some(i) = cell(h.x + ox, h.y + oy) {
            if d.ground[i] == Ground::Agar && out[i] != 0.0 {
                out[i] = 0.0;
                heap.push(Reverse((0f32.to_bits(), i as u32)));
            }
        }
    }
    const S2: f32 = std::f32::consts::SQRT_2;
    const S5: f32 = 2.236_068;
    // (dx, dy, cost, kind): 0 straight, 1 diagonal, 2 knight's move.
    const MOVES: [(i32, i32, f32, u8); 16] = [
        (1, 0, 1.0, 0),
        (-1, 0, 1.0, 0),
        (0, 1, 1.0, 0),
        (0, -1, 1.0, 0),
        (1, 1, S2, 1),
        (1, -1, S2, 1),
        (-1, 1, S2, 1),
        (-1, -1, S2, 1),
        (2, 1, S5, 2),
        (2, -1, S5, 2),
        (-2, 1, S5, 2),
        (-2, -1, S5, 2),
        (1, 2, S5, 2),
        (-1, 2, S5, 2),
        (1, -2, S5, 2),
        (-1, -2, S5, 2),
    ];
    while let Some(Reverse((bits, i))) = heap.pop() {
        let di = f32::from_bits(bits);
        let i = i as usize;
        if di > out[i] || di >= reach {
            continue;
        }
        let (x, y) = ((i % W) as i32, (i / W) as i32);
        for &(dx, dy, c, kind) in &MOVES {
            let (nx, ny) = (x + dx, y + dy);
            if !open(nx, ny) {
                continue;
            }
            // Diagonals need one of the two corner cells (no squeezing
            // through a pinch); knight's moves need both cells they cross.
            let ok = match kind {
                1 => open(x + dx, y) || open(x, y + dy),
                2 if dx.abs() == 2 => open(x + dx / 2, y) && open(x + dx / 2, y + dy),
                2 => open(x, y + dy / 2) && open(x + dx, y + dy / 2),
                _ => true,
            };
            if !ok {
                continue;
            }
            let j = idx(nx as usize, ny as usize);
            let thin = 1.0 - (d.trail[j] / CORE_TOP).min(1.0);
            let nd = di + c * (1.0 + core * thin);
            if nd < out[j] {
                out[j] = nd;
                heap.push(Reverse((nd.to_bits(), j as u32)));
            }
        }
    }
}

/// The nearest geodesic distance at a bell's rim, or `None` if unreached.
pub fn bell_geodesic(d: &Dish, dist: &[f32], bell: usize) -> Option<f32> {
    let p = &d.piles[bell];
    let mut best = f32::INFINITY;
    for (ox, oy) in disc(p.r + 1.0) {
        if let Some(i) = cell(p.x + ox, p.y + oy) {
            best = best.min(dist[i]);
        }
    }
    best.is_finite().then_some(best)
}

/// **A loop's second route to a bell**, as (its length, the map of the
/// search that found it), or `None` if there is no second route at least
/// `ECHO_GAP` longer than the first.
///
/// A shortest-path search cannot see a second route on its own: the first
/// arrival claims every cell. So close off the way the first route came in
/// -- a wedge of 120 degrees round its entry point, out to `ECHO_GAP` cells
/// beyond the bell's rim -- and search again. Whatever still reaches the
/// rim has come round by another tube.
pub fn echo_route(d: &Dish, main: &[f32], bell: usize, reach: f32) -> Option<(f32, Vec<f32>)> {
    let first = bell_geodesic(d, main, bell)?;
    let p = &d.piles[bell];
    let rim: Vec<usize> = disc(p.r + 1.0).filter_map(|(ox, oy)| cell(p.x + ox, p.y + oy)).collect();
    let entry: Vec<usize> = rim.iter().copied().filter(|&i| main[i] <= first + 2.0).collect();
    let n = entry.len() as f32;
    let ex = entry.iter().map(|&i| (i % W) as f32 + 0.5).sum::<f32>() / n - p.x;
    let ey = entry.iter().map(|&i| (i / W) as f32 + 0.5).sum::<f32>() / n - p.y;
    let el = ex.hypot(ey).max(0.01);
    let wedge = |i: usize| {
        let (x, y) = ((i % W) as f32 + 0.5 - p.x, (i / W) as f32 + 0.5 - p.y);
        let r = x.hypot(y);
        r <= p.r + 1.0 + ECHO_GAP && (x * ex + y * ey) / (r.max(0.01) * el) > 0.5
    };
    let mut alt = Vec::new();
    geodesic(d, reach, CORE_WEIGHT, wedge, &mut alt);
    let second = rim
        .iter()
        .filter(|&&i| !wedge(i))
        .map(|&i| alt[i])
        .fold(f32::INFINITY, f32::min);
    (second.is_finite() && second >= first + ECHO_GAP).then_some((second, alt))
}

/// How loud a route rings, 0..1: **a thick tube rings louder, and a
/// starving one fades** (the pitch's law 1, heard). Walks the map downhill
/// from the bell's nearest rim cell to the heart and reads the trail and the
/// feeding search along the way.
fn route_gain(d: &Dish, dist: &[f32], bell: usize) -> f32 {
    let p = &d.piles[bell];
    let mut at = None;
    for (ox, oy) in disc(p.r + 1.0) {
        if let Some(i) = cell(p.x + ox, p.y + oy) {
            if dist[i].is_finite() && at.is_none_or(|j: usize| dist[i] < dist[j]) {
                at = Some(i);
            }
        }
    }
    let Some(mut i) = at else { return 0.0 };
    let (mut n, mut thick, mut starving) = (0.0f32, 0.0f32, 0.0f32);
    for _ in 0..600 {
        n += 1.0;
        thick += (d.trail[i] / 20.0).min(1.0);
        if d.fed[i] == u16::MAX {
            starving += 1.0;
        }
        if dist[i] <= 0.0 {
            break;
        }
        let (x, y) = ((i % W) as i32, (i / W) as i32);
        let mut best = (dist[i], i);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && (nx as usize) < W && (ny as usize) < H {
                    let j = idx(nx as usize, ny as usize);
                    if dist[j] < best.0 {
                        best = (dist[j], j);
                    }
                }
            }
        }
        if best.1 == i {
            break;
        }
        i = best.1;
    }
    let thick = thick / n;
    let starving = starving / n;
    ((0.35 + 0.65 * thick) * (1.0 - 0.8 * starving)).clamp(0.0, 1.0)
}

// ----------------------------------------------------------------- the organ

/// Who sounds a note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Voice {
    Heart,
    /// Bell 0 (low), 1 (mid) or 2 (high).
    Bell(usize),
}

impl Voice {
    pub fn colour(self) -> [u8; 3] {
        match self {
            Voice::Heart => COLOURS[0],
            Voice::Bell(b) => COLOURS[b + 1],
        }
    }
}

/// One note, at the step it sounds.
#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub step: u64,
    pub voice: Voice,
    /// 0..1.
    pub gain: f32,
    /// -1 (left of the dish) ..= 1 (right).
    pub pan: f32,
    /// Came round a loop: the second arrival at its bell this bar.
    pub echo: bool,
    /// Seconds after the beat.
    pub after: f32,
}

/// Counts of what the organ did, for the probe and the HUD.
#[derive(Clone, Debug, Default)]
pub struct OrganCounts {
    pub beats: u32,
    /// First arrivals, per bell.
    pub notes: [u32; 3],
    /// Second arrivals (echoes), per bell.
    pub echoes: [u32; 3],
    /// First arrivals within `ON_BEAT_MS` of the bell's target tick, and not.
    pub on_beat: u32,
    pub off_beat: u32,
}

/// One pulse: the beat it left the heart on, and the maps its fronts ride.
pub struct Pulse {
    pub beat_at: u64,
    /// Tube length from the heart to every cell, at the beat.
    pub map: Vec<f32>,
    /// Echo maps, per bell that has a second route.
    pub echo_maps: Vec<(usize, Vec<f32>)>,
}

/// The dish, the beat clock, the pulses in flight and the notes to come.
pub struct Organ {
    pub dish: Dish,
    /// Pulses still running, oldest first: at most two, as a pulse runs for
    /// two bars (`PULSE_REACH`).
    pub pulses: Vec<Pulse>,
    /// The newest pulse's notes, in the order they sound: what the bar in
    /// the HUD shows.
    pub bar: Vec<Note>,
    /// Notes scheduled and not yet sounded, by step.
    pending: Vec<Note>,
    /// Every note sounded so far.
    pub played: Vec<Note>,
    pub counts: OrganCounts,
    /// Show the target ticks and count notes against them.
    pub target: bool,
}

impl Organ {
    pub fn new(seed: u64) -> Self {
        Self::from_dish(new_dish(seed))
    }

    /// Wrap a dish (a fresh one, or the probe's settled ones).
    pub fn from_dish(dish: Dish) -> Self {
        Organ {
            dish,
            pulses: Vec::new(),
            bar: Vec::new(),
            pending: Vec::new(),
            played: Vec::new(),
            counts: OrganCounts::default(),
            target: true,
        }
    }

    /// One step; returns the notes that sounded in it.
    pub fn step(&mut self) -> Vec<Note> {
        if self.dish.t.is_multiple_of(BEAT_STEPS) {
            self.fire();
        }
        step(&mut self.dish);
        let mut out = Vec::new();
        let t = self.dish.t;
        let due = self.pending.iter().take_while(|n| n.step < t).count();
        for n in self.pending.drain(..due) {
            match n.voice {
                Voice::Heart => {}
                Voice::Bell(b) if n.echo => self.counts.echoes[b] += 1,
                Voice::Bell(b) => {
                    self.counts.notes[b] += 1;
                    if self.target {
                        if ((n.after - TARGET[b]) * 1000.0).abs() <= ON_BEAT_MS {
                            self.counts.on_beat += 1;
                        } else {
                            self.counts.off_beat += 1;
                        }
                    }
                }
            }
            self.played.push(n);
            out.push(n);
        }
        out
    }

    /// The beat: measure every tube from the heart once, and schedule the
    /// pulse's notes.
    fn fire(&mut self) {
        let d = &self.dish;
        let t = d.t;
        self.counts.beats += 1;
        let two_bars = 2 * BEAT_STEPS;
        let mut recycled = None;
        self.pulses.retain_mut(|p| {
            let keep = t < p.beat_at + two_bars;
            if !keep {
                recycled = Some(std::mem::take(&mut p.map));
            }
            keep
        });
        let mut map = recycled.unwrap_or_default();
        heart_geodesic(d, PULSE_REACH + 1.0, &mut map);
        let pan = |x: f32| (x / W as f32 * 2.0 - 1.0) * 0.7;
        let sps = STEPS_PER_SECOND as f32;
        let h = &d.piles[0];
        let mut notes = vec![Note {
            step: t,
            voice: Voice::Heart,
            gain: 1.0,
            pan: pan(h.x),
            echo: false,
            after: 0.0,
        }];
        let mut echo_maps = Vec::new();
        for b in 0..3 {
            let Some(len) = bell_geodesic(d, &map, b + 1) else { continue };
            if len > PULSE_REACH {
                continue;
            }
            let p = &d.piles[b + 1];
            let after = len / PULSE_SPEED;
            notes.push(Note {
                step: t + (after * sps).round() as u64,
                voice: Voice::Bell(b),
                gain: route_gain(d, &map, b + 1),
                pan: pan(p.x),
                echo: false,
                after,
            });
            if let Some((len2, alt)) = echo_route(d, &map, b + 1, PULSE_REACH + 1.0) {
                if len2 <= PULSE_REACH {
                    let after = len2 / PULSE_SPEED;
                    notes.push(Note {
                        step: t + (after * sps).round() as u64,
                        voice: Voice::Bell(b),
                        gain: route_gain(d, &alt, b + 1) * 0.6,
                        pan: pan(p.x),
                        echo: true,
                        after,
                    });
                    echo_maps.push((b, alt));
                }
            }
        }
        notes.sort_by_key(|n| n.step);
        self.pending.extend(notes.iter().copied());
        self.pending.sort_by_key(|n| n.step);
        self.bar = notes;
        self.pulses.push(Pulse { beat_at: t, map, echo_maps });
    }

    /// Seconds since the last beat.
    pub fn since_beat(&self) -> f32 {
        let at = self.pulses.last().map_or(0, |p| p.beat_at);
        (self.dish.t - at) as f32 / STEPS_PER_SECOND as f32
    }
}

// --------------------------------------------------------------- the drawing

/// Draw a frame: the mould game's own dish drawing, then the pulse, the
/// bells and the organ's own text rows in place of the mould game's.
pub fn draw(o: &Organ, view: &super::render::View, scale: u32, frame: &mut [u8]) {
    use super::render;
    let d = &o.dish;
    render::draw(d, view, scale, frame);
    let (fw, fh) = render::frame_size(scale);
    let s = scale as usize;
    // The pulse: a bright band riding the tubes, a full replace on a fixed
    // ramp (white at the front, the heart's colour behind), so it reads on
    // any tube. Echo fronts ride their own maps, drawn only where they come
    // later than the first front (the long way round), in the bell's colour.
    let mut paint = |i: usize, c: [u8; 3]| {
        let (x, y) = (i % W, i / W);
        for py in 0..s {
            let row = ((y * s + py) * fw as usize + x * s) * 4;
            for px in 0..s {
                let k = row + px * 4;
                frame[k..k + 3].copy_from_slice(&c);
            }
        }
    };
    let band = |behind: f32, tint: [u8; 3]| -> Option<[u8; 3]> {
        if !(0.0..PULSE_BAND).contains(&behind) {
            return None;
        }
        let f = 1.0 - behind / PULSE_BAND;
        Some([0, 1, 2].map(|k| (tint[k] as f32 + (255.0 - tint[k] as f32) * f * f) as u8))
    };
    for pulse in &o.pulses {
        let front = (d.t - pulse.beat_at) as f32 / STEPS_PER_SECOND as f32 * PULSE_SPEED;
        for i in 0..W * H {
            let m = pulse.map[i];
            if !m.is_finite() || d.trail[i] <= d.params.tube {
                continue;
            }
            if let Some(c) = band(front - m, [200, 60, 120]) {
                paint(i, c);
            }
        }
        for (b, alt) in &pulse.echo_maps {
            let tint = COLOURS[b + 1].map(|v| v / 2);
            for (i, &a) in alt.iter().enumerate() {
                if !a.is_finite() || a < pulse.map[i] + 3.0 || d.trail[i] <= d.params.tube {
                    continue;
                }
                if let Some(c) = band(front - a, tint) {
                    paint(i, c);
                }
            }
        }
    }
    // The heart and the bells: a coloured ring always, a flash when they
    // sound -- a filled disc that fades and a ring that spreads.
    let sps = STEPS_PER_SECOND as f32;
    for (k, p) in d.piles.iter().filter(|p| !p.oats).enumerate().take(4) {
        let voice = if k == 0 { Voice::Heart } else { Voice::Bell(k - 1) };
        let c = voice.colour();
        ring(frame, fw, scale, (p.x, p.y), p.r + 3.0, c);
        let last = o.played.iter().rev().find(|n| n.voice == voice);
        if let Some(n) = last {
            let age = (d.t.saturating_sub(n.step)) as f32 / sps;
            if age < 0.5 {
                let f = 1.0 - age / 0.5;
                let g = 0.4 + 0.6 * n.gain;
                let fill = [0, 1, 2].map(|j| (c[j] as f32 + (255.0 - c[j] as f32) * f * 0.6) as u8);
                disc_px(frame, fw, scale, (p.x, p.y), p.r * (0.6 + 0.4 * f * g), fill);
                ring(frame, fw, scale, (p.x, p.y), p.r + 3.0 + 14.0 * age / 0.5, c.map(|v| (v as f32 * f) as u8));
            }
        }
    }
    hud(o, view, scale, frame, (fw, fh));
}

fn put(frame: &mut [u8], fw: u32, x: i32, y: i32, c: [u8; 3]) {
    let fh = frame.len() as u32 / 4 / fw;
    if x < 0 || y < 0 || x as u32 >= fw || y as u32 >= fh {
        return;
    }
    let i = (y as usize * fw as usize + x as usize) * 4;
    frame[i..i + 3].copy_from_slice(&c);
    frame[i + 3] = 255;
}

fn ring(frame: &mut [u8], fw: u32, scale: u32, (cx, cy): (f32, f32), r: f32, c: [u8; 3]) {
    let s = scale as f32;
    let rp = r * s;
    let n = (rp * 6.3).ceil().max(8.0) as usize;
    for k in 0..n {
        let a = k as f32 / n as f32 * TAU;
        put(frame, fw, (cx * s + a.cos() * rp) as i32, (cy * s + a.sin() * rp) as i32, c);
    }
}

fn disc_px(frame: &mut [u8], fw: u32, scale: u32, (cx, cy): (f32, f32), r: f32, c: [u8; 3]) {
    let s = scale as f32;
    let rp = r * s;
    let n = rp.ceil() as i32;
    for dy in -n..=n {
        for dx in -n..=n {
            if (dx * dx + dy * dy) as f32 <= rp * rp {
                put(frame, fw, (cx * s) as i32 + dx, (cy * s) as i32 + dy, c);
            }
        }
    }
}

fn text(frame: &mut [u8], (fw, fh): (u32, u32), x: i32, y: i32, s: &str, c: [u8; 3]) {
    crate::hud::draw_text_crisp(frame, (fw, fh), (x / 2, y / 2), s, [c[0], c[1], c[2], 255]);
}

fn text_px(s: &str) -> i32 {
    crate::hud::text_width(s) * 2
}

const TEXT: [u8; 3] = [230, 230, 220];
const DIM: [u8; 3] = [130, 136, 132];
const HOT: [u8; 3] = [255, 214, 60];

/// The organ's own rows under the dish: the tools, **the bar** (one beat
/// drawn as a line, with the heart's thump at the left, a tick where each
/// bell rang, a hollow tick for each echo, the target ticks as ghosts, and
/// a playhead), the times, and the hint.
fn hud(o: &Organ, view: &super::render::View, scale: u32, frame: &mut [u8], (fw, fh): (u32, u32)) {
    use super::Tool;
    let d = &o.dish;
    let top = (H as u32 * scale) as i32;
    for y in top..fh as i32 {
        for x in 0..fw as i32 {
            put(frame, fw, x, y, [12, 14, 16]);
        }
    }
    // Line 1: the tools.
    let mut x = 6;
    for (k, t) in Tool::ALL.iter().enumerate() {
        let n = match t {
            Tool::Lamp => None,
            Tool::Oats => Some(d.oats_left),
            Tool::Salt => Some(d.salt_left),
            Tool::Snip => Some(d.snips_left),
            Tool::Water => Some(d.water_left),
        };
        let label = match n {
            Some(n) => format!("{} {} {}", k + 1, t.name(), n),
            None => format!("{} {}", k + 1, t.name()),
        };
        let label = if *t == view.tool { format!("[{label}]") } else { label };
        let c = if *t == view.tool {
            HOT
        } else if n == Some(0) {
            DIM
        } else {
            TEXT
        };
        text(frame, (fw, fh), x, top + 4, &label, c);
        x += text_px(&label) + 12;
    }
    let tools_end = x;
    if view.keys {
        let keys = if view.paused { "PAUSED  P: GO" } else { "T: TARGET  P: PAUSE  R: RESTART" };
        let w = text_px(keys);
        if x + 12 + w + 6 < fw as i32 {
            text(frame, (fw, fh), fw as i32 - w - 6, top + 4, keys, DIM);
        }
    }
    // Line 2: the bar.
    let (bx0, bx1) = (8i32, fw as i32 - 8);
    let by = top + 30;
    let bar_s = BEAT_STEPS as f32 / STEPS_PER_SECOND as f32;
    let at = |secs: f32| bx0 + ((bx1 - bx0) as f32 * (secs / bar_s).clamp(0.0, 1.0)) as i32;
    for xx in bx0..=bx1 {
        put(frame, fw, xx, by, [60, 64, 70]);
    }
    for q in 0..=8 {
        let xx = at(q as f32 * bar_s / 8.0);
        let hgt = if q % 2 == 0 { 3 } else { 1 };
        for yy in by - hgt..=by + hgt {
            put(frame, fw, xx, yy, [60, 64, 70]);
        }
    }
    if o.target {
        for b in 0..3 {
            let xx = at(TARGET[b]);
            let c = COLOURS[b + 1].map(|v| v / 3);
            for yy in by - 9..=by + 9 {
                for dx in -2i32..=2 {
                    if dx.abs() == 2 || yy == by - 9 || yy == by + 9 {
                        put(frame, fw, xx + dx, yy, c);
                    }
                }
            }
        }
    }
    for n in &o.bar {
        // A note past the end of the bar rings in the next one: drawn where
        // it lands there, shorter.
        let next = n.after >= bar_s;
        let xx = at(n.after % bar_s);
        let c = n.voice.colour();
        let lit = n.step < d.t;
        let hgt = if matches!(n.voice, Voice::Heart) {
            9
        } else if next {
            4
        } else {
            7
        };
        for yy in by - hgt..=by + hgt {
            for dx in -1..=1 {
                let edge = dx != 0 || yy == by - hgt || yy == by + hgt;
                if n.echo && !edge {
                    continue;
                }
                let c = if lit { c } else { c.map(|v| v / 2) };
                put(frame, fw, xx + dx, yy, c);
            }
        }
    }
    let ph = at(o.since_beat());
    for yy in by - 11..=by + 11 {
        put(frame, fw, ph, yy, [240, 240, 240]);
    }
    // Line 3: when each bell rings this bar, in its colour.
    let mut x = 6;
    let y3 = top + 46;
    for b in 0..3 {
        let first = o.bar.iter().find(|n| n.voice == Voice::Bell(b) && !n.echo);
        let echo = o.bar.iter().find(|n| n.voice == Voice::Bell(b) && n.echo);
        let s = match (first, echo) {
            (Some(f), Some(e)) => format!("{} {:.2}/{:.2}S", NOTE_NAMES[b], f.after, e.after),
            (Some(f), None) => format!("{} {:.2}S", NOTE_NAMES[b], f.after),
            (None, _) => format!("{} SILENT", NOTE_NAMES[b]),
        };
        text(frame, (fw, fh), x, y3, &s, COLOURS[b + 1]);
        x += text_px(&s) + 14;
    }

    // Line 4: what a click does.
    let hint = match view.tool {
        Tool::Lamp => "LAMP: PUSH A TUBE, ITS NOTE COMES LATE",
        Tool::Oats => "OATS: THE MOULD REACHES FOR THEM",
        Tool::Salt => "SALT: A WALL THE MOULD GROWS ROUND",
        Tool::Snip => "SNIP: CUT A TUBE",
        Tool::Water => "WATER: WAKES SLEEPING MOULD",
    };
    text(frame, (fw, fh), 6, top + 62, hint, DIM);
    if o.target {
        // Beside the hint if it fits (the window), else beside the tools
        // (the probe's half-size pictures, which show no keys).
        let s = format!("ON BEAT {}/{}", o.counts.on_beat, o.counts.on_beat + o.counts.off_beat);
        let w = text_px(&s);
        if 6 + text_px(hint) + 16 + w + 6 < fw as i32 {
            text(frame, (fw, fh), fw as i32 - w - 6, top + 62, &s, TEXT);
        } else if !view.keys && tools_end + 12 + w + 6 < fw as i32 {
            text(frame, (fw, fh), fw as i32 - w - 6, top + 4, &s, TEXT);
        }
    }
}

// ----------------------------------------------------------------- the sound

/// **The organ's voices**: a struck bar for each bell (a sine with a fourth
/// partial that dies fast, like a marimba) and a low falling thump for the
/// heart. Pure and allocation-free after `new`, so the window's sound thread
/// and the offline WAV render are the same code. The lab's synth
/// (`audio::synth`) was not reused: it has no voice whose pitch a caller
/// chooses -- its tones are fixed alerts -- and adding one would edit a
/// shared file.
pub struct Voices {
    sr: f32,
    live: Vec<Ringing>,
}

#[derive(Clone, Copy)]
struct Ringing {
    hz: f32,
    gain: f32,
    pan: f32,
    /// Samples since it was struck.
    age: u32,
    heart: bool,
    phase: [f32; 2],
}

const MAX_VOICES: usize = 24;

impl Voices {
    pub fn new(sample_rate: u32) -> Self {
        Voices { sr: sample_rate as f32, live: Vec::with_capacity(MAX_VOICES) }
    }

    pub fn strike(&mut self, n: &Note) {
        if self.live.len() >= MAX_VOICES {
            // Steal the oldest.
            let k = (0..self.live.len()).max_by_key(|&k| self.live[k].age).unwrap_or(0);
            self.live.swap_remove(k);
        }
        let (hz, heart) = match n.voice {
            Voice::Heart => (HEART_HZ, true),
            Voice::Bell(b) => (NOTES[b], false),
        };
        self.live.push(Ringing { hz, gain: n.gain, pan: n.pan, age: 0, heart, phase: [0.0; 2] });
    }

    /// Interleaved samples, added into `out` (which the caller zeroes).
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        let channels = channels.max(1);
        let dt = 1.0 / self.sr;
        for frame in out.chunks_mut(channels) {
            let mut l = 0.0;
            let mut r = 0.0;
            for v in self.live.iter_mut() {
                let t = v.age as f32 * dt;
                let s = if v.heart {
                    // A kick: pitch falls by half over 80 ms, gone by 300.
                    let hz = v.hz * (1.0 + (-t / 0.03).exp());
                    v.phase[0] = (v.phase[0] + hz * dt).fract();
                    let env = (-t / 0.09).exp() * (t / 0.003).min(1.0);
                    (v.phase[0] * TAU).sin() * env * 0.9
                } else {
                    v.phase[0] = (v.phase[0] + v.hz * dt).fract();
                    v.phase[1] = (v.phase[1] + v.hz * 3.93 * dt).fract();
                    let attack = (t / 0.002).min(1.0);
                    let body = (v.phase[0] * TAU).sin() * (-t / 0.55).exp();
                    let tine = (v.phase[1] * TAU).sin() * (-t / 0.06).exp() * 0.45;
                    (body + tine) * attack * 0.5
                } * v.gain;
                l += s * (1.0 - v.pan) * 0.5;
                r += s * (1.0 + v.pan) * 0.5;
                v.age += 1;
            }
            if channels == 1 {
                frame[0] += (l + r).tanh();
            } else {
                frame[0] += l.tanh();
                frame[1] += r.tanh();
            }
        }
        let gone = (self.sr * 2.5) as u32;
        self.live.retain(|v| v.age < gone);
    }
}

/// The notes of a run as a stereo WAV body: `seconds` long, interleaved.
pub fn render_notes(notes: &[Note], seconds: f32, sample_rate: u32) -> Vec<f32> {
    let total = (seconds * sample_rate as f32) as usize;
    let mut out = vec![0.0f32; total * 2];
    let mut v = Voices::new(sample_rate);
    let mut k = 0;
    let block = 256;
    let mut at = 0;
    while at < total {
        let end = (at + block).min(total);
        while k < notes.len() {
            let s = (notes[k].step as f64 / STEPS_PER_SECOND as f64 * sample_rate as f64) as usize;
            if s >= end {
                break;
            }
            // Render up to the note's own sample, then strike it there.
            if s > at {
                v.render(&mut out[at * 2..s * 2], 2);
                at = s;
            }
            v.strike(&notes[k]);
            k += 1;
        }
        v.render(&mut out[at * 2..end * 2], 2);
        at = end;
    }
    out
}

// ------------------------------------------------------------ scripted hands

/// **The scripted hand that joins the bells**: oats only, one at a time,
/// part-way from the nearest mould to the nearest bell the pulse cannot
/// reach yet -- the oats branch of `play::Plan`, aimed at bells. It stops
/// touching the dish once every bell is joined, which is what step 0 needs:
/// a joined dish, then nobody's hand on it.
pub struct Joiner {
    next_think: u64,
    dist: Vec<u16>,
}

impl Joiner {
    pub fn new() -> Self {
        Joiner { next_think: 0, dist: Vec::new() }
    }

    /// Bells a pulse can reach now.
    pub fn joined(&mut self, d: &Dish) -> [bool; 3] {
        heart_distance(d, u16::MAX - 1, &mut self.dist);
        [1, 2, 3].map(|b| bell_distance(d, &self.dist, b).is_some())
    }

    /// One look at the dish; true once every bell is joined.
    pub fn act(&mut self, d: &mut Dish) -> bool {
        if d.t < self.next_think {
            return false;
        }
        self.next_think = d.t + STEPS_PER_SECOND as u64 / 2;
        let joined = self.joined(d);
        if joined.iter().all(|&j| j) {
            return true;
        }
        let live_oats = d.piles.iter().filter(|p| p.oats).count();
        if d.oats_left == 0 || live_oats > 0 {
            return false;
        }
        // (gap to the bell's rim, nearest walker, the bell)
        type Lead = (f32, (f32, f32), (f32, f32));
        let mut best: Option<Lead> = None;
        for (k, b) in (1..4).enumerate() {
            if joined[k] {
                continue;
            }
            let p = &d.piles[b];
            for w in d.walkers.iter().step_by(5) {
                let g = (w.x - p.x).hypot(w.y - p.y) - p.r;
                if best.is_none_or(|bb| g < bb.0) {
                    best = Some((g, (w.x, w.y), (p.x, p.y)));
                }
            }
        }
        if let Some((g, (wx, wy), (px, py))) = best {
            let f = (22.0 / (g + 6.0)).min(1.0);
            d.place_oats(wx + (px - wx) * f, wy + (py - wy) * f);
        }
        false
    }
}

impl Default for Joiner {
    fn default() -> Self {
        Self::new()
    }
}

/// The middle of a bell's tube and the unit vector across it, pointing away
/// from the heart: where a hand puts the lamp or a salt wall.
pub fn tube_middle(d: &Dish, bell: usize) -> Option<((f32, f32), (f32, f32))> {
    let mut dist = Vec::new();
    heart_distance(d, 2000, &mut dist);
    let route = route_to_bell(d, &dist, bell);
    let n = route.len();
    if n < 8 {
        return None;
    }
    let xy = |i: usize| ((i % W) as f32 + 0.5, (i / W) as f32 + 0.5);
    let (mx, my) = xy(route[n / 2]);
    let (ax, ay) = xy(route[n / 2 - 6.min(n / 2)]);
    let (bx, by) = xy(route[(n / 2 + 6).min(n - 1)]);
    let (tx, ty) = (bx - ax, by - ay);
    let tl = tx.hypot(ty).max(0.01);
    let (mut nx, mut ny) = (-ty / tl, tx / tl);
    let h = &d.piles[0];
    if (mx + nx - h.x).hypot(my + ny - h.y) < (mx - h.x).hypot(my - h.y) {
        nx = -nx;
        ny = -ny;
    }
    Some(((mx, my), (nx, ny)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ruler sees shape: a straight tube painted by hand, then the same
    /// tube with a sideways bow, must measure longer on the geodesic and the
    /// same on the step count (which is why the organ does not use it).
    #[test]
    fn a_bowed_tube_is_longer() {
        let mut d = new_dish(1);
        d.trail.fill(0.0);
        let (hx, hy) = (d.piles[0].x, d.piles[0].y);
        let (bx, by) = (d.piles[3].x, d.piles[3].y);
        let paint = |d: &mut Dish, bow: f32| {
            d.trail.fill(0.0);
            for k in 0..=400 {
                let f = k as f32 / 400.0;
                let (mut x, mut y) = (hx + (bx - hx) * f, hy + (by - hy) * f);
                let (nx, ny) = (-(by - hy), bx - hx);
                let nl = nx.hypot(ny);
                let off = bow * (f * std::f32::consts::PI).sin();
                x += nx / nl * off;
                y += ny / nl * off;
                for (ox, oy) in disc(3.0) {
                    if let Some(i) = cell(x + ox, y + oy) {
                        d.trail[i] = 10.0;
                    }
                }
            }
        };
        let mut g = Vec::new();
        let mut s = Vec::new();
        paint(&mut d, 0.0);
        heart_geodesic(&d, 1000.0, &mut g);
        heart_distance(&d, 1000, &mut s);
        let (g0, s0) = (bell_geodesic(&d, &g, 3).unwrap(), bell_distance(&d, &s, 3).unwrap());
        paint(&mut d, 18.0);
        heart_geodesic(&d, 1000.0, &mut g);
        heart_distance(&d, 1000, &mut s);
        let (g1, s1) = (bell_geodesic(&d, &g, 3).unwrap(), bell_distance(&d, &s, 3).unwrap());
        assert!(g1 > g0 + 4.0, "geodesic: straight {g0}, bowed {g1}");
        assert!(s1.abs_diff(s0) <= 2, "step count is blind to the bow: straight {s0}, bowed {s1}");
    }

    /// A loop rings twice; a single tube once.
    #[test]
    fn a_loop_echoes() {
        let mut d = new_dish(1);
        let h = (d.piles[0].x, d.piles[0].y);
        let b = (d.piles[1].x, d.piles[1].y);
        let line = |d: &mut Dish, a: (f32, f32), c: (f32, f32)| {
            for k in 0..=300 {
                let f = k as f32 / 300.0;
                let (x, y) = (a.0 + (c.0 - a.0) * f, a.1 + (c.1 - a.1) * f);
                for (ox, oy) in disc(2.5) {
                    if let Some(i) = cell(x + ox, y + oy) {
                        d.trail[i] = 10.0;
                    }
                }
            }
        };
        d.trail.fill(0.0);
        line(&mut d, h, b);
        let mut g = Vec::new();
        heart_geodesic(&d, 1000.0, &mut g);
        assert!(echo_route(&d, &g, 1, 1000.0).is_none(), "one tube, no echo");
        // A second, longer route: up from the heart, across, and down into
        // the bell from the far side, well clear of the first tube.
        let up = (h.0, h.1 - 50.0);
        let over = (b.0, b.1 - 25.0);
        line(&mut d, h, up);
        line(&mut d, up, over);
        line(&mut d, over, b);
        heart_geodesic(&d, 1000.0, &mut g);
        let first = bell_geodesic(&d, &g, 1).unwrap();
        let (second, _) = echo_route(&d, &g, 1, 1000.0).expect("the loop should echo");
        assert!(second >= first + ECHO_GAP, "first {first} second {second}");
    }

    #[test]
    fn same_seed_same_notes() {
        let mut a = Organ::new(5);
        let mut b = Organ::new(5);
        for _ in 0..(BEAT_STEPS * 4) {
            a.step();
            b.step();
        }
        assert_eq!(a.played.len(), b.played.len());
        for (x, y) in a.played.iter().zip(&b.played) {
            assert_eq!(x.step, y.step);
        }
    }
}
