//! **Ground standing on nothing** -- the standing census §Z18 asked for, and
//! the first thing that build needed.
//!
//! The owner has named this defect twice, unprompted, on cards that asked
//! about something else: *"Both look bad and have lots of stuff floating in
//! the air."* Spoil an ant hauls out is tamped, tamped ground is
//! `self_supporting`, and `self_supporting` is the rule that says a cell may
//! not fall -- so anything that later takes away what a pellet was resting on
//! strands it for ever, and a colony stacking spoil on its own spoil builds a
//! lattice out of the result.
//!
//! **Nothing in the harness counted it.** `latecensus`'s `mound` and
//! `packed_above` count cells whether they are standing on anything or not,
//! and `labnest`'s `caved` counts an event. `soilfork mode=fork` does carry a
//! `hanging` column, and it is the right quantity -- this file is its
//! scene-independent form, with the controls the fix has to be argued from.
//!
//! **A standing count, not a creation rate**, because the complaint is about
//! what is still there. An event-rate metric on this exact class of question
//! has already blamed the wrong mechanism in this repo (`CLAUDE.md`, *when the
//! complaint is visible and persistent, measure the standing state*): films
//! were attributed to the mechanism that *made* 76% of them, all of which
//! existed for one frame.
//!
//! ## What it counts
//!
//! *Ground* is a non-empty, non-organism `Powder` or `Solid` cell -- the box
//! floor, the bank, the lining, loose tilth, a heap of spoil. **A plant cell
//! is not ground, deliberately**: spoil posted into a canopy is held up by
//! leaves, which is the engulfing half of the same complaint and not a wall.
//!
//! *Anchored* is whatever a flood from the bottom row of the world reaches
//! through ground. `anchors` is printed on every line so an empty seed set --
//! a scene with no floor -- reads as a broken harness rather than as a world
//! made entirely of floating dirt.
//!
//! | column | what it is |
//! |---|---|
//! | `hang` | `self_supporting` cells the 8-connected flood never reaches. **The headline.** |
//! | `pieces` | 8-connected components of those -- two cells a piece or sixty |
//! | `hang4` | the same count with a 4-connected flood: a cell held only by a corner counts as hanging |
//! | `over` | `self_supporting` cells that *are* anchored and have air directly beneath -- **every gallery roof in the world.** The specificity column: a fix that takes this down with it has taken the nest down |
//! | `loose` | loose soil cells, so a rule that converts worked ground wholesale is visible as its product |
//! | `packed` | every `self_supporting` cell, the denominator |
//!
//! ## Controls
//!
//! `mode=selftest` runs six, and they are why the numbers above can be
//! quoted. Three are positive (a hand-placed floating block must read
//! exactly itself; a corner-hung cell must separate `hang` from `hang4`; a
//! block whose support is deleted must *move* the count), two are negative (a
//! bare box, and a carved gallery whose roof must read zero however much air
//! is under it), and one is the harness's own (`anchors` must be nonzero on a
//! box with a floor). `CLAUDE.md`: run the positive control, and check it
//! stays quiet on a case you know is fine.
//!
//! ```text
//! cargo run --release --example hangcensus -- mode=selftest
//! cargo run --release --example hangcensus -- scenario=played_bed seed=1 frames=120000 sample=20000
//! PIXEL_PHYSICS_SPOIL_FOOTING=off cargo run --release --example hangcensus -- scenario=played_bed seed=1 frames=120000
//! ```
//!
//! The two arms of the repair come out of **one binary across one env
//! switch** (`update::spoil_footing`), which is the shape this repo requires
//! of a standing quantity: a standing count has no baseline of its own, so
//! the only honest comparison is the same build run twice.

use pixel_physics::lab::census::{self, Ids};
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
struct Hang {
    /// `self_supporting` cells with no 8-connected path through ground to the
    /// world floor.
    hang: usize,
    /// 8-connected components of those.
    pieces: usize,
    /// The same census with a 4-connected flood -- a corner does not hold.
    hang4: usize,
    /// Anchored `self_supporting` cells with air directly beneath: the roofs.
    over: usize,
    /// Loose soil cells in the world.
    loose: usize,
    /// Every `self_supporting` cell, anchored or not.
    packed: usize,
    /// Ground cells on the bottom row -- the flood's seed set. Zero means
    /// this harness measured a scene it does not understand, not a world of
    /// floating dirt.
    anchors: usize,
}

/// Is this cell load-bearing ground? A non-empty, non-organism `Powder` or
/// `Solid`. See the header on why a plant cell is not.
fn is_ground(world: &World, x: i32, y: i32) -> bool {
    let cell = world.get(x, y);
    if cell.is_empty() || cell.organism_id() != 0 {
        return false;
    }
    matches!(world.materials.get(cell.material).kind, MaterialKind::Powder | MaterialKind::Solid)
}

/// Flood upward from the bottom row through ground, `diagonals` deciding
/// whether a corner contact carries. Returns the reached set and its seed
/// count.
fn anchored(world: &World, w: i32, h: i32, diagonals: bool) -> (Vec<bool>, usize) {
    let mut seen = vec![false; (w * h) as usize];
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    let mut stack = Vec::new();
    let mut anchors = 0;
    for x in 0..w {
        if is_ground(world, x, h - 1) {
            seen[idx(x, h - 1)] = true;
            stack.push((x, h - 1));
            anchors += 1;
        }
    }
    const N4: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
    const N8: [(i32, i32); 8] = [(0, -1), (0, 1), (-1, 0), (1, 0), (-1, -1), (1, -1), (-1, 1), (1, 1)];
    while let Some((x, y)) = stack.pop() {
        let ring: &[(i32, i32)] = if diagonals { &N8 } else { &N4 };
        for (dx, dy) in ring {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || nx >= w || ny < 0 || ny >= h || seen[idx(nx, ny)] || !is_ground(world, nx, ny) {
                continue;
            }
            seen[idx(nx, ny)] = true;
            stack.push((nx, ny));
        }
    }
    (seen, anchors)
}

fn census(world: &World, w: i32, h: i32) -> Hang {
    let (seen8, anchors) = anchored(world, w, h, true);
    let (seen4, _) = anchored(world, w, h, false);
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    let mut f = Hang { anchors, ..Hang::default() };
    let mut piece_seen = vec![false; (w * h) as usize];
    let soil = world.materials.id_of("soil");
    for y in 0..h {
        for x in 0..w {
            let cell = world.get(x, y);
            if cell.is_empty() || cell.organism_id() != 0 {
                continue;
            }
            if Some(cell.material) == soil {
                f.loose += 1;
            }
            if !world.materials.get(cell.material).self_supporting {
                continue;
            }
            f.packed += 1;
            if !seen4[idx(x, y)] {
                f.hang4 += 1;
            }
            if seen8[idx(x, y)] {
                // Anchored: the only question left is whether it is a roof.
                if world.get(x, y + 1).is_empty() {
                    f.over += 1;
                }
                continue;
            }
            f.hang += 1;
            if piece_seen[idx(x, y)] {
                continue;
            }
            // A new piece: flood it so the rest of it is not counted again.
            f.pieces += 1;
            piece_seen[idx(x, y)] = true;
            let mut st = vec![(x, y)];
            while let Some((px, py)) = st.pop() {
                for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let (nx, ny) = (px + dx, py + dy);
                    if nx < 0 || nx >= w || ny < 0 || ny >= h || piece_seen[idx(nx, ny)] || seen8[idx(nx, ny)] {
                        continue;
                    }
                    if is_ground(world, nx, ny) && world.materials.get(world.get(nx, ny).material).self_supporting {
                        piece_seen[idx(nx, ny)] = true;
                        st.push((nx, ny));
                    }
                }
            }
        }
    }
    f
}

fn row(label: &str, f: Hang) {
    println!(
        "{label:<26} hang {:>5} in {:>4} piece(s) | hang4 {:>5} | over {:>5} | loose {:>6} packed {:>6} | anchors {:>4}",
        f.hang, f.pieces, f.hang4, f.over, f.loose, f.packed, f.anchors
    );
}

/// A bare box with the bank carved and hand-placed ground, so every column
/// above is read against a case whose answer is known -- in **both**
/// directions, which is the half `CLAUDE.md` records this repo as repeatedly
/// missing: quiet where nothing is wrong, and moving where something is.
fn selftest() {
    let spec = LabBox { colonies: 0, founders: 0, ..LabBox::default() };
    let mut world = spec.build();
    let (w, h) = (spec.width, spec.height);
    let packed = world.materials.id_of("packedsoil").expect("packedsoil is compiled in");
    let cx = w / 2;

    // --- control 1: the bare box ------------------------------------------
    // Nothing has been placed and nothing has been cut. Every cell of the
    // bank is standing on the cell below it down to the floor.
    let base = census(&world, w, h);
    row("bare box", base);
    assert!(base.anchors > 0, "the flood found no floor to start from -- this harness cannot read this scene, and every other number on the line is meaningless");
    assert_eq!((base.hang, base.pieces, base.hang4), (0, 0, 0), "a bank resting on the box floor has nothing hanging in it");

    // --- control 2: a gallery with a packed roof, the case known fine -----
    // **The specificity control, and the one that matters most.** Every cell
    // of a gallery roof has air directly beneath it -- that is what a roof is
    // -- so a shape-based reading of "standing on nothing" counts the whole
    // nest. This must read zero.
    for x in cx - 6..=cx + 6 {
        for y in spec.ground_y + 8..=spec.ground_y + 10 {
            world.set(x, y, Cell::EMPTY);
        }
        world.set(x, spec.ground_y + 7, Cell::new(packed, 0));
        world.set(x, spec.ground_y + 11, Cell::new(packed, 0));
    }
    let gallery = census(&world, w, h);
    row("...gallery cut and lined", gallery);
    assert_eq!(gallery.hang, 0, "a lined gallery is worked ground attached to the bank, and none of it is hanging however much air is under the roof");
    assert!(gallery.over >= 13, "thirteen roof cells with air beneath them must show up as overhang -- if they do not, the column is blind and cannot say what a fix costs");

    // --- control 3: a floating block, the positive control ----------------
    // Four packed cells in open sky, forty rows up, touching nothing. The
    // census must report exactly four, in one piece.
    let sky = spec.ground_y - 40;
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        world.set(cx + 20 + dx, sky + dy, Cell::new(packed, 0));
    }
    let floater = census(&world, w, h);
    row("...2x2 block in open sky", floater);
    assert_eq!((floater.hang, floater.pieces), (4, 1), "four hand-placed cells with nothing under them are four hanging cells in one piece -- this is the number the whole build is argued from");
    assert_eq!(floater.over, gallery.over, "a block in the sky is not an overhang: it is not anchored at all, and the two columns must not double-count");

    // --- control 4: 4- against 8-connectivity -----------------------------
    // A packed cell touching the world only by a single corner: held on the
    // forgiving reading and hanging on the strict one, which is the whole
    // difference between the two columns. If they never disagree, one of them
    // is not being computed.
    //
    // **Built rather than assumed, and that is the lesson of this control.**
    // The first version placed one cell at `ground_y - 1` and called it
    // corner-hung; it was sitting squarely on the bank, and every column read
    // unchanged -- a control that measured nothing and looked like a pass. So
    // the air beneath is asserted before the reading is trusted.
    let hx = cx - 20;
    world.set(hx, spec.ground_y - 1, Cell::new(packed, 0));
    world.set(hx, spec.ground_y - 2, Cell::new(packed, 0));
    let (tx, ty) = (hx + 1, spec.ground_y - 3);
    assert!(world.get(tx, ty + 1).is_empty() && world.get(tx + 1, ty + 1).is_empty(), "the test cell must have air under it, or this control is about a cell standing on the bank");
    world.set(tx, ty, Cell::new(packed, 0));
    let corner = census(&world, w, h);
    row("...one cell hung by a corner", corner);
    assert_eq!(corner.hang, floater.hang, "a cell touching a standing pillar diagonally is attached on the 8-connected reading");
    assert_eq!(corner.hang4, floater.hang4 + 1, "...and hanging on the 4-connected one -- if these two columns never disagree, one of them is not being computed");

    // --- control 5: sensitivity -- take the support away -----------------
    // **The half this repo records itself as missing.** The controls above
    // check the number stays quiet; this checks it *moves*. A packed cap on a
    // packed pillar, then the pillar deleted: the cap must go from anchored to
    // hanging with nothing else in the world changing.
    let (px, py) = (cx - 30, spec.ground_y - 1);
    world.set(px, py, Cell::new(packed, 0));
    world.set(px, py - 1, Cell::new(packed, 0));
    let propped = census(&world, w, h);
    row("...a capped pillar, propped", propped);
    assert_eq!(propped.hang, corner.hang, "a pillar standing on the bank is standing on something");
    world.set(px, py, Cell::EMPTY);
    let undermined = census(&world, w, h);
    row("...the pillar undermined", undermined);
    assert_eq!(undermined.hang, corner.hang + 1, "the cap is now standing on nothing and the census must say so -- a number that cannot move is not a measurement");
    assert_eq!(undermined.pieces, corner.pieces + 1, "and it is its own piece");

    println!("hangcensus selftest: PASS -- quiet on a lined gallery, exact on a hand-placed floater, and it moves when the support goes");
}

fn main() {
    let mode: String = arg("mode").unwrap_or_else(|| "run".to_string());
    if mode == "selftest" {
        selftest();
        return;
    }
    let frames: u64 = arg("frames").unwrap_or(120_000);
    let sample: u64 = arg("sample").unwrap_or(20_000);
    let scenario_name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let mut scenario = Scenario::load(&scenario_name).unwrap_or_else(|e| {
        eprintln!("scenario {scenario_name}: {e}");
        std::process::exit(2);
    });
    if let Some(sd) = arg::<u64>("seed") {
        scenario.bed.seed = sd;
    }
    let spec = scenario.bed.clone();
    // **Echoed before a frame runs, `plant_probe`'s rule**: a log that does
    // not name its own arm was written by a binary that never had one, and
    // eight byte-identical logs are what that looks like from the outside.
    println!(
        "hangcensus: scenario={} seed={} frames={frames} sample={sample} threads={} footing={}",
        scenario.name,
        spec.seed,
        std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "default".into()),
        std::env::var("PIXEL_PHYSICS_SPOIL_FOOTING").unwrap_or_else(|_| "on (default)".into()),
    );
    let (mut world, planted, placed) = scenario.build();
    println!(
        "  bed: {} of {} founders planted; scenario placed {} cells, {} plants, {} animals",
        planted.planted, planted.asked, placed.cells, placed.plants, placed.animals
    );
    let ids = Ids::resolve(&world);
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    for f in 0..=frames {
        pixel_physics::lab::scenario::tick_timeline(&scenario, &mut world, &spec);
        if f % sample == 0 {
            let s = census::census(&world, &spec, 0.0, &[spec.width / 2], &ids);
            let hang = census(&world, spec.width, spec.height);
            row(&format!("frame {f}"), hang);
            println!(
                "{:<26} ants {:>5} roofed {:>6} pit {:>5} pack^ {:>6} pack< {:>6} mound_high {:>3} digs {:>7} dumped {:>7}",
                "", s.ants, s.roofed, s.pit, s.packed_above, s.packed_below, s.mound_high, world.creature_stats.digs, world.creature_stats.spoil_dumped
            );
        }
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
    }
    let hang = census(&world, spec.width, spec.height);
    let s = census::census(&world, &spec, 0.0, &[spec.width / 2], &ids);
    row(&format!("frame {frames} (final)"), hang);
    // **One line every arm of this can be grepped out of.** The SUMMARY
    // convention: `main`'s fields first, new ones appended, never interleaved.
    println!(
        "SUMMARY hang={} pieces={} hang4={} over={} loose={} packed={} ants={} roofed={} packed_above={} mound_high={} digs={} dumped={} seed={} frames={}",
        hang.hang, hang.pieces, hang.hang4, hang.over, hang.loose, hang.packed,
        s.ants, s.roofed, s.packed_above, s.mound_high,
        world.creature_stats.digs, world.creature_stats.spoil_dumped, spec.seed, frames
    );
}
