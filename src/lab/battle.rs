//! **The battle view** (F9): who is fighting whom, where, and who is winning,
//! drawn over the box.
//!
//! Asked for by the owner on 2026-10-03 ("visual, user-friendly ways to track
//! the battle"), mocked first as pictures on the colony-wars page
//! (`examples/colonywar.rs` draws the same layers into a PNG) and picked as
//! four layers without a written log:
//!
//! - **Kill marks**: a cross where an animal of a colony was killed, in the
//!   victim's colony colour, fading over [`MARK_FRAMES`]. A kill by an animal
//!   with no colony (a beetle) is a red ring instead, so a predator's raid and
//!   a war between colonies read apart at a glance.
//! - **Territory**: a band along the top of the box, each column in the
//!   colour of the colony whose adults made up at least [`HOLD_SHARE`] of the
//!   ants seen in it lately. Borders move as colonies grow and shrink.
//! - **Scoreboard**: one line per colony (alive, killed, lost, columns held)
//!   and one per predator species (alive, ants eaten, lost to ants).
//! - **Strength strip**: ants alive per colony (solid) and food brought in
//!   per [`SNAP_FRAMES`] (dotted), so a colony losing to hunger and one losing
//!   to fighting look different. Measured on 18 lab beds, hunger is far more
//!   common.
//!
//! **Read-only.** Everything comes from the world's own records -- the kill
//! log, the graveyard (a kill record has no position; the victim's grave
//! does), the colony books -- and the view keeps only what it needs to draw.
//! **Shown only when there is a battle to show** (two colonies, or a colony
//! and a predator, have been in the box) unless switched off with F9: a box
//! with one colony draws nothing extra.

use std::collections::BTreeMap;

use crate::hud;
use crate::render::{self, Hud, Renderer};
use crate::sim::organism::{DeathCause, SpeciesId};
use crate::sim::world::{Account, World};

/// How long a kill mark stays, in frames.
pub const MARK_FRAMES: u64 = 6000;
/// How often the territory tally samples where the ants are.
const SAMPLE_FRAMES: u64 = 25;
/// Per sample, how much of the old tally survives: about 1,000 frames of
/// memory at one sample per 25 (`0.975^40` is a third).
const TALLY_KEEP: f32 = 0.975;
/// The share of a column's recent ants one colony needs to hold it.
const HOLD_SHARE: f32 = 0.7;
/// The least recent presence a column needs before anybody holds it.
const HOLD_MIN: f32 = 2.0;
/// One point on the strength strip per this many frames.
pub const SNAP_FRAMES: u64 = 1000;
/// Points kept on the strip.
const SNAPS_KEPT: usize = 120;

/// The colour of a kill by something with no colony.
const PREDATOR: [u8; 4] = [235, 60, 40, 255];
const OUTLINE: [u8; 4] = [12, 12, 14, 255];
const PANEL_BG: [u8; 4] = [17, 19, 24, 255];
const PANEL_EDGE: [u8; 4] = [88, 98, 114, 255];
const TITLE: [u8; 4] = [238, 238, 210, 255];
const VALUE: [u8; 4] = [228, 234, 240, 255];
const FAINT: [u8; 4] = [132, 139, 153, 255];

/// One killing, placed.
#[derive(Clone, Copy, Debug)]
struct Mark {
    frame: u64,
    at: (i32, i32),
    victim_colony: u32,
    /// The killer had no colony: a predator.
    by_predator: bool,
}

/// One point on the strength strip.
#[derive(Clone, Debug, Default)]
struct Snap {
    alive: BTreeMap<u32, u32>,
    food: BTreeMap<u32, f64>,
}

/// A side's running score.
#[derive(Clone, Copy, Debug, Default)]
pub struct Score {
    pub killed: u32,
    pub lost: u32,
}

/// **The battle view's state**: what it has read from the world so far.
#[derive(Clone, Debug)]
pub struct BattleView {
    /// F9. On unless switched off; even on, it draws only when there is a
    /// battle to show ([`BattleView::has_battle`]).
    pub on: bool,
    seen: usize,
    last_frame: u64,
    last_sample: u64,
    marks: Vec<Mark>,
    /// Per colony, the recent presence of its adults in each column.
    tally: BTreeMap<u32, Vec<f32>>,
    /// Per colony: kills of others, and its own lost to others.
    colonies: BTreeMap<u32, Score>,
    /// Per predator species (no colony): ants it killed, and its own lost to
    /// colony animals.
    predators: BTreeMap<u16, Score>,
    snaps: Vec<Snap>,
    next_snap: u64,
    food_seen: BTreeMap<u32, f64>,
    width: usize,
}

impl Default for BattleView {
    fn default() -> Self {
        Self {
            on: true,
            seen: 0,
            last_frame: 0,
            last_sample: 0,
            marks: Vec::new(),
            tally: BTreeMap::new(),
            colonies: BTreeMap::new(),
            predators: BTreeMap::new(),
            snaps: Vec::new(),
            next_snap: 0,
            food_seen: BTreeMap::new(),
            width: 0,
        }
    }
}

/// The colour a colony wears under "by colony" -- the same palette the
/// animals themselves use, so a mark and the ants it belongs to match.
pub fn colony_colour(colony: u32) -> [u8; 4] {
    let p = render::group_palette(colony.saturating_sub(1) as usize);
    [p[0] as u8, p[1] as u8, p[2] as u8, 255]
}

/// Does this species live in colonies (it keeps brood)?
fn is_colonial(world: &World, sid: SpeciesId) -> bool {
    world.species.get(sid).creature.as_ref().is_some_and(|d| d.brood.is_some())
}

impl BattleView {
    /// F9: flip the view. Returns the new state.
    pub fn toggle(&mut self) -> bool {
        self.on = !self.on;
        self.on
    }

    /// **Is there a battle to show?** Two colonies have had ants in the box,
    /// or a predator has killed or been killed by a colony animal.
    pub fn has_battle(&self) -> bool {
        self.colonies.len() >= 2 || !self.predators.is_empty()
    }

    /// Read what is new in the world since the last call. Cheap when nothing
    /// happened: the kill log is read from where it was left, and the ants
    /// are walked once per [`SAMPLE_FRAMES`].
    pub fn update(&mut self, world: &World) {
        let width = world.bounds().map_or(0, |b| (b.max_x - b.min_x + 1).max(0) as usize);
        // A different box (a rebuild, a chamber switch, a scenario load)
        // starts the view again rather than carrying marks from another world.
        if world.frame < self.last_frame || world.kills_log.len() < self.seen || width != self.width {
            let on = self.on;
            *self = Self { on, width, ..Self::default() };
        }
        self.last_frame = world.frame;
        // ---- kills, placed by the victim's grave
        if world.kills_log.len() > self.seen {
            let mut used = Vec::new();
            for k in &world.kills_log[self.seen..] {
                let victim_col = is_colonial(world, k.victim_species) && k.victim_colony != 0;
                let attacker_col = is_colonial(world, k.attacker_species) && k.attacker_colony != 0;
                if !victim_col && !attacker_col {
                    continue;
                }
                if victim_col && attacker_col && k.victim_colony == k.attacker_colony {
                    continue;
                }
                match (attacker_col, victim_col) {
                    (true, _) => self.colonies.entry(k.attacker_colony).or_default().killed += 1,
                    (false, _) => self.predators.entry(k.attacker_species.0).or_default().killed += 1,
                }
                match victim_col {
                    true => self.colonies.entry(k.victim_colony).or_default().lost += 1,
                    false => self.predators.entry(k.victim_species.0).or_default().lost += 1,
                }
                if !victim_col {
                    continue;
                }
                let grave = world.graveyard.recent().find(|g| {
                    g.died_frame == k.frame && g.colony == k.victim_colony && g.creature && g.cause == DeathCause::Killed && !used.contains(&g.id)
                });
                if let Some(g) = grave {
                    used.push(g.id);
                    self.marks.push(Mark { frame: k.frame, at: g.at, victim_colony: k.victim_colony, by_predator: !attacker_col });
                }
            }
            self.seen = world.kills_log.len();
        }
        let now = world.frame;
        self.marks.retain(|m| now.saturating_sub(m.frame) < MARK_FRAMES);
        // ---- where each colony's adults are, and how many there are
        if now < self.last_sample + SAMPLE_FRAMES && self.last_sample != 0 {
            return;
        }
        // **Forget by elapsed frames, not by samples taken.** The view is
        // read at the draw rate, and at the fast speeds a draw can be a
        // thousand ticks apart; a per-sample decay would forget faster the
        // faster the clock ran.
        let elapsed = now.saturating_sub(self.last_sample).max(1) as f32;
        let keep = TALLY_KEEP.powf(elapsed / SAMPLE_FRAMES as f32);
        self.last_sample = now.max(1);
        let x0 = world.bounds().map_or(0, |b| b.min_x);
        for v in self.tally.values_mut() {
            for p in v.iter_mut() {
                *p *= keep;
            }
        }
        let mut alive: BTreeMap<u32, u32> = BTreeMap::new();
        for id in world.live_organism_ids() {
            let Some(st) = world.organism(id) else { continue };
            if st.colony == 0 || st.brood.is_some() || !is_colonial(world, st.species) {
                continue;
            }
            *alive.entry(st.colony).or_default() += 1;
            self.colonies.entry(st.colony).or_default();
            let Some(&(x, _)) = st.chain.first() else { continue };
            let i = (x - x0) as usize;
            if i < width {
                self.tally.entry(st.colony).or_insert_with(|| vec![0.0; width])[i] += 1.0;
            }
        }
        // ---- the strength strip, one point per SNAP_FRAMES
        if now >= self.next_snap {
            self.next_snap = (now / SNAP_FRAMES + 1) * SNAP_FRAMES;
            let mut snap = Snap { alive, ..Snap::default() };
            for &c in self.colonies.keys() {
                let books = world.colony_books(c);
                let total = books.get(Account::HarvestedPlant) + books.get(Account::HarvestedCorpse);
                let before = self.food_seen.insert(c, total).unwrap_or(total);
                snap.food.insert(c, (total - before).max(0.0));
            }
            self.snaps.push(snap);
            if self.snaps.len() > SNAPS_KEPT {
                self.snaps.remove(0);
            }
        }
    }

    /// Which colony holds each column: `None` where nobody does.
    pub fn held(&self) -> Vec<Option<u32>> {
        (0..self.width)
            .map(|x| {
                let total: f32 = self.tally.values().map(|v| v[x]).sum();
                if total < HOLD_MIN {
                    return None;
                }
                self.tally.iter().find(|(_, v)| v[x] / total >= HOLD_SHARE).map(|(&c, _)| c)
            })
            .collect()
    }

    /// The scoreboard's lines, in the order drawn: (colour, text).
    pub fn score_lines(&self, world: &World) -> Vec<([u8; 4], String)> {
        let held = self.held();
        let alive = self.snaps.last().map(|s| s.alive.clone()).unwrap_or_default();
        let mut out = Vec::new();
        for (&c, s) in &self.colonies {
            let cols = held.iter().filter(|h| **h == Some(c)).count();
            let n = alive.get(&c).copied().unwrap_or(0);
            let name = if n == 0 { format!("COLONY {c} GONE") } else { format!("COLONY {c}  {n} ALIVE") };
            out.push((colony_colour(c), format!("{name}  KILLED {}  LOST {}  HOLDS {cols}", s.killed, s.lost)));
        }
        for (&raw, s) in &self.predators {
            let sid = SpeciesId(raw);
            let name = world.species.get(sid).name.to_uppercase();
            let n = world
                .live_organism_ids()
                .into_iter()
                .filter(|&id| world.organism(id).is_some_and(|st| st.species == sid && st.brood.is_none()))
                .count();
            out.push((PREDATOR, format!("{name}  {n} ALIVE  ATE {} ANTS  LOST {}", s.killed, s.lost)));
        }
        out
    }

    /// Kill marks and the territory band, drawn over the world.
    pub fn draw_world(&self, hc: Hud, frame: &mut [u8], world: &World, renderer: &Renderer, bottom: i32) {
        if !self.on || !self.has_battle() {
            return;
        }
        let Some(b) = world.bounds() else { return };
        // ---- territory: a band along the top of the box
        let (_, top, _, _, _) = renderer.world_rect_to_logical(b.min_x, b.min_y, b.min_x, b.min_y);
        let band_y = top.max(0);
        for (i, h) in self.held().into_iter().enumerate() {
            let Some(c) = h else { continue };
            let x = b.min_x + i as i32;
            let (lx0, _, lx1, _, _) = renderer.world_rect_to_logical(x, b.min_y, x, b.min_y);
            for lx in lx0..=lx1 {
                for dy in 0..3 {
                    hc.put(frame, lx, band_y + dy, colony_colour(c));
                }
            }
        }
        // ---- kill marks, oldest first so the newest sit on top
        for m in &self.marks {
            let age = world.frame.saturating_sub(m.frame) as f32 / MARK_FRAMES as f32;
            let k = 1.0 - 0.6 * age.clamp(0.0, 1.0);
            let base = if m.by_predator { PREDATOR } else { colony_colour(m.victim_colony) };
            let c = [(base[0] as f32 * k) as u8, (base[1] as f32 * k) as u8, (base[2] as f32 * k) as u8, 255];
            let (x0, y0, x1, y1, _) = renderer.world_rect_to_logical(m.at.0, m.at.1, m.at.0, m.at.1);
            let (cx, cy) = ((x0 + x1) / 2, (y0 + y1) / 2);
            if cy >= bottom || cy < 0 {
                continue;
            }
            // Each mark sits on a dark outline one pixel wider, so it reads
            // against grass, soil and sky alike -- a full replace on fixed
            // colours, never a blend into what is underneath.
            let shape: Vec<(i32, i32)> = if m.by_predator {
                // A ring three out.
                vec![(-3, -1), (-3, 0), (-3, 1), (3, -1), (3, 0), (3, 1), (-1, -3), (0, -3), (1, -3), (-1, 3), (0, 3), (1, 3), (-2, -2), (2, -2), (-2, 2), (2, 2)]
            } else {
                (-3..=3).flat_map(|d| [(d, d), (d, -d)]).collect()
            };
            for &(dx, dy) in &shape {
                for (ox, oy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    hc.put(frame, cx + dx + ox, cy + dy + oy, OUTLINE);
                }
            }
            for &(dx, dy) in &shape {
                hc.put(frame, cx + dx, cy + dy, c);
            }
        }
    }

    /// The scoreboard and strength strip, in a panel at `(x, y)` (top-right
    /// corner of the panel at `right`).
    pub fn draw_panel(&self, hc: Hud, frame: &mut [u8], world: &World, right: i32, y: i32) {
        if !self.on || !self.has_battle() {
            return;
        }
        let lines = self.score_lines(world);
        let line_h = hud::GLYPH_HEIGHT + 2;
        let text_w = lines.iter().map(|(_, s)| hud::text_width(s)).max().unwrap_or(0).max(hud::text_width("BATTLE  F9 HIDES"));
        let strip_h = 34;
        let w = text_w + 18;
        let h = 6 + line_h * (lines.len() as i32 + 1) + 4 + strip_h + line_h + 4;
        let x = right - w;
        for yy in y..y + h {
            for xx in x..x + w {
                hc.put(frame, xx, yy, PANEL_BG);
            }
        }
        for xx in x..x + w {
            hc.put(frame, xx, y, PANEL_EDGE);
            hc.put(frame, xx, y + h - 1, PANEL_EDGE);
        }
        for yy in y..y + h {
            hc.put(frame, x, yy, PANEL_EDGE);
            hc.put(frame, x + w - 1, yy, PANEL_EDGE);
        }
        hc.text(frame, x + 6, y + 4, "BATTLE  F9 HIDES", TITLE);
        for (i, (c, s)) in lines.iter().enumerate() {
            let ly = y + 4 + line_h * (i as i32 + 1);
            for dy in 1..hud::GLYPH_HEIGHT - 1 {
                for dx in 0..4 {
                    hc.put(frame, x + 6 + dx, ly + dy, *c);
                }
            }
            hc.text(frame, x + 13, ly, s, VALUE);
        }
        // ---- the strength strip
        let sy = y + 6 + line_h * (lines.len() as i32 + 1);
        let (sx, sw) = (x + 6, w - 12);
        for xx in sx..sx + sw {
            hc.put(frame, xx, sy + strip_h, FAINT);
        }
        let n = self.snaps.len();
        if n >= 2 {
            let max_alive = self.snaps.iter().flat_map(|s| s.alive.values().copied()).max().unwrap_or(1).max(1) as f32;
            let max_food = self.snaps.iter().flat_map(|s| s.food.values().copied()).fold(0.0f64, f64::max).max(1e-9);
            let px = |i: usize| sx + (i as i32 * (sw - 1)) / (SNAPS_KEPT as i32 - 1).max(1);
            for &c in self.colonies.keys() {
                let col = colony_colour(c);
                let mut prev: Option<(i32, i32)> = None;
                for (i, s) in self.snaps.iter().enumerate() {
                    let v = s.alive.get(&c).copied().unwrap_or(0) as f32 / max_alive;
                    let p = (px(i), sy + strip_h - 1 - (v * (strip_h - 2) as f32) as i32);
                    if let Some(q) = prev {
                        segment(hc, frame, q, p, col);
                    }
                    prev = Some(p);
                }
                // Food: a dot per point, every other point, so it reads as a
                // dotted line beside the solid one.
                for (i, s) in self.snaps.iter().enumerate().filter(|(i, _)| i % 2 == 0) {
                    let v = (s.food.get(&c).copied().unwrap_or(0.0) / max_food) as f32;
                    hc.put(frame, px(i), sy + strip_h - 1 - (v * (strip_h - 2) as f32) as i32, col);
                }
            }
        }
        hc.text(frame, x + 6, sy + strip_h + 3, "LINE ANTS ALIVE  DOTS FOOD IN", FAINT);
    }
}

/// A one-pixel line between two logical points.
fn segment(hc: Hud, frame: &mut [u8], (x0, y0): (i32, i32), (x1, y1): (i32, i32), colour: [u8; 4]) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for s in 0..=steps {
        let x = x0 + (x1 - x0) * s / steps;
        let y = y0 + (y1 - y0) * s / steps;
        hc.put(frame, x, y, colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Two colonies in one column: whoever has 70% of the recent ants holds
    /// it, and a column split evenly is held by nobody.** The positive and the
    /// negative case of the one rule the band draws.
    #[test]
    fn a_column_is_held_only_by_a_clear_majority() {
        let mut v = BattleView { width: 3, ..BattleView::default() };
        v.tally.insert(1, vec![8.0, 5.0, 0.5]);
        v.tally.insert(2, vec![1.0, 5.0, 0.5]);
        assert_eq!(v.held(), vec![Some(1), None, None]);
    }
}
