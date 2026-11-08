//! **Drawing the mine.** One function, [`paint`], used by the window and by
//! the harness alike, so a GIF from `examples/firedampprobe.rs` is the
//! window's own pixels.
//!
//! **The gas is three fixed colours over dark rock, never a blend**: pale
//! violet for too thin, yellow for will burn, deep green for too thick. That
//! is the readability gate the brief sets, and the reason the rock is drawn
//! at under half its engine brightness.

use crate::hud;
use crate::sim::material::{self, MaterialKind};

use super::gas::{self, coin, idx, Band, H, W};
use super::{BangSize, FaceState, Game, Outcome, Preview, Risk, Tool, LEVEL_TICKS, TICKS_PER_SECOND};

/// Screen pixels per cell.
pub const S: usize = 4;
/// Four lines under the tool row: the tool, the pointer, and a two-line
/// message. At three lines the pointer's reading was cut off mid-word.
pub const HUD_H: usize = 118;
pub const FW: usize = W * S;
pub const FH: usize = H * S + HUD_H;

pub const THIN: [u8; 3] = [128, 108, 178];
pub const BURN: [u8; 3] = [244, 206, 48];
pub const THICK: [u8; 3] = [36, 150, 70];
const AIR: [u8; 3] = [13, 12, 17];
const DOOR: [u8; 3] = [158, 108, 52];
const TEXT: [u8; 4] = [235, 230, 220, 255];
const DIM: [u8; 4] = [150, 145, 140, 255];

/// Tool buttons in the bar: (tool, x, y, w, h) in screen pixels.
pub fn buttons() -> [(Tool, i32, i32, i32, i32); 4] {
    let y = (H * S) as i32 + 6;
    [0, 1, 2, 3].map(|k| (Tool::ALL[k], 8 + k as i32 * 150, y, 142, 26))
}

pub fn button_at(px: i32, py: i32) -> Option<Tool> {
    buttons().iter().find(|b| px >= b.1 && px < b.1 + b.3 && py >= b.2 && py < b.2 + b.4).map(|b| b.0)
}

pub fn tool_sentence(t: Tool) -> &'static str {
    match t {
        Tool::Fan => "FAN: PRESS AND DRAG TO AIM. IT PUSHES AND STIRS THE GAS.",
        Tool::Door => "DOOR: CLICK A TUNNEL TO HANG ONE. CLICK A DOOR TO SLAM IT.",
        Tool::Match => "MATCH: DROPS DOWN AND LIGHTS THE FIRST GAS THAT CAN BURN.",
        Tool::Fuse => "FUSE: LIGHTS BY ITSELF WHEN THE GAS ON IT CAN BURN.",
    }
}

fn put(frame: &mut [u8], x: i32, y: i32, c: [u8; 3]) {
    if x < 0 || y < 0 || x as usize >= FW || y as usize >= FH {
        return;
    }
    let i = (y as usize * FW + x as usize) * 4;
    frame[i..i + 3].copy_from_slice(&c);
    frame[i + 3] = 255;
}

fn rect(frame: &mut [u8], x: i32, y: i32, w: i32, h: i32, c: [u8; 3]) {
    for yy in y..y + h {
        for xx in x..x + w {
            put(frame, xx, yy, c);
        }
    }
}

fn outline(frame: &mut [u8], x: i32, y: i32, w: i32, h: i32, c: [u8; 3]) {
    for xx in x..x + w {
        put(frame, xx, y, c);
        put(frame, xx, y + h - 1, c);
    }
    for yy in y..y + h {
        put(frame, x, yy, c);
        put(frame, x + w - 1, yy, c);
    }
}

fn disc(frame: &mut [u8], cx: i32, cy: i32, r: i32, c: [u8; 3]) {
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                put(frame, cx + dx, cy + dy, c);
            }
        }
    }
}

fn line(frame: &mut [u8], (x0, y0): (i32, i32), (x1, y1): (i32, i32), thick: i32, c: [u8; 3]) {
    let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for k in 0..=n {
        let x = x0 + (x1 - x0) * k / n;
        let y = y0 + (y1 - y0) * k / n;
        rect(frame, x - thick / 2, y - thick / 2, thick, thick, c);
    }
}

/// Text at a screen pixel position (the crisp font is addressed in half
/// pixels).
pub fn text(frame: &mut [u8], x: i32, y: i32, s: &str, c: [u8; 4]) {
    hud::draw_text_crisp(frame, (FW as u32, FH as u32), (x / 2, y / 2), s, c);
}

fn text_w(s: &str) -> i32 {
    s.chars().count() as i32 * 12
}

/// Text with a dark backing, so a label over rock or gas stays legible.
fn label(frame: &mut [u8], x: i32, y: i32, s: &str, c: [u8; 4]) {
    let w = text_w(s);
    shade_rect(frame, x - 3, y - 2, w + 6, 18, 0.35);
    text(frame, x, y, s, c);
}

fn shade_rect(frame: &mut [u8], x: i32, y: i32, w: i32, h: i32, keep: f32) {
    for yy in y.max(0)..(y + h).min(FH as i32) {
        for xx in x.max(0)..(x + w).min(FW as i32) {
            let i = (yy as usize * FW + xx as usize) * 4;
            for k in 0..3 {
                frame[i + k] = (frame[i + k] as f32 * keep) as u8;
            }
        }
    }
}

fn star(frame: &mut [u8], cx: i32, cy: i32, r: i32, filled: bool) {
    let c = if filled { [250, 210, 60] } else { [70, 66, 60] };
    for dy in -r..=r {
        for dx in -r..=r {
            // A four-point star: |dx|^.5 + |dy|^.5 <= r^.5
            let v = (dx.abs() as f32).sqrt() + (dy.abs() as f32).sqrt();
            if v <= (r as f32).sqrt() * 1.15 {
                put(frame, cx + dx, cy + dy, c);
            }
        }
    }
}

fn wrap(s: &str, cols: usize) -> Vec<String> {
    let mut lines = vec![String::new()];
    for word in s.split(' ') {
        let cur = lines.last_mut().unwrap();
        if !cur.is_empty() && cur.len() + 1 + word.len() > cols {
            lines.push(word.to_string());
        } else {
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
        }
    }
    lines
}

fn cell_colour(g: &Game, x: i32, y: i32, glow: &[f32]) -> [u8; 3] {
    let i = idx(x, y);
    let gs = &g.gas;
    if gs.burn[i] > 0 {
        return if coin(g.seed, g.tick / 3, i) > 0.5 { [255, 236, 170] } else { [255, 150, 40] };
    }
    let cell = g.world.get(x, y);
    let mat = g.world.materials.get(cell.material);
    // Doors: drawn over whatever is there.
    for d in &g.doors {
        if d.cells.contains(&i) {
            if d.closed {
                let plank = if d.vertical { (y / 3) % 2 == 0 } else { (x / 3) % 2 == 0 };
                return if plank { DOOR } else { [128, 86, 40] };
            } else if d.vertical && (y == d.bbox.1 || y == d.bbox.3) || !d.vertical && (x == d.bbox.0 || x == d.bbox.2) {
                return [110, 76, 40];
            }
        }
    }
    let open = cell.is_empty() || mat.kind == MaterialKind::Gas;
    if open {
        if gs.open[i] {
            match gas::band(gs.c[i]) {
                Band::Thin => return THIN,
                Band::Burnable => return BURN,
                Band::Thick => return THICK,
                Band::Clear => {}
            }
        }
        if gs.ember[i] > 0 {
            let t = gs.ember[i] as f32 / gas::EMBER_TICKS as f32;
            return [(40.0 + 140.0 * t) as u8, (16.0 + 50.0 * t) as u8, 14];
        }
        if mat.kind == MaterialKind::Gas {
            return [52, 50, 54];
        }
        let l = glow[i];
        return [
            (AIR[0] as f32 + 95.0 * l).min(255.0) as u8,
            (AIR[1] as f32 + 62.0 * l).min(255.0) as u8,
            (AIR[2] as f32 + 18.0 * l).min(255.0) as u8,
        ];
    }
    let p = mat.palette[cell.shade as usize % mat.palette.len()];
    let k = if cell.material == g.level.ore_material {
        1.0
    } else if cell.material == material::BEDROCK {
        0.25
    } else if mat.kind == MaterialKind::Powder {
        if mat.name == "shard" {
            0.95
        } else {
            0.55
        }
    } else {
        0.40
    };
    // Rock near a lit lamp is lit a little too.
    let l = 1.0 + 1.2 * glow[i];
    // Rock a blast has just heated glows, so the scorch shows where it hit.
    let hot = ((cell.temperature() as f32 - 80.0) / 400.0).clamp(0.0, 1.0);
    [
        (p[0] as f32 * k * l + 230.0 * hot).min(255.0) as u8,
        (p[1] as f32 * k * l + 90.0 * hot).min(255.0) as u8,
        (p[2] as f32 * k * l * 0.9 + 20.0 * hot).min(255.0) as u8,
    ]
}

/// Draw the whole window: the mine, the tools, and the bar.
pub fn paint(g: &Game, frame: &mut [u8], cursor: Option<(i32, i32)>) {
    // Lamp light, for the rock and air only (never the gas: the gas colours
    // are fixed).
    let mut glow = vec![0.0f32; W * H];
    for f in &g.faces {
        if f.state == FaceState::Blown {
            continue;
        }
        let strength = if f.state == FaceState::Working { 1.0 } else { 0.45 };
        let (lx, ly) = f.lamp;
        for dy in -18..=18 {
            for dx in -18..=18 {
                let (x, y) = (lx + dx, ly + dy);
                if gas::inside(x, y) {
                    let d = ((dx * dx + dy * dy) as f32).sqrt();
                    let v = (1.0 - d / 18.0).max(0.0);
                    glow[idx(x, y)] += strength * v * v;
                }
            }
        }
    }
    for y in 0..H as i32 {
        for x in 0..W as i32 {
            let c = cell_colour(g, x, y, &glow);
            let (sx, sy) = (x * S as i32, y * S as i32);
            for yy in 0..S as i32 {
                let row = ((sy + yy) as usize * FW + sx as usize) * 4;
                for xx in 0..S {
                    let j = row + xx * 4;
                    frame[j..j + 3].copy_from_slice(&c);
                    frame[j + 3] = 255;
                }
            }
        }
    }
    // The night sky at the top of the shaft.
    rect(frame, 14 * S as i32, 0, 11 * S as i32, 2 * S as i32, [24, 30, 52]);
    // Debris in flight.
    for p in g.particles.iter() {
        let mat = g.world.materials.get(p.material);
        let c = mat.palette[p.shade as usize % mat.palette.len()];
        rect(frame, p.x as i32 * S as i32, p.y as i32 * S as i32, S as i32, S as i32, [c[0], c[1], c[2]]);
    }
    let sc = |x: i32| x * S as i32 + S as i32 / 2;

    // The leak.
    let (lx, ly) = g.level.leak_at;
    for k in 0..3 {
        let x = sc(lx - 2 + 2 * k);
        line(frame, (x, sc(ly)), (x + 2 - k * 2, sc(ly) + 8), 2, [60, 120, 60]);
    }

    // Fans: hub, blades, the aim, and puffs moving down the cone.
    for f in &g.fans {
        let (ax, ay) = (f.angle.cos(), f.angle.sin());
        for k in 0..6 {
            let d = ((g.tick as f32 * 0.7 + k as f32 * 7.0) % 42.0) + 3.0;
            let (px, py) = (f.x as f32 + ax * d, f.y as f32 + ay * d);
            if g.cell_open(px as i32, py as i32) {
                let fade = 1.0 - d / 45.0;
                let c = [(150.0 + 90.0 * fade) as u8, (190.0 + 60.0 * fade) as u8, 255];
                rect(frame, sc(px as i32) - 1, sc(py as i32) - 1, 3, 3, c);
            } else {
                break;
            }
        }
        let (cx, cy) = (sc(f.x), sc(f.y));
        disc(frame, cx, cy, 9, [60, 66, 74]);
        let spin = g.tick as f32 * 0.5;
        for b in 0..3 {
            let a = spin + b as f32 * 2.094;
            line(frame, (cx, cy), (cx + (a.cos() * 8.0) as i32, cy + (a.sin() * 8.0) as i32), 3, [200, 210, 220]);
        }
        line(frame, (cx, cy), (cx + (ax * 22.0) as i32, cy + (ay * 22.0) as i32), 2, [140, 200, 255]);
        let (hx, hy) = (cx + (ax * 22.0) as i32, cy + (ay * 22.0) as i32);
        for side in [-1.0f32, 1.0] {
            let a = f.angle + std::f32::consts::PI + side * 0.6;
            line(frame, (hx, hy), (hx + (a.cos() * 7.0) as i32, hy + (a.sin() * 7.0) as i32), 2, [140, 200, 255]);
        }
    }

    // Fuses: a post with a head that shows whether the gas on it is ready.
    for f in &g.fuses {
        let (x, y) = (sc(f.x), sc(f.y));
        rect(frame, x - 1, y - 10, 3, 14, [150, 120, 80]);
        let ready = match gas::band(g.gas.at(f.x, f.y)) {
            Band::Thick => [36, 150, 70],
            Band::Thin => [128, 108, 178],
            _ => [230, 60, 40],
        };
        disc(frame, x, y - 12, 4, ready);
        outline(frame, x - 5, y - 17, 11, 11, [240, 240, 240]);
    }

    // Matches.
    for m in &g.matches {
        let (x, y) = (sc(m.x as i32), sc(m.y as i32));
        let c = if (g.tick / 3).is_multiple_of(2) { [255, 240, 160] } else { [255, 170, 60] };
        disc(frame, x, y, 3, c);
        rect(frame, x - 1, y + 3, 2, 6, [180, 140, 90]);
    }

    // Lamps and face labels.
    for (k, f) in g.faces.iter().enumerate() {
        let (x, y) = (sc(f.lamp.0), sc(f.lamp.1));
        let (word, col) = match f.state {
            FaceState::Working => ("WORKING", [180, 240, 160, 255]),
            FaceState::GasNear => ("GAS NEAR - STOPPED", [250, 210, 90, 255]),
            FaceState::NoAir => ("NO FRESH AIR", [250, 210, 90, 255]),
            FaceState::Singed => ("SINGED", [250, 150, 80, 255]),
            FaceState::Blown => ("BLOWN UP", [255, 90, 70, 255]),
        };
        if f.state == FaceState::Blown {
            line(frame, (x - 10, y - 10), (x + 10, y + 10), 4, [230, 60, 50]);
            line(frame, (x - 10, y + 10), (x + 10, y - 10), 4, [230, 60, 50]);
        } else {
            let lit = if f.state == FaceState::Working { [255, 220, 120] } else { [200, 140, 60] };
            disc(frame, x, y, 6, [80, 60, 30]);
            disc(frame, x, y, 4, lit);
        }
        let name = super::Face::name(k);
        let lxp = if k == 0 { x - 150 } else { x - 210 };
        label(frame, lxp, y - 44, name, TEXT);
        label(frame, lxp, y - 26, word, col);
    }
    label(frame, 27 * S as i32, 6, "AIR SHAFT", DIM);
    label(frame, sc(lx) - 18, sc(ly) + 12, "LEAK", [140, 210, 140, 255]);
    if g.tick < 20 * TICKS_PER_SECOND {
        label(frame, 150 * S as i32, 132 * S as i32, "ORE SEAM", [200, 230, 245, 255]);
    }

    // Pops.
    for (x, y, s, _) in &g.pops {
        label(frame, sc(*x) - text_w(s) / 2, sc(*y), s, [255, 240, 150, 255]);
    }

    // The key to the three colours, top right.
    let kx = FW as i32 - 196;
    shade_rect(frame, kx - 8, 4, 200, 64, 0.3);
    for (k, (c, s)) in [(THIN, "TOO THIN"), (BURN, "WILL BURN"), (THICK, "TOO THICK")].iter().enumerate() {
        let y = 10 + k as i32 * 19;
        rect(frame, kx, y, 22, 14, *c);
        text(frame, kx + 30, y, s, TEXT);
    }

    // Cursor ring, and a door's tap box when the pointer is on one.
    if let Some((cx, cy)) = cursor {
        if gas::inside(cx, cy) {
            let (x, y) = (sc(cx), sc(cy));
            outline(frame, x - 7, y - 7, 15, 15, [240, 240, 240]);
            if g.tool == Tool::Match {
                // The straight-down line the match will fall along.
                let mut yy = cy + 1;
                while g.cell_open(cx, yy) && yy < cy + 60 {
                    if yy % 3 == 0 {
                        rect(frame, x - 1, sc(yy) - 1, 2, 2, [200, 190, 170]);
                    }
                    yy += 1;
                }
            }
        }
    }

    // What the match or fuse in hand would burn.
    if let Some(p) = g.preview.as_ref().filter(|p| p.tool == g.tool) {
        draw_preview(g, frame, p);
    }

    // Flash.
    if g.flash > 0.02 {
        let a = g.flash * 0.45;
        for px in frame[..FW * H * S * 4].chunks_exact_mut(4) {
            for v in &mut px[..3] {
                *v = (*v as f32 + (255.0 - *v as f32) * a) as u8;
            }
        }
    }

    // The bar.
    let top = (H * S) as i32;
    rect(frame, 0, top, FW as i32, HUD_H as i32, [24, 22, 27]);
    for (tool, x, y, w, h) in buttons() {
        let left = match tool {
            Tool::Fan => g.stock.fans,
            Tool::Door => g.stock.doors,
            Tool::Match => g.stock.matches,
            Tool::Fuse => g.stock.fuses,
        };
        let on = g.tool == tool;
        rect(frame, x, y, w, h, if on { [70, 64, 50] } else { [38, 36, 42] });
        outline(frame, x, y, w, h, if on { [250, 220, 120] } else { [80, 76, 84] });
        let k = Tool::ALL.iter().position(|t| *t == tool).unwrap() + 1;
        text(frame, x + 8, y + 6, &format!("{k} {} X{left}", tool.label()), if left == 0 { DIM } else { TEXT });
    }
    let secs = g.seconds_left();
    let ore = g.ore();
    let info = format!("TIME {}:{:02}   ORE {ore}/{}", secs / 60, secs % 60, g.next_star());
    text(frame, 620, top + 12, &info, TEXT);
    let stars_now = g.stars();
    for s in 0..3 {
        star(frame, 940 + s * 28, top + 19, 10, (s as u32) < stars_now);
    }
    let mut y = top + 40;
    text(frame, 10, y, tool_sentence(g.tool), DIM);
    y += 18;
    // The pointer's own line: what a light here would do, or the gas here.
    let preview = g.preview.as_ref().filter(|p| p.tool == g.tool);
    let (reading, colour) = match preview {
        Some(p) => verdict(p),
        None => (cursor.filter(|&(x, y)| gas::inside(x, y)).map(|(x, y)| format!("{}.", g.reading(x, y))).unwrap_or_default(), TEXT),
    };
    if let Some(l) = wrap(&reading, 84).first() {
        text(frame, 10, y, l, colour);
    }
    y += 18;
    if let Some((m, _)) = &g.message {
        for l in wrap(m, 84).iter().take(2) {
            text(frame, 10, y, l, [255, 236, 170, 255]);
            y += 18;
        }
    }
    // The end of a shift.
    if g.over {
        let (pw, ph) = (520, 150);
        let (px, py) = ((FW as i32 - pw) / 2, 200);
        shade_rect(frame, px, py, pw, ph, 0.2);
        outline(frame, px, py, pw, ph, [250, 220, 120]);
        let title = if g.faces_standing() == 0 { "THE MINE IS LOST" } else { "END OF SHIFT" };
        text(frame, px + (pw - text_w(title)) / 2, py + 14, title, TEXT);
        for s in 0..3 {
            star(frame, px + pw / 2 - 40 + s * 40, py + 58, 16, (s as u32) < stars_now);
        }
        let l = format!("{} ORE   {} OF 2 FACES STANDING", ore, g.faces_standing());
        text(frame, px + (pw - text_w(&l)) / 2, py + 86, &l, TEXT);
        let l = "R: SAME MINE AGAIN    N: A NEW MINE";
        text(frame, px + (pw - text_w(l)) / 2, py + 116, l, DIM);
    }
    let _ = LEVEL_TICKS;
}

const RED: [u8; 3] = [255, 72, 60];
const AMBER: [u8; 3] = [255, 160, 60];

fn worst(p: &Preview) -> Risk {
    p.risk[0].max(p.risk[1])
}

/// The pointer line for a match or fuse in hand.
fn verdict(p: &Preview) -> (String, [u8; 4]) {
    let thing = if p.tool == Tool::Match { "THE MATCH" } else { "THE FUSE" };
    let s = match p.outcome {
        Outcome::FizzleThick => format!("{thing} WILL FIZZLE IN THE GREEN: TOO THICK. STIR IT WITH A FAN FIRST."),
        Outcome::FizzleThin => format!("{thing} WILL GO OUT: NO GAS THAT CAN BURN WHERE IT LANDS."),
        Outcome::Waits => "THE FUSE WILL WAIT HERE UNTIL THE GAS ON IT TURNS YELLOW.".to_string(),
        Outcome::Burns => {
            let what = match p.size {
                BangSize::Pop => "POP: IT BURNS THE OUTLINED YELLOW, TOO LITTLE TO GO BANG.".to_string(),
                b => format!("{}: IT BURNS THE OUTLINED YELLOW AND BLASTS THE CIRCLE.", b.label()),
            };
            let mut harm = String::new();
            for (k, r) in p.risk.iter().enumerate() {
                let name = super::Face::name(k);
                match r {
                    Risk::Lost => harm = format!(" IT TAKES THE {name}."),
                    Risk::Singed if harm.is_empty() => harm = format!(" IT SINGES THE {name}."),
                    _ => {}
                }
            }
            what + &harm
        }
    };
    let c = match (p.outcome, worst(p)) {
        (Outcome::Burns, Risk::Lost) => [RED[0], RED[1], RED[2], 255],
        (Outcome::Burns, Risk::Singed) => [AMBER[0], AMBER[1], AMBER[2], 255],
        (Outcome::Burns, _) => [255, 255, 255, 255],
        _ => [200, 190, 175, 255],
    };
    (s, c)
}

fn draw_preview(g: &Game, frame: &mut [u8], p: &Preview) {
    let sc = |v: i32| v * S as i32 + S as i32 / 2;
    let (ax, ay) = (sc(p.at.0), sc(p.at.1));
    match p.outcome {
        Outcome::FizzleThick | Outcome::FizzleThin => {
            line(frame, (ax - 6, ay - 6), (ax + 6, ay + 6), 2, [220, 210, 195]);
            line(frame, (ax - 6, ay + 6), (ax + 6, ay - 6), 2, [220, 210, 195]);
            let s = if p.outcome == Outcome::FizzleThick { "FIZZLES: TOO THICK" } else { "GOES OUT" };
            label(frame, ax - text_w(s) / 2, ay + 10, s, [230, 222, 205, 255]);
            return;
        }
        Outcome::Waits => {
            let s = "WAITS FOR YELLOW";
            label(frame, ax - text_w(s) / 2, ay + 10, s, [230, 222, 205, 255]);
            return;
        }
        Outcome::Burns => {}
    }
    // The outline pulses, so it reads as a forecast over the gas and never
    // as gas itself; red when it costs a face, amber when it singes one.
    let pulse = ((g.tick as f32 / 9.0).sin() * 0.5 + 0.5) * 0.2 + 0.8;
    let base = match worst(p) {
        Risk::Lost => RED,
        Risk::Singed => AMBER,
        Risk::Safe => [255, 255, 255],
    };
    let col = base.map(|v| (v as f32 * pulse) as u8);
    let mut mark = vec![false; W * H];
    for &c in &p.cells {
        mark[c] = true;
    }
    let s = S as i32;
    for &c in &p.cells {
        let (x, y) = ((c % W) as i32, (c / W) as i32);
        let (px, py) = (x * s, y * s);
        // Lift the patch itself a little, so a thin ribbon stands out from
        // yellow that is not joined to it.
        for yy in py..py + s {
            for xx in px..px + s {
                let o = (yy as usize * FW + xx as usize) * 4;
                for v in &mut frame[o..o + 3] {
                    *v = (*v as f32 + (255.0 - *v as f32) * 0.15) as u8;
                }
            }
        }
        let open = |dx: i32, dy: i32| {
            let (nx, ny) = (x + dx, y + dy);
            !gas::inside(nx, ny) || !mark[idx(nx, ny)]
        };
        // Two pixels wide on the patch's own side of the edge, so a
        // one-cell ribbon still shows as a solid line, with a dark pixel
        // just outside it so it reads against pale yellow and violet alike.
        const EDGE: [u8; 3] = [20, 16, 24];
        if open(-1, 0) {
            rect(frame, px - 1, py, 1, s, EDGE);
            rect(frame, px, py, 2, s, col);
        }
        if open(1, 0) {
            rect(frame, px + s, py, 1, s, EDGE);
            rect(frame, px + s - 2, py, 2, s, col);
        }
        if open(0, -1) {
            rect(frame, px, py - 1, s, 1, EDGE);
            rect(frame, px, py, s, 2, col);
        }
        if open(0, 1) {
            rect(frame, px, py + s, s, 1, EDGE);
            rect(frame, px, py + s - 2, s, 2, col);
        }
    }
    // Where it starts.
    disc(frame, ax, ay, 3, col);
    // The blast, dotted, from the pocket's heart.
    let (hx, hy) = (sc(p.heart.0), sc(p.heart.1));
    if p.radius > 0 {
        let r = (p.radius * s) as f32;
        let n = (r * 0.9) as i32;
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            let (x, y) = (hx + (r * a.cos()) as i32, hy + (r * a.sin()) as i32);
            rect(frame, x - 1, y - 1, 3, 3, col);
        }
    }
    let word = p.size.label();
    let ly = if p.radius > 0 { hy - p.radius * s - 22 } else { hy - 26 };
    label(frame, hx - text_w(word) / 2, ly.max(4), word, [col[0], col[1], col[2], 255]);
}

/// **The start card** (2026-11-08): the goal, every tool and key, and what
/// earns a star, on one panel over the mine. Added after the owner's
/// playtests of the sibling demos: his first complaint in both was not
/// knowing the goal, the options, or what each tool did. The window opens
/// on it, paused; `H` brings it back.
pub fn help(frame: &mut [u8]) {
    let (pw, ph) = (900, 424);
    let (px, py) = ((FW as i32 - pw) / 2, 70);
    shade_rect(frame, px, py, pw, ph, 0.12);
    outline(frame, px, py, pw, ph, [250, 220, 120]);
    let gold = [255, 226, 130, 255];
    let x = px + 28;
    let mut y = py + 20;
    text(frame, x, y, "FIREDAMP", gold);
    y += 30;
    text(frame, x, y, "GOAL: GET AS MUCH ORE OUT AS YOU CAN IN A 3-MINUTE SHIFT.", gold);
    y += 20;
    for l in [
        "GAS LEAKS INTO THE SUMP. YELLOW GAS BURNS, AND A BIG YELLOW",
        "POCKET GOES BANG: IT BREAKS ROCK AND FREES ORE. GAS THAT REACHES",
        "A LAMP BLOWS UP THAT WORK FACE FOR THE REST OF THE SHIFT.",
    ] {
        text(frame, x, y, l, TEXT);
        y += 20;
    }
    y += 14;
    for (k, what) in [
        ("1 FAN", "PUSHES AND THINS GAS. PRESS, THEN DRAG TO AIM IT."),
        ("2 DOOR", "BLOCKS A TUNNEL. CLICK IT TO SLAM OR OPEN IT."),
        ("3 MATCH", "FALLS AND LIGHTS THE FIRST YELLOW IT HITS. HOLD IT"),
        ("", "OVER THE GAS TO SEE WHAT WILL BURN AND HOW BIG."),
        ("4 FUSE", "LIGHTS ITSELF WHEN THE GAS ON IT TURNS YELLOW."),
        ("RIGHT CLICK", "PICKS A FAN, DOOR OR FUSE BACK UP."),
    ] {
        text(frame, x, y, k, gold);
        text(frame, x + 170, y, what, TEXT);
        y += 20;
    }
    y += 14;
    for l in [
        "STARS AT 60, 140 AND 220 ORE. A FACE WITH CLEAR AIR SENDS A CART",
        "OF ORE EVERY 3 SECONDS; A BANG BY THE ORE SEAM BLASTS MORE LOOSE.",
        "TOO THIN (VIOLET) OR TOO THICK (GREEN) GAS WILL NOT BURN.",
    ] {
        text(frame, x, y, l, TEXT);
        y += 20;
    }
    y += 14;
    text(frame, x, y, "SPACE PAUSE    R SAME MINE    N NEW MINE    H THIS CARD", DIM);
    y += 26;
    text(frame, x, y, "CLICK OR PRESS ANY KEY TO START.", gold);
}
