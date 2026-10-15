//! **Can this world grow a pillar from a building mark?** -- the second
//! substrate test of PR 701's follow-up
//! (`/mnt/project-files/emergence-sandbox/plan-2026-10-14.md`), after
//! Khuong et al. 2016 (*Stigmergic construction and topochemical
//! information shape ant nest architecture*, PNAS 113:1303).
//!
//! In Khuong's model a builder drops its pellet where pellets were recently
//! dropped, read from a mark that the pellet carries and that fades with a
//! lifetime. Positive feedback on fresh building makes discrete pillars, and
//! the mark's lifetime is "a highly influential parameter": too short and
//! deposits never reinforce, too long and every old deposit attracts, so the
//! building spreads into a blanket. The repo declined this rule before it
//! was built because the lining erased the marker (`dead-ends.md`, the Khuong
//! entry), so it has never been run here.
//!
//! The walkers are **not the shipped ant**, as in `sortbox`: no brain, no
//! energy, no pulls. They cling to surfaces (they may stand only in an open
//! cell touching something, else they fall a cell). An unladen walker beside
//! the source (a loose `soil` floor) takes a cell with `pick=` (0.05). A
//! laden walker considers the open cells beside it that touch something
//! solid, and drops into one with `p0 + (1 - p0) * m / (m + K)`, where `m`
//! is the sum of fresh marks in the 3x3 round that cell (each mark is
//! `exp(-age / tau)`), `p0=` 0.002 and `K=` 1. The dropped cell is marked.
//! **The mark lives in this harness, on the cell's position**: when the cell
//! falls away the mark is gone with it, which is what the engine would do
//! too, since nothing in it carries a mark on a moving cell.
//!
//! **Arms change the medium and the mark, never the rule:**
//! - `build=` the material dropped: `spoil` (stands only on something
//!   directly beneath it -- `needs_footing`), `soil` (loose, falls and rolls),
//!   or `packedsoil` (never moves; the stamp the nest walls are made of).
//! - `tau=` the mark's lifetime in frames; `0` is no mark at all (p0 only,
//!   the control for "the mark does the work"), `inf` a mark that never fades.
//! - `physics=off` holds every dropped cell where it was put, the
//!   textbook setting and the positive control.
//! - `sites=game` drops only where the shipped ant would put spoil (two of
//!   the three cells below filled, three open above). The default, `any`,
//!   is the sandbox's own looser rule: any open cell touching something,
//!   walls and ceilings included. **The two give different counts**
//!   (4 seeds, 60k frames, `build=spoil tau=300`: 264-283 cells standing
//!   under `any`, 357-407 under `game`), so a number from this binary
//!   measures its drop rule as well as the medium.
//!
//! Read per stop: built cells standing (dropped and still the built
//! material), structures (8-connected groups of built cells), **pillars**
//! (structures at least 5 tall and taller than they are wide, not touching
//! the box's side walls or ceiling -- a wall lining is not a pillar), the
//! tallest structure of any kind,
//! and the share of built cells in pillars. `png=`/`gif=` as `sortbox`.

use pixel_physics::sim::parallel;
use pixel_physics::sim::rng::Rng;
use pixel_physics::sim::{Cell, MaterialId, Rect, World};

/// Every key this binary reads. An unknown or repeated key panics rather
/// than silently running another arm (a typo once ran the wrong one).
const KEYS: &[&str] = &[
    "seed", "seeds", "frames", "every", "walkers", "width", "height", "build", "source", "tau", "physics", "rework",
    "pick", "p0", "K", "sites", "png", "gif", "gifevery",
];

fn check_args() {
    let mut seen = std::collections::HashSet::new();
    for a in std::env::args().skip(1) {
        let key = a.split_once('=').map_or(a.as_str(), |(k, _)| k).to_string();
        assert!(
            KEYS.contains(&key.as_str()),
            "pillarbox: unknown argument `{a}` (known: {})",
            KEYS.join(", ")
        );
        assert!(seen.insert(key.clone()), "pillarbox: `{key}` given twice");
    }
}

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| {
            v.parse()
                .ok()
                .unwrap_or_else(|| panic!("pillarbox: `{a}` does not parse"))
        })
    })
}

const DIRS: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];

/// Thickness of the stone box, in cells.
const WALL: i32 = 3;

struct Walker {
    x: i32,
    y: i32,
    dir: usize,
    laden: bool,
}

fn main() {
    check_args();
    let seeds: Vec<u64> = match arg::<u64>("seed") {
        Some(s) => vec![s],
        None => (1..=arg::<u64>("seeds").unwrap_or(4)).collect(),
    };
    let frames: u64 = arg("frames").unwrap_or(60_000);
    let every: u64 = arg("every").unwrap_or(15_000);
    let n_walkers: usize = arg("walkers").unwrap_or(80);
    let build: String = arg("build").unwrap_or_else(|| "spoil".to_string());
    let tau: f32 = match arg::<String>("tau").as_deref() {
        Some("inf") => f32::INFINITY,
        Some(v) => v.parse().expect("tau=<frames>|inf"),
        None => 2_000.0,
    };
    let physics = arg::<String>("physics").as_deref() != Some("off");
    // `rework=0` keeps building out of the source (only the floor's soil is taken).
    let rework = arg::<u8>("rework").unwrap_or(1) == 1;
    let pick: f32 = arg("pick").unwrap_or(0.05);
    let p0: f32 = arg("p0").unwrap_or(0.002);
    let k: f32 = arg("K").unwrap_or(1.0);
    let (w, h): (i32, i32) = (arg("width").unwrap_or(200), arg("height").unwrap_or(90));
    let png: Option<String> = arg("png");
    let gif: Option<String> = arg("gif");
    let gif_every: u64 = arg("gifevery").unwrap_or(1_000);
    println!("pillarbox: build={build} tau={tau} rework={rework} physics={} frames={frames} walkers={n_walkers} pick={pick} p0={p0} K={k} box={w}x{h}", if physics { "on" } else { "off" });
    for seed in seeds {
        run(
            seed,
            frames,
            every,
            n_walkers,
            &build,
            tau,
            physics,
            rework,
            pick,
            p0,
            k,
            w,
            h,
            png.as_deref(),
            gif.as_deref(),
            gif_every,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    seed: u64,
    frames: u64,
    every: u64,
    n_walkers: usize,
    build: &str,
    tau: f32,
    physics: bool,
    rework: bool,
    pick: f32,
    p0: f32,
    k: f32,
    w: i32,
    h: i32,
    png: Option<&str>,
    gif: Option<&str>,
    gif_every: u64,
) {
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    let stone = world.materials.id_of("stone").expect("stone");
    // `source=` the floor the pellets are taken from (loose `soil` by default).
    let source: String = arg("source").unwrap_or_else(|| "soil".to_string());
    let soil = world
        .materials
        .id_of(&source)
        .unwrap_or_else(|| panic!("no material {source}"));
    let built = world
        .materials
        .id_of(build)
        .unwrap_or_else(|| panic!("no material {build}"));
    // The census counts every cell of the built material, so building out
    // of the floor's own material would count the floor (1,940 "built" at
    // frame 0) and let fresh building be picked straight back up.
    assert!(
        built != soil,
        "pillarbox: build={build} is the source floor too; pass source= another material"
    );
    // `sites=game` drops only where the shipped ant would put spoil
    // (`creature::spoil_site_fits`, unfooted form): two of the three cells
    // below filled and three open cells above. The default, `any`, drops
    // into any open cell touching something, walls and ceilings included,
    // where `spoil` mostly slumps straight back to soil.
    let game_sites = match arg::<String>("sites").as_deref() {
        None | Some("any") => false,
        Some("game") => true,
        Some(v) => panic!("pillarbox: sites={v}: want any or game"),
    };
    let wall = WALL;
    let floor_top = h - wall - 10;
    for y in 0..h {
        for x in 0..w {
            if x < wall || y < wall || x >= w - wall || y >= h - wall {
                world.set(x, y, Cell::new(stone, 0));
            } else if y >= floor_top {
                world.set(x, y, Cell::new(soil, 0));
            }
        }
    }
    // Let the source settle before anyone walks.
    if physics {
        for _ in 0..200 {
            parallel::step(&mut world);
        }
    }
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    // Frame each cell was dropped at, or `u64::MAX` for none.
    let mut mark: Vec<u64> = vec![u64::MAX; (w * h) as usize];
    let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x9111a);
    let open = |wd: &World, x: i32, y: i32| wd.in_bounds(x, y) && wd.is_empty(x, y);
    let touching = |wd: &World, x: i32, y: i32| {
        DIRS.iter()
            .any(|&(dx, dy)| wd.in_bounds(x + dx, y + dy) && !wd.is_empty(x + dx, y + dy))
    };
    let mut walkers = Vec::new();
    while walkers.len() < n_walkers {
        let x = wall + 1 + rng.below((w - 2 * wall - 2) as u32) as i32;
        let y = floor_top - 1;
        if open(&world, x, y) {
            walkers.push(Walker {
                x,
                y,
                dir: rng.below(8) as usize,
                laden: false,
            });
        }
    }
    let mut shots = Vec::new();
    let (mut picks, mut drops) = (0u64, 0u64);
    for f in 0..=frames {
        if f > 0 {
            if physics {
                parallel::step(&mut world);
            }
            // A mark goes with its cell: a cell that is no longer the built
            // material (it fell away or was taken) has no mark.
            for y in wall..h - wall {
                for x in wall..w - wall {
                    let i = idx(x, y);
                    if mark[i] != u64::MAX && world.get(x, y).material != built {
                        mark[i] = u64::MAX;
                    }
                }
            }
            let fresh = |mark: &Vec<u64>, x: i32, y: i32| -> f32 {
                if tau == 0.0 {
                    return 0.0;
                }
                let mut m = 0.0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (px, py) = (x + dx, y + dy);
                        if px < 0 || py < 0 || px >= w || py >= h {
                            continue;
                        }
                        let t = mark[idx(px, py)];
                        if t != u64::MAX {
                            m += if tau.is_infinite() {
                                1.0
                            } else {
                                (-((f - t) as f32) / tau).exp()
                            };
                        }
                    }
                }
                m
            };
            // This cell's own mark, 0 for none.
            let own_mark = |mark: &Vec<u64>, x: i32, y: i32| -> f32 {
                let t = mark[idx(x, y)];
                if t == u64::MAX || tau == 0.0 {
                    0.0
                } else if tau.is_infinite() {
                    1.0
                } else {
                    (-((f - t) as f32) / tau).exp()
                }
            };
            for i in 0..walkers.len() {
                let (x, y) = (walkers[i].x, walkers[i].y);
                if !touching(&world, x, y) {
                    if open(&world, x, y + 1) {
                        walkers[i].y += 1;
                    }
                    continue;
                }
                if !walkers[i].laden {
                    // **Any pellet whose own mark has faded is material**, the
                    // source floor and old building alike, as Khuong's ants
                    // rework their own deposits. Without this the first
                    // blanket of building buries the source and construction
                    // stops at about one layer (every setting of a 12-arm
                    // sweep stopped at 1,300-1,600 cells, 2026-10-14).
                    let src: Vec<(i32, i32)> = DIRS
                        .iter()
                        .map(|&(dx, dy)| (x + dx, y + dy))
                        .filter(|&(px, py)| {
                            if !world.in_bounds(px, py) {
                                return false;
                            }
                            let m = world.get(px, py).material;
                            m == soil || (rework && m == built && own_mark(&mark, px, py) < 0.1)
                        })
                        .collect();
                    if !src.is_empty() && rng.unit_f32() < pick {
                        let (px, py) = src[rng.below(src.len() as u32) as usize];
                        world.set(px, py, Cell::EMPTY);
                        walkers[i].laden = true;
                        picks += 1;
                    }
                } else {
                    let spots: Vec<(i32, i32)> = DIRS
                        .iter()
                        .map(|&(dx, dy)| (x + dx, y + dy))
                        .filter(|&(px, py)| {
                            open(&world, px, py)
                                && touching(&world, px, py)
                                && (!game_sites || game_site(&world, px, py))
                                && !walkers.iter().any(|o| (o.x, o.y) == (px, py))
                        })
                        .collect();
                    if !spots.is_empty() {
                        let (px, py) = spots[rng.below(spots.len() as u32) as usize];
                        let m = fresh(&mark, px, py);
                        let p = p0 + (1.0 - p0) * m / (m + k);
                        if rng.unit_f32() < p {
                            world.set(px, py, Cell::new(built, 0));
                            mark[idx(px, py)] = f;
                            walkers[i].laden = false;
                            drops += 1;
                        }
                    }
                }
                if rng.below(8) == 0 {
                    walkers[i].dir = (walkers[i].dir + if rng.flip() { 1 } else { 7 }) % 8;
                }
                let mut moved = false;
                for attempt in 0..8 {
                    let d = (walkers[i].dir + [0, 1, 7, 2, 6, 3, 5, 4][attempt]) % 8;
                    let (nx, ny) = (x + DIRS[d].0, y + DIRS[d].1);
                    if open(&world, nx, ny) && touching(&world, nx, ny) {
                        walkers[i].x = nx;
                        walkers[i].y = ny;
                        walkers[i].dir = d;
                        moved = true;
                        break;
                    }
                }
                if !moved {
                    walkers[i].dir = rng.below(8) as usize;
                }
            }
            for wk in walkers.iter_mut() {
                while !open(&world, wk.x, wk.y) && wk.y > wall {
                    wk.y -= 1;
                }
            }
        }
        if gif.is_some() && f % gif_every == 0 {
            shots.push(draw(&world, &walkers, stone, soil, built, w, h));
        }
        if f % every == 0 {
            let c = census(&world, built, w, h);
            println!(
                "seed {seed} f {f:>6}: built {} | structures {} | pillars {} (cells in pillars {:.0}%, {} at least 30 tall) | tallest {} | laden {} | picks {picks} drops {drops}",
                c.cells,
                c.structures,
                c.pillars,
                100.0 * c.pillar_cells as f32 / c.cells.max(1) as f32,
                c.tall,
                c.tallest,
                walkers.iter().filter(|w| w.laden).count()
            );
            if let Some(dir) = png {
                let _ = std::fs::create_dir_all(dir);
                let _ = draw(&world, &walkers, stone, soil, built, w, h).save(format!("{dir}/s{seed}_f{f:06}.png"));
            }
        }
    }
    if let Some(path) = gif {
        let delay = image::Delay::from_numer_denom_ms(100, 1);
        let fr: Vec<image::Frame> = shots
            .into_iter()
            .map(|img| image::Frame::from_parts(img, 0, 0, delay))
            .collect();
        let file = std::fs::File::create(path).expect("gif path");
        let mut enc = image::codecs::gif::GifEncoder::new(file);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
        enc.encode_frames(fr).expect("encode");
        println!("pillarbox: wrote {path}");
    }
}

struct Census {
    cells: usize,
    structures: usize,
    pillars: usize,
    pillar_cells: usize,
    tallest: i32,
    /// Pillars at least [`TALL_PILLAR`] high: the no-mark control (`tau=0`)
    /// makes 5-27-tall lumps that the 5-tall bar counts as pillars (7 on
    /// each of seeds 1-2, `sites=game rework=0`), so the bar is read here.
    tall: usize,
}

/// The height a pillar counts as tall at, for [`Census::tall`].
const TALL_PILLAR: i32 = 30;

/// `creature::spoil_site_fits` without the footing test (`SPOIL_HEADROOM`
/// is 3): two of the three cells below filled, three open cells above.
fn game_site(wd: &World, x: i32, y: i32) -> bool {
    [(-1, 1), (0, 1), (1, 1)]
        .iter()
        .filter(|(dx, dy)| !wd.is_empty(x + dx, y + dy))
        .count()
        >= 2
        && (1..=3).all(|dy| wd.is_empty(x, y - dy))
}

fn census(wd: &World, built: MaterialId, w: i32, h: i32) -> Census {
    let mut seen = std::collections::HashSet::new();
    let mut c = Census {
        cells: 0,
        structures: 0,
        pillars: 0,
        pillar_cells: 0,
        tallest: 0,
        tall: 0,
    };
    for y in 0..h {
        for x in 0..w {
            if wd.get(x, y).material != built || !seen.insert((x, y)) {
                continue;
            }
            let mut stack = vec![(x, y)];
            let (mut n, mut x0, mut x1, mut y0, mut y1) = (0usize, x, x, y, y);
            while let Some((cx, cy)) = stack.pop() {
                n += 1;
                x0 = x0.min(cx);
                x1 = x1.max(cx);
                y0 = y0.min(cy);
                y1 = y1.max(cy);
                for &(dx, dy) in &DIRS {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if wd.in_bounds(nx, ny) && wd.get(nx, ny).material == built && seen.insert((nx, ny)) {
                        stack.push((nx, ny));
                    }
                }
            }
            c.cells += n;
            c.structures += 1;
            let (bw, bh) = (x1 - x0 + 1, y1 - y0 + 1);
            c.tallest = c.tallest.max(bh);
            // A lining up the box's side wall or a growth hanging from its
            // ceiling is not a pillar, however tall and narrow it is.
            let on_box = x0 <= WALL || x1 >= w - WALL - 1 || y0 <= WALL;
            if bh >= 5 && bh > bw && !on_box {
                c.pillars += 1;
                c.pillar_cells += n;
                c.tall += usize::from(bh >= TALL_PILLAR);
            }
        }
    }
    c
}

fn draw(
    wd: &World,
    walkers: &[Walker],
    stone: MaterialId,
    soil: MaterialId,
    built: MaterialId,
    w: i32,
    h: i32,
) -> image::RgbaImage {
    const SCALE: u32 = 3;
    let mut img = image::RgbaImage::from_pixel(w as u32 * SCALE, h as u32 * SCALE, image::Rgba([24, 22, 28, 255]));
    let mut put = |x: i32, y: i32, c: [u8; 4]| {
        for dy in 0..SCALE {
            for dx in 0..SCALE {
                img.put_pixel(x as u32 * SCALE + dx, y as u32 * SCALE + dy, image::Rgba(c));
            }
        }
    };
    for y in 0..h {
        for x in 0..w {
            let m = wd.get(x, y).material;
            if m == stone {
                put(x, y, [95, 95, 100, 255]);
            } else if m == soil {
                put(x, y, [110, 80, 55, 255]);
            } else if m == built {
                put(x, y, [220, 170, 90, 255]);
            }
        }
    }
    for wk in walkers {
        put(
            wk.x,
            wk.y,
            if wk.laden {
                [255, 230, 60, 255]
            } else {
                [220, 50, 50, 255]
            },
        );
    }
    img
}
