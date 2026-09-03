//! **How much of the difference between two plants is their genome, and how
//! much is where they happened to stand?** — the noise floor under every
//! plant result this project has published.
//!
//! Owner, 2026-09-03: *"within the current engine clones of the same plant
//! end up growing/looking very different from one another which makes it much
//! harder to identify when growth patterns do change."*
//!
//! The scatter itself is not news — `plant-heritability-survey-design-
//! 2026-08-27.md` records **31 to 153 cells and 90 / 438 / 1,435 root cells
//! for identical genomes** — but it is recorded there as a *method note*
//! ("do not quote a stand median"), never as a defect. The consequence
//! nobody had drawn is the one that matters: **if developmental scatter is
//! larger than genetic difference, then selection cannot see the genome
//! either.** A population would then re-roll its variety every generation
//! instead of inheriting it, which explains an invisible architectural lever
//! at least as well as composition does.
//!
//! So the quantity this prints is broad-sense heritability, one descriptor
//! at a time:
//!
//! ```text
//! H2 = 1 - Var(clone arm) / Var(population arm)
//! ```
//!
//! — where the clone arm is genetically identical individuals standing in the
//! same bed, so its whole spread is position plus developmental noise, and
//! the population arm is the shipped stand. **H2 near zero means the genome
//! is invisible in that descriptor**, whatever the genome can express.
//!
//! ```text
//! cargo run --release --example clone_variance -- species=herb founders=16 frames=16000
//! cargo run --release --example clone_variance -- species=herb shift=1     # the one-cell-over arm
//! ```
//!
//! ## Three arms, and the third is the one that names the mechanism
//!
//! - **`pop`** — the shipped stand: every founder its own genome.
//! - **`clone`** — every founder carrying founder 0's genome, written through
//!   `World::set_organism_genotype` (which also sets `inherited`, or
//!   `seed_genotype` redraws the whole thing at germination and the arm
//!   silently becomes the control).
//! - **`spread`** — the positive control for the estimator, and it is
//!   mandatory. Half the founders get every continuous draw at `-1` and half
//!   at `+1`, i.e. the two most distant genomes the engine can express. If
//!   `H2` does not go high here, the descriptors are blind and every number
//!   above them is void — `CLAUDE.md`'s sensitivity half, which this repo has
//!   six recorded occurrences of skipping.
//!
//! And separately, `shift=1`: **one plant, alone, in an identical bed, moved
//! one column at a time.** No neighbours, no competition, no genetic
//! difference — so whatever moves is *position*. It exists because `plant.rs`'s
//! growth RNG is `rng::stream(organism_id, cx, cy, frame)`, so a plant's whole
//! development is a function of **where in the world it is standing**: two
//! clones cannot develop alike, ever, however identical their genomes and
//! their surroundings. Measured on `herb`, twelve positions: **83 to 181 cells
//! and 27 to 63 rows tall**, from one genome.
//!
//! **What this arm does NOT measure is the `organism_id` term, and an earlier
//! version of this comment claimed it did.** Each run here builds a fresh world
//! and plants one thing, so the founder gets the **same id every time** — the
//! id is constant by construction and the whole 0.28 belongs to position.
//! Whether the id adds anything on top is unmeasured; the arm that would settle
//! it needs a way to advance the organism counter without putting another plant
//! in the bed. Since position alone already produces that spread, the id is not
//! the cheap lever it looked like.

mod common;

use pixel_physics::render::Renderer;
use pixel_physics::sim::organism;
use pixel_physics::sim::parallel;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(name: &str) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().expect(name)))
}

fn sarg(name: &str) -> Option<String> {
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(str::to_string))
}

/// One plant, as the numbers a person would use to say two plants look
/// different.
///
/// **Size is carried and is not the headline**, for the reason
/// `plant-heritability-survey-design-2026-08-27.md` §3 gives: every
/// discriminating result in this project's record so far is a magnitude, and
/// the owner's verdict three separate times is *"the biggest differences are
/// still size and color"*. A ranking on size is guaranteed a positive result
/// and answers nothing. The shape columns are the ones to read.
#[derive(Clone, Copy, Default)]
struct Shape {
    cells: f32,
    height: f32,
    width: f32,
    /// height / width — the single number closest to "is this a spire or a
    /// dome".
    slenderness: f32,
    /// leaf cells as a fraction of the whole plant.
    foliage_share: f32,
    /// root cells as a fraction of the whole plant.
    root_share: f32,
    /// mean leaf row as a fraction of the plant's own height, 0 at the collar
    /// and 1 at the apex.
    foliage_centre: f32,
}

const COLUMNS: [&str; 7] = ["cells", "height", "width", "slender", "foliage%", "root%", "folcentre"];

impl Shape {
    fn get(&self, i: usize) -> f32 {
        match i {
            0 => self.cells,
            1 => self.height,
            2 => self.width,
            3 => self.slenderness,
            4 => self.foliage_share,
            5 => self.root_share,
            _ => self.foliage_centre,
        }
    }
}

/// One organism's running census while the grid is scanned: body cells, leaf
/// cells, the bounding box, and the summed leaf row for the foliage centre.
type Tally = (u32, u32, i32, i32, i32, i32, i64);

/// Census one organism off the grid.
///
/// Off the grid rather than off `OrganismState`, the way `plant_probe` and
/// `genome_drift` both do it: the cell **type** lives in the grid cell's
/// `aux`, and `OrganismCell` carries resources and support distance without
/// it.
fn shapes(w: &World, ids: &[(u16, i32, i32)]) -> Vec<Shape> {
    let Some(b) = w.bounds() else { return Vec::new() };
    let mut acc: std::collections::HashMap<u16, Tally> = std::collections::HashMap::new();
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            let id = c.organism_id();
            if id == 0 || !ids.iter().any(|&(i, _, _)| i == id) {
                continue;
            }
            let Some(ct) = organism::cell_type(c.aux()) else { continue };
            // Seeds are excluded from the body: a plant is not taller for
            // carrying seeds, and `herb` carries a great many.
            if ct == organism::CellType::Seed {
                continue;
            }
            let e = acc.entry(id).or_insert((0, 0, i32::MAX, i32::MIN, i32::MAX, i32::MIN, 0));
            e.0 += 1;
            if ct == organism::CellType::Leaf {
                e.1 += 1;
                e.6 += y as i64;
            }
            e.2 = e.2.min(y);
            e.3 = e.3.max(y);
            e.4 = e.4.min(x);
            e.5 = e.5.max(x);
        }
    }
    // Root cells are counted from the organism's own tally rather than from
    // the grid: `MatureBody` is what a root tip becomes, so the grid cannot
    // tell a thickened root from a thickened stem, and the state carries the
    // count the engine itself keeps.
    let mut out = Vec::new();
    for &(id, _, _) in ids {
        let Some(&(cells, leaves, min_y, max_y, min_x, max_x, leaf_y_sum)) = acc.get(&id) else { continue };
        if cells < 20 {
            continue; // not established; see plant_probe's own threshold
        }
        let roots = w.organism(id).map_or(0, |s| s.root_cells) as f32;
        let height = (max_y - min_y + 1) as f32;
        let width = (max_x - min_x + 1) as f32;
        let leaf_centre =
            if leaves > 0 { 1.0 - ((leaf_y_sum as f32 / leaves as f32) - min_y as f32) / height.max(1.0) } else { 0.0 };
        out.push(Shape {
            cells: cells as f32,
            height,
            width,
            slenderness: height / width.max(1.0),
            foliage_share: leaves as f32 / cells as f32,
            root_share: roots / cells as f32,
            foliage_centre: leaf_centre,
        });
    }
    out
}

/// Render one world cropped **tight to the plant in both axes**, for the
/// shift arm — where the question is what one genome looks like ten times
/// over, and a bed of mostly soil between them buries it.
///
/// Every panel is cropped to the same *height* so the strip lines up: a
/// per-panel crop would scale each plant differently and turn a size
/// difference into a framing difference, which is the one thing this card
/// must not do.
fn render_plant_tight(w: &World, pad_x: i32, panel_h: u32) -> (Vec<u8>, u32, u32) {
    let b = w.bounds().expect("the plant scene sets bounds");
    let (width, height) = ((b.max_x - b.min_x + 1) as u32, (b.max_y - b.min_y + 1) as u32);
    let mut buf = vec![0u8; (width * height * 4) as usize];
    let mut renderer = Renderer::new();
    renderer.pinned_light = Some(pixel_physics::sky::frame_for_daylight(1.0));
    let particles = pixel_physics::sim::particle::ParticleSystem::new();
    renderer.draw(w, &particles, &std::collections::HashSet::new(), &mut buf, (width, height), true);
    let (mut x0, mut x1, mut y1) = (i32::MAX, i32::MIN, i32::MIN);
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            if w.get(x, y).organism_id() != 0 {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    if x0 > x1 {
        return (vec![0u8; 0], 0, 0);
    }
    // Anchored at the *bottom* of the plant, not centred: every panel shares
    // a ground line, so a taller plant reads as taller rather than as
    // differently framed.
    let bottom = (y1 + 4).min(b.max_y);
    let top = (bottom - panel_h as i32 + 1).max(b.min_y);
    let (x0, x1) = ((x0 - pad_x).max(b.min_x), (x1 + pad_x).min(b.max_x));
    let (pw, ph) = ((x1 - x0 + 1) as u32, (bottom - top + 1) as u32);
    let mut out = vec![0u8; (pw * ph * 4) as usize];
    for row in 0..ph {
        for col in 0..pw {
            let src = (((top - b.min_y) as u32 + row) * width + (x0 - b.min_x) as u32 + col) * 4;
            let dst = ((row * pw) + col) * 4;
            out[dst as usize..dst as usize + 4].copy_from_slice(&buf[src as usize..src as usize + 4]);
        }
    }
    (out, pw, ph)
}

/// Lay panels out left to right on a common baseline, with a one-pixel rule
/// between them so the eye can tell where one plant ends.
fn tile(panels: &[(Vec<u8>, u32, u32)]) -> (Vec<u8>, u32, u32) {
    let h = panels.iter().map(|p| p.2).max().unwrap_or(0);
    let gap = 3u32;
    let total_w: u32 = panels.iter().map(|p| p.1 + gap).sum::<u32>().saturating_sub(gap);
    let mut out = vec![0u8; (total_w * h * 4) as usize];
    // Fill with the sky colour of the first panel's top-left pixel, so the
    // gaps read as background rather than as black bars.
    if let Some((first, fw, _)) = panels.first() {
        if *fw > 0 {
            for px in out.chunks_exact_mut(4) {
                px.copy_from_slice(&first[0..4]);
            }
        }
    }
    let mut x = 0u32;
    for (buf, pw, ph) in panels {
        if *pw == 0 {
            continue;
        }
        let y_off = h - ph;
        for row in 0..*ph {
            for col in 0..*pw {
                let src = ((row * pw) + col) * 4;
                let dst = (((row + y_off) * total_w) + x + col) * 4;
                out[dst as usize..dst as usize + 4].copy_from_slice(&buf[src as usize..src as usize + 4]);
            }
        }
        x += pw + gap;
    }
    (out, total_w, h)
}

/// Leaf cells, and how elongated their clusters are — the pair that says
/// whether a change moved foliage *shape* or foliage *amount*.
fn leaf_shape_census(w: &World) -> (usize, f32, usize) {
    let Some(b) = w.bounds() else { return (0, 0.0, 0) };
    let mut leaves: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            if c.organism_id() != 0 && organism::cell_type(c.aux()) == Some(organism::CellType::Leaf) {
                leaves.insert((x, y));
            }
        }
    }
    const N8: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];
    let mut seen: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
    let (mut total, mut n) = (0.0f32, 0usize);
    for &start in leaves.iter() {
        if seen.contains(&start) {
            continue;
        }
        let mut stack = vec![start];
        let mut group = Vec::new();
        seen.insert(start);
        while let Some(p) = stack.pop() {
            group.push(p);
            for (dx, dy) in N8 {
                let q = (p.0 + dx, p.1 + dy);
                if leaves.contains(&q) && seen.insert(q) {
                    stack.push(q);
                }
            }
        }
        if group.len() < 3 {
            continue;
        }
        let (x0, x1) = (group.iter().map(|p| p.0).min().unwrap(), group.iter().map(|p| p.0).max().unwrap());
        let (y0, y1) = (group.iter().map(|p| p.1).min().unwrap(), group.iter().map(|p| p.1).max().unwrap());
        total += (x1 - x0 + 1).max(y1 - y0 + 1) as f32 / group.len() as f32;
        n += 1;
    }
    (leaves.len(), if n > 0 { total / n as f32 } else { 0.0 }, n)
}

fn mean(v: &[f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f32>() / v.len() as f32
}

fn variance(v: &[f32]) -> f32 {
    if v.len() < 2 {
        return 0.0;
    }
    let m = mean(v);
    v.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / (v.len() - 1) as f32
}

/// Coefficient of variation — the scale-free spread, so `cells` and
/// `foliage share` can sit in one table without one drowning the other.
fn cv(v: &[f32]) -> f32 {
    let m = mean(v);
    if m.abs() < 1e-6 {
        return 0.0;
    }
    variance(v).sqrt() / m.abs()
}

fn median(v: &[f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    s[s.len() / 2]
}

/// Which arm the founders' genomes come from.
#[derive(Clone, Copy, PartialEq)]
enum Arm {
    /// The shipped stand.
    Pop,
    /// Every founder carrying founder 0's genome.
    Clone,
    /// Half at every draw `-1`, half at every draw `+1` — the estimator's
    /// positive control.
    Spread,
}

fn build(species: &str, founders: usize, worldseed: Option<u64>, width_override: Option<i32>, key: organism::DevelopmentalKey) -> (World, Vec<(u16, i32, i32)>) {
    let d = common::PlantScene::default();
    let scene = common::PlantScene {
        trees: founders,
        width: width_override.unwrap_or(d.width * (founders as i32).max(1) / d.trees as i32),
        species: species.to_string(),
        seed: worldseed,
        ..Default::default()
    };
    let mut w = scene.build();
    // **Set before a single seed germinates, which is what makes this a
    // per-run arm rather than a mid-run change.** `PlantScene::build` places
    // `Seed` cells; germination happens during stepping, and `stamp_origin`
    // folds the key at that moment -- so a key written here reaches every
    // plant in the bed and a key written later would reach only the seeds
    // still waiting.
    w.developmental_key = key;
    // The founders and where each one is standing, in a deterministic order:
    // the scene plants them left to right, so scanning the grid in column
    // order names them the same way every run.
    //
    // **The coordinate is carried because a founder does not have a genome
    // yet, and finding that out cost a whole sweep.** `PlantScene::build`
    // plants through `World::plant_tree_species`, which allocates the
    // organism and writes the cell and **never calls `plant::seed_genotype`**
    // -- only `World::plant_tree` does. So at frame 0 every founder holds
    // `genotype_draws = [0.0; N]`, the species mean, and they are all
    // identical: the first version of this harness cloned founder `ref` and
    // produced **byte-identical output at ref=0, 1 and 5**, which is this
    // repo's standing tell for a knob that was never connected. The arm was
    // not wrong so much as vacuous -- a clone stand of the species mean,
    // reported as a clone of a sampled individual.
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
    (w, ids)
}

fn apply_arm(w: &mut World, ids: &[(u16, i32, i32)], arm: Arm, reference: usize) {
    match arm {
        Arm::Pop => {}
        Arm::Clone => {
            // **Which founder is cloned is an argument, not always the
            // first.** One genome is one sample: a founder that happens to
            // sit near a threshold in the economy makes every clone of it
            // sit there too, so a single reference genome can report a
            // clone stand as *more* variable than a mixed one and read as a
            // finding. `ref=` sweeps it, and it is only a real knob because
            // of the line below.
            let Some(&(src, sx, sy)) = ids.get(reference.min(ids.len().saturating_sub(1))) else { return };
            // **Found a real genome first.** See `build`: a founder holds the
            // species mean until something calls this, so without it every
            // `ref=` clones the same all-zero draw and the argument is inert.
            pixel_physics::sim::plant::seed_genotype(w, src, sx, sy);
            let Some((draws, alleles, params, dev)) = w.organism_genotype(src) else { return };
            // **The developmental seed is written too, and leaving it out
            // would make this arm measure nothing under
            // `DevelopmentalKey::Plant`.** It decides which shape a genome
            // grows into, so a "clone" stand carrying one genome and sixteen
            // different developments is not a clone stand -- and the failure
            // is invisible, because it reads as *the change did nothing*.
            // Exactly how `ref=` was inert (report SS2.1).
            for &(id, _, _) in ids {
                w.set_organism_genotype(id, draws, alleles, params, dev);
            }
        }
        Arm::Spread => {
            let Some((_, _, params, _)) = ids.first().and_then(|&(id, _, _)| w.organism_genotype(id)) else { return };
            // **The discrete loci go to their extremes too, not just the
            // continuous draws.** The point of this arm is *the widest
            // contrast the engine can express*; half of that vocabulary is
            // the six alleles, and a control that held them fixed would be
            // testing only whether the descriptors can see a multiplier.
            let mut low = [0u8; organism::DISCRETE_LOCI];
            let mut high = [0u8; organism::DISCRETE_LOCI];
            for (locus, h) in high.iter_mut().enumerate() {
                *h = organism::LOCUS_ALLELES[locus].saturating_sub(1);
                low[locus] = 0;
            }
            for (i, &(id, _, _)) in ids.iter().enumerate() {
                let (v, a) = if i % 2 == 0 { (-1.0, low) } else { (1.0, high) };
                // **Each founder keeps its own developmental seed here**, and
                // that is the opposite of the clone arm on purpose: this arm
                // is the denominator of the sensitivity control, so it must
                // carry every source of variation the engine has. Handing all
                // of them one seed would shrink the spread this arm exists to
                // maximise and quietly inflate every H2 measured against it.
                let dev = w.organism_genotype(id).map(|g| g.3).unwrap_or(0);
                w.set_organism_genotype(id, [v; organism::GENOTYPE_TRAITS], a, params, dev);
            }
        }
    }
}

fn run(bed: Bed<'_>, arm: Arm, reference: usize) -> Vec<Shape> {
    run_and_maybe_render(bed, arm, reference, None).0
}

/// **The picture, on the same run that produced the numbers.**
///
/// `CLAUDE.md`: *having rendered something, show it — don't describe it*, and
/// the specific reason a card needs this rather than a re-render is that a
/// contact sheet made from a second run of "the same" scene is a different
/// bed. The image and the `H2` above it come out of one `World`.
///
/// Through the shipped `Renderer`, not a hand-rolled palette walk: what
/// reaches the screen is what the lighting makes of the palette, which is
/// exactly the difference `burrow_probe`'s `contrast=1` arm exists to catch.
/// The five things that describe *which bed* a run happens in, bundled
/// because they travel together through every entry point here and because
/// the alternative is an eight-argument function.
#[derive(Clone, Copy)]
struct Bed<'a> {
    species: &'a str,
    founders: usize,
    frames: u64,
    worldseed: Option<u64>,
    key: organism::DevelopmentalKey,
}

fn run_and_maybe_render(bed: Bed<'_>, arm: Arm, reference: usize, png: Option<&str>) -> (Vec<Shape>, ()) {
    let Bed { species, founders, frames, worldseed, key } = bed;
    let (mut w, ids) = build(species, founders, worldseed, None, key);
    apply_arm(&mut w, &ids, arm, reference);
    for _ in 0..frames {
        parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
    }
    if let Some(path) = png {
        let (buf, width, ch) = render_stand(&w);
        image::save_buffer(path, &buf, width, ch, image::ColorType::Rgba8).expect("write png");
        println!("  wrote {path} ({width}x{ch})");
    }
    (shapes(&w, &ids), ())
}

/// Render one world through the shipped `Renderer`, cropped to the band the
/// stand occupies. Returns `(rgba, width, height)`.
fn render_stand(w: &World) -> (Vec<u8>, u32, u32) {
    {
        let b = w.bounds().expect("the plant scene sets bounds");
        let (width, height) = ((b.max_x - b.min_x + 1) as u32, (b.max_y - b.min_y + 1) as u32);
        let mut buf = vec![0u8; (width * height * 4) as usize];
        let mut renderer = Renderer::new();
        // **Pinned to noon.** `sky::frame_for_daylight` exists because the
        // day/night cycle is a designed oscillator and a card rendered at an
        // arbitrary phase is a card about the hour it was taken -- the first
        // sheet from this harness came out at night and the stand was barely
        // legible. `CLAUDE.md`'s "divide the oscillator out" applied to a
        // picture rather than to a number.
        renderer.pinned_light = Some(pixel_physics::sky::frame_for_daylight(1.0));
        let particles = pixel_physics::sim::particle::ParticleSystem::new();
        renderer.draw(w, &particles, &std::collections::HashSet::new(), &mut buf, (width, height), true);
        // Crop to the band the stand occupies, with margin -- the scene is
        // 200 rows of sky over a bed, and a card of mostly sky is a card
        // nobody can judge. `review/SKILL.md`: render wide, declare tight.
        let (mut top, mut bottom) = (i32::MAX, i32::MIN);
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                if w.get(x, y).organism_id() != 0 {
                    top = top.min(y);
                    bottom = bottom.max(y);
                }
            }
        }
        let (top, bottom) = if top <= bottom {
            ((top - 12).max(b.min_y), (bottom + 8).min(b.max_y))
        } else {
            (b.min_y, b.max_y)
        };
        let ch = (bottom - top + 1) as u32;
        let mut crop = vec![0u8; (width * ch * 4) as usize];
        for row in 0..ch {
            let src = ((top - b.min_y) as u32 + row) * width * 4;
            crop[(row * width * 4) as usize..((row + 1) * width * 4) as usize]
                .copy_from_slice(&buf[src as usize..(src + width * 4) as usize]);
        }
        (crop, width, ch)
    }
}

fn table(label: &str, arms: &[Shape]) {
    print!("  {label:<9} n={:<4}", arms.len());
    for i in 0..COLUMNS.len() {
        let col: Vec<f32> = arms.iter().map(|s| s.get(i)).collect();
        print!("  {:>9}", format!("{:.3}", median(&col)));
    }
    println!();
    print!("  {:<9} CV   ", "");
    for i in 0..COLUMNS.len() {
        let col: Vec<f32> = arms.iter().map(|s| s.get(i)).collect();
        print!("  {:>9}", format!("{:.3}", cv(&col)));
    }
    println!();
}

fn main() {
    let species = sarg("species").unwrap_or_else(|| "herb".to_string());
    let founders: usize = arg("founders").unwrap_or(16);
    let frames: u64 = arg("frames").unwrap_or(16_000);
    let worldseed: Option<u64> = arg("worldseed");
    let shift: u32 = arg("shift").unwrap_or(0);
    let seeds: usize = arg("seeds").unwrap_or(1);
    let reference: usize = arg("ref").unwrap_or(0);

    println!(
        "clone_variance: species={species} founders={founders} frames={frames} worldseed={worldseed:?} shift={shift} \
         param_mutation_chance={}",
        pixel_physics::sim::plant::param_mutation_chance_seed()
    );

    // **Which developmental key this run uses** -- `world` (today's, and the
    // default so an unqualified run is the shipped behaviour), or an integer
    // coarseness for `DevelopmentalKey::Plant`. `dev=0` drops the germination
    // coordinate; `dev=1` folds it at full resolution. See
    // `organism::DevelopmentalKey`, which carries which instrument reads
    // which end -- they are not the same question.
    let key = match sarg("dev").unwrap_or_else(|| "world".to_string()).as_str() {
        "world" | "control" => organism::DevelopmentalKey::World,
        n => organism::DevelopmentalKey::Plant {
            coarseness: n.parse().expect("dev= takes `world` or an integer coarseness"),
        },
    };
    println!("  developmental key: {key:?}");
    let bed = Bed { species: &species, founders, frames, worldseed, key };
    if shift > 0 {
        one_cell_over(&species, founders, frames, worldseed, sarg("png"), key);
        return;
    }
    // **`spread=` renders one bed per leaf-spread setting.** Separate from
    // the three-arm mode because the question is different: those arms are
    // about *variance* between plants, this is about the *shape of the
    // foliage* on all of them, and mixing them on one card would ask the
    // owner two questions at once.
    if let Some(vals) = sarg("spread") {
        let stem = sarg("png").unwrap_or_else(|| "/tmp/spread".to_string());
        for v in vals.split(',') {
            let value: f32 = v.parse().expect("spread values are comma-separated floats");
            let d = common::PlantScene::default();
            let scene = common::PlantScene {
                trees: founders,
                width: d.width * (founders as i32).max(1) / d.trees as i32,
                species: species.clone(),
                seed: worldseed,
                ..Default::default()
            };
            let mut w = scene.build();
            let id = w.species.id_of(&species).expect("species is compiled in");
            // Through `set_param` into the live registry -- editing the
            // `.ron` and re-running a prebuilt binary is the `include_str!`
            // trap, and it produces bit-identical "runs".
            assert!(
                w.species.set_param(id, organism::CellType::GrowingTip, organism::ParamId::LeafSpread, 0, value),
                "leaf_spread={value} matched no Grow on {species}"
            );
            for _ in 0..frames {
                parallel::step(&mut w);
                w.step_active_sites();
                w.step_fields();
            }
            // **The two numbers the card needs, from the run that made the
            // picture.** `leaf cells` says the arms place the same amount of
            // foliage -- if it moved, the card is about how much leaf there
            // is and not about its shape, which is a different question and
            // the one this lever must not be answering. `elongation` is the
            // shape itself: the long side of each 8-connected leaf cluster's
            // bounding box divided by its cell count, so a line reads high
            // and a blob low.
            let (leaf_cells, elong, clusters) = leaf_shape_census(&w);
            let (buf, pw, ph) = render_stand(&w);
            let path = format!("{stem}_{}.png", v.replace('.', "p"));
            image::save_buffer(&path, &buf, pw, ph, image::ColorType::Rgba8).expect("write png");
            println!("  leaf cells {leaf_cells}, clusters of 3+ {clusters}, mean elongation {elong:.3}");
            println!("  leaf_spread={value}: wrote {path} ({pw}x{ph})");
        }
        return;
    }
    // **`png=` renders the three arms from the runs that produced the
    // numbers**, so a card and its `meta` cannot come from different beds.
    if let Some(stem) = sarg("png") {
        for (arm, name) in [(Arm::Pop, "pop"), (Arm::Clone, "clone"), (Arm::Spread, "spread")] {
            let path = format!("{stem}_{name}.png");
            let shapes = run_and_maybe_render(bed, arm, reference, Some(&path)).0;
            println!("  {name}: {} established, median cells {:.0}", shapes.len(), median(&shapes.iter().map(|s| s.cells).collect::<Vec<_>>()));
        }
        return;
    }

    // **Variances are pooled *within* seed and then averaged, never pooled
    // across seeds.** A pooled-across-seeds variance carries the
    // between-world difference in both arms, which inflates both and drags
    // every ratio toward 1 -- the same shape as an unremoved oscillator
    // (`CLAUDE.md`). Averaging the within-seed variances is the estimator the
    // question actually asks for: *within one bed, how much of the spread is
    // genome*.
    let mut var_pop = [0.0f32; COLUMNS.len()];
    let mut var_clone = [0.0f32; COLUMNS.len()];
    let mut var_spread = [0.0f32; COLUMNS.len()];
    let mut n_seeds = 0.0f32;
    let (mut all_pop, mut all_clone, mut all_spread): (Vec<Shape>, Vec<Shape>, Vec<Shape>) = (vec![], vec![], vec![]);
    for k in 0..seeds.max(1) {
        let ws = worldseed.map(|w| w + k as u64).or(Some(1 + k as u64));
        let seeded = Bed { worldseed: ws, ..bed };
        let pop = run(seeded, Arm::Pop, reference);
        let clones = run(seeded, Arm::Clone, reference);
        let spread = run(seeded, Arm::Spread, reference);
        println!("\n  --- worldseed {:?}, ref founder {reference} ---", ws);
        print!("  {:<9} {:<5}", "arm", "");
        for c in COLUMNS {
            print!("  {c:>9}");
        }
        println!("   (median, then coefficient of variation)");
        table("pop", &pop);
        table("clone", &clones);
        table("spread", &spread);
        for i in 0..COLUMNS.len() {
            var_pop[i] += variance(&pop.iter().map(|s| s.get(i)).collect::<Vec<_>>());
            var_clone[i] += variance(&clones.iter().map(|s| s.get(i)).collect::<Vec<_>>());
            var_spread[i] += variance(&spread.iter().map(|s| s.get(i)).collect::<Vec<_>>());
        }
        n_seeds += 1.0;
        all_pop.extend(pop);
        all_clone.extend(clones);
        all_spread.extend(spread);
    }
    for i in 0..COLUMNS.len() {
        var_pop[i] /= n_seeds;
        var_clone[i] /= n_seeds;
        var_spread[i] /= n_seeds;
    }

    println!("\n== pooled over {n_seeds} world seed(s), founders={founders} ==");
    println!(
        "  established plants censused: pop {} / clone {} / spread {}",
        all_pop.len(),
        all_clone.len(),
        all_spread.len()
    );
    // **H2 clamped at zero rather than reported negative.** A clone arm whose
    // spread exceeds the population's is sampling noise at this n, not
    // negative heritability, and a negative number in this column invites a
    // reading it cannot support. The raw variance ratio is printed beside it
    // so the clamping is visible rather than silent.
    println!("\n  broad-sense heritability, H2 = 1 - Var(clone)/Var(pop):");
    print!("  {:<15}", "");
    for c in COLUMNS {
        print!("  {c:>9}");
    }
    println!();
    print!("  {:<15}", "Var(clone)/Var(pop)");
    for i in 0..COLUMNS.len() {
        print!("  {:>9}", format!("{:.3}", if var_pop[i] > 0.0 { var_clone[i] / var_pop[i] } else { f32::NAN }));
    }
    println!();
    print!("  {:<15}", "H2");
    for i in 0..COLUMNS.len() {
        let h = if var_pop[i] > 0.0 { (1.0 - var_clone[i] / var_pop[i]).max(0.0) } else { 0.0 };
        print!("  {:>9}", format!("{:.3}", h));
    }
    println!();
    let mut control = [0.0f32; COLUMNS.len()];
    print!("  {:<15}", "H2 (control)");
    for i in 0..COLUMNS.len() {
        control[i] = if var_spread[i] > 0.0 { (1.0 - var_clone[i] / var_spread[i]).max(0.0) } else { 0.0 };
        print!("  {:>9}", format!("{:.3}", control[i]));
    }
    println!();

    // **The estimator's own sensitivity, printed and asserted.** `spread`
    // stands the two most distant genomes the engine can express in one bed;
    // if no descriptor separates them from a clone stand, the descriptor set
    // cannot see a genome at all and every H2 above it is a statement about
    // this harness rather than about the engine.
    let best = control.iter().cloned().fold(0.0f32, f32::max);
    let best_i = control.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0);
    println!("\n  positive control: the widest contrast the engine can express reaches H2 = {best:.3} on `{}`", COLUMNS[best_i]);
    assert!(
        all_clone.len() >= 3 && all_pop.len() >= 3,
        "fewer than three established plants in an arm -- a scene error, not a result"
    );
    assert!(
        best > 0.2,
        "positive control failed: the widest genetic contrast the engine can express does not separate from a clone \
         stand on ANY descriptor. The descriptor set is blind and every H2 above is void."
    );
}

/// **One plant, alone, in an identical bed, moved one column at a time.**
///
/// The arm that names the mechanism. `plant.rs`'s growth draws come from
/// `rng::stream(organism_id, cx, cy, frame)`, so a plant's development is a pure
/// function of *where it is* and *which slot it got* — there is no
/// per-organism seed that survives being moved. Two consequences a player
/// would not guess: a plant one column over is a different plant, and a plant
/// that germinates into organism slot 7 rather than 6 is a different plant
/// again.
fn one_cell_over(species: &str, n: usize, frames: u64, worldseed: Option<u64>, png: Option<String>, key: organism::DevelopmentalKey) {
    println!("\n  one founder, alone, same genome, moved one column at a time ({n} positions):");
    print!("  {:<15}", "");
    for c in COLUMNS {
        print!("  {c:>9}");
    }
    println!();
    // A single reference genome, taken from the first bed and written onto
    // every subsequent one, so the only thing that differs between runs is
    // the column the plant stands in.
    let (w0, ids0) = build(species, 1, worldseed, None, key);
    let Some(&(src, sx, sy)) = ids0.first() else {
        println!("  REFUSING: the reference bed planted nothing.");
        return;
    };
    let mut w0 = w0;
    pixel_physics::sim::plant::seed_genotype(&mut w0, src, sx, sy);
    let Some(reference) = w0.organism_genotype(src) else {
        println!("  REFUSING: the reference founder has no genome.");
        return;
    };
    let mut all: Vec<Shape> = Vec::new();
    let mut panels: Vec<(Vec<u8>, u32, u32)> = Vec::new();
    for step in 0..n {
        // `PlantScene` centres a single founder, so the column is moved by
        // widening the bed by one -- which keeps the founder's surroundings
        // identical in *kind* while moving its coordinate.
        let d = common::PlantScene::default();
        let (mut w, ids) = build(species, 1, worldseed, Some(d.width + 2 * step as i32), key);
        for &(id, _, _) in &ids {
            // `reference.3` is the developmental seed, and it is the whole
            // point of this arm under `DevelopmentalKey::Plant`: one genome
            // AND one development, moved one column at a time. Without it
            // every position would still be a different plant and the CV
            // below could not fall however well the key change worked.
            w.set_organism_genotype(id, reference.0, reference.1, reference.2, reference.3);
        }
        for _ in 0..frames {
            parallel::step(&mut w);
            w.step_active_sites();
            w.step_fields();
        }
        let s = shapes(&w, &ids);
        if png.is_some() {
            panels.push(render_plant_tight(&w, 3, 96));
        }
        if let Some(s0) = s.first() {
            all.push(*s0);
            print!("  {:<15}", format!("+{step} col"));
            for i in 0..COLUMNS.len() {
                print!("  {:>9}", format!("{:.3}", s0.get(i)));
            }
            println!();
        } else {
            println!("  {:<15}  (did not establish)", format!("+{step} col"));
        }
    }
    print!("  {:<15}", "CV");
    for i in 0..COLUMNS.len() {
        let col: Vec<f32> = all.iter().map(|s| s.get(i)).collect();
        print!("  {:>9}", format!("{:.3}", cv(&col)));
    }
    println!();
    println!(
        "  -- every row above is the SAME genome. Whatever spread this shows is the floor under \
         any claim that two genomes differ."
    );
    if let Some(path) = png {
        let (buf, w, h) = tile(&panels);
        if w > 0 {
            image::save_buffer(&path, &buf, w, h, image::ColorType::Rgba8).expect("write png");
            println!("  wrote {path} ({w}x{h}) -- {} panels, one genome", panels.len());
        }
    }
}
