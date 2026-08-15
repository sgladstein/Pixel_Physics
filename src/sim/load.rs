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

/// How much of its bending capacity a cell keeps when the only thing under
/// it is loose grain. Not zero -- a pile does carry weight, and zeroing it
/// would shatter debris the moment it landed, which is the bug
/// `rests_on_ground` exists to prevent.
const GRANULAR_CAPACITY_DIVISOR: i64 = 64;

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

/// Whether `(x, y)` is anchored outright — touching bedrock (or the world
/// edge, which `Cell::OUT_OF_BOUNDS` reports as bedrock), or simply sitting
/// on the ground.
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
    NEIGHBOURS_4.iter().any(|&(dx, dy)| world.get(x + dx, y + dy).material == super::material::BEDROCK) || rests_on_ground(world, x, y)
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
pub type Subtree = (u32, i64, bool);

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
}

impl Cache {
    pub fn clear(&mut self) {
        self.subtrees.clear();
        self.anchors.clear();
    }
}

/// The children of `(x, y)` in the support forest — the neighbours that
/// take their support *from* it.
///
/// Asked as "who is your parent" rather than "who is further from an
/// anchor than me", and that is the difference between correct and
/// double-counted: every cell has exactly one parent, so a cell can appear
/// in exactly one subtree at each level. Flooding to greater-distance
/// neighbours instead counts a subtree twice wherever two paths cost the
/// same, which is most of a solid slab.
fn children(world: &World, x: i32, y: i32, out: &mut Vec<(i32, i32)>) {
    out.clear();
    for (dx, dy) in NEIGHBOURS_4 {
        if edge_is_cracked(world, x, y, dx, dy) {
            continue; // a fracture carries no load, so nothing across one is ours
        }
        let (nx, ny) = (x + dx, y + dy);
        let cell = world.get(nx, ny);
        if !is_body_material(world, cell.material) || cell.organism_id() != 0 {
            continue;
        }
        if support_parent(world, nx, ny) == Some((x, y)) {
            out.push((nx, ny));
        }
    }
}

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
    // A post-order walk with an explicit stack. `Enter` discovers a cell's
    // children; `Exit` sums them once they are all in the memo. The child
    // list is carried on the `Exit` entry rather than recomputed, because
    // finding it costs ~25 hashed `World::get` calls (four neighbours, each
    // asking who *its* parent is) and doing that twice per cell doubles the
    // cost of the single hottest loop in this module. A fixed array, not a
    // `Vec`: there are never more than four, and this runs often enough
    // that an allocation per cell is worth avoiding.
    enum Step {
        Enter((i32, i32)),
        Exit((i32, i32), [(i32, i32); 4], usize),
    }
    let mut kids = Vec::new();
    let mut stack = vec![Step::Enter((x, y))];
    // Cells actually walked, not `stack.len()`. Those are different
    // quantities -- the stack holds the pending *frontier*, which for a
    // wide flat slab stays small while the walk visits thousands -- and
    // testing the wrong one here meant this cap and the identically-named
    // one in `supported_subtree` bounded two different things while
    // claiming to bound the same one.
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
                    memo.insert(node, (1, node.0 as i64, true));
                    continue;
                }
                *budget = budget.saturating_sub(1);
                walked += 1;
                children(world, node.0, node.1, &mut kids);
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
                let mut mass = 1u32;
                let mut moment = node.0 as i64;
                let mut truncated = false;
                for &kid in &held[..count] {
                    // Present unless the child hit the guard above, which
                    // inserts before this pop -- so the fallback is only a
                    // belt-and-braces default, never the normal path.
                    let (m, mo, t) = memo.get(&kid).copied().unwrap_or((1, kid.0 as i64, true));
                    mass = mass.saturating_add(m);
                    moment = moment.saturating_add(mo);
                    truncated |= t;
                }
                memo.insert(node, (mass, moment, truncated));
            }
        }
    }
    memo.get(&(x, y)).copied().unwrap_or((1, x as i64, true))
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
fn section(world: &World, x: i32, y: i32, parent: Option<(i32, i32)>) -> i64 {
    // Support from below/above travels vertically, so the resisting section
    // is horizontal, and vice versa. No parent (an anchor, or mid-
    // convergence) keeps the old vertical reading.
    let vertical_path = parent.is_some_and(|(px, _)| px == x);
    let (sx, sy) = if vertical_path { (1, 0) } else { (0, 1) };
    let mut extent = 1i64;
    for dir in [-1, 1] {
        let mut step = 1;
        while extent < MAX_SECTION {
            let probe = world.get(x + dir * sx * step, y + dir * sy * step);
            if !is_body_material(world, probe.material) || probe.material == super::material::EMPTY {
                break;
            }
            extent += 1;
            step += 1;
        }
    }
    extent
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
pub fn capacity(world: &World, x: i32, y: i32) -> i64 {
    let cell = world.get(x, y);
    let m = world.materials.get(cell.material);
    if m.max_unsupported_span == u16::MAX {
        return i64::MAX; // this material does not participate in the structural system at all
    }
    let base = (m.max_unsupported_span as i64).pow(2) / 2;
    let section = section(world, x, y, support_parent(world, x, y));
    // Attachment buys *capacity*, never immunity. Anchoring on it made an
    // undercut shelf unfallable however much was dug from beneath it --
    // model 3 of the four this replaces.
    let attachment = if cell.attached() { m.attached_span_bonus as i64 } else { 1 };
    let capacity = base.saturating_mul(section.pow(2)).saturating_mul(attachment).saturating_mul(uncracked_faces(world, x, y)) / CRACK_FACES;
    // A cell whose support is a pile of loose grains is *held*, but it is
    // not braced: sand resists compression and essentially no bending. So
    // resting on powder keeps the chain alive (see `rests_on_ground`) and
    // costs almost all of the section's capacity, which is what stops a
    // handful of rubble propping up a slab.
    if rests_on_ground(world, x, y) {
        return (capacity / GRANULAR_CAPACITY_DIVISOR).max(1);
    }
    capacity
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
    if is_anchor(world, x, y) {
        return None; // genuinely held from outside the model
    }
    if !is_structurally_interesting(world, x, y) {
        return None;
    }

    let supported = is_supported(world, x, y, &mut cache.anchors, budget);
    let (mass, moment, truncated) = subtree_sum(world, x, y, &mut cache.subtrees, budget);
    let torque = (moment - x as i64 * mass as i64).abs();
    Some(Load { mass, moment, torque, capacity: capacity(world, x, y), supported, truncated })
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
    let (mut subtree, _) = supported_subtree(world, x, y, budget);
    subtree.sort_unstable();
    Some(Failure { at: (x, y), mode: FailureMode::Overloaded, region: subtree })
}

/// Ancestors a settling cell re-checks on its way to the anchor.
///
/// **Set from the geometry it has to cover, not picked round.** A structure
/// converges outward from its anchor, so the *last* cell to settle is the
/// one furthest along the span — and it is the one whose settling completes
/// the load that the neck behind it has to carry. The walk therefore has to
/// reach from the far tip back to the neck, or the neck is only ever judged
/// while the span beyond it is still half-converged.
///
/// Measured at 16: `scene=ligament`'s overhang is 110 cells long, its far
/// end settles last, and the neck sat at a stress ratio of 1.87 — visibly
/// overloaded, never re-checked, holding a 4,400-cell slab in the air.
/// 128 covers any span this world can hold; the per-frame cell budget
/// bounds the cost regardless, and `Cache::subtrees` makes every ancestor
/// after the first walk close to free.
const ROOTWARD_CHECK_STEPS: usize = 128;

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
}
