//! **How much of the picture is a creature?** The appearance instrument for
//! `Reports/creature-appearance-design.md`.
//!
//! `Reports/plant-appearance-design.md` found that a silhouette is set by
//! **extent, composition and palette**, and that every architectural lever
//! the plant line built moved only *which cell gets a label*. A creature
//! sits at the opposite end of the same axis: an ant is a **two-cell**
//! chain, so it has no composition to move at all, and the two levers that
//! remain are extent and palette.
//!
//! This harness measures the one quantity that decides whether either of
//! them is worth spending: **how much luminance a body puts on screen that
//! the ground would not have put there anyway.** It is a paired render --
//! the same world with and without the body -- because a body's colour
//! alone says nothing (a dark ant on dark soil and a dark ant on sand are
//! different pictures, and only the pair separates them).
//!
//! ```text
//! cargo run --release --example creature_look -- mode=probe out=/tmp/probe.png
//! cargo run --release --example creature_look -- mode=live species=ant count=40 frames=600
//! ```
//!
//! **`ink` is the headline and `px` is not.** `px` counts cells the body
//! covers, which is just its size restated; `ink` is the summed absolute
//! luminance difference over those pixels, so a body the exact colour of
//! the ground it stands on scores near zero however large it is. The
//! `soil-on-soil` row of `mode=probe` is that positive-control-in-reverse:
//! a probe painted in the ground's own material, whose ink must come out at
//! the speckle floor and nothing more.

use std::collections::HashSet;

use pixel_physics::app::{HEIGHT, WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::{MaterialId, MaterialKind};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{creature, parallel, rng};

/// How many rows deep `ground=` re-skins the surface. Deeper than any probe
/// is tall, so no probe can ever see the terrain's own material underneath
/// the one the arm asked for.
const GROUND_SKIN: i32 = 6;

/// Where in the day every frame this harness renders is pinned. 0.5 is
/// midday -- the brightest the world gets, and so the *most* favourable
/// hour for finding a dark body against a lit ground.
const DAYLIGHT: f32 = 1.0;

/// Rec. 601 luma. The channel the eye reads a 1-2 cell object by --
/// `render.rs`'s own note on the gut tint says so, after a blind A/B in
/// which hue lost.
fn luma(px: &[u8]) -> f32 {
    0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32
}

/// Noon, pinned. **The day/night cycle is a designed oscillator and it
/// aliases straight into every luminance number here** -- the first run of
/// this harness landed at night and reported a surround luma of 28 against
/// the 153 the same world gives at midday, which would have made every
/// contrast figure a statement about the hour. `CLAUDE.md`'s rule about
/// dividing a designed cycle out of any number it reaches, applied to a
/// measurement rather than to a threshold.
fn render(world: &World, frame: &mut [u8]) {
    let mut r = Renderer::new();
    r.pinned_light = Some(pixel_physics::sky::frame_for_daylight(DAYLIGHT));
    let particles = ParticleSystem::new();
    r.draw(world, &particles, &HashSet::new(), frame, (WIDTH, HEIGHT), true);
}

/// Topmost **ground** row in a column -- `None` where the column is open
/// water or has none.
///
/// **Written first as "topmost cell that is not air", and that was wrong on
/// a vegetated world**: the topmost non-air cell under a tree is a `Plant`,
/// so the test rejected the column and **9 of 24 probes placed**. Ground is
/// what a body stands on, so ground is what this looks for, and foliage
/// overhead is not an objection to standing under it.
///
/// The liquid check is the other half and it is `open-bugs-handoff.md` R2's
/// failure written as a guard: a column whose ground has water on top of it
/// is a column a creature cannot stand in, and putting one there produces a
/// picture of a lake with animals drowning in it.
fn surface(world: &World, x: i32) -> Option<i32> {
    let y = (0..HEIGHT as i32).find(|&y| matches!(world.materials.kind(world.get(x, y).material), MaterialKind::Solid | MaterialKind::Powder))?;
    (!(0..y).any(|yy| world.materials.kind(world.get(x, yy).material) == MaterialKind::Liquid)).then_some(y)
}

/// What a body of `cells` cells is worth on screen, paired against the same
/// world without it.
struct Ink {
    /// Screen pixels the body changed at all.
    px: usize,
    /// Summed |delta luma| over those pixels -- the headline.
    ink: f32,
    /// Mean luma of the pixels the body covers, in the *with* frame.
    body_luma: f32,
    /// Mean luma those same pixels had in the *without* frame: what the
    /// body is standing in front of.
    ground_luma: f32,
    /// Mean luma of the **surround** in the *without* frame -- everything
    /// within `SURROUND` cells of the body that is not the body.
    ///
    /// This is the number the footprint columns cannot give. A body
    /// standing on the skyline is drawn against sky, which is bright, so
    /// `ground_luma` says a dark body there has enormous contrast -- and it
    /// still cannot be found, because what the eye is actually separating
    /// it from is the *neighbourhood*, which on a vegetated surface is
    /// mostly dark. Measured 2026-08-29: reading `ground_luma` alone said
    /// the shipped ant has the best contrast of any arm in the grid.
    surround_luma: f32,
}

/// How far out the surround ring reaches, in cells. Three is about the
/// distance a 1-2 cell object has to differ over before the eye separates
/// it from its background at all; wider and the ring is measuring the
/// scene rather than the object's setting.
const SURROUND: i32 = 3;

fn measure(with: &[u8], without: &[u8], cells: &[(i32, i32)]) -> Ink {
    let (mut px, mut ink, mut bl, mut gl) = (0usize, 0.0f32, 0.0f32, 0.0f32);
    let body: std::collections::HashSet<(i32, i32)> = cells.iter().copied().collect();
    let (mut sl, mut sn) = (0.0f32, 0usize);
    for &(x, y) in cells {
        for dy in -SURROUND..=SURROUND {
            for dx in -SURROUND..=SURROUND {
                let (sx, sy) = (x + dx, y + dy);
                if body.contains(&(sx, sy)) || sx < 0 || sy < 0 || sx >= WIDTH as i32 || sy >= HEIGHT as i32 {
                    continue;
                }
                let i = ((sy as u32 * WIDTH + sx as u32) * 4) as usize;
                sl += luma(&without[i..i + 4]);
                sn += 1;
            }
        }
    }
    for &(x, y) in cells {
        if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
            continue;
        }
        let i = ((y as u32 * WIDTH + x as u32) * 4) as usize;
        let (a, b) = (luma(&with[i..i + 4]), luma(&without[i..i + 4]));
        bl += a;
        gl += b;
        let d = (a - b).abs();
        if d > 0.5 {
            px += 1;
        }
        ink += d;
    }
    let n = cells.len().max(1) as f32;
    Ink { px, ink, body_luma: bl / n, ground_luma: gl / n, surround_luma: sl / sn.max(1) as f32 }
}

/// The probe shapes, smallest first. Offsets from a bottom-front origin,
/// y up-negative, so every probe sits *on* the surface rather than in it.
/// **How many places in this picture are already at least as contrasty as
/// the body is?** Slides the body's own bounding box over the frame *the
/// body is not in*, scores every position with the same
/// body-against-surround statistic `measure` uses, and counts the ones that
/// beat `threshold`.
///
/// This is the number the contrast column cannot give, and it is the whole
/// explanation of the picture. A shipped ant on a lit skyline has enormous
/// contrast -- and so does every rock edge, every leaf, every grain of a
/// speckled soil, because a pixel-art world's texture *is* 1-2 cell
/// luminance noise. Contrast tells you the body is different from what is
/// behind it. This tells you how many other things in the frame are equally
/// different, which is how many candidates the eye has to reject before it
/// gets to the animal.
///
/// The count is over a frame with the bodies removed, so a body can never
/// be its own decoy.
fn decoys(frame: &[u8], w: i32, h: i32, threshold: f32) -> usize {
    let (fw, fh) = (WIDTH as i32, HEIGHT as i32);
    let lum: Vec<f32> = (0..fw * fh).map(|i| luma(&frame[(i as usize) * 4..(i as usize) * 4 + 4])).collect();
    let at = |x: i32, y: i32| lum[(y * fw + x) as usize];
    let mut hits = 0usize;
    for y in SURROUND..fh - h - SURROUND {
        for x in SURROUND..fw - w - SURROUND {
            let mut inner = 0.0;
            for dy in 0..h {
                for dx in 0..w {
                    inner += at(x + dx, y + dy);
                }
            }
            inner /= (w * h) as f32;
            let (mut outer, mut n) = (0.0, 0);
            for dy in -SURROUND..h + SURROUND {
                for dx in -SURROUND..w + SURROUND {
                    if dx >= 0 && dx < w && dy >= 0 && dy < h {
                        continue;
                    }
                    outer += at(x + dx, y + dy);
                    n += 1;
                }
            }
            if (inner - outer / n as f32).abs() >= threshold {
                hits += 1;
            }
        }
    }
    hits
}

fn shapes() -> Vec<(&'static str, i32, i32, Vec<(i32, i32)>)> {
    let block = |w: i32, h: i32| -> Vec<(i32, i32)> { (0..w).flat_map(move |dx| (0..h).map(move |dy| (-dx, -dy))).collect() };
    vec![
        ("1  (1x1)", 1, 1, block(1, 1)),
        ("2  (2x1) = shipped ant", 2, 1, block(2, 1)),
        ("4  (2x2) = shipped beetle", 2, 2, block(2, 2)),
        ("6  (3x2)", 3, 2, block(3, 2)),
        ("9  (3x3)", 3, 3, block(3, 3)),
        ("16 (4x4)", 4, 4, block(4, 4)),
    ]
}

fn main() {
    let mut mode = String::from("probe");
    let mut species = String::from("ant");
    let mut count = 40i32;
    let mut frames = 600u32;
    let mut out = String::new();
    let mut seed = 1u64;
    let mut ground = String::new();
    let mut preset = String::from("rolling");
    let mut zoom = 1i32;
    let mut crop = (0i32, 0i32, WIDTH as i32, HEIGHT as i32);
    let mut warmup = 2400u32;
    let mut shots = 1u32;
    let mut every = 12u32;
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap_or_else(|| panic!("expected key=value, got {a:?}"));
        match k {
            "mode" => mode = v.into(),
            "species" => species = v.into(),
            "count" => count = v.parse().expect("count"),
            "frames" => frames = v.parse().expect("frames"),
            "out" => out = v.into(),
            "seed" => seed = v.parse().expect("seed"),
            "ground" => ground = v.into(),
            "preset" => preset = v.into(),
            "zoom" => zoom = v.parse().expect("zoom"),
            "crop" => {
                let n: Vec<i32> = v.split(',').map(|p| p.parse().expect("crop=x,y,w,h")).collect();
                assert_eq!(n.len(), 4, "crop=x,y,w,h");
                crop = (n[0], n[1], n[2], n[3]);
            }
            "warmup" => warmup = v.parse().expect("warmup"),
            "shots" => shots = v.parse().expect("shots"),
            "every" => every = v.parse().expect("every"),
            _ => panic!("unknown argument {k:?}"),
        }
    }
    // Echo the parameters, per `CLAUDE.md`'s stale-harness rule: a log that
    // does not name its own settings was written by a binary that never had
    // them.
    println!("creature_look: mode={mode} species={species} count={count} frames={frames} seed={seed} ground={ground:?} preset={preset} warmup={warmup}");

    let mut world = World::new(Rect::new(0, 0, WIDTH as i32 - 1, HEIGHT as i32 - 1));
    world.seed = seed;
    // **A generated world, not the legacy test bed.** `build_terrain` is
    // `Spec::Legacy` -- thin bare platforms over open sky, which is the
    // easiest picture a small dark body will ever be in and so exactly the
    // wrong scene for this question. `CLAUDE.md`: a scene that contradicts
    // the code will look like a bug in the code.
    //
    // `rolling` rather than `wetland`: the colony scene's preset has open
    // water in it by design, and both of its standing bugs
    // (`open-bugs-handoff.md` R and R2) are about creatures being put on
    // it. Nothing here needs water.
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("worldgen presets unavailable: {e}");
    }
    let params = presets.get(&preset).unwrap_or_else(|| panic!("no worldgen preset {preset:?}"));
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });
    // Let what worldgen sowed actually grow. A seedling is not vegetation,
    // and the whole question here is whether a body can be found *in* a
    // vegetated surface -- so a scene measured before the plants are up
    // measures the warmup.
    for _ in 0..warmup {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
    }
    // **A creature's readability is not a property of the creature.** The
    // same body is a different picture on sand and on soil, so the ground
    // is an arm of the experiment rather than a backdrop: `ground=soil`
    // re-skins the top rows of whatever worldgen produced, leaving the
    // terrain's shape -- and so every probe's footing -- identical.
    if !ground.is_empty() {
        let id = world.materials.id_of(&ground).unwrap_or_else(|| panic!("no material named {ground:?}"));
        let shades = world.materials.get(id).palette.len().max(1) as u32;
        for x in 0..WIDTH as i32 {
            let Some(sy) = surface(&world, x) else { continue };
            for y in sy..(sy + GROUND_SKIN).min(HEIGHT as i32) {
                let shade = rng::stream(world.seed, x as u64, y as u64, 77).below(shades) as u8;
                world.set(x, y, Cell::new(id, shade).with_attached(true));
            }
        }
    }

    match mode.as_str() {
        "probe" => probe(world, &out, crop, zoom),
        "live" => live(world, &species, count, frames, &out, crop, zoom, shots, every),
        other => panic!("unknown mode {other:?}"),
    }
}

/// The findability grid: every probe shape at every probe value, painted on
/// the world's own surface and measured against the same world without it.
fn probe(mut world: World, out: &str, crop: (i32, i32, i32, i32), zoom: i32) {
    let mut base = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    render(&world, &mut base);

    // The value axis. Hue is held near-constant on purpose -- `render.rs`'s
    // gut-tint note records a blind A/B in which *hue* lost at this size,
    // and an arm that moved both would not say which one carried it.
    let values: Vec<(&str, MaterialId)> = ["ant", "chitin_mid", "chitin_pale", "soil"]
        .iter()
        .filter_map(|n| world.materials.id_of(n).map(|id| (*n, id)))
        .collect();

    println!("\n{:<26} {:>12} {:>6} {:>8} {:>8} {:>10} {:>8} {:>8} {:>8}", "probe", "material", "cells", "body L", "behind L", "surround L", "|contr|", "ink", "decoys");
    let mut placed = 0usize;
    // **Probes must not crowd each other.** The nudge search below hunts
    // for footing, and without this it happily found the *same* footing
    // three times: three 4x4 probes landed at x=311, 315 and 317, painting
    // over one another. Two costs, and the second is the quiet one -- the
    // overlap corrupts the body reading, and a neighbour inside the
    // `SURROUND` ring corrupts the surround reading of a probe that looks
    // perfectly placed.
    let mut painted: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
    let mut rows: Vec<(String, String, i32, i32, Vec<(i32, i32)>)> = Vec::new();
    // **Lay the grid out over the dry ground the world actually has, not
    // over the x axis.** Spacing the probes evenly across 0..WIDTH put a
    // third of them over this seed's lake, where there is no footing at
    // all: **11 of 24 placed**, and the missing ones were the two smallest
    // shapes -- the arms the whole question is about. The x of a probe is a
    // layout choice and nothing is compared across it; where it stands is
    // not.
    let dry: Vec<i32> = (0..WIDTH as i32).filter(|&x| surface(&world, x).is_some()).collect();
    let nv = values.len();
    let slots = shapes().len() * nv;
    assert!(dry.len() >= slots * 4, "only {} dry columns -- not enough ground to lay {slots} probes on", dry.len());
    for (si, (sname, sw, sh, offs)) in shapes().into_iter().enumerate() {
        for (vi, (vname, vid)) in values.iter().enumerate() {
            // Shape-major, values adjacent: the comparison this grid exists
            // for is *between values at one shape*, and that is only clean
            // if those four stand on the same few metres of ground.
            let slot = si * nv + vi;
            let x0 = dry[(slot * dry.len()) / slots + dry.len() / (slots * 2)];
            let Some((x, base_y)) = (0..30)
                .flat_map(|d| [x0 + d, x0 - d])
                .filter_map(|x| surface(&world, x).map(|sy| (x, sy - 1)))
                .find(|&(x, by)| {
                    offs.iter().all(|&(dx, dy)| {
                        world.is_empty(x + dx, by + dy)
                            && (-SURROUND..=SURROUND).all(|ry| (-SURROUND..=SURROUND).all(|rx| !painted.contains(&(x + dx + rx, by + dy + ry))))
                    })
                })
            else {
                continue;
            };
            let cells: Vec<(i32, i32)> = offs.iter().map(|&(dx, dy)| (x + dx, base_y + dy)).collect();
            let shades = world.materials.get(*vid).palette.len().max(1) as u32;
            for (i, &(cx, cy)) in cells.iter().enumerate() {
                let shade = rng::stream(world.seed, si as u64, i as u64, vi as u64).below(shades) as u8;
                world.set(cx, cy, Cell::new(*vid, shade));
            }
            painted.extend(cells.iter().copied());
            placed += 1;
            println!("  placed {sname:<24} {vname:<12} at ({x},{base_y})");
            rows.push((sname.to_string(), vname.to_string(), sw, sh, cells));
        }
    }
    let mut with = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    render(&world, &mut with);

    for (sname, vname, sw, sh, cells) in &rows {
        let m = measure(&with, &base, cells);
        let n = cells.len();
        let d = decoys(&base, *sw, *sh, (m.body_luma - m.surround_luma).abs());
        println!(
            "{sname:<26} {vname:>12} {n:>6} {:>8.1} {:>8.1} {:>10.1} {:>8.1} {:>8.0} {:>8}",
            m.body_luma,
            m.ground_luma,
            m.surround_luma,
            (m.body_luma - m.surround_luma).abs(),
            m.ink,
            d
        );
    }
    println!("\nprobes placed: {placed} of {slots}   painted cells: {}", painted.len());

    // **The controlled version of the `decoys` column above.** Each probe
    // scores against *its own* achieved contrast, and where a probe stands
    // decides that -- the 2x1 ant landed against sky and scored 103 while
    // the 2x2 ant landed against ground and scored 80, so the two rows
    // differ in placement as well as in size and cannot be read against
    // each other. Here the threshold is held fixed and **only the window
    // size moves**, which is the comparison the whole question turns on.
    //
    // The `0` row is the positive control this needs by `CLAUDE.md`'s rule:
    // at a threshold of zero every window must count, so a row that is not
    // the full window population means the counter never fired.
    println!("\nDecoys at a fixed contrast, over the same frame -- how many places in the picture");
    println!("are at least this different from their own surroundings, at each body size:\n");
    print!("{:<26}", "body size");
    for t in [0.0f32, 40.0, 60.0, 80.0, 100.0] {
        print!("{:>12}", format!("|contr|>={t:.0}"));
    }
    println!();
    for (sname, sw, sh, _) in shapes().into_iter().map(|(n, w, h, o)| (n, w, h, o)) {
        print!("{sname:<26}");
        for t in [0.0f32, 40.0, 60.0, 80.0, 100.0] {
            print!("{:>12}", decoys(&base, sw, sh, t));
        }
        println!();
    }
    if !out.is_empty() {
        save(&with, out, crop, zoom);
    }
}

/// A colony of real creatures, walked for `frames`, then measured the same
/// way. This is the arm that costs something: it is the shipped placement
/// and movement code, so a body that cannot be placed on this terrain, or
/// cannot walk on it, shows up here and not in `mode=probe`.
#[allow(clippy::too_many_arguments)]
fn live(mut world: World, species: &str, count: i32, frames: u32, out: &str, crop: (i32, i32, i32, i32), zoom: i32, shots: u32, every: u32) {
    // Spread the colony over the ground the world actually has, for the
    // same reason `probe` does: spacing it evenly across the x axis put
    // most of it over this seed's lake, and "27 of 40 refused" then reads
    // as a body-plan cost when it is a scene error.
    let dry: Vec<i32> = (0..WIDTH as i32).filter(|&x| surface(&world, x).is_some()).collect();
    assert!(dry.len() >= count as usize * 2, "only {} dry columns for {count} creatures", dry.len());
    let mut sites = Vec::new();
    let mut refused = 0;
    for i in 0..count {
        let x = dry[(i as usize * dry.len()) / count as usize];
        let Some(sy) = surface(&world, x) else {
            refused += 1;
            continue;
        };
        match creature::plant_creature_seed(&mut world, x, sy - 1, species) {
            Some(site) => sites.push(site),
            None => refused += 1,
        }
    }
    for s in sites {
        world.schedule_active_site(s);
    }
    println!("placed {} of {count} ({refused} refused placement)", count - refused);
    // **A frame sequence, not a still, when `shots > 1`.** One of the four
    // creature cards in the queue's whole history came back "need an
    // animation to tell", and motion is the one findability cue a contact
    // sheet structurally cannot carry: a two-cell body that is invisible
    // standing still is not necessarily invisible walking. The last
    // `shots` samples are taken `every` frames apart so the sequence ends
    // at the same instant the still would have.
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let first_shot = frames.saturating_sub((shots - 1) * every);
    let mut shot = 0;
    for f in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        if shots > 1 && f + 1 >= first_shot && (f + 1 - first_shot) % every == 0 && shot < shots && !out.is_empty() {
            render(&world, &mut frame);
            let path = match out.rfind('.') {
                Some(i) => format!("{}-{shot:02}{}", &out[..i], &out[i..]),
                None => format!("{out}-{shot:02}"),
            };
            save(&frame, &path, crop, zoom);
            shot += 1;
        }
    }

    // Where the bodies are now, read off the world rather than off the
    // organism list, so a creature that has lost cells counts as what is
    // actually on screen.
    let Some(mat) = world.materials.id_of(species) else { panic!("no material named {species:?}") };
    let mut cells: Vec<(i32, i32)> = Vec::new();
    let mut ids: std::collections::HashSet<u16> = std::collections::HashSet::new();
    for y in 0..HEIGHT as i32 {
        for x in 0..WIDTH as i32 {
            let c = world.get(x, y);
            if c.material == mat {
                cells.push((x, y));
                ids.insert(c.organism_id());
            }
        }
    }
    // Where the bodies ended up, so a crop can be aimed at them rather than
    // guessed. The densest 120-cell window, which is what a review card
    // wants to be pointed at.
    if !cells.is_empty() {
        let best = (0..WIDTH as i32 - 120)
            .max_by_key(|&x| cells.iter().filter(|&&(cx, _)| cx >= x && cx < x + 120).count())
            .unwrap_or(0);
        let ys: Vec<i32> = cells.iter().filter(|&&(cx, _)| cx >= best && cx < best + 120).map(|&(_, cy)| cy).collect();
        let (lo, hi) = (ys.iter().copied().min().unwrap_or(0), ys.iter().copied().max().unwrap_or(0));
        println!("densest 120-cell window: x={best}  bodies there={}  y range {lo}..{hi}", ys.len());
    }
    let mut with = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    render(&world, &mut with);
    if !out.is_empty() {
        save(&with, out, crop, zoom);
    }
    // The paired frame: the same world with the bodies lifted out. Done by
    // mutating the world after the `with` frame is already captured, rather
    // than on a copy -- `World` is deliberately not `Clone` (it owns the
    // chunk decomposition), and the run is over by this point.
    for &(x, y) in &cells {
        world.set(x, y, Cell::EMPTY);
    }
    let mut without = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    render(&world, &mut without);
    let m = measure(&with, &without, &cells);

    // Counted off the *screen* -- distinct organism ids among the cells that
    // are actually drawn -- rather than off the spawn ledger, because the
    // number the review card needs in its `meta` is how many creatures are
    // in the frame, not how many were ever created.
    let live_n = ids.len();
    println!("\nspecies={species}  live creatures={live_n}  body cells on screen={}", cells.len());
    println!("cells/creature = {:.2}", cells.len() as f32 / live_n.max(1) as f32);
    println!(
        "px changed={}  body L={:.1}  behind L={:.1}  surround L={:.1}  |contrast|={:.1}",
        m.px,
        m.body_luma,
        m.ground_luma,
        m.surround_luma,
        (m.body_luma - m.surround_luma).abs()
    );
    println!("ink={:.0}  ink/creature={:.0}", m.ink, m.ink / live_n.max(1) as f32);
    // **What a wider body costs, from the far side of the call.** A body
    // that cannot be placed shows up in `refused` above; a body that is
    // placed and then cannot *walk* shows up only here, and it is the cost
    // `BodyPlan`'s own doc warns about -- "a wide body handles rough ground
    // badly, often no legal position at all". `moves` alone cannot see it:
    // a creature that never moves and one that moves freely both report a
    // plausible-looking number until the blocked count sits beside it.
    let st = &world.creature_stats;
    let attempts = st.moves + st.moves_blocked;
    println!(
        "moves={} blocked={} ({:.0}% of {attempts} attempts)  tumbles={} falls={}",
        st.moves,
        st.moves_blocked,
        100.0 * st.moves_blocked as f32 / attempts.max(1) as f32,
        st.tumbles,
        st.falls
    );
}

/// Write a crop of the frame, magnified with nearest-neighbour replication.
///
/// **Both are mandatory in practice, not conveniences.** The review skill's
/// own note records a card posted at 190x130 that the owner could see
/// nothing in; the stills he has been able to judge are 700-950 px across.
/// A 512x320 frame of a whole world is the same failure at a different
/// size -- a two-cell body in it is two pixels.
fn save(frame: &[u8], out: &str, crop: (i32, i32, i32, i32), zoom: i32) {
    let (cx, cy, cw, ch) = crop;
    let (w, h) = ((cw * zoom) as u32, (ch * zoom) as u32);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    for y in 0..ch {
        for x in 0..cw {
            let (sx, sy) = (cx + x, cy + y);
            if sx < 0 || sy < 0 || sx >= WIDTH as i32 || sy >= HEIGHT as i32 {
                continue;
            }
            let src = ((sy as u32 * WIDTH + sx as u32) * 4) as usize;
            for zy in 0..zoom {
                for zx in 0..zoom {
                    let d = (((y * zoom + zy) as u32 * w + (x * zoom + zx) as u32) * 4) as usize;
                    buf[d..d + 4].copy_from_slice(&frame[src..src + 4]);
                }
            }
        }
    }
    image::save_buffer(out, &buf, w, h, image::ColorType::Rgba8).expect("write png");
    println!("wrote {out} ({w}x{h}, crop {cx},{cy},{cw},{ch} at zoom {zoom})");
}
