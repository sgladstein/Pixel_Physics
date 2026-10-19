//! **What else can a herb look like?** Random variants of `herb.ron`, each
//! grown alone, drawn side by side through the shipped `Renderer`.
//!
//! ```text
//! cargo run --release --example herb_morphs -- n=12 seeds=2 frames=20000 out=herbs.png
//! ```
//!
//! Built 2026-10-19 for the plant-trainer engine review
//! (`Reports/plant-engine-trainability-review-2026-10-19.md`), to answer
//! *how flexible the current design is* by looking rather than by reading:
//! a trainer can only find what the species file can reach, and this is a
//! cheap look at the reach before a trainer is built.
//!
//! **Column 0 is the shipped herb**, the control. Every other column is one
//! variant: a handful of the shoot's numbers moved together, each by a log
//! factor drawn from `sigma`, through `ParamGenome::apply` — the same door a
//! heritable parameter mutation goes through, so a variant is something the
//! engine's own mutation could in principle reach, only further and faster.
//! `after_metamers` (when the flowering apex ends the axis) is varied in the
//! fate table beside them.
//!
//! **Each row is a different world seed with the same genome.** A plant's
//! growth draws are keyed on the world (`DevelopmentalKey::World`, the
//! shipped default), so two copies of one genome grow into different plants;
//! `dev=0` grows them under `DevelopmentalKey::Plant { coarseness: 0 }`
//! instead, where siblings develop alike. **Read a column, not a tile**: a
//! look that changes between rows is chance, one that holds down a column is
//! the genome.
//!
//! Every variant goes through `SpeciesRegistry::register_ron` as text, so it
//! takes exactly the parse and validation path a shipped species does.

mod common;

use pixel_physics::render::Renderer;
use pixel_physics::sim::organism::{self, CellType, DevelopmentalKey, ParamGenome, ParamId, SpeciesDef};
use pixel_physics::sim::parallel;
use pixel_physics::sim::plant;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::species_export;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(name: &str) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().expect(name)))
}

fn arg_str(name: &str) -> Option<String> {
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(str::to_string))
}

/// splitmix64: the variant draws are the harness's own, so they do not move
/// any stream the engine reads.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// A symmetric draw on `[-1, 1]`.
    fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

/// How a parameter is moved: by a log factor on its authored value, or —
/// where the authored value is zero and a factor could never leave it — by
/// a draw from a range.
#[derive(Clone, Copy)]
enum Move {
    Log,
    Range(f32, f32),
}

/// The shoot's look levers, with where they live.
const LEVERS: &[(CellType, ParamId, Move)] = &[
    (CellType::GrowingTip, ParamId::BranchChance, Move::Range(0.0, 0.3)),
    (CellType::GrowingTip, ParamId::MaxActiveTips, Move::Range(2.0, 14.0)),
    (CellType::GrowingTip, ParamId::Plastochron, Move::Log),
    (CellType::GrowingTip, ParamId::Internode, Move::Log),
    (CellType::GrowingTip, ParamId::BranchAngle, Move::Log),
    (CellType::GrowingTip, ParamId::LeafCluster, Move::Log),
    (CellType::GrowingTip, ParamId::OrganCluster, Move::Log),
    (CellType::GrowingTip, ParamId::LeafSpread, Move::Range(0.0, 1.0)),
    (CellType::GrowingTip, ParamId::HeadingInertia, Move::Range(0.3, 0.98)),
    (CellType::GrowingTip, ParamId::UpwardWeight, Move::Log),
    (CellType::GrowingTip, ParamId::LightWeight, Move::Log),
    (CellType::GrowingTip, ParamId::TurgorSource, Move::Log),
    (CellType::DormantBud, ParamId::BudBreakAcrotony, Move::Log),
    (CellType::MatureBody, ParamId::PipeRatio, Move::Log),
];

/// One variant of `base`: `k` levers moved, plus `after_metamers`. Returns
/// the species text and a one-line description of what moved.
fn variant(world: &World, base_text: &str, name: &str, k: usize, sigma: f32, draw: &mut Draw) -> (String, String) {
    let mut def: SpeciesDef = ron::from_str(base_text).expect("herb.ron parses");
    def.name = name.to_string();
    let herb = world.species.id_of("herb").expect("herb is compiled in");
    let scale_of = |p: ParamId| world.species.param_scale(p);
    let mut picked: Vec<usize> = Vec::new();
    while picked.len() < k.min(LEVERS.len()) {
        let i = (draw.next() % LEVERS.len() as u64) as usize;
        if !picked.contains(&i) {
            picked.push(i);
        }
    }
    picked.sort_unstable();
    let mut said = Vec::new();
    for &i in &picked {
        let (cell_type, param, how) = LEVERS[i];
        let factor = (draw.signed() * sigma).exp();
        let from_range = match how {
            Move::Range(lo, hi) => Some(lo + (hi - lo) * draw.unit()),
            Move::Log => None,
        };
        let mut genome = ParamGenome::default();
        let mut shown = None;
        for tier in 0..organism::BRANCH_ORDERS as u8 {
            let authored = world
                .species
                .param_in_force(herb, &ParamGenome::default(), (cell_type, param, tier))
                .unwrap_or(0.0);
            let raw = from_range.unwrap_or(authored * factor);
            let value = organism::clamp_param(param, raw, scale_of(param));
            shown.get_or_insert((authored, value));
            genome.set(cell_type, param, tier, value);
        }
        for (ct, behaviors) in def.cell_types.iter_mut() {
            if *ct == cell_type {
                for b in behaviors.iter_mut() {
                    genome.apply(cell_type, b);
                }
            }
        }
        if let Some((from, to)) = shown {
            said.push(format!("{}:{from:.3}->{to:.3}", param.name()));
        }
    }
    // When the flowering apex ends the axis -- a fate-table number, not a
    // `ParamId`, so it is moved here directly.
    for (ct, rules) in def.fates.iter_mut() {
        if *ct == CellType::GrowingTip {
            for f in rules.iter_mut() {
                if let Some(m) = f.after_metamers.as_mut() {
                    let to = (3 + draw.next() % 18) as u8;
                    said.push(format!("after_metamers:{m}->{to}"));
                    *m = to;
                }
            }
        }
    }
    (species_export::to_ron(&def).expect("a variant serialises"), said.join(" "))
}

/// What one grown bed holds, counted from the cells on screen by material.
#[derive(Default)]
struct Census {
    organisms: usize,
    above: usize,
    wood: usize,
    leaf: usize,
    organ: usize,
    root: usize,
    height: i32,
    width: i32,
    /// The founder's age in organism ticks, or `None` once it is dead. A
    /// founder younger than the run means its id went to a seedling after
    /// it died (`World::free_organism` reuses slots).
    founder_age: Option<u32>,
    /// Plant tissue above the ground that no organism owns.
    orphans: usize,
    /// The three commonest reasons the species' shoot tips stopped.
    why: String,
}

fn census(w: &World, width: i32, ground_y: i32, keep: &dyn Fn(u32) -> bool) -> Census {
    let mut c = Census::default();
    let mut ids = std::collections::HashSet::new();
    let (mut top, mut left, mut right) = (ground_y, width, -1);
    for y in 0..w.bounds().map_or(0, |b| b.max_y + 1) {
        for x in 0..width {
            let cell = w.get(x, y);
            let id = cell.organism_id();
            if id == 0 && y < ground_y {
                let name = w.materials.get(cell.material).name.as_str();
                if matches!(name, "leaf" | "wood" | "flower" | "fruit" | "grassblade") {
                    c.orphans += 1;
                }
            }
            // A plant bed: every organism here is a plant.
            if !keep(id) || w.organism(id).is_none() {
                continue;
            }
            ids.insert(id);
            let name = w.materials.get(cell.material).name.as_str();
            if y < ground_y {
                c.above += 1;
                top = top.min(y);
                left = left.min(x);
                right = right.max(x);
                match name {
                    "leaf" | "grassblade" => c.leaf += 1,
                    "flower" | "fruit" => c.organ += 1,
                    _ => c.wood += 1,
                }
            } else {
                c.root += 1;
            }
        }
    }
    c.organisms = ids.len();
    c.height = ground_y - top;
    c.width = if right >= left { right - left + 1 } else { 0 };
    c
}

fn main() {
    let n: usize = arg("n").unwrap_or(12);
    let seeds: u64 = arg("seeds").unwrap_or(2);
    let frames: u64 = arg("frames").unwrap_or(20_000);
    let sigma: f32 = arg("sigma").unwrap_or(0.9);
    let k: usize = arg("k").unwrap_or(5);
    let seed0: u64 = arg("seed0").unwrap_or(1);
    let width: i32 = arg("width").unwrap_or(96);
    let up: i32 = arg("up").unwrap_or(110);
    let scale: usize = arg("scale").unwrap_or(3);
    let dev: Option<u32> = arg("dev");
    // `solo=1` draws and counts the founder alone: its seedlings are left
    // out of the tile and the census, so a column shows one plant's form
    // rather than the stand it seeded.
    let solo = arg_str("solo").as_deref() == Some("1");
    let out = arg_str("out").unwrap_or_else(|| "herb_morphs.png".to_string());
    println!(
        "herb_morphs: n={n} seeds={seeds} frames={frames} sigma={sigma} k={k} seed0={seed0} width={width} up={up} \
         dev={} solo={solo} out={out}",
        dev.map_or("world".to_string(), |c| format!("plant:{c}"))
    );

    let base_text = include_str!("../assets/species/herb.ron");
    let probe = common::PlantScene { width, trees: 1, species: "herb".into(), ..Default::default() }.build();
    let mut draw = Draw(seed0);
    let mut variants: Vec<(String, String, String)> = vec![("herb".into(), base_text.to_string(), "shipped herb".into())];
    for i in 1..n {
        let name = format!("herbmorph{i}");
        let (text, said) = variant(&probe, base_text, &name, k, sigma, &mut draw);
        variants.push((name, text, said));
    }

    let ground_y = common::PlantScene::default().ground_y;
    let (tile_w, tile_h) = (width as usize, (up + 14) as usize);
    let jobs: Vec<(usize, u64)> = (0..variants.len()).flat_map(|v| (1..=seeds).map(move |s| (v, s))).collect();
    let results: Vec<(usize, u64, Census, Vec<u8>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .iter()
            .map(|&(v, s)| {
                let (name, text, _) = &variants[v];
                scope.spawn(move || {
                    let scene = common::PlantScene {
                        width,
                        trees: 1,
                        species: name.clone(),
                        species_ron: (v > 0).then(|| text.clone()),
                        seed: Some(s),
                        ..Default::default()
                    };
                    let mut w = scene.build();
                    // The founder is the one organism the bed holds at
                    // planting.
                    let founder = (0..w.bounds().map_or(0, |b| b.max_y + 1))
                        .flat_map(|y| (0..width).map(move |x| (x, y)))
                        .map(|(x, y)| w.get(x, y).organism_id())
                        .find(|&id| id != 0)
                        .expect("the bed planted its founder");
                    let keep = |id: u32| id != 0 && (!solo || id == founder);
                    if let Some(c) = dev {
                        w.developmental_key = DevelopmentalKey::Plant { coarseness: c };
                    }
                    for _ in 0..frames {
                        parallel::step(&mut w);
                        w.step_active_sites();
                        w.step_fields();
                    }
                    let mut c = census(&w, width, ground_y, &keep);
                    c.founder_age = w.organism(founder).map(|st| st.age_ticks);
                    // Why the species' shoot tips stopped, commonest first
                    // (`plant::GrowWhy`, the reasons the game's own plant
                    // page shows). Founder and seedlings together.
                    if let Some(g) = w.species.id_of(name).and_then(|id| w.grow_census.get(id.0 as usize)) {
                        let mut why: Vec<(u64, &str)> =
                            g.retired[0].iter().zip(plant::GROW_WHY_NAMES).map(|(&n, s)| (n, s)).filter(|p| p.0 > 0).collect();
                        why.sort_unstable_by(|a, b| b.cmp(a));
                        c.why = why.iter().take(3).map(|(n, s)| format!("{s}:{n}")).collect::<Vec<_>>().join(",");
                    }
                    if solo {
                        // **Drawn from a fresh copy of the bed holding the
                        // founder alone.** Clearing the seedlings' cells in
                        // the grown world by `set` is not enough: it leaves
                        // their organism records behind, and measured, 472
                        // of 6,615 empty cells above the ground still drew
                        // as plant afterwards (0 in the fresh copy). A bed
                        // that never held the seedlings cannot draw them.
                        // Below the ground the grown bed is copied whole --
                        // soil and roots as they are.
                        let mut fresh = common::PlantScene { width, trees: 0, ..Default::default() }.build();
                        for y in 0..(ground_y + 20) {
                            for x in 0..width {
                                let cell = w.get(x, y);
                                if y >= ground_y || cell.organism_id() == founder {
                                    fresh.set(x, y, cell);
                                }
                            }
                        }
                        w = fresh;
                    }
                    let (fw, fh) = (width as u32, (ground_y + 20) as u32);
                    let mut frame = vec![0u8; (fw * fh * 4) as usize];
                    let mut renderer = Renderer::new();
                    // Every tile at the same hour of full day, so a sheet
                    // compares plants and not the time of day each run
                    // happened to stop at. Render-only: the simulation's
                    // own clock is untouched.
                    renderer.pinned_light = Some(pixel_physics::sky::frame_for_daylight(1.0));
                    let touched = w.take_touched_chunks();
                    renderer.draw(&w, &ParticleSystem::new(), &touched, &mut frame, (fw, fh), true);
                    let mut tile = vec![0u8; tile_w * tile_h * 4];
                    for ty in 0..tile_h {
                        let fy = (ground_y - up) as usize + ty;
                        let src = (fy * fw as usize) * 4;
                        tile[ty * tile_w * 4..(ty + 1) * tile_w * 4].copy_from_slice(&frame[src..src + tile_w * 4]);
                    }
                    (v, s, c, tile)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("a bed panicked")).collect()
    });

    println!(
        "{:>3} {:>4} {:>5} {:>6} {:>5} {:>5} {:>5} {:>6} {:>5} {:>5} {:>5} {:>5}  {:<44} moved",
        "v", "seed", "orgs", "above", "wood%", "leaf%", "organ", "root", "high", "wide", "age", "orph", "tips stopped"
    );
    for (v, s, c, _) in &results {
        let pct = |x: usize| if c.above > 0 { 100.0 * x as f32 / c.above as f32 } else { 0.0 };
        println!(
            "{:>3} {:>4} {:>5} {:>6} {:>5.1} {:>5.1} {:>5} {:>6} {:>5} {:>5} {:>5} {:>5}  {:<44} {}",
            v,
            s,
            c.organisms,
            c.above,
            pct(c.wood),
            pct(c.leaf),
            c.organ,
            c.root,
            c.height,
            c.width,
            c.founder_age.map_or("dead".to_string(), |a| a.to_string()),
            c.orphans,
            c.why,
            if *s == 1 { variants[*v].2.as_str() } else { "" }
        );
    }

    // The sheet: one column per variant, one row per world seed, a dark gap
    // between tiles, every cell drawn `scale` pixels square.
    let gap = 2usize;
    let cols = variants.len();
    let rows = seeds as usize;
    let sw = cols * (tile_w * scale + gap) + gap;
    let sh = rows * (tile_h * scale + gap) + gap;
    let mut sheet = vec![0u8; sw * sh * 4];
    for px in sheet.chunks_exact_mut(4) {
        px.copy_from_slice(&[24, 24, 28, 255]);
    }
    for (v, s, _, tile) in &results {
        let ox = gap + v * (tile_w * scale + gap);
        let oy = gap + (*s as usize - 1) * (tile_h * scale + gap);
        for ty in 0..tile_h {
            for tx in 0..tile_w {
                let p = &tile[(ty * tile_w + tx) * 4..(ty * tile_w + tx) * 4 + 4];
                for dy in 0..scale {
                    for dx in 0..scale {
                        let i = ((oy + ty * scale + dy) * sw + ox + tx * scale + dx) * 4;
                        sheet[i..i + 3].copy_from_slice(&p[..3]);
                        sheet[i + 3] = 255;
                    }
                }
            }
        }
    }
    image::save_buffer(&out, &sheet, sw as u32, sh as u32, image::ColorType::Rgba8).expect("writing the sheet");
    println!("herb_morphs: wrote {out} ({sw}x{sh}): columns are variants (0 = shipped herb), rows are world seeds");
}
