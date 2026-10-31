//! **The seat card** (2026-10-30): one panel, the same in every seat, that
//! says what you are, what you are trying to do, how the game scores you,
//! how you lose, and which key does what.
//!
//! Asked for by the owner on 10-30 ("better UI and controls, with
//! explanations of the controls and demo goals in game"). Before it, a seat
//! explained itself in one ticker line built from `help_line`, which the
//! ticker clips at about 72 characters -- every seat's line was over 100, so
//! the player saw the first half of the keys and none of the goal.
//!
//! **Open at start, in the window only** (`bin/lab.rs`; a headless harness
//! never sees it, so its keys still reach the seat). A demo opens either
//! paused or in its seat's warm-up, and either way the card is what the
//! player reads first; a warm-up that ends under it pauses until it closes
//! (`Lab::demo_frame`). `Space` over the card means play (`Lab::demo_space`),
//! any other key just closes it, and `H` opens it again at any time with
//! the clock paused under it.
//!
//! The words live in [`card`], one arm per seat, kept to what the seat does
//! *as built*: a card that promises a verb the seat lacks is worse than no
//! card. `every_card_fits_its_panel` holds each one to the panel's size and
//! to the glyphs the HUD can draw.

use super::{DemoSeat, Lab};
use crate::hud::{text_width, GLYPH_HEIGHT};

/// What one seat's card says.
pub struct SeatCard {
    /// Big line at the top: who you are.
    pub title: &'static str,
    /// What you are trying to do, in a sentence or three; wrapped to fit.
    pub goal: &'static str,
    /// How the game scores you.
    pub score: &'static str,
    /// How you lose.
    pub lose: &'static str,
    /// The seat's own keys, `(key, what it does)`. The keys every seat shares
    /// (`Space`, `PgUp/PgDn`, `H`) are added by [`draw`].
    pub keys: &'static [(&'static str, &'static str)],
}

/// The panel's outer width and left edge, in logical pixels of the 512x320
/// canvas. 420 leaves the world visible at both sides.
const PANEL_W: i32 = 420;
const PANEL_X: i32 = (super::super::WIDTH as i32 - PANEL_W) / 2;
const PANEL_Y: i32 = 12;
const PAD: i32 = 10;
const ROW: i32 = GLYPH_HEIGHT + 4;
/// The key column's label width: `PGUP/DN` and a gap.
const KEY_W: i32 = 6 * 8;
/// Two key columns side by side.
const COL_W: i32 = (PANEL_W - 2 * PAD) / 2;

/// Longest description a key may carry and still fit its column.
pub const KEY_DESC_MAX: usize = ((COL_W - KEY_W - 6) / 6) as usize;
const _: () = assert!(KEY_DESC_MAX >= 20, "key column too narrow");
/// Characters per wrapped prose line.
const LINE_CHARS: usize = ((PANEL_W - 2 * PAD) / 6) as usize;

const AMBER: [u8; 4] = [242, 179, 61, 255];
const INK: [u8; 4] = [232, 230, 222, 255];
const MUTED: [u8; 4] = [150, 158, 168, 255];
const RED: [u8; 4] = [236, 120, 100, 255];
const GREEN: [u8; 4] = [140, 210, 130, 255];

/// The keys every seat shares, appended after the seat's own.
const SHARED_KEYS: &[(&str, &str)] = &[("SPACE", "RUN OR PAUSE THE CLOCK"), ("PGUP/DN", "FASTER / SLOWER"), ("H", "THIS CARD")];

/// Each seat's card. Keep it to what the seat does as built.
pub fn card(seat: DemoSeat) -> SeatCard {
    match seat {
        DemoSeat::Queen => SeatCard {
            title: "QUEEN -- YOU FOUND THE COLONY",
            goal: "EAT, CARRY FOOD HOME AND LAY. YOUR EGGS BECOME WORKERS. FOOD FALLS ONLY IN SUMMER, SO STORE ENOUGH TO GET THROUGH THREE WINTERS. A WILD EGG STARTS A NEW FAMILY LINE IN ITS OWN COLOUR: A GAMBLE THAT COSTS YOU 300 J (FERAL 700 J).",
            score: "WORKERS ALIVE AT THE END OF EACH WINTER",
            lose: "YOU DIE, OR A WINTER ENDS WITH NO WORKERS",
            keys: &[
                ("ARROWS", "WALK"),
                ("TAB", "CAMERA FOLLOWS YOU"),
                ("E", "EAT"),
                ("G", "DIG"),
                ("L", "LAY AN EGG"),
                ("Q", "DROP WHAT YOU CARRY"),
                ("P", "WILD EGG (AGAIN: FERAL)"),
                ("Y", "KEEP LINE: NEXT 5 EGGS"),
                ("K", "NEXT LEDGER ROW"),
                ("J", "RANK LINES: DIGS/TRIPS"),
                ("U", "LINE COLOURS ON/OFF"),
                ("R", "NEW COLONY"),
            ],
        },
        DemoSeat::Keeper => SeatCard {
            title: "KEEPER -- A HAND OVER THE BOX",
            goal: "FOOD FALLS AT FAR SPOTS AND ROTS WHILE YOU WATCH. PICK ANTS UP AND SET THEM ON IT BEFORE IT GOES GREY. CARRIES ARE FEW AND COME BACK SLOWLY, AND A LIFT TIRES THE ANT. AT THE END YOU ARE JUDGED AGAINST THE SAME BOX LEFT ALONE.",
            score: "THE VERDICT IS ANTS AT THE END AGAINST THE SAME BOX LEFT ALONE. FOOD SAVED IS YOUR RUNNING TALLY",
            lose: "FEWER THAN 10 ANTS. THE ROUND ENDS IN ABOUT 7 MINUTES",
            keys: &[
                ("Y", "HAND: CARRY (1 CHARGE)"),
                ("DRAG", "CARRY AN ANT TO FOOD"),
                ("U", "HAND: TREAT (10 FOOD)"),
                ("J", "RINGS / TRAFFIC / OFF"),
                ("K", "THE VERDICT CARD"),
                ("WASD", "PAN THE VIEW"),
            ],
        },
        DemoSeat::Scout => SeatCard {
            title: "SCOUT -- ONE WORKER WHO KNOWS THE WAY",
            goal: "THE NEAR FOOD HAS STOPPED: A LEAN SEASON. FOOD LIES FAR TO THE EAST, SEALED: ONLY AN ANT WHO WAS LED THERE CAN CARRY FROM IT. TAP A SISTER AND LEAD HER, AND WHEN SHE STOPS, WAIT. YOUR PUPILS LEAD SISTERS OF THEIR OWN.",
            score: "FOOD YOUR PUPILS BRING HOME: 1 A TRIP, 2 FOR THEIR PUPILS, 3 AFTER THAT",
            lose: "FEWER THAN 1 IN 5 OF THE COLONY ALIVE AT SPRING, OR YOU STARVE",
            keys: &[
                ("ARROWS", "WALK"),
                ("TAB", "CAMERA FOLLOWS YOU"),
                ("Y", "TAP A SISTER TO FOLLOW"),
                ("U", "PIN YOUR PUPIL"),
                ("P", "SNIFF NEAREST SISTER"),
                ("F", "SHARE FOOD"),
                ("G", "DIG"),
                ("E", "EAT"),
                ("Q", "DROP WHAT YOU CARRY"),
                ("R", "NEW SEASON"),
            ],
        },
        DemoSeat::Guest => SeatCard {
            title: "GUEST -- A ROVE BEETLE IN THE NEST",
            goal: "THE ANTS KNOW YOU BY SMELL. RUB THEM UNTIL YOU SMELL LIKE FAMILY, THEN BEG FOR FOOD. EVERY BEG MAKES THEM WARIER, SO SPACE YOUR ASKS AND RUB IN BETWEEN. WITH ENERGY TO SPARE, SLIP INTO THE NEST AND LAY AN EGG.",
            score: "1 A SECOND ALIVE, 100 AN EGG (200 BESIDE THEIR BROOD), PLUS ENERGY LEFT. A VISIT LASTS 5 MINUTES",
            lose: "ALL THREE OF YOUR BEETLES ARE KILLED",
            keys: &[
                ("ARROWS", "MOVE"),
                ("TAB", "CAMERA FOLLOWS YOU"),
                ("R", "RUB AN ANT (TAKE SMELL)"),
                ("P", "SOAK IN SMELL ON/OFF"),
                ("Y", "BEG FOR FOOD"),
                ("L", "LAY AN EGG"),
                ("B", "KILL A LARVA"),
                ("E", "EAT A CORPSE"),
                ("J", "JUMP"),
                ("K", "NEW VISIT AT THE END"),
            ],
        },
        DemoSeat::Enemy => SeatCard {
            title: "RAIDER -- A CENTIPEDE HUNTING THE COLONY",
            goal: "YOUR TANK DRAINS: EAT LONE ANTS ON THE SURFACE TO KEEP GOING. WHEN FED AND WHOLE, DIVE THROUGH THEIR DOOR, GRAB A WHITE LARVA AND RUN IT BACK TO YOUR GOLD FLAG. THEY BITE BACK, AND THE SEGMENT THEY BITE COMES OFF: BACK OUT TAIL FIRST.",
            score: "1 PER ANT KILLED, 10 PER LARVA BROUGHT TO THE FLAG, OVER 3 RAIDERS",
            lose: "A RAIDER DIES IF ITS HEAD IS BITTEN OFF OR IT STARVES. THE THIRD DEATH ENDS THE GAME",
            keys: &[
                ("ARROWS", "CREEP"),
                ("SHIFT", "SPRINT (HOLD)"),
                ("TAB", "CAMERA FOLLOWS YOU"),
                ("B", "GRAB A LARVA, OR BITE"),
                ("G", "CUT OR SHOVE SOIL"),
                ("E", "EAT"),
                ("Q", "DROP (A LARVA SCORES 0)"),
                ("Y", "SCORE CARD"),
                ("R", "NEXT RAIDER / NEW GAME"),
            ],
        },
        DemoSeat::Satchel => SeatCard {
            title: "SATCHEL -- STOP THE SPIDERS WITH A HAND OF CARDS",
            goal: "SPIDERS LAND NEAR THE NEST AND EAT YOUR ANTS. EACH CLICK PLAYS YOUR NEXT CARD WHERE YOU CLICK, AND THE TOP LINE SAYS WHAT IT WILL BE. FIRE BURNS A CIRCLE, ANTS TOO. CAVE-IN MAKES TUNNELS FALL IN. BIG MAKES THE NEXT CARD BIGGER. DOUBLE PLAYS TWO AT ONCE. TRAP WAITS: WHEN SOMETHING DIES IN ITS GREEN CIRCLE, THE CARD INSIDE GOES OFF ON THE BODY, AND IF THAT KILLS AGAIN IT GOES OFF AGAIN, UP TO 3 TIMES. CARDS COST ENERGY, WHICH REFILLS. AFTER THE LAST CARD THEY RELOAD.",
            score: "3 STARS: LAST 5 MINUTES, KEEP HALF YOUR ANTS, LET NO SPIDER ESCAPE YOUR CARDS",
            lose: "UNDER 5 ANTS ENDS THE ROUND. ANTS YOUR CARDS BURN COUNT. CLICK AFTER TO PLAY ON",
            keys: &[("CLICK", "PLAY YOUR NEXT CARD"), ("Y", "SWAP CARDS (PAUSED)"), ("WASD", "MOVE THE VIEW")],
        },
        DemoSeat::Mind => SeatCard {
            title: "COLONY MIND",
            goal: "NOT BUILT YET: THIS SEAT OPENS ON A PLAIN BOX.",
            score: "NONE YET",
            lose: "NONE YET",
            keys: &[],
        },
    }
}

/// The card as this run shows it: under `stakes=0` the seat is the old
/// sandbox, and a card promising a score and a way to lose would be wrong
/// (adversarial review, 2026-10-31).
pub fn shown_card(demo: &super::Demo) -> SeatCard {
    let mut c = card(demo.seat);
    if demo.args.get("stakes").is_some_and(|v| v == "0") {
        c.score = "NONE -- THIS IS THE SANDBOX (STAKES=0)";
        c.lose = "NOTHING: NO WAY TO LOSE IN THE SANDBOX";
    }
    c
}

/// `text` broken at spaces into lines of at most `max` characters.
pub fn wrap(text: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > max {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Every key line the card shows: the seat's own, then the shared ones.
fn all_keys(c: &SeatCard) -> Vec<(&'static str, &'static str)> {
    c.keys.iter().chain(SHARED_KEYS).copied().collect()
}

/// The panel's height for this card, in logical pixels.
pub fn panel_height(c: &SeatCard) -> i32 {
    let goal = wrap(c.goal, LINE_CHARS).len() as i32;
    let score = wrap(&format!("SCORE: {}", c.score), LINE_CHARS).len() as i32;
    let lose = wrap(&format!("YOU LOSE: {}", c.lose), LINE_CHARS).len() as i32;
    let keys = (all_keys(c).len() as i32 + 1) / 2;
    // title, gap, goal, gap, score, lose, gap, KEYS label, keys, gap, footer
    PAD * 2 + ROW + 4 + ROW * goal + 4 + ROW * (score + lose) + 6 + ROW + ROW * keys + 6 + ROW
}

/// **Paint the card** on the interface canvas, over everything the seat drew.
pub fn draw(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let Some(demo) = lab.demo.as_ref() else { return };
    if !demo.card {
        return;
    }
    let c = shown_card(demo);
    let h = panel_height(&c);
    // Modal: it is meant to cover the seat's panel and the cell readout.
    crate::render::paint_owner::set(crate::render::paint_owner::MODAL);
    hc.blend_rect(frame_buf, PANEL_X, PANEL_Y, PANEL_W, h, [12, 14, 18, 255], 0.96);
    hc.fill_rect(frame_buf, PANEL_X, PANEL_Y, PANEL_W, 1, AMBER);
    hc.fill_rect(frame_buf, PANEL_X, PANEL_Y + h - 1, PANEL_W, 1, AMBER);
    let x = PANEL_X + PAD;
    let mut y = PANEL_Y + PAD;
    hc.text(frame_buf, x, y, c.title, AMBER);
    y += ROW + 4;
    for line in wrap(c.goal, LINE_CHARS) {
        hc.text(frame_buf, x, y, &line, INK);
        y += ROW;
    }
    y += 4;
    for (label, body, colour) in [("SCORE:", c.score, GREEN), ("YOU LOSE:", c.lose, RED)] {
        for (i, line) in wrap(&format!("{label} {body}"), LINE_CHARS).into_iter().enumerate() {
            if i == 0 {
                hc.text(frame_buf, x, y, label, colour);
                let rest = line[label.len()..].to_string();
                hc.text(frame_buf, x + text_width(label), y, &rest, INK);
            } else {
                hc.text(frame_buf, x, y, &line, INK);
            }
            y += ROW;
        }
    }
    y += 6;
    hc.text(frame_buf, x, y, "KEYS", MUTED);
    y += ROW;
    let keys = all_keys(&c);
    let rows = keys.len().div_ceil(2);
    for (i, (k, what)) in keys.iter().enumerate() {
        // Down the left column first, then the right, so a seat's own keys
        // read in order before the shared ones.
        let (col, row) = (i / rows, i % rows);
        let kx = x + col as i32 * COL_W;
        let ky = y + row as i32 * ROW;
        hc.text(frame_buf, kx, ky, k, AMBER);
        hc.text(frame_buf, kx + KEY_W, ky, what, INK);
    }
    y += ROW * rows as i32 + 6;
    let foot = "PRESS SPACE TO PLAY -- ANY KEY CLOSES THIS CARD";
    hc.text(frame_buf, PANEL_X + (PANEL_W - text_width(foot)) / 2, y, foot, MUTED);
}

/// **The reminder while the card is shut**: one short line, top right, so
/// the player always knows where the explanation went.
pub fn draw_reminder(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let Some(demo) = lab.demo.as_ref() else { return };
    if demo.card {
        return;
    }
    let t = "H: GOAL AND KEYS";
    let w = text_width(t) + 8;
    let x = super::super::WIDTH as i32 - w - 4;
    let y = REMINDER_Y;
    crate::render::paint_owner::set("SEAT CARD");
    hc.blend_rect(frame_buf, x, y, w, GLYPH_HEIGHT + 2, [12, 14, 18, 255], 0.6);
    hc.text(frame_buf, x + 4, y + 1, t, MUTED);
}

/// The reminder's top edge: the row of the lab's own FRAME counter, above
/// where every seat's panel starts (the queen's ledger and the scout's panel
/// both open at about 12).
const REMINDER_Y: i32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    /// Every card fits above the lab's bar, every key line fits its column,
    /// and every character is one the HUD font can draw. The positive
    /// control is the length check itself: `KEY_DESC_MAX` is computed from
    /// the column, so a description one character too long fails here.
    #[test]
    fn every_card_fits_its_panel() {
        for seat in DemoSeat::ALL {
            let c = card(seat);
            let bottom = PANEL_Y + panel_height(&c);
            assert!(bottom <= crate::lab::ui::bar_top() - 4, "{seat:?}: card runs to {bottom}, under the bar");
            assert!(text_width(c.title) <= PANEL_W - 2 * PAD, "{seat:?}: title too wide");
            for (k, what) in all_keys(&c) {
                assert!(text_width(k) + 4 <= KEY_W, "{seat:?}: key label {k} too wide");
                assert!(what.len() <= KEY_DESC_MAX, "{seat:?}: '{what}' is {} > {KEY_DESC_MAX}", what.len());
            }
            let all = format!("{} {} {} {}", c.title, c.goal, c.score, c.lose);
            for ch in all.chars().chain(all_keys(&c).iter().flat_map(|(k, w)| k.chars().chain(w.chars()))) {
                assert!(crate::hud::has_glyph(ch), "{seat:?}: no glyph for {ch:?}");
            }
        }
    }

    /// **The card's keys**: the key that closes it does nothing else, `H`
    /// mid-game pauses and closing resumes, `Space` over it plays and is the
    /// lab's again once it is down, and a warm-up ending under it waits.
    #[test]
    fn the_card_takes_one_key_and_h_brings_it_back() {
        use super::super::{Demo, DemoKey};
        let sc = crate::lab::scenario::Scenario::load("steady_income").expect("steady_income ships");
        let mut lab = Lab::new(sc.bed.clone());
        lab.load_scenario(sc);
        lab.demo = Some(Demo::new(DemoSeat::Scout));
        lab.demo.as_mut().unwrap().window = true;
        let up = |lab: &Lab| lab.demo.as_ref().is_some_and(|d| d.card);
        lab.time.pause();
        lab.open_seat_card(false);
        assert!(up(&lab));
        assert!(lab.demo_key(DemoKey::Right, true), "the card let a key through");
        assert!(!up(&lab), "a key did not close the card");
        assert!(!lab.demo.as_ref().unwrap().held.right, "the key that closed the card also steered");

        lab.time.toggle_phase();
        assert!(lab.time.multiple() > 0);
        assert!(lab.demo_key(DemoKey::Letter('H'), true));
        assert!(up(&lab) && lab.time.multiple() == 0, "H did not pause under the card");
        assert!(lab.demo_key(DemoKey::Letter('Y'), true));
        assert!(!up(&lab) && lab.time.multiple() > 0, "closing did not resume the clock H paused");

        lab.time.pause();
        lab.open_seat_card(false);
        assert!(lab.demo_space(), "Space over the card went to the lab");
        assert!(!up(&lab) && lab.time.multiple() > 0, "Space over the card did not play");
        assert!(!lab.demo_space(), "Space with no card was taken from the lab");

        lab.time.set_preset(5);
        lab.open_seat_card(false);
        lab.demo_frame();
        assert!(lab.time.multiple() > super::super::STEER_CAP, "the card paused a warm-up");
        lab.time.set_preset(0);
        lab.demo_frame();
        assert!(up(&lab) && lab.time.multiple() == 0, "the warm-up ended under the card and did not wait");
        lab.close_seat_card();
        assert!(lab.time.multiple() > 0, "closing did not resume after the warm-up");
    }

    #[test]
    fn wrap_keeps_every_word_and_the_limit() {
        let t = "ONE TWO THREE FOUR FIVE SIX SEVEN";
        let lines = wrap(t, 9);
        assert!(lines.iter().all(|l| l.len() <= 9), "{lines:?}");
        assert_eq!(lines.join(" "), t);
    }
}
