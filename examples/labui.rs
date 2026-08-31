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

use pixel_physics::lab::ui::{Action, Panel, Tool};
use pixel_physics::lab::{scene::LabBox, Lab, HEIGHT, WIDTH};

const COLS: usize = 3;

fn main() {
    let mut frames = 900u32;
    let mut out = "labui.png".to_string();
    let mut split = false;
    let mut only: Option<String> = None;
    for arg in std::env::args().skip(1) {
        match arg.split_once('=') {
            Some(("frames", v)) => frames = v.parse().expect("frames=N"),
            Some(("out", v)) => out = v.to_string(),
            // **One tile per file, at full size.** A twenty-tile sheet is
            // 1536 pixels wide and every viewer that opens it scales it down,
            // which is exactly the wrong thing to do to a page of 5x7 glyphs:
            // the sheet answers "is the bar right" and cannot answer "can you
            // read this row". `only=` picks the tiles whose title contains it.
            Some(("split", v)) => split = v == "1",
            Some(("only", v)) => only = Some(v.to_string()),
            _ => eprintln!("ignoring unknown argument {arg:?}"),
        }
    }

    // **A scratch shelf, before the `Lab` is built** — `Lab::new` reads the
    // rack, and `KEEP` below writes to it. Without this the sheet would drop
    // jars into `assets/shelf` in the working tree every time it ran, and
    // would also photograph whatever a previous run had left there.
    let shelf = std::env::temp_dir().join(format!("pixel_physics_labui_shelf_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&shelf);
    std::env::set_var(pixel_physics::sim::specimen::SHELF_DIR_ENV, &shelf);

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

    // 10-15. **Gate 4's verbs**, each fired by a real click on its own button
    //     followed by a real click on the world, with the count beside it.
    //     `CLAUDE.md`: an image says what and where, and only a count says
    //     whether it fired -- a seed is one cell and an ant is two, so every
    //     one of these looks identical to nothing happening in a still frame.
    lab.ui.panel = None;
    // **The bed's surface, from the spec rather than found by scanning.**
    // Scanning down from the top of the screen for the first non-empty cell
    // was the first version and it found the box's *ceiling*: every verb below
    // then aimed 150 rows above the soil, every counter read zero, and the
    // sheet would have been six tiles of nothing captioned as six working
    // verbs. `CLAUDE.md`: a scene that contradicts the code looks like a bug
    // in the code.
    let ground = |lab: &Lab| -> (i32, i32) {
        let (wx, wy) = (lab.spec.width / 2, lab.spec.ground_y);
        lab.renderer.world_to_screen(wx, wy).expect("the bed's surface is on screen")
    };

    // PLANT: pick the tool, name the species, put one in.
    let at = centre(&lab, Action::Tool(Tool::Plant));
    click(&mut lab, at);
    let before = lab.world.live_organism_count();
    let (gx, gy) = ground(&lab);
    click(&mut lab, (gx, gy));
    fired.push(format!(
        "PLANT: organisms {} -> {} (tool {:?})",
        before,
        lab.world.live_organism_count(),
        lab.ui.tool()
    ));
    lab.set_cursor(Some((gx, gy - 6)));
    tiles.push(("VERB: PLANT".into(), shot(&mut lab)));

    // COLONY.
    let at = centre(&lab, Action::Tool(Tool::Colony));
    click(&mut lab, at);
    let before = lab.world.live_creature_count();
    click(&mut lab, (gx + 40, gy));
    fired.push(format!(
        "COLONY: creatures {} -> {}",
        before,
        lab.world.live_creature_count()
    ));
    lab.set_cursor(Some((gx + 40, gy - 6)));
    tiles.push(("VERB: COLONY".into(), shot(&mut lab)));

    // CULL, both kingdoms, because they die by different paths and a sheet
    // that only showed one would say nothing about the other. A plant is
    // marked senescent and **keeps its cells** -- that is the graded death;
    // an ant has no senescence path at all, so its energy goes to zero and
    // the next tick writes a corpse.
    let at = centre(&lab, Action::Tool(Tool::Cull));
    click(&mut lab, at);
    for (what, cell) in [("PLANT", living_cell_of(&lab, false)), ("ANT", living_cell_of(&lab, true))] {
        let Some((wx, wy)) = cell else {
            fired.push(format!("CULL {what}: nothing alive to aim at"));
            continue;
        };
        let id = lab.world.get(wx, wy).organism_id();
        let before = lab.world.organism_state(id).map(|s| (s.senescent, s.cells.len(), s.energy));
        let (sx, sy) = lab.renderer.world_to_screen(wx, wy).unwrap_or((wx, wy));
        click(&mut lab, (sx, sy));
        let after = lab.world.organism_state(id).map(|s| (s.senescent, s.cells.len(), s.energy));
        fired.push(format!(
            "CULL {what}: (senescent, cells, energy) {before:?} -> {after:?}"
        ));
        lab.set_cursor(Some((sx, sy)));
    }
    tiles.push(("VERB: CULL".into(), shot(&mut lab)));
    // ...and the ant's death needs a tick to land, which is the honest half:
    // a stopped box does not kill anything, it only marks it.
    let creatures_before = lab.world.live_creature_count();
    let frame_before = lab.world.frame;
    // **Start it, do not toggle it.** A blind `TogglePhase` here stopped a box
    // that was already running, and the arm then reported 88 -> 88 with the
    // world at a standstill -- a null that looks exactly like "the cull does
    // nothing". The frame count beside it is what says the box actually ran.
    if lab.time.phase != pixel_physics::lab::time::Phase::Running {
        lab.act(Action::TogglePhase);
    }
    for _ in 0..90 {
        lab.advance(std::time::Duration::from_millis(16));
    }
    lab.act(Action::TogglePhase);
    let corpse = lab.world.materials.id_of("corpse").expect("corpse");
    let corpses: u32 = {
        let b = lab.world.bounds().expect("bounds");
        (b.min_y..=b.max_y)
            .flat_map(|y| (b.min_x..=b.max_x).map(move |x| (x, y)))
            .map(|(x, y)| u32::from(lab.world.get(x, y).material == corpse))
            .sum()
    };
    fired.push(format!(
        "CULLED ANT, {} FRAMES LATER: creatures {creatures_before} -> {}, {corpses} corpse cells standing",
        lab.world.frame - frame_before,
        lab.world.live_creature_count()
    ));

    // SOIL and WATER: a real drag, and the two `aux` conventions read back.
    let at = centre(&lab, Action::Tool(Tool::Soil));
    click(&mut lab, at);
    for _ in 0..3 {
        let at = centre(&lab, Action::Brush(1));
    click(&mut lab, at);
    }
    let (px, py) = (gx - 120, gy - 40);
    lab.set_cursor(Some((px, py)));
    lab.press(px, py);
    for step in 1..=40 {
        lab.set_cursor(Some((px + step, py)));
        lab.drag(px + step, py);
    }
    lab.release(px + 40, py);
    let soil = lab.world.materials.id_of("soil").expect("soil");
    let (mut soil_cells, mut damp) = (0u32, 0u32);
    for y in py - 20..py + 20 {
        for x in px - 20..px + 70 {
            let (wx, wy) = lab.renderer.screen_to_world(x, y);
            let cell = lab.world.get(wx, wy);
            if cell.material == soil {
                soil_cells += 1;
                damp += u32::from(cell.aux() > 0);
            }
        }
    }
    fired.push(format!(
        "SOIL BRUSH: {soil_cells} cells laid down, {damp} of them damp (aux 0 on a POWDER is DRY)"
    ));
    tiles.push(("BRUSH: SOIL".into(), shot(&mut lab)));

    let at = centre(&lab, Action::Tool(Tool::Water));
    click(&mut lab, at);
    let (wx0, wy0) = (gx + 90, gy - 50);
    lab.set_cursor(Some((wx0, wy0)));
    lab.press(wx0, wy0);
    for step in 1..=30 {
        lab.set_cursor(Some((wx0 + step, wy0)));
        lab.drag(wx0 + step, wy0);
    }
    lab.release(wx0 + 30, wy0);
    let water = lab.world.materials.id_of("water").expect("water");
    let (mut cells, mut full) = (0u32, 0u32);
    for y in wy0 - 20..wy0 + 20 {
        for x in wx0 - 20..wx0 + 60 {
            let (a, b) = lab.renderer.screen_to_world(x, y);
            let cell = lab.world.get(a, b);
            if cell.material == water {
                cells += 1;
                full += u32::from(
                    pixel_physics::sim::update::liquid_fill(cell)
                        == pixel_physics::sim::material::LIQUID_FULL,
                );
            }
        }
    }
    fired.push(format!(
        "WATER BRUSH: {cells} cells laid down, {full} of them full (aux 0 on a LIQUID is FULL)"
    ));
    tiles.push(("BRUSH: WATER".into(), shot(&mut lab)));

    // The eraser, on the right button, over the water that was just laid.
    lab.press_erase(wx0 + 10, wy0);
    for step in 11..=30 {
        lab.set_cursor(Some((wx0 + step, wy0)));
        lab.drag(wx0 + step, wy0);
    }
    lab.end_stroke();
    let mut left = 0u32;
    for y in wy0 - 20..wy0 + 20 {
        for x in wx0 - 20..wx0 + 60 {
            let (a, b) = lab.renderer.screen_to_world(x, y);
            left += u32::from(lab.world.get(a, b).material == water);
        }
    }
    fired.push(format!("RIGHT-DRAG ERASE: water {cells} -> {left}"));
    tiles.push(("ERASE (RIGHT BUTTON)".into(), shot(&mut lab)));

    // **KEEP, the shelf, and FREE** — the three halves of the specimen rack,
    // each fired by a real click and each with its counter beside it. A jar
    // is a file the sheet cannot show, a freed ant is two dark cells, and a
    // clone and a four-brood release look identical at this zoom: every one
    // of these tiles is a picture that says nothing on its own.
    let at = centre(&lab, Action::Tool(Tool::Keep));
    click(&mut lab, at);
    let kept = living_cell_of(&lab, false).or_else(|| living_cell_of(&lab, true));
    match kept {
        Some((wx, wy)) => {
            let (sx, sy) = lab.renderer.world_to_screen(wx, wy).unwrap_or((wx, wy));
            let before = lab.ui.shelf().len();
            click(&mut lab, (sx, sy));
            fired.push(format!(
                "KEEP: jars {before} -> {} -- {:?}",
                lab.ui.shelf().len(),
                lab.ui.notice_text().unwrap_or_default()
            ));
            lab.set_cursor(Some((sx, sy)));
        }
        None => fired.push("KEEP: nothing alive to aim at".into()),
    }
    tiles.push(("VERB: KEEP".into(), shot(&mut lab)));

    // The rack itself, opened by a real click on its own chip, with a jar
    // row hovered so the tile carries the page *and* its explanation.
    let at = centre(&lab, Action::Panel(Panel::Shelf));
    click(&mut lab, at);
    let _ = shot(&mut lab);
    fired.push(format!("click opened THE SHELF: {}", lab.ui.panel == Some(Panel::Shelf)));
    if let Some(r) = lab.ui.widget_rect(Action::ShelfSelect(0)) {
        lab.set_cursor(Some((r.x + 20, r.y + 4)));
    }
    tiles.push(("PAGE: THE SHELF".into(), shot(&mut lab)));

    // Arm the first jar by clicking its row, turn the dial up, and read back
    // that both actually took.
    if lab.ui.widget_rect(Action::ShelfSelect(0)).is_some() {
        let at = centre(&lab, Action::ShelfSelect(0));
        click(&mut lab, at);
        let _ = shot(&mut lab);
        fired.push(format!(
            "CLICKING A JAR ARMED IT: {:?}, tool now {:?}",
            lab.ui.armed_jar().map(|j| j.name.clone()),
            lab.ui.tool()
        ));
        for _ in 0..3 {
            let at = centre(&lab, Action::Broods(1));
            click(&mut lab, at);
            let _ = shot(&mut lab);
        }
        fired.push(format!("BROOD DIAL: {}", lab.ui.brood_label()));
        tiles.push((format!("SHELF: {}", lab.ui.brood_label()), shot(&mut lab)));

        // ...and DRIFT, which breeds the jar without releasing it.
        let before = lab.ui.shelf().len();
        let at = centre(&lab, Action::ShelfDrift);
        click(&mut lab, at);
        let _ = shot(&mut lab);
        fired.push(format!(
            "DRIFT: jars {before} -> {} -- {:?}",
            lab.ui.shelf().len(),
            lab.ui.notice_text().unwrap_or_default()
        ));
        tiles.push(("SHELF: AFTER DRIFT".into(), shot(&mut lab)));
    }

    // FREE, into the bed. **The count is the whole tile**: what lands is one
    // seed or a two-cell body, and the notice carries how many genome slots
    // the dial moved, which is the only thing that separates a clone from a
    // three-brood descendant on screen.
    let before = lab.world.live_organism_count();
    click(&mut lab, (gx + 20, gy));
    fired.push(format!(
        "FREE: organisms {before} -> {} -- {:?}",
        lab.world.live_organism_count(),
        lab.ui.notice_text().unwrap_or_default()
    ));
    lab.set_cursor(Some((gx + 20, gy - 6)));
    tiles.push(("VERB: FREE".into(), shot(&mut lab)));
    if lab.ui.panel == Some(Panel::Shelf) {
        let at = centre(&lab, Action::Panel(Panel::Shelf));
        click(&mut lab, at);
    }

    // The overlay, named on its own face.
    let at = centre(&lab, Action::Tool(Tool::Look));
    click(&mut lab, at);
    for _ in 0..5 {
        let at = centre(&lab, Action::CycleOverlay);
    click(&mut lab, at);
    }
    fired.push(format!("OVERLAY: {}", lab.renderer.field_overlay.label()));
    let at = centre(&lab, Action::CycleOverlay);
    lab.set_cursor(Some(at));
    tiles.push((format!("OVERLAY {}", lab.renderer.field_overlay.label()), shot(&mut lab)));
    for _ in 0..2 {
        let at = centre(&lab, Action::CycleOverlay);
    click(&mut lab, at);
    }

    // **The parameters page**, and the count beside every tile is the point:
    // a row that paints its figure correctly and refuses to move looks
    // identical here to one that works. So every arm below reads the value
    // back out of the registry after the click and prints both.
    //
    // `draw` between the click and the aim, every time: the page's rectangles
    // are retained from the last painted frame — deliberately, so a click is
    // tested against what the player was looking at — so a harness that
    // clicked and immediately aimed would be aiming at the page before it
    // existed.
    let at = centre(&lab, Action::Panel(Panel::Params));
    click(&mut lab, at);
    let _ = shot(&mut lab);
    fired.push(format!("click opened PARAMS: {}", lab.ui.panel == Some(Panel::Params)));

    let (mut moved, mut stuck): (u32, Vec<String>) = (0, Vec::new());
    let mut at_ceiling: Vec<String> = Vec::new();
    for (i, group) in pixel_physics::lab::params::GROUPS.iter().enumerate() {
        let at = centre(&lab, Action::ParamGroup(i));
        click(&mut lab, at);
        let _ = shot(&mut lab);
        let list = lab.ui.page_params(&lab.world, &lab.spec);
        fired.push(format!(
            "PARAMS page {}: {} rows, {} writable, {} shown-only",
            group.label(),
            list.len(),
            list.iter().filter(|p| p.writable()).count(),
            list.iter().filter(|p| !p.writable()).count()
        ));

        // A row hovered, so the tile carries a page *and* the explanation the
        // owner asked every label to have. **Before anything is adjusted**:
        // these tiles are the page as it ships, and a sheet of values a
        // harness had just walked upward would be a picture of the harness.
        if let Some(r) = lab.ui.widget_rect(Action::ParamAdjust(2, 1)).or_else(|| lab.ui.widget_rect(Action::ParamAdjust(0, 1))) {
            lab.set_cursor(Some((r.x - 60, r.y + 4)));
        }
        tiles.push((format!("PARAMS: {}", group.label()), shot(&mut lab)));
    }
    lab.set_cursor(None);

    // **Now** every writable row on every page, moved by a real click on its
    // own `+` face, and the value read back. This runs after the tiles above
    // because it walks every number in the box upward, and a picture of that
    // is a picture of the harness rather than of the game.
    for (i, group) in pixel_physics::lab::params::GROUPS.iter().enumerate() {
        let at = centre(&lab, Action::ParamGroup(i));
        click(&mut lab, at);
        let _ = shot(&mut lab);
        // Every writable row, moved by a real click on its own `+` face, in
        // as many pagefuls as the page has. `params::write` returning `false`
        // is a knob with a reader and no writer — the failure that looks
        // exactly like working code from outside — so the names of any that
        // did not move are printed rather than counted.
        let mut seen = 0usize;
        loop {
            let mut aimed = false;
            for row in 0..lab.ui.page_params(&lab.world, &lab.spec).len() {
                let Some(r) = lab.ui.widget_rect(Action::ParamAdjust(row, 1)) else { continue };
                aimed = true;
                seen += 1;
                let before = lab.ui.page_params(&lab.world, &lab.spec)[row].display();
                click(&mut lab, (r.x + r.w / 2, r.y + r.h / 2));
                let _ = shot(&mut lab);
                let after = lab.ui.page_params(&lab.world, &lab.spec)[row].display();
                let p = &lab.ui.page_params(&lab.world, &lab.spec)[row];
                // **A row already at its ceiling is a clamp, not a stuck
                // knob**, and calling it stuck would be a false positive that
                // hides a real one. `water.flow_rate` ships at 1000 of 1000.
                let clamped = p.tunable.value >= p.tunable.max;
                if before == after && !clamped {
                    stuck.push(format!("{}.{} stuck at {before}", p.tunable.category, p.tunable.name));
                } else if before != after {
                    moved += 1;
                } else {
                    at_ceiling.push(format!("{}.{}", p.tunable.category, p.tunable.name));
                }
            }
            let Some(down) = lab.ui.widget_rect(Action::ParamScroll(1)) else { break };
            let was = lab.ui.param_scroll();
            click(&mut lab, (down.x + down.w / 2, down.y + down.h / 2));
            let _ = shot(&mut lab);
            if lab.ui.param_scroll() == was || !aimed {
                break;
            }
        }

        // Back to the top, then hover a row so the tile carries a page *and*
        // the explanation the owner asked every label to have.
        while lab.ui.param_scroll() > 0 {
            let Some(up) = lab.ui.widget_rect(Action::ParamScroll(-1)) else { break };
            click(&mut lab, (up.x + up.w / 2, up.y + up.h / 2));
            let _ = shot(&mut lab);
        }
        if let Some(r) = lab.ui.widget_rect(Action::ParamAdjust(2, 1)).or_else(|| lab.ui.widget_rect(Action::ParamAdjust(0, 1))) {
            lab.set_cursor(Some((r.x - 60, r.y + 4)));
        }
        fired.push(format!("PARAMS page {}: {seen} clicks on a +/- face landed on one", group.label()));
    }
    fired.push(format!(
        "PARAMS: {moved} rows moved by their own + face; already at their ceiling: {at_ceiling:?}; STUCK: {stuck:?}"
    ));

    // The save path, **reported and not run**: it writes into `assets/`, and a
    // contact-sheet harness that edited the repository's own species files
    // would be a harness nobody could run twice.
    for (i, _) in pixel_physics::lab::params::GROUPS.iter().enumerate() {
        let at = centre(&lab, Action::ParamGroup(i));
        click(&mut lab, at);
        let _ = shot(&mut lab);
        for p in lab.ui.page_params(&lab.world, &lab.spec) {
            if !p.writable() {
                continue;
            }
            // A dry run: everything `save` checks except the write itself.
            fired.push(format!("SAVEABLE {}.{}: {}", p.tunable.category, p.tunable.name, pixel_physics::lab::params::save_check(&p)));
        }
    }

    let at = centre(&lab, Action::Panel(Panel::Params));
    click(&mut lab, at);
    let _ = shot(&mut lab);

    // **The specimen**, both kingdoms. The cell page grows the individual's
    // own rows under the cell's, and the two kingdoms carry different state —
    // a plant has a genotype and an allele set, an animal has body traits and
    // an errand — so one tile could not show the feature.
    let at = centre(&lab, Action::Tool(Tool::Look));
    click(&mut lab, at);
    for (what, animal) in [("PLANT", false), ("ANT", true)] {
        let Some((wx, wy)) = living_cell_of(&lab, animal) else {
            fired.push(format!("SPECIMEN {what}: nothing alive to aim at"));
            continue;
        };
        let rows = pixel_physics::lab::params::specimen_rows(&lab.world, lab.world.get(wx, wy).organism_id());
        fired.push(format!("SPECIMEN {what} at ({wx},{wy}): {} rows -- {}", rows.len(),
            rows.iter().map(|(l, v, _)| format!("{l} {v}")).collect::<Vec<_>>().join(", ")));
        let (sx, sy) = lab.renderer.world_to_screen(wx, wy).unwrap_or((wx, wy));
        if lab.ui.inspecting().is_some() {
            let prev = lab.ui.inspecting().unwrap();
            let (px, py) = lab.renderer.world_to_screen(prev.0, prev.1).unwrap_or(prev);
            click(&mut lab, (px, py));
        }
        click(&mut lab, (sx, sy));
        let _ = shot(&mut lab);
        // Hover one of the individual's own rows, so the tile carries the
        // explanation as well as the figure.
        if let Some(r) = lab.ui.inspect_rect() {
            lab.set_cursor(Some((r.x + 20, r.y + 60)));
        }
        tiles.push((format!("SPECIMEN: {what}"), shot(&mut lab)));
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
        Action::Panel(Panel::Params),
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

    let tiles: Vec<(String, Vec<u8>)> = match &only {
        Some(want) => tiles.into_iter().filter(|(t, _)| t.contains(want.as_str())).collect(),
        None => tiles,
    };
    if split {
        let stem = out.strip_suffix(".png").unwrap_or(&out);
        for (title, frame) in &tiles {
            let name = format!("{stem}-{}.png", title.to_lowercase().replace([' ', ':', '/'], "-"));
            image::save_buffer(&name, frame, WIDTH, HEIGHT, image::ColorType::Rgba8).expect("write a tile");
            println!("wrote {name}");
        }
        return;
    }
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

/// A cell owned by a live organism of the asked-for kingdom.
fn living_cell_of(lab: &Lab, animal: bool) -> Option<(i32, i32)> {
    let bounds = lab.world.bounds()?;
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let cell = lab.world.get(x, y);
            let Some(state) = lab.world.organism(cell.organism_id()) else { continue };
            if state.senescent {
                continue;
            }
            if lab.world.species.get(state.species).creature.is_some() == animal {
                return Some((x, y));
            }
        }
    }
    None
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
