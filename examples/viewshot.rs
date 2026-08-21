//! What the *player's viewport* shows of a world larger than itself.
//!
//! `filmstrip` renders whole small worlds, which was the same picture while
//! the world was exactly one screen. It no longer is: the world ships four
//! screens wide and twice as deep, and almost everything that can go wrong
//! with that is invisible in a whole-world render —
//!
//! - the camera clamped so hard at an edge that half the screen is outside
//!   the world,
//! - the sky's horizon band or its moon placed against *world* bounds, so it
//!   lands off-screen and is never seen,
//! - a world whose surface has drifted below the initial view, so a fresh
//!   start shows nothing but sky.
//!
//! So this renders viewport-sized frames at camera positions across the
//! world, through the same `Renderer` the app uses, and lays them out as a
//! contact sheet. One row is one traverse.
//!
//! ```text
//! cargo run --release --example viewshot
//! cargo run --release --example viewshot -- seed=7 preset=arid shots=6
//! cargo run --release --example viewshot -- frame=1800   # night, for stars and moon
//! cargo run --release --example viewshot -- vault=1 crop=180,90,160,140 zoom=4
//! ```
//!
//! Written for the world-size step, kept for weather: a rain or snow sky is
//! judged from the viewport too, and drawn precipitation is position-hashed
//! against *world* coordinates, so it must not slide as the camera pans.

use pixel_physics::app::{HEIGHT, WIDTH, WORLD_HEIGHT, WORLD_WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;
use pixel_physics::worldgen::WorldgenParams;

struct Args {
    seed: u32,
    preset: String,
    shots: usize,
    frame: usize,
    settle: usize,
    rain: String,
    mine: bool,
    vault: bool,
    boulder: bool,
    reveal: bool,
    light: pixel_physics::render::TerrainLight,
    spring: i32,
    age: Option<f32>,
    zoom: usize,
    crop: Option<(usize, usize, usize, usize)>,
    out: String,
}

fn main() {
    let mut a = Args {
        seed: 1,
        preset: String::new(),
        shots: 4,
        // Mid-morning by default: the sky is lit, which is the harder case
        // for judging ground colour. `frame=1800` is the other half of the
        // cycle and is where stars and the moon are.
        frame: 600,
        settle: 60,
        rain: String::new(),
        mine: false,
        vault: false,
        boulder: false,
        reveal: false,
        light: pixel_physics::render::TerrainLight::default(),
        spring: 0,
        age: None,
        zoom: 1,
        crop: None,
        out: "target/filmstrips/viewshot.png".to_string(),
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "seed" => a.seed = v.parse().expect("seed=N"),
            "preset" => a.preset = v.to_string(),
            "shots" => a.shots = v.parse::<usize>().expect("shots=N").max(1),
            "frame" => a.frame = v.parse().expect("frame=N"),
            "settle" => a.settle = v.parse().expect("settle=N"),
            // `rain=1` jumps the world clock to a frame this seed is actually
            // raining hard at, rather than making the reader guess one. Most
            // frames are clear by design, so without this a rain render is
            // mostly a render of no rain.
            "rain" => a.rain = v.to_string(),
            // `mine=1` cuts narrow shafts down into the terrain after it has
            // settled, which is the reproduction for "the sky follows the
            // pick down a hole". Shafts of three different widths, because
            // the failure was reported for a *narrow* one and a fix that only
            // worked for narrow ones would be worth knowing about.
            "mine" => a.mine = v != "0",
            // `vault=1` aims the camera at a sealed chamber and sinks a shaft
            // into it, which is the only way to photograph the round-2 vault
            // pass at all: a vault sits 200+ rows below the surface, so every
            // other view in this harness -- all of which frame the skyline --
            // is guaranteed to miss it, and `filmstrip`'s worldgen scene
            // builds at 512x320 where the depth band does not even exist.
            //
            // The chamber is *found* rather than passed in, because its
            // position is a noise draw and a hardcoded coordinate would go
            // stale the moment anything upstream of `Purpose::Vault` changes.
            "vault" => a.vault = v != "0",
            // `boulder=1` aims at a seated erosion boulder, which is the
            // only way to photograph one: they are 2-5 cells wide, they
            // seat in roughly one world in eight (measured at the shipped
            // size, canyon), and a strip framed anywhere else is a strip of
            // hillside. Found rather than passed in, for the same reason
            // `vault` finds its chamber -- the position is a draw and a
            // hardcoded coordinate goes stale.
            "boulder" => a.boulder = v != "0",
            // `reveal=1` turns on the F11 void X-ray, so a strip can show
            // where every sealed chamber and cavity sits without digging.
            "reveal" => a.reveal = v != "0",
            // `light=flat` renders the pre-review look, for A/B strips of
            // the terrain depth light (`F10` in the app).
            "light" => {
                a.light = match v {
                    "depth" => pixel_physics::render::TerrainLight::Depth,
                    "flat" | "off" => pixel_physics::render::TerrainLight::Off,
                    other => panic!("unknown light {other:?} (depth|flat)"),
                }
            }
            // `spring=N` installs one N-column spring at the tallest cliff
            // rim and matching drains at the lowest basin — the shipped
            // mechanics from `sim/spring.rs`, placed by the same scan
            // worldgen's placement pass will refine later. `spring=1` is a
            // seep, `spring=4` a waterfall. The ledger prints beside the
            // image so "did it fire" is a number, not a reading of pixels.
            "spring" => a.spring = v.parse().expect("spring=N columns"),
            // `age=N` overrides the preset's `world_age` for one render.
            // The erosion design promises hoodoos and spires as a side
            // effect of the differential rates; the prominence table says
            // the shipped ages produce none. Whether *any* age produces
            // them is the question that separates "tune the rates" from
            // "the mechanism is missing", and it is answerable without
            // touching `erosion.rs` -- which the data track owns.
            "age" => a.age = Some(v.parse().expect("age=N")),
            // `zoom=K` and `crop=x,y,w,h` exist because a cave is judged at
            // the scale its formations are *drawn* at, and a 512x320 viewport
            // tile reduced onto a contact sheet is not that scale: a
            // stalactite is four pixels tall there, so "a comb of teeth" and
            // "a forest of formations" are the same picture. `crop` is in
            // viewport coordinates (the same frame the shot draws), applied
            // per shot before the tiles are laid side by side; `zoom` is a
            // nearest-neighbour magnify, never a filter -- a smoothed pixel
            // grid would invent detail that is not in the world.
            "zoom" => a.zoom = v.parse::<usize>().expect("zoom=K").max(1),
            "crop" => {
                let n: Vec<usize> = v.split(',').map(|t| t.parse().expect("crop=x,y,w,h")).collect();
                assert_eq!(n.len(), 4, "crop=x,y,w,h");
                a.crop = Some((n[0], n[1], n[2], n[3]));
            }
            "out" => a.out = v.to_string(),
            _ => panic!("unknown argument {arg:?}"),
        }
    }

    let bounds = Rect::new(0, 0, WORLD_WIDTH as i32 - 1, WORLD_HEIGHT as i32 - 1);
    let mut world = World::new(bounds);
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    let name = if a.preset.is_empty() { presets.default_name() } else { a.preset.clone() };
    let Some(params) = presets.get(&name) else { panic!("unknown preset {name:?}") };
    let aged;
    let params = match a.age {
        Some(age) => {
            aged = WorldgenParams { world_age: age, ..params.clone() };
            &aged
        }
        None => params,
    };
    let build = std::time::Instant::now();
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed: a.seed as u64 });
    let build_ms = build.elapsed().as_secs_f64() * 1000.0;

    // `rain=wet` / `rain=dry` pick a frame that is *both* the weather asked
    // for and the same time of day, so the two renders differ by the weather
    // and by nothing else. Without the second half the comparison is
    // worthless: the first attempt at this landed the wet render at midnight
    // and produced a picture that was darker for entirely the wrong reason.
    //
    // The frame chosen is the one the settle loop will *end* on, not the one
    // it starts from, because that is the frame actually rendered.
    if !a.rain.is_empty() {
        use pixel_physics::sim::{field, weather};
        let day = field::DAY_NIGHT_PERIOD_FRAMES;
        let want_wet = a.rain != "dry";
        let want_kind = match a.rain.as_str() {
            "snow" => Some(weather::Precipitation::Snow),
            // `rain=bolt` finds a frame where a strike is actually lit,
            // which is a few frames in every few hundred -- a render aimed
            // at "a storm" would essentially never contain one.
            "bolt" => Some(weather::Precipitation::Rain),
            "wet" => Some(weather::Precipitation::Rain),
            _ => None,
        };
        let end = |start: u64| start + a.settle as u64;
        let noonish = |f: u64| field::sun_elevation(end(f)) > 0.85;
        let chosen = (0..weather::WEATHER_EPOCH_FRAMES * 40)
            .step_by(if a.rain == "bolt" { 1 } else { 30 })
            .filter(|&f| noonish(f))
            .filter(|&f| weather::at(world.seed, end(f)).is_precipitating() == want_wet)
            .filter(|&f| want_kind.is_none_or(|k| weather::at(world.seed, end(f)).kind == k))
            .filter(|&f| a.rain != "bolt" || weather::strike(world.seed, end(f), world.bounds()).is_some_and(|s| s.age <= 2))
            .max_by(|&x, &y| {
                let (a, b) = (weather::at(world.seed, end(x)).intensity, weather::at(world.seed, end(y)).intensity);
                if want_wet { a.total_cmp(&b) } else { b.total_cmp(&a) }
            })
            .unwrap_or_else(|| panic!("no near-noon {} frame found for seed {}", a.rain, world.seed));
        world.frame = chosen;
        let w = weather::at(world.seed, end(chosen));
        println!(
            "frame {chosen} -> renders at {}: {:?} intensity {:.2}, wind {:+.2}, sun {:+.2} (day {})",
            end(chosen),
            w.kind,
            w.intensity,
            w.wind,
            field::sun_elevation(end(chosen)),
            end(chosen) / day
        );
    }

    if a.spring > 0 {
        // The tallest single-sided drop in the world, measured over a
        // 12-column window on the raw surface — where an aquifer would
        // daylight. Worldgen's placement pass replaces this scan with the
        // planned water table; the registration and mechanics are the
        // shipped ones either way.
        let surface = |x: i32| -> i32 {
            (0..WORLD_HEIGHT as i32)
                .find(|&y| {
                    matches!(
                        world.materials.kind(world.get(x, y).material),
                        pixel_physics::sim::material::MaterialKind::Solid | pixel_physics::sim::material::MaterialKind::Powder
                    )
                })
                .unwrap_or(WORLD_HEIGHT as i32 - 1)
        };
        let heights: Vec<i32> = (0..WORLD_WIDTH as i32).map(surface).collect();
        let mut best = (0i32, 1i32, 0i32); // (rim_x, dir, drop)
        for x in 0..WORLD_WIDTH as i32 {
            for dir in [1i32, -1] {
                let mut deepest = 0;
                for d in 1..=12 {
                    let nx = (x + dir * d).clamp(0, WORLD_WIDTH as i32 - 1);
                    deepest = deepest.max(heights[nx as usize] - heights[x as usize]);
                }
                if deepest > best.2 {
                    best = (x, dir, deepest);
                }
            }
        }
        let (rim, dir, drop) = best;
        let span = a.spring.min(pixel_physics::sim::spring::MAX_SPAN);
        // The span hangs off the rim on the falling side, so the whole
        // sheet drops down the face rather than half of it landing on the
        // bench behind the edge.
        let x0 = if dir > 0 { rim + 1 } else { rim - span };
        let outlet = (x0.clamp(0, WORLD_WIDTH as i32 - span), heights[rim as usize]);
        assert!(world.add_spring(outlet.0, outlet.1, span));
        // Drains match the flow, one per emission column, across the
        // world's lowest floor — the basin the fall ultimately feeds.
        let low = (0..WORLD_WIDTH as i32).max_by_key(|&x| heights[x as usize]).unwrap_or(0);
        for d in 0..span {
            let dx = (low + d - span / 2).clamp(0, WORLD_WIDTH as i32 - 1);
            world.add_drain(dx, heights[dx as usize] - 1);
        }
        println!("spring span {span} at ({}, {}) over a {drop}-cell drop; {span} drains around x={low}", outlet.0, outlet.1);
    }

    // Let it settle before looking. Generated terrain is meant to be at rest
    // by construction, so anything still moving here is worth seeing in the
    // image rather than hidden by rendering frame zero.
    // Timed over the tail of the settle — the steady state, where costs
    // hide (D8's lesson). Two runs, `spring=N` against `spring=0`, give the
    // standing bill of a fall as a paired same-session comparison.
    let timed_tail = 300.min(a.settle);
    let mut tail_worst = 0.0f64;
    let mut tail_sum = 0.0f64;
    for i in 0..a.settle {
        let t = std::time::Instant::now();
        pixel_physics::sim::parallel::step(&mut world);
        world.step_active_sites();
        // The field too, which this loop used to leave out. Harmless while
        // everything judged here was drawn (rain, the sky, the camera) and
        // not harmless once anything *reads* a field channel: evaporation
        // rates off the humidity above a water surface, and with no field
        // step that humidity is zero everywhere, so a long settle would dry
        // every lake in the world and the picture would be of a bug in the
        // harness. Matches `App::update`'s own phase order.
        world.step_fields();
        if i + timed_tail >= a.settle {
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            tail_worst = tail_worst.max(ms);
            tail_sum += ms;
        }
    }
    if timed_tail > 0 {
        println!(
            "settle tail ({timed_tail} frames): mean {:.3} ms, worst {:.3} ms",
            tail_sum / timed_tail as f64,
            tail_worst
        );
    }

    let mut renderer = Renderer::new();
    renderer.terrain_light = a.light;
    renderer.reveal_voids = a.reveal;
    let particles = ParticleSystem::new();
    let (vw, vh) = (WIDTH as usize, HEIGHT as usize);
    // The tile written into the sheet: the crop window magnified, or the
    // whole viewport at 1x when neither was asked for.
    let (cx0, cy0, cw, ch) = a.crop.unwrap_or((0, 0, vw, vh));
    assert!(cx0 + cw <= vw && cy0 + ch <= vh, "crop {cx0},{cy0},{cw},{ch} is outside the {vw}x{vh} viewport");
    let (tw, th) = (cw * a.zoom, ch * a.zoom);
    let mut sheet = vec![0u8; tw * th * a.shots * 4];
    let mut frame = vec![0u8; vw * vh * 4];

    // Where a system is, if one was asked for. Round 3 made the subject a
    // cave system rather than a single chamber, so the finder aims at the
    // *tallest column of deep air* -- the largest chamber of whichever
    // system carved most. Air this deep can only be carved void (nothing
    // else makes air under the massif at genesis), which also stops the old
    // gravel search from landing the camera on a pocket lens instead.
    // A seated boulder: a *stone* cell at the very top of its column with
    // soil-covered ground either side of it, standing proud of the local
    // trend. That is what the pass writes -- a dome of stone displacing the
    // loose cover -- and it is what the eye picks out. The camera is aimed
    // at the best bump rather than the first, and what was found is printed,
    // so a finder that latched onto an outcrop instead is visible in the log
    // rather than silently mis-framing the picture.
    let boulder_at = if a.boulder {
        // **Ask the generator where the boulders are, rather than inferring
        // it from the picture.** Two heuristic finders were written before
        // this one and both found the wrong object: "the most prominent
        // stone bump" found an ordinary sandstone outcrop, and "a 2-6 column
        // run of cap-rock at the surface" found 6-16 candidates per world
        // (cap-rock *beds* outcrop in the same family) where only one or two
        // boulders exist. Every judgement made from those renders was a
        // judgement of something else -- CLAUDE.md's "check the scene still
        // contains the situation you think it does", arriving in the harness
        // instead of in the world.
        //
        // `erosion::Deposits::boulder` is the marker array the pass itself
        // reads, and `Terrain::plan_all_with_deposits` is public, so the
        // answer is exact and costs one extra plan (no cell pass). The run
        // merge below mirrors `passes::boulders` -- if that pass changes how
        // it merges, this goes with it.
        use pixel_physics::worldgen::column::Terrain;
        let soil = world.materials.id_of("soil").expect("soil is compiled in");
        let sand = world.materials.id_of("sand").expect("sand is compiled in");
        let terrain = Terrain::new(
            a.seed as u64,
            params,
            WORLD_WIDTH as i32,
            WORLD_HEIGHT as i32,
            world.materials.get(soil).friction_angle.to_radians().tan(),
            world.materials.get(sand).friction_angle.to_radians().tan(),
        );
        let (plans, deposits) = terrain.plan_all_with_deposits();
        let mut centres: Vec<i32> = Vec::new();
        let mut x = 0i32;
        while x < WORLD_WIDTH as i32 {
            if !deposits.boulder[x as usize] {
                x += 1;
                continue;
            }
            let start = x;
            while x < WORLD_WIDTH as i32 && deposits.boulder[x as usize] {
                x += 1;
            }
            centres.push((start + x - 1) / 2);
        }
        let top = |x: i32| -> i32 {
            (0..WORLD_HEIGHT as i32)
                .find(|&y| world.get(x, y).material != material::EMPTY)
                .unwrap_or(WORLD_HEIGHT as i32 - 1)
        };
        let tops: Vec<i32> = (0..WORLD_WIDTH as i32).map(top).collect();
        // Prominence over the whole world, so the boulder's own number can
        // be read against what an ordinary hillside does. A metric with no
        // null case is a number nobody can interpret.
        let prom = |x: i32| -> i32 {
            let l = tops[(x - 5).max(0) as usize];
            let r = tops[(x + 5).min(WORLD_WIDTH as i32 - 1) as usize];
            (l - tops[x as usize]).min(r - tops[x as usize])
        };
        let mut all: Vec<i32> = (5..WORLD_WIDTH as i32 - 5).map(prom).collect();
        all.sort_unstable();
        let q = |f: f32| all[((all.len() as f32 - 1.0) * f) as usize];
        // **Prominence at several reaches, because the reach is a scale and
        // a single one cannot see past it.** Measured at 5 columns only, a
        // 40-cell hoodoo 12 columns wide scores *zero* -- both sample points
        // land on top of the formation itself -- so "max prominence in the
        // world is 2" would have read as "there are no standing residuals
        // anywhere" when it really meant "none narrower than 10 columns".
        // The erosion design promises hoodoos as a side effect of the
        // differential rates; whether it delivers them is exactly this
        // question, and it is unanswerable at one reach.
        for reach in [5i32, 15, 30, 60] {
            let pr = |x: i32| -> i32 {
                let l = tops[(x - reach).max(0) as usize];
                let r = tops[(x + reach).min(WORLD_WIDTH as i32 - 1) as usize];
                (l - tops[x as usize]).min(r - tops[x as usize])
            };
            let mut v: Vec<i32> = (reach..WORLD_WIDTH as i32 - reach).map(pr).collect();
            v.sort_unstable();
            let qq = |f: f32| v[((v.len() as f32 - 1.0) * f) as usize];
            println!(
                "  prominence at reach {reach:>2}: med {:>3} p90 {:>3} p99 {:>3} max {:>3}",
                qq(0.5),
                qq(0.9),
                qq(0.99),
                v[v.len() - 1]
            );
        }
        // Headroom: how much sky a formation could rise into, and how much
        // relief the terrain already has, so a proposed size can be judged
        // against the world rather than against a wish.
        let hi = *tops.iter().min().expect("non-empty");
        let lo = *tops.iter().max().expect("non-empty");
        println!(
            "  surface runs y {hi}..{lo} ({} cells of relief); sky above the highest ground: {hi} rows",
            lo - hi
        );
        println!(
            "  surface prominence at reach 5: med {} p90 {} p99 {} max {}",
            q(0.5),
            q(0.9),
            q(0.99),
            all[all.len() - 1]
        );
        // Seated or not: the pass writes cap-rock-family stone above the
        // plan surface, so a marker whose crown is not that has been
        // rejected (round-4 finding R4-1: a `brows` lip usually got there
        // first). Reported per marker, because "the pass fired" and "you can
        // see it" are different questions and the counter answers only one.
        let seated: Vec<i32> = centres
            .iter()
            .copied()
            .filter(|&cx| {
                let ground = plans[cx as usize].surface_y;
                (1..=4).any(|row| {
                    let c = world.get(cx, ground - row);
                    Some(c.material) == world.materials.id_of("stone") && c.shade / 4 == 3
                })
            })
            .collect();
        println!(
            "  boulder markers: {} runs; seated: {}",
            centres.len(),
            seated.len()
        );
        match seated.first().copied() {
            Some(cx) => {
                let y = tops[cx as usize];
                let p = prom(cx);
                let pct = 100.0 * all.iter().filter(|&&v| v < p).count() as f32 / all.len() as f32;
                let height = (1..=6)
                    .take_while(|&row| {
                        let c = world.get(cx, plans[cx as usize].surface_y - row);
                        Some(c.material) == world.materials.id_of("stone") && c.shade / 4 == 3
                    })
                    .count();
                println!(
                    "  boulder at ({cx}, {y}): {height} cells tall, stands {p} proud -- \
                     the {pct:.0}th percentile of ordinary hillside (player is 14 tall)"
                );
                Some((cx, y))
            }
            None => {
                println!("  NO SEATED BOULDER in this world -- try another seed");
                None
            }
        }
    } else {
        None
    };

    let vault_at = if a.vault {
        let deep = WORLD_HEIGHT as i32 / 2;
        let mut found: Option<(i32, i32)> = None;
        let mut best = 0;
        let mut air = 0usize;
        for x in 0..WORLD_WIDTH as i32 {
            let mut run = 0;
            for y in deep..WORLD_HEIGHT as i32 {
                if world.get(x, y).material == material::EMPTY {
                    run += 1;
                    air += 1;
                    if run > best {
                        best = run;
                        found = Some((x, y - run / 2));
                    }
                } else {
                    run = 0;
                }
            }
        }
        match found {
            Some((x, y)) => println!("  vault found at ({x}, {y}): tallest chamber {best} rows, {air} cells of deep air in the world"),
            None => println!("  NO VAULT in this world -- try another seed"),
        }
        found
    } else {
        None
    };

    // Camera targets spread across the world's width at the height of the
    // ground, so each shot frames a different region rather than a different
    // patch of sky. Reported next to the image, because a contact sheet
    // cannot show *where* it was taken and four pictures of the same hill
    // look exactly like four different hills.
    println!("world {}x{} ({name}, seed {}), built in {build_ms:.0} ms", WORLD_WIDTH, WORLD_HEIGHT, a.seed);
    for shot in 0..a.shots {
        // Aimed at the strike when there is one: a bolt lands anywhere in a
        // world four screens wide, so a shot framed anywhere else is a render
        // of a flash with the interesting part outside it.
        let aimed = pixel_physics::sim::weather::strike(world.seed, world.frame, world.bounds()).map(|s| s.x);
        let x = match (a.vault || a.boulder, a.mine, aimed) {
            (true, _, _) => vault_at
                .or(boulder_at)
                .map(|(vx, _)| vx)
                .unwrap_or(WORLD_WIDTH as i32 / 2),
            (_, true, _) => WORLD_WIDTH as i32 / 4,
            (_, _, Some(sx)) => sx,
            _ => ((shot as f32 + 0.5) / a.shots as f32 * WORLD_WIDTH as f32) as i32,
        };
        // Normally the camera is aimed at the skyline, which is the right
        // target for every other scene here and exactly wrong for a vault:
        // the whole feature is below the bottom of that frame.
        let ground = match (a.vault || a.boulder, vault_at.or(boulder_at)) {
            (true, Some((_, vy))) => vy,
            _ => (0..WORLD_HEIGHT as i32)
                .find(|&y| world.get(x, y).material != material::EMPTY)
                .unwrap_or(WORLD_HEIGHT as i32 / 2),
        };
        renderer.follow((x, ground), (WIDTH, HEIGHT), world.bounds());
        let (cam_x, cam_y) = (renderer.camera_x, renderer.camera_y);
        // Clamped hard against an edge is legitimate at the ends of the
        // world and a bug in the middle, so print the camera rather than
        // asserting: the reader can see which case this is.
        println!(
            "  shot {shot}: target ({x}, {ground}) -> camera ({cam_x}, {cam_y}), \
             showing world x {cam_x}..{}",
            cam_x + WIDTH as i32
        );

        // The frame buffer persists across shots and only the *first* draw
        // is forced full. That is deliberate and is the point of this
        // harness: every later shot has to be repainted by the camera move
        // alone. Without `last_camera` feeding the renderer's `full` flag,
        // the dirty-rect skip leaves the previous shot's pixels in place and
        // the sheet shows the same view four times over -- a failure that is
        // invisible in any single image, and the one thing most likely to go
        // wrong when a viewport starts moving.
        let touched = world.take_touched_chunks();
        renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH, HEIGHT), shot == 0);

        // Mined *between* draws, which is the sequence a player performs and
        // the only one that reproduces the bug. Doing it before the first
        // draw does not: the renderer's opening scan of the skyline would see
        // the shaft already there and record its floor as the true ground, so
        // the sky comes down the hole and the fix appears not to work. The
        // reproduction has to contain the order of events, not just the
        // final state.
        // The found-a-secret moment: a shaft sunk from the surface all the
        // way to the chamber, so the strip shows the breach rather than a
        // sealed room nobody could have reached. Cut on the *second* shot so
        // the sheet carries the before and the after side by side.
        if a.vault && shot == 1 {
            if let Some((vx, vy)) = vault_at {
                let top = (0..WORLD_HEIGHT as i32)
                    .find(|&y| world.get(vx, y).material != material::EMPTY)
                    .unwrap_or(0);
                for x in vx - 1..=vx + 1 {
                    for y in top..=vy {
                        world.set(x, y, pixel_physics::sim::cell::Cell::EMPTY);
                    }
                }
                println!("  mined a 3-wide shaft at x={vx} from y={top} down to the chamber at y={vy}");
                pixel_physics::sim::parallel::step(&mut world);
            }
        }
        if a.mine && shot == 0 {
            for (i, w) in [1i32, 3, 8].iter().enumerate() {
                let cx = cam_x + 140 + i as i32 * 90;
                let top = (0..WORLD_HEIGHT as i32)
                    .find(|&y| world.get(cx, y).material != material::EMPTY)
                    .unwrap_or(0);
                for x in cx - w / 2..=cx + w / 2 {
                    for y in top..top + 150 {
                        world.set(x, y, pixel_physics::sim::cell::Cell::EMPTY);
                    }
                }
                println!("  mined a {w}-wide shaft at x={cx}, 150 deep from y={top}");
            }
            // One step, so the chunks register as touched. `World::set` alone
            // does not populate `touched_chunks` -- that happens in
            // `end_step` -- so without this the next draw's dirty-rect skip
            // has nothing to repaint and the shafts are invisible for a
            // reason that has nothing to do with the skyline.
            pixel_physics::sim::parallel::step(&mut world);
        }

        for y in 0..th {
            for x in 0..tw {
                let src = ((cy0 + y / a.zoom) * vw + cx0 + x / a.zoom) * 4;
                let dst = (y * tw * a.shots + shot * tw + x) * 4;
                sheet[dst..dst + 4].copy_from_slice(&frame[src..src + 4]);
            }
        }
        // Step between shots so the day advances a little and the sky is not
        // bit-identical in every tile -- a sheet of identical skies hides a
        // sky that is not being redrawn at all.
        for _ in 0..(a.frame / a.shots.max(1)) {
            pixel_physics::sim::parallel::step(&mut world);
        }
    }

    let (sw, sh) = ((tw * a.shots) as u32, th as u32);
    if let Some(dir) = std::path::Path::new(&a.out).parent() {
        std::fs::create_dir_all(dir).expect("creating the output directory");
    }
    image::save_buffer(&a.out, &sheet, sw, sh, image::ColorType::Rgba8).expect("writing the sheet");
    println!("contact sheet ({sw}x{sh}, {} viewport shots): {}", a.shots, a.out);
    if a.spring > 0 {
        // The counter next to the picture. A fall on screen with emitted=0
        // would be pond water wandering into frame, not the spring working.
        let l = world.spring_ledger;
        println!("spring ledger: emitted {} drained {} throttled {} (fill units; {} cells emitted)", l.emitted, l.drained, l.throttled, l.emitted / pixel_physics::sim::material::LIQUID_FULL as u64);
    }
}
