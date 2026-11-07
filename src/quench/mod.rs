//! **Quench: aim the roof, not the water.** `cargo run --release --bin quench`
//!
//! Probe 5 of the 2026-11-07 new-game brainstorm
//! (`/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/5-quench.md`):
//! lava pours down a gorge toward a village, and a forge at the bottom needs a
//! trickle of it. The player throws water and places stone lids. Steam under a
//! lid drips straight back onto the lava, so one splash becomes a small quench
//! engine that builds stone where the roof is.
//!
//! **The question this answers is about fun, not physics**: does the engine's
//! own chemistry make a combo (Lid, then Splash under it) read as a clever play?
//! So this is the least that lets somebody feel that verb. Everything here is
//! game code; the engine changes it needs are `World::set_game_roofed` and
//! `World::crust_quench_depth`, both inert unless this game sets them.
//!
//! # Round 2 (2026-11-07): the dam works, mostly because of the level
//!
//! Two lids stacked over the dam site with water under each let 71 cells of
//! lava past, lids alone ~187, nothing ~210, water alone ~310 (ten seeds,
//! `Reports/quench-probe-2026-11-07.md`). The levers, by size: lids must hang
//! clear ([`LID_CLEARANCE`]), a sill keeps splash water out of the forge,
//! the flood is off, and water quenches lava through thin crust
//! ([`CRUST_DEPTH`]). On the shipped engine the same plan lets 115 past.
//!
//! # Why the lid needs a game-declared roof (measured 2026-11-07)
//!
//! `examples/quenchfact.rs`, one 81-cell splash on a 55x12 lava pool under a
//! 69-wide, 4-thick stone lid 14 rows up:
//!
//! | arm | water sent to the sky |
//! |---|---|
//! | no lid | 81.0 of 81 cells |
//! | lid present before frame one | 81.0 |
//! | lid placed after frame one | 81.0 |
//! | lid placed after, shadow declared roofed | 12.2 by 2 s (rest evaporates slowly off the hot crust) |
//!
//! The engine decides "indoors" from a map frozen on the first frame, and
//! that map lets the open sky reach 32 cells in under cover, so no lid a
//! player could place counts as a roof. The wiki's "anything with ground
//! above it" is true of caves, not of lids.
//!
//! # The two engine facts the brief asked to check first
//!
//! - **The drip does not keep quenching a still pool.** The splash quenches
//!   for ~0.2 s, and the crust it makes separates the rest of the water from
//!   the lava; the condensate then drips onto crust. On a *moving* flow the
//!   lava keeps arriving under the drip, which is why the level is a gorge
//!   with a vent rather than a pool.
//! - **Crust: the shipped rule is kept** (it crumbles to rubble and sinks).
//!   No second crust material.

use crate::sim::cell::Cell;
use crate::sim::chunk::Rect;
use crate::sim::clock::SkyPin;
use crate::sim::explosion::Blasts;
use crate::sim::material::{self, MaterialId, MaterialKind};
use crate::sim::particle::ParticleSystem;
use crate::sim::weather::Weather;
use crate::sim::world::World;
use crate::sim::{frame, player};

/// World size, the phone-readable size `common.md` asks every probe to use.
pub const W: i32 = 256;
pub const H: i32 = 160;
/// The simulation's fixed rate, as `main.rs` runs it.
pub const TICKS_PER_SECOND: u64 = 60;

/// Seconds the vent pours lava for.
pub const VENT_SECONDS: u64 = 75;
/// Seconds at which the flood arrives.
pub const FLOOD_AT: u64 = 100;
/// Ticks between molten cells from the vent. Measured 2026-11-07: three a
/// tick drowned the whole gorge to the brim by 30 s.
pub const VENT_EVERY: u64 = 4;
/// Ticks between flood rows (`flood_src` is 20 cells, so 20 cells every
/// this many ticks). Every 2 ticks put ~4,800 cells in and drowned the whole
/// map to two-thirds height (2026-11-07).
pub const FLOOD_EVERY: u64 = 6;
/// Seconds the flood pours for.
pub const FLOOD_SECONDS: u64 = 8;
/// Seconds at which the round is scored and ends.
pub const ROUND_SECONDS: u64 = 135;

/// The water tank the player throws from, in cells.
pub const TANK: i32 = 400;
/// One bead of steam under a roof in this many goes back to the tank.
pub const CATCH_ONE_IN: u32 = 3;
/// Water one Splash throws.
pub const SPLASH_RADIUS: i32 = 4;
/// Lids the player has for the round.
pub const LIDS: u32 = 4;
/// A lid's size in cells.
pub const LID_W: i32 = 24;
pub const LID_H: i32 = 3;
/// Rows of skirt down each end of a lid.
pub const LID_SKIRT: i32 = 3;
/// Rows under a lid's skirts that must be open for it to be placed.
///
/// **Measured 2026-11-07 (round 2): a lid whose skirts came within one row of
/// the floor was a wall without any water** -- lava through a one-row gap
/// crusts shut on its own, so the lids-only control held back exactly as much
/// lava as lids with water (122 against 125 cells past the dam). Four open
/// rows under the skirts is a gap lava flows through until water seals it.
pub const LID_CLEARANCE: i32 = 4;
/// Dig radius.
pub const DIG_RADIUS: i32 = 3;
/// Molten cells in the forge for it to count as burning.
pub const FORGE_NEEDS: u32 = 6;
/// Ticks between molten cells the forge burns. The vent gives one every
/// `VENT_EVERY` (4), so the forge alone takes two-thirds of the pour.
pub const FORGE_EATS_EVERY: u64 = 6;
/// Seconds of a burning forge that score full marks.
pub const FORGE_FULL_SECONDS: f32 = 45.0;
/// Lava (molten, or the loose stone it became) at or right of this column has
/// got past the dam site: [`Counts::past`].
pub const PAST_X: i32 = 175;
/// How many cells of crust water quenches lava through
/// (`World::crust_quench_depth`); 0 is the shipped rule. See
/// `fire::try_react_through_crust` for why this game needs it.
pub const CRUST_DEPTH: u8 = 4;

/// **Steam under a roof beads on the roof's underside and drips at once.**
///
/// The game's one rule on top of the engine, and the probe's switch
/// ([`Quench::fast_drip`], key `F`). Returns how many cells beaded.
///
/// Measured first without it (`examples/quenchfact.rs`, a lava stream on a
/// flat floor, one radius-4 splash, five seeds, means): the shipped engine
/// condenses steam only once it has cooled to 45C, and steam born off lava at
/// ~1000C under a lid six rows up runs sideways out from under the lid long
/// before then. A flat lid gave **4 drips** and ran 2.0 s against the open
/// splash's 1.3 s; a cup-shaped lid (three-row skirts) **41 drips**, 5 s and
/// +27 rock in the site; a lid held at 15C made no difference (5 drips),
/// because heat crosses from a gas slowly. So the lid-and-drip engine the
/// design leans on does not run on the shipped chemistry -- the drip
/// direction is right, the timing is not.
///
/// This turns condensation on contact with the roof into the rule *inside a
/// declared roof only* -- a cell [`World::set_game_roofed`] marked whose upper
/// neighbour is solid -- so it touches nothing outside this game. Fill is
/// carried across exactly as `fire::transform` carries it (steam's `aux` is
/// its source water's fill), so the water is moved and not made.
pub fn bead_under_roofs(world: &mut World, steam: MaterialId) -> u32 {
    bead_under_roofs_catching(world, steam, |_, _| false).0
}

/// [`bead_under_roofs`], with `catch` asked of every bead: `true` takes the
/// water out of the world (the game puts it back in the player's tank)
/// instead of letting it drip. Returns (beaded, caught fill in cells).
pub fn bead_under_roofs_catching(world: &mut World, steam: MaterialId, mut catch: impl FnMut(i32, i32) -> bool) -> (u32, f32) {
    let mut beaded = 0;
    let mut caught = 0.0;
    for y in 1..H {
        for x in 0..W {
            let c = world.get(x, y);
            if c.material != steam || !world.is_game_roofed(x, y) {
                continue;
            }
            let above = world.get(x, y - 1);
            if !matches!(world.materials.kind(above.material), MaterialKind::Solid) {
                continue;
            }
            beaded += 1;
            if catch(x, y) {
                // aux is 0-means-full on both steam and water.
                caught += (material::LIQUID_FULL - c.aux().min(material::LIQUID_FULL)) as f32 / material::LIQUID_FULL as f32;
                world.set(x, y, Cell::EMPTY);
                continue;
            }
            let shade = (crate::sim::rng::jitter(x, y + world.frame as i32) * 255.0) as u8;
            world.set(x, y, Cell::new(material::WATER, shade).with_temperature(60).with_aux(c.aux()));
        }
    }
    (beaded, caught)
}

/// What the player is holding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Splash,
    Lid,
    Dig,
}

impl Tool {
    pub const ALL: [Tool; 3] = [Tool::Splash, Tool::Lid, Tool::Dig];
    pub fn name(self) -> &'static str {
        match self {
            Tool::Splash => "SPLASH",
            Tool::Lid => "LID",
            Tool::Dig => "DIG",
        }
    }
}

/// A rectangle of cells, inclusive.
#[derive(Clone, Copy, Debug)]
pub struct Area {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Area {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (self.y0..=self.y1).flat_map(move |y| (self.x0..=self.x1).map(move |x| (x, y)))
    }
}

/// One house: its wood cells as built, and what has happened to it.
#[derive(Clone, Debug)]
pub struct House {
    pub area: Area,
    pub wood_at_start: u32,
    /// Lava has touched it (it will be burning).
    pub burned: bool,
    /// Water has touched it during or after the flood.
    pub flooded: bool,
}

/// Something the player did, and when -- every action goes in the record so
/// a run can be replayed and scored (`PLAYER ACTIONS ALL IN THE DATA`).
#[derive(Clone, Copy, Debug)]
pub struct Action {
    pub tick: u64,
    pub tool: Tool,
    pub x: i32,
    pub y: i32,
}

/// Running totals the screen and the harness both read.
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    /// Ticks the forge had at least `FORGE_NEEDS` molten cells in it.
    pub forge_ticks: u64,
    /// Molten cells the forge has burned.
    pub forge_burned: u32,
    /// Water cells caught under roofs and put back in the tank.
    pub caught: u32,
    /// Peak share of the dam outline that is rock, 0..1.
    pub dam_peak: f32,
    /// Share of the dam outline that is rock now.
    pub dam_now: f32,
    /// Lava turned to stone by water (the engine's `reacted` counter since start).
    pub quenched: u32,
    /// Quench events that happened inside a lid's shadow.
    pub quenched_under_lid: u32,
    /// Water cells that condensed inside a lid's shadow and fell back.
    pub drips: u32,
    /// Lava, and loose stone made from it, at or past [`PAST_X`] now: what
    /// got past the dam site.
    pub past: u32,
    /// Splashes, lids and digs used.
    pub splashes: u32,
    pub lids: u32,
    pub digs: u32,
    /// Ticks from the most recent splash to the first quench after it.
    pub last_splash_to_quench: Option<u64>,
}

/// The game.
pub struct Quench {
    pub world: World,
    pub particles: ParticleSystem,
    pub blasts: Blasts,
    tuning: player::Tuning,
    pub tick: u64,
    pub tank: i32,
    pub lids_left: u32,
    pub tool: Tool,
    pub houses: Vec<House>,
    pub dam: Area,
    pub forge: Area,
    pub vent: (i32, i32),
    pub flood_src: Area,
    pub actions: Vec<Action>,
    pub counts: Counts,
    pub lava: MaterialId,
    steam: MaterialId,
    wood: MaterialId,
    /// Lid footprints placed, for drawing their shadow.
    pub lid_areas: Vec<Area>,
    /// Frames of glow left on each cell where lava just became stone under water.
    glow: Vec<u8>,
    prev_mat: Vec<MaterialId>,
    last_splash_tick: Option<u64>,
    /// Message shown on screen for a moment after an action.
    pub note: Option<(String, u64)>,
    /// Steam under a roof beads and drips at once ([`bead_under_roofs`]).
    /// On by default; `F` turns it off to play the shipped engine as is.
    pub fast_drip: bool,
    caught: f32,
    /// Ticks between the vent's pours ([`VENT_EVERY`]); a field so the probe
    /// can sweep the eruption's size.
    pub vent_every: u64,
    /// How long the flood pours, 0 for no flood (the default). **Off since
    /// round 2 (2026-11-07)**: at [`FLOOD_SECONDS`] it stood three rows up
    /// three or four houses in every plan, a dam or none, so it scored
    /// nothing but itself; and the water a player throws drains to the
    /// village too, so a flood check then counted the player's own splashes.
    /// The probe's `flood=8` puts it back.
    pub flood_seconds: u64,
}

fn idx(x: i32, y: i32) -> usize {
    (y * W + x) as usize
}

/// The cells a lid placed at `(cx, cy)` covers (its top, not its skirts).
pub fn lid_area(cx: i32, cy: i32) -> Area {
    Area { x0: (cx - LID_W / 2).max(0), y0: cy - LID_H / 2, x1: (cx + LID_W / 2 - 1).min(W - 1), y1: cy - LID_H / 2 + LID_H - 1 }
}

/// The gorge floor's height at column `x`: high at the vent, a slope down to
/// the forge's pit, the flat dam site, and the village bench past it.
///
/// **The forge sits upstream of the dam, not past the village** -- tried the
/// other way first (2026-11-07): in a side view there is no way for lava to
/// reach a forge past the houses without crossing them. Here the lava fills
/// the forge first and then runs on to the houses unless a dam stops it, so
/// doing nothing burns the village, quenching everything at the vent starves
/// the forge, and the plan is a dam *between* the two.
pub fn floor_y(x: i32) -> i32 {
    match x {
        ..=20 => 58,
        21..=105 => 58 + ((x - 20) * 54) / 85, // 58 -> 112
        106..=126 => 126,                      // the forge pit
        // A sill between the forge and the dam site: water thrown at the dam
        // ran back down into the forge's pit and quenched its lava (forge
        // burned 296 cells against 415 with the same lids and no water,
        // 2026-11-07 round 2). Lava tops the sill once the pit is full.
        127..=130 => 115,
        131..=178 => 118, // the dam site
        // The village climbs away from the dam in steps of two rows, one
        // house per step, so how high the lava (or the flood) stands decides
        // how many houses it takes -- a graded loss rather than all or none.
        // A flat bench lost all four houses to every plan (2026-11-07).
        179..=242 => 116 - 2 * ((x - 179) / 16),
        _ => 108,
    }
}

impl Quench {
    pub fn new(seed: u64) -> Self {
        let mut world = World::new(Rect::new(0, 0, W - 1, H - 1));
        world.seed = seed;
        world.weather_override = Some(Weather::CLEAR);
        world.set_sky_hold(SkyPin::Noon.hold());
        world.crust_quench_depth = CRUST_DEPTH;
        let lava = world.materials.id_of("lava").expect("lava is compiled in");
        let steam = world.materials.id_of("steam").expect("steam is compiled in");
        let wood = world.materials.id_of("wood").expect("wood is compiled in");
        let rock = |x: i32, y: i32| Cell::new(material::STONE, (crate::sim::rng::jitter(x, y) * 255.0) as u8).with_attached(true);

        // The gorge: rock below the floor line, and a high back wall on the
        // left so the vent's pool runs right rather than off the map.
        for x in 0..W {
            let f = floor_y(x);
            for y in f..H {
                world.set(x, y, rock(x, y));
            }
        }
        for y in 10..58 {
            for x in 0..6 {
                world.set(x, y, rock(x, y));
            }
        }
        // The rock overhang: a shelf over the upper slope, the free roof C's
        // design starts the player under. Quenching here stops the lava
        // high up -- which is also what starves the forge.
        let overhang = Area { x0: 50, y0: 58, x1: 92, y1: 62 };
        for (x, y) in overhang.cells() {
            world.set(x, y, rock(x, y));
        }
        // No pillar: one down to the slope walled the vent's pool in and the
        // lava crusted there without ever reaching the gorge (2026-11-07).
        // Placed stone holds itself up (`wiki/structural-collapse.md`).
        // The forge: the pit at the foot of the slope.
        let forge = Area { x0: 106, y0: 118, x1: 126, y1: 125 };
        // The village: four wooden houses on the bench past the dam.
        let mut houses = Vec::new();
        for &x0 in &[186, 202, 218, 234] {
            let base = floor_y(x0);
            let area = Area { x0, y0: base - 7, x1: x0 + 7, y1: base - 1 };
            let mut n = 0;
            for (x, y) in area.cells() {
                // A peaked roof: skip the top corners.
                let dy = y - area.y0;
                let edge = (x - area.x0).min(area.x1 - x);
                if dy < 3 && edge < 3 - dy {
                    continue;
                }
                world.set(x, y, Cell::new(wood, ((x * 13 + y * 7) & 0xff) as u8));
                n += 1;
            }
            houses.push(House { area, wood_at_start: n, burned: false, flooded: false });
        }
        let dam = Area { x0: 140, y0: 96, x1: 170, y1: 117 };
        let flood_src = Area { x0: 8, y0: 40, x1: 17, y1: 41 };

        let mut q = Self {
            world,
            particles: ParticleSystem::new(),
            blasts: Blasts::default(),
            tuning: player::Tuning::default(),
            tick: 0,
            tank: TANK,
            lids_left: LIDS,
            tool: Tool::Splash,
            houses,
            dam,
            forge,
            vent: (8, 55),
            flood_src,
            actions: Vec::new(),
            counts: Counts::default(),
            lava,
            steam,
            wood,
            lid_areas: Vec::new(),
            glow: vec![0; (W * H) as usize],
            prev_mat: vec![material::EMPTY; (W * H) as usize],
            last_splash_tick: None,
            note: None,
            fast_drip: true,
            caught: 0.0,
            vent_every: VENT_EVERY,
            // Off by default; see `flood_seconds`.
            flood_seconds: 0,
        };
        // The overhang is a roof the engine's frozen map will not see (it is
        // narrower than the 32-cell reach of the open sky), so it is declared
        // the same way a placed lid is.
        q.declare_roof(overhang);
        q
    }

    /// The column of the furthest-downstream molten cell, if any.
    pub fn lava_front(&self) -> Option<i32> {
        (0..W).rev().find(|&x| (0..H).any(|y| self.world.get(x, y).material == self.lava))
    }

    pub fn seconds(&self) -> f32 {
        self.tick as f32 / TICKS_PER_SECOND as f32
    }

    pub fn over(&self) -> bool {
        self.tick >= ROUND_SECONDS * TICKS_PER_SECOND
    }

    fn is_ground(&self, m: MaterialId) -> bool {
        matches!(self.world.materials.kind(m), MaterialKind::Solid | MaterialKind::Powder)
    }

    /// Mark the space under `area` (down to the first ground) as roofed.
    fn declare_roof(&mut self, area: Area) {
        for x in area.x0..=area.x1 {
            for y in (area.y1 + 1)..H {
                let m = self.world.get(x, y).material;
                if self.is_ground(m) {
                    break;
                }
                self.world.set_game_roofed(x, y, true);
            }
        }
    }

    /// What the held tool would do at `(x, y)`, in plain words, or why not.
    pub fn preview(&self, x: i32, y: i32) -> Result<String, String> {
        match self.tool {
            Tool::Splash => {
                if self.tank < 20 {
                    Err("TANK EMPTY".into())
                } else {
                    Ok(format!("THROW WATER HERE ({} LEFT)", self.tank))
                }
            }
            Tool::Lid => match self.lid_refusal(x, y) {
                Some(why) => Err(why.into()),
                None => Ok(format!("PUT A STONE ROOF HERE ({} LEFT)", self.lids_left)),
            },
            Tool::Dig => Ok("DIG A CHANNEL HERE".into()),
        }
    }

    /// Use the held tool at `(x, y)`.
    pub fn act(&mut self, tool: Tool, x: i32, y: i32) -> bool {
        if self.over() {
            return false;
        }
        let done = match tool {
            Tool::Splash => self.splash(x, y),
            Tool::Lid => self.place_lid(x, y),
            Tool::Dig => self.dig(x, y),
        };
        if done {
            self.actions.push(Action { tick: self.tick, tool, x, y });
        }
        done
    }

    fn splash(&mut self, cx: i32, cy: i32) -> bool {
        if self.tank < 20 {
            self.note = Some(("TANK EMPTY".into(), self.tick));
            return false;
        }
        let r = SPLASH_RADIUS;
        let mut n = 0;
        for y in (cy - r)..=(cy + r) {
            for x in (cx - r)..=(cx + r) {
                if !(1..W - 1).contains(&x) || !(1..H - 1).contains(&y) || (x - cx).pow(2) + (y - cy).pow(2) > r * r {
                    continue;
                }
                let m = self.world.get(x, y).material;
                if (m == material::EMPTY || m == self.steam || m == material::SMOKE) && n < self.tank {
                    self.world.set(x, y, Cell::new(material::WATER, ((x * 5 + y * 3) & 0xff) as u8));
                    n += 1;
                }
            }
        }
        if n == 0 {
            return false;
        }
        self.tank -= n;
        self.counts.splashes += 1;
        self.last_splash_tick = Some(self.tick);
        true
    }

    /// Why a lid could not go at `(cx, cy)`, if it could not.
    pub fn lid_refusal(&self, cx: i32, cy: i32) -> Option<&'static str> {
        if self.lids_left == 0 {
            return Some("NO LIDS LEFT");
        }
        let area = lid_area(cx, cy);
        if area.y0 < 2 || area.y1 > H - 6 {
            return Some("TOO HIGH OR LOW");
        }
        // **A lid is a roof, not a wall**: it needs open space under it,
        // down past its skirts. Without this rule the lid is simply the best
        // wall in the game -- two stacked across the gorge stop every drop --
        // and the probe stops asking whether the *chemistry* is fun. Only
        // ground counts against it: a lid may hang over lava or water.
        let mut ground = 0;
        let mut total = 0;
        for x in area.x0..=area.x1 {
            for y in (area.y1 + 1)..=(area.y1 + LID_SKIRT + LID_CLEARANCE) {
                total += 1;
                let c = self.world.get(x, y);
                if self.is_ground(c.material) {
                    ground += 1;
                }
            }
        }
        if ground * 20 > total {
            return Some("A LID NEEDS OPEN SPACE UNDER IT");
        }
        None
    }

    fn place_lid(&mut self, cx: i32, cy: i32) -> bool {
        if let Some(why) = self.lid_refusal(cx, cy) {
            self.note = Some((why.into(), self.tick));
            return false;
        }
        let area = lid_area(cx, cy);
        let mut n = 0;
        let stone = |x: i32, y: i32| Cell::new(material::STONE, (crate::sim::rng::jitter(x, y) * 255.0) as u8).with_attached(true);
        for (x, y) in area.cells() {
            let m = self.world.get(x, y).material;
            if m == material::EMPTY || m == self.steam || m == material::SMOKE || m == material::WATER {
                self.world.set(x, y, stone(x, y));
                n += 1;
            }
        }
        // The skirts: an upturned cup rather than a flat slab. Measured in
        // `examples/quenchfact.rs`: a flat lid six rows over a lava stream
        // held 4 drips, the same lid with three-row skirts 41 -- steam runs
        // sideways out from under a flat lid before it can bead.
        for x in [area.x0, area.x0 + 1, area.x1 - 1, area.x1] {
            for y in (area.y1 + 1)..=(area.y1 + LID_SKIRT) {
                let m = self.world.get(x, y).material;
                if m == material::EMPTY || m == self.steam || m == material::SMOKE {
                    self.world.set(x, y, stone(x, y));
                }
            }
        }
        if n == 0 {
            return false;
        }
        self.declare_roof(Area { x0: area.x0 + 2, x1: area.x1 - 2, ..area });
        self.lid_areas.push(area);
        self.lids_left -= 1;
        self.counts.lids += 1;
        true
    }

    fn dig(&mut self, cx: i32, cy: i32) -> bool {
        let r = DIG_RADIUS;
        let mut n = 0;
        for y in (cy - r)..=(cy + r) {
            for x in (cx - r)..=(cx + r) {
                if !(1..W - 1).contains(&x) || !(1..H - 3).contains(&y) || (x - cx).pow(2) + (y - cy).pow(2) > r * r {
                    continue;
                }
                let c = self.world.get(x, y);
                if self.is_ground(c.material) && c.material != material::BEDROCK {
                    self.world.set(x, y, Cell::EMPTY);
                    n += 1;
                }
            }
        }
        if n > 0 {
            self.counts.digs += 1;
        }
        n > 0
    }

    /// One simulation tick: the vent, the flood, the engine, and the score.
    pub fn step(&mut self) {
        if self.over() {
            return;
        }
        let secs = self.tick / TICKS_PER_SECOND;
        // The vent: a steady pour at the top of the gorge.
        // The vent keeps itself open: crust or rubble on its mouth is
        // re-melted, or the first skin of stone over it ends the eruption
        // (seen 2026-11-07: the vent sealed itself at ~20 s).
        if secs < VENT_SECONDS && self.tick.is_multiple_of(self.vent_every) {
            let (vx, vy) = self.vent;
            // The seed picks which of the vent's three cells each pour comes
            // out of, so paired seeds are different eruptions and not one
            // eruption relabelled (the first three-seed table was identical
            // to the cell across seeds, 2026-11-07).
            let k = (crate::sim::rng::jitter3(self.world.seed as i32, self.tick as i32, 7) * 3.0) as i32;
            let (x, y) = (vx + k.clamp(0, 2), vy);
            let c = self.world.get(x, y);
            let open = c.material == material::EMPTY
                || ((c.material == material::STONE || c.material == material::RUBBLE) && !c.attached());
            if open {
                self.world.set(x, y, Cell::new(self.lava, (crate::sim::rng::jitter(x, self.tick as i32) * 255.0) as u8));
            }
        }
        // The flood: meltwater down the same gorge.
        if (FLOOD_AT..FLOOD_AT + self.flood_seconds).contains(&secs) && self.tick.is_multiple_of(FLOOD_EVERY) {
            let src = self.flood_src;
            for (x, y) in src.cells() {
                if self.world.get(x, y).material == material::EMPTY {
                    self.world.set(x, y, Cell::new(material::WATER, ((x * 3 + y) & 0xff) as u8));
                }
            }
        }
        for (i, slot) in self.prev_mat.iter_mut().enumerate() {
            *slot = self.world.get(i as i32 % W, i as i32 / W).material;
        }
        if self.fast_drip {
            // **A roof pays the tank back.** One bead in `CATCH_ONE_IN` is
            // caught rather than dripped -- C's cistern, folded into the lid
            // so the reason to aim a roof shows on the tank counter.
            let tick = self.tick;
            let (beaded, caught) = bead_under_roofs_catching(&mut self.world, self.steam, |x, y| {
                crate::sim::rng::jitter3(x, y, tick as i32) < 1.0 / CATCH_ONE_IN as f32
            });
            self.counts.drips += beaded;
            self.caught += caught;
            let whole = self.caught.floor();
            if whole >= 1.0 {
                self.tank = (self.tank + whole as i32).min(TANK);
                self.counts.caught += whole as u32;
                self.caught -= whole;
            }
        }
        let reacted0 = self.world.phase_changes.reacted;
        frame::step(&mut self.world, &mut self.particles, &mut self.blasts, player::PlayerInput::default(), &self.tuning);
        self.tick += 1;
        let reacted = self.world.phase_changes.reacted - reacted0;
        self.counts.quenched += reacted;
        if reacted > 0 {
            if let Some(t) = self.last_splash_tick.take() {
                self.counts.last_splash_to_quench = Some(self.tick - t);
            }
        }
        self.census();
    }

    /// Glow, drips, forge, houses, dam -- one pass over the world.
    fn census(&mut self) {
        let flood_started = self.flood_seconds > 0 && self.tick >= FLOOD_AT * TICKS_PER_SECOND;
        let mut forge_molten = 0;
        let mut dam_rock = 0;
        let mut dam_cells = 0;
        let mut past = 0;
        for y in 0..H {
            for x in 0..W {
                let i = idx(x, y);
                let m = self.world.get(x, y).material;
                let was = self.prev_mat[i];
                if self.glow[i] > 0 {
                    self.glow[i] -= 1;
                }
                if was == self.lava && m == material::STONE {
                    // Quenched rather than merely cooled: water or steam beside it.
                    let wet = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                        let n = self.world.get(x + dx, y + dy).material;
                        n == self.steam || n == material::WATER
                    });
                    if wet {
                        self.glow[i] = 40;
                        if self.world.is_game_roofed(x, y) || self.world.is_game_roofed(x, y - 1) {
                            self.counts.quenched_under_lid += 1;
                        }
                    }
                }
                if was == self.steam && m == material::WATER && self.world.is_game_roofed(x, y) {
                    self.counts.drips += 1;
                }
                if self.forge.contains(x, y) && m == self.lava {
                    forge_molten += 1;
                }
                if x >= PAST_X {
                    let c = self.world.get(x, y);
                    if m == self.lava || ((m == material::STONE || m == material::RUBBLE) && !c.attached()) {
                        past += 1;
                    }
                }
                if self.dam.contains(x, y) && self.dam_mask(x, y) {
                    dam_cells += 1;
                    if self.is_ground(m) && m != material::BEDROCK && !self.world.get(x, y).attached() {
                        dam_rock += 1;
                    }
                }
            }
        }
        if forge_molten >= FORGE_NEEDS {
            self.counts.forge_ticks += 1;
        }
        // **The forge uses what it gets**: one molten cell every
        // `FORGE_EATS_EVERY` ticks, from the bottom of its pit. Without a
        // sink every plan had the same 1,100 cells of lava to put somewhere
        // and all of them buried the village; the forge is D's "needs some
        // lava", made literal, and its output is what it burned.
        if self.tick.is_multiple_of(FORGE_EATS_EVERY) {
            let f = self.forge;
            if let Some((x, y)) = (f.y0..=f.y1).rev().flat_map(|y| (f.x0..=f.x1).map(move |x| (x, y))).find(|&(x, y)| self.world.get(x, y).material == self.lava) {
                self.world.set(x, y, Cell::EMPTY);
                self.counts.forge_burned += 1;
            }
        }
        self.counts.past = past;
        self.counts.dam_now = dam_rock as f32 / dam_cells.max(1) as f32;
        self.counts.dam_peak = self.counts.dam_peak.max(self.counts.dam_now);
        for h in 0..self.houses.len() {
            let area = self.houses[h].area;
            let ring = Area { x0: area.x0 - 1, y0: area.y0 - 1, x1: area.x1 + 1, y1: area.y1 };
            for (x, y) in ring.cells() {
                let m = self.world.get(x, y).material;
                if m == self.lava {
                    self.houses[h].burned = true;
                }
                // Flooded means water standing three rows up the walls, not a
                // puddle: splash water draining past the dam wet every house's
                // doorstep and scored as a flood in every plan (round 2).
                if m == material::WATER && flood_started && y <= area.y1 - 2 {
                    self.houses[h].flooded = true;
                }
            }
            if !self.houses[h].burned {
                let wood_now = area.cells().filter(|&(x, y)| self.world.get(x, y).material == self.wood).count() as u32;
                if wood_now * 10 < self.houses[h].wood_at_start * 7 {
                    self.houses[h].burned = true;
                }
            }
        }
    }

    /// The dam outline is a trapezoid inside `dam`: wide at the floor,
    /// narrow at the top, the shape a dam is drawn.
    pub fn dam_mask(&self, x: i32, y: i32) -> bool {
        let d = self.dam;
        if !d.contains(x, y) || y >= floor_y(x) {
            return false;
        }
        let from_top = y - d.y0;
        let inset = ((d.y1 - d.y0) - from_top) / 3;
        x >= d.x0 + inset && x <= d.x1 - inset
    }

    /// Forge output as a share of full marks, 0..1.
    pub fn forge_share(&self) -> f32 {
        (self.counts.forge_ticks as f32 / TICKS_PER_SECOND as f32 / FORGE_FULL_SECONDS).min(1.0)
    }

    pub fn houses_safe(&self) -> usize {
        self.houses.iter().filter(|h| !h.burned && !h.flooded).count()
    }

    /// Points out of 200: forge up to 100, houses 25 each.
    pub fn score(&self) -> u32 {
        (self.forge_share() * 100.0).round() as u32 + 25 * self.houses_safe() as u32
    }

    /// Stars, 0..=3.
    pub fn stars(&self) -> u32 {
        match self.score() {
            s if s >= 175 => 3,
            s if s >= 125 => 2,
            s if s >= 75 => 1,
            _ => 0,
        }
    }

    /// Draw the world and its overlays into a `W`x`H` RGBA buffer.
    pub fn draw_world(&mut self, renderer: &mut crate::render::Renderer, buf: &mut [u8]) {
        let touched = self.world.take_touched_chunks();
        renderer.draw(&self.world, &self.particles, &touched, buf, (W as u32, H as u32), true);
        let blend = |buf: &mut [u8], x: i32, y: i32, c: [u8; 3], a: f32| {
            if x < 0 || y < 0 || x >= W || y >= H {
                return;
            }
            let i = idx(x, y) * 4;
            for k in 0..3 {
                buf[i + k] = (buf[i + k] as f32 * (1.0 - a) + c[k] as f32 * a) as u8;
            }
        };
        for y in 0..H {
            for x in 0..W {
                let i = idx(x, y);
                // The dam outline, dotted.
                let edge = self.dam_mask(x, y) && (!self.dam_mask(x, y - 1) || !self.dam_mask(x - 1, y) || !self.dam_mask(x + 1, y));
                if edge && (x + y) % 3 == 0 {
                    blend(buf, x, y, [255, 255, 255], 0.75);
                }
                // Steam under a roof reads as a glow, so the engine is visible
                // at phone size.
                let m = self.world.get(x, y).material;
                if self.world.is_game_roofed(x, y) && (m == self.steam || m == material::WATER) {
                    blend(buf, x, y, [190, 235, 255], 0.55);
                }
                if self.glow[i] > 0 {
                    let a = self.glow[i] as f32 / 40.0;
                    blend(buf, x, y, [255, 240, 160], 0.8 * a);
                }
                if self.forge.contains(x, y) && y == self.forge.y0 - 1 + 1 && x % 2 == 0 && self.world.get(x, y).material == material::EMPTY {
                    blend(buf, x, y, [255, 140, 40], 0.6);
                }
            }
        }
    }
}

/// The window's frame: the world at [`SCALE`]x, with the HUD band under it.
pub const SCALE: i32 = 3;
pub const HUD_H: i32 = 66;
pub const FRAME_W: u32 = (W * SCALE) as u32;
pub const FRAME_H: u32 = (H * SCALE + HUD_H) as u32;

const INK: [u8; 4] = [235, 235, 225, 255];
const DIM: [u8; 4] = [150, 150, 145, 255];
const HOT: [u8; 4] = [255, 170, 70, 255];
const COOL: [u8; 4] = [150, 210, 255, 255];
const BAD: [u8; 4] = [255, 110, 100, 255];

fn mmss(secs: f32) -> String {
    let s = secs.max(0.0).ceil() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

impl Quench {
    /// Draw the whole window: the world scaled up, the cursor's footprint,
    /// and the HUD in plain words. `world_buf` is scratch, `W`x`H` RGBA.
    /// The harness draws through this too, so its pictures are the window.
    pub fn draw_frame(&mut self, renderer: &mut crate::render::Renderer, world_buf: &mut [u8], frame: &mut [u8], cursor: Option<(i32, i32)>) {
        self.draw_world(renderer, world_buf);
        let fw = FRAME_W as usize;
        for y in 0..(H * SCALE) as usize {
            for x in 0..fw {
                let s = ((y / SCALE as usize) * W as usize + x / SCALE as usize) * 4;
                let d = (y * fw + x) * 4;
                frame[d..d + 3].copy_from_slice(&world_buf[s..s + 3]);
                frame[d + 3] = 255;
            }
        }
        for px in frame[(H * SCALE) as usize * fw * 4..].chunks_exact_mut(4) {
            px.copy_from_slice(&[22, 20, 24, 255]);
        }
        // The cursor: the tool's footprint, outlined, green if it will work.
        if let Some((cx, cy)) = cursor {
            let ok = self.preview(cx, cy).is_ok();
            let col = if ok { [120, 255, 140, 255] } else { [255, 90, 80, 255] };
            let mut mark = |x: i32, y: i32| {
                if x >= 0 && y >= 0 && x < W && y < H {
                    for k in 0..SCALE {
                        crate::render::put(frame, FRAME_W, FRAME_H, x * SCALE + k, y * SCALE, col);
                        crate::render::put(frame, FRAME_W, FRAME_H, x * SCALE, y * SCALE + k, col);
                    }
                }
            };
            match self.tool {
                Tool::Lid => {
                    let a = lid_area(cx, cy);
                    for x in a.x0..=a.x1 {
                        mark(x, a.y0);
                        mark(x, a.y1 + LID_SKIRT);
                    }
                    for y in a.y0..=(a.y1 + LID_SKIRT) {
                        mark(a.x0, y);
                        mark(a.x1, y);
                    }
                }
                Tool::Splash | Tool::Dig => {
                    let r = if self.tool == Tool::Splash { SPLASH_RADIUS } else { DIG_RADIUS };
                    for k in 0..32 {
                        let t = k as f32 / 32.0 * std::f32::consts::TAU;
                        mark(cx + (t.cos() * r as f32).round() as i32, cy + (t.sin() * r as f32).round() as i32);
                    }
                }
            }
        }
        let size = (FRAME_W, FRAME_H);
        let text = |frame: &mut [u8], x: i32, row: i32, t: &str, c: [u8; 4]| {
            crate::hud::draw_text_scaled(frame, size, 2, (x, (H * SCALE) / 2 + 3 + row * 10), t, c);
        };
        let secs = self.seconds();
        // Row 1: how it is going.
        let forge = (self.forge_share() * 100.0).round() as u32;
        let mut x = 3;
        for (t, c) in [
            (format!("FORGE {forge}%"), if forge > 0 { HOT } else { DIM }),
            (format!("HOUSES SAFE {}/{}", self.houses_safe(), self.houses.len()), if self.houses_safe() == self.houses.len() { INK } else { BAD }),
            (format!("WATER {}", self.tank), COOL),
            (format!("LIDS {}", self.lids_left), INK),
        ] {
            text(frame, x, 0, &t, c);
            x += crate::hud::text_width(&t) + 8;
        }
        let flood = self.flood_seconds > 0;
        let clock = if flood && secs < FLOOD_AT as f32 {
            format!("FLOOD IN {}", mmss(FLOOD_AT as f32 - secs))
        } else if !self.over() {
            format!("ENDS IN {}", mmss(ROUND_SECONDS as f32 - secs))
        } else {
            "ROUND OVER".to_string()
        };
        text(frame, FRAME_W as i32 / 2 - crate::hud::text_width(&clock) - 3, 0, &clock, if flood && secs >= FLOOD_AT as f32 { COOL } else { INK });
        // Row 2: the tools.
        let mut x = 3;
        for (i, t) in Tool::ALL.iter().enumerate() {
            let label = format!("{} {}", i + 1, t.name());
            text(frame, x, 1, &label, if *t == self.tool { [255, 255, 120, 255] } else { DIM });
            x += crate::hud::text_width(&label) + 8;
        }
        let on = |b: bool| if b { "ON" } else { "OFF" };
        let rules = format!("F LID DRIP {}  C WATER THROUGH CRUST {}", on(self.fast_drip), on(self.world.crust_quench_depth > 0));
        text(frame, x, 1, &rules, DIM);
        // Row 3: what the next click does, or what just happened.
        let line = if self.over() {
            format!("SCORE {} - STARS {}/3   R TO PLAY AGAIN", self.score(), self.stars())
        } else if let Some((n, t)) = self.note.as_ref().filter(|(_, t)| self.tick < t + 120) {
            let _ = t;
            n.clone()
        } else if let Some((cx, cy)) = cursor {
            match self.preview(cx, cy) {
                Ok(t) => format!("CLICK: {t}"),
                Err(t) => t,
            }
        } else {
            // Round 2's one hint: the combo is not guessable from the tools.
            "A LID IS A ROOF. WATER UNDER IT AS LAVA ARRIVES SEALS IT.".to_string()
        };
        text(frame, 3, 2, &line, INK);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lid_turns_its_shadow_into_a_roof_and_nothing_else_does() {
        let mut q = Quench::new(1);
        q.step();
        assert!(!q.world.is_game_roofed(160, 60), "open air over the dam site is not roofed");
        assert!(q.act(Tool::Lid, 160, 60));
        assert!(q.world.is_game_roofed(160, 70), "the lid's shadow is roofed");
        assert!(!q.world.is_outdoors(160, 70), "and condensation sees it as indoors");
        assert!(!q.world.is_game_roofed(130, 70), "beside the lid is not");
    }

    #[test]
    fn a_fresh_world_has_no_game_roof() {
        let mut w = World::new(Rect::new(0, 0, 63, 63));
        assert!(!w.is_game_roofed(10, 10));
        w.set_game_roofed(10, 10, false);
        assert!(!w.is_game_roofed(10, 10), "clearing on an empty map allocates nothing and answers false");
        w.set_game_roofed(10, 10, true);
        assert!(w.is_game_roofed(10, 10));
        w.set_game_roofed(10, 10, false);
        assert!(!w.is_game_roofed(10, 10));
    }
}
