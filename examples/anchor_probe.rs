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

fn main() {
    let mut frames = 900usize;
    let mut spans: Vec<i32> = vec![40, 45, 50, 55, 60, 70, 80, 100];
    let mut sand_span = DEFAULT_SAND;
    for arg in std::env::args().skip(1) {
        if let Some((k, v)) = arg.split_once('=') {
            match k {
                "frames" => frames = v.parse().expect("frames=N"),
                "spans" => spans = v.split(',').map(|t| t.parse().expect("spans=A,B,C")).collect(),
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
