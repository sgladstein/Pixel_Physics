//! **Do the research digging rules dig, in the research's own setup?**
//! (`/mnt/project-files/emergence-sandbox/textbook-ant/why-digging-fails/`,
//! the owner's "Run that test", 2026-10-17.)
//!
//! Every excavation experiment our textbook ant's digging rules come from
//! started the ants **inside** a hole with soil all round (Toffin et al. 2009,
//! a central hole; Buhl et al. 2004, the edge of a sand disk), in **moist,
//! cohesive** soil, with the dug soil **carried out of the arena**. In the
//! lab box none of the three holds, and the nest stays a pit. This box gives
//! the rules all three and nothing else:
//!
//! - a bed of ground over stone (`bed=packedsoil`, the default, holds its own
//!   weight as moist soil does; `bed=soil` is our loose powder);
//! - a round cavity of radius `r=` cut `depth=` rows into the bed, the ants
//!   planted in it and sealed in (there is no door);
//! - the dug soil removed: run it with `PIXEL_PHYSICS_DIG_SPOIL=destroy`
//!   (`creature::spoil_kept`), which the binary checks and prints.
//!
//! The ants are the textbook ant ([`textbook::NestCfg`]) with every ant a
//! digger, no food and no foragers, fed every 500 frames, births paused,
//! mutation off. The rules are the ones already built: a correlated random
//! walk, cut ahead with chance `dig x min(1, nestmates within 2 / digcrowd)`
//! (Rasse & Deneubourg 2001; Buhl et al. 2004), back out of a jam (Aguilar
//! et al. 2018). `down=` (default 0) is the walk's downward pull, which no
//! paper has. **The control is `digcrowd=0`**: digging ignores company.
//!
//! Read per stop: cells dug, the ants, and the shape -- `reach` is the
//! furthest dug cell from the start's centre over the radius of a disc of
//! the same area (1 for a round room, well above for tunnels), and `tips`
//! counts dug cells with exactly one dug neighbour of four (tunnel ends).
//! `png=DIR` draws each stop.

use pixel_physics::sim::creature::{self, textbook};
use pixel_physics::sim::{material, parallel, Cell, Rect, World};

const KEYS: &[&str] = &[
    "seed", "frames", "every", "ants", "r", "depth", "bed", "dig", "digcrowd", "down", "jam", "turn", "png", "width",
    "height", "line", "vanish",
];

fn check_args() {
    for a in std::env::args().skip(1) {
        let key = a.split('=').next().unwrap_or("");
        assert!(KEYS.contains(&key), "holebox: unknown key in `{a}`");
    }
}

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| {
            v.parse()
                .ok()
                .unwrap_or_else(|| panic!("holebox: `{a}` does not parse"))
        })
    })
}

const WALL: i32 = 3;
const FEED_EVERY: u64 = 500;

fn main() {
    check_args();
    creature::mutation_off_for_measuring();
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(40_000);
    let every: u64 = arg("every").unwrap_or(10_000);
    let n: usize = arg("ants").unwrap_or(40);
    let r: i32 = arg("r").unwrap_or(8);
    let depth: i32 = arg("depth").unwrap_or(40);
    let bed: String = arg("bed").unwrap_or_else(|| "packedsoil".into());
    let (w, h): (i32, i32) = (arg("width").unwrap_or(240), arg("height").unwrap_or(160));
    let png: Option<String> = arg("png");
    let destroy = std::env::var("PIXEL_PHYSICS_DIG_SPOIL").as_deref() == Ok("destroy");
    println!(
        "holebox: seed={seed} frames={frames} ants={n} r={r} depth={depth} bed={bed} spoil {} box={w}x{h}",
        if destroy {
            "removed (DIG_SPOIL=destroy)"
        } else {
            "KEPT (set PIXEL_PHYSICS_DIG_SPOIL=destroy)"
        }
    );

    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let stone = world.materials.id_of("stone").expect("stone");
    let ground = world
        .materials
        .id_of(&bed)
        .unwrap_or_else(|| panic!("no material {bed}"));
    let bed_top = 20;
    let (cx, cy) = (w / 2, bed_top + depth);
    let in_hole = |x: i32, y: i32| (x - cx).pow(2) + (y - cy).pow(2) <= r * r;
    for y in 0..h {
        for x in 0..w {
            if x < WALL || x >= w - WALL || y >= h - WALL {
                world.set(x, y, Cell::new(stone, 0));
            } else if y >= bed_top && !in_hole(x, y) {
                world.set(x, y, Cell::new(ground, 0));
            }
        }
    }
    // `line=1`: a ring of packed soil two cells thick (one leaks on its
    // diagonals) round the starting hole, the lining an
    // ant leaves on every wall it cuts (`creature::line_burrow`), so a loose
    // bed has a hole to start in at all (unlined, it fills in while the bed
    // settles and no ant can be planted).
    if arg::<u8>("line").unwrap_or(0) == 1 {
        let packed = world.materials.id_of("packedsoil").expect("packedsoil");
        for y in cy - r - 2..=cy + r + 2 {
            for x in cx - r - 2..=cx + r + 2 {
                if !in_hole(x, y) && (x - cx).pow(2) + (y - cy).pow(2) <= (r + 2).pow(2) {
                    world.set(x, y, Cell::new(packed, 0));
                }
            }
        }
    }
    for _ in 0..200 {
        parallel::step(&mut world);
    }
    let at_start: Vec<bool> = (0..w * h)
        .map(|i| ![material::EMPTY, stone].contains(&world.get(i % w, i / w).material))
        .collect();

    // Plant the colony in the hole: every open cell of it, in a seeded order.
    let id = world.species.id_of("ant").expect("ant ships");
    if let Some(c) = world.species.get_mut(id).creature.as_mut() {
        c.life_half_life = 10_000_000;
    }
    world.births_paused = true;
    let mut spots: Vec<(i32, i32)> = (cy - r..=cy + r)
        .flat_map(|y| (cx - r..=cx + r).map(move |x| (x, y)))
        .filter(|&(x, y)| in_hole(x, y))
        .collect();
    let mut rng = pixel_physics::sim::rng::Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x401e);
    for i in (1..spots.len()).rev() {
        spots.swap(i, rng.below(i as u32 + 1) as usize);
    }
    // Floor cells first, so the planting does not depend on bodies landing.
    spots.sort_by_key(|&(x, y)| !(in_hole(x, y) && !in_hole(x, y + 1)));
    let mut colony = None;
    let mut placed = 0;
    for &(x, y) in &spots {
        if placed == n {
            break;
        }
        if let Some(site) = creature::plant_founder_in(&mut world, x, y, "ant", colony, None) {
            if colony.is_none() {
                colony = creature::colony_of_site(&world, &site);
            }
            world.schedule_active_site(site);
            placed += 1;
        }
    }
    let cfg = cfg(ground, (cx, cy));
    println!("  ants placed {placed} of {n}; textbook {:?}", cfg.nest);
    world.textbook = Some(Box::new(textbook::Textbook::new(id, cfg)));

    for f in 0..=frames {
        if f > 0 {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
            world.step_pheromones();
            if f % FEED_EVERY == 0 {
                feed(&mut world);
            }
        }
        if f % every == 0 {
            println!("{}", census(&world, &at_start, w, h, (cx, cy), r, f));
            if let Some(dir) = png.as_deref() {
                draw(&world, &at_start, w, h, stone, ground, dir, f);
            }
        }
    }
}

fn cfg(ground: material::MaterialId, home: (i32, i32)) -> textbook::TextbookCfg {
    // No food in this box; the forage half is only there because the nest
    // half is routed through it, and every ant is a digger.
    let forage = textbook::ForageCfg {
        food: material::EMPTY,
        home,
        home_r: 4,
        aim_home: home,
        feeder: home,
        vector: 0.0,
        vector_out: 0.0,
        out_flat: false,
        k: 2.0,
        n: 2,
        lay: 0.0,
        lay_out: false,
        out_frac: 1.0,
        read: false,
        give_up: 300,
        search_r: 12,
        sense: 6,
        crowd: 0.0,
        no_entry: 0.0,
        repel: 0.0,
        mark_fork: false,
        mark_to_fork: false,
        explore: 0.0,
        explore_c: 0.0,
        explore_k: 0.0,
        explore_max: 0.0,
        explore_lay: 0.0,
        kn: false,
    };
    let nest = textbook::NestCfg {
        forager_frac: 0.0,
        dig: arg("dig").unwrap_or(0.2),
        dig_crowd: arg("digcrowd").unwrap_or(3.0),
        roof: 6,
        up: 1.0,
        down: arg("down").unwrap_or(0.0),
        home: 0.0,
        clear_r: 4,
        jam: arg("jam").unwrap_or(3),
        loose: false,
        straight: false,
        inside_only: false,
        wait: 0,
        wait_p: 0.0,
        dig_new: 1.0,
        dig_fresh: 600,
        dig_pull: 0.0,
        vanish: arg::<u8>("vanish").unwrap_or(1) == 1,
        store_r: 0,
    };
    textbook::TextbookCfg {
        p_move: 1.0,
        turn: arg("turn").unwrap_or(0.125),
        dt: 4.0,
        drop_base: 0.025,
        drop_per: 0.11,
        pick: 0.029,
        tau: 1_800.0,
        template: 2.0,
        source: ground,
        n_rock: false,
        pick_around: true,
        base_always: true,
        build_on_box: true,
        forage: Some(forage),
        sort: None,
        nest: Some(nest),
    }
}

fn feed(world: &mut World) {
    use pixel_physics::sim::world::Account;
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        let Some(start) = world.species.get(st.species).creature.as_ref().map(|c| c.start_energy) else {
            continue;
        };
        let short = start - st.energy;
        if short <= 0.0 {
            continue;
        }
        let colony = world.colony_of(id);
        world.set_organism_energy(id, start);
        world.book(colony, Account::Granted, f64::from(short));
    }
}

/// A cell dug: ground at the start, not ground now (an ant in it counts as
/// dug), or the starting hole itself.
fn dug_mask(world: &World, at_start: &[bool], w: i32, h: i32, (cx, cy): (i32, i32), r: i32) -> Vec<bool> {
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let c = world.get(x, y);
            let open = c.material == material::EMPTY || c.organism_id() != 0;
            (at_start[i as usize] && open) || ((x - cx).pow(2) + (y - cy).pow(2) <= r * r && open)
        })
        .collect()
}

fn census(world: &World, at_start: &[bool], w: i32, h: i32, centre: (i32, i32), r: i32, f: u64) -> String {
    let mask = dug_mask(world, at_start, w, h, centre, r);
    let at = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && mask[(y * w + x) as usize];
    let area = mask.iter().filter(|&&b| b).count();
    let (mut far, mut tips, mut deep, mut up) = (0.0f32, 0, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if !at(x, y) {
                continue;
            }
            far = far.max((((x - centre.0).pow(2) + (y - centre.1).pow(2)) as f32).sqrt());
            let nb = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .filter(|&&(dx, dy)| at(x + dx, y + dy))
                .count();
            tips += usize::from(nb == 1);
            deep = deep.max(y - centre.1);
            up = up.max(centre.1 - y);
        }
    }
    let disc = (area as f32 / std::f32::consts::PI).sqrt();
    let ants = world
        .live_organism_ids()
        .into_iter()
        .filter(|&id| {
            world
                .organism(id)
                .is_some_and(|s| world.species.get(s.species).creature.is_some() && !s.chain.is_empty())
        })
        .count();
    let c = world.textbook.as_ref().map(|t| t.counts);
    format!(
        "f={f} ants={ants} dug {} (hole {}) reach {:.2} tips {tips} deepest +{deep} highest -{up} | digs {} dig_rolls {}",
        area.saturating_sub(0),
        (std::f32::consts::PI * (r * r) as f32) as usize,
        if disc > 0.0 { far / disc } else { 0.0 },
        c.map_or(0, |c| c.digs),
        c.map_or(0, |c| c.dig_rolls),
    )
}

#[allow(clippy::too_many_arguments)]
fn draw(
    world: &World,
    at_start: &[bool],
    w: i32,
    h: i32,
    stone: material::MaterialId,
    ground: material::MaterialId,
    dir: &str,
    f: u64,
) {
    const SCALE: u32 = 3;
    std::fs::create_dir_all(dir).expect("png dir");
    let mut img = image::RgbaImage::from_pixel(w as u32 * SCALE, h as u32 * SCALE, image::Rgba([24, 22, 28, 255]));
    for y in 0..h {
        for x in 0..w {
            let c = world.get(x, y);
            let col = if c.organism_id() != 0 {
                [235, 200, 60, 255]
            } else if c.material == stone {
                [95, 95, 100, 255]
            } else if c.material == ground {
                [120, 88, 60, 255]
            } else if c.material != material::EMPTY {
                [70, 52, 40, 255]
            } else if at_start[(y * w + x) as usize] {
                [10, 10, 12, 255]
            } else {
                [24, 22, 28, 255]
            };
            for dy in 0..SCALE {
                for dx in 0..SCALE {
                    img.put_pixel(x as u32 * SCALE + dx, y as u32 * SCALE + dy, image::Rgba(col));
                }
            }
        }
    }
    img.save(format!("{dir}/hole_f{f:06}.png")).expect("png");
}
