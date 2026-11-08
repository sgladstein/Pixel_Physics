//! **Challenge dishes: hand-made dishes with one question each.**
//!
//! Owner's first play (2026-11-08): the herding is fun, "but there needs to
//! be a challenge". The near-term-challenges brainstorm
//! (`/mnt/project-files/game-design/mould-game-brainstorm-2026-11-08/
//! near-term-challenges.md`) designed a run of eight dishes after the
//! lesson; these are the first three, each asking a different question:
//!
//! * **Behind the Wall** -- *where can the mould go?* The food is behind a
//!   salt wall with a gap at the bottom. Scent does not cross salt, so oats
//!   aimed straight at the food pull the mould into the wall; the way is
//!   down, through the gap and back up. Stars by how soon it gets there.
//! * **Beat the Ruler** -- *what shape should it take?* Three foods in a
//!   big triangle and a dotted ruler: the two shortest food-to-food sides.
//!   Building the ruler scores 1.0 at best; the only way under it is a
//!   meeting point in empty space, the owner's "little bridges where there
//!   wasn't even food" made the goal.
//! * **Running Dry** -- *when should it move?* Every pile is small and runs
//!   out; new food turns up further on every 20 s. Score: mould alive at
//!   the bell.
//!
//! A level is a plain description (`Level`), placed by hand with no seed
//! jitter, so a dish is a puzzle you can learn rather than a dice roll. The
//! dish seed only drives the mould's own randomness. `Dish::from_level`
//! builds it; `Dish::level_tick` scores it.
//!
//! **No storm in any of the three** (the brief left it per dish). Each dish
//! asks one question, and the storm asks a fourth: in Beat the Ruler the
//! winning shape is a tree, which the storm (aimed at the cut that costs
//! the most food) always splits, so it would punish the answer; in Running
//! Dry the only tube worth cutting is the finger to the next food, which
//! makes the score luck; Behind the Wall ends when the food is reached.
//! Storm Season, the brainstorm's dish 5, is where it belongs.
//!
//! Every star bar here is set from `mouldprobe level=N` with headroom
//! (`Reports/mould-probe-2026-11-07.md`, *Challenge dishes*): doing nothing
//! scores 0, a random hand at most 1, the scripted intended hand
//! (`play::Route`) 3.

use super::{Dish, Phase, Score, Segment, Tool, STEPS_PER_SECOND};

/// A food pile a level places by hand.
#[derive(Clone, Copy, Debug)]
pub struct PileSpec {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub food: f32,
    /// Seconds into the dish it turns up (the dotted ring shows 5 s
    /// before), or `None` for there from the start.
    pub arrives: Option<u32>,
}

/// What the player is given. A tool with a stock of 0 is not on the bar.
#[derive(Clone, Copy, Debug)]
pub struct Tools {
    pub oats: u32,
    /// Seconds per oat coming back, up to `oats`; 0 never.
    pub oats_regrow: u32,
    pub salt: u32,
    pub snips: u32,
}

/// How a level is won. Stars are tiers: 1, 2 or 3, each needing the last.
#[derive(Clone, Copy, Debug)]
pub enum Win {
    /// Join every pile. One star for doing it at all, two by `by[0]`
    /// seconds, three by `by[1]`. The dish ends the moment it is done.
    JoinAll { by: [u32; 2] },
    /// Join every pile, with less tube than the ruler (the shortest tree of
    /// straight food-to-food tubes). Read at the bell: one star joined, two
    /// at or under `bars[0]` times the ruler, three at or under `bars[1]`.
    UnderRuler { bars: [f32; 2] },
    /// Mould alive at the bell: one, two, three stars at these walkers.
    AliveAtBell { bars: [u32; 3] },
}

/// One hand-made dish.
#[derive(Debug)]
pub struct Level {
    /// The word `mould-progress.txt` stores its stars under.
    pub key: &'static str,
    pub name: &'static str,
    /// The goal, on the briefing card and the picker: ~40 characters a line.
    pub goal: [&'static str; 2],
    /// The one thing that makes this dish different, for the card.
    pub twist: &'static str,
    /// Home first: the mould starts on it.
    pub piles: &'static [PileSpec],
    /// Salt the level lays before the mould starts.
    pub salt: &'static [Segment],
    pub tools: Tools,
    /// Seconds to the bell.
    pub seconds: u32,
    pub win: Win,
}

const fn pile(x: f32, y: f32, r: f32, food: f32) -> PileSpec {
    PileSpec {
        x,
        y,
        r,
        food,
        arrives: None,
    }
}

const fn later(x: f32, y: f32, food: f32, at: u32) -> PileSpec {
    PileSpec {
        x,
        y,
        r: 6.0,
        food,
        arrives: Some(at),
    }
}

/// `Params::stray_reach` on every hand-made dish.
pub const STRAY_REACH: f32 = 24.0;

/// Running Dry's piles: small, about 35 s of food at a full table.
pub const DRY_FOOD: f32 = 5_000.0;

/// The challenge dishes, in the order they unlock after the lesson.
pub static DISHES: [Level; 3] = [
    Level {
        key: "wall",
        name: "BEHIND THE WALL",
        goal: ["REACH THE FOOD BEHIND THE SALT.", "SCENT CAN'T CROSS SALT: GO ROUND."],
        twist: "THE WALL HAS ONE GAP, AT THE BOTTOM.",
        // Placed from the first run: at the brainstorm's (40,80) and (210,80) with
        // the gap below y = 125, the way round was 207 cells, and a relay of
        // oats (each drains in about 3 s once eaten) died out after five oats
        // had carried it 98 cells: the intended hand never arrived on 2 of 2
        // seeds. Closer piles and a taller gap make it 157 cells: home's own
        // 70 and four oats.
        piles: &[pile(62.0, 80.0, 7.0, 20_000.0), pile(194.0, 80.0, 6.0, 20_000.0)],
        // From the top rim down to y = 112: the gap is the bottom 45 cells.
        salt: &[((128.0, 0.0), (128.0, 112.0))],
        tools: Tools {
            oats: 5,
            oats_regrow: 15,
            salt: 0,
            snips: 0,
        },
        seconds: 90,
        win: Win::JoinAll { by: [60, 45] },
    },
    Level {
        key: "ruler",
        name: "BEAT THE RULER",
        goal: ["JOIN ALL THREE WITH LESS TUBE", "THAN THE DOTTED RULER."],
        twist: "THE RULER JOINS FOOD TO FOOD. MEET BETWEEN.",
        piles: &[
            pile(63.0, 134.0, 6.0, 20_000.0),
            pile(128.0, 22.0, 6.0, 20_000.0),
            pile(193.0, 134.0, 6.0, 20_000.0),
        ],
        salt: &[],
        tools: Tools {
            oats: 5,
            oats_regrow: 15,
            salt: 0,
            snips: 0,
        },
        seconds: 100,
        // Set from the probe: the intended hand's meeting point reads
        // 0.87-0.92x (13 runs), networks built along the sides 0.99-1.03x, and
        // doing nothing, when strays could still claim the far foods, 0.99-
        // 1.00x. So 2 stars needs some meeting point (0.98), not the ruler.
        win: Win::UnderRuler { bars: [0.98, 0.95] },
    },
    Level {
        key: "dry",
        name: "RUNNING DRY",
        goal: ["KEEP THE MOULD ALIVE.", "THE FOOD WON'T LAST."],
        twist: "NEW FOOD EVERY 20 S, FURTHER ON EACH TIME.",
        piles: &[
            pile(32.0, 80.0, 7.0, DRY_FOOD),
            pile(50.0, 54.0, 6.0, DRY_FOOD),
            pile(50.0, 106.0, 6.0, DRY_FOOD),
            later(135.0, 32.0, DRY_FOOD, 20),
            later(140.0, 122.0, DRY_FOOD, 40),
            later(220.0, 75.0, DRY_FOOD, 60),
            later(150.0, 62.0, DRY_FOOD, 80),
            later(205.0, 128.0, DRY_FOOD, 100),
        ],
        salt: &[],
        tools: Tools {
            oats: 5,
            oats_regrow: 10,
            salt: 0,
            snips: 0,
        },
        seconds: 120,
        // Set from the probe (5 seeds): the intended hand ends with 354-411
        // alive, the simple plan 0-261, random and doing nothing 0. The
        // brainstorm's 300/600/900 (three fed piles at the bell) is out of
        // reach: the bell always finds the mould between piles.
        win: Win::AliveAtBell { bars: [100, 200, 300] },
    },
];

impl Level {
    /// The mould's numbers on a hand-made dish: the defaults, with strays
    /// able to start on food only near fed mould (`Params::stray_reach`),
    /// so the leash on the card is the leash in the dish.
    pub fn params() -> super::Params {
        super::Params {
            stray_reach: STRAY_REACH,
            ..super::Params::default()
        }
    }

    /// The stars line for the HUD and the card: what each star needs.
    pub fn stars_line(&self) -> String {
        match self.win {
            Win::JoinAll { by } => format!("STARS: REACH IT. BY {}. BY {}", clock(by[0]), clock(by[1])),
            Win::UnderRuler { bars } => format!("STARS: JOIN ALL 3. {:.2}X RULER. {:.2}X", bars[0], bars[1]),
            Win::AliveAtBell { bars } => format!("STARS: {}, {} AND {} MOULD AT THE BELL", bars[0], bars[1], bars[2]),
        }
    }

    pub fn has(&self, t: Tool) -> bool {
        match t {
            Tool::Lamp => true,
            Tool::Oats => self.tools.oats > 0,
            Tool::Salt => self.tools.salt > 0,
            Tool::Snip => self.tools.snips > 0,
        }
    }

    /// The ruler: the shortest tree of straight rim-to-rim tubes over every
    /// pile, as (length in cells, its edges by pile index). The same tree
    /// and the same rim-to-rim lengths `Dish::measure` scores thrift with.
    pub fn ruler(&self) -> (f32, Vec<(usize, usize)>) {
        let p = self.piles;
        let n = p.len();
        let gap = |a: usize, b: usize| ((p[a].x - p[b].x).hypot(p[a].y - p[b].y) - p[a].r - p[b].r).max(1.0);
        let mut inside = vec![false; n];
        let mut best: Vec<(f32, usize)> = vec![(f32::INFINITY, 0); n];
        let (mut total, mut edges) = (0.0, Vec::new());
        if n == 0 {
            return (0.0, edges);
        }
        best[0] = (0.0, 0);
        for _ in 0..n {
            let u = (0..n)
                .filter(|&v| !inside[v])
                .min_by(|&a, &b| best[a].0.total_cmp(&best[b].0))
                .expect("a pile left");
            inside[u] = true;
            if u != 0 {
                total += best[u].0;
                edges.push((best[u].1, u));
            }
            for v in 0..n {
                if !inside[v] && gap(u, v) < best[v].0 {
                    best[v] = (gap(u, v), u);
                }
            }
        }
        (total, edges)
    }

    /// The best possible meeting point (the Fermat point, by Weiszfeld's
    /// iteration over the pile centres) and that network's length over the
    /// ruler: the "perfect" answer drawn faintly after Beat the Ruler.
    pub fn meeting_point(&self) -> ((f32, f32), f32) {
        let p = self.piles;
        let n = p.len().max(1) as f32;
        let mut at = (
            p.iter().map(|q| q.x).sum::<f32>() / n,
            p.iter().map(|q| q.y).sum::<f32>() / n,
        );
        for _ in 0..100 {
            let (mut sx, mut sy, mut sw) = (0.0, 0.0, 0.0);
            for q in p {
                let d = (q.x - at.0).hypot(q.y - at.1).max(0.01);
                sx += q.x / d;
                sy += q.y / d;
                sw += 1.0 / d;
            }
            at = (sx / sw, sy / sw);
        }
        // From one cell outside each rim, as `Dish::best_join` measures.
        let len: f32 = p
            .iter()
            .map(|q| ((q.x - at.0).hypot(q.y - at.1) - q.r - 1.0).max(0.0))
            .sum();
        let ruler = self.ruler().0.max(1.0);
        (at, len / ruler)
    }
}

/// Seconds as M:SS.
pub fn clock(s: u32) -> String {
    format!("{}:{:02}", s / 60, s % 60)
}

/// The score card's words: an amount, and one line of why, in world words,
/// picked by the condition that failed.
#[derive(Clone, Debug, Default)]
pub struct Card {
    pub stars: u32,
    /// The graded number, said plainly ("REACHED THE FOOD AT 0:41").
    pub amount: String,
    /// What to do differently, or what went right.
    pub why: String,
}

/// The why line for a seeded (free) dish or the lesson, from its score.
pub fn free_card(s: &Score) -> Card {
    let networked = s.reach >= super::THRIFT_MIN_JOINED;
    let why = if s.reach < s.piles {
        let n = s.piles - s.reach;
        format!(
            "{n} FOOD PILE{} NEVER JOINED: OATS LEAD THE WAY",
            if n == 1 { "" } else { "S" }
        )
    } else if networked && !s.stars[1] {
        format!("NETWORK {:.2}X THE SHORTEST: SNIP THE SPURS", s.ratio())
    } else if networked && !s.stars[2] {
        let lost = s.reach - s.after;
        format!(
            "THE STORM CUT OFF {lost} PILE{}: A LOOP HOLDS",
            if lost == 1 { "" } else { "S" }
        )
    } else if !networked {
        "TOO FEW PILES JOINED TO SCORE SHAPE".to_string()
    } else {
        "JOINED, SHORT AND STORM-PROOF.".to_string()
    };
    Card {
        stars: s.star_count(),
        amount: String::new(),
        why,
    }
}

impl Dish {
    /// A hand-made dish. `seed` drives only the mould's own randomness (the
    /// inoculum, the walkers' turns); the layout is the level's.
    pub fn from_level(level: &'static Level, seed: u64) -> Dish {
        Self::from_level_with(level, seed, Level::params())
    }

    /// `from_level` with the mould's numbers given, for the probe.
    pub fn from_level_with(level: &'static Level, seed: u64, params: super::Params) -> Dish {
        let mut rng = super::Rng::new(seed);
        // One draw, so a level's seed stream is never the same as a free
        // dish's of the same number.
        rng.next_u64();
        let mut ground = super::agar_ground();
        for &((ax, ay), (bx, by)) in level.salt {
            lay_salt(&mut ground, level, (ax, ay), (bx, by));
        }
        let make = |s: &PileSpec| super::Pile {
            x: s.x,
            y: s.y,
            r: s.r,
            food: s.food,
            full: s.food,
            oats: false,
            expires: u64::MAX,
            eaters: 0,
        };
        let sps = STEPS_PER_SECOND as u64;
        let piles = level.piles.iter().filter(|s| s.arrives.is_none()).map(make).collect();
        let pending = level
            .piles
            .iter()
            .filter_map(|s| s.arrives.map(|at| (at as u64 * sps, make(s))))
            .collect();
        let mut d = Dish::assemble(seed, params, rng, ground, piles, pending);
        d.level = Some(level);
        d.oats_stock = level.tools.oats;
        d.oats_left = level.tools.oats;
        d.oats_regrow = level.tools.oats_regrow;
        d.salt_left = level.tools.salt;
        d.snips_left = level.tools.snips;
        d.storm_at = level.seconds as u64 * sps;
        d
    }

    /// Whether this dish puts tool `t` on the bar at all.
    pub fn has_tool(&self, t: Tool) -> bool {
        self.level.is_none_or(|l| l.has(t))
    }

    /// The level's tube over its ruler, right now: Beat the Ruler's live
    /// number. `None` on any other dish.
    pub fn ruler_ratio(&self) -> Option<f32> {
        let l = self.level?;
        if !matches!(l.win, Win::UnderRuler { .. }) {
            return None;
        }
        self.best_join().map(|j| j / l.ruler().0.max(1.0))
    }

    /// **The shortest join through the tube**: the length of the shortest
    /// tree inside the mould's tube that joins every dish pile, rim to rim,
    /// or `None` while they are not all joined. For three piles that tree
    /// meets at one cell, so it is the least, over every tube cell, of the
    /// sum of the tube distances from it to the three piles; a ruler-shaped
    /// network scores its two sides (the cell is a pile's rim), a network
    /// with a junction in the middle scores the three spokes.
    ///
    /// This, not `Score::length`, is what Beat the Ruler scores. Length is
    /// tube area over `TUBE_WIDTH`, and area is set by how much food is
    /// joined (about 300 walkers a pile), not by shape: on the first run
    /// doing nothing read 1.40-1.42x and the intended three-spoke hand
    /// 1.39-1.75x, a spread three times the 13% a meeting point can win.
    /// Spurs, loops and fat tubes do not lengthen the shortest join, so
    /// the number answers the dish's one question: did you meet between?
    ///
    /// Distances are a 16-neighbour search (steps of 1, 1.41 and 2.24), so
    /// a straight tube at any angle reads within 3% of its true length.
    pub fn best_join(&self) -> Option<f32> {
        let piles: Vec<&super::Pile> = self.dish_piles().collect();
        if piles.len() < 2 || !self.pending.is_empty() {
            return None;
        }
        let fields: Vec<Vec<u32>> = piles.iter().map(|p| self.tube_distance(p)).collect();
        let mut best = u32::MAX;
        for i in 0..fields[0].len() {
            let mut sum = 0u32;
            for f in &fields {
                sum = sum.saturating_add(f[i]);
            }
            best = best.min(sum);
        }
        (best != u32::MAX).then(|| best as f32 / 100.0)
    }

    /// Tube distance, in hundredths of a cell, from a pile's rim to every
    /// tube cell (`u32::MAX` where no tube leads).
    fn tube_distance(&self, p: &super::Pile) -> Vec<u32> {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;
        const STEPS: [(i32, i32, u32); 16] = [
            (1, 0, 100),
            (-1, 0, 100),
            (0, 1, 100),
            (0, -1, 100),
            (1, 1, 141),
            (1, -1, 141),
            (-1, 1, 141),
            (-1, -1, 141),
            (2, 1, 224),
            (2, -1, 224),
            (-2, 1, 224),
            (-2, -1, 224),
            (1, 2, 224),
            (1, -2, 224),
            (-1, 2, 224),
            (-1, -2, 224),
        ];
        let (w, h) = (super::W as i32, super::H as i32);
        let tube = self.params.tube;
        let mut dist = vec![u32::MAX; super::W * super::H];
        let mut heap = BinaryHeap::new();
        for (ox, oy) in super::disc(p.r + 1.0) {
            if let Some(i) = super::cell(p.x + ox, p.y + oy) {
                if self.ground[i] == super::Ground::Agar {
                    dist[i] = 0;
                    heap.push(Reverse((0u32, i)));
                }
            }
        }
        while let Some(Reverse((d, i))) = heap.pop() {
            if d > dist[i] {
                continue;
            }
            let (x, y) = ((i % super::W) as i32, (i / super::W) as i32);
            for &(dx, dy, c) in &STEPS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let j = ny as usize * super::W + nx as usize;
                if self.ground[j] != super::Ground::Agar || self.trail[j] <= tube {
                    continue;
                }
                let nd = d + c;
                if nd < dist[j] {
                    dist[j] = nd;
                    heap.push(Reverse((nd, j)));
                }
            }
        }
        dist
    }

    /// Scoring a level, once a step from `phase_tick` while it grows: the
    /// level's own end in place of the storm.
    pub(super) fn level_tick(&mut self, level: &'static Level) {
        let bell = self.t + 1 >= self.storm_at;
        let sps = STEPS_PER_SECOND as u64;
        if matches!(level.win, Win::UnderRuler { .. }) && self.t.is_multiple_of(sps / 2) {
            self.join_now = self.ruler_ratio();
        }
        match level.win {
            Win::JoinAll { by } => {
                if !bell && !self.t.is_multiple_of(super::FED_EVERY) {
                    return;
                }
                let s = self.measure();
                let done = self.pending.is_empty() && s.piles >= 2 && s.reach == s.piles;
                if !done && !bell {
                    return;
                }
                let secs = (self.t / sps) as u32;
                if done {
                    let stars = 1 + (secs <= by[0]) as u32 + (secs <= by[1]) as u32;
                    let why = match stars {
                        3 => "STRAIGHT ROUND THE WALL. NOTHING WASTED.".to_string(),
                        2 => format!("FOR 3 STARS, REACH IT BY {}", clock(by[1])),
                        _ => format!("FOR 2 STARS, REACH IT BY {}", clock(by[0])),
                    };
                    self.finish(s, stars, format!("REACHED THE FOOD AT {}", clock(secs)), why)
                } else {
                    let gap = self.nearest_unjoined_gap();
                    let past = self.got_past_salt(level);
                    let why = if past {
                        "PAST THE WALL, BUT OUT OF REACH: LAY OATS".to_string()
                    } else {
                        "SCENT CAN'T CROSS SALT: LEAD IT TO THE GAP".to_string()
                    };
                    self.finish(s, 0, format!("NOT REACHED: THE MOULD CAME WITHIN {gap:.0} CELLS"), why)
                }
            }
            Win::UnderRuler { bars } => {
                if !bell {
                    return;
                }
                let s = self.measure();
                let joined = self.ruler_ratio();
                self.join_now = joined;
                let all = joined.is_some();
                let ratio = joined.unwrap_or(0.0);
                let stars = if all {
                    1 + (ratio <= bars[0]) as u32 + (ratio <= bars[1]) as u32
                } else {
                    0
                };
                let (_, perfect) = level.meeting_point();
                let amount = if all {
                    format!("ALL 3 JOINED, {ratio:.2}X THE RULER")
                } else {
                    format!("{} OF {} FOODS JOINED AT THE BELL", s.reach, s.piles)
                };
                let why = match stars {
                    0 => "MEET IN THE MIDDLE, THEN BRANCH OUT".to_string(),
                    1 => format!("{ratio:.2}X: TRY MEETING IN THE MIDDLE"),
                    2 => format!("UNDER {:.2}X FOR 3: MEET NEARER THE MIDDLE", bars[1]),
                    _ => format!("A BRIDGE WHERE NO FOOD WAS. BEST {perfect:.2}X"),
                };
                self.finish(s, stars, amount, why)
            }
            Win::AliveAtBell { bars } => {
                if !bell {
                    return;
                }
                let s = self.measure();
                let alive = self.walkers.len() as u32;
                let stars = bars.iter().filter(|&&b| alive >= b).count() as u32;
                let why = match stars {
                    3 => "IT KEPT MOVING HOUSE, AND IT LIVED".to_string(),
                    _ if alive == 0 => "THE FOOD RAN OUT; NOTHING HEADED FOR MORE".to_string(),
                    k => format!(
                        "{} FOR {} STAR{}: MOVE ON SOONER",
                        bars[k as usize],
                        k + 1,
                        if k == 0 { "" } else { "S" }
                    ),
                };
                self.finish(s, stars, format!("{alive} MOULD ALIVE AT THE BELL"), why)
            }
        }
    }

    fn finish(&mut self, s: Score, stars: u32, amount: String, why: String) {
        self.lamp = None;
        self.stroke = None;
        self.update_light();
        self.score = Some(Score {
            after: s.reach,
            stars: [stars >= 1, stars >= 2, stars >= 3],
            ..s
        });
        self.card = Some(Card { stars, amount, why });
        self.phase = Phase::Over;
    }

    /// Cells from the nearest walker to the rim of the nearest dish pile not
    /// joined to home: how close a failed reach came.
    fn nearest_unjoined_gap(&self) -> f32 {
        let mut best = f32::INFINITY;
        for p in self.dish_piles().skip(1).filter(|p| !p.reached()) {
            for w in &self.walkers {
                best = best.min(((w.x - p.x).hypot(w.y - p.y) - p.r).max(0.0));
            }
        }
        if best.is_finite() {
            best
        } else {
            0.0
        }
    }

    /// Whether any mould stands on the far side of the level's salt from
    /// home (the side test of the first salt line).
    fn got_past_salt(&self, level: &Level) -> bool {
        let Some(&((ax, ay), (bx, by))) = level.salt.first() else {
            return true;
        };
        let side = |x: f32, y: f32| (bx - ax) * (y - ay) - (by - ay) * (x - ax);
        let home = side(level.piles[0].x, level.piles[0].y);
        self.walkers.iter().any(|w| side(w.x, w.y) * home < 0.0)
    }
}

/// A level's salt, laid straight onto the ground: three cells wide like a
/// player's stroke, never on a pile.
fn lay_salt(ground: &mut [super::Ground], level: &Level, a: (f32, f32), b: (f32, f32)) {
    let len = (b.0 - a.0).hypot(b.1 - a.1);
    let n = (len.ceil() as usize).max(1);
    for k in 0..=n {
        let f = k as f32 / n as f32;
        let (x, y) = (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
        for (ox, oy) in super::disc(1.5) {
            let Some(i) = super::cell(x + ox, y + oy) else { continue };
            let (cx, cy) = ((i % super::W) as f32 + 0.5, (i / super::W) as f32 + 0.5);
            let on_pile = level.piles.iter().any(|p| (p.x - cx).hypot(p.y - cy) <= p.r);
            if ground[i] == super::Ground::Agar && !on_pile {
                ground[i] = super::Ground::Salt;
            }
        }
    }
}

// --------------------------------------------------------------- progress

/// Entries on the dish picker: the lesson, the dishes, then free play.
pub const ENTRIES: usize = DISHES.len() + 2;

/// Best stars per entry, saved to `mould-progress.txt` beside the window's
/// executable as one `key stars` line each. The lesson's line is 1 once it
/// has been finished. A missing or garbled file is a fresh start: a line
/// that does not parse is skipped, never a panic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// Lesson first, then `DISHES` in order.
    pub best: [u32; DISHES.len() + 1],
}

impl Progress {
    fn key(k: usize) -> &'static str {
        if k == 0 {
            "lesson"
        } else {
            DISHES[k - 1].key
        }
    }

    pub fn parse(text: &str) -> Progress {
        let mut p = Progress::default();
        for line in text.lines() {
            let mut words = line.split_whitespace();
            let (Some(key), Some(n)) = (words.next(), words.next()) else {
                continue;
            };
            let Ok(n) = n.parse::<u32>() else { continue };
            if let Some(k) = (0..p.best.len()).find(|&k| Self::key(k) == key) {
                p.best[k] = p.best[k].max(n.min(3));
            }
        }
        p
    }

    pub fn text(&self) -> String {
        (0..self.best.len())
            .map(|k| format!("{} {}\n", Self::key(k), self.best[k]))
            .collect()
    }

    /// Picker entry `e` is open: the lesson and free play always; a dish
    /// once the entry before it has a star (the lesson, once finished).
    pub fn unlocked(&self, e: usize) -> bool {
        e == 0 || e == ENTRIES - 1 || self.best.get(e - 1).is_some_and(|&b| b >= 1)
    }

    /// Record a result; true if it was a new best.
    pub fn record(&mut self, e: usize, stars: u32) -> bool {
        match self.best.get_mut(e) {
            Some(b) if stars > *b => {
                *b = stars.min(3);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mould::play::{Player, Route};

    fn play(level: usize, seed: u64, hand: &mut dyn Player) -> Dish {
        let mut d = Dish::from_level(&DISHES[level], seed);
        while d.phase != Phase::Over {
            hand.act(&mut d);
            for _ in 0..super::super::STEPS_PER_FRAME {
                d.step();
            }
        }
        d
    }

    /// The layout is the level's: the seed moves the mould, not the food.
    #[test]
    fn a_level_is_the_same_dish_on_every_seed() {
        for l in &DISHES {
            let (a, b) = (Dish::from_level(l, 1), Dish::from_level(l, 9));
            assert_eq!(a.ground, b.ground, "{}", l.name);
            let at = |d: &Dish| d.piles.iter().map(|p| (p.x, p.y, p.food)).collect::<Vec<_>>();
            assert_eq!(at(&a), at(&b));
            assert_eq!(a.pending.len(), b.pending.len());
            assert_eq!(a.oats_left, l.tools.oats);
            assert_eq!(a.snips_left, l.tools.snips);
        }
    }

    /// Cheap structural checks: the wall is salt with a gap, its home is on
    /// the open side, and the ruler is the two short sides.
    #[test]
    fn the_wall_has_a_gap_and_the_ruler_two_sides() {
        let d = Dish::from_level(&DISHES[0], 1);
        let at = |x: usize, y: usize| d.ground[y * super::super::W + x];
        assert_eq!(at(128, 60), super::super::Ground::Salt);
        assert_eq!(at(128, 140), super::super::Ground::Agar, "the gap");
        let (len, edges) = DISHES[1].ruler();
        assert_eq!(edges.len(), 2);
        let (_, perfect) = DISHES[1].meeting_point();
        assert!(
            len > 200.0 && perfect < 0.95 && perfect > 0.8,
            "ruler {len}, perfect {perfect}"
        );
    }

    fn score(level: usize, seed: u64, hand: &mut dyn Player) -> Card {
        play(level, seed, hand).card.expect("a dish ends with a card")
    }

    /// Doing nothing scores 0: the leash is shorter than the way round the
    /// wall, and strays cannot start on food far from fed mould. About 6 s
    /// in release (CI runs release).
    #[test]
    fn doing_nothing_scores_nothing_behind_the_wall() {
        let c = score(0, 1, &mut crate::mould::play::Idle);
        assert_eq!(c.stars, 0, "{} / {}", c.amount, c.why);
    }

    /// Doing nothing never joins the far two foods. About 7 s.
    #[test]
    fn doing_nothing_scores_nothing_beating_the_ruler() {
        let c = score(1, 1, &mut crate::mould::play::Idle);
        assert_eq!(c.stars, 0, "{} / {}", c.amount, c.why);
    }

    /// Running Dry left alone, the cheap structural half that CI runs: by
    /// 50 s the first two new piles have turned up, and doing nothing has
    /// eaten neither. (The whole dish, below, is about 20 s, so ignored.)
    #[test]
    fn doing_nothing_never_reaches_new_food_running_dry() {
        let mut d = Dish::from_level(&DISHES[2], 1);
        while d.t < 50 * STEPS_PER_SECOND as u64 {
            d.step();
        }
        let late: Vec<&super::super::Pile> = d.dish_piles().skip(3).collect();
        assert_eq!(late.len(), 2, "the 0:20 and 0:40 piles are out");
        for p in late {
            assert_eq!(p.food, p.full, "pile at ({}, {}) was eaten", p.x, p.y);
        }
    }

    #[test]
    #[ignore = "a whole 120 s dish, about 20 s in release: run with --ignored"]
    fn doing_nothing_scores_nothing_running_dry() {
        let c = score(2, 1, &mut crate::mould::play::Idle);
        assert_eq!(c.stars, 0, "{} / {}", c.amount, c.why);
    }

    /// The positive control: the intended hand takes 3 stars, so the zeros
    /// above are the dish, not a scorer that cannot pay out. Behind the Wall
    /// ends when the food is reached (about 1.5 s); the other two run to the
    /// bell, so they are ignored for CI's sake.
    #[test]
    fn the_intended_hand_takes_three_stars_behind_the_wall() {
        let c = score(0, 1, &mut Route::for_level(0));
        assert_eq!(c.stars, 3, "{} / {}", c.amount, c.why);
    }

    #[test]
    #[ignore = "two whole dishes, about 17 s in release: run with --ignored"]
    fn the_intended_hand_takes_three_stars_on_the_others() {
        for (k, l) in DISHES.iter().enumerate().skip(1) {
            let c = score(k, 1, &mut Route::for_level(k));
            assert_eq!(c.stars, 3, "{}: {} / {}", l.name, c.amount, c.why);
        }
    }

    #[test]
    fn progress_survives_a_bad_file() {
        let p = Progress::parse("lesson 1\nwall three\nruler 9\n\x00garbage\ndry\nwall 2 extra\n");
        assert_eq!(p.best, [1, 2, 3, 0]);
        assert!(p.unlocked(1) && p.unlocked(2) && p.unlocked(3) && p.unlocked(4));
        assert_eq!(Progress::parse(&p.text()), p);
        let fresh = Progress::parse("");
        assert!(fresh.unlocked(0) && !fresh.unlocked(1) && fresh.unlocked(ENTRIES - 1));
    }
}
