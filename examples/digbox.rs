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
//! dig_urge = squash(-0.3*Bias - 1.0*SurfaceCurvature + 0.8*FoodAdjacent - 0.55*MoistureGrad + 2.5*u5 - 2.5*u6)
//!            (the first two terms since 2026-09-28; before, `0.15*Bias` alone)
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
//! **Fed, since 2026-09-29** ([`feed`]): every ant is topped up to its start
//! energy each frame, so nothing starves and no ant is hungry. Before that the
//! box starved its colony from the first tick, and a hungry colony scouts and
//! digs as a restless one (report `nest-one-entrance-2026-09-29.md` §14);
//! `hungry` restores it. The paragraph below is that box's arithmetic.
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

/// How long a refilled dug cell must stay ground to count as filled rather
/// than passed through: a falling grain crosses a cell in one frame.
const REFILL_STANDING: u64 = 100;
/// Spoil this close to a dig target counts as beside it (Chebyshev cells).
const SPOIL_NEAR: i32 = 2;
/// ...and this recently put down counts as fresh. The biology's fresh heap
/// stops drawing digging within about an hour; an ant here decides every
/// six frames, so a thousand frames is some 170 decisions.
const SPOIL_FRESH: u64 = 1_000;
/// The causes `NestFunnel::surf_opened` books an opening of the old surface
/// row under, in its order.
const SURF_CAUSES: usize = 5;
/// How far up the time budget looks for cover: `creature::under_cover`'s own
/// reach.
const COVER_ROWS: i32 = 20;
/// ...and how near the door, in columns, counts as at it.
const DOOR_NEAR: i32 = 8;
const SURF_CAUSE_NAMES: [&str; SURF_CAUSES] =
    ["cut from above, open sky", "cut from above, under a heap", "cut from level or below, open sky", "cut from level or below, under a heap", "no cut: the ground fell away"];

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
    /// A nest worker (`OrganismState::nest_bound_until` still ahead): one ant
    /// in four under the shipped storeroom, at home in the founding cut.
    nest_bound: bool,
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
    /// The frame it last put a pellet down in the open, until it is next
    /// under cover: the return trip `NestFunnel::back_under` times.
    open_drop: Option<u64>,
}

#[derive(Default)]
struct NestFunnel {
    /// `gridout=`: trace every cell's packing -- the frame it last became
    /// `packedsoil` (`u64::MAX`: not since frame 0) and what it was just
    /// before, as [`PACKED_FROM`] indexes. Off unless asked: a whole-box
    /// scan a frame.
    pack_trace: bool,
    packed_frame: Vec<u64>,
    packed_from: Vec<u8>,
    ants: std::collections::BTreeMap<u32, AntTrack>,
    before: std::collections::BTreeMap<u32, AntBefore>,
    /// The box as it stood before the frame: `None` is an organism's cell.
    grid: Vec<Option<MaterialId>>,
    /// ...and whose, where `grid` is `None` (0 elsewhere), so a pellet set
    /// down on its own carrier can be told from one set on a nestmate.
    grid_org: Vec<u32>,
    /// Frame each cell was last cut (`u64::MAX`: never). A digger steps into
    /// the cell it has just cut, so "an animal under it" a frame later is
    /// most often the cut that took the footing; this is what separates them.
    cut_frame: Vec<u64>,
    /// Frame each cell last had a pellet put down in it (`u64::MAX`: never),
    /// and by whom.
    put_frame: Vec<u64>,
    put_by: Vec<u32>,
    /// Cells dug, or filled by a pellet, since frame 0: re-cutting them is
    /// churn, not new ground.
    touched: Vec<bool>,
    /// Cells dug since frame 0: a pellet put down in one is the hole refilled.
    dug: Vec<bool>,
    /// The same cells as a list, so the refill scan visits only them.
    dug_list: Vec<usize>,
    /// This frame's put-down sites, which the refill scan leaves to the
    /// pellet ledger.
    put_sites: Vec<usize>,
    /// **Every frame a dug cell went from room to ground**, with no pellet put
    /// there: by what arrived (spoil, soil, anything else) and whether the
    /// cell straight above emptied in the same frame. **These count passes,
    /// not holes filled**: a grain falling down a dug shaft goes one cell a
    /// frame and books one pass per cell, "from above" each time, and a
    /// diagonal slide books "from the side" -- so the split is the grains'
    /// path, not two ways of failing (`refill_standing` is the filled count).
    /// **And the material cannot name the source**: worked ground never falls
    /// as itself (`update_powder`'s `self_supporting` branch turns lining or a
    /// pellet into `soil` where it stands, and the soil falls later), so what
    /// arrives is `soil` whatever it was (`turned_loose` counts the
    /// conversions). Both corrections are 2026-09-27's, found by reading
    /// `update_powder` after the first reading of these numbers ("the bank,
    /// not the heaps") had been written up.
    refill_fall: [u64; 3],
    refill_from_above: u64,
    refill_from_side: u64,
    /// **Worked ground turned loose where it stands**, the supply the passes
    /// above are drawn from: [lining, pellet][below the old surface, above].
    turned_loose: [[u64; 2]; 2],
    /// **Why a pellet above the old surface turned loose**, read off the cell
    /// straight beneath it, which is the one `update_powder`'s footing test
    /// reads: [its carrier's own body, another animal, air, cut out from
    /// under it in the last two frames, the ground under it fell away,
    /// anything else]. 2026-09-27: the heaps are 78% of what turns loose,
    /// and each cause wants a different lever.
    loose_why: [u64; 6],
    /// ...and how long it had stood since it was put down, in frames:
    /// [<= 1, <= 10, <= 100, <= 1,000, longer, not put by a tracked ant].
    loose_age: [u64; 6],
    /// **Pellets put down with no footing**: what the cell straight beneath
    /// held at the start of the frame -- [the carrier's own body, another
    /// animal, air]. `act`'s drop site asks for two *filled* cells of the
    /// three below, and an animal is filled; the footing rule asks for
    /// *ground* straight below, and an animal is not.
    put_unfooted: [u64; 3],
    /// Of `put_unfooted`, the ones posted up the column rather than set
    /// beside the head.
    put_unfooted_lifted: [u64; 3],
    /// Ant-frames with a pellet in the jaws, of all ant-frames: an ant
    /// holding a pellet cannot dig, so a drop that finds nowhere to go is
    /// paid for here.
    held_frames: u64,
    ant_frames: u64,
    /// **Is there fresh spoil beside where an ant digs?** At every decision
    /// with the jaws free and diggable ground straight ahead, and at every
    /// placed cut: spoil within `SPOIL_NEAR` cells of the target, any age,
    /// and put down within `SPOIL_FRESH` frames. The biology's one positive
    /// stigmergic dig cue is a pile of fresh pellets
    /// (`nest-biology-digging-signals-2026-09-19.md` §3.2, §10 rank 4), and a
    /// rule reading it can only steer where the answer differs between the
    /// places an ant could dig -- counted at the decision, not over the box,
    /// which is the Khuong entry's lesson in `dead-ends.md`.
    /// [decisions, cuts][faced, spoil near, fresh spoil near].
    spoil_near: [[u64; 3]; 2],
    /// **Where a cut opens, and whether spoil lies beside it.** The panel's
    /// mouths are runs of dug cells in the old surface row, so a cut in that
    /// row with neither neighbour ever dug opens a new mouth: from the
    /// surface, or from below when the digger's head is under the row (a
    /// tunnel breaking out). A heap cue can close mouths only if spoil
    /// separates those cuts from the digging it should keep, at a mouth
    /// already open and below the old surface.
    /// [new mouth from the surface, new mouth from below, a mouth already
    /// open, below the old surface, in the heaps] x [cuts, spoil near, fresh
    /// spoil near, spoil cells, fresh spoil cells], within `SPOIL_NEAR` of
    /// the cut.
    cut_kind_spoil: [[u64; 5]; 5],
    /// Of each `cut_kind_spoil` row's cuts, those a nest worker made: who
    /// opens the mouths, the caste that lives in the cut or the foragers.
    cut_kind_worker: [u64; 5],
    /// **What opened each cell of the old surface row**, the row the panel's
    /// mouths are runs of: every frame a cell of it goes from ground to room
    /// (empty, or an animal standing in it), by cause -- [a cut by an ant
    /// whose head was above the row, into a cell open to the sky; the same
    /// under cover (ground above the cell, a heap); a cut from level or
    /// below, open to the sky; the same under cover; no cut there this frame,
    /// the ground fell away] -- and whether it started a run of room along
    /// the row or widened one (a neighbour in the row was room before the
    /// frame). The heap cue ([`creature::spoil_cue_factor`]) judges only a
    /// cut into a cell open to the sky; a cut under cover by an enclosed
    /// digger, or into the floor below a roofed one, it leaves alone, and a
    /// fall it never sees. So the split says which openings a cue can close.
    surf_opened: [[u64; 2]; SURF_CAUSES],
    /// ...by distance from the door's centre column: [3-4, beside the paint;
    /// 5-8; 9-16; 17 or more].
    surf_opened_dist: [[u64; 4]; SURF_CAUSES],
    /// ...and the pellets within `SPOIL_NEAR` of each opening cut, summed, so
    /// the cue's saturation can be read per cause.
    surf_opened_pellets: [u64; SURF_CAUSES],
    /// The latest opening's cause per column (`u8::MAX`: not opened since
    /// frame 0), for the stops' census of the room standing in the row.
    surf_last: Vec<u8>,
    /// **The crust's own break**: the cause of each column's *first* opening
    /// (`u8::MAX`: never), since every later one is loose ground that fell
    /// into the hole passing through it again -- a grain falling down a dug
    /// shaft books one opening per cell it leaves.
    surf_first: Vec<u8>,
    /// First openings by cause, and for the falls, by what fell: [soil,
    /// lining, spoil, anything else].
    surf_first_n: [u64; SURF_CAUSES],
    surf_first_fell: [u64; 4],
    /// This frame's placed cuts: (x, y, the digger's head row).
    cuts_this_frame: Vec<(i32, i32, i32)>,
    /// **Where the colony's time goes**: ant-frames by where the head was
    /// before the frame -- [under cover (ground within `COVER_ROWS` above it
    /// in its column), in the open within `DOOR_NEAR` columns of the door, in
    /// the open further out] -- and [empty-jawed, holding a pellet]; and the
    /// cuts made from each place. A dig rate that falls is either fewer
    /// ant-frames where digging happens or fewer cuts per such frame, and the
    /// two want different levers.
    time_at: [[u64; 2]; 3],
    cuts_at: [u64; 3],
    /// **The trip back**: for every pellet put down in the open, frames until
    /// its carrier was next under cover. Carriers still out when the run ends
    /// (or that died out there) are the ants whose `open_drop` is still set.
    back_under: Vec<u64>,
    /// **A refill that stays**: a dug cell still ground `REFILL_STANDING`
    /// frames after it filled. [by a fall, by a pellet put there].
    refill_standing: [u64; 2],
    refill_pending: Vec<bool>,
    refill_queue: Vec<(u64, usize, usize)>,
    /// (frame due, cut x, cut y, ant) waiting on the lasting check.
    pending: Vec<(u64, i32, i32, u32)>,
    // The dig ledger: every cut in exactly one bucket.
    cut_above: u64,
    cut_again: u64,
    /// Of `cut_again`, by what the cell was made of when it was cut again:
    /// spoil (a pellet, or a heap that slid back in), soil (the bank fell
    /// in), lining, anything else. Says what the churn is made of.
    again_spoil: u64,
    again_soil: u64,
    again_lining: u64,
    again_other: u64,
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
    /// Of `put_lifted`, the pellets found neither beside the head nor up its
    /// column: carried out through the passages (`SPOIL_LIFT=out`).
    put_out: u64,
    /// **Where a pellet posted up the column left from**: the carrier's head
    /// before the frame -- [in the founding cut, under cover 1-4 rows below the
    /// old surface, 5-8, 9-16, 17 or more, not under cover]. Under
    /// `PIXEL_PHYSICS_SPOIL_OUT` a lift is a carrier that gave up the walk out,
    /// and where it gave up says whether the way out jammed or lost it.
    lift_from: [u64; 6],
    put_above: u64,
    put_below: u64,
    /// Of `put_below`, landed in a cell dug since frame 0: the hole refilled.
    put_refill: u64,
    /// The carrier died holding it; the engine drops it by its corpse.
    died_holding: u64,
    site_not_found: u64,
}

impl NestFunnel {
    /// Spoil within `SPOIL_NEAR` of `(tx, ty)` in the grid as it stood before
    /// the frame: (any age, put down within `SPOIL_FRESH` frames).
    fn spoil_near_of(grid: &[Option<MaterialId>], put_frame: &[u64], b: &Box2, spoil: Option<MaterialId>, frame: u64, tx: i32, ty: i32) -> (bool, bool) {
        let Some(spoil) = spoil else { return (false, false) };
        let (mut near, mut fresh) = (false, false);
        for y in ty - SPOIL_NEAR..=ty + SPOIL_NEAR {
            for x in tx - SPOIL_NEAR..=tx + SPOIL_NEAR {
                if x < 0 || y < 0 || x >= b.w || y >= b.h {
                    continue;
                }
                let i = (y * b.w + x) as usize;
                if grid[i] == Some(spoil) {
                    near = true;
                    if put_frame[i] != u64::MAX && frame.saturating_sub(put_frame[i]) <= SPOIL_FRESH {
                        fresh = true;
                        return (near, fresh);
                    }
                }
            }
        }
        (near, fresh)
    }

    /// How many spoil cells lie within `SPOIL_NEAR` of `(tx, ty)` in the grid
    /// as it stood before the frame: (any age, put down within `SPOIL_FRESH`
    /// frames). The count, not the flag: a cue that makes one hole outdraw
    /// another needs to know how much spoil, not only whether any.
    fn spoil_cells_of(grid: &[Option<MaterialId>], put_frame: &[u64], b: &Box2, spoil: Option<MaterialId>, frame: u64, tx: i32, ty: i32) -> (u64, u64) {
        let Some(spoil) = spoil else { return (0, 0) };
        let (mut any, mut fresh) = (0u64, 0u64);
        for y in ty - SPOIL_NEAR..=ty + SPOIL_NEAR {
            for x in tx - SPOIL_NEAR..=tx + SPOIL_NEAR {
                if x < 0 || y < 0 || x >= b.w || y >= b.h {
                    continue;
                }
                let i = (y * b.w + x) as usize;
                if grid[i] == Some(spoil) {
                    any += 1;
                    if put_frame[i] != u64::MAX && frame.saturating_sub(put_frame[i]) <= SPOIL_FRESH {
                        fresh += 1;
                    }
                }
            }
        }
        (any, fresh)
    }

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
                    nest_bound: st.nest_bound_until > world.frame,
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
            self.refill_pending = vec![false; n];
            self.cut_frame = vec![u64::MAX; n];
            self.put_frame = vec![u64::MAX; n];
            self.put_by = vec![0; n];
            self.packed_frame = vec![u64::MAX; n];
            self.packed_from = vec![0; n];
        }
        self.grid.clear();
        self.grid_org.clear();
        for y in 0..b.h {
            for x in 0..b.w {
                let c = world.get(x, y);
                self.grid.push(if c.organism_id() != 0 { None } else { Some(c.material) });
                self.grid_org.push(c.organism_id());
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
        let spoil_id = world.materials.id_of("spoil");
        let ids: Vec<u32> = self.before.keys().copied().collect();
        for id in ids {
            let pre = self.before[&id];
            self.ant_frames += 1;
            if pre.holding {
                self.held_frames += 1;
            }
            // Where the head was before the frame: under cover, or in the
            // open near the door or out beyond it.
            let place = {
                let (hx, hy) = pre.head;
                let covered = (1..=COVER_ROWS).any(|dy| inside(hx, hy - dy) && matches!(self.grid[at(hx, hy - dy)], Some(m) if Self::is_ground(world, m)));
                if covered {
                    0
                } else if (hx - b.w / 2).abs() <= DOOR_NEAR {
                    1
                } else {
                    2
                }
            };
            self.time_at[place][usize::from(pre.holding)] += 1;
            let track = self.ants.get_mut(&id).expect("registered in before");
            if place == 0 {
                if let Some(t0) = track.open_drop.take() {
                    self.back_under.push(frame - t0);
                }
            }
            let (dx, dy) = DIRS[pre.heading as usize % 8];
            let (tx, ty) = (pre.head.0 + dx, pre.head.1 + dy);
            // N1: at the moment act ran, jaws free and diggable ground ahead.
            if pre.crop_empty && !pre.holding && inside(tx, ty) {
                if let Some(m) = self.grid[at(tx, ty)] {
                    if Self::is_ground(world, m) && world.materials.get(m).penetration_resistance <= 1.0 {
                        track.stage = track.stage.max(1);
                        let (near, fresh) = Self::spoil_near_of(&self.grid, &self.put_frame, b, spoil_id, frame, tx, ty);
                        self.spoil_near[0][0] += 1;
                        self.spoil_near[0][1] += u64::from(near);
                        self.spoil_near[0][2] += u64::from(fresh);
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
                    self.cuts_this_frame.push((tx, ty, pre.head.1));
                    self.cuts_at[place] += 1;
                    let (near, fresh) = Self::spoil_near_of(&self.grid, &self.put_frame, b, spoil_id, frame, tx, ty);
                    self.spoil_near[1][0] += 1;
                    self.spoil_near[1][1] += u64::from(near);
                    self.spoil_near[1][2] += u64::from(fresh);
                    // Where it opened: read before this cut is booked as dug.
                    let kind = match ty - b.surface {
                        row if row < 0 => 4,
                        row if row > 0 => 3,
                        _ => {
                            let dug_at = |x: i32| inside(x, ty) && self.dug[at(x, ty)];
                            if dug_at(tx) || dug_at(tx - 1) || dug_at(tx + 1) {
                                2
                            } else if pre.head.1 > b.surface {
                                1
                            } else {
                                0
                            }
                        }
                    };
                    let (cells, fresh_cells) = Self::spoil_cells_of(&self.grid, &self.put_frame, b, spoil_id, frame, tx, ty);
                    self.cut_kind_worker[kind] += u64::from(pre.nest_bound);
                    let k = &mut self.cut_kind_spoil[kind];
                    k[0] += 1;
                    k[1] += u64::from(cells > 0);
                    k[2] += u64::from(fresh_cells > 0);
                    k[3] += cells;
                    k[4] += fresh_cells;
                    let was = self.grid[at(tx, ty)];
                    debug_assert!(was.is_some(), "opened() admits only a material cell");
                    let row = ty - b.surface;
                    let reached = if row < 0 {
                        self.cut_above += 1;
                        2
                    } else if self.touched[at(tx, ty)] {
                        self.cut_again += 1;
                        match was.map(|m| world.materials.get(m).name.as_str()) {
                            Some("spoil") => self.again_spoil += 1,
                            Some("soil") => self.again_soil += 1,
                            Some("packedsoil") => self.again_lining += 1,
                            _ => self.again_other += 1,
                        }
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
                    self.cut_frame[at(tx, ty)] = frame;
                    if !self.dug[at(tx, ty)] {
                        self.dug_list.push(at(tx, ty));
                    }
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
                // ...and failing both, carried out through the passages
                // (`PIXEL_PHYSICS_SPOIL_LIFT=out`): the nearest cell in the
                // box where one appeared that no earlier ant this frame has
                // claimed.
                let carried_out = || {
                    let mut best: Option<(i32, (i32, i32))> = None;
                    for y in 0..b.h {
                        for x in 0..b.w {
                            if appeared(x, y) && !self.put_sites.contains(&at(x, y)) {
                                let d = (x - hx).abs().max((y - hy).abs());
                                if best.is_none_or(|(bd, _)| d < bd) {
                                    best = Some((d, (x, y)));
                                }
                            }
                        }
                    }
                    best.map(|(_, s)| s)
                };
                let site = beside
                    .map(|s| (s, false))
                    .or_else(|| (1..=160).map(|dy| (hx, hy - dy)).find(|&(x, y)| appeared(x, y)).map(|s| (s, true)))
                    .or_else(|| {
                        let s = carried_out()?;
                        self.put_out += 1;
                        Some((s, true))
                    });
                match site {
                    None => self.site_not_found += 1,
                    Some(((sx, sy), lifted)) => {
                        if lifted {
                            self.put_lifted += 1;
                            let (hx, hy) = pre.head;
                            let in_cut = world.nest_sites.iter().any(|s| s.shaft.is_some_and(|c| c.contains(hx, hy)));
                            let depth = hy - b.surface;
                            self.lift_from[if in_cut {
                                0
                            } else if place != 0 {
                                5
                            } else {
                                match depth {
                                    ..=4 => 1,
                                    5..=8 => 2,
                                    9..=16 => 3,
                                    _ => 4,
                                }
                            }] += 1;
                        } else {
                            self.put_beside += 1;
                        }
                        if place != 0 {
                            track.open_drop = Some(frame);
                        }
                        let out = sy < b.surface;
                        if out {
                            self.put_above += 1;
                        } else {
                            self.put_below += 1;
                            if self.dug[at(sx, sy)] {
                                self.put_refill += 1;
                                if !self.refill_pending[at(sx, sy)] {
                                    self.refill_pending[at(sx, sy)] = true;
                                    self.refill_queue.push((frame + REFILL_STANDING, at(sx, sy), 1));
                                }
                            }
                        }
                        self.touched[at(sx, sy)] = true;
                        self.put_sites.push(at(sx, sy));
                        self.put_frame[at(sx, sy)] = frame;
                        self.put_by[at(sx, sy)] = id;
                        if inside(sx, sy + 1) {
                            let j = at(sx, sy + 1);
                            let k = match self.grid[j] {
                                None if self.grid_org[j] == id => Some(0),
                                None => Some(1),
                                Some(m) if m == material::EMPTY => Some(2),
                                Some(_) => None,
                            };
                            if let Some(k) = k {
                                self.put_unfooted[k] += 1;
                                if lifted {
                                    self.put_unfooted_lifted[k] += 1;
                                }
                            }
                        }
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
        // The opening ledger: every cell of the old surface row that was
        // ground before the frame and is room after it, by what opened it.
        let room = |c: Option<MaterialId>| matches!(c, None | Some(material::EMPTY));
        if self.surf_last.len() != b.w as usize {
            self.surf_last = vec![u8::MAX; b.w as usize];
            self.surf_first = vec![u8::MAX; b.w as usize];
        }
        let door_x = b.w / 2;
        let sy = b.surface;
        for x in 0..b.w {
            let was = self.grid[at(x, sy)];
            if !matches!(was, Some(m) if Self::is_ground(world, m)) {
                continue;
            }
            let now = world.get(x, sy);
            if now.organism_id() == 0 && now.material != material::EMPTY {
                continue;
            }
            let covered = (0..sy).any(|yy| matches!(self.grid[at(x, yy)], Some(m) if Self::is_ground(world, m)));
            let cause = match self.cuts_this_frame.iter().find(|c| c.0 == x && c.1 == sy) {
                Some(&(_, _, head_y)) => usize::from(head_y >= sy) * 2 + usize::from(covered),
                None => 4,
            };
            let widen = (x > 0 && room(self.grid[at(x - 1, sy)])) || (x + 1 < b.w && room(self.grid[at(x + 1, sy)]));
            self.surf_opened[cause][usize::from(widen)] += 1;
            let bin = match (x - door_x).abs() {
                0..=4 => 0,
                5..=8 => 1,
                9..=16 => 2,
                _ => 3,
            };
            self.surf_opened_dist[cause][bin] += 1;
            if cause < 4 {
                self.surf_opened_pellets[cause] += Self::spoil_cells_of(&self.grid, &self.put_frame, b, spoil_id, frame, x, sy).0;
            }
            self.surf_last[x as usize] = cause as u8;
            if self.surf_first[x as usize] == u8::MAX {
                self.surf_first[x as usize] = cause as u8;
                self.surf_first_n[cause] += 1;
                if cause == 4 {
                    let name = was.map(|m| world.materials.get(m).name.as_str());
                    self.surf_first_fell[match name {
                        Some("soil") => 0,
                        Some("packedsoil") => 1,
                        Some("spoil") => 2,
                        _ => 3,
                    }] += 1;
                }
            }
        }
        self.cuts_this_frame.clear();
        // The refill scan: every dug cell that was room before the frame and
        // is ground now, unless a pellet was put there (the pellet ledger's).
        for &i in &self.dug_list {
            if !room(self.grid[i]) || self.put_sites.contains(&i) {
                continue;
            }
            let (x, y) = ((i as i32) % b.w, (i as i32) / b.w);
            let now = world.get(x, y);
            if now.organism_id() != 0 || !Self::is_ground(world, now.material) {
                continue;
            }
            let slot = match world.materials.get(now.material).name.as_str() {
                "spoil" => 0,
                "soil" => 1,
                _ => 2,
            };
            self.refill_fall[slot] += 1;
            if !self.refill_pending[i] {
                self.refill_pending[i] = true;
                self.refill_queue.push((frame + REFILL_STANDING, i, 0));
            }
            let above_fell = y > 0 && matches!(self.grid[at(x, y - 1)], Some(m) if Self::is_ground(world, m)) && {
                let a = world.get(x, y - 1);
                a.organism_id() != 0 || a.material == material::EMPTY
            };
            if above_fell {
                self.refill_from_above += 1;
            } else {
                self.refill_from_side += 1;
            }
        }
        self.put_sites.clear();
        // Worked ground turned loose in place: lining or a pellet before the
        // frame, `soil` at the same cell after it.
        let (soil, lining, pellet) = (world.materials.id_of("soil"), world.materials.id_of("packedsoil"), world.materials.id_of("spoil"));
        if let Some(soil) = soil {
            for (i, was) in self.grid.iter().enumerate() {
                let kind = match *was {
                    Some(m) if Some(m) == lining => 0,
                    Some(m) if Some(m) == pellet => 1,
                    _ => continue,
                };
                let (x, y) = ((i as i32) % b.w, (i as i32) / b.w);
                let now = world.get(x, y);
                if now.organism_id() == 0 && now.material == soil {
                    self.turned_loose[kind][usize::from(y < b.surface)] += 1;
                    if kind == 1 && y < b.surface {
                        // What the footing test saw beneath it. A cut in the
                        // last two frames first: the digger stands in it now.
                        let why = if y + 1 >= b.h {
                            5
                        } else {
                            let j = at(x, y + 1);
                            if self.cut_frame[j] != u64::MAX && frame.saturating_sub(self.cut_frame[j]) <= 2 {
                                3
                            } else {
                                match self.grid[j] {
                                    None if self.put_by[i] != 0 && self.grid_org[j] == self.put_by[i] => 0,
                                    None => 1,
                                    Some(m) if m == material::EMPTY => 2,
                                    Some(m) if Self::is_ground(world, m) => {
                                        let under = world.get(x, y + 1);
                                        if under.material == material::EMPTY || under.organism_id() != 0 {
                                            4
                                        } else {
                                            5
                                        }
                                    }
                                    Some(_) => 5,
                                }
                            }
                        };
                        self.loose_why[why] += 1;
                        let age = if self.put_frame[i] == u64::MAX {
                            5
                        } else {
                            match frame.saturating_sub(self.put_frame[i]) {
                                0..=1 => 0,
                                2..=10 => 1,
                                11..=100 => 2,
                                101..=1000 => 3,
                                _ => 4,
                            }
                        };
                        self.loose_age[age] += 1;
                    }
                }
            }
        }
        // The standing check: still ground, so filled rather than passed.
        let due: Vec<(u64, usize, usize)> = self.refill_queue.iter().copied().filter(|p| p.0 <= frame).collect();
        self.refill_queue.retain(|p| p.0 > frame);
        for (_, i, kind) in due {
            self.refill_pending[i] = false;
            let (x, y) = ((i as i32) % b.w, (i as i32) / b.w);
            let c = world.get(x, y);
            if c.organism_id() == 0 && Self::is_ground(world, c.material) {
                self.refill_standing[kind] += 1;
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
        // **Where each packed cell came from** (`gridout=`): a cell that is
        // `packedsoil` now and was not before the frame was packed by this
        // frame's digs, and `grid` still says what it was.
        if self.pack_trace {
            if let Some(packed) = world.materials.id_of("packedsoil") {
                let soil = world.materials.id_of("soil");
                for y in 0..b.h {
                    for x in 0..b.w {
                        let i = at(x, y);
                        let c = world.get(x, y);
                        if c.material != packed || c.organism_id() != 0 {
                            continue;
                        }
                        let was = self.grid.get(i).copied().flatten();
                        if was == Some(packed) {
                            continue;
                        }
                        self.packed_frame[i] = frame;
                        self.packed_from[i] = if y < b.surface {
                            3
                        } else if was == soil && self.dug[i] {
                            2
                        } else if was == soil {
                            1
                        } else {
                            4
                        };
                    }
                }
            }
        }
    }

    /// **The room standing in the old surface row now, by what last opened
    /// each cell**, and its runs (the panel's mouths, the founding cut's own
    /// mouth aside) by their nearest column to the door.
    fn print_openings(&self, frame: u64, world: &World, b: &Box2, portal: &dyn Fn(i32, i32) -> bool) {
        if self.surf_last.len() != b.w as usize {
            return;
        }
        let door_x = b.w / 2;
        let sy = b.surface;
        let open = |x: i32| {
            let c = world.get(x, sy);
            (c.material == material::EMPTY || c.organism_id() != 0) && !portal(x, sy)
        };
        let mut by_cause = [0u64; SURF_CAUSES + 1];
        let mut by_first = [0u64; SURF_CAUSES + 1];
        let mut runs = [0u64; 4];
        let mut run_near: Option<i32> = None;
        for x in 1..b.w - 1 {
            if open(x) {
                let c = self.surf_last[x as usize];
                by_cause[if c == u8::MAX { SURF_CAUSES } else { c as usize }] += 1;
                let c = self.surf_first[x as usize];
                by_first[if c == u8::MAX { SURF_CAUSES } else { c as usize }] += 1;
                let d = (x - door_x).abs();
                run_near = Some(run_near.map_or(d, |n| n.min(d)));
            }
            if !open(x) || x == b.w - 2 {
                if let Some(d) = run_near.take() {
                    runs[match d {
                        0..=4 => 0,
                        5..=8 => 1,
                        9..=16 => 2,
                        _ => 3,
                    }] += 1;
                }
            }
        }
        let causes: Vec<String> = SURF_CAUSE_NAMES.iter().zip(by_cause.iter()).map(|(n, v)| format!("{n} {v}")).collect();
        println!(
            "OPENINGS frame={frame} room standing in the old surface row, by what last opened it: {} | not booked {}; runs of it (mouths, the founding cut's aside) by the nearest column to the door: 3-4 {}, 5-8 {}, 9-16 {}, 17+ {}",
            causes.join(" | "),
            by_cause[SURF_CAUSES],
            runs[0],
            runs[1],
            runs[2],
            runs[3]
        );
        let causes: Vec<String> = SURF_CAUSE_NAMES.iter().zip(by_first.iter()).map(|(n, v)| format!("{n} {v}")).collect();
        println!("OPENINGS frame={frame} the same room, by what first broke its column's crust: {} | not booked {}", causes.join(" | "), by_first[SURF_CAUSES]);
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
            "LEDGER frame={frame} cuts {cuts} + target mismatch {} = engine digs {}: above the old surface {}, a pellet or refill cut again {}, new ground open to the sky {}, new ground under a roof {} (of the new ground, tunnel lining {}; placed by elimination {}); mismatch: ahead refilled {}, ahead not ground {}; the cells cut again were spoil {}, soil {}, lining {}, other {}",
            self.target_mismatch,
            st.digs,
            self.cut_above,
            self.cut_again,
            self.cut_new_open,
            self.cut_new_roofed,
            self.cut_new_lining,
            self.cut_retargeted,
            self.mismatch_refilled,
            self.mismatch_not_ground,
            self.again_spoil,
            self.again_soil,
            self.again_lining,
            self.again_other
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
        println!("LEDGER frame={frame} of the pellets posted up, carried out through the passages {} (engine spoil_lifted_out {})", self.put_out, st.spoil_lifted_out);
        let l = self.lift_from;
        println!(
            "LIFTS frame={frame} pellets posted up the column, by where the carrier stood: in the founding cut {} | under cover 1-4 rows below the old surface {} | 5-8 {} | 9-16 {} | 17+ {} | not under cover {}; drop rolls held inside the nest (SPOIL_OUT keep) {}, of them after patience ran out near the door (SPOIL_HOLD) {}, and after patience ran out with no room beside, kept carrying {}",
            l[0], l[1], l[2], l[3], l[4], l[5], st.spoil_kept_inside, st.spoil_held_near_door, st.spoil_kept_no_lift
        );
        println!(
            "LEDGER frame={frame} dug cells refilled: by a pellet {}, fell in {} (spoil {}, soil {}, other {}; from the cell above {}, from the side {}); still ground {REFILL_STANDING} frames later: by a fall {}, by a pellet {}; worked ground turned loose in place: lining {} below and {} above the old surface, pellets {} below and {} above",
            self.put_refill,
            self.refill_fall.iter().sum::<u64>(),
            self.refill_fall[0],
            self.refill_fall[1],
            self.refill_fall[2],
            self.refill_from_above,
            self.refill_from_side,
            self.refill_standing[0],
            self.refill_standing[1],
            self.turned_loose[0][0],
            self.turned_loose[0][1],
            self.turned_loose[1][0],
            self.turned_loose[1][1]
        );
        let [own, other, air, cut, fell, else_] = self.loose_why;
        let [a1, a10, a100, a1000, older, unknown] = self.loose_age;
        println!(
            "LEDGER frame={frame} pellets above the old surface turned loose {}: on its carrier's own body {own}, on another animal {other}, over air {air}, cut out from under it {cut}, the ground under it fell {fell}, other {else_}; had stood <=1 frame {a1}, <=10 {a10}, <=100 {a100}, <=1000 {a1000}, longer {older}, unknown {unknown}; pellets put down with no footing: on the carrier's own body {}, on another animal {}, over air {} (of them posted up the column: {}, {}, {})",
            self.turned_loose[1][1],
            self.put_unfooted[0],
            self.put_unfooted[1],
            self.put_unfooted[2],
            self.put_unfooted_lifted[0],
            self.put_unfooted_lifted[1],
            self.put_unfooted_lifted[2]
        );
        println!(
            "LEDGER frame={frame} ant-frames holding a pellet {} of {} ({:.1}%)",
            self.held_frames,
            self.ant_frames,
            if self.ant_frames > 0 { 100.0 * self.held_frames as f64 / self.ant_frames as f64 } else { 0.0 }
        );
        let pct = |a: u64, b: u64| if b > 0 { 100.0 * a as f64 / b as f64 } else { 0.0 };
        let [[fd, sd, nd], [fc, sc, nc]] = self.spoil_near;
        println!(
            "LEDGER frame={frame} spoil within {SPOIL_NEAR} cells of the dig target: at the decisions to dig {sd} of {fd} ({:.1}%), fresh (<= {SPOIL_FRESH} frames) {nd} ({:.1}%); at the cuts {sc} of {fc} ({:.1}%), fresh {nc} ({:.1}%)",
            pct(sd, fd),
            pct(nd, fd),
            pct(sc, fc),
            pct(nc, fc)
        );
        let kinds: Vec<String> = ["new mouth from the surface", "new mouth from below", "a mouth already open", "below the old surface", "in the heaps"]
            .iter()
            .zip(self.cut_kind_spoil.iter())
            .map(|(name, &[n, near, fresh, cells, fresh_cells])| {
                let mean = |c: u64| if n > 0 { c as f64 / n as f64 } else { 0.0 };
                format!("{name} {n} (spoil near {:.1}%, fresh {:.1}%, mean {:.2} cells, fresh {:.2})", pct(near, n), pct(fresh, n), mean(cells), mean(fresh_cells))
            })
            .collect();
        println!("LEDGER frame={frame} spoil within {SPOIL_NEAR} cells of the cut, by where it opened: {}", kinds.join(" | "));
        let by_worker: Vec<String> = ["new mouth from the surface", "new mouth from below", "a mouth already open", "below the old surface", "in the heaps"]
            .iter()
            .zip(self.cut_kind_spoil.iter().zip(self.cut_kind_worker.iter()))
            .map(|(name, (row, &w))| format!("{name} {w} of {}", row[0]))
            .collect();
        println!("LEDGER frame={frame} cuts made by nest workers, by where it opened: {}", by_worker.join(" | "));
        let opened: Vec<String> = SURF_CAUSE_NAMES
            .iter()
            .enumerate()
            .map(|(c, name)| {
                let [new, widen] = self.surf_opened[c];
                let d = self.surf_opened_dist[c];
                let pellets = if c < 4 { format!(", pellets near {:.1}", self.surf_opened_pellets[c] as f64 / (new + widen).max(1) as f64) } else { String::new() };
                format!("{name} {} (new {new}, widened {widen}; from the door 3-4 {}, 5-8 {}, 9-16 {}, 17+ {}{pellets})", new + widen, d[0], d[1], d[2], d[3])
            })
            .collect();
        println!("OPENINGS frame={frame} cells of the old surface row opened, by what opened them: {}", opened.join(" | "));
        let places = ["under cover", "in the open at the door", "in the open beyond it"];
        let budget: Vec<String> = places
            .iter()
            .enumerate()
            .map(|(p, name)| {
                let [empty, holding] = self.time_at[p];
                format!("{name} {} ant-frames ({holding} holding a pellet), {} cuts", empty + holding, self.cuts_at[p])
            })
            .collect();
        let mut back = self.back_under.clone();
        back.sort_unstable();
        let q = |f: f64| back.get(((back.len() as f64 - 1.0) * f).round() as usize).copied().unwrap_or(0);
        let out_now = self.ants.values().filter(|t| t.open_drop.is_some()).count();
        println!(
            "TIME frame={frame} where the colony's time went: {} | pellets put down in the open whose carrier came back under cover {}: frames to get back, median {} p90 {}; carriers still out (or died out) {}",
            budget.join(" | "),
            back.len(),
            q(0.5),
            q(0.9),
            out_now
        );
        let first: Vec<String> = SURF_CAUSE_NAMES.iter().zip(self.surf_first_n.iter()).map(|(n, v)| format!("{n} {v}")).collect();
        let f = self.surf_first_fell;
        println!(
            "OPENINGS frame={frame} columns whose crust broke, by what broke it first: {} (the falls were soil {}, lining {}, spoil {}, other {})",
            first.join(" | "),
            f[0],
            f[1],
            f[2],
            f[3]
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
        std::collections::BTreeMap::from([(id, AntBefore { head, heading, digs, holding, pellet: if holding { spoil } else { material::EMPTY }, crop_empty: true, nest_bound: false })])
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

    // 4. The refill ledger: case 3's cave-in was soil arriving with the cell
    //    above it untouched (from the side); now a roof collapse -- cut the
    //    roof, post the pellet out, and let the soil above drop into the cut.
    frame += 1;
    let roof = cut(&mut w, &mut fun, frame, 2);
    frame += 1;
    put(&mut w, &mut fun, frame, head.0, b.surface - 3, 4);
    frame += 1;
    fun.before_with(&w, b, ant(0, 4, false));
    w.set(roof.0, roof.1 - 1, Cell::EMPTY);
    w.set(roof.0, roof.1, Cell::new(soil, 0));
    fun.after_with(&w, b, frame, &after(4, false));
    println!(
        "  funnel: refills that fell in -> soil {}, spoil {}, from above {}, from the side {} (must be 2, 0, 1, 1; the pellet put back in its hole is the pellet ledger's: {})",
        fun.refill_fall[1], fun.refill_fall[0], fun.refill_from_above, fun.refill_from_side, fun.put_refill
    );
    assert_eq!((fun.refill_fall, fun.refill_from_above, fun.refill_from_side, fun.put_refill), ([0, 2, 0], 1, 1, 1), "the refill ledger must place a roof collapse above and a cave-in beside, and leave a put-back pellet to the pellet ledger");

    // 4b. Passes against fills, and worked ground turned loose. A grain
    //     falling through a three-cell dug shaft books three passes and no
    //     standing fill; a grain that stops in one books one; lining with
    //     air below and a pellet with no footing, turned to soil in place,
    //     book one conversion each.
    let (cx, r0) = (b.w / 2 + 20, b.surface + 8);
    for y in r0..r0 + 4 {
        w.set(cx, y, Cell::EMPTY);
    }
    for y in r0..r0 + 3 {
        let i = (y * b.w + cx) as usize;
        fun.dug[i] = true;
        fun.dug_list.push(i);
    }
    // Let the earlier cases' fills (the cave-in, the roof collapse) come due
    // first: they stay ground, so they count, and must not land in this window.
    frame += REFILL_STANDING + 1;
    idle(&mut w, &mut fun, frame, 4);
    let stand0 = fun.refill_standing;
    let falls0: u64 = fun.refill_fall.iter().sum();
    for step in 0..4 {
        frame += 1;
        fun.before_with(&w, b, ant(0, 4, false));
        w.set(cx, r0 + step - 1, Cell::EMPTY);
        w.set(cx, r0 + step, Cell::new(soil, 0));
        fun.after_with(&w, b, frame, &after(4, false));
    }
    frame += REFILL_STANDING + 1;
    idle(&mut w, &mut fun, frame, 4);
    let passes = fun.refill_fall.iter().sum::<u64>() - falls0;
    println!("  funnel: a grain falling through a 3-cell dug shaft -> passes {passes}, standing fills {} (must be 3 and 0)", fun.refill_standing[0] - stand0[0]);
    assert_eq!((passes, fun.refill_standing[0] - stand0[0]), (3, 0), "a grain passing through must not count as a filled hole");
    frame += 1;
    fun.before_with(&w, b, ant(0, 4, false));
    w.set(cx, r0 + 2, Cell::new(soil, 0));
    fun.after_with(&w, b, frame, &after(4, false));
    frame += REFILL_STANDING + 1;
    idle(&mut w, &mut fun, frame, 4);
    let lining_id = w.materials.id_of("packedsoil").expect("packedsoil is a shipped material");
    frame += 1;
    w.set(cx + 2, r0, Cell::new(lining_id, 0));
    w.set(cx + 3, b.surface - 5, Cell::new(spoil, 0));
    idle(&mut w, &mut fun, frame, 4);
    frame += 1;
    fun.before_with(&w, b, ant(0, 4, false));
    w.set(cx + 2, r0, Cell::new(soil, 0));
    w.set(cx + 3, b.surface - 5, Cell::new(soil, 0));
    fun.after_with(&w, b, frame, &after(4, false));
    println!(
        "  funnel: a grain that stops -> standing fills {}; lining and a pellet turned loose -> lining below {}, pellets above {} (must be 1, 1, 1)",
        fun.refill_standing[0] - stand0[0],
        fun.turned_loose[0][0],
        fun.turned_loose[1][1]
    );
    assert_eq!((fun.refill_standing[0] - stand0[0], fun.turned_loose[0][0], fun.turned_loose[1][1]), (1, 1, 1), "a fill that stays and worked ground turned loose must each be booked once");

    // 4c. Why a pellet on the surface turned loose, one case per cause, in a
    //     fresh box: each pellet is put down by hand beside an ant standing on
    //     the surface and turned to soil the frame after (the cut, in the same
    //     frame) -- on its carrier's own body, on another animal, over air,
    //     cut out from under it, and on ground that fell away beneath it.
    {
        let mut w = build(b);
        let mut f = NestFunnel::default();
        let rec = |head: (i32, i32), heading: u8, digs: u32, holding: bool| {
            std::collections::BTreeMap::from([(id, AntBefore { head, heading, digs, holding, pellet: if holding { spoil } else { material::EMPTY }, crop_empty: true, nest_bound: false })])
        };
        let aft = |digs: u32, holding: bool| std::collections::BTreeMap::from([(id, AntAfter { digs, holding })]);
        let hy = b.surface - 1;
        let mut fr = 10u64;
        // Put a pellet at (px, py) beside the head, then turn it loose, with
        // `between` run on the world inside the second frame.
        // `cuts` is the engine's dig count for the second frame: 1 only where
        // `between` is the ant's own cut.
        let mut case = |w: &mut World, f: &mut NestFunnel, head: (i32, i32), (px, py): (i32, i32), cuts: u32, between: &dyn Fn(&mut World)| {
            fr += 1;
            f.before_with(w, b, rec(head, 0, 0, true));
            w.set(px, py, pellet);
            f.after_with(w, b, fr, &aft(0, false));
            fr += 1;
            f.before_with(w, b, rec(head, 7, 0, false));
            between(w);
            w.set(px, py, Cell::new(soil, 0));
            f.after_with(w, b, fr, &aft(cuts, false));
        };
        let x0 = b.w / 2 - 40;
        // (a) On its own body: the carrier's cell straight beneath the put.
        w.set(x0, hy, Cell::new(soil, 0).with_organism_id(id));
        case(&mut w, &mut f, (x0 + 1, hy), (x0, hy - 1), 0, &|_| {});
        // (b) On another animal.
        w.set(x0 + 10, hy, Cell::new(soil, 0).with_organism_id(id + 1));
        case(&mut w, &mut f, (x0 + 11, hy), (x0 + 10, hy - 1), 0, &|_| {});
        // (c) Over air: a hole in the surface under the put.
        w.set(x0 + 21, b.surface, Cell::EMPTY);
        case(&mut w, &mut f, (x0 + 20, hy), (x0 + 21, hy), 0, &|_| {});
        // (d) Cut out from under it: the ant, facing down-right, cuts the
        //     ground the pellet stands on in the frame it turns loose.
        case(&mut w, &mut f, (x0 + 30, hy), (x0 + 31, hy), 1, &|w| w.set(x0 + 31, b.surface, Cell::EMPTY));
        // (e) The ground under it fell: emptied with no dig.
        case(&mut w, &mut f, (x0 + 40, hy), (x0 + 41, hy), 0, &|w| w.set(x0 + 41, b.surface, Cell::EMPTY));
        println!(
            "  funnel: pellets on the surface turned loose -> own body, another animal, air, cut from under, ground fell, other {:?}; put down with no footing: own body, another animal, air {:?}; age <=1 frame {} (must be [1, 1, 1, 1, 1, 0], [1, 1, 1], 5)",
            f.loose_why, f.put_unfooted, f.loose_age[0]
        );
        assert_eq!((f.loose_why, f.put_unfooted, f.loose_age), ([1, 1, 1, 1, 1, 0], [1, 1, 1], [5, 0, 0, 0, 0, 0]), "each cause of a pellet turning loose must be booked once, under its own name");
        assert_eq!(f.cut_new_open + f.cut_above + f.cut_again + f.cut_new_roofed, 1, "case (d)'s cut, and only it, must be placed");
    }

    // 4d. Spoil beside the dig target: a pellet put down by hand, then three
    //     decisions to dig with the jaws free -- beside it while fresh, beside
    //     it once stale, and out of reach of it.
    {
        let mut w = build(b);
        let mut f = NestFunnel::default();
        let rec = |head: (i32, i32), heading: u8, holding: bool| {
            std::collections::BTreeMap::from([(id, AntBefore { head, heading, digs: 0, holding, pellet: if holding { spoil } else { material::EMPTY }, crop_empty: true, nest_bound: false })])
        };
        let aft = |holding: bool| std::collections::BTreeMap::from([(id, AntAfter { digs: 0, holding })]);
        let (hx, hy) = (b.w / 2 + 40, b.surface - 1);
        f.before_with(&w, b, rec((hx, hy), 0, true));
        w.set(hx + 1, hy, pellet);
        f.after_with(&w, b, 10, &aft(false));
        // Facing down-right, at ground one column past the pellet: within 2.
        let face = |f: &mut NestFunnel, w: &World, frame: u64, head: (i32, i32)| {
            f.before_with(w, b, rec(head, 7, false));
            f.after_with(w, b, frame, &aft(false));
        };
        face(&mut f, &w, 11, (hx, hy));
        face(&mut f, &w, 11 + SPOIL_FRESH + 1, (hx, hy));
        face(&mut f, &w, 11 + SPOIL_FRESH + 2, (hx - 20, hy));
        println!("  funnel: decisions to dig beside a fresh pellet, a stale one, none -> faced, spoil near, fresh {:?} (must be [3, 2, 1])", f.spoil_near[0]);
        assert_eq!(f.spoil_near[0], [3, 2, 1], "spoil beside a dig target must be booked, and fresh only while fresh");
    }

    // 4e. Where a cut opens: a new mouth cut from the surface beside a fresh
    //     pellet, the same mouth widened, a new mouth cut from below, a cut
    //     below the old surface, and a cut into a heap.
    {
        let mut w = build(b);
        let mut f = NestFunnel::default();
        let rec = |head: (i32, i32), heading: u8, holding: bool| {
            std::collections::BTreeMap::from([(id, AntBefore { head, heading, digs: 0, holding, pellet: if holding { spoil } else { material::EMPTY }, crop_empty: true, nest_bound: false })])
        };
        let aft = |digs: u32| std::collections::BTreeMap::from([(id, AntAfter { digs, holding: false })]);
        let (x0, s) = (b.w / 2 - 60, b.surface);
        // Pellets put down by hand at frame 10: beside the first mouth, and
        // one standing alone to be cut as a heap.
        for px in [x0 + 1, x0 + 40] {
            f.before_with(&w, b, rec((px + 1, s - 1), 0, true));
            w.set(px, s - 1, pellet);
            f.after_with(&w, b, 10, &aft(0));
        }
        let mut fr = 10u64;
        let mut cut_at = |w: &mut World, f: &mut NestFunnel, head: (i32, i32), heading: u8| {
            fr += 1;
            let (dx, dy) = pixel_physics::sim::creature::DIRS[heading as usize];
            f.before_with(w, b, rec(head, heading, false));
            w.set(head.0 + dx, head.1 + dy, Cell::EMPTY);
            f.after_with(w, b, fr, &aft(1));
        };
        cut_at(&mut w, &mut f, (x0, s - 1), 6); // down: a new mouth, from the surface
        cut_at(&mut w, &mut f, (x0 + 2, s - 1), 5); // down-left: beside it, widened
        w.set(x0 + 20, s + 1, Cell::EMPTY);
        cut_at(&mut w, &mut f, (x0 + 20, s + 1), 2); // up: a new mouth, from below
        cut_at(&mut w, &mut f, (x0 + 20, s + 1), 6); // down: below the old surface
        cut_at(&mut w, &mut f, (x0 + 41, s - 1), 4); // left: into the lone pellet
        println!(
            "  funnel: cuts by where they opened (new from the surface, new from below, open, below, heaps) x (cuts, near, fresh, cells, fresh cells) -> {:?} (must be [[1, 1, 1, 1, 1], [1, 0, 0, 0, 0], [1, 1, 1, 1, 1], [1, 0, 0, 0, 0], [1, 1, 1, 1, 1]])",
            f.cut_kind_spoil
        );
        assert_eq!(
            f.cut_kind_spoil,
            [[1, 1, 1, 1, 1], [1, 0, 0, 0, 0], [1, 1, 1, 1, 1], [1, 0, 0, 0, 0], [1, 1, 1, 1, 1]],
            "each cut must be booked under where it opened, with the spoil beside it"
        );
    }

    // 5. The ledger's own control: a dig the world does not show must be
    //    booked as unplaceable, not quietly dropped or guessed.
    fun.before_with(&w, b, ant(0, 4, false));
    frame += 1;
    fun.after_with(&w, b, frame, &after(5, false));
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
    /// **Where each ant is homed**: the door, as the game founds under one
    /// (`World::door_anchor`), or `None` -- the strip -- where each ant keeps
    /// the cell it landed on as home, which is also what founding does there.
    /// Before 2026-09-29 the trickle homed every ant where it landed even
    /// under a door, so a door arm here was not the game's door.
    anchor: Option<(i32, i32)>,
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
    fn new(target: usize, rate: usize, seed: u64, span: i32, anchor: Option<(i32, i32)>) -> Self {
        let mut order = Vec::new();
        if seed > 0 {
            use pixel_physics::sim::rng;
            order = (0..span).collect();
            let mut draw = rng::stream(seed, 0xD1_6B0C, 0, 0);
            for i in (1..order.len()).rev() {
                order.swap(i, draw.below(i as u32 + 1) as usize);
            }
        }
        Trickle { colony: None, anchor, placed: 0, target, rate, cursor: 0, order }
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
            if let Some(site) = pixel_physics::sim::creature::plant_founder_in(world, x, y, "ant", self.colony, self.anchor) {
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

/// **Every pellet's trip, from the cut to where it went down** (`TRIPS`).
///
/// Built 2026-09-29 for the question the ledgers could not answer: under
/// `PIXEL_PHYSICS_SPOIL_OUT` with `keep` never lifting from inside the nest,
/// three quarters of the colony's ant-frames under cover were spent holding a
/// pellet, some 680 a pellet at 200 ants, to carry it out of a nest ten rows
/// deep. A rate says how often, never why (`CLAUDE.md`, *tracing
/// individuals*), so each trip is followed frame by frame: did the head move,
/// and if it stood, what was in the cell it faced -- an animal, ground, or
/// nothing (it chose not to step) -- and was the carrier's patience under the
/// give-up line. Reads the world between frames and writes nothing.
/// `tripcsv=PATH` also writes one row per carrier per frame.
#[derive(Default)]
struct TripLog {
    open: std::collections::BTreeMap<u32, Trip>,
    closed: Vec<Trip>,
    before: std::collections::BTreeMap<u32, TripAnt>,
    csv: Option<std::io::BufWriter<std::fs::File>>,
    /// `decisions=PATH`: the engine's own per-decision row
    /// (`creature::DecisionRow`, via `World::decision_log`) for every decision
    /// an animal made holding a pellet -- which headings were usable, the move
    /// roll against `p_move`, what came of it, the chooser's patience.
    decisions: Option<std::io::BufWriter<std::fs::File>>,
    /// **Who a carrier stood facing** (`JAM`), by what that animal was doing
    /// ([`TripAnt::role`]), and of those how many stood still themselves.
    /// The report said carriers stood "behind one another" in the shaft at
    /// 200 ants and never traced it: a queue of carriers, nest workers at
    /// home in the cut and diggers on their way down want different fixes.
    stood_by: [u64; 4],
    stood_by_still: [u64; 4],
    /// Heads in the founding shaft each frame, summed by role, over
    /// `shaft_frames` frames; and in the rest of the founding cut.
    shaft_pop: [u64; 4],
    cut_pop: [u64; 4],
    /// Heads on the two rows over the mouth (a column either side), by role:
    /// the colony standing on its own way out. Every founder's home is the
    /// cell over the mouth's middle (`World::door_anchor`).
    over_pop: [u64; 4],
    shaft_frames: u64,
    /// The same stands by where the carrier was ([`TripAnt::place`]) and
    /// what it faced: an animal by role, then ground, then nothing.
    stood_at: [[u64; 6]; 4],
    /// **Each animal's last decision row and the frame its head last
    /// moved**, for [`pile_census`]: who stands on the mound over the mouth,
    /// who is underground, and what each was deciding. Filled only under
    /// `pile` or `antscsv=`, which switch the engine's decision log on (a
    /// trace; the run is bit-identical with it).
    last_row: std::collections::BTreeMap<u32, pixel_physics::sim::creature::DecisionRow>,
    last_moved: std::collections::BTreeMap<u32, u64>,
}

#[derive(Clone, Copy)]
struct TripAnt {
    head: (i32, i32),
    heading: u8,
    holding: bool,
    patience: f32,
    covered: bool,
    in_cut: bool,
    /// Its head in the shaft proper, not the chamber or the side room.
    in_shaft: bool,
    /// What it is doing: 0 carrying a pellet, 1 a store load, 2 nest-bound
    /// with nothing held, 3 anything else (a digger, a scout, an idler).
    role: u8,
    /// Where: 0 at the mouth (the rim and the shaft's top rows,
    /// `ShaftFootprint::touches_mouth`), 1 lower in the shaft, 2 elsewhere in
    /// the founding cut (the chamber, the side room), 3 outside the cut.
    place: u8,
}

/// [`TripAnt::place`]'s names, for `JAM`.
const JAM_PLACES: [&str; 4] = ["at the mouth", "lower in the shaft", "in the chamber or side room", "outside the cut"];

/// [`TripAnt::role`]'s names, for `JAM`.
const JAM_ROLES: [&str; 4] = ["carrying a pellet", "a store load", "nest-bound, empty", "other, empty"];

#[derive(Clone, Copy, Default)]
struct Trip {
    start: u64,
    /// Rows below the old surface of the head when the pellet was taken up.
    depth: i32,
    in_cut_start: bool,
    frames: u64,
    moved: u64,
    stood_ant: u64,
    stood_ground: u64,
    stood_open: u64,
    low_patience: u64,
    in_cut: u64,
    covered: u64,
    open_air: u64,
    /// Frames from taking the pellet up to the first frame outside the nest.
    left_after: Option<u64>,
    /// 0 put down outside the nest, 1 put down inside it, 2 died holding.
    end: u8,
}

/// Patience under which `SPOIL_OUT`'s `keep` lets a carrier go
/// (`creature::DIG_RETURN_GIVE_UP`, private there).
const TRIP_GIVE_UP: f32 = 0.1;

impl TripLog {
    fn ants(world: &World) -> std::collections::BTreeMap<u32, TripAnt> {
        let mut out = std::collections::BTreeMap::new();
        for id in world.live_organism_ids() {
            let Some(st) = world.organism(id) else { continue };
            if world.species.get(st.species).creature.is_none() {
                continue;
            }
            let Some(&(hx, hy)) = st.chain.first() else { continue };
            let covered = (1..=COVER_ROWS).any(|dy| {
                let c = world.get(hx, hy - dy);
                c.material != material::EMPTY && c.organism_id() == 0 && matches!(world.materials.kind(c.material), MaterialKind::Powder | MaterialKind::Solid)
            });
            let cut = world.nearest_nest_site(hx, hy).and_then(|i| world.nest_sites.get(i)).and_then(|s| s.shaft);
            let in_cut = cut.is_some_and(|c| c.contains(hx, hy));
            let in_shaft = cut.is_some_and(|c| c.in_shaft(hx, hy));
            let place = match cut {
                Some(c) if c.touches_mouth(hx, hy) => 0,
                Some(c) if c.in_shaft(hx, hy) => 1,
                Some(c) if c.contains(hx, hy) => 2,
                _ => 3,
            };
            let role = match st.spoil {
                Some(p) if !p.store => 0,
                Some(_) => 1,
                None if st.nest_bound_until > world.frame => 2,
                None => 3,
            };
            out.insert(
                id,
                TripAnt { head: (hx, hy), heading: st.heading, holding: st.spoil.is_some_and(|p| !p.store), patience: st.home_patience, covered, in_cut, in_shaft, role, place },
            );
        }
        out
    }

    fn before(&mut self, world: &World) {
        self.before = Self::ants(world);
    }

    /// Drains the frame's decision rows, keeping those of animals that held a
    /// pellet before the frame. Call between the step and [`Self::after`].
    fn log_decisions(&mut self, world: &mut World) {
        use pixel_physics::sim::creature::{DECISION_OUTCOME_NAMES, DROP_WHY_NAMES};
        use std::io::Write;
        let Some(log) = world.decision_log.as_mut() else { return };
        let rows = std::mem::take(log);
        for r in &rows {
            self.last_row.insert(r.id, *r);
        }
        let Some(w) = self.decisions.as_mut() else { return };
        for r in rows {
            if !self.before.get(&r.id).is_some_and(|a| a.holding) {
                continue;
            }
            let _ = writeln!(
                w,
                "{},{},{},{},{},{},{},{},{:08b},{},{},{:.3},{:.3},{:.3},{:.3},{},{:.3},{:.3},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.1}",
                r.frame,
                r.id,
                r.head.0,
                r.head.1,
                r.head_after.0,
                r.head_after.1,
                r.heading,
                r.heading_after,
                r.usable,
                r.anchor.0,
                r.anchor.1,
                r.home_aligned,
                r.p_move,
                r.roll_move,
                r.roll_tumble,
                DECISION_OUTCOME_NAMES[r.outcome as usize],
                r.patience,
                r.chosen_cos,
                DROP_WHY_NAMES[r.drop as usize],
                r.drop_roll,
                r.drop_p,
                r.at_nest,
                r.crowding,
                r.energy,
                r.energy_j
            );
        }
    }

    fn after(&mut self, world: &World, b: &Box2, frame: u64) {
        use pixel_physics::sim::creature::DIRS;
        use std::io::Write;
        let now = Self::ants(world);
        for (&id, a) in &now {
            if self.before.get(&id).is_none_or(|p| p.head != a.head) {
                self.last_moved.insert(id, frame);
            }
        }
        for (&id, pre) in &self.before {
            let post = now.get(&id);
            if pre.holding {
                let t = self.open.entry(id).or_insert_with(|| Trip { start: frame, depth: pre.head.1 - b.surface, in_cut_start: pre.in_cut, ..Trip::default() });
                t.frames += 1;
                let inside = pre.covered || pre.in_cut;
                if pre.in_cut {
                    t.in_cut += 1;
                } else if pre.covered {
                    t.covered += 1;
                } else {
                    t.open_air += 1;
                    t.left_after.get_or_insert(frame.saturating_sub(t.start));
                }
                if pre.patience < TRIP_GIVE_UP {
                    t.low_patience += 1;
                }
                // 0 moved; else what stood in the cell it faced: 1 an animal,
                // 2 ground, 3 nothing.
                let ahead = match post {
                    Some(q) if q.head != pre.head => 0u8,
                    _ => {
                        let (dx, dy) = DIRS[pre.heading as usize % 8];
                        let c = world.get(pre.head.0 + dx, pre.head.1 + dy);
                        if c.organism_id() != 0 {
                            1
                        } else if c.material != material::EMPTY && matches!(world.materials.kind(c.material), MaterialKind::Powder | MaterialKind::Solid) {
                            2
                        } else {
                            3
                        }
                    }
                };
                match ahead {
                    0 => t.moved += 1,
                    1 => t.stood_ant += 1,
                    2 => t.stood_ground += 1,
                    _ => t.stood_open += 1,
                }
                if ahead >= 2 {
                    self.stood_at[pre.place as usize][2 + ahead as usize] += 1;
                }
                if ahead == 1 {
                    let (dx, dy) = DIRS[pre.heading as usize % 8];
                    let bid = world.get(pre.head.0 + dx, pre.head.1 + dy).organism_id();
                    let role = self.before.get(&bid).map_or(3, |a| a.role) as usize;
                    self.stood_by[role] += 1;
                    self.stood_at[pre.place as usize][role] += 1;
                    if self.before.get(&bid).zip(now.get(&bid)).is_some_and(|(a, q)| a.head == q.head) {
                        self.stood_by_still[role] += 1;
                    }
                }
                if let Some(w) = self.csv.as_mut() {
                    let _ = writeln!(
                        w,
                        "{frame},{id},{},{},{},{},{ahead},{:.3},{},{}",
                        t.start,
                        pre.head.0,
                        pre.head.1,
                        pre.heading,
                        pre.patience,
                        u8::from(inside),
                        u8::from(pre.in_cut)
                    );
                }
                let ended = match post {
                    None => Some(2),
                    Some(q) if !q.holding => Some(u8::from(inside)),
                    _ => None,
                };
                if let Some(end) = ended {
                    let mut t = self.open.remove(&id).expect("opened above");
                    t.end = end;
                    self.closed.push(t);
                }
            }
        }
        let cut = world.nest_sites.iter().find_map(|s| s.shaft);
        for a in now.values() {
            if a.in_shaft {
                self.shaft_pop[a.role as usize] += 1;
            } else if a.in_cut {
                self.cut_pop[a.role as usize] += 1;
            }
            if cut.is_some_and(|c| (c.x0 - 1..=c.x1 + 1).contains(&a.head.0) && (c.top - 2..c.top).contains(&a.head.1)) {
                self.over_pop[a.role as usize] += 1;
            }
        }
        self.shaft_frames += 1;
        self.before = now;
    }

    fn print(&self, frame: u64) {
        let stood = self.stood_by.iter().sum::<u64>().max(1) as f64;
        let by = (0..4)
            .map(|r| format!("{} {:.1}% (itself still {:.1}%)", JAM_ROLES[r], 100.0 * self.stood_by[r] as f64 / stood, 100.0 * self.stood_by_still[r] as f64 / self.stood_by[r].max(1) as f64))
            .collect::<Vec<_>>()
            .join(", ");
        let mean = |pop: &[u64; 4]| (0..4).map(|r| format!("{} {:.2}", JAM_ROLES[r], pop[r] as f64 / self.shaft_frames.max(1) as f64)).collect::<Vec<_>>().join(", ");
        println!(
            "JAM frame={frame} carrying frames standing facing an animal {}: it was {by} | heads in the founding shaft, mean over frames: {} | in the rest of the founding cut: {} | on the two rows over the mouth: {}",
            self.stood_by.iter().sum::<u64>(),
            mean(&self.shaft_pop),
            mean(&self.cut_pop),
            mean(&self.over_pop)
        );
        let all = self.stood_at.iter().flatten().sum::<u64>().max(1) as f64;
        let at = (0..4)
            .map(|p| {
                let r = &self.stood_at[p];
                format!(
                    "{} {:.1}% (facing a carrier {}, a store load {}, nest-bound {}, other {}, ground {}, nothing {})",
                    JAM_PLACES[p],
                    100.0 * r.iter().sum::<u64>() as f64 / all,
                    r[0],
                    r[1],
                    r[2],
                    r[3],
                    r[4],
                    r[5]
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        println!("JAMAT frame={frame} carrying frames standing, by where the carrier stood: {at}");
        let c = &self.closed;
        let n = c.len();
        let q = |mut v: Vec<u64>, p: f64| -> u64 {
            if v.is_empty() {
                return 0;
            }
            v.sort_unstable();
            v[((v.len() - 1) as f64 * p).round() as usize]
        };
        let ends = |e: u8| c.iter().filter(|t| t.end == e).count();
        let durs: Vec<u64> = c.iter().map(|t| t.frames).collect();
        let left: Vec<u64> = c.iter().filter_map(|t| t.left_after).collect();
        let sum = |f: &dyn Fn(&Trip) -> u64| c.iter().map(f).sum::<u64>();
        let all = sum(&|t| t.frames).max(1) as f64;
        let pct = |v: u64| 100.0 * v as f64 / all;
        let by = |lo: i32, hi: i32| {
            let v: Vec<u64> = c.iter().filter(|t| !t.in_cut_start && (lo..=hi).contains(&t.depth)).map(|t| t.frames).collect();
            format!("{}/{}", v.len(), q(v, 0.5))
        };
        let cut: Vec<u64> = c.iter().filter(|t| t.in_cut_start).map(|t| t.frames).collect();
        println!(
            "TRIPS frame={frame} pellets carried and let go {n} (outside the nest {}, inside it {}, died holding {}), still held {} | frames a trip median {} p90 {} | left the nest on {} of them, frames to leave median {} p90 {} | of the carrying frames: moving {:.1}%, standing facing an animal {:.1}%, facing ground {:.1}%, facing nothing {:.1}% | patience under the give-up line {:.1}% | in the founding cut {:.1}%, under cover elsewhere {:.1}%, in the open {:.1}% | trips by where the pellet was taken up (n/median frames): in the cut {}/{}, 1-4 rows down {}, 5-8 {}, 9-16 {}, 17+ {}",
            ends(0),
            ends(1),
            ends(2),
            self.open.len(),
            q(durs.clone(), 0.5),
            q(durs, 0.9),
            left.len(),
            q(left.clone(), 0.5),
            q(left, 0.9),
            pct(sum(&|t| t.moved)),
            pct(sum(&|t| t.stood_ant)),
            pct(sum(&|t| t.stood_ground)),
            pct(sum(&|t| t.stood_open)),
            pct(sum(&|t| t.low_patience)),
            pct(sum(&|t| t.in_cut)),
            pct(sum(&|t| t.covered)),
            pct(sum(&|t| t.open_air)),
            cut.len(),
            q(cut, 0.5),
            by(i32::MIN, 4),
            by(5, 8),
            by(9, 16),
            by(17, i32::MAX),
        );
    }
}

/// **The mound over the nest, by distance from the door** (`CRATER`, every
/// stop): cells of ground standing above the old surface -- spoil, and
/// whatever it turned into -- in bins of columns from the nest site's
/// centre, and how many stand over the mouth's own columns. Built for
/// `PIXEL_PHYSICS_SPOIL_RING` (`creature::spoil_ring`), whose question is
/// whether the colony stops burying its own door (one-entrance report §11):
/// the first bin is the door and the mouth, and a crater is a dip there with
/// a ring beyond. With the carry counters beside it, the "it fired" half.
fn crater(world: &World, b: &Box2, frame: u64) -> String {
    const BINS: [(i32, i32, &str); 6] = [(0, 2, "0-2"), (3, 5, "3-5"), (6, 9, "6-9"), (10, 14, "10-14"), (15, 24, "15-24"), (25, i32::MAX, "25+")];
    let Some(site) = world.nest_sites.first() else { return format!("CRATER frame={frame} no nest site") };
    let mut bins = [0u32; 6];
    let mut over_mouth = 0u32;
    let mouth = site.shaft.map(|c| (c.x0, c.x1));
    for x in 0..b.w {
        for y in 0..b.surface {
            let c = world.get(x, y);
            if c.material == material::EMPTY || c.organism_id() != 0 || !matches!(world.materials.kind(c.material), MaterialKind::Powder | MaterialKind::Solid) {
                continue;
            }
            let d = (x - site.x).abs();
            if let Some(i) = BINS.iter().position(|&(lo, hi, _)| (lo..=hi).contains(&d)) {
                bins[i] += 1;
            }
            if mouth.is_some_and(|(x0, x1)| (x0..=x1).contains(&x)) {
                over_mouth += 1;
            }
        }
    }
    let st = world.creature_stats;
    let by = BINS.iter().zip(bins).map(|(&(_, _, name), n)| format!("{name} {n}")).collect::<Vec<_>>().join(", ");
    format!(
        "CRATER frame={frame} ground above the old surface, by columns from the nest's centre: {by} | over the mouth's columns {over_mouth} | carry (SPOIL_RING) distances drawn {}, drop rolls held short of them {}, let go back in a tunnel {}",
        st.spoil_ring_drawn, st.spoil_ring_held, st.spoil_ring_let_go
    )
}

/// **How wide the nest's passages are** (`WIDTH`, every stop): every open
/// cell below the old surface -- empty, or an animal standing in it -- by
/// the width of the passage it sits in, the shorter of its open runs across
/// and down, capped at 3. A passage the dig cut one cell at a time reads 1;
/// `PIXEL_PHYSICS_DIG_WIDEN` (`creature::dig_widen_of`) is for making it 2,
/// and its counter is beside the census as the "it fired" half.
fn widths(world: &World, b: &Box2, frame: u64) -> String {
    let open = |x: i32, y: i32| {
        if x < 0 || x >= b.w || y < b.surface || y >= b.floor {
            return false;
        }
        let c = world.get(x, y);
        c.material == material::EMPTY || matches!(world.materials.kind(c.material), MaterialKind::Creature)
    };
    let run = |x: i32, y: i32, (sx, sy): (i32, i32)| {
        let mut n = 1;
        for sign in [-1, 1] {
            let mut k = 1;
            while n < 3 && open(x + sign * sx * k, y + sign * sy * k) {
                n += 1;
                k += 1;
            }
        }
        n.min(3)
    };
    let mut by = [0u32; 3];
    // **Thick**: the cell is one of a 2x2 block of open cells, which a
    // passage one cell across never has, straight or diagonal. The run test
    // above reads a diagonal band two cells across as 1 (its vertical run is
    // one), so this is the reading for how a gallery looks. **Corner-only**:
    // open, with no open cell beside it but one at a corner -- a diagonal
    // line of single cells, the thinnest thing the dig makes.
    let (mut thick, mut corner_only) = (0u32, 0u32);
    for y in b.surface..b.floor {
        for x in 0..b.w {
            if open(x, y) {
                let w = run(x, y, (1, 0)).min(run(x, y, (0, 1)));
                by[(w - 1) as usize] += 1;
                if [(-1, -1), (0, -1), (-1, 0), (0, 0)].iter().any(|&(ox, oy)| (0..2).all(|i| (0..2).all(|j| open(x + ox + i, y + oy + j)))) {
                    thick += 1;
                }
                if !(open(x - 1, y) || open(x + 1, y) || open(x, y - 1) || open(x, y + 1)) && (open(x - 1, y - 1) || open(x + 1, y - 1) || open(x - 1, y + 1) || open(x + 1, y + 1)) {
                    corner_only += 1;
                }
            }
        }
    }
    let total = by.iter().sum::<u32>().max(1) as f64;
    format!(
        "WIDTH frame={frame} open cells below the old surface by passage width: 1 cell {} ({:.0}%), 2 cells {} ({:.0}%), 3 or more {} ({:.0}%) | in a 2x2 open block {} ({:.0}%), joined at a corner only {} ({:.0}%) | walls cut to widen a passage (DIG_WIDEN) {}",
        by[0],
        100.0 * by[0] as f64 / total,
        by[1],
        100.0 * by[1] as f64 / total,
        by[2],
        100.0 * by[2] as f64 / total,
        thick,
        100.0 * thick as f64 / total,
        corner_only,
        100.0 * corner_only as f64 / total,
        world.creature_stats.digs_widened
    )
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
    // **`look=` picks the colours a sheet is drawn in**: `lab` (the default,
    // owner 2026-09-29: "match the lab colors, not sky background, easier to
    // see ants and tunnels") or `sky`, the open-country look every sheet
    // before that date was drawn in. Pictures only -- see [`LabLook`] for why
    // the numbers cannot move.
    let lab_look = arg::<String>("look").is_none_or(|v| v != "sky");

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
            // **With food the ant keeps `ant.ron`'s endowment** (200 J), as
            // the colony bed does: the 20,000 J box default is an endowment
            // nothing ever has to eat against, which is what `food=` exists
            // to end.
            let fed_by_pile = arg::<i32>("food").unwrap_or(0) > 0;
            c.start_energy = arg("energy").unwrap_or(if fed_by_pile { c.start_energy } else { 20_000.0 });
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
    // Read after the paint, which cuts the founding shaft the anchor sits on.
    let door_anchor = world.door_anchor(b.w / 2, b.surface - 1, "ant");
    let mut food_pile = FoodPile::from_args(&mut world, &b);
    let mut trickle = Trickle::new(ants as usize, arg("rate").unwrap_or(4), seed, span, door_anchor);

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
    match &food_pile {
        None => println!(
            "  no food, no plants, no lamps, no weather -- FoodAdjacent is 0 by construction, so the dig you see is the nest mechanism alone"
        ),
        Some(p) => println!(
            "  FOOD: {} cells of {} at x {} ({} columns east of the nest), refilled every {} frames; onlyfood {}; no plants, no lamps, no weather",
            p.n,
            world.materials.get(p.larder).name,
            p.x,
            p.x - b.w / 2,
            p.refill,
            if arg::<String>("onlyfood").as_deref() == Some("off") { "off" } else { "on" }
        ),
    }
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
        None => println!("  founding: painted strip only, nothing dug (PIXEL_PHYSICS_NEST_SHAFT=off)"),
    }
    // **Where the site and its cut are**, since `SPOIL_OUT`'s haul pulls a
    // carrier to `(site.x, site.surface)`: a log that does not say whether
    // that point is inside the cut cannot say where the walk out ends.
    for s in &world.nest_sites {
        match s.shaft {
            Some(c) => println!(
                "  site: x {} surface {} | shaft columns {}..{} rows {}..{} (mouth to {}) | chamber columns {}..{} rows {}..{} | the haul's target ({}, {}) inside the cut: {}",
                s.x, s.surface, c.x0, c.x1, c.top, c.bottom, c.mouth_bottom, c.chamber_x0, c.chamber_x1, c.chamber_top, c.chamber_bottom, s.x, s.surface, c.contains(s.x, s.surface)
            ),
            None => println!("  site: x {} surface {} | no cut", s.x, s.surface),
        }
    }
    // **The door and the storeroom, echoed** for the reason the shaft is: both
    // ship on since 2026-09-29, and a log that does not name them cannot say
    // which founding it ran.
    match pixel_physics::sim::creature::nest_door_of(&world) {
        Some(d) => println!("  door: half-width {d} painted (PIXEL_PHYSICS_NEST_DOOR), every ant homed at {door_anchor:?}, as founding homes them"),
        None => println!("  door: the strip (PIXEL_PHYSICS_NEST_DOOR=off), every ant homed where it lands"),
    }
    println!("  storeroom: {} (PIXEL_PHYSICS_STOREROOM)", pixel_physics::sim::creature::storeroom_of(&world));
    println!("  {}", pixel_physics::sim::creature::spoil_switches_line());
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
    if lab_look {
        renderer.creature_colour = pixel_physics::render::CreatureColour::Colony;
    }
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
    // **Every pellet's trip** (see [`TripLog`]), with the funnel.
    let mut trips = TripLog::default();
    if let Some(path) = arg::<String>("tripcsv") {
        use std::io::Write;
        let mut w = std::io::BufWriter::new(std::fs::File::create(&path).expect("tripcsv: cannot create the file"));
        let _ = writeln!(w, "frame,id,trip_start,hx,hy,heading,ahead,patience,inside,in_cut");
        trips.csv = Some(w);
    }
    // **`gridout=PATH`: the whole box at every stop** -- see [`write_grid`].
    let mut grid_out = arg::<String>("gridout").map(|path| std::io::BufWriter::new(std::fs::File::create(&path).expect("gridout: cannot create the file")));
    funnel.pack_trace = grid_out.is_some();
    // **Fed by default since 2026-09-29** -- see [`feed`]. The owner, asked
    // whether the box should keep its colony fed so every picture shows a
    // living colony: "Yes". `hungry` restores the starving box every run
    // before that date was taken on; `fed` is still accepted and does
    // nothing more.
    let fed = !flag("hungry");
    if fed {
        println!("  fed: every ant topped up to start_energy each frame (booked as granted); no food on the ground, so FoodAdjacent stays 0 (`hungry` for the starving box)");
    } else {
        println!("  hungry: no ant is fed, so the colony is below start_energy from its first tick and starves (the box before 2026-09-29)");
    }
    // **`pile` / `antscsv=PATH`: who stands where, and why** -- see
    // [`pile_census`]. Both switch the engine's decision log on.
    let pile_on = flag("pile") || arg::<String>("antscsv").is_some();
    let mut ants_csv = arg::<String>("antscsv").map(|path| {
        use std::io::Write;
        let mut w = std::io::BufWriter::new(std::fs::File::create(&path).expect("antscsv: cannot create the file"));
        let _ = writeln!(w, "{}", PILE_CSV_HEADER);
        w
    });
    if pile_on {
        world.decision_log = Some(Vec::new());
    }
    if let Some(path) = arg::<String>("decisions") {
        use std::io::Write;
        let mut w = std::io::BufWriter::new(std::fs::File::create(&path).expect("decisions: cannot create the file"));
        let _ = writeln!(w, "frame,id,hx,hy,hx_after,hy_after,heading,heading_after,usable,anchor_x,anchor_y,home_aligned,p_move,roll_move,roll_tumble,outcome,patience,chosen_cos,drop,drop_roll,drop_p,at_nest,crowding,energy,energy_j");
        trips.decisions = Some(w);
        world.decision_log = Some(Vec::new());
    }
    for f in 0..=frames {
        if f > 0 {
            if funnel_on {
                funnel.before(&world, &b);
                trips.before(&world);
            }
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
            world.step_pheromones();
            if funnel_on {
                trips.log_decisions(&mut world);
                funnel.after(&world, &b, f);
                trips.after(&world, &b, f);
            }
        }
        trickle.step(&mut world, &b);
        if fed {
            feed(&mut world);
        }
        if let Some(p) = food_pile.as_mut().filter(|p| f > 0 && p.refill > 0 && f.is_multiple_of(p.refill)) {
            p.place(&mut world);
        }
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
            println!("{}", crater(&world, &b, f));
            println!("{}", widths(&world, &b, f));
            if flag("trace") {
                trace(&world);
            }
            if funnel_on && f > 0 {
                println!("{}", fill_census(&world, &b, &funnel.dug, f));
                if let Some(w) = grid_out.as_mut() {
                    write_grid(w, &world, &b, &funnel, f);
                    println!("{}", pack_census(&world, &funnel, f));
                }
                if pile_on {
                    println!("{}", pile_census(&world, &b, &trips, f, ants_csv.as_mut()));
                }
                if let Some(p) = &food_pile {
                    let st = world.creature_stats;
                    println!(
                        "FOOD frame={f} cells placed {} (the refill skipped {} occupied slots), standing in the pile {}; live ants {}, born {}, deliveries {}; buds held for the colony's hunger {}",
                        p.placed,
                        p.skipped,
                        p.standing(&world),
                        TripLog::ants(&world).len(),
                        st.births,
                        st.deliveries,
                        st.buds_held_for_need
                    );
                }
                println!(
                    "REST frame={f} resting inside (PIXEL_PHYSICS_NEST_REST) {:?}: decisions under the rest pull {}, the way in {} steps deep",
                    pixel_physics::sim::creature::nest_rest_of(&world),
                    world.creature_stats.rest_pulls,
                    world.nest_ways.first().map_or(0, |w| w.depth())
                );
                funnel.print(f, &world);
                trips.print(f);
                let cut = world.nest_sites.iter().find_map(|s| s.shaft);
                funnel.print_openings(f, &world, &b, &|x, y| cut.is_some_and(|c| c.contains(x, y)));
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
                let look = lab_look.then(|| LabLook::dress(&mut world, &b));
                renderer.draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
                if let Some(look) = look {
                    look.undress(&mut world);
                }
                if tint_out.is_some() {
                    let mut tinted = buf.clone();
                    tint(&world, &b, &funnel.dug, &mut tinted);
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
        "SUMMARY digs={} rolls={} per_roll={:.3} roofed={roofed} open={open} ants_in_it={bodies} room_total={} hauled_up={above} spoil_dumped={} room={rw}w x{rh}h vert={:.2} iqr={iqr} p50x={p50x:+} aimed_down={} down_refused={}",
        st.digs,
        st.dig_rolls,
        if st.dig_rolls > 0 { st.digs as f64 / st.dig_rolls as f64 } else { 0.0 },
        roofed + open + bodies,
        st.spoil_dumped,
        if rw > 0 { rh as f64 / rw as f64 } else { 0.0 },
        st.digs_aimed_down,
        st.digs_down_refused
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
        // `PIXEL_PHYSICS_SPOIL_CUE`'s "it fired" half; the funnel's cut-kind
        // line is the effect, from the far side of the call.
        if st.spoil_cue_applied > 0 {
            println!(
                "SUMMARY heap cue: scaled {} dig rolls, mean factor {:.3} of the urge let through",
                st.spoil_cue_applied,
                st.spoil_cue_kept_milli as f64 / 1000.0 / st.spoil_cue_applied as f64
            );
        }
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

/// **The lab's colours, worn for the one instant a picture is taken.**
///
/// Owner, 2026-09-29: *"change your test/images to match the lab colors? Not
/// sky background, easier to see ants and tunnels."* Outdoors, dug void draws
/// as the sky carried underground -- a dark blue -- the air over the box is
/// a bright band that takes the eye off the ground, and the ant's own
/// palette (`ant.ron`, 28-52 on every channel) is a near-black brown within a
/// few steps of both the soil and the tunnel. The lab answers all three, and
/// this wears its three answers:
///
/// - **the air is a room**, a dark slate wall, and dug space below the bench
///   line is warm near-black earth (`sky::Interior`), so a tunnel reads as a
///   hole in the ground rather than as a strip of night;
/// - **each colony wears its group colour** (`CreatureColour::Colony`, what
///   `Lab::new` opens on), amber for the first, so an ant stands off both
///   the soil and the tunnel it is walking in;
/// - **the painted nest is worked earth** (`lab::earth_toned_nest`, whose
///   tones this copies, since that function is private to the lab), so the
///   door is not a second tan-and-amber thing to tell apart from the ants.
///
/// **No lamps.** The lab's pools of light sit under a ceiling many rows above
/// the bench and are out of frame wherever the ground is; this box has 24
/// rows of air, so the same pools would sit in every picture as a bright
/// band -- the thing this replaces. An empty lamp list is the room unlit by
/// fixtures, which at the bench is what the lab looks like.
///
/// **Worn only around `Renderer::draw` and taken off before the next frame
/// is simulated.** `World::set_enclosure`'s doc said no simulation pass
/// reads the room; two do. `evaporation::is_enclosed` values the water a
/// drying cell releases at the sealed-box rate (260 against 2 in open air),
/// which humidifies the air over the soil and brakes further drying, and
/// `weather::condense_under_a_lid` drips banked water back from the ceiling
/// -- and the dig wiring reads soil moisture (`(MoistureGrad, Dig, -0.55)`
/// in the module doc). Measured 2026-09-29 with a scratch binary that left
/// the room declared for the whole run (walked 40 ants seed 9, today's 40
/// ants seed 2 and 200 ants seed 21, 24,000 frames each): **every census
/// line identical**, and the only trace is surface and floor soil drawn 1-2
/// colour steps apart from frame 18,000 on -- the soil is wetter, and no ant
/// has yet read the difference. Harmless in these runs, and not guaranteed
/// in a longer or wetter one, so the room stays on for the draw alone:
/// declared there, nothing that steps the world can see it. The colony
/// colour is the renderer's own setting and reaches nothing else.
struct LabLook {
    nest: Option<(MaterialId, Vec<[u8; 4]>, usize)>,
}

impl LabLook {
    const WORKED_EARTH: [[u8; 4]; 4] = [[48, 38, 32, 255], [56, 45, 38, 255], [42, 33, 28, 255], [62, 50, 42, 255]];

    fn dress(world: &mut World, b: &Box2) -> Self {
        world.set_enclosure(Some(pixel_physics::sim::enclosure::Enclosure::new(0, b.surface)));
        let nest = world.materials.id_of("nest").map(|id| {
            let def = world.materials.get_mut(id);
            let old = (id, std::mem::replace(&mut def.palette, Self::WORKED_EARTH.to_vec()), def.base_shades);
            def.base_shades = Self::WORKED_EARTH.len();
            old
        });
        LabLook { nest }
    }

    fn undress(self, world: &mut World) {
        world.set_enclosure(None);
        if let Some((id, palette, shades)) = self.nest {
            let def = world.materials.get_mut(id);
            def.palette = palette;
            def.base_shades = shades;
        }
    }
}

/// **`food=N`: a pile of food out on the surface, the colony bed's**
/// (`examples/trailfollow.rs`'s `place_food`, at its gap-90 settings), so the
/// nest is built by a colony that has to feed itself. Owner, 2026-10-01:
/// "lets try to build a nest with in test environment with hungry ants and
/// food. No plants. Use something similar to the gap 90 environment."
///
/// `N` cells of `larder=` (fruit) stacked twelve wide, `gap=` (90) columns
/// east of the nest's centre on the surface, topped back up every
/// `refill=` (400) frames into slots that hold air or larder -- a slot an ant
/// or anything else stands in is left alone, as the bed learned to
/// (`refill_skipped`). Unless `onlyfood=off`, the larder is the only food in
/// the world (the bed's `Diet::isolate`): every other material's food value
/// and `worth_in_aux` are cleared, so the colony cannot live on its dead.
/// Meant with `hungry`: a fed box has no hunger, and no reason to forage.
/// **Run it with `PIXEL_PHYSICS_BUD_SITE=nest`**, as the bed does: without
/// it the colony buds new founders at the pile, and over 24,000 frames 20
/// founders became 777-1,251 ants (main a25c28f, seeds 1-2). With it, the
/// same runs ended at 30-40.
struct FoodPile {
    x: i32,
    top: i32,
    n: i32,
    refill: u64,
    larder: MaterialId,
    /// Cells introduced, the first placing and every refill.
    placed: u64,
    /// Slots a refill found holding something else, and left.
    skipped: u64,
}

impl FoodPile {
    fn from_args(world: &mut World, b: &Box2) -> Option<Self> {
        let n: i32 = arg("food").unwrap_or(0);
        if n <= 0 {
            return None;
        }
        let gap: i32 = arg("gap").unwrap_or(90);
        let x = b.w / 2 + gap;
        assert!(x + 6 < b.w - 1, "food: the pile at x {x} (gap {gap}) does not fit a box {} wide; widen w=", b.w);
        let name: String = arg("larder").unwrap_or_else(|| "fruit".to_string());
        let larder = world.materials.id_of(&name).unwrap_or_else(|| panic!("larder material {name:?} is not compiled in"));
        if arg::<String>("onlyfood").as_deref() != Some("off") {
            assert_ne!(Some(larder), world.materials.id_of("corpse"), "larder=corpse defeats the isolation: the colony would eat its dead");
            let keep = world.materials.get(larder).food_energy;
            assert!(keep > 0.0, "larder {name:?} carries no food_energy");
            for id in (0..world.materials.len() as u16).map(MaterialId) {
                let m = world.materials.get_mut(id);
                m.food_energy = 0.0;
                m.worth_in_aux = false;
            }
            world.materials.get_mut(larder).food_energy = keep;
        }
        let mut pile = FoodPile { x, top: b.surface - 1, n, refill: arg("refill").unwrap_or(400), larder, placed: 0, skipped: 0 };
        pile.place(world);
        Some(pile)
    }

    fn place(&mut self, world: &mut World) {
        for i in 0..self.n {
            let (fx, fy) = (self.x + (i % 12) - 6, self.top - i / 12);
            let m = world.get(fx, fy).material;
            if m == self.larder {
                world.set(fx, fy, Cell::new(self.larder, 0));
                continue;
            }
            if m != material::EMPTY {
                self.skipped += 1;
                continue;
            }
            self.placed += 1;
            world.set(fx, fy, Cell::new(self.larder, 0));
        }
    }

    /// Larder standing in the pile's slots now.
    fn standing(&self, world: &World) -> u64 {
        (0..self.n).filter(|i| world.get(self.x + (i % 12) - 6, self.top - i / 12).material == self.larder).count() as u64
    }
}

/// **Every ant topped up to its start energy each frame** -- the box's
/// default since 2026-09-29 (`hungry` turns it off) -- as though the colony
/// ate from a store, booked to the colony's ledger as granted.
///
/// Built 2026-09-29 by tracing the 200-ant pile the owner saw on the mouth
/// (`PILE`): the box sets `start_energy` to `energy=` and puts no food in it,
/// so **every ant is below its start energy from its first tick** and the
/// engine reads the whole colony as hungry for the whole run. Hungry, an
/// empty ant scouts (`scout_w` 0.5 at frame 6,000 on the mound, 1.3 by
/// 12,000), and a nest worker is pulled home only while fed (`home_pull`,
/// `chooser_step`'s way out), so 38-43 of the 50 nest workers stood above
/// ground at frame 6,000 at 200 ants. The box asks what a colony digs; this
/// asks it of a colony that is not starving. There is still no food on the
/// ground, so `FoodAdjacent` stays 0 and the dig is still the nest
/// mechanism alone; held at `start_energy` (1,000 on the lane's runs), an
/// ant sits under the ~1,040 J budding floor, so the count stays fixed.
fn feed(world: &mut World) {
    use pixel_physics::sim::world::Account;
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        let Some(start) = world.species.get(st.species).creature.as_ref().map(|c| c.start_energy) else { continue };
        let short = start - st.energy;
        if short <= 0.0 {
            continue;
        }
        // `set_organism_energy` books nothing, so the grant is booked here
        // and the colony's ledger still closes across the top-up.
        let colony = world.colony_of(id);
        world.set_organism_energy(id, start);
        world.book(colony, Account::Granted, f64::from(short));
    }
}

/// What a packed cell was just before it was packed, for `gridout=`'s
/// `PACKED_FROM` block: `0` packed before the trace began (the founding
/// cut's lining), `1` undug soil below the old ground line (a wall), `2`
/// soil standing in a cell dug since frame 0 (a hole that refilled and was
/// then tamped), `3` anything above the old ground line (a heap), `4` other.
const PACKED_FROM: [&str; 5] = ["before the trace", "undug soil (a wall)", "soil back in a dug cell (a refill)", "above the old ground line (a heap)", "other"];

/// One `PACK` line: every packed cell standing now, by what it was just
/// before it was packed ([`PACKED_FROM`]).
fn pack_census(world: &World, funnel: &NestFunnel, frame: u64) -> String {
    let packed = world.materials.id_of("packedsoil");
    let mut n = [0u32; 5];
    for (i, &from) in funnel.packed_from.iter().enumerate() {
        let (w, _) = world.bounds().map_or((0, 0), |r| (r.max_x - r.min_x + 1, r.max_y - r.min_y + 1));
        if w <= 0 {
            break;
        }
        let (x, y) = ((i as i32) % w, (i as i32) / w);
        if Some(world.get(x, y).material) == packed {
            n[usize::from(from).min(4)] += 1;
        }
    }
    let total: u32 = n.iter().sum();
    let parts: Vec<String> = PACKED_FROM.iter().zip(n).map(|(name, c)| format!("{name} {c}")).collect();
    format!("PACK frame={frame} packed cells standing {total}, by what they were: {}", parts.join(", "))
}

/// **`gridout=PATH`: the whole box at every stop, as text**, for questions
/// the fixed census lines were not written to answer.
///
/// Built 2026-09-29 for the owner, on the tinted Q3 sheet: *"the tamped
/// tunnel is not just walls around a tunnel or chamber. You have chambers
/// fully enclosed by tamped soil and big blocks of tamped soil."* How thick
/// the tamped ground is, which voids are sealed off from the mouth, and
/// what each packed cell was before it was tamped are all questions about
/// shape, so the shape is written out and read in a script. Per stop:
/// `GRID` (one character a cell: `.` empty, `s` soil, `P` packed, `o`
/// spoil, `a` a live animal, `#` stone, `c` corpse, `n` nest, `?` other),
/// `DUG` (cells cut since frame 0), `PACKED_FROM` ([`PACKED_FROM`]'s index,
/// `-` where the cell is not packed) and `PACKED_FRAME` (the frame it was
/// packed, `-1` if before the trace).
fn write_grid(w: &mut std::io::BufWriter<std::fs::File>, world: &World, b: &Box2, funnel: &NestFunnel, frame: u64) {
    use std::io::Write;
    let id = |n: &str| world.materials.id_of(n);
    let (soil, packed, spoil, corpse, nest) = (id("soil"), id("packedsoil"), id("spoil"), id("corpse"), id("nest"));
    let _ = writeln!(w, "GRID frame={frame} w={} h={} surface={} floor={}", b.w, b.h, b.surface, b.floor);
    for y in 0..b.h {
        let row: String = (0..b.w)
            .map(|x| {
                let c = world.get(x, y);
                let m = Some(c.material);
                if c.material == material::EMPTY {
                    '.'
                } else if c.organism_id() != 0 && world.materials.kind(c.material) == MaterialKind::Creature {
                    'a'
                } else if c.material == material::STONE {
                    '#'
                } else if m == soil {
                    's'
                } else if m == packed {
                    'P'
                } else if m == spoil {
                    'o'
                } else if m == corpse {
                    'c'
                } else if m == nest {
                    'n'
                } else {
                    '?'
                }
            })
            .collect();
        let _ = writeln!(w, "{row}");
    }
    let at = |x: i32, y: i32| (y * b.w + x) as usize;
    let _ = writeln!(w, "DUG");
    for y in 0..b.h {
        let row: String = (0..b.w).map(|x| if funnel.dug.get(at(x, y)).copied().unwrap_or(false) { '1' } else { '0' }).collect();
        let _ = writeln!(w, "{row}");
    }
    let _ = writeln!(w, "PACKED_FROM");
    for y in 0..b.h {
        let row: String = (0..b.w)
            .map(|x| {
                if Some(world.get(x, y).material) != packed {
                    '-'
                } else {
                    char::from(b'0' + funnel.packed_from.get(at(x, y)).copied().unwrap_or(0))
                }
            })
            .collect();
        let _ = writeln!(w, "{row}");
    }
    let _ = writeln!(w, "PACKED_FRAME");
    for y in 0..b.h {
        let row: Vec<String> = (0..b.w)
            .map(|x| match funnel.packed_frame.get(at(x, y)).copied().unwrap_or(u64::MAX) {
                u64::MAX => "-1".to_string(),
                f => f.to_string(),
            })
            .collect();
        let _ = writeln!(w, "{}", row.join(" "));
    }
    let _ = w.flush();
}

/// How far either side of the site's column the mound over the mouth is
/// read, for [`pile_census`]: the carry's drop column is the door's half
/// width, one, and a Gamma(2, 2) draw (median about 3), so twelve holds
/// nearly every pellet and every animal standing on them.
const PILE_REACH: i32 = 12;

const PILE_PLACES: [&str; 4] = ["on the mound over the mouth", "in the mouth", "underground", "out on the surface"];

const PILE_CSV_HEADER: &str = "frame,id,hx,hy,place,worker,holding,home_now,energy_j,frames_still,row_age,at_nest,crowding,stillness,energy_in,outcome,moved,p_move,drive,scout_w,scout_home,home_cos,anchor_x,anchor_y";

/// **Who stands where, and what each was deciding** -- one `PILE` line per
/// stop, and with `antscsv=PATH` one row per live animal.
///
/// Built 2026-09-29 for the owner, looking at the 200-ant sheets in the lab's
/// colours: *"It is just a huge pile of ants at the entrance and they totally
/// fill the nest."* A picture shows the pile; it cannot say whether the
/// animals in it are queued to get in, standing at home, or waiting to get
/// out -- three different fixes. So every live animal is booked by where its
/// head is ([`PILE_PLACES`]: the mound is above the old ground line within
/// [`PILE_REACH`] columns of the site) and, from its own last decision row,
/// by what it read and did: whether it stood at home (the shipped home test
/// replayed on the head -- nest material in the eight cells round it, or for
/// a nest worker the founding cut, `nest_within_reach`), how long its head
/// has not moved, and whether its last move roll stepped.
fn pile_census(world: &World, b: &Box2, trips: &TripLog, frame: u64, csv: Option<&mut std::io::BufWriter<std::fs::File>>) -> String {
    use pixel_physics::sim::creature::DECISION_OUTCOME_NAMES;
    use std::io::Write;
    let nest = world.materials.id_of("nest");
    let site = world.nest_sites.first().copied();
    let cut = site.and_then(|s| s.shaft);
    // Per place: animals, nest workers, holding a pellet, at home, still for
    // 60+ frames (ten decisions), energy summed (joules).
    let mut n = [0u32; 4];
    let mut workers = [0u32; 4];
    let mut holding = [0u32; 4];
    let mut home = [0u32; 4];
    let mut still = [0u32; 4];
    let mut energy = [0f64; 4];
    let mut rows = Vec::new();
    for (&id, a) in &TripLog::ants(world) {
        let Some(st) = world.organism(id) else { continue };
        let (hx, hy) = a.head;
        let place = if cut.is_some_and(|c| c.touches_mouth(hx, hy)) {
            1
        } else if hy >= b.surface {
            2
        } else if site.is_some_and(|s| (hx - s.x).abs() <= PILE_REACH) {
            0
        } else {
            3
        };
        let worker = st.nest_bound_until == u64::MAX;
        let home_now = if worker && cut.is_some_and(|c| c.touches(hx, hy)) {
            true
        } else {
            (-1..=1).any(|dy| (-1..=1).any(|dx| (dx, dy) != (0, 0) && Some(world.get(hx + dx, hy + dy).material) == nest))
        };
        let frames_still = frame.saturating_sub(trips.last_moved.get(&id).copied().unwrap_or(0));
        n[place] += 1;
        workers[place] += u32::from(worker);
        holding[place] += u32::from(a.holding);
        home[place] += u32::from(home_now);
        still[place] += u32::from(frames_still >= 60);
        energy[place] += f64::from(st.energy);
        if csv.is_some() {
            let r = trips.last_row.get(&id);
            let f = |v: Option<f32>| v.map_or(String::new(), |v| format!("{v:.3}"));
            rows.push(format!(
                "{frame},{id},{hx},{hy},{place},{},{},{},{:.1},{frames_still},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                u8::from(worker),
                u8::from(a.holding),
                u8::from(home_now),
                st.energy,
                r.map_or(String::new(), |r| frame.saturating_sub(r.frame).to_string()),
                f(r.map(|r| r.at_nest)),
                f(r.map(|r| r.crowding)),
                f(r.map(|r| r.stillness)),
                f(r.map(|r| r.energy)),
                r.map_or("", |r| DECISION_OUTCOME_NAMES[r.outcome as usize]),
                r.map_or(String::new(), |r| u8::from(r.moved).to_string()),
                f(r.map(|r| r.p_move)),
                f(r.map(|r| r.drive)),
                f(r.map(|r| r.scout_w)),
                r.map_or(String::new(), |r| u8::from(r.scout_home).to_string()),
                f(r.map(|r| r.home_cos)),
                r.map_or(String::new(), |r| r.anchor.0.to_string()),
                r.map_or(String::new(), |r| r.anchor.1.to_string()),
            ));
        }
    }
    if let Some(w) = csv {
        for row in rows {
            let _ = writeln!(w, "{row}");
        }
        let _ = w.flush();
    }
    let live: u32 = n.iter().sum();
    let parts: Vec<String> = (0..4)
        .map(|p| {
            let e = if n[p] > 0 { energy[p] / f64::from(n[p]) } else { 0.0 };
            format!(
                "{} {} (nest workers {}, holding a pellet {}, at home {}, still 60+ frames {}, energy {e:.0})",
                PILE_PLACES[p], n[p], workers[p], holding[p], home[p], still[p]
            )
        })
        .collect();
    format!("PILE frame={frame} live {live}: {}", parts.join(" | "))
}

/// **What stands in the holes the colony dug, and on their walls**, one
/// `FILL` line per stop: every cell below the old ground line that was dug
/// since frame 0, by what it holds now, and the tamped lining standing on
/// ground that was never dug.
///
/// Built 2026-09-29 for the owner's question about a picture: *"What is the
/// darker brown in the nest, the borders/edges of the tunnels? ... it looks
/// like the tunnels have filled in with darker brown material."* Lining,
/// pellets and the dead all draw as a brown near soil's, so the picture could
/// not answer it and [`tint`] with this line can. A hole counts once however
/// often it was re-cut; the denominator is the dug mask, not the dig count.
fn fill_census(world: &World, b: &Box2, dug: &[bool], frame: u64) -> String {
    let id = |n: &str| world.materials.id_of(n);
    let (spoil, packed, soil, corpse) = (id("spoil"), id("packedsoil"), id("soil"), id("corpse"));
    let (mut open, mut ant, mut dead, mut pellet, mut lined, mut ground, mut other) = (0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
    let mut lining = 0u32;
    for y in b.surface..b.floor {
        for x in 1..b.w - 1 {
            let cell = world.get(x, y);
            let was_dug = dug.get((y * b.w + x) as usize).copied().unwrap_or(false);
            if !was_dug {
                lining += u32::from(Some(cell.material) == packed);
                continue;
            }
            let slot = if cell.material == material::EMPTY {
                &mut open
            } else if cell.organism_id() != 0 && world.materials.kind(cell.material) == MaterialKind::Creature {
                &mut ant
            } else if Some(cell.material) == corpse {
                &mut dead
            } else if Some(cell.material) == spoil {
                &mut pellet
            } else if Some(cell.material) == packed {
                &mut lined
            } else if Some(cell.material) == soil {
                &mut ground
            } else {
                &mut other
            };
            *slot += 1;
        }
    }
    let total = open + ant + dead + pellet + lined + ground + other;
    format!(
        "FILL frame={frame} cells dug below the old ground line since frame 0: {total}, holding now: open {open}, a live ant {ant}, a corpse {dead}, a pellet {pellet}, lining {lined}, soil {ground}, other {other} | tamped lining on undug walls {lining}"
    )
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
/// - a `corpse` (an animal that died, until it rots to soil): **green**
/// - `soil` below the old ground line in a cell the colony dug since frame 0
///   (a hole refilled -- a rotted corpse, a slumped pellet, ground that
///   fell): **violet**. Needs the funnel's dug mask; `nofunnel` leaves it
///   as drawn.
///
/// Stone, sky, water, dug void and undisturbed soil are left as the
/// renderer drew them. The last two classes were added 2026-09-29 for the
/// owner's *"what is the darker brown in the nest, the borders/edges of the
/// tunnels? ... it looks like the tunnels have filled in with darker brown
/// material"* -- a question about a colony that had starved by the stop in
/// question, so its dead had to be told apart from its spoil.
fn tint(world: &World, b: &Box2, dug: &[bool], buf: &mut [u8]) {
    let id = |n: &str| world.materials.id_of(n);
    let (spoil, packed, nest, soil, corpse) = (id("spoil"), id("packedsoil"), id("nest"), id("soil"), id("corpse"));
    for y in 0..b.h {
        for x in 0..b.w {
            let cell = world.get(x, y);
            let was_dug = dug.get((y * b.w + x) as usize).copied().unwrap_or(false);
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
            } else if Some(cell.material) == corpse {
                Some([40, 220, 40])
            } else if Some(cell.material) == soil && y < b.surface {
                Some([255, 230, 0])
            } else if Some(cell.material) == soil && was_dug {
                Some([170, 120, 255])
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
