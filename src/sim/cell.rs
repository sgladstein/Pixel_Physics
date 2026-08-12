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

/// Set while a cell is on fire. While this is set, `aux` holds the remaining
/// burn duration in frames rather than whatever its material kind would
/// otherwise store there — see the note on `aux` below.
const FLAG_BURNING: u8 = 0b0000_0010;

/// Default temperature for a newly created cell, in Celsius. Room temperature;
/// chosen so cells created before the M13 ambient field exists still hold a
/// believable value instead of 0 or an extreme.
pub const AMBIENT_TEMPERATURE: i16 = 20;

/// One simulated pixel, widened from 4 to 8 bytes (M12) to give every cell a
/// temperature and a spare field for kind-specific state.
///
/// A 2048x2048 world costs 32 MB at this size — double M2's 4-byte cell, still
/// small enough to keep whole chunks comfortably inside cache during a sweep.
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
    /// Meaning depends on the cell's current state, checked in this order:
    ///
    /// 1. **`is_burning()`** — remaining burn duration in frames, countdown to
    ///    zero. Takes priority over the kind-specific meaning below because
    ///    combustion is transient: whatever the cell is about to become when
    ///    the fire finishes, its steady-state `aux` value will need
    ///    recomputing anyway, so nothing of value is preserved by protecting it.
    /// 2. **Otherwise, by `MaterialKind`**:
    ///    - `Solid` / `Plant` → distance to the nearest anchor (M17
    ///      structural integrity, extended to `Plant` by architecture
    ///      item 9). `Plant` originally reserved this slot for a per-cell
    ///      growth stage instead; that never got built, since real per-tip
    ///      growth state lives in `TreeState`/`Tip`/`RootTip` (`plant.rs`)
    ///      instead — it doesn't fit in a `u16` regardless (attractor
    ///      lists, channel strength) — so the slot was genuinely free.
    ///    - `Creature` → owning creature id (M18)
    ///    - `Powder` / `Liquid` / `Gas` → unused, always 0
    ///
    /// A tagged union rather than four parallel side tables. Less elegant, but
    /// honest about what these engines actually do, and every interpretation is
    /// written down here rather than scattered across call sites.
    aux: u16,
}

impl Cell {
    pub const EMPTY: Cell = Cell {
        material: material::EMPTY,
        shade: 0,
        flags: 0,
        temperature: AMBIENT_TEMPERATURE,
        aux: 0,
    };

    /// The sentinel returned for reads outside the world's bounds. It is solid,
    /// so material treats the edge of the world as a wall and stops there
    /// instead of falling out of it.
    pub const OUT_OF_BOUNDS: Cell = Cell {
        material: material::BEDROCK,
        shade: 0,
        flags: 0,
        temperature: AMBIENT_TEMPERATURE,
        aux: 0,
    };

    pub fn new(material: MaterialId, shade: u8) -> Self {
        Self {
            material,
            shade,
            flags: 0,
            temperature: AMBIENT_TEMPERATURE,
            aux: 0,
        }
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.material == material::EMPTY
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

    /// Start burning with the given remaining duration in frames. Overwrites
    /// `aux`, since a burning cell's `aux` is always the burn timer regardless
    /// of what its material kind would otherwise store there.
    #[inline]
    pub fn ignite(&mut self, duration_frames: u16) {
        self.set_flag(FLAG_BURNING, true);
        self.aux = duration_frames;
    }

    /// Remaining burn duration. Only meaningful when `is_burning()`; returns 0
    /// otherwise so a stale read cannot be mistaken for "still burning".
    #[inline]
    pub fn burn_remaining(self) -> u16 {
        if self.is_burning() {
            self.aux
        } else {
            0
        }
    }

    /// Count the burn timer down by one frame. Clears the burning flag and
    /// zeroes `aux` when it reaches zero, so the cell falls through to its
    /// kind-specific `aux` meaning (starting from 0, i.e. recomputed fresh)
    /// rather than leaving a stale timer value behind.
    #[inline]
    pub fn tick_burn(&mut self) {
        debug_assert!(self.is_burning(), "tick_burn called on a non-burning cell");
        self.aux = self.aux.saturating_sub(1);
        if self.aux == 0 {
            self.set_flag(FLAG_BURNING, false);
        }
    }

    #[inline]
    pub fn extinguish(&mut self) {
        self.set_flag(FLAG_BURNING, false);
        self.aux = 0;
    }

    /// Kind-specific `aux` value. Only meaningful when the cell is not
    /// burning — see the field doc on `aux` for what each kind stores here.
    #[inline]
    pub fn aux(self) -> u16 {
        self.aux
    }

    /// Set the kind-specific `aux` value. Must not be called on a burning
    /// cell, since that would corrupt the burn timer; `debug_assert`s rather
    /// than silently overwriting, because a caller hitting this has a bug
    /// worth seeing immediately rather than losing fire state quietly.
    #[inline]
    pub fn set_aux(&mut self, value: u16) {
        debug_assert!(!self.is_burning(), "set_aux called on a burning cell");
        self.aux = value;
    }

    #[inline]
    pub fn with_aux(mut self, value: u16) -> Self {
        self.set_aux(value);
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
    fn cell_is_eight_bytes() {
        // Guards the memory budget that makes large worlds affordable: double
        // M2's 4 bytes, in exchange for a per-cell temperature and aux slot.
        assert_eq!(std::mem::size_of::<Cell>(), 8);
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
    fn igniting_a_cell_overwrites_aux_with_the_burn_timer() {
        let mut c = Cell::new(material::OIL, 0).with_aux(99);
        c.ignite(180);
        assert!(c.is_burning());
        assert_eq!(c.burn_remaining(), 180);
        // The prior aux value (a stand-in for e.g. a structural distance) is
        // gone — burning cells always read aux as the timer.
        assert_eq!(c.aux(), 180);
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
        assert_eq!(c.aux(), 0, "aux should be cleared, not left at the stale timer value");
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
