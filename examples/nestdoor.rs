//! **Is the colony's door still there, and is anyone standing at it?**
//!
//! `open-bugs-handoff.md` §T2 is the round trip that closes for the founders
//! and then stops: on the played bed, seed 1, `deliveries` and `nest_visits`
//! are *identical* at 9,000, 30,000 and 120,000 frames while `pickups` scales
//! 358 -> 1,108 -> 5,058. A frozen `deliveries` has four candidate causes that
//! a summary line cannot tell apart, so this harness gives each one a counter
//! that can move only under it, and samples them in **one** run:
//!
//! - **the door is gone or buried** -- the nest patch censused every sample:
//!   how many of the cells first painted are still `nest`, what is standing on
//!   the ones that are not, how many have air beside them, and how many have a
//!   cell an ant could actually *stand* in next to them;
//! - **nobody reaches it** -- per window, how many distinct animals were
//!   8-adjacent to a nest cell at least once, and the peak number at one time;
//! - **the laden ant unloads en route** -- pickups / drops / deliveries per
//!   window, with the laden animals' mean Chebyshev distance to the nearest
//!   standing nest cell;
//! - **only the founders ever delivered** -- living animals' own
//!   `life.deliveries`, split generation 0 against the rest.
//!
//! Every counter column is a *delta over the window*; the door columns are
//! standing counts. `CLAUDE.md` asks for both: a standing complaint needs a
//! standing count, and "did it fire at all" needs a counter.
//!
//! ```text
//! cargo run --release --example nestdoor -- seed=1 frames=120000 every=10000
//! cargo run --release --example nestdoor -- control=selftest
//! ```
//!
//! **`control=selftest` is the positive control** and it is not optional.
//! `CLAUDE.md`: a number that is arithmetically correct and answers a
//! different question looks exactly like a result. It founds the colony, then
//! *by hand* buries one half of the patch under packed spoil and erases the
//! other half, and asserts that `covered` reads the buried half, `lost` the
//! erased half, and `stand` (cells with somewhere to stand beside them) falls.
//! A census that cannot report a door it was shown being buried cannot report
//! one the colony buries.
//!
//! It echoes its own parameters on the first line -- the megastudy gotcha.

use pixel_physics::lab::rain::Rain;
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::creature::DecisionRow;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}="))
            .map(|v| v.parse().ok().expect("parses"))
    })
}

const N8: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// What the door looks like right now.
struct Door {
    /// Of the cells first painted `nest`, how many still are.
    held: usize,
    /// ...and how many are not, with what is standing on them.
    lost: usize,
    lost_by: Vec<(String, usize)>,
    /// Standing nest cells with at least one empty neighbour -- air beside the
    /// door. Necessary for an ant to be adjacent; not sufficient.
    airy: usize,
    /// Standing nest cells with a neighbour an ant could *stand* in: empty,
    /// with something under it. This is the one that answers "can `AtNest`
    /// fire".
    stand: usize,
    /// Standing nest cells with a non-empty cell directly above.
    covered: usize,
    covered_by: Vec<(String, usize)>,
    /// **The specificity control for "the patch floods".** Free `Liquid`
    /// cells standing in the eight rows above each patch column, against the
    /// same count over the columns immediately outside the patch on either
    /// side. A nest that simply sits in a hollow would flood along with its
    /// neighbours; a nest that floods *because it is impermeable* floods
    /// alone. Without this pair the water column is a number about the
    /// terrain wearing a number about the nest.
    water_on: usize,
    water_off: usize,
}

/// Free liquid standing in the `LOOK_UP` rows above `(x, y)`.
const LOOK_UP: i32 = 8;
fn water_above(world: &World, x: i32, y: i32) -> usize {
    (1..=LOOK_UP)
        .filter(|dy| world.materials.kind(world.get(x, y - dy).material) == MaterialKind::Liquid)
        .count()
}

fn census_door(world: &World, patch: &[(i32, i32)]) -> Door {
    let nest = world.materials.id_of("nest");
    let mut d = Door {
        held: 0,
        lost: 0,
        lost_by: Vec::new(),
        airy: 0,
        stand: 0,
        covered: 0,
        covered_by: Vec::new(),
        water_on: 0,
        water_off: 0,
    };
    // The paired control, taken over the same number of columns: the band of
    // ordinary ground immediately outside each end of the patch, at the same
    // row the patch sits on.
    if let (Some(&(lx, ly)), Some(&(rx, ry))) = (patch.first(), patch.last()) {
        let half = (patch.len() as i32 + 1) / 2;
        for i in 1..=half {
            d.water_off += water_above(world, lx - i, ly) + water_above(world, rx + i, ry);
        }
    }
    let mut lost_hist: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut cover_hist: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for &(x, y) in patch {
        d.water_on += water_above(world, x, y);
        let m = world.get(x, y).material;
        if Some(m) != nest {
            d.lost += 1;
            *lost_hist.entry(world.materials.get(m).name.clone()).or_default() += 1;
            continue;
        }
        d.held += 1;
        let mut airy = false;
        let mut stand = false;
        for (dx, dy) in N8 {
            let (nx, ny) = (x + dx, y + dy);
            if world.is_empty(nx, ny) {
                airy = true;
                // A footing under it, the same shape the spoil drop's `open`
                // predicate uses for "a pellet would stay here": an ant
                // hanging in mid-air is not at the door.
                if !world.is_empty(nx, ny + 1) {
                    stand = true;
                }
            }
        }
        d.airy += usize::from(airy);
        d.stand += usize::from(stand);
        if !world.is_empty(x, y - 1) {
            d.covered += 1;
            *cover_hist
                .entry(world.materials.get(world.get(x, y - 1).material).name.clone())
                .or_default() += 1;
        }
    }
    d.lost_by = lost_hist.into_iter().collect();
    d.covered_by = cover_hist.into_iter().collect();
    d.lost_by.sort_by_key(|e| std::cmp::Reverse(e.1));
    d.covered_by.sort_by_key(|e| std::cmp::Reverse(e.1));
    d
}

/// **Is the founding cut still a way home?** What stands in every cell of
/// each nest site's founding cut (`ShaftFootprint::cells`), by material name,
/// with the empty cells first -- and how much of the dug home
/// (`World::nest_dug`, the open cells a walk from the door reaches) lies
/// nearest that site.
///
/// Added 2026-10-03 for the shut door the laying lane found on main with the
/// door rules in (PR 562): from about 80,000 frames no ant's head reaches
/// home and the dug home shrinks to 3-5 cells. The painted patch the
/// `Door` census reads is the lid over that shaft, not the shaft, so it could
/// not say what closed it.
/// `dump=1` on the command line: [`census_cuts`] prints the cut cell by cell.
fn dump_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| arg::<u8>("dump").unwrap_or(0) == 1)
}

fn census_cuts(world: &World) -> Vec<String> {
    let mut out = Vec::new();
    for (i, site) in world.nest_sites.iter().enumerate() {
        let Some(cut) = site.shaft else { continue };
        let mut hist: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        let cells = cut.cells();
        let mut open = 0;
        let mut mouth_open = 0;
        let mut mouth = 0;
        for &(x, y) in &cells {
            let m = world.get(x, y).material;
            if y <= cut.mouth_bottom && (cut.x0..=cut.x1).contains(&x) {
                mouth += 1;
                mouth_open += usize::from(m == material::EMPTY);
            }
            if m == material::EMPTY {
                open += 1;
            } else {
                *hist.entry(world.materials.get(m).name.clone()).or_default() += 1;
            }
        }
        let mut hist: Vec<(String, usize)> = hist.into_iter().collect();
        hist.sort_by_key(|e| std::cmp::Reverse(e.1));
        // The ground the pit is cut into: every water-holding cell beside the
        // cut, its mean moisture and how many are saturated. A pond stands in
        // a pit only once its walls can take no more.
        let inside: std::collections::HashSet<(i32, i32)> = cells.iter().copied().collect();
        let mut walls: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
        for &(x, y) in &cells {
            for (dx, dy) in N8 {
                let n = (x + dx, y + dy);
                if !inside.contains(&n) && world.materials.get(world.get(n.0, n.1).material).water_capacity > 0 {
                    walls.insert(n);
                }
            }
        }
        let wet: Vec<u16> = walls.iter().map(|&(x, y)| world.get(x, y).aux()).collect();
        let wall_mean = if wet.is_empty() {
            0.0
        } else {
            wet.iter().map(|&m| f64::from(m)).sum::<f64>() / wet.len() as f64
        };
        let wall_sat = wet.iter().filter(|&&m| m >= material::SOIL_SATURATED).count();
        let home = world
            .nest_dug
            .iter()
            .filter(|&&(x, _)| {
                world
                    .nest_sites
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, s)| (s.x - x).abs())
                    .map(|(j, _)| j)
                    == Some(i)
            })
            .count();
        // **Crater or mound?** The ground's height over the mouth's row,
        // every third column from 24 left of the cut to 24 right: the first
        // powder or solid cell from 30 rows over the mouth down (so not air,
        // water, an animal or a standing plant). Negative is below the founding surface. A door at the
        // bottom of a bowl is a sump; one on a summit sheds its rain.
        let mid = (cut.x0 + cut.x1) / 2;
        // From 30 rows over the mouth: higher than any heap, under the lid.
        let top_row = cut.top - 30;
        let profile: Vec<i32> = (-8..=8)
            .map(|k| {
                let x = mid + 3 * k;
                let mut y = top_row;
                while y < cut.bottom + 40 {
                    let m = world.get(x, y).material;
                    let kind = world.materials.kind(m);
                    if matches!(kind, MaterialKind::Powder | MaterialKind::Solid) {
                        break;
                    }
                    y += 1;
                }
                cut.top - y
            })
            .collect();
        out.push(format!(
            "          ground over the mouth row, x {}..{} step 3: {profile:?}",
            mid - 24,
            mid + 24
        ));
        // **`dump=1`: the cut cell by cell**, so a pond that never drains can
        // be read against the ground it sits on. Two grids over the cut and
        // three cells round it: what is there (`.` air, `~` water, `s` soil,
        // `p` packed soil, `o` spoil, `a` animal, `g` living plant, `#` other
        // solid, `*` anything else), and its wetness in tenths of full
        // (`X` saturated soil, `-` holds no water; water reads its fill).
        // Any other material prints as its initial in upper case, with a
        // legend.
        if dump_on() {
            let (gx0, gx1) = (cut.x0.min(cut.chamber_x0) - 3, cut.x1.max(cut.chamber_x1) + 3);
            let (gy0, gy1) = (cut.top - 6, cut.chamber_bottom.max(cut.bottom) + 8);
            let mut legend: std::collections::BTreeMap<char, String> = Default::default();
            for y in gy0..=gy1 {
                let mut what = String::new();
                let mut wet = String::new();
                for x in gx0..=gx1 {
                    let c = world.get(x, y);
                    let m = world.materials.get(c.material);
                    let kind = world.materials.kind(c.material);
                    what.push(match (kind, m.name.as_str()) {
                        _ if c.material == material::EMPTY => '.',
                        (MaterialKind::Liquid, _) => '~',
                        (_, "soil") => 's',
                        (_, "packedsoil") => 'p',
                        (_, "spoil") => 'o',
                        (MaterialKind::Creature, _) => 'a',
                        (MaterialKind::Plant, _) => 'g',
                        (MaterialKind::Solid, _) => '#',
                        // Anything else by its initial, upper-case, named in
                        // the legend line under the grid.
                        (_, name) => {
                            let ch = name.chars().next().unwrap_or('*').to_ascii_uppercase();
                            legend.insert(ch, name.to_string());
                            ch
                        }
                    });
                    wet.push(if kind == MaterialKind::Liquid {
                        char::from_digit(
                            u32::from(pixel_physics::sim::update::liquid_fill(c)) * 9
                                / u32::from(material::LIQUID_FULL),
                            10,
                        )
                        .unwrap_or('?')
                    } else if m.water_capacity == 0 {
                        '-'
                    } else if c.aux() >= material::SOIL_SATURATED {
                        'X'
                    } else {
                        char::from_digit(u32::from(c.aux()) * 10 / u32::from(material::SOIL_SATURATED), 10)
                            .unwrap_or('?')
                    });
                }
                out.push(format!("          dump y {y:>3} x {gx0}..{gx1}: {what}  {wet}"));
            }
            out.push(format!("          dump legend: {legend:?}"));
        }
        out.push(format!(
            "          cut {i} at x {}: dug home {home} | mouth open {mouth_open}/{mouth} | cut open {open}/{} | filled by {hist:?} | walls {} cells, moisture {wall_mean:.0}, saturated {wall_sat}",
            site.x,
            cells.len(),
            walls.len()
        ));
    }
    out
}

/// Is `(x, y)` home -- in or beside a dug home cell or the painted patch?
/// The laying lane's probe predicate (`/mnt/project-files/laying/anchor-probe/`),
/// so "reached home" means the same thing in both lanes.
fn at_home(world: &World, (x, y): (i32, i32), nest: Option<material::MaterialId>) -> bool {
    (-1..=1).any(|dy| {
        (-1..=1).any(|dx| {
            world.nest_dug.contains(&(x + dx, y + dy)) || nest.is_some_and(|m| world.get(x + dx, y + dy).material == m)
        })
    })
}

/// Every cell currently painted `nest`, wherever it is -- so a patch that
/// *moved* (a nest cell carried off as spoil and set down elsewhere) is not
/// read as a patch that vanished.
fn all_nest_cells(world: &World) -> Vec<(i32, i32)> {
    let Some(nest) = world.materials.id_of("nest") else {
        return Vec::new();
    };
    let Some(b) = world.bounds() else { return Vec::new() };
    let mut out = Vec::new();
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            if world.get(x, y).material == nest {
                out.push((x, y));
            }
        }
    }
    out
}

/// One living animal: its id, where its head is, its generation, its own
/// delivery count, and whether it is carrying anything. A named type because
/// the tuple is past the width clippy accepts inline, and because every
/// reader of it wants all five.
type Animal = (OrganismId, (i32, i32), u16, u32, bool);

/// Heads of every living animal, with generation, its own delivery count, and
/// whether it is carrying anything.
fn animals(world: &World) -> Vec<Animal> {
    world
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| {
            let st = world.organism(id)?;
            world.species.get(st.species).creature.as_ref()?;
            let &head = st.chain.first()?;
            Some((id, head, st.generation, st.life.deliveries, st.crop.is_some()))
        })
        .collect()
}

fn main() {
    let control: String = arg("control").unwrap_or_else(|| "run".to_string());
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(120_000);
    let every: u64 = arg("every").unwrap_or(10_000);
    let scenario: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    // **`rain=` is the control the door census cannot be read without.** If
    // what stands on the patch is free water rather than the colony's own
    // spoil, then turning the mister off has to clear it and nothing else
    // does -- `CLAUDE.md`'s "the control is to hold the semantic rule fixed,
    // not to add another metric". `Rain::from_index` (OFF/LIGHT/STEADY/HEAVY),
    // overriding the scenario's own setting, which is `waterstand`'s flag and
    // for the same reason.
    let rain: Option<u8> = arg("rain");
    println!(
        "nestdoor: control={control} scenario={scenario} seed={seed} frames={frames} every={every} rain={}",
        rain.map_or("-".to_string(), |r| r.to_string())
    );

    let mut sc = Scenario::load(&scenario).unwrap_or_else(|e| {
        eprintln!("scenario {scenario}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    let msg = lab.load_scenario(sc);
    if let Some(r) = rain {
        lab.spec.rain = Rain::from_index(r);
    }
    println!("  {msg} | rain {}", lab.spec.rain.label());
    // **Where does the water in a flooded door come from?** Three oracle
    // arms, each removing one route and nothing else, so the route whose
    // removal keeps the founding cut dry is the one that floods it:
    //
    // - `levels=1` turns on `World::soil_capillary_levels` (the bed's own
    //   "water levels sideways" dial, off by the owner's ruling) -- a wall the
    //   pit has soaked to saturation can then pass water on sideways;
    // - `umbrella=1` deletes every liquid cell still falling through the air
    //   over the mouth (its columns and one either side, from the room's top
    //   to three rows above the mouth) -- rain straight into the hole;
    // - `berm=1` sets two rows of stone either side of the mouth at founding
    //   -- runoff across the surface into it.
    //
    // Oracles, not proposals: none is a mechanism the game would ship.
    let levels = arg::<u8>("levels").unwrap_or(0) == 1;
    let umbrella = arg::<u8>("umbrella").unwrap_or(0) == 1;
    let berm = arg::<u8>("berm").unwrap_or(0) == 1;
    // `pump=1`: the ceiling for any water fix -- every liquid cell inside
    // the founding cut is deleted each frame, so the door can never hold a
    // pond. What the colony does with a door that cannot flood is the most
    // any route-specific fix could buy.
    let pump = arg::<u8>("pump").unwrap_or(0) == 1;
    // `wake=<k>`: every `k` frames every chunk is examined in full
    // (`World::wake_all`, the engine's own control for "the rules are wrong"
    // against "the sweep never looked"). If a pond that stands in the cut for
    // ever drains under this, it stood because nothing visited the ground
    // beside it, not because that ground was full.
    let wake = arg::<u64>("wake").unwrap_or(0);
    // `keepopen=1`: the door can never be shut -- every cell of the founding
    // cut holding water, a plant, or ground (soil, packed soil, spoil) is
    // deleted each frame. Brood, food, crumbs and corpses are left: the
    // first version deleted everything but air and animals, which emptied
    // the brood pile and the larder in the chamber and shrank the colony
    // (seed 1, 43 ants to 13 by frame 80,000) -- an oracle that starves the
    // colony answers a different question. Paired with the laying lane's
    // scratch probe `PIXEL_PHYSICS_BIRTH_ANCHOR=nest` (newborns homed at the
    // door; not on main, `/mnt/project-files/laying/anchor-probe/probe.diff`)
    // it separates "the door shuts" from "nobody knows where the door is";
    // the oracle line echoes that variable so a log says which it was.
    let keepopen = arg::<u8>("keepopen").unwrap_or(0) == 1;
    let mut kept_open: std::collections::BTreeMap<String, u64> = Default::default();
    // `mist=half`: the mister at half its LIGHT rate -- the box's own
    // `rain::tick` at LIGHT, called on every second due frame (every 80
    // ticks rather than 40) with the box's own rain turned off. The lever is
    // the box's water income, the one thing every route shares.
    let mist_half = arg::<String>("mist").as_deref() == Some("half");
    if mist_half {
        lab.spec.rain = Rain::Off;
    }
    let mut pumped = 0u64;
    if levels {
        lab.world.soil_capillary_levels = true;
    }
    println!("  oracles: levels={levels} umbrella={umbrella} berm={berm} pump={pump} mist_half={mist_half} wake={wake} keepopen={keepopen} birth_anchor={} dump={} powder_floats={}", std::env::var("PIXEL_PHYSICS_BIRTH_ANCHOR").unwrap_or_else(|_| "-".into()), dump_on(), std::env::var("PIXEL_PHYSICS_POWDER_FLOATS").unwrap_or_else(|_| "-".into()));
    let room_top = lab.spec.room_top();
    let mut umbrella_caught = 0u64;
    let mut bermed = false;
    // **`shots=<dir>`: a cutaway of the nest at every sample**, through the
    // lab's own renderer (`Lab::draw`, the pixels a player sees), camera
    // centred on the founding cut at `zoom=` (default 4) and cropped to the
    // world above the toolbar. Drawn after the census, so the picture is the
    // state the printed numbers describe. Files are `door_f<frame>.png`.
    let shots: Option<String> = arg("shots");
    let zoom: u32 = arg("zoom").unwrap_or(4).max(1);
    let mut camera_set = false;
    // **`anttrace=<file>`: every colony animal, every `antevery=` frames
    // (default 1,000)** -- see [`ant_rows`]. Scott's "why are they not in the
    // nest, why are they not digging", asked of individuals rather than of a
    // population: one row per animal per sample, and one `ANTS` line per
    // sample summing them. The engine's own decision log is switched on for
    // the `ANT_LOG_FRAMES` frames before each sample only, so the anchor and
    // the last outcome are each animal's latest decision.
    let ant_every: u64 = arg("antevery").unwrap_or(1_000).max(ANT_LOG_FRAMES + 1);
    let mut ant_out = arg::<String>("anttrace").map(|path| {
        use std::io::Write;
        let mut w = std::io::BufWriter::new(
            std::fs::File::create(&path).unwrap_or_else(|e| panic!("nestdoor: anttrace {path}: {e}")),
        );
        writeln!(w, "{ANT_HEADER}").expect("write anttrace header");
        w
    });
    let mut last_decision: std::collections::HashMap<OrganismId, DecisionRow> = Default::default();
    if let Some(dir) = &shots {
        std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("nestdoor: shots {dir}: {e}"));
        // The key list paints over the whole box outside the real binary.
        lab.show_help = false;
        // ...and the biosphere readout opens showing after a load.
        if lab.stats.showing() {
            lab.stats.toggle();
        }
        for _ in 1..zoom {
            lab.renderer.adjust_zoom(1);
        }
    }

    // The patch is painted when the colony is founded, which on the played
    // bed is frame 6,000 -- so it cannot be read before the run starts, and a
    // harness that sampled frame 0 would report "no nest" and stop.
    let mut patch: Vec<(i32, i32)> = Vec::new();
    let mut founded = false;
    // Per-window accumulators over *every* frame, not only the sample frames:
    // an ant at the door for one frame in ten thousand is the whole question.
    let mut seen_at_nest: std::collections::BTreeSet<OrganismId> = std::collections::BTreeSet::new();
    // Distinct animals whose head was home (`at_home`) at least once in the
    // window, and the most at once -- the coordinator's "did any ant reach
    // home" over time.
    let mut seen_home: std::collections::BTreeSet<OrganismId> = std::collections::BTreeSet::new();
    let mut peak_home = 0usize;
    let mut peak_at_nest = 0usize;
    let mut laden_dist_sum = 0f64;
    let mut laden_dist_n = 0u64;
    let mut closest = i32::MAX;
    let mut all_dist_sum = 0f64;
    let mut all_dist_n = 0u64;
    let mut prev = [0u64; 8];

    println!(
        "  {:>7} | {:>4} {:>4} {:>4} {:>4} | {:>5} {:>5} {:>5} {:>5} | {:>5} {:>5} | {:>5} {:>4} {:>4} {:>3} | {:>6}",
        "frame",
        "held",
        "lost",
        "stnd",
        "covd",
        "pickp",
        "drops",
        "deliv",
        "visit",
        "digs",
        "spoil",
        "alive",
        "born",
        "died",
        "gen",
        "atnest"
    );

    for f in 0..=frames {
        if !founded {
            let cells = all_nest_cells(&lab.world);
            if !cells.is_empty() {
                patch = cells;
                founded = true;
                let (x0, x1) = (
                    patch.iter().map(|c| c.0).min().unwrap_or(0),
                    patch.iter().map(|c| c.0).max().unwrap_or(0),
                );
                let (y0, y1) = (
                    patch.iter().map(|c| c.1).min().unwrap_or(0),
                    patch.iter().map(|c| c.1).max().unwrap_or(0),
                );
                // The centre is printed because a review card of the door has
                // to be aimed at it: `labgif center=x,y` takes exactly this.
                println!(
                    "  frame {f:>7}: nest patch painted, {} cells, x {x0}..{x1}, y {y0}..{y1}, centre {},{}",
                    patch.len(),
                    (x0 + x1) / 2,
                    (y0 + y1) / 2
                );
                if control == "selftest" {
                    return selftest(&mut lab, &patch);
                }
            }
        }
        // Per-frame, because "was anyone ever at the door in this window" is
        // not answerable from a sample every ten thousand frames.
        if founded {
            let nest = lab.world.materials.id_of("nest");
            let mut at_now = 0usize;
            let mut home_now = 0usize;
            for (id, (hx, hy), _gen, _del, laden) in animals(&lab.world) {
                if at_home(&lab.world, (hx, hy), nest) {
                    home_now += 1;
                    seen_home.insert(id);
                }
                if N8
                    .iter()
                    .any(|&(dx, dy)| Some(lab.world.get(hx + dx, hy + dy).material) == nest)
                {
                    at_now += 1;
                    seen_at_nest.insert(id);
                }
                // **How close did anyone get?** `stnd` says the door has
                // somewhere to stand beside it and `atnest` says whether
                // anyone did; neither separates "the colony never came near"
                // from "it came within one cell and something at the last
                // step refused". The minimum over the window does.
                if f % 20 == 0 {
                    if let Some(d) = patch
                        .iter()
                        .filter(|&&(px, py)| Some(lab.world.get(px, py).material) == nest)
                        .map(|&(px, py)| (px - hx).abs().max((py - hy).abs()))
                        .min()
                    {
                        closest = closest.min(d);
                        all_dist_sum += f64::from(d);
                        all_dist_n += 1;
                    }
                }
                if laden && f % 200 == 0 {
                    // Sampled rather than every frame: it is an O(patch) scan
                    // per laden animal, and the mean is what is read.
                    if let Some(d) = patch
                        .iter()
                        .filter(|&&(px, py)| Some(lab.world.get(px, py).material) == nest)
                        .map(|&(px, py)| (px - hx).abs().max((py - hy).abs()))
                        .min()
                    {
                        laden_dist_sum += f64::from(d);
                        laden_dist_n += 1;
                    }
                }
            }
            peak_at_nest = peak_at_nest.max(at_now);
            peak_home = peak_home.max(home_now);
        }

        if founded && f % ant_every == 0 && f % every != 0 && f != frames {
            if let Some(out) = ant_out.as_mut() {
                println!(
                    "{}",
                    ant_rows(&lab.world, lab.spec.ground_y, &patch, &last_decision, out, f)
                );
            }
        }
        if f % every == 0 || f == frames {
            let w = &lab.world;
            let s = &w.creature_stats;
            let now = [
                s.pickups,
                s.drops,
                s.deliveries,
                s.nest_visits,
                s.digs,
                s.spoil_dumped,
                s.births,
                s.deaths,
            ];
            let d = census_door(w, &patch);
            let live = animals(w);
            let gen = live.iter().map(|a| a.2).max().unwrap_or(0);
            let g0_del: u32 = live.iter().filter(|a| a.2 == 0).map(|a| a.3).sum();
            let gn_del: u32 = live.iter().filter(|a| a.2 > 0).map(|a| a.3).sum();
            println!(
                "  {f:>7} | {:>4} {:>4} {:>4} {:>4} | {:>5} {:>5} {:>5} {:>5} | {:>5} {:>5} | {:>5} {:>4} {:>4} {:>3} | {:>6}",
                d.held,
                d.lost,
                d.stand,
                d.covered,
                now[0] - prev[0],
                now[1] - prev[1],
                now[2] - prev[2],
                now[3] - prev[3],
                now[4] - prev[4],
                now[5] - prev[5],
                live.len(),
                now[6] - prev[6],
                now[7] - prev[7],
                gen,
                seen_at_nest.len(),
            );
            if founded {
                println!(
                    "          door: airy {} | standing water on the patch {} vs {} over the same width of ordinary ground beside it | covered by {:?} | lost to {:?} | at-nest peak {} | laden mean dist {:.1} over {} | every ant: mean {:.1}, closest anyone came {} | live deliveries gen0 {} / later {}",
                    d.airy,
                    d.water_on,
                    d.water_off,
                    d.covered_by,
                    d.lost_by,
                    peak_at_nest,
                    if laden_dist_n == 0 { 0.0 } else { laden_dist_sum / laden_dist_n as f64 },
                    laden_dist_n,
                    if all_dist_n == 0 { 0.0 } else { all_dist_sum / all_dist_n as f64 },
                    if closest == i32::MAX { -1 } else { closest },
                    g0_del,
                    gn_del
                );
            }
            if founded {
                // Water anywhere the colony ever dug, and the deepest dug row
                // holding it: a pond in the founding cut is the door shutting,
                // one in a gallery below it is the nest flooding from the
                // bottom up.
                let dug_water: Vec<(i32, i32)> = w
                    .dug_cells
                    .iter()
                    .copied()
                    .filter(|&(x, y)| w.materials.kind(w.get(x, y).material) == MaterialKind::Liquid)
                    .collect();
                let dug_deepest = w.dug_cells.iter().map(|c| c.1).max().unwrap_or(0);
                println!(
                    "          home: reached by {} animal(s) this window, at most {} at once | dug home {} cells over {} site(s) | ever dug {} cells, deepest row {}, water in them {} (rows {}..{})",
                    seen_home.len(),
                    peak_home,
                    w.nest_dug.len(),
                    w.nest_sites.len(),
                    w.dug_cells.len(),
                    dug_deepest,
                    dug_water.len(),
                    dug_water.iter().map(|c| c.1).min().unwrap_or(0),
                    dug_water.iter().map(|c| c.1).max().unwrap_or(0)
                );
                for line in census_cuts(w) {
                    println!("{line}");
                }
            }
            if let (Some(out), true) = (ant_out.as_mut(), founded && f % ant_every == 0) {
                println!("{}", ant_rows(w, lab.spec.ground_y, &patch, &last_decision, out, f));
            }
            if let (Some(dir), true) = (&shots, founded) {
                if let Some(cut) = lab.world.nest_sites.iter().find_map(|s| s.shaft) {
                    let (full_w, full_h) = (WIDTH, HEIGHT);
                    let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, lab.spec.width - 1, lab.spec.height - 1);
                    let (span_x, span_y) = lab.renderer.visible_span((full_w, full_h));
                    if !camera_set {
                        // Centred a little below the mouth: the room under the
                        // shaft and the ground either side are the picture.
                        let (ccx, ccy) = ((cut.x0 + cut.x1) / 2, cut.top + 14);
                        lab.renderer
                            .set_camera(ccx - span_x / 2, ccy - span_y / 2, (full_w, full_h), Some(bounds));
                        camera_set = true;
                        println!("  shots: camera on ({ccx},{ccy}) at {zoom}x, {span_x}x{span_y} cells");
                    }
                    let mut full = vec![0u8; (full_w * full_h * 4) as usize];
                    lab.draw(&mut full, 60.0);
                    // The world only: below the corner readout, above the toolbar.
                    let (y0, rows) = (24u32, 216u32);
                    let crop = full[(y0 * full_w * 4) as usize..((y0 + rows) * full_w * 4) as usize].to_vec();
                    if let Some(img) = image::RgbaImage::from_raw(full_w, rows, crop) {
                        let path = std::path::Path::new(dir).join(format!("door_f{f:06}.png"));
                        if let Err(e) =
                            image::imageops::resize(&img, full_w * 2, rows * 2, image::imageops::FilterType::Nearest)
                                .save(&path)
                        {
                            eprintln!("nestdoor: shot {}: {e}", path.display());
                        }
                    }
                }
            }
            seen_home.clear();
            peak_home = 0;
            prev = now;
            seen_at_nest.clear();
            peak_at_nest = 0;
            laden_dist_sum = 0.0;
            laden_dist_n = 0;
            closest = i32::MAX;
            all_dist_sum = 0.0;
            all_dist_n = 0;
        }
        if f < frames {
            if ant_out.is_some() {
                // This tick is one of the last `ANT_LOG_FRAMES` before a
                // sample: log it; otherwise leave the engine's log off. A
                // sample just taken has read the last window, so it is cleared.
                if f % ant_every == 0 {
                    last_decision.clear();
                }
                lab.world.decision_log = (ant_every - f % ant_every <= ANT_LOG_FRAMES).then(Vec::new);
            }
            lab.tick_for_harness();
            if let Some(log) = lab.world.decision_log.as_mut() {
                for r in log.drain(..) {
                    last_decision.insert(r.id, r);
                }
            }
            if berm && !bermed {
                if let Some(cut) = lab.world.nest_sites.iter().find_map(|s| s.shaft) {
                    let stone = lab.world.materials.id_of("stone").expect("stone ships");
                    for x in [cut.x0 - 2, cut.x0 - 1, cut.x1 + 1, cut.x1 + 2] {
                        for y in [cut.top - 2, cut.top - 1] {
                            if lab.world.get(x, y).material == material::EMPTY {
                                lab.world.set(x, y, Cell::new(stone, 0));
                            }
                        }
                    }
                    bermed = true;
                    println!("  frame {f:>7}: berm set beside the mouth, columns {}..{} and {}..{}, rows {}..{} (mouth x {}..{}, top {}, mouth bottom {}, shaft bottom {}, chamber x {}..{} y {}..{})", cut.x0 - 2, cut.x0 - 1, cut.x1 + 1, cut.x1 + 2, cut.top - 2, cut.top - 1, cut.x0, cut.x1, cut.top, cut.mouth_bottom, cut.bottom, cut.chamber_x0, cut.chamber_x1, cut.chamber_top, cut.chamber_bottom);
                }
            }
            if keepopen {
                let cuts: Vec<_> = lab.world.nest_sites.iter().filter_map(|s| s.shaft).collect();
                for cut in cuts {
                    for (x, y) in cut.cells() {
                        let c = lab.world.get(x, y);
                        let kind = lab.world.materials.kind(c.material);
                        let name = lab.world.materials.get(c.material).name.as_str();
                        if matches!(kind, MaterialKind::Liquid | MaterialKind::Plant)
                            || matches!(name, "soil" | "packedsoil" | "spoil")
                        {
                            *kept_open
                                .entry(lab.world.materials.get(c.material).name.clone())
                                .or_default() += 1;
                            lab.world.set(x, y, Cell::EMPTY);
                        }
                    }
                }
            }
            if wake > 0 && lab.world.frame.is_multiple_of(wake) {
                lab.world.wake_all();
            }
            if mist_half && lab.world.frame.is_multiple_of(80) {
                pixel_physics::lab::rain::tick(&mut lab.world, &lab.spec, Rain::Light);
            }
            if pump {
                let cuts: Vec<_> = lab.world.nest_sites.iter().filter_map(|s| s.shaft).collect();
                for cut in cuts {
                    for (x, y) in cut.cells() {
                        let c = lab.world.get(x, y);
                        if lab.world.materials.kind(c.material) == MaterialKind::Liquid {
                            pumped += u64::from(pixel_physics::sim::update::liquid_fill(c));
                            lab.world.set(x, y, Cell::EMPTY);
                        }
                    }
                }
            }
            if umbrella {
                let cuts: Vec<_> = lab.world.nest_sites.iter().filter_map(|s| s.shaft).collect();
                for cut in cuts {
                    for x in cut.x0 - 1..=cut.x1 + 1 {
                        for y in room_top..=cut.top - 3 {
                            let c = lab.world.get(x, y);
                            if lab.world.materials.kind(c.material) == MaterialKind::Liquid {
                                umbrella_caught += u64::from(pixel_physics::sim::update::liquid_fill(c));
                                lab.world.set(x, y, Cell::EMPTY);
                            }
                        }
                    }
                }
            }
        }
    }
    if keepopen {
        println!("KEEPOPEN cleared from the founding cut: {kept_open:?}");
    }
    if pump {
        println!(
            "PUMP took {pumped} units of water out of the founding cut ({:.1} full cells)",
            pumped as f64 / f64::from(material::LIQUID_FULL)
        );
    }
    if umbrella {
        println!(
            "UMBRELLA caught {umbrella_caught} units of falling water over the mouth ({:.1} full cells)",
            umbrella_caught as f64 / f64::from(material::LIQUID_FULL)
        );
    }
    if let Some(out) = ant_out.as_mut() {
        use std::io::Write;
        out.flush().expect("flush anttrace");
    }
    let s = &lab.world.creature_stats;
    println!(
        "SUMMARY nestdoor seed={seed} frames={frames} pickups={} drops={} deliveries={} nest_visits={} digs={} spoil={} births={} deaths={} nest_cells_now={}",
        s.pickups,
        s.drops,
        s.deliveries,
        s.nest_visits,
        s.digs,
        s.spoil_dumped,
        s.births,
        s.deaths,
        all_nest_cells(&lab.world).len()
    );
}

/// How many frames before each `anttrace` sample the engine's decision log
/// runs. An ant decides every `tick_interval` (6) frames, so 12 catches every
/// living animal at least once without paying for the log all run.
const ANT_LOG_FRAMES: u64 = 12;

const ANT_HEADER: &str = "frame,id,species,gen,x,y,cargo,energy,zone,home,d_home,route,route_wet,anchor_x,anchor_y,d_anchor,outcome,at_nest_in,crowding_in,dig_urge,ahead";

/// Could an ant stand in this cell? Air, or another animal (ants pass
/// through nestmates, `passes_through_kin`) -- not brood, which nothing moves,
/// and not ground, which has to be dug. `wet` also lets it through liquid.
fn open_for_ant(world: &World, x: i32, y: i32, wet: bool) -> bool {
    let c = world.get(x, y);
    if c.material == material::EMPTY {
        return true;
    }
    let id = c.organism_id();
    if id != 0 {
        return world
            .organism(id)
            .is_some_and(|s| s.brood.is_none() && world.species.get(s.species).creature.is_some());
    }
    wet && world.materials.kind(c.material) == MaterialKind::Liquid
}

/// Steps from every open cell to home through cells an ant could stand in,
/// 8-connected, from every open cell touching a home cell (`at_home`'s own
/// reach): `-1` where there is no way home without digging.
fn route_home(world: &World, home: &[(i32, i32)], wet: bool) -> (pixel_physics::sim::chunk::Rect, Vec<i32>) {
    let b = world.bounds().expect("a lab world has bounds");
    let w = (b.max_x - b.min_x + 1) as usize;
    let h = (b.max_y - b.min_y + 1) as usize;
    let mut dist = vec![-1i32; w * h];
    let idx = |x: i32, y: i32| (y - b.min_y) as usize * w + (x - b.min_x) as usize;
    let inside = |x: i32, y: i32| x >= b.min_x && x <= b.max_x && y >= b.min_y && y <= b.max_y;
    let mut queue = std::collections::VecDeque::new();
    for &(hx, hy) in home {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y) = (hx + dx, hy + dy);
                if inside(x, y) && dist[idx(x, y)] < 0 && open_for_ant(world, x, y, wet) {
                    dist[idx(x, y)] = 0;
                    queue.push_back((x, y));
                }
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let d = dist[idx(x, y)];
        for (dx, dy) in N8 {
            let (nx, ny) = (x + dx, y + dy);
            if inside(nx, ny) && dist[idx(nx, ny)] < 0 && open_for_ant(world, nx, ny, wet) {
                dist[idx(nx, ny)] = d + 1;
                queue.push_back((nx, ny));
            }
        }
    }
    (b, dist)
}

/// **One `anttrace` row per colony animal, and the `ANTS` line that sums
/// them** -- the per-individual answer to "why is it not in the nest, why is
/// it not digging". For each animal:
///
/// - where it is: `zone` is `home` (`at_home`, the laying lane's predicate),
///   `door` (within 3 cells of the painted patch), `below` (under the old
///   ground line), `surface` (within 3 rows above it) or `aloft`;
/// - whether it *could* get home: `route` is the walk home through air and
///   nestmates (`-1`: none without digging), `route_wet` the same through
///   water too, so a door shut by a pond reads `-1` then a number;
/// - where it is *trying* to go: `anchor` is its latest decision's homing
///   anchor (`creature::DecisionRow::anchor`), `-` if it made none in the log
///   window;
/// - whether it wants to dig: `dig_urge` is the brain's `Dig` output clamped
///   as `act` clamps it, from `creature::probe_full` on its head now, beside
///   the two inputs the nest's dig gate reads (`AtNest`, `Crowding`), and
///   `ahead`, what it faces -- what it would cut. A laden animal never digs.
fn ant_rows(
    world: &World,
    ground_y: i32,
    patch: &[(i32, i32)],
    last: &std::collections::HashMap<OrganismId, DecisionRow>,
    out: &mut impl std::io::Write,
    f: u64,
) -> String {
    use pixel_physics::sim::brain::{BrainInput as I, BrainOutput as O};
    let nest = world.materials.id_of("nest");
    let mut home: Vec<(i32, i32)> = world.nest_dug.iter().copied().collect();
    home.extend(all_nest_cells(world));
    home.sort_unstable();
    home.dedup();
    let (b, dry) = route_home(world, &home, false);
    let (_, wet) = route_home(world, &home, true);
    let w = b.max_x - b.min_x + 1;
    let at = |v: &[i32], x: i32, y: i32| v[((y - b.min_y) * w + (x - b.min_x)) as usize];
    let name = |x: i32, y: i32| -> String {
        let c = world.get(x, y);
        let id = c.organism_id();
        if id != 0 {
            if let Some(s) = world.organism(id) {
                if s.brood.is_some() {
                    return "brood".into();
                }
                if world.species.get(s.species).creature.is_some() {
                    return "animal".into();
                }
            }
        }
        world.materials.get(c.material).name.clone()
    };
    let (mut n, mut n_home, mut n_door, mut n_below, mut n_surface, mut n_aloft) = (0, 0, 0, 0, 0, 0);
    let (mut no_route, mut wet_only, mut laden) = (0, 0, 0);
    let (mut urge_sum, mut urge_on, mut crowd_sum, mut home_empty) = (0f32, 0, 0f32, 0);
    let mut ahead_home: std::collections::BTreeMap<String, u32> = Default::default();
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        let Some(def) = world.species.get(st.species).creature.as_ref() else {
            continue;
        };
        if world.colony_of(id) == 0 {
            continue;
        }
        let Some(&(x, y)) = st.chain.first() else { continue };
        n += 1;
        let is_home = at_home(world, (x, y), nest);
        let d_home = home
            .iter()
            .map(|&(hx, hy)| (hx - x).abs().max((hy - y).abs()))
            .min()
            .unwrap_or(-1);
        let d_door = patch
            .iter()
            .map(|&(px, py)| (px - x).abs().max((py - y).abs()))
            .min()
            .unwrap_or(i32::MAX);
        let zone = if is_home {
            "home"
        } else if d_door <= 3 {
            "door"
        } else if y >= ground_y {
            "below"
        } else if y >= ground_y - 3 {
            "surface"
        } else {
            "aloft"
        };
        match zone {
            "home" => n_home += 1,
            "door" => n_door += 1,
            "below" => n_below += 1,
            "surface" => n_surface += 1,
            _ => n_aloft += 1,
        }
        let route = at(&dry, x, y);
        let route_wet = at(&wet, x, y);
        if !is_home && route < 0 {
            no_route += 1;
            if route_wet >= 0 {
                wet_only += 1;
            }
        }
        let cargo = if st.crop.is_some() {
            "food"
        } else if st.spoil.is_some() {
            "spoil"
        } else {
            "-"
        };
        if cargo != "-" {
            laden += 1;
        }
        let (inp, _, outp, _) = pixel_physics::sim::creature::probe_full(world, x, y, id, def);
        let urge = outp[O::Dig as usize].clamp(0.0, 1.0);
        let (hdx, hdy) = pixel_physics::sim::creature::DIRS[(st.heading % 8) as usize];
        let ahead = name(x + hdx, y + hdy);
        if is_home {
            urge_sum += urge;
            crowd_sum += inp[I::Crowding as usize];
            if urge > 0.05 && cargo == "-" {
                urge_on += 1;
            }
            if cargo == "-" {
                home_empty += 1;
            }
            *ahead_home.entry(ahead.clone()).or_default() += 1;
        }
        let (ax, ay, outcome) = match last.get(&id) {
            Some(r) => (
                r.anchor.0.to_string(),
                r.anchor.1.to_string(),
                pixel_physics::sim::creature::DECISION_OUTCOME_NAMES[r.outcome as usize],
            ),
            None => ("-".into(), "-".into(), "-"),
        };
        let d_anchor = last.get(&id).map_or("-".to_string(), |r| {
            (r.anchor.0 - x).abs().max((r.anchor.1 - y).abs()).to_string()
        });
        writeln!(
            out,
            "{f},{id},{},{},{x},{y},{cargo},{:.0},{zone},{},{d_home},{route},{route_wet},{ax},{ay},{d_anchor},{outcome},{:.2},{:.2},{:.3},{ahead}",
            world.species.get(st.species).name,
            st.generation,
            st.energy,
            u8::from(is_home),
            inp[I::AtNest as usize],
            inp[I::Crowding as usize],
            urge
        )
        .expect("write anttrace");
    }
    format!(
        "ANTS frame={f} colony animals {n}: home {n_home}, at the door {n_door}, below the old ground line {n_below}, on the surface {n_surface}, aloft {n_aloft} | not home with no dry way home {no_route} (of them a way only through water {wet_only}) | carrying {laden} | home now {} cells | at home: empty-jawed {home_empty}, dig urge mean {:.3}, over 0.05 and empty {urge_on}, crowding mean {:.2}, facing {:?}",
        home.len(),
        if n_home == 0 { 0.0 } else { urge_sum / n_home as f32 },
        if n_home == 0 { 0.0 } else { crowd_sum / n_home as f32 },
        ahead_home
    )
}

/// The positive control. Bury half the patch, erase the other half, and check
/// each column reports what it was shown.
fn selftest(lab: &mut Lab, patch: &[(i32, i32)]) {
    let before = census_door(&lab.world, patch);
    // **The water pair, before anything is buried**, because the claim the
    // run makes is that the patch floods *and the ground beside it does not*,
    // and a pair that cannot tell those apart is not a control. Four cells of
    // water over four patch columns must move `water_on` and must leave
    // `water_off` alone; four over the columns outside must do the reverse.
    let water = lab.world.materials.id_of("water").expect("water ships");
    let full = |m| Cell::new(m, 0).with_aux(pixel_physics::sim::material::LIQUID_FULL);
    for &(x, y) in patch.iter().take(4) {
        lab.world.set(x, y - 1, full(water));
    }
    let wet_on = census_door(&lab.world, patch);
    let (ox, oy) = *patch.first().expect("non-empty patch");
    for i in 1..=4 {
        lab.world.set(ox - i, oy - 1, full(water));
    }
    let wet_off = census_door(&lab.world, patch);
    let packed = lab.world.materials.id_of("packedsoil").expect("packedsoil ships");
    let half = patch.len() / 2;
    // Bury the first half: three cells of packed spoil straight up, which is
    // what the dump verb does to a column.
    for &(x, y) in &patch[..half] {
        for dy in 1..=3 {
            lab.world.set(x, y - dy, Cell::new(packed, 0));
        }
    }
    // Erase the second half outright.
    for &(x, y) in &patch[half..] {
        lab.world.set(x, y, Cell::new(material::EMPTY, 0));
    }
    let after = census_door(&lab.world, patch);
    let mut ok = true;
    let mut check = |name: &str, pass: bool, saw: String| {
        println!("  {} {name}: {saw}", if pass { "PASS" } else { "FAIL" });
        ok &= pass;
    };
    check(
        "patch is non-trivial",
        patch.len() >= 8,
        format!("{} cells", patch.len()),
    );
    check(
        "before: all held",
        before.held == patch.len(),
        format!("held {} of {}", before.held, patch.len()),
    );
    check(
        "before: somewhere to stand",
        before.stand > 0,
        format!("stand {}", before.stand),
    );
    check(
        "after: erased half reads lost",
        after.lost == patch.len() - half,
        format!("lost {} (erased {})", after.lost, patch.len() - half),
    );
    check(
        "after: buried half still held",
        after.held == half,
        format!("held {} (expected {half})", after.held),
    );
    check(
        "after: buried half reads covered",
        after.covered == half,
        format!("covered {} (expected {half})", after.covered),
    );
    check(
        "after: burying costs footing",
        after.stand < before.stand,
        format!("stand {} -> {}", before.stand, after.stand),
    );
    check(
        "after: the cover is named",
        after.covered_by.iter().any(|(n, _)| n == "packedsoil"),
        format!("{:?}", after.covered_by),
    );
    check(
        "water over the patch moves water_on only",
        wet_on.water_on >= before.water_on + 4 && wet_on.water_off == before.water_off,
        format!(
            "on {} -> {}, off {} -> {}",
            before.water_on, wet_on.water_on, before.water_off, wet_on.water_off
        ),
    );
    check(
        "water beside the patch moves water_off only",
        wet_off.water_off >= wet_on.water_off + 4 && wet_off.water_on == wet_on.water_on,
        format!(
            "on {} -> {}, off {} -> {}",
            wet_on.water_on, wet_off.water_on, wet_on.water_off, wet_off.water_off
        ),
    );
    // A kind sanity line, so a rename of `packedsoil` fails loudly rather
    // than reading as a clean zero.
    check(
        "packedsoil is ground",
        matches!(
            lab.world.materials.kind(packed),
            MaterialKind::Solid | MaterialKind::Powder
        ),
        format!("{:?}", lab.world.materials.kind(packed)),
    );
    println!("selftest: {}", if ok { "ALL PASS" } else { "FAILURES" });
    if !ok {
        std::process::exit(1);
    }
}
