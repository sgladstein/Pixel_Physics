//! What Heartwood draws: the valley through the shipped renderer, and the
//! game over it -- the tips you can play on, the outline of where the next
//! card would go, the hand, the purse and the light share.
//!
//! **Every word on screen is a world word** (`common.md`'s bar, learned from
//! the satchel probe's unreadable HUD): no tags, no internal names. The
//! line under the hand always says what the card under the pointer would do
//! to the tip you have picked.

use super::{CardKind, Heartwood, Verdict, H, W};
use crate::hud::{draw_text_scaled, GLYPH_HEIGHT, GLYPH_WIDTH};
use crate::render::Renderer;

/// Screen pixels per world cell.
pub const ZOOM: i32 = 3;
pub const VIEW_W: u32 = (W * ZOOM) as u32;
pub const VIEW_H: u32 = (H * ZOOM) as u32;
/// The strip under the valley.
pub const HUD_H: u32 = 156;
pub const FRAME_W: u32 = VIEW_W;
pub const FRAME_H: u32 = VIEW_H + HUD_H;

/// One hand card's box on screen, in frame pixels.
pub fn card_rect(i: usize) -> (i32, i32, i32, i32) {
    let w = 146;
    let x = 6 + i as i32 * (w + 6);
    (x, VIEW_H as i32 + 30, w, 54)
}

/// Which hand card is under a frame pixel.
pub fn card_under(game: &Heartwood, (px, py): (i32, i32)) -> Option<usize> {
    (0..game.hand.len()).find(|&i| {
        let (x, y, w, h) = card_rect(i);
        px >= x && px < x + w && py >= y && py < y + h
    })
}

/// A frame pixel to the world cell under it, inside the valley view.
pub fn to_cell((px, py): (i32, i32)) -> Option<(i32, i32)> {
    if px < 0 || py < 0 || px >= VIEW_W as i32 || py >= VIEW_H as i32 {
        return None;
    }
    Some((px / ZOOM, py / ZOOM))
}

pub struct Screen {
    pub renderer: Renderer,
    world_buf: Vec<u8>,
    first: bool,
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen {
    pub fn new() -> Self {
        let mut renderer = Renderer::new();
        for _ in 1..ZOOM {
            renderer.adjust_zoom(1);
        }
        renderer.set_camera(0, 0, (VIEW_W, VIEW_H), Some(crate::sim::chunk::Rect::new(0, 0, W - 1, H - 1)));
        Screen { renderer, world_buf: vec![0; (VIEW_W * VIEW_H * 4) as usize], first: true }
    }

    /// Draw one frame into `frame` (`FRAME_W` x `FRAME_H` RGBA). `hover` is
    /// the hand card under the pointer, if any.
    pub fn draw(&mut self, game: &mut Heartwood, frame: &mut [u8], hover: Option<usize>) {
        let touched = game.world.take_touched_chunks();
        self.renderer.draw(&game.world, &game.particles, &touched, &mut self.world_buf, (VIEW_W, VIEW_H), self.first);
        self.first = false;
        let row = (VIEW_W * 4) as usize;
        frame[..self.world_buf.len()].copy_from_slice(&self.world_buf);
        for px in frame[self.world_buf.len()..].chunks_exact_mut(4) {
            px.copy_from_slice(&[18, 20, 16, 255]);
        }
        let _ = row;
        overlay(game, frame, hover);
        hud(game, frame, hover);
    }
}

fn fill(frame: &mut [u8], (x, y, w, h): (i32, i32, i32, i32), c: [u8; 4]) {
    for yy in y.max(0)..(y + h).min(FRAME_H as i32) {
        for xx in x.max(0)..(x + w).min(FRAME_W as i32) {
            let i = ((yy as u32 * FRAME_W + xx as u32) * 4) as usize;
            if c[3] == 255 {
                frame[i..i + 4].copy_from_slice(&c);
            } else {
                let a = c[3] as u32;
                for k in 0..3 {
                    frame[i + k] = ((frame[i + k] as u32 * (255 - a) + c[k] as u32 * a) / 255) as u8;
                }
            }
        }
    }
}

fn ring(frame: &mut [u8], (cx, cy): (i32, i32), r: i32, c: [u8; 4]) {
    fill(frame, (cx - r, cy - r, 2 * r + 1, 1), c);
    fill(frame, (cx - r, cy + r, 2 * r + 1, 1), c);
    fill(frame, (cx - r, cy - r, 1, 2 * r + 1), c);
    fill(frame, (cx + r, cy - r, 1, 2 * r + 1), c);
}

fn text(frame: &mut [u8], (x, y): (i32, i32), s: &str, scale: i32, c: [u8; 4]) {
    draw_text_scaled(frame, (FRAME_W, FRAME_H), scale, (x / scale, y / scale), s, c);
}

fn text_w(s: &str, scale: i32) -> i32 {
    s.chars().count() as i32 * (GLYPH_WIDTH + 1) * scale
}

const WHITE: [u8; 4] = [236, 236, 228, 255];
const DIM: [u8; 4] = [150, 150, 140, 255];
const GOLD: [u8; 4] = [250, 214, 90, 255];
const MINE: [u8; 4] = [120, 220, 110, 255];
const THEIRS: [u8; 4] = [220, 120, 90, 255];

/// Marks drawn over the valley.
fn overlay(game: &Heartwood, frame: &mut [u8], hover: Option<usize>) {
    let cell = |(x, y): (i32, i32)| (x * ZOOM + ZOOM / 2, y * ZOOM + ZOOM / 2);
    // **Leaves the low sun does not reach**, either tree's: dimmed, so
    // shading the rival is something you can watch happen.
    for &(x, y) in &game.dark {
        fill(frame, (x * ZOOM, y * ZOOM, ZOOM, ZOOM), [10, 16, 30, 150]);
    }
    // The two low suns, in the top corners, and the way their light comes in.
    for &(fx, fy) in &super::SUNS {
        let sx0 = if fx < 0 { 34 } else { VIEW_W as i32 - 34 };
        let sy0 = 34;
        fill(frame, (sx0 - 10, sy0 - 10, 21, 21), [255, 222, 120, 255]);
        for k in 1..6 {
            let (dx, dy) = (-fx * k * 5, -fy * k * 5);
            fill(frame, (sx0 + dx, sy0 + dy, 3, 3), [255, 222, 120, 150]);
        }
    }
    // **Where the card would go**: the whole footprint, bold enough to read
    // on a phone -- white when it will play, amber when water will stop it
    // short, red when it will not play at all (with the reason on the line
    // under the hand).
    let preview_card = hover.or(if game.hand.is_empty() { None } else { Some(0) });
    if let Some(i) = preview_card {
        let colour = match game.preview_verdict(i) {
            Some(Verdict::No(_)) => [240, 70, 60, 230],
            Some(Verdict::Short(_)) => [250, 180, 60, 230],
            _ => [255, 255, 255, 220],
        };
        let leafy = game.hand.get(i) == Some(&CardKind::LeafOut);
        for p in game.preview(i) {
            let (sx, sy) = cell(p);
            fill(frame, (sx - 2, sy - 2, 5, 5), [0, 0, 0, 120]);
            fill(frame, (sx - 1, sy - 1, 3, 3), colour);
            if leafy {
                for dx in [-2, 2] {
                    let (lx, ly) = cell((p.0 + dx, p.1));
                    fill(frame, (lx - 1, ly - 1, 3, 3), [110, 200, 90, 200]);
                }
            }
        }
    }
    let blink = (game.world.frame / 20).is_multiple_of(2);
    for (tag, p) in game.tips() {
        let (sx, sy) = cell(p);
        let idle = game.idle(tag);
        if game.selected.contains(&tag) {
            ring(frame, (sx, sy), 5, GOLD);
            ring(frame, (sx, sy), 6, [0, 0, 0, 160]);
        } else if idle {
            if blink {
                ring(frame, (sx, sy), 4, WHITE);
            }
        } else {
            fill(frame, (sx - 1, sy - 1, 3, 3), GOLD);
        }
    }
    for b in game.buds.iter().filter(|b| !b.fired) {
        let (sx, sy) = cell(b.at);
        fill(frame, (sx - 2, sy, 5, 1), [255, 140, 200, 255]);
        fill(frame, (sx, sy - 2, 1, 5), [255, 140, 200, 255]);
    }
    // Labels on the two trees.
    let label = |frame: &mut [u8], x: i32, s: &str, c: [u8; 4]| {
        let sx = x * ZOOM - text_w(s, 2) / 2;
        text(frame, (sx, (super::GROUND_Y + 3) * ZOOM), s, 2, c);
    };
    label(frame, super::MY_X, "YOU", MINE);
    label(frame, super::RIVAL_X, "RIVAL", THEIRS);
}

fn hud(game: &Heartwood, frame: &mut [u8], hover: Option<usize>) {
    // The goal, always on screen, between the two suns.
    let goal = "GOAL: CATCH MORE SUN THAN THE RIVAL";
    let band = text_w("LIGHT AT YEAR END: 40% = 1 STAR  55% = 2  70% = 3     H: HOW TO PLAY", 1).max(text_w(goal, 2)) + 16;
    fill(frame, ((VIEW_W as i32 - band) / 2, 4, band, 34), [10, 14, 10, 150]);
    text(frame, ((VIEW_W as i32 - text_w(goal, 2)) / 2, 8), goal, 2, WHITE);
    let stars = "LIGHT AT YEAR END: 40% = 1 STAR  55% = 2  70% = 3     H: HOW TO PLAY";
    text(frame, ((VIEW_W as i32 - text_w(stars, 1)) / 2, 28), stars, 1, WHITE);
    let top = VIEW_H as i32;
    // Line 1: season, light share, sugar, prunes.
    let season = game.season();
    let left = if game.over { 0 } else { super::SEASON_FRAMES - game.t % super::SEASON_FRAMES };
    let share = game.share();
    let s1 = if game.over {
        format!("YEAR OVER: {} STARS", game.stars())
    } else {
        format!("SEASON {}/{}  {}S LEFT", season + 1, super::SEASONS, left / 60)
    };
    text(frame, (6, top + 6), &s1, 2, WHITE);
    // The light bar: yours against the rival's, with the star marks.
    let (bx, bw) = (250, 260);
    fill(frame, (bx, top + 6, bw, 14), [60, 40, 34, 255]);
    fill(frame, (bx, top + 6, (bw as f32 * share) as i32, 14), [70, 150, 60, 255]);
    for s in super::STARS {
        fill(frame, (bx + (bw as f32 * s) as i32, top + 4, 1, 18), GOLD);
    }
    text(frame, (bx + 4, top + 8), &format!("YOUR LIGHT {}%", (share * 100.0).round() as i32), 1, WHITE);
    let rate = game.net_rate();
    let s3 = format!("SUGAR {} ({}{:.1}/S)  PRUNES {}", game.sugar.floor() as i32, if rate >= 0.0 { "+" } else { "" }, rate, game.prunes);
    text(frame, (FRAME_W as i32 - text_w(&s3, 2) - 6, top + 6), &s3, 2, GOLD);
    // The tree's books, in words: is it fed or starving?
    let (msg, col) = if game.starving() {
        ("YOUR TREE IS STARVING: ITS WOOD COSTS MORE THAN ITS LEAVES EARN", [240, 110, 90, 255])
    } else {
        ("YOUR TREE IS FED: ITS LEAVES EARN MORE THAN ITS WOOD COSTS", [140, 210, 120, 255])
    };
    text(frame, (6, top + 22), msg, 1, col);
    // Line 2: the hand.
    for (i, &card) in game.hand.iter().enumerate() {
        let (x, y, w, h) = card_rect(i);
        let afford = game.sugar + 1e-4 >= card.cost();
        let bg = if hover == Some(i) { [80, 76, 52, 255] } else { [44, 46, 38, 255] };
        fill(frame, (x, y, w, h), bg);
        ring_rect(frame, (x, y, w, h), if afford { GOLD } else { [90, 90, 80, 255] });
        let label = if card == CardKind::Lean { "LEAN" } else { card.name() };
        text(frame, (x + 6, y + 6), &format!("{} {}", i + 1, label), 2, if afford { WHITE } else { DIM });
        text(frame, (x + 6, y + 30), &format!("{} SUGAR", card.cost() as i32), 2, if afford { GOLD } else { DIM });
        card_icon(frame, card, (x + w - 22, y + 32));
    }
    // Line 3: what the card under the pointer would do, to which tip.
    let line3 = match hover.or(if game.hand.is_empty() { None } else { Some(0) }).and_then(|i| game.hand.get(i).copied()) {
        Some(card) => {
            let mut s = format!("{}: {}", card.name(), card.sentence());
            if card == CardKind::Lean {
                if let Some(side) = game.lean_side() {
                    s.push_str(if side > 0.0 { " (RIGHT)" } else { " (LEFT)" });
                }
            }
            match hover.or(Some(0)).and_then(|i| game.preview_verdict(i)) {
                Some(Verdict::No(why)) => s = format!("{}: {}", card.name(), why),
                Some(Verdict::Short(n)) => s = format!("{}: WATER REACHES ONLY {} MORE CELLS", card.name(), n),
                _ => {}
            }
            s
        }
        None => format!("NO CARDS IN HAND. A NEW ONE COMES EVERY {:.1} S", heartwood_draw_seconds()),
    };
    let line3_colour = match hover.or(Some(0)).and_then(|i| game.preview_verdict(i)) {
        Some(Verdict::No(_)) => [240, 90, 80, 255],
        Some(Verdict::Short(_)) => [250, 190, 80, 255],
        _ => WHITE,
    };
    text(frame, (6, top + 92), &line3, 2, line3_colour);
    // The picked tip's queue, and the latest note.
    let queue = game.selected.iter().next().map(|&t| {
        let mut parts: Vec<&str> = Vec::new();
        if let Some(c) = game.doing.get(&t) {
            parts.push(c.short());
        }
        if let Some(q) = game.queues.get(&t) {
            parts.extend(q.iter().map(|c| c.short()));
        }
        if let Some(why) = game.blocked.get(&t).filter(|_| parts.is_empty()) {
            format!("THIS TIP IS STUCK: {why}")
        } else if parts.is_empty() {
            "THIS TIP IS WAITING FOR A CARD".to_string()
        } else {
            format!("THIS TIP: {}", parts.join(" > "))
        }
    });
    if let Some(q) = queue {
        text(frame, (6, top + 114), &q, 2, GOLD);
    }
    if let Some(n) = game.notes.last() {
        let s = &n.text;
        text(frame, (FRAME_W as i32 - text_w(s, 1) - 6, top + 118), s, 1, DIM);
    }
    let keys = "CLICK A TIP: PICK IT   1-5 OR CLICK A CARD: PLAY IT THERE   RIGHT-CLICK YOUR TREE: PRUNE   N: NEW GAME   H: HELP";
    text(frame, (6, top + 138), keys, 1, DIM);
    let seasons: Vec<String> = game.tally.seasons.iter().map(|s| format!("{}%", (s * 100.0).round() as i32)).collect();
    let st = format!(
        "SEASONS {}   BRANCHES LOST {}   SUGAR SPENT {}",
        if seasons.is_empty() { "-".to_string() } else { seasons.join(" ") },
        game.tally.limbs_lost,
        game.tally.sugar_spent as i32
    );
    text(frame, (FRAME_W as i32 - text_w(&st, 1) - 6, top + 128), &st, 1, DIM);
    let _ = GLYPH_HEIGHT;
}

/// **How to play**, over the valley: the goal, how a star is won, the two
/// clicks a card takes, what each card does, and the keys. The bin shows it
/// at the start (paused) and on H. Scott's playtests of Storm and Mould
/// (2026-11-08) said the same thing twice: he did not know the goal, his
/// options, or what each tool did -- so this is on screen, not in a README.
pub const HELP: [&str; 21] = [
    "GOAL: GROW YOUR TREE (LEFT) TO CATCH MORE SUN",
    "THAN THE RIVAL TREE (RIGHT).",
    "WIN: YOUR SHARE OF THE LIGHT WHEN SEASON 3 ENDS.",
    "40% = 1 STAR    55% = 2 STARS    70% = 3 STARS",
    "",
    "YOUR TREE ONLY GROWS WHERE YOU PLAY A CARD:",
    "1. CLICK A BLINKING TIP OF YOUR TREE TO PICK IT",
    "2. PRESS 1-5 OR CLICK A CARD TO GROW IT FROM THERE",
    "   THE OUTLINE SHOWS WHERE IT GOES. RED: IT WILL NOT.",
    "",
    "GROW UP    40 CELLS STRAIGHT UP",
    "LEAN       36 CELLS CURVING TOWARD THE SUN",
    "FORK       SPLITS THE TIP INTO TWO TIPS",
    "LEAF OUT   14 CELLS COVERED IN LEAVES",
    "BUD        A SPARE TIP, IN CASE THIS BRANCH IS LOST",
    "",
    "SUGAR: SUNLIT LEAVES EARN IT, WOOD COSTS IT, CARDS USE IT",
    "SHADE: DARK LEAVES GET NO SUN. GROW OVER THE RIVAL.",
    "RIGHT-CLICK YOUR TREE: PRUNE    N: NEW GAME",
    "",
    "CLICK OR PRESS A KEY TO START    H: THIS PAGE AGAIN",
];

/// Draw [`HELP`] over the valley (not the HUD, so the hand stays readable).
pub fn help_panel(frame: &mut [u8]) {
    fill(frame, (0, 0, VIEW_W as i32, VIEW_H as i32), [12, 16, 12, 225]);
    text(frame, (24, 12), "HEARTWOOD: HOW TO PLAY", 3, GOLD);
    let mut y = 50;
    for (i, line) in HELP.iter().enumerate() {
        if line.is_empty() {
            y += 8;
            continue;
        }
        let c = if i < 4 { MINE } else if i == HELP.len() - 1 { GOLD } else { WHITE };
        text(frame, (24, y), line, 2, c);
        y += 19;
    }
}

fn ring_rect(frame: &mut [u8], (x, y, w, h): (i32, i32, i32, i32), c: [u8; 4]) {
    fill(frame, (x, y, w, 1), c);
    fill(frame, (x, y + h - 1, w, 1), c);
    fill(frame, (x, y, 1, h), c);
    fill(frame, (x + w - 1, y, 1, h), c);
}

/// A tiny picture of what the card does.
fn card_icon(frame: &mut [u8], card: CardKind, (x, y): (i32, i32)) {
    let wood = [150, 110, 70, 255];
    let leaf = [90, 170, 70, 255];
    let px = |frame: &mut [u8], dx: i32, dy: i32, c: [u8; 4]| fill(frame, (x + dx * 2, y + dy * 2, 2, 2), c);
    match card {
        CardKind::GrowUp => {
            for k in 0..8 {
                px(frame, 4, 8 - k, wood);
            }
        }
        CardKind::Lean => {
            for k in 0..8 {
                px(frame, 2 + k * 2 / 3, 8 - k, wood);
            }
        }
        CardKind::Fork => {
            for k in 0..4 {
                px(frame, 4, 8 - k, wood);
                px(frame, 4 - k, 4 - k, wood);
                px(frame, 4 + k, 4 - k, wood);
            }
        }
        CardKind::LeafOut => {
            for k in 0..5 {
                px(frame, 4, 8 - k, wood);
                px(frame, 3, 7 - k, leaf);
                px(frame, 5, 7 - k, leaf);
                px(frame, 2, 6 - k, leaf);
                px(frame, 6, 6 - k, leaf);
            }
        }
        CardKind::Bud => {
            for k in 0..8 {
                px(frame, 4, 8 - k, wood);
            }
            px(frame, 5, 5, [255, 140, 200, 255]);
            px(frame, 6, 5, [255, 140, 200, 255]);
        }
    }
}

fn heartwood_draw_seconds() -> f32 {
    super::DRAW_EVERY as f32 / 60.0
}
