//! **Can two clones grow *identically*?** — the owner's question, asked as a
//! yes/no rather than as a variance.
//!
//! `clone_variance` answers *how much* of the difference between two plants is
//! their genome; it reports coefficients of variation, and a CV is a summary
//! that cannot say whether the remaining scatter is one cell or a whole crown.
//! The owner's complaint after two rounds of that instrument was that a clone
//! bed still shows "an extreme amount of variability", and the sharpest form
//! of the question is: **hold the environment as still as the engine allows,
//! and do two copies of one plant come out the same?**
//!
//! So this harness measures identity rather than spread, and its headline is a
//! **divergence frame**: the first moment at which two clones differ by one
//! cell. A pair that never diverges is the answer; a pair that diverges at
//! frame 900 names the moment to go and look at, which no variance can.
//!
//! **Why a divergence frame and not a final-shape distance.** A final Jaccard
//! of 0.6 is compatible with "they were never the same" and with "they were
//! identical for 20,000 frames and then one branch broke". Those want
//! completely different fixes, and only the onset separates them. The final
//! distance is printed too, because the onset alone cannot say whether the
//! divergence stayed small.
//!
//! **The comparison is translation-invariant, and has to be.** Two plants at
//! different `x` are the same plant only up to where they stand, so every
//! cell is expressed relative to its own plant's collar before anything is
//! compared. A comparison in world coordinates would report two perfect
//! clones as sharing nothing.
//!
//! ```text
//! cargo run --release --example clone_identity -- species=tree plants=2 frames=8000
//! cargo run --release --example clone_identity -- species=tree plants=2 frames=8000 gap=200
//! ```
use pixel_physics::render::Renderer;
use pixel_physics::sim::{organism, parallel, plant, World};
use std::collections::{BTreeMap, BTreeSet};

#[path = "common/mod.rs"]
mod common;

/// One plant's cells, expressed relative to its own collar. A `BTreeSet` so
/// two bodies compare with `==` and the printed order is stable.
type Body = BTreeSet<(i32, i32)>;

/// One arm of an ablation: the column it was planted at, the organism id it
/// was given, and what grew.
type Arm = (i32, u16, Body);

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}="))?.parse::<T>().ok())
}

fn sarg(name: &str) -> Option<String> {
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.to_string()))
}

/// Every organism-owned cell in the world, grouped by organism and expressed
/// **relative to that organism's own collar**.
///
/// `origin` is where the plant germinated, stamped by `plant::stamp_origin`,
/// and is the frame of reference the developmental key already uses -- so two
/// clones that grew the same shape have identical sets here whatever column
/// they are standing in.
fn relative_bodies(w: &World) -> BTreeMap<u16, Body> {
    let mut out: BTreeMap<u16, Body> = BTreeMap::new();
    let Some(b) = w.bounds() else { return out };
    for x in b.min_x..=b.max_x {
        for y in b.min_y..=b.max_y {
            let id = w.get(x, y).organism_id();
            if id == 0 {
                continue;
            }
            let Some((ox, oy)) = w.organism(id).and_then(|s| s.origin) else { continue };
            out.entry(id).or_default().insert((x - ox, y - oy));
        }
    }
    out
}

/// Shared cells over union -- 1.0 is identical, 0.0 is disjoint.
fn jaccard(a: &Body, b: &Body) -> f32 {
    let inter = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;
    if union == 0.0 {
        1.0
    } else {
        inter / union
    }
}

/// One plant, rendered in a **fixed window anchored on its own collar**, so
/// every panel of a column strip is the same size and the same crop.
///
/// A per-plant tight crop would be the wrong picture here: the question the
/// strip asks is *how much do these differ*, and a crop that rescales itself
/// to each plant hides exactly the difference being asked about -- a plant
/// twice the size would come back the same size on the card. So the window is
/// constant and the plant is drawn at whatever fraction of it it fills.
///
/// Pinned to noon for the same reason `clone_variance::render_stand` is: the
/// day/night cycle is a designed oscillator, and a card rendered at an
/// arbitrary phase is a card about the hour it was taken.
fn render_window(w: &World, cx: i32, gy: i32, half_w: i32, up: i32, down: i32) -> (Vec<u8>, u32, u32) {
    let b = w.bounds().expect("the plant scene sets bounds");
    let (ww, wh) = ((b.max_x - b.min_x + 1) as u32, (b.max_y - b.min_y + 1) as u32);
    let mut buf = vec![0u8; (ww * wh * 4) as usize];
    let mut renderer = Renderer::new();
    renderer.pinned_light = Some(pixel_physics::sky::frame_for_daylight(1.0));
    let particles = pixel_physics::sim::particle::ParticleSystem::new();
    renderer.draw(w, &particles, &std::collections::HashSet::new(), &mut buf, (ww, wh), true);
    let (x0, x1) = ((cx - half_w).max(b.min_x), (cx + half_w).min(b.max_x));
    let (y0, y1) = ((gy - up).max(b.min_y), (gy + down).min(b.max_y));
    let (cw, ch) = ((x1 - x0 + 1) as u32, (y1 - y0 + 1) as u32);
    let mut crop = vec![0u8; (cw * ch * 4) as usize];
    for row in 0..ch {
        let sy = (y0 - b.min_y) as u32 + row;
        let sx = (x0 - b.min_x) as u32;
        let src = ((sy * ww + sx) * 4) as usize;
        let dst = (row * cw * 4) as usize;
        crop[dst..dst + (cw * 4) as usize].copy_from_slice(&buf[src..src + (cw * 4) as usize]);
    }
    (crop, cw, ch)
}

fn main() {
    let species = sarg("species").unwrap_or_else(|| "tree".to_string());
    let plants: usize = arg("plants").unwrap_or(2);
    let frames: u64 = arg("frames").unwrap_or(8_000);
    let worldseed: u64 = arg("worldseed").unwrap_or(1);
    // How far apart the two stand, in columns. Wide by default: the question
    // is what two clones do when nothing about the bed distinguishes them, and
    // a neighbour's shade is a difference between them.
    let gap: i32 = arg("gap").unwrap_or(220);
    let check_every: u64 = arg("check").unwrap_or(50);
    let sterile: bool = arg::<u32>("sterile").unwrap_or(1) != 0;
    // **The developmental key under test.** `0` is `Plant { coarseness: 0 }`,
    // which drops the germination coordinate from the growth key entirely --
    // the end of the dial at which two clones *should* be able to be
    // identical. `world` is the shipped key, where position is in the key and
    // two plants in different columns are different plants by construction.
    // **The ablation ladder.** Two clones were identical to frame 1,500 and
    // then diverged, so something is breaking a symmetry that held. Each of
    // these removes one candidate, and the one that stops the divergence is
    // the answer -- which is cheaper and far more conclusive than reasoning
    // about which of them *ought* to matter.
    //
    // `weather=clear` pins the sky, so there is no gust and no rain to arrive
    // at one plant a frame before the other. `daynight=1` pins the world's
    // start frame to a multiple of the day period, which `PlantScene`'s own
    // doc says fixes the day phase and every organism's tick offset together.
    let clear: bool = arg::<u32>("clear").unwrap_or(0) != 0;
    // **`solo=1` gives each clone its own world.** The strongest form of the
    // question: not "do two plants sharing a bed stay identical" -- they can
    // shade each other, and each is part of the other's environment -- but
    // "does the same plant, alone, in the same world, grown at a different
    // column, come out the same plant?" Everything two co-existing plants can
    // do to each other is gone, so anything left is position leaking into
    // development, which at `dev=0` is supposed to be impossible.
    //
    // Built by deleting the other seeds rather than by building a
    // one-tree scene per position: the bed then really is identical between
    // arms, down to the world seed and the soil, and only which seed survives
    // differs. A scene rebuilt per position would vary its own width and be a
    // different world.
    let solo: bool = arg::<u32>("solo").unwrap_or(0) != 0;
    let devarg = sarg("dev").unwrap_or_else(|| "0".to_string());
    let key = if devarg == "world" {
        organism::DevelopmentalKey::World
    } else {
        organism::DevelopmentalKey::Plant { coarseness: devarg.parse().expect("dev=world or an integer") }
    };

    println!(
        "clone_identity: species={species} plants={plants} frames={frames} worldseed={worldseed} gap={gap} \
         dev={devarg} sterile={sterile} clear={clear}"
    );

    let width = gap * (plants as i32 + 1);
    let scene = common::PlantScene {
        trees: plants,
        width,
        species: species.clone(),
        seed: Some(worldseed),
        ..Default::default()
    };
    let mut w = scene.build();
    w.developmental_key = key;
    if clear {
        // The negative control for "the weather broke the tie". `Pin::Clear`
        // is the zero arm of the pin set -- measured at 0 gusts delivered and
        // 0 bolts against BREEZE's 230 and STORM's 1.
        w.set_weather_pin(pixel_physics::sim::weather::Pin::Clear);
    }
    if sterile {
        let sp = w.species.id_of(&species).expect("species is compiled in");
        assert!(
            w.species.set_param(sp, organism::CellType::MatureBody, organism::ParamId::SeedMaturity, 0, 1.0e9),
            "sterile matched no Reproduce behaviour on {species}"
        );
    }

    // The founders, in column order, with where each was planted.
    let mut ids: Vec<(u16, i32, i32)> = Vec::new();
    if let Some(b) = w.bounds() {
        for x in b.min_x..=b.max_x {
            for y in b.min_y..=b.max_y {
                let id = w.get(x, y).organism_id();
                if id != 0 && !ids.iter().any(|&(i, _, _)| i == id) {
                    ids.push((id, x, y));
                }
            }
        }
    }
    assert!(ids.len() >= 2, "need at least two founders, found {}", ids.len());

    // **One genome and one lineage seed across all of them**, written through
    // `set_organism_genotype` so `inherited` is set -- otherwise germination
    // redraws the genotype from the resting coordinate and the arm is vacuous.
    // `seed_genotype` first, because a founder holds the species mean until
    // something draws one and cloning that would clone nothing.
    let (src, sx, sy) = ids[0];
    plant::seed_genotype(&mut w, src, sx, sy);
    let (draws, alleles, params, dev) = w.organism_genotype(src).expect("the reference has a genome");
    for &(id, _, _) in &ids {
        w.set_organism_genotype(id, draws, alleles, params, dev);
    }
    let distinct: BTreeSet<u64> = ids.iter().filter_map(|&(id, _, _)| w.organism_genotype(id).map(|g| g.3)).collect();
    println!("  {} founders carrying {} distinct developmental seed(s)", ids.len(), distinct.len());

    // **`solo=1`: one world per plant, everything else held.** Each arm keeps
    // exactly one founder and deletes the rest before a frame runs, so the
    // survivor grows alone in a bed that is otherwise identical to every other
    // arm's. The comparison afterwards is the same translation-invariant one.
    if solo {
        let mut bodies: Vec<Arm> = Vec::new();
        for &(keep, kx, _) in &ids {
            let mut wk = scene.build();
            wk.developmental_key = key;
            if clear {
                wk.set_weather_pin(pixel_physics::sim::weather::Pin::Clear);
            }
            if sterile {
                let sp = wk.species.id_of(&species).expect("species is compiled in");
                assert!(wk.species.set_param(
                    sp,
                    organism::CellType::MatureBody,
                    organism::ParamId::SeedMaturity,
                    0,
                    1.0e9
                ));
            }
            for &(id, x, y) in &ids {
                if id == keep {
                    plant::seed_genotype(&mut wk, id, x, y);
                    wk.set_organism_genotype(id, draws, alleles, params, dev);
                } else {
                    wk.set(x, y, pixel_physics::sim::Cell::EMPTY);
                }
            }
            for _ in 0..frames {
                parallel::step(&mut wk);
                wk.step_active_sites();
                wk.step_fields();
            }
            let b = relative_bodies(&wk).remove(&keep).unwrap_or_default();
            println!("  solo world keeping organism {keep} (planted x={kx}): {} cells", b.len());
            bodies.push((kx, keep, b));
        }
        println!("\n  --- each grown ALONE, same world, different column ---");
        println!("  {:<10} {:>10} {:>10} {:>12}", "pair", "cells", "shared", "jaccard");
        let mut all_same = true;
        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                let (a, b) = (&bodies[i].2, &bodies[j].2);
                if a != b {
                    all_same = false;
                }
                println!(
                    "  {:<10} {:>10} {:>10} {:>12.4}",
                    format!("x{}-x{}", bodies[i].0, bodies[j].0),
                    format!("{}/{}", a.len(), b.len()),
                    a.intersection(b).count(),
                    jaccard(a, b)
                );
            }
        }
        println!(
            "\n  {}",
            if all_same {
                "IDENTICAL: the same plant grown alone at different columns is cell-for-cell the same plant"
            } else {
                "NOT IDENTICAL: column alone changes the plant, with no neighbour to blame"
            }
        );
        return;
    }

    // **`columns=a,b,c`: one plant, one world, planted at each column in
    // turn.** The tightest control in the file, and the one that separates
    // the two things `solo=1` moves at once. `solo` keeps founder 1 in one
    // world and founder 2 in another, so *column* and *organism id* change
    // together -- and the id is not inert: `CLAUDE.md` records that a plant's
    // tick offset is `(frame + id) % ORGANISM_TICK_INTERVAL`, so two ids tick
    // on different frames and read the world at different instants.
    //
    // Here every scene founder is deleted first and exactly one tree is
    // planted, so each arm allocates the same organism id and only the column
    // differs. If the plants still differ, position reaches development
    // through something that is not the growth key, the neighbours, the
    // weather, or the id.
    if let Some(list) = sarg("columns") {
        let cols: Vec<i32> = list.split(',').map(|c| c.parse().expect("columns are comma-separated integers")).collect();
        let mut bodies: Vec<Arm> = Vec::new();
        for &cx in &cols {
            let mut wk = scene.build();
            wk.developmental_key = key;
            if clear {
                wk.set_weather_pin(pixel_physics::sim::weather::Pin::Clear);
            }
            if sterile {
                let sp = wk.species.id_of(&species).expect("species is compiled in");
                assert!(wk.species.set_param(
                    sp,
                    organism::CellType::MatureBody,
                    organism::ParamId::SeedMaturity,
                    0,
                    1.0e9
                ));
            }
            for &(_, x, y) in &ids {
                wk.set(x, y, pixel_physics::sim::Cell::EMPTY);
            }
            let gy = ids[0].2;
            assert!(wk.plant_tree_species(cx, gy, &species), "nothing planted at column {cx}");
            let planted: Vec<u16> = {
                let mut v = Vec::new();
                if let Some(b) = wk.bounds() {
                    for x in b.min_x..=b.max_x {
                        for y in b.min_y..=b.max_y {
                            let id = wk.get(x, y).organism_id();
                            if id != 0 && !v.contains(&id) {
                                v.push(id);
                            }
                        }
                    }
                }
                v
            };
            assert_eq!(planted.len(), 1, "column {cx} has {} organisms, expected exactly one", planted.len());
            let id = planted[0];
            plant::seed_genotype(&mut wk, id, cx, gy);
            wk.set_organism_genotype(id, draws, alleles, params, dev);
            for _ in 0..frames {
                parallel::step(&mut wk);
                wk.step_active_sites();
                wk.step_fields();
            }
            let b = relative_bodies(&wk).remove(&id).unwrap_or_default();
            // **What the count counts, printed beside it.** Column 285 measured
            // 151 cells and rendered a full tree -- a number and a picture
            // answering different questions, which is `CLAUDE.md`'s standing
            // trap. `relative_bodies` counts cells the organism still *owns*;
            // a senescent plant's tissue keeps rendering as wood and foliage
            // while `rot_remains` carries it out, and dead cells stop being
            // owned. So a small count beside a big picture is a plant that
            // died, not an instrument fault -- but only these three numbers
            // can tell those apart.
            let (alive, senescent, owned) = match wk.organism(id) {
                Some(st) => (true, st.senescent, st.cells.len()),
                None => (false, false, 0),
            };
            println!(
                "  column {cx}: organism id {id}, {} cells | alive {alive} senescent {senescent} owned {owned}",
                b.len()
            );
            // **Every organism id standing in the world at the end, and the
            // plant material owned by none.** Column 285 measured 151 cells
            // and rendered a full crown -- 16 plant pixels per counted cell
            // where the other eleven columns read 0.92 to 1.04 -- so ~2,300
            // cells of wood and foliage are standing that this organism does
            // not own. Either they were disowned or they belong to a second
            // organism, and `cells` is wrong about the plant's size in both
            // cases. `dead-ends.md` records the neighbouring failure (a freed
            // slot leaving 160 orphan cells carrying its id) but not this one.
            {
                let mut per_id: BTreeMap<u16, usize> = BTreeMap::new();
                let mut unowned_plant = 0usize;
                if let Some(bb) = wk.bounds() {
                    for x in bb.min_x..=bb.max_x {
                        for y in bb.min_y..=bb.max_y {
                            let cell = wk.get(x, y);
                            let oid = cell.organism_id();
                            if oid != 0 {
                                *per_id.entry(oid).or_default() += 1;
                            } else if wk.materials.kind(cell.material) == pixel_physics::sim::material::MaterialKind::Plant {
                                unowned_plant += 1;
                            }
                        }
                    }
                }
                println!("    ids standing: {per_id:?} | unowned plant cells {unowned_plant}");
            }
            if let Some(stem) = sarg("png") {
                // **The window is an argument because the right one depends on
                // the age.** At 6,000 frames a `tree` is a whip and a tight
                // window is generous; at 20,000 it has a crown and the same
                // window crops it, which would make a big plant *look* the
                // same size as a small one -- the exact difference the strip
                // exists to show.
                let win = sarg("win").unwrap_or_else(|| "70,150,22".to_string());
                let v: Vec<i32> = win.split(',').map(|n| n.parse().expect("win=halfw,up,down")).collect();
                assert_eq!(v.len(), 3, "win takes three numbers: halfw,up,down");
                let (buf, pw, ph) = render_window(&wk, cx, gy, v[0], v[1], v[2]);
                let path = format!("{stem}_{cx}.png");
                image::save_buffer(&path, &buf, pw, ph, image::ColorType::Rgba8).expect("write png");
                println!("    wrote {path} ({pw}x{ph})");
            }
            bodies.push((cx, id, b));
        }
        let one_id = bodies.iter().all(|b| b.1 == bodies[0].1);
        println!(
            "\n  --- one plant, one world, planted at each column (organism id {}) ---",
            if one_id { format!("{} in every arm", bodies[0].1) } else { "DIFFERS between arms -- not a clean control".into() }
        );
        println!("  {:<14} {:>10} {:>10} {:>12}", "pair", "cells", "shared", "jaccard");
        let mut all_same = true;
        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                let (a, b) = (&bodies[i].2, &bodies[j].2);
                if a != b {
                    all_same = false;
                }
                println!(
                    "  {:<14} {:>10} {:>10} {:>12.4}",
                    format!("x{}-x{}", bodies[i].0, bodies[j].0),
                    format!("{}/{}", a.len(), b.len()),
                    a.intersection(b).count(),
                    jaccard(a, b)
                );
            }
        }
        println!(
            "\n  {}",
            if all_same {
                "IDENTICAL: with the id held, column does not change the plant"
            } else {
                "NOT IDENTICAL: column changes the plant with the organism id held fixed"
            }
        );
        return;
    }

    // Step, checking for the first frame at which any two of them differ.
    let mut diverged_at: Option<u64> = None;
    let mut first_diff: Option<(u16, u16, usize)> = None;
    for f in 0..frames {
        parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
        if diverged_at.is_some() || (f + 1) % check_every != 0 {
            continue;
        }
        let bodies = relative_bodies(&w);
        let live: Vec<(&u16, &BTreeSet<(i32, i32)>)> =
            ids.iter().filter_map(|&(id, _, _)| bodies.get_key_value(&id)).collect();
        // Only compare once every founder has actually germinated: a plant
        // that is still a seed has no body, and "one has a body and one does
        // not" is a difference in *timing* that the next block reports
        // separately and by name.
        if live.len() < ids.len() {
            continue;
        }
        for i in 0..live.len() {
            for j in (i + 1)..live.len() {
                if live[i].1 != live[j].1 {
                    let d = live[i].1.symmetric_difference(live[j].1).count();
                    diverged_at = Some(w.frame);
                    first_diff = Some((*live[i].0, *live[j].0, d));
                }
            }
        }
    }

    println!("\n  --- what the engine gave each of them ---");
    println!("  {:<6} {:>8} {:>10} {:>14} {:>12}", "id", "planted", "origin", "germ frame", "tick phase");
    for &(id, px, _) in &ids {
        let Some(s) = w.organism(id) else {
            println!("  {id:<6} {px:>8}   (gone)");
            continue;
        };
        let o = s.origin.map(|(a, b)| format!("{a},{b}")).unwrap_or_else(|| "-".into());
        println!(
            "  {:<6} {:>8} {:>10} {:>14} {:>12}",
            id,
            px,
            o,
            s.germination_frame,
            (s.germination_frame + id as u64) % plant::ORGANISM_TICK_INTERVAL
        );
    }

    let bodies = relative_bodies(&w);
    println!("\n  --- how alike they ended up ---");
    println!("  {:<10} {:>8} {:>10} {:>12}", "pair", "cells", "shared", "jaccard");
    let live: Vec<(u16, &BTreeSet<(i32, i32)>)> =
        ids.iter().filter_map(|&(id, _, _)| bodies.get(&id).map(|b| (id, b))).collect();
    for i in 0..live.len() {
        for j in (i + 1)..live.len() {
            let (a, b) = (live[i].1, live[j].1);
            println!(
                "  {:<10} {:>8} {:>10} {:>12.4}",
                format!("{}-{}", live[i].0, live[j].0),
                format!("{}/{}", a.len(), b.len()),
                a.intersection(b).count(),
                jaccard(a, b)
            );
        }
    }

    match (diverged_at, first_diff) {
        (Some(f), Some((a, b, d))) => {
            println!("\n  FIRST DIVERGENCE: frame {f}, organisms {a} and {b}, {d} cell(s) apart");
            println!("  (checked every {check_every} frames, so the true onset is within that window)");
        }
        _ if live.len() < 2 => println!("\n  FIRST DIVERGENCE: not measurable -- fewer than two plants survived"),
        _ => println!("\n  NO DIVERGENCE: every pair was cell-for-cell identical at every check to frame {frames}"),
    }
}
