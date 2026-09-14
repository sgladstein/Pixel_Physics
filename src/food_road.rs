//! **The food road and the harvest map — where a colony's food comes from,
//! and how it moves.**
//!
//! Owner's ask, round 35: *"We should explore better instruments,
//! visualizations, whatever for the player to understand the food economy of
//! each colony. I want to know what they are eating, where it is coming from,
//! if/where it is being stored or movement paths, general colony food
//! stats/balances."* This module owns the two halves of that which are *seen*
//! rather than read — the path and the source. The books are a separate
//! lane's.
//!
//! **Why a picture at all, when a panel of numbers is cheaper.**
//! `Reports/instruments.md` records the owner's verdict on a contact sheet of
//! a starving colony: *"visually, I cannot tell anything from these. ants are
//! mostly visible with there motion."* An ant is two dark cells at play zoom
//! and is picked out of dark soil by **moving**. A food economy is motion by
//! definition — stuff going from where it grew to where it is eaten — so the
//! thing a player cannot get from a ledger is exactly the thing this draws:
//! the shape on the ground that a hundred small journeys add up to.
//!
//! # The two channels
//!
//! **The road** ([`FoodRoad::trail`]) is per *cell*, and that is deliberate
//! against the coarse grid below. A road is one cell wide; at any tile size
//! worth calling coarse it is a blob, and a blob cannot say *which way* the
//! food went. It carries two weights per cell — where animals walked, and
//! where they walked **carrying** — so the distinction the owner asked for
//! (laden against merely walked) is drawn rather than described.
//!
//! **The harvest map** ([`FoodRoad::harvest`]) is per *tile*, per *colony*.
//! The player wants regions — *that patch of leaf is feeding them* — and a
//! per-cell channel over the whole world would cost the dirty-rect render
//! skip for a picture nobody reads at that resolution.
//!
//! **[`FoodRoad::tile`] is 8 cells, not [`crate::sim::chunk::CHUNK_SIZE`].**
//! A chunk is 64 cells; the shipped lab bed is 512x320, so a chunk grid is
//! **8x5 tiles for the entire box** and cannot distinguish one plant from
//! the one beside it — which is the question the map exists to answer. Eight
//! gives 64x40 over the same bed, is still 1/64th of a per-cell channel, and
//! is a field rather than a constant so a harness can sweep it.
//!
//! # Both decay, and the decay is lazy
//!
//! A road that never fades is an all-time smear: after an hour every cell an
//! ant has ever crossed is lit and the picture says nothing about where the
//! colony is working *now*. So both channels are exponential in age.
//!
//! **Nothing is decayed on a timer.** Each entry stores the weight and the
//! frame it was last written, and the decay is applied when it is *read* —
//! so the per-tick cost is one hash write per animal and nothing else,
//! rather than a sweep over every cell anyone has ever visited. A periodic
//! eviction pass ([`FoodRoad::EVICT_EVERY`]) drops what has faded past
//! seeing, which is what bounds the maps on a session that runs for hours.
//!
//! # What counts as a step, and what counts as a harvest
//!
//! **A standing animal lays no road.** The mark is written only on a tick
//! where [`crate::sim::organism::LifeCounters::moves`] actually advanced. A
//! laden ant resting for five hundred ticks would otherwise burn a hotspot
//! into one cell that reads as the busiest junction in the box, and a rest is
//! not a road — round 33 found 18.5-21.9% of a bed standing still, so this is
//! not a corner case here.
//!
//! **A harvest is a bite.** `LifeCounters::bites` is *"mouthfuls taken into
//! the crop"*, so its increase between two observations is food leaving the
//! world at the animal's head, and the crop's own `unit` is the face value of
//! one cell of what went in. The weight laid down is `bites x unit` — value,
//! not events, because a mouthful of flower is three of leaf and a count
//! cannot say so.
//!
//! **Face value taken, not `creature::diet_yield`, and the difference is the
//! difference between two questions.** `diet_yield` is what *this* animal got
//! out of the mouthful — `food_value` scaled by how far the food sits from its
//! gut bias — and it is the right number for the books, which ask what the
//! colony earned. This map asks where the food *came from*, and shading a
//! patch by who happened to eat it would draw one stand of leaf two different
//! brightnesses depending on which forager reached it first. So the weight
//! here is what left the world at that tile, and every readout names it
//! `face value taken` rather than joules, because it is not what anybody ate.
//! (`Crop::unit` is a `min` over everything ingested, so a mixed load
//! understates — one-directional, and in the safe direction, which is the
//! crop's own doc's reasoning and not a second one.)
//!
//! # Where this is observed from, and the world it belongs to
//!
//! **Observed from the tick loop and from nowhere else.** [`FoodRoad::observe`]
//! is idempotent per `World::frame` so it is safe to call more than once, but
//! `Renderer::draw` deliberately does **not** call it — and that is not a
//! simplification, it is a bug that was built and removed.
//!
//! The lab holds a *rack* of worlds and draws thumbnails of the inactive ones
//! **through the same `Renderer`** (`Lab::thumb`, a forced-full draw of
//! `ch.world`). Observing at draw time therefore read another chamber's
//! animals into this chamber's map — and worse, the per-animal counter
//! readings are keyed on a `u16` organism handle that every world reissues
//! from its own first tick, so the deltas would be nonsense rather than
//! merely foreign. Cycling the rack would quietly rewrite the road.
//!
//! So the observation is the tick loop's, and the *drawing* is guarded by
//! [`FoodRoad::describes`]: the channels are drawn only for the world this map
//! was actually built from. A chamber thumbnail draws with no overlay rather
//! than with somebody else's.
//!
//! **The consequence, stated rather than left to be discovered: a binary that
//! only draws gets nothing.** Today that is only the outdoor sandbox, which
//! has no key for this view; whoever gives it one has to hook `observe` into
//! that game's own tick. [`FoodRoad::observed_frames`] against the frames
//! actually run is what says whether that was done — a road built from one
//! sample in two hundred is a smear, and a smear and a road look identical in
//! a picture.
//!
//! **It is free when the overlay is off.** `observe` returns on one enum
//! compare, the maps stay empty, and `Renderer::draw` never asks them
//! anything — the same "zero cost until opted in" shape `field_overlay`
//! already has. The corollary is that turning it on shows an empty world
//! until the animals move again, which is correct for a channel whose whole
//! claim is *this is the road they are using now*.

use crate::sim::cell::OrganismId;
use std::collections::HashMap;

use crate::sim::world::World;

/// Which of the two channels [`FoodRoad`] draws, cycled by `F7` in the lab.
///
/// `Off` by default. A *look* ships default-off pending the owner's eye —
/// the lab's standing split between a look and a behaviour, where *"ship
/// everything on"* governs only the latter.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FoodOverlay {
    #[default]
    Off,
    /// The road alone: where animals walked, and where they walked laden.
    Road,
    /// The harvest map alone: which regions the food came out of, in each
    /// colony's own colour.
    Harvest,
    /// Both, road over harvest. The one picture that answers the whole ask —
    /// coloured patches saying *whose food, and from where*, with the bright
    /// laden roads running out of them saying *and this is how it travels*.
    Both,
}

impl FoodOverlay {
    pub fn next(self) -> Self {
        match self {
            FoodOverlay::Off => FoodOverlay::Road,
            FoodOverlay::Road => FoodOverlay::Harvest,
            FoodOverlay::Harvest => FoodOverlay::Both,
            FoodOverlay::Both => FoodOverlay::Off,
        }
    }

    /// The HUD caption. Named for what it shows a player, not for the field
    /// it reads — `CLAUDE.md`'s rule on naming a readout for what it says
    /// about the world.
    pub fn label(self) -> &'static str {
        match self {
            FoodOverlay::Off => "OFF",
            FoodOverlay::Road => "FOOD ROAD",
            FoodOverlay::Harvest => "HARVEST MAP",
            FoodOverlay::Both => "ROAD + HARVEST",
        }
    }
}

/// What one harvest tile draws: the colour, and how much of the tile the
/// wash takes. See [`HARVEST_DITHER`].
#[derive(Clone, Copy, Debug)]
pub struct TileMark {
    pub rgb: [f32; 3],
    pub cover: f32,
}

impl TileMark {
    /// Whether this cell of the tile is one the wash paints. The pattern is
    /// keyed on *world* coordinates rather than on the offset within the
    /// tile, so it does not restart at every tile edge and draw a grid.
    pub fn covers(self, x: i32, y: i32) -> bool {
        HARVEST_DITHER[y.rem_euclid(4) as usize][x.rem_euclid(4) as usize] <= self.cover
    }
}

/// One decaying accumulator: a weight and the frame it was last added to.
/// See the module doc on why the decay is applied at read.
#[derive(Clone, Copy, Debug, Default)]
struct Mark {
    weight: f32,
    stamp: u64,
}

impl Mark {
    /// The weight as of `now`, `half_life` in frames.
    fn read(self, now: u64, half_life: f32) -> f32 {
        let age = now.saturating_sub(self.stamp) as f32;
        self.weight * 0.5f32.powf(age / half_life)
    }

    fn add(&mut self, amount: f32, now: u64, half_life: f32) {
        self.weight = self.read(now, half_life) + amount;
        self.stamp = now;
    }
}

/// What one cell of road carries.
#[derive(Clone, Copy, Debug, Default)]
struct RoadCell {
    /// Steps taken across this cell with nothing in the crop.
    walked: Mark,
    /// Steps taken across it **carrying** — the food road proper.
    laden: Mark,
}

/// Per-animal state carried between observations, so a *change* can be read
/// off counters that only ever go up.
#[derive(Clone, Copy, Debug)]
struct Seen {
    moves: u32,
    bites: u32,
}

/// The two channels, their counters, and the knobs a harness sweeps.
///
/// Held on `Renderer` (beside `idle_tracks`, which is observational state of
/// the same kind) so that a binary which only draws still gets a sampled
/// road for free. See the module doc.
#[derive(Debug, Default)]
pub struct FoodRoad {
    /// Which channel is drawn. `Off` is free — see the module doc.
    pub mode: FoodOverlay,
    /// Cells of road, sparse: only what has actually been walked.
    trail: HashMap<(i32, i32), RoadCell>,
    /// Harvest by `(colony, tile)`, sparse: only tiles food has come out of.
    harvest: HashMap<(u32, (i32, i32)), Mark>,
    /// Per-animal counter readings as of the last observation.
    seen: HashMap<OrganismId, Seen>,
    /// The last `World::frame` observed, so `observe` is idempotent per tick
    /// and can be called from both the tick loop and `draw`.
    last_frame: Option<u64>,
    /// The bounds of the world last observed — half of the "is this still
    /// the same world" check in [`FoodRoad::observe`].
    last_bounds: Option<crate::sim::chunk::Rect>,

    // ---- knobs, fields rather than constants so a harness can sweep them ----
    /// Cells to a side of one harvest tile. See the module doc on why this is
    /// 8 and not a chunk.
    pub tile: i32,
    /// **Whether the empty-handed road is drawn at all.**
    ///
    /// A runtime selector rather than a decision, which is this repo's
    /// standing convention for *"does this look right"*: five grain modes
    /// behind one key settled in minutes a question no amount of argument or
    /// still images had. The two readings it separates are real and the beds
    /// disagree about which is right — on the played bed nearly every step is
    /// laden (392 against 44 over 3,600 frames) and the cool channel is a few
    /// marks of context, while on `far_larder` the colony wanders without
    /// finding anything and the cool channel is most of what is on screen.
    /// Which of those the owner wants is a question about the picture.
    ///
    /// `true` ships, because a road with no *"and here is where they went
    /// empty-handed"* cannot show the difference the owner asked to see.
    pub show_walked: bool,
    /// Frames for a road mark to halve. Ten seconds at the shipped 60 Hz —
    /// short, because the road's claim is *now*.
    pub road_half_life: f32,
    /// Frames for a harvest mark to halve. A minute: where the food has been
    /// coming from is a slower question than where the traffic is, and a
    /// patch that has just been stripped should stay lit long enough to be
    /// read as the reason the road leads there.
    pub harvest_half_life: f32,
    /// **Fix the road ramp at this weight instead of tracking the bed.**
    /// `None` — the default — is the adaptive scale; see [`Self::refresh`]
    /// for why, and what it costs. Set it to compare two sheets on one bar.
    pub road_full: Option<f32>,
    /// The same for the harvest ramp, in face value.
    pub harvest_full: Option<f32>,
    /// The road weight currently drawn as full brightness. Tracked rather
    /// than authored — [`Self::refresh`].
    road_scale: f32,
    /// The harvest face value currently drawn as full brightness.
    harvest_scale: f32,
    /// The frame [`Self::refresh`] last ran, so the easing is measured in
    /// simulated time rather than in calls — see that method.
    last_refresh: Option<u64>,

    // ---- counters: "did it fire at all" needs a number, not a picture ----
    /// Ticks observed. Against the frames actually run, this is how sampled
    /// the road is — see the module doc.
    pub observed_frames: u64,
    /// Animal-ticks marked as a laden step.
    pub laden_steps: u64,
    /// Animal-ticks marked as an unladen step.
    pub walked_steps: u64,
    /// Bites seen, i.e. harvest events attributed to a tile.
    pub harvest_bites: u64,
    /// Face value those bites took out of the world.
    pub harvest_value: f64,
}

impl FoodRoad {
    /// Ticks between eviction passes. The pass is O(live entries) and the
    /// entries it drops are below seeing, so this trades a little memory for
    /// keeping the per-tick cost at one hash write per animal.
    pub const EVICT_EVERY: u64 = 256;
    /// Read weight below which an entry is dropped by the eviction pass, as
    /// a fraction of the channel's `_full`. Under the ramp floor, so nothing
    /// visible is ever evicted.
    const EVICT_BELOW: f32 = 0.01;

    pub fn new() -> Self {
        FoodRoad {
            mode: FoodOverlay::Off,
            trail: HashMap::new(),
            harvest: HashMap::new(),
            seen: HashMap::new(),
            last_frame: None,
            last_bounds: None,
            tile: 8,
            show_walked: true,
            road_half_life: 600.0,
            harvest_half_life: 3600.0,
            road_full: None,
            harvest_full: None,
            road_scale: ROAD_SCALE_MIN,
            harvest_scale: HARVEST_SCALE_MIN,
            last_refresh: None,
            observed_frames: 0,
            laden_steps: 0,
            walked_steps: 0,
            harvest_bites: 0,
            harvest_value: 0.0,
        }
    }

    /// Drop everything, including the counters — for a rebuilt box, where
    /// carrying a road over from the previous world would draw a map of a
    /// place that no longer exists.
    pub fn forget(&mut self) {
        self.trail.clear();
        self.harvest.clear();
        self.seen.clear();
        self.last_frame = None;
        self.last_bounds = None;
        self.road_scale = ROAD_SCALE_MIN;
        self.harvest_scale = HARVEST_SCALE_MIN;
        self.last_refresh = None;
        self.observed_frames = 0;
        self.laden_steps = 0;
        self.walked_steps = 0;
        self.harvest_bites = 0;
        self.harvest_value = 0.0;
    }

    /// Take one reading of the world.
    ///
    /// Idempotent per `World::frame`, so the tick loop and `Renderer::draw`
    /// can both call it. Returns immediately — before touching anything —
    /// while the overlay is off.
    pub fn observe(&mut self, world: &World) {
        if self.mode == FoodOverlay::Off {
            // Not merely a skip: a road accumulated while nothing is drawing
            // it is an hour of memory bought for a picture nobody asked for.
            // The cost of the other side of this trade is documented in the
            // module doc -- turning the overlay on starts from an empty map.
            if !self.trail.is_empty() || !self.harvest.is_empty() || !self.seen.is_empty() {
                self.forget();
            }
            return;
        }
        let now = world.frame;
        if self.last_frame == Some(now) {
            return;
        }
        // **The world under this map can be swapped out without anyone
        // telling the renderer** — the lab's rack holds several chambers and
        // `ENTER` walks into one, where `App::reset`'s `forget_world` seam
        // does not exist. A road belongs to the ground it was walked on, and
        // worse, the per-animal counter readings behind it are keyed on a
        // `u16` handle the new world reuses from its own first tick.
        //
        // A box of a different size, or a clock that has gone backwards, is
        // certainly another world. A swap between two same-sized chambers at
        // a higher frame is not caught here; the per-animal guard below is
        // what covers that, and what is left over is one tile briefly too
        // bright rather than a whole life landing in one cell.
        let identity = (world.bounds(), now);
        let swapped = match (self.last_frame, self.last_bounds) {
            (Some(last), bounds) => now < last || bounds != identity.0,
            (None, _) => false,
        };
        if swapped {
            self.forget();
        }
        self.last_bounds = identity.0;
        self.last_frame = Some(now);
        self.observed_frames += 1;

        let live = world.live_organism_ids();
        let mut alive: std::collections::HashSet<OrganismId> = std::collections::HashSet::with_capacity(live.len());
        for id in live {
            let Some(state) = world.organism(id) else { continue };
            // Plants share the state type and have no `moves` or `bites` to
            // speak of; both read 0 for ever, so every one of them would
            // enrol as a permanently-still animal.
            if world.species.get(state.species).creature.is_none() {
                continue;
            }
            alive.insert(id);
            let Some(&(hx, hy)) = state.chain.first() else { continue };
            let previous = self.seen.insert(id, Seen { moves: state.life.moves, bites: state.life.bites });
            // An animal seen for the first time contributes nothing this
            // tick: its counters are absolute totals, and reading them as a
            // delta would lay down its whole life at whatever cell it
            // happens to be standing on. `CLAUDE.md`'s "ask what your number
            // counts when nothing is wrong" -- here the wrong answer is a
            // brand-new colony whose entire history lights one tile.
            let Some(previous) = previous else { continue };
            // **A counter that has gone backwards is not this animal's.**
            // `moves` and `bites` only ever climb, so a fall means the `u16`
            // handle has been reclaimed and reissued — at which point the
            // stored reading belongs to somebody dead and the delta against
            // it is a whole life, laid down in one tick at whatever cell the
            // newcomer is standing on. Dropped to a first sighting instead,
            // which costs one tick of one animal's road.
            if state.life.moves < previous.moves || state.life.bites < previous.bites {
                continue;
            }

            if state.life.moves != previous.moves {
                let laden = state.crop.is_some_and(|c| c.cells > 0);
                let cell = self.trail.entry((hx, hy)).or_default();
                if laden {
                    cell.laden.add(1.0, now, self.road_half_life);
                    self.laden_steps += 1;
                } else {
                    cell.walked.add(1.0, now, self.road_half_life);
                    self.walked_steps += 1;
                }
            }

            let bites = state.life.bites.saturating_sub(previous.bites);
            if bites > 0 {
                // The crop holds what was just taken, and `unit` is the face
                // value of one cell of it -- so this is value leaving the
                // world here, not a count of mouthfuls. A crop emptied by a
                // delivery in the same window has no `unit` left to read;
                // the species' own appetite for what is under the head is
                // the honest fallback, and a flat 1.0 is the last resort.
                let unit = state
                    .crop
                    .map(|c| c.unit)
                    .filter(|u| *u > 0.0)
                    .unwrap_or_else(|| fallback_unit(world, hx, hy));
                let value = bites as f32 * unit;
                let tile = (hx.div_euclid(self.tile), hy.div_euclid(self.tile));
                self.harvest.entry((state.colony, tile)).or_default().add(value, now, self.harvest_half_life);
                self.harvest_bites += bites as u64;
                self.harvest_value += value as f64;
            }
        }
        // A handle this map never revisits (death, starvation, a colony wiped
        // out) must not sit in it for ever -- and worse, `u16` handles are
        // reused, so a stale entry would be read as the *new* animal's
        // previous counters and lay down its whole life in one tick.
        self.seen.retain(|id, _| alive.contains(id));

        if now.is_multiple_of(Self::EVICT_EVERY) {
            self.evict(now);
        }
    }

    /// Drop what has faded past seeing. See [`Self::EVICT_EVERY`].
    ///
    /// **Against the tracked scale, so the floor follows the bed.** A fixed
    /// floor would either hold a hundred thousand faded cells on a busy bed
    /// or evict the only three marks on a quiet one.
    fn evict(&mut self, now: u64) {
        let road_floor = self.road_scale * Self::EVICT_BELOW;
        let half = self.road_half_life;
        self.trail.retain(|_, c| c.walked.read(now, half) >= road_floor || c.laden.read(now, half) >= road_floor);
        let harvest_floor = self.harvest_scale * Self::EVICT_BELOW;
        let half = self.harvest_half_life;
        self.harvest.retain(|_, m| m.read(now, half) >= harvest_floor);
    }

    /// **Re-aim both ramps at the bed in front of them, once per draw.**
    ///
    /// # Why the scale is tracked rather than authored
    ///
    /// A fixed bar was tried first and cannot span the beds this has to
    /// draw. Measured over 9,000 frames: the played bed's heaviest harvest
    /// tile held **92,699** of face value, and `far_larder`'s — a colony
    /// that never reaches its food and ends up eating its own dead — held
    /// **753**. That is two and a half orders of magnitude between two
    /// shipped scenarios, so any single constant renders one of them as a
    /// white square and the other as an empty world. `CLAUDE.md`: a bar set
    /// from one run is a sample from a wide distribution.
    ///
    /// **What that costs, stated rather than hidden: two sheets are not on
    /// one bar.** A tile half as bright in a later frame may be half as fed
    /// or may be sharing the map with something richer. The channel's
    /// question — *which regions feed this colony* — is a relative one and
    /// survives that; a quantitative one does not, and goes to
    /// `examples/foodroad.rs`, which prints the raw values. Set
    /// [`Self::harvest_full`] or [`Self::road_full`] to pin a bar when two
    /// sheets really must be compared.
    ///
    /// **Both road channels share one scale**, deliberately: the whole claim
    /// of the road is that laden traffic looks different from empty-handed
    /// traffic, and normalising the two separately would erase the ratio
    /// that says so.
    ///
    /// Smoothed toward the live maximum rather than snapped to it, or one
    /// busy frame dims the entire map and the picture flickers.
    ///
    /// **The smoothing is in simulated frames, not in calls, and that
    /// distinction was a real defect for one run.** Written as a fixed
    /// fraction per call it converged in about a second in the app — which
    /// draws every frame — and took **3,600 frames to get two thirds of the
    /// way** in a harness capturing one frame in four hundred, so every
    /// sheet it wrote was drawn on a ramp aimed at a bed an hour out of
    /// date. The tell was the probe: `max 17.05 of full` is a channel whose
    /// scale has not caught up, and it is printed beside every frame for
    /// exactly that reason.
    ///
    /// The first refresh after a reset snaps instead of easing, so a card's
    /// opening frame is not a picture of the floor.
    pub fn refresh(&mut self, now: u64) {
        if !self.on() {
            return;
        }
        let alpha = match self.last_refresh {
            None => 1.0,
            Some(then) => {
                let elapsed = now.saturating_sub(then).max(1) as f32;
                1.0 - (1.0 - SCALE_TRACK).powf(elapsed)
            }
        };
        self.last_refresh = Some(now);
        let half = self.road_half_life;
        let road_max = self
            .trail
            .values()
            .map(|c| c.walked.read(now, half).max(c.laden.read(now, half)))
            .fold(0.0f32, f32::max);
        self.road_scale += (road_max.max(ROAD_SCALE_MIN) - self.road_scale) * alpha;
        let half = self.harvest_half_life;
        let harvest_max = self.harvest.values().map(|m| m.read(now, half)).fold(0.0f32, f32::max);
        self.harvest_scale += (harvest_max.max(HARVEST_SCALE_MIN) - self.harvest_scale) * alpha;
    }

    /// The road weight drawn as full brightness right now — the override if
    /// one is set, else the tracked scale.
    pub fn road_scale(&self) -> f32 {
        self.road_full.unwrap_or(self.road_scale).max(f32::EPSILON)
    }

    /// The harvest face value drawn as full brightness right now.
    pub fn harvest_scale(&self) -> f32 {
        self.harvest_full.unwrap_or(self.harvest_scale).max(f32::EPSILON)
    }

    /// Whether anything is drawn at all — one enum compare, so the per-pixel
    /// path can hoist it.
    pub fn on(&self) -> bool {
        self.mode != FoodOverlay::Off
    }

    /// **Is this the world this map was built from?**
    ///
    /// The guard that keeps a chamber thumbnail from being painted with the
    /// active box's road — see the module doc. Two cheap comparisons: the
    /// clock and the bounds. A second world at the identical frame *and*
    /// identical size is not separated by this and does not need to be; what
    /// it would cost is one thumbnail wearing the wrong overlay, against the
    /// alternative of the map itself being rewritten.
    pub fn describes(&self, world: &World) -> bool {
        self.on() && self.last_frame == Some(world.frame) && self.last_bounds == world.bounds()
    }

    /// The per-tile colour lookup for one frame, resolved **once per draw**
    /// rather than per pixel.
    ///
    /// A tile's colour is the colour of the colony taking the most out of it,
    /// and its brightness is the total across every colony there. Resolving
    /// that per pixel would mean walking every colony's entry for every
    /// pixel of a 64-cell-wide tile.
    pub fn tile_colours(&self, now: u64) -> HashMap<(i32, i32), TileMark> {
        let scale = self.harvest_scale();
        let mut total: HashMap<(i32, i32), (f32, u32, f32)> = HashMap::new();
        for (&(colony, tile), mark) in &self.harvest {
            let v = mark.read(now, self.harvest_half_life);
            let slot = total.entry(tile).or_insert((0.0, colony, 0.0));
            slot.0 += v;
            if v > slot.2 {
                slot.1 = colony;
                slot.2 = v;
            }
        }
        total
            .into_iter()
            .map(|(tile, (sum, colony, _))| {
                let t = (sum / scale).clamp(0.0, 1.0);
                let hue = colony_hue(colony);
                let k = ramp(t);
                (tile, TileMark { rgb: [hue[0] * k, hue[1] * k, hue[2] * k], cover: ramp(t) * HARVEST_MAX_COVER })
            })
            .collect()
    }

    /// The road's colour at one cell, or `None` where no road runs.
    ///
    /// **A full replace on a fixed dark-to-bright ramp, never a blend into
    /// the cell's own colour.** `CLAUDE.md` records what a blend costs: a
    /// canopy-density sheet read as blank because the ramp was red, wood is
    /// brown, and a mid-range value moved one colour byte from 139 to 155 —
    /// and the obvious reading, *"the mechanism is dead"*, would have sent a
    /// fix at working code. A road is one cell wide over dirt, which is that
    /// case exactly.
    pub fn road_at(&self, x: i32, y: i32, now: u64) -> Option<[f32; 3]> {
        let cell = self.trail.get(&(x, y))?;
        let full = self.road_scale();
        let laden = cell.laden.read(now, self.road_half_life) / full;
        let walked = cell.walked.read(now, self.road_half_life) / full;
        // **Laden wins wherever there is any of it**, rather than whichever
        // weight is larger. The question the channel answers is *where does
        // the food go*, and a haul route is also the route everyone walks
        // back along empty -- scoring the two against each other would erase
        // the road exactly where it is busiest.
        if laden >= VISIBLE {
            Some(two_stop(ROAD_LADEN_LOW, ROAD_LADEN_HIGH, laden))
        } else if self.show_walked && walked >= VISIBLE {
            Some(two_stop(ROAD_WALKED_LOW, ROAD_WALKED_HIGH, walked))
        } else {
            None
        }
    }

    // ---- readouts for the probe: an overlay must be paired with numbers ----

    /// Every live road cell as `(x, y, walked, laden)`, decayed to `now`.
    ///
    /// **Raw weight, not a fraction of the ramp** — the ramp is tracked
    /// ([`Self::refresh`]), so a fraction of it cannot say whether the scale
    /// is aimed anywhere near the data, which is the one thing a probe
    /// beside an adaptive overlay exists to answer.
    pub fn road_readout(&self, now: u64) -> Vec<(i32, i32, f32, f32)> {
        self.trail
            .iter()
            .map(|(&(x, y), c)| (x, y, c.walked.read(now, self.road_half_life), c.laden.read(now, self.road_half_life)))
            .collect()
    }

    /// Every live harvest entry as `(colony, tile_x, tile_y, value)`, decayed
    /// to `now`. **Face value, not a fraction** — the probe needs the raw
    /// scale to say whether `harvest_full` is set anywhere near the data.
    pub fn harvest_readout(&self, now: u64) -> Vec<(u32, i32, i32, f32)> {
        self.harvest.iter().map(|(&(colony, (tx, ty)), m)| (colony, tx, ty, m.read(now, self.harvest_half_life))).collect()
    }

    /// How many cells of road and tiles of harvest are being held.
    pub fn footprint(&self) -> (usize, usize, usize) {
        (self.trail.len(), self.harvest.len(), self.seen.len())
    }
}

/// The face value of one cell of whatever is under an animal's head, for the
/// case where the crop has already been emptied by the time the bite is seen.
///
/// Through `creature::food_value`, which is the same pricing the eat verb
/// itself uses — including a corpse's per-cell worth out of `Cell::aux`. A
/// second spelling of the price table here would be a second thing to keep
/// in step, and a map drawn on a stale one would be wrong in exactly the way
/// that looks like a result.
fn fallback_unit(world: &World, x: i32, y: i32) -> f32 {
    let v = crate::sim::creature::food_value(world, world.get(x, y));
    if v > 0.0 {
        v
    } else {
        1.0
    }
}

/// **The road's two ramps, each a pair of stops rather than one colour
/// scaled toward black — and the reason is that the surface of this world is
/// pale.**
///
/// The first build scaled a single amber toward black on `render.rs`'s
/// `scalar_ramp` pattern, which is right for a channel drawn over dark rock
/// and wrong here: a road is laid at the animal's *head*, an ant walks on
/// the lit surface band, and a dim amber over cream ground is a stain rather
/// than a line. Rendered and looked at, most of the road was simply not
/// there — `CLAUDE.md`'s *look before you measure*, and the counters said
/// 262 cells the whole time.
///
/// So each channel runs between two stops, and what carries a quiet reading
/// is **saturation** rather than dimness: the low end of the laden road is a
/// saturated red-orange, which nothing in soil, leaf, bark or sky is, and
/// the high end is a near-white heat. Cool against hot is then the
/// categorical split between merely-walked and carrying, readable before any
/// brightness is compared.
const ROAD_LADEN_LOW: [f32; 3] = [255.0, 72.0, 24.0];
const ROAD_LADEN_HIGH: [f32; 3] = [255.0, 246.0, 196.0];
const ROAD_WALKED_LOW: [f32; 3] = [46.0, 66.0, 116.0];
const ROAD_WALKED_HIGH: [f32; 3] = [138.0, 186.0, 246.0];

/// **The harvest wash is dithered, not painted flat, and that is what keeps
/// it from erasing the food it is pointing at.**
///
/// A flat full replace over an 8-cell tile was built first and looked wrong
/// the moment it was rendered: the heaviest tiles sit exactly on the plants
/// being eaten, so the map covered its own subject with a solid amber block
/// and the picture could no longer say whether there was anything there. A
/// blend is not the alternative — `CLAUDE.md` records what a magnitude-
/// scaled blend costs — so the pixels this takes are still a full replace;
/// there are just fewer of them the less a tile has given up.
///
/// **Coverage is the magnitude, alongside brightness.** That is the first
/// law applied to a readout: a tile that has fed the colony a little is a
/// scatter of dots, one that has fed it everything is nearly solid, and
/// there is a middle rather than lit-or-unlit.
///
/// Ordered 4x4, so the pattern is stable between frames — a random dither
/// would crawl, and a crawling region is a region that looks like it is
/// moving.
const HARVEST_DITHER: [[f32; 4]; 4] =
    [[0.0625, 0.5625, 0.1875, 0.6875], [0.8125, 0.3125, 0.9375, 0.4375], [0.25, 0.75, 0.125, 0.625], [1.0, 0.5, 0.875, 0.375]];

/// The most of a tile the wash may ever take, so even a tile the whole
/// colony is living off still shows the ground it is drawn over.
const HARVEST_MAX_COVER: f32 = 0.8;

/// The floor of every ramp here, matching `render.rs`'s `SCALAR_RAMP_FLOOR`:
/// a faint mark is still a mark, and a road's thin end should recede without
/// vanishing.
const RAMP_FLOOR: f32 = 0.18;

/// The fraction of `road_full` below which a cell is not drawn at all.
///
/// Not zero: without it every cell any animal has crossed in the last ten
/// seconds is lit at the ramp floor, and the picture is a fog rather than a
/// road. Deliberately the same order as [`FoodRoad::EVICT_BELOW`] so that
/// what is held and what is seen do not drift far apart.
const VISIBLE: f32 = 0.02;

/// The floor under the tracked road scale — a bed with two marks on it must
/// not draw both at full brightness and read as a motorway. Set at the
/// measured p90 of the quietest bed tried (`far_larder`, a colony that never
/// reaches its food): 1.7 laden and 6.0 walked in raw weight.
const ROAD_SCALE_MIN: f32 = 2.0;
/// The same for harvest, in face value: one mouthful of the cheapest thing
/// an ant eats. Below this a tile holds less than a single bite and there is
/// nothing there to rank.
const HARVEST_SCALE_MIN: f32 = 480.0;
/// How far the tracked scale moves toward the live maximum each `refresh`.
/// Slow enough that one busy frame does not dim the whole map, fast enough
/// that a colony moving to a new patch is followed within a few seconds.
const SCALE_TRACK: f32 = 0.05;

/// **Square root, not linear, and against this module's own `scalar_ramp`
/// heritage.**
///
/// `render.rs`'s field ramps are deliberately linear, on the argument that a
/// perceptual curve makes "how much" harder to judge between two tiles of a
/// contact sheet. That argument holds for a field channel, whose readings
/// across a world span maybe one order of magnitude. It fails here, and the
/// measurement is the reason: on the played bed the harvest tiles ran
/// **p50 2,280, p90 24,780, max 92,699** — a fortyfold spread between the
/// median tile and the heaviest. Drawn linearly that is one white tile and
/// thirty-seven black ones, which is `CLAUDE.md`'s first law failing exactly
/// as the uniform-powder rubble did: an outcome is a distribution, and a map
/// that can only say "here" and "nowhere" has no middle.
///
/// Under a square root the same three land at 0.16, 0.52 and 1.00. The cost
/// is real and is stated rather than hidden: twice as bright is four times
/// the food, so this ranks regions and does not measure them. The measuring
/// is `examples/foodroad.rs`'s, which prints the raw values beside the
/// picture — the pairing `CLAUDE.md` requires of every debug channel.
fn ramp(t: f32) -> f32 {
    RAMP_FLOOR + (1.0 - RAMP_FLOOR) * t.clamp(0.0, 1.0).sqrt()
}

/// Walk a reading from the channel's quiet stop to its loud one, on the
/// square-root curve [`ramp`] documents.
fn two_stop(low: [f32; 3], high: [f32; 3], t: f32) -> [f32; 3] {
    let k = t.clamp(0.0, 1.0).sqrt();
    [low[0] + (high[0] - low[0]) * k, low[1] + (high[1] - low[1]) * k, low[2] + (high[2] - low[2]) * k]
}

/// A colony's hue, taken from the same palette the animals themselves wear
/// under `CreatureColour::Colony` — so a harvest patch and the ants coming
/// out of it are the same colour, deliberately, rather than two unrelated
/// codings of one fact.
fn colony_hue(colony: u32) -> [f32; 3] {
    if colony == 0 {
        // Animals of no colony: the same neutral the creature tint uses for
        // them, so "nobody's" reads as nobody's here too.
        [190.0, 190.0, 190.0]
    } else {
        crate::render::group_palette(colony as usize - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mark_halves_over_its_half_life() {
        let mut m = Mark::default();
        m.add(1.0, 0, 100.0);
        assert!((m.read(100, 100.0) - 0.5).abs() < 1e-5, "one half-life is half: {}", m.read(100, 100.0));
        assert!((m.read(200, 100.0) - 0.25).abs() < 1e-5, "two half-lives is a quarter");
    }

    #[test]
    fn adding_to_a_decayed_mark_adds_to_the_decayed_value_not_the_stored_one() {
        // The whole point of the lazy decay: a cell walked once, left for a
        // half-life, then walked again must read 1.5 and not 2.0 -- or a road
        // nobody has used for an hour comes back to full brightness on one
        // step.
        let mut m = Mark::default();
        m.add(1.0, 0, 100.0);
        m.add(1.0, 100, 100.0);
        assert!((m.read(100, 100.0) - 1.5).abs() < 1e-5, "got {}", m.read(100, 100.0));
    }

    #[test]
    fn the_ramp_floor_is_not_black() {
        assert!(ramp(0.0) > 0.1, "a faint mark must still be a mark");
        assert!((ramp(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_laden_cell_wins_over_a_walked_one_even_when_it_is_fainter() {
        // The haul route is also the route everyone walks back along empty,
        // so scoring the two against each other erases the road where it is
        // busiest. Documented on `road_at`; asserted here so a later
        // "simplification" to `max` fails.
        let mut road = FoodRoad::new();
        road.mode = FoodOverlay::Road;
        let cell = road.trail.entry((3, 4)).or_default();
        cell.walked.add(10.0, 0, road.road_half_life);
        cell.laden.add(1.0, 0, road.road_half_life);
        road.refresh(0);
        let got = road.road_at(3, 4, 0).expect("a walked cell draws");
        assert!(got[2] < got[0], "laden is amber (red-dominant), got {got:?}");
    }

    #[test]
    fn off_costs_nothing_and_holds_nothing() {
        let mut road = FoodRoad::new();
        road.trail.insert((1, 1), RoadCell::default());
        let mut w = World::new(crate::sim::chunk::Rect::new(0, 0, 63, 63));
        road.observe(&w);
        assert!(road.trail.is_empty(), "off drops what it was holding rather than accumulating");
        assert_eq!(road.observed_frames, 0, "off observes nothing");
        w.frame += 1;
        road.observe(&w);
        assert_eq!(road.observed_frames, 0);
    }

    #[test]
    fn observing_twice_in_one_frame_counts_once() {
        let mut road = FoodRoad::new();
        road.mode = FoodOverlay::Road;
        let w = World::new(crate::sim::chunk::Rect::new(0, 0, 63, 63));
        road.observe(&w);
        road.observe(&w);
        assert_eq!(road.observed_frames, 1, "idempotent per world frame -- the tick loop and `draw` both call it");
    }

    #[test]
    fn the_ramp_eases_in_simulated_time_not_in_calls() {
        // The defect this is named for: a fixed fraction per call converges
        // in a second in the app and takes an hour in a harness capturing at
        // a stride, so every sheet is drawn on a stale ramp. Two refreshes
        // 400 frames apart must reach what 400 consecutive ones reach.
        let mut a = FoodRoad::new();
        a.mode = FoodOverlay::Road;
        let mut b = FoodRoad::new();
        b.mode = FoodOverlay::Road;
        // No decay, so the two arms see the same live maximum throughout.
        for r in [&mut a, &mut b] {
            r.road_half_life = 1.0e9;
            r.trail.entry((0, 0)).or_default().laden.add(100.0, 0, r.road_half_life);
            r.refresh(0);
            // The first refresh snaps, so both start converged; move them
            // off it together and let the easing do the work.
            r.road_scale = ROAD_SCALE_MIN;
        }
        a.refresh(400);
        for f in 1..=400 {
            b.refresh(f);
        }
        let (x, y) = (a.road_scale(), b.road_scale());
        assert!((x - y).abs() / y < 0.01, "one 400-frame step must land where 400 one-frame steps do: {x} against {y}");
    }

    #[test]
    fn the_first_refresh_snaps_so_an_opening_frame_is_not_the_floor() {
        let mut road = FoodRoad::new();
        road.mode = FoodOverlay::Harvest;
        road.harvest.entry((1, (0, 0))).or_default().add(50_000.0, 0, road.harvest_half_life);
        road.refresh(0);
        assert!((road.harvest_scale() - 50_000.0).abs() < 1.0, "got {}", road.harvest_scale());
    }

    #[test]
    fn another_world_is_not_drawn_with_this_one_s_road() {
        // The lab's rack draws inactive chambers through the same `Renderer`.
        // Painting one of those with the active box's road is the cosmetic
        // half of that bug; the expensive half -- observing another world's
        // animals into this map, against `u16` handles every world reissues
        // from its own first tick -- is prevented by `observe` simply not
        // being called from `draw` at all.
        let mut road = FoodRoad::new();
        road.mode = FoodOverlay::Road;
        let mine = World::new(crate::sim::chunk::Rect::new(0, 0, 63, 63));
        road.observe(&mine);
        assert!(road.describes(&mine), "the world it was built from");

        let mut elsewhere = World::new(crate::sim::chunk::Rect::new(0, 0, 63, 63));
        elsewhere.frame = 900;
        assert!(!road.describes(&elsewhere), "a chamber at another frame is another world");

        let bigger = World::new(crate::sim::chunk::Rect::new(0, 0, 127, 127));
        assert!(!road.describes(&bigger), "a box of another size is another world");

        road.mode = FoodOverlay::Off;
        assert!(!road.describes(&mine), "off draws nothing, its own world included");
    }

    #[test]
    fn a_tile_is_finer_than_a_chunk() {
        // The departure from the round's brief is deliberate and the reason
        // is arithmetic: at a chunk, the shipped 512x320 bed is 8x5 tiles.
        let road = FoodRoad::new();
        assert!(road.tile < crate::sim::chunk::CHUNK_SIZE);
        assert!(512 / road.tile >= 32, "a harvest map must be able to separate one plant from the next");
    }
}
