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
                world.set(x, y, Cell::new(soil_id, 0).with_attached(true).with_aux(wet));
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
fn census(world: &World, b: &Box2) -> (usize, usize) {
    let (mut roofed, mut open) = (0, 0);
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
            } else if cell.material == material::EMPTY {
                if covered {
                    roofed += 1;
                } else {
                    open += 1;
                }
            }
        }
    }
    (roofed, open)
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
struct Trickle {
    colony: Option<u32>,
    placed: usize,
    target: usize,
    /// How many to try per frame. The cap is space, not this.
    rate: usize,
    cursor: i32,
}

impl Trickle {
    fn new(target: usize, rate: usize) -> Self {
        Trickle { colony: None, placed: 0, target, rate, cursor: 0 }
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
            let x = cx - half + self.cursor;
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
    let scale: u32 = arg("scale").unwrap_or(3);

    let wet: u16 = arg("wet").unwrap_or(material::SOIL_FIELD_CAPACITY);
    let mut world = build_wet(&b, wet);

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
    world.paint_nest_patch(b.w / 2, b.surface - 1);
    let mut trickle = Trickle::new(ants as usize, arg("rate").unwrap_or(4));

    // The endowment horizon, printed rather than assumed -- a run past it is
    // measuring starvation, not digging.
    let id = world.species.id_of("ant").expect("ant ships");
    let def = world.species.get(id).creature.as_ref().expect("ant is a creature");
    let per_tick = def.idle_cost_per_cell * def.body.len() as f32;
    let horizon = (def.start_energy / per_tick) as u64 * def.tick_interval.max(1);

    println!(
        "digbox: {}x{} box, soil {} rows ({}..{}), colony of {} trickled in at {}/frame, seed-free scene",
        b.w, b.h, soil, b.surface, b.floor, ants, trickle.rate
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
    println!();
    println!(
        "  soil wetness {wet} of {} saturated ({}); the column the dig decision actually reads is `wet grad`",
        material::SOIL_SATURATED,
        if wet >= material::SOIL_FIELD_CAPACITY { "at or above field capacity" } else { "below field capacity" }
    );
    println!();
    println!(
        "{:>8}  {:>5}  {:>7}  {:>9}  {:>8}  {:>7}  {:>7}  {:>8}  {:>9}  {:>9}",
        "frame", "ants", "digs", "dig rolls", "per roll", "roofed", "open", "charge", "wet grad", "its cost"
    );

    let particles = ParticleSystem::default();
    let mut renderer = Renderer::new();
    let mut blasts = Blasts::default();
    let _ = &mut blasts;
    let stops: Vec<u64> = (0..=frames).step_by((frames / 6).max(1) as usize).collect();
    let mut shots: Vec<Vec<u8>> = Vec::new();

    for f in 0..=frames {
        if f > 0 {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
            world.step_pheromones();
        }
        trickle.step(&mut world, &b);
        if stops.contains(&f) {
            let (roofed, open) = census(&world, &b);
            let (n, e) = charge(&world);
            let st = world.creature_stats;
            let per_roll = if st.dig_rolls > 0 { st.digs as f64 / st.dig_rolls as f64 } else { 0.0 };
            let (mg_mean, mg_med) = moisture_seen(&world);
            // What that gradient is worth to the decision, in the dig
            // probability's own units -- a number nobody can read off the
            // gradient itself, because it lands inside a squash.
            let cost = 0.55 * mg_mean;
            println!(
                "{f:>8}  {n:>5}  {:>7}  {:>9}  {per_roll:>8.3}  {roofed:>7}  {open:>7}  {e:>7.1}  {:>9.3}  {:>9.3}",
                st.digs, st.dig_rolls, mg_med, cost
            );
            if out.is_some() {
                let (vw, vh) = (b.w as u32, b.h as u32);
                let mut buf = vec![0u8; (vw * vh * 4) as usize];
                let touched = world.take_touched_chunks();
                renderer.draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
                shots.push(buf);
            }
        }
    }

    let st = world.creature_stats;
    let (roofed, open) = census(&world, &b);
    println!();
    println!(
        "SUMMARY digs={} rolls={} per_roll={:.3} roofed={roofed} open={open} at_nest_ticks={} spoil_dumped={}",
        st.digs,
        st.dig_rolls,
        if st.dig_rolls > 0 { st.digs as f64 / st.dig_rolls as f64 } else { 0.0 },
        st.at_nest_ticks,
        st.spoil_dumped
    );
    let bands = st.at_nest_crowding;
    let total: u64 = bands.iter().sum();
    if total > 0 {
        println!(
            "SUMMARY how packed it felt while at the nest, 0.0..1.0 in tenths: {bands:?}  (top tenth {:.1}% of at-nest ticks)",
            bands[9] as f64 * 100.0 / total as f64
        );
    }

    if let Some(path) = out {
        let (tw, th) = (b.w as u32, b.h as u32);
        let (sw, sh) = (tw * scale, th * shots.len() as u32 * scale);
        let mut sheet = vec![0u8; (sw * sh * 4) as usize];
        for (i, tile) in shots.iter().enumerate() {
            let y0 = i as u32 * th * scale;
            for y in 0..th {
                for ry in 0..scale {
                    let dst_row = ((y0 + y * scale + ry) * sw * 4) as usize;
                    for x in 0..tw {
                        let src = ((y * tw + x) * 4) as usize;
                        let px = &tile[src..src + 4];
                        for rx in 0..scale {
                            let dst = dst_row + ((x * scale + rx) * 4) as usize;
                            sheet[dst..dst + 4].copy_from_slice(px);
                        }
                    }
                }
            }
        }
        image::save_buffer(&path, &sheet, sw, sh, image::ColorType::Rgba8).expect("writing the sheet");
        println!("wrote {path} ({sw}x{sh}, {} stops top to bottom)", shots.len());
    }
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
    let (roofed, open) = census(&bare, b);
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
    let (roofed2, open2) = census(&carved, b);
    println!("  a hand-carved 10x3 chamber 10 rows down reads roofed {roofed2} open {open2} (must be 30 and 0)");
    assert_eq!((roofed2, open2), (30, 0), "the census must find a known chamber, and must call it roofed rather than open");

    // ...and a shaft cut to the surface must read OPEN, not roofed -- the
    // distinction the whole question rests on.
    let mut shaft = build(b);
    for y in b.surface..b.surface + 6 {
        shaft.set(cx, y, Cell::EMPTY);
    }
    let (roofed3, open3) = census(&shaft, b);
    println!("  a 6-deep shaft open to the sky reads roofed {roofed3} open {open3} (must be 0 and 6)");
    assert_eq!((roofed3, open3), (0, 6), "a hole open to the sky is not a room");

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
    let (roofed4, _) = census(&live, b);
    println!("  {founded} ants for 4,000 frames: {} digs, {} rolls, roofed {roofed4}", st.digs, st.dig_rolls);
    assert!(st.dig_rolls > 0, "not one dig was even attempted -- the colony is not thinking, so any null from this box is the harness");
    assert!(st.digs > 0, "digs attempted but none landed -- every roll hit air, rock or another ant, and this box cannot answer a digging question");
    println!("digbox selftest: PASS -- the box is empty when nobody digs, finds a known chamber, calls a shaft open, and its ants dig");
}
