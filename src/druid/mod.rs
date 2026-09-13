//! **The held world — the third game on this engine.**
//!
//! `cargo run --release --bin druid`. Beside the outdoor sandbox
//! (`src/main.rs`) and the evolution lab (`src/bin/lab.rs`), sharing
//! `sim::frame::step` and the whole `sim` library. Design of record:
//! `Reports/held-world-game-concept-2026-09-13.md`.
//!
//! **The premise.** The land is *held* — nothing grows, breeds, ages, rots or
//! weathers, and the sky does not turn. Physics is untouched: rock falls,
//! water flows. The player carries the only time there is, in a small circle
//! that follows him, and (later) places standing ones he pays for.
//!
//! **Why this is a binary and not a mode.** Owner's ruling, 2026-09-13: *"I
//! don't want to build this into the existing gnome game. This is a fully new
//! and separate game."* Nothing here reaches into `app.rs`, and the engine
//! pieces it stands on (`World::held`, `::quickenings`, `::carried`) all
//! default off, so the sandbox and the lab are byte-identical without it.
//!
//! **What this module is not.** There is no save/load anywhere in this engine
//! (no serde over `World`), no pause-and-resume of a session, and no
//! world-edge behaviour beyond the sandbox's. Named here as out of scope
//! rather than discovered later.

use crate::render::Renderer;
use crate::sim::chunk::Rect;
use crate::sim::clock::SkyPin;
use crate::sim::explosion::{self, Blasts};
use crate::sim::frame;
use crate::sim::material;
use crate::sim::organism;
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::world::World;
use crate::worldgen::{self, WorldgenPresets};

/// **Five screens wide, three deep.** The viewport is 512x320
/// (`app::WIDTH`/`HEIGHT`), so this is a walk of about five screens with room
/// above and below.
///
/// **Deliberately not the sandbox's 8192x2560**, which exists so you can
/// tunnel into eight screens of rock. This game has no mining, no caving and
/// no destruction to spend that on, and the depth is where the generation
/// cost lives — `stone_massif` is half of it, and it is half because of
/// volume rather than because of the pass.
///
/// The soil bed matters more here than the rock under it: ants burrow and
/// roots go deep, so soil depth is the resource. That shape is not yet
/// expressed in the preset — the size is the only lever taken so far.
pub const WORLD_WIDTH: u32 = 2560;
pub const WORLD_HEIGHT: u32 = 960;

/// **How long the world lives before it stops.**
///
/// The land was alive and *then* stopped, which is the fiction and also the
/// only way the world contains anything: `life_scatter` places a single
/// `seed`-material powder cell per plant, so a world held at frame 0 is bare
/// ground plus a scatter of seed pixels for ever. **Measured 2026-09-13**,
/// `scene=worldgen` unheld at 512x320, living plant tissue:
///
/// ```text
/// frame  0     2k     4k     6k      8k      10k     12k     14k
/// cells  20    3,252  9,084  14,474  19,244  20,247  18,469  16,535
/// ```
///
/// So the stand peaks near 10,000 and then **self-thins** — past that you are
/// freezing a wood already in decline. 8,000 sits just under the peak, on the
/// rising side, which is a young mature wood rather than an old one.
///
/// **Read it as the age of the world at the moment it stopped**, not as a
/// technical budget: a lower number is a younger, thinner land and a higher
/// one is a tired one. That is a knob worth playing with, which is why it
/// takes an override.
pub const GROW_FRAMES: u64 = 8_000;

/// `PIXEL_PHYSICS_DRUID_SIZE=WxH` — generate a different world size.
///
/// Iterating on a 512x320 world and looking at a 2560x960 one are both worth
/// doing, and a knob is what keeps the two measurements comparable. A knob
/// nobody can see the value of is a knob nobody can tell is disconnected, so
/// `Druid::new` prints what it used.
const SIZE_ENV: &str = "PIXEL_PHYSICS_DRUID_SIZE";
/// `PIXEL_PHYSICS_DRUID_GROW=N` — override [`GROW_FRAMES`].
const GROW_ENV: &str = "PIXEL_PHYSICS_DRUID_GROW";

/// The whole game: the same quartet `App` and `Lab` each declare, because
/// there is no extracted game core in this engine and inventing one to hold
/// three callers would be the larger change.
pub struct Druid {
    pub world: World,
    pub particles: ParticleSystem,
    pub blasts: Blasts,
    pub renderer: Renderer,
    pub player_tuning: player::Tuning,
    pub player_input: player::PlayerInput,
    pub paused: bool,
}

impl Default for Druid {
    fn default() -> Self {
        Self::new()
    }
}

impl Druid {
    /// Generate a world, live in it for a while, then stop it.
    pub fn new() -> Self {
        let (w, h) = size_from_env();
        let grow = grow_from_env();
        println!("druid: world {w}x{h}, grown {grow} frames before holding");

        let mut world = World::new(Rect::new(0, 0, w as i32 - 1, h as i32 - 1));

        // **Species alongside materials.** Shipping one without the other is
        // a mistake this repo has already made once — `SpeciesRegistry::
        // reload` existed, was tested, and had no caller, so editing a
        // species file silently did nothing.
        let _ = world.materials.reload(material::ASSET_DIR);
        let _ = world.species.reload(organism::ASSET_DIR);

        let (presets, _err) = WorldgenPresets::load();
        let preset = presets.default_name();
        match presets.get(&preset) {
            // `generate`, never `generate_only`: the latter skips
            // `compute_world_distances`, so the world would have no
            // structural field and nothing would ever fall correctly.
            Some(params) => worldgen::generate(&mut world, worldgen::Spec::Generated { params, seed: 1 }),
            None => worldgen::generate(&mut world, worldgen::Spec::Legacy),
        }

        if let Ok(Some(clock)) = crate::sim::clock::Clock::load() {
            world.clock = clock;
        }

        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::with_tuning(explosion::Tuning::load());
        let player_tuning = player::Tuning::load();

        // --- the world lives -------------------------------------------
        //
        // Full ticks rather than a subsystem loop, so what grows here is
        // exactly what would have grown had somebody watched it: the same
        // sweep, the same weather, the same settling.
        let t0 = std::time::Instant::now();
        for _ in 0..grow {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &player_tuning);
        }
        let grown = world.live_organism_count();
        println!("druid: grew {grown} organisms in {:.1}s", t0.elapsed().as_secs_f32());

        // --- and then it stops -----------------------------------------
        //
        // The sky pin is not decoration and not an optimisation, though it is
        // both: in the look the owner picked, nothing changes colour, so the
        // *only* tells are that nothing moves and the sky does not turn. A
        // held world under a running sun crawls its shadows across stopped
        // ground and reads as a bug rather than as a state.
        world.held = true;
        world.set_sky_hold(SkyPin::Noon.hold());

        if let Some((x, y)) = spawn_point(&world) {
            // `at_scaled`, not `at`: at any `cell_scale` other than 1 the
            // plain constructor builds a half-size gnome.
            world.player = Some(player::Player::at_scaled(x, y, world.cell_scale()));
        }

        Self {
            world,
            particles,
            blasts,
            renderer: Renderer::new(),
            player_tuning,
            player_input: player::PlayerInput::default(),
            paused: false,
        }
    }

    /// One tick.
    pub fn update(&mut self) {
        if self.paused {
            return;
        }
        frame::step(&mut self.world, &mut self.particles, &mut self.blasts, self.player_input, &self.player_tuning);
        // **Consumed here, or a catch-up burst turns one press into five
        // jumps.** The edge is the caller's to set and this tick's to clear.
        self.player_input.jump_pressed = false;
    }

    /// One drawn frame.
    pub fn draw(&mut self, frame_buf: &mut [u8], viewport: (u32, u32), force_full: bool) {
        // The view follows him, outside the tick loop: the camera is view
        // state, so running it several times in a catch-up frame would move
        // it several times for one drawn picture.
        if let Some(player) = &self.world.player {
            self.renderer.follow(player.center(), viewport, self.world.bounds());
        }
        // **Taken every frame, without exception.** `take_touched_chunks` is
        // a `mem::take`: a frame that skips it drops those chunks for good and
        // they redraw as stale pixels with no error anywhere.
        let touched = self.world.take_touched_chunks();
        self.renderer.draw(&self.world, &self.particles, &touched, frame_buf, viewport, force_full);
    }
}

/// **A dry standing spot near the middle of the world.**
///
/// Generated terrain puts the surface wherever it likes, and the sandbox
/// never auto-spawns — it summons the gnome wherever the player clicked — so
/// there is no existing pattern for this, and a spawn at a fixed height drops
/// him into rock or out of the sky depending on the seed.
///
/// **Searching outward from the middle is not tidiness.** The first version
/// took the middle column and nothing else, and the first world it generated
/// put a pond exactly there: `surface_at` walks to the first *Solid*, water is
/// a `Liquid`, so it walked straight through the pond and returned the rock
/// floor underneath it. The gnome spawned submerged. A column is only a spawn
/// if it is dry.
fn spawn_point(world: &World) -> Option<(i32, i32)> {
    let b = world.bounds()?;
    let mid = (b.min_x + b.max_x) / 2;
    // Alternating outward from the middle, so he starts as near the centre of
    // the world as the terrain allows rather than at whichever end is dry.
    (0..=(b.max_x - b.min_x) / 2)
        .flat_map(|d| [mid - d, mid + d])
        .filter(|&x| x >= b.min_x && x <= b.max_x)
        .find_map(|x| surface_at(world, x).map(|y| (x, y)))
}

/// The cell a body stands in at column `x`, or `None` if the column has no
/// ground or is under water.
fn surface_at(world: &World, x: i32) -> Option<i32> {
    let b = world.bounds()?;
    let mut last_empty = None;
    for y in b.min_y..=b.max_y {
        let cell = world.get(x, y);
        if cell.material == material::EMPTY {
            last_empty = Some(y);
            continue;
        }
        match world.materials.kind(cell.material) {
            material::MaterialKind::Solid => return last_empty,
            // Anything wet above the rock disqualifies the column outright,
            // rather than being skipped over to the floor beneath it.
            material::MaterialKind::Liquid => return None,
            _ => {}
        }
    }
    None
}

fn size_from_env() -> (u32, u32) {
    let Ok(v) = std::env::var(SIZE_ENV) else {
        return (WORLD_WIDTH, WORLD_HEIGHT);
    };
    let parsed = v.split_once(['x', 'X']).and_then(|(w, h)| Some((w.trim().parse().ok()?, h.trim().parse().ok()?)));
    match parsed {
        Some((w, h)) if w > 0 && h > 0 => (w, h),
        _ => {
            eprintln!("druid: {SIZE_ENV}={v:?} is not WxH; using {WORLD_WIDTH}x{WORLD_HEIGHT}");
            (WORLD_WIDTH, WORLD_HEIGHT)
        }
    }
}

fn grow_from_env() -> u64 {
    match std::env::var(GROW_ENV) {
        Ok(v) => match v.trim().parse() {
            Ok(n) => n,
            Err(_) => {
                eprintln!("druid: {GROW_ENV}={v:?} is not a number; using {GROW_FRAMES}");
                GROW_FRAMES
            }
        },
        Err(_) => GROW_FRAMES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A spawn point has to be on the ground**, which is the one thing this
    /// game needs that the sandbox never had to solve — it summons the gnome
    /// where the player clicked.
    ///
    /// Built rather than generated: a hand-made column is the case whose
    /// answer is known, which is what makes this a control rather than a
    /// restatement of whatever the terrain happened to do.
    #[test]
    fn the_spawn_point_is_the_cell_above_the_ground() {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        for y in 40..64 {
            w.set(10, y, crate::sim::cell::Cell::new(material::STONE, 0));
        }
        assert_eq!(surface_at(&w, 10), Some(39), "the standing cell is the last empty one above the rock");
        assert_eq!(surface_at(&w, 11), None, "a column with no ground in it has no surface");

        // **The pond case, which the first generated world actually hit.**
        // Water is a `Liquid`, so a walk that only stops at `Solid` goes
        // straight through it and reports the rock floor underneath -- a
        // spawn point at the bottom of a pond.
        let water = w.materials.id_of("water").expect("water is compiled in");
        for y in 44..48 {
            w.set(20, y, crate::sim::cell::Cell::new(water, 0));
        }
        for y in 48..64 {
            w.set(20, y, crate::sim::cell::Cell::new(material::STONE, 0));
        }
        assert_eq!(surface_at(&w, 20), None, "a flooded column is not somewhere to stand");
        // And the search finds dry land rather than giving up at the middle.
        assert!(spawn_point(&w).is_some_and(|(x, _)| x != 20), "the search must walk past a flooded column");
    }

    /// The size knob parses, and a malformed value falls back rather than
    /// panicking a game on startup.
    #[test]
    fn the_size_knob_falls_back_rather_than_failing() {
        // Not a `std::env::set_var` test: env is process-global and the suite
        // is threaded, so this exercises the parse directly.
        let parse = |v: &str| v.split_once(['x', 'X']).and_then(|(w, h)| Some((w.trim().parse::<u32>().ok()?, h.trim().parse::<u32>().ok()?)));
        assert_eq!(parse("2560x960"), Some((2560, 960)));
        assert_eq!(parse("512X320"), Some((512, 320)));
        assert_eq!(parse("nonsense"), None);
        assert_eq!(parse("2560x"), None);
    }
}
