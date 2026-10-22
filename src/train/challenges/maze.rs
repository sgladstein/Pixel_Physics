//! **Follow a laid trail through a maze** (per ant, or clones).
//!
//! The trail rung ([`super::Trail`]) asks one question on a bare floor:
//! left or right, and the trail says which. A maze asks it again at every
//! fork, after corners and climbs the straight floor never has, so it tests
//! whether reading a trail survives the shapes a nest's galleries make.
//!
//! **The bed**: solid stone, with tunnels cut into it. A start corridor runs
//! east from a dead end; at each fork it meets a vertical shaft that goes
//! both up and down, and from the top and the bottom of the shaft a
//! corridor carries on east. One of the two goes on to the next fork; the
//! other is a dead end a little shorter than the way on. After the last
//! fork the way on ends at the goal. A channel-B trail is stamped over every
//! cell of the way through, from the start corridor to the goal, rising
//! towards the goal unless `ramp` is off; the dead ends carry none.
//!
//! **Mirrored**: odd layouts take the other branch at every fork, so an ant
//! that always climbs (or always drops) wins one side and loses the other,
//! and the pair scores its worse side. Gravity makes the two sides unequal on
//! purpose: dropping into a shaft is easier than climbing out of one, and
//! only reading the trail wins both.
//!
//! **Score per ant**: the mean over the run of its signed progress, `1 -
//! d / d0`, where `d` is the walking distance from its head to the goal
//! through the tunnels and `d0` that distance from where it started. Walking
//! into a dead end moves it away from the goal, so the score falls there by
//! itself: no rule says which branch is wrong, the tunnels do. Parts:
//! `reach` (best progress), `end` (progress at the bell), `goal` (got
//! within a few cells of the goal at all), `lost` (share of the run spent in
//! a dead end), `walked` (columns moved).
//!
//! Laying is banned (`EmitA`, `EmitB`), so the stamped trail is the only one,
//! and by default nothing decays or spreads. **`spread`** runs the plane's
//! own pass over the stamped trail for that many passes before the ants
//! start, at the plane's blend for channel B (`Pheromones::set_channel_diffuse`
//! if `blend` is set), and then freezes it -- what a trail laid some time ago
//! looks like by the time a recruit walks it. The plane knows nothing of
//! stone, so a spread trail also seeps into the rock and into a parallel
//! tunnel; **`walls`** tries the two other rules a plane could have
//! ([`Walls`]: rock as a sink, rock as a mirror) -- an experiment here, not a
//! property of the shipped plane.
use super::*;

pub struct Maze {
    pub ants: usize,
    pub frames: u64,
    pub clones: bool,
    /// Forks between the start and the goal.
    pub forks: usize,
    /// The trail rises towards the goal; off, it is the same strength
    /// everywhere on the way through and only says "this way" at a fork.
    pub ramp: bool,
    /// Plane passes run over the stamped trail before the ants start.
    pub spread: u32,
    /// Channel B's blend for those passes, if not the plane's own.
    pub blend: Option<f32>,
    /// How the spread meets the rock ([`Walls`]).
    pub walls: Walls,
}

/// Rows of open air in a corridor, above its floor.
const ROOM: i32 = 4;
/// Columns in a shaft.
const SHAFT: i32 = 3;
const W: i32 = 256;
const H: i32 = 112;
/// How near the goal's end an ant's head must get to count as there.
const GOAL_CELLS: i32 = 4;

/// What a cell of the bed is: rock, the way through, or a dead end.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Rock,
    Way,
    Dead,
}

/// One drawn maze: the cells, the walking distance to the goal from every
/// open cell, and where the ants stand.
pub struct Plan {
    pub kind: Vec<Kind>,
    pub to_goal: Vec<i32>,
    pub from_start: Vec<i32>,
    pub goal: (i32, i32),
    pub start_floor: i32,
    pub start_x: i32,
}

impl Plan {
    fn at(&self, x: i32, y: i32) -> Kind {
        if x < 0 || y < 0 || x >= W || y >= H {
            return Kind::Rock;
        }
        self.kind[(y * W + x) as usize]
    }

    /// **Draw a maze from a layout.** The geometry comes from the seed, so a
    /// pair shares it; the side taken at each fork from the seed too, flipped
    /// on odd `k`.
    pub fn draw(layout: Layout, forks: usize) -> Plan {
        let mut geo = super::super::Rng(layout.seed ^ 0x3A2E_0F0E);
        let mut kind = vec![Kind::Rock; (W * H) as usize];
        let mut cut = |x0: i32, y0: i32, x1: i32, y1: i32, k: Kind| {
            for y in y0.min(y1)..=y0.max(y1) {
                for x in x0.min(x1)..=x0.max(x1) {
                    if (0..W).contains(&x) && (0..H).contains(&y) {
                        let c = &mut kind[(y * W + x) as usize];
                        // The way through wins where a dead end's cut overlaps it.
                        if *c != Kind::Way {
                            *c = k;
                        }
                    }
                }
            }
        };
        let flip = !layout.k.is_multiple_of(2);
        let start_x = 4;
        let start_floor = H / 2 + 4;
        let mut floor = start_floor;
        let mut x = start_x + 24 + geo.below(8) as i32;
        // The start corridor, from its dead end to the first shaft.
        cut(start_x, floor - ROOM, x + SHAFT - 1, floor - 1, Kind::Way);
        for _ in 0..forks {
            let drop = 7 + geo.below(4) as i32;
            let up = geo.below(2) == 0;
            let up = up != flip;
            let (way, dead) = if up {
                (floor - drop, floor + drop)
            } else {
                (floor + drop, floor - drop)
            };
            let on = 30 + geo.below(14) as i32;
            let blind = on - 6 - geo.below(6) as i32;
            // The shaft: its half towards the way on is the way, the other
            // half a dead end.
            let (lo, hi) = (floor - drop - ROOM, floor + drop - 1);
            if up {
                cut(x, lo, x + SHAFT - 1, floor - 1, Kind::Way);
                cut(x, floor, x + SHAFT - 1, hi, Kind::Dead);
            } else {
                cut(x, floor - ROOM, x + SHAFT - 1, hi, Kind::Way);
                cut(x, lo, x + SHAFT - 1, floor - ROOM - 1, Kind::Dead);
            }
            cut(x + SHAFT, way - ROOM, x + SHAFT + on, way - 1, Kind::Way);
            cut(x + SHAFT, dead - ROOM, x + SHAFT + blind, dead - 1, Kind::Dead);
            floor = way;
            x += SHAFT + on;
        }
        // The last stretch, to the goal.
        let last = 16 + geo.below(10) as i32;
        cut(x, floor - ROOM, x + last, floor - 1, Kind::Way);
        let goal = (x + last, floor - 1);
        let to_goal = walk_from(&kind, &[goal], |_| true);
        let from_start = walk_from(&kind, &[(start_x, start_floor - 1)], |k| k == Kind::Way);
        Plan {
            kind,
            to_goal,
            from_start,
            goal,
            start_floor,
            start_x,
        }
    }

    /// Signed progress at a head: 1 at the goal, 0 where the ant started,
    /// below 0 farther away than that.
    fn progress(&self, (x, y): (i32, i32), d0: i32) -> f64 {
        let d = if self.at(x, y) == Kind::Rock {
            d0
        } else {
            self.to_goal[(y * W + x) as usize]
        };
        (1.0 - f64::from(d) / f64::from(d0.max(1))).clamp(-1.0, 1.0)
    }
}

/// Walking distance through open cells (4-neighbour) from `from`, over the
/// cells `ok` lets through; `i32::MAX` where none reaches.
fn walk_from(kind: &[Kind], from: &[(i32, i32)], ok: impl Fn(Kind) -> bool) -> Vec<i32> {
    let mut d = vec![i32::MAX; kind.len()];
    let mut q = std::collections::VecDeque::new();
    for &(x, y) in from {
        d[(y * W + x) as usize] = 0;
        q.push_back((x, y));
    }
    while let Some((x, y)) = q.pop_front() {
        let here = d[(y * W + x) as usize];
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if !(0..W).contains(&nx) || !(0..H).contains(&ny) {
                continue;
            }
            let i = (ny * W + nx) as usize;
            if kind[i] != Kind::Rock && ok(kind[i]) && d[i] == i32::MAX {
                d[i] = here + 1;
                q.push_back((nx, ny));
            }
        }
    }
    d
}

impl Challenge for Maze {
    fn name(&self) -> &'static str {
        "maze"
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
        &["along", "reach", "end", "goal", "lost", "walked"]
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
        // The goal, so a picture shows where the way ends.
        let p = Plan::draw(layout, self.forks);
        (0..GOAL_CELLS)
            .map(|i| ((p.goal.0 - i, p.goal.1 + 1), [255, 200, 0]))
            .collect()
    }
}

impl Maze {
    /// The bed, its trail laid (and spread, if asked), with no ants yet.
    pub fn world(&self, layout: Layout) -> (World, Plan) {
        let plan = Plan::draw(layout, self.forks);
        let mut w = World::new(Rect::new(0, 0, W - 1, H - 1));
        hold_training_world(&mut w);
        w.seed = layout.seed;
        for y in 0..H {
            for x in 0..W {
                if plan.at(x, y) == Kind::Rock {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
        }
        // Half the plane's range at the peak, give or take 30%, as the trail
        // rung stamps it, so the two arenas read alike.
        let mut geo = super::super::Rng(layout.seed ^ 0x7EA1_1A70);
        let peak = pheromone::Scent::MAX as f32 * (0.35 + 0.3 * geo.unit());
        let base = 0.1 + 0.3 * geo.unit();
        let span = plan.from_start[(plan.goal.1 * W + plan.goal.0) as usize].max(1) as f32;
        for y in 0..H {
            for x in 0..W {
                let d = plan.from_start[(y * W + x) as usize];
                if plan.at(x, y) != Kind::Way || d == i32::MAX {
                    continue;
                }
                let t = if self.ramp { d as f32 / span } else { 1.0 };
                let amount = (base + (1.0 - base) * t) * peak;
                w.deposit_pheromone(Channel::B, x, y, amount as pheromone::Scent);
            }
        }
        if self.spread > 0 {
            spread_trail(
                &mut w,
                &plan,
                self.spread,
                self.blend.unwrap_or(pheromone::DIFFUSE),
                self.walls,
            );
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
        let head_y = plan.start_floor - 1;
        for i in 0..n {
            // Three cells apart, from a few cells in from the start's dead end.
            w.plant_ant(plan.start_x + 3 + 3 * i as i32, head_y);
        }
        frame(&mut w, false);
        let ids = ants_of(&w, sp);
        assert_eq!(ids.len(), n, "every planted ant became an animal");
        let head = |w: &World, id: OrganismId| w.organism(id).and_then(|s| s.chain.first().copied());
        let mut d0 = vec![1; n];
        for (k, &id) in ids.iter().enumerate() {
            if super::super::is_mind(genomes[k].len()) {
                creature::mind::give(&mut w, id, arena_mind(genomes[k], self.verbs(), self.mind_emits()));
            } else {
                w.set_organism_genome(id, genomes[k][..super::super::SCORER].to_vec());
            }
            let (x, y) = head(&w, id).unwrap_or((plan.start_x, head_y));
            d0[k] = plan
                .to_goal
                .get((y * W + x) as usize)
                .copied()
                .filter(|&d| d != i32::MAX)
                .unwrap_or(1);
        }
        let mut sum = vec![0.0f64; n];
        let mut reach = vec![f64::MIN; n];
        let mut end = vec![0.0f64; n];
        let mut goal = vec![false; n];
        let mut lost = vec![0u64; n];
        let mut walked = vec![0i64; n];
        let mut last: Vec<Option<(i32, i32)>> = ids.iter().map(|&id| head(&w, id)).collect();
        for f in 1..self.frames {
            frame(&mut w, false);
            for (k, &id) in ids.iter().enumerate() {
                let Some(p) = head(&w, id) else { continue };
                let g = plan.progress(p, d0[k]);
                sum[k] += g;
                reach[k] = reach[k].max(g);
                end[k] = g;
                if (p.0 - plan.goal.0).abs() < GOAL_CELLS && (p.1 - plan.goal.1).abs() < GOAL_CELLS {
                    goal[k] = true;
                }
                if plan.at(p.0, p.1) == Kind::Dead {
                    lost[k] += 1;
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
                        reach[k].max(0.0),
                        end[k],
                        f64::from(u8::from(goal[k])),
                        lost[k] as f64 / len,
                        walked[k] as f64,
                    ],
                }
            })
            .collect()
    }
}

/// How a spread trail meets the rock.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Walls {
    /// As the plane does today: the 3x3 mean runs through stone as through air.
    Through,
    /// Scent that reaches stone is lost there (the plane's pass, then the
    /// rock wiped).
    Sink,
    /// Stone is not there for the mean: each open cell blends towards the
    /// mean of its open neighbours only, so nothing leaks and nothing is lost
    /// to the walls.
    Reflect,
}

/// **Run `passes` of the plane's blend and channel B's decay over the
/// stamped trail**, then put the result back as the frozen trail. A model of
/// [`pheromone::PheromonePlane`]'s pass in `f32` (no 1-unit floor, no
/// rounding), so the three wall rules differ only in the wall rule.
pub fn spread_trail(w: &mut World, plan: &Plan, passes: u32, blend: f32, walls: Walls) {
    let rho = pheromone::b_rho();
    let (wi, hi) = (W as usize, H as usize);
    let open: Vec<bool> = plan.kind.iter().map(|&k| k != Kind::Rock).collect();
    let mut v: Vec<f32> = (0..wi * hi)
        .map(|i| f32::from(w.pheromone_at(Channel::B, (i % wi) as i32, (i / wi) as i32)))
        .collect();
    let mut next = v.clone();
    for _ in 0..passes {
        for y in 0..hi {
            for x in 0..wi {
                let i = y * wi + x;
                let (mut sum, mut n) = (0.0f32, 0.0f32);
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        let inside = nx >= 0 && ny >= 0 && (nx as usize) < wi && (ny as usize) < hi;
                        let j = if inside {
                            ny as usize * wi + nx as usize
                        } else {
                            usize::MAX
                        };
                        match walls {
                            Walls::Reflect => {
                                if inside && open[j] {
                                    sum += v[j];
                                    n += 1.0;
                                }
                            }
                            _ => {
                                sum += if inside { v[j] } else { 0.0 };
                                n += 1.0;
                            }
                        }
                    }
                }
                let here = v[i];
                let mean = if n > 0.0 { sum / n } else { here };
                let mut out = (here + (mean - here) * blend) * (1.0 - rho);
                if walls != Walls::Through && !open[i] {
                    out = 0.0;
                }
                next[i] = out;
            }
        }
        std::mem::swap(&mut v, &mut next);
    }
    let mut p = pheromone::Pheromones::new(Rect::new(0, 0, W - 1, H - 1));
    for (i, &s) in v.iter().enumerate() {
        let s = s.round().clamp(0.0, f32::from(pheromone::Scent::MAX)) as pheromone::Scent;
        if s > 0 {
            p.deposit(Channel::B, (i % wi) as i32, (i / wi) as i32, s);
        }
    }
    w.pheromones = p;
}

/// **What the trail says, read by no brain**: over every cell of the way
/// through, whether the strongest open cell next to it (8-neighbour) is
/// nearer the goal by walking. Returns the share that points the right way,
/// the share with no stronger neighbour at all (a plateau or a peak), and
/// the share of way cells next to a dead end whose strongest neighbour is in
/// the dead end. A ramp stamped and left alone is the positive control: every
/// cell whose cross-section neighbours tie reads right through the ramp.
pub fn readability(w: &World, plan: &Plan) -> (f64, f64, f64) {
    let (mut cells, mut right, mut flat, mut fork, mut astray) = (0u32, 0u32, 0u32, 0u32, 0u32);
    for y in 0..H {
        for x in 0..W {
            if plan.at(x, y) != Kind::Way {
                continue;
            }
            cells += 1;
            let here = w.pheromone_at(Channel::B, x, y);
            let d = plan.to_goal[(y * W + x) as usize];
            let mut best: Option<(pheromone::Scent, i32, i32)> = None;
            let mut by_dead = false;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    if (dx, dy) == (0, 0) || plan.at(nx, ny) == Kind::Rock {
                        continue;
                    }
                    by_dead |= plan.at(nx, ny) == Kind::Dead;
                    let s = w.pheromone_at(Channel::B, nx, ny);
                    if s > here && best.is_none_or(|b| s > b.0) {
                        best = Some((s, nx, ny));
                    }
                }
            }
            if by_dead {
                fork += 1;
            }
            match best {
                None => flat += 1,
                Some((_, nx, ny)) => {
                    if plan.to_goal[(ny * W + nx) as usize] < d {
                        right += 1;
                    }
                    if by_dead && plan.at(nx, ny) == Kind::Dead {
                        astray += 1;
                    }
                }
            }
        }
    }
    let c = f64::from(cells.max(1));
    (
        f64::from(right) / c,
        f64::from(flat) / c,
        f64::from(astray) / f64::from(fork.max(1)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::creature::mind;

    fn maze() -> Maze {
        Maze {
            ants: 3,
            frames: 8000,
            clones: true,
            forks: 3,
            ramp: true,
            spread: 0,
            blend: None,
            walls: Walls::Through,
        }
    }

    /// **The arena's two controls, both ways round**: the hand-built
    /// follower (`mind::preset("follower")`) reaches the goal on both sides
    /// of a mirrored pair, and the same brain with its scent reads cut
    /// (`mind::blinded`) scores nothing on the worse side. If the trail went
    /// unstamped, or the score stopped reading the tunnels, the first fails;
    /// if a side could be won without the trail, the second does.
    #[test]
    fn the_follower_takes_the_trail_and_blinded_it_does_not() {
        let m = maze();
        let layouts = super::super::super::holdout_layouts(2);
        let g = super::super::super::start_mind(&m, "follower");
        let b = mind::blinded(&g);
        let worse = |g: &[f32]| {
            let v: Vec<Score> = layouts
                .iter()
                .map(|&l| m.run(l, &[g], &mut |_, _| {}).remove(0))
                .collect();
            m.combine(&v, &layouts).fit
        };
        let (seeing, blind) = (worse(&g), worse(&b));
        assert!(seeing > 0.4, "the follower took the trail on both sides: {seeing:.3}");
        assert!(blind < 0.1, "blinded, it cannot: {blind:.3}");
    }

    /// **The readability readout's controls**: a stamped ramp points the way
    /// at most cells and never into a dead end; a flat trail points nowhere.
    #[test]
    fn a_ramp_points_the_way_and_a_flat_trail_points_nowhere() {
        let layout = super::super::super::holdout_layouts(1)[0];
        let ramp = maze();
        let (w, plan) = ramp.world(layout);
        let (right, flat, astray) = readability(&w, &plan);
        assert!(
            right > 0.8 && flat < 0.01 && astray == 0.0,
            "ramp: {right} {flat} {astray}"
        );
        let level = Maze { ramp: false, ..maze() };
        let (w, plan) = level.world(layout);
        let (right, flat, _) = readability(&w, &plan);
        assert!(right == 0.0 && flat == 1.0, "flat: {right} {flat}");
    }

    /// **Rock that blocks the spread keeps the trail pointing the way**
    /// where a spread through rock does not: the finding the arena's
    /// `walls` switch exists for, pinned so a change to the model shows.
    #[test]
    fn a_spread_that_stops_at_rock_keeps_the_trail_readable() {
        let layout = super::super::super::holdout_layouts(1)[0];
        let read = |walls| {
            let (w, plan) = Maze {
                spread: 12,
                walls,
                ..maze()
            }
            .world(layout);
            readability(&w, &plan).0
        };
        let (through, reflect) = (read(Walls::Through), read(Walls::Reflect));
        assert!(reflect > through + 0.15, "through {through:.3}, reflect {reflect:.3}");
    }
}
