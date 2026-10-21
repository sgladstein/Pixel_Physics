//! **What decides whether a dug roof stands** — the pictures behind
//! `Reports/soil-physics-realism-2026-10-21.md` §2.
//!
//! `burrow_probe` asks *does a tunnel stand*; this asks *what is the rule
//! that decides it*, by varying the one quantity a real roof's stability
//! depends on most -- its **span** -- and the two the engine's rule actually
//! reads: the **thickness** of worked ground over the hole, and whether the
//! worked cell is **spoil** (which needs footing) or **packed soil** (which
//! does not). Moisture rides along as a fourth arm because real cohesion is
//! moisture-dependent and the engine's is not.
//!
//! | arm | what is cut | what the real thing would do |
//! |---|---|---|
//! | `dry` | lined chambers 3 tall, spans 4 / 16 / 48 / 128, bank at aux 250 | wide spans fail first |
//! | `wet` | the same, bank at `SOIL_SATURATED` | saturated walls slump |
//! | `loose` | the same chambers, **no lining** | the roof pours in |
//! | `bridge` | a lined slab between two voids, thickness 1 / 2 at span 8 and 48 | thin and wide fails first |
//! | `shelf` | a cantilevered shelf 2 thick, 14 long: packed soil, then spoil | both are weak; spoil weaker |
//!
//! Every row prints **caved** -- carved cells with ground standing in them
//! -- and, for the bridge and shelf, **held**: cells of the structure still
//! where they were built. A picture of an intact roof and a roof the census
//! never reached look the same (`CLAUDE.md`: "did it fire at all" needs a
//! counter), and the `loose` arm is the positive control: it must read
//! caved > 0 or the instrument is measuring its own scene.
//!
//! ```text
//! cargo run --release --example soil_cases -- png=soil_cases.png
//! cargo run --release --example soil_cases -- frames=3600
//! ```

mod common;

use std::collections::HashSet;

use common::PlantScene;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material::{self, MaterialId, MaterialKind};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::weather::Pin;
use pixel_physics::sim::{frame, player, Cell, World};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses"))
    })
}

const W: i32 = 300;
const H: i32 = 110;
const GROUND: i32 = 10;
const SOIL: i32 = 80;
/// Crop of the render that every arm shares, so rows of the sheet line up.
const CROP: (i32, i32, i32, i32) = (0, 16, W, 30);

/// The cells of a built structure and the material each was built with.
type Built = Vec<(i32, i32, MaterialId)>;

/// A named set of cells, censused for what is standing in it.
struct Region {
    name: String,
    cells: Vec<(i32, i32)>,
}

impl Region {
    /// Carved cells that now hold **ground** -- a powder or a solid. Raw
    /// material, not `Cell::is_empty`, for `burrow_probe`'s reason.
    fn caved(&self, w: &World) -> usize {
        self.cells
            .iter()
            .filter(|(x, y)| {
                let m = w.get(*x, *y).material;
                m != material::EMPTY
                    && matches!(w.materials.get(m).kind, MaterialKind::Powder | MaterialKind::Solid)
            })
            .count()
    }

    /// Built cells still holding the material they were built with.
    fn held(&self, w: &World, built: &[(i32, i32, MaterialId)]) -> usize {
        built.iter().filter(|(x, y, m)| w.get(*x, *y).material == *m).count()
    }
}

fn rect(x0: i32, y0: i32, w: i32, h: i32) -> Vec<(i32, i32)> {
    (y0..y0 + h).flat_map(|y| (x0..x0 + w).map(move |x| (x, y))).collect()
}

/// `creature::line_burrow`'s lining in one pass: the 8-ring around every
/// carved cell, worked through `packs_into` so this measures the shipped
/// rule and not a material name.
fn line(w: &mut World, carved: &HashSet<(i32, i32)>) -> usize {
    let mut ring: Vec<(i32, i32)> = Vec::new();
    for &(cx, cy) in carved {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let n = (cx + dx, cy + dy);
                if !carved.contains(&n) {
                    ring.push(n);
                }
            }
        }
    }
    ring.sort_unstable();
    ring.dedup();
    let mut n = 0;
    for (x, y) in ring {
        let c = w.get(x, y);
        if let Some(p) = w.materials.get(c.material).packs_into {
            let mut lined = c;
            lined.material = p;
            w.set(x, y, lined);
            n += 1;
        }
    }
    n
}

/// **A flat material map, not the game's render.** The lit render is
/// dark at night and soil-on-soil has no contrast, so a lining or a fallen
/// slab is invisible in it; this is a full replace on fixed colours, which
/// is what `CLAUDE.md` asks of a debug readout.
fn shoot(w: &World) -> Vec<u8> {
    let (cx, cy, cw, ch) = CROP;
    let mut out = Vec::with_capacity((cw * ch * 4) as usize);
    for y in cy..cy + ch {
        for x in cx..cx + cw {
            let c = w.get(x, y);
            let name = w.materials.get(c.material).name.as_str();
            let rgb: [u8; 3] = match name {
                _ if c.material == material::EMPTY => [12, 12, 16],
                "soil" => [110, 78, 48],
                "packedsoil" => [214, 140, 60],
                "spoil" => [236, 214, 92],
                "water" => [60, 120, 230],
                "stone" => [120, 120, 120],
                _ => [200, 60, 200],
            };
            out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    out
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(1_200);
    let png: Option<String> = arg("png");
    let zoom: u32 = arg("zoom").unwrap_or(3);
    println!("soil_cases: frames={frames} world={W}x{H} ground={GROUND} soil={SOIL}");

    let mut sheet_rows: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut lines: Vec<String> = Vec::new();

    for arm in ["dry", "wet", "loose", "bridge", "shelf"] {
        let scene = PlantScene {
            width: W,
            height: H,
            ground_y: GROUND,
            soil_depth: SOIL,
            trees: 0,
            species: "herb".to_string(),
            seed: Some(1),
            ..PlantScene::default()
        };
        let mut w = scene.build();
        w.set_weather_pin(Pin::Clear);

        let bank_aux = match arm {
            "wet" => material::SOIL_SATURATED,
            "dry" => 250,
            _ => material::SOIL_FIELD_CAPACITY,
        };
        for x in 0..W {
            for y in GROUND..GROUND + SOIL {
                let c = w.get(x, y);
                if w.materials.get(c.material).water_capacity > 0 {
                    w.set(x, y, c.with_aux(bank_aux));
                }
            }
        }

        let mut voids: Vec<Region> = Vec::new();
        let mut structures: Vec<(Region, Built)> = Vec::new();
        let roof_y = 30;
        match arm {
            "dry" | "wet" | "loose" => {
                let mut x = 8;
                for span in [4, 16, 48, 128] {
                    voids.push(Region { name: format!("span {span}"), cells: rect(x, roof_y, span, 3) });
                    x += span + 24;
                }
            }
            "bridge" => {
                // Two voids, one above the other, the slab between them
                // `t` thick. Both are lined, so the slab is worked ground
                // through and through -- the case a tunnel driven under
                // another one leaves.
                let mut x = 10;
                for (span, t) in [(8, 1), (8, 2), (48, 1), (48, 2)] {
                    let top = rect(x, roof_y - 3 - t, span, 3);
                    let bottom = rect(x, roof_y, span, 3);
                    let slab: Vec<(i32, i32)> = rect(x, roof_y - t, span, t);
                    voids.push(Region { name: format!("span {span} t{t} below"), cells: bottom });
                    voids.push(Region { name: format!("span {span} t{t} above"), cells: top });
                    structures.push((Region { name: format!("slab span {span} thick {t}"), cells: slab }, Vec::new()));
                    x += span + 26;
                }
            }
            "shelf" => {
                // A shelf sticking out of a wall over an open pit, the shape
                // an ant leaves when it undercuts a ledge. Built of packed
                // soil on the left and spoil on the right; the pit is not
                // lined so nothing else on the shelf is worked ground.
                let packed = w.materials.id_of("packedsoil").expect("packedsoil is compiled in");
                let spoil = w.materials.id_of("spoil").expect("spoil is compiled in");
                for (i, (name, m)) in [("packed shelf", packed), ("spoil shelf", spoil)].into_iter().enumerate() {
                    let x0 = 30 + i as i32 * 140;
                    let pit = rect(x0, roof_y - 2, 40, 12);
                    // The shelf: rows roof_y-4..roof_y-2, x0..x0+14, over the pit.
                    let shelf = rect(x0, roof_y - 4, 14, 2);
                    // Air over the shelf too, so it is a ledge and not a roof.
                    let above = rect(x0, roof_y - 10, 40, 8);
                    // Carve pit, air and shelf footprint together and line
                    // the cut, so the walls the shelf hangs from are worked
                    // ground as an ant would leave them; then write the
                    // shelf back into the hole in its own material.
                    let carved: HashSet<(i32, i32)> =
                        pit.iter().chain(above.iter()).copied().collect();
                    for &(x, y) in &carved {
                        w.set(x, y, Cell::EMPTY);
                    }
                    line(&mut w, &carved);
                    let above: Vec<(i32, i32)> =
                        above.into_iter().filter(|c| !shelf.contains(c)).collect();
                    let mut built = Vec::new();
                    for &(x, y) in &shelf {
                        let mut c = w.get(x, y);
                        c.material = m;
                        w.set(x, y, c);
                        built.push((x, y, m));
                    }
                    let mut cells = pit.clone();
                    cells.extend(above.iter().copied());
                    voids.push(Region { name: format!("{name} pit"), cells });
                    structures.push((Region { name: name.to_string(), cells: shelf }, built));
                }
            }
            _ => unreachable!(),
        }

        if arm != "shelf" {
            for v in &voids {
                for &(x, y) in &v.cells {
                    w.set(x, y, Cell::EMPTY);
                }
            }
            if arm != "loose" {
                let carved: HashSet<(i32, i32)> =
                    voids.iter().flat_map(|v| v.cells.iter().copied()).collect();
                let n = line(&mut w, &carved);
                lines.push(format!("{arm}: wall cells worked into packedsoil: {n}"));
            }
            // The slab's material is whatever lining made of it.
            for (s, built) in structures.iter_mut() {
                *built = s.cells.iter().map(|&(x, y)| (x, y, w.get(x, y).material)).collect();
            }
        }

        // Scene check: nothing standing in a carved cell before a tick.
        for v in &voids {
            assert_eq!(v.caved(&w), 0, "{arm}/{}: ground in the hole at frame 0", v.name);
        }

        let mut particles = ParticleSystem::default();
        let mut blasts = Blasts::default();
        let tuning = player::Tuning::default();
        let before = shoot(&w);
        for _ in 0..frames {
            frame::step(&mut w, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }
        let after = shoot(&w);

        println!("\n=== {arm} (bank aux {bank_aux}) ===");
        for v in &voids {
            println!("  {:<26} caved {:>4} / {:<4}", v.name, v.caved(&w), v.cells.len());
        }
        for (s, built) in &structures {
            println!("  {:<26} held  {:>4} / {:<4}", s.name, s.held(&w, built), built.len());
        }
        sheet_rows.push((before, after));
    }
    for l in lines {
        println!("{l}");
    }

    if let Some(out) = png {
        // Two columns -- frame 0 and the last frame -- one row per arm,
        // 2 px dividers, nearest-neighbour zoom.
        let (_, _, cw, ch) = CROP;
        let (cw, ch) = (cw as u32, ch as u32);
        let gap = 2u32;
        let sw = (cw * 2 + gap) * zoom;
        let rows = sheet_rows.len() as u32;
        let sh = (ch * rows + gap * (rows - 1)) * zoom;
        let mut sheet = vec![40u8; (sw * sh * 4) as usize];
        for (r, (a, b)) in sheet_rows.iter().enumerate() {
            for (col, img) in [a, b].into_iter().enumerate() {
                let ox = col as u32 * (cw + gap) * zoom;
                let oy = r as u32 * (ch + gap) * zoom;
                for y in 0..ch * zoom {
                    for x in 0..cw * zoom {
                        let s = (((y / zoom) * cw + x / zoom) * 4) as usize;
                        let d = (((oy + y) * sw + ox + x) * 4) as usize;
                        sheet[d..d + 4].copy_from_slice(&img[s..s + 4]);
                    }
                }
            }
        }
        image::save_buffer(&out, &sheet, sw, sh, image::ColorType::Rgba8).expect("writing the sheet");
        println!("\nwrote {out} ({sw}x{sh}): rows dry, wet, loose, bridge, shelf; left frame 0, right frame {frames}");
    }
}
