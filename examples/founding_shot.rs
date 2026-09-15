//! **What a founding leaves on the ground, and what the screen that orders it
//! looks like** — the held world's `C`, drawn headless.
//!
//! Owner playtest, 2026-09-14, three complaints in one act:
//!
//! 1. *"when I found ants there are little yellow bars that get placed onto
//!    the ground. I don't like this."*
//! 2. *"when founding ants sometime they are not founding far away from me.
//!    it should still happen right under or next to the druid."*
//! 3. *"the found menu needs to be way improved."*
//!
//! All three are judge-by-eye and the app cannot be keyed on a headless box,
//! so this drives `Druid` exactly as `src/bin/druid.rs` does and writes the
//! picture — `druid_aura`'s reason for existing, pointed at the other verb.
//! **It does not go through the window**, so it costs a second rather than
//! the two minutes an `xvfb` + lavapipe capture of the real binary does, and
//! it can render the *world alone* (`screen=0`), which the real binary cannot:
//! the biosphere page and the button bar cover the ground the nest patch is
//! painted on.
//!
//! **Every picture comes with the counts, because a picture cannot say
//! whether the thing it shows is the thing that made it.** `CLAUDE.md`: *"did
//! it fire at all" needs a counter*. So it prints, beside every run:
//!
//! - **`stations` / `placed`** — how many stands `colony_stations` offered and
//!   how many animals took. A founding that placed nobody and one that placed
//!   twelve look identical at play zoom.
//! - **`reach`** — the furthest station from the gnome's own column, and the
//!   median. This is item 2 as a number: the complaint is about *distance*,
//!   and a picture of a colony cannot say whether the far one is 20 cells out
//!   or 99.
//! - **`nest cols` and the run histogram** — the patch's painted columns and
//!   the lengths of the unbroken runs of them. This is item 1 as a number:
//!   a comb of eighteen runs of two is a **barcode**, and that is what the
//!   complaint is describing. A picture shows tan cells either way.
//!
//! ```text
//! cargo run --release --example founding_shot -- out=/tmp/nest.png crop=fit mag=3
//! cargo run --release --example founding_shot -- screen=1 offer=1 out=/tmp/menu.png
//! cargo run --release --example founding_shot -- body=5 founders=24 out=/tmp/wide.png
//! ```
//!
//! **It echoes its own parameters on the first line**, per `CLAUDE.md`: a
//! 3.5-hour study once produced byte-identical logs because an argument
//! reached a binary that predated it.

use pixel_physics::app::{HEIGHT, WIDTH};
use pixel_physics::druid::{founding, Druid};

struct Args {
    /// Commit a founding before the picture is taken.
    found: bool,
    /// Leave the founding screen open in the picture.
    offer: bool,
    /// Draw the interface as well as the world. Off by default: the page and
    /// the bar cover the ground, which is what item 1 is about.
    screen: bool,
    body: usize,
    founders: i32,
    pick: usize,
    /// Player ticks to run before the shot, so the flow has landed.
    ticks: usize,
    zoom: i32,
    /// `invisible=1` -- the framebuffer pair instead of a picture. See the
    /// call site for why it is pixels rather than palette entries.
    invisible: bool,
    /// `crop=x,y,w,h` in screen pixels, or `crop=fit` for a band centred on
    /// the gnome that holds the whole patch.
    crop: Option<(u32, u32, u32, u32)>,
    fit: bool,
    mag: u32,
    out: String,
}

fn main() {
    let mut a = Args {
        found: true,
        offer: false,
        screen: false,
        body: 0,
        founders: founding::FOUNDERS_DEFAULT,
        pick: 0,
        ticks: 20,
        zoom: 4,
        invisible: false,
        crop: None,
        fit: false,
        mag: 2,
        out: "/tmp/founding_shot.png".into(),
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "found" => a.found = v != "0",
            "offer" => a.offer = v != "0",
            "screen" => a.screen = v != "0",
            "body" => a.body = v.parse().unwrap_or(0),
            "founders" => a.founders = v.parse().unwrap_or(founding::FOUNDERS_DEFAULT),
            "pick" => a.pick = v.parse().unwrap_or(0),
            "ticks" => a.ticks = v.parse().unwrap_or(20),
            "invisible" => a.invisible = v != "0",
            "zoom" => a.zoom = v.parse().unwrap_or(4),
            "mag" => a.mag = v.parse().unwrap_or(2).max(1),
            "out" => a.out = v.into(),
            "crop" if v == "fit" => a.fit = true,
            "crop" => {
                let n: Vec<u32> = v.split(',').filter_map(|t| t.parse().ok()).collect();
                a.crop = (n.len() == 4).then(|| (n[0], n[1], n[2], n[3]));
            }
            _ => eprintln!("founding_shot: ignoring unknown argument `{arg}`"),
        }
    }
    println!(
        "founding_shot: found={} offer={} screen={} body={} founders={} pick={} ticks={} zoom={} mag={} -> {}",
        a.found, a.offer, a.screen, a.body, a.founders, a.pick, a.ticks, a.zoom, a.mag, a.out
    );

    let mut game = Druid::new();
    game.unlimited = true;
    for _ in 0..64 {
        if game.renderer.zoom >= a.zoom && game.renderer.zoom_out_stride <= 1 {
            break;
        }
        game.renderer.adjust_zoom(1);
    }
    // Let him land before anything is founded at his feet: `Druid::new`
    // drops him in and the first frames are a fall.
    for _ in 0..60 {
        game.update();
    }
    let Some((px, py)) = game.world.player.as_ref().map(|p| p.center()) else {
        println!("founding_shot: no player -- nothing to found at");
        return;
    };

    // **The station census is taken before the founding, not after.** After
    // it, the ground has animals on it and `colony_stations` answers a
    // different question -- which is exactly the shape `CLAUDE.md` warns a
    // late census has.
    let species = founding::STOCKS[a.body.min(founding::STOCKS.len() - 1)].species;
    let stations = game
        .world
        .species
        .id_of(species)
        .map(|id| game.world.colony_stations(px, py, id, a.founders))
        .unwrap_or_default();
    let mut reach: Vec<i32> = stations.iter().map(|&(cx, _)| (cx - px).abs()).collect();
    reach.sort_unstable();

    let mut placed = 0;
    if a.found || a.offer {
        game.toggle_founding();
        if let Some(offer) = game.offer.as_mut() {
            offer.body = a.body.min(founding::STOCKS.len() - 1);
            offer.founders = a.founders;
            offer.picked = a.pick.min(founding::OFFERED - 1);
        }
    }
    if a.found {
        placed = game.commit_founding();
    }
    for _ in 0..a.ticks {
        game.update();
    }
    if a.offer && game.offer.is_none() {
        game.toggle_founding();
        if let Some(offer) = game.offer.as_mut() {
            offer.body = a.body.min(founding::STOCKS.len() - 1);
            offer.founders = a.founders;
            offer.picked = a.pick.min(founding::OFFERED - 1);
        }
    }
    // **The pause, as a counter rather than a claim** -- item 3's third
    // clause, *"The game should pause when in the founding menu."* A still of
    // the screen cannot show a world standing still, and "the clock stopped"
    // is exactly the sort of thing that reads as true because the picture
    // looks the same either way. So: ticks before, twenty updates, ticks
    // after. The control is the row above -- with the screen shut the same
    // twenty updates move it by twenty.
    if a.offer {
        let before = game.ticks;
        for _ in 0..20 {
            game.update();
        }
        println!("  screen open: 20 updates moved the clock by {} tick(s) (0 is the pause; 20 is the world still running under the menu)", game.ticks - before);
    }

    // **The patch's run histogram, which is item 1 as a number.** A column
    // counts as painted if any cell in it near the surface is `nest`; the
    // runs are the unbroken stretches of painted columns, and a regular comb
    // is what reads as bars.
    {
        // The control for the line above, and it is the cheap half: with the
        // screen shut the same twenty updates must move the clock by twenty,
        // or the zero above is a world that had already stopped for some
        // other reason and says nothing about the menu.
        let shut = game.offer.is_some();
        let stashed = shut.then(|| game.close_founding());
        let before = game.ticks;
        for _ in 0..20 {
            game.update();
        }
        println!("  control, screen shut: 20 updates moved the clock by {} tick(s)", game.ticks - before);
        if stashed.is_some() {
            game.toggle_founding();
        }
    }
    let (cols, runs) = nest_runs(&game.world, px, py);
    println!(
        "  stations {:2} offered, {placed:2} placed | reach max {} median {} | nest cols {} in {} runs {:?}",
        stations.len(),
        reach.last().copied().unwrap_or(0),
        reach.get(reach.len() / 2).copied().unwrap_or(0),
        cols,
        runs.len(),
        runs
    );

    // **Is the threshold invisible?** -- item 1's real bar, and it is a claim
    // about the **rendered frame** rather than about the material table. The
    // owner rated a shape-only fix 1 of 5: *"There should be no color. If we
    // have to have this, it should be invisible."*
    //
    // A material-level assertion (these cells now share a colour id) would be
    // the readout being a function of the thing it debugs -- `cell_colour`
    // tints by several things downstream of the palette, so two cells can
    // agree on their palette entry and still draw differently. So: **two
    // framebuffers of the same world, one where a colony was founded and one
    // where it was not, differing in no pixel over the patch.**
    //
    // The ants are painted out of both arms rather than excluded by
    // arithmetic: an animal standing on the door is a real difference between
    // the two worlds and has nothing to do with whether the ground shows.
    if a.invisible {
        invisible(&mut game, px, py, a.zoom);
        return;
    }
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    if a.screen {
        game.draw(&mut buf, true);
    } else {
        if let Some(player) = &game.world.player {
            game.renderer.follow(player.center(), (WIDTH, HEIGHT), game.world.bounds());
        }
        let touched = game.world.take_touched_chunks();
        game.renderer.draw(&game.world, &game.particles, &touched, &mut buf, (WIDTH, HEIGHT), true);
    }

    // `crop=fit`: a band around the middle of the screen, which is where the
    // camera has just put him.
    let crop = if a.fit { Some((WIDTH / 2 - 140, HEIGHT / 2 - 40, 280, 100)) } else { a.crop };
    let img = present(&buf, crop, a.mag);
    match img.save(&a.out) {
        Ok(()) => println!("  wrote {} ({}x{})", a.out, img.width(), img.height()),
        Err(e) => println!("  could not write {}: {e}", a.out),
    }
}

/// **The two framebuffers, and how many pixels of the patch differ.**
///
/// Both arms come out of **one** world and one binary: the patch is painted,
/// the frame taken, the painted cells put back exactly as they were, and the
/// frame taken again. Two separate runs would be two worlds by the second
/// frame (`CLAUDE.md`), and two binaries would be two builds.
fn invisible(game: &mut Druid, px: i32, py: i32, zoom: i32) {
    use pixel_physics::app::{HEIGHT, WIDTH};
    let before: Vec<(i32, i32, pixel_physics::sim::cell::Cell)> =
        ((px - 64)..=(px + 64)).flat_map(|cx| ((py - 48)..=(py + 48)).map(move |cy| (cx, cy))).map(|(cx, cy)| (cx, cy, game.world.get(cx, cy))).collect();

    // **The haze is switched off for both arms, and the control is why.**
    // The quickening she carries draws an animated aura, so two draws of one
    // unchanged world differ by about a thousand pixels of it -- measured, as
    // a non-zero `off_patch` on the first run of this. That is a renderer that
    // is not a pure function of the world, which is fine for a haze and fatal
    // for a frame comparison: it would put noise the size of the whole
    // question into the control. `druid_aura` owns judging the haze.
    game.renderer.aura = pixel_physics::render::AuraTuning::off();
    let mut founded = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut bare = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    game.world.paint_nest_patch(px, py);
    let painted: Vec<(i32, i32)> = before.iter().filter(|&&(cx, cy, c)| game.world.get(cx, cy).material != c.material).map(|&(cx, cy, _)| (cx, cy)).collect();
    draw_world(game, &mut founded);
    // Put the ground back, cell for cell, and draw the same world again.
    for &(cx, cy, c) in &before {
        game.world.set(cx, cy, c);
    }
    draw_world(game, &mut bare);

    // Only the cells the patch actually took are in question; the rest of the
    // frame is the same world twice and must be identical, which is the
    // control -- without it a zero here could mean the renderer never ran.
    let mut on_patch = 0usize;
    let mut off_patch = 0usize;
    let mut worst = 0i32;
    for (i, (p, q)) in founded.chunks_exact(4).zip(bare.chunks_exact(4)).enumerate() {
        if p == q {
            continue;
        }
        let (sx, sy) = ((i as u32 % WIDTH) as i32, (i as u32 / WIDTH) as i32);
        let cell = game.renderer.screen_to_world(sx, sy);
        let (wx, wy) = cell;
        let here = painted.iter().any(|&(cx, cy)| cx == wx && cy == wy);
        if here {
            on_patch += 1;
        } else {
            off_patch += 1;
        }
        worst = worst.max(p.iter().zip(q).map(|(a, b)| (*a as i32 - *b as i32).abs()).max().unwrap_or(0));
    }
    // **Which cells differ, against what the ground under them was holding.**
    // `cell_colour` darkens ground by held water and gates that on
    // `water_capacity`, which only `soil.ron` opts in to -- so the standing
    // hypothesis for any residue is that a nest cell draws *dry*. This is the
    // number that confirms or kills it, rather than an argument: the water the
    // replaced cell was holding, for the cells that differ and for the cells
    // that do not.
    let water = |cells: &[(i32, i32)]| -> (u16, u16, usize) {
        let vals: Vec<u16> = before.iter().filter(|&&(cx, cy, _)| cells.iter().any(|&(px, py)| px == cx && py == cy)).map(|&(_, _, c)| c.aux()).collect();
        (vals.iter().copied().min().unwrap_or(0), vals.iter().copied().max().unwrap_or(0), vals.len())
    };
    let differing: Vec<(i32, i32)> = painted
        .iter()
        .copied()
        .filter(|&(cx, cy)| {
            game.renderer.world_to_screen(cx, cy).is_some_and(|(sx, sy)| {
                let i = (sy as u32 * WIDTH + sx as u32) as usize * 4;
                founded.get(i..i + 4) != bare.get(i..i + 4)
            })
        })
        .collect();
    let same: Vec<(i32, i32)> = painted.iter().copied().filter(|c| !differing.contains(c)).collect();
    let (dlo, dhi, dn) = water(&differing);
    let (slo, shi, sn) = water(&same);
    println!(
        "  invisible at zoom {zoom}: {} nest cells painted | {on_patch} pixel(s) of the patch differ, worst channel {worst} | {off_patch} elsewhere (the control: the rest of the frame is the same world twice and must be 0)",
        painted.len()
    );
    println!("  held water of the ground replaced: {dn} cell(s) that differ hold {dlo}..{dhi}, {sn} cell(s) that match hold {slo}..{shi}");
    // **What the patch actually paints over**, because "invisible" is against
    // the ground that is there rather than against a material named in a
    // constant. A threshold toned to `soil` is only invisible where the
    // surface *is* soil, and a grown world has litter and root in it.
    let mut kinds: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for &(cx, cy) in &painted {
        if let Some(&(_, _, c)) = before.iter().find(|&&(bx, by, _)| bx == cx && by == cy) {
            *kinds.entry(game.world.materials.get(c.material).name.clone()).or_default() += 1;
        }
    }
    println!("  ground replaced: {kinds:?}");
    println!("  {}", if on_patch == 0 { "THE THRESHOLD IS INVISIBLE" } else { "THE THRESHOLD STILL SHOWS" });
}

fn draw_world(game: &mut Druid, buf: &mut [u8]) {
    use pixel_physics::app::{HEIGHT, WIDTH};
    if let Some(player) = &game.world.player {
        game.renderer.follow(player.center(), (WIDTH, HEIGHT), game.world.bounds());
    }
    let touched = game.world.take_touched_chunks();
    game.renderer.draw(&game.world, &game.particles, &touched, buf, (WIDTH, HEIGHT), true);
}

/// Painted columns of the nest patch, and the lengths of the unbroken runs.
///
/// Scanned over the band `paint_nest_patch` can reach, and down a few rows
/// from the stand so a patch on sloping ground is still found. **It reports
/// `None` as a zero-length run rather than skipping it**, which is what makes
/// the histogram a barcode detector: `[2,2,2,2,...]` is the comb.
fn nest_runs(world: &pixel_physics::sim::world::World, px: i32, py: i32) -> (usize, Vec<usize>) {
    let Some(nest) = world.materials.id_of("nest") else {
        return (0, Vec::new());
    };
    let mut runs = Vec::new();
    let mut run = 0usize;
    let mut cols = 0usize;
    for cx in (px - 64)..=(px + 64) {
        let painted = ((py - 48)..=(py + 48)).any(|cy| world.get(cx, cy).material == nest);
        if painted {
            run += 1;
            cols += 1;
        } else if run > 0 {
            runs.push(run);
            run = 0;
        }
    }
    if run > 0 {
        runs.push(run);
    }
    (cols, runs)
}

/// Crop then nearest-neighbour magnify, so a 53-column patch is judged at
/// something other than a tenth of the card's width.
fn present(buf: &[u8], crop: Option<(u32, u32, u32, u32)>, mag: u32) -> image::RgbaImage {
    let full = image::RgbaImage::from_raw(WIDTH, HEIGHT, buf.to_vec()).expect("frame is WIDTH*HEIGHT*4");
    let (x, y, w, h) = crop.unwrap_or((0, 0, WIDTH, HEIGHT));
    let (x, y) = (x.min(WIDTH - 1), y.min(HEIGHT - 1));
    let (w, h) = (w.min(WIDTH - x), h.min(HEIGHT - y));
    let cut = image::imageops::crop_imm(&full, x, y, w, h).to_image();
    if mag <= 1 {
        return cut;
    }
    image::imageops::resize(&cut, w * mag, h * mag, image::imageops::FilterType::Nearest)
}
