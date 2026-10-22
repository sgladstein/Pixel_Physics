//! **The general nest score: how much of a dug nest is well built**, by
//! properties rather than against one fixed blueprint.
//!
//! A port of the teacher lane's prototype,
//! `Reports/brain-training/teacher-training/measurements/blueprints/nestscore.py`
//! (PR #753; the design is section 6.1 of `teacher-design-2026-10-21.md`),
//! made at the owner's go on 2026-10-21 **as a printed number only**: no
//! trainer is paid by it.
//!
//! **The climb, and the niche that fixes it** (owner: "port it first then fix
//! the climber"). As built ([`PROTOTYPE`]) a chamber dug floor first earned
//! nothing until its fifth row -- rows 1-2 read as cracks and were charged,
//! rows 3-4 as tunnel already at its allowance (24, 10, -4, 23, 22, then 125)
//! -- and letting low rooms count as chambers instead made a corridor maze
//! the best nest (23 -> 1,238), because in a 2D slice a flat room and a
//! corridor are one shape. What tells them apart is the ways in: a room being
//! dug is a dead end or a bulge on a passage, a corridor is crossed by every
//! tunnel it meets. So [`HARVESTER`] adds **niches**: what is left after the
//! chambers, at least 7 wide in every row, flat-floored, at least 3x as wide
//! as tall, with **at most two openings**, paid on height from the first row
//! (full at 6, where it is nearly a chamber). The climb then rises every row
//! (24, 30, 45, 70, 105, 125, 208, 238), the maze stays at 23, and the
//! blueprints move by at most 55 and keep their order. **Niche credit is
//! capped** at 120 cells, or half what finished chambers earn: uncapped, a
//! comb of one-row shelves off a shaft scored 1,731 against the harvester's
//! 761; capped it scores 144-174, under every 7-cell blueprint (the lowest,
//! 264). The price: a shelf comb that was charged -756 to -994 as cracks now
//! reads as a small positive nest. Prototype: `climb2.py`, project files
//! `nest-score/`.
//!
//! The owner's question it answers (2026-10-21): "can we reward more general
//! things like number of well proportioned tunnels and chambers?", with
//! chambers 7 cells tall. So a nest is scored for what it *has*, graded, never
//! pass/fail, in **well-built cells**: `chambers + 0.5 x tunnels - bad`.
//!
//! - **Chambers**: roofed rooms with a core 7 wide and 5 tall (a 7x5 box fits
//!   inside every part of one, so a passage up to 6 wide at any angle is never
//!   a chamber; a 5x5 core let 5-wide shafts through, measured on the
//!   staircase blueprint), each graded on **height** (full 6-8, none at 3 or
//!   13), **proportion** (full at width >= 3x height, none at 1x) and a **flat
//!   floor**. Credit is area x grade with the area capped, so one enormous
//!   room cannot stand in for several.
//! - **Tunnels**: the rest of the nest, graded per cell on width across (full
//!   3-6, none at 1 or 10), and credited **only as much as the chambers they
//!   serve** (or a small allowance for the shaft), so a maze is not a nest.
//! - **Layout** (species): the harvester is top-heavy -- the share of chamber
//!   area in the upper half scales chamber credit 0.75-1.
//! - **Charged**: cracks (open cells under 3 wide), a pit open to the sky away
//!   from the mouth, and open cells not joined to the door.
//!
//! Only below-ground cells joined to the door count; the door's mouth may be
//! open to the sky over `mouth` columns either side. A wide crater mouth is
//! charged as pit but does not cut the nest under it off (it did in the
//! prototype's first version: 1,072 cells of the random-walk cave read
//! "stray").
//!
//! **The port is held to the prototypes cell for cell**: the 15 controls,
//! the 8-step climb, a niche mid-shaft and 4 shelf combs are exported from
//! the Python with both prototypes' numbers (`nestscore-controls.txt`), and
//! the tests read every case back.
//!
//! **Nothing here touches the world or a random stream** -- it is a read --
//! so printing it cannot move a run (the brain lane's guards compare runs bit
//! for bit).

use crate::lab::census::{extents, surface_of, MOUND_REACH};
use crate::lab::scene::LabBox;
use crate::sim::cell::Cell;
use crate::sim::creature::food_value;
use crate::sim::material::{MaterialId, MaterialKind};
use crate::sim::world::World;
use std::collections::VecDeque;

/// A species' nest: the shapes the score pays for. One today, the owner's pick.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    /// Chamber height (cells) at full credit, low and high.
    pub h_full: (f64, f64),
    /// Chamber height at no credit, low and high.
    pub h_zero: (f64, f64),
    /// Width over height at full credit, and at none.
    pub aspect_full: f64,
    pub aspect_zero: f64,
    /// Tunnel width across at full credit, and at none.
    pub w_full: (f64, f64),
    pub w_zero: (f64, f64),
    /// Half the chamber core, `(x, y)`: (3, 2) is a 7-wide, 5-tall box.
    pub core: (usize, usize),
    /// The fewest cells a chamber needs.
    pub min_room: usize,
    /// The most cells of one chamber that are paid.
    pub room_cap: usize,
    /// Tunnel cells paid before there is any chamber (the shaft).
    pub tunnel_free: f64,
    /// The share of chamber area wanted in the upper half, or `None`.
    pub top_heavy: Option<f64>,
    /// Columns either side of the door that may be open to the sky.
    pub mouth: i32,
    /// Open nest cells an ant: about 8% of floor under bodies, the real
    /// crowding figure.
    pub cells_per_ant: usize,
    /// Rooms on their way to being chambers, or `None` for the prototype's
    /// score as built.
    pub niche: Option<Niche>,
}

/// **The climb fix** (owner, 2026-10-21: "port it first then fix the
/// climber"): what a niche is and what it is paid.
#[derive(Clone, Copy, Debug)]
pub struct Niche {
    /// Half the bar a niche must hold in every part: (3, 0) is 7 wide, 1 tall.
    pub bar: (usize, usize),
    /// The fewest cells a niche needs: one row of the bar.
    pub min_cells: usize,
    /// Height (cells) at no credit, and at full.
    pub h_zero: f64,
    pub h_full: f64,
    /// Openings at full credit, and at none.
    pub open_full: f64,
    pub open_zero: f64,
    /// Niche credit paid before there is any chamber, and the share of the
    /// chambers' credit it may reach after.
    pub free: f64,
    pub share: f64,
}

/// The niche as measured on the climb and the combs (module doc).
pub const NICHE: Niche = Niche {
    bar: (3, 0),
    min_cells: 7,
    h_zero: 0.0,
    h_full: 6.0,
    open_full: 2.0,
    open_zero: 3.0,
    free: 120.0,
    share: 0.5,
};

/// **The harvester stack, chambers 7 cells tall** (owner's pick, 2026-10-21).
pub const HARVESTER: Profile = Profile {
    h_full: (6.0, 8.0),
    h_zero: (3.0, 13.0),
    aspect_full: 3.0,
    aspect_zero: 1.0,
    w_full: (3.0, 6.0),
    w_zero: (1.0, 10.0),
    core: (3, 2),
    min_room: 30,
    room_cap: 400,
    tunnel_free: 60.0,
    top_heavy: Some(0.5),
    mouth: 4,
    cells_per_ant: 25,
    niche: Some(NICHE),
};

/// **The prototype's score as built** (`nestscore.py`, PR #753): the
/// harvester with no niches. Kept so the port stays checked against the
/// Python cell for cell, and so the climb's valley stays on record.
pub const PROTOTYPE: Profile = Profile {
    niche: None,
    ..HARVESTER
};

/// The harvester's shapes without its layout preference.
pub const GENERAL: Profile = Profile {
    top_heavy: None,
    ..HARVESTER
};

/// One chamber the score found.
#[derive(Clone, Copy, Debug, Default)]
pub struct Chamber {
    pub area: usize,
    /// Median column height over the chamber's full-height columns.
    pub height: f64,
    /// Columns it spans.
    pub width: f64,
    /// Share of full-height columns whose floor is within a row of the median.
    pub flat: f64,
    /// 0-1: height x proportion x (0.5 + 0.5 flat).
    pub grade: f64,
    /// `grade x min(area, room_cap)`, before layout.
    pub credit: f64,
    /// Mean row of its cells.
    pub depth: f64,
    /// Ways into it from the rest of the nest (niches only; 0 for a chamber).
    pub openings: usize,
}

/// What the score made of each cell, for a picture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// Ground, sky, or open air above the ground.
    Not,
    Chamber,
    /// A room on its way to being a chamber.
    Niche,
    Tunnel,
    /// A tunnel cell under 3 wide.
    Crack,
    /// Open to the sky below the ground, away from the mouth.
    Pit,
    /// Open below the ground but not joined to the door.
    Stray,
}

/// The score and every part of it, unrounded.
#[derive(Clone, Debug, Default)]
pub struct NestScore {
    /// Well-built cells: `chamber_credit + niche_credit + 0.5 x tunnel_credit - bad`.
    pub score: f64,
    /// Nest cells: below ground, joined to the door, not pit.
    pub dug: usize,
    pub chambers: Vec<Chamber>,
    /// Chamber credit after layout.
    pub chamber_credit: f64,
    pub layout: f64,
    pub niches: Vec<Chamber>,
    /// Niche credit after its cap.
    pub niche_credit: f64,
    pub tunnel_cells: usize,
    /// Tunnel credit after its cap.
    pub tunnel_credit: f64,
    pub cracks: usize,
    pub pit: usize,
    pub stray: usize,
}

impl NestScore {
    /// Score per nest cell: how well, whatever the size.
    pub fn per_cell(&self) -> f64 {
        if self.dug > 0 {
            self.score / self.dug as f64
        } else {
            0.0
        }
    }

    /// The sum of the chambers' grades: "how many good chambers".
    pub fn good_chambers(&self) -> f64 {
        self.chambers.iter().map(|c| c.grade).sum()
    }

    /// The share of `ants x cells_per_ant` reached, 0-1.
    pub fn fit(&self, ants: usize, p: &Profile) -> f64 {
        let target = (ants * p.cells_per_ant) as f64;
        if target > 0.0 {
            (self.score.max(0.0) / target).min(1.0)
        } else {
            0.0
        }
    }
}

/// 0 at `zero`, 1 at `full`, linear between, clipped (either direction).
fn ramp(v: f64, zero: f64, full: f64) -> f64 {
    if full == zero {
        return f64::from(u8::from(v >= full));
    }
    ((v - zero) / (full - zero)).clamp(0.0, 1.0)
}

/// 1 inside `full`, falling linearly to 0 at `zero`.
fn band(v: f64, full: (f64, f64), zero: (f64, f64)) -> f64 {
    ramp(v, zero.0, full.0).min(ramp(v, zero.1, full.1))
}

/// numpy's median: the mean of the middle two when the count is even.
fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// 4-connected components of `mask`, each its cells in discovery order.
fn components(mask: &[bool], w: usize, h: usize) -> (Vec<u32>, Vec<Vec<(usize, usize)>>) {
    let mut lab = vec![0u32; w * h];
    let mut comps = Vec::new();
    for i in 0..w * h {
        if !mask[i] || lab[i] != 0 {
            continue;
        }
        let id = comps.len() as u32 + 1;
        lab[i] = id;
        let mut q = VecDeque::from([(i % w, i / w)]);
        let mut cells = Vec::new();
        while let Some((x, y)) = q.pop_front() {
            cells.push((x, y));
            let mut visit = |nx: usize, ny: usize| {
                let j = ny * w + nx;
                if mask[j] && lab[j] == 0 {
                    lab[j] = id;
                    q.push_back((nx, ny));
                }
            };
            if y + 1 < h {
                visit(x, y + 1);
            }
            if y > 0 {
                visit(x, y - 1);
            }
            if x + 1 < w {
                visit(x + 1, y);
            }
            if x > 0 {
                visit(x - 1, y);
            }
        }
        comps.push(cells);
    }
    (lab, comps)
}

/// `m` shifted by `(dx, dy)` (`out[y][x] = m[y + dy][x + dx]`), zero off the edge.
fn shifted(m: &[bool], w: usize, h: usize, dx: isize, dy: isize) -> impl Fn(usize, usize) -> bool + '_ {
    move |x, y| {
        let (sx, sy) = (x as isize + dx, y as isize + dy);
        sx >= 0 && sy >= 0 && (sx as usize) < w && (sy as usize) < h && m[sy as usize * w + sx as usize]
    }
}

/// Cells whose `(2rx+1) x (2ry+1)` box lies wholly inside `m`; off the edge
/// counts as outside.
fn erode(m: &[bool], w: usize, h: usize, (rx, ry): (usize, usize)) -> Vec<bool> {
    let mut out = m.to_vec();
    for dy in -(ry as isize)..=ry as isize {
        for dx in -(rx as isize)..=rx as isize {
            let s = shifted(m, w, h, dx, dy);
            for y in 0..h {
                for x in 0..w {
                    out[y * w + x] &= s(x, y);
                }
            }
        }
    }
    out
}

/// Every cell within a `(2rx+1) x (2ry+1)` box of a cell of `m`.
fn dilate(m: &[bool], w: usize, h: usize, (rx, ry): (usize, usize)) -> Vec<bool> {
    let mut out = m.to_vec();
    for dy in -(ry as isize)..=ry as isize {
        for dx in -(rx as isize)..=rx as isize {
            let s = shifted(m, w, h, dx, dy);
            for y in 0..h {
                for x in 0..w {
                    out[y * w + x] |= s(x, y);
                }
            }
        }
    }
    out
}

/// Length of the straight run of `m` cells through each cell along `(dx, dy)`.
fn runs(m: &[bool], w: usize, h: usize, dx: isize, dy: isize) -> Vec<u32> {
    let at = |x: isize, y: isize| -> Option<usize> {
        (x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h).then(|| y as usize * w + x as usize)
    };
    let mut fwd = vec![0u32; w * h];
    let mut bwd = vec![0u32; w * h];
    // Rows in the direction of `dy` (all callers have dy >= 0, so a row's
    // predecessor is done before it), columns in the direction of `dx`.
    let xs: Vec<isize> = if dx >= 0 {
        (0..w as isize).collect()
    } else {
        (0..w as isize).rev().collect()
    };
    let ys: Vec<isize> = if dy >= 0 {
        (0..h as isize).collect()
    } else {
        (0..h as isize).rev().collect()
    };
    for &y in &ys {
        for &x in &xs {
            let i = at(x, y).expect("in bounds");
            if m[i] {
                fwd[i] = 1 + at(x - dx, y - dy).map_or(0, |p| fwd[p]);
            }
        }
    }
    for &y in ys.iter().rev() {
        for &x in xs.iter().rev() {
            let i = at(x, y).expect("in bounds");
            if m[i] {
                bwd[i] = 1 + at(x + dx, y + dy).map_or(0, |n| bwd[n]);
            }
        }
    }
    fwd.iter().zip(&bwd).map(|(f, b)| (f + b).saturating_sub(1)).collect()
}

/// **The score of one grid.** `open` is `w x h`, row-major, true where an
/// ant could stand (air, or anything not ground); `ground` is each column's
/// original top-of-ground row (a cell is below ground at that row or deeper);
/// `door_x` is the mouth's column. Returns the score and each cell's class.
pub fn score(open: &[bool], w: usize, h: usize, ground: &[i32], door_x: usize, p: &Profile) -> (NestScore, Vec<Class>) {
    assert_eq!(open.len(), w * h, "the grid is w x h");
    assert_eq!(ground.len(), w, "one ground row a column");
    let below = |x: usize, y: usize| y as i32 >= ground[x];
    // Open to the sky: an unbroken open column from the top down to the cell.
    let mut sky = vec![false; w * h];
    for x in 0..w {
        let mut y = 0;
        while y < h && open[y * w + x] {
            sky[y * w + x] = true;
            y += 1;
        }
    }
    let (m0, m1) = (
        (door_x as i32 - p.mouth).max(0) as usize,
        (door_x as i32 + p.mouth + 1).min(w as i32) as usize,
    );
    let in_mouth = |x: usize| x >= m0 && x < m1;
    let pit: Vec<bool> = (0..w * h)
        .map(|i| sky[i] && below(i % w, i / w) && !in_mouth(i % w))
        .collect();
    // Joined to the door through any open cell below ground, a crater
    // included: a wide mouth is charged as pit but does not cut the nest
    // under it off.
    let reach: Vec<bool> = (0..w * h).map(|i| open[i] && below(i % w, i / w)).collect();
    let (lab, _) = components(&reach, w, h);
    let g = ground[door_x].max(0) as usize;
    let mut door_ids = Vec::new();
    for x in m0..m1 {
        for y in g..(g + 3).min(h) {
            let id = lab[y * w + x];
            if id != 0 && !door_ids.contains(&id) {
                door_ids.push(id);
            }
        }
    }
    let joined: Vec<bool> = lab.iter().map(|id| *id != 0 && door_ids.contains(id)).collect();
    let nest: Vec<bool> = (0..w * h).map(|i| joined[i] && !pit[i]).collect();
    let stray: Vec<bool> = (0..w * h).map(|i| reach[i] && !joined[i] && !pit[i]).collect();

    // Chambers: the opening of the nest by the core box.
    let opened: Vec<bool> = dilate(&erode(&nest, w, h, p.core), w, h, p.core)
        .iter()
        .zip(&nest)
        .map(|(a, b)| *a && *b)
        .collect();
    let (_, ccomps) = components(&opened, w, h);
    let mut chambers = Vec::new();
    let mut chamber_mask = vec![false; w * h];
    let nest_rows: Vec<usize> = (0..h).filter(|&y| (0..w).any(|x| nest[y * w + x])).collect();
    let (top, bot) = (
        nest_rows.first().copied().unwrap_or(0),
        nest_rows.last().copied().unwrap_or(0),
    );
    for cells in &ccomps {
        if cells.len() < p.min_room {
            continue;
        }
        let mut c = room_of(cells);
        c.grade = band(c.height, p.h_full, p.h_zero)
            * ramp(c.width / c.height, p.aspect_zero, p.aspect_full)
            * (0.5 + 0.5 * c.flat);
        c.credit = c.grade * c.area.min(p.room_cap) as f64;
        chambers.push(c);
        for &(x, y) in cells {
            chamber_mask[y * w + x] = true;
        }
    }

    // Niches: rooms on their way to being chambers (module doc, *the climb*).
    let mut niches = Vec::new();
    let mut niche_mask = vec![false; w * h];
    if let Some(n) = p.niche {
        let rest: Vec<bool> = (0..w * h).map(|i| nest[i] && !chamber_mask[i]).collect();
        let bar: Vec<bool> = dilate(&erode(&rest, w, h, n.bar), w, h, n.bar)
            .iter()
            .zip(&rest)
            .map(|(a, b)| *a && *b)
            .collect();
        let (_, ncomps) = components(&bar, w, h);
        for cells in &ncomps {
            if cells.len() < n.min_cells {
                continue;
            }
            let mut c = room_of(cells);
            c.openings = openings(cells, &nest, w, h);
            let shape = ramp(c.width / c.height, p.aspect_zero, p.aspect_full)
                * (0.5 + 0.5 * c.flat)
                * ramp(c.openings as f64, n.open_zero, n.open_full);
            if shape <= 0.0 {
                continue;
            }
            // A one-row slot is a niche at almost no credit rather than a
            // crack: the floor of a room being dug is not a defect.
            c.grade = ramp(c.height, n.h_zero, n.h_full) * shape;
            c.credit = c.grade * c.area as f64;
            niches.push(c);
            for &(x, y) in cells {
                niche_mask[y * w + x] = true;
            }
        }
    }

    // Tunnels: the rest of the nest, graded on width across (the shortest
    // straight run through the cell).
    let s2 = std::f64::consts::SQRT_2;
    let (r0, r1, r2, r3) = (
        runs(&nest, w, h, 1, 0),
        runs(&nest, w, h, 0, 1),
        runs(&nest, w, h, 1, 1),
        runs(&nest, w, h, -1, 1),
    );
    let mut class = vec![Class::Not; w * h];
    let (mut tunnel_cells, mut cracks, mut tgrade) = (0usize, 0usize, 0.0f64);
    for i in 0..w * h {
        if pit[i] {
            class[i] = Class::Pit;
        } else if stray[i] {
            class[i] = Class::Stray;
        } else if chamber_mask[i] {
            class[i] = Class::Chamber;
        } else if niche_mask[i] {
            class[i] = Class::Niche;
        } else if nest[i] {
            let width = (r0[i] as f64)
                .min(r1[i] as f64)
                .min(r2[i] as f64 * s2)
                .min(r3[i] as f64 * s2);
            tunnel_cells += 1;
            tgrade += band(width, p.w_full, p.w_zero);
            if width < 3.0 {
                cracks += 1;
                class[i] = Class::Crack;
            } else {
                class[i] = Class::Tunnel;
            }
        }
    }
    let mut chamber_credit: f64 = chambers.iter().map(|c| c.credit).sum();
    let mut layout = 1.0;
    if let Some(want) = p.top_heavy {
        if chambers.len() >= 2 && bot > top {
            let mid = top as f64 + (bot - top) as f64 / 2.0;
            let all: usize = chambers.iter().map(|c| c.area).sum();
            let upper: usize = chambers.iter().filter(|c| c.depth <= mid).map(|c| c.area).sum();
            layout = 0.75 + 0.25 * ramp(upper as f64 / all as f64, 0.0, want);
        }
    }
    chamber_credit *= layout;
    // Rooms under way are paid up to a fixed allowance, or half what the
    // finished chambers earn: a comb of shelves off a shaft (every slot a
    // dead end) otherwise outscored the harvester, 1,731 against 761.
    let niche_credit = match p.niche {
        Some(n) => niches
            .iter()
            .map(|c| c.credit)
            .sum::<f64>()
            .min(n.free.max(n.share * chamber_credit)),
        None => 0.0,
    };
    let tunnel_credit = tgrade.min(p.tunnel_free.max(chamber_credit + niche_credit));
    let (pit_n, stray_n) = (pit.iter().filter(|b| **b).count(), stray.iter().filter(|b| **b).count());
    let bad = 0.5 * cracks as f64 + pit_n as f64 + 0.5 * stray_n as f64;
    let s = NestScore {
        score: chamber_credit + niche_credit + 0.5 * tunnel_credit - bad,
        dug: nest.iter().filter(|b| **b).count(),
        chambers,
        chamber_credit,
        layout,
        niches,
        niche_credit,
        tunnel_cells,
        tunnel_credit,
        cracks,
        pit: pit_n,
        stray: stray_n,
    };
    (s, class)
}

/// A room's shape from its cells: per column its height (cells) and floor
/// (deepest row); height and floor read over the full-height columns only,
/// leaving out the tapered tips.
fn room_of(cells: &[(usize, usize)]) -> Chamber {
    let mut cols: std::collections::BTreeMap<usize, (usize, usize)> = std::collections::BTreeMap::new();
    for &(x, y) in cells {
        let e = cols.entry(x).or_insert((0, 0));
        e.0 += 1;
        e.1 = e.1.max(y);
    }
    let hmax = cols.values().map(|c| c.0).max().unwrap_or(0) as f64;
    let full: Vec<(usize, usize)> = cols.values().copied().filter(|c| c.0 as f64 >= 0.5 * hmax).collect();
    let height = median(&mut full.iter().map(|c| c.0 as f64).collect::<Vec<_>>());
    let fl = median(&mut full.iter().map(|c| c.1 as f64).collect::<Vec<_>>());
    let flat = full.iter().filter(|c| (c.1 as f64 - fl).abs() <= 1.0).count() as f64 / full.len() as f64;
    let area = cells.len();
    let depth = cells.iter().map(|c| c.1 as f64).sum::<f64>() / area as f64;
    Chamber {
        area,
        height,
        width: cols.len() as f64,
        flat,
        depth,
        ..Chamber::default()
    }
}

/// **How many ways into a room**: the nest cells outside it that touch it
/// (4-adjacent), grouped 8-connected. A dead end has one, a bulge on a
/// passage two, and a stretch of corridor in a maze one per crossing tunnel.
fn openings(cells: &[(usize, usize)], nest: &[bool], w: usize, h: usize) -> usize {
    let mut room = vec![false; w * h];
    for &(x, y) in cells {
        room[y * w + x] = true;
    }
    let mut contact = vec![false; w * h];
    for &(x, y) in cells {
        let mut touch = |nx: usize, ny: usize| {
            let j = ny * w + nx;
            if nest[j] && !room[j] {
                contact[j] = true;
            }
        };
        if x > 0 {
            touch(x - 1, y);
        }
        if x + 1 < w {
            touch(x + 1, y);
        }
        if y > 0 {
            touch(x, y - 1);
        }
        if y + 1 < h {
            touch(x, y + 1);
        }
    }
    let mut seen = vec![false; w * h];
    let mut groups = 0;
    for i in 0..w * h {
        if !contact[i] || seen[i] {
            continue;
        }
        groups += 1;
        seen[i] = true;
        let mut stack = vec![(i % w, i / w)];
        while let Some((x, y)) = stack.pop() {
            for dy in -1isize..=1 {
                for dx in -1isize..=1 {
                    let (nx, ny) = (x as isize + dx, y as isize + dy);
                    if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if contact[j] && !seen[j] {
                        seen[j] = true;
                        stack.push((nx as usize, ny as usize));
                    }
                }
            }
        }
    }
    groups
}

/// **The score of a lab world's nest**, read off the grid: open is anything
/// that is not ground, where ground is a powder or solid that is not food and
/// belongs to no organism -- `nestcensus`'s and the deep trace's definition,
/// so an ant, a brood cell, water and a heap of food are open -- and the
/// ground row is the world's frozen datum ([`surface_of`]). `door_x` is the
/// door's world column. Returns the score and each cell's class over the
/// world's extents (row-major, `extents`' order).
///
/// **Every cell more than [`MOUND_REACH`] rows above its own column's ground
/// reads as air**, whatever is there. The score's sky is an unbroken open
/// column from the grid's top row down, and a lab box has a lid: read from
/// the lid, no column was ever open to the sky, so a crater read as nest and
/// its pit as 0 (the teacher lane's in-run colony at 200k frames, seed 6: pit
/// 0, dug 55, score -61.9 read from the lid; pit 59, dug 25, score -81.8 read
/// from its map, which starts in the air). `MOUND_REACH` is the census's own
/// line between the bed's mound and the box's lid or a lamp, and it is drawn
/// **per column**, as the census draws it: the first fix (d8770ccb) started
/// the whole grid `MOUND_REACH` above the *highest* ground, and a box's wall
/// columns freeze their datum just under the lid (`freeze_room_datum`), so in
/// the in-run bed the grid still started at the lid and every 25k check of a
/// 64-seed cloud batch printed pit 0 (2026-10-22; z30_3 read pit 0, dug 17
/// in the run and pit 46, dug 5 from its map).
pub fn of_world(world: &World, spec: &LabBox, door_x: i32, p: &Profile) -> (NestScore, Vec<Class>) {
    let g = Grid::of(world, spec);
    let door = (door_x - g.x0).clamp(0, g.w as i32 - 1) as usize;
    let (s, class) = score(&g.open, g.w, g.h, &g.ground, door, p);
    (s, class)
}

/// **Whether a lab world's nest still reaches the open air** -- the colony
/// score's door rung (`A ladder of dig metrics for the colony score`,
/// 2026-10-21). The nest is every open cell at least `rows` rows below the
/// frozen datum within `reach` columns of `door_x`, and the door is open when
/// an unbroken 4-connected run of open cells joins the nest's **largest
/// piece** to the air (the grid's top row). The largest piece, because a plug
/// part way down the shaft leaves the stub above it reaching the air while
/// the room below is shut (this function's own test, as first written).
/// `None` when there is no nest yet, so a check before the first cut is not
/// counted either way.
///
/// **Why it is needed**: as trained, a crater is always open, so nothing
/// asked whether the door was. With rim cuts banned (a ceiling test on the
/// teacher lane's colony, 30 ants, seed 1) the colony dug a room about 22 by
/// 4 cells and then sealed it, letting go of 2,940 pellets inside; a lone ant
/// was cut off from the air on up to 41% of checks. Open is [`of_world`]'s
/// open, so an ant standing in the shaft is not a plug.
pub fn door_open(world: &World, spec: &LabBox, door_x: i32, rows: i32, reach: i32) -> Option<bool> {
    let g = Grid::of(world, spec);
    let (w, h) = (g.w, g.h);
    let nest: Vec<usize> = (0..w)
        .filter(|&x| (x as i32 + g.x0 - door_x).abs() <= reach)
        .flat_map(|x| {
            let from = (g.ground[x] + rows).max(0) as usize;
            (from..h).map(move |y| y * w + x)
        })
        .filter(|&i| g.open[i])
        .collect();
    if nest.is_empty() {
        return None;
    }
    let mut seen = vec![false; w * h];
    let mut queue: VecDeque<usize> = (0..w).filter(|&x| g.open[x]).collect();
    for &i in &queue {
        seen[i] = true;
    }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % w, i / w);
        let next = [
            (x > 0).then(|| i - 1),
            (x + 1 < w).then(|| i + 1),
            (y > 0).then(|| i - w),
            (y + 1 < h).then(|| i + w),
        ];
        for j in next.into_iter().flatten() {
            if g.open[j] && !seen[j] {
                seen[j] = true;
                queue.push_back(j);
            }
        }
    }
    // The nest's pieces, by the same 4-connected rule, kept to nest cells.
    let mut is_nest = vec![false; w * h];
    for &i in &nest {
        is_nest[i] = true;
    }
    let mut piece = vec![usize::MAX; w * h];
    let (mut best, mut best_len) = (0, 0);
    for &start in &nest {
        if piece[start] != usize::MAX {
            continue;
        }
        piece[start] = start;
        let mut stack = vec![start];
        let mut len = 0;
        while let Some(i) = stack.pop() {
            len += 1;
            let (x, y) = (i % w, i / w);
            let next = [
                (x > 0).then(|| i - 1),
                (x + 1 < w).then(|| i + 1),
                (y > 0).then(|| i - w),
                (y + 1 < h).then(|| i + w),
            ];
            for j in next.into_iter().flatten() {
                if is_nest[j] && piece[j] == usize::MAX {
                    piece[j] = start;
                    stack.push(j);
                }
            }
        }
        if len > best_len {
            (best, best_len) = (start, len);
        }
    }
    Some(nest.iter().any(|&i| piece[i] == best && seen[i]))
}

/// A lab world read as the score's grid over its extents: [`of_world`]'s
/// open cells, with the air above each column's reach, and the datum.
struct Grid {
    open: Vec<bool>,
    w: usize,
    h: usize,
    ground: Vec<i32>,
    x0: i32,
}

impl Grid {
    fn of(world: &World, spec: &LabBox) -> Grid {
        let (xs, ys) = extents(world, spec);
        let x0 = *xs.start();
        let w = (xs.end() - x0 + 1).max(0) as usize;
        let datum: Vec<i32> = xs.clone().map(|x| surface_of(world, spec, x)).collect();
        let y0 = *ys.start();
        let h = (ys.end() - y0 + 1).max(0) as usize;
        let solid: Vec<bool> = (0..world.materials.len())
            .map(|i| {
                let m = MaterialId(i as u16);
                matches!(world.materials.kind(m), MaterialKind::Powder | MaterialKind::Solid)
                    && food_value(world, Cell::new(m, 0)) <= 0.0
            })
            .collect();
        let mut open = vec![false; w * h];
        for y in y0..=*ys.end() {
            for (i, x) in xs.clone().enumerate() {
                let c = world.get(x, y);
                open[(y - y0) as usize * w + i] =
                    y < datum[i] - MOUND_REACH || c.organism_id() != 0 || !solid[c.material.0 as usize];
            }
        }
        let ground: Vec<i32> = datum.iter().map(|g| g - y0).collect();
        Grid { open, w, h, ground, x0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One case of the prototype's export: its name, numbers and grid.
    struct Case {
        name: String,
        want: Vec<f64>,
        open: Vec<bool>,
    }

    const W: usize = 256;
    const H: usize = 128;
    const GROUND: i32 = 64;
    const DOOR: usize = 128;

    fn cases() -> Vec<Case> {
        let mut out = Vec::new();
        let mut cur: Option<Case> = None;
        for line in include_str!("nestscore-controls.txt").lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some(head) = line.strip_prefix("= ") {
                let mut f = head.split('|');
                let name = f.next().expect("a name").to_string();
                let want = f.map(|v| v.parse::<f64>().expect("a number")).collect();
                let mut open = vec![false; W * H];
                open[..GROUND as usize * W].fill(true);
                cur = Some(Case { name, want, open });
            } else if line == "." {
                out.push(cur.take().expect("a case to end"));
            } else {
                let c = cur.as_mut().expect("a case open");
                let mut f = line.split(' ');
                let y: usize = f.next().expect("a row").parse().expect("a row number");
                for run in f {
                    let (a, b) = run.split_once('-').expect("a run");
                    let (a, b): (usize, usize) = (a.parse().expect("x0"), b.parse().expect("x1"));
                    c.open[y * W + a..=y * W + b].fill(true);
                }
            }
        }
        out
    }

    fn run(c: &Case, p: &Profile) -> NestScore {
        score(&c.open, W, H, &[GROUND; W], DOOR, p).0
    }

    /// **The port agrees with the prototypes on all 28 cases**: the 15
    /// controls (blueprints, the ladder's plan, the built-bad shapes), the 8
    /// steps of the climb, a niche mid-shaft and 4 combs. As built
    /// ([`PROTOTYPE`], `nestscore.py`) every part to the prototype's own
    /// rounding (one decimal for the credits, two for the graded chamber
    /// count) and exactly for every count; with niches ([`HARVESTER`], the
    /// niche prototype `climb2.py`) the score. A tight assert on a
    /// deterministic function, so it needs no planted fault to prove it can
    /// fail; it was watched failing anyway with the core at (2, 2) (the first
    /// blueprint drawing's 5x5 core).
    #[test]
    fn the_port_matches_the_prototype_on_every_control() {
        let cases = cases();
        assert_eq!(cases.len(), 28, "15 controls, 8 climb steps, a niche and 4 combs");
        let close = |got: f64, want: f64, places: i32| (got - want).abs() <= 0.5 * 10f64.powi(-places) + 1e-9;
        for c in &cases {
            let s = run(c, &PROTOTYPE);
            let w = &c.want;
            assert!(
                close(s.score, w[0], 1),
                "{}: score {} against {}",
                c.name,
                s.score,
                w[0]
            );
            assert_eq!(
                (s.dug, s.chambers.len()),
                (w[1] as usize, w[2] as usize),
                "{}: dug, chambers",
                c.name
            );
            assert!(
                close(s.good_chambers(), w[3], 2),
                "{}: graded {} against {}",
                c.name,
                s.good_chambers(),
                w[3]
            );
            assert!(
                close(s.chamber_credit, w[4], 1),
                "{}: chamber credit {} against {}",
                c.name,
                s.chamber_credit,
                w[4]
            );
            assert!(
                close(s.layout, w[5], 2),
                "{}: layout {} against {}",
                c.name,
                s.layout,
                w[5]
            );
            assert_eq!(s.tunnel_cells, w[6] as usize, "{}: tunnel cells", c.name);
            assert!(
                close(s.tunnel_credit, w[7], 1),
                "{}: tunnel credit {} against {}",
                c.name,
                s.tunnel_credit,
                w[7]
            );
            assert_eq!(
                (s.cracks, s.pit, s.stray),
                (w[8] as usize, w[9] as usize, w[10] as usize),
                "{}: cracks, pit, stray",
                c.name
            );
            let n = run(c, &HARVESTER);
            // 0.1, not 0.05: the niche prototype adds to the as-built
            // chamber credit after `nestscore.py` has rounded it.
            assert!(
                (n.score - w[11]).abs() <= 0.1 + 1e-9,
                "{}: with niches {} against {}",
                c.name,
                n.score,
                w[11]
            );
        }
    }

    /// **The ranking the controls exist for**, with niches and without: every
    /// blueprint drawn at 7-cell chambers above every built-bad shape but the
    /// one huge flat hall (a well-proportioned room the score rightly pays,
    /// 428), and the crater and the cracks below zero.
    #[test]
    fn blueprints_outrank_the_bad_shapes() {
        let cases = cases();
        for p in [&PROTOTYPE, &HARVESTER] {
            let of = |prefix: &str| -> Vec<(String, f64)> {
                cases
                    .iter()
                    .filter(|c| c.name.starts_with(prefix))
                    .map(|c| (c.name.clone(), run(c, p).score))
                    .collect()
            };
            let good: Vec<_> = of("blueprint")
                .into_iter()
                .filter(|(n, _)| n.contains("(7-cell"))
                .collect();
            let bad: Vec<_> = of("bad:")
                .into_iter()
                .filter(|(n, _)| !n.contains("flat hall"))
                .collect();
            assert_eq!((good.len(), bad.len()), (5, 7));
            let worst_good = good.iter().map(|g| g.1).fold(f64::MAX, f64::min);
            let best_bad = bad.iter().map(|b| b.1).fold(f64::MIN, f64::max);
            assert!(
                worst_good > 4.0 * best_bad,
                "worst blueprint {worst_good} against best bad shape {best_bad}"
            );
            for (n, s) in &bad {
                if n.contains("crater") || n.contains("cracks") {
                    assert!(*s < 0.0, "{n}: {s}");
                }
            }
        }
    }

    /// **The climb as the prototype built it** (why the port first printed
    /// and did not pay): a chamber dug floor first earns nothing over the
    /// bare shaft until its fifth row, and its second row goes below zero.
    #[test]
    fn as_built_a_chamber_pays_nothing_until_its_fifth_row() {
        let cases = cases();
        let climb: Vec<f64> = cases
            .iter()
            .filter(|c| c.name.starts_with("climb:"))
            .map(|c| run(c, &PROTOTYPE).score)
            .collect();
        assert_eq!(climb.len(), 8);
        let shaft = climb[0];
        assert!(
            climb[1..5].iter().all(|s| *s <= shaft),
            "rows 1-4 earn nothing over the shaft: {climb:?}"
        );
        assert!(climb[2] < 0.0, "two rows read as cracks and go below zero: {climb:?}");
        assert!(climb[5] > 4.0 * shaft, "row 5 is the first that pays: {climb:?}");
    }

    /// **The climb fixed**: with niches every row of the chamber pays more
    /// than the last, from the first -- the bar set before building it
    /// ("Dig and nest scoring", 2026-10-21) -- while the corridor maze stays
    /// under 50, the 7-cell blueprints keep their order, and no comb of
    /// shelves reaches the worst of them. The prototype is the positive
    /// control: the same assertions fail on it at row 1.
    #[test]
    fn with_niches_every_row_of_a_chamber_pays() {
        let cases = cases();
        let climb = |p: &Profile| -> Vec<f64> {
            cases
                .iter()
                .filter(|c| c.name.starts_with("climb:"))
                .map(|c| run(c, p).score)
                .collect()
        };
        let rises = |v: &[f64]| v.windows(2).all(|p| p[1] > p[0]);
        assert!(rises(&climb(&HARVESTER)), "every row pays: {:?}", climb(&HARVESTER));
        assert!(
            !rises(&climb(&PROTOTYPE)),
            "the prototype has the valley: {:?}",
            climb(&PROTOTYPE)
        );
        let named = |part: &str| run(cases.iter().find(|c| c.name.contains(part)).expect(part), &HARVESTER).score;
        assert!(named("corridor maze") < 50.0, "the maze: {}", named("corridor maze"));
        let order = [
            "garden_ant_farm (7",
            "harvester_stack (7",
            "carpenter_shallow (7",
            "staircase_kebab (7",
            "round_room_budding (7",
        ];
        let blue: Vec<f64> = order.iter().map(|n| named(n)).collect();
        assert!(
            blue.windows(2).all(|p| p[0] > p[1]),
            "the blueprints keep their order: {blue:?}"
        );
        for c in cases.iter().filter(|c| c.name.starts_with("comb:")) {
            let s = run(c, &HARVESTER).score;
            assert!(s < blue[4], "{}: {s} against the worst blueprint {}", c.name, blue[4]);
        }
        let mid = run(
            cases
                .iter()
                .find(|c| c.name.starts_with("niche:"))
                .expect("the mid-shaft niche"),
            &HARVESTER,
        );
        assert_eq!(
            (mid.niches.len(), mid.niches[0].openings),
            (1, 2),
            "a bulge on the shaft has two openings: {mid:?}"
        );
    }

    /// **The world reader sees what the grid reader sees**: the climb's last
    /// step (a 16-row shaft with a 28 x 7 side chamber, 238 in the
    /// prototype) carved into a real lab bed reads the same, and the bed
    /// before the carving reads nothing -- no door, no nest, no score. Then a
    /// 2-tall slot cut off the chamber's floor: the prototype charges it as
    /// crack, the niche pays it a little -- so the reader is blind to
    /// neither.
    #[test]
    fn a_nest_carved_in_a_lab_bed_scores_as_on_the_grid() {
        let spec = LabBox {
            founders: 0,
            colonies: 0,
            width: 128,
            ..LabBox::default()
        };
        let mut world = spec.build();
        let door = 60;
        let (bare, _) = of_world(&world, &spec, door, &HARVESTER);
        assert_eq!(
            (bare.dug, bare.chambers.len(), bare.score),
            (0, 0, 0.0),
            "an uncut bed: {bare:?}"
        );
        // Rows below the ground line, inclusive.
        let carve = |world: &mut World, x0: i32, x1: i32, y0: i32, y1: i32| {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    world.set(x, spec.ground_y + y, Cell::EMPTY);
                }
            }
        };
        carve(&mut world, door - 2, door + 1, 0, 15);
        carve(&mut world, door + 2, door + 29, 9, 15);
        let (s, class) = of_world(&world, &spec, door, &HARVESTER);
        assert_eq!(
            (s.dug, s.chambers.len(), s.cracks, s.pit, s.stray),
            (16 * 4 + 28 * 7, 1, 6, 0, 0),
            "{s:?}"
        );
        assert!(
            (s.score - 238.0).abs() < 0.05,
            "the climb's last step reads {} in a bed, 238 on the grid",
            s.score
        );
        assert!(class.contains(&Class::Chamber) && class.contains(&Class::Tunnel));
        carve(&mut world, door + 30, door + 45, 14, 15);
        let (old, _) = of_world(&world, &spec, door, &PROTOTYPE);
        assert!(
            old.cracks >= s.cracks + 16 && old.score < s.score,
            "as built a 2-tall slot is crack: {} -> {} cracks, {} -> {}",
            s.cracks,
            old.cracks,
            s.score,
            old.score
        );
        let (new, class) = of_world(&world, &spec, door, &HARVESTER);
        assert!(
            new.niches.len() == 1 && class.contains(&Class::Niche) && new.score > s.score,
            "with niches it is a niche: {new:?}"
        );
        // A crater open to the sky away from the door is pit, under the box's
        // lid as on the grid: read from the lid down it was nest (pit 0).
        carve(&mut world, door - 30, door - 15, 0, 3);
        let (crater, class) = of_world(&world, &spec, door, &HARVESTER);
        assert_eq!(crater.pit, 16 * 4, "{crater:?}");
        let (xs, ys) = extents(&world, &spec);
        assert_eq!(
            class.len(),
            ((xs.end() - xs.start() + 1) * (ys.end() - ys.start() + 1)) as usize,
            "a class a cell"
        );
        assert!(
            crater.score < new.score - 60.0,
            "a crater costs: {} -> {}",
            new.score,
            crater.score
        );
    }

    /// A wall column whose datum froze just under the lid must not drag the
    /// whole grid's sky up to the lid: the crater still reads as pit.
    #[test]
    fn a_wall_does_not_hide_a_crater() {
        let spec = LabBox {
            founders: 0,
            colonies: 0,
            width: 128,
            ..LabBox::default()
        };
        let mut world = spec.build();
        let (xs, ys) = extents(&world, &spec);
        let wall = *xs.start() + 2;
        let soil = world.get(60, spec.ground_y + 30);
        for y in *ys.start()..spec.ground_y {
            world.set(wall, y, soil);
        }
        world.freeze_room_datum();
        assert!(
            surface_of(&world, &spec, wall) < surface_of(&world, &spec, 60) - MOUND_REACH,
            "the control: the wall's datum sits far above the bed's ({} vs {})",
            surface_of(&world, &spec, wall),
            surface_of(&world, &spec, 60)
        );
        let g = surface_of(&world, &spec, 60);
        for y in g..g + 4 {
            for x in 30..46 {
                world.set(x, y, Cell::EMPTY);
            }
        }
        let (crater, class) = of_world(&world, &spec, 60, &HARVESTER);
        assert_eq!(crater.pit, 16 * 4, "{crater:?}");
        assert_eq!(
            class.len(),
            ((xs.end() - xs.start() + 1) * (ys.end() - ys.start() + 1)) as usize
        );
    }

    /// The door rung: no nest reads None, a dug nest reaches the air, a plug
    /// two rows down the shaft cuts it off, and digging the plug out opens it.
    #[test]
    fn a_plugged_shaft_cuts_the_nest_off_from_the_air() {
        let spec = LabBox {
            founders: 0,
            colonies: 0,
            width: 128,
            ..LabBox::default()
        };
        let mut world = spec.build();
        let door = 60;
        let soil = world.get(door, spec.ground_y + 30);
        assert_eq!(door_open(&world, &spec, door, 3, 20), None, "an uncut bed has no nest");
        let fill = |world: &mut World, x0: i32, x1: i32, y0: i32, y1: i32, c: Cell| {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    world.set(x, spec.ground_y + y, c);
                }
            }
        };
        fill(&mut world, door - 8, door + 8, 0, 2, Cell::EMPTY);
        assert_eq!(
            door_open(&world, &spec, door, 3, 20),
            None,
            "a crater three rows deep is not a nest"
        );
        fill(&mut world, door - 1, door, 3, 12, Cell::EMPTY);
        fill(&mut world, door + 1, door + 20, 9, 12, Cell::EMPTY);
        assert_eq!(door_open(&world, &spec, door, 3, 20), Some(true), "a shaft to a room");
        fill(&mut world, door - 1, door, 4, 5, soil);
        assert_eq!(
            door_open(&world, &spec, door, 3, 20),
            Some(false),
            "plugged two rows down"
        );
        fill(&mut world, door - 1, door, 5, 5, Cell::EMPTY);
        assert_eq!(
            door_open(&world, &spec, door, 3, 20),
            Some(false),
            "a one-row plug still shuts it"
        );
        fill(&mut world, door - 1, door - 1, 4, 4, Cell::EMPTY);
        assert_eq!(
            door_open(&world, &spec, door, 3, 20),
            Some(true),
            "one cell cut through reopens it"
        );
        // The room alone, beyond `reach` of a moved door, is no nest of that door's.
        assert_eq!(door_open(&world, &spec, door + 60, 3, 20), None);
    }

    /// The general profile drops only the layout term.
    #[test]
    fn the_general_profile_has_no_layout_term() {
        let cases = cases();
        for c in cases.iter().filter(|c| c.name.starts_with("blueprint")) {
            let (h, g) = (run(c, &HARVESTER), run(c, &GENERAL));
            assert_eq!(g.layout, 1.0, "{}", c.name);
            assert!(g.chamber_credit >= h.chamber_credit - 1e-9, "{}", c.name);
        }
    }
}
