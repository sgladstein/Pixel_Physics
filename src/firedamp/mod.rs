//! **Firedamp: the gas is your dynamite.** A probe, not a game
//! (`/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/3-firedamp.md`).
//!
//! A mine keeps filling with gas that seeps from a crack in the sump. It is
//! heavier than air, so it pools and climbs toward the miners' lamps at the
//! two work faces. You herd it with fans and doors and burn it off with
//! matches and fuses. Gas only burns in a middle band -- too thin will not
//! carry a flame and too thick will not either -- so the one rule to learn is
//! *too thick to burn, so thin it first*. A big burnable pocket goes bang
//! through the engine's own explosion code, and a bang beside the ore seam is
//! the only way to get much ore: greed against safety.
//!
//! **The question the probe answers**: is that rule learned from one fizzle,
//! and does herding gas feel crisp or mushy?
//!
//! Its own binary (`src/bin/firedamp.rs`) and its own harness
//! (`examples/firedampprobe.rs`). It touches no shared engine file: the rock,
//! rubble and blasts are an ordinary `World` driven by `sim::frame::step`,
//! and the gas, the flame and the tools live here.

pub mod gas;
pub mod level;
pub mod paint;

use crate::sim::explosion::Blasts;
use crate::sim::frame;
use crate::sim::material::MaterialKind;
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::world::World;

use gas::{burnable, idx, inside, Gas, H, N, W};
use level::Level;

pub const TICKS_PER_SECOND: u64 = 60;
/// Three minutes.
pub const LEVEL_TICKS: u64 = 180 * TICKS_PER_SECOND;
/// Ore for one, two and three stars. The faces alone, never stopped, send up
/// 120 carts in a shift; doing nothing loses both faces and sends up about
/// 40. The rest has to come from blasts.
pub const ORE_STARS: [u32; 3] = [60, 140, 220];
/// A working face sends up one cart of ore this often.
const CART_TICKS: u64 = 3 * TICKS_PER_SECOND;
/// Share of the difference that seeps through a shut door per tick, per row.
/// A door holds gas back; it does not hold it for ever.
const DOOR_LEAK: f32 = 0.015;
/// A face goes dark while there is any gas this close to its lamp.
pub const NEAR_LAMP: i32 = 10;
/// Burnable gas this close to a lit lamp sets it off.
const LAMP_LIGHTS: i32 = 2;
/// A face singed by flame or a nearby bang is dark this long.
const SINGE_TICKS: u64 = 10 * TICKS_PER_SECOND;

/// Inside-gas a burning pocket needs to go bang (see `gas::pocket`).
pub const BANG_MIN: f32 = 10.0;

const FAN_REACH: f32 = 44.0;
const FAN_HALF_ANGLE: f32 = 0.5;
const FAN_PUSH: f32 = 0.45;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Fan,
    Door,
    Match,
    Fuse,
}

impl Tool {
    pub const ALL: [Tool; 4] = [Tool::Fan, Tool::Door, Tool::Match, Tool::Fuse];
    pub fn label(self) -> &'static str {
        match self {
            Tool::Fan => "FAN",
            Tool::Door => "DOOR",
            Tool::Match => "MATCH",
            Tool::Fuse => "FUSE",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Fan {
    pub x: i32,
    pub y: i32,
    pub angle: f32,
}

#[derive(Clone, Debug)]
pub struct Door {
    pub cells: Vec<usize>,
    pub closed: bool,
    pub x: i32,
    pub y: i32,
    pub vertical: bool,
    /// Bounding box, for taps.
    pub bbox: (i32, i32, i32, i32),
}

#[derive(Clone, Copy, Debug)]
pub struct Match {
    pub x: f32,
    pub y: f32,
    pub vy: f32,
    /// Ticks left burning once landed.
    pub life: u32,
    pub landed: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Fuse {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceState {
    Working,
    /// Gas near the lamp: the miners have downed tools.
    GasNear,
    /// No open way back to the air shaft (a door is shut, or rubble).
    NoAir,
    /// Flame or a nearby bang singed it; back to work after `dark_until`.
    Singed,
    Blown,
}

#[derive(Clone, Copy, Debug)]
pub struct Face {
    pub lamp: (i32, i32),
    pub state: FaceState,
    pub dark_until: u64,
    pub carts: u32,
    cart_timer: u64,
    pub blown_at: Option<u64>,
    /// Ticks this face spent working, for the harness.
    pub working_ticks: u64,
}

impl Face {
    pub fn name(i: usize) -> &'static str {
        if i == 0 {
            "LOW FACE"
        } else {
            "HIGH FACE"
        }
    }
}

/// Things that happened this tick, for the sound and the flash. The binary
/// drains them; the harness counts them.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    /// A match went out without lighting anything.
    Fizzle { x: i32, y: i32, thick: bool },
    /// Something lit gas.
    Lit { x: i32, y: i32 },
    /// A pocket went bang; `gas` is the gas inside it.
    Bang { x: i32, y: i32, gas: f32, radius: i32 },
    FaceBlown { x: i32, y: i32 },
    Ore { x: i32, y: i32, n: u32 },
    Cart { x: i32, y: i32 },
    Fan { x: i32, y: i32 },
    Door { x: i32, y: i32, closed: bool },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    pub matches: u32,
    pub fizzle_thick: u32,
    pub fizzle_thin: u32,
    pub lights: u32,
    pub bangs: u32,
    pub biggest_bang: f32,
    pub faces_blown: u32,
    pub fans_placed: u32,
    pub door_taps: u32,
    pub fuses_placed: u32,
    pub fuses_fired: u32,
    /// **Does the flame stop where the yellow stops?** For every deliberate
    /// light (match, fuse, lamp): the cells of the yellow pocket it lit, as
    /// a player could see them at that instant; how many of those burned in
    /// the next three seconds; and how many cells burned in that window
    /// *outside* it. Inside/predicted near 1 and outside near 0 means the
    /// colours told the truth. A bang's fireball can reach a second pocket,
    /// and two lights close together share a window, so outside is an upper
    /// bound.
    pub predicted_cells: u32,
    pub burnt_inside: u32,
    pub burnt_outside: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Stock {
    pub fans: u32,
    pub doors: u32,
    pub matches: u32,
    pub fuses: u32,
}

impl Default for Stock {
    fn default() -> Self {
        Stock { fans: 3, doors: 3, matches: 12, fuses: 4 }
    }
}

struct PendingBang {
    heart: usize,
    gas: f32,
    due: u64,
}

pub struct Game {
    pub seed: u64,
    pub tick: u64,
    pub world: World,
    pub particles: ParticleSystem,
    pub blasts: Blasts,
    pub gas: Gas,
    pub level: Level,
    pub faces: [Face; 2],
    pub fans: Vec<Fan>,
    pub doors: Vec<Door>,
    pub matches: Vec<Match>,
    pub fuses: Vec<Fuse>,
    pub stock: Stock,
    pub tool: Tool,
    pub ore_blasted: u32,
    pub counts: Counts,
    pub events: Vec<Event>,
    /// The line under the tool row: what just happened, in world words.
    pub message: Option<(String, u64)>,
    /// White-out, 0..1, decaying.
    pub flash: f32,
    pub over: bool,
    /// Pops of text in the mine ("+12 ORE"), (x, y, text, until).
    pub pops: Vec<(i32, i32, String, u64)>,
    /// A fan being aimed, by index.
    pub aiming: Option<usize>,
    /// Fans need their cones re-cast.
    fans_dirty: bool,
    pending: Vec<PendingBang>,
    /// When each cell was last claimed by a pocket check, so a pocket is
    /// measured once as the flame reaches it, not every tick.
    claimed: Vec<u64>,
    seen: Vec<bool>,
    air: Vec<bool>,
    /// The tick each cell last caught fire.
    lit_at: Vec<u64>,
    /// The tick a bang's fireball last lit each cell. A bang reaching past
    /// its own pocket is a bang, which the player sees and hears; it is
    /// left out of `Counts::burnt_outside`.
    fireball_at: Vec<u64>,
    /// Lights waiting to be scored for `Counts::burnt_inside`.
    light_log: Vec<(u64, Vec<usize>)>,
    ore_left: usize,
    last_bang_at: (i32, i32),
    player_tuning: player::Tuning,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut world = level::new_world();
        let level = level::build(&mut world, seed);
        let mut gas = Gas::new();
        gas.vents = level.vents.clone();
        let faces = [0, 1].map(|i| Face {
            lamp: level.lamps[i],
            state: FaceState::Working,
            dark_until: 0,
            carts: 0,
            cart_timer: 0,
            blown_at: None,
            working_ticks: 0,
        });
        let ore_left = level.ore.len();
        let mut g = Game {
            seed,
            tick: 0,
            world,
            particles: ParticleSystem::default(),
            blasts: Blasts::default(),
            gas,
            level,
            faces,
            fans: Vec::new(),
            doors: Vec::new(),
            matches: Vec::new(),
            fuses: Vec::new(),
            stock: Stock::default(),
            tool: Tool::Fan,
            ore_blasted: 0,
            counts: Counts::default(),
            events: Vec::new(),
            message: None,
            flash: 0.0,
            over: false,
            pops: Vec::new(),
            aiming: None,
            fans_dirty: true,
            pending: Vec::new(),
            claimed: vec![0; N],
            seen: vec![false; N],
            air: vec![false; N],
            lit_at: vec![0; N],
            fireball_at: vec![0; N],
            light_log: Vec::new(),
            ore_left,
            last_bang_at: (122, 115),
            player_tuning: player::Tuning::default(),
        };
        g.rebuild_open();
        g.say("GAS IS LEAKING INTO THE SUMP. KEEP IT OFF THE LAMPS, AND BLOW IT BY THE ORE.");
        g
    }

    pub fn ore(&self) -> u32 {
        self.ore_blasted + self.faces.iter().map(|f| f.carts).sum::<u32>()
    }

    pub fn faces_standing(&self) -> usize {
        self.faces.iter().filter(|f| f.state != FaceState::Blown).count()
    }

    /// 0 to 3, from the ore alone. A face you lose stops sending carts, and a
    /// face behind a shut door gets no fresh air and stops too, so the faces
    /// count through the ore rather than beside it -- which is what keeps a
    /// door slammed for the whole shift from being a win.
    pub fn stars(&self) -> u32 {
        ORE_STARS.iter().filter(|&&s| self.ore() >= s).count() as u32
    }

    /// The ore for the next star, or the last one once all three are won.
    pub fn next_star(&self) -> u32 {
        ORE_STARS.iter().copied().find(|&s| self.ore() < s).unwrap_or(ORE_STARS[2])
    }

    pub fn seconds_left(&self) -> u64 {
        LEVEL_TICKS.saturating_sub(self.tick) / TICKS_PER_SECOND
    }

    pub fn say(&mut self, text: &str) {
        self.message = Some((text.to_string(), self.tick + 5 * TICKS_PER_SECOND));
    }

    fn pop(&mut self, x: i32, y: i32, text: String) {
        self.pops.push((x, y, text, self.tick + 2 * TICKS_PER_SECOND));
    }

    fn door_at(&self, i: usize) -> bool {
        self.doors.iter().any(|d| d.closed && d.cells.contains(&i))
    }

    /// Gas may be where the rock is empty or holds smoke, and no shut door
    /// stands.
    fn rebuild_open(&mut self) {
        let mut shut = vec![false; N];
        for d in &self.doors {
            if d.closed {
                for &i in &d.cells {
                    shut[i] = true;
                }
            }
        }
        for y in 0..H {
            for x in 0..W {
                let i = y * W + x;
                let cell = self.world.get(x as i32, y as i32);
                let empty = cell.is_empty() || self.world.materials.get(cell.material).kind == MaterialKind::Gas;
                self.gas.open[i] = empty && !shut[i];
            }
        }
        self.gas.clear_closed();
    }

    pub fn cell_open(&self, x: i32, y: i32) -> bool {
        inside(x, y) && self.gas.open[idx(x, y)]
    }

    fn rock_empty(&self, x: i32, y: i32) -> bool {
        if !inside(x, y) {
            return false;
        }
        let cell = self.world.get(x, y);
        cell.is_empty() || self.world.materials.get(cell.material).kind == MaterialKind::Gas
    }

    // ----------------------------------------------------------------- tools

    /// A press in the mine with the current tool. A press on a door taps it,
    /// whatever the tool, because a door is the verb you need fastest.
    pub fn press(&mut self, x: i32, y: i32) {
        if self.over {
            return;
        }
        if let Some(d) = self.door_hit(x, y) {
            self.tap_door(d);
            return;
        }
        match self.tool {
            Tool::Fan => {
                if let Some(f) = self.fans.iter().position(|f| (f.x - x).pow(2) + (f.y - y).pow(2) <= 16) {
                    self.aiming = Some(f);
                    return;
                }
                if self.stock.fans == 0 {
                    self.say("NO FANS LEFT. RIGHT-CLICK A FAN TO PICK IT UP.");
                    return;
                }
                if !self.rock_empty(x, y) {
                    self.say("A FAN NEEDS OPEN TUNNEL, NOT ROCK.");
                    return;
                }
                self.place_fan(x, y, 0.0);
                self.aiming = Some(self.fans.len() - 1);
            }
            Tool::Door => {
                self.place_door(x, y);
            }
            Tool::Match => {
                self.throw_match(x, y);
            }
            Tool::Fuse => {
                self.place_fuse(x, y);
            }
        }
    }

    /// While the button is held: a fan being placed turns to face the
    /// pointer, live.
    pub fn drag(&mut self, x: i32, y: i32) {
        if let Some(f) = self.aiming {
            let fan = self.fans[f];
            let (dx, dy) = ((x - fan.x) as f32, (y - fan.y) as f32);
            if dx * dx + dy * dy >= 9.0 {
                self.fans[f].angle = dy.atan2(dx);
                self.fans_dirty = true;
            }
        }
    }

    pub fn release(&mut self, x: i32, y: i32) {
        self.drag(x, y);
        self.aiming = None;
    }

    /// Right button: pick up a fan, door or fuse, and get it back.
    pub fn pick_up(&mut self, x: i32, y: i32) {
        if let Some(f) = self.fans.iter().position(|f| (f.x - x).pow(2) + (f.y - y).pow(2) <= 25) {
            self.fans.remove(f);
            self.stock.fans += 1;
            self.fans_dirty = true;
            return;
        }
        if let Some(d) = self.door_hit(x, y) {
            self.doors.remove(d);
            self.stock.doors += 1;
            self.rebuild_open();
            return;
        }
        if let Some(f) = self.fuses.iter().position(|f| (f.x - x).abs() <= 3 && (f.y - y).abs() <= 4) {
            self.fuses.remove(f);
            self.stock.fuses += 1;
        }
    }

    pub fn place_fan(&mut self, x: i32, y: i32, angle: f32) -> bool {
        if self.stock.fans == 0 || !self.rock_empty(x, y) {
            return false;
        }
        self.stock.fans -= 1;
        self.fans.push(Fan { x, y, angle });
        self.fans_dirty = true;
        self.counts.fans_placed += 1;
        self.events.push(Event::Fan { x, y });
        true
    }

    pub fn aim_fan(&mut self, i: usize, angle: f32) {
        if let Some(f) = self.fans.get_mut(i) {
            f.angle = angle;
            self.fans_dirty = true;
        }
    }

    pub fn remove_fan(&mut self, i: usize) {
        if i < self.fans.len() {
            self.fans.remove(i);
            self.stock.fans += 1;
            self.fans_dirty = true;
        }
    }

    fn door_hit(&self, x: i32, y: i32) -> Option<usize> {
        self.doors.iter().position(|d| {
            let (x0, y0, x1, y1) = d.bbox;
            x >= x0 - 2 && x <= x1 + 2 && y >= y0 - 2 && y <= y1 + 2
        })
    }

    /// A door goes across the tunnel the short way: it runs up and down from
    /// the press until it meets rock, or sideways if that is shorter.
    pub fn place_door(&mut self, x: i32, y: i32) -> bool {
        if self.stock.doors == 0 {
            self.say("NO DOORS LEFT. RIGHT-CLICK A DOOR TO TAKE IT DOWN.");
            return false;
        }
        if !self.rock_empty(x, y) {
            self.say("A DOOR GOES IN OPEN TUNNEL, NOT ROCK.");
            return false;
        }
        let run = |dx: i32, dy: i32| {
            let mut n = 0;
            while n < 24 && self.rock_empty(x + dx * (n + 1), y + dy * (n + 1)) {
                n += 1;
            }
            n
        };
        let (up, down, left, right) = (run(0, -1), run(0, 1), run(-1, 0), run(1, 0));
        let vertical = up + down <= left + right;
        let mut cells = Vec::new();
        let (x0, y0, x1, y1);
        if vertical {
            for yy in y - up..=y + down {
                for xx in x - 1..=x + 1 {
                    if self.rock_empty(xx, yy) {
                        cells.push(idx(xx, yy));
                    }
                }
            }
            y0 = y - up;
            y1 = y + down;
            x0 = x - 1;
            x1 = x + 1;
        } else {
            for xx in x - left..=x + right {
                for yy in y - 1..=y + 1 {
                    if self.rock_empty(xx, yy) {
                        cells.push(idx(xx, yy));
                    }
                }
            }
            x0 = x - left;
            x1 = x + right;
            y0 = y - 1;
            y1 = y + 1;
        }
        self.stock.doors -= 1;
        self.doors.push(Door { cells, closed: false, x, y, vertical, bbox: (x0, y0, x1, y1) });
        let d = self.doors.len() - 1;
        self.tap_door(d);
        true
    }

    /// Slam or open. A slammed door shoves the gas in its way to either side.
    pub fn tap_door(&mut self, d: usize) {
        let closed = !self.doors[d].closed;
        self.doors[d].closed = closed;
        self.counts.door_taps += 1;
        let (x, y, vertical) = (self.doors[d].x, self.doors[d].y, self.doors[d].vertical);
        if closed {
            let cells = self.doors[d].cells.clone();
            for i in cells {
                let c = self.gas.c[i];
                if c <= 0.0 {
                    continue;
                }
                let (cx, cy) = ((i % W) as i32, (i / W) as i32);
                let (a, b) = if vertical { ((cx - 2, cy), (cx + 2, cy)) } else { ((cx, cy - 2), (cx, cy + 2)) };
                self.gas.c[i] = 0.0;
                for (px, py) in [a, b] {
                    if inside(px, py) && !self.door_at(idx(px, py)) && self.gas.open[idx(px, py)] {
                        self.gas.add(idx(px, py), c * 0.5);
                    }
                }
            }
        }
        self.rebuild_open();
        self.events.push(Event::Door { x, y, closed });
    }

    pub fn throw_match(&mut self, x: i32, y: i32) -> bool {
        if self.stock.matches == 0 {
            self.say("NO MATCHES LEFT.");
            return false;
        }
        if !self.rock_empty(x, y) {
            self.say("THAT IS SOLID ROCK.");
            return false;
        }
        self.stock.matches -= 1;
        self.counts.matches += 1;
        self.matches.push(Match { x: x as f32 + 0.5, y: y as f32, vy: 0.3, life: 50, landed: false });
        true
    }

    pub fn place_fuse(&mut self, x: i32, y: i32) -> bool {
        if self.stock.fuses == 0 {
            self.say("NO FUSES LEFT.");
            return false;
        }
        if !self.rock_empty(x, y) {
            self.say("A FUSE GOES IN OPEN TUNNEL, NOT ROCK.");
            return false;
        }
        self.stock.fuses -= 1;
        self.counts.fuses_placed += 1;
        self.fuses.push(Fuse { x, y });
        self.say("FUSE SET. IT LIGHTS BY ITSELF WHEN THE GAS ON IT TURNS YELLOW.");
        true
    }

    // ------------------------------------------------------------ the tick

    pub fn step(&mut self) {
        if self.over {
            return;
        }
        self.tick += 1;
        self.gas.lit_this_tick = 0;
        frame::step(&mut self.world, &mut self.particles, &mut self.blasts, player::PlayerInput::default(), &self.player_tuning);
        self.rebuild_open();
        if self.fans_dirty || self.tick.is_multiple_of(30) {
            self.cast_fans();
            self.fans_dirty = false;
        }
        self.leak();
        self.gas.step_flow();
        self.seep_doors();
        self.step_matches();
        self.step_fuses();
        self.gas.step_flame(self.seed, self.tick);
        self.score_lights();
        self.claim_pockets();
        self.fire_bangs();
        self.step_faces();
        self.count_ore();
        self.flash *= 0.85;
        let t = self.tick;
        self.pops.retain(|p| p.3 > t);
        if self.message.as_ref().is_some_and(|m| m.1 <= t) {
            self.message = None;
        }
        if self.tick >= LEVEL_TICKS || self.faces_standing() == 0 {
            self.over = true;
            let s = self.stars();
            let why = if self.faces_standing() == 0 { "BOTH FACES BLEW. THE MINE IS LOST." } else { "THE SHIFT IS OVER." };
            self.say(&format!("{why} {} ORE, {s} STAR{}. R TO TRY AGAIN, N FOR A NEW MINE.", self.ore(), if s == 1 { "" } else { "S" }));
            self.message = Some((self.message.take().unwrap().0, u64::MAX));
        }
    }

    fn leak(&mut self) {
        let mut rate = self.level.leak_rate;
        let t = self.tick;
        if self.level.surges.iter().any(|&s| t >= s && t < s + level::SURGE_TICKS) {
            rate *= 2.5;
        }
        // The leak slowly widens: half again by the end of the shift.
        rate *= 1.0 + 0.5 * (t as f32 / LEVEL_TICKS as f32);
        // The gas wells up through the pool like water from a spring: it
        // passes through every full cell joined to the crack and enters at
        // the *lowest* cells with room. So the pool rises flat across the
        // sump and up both arms, rather than heaping over the crack -- which
        // it did when it entered at the nearest cell with room (looked at,
        // 2026-11-07: a green mound over the leak).
        let mut left = rate;
        let mut seen = vec![false; 0];
        seen.resize(N, false);
        // **Rubble does not stop the leak.** A bang drops rubble on the
        // crack; the gas works up through it and comes out at the first open
        // cell above. Without this one early bang buried the crack and the
        // mine went quiet for the rest of the shift (seen 2026-11-07: faces
        // working all three minutes after a single light, on 9 of 12 seeds).
        let mut stack: Vec<usize> = Vec::new();
        for &i in &self.level.leak {
            let (x, y) = ((i % W) as i32, (i / W) as i32);
            if let Some(yy) = (0..40).map(|k| y - k).find(|&yy| yy >= 0 && self.gas.open[idx(x, yy)]) {
                stack.push(idx(x, yy));
            }
        }
        let mut frontier: Vec<usize> = Vec::new();
        for &i in &stack {
            seen[i] = true;
        }
        while let Some(i) = stack.pop() {
            if self.gas.c[i] < 0.97 {
                frontier.push(i);
                continue;
            }
            let (x, y) = ((i % W) as i32, (i / W) as i32);
            for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if inside(nx, ny) {
                    let j = idx(nx, ny);
                    if self.gas.open[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        frontier.sort_by_key(|&i| std::cmp::Reverse(i / W));
        for i in frontier {
            if left <= 1e-4 {
                break;
            }
            left -= self.gas.add(i, left);
        }
    }

    /// A shut door is not a seal: gas seeps through it, row by row, slowly.
    fn seep_doors(&mut self) {
        for d in &self.doors {
            if !d.closed {
                continue;
            }
            let (x0, y0, x1, y1) = d.bbox;
            let pairs: Vec<(i32, i32, i32, i32)> = if d.vertical {
                (y0..=y1).map(|y| (x0 - 1, y, x1 + 1, y)).collect()
            } else {
                (x0..=x1).map(|x| (x, y0 - 1, x, y1 + 1)).collect()
            };
            for (ax, ay, bx, by) in pairs {
                if !inside(ax, ay) || !inside(bx, by) {
                    continue;
                }
                let (a, b) = (idx(ax, ay), idx(bx, by));
                if !self.gas.open[a] || !self.gas.open[b] {
                    continue;
                }
                let f = DOOR_LEAK * (self.gas.c[a] - self.gas.c[b]);
                self.gas.c[a] -= f;
                self.gas.c[b] += f;
            }
        }
    }

    /// Re-cast every fan's cone: push along the cone, stir inside it, and
    /// nothing past rock (line of sight from the fan).
    fn cast_fans(&mut self) {
        self.gas.vx.iter_mut().for_each(|v| *v = 0.0);
        self.gas.vy.iter_mut().for_each(|v| *v = 0.0);
        self.gas.stir.iter_mut().for_each(|v| *v = 0.0);
        self.gas.cones.clear();
        let reach = FAN_REACH as i32;
        for f in &self.fans {
            let mut cone = Vec::new();
            let (ax, ay) = (f.angle.cos(), f.angle.sin());
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let (x, y) = (f.x + dx, f.y + dy);
                    if !inside(x, y) {
                        continue;
                    }
                    let d = ((dx * dx + dy * dy) as f32).sqrt();
                    if d > FAN_REACH {
                        continue;
                    }
                    let i = idx(x, y);
                    if !self.gas.open[i] {
                        continue;
                    }
                    let (ux, uy) = if d > 0.0 { (dx as f32 / d, dy as f32 / d) } else { (ax, ay) };
                    let cosang = ux * ax + uy * ay;
                    if d > 1.5 && cosang < FAN_HALF_ANGLE.cos() {
                        continue;
                    }
                    if !line_open(&self.gas.open, f.x, f.y, x, y) {
                        continue;
                    }
                    let fall = (1.0 - d / FAN_REACH).max(0.0);
                    let speed = FAN_PUSH * fall.sqrt();
                    let (px, py) = (0.75 * ax + 0.25 * ux, 0.75 * ay + 0.25 * uy);
                    self.gas.vx[i] += speed * px;
                    self.gas.vy[i] += speed * py;
                    let w = (0.35 + 0.65 * fall).min(1.0);
                    self.gas.stir[i] = (self.gas.stir[i] + w).min(1.0);
                    cone.push((i, w));
                }
            }
            self.gas.cones.push(cone);
        }
    }

    fn step_matches(&mut self) {
        let mut out = Vec::new();
        let mut keep = Vec::new();
        let ms = std::mem::take(&mut self.matches);
        for mut m in ms {
            let mut done = false;
            if !m.landed {
                m.vy = (m.vy + 0.08).min(1.6);
                let steps = m.vy.ceil() as i32;
                for _ in 0..steps {
                    let (x, y) = (m.x as i32, m.y as i32);
                    let c = self.gas.at(x, y);
                    if burnable(c) && self.gas.open[idx(x, y)] {
                        self.light_from(x, y);
                        done = true;
                        break;
                    }
                    if c > gas::BURN_HI {
                        out.push(Event::Fizzle { x, y, thick: true });
                        done = true;
                        break;
                    }
                    if !self.cell_open(x, y + 1) {
                        m.landed = true;
                        break;
                    }
                    m.y += m.vy / steps as f32;
                }
            } else {
                let (x, y) = (m.x as i32, m.y as i32);
                if let Some((px, py)) = self.gas.burnable_near(x, y, 1) {
                    self.light_from(px, py);
                    done = true;
                } else if self.gas.at(x, y) > gas::BURN_HI {
                    out.push(Event::Fizzle { x, y, thick: true });
                    done = true;
                } else {
                    m.life = m.life.saturating_sub(1);
                    if m.life == 0 {
                        out.push(Event::Fizzle { x, y, thick: false });
                        done = true;
                    }
                }
            }
            if !done {
                keep.push(m);
            }
        }
        self.matches = keep;
        for e in out {
            if let Event::Fizzle { x, y, thick } = e {
                if thick {
                    self.counts.fizzle_thick += 1;
                    self.say("FIZZLE. THE GREEN IS TOO THICK TO BURN. STIR IT WITH A FAN TO THIN IT.");
                } else {
                    self.counts.fizzle_thin += 1;
                    if self.gas.max_near(x, y, 2) > gas::CLEAR {
                        self.say("FIZZLE. THE PALE GAS IS TOO THIN TO BURN.");
                    } else {
                        self.say("THE MATCH WENT OUT. NO GAS HERE.");
                    }
                }
            }
            self.events.push(e);
        }
    }

    fn step_fuses(&mut self) {
        let mut fired = Vec::new();
        for (k, f) in self.fuses.iter().enumerate() {
            if let Some(p) = self.gas.burnable_near(f.x, f.y, 1) {
                fired.push((k, p));
            }
        }
        for &(k, (x, y)) in fired.iter().rev() {
            self.fuses.remove(k);
            self.counts.fuses_fired += 1;
            self.light_from(x, y);
        }
    }

    /// A deliberate light -- a match, a fuse, a lamp. Measures the pocket it
    /// lights, so a big one can go bang when the flame reaches its heart.
    fn light_from(&mut self, x: i32, y: i32) {
        // What the player could see: the yellow pocket joined to this spot,
        // measured before the light changes anything.
        if let Some(p) = gas::pocket(&self.gas, x, y, &mut self.seen) {
            for &c in &p.cells {
                self.seen[c] = false;
            }
            self.counts.predicted_cells += p.cells.len() as u32;
            self.light_log.push((self.tick, p.cells));
        }
        if self.gas.light(x, y) {
            self.counts.lights += 1;
            self.events.push(Event::Lit { x, y });
            self.measure_pocket(x, y);
        }
    }

    fn measure_pocket(&mut self, x: i32, y: i32) {
        let i = idx(x, y);
        if self.tick > 0 && self.claimed[i] + 180 > self.tick && self.claimed[i] != 0 {
            return;
        }
        // `seen` is reused; clear only what the last fill touched.
        if let Some(p) = gas::pocket(&self.gas, x, y, &mut self.seen) {
            for &c in &p.cells {
                self.seen[c] = false;
                self.claimed[c] = self.tick.max(1);
            }
            if p.inner_gas >= BANG_MIN {
                let (hx, hy) = p.heart;
                let dist = (((hx - x).pow(2) + (hy - y).pow(2)) as f32).sqrt();
                self.pending.push(PendingBang { heart: idx(hx, hy), gas: p.inner_gas, due: self.tick + 20 + (dist * 2.0) as u64 });
            }
        } else {
            // The light itself is no longer burnable (already lit): clear the
            // start cell's mark, which `pocket` never set.
        }
        let _ = i;
    }

    fn score_lights(&mut self) {
        let t = self.tick;
        // Stamped from the gas's own list of what caught, not by looking for
        // a fresh burn timer: a light from a match or a fireball lands outside
        // the flame step, so its timer has already ticked down once by the
        // time anything here could look. The first version looked, and so
        // never saw a fireball-lit cell burn at all -- it scored the planned
        // player's bangs as burning 34% of their pocket.
        for i in std::mem::take(&mut self.gas.just_lit) {
            self.lit_at[i] = t;
        }
        const WINDOW: u64 = 3 * TICKS_PER_SECOND;
        let due: Vec<(u64, Vec<usize>)> = {
            let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.light_log).into_iter().partition(|(t0, _)| t >= t0 + WINDOW);
            self.light_log = keep;
            due
        };
        for (t0, cells) in due {
            let mut mark = vec![false; N];
            for &c in &cells {
                mark[c] = true;
            }
            let window = |at: u64| at >= t0 && at <= t0 + WINDOW;
            self.counts.burnt_inside += cells.iter().filter(|&&c| window(self.lit_at[c])).count() as u32;
            self.counts.burnt_outside += (0..N).filter(|&c| !mark[c] && window(self.lit_at[c]) && !window(self.fireball_at[c])).count() as u32;
        }
    }

    /// The flame finds new pockets by itself as it runs: any burnable cell
    /// beside a burning one that no pocket check has claimed recently.
    fn claim_pockets(&mut self) {
        if !self.tick.is_multiple_of(3) {
            return;
        }
        let mut starts = Vec::new();
        for y in 1..H - 1 {
            for x in 1..W - 1 {
                let i = y * W + x;
                if self.gas.burn[i] == 0 {
                    continue;
                }
                for (dx, dy) in gas::NEIGHBOURS {
                    let j = (i as isize + dy * W as isize + dx) as usize;
                    if self.gas.burn[j] == 0 && self.gas.open[j] && burnable(self.gas.c[j]) && (self.claimed[j] == 0 || self.claimed[j] + 180 <= self.tick) {
                        starts.push(j);
                    }
                }
            }
        }
        for j in starts {
            if self.claimed[j] == 0 || self.claimed[j] + 180 <= self.tick {
                self.measure_pocket((j % W) as i32, (j / W) as i32);
            }
        }
    }

    fn fire_bangs(&mut self) {
        let t = self.tick;
        let mut go = Vec::new();
        self.pending.retain(|p| {
            let reached = self.gas.burn[p.heart] > 0 || self.gas.ember[p.heart] > 0;
            if reached || t >= p.due {
                go.push((p.heart, p.gas));
                false
            } else {
                true
            }
        });
        for (heart, g) in go {
            let (x, y) = ((heart % W) as i32, (heart / W) as i32);
            self.bang(x, y, g);
        }
    }

    /// Hand a pocket to the engine's explosion. Its size follows the gas
    /// that was in it.
    pub fn bang(&mut self, x: i32, y: i32, g: f32) {
        let radius = (4.0 + 1.7 * g.sqrt()).clamp(6.0, 22.0) as i32;
        let strength = (50.0 + 3.0 * g).min(220.0);
        self.blasts.trigger_with(&mut self.world, &mut self.particles, x, y, radius, strength);
        self.counts.bangs += 1;
        self.counts.biggest_bang = self.counts.biggest_bang.max(g);
        self.events.push(Event::Bang { x, y, gas: g, radius });
        self.flash = (self.flash + 0.25 + g / 80.0).min(1.0);
        self.last_bang_at = (x, y);
        // The fireball lights whatever burnable gas it reaches.
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius && self.gas.light(x + dx, y + dy) {
                    self.fireball_at[idx(x + dx, y + dy)] = self.tick;
                }
            }
        }
        // A face inside the blast is gone; one near it is singed.
        for k in 0..2 {
            let (lx, ly) = self.faces[k].lamp;
            let d2 = (lx - x).pow(2) + (ly - y).pow(2);
            if self.faces[k].state == FaceState::Blown {
                continue;
            }
            if d2 <= (radius + 2).pow(2) {
                self.blow_face(k, false);
            } else if d2 <= (radius + 10).pow(2) {
                self.singe(k);
            }
        }
        self.rebuild_open();
    }

    fn singe(&mut self, k: usize) {
        let f = &mut self.faces[k];
        if f.state != FaceState::Blown {
            f.state = FaceState::Singed;
            f.dark_until = self.tick + SINGE_TICKS;
            let n = Face::name(k);
            self.say(&format!("{n} SINGED. THE MINERS ARE BACK IN TEN SECONDS."));
        }
    }

    fn blow_face(&mut self, k: usize, with_blast: bool) {
        let (x, y) = self.faces[k].lamp;
        self.faces[k].state = FaceState::Blown;
        self.faces[k].blown_at = Some(self.tick);
        self.counts.faces_blown += 1;
        self.events.push(Event::FaceBlown { x, y });
        let n = Face::name(k);
        self.say(&format!("THE {n} BLEW UP. GAS REACHED ITS LAMP."));
        if with_blast {
            self.blasts.trigger_with(&mut self.world, &mut self.particles, x, y, 9, 130.0);
            self.flash = 1.0;
            self.rebuild_open();
        }
    }

    fn step_faces(&mut self) {
        if self.tick.is_multiple_of(15) {
            self.fill_air();
        }
        for k in 0..2 {
            if self.faces[k].state == FaceState::Blown {
                continue;
            }
            let (lx, ly) = self.faces[k].lamp;
            // A lit lamp in burnable gas: that is the face gone.
            if let Some((px, py)) = self.gas.burnable_near(lx, ly, LAMP_LIGHTS) {
                self.light_from(px, py);
                self.blow_face(k, true);
                continue;
            }
            if self.gas.burning_near(lx, ly, 4) && self.faces[k].state != FaceState::Singed {
                self.singe(k);
            }
            let f = &mut self.faces[k];
            let state = if f.state == FaceState::Singed && self.tick < f.dark_until {
                FaceState::Singed
            } else if self.gas.max_near(lx, ly, NEAR_LAMP) > gas::CLEAR {
                FaceState::GasNear
            } else if !self.air[idx(lx, ly)] {
                FaceState::NoAir
            } else {
                FaceState::Working
            };
            f.state = state;
            if state == FaceState::Working {
                f.working_ticks += 1;
                f.cart_timer += 1;
                if f.cart_timer >= CART_TICKS {
                    f.cart_timer = 0;
                    f.carts += 1;
                    self.events.push(Event::Cart { x: lx, y: ly });
                }
            }
        }
    }

    /// Which open cells have a way back to the air shaft.
    fn fill_air(&mut self) {
        self.air.iter_mut().for_each(|a| *a = false);
        let mut stack: Vec<usize> = self.level.air.iter().copied().filter(|&i| self.gas.open[i]).collect();
        for &i in &stack {
            self.air[i] = true;
        }
        while let Some(i) = stack.pop() {
            let (x, y) = ((i % W) as i32, (i / W) as i32);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if inside(nx, ny) {
                    let j = idx(nx, ny);
                    if !self.air[j] && self.gas.open[j] {
                        self.air[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
    }

    fn count_ore(&mut self) {
        if !self.tick.is_multiple_of(10) {
            return;
        }
        let ore = self.level.ore_material;
        let left = self.level.ore.iter().filter(|&&i| self.world.get((i % W) as i32, (i / W) as i32).material == ore).count();
        if left < self.ore_left {
            let n = (self.ore_left - left) as u32;
            self.ore_left = left;
            self.ore_blasted += n;
            let (x, y) = self.last_bang_at;
            self.events.push(Event::Ore { x, y, n });
            // One pop per blast: ore that keeps falling out of the same
            // crater adds to the number already showing.
            let t = self.tick;
            if let Some(p) = self.pops.iter_mut().find(|p| p.0 == x && p.1 == y - 6 && p.2.ends_with(" ORE")) {
                let had: u32 = p.2.trim_start_matches('+').trim_end_matches(" ORE").parse().unwrap_or(0);
                p.2 = format!("+{} ORE", had + n);
                p.3 = t + 2 * TICKS_PER_SECOND;
            } else {
                self.pop(x, y - 6, format!("+{n} ORE"));
            }
        }
    }

    /// For the screen's sentence: what the gas under the pointer would do.
    pub fn reading(&self, x: i32, y: i32) -> &'static str {
        if !self.rock_empty(x, y) {
            return "SOLID ROCK";
        }
        match gas::band(self.gas.at(x, y)) {
            gas::Band::Clear => "CLEAN AIR",
            gas::Band::Thin => "GAS HERE IS TOO THIN TO BURN",
            gas::Band::Burnable => "GAS HERE WILL BURN",
            gas::Band::Thick => "GAS HERE IS TOO THICK TO BURN",
        }
    }

    /// The pocket a light here would burn, and whether it would go bang --
    /// for the harness's "can you say where the flame will stop" check.
    pub fn predict(&mut self, x: i32, y: i32) -> Option<(usize, f32)> {
        let p = gas::pocket(&self.gas, x, y, &mut self.seen)?;
        for &c in &p.cells {
            self.seen[c] = false;
        }
        Some((p.cells.len(), p.inner_gas))
    }
}

/// Bresenham through open cells.
fn line_open(open: &[bool], x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let (mut x, mut y, mut err) = (x0, y0, dx + dy);
    loop {
        if !inside(x, y) || !open[idx(x, y)] {
            return false;
        }
        if x == x1 && y == y1 {
            return true;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The brief's first bar: doing nothing must lose. Measured 36-39 s to
    /// the first face over 12 seeds; the guard has headroom both ways.
    #[test]
    fn doing_nothing_loses_a_face_inside_a_minute() {
        let mut g = Game::new(1);
        while g.tick < 60 * TICKS_PER_SECOND && g.faces.iter().all(|f| f.state != FaceState::Blown) {
            g.step();
        }
        let t = g.tick as f32 / TICKS_PER_SECOND as f32;
        assert!(g.faces.iter().any(|f| f.state == FaceState::Blown), "no face blew in 60 s of doing nothing");
        assert!(t > 20.0, "a face blew at {t:.0} s, too early to react to");
    }

    /// A lit crack is a flare, not a torch: after one match on the churned
    /// sump, the fire must go out by itself. This is the setup that made the
    /// torch (seed 1, the fan switched on half a second in): without
    /// `gas::FLAME_REACH` it burnt on every tick to the end of the shift; at
    /// 255 it lasted 38 s, at 64 19 s, at 48 13 s, at 32 6 s (seed 4: 39,
    /// 21, 14, 9 s). The bound sits between 32 and 48 with headroom, so it
    /// goes red if the reach is put back up.
    #[test]
    fn a_lit_crack_goes_out() {
        let mut g = Game::new(1);
        let a = (11.0f32).atan2(20.0);
        while g.tick < 30 {
            g.step();
        }
        g.place_fan(106, 110, a);
        while !g.predict(122, 118).is_some_and(|p| p.1 >= 30.0) {
            assert!(g.tick < 30 * TICKS_PER_SECOND, "the sump never turned yellow enough to light");
            g.step();
        }
        assert!(g.throw_match(122, 108));
        let lit_at = g.tick;
        // Wait for the fire to start, then for it to go out.
        while g.gas.burning_count() == 0 && g.tick < lit_at + 2 * TICKS_PER_SECOND {
            g.step();
        }
        assert!(g.gas.burning_count() > 0, "the match lit nothing");
        while g.gas.burning_count() > 0 {
            g.step();
            assert!(g.faces.iter().all(|f| f.blown_at.is_none()), "a face blew while the flare burnt, so the test no longer measures the flare");
            assert!(g.tick < lit_at + 10 * TICKS_PER_SECOND, "still burning 10 s after the only light: the crack is a torch again");
        }
    }
}
