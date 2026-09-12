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
use pixel_physics::lab::Lab;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
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
    let mut d =
        Door { held: 0, lost: 0, lost_by: Vec::new(), airy: 0, stand: 0, covered: 0, covered_by: Vec::new(), water_on: 0, water_off: 0 };
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
            *cover_hist.entry(world.materials.get(world.get(x, y - 1).material).name.clone()).or_default() += 1;
        }
    }
    d.lost_by = lost_hist.into_iter().collect();
    d.covered_by = cover_hist.into_iter().collect();
    d.lost_by.sort_by_key(|e| std::cmp::Reverse(e.1));
    d.covered_by.sort_by_key(|e| std::cmp::Reverse(e.1));
    d
}

/// Every cell currently painted `nest`, wherever it is -- so a patch that
/// *moved* (a nest cell carried off as spoil and set down elsewhere) is not
/// read as a patch that vanished.
fn all_nest_cells(world: &World) -> Vec<(i32, i32)> {
    let Some(nest) = world.materials.id_of("nest") else { return Vec::new() };
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
type Animal = (u16, (i32, i32), u16, u32, bool);

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
    println!("nestdoor: control={control} scenario={scenario} seed={seed} frames={frames} every={every} rain={}", rain.map_or("-".to_string(), |r| r.to_string()));

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

    // The patch is painted when the colony is founded, which on the played
    // bed is frame 6,000 -- so it cannot be read before the run starts, and a
    // harness that sampled frame 0 would report "no nest" and stop.
    let mut patch: Vec<(i32, i32)> = Vec::new();
    let mut founded = false;
    // Per-window accumulators over *every* frame, not only the sample frames:
    // an ant at the door for one frame in ten thousand is the whole question.
    let mut seen_at_nest: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
    let mut peak_at_nest = 0usize;
    let mut laden_dist_sum = 0f64;
    let mut laden_dist_n = 0u64;
    let mut closest = i32::MAX;
    let mut all_dist_sum = 0f64;
    let mut all_dist_n = 0u64;
    let mut prev = [0u64; 8];

    println!(
        "  {:>7} | {:>4} {:>4} {:>4} {:>4} | {:>5} {:>5} {:>5} {:>5} | {:>5} {:>5} | {:>5} {:>4} {:>4} {:>3} | {:>6}",
        "frame", "held", "lost", "stnd", "covd", "pickp", "drops", "deliv", "visit", "digs", "spoil", "alive", "born", "died", "gen", "atnest"
    );

    for f in 0..=frames {
        if !founded {
            let cells = all_nest_cells(&lab.world);
            if !cells.is_empty() {
                patch = cells;
                founded = true;
                let (x0, x1) = (patch.iter().map(|c| c.0).min().unwrap_or(0), patch.iter().map(|c| c.0).max().unwrap_or(0));
                let (y0, y1) = (patch.iter().map(|c| c.1).min().unwrap_or(0), patch.iter().map(|c| c.1).max().unwrap_or(0));
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
            for (id, (hx, hy), _gen, _del, laden) in animals(&lab.world) {
                if N8.iter().any(|&(dx, dy)| Some(lab.world.get(hx + dx, hy + dy).material) == nest) {
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
        }

        if f % every == 0 || f == frames {
            let w = &lab.world;
            let s = &w.creature_stats;
            let now = [s.pickups, s.drops, s.deliveries, s.nest_visits, s.digs, s.spoil_dumped, s.births, s.deaths];
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
            lab.tick_for_harness();
        }
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
    check("patch is non-trivial", patch.len() >= 8, format!("{} cells", patch.len()));
    check("before: all held", before.held == patch.len(), format!("held {} of {}", before.held, patch.len()));
    check("before: somewhere to stand", before.stand > 0, format!("stand {}", before.stand));
    check("after: erased half reads lost", after.lost == patch.len() - half, format!("lost {} (erased {})", after.lost, patch.len() - half));
    check("after: buried half still held", after.held == half, format!("held {} (expected {half})", after.held));
    check("after: buried half reads covered", after.covered == half, format!("covered {} (expected {half})", after.covered));
    check("after: burying costs footing", after.stand < before.stand, format!("stand {} -> {}", before.stand, after.stand));
    check("after: the cover is named", after.covered_by.iter().any(|(n, _)| n == "packedsoil"), format!("{:?}", after.covered_by));
    check(
        "water over the patch moves water_on only",
        wet_on.water_on >= before.water_on + 4 && wet_on.water_off == before.water_off,
        format!("on {} -> {}, off {} -> {}", before.water_on, wet_on.water_on, before.water_off, wet_on.water_off),
    );
    check(
        "water beside the patch moves water_off only",
        wet_off.water_off >= wet_on.water_off + 4 && wet_off.water_on == wet_on.water_on,
        format!("on {} -> {}, off {} -> {}", wet_on.water_on, wet_off.water_on, wet_on.water_off, wet_off.water_off),
    );
    // A kind sanity line, so a rename of `packedsoil` fails loudly rather
    // than reading as a clean zero.
    check(
        "packedsoil is ground",
        matches!(lab.world.materials.kind(packed), MaterialKind::Solid | MaterialKind::Powder),
        format!("{:?}", lab.world.materials.kind(packed)),
    );
    println!("selftest: {}", if ok { "ALL PASS" } else { "FAILURES" });
    if !ok {
        std::process::exit(1);
    }
}
