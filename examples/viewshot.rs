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

struct Args {
    seed: u32,
    preset: String,
    shots: usize,
    frame: usize,
    settle: usize,
    rain: String,
    mine: bool,
    gif: bool,
    every: usize,
    pan: f32,
    zoom: i32,
    stride: i32,
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
        gif: false,
        every: 1,
        pan: 0.0,
        zoom: 1,
        stride: 1,
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
            // `pan=SECONDS` holds the scroll key for that long between shots,
            // through **`Renderer::pan` itself** rather than `follow`.
            //
            // Driving it with `follow` and captioning the sheet "panning"
            // would be exactly the failure `CLAUDE.md` records twice: four
            // camera positions look identical whichever function put the
            // camera there, so the picture would be evidence about nothing.
            //
            // Sixty small calls a second rather than one big one, because the
            // residual carry and the stride quantisation are per-call
            // behaviour and a single call with a large `seconds` steps clean
            // over both.
            "pan" => a.pan = v.parse().expect("pan=SECONDS_PER_SHOT"),
            // The scale the pan is judged at. Its whole claim is that the
            // picture slides at the same rate however far in or out you are,
            // and one row at one zoom cannot show that.
            "zoom" => a.zoom = v.parse().expect("zoom=N"),
            "stride" => a.stride = v.parse().expect("stride=N"),
            // `gif=1` writes one frame per simulated frame of the scroll
            // instead of a contact sheet. `CLAUDE.md`: reach for the
            // animation "when the question is whether something *moves*
            // right, which a grid of stills cannot answer" -- and whether a
            // scroll rate feels right is exactly that question. A sheet can
            // show that the camera reached six different places; only this
            // can show how it got there.
            "gif" => a.gif = v != "0",
            // Encode every Nth simulated frame. The scroll still *runs* at 60
            // Hz — only the sampling thins — and the gif's frame delay is set
            // from this, so a sampled animation plays back at the true rate
            // and only looks choppier.
            //
            // That trade is the right one here and cropping is not, which is
            // worth stating because cropping is what the review skill
            // recommends in general: apparent scroll speed is measured
            // *against the viewport*, so a half-width crop would read as twice
            // the speed — it would distort the exact quantity the card is
            // asking about. A scrolling world is also the worst case for
            // inter-frame compression, since no pixel holds still.
            "every" => a.every = v.parse::<usize>().expect("every=N").max(1),
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

    // Let it settle before looking. Generated terrain is meant to be at rest
    // by construction, so anything still moving here is worth seeing in the
    // image rather than hidden by rendering frame zero.
    for _ in 0..a.settle {
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
    }

    let mut renderer = Renderer::new();
    renderer.zoom = a.zoom;
    renderer.zoom_out_stride = a.stride;
    let particles = ParticleSystem::new();
    let (vw, vh) = (WIDTH as usize, HEIGHT as usize);
    let mut sheet = vec![0u8; vw * vh * a.shots * 4];
    let mut frame = vec![0u8; vw * vh * 4];

    // Camera targets spread across the world's width at the height of the
    // ground, so each shot frames a different region rather than a different
    // patch of sky. Reported next to the image, because a contact sheet
    // cannot show *where* it was taken and four pictures of the same hill
    // look exactly like four different hills.
    println!("world {}x{} ({name}, seed {}), built in {build_ms:.0} ms", WORLD_WIDTH, WORLD_HEIGHT, a.seed);

    // The animation branch: one encoded frame per simulated frame of a held
    // scroll key, driven through the same `Renderer::pan` the app calls at the
    // same 60 Hz. `shots` is the number of *seconds* of held key here rather
    // than a number of tiles.
    //
    // Separate from the contact-sheet loop below rather than folded into it,
    // because the two answer different questions and want different sampling:
    // a sheet wants a handful of far-apart positions, and this wants every
    // intermediate one. Reports the worst frame and the recomputed count for
    // the same reason the sheet does -- a scroll that looks smooth because the
    // dirty-rect skip froze it would look smooth in a gif too.
    if a.gif {
        let ground = (0..WORLD_HEIGHT as i32)
            .find(|&y| world.get(0, y).material != material::EMPTY)
            .unwrap_or(WORLD_HEIGHT as i32 / 2);
        renderer.set_camera(0, ground - HEIGHT as i32 / 2, (WIDTH, HEIGHT), world.bounds());
        let total = a.shots * 60;
        let mut frames = Vec::with_capacity(total);
        // Split by whether the camera moved this frame, because that split is
        // the whole cost question: a moving camera invalidates the buffer and
        // pays a full redraw, a pinned one keeps the dirty-rect skip. Measured
        // as a pair in one run, on one machine, in one scene -- `CLAUDE.md`
        // records a 25-50% "regression" that turned out to be a baseline
        // remembered from a different session on a machine that had since
        // slowed down.
        let (mut worst_moving, mut worst_pinned) = (0.0f64, 0.0f64);
        let mut skipped = 0usize;
        let start_x = renderer.camera_x;
        // Frames on which the camera actually moved, so the rate below is the
        // scroll's own speed rather than an average diluted by however long
        // the run sat pinned at the world's edge afterwards. The first
        // measurement of this read 0.75 screens/s for a scroll running at
        // exactly 1.5, purely because half the run had already arrived.
        let mut moving_frames = 0usize;
        for i in 0..total {
            let was = renderer.camera_x;
            renderer.pan((1, 0), 1.0 / 60.0, (WIDTH, HEIGHT), world.bounds());
            let moved = renderer.camera_x != was;
            if moved {
                moving_frames += 1;
            }
            pixel_physics::sim::parallel::step(&mut world);
            let touched = world.take_touched_chunks();
            let started = std::time::Instant::now();
            let recomputed = renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH, HEIGHT), i == 0);
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            if moved {
                worst_moving = worst_moving.max(ms);
            } else {
                worst_pinned = worst_pinned.max(ms);
            }
            if recomputed < vw * vh {
                skipped += 1;
            }
            if i % a.every == 0 {
                frames.push(frame.clone());
            }
        }
        let travelled = renderer.camera_x - start_x;
        let moving_secs = (moving_frames as f32 / 60.0).max(1.0 / 60.0);
        println!(
            "  scrolled world x {start_x} -> {} ({travelled} cells over {moving_frames} moving frames \
             = {:.0} cells/s, {:.2} screens/s; {} s of key held in total)",
            renderer.camera_x,
            travelled as f32 / moving_secs,
            // Divided by the **visible span**, not by `WIDTH`. A screenful is
            // 128 cells at zoom 4 and 1024 at stride 2, so dividing by the
            // framebuffer width reported a scroll running at exactly 1.5
            // screens/s as 0.38 and 3.00 -- the readout wrong at every scale
            // but 1:1, which is precisely the claim it exists to check.
            travelled as f32 / moving_secs / renderer.visible_span((WIDTH, HEIGHT)).0 as f32,
            a.shots
        );
        // A frame that recomputed less than the whole buffer is one the camera
        // did not move on -- at the world's edge that is correct and expected,
        // anywhere else it is the smear this harness exists to catch.
        println!("  {total} frames, {skipped} not fully redrawn (expected once the camera clamps at the world edge)");
        println!("  worst render frame: {worst_moving:.2} ms scrolling, {worst_pinned:.2} ms once pinned at the edge");

        // Delay follows the sampling, so a thinned animation still plays back
        // at the speed the scroll actually runs at.
        let delay_ms = ((a.every as u64 * 1000) / 60).max(16);
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms));
        let encoded = frames.len();
        let file = std::fs::File::create(&a.out).expect("creating the gif");
        let mut encoder = image::codecs::gif::GifEncoder::new(file);
        encoder.set_repeat(image::codecs::gif::Repeat::Infinite).expect("gif repeat");
        for f in frames {
            let buf = image::RgbaImage::from_raw(WIDTH, HEIGHT, f).expect("gif frame");
            encoder.encode_frame(image::Frame::from_parts(buf, 0, 0, delay)).expect("gif frame");
        }
        drop(encoder);
        println!(
            "animated gif ({}x{}, {encoded} of {total} frames at {delay_ms} ms): {}",
            WIDTH, HEIGHT, a.out
        );
        return;
    }

    for shot in 0..a.shots {
        // Aimed at the strike when there is one: a bolt lands anywhere in a
        // world four screens wide, so a shot framed anywhere else is a render
        // of a flash with the interesting part outside it.
        let aimed = pixel_physics::sim::weather::strike(world.seed, world.frame, world.bounds()).map(|s| s.x);
        let x = match (a.mine, aimed) {
            (true, _) => WORLD_WIDTH as i32 / 4,
            (_, Some(sx)) => sx,
            _ => ((shot as f32 + 0.5) / a.shots as f32 * WORLD_WIDTH as f32) as i32,
        };
        let ground = (0..WORLD_HEIGHT as i32)
            .find(|&y| world.get(x, y).material != material::EMPTY)
            .unwrap_or(WORLD_HEIGHT as i32 / 2);
        // `pan=` scrolls with the real map-scroll rate between shots instead
        // of teleporting to a target, so consecutive tiles are what a player
        // holding `D` actually sees. At the default 1.5 screens/s, `pan=0.7`
        // is a little over one screenful per tile, so neighbouring tiles
        // should abut with a sliver of overlap -- and **uneven tile spacing
        // is the dropped-residual bug made visible in a still image**, which
        // is unusual enough to be worth exploiting.
        if a.pan > 0.0 {
            if shot == 0 {
                renderer.set_camera(0, ground - HEIGHT as i32 / 2, (WIDTH, HEIGHT), world.bounds());
            }
            for _ in 0..(a.pan * 60.0) as u32 {
                renderer.pan((1, 0), 1.0 / 60.0, (WIDTH, HEIGHT), world.bounds());
            }
        } else {
            renderer.follow((x, ground), (WIDTH, HEIGHT), world.bounds());
        }
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
        let started = std::time::Instant::now();
        let recomputed = renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH, HEIGHT), shot == 0);
        // The discrete "did it fire" number, printed next to the picture
        // because the picture cannot show it. `WIDTH * HEIGHT` means the
        // camera move forced the full redraw; anything less means the
        // dirty-rect skip kept the previous shot's pixels and the sheet is
        // several copies of one view wearing different captions. Also the
        // frame cost, measured here rather than remembered from another
        // session on another machine.
        println!(
            "    {recomputed} px recomputed ({} = full) in {:.2} ms",
            vw * vh,
            started.elapsed().as_secs_f64() * 1000.0
        );

        // Mined *between* draws, which is the sequence a player performs and
        // the only one that reproduces the bug. Doing it before the first
        // draw does not: the renderer's opening scan of the skyline would see
        // the shaft already there and record its floor as the true ground, so
        // the sky comes down the hole and the fix appears not to work. The
        // reproduction has to contain the order of events, not just the
        // final state.
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

        for y in 0..vh {
            let src = y * vw * 4;
            let dst = (y * vw * a.shots + shot * vw) * 4;
            sheet[dst..dst + vw * 4].copy_from_slice(&frame[src..src + vw * 4]);
        }
        // Step between shots so the day advances a little and the sky is not
        // bit-identical in every tile -- a sheet of identical skies hides a
        // sky that is not being redrawn at all.
        for _ in 0..(a.frame / a.shots.max(1)) {
            pixel_physics::sim::parallel::step(&mut world);
        }
    }

    let (sw, sh) = ((vw * a.shots) as u32, vh as u32);
    if let Some(dir) = std::path::Path::new(&a.out).parent() {
        std::fs::create_dir_all(dir).expect("creating the output directory");
    }
    image::save_buffer(&a.out, &sheet, sw, sh, image::ColorType::Rgba8).expect("writing the sheet");
    println!("contact sheet ({sw}x{sh}, {} viewport shots): {}", a.shots, a.out);
}
