//! **A film of minds walking the trail maze** (`train::challenges::Maze`),
//! one band per genome, stacked, so two brains on the same maze can be
//! watched side by side.
//!
//! ```text
//! cargo run --release --example mazefilm -- genomes=follower,s171.txt,h1.txt k=0 out=maze.gif
//! ```
//!
//! `genomes=` is a comma list of `mind::preset` names or mind files; `k=` the
//! hold-out layout (`train::holdout_layouts`); `every=` frames between shots
//! (100); `frames=` the run (12,000); `scale=` pixels per cell (2); `forks=`,
//! `ramp=`, `spread=`, `blend=`, `walls=` as `examples/train`. `open=1` films
//! the open-air maze (`train::challenges::OpenMaze`) instead: the frame's
//! bars grey, the path's bars light grey, the floor grey. `blind=1` cuts
//! each genome's scent reads (`mind::blinded`). Drawn with fixed colours, not
//! the game renderer: rock grey, dead ends dark red, the trail's channel B as
//! green over open cells (brighter is stronger, on a fixed ramp), ants white
//! with the head yellow, the goal orange. Prints each band's score.
use pixel_physics::sim::creature::mind;
use pixel_physics::sim::pheromone::{self, Channel};
use pixel_physics::train::challenges::maze::{Kind, Plan, Walls};
use pixel_physics::train::challenges::{openmaze, Maze, OpenMaze};
use pixel_physics::train::{self, Challenge};

fn main() {
    let mut genomes = "follower".to_string();
    let (mut k, mut every, mut frames, mut scale) = (0u64, 100u64, 12_000u64, 2u32);
    let mut out = "maze.gif".to_string();
    let mut maze = Maze {
        ants: 6,
        frames: 12_000,
        clones: false,
        forks: 3,
        ramp: true,
        spread: 0,
        blend: None,
        walls: Walls::Through,
        laden: false,
    };
    let mut blind = false;
    let mut open = false;
    for a in std::env::args().skip(1) {
        let (key, v) = a.split_once('=').unwrap_or((a.as_str(), "1"));
        match key {
            "genomes" => genomes = v.into(),
            "k" => k = v.parse().unwrap(),
            "every" => every = v.parse().unwrap(),
            "frames" => frames = v.parse().unwrap(),
            "scale" => scale = v.parse().unwrap(),
            "out" => out = v.into(),
            "ants" => maze.ants = v.parse().unwrap(),
            "forks" => maze.forks = v.parse().unwrap(),
            "ramp" => maze.ramp = v == "1",
            "spread" => maze.spread = v.parse().unwrap(),
            "blend" => maze.blend = Some(v.parse().unwrap()),
            "walls" => {
                maze.walls = match v {
                    "through" => Walls::Through,
                    "sink" => Walls::Sink,
                    "reflect" => Walls::Reflect,
                    _ => panic!("walls={v}: through|sink|reflect"),
                }
            }
            "blind" => blind = v == "1",
            "open" => open = v == "1",
            "laden" => maze.laden = v == "1",
            _ => panic!("unknown argument {a}"),
        }
    }
    maze.frames = frames;
    println!(
        "mazefilm genomes={genomes} k={k} every={every} frames={frames} scale={scale} ants={} forks={} ramp={} spread={} blend={:?} walls={:?} blind={blind} open={open} laden={} out={out}",
        maze.ants, maze.forks, maze.ramp, maze.spread, maze.blend, maze.walls, maze.laden
    );
    let layout = train::holdout_layouts(k as usize + 1)[k as usize];
    let (w, h) = (256i32, 112i32);
    let opened = OpenMaze {
        ants: maze.ants,
        frames,
        clones: false,
        ramp: maze.ramp,
        laden: maze.laden,
    };
    // The base colour of every cell, and whether the trail is drawn over it.
    let (base, y0, y1): (Vec<([u8; 3], bool)>, i32, i32) = if open {
        let plan = openmaze::Plan::draw(layout);
        let base = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                if plan.path[i as usize] != i32::MAX {
                    ([150, 150, 160], false)
                } else if plan.at(x, y) || y >= h - 4 {
                    ([70, 70, 76], false)
                } else {
                    ([10, 10, 14], true)
                }
            })
            .collect();
        (base, 0, h - 1)
    } else {
        let plan = Plan::draw(layout, maze.forks);
        // Crop to the rows the maze uses, plus a margin.
        let rows: Vec<i32> = (0..h)
            .filter(|&y| (0..w).any(|x| plan.kind[(y * w + x) as usize] != Kind::Rock))
            .collect();
        let (y0, y1) = ((rows[0] - 3).max(0), (rows[rows.len() - 1] + 3).min(h - 1));
        let (bed, _) = maze.world(layout);
        let (right, flat, astray) = pixel_physics::train::challenges::maze::readability(&bed, &plan);
        println!("trail as laid: {right:.3} of way cells point uphill towards the goal, {flat:.3} have no stronger neighbour, {astray:.3} of cells at a dead end's mouth point into it");
        let base = plan
            .kind
            .iter()
            .map(|k| match k {
                Kind::Rock => ([70, 70, 76], false),
                Kind::Dead => ([40, 8, 8], true),
                Kind::Way => ([10, 10, 14], true),
            })
            .collect();
        (base, y0, y1)
    };
    let goal = if open {
        vec![openmaze::Plan::draw(layout).goal]
    } else {
        let g = Plan::draw(layout, maze.forks).goal;
        (0..4).map(|gx| (g.0 - gx, g.1 + 1)).collect()
    };
    let band_h = (y1 - y0 + 1) as u32;
    if genomes.is_empty() {
        return;
    }
    let names: Vec<&str> = genomes.split(',').collect();
    let mut bands: Vec<Vec<Vec<u8>>> = Vec::new();
    for name in &names {
        let mut g = train::start_mind(&maze, name);
        if blind {
            g = mind::blinded(&g);
        }
        let mut shots: Vec<Vec<u8>> = Vec::new();
        let arena: &dyn Challenge = if open { &opened } else { &maze };
        let scores = arena.run(layout, &vec![&g[..]; maze.ants], &mut |f, world| {
            if f % every != 0 {
                return;
            }
            let mut img = vec![0u8; (w as u32 * band_h * 3) as usize];
            for y in y0..=y1 {
                for x in 0..w {
                    let i = (((y - y0) * w + x) * 3) as usize;
                    let (mut c, scented) = base[(y * w + x) as usize];
                    if scented {
                        let b = world.pheromone_at(Channel::B, x, y) as f32 / pheromone::Scent::MAX as f32;
                        if b > 0.0 {
                            c = [10, (60.0 + 195.0 * b.min(1.0)) as u8, 30];
                        }
                    }
                    img[i..i + 3].copy_from_slice(&c);
                }
            }
            for &(x, y) in &goal {
                if (y0..=y1).contains(&y) {
                    let i = (((y - y0) * w + x) * 3) as usize;
                    img[i..i + 3].copy_from_slice(&[255, 140, 0]);
                }
            }
            for id in world.live_organism_ids() {
                let Some(st) = world.organism(id) else { continue };
                for (n, &(x, y)) in st.chain.iter().enumerate() {
                    if (y0..=y1).contains(&y) && (0..w).contains(&x) {
                        let i = (((y - y0) * w + x) * 3) as usize;
                        img[i..i + 3].copy_from_slice(if n == 0 { &[255, 230, 0] } else { &[255, 255, 255] });
                    }
                }
            }
            shots.push(img);
        });
        let fit: f64 = scores.iter().map(|s| s.fit).sum::<f64>() / scores.len() as f64;
        let reached = scores.iter().filter(|s| s.parts[3] > 0.0).count();
        let lost: f64 = scores.iter().map(|s| s.parts[4]).sum::<f64>() / scores.len() as f64;
        if open {
            println!(
                "band {name}: along {fit:.3}, {reached} of {} ants reached the goal, {lost:.2} of the run on the path",
                scores.len()
            );
        } else {
            println!(
                "band {name}: along {fit:.3}, {reached} of {} ants reached the goal, {lost:.2} of the run in dead ends",
                scores.len()
            );
        }
        bands.push(shots);
    }
    // Stack the bands, with a white rule between, scaled up.
    let n = bands.iter().map(|b| b.len()).min().unwrap_or(0);
    let total_h = (band_h + 2) * bands.len() as u32;
    let (ow, oh) = (w as u32 * scale, total_h * scale);
    let file = std::fs::File::create(&out).expect("create gif");
    let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
    for s in 0..n {
        let mut frame = image::RgbaImage::new(ow, oh);
        for (b, shots) in bands.iter().enumerate() {
            let top = b as u32 * (band_h + 2);
            for y in 0..band_h + 2 {
                for x in 0..w as u32 {
                    let c = if y >= band_h {
                        [255, 255, 255]
                    } else {
                        let i = ((y * w as u32 + x) * 3) as usize;
                        [shots[s][i], shots[s][i + 1], shots[s][i + 2]]
                    };
                    for dy in 0..scale {
                        for dx in 0..scale {
                            frame.put_pixel(
                                x * scale + dx,
                                (top + y) * scale + dy,
                                image::Rgba([c[0], c[1], c[2], 255]),
                            );
                        }
                    }
                }
            }
        }
        enc.encode_frame(image::Frame::from_parts(
            frame,
            0,
            0,
            image::Delay::from_numer_denom_ms(60, 1),
        ))
        .expect("frame");
    }
    println!("{n} frames in {out}");
}
