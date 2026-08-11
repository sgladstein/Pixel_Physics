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

/// Exactly 4 bytes: a 2048x2048 world costs 16 MB, which keeps whole chunks
/// comfortably inside cache during a sweep.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub material: MaterialId,
    /// Index into the material's palette, chosen once when the cell is created
    /// so bulk material has visible grain. Wrapped modulo palette length at
    /// render time, so any value is valid.
    pub shade: u8,
    flags: u8,
}

impl Cell {
    pub const EMPTY: Cell = Cell {
        material: material::EMPTY,
        shade: 0,
        flags: 0,
    };

    /// The sentinel returned for reads outside the world's bounds. It is solid,
    /// so material treats the edge of the world as a wall and stops there
    /// instead of falling out of it.
    pub const OUT_OF_BOUNDS: Cell = Cell {
        material: material::BEDROCK,
        shade: 0,
        flags: 0,
    };

    pub fn new(material: MaterialId, shade: u8) -> Self {
        Self {
            material,
            shade,
            flags: 0,
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
        if moved {
            self.flags |= FLAG_MOVED;
        } else {
            self.flags &= !FLAG_MOVED;
        }
        self
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
    fn cell_is_four_bytes() {
        // Guards the memory budget that makes large worlds affordable.
        assert_eq!(std::mem::size_of::<Cell>(), 4);
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
    fn out_of_bounds_sentinel_is_not_empty() {
        // Material must never treat the world edge as free space.
        assert!(!Cell::OUT_OF_BOUNDS.is_empty());
    }
}
