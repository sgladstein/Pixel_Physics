//! **Does a tunnel dug in soil survive?** — the measurement behind the
//! evolution lab's digging question, and a direct check on a claim
//! `wiki/ants.md` makes today: *"Turn a colony loose on a soil bank and it
//! hollows it out, leaving the stone beneath untouched."*
//!
//! `Reports/evolution-lab-design-guide-2026-08-30.md` §2b records the owner's
//! decision to decline collapsing tunnels, and reads the cost of that as the
//! structural scheduler's 16%. It then says a repose angle is harmless —
//! *"a dug wall that slumps a little is available and free; a roof that falls
//! in is what was declined."* **That is the thing this harness tests**, and
//! it is testable because the two mechanisms are separable: the structural
//! scheduler is not what closes a hole in a powder. `update_powder`'s
//! straight-down rule is, and it runs in the CA sweep whether or not any
//! structural code is linked.
//!
//! Three arms, each a bed with the same excavation cut into it:
//!
//! | arm | bed |
//! |---|---|
//! | `soil` | the lab's own bed — `soil`, a `Powder` |
//! | `sand` | `sand`, the loosest shipped powder — the *negative* control, expected worse |
//! | `stone` | `stone`, a `Solid` — the **positive control** |
//! | `lined` | `soil`, with the excavation's wall worked into `packedsoil` — what an ant now leaves behind |
//! | `flooded` | `lined`, with the shaft filled with water — the wall wets from the inside |
//! | `watertable` | `lined`, dug into a bank already at `SOIL_SATURATED` — the wall wets from the outside |
//!
//! **The last three exist to keep the mechanic from being a binary.**
//! `CLAUDE.md`'s first law is that an outcome is a distribution, not a
//! switch, so a lining that could never fail would be the same defect as a
//! tunnel that always does. `packedsoil` reverts to `soil` above
//! `material::SOIL_FIELD_CAPACITY`, and these two arms are the wet halves
//! of that: one soaks the wall from the void, one from the bank.
//!
//! **Two columns per void, and the second one is why the wet arms are
//! readable at all.** `open` is *materially empty*, which a flooded shaft
//! is not — so on `flooded` the shaft reads 0% open on frame 0, before a
//! single tick, purely because there is water standing in it. That is
//! `CLAUDE.md`'s "ask what your number counts when nothing is wrong"
//! exactly. `caved` counts the void's cells that now hold **ground** (a
//! `Powder` or a `Solid`), which is the thing actually being claimed, and
//! it stays 0 for water. Read `caved`, not `open`, on any arm with liquid
//! in it.
//!
//! **The positive control is the point.** `CLAUDE.md`: *a null looks the same
//! whether the mechanism is quiet or the probe never reached it*, and *run the
//! positive control — construct the case whose answer you know is non-zero and
//! check the instrument reports it.* A tunnel in stone must read 100%
//! surviving at every frame, or this harness is measuring its own scene
//! construction and not the physics.
//!
//! The excavation is what an ant would actually dig, not an abstract cavity:
//! a vertical shaft from the surface, a horizontal gallery off it, and a
//! chamber at the end. Each is censused separately, because they fail for
//! different reasons and a single pooled number would hide that — a shaft is
//! a vertical face (the repose rule), a gallery has a roof (the straight-down
//! rule), and a chamber is both with a longer span.
//!
//! # The `colony` arm — the one that is not a hand-carved cavity
//!
//! Every arm above answers *does a lined tunnel stand*. It cannot answer *do
//! ants produce one*, and those need separating: a hand-written lining is a
//! claim about `update_powder`, and a colony is a claim about
//! `creature::line_burrow` and about whether digging reaches soil at all.
//! `arms=colony` puts 55 ants on a soil bank over stone and censuses the
//! **standing void inside the bank** — the quantity a player would call a
//! nest — with `digs` and `packed` printed beside it every time, because a
//! bank with no holes in it and a colony that never dug are the same picture
//! (`CLAUDE.md`: "did it fire at all" needs a counter, not a picture).
//!
//! **Its baseline is the same binary with the lining switched off**, which is
//! what `PIXEL_PHYSICS_BURROW_LINING=off` is for. A standing quantity has no
//! baseline of its own; run both and read the pair:
//!
//! ```text
//! cargo run --release --example burrow_probe -- arms=colony seeds=4
//! PIXEL_PHYSICS_BURROW_LINING=off cargo run --release --example burrow_probe -- arms=colony seeds=4
//! ```
//!
//! ```text
//! cargo run --release --example burrow_probe
//! cargo run --release --example burrow_probe -- frames=3600 seeds=4
//! cargo run --release --example burrow_probe -- arms=soil width=256
//! ```

mod common;

use common::PlantScene;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::weather::Pin;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::{frame, material, player, Cell, World};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses"))
    })
}

/// One dug void, censused on its own. `cells` is what was carved; the census
/// counts how many of them are still materially empty.
struct Void {
    name: &'static str,
    cells: Vec<(i32, i32)>,
}

impl Void {
    /// **Raw material equality, not `Cell::is_empty`.** `is_empty` is
    /// managed-aware and answers "is this position available", which is a
    /// different question from "is there material here" (`CLAUDE.md`'s
    /// gotcha). A tunnel refilled with soil must not read as empty.
    fn open(&self, world: &World) -> usize {
        self.cells.iter().filter(|(x, y)| world.get(*x, *y).material == material::EMPTY).count()
    }

    /// How many of the carved cells have **ground** standing in them --
    /// anything the sweep treats as a `Powder` or a `Solid`.
    ///
    /// `open` alone cannot carry the wet arms. A shaft full of water is not
    /// materially empty, so it reads 0% open on frame 0 with nothing having
    /// happened; a shaft whose roof fell in reads 0% open too, and those are
    /// the opposite finding. This column separates them, and it is the one
    /// to quote whenever there is liquid in the scene.
    fn caved(&self, world: &World) -> usize {
        self.cells
            .iter()
            .filter(|(x, y)| {
                matches!(
                    world.materials.kind(world.get(*x, *y).material),
                    MaterialKind::Powder | MaterialKind::Solid
                )
            })
            .count()
    }
}


/// **Is the standing void one gallery, or a scatter of bites?** — a
/// connected-component pass over the empty cells in the bank footprint.
///
/// `roofed` says *how much* enclosed void there is and cannot distinguish
/// 130 cells of tunnel from 130 separate nibbles at an open face. Those are
/// the same number and opposite findings — `CLAUDE.md`'s *a mean over events
/// is not a mean over the thing you care about*, one level up: a gallery is a
/// long connected run, quarrying is a scatter of singletons.
///
/// **Eight-connected, because the digger is.** `creature.rs`'s dig steps the
/// cell in the ant's `heading`, and headings are the eight compass
/// directions, so two diagonally-touching void cells are one passage to the
/// animal that made them. `CLAUDE.md`: *a traversal must use the same
/// neighbourhood the writer used* — a four-neighbour pass here would report a
/// diagonal tunnel as a row of singletons and manufacture the very finding
/// this measurement exists to test for.
///
/// Each component carries its **roofed** count as well as its size, because
/// the two answer different halves. The quarried corner of a bank is one
/// enormous component with almost no roof over it; a gallery is a smaller
/// component that is nearly all roof. Reading size alone would score the
/// erosion case as the best tunneller in the run.
///
/// Returned largest-first.
fn void_components(
    world: &World,
    (x0, x1): (i32, i32),
    (y0, y1): (i32, i32),
) -> Vec<Component> {
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let idx = |x: i32, y: i32| (y - y0) as usize * w + (x - x0) as usize;

    // Roofed is a per-column prefix over the *whole* column, not just the
    // footprint: a cell is roofed when ground stands above it anywhere,
    // including the bank's own untouched cap above `y0`.
    let mut empty = vec![false; w * h];
    let mut roofed = vec![false; w * h];
    for x in x0..x1 {
        let mut above = 0usize;
        for y in 0..y1 {
            let m = world.get(x, y).material;
            if y >= y0 && m == material::EMPTY {
                empty[idx(x, y)] = true;
                roofed[idx(x, y)] = above > 0;
            }
            if matches!(world.materials.kind(m), MaterialKind::Powder | MaterialKind::Solid) {
                above += 1;
            }
        }
    }

    let mut seen = vec![false; w * h];
    let mut out = Vec::new();
    let mut stack: Vec<(i32, i32)> = Vec::new();
    for sy in y0..y1 {
        for sx in x0..x1 {
            if !empty[idx(sx, sy)] || seen[idx(sx, sy)] {
                continue;
            }
            seen[idx(sx, sy)] = true;
            stack.push((sx, sy));
            let mut comp = Component::default();
            while let Some((cx, cy)) = stack.pop() {
                comp.cells += 1;
                if roofed[idx(cx, cy)] {
                    comp.roofed += 1;
                }
                for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx < x0 || nx >= x1 || ny < y0 || ny >= y1 {
                        continue;
                    }
                    if empty[idx(nx, ny)] && !seen[idx(nx, ny)] {
                        seen[idx(nx, ny)] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            out.push(comp);
        }
    }
    out.sort_unstable_by(|a, b| b.cells.cmp(&a.cells).then(b.roofed.cmp(&a.roofed)));
    out
}

/// One connected run of standing void: how big it is, and how much of it has
/// ground overhead.
#[derive(Default, Clone, Copy)]
struct Component {
    cells: usize,
    roofed: usize,
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(1_800);
    let width: i32 = arg("width").unwrap_or(256);
    let height: i32 = arg("height").unwrap_or(320);
    // **The bed has to fit in the world, and the builder will not say so.**
    // `PlantScene` writes its stone floor at `ground_y + soil + STONE_DEPTH`
    // and `World::set` silently drops anything past the bottom edge, so a bed
    // deeper than the world produces a bed with no floor *and* an excavation
    // carved into rows that do not exist -- which reads as "the tunnel closed
    // instantly" at frame 0, before a single tick has run. That is
    // `CLAUDE.md`'s scene-error trap exactly, and it is why the frame-0 row
    // below is asserted rather than merely printed.
    let ground: i32 = arg("ground").unwrap_or(60);
    let soil: i32 = arg("soil").unwrap_or(200);
    let seeds: u64 = arg("seeds").unwrap_or(1);
    let want: String =
        arg("arms").unwrap_or_else(|| "soil,sand,stone,lined,flooded,watertable".to_string());
    let ants: i32 = arg("ants").unwrap_or(55);
    let colony_frames: u64 = arg("colonyframes").unwrap_or(8_000);

    let png: Option<String> = arg("png");
    if want.split(',').any(|w| w == "colony") {
        colony_arm(seeds, ants, colony_frames, png.as_deref());
    }

    println!(
        "burrow_probe: frames={frames} width={width}x{height} soil={soil} seeds={seeds} arms={want}"
    );
    println!(
        "\nan excavation is cut into each bed and censused as it fills in. \
         `stone` is the positive control and must read 100% at every frame."
    );

    for arm in
        ["soil", "sand", "stone", "lined", "flooded", "watertable"].iter().filter(|a| want.split(',').any(|w| &w == *a))
    {
        println!("\n=== arm {arm} ===");
        println!(
            "{:>6}  {:>7}  {:>24}  {:>24}  {:>24}",
            "seed", "frame", "shaft open/caved", "gallery open/caved", "chamber open/caved"
        );

        for seed in 1..=seeds {
            let mut scene = PlantScene { species: "herb".to_string(), ..PlantScene::default() };
            scene.width = width;
            scene.height = height;
            scene.ground_y = ground;
            scene.soil_depth = soil;
            scene.trees = 0;
            scene.seed = Some(seed);
            let mut world = scene.build();
            // No weather, and a held light: the lab's own operating point.
            // Rain into an open shaft is a real hazard and a different
            // experiment; it must not ride along inside this one.
            world.set_weather_pin(Pin::Clear);

            // **Repaint the bed in this arm's material.** The scene builder
            // only makes soil, so `sand` and `stone` are written over the
            // soil rows it produced -- same geometry, same stone floor
            // underneath, one material changed. That is the A/B `CLAUDE.md`
            // asks for: two arms differing in one thing.
            //
            // The three wet/lined arms keep the soil bed and differ only
            // after the carve, below -- so `lined` against `soil` is an A/B
            // in one thing, which is what makes the comparison mean
            // anything (`CLAUDE.md`: an A/B whose arms differ in two things
            // carries half its effect in the thing that was not under
            // test).
            if *arm == "sand" || *arm == "stone" {
                let id = world
                    .materials
                    .id_of(arm)
                    .unwrap_or_else(|| panic!("{arm} is a compiled-in material"));
                for x in 0..width {
                    for y in ground..(ground + soil) {
                        world.set(x, y, Cell::new(id, 0));
                    }
                }
            }

            // The excavation. A shaft down from the surface, a gallery
            // running off its foot, and a chamber at the gallery's end --
            // 3 cells tall, which is what an ant fits through.
            let shaft_x = width / 3;
            let shaft_bottom = ground + soil / 2;
            let gallery_y = shaft_bottom;
            let gallery_end = shaft_x + 60;

            let mut shaft = Void { name: "shaft", cells: Vec::new() };
            for y in ground..shaft_bottom {
                for x in shaft_x..(shaft_x + 3) {
                    shaft.cells.push((x, y));
                }
            }
            let mut gallery = Void { name: "gallery", cells: Vec::new() };
            for x in shaft_x..gallery_end {
                for y in gallery_y..(gallery_y + 3) {
                    gallery.cells.push((x, y));
                }
            }
            let mut chamber = Void { name: "chamber", cells: Vec::new() };
            for x in gallery_end..(gallery_end + 16) {
                for y in (gallery_y - 4)..(gallery_y + 4) {
                    chamber.cells.push((x, y));
                }
            }

            let voids = [&shaft, &gallery, &chamber];
            for v in voids {
                for (x, y) in &v.cells {
                    world.set(*x, *y, Cell::EMPTY);
                }
            }
            let carved: Vec<usize> = voids.iter().map(|v| v.cells.len()).collect();

            // **The lining, and it is the same set of cells an ant produces.**
            // `creature::line_burrow` packs the 8-neighbourhood of every cell
            // it empties, so the union of those rings over a swept-out tunnel
            // is exactly the shell around the excavation -- which is what this
            // writes in one pass. Doing it here rather than by running ants
            // separates the two claims: this arm asks *does a lined tunnel
            // stand*, and the colony run in `ascii`'s excavation scene asks
            // *do ants produce one*. A single harness answering both could not
            // tell a dead lining from ants that never dug.
            //
            // Routed through `Material::packs_into` rather than
            // `id_of("packedsoil")` so this measures the shipped rule: if the
            // material were renamed or the field dropped, this arm would come
            // back identical to `soil` instead of silently lining itself by a
            // path the game does not use.
            let lined_arm = matches!(*arm, "lined" | "flooded" | "watertable");
            let mut lining = 0usize;
            if lined_arm {
                let carved_set: std::collections::HashSet<(i32, i32)> =
                    voids.iter().flat_map(|v| v.cells.iter().copied()).collect();
                let mut wall: Vec<(i32, i32)> = Vec::new();
                for &(cx, cy) in &carved_set {
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let n = (cx + dx, cy + dy);
                            if !carved_set.contains(&n) {
                                wall.push(n);
                            }
                        }
                    }
                }
                wall.sort_unstable();
                wall.dedup();
                for (wx, wy) in wall {
                    let cell = world.get(wx, wy);
                    let Some(packed) = world.materials.get(cell.material).packs_into else {
                        continue;
                    };
                    let mut lined = cell;
                    lined.material = packed;
                    world.set(wx, wy, lined);
                    lining += 1;
                }
            }

            // **Wet the wall from the outside**: the whole bank at
            // `SOIL_SATURATED`, which is a gallery driven below the water
            // table. Every packed cell is then over `SOIL_FIELD_CAPACITY` and
            // `slumps_into` should take the lining apart on the first sweep.
            if *arm == "watertable" {
                for x in 0..width {
                    for y in ground..(ground + soil) {
                        let cell = world.get(x, y);
                        if world.materials.get(cell.material).water_capacity > 0 {
                            world.set(x, y, cell.with_aux(material::SOIL_SATURATED));
                        }
                    }
                }
            }

            // **Wet the wall from the inside**: standing water in the shaft,
            // which drains down it and along the gallery, infiltrating the
            // lining as it goes. This is the arm whose `open` column is
            // uninformative -- water is not materially empty -- and the reason
            // `caved` exists.
            let mut poured = 0usize;
            if *arm == "flooded" {
                let water = world.materials.id_of("water").expect("water is compiled in");
                for (x, y) in &shaft.cells {
                    world.set(*x, *y, Cell::new(water, 0));
                    poured += 1;
                }
            }
            if lined_arm {
                println!("{:>6}  {:>7}  wall cells worked into packedsoil: {lining}{}", "", "-",
                    if poured > 0 { format!(", water poured into the shaft: {poured}") } else { String::new() });
            }

            let mut particles = ParticleSystem::default();
            let mut blasts = Blasts::default();
            let tuning = player::Tuning::default();

            let report = |world: &World, f: u64| {
                let cols: Vec<String> = voids
                    .iter()
                    .zip(&carved)
                    .map(|(v, n)| {
                        let open = v.open(world);
                        let caved = v.caved(world);
                        format!(
                            "{open:>4}/{n:<4}{:>6.1}%/{:>5.1}%",
                            100.0 * open as f64 / *n as f64,
                            100.0 * caved as f64 / *n as f64
                        )
                    })
                    .collect();
                println!("{seed:>6}  {f:>7}  {:>24}  {:>24}  {:>24}", cols[0], cols[1], cols[2]);
            };

            // **The scene check, as an assertion.** Every carved cell must be
            // open before any tick runs. If it is not, the excavation is not
            // where the harness thinks it is and every number below is about
            // the scene rather than about the physics.
            report(&world, 0);
            // **The flooded arm is exempt from the emptiness half and not
            // from the check**: its shaft is deliberately full of water, so
            // `open` is 0 there by construction. What must still hold on every
            // arm is that no *ground* is standing in the excavation before a
            // tick runs, which is the scene error this assertion exists to
            // catch, so `caved` is asserted for all arms and `open` for the
            // dry ones.
            for (v, n) in voids.iter().zip(&carved) {
                assert_eq!(v.caved(&world), 0, "{} had ground standing in it at frame 0", v.name);
                if *arm == "flooded" {
                    continue;
                }
                assert_eq!(
                    v.open(&world),
                    *n,
                    "{} was not fully carved at frame 0 -- the excavation is outside the bed \
                     (ground={ground} soil={soil} height={height}); every number after this \
                     would be a measurement of the scene",
                    v.name
                );
            }
            let marks = [1u64, 5, 30, 120, 600, frames];
            for f in 1..=frames {
                frame::step(
                    &mut world,
                    &mut particles,
                    &mut blasts,
                    player::PlayerInput::default(),
                    &tuning,
                );
                if marks.contains(&f) {
                    report(&world, f);
                }
            }
        }
    }
}

/// **Do real ants leave a standing tunnel?**
///
/// The bank, the floor and the founder count are `examples/ascii.rs`'s
/// excavation scene, deliberately: that scene is CI-gated and already
/// establishes that 55 ants chew soil at 0.8 and are stopped by stone, so
/// reusing its geometry means the only new claim here is what is *left*
/// afterwards. What it adds is the census that scene never had — standing
/// void inside the bank footprint — plus a seed loop, because outcomes here
/// are chaotic in the seed and one run is a sample from a wide distribution.
///
/// **Three numbers on every row and none of them is optional.** `void` is the
/// spatial claim. `digs` is the near-side counter — did the verb fire at all
/// — and `packed` is the far-side effect counter on the same call, which is
/// the pairing `CLAUDE.md` requires after a mining harness reported 200 cuts
/// that removed 0 cells. A renamed `packedsoil`, a dropped `packs_into`, or a
/// dig that only ever lands in stone all read as `packed 0` here and are
/// invisible in `digs`.
fn colony_arm(seeds: u64, ants: i32, frames: u64, png: Option<&str>) {
    use pixel_physics::render::Renderer;
    use pixel_physics::sim::chunk::Rect;
    use pixel_physics::sim::parallel;
    use pixel_physics::sim::particle::ParticleSystem;

    let lining_on = std::env::var("PIXEL_PHYSICS_BURROW_LINING").as_deref() != Ok("off");
    println!("\n=== arm colony ===  lining {}", if lining_on { "ON" } else { "OFF (ablated)" });
    println!("  55 ants on a soil bank over stone; `void` is standing empty cells inside the bank");
    println!(
        "  `void` is every empty cell in the bank footprint -- **erosion moves it**; \n           `roofed` is empty with ground standing above it, which erosion cannot produce. Read `roofed`."
    );
    println!(
        "  `comps`/`largest`/`ge8` are the connected-component split of that void (8-connected,\n           the neighbourhood the digger uses). `lgroof` is how much of the largest run has\n           ground overhead: a quarried face is one huge run with no roof, a gallery is a\n           smaller run that is nearly all roof."
    );
    println!(
        "{:>6}  {:>7}  {:>8}  {:>8}  {:>8}  {:>6}  {:>8}  {:>5}  {:>7}  {:>7}  {:>7}  {:>9}  {:>10}",
        "seed",
        "frame",
        "void",
        "roofed",
        "roofed3",
        "comps",
        "largest",
        "ge8",
        "lgroof",
        "digs",
        "packed",
        "soil",
        "packedsoil"
    );

    for seed in 1..=seeds {
        let (w, h) = (200i32, 120i32);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        world.set_weather_pin(Pin::Clear);
        // **Held at noon.** The day/night cycle is the largest visual signal
        // in the engine, and a contact sheet whose stops fall at 0/500/2,000/
        // 8,000 frames lands two of them after dark: the first sheet taken
        // here had a moon in it and the bank was unreadable. That is
        // `CLAUDE.md`'s designed-oscillator rule applied to a picture rather
        // than to a number -- the light must be divided out, or every stop is
        // its own phase plus the thing being looked at.
        world.set_sky_hold(Some(pixel_physics::sky::frame_for_daylight(1.0)));
        let soil_id = world.materials.id_of("soil").expect("soil");
        let packed_id = world.materials.id_of("packedsoil").expect("packedsoil");
        let nest_id = world.materials.id_of("nest").expect("nest");
        let floor = h - 8;
        let (bank_x0, bank_x1) = (40i32, 160i32);
        let (bank_y0, bank_y1) = (floor - 30, floor);

        for x in 0..w {
            for y in floor..h {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        for x in bank_x0..bank_x1 {
            for y in bank_y0..bank_y1 {
                world.set(x, y, Cell::new(soil_id, 0).with_attached(true));
            }
        }
        for x in 16..bank_x0 {
            world.set(x, floor, Cell::new(nest_id, 0).with_attached(true));
        }
        // Founder placement is seeded by shifting the row start, so the four
        // seeds are genuinely different colonies rather than one colony four
        // times -- `World::new` alone does not vary here the way `PlantScene`
        // does, and a "seed sweep" over identical worlds is the tidy,
        // meaningless result `CLAUDE.md` warns is the tell of an artifact.
        let off = (seed as i32 - 1) * 3;
        for i in 0..ants {
            world.plant_ant(20 + off + i % 10 * 2, floor - 1 - (i / 10));
        }

        // **`void` alone cannot say a nest happened, and the first version of
        // this arm shipped believing it could.** Measured 2026-08-30: with the
        // lining ablated, 610 digs left **788** standing empty cells inside the
        // bank footprint -- against 803 cells of soil gone from it. The two
        // numbers are the same number. Digging *destroys* its spoil
        // (`creature.rs`'s dig, "spoil is destroyed in v1"), so every cell an
        // ant removes lowers the bank by one cell somewhere, and the empty
        // rows that opens up at the **top** of the footprint are counted by a
        // rectangle census exactly as if they were a chamber. A colony that
        // eats a bank down from above and one that hollows it out score
        // identically, which is `CLAUDE.md`'s "ask what your number counts
        // when nothing is wrong" with the answer *it counts erosion*.
        //
        // `roofed` is the column that states the claim: an empty cell with
        // **ground standing above it**, which is the one thing lowering a
        // surface can never produce. `roofed3` requires three cells of it, so
        // a crumb bridging a pit does not read as a chamber. Neither can be
        // moved by erosion at all, which is what makes them the pair to read.
        let census = |world: &World| {
            let mut void = 0usize;
            let mut roofed = 0usize;
            let mut roofed3 = 0usize;
            let mut soil = 0usize;
            let mut packed = 0usize;
            for x in bank_x0..bank_x1 {
                // Walk the column downward carrying how much ground is
                // standing above the current row -- one pass, and it counts
                // the roof rather than merely detecting it.
                let mut above = 0usize;
                for y in 0..bank_y1 {
                    let m = world.get(x, y).material;
                    let ground = matches!(
                        world.materials.kind(m),
                        MaterialKind::Powder | MaterialKind::Solid
                    );
                    if y >= bank_y0 {
                        if m == material::EMPTY {
                            void += 1;
                            if above > 0 {
                                roofed += 1;
                            }
                            if above >= 3 {
                                roofed3 += 1;
                            }
                        } else if m == soil_id {
                            soil += 1;
                        } else if m == packed_id {
                            packed += 1;
                        }
                    }
                    if ground {
                        above += 1;
                    }
                }
            }
            (void, roofed, roofed3, soil, packed)
        };

        // The sheet is written for the first seed only: it is there to show
        // *what* and *where*, and the table above it is what says how much
        // and whether it came back. Four pictures of four seeds would be
        // four samples of a wide distribution presented as if they were a
        // result.
        let mut renderer = Renderer::new();
        // **The lighting model is a parameter of this measurement, not a
        // constant of it.** `SkyLight::Coarse4` is the shipped default and
        // propagates sky light on a **4-cell block grid**; an ant gallery is
        // one to three cells across, so the nest is a feature finer than the
        // model that is supposed to darken it. `sky-light-design.md` measured
        // exactly this one step coarser -- block 8 "loses a one-cell shaft
        // entirely" -- and nothing had asked what block 4 does to a structure
        // the size of a burrow. `light=depth|4|2|1` asks.
        renderer.sky_light = match arg::<String>("light").as_deref() {
            Some("depth") => pixel_physics::render::SkyLight::Depth,
            Some("2") => pixel_physics::render::SkyLight::Coarse2,
            Some("1") => pixel_physics::render::SkyLight::Exact,
            _ => pixel_physics::render::SkyLight::Coarse4,
        };
        let particles = ParticleSystem::new();
        let (vw, vh) = (w as u32, h as u32);
        let mut tiles: Vec<Vec<u8>> = Vec::new();
        let shoot = png.is_some() && seed == 1;

        // Steerable so one stop can be asked for on its own: a four-tile
        // column is the right shape for reading a run and the wrong shape for
        // a side-by-side comparison, which is what a review card wants.
        let marks: Vec<u64> = match arg::<String>("marks") {
            Some(v) => v.split(',').map(|m| m.parse().expect("marks=a,b,c")).collect(),
            None => vec![0, 500, 2_000, frames],
        };
        for f in 0..=frames {
            if f > 0 {
                parallel::step(&mut world);
                world.step_active_sites();
                world.step_fields();
                world.step_pheromones();
            }
            if marks.contains(&f) {
                let (void, roofed, roofed3, soil, packed) = census(&world);
                let comps = void_components(&world, (bank_x0, bank_x1), (bank_y0, bank_y1));
                let largest = comps.first().copied().unwrap_or_default();
                let ge8 = comps.iter().filter(|c| c.cells >= 8).count();
                let st = world.creature_stats;
                println!(
                    "{seed:>6}  {f:>7}  {void:>8}  {roofed:>8}  {roofed3:>8}  {:>6}  {:>8}  {ge8:>5}  {:>7}  {:>7}  {:>7}  {soil:>9}  {packed:>10}",
                    comps.len(),
                    largest.cells,
                    largest.roofed,
                    st.digs,
                    st.packed
                );
                // The distribution, at the last stop only. `comps`/`largest`
                // are order statistics over it and cannot say whether the
                // remainder is forty pockets or four hundred crumbs -- which
                // is the whole question when the total is the same number
                // either way.
                if f == *marks.iter().max().unwrap_or(&0) {
                    let singles = comps.iter().filter(|c| c.cells == 1).count();
                    let top: Vec<String> =
                        comps.iter().take(8).map(|c| format!("{}({})", c.cells, c.roofed)).collect();
                    println!(
                        "         sizes(roofed): {}  ...  singletons {singles} of {}",
                        top.join(" "),
                        comps.len()
                    );
                }
                if shoot {
                    let mut buf = vec![0u8; (vw * vh * 4) as usize];
                    let touched = world.take_touched_chunks();
                    renderer.draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
                    tiles.push(buf);
                }
            }
        }

        // **What do these things actually look like next to each other?** --
        // the render-side half of the legibility question, and it cannot be
        // answered from the palette tables. `soil.ron` and `packedsoil.ron`
        // list twelve tones each, but what reaches the screen is whatever
        // `Renderer::draw` makes of them after lighting and shading, and it is
        // the screen the owner is judging. So this samples the **shipped
        // renderer's own output buffer**, one pixel per cell, and reports the
        // mean colour and relative luminance of each class standing in the
        // bank.
        //
        // The column that matters is not the gap between two means. It is the
        // gap between two means measured against the **spread within one
        // class**: a bank is deliberately mottled, and a lining whose tone sits
        // inside that mottle is not a faint signal, it is no signal -- the two
        // populations overlap and no amount of looking separates them. That is
        // the quantitative form of the owner's *"These look identical"*.
        //
        // Generalises well past ants: any question of the form "can a player
        // see X against Y" in this engine is this measurement, and nothing
        // else here could answer it.
        if std::env::args().any(|a| a == "contrast=1") && seed == 1 {
            let mut buf = vec![0u8; (vw * vh * 4) as usize];
            let touched = world.take_touched_chunks();
            let mut r2 = Renderer::new();
            r2.sky_light = renderer.sky_light;
            r2.draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
            let lum = |c: [f64; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
            let mut classes: Vec<(&str, Vec<f64>, [f64; 3])> = vec![
                ("soil", Vec::new(), [0.0; 3]),
                ("packedsoil", Vec::new(), [0.0; 3]),
                ("void (roofed)", Vec::new(), [0.0; 3]),
                ("void (open face)", Vec::new(), [0.0; 3]),
                ("sky (control)", Vec::new(), [0.0; 3]),
            ];
            // **Two controls, because a colour is only a finding against a
            // known answer.** `CLAUDE.md`: construct the case whose answer you
            // know and check the instrument reports it. Open sky must come
            // back pale and tight; the unroofed void the ants opened at the
            // face is open air and must come back indistinguishable from it.
            // If either misses, the sampling is wrong and the roofed-void row
            // says nothing.
            for x in bank_x0..bank_x1 {
                for y in (bank_y0 - 12).max(0)..bank_y0 - 2 {
                    if world.get(x, y).material == material::EMPTY {
                        let o = ((y as u32 * vw + x as u32) * 4) as usize;
                        let px = [buf[o] as f64, buf[o + 1] as f64, buf[o + 2] as f64];
                        for (k, v) in px.iter().enumerate() {
                            classes[4].2[k] += v;
                        }
                        classes[4].1.push(lum(px));
                    }
                }
            }
            for x in bank_x0..bank_x1 {
                let mut above = 0usize;
                for y in 0..bank_y1 {
                    let m = world.get(x, y).material;
                    let ground = matches!(
                        world.materials.kind(m),
                        MaterialKind::Powder | MaterialKind::Solid
                    );
                    if y >= bank_y0 {
                        let which = if m == soil_id {
                            Some(0)
                        } else if m == packed_id {
                            Some(1)
                        } else if m == material::EMPTY {
                            Some(if above > 0 { 2 } else { 3 })
                        } else {
                            None
                        };
                        if let Some(i) = which {
                            let o = ((y as u32 * vw + x as u32) * 4) as usize;
                            let px = [buf[o] as f64, buf[o + 1] as f64, buf[o + 2] as f64];
                            for (k, v) in px.iter().enumerate() {
                                classes[i].2[k] += v;
                            }
                            classes[i].1.push(lum(px));
                        }
                    }
                    if ground {
                        above += 1;
                    }
                }
            }
            println!("  contrast, as the shipped renderer draws it (one pixel per cell, frame {frames}):");
            println!("{:>16}  {:>7}  {:>17}  {:>7}  {:>15}", "class", "cells", "mean RGB", "mean L", "L range (p5-p95)");
            let mut summary: Vec<(String, f64, f64, f64)> = Vec::new();
            for (name, mut ls, sum) in classes.into_iter() {
                if ls.is_empty() {
                    continue;
                }
                let n = ls.len() as f64;
                let mean = lum([sum[0] / n, sum[1] / n, sum[2] / n]);
                ls.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in a luminance"));
                let (lo, hi) = (ls[ls.len() / 20], ls[ls.len() * 19 / 20]);
                println!(
                    "{name:>16}  {:>7}  {:>17}  {mean:>7.1}  {:>15}",
                    ls.len(),
                    format!("({:.0}, {:.0}, {:.0})", sum[0] / n, sum[1] / n, sum[2] / n),
                    format!("{lo:.1} - {hi:.1}")
                );
                summary.push((name.to_string(), mean, lo, hi));
            }
            // The verdict line. Two classes are *separable* only if the gap
            // between their means beats the spread each of them already has --
            // otherwise a cell of one is routinely brighter than a cell of the
            // other and the boundary between them is not a boundary.
            for i in 0..summary.len() {
                for j in i + 1..summary.len() {
                    let (a, b) = (&summary[i], &summary[j]);
                    let gap = (a.1 - b.1).abs();
                    let spread = ((a.3 - a.2) + (b.3 - b.2)) / 2.0;
                    println!(
                        "  {} vs {}: mean gap {:.1} of 255 against a within-class spread of {:.1} -- {}",
                        a.0,
                        b.0,
                        gap,
                        spread,
                        if gap > spread { "separable" } else { "**overlapping: these read as one material**" }
                    );
                }
            }
        }

        // **An ASCII dump of the bank, because a contact sheet cannot answer
        // the question the roofed count raises.** `roofed` says *how much*
        // enclosed void there is; it cannot say whether that is one gallery,
        // forty disconnected pockets, or a rind of overhang along one face --
        // and those are different findings with the same number
        // (`CLAUDE.md`: an image tells you what and where, a metric how much).
        // At three pixels a cell a one-cell gallery is invisible in the sheet,
        // so this is the channel that shows the shape.
        if std::env::args().any(|a| a == "map=1") && seed == 1 {
            println!("  bank at frame {frames}: '.' loose soil, '#' packedsoil, ' ' void, ',' other");
            for y in bank_y0..bank_y1 {
                let mut row = String::new();
                for x in bank_x0..bank_x1 {
                    let m = world.get(x, y).material;
                    row.push(if m == material::EMPTY {
                        ' '
                    } else if m == soil_id {
                        '.'
                    } else if m == packed_id {
                        '#'
                    } else {
                        ','
                    });
                }
                println!("  |{row}|");
            }
        }

        if shoot {
            // **Crop to the bank, then magnify.** The first version of this
            // sheet rendered the whole 200x120 world at zoom 3 and was
            // unreadable for the thing it exists to show: the world is mostly
            // sky, the galleries are one to three cells across, and the ants
            // work the bank's near face. Rendered whole, the lined bank and
            // the ablated one look like the same brown mound -- which would
            // have had the picture contradict a `roofed` count of 130 against
            // 0 and the ASCII map that shows a warren, and the picture is what
            // this project settles arguments with.
            //
            // Done here rather than through the renderer's own zoom, which
            // moves the *camera* rather than the scale of the output.
            //
            // **The default window is the near third, not the whole bank.**
            // The colony enters from the nest at x=16..40 and works into the
            // face it meets, so the excavation is a pocket at that end and the
            // other two thirds are undisturbed ground -- a window over the
            // whole bank spends 70% of its pixels on soil nothing happened to,
            // and at that scale a three-cell gallery is three pixels.
            //
            // Steerable all the same, because "is there a nest in there" and
            // "what does the bank look like" want different framings and
            // neither answers the other.
            let zoom: u32 = arg("zoom").unwrap_or(10);
            let crop: String = arg("crop").unwrap_or_else(|| {
                format!("{},{},56,{}", bank_x0 - 26, bank_y0 - 6, bank_y1 - bank_y0 + 12)
            });
            let c: Vec<u32> = crop.split(',').map(|v| v.parse().expect("crop=x,y,w,h")).collect();
            let (cx0, cy0, cw, ch) = (c[0], c[1], c[2], c[3]);
            let (sw, sh) = (cw * zoom, ch * zoom * tiles.len() as u32);
            let mut sheet = vec![0u8; (sw * sh * 4) as usize];
            for (i, tile) in tiles.iter().enumerate() {
                let y0 = i as u32 * ch * zoom;
                for y in 0..ch * zoom {
                    for x in 0..sw {
                        let src = (((cy0 + y / zoom) * vw + cx0 + x / zoom) * 4) as usize;
                        let dst = (((y0 + y) * sw + x) * 4) as usize;
                        sheet[dst..dst + 4].copy_from_slice(&tile[src..src + 4]);
                    }
                }
            }
            let out = png.expect("checked above");
            image::save_buffer(out, &sheet, sw, sh, image::ColorType::Rgba8)
                .expect("writing the sheet");
            println!("  wrote {out} ({sw}x{sh}) -- frames {marks:?} stacked top to bottom");
        }
    }
}
