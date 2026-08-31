//! **Where does a dead plant's mass go, in a sealed box?**
//!
//! The owner's question, 2026-08-31: *"after a plant or tree dies does every
//! part of it actually degrade to soil?"* That is a question about a
//! **ledger**, and nothing in `examples/` kept one. `crown_census` counts
//! material by height, `labsoil` counts the bed's water and the stand it
//! carries, `litter_probe` says where litter comes to *rest* — none of them
//! follows a cohort of plant cells from standing tissue through the litter
//! pool to whatever is left when the pool has finished rotting.
//!
//! This does, in the lab's own bed, and the reason it has to be the lab's bed
//! is that **the lab is sealed**. Outdoors a plant is a source: it fixes
//! carbon out of light and builds cells with it, so matter arriving from
//! nowhere is the normal case and `litter.ron`'s 5% yield is the sink that
//! stops the forest floor climbing forever. In a box with a ceiling the same
//! number reads the other way round — the bed is finite, roots eat it, and
//! what rot gives back is the only thing that refills it.
//!
//! ```text
//! cargo run --release --example labmass                          # the standard cycle
//! cargo run --release --example labmass -- control=empty         # nothing alive: every figure must be 0
//! cargo run --release --example labmass -- yield=1               # positive control: the return must be ~100%
//! cargo run --release --example labmass -- yield=0               # negative control: the return must be exactly 0
//! cargo run --release --example labmass -- grow=9000 rot=24000   # a longer cycle
//! cargo run --release --example labmass -- colonies=1            # with ants eating the floor
//! ```
//!
//! # What it reports, and why these three numbers rather than one
//!
//! - **The decay step** — `rotted_to_solid / (to_solid + to_nothing)`. What
//!   fraction of the cells that *reached* the rot roll left a solid behind.
//!   This is `Material::decay_yield` measured rather than read off the file,
//!   and it is the only one of the three that should match a constant.
//! - **The whole-plant return** — soil cells produced, over the plant cells
//!   that died. Lower than the decay step whenever some tissue never reaches
//!   the litter pool at all: `deadwood` and `corpse` have no `decays_into`
//!   and are permanent, and anything an ant eats leaves by a third door.
//! - **The bed balance** — the mineral bed's cell count at the end against
//!   its count before anything was planted. **This is the sealed-box bottom
//!   line**, and it is the one that decides whether a multi-generation
//!   experiment in this box means anything.
//!
//! # The controls, which are the point of the harness
//!
//! `CLAUDE.md`: *ask what your number counts when nothing is wrong* — six
//! occurrences, and a ledger over a living box is exactly the shape that
//! lies. So:
//!
//! - `control=empty` is the **specificity** half. No founders, so nothing
//!   dies, so every ledger figure must be zero and the mineral bed must not
//!   move a cell. It doubles as the oscillator check on the quantity this
//!   harness actually reads: `CLAUDE.md` records `cells lost` riding the
//!   water cycle at ±1,700 cells, which would swamp every figure here, and
//!   an empty bed that holds still says the lab's pinned weather has taken
//!   that oscillator out of *this* census rather than out of some other one.
//! - `yield=1` is the **sensitivity** half, and it is not optional. A return
//!   fraction of 5% and a channel that never fired look identical if you only
//!   census soil. At full yield the same run must report ~100% and
//!   `to_nothing 0`; if it does not, the instrument cannot see the thing it
//!   is named for and its 5% was never evidence.
//! - **The root-sink prediction** is a positive control on the *model*
//!   rather than on the instrument. Roots occupy soil by overwriting it, so
//!   the mineral bed must fall by about the number of plant cells standing
//!   below the ground line. The harness predicts it and prints the residual;
//!   a large one means the sink is somewhere this reasoning has not looked.
//! - **The plateau check** is `CLAUDE.md`'s cascade rule: a census taken
//!   before the pool has finished rotting reads a *delay* as a loss. The
//!   last two stops must agree on the mineral count, and the run says so
//!   rather than leaving the reader to assume it.
//!
//! **It echoes its own parameters on the first line**, and `yield=` is
//! echoed as the value actually in force — `decay::decay_yield_override`
//! reads the environment once through a `OnceLock`, so a knob nobody can see
//! the value of is a knob nobody can tell is disconnected.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::material::{MaterialId, MaterialKind, EMPTY};
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// The dead-organic pool: materials a plant or an animal body becomes on the
/// way out, none of which is `MaterialKind::Plant` any more.
///
/// **Listed by name rather than derived, and the list is checked rather than
/// trusted.** There is no flag on `Material` that says "this came from
/// something that was alive" — `decays_into` is not it (`ash` weathers to
/// soil and is mineral; `deadwood` came from a tree and decays into nothing),
/// and `MaterialKind` is not it either (all five of these are `Powder` or
/// `Solid`, the same kinds as sand and stone). So the classification is
/// stated here, and every material found in the bed that lands in no bucket
/// is reported by name under `unclassified` — which is what stops a material
/// added later from being silently dropped out of the ledger.
const NECROMASS: &[&str] = &["litter", "windfall", "deadleaf", "deadwood", "log", "corpse"];
/// The mineral bed. `packedsoil` is here because an ant packing a gallery
/// wall does not take the cell out of the bed, it only changes its state —
/// counting soil alone would score a colony's tunnelling as mineral loss.
const MINERAL: &[&str] = &["soil", "packedsoil"];

#[derive(Default, Clone, Debug)]
struct Ledger {
    frame: u64,
    /// Cells owned by a living or standing-dead plant organism.
    plant: usize,
    /// ...of which, below the ground line: the cells that took a bed cell.
    plant_below: usize,
    /// Cells owned by an animal.
    animal: usize,
    /// The dead-organic pool, per material.
    necro: Vec<(String, usize)>,
    /// The mineral bed.
    mineral: usize,
    /// Anything standing in the bed that is in none of the buckets above.
    unclassified: Vec<(String, usize)>,
    /// The decay channel's own resolution counters. `to_solid` counts every
    /// decay that left *a solid of any kind*, so it includes the
    /// `deadleaf -> litter` step that produces no soil; `onward` is how many
    /// of those were that intermediate step, and the difference is what
    /// reached the end of a chain. See `World::rotted_onward`.
    to_solid: u32,
    to_nothing: u32,
    onward: u32,
    /// Pool cells whose material has no `decays_into` at all — matter that
    /// has stopped moving through the cycle for good. Read off the material
    /// rather than from a list of names, so it falls to zero by itself if a
    /// material is given a decay target.
    locked: usize,
    /// Seeds standing in the bed. Counted apart from the pool: a seed is a
    /// propagule waiting to germinate, not necromass on its way to soil, and
    /// folding it in makes the pool appear to drain when a seed bank sprouts.
    seeds: usize,
    /// Living organisms, split.
    plants: usize,
    animals: usize,
    senescent: usize,
    /// Deepest plant generation alive. A bed that has turned over is the only
    /// bed on which a run-down claim means anything: `CLAUDE.md`'s own
    /// `genome_drift` note is that a drift study over a population that never
    /// turns over cannot answer its question, and the same is true of a mass
    /// ledger over one cohort.
    generation: u16,
}

impl Ledger {
    fn necro_total(&self) -> usize {
        self.necro.iter().map(|(_, n)| n).sum()
    }
}

fn census(world: &World, spec: &LabBox) -> Ledger {
    let mut c = Ledger { frame: world.frame, ..Ledger::default() };
    let mut necro = vec![0usize; world.materials.len()];
    let mut other = vec![0usize; world.materials.len()];
    let mut necro_ids = Vec::new();
    for name in NECROMASS {
        if let Some(id) = world.materials.id_of(name) {
            necro_ids.push((*name, id));
        }
    }
    let mineral_ids: Vec<_> = MINERAL.iter().filter_map(|n| world.materials.id_of(n)).collect();

    for y in 0..spec.height {
        for x in 0..spec.width {
            let cell = world.get(x, y);
            // Raw `EMPTY`, never `Cell::is_empty`, which is managed-aware and
            // answers "is this position available" rather than "is there
            // material here" (`burrow_probe`'s note, and `labsoil`'s).
            if cell.material == EMPTY {
                continue;
            }
            if mineral_ids.contains(&cell.material) {
                c.mineral += 1;
                continue;
            }
            if necro_ids.iter().any(|(_, id)| *id == cell.material) {
                necro[cell.material.0 as usize] += 1;
                continue;
            }
            // **Material-kind, not organism ownership.** A senescent plant's
            // cells are still `MaterialKind::Plant` and still owned, and they
            // are exactly the mass this harness is following — so the test
            // has to admit them. Ownership is used only for the animal split
            // below, where the two agree.
            if world.materials.kind(cell.material) == MaterialKind::Plant {
                c.plant += 1;
                if y >= spec.ground_y {
                    c.plant_below += 1;
                }
                continue;
            }
            if world.materials.kind(cell.material) == MaterialKind::Creature {
                c.animal += 1;
                continue;
            }
            other[cell.material.0 as usize] += 1;
        }
    }
    for (name, id) in &necro_ids {
        let n = necro[id.0 as usize];
        if n > 0 {
            c.necro.push((name.to_string(), n));
            // **Data, not a name list.** A pool material with no
            // `decays_into` is matter the cycle cannot move again. Asking the
            // material means this line reports the truth after the fix as
            // well as before it, which a hardcoded `["deadwood"]` would not.
            if world.materials.get(*id).decays_into.is_none() {
                c.locked += n;
            }
        }
    }
    if let Some(seed) = world.materials.id_of("seed") {
        c.seeds = other[seed.0 as usize];
    }
    // Only the inert furniture of the box should land here — stone, the
    // shell, the lamps, water. Anything else is a material this ledger does
    // not know about, and it is printed so it cannot hide.
    let furniture =
        ["stone", "basalt", "limestone", "sandstone", "mudstone", "growlamp", "water", "nest", "gravel", "seed"];
    for (id, n) in other.iter().enumerate() {
        if *n == 0 {
            continue;
        }
        let name = &world.materials.get(MaterialId(id as u16)).name;
        if !furniture.contains(&name.as_str()) {
            c.unclassified.push((name.clone(), *n));
        }
    }

    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_some() {
            c.animals += 1;
        } else {
            c.plants += 1;
            c.generation = c.generation.max(state.generation);
            if state.senescent {
                c.senescent += 1;
            }
        }
    }
    c.to_solid = world.rotted_to_solid;
    c.to_nothing = world.rotted_to_nothing;
    c.onward = world.rotted_onward;
    c
}

fn line(c: &Ledger) {
    let necro: Vec<String> = c.necro.iter().map(|(m, n)| format!("{m} {n}")).collect();
    println!(
        "  frame {:>7}: plant {:>6} (below {:>5})  animal {:>4} | pool {:>6} [{}] locked {:>5} | seeds {:>4} | mineral {:>7} | rot: to-soil {:>5} onward {:>5} to-nothing {:>6} | plants {:>3} ({} senescent) gen {:>2} ants {:>3}{}",
        c.frame,
        c.plant,
        c.plant_below,
        c.animal,
        c.necro_total(),
        necro.join(", "),
        c.locked,
        c.seeds,
        c.mineral,
        c.to_solid - c.onward,
        c.onward,
        c.to_nothing,
        c.plants,
        c.senescent,
        c.generation,
        c.animals,
        if c.unclassified.is_empty() { String::new() } else { format!(" | UNCLASSIFIED {:?}", c.unclassified) },
    );
}

fn tick(lab: &mut Lab) {
    // The same sequence `Lab::advance` runs, reached directly so a headless
    // run is deterministic — `advance` is wall-clock bounded. `labstats` does
    // the same and for the same reason.
    pixel_physics::sim::frame::step(
        &mut lab.world,
        &mut lab.particles,
        &mut lab.blasts,
        pixel_physics::sim::player::PlayerInput::default(),
        &pixel_physics::sim::player::Tuning::default(),
    );
}

/// Kill half the living plants, oldest slot first — the lab's own graded
/// cull, so a repeated-disturbance run puts the box through the button the
/// owner actually has rather than through a harness-only total wipe.
fn cull_half(world: &mut World) -> usize {
    let ids: Vec<u16> = world
        .live_organism_ids()
        .into_iter()
        .filter(|id| world.organism(*id).is_some_and(|s| world.species.get(s.species).creature.is_none()))
        .collect();
    let mut killed = 0;
    for id in ids.iter().take(ids.len() / 2) {
        if world.mark_organism_senescent(*id) {
            killed += 1;
        }
    }
    killed
}

/// Kill every living plant, so the cohort whose mass is being followed dies
/// at one known frame.
///
/// `mark_organism_senescent` is the shipped experimental-disturbance seam and
/// it produces the **graded** death `plant::rot_remains` carries out at the
/// species half-life, rather than erasing cells — which is the whole point:
/// erasing them would delete the mass this harness exists to follow, and
/// would measure the harness rather than the world.
fn cull_all(world: &mut World) -> usize {
    let ids = world.live_organism_ids();
    let mut killed = 0;
    for id in ids {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_some() {
            continue;
        }
        if world.mark_organism_senescent(id) {
            killed += 1;
        }
    }
    killed
}

fn main() {
    let control: String = arg("control").unwrap_or_else(|| "run".to_string());
    let grow: u64 = arg("grow").unwrap_or(9_000);
    let rot: u64 = arg("rot").unwrap_or(18_000);
    let every: u64 = arg("every").unwrap_or(3_000);
    let seed: u64 = arg("seed").unwrap_or(1);
    let founders = if control == "empty" { 0 } else { arg("founders").unwrap_or(8) };
    // **`cull=0` runs the bed on its own life cycle instead of a forced
    // cohort**, which is the arm that answers the question
    // `Reports/soil-accumulation-and-the-carbon-cycle.md` closes on: *"whether
    // a second and third cohort keep drawing the floor down is not measured,
    // and it is the question this report would ask next."* A forced cull
    // measures one cohort by construction; a long uncontrolled run measures
    // the stand turning over, and `Ledger::generation` is what says it did.
    let culling: u64 = arg("cull").unwrap_or(1);
    // **Cull half the stand every N frames**, which is what the lab's own
    // control bar does -- a graded cull is a shipped button, not a harness
    // invention. The single cull below measures one cohort's fate; this
    // measures what repeated disturbance leaves behind, which is the mode the
    // owner will actually put the box through.
    let cull_every: u64 = arg("cullevery").unwrap_or(0);
    let png: Option<String> = arg("png");
    let colonies: usize = arg("colonies").unwrap_or(0);

    // **Set before the first tick, because the override is read once through
    // a `OnceLock`.** `decay::decay_yield_override` exists at all because
    // materials are `include_str!`d and editing `litter.ron` is invisible to
    // an already-built binary; taking it as an argument here means the
    // control arms are one command apart instead of one rebuild apart, and
    // means the value can be echoed. Edition 2021, so `set_var` is safe.
    let yield_arg: Option<f32> = arg("yield");
    if let Some(v) = yield_arg {
        std::env::set_var("DECAY_YIELD", format!("{v}"));
    }
    let yield_in_force = std::env::var("DECAY_YIELD").ok();

    let spec = LabBox {
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        soil_depth: arg("soil").unwrap_or(80),
        founders,
        colonies,
        compartments: arg("walls").unwrap_or(1),
        seed,
        ..LabBox::default()
    };
    println!(
        "labmass: control={control} grow={grow} rot={rot} every={every} founders={founders} \
         colonies={colonies} soil={} walls={} seed={seed} species={} DECAY_YIELD={}",
        spec.soil_depth,
        spec.compartments,
        spec.species,
        yield_in_force.as_deref().unwrap_or("(unset -- each material's own)"),
    );

    let mut lab = Lab::new(spec.clone());
    // **Before anything has grown.** The bed balance is measured against
    // this, so it is taken at frame 0 rather than reconstructed from the
    // spec's arithmetic — a founder is planted by `LabBox::build`, and a
    // reconstruction would have to know whether that has happened yet.
    let start = census(&lab.world, &spec);
    println!("\n--- frame 0: the bed before anything grows ---");
    line(&start);

    println!("\n--- growing for {grow} frames ---");
    let mut history = vec![start.clone()];
    let mut repeat_culls = 0usize;
    for f in 1..=grow {
        tick(&mut lab);
        if cull_every > 0 && f % cull_every == 0 {
            let n = cull_half(&mut lab.world);
            repeat_culls += n;
            println!("  frame {f:>7}: culled {n} (half the stand)");
        }
        if f % every == 0 {
            let c = census(&lab.world, &spec);
            line(&c);
            history.push(c);
        }
    }

    let at_cull = census(&lab.world, &spec);
    let killed = if culling > 0 {
        let n = cull_all(&mut lab.world);
        println!("\n--- frame {}: CULLED {n} plants ---", at_cull.frame);
        n
    } else {
        println!("\n--- frame {}: no cull (cull=0) -- the bed runs on its own life cycle ---", at_cull.frame);
        0
    };
    line(&at_cull);

    println!("\n--- rotting for {rot} frames ---");
    for f in 1..=rot {
        tick(&mut lab);
        if f % every == 0 {
            let c = census(&lab.world, &spec);
            line(&c);
            history.push(c);
        }
    }
    let end = census(&lab.world, &spec);

    if repeat_culls > 0 {
        println!("\n(repeated culls over the run: {repeat_culls} organisms)");
    }
    report(&start, &at_cull, &end, &history, killed, &control, yield_arg, culling);
    if let Some(prefix) = png {
        let crop: Option<String> = arg("crop");
        let crop = crop.map(|c| {
            let v: Vec<u32> = c.split(',').map(|n| n.parse().expect("crop=x,y,w,h")).collect();
            (v[0], v[1], v[2], v[3])
        });
        let mark: u64 = arg("mark").unwrap_or(0);
        shoot(&mut lab, &format!("{prefix}.png"), crop, arg("zoom").unwrap_or(3), mark > 0);
    }
}

/// One frame of the bed, through the shipped renderer, with no window.
///
/// The lab draws its air as a room (`sky::Interior`), so a contact sheet of
/// this bed is legible in a way an outdoor crop is not -- and the review
/// queue's own note is that the stills the owner can judge are 700-950 px
/// across, so the frame is upscaled by an integer factor with nearest
/// neighbour rather than smoothed.
fn shoot(lab: &mut Lab, path: &str, crop: Option<(u32, u32, u32, u32)>, zoom: u32, mark: bool) {
    let (w, h) = (pixel_physics::lab::WIDTH, pixel_physics::lab::HEIGHT);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let touched = lab.world.take_touched_chunks();
    lab.renderer.draw(&lab.world, &lab.particles, &touched, &mut buf, (w, h), true);
    // **`mark=1` paints every cell the cycle cannot move again.**
    //
    // Not decoration: `Reports/plant-appearance-design.md`'s lesson is that
    // soil, litter and deadwood are one mid-brown speckle at the zoom a card
    // is judged at, and a plain before/after of this change is two pictures of
    // brown. The count says whether it fired; this says *where*, which is the
    // half a number cannot give.
    //
    // A **full replace** on a fixed colour, never a blend into the cell's own
    // — `CLAUDE.md`'s overlay rule, from a canopy-density sheet that read as
    // blank because a mid-range value moved one colour byte from 139 to 155
    // against a brown background. Magenta because nothing in this bed is near
    // it, and `litter_probe`'s overlay already uses it for the same job.
    if mark {
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let m = lab.world.get(x, y).material;
                if m == pixel_physics::sim::material::EMPTY {
                    continue;
                }
                let def = lab.world.materials.get(m);
                // The same data test the ledger's `locked` column uses, so the
                // picture and the number cannot disagree: a debris material
                // with nowhere to go.
                if def.decays_into.is_none() && NECROMASS.contains(&def.name.as_str()) {
                    let i = ((y as u32 * w + x as u32) * 4) as usize;
                    buf[i..i + 4].copy_from_slice(&[255, 0, 200, 255]);
                }
            }
        }
    }
    // **The crop is what makes the picture judgeable, not a nicety.**
    // `Reports/instruments.md` records the same finding for `litter_probe`:
    // the answer here is read by eye, and a whole 512x320 bed shrinks the
    // one interesting band -- the few rows either side of the ground line --
    // to a smear a couple of pixels tall. Defaults to the surface band.
    let (cx, cy, cw, ch) = crop.unwrap_or((128, 140, 256, 64));
    let (cw, ch) = (cw.min(w - cx), ch.min(h - cy));
    let (zw, zh) = (cw * zoom, ch * zoom);
    let mut big = vec![0u8; (zw * zh * 4) as usize];
    for y in 0..zh {
        for x in 0..zw {
            let src = (((cy + y / zoom) * w + (cx + x / zoom)) * 4) as usize;
            let dst = ((y * zw + x) * 4) as usize;
            big[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
        }
    }
    image::save_buffer(path, &big, zw, zh, image::ColorType::Rgba8).expect("writing the bed");
    println!("wrote {path} ({zw}x{zh}, crop {cx},{cy},{cw},{ch} at {zoom}x)");
}

#[allow(clippy::too_many_arguments)]
fn report(
    start: &Ledger,
    cull: &Ledger,
    end: &Ledger,
    history: &[Ledger],
    killed: usize,
    control: &str,
    yield_arg: Option<f32>,
    culling: u64,
) {
    println!("\n=== the ledger ===");
    // **`to_soil`, not `to_solid`.** A `deadleaf` decaying into `litter`
    // leaves a solid behind and produces no soil at all, so the raw counter
    // answers "how many decays left something" rather than "how much came
    // back" -- measured here at 34% against a true 8%, a fourfold
    // overstatement. `rotted_onward` is that intermediate step and comes off
    // the top.
    // **Whole-run, not cull-to-end, and that is what closes the cohort.** A
    // seed bank germinates during the rot phase -- 278 seeds sprouted in the
    // first measured run -- so plant mass created *after* the cull rots into
    // the same pool while sitting outside a cull-time denominator. Counting
    // from frame 0 puts every cell that ever entered the pool on both sides
    // of the fraction, which is the only version of this number that is
    // closed in a sealed box.
    //
    // **`to_soil`, not `to_solid`.** A `deadleaf` decaying into `litter`
    // leaves a solid behind and returns nothing, so the raw counter answers
    // "how many decays left something" rather than "how much came back" --
    // it read 34% against a true 8% on the first run of this harness, a
    // fourfold overstatement. `rotted_onward` is that intermediate step and
    // comes off the top; it is also why the denominator is built from
    // **terminal** rolls, since a cell that goes deadleaf -> litter -> soil
    // must be counted once and not twice.
    let to_soil = end.to_solid - end.onward;
    let to_nothing = end.to_nothing;
    let standing = end.necro_total();
    // Every cell that has ever entered the dead-organic pool and either
    // resolved or is still sitting in it.
    let entered = to_soil as usize + to_nothing as usize + standing;
    let frac = |n: usize| n as f64 / entered.max(1) as f64 * 100.0;

    println!("  of {entered} cells that entered the pool over the whole run:");
    println!("    -> soil, the return              {:>7}  {:>5.1}%", to_soil, frac(to_soil as usize));
    println!("    -> nothing, rot's discard        {:>7}  {:>5.1}%", to_nothing, frac(to_nothing as usize));
    println!(
        "    -> locked, no decays_into        {:>7}  {:>5.1}%   {:?}",
        end.locked,
        frac(end.locked),
        end.necro
    );
    let draining = standing - end.locked;
    println!("    -> still draining                {:>7}  {:>5.1}%", draining, frac(draining));
    println!("    -> still standing as plant       {:>7}  (outside the pool)", end.plant);

    // The chain-ending yield, measured rather than read off the asset file.
    let terminal = to_soil + to_nothing;
    let step = if terminal > 0 { to_soil as f64 / terminal as f64 } else { f64::NAN };
    println!(
        "\n  decay step        : {to_soil} of {terminal} chain-ending rot rolls left soil = {:.2}%  \
         ({} more were the deadleaf -> litter step, which returns nothing)",
        step * 100.0,
        end.onward
    );
    println!(
        "  mass returned     : {:.2}% of everything that has died reached soil; {:.1}% is locked for good",
        frac(to_soil as usize),
        frac(end.locked)
    );

    // The sealed-box bottom line, which needs no cohort at all.
    let bed = end.mineral as i64 - start.mineral as i64;
    println!(
        "  bed balance       : mineral {} -> {} = {}{} cells over one cycle ({:+.2}%)",
        start.mineral,
        end.mineral,
        if bed >= 0 { "+" } else { "" },
        bed,
        bed as f64 / start.mineral.max(1) as f64 * 100.0
    );
    println!(
        "  cohort at the cull: {} plant cells in {killed} plants; {} pool cells already standing",
        cull.plant,
        cull.necro_total()
    );

    println!("\n=== controls ===");
    let mut all = true;
    let mut ok = |name: &str, pass: bool, said: String| {
        println!("  [{}] {name}: {said}", if pass { "PASS" } else { "FAIL" });
        all &= pass;
    };

    // **The plateau check.** `CLAUDE.md`'s cascade rule: a census taken
    // before the pool has finished rotting reads a delay as a loss. Read the
    // quantity that has stopped moving across two consecutive stops.
    let tail: Vec<&Ledger> = history.iter().rev().take(2).collect();
    if let [last, prev] = tail[..] {
        let moved = last.mineral.abs_diff(prev.mineral);
        ok(
            "the mineral bed has plateaued",
            moved * 200 <= prev.mineral.max(1),
            format!("last two stops {} -> {} ({moved} cells, want <=0.5%)", prev.mineral, last.mineral),
        );
        // **Either the pool has stopped, or there is too little left in it to
        // move the answer** -- and the second clause is not slack, it is what
        // makes the check well conditioned at the tail. Stated as a bare 5%
        // relative test it demanded that a pool of 30 cells move by at most
        // one, so it failed every arm in which the pool had very nearly
        // drained: it was strictest exactly where it had least to say. The
        // question it exists for is whether a census caught the pool
        // mid-drain and read a delay as a loss (`CLAUDE.md`'s cascade rule),
        // and a residue under 2% of everything that ever entered cannot.
        let pool_moved = last.necro_total().abs_diff(prev.necro_total());
        let entered_so_far = (last.to_solid - last.onward) as usize + last.to_nothing as usize + last.necro_total();
        let negligible = last.necro_total() * 50 <= entered_so_far;
        ok(
            "the pool has stopped draining",
            pool_moved * 20 <= prev.necro_total().max(1) || negligible,
            format!(
                "last two stops {} -> {} ({pool_moved} cells); {} left of {entered_so_far} ever entered",
                prev.necro_total(),
                last.necro_total(),
                last.necro_total()
            ),
        );
    }

    // **The root-sink prediction.** A positive control on the reading of the
    // mechanism rather than on the instrument: roots occupy a bed cell by
    // overwriting it, so between frame 0 and the cull the mineral bed should
    // fall by about the plant tissue standing below the ground line. A large
    // residual means the sink is somewhere this reasoning has not looked, and
    // the ledger's story about *why* the bed runs down is wrong even if its
    // arithmetic is right.
    // **The root-sink prediction**, a positive control on the *reading* of
    // the mechanism rather than on the instrument. Roots occupy a bed cell by
    // overwriting it, so every root cell standing below the ground line came
    // out of the mineral bed -- and the only thing putting cells back is rot.
    // So `bed fell + soil produced` must be at least the standing root count.
    //
    // Stated as an inequality rather than as an equality, and the first
    // version was the equality: it failed in two of three arms because
    // `plant_below` counts the roots *surviving* at the cull, while a root
    // that grew and died earlier in the run took its bed cell just as
    // permanently. The equality was not measuring a defect, it was asserting
    // a model that undercounts its own left-hand side by construction.
    if cull.plant_below > 0 {
        let fell = start.mineral as i64 - cull.mineral as i64;
        let made = (cull.to_solid - cull.onward) as i64;
        ok(
            "roots came out of the bed",
            fell + made >= cull.plant_below as i64,
            format!(
                "bed fell {fell} + {made} soil made during growth = {} against {} standing root cells",
                fell + made,
                cull.plant_below
            ),
        );
    }

    match control {
        // **Specificity.** Nothing alive, so nothing can die: every ledger
        // figure must be zero. And the mineral bed must hold still, which is
        // the oscillator check on the exact quantity the bed balance reads.
        "empty" => {
            ok("no plants", cull.plants == 0, format!("plants {}", cull.plants));
            ok("nothing was culled", killed == 0, format!("{killed} culled"));
            ok("no plant tissue", end.plant == 0, format!("{} cells", end.plant));
            ok("no rot rolls", terminal + end.onward == 0, format!("{} rolls", terminal + end.onward));
            ok(
                "the mineral bed did not move",
                bed.abs() * 1000 <= start.mineral as i64,
                format!("{bed:+} cells of {} (want within 0.1%)", start.mineral),
            );
        }
        _ if culling == 0 => {
            // The uncontrolled arm. Its claim is about a bed that turned
            // over, so the control is that it *did* -- a run-down measured on
            // one cohort that never bred is a statement about that cohort.
            let gen = history.iter().map(|h| h.generation).max().unwrap_or(0);
            ok("the stand turned over", gen >= 2, format!("deepest plant generation {gen}"));
            ok("the decay channel fired", terminal > 0, format!("{terminal} chain-ending rolls"));
        }
        _ => {
            ok("something was culled", killed > 0, format!("{killed} plants"));
            ok("the plant cohort had mass", cull.plant > 0, format!("{} cells", cull.plant));
            // **The channel fired.** `World::rotted_to_solid`'s own doc names
            // this: a run with standing litter and no rot rolls at all reads
            // identically to a working channel with a low yield, if you only
            // census soil.
            ok("the decay channel fired", terminal > 0, format!("{terminal} chain-ending rolls"));
        }
    }

    // **Sensitivity.** The number must be able to move. At full yield every
    // rot roll leaves a solid; at zero, none does. Without this arm a 5%
    // return is not evidence of a 5% yield -- it is equally consistent with
    // an instrument that cannot see soil production at all.
    match yield_arg {
        Some(y) if y >= 1.0 => {
            ok("full yield returns everything", step > 0.99, format!("{:.2}%", step * 100.0));
            ok("nothing rots to nothing", to_nothing == 0, format!("to_nothing {to_nothing}"));
            ok("the bed gains", bed > 0, format!("{bed:+} cells"));
        }
        Some(y) if y <= 0.0 => {
            ok("zero yield returns nothing", to_soil == 0, format!("to_soil {to_soil}"));
            ok("the rolls still happened", to_nothing > 0, format!("to_nothing {to_nothing}"));
        }
        _ => {}
    }

    if !end.unclassified.is_empty() {
        ok("every standing material is classified", false, format!("{:?}", end.unclassified));
    }

    println!("VERDICT: {}", if all { "controls held" } else { "A CONTROL FAILED" });
}
