//! Where shed litter actually comes to rest, and whether it rots.
//!
//! **The count `Reports/plant-implementation-plan.md`'s WP-B2 deliberately
//! did not run.** That package landed litter, switched all three abscission
//! sites over and closed the decay-site blocker, then recorded: *"Not run:
//! the edible-cells-near-surface count. It is a creature-side quantity and
//! the creature branch sets its own bar when it consumes this."* This is
//! that harness.
//!
//! The question is not "is there litter" — a total says yes and means
//! nothing. What matters is *where it stopped*, and the split that answers
//! it is *what is holding each cell up*.
//!
//! **This harness predates the fix it was built to find.** When it was
//! written, `plant.rs::shed_to_litter` wrote litter in place, at the leaf's
//! own position, and let the powder fall from there — so a leaf shed in the
//! middle of a crown landed on the first branch under it and stayed, and
//! 3,825 of 4,330 standing cells were resting on plant tissue. It now walks
//! the leaf down through its own crown to where it would have come to rest.
//!
//!   - **on terrain** — it reached the ground. A walking creature can touch
//!     it; a forest floor accumulates.
//!   - **against plant** — the cell underneath is a live organism cell.
//!   - **airborne** — still falling this frame.
//!
//! **`against plant` is two opposite verdicts wearing one number, and
//! reading it as the bad one cost a whole detour.** A drift piled against a
//! trunk at floor level rests *on the trunk*, because a litter cell is a grid
//! cell and cannot go behind a tree the way the gnome can — he is an entity
//! with his own collision rules, it is a material, and two materials cannot
//! share a cell. That case is `litter.ron`'s 42-degree friction angle working
//! as designed. Litter genuinely caught in the canopy is the mechanism
//! failing. They score identically here.
//!
//! **So this column is only meaningful beside the height bands**, which are
//! what tell the two apart, and it must never be quoted alone. Measured on
//! this scene at 12,000 frames: 39.3% against plant, and 88% of all litter
//! within four rows of the ground — so nearly all of it is round trunk bases.
//!
//! Also prints the 3-rows-above-terrain count, which is the form the
//! creature side's earlier reading took, so the two are comparable.
//!
//! **`canopy top` is printed first and is a gate, not decoration.** Four
//! conclusions in this project came from harnesses that did not contain the
//! situation they claimed to measure, one of them a tree that never grew
//! because the runner omitted the light field. If nothing grew, every number
//! below it is void — and `litter 0` from a bare world is indistinguishable
//! from `litter 0` from a working one.
//!
//! `out=x.png` writes the classification as a picture: **litter resting on a
//! branch in magenta, litter resting on the ground in cyan**, over a world
//! dimmed to a quarter. **Read the picture, not just the split** — the shape
//! is the answer: magenta in vertical streaks hugging the trunks is litter
//! drifted against tree bases, and magenta scattered through the canopy is
//! litter genuinely stuck up a tree. A plain screenshot cannot show either,
//! because litter and wood are both browns. So
//! this is a **full replace on fixed colours**, not a tint: a magnitude
//! blend into a brown cell was tried once elsewhere in this engine and
//! produced a sheet that read as blank.
//!
//! ```text
//! cargo run --release --example litter_probe
//! cargo run --release --example litter_probe -- frames=12000 every=2000 trees=8
//! cargo run --release --example litter_probe -- frames=12000 out=/tmp/litter.png
//! cargo run --release --example litter_probe -- out=/tmp/l.png crop=140,80,200,140 zoom=4
//! ```

mod common;

use pixel_physics::sim::material;
use pixel_physics::sim::parallel;
use pixel_physics::sim::World;

/// What is underneath a litter cell, once you walk down through whatever
/// litter is stacked below it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rest {
    Terrain,
    Plant,
    Airborne,
}

/// Walk down from `(x, y)` through contiguous litter and report what the
/// bottom of that column is standing on.
///
/// **Through the pile, not one cell.** The cell directly below a litter cell
/// is usually more litter, so a one-cell test answers "is this a pile"
/// rather than "what is holding the pile up". A drift resting on a branch
/// and the same drift on the floor look identical until the walk bottoms
/// out.
fn rest_of(world: &World, litter: pixel_physics::sim::material::MaterialId, x: i32, y: i32, max_y: i32) -> Rest {
    let mut yy = y;
    while yy < max_y {
        let below = world.get(x, yy + 1);
        if below.material == litter {
            yy += 1;
            continue;
        }
        // Raw `material == EMPTY`, not `is_empty()`: the managed-aware
        // helper reads a promoted liquid body's container cells as
        // not-empty, and the question here is "is there material here".
        if below.material == material::EMPTY {
            return Rest::Airborne;
        }
        if below.organism_id() != 0 {
            // **"Resting on plant" is not "stranded up a tree", and reading
            // it as one cost a whole detour.** A drift that has piled against
            // a trunk at floor level rests on the trunk, because a litter
            // cell *is* a grid cell and cannot be behind the tree the way the
            // gnome can -- he is an entity with his own collision rules, it
            // is a material, and two materials cannot share a cell. So this
            // bucket counts the drift-against-a-trunk case and the
            // caught-in-the-canopy case together, and they are opposite
            // verdicts: the first is `litter.ron`'s 42-degree friction angle
            // working, the second is the mechanism failing.
            //
            // The height bands below are what separate them, so **read this
            // number with them and never on its own**. Measured on this scene
            // at 12,000 frames: 39.3% rests on plant, and 88% of all litter
            // is within four rows of the ground -- so nearly all of that 39%
            // is piled round trunk bases, which is what a forest floor does.
            return Rest::Plant;
        }
        return Rest::Terrain;
    }
    // Bottomed out against the world edge, which is terrain for this
    // purpose — nothing can be resting on a branch down there.
    Rest::Terrain
}

/// **Is there air underneath this cell, anywhere between it and the ground?**
///
/// This is the question the other two columns cannot answer, and adding it is
/// the correction to a card that went to the owner and came back *"still
/// didn't look like the leaves were all on the floor"*.
///
/// Both existing measures conflate a leaf stuck up a tree with a leaf lying on
/// the forest floor, by two different routes:
///
/// - `rest_of` reports `Plant` for a drift banked against a root collar, which
///   is at floor level and is the design working;
/// - the height bands measure against `terrain_top`, which **excludes litter
///   on purpose**, so a leaf resting on top of a twenty-deep mat reads as
///   "twenty rows above the ground" while being the top of the floor.
///
/// Neither is wrong; both are confounded, and a picture painted from either
/// reads as "still stuck in the tree" whichever it is. Air underneath is not
/// confounded: a leaf held up by a branch has a gap below it and a leaf on a
/// pile does not, however deep the pile or whatever the pile rests on.
///
/// Returns the count of empty cells strictly between `(x, y)` and the terrain
/// top, so the size of the gap is available as well as its existence -- a
/// one-cell hole under a slumping drift and a forty-cell drop out of a crown
/// are different claims.
fn air_below(world: &World, x: i32, y: i32, terrain_y: i32) -> i32 {
    ((y + 1)..=terrain_y)
        .filter(|&yy| world.get(x, yy).material == material::EMPTY)
        .count() as i32
}

/// Topmost terrain row in column `x` — soil or stone, never plant and never
/// litter.
///
/// Litter is excluded on purpose: a mat of it *becomes* the walkable surface,
/// so including it would make a deep pile measure as "0 rows above the
/// surface" by definition and the metric could never report a pile at all.
fn terrain_top(world: &World, soil: material::MaterialId, x: i32, y0: i32, y1: i32) -> Option<i32> {
    (y0..=y1).find(|&y| {
        let m = world.get(x, y).material;
        m == material::STONE || m == soil
    })
}

fn main() {
    let mut frames = 6000u64;
    let mut every = 1000u64;
    let mut out: Option<String> = None;
    let mut crop: Option<(i32, i32, i32, i32)> = None;
    let mut zoom = 1i32;
    let mut plain = false;
    let mut scene = common::PlantScene::default();
    // Echo every parameter below, so a log that does not name its settings
    // was written by a binary that never had them — the megastudy that
    // produced eight byte-identical logs is why this line exists.
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            "every" => every = v.parse().expect("every"),
            "trees" => scene.trees = v.parse().expect("trees"),
            "species" => scene.species = v.to_string(),
            "ground" => scene.ground_y = v.parse().expect("ground"),
            "startframe" => scene.start_frame = v.parse().expect("startframe"),
            "out" => out = Some(v.to_string()),
            // `crop=x,y,w,h` and `zoom=N` on the overlay, because the answer
            // this harness produces is judged by eye and a 512x320 sheet with
            // the interesting part 180 px wide is not judgeable. The review
            // protocol (`.claude/skills/review/SKILL.md`) records a card the
            // owner could see nothing in for exactly this reason.
            "crop" => {
                let n: Vec<i32> = v.split(',').map(|t| t.parse().expect("crop=x,y,w,h")).collect();
                assert_eq!(n.len(), 4, "crop=x,y,w,h");
                crop = Some((n[0], n[1], n[2], n[3]));
            }
            "zoom" => zoom = v.parse().expect("zoom=N"),
            "plain" => plain = v != "0",
            other => panic!("unknown arg {other:?}; known: frames, every, trees, species, ground, startframe, out, crop, zoom, plain"),
        }
    }
    println!(
        "litter_probe: species={} trees={} frames={} every={} ground={} start_frame={} world={}x{}",
        scene.species, scene.trees, frames, every, scene.ground_y, scene.start_frame, scene.width, scene.height
    );

    let mut world = scene.build();
    let litter = world.materials.id_of("litter").expect("litter is a compiled-in material");
    let soil = world.materials.id_of("soil").expect("soil is a compiled-in material");
    let (w, h) = (scene.width, scene.height);

    // **Height above the local terrain, as a profile.** The on-terrain /
    // on-plant split answers "what is holding this up" and is the right
    // question for litter hanging in a crown -- but it mis-sorts a deep
    // drift piled against a trunk, which bottoms out on the root collar and
    // reads as "on a branch" while being unambiguously part of the forest
    // floor. `litter.ron` asks for exactly those drifts (`friction_angle:
    // 42.0` -- "a drift piles up against a trunk rather than running out to
    // a level sheet"), so they are the design working, not a defect.
    //
    // Height cannot be fooled that way, and it is the quantity a foraging
    // creature actually cares about: how far above the ground is the food.
    // A count needs a bar; this profile does not.
    const HEIGHT_BANDS: [i32; 6] = [1, 2, 4, 8, 16, 32];
    // `against-plant`, not `on-plant`: at floor level the thing a drift rests
    // on is usually a trunk, and the old label read as "stuck up a tree".
    println!("  frame | canopy |  litter | on-terrain  against-plant  airborne | <=3 rows | rotted (damp/dry)");
    let sample = |world: &World| {
        let canopy = common::canopy_top(world).map(|y| y.to_string()).unwrap_or_else(|| "NONE".into());
        let (mut total, mut on_terrain, mut on_plant, mut airborne, mut near) = (0u32, 0u32, 0u32, 0u32, 0u32);
        let mut bands = [0u32; HEIGHT_BANDS.len()];
        // The unconfounded pair: how many litter cells have air underneath
        // them, and how far off the ground the worst one is. See `air_below`.
        let (mut suspended, mut worst_gap, mut worst_height) = (0u32, 0i32, 0i32);
        let (mut high, mut high_boxed_by_plant) = (0u32, 0u32);
        for x in 0..w {
            let top = terrain_top(world, soil, x, 0, h - 1);
            for y in 0..h {
                if world.get(x, y).material != litter {
                    continue;
                }
                total += 1;
                match rest_of(world, litter, x, y, h - 1) {
                    Rest::Terrain => on_terrain += 1,
                    Rest::Plant => on_plant += 1,
                    Rest::Airborne => airborne += 1,
                }
                if let Some(t) = top {
                    if (t - y) <= 3 && y < t {
                        near += 1;
                    }
                    // Diagnostic for the follow-up complaint: litter that is
                    // grounded but *high* is a pile climbing off the floor,
                    // and what stops a pile spreading is a neighbour it
                    // cannot roll into. Counting how often that neighbour is
                    // plant tissue says whether the climb is confinement by
                    // the tree or ordinary repose.
                    if (t - y) > 8 {
                        high += 1;
                        let l = world.get(x - 1, y);
                        let r = world.get(x + 1, y);
                        if l.organism_id() != 0 || r.organism_id() != 0 {
                            high_boxed_by_plant += 1;
                        }
                    }
                    let gap = air_below(world, x, y, t);
                    if gap > 0 {
                        suspended += 1;
                        worst_gap = worst_gap.max(gap);
                        worst_height = worst_height.max(t - y);
                    }
                    // Cumulative: band `i` counts litter within
                    // `HEIGHT_BANDS[i]` rows of the terrain top. Litter at or
                    // below the terrain line (buried by a slump) counts as
                    // ground, which it is.
                    for (i, &b) in HEIGHT_BANDS.iter().enumerate() {
                        if (t - y) <= b {
                            bands[i] += 1;
                        }
                    }
                }
            }
        }
        println!(
            "  {:>6} | {:>6} | {:>7} | {:>10} {:>9} {:>9} | {:>8} | {} / {}",
            world.frame, canopy, total, on_terrain, on_plant, airborne, near, world.decayed_damp, world.decayed_dry
        );
        // **Read this line, not the split above it.** It is the only one that
        // answers "are the leaves on the floor" without conflating a drift
        // against a trunk, or the top of a deep mat, with a leaf up a tree.
        if total > 0 {
            println!(
                "         SUSPENDED (air underneath): {suspended} of {total} ({:.1}%), worst gap {worst_gap} cells, highest {worst_height} rows above terrain",
                100.0 * suspended as f32 / total as f32
            );
            println!(
                "         grounded but >8 rows up: {high} ({:.1}%), of which {high_boxed_by_plant} have plant tissue immediately left or right ({:.1}%)",
                100.0 * high as f32 / total as f32,
                if high > 0 { 100.0 * high_boxed_by_plant as f32 / high as f32 } else { 0.0 }
            );
        }
        if total > 0 {
            let pct: Vec<String> = HEIGHT_BANDS
                .iter()
                .zip(&bands)
                .map(|(b, n)| format!("<={b}: {:.0}%", 100.0 * *n as f32 / total as f32))
                .collect();
            println!("         within N rows of terrain -- {}", pct.join("  "));
        }
        (total, on_terrain, on_plant, near, suspended)
    };

    let mut run = 0u64;
    sample(&world);
    while run < frames {
        let chunk = every.min(frames - run);
        for _ in 0..chunk {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
        }
        run += chunk;
        sample(&world);
    }

    let (total, on_terrain, on_plant, near, suspended) = sample(&world);
    let rotted = world.decayed_damp + world.decayed_dry;
    println!();
    println!("  shed cells that ever existed (standing + rotted): {}", total + rotted);
    if total > 0 {
        println!(
            "  standing split: on-terrain {:.1}%  against-plant {:.1}% (read with the bands above -- mostly drifts round trunk bases)  |  within 3 rows of terrain {:.1}%",
            100.0 * on_terrain as f32 / total as f32,
            100.0 * on_plant as f32 / total as f32,
            100.0 * near as f32 / total as f32,
        );
        println!(
            "  SUSPENDED (air underneath, the unconfounded one): {:.1}% -- everything else is on the floor, whatever it is nominally resting on",
            100.0 * suspended as f32 / total as f32
        );
    }
    println!("  rotted: {} damp + {} dry = {}", world.decayed_damp, world.decayed_dry, rotted);

    if let Some(path) = out {
        write_overlay(&world, litter, soil, &path, &View { w, h, crop, zoom, plain });
        println!("  overlay: {path} (magenta = HELD UP off the ground, cyan = on the forest floor)");
    }
}

/// Paint the same classification the census counts, so the picture and the
/// number are the same quantity rather than two things that have to be
/// argued into agreement.
/// Where to look and how big to draw it — grouped rather than passed as four
/// more parameters, because `write_overlay` was one over clippy's
/// `too_many_arguments` limit the moment the soil id joined it.
struct View {
    /// Full world size, and the default crop.
    w: i32,
    h: i32,
    crop: Option<(i32, i32, i32, i32)>,
    zoom: i32,
    /// Draw the world in its own colours with no markers and no dimming —
    /// *what the scene actually looks like*, rather than the classification.
    ///
    /// Both are needed and they answer different questions. The marked
    /// overlay says which cells are held off the ground; this says whether a
    /// person looking at the screen would call it a forest floor. The whole
    /// reason this option exists is that a card carrying only the marked
    /// version was read, reasonably, as showing litter still in the canopy.
    ///
    /// Material palettes at full brightness, deliberately not the game's own
    /// lighting: this scene spends half its time at night, and a picture that
    /// is mostly darkness answers nothing.
    plain: bool,
}

fn write_overlay(world: &World, litter: material::MaterialId, soil_id: material::MaterialId, path: &str, view: &View) {
    let (w, h) = (view.w, view.h);
    let (cx, cy, cw, ch) = view.crop.unwrap_or((0, 0, w, h));
    let zoom = view.zoom.max(1);
    let (ow, oh) = (cw * zoom, ch * zoom);
    let mut buf = vec![0u8; (ow * oh * 3) as usize];
    for px in 0..cw {
        for py in 0..ch {
            let (x, y) = (cx + px, cy + py);
            let c = world.get(x, y);
            let rgb = if view.plain {
                let pal = &world.materials.get(c.material).palette;
                if c.material == material::EMPTY {
                    [10, 12, 20]
                } else {
                    let raw = pal[c.shade as usize % pal.len().max(1)];
                    [raw[0], raw[1], raw[2]]
                }
            } else if c.material == litter {
                // **Painted on `air_below`, not on `rest_of`.** The first
                // version of this overlay coloured by what the cell was
                // resting on, which put a floor-level drift banked against a
                // trunk in the same magenta as a leaf stuck forty rows up a
                // tree -- and the owner read the picture the way anyone
                // would, as litter still in the canopy. The caption said
                // otherwise and a caption does not beat a picture. Air
                // underneath is the honest split: magenta is genuinely held
                // up off the ground, cyan is the forest floor.
                match terrain_top(world, soil_id, x, 0, h - 1) {
                    Some(t) if air_below(world, x, y, t) > 0 => [255u8, 0, 220],
                    _ => [0, 230, 255],
                }
            } else if c.material == material::EMPTY {
                [10, 12, 20]
            } else {
                // A quarter of the material's own colour: enough to read the
                // trunk, the crown and the ground line as context, dim
                // enough that nothing competes with the two markers.
                let pal = &world.materials.get(c.material).palette;
                let raw = pal[c.shade as usize % pal.len().max(1)];
                [raw[0] / 4, raw[1] / 4, raw[2] / 4]
            };
            // Nearest-neighbour, deliberately: a litter cell is one pixel
            // and the whole question is where it is, so any smoothing
            // filter would blend the two markers into the dim world behind
            // them and invent colours that mean nothing.
            for dy in 0..zoom {
                for dx in 0..zoom {
                    let i = (((py * zoom + dy) * ow + px * zoom + dx) * 3) as usize;
                    buf[i..i + 3].copy_from_slice(&rgb);
                }
            }
        }
    }
    image::RgbImage::from_raw(ow as u32, oh as u32, buf)
        .expect("buffer is ow*oh*3")
        .save(path)
        .expect("write overlay png");
}
