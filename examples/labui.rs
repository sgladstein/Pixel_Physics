//! **A contact sheet of the lab's control bar, in every state it has.**
//!
//! `cargo run --release --example labui`
//!
//! The bar exists to be looked at, and the two states that matter most —
//! hover and pressed — are exactly the two a screenshot of the running binary
//! cannot show, because a framebuffer dump has no pointer in it. So this
//! drives a real `Lab` with synthetic cursor positions and synthetic clicks
//! and lays the results out in a grid.
//!
//! **It aims every click through `Ui::widget_rect`**, which reads the retained
//! layout — so a tile cannot be a picture of a click landing somewhere the
//! button is not. And it prints, beside each tile, whether the click actually
//! *fired*: `CLAUDE.md`'s standing rule is that an image shows what and where
//! and only a count shows whether, and a bar whose buttons paint correctly and
//! do nothing would look identical here without it.
//!
//! `frames=N` warms the box for N ticks first (default 900, enough for the
//! founders to have grown into something the pages have numbers about).

use pixel_physics::lab::ui::{Action, Panel};
use pixel_physics::lab::{scene::LabBox, Lab, HEIGHT, WIDTH};

const COLS: usize = 3;

fn main() {
    let mut frames = 900u32;
    let mut out = "labui.png".to_string();
    for arg in std::env::args().skip(1) {
        match arg.split_once('=') {
            Some(("frames", v)) => frames = v.parse().expect("frames=N"),
            Some(("out", v)) => out = v.to_string(),
            _ => eprintln!("ignoring unknown argument {arg:?}"),
        }
    }

    let mut lab = Lab::new(LabBox::default());
    lab.show_help = false;
    // **Start it before warming it.** A fresh lab is paused and `advance`
    // then runs no ticks at all, so warming a paused box grows nothing and
    // every page draws the numbers of a bed that has been standing still —
    // which is exactly what this sheet looked like the first time the phase
    // model changed under it. Put back to paused afterwards, because paused is
    // the state the bar is being photographed in.
    lab.act(Action::TogglePhase);
    for _ in 0..frames {
        lab.advance(std::time::Duration::from_millis(16));
    }
    let grown = lab.world.frame;
    assert!(grown > 0, "the box never ticked -- the sheet would be of an empty bed");
    lab.act(Action::TogglePhase);
    // One draw before anything is aimed: the layout is retained from the last
    // painted frame, so nothing can be clicked until a frame has been painted.
    let mut warm = blank();
    lab.draw(&mut warm, 60.0);

    let mut tiles: Vec<(String, Vec<u8>)> = Vec::new();
    let mut fired: Vec<String> = Vec::new();

    // 1. Resting, no pointer.
    lab.set_cursor(None);
    tiles.push(("BAR AT REST".into(), shot(&mut lab)));

    // 2. Hover over a page button, which is also the hover-explanation case.
    let plants = centre(&lab, Action::Panel(Panel::Plants));
    lab.set_cursor(Some(plants));
    tiles.push(("HOVER: PLANTS".into(), shot(&mut lab)));

    // 3. Pressed but not released. The gesture is armed and nothing has
    //    happened yet, which is the whole point of firing on release.
    let rebuild = centre(&lab, Action::Reset);
    lab.set_cursor(Some(rebuild));
    lab.press(rebuild.0, rebuild.1);
    let frame_before = lab.world.frame;
    tiles.push(("PRESSED: REBUILD".into(), shot(&mut lab)));
    fired.push(format!(
        "press alone rebuilt the box: {} (world frame {} -> {})",
        lab.world.frame < frame_before,
        frame_before,
        lab.world.frame
    ));
    // Slide off and release: the gesture must be taken back.
    lab.set_cursor(Some(plants));
    lab.release(plants.0, plants.1);
    fired.push(format!(
        "release off the button rebuilt the box: {} (world frame {})",
        lab.world.frame < frame_before,
        lab.world.frame
    ));
    lab.ui.toggle_panel(Panel::Plants); // undo the release that landed here

    // 4-6. Each page, opened by a real click on its own button.
    for (panel, action) in [
        (Panel::Plants, Action::Panel(Panel::Plants)),
        (Panel::Ants, Action::Panel(Panel::Ants)),
        (Panel::Box, Action::Panel(Panel::Box)),
    ] {
        if let Some(open) = lab.ui.panel {
            let at = centre(&lab, Action::Panel(open));
            click(&mut lab, at);
        }
        let at = centre(&lab, action);
        click(&mut lab, at);
        fired.push(format!("click opened {panel:?}: {}", lab.ui.panel == Some(panel)));
        // Hover the second row, so the tile shows a page *and* the explanation
        // the owner asked every label to carry.
        lab.set_cursor(Some((at.0 + 20, pixel_physics::lab::ui::bar_top() - 60)));
        tiles.push((format!("PAGE: {panel:?}"), shot(&mut lab)));
    }
    if let Some(open) = lab.ui.panel {
        let at = centre(&lab, Action::Panel(open));
        click(&mut lab, at);
    }

    // 7. The speed ladder latched, with the achieved readout beside it.
    let fast = centre(&lab, Action::Preset(4));
    click(&mut lab, fast);
    for _ in 0..30 {
        lab.advance(std::time::Duration::from_millis(16));
    }
    fired.push(format!("click set the dial to {}x", lab.time.requested));
    lab.set_cursor(Some(fast));
    tiles.push((format!("DIAL AT {}X", lab.time.requested), shot(&mut lab)));
    let slow = centre(&lab, Action::Preset(0));
    click(&mut lab, slow);

    // 8. The inspector, aimed at a cell an organism actually owns.
    let target = living_cell(&lab);
    match target {
        Some((wx, wy)) => {
            let (sx, sy) = lab.renderer.world_to_screen(wx, wy).unwrap_or((wx, wy));
            lab.set_cursor(Some((sx, sy)));
            click(&mut lab, (sx, sy));
            fired.push(format!(
                "click on ({wx},{wy}) opened the inspector: {}",
                lab.ui.inspecting() == Some((wx, wy))
            ));
            lab.set_cursor(Some((60, pixel_physics::lab::ui::bar_top() - 40)));
            tiles.push(("INSPECT A LIVING CELL".into(), shot(&mut lab)));
        }
        None => {
            fired.push("no organism cell found to inspect".into());
            tiles.push(("INSPECT: NO ORGANISM".into(), shot(&mut lab)));
        }
    }

    // 9. A click that lands on the bar must not also inspect the cell behind
    //    it. Nothing to see; the count is the whole result.
    let before = lab.ui.inspecting();
    let on_bar = centre(&lab, Action::Stats);
    click(&mut lab, on_bar);
    fired.push(format!(
        "a click on the bar also moved the inspector: {}",
        lab.ui.inspecting() != before
    ));
    lab.set_cursor(None);
    // Named from the state the click produced, not from what it was expected
    // to produce: the pages are mutually exclusive with the biosphere page, so
    // whether this click turns it on or off depends on what was open before.
    let stats_now = if lab.stats.showing() { "ON" } else { "OFF" };
    tiles.push((format!("STATS TOGGLED {stats_now}"), shot(&mut lab)));

    for line in &fired {
        println!("{line}");
    }
    // The bar's own coordinates, so a headless run of the *real* binary can be
    // aimed with `PIXEL_PHYSICS_LAB_CLICK` at where a button actually is
    // rather than at where it looks like it is in a screenshot.
    for action in [
        Action::TogglePhase,
        Action::Slower,
        Action::Faster,
        Action::Preset(0),
        Action::Preset(5),
        Action::Panel(Panel::Plants),
        Action::Panel(Panel::Ants),
        Action::Panel(Panel::Box),
        Action::Stats,
        Action::Help,
        Action::Reset,
    ] {
        if let Some(r) = lab.ui.widget_rect(action) {
            println!("{action:?} at {},{} ({}x{})", r.x + r.w / 2, r.y + r.h / 2, r.w, r.h);
        }
    }
    println!(
        "warmed to frame {grown}: organisms {} creatures {}",
        lab.world.live_organism_count(),
        lab.world.live_creature_count()
    );

    write_sheet(&tiles, &out);
    println!("wrote {out}");
}

fn blank() -> Vec<u8> {
    vec![0u8; (WIDTH * HEIGHT * 4) as usize]
}

fn shot(lab: &mut Lab) -> Vec<u8> {
    let mut frame = blank();
    lab.draw(&mut frame, 60.0);
    frame
}

/// The middle of the button for `action`, from the retained layout.
fn centre(lab: &Lab, action: Action) -> (i32, i32) {
    let r = lab.ui.widget_rect(action).expect("the bar has no button for this action");
    (r.x + r.w / 2, r.y + r.h / 2)
}

fn click(lab: &mut Lab, (x, y): (i32, i32)) {
    lab.set_cursor(Some((x, y)));
    lab.press(x, y);
    lab.release(x, y);
}

/// Any cell an organism owns, preferring one an animal owns — an ant is the
/// case the inspector exists for.
fn living_cell(lab: &Lab) -> Option<(i32, i32)> {
    let bounds = lab.world.bounds()?;
    let mut plant = None;
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let cell = lab.world.get(x, y);
            let Some(state) = lab.world.organism(cell.organism_id()) else { continue };
            if lab.world.species.get(state.species).creature.is_some() {
                return Some((x, y));
            }
            plant.get_or_insert((x, y));
        }
    }
    plant
}

fn write_sheet(tiles: &[(String, Vec<u8>)], out: &str) {
    const LABEL: u32 = 12;
    let cell_h = HEIGHT + LABEL;
    let rows = tiles.len().div_ceil(COLS) as u32;
    let (sw, sh) = (WIDTH * COLS as u32, cell_h * rows);
    let mut sheet = vec![24u8; (sw * sh * 4) as usize];
    for p in sheet.chunks_exact_mut(4) {
        p[3] = 255;
    }
    for (i, (title, frame)) in tiles.iter().enumerate() {
        let (cx, cy) = ((i % COLS) as u32 * WIDTH, (i / COLS) as u32 * cell_h);
        pixel_physics::hud::draw_text(
            &mut sheet,
            sw,
            sh,
            cx as i32 + 4,
            cy as i32 + 3,
            title,
            [235, 235, 200, 255],
        );
        for y in 0..HEIGHT {
            let src = (y * WIDTH * 4) as usize;
            let dst = (((cy + LABEL + y) * sw + cx) * 4) as usize;
            sheet[dst..dst + (WIDTH * 4) as usize]
                .copy_from_slice(&frame[src..src + (WIDTH * 4) as usize]);
        }
    }
    image::save_buffer(out, &sheet, sw, sh, image::ColorType::Rgba8).expect("write the sheet");
}
