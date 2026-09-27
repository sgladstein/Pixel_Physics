//! **A bare box that asks one question: what does a colony dig, and why?**
//!
//! Everything a colony normally spends its time on is deleted. No plants, no
//! seeds, no litter, no lamps, no weather, no water, no predators — and above
//! all **no food**, so there is nothing to forage and the only thing an ant
//! can do with its life is walk and dig. The bed is stone, soil and air.
//!
//! **Why no food is the whole point, and not merely tidiness.** The shipped
//! ant's dig wiring is
//!
//! ```text
//! dig_urge = squash(0.15*Bias + 0.8*FoodAdjacent - 0.55*MoistureGrad + 2.5*u5 - 2.5*u6)
//! u5 = squash(-30 + 30*AtNest + 6*Crowding)    u6 = squash(-30 + 30*AtNest - 6*Crowding)
//! ```
//!
//! and `(FoodAdjacent, Dig, 0.8)` is the largest direct term. Away from the
//! nest it takes dig probability from **0.154 to 0.496** — that term, not the
//! chamber gate, is most of the digging on a bed with food lying about, and it
//! is quarrying beside a meal rather than building a room. A box with no food
//! sets it to zero by construction, so what is left is the nest mechanism
//! alone.
//!
//! **Nothing starves, because the run fits inside the endowment.** `ant.ron`
//! authors `start_energy: 200` against `idle_cost_per_cell: 0.05` on a
//! two-cell body, so an idle ant lives `200/0.1 = 2,000` decision ticks —
//! **12,000 frames** at `tick_interval: 6`. `budget` prints that horizon and
//! the default `frames=` sits under it. A digger spends faster
//! (`dig_cost_in_moves: 6.0`), so ants run *down* rather than starve to death
//! mid-question, and the census says how much charge is left.
//!
//! This is deliberately **not** the lab bed. The lab bed cannot answer a
//! digging question: measured 2026-09-19 over a four-way sweep of the nest's
//! row reach at three seeds, roofed void came back 226/116/381/293/262 on one
//! seed and 97/86/77/58/108 on another — no trend, because colony booms,
//! crashes, starvation and plant dynamics swamp the parameter. Isolation is
//! what buys a readable answer.
//!
//! ```text
//! cargo run --release --example digbox
//! cargo run --release --example digbox -- ants=40 frames=12000 out=/tmp/box.png
//! cargo run --release --example digbox -- seed=3          # a different colony in the same box
//! cargo run --release --example digbox -- selftest
//! PIXEL_PHYSICS_NEST_SITE_ROWS=8 cargo run --release --example digbox
//! ```
//!
//! **`selftest` is the positive control and it runs two arms**, because the
//! two ways this harness can lie are opposite: a box with no ants must census
//! **zero** dug void (specificity — if it reads void with nobody digging, the
//! census is measuring the scene), and a box with ants must read **non-zero**
//! digs (sensitivity — a harness where the colony never places, or never
//! wakes, reports a clean null that looks exactly like a finding). `CLAUDE.md`
//! records this repo shipping the second failure repeatedly.

use pixel_physics::render::Renderer;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material::{MaterialId, MaterialKind};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::weather::Pin;
use pixel_physics::sim::{material, parallel, Cell, World};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses"))
    })
}

fn flag(key: &str) -> bool {
    std::env::args().skip(1).any(|a| a == key)
}

/// The box's geometry, in one place so the census and the builder cannot
/// disagree about where the ground is.
struct Box2 {
    w: i32,
    h: i32,
    /// First row of stone — the floor the soil rests on.
    floor: i32,
    /// First row of soil. Everything above is air.
    surface: i32,
}

impl Box2 {
    fn new(w: i32, h: i32, soil: i32, sky: i32) -> Self {
        let surface = sky;
        Box2 { w, h, floor: surface + soil, surface }
    }
}

/// Build the bare box: a stone shell, a soil fill, air above, nothing else.
///
/// **Soil is placed `with_attached(true)`** exactly as `burrow_probe`'s bank
/// is, so the fill starts settled rather than slumping for the first hundred
/// frames and reading as excavation.
fn build(b: &Box2) -> World {
    build_wet(b, material::SOIL_FIELD_CAPACITY)
}

/// The box at a chosen soil wetness.
///
/// **`wet=` exists because `(MoistureGrad, Dig, -0.55)` is a real term and
/// cannot be reasoned about, only measured.** `creature::moisture_gradient`
/// returns an unsigned *magnitude* -- the length of the moisture gradient
/// over a 8-cell span, normalised -- so it is largest wherever moisture
/// *changes* fastest, which in a uniformly-wet bed is the air/soil boundary
/// and nowhere else. A negative weight on it therefore suppresses digging at
/// the surface and leaves the deep soil untouched, which is the opposite of
/// what the term's name suggests to a reader. Setting the fill uniformly and
/// sweeping it is how that stops being an argument.
///
/// Useful values: `SOIL_WILTING_POINT` 180, `SOIL_FIELD_CAPACITY` 620,
/// `SOIL_SATURATED` 1000.
fn build_wet(b: &Box2, wet: u16) -> World {
    build_graded(b, wet, None)
}

/// `build_wet`, with an optional linear top-to-bottom moisture ramp.
///
/// **A uniform fill has no vertical gradient at any value**, so on the
/// default bed `MoistureLateral` and `MoistureGrad` report the air/soil step
/// and nothing about depth -- which is what `creature::moisture_gradient`'s
/// own doc says that channel measures. A taxis that is supposed to take an
/// ant downward has to be given somewhere to climb, and `wetgrad=top:bottom`
/// is that bed. Measured in `examples/burrow_probe`, which gained the same
/// argument first: over 55 ants, `MoistureLateral` reads **1 distinct value
/// on dry ground, 26 at a uniform field capacity, 3 at saturation (it
/// clips), and 44 on a 120->980 ramp** -- so the ramp is the only one of the
/// four beds that gives the channel its full range.
fn build_graded(b: &Box2, wet: u16, grad: Option<(u16, u16)>) -> World {
    let mut world = World::new(pixel_physics::sim::Rect::new(0, 0, b.w - 1, b.h - 1));
    // No weather and a held sky: a designed oscillator must not alias into
    // anything measured here (`CLAUDE.md`).
    world.set_weather_pin(Pin::Clear);
    world.set_sky_hold(Some(pixel_physics::sky::frame_for_daylight(1.0)));
    let soil_id = world.materials.id_of("soil").expect("soil ships");

    for x in 0..b.w {
        for y in 0..b.h {
            // Stone shell: floor, and one column of wall each side, so soil
            // cannot pour out of the world and read as a dug void.
            if y >= b.floor || x == 0 || x == b.w - 1 {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            } else if y >= b.surface {
                let aux = match grad {
                    Some((top, bottom)) => {
                        let span = (b.floor - b.surface).max(1) as f32;
                        let t = (y - b.surface) as f32 / span;
                        (top as f32 + (bottom as f32 - top as f32) * t).round() as u16
                    }
                    None => wet,
                };
                world.set(x, y, Cell::new(soil_id, 0).with_attached(true).with_aux(aux));
            }
        }
    }
    world
}

/// Dug void: empty cells inside the original soil block.
///
/// **Split into roofed and open**, because `CLAUDE.md`'s metric trap says a
/// hole open to the sky is not a room. `roofed` is void with ground standing
/// somewhere above it in the column; `open` is void the sky can see.
/// **Plus the extent of the room itself, which is the only reading here
/// that can tell a shaft from a lens.**
///
/// `roofed`, `open` and `bodies` are counts: a 46-wide by 2-deep scrape and
/// a 2-wide by 46-deep shaft of the same volume read **identical** on all
/// three. `examples/burrow_probe`'s `arms=selftest` asserts exactly that
/// against one bar drawn both ways, and the same blindness was live here.
///
/// **And the `trace` line's "spread over N columns x M rows" does not fill
/// the gap, which is the trap worth naming.** That is the spread of
/// *at-nest ants*, so it follows `PIXEL_PHYSICS_NEST_SITE_COLS`/`_ROWS` **by
/// construction** -- set the reach to 3 columns and it reports 6, which is
/// the definition of the dial and not a result about digging. Measured
/// 2026-09-19: across the whole `cols` sweep the ant spread tracked the dial
/// exactly while `room_total` sat at 453-668 with no trend, and a rendered
/// pair showed no visible difference underground. Read `room w x h` below.
///
/// **And the bounding box is an extreme-value statistic, which is the trap
/// that replaced the last one.** `room w` is `max(x) - min(x)`: one stray
/// dug cell at the edge of the box sets it, and nothing about the other
/// thousand moves it. Measured 2026-09-19 over twelve seeds of the
/// *unchanged* control, `room w` runs **96 to 191** -- a 2x spread with no
/// arm and no switch, wider than the差 between any two arms the shape
/// report compared at one run each. So a bbox can say *a shaft happened*
/// (nothing reaches that) and cannot rank two lenses.
///
/// `iqr` is the companion that can: the number of columns holding the
/// **middle half** of the room's cells, which is a quantile rather than a
/// maximum and therefore does not move when one ant wanders. It is the
/// reading for *"did this concentrate the nest"* -- the question every
/// aggregation-point arm is about. `p50x` is where that middle half is
/// centred, in columns from the nest patch, so a widening that is **local
/// to the aggregation** and one that is a uniform rise everywhere can be
/// told apart, which is Stage 2's own check that can fail.
fn census(world: &World, b: &Box2) -> (usize, usize, usize, usize, i32, i32, i32, i32) {
    census_masked(world, b, &|_, _| false)
}

/// [`census`], with the cells `skip` names left out of every room count
/// (the ground in them still roofs what is below, as ground does).
///
/// **For the founding cut, which the colony did not dig.** Measured
/// 2026-09-26 over twelve seeds: every arm with a shaft read the room's
/// middle half **narrower on 12 of 12** and deeper on 11-12 -- but the cut's
/// own chamber is ~40 roofed cells at the centre and its floor sits 22 rows
/// down, so part of that is the census counting what founding handed the
/// colony. `CLAUDE.md`'s question -- *what does this number count when
/// nothing is wrong?* -- answered: a shaft arm with no ants reads a room by
/// construction. Scoring the room with the cut masked out is what separates
/// "the colony dug a deeper, narrower nest" from "the census found the hole".
fn census_masked(world: &World, b: &Box2, skip: &dyn Fn(i32, i32) -> bool) -> (usize, usize, usize, usize, i32, i32, i32, i32) {
    let (mut roofed, mut open) = (0, 0);
    // **Cells below the old surface that hold an animal.**
    //
    // `roofed` and `open` count cells that are materially EMPTY, and **a
    // gallery with an ant standing in it is not empty**. At 500+ animals of a
    // two-cell body that is a thousand cells, most of them underground, and
    // leaving them out makes the nest read far smaller than it is. The tell
    // was a conservation failure: 941 cells hauled above the original surface
    // against 322 cells of void below it, when digging only moves material
    // and the two should account for each other.
    let mut bodies = 0;
    // **Material standing ABOVE the original surface.** Digging here moves
    // material, it does not destroy it: `spoil_dumped` tracks `digs` to
    // within a couple of percent, so every cell cut is put back down
    // somewhere. If it is put back inside the excavation, the void it came
    // from is cancelled. The only cells that can leave a net hole behind are
    // the ones carried clear of the ground, which is the mound. So this
    // column and the void columns are two sides of one conservation law, and
    // reading them together is what says whether a nest is limited by
    // *digging* or by *haulage*.
    let mut above = 0;
    for x in 1..b.w - 1 {
        for y in 0..b.surface {
            let cell = world.get(x, y);
            let kind = world.materials.kind(cell.material);
            if cell.material != material::EMPTY
                && matches!(kind, MaterialKind::Powder | MaterialKind::Solid)
                && cell.organism_id() == 0
            {
                above += 1;
            }
        }
    }
    for x in 1..b.w - 1 {
        let mut covered = false;
        for y in b.surface..b.floor {
            let cell = world.get(x, y);
            let kind = world.materials.kind(cell.material);
            let is_ground = cell.material != material::EMPTY
                && matches!(kind, MaterialKind::Powder | MaterialKind::Solid)
                && cell.organism_id() == 0;
            if is_ground {
                covered = true;
            } else if skip(x, y) {
                continue;
            } else if cell.material == material::EMPTY {
                if covered {
                    roofed += 1;
                } else {
                    open += 1;
                }
            } else if cell.organism_id() != 0 && kind == MaterialKind::Creature {
                bodies += 1;
            }
        }
    }
    // The bounding box of the room -- void and the bodies standing in it,
    // which is the same "room, occupied or not" this census already counts.
    let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for x in 1..b.w - 1 {
        let mut covered = false;
        for y in b.surface..b.floor {
            let cell = world.get(x, y);
            let kind = world.materials.kind(cell.material);
            let is_ground = cell.material != material::EMPTY
                && matches!(kind, MaterialKind::Powder | MaterialKind::Solid)
                && cell.organism_id() == 0;
            if is_ground {
                covered = true;
                continue;
            }
            let is_room = cell.material == material::EMPTY || kind == MaterialKind::Creature;
            if is_room && covered && !skip(x, y) {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    let (rw, rh) = if x1 >= x0 { (x1 - x0 + 1, y1 - y0 + 1) } else { (0, 0) };
    // **The middle half of the room, by column.** Counted per column, then
    // walked from both ends discarding a quarter of the mass at each --
    // so the answer is how wide the nest is where the nest actually is,
    // and a single cell at the far wall cannot set it.
    let mut per_col = vec![0i64; b.w.max(1) as usize];
    for x in 1..b.w - 1 {
        let mut covered = false;
        for y in b.surface..b.floor {
            let cell = world.get(x, y);
            let kind = world.materials.kind(cell.material);
            let is_ground = cell.material != material::EMPTY
                && matches!(kind, MaterialKind::Powder | MaterialKind::Solid)
                && cell.organism_id() == 0;
            if is_ground {
                covered = true;
                continue;
            }
            if covered && !skip(x, y) && (cell.material == material::EMPTY || kind == MaterialKind::Creature) {
                per_col[x as usize] += 1;
            }
        }
    }
    let total: i64 = per_col.iter().sum();
    let (mut lo, mut hi) = (0i32, b.w - 1);
    if total > 0 {
        let q = total / 4;
        let mut run = 0i64;
        for x in 0..b.w {
            run += per_col[x as usize];
            if run > q {
                lo = x;
                break;
            }
        }
        run = 0;
        for x in (0..b.w).rev() {
            run += per_col[x as usize];
            if run > q {
                hi = x;
                break;
            }
        }
    }
    let (iqr, p50x) = if total > 0 { (hi - lo + 1, (lo + hi) / 2 - b.w / 2) } else { (0, 0) };
    (roofed, open, above, bodies, rw, rh, iqr, p50x)
}

/// **Chambers and tunnels, or one hole?** — the question `vert` and `iqr`
/// structurally cannot answer, and the one the owner's spec is written in.
///
/// Owner ruling, 2026-09-20: *"the chambers have to be readable. Chambers
/// that are just two cells tall are not going to look like a chamber in this
/// game. We can go bigger but at some point the answer is just digging one
/// giant hole which is actually what they do right now and then it's not
/// chambers and tunnels."* The target therefore has a failure mode on
/// **both** sides, and the quantity separating them is not volume: it is the
/// **contrast** between a chamber's bore and the bore of the passage joining
/// it — he put the passage at about 4 cells and the chamber at 2–4x that.
///
/// **Why every column already here is blind to it.** `roofed`, `open` and
/// `room total` are counts, so they rank a bigger hole above a better one.
/// `vert` and `iqr` describe the bounding box of the *whole* excavation, so
/// one chamber of 12x32 and a 12x32 smear score identically. Five arms have
/// now been scored on those columns and every one came back a coin flip —
/// which is what a shape question looks like when it is measured by volume.
///
/// **And not connected components either.** A nest is connected *by
/// definition* — the tunnels join the chambers — so a component count over
/// the void is 1 for the target and 1 for the giant hole. The separation has
/// to come from *width*, not from adjacency.
///
/// **So: a Chebyshev distance transform over the room.** Every void cell
/// gets the radius of the largest square of void centred on it, which makes
/// a 4-cell bore read 2 whichever way it runs — the property worth having,
/// because a shaft and a tunnel are one feature rotated. A chamber 12 cells
/// tall reads 6. Cells at or above [`CHAMBER_R`] are *chamber core*; the
/// rest is passage. Counting components **of the core** is what separates
/// the three cases: two chambers joined by a narrow tunnel are two, because
/// the tunnel never reaches the core threshold; a hole with no narrow waist
/// anywhere is one; scratches reach the threshold nowhere and are none.
///
/// Reads the same "room" the rest of this census does — roofed void, with
/// the ants standing in it counted as room rather than as wall, so a chamber
/// does not stop being one when somebody walks into it.
struct Chambers {
    /// Discrete chambers: components of the core. **0 = scratches, 1 = the
    /// giant hole, 2+ = structure** — but read `max_w` with it, because one
    /// chamber is also what a correct small nest looks like early on.
    count: i32,
    /// Median chamber box, in cells, recovered by letting each core cell
    /// claim the square it is the centre of. Against the owner's spec
    /// directly: 8–16 tall, and wider than tall.
    med_h: i32,
    med_w: i32,
    /// The widest chamber. **This is the giant-hole tell**: today's nest is
    /// one chamber ~120 cells wide, which no ruling in this file calls a
    /// chamber.
    max_w: i32,
    /// Typical bore of the void that is *not* chamber — the tunnels, plus
    /// the fringe of every chamber, which is why it reads a little under the
    /// true passage width rather than over it.
    passage: i32,
    /// Median chamber bore over passage bore. **The owner's 2–4x.** 1.0 is
    /// "there is no distinction between a room and a corridor here".
    contrast: f32,
}

/// Half the smallest bore this game will read as a chamber: the owner's
/// 2x-the-passage floor, at a 4-cell passage, is 8 cells — so radius 4.
/// **A design constant, not a measurement**, and deliberately the *bottom*
/// of the stated band so the census does not quietly raise the bar.
const CHAMBER_R: i32 = 4;

fn chambers(world: &World, b: &Box2) -> Chambers {
    let (w, h) = (b.w as usize, (b.floor - b.surface) as usize);
    let idx = |x: usize, y: usize| y * w + x;
    // Room mask, on the census's own predicate. Anything outside stays
    // solid, so the box walls bound the transform instead of leaking.
    let mut void = vec![false; w * h];
    for x in 1..b.w - 1 {
        let mut covered = false;
        for y in b.surface..b.floor {
            let cell = world.get(x, y);
            let kind = world.materials.kind(cell.material);
            let is_ground = cell.material != material::EMPTY
                && matches!(kind, MaterialKind::Powder | MaterialKind::Solid)
                && cell.organism_id() == 0;
            if is_ground {
                covered = true;
                continue;
            }
            if covered && (cell.material == material::EMPTY || kind == MaterialKind::Creature) {
                void[idx(x as usize, (y - b.surface) as usize)] = true;
            }
        }
    }
    chambers_of(&void, w, h)
}

/// **Chebyshev distance to the nearest non-room cell**, two passes: every
/// room cell gets the radius of the largest square of room centred on it.
/// Split out of [`chambers`] so the scoreboard reads bores through the same
/// transform the chamber census does.
fn chebyshev(void: &[bool], w: usize, h: usize) -> Vec<i32> {
    let idx = |x: usize, y: usize| y * w + x;
    const FAR: i32 = 1 << 20;
    let mut d: Vec<i32> = void.iter().map(|&v| if v { FAR } else { 0 }).collect();
    for y in 0..h {
        for x in 0..w {
            if d[idx(x, y)] == 0 {
                continue;
            }
            let mut best = FAR;
            if x > 0 {
                best = best.min(d[idx(x - 1, y)]);
            }
            if y > 0 {
                best = best.min(d[idx(x, y - 1)]);
                if x > 0 {
                    best = best.min(d[idx(x - 1, y - 1)]);
                }
                if x + 1 < w {
                    best = best.min(d[idx(x + 1, y - 1)]);
                }
            }
            d[idx(x, y)] = d[idx(x, y)].min(best + 1);
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            if d[idx(x, y)] == 0 {
                continue;
            }
            let mut best = FAR;
            if x + 1 < w {
                best = best.min(d[idx(x + 1, y)]);
            }
            if y + 1 < h {
                best = best.min(d[idx(x, y + 1)]);
                if x + 1 < w {
                    best = best.min(d[idx(x + 1, y + 1)]);
                }
                if x > 0 {
                    best = best.min(d[idx(x - 1, y + 1)]);
                }
            }
            d[idx(x, y)] = d[idx(x, y)].min(best + 1);
        }
    }
    d
}

/// [`chambers`] over any room mask `void` (`w` x `h`, row 0 the old
/// surface), so a null model's mask is scored by the same census as the
/// colony's.
fn chambers_of(void: &[bool], w: usize, h: usize) -> Chambers {
    let idx = |x: usize, y: usize| y * w + x;
    let d = chebyshev(void, w, h);
    // Components of the core, 8-connected, flood filled iteratively -- a
    // recursive fill blows the stack on a box-wide hole, which is exactly
    // the case this census exists to name.
    let core: Vec<bool> = d.iter().map(|&v| v >= CHAMBER_R).collect();
    let mut seen = vec![false; w * h];
    let mut boxes: Vec<(i32, i32)> = Vec::new();
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for sy in 0..h {
        for sx in 0..w {
            if !core[idx(sx, sy)] || seen[idx(sx, sy)] {
                continue;
            }
            let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
            seen[idx(sx, sy)] = true;
            stack.push((sx, sy));
            while let Some((x, y)) = stack.pop() {
                // Each core cell claims the square it is the centre of, so
                // the box is the chamber rather than its middle band.
                let r = d[idx(x, y)] - 1;
                x0 = x0.min(x as i32 - r);
                x1 = x1.max(x as i32 + r);
                y0 = y0.min(y as i32 - r);
                y1 = y1.max(y as i32 + r);
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                            continue;
                        }
                        let (nx, ny) = (nx as usize, ny as usize);
                        if core[idx(nx, ny)] && !seen[idx(nx, ny)] {
                            seen[idx(nx, ny)] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            boxes.push((y1 - y0 + 1, x1 - x0 + 1));
        }
    }
    // Passage: the void that never reached the core threshold.
    let mut passage_bores: Vec<i32> = d
        .iter()
        .zip(void.iter())
        .filter(|(&dv, &v)| v && dv < CHAMBER_R)
        .map(|(&dv, _)| dv * 2)
        .collect();
    passage_bores.sort_unstable();
    let passage = passage_bores.get(passage_bores.len() / 2).copied().unwrap_or(0);
    let mut hs: Vec<i32> = boxes.iter().map(|&(hh, _)| hh).collect();
    let mut ws: Vec<i32> = boxes.iter().map(|&(_, ww)| ww).collect();
    hs.sort_unstable();
    ws.sort_unstable();
    let med_h = hs.get(hs.len() / 2).copied().unwrap_or(0);
    let med_w = ws.get(ws.len() / 2).copied().unwrap_or(0);
    let max_w = ws.last().copied().unwrap_or(0);
    let contrast = if passage > 0 { med_h as f32 / passage as f32 } else { 0.0 };
    Chambers { count: boxes.len() as i32, med_h, med_w, max_w, passage, contrast }
}

// =============================================================================
// THE NEST SCOREBOARD: is the colony building anything, or scratching at random?
// =============================================================================
//
// Owner, 2026-09-27: *"Do you have a good way to evaluate how well a nest is
// being built versus random digging?"* Nothing here did. Every column above
// either ranks how MUCH was dug or describes a bounding box, and none says
// whether what was dug is more nest-like than the same number of cells removed
// by a rule with no nest in it. So: one boolean mask per excavation, one panel
// of shape metrics over it, and several null models that carve exactly as many
// cells from the same ground, all scored by the identical code. The colony's
// score on a metric is its percentile among a null's draws (mid-rank, ties
// split), so **0.5 reads "indistinguishable from this kind of random
// digging"** and 1.0 "more nest-like than every draw".
//
// Two grades, never pooled. STRUCTURE is against the nulls; SPEC is the
// owner's 2026-09-20 geometry in absolute terms (`SPEC` line), because a
// giant hole beats random on connectivity and still fails the spec.

/// A boolean mask over the soil band: `w` columns by `h` rows, row 0 the old
/// ground surface.
#[derive(Clone)]
struct Mask {
    w: usize,
    h: usize,
    on: Vec<bool>,
}

impl Mask {
    fn empty(w: usize, h: usize) -> Self {
        Mask { w, h, on: vec![false; w * h] }
    }
    fn at(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h && self.on[y as usize * self.w + x as usize]
    }
    fn set(&mut self, x: usize, y: usize) {
        self.on[y * self.w + x] = true;
    }
    fn count(&self) -> usize {
        self.on.iter().filter(|&&v| v).count()
    }
}

/// **The room as a mask**, on [`census_masked`]'s own predicate: below the
/// old surface, not ground, and empty or an animal. Roofed and open both,
/// because `roofed` is one of the panel's metrics rather than a filter, and
/// less the cells `skip` names (the founding cut, which the colony did not
/// dig).
fn room_mask(world: &World, b: &Box2, skip: &dyn Fn(i32, i32) -> bool) -> Mask {
    let mut m = Mask::empty(b.w as usize, (b.floor - b.surface) as usize);
    for x in 1..b.w - 1 {
        for y in b.surface..b.floor {
            let cell = world.get(x, y);
            let kind = world.materials.kind(cell.material);
            let is_ground = cell.material != material::EMPTY
                && matches!(kind, MaterialKind::Powder | MaterialKind::Solid)
                && cell.organism_id() == 0;
            if is_ground || skip(x, y) {
                continue;
            }
            if cell.material == material::EMPTY || kind == MaterialKind::Creature {
                m.set(x as usize, (y - b.surface) as usize);
            }
        }
    }
    m
}

/// **Where a null may dig, and where it starts from**, read once at frame 0.
struct Ground {
    /// Every band cell an ant could cut at frame 0: ground within
    /// `dig_force` 1.0, so soil but never the painted nest (6.0) or the
    /// stone shell.
    elig: Mask,
    elig_idx: Vec<usize>,
    elig_rows: Vec<Vec<usize>>,
    /// The founding cut: void the colony was handed. A gallery that opens
    /// into it is open to the outside, for the colony and a null alike.
    portal: Mask,
    /// Eligible cells within 2 of the door (painted nest or founding cut),
    /// where the door-grown nulls begin.
    starts: Vec<usize>,
}

fn ground_at(world: &World, b: &Box2, cut: &dyn Fn(i32, i32) -> bool) -> Ground {
    let (w, h) = (b.w as usize, (b.floor - b.surface) as usize);
    let nest = world.materials.id_of("nest");
    let mut elig = Mask::empty(w, h);
    let mut portal = Mask::empty(w, h);
    let mut door: Vec<(i32, i32)> = Vec::new();
    for x in 1..b.w - 1 {
        for y in (b.surface - 2)..b.floor {
            let row = y - b.surface;
            if cut(x, y) {
                if row >= 0 {
                    portal.set(x as usize, row as usize);
                }
                door.push((x, row));
                continue;
            }
            let cell = world.get(x, y);
            if Some(cell.material) == nest {
                door.push((x, row));
                continue;
            }
            let kind = world.materials.kind(cell.material);
            let ground = cell.material != material::EMPTY && matches!(kind, MaterialKind::Powder | MaterialKind::Solid) && cell.organism_id() == 0;
            if row >= 0 && ground && world.materials.get(cell.material).penetration_resistance <= 1.0 {
                elig.set(x as usize, row as usize);
            }
        }
    }
    let elig_idx: Vec<usize> = (0..w * h).filter(|&i| elig.on[i]).collect();
    let mut elig_rows: Vec<Vec<usize>> = vec![Vec::new(); h];
    for &i in &elig_idx {
        elig_rows[i / w].push(i);
    }
    let starts: Vec<usize> = elig_idx
        .iter()
        .copied()
        .filter(|&i| {
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            door.iter().any(|&(dx, dy)| (dx - x).abs() <= 2 && (dy - y).abs() <= 2)
        })
        .collect();
    Ground { elig, elig_idx, elig_rows, portal, starts }
}

/// One excavation's shape, every number read the same way off any mask.
#[derive(Clone, Copy, Default)]
struct Panel {
    n: usize,
    /// Entrances: runs of dug (or cut) cells along the old surface row.
    mouths: f32,
    /// Share of the dug cells in pieces that reach the outside, through the
    /// surface row or the founding cut. A nest is a place you can walk into.
    reach: f32,
    /// Share in the largest 8-connected piece.
    largest: f32,
    /// Share with ground somewhere above in the column: a room, not a pit.
    roofed: f32,
    /// 90th percentile depth below the old surface, in rows.
    depth90: f32,
    /// Columns holding the middle half of the dug cells: concentration.
    iqr: f32,
    /// Share of dug cells at Chebyshev radius 2 or more, i.e. bore 3+: a
    /// passage rather than a two-cell scratch.
    wide: f32,
}

/// The panel's metrics, **directions fixed before any run**: the name, and
/// whether a higher value is the more nest-like one. `mouths` is scored as
/// its distance from the owner's one mouth.
const METRICS: [(&str, bool); 7] =
    [("mouths", false), ("reach", true), ("largest", true), ("roofed", true), ("depth90", true), ("iqr", false), ("wide", true)];

fn metric(p: &Panel, i: usize) -> f32 {
    match i {
        0 => (p.mouths - 1.0).abs(),
        1 => p.reach,
        2 => p.largest,
        3 => p.roofed,
        4 => p.depth90,
        5 => p.iqr,
        _ => p.wide,
    }
}

fn panel_of(m: &Mask, portal: &Mask) -> Panel {
    let (w, h) = (m.w, m.h);
    let n = m.count();
    if n == 0 {
        return Panel::default();
    }
    let mut mouths = 0;
    let mut in_run = false;
    for x in 0..w {
        let v = m.on[x] || portal.on[x];
        if v && !in_run {
            mouths += 1;
        }
        in_run = v;
    }
    // Pieces, 8-connected like every other census here, iteratively.
    let mut seen = vec![false; w * h];
    let (mut reached, mut largest) = (0usize, 0usize);
    let mut stack: Vec<usize> = Vec::new();
    for s in 0..w * h {
        if !m.on[s] || seen[s] {
            continue;
        }
        let (mut size, mut out) = (0usize, false);
        seen[s] = true;
        stack.push(s);
        while let Some(i) = stack.pop() {
            size += 1;
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            if y == 0 {
                out = true;
            }
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    if portal.at(nx, ny) {
                        out = true;
                    }
                    if m.at(nx, ny) {
                        let j = ny as usize * w + nx as usize;
                        if !seen[j] {
                            seen[j] = true;
                            stack.push(j);
                        }
                    }
                }
            }
        }
        largest = largest.max(size);
        if out {
            reached += size;
        }
    }
    let mut roofed = 0usize;
    for x in 0..w {
        let mut covered = false;
        for y in 0..h {
            let i = y * w + x;
            if m.on[i] {
                if covered {
                    roofed += 1;
                }
            } else if !portal.on[i] {
                covered = true;
            }
        }
    }
    let mut depths: Vec<usize> = (0..w * h).filter(|&i| m.on[i]).map(|i| i / w).collect();
    depths.sort_unstable();
    let depth90 = depths[(depths.len() * 9 / 10).min(depths.len() - 1)] as f32;
    let mut per_col = vec![0usize; w];
    for i in 0..w * h {
        if m.on[i] {
            per_col[i % w] += 1;
        }
    }
    let q = n / 4;
    let (mut lo, mut hi, mut run) = (0usize, w - 1, 0usize);
    for (x, &c) in per_col.iter().enumerate() {
        run += c;
        if run > q {
            lo = x;
            break;
        }
    }
    run = 0;
    for x in (0..w).rev() {
        run += per_col[x];
        if run > q {
            hi = x;
            break;
        }
    }
    // Bores on a copy padded with one ring of ground, so the surface row and
    // the band's edges bound the transform instead of reading as open room.
    let (pw, ph) = (w + 2, h + 2);
    let mut padded = vec![false; pw * ph];
    for y in 0..h {
        for x in 0..w {
            padded[(y + 1) * pw + x + 1] = m.on[y * w + x];
        }
    }
    let d = chebyshev(&padded, pw, ph);
    let wide = (0..pw * ph).filter(|&i| padded[i] && d[i] >= 2).count();
    Panel {
        n,
        mouths: mouths as f32,
        reach: reached as f32 / n as f32,
        largest: largest as f32 / n as f32,
        roofed: roofed as f32 / n as f32,
        depth90,
        iqr: (hi.saturating_sub(lo) + 1) as f32,
        wide: wide as f32 / n as f32,
    }
}

/// Mid-rank percentile of `c` among `draws`, oriented so 1.0 is more
/// nest-like than every draw, and the share of draws it ties.
fn percentile(c: f32, draws: &[f32], higher: bool) -> (f32, f32) {
    let (mut beat, mut tie) = (0usize, 0usize);
    for &d in draws {
        if (d - c).abs() <= 1e-6 {
            tie += 1;
        } else if (c > d) == higher {
            beat += 1;
        }
    }
    let k = draws.len().max(1) as f32;
    ((beat as f32 + 0.5 * tie as f32) / k, tie as f32 / k)
}

/// **The null models, weakest to strongest.** Each carves exactly `n` cells
/// of the eligible ground and has no nest logic in it.
///
/// - `uniform`: anywhere in the band. A floor: any connected dig beats it.
/// - `rows`: the colony's own count in every row, placed at random along
///   the row. Keeps how deep the colony dug and asks only whether the
///   arrangement is a nest; its `depth90` ties by construction.
/// - `eden`: one blob grown a random neighbour at a time from the door.
///   Connected and entered, with no plan: the strictest test of
///   compactness, and a pit rather than a room.
/// - `walkers`: as many diggers as the colony has ants, starting beside the
///   door, each walking straight with probability 0.8 and digging whatever
///   it walks into. The engine's own rule stripped of the nest gate, the
///   brain and the spoil: the closest thing to "random digging by ants".
#[derive(Clone, Copy, PartialEq)]
enum Null {
    Uniform,
    Rows,
    Eden,
    Walkers,
}

const NULLS: [(Null, &str); 4] = [(Null::Uniform, "uniform"), (Null::Rows, "rows"), (Null::Eden, "eden"), (Null::Walkers, "walkers")];

/// Straight-ahead persistence of the `walkers` null, per step.
const WALKER_PERSIST: f32 = 0.8;

fn draw_null(kind: Null, g: &Ground, colony: &Mask, n: usize, walkers: usize, rng: &mut pixel_physics::sim::rng::Rng) -> Mask {
    let (w, h) = (g.elig.w, g.elig.h);
    let mut m = Mask::empty(w, h);
    let pick = |pool: &mut Vec<usize>, k: usize, rng: &mut pixel_physics::sim::rng::Rng, m: &mut Mask| {
        let k = k.min(pool.len());
        for i in 0..k {
            let j = i + rng.below((pool.len() - i) as u32) as usize;
            pool.swap(i, j);
            m.on[pool[i]] = true;
        }
    };
    match kind {
        Null::Uniform => {
            let mut pool = g.elig_idx.clone();
            pick(&mut pool, n, rng, &mut m);
        }
        Null::Rows => {
            for y in 0..h {
                let want = (0..w).filter(|&x| colony.on[y * w + x]).count();
                if want > 0 {
                    let mut pool = g.elig_rows[y].clone();
                    pick(&mut pool, want, rng, &mut m);
                }
            }
        }
        Null::Eden => {
            let mut in_front = vec![false; w * h];
            let mut front: Vec<usize> = Vec::new();
            let mut dug = 0usize;
            while dug < n {
                if front.is_empty() {
                    // (Re)seed beside the door; fall back to anywhere eligible.
                    let pool: Vec<usize> = g.starts.iter().copied().filter(|&i| !m.on[i]).collect();
                    let pool = if pool.is_empty() { g.elig_idx.iter().copied().filter(|&i| !m.on[i]).collect() } else { pool };
                    if pool.is_empty() {
                        break;
                    }
                    let s = pool[rng.below(pool.len() as u32) as usize];
                    front.push(s);
                    in_front[s] = true;
                }
                let k = rng.below(front.len() as u32) as usize;
                let i = front.swap_remove(k);
                if m.on[i] {
                    continue;
                }
                m.on[i] = true;
                dug += 1;
                let (x, y) = ((i % w) as i32, (i / w) as i32);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if g.elig.at(x + dx, y + dy) {
                        let j = (y + dy) as usize * w + (x + dx) as usize;
                        if !m.on[j] && !in_front[j] {
                            in_front[j] = true;
                            front.push(j);
                        }
                    }
                }
            }
        }
        Null::Walkers => {
            const DIRS: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
            let starts = if g.starts.is_empty() { &g.elig_idx } else { &g.starts };
            if starts.is_empty() {
                return m;
            }
            let mut ws: Vec<(i32, i32, usize)> = Vec::new();
            let mut dug = 0usize;
            for _ in 0..walkers.max(1) {
                let s = starts[rng.below(starts.len() as u32) as usize];
                if !m.on[s] && dug < n {
                    m.on[s] = true;
                    dug += 1;
                }
                ws.push(((s % w) as i32, (s / w) as i32, rng.below(8) as usize));
            }
            let mut steps = 0usize;
            while dug < n && steps < 400 * n.max(1) {
                for wk in ws.iter_mut() {
                    steps += 1;
                    if rng.unit_f32() >= WALKER_PERSIST {
                        wk.2 = rng.below(8) as usize;
                    }
                    let (nx, ny) = (wk.0 + DIRS[wk.2].0, wk.1 + DIRS[wk.2].1);
                    if !(g.elig.at(nx, ny) || m.at(nx, ny)) {
                        wk.2 = rng.below(8) as usize;
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if !m.on[j] {
                        m.on[j] = true;
                        dug += 1;
                    }
                    wk.0 = nx;
                    wk.1 = ny;
                    if dug >= n {
                        break;
                    }
                }
            }
        }
    }
    m
}

/// The colony's percentile against one null, per metric, over `k` draws;
/// and each metric's tie share.
fn score_against(colony: &Panel, colony_mask: &Mask, g: &Ground, kind: Null, k: usize, walkers: usize, seed: (u64, u64)) -> Vec<(f32, f32)> {
    let draws: Vec<Panel> = (0..k)
        .map(|i| {
            let mut rng = pixel_physics::sim::rng::stream(seed.0, seed.1, kind as u64, i as u64);
            panel_of(&draw_null(kind, g, colony_mask, colony.n, walkers, &mut rng), &g.portal)
        })
        .collect();
    (0..METRICS.len())
        .map(|mi| {
            let v: Vec<f32> = draws.iter().map(|p| metric(p, mi)).collect();
            percentile(metric(colony, mi), &v, METRICS[mi].1)
        })
        .collect()
}

/// The `SCORE` and `SPEC` lines for one excavation. Parse with
/// `scripts/nestscore.py`, which orders them by seed.
fn print_score(frame: u64, colony_mask: &Mask, g: &Ground, k: usize, walkers: usize, seed: u64) {
    let p = panel_of(colony_mask, &g.portal);
    if p.n < 10 {
        println!("SCORE frame={frame} n={} (fewer than 10 dug cells: not scored)", p.n);
        return;
    }
    let raw: Vec<String> = (0..METRICS.len())
        .map(|i| {
            let v = if i == 0 { p.mouths } else { metric(&p, i) };
            format!("{}={v:.3}", METRICS[i].0)
        })
        .collect();
    println!("SCORE frame={frame} n={} colony {}", p.n, raw.join(" "));
    for (kind, name) in NULLS {
        let s = score_against(&p, colony_mask, g, kind, k, walkers, (seed, frame));
        let cols: Vec<String> = (0..METRICS.len()).map(|i| format!("{}={:.3}", METRICS[i].0, s[i].0)).collect();
        let ties: Vec<&str> = (0..METRICS.len()).filter(|&i| s[i].1 > 0.9).map(|i| METRICS[i].0).collect();
        let mut live: Vec<f32> = (0..METRICS.len()).filter(|&i| s[i].1 <= 0.9).map(|i| s[i].0).collect();
        live.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let med = if live.is_empty() { f32::NAN } else { live[live.len() / 2] };
        println!(
            "SCORE frame={frame} n={} null={name} k={k} {} median={med:.3} ties={}",
            p.n,
            cols.join(" "),
            if ties.is_empty() { "-".to_string() } else { ties.join(",") }
        );
    }
    // The owner's geometry, absolute: chambers on the roofed part.
    let mut roofed = vec![false; colony_mask.w * colony_mask.h];
    for x in 0..colony_mask.w {
        let mut covered = false;
        for y in 0..colony_mask.h {
            let i = y * colony_mask.w + x;
            if colony_mask.on[i] {
                roofed[i] = covered;
            } else if !g.portal.on[i] {
                covered = true;
            }
        }
    }
    let ch = chambers_of(&roofed, colony_mask.w, colony_mask.h);
    println!(
        "SPEC frame={frame} mouths={} chambers={} passage={} contrast={:.1} widest={}   -- spec: 1 mouth, passage ~4, chambers 8-16 tall and wider than tall, contrast 2-4x",
        p.mouths, ch.count, ch.passage, ch.contrast, ch.max_w
    );
}

/// **What the scoreboard compared, drawn**: the colony's excavation and one
/// draw of each null at the same cell count, one tile each, top to bottom
/// (colony, uniform, rows, eden, walkers). Sky dark, ground brown, dug
/// cells black, the door (painted nest or founding cut) white. `maskout=`
/// writes it at the last stop, because a percentile says how often and
/// never what.
fn mask_tiles(colony: &Mask, g: &Ground, b: &Box2, walkers: usize, seed: u64, frame: u64) -> Vec<Vec<u8>> {
    let mut masks = vec![colony.clone()];
    for (kind, _) in NULLS {
        let mut rng = pixel_physics::sim::rng::stream(seed, frame, kind as u64, 0);
        masks.push(draw_null(kind, g, colony, colony.count(), walkers, &mut rng));
    }
    masks
        .iter()
        .map(|m| {
            let mut buf = vec![0u8; (b.w * b.h * 4) as usize];
            for y in 0..b.h {
                for x in 0..b.w {
                    let row = y - b.surface;
                    let rgba: [u8; 4] = if row < 0 {
                        [28, 32, 44, 255]
                    } else if row as usize >= m.h || x == 0 || x == b.w - 1 {
                        [90, 90, 90, 255]
                    } else if m.at(x, row) {
                        [0, 0, 0, 255]
                    } else if g.portal.at(x, row) || !(g.elig.at(x, row)) {
                        [240, 240, 240, 255]
                    } else {
                        [134, 96, 62, 255]
                    };
                    let i = ((y * b.w + x) * 4) as usize;
                    buf[i..i + 4].copy_from_slice(&rgba);
                }
            }
            buf
        })
        .collect()
}

/// **The scoreboard's own controls, mask-only and in seconds.** Run from
/// `selftest`, before any colony is scored, because a score is only
/// evidence if it can go high for a nest and sits at the middle for noise.
fn scoreboard_selftest() {
    let (w, h) = (200usize, 60usize);
    let c = w as i32 / 2;
    let mut elig = Mask::empty(w, h);
    for y in 0..h {
        for x in 1..w - 1 {
            elig.set(x, y);
        }
    }
    let elig_idx: Vec<usize> = (0..w * h).filter(|&i| elig.on[i]).collect();
    let mut elig_rows = vec![Vec::new(); h];
    for &i in &elig_idx {
        elig_rows[i / w].push(i);
    }
    // A 5-wide door on the surface at the centre, as NEST_DOOR=2 paints it.
    let starts: Vec<usize> = elig_idx.iter().copied().filter(|&i| (i / w) <= 2 && ((i % w) as i32 - c).abs() <= 4).collect();
    let g = Ground { elig, elig_idx, elig_rows, portal: Mask::empty(w, h), starts };

    // (1) POSITIVE: an ideal small nest -- a 4-wide shaft from the surface,
    // 8 rows, into a chamber 8 tall and 13 wide. 136 cells, about what 40
    // ants dig here in 12,000 frames.
    let mut ideal = Mask::empty(w, h);
    for y in 0..8 {
        for x in (c - 2)..(c + 2) {
            ideal.set(x as usize, y);
        }
    }
    for y in 8..16 {
        for x in (c - 6)..=(c + 6) {
            ideal.set(x as usize, y);
        }
    }
    let p = panel_of(&ideal, &g.portal);
    assert_eq!(p.n, 136, "the ideal nest must be 136 cells");
    println!(
        "  scoreboard, ideal nest ({} cells): mouths {} reach {:.2} largest {:.2} roofed {:.2} depth90 {} iqr {} wide {:.2}",
        p.n, p.mouths, p.reach, p.largest, p.roofed, p.depth90, p.iqr, p.wide
    );
    let mut table = Vec::new();
    for (kind, name) in NULLS {
        let s = score_against(&p, &ideal, &g, kind, 200, 40, (7, 0));
        let cols: Vec<String> = (0..METRICS.len()).map(|i| format!("{} {:.2}{}", METRICS[i].0, s[i].0, if s[i].1 > 0.9 { "(tie)" } else { "" })).collect();
        println!("    against {name:>8}: {}", cols.join("  "));
        table.push(s);
    }
    let at = |null: usize, name: &str| table[null][METRICS.iter().position(|m| m.0 == name).unwrap()];
    for (null, name) in [(0, "reach"), (0, "largest"), (1, "reach"), (1, "largest"), (2, "depth90"), (2, "roofed"), (3, "depth90")] {
        assert!(
            at(null, name).0 >= 0.95,
            "the ideal nest scores {:.2} on {name} against {}: a nest by construction must beat that null there, or the score cannot see a nest",
            at(null, name).0,
            NULLS[null].1
        );
    }
    assert!(at(1, "depth90").1 >= 0.9, "the rows null must tie the colony's depth by construction");

    // (2) NEGATIVE: each null against itself must score in the middle. A
    // biased generator, scorer or tie rule reads as a finding.
    for (ni, (kind, name)) in NULLS.iter().enumerate() {
        let mut per_metric: Vec<Vec<f32>> = vec![Vec::new(); METRICS.len()];
        for t in 0..40u64 {
            let mut rng = pixel_physics::sim::rng::stream(99, t, ni as u64, 0);
            let pseudo = draw_null(*kind, &g, &ideal, 136, 40, &mut rng);
            let pp = panel_of(&pseudo, &g.portal);
            let s = score_against(&pp, &pseudo, &g, *kind, 100, 40, (1000 + t, 1));
            for mi in 0..METRICS.len() {
                if s[mi].1 <= 0.9 {
                    per_metric[mi].push(s[mi].0);
                }
            }
        }
        let meds: Vec<String> = per_metric
            .iter_mut()
            .enumerate()
            .map(|(mi, v)| {
                if v.len() < 10 {
                    return format!("{} tie", METRICS[mi].0);
                }
                // **The mean, not the median.** A mid-rank percentile has
                // mean 0.5 against its own distribution by construction,
                // but on a lumpy metric -- `mouths` is 1 for most blobs --
                // its median sits at the mode (measured: eden's `mouths`
                // median 0.73 against itself). 40 draws put the mean within
                // about 0.05 of 0.5, so 0.35-0.65 is three standard errors.
                let mean = v.iter().sum::<f32>() / v.len() as f32;
                assert!(
                    (0.35..=0.65).contains(&mean),
                    "{name} against itself scores a mean {mean:.2} on {}: a null must sit at its own middle",
                    METRICS[mi].0
                );
                format!("{} {mean:.2}", METRICS[mi].0)
            })
            .collect();
        println!("    {name:>8} against itself (mean of 40): {}", meds.join("  "));
    }
}

// =============================================================================
// THE NEST FUNNEL: every ant, booked at the furthest point of nest work it reached
// =============================================================================
//
// The funnel skill's shape (`.claude/skills/funnel/SKILL.md`), for digging:
// count individuals, not events, each at its high-water mark, and gate each
// stage on the state that makes it real rather than on the event that looks
// like it. A dig that cut a pellet somebody put back is not nest work; a
// pellet posted up the column is out of the ground wherever the head is.
//
// Read off the world between frames, with no engine change: an ant ticks at
// most once a frame (`tick_interval` 6), and within a tick it acts before it
// moves, so a dig seen after a frame was aimed at the pre-frame head plus the
// pre-frame heading, and a pellet that left the jaws landed beside that head
// or straight up its column. `target mismatch` and `site not found` count
// every event those two assumptions could not place; they must read 0.

/// The stages, in order. An ant's `stage` is the index of the furthest.
const FUNNEL_STAGES: [&str; 7] = [
    "lived",
    "faced diggable ground, jaws free",
    "cut a cell",
    "cut new ground (below the old surface, never dug or filled)",
    "cut new ground with a roof over it",
    "carried that pellet out above the old surface",
    "the cut still open 1,500 frames later",
];

/// How long a cut must stay open to count as built rather than scratched.
const FUNNEL_LASTING: u64 = 1_500;

#[derive(Clone, Copy)]
struct AntBefore {
    head: (i32, i32),
    heading: u8,
    digs: u32,
    holding: bool,
    /// The held pellet's material: a put-down is a cell of *this* appearing,
    /// so a soil grain falling into the gallery is never read as one.
    pellet: MaterialId,
    crop_empty: bool,
}

/// What the funnel needs of an ant after the frame. Separate from the world
/// so the classification can be driven by hand (`funnel_selftest`):
/// `World::organism_mut` is `pub(crate)`, so an example cannot edit an ant.
#[derive(Clone, Copy)]
struct AntAfter {
    digs: u32,
    holding: bool,
}

#[derive(Clone, Copy, Default)]
struct AntTrack {
    stage: usize,
    /// The cut whose pellet is in the jaws: (x, y, stage it reached).
    cut: Option<(i32, i32, usize)>,
    /// Cuts this ant made that passed every stage: roofed new ground, the
    /// pellet carried out, the cell still open `FUNNEL_LASTING` frames later.
    cycles: u32,
    /// Every cut this ant made, placed or not by stage.
    cuts: u32,
}

#[derive(Default)]
struct NestFunnel {
    ants: std::collections::BTreeMap<u32, AntTrack>,
    before: std::collections::BTreeMap<u32, AntBefore>,
    /// The box as it stood before the frame: `None` is an organism's cell.
    grid: Vec<Option<MaterialId>>,
    /// Cells dug, or filled by a pellet, since frame 0: re-cutting them is
    /// churn, not new ground.
    touched: Vec<bool>,
    /// Cells dug since frame 0: a pellet put down in one is the hole refilled.
    dug: Vec<bool>,
    /// (frame due, cut x, cut y, ant) waiting on the lasting check.
    pending: Vec<(u64, i32, i32, u32)>,
    // The dig ledger: every cut in exactly one bucket.
    cut_above: u64,
    cut_again: u64,
    cut_new_open: u64,
    cut_new_roofed: u64,
    cut_new_lining: u64,
    /// Cuts placed by elimination (the one 8-neighbour of the head that went
    /// ground -> empty) because the cell straight ahead had not: counted so
    /// that a heading assumption that stops holding shows up here first.
    cut_retargeted: u64,
    target_mismatch: u64,
    /// Of `target_mismatch`: the cell ahead was ground before and after the
    /// frame (cut and refilled within it, most likely by falling powder)...
    mismatch_refilled: u64,
    /// ...or it was not ground before the frame at all.
    mismatch_not_ground: u64,
    // The pellet ledger.
    put_beside: u64,
    put_lifted: u64,
    put_above: u64,
    put_below: u64,
    /// Of `put_below`, landed in a cell dug since frame 0: the hole refilled.
    put_refill: u64,
    /// The carrier died holding it; the engine drops it by its corpse.
    died_holding: u64,
    site_not_found: u64,
}

impl NestFunnel {
    fn is_ground(world: &World, m: MaterialId) -> bool {
        m != material::EMPTY && matches!(world.materials.kind(m), MaterialKind::Powder | MaterialKind::Solid)
    }

    /// Every live creature's state, as `before` needs it.
    fn ants_before(world: &World) -> std::collections::BTreeMap<u32, AntBefore> {
        let mut out = std::collections::BTreeMap::new();
        for id in world.live_organism_ids() {
            let Some(st) = world.organism(id) else { continue };
            if world.species.get(st.species).creature.is_none() {
                continue;
            }
            let Some(&head) = st.chain.first() else { continue };
            out.insert(
                id,
                AntBefore {
                    head,
                    heading: st.heading,
                    digs: st.life.digs,
                    holding: st.spoil.is_some(),
                    pellet: st.spoil.map_or(material::EMPTY, |p| p.cell.material),
                    crop_empty: st.crop.is_none(),
                },
            );
        }
        out
    }

    /// Every live creature's state, as `after` needs it. An ant missing here
    /// has died.
    fn ants_after(world: &World) -> std::collections::BTreeMap<u32, AntAfter> {
        let mut out = std::collections::BTreeMap::new();
        for id in world.live_organism_ids() {
            let Some(st) = world.organism(id) else { continue };
            if world.species.get(st.species).creature.is_some() {
                out.insert(id, AntAfter { digs: st.life.digs, holding: st.spoil.is_some() });
            }
        }
        out
    }

    fn before(&mut self, world: &World, b: &Box2) {
        self.before_with(world, b, Self::ants_before(world));
    }

    fn after(&mut self, world: &World, b: &Box2, frame: u64) {
        self.after_with(world, b, frame, &Self::ants_after(world));
    }

    fn before_with(&mut self, world: &World, b: &Box2, ants: std::collections::BTreeMap<u32, AntBefore>) {
        let n = (b.w * b.h) as usize;
        if self.touched.len() != n {
            self.touched = vec![false; n];
            self.dug = vec![false; n];
        }
        self.grid.clear();
        for y in 0..b.h {
            for x in 0..b.w {
                let c = world.get(x, y);
                self.grid.push(if c.organism_id() != 0 { None } else { Some(c.material) });
            }
        }
        for &id in ants.keys() {
            self.ants.entry(id).or_default();
        }
        self.before = ants;
    }

    fn after_with(&mut self, world: &World, b: &Box2, frame: u64, now: &std::collections::BTreeMap<u32, AntAfter>) {
        use pixel_physics::sim::creature::DIRS;
        let at = |x: i32, y: i32| (y * b.w + x) as usize;
        let inside = |x: i32, y: i32| x >= 0 && y >= 0 && x < b.w && y < b.h;
        let ids: Vec<u32> = self.before.keys().copied().collect();
        for id in ids {
            let pre = self.before[&id];
            let track = self.ants.get_mut(&id).expect("registered in before");
            let (dx, dy) = DIRS[pre.heading as usize % 8];
            let (tx, ty) = (pre.head.0 + dx, pre.head.1 + dy);
            // N1: at the moment act ran, jaws free and diggable ground ahead.
            if pre.crop_empty && !pre.holding && inside(tx, ty) {
                if let Some(m) = self.grid[at(tx, ty)] {
                    if Self::is_ground(world, m) && world.materials.get(m).penetration_resistance <= 1.0 {
                        track.stage = track.stage.max(1);
                    }
                }
            }
            let Some(&st) = now.get(&id) else {
                if pre.holding {
                    self.died_holding += 1;
                }
                continue;
            };
            // A cut: the lifetime count rose, and a cell beside the head was
            // ground before the frame and is empty now -- the one straight
            // ahead if it qualifies, else the only one of the other seven.
            let cuts = st.digs.saturating_sub(pre.digs);
            if cuts > 1 {
                // An ant acts at most once a frame; if that ever stops being
                // true the extra cuts cannot be placed, and say so.
                self.target_mismatch += u64::from(cuts - 1);
            }
            if cuts > 0 {
                // Opened: ground before the frame, and now either empty or
                // standing under an animal -- a digger acts and then moves,
                // and the cell it has just cut is the one it steps into.
                let opened = |x: i32, y: i32| {
                    inside(x, y) && matches!(self.grid[at(x, y)], Some(was) if Self::is_ground(world, was)) && {
                        let now = world.get(x, y);
                        now.material == material::EMPTY || now.organism_id() != 0
                    }
                };
                let target = if opened(tx, ty) {
                    Some((tx, ty))
                } else {
                    let others: Vec<(i32, i32)> = DIRS.iter().map(|&(ox, oy)| (pre.head.0 + ox, pre.head.1 + oy)).filter(|&(x, y)| (x, y) != (tx, ty) && opened(x, y)).collect();
                    if others.len() == 1 {
                        self.cut_retargeted += 1;
                        Some(others[0])
                    } else {
                        None
                    }
                };
                if let Some((tx, ty)) = target {
                    let was = self.grid[at(tx, ty)];
                    debug_assert!(was.is_some(), "opened() admits only a material cell");
                    let row = ty - b.surface;
                    let reached = if row < 0 {
                        self.cut_above += 1;
                        2
                    } else if self.touched[at(tx, ty)] {
                        self.cut_again += 1;
                        2
                    } else {
                        if world.materials.id_of("packedsoil") == was {
                            self.cut_new_lining += 1;
                        }
                        let roofed = (b.surface..ty).any(|yy| matches!(self.grid[at(tx, yy)], Some(m) if Self::is_ground(world, m)));
                        if roofed {
                            self.cut_new_roofed += 1;
                            4
                        } else {
                            self.cut_new_open += 1;
                            3
                        }
                    };
                    self.touched[at(tx, ty)] = true;
                    self.dug[at(tx, ty)] = true;
                    track.cuts += 1;
                    track.stage = track.stage.max(reached);
                    track.cut = Some((tx, ty, reached));
                } else {
                    self.target_mismatch += 1;
                    // Why, for the cell straight ahead: not ground before the
                    // frame, or ground again after it.
                    match (inside(tx, ty).then(|| self.grid[at(tx, ty)]).flatten(), inside(tx, ty).then(|| world.get(tx, ty))) {
                        (Some(was), Some(now)) if Self::is_ground(world, was) && now.organism_id() == 0 && Self::is_ground(world, now.material) => self.mismatch_refilled += 1,
                        _ => self.mismatch_not_ground += 1,
                    }
                }
            }
            // A put-down: the pellet left the jaws. Beside the head first,
            // then straight up its column, as the engine places it.
            if pre.holding && !st.holding {
                let (hx, hy) = pre.head;
                let appeared = |x: i32, y: i32| inside(x, y) && self.grid[at(x, y)] == Some(material::EMPTY) && world.get(x, y).material == pre.pellet;
                let beside = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
                    .iter()
                    .map(|&(ox, oy)| (hx + ox, hy + oy))
                    .find(|&(x, y)| appeared(x, y));
                let site = beside.map(|s| (s, false)).or_else(|| (1..=160).map(|dy| (hx, hy - dy)).find(|&(x, y)| appeared(x, y)).map(|s| (s, true)));
                match site {
                    None => self.site_not_found += 1,
                    Some(((sx, sy), lifted)) => {
                        if lifted {
                            self.put_lifted += 1;
                        } else {
                            self.put_beside += 1;
                        }
                        let out = sy < b.surface;
                        if out {
                            self.put_above += 1;
                        } else {
                            self.put_below += 1;
                            if self.dug[at(sx, sy)] {
                                self.put_refill += 1;
                            }
                        }
                        self.touched[at(sx, sy)] = true;
                        if let Some((cx, cy, reached)) = track.cut.take() {
                            if out && reached >= 4 {
                                track.stage = track.stage.max(5);
                                self.pending.push((frame + FUNNEL_LASTING, cx, cy, id));
                            }
                        }
                    }
                }
            }
        }
        // The lasting check: the cut is still room (empty or an animal).
        let due: Vec<(u64, i32, i32, u32)> = self.pending.iter().copied().filter(|p| p.0 <= frame).collect();
        self.pending.retain(|p| p.0 > frame);
        for (_, x, y, id) in due {
            let c = world.get(x, y);
            let open = c.material == material::EMPTY || world.materials.kind(c.material) == MaterialKind::Creature;
            if open {
                if let Some(t) = self.ants.get_mut(&id) {
                    t.stage = t.stage.max(6);
                    t.cycles += 1;
                }
            }
        }
    }

    fn print(&self, frame: u64, world: &World) {
        let st = world.creature_stats;
        let total = self.ants.len();
        println!("FUNNEL frame={frame} ants={total}  (booked at the furthest stage each ant ever reached)");
        let mut prev = total;
        for (i, name) in FUNNEL_STAGES.iter().enumerate() {
            let n = self.ants.values().filter(|t| t.stage >= i).count();
            println!(
                "FUNNEL   {name:<60} {n:>4}  of prev {:>5.1}%  of all {:>5.1}%",
                if prev > 0 { 100.0 * n as f64 / prev as f64 } else { 0.0 },
                if total > 0 { 100.0 * n as f64 / total as f64 } else { 0.0 }
            );
            prev = n;
        }
        let mut cyc = [0usize; 4];
        for t in self.ants.values() {
            cyc[(t.cycles as usize).min(3)] += 1;
        }
        println!(
            "FUNNEL   complete cycles per ant: 0: {}  1: {}  2: {}  3+: {}   (cuts still waiting on the lasting check: {})",
            cyc[0], cyc[1], cyc[2], cyc[3], self.pending.len()
        );
        // **The rate, which the high-water mark cannot show.** Over a long run
        // nearly every ant reaches every stage once by chance, so the stages
        // saturate; what separates building from scratching is how much of
        // the work built something.
        let built: u64 = self.ants.values().map(|t| u64::from(t.cycles)).sum();
        let placed: u64 = self.ants.values().map(|t| u64::from(t.cuts)).sum();
        let mut per: Vec<u32> = self.ants.values().map(|t| t.cuts).collect();
        per.sort_unstable();
        println!(
            "FUNNEL   cuts that built {built} of {placed} placed cuts ({:.1}%); cuts per ant: median {} max {}",
            if placed > 0 { 100.0 * built as f64 / placed as f64 } else { 0.0 },
            per.get(per.len() / 2).copied().unwrap_or(0),
            per.last().copied().unwrap_or(0)
        );
        let cuts = self.cut_above + self.cut_again + self.cut_new_open + self.cut_new_roofed;
        println!(
            "LEDGER frame={frame} cuts {cuts} + target mismatch {} = engine digs {}: above the old surface {}, a pellet or refill cut again {}, new ground open to the sky {}, new ground under a roof {} (of the new ground, tunnel lining {}; placed by elimination {}); mismatch: ahead refilled {}, ahead not ground {}",
            self.target_mismatch,
            st.digs,
            self.cut_above,
            self.cut_again,
            self.cut_new_open,
            self.cut_new_roofed,
            self.cut_new_lining,
            self.cut_retargeted,
            self.mismatch_refilled,
            self.mismatch_not_ground
        );
        println!(
            "LEDGER frame={frame} pellets put down {} + died holding {} + site not found {} = engine spoil_dumped {} + spoil_lost {}: beside the head {}, posted up the column {}; landed above the old surface {}, below it {} (into a dug cell {})",
            self.put_beside + self.put_lifted,
            self.died_holding,
            self.site_not_found,
            st.spoil_dumped,
            st.spoil_lost,
            self.put_beside,
            self.put_lifted,
            self.put_above,
            self.put_below,
            self.put_refill
        );
        // Who to trace: a few ids stopped at each stage.
        for (i, name) in FUNNEL_STAGES.iter().enumerate().skip(1) {
            let stuck: Vec<String> = self.ants.iter().filter(|(_, t)| t.stage == i).take(6).map(|(id, _)| id.to_string()).collect();
            if !stuck.is_empty() {
                println!("FUNNEL   stopped at \"{name}\": {}", stuck.join(" "));
            }
        }
    }
}

/// **The funnel's positive control**: one ant driven by hand through a
/// complete cycle and through two that must not count, then a dig that
/// changed no cell, which the ledger must refuse to place. The funnel reads
/// only the world, so a hand-edited world is a fair test of it: every edit
/// here is one the engine makes in `act` (cell emptied, `life.digs` bumped,
/// pellet into `spoil`; pellet written back, `spoil` cleared).
fn funnel_selftest(b: &Box2) {
    use pixel_physics::sim::creature::DIRS;
    let mut w = build(b);
    let spoil = w.materials.id_of("spoil").expect("spoil is a shipped material");
    let soil = w.materials.id_of("soil").expect("soil is a shipped material");
    // A gallery six rows down for the ant to stand in, soil above it.
    let (gx, gy) = (b.w / 2 - 6, b.surface + 6);
    for x in gx..gx + 12 {
        w.set(x, gy, Cell::EMPTY);
    }
    // The ant is a record, not a body: the funnel reads cells from the world
    // and each ant's state from `before_with`/`after_with`, so its head can
    // stand in the gallery without a live organism there.
    let id = 1u32;
    let head = (gx + 5, gy);
    let pellet = Cell::new(spoil, 0);
    let mut fun = NestFunnel::default();
    let mut frame = 0u64;
    let mut digs = 0u32;
    let ant = |heading: u8, digs: u32, holding: bool| {
        std::collections::BTreeMap::from([(id, AntBefore { head, heading, digs, holding, pellet: if holding { spoil } else { material::EMPTY }, crop_empty: true })])
    };
    let after = |digs: u32, holding: bool| std::collections::BTreeMap::from([(id, AntAfter { digs, holding })]);
    // One dig by hand: face `dir`, cut the cell ahead, take the pellet.
    let mut cut = |w: &mut World, fun: &mut NestFunnel, frame: u64, dir: u8| {
        fun.before_with(w, b, ant(dir, digs, false));
        let (tx, ty) = (head.0 + DIRS[dir as usize].0, head.1 + DIRS[dir as usize].1);
        w.set(tx, ty, Cell::EMPTY);
        digs += 1;
        fun.after_with(w, b, frame, &after(digs, true));
        (tx, ty)
    };
    // One put-down by hand, at `(x, y)`. `digs` is not read on a put-down.
    let put = |w: &mut World, fun: &mut NestFunnel, frame: u64, x: i32, y: i32, digs: u32| {
        fun.before_with(w, b, ant(0, digs, true));
        w.set(x, y, pellet);
        fun.after_with(w, b, frame, &after(digs, false));
    };
    let idle = |w: &mut World, fun: &mut NestFunnel, frame: u64, digs: u32| {
        fun.before_with(w, b, ant(0, digs, false));
        fun.after_with(w, b, frame, &after(digs, false));
    };

    // 1. The whole cycle: roofed new ground straight down, the pellet posted
    //    up the column to the air, the cut still open 1,500 frames on.
    frame += 1;
    let first = cut(&mut w, &mut fun, frame, 6);
    frame += 1;
    put(&mut w, &mut fun, frame, head.0, b.surface - 1, 1);
    frame += FUNNEL_LASTING;
    idle(&mut w, &mut fun, frame, 1);
    let t = fun.ants[&id];
    println!(
        "  funnel: a hand-driven tunnel cut, pellet posted out, still open {FUNNEL_LASTING} frames on -> stage {} ({}), cycles {} (must be 6 and 1)",
        t.stage, FUNNEL_STAGES[t.stage], t.cycles
    );
    assert_eq!((t.stage, t.cycles, fun.cut_new_roofed, fun.put_lifted, fun.put_above), (6, 1, 1, 1, 1), "the funnel must book a complete cycle as one");

    // 2. A pellet put straight back into the hole it came from: refill, and
    //    no second cycle.
    frame += 1;
    cut(&mut w, &mut fun, frame, 5);
    frame += 1;
    put(&mut w, &mut fun, frame, first.0, first.1, 2);
    // 3. A cut whose pellet goes out and which caves in before it has stood
    //    1,500 frames: carried out, and not built.
    frame += 1;
    let third = cut(&mut w, &mut fun, frame, 7);
    frame += 1;
    put(&mut w, &mut fun, frame, head.0, b.surface - 2, 3);
    frame += 1;
    fun.before_with(&w, b, ant(0, 3, false));
    w.set(third.0, third.1, Cell::new(soil, 0));
    fun.after_with(&w, b, frame, &after(3, false));
    frame += FUNNEL_LASTING;
    idle(&mut w, &mut fun, frame, 3);
    let t = fun.ants[&id];
    println!(
        "  funnel: + a pellet put back in its own hole, + a cut that caved in before {FUNNEL_LASTING} frames -> cycles {}, refill {}, roofed cuts {}, out {} (must be 1, 1, 3, 2)",
        t.cycles, fun.put_refill, fun.cut_new_roofed, fun.put_above
    );
    assert_eq!((t.cycles, fun.put_refill, fun.put_beside, fun.cut_new_roofed, fun.put_above, fun.put_below), (1, 1, 1, 3, 2, 1), "a refilled or caved-in cut must not count as built");

    // 4. The ledger's own control: a dig the world does not show must be
    //    booked as unplaceable, not quietly dropped or guessed.
    fun.before_with(&w, b, ant(0, 3, false));
    frame += 1;
    fun.after_with(&w, b, frame, &after(4, false));
    println!("  funnel: a dig that changed no cell -> target mismatch {} (must be 1; before it, 0)", fun.target_mismatch);
    assert_eq!((fun.target_mismatch, fun.site_not_found), (1, 0), "the ledger must refuse to place a cut it cannot see");
}

/// **The moisture gradient as the ant's own brain reads it**, sampled at
/// every ant's head.
///
/// `CLAUDE.md`: *measure the number the consumer computes, never the stored
/// value.* The stored value is a `u16` of soil water; what reaches the dig
/// decision is `moisture_gradient`, and the two can disagree completely -- a
/// uniformly saturated bed stores a large number everywhere and presents a
/// gradient of exactly zero.
fn moisture_seen(world: &World) -> (f32, f32) {
    let mut v: Vec<f32> = Vec::new();
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if world.species.get(st.species).creature.is_none() {
            continue;
        }
        let Some(&(hx, hy)) = st.chain.first() else { continue };
        v.push(pixel_physics::sim::creature::moisture_gradient(world, hx, hy));
    }
    if v.is_empty() {
        return (0.0, 0.0);
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = v.iter().sum::<f32>() / v.len() as f32;
    (mean, v[v.len() / 2])
}

/// How much charge the colony has left, as a fraction of what it started
/// with — the number that says whether a quiet census means *"they stopped
/// digging"* or *"they ran out"*, which are the same picture.
fn charge(world: &World) -> (usize, f32) {
    let mut n = 0;
    let mut total = 0.0f32;
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if world.species.get(st.species).creature.is_none() {
            continue;
        }
        n += 1;
        total += st.energy;
    }
    (n, if n > 0 { total / n as f32 } else { 0.0 })
}


/// Put a colony of `ants` in the box, a few at a time.
///
/// **Two ceilings, and neither is a bug in the engine.** `found_colony_of`
/// places on `colony_stations`, spaced `COLONY_ANT_SPACING` (4) apart inside
/// the patch's `COLONY_HALF_WIDTH` (26), so the founding verb tops out near
/// **49** however many are asked for -- measured, `ants=200` founded 49. And
/// placing the shortfall all at once in one frame stacks them forty rows into
/// the air, which is not a colony, it is a tower.
///
/// The lab reaches thousands because ants are **born over time** into space
/// that the previous ones have left. This does the same thing without
/// reproduction: a few per frame, at the nest, until the target is met.
/// Ants placed earlier have walked off by the time the next batch lands, so
/// the same few rows take an unbounded number of animals.
///
/// **Every one carries the first ant's colony label.** `plant_ant` in a loop
/// mints a fresh colony per call, and `instruments.md` records a harness that
/// did exactly that and spent months reporting on **55 mutual strangers**
/// rather than a colony.
///
/// **And it takes a seed, because this box had none and twelve seeds is the
/// house rule for anything chaotic in the seed.** The box's scene is fixed
/// -- one stone shell, one uniform fill, no noise anywhere -- so the only
/// thing a seed can vary is *which colony* gets founded in it. `seed=0` is
/// the shipped left-to-right walk and is bit-exact with every number taken
/// here before 2026-09-19; `seed=N` shuffles the order the patch's columns
/// are filled in, which is `examples/burrow_probe`'s own device and for the
/// same reason: an ant placed at a different column starts its walk facing
/// different ground, and by frame 6,000 that is a different colony rather
/// than the same one slid sideways.
///
/// **Why a seed was needed at all**, since a deterministic box is a virtue:
/// every arm-versus-arm number this line published came from **one run per
/// arm**, which `CLAUDE.md` calls a sample from a wide distribution. It is
/// not a hypothetical here -- the three `Crowding` nulls were scored that
/// way, and `dead-ends.md`'s own `(Crowding, Dig, 0.6)` entry records four
/// seeds reading 4 of 4 and twelve reading 16 of 33.
struct Trickle {
    colony: Option<u32>,
    placed: usize,
    target: usize,
    /// How many to try per frame. The cap is space, not this.
    rate: usize,
    cursor: i32,
    /// The order the patch's columns are tried in. Empty at `seed=0`, which
    /// is the plain walk.
    order: Vec<i32>,
}

impl Trickle {
    fn new(target: usize, rate: usize, seed: u64, span: i32) -> Self {
        let mut order = Vec::new();
        if seed > 0 {
            use pixel_physics::sim::rng;
            order = (0..span).collect();
            let mut draw = rng::stream(seed, 0xD1_6B0C, 0, 0);
            for i in (1..order.len()).rev() {
                order.swap(i, draw.below(i as u32 + 1) as usize);
            }
        }
        Trickle { colony: None, placed: 0, target, rate, cursor: 0, order }
    }

    fn done(&self) -> bool {
        self.placed >= self.target
    }

    /// Try to place up to `rate` more ants along the nest patch. Returns how
    /// many landed this call; a full row simply places nobody and is retried
    /// next frame, which is what makes the trickle self-pacing.
    fn step(&mut self, world: &mut World, b: &Box2) -> usize {
        if self.done() {
            return 0;
        }
        let cx = b.w / 2;
        let half = 26.min(b.w / 2 - 2);
        let span = half * 2 + 1;
        let mut landed = 0;
        for _ in 0..self.rate {
            if self.done() {
                break;
            }
            // Walk the patch rather than drawing at random: a deterministic
            // sweep keeps the box seed-free, and `CLAUDE.md` wants
            // determinism for same-build runs.
            self.cursor = (self.cursor + 1) % span;
            // `seed=0` keeps the bare walk, so every figure taken from this
            // box before it gained a seed still reproduces to the cell.
            let col = if self.order.is_empty() { self.cursor } else { self.order[self.cursor as usize] };
            let x = cx - half + col;
            let y = b.surface - 1;
            if let Some(site) = pixel_physics::sim::creature::plant_creature_seed_in(world, x, y, "ant", self.colony) {
                if self.colony.is_none() {
                    self.colony = pixel_physics::sim::creature::colony_of_site(world, &site);
                }
                world.schedule_active_site(site);
                self.placed += 1;
                landed += 1;
            }
        }
        landed
    }
}

/// **What every ant is actually sensing, and what it decides, at one tick.**
///
/// Built bottom-up: an arm-versus-arm census says *that* the nest is a lens
/// and cannot say *why*. This reads the real thing through
/// `creature::probe_full`, which runs `sense` and `eval_brain` exactly as the
/// tick does and hands back the true input, hidden and output vectors --
/// the same instrument `trailfollow` traces its focal ant with. Reproducing
/// the wiring by hand here was the first version and was deleted: a replica
/// drifts silently the moment a weight or a sense changes.
///
/// The question it exists to answer is Toffin's.
/// `Reports/nest-biology-2026-09-19.md` §4.1: in homogeneous 2D excavation
/// the transition from a round cavity to a **branched** structure is driven
/// by worker density *along the perimeter* -- a local reading. High density
/// digs uniformly and gives a circle; falling density gives localised buds
/// and gives branching. **A rule whose inputs are identical for every ant
/// has no spatial variation for a bud to form at**, so it can only ever
/// produce the circle. The `distinct` column is that claim, measured.
/// **Candidate *local* senses, computed here and wired to nothing.**
///
/// The biology names worker density *along the excavation perimeter* as the
/// quantity that decides round-cavity against branched network
/// (`Reports/nest-biology-2026-09-19.md` §4.1). The engine has a local
/// worker count already -- `sense`'s `density` -- and it is useless for that
/// purpose by construction: `CROWDING_RADIUS` 2 gives a 5x5 of 24 neighbours
/// and `CROWDING_SCALE` 8 divides by eight, so **four nearby ants pin it at
/// 1.000** and it never moves again. `dead-ends.md`'s `(Crowding, Dig, 0.6)`
/// entry measured exactly that: median 1.000 with p90 and max pinned.
///
/// So the question before wiring anything is not *"should a local sense drive
/// digging"* -- it is **"is there a local reading in this world that varies
/// at all, and over what range"**. A sense with no range cannot break a
/// symmetry however it is weighted, and this repo has shipped that mistake in
/// three unrelated subsystems (`CLAUDE.md`, the decaying-gradient rule).
///
/// Five candidates, each returning 0..1, none of them connected to a brain
/// slot. `workers_r2_s8` reproduces the shipped `density` exactly so the
/// table carries its own control: if that column shows range, this function
/// disagrees with `sense` and nothing else here should be believed.
fn local_senses(world: &World, x: i32, y: i32, me: u32) -> [f32; 5] {
    let count = |r: i32, want_creature: bool| {
        let mut n = 0;
        let mut total = 0;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx == 0 && dy == 0 {
                    continue;
                }
                total += 1;
                let cell = world.get(x + dx, y + dy);
                if cell.organism_id() == me {
                    continue;
                }
                let is_creature = world.materials.kind(cell.material) == MaterialKind::Creature;
                if want_creature == is_creature {
                    n += 1;
                }
            }
        }
        (n as f32, total as f32)
    };
    let (c2, n2) = count(2, true);
    let (c6, n6) = count(6, true);
    // Open space right here: empty cells in a radius-4 disc.
    let mut open = 0.0;
    let mut open_total = 0.0;
    for dy in -4..=4 {
        for dx in -4..=4 {
            if dx == 0 && dy == 0 {
                continue;
            }
            open_total += 1.0;
            if world.get(x + dx, y + dy).material == material::EMPTY {
                open += 1.0;
            }
        }
    }
    // Am I at a face? Diggable ground in the 8-neighbourhood.
    let mut face = 0.0;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let cell = world.get(x + dx, y + dy);
            let kind = world.materials.kind(cell.material);
            if cell.material != material::EMPTY
                && matches!(kind, MaterialKind::Powder | MaterialKind::Solid)
                && cell.organism_id() == 0
            {
                face += 1.0;
            }
        }
    }
    [
        (c2 / 8.0).min(1.0), // the shipped `density`, reproduced -- the control
        c2 / n2,             // same count, full normaliser
        c6 / n6,             // a wider perimeter reading
        open / open_total,   // how open it is right here
        face / 8.0,          // how much wall is within reach
    ]
}

fn trace(world: &World) {
    use pixel_physics::sim::brain::{BrainInput as I, BrainOutput as O};
    let Some(sid) = world.species.id_of("ant") else { return };
    let Some(def) = world.species.get(sid).creature.as_ref().cloned() else { return };

    // Every input that reaches `Dig` in `ant.ron`, and `SurfaceCurvature`,
    // which reaches `DropSpoil` instead: the put-down is what frees the jaws
    // for the next dig. Plus the decision itself.
    let watch: [(&str, usize); 5] = [
        ("food beside it", I::FoodAdjacent as usize),
        ("am I home", I::AtNest as usize),
        ("how packed it feels", I::Crowding as usize),
        ("wetness gradient", I::MoistureGrad as usize),
        ("shape of the wall", I::SurfaceCurvature as usize),
    ];
    let mut cols: Vec<Vec<f32>> = vec![Vec::new(); watch.len()];
    let mut digs: Vec<f32> = Vec::new();
    let mut local: Vec<Vec<f32>> = vec![Vec::new(); 5];
    let local_names = [
        "nestmates near (shipped)",
        "nestmates near, ranged",
        "nestmates, wide reach",
        "open space right here",
        "wall within reach",
    ];
    let mut at_nest = 0usize;
    let (mut xs, mut ys): (Vec<i32>, Vec<i32>) = (Vec::new(), Vec::new());

    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if world.species.get(st.species).creature.is_none() {
            continue;
        }
        let Some(&(hx, hy)) = st.chain.first() else { continue };
        let (inp, _hid, out, _) = pixel_physics::sim::creature::probe_full(world, hx, hy, id, &def);
        if inp[I::AtNest as usize] <= 0.0 {
            continue;
        }
        at_nest += 1;
        xs.push(hx);
        ys.push(hy);
        for (k, (_, slot)) in watch.iter().enumerate() {
            cols[k].push(inp[*slot]);
        }
        digs.push(out[O::Dig as usize].clamp(0.0, 1.0));
        let l = local_senses(world, hx, hy, world.get(hx, hy).organism_id());
        for (k, v) in l.iter().enumerate() {
            local[k].push(*v);
        }
    }

    if at_nest == 0 {
        println!("  trace: not one ant is at the nest this tick");
        return;
    }
    let distinct = |v: &[f32]| {
        let mut u: Vec<i64> = v.iter().map(|x| (x * 1e6) as i64).collect();
        u.sort_unstable();
        u.dedup();
        u.len()
    };
    let lohi = |v: &[f32]| {
        let mut w = v.to_vec();
        w.sort_by(|a, c| a.partial_cmp(c).unwrap());
        (w[0], w[w.len() - 1])
    };
    let dx = xs.iter().max().unwrap() - xs.iter().min().unwrap();
    let dy = ys.iter().max().unwrap() - ys.iter().min().unwrap();
    println!("  trace: {at_nest} ants at the nest, spread over {dx} columns x {dy} rows");
    println!("         {:<22} {:>9} {:>9} {:>9}", "what it senses", "lowest", "highest", "distinct");
    for (k, (name, _)) in watch.iter().enumerate() {
        let (lo, hi) = lohi(&cols[k]);
        println!("         {:<22} {lo:>9.4} {hi:>9.4} {:>9}", name, distinct(&cols[k]));
    }
    let (lo, hi) = lohi(&digs);
    println!("         {:<22} {lo:>9.4} {hi:>9.4} {:>9}   <-- what it DECIDES", "dig probability", distinct(&digs));
    println!("         -- candidate LOCAL senses, computed here and wired to nothing --");
    for (k, name) in local_names.iter().enumerate() {
        let (lo, hi) = lohi(&local[k]);
        println!("         {:<22} {lo:>9.4} {hi:>9.4} {:>9}", name, distinct(&local[k]));
    }
}

fn main() {
    let ants: i32 = arg("ants").unwrap_or(40);
    let soil: i32 = arg("soil").unwrap_or(60);
    let sky: i32 = arg("sky").unwrap_or(24);
    let w: i32 = arg("w").unwrap_or(200);
    let selftest = flag("selftest");
    let b = Box2::new(w, sky + soil + 8, soil, sky);

    if selftest {
        selftest_run(&b);
        return;
    }

    let frames: u64 = arg("frames").unwrap_or(12_000);
    let out: Option<String> = arg("out");
    // **`tintout=` writes the same stops a second time with every class of
    // ground painted flat**, from the same run -- so the plain sheet and the
    // tinted one are the same instants, not two runs that happened to agree.
    // See [`tint`] for the colours and why they are a full replace.
    let tint_out: Option<String> = arg("tintout");
    let scale: u32 = arg("scale").unwrap_or(3);

    let wet: u16 = arg("wet").unwrap_or(material::SOIL_FIELD_CAPACITY);
    let grad: Option<(u16, u16)> = arg::<String>("wetgrad").map(|v| {
        let (a, c) = v.split_once(':').unwrap_or_else(|| panic!("wetgrad= wants top:bottom, got `{v}`"));
        (a.trim().parse().expect("wetgrad top"), c.trim().parse().expect("wetgrad bottom"))
    });
    let mut world = build_graded(&b, wet, grad);
    match grad {
        Some((top, bottom)) => println!("  bed graded aux {top} (top) -> {bottom} (bottom) over {} rows", b.floor - b.surface),
        None => println!("  bed at uniform aux {wet}  [wilting 180, field capacity 620, saturated 1000]"),
    }

    // **The endowment is a knob here, and it has to be.** A digger spends
    // `dig_cost_in_moves` 6.0 per cell, so 200 ants on the shipped
    // `start_energy: 200` are dead by frame 6,000 and the census is measuring
    // a die-off rather than a nest. This is a test box: the question is what a
    // colony digs, not whether it can feed itself, and starvation is the lab's
    // question. `energy=200` restores the shipped value.
    {
        let id = world.species.id_of("ant").expect("ant ships");
        if let Some(c) = world.species.get_mut(id).creature.as_mut() {
            c.start_energy = arg("energy").unwrap_or(20_000.0);
        }
    }
    // **`gate=` re-centres the chamber gate on the band `Crowding` actually
    // occupies**, patched into the genome here rather than edited into
    // `ant.ron` so an arm and its control run from one binary.
    //
    // `ant.ron` authors `(Bias, 5, -30)`, `(AtNest, 5, +30)`,
    // `(Crowding, 5, +6)` and the mirror on unit 6, and units 5 and 6 drive
    // **`Dig` and nothing else** -- so this reallocates nothing outside the
    // dig decision, and `(Crowding, Move, -0.3)` is a separate direct weight
    // that is not touched.
    //
    // At the nest the pair reduces to `2.5*squash(6c) - 2.5*squash(-6c)`,
    // and `6c` over the realised band 0.57..0.92 is 3.4..5.5 -- already deep
    // in `squash`. Measured: the input moves a third of its scale and the
    // decision moves 0.013, a **26x compression**. Lowering the `AtNest`
    // weight shifts the pair's operating point down into the responsive part
    // of the curve without touching what it does away from the nest, where
    // `AtNest` is 0 and the weight cannot apply: at 25.5 the same band spans
    // **0.517..0.787**, a swing of 0.269 against 0.013, and the
    // away-from-nest value is 0.152 either way.
    if let Some(g) = arg::<f32>("gate") {
        use pixel_physics::sim::brain::{ih_slot, BrainInput};
        let id = world.species.id_of("ant").expect("ant ships");
        let mut genome = world.species.get(id).genome.clone();
        genome[ih_slot(BrainInput::AtNest, 5)] = g;
        genome[ih_slot(BrainInput::AtNest, 6)] = g;
        world.species.set_genome(id, genome);
        println!("  chamber gate re-centred: (AtNest, 5/6) = {g} instead of the authored 30.0");
    }
    // **`curvdig=` wires the one local sense the ant already has to `Dig`.**
    //
    // `ant.ron` authors `(SurfaceCurvature, Drop, 0.169)` and
    // `(SurfaceCurvature, DropSpoil, 0.169)` -- a positive weight on
    // *exposure* for deposition, which is the ant/termite construction rule
    // exactly: material accumulates on bumps, bumps become pillars. The
    // engine therefore already ships the **deposition** half of stigmergy.
    // It ships no excavation counterpart: `(SurfaceCurvature, Dig, w)` is
    // absent, and `Dig` reads nothing that varies from one cell to the next.
    //
    // Measured 2026-09-19: of the five senses reaching `Dig`, four hold a
    // single value across the whole colony and curvature holds **16 distinct
    // values among 51 ants standing in one nest at one tick**. It is the only
    // spatial signal available to the decision.
    //
    // SIGN. `surface_curvature` returns `+1` for a spike of ground with
    // nothing around it and `-1` for buried. So a **negative** weight digs
    // harder where the animal is already enclosed -- the tunnel-deepening
    // feedback, a dent becoming a gallery -- and a positive weight digs
    // harder at an exposed face, widening the open pit. The sign is exactly
    // the kind of thing this repo measures rather than argues, so sweep it.
    if let Some(w) = arg::<f32>("curvdig") {
        use pixel_physics::sim::brain::{io_slot, BrainInput, BrainOutput};
        let id = world.species.id_of("ant").expect("ant ships");
        let mut genome = world.species.get(id).genome.clone();
        genome[io_slot(BrainInput::SurfaceCurvature, BrainOutput::Dig)] = w;
        world.species.set_genome(id, genome);
        println!("  (SurfaceCurvature, Dig) = {w}  [negative digs where buried; positive digs at an exposed face]");
    }
    // **`wire=Input:Output:w,...` -- any genome weight by name, at runtime.**
    // `gate=` and `curvdig=` above each needed their own argument and their
    // own slot line, so sweeping a new pair meant editing this file; and
    // editing `ant.ron` instead cannot work at all, because species assets
    // are `include_str!`ed (`src/sim/organism.rs:7345`) and a prebuilt binary
    // re-run against an edited `.ron` gives bit-identical "runs". Same
    // argument, same spelling and same refusal as `examples/burrow_probe`'s.
    //
    // It prints every pair with its before value. A knob nobody can see the
    // value of is a knob nobody can tell is disconnected -- which is not
    // hypothetical: a `wet=` added to this line of work on 2026-09-19 was
    // inserted into the wrong arm's branch, never ran, and produced a whole
    // "dead at every wetness" finding that had to be retracted.
    if let Some(spec) = arg::<String>("wire") {
        use pixel_physics::sim::brain::{INPUT_NAMES, INPUT_SLOTS, OUTPUT_NAMES};
        let id = world.species.id_of("ant").expect("ant ships");
        let mut genome = world.species.get(id).genome.clone();
        let mut set: Vec<String> = Vec::new();
        for triple in spec.split(',').filter(|t| !t.trim().is_empty()) {
            let parts: Vec<&str> = triple.split(':').collect();
            assert_eq!(parts.len(), 3, "wire= wants Input:Output:weight triples, got `{triple}`");
            let find = |names: &[&str], want: &str, what: &str| -> usize {
                names.iter().position(|n| n.eq_ignore_ascii_case(want.trim())).unwrap_or_else(|| {
                    panic!("wire=: no such brain {what} `{}`. Known {what}s: {}", want.trim(), names.join(", "))
                })
            };
            let i = find(&INPUT_NAMES, parts[0], "input");
            let o = find(&OUTPUT_NAMES, parts[1], "output");
            let w: f32 = parts[2].trim().parse().unwrap_or_else(|_| panic!("wire=: `{}` is not a weight", parts[2]));
            let before = genome[o * INPUT_SLOTS + i];
            genome[o * INPUT_SLOTS + i] = w;
            set.push(format!("({}, {}) {before} -> {w}", INPUT_NAMES[i], OUTPUT_NAMES[o]));
        }
        world.species.set_genome(id, genome);
        println!("  PATCHED genome by name: {}", set.join("; "));
    }
    world.paint_nest_patch(b.w / 2, b.surface - 1);
    let seed: u64 = arg("seed").unwrap_or(0);
    let span = 26.min(b.w / 2 - 2) * 2 + 1;
    let mut trickle = Trickle::new(ants as usize, arg("rate").unwrap_or(4), seed, span);

    // The endowment horizon, printed rather than assumed -- a run past it is
    // measuring starvation, not digging.
    let id = world.species.id_of("ant").expect("ant ships");
    let def = world.species.get(id).creature.as_ref().expect("ant is a creature");
    let per_tick = def.idle_cost_per_cell * def.body.len() as f32;
    let horizon = (def.start_energy / per_tick) as u64 * def.tick_interval.max(1);

    println!(
        "digbox: {}x{} box, soil {} rows ({}..{}), colony of {} trickled in at {}/frame, seed={} ({})",
        b.w, b.h, soil, b.surface, b.floor, ants, trickle.rate, seed,
        if seed == 0 { "the shipped walk; the scene itself carries no noise" } else { "founding order shuffled" }
    );
    println!(
        "  no food, no plants, no lamps, no weather -- FoodAdjacent is 0 by construction, so the dig you see is the nest mechanism alone"
    );
    println!(
        "  idle endowment {horizon} frames (start_energy {} / {:.2} per tick x tick_interval {}); running {frames}{}",
        def.start_energy,
        per_tick,
        def.tick_interval,
        if frames > horizon { "  *** PAST THE ENDOWMENT: this run measures starvation ***" } else { "" }
    );
    match pixel_physics::sim::creature::nest_site_rows() {
        Some(r) => println!("  AtNest: SITE-based, reach {r} rows (PIXEL_PHYSICS_NEST_SITE_ROWS)"),
        None => println!("  AtNest: shipped material test (8-adjacency to a nest cell, ~1 row)"),
    }
    // **Echoed because nothing did**: a log alone could not say whether the
    // founding shaft was on, so an arm and its control were indistinguishable
    // after the fact -- the stale-harness failure `CLAUDE.md` records.
    match pixel_physics::sim::creature::nest_shaft_rows() {
        Some(r) => println!(
            "  founding: DUG shaft {r} rows x {} wide + entrance chamber (PIXEL_PHYSICS_NEST_SHAFT / _WIDTH), cut lined: {}",
            pixel_physics::sim::creature::nest_shaft_width(),
            std::env::var("PIXEL_PHYSICS_BURROW_LINING").as_deref() != Ok("off")
        ),
        None => println!("  founding: painted strip only, nothing dug (PIXEL_PHYSICS_NEST_SHAFT unset)"),
    }
    match pixel_physics::sim::creature::nest_home(&world) {
        pixel_physics::sim::creature::NestHome::Material => {}
        pixel_physics::sim::creature::NestHome::Shaft => {
            println!("  home: the painted nest AND the founding cut (PIXEL_PHYSICS_NEST_HOME=shaft) -- in the shaft, the chamber or on the mouth's rim an ant is AtNest")
        }
        pixel_physics::sim::creature::NestHome::Mouth => println!(
            "  home: the painted nest AND the founding cut's mouth (PIXEL_PHYSICS_NEST_HOME=mouth) -- on the rim or in the first {} rows down an ant is AtNest; deeper it is away",
            pixel_physics::sim::creature::NEST_MOUTH_ROWS
        ),
    }
    println!();
    println!(
        "  soil wetness {wet} of {} saturated ({}); the column the dig decision actually reads is `wet grad`",
        material::SOIL_SATURATED,
        if wet >= material::SOIL_FIELD_CAPACITY { "at or above field capacity" } else { "below field capacity" }
    );
    println!();
    println!(
        "{:>8}  {:>5}  {:>7}  {:>7}  {:>6}  {:>10}  {:>10}  {:>9}  {:>8}",
        "frame", "ants", "digs", "roofed", "open", "ants in it", "room total", "hauled up", "charge"
    );

    let particles = ParticleSystem::default();
    let mut renderer = Renderer::new();
    let mut blasts = Blasts::default();
    let _ = &mut blasts;
    // **`stops=` names the frames outright**, because a run whose sheet is
    // sampled by an even division cannot be compared against another run of a
    // different length -- the panels are at different instants and the eye
    // reads the difference as the world changing.
    let stops: Vec<u64> = match arg::<String>("stops") {
        Some(v) => v.split(',').map(|f| f.trim().parse().expect("stops=a,b,c")).collect(),
        None => (0..=frames).step_by((frames / 6).max(1) as usize).collect(),
    };
    let mut shots: Vec<Vec<u8>> = Vec::new();
    let mut tinted_shots: Vec<Vec<u8>> = Vec::new();
    // **The nest scoreboard** (see [`print_score`]): `nulls=K` draws per
    // null model at every stop, 0 to switch it off. It reads the world and
    // draws from its own seeded streams, so it cannot change the run.
    let score_k: usize = arg("nulls").unwrap_or(200);
    let mut ground: Option<Ground> = None;
    if score_k > 0 {
        println!(
            "  scoreboard: {score_k} draws per null (uniform, rows, eden, walkers x{ants} at persistence {WALKER_PERSIST}); SCORE = percentile among the draws, 0.5 = like random digging, 1.0 = more nest-like than every draw"
        );
    }
    if tint_out.is_some() {
        println!("  tint: spoil ORANGE, tunnel lining (packedsoil) CYAN, nest WHITE, ants MAGENTA, loose soil above the old ground line YELLOW, stone left grey");
    }

    // **The nest funnel** (see [`NestFunnel`]): on unless `nofunnel`. It
    // reads the world between frames and writes nothing, so it cannot change
    // the run; its LEDGER lines reconcile with the engine's own counters.
    let funnel_on = !flag("nofunnel");
    let mut funnel = NestFunnel::default();
    for f in 0..=frames {
        if f > 0 {
            if funnel_on {
                funnel.before(&world, &b);
            }
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
            world.step_pheromones();
            if funnel_on {
                funnel.after(&world, &b, f);
            }
        }
        trickle.step(&mut world, &b);
        if f == 0 && score_k > 0 {
            // Before any ant has dug (digging starts with frame 1's step)
            // and after founding, so a founding cut is already ground's
            // missing piece rather than something a null may carve.
            let cut = world.nest_sites.iter().find_map(|s| s.shaft);
            ground = Some(ground_at(&world, &b, &|x, y| cut.is_some_and(|c| c.contains(x, y))));
        }
        if stops.contains(&f) {
            let (roofed, open, above, bodies, _rw, _rh, _iqr, _p50x) = census(&world, &b);
            let (n, e) = charge(&world);
            let st = world.creature_stats;
            // `per roll` and the wetness gradient moved to the SUMMARY: the
            // per-stop row is now the conservation ledger (what was dug, what
            // stands open, who is standing in it, what was hauled clear), and
            // a wide row nobody can read across is worse than two narrow ones.
            let _ = moisture_seen(&world);
            println!(
                "{f:>8}  {n:>5}  {:>7}  {roofed:>7}  {open:>6}  {bodies:>10}  {:>10}  {above:>9}  {e:>8.1}",
                st.digs,
                roofed + open + bodies
            );
            if let Some(line) = cut_census(&world) {
                println!("{line}");
            }
            if flag("trace") {
                trace(&world);
            }
            if funnel_on && f > 0 {
                funnel.print(f, &world);
            }
            if let Some(g) = &ground {
                let cut = world.nest_sites.iter().find_map(|s| s.shaft);
                let m = room_mask(&world, &b, &|x, y| cut.is_some_and(|c| c.contains(x, y)));
                print_score(f, &m, g, score_k, ants as usize, seed);
                if f == *stops.iter().max().unwrap_or(&f) {
                    if let Some(path) = arg::<String>("maskout") {
                        write_sheet(&path, &mask_tiles(&m, g, &b, ants as usize, seed, f), &b, scale);
                        println!("  maskout: tiles top to bottom are the colony, then one draw each of uniform, rows, eden, walkers, all {} cells", m.count());
                    }
                }
            }
            if out.is_some() || tint_out.is_some() {
                let (vw, vh) = (b.w as u32, b.h as u32);
                let mut buf = vec![0u8; (vw * vh * 4) as usize];
                let touched = world.take_touched_chunks();
                renderer.draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
                if tint_out.is_some() {
                    let mut tinted = buf.clone();
                    tint(&world, &b, &mut tinted);
                    tinted_shots.push(tinted);
                }
                shots.push(buf);
            }
        }
    }

    let st = world.creature_stats;
    let (roofed, open, above, bodies, rw, rh, iqr, p50x) = census(&world, &b);
    println!();
    println!(
        "SUMMARY digs={} rolls={} per_roll={:.3} roofed={roofed} open={open} ants_in_it={bodies} room_total={} hauled_up={above} spoil_dumped={} room={rw}w x{rh}h vert={:.2} iqr={iqr} p50x={p50x:+} aimed_down={}",
        st.digs,
        st.dig_rolls,
        if st.dig_rolls > 0 { st.digs as f64 / st.dig_rolls as f64 } else { 0.0 },
        roofed + open + bodies,
        st.spoil_dumped,
        if rw > 0 { rh as f64 / rw as f64 } else { 0.0 },
        st.digs_aimed_down
    );
    // **The shape columns above rank a bigger hole above a better one.**
    // This one does not: see `chambers`. Printed beside them rather than
    // instead of them, because every prior arm was scored on `room_total`
    // and those numbers have to stay comparable.
    // **Did the haulage rule fire at all?** `PIXEL_PHYSICS_SPOIL_HAUL` steers
    // the heading through `tumble`, which is the *blocked-path* re-roll and
    // not the ordinary step -- so a null on the mound could equally mean the
    // mechanism is wrong or that it almost never gets a turn. Those want
    // opposite work, and only this pair separates them: `CLAUDE.md`'s rule
    // that a "did it happen" counter must be read beside the effect it claims.
    println!(
        "SUMMARY tumbles: {} total, {} of them steered homeward ({:.1}%)",
        st.tumbles,
        st.tumbles_homeward,
        if st.tumbles > 0 { 100.0 * st.tumbles_homeward as f64 / st.tumbles as f64 } else { 0.0 }
    );
    let ch = chambers(&world, &b);
    println!(
        "SUMMARY chambers={} median={}h x{}w widest={}w passage={} contrast={:.1}x   -- owner spec 2026-09-20: passage ~4, chamber 8-16 tall and wider than tall, 2-4x contrast",
        ch.count, ch.med_h, ch.med_w, ch.max_w, ch.passage, ch.contrast
    );
    if let Some(line) = cut_census(&world) {
        println!("SUMMARY {}", line.trim());
    }
    {
        // Printed for every arm, so an arm with no cut reads the plain census
        // again and the two lines compare like for like across arms.
        let cut = world.nest_sites.iter().find_map(|s| s.shaft);
        let (r, o, _, bd, w, h, iq, _) = census_masked(&world, &b, &|x, y| cut.is_some_and(|c| c.contains(x, y)));
        println!(
            "SUMMARY dug by the colony, outside the founding cut: room_total={} room={w}w x{h}h vert={:.2} iqr={iq}",
            r + o + bd,
            if w > 0 { h as f64 / w as f64 } else { 0.0 }
        );
    }
    // **Can the one remaining candidate demonstrate itself?**
    //
    // `CLAUDE.md`: *check that a planned step can demonstrate itself, before
    // promising it will.* Four levers have come back negative and the only
    // one left is Toffin's self-amplification -- in this engine, "dig where
    // fresh spoil is next to you", a material adjacency test rather than a
    // pheromone. That rule can only concentrate digging if **having spoil
    // beside you actually discriminates between candidate cells**. If nearly
    // every diggable cell already has spoil in reach, the term is satisfied
    // everywhere and no weight on it can move anything -- the same shape of
    // failure as a sensor with no range, arriving through a material.
    //
    // So: over every cell that a dig could target (ground, below the old
    // surface), how many have at least one `spoil` cell in the 8
    // neighbourhood the digger itself uses?
    {
        let spoil_id = world.materials.id_of("spoil");
        let (mut diggable, mut with_spoil) = (0usize, 0usize);
        for x in 1..b.w - 1 {
            for y in b.surface..b.floor {
                let cell = world.get(x, y);
                let kind = world.materials.kind(cell.material);
                if cell.material == material::EMPTY || !matches!(kind, MaterialKind::Powder | MaterialKind::Solid) || cell.organism_id() != 0 {
                    continue;
                }
                diggable += 1;
                if let Some(sp) = spoil_id {
                    let near = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
                        .iter()
                        .any(|(dx, dy)| world.get(x + dx, y + dy).material == sp);
                    if near {
                        with_spoil += 1;
                    }
                }
            }
        }
        // And how much `spoil` exists anywhere at all -- above the old
        // surface as well as below it. If the adjacency above is near zero
        // because there are barely any spoil cells in the world, that is a
        // fact about the material's lifetime, not about where haulage puts
        // it, and it decides the amplification term's feasibility outright.
        let mut spoil_cells = 0usize;
        if let Some(sp) = spoil_id {
            for x in 0..b.w {
                for y in 0..b.floor {
                    if world.get(x, y).material == sp {
                        spoil_cells += 1;
                    }
                }
            }
        }
        let pct = if diggable > 0 { 100.0 * with_spoil as f64 / diggable as f64 } else { 0.0 };
        // **What the mound is made of, by material.**
        //
        // The adjacency line below says a 'dig near fresh spoil' rule has
        // nothing to read, and on its own it cannot say *why* -- whether the
        // pellets were never put down, were hauled somewhere else, or were
        // relabelled after they landed. Those want three different repairs
        // and only this census tells them apart. `CLAUDE.md`'s *ask what
        // your number counts when nothing is wrong*: 2,807 pellets against
        // 96 standing cells of `spoil` is either a marker with a short life
        // or a census looking in the wrong place, and a histogram cannot be
        // either.
        {
            let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
            for x in 1..b.w - 1 {
                for y in 0..b.surface {
                    let cell = world.get(x, y);
                    if cell.material == material::EMPTY || cell.organism_id() != 0 {
                        continue;
                    }
                    *counts.entry(world.materials.get(cell.material).name.clone()).or_default() += 1;
                }
            }
            let mut v: Vec<(String, usize)> = counts.into_iter().collect();
            v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
            let line: Vec<String> = v.iter().map(|(n, c)| format!("{n} {c}")).collect();
            println!("SUMMARY the mound, by material: {}", line.join(", "));
            // **What is holding the mound up** -- owner playtest, 2026-09-20,
            // looking at a 7-stop sheet of this very run: *"there's a hole in
            // the ground with weird floating spoil above it that isn't
            // actually where the nest entrance was landed."*
            //
            // Three numbers, because the complaint has three parts and they
            // want different repairs. A cell **standing on nothing** is a
            // support bug. A cell **standing on an ant** is `dead-ends.md`'s
            // residual hanging class arriving without a plant in the box to
            // blame -- worked ground cut in place resting on a body, which
            // walks away. And the mound's **offset from the nest** says
            // whether haulage is putting the crater where the door is; the
            // biology (section 3 of the excavation reference) has an ant walk a
            // few body lengths out and drop, so a small offset is correct
            // and a large one is not.
            // **Orphans, on the rule's own definition** -- worked ground with
            // no path down to the world floor through ground, which is what
            // `hangcensus` counts and what `update::unpack_orphans` acts on.
            //
            // The `floating` column below is NOT this and must not be read as
            // it: it counts a cell with air directly beneath, which every
            // roof of every cavity in the mound also has. An anchored
            // overhang is legitimate and reads as `floating`; only an orphan
            // is a bug. Measuring the repair on `floating` would have scored
            // it against a number it is not trying to move -- `CLAUDE.md`'s
            // *ask what your number counts when nothing is wrong*, caught
            // here by the repair failing to zero a column it never should.
            let orphans = {
                let (bw, bh) = (b.w as usize, (b.floor + 1) as usize);
                let gidx = |x: usize, y: usize| y * bw + x;
                let is_gnd = |wld: &World, x: i32, y: i32| {
                    let c = wld.get(x, y);
                    c.material != material::EMPTY
                        && c.organism_id() == 0
                        && matches!(wld.materials.kind(c.material), MaterialKind::Powder | MaterialKind::Solid)
                };
                let mut seen = vec![false; bw * bh];
                let mut st: Vec<(i32, i32)> = Vec::new();
                for x in 0..b.w {
                    if is_gnd(&world, x, b.floor) && !seen[gidx(x as usize, b.floor as usize)] {
                        seen[gidx(x as usize, b.floor as usize)] = true;
                        st.push((x, b.floor));
                    }
                }
                while let Some((x, y)) = st.pop() {
                    for (dx, dy) in [(-1i32, -1i32), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= b.w || ny > b.floor {
                            continue;
                        }
                        if !seen[gidx(nx as usize, ny as usize)] && is_gnd(&world, nx, ny) {
                            seen[gidx(nx as usize, ny as usize)] = true;
                            st.push((nx, ny));
                        }
                    }
                }
                let mut o = 0i64;
                for x in 0..b.w {
                    for y in 0..=b.floor {
                        if !seen[gidx(x as usize, y as usize)] && is_gnd(&world, x, y) {
                            o += 1;
                        }
                    }
                }
                o
            };
            println!(
                "SUMMARY orphaned ground (no path to the floor): {orphans} cells   |   un-packed this run: {}",
                st.unpacked
            );
            let (mut floating, mut on_ant, mut sx, mut n) = (0i64, 0i64, 0i64, 0i64);
            // **Which material is doing the floating**, because the repair
            // already on `main` reaches exactly one of them. `spoil.ron`
            // carries `needs_footing: true`, so a dumped pellet is a wall
            // only while something is under it -- but a mound is mostly
            // `packedsoil`, which is worked ground *cut in place* rather than
            // a pellet and carries no such flag. If the floaters are packed,
            // the landed repair cannot reach them and a second one is needed;
            // if they are spoil, the flag is not doing its job. The lab-bed
            // lane could not run this test because its floaters were plants.
            let mut float_mat: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
            for x in 1..b.w - 1 {
                for y in 0..b.surface {
                    let cell = world.get(x, y);
                    if cell.material == material::EMPTY || cell.organism_id() != 0 {
                        continue;
                    }
                    sx += x as i64;
                    n += 1;
                    let below = world.get(x, y + 1);
                    if below.material == material::EMPTY {
                        floating += 1;
                        *float_mat.entry(world.materials.get(cell.material).name.clone()).or_default() += 1;
                    } else if below.organism_id() != 0 && world.materials.kind(below.material) == MaterialKind::Creature {
                        on_ant += 1;
                    }
                }
            }
            let centroid = if n > 0 { (sx / n) as i32 } else { b.w / 2 };
            println!(
                "SUMMARY the mound stands on: nothing {floating}, an ant {on_ant}, ground {} of {n} cells   |   its centre is {:+} columns from the nest door",
                n - floating - on_ant,
                centroid - b.w / 2
            );
            {
                let mut v: Vec<(String, usize)> = float_mat.into_iter().collect();
                v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
                let line: Vec<String> = v.iter().map(|(n, c)| format!("{n} {c}")).collect();
                println!(
                    "SUMMARY ...and what is floating, by material: {}   -- `spoil` carries needs_footing on main; `packedsoil` does not",
                    if line.is_empty() { "nothing".to_string() } else { line.join(", ") }
                );
            }
        }
        println!("SUMMARY spoil standing in the world: {spoil_cells} cells, against {} pellets ever put down", st.spoil_dumped);
        // **Khuong's rule, priced where the decision is taken.** The
        // adjacency line below is the *dig* side's denominator, averaged over
        // the whole buried world; this is the *drop* side's, counted at every
        // spoil drop over the eight cells that animal could actually have
        // used. They are different questions and the survey's construction
        // headline is the second one.
        let cand = st.spoil_drop_candidates;
        if cand > 0 {
            println!(
                "SUMMARY at the drop: {} of {cand} places a pellet would stay had a pellet already in reach ({:.1}%); {} of {} drops had both kinds to choose between",
                st.spoil_drop_candidates_by_spoil,
                100.0 * st.spoil_drop_candidates_by_spoil as f64 / cand as f64,
                st.spoil_drops_discriminable,
                st.spoil_dumped
            );
            // **And how many drops never chose a neighbour at all.** A
            // pellet with no place beside the animal that would hold it goes
            // up the column instead (`creature.rs`'s `lifted` branch), and a
            // rule about which neighbour to prefer cannot reach one of
            // those. It is the other half of the same feasibility question
            // and it is much the larger half.
            println!(
                "SUMMARY ...and {} of {} drops went up the column instead, where there is no neighbour to prefer ({:.0}%)",
                st.spoil_lifted,
                st.spoil_dumped,
                100.0 * st.spoil_lifted as f64 / st.spoil_dumped.max(1) as f64
            );
        }
        if st.spoil_holds_under_cover > 0 {
            println!("SUMMARY drop rolls damped for being under cover: {}", st.spoil_holds_under_cover);
        }
        println!(
            "SUMMARY spoil adjacency: {with_spoil} of {diggable} diggable cells have spoil in reach ({pct:.1}%)               -- a 'dig near fresh spoil' rule discriminates only in the gap between that and 100%"
        );
    }
    let bands = st.at_nest_crowding;
    let total: u64 = bands.iter().sum();
    if total > 0 {
        println!(
            "SUMMARY how packed it felt while at the nest, 0.0..1.0 in tenths: {bands:?}  (top tenth {:.1}% of at-nest ticks)",
            bands[9] as f64 * 100.0 / total as f64
        );
    }

    if let Some(path) = out {
        write_sheet(&path, &shots, &b, scale);
    }
    if let Some(path) = tint_out {
        write_sheet(&path, &tinted_shots, &b, scale);
    }
}

/// One column of stops, top to bottom, magnified by `scale` and cut to
/// `crop=`.
///
/// **`crop=x,y,w,h` in world cells, because a 400-wide box drawn whole is
/// 99% undisturbed dirt.** The nest is 50-odd columns of a 400-column world
/// and the question is always what happened *at* it, so a sheet of the whole
/// box puts the answer in a twentieth of its own picture -- which the owner
/// reads on a phone. Same spelling as `examples/filmstrip`'s.
fn write_sheet(path: &str, shots: &[Vec<u8>], b: &Box2, scale: u32) {
    let (cx0, cy0, cw, ch) = match arg::<String>("crop") {
        Some(v) => {
            let n: Vec<i32> = v.split(',').map(|p| p.trim().parse().expect("crop=x,y,w,h")).collect();
            assert_eq!(n.len(), 4, "crop= wants x,y,w,h in world cells, got `{v}`");
            (n[0].clamp(0, b.w - 1), n[1].clamp(0, b.h - 1), n[2], n[3])
        }
        None => (0, 0, b.w, b.h),
    };
    let (cw, ch) = (cw.min(b.w - cx0).max(1), ch.min(b.h - cy0).max(1));
    let (tw, th) = (cw as u32, ch as u32);
    let (sw, sh) = (tw * scale, th * shots.len() as u32 * scale);
    let mut sheet = vec![0u8; (sw * sh * 4) as usize];
    for (i, tile) in shots.iter().enumerate() {
        let y0 = i as u32 * th * scale;
        for y in 0..th {
            for ry in 0..scale {
                let dst_row = ((y0 + y * scale + ry) * sw * 4) as usize;
                for x in 0..tw {
                    let src = (((y + cy0 as u32) * b.w as u32 + x + cx0 as u32) * 4) as usize;
                    let px = &tile[src..src + 4];
                    for rx in 0..scale {
                        let dst = dst_row + ((x * scale + rx) * 4) as usize;
                        sheet[dst..dst + 4].copy_from_slice(px);
                    }
                }
            }
        }
    }
    image::save_buffer(path, &sheet, sw, sh, image::ColorType::Rgba8).expect("writing the sheet");
    println!("wrote {path} ({sw}x{sh}, {} stops top to bottom)", shots.len());
}

/// **Every class of ground painted flat**, so a sheet says *what* each pixel
/// is rather than leaving it to a brown against a slightly greyer brown.
///
/// Owner, on a sheet of this very box (card `20260920T052914002Z-a24f02`):
/// *"what are all the gray pixels in the image?"* The worked soils carry
/// their own palette -- `packedsoil.ron` and `spoil.ron` share one, a
/// desaturated brown next to `soil`'s warmer one -- so at play zoom the
/// lining of every tunnel and every dumped pellet reads as grey grit. A
/// picture cannot say which is which; this can.
///
/// **A full replace on fixed colours, never a blend** (`CLAUDE.md`'s *a
/// debug readout must not be a function of the thing it debugs*): a
/// magnitude blend into the cell's own colour once made a working overlay
/// read as blank. The precedents are `soilfork`'s and `hangcensus`'s orange.
///
/// - `spoil` (a dumped pellet): **orange**
/// - `packedsoil` (tunnel lining, tamped ground): **cyan**
/// - `nest` (the painted door or strip): **white**
/// - an animal: **magenta**
/// - loose `soil` standing above the old ground line (a heap that was
///   spoil and slumped, or ground that fell): **yellow**
///
/// Stone, sky, water and dug void are left as the renderer drew them.
fn tint(world: &World, b: &Box2, buf: &mut [u8]) {
    let id = |n: &str| world.materials.id_of(n);
    let (spoil, packed, nest, soil) = (id("spoil"), id("packedsoil"), id("nest"), id("soil"));
    for y in 0..b.h {
        for x in 0..b.w {
            let cell = world.get(x, y);
            let rgb: Option<[u8; 3]> = if cell.material == material::EMPTY {
                None
            } else if cell.organism_id() != 0 && world.materials.kind(cell.material) == MaterialKind::Creature {
                Some([255, 0, 200])
            } else if Some(cell.material) == spoil {
                Some([255, 140, 0])
            } else if Some(cell.material) == packed {
                Some([0, 210, 255])
            } else if Some(cell.material) == nest {
                Some([255, 255, 255])
            } else if Some(cell.material) == soil && y < b.surface {
                Some([255, 230, 0])
            } else {
                None
            };
            if let Some(c) = rgb {
                let i = ((y * b.w + x) * 4) as usize;
                buf[i..i + 3].copy_from_slice(&c);
                buf[i + 3] = 255;
            }
        }
    }
}

/// **What is standing in the founding cut now**, one line, or `None` when no
/// site carries a cut (`PIXEL_PHYSICS_NEST_SHAFT` unset).
///
/// **The census above cannot answer "is the shaft still there", and it is the
/// trap this line exists for.** Its `open` column counts empty cells in the
/// original soil block that the sky can see, so when an unlined cut collapses
/// the void does not vanish -- the roof falls into the chamber and the soil
/// column over it drops, and the same volume reappears as a dip in the
/// surface. Measured 2026-09-26, `NEST_SHAFT=20`, no ants: `roofed + open`
/// read **82 at every stop** while the shaft and chamber were gone by frame
/// 5. This counts the cut's own cells, from the footprint the cut recorded on
/// its site (`NestSite::shaft`), so a collapse reads as a collapse.
fn cut_census(world: &World) -> Option<String> {
    let fp = world.nest_sites.iter().find_map(|s| s.shaft)?;
    let id = |n: &str| world.materials.id_of(n);
    let (spoil, packed, soil) = (id("spoil"), id("packedsoil"), id("soil"));
    let cells = fp.cells();
    let (mut open, mut ants, mut soils, mut lining, mut pellets, mut other) = (0, 0, 0, 0, 0, 0);
    for &(x, y) in &cells {
        let c = world.get(x, y);
        if c.material == material::EMPTY {
            open += 1;
        } else if c.organism_id() != 0 && world.materials.kind(c.material) == MaterialKind::Creature {
            ants += 1;
        } else if Some(c.material) == soil {
            soils += 1;
        } else if Some(c.material) == packed {
            lining += 1;
        } else if Some(c.material) == spoil {
            pellets += 1;
        } else {
            other += 1;
        }
    }
    Some(format!(
        "          cut: {open} of {} cells still open | filled by ants {ants}, loose soil {soils}, lining {lining}, spoil {pellets}, other {other}",
        cells.len()
    ))
}

/// Two arms, because the two ways this harness can lie are opposite ones.
fn selftest_run(b: &Box2) {
    // The scoreboard first: mask-only, and seconds.
    scoreboard_selftest();
    // The nest funnel: a hand-driven ant, no stepping, milliseconds.
    funnel_selftest(b);
    // Specificity: nobody digs, so the census must find nothing. A box that
    // reads void with no ants in it is measuring its own scene.
    let mut bare = build(b);
    for _ in 0..600 {
        parallel::step(&mut bare);
        bare.step_active_sites();
        bare.step_fields();
    }
    let (roofed, open, ..) = census(&bare, b);
    println!("digbox selftest: empty box after 600 frames reads roofed {roofed} open {open} (both must be 0)");
    assert_eq!((roofed, open), (0, 0), "a box with no ants must hold no dug void -- the soil fill is slumping, or the census is measuring the scene");

    // A hand-carved chamber must be found, or the census cannot see a nest
    // when there is one.
    let mut carved = build(b);
    let (cx, cy) = (b.w / 2, b.surface + 10);
    for x in cx - 5..cx + 5 {
        for y in cy..cy + 3 {
            carved.set(x, y, Cell::EMPTY);
        }
    }
    let (roofed2, open2, ..) = census(&carved, b);
    println!("  a hand-carved 10x3 chamber 10 rows down reads roofed {roofed2} open {open2} (must be 30 and 0)");
    assert_eq!((roofed2, open2), (30, 0), "the census must find a known chamber, and must call it roofed rather than open");

    // ...and a shaft cut to the surface must read OPEN, not roofed -- the
    // distinction the whole question rests on.
    let mut shaft = build(b);
    for y in b.surface..b.surface + 6 {
        shaft.set(cx, y, Cell::EMPTY);
    }
    let (roofed3, open3, ..) = census(&shaft, b);
    println!("  a 6-deep shaft open to the sky reads roofed {roofed3} open {open3} (must be 0 and 6)");
    assert_eq!((roofed3, open3), (0, 6), "a hole open to the sky is not a room");

    // **And `iqr` must tell a concentrated room from a scattered one of the
    // identical volume, which no other column here can.** This is the same
    // guard `burrow_probe`'s `arms=selftest` draws with a bar and the bar
    // rotated, pointed at the other blind spot: `roofed`, `open`, `bodies`
    // and `room total` are counts, so thirty cells in one chamber and thirty
    // cells in ten scattered pits read identical -- and the *bounding box*
    // reads the scattered one as the WIDER nest, which is the reading the
    // three-negatives report's shape table rests on. Both halves are
    // asserted, so the test fails if `iqr` ever stops discriminating and
    // also if the counts ever start to.
    let mut lump = build(b);
    let mut spread = build(b);
    for i in 0..10 {
        for y in cy..cy + 3 {
            // One block of ten columns against ten single columns spread
            // evenly across the whole box -- same thirty cells, same depth,
            // same roof. The spacing is derived from `b.w` rather than
            // authored, because the selftest runs at the default width and
            // a hardcoded stride walked the pits off the edge of the world,
            // where they are not dug at all and the control silently
            // compares thirty cells against eighteen.
            lump.set(cx - 5 + i, y, Cell::EMPTY);
            spread.set(2 + i * ((b.w - 6) / 10), y, Cell::EMPTY);
        }
    }
    let (lr, lo_, _, _, lw, _, liqr, _) = census(&lump, b);
    let (sr, so_, _, _, sw, _, siqr, _) = census(&spread, b);
    println!(
        "  thirty cells in one chamber against thirty in ten scattered pits: room total {} vs {}, bbox {lw}w vs {sw}w, iqr {liqr} vs {siqr}"
    , lr + lo_, sr + so_);
    assert_eq!(lr + lo_, sr + so_, "the two beds must hold the same volume, or this control is not one");
    assert!(lw < sw, "the scattered bed must have the wider bounding box, which is the point: a max statistic calls scattering a bigger nest");
    assert!(
        siqr > liqr * 4,
        "iqr {siqr} against {liqr}: the middle-half width must separate a concentrated room from a scattered one, or it is a third blind column"
    );

    // **And the chamber census must tell the owner's three cases apart**,
    // which is the whole reason it exists: `roofed`/`room total` rank the
    // giant hole *highest* of the three, and `vert`/`iqr` describe a
    // bounding box that the target and the smear share.
    //
    // Owner, 2026-09-20: *"Chambers that are just two cells tall are not
    // going to look like a chamber... at some point the answer is just
    // digging one giant hole which is actually what they do right now and
    // then it's not chambers and tunnels."* Three hand-carved beds, one per
    // case, and the assertions are what make this a control rather than a
    // printout -- put any of the three shapes in and the wrong one out, and
    // this goes red.
    let carve = |wld: &mut World, x: i32, y: i32, cw: i32, chh: i32| {
        for yy in y..y + chh {
            for xx in x..x + cw {
                wld.set(xx, yy, Cell::EMPTY);
            }
        }
    };
    // (a) TARGET: two chambers joined by a 4-cell tunnel, under a roof.
    let mut target = build(b);
    carve(&mut target, cx - 50, cy, 32, 12); // chamber 12 tall x 32 wide
    carve(&mut target, cx - 18, cy + 4, 20, 4); // passage, 4 cells across
    carve(&mut target, cx + 2, cy + 2, 24, 10); // chamber 10 tall x 24 wide
    let ct = chambers(&target, b);
    // (b) TOO BIG: one hole with no narrow waist anywhere -- today's nest.
    let mut hole = build(b);
    carve(&mut hole, 10, cy, 120, 14);
    let chl = chambers(&hole, b);
    // (c) TOO SMALL: galleries two cells tall, which read as scratches.
    let mut scratch = build(b);
    for i in 0..4 {
        carve(&mut scratch, 20 + i * 40, cy + i * 3, 34, 2);
    }
    let cs = chambers(&scratch, b);
    println!(
        "  chamber census on three carved beds -- target: {} chambers, {}h x{}w, passage {}, contrast {:.1}x",
        ct.count, ct.med_h, ct.med_w, ct.passage, ct.contrast
    );
    println!(
        "                                       one hole: {} chamber(s), widest {}w   |   scratches: {} chambers",
        chl.count, chl.max_w, cs.count
    );
    assert_eq!(ct.count, 2, "two chambers joined by a 4-cell tunnel must read as two, or the core threshold is leaking through the passage");
    assert!(
        ct.contrast >= 2.0,
        "target contrast {:.1}x: a chamber 2-4x the passage must read at or above 2x, or the column cannot score the owner's spec",
        ct.contrast
    );
    assert_eq!(chl.count, 1, "a hole with no narrow waist is one chamber, however big");
    assert!(
        chl.max_w > ct.max_w * 2,
        "the giant hole must read far wider ({}w) than the target's chambers ({}w) -- that width IS the tell, and without it this census cannot name today's failure",
        chl.max_w,
        ct.max_w
    );
    assert_eq!(cs.count, 0, "two-cell galleries are not chambers -- if these count, the threshold is below what this game can read");

    // Sensitivity: ants in the box must actually dig. A colony that never
    // places, or never wakes, reports a clean null that reads as a finding.
    let mut live = build(b);
    let founded = live.found_colony_of(b.w / 2, b.surface - 1, "ant", 30);
    assert!(founded >= 20, "only {founded} of 30 ants placed -- this box is not the colony it reports on");
    for _ in 0..4_000 {
        parallel::step(&mut live);
        live.step_active_sites();
        live.step_fields();
        live.step_pheromones();
    }
    let st = live.creature_stats;
    let (roofed4, ..) = census(&live, b);
    println!("  {founded} ants for 4,000 frames: {} digs, {} rolls, roofed {roofed4}", st.digs, st.dig_rolls);
    assert!(st.dig_rolls > 0, "not one dig was even attempted -- the colony is not thinking, so any null from this box is the harness");
    assert!(st.digs > 0, "digs attempted but none landed -- every roll hit air, rock or another ant, and this box cannot answer a digging question");
    println!("digbox selftest: PASS -- the box is empty when nobody digs, finds a known chamber, calls a shaft open, and its ants dig");
}
