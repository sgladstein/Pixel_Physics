//! Liquid heightfield bodies (`Reports/liquid-heightfield-design.md`).
//!
//! Step 1 of that design's build order (§11): the ownership substrate and
//! the promote/demote round trip, with no solver and no absorption yet. A
//! promoted body is a [`LiquidBody`] — it holds per-column heights (`h`) on
//! the same `material::LIQUID_FULL` scale a `Liquid` cell's own `aux`
//! already uses, and in this step never changes on its own: promotion marks
//! the existing cells `FLAG_MANAGED` without moving any mass, and a body
//! only ever leaves the world via demotion (also mass-free — see
//! `World::demote_body`).
//!
//! The grid never lies (§2a): a promoted body's cells stay exactly correct
//! in `World`'s ordinary storage at all times. What promotion changes is
//! *who is allowed to write them* — `update::update_cell` skips a managed
//! cell outright, and any write into one from outside the body's own
//! rasterizer (`World::set_owned`) demotes it (`World::set`'s own doc).

use std::collections::{BTreeMap, HashSet};

use super::cell::Cell;
use super::material::{self, MaterialId, MaterialKind};
use super::world::World;

/// Below this many columns, ordinary CA diffusion levels a puddle fast
/// enough that promotion is pure overhead — design doc §3b.4. The design's
/// own empirical anchor (`a_wide_deep_water_column_levels_out_instead_of_
/// only_eroding_at_the_edges`'s test comment) puts the real line somewhere
/// between 40 (still levels) and 100 (does not) columns wide at reach 1;
/// starting here rather than at either end. **Untuned, flagged as such.**
pub const MIN_BODY_COLUMNS: usize = 32;

/// The flood fill's cap — design doc §3b.5. A refusal by cap is "not this
/// frame," not an error; a body this large is vanishingly unlikely in
/// ordinary play and the cap exists only to bound worst-case promotion cost.
pub const MAX_BODY_CELLS: usize = 20_000;

/// Persistent per-interface flux gain — design doc §7a/§7c: the piece
/// without which a per-step relaxation is merely diffusion at a coarser
/// granularity. `0.4` is the design doc's own recommended starting point
/// (well inside the `c <= 0.5` diffusive-stability bound, the conservative
/// reading). **Untuned against real play, flagged as such** — the design
/// doc's own §7f says this should eventually become the `flow_rate` a
/// material already carries in its `.ron` (rescaled), not a second,
/// parallel viscosity knob; deferred here since retuning a value the CA
/// path also reads risks a regression in already-tuned behaviour that
/// can't be verified without watching it played, not something to do
/// blind in an unattended pass.
const SOLVER_GAIN: f64 = 0.4;

/// Per-frame flux retention — design doc §7c. `0.9` is the design doc's own
/// recommended starting point. **Untuned**, same caveat as `SOLVER_GAIN`.
const SOLVER_DAMP: f64 = 0.9;

/// A body counts as quiescent — and snaps to its exact analytic equilibrium
/// and sleeps — once every interface's level difference *and* flux both sit
/// under this, in `LIQUID_FULL`-scale units (design doc §7d). `30` (3% of
/// `LIQUID_FULL`) is not a guess: `SOLVER_GAIN`/`SOLVER_DAMP`'s own integer
/// rounding (`f64::round` each frame) leaves a genuine residual limit
/// cycle — measured directly (a 40-column body with a single-column
/// spike settles to a *persistent* oscillation around 11-21 units, never
/// fully decaying to zero, since a small level difference can round back
/// up to a nonzero flux every frame) — so a tighter epsilon would simply
/// never fire and the body would never sleep, not "take a while" to reach
/// it. 30 clears that measured floor with real margin. **Still flagged as
/// untuned against real play** like the solver constants above — this is
/// "confirmed to actually trigger," not "confirmed to look right."
const SNAP_EPSILON: i64 = 30;

/// Frames between `try_extend` attempts — design doc §3d. Checked at both
/// edges, no flood fill, so this is cheap even short; kept at a few dozen
/// frames rather than every frame purely to avoid redundant work re-
/// checking a column that almost certainly hasn't changed since the last
/// check. **Untuned against real play, flagged as such.**
const EXTEND_INTERVAL: u64 = 30;

/// Defensive cap on how far `try_extend` walks upward scanning a
/// candidate column's own extent — not expected to ever bind (the scan
/// already stops at the first non-matching cell), but a bounded loop is
/// cheap insurance against an unforeseen interaction leaving a candidate
/// column's material check never failing.
const MAX_EXTEND_SCAN: i32 = 10_000;

/// How far above the body's own average column height an edge column's
/// fill must rise, relative to that average, before it counts as
/// overflowing rather than merely unevenly loaded — design doc §6c's own
/// "that column's level exceeds what the body can contain," made concrete.
/// The design doc does not name an exact trigger for this (it describes
/// the *behaviour* — an overloaded edge with somewhere to spill demotes —
/// without specifying the numeric threshold), so this is a judgment call
/// documented as such, not a value transcribed from the report.
/// **Untuned against real play.**
const EDGE_OVERFLOW_RATIO: f64 = 2.0;

/// Frames a body's `try_extend` stays suppressed after an edge demotion —
/// design doc §4c's thrash control, scoped down to the one form of thrash
/// that's actually reachable at this step: CA settling a just-demoted
/// column into a puddle that sits at exactly the row range `try_extend`
/// would re-claim, on the very next check, oscillating forever. §4c's own
/// full shape (exponential backoff, reset after a promotion survives N
/// frames) is written against a *promotion* candidate queue that doesn't
/// exist yet (no automatic promotion trigger is wired up at all — see
/// `PLAN.md`'s own note on that gap); a flat per-body cooldown after a
/// demotion is the part of it that already applies today. **Untuned.**
const DEMOTE_COOLDOWN_FRAMES: u64 = 120;

/// One promoted liquid body — design doc §2b/§3a/§7/§9a. A body is always
/// exactly one material (§3b.1) and always exactly one contiguous run of
/// x-columns — 4-connectivity guarantees no gaps (`label_body`'s own doc).
pub struct LiquidBody {
    pub material: MaterialId,
    pub x0: i32,
    /// Per column (indexed from `x0`): topmost liquid cell, inclusive.
    pub top_y: Vec<i32>,
    /// Per column: first non-body row below the column's liquid, exclusive
    /// — i.e. the column's own cells span `top_y[i]..bed_y[i]`.
    pub bed_y: Vec<i32>,
    /// Per column: total fill in units on the `material::LIQUID_FULL`
    /// scale, summed over the column's own cells. Computed *from* the
    /// cells at promotion (`World::promote_liquid_body`) and never
    /// otherwise — this is what makes promotion mass-free.
    pub h: Vec<u32>,
    /// Signed flux across interface `i | i+1`, one entry per adjacent
    /// column pair (`columns() - 1` long) — design doc §7a/§7b. Positive
    /// means flow from column `i` to `i + 1`. Persistent across frames on
    /// purpose: this *is* the mechanism that makes leveling O(width) rather
    /// than O(width²) diffusion (§7a's own correction to the research
    /// report it's built from). Resized by `try_extend`/`demote_edge_
    /// column` (design doc §3d/§6c) — always kept exactly one shorter than
    /// `columns()`, an entry inserted/removed at whichever edge changed.
    pub flux: Vec<i32>,
    /// Set once quiescent (design doc §7d/§4a — a *whole-body* measurement,
    /// never a per-cell test; see `step`'s own doc for why quiescence is
    /// explicitly not what gates *promotion*, only this). `step` returns
    /// immediately, doing no work at all, while this is `true` — a sleeping
    /// body costs nothing per frame, which is the entire point (design doc
    /// §8c). Cleared by `World::absorb_liquid` crediting more fill in (the
    /// body has something new to redistribute) — one of the wake triggers
    /// the design doc names; `try_extend` claiming a new column is another
    /// (not yet wired as a wake trigger itself, since `try_extend` only
    /// ever runs from inside `step`, which by construction only executes
    /// while already awake).
    pub asleep: bool,
    /// Frame `try_extend` is suppressed until, set by `demote_edge_column`
    /// — design doc §4c's thrash control, scoped to what's reachable
    /// today; see `DEMOTE_COOLDOWN_FRAMES`'s own doc for why this isn't
    /// §4c's full exponential-backoff shape.
    pub extend_cooldown_until: u64,
}

impl LiquidBody {
    pub fn columns(&self) -> usize {
        self.top_y.len()
    }

    /// Every cell this body owns and, once a solver exists, moves —
    /// design doc §3c. Not the same set as `container_positions` below,
    /// which the body depends on but never moves or counts as its own mass.
    pub fn managed_positions(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (0..self.columns()).flat_map(move |i| {
            let x = self.x0 + i as i32;
            (self.top_y[i]..self.bed_y[i]).map(move |y| (x, y))
        })
    }

    /// The bed and walls immediately outside the body's own cells — design
    /// doc §3c. Flagged `FLAG_MANAGED` exactly like the body's own cells
    /// (so disturbing them also demotes, through the identical one-bit
    /// test) but never counted in `h` and never moved by the body. Derived
    /// from `managed_positions` rather than stored separately: for every
    /// body cell, its left/right/below neighbour is a container cell
    /// unless that neighbour is itself a body cell (an interior column
    /// boundary). Deliberately excludes "above" — that is the free surface
    /// §3b.3 requires stay open, and is where a later step's absorption
    /// inflow lands.
    pub fn container_positions(&self) -> Vec<(i32, i32)> {
        let body: HashSet<(i32, i32)> = self.managed_positions().collect();
        let mut container = HashSet::new();
        for &(x, y) in &body {
            for (dx, dy) in [(-1, 0), (1, 0), (0, 1)] {
                let p = (x + dx, y + dy);
                if !body.contains(&p) {
                    container.insert(p);
                }
            }
        }
        container.into_iter().collect()
    }

    pub fn total_fill(&self) -> u64 {
        self.h.iter().map(|&fill| fill as u64).sum()
    }

    /// Whether `(x, y)` is a cell this body owns — its own liquid cells or
    /// its bed. A fast column-range check first (the common case, since
    /// most disturbances land in the body's own bulk), falling back to the
    /// full `container_positions` set for the remaining case: a step wall
    /// where an interior column boundary is exposed because a neighbouring
    /// column is shorter. Used by `World::find_body_at` to resolve which of
    /// a chunk's few candidate bodies a disturbed position belongs to.
    pub fn owns(&self, x: i32, y: i32) -> bool {
        let i = x - self.x0;
        if i >= 0 && (i as usize) < self.columns() {
            let i = i as usize;
            if (y >= self.top_y[i] && y < self.bed_y[i]) || y == self.bed_y[i] {
                return true;
            }
        }
        self.container_positions().contains(&(x, y))
    }

    /// Write cell `(x, y)` of this body's own material with `fill` units,
    /// preserving the existing shade if the cell is already this body's own
    /// material (so a cell that merely changes its partial fill doesn't
    /// redraw with fresh grain every frame), drawing a new one otherwise.
    ///
    /// A genuine no-op — no `set_owned` call at all — when the cell is
    /// already exactly correct. Found by independent review: without this
    /// check, the "same whole-cell count" branch of `rasterize_column`
    /// below unconditionally rewrote the topmost cell every single call,
    /// which `Chunk::set_world`/`mark_dirty` reads as a real change
    /// regardless of whether the bytes actually differ — so every live
    /// body's chunk stayed marked dirty on every solver frame even at
    /// perfect, unmoving equilibrium, defeating design doc §7e's entire
    /// point ("nothing written, so nothing dirties, so a quiet column
    /// keeps letting its chunk sleep") and, transitively, the sleep
    /// mechanism step 4 depends on.
    fn write_liquid_cell(&self, world: &mut World, x: i32, y: i32, fill: u16) {
        // `aux == 0` is `liquid_fill`'s own "treat as full" sentinel
        // (`update::liquid_fill`'s doc), not "empty" -- a genuinely full
        // cell is written as 0, matching every other liquid write.
        let aux = if fill >= material::LIQUID_FULL { 0 } else { fill };
        let existing = world.get(x, y);
        if existing.material == self.material && existing.managed() && existing.aux() == aux {
            return;
        }
        let cell = if existing.material == self.material {
            existing.with_managed(true).with_aux(aux)
        } else {
            let shades = world.materials.get(self.material).palette.len().max(1) as u32;
            let shade = world.rng.below(shades) as u8;
            Cell::new(self.material, shade).with_managed(true).with_aux(aux)
        };
        world.set_owned(x, y, cell);
    }

    /// Rasterize column `i` back into cells if its whole-cell count, or its
    /// topmost cell's own partial fill, has changed since it was last
    /// written — design doc §7e. Called after `h[i]` changes (`World::
    /// absorb_liquid`'s credit, and `step`'s own solver below). A no-op,
    /// deliberately, when nothing actually changed: nothing is written, so
    /// nothing dirties, so a quiet column keeps letting its chunk sleep.
    ///
    /// Three cases, all driven by comparing the whole-cell count `h[i]`
    /// implies against what's currently on the grid (`bed_y[i] - top_y[i]`):
    /// **grows** (claims new cells upward from `top_y[i]`, flagging their
    /// own newly-exposed left/right/below neighbours as container cells,
    /// mirroring what `World::promote_liquid_body` did for the body's
    /// initial footprint, just incrementally); **shrinks** (§11 step 3's
    /// solver is what makes this reachable — nothing before it ever removed
    /// mass — clearing the vacated rows back to `Cell::EMPTY` and unflagging
    /// any container cell no longer adjacent to *any* remaining body cell);
    /// or **stays the same count** but the topmost cell's own partial fill
    /// changed (a small solver step that doesn't cross a whole-cell
    /// boundary) — the one case Step 2's original version of this function
    /// missed entirely, since absorption's whole-cell-at-a-time credits
    /// never exercised it, but the solver's small, continuous transfers do.
    pub(crate) fn rasterize_column(&mut self, world: &mut World, i: usize) {
        let whole = self.h[i] / material::LIQUID_FULL as u32;
        let partial = (self.h[i] % material::LIQUID_FULL as u32) as u16;
        let new_count = whole as i32 + i32::from(partial > 0);
        let old_count = self.bed_y[i] - self.top_y[i];
        let x = self.x0 + i as i32;

        if new_count == old_count {
            // Same cell count, but the topmost cell's own partial fill may
            // still have moved -- always safe, and only ever one cell, to
            // just re-check it.
            if new_count > 0 {
                self.write_liquid_cell(world, x, self.top_y[i], if partial > 0 { partial } else { material::LIQUID_FULL });
            }
            return;
        }

        // Snapshotted before changing the footprint so the container-
        // flagging pass below can tell exactly which positions are newly
        // exposed or newly abandoned, rather than touching ones that were
        // already correct.
        let old_container: HashSet<(i32, i32)> = self.container_positions().into_iter().collect();
        let old_top = self.top_y[i];

        if new_count > old_count {
            let grow = new_count - old_count;
            let new_top = old_top - grow;
            self.top_y[i] = new_top;
            // Upper bound is `old_top` (inclusive -- that cell already
            // held real fill and may need its own aux refreshed) only
            // when there *was* a previous top, i.e. `old_count > 0`. A
            // column growing from genuinely empty has `old_top == bed_y`
            // (§7e's own "no cells" state), which is the container/bed
            // position, not a body cell -- writing through it would
            // manufacture a cell (and its mass) out of nothing. Found via
            // a mass-conservation test: a column that drained to zero one
            // frame and received flux back the next wrote real water into
            // its own bed cell, inflating total world fill by one full
            // cell's worth.
            let write_upper = if old_count > 0 { old_top } else { old_top - 1 };
            for y in new_top..=write_upper {
                let fill = if y == new_top && partial > 0 { partial } else { material::LIQUID_FULL };
                self.write_liquid_cell(world, x, y, fill);
            }
        } else {
            let shrink = old_count - new_count;
            let new_top = old_top + shrink;
            for y in old_top..new_top {
                world.set_owned(x, y, Cell::EMPTY);
            }
            self.top_y[i] = new_top;
            if new_count > 0 {
                self.write_liquid_cell(world, x, new_top, if partial > 0 { partial } else { material::LIQUID_FULL });
            }
        }

        // Container cells: flag newly-exposed ones, clear ones no longer
        // adjacent to any remaining body cell of *this* body. Known,
        // accepted gap: if a position happens to also be a container cell
        // of a *different* live body (two bodies with immediately
        // touching footprints), clearing it here would incorrectly
        // un-protect that other body's own wall too -- rare enough
        // (different bodies are already different materials, so this only
        // arises from two disjoint same-material components separated by
        // exactly one cell) that it's flagged rather than solved in this
        // pass.
        let new_container: HashSet<(i32, i32)> = self.container_positions().into_iter().collect();
        for &(cx, cy) in new_container.difference(&old_container) {
            let cell = world.get(cx, cy);
            world.set_owned(cx, cy, cell.with_managed(true));
        }
        for &(cx, cy) in old_container.difference(&new_container) {
            let cell = world.get(cx, cy);
            if cell.managed() {
                world.set_owned(cx, cy, cell.with_managed(false));
            }
        }
    }

    /// Advance this body by one frame — the persistent-flux pipe solver,
    /// design doc §7a-§7c, plus the quiescence check and terminal snap
    /// (§7d) that put it to sleep once genuinely converged. Returns
    /// immediately, doing no work at all, once `asleep` — design doc §8c:
    /// a sleeping body costs nothing per frame, and `World::absorb_liquid`
    /// is what wakes it again. No-op for a single-column body too (nothing
    /// to level against) — not reachable in practice, since `label_body`'s
    /// own `MIN_BODY_COLUMNS` validation guarantees every promoted body
    /// starts with at least 32.
    pub(crate) fn step(&mut self, world: &mut World) {
        // try_extend (§3d), checked *before* the `asleep` early return,
        // deliberately: a settled, sleeping lake sitting next to a
        // leftover puddle is exactly the ordinary case this exists to
        // resolve, and a successful claim is one of the wake triggers the
        // design doc itself names (`asleep`'s own doc). Checked
        // periodically, both edges -- re-absorbs a pool that spilled
        // sideways (via edge demotion, or simply painted there) without a
        // full re-promotion. Suppressed during the post-demotion cooldown
        // (§4c) -- otherwise a column just demoted, still sitting right
        // there as an ordinary CA puddle, gets immediately re-claimed on
        // the very next check, oscillating between the two states
        // forever. Found the hard way: with this check placed *after* the
        // `asleep` return instead, a body that reached equilibrium and
        // slept immediately after a demotion could never run `try_extend`
        // again -- the demoted puddle sat there forever, unclaimed, no
        // matter how long the cooldown had since expired.
        if self.columns() >= 2 && world.frame >= self.extend_cooldown_until && world.frame.is_multiple_of(EXTEND_INTERVAL) {
            let claimed_left = self.try_extend(world, -1);
            let claimed_right = self.try_extend(world, 1);
            if claimed_left || claimed_right {
                self.asleep = false;
                // A claimed column also gets its own cooldown -- reusing
                // the identical field/duration a demotion sets, now
                // dual-purpose ("no extend, and no demote-eligibility
                // either, until this frame"). Found the hard way: a
                // column `demote_edge_column` handed back is very often
                // tall enough that reclaiming it whole immediately re-
                // trips `overloaded_edge` against the body's now-diluted
                // average -- without this, that demotes it right back a
                // frame or two later, which resets *its own* cooldown and
                // repeats forever, just slower than the same-frame version
                // this replaced. The cooldown window instead gives the
                // persistent-flux solver real time to spread the
                // reclaimed mass out before the column is reconsidered
                // for demotion at all (`overloaded_edge`'s own call below
                // is gated on this same field).
                self.extend_cooldown_until = world.frame + DEMOTE_COOLDOWN_FRAMES;
                return;
            }
        }

        if self.asleep {
            // Equilibrium requires containment, not just flatness. A body
            // that can still spill has somewhere to go no matter how level
            // its surface is, so it does not get to sleep through it.
            if self.edge_with_room(world).is_none() {
                return;
            }
            self.asleep = false;
        }
        if self.columns() < 2 {
            return;
        }

        // Edge demotion (§6c), checked against last frame's already-
        // settled `h[]`: an edge column significantly overloaded relative
        // to the body's own average, with somewhere outside to spill to,
        // is handed back to the CA rather than allowed to grow without
        // bound. At most one demotion per frame -- still overloaded next
        // frame simply checks again. Not checked while asleep (skipped by
        // the early return above): a sleeping body's `h[]` cannot have
        // changed since it fell asleep already not-overloaded (the
        // terminal snap that put it to sleep only ever produces a flat,
        // in-equilibrium state), so there is nothing new to detect. Also
        // gated on the cooldown field a *reclaim* now sets too (see the
        // `try_extend` block above) -- a column just handed back needs
        // real frames of solving before it's fair to judge overloaded
        // again, or reclaim and demote thrash each other forever.
        if world.frame >= self.extend_cooldown_until {
            // `overloaded_edge` first (an edge growing without bound is the
            // urgent case), then plain uncontainment: a level body beside
            // open floor sheds its edge column back to the CA, which is
            // exactly how §6c says outflow is supposed to happen -- "edge
            // demotion, not a special rule". The shed column flows away as
            // ordinary water, `try_extend` reclaims whatever is left after
            // the cooldown, and the body walks outward one column at a time
            // until it finds walls and genuinely is contained.
            if let Some(edge_i) = self.overloaded_edge(world).or_else(|| self.edge_with_room(world)) {
                self.demote_edge_column(world, edge_i);
                if self.columns() < 2 {
                    return; // demoted down to nothing left to level
                }
            }
        }

        let n = self.columns();
        // Any fixed reference works -- only differences ever get used
        // (design doc §7b) -- so this is recomputed locally each call
        // rather than stored. The deepest bed keeps every `level` value
        // non-negative, which isn't load-bearing but is easier to read
        // while debugging than an arbitrary offset would be.
        let ref_bed = self.bed_y.iter().copied().max().unwrap_or(0);
        let level: Vec<i64> =
            (0..n).map(|i| (ref_bed - self.bed_y[i]) as i64 * material::LIQUID_FULL as i64 + self.h[i] as i64).collect();

        // 1. Flux update -- the persistent term is the whole point (§7a).
        for i in 0..n - 1 {
            let d = level[i] - level[i + 1];
            let damped = (self.flux[i] as f64 * SOLVER_DAMP).round() as i64;
            let gained = (SOLVER_GAIN * d as f64).round() as i64;
            self.flux[i] = damped.saturating_add(gained).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        }

        // 2. K clamp: a column cannot pay out more than it holds (§7b step
        // 2). Single forward pass over columns, exactly as pseudocoded --
        // a later column's own clamp can further shrink a flux value an
        // earlier column's clamp already touched, which is conservative
        // (never lets `h` go negative) even though it isn't perfectly
        // optimal, the same trade the design doc's own pseudocode accepts.
        for i in 0..n {
            let right_out = if i + 1 < n { self.flux[i].max(0) as i64 } else { 0 };
            let left_out = if i > 0 { (-self.flux[i - 1]).max(0) as i64 } else { 0 };
            let out = right_out + left_out;
            if out > self.h[i] as i64 && out > 0 {
                let cap = self.h[i] as i64;
                if i + 1 < n && self.flux[i] > 0 {
                    self.flux[i] = ((self.flux[i] as i64 * cap) / out) as i32;
                }
                if i > 0 && self.flux[i - 1] < 0 {
                    self.flux[i - 1] = -(((-self.flux[i - 1]) as i64 * cap) / out) as i32;
                }
            }
        }

        // Quiescence (design doc §4a/§7d): a whole-body measurement over
        // exactly the values this frame's redistribution is about to
        // apply, computed here (in O(width), from arrays already in hand)
        // rather than as any per-cell test. This is *not* a promotion
        // gate — §4a is explicit that a still-moving body is the whole
        // point of promoting it in the first place — it only gates this
        // body's own sleep, checked fresh every frame. A body that has
        // already converged snaps to its exact equilibrium this same
        // frame rather than needing one more, redundant, tiny nudge first.
        let quiescent = (0..n - 1).all(|i| (level[i] - level[i + 1]).abs() < SNAP_EPSILON && (self.flux[i] as i64).abs() < SNAP_EPSILON);

        // 3. Apply -- exactly conservative by construction (§7b step 3):
        // every interface debits one column and credits its neighbour by
        // the same integer, so `Σ h` is invariant regardless of what steps
        // 1-2 computed. `.min` against the paying column's own `h` is a
        // second, redundant safety net on top of the K clamp above --
        // cheap, and turns "the clamp had a bug" into "leveling is
        // slightly off" instead of an integer underflow panic.
        for i in 0..n - 1 {
            let f = self.flux[i];
            if f > 0 {
                let amount = (f as u32).min(self.h[i]);
                self.h[i] -= amount;
                self.h[i + 1] += amount;
            } else if f < 0 {
                // `unsigned_abs`, not `-f as u32` -- `-i32::MIN` overflows
                // (`i32::MIN`'s magnitude has no positive `i32`
                // representation), while `unsigned_abs` computes it
                // directly in `u32`, which does have room. Not reachable
                // today given `MAX_BODY_CELLS` bounds every realistic
                // fill/flux value far below either limit, but a correct,
                // equally cheap spelling costs nothing to just use.
                let amount = f.unsigned_abs().min(self.h[i + 1]);
                self.h[i] += amount;
                self.h[i + 1] -= amount;
            }
        }

        if quiescent {
            self.terminal_snap(world, ref_bed);
        } else {
            for i in 0..n {
                self.rasterize_column(world, i);
            }
        }
    }

    /// Solve the body's exact analytic equilibrium and snap to it, zero
    /// the flux array, and mark the body asleep — design doc §7d. Only
    /// ever called once quiescence already holds (`step`'s own check), so
    /// the change this makes is sub-cell: surface flatness becomes
    /// *exact* (§7d's own claim, ≤ 1 fill unit / 0.1% of `LIQUID_FULL`),
    /// not merely "close enough," and it is not water teleporting.
    fn terminal_snap(&mut self, world: &mut World, ref_bed: i32) {
        let n = self.columns();
        let bed_levels: Vec<i64> = (0..n).map(|i| (ref_bed - self.bed_y[i]) as i64 * material::LIQUID_FULL as i64).collect();
        let total: i64 = self.h.iter().map(|&h| h as i64).sum();

        // The equilibrium surface elevation `L` is monotone in its own
        // total contribution (design doc §7d), so binary search finds the
        // smallest integer `L` whose contribution across every column
        // reaches `total` in O(n log range).
        let contribution = |l: i64| -> i64 { bed_levels.iter().map(|&b| (l - b).max(0)).sum() };
        let mut lo = *bed_levels.iter().min().unwrap();
        let mut hi = lo + total + 1;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if contribution(mid) < total {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let l = lo;

        let mut new_h: Vec<i64> = bed_levels.iter().map(|&b| (l - b).max(0)).collect();
        let mut overshoot = new_h.iter().sum::<i64>() - total;
        debug_assert!(overshoot >= 0, "the smallest L with contribution(L) >= total should never undershoot");
        // Distribute the exact integer remainder deterministically --
        // lowest column index first (design doc §7d), never by hash or
        // iteration order (issue #7's standing rule, applied here too).
        for h in new_h.iter_mut() {
            if overshoot <= 0 {
                break;
            }
            if *h > 0 {
                *h -= 1;
                overshoot -= 1;
            }
        }
        debug_assert_eq!(overshoot, 0, "the equilibrium solve should exactly account for every unit of total fill");

        for (h, &new) in self.h.iter_mut().zip(&new_h) {
            *h = new as u32;
        }
        self.flux.iter_mut().for_each(|f| *f = 0);
        for i in 0..n {
            self.rasterize_column(world, i);
        }
        self.asleep = true;
    }

    /// Whether an edge column (0 or `columns() - 1`) counts as overflowing
    /// — design doc §6c: significantly overloaded relative to the body's
    /// own average, *and* with somewhere outside to spill into. Returns
    /// the first such edge found (left checked before right).
    ///
    /// "Somewhere to spill" is checked by raw material, not by `managed()`
    /// or `Cell::is_empty()` (which, since the fix that let ordinary
    /// movement respect `FLAG_MANAGED`, now deliberately excludes managed
    /// cells): the position immediately outside *any* edge column is
    /// always this body's own container wall, flagged managed at
    /// promotion (or by an earlier `try_extend`) precisely so a
    /// disturbance there demotes the body — which means it is *always*
    /// "managed" and would make this check permanently unsatisfiable if it
    /// required the opposite. The wall's own material is what actually
    /// matters: still genuinely empty (or gas) underneath the flag means
    /// there is real room, and `demote_edge_column`'s own container diff
    /// is what correctly un-flags that wall once it stops being adjacent
    /// to any surviving body cell.
    fn overloaded_edge(&self, world: &World) -> Option<usize> {
        let n = self.columns();
        if n < 2 {
            return None;
        }
        let avg = self.h.iter().map(|&h| h as f64).sum::<f64>() / n as f64;
        if avg <= 0.0 {
            return None;
        }
        for &edge_i in &[0usize, n - 1] {
            if (self.h[edge_i] as f64) > avg * EDGE_OVERFLOW_RATIO && self.edge_has_room(world, edge_i) {
                return Some(edge_i);
            }
        }
        None
    }

    /// Open space beside edge column `edge_i`, at the middle of its depth.
    fn edge_has_room(&self, world: &World, edge_i: usize) -> bool {
        let outside_x = if edge_i == 0 { self.x0 - 1 } else { self.x0 + self.columns() as i32 };
        let mid_y = (self.top_y[edge_i] + self.bed_y[edge_i]) / 2;
        let outside = world.get(outside_x, mid_y);
        outside.material == material::EMPTY || world.materials.kind(outside.material) == MaterialKind::Gas
    }

    /// An edge column with open space beside it, at the middle of its own
    /// depth — somewhere this body's water could actually go.
    ///
    /// Split out of `overloaded_edge` because it answers a second, more
    /// basic question that had no answer at all: **is this body contained?**
    /// A body that is internally level but has open floor beside it is not
    /// in equilibrium, however flat its surface is, and both of the gates
    /// that used to stand between it and spreading were keyed on the wrong
    /// thing:
    ///
    /// - `overloaded_edge` required an edge column above
    ///   `EDGE_OVERFLOW_RATIO` times the body's own average, which a level
    ///   body is by definition never above.
    /// - `step`'s `asleep` early return skipped the check entirely, on the
    ///   stated reasoning that a body that fell asleep flat and
    ///   not-overloaded "cannot have changed, so there is nothing new to
    ///   detect". True of its `h[]`, and beside the point: nothing about
    ///   being flat means being contained.
    ///
    /// Together those meant a promoted body simply never spread. Measured
    /// before this: a four-deep, forty-column body on a floor with a
    /// hundred and fifty columns of open room beside it fell asleep by
    /// frame 100 and had not moved one cell by frame 4000. That is the gap
    /// that got automatic promotion reverted (`127e177`) — "the persistent-
    /// flux solver has no mechanism to drive an internally-level body to
    /// expand into open floor space beside it."
    fn edge_with_room(&self, world: &World) -> Option<usize> {
        let n = self.columns();
        if n < 2 {
            return None;
        }
        // Alternated by frame, not scanned left-first. Always testing edge 0
        // first meant the left edge won every time it had room, so a body
        // with room on *both* sides was eaten entirely from the left while
        // its right edge never moved once -- measured on a 100-column block:
        // `x0` walked 60 -> 110 across 6000 frames while the right edge sat
        // at 159 throughout. Same directional-bias problem, and the same
        // answer, as `update::step`'s alternating `rightward` scan.
        let order = if world.frame.is_multiple_of(2) { [0usize, n - 1] } else { [n - 1, 0usize] };
        order.into_iter().find(|&edge_i| self.h[edge_i] > 0 && self.edge_has_room(world, edge_i))
    }

    /// Demote edge column `edge_i` (0 or `columns() - 1`) back to ordinary
    /// CA control — design doc §6c. The column's own cells simply lose
    /// `FLAG_MANAGED` (their content is already correct, matching
    /// demotion's usual "no mass moves" property, `World::demote_body`'s
    /// own doc); the body's own arrays shrink by one entry, and `x0`
    /// advances if the column removed was the left edge. Container cells
    /// that were exclusively protecting this column (not shared with any
    /// surviving column) are un-flagged too, via the same before/after
    /// diff `rasterize_column` already uses for the identical question.
    fn demote_edge_column(&mut self, world: &mut World, edge_i: usize) {
        self.extend_cooldown_until = world.frame + DEMOTE_COOLDOWN_FRAMES;
        let x = self.x0 + edge_i as i32;
        let old_container: HashSet<(i32, i32)> = self.container_positions().into_iter().collect();

        for y in self.top_y[edge_i]..self.bed_y[edge_i] {
            let cell = world.get(x, y);
            if cell.managed() {
                world.set_owned(x, y, cell.with_managed(false));
            }
        }
        self.top_y.remove(edge_i);
        self.bed_y.remove(edge_i);
        self.h.remove(edge_i);
        if edge_i == 0 {
            self.x0 += 1;
        }
        // `flux` has exactly one fewer entry than columns; the interface
        // that touched this edge is the one to drop (an edge column has
        // exactly one).
        if !self.flux.is_empty() {
            if edge_i == 0 {
                self.flux.remove(0);
            } else {
                self.flux.pop();
            }
        }

        let new_container: HashSet<(i32, i32)> = self.container_positions().into_iter().collect();
        for &(cx, cy) in old_container.difference(&new_container) {
            let cell = world.get(cx, cy);
            if cell.managed() {
                world.set_owned(cx, cy, cell.with_managed(false));
            }
        }
    }

    /// Attempt to claim one more column at the given edge — design doc
    /// §3d. `direction`: `-1` extends left (a new column at `x0 - 1`),
    /// `1` extends right (a new column at `x0 + columns()`). Returns
    /// whether a column was actually claimed.
    ///
    /// The candidate's own vertical extent is scanned independently —
    /// anchored at the neighbouring edge column's own bed row (their
    /// shared floor, since a flat-bedded body has every column resting at
    /// the same depth) and walked upward through contiguous same-
    /// material, unmanaged cells — rather than assumed to match the
    /// neighbour's own height. Found the hard way: a column just handed
    /// back by `demote_edge_column` is very often *much* taller than the
    /// neighbour it demoted from (an overloaded edge column is exactly
    /// the tall case that triggered the demotion in the first place), and
    /// checking "is there a free surface" only one row above the
    /// neighbour's own short height finds more of the same candidate
    /// column's water there instead of open air — permanently refusing
    /// the very puddle `try_extend` exists to reclaim. Still no flood
    /// fill (§3b's per-column checks applied to one column), matching the
    /// design doc's own "two columns tested" scope; `MAX_EXTEND_SCAN` is
    /// a defensive cap only, not expected to ever bind in practice (the
    /// scan already stops at the first non-matching cell, which every
    /// real column reaches well before any cap would matter).
    fn try_extend(&mut self, world: &mut World, direction: i32) -> bool {
        let n = self.columns();
        let edge_i = if direction < 0 { 0 } else { n - 1 };
        let candidate_x = if direction < 0 { self.x0 - 1 } else { self.x0 + n as i32 };
        let bed = self.bed_y[edge_i];

        let is_own_liquid = |world: &World, y: i32| {
            let cell = world.get(candidate_x, y);
            cell.material == self.material && !cell.managed()
        };
        if !is_own_liquid(world, bed - 1) {
            return false; // nothing there at all
        }
        let mut top = bed - 1;
        while top > bed - MAX_EXTEND_SCAN && is_own_liquid(world, top - 1) {
            top -= 1;
        }
        let above = world.get(candidate_x, top - 1);
        if !above.is_empty() && world.materials.kind(above.material) != MaterialKind::Gas {
            return false; // no free surface -- refuse rather than claim a sealed/ceiling-capped column
        }

        let old_container: HashSet<(i32, i32)> = self.container_positions().into_iter().collect();

        let fill: u32 = (top..bed).map(|y| super::update::liquid_fill(world.get(candidate_x, y)) as u32).sum();
        if direction < 0 {
            self.x0 -= 1;
            self.top_y.insert(0, top);
            self.bed_y.insert(0, bed);
            self.h.insert(0, fill);
            self.flux.insert(0, 0);
        } else {
            self.top_y.push(top);
            self.bed_y.push(bed);
            self.h.push(fill);
            self.flux.push(0);
        }
        for y in top..bed {
            let cell = world.get(candidate_x, y);
            world.set_owned(candidate_x, y, cell.with_managed(true));
        }

        let new_container = self.container_positions();
        for (cx, cy) in new_container {
            if !old_container.contains(&(cx, cy)) {
                let cell = world.get(cx, cy);
                // Never claim live liquid of our own kind as container.
                //
                // At promotion this case cannot arise: `label_body` takes a
                // maximal connected region, so anything adjacent to it is by
                // construction *not* more of the same unmanaged liquid. An
                // incremental claim breaks that invariant -- extending onto
                // column `x0 - 1` exposes `x0 - 2` as new container, and that
                // may be an ordinary CA puddle of our own material that was
                // simply too shallow to claim this time.
                //
                // Flagging it `managed` froze it permanently: `update_cell`
                // skips managed cells, and no body's `h[]` owns it, so it was
                // simulated by nothing and belonged to nothing. It is also
                // the reachable path into `World::absorb_liquid`'s bounds
                // miss, which drops fill the caller has already debited.
                //
                // Skipping is right rather than merely safe. The container
                // flag means "a liquid body depends on you", which is a claim
                // about structure -- a bed, a wall. Our own live liquid is
                // not structure; it is a candidate for the *next*
                // `try_extend`, and leaving it unflagged is what lets that
                // happen. Found by review.
                if cell.material == self.material && !cell.managed() {
                    continue;
                }
                world.set_owned(cx, cy, cell.with_managed(true));
            }
        }
        true
    }
}

/// A validated candidate for promotion — design doc §3a's `BodyScan`.
/// Returned by `label_body` only once every one of §3b's structural checks
/// already holds; there is no separate "scan then validate" caller step.
pub struct BodyScan {
    pub material: MaterialId,
    pub x0: i32,
    pub top_y: Vec<i32>,
    pub bed_y: Vec<i32>,
    pub fill: Vec<u32>,
}

/// Bounded 4-connected flood fill over same-material `Liquid` cells
/// starting at `(x, y)`, validated against design doc §3b. `None` if
/// `(x, y)` isn't `Liquid`, any cell in the component is already
/// `FLAG_MANAGED` (already claimed by a different promoted body — checked
/// per visited cell, not just at `(x, y)`, so a caller can never end up
/// with two live `BodyId`s over overlapping cells; found by independent
/// review: promoting an already-promoted pool a second time used to
/// silently succeed, leaving a ghost `BodyId` in `body_index` that nothing
/// could ever demote, since its own cells could never again read as
/// disturbed once the *first* body's demotion had already cleared their
/// flag), the component fails any structural check, or it exceeds
/// `MAX_BODY_CELLS`.
///
/// Deliberately a new function rather than a generalization of `rigid::
/// label_component` — design doc §3a: the predicate (`Solid`-and-not-
/// `BEDROCK` there, same-`MaterialId`-`Liquid` here) and the return shape
/// (positions only there; per-column extents and fill here) both differ
/// enough that sharing would mean generics over both, which is more
/// machinery than two short functions.
pub fn label_body(world: &World, x: i32, y: i32) -> Option<BodyScan> {
    let start = world.get(x, y);
    if world.materials.kind(start.material) != MaterialKind::Liquid || start.managed() {
        return None;
    }
    let material = start.material;

    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    let mut stack = vec![(x, y)];
    visited.insert((x, y));
    // `BTreeMap` rather than `HashMap`: iterated below to build `top_y`/
    // `bed_y`/`fill` in ascending-x order, and this module has no reason to
    // reintroduce issue #7's hazard (`scheduler.rs`'s own doc) for a
    // per-body scan that has no cross-run ordering requirement of its own,
    // but also no reason not to be deterministic for free.
    let mut columns: BTreeMap<i32, Vec<i32>> = BTreeMap::new();

    while let Some((cx, cy)) = stack.pop() {
        columns.entry(cx).or_default().push(cy);
        if visited.len() > MAX_BODY_CELLS {
            return None;
        }
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (cx + dx, cy + dy);
            if visited.contains(&(nx, ny)) {
                continue;
            }
            let neighbour = world.get(nx, ny);
            if neighbour.material == material {
                // Already claimed by a different live body -- refuse the
                // whole scan rather than silently accepting it. See this
                // function's own doc for the exact ghost-`BodyId` bug this
                // guards against.
                if neighbour.managed() {
                    return None;
                }
                visited.insert((nx, ny));
                stack.push((nx, ny));
            }
        }
    }

    if columns.len() < MIN_BODY_COLUMNS {
        return None;
    }

    let x0 = *columns.keys().next().unwrap();
    let x1 = *columns.keys().next_back().unwrap();
    let width = (x1 - x0 + 1) as usize;
    // 4-connectivity guarantees the x-values present have no gaps (design
    // doc §3a): reaching a liquid cell in column x+2 from one in column x
    // requires passing through *some* liquid cell in column x+1, so a
    // BTreeMap with fewer keys than the span it covers would mean that
    // guarantee broke somewhere upstream, not a case to handle gracefully.
    if columns.len() != width {
        return None;
    }

    let mut top_y = vec![0i32; width];
    let mut bed_y = vec![0i32; width];
    let mut fill = vec![0u32; width];

    for (col_x, ys) in &columns {
        let i = (col_x - x0) as usize;
        let min_y = *ys.iter().min().unwrap();
        let max_y = *ys.iter().max().unwrap();
        // §3b.2: single vertical span per column -- the column's own cell
        // count must equal its y-range, or this column has a gap (two
        // separate bodies stacked in one column, connected to each other
        // only around some other column). Refused, stays CA.
        if ys.len() != (max_y - min_y + 1) as usize {
            return None;
        }
        // §3b.3: a free surface directly above the column's top cell.
        // Refuses sealed/ceiling-capped bodies -- not this design's
        // problem, see `Reports/liquid-heightfield-design.md` §14.
        let above = world.get(*col_x, min_y - 1);
        if !above.is_empty() && world.materials.kind(above.material) != MaterialKind::Gas {
            return None;
        }
        top_y[i] = min_y;
        bed_y[i] = max_y + 1;
        fill[i] = ys.iter().map(|&cy| super::update::liquid_fill(world.get(*col_x, cy)) as u32).sum();
    }

    Some(BodyScan { material, x0, top_y, bed_y, fill })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::explosion;
    use crate::sim::material;
    use crate::sim::parallel;
    use crate::sim::particle::ParticleSystem;
    use crate::sim::update;
    use crate::sim::world::BodyId;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 99, 99))
    }

    const POOL_X0: i32 = 10;
    const POOL_WIDTH: i32 = 40; // >= MIN_BODY_COLUMNS (32)
    const FLOOR_Y: i32 = 55;
    const WATER_Y: i32 = 54;

    /// A flat, `MIN_BODY_COLUMNS`-plus pool on a stone floor with open sky
    /// above — the minimal shape `label_body` will actually promote.
    fn build_pool(w: &mut World) {
        for x in (POOL_X0 - 1)..=(POOL_X0 + POOL_WIDTH) {
            w.set(x, FLOOR_Y, Cell::new(material::STONE, 0));
        }
        for x in POOL_X0..POOL_X0 + POOL_WIDTH {
            w.set(x, WATER_Y, Cell::new(material::WATER, 0));
        }
    }

    /// `build_pool` plus side walls, so the body is genuinely *contained*.
    ///
    /// `build_pool` deliberately has none: its floor runs one cell past the
    /// water on each side and there is open sky above, which is the minimal
    /// shape `label_body` will promote and exactly what the extend/demote
    /// tests need. It is the wrong shape for anything asserting a body
    /// settles and sleeps, because such a pool is sitting on a ledge and
    /// should spill off it -- once `edge_with_room` existed, it does, and
    /// the tests that assumed otherwise were asserting a bug.
    fn build_contained_pool(w: &mut World) {
        build_pool(w);
        for y in 0..FLOOR_Y {
            w.set(POOL_X0 - 1, y, Cell::new(material::STONE, 0));
            w.set(POOL_X0 + POOL_WIDTH, y, Cell::new(material::STONE, 0));
        }
    }

    fn pool_positions() -> Vec<(i32, i32)> {
        (POOL_X0..POOL_X0 + POOL_WIDTH).map(|x| (x, WATER_Y)).collect()
    }

    /// Same material and fill (`aux`) -- the actual "mass" claim -- ignoring
    /// `flags`, which *correctly* differs once one is demoted (`managed`
    /// clears) and the other was snapshotted while still promoted.
    fn same_content(a: Cell, b: Cell) -> bool {
        a.material == b.material && a.aux() == b.aux()
    }

    /// Sum of `liquid_fill` over every `Liquid`-kind cell in the world --
    /// the actual conserved quantity (not cell count, which absorption's
    /// whole point is to change).
    fn total_liquid_fill(w: &World) -> u64 {
        let bounds = w.bounds().unwrap();
        let mut sum = 0u64;
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                let c = w.get(x, y);
                if w.materials.kind(c.material) == MaterialKind::Liquid {
                    sum += update::liquid_fill(c) as u64;
                }
            }
        }
        sum
    }

    #[test]
    fn promoting_a_pool_flags_every_cell_and_moves_no_mass() {
        let mut w = test_world();
        build_pool(&mut w);
        let before: Vec<Cell> = pool_positions().iter().map(|&(x, y)| w.get(x, y)).collect();

        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("a wide flat pool should promote");

        for (&(x, y), &original) in pool_positions().iter().zip(&before) {
            let cell = w.get(x, y);
            assert!(cell.managed(), "promoted cell at ({x},{y}) was not flagged managed");
            assert_eq!(cell.material, original.material, "promotion changed a cell's material");
            assert_eq!(cell.aux(), original.aux(), "promotion moved mass -- aux changed at ({x},{y})");
        }

        // B-1's other half: cell_count == sum(ceil(h[i] / LIQUID_FULL)).
        let body = w.body(id).unwrap();
        let cell_count = body.managed_positions().count();
        let expected: usize = body.h.iter().map(|&h| h.div_ceil(material::LIQUID_FULL as u32) as usize).sum();
        assert_eq!(cell_count, expected, "cell_count != sum(ceil(h[i]/LIQUID_FULL)) invariant broke");
    }

    #[test]
    fn promoting_an_already_managed_pool_a_second_time_fails() {
        // Independent review's finding: `label_body` used to accept a
        // component regardless of whether its cells were already `FLAG_
        // MANAGED` by a different live body. Promoting the same pool twice
        // produced two live `BodyId`s over identical cells -- demoting the
        // first cleared `FLAG_MANAGED` everywhere, silently orphaning the
        // second (`body_index`/`bodies` still held it, but nothing could
        // ever disturb its now-unmanaged cells to trigger its own
        // demotion; a permanent ghost registration).
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");

        let second = w.promote_liquid_body(POOL_X0, WATER_Y);

        assert!(second.is_none(), "promoting an already-managed pool a second time should refuse, not create a second BodyId");
        assert_eq!(w.body_count(), 1, "exactly one live body should exist after the refused second attempt");
    }

    #[test]
    fn a_promoted_body_does_not_move_for_2000_frames() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        // Snapshotted *after* promotion -- promotion itself flips `managed`,
        // so the claim under test is "nothing changes across 2000 frames",
        // not "promotion changed nothing" (a different, already-covered
        // claim, see `promoting_a_pool_flags_every_cell_and_moves_no_mass`).
        let before: Vec<Cell> = pool_positions().iter().map(|&(x, y)| w.get(x, y)).collect();

        for _ in 0..2000 {
            parallel::step(&mut w);
        }

        for (&(x, y), &original) in pool_positions().iter().zip(&before) {
            assert_eq!(w.get(x, y), original, "a promoted cell moved or changed at ({x},{y})");
        }
    }

    #[test]
    fn promote_run_then_demote_round_trip_is_mass_exact() {
        let mut w = test_world();
        build_pool(&mut w);
        let before: Vec<Cell> = pool_positions().iter().map(|&(x, y)| w.get(x, y)).collect();
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");

        for _ in 0..2000 {
            parallel::step(&mut w);
        }
        w.demote_body(id);

        for (&(x, y), &original) in pool_positions().iter().zip(&before) {
            let cell = w.get(x, y);
            assert!(!cell.managed(), "cell at ({x},{y}) is still flagged managed after demotion");
            assert_eq!(cell, original, "promote -> 2000 frames -> demote was not bit identical at ({x},{y})");
        }
        assert_eq!(w.body_count(), 0, "the body's slot should be freed after demotion");
    }

    // --- Every disturbance step 1's own verify list enumerates: paint,
    // erase, an explosion, fire (a reaction writing into a neighbour), a
    // falling grain of sand, and digging out the bed. Each demotes, and
    // demotion itself adds no further change beyond the disturbance's own
    // write -- checked by comparing every *other* cell of the body against
    // its pre-disturbance state.

    #[test]
    fn painting_over_a_managed_cell_demotes_the_body() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();
        let target = (POOL_X0 + 5, WATER_Y);
        let rest: Vec<(i32, i32)> = pool_positions().into_iter().filter(|&p| p != target).collect();
        let rest_before: Vec<Cell> = rest.iter().map(|&(x, y)| w.get(x, y)).collect();

        w.set(target.0, target.1, Cell::new(material::SAND, 0));

        assert_eq!(w.body_count(), 0, "painting a managed cell should have demoted its body");
        assert_eq!(w.get(target.0, target.1).material, material::SAND, "the paint itself should have taken effect");
        for (&(x, y), &original) in rest.iter().zip(&rest_before) {
            let cell = w.get(x, y);
            assert!(!cell.managed(), "cell at ({x},{y}) is still flagged managed after its body was demoted");
            assert!(same_content(cell, original), "demotion disturbed an untouched cell's content at ({x},{y})");
        }
    }

    #[test]
    fn erasing_a_managed_cell_demotes_the_body() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();
        let target = (POOL_X0 + 5, WATER_Y);
        let rest: Vec<(i32, i32)> = pool_positions().into_iter().filter(|&p| p != target).collect();
        let rest_before: Vec<Cell> = rest.iter().map(|&(x, y)| w.get(x, y)).collect();

        w.set(target.0, target.1, Cell::EMPTY);

        assert_eq!(w.body_count(), 0, "erasing a managed cell should have demoted its body");
        assert!(w.get(target.0, target.1).is_empty());
        for (&(x, y), &original) in rest.iter().zip(&rest_before) {
            let cell = w.get(x, y);
            assert!(!cell.managed(), "cell at ({x},{y}) is still flagged managed after its body was demoted");
            assert!(same_content(cell, original), "demotion disturbed an untouched cell's content at ({x},{y})");
        }
    }

    #[test]
    fn digging_out_the_bed_demotes_the_body() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();
        let bed = (POOL_X0 + 5, FLOOR_Y);
        assert!(w.get(bed.0, bed.1).managed(), "test setup: the bed cell should have been flagged as a container cell");
        let before: Vec<Cell> = pool_positions().iter().map(|&(x, y)| w.get(x, y)).collect();

        w.set(bed.0, bed.1, Cell::EMPTY);

        assert_eq!(w.body_count(), 0, "digging out the bed should have demoted the body");
        for (&(x, y), &original) in pool_positions().iter().zip(&before) {
            let cell = w.get(x, y);
            assert!(!cell.managed(), "cell at ({x},{y}) is still flagged managed after its body was demoted");
            assert!(same_content(cell, original), "demotion disturbed the body's own cells' content at ({x},{y})");
        }
    }

    #[test]
    fn an_explosion_touching_the_body_demotes_it() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();
        let mut particles = ParticleSystem::new();

        explosion::trigger(&mut w, &mut particles, POOL_X0 + 5, WATER_Y, 3, 150.0);

        assert_eq!(w.body_count(), 0, "an explosion touching the body should have demoted it");
        for x in POOL_X0 + 20..POOL_X0 + POOL_WIDTH {
            assert!(!w.get(x, WATER_Y).managed(), "a far cell is still flagged managed after demotion");
        }
    }

    #[test]
    fn a_falling_grain_of_sand_displacing_into_the_body_demotes_it() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();
        let target_x = POOL_X0 + 5;
        // Directly above the body's own free surface -- an unmanaged
        // position, so placing the grain here is not itself a disturbance.
        w.set(target_x, WATER_Y - 1, Cell::new(material::SAND, 0));

        for _ in 0..10 {
            parallel::step(&mut w);
        }

        assert_eq!(w.body_count(), 0, "a grain of sand displacing into the body should have demoted it");
        assert_eq!(w.get(target_x, WATER_Y).material, material::SAND, "the grain should have sunk into the managed cell");
    }

    #[test]
    fn a_reaction_writing_into_the_body_demotes_it() {
        // Mirrors `fire.rs`'s own synthetic-reaction test pattern
        // (`react_liquid_a`/`b`/`c`) -- no shipped material reacts with
        // anything today, so a real "fire disturbs a neighbour" scenario
        // needs one built for the test, the same way that suite does.
        let dir = std::env::temp_dir().join("pixel-physics-liquid-body-reaction-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("spark.ron"),
            "(name: \"spark\", kind: Solid, density: 1.0, colors: [(200, 0, 0)], \
             reactions: [(with: \"water\", produces: (\"spark_ash\", \"steam\"), chance: 1.0)])",
        )
        .unwrap();
        std::fs::write(dir.join("spark_ash.ron"), "(name: \"spark_ash\", kind: Solid, density: 1.0, colors: [(80, 80, 80)])").unwrap();
        std::fs::write(dir.join("steam.ron"), "(name: \"steam\", kind: Gas, density: 0.01, colors: [(220, 220, 220)])").unwrap();

        let mut w = test_world();
        w.materials.reload(&dir).unwrap();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();

        let spark = w.materials.id_of("spark").unwrap();
        let target_x = POOL_X0 + 5;
        // Directly above the free surface, same reasoning as the falling-
        // sand test: unmanaged, so placing it is not itself a disturbance,
        // and its own neighbour scan (`fire::update`'s `try_react`) reaches
        // straight down into the managed cell below it.
        w.set(target_x, WATER_Y - 1, Cell::new(spark, 0));

        update::step(&mut w);

        assert_eq!(w.body_count(), 0, "a reaction writing into the body should have demoted it");
        let steam = w.materials.id_of("steam").unwrap();
        assert_eq!(w.get(target_x, WATER_Y).material, steam, "the reaction should have written steam into the managed cell");

        std::fs::remove_dir_all(&dir).ok();
    }

    // --- Step 2 (design doc §11): absorption and lazy rasterization.
    // Deliberately still no solver -- a column that absorbs mass grows
    // taller in place ("the informative failure" the design doc itself
    // predicts) rather than spreading sideways.

    #[test]
    fn absorbing_a_falling_cell_grows_the_body_and_conserves_mass() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let col = 5usize;
        let x = POOL_X0 + col as i32;
        // A full, unmanaged water cell directly above the free surface --
        // `transfer_liquid_vertical` will find the managed cell below it
        // on the very next CA step.
        w.set(x, WATER_Y - 1, Cell::new(material::WATER, 0));

        let h_before = w.body(id).unwrap().h[col];
        let total_before = total_liquid_fill(&w);

        update::step(&mut w);

        let body = w.body(id).expect("the body should still be alive -- absorption is not a disturbance");
        assert_eq!(
            body.h[col],
            h_before + material::LIQUID_FULL as u32,
            "the body's own column should have gained exactly the source cell's fill"
        );
        // Rasterization writes the absorbed mass straight back into the
        // same position the source cell fell from -- the column grew by
        // exactly one cell, and that new cell is the topmost.
        let top = w.get(x, WATER_Y - 1);
        assert!(top.managed(), "the newly rasterized cell should be flagged managed");
        assert_eq!(top.material, material::WATER);
        assert_eq!(top.aux(), 0, "a full rasterized cell should carry the LIQUID_FULL sentinel (aux == 0), not a raw 1000");
        assert_eq!(total_liquid_fill(&w), total_before, "absorption must not create or destroy mass");
    }

    #[test]
    fn repeated_pours_onto_one_column_pile_up_into_a_visible_spike() {
        // The design doc's own predicted step-2 behaviour, made concrete:
        // without a solver to spread it, repeatedly pouring onto one
        // column grows *that* column while its neighbours stay untouched.
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let spike_x = POOL_X0 + 5;
        let quiet_x = POOL_X0 + 25;

        // Dropped from well above the free surface, not repeatedly at a
        // fixed row just above it -- once the first pour rasterizes into
        // that exact row, it is itself managed, and painting a *second*
        // cell directly there would be a disturbance (`World::set`'s own
        // check), demoting the body instead of growing it. Free-falling
        // from further up lands correctly on the current top regardless of
        // how tall the column has grown by that point.
        for _ in 0..4 {
            w.set(spike_x, WATER_Y - 10, Cell::new(material::WATER, 0));
            for _ in 0..12 {
                update::step(&mut w);
            }
        }

        let column_height = |w: &World, x: i32| -> i32 {
            let mut y = WATER_Y;
            let mut h = 0;
            while w.get(x, y).managed() && w.materials.kind(w.get(x, y).material) == MaterialKind::Liquid {
                h += 1;
                y -= 1;
            }
            h
        };
        let spike_height = column_height(&w, spike_x);
        let quiet_height = column_height(&w, quiet_x);
        assert!(spike_height > quiet_height, "repeated pours onto one column should pile up higher than an untouched column: spike={spike_height}, quiet={quiet_height}");
        assert_eq!(quiet_height, 1, "an untouched column should be exactly as tall as it started");
    }

    #[test]
    fn demoting_a_grown_body_clears_every_newly_claimed_cell_too() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let x = POOL_X0 + 5;
        w.set(x, WATER_Y - 1, Cell::new(material::WATER, 0));
        update::step(&mut w);
        assert!(w.get(x, WATER_Y - 1).managed(), "test setup: the pour should have grown the column by one managed cell");
        let grown_cell_before = w.get(x, WATER_Y - 1);

        w.demote_body(id);

        assert_eq!(w.body_count(), 0);
        let grown_cell_after = w.get(x, WATER_Y - 1);
        assert!(!grown_cell_after.managed(), "the newly-grown cell should be unmanaged after demotion");
        assert!(same_content(grown_cell_after, grown_cell_before), "demotion should not change the grown cell's own content");
    }

    #[test]
    fn digging_beside_a_newly_grown_cell_demotes_the_body() {
        // The container-cell bookkeeping must extend as the body grows, not
        // just cover its original footprint at promotion time.
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let x = POOL_X0 + 5;
        w.set(x, WATER_Y - 1, Cell::new(material::WATER, 0));
        update::step(&mut w);
        assert!(w.get(x, WATER_Y - 1).managed(), "test setup: the pour should have grown the column by one managed cell");
        let wall = (x - 1, WATER_Y - 1);
        assert!(w.get(wall.0, wall.1).managed(), "the newly grown row's own wall should have been flagged managed too");

        w.set(wall.0, wall.1, Cell::EMPTY);

        assert_eq!(w.body_count(), 0, "digging beside a newly grown cell should have demoted the body");
    }

    #[test]
    fn absorption_works_under_the_parallel_driver_too() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let col = 5usize;
        let x = POOL_X0 + col as i32;
        w.set(x, WATER_Y - 1, Cell::new(material::WATER, 0));
        let total_before = total_liquid_fill(&w);

        parallel::step(&mut w);

        let body = w.body(id).expect("the body should still be alive under the parallel driver too");
        assert_eq!(body.h[col], material::LIQUID_FULL as u32 * 2, "absorption should credit the body the same way under the parallel driver");
        assert_eq!(total_liquid_fill(&w), total_before, "absorption must not create or destroy mass under the parallel driver either");
    }

    #[test]
    fn a_column_growing_across_a_chunk_boundary_stays_findable() {
        // Independent review's finding: `promote_liquid_body` only ever
        // registers a body's *initial* footprint in `body_index`, but
        // `rasterize_column`'s growth can claim cells in a chunk the body
        // never touched at promotion time. Grown directly via
        // `World::absorb_liquid` with a large fill, rather than physically
        // simulating tens of thousands of frames of falling cells, to cross
        // a real `CHUNK_SIZE` (64-row) boundary in one call.
        let mut w = World::new(Rect::new(0, 0, 99, 199));
        let floor_y = 151;
        let water_y = 150;
        for x in (POOL_X0 - 1)..=(POOL_X0 + POOL_WIDTH) {
            w.set(x, floor_y, Cell::new(material::STONE, 0));
        }
        for x in POOL_X0..POOL_X0 + POOL_WIDTH {
            w.set(x, water_y, Cell::new(material::WATER, 0));
        }
        let id = w.promote_liquid_body(POOL_X0, water_y).expect("test setup: pool should promote");
        let col = 5usize;
        let x = POOL_X0 + col as i32;

        w.absorb_liquid(x, water_y, 70_000);

        let new_top = w.body(id).unwrap().top_y[col];
        assert!(water_y - new_top >= 64, "test setup: growth should have crossed at least one chunk boundary, only reached {}", water_y - new_top);

        // Absorption into the same column, now resolved entirely inside
        // the newly-crossed chunk, must still find the body -- silently
        // lost mass is exactly the bug independent review found.
        let h_before = w.body(id).unwrap().h[col];
        w.absorb_liquid(x, new_top, 500);
        assert_eq!(w.body(id).unwrap().h[col], h_before + 500, "absorption into the newly-crossed chunk was silently lost");

        // A disturbance in the newly-crossed chunk must also still demote
        // the body -- the write-seam's whole invariant otherwise silently
        // stops holding for that region.
        w.set(x, new_top, Cell::new(material::SAND, 0));
        assert_eq!(w.body_count(), 0, "a disturbance in the newly-crossed chunk should have demoted the body");
    }

    // --- Step 3 (design doc §11): the persistent-flux pipe solver.

    fn max_h_diff(w: &World, id: BodyId) -> i64 {
        let body = w.body(id).unwrap();
        let max_h = *body.h.iter().max().unwrap();
        let min_h = *body.h.iter().min().unwrap();
        max_h as i64 - min_h as i64
    }

    #[test]
    fn the_solver_levels_an_uneven_column_over_many_frames() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let spike_x = POOL_X0 + 5;
        // Grows one column dramatically taller than its neighbours,
        // directly via absorb_liquid rather than physically simulating the
        // pour -- the claim under test is the solver's own leveling, not
        // absorption (already covered by step 2's own tests).
        w.absorb_liquid(spike_x, WATER_Y, 20_000);

        let diff_before = max_h_diff(&w, id);
        assert!(diff_before > 15_000, "test setup should have created a real height difference, got {diff_before}");

        for _ in 0..2000 {
            w.step_liquid_bodies();
        }

        let diff_after = max_h_diff(&w, id);
        assert!(diff_after < diff_before / 4, "the solver should have substantially leveled the spike over 2000 frames: before={diff_before}, after={diff_after}");
    }

    #[test]
    fn the_solver_conserves_mass_every_frame() {
        let mut w = test_world();
        build_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        w.absorb_liquid(POOL_X0 + 5, WATER_Y, 20_000);
        // Whole-world liquid fill, not just `body.total_fill()` -- an edge
        // demotion (step 5) legitimately moves mass *out* of the body's
        // own accounting and onto the ordinary grid as ordinary CA liquid,
        // which is real conservation, not a loss. `body.total_fill()`
        // alone can't tell the difference; the true invariant is that the
        // total across both never changes, which is exactly what this
        // scene exercises now that a wave reaching the edge can trigger a
        // real demotion mid-run.
        let total_before = total_liquid_fill(&w);

        for frame in 0..500 {
            w.step_liquid_bodies();
            let total_now = total_liquid_fill(&w);
            assert_eq!(total_now, total_before, "mass drifted at frame {frame}");
        }
    }

    #[test]
    fn the_solver_is_deterministic() {
        let run = || {
            let mut w = test_world();
            build_pool(&mut w);
            let id = w.promote_liquid_body(POOL_X0, WATER_Y).unwrap();
            w.absorb_liquid(POOL_X0 + 5, WATER_Y, 20_000);
            w.absorb_liquid(POOL_X0 + 30, WATER_Y, 8_000);
            for _ in 0..500 {
                w.step_liquid_bodies();
            }
            w.body(id).unwrap().h.clone()
        };
        assert_eq!(run(), run(), "the same starting state should level to bit-identical column heights every run");
    }

    #[test]
    fn the_solver_levels_correctly_across_a_chunk_boundary() {
        let mut w = World::new(Rect::new(0, 0, 99, 199));
        let floor_y = 151;
        let water_y = 150;
        for x in (POOL_X0 - 1)..=(POOL_X0 + POOL_WIDTH) {
            w.set(x, floor_y, Cell::new(material::STONE, 0));
        }
        for x in POOL_X0..POOL_X0 + POOL_WIDTH {
            w.set(x, water_y, Cell::new(material::WATER, 0));
        }
        let id = w.promote_liquid_body(POOL_X0, water_y).expect("test setup: pool should promote");
        let spike_col = 5;
        let spike_x = POOL_X0 + spike_col;
        // Grows the spike column far enough to cross a real chunk boundary
        // (matching `a_column_growing_across_a_chunk_boundary_stays_
        // findable`'s own setup), so the solver's own writes during
        // leveling exercise the cross-chunk path too, not just growth.
        w.absorb_liquid(spike_x, water_y, 70_000);
        assert!(water_y - w.body(id).unwrap().top_y[spike_col as usize] >= 64, "test setup should have crossed a chunk boundary");

        let diff_before = max_h_diff(&w, id);
        for _ in 0..3000 {
            w.step_liquid_bodies();
        }
        let diff_after = max_h_diff(&w, id);
        assert!(diff_after < diff_before / 4, "leveling across a chunk boundary should still substantially converge: before={diff_before}, after={diff_after}");
        assert_eq!(w.body_count(), 1, "the body should still be alive and singular after leveling across a chunk boundary");

        // `register_body_chunks` must keep `body_index` correct not just
        // once (at the growth that first crossed the boundary) but across
        // every subsequent solver frame -- disturbing the crossed chunk's
        // own top cell here, after 3000 frames of solver activity, should
        // still resolve to and demote the body.
        let top = w.body(id).unwrap().top_y[spike_col as usize];
        w.set(spike_x, top, Cell::new(material::SAND, 0));
        assert_eq!(w.body_count(), 0, "a disturbance in the crossed chunk should still demote the body after many solver frames");
    }

    #[test]
    fn a_flat_body_at_equilibrium_does_not_keep_dirtying_its_chunk() {
        // Independent review's finding: `rasterize_column`'s "same whole-
        // cell count" branch used to call `write_liquid_cell`
        // unconditionally, and `Chunk::set_world`/`mark_dirty` don't
        // compare bytes -- so a perfectly flat, already-level body (every
        // interface's flux settling near zero, nothing left to redistribute)
        // still marked its own chunk dirty every single frame the solver
        // ran, defeating design doc §7e's entire point and, transitively,
        // the sleep mechanism a later step depends on.
        let mut w = test_world();
        build_contained_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");

        // Real per-frame order (CA sweep, then liquid bodies), matching
        // `app.rs`, for a few frames so any initial dirty state clears.
        for _ in 0..5 {
            parallel::step(&mut w);
            w.step_liquid_bodies();
        }
        w.take_touched_chunks();

        for _ in 0..20 {
            parallel::step(&mut w);
            w.step_liquid_bodies();
        }
        let touched = w.take_touched_chunks();
        assert!(touched.is_empty(), "a flat, already-level body should not keep dirtying its chunk every frame: {touched:?}");
    }

    // --- Step 4 (design doc §11): quiescence, the terminal snap, body sleep.

    #[test]
    fn a_leveling_body_eventually_snaps_flat_and_sleeps() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        w.absorb_liquid(POOL_X0 + 5, WATER_Y, 20_000);
        // Whole-world fill, not `body.total_fill()` alone -- see `the_
        // solver_conserves_mass_every_frame`'s identical comment: an edge
        // demotion during leveling moves mass out of the body's own
        // accounting onto the ordinary grid, which is real conservation,
        // not a loss, and this scene can genuinely trigger one now that
        // `overloaded_edge` actually fires.
        let total_before = total_liquid_fill(&w);

        for _ in 0..3000 {
            w.step_liquid_bodies();
        }

        let body = w.body(id).expect("body should still be alive");
        assert!(body.asleep, "a body that has fully leveled should be asleep");
        assert!(body.flux.iter().all(|&f| f == 0), "flux should be zeroed by the terminal snap");
        // 10b's own bar: surface flatness within 0.1% of LIQUID_FULL (<= 1
        // fill unit) between adjacent columns -- the terminal snap should
        // deliver *exact* (differing only by the deterministic remainder
        // distribution, at most 1 unit), not merely "close."
        let max_adjacent_diff = body.h.windows(2).map(|w| (w[0] as i64 - w[1] as i64).abs()).max().unwrap();
        assert!(max_adjacent_diff <= 1, "the terminal snap should leave adjacent columns within 1 fill unit of each other, got {max_adjacent_diff}");
        assert_eq!(total_liquid_fill(&w), total_before, "the terminal snap (and any edge demotion along the way) must not lose or manufacture mass");
    }

    #[test]
    fn a_sleeping_body_lets_its_chunk_reach_zero_active_chunks() {
        let mut w = test_world();
        build_contained_pool(&mut w);
        w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        w.absorb_liquid(POOL_X0 + 5, WATER_Y, 20_000);

        for _ in 0..3000 {
            parallel::step(&mut w);
            w.step_liquid_bodies();
        }

        assert_eq!(w.active_chunk_count(), 0, "a body that has fully leveled and slept should let its chunks settle -- design doc B-6");
    }

    #[test]
    fn absorption_wakes_a_sleeping_body() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        for _ in 0..500 {
            w.step_liquid_bodies();
        }
        assert!(w.body(id).unwrap().asleep, "test setup: a flat pool should already be asleep");

        w.absorb_liquid(POOL_X0 + 5, WATER_Y, 500);

        assert!(!w.body(id).unwrap().asleep, "absorbing new mass should wake a sleeping body");
    }

    // --- Step 5 (design doc §11): try_extend, edge demotion, cooldowns.

    /// Shared setup for the `try_extend` tests below: promote a pool, then
    /// force a *real* edge demotion (overload column 0, run one step) --
    /// the only way a position immediately beside a live body's edge ever
    /// legitimately becomes unmanaged. Painting a fresh puddle directly
    /// there via `World::set` is not a valid way to set this up: that
    /// exact position is always the body's own container wall, flagged
    /// managed at promotion, so writing into it is itself a disturbance
    /// and demotes the *whole* body before `try_extend` ever runs --
    /// confirmed while first writing this test (both tests panicked with
    /// the body gone entirely). A real demotion is the only path that
    /// correctly clears the flag first.
    fn build_pool_with_a_real_edge_demotion(w: &mut World) -> (BodyId, i32) {
        build_pool(w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        // Just enough to cross the overload threshold once (h[0] = 4000
        // against an average of ~1075, threshold ~2150) and settle back
        // near flat afterward (remaining ~39000 over 39 columns is again
        // ~1000/column) -- deliberately not a large amount, to keep this
        // setup to exactly one demotion rather than a cascade that would
        // keep moving `x0` throughout the test's own wait loop.
        w.absorb_liquid(POOL_X0, WATER_Y, 3_000);
        w.step_liquid_bodies(); // triggers the edge demotion at frame 0
        let demoted_x = w.body(id).unwrap().x0 - 1; // the column that was just demoted
        assert!(!w.get(demoted_x, WATER_Y).managed(), "test setup: the demoted column should be genuinely unmanaged");
        (id, demoted_x)
    }

    /// Absorbing at a body's edge must not destroy the fill.
    ///
    /// `update::transfer_liquid_vertical` empties the source cell and credits
    /// the whole amount through `absorb_liquid` in the same call, so that a
    /// debit can never be separated from its credit — `CellSurface::
    /// absorb_liquid`'s own doc says as much. A bounds check inside
    /// `absorb_liquid` was separating them anyway: `find_body_at` resolves a
    /// body for its *container* cells too (bed and walls, at `x0 - 1` and
    /// `x0 + columns()`), which are outside `h`, and the fill was silently
    /// dropped there.
    ///
    /// Found by review, and reachable in principle via `try_extend`'s
    /// container claim. Latent in practice today only because nothing in
    /// production promotes a body (`127e177`).
    #[test]
    fn absorbing_at_a_bodys_container_edge_conserves_the_fill() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        let before = w.body(id).unwrap().total_fill();

        // A container cell: outside `h`, but `owns` resolves the body for it.
        let edge_x = POOL_X0 - 1;
        assert!(w.body(id).unwrap().owns(edge_x, WATER_Y), "test setup: the wall should resolve to this body");

        w.absorb_liquid(edge_x, WATER_Y, 2_500);

        let after = w.body(id).expect("body should still be alive").total_fill();
        assert_eq!(
            after,
            before + 2_500,
            "absorbing at the body's own container edge lost the fill: {before} -> {after}"
        );
    }

    /// A promoted body that is internally level must still spread onto open
    /// floor beside it.
    ///
    /// This is the gap that got automatic promotion reverted (`127e177`):
    /// "promoting an already-flat rectangular block freezes it permanently
    /// short of its correct spread, since the persistent-flux solver has no
    /// mechanism to drive an internally-level body to expand into open floor
    /// space beside it."
    ///
    /// Two gates stood in the way, both keyed on the wrong question:
    /// `overloaded_edge` wanted an edge column above `EDGE_OVERFLOW_RATIO`
    /// times the body's own average, which a *level* body never is; and
    /// `step`'s `asleep` early return skipped the check entirely, reasoning
    /// that a body which fell asleep flat cannot have changed. True of its
    /// `h[]`, and beside the point -- flat is not the same as contained.
    ///
    /// Measured before the fix: not one cell of movement in 4000 frames.
    /// After: the body walks outward a column at a time and the water
    /// crosses the whole 150 columns of open floor.
    ///
    /// The CA sweep runs here alongside `step_liquid_bodies`, and that is
    /// load-bearing rather than incidental realism. §6c's outflow *is* edge
    /// demotion -- the body hands a column back and ordinary water rules
    /// carry it away. With `step_liquid_bodies` alone the shed column simply
    /// sits there and `try_extend` reclaims it on the next cycle, which
    /// reads as a thrash between the two and moves no water at all. A first
    /// version of this test did exactly that and looked like a failure.
    #[test]
    fn a_level_body_spreads_onto_open_floor_beside_it() {
        let mut w = World::new(Rect::new(0, 0, 199, 63));
        for x in 0..200 {
            w.set(x, FLOOR_Y, Cell::new(material::STONE, 0));
        }
        // Walled on the left, wide open to the right.
        for y in 0..FLOOR_Y {
            w.set(POOL_X0 - 1, y, Cell::new(material::STONE, 0));
        }
        for x in POOL_X0..POOL_X0 + POOL_WIDTH {
            for y in WATER_Y - 3..=WATER_Y {
                w.set(x, y, Cell::new(material::WATER, 0));
            }
        }
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: the pool should promote");
        let started_at = POOL_X0 + POOL_WIDTH - 1;
        let fill_before = w.body(id).unwrap().total_fill();

        for _ in 0..4000 {
            super::super::update::step(&mut w);
            w.step_liquid_bodies();
        }

        let furthest = (0..200)
            .rev()
            .find(|&x| (0..FLOOR_Y).any(|y| w.get(x, y).material == material::WATER))
            .expect("the water cannot have vanished entirely");
        assert!(
            furthest > started_at + 50,
            "a level body on open floor only reached x={furthest}, having started at x={started_at}              with 150 columns of room (before the fix it moved zero cells in 4000 frames)"
        );
        // It spread by *shedding*, not by inventing water.
        let body_fill = w.body(id).map_or(0, |b| b.total_fill());
        assert!(body_fill < fill_before, "the body should have handed mass back to the CA, not kept all {fill_before}");
    }

    /// A body that has shed itself down to a single column must hand the rest
    /// back, not sit on it.
    ///
    /// `LiquidBody::step` bails at `columns() < 2` — there are no interfaces
    /// left to move flux across, so the solver has nothing to do. Before edge
    /// shedding could fire on an *uncontained* body (`edge_with_room`) that
    /// state was unreachable in practice; it is now the normal end of a body
    /// spreading onto open floor.
    ///
    /// Without the hand-back the leftovers strand. Measured on a 100-column
    /// basin: a promoted body walked itself down to one column still holding
    /// 40,000 fill — forty cells of water stacked in a single column that
    /// nothing would ever move again — and the basin never levelled at all,
    /// finishing at a flatness of 24,174 against a bar of 20.
    #[test]
    fn a_body_shed_down_to_one_column_demotes_instead_of_stranding() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: the pool should promote");

        // `build_pool` has no walls, so this body is uncontained and sheds.
        for _ in 0..6000 {
            super::super::update::step(&mut w);
            w.step_liquid_bodies();
        }

        match w.body(id) {
            None => {} // handed back entirely, which is the point
            Some(b) => assert!(
                b.columns() >= 2,
                "a body shed down to {} column(s) is stranded: its solver bails at <2, so the {} fill it \
                 still holds can never move again",
                b.columns(),
                b.total_fill()
            ),
        }
    }

    #[test]
    fn try_extend_claims_an_adjacent_puddle() {
        let mut w = test_world();
        let (id, demoted_x) = build_pool_with_a_real_edge_demotion(&mut w);
        let columns_before = w.body(id).unwrap().columns();

        // Past the post-demotion cooldown, landing on an EXTEND_INTERVAL-
        // aligned frame.
        for _ in 0..(DEMOTE_COOLDOWN_FRAMES + EXTEND_INTERVAL * 2) {
            w.begin_step();
            w.step_liquid_bodies();
        }

        let body = w.body(id).expect("body should still be alive");
        assert!(body.columns() > columns_before, "try_extend should have reclaimed the demoted column once the cooldown expired");
        assert!(w.get(demoted_x, WATER_Y).managed(), "the reclaimed cell should now be managed again");
    }

    /// Extending must not freeze the CA water just beyond the column it
    /// claims.
    ///
    /// `try_extend` flags every newly-exposed container position `managed`.
    /// At promotion that is safe by construction — `label_body` takes a
    /// maximal connected region, so nothing adjacent to it is more of the
    /// same unmanaged liquid. An incremental claim breaks the invariant:
    /// claiming column `x0 - 1` exposes `x0 - 2` as new container, and that
    /// can be an ordinary CA puddle of the body's own material.
    ///
    /// Flagged, such a cell is simulated by nothing (`update_cell` skips
    /// managed cells) and owned by nothing (no body's `h[]` covers it) —
    /// water frozen permanently, and the reachable path into
    /// `World::absorb_liquid`'s bounds miss. Found by review.
    #[test]
    fn try_extend_does_not_freeze_the_ca_water_beyond_the_column_it_claims() {
        let mut w = test_world();
        let (id, demoted_x) = build_pool_with_a_real_edge_demotion(&mut w);

        // An ordinary CA puddle one column further out than the one
        // `try_extend` is about to reclaim — exactly the position that
        // becomes "new container" the moment it does.
        let beyond_x = demoted_x - 1;
        w.set(beyond_x, FLOOR_Y, Cell::new(material::STONE, 0));
        w.set(beyond_x, WATER_Y, Cell::new(material::WATER, 0));
        assert!(!w.get(beyond_x, WATER_Y).managed(), "test setup: the puddle beyond starts unmanaged");

        for _ in 0..(DEMOTE_COOLDOWN_FRAMES + EXTEND_INTERVAL * 2) {
            w.begin_step();
            w.step_liquid_bodies();
        }

        let body = w.body(id).expect("body should still be alive");
        assert!(w.get(demoted_x, WATER_Y).managed(), "test setup: the demoted column should have been reclaimed");

        let beyond = w.get(beyond_x, WATER_Y);
        let owned_by_body = body.owns(beyond_x, WATER_Y) && beyond_x >= body.x0;
        assert!(
            !beyond.managed() || owned_by_body,
            "the CA puddle at x={beyond_x} was flagged managed without being taken into the body: \
             nothing simulates it and nothing owns it, so it is frozen for good"
        );
    }

    #[test]
    fn try_extend_refuses_a_puddle_of_a_different_material() {
        let mut w = test_world();
        let (id, demoted_x) = build_pool_with_a_real_edge_demotion(&mut w);
        let columns_before = w.body(id).unwrap().columns();
        // Something *other* than water settled in the demoted spot instead
        // -- `set_owned` here is test tooling standing in for "the CA
        // settled something else there," not a disturbance to react to.
        let oil = w.materials.id_of("oil").expect("oil is a compiled-in material");
        for y in w.body(id).unwrap().top_y[0]..w.body(id).unwrap().bed_y[0] {
            w.set_owned(demoted_x, y, Cell::new(oil, 0));
        }

        for _ in 0..(DEMOTE_COOLDOWN_FRAMES + EXTEND_INTERVAL * 2) {
            w.begin_step();
            w.step_liquid_bodies();
        }

        // Asserted against the *left* edge specifically, not the column
        // count: the body may legitimately shed a column at its other edge
        // now that an uncontained edge spills (`edge_with_room`), which has
        // nothing to do with what this test is about.
        let body = w.body(id).unwrap();
        assert!(body.x0 > demoted_x, "the oil column at x={demoted_x} was claimed; body now starts at x0={}", body.x0);
        assert!(!w.get(demoted_x, WATER_Y).managed(), "a different-material puddle should never be claimed");
        let _ = columns_before;
    }

    #[test]
    fn an_overloaded_edge_column_demotes_when_it_has_somewhere_to_spill() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        // Massively overloads the leftmost column relative to the body's
        // own average, with open space just outside its left edge.
        w.absorb_liquid(POOL_X0, WATER_Y, 20_000);
        let columns_before = w.body(id).unwrap().columns();
        let total_before = w.body(id).unwrap().total_fill();

        w.step_liquid_bodies();

        let body = w.body(id).expect("body should still be alive after an edge demotion");
        assert!(body.columns() < columns_before, "an overloaded edge column with somewhere to spill should have demoted");
        // No mass moves on demotion (design doc §5b/§6c) -- the body's own
        // remaining total plus whatever the demoted column now holds as
        // ordinary CA liquid should equal what the body held before.
        let demoted_x = POOL_X0; // the original leftmost column
        let mut demoted_fill = 0u64;
        for y in 0..w.bounds().unwrap().max_y {
            let c = w.get(demoted_x, y);
            if w.materials.kind(c.material) == MaterialKind::Liquid {
                demoted_fill += update::liquid_fill(c) as u64;
            }
        }
        assert_eq!(body.total_fill() + demoted_fill, total_before, "demotion must not lose or manufacture mass");
    }

    #[test]
    fn extend_is_suppressed_during_the_post_demotion_cooldown_then_resumes() {
        let mut w = test_world();
        build_pool(&mut w);
        let id = w.promote_liquid_body(POOL_X0, WATER_Y).expect("test setup: pool should promote");
        w.absorb_liquid(POOL_X0, WATER_Y, 20_000);
        w.step_liquid_bodies(); // triggers the edge demotion at frame 0
        let columns_after_demotion = w.body(id).unwrap().columns();
        assert!(w.body(id).unwrap().extend_cooldown_until > 0, "test setup: a demotion should have set a cooldown");

        // The demoted column's own cells are still sitting there, now
        // unmanaged, matching material and row range -- exactly what
        // try_extend would otherwise reclaim on its very next check.
        // Snapshotted once rather than re-read each iteration, and the
        // loop stops strictly *before* that frame (not through it): `step`
        // itself allows `try_extend` on the boundary frame where `world.
        // frame == extend_cooldown_until` (its own gate is `>=`), so
        // looping up to and including that frame would make "should stay
        // suppressed" a coin flip on whichever frame the claim actually
        // lands on, rather than a real test of the window before it.
        // Bounded well past what a single cooldown window should ever
        // need, so a regression that makes the cooldown never actually
        // expire (this test's own history -- see `step`'s doc on the
        // extend/demote thrash it used to hit) fails loudly instead of
        // hanging the test suite.
        let cooldown_until = w.body(id).unwrap().extend_cooldown_until;
        let mut guard = 0;
        while w.frame + 1 < cooldown_until {
            w.begin_step(); // advances world.frame without a full CA sweep
            w.step_liquid_bodies();
            guard += 1;
            assert!(guard < 10_000, "cooldown never expired -- likely an extend/demote thrash");
        }
        assert_eq!(w.body(id).unwrap().columns(), columns_after_demotion, "extend should stay suppressed for the whole cooldown window");

        // Run past the cooldown, landing on an EXTEND_INTERVAL-aligned
        // frame, and confirm the puddle is eventually reclaimed.
        for _ in 0..EXTEND_INTERVAL * 2 {
            w.begin_step();
            w.step_liquid_bodies();
        }
        assert!(w.body(id).unwrap().columns() > columns_after_demotion, "extend should reclaim the puddle once the cooldown has expired");
    }

    #[test]
    fn a_body_that_wakes_via_try_extend_from_asleep_registers_its_new_chunk() {
        // Independent review's finding: `try_extend` runs even while
        // asleep so a sleeping body can reclaim a neighbour, and a
        // successful claim can grow the footprint into a chunk the body
        // never touched before. `step_liquid_bodies` used to gate
        // `register_body_chunks` on the body's *pre-step* sleep state
        // (`was_asleep`) alone, so a body that was already asleep going
        // into the exact frame it reclaims a chunk-crossing column skipped
        // registration entirely -- silently desyncing disturbance/demotion
        // handling in that chunk from then on.
        let mut w = World::new(Rect::new(0, 0, 99, 199));
        let floor_y = 151;
        let water_y = 150;
        for x in (POOL_X0 - 1)..=(POOL_X0 + POOL_WIDTH) {
            w.set(x, floor_y, Cell::new(material::STONE, 0));
        }
        for x in POOL_X0..POOL_X0 + POOL_WIDTH {
            w.set(x, water_y, Cell::new(material::WATER, 0));
        }
        let id = w.promote_liquid_body(POOL_X0, water_y).expect("test setup: pool should promote");
        w.absorb_liquid(POOL_X0, water_y, 3_000);
        w.step_liquid_bodies(); // triggers the edge demotion at frame 0
        let demoted_x = w.body(id).unwrap().x0 - 1;
        assert!(!w.get(demoted_x, water_y).managed(), "test setup: the demoted column should be genuinely unmanaged");

        // Pile extra water directly on the now-ordinary demoted column,
        // reaching well past a real 64-row chunk boundary above it -- a
        // chunk the body's own footprint has never touched.
        for dy in 1..90 {
            w.set(demoted_x, water_y - dy, Cell::new(material::WATER, 0));
        }
        let new_chunk_y = water_y - 89;
        assert!(water_y - new_chunk_y >= 64, "test setup: the pile should cross a real chunk boundary");

        // Let the body settle asleep, strictly before the post-demotion
        // cooldown expires -- the reclaim below must land on a frame the
        // body was genuinely already asleep going into, or this doesn't
        // exercise the bug. Bounded well past what settling should ever
        // need, so a regression that stops the body from sleeping fails
        // loudly instead of hanging the suite.
        let cooldown_until = w.body(id).unwrap().extend_cooldown_until;
        let mut guard = 0;
        while !w.body(id).unwrap().asleep {
            assert!(w.frame + 1 < cooldown_until, "test setup: body did not settle asleep before the cooldown expired");
            w.begin_step();
            w.step_liquid_bodies();
            guard += 1;
            assert!(guard < 10_000, "body never settled asleep");
        }

        // Advance to the reclaiming frame without ever calling
        // `step_liquid_bodies` in between, so the body stays asleep right
        // up to (and going into) that exact call.
        while w.frame < cooldown_until || !w.frame.is_multiple_of(EXTEND_INTERVAL) {
            w.begin_step();
        }
        assert!(w.body(id).unwrap().asleep, "test setup: still asleep immediately before the reclaiming frame");

        w.step_liquid_bodies();
        assert!(!w.body(id).unwrap().asleep, "test setup: try_extend should have reclaimed the column and woken the body");
        assert!(w.get(demoted_x, new_chunk_y).managed(), "test setup: the reclaim should have grown all the way up into the new chunk");

        // The real assertion: a disturbance in that newly-claimed chunk
        // must still demote the body. Before the fix this silently no-op'd
        // because the chunk was never added to `body_index`.
        w.demote_body_at(demoted_x, new_chunk_y);
        assert_eq!(w.body_count(), 0, "a disturbance in the newly-claimed chunk should have demoted the body");
    }
}
