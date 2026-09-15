//! **What does a colony's homing actually rest on, and what does the width of
//! its door do to it?** Built for `Reports/nest-design-2026-09-14.md`.
//!
//! Three scenes, one readout, so the same question can be asked of the bed
//! the constraint was measured on, of a bank with a real round trip, and of
//! the lab's own played bed:
//!
//! - **`scene=loop`** -- `examples/ascii.rs`'s "ants: the foraging loop", the
//!   scene the 414-delivery constraint was measured on. **It is not a homing
//!   scene**: only 15 of its 55 ants are ever placed (worldgen seed 1 puts
//!   water and slope under the other 40), they live on the 74-column patch,
//!   and channel A reads **706 at frame 2,000 and 0 by 6,000** while
//!   deliveries climb 241 -> 670. A delivery there is an ant eating beside
//!   its door. Kept as the control that says so.
//! - **`scene=probe`** -- `examples/forage_probe.rs`'s bank: stone floor, a
//!   32-column patch at `16..48`, a corpse pile `gap=` (87) cells past its
//!   right edge, 55 ants at `20 + spacing=` (2) `i`. **At the shipped gap
//!   nobody reaches the pile** (deepest excursion 28 of 87 on seed 1) and
//!   49 of 55 ants are dead by 8,000 frames -- `creature-direction.md` §13g
//!   already records a flat floor as degenerate. `gap=30` is the reachable
//!   form.
//! - **`scene=bed`** -- the lab's `played_bed` scenario through `Lab`, the
//!   world `nestdoor` and `latecensus` measure; `frames=` defaults to 40,000
//!   here because a frame costs ~3 ms.
//!
//! And the two things the report had to vary and no scene can:
//!
//! - **`width=`** -- how many columns the patch spans, centred where the
//!   scene's own patch is centred (on `bed`, applied the frame the scenario
//!   paints it, by extending or erasing surface cells). A site- or blob-based
//!   `AtNest` with radius `r` is, to every counter here, a patch `2r` wide,
//!   so sweeping this *is* pricing the blob before anything is built.
//! - **`arm=`** -- which half of the homing circuit is cut, by patching the
//!   species genome before the first ant is planted (the `ascii`
//!   `DROP_MOISTURE` pattern: after the spawn it is too late, the genome has
//!   been copied). `shipped` is the control; `noemit` zeroes the odometer's
//!   output leg so no ant lays a grain of channel A; `nosteer` leaves the
//!   plane laid and zeroes the two `Move` legs that read it, so a laden ant
//!   can no longer walk up it.
//! - **`diffuse=`** -- channel A's blend fraction, through the setter Lane C
//!   shipped, so the homing plane's lifetime can be moved without touching B.
//!
//! What it reads, and why each is a *standing* count rather than an event:
//!
//! - **`deliveries`** is the number the constraint quotes, and it is a drop
//!   made while 8-adjacent to nest material -- so a patch under the whole
//!   band scores every drop as a delivery. It cannot rank footprints on its
//!   own; the next one can.
//! - **`larder`** -- food cells within `LARDER_HALF` columns of the patch
//!   *centre* at the end, minus the same census when the ants were placed: a
//!   fixed window whatever the width, so a wide door and a narrow one are
//!   scored on the same ground. On `probe` nothing grows, so it is exactly
//!   the corpse carried home.
//! - **ant-ticks by band** -- every `sample` frames, each live ant's head is
//!   binned `nest` / `mid` / `food`, and the laden ones separately. The
//!   owner's claim is *"ants mostly move to where food is"*; this is that
//!   claim as a number.
//! - **channel A by band** -- the gradient itself, at the end and mid-run.
//! - **`lost`** -- patch cells first painted that are no longer `nest`.
//!
//! ```text
//! cargo run --release --example nesthome -- scene=probe arm=noemit seed=3
//! cargo run --release --example nesthome -- scene=probe width=120
//! cargo run --release --example nesthome -- scene=bed frames=40000 arm=nosteer
//! ```
//!
//! Echoes its own parameters on the first line (the megastudy gotcha), and
//! prints one `SUMMARY` line a script can grep.

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::Lab;
use pixel_physics::sim::brain::{ho_slot, BrainOutput};
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::pheromone::Channel;
use pixel_physics::sim::{parallel, Cell, World};

/// Half-width of the fixed larder window around the patch centre.
const LARDER_HALF: i32 = 30;

fn arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{name}=")).and_then(|v| v.parse().ok()))
        .unwrap_or(default)
}

fn arg_str(name: &str, default: &str) -> String {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{name}=")).map(str::to_string))
        .unwrap_or_else(|| default.to_string())
}

fn surface(world: &World, x: i32, h: i32) -> i32 {
    surface_from(world, x, 0, h)
}

/// The first ground cell at or below `from` -- the lab's box has a sealed
/// lid, so a scan from row 0 finds the ceiling, and a patch "widened" there
/// is painted 160 rows above the colony (measured: four widths, byte-identical
/// runs, before this existed).
fn surface_from(world: &World, x: i32, from: i32, h: i32) -> i32 {
    (from.max(0)..h)
        .find(|&y| {
            let c = world.get(x, y);
            c.organism_id() == 0 && matches!(world.materials.kind(c.material), MaterialKind::Solid | MaterialKind::Powder)
        })
        .unwrap_or(h - 1)
}

fn food_cells(world: &World, keep: impl Fn(i32) -> bool) -> usize {
    let Some(b) = world.bounds() else { return 0 };
    (b.min_x..=b.max_x)
        .flat_map(|x| (b.min_y..=b.max_y).map(move |y| (x, y)))
        .filter(|&(x, y)| keep(x) && pixel_physics::sim::creature::food_value(world, world.get(x, y)) > 0.0)
        .count()
}

fn channel_total(world: &World, ch: Channel, lo: i32, hi: i32) -> u64 {
    let Some(b) = world.bounds() else { return 0 };
    (lo.max(b.min_x)..hi.min(b.max_x + 1)).flat_map(|x| (b.min_y..=b.max_y).map(move |y| (x, y))).map(|(x, y)| world.pheromone_at(ch, x, y) as u64).sum()
}

/// Cut the homing circuit in the species genome. Must run before any ant
/// of the species is placed.
fn apply_arm(world: &mut World, arm: &str) -> bool {
    let id = world.species.id_of("ant").expect("ant");
    let mut g = world.species.get(id).genome.clone();
    match arm {
        "shipped" => {}
        "noemit" => {
            let s = ho_slot(4, BrainOutput::EmitA);
            println!("  ABLATED: odometer output leg (4, EmitA, {:.1}) -> 0: no ant lays channel A", g[s]);
            g[s] = 0.0;
        }
        "nosteer" => {
            let (s0, s1) = (ho_slot(0, BrainOutput::Move), ho_slot(1, BrainOutput::Move));
            println!("  ABLATED: homing steer legs (0, Move, {:.1}) and (1, Move, {:.1}) -> 0: channel A laid, never followed", g[s0], g[s1]);
            g[s0] = 0.0;
            g[s1] = 0.0;
        }
        other => {
            println!("unknown arm {other}; use shipped|noemit|nosteer");
            return false;
        }
    }
    world.species.set_genome(id, g);
    true
}

/// Paint a patch `width` wide centred on `centre` along the surface, or --
/// on a bed where the scenario has already painted one -- extend or erase
/// the existing patch to that width. Returns the cells that are `nest` after.
fn set_patch_width(world: &mut World, centre: i32, width: i32, from_y: i32, h: i32) -> Vec<(i32, i32)> {
    let nest = world.materials.id_of("nest").expect("nest");
    let soil = world.materials.id_of("soil").expect("soil");
    let Some(b) = world.bounds() else { return Vec::new() };
    let lo = centre - width / 2;
    let hi = lo + width;
    let mut cells = Vec::new();
    for x in (b.min_x + 1)..b.max_x {
        let sy = surface_from(world, x, from_y, h);
        let c = world.get(x, sy);
        let inside = x >= lo && x < hi;
        if inside && c.material != nest && matches!(world.materials.kind(c.material), MaterialKind::Solid | MaterialKind::Powder) {
            world.set(x, sy, Cell::new(nest, 0).with_attached(c.attached()));
        } else if !inside && c.material == nest {
            world.set(x, sy, Cell::new(soil, 0));
        }
        if world.get(x, sy).material == nest {
            cells.push((x, sy));
        }
    }
    cells
}

struct Bands {
    nest_lo: i32,
    nest_hi: i32,
    /// `food` is `|x - centre| > far` on the bed and `x >= far` elsewhere.
    far: i32,
    centre: i32,
    radial: bool,
}

impl Bands {
    fn of(&self, x: i32) -> usize {
        if (self.nest_lo..self.nest_hi).contains(&x) {
            0
        } else if (self.radial && (x - self.centre).abs() > self.far) || (!self.radial && x >= self.far) {
            2
        } else {
            1
        }
    }
}

struct Census {
    ticks: [u64; 3],
    laden: [u64; 3],
}

fn census(world: &World, bands: &Bands, c: &mut Census) {
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if world.species.get(st.species).creature.is_none() {
            continue;
        }
        let Some(&(hx, _)) = st.chain.first() else { continue };
        let b = bands.of(hx);
        c.ticks[b] += 1;
        if st.crop.is_some() {
            c.laden[b] += 1;
        }
    }
}

fn main() {
    let scene = arg_str("scene", "probe");
    let seed: u64 = arg("seed", 1);
    let default_frames = match scene.as_str() {
        "bed" => 40_000,
        "loop" => 12_000,
        _ => 8_000,
    };
    let frames: usize = arg("frames", default_frames);
    let sample: usize = arg("sample", 250);
    let arm = arg_str("arm", "shipped");
    let diffuse: f32 = arg("diffuse", -1.0);
    let width: i32 = arg("width", -1);
    let diffuse_label = if diffuse < 0.0 { "shipped".to_string() } else { format!("{diffuse}") };
    println!("nesthome: scene={scene} seed={seed} width={} arm={arm} diffuse={diffuse_label} frames={frames} sample={sample}", if width < 0 { "scene".to_string() } else { format!("{width}") });

    // Everything the three scenes differ in, resolved here; the loop below
    // is shared.
    let mut lab: Option<Lab> = None;
    let mut world_owned: Option<World> = None;
    let bands;
    let h;
    let mut patch: Vec<(i32, i32)>;
    let mut bed_pending_width = false;
    match scene.as_str() {
        "loop" => {
            let (w, hh) = (512i32, 120i32);
            h = hh;
            let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
            let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
            if let Some(e) = err {
                println!("worldgen presets unavailable ({e})");
                return;
            }
            let Some(params) = presets.get(&presets.default_name()) else { return };
            pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });
            patch = set_patch_width(&mut world, 53, if width < 0 { 74 } else { width }, 0, h);
            for i in 0..6 {
                let x = 230 + i * 40;
                let sy = surface(&world, x, h);
                world.plant_tree(x, sy - 1);
            }
            for _ in 0..2400 {
                parallel::step(&mut world);
                world.step_active_sites();
                world.step_fields();
            }
            let litter = world.materials.id_of("litter").expect("litter");
            for x in (200..470).step_by(3) {
                let sy = surface(&world, x, h);
                if sy > 0 && world.is_empty(x, sy - 1) {
                    world.set(x, sy - 1, Cell::new(litter, (x as u8) % 4));
                }
            }
            if !apply_arm(&mut world, &arm) {
                return;
            }
            let mut colony = None;
            let mut placed_x = Vec::new();
            for i in 0..55 {
                let ax = 24 + i * 4;
                let sy = surface(&world, ax, h);
                if let Some(site) = pixel_physics::sim::creature::plant_creature_seed_in(&mut world, ax, sy - 1, "ant", colony) {
                    colony = colony.or_else(|| pixel_physics::sim::creature::colony_of_site(&world, &site));
                    world.schedule_active_site(site);
                    placed_x.push(ax);
                }
            }
            println!("  placed {} of 55 ants at x = {placed_x:?}", placed_x.len());
            bands = Bands { nest_lo: 16, nest_hi: 90, far: 200, centre: 53, radial: false };
            world_owned = Some(world);
        }
        "probe" => {
            let (w, hh) = (320i32, 120i32);
            h = hh;
            let floor = h - 8;
            let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
            world.seed = seed;
            for x in 0..w {
                for y in floor..h {
                    world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            patch = set_patch_width(&mut world, 32, if width < 0 { 32 } else { width }, 0, h);
            let corpse = world.materials.id_of("corpse").expect("corpse");
            let gap: i32 = arg("gap", 87);
            let spacing: i32 = arg("spacing", 2);
            let food_x = 48 + gap;
            for x in food_x..(food_x + 35) {
                for y in (floor - 5)..floor {
                    world.set(x, y, Cell::new(corpse, 0));
                }
            }
            if !apply_arm(&mut world, &arm) {
                return;
            }
            for i in 0..55 {
                world.plant_ant(20 + i * spacing, floor - 1);
            }
            println!("  probe: pile at x {food_x}..{}, ants at 20 + {spacing}i", food_x + 35);
            bands = Bands { nest_lo: 16, nest_hi: 48, far: food_x, centre: 32, radial: false };
            world_owned = Some(world);
        }
        "bed" => {
            let mut sc = Scenario::load("played_bed").unwrap_or_else(|e| {
                eprintln!("scenario played_bed: {e}");
                std::process::exit(1);
            });
            sc.bed.seed = seed;
            let mut l = Lab::new(sc.bed.clone());
            let msg = l.load_scenario(sc);
            println!("  {msg}");
            // **After the load, not before.** `load_scenario` calls
            // `Lab::reset`, which builds a brand-new `World` -- a genome
            // patched into the one `Lab::new` made is thrown away with it,
            // and the three arms come back byte-identical (measured, 3 of 3
            // seeds, before this line moved). The colony itself arrives from
            // the timeline at frame 6,000, so there is time to cut the wire.
            if !apply_arm(&mut l.world, &arm) {
                return;
            }
            h = l.world.bounds().map_or(320, |b| b.max_y + 1);
            patch = Vec::new();
            bed_pending_width = true;
            bands = Bands { nest_lo: 0, nest_hi: 0, far: 96, centre: 0, radial: true };
            lab = Some(l);
        }
        other => {
            println!("unknown scene {other}; use loop|probe|bed");
            return;
        }
    }
    let mut bands = bands;
    if diffuse >= 0.0 {
        let w = lab.as_mut().map(|l| &mut l.world).or(world_owned.as_mut()).expect("a world");
        w.pheromones.set_channel_diffuse(Channel::A, diffuse);
    }

    let mut c = Census { ticks: [0; 3], laden: [0; 3] };
    let mut larder_at_start: Option<usize> = None;
    let mut worst = std::time::Duration::ZERO;
    let mut sum = std::time::Duration::ZERO;
    let mut stepped = 0usize;
    for f in 0..frames {
        // The bed founds its colony from the scenario on its own schedule;
        // catch the patch the frame it appears, size it, and set the bands.
        if bed_pending_width {
            let world = &mut lab.as_mut().expect("bed").world;
            let nest = world.materials.id_of("nest").expect("nest");
            let Some(b) = world.bounds() else { break };
            let cells: Vec<(i32, i32)> = (b.min_x..=b.max_x)
                .flat_map(|x| (b.min_y..=b.max_y).map(move |y| (x, y)))
                .filter(|&(x, y)| world.get(x, y).material == nest)
                .collect();
            if !cells.is_empty() {
                let (x0, x1) = (cells.iter().map(|c| c.0).min().unwrap_or(0), cells.iter().map(|c| c.0).max().unwrap_or(0));
                let centre = (x0 + x1) / 2;
                println!("  frame {f}: scenario painted {} nest cells, x {x0}..{x1}, centre {centre}", cells.len());
                let top = cells.iter().map(|c| c.1).min().unwrap_or(0) - 24;
                patch = if width < 0 { cells } else { set_patch_width(world, centre, width, top, h) };
                let (plo, phi) = (patch.iter().map(|c| c.0).min().unwrap_or(0), patch.iter().map(|c| c.0).max().unwrap_or(0) + 1);
                bands = Bands { nest_lo: plo, nest_hi: phi, far: 96, centre, radial: true };
                println!("  patch now {} cells, x {plo}..{phi}", patch.len());
                bed_pending_width = false;
            }
        }
        if larder_at_start.is_none() && !bed_pending_width {
            let world = lab.as_ref().map(|l| &l.world).or(world_owned.as_ref()).expect("a world");
            larder_at_start = Some(food_cells(world, |x| (x - bands.centre).abs() <= LARDER_HALF));
            println!("  frame {f}: {} creatures alive, larder window holds {} food cells", world.live_creature_count(), larder_at_start.unwrap_or(0));
        }
        let started = std::time::Instant::now();
        match (&mut lab, &mut world_owned) {
            (Some(l), _) => l.tick_for_harness(),
            (None, Some(w)) => {
                parallel::step(w);
                w.step_active_sites();
                w.step_fields();
                w.step_pheromones();
            }
            _ => unreachable!(),
        }
        let took = started.elapsed();
        worst = worst.max(took);
        sum += took;
        stepped += 1;
        let world = lab.as_ref().map(|l| &l.world).or(world_owned.as_ref()).expect("a world");
        if f % (frames / 4).max(1) == 0 && f > 0 {
            let st = world.creature_stats;
            let span = bands.far.min(64);
            println!(
                "  at {f}: alive {} deliveries {} pickups {} trips {} | channel A near/mid/far {} / {} / {}",
                world.live_creature_count(),
                st.deliveries,
                st.pickups,
                st.forage_trips,
                channel_total(world, Channel::A, bands.centre - span, bands.centre + span),
                channel_total(world, Channel::A, bands.centre + span, bands.centre + 2 * span),
                channel_total(world, Channel::A, bands.centre + 2 * span, i32::MAX / 2),
            );
        }
        if f % sample == 0 && !bed_pending_width {
            census(world, &bands, &mut c);
        }
    }

    let world = lab.as_ref().map(|l| &l.world).or(world_owned.as_ref()).expect("a world");
    let nest = world.materials.id_of("nest").expect("nest");
    let st = world.creature_stats;
    let larder = food_cells(world, |x| (x - bands.centre).abs() <= LARDER_HALF) as i64 - larder_at_start.unwrap_or(0) as i64;
    let lost = patch.iter().filter(|&&(x, y)| world.get(x, y).material != nest).count();
    let bw = 32;
    let profile: Vec<String> = (0..10).map(|i| format!("{}", channel_total(world, Channel::A, i * bw, (i + 1) * bw))).collect();
    let total: u64 = c.ticks.iter().sum::<u64>().max(1);
    let ltotal: u64 = c.laden.iter().sum::<u64>().max(1);
    println!(
        "  pickups {} drops {} deliveries {} nest_visits {} forage_trips {} deepest {} at_nest_ticks {} | alive {} deaths {} births {}",
        st.pickups,
        st.drops,
        st.deliveries,
        st.nest_visits,
        st.forage_trips,
        st.forage_depth_max,
        st.at_nest_ticks,
        world.live_creature_count(),
        st.deaths,
        st.births
    );
    println!("  larder (±{LARDER_HALF} of centre {}): {larder:+} | patch cells lost {lost} of {}", bands.centre, patch.len());
    println!(
        "  ant-ticks nest/mid/food: {:.1}% / {:.1}% / {:.1}% | laden ant-ticks nest/mid/food: {:.1}% / {:.1}% / {:.1}% ({} laden samples of {})",
        100.0 * c.ticks[0] as f64 / total as f64,
        100.0 * c.ticks[1] as f64 / total as f64,
        100.0 * c.ticks[2] as f64 / total as f64,
        100.0 * c.laden[0] as f64 / ltotal as f64,
        100.0 * c.laden[1] as f64 / ltotal as f64,
        100.0 * c.laden[2] as f64 / ltotal as f64,
        c.laden.iter().sum::<u64>(),
        total
    );
    println!("  channel A by 32-col band from x=0: {}", profile.join(" "));
    println!("  frame cost: worst {:.3} ms, mean {:.3} ms over {stepped} frames", worst.as_secs_f64() * 1000.0, sum.as_secs_f64() * 1000.0 / stepped.max(1) as f64);
    println!(
        "SUMMARY scene={scene} arm={arm} width={} seed={seed} diffuse={diffuse_label} pickups={} deliveries={} visits={} trips={} deepest={} larder={larder} lost={lost} alive={} nest%={:.1} food%={:.1} laden_nest%={:.1} laden_food%={:.1} A={}",
        patch.len(),
        st.pickups,
        st.deliveries,
        st.nest_visits,
        st.forage_trips,
        st.forage_depth_max,
        world.live_creature_count(),
        100.0 * c.ticks[0] as f64 / total as f64,
        100.0 * c.ticks[2] as f64 / total as f64,
        100.0 * c.laden[0] as f64 / ltotal as f64,
        100.0 * c.laden[2] as f64 / ltotal as f64,
        profile.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(",")
    );
}
