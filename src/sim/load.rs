//! The support forest, and what every structural cell is carrying.
//!
//! `structural.rs` computes each cell's **distance** to an anchor. This
//! module answers the different question that distance cannot:
//! *how much is hanging off this cell, and how far out?*
//!
//! # Why reach was the wrong criterion
//!
//! Failure used to be evaluated per cell as *its own reach* against *its
//! own span*. That is a statement about where a cell is, not about what it
//! holds, and the two come apart exactly where it matters. A crack at a
//! beam's root weakens a cell sitting at distance ~10, which would never
//! have failed at any span; the far end that *is* near its limit is not the
//! part that was hit. So a worked root never gave way (six blows at the
//! join of a 160-cell shelf left it standing — `filmstrip scene=worked`),
//! and an overhang hanging by a two-cell ligament did not notice.
//!
//! Rock does not fail where it is furthest from support. It fails where the
//! stress is highest, which is at the neck. So the criterion is
//! **`torque > capacity`**, and reach becomes purely a support-ordering
//! potential — it decides *which way is downhill toward an anchor*, and
//! nothing else.
//!
//! # Load is a moment, not a weight
//!
//! Fifty cells stacked against a wall is fine; the same fifty reaching
//! fifty cells out is not. Mass alone cannot tell those apart and will let
//! any thick tower collapse or any long shelf stand depending on how it is
//! tuned. The quantity is bending moment, `Σ mᵢ(xᵢ − x_c)`, and it does not
//! need a per-cell scan of everything a cell supports because the sum
//! separates:
//!
//! ```text
//! torque(c) = | Sx(c) − x_c · M(c) |     M = Σ mᵢ,  Sx = Σ mᵢ xᵢ
//! ```
//!
//! Two plain sums over the set of cells `c` supports. `mass` is 1 per cell
//! for now — material density is the obvious next term and deliberately not
//! the first one, since it multiplies a quantity that has not been
//! calibrated by eye yet.
//!
//! This distinguishes the cases that matter with no special-casing: a
//! vertical tower's mass sits directly above it so its torque is ~0 and it
//! stands at any height; a cantilever's grows with length so its root
//! fails; a big overhang on a thin ligament concentrates the whole moment
//! into a section that cannot carry it, and the neck snaps.
//!
//! # Where the forest comes from, and why nothing is stored
//!
//! Every cell's support parent is the neighbour its distance relaxation
//! took its minimum from, which makes the support graph a forest rooted at
//! anchors. `Reports/load-model-handoff.md` §3 called for that parent to be
//! recorded in a sparse side table on `World`, on the grounds that
//! `Cell::flags` is full (8/8 bits) and `Cell` must not grow again.
//!
//! **Both constraints are satisfied better by storing nothing at all.**
//! `support_parent` re-derives the parent from the neighbours' currently
//! stored distances — the identical `argmin` the relaxation itself
//! computes, four reads and the same `NEIGHBOURS_4` tie-break, so it is
//! deterministic and it is exact. A stored parent can only be *staler* than
//! that, and a side table keyed by position is a determinism trap
//! (`PLAN.md` issue #7) and a staleness hazard for no benefit. The
//! measurement that settled it: a check on the `worked` shelf's ligament
//! floods ~1,500 cells, which is well inside one frame's budget and is the
//! largest subtree any of the acceptance scenes produces.
//!
//! **This is not the thing §2c forbids.** What must not be done is
//! re-deriving a cell's *subtree* by flooding to "neighbours with greater
//! distance": equal-cost paths make that count a subtree twice, and with a
//! zero-cost step it can cycle outright. Deriving the *parent* is a
//! function — every cell has exactly one — so flooding to "neighbours whose
//! parent is me" visits each cell at most once by construction. The
//! difference is the whole reason this is safe and that was not.
//!
//! # Two ways to fail, and they are not the same event
//!
//! - **Unsupported** — the support chain does not reach an anchor at all.
//!   Nothing holds this; it is not overloaded, it is falling. The piece
//!   that comes away is the connected region that *also* cannot reach an
//!   anchor, which is ordinary connectivity.
//! - **Overloaded** — the chain does reach an anchor, but the moment
//!   exceeds what the section can carry. The piece that comes away is the
//!   failing cell's **subtree**: precisely what it was holding up, which is
//!   what stops being held the instant it breaks.
//!
//! That second one is what replaces `rigid::label_failing_region`'s "when
//! anything fails, take the whole appendage" flood. That flood existed
//! *only* because the root never failed on its own; once it does, the piece
//! is a consequence of the model rather than a mechanism bolted beside it.

use std::collections::{HashMap, HashSet, VecDeque};

use super::material::MaterialKind;
use super::structural::{edge_is_cracked, is_body_material, NEIGHBOURS_4};
use super::world::World;

/// Cells one detached piece may contain, and how far the support search
/// will look before giving up. Not a "too big to break" test —
/// `rigid::fracture` splits whatever it is given into fragments, so a large
/// collapse yields *more* pieces, never fewer.
///
/// Was 4,000, which was visibly too small: `scene=ligament`'s overhang is
/// 110x40, so when its neck snapped the search over the freed piece ran out
/// of room, defaulted to "still supported", and left 4,400 cells of rock
/// hanging in open air. Raised once the anchor cache below made the search
/// cost O(region) per *frame* rather than per check — the cap had been
/// standing in for a cost problem that is now fixed properly.
const MAX_REGION_CELLS: usize = 20_000;

/// Cells one subtree walk may visit before it stops accumulating.
///
/// Much smaller than `MAX_REGION_CELLS`, and it is a **performance** bound
/// rather than a physical one — this walk runs per evaluated cell, so its
/// cap is what stops a tall column's surface from costing the square of its
/// own height. Measured before it existed: `scene=capped` reached a
/// **6,556 ms** frame and `scene=strike` 4,456 ms, because every cell of a
/// solid block flooded thousands of cells to ask what it was carrying.
///
/// Truncating is safe in the one direction that matters. The partial sum is
/// a genuine **lower bound** on the torque — every cell counted is really
/// there — so failing on it is always correct, and the only thing lost is
/// sensitivity in cases that were marginal anyway. The opposite choice
/// (guess high when truncated) fails terrain upward, which is the outcome
/// this whole subsystem has been burned by three times.
///
/// A piece bigger than this simply comes down in stages: the truncated
/// region drops, the rest re-evaluates and drops after it. Progressive
/// collapse is what `Reports/fracture-mechanics-design.md` §3.4 asks for
/// anyway — "a collapse that resolves over a second reads better than one
/// that resolves instantly".
const MAX_SUBTREE_CELLS: usize = 8192;

/// Cells every load walk in one frame may visit between them, across all
/// checks. The counterpart of `scheduler::MAX_SITES_PER_FRAME`, and needed
/// alongside it for the same reason: that cap bounds how many sites are
/// *examined*, and this one bounds how much work examining them may do.
///
/// A cascade wants to resolve all at once — each break changes loads, which
/// triggers more breaks, in the same frame — and that is both a frame spike
/// and worse-looking than a progressive collapse. Deferring the overflow to
/// the next frame is the same "not lost, not requeued, just deferred" shape
/// the site cap already uses.
///
/// **Set from measurement, not from an aspiration.** One walked cell costs
/// roughly a microsecond, almost all of it hashed `World::get` (issue #5's
/// pattern: four neighbours, each asking who *its* parent is, is ~25
/// lookups). 6,000 buys about 6 ms of a 16.6 ms frame at 60 Hz, leaving the
/// CA sweep its room. Raising it is the first thing to try if collapses
/// feel sluggish, and indexing the chunk directly instead of going through
/// `World::get` is what would make it affordable to.
pub const MAX_LOAD_CELLS_PER_FRAME: u32 = 12_000;

/// How far up a support chain a walk will follow before giving up and
/// calling the cell supported.
///
/// The distance relaxation is label-correcting and converges over several
/// ticks, so a walk over a half-converged forest can follow a parent that
/// is no longer valid. Two guards, neither optional: every step must
/// *strictly* decrease distance (which alone makes a cycle impossible, and
/// bounds the walk by the cell's own distance), and the walk is capped
/// here. Over the cap resolves to "supported", because the failure mode of
/// the other choice is a mountain deciding it is falling.
const MAX_SUPPORT_WALK: usize = 512;

/// Longest section a cell gets credit for, in cells. Past this the capacity
/// is already far beyond anything a 512-wide world contains and each extra
/// cell is another read for nothing. Squared below, so this is a ceiling of
/// 1600 on the section term.
const MAX_SECTION: i64 = 40;

/// Edges a cell has, and so the denominator for how much of its section a
/// fracture has taken away.
const CRACK_FACES: i64 = 4;

/// The middle-third rule: a reaction that cannot pull must stay within the
/// kern, so a footing of width B resists at most `N * B / 6`. See
/// `bearing_moment`, which is what rock standing on loose grain is judged
/// by now.
///
/// `GRANULAR_CAPACITY_DIVISOR` stood here beside it -- a flat 64x cut to
/// the bending capacity of anything bedded on powder. It is gone rather
/// than merely unused: `bearing_moment` replaced it, and its own doc
/// records why (it was a counterweight for `structural::tick` rooting
/// eagerly on powder, and once the rooting was fixed it had nothing left
/// to cancel). Left as a dead constant it read as a lever someone might
/// still reach for, which is the opposite of true.
const KERN_DENOMINATOR: i64 = 6;

/// What one cell carries and what it can carry. Everything the failure test
/// looks at, in one struct, so the hover inspector and the criterion cannot
/// disagree about a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Load {
    /// Total supported mass, including this cell's own.
    pub mass: u32,
    /// `Σ mᵢ xᵢ` over the same set. `i64`, not `i32`: a 4,000-cell region at
    /// x≈500 is only ~2×10⁶, but a streamed world's coordinates are
    /// unbounded and nothing here wants a silent wrap.
    pub moment: i64,
    /// `|moment − x·mass|` — the bending moment about this cell.
    pub torque: i64,
    /// What the section can carry. See `capacity`.
    pub capacity: i64,
    /// Whether the support chain reaches an anchor at all.
    pub supported: bool,
    /// Whether the subtree walk hit `MAX_REGION_CELLS`, so `mass`/`moment`
    /// are floors rather than totals. Surfaced rather than hidden: a
    /// truncated reading that looks safe is exactly the kind of thing that
    /// wastes a session.
    pub truncated: bool,
}

impl Load {
    /// The stress ratio — `Reports/fracture-mechanics-design.md` §1c's
    /// "load / capacity **is** stress", and what crack propagation will
    /// follow when it is built.
    pub fn stress(&self) -> f32 {
        if self.capacity <= 0 {
            return f32::INFINITY;
        }
        self.torque as f32 / self.capacity as f32
    }

    fn fails(&self) -> bool {
        !self.supported || self.torque > self.capacity
    }
}

/// Whether `(x, y)` is worth evaluating at all.
///
/// **Attached, no cracked edge, no empty neighbour → cannot fail, skip.**
/// Two tests covering nearly the whole world, and they run before any
/// computation: bulk terrain is braced by mass the slice cannot show, and
/// asking what it carries is both meaningless and the difference between a
/// cost proportional to *surface area* and one proportional to volume. The
/// same argument that makes the distance pass affordable.
pub fn is_structurally_interesting(world: &World, x: i32, y: i32) -> bool {
    let cell = world.get(x, y);
    if !cell.attached() {
        return true; // foreground: it has to earn its support like anything else
    }
    NEIGHBOURS_4.iter().any(|&(dx, dy)| {
        edge_is_cracked(world, x, y, dx, dy) || world.get(x + dx, y + dy).material == super::material::EMPTY
    })
}

/// The neighbour `(x, y)`'s support comes from, or `None` if it is an
/// anchor or has no valid support.
///
/// The identical `argmin` the relaxation computes — cost charged by
/// direction, cracked edges skipped because support does not cross a
/// fracture, burning neighbours skipped because their material may change
/// out from under this. Ties break on `NEIGHBOURS_4` order, which is what
/// makes the forest identical run to run.
///
/// The `neighbour < own` test is the staleness guard: a parent must be
/// *strictly* closer to an anchor. That alone makes a cycle impossible and
/// bounds any walk by the cell's own distance. A cell whose stored distance
/// has gone stale-low relative to its neighbours comes back `None` and is
/// treated as a chain root — never as free of load, which would be the
/// dangerous direction.
pub fn support_parent(world: &World, x: i32, y: i32) -> Option<(i32, i32)> {
    let cell = world.get(x, y);
    let own = cell.aux();
    if own == 0 {
        return None; // an anchor is held from outside the model; nothing above it to find
    }
    let mut best: Option<((i32, i32), u16)> = None;
    for (dx, dy) in NEIGHBOURS_4 {
        if edge_is_cracked(world, x, y, dx, dy) {
            continue;
        }
        let (nx, ny) = (x + dx, y + dy);
        let neighbour = world.get(nx, ny);
        if !is_body_material(world, neighbour.material) || neighbour.organism_id() != 0 || neighbour.is_burning() {
            continue;
        }
        if neighbour.aux() >= own {
            continue; // not closer to an anchor than we are -- cannot be holding us up
        }
        // `y` grows downward: `dy == 1` is the cell beneath (standing on
        // it) and `dy == -1` the cell above (hanging from it). Getting this
        // backwards silently prices towers as cantilevers.
        let step = {
            let m = world.materials.get(neighbour.material);
            match dy {
                1 => m.support_cost_below,
                -1 => m.support_cost_above,
                _ => m.support_cost_beside,
            }
        };
        let cost = neighbour.aux().saturating_add(step);
        if best.is_none_or(|(_, b)| cost < b) {
            best = Some(((nx, ny), cost));
        }
    }
    best.map(|(pos, _)| pos)
}

/// Whether a horizontal run is a **member** -- something that fails as one
/// piece -- rather than a window cut out of something larger.
///
/// A member ends in air at both hands: a wall, a pillar, a door jamb. The
/// forty cells `MAX_SECTION` will hand back from the middle of a slab that
/// keeps going are not a member, they are where the walk ran out of
/// patience, and there is nothing for the cells of such a run to
/// redistribute between.
///
/// # Why this test exists
///
/// The first version of the section rule in `evaluate_within` had no such
/// test, and it fed `Reports/next-session-handoff.md` 1c -- the big
/// strike's *lateral* unzip, where a shallow surface sheet fails sideways
/// along itself. Coupling every forty-cell window of that sheet made every
/// cell of it inherit the worst load anywhere within twenty cells, which is
/// the unzip's own mechanism handed a shortcut. Measured on
/// `seedsweep.sh strike=12` run to rest: rock destroyed p90 2,227 -> 4,465,
/// with single runs reporting 49,265 overload failures. With this test the
/// sweep comes back to the cell.
///
/// # This is not the forbidden size cap
///
/// `CLAUDE.md` rules out `if too_big { return }`, and this is close enough
/// to be worth saying why it is not that. The forbidden shape is a *work*
/// cap deciding whether behaviour happens, so the biggest cases get the
/// least of it. This asks what the object is. An unbounded run has no ends
/// to spread a load between, so its cells keep exactly the load they
/// already carried -- the shipped per-cell reading, which is the safe
/// direction rather than a weaker one.
fn is_member(world: &World, section: &[(i32, i32)]) -> bool {
    let (Some(&(x, y)), Some(lo_x), Some(hi_x), Some(lo_y), Some(hi_y)) = (
        section.first(),
        section.iter().map(|c| c.0).min(),
        section.iter().map(|c| c.0).max(),
        section.iter().map(|c| c.1).min(),
        section.iter().map(|c| c.1).max(),
    ) else {
        return false;
    };
    // Whichever coordinate moves is the run's axis. Asked of the section
    // rather than assumed, because `section_cells` returns a *vertical* run
    // for a cell whose support comes from the side, and a version of this
    // that read `x` unconditionally would have been quietly asking about
    // the wrong two neighbours the day anything widened the caller's gate.
    let (a, b) = if hi_y > lo_y { ((x, lo_y - 1), (x, hi_y + 1)) } else { ((lo_x - 1, y), (hi_x + 1, y)) };
    !is_body_material(world, world.get(a.0, a.1).material) && !is_body_material(world, world.get(b.0, b.1).material)
}

/// Whether `(x, y)` passes its load straight down -- its support is the
/// cell directly below or directly above it, or it stands on the ground.
///
/// The predicate `section_cells` runs to choose an axis, named and reused
/// so that "is this cell a column" is asked the same way in both places.
/// Whichever way it is asked, it has to agree with `section_cells`, or the
/// run and the flow crossing it would be describing different objects.
fn flows_down(world: &World, x: i32, y: i32) -> bool {
    match support_parent(world, x, y) {
        Some((px, _)) => px == x,
        None => rests_on_ground(world, x, y),
    }
}

/// Whether `(x, y)` borders literal bedrock (or the world edge, which
/// `Cell::OUT_OF_BOUNDS` reports as bedrock) — held from outside the model
/// entirely, and the **only** thing that exempts a cell from evaluation.
/// See `evaluate_within` for why this is narrower than `is_anchor`.
fn touches_bedrock(world: &World, x: i32, y: i32) -> bool {
    NEIGHBOURS_4.iter().any(|&(dx, dy)| world.get(x + dx, y + dy).material == super::material::BEDROCK)
}

/// Whether `(x, y)` is anchored outright — touching bedrock, or simply
/// sitting on the ground. This is where a support *chain* walk stops. It is
/// deliberately **not** what exempts a cell from evaluation — that is
/// `touches_bedrock` alone, and conflating the two is exactly how B2
/// shipped dead (see `evaluate_within`).
///
/// Read from the world directly rather than from `aux == 0`, deliberately.
/// A stored distance is a cache that lags a disturbance by up to a tick;
/// this is the thing the cache is *of*, and the support question below has
/// to be answerable while the cache is still catching up.
///
/// Powder counts as ground and solid does not, which looks inconsistent and
/// is the whole subtlety: solid support is already handled properly by the
/// relaxation, which asks whether the cell below can *itself* reach an
/// anchor, whereas a blob floating in mid-air rests on its own lower cells
/// and would declare every one of them grounded. A `Powder` cell carries no
/// distance to consult, so it is the one case the relaxation genuinely
/// cannot see — and treating a granular pile as ground is safe because it
/// is under the CA sweep's control: if it flows out from underneath, that
/// write dirties the chunk and whatever sat on it is re-examined.
fn is_anchor(world: &World, x: i32, y: i32) -> bool {
    touches_bedrock(world, x, y) || rests_on_ground(world, x, y)
}

/// Whether `(x, y)` is sitting on loose material that can hold its weight.
///
/// **Supported is not the same as exempt, and conflating them was a bug
/// with three separate symptoms.** Powder underneath genuinely terminates
/// a support chain -- that is what stops a chunk shattering the moment it
/// lands on its own rubble, which is the case the predicate was written
/// for. What it must *not* do is excuse the cell from the torque test,
/// because a granular pile's resistance to a bending moment is
/// approximately none.
///
/// Treating them as the same thing meant a few grains of rubble could hold
/// up anything: `scene=ligament`'s 4,400-cell slab stayed in the air
/// propped on debris wedged in its own notch, the `worked` shelf kept a
/// 1-cell skin resting on nothing much, and a sprinkle of sand under a
/// cantilever made it arbitrarily long. See `capacity`, which now charges
/// a cell supported this way for what the pile can actually carry.
fn rests_on_ground(world: &World, x: i32, y: i32) -> bool {
    world.materials.kind(world.get(x, y + 1).material) == MaterialKind::Powder
}

/// Whether the support chain from `(x, y)` reaches an anchor.
///
/// **A chain that arrives is proof, not a guess.** Every step is verified at
/// walk time to be a real, uncracked, body-material adjacency to a cell
/// strictly closer to an anchor, so arriving means a genuine load path
/// exists right now. That is what makes this usable as the cheap test even
/// though the distance field it reads is a lagging cache.
///
/// A chain that *fails* to arrive proves nothing, which is the asymmetry
/// `is_supported` below is built around.
fn chain_reaches_anchor(world: &World, x: i32, y: i32, memo: &mut AnchorMemo) -> bool {
    if let Some(&known) = memo.get(&(x, y)) {
        return known;
    }
    let mut path = Vec::new();
    let mut at = (x, y);
    let arrived = loop {
        if let Some(&known) = memo.get(&at) {
            break known;
        }
        if is_anchor(world, at.0, at.1) {
            break true;
        }
        path.push(at);
        if path.len() > MAX_SUPPORT_WALK {
            break true; // over the cap: call it supported, never the other way
        }
        match support_parent(world, at.0, at.1) {
            Some(parent) => at = parent,
            None => break false,
        }
    };
    memo.insert(at, arrived);
    for step in path {
        memo.insert(step, arrived);
    }
    arrived
}

/// Whether anything at all is holding `(x, y)` up.
///
/// # Why this is not just "walk up the parent chain"
///
/// Because a broken chain does not mean a fallen piece. The distance field
/// is a label-correcting relaxation that converges over several ticks, and
/// the instant a blow scores a fracture across a shelf, every cell whose
/// shortest path used to run through that fracture has a *stale* distance
/// and no valid parent — while being perfectly well supported by a longer
/// way round. Measured, not supposed: right after `scene=worked`'s six
/// blows, the shelf's underside read `UNSUPPORTED` with a torque well
/// inside its capacity, which as a failure criterion would have dropped
/// rock that was still holding.
///
/// The other half of the trap is why the obvious guard does not work.
/// "Only judge a cell whose stored distance is consistent with its
/// neighbours" correctly defers that case — and also defers *every*
/// genuinely detached piece forever, because a region with no anchor is
/// exactly the one whose distances climb without ever settling (the
/// count-to-infinity dynamic in `structural.rs`'s module doc). The two
/// cases are indistinguishable by consistency.
///
/// So: try the cheap proof first, and only when it fails, pay for the
/// definite answer — a bounded flood of the connected region, which does
/// not consult a stored distance for anything except as a shortcut. It
/// terminates the moment it touches any cell with an intact chain, so the
/// supported case stays cheap even against a cliff a hundred cells from
/// bedrock, and it exhausts quickly on a genuinely detached piece, which
/// is small by construction. Over the cap resolves to supported: a
/// 4,000-cell region deciding it is falling is the outcome worth being
/// paranoid about.
fn is_supported(world: &World, x: i32, y: i32, memo: &mut AnchorMemo, budget: &mut u32) -> bool {
    if chain_reaches_anchor(world, x, y, memo) {
        return true;
    }
    let mut seen: HashSet<(i32, i32)> = HashSet::from([(x, y)]);
    let mut queue = VecDeque::from([(x, y)]);
    let mut visited = 0usize;
    while let Some((cx, cy)) = queue.pop_front() {
        visited += 1;
        *budget = budget.saturating_sub(1);
        // Over the cap, or out of frame budget: never conclude "falling"
        // from an unfinished search. The caller defers instead, and the
        // budget is generous enough for a whole detached piece to be
        // searched in one frame, so deferring cannot become a livelock.
        if visited > MAX_REGION_CELLS || *budget == 0 {
            return true;
        }
        for (dx, dy) in NEIGHBOURS_4 {
            let next = (cx + dx, cy + dy);
            if seen.contains(&next) || edge_is_cracked(world, cx, cy, dx, dy) {
                continue; // support does not cross a fracture
            }
            let cell = world.get(next.0, next.1);
            if !is_body_material(world, cell.material) || cell.organism_id() != 0 {
                continue;
            }
            if chain_reaches_anchor(world, next.0, next.1, memo) {
                return true; // this piece is joined to something that is genuinely held
            }
            seen.insert(next);
            queue.push_back(next);
        }
    }
    false // the whole connected piece was explored and none of it reaches an anchor
}

/// What one cell's subtree weighs and where that weight sits: total mass,
/// `Σ mᵢ xᵢ`, and whether the walk was cut short.
/// Accumulated mass and moment in `LOAD_SCALE` fixed point, plus
/// whether the walk was cut short. Mass is `i64` rather than `u32`
/// because load is *divided* as it splits between supports, so it needs
/// the same fixed-point headroom the moment does.
pub type Subtree = (i64, i64, bool);

/// Per-frame cache of `subtree_sum` results, cleared by `scheduler::step`.
pub type SubtreeMemo = HashMap<(i32, i32), Subtree>;

/// Per-frame cache of "does this cell's support chain reach an anchor",
/// cleared alongside `SubtreeMemo`.
///
/// Shared across every check in a frame for the same reason the subtree
/// sums are: during a cascade, many cells have broken chains and each one
/// used to re-walk the same region from scratch. Sharing turns the support
/// question from O(region) per check into O(region) per frame, which is
/// what makes it affordable to search a large detached piece to the end
/// instead of capping the search and guessing.
pub type AnchorMemo = HashMap<(i32, i32), bool>;

/// Everything `load` caches for the span of one frame. Held on `World` and
/// handed to the walks as one borrow.
#[derive(Default)]
pub struct Cache {
    pub subtrees: SubtreeMemo,
    pub anchors: AnchorMemo,
    pub cuts: CutMemo,
}

impl Cache {
    pub fn clear(&mut self) {
        self.subtrees.clear();
        self.anchors.clear();
        self.cuts.clear();
    }
}

/// The worst subtree crossing one horizontal cut, keyed by the cut itself.
///
/// Every cell of a cut computes the same answer -- that is the entire point
/// of judging the section rather than the cell -- so a seventeen-deep wall
/// was walking seventeen cells seventeen times. Measured before this went
/// in: `scene=room wall=8 dig=1` at **55.89 ms** a frame against 27.71 with
/// the rule off, and `scene=worldcrack strike=12` at 26.64 against 14.54.
/// Roughly a doubling on exactly the scenes the model exists for, which
/// `CLAUDE.md` calls a hard constraint rather than a tiebreaker.
///
/// Keyed on `(min_x, max_x, y)` and not on the starting cell, because the
/// run is what decides the answer. It has to be the *span* rather than one
/// end: `section_cells` grows outward from wherever it was asked and stops
/// at `MAX_SECTION`, so two cells far apart in a long run see different
/// forty-cell windows and genuinely must not share an entry.
pub type CutMemo = HashMap<(i32, i32, i32), Subtree>;

/// How many neighbours are genuinely holding `(x, y)` up: those strictly
/// closer to an anchor, across an uncracked edge.
///
/// **A cell can have more than one, and that is the whole point.** The
/// `support_parent` used for walking chains picks a single best one, which
/// is right for "which way is downhill" and wrong for "who is carrying
/// this" -- a slab in the middle of a bridge span is held by both legs,
/// not by whichever the tie-break happened to prefer.
fn support_count(world: &World, x: i32, y: i32) -> i64 {
    let own = world.get(x, y).aux();
    if own == 0 {
        return 0; // an anchor is held from outside the model
    }
    NEIGHBOURS_4
        .iter()
        .filter(|&&(dx, dy)| {
            if edge_is_cracked(world, x, y, dx, dy) {
                return false;
            }
            let n = world.get(x + dx, y + dy);
            is_body_material(world, n.material) && n.organism_id() == 0 && n.aux() < own
        })
        .count() as i64
}

/// The neighbours that lean on `(x, y)` -- those strictly further from an
/// anchor, across an uncracked edge.
///
/// # Why this is "further away" and not "whose parent is me"
///
/// It used to be the latter, and that was the single biggest defect in
/// this model. `support_parent` is a *function*, so parent-based flooding
/// builds a spanning **tree** -- and where a structure offers several
/// routes to the ground, a tree picks exactly one and sends the entire
/// load down it. Reported from play, and visible in the stress view as a
/// **one-pixel red line** through an otherwise green building: a table's
/// whole span loaded its left leg while the right leg carried nothing.
///
/// So load spreads over every route that exists, which is what real
/// structures do. Each cell divides what it carries among its own supports
/// (`support_count`), so a cell reached two ways contributes half to each.
/// That is a flow over a DAG rather than a walk over a tree.
///
/// **This is not the double-counting pitfall it resembles.** Flooding to
/// greater-distance neighbours and summing *whole* subtrees really does
/// count a subtree once per path, which is the trap the parent-based
/// version was written to avoid. Dividing each cell's load by the number
/// of supports it has is exactly what makes the total conserved again:
/// what leaves a cell is what it carries, however many ways it splits.
fn dependants(world: &World, x: i32, y: i32, out: &mut Vec<(i32, i32)>) {
    out.clear();
    let own = world.get(x, y).aux();
    for (dx, dy) in NEIGHBOURS_4 {
        if edge_is_cracked(world, x, y, dx, dy) {
            continue; // a fracture carries no load, so nothing across one leans on us
        }
        let (nx, ny) = (x + dx, y + dy);
        let cell = world.get(nx, ny);
        if !is_body_material(world, cell.material) || cell.organism_id() != 0 {
            continue;
        }
        if cell.aux() > own {
            out.push((nx, ny));
        }
    }
}

/// Fixed-point scale for accumulated mass and moment.
///
/// Load is divided as it splits between supports, and integer division
/// truncates -- down a long chain that would quietly leak mass. One cell
/// weighs `LOAD_SCALE`, so a split four ways still lands on a whole
/// number. `i64` has room to spare: 20,000 cells at x~500 is ~4e11 against
/// a ceiling of 9e18.
const LOAD_SCALE: i64 = 4096;

/// Total mass and moment of everything `(x, y)` supports, including itself.
///
/// # Why this is memoized, and why that is the difference between
/// playable and not
///
/// A cell's subtree sum is its own mass plus its children's sums, so the
/// obvious implementation floods the whole subtree per cell. That is
/// quadratic in the height of a structure and it does not survive contact
/// with a real one: measured at **6,556 ms in a single frame** on
/// `scene=capped` (a 60x192 column with an overhanging cap) and 4,456 ms on
/// `scene=strike`, because a disturbance sends a wavefront of distance
/// changes through every cell of the structure and each one re-walked
/// thousands of cells that the cell beside it had just walked.
///
/// Sharing the results across one frame collapses that to O(cells) total:
/// the first walk up a column memoizes every intermediate, and the
/// thousands of checks behind it hit the cache. `scheduler::step` clears
/// the memo each frame, and `structural::tick` clears it again the moment a
/// break mutates the grid, because both invalidate the forest this
/// describes.
///
/// Iterative rather than recursive on purpose — a support chain is as deep
/// as a structure is tall, and a 200-cell column is a 200-deep recursion.
fn subtree_sum(world: &World, x: i32, y: i32, memo: &mut SubtreeMemo, budget: &mut u32) -> Subtree {
    if let Some(&known) = memo.get(&(x, y)) {
        return known;
    }
    // A post-order walk with an explicit stack. `Enter` discovers what
    // leans on a cell; `Exit` sums their shares once they are in the memo.
    // The list is carried on the `Exit` entry rather than recomputed,
    // because finding it costs hashed `World::get` calls and doing that
    // twice per cell doubles the cost of the hottest loop in this module.
    enum Step {
        Enter((i32, i32)),
        Exit((i32, i32), [(i32, i32); 4], usize),
    }
    let mut kids = Vec::new();
    let mut stack = vec![Step::Enter((x, y))];
    // Cells actually walked, not `stack.len()` -- different quantities, and
    // testing the wrong one meant this cap and the identically-named one in
    // `supported_subtree` bounded different things while claiming to bound
    // the same one.
    let mut walked = 0usize;
    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(node) => {
                if memo.contains_key(&node) {
                    continue;
                }
                if *budget == 0 || walked >= MAX_SUBTREE_CELLS {
                    // Out of room. Recorded as carrying only itself and
                    // flagged truncated, which **under**states the load --
                    // the safe direction, since the failure mode of
                    // overstating it is terrain deciding it is falling.
                    memo.insert(node, (LOAD_SCALE, LOAD_SCALE * node.0 as i64, true));
                    continue;
                }
                *budget = budget.saturating_sub(1);
                walked += 1;
                dependants(world, node.0, node.1, &mut kids);
                let mut held = [(0, 0); 4];
                for (slot, &kid) in held.iter_mut().zip(kids.iter()) {
                    *slot = kid;
                }
                stack.push(Step::Exit(node, held, kids.len()));
                for &kid in &kids {
                    if !memo.contains_key(&kid) {
                        stack.push(Step::Enter(kid));
                    }
                }
            }
            Step::Exit(node, held, count) => {
                let mut mass = LOAD_SCALE;
                let mut moment = LOAD_SCALE * node.0 as i64;
                let mut truncated = false;
                for &kid in &held[..count] {
                    let (m, mo, t) = memo.get(&kid).copied().unwrap_or((LOAD_SCALE, LOAD_SCALE * kid.0 as i64, true));
                    // Each dependant divides what it carries among *its*
                    // supports and we receive one share. This is what keeps
                    // the total conserved while letting load spread over
                    // every route that exists.
                    let share = support_count(world, kid.0, kid.1).max(1);
                    mass = mass.saturating_add(m / share);
                    moment = moment.saturating_add(mo / share);
                    truncated |= t;
                }
                memo.insert(node, (mass, moment, truncated));
            }
        }
    }
    memo.get(&(x, y)).copied().unwrap_or((LOAD_SCALE, LOAD_SCALE * x as i64, true))
}

/// Every cell whose support chain passes through `(x, y)`, including it.
///
/// The cell *list*, as opposed to `subtree_sum`'s totals — needed only once
/// a cell has already been judged to fail, so it is worth walking properly
/// rather than caching. See `children` for why this cannot double-count.
///
/// Flooded by asking each candidate *who its parent is* and keeping only
/// those that answer `(x, y)` — not by walking to neighbours with a greater
/// distance, which double-counts on equal-cost paths. See the module doc.
fn supported_subtree(world: &World, x: i32, y: i32, budget: &mut u32) -> (Vec<(i32, i32)>, bool) {
    let mut out = Vec::new();
    let mut seen: HashSet<(i32, i32)> = HashSet::from([(x, y)]);
    let mut queue = VecDeque::from([(x, y)]);
    let mut truncated = false;
    while let Some((cx, cy)) = queue.pop_front() {
        out.push((cx, cy));
        *budget = budget.saturating_sub(1);
        if out.len() >= MAX_SUBTREE_CELLS || *budget == 0 {
            truncated = true;
            break;
        }
        for (dx, dy) in NEIGHBOURS_4 {
            let next = (cx + dx, cy + dy);
            if seen.contains(&next) {
                continue;
            }
            if edge_is_cracked(world, cx, cy, dx, dy) {
                continue; // a fracture carries no load, so nothing across one is ours
            }
            let cell = world.get(next.0, next.1);
            if !is_body_material(world, cell.material) || cell.organism_id() != 0 {
                continue;
            }
            if support_parent(world, next.0, next.1) == Some((cx, cy)) {
                seen.insert(next);
                queue.push_back(next);
            }
        }
    }
    (out, truncated)
}

/// The connected region around `(x, y)` that cannot reach an anchor.
///
/// The *unsupported* failure's piece. Ordinary 4-connectivity, stopped by
/// fractures and by any cell that is still genuinely held — so a mined roof
/// still attached at both ends gives up only the part that has actually
/// come free, while a cut bridge span or a blob painted in mid-air comes
/// away whole rather than dissolving a cell at a time.
fn detached_piece(world: &World, x: i32, y: i32, memo: &mut AnchorMemo, budget: &mut u32) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    let mut seen: HashSet<(i32, i32)> = HashSet::from([(x, y)]);
    let mut queue = VecDeque::from([(x, y)]);
    while let Some((cx, cy)) = queue.pop_front() {
        out.push((cx, cy));
        if out.len() >= MAX_REGION_CELLS {
            break;
        }
        for (dx, dy) in NEIGHBOURS_4 {
            let next = (cx + dx, cy + dy);
            if seen.contains(&next) || edge_is_cracked(world, cx, cy, dx, dy) {
                continue;
            }
            let cell = world.get(next.0, next.1);
            if !is_body_material(world, cell.material) || cell.organism_id() != 0 {
                continue;
            }
            seen.insert(next);
            if !is_supported(world, next.0, next.1, memo, budget) {
                queue.push_back(next);
            }
        }
    }
    // Sorted so the fragment seed order below is identical run to run -- a
    // `HashSet`-driven flood is not, which is issue #7's determinism trap
    // in a new place.
    out.sort_unstable();
    out
}

/// How far the section runs perpendicular to where the support is coming
/// from, in cells.
///
/// Bending capacity grows with the **square** of section depth, which is
/// why a joist is deep rather than wide, and it is the dominant term in a
/// real beam: a one-cell ledge and a forty-cell-thick cap are not remotely
/// the same structure. Reported from play before this existed as a tall
/// thick column with "a shallow overhang ... crumbles by itself".
///
/// # Why the direction depends on the parent
///
/// The old `depth_factor` always measured *vertically*, on the reasoning
/// that a horizontal overhang is the case that fails and vertical is
/// perpendicular to its span. That is right for a beam and wrong in
/// general: the section that resists a moment is the one perpendicular to
/// the path the load is travelling. For a cell supported from beside, that
/// is still vertical; for one supported from below — a column carrying an
/// eccentric cap — it is *horizontal*, which is why a thin tall wall cannot
/// hold something hanging off its side however tall it is. Generalizing to
/// "perpendicular to the parent direction" agrees with the old rule on
/// every case it was written for.
///
/// # Why this returns the cells and not just a depth
///
/// Two callers need the membership, not the count: `failing_region` breaks
/// the whole section off together, and `capacity` reads how much of the
/// section is still attached. The count-only wrapper that used to sit here
/// was deleted when the second of those arrived — one walk, one meaning.
///
/// Ordered outward from `(x, y)` and capped at `MAX_SECTION`.
fn section_cells(world: &World, x: i32, y: i32, parent: Option<(i32, i32)>) -> Vec<(i32, i32)> {
    // Support from below/above travels vertically, so the resisting section
    // is horizontal, and vice versa.
    //
    // A cell with *no* parent is either an anchor or mid-convergence, and
    // those two want opposite readings. An anchor resting on ground is
    // supported from directly below: the load path is vertical and the
    // resisting section is horizontal, exactly as it would be if the cell
    // beneath were rock and had become its parent. Reading it vertically
    // instead measured the section *down the hole it stands beside* and
    // returned 1 to 3 cells -- which is where the dig cascade's
    // `capacity 2 / 8 / 18` came from. The identical slab cell scored up to
    // 1600x weaker for being propped on its own rubble rather than on
    // stone, so a crater rim ate itself one course at a time. Measured on
    // `scene=worldcrack preset=flat seed=7`: 23,042 cells lost to one
    // radius-6 hole in dead-level homogeneous rock.
    //
    // Mid-convergence keeps the old vertical reading -- it has no support
    // direction yet to be perpendicular to.
    let vertical_path = match parent {
        Some((px, _)) => px == x,
        None => rests_on_ground(world, x, y),
    };
    let (sx, sy) = if vertical_path { (1, 0) } else { (0, 1) };
    let mut out = vec![(x, y)];
    for dir in [-1, 1] {
        let mut step = 1;
        while (out.len() as i64) < MAX_SECTION {
            let (px, py) = (x + dir * sx * step, y + dir * sy * step);
            let probe = world.get(px, py);
            if !is_body_material(world, probe.material) || probe.material == super::material::EMPTY {
                break;
            }
            out.push((px, py));
            step += 1;
        }
    }
    out
}

/// How much of `(x, y)`'s cross-section a fracture has taken away, as a
/// numerator over `CRACK_FACES`.
///
/// Counts cracked edges, not *intact* ones, and the distinction matters: a
/// cell at a free rock face has fewer neighbours than one buried in a slab,
/// and being at a surface is not damage. Only a fracture is. Floored at 1 so
/// a heavily scored cell is very weak rather than instantly gone — the point
/// is a piece that sags and then fails, not one that vanishes the moment it
/// is touched. All four edges cracked needs no handling: the relaxation
/// simply finds no path through the cell at all.
fn uncracked_faces(world: &World, x: i32, y: i32) -> i64 {
    let cracked = NEIGHBOURS_4.iter().filter(|&&(dx, dy)| edge_is_cracked(world, x, y, dx, dy)).count() as i64;
    (CRACK_FACES - cracked).max(1)
}

/// What `(x, y)` can carry.
///
/// ```text
/// capacity = base × section² × attachment × uncracked_fraction
/// ```
///
/// # Where `base` comes from, and what happened to `max_unsupported_span`
///
/// `max_unsupported_span` is **reinterpreted, not deleted**: it is now the
/// reach at which a *one-cell-deep, unattached* beam of this material gives
/// way, which is what it always described in practice and is the only
/// reading under which its shipped values still mean something. The root
/// moment of a unit-mass beam of length `L` and depth `D` is about
/// `D·L²/2`, so at `D = 1` that reach is reached when the base equals
/// `span²/2` — stone's 16 gives 128, wood's 8 gives 32, and both keep
/// behaving the way their `.ron` comments say they do.
///
/// Leaving it half-live was the alternative and is explicitly the trap:
/// when `confinement_radius` was superseded its tests kept passing while
/// testing nothing, because an undisturbed slab sits at a self-consistent
/// distance and stops rescheduling. One meaning, stated here, or none.
/// The moment arm a roof owes for the opening beneath it, or `None` if
/// this cell is not a roof.
///
/// # The problem, which is geometric and not a bug
///
/// A real mine gallery's roof spans its **width** -- a few metres --
/// however many kilometres long the drive is, because the rock either side
/// carries it along the whole length. This engine is a 2D side view, and
/// in a side view there is no "either side": a horizontal bore is a slot,
/// and its roof genuinely spans the entire excavation. So the model was
/// not wrong about the span it saw; the span it saw was real. Measured, a
/// 140-cell corridor under 8 cells of cover collapsed at every bore size,
/// and the owner's expectation -- *"a super short tunnel should be able to
/// have a longer span. ant tunnel vs digging a mine"* -- could not be met
/// by any tuning, because nothing in the model knew how big the hole was.
///
/// # Terzaghi's rock load, and the owner's simplification of it
///
/// Rock over an opening arches: it does not all bear on the roof, but
/// carries most of its weight sideways into the ground either side, and
/// the arch that forms is set by the *size of the opening* -- `k(B + Ht)`
/// for width `B` and height `Ht` -- not by depth and not by how far the
/// drive has been pushed.
///
/// `B` is out of plane and a side view cannot see it. **The owner's
/// suggestion is what makes this affordable: assume the opening is as wide
/// as it is tall**, so `B = Ht` and the span follows from height alone.
/// Height is the one dimension a side view does have, and it is a short
/// bounded scan downward -- where measuring `B` would mean walking the
/// whole length of the tunnel for every cell of its roof, which is exactly
/// the per-cell-per-frame cost `is_structurally_interesting` exists to
/// avoid.
///
/// # Why this is the arm and not the mass
///
/// Reducing the *load* was tried first and did almost nothing, which only
/// the A/B switch showed: with 8 cells of cover a roof cell's mass is
/// already below anything the arch would cap it at, so `min(mass, Hp)`
/// never bit -- bore 9 measured 30% and 39% of its cave surviving with the
/// relief on and 30% and 40% with it off. The load was never the binding
/// constraint at these depths; **the span is**, through `torque = M x arm`
/// where the arm reaches the centroid of everything carried, which for a
/// tunnel roof is halfway down the drive. So the arch belongs where the
/// two clamps above it already live: on the arm.
///
/// # Why it is safe where it is not wanted
///
/// A `min`, never a bonus, so it can only ever reduce a torque that was
/// already computed. And it requires **rock directly overhead**: an
/// overhang's tip has sky above it and nothing to form an arch out of, so
/// `scene=undercut`'s spalling and `scene=ligament`'s neck are judged
/// exactly as before.
fn arch_span(world: &World, x: i32, y: i32) -> Option<i64> {
    if !world.arch_relief {
        return None;
    }
    // Something to arch *out of*. Without cover there is no arch, only a
    // cantilever, and a cantilever is what the unclamped arm already is.
    if !is_body_material(world, world.get(x, y - 1).material) {
        return None;
    }
    // Something to arch *over*, which may be some way below: the whole
    // roof beam arches, not just the single course of cells forming the
    // ceiling. Testing only the cell immediately beneath was tried and was
    // far too narrow to matter -- the clamp then reached one row while the
    // overload failures were happening in the rock above it, and the A/B
    // came back identical on two of three bore sizes.
    let mut cover = 0u32;
    while world.get(x, y + 1 + cover as i32).material != super::material::EMPTY {
        cover += 1;
        if cover > MAX_COVER_PROBE || !world.in_bounds(x, y + 1 + cover as i32) {
            return None; // solid all the way down; no opening to arch over
        }
    }
    let mouth = y + 1 + cover as i32;
    let mut height = 0u32;
    while height < MAX_OPENING_PROBE {
        let below = mouth + height as i32;
        if !world.in_bounds(x, below) || world.get(x, below).material != super::material::EMPTY {
            break;
        }
        height += 1;
    }
    // `k(B + Ht)` with `B = Ht`, halved because a moment arm is measured
    // from the centre of a span rather than across it.
    Some((ARCH_LOAD_K * height.max(1) as i64).max(1))
}

/// Terzaghi's `k`, relating the arch a roof forms to the size of the
/// opening under it.
///
/// His own table runs from 0 for intact rock to about 1.5(B + Ht) for
/// heavily broken ground. 1 is the moderately-jointed middle and a place
/// to sweep from, not a derived value: raising it lets a roof own a longer
/// arm, so tunnels collapse sooner.
const ARCH_LOAD_K: i64 = 4;

/// How far below itself a cell will look for the opening it roofs. Bounded
/// because this runs per evaluated cell: `is_structurally_interesting`
/// already keeps it off the interior of a massif, and past this depth of
/// cover a roof is not what is holding anything up anyway.
const MAX_COVER_PROBE: u32 = 8;

/// How deep the opening probe looks. Past this the hole is a cavern and
/// the distinction has stopped mattering -- the arm is already longer than
/// anything that changes an outcome.
const MAX_OPENING_PROBE: u32 = 16;

pub fn capacity(world: &World, x: i32, y: i32) -> i64 {
    let parent = support_parent(world, x, y);
    let section = section_cells(world, x, y, parent);
    capacity_within(world, x, y, parent, &section)
}

/// `capacity`, reusing a parent and section the caller has already walked.
///
/// Not an optimisation for its own sake: `evaluate_within` needs both to
/// decide which object is on trial, and walking a forty-cell run twice per
/// evaluated cell is a `World::get` per cell per frame in the hottest loop
/// this module has. Measured on `scene=worldcrack strike=12`, splitting
/// this out took the frame back from 21.85 ms to 14.44 against a 12.96 ms
/// baseline.
fn capacity_within(world: &World, x: i32, y: i32, parent: Option<(i32, i32)>, section_cells: &[(i32, i32)]) -> i64 {
    let cell = world.get(x, y);
    let m = world.materials.get(cell.material);
    if m.max_unsupported_span == u16::MAX {
        return i64::MAX; // this material does not participate in the structural system at all
    }
    let base = (m.max_unsupported_span as i64).pow(2) / 2;
    let section = section_cells.len() as i64;
    // Attachment buys *capacity*, never immunity. Anchoring on it made an
    // undercut shelf unfallable however much was dug from beneath it --
    // model 3 of the four this replaces.
    //
    // # Keying: the bonus belongs to the *joint*, not to the cell
    //
    // Reported from play: "we should be able to attach foreground objects
    // to background objects." The load path already crosses that interface
    // -- a beam built against a cliff takes its support from the cliff
    // like anything else -- but the *capacity* at the joint did not, so
    // the first foreground cell was as weak as if it were floating, and
    // built structures tore off exactly where they met the terrain.
    //
    // So a cell is keyed when the neighbour actually holding it up is
    // background, whether or not it is itself. **This cannot chain**, and
    // that is the whole reason it is safe: the bonus is a property of an
    // *edge*, so painting A against a cliff does not turn A into a source
    // of it for B. The obvious alternative -- "adjacent to attached
    // becomes attached" -- spreads without limit, and since the floor is
    // attached, everything built on the ground would inherit it. That is
    // how all four earlier support models made player structures
    // indestructible.
    //
    // The "but the floor is attached" objection resolves here in the right
    // direction rather than being dodged: a tower's *base* is keyed to the
    // ground, and nothing above it is. Foundations are exactly where a
    // keyed joint belongs.
    //
    // # Tried and reverted: averaging the bonus over the section
    //
    // The bonus is a **cliff** -- one flag, twelvefold, on or off -- and
    // the owner predicted the consequence before it was seen: *"we are
    // deciding structures are solid when built... then any little damage
    // turns off the protection, the whole structure will just collapse."*
    // Measured, on `scene=room`: a radius-1 chisel cut into a 17-cell-thick
    // wall brought down 2,516 cells of a room that stood untouched at 15%
    // of capacity, because `mine`'s detach band reaches clean across a wall
    // that thin.
    //
    // Grading it -- attachment as the *mean* over the section, so three
    // loosened cells out of seventeen cost three seventeenths of the bonus
    // -- is the obvious graded-beats-binary fix and it **breaks
    // `scene=undercut`**, which went to zero failures. Undercut spalls
    // precisely because the rows a dig loosened are weak while the rows
    // above them are not: at a section of 6 with 3 rows loosened, the mean
    // reads 6.5x where the old rule read 1x, and the shelf holds. Weakness
    // being *per cell* is the spalling mechanism, not an artifact of it.
    //
    // It also did not help the case it was written for: at the cut, every
    // cell of the wall's cross-section is loosened, so the mean equals the
    // minimum and nothing changes. Wrong on both counts, and recorded here
    // so the next reader does not spend the afternoon on it. The cliff is
    // real; see `Reports/next-session-handoff.md` for where it actually
    // comes from.
    let keyed = cell.attached() || parent.is_some_and(|(px, py)| world.get(px, py).attached());
    let attachment = if keyed { m.attached_span_bonus as i64 } else { 1 };
    let capacity = base.saturating_mul(section.pow(2)).saturating_mul(attachment).saturating_mul(uncracked_faces(world, x, y)) / CRACK_FACES;
    // A cell whose support is a pile of loose grains is *held*, but it is
    // not braced: sand resists compression and essentially no bending. So
    // resting on powder keeps the chain alive (see `rests_on_ground`) and
    // costs almost all of the section's capacity, which is what stops a
    // handful of rubble propping up a slab.
    //
    // **Tried and reverted: gating this on `parent.is_none()`**, so that
    // only a cell with no solid support at all takes the granular cut. The
    // reasoning was good and the measurement said no: not one cell moved,
    // across six presets and three seeds. `scene=worldcrack preset=flat`
    // stayed at 599/637/338 cells lost to one hole, to the cell.
    //
    // Why it changed nothing is the useful part, and it redirects the
    // search. The cells taking the cut read `capacity 2`, `8`, `18` --
    // `base` over 64 at sections of one, two and three -- with
    // `attachment 1`, and `attachment` is 1 exactly when neither the cell
    // nor its parent is attached. They have **no solid parent to begin
    // with**: they are already loose fragments at the rubble face, not slab
    // rock that a grain happened to undercut. So the divisor is a symptom
    // of their state, not the cause of it, and the question worth asking is
    // why a homogeneous slab produces six hundred parentless section-1
    // cells around one hole at all.
    // **The answer to that last question, found later: because ground rooted
    // eagerly.** `structural::tick` gave any cell with powder under it a
    // distance of 0, so it had no parent *by construction* -- which is why
    // this gate moved nothing when it was first tried. It was not wrong, it
    // was premature: the condition it tests was universally true. With
    // ground demoted to a last-resort root (see `tick`), a slab cell over
    // its own rubble keeps its lateral parent, and the gate now separates
    // the two cases it was always meant to separate: rock that is *spanning*
    // and happens to touch grain, from rock the pile is genuinely all that
    // holds up. Only the second is charged a pile's resistance to bending,
    // which is about none.
    // The granular case is no longer a division here. What a pile can do is a
    // property of the *load* standing on it, not of the rock's own section,
    // so it is applied in `evaluate_within` where the mass is known -- see
    // `bearing_moment`.
    capacity
}

/// The moment a piece resting on loose ground can resist before it tips.
///
/// **A pile pushes; it cannot pull.** That single restriction is the whole
/// model, and it is the standard no-tension (Winkler) bed: the ground's
/// reaction may shift anywhere within the contact width to balance the load,
/// but it can never become tension to hold an edge down. So the resultant
/// has to stay inside the base, and once it leaves the middle third the far
/// edge lifts and the piece rotates instead of resisting. That is the same
/// kern argument the vertical-arm clamp above already runs on a column, which
/// is why this belongs in the model rather than beside it.
///
/// The result is `N * B / 6` — and since `torque` is `N * eccentricity`, the
/// criterion reduces to *is the centre of mass within a sixth of the contact
/// width*, which is mass-independent and therefore scale-free. A chunk lying
/// flat on rubble holds however heavy it is; a shelf reaching out past its
/// own footing tips however light it is.
///
/// **This replaces `GRANULAR_CAPACITY_DIVISOR`**, a flat 64x cut that was
/// really a counterweight for `structural::tick` rooting eagerly on powder
/// (see the note there). With the rooting fixed, the divisor had nothing left
/// to cancel and became pure overcorrection: `scene=roomcut` fell to 2
/// overload failures against its bar of 5.
fn bearing_moment(world: &World, x: i32, y: i32, mass: u32) -> i64 {
    // The contact width is the *piece's* footing, not this one cell's.
    //
    // Counting only the cells whose own underside touches grain was tried
    // first and is wrong twice over. A pile is lumpy, so a slab lying on its
    // own rubble touches it in patches and read as several knife-edge
    // footings a cell or two wide -- and a footing that narrow tips at any
    // eccentricity past a sixth of a cell, so the slab was dismantled one
    // cell at a time. It also ignores that a pile *conforms*: grain under a
    // load punches and settles until the contact spreads, which this engine
    // does anyway a few frames later, so judging the instantaneous patchy
    // contact judges a state that is about to stop existing.
    //
    // So the footing is the run of rock at this row that has *anything*
    // beneath it, grain or stone -- which is the width the piece actually
    // stands on once the grain has settled. Measured on
    // `scene=worldcrack preset=flat`, the patchy reading cost 48,109 cells to
    // one dig against 894 for the bug it was meant to fix.
    let mut base: i64 = 1;
    for dir in [-1, 1] {
        let mut step = 1;
        while base < MAX_SECTION {
            let (px, py) = (x + dir * step, y);
            if !is_body_material(world, world.get(px, py).material) {
                break;
            }
            if world.get(px, py + 1).material == super::material::EMPTY {
                break; // this much of the run is bridging, not standing
            }
            base += 1;
            step += 1;
        }
    }
    (mass as i64 * base / KERN_DENOMINATOR).max(1)
}

/// What `(x, y)` carries and what it can carry, or `None` if it is not a
/// cell this model has anything to say about — not body material, organism-
/// owned (those route through `structural::organism_structural_tick`'s own
/// BFS), an anchor, or attached bulk that the early-out skips.
pub fn evaluate(world: &World, x: i32, y: i32) -> Option<Load> {
    // Unbudgeted and uncached: this form is for readouts (the hover
    // inspector, `filmstrip`'s `load=` probe), which ask about one cell and
    // must give the same answer however busy the frame was.
    let mut unlimited = u32::MAX;
    let mut cache = Cache::default();
    evaluate_within(world, x, y, &mut cache, &mut unlimited)
}

/// `evaluate`, but sharing a caller-owned cache across many cells.
///
/// The plain `evaluate` builds a fresh `Cache` per call, which is right
/// for a single readout and quadratic for a whole screen of them: each
/// cell would re-walk subtrees its neighbour had just walked. A screen-wide
/// pass (`App::draw_stress_overlay`) hands one cache to every cell and
/// pays O(region) instead of O(region x subtree).
pub fn evaluate_with_cache(world: &World, x: i32, y: i32, cache: &mut Cache, budget: &mut u32) -> Option<Load> {
    evaluate_within(world, x, y, cache, budget)
}

fn evaluate_within(world: &World, x: i32, y: i32, cache: &mut Cache, budget: &mut u32) -> Option<Load> {
    let cell = world.get(x, y);
    if !is_body_material(world, cell.material) || cell.organism_id() != 0 {
        return None;
    }
    if cell.material == super::material::BEDROCK {
        return None; // the anchor material itself, and never a thing that falls
    }
    // Asked of the *world*, not of the cached distance. `aux == 0` was the
    // test here, and it is not the same claim: everything a player paints
    // starts at `aux == 0`, and `rigid::settle` deliberately writes 0 into
    // every cell of a landed body. So both read as "anchored" and became
    // permanently immune to every failure mode -- which is the binary
    // immunity all four earlier support models were rejected for, moved
    // out of the criterion where it was visible and into a guard where it
    // was not. Measured before this changed: `scene=capped` evaluated 2
    // cells out of 15,840 across 600 frames.
    //
    // Bedrock adjacency only -- deliberately *not* `is_anchor`. The other
    // half of that predicate, resting on powder, terminates a support
    // *chain* (the piece is held, not falling) but must still be
    // torque-tested against the sliver of capacity a granular prop
    // actually provides. Routing this guard through `is_anchor` made
    // `capacity`'s `GRANULAR_CAPACITY_DIVISOR` branch unreachable -- B2 of
    // `Reports/destruction-plan.md` shipped dead by its own commit, and a
    // few grains of rubble were still a full anchor for anything above
    // them. §5.7's vacuous-pass shape, caught by review rather than test.
    if touches_bedrock(world, x, y) {
        return None; // genuinely held from outside the model
    }
    if !is_structurally_interesting(world, x, y) {
        return None;
    }

    let supported = is_supported(world, x, y, &mut cache.anchors, budget);
    let parent = support_parent(world, x, y);
    // The same predicate `section_cells` runs, hoisted because three things
    // below need to know it and it decides which *object* is on trial.
    let vertical_path = flows_down(world, x, y);
    let section = section_cells(world, x, y, parent);
    let (mut mass_fp, mut moment_fp, mut truncated) = subtree_sum(world, x, y, &mut cache.subtrees, budget);
    // **On a column, the thing under trial is the section, not the cell --
    // and until this line the demand was per cell while the capacity was
    // per section.** That mismatch is the one-pixel red line the owner
    // reported three times, and the note below has said so since it was
    // found. Measured on `scene=room wall=8 dig=0`, straight across one
    // 17-cell wall at y=200:
    //
    // ```
    // (132,200)  mass  157   the outer face
    // (136..144) not evaluated -- buried, so `is_structurally_interesting`
    //                            never asks
    // (148,200)  mass 3058   the inner face
    // ```
    //
    // Nineteen to one, and thirteen of the seventeen cells hold no opinion
    // at all. Every one of them was then compared against `base x 17^2` --
    // the bending capacity of the *whole* wall. So the inner face was
    // judged roughly right by accident (it happened to carry nearly
    // everything), the outer face was judged free, and the room stood
    // because of the accident rather than because of the model. Cut the
    // inner face and the building goes; cut anywhere else in the same wall
    // and nothing happens, which is exactly what was reported.
    //
    // `section_cells` on a vertical path is a horizontal run: a **cut**
    // across the member. So every cell of the cut is judged at the cut's
    // worst, which is only saying in the criterion what `failing_region`
    // already says about the outcome three screens down -- *the section
    // fails, not the cell*. A wall does not part one lamina at a time, so
    // no lamina of it gets to be judged as though the rest were not there.
    //
    // # Why the worst and not the sum
    //
    // Summing is the first thing to reach for -- the flow through a cut is
    // the sum of what crosses it, and `subtree_sum` has already divided
    // every hand-off by `support_count` so the shares do add up without
    // double-counting. It was built, measured and withdrawn, and the
    // measurement is worth keeping because it is not a tuning failure.
    //
    // A sum changes what a thick member is *worth*, not just how the load
    // inside it is arranged. Demand `N x D/2` against capacity `base x D^2`
    // makes a column's allowable load `2 x base x D` -- linear in its
    // width, which is the honest answer for something in compression, and
    // the model has been quadratic in it since it was written. Every
    // constant in the file is calibrated against the quadratic. Measured on
    // the acceptance suite: `scene=worked` went from 7 overload failures to
    // **918** and lost 1,351 rock against 21, and `caveshallow` went 214 to
    // 2,260. Not because the arrangement was wrong but because a
    // seventeen-deep wall stopped being seventeen times stronger than the
    // arithmetic elsewhere assumes.
    //
    // Restoring that would mean lifting `base`, and that is on the
    // do-not-retry list for a reason of its own: it takes `scene=undercut`
    // to zero spalling. So the quadratic is a real and separate defect,
    // recorded in the handoff, and **not** something to smuggle in behind a
    // distribution fix. The worst-of-the-cut is a pure redistribution: the
    // aggregate strength of every structure in the world is exactly what it
    // was, and only *where* the model is willing to fail has changed.
    //
    // # Only the cells of the run that are themselves columns
    //
    // A horizontal run is a genuine cut only where the flow crossing it is
    // vertical. Where it reaches out of a wall and into the roof it lies
    // **along** the flow instead of across it, and a roof cell beside a
    // wall carries the whole roof.
    //
    // On the summing version this was catastrophic and obvious --
    // `mass 62,391` at the top of a room built from about 14,000 cells,
    // and a number larger than the world it came from needs no argument.
    // Under a max it is neither, which is worth stating plainly because it
    // decides whether the filter is worth its cost: taking a maximum
    // cannot double-count, so no hand-built scene here shows any
    // difference at all, and the acceptance suite moves by a few failures
    // either way. It survives on the seed sweep and only there.
    // `seedsweep.sh strike=12` run to rest, 24 runs, filter off against
    // filter on: rock destroyed **max 4,357 against 2,433**, cells lost max
    // 1,392 against 847, with p90 a tie inside the noise (1,753 / 1,782).
    // Nearly half the worst case, for about 33% of the frame on
    // `scene=worldcrack strike=12` (22.72 ms against 15.10).
    //
    // Do not read a single cascade scene to re-decide this. Two runs that
    // diverge on one frame are different worlds by the next, and comparing
    // their totals at frame 1,202 reported the filter as ten times *worse*
    // on one scene while the order statistics over 24 runs said the
    // opposite.
    //
    // Nothing is lost by leaving those cells out: a cell whose load travels
    // sideways is counted at whichever column it eventually turns down
    // into.
    // `vertical_path` is an **early-out, not the correctness gate** -- the
    // `flows_down` filter inside the loop already contributes nothing from
    // a cell whose load travels sideways, so a shelf comes out identical
    // either way. It is here because it is one comparison against a walk of
    // the run, on every evaluated cell of every frame.
    if vertical_path && world.section_share && is_member(world, &section) {
        let lo = section.iter().map(|c| c.0).min().unwrap_or(x);
        let hi = section.iter().map(|c| c.0).max().unwrap_or(x);
        let worst = match cache.cuts.get(&(lo, hi, y)) {
            Some(&known) => known,
            None => {
                // Seeded from this cell, so its own subtree is already in
                // the running -- which is why it is skipped inside the loop.
                let mut worst = (mass_fp, moment_fp, truncated);
                for &(sx, sy) in &section {
                    if (sx, sy) == (x, y) || !flows_down(world, sx, sy) {
                        continue;
                    }
                    let (cm, cmo, ct) = subtree_sum(world, sx, sy, &mut cache.subtrees, budget);
                    // The whole `(mass, moment)` pair moves together, never
                    // the larger mass with this cell's own moment: they
                    // describe one subtree and mixing two of them describes
                    // nothing. `truncated` is the exception and is OR-ed,
                    // because a truncated share anywhere in the cut
                    // understates the cut.
                    worst.2 |= ct;
                    if cm > worst.0 {
                        (worst.0, worst.1) = (cm, cmo);
                    }
                }
                cache.cuts.insert((lo, hi, y), worst);
                worst
            }
        };
        (mass_fp, moment_fp, truncated) = worst;
    }
    let mass = (mass_fp / LOAD_SCALE) as u32;
    let moment = moment_fp / LOAD_SCALE;
    // **Dividing the moment by the section was tried here and reverted**,
    // and it is still the wrong answer to the concentration the block
    // above fixes. Peak bending stress in a section of depth D is M/D²,
    // which this model already has right (capacity carries the D², torque
    // carries the M). Dividing again makes it M/D³ -- over-strong, so an
    // undercut shelf stops spalling. Worse, it double-counts: in a shelf
    // the rows *already* chain independently to the cliff, so that sharing
    // is present in the tree and must not be applied twice.
    //
    // The block above shares nothing on a shelf for exactly that reason:
    // it applies only where the load path is vertical, which is the one
    // place the tree shares nothing at all.
    let mut torque = (moment_fp - x as i64 * mass_fp).abs() / LOAD_SCALE;
    // **A column is not charged the beam's moment.** Measured on
    // `scene=room`, and it is the reason one chisel cut brought a whole
    // building down: the torque at the inner face of a wall read 76,028 at
    // y=170, at y=200, at y=250 and at y=300 -- *identical all the way to
    // the floor*, 140 cells below the roof that produced it. Every cell of
    // every wall in a building sat at exactly the stress of the worst point
    // of the roof it carried, so nothing was ever safely deep inside a
    // structure, and any single cell that lost its attachment bonus
    // (capacity / 12) failed instantly wherever it happened to be. That is
    // the unzip: not a runaway propagation front, which was the standing
    // hypothesis and is disproved -- see the handoff.
    //
    // Where the moment actually goes: `|Sx - x·M|` is `M × |x̄ - x|`, the
    // moment of everything the cell carries about the cell. For a beam
    // reaching sideways that is right and is the whole model. For a
    // *column* it charges the eccentricity of the roof's centroid -- fifty
    // cells away -- to rock that is simply passing a vertical load
    // downward. The load leaves the beam and enters the column *at the
    // joint*; below the joint the column carries force, not the beam's
    // bending. Charging it again is double-counting, in the same family as
    // the reverted divide-by-section above.
    //
    // So on a vertical support step the arm is clamped to the column's own
    // half-thickness -- the kern: a load landing within a section's middle
    // puts none of it into tension. Beyond that half-width the moment is
    // the *beam's*, and the beam is already being tested for it.
    //
    // Deliberately a clamp and not an exemption. A column genuinely loaded
    // off-axis still carries `M × half-width`, which grows with what it
    // holds, so a thin wall under a heavy eccentric cap still fails -- it
    // just fails at a stress its own thickness explains rather than at one
    // borrowed from something fifty cells away. Every acceptance scene
    // takes its support horizontally at the point that fails
    // (`worked`/`undercut`/`ligament` are all shelves and necks) and so is
    // untouched by this; `capped` and `terrain` can only get stronger.
    //
    // A cell whose support *is* the ground gets the same treatment: it is a
    // column one step long, passing its load into what it stands on, so it
    // owes its own half-thickness rather than the eccentricity of whatever
    // it carries. Without this a rubble-propped slab cell was charged a
    // beam's moment against a capacity the granular divisor had already cut
    // by 64 -- the other half of the crater-rim failure described on
    // `section_cells`.
    if vertical_path {
        let half = (section.len() as i64 / 2).max(1);
        torque = torque.min(mass as i64 * half);
    }
    // **A roof spans the opening under it, not the length of the drive.**
    // The third clamp of the same family, and the one that puts back what
    // a side view throws away -- see `arch_span`.
    if let Some(arm) = arch_span(world, x, y) {
        torque = torque.min(mass as i64 * arm);
    }
    // A piece the ground alone holds up is judged on what the ground can do,
    // as well as on what the rock can do -- whichever gives first.
    let mut capacity = capacity_within(world, x, y, parent, &section);
    // `mass` here is now the whole cut's flow, and that is deliberate: this
    // is the rule `CLAUDE.md` names as having been got wrong once already
    // by being "correct for a piece and applied per cell", so a slab lying
    // on its own rubble was judged as many separate knife-edge footings.
    // `bearing_moment` already reads the *piece's* footing width; feeding it
    // the *piece's* weight is the other half of that pairing.
    if parent.is_none() && rests_on_ground(world, x, y) {
        capacity = capacity.min(bearing_moment(world, x, y, mass));
    }
    Some(Load { mass, moment, torque, capacity, supported, truncated })
}

/// The piece that comes away if `(x, y)` fails, or `None` if it holds.
///
/// The two failures produce genuinely different regions — see the module
/// doc. Both come back sorted, so whatever `rigid::fracture` seeds from is
/// identical run to run.
pub fn failing_region(world: &World, x: i32, y: i32, cache: &mut Cache, budget: &mut u32) -> Option<Failure> {
    let load = evaluate_within(world, x, y, cache, budget)?;
    if !load.fails() {
        return None;
    }
    if !load.supported {
        let region = detached_piece(world, x, y, &mut cache.anchors, budget);
        return Some(Failure { at: (x, y), mode: FailureMode::Unsupported, region });
    }
    // **The section fails, not the cell.**
    //
    // Rock does not part one lamina at a time; it breaks across its
    // section. And the model's own arithmetic makes that more than a
    // cosmetic point: because `NEIGHBOURS_4` breaks ties toward horizontal
    // parents, a slab decomposes into *independent single-row chains*
    // (measured: `mass 115 torque 6555` on a 12-deep shelf is one row,
    // exactly 114*115/2). So one cell's subtree is a **one-cell-thick
    // strip**, and handing that to `rigid::fracture` produced three of the
    // symptoms on the bug list at once:
    //
    // - a collapse delivered sticks and grit. `take_fragment` was changed
    //   to BFS specifically so fragments would be blocky rather than "thin
    //   individual pixel lines", and it cannot help when its input is
    //   already one cell wide.
    // - a 1-cell skin survived on a collapsed shelf -- the top row is a
    //   separate chain that has to fail on its own account.
    // - `DETACH_DEPTH`/`CRACK_DETACH_DEPTH` were both sized so "pieces can
    //   only be as thick as the loosened rock they came from", a rationale
    //   the load model had silently invalidated since the piece's thickness
    //   was set by the subtree and was 1 regardless.
    //
    // So the region is the union of the subtrees hanging off every cell of
    // the failing section. Bounded by `MAX_SECTION`, on a path that only
    // runs once something has already been judged to fail, and every walk
    // after the first is close to free because `Cache::subtrees` already
    // holds it.
    let mut region: HashSet<(i32, i32)> = HashSet::new();
    for (cx, cy) in section_cells(world, x, y, support_parent(world, x, y)) {
        let (sub, _) = supported_subtree(world, cx, cy, budget);
        region.extend(sub);
    }
    let mut region: Vec<(i32, i32)> = region.into_iter().collect();
    // Sorted, not merely collected: a `HashSet`'s iteration order is
    // randomized per process, and this feeds `rigid::fracture`'s fragment
    // seeding. Issue #7's determinism trap in a new place.
    region.sort_unstable();
    Some(Failure { at: (x, y), mode: FailureMode::Overloaded, region })
}

/// Ancestors a settling cell re-checks on its way to the anchor.
///
/// **Set from measurement, and the measurement overturned the previous
/// reasoning here.** This was 128, justified by `scene=ligament`: at 16
/// its neck stood at a stress ratio of 1.87, because the overhang's far
/// end settles last and the walk had to reach back from there to the neck.
///
/// That is no longer true, and instrumenting it is what showed so. With
/// the section failing rather than the cell (`failing_region`) and
/// budget exhaustion reported rather than swallowed (`ChainVerdict`), the
/// furthest any failure lands from the check that found it, across every
/// scene, is:
///
/// ```text
/// ligament 0    undercut 0    mine 0    snap 0    worked 29    strike 34
/// ```
///
/// `ligament` is **zero** — the neck now fails on its own check, and the
/// walk contributes nothing to the scene it was raised for. 48 is the
/// measured maximum plus headroom, per `CLAUDE.md`'s "set bars from
/// measurement with headroom, never from an aspiration and never sitting
/// on the measured value".
///
/// The reduction matters beyond cost. `Reports/prior-art-destruction.md`
/// flags a long walk as having 7 Days to Die's exact bug shape — a blow
/// bringing down rock a hundred cells away, frames later, which players
/// experienced as bases collapsing for no visible reason. A collapse the
/// player did not cause is worse than one that does not happen, and this
/// bounds how far a consequence can travel from its cause without needing
/// the disturbance-anchored rework that idea originally called for.
const ROOTWARD_CHECK_STEPS: usize = 48;

/// `failing_region` for `(x, y)` **and its ancestors**, returning the first
/// piece that comes away.
///
/// # Why the cell being checked is not the only one that can fail
///
/// Load is non-local: what a cell carries depends on everything hanging off
/// it. But the distance relaxation converges *outward from the anchor*, so
/// a root settles before the span it supports exists as far as the support
/// forest is concerned. It gets judged carrying almost nothing, passes,
/// stops rescheduling itself, and is never asked again while the load
/// arrives behind it.
///
/// Measured on `scene=ligament` before this existed: the neck sat at a
/// stress ratio of **2.52** — two and a half times over its capacity — and
/// stood there indefinitely, with a 4,400-cell overhang hanging off it. The
/// only tick that ever judged it ran before the overhang had converged.
///
/// Walking here rather than scheduling the ancestors is deliberate; see
/// `structural::tick`'s own note for the queue blow-up that alternative
/// caused. Ancestors are nearly free after the first walk because
/// `Cache::subtrees` already holds their sums.
pub fn failing_along_support_chain(world: &World, x: i32, y: i32, cache: &mut Cache, budget: &mut u32) -> ChainVerdict {
    let mut at = (x, y);
    for _ in 0..ROOTWARD_CHECK_STEPS {
        if *budget == 0 {
            // Out of room this frame. Reported rather than swallowed: a
            // half-finished walk that returns "holds" is a *dropped check*,
            // and the cell then stops rescheduling itself and is never
            // asked again. That is how `scene=ligament`'s neck kept
            // standing at a stress ratio of 1.87 -- the walk that would
            // have caught it ran out of budget and quietly said fine.
            return ChainVerdict::Deferred;
        }
        if let Some(failure) = failing_region(world, at.0, at.1, cache, budget) {
            return ChainVerdict::Failing(failure);
        }
        match support_parent(world, at.0, at.1) {
            Some(parent) => at = parent,
            None => break, // reached an anchor
        }
    }
    ChainVerdict::Holds
}

/// Which of the two failures fired. They are different events with
/// different causes, and a contact sheet cannot tell them apart — a piece
/// that was overloaded and a piece that was never held look identical
/// falling. `CLAUDE.md`: "did it fire at all" needs a counter, not a
/// picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureMode {
    /// The chain reached an anchor, but the moment exceeded the section.
    Overloaded,
    /// Nothing held it at all.
    Unsupported,
}

/// A failure, and **where** it happened.
///
/// The position is not the cell that was checked — it is the ancestor the
/// chain walk found to be over its limit, which may be many cells away.
/// Carried out so the impulse, and therefore what the eye reads as the
/// origin of the collapse, lands on the joint that gave way rather than on
/// the middle of the piece that fell.
pub struct Failure {
    pub at: (i32, i32),
    pub mode: FailureMode,
    pub region: Vec<(i32, i32)>,
}

/// What a chain walk concluded. `Deferred` is not `Holds` — see
/// `failing_along_support_chain`.
pub enum ChainVerdict {
    Failing(Failure),
    Holds,
    Deferred,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::material;
    use crate::sim::structural;

    fn test_world() -> World {
        World::new(Rect::new(0, 0, 63, 63))
    }

    /// A free-standing wall, 11 cells thick, with a cap reaching out to
    /// one side so the roof's load enters at one face.
    ///
    /// Unattached on purpose: `is_structurally_interesting` skips anything
    /// buried and attached, and a wall that is never evaluated proves
    /// nothing. This is the shape a player builds anyway.
    fn wall_under_a_one_sided_cap(share: bool) -> World {
        let mut w = test_world();
        w.section_share = share;
        for y in 20..64 {
            for x in 20..31 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for y in 17..20 {
            for x in 20..56 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        structural::compute_world_distances(&mut w);
        w
    }

    /// **A wall carries its roof through every cell of its thickness**, not
    /// through whichever face `NEIGHBOURS_4` happened to break a tie
    /// toward.
    ///
    /// The regression test for the defect the owner reported three separate
    /// times and saw as a one-pixel red line down an otherwise green
    /// building. Measured in the harness on `scene=room wall=8 dig=0`, at
    /// one row of one wall: the outer face carried mass 157 and the inner
    /// face 2,956, and the eleven cells between them were never evaluated
    /// at all -- while every one of them was judged against the bending
    /// capacity of the whole 17-cell section.
    ///
    /// Paired against the same world with the rule switched off rather than
    /// against a remembered number, per `CLAUDE.md`. That is also what
    /// makes it non-vacuous: if this geometry ever stops producing a
    /// concentration, the control half fails first and says which end went
    /// wrong, instead of the equality passing for the wrong reason.
    #[test]
    fn both_faces_of_a_wall_carry_the_roof_not_just_the_one_it_lands_on() {
        let concentrated = wall_under_a_one_sided_cap(false);
        let outer = evaluate(&concentrated, 20, 40).expect("outer face evaluable").mass;
        let inner = evaluate(&concentrated, 30, 40).expect("inner face evaluable").mass;
        assert!(
            inner > outer * 4,
            "test setup: this geometry must concentrate load on the near face, but read outer {outer} inner {inner} -- with no concentration to remove, the assertion below would pass for the wrong reason"
        );

        let shared = wall_under_a_one_sided_cap(true);
        let outer = evaluate(&shared, 20, 40).expect("outer face evaluable").mass;
        let inner = evaluate(&shared, 30, 40).expect("inner face evaluable").mass;
        assert_eq!(outer, inner, "both faces of one wall are one member and must be judged as one");
        assert!(outer > 0, "and the shared reading must be the load, not zero");

        // **Nothing may end up carrying more than the world contains.**
        //
        // A standing sanity invariant rather than a guard on any one rule,
        // and cheap enough to be worth having. The summing version of this
        // code read `mass 62,391` at the wall-to-cap joint of a room built
        // from about 14,000 cells, by taking its cut *along* a cantilever
        // instead of across it. A number larger than the world it came from
        // is the tell.
        //
        // Stated honestly: this does **not** currently fail if the
        // `flows_down` filter is deleted. Under a max rather than a sum
        // nothing double-counts, so the filter earns its place on the seed
        // sweep's worst case and nowhere a unit test can reach -- see the
        // note in `evaluate_within`. What this catches is the arithmetic
        // going wrong again in the direction it already went wrong once.
        let stone: u32 = (0..64).map(|y| (0..64).filter(|&x| shared.get(x, y).material == material::STONE).count() as u32).sum();
        for x in 20..31 {
            let at_the_joint = evaluate(&shared, x, 19).expect("the cap's underside is evaluable").mass;
            assert!(
                at_the_joint <= stone,
                "({x},19) reports carrying {at_the_joint} of a structure that is only {stone} cells -- the cut is being taken along the load rather than across it"
            );
        }
    }

    /// A run that never finds an end is **not** a member, and its cells
    /// keep exactly the load they already carried.
    ///
    /// The guard on the half of this rule that cost the most to find, and a
    /// paired comparison so it cannot pass by the mechanism being dead:
    /// the *same* tower on the *same* wall, and only the wall's thickness
    /// differs. At 11 thick the run ends in air on both hands, so the far
    /// side goes 11 -> 31 and picks up the tower. At 45 thick the run is
    /// still stone where `MAX_SECTION` stops it, so it is a window rather
    /// than a member and the far side stays at 11.
    ///
    /// Without the distinction, `seedsweep.sh strike=12` run to rest went
    /// from p90 2,227 rock destroyed to 4,465, with single runs reporting
    /// 49,265 overload failures: a shallow surface sheet is one unbroken
    /// run, and coupling every 40-cell window of it is
    /// `Reports/next-session-handoff.md` 1c's lateral unzip given a
    /// shortcut.
    #[test]
    fn a_run_with_no_ends_is_not_a_member_so_nothing_is_shared_across_it() {
        /// A wall on the floor with a tower on its left end -- a one-sided
        /// load that a member is supposed to spread and a window is not.
        fn wall_under_a_tower(share: bool, x0: i32, x1: i32) -> World {
            let mut w = test_world();
            w.section_share = share;
            for y in 30..64 {
                for x in x0..x1 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            for y in 10..30 {
                for x in x0..(x0 + 5) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            structural::compute_world_distances(&mut w);
            w
        }
        // The far side of a wall thin enough to have two ends.
        let thin_off = evaluate(&wall_under_a_tower(false, 2, 13), 12, 40).expect("evaluable").mass;
        let thin_on = evaluate(&wall_under_a_tower(true, 2, 13), 12, 40).expect("evaluable").mass;
        assert!(
            thin_on > thin_off,
            "test setup: the far side of an 11-thick wall must gain load from the tower ({thin_off} -> {thin_on}), or the exclusion below is only proof that the rule never runs"
        );

        // A wall too thick for the run to find an end, where `MAX_SECTION`
        // stops it in solid rock.
        //
        // Probed at x=40 and not at the far face, and that is the whole
        // sharpness of this test: `section_cells` fills leftward first, so
        // the 40-cell window here is x=2..41 and **contains the tower**.
        // The far face's window starts at x=5 and misses it, so it reads
        // unchanged whether the rule fires or not -- which is what the
        // first version of this test asserted, and it passed with the
        // member gate deleted.
        let wide_off = evaluate(&wall_under_a_tower(false, 2, 47), 40, 40).expect("evaluable").mass;
        let wide_on = evaluate(&wall_under_a_tower(true, 2, 47), 40, 40).expect("evaluable").mass;
        assert_eq!(wide_on, wide_off, "a run with no ends has nothing to redistribute between and must be untouched");
    }

    /// A shelf is untouched: it takes its support sideways, and its rows
    /// already chain independently to the cliff.
    ///
    /// The standing guard against re-introducing the reverted
    /// divide-by-section, which took `scene=undercut` to zero failures. The
    /// section rule is gated on a vertical load path precisely so that a
    /// horizontal one keeps every number it had.
    #[test]
    fn a_cantilevered_shelf_shares_nothing_because_its_path_is_sideways() {
        fn shelf(share: bool) -> World {
            let mut w = test_world();
            w.section_share = share;
            for y in 10..54 {
                for x in 0..20 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for y in 20..26 {
                for x in 20..50 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            structural::compute_world_distances(&mut w);
            w
        }
        let (off, on) = (shelf(false), shelf(true));
        for x in [21, 30, 40] {
            for y in 20..26 {
                assert_eq!(
                    evaluate(&off, x, y).map(|l| (l.mass, l.torque, l.capacity)),
                    evaluate(&on, x, y).map(|l| (l.mass, l.torque, l.capacity)),
                    "the shelf cell ({x},{y}) takes its support horizontally and must be judged exactly as before"
                );
            }
        }
    }

    /// A roof over a **small** opening owes a shorter arm than the same
    /// roof over a big one, which is the whole of requirement 2: *"a super
    /// short tunnel should be able to have a longer span. ant tunnel vs
    /// digging a mine."*
    ///
    /// Asserted on the arm rather than on a finished torque, and that is a
    /// deliberate retreat rather than laziness. `torque` is
    /// `min(natural, mass x arm)`, so a test that wants to see the clamp
    /// *bind* has to first contrive a natural torque large enough to be
    /// clamped — and the obvious symmetric slot gives a centroid directly
    /// overhead, an arm of zero, and two torques of 0 that compare equal
    /// while proving nothing. Building geometry elaborate enough to dodge
    /// that would be testing the geometry.
    ///
    /// The end-to-end behaviour is gated where it is legible instead:
    /// `scripts/acceptance.sh`'s `cavedeep` / `caveshallow` pair, and the
    /// measured bore table in `Reports/next-session-handoff.md` (a 5-cell
    /// tunnel keeps 100% of a 140-cell drive where a 13-cell gallery keeps
    /// 25%).
    #[test]
    fn a_roof_over_a_small_opening_owes_a_shorter_arm_than_over_a_big_one() {
        fn roof_over_slot(height: i32) -> World {
            let mut w = test_world();
            for y in 10..50 {
                for x in 0..64 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for y in 30..(30 + height) {
                for x in 8..56 {
                    w.set(x, y, Cell::EMPTY);
                }
            }
            structural::compute_world_distances(&mut w);
            w
        }

        let arm = |height: i32| arch_span(&roof_over_slot(height), 32, 29).expect("a ceiling over a slot is a roof");
        let (small, big) = (arm(3), arm(12));
        assert!(
            small < big,
            "a roof over a 3-cell opening owes an arm of {small} and over a 12-cell one {big};              the smaller hole must be the lighter job or an ant tunnel cannot outlast a gallery"
        );
        // And the relief is bounded rather than unlimited: a cavern does
        // not go on earning a longer arm forever, or a big enough hole
        // would be charged more than the unclamped model ever would.
        assert_eq!(arm(40), arm(MAX_OPENING_PROBE as i32), "the opening probe must saturate");
    }

    /// The arch needs rock to form out of, and an overhang has none.
    ///
    /// This is the guard on the **replacement artifact** rather than the
    /// original one. The way this change fails is not by leaving tunnels
    /// fragile, it is by quietly relieving every cell that happens to have
    /// a void beneath it — which would take `scene=undercut`'s spalling
    /// and `scene=ligament`'s neck with it, and those are two of the four
    /// cases that killed the earlier support models.
    #[test]
    fn an_overhang_with_sky_above_it_gets_no_arch() {
        let mut w = test_world();
        // A one-cell-thick shelf reaching out from the left wall, open
        // above and below: a cantilever, not a roof.
        for x in 0..40 {
            w.set(x, 30, Cell::new(material::STONE, 0).with_attached(true));
        }
        structural::compute_world_distances(&mut w);
        assert_eq!(
            arch_span(&w, 32, 30),
            None,
            "a shelf with nothing above it has no arch to form; relieving it would stop undercuts spalling"
        );
    }

    /// A budget large enough that no test below is measuring the cap
    /// instead of the model. The per-frame budget is a scheduling concern;
    /// these are about arithmetic and shape.
    fn unbounded() -> u32 {
        u32::MAX
    }

    /// A one-cell-thick horizontal beam from the left world edge (which
    /// reads as `BEDROCK`, so `x = 0` is an anchor) out to `length`.
    fn beam(length: i32) -> World {
        let mut w = test_world();
        for x in 0..=length {
            w.set(x, 30, Cell::new(material::STONE, 0));
        }
        structural::compute_world_distances(&mut w);
        w
    }

    #[test]
    fn a_cantilevers_torque_is_the_sum_of_its_lever_arms() {
        // The arithmetic, against a case whose answer can be worked out by
        // hand -- `CLAUDE.md`: sanity-check a new metric against a case you
        // know is fine, before trusting it about one you don't. A 10-cell
        // beam seen from x=1 supports x=1..10, whose lever arms are
        // 0,1,2,...,9, summing to 45.
        let w = beam(10);
        let load = evaluate(&w, 1, 30).expect("a beam cell should be evaluable");
        assert_eq!(load.mass, 10, "x=1 should be carrying itself and everything beyond it");
        assert_eq!(load.torque, 45, "torque should be the sum of lever arms 0..=9");
        assert!(load.supported, "the beam reaches the left world edge");
        assert!(!load.truncated, "a 10-cell beam is nowhere near the region cap");
    }

    #[test]
    fn a_towers_mass_sits_above_it_so_its_torque_is_zero() {
        // The case mass alone cannot distinguish from a cantilever, and the
        // reason the quantity has to be a moment. Fifty cells stacked is
        // fine; fifty reaching out is not.
        let mut w = test_world();
        for y in 20..=63 {
            w.set(30, y, Cell::new(material::STONE, 0));
        }
        structural::compute_world_distances(&mut w);
        let load = evaluate(&w, 30, 40).expect("a tower cell should be evaluable");
        assert!(load.mass > 15, "test setup: the tower cell should be carrying the column above it, found {}", load.mass);
        assert_eq!(load.torque, 0, "a vertical column's mass sits directly above it");
        assert!(load.torque <= load.capacity, "a tower must stand at any height");
    }

    #[test]
    fn support_parents_form_a_forest_rooted_at_anchors() {
        // The structural claim the whole accumulation rests on: following
        // parents terminates, at an anchor, from every cell -- no cycles,
        // no orphans. Asserted over a shape with both a vertical and a
        // horizontal run so neither direction is untested.
        let mut w = test_world();
        for y in 40..=63 {
            w.set(10, y, Cell::new(material::STONE, 0));
        }
        for x in 10..40 {
            w.set(x, 40, Cell::new(material::STONE, 0));
        }
        structural::compute_world_distances(&mut w);

        for (x, y) in (40..=63).map(|y| (10, y)).chain((10..40).map(|x| (x, 40))) {
            let mut at = (x, y);
            let mut steps = 0;
            while w.get(at.0, at.1).aux() != 0 {
                let parent = support_parent(&w, at.0, at.1).unwrap_or_else(|| panic!("({},{}) has no parent and is not an anchor", at.0, at.1));
                assert!(
                    w.get(parent.0, parent.1).aux() < w.get(at.0, at.1).aux(),
                    "a parent must be strictly closer to an anchor, or the walk can cycle"
                );
                at = parent;
                steps += 1;
                assert!(steps < MAX_SUPPORT_WALK, "walking up from ({x},{y}) never reached an anchor");
            }
        }
    }

    #[test]
    fn a_subtree_counts_every_cell_once_where_two_paths_are_equally_short() {
        // Pitfall 5.1, as a test rather than a comment. A solid block has
        // many cells reachable by two equally short routes; flooding to
        // "neighbours with a greater distance" would count those subtrees
        // twice and inflate the mass. Parent-based flooding cannot, because
        // every cell has exactly one parent.
        let mut w = test_world();
        for y in 40..50 {
            for x in 10..20 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        structural::compute_world_distances(&mut w);

        let block: Vec<(i32, i32)> = (40..50).flat_map(|y| (10..20).map(move |x| (x, y))).collect();
        let mut claimed: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
        for &(x, y) in &block {
            let (subtree, _) = supported_subtree(&w, x, y, &mut unbounded());
            for &cell in &subtree {
                // Every cell appears in its own subtree and in each of its
                // ancestors', which is correct -- what must never happen is
                // the same cell appearing twice within ONE subtree.
                assert_eq!(subtree.iter().filter(|&&c| c == cell).count(), 1, "{cell:?} appears twice in ({x},{y})'s subtree");
            }
            claimed.insert((x, y), (x, y));
        }
        assert_eq!(claimed.len(), block.len(), "test setup: every block cell should have been walked");
    }

    #[test]
    fn a_piece_with_no_path_to_an_anchor_reads_as_unsupported_whole() {
        // A blob painted in mid-air. The distinction that matters is not
        // "does it fail" but "does it come away as one piece" -- taking it
        // a cell at a time is the dissolve-to-dust outcome playtesting has
        // rejected twice.
        let mut w = test_world();
        for y in 20..28 {
            for x in 20..30 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        structural::compute_world_distances(&mut w);

        let load = evaluate(&w, 25, 24).expect("a floating blob cell should be evaluable");
        assert!(!load.supported, "a blob with no path to an anchor must read as unsupported");
        let failure = failing_region(&w, 25, 24, &mut Cache::default(), &mut unbounded()).expect("an unsupported cell must fail");
        assert_eq!(failure.mode, FailureMode::Unsupported, "nothing holds this blob, so it is not *overloaded* -- it is falling");
        assert_eq!(failure.region.len(), 80, "the whole 10x8 blob should come away, not one cell of it");
    }

    #[test]
    fn a_fresh_crack_does_not_make_still_supported_rock_read_as_falling() {
        // Found by measurement, not by reasoning: probing `scene=worked`
        // right after its six blows reported the shelf's underside
        // UNSUPPORTED at a torque well inside its capacity. It was not
        // falling -- its shortest path to the cliff had just been cut, so
        // its cached distance was stale and it had no valid parent, while
        // a perfectly good load path existed the long way round.
        //
        // Reproduced here without any relaxation in between, which is the
        // worst case: every distance below is exactly as `compute_world_
        // distances` left it before the crack existed.
        let mut w = test_world();
        // A cliff against the left world edge (which reads as `BEDROCK`),
        // deliberately only as deep as the slab's *upper* half. So the
        // lower half is anchored only through the upper half, never
        // directly -- without that the fracture below cuts nothing, since
        // both halves would touch the edge on their own.
        for y in 28..32 {
            for x in 0..5 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        for y in 30..=33 {
            for x in 5..25 {
                w.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        structural::compute_world_distances(&mut w);

        // A horizontal fracture along the slab's middle, running from the
        // cliff out to x=14 but *not* to the far end -- so the lower half
        // is still joined to the upper half beyond x=14, and through it to
        // the cliff.
        for x in 5..15 {
            let cell = w.get(x, 31).with_crack_down(true);
            w.set(x, 31, cell);
        }

        let load = evaluate(&w, 8, 33).expect("a slab cell below the crack should be evaluable");
        assert!(
            load.supported,
            "rock still joined to an anchor the long way round read as falling -- torque {} capacity {}",
            load.torque,
            load.capacity
        );

        // The other half of the claim, or the assertion above would pass
        // just as happily against a function that always says "supported":
        // carry the same fracture to the far end and the lower half really
        // is cut free.
        for x in 15..25 {
            let cell = w.get(x, 31).with_crack_down(true);
            w.set(x, 31, cell);
        }
        let cut = evaluate(&w, 8, 33).expect("still evaluable once fully cut");
        assert!(!cut.supported, "a fracture carried all the way through must actually detach the piece");
    }

    #[test]
    fn a_beam_keyed_into_terrain_is_stronger_at_the_joint_and_only_there() {
        // The owner's ask -- "we should be able to attach foreground
        // objects to background objects" -- answered as a property of the
        // *joint* rather than of the cell. Both halves are asserted,
        // because the half that matters for safety is the second: if the
        // bonus chained, everything a player builds would inherit it from
        // the ground up, which is how all four earlier support models made
        // built structures indestructible.
        let mut w = test_world();
        for y in 28..40 {
            for x in 0..10 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        // Foreground beam reaching out of the cliff face.
        for x in 10..30 {
            w.set(x, 33, Cell::new(material::STONE, 0));
        }
        structural::compute_world_distances(&mut w);

        assert!(!w.get(10, 33).attached(), "test setup: the beam must be foreground");
        assert_eq!(support_parent(&w, 10, 33), Some((9, 33)), "test setup: the root's support should come from the cliff");

        let root = capacity(&w, 10, 33);
        let next = capacity(&w, 11, 33);
        let bonus = w.materials.get(material::STONE).attached_span_bonus as i64;
        assert!(bonus > 1, "test setup: stone needs a real attachment bonus for this to mean anything");
        assert_eq!(root, next * bonus, "the cell keyed into the cliff should carry the full attachment bonus");

        // And it stops there. One cell further out takes its support from
        // foreground stone, so there is no joint and no bonus.
        assert_eq!(capacity(&w, 12, 33), next, "the bonus chained outward, which would make everything built indestructible");
    }

    #[test]
    fn attached_bulk_is_skipped_before_any_work_is_done() {
        // §5.9's early-out, which is the difference between a cost
        // proportional to surface area and one proportional to volume.
        let mut w = test_world();
        for y in 40..60 {
            for x in 10..40 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        structural::compute_world_distances(&mut w);

        assert!(!is_structurally_interesting(&w, 25, 50), "a buried attached cell has nothing that could make it fail");
        assert!(evaluate(&w, 25, 50).is_none(), "attached bulk must not be evaluated at all");
        // A free face is enough to make it interesting again -- it is the
        // surface, not the volume, that this model has to look at.
        assert!(is_structurally_interesting(&w, 25, 40), "a cell on the top face is exposed and must be evaluated");
    }

    #[test]
    fn a_cracked_edge_makes_an_attached_cell_interesting_again() {
        let mut w = test_world();
        for y in 40..60 {
            for x in 10..40 {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        structural::compute_world_distances(&mut w);
        assert!(!is_structurally_interesting(&w, 25, 50), "test setup: this cell should start as ordinary bulk");

        let scored = w.get(25, 50).with_crack_right(true);
        w.set(25, 50, scored);
        assert!(is_structurally_interesting(&w, 25, 50), "a scored cell must be evaluated -- damage is the whole point of a crack");
    }

    #[test]
    fn cracks_cut_capacity_without_taking_it_to_nothing() {
        // The graded outcome §0a demands: a piece sags, strains, then goes.
        // A cell with three of four edges cracked is hanging by one and
        // should carry almost nothing -- but not zero, or it vanishes the
        // moment it is touched rather than visibly weakening first.
        let w = beam(10);
        let intact = capacity(&w, 5, 30);

        let mut scored = beam(10);
        let cell = scored.get(5, 30).with_crack_down(true);
        scored.set(5, 30, cell);
        let once = capacity(&scored, 5, 30);
        assert!(once < intact, "a cracked edge must cut what the cell can carry");
        assert!(once > 0, "a cracked cell must still carry something, or it is gone rather than weakened");
        assert_eq!(once, intact * 3 / 4, "one of four faces gone should cost a quarter of the section");
    }

    #[test]
    fn a_deeper_section_carries_much_more_than_a_thin_one() {
        // Bending capacity grows with the square of section depth. Without
        // this term a forty-cell-thick cap is exactly as fragile as a
        // one-cell ledge, which is how a thick column came to crumble under
        // a shallow overhang in play.
        let thin = beam(10);
        let mut thick = test_world();
        for y in 26..=34 {
            for x in 0..=10 {
                thick.set(x, y, Cell::new(material::STONE, 0));
            }
        }
        structural::compute_world_distances(&mut thick);

        let thin_capacity = capacity(&thin, 5, 30);
        let thick_capacity = capacity(&thick, 5, 30);
        assert!(
            thick_capacity > thin_capacity * 50,
            "a 9-deep section should carry far more than a 1-deep one, found {thick_capacity} vs {thin_capacity}"
        );
    }

    #[test]
    fn a_cell_resting_on_powder_is_evaluated_against_a_granular_capacity() {
        // B2's actual claim, guarded through `evaluate` rather than through
        // `capacity` alone — the exemption path is what shipped broken: the
        // divisor branch existed, and `evaluate_within`'s anchor guard
        // returned `None` before it could ever run, so a powder-propped
        // cell was never torque-tested at all. §5.7's vacuous-pass shape,
        // caught by review rather than by any test; this is that test.
        let mut w = beam(20);
        w.set(10, 31, Cell::new(material::SAND, 0));
        structural::compute_world_distances(&mut w);

        let load = evaluate(&w, 10, 30).expect("a powder-propped cell must be evaluated, not exempt");
        assert!(load.supported, "powder underneath keeps the chain alive — held is not the same as braced");
        // **Restated when the flat divisor was replaced by a bearing model.**
        // The old form asserted `capacity == full / 64`, which pinned the
        // arithmetic of a mechanism that no longer exists. What B2 actually
        // claims is that a grain is not a brace, and that survives the
        // change: a beam with a grain under it must be judged exactly as the
        // same beam without one, because grain neither carries a moment nor
        // shortens anything's path to an anchor. Comparing the two worlds
        // states that directly, and it can still fail for the *replacement*
        // artifact -- if powder ever again roots a cell at distance zero,
        // the propped beam's load diverges from the bare one's immediately.
        let bare = evaluate(&beam(20), 10, 30).expect("the same cell without the grain must also be evaluated");
        assert_eq!(load.capacity, bare.capacity, "a grain underneath must not change what the rock can carry");
        assert_eq!(load.mass, bare.mass, "a grain must not take any of the beam's weight");
        assert_eq!(load.torque, bare.torque, "a grain must not relieve the beam's moment");
    }

    #[test]
    fn a_sprinkle_of_sand_under_a_beam_does_not_hold_the_beam_up() {
        // The symptom named in `Reports/destruction-plan.md` B2: a grain
        // wedged under a span acted as a permanent anchor, so rock could
        // hang off it forever. The propped cell is genuinely *held* — the
        // chain terminates there — and what it carries (both arms of the
        // beam now route their support through the false zero the prop
        // puts in the distance field) is far beyond what loose grains can
        // brace against turning.
        // **Restated when ground stopped rooting eagerly.** The old form
        // asserted that the *propped cell itself* fails, which was the
        // symptom of the bug rather than the claim: the grain put a false
        // zero in the distance field, both arms of the beam re-routed through
        // it, and the cell then had to answer for a moment no pile could
        // brace. `structural::tick` no longer roots on powder unless there is
        // no other path, so there is no false zero to fail at -- and the
        // guard has to be the claim itself, which is that the grain changes
        // *nothing*. Asserting the old symptom now would demand a failure the
        // fixed model is right not to produce.
        let mut w = beam(30);
        w.set(15, 31, Cell::new(material::SAND, 0));
        structural::compute_world_distances(&mut w);
        let mut bare = beam(30);
        structural::compute_world_distances(&mut bare);

        for x in 0..=30 {
            let propped = evaluate(&w, x, 30);
            let plain = evaluate(&bare, x, 30);
            assert_eq!(
                propped.map(|l| (l.mass, l.torque, l.capacity, l.supported)),
                plain.map(|l| (l.mass, l.torque, l.capacity, l.supported)),
                "the grain under x=15 changed the beam at x={x} — it is acting as support again"
            );
            assert_eq!(
                w.get(x, 30).aux(),
                bare.get(x, 30).aux(),
                "the grain moved x={x}'s distance to an anchor — that is the false zero B2 was about"
            );
        }
    }
}



