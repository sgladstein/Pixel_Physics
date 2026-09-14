//! **What a quickening looks like** — the held world's circles, drawn
//! headless, with the old outline beside the new haze.
//!
//! Owner, 2026-09-14: *"I want you to improve the bubbles look. They
//! shouldn't be a solid line it blocks too much. I am thinking hazy
//! shimmering aura. Think about how to indicate speed visual."*
//!
//! **Why this is its own binary rather than a `filmstrip` scene.** Everything
//! in `filmstrip` builds its own frames out of `Renderer::draw` over a hand-
//! made or generated `World`; a quickening is `druid::Druid` state, and the
//! speed dial that this whole change is about is a *`Druid`* field applied by
//! `Druid::update` calling `frame::step` several times. Only the real game
//! loop produces it, so this drives `Druid::update`/`Druid::draw` exactly as
//! `src/bin/druid.rs` does — `uishot`'s reason for existing, pointed at the
//! other game.
//!
//! **A contact sheet cannot answer this question and that is the point.**
//! `CLAUDE.md`: when the question is whether something *moves* right, the GIF
//! is the only thing that answers it. `out=` ending in `.gif` writes an
//! animation; anything else writes a sheet of stills. Both are worth having —
//! the still is where the *depth* channel is read, which is the half that has
//! to survive a screenshot.
//!
//! ```text
//! cargo run --release --example druid_aura -- speed=1 out=/tmp/x1.gif
//! cargo run --release --example druid_aura -- speed=8 out=/tmp/x8.gif
//! cargo run --release --example druid_aura -- look=old speed=8 out=/tmp/old8.gif
//! cargo run --release --example druid_aura -- sheet=speeds out=/tmp/speeds.png
//! ```
//!
//! **It prints the count the review card needs.** `CLAUDE.md`'s house rule
//! for a review card is the discrete event count in the `meta`, and the count
//! that matters here is *how many pixels the haze actually tinted*: a sheet
//! showing a plausible glow is exactly what a dead aura over a lit world also
//! looks like. It is measured as the difference against the same frame drawn
//! with `alpha=0`, so it is the aura's own footprint and nothing else's.
//!
//! **It echoes its own parameters on the first line**, per `CLAUDE.md`: a
//! 3.5-hour study once produced byte-identical logs because an argument
//! reached a binary that predated it.

use pixel_physics::app::{HEIGHT, WIDTH};
use pixel_physics::druid::Druid;
use pixel_physics::render::{AuraTuning, Hud};
use pixel_physics::sim::world::Quickening;

/// Frames run before anything is drawn, so the gnome has landed and the
/// camera has settled on him. Not zero: the first frames of a fresh `Druid`
/// have the player falling, and a card of that is a card about gravity.
const SETTLE: usize = 30;

#[derive(Clone)]
struct Args {
    speed: u32,
    radius: i32,
    /// `new` (the haze) or `old` (the outline and its concentric speed
    /// rings, reconstructed here so the A/B has a real "before" arm).
    look: String,
    tune: AuraTuning,
    frames: usize,
    every: usize,
    count: usize,
    delay_ms: u16,
    out: String,
    sheet: String,
    /// `crop=x,y,w,h` in screen pixels, then `zoom=` nearest-neighbour. A
    /// 512x320 frame with a 46-cell circle in it is judged at about a tenth
    /// of the card's width otherwise, and haze is exactly the thing that
    /// disappears at that size.
    crop: Option<(u32, u32, u32, u32)>,
    zoom: u32,
    /// `cost=1` — what the haze costs, instead of what it looks like.
    cost: bool,
}

fn main() {
    let mut a = Args {
        speed: 4,
        radius: 46,
        look: "new".into(),
        tune: AuraTuning::default(),
        frames: 90,
        every: 12,
        count: 6,
        delay_ms: 33,
        out: "/tmp/druid_aura.png".into(),
        sheet: String::new(),
        crop: None,
        zoom: 1,
        cost: false,
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        let f = || v.parse::<f32>().unwrap_or(0.0);
        match k {
            "speed" => a.speed = v.parse().unwrap_or(4),
            "r" | "radius" => a.radius = v.parse().unwrap_or(46),
            "look" => a.look = v.into(),
            "alpha" => a.tune.alpha = f(),
            "depth" => a.tune.depth = f(),
            "perstep" => a.tune.depth_per_step = f(),
            "wave" => a.tune.wave = f(),
            "period" => a.tune.period = f(),
            "grain" => a.tune.grain = f(),
            "rough" => a.tune.rim_rough = f(),
            "scale" => a.tune.rim_scale = f(),
            "frames" => a.frames = v.parse().unwrap_or(90),
            "every" => a.every = v.parse().unwrap_or(12),
            "count" => a.count = v.parse().unwrap_or(6),
            "delay" => a.delay_ms = v.parse().unwrap_or(33),
            "out" => a.out = v.into(),
            "sheet" => a.sheet = v.into(),
            "cost" => a.cost = v != "false",
            "zoom" => a.zoom = v.parse().unwrap_or(1).max(1),
            "crop" => {
                let n: Vec<u32> = v.split(',').filter_map(|t| t.parse().ok()).collect();
                a.crop = (n.len() == 4).then(|| (n[0], n[1], n[2], n[3]));
            }
            _ => eprintln!("druid_aura: ignoring unknown argument `{arg}`"),
        }
    }
    let gif = a.out.to_ascii_lowercase().ends_with(".gif");
    println!(
        "druid_aura: look={} speed=x{} r={} alpha={:.2} depth={:.1}+{:.1}/step wave={:.1} period={:.0} grain={:.2} rough={:.1}@{:.0} | {} frames, {}",
        a.look,
        a.speed,
        a.radius,
        a.tune.alpha,
        a.tune.depth,
        a.tune.depth_per_step,
        a.tune.wave,
        a.tune.period,
        a.tune.grain,
        a.tune.rim_rough,
        a.tune.rim_scale,
        a.frames,
        if gif { format!("gif every frame -> {}", a.out) } else { format!("sheet of {} every {} -> {}", a.count, a.every, a.out) }
    );

    if a.cost {
        cost(&a);
        return;
    }
    if !a.sheet.is_empty() {
        sheet_of_speeds(&a);
        return;
    }

    let mut game = build(&a);
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let mut tinted_total = 0u64;
    let mut tinted_peak = 0u64;
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut control = vec![0u8; (WIDTH * HEIGHT * 4) as usize];

    for f in 0..a.frames {
        game.update();
        // **Both arms of the counter come out of the same frame.** A count
        // taken from a separate run would be a count of a different world:
        // two runs that diverge on one frame are different worlds by the
        // next (`CLAUDE.md`). `force_full` on both so neither is reading a
        // skipped chunk left over from the other.
        game.renderer.aura = if a.look == "old" { AuraTuning::off() } else { a.tune };
        game.draw(&mut buf, (WIDTH, HEIGHT), true);
        if a.look == "old" {
            draw_old_rings(&mut game, &mut buf, a.speed);
        }
        let saved = game.renderer.aura;
        game.renderer.aura = AuraTuning::off();
        game.draw(&mut control, (WIDTH, HEIGHT), true);
        game.renderer.aura = saved;
        let tinted = buf.chunks_exact(4).zip(control.chunks_exact(4)).filter(|(p, q)| p != q).count() as u64;
        tinted_total += tinted;
        tinted_peak = tinted_peak.max(tinted);

        let take = if gif { true } else { f >= SETTLE && (f - SETTLE).is_multiple_of(a.every) && shots.len() < a.count };
        if take {
            shots.push(present(&buf, a.crop, a.zoom));
        }
    }

    println!(
        "  haze footprint: {} pixels/frame mean, {} peak (0 would mean the aura never fired) | \
         the renderer measured x{} against the dial's x{}, so the haze reaches {:.1} cells inward",
        tinted_total / a.frames.max(1) as u64,
        tinted_peak,
        game.renderer.aura_rate(),
        a.speed,
        a.tune.depth + (game.renderer.aura_rate().saturating_sub(1)) as f32 * a.tune.depth_per_step
    );

    if gif {
        write_gif(&a.out, shots, a.delay_ms);
    } else {
        write_sheet(&a.out, &shots);
    }
}

/// **What the haze costs, on the state the dirty-rect skip exists for.**
///
/// `CLAUDE.md`, twice over. *Measure a cost against the state the
/// optimisation exists for* — a held world standing still with a circle
/// placed in it, which is this game's whole premise and precisely where a
/// full redraw every frame would be paid for nothing. And *gate on counters,
/// never on wall clock*: `Renderer::draw` returns the number of pixels it
/// recomputed, which is what a defeated skip actually looks like, and it does
/// not move with whatever else the box is doing.
///
/// The clock is reported too, because a counter cannot price a per-pixel
/// transform — but **paired and alternating**, on two settings of one
/// binary, which is the only shape this repo trusts a timing in.
fn cost(a: &Args) {
    use std::time::Instant;
    let mut game = build(a);
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    for _ in 0..60 {
        game.update();
        game.draw(&mut buf, (WIDTH, HEIGHT), false);
    }

    // --- the counter -------------------------------------------------------
    //
    // **Its own loop, and the first version did not have one.** Driving both
    // `Druid::draw` and `Renderer::draw` on the same frame reported `0
    // px/frame` for the control arm, because the first call had already
    // consumed `take_touched_chunks` and the second was handed an empty set:
    // a number that is arithmetically correct and answers a different
    // question than the one asked, which is `CLAUDE.md`'s single
    // worst-recurring failure here. This loop owns the touched set.
    let mut px = [0u64; 2];
    let mut pxn = [0u64; 2];
    for block in 0..8 {
        for arm in 0..2usize {
            let on = (block + arm) % 2 == 0;
            game.renderer.aura = if on { a.tune } else { AuraTuning::off() };
            let idx = usize::from(on);
            for _ in 0..a.frames {
                game.update();
                let touched = game.world.take_touched_chunks();
                px[idx] += game.renderer.draw(&game.world, &game.particles, &touched, &mut buf, (WIDTH, HEIGHT), false) as u64;
                pxn[idx] += 1;
            }
        }
    }

    // --- the clock ---------------------------------------------------------
    //
    // Whole frame — `Druid::draw`, the call the game itself makes, HUD and
    // all — because a subsystem harness overstates (`CLAUDE.md`: the same
    // field change measured -50% in its own harness and -27% through
    // `App::update`). Eight alternating blocks of two settings of one binary,
    // so a slow patch in the machine lands in both arms.
    let mut ms_total = [0.0f64; 2];
    let mut ms_worst = [0.0f64; 2];
    let mut n = [0u64; 2];
    for block in 0..8 {
        for arm in 0..2usize {
            let on = (block + arm) % 2 == 0;
            game.renderer.aura = if on { a.tune } else { AuraTuning::off() };
            let idx = usize::from(on);
            for _ in 0..a.frames {
                game.update();
                let t = Instant::now();
                game.draw(&mut buf, (WIDTH, HEIGHT), false);
                let dt = t.elapsed().as_secs_f64() * 1000.0;
                ms_total[idx] += dt;
                ms_worst[idx] = ms_worst[idx].max(dt);
                n[idx] += 1;
            }
        }
    }

    for (idx, name) in [(0usize, "aura off"), (1usize, "aura on ")] {
        println!(
            "  {name}: {:>8} px recomputed/frame | {:.3} ms mean, {:.3} ms worst over {} frames",
            px[idx] / pxn[idx].max(1),
            ms_total[idx] / n[idx].max(1) as f64,
            ms_worst[idx],
            n[idx]
        );
    }
    println!(
        "  delta: {:+} px/frame, {:+.3} ms mean, {:+.3} ms worst (whole frame, Druid::draw, x{} on a held world with one r={} circle)",
        px[1] as i64 / pxn[1].max(1) as i64 - px[0] as i64 / pxn[0].max(1) as i64,
        ms_total[1] / n[1].max(1) as f64 - ms_total[0] / n[0].max(1) as f64,
        ms_worst[1] - ms_worst[0],
        a.speed,
        a.radius
    );
    println!("  for scale, a full repaint of this frame is {} px.", WIDTH * HEIGHT);
}

fn build(a: &Args) -> Druid {
    let mut game = Druid::new();
    game.speed = a.speed;
    game.unlimited = true;
    // The key legend is half the frame and this card is about a circle.
    game.show_keys = false;
    for _ in 0..SETTLE {
        game.update();
    }
    // Placed beside him rather than under him, so the carried circle and the
    // standing one are both in frame and can be told apart — which is the
    // thing `RING_CARRIED` used to do with a colour and the haze now has to.
    if let Some(p) = game.world.player.as_ref() {
        let (px, py) = p.center();
        game.world.quickenings.push(Quickening::at(px + a.radius + 18, py - 6, a.radius));
    }
    game
}

/// **The look this change replaced**, rebuilt here rather than left to
/// memory: one outline per circle plus one more inside it per two steps of
/// the dial, tinted from cold blue toward white with the speed. Copied from
/// `druid::hud::rings` as it stood at `f2652979`, because a blind A/B whose
/// "before" arm is a description is not an A/B.
fn draw_old_rings(game: &mut Druid, frame: &mut [u8], speed: u32) {
    const RING_STANDING: [u8; 4] = [150, 220, 255, 255];
    const FLOW: [u8; 4] = [255, 250, 225, 255];
    const RING_CARRIED: [u8; 4] = [255, 214, 140, 255];
    let t = ((speed.max(1) - 1) as f32 / 7.0).clamp(0.0, 1.0);
    let tint = [
        (RING_STANDING[0] as f32 + (FLOW[0] as f32 - RING_STANDING[0] as f32) * t) as u8,
        (RING_STANDING[1] as f32 + (FLOW[1] as f32 - RING_STANDING[1] as f32) * t) as u8,
        (RING_STANDING[2] as f32 + (FLOW[2] as f32 - RING_STANDING[2] as f32) * t) as u8,
        255,
    ];
    let hc = Hud::new(WIDTH, HEIGHT, 1);
    let r = &game.renderer;
    let mut circles: Vec<(i32, i32, i32, [u8; 4])> = Vec::new();
    let on_screen = |x: i32, y: i32, rad: i32, colour: [u8; 4], out: &mut Vec<(i32, i32, i32, [u8; 4])>| {
        if let (Some((cx, cy)), Some((ex, _))) = (r.world_to_screen(x, y), r.world_to_screen(x + rad, y)) {
            out.push((cx, cy, ex - cx, colour));
        }
    };
    for q in &game.world.quickenings {
        on_screen(q.x, q.y, q.r, tint, &mut circles);
        for i in 1..=(speed.saturating_sub(1) / 2) as i32 {
            on_screen(q.x, q.y, q.r - i * 3, tint, &mut circles);
        }
    }
    if let Some(q) = game.world.carried {
        on_screen(q.x, q.y, q.r, RING_CARRIED, &mut circles);
    }
    for (cx, cy, rad, colour) in circles {
        hc.circle(frame, cx, cy, rad, colour);
    }
}

/// One still per speed on the dial, side by side — where the *depth* channel
/// is read, because that is the half a paused screen still carries.
fn sheet_of_speeds(a: &Args) {
    let speeds: Vec<u32> = a.sheet.split(',').filter_map(|s| s.parse().ok()).collect();
    let speeds = if speeds.is_empty() { vec![1u32, 2, 4, 8] } else { speeds };
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    for &s in &speeds {
        let mut args = a.clone();
        args.speed = s;
        let mut game = build(&args);
        let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        game.renderer.aura = if a.look == "old" { AuraTuning::off() } else { a.tune };
        // **Drawn every frame, not once at the end.** `Renderer::aura_rate`
        // is *world frames per drawn frame*, so a harness that updates a
        // hundred times and draws once measures the whole run as one frame.
        // That is the instrument answering the question it was asked rather
        // than the one intended — `CLAUDE.md`'s recurring failure — and it
        // read x1 at every setting of the dial until this loop drew like the
        // game does.
        for _ in 0..a.frames {
            game.update();
            game.draw(&mut buf, (WIDTH, HEIGHT), true);
        }
        if a.look == "old" {
            draw_old_rings(&mut game, &mut buf, s);
        }
        println!(
            "  x{s}: renderer measured x{}, haze reaches {:.1} cells inward",
            game.renderer.aura_rate(),
            a.tune.depth + (game.renderer.aura_rate().saturating_sub(1)) as f32 * a.tune.depth_per_step
        );
        shots.push(present(&buf, a.crop, a.zoom));
    }
    write_sheet(&a.out, &shots);
}

/// The frame as it goes on the card: cropped to the thing under discussion,
/// then magnified nearest-neighbour so a pixel stays a pixel.
fn present(buf: &[u8], crop: Option<(u32, u32, u32, u32)>, zoom: u32) -> image::RgbaImage {
    let full = image::RgbaImage::from_raw(WIDTH, HEIGHT, buf.to_vec()).expect("frame");
    let cut = match crop {
        Some((x, y, w, h)) => image::imageops::crop_imm(&full, x, y, w, h).to_image(),
        None => full,
    };
    if zoom <= 1 {
        return cut;
    }
    image::imageops::resize(&cut, cut.width() * zoom, cut.height() * zoom, image::imageops::FilterType::Nearest)
}

fn write_sheet(out: &str, shots: &[image::RgbaImage]) {
    if shots.is_empty() {
        eprintln!("druid_aura: nothing to write");
        return;
    }
    let (w, h) = (shots[0].width(), shots[0].height());
    let cols = if shots.len() > 3 { 2u32 } else { 1u32 };
    let rows = shots.len().div_ceil(cols as usize) as u32;
    let mut sheet = image::RgbaImage::new(w * cols, h * rows);
    for (i, img) in shots.iter().enumerate() {
        let (cx, cy) = (i as u32 % cols, i as u32 / cols);
        image::imageops::replace(&mut sheet, img, (cx * w) as i64, (cy * h) as i64);
    }
    match sheet.save(out) {
        Ok(()) => println!("  wrote {out} ({} tiles, {}x{})", shots.len(), sheet.width(), sheet.height()),
        Err(e) => eprintln!("druid_aura: failed to write {out}: {e}"),
    }
}

fn write_gif(out: &str, shots: Vec<image::RgbaImage>, delay_ms: u16) {
    let n = shots.len();
    let delay = image::Delay::from_numer_denom_ms(delay_ms as u32, 1);
    let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
    match std::fs::File::create(out) {
        Ok(file) => {
            let mut encoder = image::codecs::gif::GifEncoder::new(file);
            if let Err(e) = encoder.set_repeat(image::codecs::gif::Repeat::Infinite) {
                eprintln!("druid_aura: gif set_repeat failed: {e}");
            }
            if let Err(e) = encoder.encode_frames(frames) {
                eprintln!("druid_aura: gif encode failed: {e}");
            }
            println!("  wrote {out} ({n} frames at {delay_ms} ms -> {:.1} s)", n as f64 * delay_ms as f64 / 1000.0);
        }
        Err(e) => eprintln!("druid_aura: failed to create {out}: {e}"),
    }
}
