//! **Storm Glass** -- lightning that leaves a mark. `cargo run --release --bin storm`
//!
//! A prototype, not a game: the one question it exists to answer is whether
//! *aiming lightning* is fun in the hand or feels like dice (brief:
//! `/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/1-lightning.md`).
//!
//! **The verb.** Hold to call a bolt above the pointer; the leader creeps
//! down while you hold, with a dotted line showing where it will most likely
//! go. Release to strike. A longer hold is a straighter, heavier bolt, but
//! hold too long and the cloud lets go by itself onto the tallest iron --
//! the bell tower you are protecting. A click pours a splash of water and
//! the leader bends toward the wet ground as you watch; a right click drives
//! an iron rod.
//!
//! **The mark.** Where a bolt goes into the ground it keeps going a little
//! way, and the ground it passes through is changed for good: soil burns to
//! scorch, which conducts, and sand melts to glass, which does not. Later
//! bolts are drawn to old scorch and kept out of old glass, and a bolt that
//! lands anywhere on a scorch line lights every lamp that line touches. So
//! over a storm the player's own strikes grow into wiring.
//!
//! **What it touches in the engine: nothing shared.** The bolt is this
//! module's; it reads cells and writes cells through `World::get`/`set` and
//! nothing else. Glass and scorch are `crystal` and `basalt` *repainted in
//! this world's own material table* (`Ids::repaint`), the same runtime
//! palette swap `druid::ground_toned_nest` uses, so the shared registry is
//! unchanged and no other game can see it.

pub mod bolt;
pub mod draw;
pub mod level;

use crate::render::Renderer;
use crate::sim::cell::Cell;
use crate::sim::explosion::Blasts;
use crate::sim::material::{self, MaterialId, MaterialKind};
use crate::sim::particle::ParticleSystem;
use crate::sim::rng::Rng;
use crate::sim::world::World;
use crate::sim::{clock, frame, player, weather};
use bolt::{Medium, Path, Push, Stuff};
use level::{Level, H, W};

pub const TICKS_PER_SECOND: u32 = 60;
/// The underside of the cloud: every bolt starts on this row.
pub const CLOUD_BASE: i32 = 18;
/// Bolts in one storm.
pub const BOLTS: u32 = 8;
pub const SPLASHES: u32 = 4;
pub const RODS: u32 = 2;

/// Seconds of hold that make a full-weight bolt.
pub const FULL_HOLD: f32 = 1.4;
/// Seconds of hold after which the cloud lets go by itself, onto the bell.
pub const OVERFLOW_HOLD: f32 = 2.3;
/// Seconds the cloud takes to gather charge after a strike.
pub const RECHARGE: f32 = 1.2;
/// Seconds the cloud waits, charged, before it strikes the bell by itself.
/// This is "doing nothing must lose".
pub const PATIENCE: f32 = 4.0;
/// How far the leader creeps while you hold: it stops this many cells above
/// the nearest ground, so the last stretch -- where it bends -- is decided
/// on release.
pub const LEADER_STALL: u16 = 34;
/// Ions fade by this factor every tick: about 3 seconds to a twentieth.
pub const ION_FADE: f32 = 0.983;

/// The materials this game uses, looked up once.
#[derive(Clone, Copy, Debug)]
pub struct Ids {
    pub soil: MaterialId,
    pub litter: MaterialId,
    pub log: MaterialId,
    pub iron: MaterialId,
    pub glass: MaterialId,
    pub scorch: MaterialId,
    pub lamp_dark: MaterialId,
    pub lamp_lit: MaterialId,
    pub steam: MaterialId,
}

impl Ids {
    pub fn of(w: &World) -> Self {
        let id = |n: &str| w.materials.id_of(n).unwrap_or_else(|| panic!("`{n}` is a compiled-in material"));
        Ids {
            soil: id("soil"),
            litter: id("litter"),
            log: id("log"),
            iron: id("ironstone"),
            glass: id("crystal"),
            scorch: id("basalt"),
            lamp_dark: id("spar"),
            lamp_lit: id("growlamp"),
            steam: id("steam"),
        }
    }

    /// **Repaint five shipped materials in this world only.** Glass is pale
    /// fulgurite green and glows faintly, scorch is charcoal, the valley's
    /// soil is a lighter loam for the scorch to show on, the dark lamp is
    /// cold iron-grey, and the lit lamp glows. `glow` on the dark lamp is cut
    /// to zero, because `spar` ships as a light source and an unlit lamp that
    /// shines is a lie on screen.
    pub fn repaint(&self, w: &mut World) {
        let set = |w: &mut World, id: MaterialId, colours: &[[u8; 3]], glow: f32| {
            let m = w.materials.get_mut(id);
            m.palette = colours.iter().map(|c| [c[0], c[1], c[2], 255]).collect();
            m.base_shades = colours.len();
            m.glow = glow;
        };
        // Glass glows a little: at dusk the first, unlit palette was the
        // same grey as the sand around it, and ninety cells of fulgurite
        // read as nothing at play scale.
        set(w, self.glass, &[[176, 232, 220], [204, 246, 236], [150, 214, 204], [226, 252, 246]], 0.9);
        set(w, self.scorch, &[[30, 26, 26], [42, 38, 40], [22, 20, 22], [58, 54, 58]], 0.0);
        // The valley's loam, lighter than the shipped soil so a black scar
        // reads on it at dusk -- the first look had every scorch line
        // invisible against the shipped dark-brown families.
        set(w, self.soil, &[[132, 100, 70], [120, 90, 62], [144, 110, 78], [112, 84, 58]], 0.0);
        set(w, self.lamp_dark, &[[84, 88, 98], [74, 78, 88], [96, 100, 110], [66, 70, 80]], 0.0);
        // Cabins do not rot inside one storm. Shipped log decays to soil
        // fast enough that the first look had the village slumping into
        // mounds with no bolt near it (34 cells in 25 seconds).
        let log = w.materials.get_mut(self.log);
        log.decay_chance_damp = 0.0;
        log.decay_chance_dry = 0.0;
        set(w, self.lamp_lit, &[[255, 236, 170], [255, 246, 200], [250, 222, 140], [255, 250, 220]], 2.4);
    }
}

/// How well a cell conducts, and whether a bolt may pass it.
pub fn conductivity(w: &World, ids: &Ids, c: Cell) -> (Stuff, f32) {
    let m = c.material;
    if m == material::EMPTY {
        return (Stuff::Air, 0.0);
    }
    if m == ids.glass {
        return (Stuff::Glass, 0.0);
    }
    if m == ids.iron || m == ids.lamp_dark || m == ids.lamp_lit {
        return (Stuff::Ground, 1.0);
    }
    if m == ids.scorch {
        return (Stuff::Ground, 0.85);
    }
    match w.materials.kind(m) {
        MaterialKind::Empty | MaterialKind::Gas => (Stuff::Air, 0.0),
        MaterialKind::Liquid => (Stuff::Ground, 0.8),
        _ => {
            if m == ids.soil {
                let wet = (c.aux() as f32 / material::SOIL_FIELD_CAPACITY as f32).min(1.3);
                (Stuff::Ground, 0.12 + 0.45 * wet)
            } else if m == material::SAND {
                (Stuff::Ground, 0.08)
            } else if m == ids.log || m == ids.litter {
                (Stuff::Ground, 0.15)
            } else {
                (Stuff::Ground, 0.04)
            }
        }
    }
}

/// Is this cell part of a wire -- something a current runs along?
fn is_wire(ids: &Ids, w: &World, c: Cell) -> bool {
    let m = c.material;
    m == ids.scorch || m == ids.iron || m == ids.lamp_dark || m == ids.lamp_lit || (m != material::EMPTY && w.materials.kind(m) == MaterialKind::Liquid)
}

/// One tick of the player's hands.
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    /// The pointer, in world cells.
    pub at: (i32, i32),
    /// The call button is down.
    pub hold: bool,
    /// Pour water at the pointer (an edge, not a level).
    pub splash: bool,
    /// Drive an iron rod at the pointer (an edge).
    pub rod: bool,
}

#[derive(Clone, Debug)]
pub enum Phase {
    Gathering { t: u32 },
    Ready { t: u32 },
    Calling { t: u32, from: (i32, i32), leader: Vec<(i32, i32)>, last: (i32, i32) },
    Over,
}

/// Why a bolt came down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The player released it.
    Called,
    /// Held too long.
    Overheld,
    /// Not called in time.
    Ignored,
}

/// The bolt the screen is showing, and everything it did.
#[derive(Clone, Debug)]
pub struct Strike {
    pub main: Vec<(i32, i32)>,
    pub branches: Vec<Path>,
    pub hit: Option<(i32, i32)>,
    /// Cells the current burned or melted on its way into the ground.
    pub roots: Vec<(i32, i32)>,
    /// Every wire cell the current ran along.
    pub network: Vec<(i32, i32)>,
    pub heavy: f32,
    pub age: u32,
    pub cause: Cause,
    /// What it hit, in plain words, for the screen.
    pub said: String,
}

/// Where a called bolt was aimed (its cloud cell) and the ground cell it
/// touched, if any.
pub type Landing = ((i32, i32), Option<(i32, i32)>);

/// A self-strike that interrupts a call: where the call began and the
/// leader it had grown.
type Interrupted<'a> = ((i32, i32), &'a [(i32, i32)]);

/// Running counts, for the screen and for the probe.
#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub bolts: u32,
    pub called: u32,
    pub overheld: u32,
    pub ignored: u32,
    pub lamp_hits: [u32; 2],
    pub bell_hits: u32,
    /// Lamps reached through a scorch line or rod rather than struck
    /// directly -- the wiring paying off.
    pub wired_hits: u32,
    pub glass_cells: u32,
    pub scorch_cells: u32,
    pub steam_cells: u32,
    pub fires: u32,
    pub splashes: u32,
    pub rods: u32,
    /// Where each called bolt was aimed and where it landed.
    pub landings: Vec<Landing>,
}

pub struct Storm {
    pub world: World,
    pub particles: ParticleSystem,
    pub blasts: Blasts,
    pub renderer: Renderer,
    pub level: Level,
    pub ids: Ids,
    pub seed: u64,
    pub rng: Rng,
    pub tick: u64,
    pub phase: Phase,
    pub ions: Vec<f32>,
    medium: Medium,
    pub likely: Path,
    pub feelers: Vec<Path>,
    pub strike: Option<Strike>,
    pub flash: f32,
    pub bolts_left: u32,
    pub splashes_left: u32,
    pub rods_left: u32,
    /// Light in each lamp, 0..=3.
    pub lamps: [u8; 2],
    pub tally: Tally,
    pub huts_start: u32,
    pub pointer: (i32, i32),
    /// The world as the renderer last drew it, kept between frames so the
    /// renderer's dirty-rect skip still works: every overlay is drawn on the
    /// composed frame, never into this.
    world_buf: Vec<u8>,
    world_buf_fresh: bool,
}

impl Storm {
    pub fn new(seed: u64) -> Self {
        let level = Level::valley();
        let probe = World::new(crate::sim::chunk::Rect::new(0, 0, 15, 15));
        let ids = Ids::of(&probe);
        let mut world = level.build(&ids, seed);
        ids.repaint(&mut world);
        // Dusk, held, and no weather of the engine's own: its lightning is
        // a picture with no effect, and two kinds of bolt on one screen
        // would teach the wrong thing.
        world.set_sky_hold(clock::SkyPin::Dusk.hold());
        world.set_weather_pin(weather::Pin::Clear);
        let huts_start = count(&world, ids.log);
        let n = (W * H) as usize;
        let mut s = Storm {
            world,
            particles: ParticleSystem::new(),
            blasts: Blasts::default(),
            renderer: Renderer::new(),
            level,
            ids,
            seed,
            rng: Rng::new(seed.wrapping_mul(0x9E37_79B9).wrapping_add(17)),
            tick: 0,
            phase: Phase::Gathering { t: 0 },
            ions: vec![0.0; n],
            medium: Medium { w: W, h: H, stuff: vec![Stuff::Air; n], cond: vec![0.0; n], reach: Vec::new(), ions: vec![0.0; n] },
            likely: Path::default(),
            feelers: Vec::new(),
            strike: None,
            flash: 0.0,
            bolts_left: BOLTS,
            splashes_left: SPLASHES,
            rods_left: RODS,
            lamps: [0; 2],
            tally: Tally::default(),
            huts_start,
            pointer: (W / 2, CLOUD_BASE + 10),
            world_buf: vec![0; n * 4],
            world_buf_fresh: false,
        };
        // Let the powders settle before the first bolt.
        for _ in 0..30 {
            s.step_world();
        }
        s
    }

    fn step_world(&mut self) {
        frame::step(&mut self.world, &mut self.particles, &mut self.blasts, player::PlayerInput::default(), &player::Tuning::default());
    }

    pub fn is_over(&self) -> bool {
        matches!(self.phase, Phase::Over)
    }

    /// How heavy a hold of `t` ticks makes the bolt, 0..1.
    pub fn heaviness(t: u32) -> f32 {
        (t as f32 / (FULL_HOLD * TICKS_PER_SECOND as f32)).clamp(0.0, 1.0)
    }

    /// Stars for the storm so far: lamp light against bell hits.
    pub fn points(&self) -> i32 {
        self.lamps.iter().map(|&p| p as i32).sum::<i32>() - self.tally.bell_hits as i32 - self.huts_lost_penalty()
    }

    pub fn huts_lost(&self) -> u32 {
        self.huts_start.saturating_sub(count(&self.world, self.ids.log))
    }

    /// A hut is worth one point once a fifth of its walls is gone.
    fn huts_lost_penalty(&self) -> i32 {
        (self.huts_lost() as f32 / (self.huts_start as f32 * 0.2 / self.level.huts.len() as f32)).floor().min(2.0) as i32
    }

    pub fn stars(&self) -> u32 {
        match self.points() {
            6.. => 3,
            4..=5 => 2,
            2..=3 => 1,
            _ => 0,
        }
    }

    /// One tick of the game.
    pub fn update(&mut self, input: Input) {
        self.tick += 1;
        self.pointer = (input.at.0.clamp(0, W - 1), input.at.1.clamp(0, H - 1));
        if !self.is_over() {
            if input.splash && self.splashes_left > 0 && self.pour(self.pointer) {
                self.splashes_left -= 1;
                self.tally.splashes += 1;
            }
            if input.rod && self.rods_left > 0 && self.drive_rod(self.pointer) {
                self.rods_left -= 1;
                self.tally.rods += 1;
            }
        }
        self.step_world();
        for v in self.ions.iter_mut() {
            *v = if *v > 0.003 { *v * ION_FADE } else { 0.0 };
        }
        self.flash *= 0.78;
        if let Some(s) = &mut self.strike {
            s.age += 1;
        }

        let phase = std::mem::replace(&mut self.phase, Phase::Over);
        self.phase = match phase {
            Phase::Over => Phase::Over,
            Phase::Gathering { t } => {
                if self.bolts_left == 0 {
                    if t > TICKS_PER_SECOND * 2 {
                        Phase::Over
                    } else {
                        Phase::Gathering { t: t + 1 }
                    }
                } else if t as f32 >= RECHARGE * TICKS_PER_SECOND as f32 {
                    Phase::Ready { t: 0 }
                } else {
                    Phase::Gathering { t: t + 1 }
                }
            }
            Phase::Ready { t } => {
                if input.hold {
                    let from = (self.pointer.0, CLOUD_BASE);
                    Phase::Calling { t: 0, from, leader: Vec::new(), last: (0, 1) }
                } else if t as f32 >= PATIENCE * TICKS_PER_SECOND as f32 {
                    self.self_strike(None, Cause::Ignored);
                    Phase::Gathering { t: 0 }
                } else {
                    Phase::Ready { t: t + 1 }
                }
            }
            Phase::Calling { t, from, mut leader, mut last } => {
                self.sense(None);
                let push = Push { heavy: Self::heaviness(t), pull: 1.0, range: bolt::CALLED_RANGE };
                let tip = leader.last().copied().unwrap_or(from);
                if t as f32 >= OVERFLOW_HOLD * TICKS_PER_SECOND as f32 {
                    self.self_strike(Some((from, &leader)), Cause::Overheld);
                    Phase::Gathering { t: 0 }
                } else if !input.hold {
                    self.release(from, &leader, tip, last, push);
                    Phase::Gathering { t: 0 }
                } else {
                    // Creep: about one and a half cells a tick until it stalls
                    // above the ground.
                    let creep = if t % 2 == 0 { 2 } else { 1 };
                    for _ in 0..creep {
                        let at = leader.last().copied().unwrap_or(from);
                        let Some(i) = self.medium.idx(at.0, at.1) else { break };
                        if self.ground_gap(at) <= LEADER_STALL || self.medium.reach[i] < 30 {
                            break;
                        }
                        // **The leader creeps carefully**: at least three
                        // quarters weight, whatever the hold so far. It grows
                        // during the *start* of the hold, when the weight is
                        // still low, so at the hold's own weight the part of
                        // the bolt the player watched form was always the
                        // wobbly part -- measured over open sand, a full 1.4 s
                        // hold still landed a median 8 cells off its aim. The
                        // hold now decides the last stretch below the stall,
                        // which is where a bolt bends anyway.
                        let careful = Push { heavy: push.heavy.max(0.75), ..push };
                        match bolt::choose(&self.medium, at, last, careful, &mut self.rng, false) {
                            Some(d) => {
                                let next = (at.0 + d.0, at.1 + d.1);
                                let j = self.medium.idx(next.0, next.1).expect("in bounds");
                                if self.medium.stuff[j] != Stuff::Air {
                                    break;
                                }
                                leader.push(next);
                                last = d;
                            }
                            None => break,
                        }
                    }
                    let tip = leader.last().copied().unwrap_or(from);
                    self.likely = bolt::likely(&self.medium, tip, last, push, 400);
                    if t % 5 == 0 {
                        self.feelers.clear();
                        for _ in 0..3 {
                            let len = 12 + self.rng.below(16) as usize;
                            let f = bolt::grow(&self.medium, tip, last, push, &mut self.rng, len, None);
                            self.feelers.push(f);
                        }
                    }
                    Phase::Calling { t: t + 1, from, leader, last }
                }
            }
        };
        if !matches!(self.phase, Phase::Calling { .. }) {
            self.likely = Path::default();
            self.feelers.clear();
        }
    }

    /// Cells straight down from `at` to the first non-air cell.
    fn ground_gap(&self, at: (i32, i32)) -> u16 {
        let mut y = at.1;
        while y < H {
            let i = (y * W + at.0) as usize;
            if self.medium.stuff[i] != Stuff::Air {
                break;
            }
            y += 1;
        }
        (y - at.1).max(0) as u16
    }

    /// Snapshot the world into the bolt's medium. `only` restricts what
    /// pulls (the cloud's own strike pulls toward the bell alone).
    fn sense(&mut self, only: Option<&[usize]>) {
        for y in 0..H {
            for x in 0..W {
                let i = (y * W + x) as usize;
                let (s, c) = conductivity(&self.world, &self.ids, self.world.get(x, y));
                self.medium.stuff[i] = s;
                self.medium.cond[i] = c;
            }
        }
        self.medium.ions.copy_from_slice(&self.ions);
        self.medium.build_reach(only);
    }

    fn release(&mut self, from: (i32, i32), leader: &[(i32, i32)], tip: (i32, i32), last: (i32, i32), push: Push) {
        let mut forks = Vec::new();
        let rest = bolt::grow(&self.medium, tip, last, push, &mut self.rng, 900, Some(&mut forks));
        let mut main: Vec<(i32, i32)> = vec![from];
        main.extend(leader.iter().copied());
        main.extend(rest.cells.iter().copied());
        self.tally.called += 1;
        self.tally.landings.push((from, rest.hit));
        self.land(main, forks, rest.hit, push.heavy, Cause::Called);
    }

    /// The cloud lets go by itself: toward the bell, hard.
    fn self_strike(&mut self, called: Option<Interrupted>, cause: Cause) {
        let bell: Vec<usize> = (self.level.bell.y0..=self.level.bell.y1)
            .flat_map(|y| (self.level.bell.x0..=self.level.bell.x1).map(move |x| (x, y)))
            .filter(|&(x, y)| self.world.get(x, y).material == self.ids.iron)
            .map(|(x, y)| (y * W + x) as usize)
            .collect();
        self.sense(Some(&bell));
        let (origin, leader) = match called {
            Some((from, leader)) => (from, leader),
            None => {
                let (bx, _) = self.level.bell.centre();
                ((bx + self.rng.below(40) as i32 - 8, CLOUD_BASE), &[][..])
            }
        };
        let start = leader.last().copied().unwrap_or(origin);
        let push = Push { heavy: 0.8, pull: 2.0, range: 400.0 };
        let mut forks = Vec::new();
        let p = bolt::grow(&self.medium, start, (0, 1), push, &mut self.rng, 900, Some(&mut forks));
        let mut main = vec![origin];
        main.extend(leader.iter().copied());
        main.extend(p.cells.iter().copied());
        match cause {
            Cause::Overheld => self.tally.overheld += 1,
            Cause::Ignored => self.tally.ignored += 1,
            Cause::Called => {}
        }
        self.land(main, forks, p.hit, push.heavy, cause);
    }

    /// Everything a bolt does when it comes down.
    fn land(&mut self, main: Vec<(i32, i32)>, branches: Vec<Path>, hit: Option<(i32, i32)>, heavy: f32, cause: Cause) {
        self.bolts_left = self.bolts_left.saturating_sub(1);
        self.tally.bolts += 1;
        self.flash = 0.55 + 0.45 * heavy;
        // Ions: thick along the channel, thin along the branches.
        for &(x, y) in &main {
            for (dx, dy, v) in [(0, 0, 1.0), (1, 0, 0.6), (-1, 0, 0.6), (0, 1, 0.5), (0, -1, 0.5)] {
                if let Some(i) = self.medium.idx(x + dx, y + dy) {
                    self.ions[i] = self.ions[i].max(v);
                }
            }
        }
        for b in &branches {
            for &(x, y) in &b.cells {
                if let Some(i) = self.medium.idx(x, y) {
                    self.ions[i] = self.ions[i].max(0.35);
                }
            }
        }
        let mut roots = Vec::new();
        let mut network = Vec::new();
        let mut said = String::from("IT FADED IN THE AIR");
        if let Some(h) = hit {
            // The burst where it lands.
            let r = 2 + (heavy * 2.5) as i32;
            self.burn_around(h, r);
            roots = self.discharge(h, heavy);
            // Branch tips that touch ground leave a small scorch too: forks
            // are drawn *and* leave a mark.
            for b in &branches {
                if let Some(bh) = b.hit {
                    let small = self.discharge(bh, 0.0);
                    roots.extend(small.into_iter().take(4));
                }
            }
            let mut seeds = roots.clone();
            seeds.push(h);
            network = self.flood(&seeds);
            said = self.score(h, &network, heavy, cause);
        }
        self.strike = Some(Strike { main, branches, hit, roots, network, heavy, age: 0, cause, said });
        self.world_buf_fresh = false;
    }

    /// What the strike lit or hit, scored. Returns the words for the screen.
    fn score(&mut self, hit: (i32, i32), network: &[(i32, i32)], heavy: f32, cause: Cause) -> String {
        let mut words = Vec::new();
        let gain = if heavy >= 0.66 { 2 } else { 1 };
        for k in 0..2 {
            let lamp = self.level.lamps[k];
            let direct = lamp.area.contains(hit);
            let wired = !direct && network.iter().any(|&c| lamp.area.contains(c));
            if direct || wired {
                self.lamps[k] = (self.lamps[k] + gain).min(3);
                self.tally.lamp_hits[k] += 1;
                if wired {
                    self.tally.wired_hits += 1;
                }
                // The head lights: the lamp's own material changes, and it
                // throws real light on the ground around it.
                for y in lamp.head.y0..=lamp.head.y1 {
                    for x in lamp.head.x0..=lamp.head.x1 {
                        let c = self.world.get(x, y);
                        if c.material == self.ids.lamp_dark {
                            self.world.set(x, y, Cell::new(self.ids.lamp_lit, c.shade));
                        }
                    }
                }
                words.push(if wired { format!("{} LIT THROUGH THE SCORCH LINE", lamp.name) } else { format!("{} LIT", lamp.name) });
            }
        }
        let bell = self.level.tower.contains(hit) || self.level.bell.contains(hit) || network.iter().any(|&c| self.level.bell.contains(c));
        if bell {
            self.tally.bell_hits += 1;
            words.push(match cause {
                Cause::Overheld => "HELD TOO LONG - IT HIT THE BELL TOWER".to_string(),
                Cause::Ignored => "THE CLOUD STRUCK THE BELL TOWER BY ITSELF".to_string(),
                Cause::Called => "IT HIT THE BELL TOWER".to_string(),
            });
        }
        if words.is_empty() {
            let c = self.world.get(hit.0, hit.1).material;
            let on = if c == material::SAND || c == self.ids.glass {
                "IT MELTED THE SAND TO GLASS"
            } else if c == self.ids.log {
                "IT HIT A HOUSE"
            } else if c == self.ids.litter {
                "IT LIT THE DRY GRASS"
            } else if self.level.ledge.contains(hit) || (hit.0 >= 214) {
                "IT HIT THE ROCK"
            } else {
                "IT SCORCHED THE GROUND"
            };
            words.push(on.to_string());
        }
        words.join(" - ")
    }

    /// Set fire to what burns and boil what is wet, in a disc.
    fn burn_around(&mut self, (cx, cy): (i32, i32), r: i32) {
        for y in cy - r..=cy + r {
            for x in cx - r..=cx + r {
                if (x - cx).pow(2) + (y - cy).pow(2) > r * r || x < 0 || y < 0 || x >= W || y >= H {
                    continue;
                }
                let mut c = self.world.get(x, y);
                if c.material == material::EMPTY {
                    continue;
                }
                let def = self.world.materials.get(c.material);
                if def.flammability > 0.0 && !c.is_burning() {
                    let d = if def.burn_duration > 0 { def.burn_duration } else { 120 };
                    c.ignite(d);
                    self.world.set(x, y, c);
                    self.tally.fires += 1;
                } else if self.world.materials.kind(c.material) == MaterialKind::Liquid {
                    self.world.set(x, y, Cell::new(self.ids.steam, 0).with_temperature(140));
                    self.tally.steam_cells += 1;
                }
            }
        }
    }

    /// **The current going into the ground** -- one to three short walks
    /// that change what they pass through for good. Returns the cells they
    /// changed, and any iron they reached.
    ///
    /// **If the bolt landed on iron or scorch, the current runs through it
    /// first** and leaves where that object meets the ground, deepest point
    /// first -- the foot of a lamp post, the buried end of a rod. The first
    /// version walked from the struck cell itself, and on a lamp the walk
    /// spent its whole length wandering inside the iron (iron out-conducts
    /// soil cubed by 60 to 1), so a lamp hit left no mark at all: 0 scorch
    /// cells over a whole planned storm. Real current goes to ground, and the
    /// foot of the post is where the scar belongs.
    ///
    /// The walks are drawn toward *other* iron and scorch through the ground,
    /// never toward the object they started in, and stop when they touch it:
    /// that contact is a new wire.
    fn discharge(&mut self, hit: (i32, i32), heavy: f32) -> Vec<(i32, i32)> {
        use std::collections::HashSet;
        let roots = 1 + (heavy * 2.2) as usize;
        let len = 8 + (heavy * 40.0) as usize;
        let struck: HashSet<(i32, i32)> = if is_wire(&self.ids, &self.world, self.world.get(hit.0, hit.1)) {
            self.flood(&[hit]).into_iter().collect()
        } else {
            HashSet::new()
        };
        let mut starts: Vec<(i32, i32)> = struck
            .iter()
            .filter(|&&(x, y)| {
                [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= W || ny >= H {
                        return false;
                    }
                    let c = self.world.get(nx, ny);
                    let (stuff, _) = conductivity(&self.world, &self.ids, c);
                    stuff == Stuff::Ground && !is_wire(&self.ids, &self.world, c)
                })
            })
            .copied()
            .collect();
        // The current leaves where the struck wire is nearest some *other*
        // iron, so each strike on a scorch line grows it toward the next
        // lamp -- the wiring has a growing tip. Ties go to the deepest cell.
        let toward = self.ground_reach(&struck);
        // A struck cell has no reach of its own (the reach skips the struck
        // object), so a start is as near as its nearest outside neighbour.
        // Sorting on the cell itself sorted on 65535 for every start and
        // left the order to the tiebreak: measured, all four starts of a rod
        // strike read 65535.
        let near = |(x, y): (i32, i32)| -> u16 {
            let mut best = toward[(y * W + x) as usize];
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < W && ny < H {
                    best = best.min(toward[(ny * W + nx) as usize].saturating_add(1));
                }
            }
            best
        };
        starts.sort_by_key(|&(x, y)| (near((x, y)), -y, x));
        starts.truncate(4);
        if starts.is_empty() {
            starts.push(hit);
        }
        let mut changed = Vec::new();
        let mut own: HashSet<(i32, i32)> = HashSet::new();
        for r in 0..roots {
            own.extend(changed.iter().copied());
            let mut at = starts[(r * 2) % starts.len()];
            let mut seen = struck.clone();
            seen.insert(at);
            for _ in 0..len {
                self.mark(at, &mut changed);
                let here = near(at);
                let mut cand = [(0.0f32, (0, 0)); 8];
                let mut total = 0.0;
                for (k, d) in [(0, 1), (-1, 1), (1, 1), (-1, 0), (1, 0), (0, -1), (-1, -1), (1, -1)].into_iter().enumerate() {
                    let n = (at.0 + d.0, at.1 + d.1);
                    if n.0 < 0 || n.1 < 0 || n.0 >= W || n.1 >= H || seen.contains(&n) {
                        continue;
                    }
                    let (st, c) = conductivity(&self.world, &self.ids, self.world.get(n.0, n.1));
                    if st != Stuff::Ground {
                        continue;
                    }
                    let j = (n.1 * W + n.0) as usize;
                    let pull = if here == u16::MAX || toward[j] == u16::MAX {
                        0.0
                    } else {
                        ((here as f32 - toward[j] as f32) / 3.0).clamp(-1.0, 1.0) * (-(here as f32) / 60.0).exp()
                    };
                    let down = if d.1 > 0 { 0.25 } else if d.1 == 0 { 0.12 } else { 0.03 };
                    let w = (c + down + 1.1 * pull).max(0.01).powi(3);
                    cand[k] = (w, n);
                    total += w;
                }
                if total <= 0.0 {
                    break;
                }
                let mut pick = self.rng.unit_f32() * total;
                let mut next = None;
                for (w, n) in cand {
                    if w <= 0.0 {
                        continue;
                    }
                    if pick <= w {
                        next = Some(n);
                        break;
                    }
                    pick -= w;
                }
                let Some(n) = next else { break };
                seen.insert(n);
                at = n;
                // Touching another wire joins it: mark the contact and stop.
                // Not this strike's own fresh scorch: the first version
                // stopped every second and third root the moment it touched
                // the first one's scar, two cells from where they all began,
                // so a full-weight strike changed 10-14 cells instead of ~40.
                if !own.contains(&n) && is_wire(&self.ids, &self.world, self.world.get(n.0, n.1)) {
                    changed.push(n);
                    break;
                }
            }
        }
        changed
    }

    /// Steps through the ground to the nearest iron that is not part of
    /// `struck` -- what draws a discharge toward a lamp post or rod.
    ///
    /// **Iron only, not old scorch.** With scorch as a source too, a rod
    /// struck beside the rock lamp grew its scar *away* from that lamp and
    /// into the field lamp's old roots, which had crept nearer: 0 lamps lit
    /// through wiring over a planned storm. A lamp should be what a scar
    /// reaches for; joining two scars still happens wherever they meet.
    fn ground_reach(&self, struck: &std::collections::HashSet<(i32, i32)>) -> Vec<u16> {
        let n = (W * H) as usize;
        let mut reach = vec![u16::MAX; n];
        let mut q = std::collections::VecDeque::new();
        for y in 0..H {
            for x in 0..W {
                let m = self.world.get(x, y).material;
                if (m == self.ids.iron || m == self.ids.lamp_dark || m == self.ids.lamp_lit) && !struck.contains(&(x, y)) {
                    let i = (y * W + x) as usize;
                    reach[i] = 0;
                    q.push_back(i);
                }
            }
        }
        while let Some(i) = q.pop_front() {
            let (x, y) = ((i as i32) % W, (i as i32) / W);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= W || ny >= H {
                    continue;
                }
                let j = (ny * W + nx) as usize;
                let (s, _) = conductivity(&self.world, &self.ids, self.world.get(nx, ny));
                if s != Stuff::Ground || reach[j] <= reach[i] + 1 || struck.contains(&(nx, ny)) {
                    continue;
                }
                reach[j] = reach[i] + 1;
                q.push_back(j);
            }
        }
        reach
    }

    /// Change one cell the current passed through, and maybe a neighbour so
    /// the scar reads two cells wide.
    fn mark(&mut self, at: (i32, i32), changed: &mut Vec<(i32, i32)>) {
        self.mark_one(at, changed);
        let side = [(1, 0), (-1, 0), (0, 1)][self.rng.below(3) as usize];
        if self.rng.chance(0.6) {
            self.mark_one((at.0 + side.0, at.1 + side.1), changed);
        }
    }

    fn mark_one(&mut self, (x, y): (i32, i32), changed: &mut Vec<(i32, i32)>) {
        if x < 0 || y < 0 || x >= W || y >= H {
            return;
        }
        let c = self.world.get(x, y);
        let m = c.material;
        let shade = (crate::sim::rng::jitter(x, y) * 255.0) as u8;
        if m == self.ids.soil {
            self.world.set(x, y, Cell::new(self.ids.scorch, shade));
            self.tally.scorch_cells += 1;
            changed.push((x, y));
        } else if m == material::SAND {
            self.world.set(x, y, Cell::new(self.ids.glass, shade));
            self.tally.glass_cells += 1;
            changed.push((x, y));
        } else if m != material::EMPTY && self.world.materials.kind(m) == MaterialKind::Liquid {
            self.world.set(x, y, Cell::new(self.ids.steam, 0).with_temperature(140));
            self.tally.steam_cells += 1;
            changed.push((x, y));
        } else if m != material::EMPTY {
            let def = self.world.materials.get(m);
            if def.flammability > 0.0 && !c.is_burning() {
                let mut c = c;
                c.ignite(if def.burn_duration > 0 { def.burn_duration } else { 120 });
                self.world.set(x, y, c);
                self.tally.fires += 1;
            }
            if m == self.ids.iron || m == self.ids.scorch {
                changed.push((x, y));
            }
        }
    }

    /// Every wire cell connected to `seeds`: the current running out along
    /// scorch, iron and water. Capped, and the cap bounds work, not the
    /// answer -- a lamp touched past it is still a lamp a long way off.
    fn flood(&self, seeds: &[(i32, i32)]) -> Vec<(i32, i32)> {
        let mut seen = std::collections::HashSet::new();
        let mut q: std::collections::VecDeque<(i32, i32)> = seeds.iter().copied().collect();
        let mut out = Vec::new();
        for &s in seeds {
            seen.insert(s);
        }
        while let Some((x, y)) = q.pop_front() {
            if out.len() > 8000 {
                break;
            }
            out.push((x, y));
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)] {
                let n = (x + dx, y + dy);
                if n.0 < 0 || n.1 < 0 || n.0 >= W || n.1 >= H || seen.contains(&n) {
                    continue;
                }
                if is_wire(&self.ids, &self.world, self.world.get(n.0, n.1)) {
                    seen.insert(n);
                    q.push_back(n);
                }
            }
        }
        out
    }

    /// A splash of water: a ball of it at the pointer, in open air only.
    fn pour(&mut self, (cx, cy): (i32, i32)) -> bool {
        let mut placed = 0;
        for y in cy - 3..=cy + 3 {
            for x in cx - 3..=cx + 3 {
                if (x - cx).pow(2) + (y - cy).pow(2) > 10 || x < 0 || y < 0 || x >= W || y >= H {
                    continue;
                }
                if self.world.get(x, y).material == material::EMPTY {
                    self.world.set(x, y, Cell::new(material::WATER, 0));
                    placed += 1;
                }
            }
        }
        placed > 0
    }

    /// An iron rod: driven into the first ground below the pointer, three
    /// cells deep, seven standing.
    fn drive_rod(&mut self, (cx, cy): (i32, i32)) -> bool {
        let mut y = cy.max(CLOUD_BASE + 2);
        while y < H && self.world.get(cx, y).material == material::EMPTY {
            y += 1;
        }
        if y >= H - 3 {
            return false;
        }
        for yy in (y - 7)..(y + 3) {
            for x in cx..=cx + 1 {
                if x < W {
                    self.world.set(x, yy, Cell::new(self.ids.iron, 0));
                }
            }
        }
        true
    }

    /// The frame size [`Self::draw`] wants.
    pub fn frame_size() -> (u32, u32) {
        draw::frame_size()
    }

    /// Draw the whole window into `frame` (RGBA, [`Self::frame_size`]).
    pub fn draw(&mut self, frame: &mut [u8]) {
        let touched = self.world.take_touched_chunks();
        let force = !self.world_buf_fresh;
        self.renderer.draw(&self.world, &self.particles, &touched, &mut self.world_buf, (W as u32, H as u32), force);
        self.world_buf_fresh = true;
        draw::compose(self, frame);
    }

    pub(crate) fn world_pixels(&self) -> &[u8] {
        &self.world_buf
    }
}

fn count(w: &World, m: MaterialId) -> u32 {
    let mut n = 0;
    for y in 0..H {
        for x in 0..W {
            if w.get(x, y).material == m {
                n += 1;
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Doing nothing must lose: a storm with no input ends with every bolt
    /// on the bell and no stars.
    #[test]
    fn doing_nothing_loses() {
        let mut s = Storm::new(1);
        while !s.is_over() && s.tick < 60 * 60 {
            s.update(Input::default());
        }
        assert!(s.is_over(), "an ignored storm must still end");
        assert_eq!(s.tally.ignored, BOLTS);
        assert!(s.tally.bell_hits >= BOLTS - 1, "the ignored cloud should strike the bell: {} of {}", s.tally.bell_hits, BOLTS);
        assert_eq!(s.stars(), 0);
    }

    /// A bolt that goes into soil leaves scorch, and into sand leaves glass.
    #[test]
    fn a_strike_leaves_a_mark() {
        let mut s = Storm::new(2);
        let before = (count(&s.world, s.ids.scorch), count(&s.world, s.ids.glass));
        s.discharge((100, level::ground_y(100) + 1), 1.0);
        s.discharge((170, level::ground_y(170) + 1), 1.0);
        let after = (count(&s.world, s.ids.scorch), count(&s.world, s.ids.glass));
        assert!(after.0 > before.0 + 5, "soil should scorch: {before:?} -> {after:?}");
        assert!(after.1 > before.1 + 5, "sand should melt: {before:?} -> {after:?}");
    }
}
