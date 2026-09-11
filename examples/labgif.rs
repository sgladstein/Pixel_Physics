//! **A short animated GIF of the lab's mister, through the real renderer.**
//!
//! Neither of `CLAUDE.md`'s two named routes reaches the lab as it stands:
//! `filmstrip`'s `gif=1` only builds outdoor scenes, and `bin/lab.rs`'s own
//! headless hook (`PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES`) writes one still,
//! never a sequence. This is the minimal third thing -- `Lab::draw` (the
//! same call the interactive binary makes every frame, bar and all) into a
//! buffer, at an interval, assembled into a GIF the identical way `main.rs`'s
//! own `CaptureSequence::finish` does (`image::codecs::gif::GifEncoder`,
//! infinite repeat, a real per-frame delay) -- so the pixels in the card are
//! the pixels a player would see, not a bespoke render path of this file's
//! own that could quietly disagree with them.
//!
//! ```text
//! cargo run --release --example labgif -- rain=steady out=/tmp/steady.gif
//! cargo run --release --example labgif -- rain=off out=/tmp/off.gif        # the control
//! cargo run --release --example labgif -- rain=heavy frames=900 every=6 out=/tmp/heavy.gif
//! ```
//!
//! **`rain=` overrides the scenario's own saved rate** (`played_bed.ron`
//! ships `Off`, the shipped default) so this can render any rate on demand
//! without a throwaway scenario file for each one. **`start=`** advances the
//! box before capturing begins -- default 30,000, past the point the played
//! bed's own population has established (`Reports/...`: plant cells and
//! organism counts are already in the thousands by 30,000) but well before
//! the late-run population crash the unwatered measurement in `lab::rain`'s
//! own header shows -- so the card is judged on a representative bed, not an
//! empty one or a dying one.
//!
//! Prints the soil-water total and the mister's own effect count
//! (`World::rain_cells`) at the start and end of the captured window, the
//! two figures `CLAUDE.md`'s review-card convention asks for beside the
//! image -- an image says whether the rate *looks* like rain, and only the
//! counters say whether it *did* anything.

use pixel_physics::lab::rain::Rain;
use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::update;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// The head cell of the first living animal of `species` (lowest organism id
/// first) -- **P2's own card knob, `follow=` below.** Reads world state only;
/// the camera write happens at the call site, which borrows `lab.renderer`
/// separately, so the two can never conflict.
fn head_of_first(world: &World, species: &str) -> Option<(i32, i32)> {
    let sid = world.species.id_of(species)?;
    world.live_organism_ids().into_iter().find_map(|id| {
        let state = world.organism(id)?;
        if state.species != sid {
            return None;
        }
        state.chain.first().copied()
    })
}

/// The bed's whole soil-water total, `soil_drawdown.rs`'s own census over
/// `water_capacity`-bearing cells -- duplicated rather than imported since
/// that file's helper is private to it and this is three lines.
fn soil_total(lab: &Lab) -> u64 {
    let spec = &lab.spec;
    let w = &lab.world;
    let mut t = 0u64;
    for y in spec.ground_y..(spec.ground_y + spec.soil_depth) {
        for x in 0..spec.width {
            let c = w.get(x, y);
            if w.materials.get(c.material).water_capacity > 0 {
                t += update::soil_moisture(c) as u64;
            }
        }
    }
    t
}

/// **Round 28's garden-loop card marker.** Draws a bright annulus (a ring,
/// not a filled disc, so it does not itself hide the one cell it points at)
/// into an RGBA buffer at `(cx, cy)` with the given outer `radius` and a
/// fixed 2px thickness -- a plain distance-squared test, no crate needed
/// for one shape. Clipped to the buffer's own bounds rather than panicking,
/// since a caller near an edge is a real case (`crop=` can leave the ring
/// partly off-frame) and losing the visible arc is better than losing the
/// frame.
#[allow(clippy::too_many_arguments)]
fn draw_ring(buf: &mut [u8], full_w: u32, full_h: u32, cx: i64, cy: i64, radius: i64, color: [u8; 4]) {
    let thickness = 2i64;
    let outer2 = radius * radius;
    let inner2 = (radius - thickness).max(0) * (radius - thickness).max(0);
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let d2 = dx * dx + dy * dy;
            if d2 > outer2 || d2 < inner2 {
                continue;
            }
            let (px, py) = (cx + dx, cy + dy);
            if px < 0 || py < 0 || px as u32 >= full_w || py as u32 >= full_h {
                continue;
            }
            let idx = ((py as u32 * full_w + px as u32) * 4) as usize;
            buf[idx..idx + 4].copy_from_slice(&color);
        }
    }
}

// **A "cells of water still in the air" census was tried here and pulled,
// per `CLAUDE.md`'s own rule to check a fresh number against a case known to
// be quiet.** The `rain=off` control read **30** cells "in the air" over
// the identical window with the mister's own counter flat at 0 the whole
// time -- `weather::condense_under_a_lid` (`soil_drawdown.rs`'s own doc:
// the sealed box's real water cycle) drops condensation from the ceiling on
// its own, unrelated to this feature, so a census of the band could not
// tell the two sources apart. `World::rain_cells` is the number that stays
// specific to this mechanism -- it is incremented nowhere else -- so that
// is what this file reports instead.

fn main() {
    let scenario_name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let seed: u64 = arg("seed").unwrap_or(1);
    let rain_name: String = arg("rain").unwrap_or_else(|| "steady".to_string());
    let rain = match rain_name.to_ascii_lowercase().as_str() {
        "off" => Rain::Off,
        "light" => Rain::Light,
        "heavy" => Rain::Heavy,
        _ => Rain::Steady,
    };
    let start: u64 = arg("start").unwrap_or(30_000);
    let frames: u64 = arg("frames").unwrap_or(600);
    let every: u64 = arg("every").unwrap_or(5);
    let zoom: u32 = arg("zoom").unwrap_or(1).max(1);
    // **A2's own reason for existing: this file's `zoom` was a pixel
    // replicate of the box's default (ceiling-level) camera, never a crop --
    // fine for a whole-box rain card, useless for "zoomed on the nest
    // column" (Brief A2). `center=x,y` opts into a real camera zoom instead,
    // through the same `Renderer` the interactive app scrolls with, and is
    // additive: leaving it unset reproduces every existing rain card
    // byte-for-byte, since neither this branch nor its cousin below ever
    // runs. Given as a world-cell *centre*, unlike `labshot`'s `look=` (a
    // top-left corner) -- centring is what "zoomed on a column" wants and a
    // corner is not.
    let center: Option<(i32, i32)> = arg::<String>("center").map(|s| {
        let v: Vec<i32> = s.split(',').map(|p| p.trim().parse().expect("center wants x,y")).collect();
        assert_eq!(v.len(), 2, "center wants exactly x,y, got {s:?}");
        (v[0], v[1])
    });
    // **`follow=<species>` -- P2's own card knob**
    // (`Reports/evolution-lab-pollinator-design-2026-09-10.md` Brief P2).
    // Reuses `center=`'s zoom machinery below, but re-centres on the head of
    // the first living animal of `species` every captured frame instead of
    // on a fixed point, so a card can follow one animal across a hop rather
    // than watching a column the animal may cross in one frame and leave
    // the next. **Orthogonal to `center`: leaving `follow` unset takes
    // neither new branch below and reproduces every existing card
    // byte-for-byte**, the identical guarantee `center=`'s own doc states
    // for itself -- the two knobs are additive, not a rewrite of one path.
    let follow: Option<String> = arg::<String>("follow");
    let out: String = arg("out").unwrap_or_else(|| "/tmp/labrain.gif".to_string());
    // **`up=N` -- a nearest-neighbour integer upscale of the `png_dir=`
    // frames only**, `labstats`' own knob and for its own reason: the review
    // page scales client-side, and the skill's measured bar is that the
    // stills the owner has been able to judge are 700-950 px across, against
    // a 190x130 crop he reported seeing none of the changes in. The lab
    // canvas is 512x320, so a card of it is under that bar before it starts,
    // and under a camera (`center=`/`follow=`) it stays 512x320 however far
    // in the camera zooms -- the renderer puts fewer, bigger world cells in
    // the same buffer rather than a bigger buffer.
    //
    // Nearest-neighbour, never a filter: every pixel here IS a world cell (or
    // an integer block of one), and a smoothed edge would invent gradients
    // the simulation does not have. Deliberately does not touch the GIF --
    // the skill says never to carry a zoom into the queue in a GIF, because
    // the page can already zoom one client-side, and the frame view is what
    // it says to prefer anyway.
    let up: u32 = arg("up").unwrap_or(1).max(1);
    // **`crop=x,y,w,h`, `filmstrip`'s own convention, added rather than
    // relying on `zoom` alone** -- the review skill is explicit that a GIF
    // should never carry `zoom` into the queue (`image-rendering: pixelated`
    // already lets the viewer zoom client-side for free), so a card asking
    // "does the visit at one flower head read" needs pixels removed before
    // encoding, not multiplied. Applied to the raw `WIDTH x HEIGHT` buffer,
    // before zoom, so the two compose exactly as `filmstrip`'s do. Absent or
    // unparsable means the whole frame, today's behaviour exactly.
    let crop: Option<(u32, u32, u32, u32)> = arg::<String>("crop").and_then(|s| {
        let parts: Vec<u32> = s.split(',').filter_map(|p| p.parse().ok()).collect();
        (parts.len() == 4).then(|| (parts[0], parts[1], parts[2], parts[3]))
    });
    // **`mark=1` -- round 28's garden-loop card, "the pip cell marked".**
    // `.claude/skills/review/SKILL.md`: "a one-cell event is unreadable on a
    // card even at 5x unless it is marked" -- a delivered `pip` is exactly
    // that (`Reports/lanes/evolution-lab-garden-loop.md`). Rings the exact
    // point `center=` put at the middle of the frame rather than taking a
    // second world coordinate: `center`'s own math (`ccx - span_x/2` etc.)
    // already puts that world cell at pixel `(full_w/2, full_h/2)` before
    // any crop, so marking there needs no camera-to-pixel inversion of its
    // own and cannot drift out of sync with what `center=` actually framed.
    // Requires `center=` -- ringing an unset default camera would ring a
    // point this file has no claim about.
    let mark: bool = arg::<u32>("mark").unwrap_or(0) != 0;
    if mark && center.is_none() {
        eprintln!("labgif: mark=1 with no center= rings nothing -- center= names the world cell to ring");
    }
    // **`png_dir=<path>` -- writes every captured frame as its own PNG
    // there too, numbered in capture order, alongside the GIF.** Added for
    // the same card: `.claude/skills/review/SKILL.md` prefers a scrubbable
    // frame sequence over a GIF "when the question is detail" -- here,
    // the exact instant a delivered pip is re-bitten -- and a GIF's own
    // frames cannot be posted individually after the fact. Optional and
    // additive: omitted, this file's output is byte-for-byte what it always
    // wrote.
    let png_dir: Option<String> = arg("png_dir");
    if let Some(dir) = &png_dir {
        std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("labgif: png_dir {dir}: {e}"));
    }

    let mut sc = Scenario::load(&scenario_name).unwrap_or_else(|e| {
        eprintln!("scenario {scenario_name}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    // **`creature=<species>` overrides who a scenario's `Colony`/
    // `Colonies` placements found** -- `labforage`'s own knob, same syntax,
    // added there first for P1's three-arm measurement
    // (`Reports/evolution-lab-pollinator-design-2026-09-10.md`). A card
    // showing a bloom-wired species arriving at a flower on the owner's own
    // played bed needs this scenario to found that species rather than the
    // `.ron` file's own `Colony(species: "ant", ...)`.
    if let Some(species) = arg::<String>("creature") {
        sc.bed.colony_species = species.clone();
        for p in sc.placements.iter_mut().chain(sc.timeline.iter_mut().map(|e| &mut e.what)) {
            match p {
                Placement::Colony { species: s, .. } | Placement::Colonies { species: s, .. } => *s = species.clone(),
                _ => {}
            }
        }
    }
    // **The warm-up runs at `Off`, whatever `rain=` asked for** -- the
    // requested rate is armed only once capturing starts, below. A card
    // built by pre-soaking the bed for `start` frames at the target rate
    // first shows a puddled bed getting marginally more puddled, which
    // answers a much weaker question than "does turning this on read as
    // rain arriving" -- and the file's own header claims the latter.
    sc.bed.rain = Rain::Off;
    let mut lab = Lab::new(sc.bed.clone());
    // The key-list overlay is on by default outside the real binary
    // (`Lab::new`'s `PIXEL_PHYSICS_LAB_HELP` gate) and paints over the whole
    // screen -- every other headless harness that calls `Lab::draw` turns it
    // off first, and this one must too or the card is a picture of the help
    // page rather than the box.
    lab.show_help = false;
    let msg = lab.load_scenario(sc);
    // **After `load_scenario`, not before**: `reset()` (which it calls)
    // replaces `self.stats` with a fresh `Stats::new()`, which opens
    // *showing* -- the biosphere overlay, covering most of the bed with
    // numbers -- so closing it earlier would be undone by the load.
    // `labstats.rs`'s own `page=ants` branch closes it for the identical
    // reason: a picture taken before this is a picture of the readout, not
    // of the rain.
    //
    // **The HUD keeps reading "PAUSED", and that is left alone rather than
    // forced.** `load_scenario` itself sets `Phase::Paused` (the moment a
    // player expects to look at what got placed before running it), and
    // this harness drives frames through `tick_for_harness` rather than the
    // dial -- forcing `Phase::Running` without also feeding `TimeControl`'s
    // `plan`/`record` loop left the corner readout printing a stale
    // "0 ticks per frame" instead, which is a worse lie than "paused". The
    // box is genuinely ticking; the label is a harness artifact either way,
    // and `context_md` on the card says so.
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    println!(
        "labgif: scenario={scenario_name} seed={seed} colony={} rain={} start={start} frames={frames} every={every} zoom={zoom} crop={} follow={} out={out} mark={mark} png_dir={} up={up}",
        lab.spec.colony_species,
        rain.label(),
        crop.map_or_else(|| "none".to_string(), |(x, y, w, h)| format!("{x},{y},{w},{h}")),
        follow.as_deref().unwrap_or("none"),
        png_dir.as_deref().unwrap_or("none")
    );
    println!("  {msg}");

    for _ in 0..start {
        lab.tick_for_harness();
    }
    // The requested rate arms right here -- the captured window is the
    // player's own experience of pressing the RAIN key on an established
    // bed, not a bed that was already soaked in it.
    lab.spec.rain = rain;
    let water_before = soil_total(&lab);
    let rain_before = lab.world.rain_cells;
    println!("  at frame {start}: soil water {water_before}, rain cells so far {rain_before} (rain now armed at {})", rain.label());

    let (full_w, full_h) = (WIDTH, HEIGHT);
    // **`center=` drives a real `Renderer` zoom**, through the identical
    // `adjust_zoom`/`set_camera` machinery the interactive app's scroll
    // wheel calls -- so the card shows fewer, bigger world cells around
    // the nest rather than the same ceiling-level view blown up or merely
    // trimmed. Set once, before the capture loop: the nest does not move
    // over a few hundred frames, and re-centring every frame would fight a
    // laden ant's own drift across the crop.
    //
    // **Orthogonal to `crop=` below, and composes with it.** `center`
    // decides which world cells `lab.draw` puts into the frame at all;
    // `crop` trims the rendered frame afterward, exactly as `filmstrip`'s
    // own crop does -- a `center` shot can still be cropped tighter.
    // `follow` reuses this same zoom/bounds/span setup -- see its own doc
    // above. Neither `adjust_zoom` nor the two reads below run at all when
    // both `center` and `follow` are unset, which is the byte-identity
    // `center=`'s own doc already promises and `follow=` inherits for free.
    let camera_mode = center.is_some() || follow.is_some();
    if camera_mode {
        for _ in 1..zoom {
            lab.renderer.adjust_zoom(1);
        }
    }
    let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, lab.spec.width - 1, lab.spec.height - 1);
    let (span_x, span_y) = lab.renderer.visible_span((full_w, full_h));
    if let Some((ccx, ccy)) = center {
        lab.renderer.set_camera(ccx - span_x / 2, ccy - span_y / 2, (full_w, full_h), Some(bounds));
        println!("  camera centred on ({ccx},{ccy}) at {zoom}x -- {span_x}x{span_y} world cells visible");
    } else if let Some(species) = &follow {
        match head_of_first(&lab.world, species) {
            Some((hx, hy)) => {
                lab.renderer.set_camera(hx - span_x / 2, hy - span_y / 2, (full_w, full_h), Some(bounds));
                println!("  camera following {species} at ({hx},{hy}) at frame {start}, {zoom}x -- {span_x}x{span_y} world cells visible");
            }
            None => println!("  follow={species}: no live animal of that species at frame {start} -- camera left at default"),
        }
    }
    // The crop rect, clamped into the real frame so an out-of-bounds request
    // (a flower head near an edge, plus margin) shrinks rather than reading
    // past the buffer -- `filmstrip`'s own crop has this same clamp.
    let (cx, cy, w, h) = match crop {
        Some((x, y, cw, ch)) => {
            let x = x.min(full_w.saturating_sub(1));
            let y = y.min(full_h.saturating_sub(1));
            (x, y, cw.min(full_w - x), ch.min(full_h - y))
        }
        None => (0, 0, full_w, full_h),
    };
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    for f in 0..=frames {
        if f % every == 0 {
            // **`follow=` re-centres every captured frame**, not every
            // tick -- the animal drifts between captures the same amount
            // either way, and re-centring only where a frame is actually
            // drawn is cheaper for no visible difference. If the followed
            // animal has died or is not found this frame, the camera is
            // simply left where it last was rather than snapping to the
            // world origin -- a card that loses its animal mid-run should
            // show the last place it was, not jump.
            if let Some(species) = &follow {
                if let Some((hx, hy)) = head_of_first(&lab.world, species) {
                    lab.renderer.set_camera(hx - span_x / 2, hy - span_y / 2, (full_w, full_h), Some(bounds));
                }
            }
            let mut full = vec![0u8; (full_w * full_h * 4) as usize];
            lab.draw(&mut full, 60.0);
            // **Ring before crop**, so the ring is just another part of the
            // rendered frame as far as the crop/zoom code below is
            // concerned -- it composes with both for free rather than
            // needing its own offset math against whichever one ran.
            if mark && center.is_some() {
                draw_ring(&mut full, full_w, full_h, (full_w / 2) as i64, (full_h / 2) as i64, 8, [255, 0, 255, 255]);
            }
            // **`PIP_PROBE=1` -- confirms the ring is on the cell it claims
            // to be, rather than trusting the camera math by eye.** Same
            // convention as `A2_DEBUG`/`WF_DEBUG`: free when unset, one env
            // read. `CLAUDE.md`'s own rule -- a debug readout must not be a
            // function of the thing it debugs -- cuts the other way here
            // too: don't trust a marked picture without a number saying the
            // mark is where it claims.
            if std::env::var("PIP_PROBE").as_deref() == Ok("1") {
                if let Some((mx, my)) = center {
                    let cell = lab.world.get(mx, my);
                    let name = lab.world.materials.get(cell.material).name.clone();
                    println!("PIP_PROBE frame={} at=({mx},{my}) material={name} organism={}", start + f, cell.organism_id());
                }
            }
            // **`PIP_TRACE=<organism_id>` -- follows one pip's own organism
            // by handle rather than by a fixed coordinate**, because `pip`
            // is a `Powder` (`plant.rs`'s own Germinate-arm comment: "a seed
            // germinates where it lands ... a seed is a Powder, it falls")
            // and a freshly delivered one dropped into an elevated empty
            // cell can fall for several ticks before it rests -- a fixed
            // `PIP_PROBE` coordinate cannot tell "fell away" from "eaten"
            // apart, and conflating them would misread which hypothesis
            // the numbers support (`Reports/lanes/evolution-lab-garden-
            // loop.md`).
            if let Ok(want) = std::env::var("PIP_TRACE") {
                if let Ok(want_id) = want.parse::<u16>() {
                    match lab.world.organism(want_id) {
                        Some(st) => {
                            let cells: Vec<(i32, i32)> = st.cells.keys().copied().collect();
                            println!("PIP_TRACE frame={} organism={want_id} cells={cells:?}", start + f);
                        }
                        None => println!("PIP_TRACE frame={} organism={want_id} GONE (slot reclaimed or never existed yet)", start + f),
                    }
                }
            }
            let buf = if crop.is_none() {
                full
            } else {
                let mut cropped = vec![0u8; (w * h * 4) as usize];
                for row in 0..h {
                    let src = (((cy + row) * full_w + cx) * 4) as usize;
                    let dst = (row * w * 4) as usize;
                    cropped[dst..dst + (w * 4) as usize].copy_from_slice(&full[src..src + (w * 4) as usize]);
                }
                cropped
            };
            // **A camera zoom already happened inside `lab.draw`** -- the
            // renderer put fewer, bigger world cells into `full` (and so
            // into `buf`) itself, so replicating pixels again here would
            // zoom twice. With no camera at all, `zoom` is still this file's
            // original pixel replicate of whatever `crop` left (or the
            // whole frame).
            //
            // **`camera_mode`, not `center.is_some()`, and that was a real
            // bug for a day.** `follow=` arrived reusing `center=`'s zoom
            // machinery and claiming to be orthogonal to it, and this one
            // test was left reading `center` alone -- so a `follow=` card
            // zoomed twice: `zoom=6` put 85x53 world cells in the 512x320
            // buffer AND replicated every pixel six times, giving a
            // 3072x1920 frame, and a `crop=` written in that frame's
            // coordinates was clamped against the real 512x320 buffer to a
            // **4x4 image**. The tell was the log line, which prints the
            // shot's own size rather than a recomputed one -- the same
            // guard that caught this file's last size lie, two comments up.
            let (zw, zh) = if camera_mode { (w, h) } else { (w * zoom, h * zoom) };
            let zoomed = if camera_mode || zoom == 1 {
                buf
            } else {
                let mut out_buf = vec![0u8; (zw * zh * 4) as usize];
                for y in 0..zh {
                    for x in 0..zw {
                        let src = (((y / zoom) * w + (x / zoom)) * 4) as usize;
                        let dst = ((y * zw + x) * 4) as usize;
                        out_buf[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
                    }
                }
                out_buf
            };
            if let Some(img) = image::RgbaImage::from_raw(zw, zh, zoomed) {
                if let Some(dir) = &png_dir {
                    let path = std::path::Path::new(dir).join(format!("frame_{:04}_f{f}.png", shots.len()));
                    // **`up=` scales the file, never the `shots` entry**: the
                    // GIF keeps the real pixels (see `up`'s own doc), and the
                    // size the log prints at the end is still read off the
                    // shot rather than recomputed.
                    let saved = if up == 1 {
                        img.save(&path)
                    } else {
                        image::imageops::resize(&img, img.width() * up, img.height() * up, image::imageops::FilterType::Nearest).save(&path)
                    };
                    if let Err(e) = saved {
                        eprintln!("labgif: png_dir frame {}: {e}", path.display());
                    }
                }
                shots.push(img);
            }
        }
        if f < frames {
            lab.tick_for_harness();
        }
    }

    let water_after = soil_total(&lab);
    let rain_after = lab.world.rain_cells;
    println!(
        "  at frame {}: soil water {water_after} ({:+}), rain cells so far {rain_after} ({:+})",
        start + frames,
        water_after as i64 - water_before as i64,
        rain_after - rain_before
    );

    // Real playback speed and a loop, `main.rs`'s own `CaptureSequence::
    // finish` convention exactly: 60 ticks/second is the shipped sim rate,
    // so `every` ticks between captures maps directly to real elapsed time.
    let delay_ms = ((every * 1000) / 60).max(1);
    let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms));
    // **Read off the actual first shot, not recomputed from `w`/`zoom`.**
    // That recomputation was always `w * zoom, h * zoom`, which was true
    // while this file only ever built one kind of frame and became a lie
    // the moment `center=` started asking `lab.draw` for fewer, bigger
    // world cells inside the *same* WIDTHxHEIGHT canvas instead. Caught by
    // hand: the log read `2560x1600` over an image that was genuinely
    // `512x320`, `CLAUDE.md`'s own "ask what your number counts" shape.
    let (shot_w, shot_h) = shots.first().map_or((w, h), |img| (img.width(), img.height()));
    let gif_frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
    let n = gif_frames.len();
    match std::fs::File::create(&out) {
        Ok(file) => {
            let mut encoder = image::codecs::gif::GifEncoder::new(file);
            if let Err(e) = encoder.set_repeat(image::codecs::gif::Repeat::Infinite) {
                eprintln!("labgif: gif set_repeat failed: {e}");
            }
            if let Err(e) = encoder.encode_frames(gif_frames) {
                eprintln!("labgif: gif encode failed: {e}");
            }
            println!("  wrote {out} ({n} frames, {shot_w}x{shot_h} each)");
        }
        Err(e) => eprintln!("labgif: failed to create {out}: {e}"),
    }
}
