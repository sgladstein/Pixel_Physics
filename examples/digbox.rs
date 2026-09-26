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
use pixel_physics::sim::material::MaterialKind;
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
    // Chebyshev distance to the nearest non-room cell, two passes.
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

    // Every input that reaches `Dig` in `ant.ron`, plus the decision itself.
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
    if tint_out.is_some() {
        println!("  tint: spoil ORANGE, tunnel lining (packedsoil) CYAN, nest WHITE, ants MAGENTA, loose soil above the old ground line YELLOW, stone left grey");
    }

    for f in 0..=frames {
        if f > 0 {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
            world.step_pheromones();
        }
        trickle.step(&mut world, &b);
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
