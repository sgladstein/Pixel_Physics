//! **Does a colony's nest stand in the lab's own bed?** — the owner's
//! playtest report, reproduced or refuted.
//!
//! Reported 2026-08-30, playing the merged lab: *"it looks to me like tunnels
//! that ants are digging are collapsing or soil is filling and they get stuck
//! underground."*
//!
//! `burrow_probe` already says a *hand-carved lined* gallery holds at 100%
//! and that a colony on a **hand-built soil bank** leaves 130 roofed cells
//! against 0 ablated. Neither of those is the scene the owner is playing.
//! The lab's bed is different in three ways that each bear on this, and
//! `CLAUDE.md`'s standing rule is that a scene which contradicts the code
//! looks exactly like a bug in the code:
//!
//! 1. **It is built at exactly `SOIL_FIELD_CAPACITY`** (`scene.rs`'s
//!    `with_aux`), and the un-pack rule fires at `aux > SOIL_FIELD_CAPACITY`.
//!    That is a margin of **one unit**. If `update_soil_water` ever pushes a
//!    wall cell one unit over — and redistributing water around a *void* is
//!    exactly the case nobody measured — the lining dissolves and the
//!    tunnel comes down. The bank scenes `burrow_probe` uses do not sit on
//!    that line.
//! 2. **It is 96 rows deep**, so an ant can be far below the surface.
//! 3. **It is sealed and lit through the shell**, so nothing dries it out.
//!
//! # The two halves of the report get separate columns, because they have
//! # different fixes
//!
//! *"Tunnels collapsing / soil filling"* is a **structure** question:
//! `roofed` (void with ground above it) and `packed` (lining still standing)
//! over time. A lining that appears and then vanishes is hypothesis 1; a
//! lining that never appears is a dig problem.
//!
//! *"They get stuck underground"* is a **motion** question, and it needs its
//! own counters or it cannot be told from "they are fine and just out of
//! sight". `buried` counts ants with ground on every side; `blocked_frac` is
//! `moves_blocked / (moves + moves_blocked)`. An ant that is walking a
//! gallery and one that is entombed both read "below the surface".
//!
//! **Read the counters, not the totals.** `CLAUDE.md`: a bank with no holes
//! in it and a colony that never dug are the same picture. `digs` and
//! `packed` are printed on every row for exactly that reason.
//!
//! ```text
//! cargo run --release --example labnest
//! cargo run --release --example labnest -- frames=12000 seeds=3
//! cargo run --release --example labnest -- dry=1     # the isolating control
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::material::MaterialId;
use pixel_physics::sim::{frame, material, player, World};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses"))
    })
}

/// Void with ground above it, inside the bed. **Roofed, not standing** —
/// `CLAUDE.md`'s metric trap: a colony quarries the open face as well as
/// tunnelling, and a pit is standing void, so censusing bare emptiness
/// scored the build with no roof at all *higher* than the one whose tunnels
/// stand. What a player calls a nest has ground over it.
fn roofed(world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> usize {
    let mut n = 0;
    for x in x0..x1 {
        for y in y0..y1 {
            if world.get(x, y).material != material::EMPTY {
                continue;
            }
            // Ground somewhere above in this column, inside the bed.
            if (y0..y).rev().any(|uy| world.get(x, uy).material != material::EMPTY) {
                n += 1;
            }
        }
    }
    n
}

/// Cells of standing lining. The quantity hypothesis 1 predicts will rise
/// and then fall.
fn packed_cells(world: &World, id: MaterialId, x0: i32, x1: i32, y0: i32, y1: i32) -> usize {
    let mut n = 0;
    for x in x0..x1 {
        for y in y0..y1 {
            if world.get(x, y).material == id {
                n += 1;
            }
        }
    }
    n
}

/// **Wall cells that have gone over the un-pack line.** The direct test of
/// hypothesis 1, and the reason this harness exists rather than a re-run of
/// `burrow_probe`: it reports the *margin* rather than the outcome, so a
/// bed sitting one unit under the threshold reads differently from one
/// sitting a hundred under it.
fn over_capacity(world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> (usize, u16) {
    let (mut n, mut worst) = (0, 0u16);
    for x in x0..x1 {
        for y in y0..y1 {
            let c = world.get(x, y);
            if c.material == material::EMPTY {
                continue;
            }
            if world.materials.get(c.material).water_capacity > 0 {
                worst = worst.max(c.aux());
                if c.aux() > material::SOIL_FIELD_CAPACITY {
                    n += 1;
                }
            }
        }
    }
    (n, worst)
}

/// **The mean the `overcap` count cannot give, and the two readings want
/// opposite fixes.**
///
/// `overcap` counts cells over the un-pack line. A bed that is genuinely
/// *flooding* and one that is merely redistributing around a threshold it
/// was built to sit exactly on produce the same rising count, and the first
/// is a water-cycle bug while the second is a knife-edge constant.
/// `CLAUDE.md`'s "ask what your number counts when nothing is wrong": at
/// frame 0 this bed is uniform, so the mean is the build value and the
/// spread is zero, and anything either number does afterwards is the run.
///
/// Returns `(mean held, total held, free water cells standing in the bed)`.
fn soil_water(world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> (f64, u64, usize) {
    let (mut total, mut cells, mut free) = (0u64, 0u64, 0usize);
    for x in x0..x1 {
        for y in y0..y1 {
            let c = world.get(x, y);
            if world.materials.kind(c.material) == pixel_physics::sim::material::MaterialKind::Liquid {
                free += 1;
                continue;
            }
            if world.materials.get(c.material).water_capacity > 0 {
                total += u64::from(c.aux());
                cells += 1;
            }
        }
    }
    (total as f64 / cells.max(1) as f64, total, free)
}

/// **Where the bed's moisture actually sits**, in bands, because a single
/// `overcap` count cannot set a threshold.
///
/// `CLAUDE.md`: *set bars from measurement with headroom, never from an
/// aspiration and never sitting on the measured value.* The un-pack line is
/// `SOIL_FIELD_CAPACITY`, which is where a drained column **rests** — so the
/// question a new line has to answer is how much of the bed reaches each
/// band in ordinary running, and only a histogram says that.
///
/// Bands are the engine's own constants plus the quarter points of the
/// drainable band, so the reading is in the vocabulary any replacement
/// threshold would be written in.
fn moisture_bands(world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> [usize; 6] {
    let fc = material::SOIL_FIELD_CAPACITY;
    let sat = material::SOIL_SATURATED;
    let band = sat - fc;
    let mut n = [0usize; 6];
    for x in x0..x1 {
        for y in y0..y1 {
            let c = world.get(x, y);
            if world.materials.get(c.material).water_capacity == 0 {
                continue;
            }
            let a = c.aux();
            if a <= fc {
                n[0] += 1;
            } else if a < fc + band / 4 {
                n[1] += 1;
            } else if a < fc + band / 2 {
                n[2] += 1;
            } else if a < fc + 3 * band / 4 {
                n[3] += 1;
            } else if a < sat {
                n[4] += 1;
            } else {
                n[5] += 1;
            }
        }
    }
    n
}

/// Ants with ground on all four sides — entombed rather than merely
/// underground. The second half of the report needs its own number.
fn buried(world: &World) -> usize {
    world
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| world.organism_state(id))
        .filter(|state| world.species.get(state.species).creature.is_some())
        .filter(|state| {
            // Every cell of the body walled in on all four sides. A body
            // with one open face is in a gallery; one with none is
            // entombed, and only the second is the owner's complaint.
            state.cells.keys().all(|(x, y)| {
                [(0, -1), (0, 1), (-1, 0), (1, 0)].iter().all(|(dx, dy)| {
                    let n = (x + dx, y + dy);
                    world.get(n.0, n.1).material != material::EMPTY
                        || state.cells.contains_key(&n)
                })
            })
        })
        .count()
}

/// **What is standing in a cell that used to be a hole** — the census the
/// owner's second reading of the report needs, and the one `roofed` cannot
/// give.
///
/// `roofed` falling says the nest is closing. It does not say what closed
/// it, and the two candidates want opposite fixes: loose `soil` running back
/// in is a lining failure, and `corpse` piling up in a gallery is a
/// *decay* failure — a carcass is a `Powder` with no `decays_into`, so it
/// falls like tilth and then stands there for the rest of the run.
/// `CLAUDE.md`'s "an image tells you what and where, a metric tells you how
/// much": this is the how-much, keyed by material name so a material added
/// later cannot be silently dropped.
///
/// The mark is taken on a cadence rather than every tick because the scan is
/// the bed (512x96) and the question is about standing state, not about a
/// cell that was empty for one frame between two digs.
struct Refill {
    /// Cells inside the bed that have been materially empty at some earlier
    /// mark. Indexed `(y - y0) * w + (x - x0)`.
    ever_void: Vec<bool>,
    w: usize,
}

impl Refill {
    fn new(x0: i32, x1: i32, y0: i32, y1: i32) -> Self {
        let w = (x1 - x0) as usize;
        Self { ever_void: vec![false; w * (y1 - y0) as usize], w }
    }

    fn mark(&mut self, world: &World, x0: i32, x1: i32, y0: i32, y1: i32) {
        for x in x0..x1 {
            for y in y0..y1 {
                if world.get(x, y).material == material::EMPTY {
                    self.ever_void[(y - y0) as usize * self.w + (x - x0) as usize] = true;
                }
            }
        }
    }

    /// `(cells refilled, per-material counts, largest first)`.
    fn census(&self, world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> (usize, Vec<(String, usize)>) {
        let mut by: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut total = 0;
        for x in x0..x1 {
            for y in y0..y1 {
                if !self.ever_void[(y - y0) as usize * self.w + (x - x0) as usize] {
                    continue;
                }
                let m = world.get(x, y).material;
                if m == material::EMPTY {
                    continue;
                }
                total += 1;
                *by.entry(world.materials.get(m).name.clone()).or_default() += 1;
            }
        }
        let mut v: Vec<(String, usize)> = by.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        (total, v)
    }
}

/// Every material standing inside the bed, largest first — the control for
/// the census above. A refill figure with no idea what the bed is made of
/// cannot say whether 400 corpse cells in the holes is most of the carrion
/// in the box or a tenth of it.
fn bed_census(world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> Vec<(String, usize)> {
    let mut by: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for x in x0..x1 {
        for y in y0..y1 {
            let m = world.get(x, y).material;
            if m == material::EMPTY {
                continue;
            }
            *by.entry(world.materials.get(m).name.clone()).or_default() += 1;
        }
    }
    let mut v: Vec<(String, usize)> = by.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// **Where in the bed the waterlogged cells are**, by depth band — the
/// question `moisture_bands` cannot answer and the one that decides how much
/// of the box can hold a tunnel.
///
/// A count of cells past the un-pack line says how many; it does not say
/// whether they are scattered beside roots (a root breaking into a gallery,
/// which is local and arguably wanted) or stacked in a band at the bottom of
/// the bed (a water table, which makes a whole depth range unusable). Those
/// are the same number and opposite findings — `CLAUDE.md`'s *ask what your
/// number counts* — and only a profile separates them.
///
/// Eight bands from the surface down, each printed as
/// `over-waterlogged/soil cells`.
fn depth_profile(world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> Vec<(i32, usize, usize, f64)> {
    let rows = y1 - y0;
    let band = (rows / 8).max(1);
    let mut out = Vec::new();
    let mut y = y0;
    while y < y1 {
        let end = (y + band).min(y1);
        let (mut over, mut cells, mut total) = (0usize, 0usize, 0u64);
        for yy in y..end {
            for x in x0..x1 {
                let c = world.get(x, yy);
                if world.materials.get(c.material).water_capacity == 0 {
                    continue;
                }
                cells += 1;
                total += u64::from(c.aux());
                if c.aux() > material::SOIL_WATERLOGGED {
                    over += 1;
                }
            }
        }
        out.push((y - y0, over, cells, total as f64 / cells.max(1) as f64));
        y = end;
    }
    out
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(9_000);
    let seeds: u64 = arg("seeds").unwrap_or(2);
    // **The isolating control for hypothesis 1.** Same bed, built a long way
    // under field capacity instead of exactly on it. If the lining survives
    // here and dissolves in the default bed, the margin is the bug and the
    // fix is the threshold, not the digging.
    let dry: bool = arg::<u32>("dry").unwrap_or(0) == 1;
    // **Plants, so the colony does not simply starve.** The bed this harness
    // was written against has no food in it at all, so every ant is dead by
    // frame ~6,000 and the structure columns after that are a census of a
    // box nothing lives in. A fed colony keeps digging, which is the regime
    // the owner plays and the only one in which "the holes fill up" is a
    // statement about a working nest.
    let founders: usize = arg("founders").unwrap_or(0);
    // How often the void mark is taken. See `Refill`.
    let mark_every: u64 = arg("markevery").unwrap_or(30);

    println!("labnest: frames={frames} seeds={seeds} dry={dry} founders={founders} markevery={mark_every}");
    println!(
        "\n{:>5} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>6} {:>7} {:>7} {:>6} {:>8}",
        "seed", "frame", "roofed", "packed", "overcap", "wettest", "ants", "buried", "digs",
        "dumped", "laden", "blocked%"
    );

    for seed in 1..=seeds {
        let spec = LabBox { colonies: 1, founders, seed, ..LabBox::default() };
        let mut world = spec.build();
        let packed_id = world.materials.id_of("packedsoil").expect("packedsoil is compiled in");

        if dry {
            let soil = world.materials.id_of("soil").expect("soil is compiled in");
            for x in 0..spec.width {
                for y in 0..spec.height {
                    let c = world.get(x, y);
                    if c.material == soil {
                        world.set(x, y, c.with_aux(material::SOIL_WILTING_POINT * 2));
                    }
                }
            }
        }

        let (y0, y1) = (spec.ground_y, spec.ground_y + spec.soil_depth);
        let (x0, x1) = (0, spec.width);

        let mut particles = ParticleSystem::default();
        let mut blasts = Blasts::default();
        let tuning = player::Tuning::default();

        let mut refill = Refill::new(x0, x1, y0, y1);
        refill.mark(&world, x0, x1, y0, y1);

        let report = |world: &World, refill: &Refill, f: u64| {
            let (over, wettest) = over_capacity(world, x0, x1, y0, y1);
            let m = world.creature_stats.moves;
            let b = world.creature_stats.moves_blocked;
            // **`dumped` and `laden` beside `digs`, which is a call counter.**
            // Digging takes one cell into the mandibles and `dumped` counts
            // the ones that came out again, so `digs - dumped` is what is in
            // flight -- and `laden`, the animals holding one right now, is
            // what says whether that is traffic or a jam. A colony that has
            // stopped digging because every ant is stuck holding a pellet it
            // cannot put down reads exactly like one that has lost interest,
            // and only this pair tells them apart.
            let laden = world
                .live_organism_ids()
                .into_iter()
                .filter(|&id| world.organism(id).is_some_and(|s| s.spoil.is_some()))
                .count();
            println!(
                "{seed:>5} {f:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>6} {:>7} {:>7} {laden:>6} {:>7.1}%",
                roofed(world, x0, x1, y0, y1),
                packed_cells(world, packed_id, x0, x1, y0, y1),
                over,
                wettest,
                world.live_creature_count(),
                buried(world),
                world.creature_stats.digs,
                world.creature_stats.spoil_dumped,
                100.0 * b as f64 / (m + b).max(1) as f64,
            );
            let (filled, by) = refill.census(world, x0, x1, y0, y1);
            let show = |v: &[(String, usize)]| {
                v.iter().take(9).map(|(n, c)| format!("{n} {c}")).collect::<Vec<_>>().join(", ")
            };
            let (mean, total, free) = soil_water(world, x0, x1, y0, y1);
            println!(
                "      soil water mean {mean:>7.1}  total {total:>10}  free water cells {free:>4}  atmosphere {:>10.0}",
                world.atmospheric_bank
            );
            let prof = depth_profile(world, x0, x1, y0, y1);
            println!(
                "      by depth (rows below the surface)  {}",
                prof.iter()
                    .map(|(d, over, cells, mean)| format!("{d}:{over}/{cells} m{mean:.0}"))
                    .collect::<Vec<_>>()
                    .join("  ")
            );
            let b = moisture_bands(world, x0, x1, y0, y1);
            println!(
                "      moisture   <=620 {:>6}  <715 {:>6}  <810 {:>6}  <905 {:>6}  <1000 {:>6}  ==1000 {:>6}",
                b[0], b[1], b[2], b[3], b[4], b[5]
            );
            println!("      refilled void {filled:>6}  [{}]", show(&by));
            println!("      bed                  [{}]", show(&bed_census(world, x0, x1, y0, y1)));
        };

        report(&world, &refill, 0);
        for f in 1..=frames {
            frame::step(
                &mut world,
                &mut particles,
                &mut blasts,
                player::PlayerInput::default(),
                &tuning,
            );
            if f % mark_every == 0 {
                refill.mark(&world, x0, x1, y0, y1);
            }
            if f % (frames / 9).max(1) == 0 {
                report(&world, &refill, f);
            }
        }
    }
}
