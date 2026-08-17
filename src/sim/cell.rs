//! The unit of simulation: one pixel of the world.

use super::material::{self, MaterialId};

/// Set on a cell that has just moved, when the sweep will reach its new
/// position again later in the same pass — moving up, or sideways in the
/// direction of the scan. Seeing it, the sweep skips the cell once and clears
/// the flag, so nothing moves twice in one frame.
///
/// This deliberately is *not* a frame-parity bit. Parity requires every cell to
/// be visited every frame to stay in step, which is precisely what dirty
/// rectangles stop doing: a cell skipped for a single frame ends up with a
/// stale parity that aliases with the current one, is skipped forever after,
/// and freezes in mid-air. A flag that is cleared when consumed cannot go stale
/// — at worst a cell that is never revisited waits one extra frame.
const FLAG_MOVED: u8 = 0b0000_0001;

/// Set while a cell is on fire. The remaining burn duration lives in its own
/// `burn_timer` field (not `aux` — see that field's doc for why the two used
/// to be aliased and no longer are).
const FLAG_BURNING: u8 = 0b0000_0010;

/// Set on a `Powder` cell that moved (fell or rolled) during the previous
/// frame it was visited; cleared the first frame it fails to move at all.
/// Read only by `roll_along_slope` (`update.rs`), which uses it to choose
/// between two different repose thresholds — `Reports/granular-mechanics-
/// research.md` §2's two-angle model: a settled pile (this bit clear) can
/// stand at a steeper "maximum stability" angle without creeping, while a
/// pile already in motion (this bit set) keeps flowing until it reaches the
/// shallower, classical "angle of repose" — real hysteresis a single-angle
/// model cannot express. Set generically by `CellSurface::move_cell`'s
/// default implementation on every successful move, not just a powder's —
/// harmless for `Liquid`/`Gas`, since nothing else reads this bit.
const FLAG_FLOWING: u8 = 0b0000_0100;

/// Set on a cell owned by a promoted `liquid::LiquidBody` (`Reports/
/// liquid-heightfield-design.md` §2a/§3c) — the ownership substrate step 1
/// of that design builds first, ahead of any solver. Meaning: the CA sweep
/// must not move this cell, and a write into it from anywhere other than
/// the body's own rasterizer (`World::set_owned`) demotes the owning body.
/// Also set on a body's "container" cells (its bed and walls, immediately
/// outside its own columns) even though those cells hold no fill of the
/// body's own and are never moved by it — the flag there means only "a
/// liquid body depends on you," so digging out the floor under a lake is
/// caught by the identical single-bit test as any other disturbance.
const FLAG_MANAGED: u8 = 0b0000_1000;

/// Set on the cell a mover *vacated*, when that move had a horizontal
/// component — a roll along a slope, a lateral descent, or a diagonal fall.
/// Read by `update_powder` and `update_liquid` (`update.rs`), each of which
/// refuses to let the cell directly above fall straight down into it for
/// the one frame the flag survives.
///
/// This is the "a slumping column may not outrun the material escaping from
/// under it" rule. Rows are swept bottom to top so that a column of falling
/// material descends as a unit — the cell below moves, and every cell above
/// it drops into the vacancy in the same sweep. That is right for a column
/// falling through air, and wrong for a *face*: when a grain escapes
/// sideways off the edge of a pile, the grain above it should topple
/// sideways too, not simply drop into the hole and leave the face standing
/// vertical. Without this, a free face erodes at exactly one grain per
/// frame no matter how tall it is, because the vacancy left by that one
/// grain rides the sweep all the way up the face and every cell above takes
/// the straight-down move in preference to its own sideways escape.
///
/// Deliberately *not* set for a straight-down move (`dx == 0`), which is
/// exactly the case the bottom-to-top sweep exists to serve.
///
/// Cleared by `CellSurface::clear_undercut` on the sweep's next visit, which
/// — since the flagged cell is always *below* the cell it constrains, and
/// rows are swept bottom to top — is always before that cell is asked to
/// fall again. So the refusal lasts exactly one frame and cannot go stale;
/// the same "a flag that is cleared when consumed cannot go stale" argument
/// `FLAG_MOVED` makes above.
///
/// Set generically by `CellSurface::move_cell`, like `FLAG_FLOWING`, rather
/// than only on the powder path — harmless for the kinds that never read it.
const FLAG_UNDERCUT: u8 = 0b0001_0000;

/// A fracture running along this cell's **right** edge — between it and
/// `(x + 1, y)`.
///
/// # Why cracks live on edges rather than on cells
///
/// A crack is a *separation between* two pieces of rock, not a property of
/// either one. Storing damage per cell can only ever erode: the cell weakens
/// until it disappears, which is the dissolve this engine was reported as
/// having. Storing it on the edge means rock can be visibly, cumulatively
/// broken while still being entirely present — the state that was missing.
///
/// Every edge in the world is the right-or-down edge of exactly one cell, so
/// these two bits describe every edge exactly once, with no ambiguity about
/// which cell owns a given crack and no second grid to keep in step. The
/// left edge of `(x, y)` is the right edge of `(x - 1, y)`; its top edge is
/// the down edge of `(x, y - 1)`.
///
/// What makes this worth the two bits: support cannot propagate across a
/// cracked edge. So "a crack has gone all the way around a piece" needs no
/// detection of its own — the piece simply stops being able to reach an
/// anchor, and the structural machinery that already exists fails it and
/// breaks it off. Cracks that do not yet surround anything change nothing
/// structurally, which is exactly right: a scored rock face is still a rock
/// face.
const FLAG_CRACK_RIGHT: u8 = 0b0100_0000;

/// A fracture along this cell's **bottom** edge — between it and
/// `(x, y + 1)`. See `FLAG_CRACK_RIGHT`.
const FLAG_CRACK_DOWN: u8 = 0b1000_0000;

/// Set on material that is part of the **background mass** — the rock the
/// 2D slice is cut through, rather than something standing in front of it.
///
/// This is the engine's answer to a question geometry cannot answer. The
/// play world is a vertical slice through a 3D world
/// (`Reports/worldgen-design.md` §0), so a cave wall is held up largely by
/// rock *out of plane* that the slice will never contain. Two earlier
/// attempts tried to infer that from shape — first "a sufficiently confined
/// cell is anchored", then "thickness scales how far a cell can span" — and
/// both failed the same way: any rule strong enough to hold up a mountain
/// also made everything the player built indestructible, and any rule weak
/// enough to let built structures break also ate the mountain. It is not a
/// property of shape. It is a property of what the material *is*.
///
/// So it is stated instead of guessed at. An attached cell is an anchor
/// outright: it never falls, whatever its shape, because the mass behind it
/// is holding it. Unattached material has to earn its support through a
/// real path, which is what makes a built cantilever a genuine engineering
/// problem.
///
/// **Attachment is lost, never gained, by destruction.** Break a piece out
/// of a cliff and the part that comes free stops being backed by anything —
/// `structural.rs` clears this as it detaches, and that transition is
/// precisely what turns "I dug into a mountain" into falling chunks and
/// rubble. Whether *placing* material can attach it is a separate question
/// this flag does not decide.
const FLAG_ATTACHED: u8 = 0b0010_0000;

/// Default temperature for a newly created cell, in Celsius. Room temperature;
/// chosen so cells created before the M13 ambient field exists still hold a
/// believable value instead of 0 or an extreme.
pub const AMBIENT_TEMPERATURE: i16 = 20;

/// One simulated pixel, widened from 8 to 12 bytes to give the burn timer and
/// organism ownership their own fields instead of both aliasing `aux`.
///
/// The 8-byte version aliased a burning cell's `aux` with its burn timer —
/// harmless while `aux` only ever held a recomputable value (an anchor
/// distance), but a real, confirmed bug for a burning `Liquid` cell: oil is
/// flammable, and the planned fill-amount use of `aux` (see that field's doc)
/// needs to survive a burn instead of being stomped by the timer mid-fire.
/// Fixed by giving the timer its own field. `organism_id` is added in the
/// same widening rather than a second one later — the planned
/// organism-substrate rewrite will hit the identical aliasing problem for a
/// burning `Plant` cell's cell-type tag, so both fields land together now
/// rather than widening `Cell` twice. Same "irrelevant at this scale" cost
/// argument the original 4→8 byte widening (M12) already made: a 2048²
/// world goes 32 MB → 48 MB.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub material: MaterialId,
    /// Index into the material's palette, chosen once when the cell is created
    /// so bulk material has visible grain. Wrapped modulo palette length at
    /// render time, so any value is valid.
    pub shade: u8,
    flags: u8,
    /// Degrees Celsius. Universal rather than kind-specific, because ignition
    /// is local — one wooden beam next to a fire must be able to be hotter
    /// than its identical neighbour three cells away, which a single coarse
    /// ambient value cannot express. The M13 field grid carries a matching
    /// *ambient* temperature that cells exchange heat with; this is the
    /// per-cell value that actually drives ignition and phase changes.
    temperature: i16,
    /// Remaining burn duration in frames, meaningful only while
    /// `is_burning()`; 0 otherwise. Split out from `aux` specifically so
    /// burning never disturbs whatever `aux` is holding — see `aux`'s own
    /// doc.
    burn_timer: u16,
    /// Kind-specific, genuinely kind-specific now that burning no longer
    /// aliases it:
    ///
    /// - `Solid` / `Plant` → distance to the nearest anchor (M17 structural
    ///   integrity, extended to `Plant` by architecture item 9).
    /// - `Creature` → owning creature id (M18).
    /// - `Powder` / `Gas` → unused, always 0.
    /// - `Liquid` → compressible-volume fill amount, on the
    ///   `material::LIQUID_FULL` scale (`update.rs`'s module doc has the
    ///   full model) — the reason this field's independence from
    ///   `burn_timer` mattered enough to widen `Cell` over, since oil is
    ///   both `Liquid` and flammable.
    ///
    /// A tagged union rather than several parallel side tables. Less
    /// elegant, but honest about what these engines actually do, and every
    /// interpretation is written down here rather than scattered across call
    /// sites.
    aux: u16,
    /// Which organism this cell belongs to; 0 means "no organism," matching
    /// the zero-is-empty/inert convention everywhere else in `Cell`. Reserved
    /// here, unused until the organism-substrate rewrite gives it real
    /// readers/writers — added in this widening rather than a second one so
    /// `Cell` only grows once.
    organism_id: u16,
}

impl Cell {
    pub const EMPTY: Cell = Cell {
        material: material::EMPTY,
        shade: 0,
        flags: 0,
        temperature: AMBIENT_TEMPERATURE,
        burn_timer: 0,
        aux: 0,
        organism_id: 0,
    };

    /// The sentinel returned for reads outside the world's bounds. It is solid,
    /// so material treats the edge of the world as a wall and stops there
    /// instead of falling out of it.
    pub const OUT_OF_BOUNDS: Cell = Cell {
        material: material::BEDROCK,
        shade: 0,
        flags: 0,
        temperature: AMBIENT_TEMPERATURE,
        burn_timer: 0,
        aux: 0,
        organism_id: 0,
    };

    pub fn new(material: MaterialId, shade: u8) -> Self {
        Self {
            material,
            shade,
            flags: 0,
            temperature: AMBIENT_TEMPERATURE,
            burn_timer: 0,
            aux: 0,
            organism_id: 0,
        }
    }

    /// A materially-empty cell that is `FLAG_MANAGED` (one of a promoted
    /// liquid body's container cells, still holding `Cell::EMPTY` until
    /// something actually writes into it — `Reports/liquid-heightfield-
    /// design.md` §3c) reads as **not** empty. Found the hard way: without
    /// this, ordinary movement rules (`try_move`'s diagonal case, in
    /// particular) treat a body's own wall as ordinary open air and move
    /// straight into it, which does correctly demote the body via `World::
    /// set`'s own disturbance check — but only *after* the movement has
    /// already won a race against the *intended* path (vertical absorption
    /// into the body's own top cell), so which one fires depends on
    /// incidental scan order rather than on anything physically meaningful.
    /// Worse, in real play a body's wall sits directly beside its own top
    /// surface, so any loose material merely falling *near* a lake — not
    /// into it — would repeatedly graze that wall and demote it. Every
    /// caller of `is_empty` (movement, growth candidates, planting) already
    /// means "is this position available to use," which a reserved
    /// container cell never is, so this is the one place that needs to
    /// change rather than auditing each caller individually.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.material == material::EMPTY && !self.managed()
    }

    /// True when this cell moved into its position during a sweep that will
    /// reach it again. The sweep must skip it once and clear the flag.
    #[inline]
    pub fn moved(self) -> bool {
        self.flags & FLAG_MOVED != 0
    }

    #[inline]
    pub fn with_moved(mut self, moved: bool) -> Self {
        self.set_flag(FLAG_MOVED, moved);
        self
    }

    /// See `FLAG_FLOWING`'s own doc. Meaningful only for `Powder` kind.
    #[inline]
    pub fn flowing(self) -> bool {
        self.flags & FLAG_FLOWING != 0
    }

    #[inline]
    pub fn with_flowing(mut self, flowing: bool) -> Self {
        self.set_flag(FLAG_FLOWING, flowing);
        self
    }

    /// See `FLAG_UNDERCUT`'s own doc. Meaningful only for `Powder` kind.
    #[inline]
    pub fn undercut(self) -> bool {
        self.flags & FLAG_UNDERCUT != 0
    }

    #[inline]
    pub fn with_undercut(mut self, undercut: bool) -> Self {
        self.set_flag(FLAG_UNDERCUT, undercut);
        self
    }

    /// See `FLAG_CRACK_RIGHT` — is this cell separated from `(x + 1, y)`?
    #[inline]
    pub fn crack_right(self) -> bool {
        self.flags & FLAG_CRACK_RIGHT != 0
    }

    #[inline]
    pub fn with_crack_right(mut self, cracked: bool) -> Self {
        self.set_flag(FLAG_CRACK_RIGHT, cracked);
        self
    }

    /// See `FLAG_CRACK_DOWN` — is this cell separated from `(x, y + 1)`?
    #[inline]
    pub fn crack_down(self) -> bool {
        self.flags & FLAG_CRACK_DOWN != 0
    }

    #[inline]
    pub fn with_crack_down(mut self, cracked: bool) -> Self {
        self.set_flag(FLAG_CRACK_DOWN, cracked);
        self
    }

    /// Whether this cell has any fracture on it at all — used by rendering
    /// to darken a scored face, and by the strike tool to find crack tips
    /// worth extending.
    #[inline]
    pub fn cracked(self) -> bool {
        self.flags & (FLAG_CRACK_RIGHT | FLAG_CRACK_DOWN) != 0
    }

    /// See `FLAG_ATTACHED`'s own doc — whether this cell is part of the
    /// background mass, and therefore anchored no matter its shape.
    #[inline]
    pub fn attached(self) -> bool {
        self.flags & FLAG_ATTACHED != 0
    }

    #[inline]
    pub fn with_attached(mut self, attached: bool) -> Self {
        self.set_flag(FLAG_ATTACHED, attached);
        self
    }

    /// See `FLAG_MANAGED`'s own doc.
    #[inline]
    pub fn managed(self) -> bool {
        self.flags & FLAG_MANAGED != 0
    }

    #[inline]
    pub fn with_managed(mut self, managed: bool) -> Self {
        self.set_flag(FLAG_MANAGED, managed);
        self
    }

    #[inline]
    pub fn temperature(self) -> i16 {
        self.temperature
    }

    #[inline]
    pub fn set_temperature(&mut self, celsius: i16) {
        self.temperature = celsius;
    }

    #[inline]
    pub fn with_temperature(mut self, celsius: i16) -> Self {
        self.temperature = celsius;
        self
    }

    #[inline]
    pub fn is_burning(self) -> bool {
        self.flags & FLAG_BURNING != 0
    }

    /// Start burning with the given remaining duration in frames. Writes only
    /// `burn_timer` — `aux` is untouched, so whatever it was holding (a
    /// structural anchor distance today, a liquid's fill amount once that
    /// lands) survives the fire.
    #[inline]
    pub fn ignite(&mut self, duration_frames: u16) {
        self.set_flag(FLAG_BURNING, true);
        self.burn_timer = duration_frames;
    }

    /// Remaining burn duration. Only meaningful when `is_burning()`; returns 0
    /// otherwise so a stale read cannot be mistaken for "still burning".
    #[inline]
    pub fn burn_remaining(self) -> u16 {
        if self.is_burning() {
            self.burn_timer
        } else {
            0
        }
    }

    /// Count the burn timer down by one frame. Clears the burning flag once
    /// it reaches zero; `aux` was never touched by burning in the first
    /// place, so there is nothing to fall through to or recompute here.
    #[inline]
    pub fn tick_burn(&mut self) {
        debug_assert!(self.is_burning(), "tick_burn called on a non-burning cell");
        self.burn_timer = self.burn_timer.saturating_sub(1);
        if self.burn_timer == 0 {
            self.set_flag(FLAG_BURNING, false);
        }
    }

    #[inline]
    pub fn extinguish(&mut self) {
        self.set_flag(FLAG_BURNING, false);
        self.burn_timer = 0;
    }

    /// Kind-specific `aux` value — see the field doc for what each kind
    /// stores here. No longer conditional on burning state; burning has its
    /// own field now.
    #[inline]
    pub fn aux(self) -> u16 {
        self.aux
    }

    /// Set the kind-specific `aux` value. Safe to call regardless of burning
    /// state now that the two no longer alias — the old debug-assert guard
    /// against corrupting an in-progress burn no longer applies.
    ///
    /// **Hazard for `Liquid` cells specifically**
    /// (`Reports/liquid-simulation-research-r2.md` §3d): `aux == 0` there
    /// means "untouched since creation, treat as full," not empty (see
    /// `material::LIQUID_FULL`'s own doc) — writing `0` on a live `Liquid`
    /// cell silently manufactures a full cell from nothing. Use
    /// `Cell::EMPTY` instead of `cell.set_aux(0)` when a liquid cell is
    /// meant to become genuinely empty. `Cell` has no `MaterialRegistry`
    /// access by design, so this can't be enforced here for every kind at
    /// once; `update.rs`'s `write_liquid_transfer` carries the real guard
    /// via a `debug_assert` at its own two call sites.
    #[inline]
    pub fn set_aux(&mut self, value: u16) {
        self.aux = value;
    }

    #[inline]
    pub fn with_aux(mut self, value: u16) -> Self {
        self.set_aux(value);
        self
    }

    /// Which organism owns this cell; 0 means none. See the field's own
    /// doc — this is what gates `aux`'s interpretation for a `Plant`/
    /// `Creature` cell between "inert material, `aux` holds an anchor
    /// distance the M17 way" and "organism tissue, `aux` holds a cell-type
    /// tag plus a resource scalar" (`organism.rs`'s `CellType`/`resource`
    /// helpers below). `Cell` itself stays agnostic to which — same
    /// "the meaning lives with the caller, not the type" shape `aux`
    /// itself already has for `Solid` vs `Creature` vs everything else.
    #[inline]
    pub fn organism_id(self) -> u16 {
        self.organism_id
    }

    #[inline]
    pub fn set_organism_id(&mut self, id: u16) {
        self.organism_id = id;
    }

    #[inline]
    pub fn with_organism_id(mut self, id: u16) -> Self {
        self.set_organism_id(id);
        self
    }

    #[inline]
    fn set_flag(&mut self, flag: u8, on: bool) {
        if on {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_is_twelve_bytes() {
        // Guards the memory budget: 8 bytes plus a dedicated burn timer and
        // organism-ownership field, both previously aliased into `aux`.
        assert_eq!(std::mem::size_of::<Cell>(), 12);
    }

    #[test]
    fn moved_flag_round_trips_without_disturbing_the_cell() {
        let c = Cell::new(material::SAND, 2);
        assert!(!c.moved(), "a fresh cell must not read as having moved");

        let c = c.with_moved(true);
        assert!(c.moved());
        assert_eq!(c.material, material::SAND);
        assert_eq!(c.shade, 2);

        let c = c.with_moved(false);
        assert!(!c.moved());
        assert_eq!(c.material, material::SAND);
        assert_eq!(c.shade, 2);
    }

    #[test]
    fn new_cells_start_at_ambient_temperature_with_no_aux() {
        let c = Cell::new(material::STONE, 0);
        assert_eq!(c.temperature(), AMBIENT_TEMPERATURE);
        assert_eq!(c.aux(), 0);
        assert!(!c.is_burning());
    }

    #[test]
    fn temperature_round_trips_and_can_go_negative() {
        // Signed so a future ice/freezing mechanic has headroom without a
        // representation change.
        let c = Cell::new(material::WATER, 0).with_temperature(-15);
        assert_eq!(c.temperature(), -15);
    }

    #[test]
    fn aux_round_trips_when_not_burning() {
        let c = Cell::new(material::STONE, 0).with_aux(42);
        assert_eq!(c.aux(), 42);
    }

    #[test]
    fn igniting_a_cell_no_longer_touches_aux() {
        // The regression this widening exists to fix: an oil cell (Liquid,
        // flammable) is a real material that will soon need its aux-held
        // value (a planned fill amount) to survive being set on fire, not
        // just a hypothetical. Confirmed by temporarily reverting `ignite`
        // to write `self.aux` instead of `self.burn_timer` and rerunning
        // this test: it fails (`burn_remaining()` reads 0 instead of 180,
        // since `burn_timer` was never actually set) — a genuine failure,
        // just surfaced through a different assertion than the aux ones
        // below, which is exactly the old aliasing bug in a new location.
        let mut c = Cell::new(material::OIL, 0).with_aux(99);
        c.ignite(180);
        assert!(c.is_burning());
        assert_eq!(c.burn_remaining(), 180);
        assert_eq!(c.aux(), 99, "aux must survive ignition completely unchanged");

        c.tick_burn();
        assert_eq!(c.aux(), 99, "aux must survive mid-burn ticking too");

        // Burn it out completely and confirm aux is still untouched.
        for _ in 0..179 {
            c.tick_burn();
        }
        assert!(!c.is_burning());
        assert_eq!(c.aux(), 99, "aux must survive burnout, not reset to 0 or the old timer value");
    }

    #[test]
    fn burn_remaining_is_zero_when_not_burning() {
        // Guards against a stale timer value being mistaken for "still on
        // fire" after extinguishing.
        let mut c = Cell::new(material::OIL, 0);
        c.ignite(10);
        c.extinguish();
        assert_eq!(c.burn_remaining(), 0);
        assert!(!c.is_burning());
    }

    #[test]
    fn tick_burn_counts_down_and_clears_at_zero() {
        let mut c = Cell::new(material::OIL, 0);
        c.ignite(2);
        c.tick_burn();
        assert!(c.is_burning());
        assert_eq!(c.burn_remaining(), 1);
        c.tick_burn();
        assert!(!c.is_burning(), "cell should stop burning when the timer hits zero");
        assert_eq!(c.burn_remaining(), 0, "burn_remaining should read 0 once out, not the stale timer value");
    }

    #[test]
    fn tick_burn_never_underflows() {
        // saturating_sub, not wrapping: if a caller ever does tick past zero
        // in a single frame (two systems both advancing the same cell), it
        // must not wrap to u16::MAX and leave the cell burning forever.
        // Bypasses the normal is_burning() guard to exercise that directly.
        let mut c = Cell::new(material::OIL, 0);
        c.ignite(1);
        c.tick_burn(); // 1 -> 0, clears FLAG_BURNING
        // Re-set the flag without going through ignite(), so tick_burn sees
        // is_burning() == true but aux is already 0 — the underflow case.
        c.set_flag(FLAG_BURNING, true);
        c.tick_burn();
        assert!(!c.is_burning());
        assert_eq!(c.burn_remaining(), 0);
    }

    #[test]
    fn calling_tick_burn_on_a_non_burning_cell_is_a_bug() {
        // The debug_assert exists so a caller that forgets to check
        // is_burning() first fails loudly in tests rather than silently
        // corrupting aux. Only meaningful in debug builds.
        if !cfg!(debug_assertions) {
            return;
        }
        let result = std::panic::catch_unwind(|| {
            let mut c = Cell::new(material::OIL, 0);
            c.tick_burn();
        });
        assert!(result.is_err(), "tick_burn should have asserted on a non-burning cell");
    }

    #[test]
    fn out_of_bounds_sentinel_is_not_empty() {
        // Material must never treat the world edge as free space.
        assert!(!Cell::OUT_OF_BOUNDS.is_empty());
    }

    #[test]
    fn out_of_bounds_sentinel_is_at_ambient_temperature() {
        // Otherwise a body of lava against the world edge would see an
        // artificial heat sink or source depending on what garbage temperature
        // the sentinel happened to have.
        assert_eq!(Cell::OUT_OF_BOUNDS.temperature(), AMBIENT_TEMPERATURE);
    }
}
