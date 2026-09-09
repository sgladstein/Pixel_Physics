//! **Is the colony starving because the bed's food is gone, or because it
//! never gets to it?** — the separator `open-bugs-handoff.md` §Z6 asks for.
//!
//! §Z6 censused nine 300,000-frame runs and found every shipped bed's colony
//! dead of starvation with the stand still standing, and said in as many
//! words that its own census could not tell the two apart:
//!
//! > Whether the colony *overgrazes* (eats the stand faster than it regrows
//! > while it lives) or *cannot reach* what is there ... are different bugs
//! > with different fixes, and this census cannot tell them apart.
//!
//! The three instruments it named cannot either, and the reason is the
//! scene rather than the counters: `forage_probe` builds a hand-made bank
//! with a food pile at a fixed gap, `stamp_probe` runs outdoor terrain, and
//! `labnest`'s columns are about the *nest* — it reports `roofed`, `packed`
//! and `buried` and has no food census at all. Nothing measures the shipped
//! bed's larder.
//!
//! # What separates them, and why it is a *cumulative* mask
//!
//! A standing count of food cannot answer this. Food is standing in *both*
//! worlds — an overgrazed bed still holds wood, and an unreachable larder is
//! by definition untouched — so `edible` on its own is the same number for
//! opposite findings. What differs is **whether the colony was ever there**:
//!
//! * **Overgrazing** — the ants walk the bed, eat what they find, and the
//!   standing food falls below the unfed control. `visited` covers the bed;
//!   `edible` is far under `colonies=0`.
//! * **Cannot reach** — the standing food is at or near the unfed control's
//!   level, and it is standing in columns no ant ever entered, or at heights
//!   no ant ever climbed to.
//!
//! So the column that decides it is `unvisited` — edible cells standing in a
//! column the colony has *never* occupied over the whole run — beside
//! `eaten` and beside the same bed at `colonies=0`.
//!
//! **The mask is over columns, not cells, and that is deliberate.** An ant
//! reaches into its head's 8-neighbourhood from the surface it walks on, and
//! the surface drifts as litter piles; a cell-exact mask would score a leaf
//! one row above a track as unreached, which is a statement about the
//! bookkeeping and not about the animal. A column mask is the *generous*
//! reading — it credits the colony with everything in any column it ever
//! walked — so a large `unvisited` is a finding that had to survive the
//! reading most likely to erase it.
//!
//! # Controls, in the binary
//!
//! * `colonies=0` — the unfed stand. `eaten` must be 0 and `edible` is the
//!   ceiling every fed run is read against. This is the specificity half.
//! * `handout=N` — a food cell dropped on the colony's doorstep every N
//!   frames, `windfall_probe`'s positive control. Supply without travel: if
//!   the colony lives with it and dies without it, the failure is delivery.
//! * `control=selftest` — the sensitivity half, and the one this file would
//!   be untrustworthy without. It plants a known count of known food at a
//!   known height and a known distance and asserts every band reports it,
//!   because a census reading zero because nothing is there and one reading
//!   zero because it is blind are the same line of output.
//!
//! ```text
//! cargo run --release --example labforage -- frames=300000 seed=1
//! cargo run --release --example labforage -- frames=300000 seed=1 colonies=0
//! cargo run --release --example labforage -- frames=300000 founders=256 colonies=3
//! cargo run --release --example labforage -- control=selftest
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::creature::{diet_yield, EAT_YIELD_THRESHOLD};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::organism::TRAIT_GUT_BIAS;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// How far above the soil surface a cell still counts as **on the floor**.
///
/// Same value and same reasoning as `windfall_probe`'s: an ant is a two-cell
/// chain standing on the ground reaching into its head's 8-neighbourhood, so
/// a couple of rows above the surface is food it takes without climbing, and
/// litter drifts the walking surface upward over a run. Generous on purpose —
/// the finding this harness exists to make is about food out of reach, and a
/// generous floor band makes that finding harder to get rather than easier.
const FLOOR_BAND: i32 = 3;

/// Above this many rows off the soil, food is up a stem rather than on the
/// ground. Between the two bands is what an ant reaches by climbing a little.
const LOW_BAND: i32 = 16;

/// Distance bands from the nearest nest column, in cells. `larder_probe`'s
/// shape: a quantity present in the world and a quantity concentrated where
/// the animals are are different findings, and only a banded census separates
/// them.
const DIST_BANDS: [i32; 4] = [16, 48, 128, i32::MAX];

#[derive(Default, Clone, Copy)]
struct Sample {
    /// Standing cells this gut would eat, by the mouth's own predicate.
    edible: usize,
    /// ...of those, within `FLOOR_BAND` rows of the soil surface.
    floor: usize,
    /// ...between `FLOOR_BAND` and `LOW_BAND` — reachable by a short climb.
    low: usize,
    /// ...above `LOW_BAND` — up a stem.
    aloft: usize,
    /// ...standing in a column no ant has occupied at any point in the run.
    unvisited: usize,
    /// Face value of every edible cell standing, in joules, at this gut.
    worth: f64,
    /// Edible cells by distance band from the nearest nest column.
    by_dist: [usize; DIST_BANDS.len()],
    ants: usize,
    /// Deepest row above the soil any ant head is standing at.
    ant_high: i32,
}

/// **What is standing here that this gut would eat, and where.**
///
/// Priced through `creature::diet_yield` and gated on
/// `EAT_YIELD_THRESHOLD` — the mouth's own predicate, called rather than
/// re-derived, because a readout that decides for itself what counts as food
/// is the standing house failure (`food_value`'s own doc records the eat verb
/// and the overlay disagreeing about what a cell is worth).
///
/// Swept over the grid rather than over the organism registry, unlike
/// `windfall_probe`'s: loose litter, corpses, fallen leaves and spoil are not
/// organism-owned and are exactly the food a walking ant meets. The sweep is
/// 512x320 per sample and the default interval is 900 frames.
fn census(world: &World, spec: &LabBox, gut: f32, visited: &[bool], nest_cols: &[i32]) -> Sample {
    let mut s = Sample { ant_high: i32::MIN, ..Sample::default() };
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_some() {
            s.ants += 1;
            if let Some(&(_, hy)) = state.chain.first() {
                s.ant_high = s.ant_high.max(spec.ground_y - hy);
            }
        }
    }
    if s.ant_high == i32::MIN {
        s.ant_high = 0;
    }
    for y in 0..spec.height {
        for x in 0..spec.width {
            let cell = world.get(x, y);
            let yielded = diet_yield(world, cell, gut);
            if yielded <= EAT_YIELD_THRESHOLD {
                continue;
            }
            // A living ant is not larder. The colony does not eat its own
            // (`is_living_kin`), and counting standing ants as food would put
            // the answer inside the question — a bed whose colony is dying
            // would read as a bed getting richer.
            if world.organism(cell.organism_id()).is_some_and(|st| world.species.get(st.species).creature.is_some()) {
                continue;
            }
            s.edible += 1;
            s.worth += yielded as f64;
            let above = spec.ground_y - y;
            if above <= FLOOR_BAND {
                s.floor += 1;
            } else if above <= LOW_BAND {
                s.low += 1;
            } else {
                s.aloft += 1;
            }
            if !visited[x.clamp(0, spec.width - 1) as usize] {
                s.unvisited += 1;
            }
            let d = nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX);
            for (b, &edge) in DIST_BANDS.iter().enumerate() {
                if d <= edge {
                    s.by_dist[b] += 1;
                    break;
                }
            }
        }
    }
    s
}

/// Every column any ant head has stood in, ever. Cumulative — the mask is
/// never cleared, so `unvisited` is a claim about the whole run rather than
/// about this frame.
fn mark_visited(world: &World, visited: &mut [bool], width: i32) {
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_none() {
            continue;
        }
        for &(x, _) in &state.chain {
            if (0..width).contains(&x) {
                visited[x as usize] = true;
            }
        }
    }
}

fn main() {
    let control: String = arg("control").unwrap_or_else(|| "run".to_string());
    let frames: u64 = arg("frames").unwrap_or(300_000);
    let sample_every: u64 = arg("sample").unwrap_or(900);
    let handout: u64 = arg("handout").unwrap_or(0);
    let spec = LabBox {
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        // `labstats`' default, so a run here and a run there are the same bed.
        soil_depth: arg("soil").unwrap_or(80),
        founders: arg("founders").unwrap_or(8),
        colonies: arg("colonies").unwrap_or(1),
        compartments: arg("walls").unwrap_or(1),
        seed: arg("seed").unwrap_or(1),
        ..LabBox::default()
    };
    if control == "selftest" {
        return selftest(spec);
    }
    // Echo the parameters. A knob nobody can see the value of is a knob
    // nobody can tell is disconnected -- `plant_probe`'s 3.5-hour lesson.
    println!(
        "labforage: frames={frames} sample={sample_every} founders={} colonies={} walls={} soil={} seed={} handout={handout}",
        spec.founders, spec.colonies, spec.compartments, spec.soil_depth, spec.seed
    );

    // Built bare and founded afterwards, for `windfall_probe`'s reason: a
    // species-level write after the founders are standing reaches nobody,
    // because `place_creature` copies the traits at placement.
    let bare = LabBox { colonies: 0, ..spec.clone() };
    let (mut world, planted) = bare.build_counted();
    let nest_cols = spec.colony_columns();
    let mut ants_placed = 0usize;
    for &x in &nest_cols {
        ants_placed += world.found_colony(x, spec.ground_y - 2);
    }
    // Read off a live founder, never off the species table: the run has to be
    // measuring the gut it says it is.
    let gut = world
        .live_organism_ids()
        .iter()
        .filter_map(|id| world.organism(*id))
        .find(|s| world.species.get(s.species).creature.is_some())
        .map(|s| s.traits[TRAIT_GUT_BIAS])
        .unwrap_or(0.0);
    println!(
        "  bed: {} of {} founders planted, {ants_placed} ants in {} colony/colonies at {nest_cols:?}, founder gut_bias {gut}",
        planted.planted, planted.asked, spec.colonies
    );
    println!(
        "  bands: floor <= {FLOOR_BAND} rows above soil, low <= {LOW_BAND}, aloft above that | distance bands {DIST_BANDS:?}\n"
    );

    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let mut visited = vec![false; spec.width as usize];
    let windfall_id = world.materials.id_of("windfall");
    let mut handed_out = 0u64;
    let mut first: Option<Sample> = None;
    let mut last = Sample::default();
    let mut peak_edible = 0usize;

    println!(
        "{:>7} {:>5} {:>7} {:>10} {:>6} {:>6} {:>6} {:>9} | {:>5} {:>5} {:>5} {:>5} | {:>4} {:>5} {:>5} {:>6}",
        "frame", "ants", "edible", "worth(J)", "floor", "low", "aloft", "unvisited",
        "d<16", "d<48", "d<128", "far", "high", "eats", "born", "died"
    );
    for f in 0..=frames {
        mark_visited(&world, &mut visited, spec.width);
        if f % sample_every == 0 {
            let s = census(&world, &spec, gut, &visited, &nest_cols);
            peak_edible = peak_edible.max(s.edible);
            if first.is_none() {
                first = Some(s);
            }
            last = s;
            let st = world.creature_stats;
            // One line per sample and every column on it, so the whole run is
            // one greppable block rather than a shape that has to be reread.
            println!(
                "{f:>7} {:>5} {:>7} {:>10.0} {:>6} {:>6} {:>6} {:>9} | {:>5} {:>5} {:>5} {:>5} | {:>4} {:>5} {:>5} {:>6}",
                s.ants, s.edible, s.worth, s.floor, s.low, s.aloft, s.unvisited,
                s.by_dist[0], s.by_dist[1], s.by_dist[2], s.by_dist[3],
                s.ant_high, st.eats, st.births, st.deaths
            );
        }
        if handout > 0 && f > 0 && f % handout == 0 {
            if let Some(wid) = windfall_id {
                let x = nest_cols[(handed_out as usize) % nest_cols.len().max(1)];
                for dy in 1..=6 {
                    let y = spec.ground_y - dy;
                    if world.get(x, y).material == pixel_physics::sim::material::EMPTY {
                        world.set(x, y, Cell::new(wid, 0));
                        handed_out += 1;
                        break;
                    }
                }
            }
        }
        if f < frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }
    }

    let st = world.creature_stats;
    let l = &world.energy_ledger;
    let cols = visited.iter().filter(|v| **v).count();
    let burn = l.metabolized + l.moved + l.synapse_tax;
    println!("\n  the colony's reach over {frames} frames:");
    println!(
        "    columns ever walked   {cols} of {} ({:.0}% of the bed's width)",
        spec.width,
        100.0 * cols as f64 / spec.width as f64
    );
    println!("    highest ant head      {} rows above the soil at the last sample", last.ant_high);
    println!(
        "    edible standing now   {} cells ({:.0} J), peak over the run {peak_edible}",
        last.edible, last.worth
    );
    println!(
        "    ...of which unvisited {} cells ({:.0}% of what is standing) -- food in a column no ant ever entered",
        last.unvisited,
        if last.edible > 0 { 100.0 * last.unvisited as f64 / last.edible as f64 } else { 0.0 }
    );
    println!("    ...by height          floor {} low {} aloft {}", last.floor, last.low, last.aloft);
    println!(
        "\n  what the colony took: eats {} pickups {} | harvested plant {:.0} J corpse {:.0} J against burn {:.0} J",
        st.eats, st.pickups, l.harvested_plant, l.harvested_corpse, burn
    );
    println!("  animals: born {} died {} alive {} | handouts placed {handed_out}", st.births, st.deaths, last.ants);
    println!(
        "SUMMARY seed={} founders={} colonies={} frames={frames} handout={handout} cols={cols} edible={} unvisited={} floor={} aloft={} \
         peak_edible={peak_edible} eats={} born={} died={} alive={} intake={:.0} burn={:.0} shares={} shared_j={:.0} moves={}",
        spec.seed, spec.founders, spec.colonies, last.edible, last.unvisited, last.floor, last.aloft,
        st.eats, st.births, st.deaths, last.ants, l.harvested_plant + l.harvested_corpse, burn, st.shares, st.shared_j, st.moves
    );
}

/// **The sensitivity half.** A census reading zero because there is nothing
/// there and one reading zero because it is blind print the same line, and
/// `CLAUDE.md`'s standing remedy is to construct the case whose answer is
/// known and watch the instrument report it.
///
/// Plants windfall (960 J face, well clear of the threshold at any gut near
/// neutral) at a known height and a known distance from the nest and asserts
/// every band moves: the total, the height band, the distance band and the
/// visited mask. Each assert is a band that could otherwise be silently
/// always-zero -- which is exactly what `aloft` and `unvisited` would look
/// like on a bed where the finding is real.
fn selftest(spec: LabBox) {
    let bare = LabBox { colonies: 0, founders: 0, ..spec.clone() };
    let (mut world, _) = bare.build_counted();
    let nest_cols = spec.colony_columns();
    let nest = nest_cols[0];
    let wid = world.materials.id_of("windfall").expect("windfall material");
    let visited_none = vec![false; spec.width as usize];
    let mut visited_all = vec![true; spec.width as usize];

    let base = census(&world, &spec, 0.0, &visited_none, &nest_cols);
    println!("labforage selftest: empty bed reads edible {} (must be 0)", base.edible);
    assert_eq!(base.edible, 0, "an unplanted bed is not food; the census is counting something it should not");

    // One cell on the floor beside the nest, one 40 rows up and 200 columns
    // away. The two differ in every band the run's finding turns on.
    let near = (nest + 2, spec.ground_y - 1);
    let far_x = (nest + 200).min(spec.width - 6);
    let far = (far_x, spec.ground_y - 40);
    world.set(near.0, near.1, Cell::new(wid, 0));
    world.set(far.0, far.1, Cell::new(wid, 0));

    let s = census(&world, &spec, 0.0, &visited_none, &nest_cols);
    println!(
        "  planted 2 cells (one at the nest on the floor, one {} columns out and 40 rows up): \
         edible {} floor {} low {} aloft {} unvisited {} by_dist {:?} worth {:.0} J",
        far.0 - nest, s.edible, s.floor, s.low, s.aloft, s.unvisited, s.by_dist, s.worth
    );
    assert_eq!(s.edible, 2, "the census must see both planted cells");
    assert_eq!(s.floor, 1, "the floor band must see the cell on the floor and only it");
    assert_eq!(s.aloft, 1, "the aloft band must see the cell 40 rows up -- otherwise `aloft 0` means nothing");
    assert_eq!(s.unvisited, 2, "with no column visited, every cell must read unvisited");
    assert!(s.by_dist[0] >= 1, "the near band must see the cell 2 columns from the nest");
    assert!(s.by_dist[2] >= 1 || s.by_dist[3] >= 1, "the far cell must land in a far distance band, not the near one");
    assert!(s.worth > 0.0, "food priced at zero is not food; diet_yield is not reaching the census");

    // ...and the mask has to be able to go the other way, or `unvisited`
    // would be a constant wearing a measurement's clothes.
    visited_all[..].fill(true);
    let s2 = census(&world, &spec, 0.0, &visited_all, &nest_cols);
    println!("  same bed with every column marked visited: unvisited {} (must be 0)", s2.unvisited);
    assert_eq!(s2.unvisited, 0, "the visited mask does not reach the census");
    assert_eq!(s2.edible, 2, "the mask must not change what is counted as food");

    // A gut that cannot digest plants must stop seeing them -- the predicate
    // is the mouth's, so this is the check that the census asks the mouth.
    let s3 = census(&world, &spec, 1.0, &visited_none, &nest_cols);
    println!("  same bed read at a pure-flesh gut (bias +1.0): edible {} (must be 0)", s3.edible);
    assert_eq!(s3.edible, 0, "a carnivore's census must not count plants; the gut is not reaching diet_yield");

    println!("labforage selftest: PASS -- every band moves for a case whose answer is known");
}
