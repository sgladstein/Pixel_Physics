//! **What the app's own HUD and panels actually look like**, headless.
//!
//! Every other renderer in `examples/` draws the *world* — `filmstrip` and
//! `viewshot` build their own frames out of `Renderer::draw` and never call
//! `App::draw`, so nothing in this repo could put a picture of a panel in
//! front of the owner. The only route was the real binary under xvfb plus
//! lavapipe, which works (see `CLAUDE.md`'s Commands section) and cannot be
//! driven: there is no key injection, so the panel it screenshots is
//! whichever one the app opens on and no row is ever selected.
//!
//! This drives `App::update`/`App::draw` exactly as `main.rs` does —
//! `camera_snap`'s reason for existing, one layer up — and then sets the
//! state a keypress would have set. So a sheet from here is the real panel:
//! the same `draw_tunables_panel`, the same glyphs, the same translucency
//! over the same world.
//!
//! ```text
//! cargo run --release --example uishot -- sheet=menus out=/tmp/menus.png
//! cargo run --release --example uishot -- sheet=sky   out=/tmp/sky.png
//! cargo run --release --example uishot -- sheet=weather
//! cargo run --release --example uishot -- menu=PHYSICS row=40
//! ```
//!
//! `sheet=` tiles several frames with a caption strip under each; with no
//! `sheet=` it renders one frame from `menu=`/`row=`/`sky=`/`weather=`.
//!
//! **Read `water` only among presets that are not freezing.** A freeze moves
//! a water cell into ice, which the ledger prices at ice's own density, so
//! the column is comparable across CLEAR/BREEZE/GALE/RAIN/STORM (which is
//! where it does its work — dry −451, rain −354, storm −159 on the shipped
//! world) and is a different ledger under FROST/SNOW/BLIZZARD.
//!
//! **It echoes its own parameters on the first line**, per `CLAUDE.md`: a
//! 3.5-hour study once produced eight byte-identical logs because a `seed=`
//! argument reached a binary that predated it, and an argument nobody can
//! see the value of is an argument nobody can tell is being ignored.

use pixel_physics::app::{App, HEIGHT, WIDTH};
use pixel_physics::hud;
use pixel_physics::sim::clock::SkyPin;
use pixel_physics::sim::field;
use pixel_physics::sim::weather::{self, Pin as WeatherPin};
use pixel_physics::tunables::TunableGroup;

/// How long the world runs before anything is drawn.
///
/// Not zero, and not settled either. A freshly generated world has nothing
/// growing on it and its field is still converging, so a panel drawn over it
/// is translucent over a flat picture — which flatters the panel and is not
/// what anyone will see. Long enough for the field to light and the moss to
/// show, short enough to render eight of these in a sitting.
const WARMUP: usize = 400;

/// Caption strip height under each tile.
const CAPTION: u32 = 12;

/// How long each tile runs *after* its pin is applied, before it is drawn.
///
/// Not zero: a pinned storm has to actually rain before there is anything to
/// photograph, and a pinned sky changes the light the field then has to
/// re-solve. Drawing on the frame the pin lands shows the old world under
/// the new sky, which is a picture of nothing having happened yet.
const SETTLE: usize = 200;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let get = |key: &str| -> Option<String> {
        args.iter().find_map(|a| a.strip_prefix(&format!("{key}=")).map(str::to_string))
    };
    let sheet = get("sheet").unwrap_or_default();
    let menu = get("menu").unwrap_or_else(|| "WORLD".into());
    let row: usize = get("row").and_then(|v| v.parse().ok()).unwrap_or(0);
    let sky = get("sky").unwrap_or_default();
    let weather = get("weather").unwrap_or_default();
    let seed: u32 = get("seed").and_then(|v| v.parse().ok()).unwrap_or(0);
    let frames: usize = get("frames").and_then(|v| v.parse().ok()).unwrap_or(WARMUP);
    let out = get("out").unwrap_or_else(|| std::env::temp_dir().join("uishot.png").display().to_string());
    let columns: u32 = get("columns").and_then(|v| v.parse().ok()).unwrap_or(2);
    let settle: usize = get("settle").and_then(|v| v.parse().ok()).unwrap_or(SETTLE);

    println!(
        "uishot: sheet={sheet:?} menu={menu} row={row} sky={sky:?} weather={weather:?} \
         seed={seed} frames={frames} settle={settle} columns={columns} out={out}"
    );

    let tiles: Vec<(String, Shot)> = match sheet.as_str() {
        "menus" => TunableGroup::all()
            .iter()
            .map(|g| (g.label().to_string(), Shot { menu: Some(*g), row, ..Shot::default() }))
            .collect(),
        "sky" => SkyPin::ALL
            .iter()
            .map(|p| (format!("SKY {}", p.label()), Shot { sky: Some(*p), ..Shot::default() }))
            .collect(),
        // **Every weather tile is pinned to noon.** Otherwise the world
        // clock runs on across the sheet and the nine tiles are at nine
        // different hours -- measured before this line existed, the sun ran
        // +0.98 on the first tile to +0.30 on the last, so BLIZZARD was
        // dimmer than CLEAR for a reason that has nothing to do with the
        // weather. `CLAUDE.md`: a designed oscillator has to be divided out
        // of anything it reaches, measurements as much as decisions. It is
        // also simply the better picture: weather is most legible in
        // daylight.
        "weather" => WeatherPin::ALL
            .iter()
            .map(|p| {
                (p.label().to_string(), Shot { weather: Some(*p), sky: Some(SkyPin::Noon), ..Shot::default() })
            })
            .collect(),
        "" => {
            let shot = Shot {
                menu: Some(group_named(&menu)),
                row,
                sky: (!sky.is_empty()).then(|| sky_named(&sky)),
                weather: (!weather.is_empty()).then(|| weather_named(&weather)),
            };
            vec![(menu.clone(), shot)]
        }
        other => {
            eprintln!("unknown sheet={other}; expected menus, sky or weather");
            std::process::exit(2);
        }
    };

    // **The count goes on the card, beside the image.** `CLAUDE.md`'s house
    // rule for a review card, and it is not decoration here: FROST and CLEAR
    // are the same picture until something freezes, BREEZE and GALE are the
    // same picture in any still, and a sky pin that silently did nothing
    // would look exactly like one that worked on a seed whose weather
    // happened to agree. Only the numbers separate those.
    let mut rendered: Vec<(String, Vec<u8>)> = Vec::new();
    for (caption, shot) in &tiles {
        // **A fresh world per tile, not one world re-photographed.**
        // Reusing one was the first version and it made the sheet a
        // *sequence*: FROST leaves ice behind, so the RAIN tile after it
        // reported 2,581 melts that were nothing to do with rain, and SNOW
        // started from an already-frozen world. Worldgen is deterministic,
        // so a fresh `App` at the same seed is the *same* world -- which is
        // what makes these paired comparisons rather than nine samples from
        // a wide distribution (`CLAUDE.md`: compare two runs, not one run
        // against a remembered number). It costs one worldgen per tile.
        let mut app = App::new();
        for _ in 0..seed {
            app.next_seed();
        }
        for _ in 0..frames {
            app.update();
        }
        let (buf, stats) = shot.render(&mut app, settle);
        println!("{caption:>16}  {stats}");
        // The counters belong beside a *pinned* tile, where they are the
        // only thing separating two identical pictures. On the menus sheet
        // nothing is pinned and nothing is stepped, so they would be a row
        // of zeroes under a picture of a panel -- noise that makes the card
        // look like it is reporting something.
        let pinned = shot.sky.is_some() || shot.weather.is_some();
        rendered.push((if pinned { format!("{caption}   {stats}") } else { caption.clone() }, buf));
    }

    // Tiled into a contact sheet, captions under each. One tile still goes
    // through this path so a single shot and a sheet are the same picture at
    // the same scale.
    let cols = columns.max(1).min(rendered.len() as u32);
    let rows = rendered.len().div_ceil(cols as usize) as u32;
    let tile_h = HEIGHT + CAPTION;
    let (sw, sh) = (WIDTH * cols, tile_h * rows);
    let mut sheet_buf = vec![0u8; (sw * sh * 4) as usize];
    for (i, (caption, buf)) in rendered.iter().enumerate() {
        let (cx, cy) = (i as u32 % cols * WIDTH, i as u32 / cols * tile_h);
        for y in 0..HEIGHT {
            let src = (y * WIDTH * 4) as usize;
            let dst = (((cy + y) * sw + cx) * 4) as usize;
            sheet_buf[dst..dst + (WIDTH * 4) as usize].copy_from_slice(&buf[src..src + (WIDTH * 4) as usize]);
        }
        // The caption strip is drawn into the sheet directly rather than
        // over the frame, so it can never be mistaken for something the app
        // itself put on screen -- which is the whole failure mode of
        // annotating a UI screenshot.
        hud::draw_text(&mut sheet_buf, sw, sh, (cx + 6) as i32, (cy + HEIGHT + 3) as i32, caption, [
            210, 214, 224, 255,
        ]);
    }

    image::save_buffer(&out, &sheet_buf, sw, sh, image::ColorType::Rgba8).expect("write sheet");
    println!("wrote {out} ({sw}x{sh}, {} tiles)", rendered.len());
}

#[derive(Default, Clone, Copy)]
struct Shot {
    menu: Option<TunableGroup>,
    row: usize,
    sky: Option<SkyPin>,
    weather: Option<WeatherPin>,
}

impl Shot {
    /// One frame, with the state a keypress would have set.
    ///
    /// **Steps after applying a pin, rather than drawing immediately.** A
    /// pinned sky changes the light the field has to re-solve and a pinned
    /// storm has to actually rain before there is anything to photograph;
    /// drawing on the frame the pin lands shows the *old* world under the
    /// new sky, which is a picture of nothing having happened yet. Two
    /// hundred frames is enough for the light to settle and for a front to
    /// put visible precipitation on screen.
    fn render(&self, app: &mut App, settle: usize) -> (Vec<u8>, String) {
        app.world.set_sky_hold(self.sky.unwrap_or_default().hold());
        app.world.set_weather_pin(self.weather.unwrap_or_default());
        // Deltas across this tile's own window, not totals: one `App` is
        // reused across the sheet (see `main`), so a running total would
        // report every previous tile's events as this one's.
        let before = app.world.phase_changes;
        // **Water as well as phase changes**, because the first version of
        // this line measured freezing and lightning only -- so RAIN, whose
        // whole job is to put water in the world, reported `froze 0 melted 0
        // bolts 0` and read as the one preset that does nothing. That is
        // `CLAUDE.md`'s metric trap exactly: a number that is arithmetically
        // correct and about the wrong quantity looks the same as a null.
        let water_before = weather::water_equivalents(&app.world);
        let mut strikes = 0usize;
        // **Wind needs its own counter and cannot borrow anybody else's.**
        // BREEZE and GALE were the three presets on this sheet that moved no
        // number at all -- identical water, no freezing, no bolts -- and are
        // identical to CLEAR in any still, because what wind does is move
        // things and a photograph cannot show that. `planned_gust` is the
        // same code path `gust` delivers through, so this counts what the
        // world actually got rather than a harness's idea of it. It is still
        // only "it fired": whether a gale *reads* as a gale is a question for
        // `filmstrip gif=1`, and this file cannot answer it.
        let mut gusts = 0usize;
        let mut gust_force = 0.0f32;
        for _ in 0..settle {
            app.update();
            // Counted at `age == 0` so one flash is one event rather than
            // `STRIKE_FRAMES` of them -- the same test `weather.rs`'s own
            // lightning guard uses.
            if app.world.lightning_at(app.world.frame).is_some_and(|s| s.age == 0) {
                strikes += 1;
            }
            if let Some(g) = weather::planned_gust(&app.world, app.world.weather()) {
                gusts += 1;
                gust_force += g.delivered;
            }
        }
        let after = app.world.phase_changes;
        let sky_frame = app.world.sky_frame();
        // **Ice *in the drawn window*, next to the world-wide event count.**
        // The shipped world is 8192x2560 and the frame shows 512x320 of it,
        // so `froze` is counting over sixteen screens of width and eight of
        // height. FROST measured 3,844 freezes against a picture whose
        // waterline was pixel-identical to CLEAR's -- which reads as a
        // counter firing while nothing happens, and is really a counter
        // answering a question about the world next to a picture of 1/128th
        // of it. Both numbers, so the card cannot be misread either way.
        let ice = app.world.materials.id_of("ice");
        let (vx0, vy0) = app.renderer.screen_to_world(0, 0);
        let (vx1, vy1) = app.renderer.screen_to_world(WIDTH as i32 - 1, HEIGHT as i32 - 1);
        let mut ice_here = 0usize;
        if let Some(ice) = ice {
            for y in vy0..=vy1 {
                for x in vx0..=vx1 {
                    if app.world.get(x, y).material == ice {
                        ice_here += 1;
                    }
                }
            }
        }
        let stats = format!(
            "sun {:+.2}  water {:+.0}  gusts {} @{:.0}  froze {}  melted {}  bolts {}  ice here {}",
            field::sun_elevation(sky_frame),
            weather::water_equivalents(&app.world) - water_before,
            gusts,
            gust_force,
            after.froze - before.froze,
            after.melted - before.melted,
            strikes,
            ice_here,
        );
        app.show_tunables = self.menu.is_some();
        if let Some(group) = self.menu {
            // Through the real key handlers, so the selection clamp and the
            // group reset are the app's rather than this file's.
            while app.tunables_group() != group {
                app.tunables_cycle_group();
            }
            app.tunables_move(self.row as i32);
        }
        let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        app.draw(&mut frame, None);
        (frame, stats)
    }
}

fn group_named(name: &str) -> TunableGroup {
    TunableGroup::all().into_iter().find(|g| g.label().eq_ignore_ascii_case(name)).unwrap_or_else(|| {
        eprintln!("unknown menu={name}; expected one of WORLD PHYSICS VISUAL EXPLOSION PLAYER");
        std::process::exit(2);
    })
}

fn sky_named(name: &str) -> SkyPin {
    SkyPin::ALL.into_iter().find(|p| p.label().eq_ignore_ascii_case(name)).unwrap_or_else(|| {
        eprintln!("unknown sky={name}");
        std::process::exit(2);
    })
}

fn weather_named(name: &str) -> WeatherPin {
    WeatherPin::ALL.into_iter().find(|p| p.label().eq_ignore_ascii_case(name)).unwrap_or_else(|| {
        eprintln!("unknown weather={name}");
        std::process::exit(2);
    })
}
