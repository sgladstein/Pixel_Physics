//! **Does a food trail build up between a nest and a pile it can never
//! empty?** The owner's playtest, 2026-10-04: *"pheromone b fades way too
//! fast ... I don't see anything ever building up even though they're going
//! back and forth on this trail"*, on a colony placed beside a big pile of
//! food from the ADD brush.
//!
//! This rebuilds that box -- the shipped `LabBox`, one colony, and a disc of
//! `provisions` (the ADD brush's own material, which never rots) painted on
//! the ground `gap=` cells from the nest -- and reads both trail planes along
//! the ground between them every `every=` frames:
//!
//! - **The profile**: for each column between nest and pile, the brightest A
//!   and B cell in the band from 12 rows above the ground to 4 below it (a
//!   trail is laid where the ants walk, which is the surface and the first
//!   rows of tunnel). Printed in `bins=` distance bins, nest first, in the
//!   plane's own 0-255 units, so a gradient reads left to right.
//! - **Build-up**: B's summed mass and lit cells in the corridor, beside how
//!   many ants are in the corridor carrying food (the ones that lay B) -- the
//!   "it fired" and the traffic that should hold it up.
//!
//! `brho=` and `bdiffuse=` set channel B's decay and blend for the run
//! (`Pheromones::set_channel_rho` / `set_channel_diffuse`), unset meaning the
//! shipped values, so a slower-fading B can be shown on the same box.
//!
//! `shot=PATH` also saves the lab's own screen at the last frame, as the
//! player sees it: the trail overlay (`overlay=a|b`, default `b`) and the
//! scent strip with the pointer on the ground halfway along the trail.
//! The B switch the game reads (`PIXEL_PHYSICS_B_RHO` /
//! `PIXEL_PHYSICS_B_DIFFUSE`) applies here too; the header line prints the
//! rates actually in force, whichever way they were set.
//!
//! ```text
//! cargo run --release --example trailprofile -- seed=1 gap=40
//! cargo run --release --example trailprofile -- seed=1 gap=40 bdiffuse=0.05
//! cargo run --release --example trailprofile -- seed=1 shot=trail.png
//! ```
use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::pheromone::{Channel, SCALE as ONE};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}="))
            .map(|v| v.parse().ok().expect("parses"))
    })
}

fn main() {
    let seed: u64 = arg("seed").unwrap_or(1);
    let gap: i32 = arg("gap").unwrap_or(40);
    let radius: i32 = arg("pile").unwrap_or(8);
    let frames: u64 = arg("frames").unwrap_or(30_000);
    let every: u64 = arg("every").unwrap_or(3_000);
    let bins: i32 = arg("bins").unwrap_or(8);
    let brho: Option<f32> = arg("brho");
    let bdiffuse: Option<f32> = arg("bdiffuse");
    let shot: Option<String> = arg("shot");
    let overlay: String = arg("overlay").unwrap_or_else(|| "b".to_string());
    let spec = LabBox {
        seed,
        ..LabBox::default()
    };
    let ground = spec.ground_y;
    let width = spec.width;
    let mut lab = Lab::new(spec);
    if let Some(r) = brho {
        lab.world.pheromones.set_channel_rho(Channel::B, r);
    }
    if let Some(d) = bdiffuse {
        lab.world.pheromones.set_channel_diffuse(Channel::B, d);
    }
    let site = *lab.world.nest_sites.first().expect("the default box founds a colony");
    // Toward the middle of the box, so the pile is never against a wall.
    let dir = if site.x < width / 2 { 1 } else { -1 };
    let food_x = site.x + dir * gap;
    let provisions = lab.world.materials.id_of("provisions").expect("provisions");
    lab.world.paint_capsule_as(
        (food_x, ground - 1 - radius),
        (food_x, ground - 1 - radius),
        radius,
        provisions,
        1.0,
    );
    let (b_fade, b_spread) = lab
        .world
        .pheromones
        .channel_rates(Channel::B)
        .expect("B is built with the world");
    println!(
        "trailprofile: seed={seed} gap={gap} pile r={radius} nest x={} food x={food_x} B fade {b_fade:.4} spread {b_spread:.3} per pass (units: 0-255 per cell)",
        site.x,
    );
    let cols: Vec<i32> = (0..=gap).map(|d| site.x + dir * d).collect();
    let band = |x: i32, ch: Channel, world: &pixel_physics::sim::world::World| {
        (ground - 12..=ground + 4)
            .map(|y| world.pheromone_at(ch, x, y))
            .max()
            .unwrap_or(0) as f32
            / ONE as f32
    };
    let bin_of = |d: usize| ((d as i32 * bins) / (gap + 1)) as usize;
    while lab.world.frame < frames {
        lab.tick_for_harness();
        if !lab.world.frame.is_multiple_of(every) {
            continue;
        }
        let w = &lab.world;
        let mut a = vec![0f32; bins as usize];
        let mut b = vec![0f32; bins as usize];
        let mut n = vec![0f32; bins as usize];
        let (mut mass, mut lit) = (0f32, 0);
        for (d, &x) in cols.iter().enumerate() {
            let (va, vb) = (band(x, Channel::A, w), band(x, Channel::B, w));
            a[bin_of(d)] += va;
            b[bin_of(d)] += vb;
            n[bin_of(d)] += 1.0;
            for y in ground - 12..=ground + 4 {
                let v = w.pheromone_at(Channel::B, x, y);
                mass += v as f32 / ONE as f32;
                lit += (v > 0) as u32;
            }
        }
        let (lo, hi) = (site.x.min(food_x), site.x.max(food_x));
        let (mut ants, mut laden) = (0, 0);
        for id in w.live_organism_ids() {
            let Some(st) = w.organism(id) else { continue };
            if w.species.get(st.species).name != "ant" {
                continue;
            }
            let Some(&(hx, hy)) = st.chain.first() else { continue };
            if hx < lo || hx > hi || hy < ground - 12 || hy > ground + 4 {
                continue;
            }
            ants += 1;
            laden += st.crop.is_some_and(|c| c.worth() > 0.0) as u32;
        }
        let fmt = |v: &[f32]| {
            v.iter()
                .zip(&n)
                .map(|(s, k)| format!("{:6.1}", s / k.max(1.0)))
                .collect::<Vec<_>>()
                .join("")
        };
        println!("frame {:>6}  A nest->food {}", w.frame, fmt(&a));
        println!(
            "               B nest->food {}   B mass {mass:7.1} lit {lit:4}  ants in corridor {ants:3} laden {laden:3}",
            fmt(&b)
        );
    }
    if let Some(path) = shot {
        use pixel_physics::render::FieldOverlay;
        lab.renderer.field_overlay = if overlay == "a" {
            FieldOverlay::PheromoneA
        } else {
            FieldOverlay::PheromoneB
        };
        lab.show_help = false;
        // The biosphere page opens by default and covers the strip's right
        // half; a player watching a trail would have it shut.
        lab.act(pixel_physics::lab::ui::Action::Stats);
        let (w, h) = lab.viewport();
        // The screen point over the ground halfway along the trail, found by
        // asking the renderer rather than assuming its camera.
        let target = (site.x + dir * gap / 2, ground - 1);
        let at = (0..h as i32)
            .flat_map(|ly| (0..w as i32).map(move |lx| (lx, ly)))
            .find(|&(lx, ly)| lab.renderer.logical_to_world(lx, ly) == target);
        lab.set_cursor(at);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        lab.draw(&mut buf, 60.0);
        let img = image::RgbaImage::from_raw(w, h, buf).expect("buffer");
        image::imageops::resize(&img, w * 2, h * 2, image::imageops::FilterType::Nearest)
            .save(&path)
            .expect("save");
        println!("wrote {path} ({w}x{h} at 2x), pointer at {at:?} over world {target:?}");
    }
}
