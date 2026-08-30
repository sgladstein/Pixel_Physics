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
        "{:>6}  {:>7}  {:>8}  {:>8}  {:>8}  {:>7}  {:>7}  {:>9}  {:>10}",
        "seed", "frame", "void", "roofed", "roofed3", "digs", "packed", "soil", "packedsoil"
    );

    for seed in 1..=seeds {
        let (w, h) = (200i32, 120i32);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        world.set_weather_pin(Pin::Clear);
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
        let particles = ParticleSystem::new();
        let (vw, vh) = (w as u32, h as u32);
        let mut tiles: Vec<Vec<u8>> = Vec::new();
        let shoot = png.is_some() && seed == 1;

        let marks = [0u64, 500, 2_000, frames];
        for f in 0..=frames {
            if f > 0 {
                parallel::step(&mut world);
                world.step_active_sites();
                world.step_fields();
                world.step_pheromones();
            }
            if marks.contains(&f) {
                let (void, roofed, roofed3, soil, packed) = census(&world);
                let st = world.creature_stats;
                println!(
                    "{seed:>6}  {f:>7}  {void:>8}  {roofed:>8}  {roofed3:>8}  {:>7}  {:>7}  {soil:>9}  {packed:>10}",
                    st.digs, st.packed
                );
                if shoot {
                    let mut buf = vec![0u8; (vw * vh * 4) as usize];
                    let touched = world.take_touched_chunks();
                    renderer.draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
                    tiles.push(buf);
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
            // work the bank's left face. Rendered whole, the lined bank and
            // the ablated one look like the same brown mound -- which would
            // have had the picture contradict a `roofed` count of 130 against
            // 0 and the ASCII map that shows a warren, and the picture is what
            // this project settles arguments with.
            //
            // Done here rather than through the renderer's own zoom, which
            // moves the *camera* rather than the scale of the output.
            // Steerable, because "is there a nest in there" and "what does
            // the bank look like" want different framings and the second one
            // cannot answer the first: at zoom 6 over the whole bank a
            // one-cell gallery is six pixels of dark brown inside dark brown.
            let zoom: u32 = arg("zoom").unwrap_or(6);
            let crop: String =
                arg("crop").unwrap_or_else(|| format!("{},{},96,38", bank_x0 - 26, bank_y0 - 4));
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
