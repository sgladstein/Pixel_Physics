//! **Follow a laid trail through open air** (per ant, or clones): the
//! harder sequel to the tunnel maze ([`super::Maze`]), asked for by the
//! owner (2026-10-22) as "no walls to block in the ants, but there is a
//! pheromone path and the ants get scored for how well they follow the
//! path."
//!
//! **The bed**: a climbing frame hung in air over a stone floor -- a
//! lattice of one-cell stone bars, a node every [`SX`] columns and [`SY`]
//! rows, every node joined to its four neighbours, each bar stopping a
//! couple of cells short of the node so the air is one open space and not a
//! grid of sealed boxes. An ant can stand on any side of any bar (footing
//! is any solid cell next to the head), so every node is a four-way choice
//! and nothing anywhere is a dead end or a wall: every way is open, and
//! only the trail says which one is the path. A short
//! stub runs in from the west to the first column; the ants start on it.
//!
//! **The path** runs from the stub to the last column, one column east at a
//! time, and at each column climbs or drops up to [`MAX_STEP`] rows first (or
//! goes straight on), so it is a staircase that turns at nearly every node.
//! A channel-B trail is stamped over the air next to the path's bars (the
//! cells a head reads from), rising towards the goal; the rest of the frame
//! carries none.
//!
//! **Mirrored**: odd layouts flip the path top to bottom, so a brain that
//! always climbs or always drops wins one side and loses the other, and the
//! pair scores its worse side.
//!
//! **Score per ant**: the mean over the run of its progress along the path,
//! counted only while its head is within [`NEAR`] cells of a path bar; a
//! frame off the path scores 0, so leaving the path costs exactly what was
//! gained on it. Progress is the arc position of the nearest path cell,
//! from 0 where the ant started to 1 at the goal, signed (walking back down
//! the stub is below 0). Parts: `along`, `reach` (best progress while on the
//! path), `end` (at the bell), `goal`, `on` (share of the run on the path),
//! `fell` (share of the run on the floor), `walked` (columns moved).
//!
//! Laying is banned (`EmitA`, `EmitB`) and the trail is frozen, as in the
//! tunnel maze.
use super::*;

pub struct OpenMaze {
    pub ants: usize,
    pub frames: u64,
    pub clones: bool,
    /// The trail rises towards the goal; off, it is the same strength along
    /// the whole path.
    pub ramp: bool,
    /// Every ant starts carrying a crop ([`super::maze::load_crops`]).
    pub laden: bool,
}

const W: i32 = 256;
const H: i32 = 112;
/// Columns and rows between lattice nodes.
pub const SX: i32 = 14;
pub const SY: i32 = 12;
/// Lattice nodes across and down.
const NI: i32 = 16;
const NJ: i32 = 7;
/// How far short of a node every bar stops, so the air round a node is open.
const GAP: i32 = 2;
/// Where the first node sits; the start stub runs from `STUB_X` to it.
const X0: i32 = 26;
const Y0: i32 = 14;
const STUB_X: i32 = 3;
/// The floor's top row: well below the frame, so a fall is a real loss.
const FLOOR: i32 = H - 4;
/// The most rows the path climbs or drops at one column.
pub const MAX_STEP: i32 = 2;
/// How near a path bar (Chebyshev, in cells) a head must be to count as on
/// the path: a head stands one cell off a bar, so this allows the body a
/// little room at a corner without crediting the next bar over.
pub const NEAR: i32 = 3;
const GOAL_CELLS: i32 = 4;

/// One drawn frame: which cells are bar, which bar cells are the path and
/// how far along it, and the arc position credited to every cell near it.
pub struct Plan {
    pub bar: Vec<bool>,
    /// Arc position (0 at the stub's west end) of each cell of the path --
    /// its bars, and the open node centres it passes through; `i32::MAX`
    /// elsewhere.
    pub path: Vec<i32>,
    /// For every cell within [`NEAR`] of the path, the arc position of the
    /// nearest path cell (ties to the farther along); `i32::MAX` elsewhere.
    pub near: Vec<i32>,
    /// The path's length in cells.
    pub len: i32,
    pub goal: (i32, i32),
    /// The stub's row; ants stand on top of it.
    pub stub_y: i32,
}

fn node(i: i32, j: i32) -> (i32, i32) {
    (X0 + i * SX, Y0 + j * SY)
}

impl Plan {
    pub fn at(&self, x: i32, y: i32) -> bool {
        (0..W).contains(&x) && (0..H).contains(&y) && self.bar[(y * W + x) as usize]
    }

    /// **Draw the frame and a path through it from a layout.** The rows the
    /// path visits come from the seed, so a pair shares them, and odd `k`
    /// flips them top to bottom.
    pub fn draw(layout: Layout) -> Plan {
        let mut geo = super::super::Rng(layout.seed ^ 0x09E4_A112);
        let flip = !layout.k.is_multiple_of(2);
        let mut bar = vec![false; (W * H) as usize];
        let put = |bar: &mut Vec<bool>, x: i32, y: i32| bar[(y * W + x) as usize] = true;
        // Every bar stops `GAP` cells short of the node at each end, so no
        // four bars close a box: in a side view a closed loop of stone is a
        // wall round the air inside it, and a full lattice was a grid of
        // sealed rooms (the first cut; the follower sat on the stub at
        // 0.02). Through the gap a head still has a bar end within one
        // cell at every corner, so it can go round or straight on.
        for j in 0..NJ {
            for i in 0..NI - 1 {
                let ((xa, y), (xb, _)) = (node(i, j), node(i + 1, j));
                for x in xa + GAP..=xb - GAP {
                    put(&mut bar, x, y);
                }
            }
        }
        for i in 0..NI {
            for j in 0..NJ - 1 {
                let ((x, ya), (_, yb)) = (node(i, j), node(i, j + 1));
                for y in ya + GAP..=yb - GAP {
                    put(&mut bar, x, y);
                }
            }
        }
        // The rows the path stands on at each column, drawn unflipped and
        // then mirrored, so the two sides of a pair are the same staircase.
        let mut rows = vec![1 + geo.below((NJ - 2) as usize) as i32];
        for _ in 1..NI {
            let j = *rows.last().expect("one row");
            let lo = (j - MAX_STEP).max(0);
            let hi = (j + MAX_STEP).min(NJ - 1);
            rows.push(lo + geo.below((hi - lo + 1) as usize) as i32);
        }
        if flip {
            for j in &mut rows {
                *j = NJ - 1 - *j;
            }
        }
        let stub_y = node(0, rows[0]).1;
        for x in STUB_X..=node(0, 0).0 - GAP {
            put(&mut bar, x, stub_y);
        }
        // The path's cells in order: the stub, then at each column the climb
        // or drop to that column's row, then east to the next column.
        let mut order: Vec<(i32, i32)> = (STUB_X..node(0, 0).0).map(|x| (x, stub_y)).collect();
        let mut at = node(0, rows[0]);
        order.push(at);
        for (i, &j) in rows.iter().enumerate() {
            let to = node(i as i32, j);
            while at.1 != to.1 {
                at.1 += (to.1 - at.1).signum();
                order.push(at);
            }
            if i + 1 < rows.len() {
                let east = node(i as i32 + 1, j);
                while at.0 != east.0 {
                    at.0 += 1;
                    order.push(at);
                }
            }
        }
        let goal = at;
        let mut path = vec![i32::MAX; (W * H) as usize];
        for (n, &(x, y)) in order.iter().enumerate() {
            path[(y * W + x) as usize] = n as i32;
        }
        // Credit every cell near the path with the nearest path cell's arc.
        let mut near = vec![i32::MAX; (W * H) as usize];
        let mut best = vec![i32::MAX; (W * H) as usize];
        for (n, &(px, py)) in order.iter().enumerate() {
            for dy in -NEAR..=NEAR {
                for dx in -NEAR..=NEAR {
                    let (x, y) = (px + dx, py + dy);
                    if !(0..W).contains(&x) || !(0..H).contains(&y) {
                        continue;
                    }
                    let i = (y * W + x) as usize;
                    let d = dx.abs().max(dy.abs());
                    if d < best[i] || (d == best[i] && n as i32 > near[i]) {
                        best[i] = d;
                        near[i] = n as i32;
                    }
                }
            }
        }
        Plan {
            bar,
            path,
            near,
            len: order.len() as i32 - 1,
            goal,
            stub_y,
        }
    }

    /// Arc position credited at a head, if it is on the path.
    pub fn arc(&self, (x, y): (i32, i32)) -> Option<i32> {
        if !(0..W).contains(&x) || !(0..H).contains(&y) {
            return None;
        }
        Some(self.near[(y * W + x) as usize]).filter(|&a| a != i32::MAX)
    }
}

impl Challenge for OpenMaze {
    fn name(&self) -> &'static str {
        "openmaze"
    }
    fn unit(&self) -> Unit {
        if self.clones {
            Unit::Colony
        } else {
            Unit::Ant
        }
    }
    fn ants(&self) -> usize {
        self.ants
    }
    fn frames(&self) -> u64 {
        self.frames
    }
    fn parts(&self) -> &'static [&'static str] {
        &["along", "reach", "end", "goal", "on", "fell", "walked"]
    }
    fn banned(&self) -> &'static [O] {
        TRAIL_BANNED
    }
    fn minds(&self) -> bool {
        true
    }
    fn mirrored(&self) -> bool {
        true
    }
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        if self.clones {
            let gs = vec![genomes[0]; self.ants];
            return vec![super::super::mean_score(&self.run_ants(layout, &gs, look))];
        }
        self.run_ants(layout, genomes, look)
    }
    fn marks(&self, layout: Layout, _w: &World) -> Vec<((i32, i32), [u8; 3])> {
        let p = Plan::draw(layout);
        vec![(p.goal, [255, 200, 0])]
    }
}

impl OpenMaze {
    /// The bed with its trail laid, and no ants yet.
    pub fn world(&self, layout: Layout) -> (World, Plan) {
        let plan = Plan::draw(layout);
        let mut w = World::new(Rect::new(0, 0, W - 1, H - 1));
        hold_training_world(&mut w);
        w.seed = layout.seed;
        for y in 0..H {
            for x in 0..W {
                if plan.at(x, y) || y >= FLOOR {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
        }
        // As the tunnel maze stamps it: half the plane's range at the peak,
        // give or take 30%, rising from a fraction of that at the start.
        let mut geo = super::super::Rng(layout.seed ^ 0x7EA1_1A70);
        let peak = pheromone::Scent::MAX as f32 * (0.35 + 0.3 * geo.unit());
        let base = 0.1 + 0.3 * geo.unit();
        let span = plan.len.max(1) as f32;
        // Over the air one cell from a path bar: where a head on the path
        // reads. The arc there is the nearest path cell's.
        for y in 0..H {
            for x in 0..W {
                if plan.at(x, y) || y >= FLOOR {
                    continue;
                }
                let next_to_path = (-1..=1).any(|dy| {
                    (-1..=1).any(|dx| {
                        let (nx, ny) = (x + dx, y + dy);
                        (0..W).contains(&nx) && (0..H).contains(&ny) && plan.path[(ny * W + nx) as usize] != i32::MAX
                    })
                });
                if !next_to_path {
                    continue;
                }
                let Some(a) = plan.arc((x, y)) else { continue };
                let t = if self.ramp { a as f32 / span } else { 1.0 };
                let amount = (base + (1.0 - base) * t) * peak;
                w.deposit_pheromone(Channel::B, x, y, amount as pheromone::Scent);
            }
        }
        (w, plan)
    }

    fn run_ants(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let (mut w, plan) = self.world(layout);
        let sp = w.species.id_of("ant").expect("ant");
        let mut def = w.species.get(sp).creature.clone().expect("creature");
        def.reproduce_threshold = 1.0e30;
        never_starve(&mut def);
        if layout.blind {
            def.home_bias = 0.0;
        }
        w.species.set_creature(sp, def);
        w.chooser = Some(creature::Chooser::Off);
        w.home_blind = layout.blind;

        let n = genomes.len();
        let head_y = plan.stub_y - 1;
        for i in 0..n {
            // On top of the stub, three cells apart.
            w.plant_ant(STUB_X + 2 + 3 * i as i32, head_y);
        }
        frame(&mut w, false);
        let ids = ants_of(&w, sp);
        assert_eq!(ids.len(), n, "every planted ant became an animal");
        if self.laden {
            super::maze::load_crops(&mut w, &ids);
        }
        let head = |w: &World, id: OrganismId| w.organism(id).and_then(|s| s.chain.first().copied());
        let mut a0 = vec![0; n];
        for (k, &id) in ids.iter().enumerate() {
            if super::super::is_mind(genomes[k].len()) {
                creature::mind::give(&mut w, id, arena_mind(genomes[k], self.verbs(), self.mind_emits()));
            } else {
                w.set_organism_genome(id, genomes[k][..super::super::SCORER].to_vec());
            }
            a0[k] = head(&w, id).and_then(|p| plan.arc(p)).unwrap_or(0);
        }
        let progress = |a: i32, a0: i32| (f64::from(a - a0) / f64::from((plan.len - a0).max(1))).clamp(-1.0, 1.0);
        let mut sum = vec![0.0f64; n];
        let mut reach = vec![0.0f64; n];
        let mut end = vec![0.0f64; n];
        let mut goal = vec![false; n];
        let mut on = vec![0u64; n];
        let mut fell = vec![0u64; n];
        let mut walked = vec![0i64; n];
        let mut last: Vec<Option<(i32, i32)>> = ids.iter().map(|&id| head(&w, id)).collect();
        for f in 1..self.frames {
            frame(&mut w, false);
            for (k, &id) in ids.iter().enumerate() {
                let Some(p) = head(&w, id) else { continue };
                let g = match plan.arc(p) {
                    Some(a) => {
                        on[k] += 1;
                        progress(a, a0[k])
                    }
                    None => 0.0,
                };
                sum[k] += g;
                reach[k] = reach[k].max(g);
                end[k] = g;
                if (p.0 - plan.goal.0).abs() < GOAL_CELLS && (p.1 - plan.goal.1).abs() < GOAL_CELLS {
                    goal[k] = true;
                }
                if p.1 >= FLOOR - 2 {
                    fell[k] += 1;
                }
                if let Some(q) = last[k] {
                    walked[k] += i64::from((p.0 - q.0).abs());
                }
                last[k] = Some(p);
            }
            look(f + 1, &w);
        }
        let len = (self.frames - 1) as f64;
        (0..n)
            .map(|k| {
                let along = sum[k] / len;
                Score {
                    fit: along,
                    parts: vec![
                        along,
                        reach[k],
                        end[k],
                        f64::from(u8::from(goal[k])),
                        on[k] as f64 / len,
                        fell[k] as f64 / len,
                        walked[k] as f64,
                    ],
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The arena's positive and negative control**: the hand-built
    /// follower (`mind::preset("follower")`) gets well along the path, and
    /// the same brain with its scent reads cut does not. If the trail stops
    /// being stamped, or the scoring stops crediting the path, the first
    /// half fails; if the frame lets an ant score without reading, the
    /// second does.
    #[test]
    fn the_follower_takes_the_open_path_and_blinded_it_does_not() {
        let arena = OpenMaze {
            ants: 4,
            frames: 6000,
            clones: false,
            ramp: true,
            laden: false,
        };
        let follower = super::super::super::start_mind(&arena, "follower");
        let blind = creature::mind::blinded(&follower);
        let layouts = super::super::super::holdout_layouts(2);
        // The worse side of the pair, as the rig scores it.
        let worse = |g: &[f32]| {
            layouts
                .iter()
                .map(|&l| {
                    let s = arena.run(l, &vec![g; arena.ants], &mut |_, _| {});
                    s.iter().map(|s| s.fit).sum::<f64>() / s.len() as f64
                })
                .fold(f64::INFINITY, f64::min)
        };
        let (seeing, blinded) = (worse(&follower), worse(&blind));
        // Measured 2026-10-22: 0.430 / 0.456 seeing, 0.003 / 0.003 blinded.
        assert!(seeing > 0.3, "the follower took the open path on both sides: {seeing:.3}");
        assert!(blinded < 0.05, "blinded, it cannot: {blinded:.3}");
    }

    /// The path is one connected run of cells along the bars from the stub
    /// to the goal, every step one cell, and the two sides of a pair mirror
    /// each other.
    #[test]
    fn the_path_is_a_walkable_staircase_and_a_pair_mirrors_it() {
        for layout in super::super::super::holdout_layouts(4) {
            let p = Plan::draw(layout);
            let mut cells: Vec<(i32, (i32, i32))> = (0..W * H)
                .filter(|&i| p.path[i as usize] != i32::MAX)
                .map(|i| (p.path[i as usize], (i % W, i / W)))
                .collect();
            cells.sort();
            assert_eq!(cells.len() as i32, p.len + 1);
            for pair in cells.windows(2) {
                let ((_, a), (_, b)) = (pair[0], pair[1]);
                assert_eq!((a.0 - b.0).abs() + (a.1 - b.1).abs(), 1, "a gap in the path at {a:?}");
                // A node's centre is open air, `GAP` from each bar end.
                let by_bar = (-GAP..=GAP).any(|dy| (-GAP..=GAP).any(|dx| p.at(b.0 + dx, b.1 + dy)));
                assert!(by_bar, "the path left the frame at {b:?}");
            }
        }
        let l = super::super::super::holdout_layouts(2);
        let (a, b) = (Plan::draw(l[0]), Plan::draw(l[1]));
        assert_eq!(a.len, b.len);
        assert_eq!(a.goal.1 - Y0, (NJ - 1) * SY - (b.goal.1 - Y0));
    }
}
