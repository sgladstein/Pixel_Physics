//! Cells to pixels.
//!
//! The simulation never writes colours. It stores a material id and a shade
//! index, and this module resolves those into RGBA at draw time. Keeping the
//! two apart is what lets M6 swap in a GPU pipeline with lighting and bloom
//! without touching a single movement rule.

use crate::sim::chunk::CHUNK_SIZE;
use crate::sim::world::World;

/// Colour shown for positions outside the world.
const VOID: [u8; 4] = [12, 12, 16, 255];
const CHUNK_BORDER_ACTIVE: [u8; 4] = [80, 200, 120, 255];
const CHUNK_BORDER_SETTLED: [u8; 4] = [60, 60, 70, 255];

pub struct Renderer {
    /// World coordinate displayed at the top-left pixel. Fixed at the origin
    /// for M2; M10 moves it with the player.
    pub camera_x: i32,
    pub camera_y: i32,
    /// Draws chunk boundaries tinted by whether the chunk will be swept next
    /// frame. This is the primary way to confirm sleeping actually works.
    pub show_chunk_overlay: bool,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            camera_x: 0,
            camera_y: 0,
            show_chunk_overlay: false,
        }
    }

    pub fn draw(&self, world: &World, frame: &mut [u8], width: u32, height: u32) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let sx = (i % width as usize) as i32;
            let sy = (i / width as usize) as i32;
            let colour = self.cell_colour(world, sx + self.camera_x, sy + self.camera_y);
            pixel.copy_from_slice(&colour);
        }

        if self.show_chunk_overlay {
            self.draw_chunk_overlay(world, frame, width, height);
        }
    }

    fn cell_colour(&self, world: &World, x: i32, y: i32) -> [u8; 4] {
        if !world.in_bounds(x, y) {
            return VOID;
        }
        let cell = world.get(x, y);
        let palette = &world.materials.get(cell.material).palette;
        // Modulo keeps any shade value valid, so a palette can shrink on hot
        // reload in M3 without invalidating cells already in the world.
        palette[cell.shade as usize % palette.len()]
    }

    fn draw_chunk_overlay(&self, world: &World, frame: &mut [u8], width: u32, height: u32) {
        for chunk in world.chunks() {
            let colour = if chunk.is_settled() {
                CHUNK_BORDER_SETTLED
            } else {
                CHUNK_BORDER_ACTIVE
            };
            let (ox, oy) = chunk.coord.origin();
            let sx = ox - self.camera_x;
            let sy = oy - self.camera_y;
            for i in 0..CHUNK_SIZE {
                put(frame, width, height, sx + i, sy, colour);
                put(frame, width, height, sx, sy + i, colour);
            }
        }
    }

    /// World position under a screen pixel. The inverse of the camera offset
    /// applied in `draw`, used to turn cursor position into a brush position.
    pub fn screen_to_world(&self, sx: i32, sy: i32) -> (i32, i32) {
        (sx + self.camera_x, sy + self.camera_y)
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

fn put(frame: &mut [u8], width: u32, height: u32, x: i32, y: i32, colour: [u8; 4]) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let i = (y as usize * width as usize + x as usize) * 4;
    frame[i..i + 4].copy_from_slice(&colour);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::material;

    #[test]
    fn draws_material_colours_and_void_outside_the_world() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        world.set(0, 0, Cell::new(material::SAND, 0));
        let renderer = Renderer::new();

        // A 128-wide framebuffer over a 64-wide world: the right half is void.
        let (w, h) = (128u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        renderer.draw(&world, &mut frame, w, h);

        let sand = world.materials.get(material::SAND).palette[0];
        assert_eq!(&frame[0..4], &sand);

        // Row 0, column 100 — past the right edge of the 64-wide world.
        let outside = 100 * 4;
        assert_eq!(&frame[outside..outside + 4], &VOID);
    }

    #[test]
    fn screen_to_world_accounts_for_the_camera() {
        let mut r = Renderer::new();
        assert_eq!(r.screen_to_world(5, 7), (5, 7));
        r.camera_x = 100;
        r.camera_y = -20;
        assert_eq!(r.screen_to_world(5, 7), (105, -13));
    }

    #[test]
    fn the_overlay_distinguishes_active_from_settled_chunks() {
        let mut world = World::new(Rect::new(0, 0, 63, 63));
        let mut renderer = Renderer::new();
        renderer.show_chunk_overlay = true;
        let (w, h) = (64u32, 64u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];

        // Freshly built chunks are dirty, so the border reads as active.
        renderer.draw(&world, &mut frame, w, h);
        assert_eq!(&frame[0..4], &CHUNK_BORDER_ACTIVE);

        // Once settled it dims.
        world.end_step();
        renderer.draw(&world, &mut frame, w, h);
        assert_eq!(&frame[0..4], &CHUNK_BORDER_SETTLED);
    }
}
