//! **Does it matter that three functions answer "what anchors a cell" three
//! different ways?** — `Reports/open-bugs-handoff.md` §S2.
//!
//! `structural.rs` writes anchor distances from three places and they
//! disagree about whether a cell resting on loose material (sand, gravel,
//! rubble, soil) counts as held up:
//!
//! | | used by | roots on loose ground? |
//! |---|---|---|
//! | `compute_world_distances` | worldgen, whole world | **never** — bedrock only |
//! | `relax_region` | the brush (`World::paint_capsule`) | **always**, immediately |
//! | `tick` | every scheduled check | only as a **last resort** |
//!
//! `tick`'s comment calls its last-resort rule *"the whole of the dig
//! cascade"*, because eager rooting makes a cell a load sink — *"a sprinkle
//! of sand under a beam holds the beam up"*. `relax_region` does the thing
//! that paragraph records removing, and does it as a Dijkstra **seed**, which
//! is the strongest form: the rooted cell is not merely held itself, it is a
//! zero-distance source every neighbour relaxes from.
//!
//! # Why this is one geometry and three routes, not three geometries
//!
//! The obvious build — paint a span, generate a span, dig a span — cannot
//! answer the question, because the three would differ in *shape* as well as
//! in rule and nothing could separate the two. `CLAUDE.md`: a paired
//! comparison cancels everything the rule under test is not about.
//!
//! So: **one world, built once, and then its distance field written three
//! ways.** Geometry, materials and seed are identical by construction; the
//! only difference is which function last decided what anchors what. The
//! material census is printed for each arm as the control that says so.
//!
//! # The scene
//!
//! A bridge with one real pier and one pile of sand under its middle:
//!
//! ```text
//!   ####                                                       <- span (stone)
//!   #  |                    ~~~~                               <- sand pile
//!   #  |                    ~~~~
//!   ################################################           <- bedrock
//!   ^ pier reaches bedrock   ^ sand reaches bedrock
//! ```
//!
//! Under `relax_region` the sand-backed cells root at 0, so the far end of
//! the span measures its distance from *the sand* — a short path. Under the
//! other two it measures from the pier, because a cell with a lateral path
//! does not take the last-resort root. If the rule matters, the two arms
//! disagree about how far out the span is, and that is what decides whether
//! its outer cells exceed what stone can hold.
//!
//! ```
//! cargo run --release --example anchor_probe
//! cargo run --release --example anchor_probe -- frames=1200 span=180
//! ```

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::world::World;
use pixel_physics::sim::{scheduler, structural};
use pixel_physics::worldgen::{self, Spec, WorldgenPresets};

const W: i32 = 256;
const H: i32 = 128;

/// Where the deck sits, and how thick it is.
const DECK_TOP: i32 = 58;
const DECK_THICK: i32 = 3;
/// The pier: the one support that genuinely reaches bedrock.
const PIER_X0: i32 = 20;
const PIER_X1: i32 = 26;
/// The sand pile under the deck. **Where it sits decides whether this probe
/// can answer anything**, and the first run of this sweep got it wrong: with
/// the pile at x100 the deck's failure margin (between span 40 and 60) was
/// reached before the deck even *touched* the sand, so every arm agreed by
/// construction and the null said nothing about the rule. The pile has to sit
/// under the span lengths that are actually marginal.
const DEFAULT_SAND: (i32, i32) = (35, 55);

fn build(span_end: i32, sand_span: (i32, i32)) -> World {
    let mut world = World::new(Rect::new(0, 0, W - 1, H - 1));
    let sand = world.materials.id_of("sand").expect("sand.ron should be embedded");
    let floor = H - 1;

    for x in 0..W {
        world.set(x, floor, Cell::new(material::BEDROCK, 0));
    }
    // The pier, bedrock to deck.
    for x in PIER_X0..=PIER_X1 {
        for y in (DECK_TOP + DECK_THICK)..floor {
            world.set(x, y, Cell::new(material::STONE, 0));
        }
    }
    // The deck, from the pier out to `span_end`.
    for x in PIER_X0..=span_end {
        for y in DECK_TOP..(DECK_TOP + DECK_THICK) {
            world.set(x, y, Cell::new(material::STONE, 0));
        }
    }
    // The sand pile, bedrock up to the deck's underside.
    for x in sand_span.0..=sand_span.1 {
        for y in (DECK_TOP + DECK_THICK)..floor {
            world.set(x, y, Cell::new(sand, 0));
        }
    }
    world
}

/// Deck cells only — the thing whose support is in question.
fn deck_cells(span_end: i32) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for x in PIER_X0..=span_end {
        for y in DECK_TOP..(DECK_TOP + DECK_THICK) {
            v.push((x, y));
        }
    }
    v
}

/// Materials standing in the world, so each arm can prove it started from the
/// same geometry. `CLAUDE.md`: a guard whose inputs do not actually vary what
/// it guards is blind, and the inverse — two arms that quietly differ in
/// their scene — is how a rule comparison measures the scene instead.
/// `Solid` cells inside a box -- so a loss can be attributed to the deck or
/// to the pier. The distinction is the whole finding: a rule that costs deck
/// cells is a rule about the thing you built, and a rule that costs *pier*
/// cells is a rule that has re-routed a bedrock-founded column's load into a
/// sand pile.
fn solid_in(world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> i64 {
    let mut n = 0;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if world.materials.kind(world.get(x, y).material) == MaterialKind::Solid {
                n += 1;
            }
        }
    }
    n
}

fn census(world: &World) -> (usize, usize, usize) {
    let (mut solid, mut powder, mut other) = (0, 0, 0);
    for y in 0..H {
        for x in 0..W {
            match world.materials.kind(world.get(x, y).material) {
                MaterialKind::Solid => solid += 1,
                MaterialKind::Powder => powder += 1,
                _ => other += 1,
            }
        }
    }
    (solid, powder, other)
}

/// One arm: write the distance field by `arm`'s rule, read it, then let the
/// load model act and report what came down.
///
/// `pre_lost` is not decoration. The `tick` arm has to *run* the scheduler to
/// write anything, and a scheduled check can destroy the cell it is judging
/// -- so on a span that is already past its margin the field is read off a
/// deck that has partly collapsed, and its numbers describe rubble rather
/// than the rule. The first version of this probe reported `tick` rooting
/// 519 of 543 cells at zero and it was exactly that: survivors resting on
/// their own debris. Any row with a non-zero `pre_lost` is contaminated and
/// must not be read as the rule's field.
struct Arm {
    at_zero: usize,
    total: usize,
    max: u16,
    pre_lost: i64,
    deck_lost: i64,
    pier_lost: i64,
}

fn run_arm(arm: &str, span_end: i32, frames: usize, sand_span: (i32, i32)) -> Arm {
    let mut world = build(span_end, sand_span);
    let deck = deck_cells(span_end);
    let solid_before = census(&world).0 as i64;
    let floor = H - 1;
    let deck_before = solid_in(&world, PIER_X0, span_end, DECK_TOP, DECK_TOP + DECK_THICK - 1);
    let pier_before = solid_in(&world, PIER_X0, PIER_X1, DECK_TOP + DECK_THICK, floor - 1);

    match arm {
        "worldgen" => structural::compute_world_distances(&mut world),
        // **Two brush arms, because the region is half the rule.**
        // `paint_capsule` relaxes the stroke's bounding box plus a 4-cell
        // margin, not the world. Running it world-wide gives the ground rule
        // its widest possible reading; running it stroke-sized also exposes
        // `relax_region`'s *boundary* condition, which trusts the values just
        // outside the box. Those are two different mechanisms and a single
        // arm cannot tell them apart.
        "brush-wide" => structural::relax_region(&mut world, Rect::new(0, 0, W - 1, H - 1)),
        "brush" => {
            const MARGIN: i32 = 4;
            structural::relax_region(
                &mut world,
                Rect::new(PIER_X0 - MARGIN, DECK_TOP - MARGIN, span_end + MARGIN, DECK_TOP + DECK_THICK - 1 + MARGIN),
            );
        }
        _ => {
            structural::compute_world_distances(&mut world);
            for &(x, y) in &deck {
                world.schedule_structural_check(x, y);
            }
            for _ in 0..200 {
                scheduler::step(&mut world);
                world.frame += 1;
            }
        }
    }
    let pre_lost = solid_before - census(&world).0 as i64;

    let d: Vec<u16> = deck.iter().map(|&(x, y)| world.get(x, y).aux()).collect();
    let at_zero = d.iter().filter(|&&v| v == 0).count();
    let max = d.iter().copied().max().unwrap_or(0);

    for &(x, y) in &deck {
        world.schedule_structural_check(x, y);
    }
    for _ in 0..frames {
        scheduler::step(&mut world);
        pixel_physics::sim::parallel::step(&mut world);
        world.frame += 1;
    }
    Arm {
        at_zero,
        total: d.len(),
        max,
        pre_lost,
        deck_lost: deck_before - solid_in(&world, PIER_X0, span_end, DECK_TOP, DECK_TOP + DECK_THICK - 1),
        pier_lost: pier_before - solid_in(&world, PIER_X0, PIER_X1, DECK_TOP + DECK_THICK, floor - 1),
    }
}


/// **How often does a real world put a build site on loose ground?**
/// `worldgen=1` — the frequency half of §S2, which the hand-built scene above
/// deliberately does not answer.
///
/// The scene above proves the *mechanism*: where a painted structure rests on
/// loose material, the brush's rule roots that contact at 0 and the structure
/// comes down. It cannot say how often a player meets that geometry, because
/// it *is* that geometry — `CLAUDE.md` is explicit that a hand-placed case is
/// blind by construction to what worldgen produces, and that a guard over a
/// procedural system has to sweep the procedure and read an order statistic.
///
/// So: generate a world, walk the surface, and at each candidate site place
/// the same small platform two ways --- once through `World::paint_capsule`
/// (which relaxes the stroke's box, the shipped brush) and once placed
/// directly followed by `compute_world_distances` (the worldgen rule). Both
/// arms are the same seed and the same platform, so the only difference is
/// the rule. Count the sites where the two disagree about what is anchored.
///
/// **The number to read is the disagreement rate, not any single site**, and
/// it is reported per seed as well as pooled, because outcomes here are
/// chaotic in the seed.
fn worldgen_census(seeds: &[u64], sites_per_seed: usize, gw: i32, gh: i32, frames: usize) {
    let (presets, err) = WorldgenPresets::load();
    if let Some(e) = err {
        eprintln!("preset load: {e}");
    }
    let name = presets.default_name();
    let params = presets.get(&name).unwrap_or_else(|| panic!("no default preset")).clone();

    println!("worldgen census: preset {name}, {gw}x{gh}, seeds {seeds:?}, {sites_per_seed} sites each, {frames} frames\n");
    println!(
        "{:>6} {:>8} {:>12} {:>14} {:>14} {:>12} {:>11} {:>11}",
        "seed", "sites", "on loose", "brush roots", "rule disagrees", "extra lost", "brush lost", "gen lost"
    );
    println!("{}", "-".repeat(94));

    let (mut tot_sites, mut tot_loose, mut tot_disagree, mut tot_extra) = (0usize, 0usize, 0usize, 0i64);
    // **Absolute losses per arm, not only their difference.** An `extra lost`
    // of zero means the rule changed nothing *only if something happened at
    // all*; if both arms lost nothing, the platforms simply stood and the
    // comparison is vacuous. Two of this session's measurements were exactly
    // that shape before a control caught them, so the control is printed
    // rather than assumed.
    let (mut tot_brush_lost, mut tot_gen_lost) = (0i64, 0i64);
    for &seed in seeds {
        let mut on_loose = 0usize;
        let mut roots = 0usize;
        let mut disagree = 0usize;
        let mut extra_lost = 0i64;
        let (mut seed_brush_lost, mut seed_gen_lost) = (0i64, 0i64);

        for i in 0..sites_per_seed {
            // Sites spread across the middle of the world, clear of the edges.
            // Spread so two platforms cannot touch: each is PLATFORM_LEN long.
            let x = gw / 8 + (i as i32 * (gw * 3 / 4)) / sites_per_seed.max(1) as i32;
            debug_assert!((gw * 3 / 4) / sites_per_seed.max(1) as i32 > PLATFORM_LEN + 8, "sites would overlap");

            let mut brush = World::new(Rect::new(0, 0, gw - 1, gh - 1));
            worldgen::generate_only(&mut brush, Spec::Generated { params: &params, seed });
            let Some(sy) = surface_of(&brush, x) else { continue };
            // The platform sits *on* the surface, which is the case in
            // question: a player building on whatever is there.
            let plat_y = sy - 1;
            let (a, b) = ((x, plat_y), (x + PLATFORM_LEN, plat_y));

            // Does anything loose touch the platform's underside? If not, no
            // rule can differ here and the site is not evidence either way.
            let loose = (x..=x + PLATFORM_LEN)
                .any(|px| brush.materials.kind(brush.get(px, plat_y + PLATFORM_RADIUS + 1).material) == MaterialKind::Powder);
            if loose {
                on_loose += 1;
            }
            tot_sites += 1;

            brush.paint_capsule(a, b, PLATFORM_RADIUS, material::STONE, 1.0);
            let plat: Vec<(i32, i32)> = (x - PLATFORM_RADIUS..=x + PLATFORM_LEN + PLATFORM_RADIUS)
                .flat_map(|px| (plat_y - PLATFORM_RADIUS..=plat_y + PLATFORM_RADIUS).map(move |py| (px, py)))
                .filter(|&(px, py)| brush.materials.kind(brush.get(px, py).material) == MaterialKind::Solid)
                .collect();
            let brush_zero = plat.iter().filter(|&&(px, py)| brush.get(px, py).aux() == 0).count();

            // The control arm: identical seed, identical platform, placed
            // without the brush's relax so `compute_world_distances` decides.
            let mut gen = World::new(Rect::new(0, 0, gw - 1, gh - 1));
            worldgen::generate_only(&mut gen, Spec::Generated { params: &params, seed });
            for &(px, py) in &plat {
                gen.set(px, py, Cell::new(material::STONE, 0));
            }
            structural::compute_world_distances(&mut gen);
            let gen_zero = plat.iter().filter(|&&(px, py)| gen.get(px, py).aux() == 0).count();

            roots += brush_zero.saturating_sub(gen_zero);
            if brush_zero > gen_zero {
                disagree += 1;
            }

            for _ in 0..frames {
                scheduler::step(&mut brush);
                pixel_physics::sim::parallel::step(&mut brush);
                brush.frame += 1;
                scheduler::step(&mut gen);
                pixel_physics::sim::parallel::step(&mut gen);
                gen.frame += 1;
            }
            // **Census the platform, not the world.** `census()` counts every
            // Solid cell, and a generated world sheds its own loose material
            // for hundreds of frames regardless of what was built on it -- so
            // a whole-world loss is dominated by settling that both arms do
            // identically, and comes back equal to the cell whatever the
            // platform did. That is a second, quieter version of the vacuity
            // this row exists to detect: the number moved, so it looks like a
            // measurement, and it is still not about the platform.
            let still_standing = |w: &World| -> i64 {
                plat.iter().filter(|&&(px, py)| w.materials.kind(w.get(px, py).material) == MaterialKind::Solid).count() as i64
            };
            let brush_lost = plat.len() as i64 - still_standing(&brush);
            let gen_lost = plat.len() as i64 - still_standing(&gen);
            extra_lost += brush_lost - gen_lost;
            seed_brush_lost += brush_lost;
            seed_gen_lost += gen_lost;
        }

        println!(
            "{seed:>6} {sites_per_seed:>8} {on_loose:>12} {roots:>14} {disagree:>14} {extra_lost:>12} {seed_brush_lost:>11} {seed_gen_lost:>11}"
        );
        tot_loose += on_loose;
        tot_disagree += disagree;
        tot_extra += extra_lost;
        tot_brush_lost += seed_brush_lost;
        tot_gen_lost += seed_gen_lost;
    }
    println!("{}", "-".repeat(94));
    let pct = |n: usize| 100.0 * n as f64 / tot_sites.max(1) as f64;
    println!(
        "{:>6} {tot_sites:>8} {:>12} {:>14} {:>14} {tot_extra:>12} {tot_brush_lost:>11} {tot_gen_lost:>11}",
        "all",
        format!("{tot_loose} ({:.0}%)", pct(tot_loose)),
        "",
        format!("{tot_disagree} ({:.0}%)", pct(tot_disagree)),
    );
    if tot_brush_lost == 0 && tot_gen_lost == 0 {
        println!("\n!! BOTH ARMS LOST NO PLATFORM CELLS -- every platform stood, so `extra lost` compares");
        println!("!! two non-events and says nothing about the rule. Make the site harder before reading it.");
    }
    println!("`brush lost` / `gen lost` count cells of the PLATFORM destroyed, not of the world:");
    println!("a generated world sheds its own loose material for hundreds of frames either way.");
    println!("\n`on loose` = the platform sits over Powder. `rule disagrees` = the brush rooted cells");
    println!("`compute_world_distances` did not. `extra lost` = Solid cells the brush arm destroyed beyond the control.");
}

/// Topmost `Solid`-or-`Powder` cell -- where a player would put something.
fn surface_of(world: &World, x: i32) -> Option<i32> {
    (0..world.bounds().map_or(0, |b| b.max_y + 1)).find(|&y| {
        matches!(world.materials.kind(world.get(x, y).material), MaterialKind::Solid | MaterialKind::Powder)
    })
}

/// The platform a site is tested with: a **one-sided** run at constant height,
/// started from the local surface and drawn rightward.
///
/// One-sided and level on purpose. A platform centred on a site and following
/// the ground is a slab lying flat, which has no span for a rule about *load
/// paths* to disagree about -- measured, at 300 frames, as an `extra lost` of
/// zero at every site while the field disagreed at 88% of them. Drawn level
/// from the surface, the terrain itself decides how much of it ends up
/// cantilevered and what its underside meets on the way down, which is what a
/// player building outward from a slope actually produces.
const PLATFORM_LEN: i32 = 40;
const PLATFORM_RADIUS: i32 = 2;

fn main() {
    let mut frames = 900usize;
    let mut spans: Vec<i32> = vec![40, 45, 50, 55, 60, 70, 80, 100];
    let mut sand_span = DEFAULT_SAND;
    let mut census_mode = false;
    let mut seeds: Vec<u64> = vec![1, 2, 3, 7, 11, 24301];
    let mut sites = 12usize;
    let (mut gw, mut gh) = (2048i32, 640i32);
    for arg in std::env::args().skip(1) {
        if let Some((k, v)) = arg.split_once('=') {
            match k {
                "frames" => frames = v.parse().expect("frames=N"),
                "spans" => spans = v.split(',').map(|t| t.parse().expect("spans=A,B,C")).collect(),
                "worldgen" => census_mode = v != "0",
                "seeds" => seeds = v.split(',').map(|t| t.parse().expect("seeds=A,B,C")).collect(),
                "sites" => sites = v.parse().expect("sites=N"),
                "size" => {
                    let (a, b) = v.split_once('x').expect("size=WxH");
                    gw = a.parse().expect("size=WxH");
                    gh = b.parse().expect("size=WxH");
                }
                "sand" => {
                    let (a, b) = v.split_once(',').expect("sand=X0,X1");
                    sand_span = (a.parse().expect("sand=X0,X1"), b.parse().expect("sand=X0,X1"));
                }
                _ => eprintln!("ignoring unknown argument {arg:?}"),
            }
        }
    }
    // Echo the parameters -- a log that does not name them was written by a
    // binary that never had them (`Reports/instruments.md`'s standing gotcha).
    if census_mode {
        worldgen_census(&seeds, sites, gw, gh, frames);
        return;
    }
    println!("anchor_probe: {W}x{H}, deck from x{PIER_X0} at y{DECK_TOP}, pier {PIER_X0}..{PIER_X1}, sand {}..{}", sand_span.0, sand_span.1);
    println!("spans {spans:?}, {frames} frames per arm\n");

    // **A sweep, not one scene**, because the question is not "does this deck
    // fall" -- past its margin every rule agrees it falls, and short of the
    // margin every rule agrees it stands. The rule can only show itself in
    // *where the margin is*, which is the gameplay quantity: how far you can
    // build before it comes down.
    println!(
        "{:>5} | {:>24} | {:>24} | {:>24} | {:>24}",
        "span", "worldgen (bedrock only)", "brush (real stroke box)", "brush-wide (whole world)", "tick (last-resort root)"
    );
    // `max` beside the damage on purpose. Under the brush's rule the deck
    // reads as *better* supported -- a far shorter distance to an anchor --
    // and is the arm that collapses. A support field and an outcome pointing
    // opposite ways is the whole finding, and one column cannot show it.
    println!("{:>5} | {:>7} {:>5} {:>10} | {:>7} {:>5} {:>10} | {:>7} {:>5} {:>10} | {:>7} {:>5} {:>10}",
        "", "at-zero", "max", "destroyed", "at-zero", "max", "destroyed", "at-zero", "max", "destroyed", "at-zero", "max", "destroyed");
    println!("{}", "-".repeat(125));

    for &span in &spans {
        let mut cells = Vec::new();
        for arm in ["worldgen", "brush", "brush-wide", "tick"] {
            let a = run_arm(arm, span, frames, sand_span);
            let flag = if a.pre_lost > 0 { "*" } else { "" };
            cells.push(format!(
                "{:>7} {:>5} {:>10}",
                format!("{}/{}", a.at_zero, a.total),
                a.max,
                format!("{}+{}{}", a.deck_lost, a.pier_lost, flag)
            ));
        }
        println!("{span:>5} | {} | {} | {} | {}", cells[0], cells[1], cells[2], cells[3]);
    }
    println!("\n* = cells were already lost before the field was read, so that row's field describes rubble, not the rule.");
    println!("`destroyed` is deck+pier Solid cells lost. **The pier reaches bedrock and should never fall at all.**");
    println!("`max` is the deck's largest distance to an anchor: lower reads as better supported.");
}
