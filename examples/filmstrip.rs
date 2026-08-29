//! Renders a scene to a **contact sheet**: several frames of one run laid out
//! in a grid, through the same `Renderer` the sandbox itself draws with, with
//! no window and no GPU.
//!
//! This exists because a numeric metric only ever sees the one quantity it was
//! written to see, and choosing that quantity correctly turns out to be the
//! hard part. Repeatedly, a metric written before anyone had looked at the
//! artifact measured the wrong thing and reported "nothing here" on a scene
//! that visibly had something: surface height hid a 9x chunk-seam effect that
//! column volume showed plainly, and occupancy hid torn seam rows that were a
//! fill deficit rather than a hole. Worse, a metric cannot see what it was not
//! asked about, so a fix that cleared one artifact while introducing a larger
//! one passed its own test and had to be reverted from live play
//! (`e816477`). An image has the opposite property: it shows everything at
//! once, including whatever nobody thought to measure.
//!
//! `examples/ascii.rs` already makes this argument ("movement rules are far
//! easier to judge by eye than by assertion") and answers it in the terminal.
//! This answers it in actual pixels, which is what the bug reports are about.
//!
//! # Why a grid rather than a GIF
//!
//! `main.rs`'s capture hook already writes a `sequence.gif`, and it is the
//! right thing for a human watching it. But several of these bugs are only
//! legible *in motion* — a fringe that regenerates every frame reads very
//! differently from one that sits still — and an animation cannot be taken in
//! at a glance or quoted in a report. Laying time out along space solves both:
//! one image, one look, motion visible as difference between neighbouring
//! tiles.
//!
//! # Why crop and zoom are not optional extras
//!
//! At 1:1 a one-pixel-tall dark line across a 512-wide world — the size a
//! filmstrip scene builds at (`WIDTH`x`HEIGHT`, viewport-sized on purpose;
//! see `viewshot.rs`'s own doc for why the *shipped*, now much larger, world
//! is a separate question this tool does not answer) — is genuinely easy to
//! miss — that is exactly how the horizontal chunk-seam tearing went
//! unnoticed until it was pointed out. `crop` and `zoom` are the difference
//! between "I looked at it" and "I saw it".
//!
//! ```text
//! cargo run --release --example filmstrip -- scene=pour start=100 every=60 count=6
//! cargo run --release --example filmstrip -- scene=pour crop=160,180,120,80 zoom=4
//! ```

use std::collections::HashSet;

use pixel_physics::render::{BubbleMode, FieldOverlay, GasMode, GrainMode, OrganismOverlay, Renderer, SkyLight, TreeDepth};
mod common;

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::pheromone::{Channel, DEPOSIT};
use pixel_physics::sim::world::World;
use pixel_physics::sim::rng;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::{explosion, material, parallel, update};

/// Name the live `chain_reach` (`F9`) for the sheet.
///
/// `SPREAD` is `i32::MAX`, and printing that as a number tells a reader
/// nothing at all -- while a sheet that does not name the mode it was run
/// at cannot be compared with the one beside it, which is the whole point
/// of sweeping the setting.
fn chain_reach_name(reach: i32) -> String {
    pixel_physics::sim::structural::CHAIN_MODES
        .iter()
        .find(|m| m.reach == reach)
        .map_or_else(|| reach.to_string(), |m| m.name.to_string())
}

const WIDTH: i32 = 512;
const HEIGHT: i32 = 320;
const FLOOR_THICKNESS: i32 = 8;

/// `scene=hop`'s shelf row, and the lanes on it.
///
/// The shelf sits high enough that the drop below it is longer than any of
/// these bodies' own hops -- which is the whole point of the scene, since
/// `ant_wide` and `ant_block` launch identically and are separated only by
/// how they come down.
const HOP_SHELF_Y: i32 = 110;
const HOP_SHELF_WIDTH: i32 = 34;

/// Where each lane starts, and which body stands on it.
///
/// **Spaced by expected range, not evenly**, because they do not travel the
/// same distance: over the ~194-cell drop this scene gives them, the 2-cell
/// chain carries about 160 cells downrange and the 3x3 block about 27. Even
/// spacing would have put the lightest body through the next lane's shelf,
/// which reads as a collision bug rather than as a glide.
const HOP_LANES: [(i32, &str); 4] = [(6, "ant"), (190, "ant_long"), (310, "ant_wide"), (410, "ant_block")];

/// How hard `scene=hop` wires the impulse verb on.
///
/// `Bias` is 1.0 and the gate is `squash(w) > 0` followed by a roll against
/// that value, so 2.0 squashes to 0.67 -- a creature that hops about two
/// times in three whenever its move roll succeeds. Not 1.0-and-always,
/// because a body that never walks never reaches the edge of its shelf.
const HOP_IMPULSE_WEIGHT: f32 = 2.0;

/// Ground level for the plant scenes, in world rows from the top.
///
/// Chosen against `field.rs`'s measured light profile, not by eye: at the
/// current `LIGHT_DECAY` the reading crosses `Germinate`'s `0.1` threshold
/// roughly **75** cells below open sky, and `ambient_light_above` samples a
/// further `FIELD_SCALE` (8) rows *above* the cell it is asked about. `40`
/// leaves most of that band as headroom, so a seed here germinates on
/// light margin rather than on the edge of it -- and so a canopy growing
/// upward from here has somewhere lit to grow into. Do not deepen this
/// without re-reading `LIGHT_DECAY`'s own doc; a scene where nothing
/// germinates looks identical to a scene where growth is broken.
const TREE_GROUND_Y: i32 = 40;

/// `scene=coldsnap`'s seed, and the run of frames it is aimed at.
///
/// **Found by search rather than picked**, the same way `weather.rs`'s own
/// tests find a frame that has weather in it (`first_frame_with`):
/// `weather::at` is a pure function of `(seed, frame)`, most frames of most
/// seeds are clear, and a scene that started at frame 0 of seed 1 would be
/// a pond under a blue sky. Swept over 4,000 seeds for a front meeting
/// three conditions at once, each of which one earlier candidate failed:
///
/// - **Intensity above ~0.75 for several hundred frames.** Not for the
///   freezing -- `WATER_CHILL_BASE` puts water below zero at any intensity
///   -- but for the *snow*, which is what makes the sheet legible as a
///   frozen surface rather than a recoloured pool. `SNOW_CHILL` is a
///   magnitude below ambient, so at intensity 0.69 a landing flake is
///   written at 2°C, which is snow's own melting point: it melts on the
///   frame it lands and a 1,800-frame storm leaves no drift at all.
///   (Measured on the first candidate, seed 38.) Above ~0.75 it settles.
/// - **The snow *ends*, and abruptly.** The thaw is half the artifact.
///   Precipitation fading out is a slow ramp -- intensity goes to zero as
///   the wet channel crosses its threshold -- so a front that ends into
///   clear weather takes ~2,000 frames to do it, and the sheet is gone
///   before the front is. A front that ends because the **chill** channel
///   drops instead switches to rain at full intensity: the cold simply
///   stops, in one frame, which is exactly the event "the front passes"
///   is meant to be. Rain then falls on the thawing sheet, which is both
///   correct and better looking than a clear sky.
/// - **Daylight.** `field::sun_elevation` is another pure function of the
///   frame, night is half of a 3,600-frame day, and a 1,200-frame run
///   lands wherever it lands: the first candidate spent its first six
///   tiles in the dark, where a pale blue sheet on dark blue water is
///   nearly invisible. Seed 2900's front ends at frame 25,010, which is
///   phase 3,410 -- shortly before noon -- so the whole run is lit and the
///   thaw happens at the brightest point of the day.
///
/// Starting 700 frames before the switch leaves time for freeze-over (~360
/// frames at this intensity: the storm chills ~0.7 pond columns a frame and
/// has to revisit them) and for snow to drift on the ice it made.
///
/// `seed=` is deliberately *not* wired to this: the frame window is chosen
/// for this seed and means nothing on another one.
const COLDSNAP_SEED: u64 = 2900;
/// The frame `scene=coldsnap` starts at, and it is the start of a **long**
/// cold spell rather than of a snowfall.
///
/// Re-found when cold stopped needing snow to fall
/// (`weather::DRY_FROST_CHILL`). The old 24,310 sat in a spell whose usable
/// cold ran about 700 frames -- twelve seconds -- which was fine when a
/// pond iced over in a third of one and useless once freezing took a
/// minute: every clip ended mid-freeze. Seed 2900's spells run to 21,570
/// frames; `weather.rs`'s `probe_cold_spells` prints them with their start
/// frames, so this is derived rather than hunted for. This one is 20,100
/// frames of unbroken cold, which holds the whole arc -- freeze-over,
/// a long hold, and a thaw -- inside one run.
const COLDSNAP_START: u64 = 124_680;
/// The frame seed 2900's cold spell ends. Not read by the simulation
/// (`weather::at` is the authority) but a run whose tiles all sit before
/// this is a run that never showed a thaw, and that is worth being able to
/// see in the log.
const COLDSNAP_SNOW_ENDS: u64 = 144_780;
const COLDSNAP_SHORE_Y: i32 = 240;
const COLDSNAP_POND_DEPTH: i32 = 20;

/// `scene=stormcycle`: seed 31337's front runs from frame 19,080 to 24,060 —
/// a rain storm peaking at intensity 0.83, with epochs of clear sky either
/// side of it. Starting at 17,400 buys about 1,700 dry frames before it
/// arrives, and a run of 8,400 frames comes out the far side with another
/// 1,700 to spare.
///
/// Picked by sweeping `weather::at`, which is a pure function of
/// `(seed, frame)` and so costs nothing to search. Rain and not snow
/// deliberately: snow puts the whole freeze/melt loop between the sky and
/// the census, and what this scene is for is the *outer* cycle on its own.
///
/// Run it as:
///
/// ```text
/// cargo run --release --example filmstrip -- \
///     scene=stormcycle start=0 every=700 count=12 zoom=2 crop=0,168,512,72
/// ```
///
/// **`every=700` is a measured choice, not a default.** At 350 the numbers
/// still show the storm but half the sheet is spent on it; at 1400 the dip
/// during the front is one tile wide and reads as noise. At 700 the twelve
/// census lines run 2500.0 up to 2510.1 across the dry lead, *down* to
/// 2505.7 through the front, and back up to 2521.3 after it — the shape the
/// scene exists to show, in numbers, next to the picture.
///
/// The crop is the shore band: at zoom 1 over a 320-row world the pond is a
/// six-pixel line and the rain is what the eye finds instead. Half the tiles
/// land at night, which is the day/night cycle doing its job over an 8,400
/// frame run rather than a fault in the sheet.
///
/// `seed=` is deliberately not wired to this: the frame window is chosen for
/// this seed and means nothing on another one.
const STORMCYCLE_SEED: u64 = 31337;
const STORMCYCLE_START: u64 = 17_400;
const STORMCYCLE_STORM: (u64, u64) = (19_080, 24_060);
const STORMCYCLE_SHORE_Y: i32 = 200;
const STORMCYCLE_DEPTH: i32 = 6;

/// `scene=watercycle`: **the outer water loop's closing demonstration** —
/// `stormcycle`'s pond and shore, cell for cell, over a window four times as
/// long and starting two clear days earlier.
///
/// A sibling rather than a longer `stormcycle`, per this file's own rule at
/// `build`: that scene's numbers are quoted in `weather.rs` and in the
/// milestone write-up, and a scene that changed underneath a recorded
/// measurement is worse than no scene. The two share a seed and a geometry,
/// so anything true of one is true of the other in its own window.
///
/// # What the window is chosen to make legible
///
/// Three separate things, and none of them shows up in a run that spans less
/// than a day:
///
/// * **The sky fills faster by day than by night.** `evaporation::warmth`
///   made the drying rate diurnal; the bank is where that becomes visible
///   over a whole world rather than over one basin. Sampling every 900
///   frames puts each tile on a quarter of `field::DAY_NIGHT_PERIOD_FRAMES`,
///   so consecutive tiles are noon, sunset, midnight, sunrise — and the
///   credit between them rises and falls with them.
/// * **A storm empties it again.** Seed 31337's front runs 19,022 to 24,060
///   (`weather`'s `probe_find_a_dry_lead_before_a_storm`; `STORMCYCLE_STORM`
///   quotes 19,080, which is that probe's coarse 60-frame grid rather than
///   the true first frame). Starting at 10,800 leaves 2.28 clear days in
///   front of it and the run carries on 4,700 frames past its end.
/// * **Nothing is created or destroyed while either happens.** The census
///   prints `water_equivalents + bank` on every tile, and it must be the
///   same number on all twenty of them.
///
/// 10,800 rather than 11,822 (exactly two days before the front) so that the
/// start lands on a multiple of `DAY_NIGHT_PERIOD_FRAMES` — the sun is at
/// noon on tile 0 and every fourth tile after it, which is what makes the
/// bank column readable as a day at all instead of as a drift.
///
/// Run it as:
///
/// ```text
/// cargo run --release --example filmstrip -- \
///     scene=watercycle start=0 every=900 count=20 zoom=2 crop=0,168,512,72
/// ```
///
/// # What the picture shows, and what only the census can
///
/// **The sheet is the control, not the result.** It shows the sky cycling
/// blue -> pink -> black -> orange four times over, and it shows real rain
/// streaking across the middle six tiles — which is what says the day and
/// the front are actually running, and that the numbers below are not a
/// readout of a clock nobody wired up. What it cannot show is the pond
/// shrinking: 1,440.0 cell-equivalents down to 1,394.1 over five days is
/// 3%, which across a 240-cell pond is a fifth of one row. Reading the sheet
/// for that and concluding nothing happened is the mistake this paragraph
/// exists to head off. An image says what and where; the census says how
/// much.
///
/// Measured, on the run above (`sky:` deltas, cell-equivalents banked per
/// quarter-day, in tile order):
///
/// | quarter ending at | on the two clear days | through the front | after it |
/// |---|---|---|---|
/// | noon | +4.36, +4.36 | -2.53 | +4.41 |
/// | sunset | +4.54, +4.35, +5.13 | +3.53 | +4.52 |
/// | midnight | +1.16, +1.16 | +0.65, +2.43 | +1.19 |
/// | sunrise | +1.19, +1.19 | -2.45 | +2.83, +3.87 |
///
/// The warm quarters credit about 4.4 and the cool ones about 1.18 — **3.7
/// to one**, and wider than the 2.5 the `evaporation` basin guards measure,
/// because this pond sits under open sky while that basin sits under a lid
/// that attenuates the sky's forcing to 4.59 of its 6 degrees. Through the
/// front the sign flips outright: the sky spends faster than a whole pond's
/// shoreline can refill it.
///
/// Two readings in that table that look wrong and are not. The night
/// quarters *inside* the front (+0.65, +2.43) bracket the wet-channel peak
/// rather than sitting on it — the front is easing by then, and the second
/// is a night crediting more than a clear night does because the rain left
/// puddles all over the bare rock and unsheltered puddles dry fast. Same
/// reason for the two elevated sunrises after it (+2.83, +3.87), which is
/// the shore giving the storm's water back and is the return half of the
/// loop rather than an anomaly.
///
/// And `water + sky` reads 3940.0 on every one of the twenty tiles: the
/// coupling changed the rate and never the ledger.
///
/// `seed=` is deliberately not wired to this: the frame window is chosen for
/// this seed and means nothing on another one.
const WATERCYCLE_START: u64 = 10_800;
/// The true first and last frames of the front this scene runs through, as
/// opposed to `STORMCYCLE_STORM`'s grid-rounded pair. Printed in the header
/// so the census lines can be read against it.
const WATERCYCLE_STORM: (u64, u64) = (19_022, 24_060);

/// Water with a varied `shade`, the way the brush lays it down
/// (`World::paint_capsule` rolls a random shade per cell). The scenes below
/// would otherwise use `Cell::new(WATER, 0)` and give every cell an
/// identical shade — which silently flattens `GrainMode::Cell` to no grain
/// at all, since that mode keys entirely off this byte. Worth knowing as a
/// real caveat of that mode and not just a harness detail: any water created
/// without a varied shade renders flat under it.
fn water_at(x: i32, y: i32) -> Cell {
    Cell::new(material::WATER, (rng::jitter(x, y) * 255.0) as u8)
}

fn stone_floor(w: &mut World) {
    for x in 0..WIDTH {
        // Terrain, so it declares itself attached the way `build_terrain`
        // does -- otherwise it is foreground material that has to hold
        // itself up, and a floor that is only anchored at the world edges
        // erodes inward from every free face.
        for y in (HEIGHT - FLOOR_THICKNESS)..HEIGHT {
            w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
}

/// The scenes the current bug list is about. Adding one is three lines, and
/// is much preferred to editing an existing one — a scene that quietly
/// changed underneath a recorded measurement is worse than no scene.
/// Upper bounds of the size classes `fastest by size` reports, in cells
/// across. Coarse on purpose: terminal velocity goes as the square root of
/// size, so the interesting contrast is between a fragment and a block, and
/// finer buckets would mostly report how many bodies happened to land in
/// each.
const SIZE_BUCKETS: [usize; 5] = [3, 6, 9, 14, usize::MAX];

/// How much water has to stand over a body's head before its speed counts as
/// evidence about the terminal velocity, in cells.
///
/// **Not one, which is what `rigid::surrounding_liquid` uses to decide whether
/// to apply drag at all.** The two questions are different and the difference
/// is a whole frame: the harness samples speeds *after* the body has moved,
/// so on the frame a body enters the water it was airborne when the clamp
/// ran, kept its full entry speed, and then moved two or three cells down --
/// and a one-cell probe then calls it submerged and records an unclamped
/// speed as though it were the cap.
///
/// That is the whole reason a working size term measured as no size term at
/// all. Six cells is more than a body can travel in a frame at any speed the
/// clamp allows, so anything this deep has been clamped at least once.
const SUBMERGED_MARGIN: i32 = 6;

/// Whether a body is *well* inside a liquid, by `rigid::surrounding_liquid`'s
/// probe -- above the top of its bounding box, at the middle column, which is
/// outside the footprint and so never reads the body's own reservation --
/// carried down `SUBMERGED_MARGIN` cells.
///
/// **Without this the sink readout measures the sky.** A terminal velocity in
/// water has no authority over a body falling through air, so a peak taken
/// over a body's whole life is its *entry* speed and says nothing about the
/// cap. Measured: sweeping the drag coefficient from 1.0 to 3.0 moved the
/// unfiltered peak from 3.37 to 2.58 and left the size ordering
/// non-monotonic, which reads as "the size term does not work" and is really
/// "this number is about the twenty rows of air above the pond".
/// `CLAUDE.md` records the same trap catching the previous sink test.
fn submerged(world: &World, b: &pixel_physics::sim::rigid::ChunkBody) -> bool {
    let (mut x0, mut x1, mut y0) = (i32::MAX, i32::MIN, i32::MAX);
    for c in &b.cells {
        let (x, y) = b.cell_position(c);
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
    }
    if x1 < x0 {
        return false;
    }
    let px = (x0 + x1) / 2;
    (1..=SUBMERGED_MARGIN).all(|d| {
        world.in_bounds(px, y0 - d)
            && world.materials.kind(world.get(px, y0 - d).material) == MaterialKind::Liquid
    })
}

/// A body's longer side, in cells -- `rigid::body_extent`, which is private.
fn body_extent_of(b: &pixel_physics::sim::rigid::ChunkBody) -> i32 {
    let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for c in &b.cells {
        x0 = x0.min(c.dx);
        x1 = x1.max(c.dx);
        y0 = y0.min(c.dy);
        y1 = y1.max(c.dy);
    }
    if x1 < x0 {
        return 1;
    }
    (x1 - x0).max(y1 - y0) + 1
}

/// Whether a contact sheet should draw every tile under the same light.
///
/// Every cell is tinted and dimmed by the day/night cycle before it is drawn
/// (`Renderer::pinned_light`, which is what this switches on), and
/// `DAY_NIGHT_PERIOD_FRAMES` is 3600. So a sheet of a process that takes days
/// walks its tiles around the light cycle, and consecutive tiles differ in
/// *brightness* for reasons that have nothing to do with whatever the sheet
/// was cut to show.
///
/// **Reported from play against the ice arc**: *"you can see a different ice
/// morphology between the first and second half"*. Reproduced, and part of it
/// was the sunset. Eight tiles every 2,700 frames is every 0.75 of a day, so
/// they landed at noon, sunset, midnight and sunrise in rotation -- the census
/// prints the phase per tile and said exactly that, unread.
/// `scripts/acceptance.sh`'s `coldsheet` had it worse at `every=1800`, exactly
/// half a day, alternating noon and midnight forever.
///
/// **Auto rather than always.** A sheet that spans a tenth of a day has no
/// aliasing to fix -- `scene=fall every=60 count=6` is six frames of a rock
/// falling -- and pinning it would only throw away a true picture of the sky.
/// It engages when the span reaches a day, which is when the aliasing starts.
/// `phase=noon` and `phase=off` force it either way.
///
/// The alternative, tried first and rejected: snap the *sample frames* to a
/// whole number of days. It works, but whole-day quantisation is the only
/// interval that shares a phase, so it doubled `coldsheet`'s span and runtime
/// and changed which frames were being judged. A render pin changes neither.
fn pin_sheet_light(args: &Args) -> bool {
    // The span the sheet covers, not the tile count: one extra tile at a
    // short interval is not the situation this exists for.
    let span = (args.every * args.count.saturating_sub(1)) as u64;
    let pin = args.phase.unwrap_or(span >= pixel_physics::sim::field::DAY_NIGHT_PERIOD_FRAMES);
    if pin {
        println!("phase: every tile drawn at noon ({span} frames spans a day or more; phase=off to see the real sky)");
    }
    pin
}

/// The grown stand the three gnome scenes share, with the two axes that
/// vary it exposed.
///
/// One builder rather than three copies, for the reason `common::mod`
/// already records: two scenes that drift apart is the failure that module
/// exists to end. See `"wood"` for why a gnome case must be sweepable at
/// all.
fn gnome_stand(args: &Args) -> World {
    let base = common::PlantScene::default();
    let plants = if args.plants > 0 { args.plants } else { base.trees };
    common::PlantScene { trees: plants, start_frame: args.frame0, ..base }.build()
}

/// The world-level settings `build` applies from the arguments:
/// `confine=`, `arch=`, `share=`, `chain_reach=`, `joints=`, `bands=`,
/// `jwidth=`.
///
/// **Applied twice, and the second time is a bug fix.** They are set before
/// the scene is built because several scenes cut into the world during
/// construction and the rule has to be in force for that cut as much as for
/// the run. But five scenes -- `grove`, `wood`, `climb`, `shake` and `fell`
/// -- build their world through `common::PlantScene` and **`return` it**,
/// discarding the `w` these were written onto. Every one of those knobs was
/// therefore silently inert on those scenes.
///
/// Caught by `CLAUDE.md`'s own tell rather than by reading the code:
/// `scene=fell` reported byte-identical output at `chain_reach=spread`,
/// `local` and `tight` -- 2,360 cells severed in all three -- and identical
/// output across settings means the knob was never connected. Re-applying
/// on the world that is actually returned is idempotent for the scenes that
/// already worked and is the whole fix for the five that did not.
fn apply_world_settings(w: &mut World, args: &Args) {
    w.crush_confined = args.confine;
    w.arch_relief = args.arch;
    w.section_share = args.share;
    if let Some(reach) = args.chain_reach {
        w.chain_reach = reach;
    }
    if let Some(spacing) = args.joint_spacing {
        if let Some(stone) = w.materials.id_of("stone") {
            w.materials.get_mut(stone).joint_spacing = spacing.max(0.0);
        }
    }
    if let Some(contrast) = args.joint_bands {
        if let Some(stone) = w.materials.id_of("stone") {
            w.materials.get_mut(stone).joint_band_contrast = contrast.clamp(0.0, 0.9);
        }
    }
}

/// Build the scene, then re-apply the world settings to whatever world came
/// out -- see `apply_world_settings` for why the second application is not
/// redundant.
fn build(args: &Args) -> World {
    let mut world = build_scene(args);
    apply_world_settings(&mut world, args);
    world
}

fn build_scene(args: &Args) -> World {
    let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
    apply_world_settings(&mut w, args);
    let floor_y = HEIGHT - FLOOR_THICKNESS;
    match args.scene.as_str() {
        // A large body released against the left wall, spreading right across
        // seven vertical chunk seams. The terracing/banding reproduction.
        "pour" => {
            stone_floor(&mut w);
            for x in 0..200 {
                for y in 30..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        // The water cycle's core loop (the M14 follow-up): a walled stone
        // chamber holding a pool with a burning oil slick floating on it,
        // under a stone ceiling low enough that the steam has somewhere to
        // condense and drip back into the pool. Deliberately needs no new
        // verb: oil floats on water and burns, which is a heat source the
        // engine already ships. Read the phase-change counters printed
        // under each tile alongside the image -- a plume the boiling
        // actually produced and a puff of painted smoke are the same grey
        // pixels at this zoom.
        // A pan of water over a hot hearth: an open basin whose *floor* is
        // a row of 700C stone, with nothing burning anywhere.
        //
        // **Built because `scene=boil` cannot demonstrate the thing this
        // scene is for**, and finding that out cost a round of looking at
        // sheets that showed nothing. `boil` heats its pool from a burning
        // oil slick lying *on the surface*, so the only water above
        // `render::BUBBLE_MIN_TEMPERATURE` is the top row or two -- 361
        // cells at frame 90 spread across a 150-wide pool, drawn under the
        // fire tint and behind a steam cloud. Everything about that is
        // correct and none of it can show a bubble rising, because there
        // is no depth of hot water for one to rise *through*.
        //
        // Heat from below is the configuration boiling actually has, it is
        // the one `fire.rs`'s `a_finite_heat_inventory_stops_boiling_and_
        // the_world_sleeps` already uses as its thermodynamic control, and
        // it terminates: a fixed inventory of heat, no source, so the pan
        // comes off the boil on its own rather than running forever.
        //
        // Deliberately **open at the top** where `boil` is sealed: the
        // steam has somewhere to go, so it does not build a cloud over the
        // very surface the effect has to be judged against.
        "simmer" => {
            stone_floor(&mut w);
            // **Shallow, and the hearth is three rows rather than one.**
            // First cut was a 61-deep pan over a single 700C row and it was
            // stone cold by frame 120 with 4 cells left over threshold --
            // a scene that contradicts what it is for looks exactly like a
            // mechanism that does not work (`CLAUDE.md`). A pan is a pan:
            // shallow enough that the heat reaches the surface, with enough
            // stone under it to hold a boil for a few hundred frames.
            let (left, right, hearth_rows, depth) = (200, 312, 3, 14);
            let hearth_y = floor_y - hearth_rows;
            let rim_y = hearth_y - depth;
            for y in rim_y..floor_y {
                w.set(left, y, Cell::new(material::STONE, 0).with_attached(true));
                w.set(right, y, Cell::new(material::STONE, 0).with_attached(true));
            }
            for x in left..=right {
                for y in hearth_y..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true).with_temperature(900));
                }
            }
            for x in (left + 1)..right {
                for y in rim_y..hearth_y {
                    w.set(x, y, water_at(x, y));
                }
            }
            println!(
                "simmer: a {}x{depth} pan over {hearth_rows} rows of 900C stone, open to the air",
                right - left - 1
            );
        }
        "boil" => {
            stone_floor(&mut w);
            let (left, right, ceiling_y, rim_y) = (180, 330, 196, 260);
            // Walls and ceiling are terrain (attached), or the chamber
            // would erode structurally and the run would measure that
            // instead of the phase loop.
            for y in ceiling_y..floor_y {
                w.set(left, y, Cell::new(material::STONE, 0).with_attached(true));
                w.set(right, y, Cell::new(material::STONE, 0).with_attached(true));
            }
            for x in left..=right {
                for y in ceiling_y..(ceiling_y + 6) {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for x in (left + 1)..right {
                for y in rim_y..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
            // A burning slick floating on the surface (oil density 0.8 <
            // water 1.0), pre-ignited -- the same state a player makes by
            // pouring oil on a pool and lighting it.
            let burn = w.materials.get(material::OIL).burn_duration;
            for x in (left + 20)..(right - 20) {
                let mut slick = Cell::new(material::OIL, (rng::jitter(x, rim_y - 1) * 255.0) as u8);
                slick.ignite(burn);
                w.set(x, rim_y - 1, slick);
            }
        }
        // The heat verb (the M14 follow-up's second half): lava poured down
        // a stone ramp into a walled pond, with a wood stand at the
        // shoreline. Three separate things to read off the sheet, and only
        // the first is what the counters answer:
        //
        // 1. **A stone crust at the interface**, stippling in over a few
        //    frames rather than drawing as a line -- that is
        //    `lava.ron`'s reaction, and `reacted` in the phase-change
        //    line below the tile is its "did it fire at all" count. Grey
        //    cells appearing where lava met water and grey cells that were
        //    already ramp are the same grey at this zoom, which is exactly
        //    the case `CLAUDE.md` says a picture cannot answer.
        // 2. **A steam plume** off the pond -- the quench's other product,
        //    born at the lava's own temperature (`fire::try_react` takes
        //    the hotter side) so it rises instead of flashing back.
        // 3. **The wood catching without a flame anywhere in the scene**,
        //    purely from conducted heat crossing `wood.ron`'s
        //    `ignition_temperature`. Nothing here is pre-ignited, unlike
        //    `scene=boil` which has to light an oil slick by hand: that is
        //    the whole point of the verb. It has no counter of its own on
        //    the tile line -- the evidence is the ash it leaves (70 cells
        //    by frame 1500, censused with a throwaway probe) and
        //    `fire.rs`'s `lava_ignites_adjacent_wood`, which asserts it on
        //    a sealed pocket where the flow cannot drain away from the
        //    wood.
        //
        // The ramp is a solid wedge rather than a slab on stilts so it
        // cannot erode structurally -- otherwise the run would be measuring
        // that instead. Everything stone here is `attached` terrain for the
        // same reason.
        "lavapour" => {
            stone_floor(&mut w);
            let lava = w.materials.id_of("lava").expect("lava is a compiled-in material");
            let wood = w.materials.id_of("wood").expect("wood is a compiled-in material");
            let (pond_left, pond_right, pond_top) = (250, 430, 250);
            // Ramp surface height at column `x`: a straight fall from the
            // head of the ramp down to the pond's near rim.
            let (ramp_x0, ramp_y0, ramp_y1) = (170, 130, pond_top - 6);
            let ramp_top = |x: i32| -> i32 {
                let t = (x - ramp_x0) as f32 / (pond_left - ramp_x0) as f32;
                ramp_y0 + ((ramp_y1 - ramp_y0) as f32 * t).round() as i32
            };
            for x in ramp_x0..=pond_left {
                for y in ramp_top(x)..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // The pond's far wall. The near one is the ramp itself, which
            // is what puts the shoreline in the lava's path.
            for y in pond_top..floor_y {
                w.set(pond_right, y, Cell::new(material::STONE, 0).with_attached(true));
            }
            for x in (pond_left + 1)..pond_right {
                for y in (pond_top + 2)..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
            // A low wood stand on the ramp just above the waterline. Low
            // deliberately: wood is `Plant` and never displaced, so a tall
            // stack would dam the ramp and the lava would never reach the
            // pond inside the run. Five cells lets the flow pool against
            // it, heat it, and then overtop it.
            for x in (pond_left - 26)..(pond_left - 12) {
                let surface = ramp_top(x);
                for y in (surface - 5)..surface {
                    w.set(x, y, Cell::new(wood, (rng::jitter(x, y) * 255.0) as u8));
                }
            }
            // A back wall at the head of the ramp. Not decoration: without
            // it the first frame's slump spreads the reservoir *left* off
            // the end of the ramp and a third of the pour lands on the
            // world floor instead of running down, which reads as a leak
            // and quietly shrinks everything downstream of it.
            for y in (ramp_y0 - 24)..=ramp_y0 {
                w.set(ramp_x0 + 1, y, Cell::new(material::STONE, 0).with_attached(true));
            }
            // The reservoir, at the head of the ramp: a finite pour, not an
            // emitter. ~660 cells -- every molten cell is off-ambient and
            // keeps its chunk awake until it crusts, so the size is still a
            // cost, just a finite one now (see `lava.ron`'s header for the
            // cooling model that made it finite).
            //
            // Do not read `reacted` as "cells of lava consumed": a flow
            // splits into partial-fill cells on the way down and each of
            // those reacts in its own right. It is a "did it fire, and is
            // it still firing" count, nothing more. The reading that
            // matters is that it climbs steeply while the front is in the
            // water and then goes *flat* alongside `froze` -- flat together
            // means the pour is finished, crusted where it stranded and
            // quenched where it reached. The standing census printed under
            // each tile is the cross-check: molten should read 0 from
            // there on.
            for x in (ramp_x0 + 2)..(ramp_x0 + 32) {
                for y in (ramp_y0 - 22)..ramp_y0 {
                    w.set(x, y, Cell::new(lava, (rng::jitter(x, y) * 255.0) as u8));
                }
            }
        }
        // **Lava dropped into water**, which is the owner's own report
        // verbatim ("if I drop lava into water, it boils, turns to steam,
        // rises about 5 ft in the air and then drops back into rain").
        //
        // `lavapour` cannot answer it. Its ramp delivers the lava at the
        // *shoreline*, so what happens next is dominated by a delta
        // building at the water's edge and by a crust bridging the pond --
        // real behaviour, and a confound when the question is what the
        // plume over open water does. Here the lava arrives in the middle
        // of a wide pond with nothing but water under it, which is what a
        // player painting lava over a lake actually produces.
        //
        // The blob is released a few rows above the surface rather than
        // placed in it, so the first contact is a real fall onto real
        // water. `span=` sets the pond width (default 200).
        "lavadrop" => {
            stone_floor(&mut w);
            let lava = w.materials.id_of("lava").expect("lava is a compiled-in material");
            let half = args.span / 2;
            let (left, right) = (256 - half, 256 + half);
            let pond_top = 250;
            for y in pond_top..floor_y {
                w.set(left, y, Cell::new(material::STONE, 0).with_attached(true));
                w.set(right, y, Cell::new(material::STONE, 0).with_attached(true));
            }
            for x in (left + 1)..right {
                for y in (pond_top + 2)..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
            let mut blob = 0;
            for x in 236..276 {
                for y in 226..246 {
                    w.set(x, y, Cell::new(lava, (rng::jitter(x, y) * 255.0) as u8));
                    blob += 1;
                }
            }
            println!(
                "lavadrop: {blob} cells of lava released 6 rows over the middle of a {}-wide pond",
                right - left - 1
            );
        }
        // A lava *lake*, open to the sky, for the owner's report that a
        // crust "freezes in place" instead of foundering. The pour scene
        // above cannot answer it: everything that crusts there is either a
        // film stranded on the ramp (correctly stuck to the ramp) or stone
        // minted by the quench reaction inside water. This scene isolates
        // the third case -- lava cooling into stone at the *middle top of a
        // lake*, with the nearest anchor 90 columns away, which is far past
        // stone.ron's `max_unsupported_span` of 16 in either direction.
        //
        // Deliberately wide and shallow-walled: the whole question is what
        // happens to a plate that has no path to an anchor, so a basin
        // narrow enough for the crust to span shore-to-shore would answer a
        // different one. `span=` sets the width (default 200).
        "lavalake" => {
            stone_floor(&mut w);
            let lava = w.materials.id_of("lava").expect("lava is a compiled-in material");
            let half = args.span / 2;
            let (left, right) = (256 - half, 256 + half);
            let (lake_top, lake_bottom) = (200, floor_y);
            for y in lake_top..floor_y {
                w.set(left, y, Cell::new(material::STONE, 0).with_attached(true));
                w.set(right, y, Cell::new(material::STONE, 0).with_attached(true));
            }
            let mut cells = 0;
            for x in (left + 1)..right {
                for y in (lake_top + 4)..lake_bottom {
                    w.set(x, y, Cell::new(lava, (rng::jitter(x, y) * 255.0) as u8));
                    cells += 1;
                }
            }
            println!(
                "lavalake: {cells} cells of lava in a {}x{} basin, open to the sky; nearest anchor {half} columns from mid-lake",
                right - left - 1,
                lake_bottom - lake_top - 4
            );
        }
        // The water cycle's freezing half, and the scene this milestone is
        // judged on by eye: **a pond with real shorelines under a snowstorm
        // that passes.** In one run it should show freeze-over creeping
        // across the surface, snow drifting on the ice it made, the front
        // lifting, the sheet melting, and the pool coming back.
        //
        // Nothing here is scripted. The cold is `weather.rs` acting on the
        // world the way it does in the app; the freeze is `fire.rs`'s
        // downward phase change against water.ron's `cooling_point`; the
        // thaw is ice.ron's melting point sitting below ambient, so the
        // *absence* of the storm is what melts it. The scene's whole job is
        // to put a pond under a front and then get out of the way.
        //
        // **The seed and the frame are chosen, not arbitrary**, the same
        // way `weather.rs`'s own tests go looking for a frame that has
        // weather in it (`first_frame_with`): most frames of most seeds are
        // clear, so a scene that started at frame 0 of seed 1 would be a
        // pond under a blue sky. See `COLDSNAP_SEED`.
        //
        // The shoreline is not decoration. `structural.rs` cannot express
        // buoyancy, so a sheet of ice on water has no support underneath it
        // at all: its only path to an anchor runs sideways along itself to
        // the shore. That makes the shore the load-bearing part of the
        // scene, and the pond width (`pond=`) the knob the whole structural
        // question turns on -- see ice.ron's `max_unsupported_span` note
        // for the sweep this scene was used to run.
        "coldsnap" => {
            stone_floor(&mut w);
            // Terrain, so it is `attached` and anchors -- the shelf is the
            // massif the pond is cut into, not something stacked in front
            // of it, and a shoreline that had to hold itself up would erode
            // and take the answer about the ice with it.
            for x in 0..WIDTH {
                for y in COLDSNAP_SHORE_Y..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            let pond = args.pond.clamp(2, WIDTH - 8);
            let left = (WIDTH - pond) / 2;
            for x in left..(left + pond) {
                // Flush with the shore rather than sunk below it: the
                // surface row is then level with the stone either side of
                // it, so the sheet's end cells sit *beside* an anchor
                // instead of one row under a lip, and snow drifting on the
                // frozen pond piles continuously with the drift on the
                // bank.
                for y in COLDSNAP_SHORE_Y..(COLDSNAP_SHORE_Y + COLDSNAP_POND_DEPTH) {
                    w.set(x, y, water_at(x, y));
                }
            }
            w.seed = COLDSNAP_SEED;
            w.frame = COLDSNAP_START;
            println!(
                "coldsnap: seed {COLDSNAP_SEED}, world frame {COLDSNAP_START} (cold until frame {}), pond {pond} cells wide at x {}..{}",
                COLDSNAP_SNOW_ENDS,
                left,
                left + pond - 1
            );
        }
        // **The outer water cycle, end to end, in one sheet.** A dry spell,
        // a front, and another dry spell, over water that is bare rock all
        // the way down.
        //
        // What is being judged here is not the picture on its own -- rain
        // falling and a puddle shrinking both looked exactly like this
        // before any of it was conserved. It is the picture *next to the
        // census*: the `water + sky` line under each tile must hold still
        // while the `standing` and `bank` halves of it trade places. That
        // pairing is `CLAUDE.md`'s "did it fire at all needs a counter, not
        // a picture" in its exact original form -- a storm that manufactures
        // water and a storm that spends banked water are the same image, and
        // only the number tells them apart.
        //
        // **One shallow pond and a lot of bare rock, and no soil**, and each
        // of the three is load-bearing. The first geometry tried here was a
        // 32-cell puddle beside a 240-cell pond, both 24 rows deep, and it
        // failed to show the thing: the bank climbed monotonically through
        // the front, because a puddle that narrow is entirely unsheltered
        // (`evaporation::shelter` is a fixed-radius stencil) and the credit
        // from it swamped everything the storm spent. Six rows rather than
        // twenty-four cuts the evaporating shoreline to a quarter without
        // touching the pond's *width*, which is the only thing shelter
        // reads -- so the dry-spell credit comes down to the same order as
        // the storm's debit and the two become legible against each other.
        //
        // The bare rock either side is not empty space. It is where the
        // front's own drops puddle, and those puddles drying afterwards is
        // most of the credit that comes back once it has passed.
        //
        // No soil because `update::update_soil_water` takes a landing water
        // cell's fill into wetness and nothing credits it back -- a real,
        // pre-existing one-way sink out of the ledger (see
        // `weather::STORM_RESERVE`), and a scene built on soil would show
        // that leak rather than the cycle.
        //
        // `watercycle` is the same geometry in a longer, earlier window --
        // see `WATERCYCLE_START`. It shares this arm rather than copying it
        // so that the two scenes cannot drift apart cell by cell; only the
        // frame the world starts on differs.
        "stormcycle" | "watercycle" => {
            for x in 0..WIDTH {
                for y in STORMCYCLE_SHORE_Y..HEIGHT {
                    // Attached, so the shore is terrain rather than
                    // something stacked in front of the sky: an unattached
                    // shelf erodes inward from every free face over the
                    // thousands of frames this scene runs for, and the pond
                    // would drain through the hole.
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for (x0, width) in [(136, 240)] {
                for y in STORMCYCLE_SHORE_Y..(STORMCYCLE_SHORE_Y + STORMCYCLE_DEPTH) {
                    for x in x0..(x0 + width) {
                        w.set(x, y, water_at(x, y));
                    }
                }
            }
            w.seed = STORMCYCLE_SEED;
            if args.scene == "watercycle" {
                w.frame = WATERCYCLE_START;
                println!(
                    "watercycle: seed {STORMCYCLE_SEED}, world frame {WATERCYCLE_START}; the front runs {}..{}, so a run of 18000 frames is two clear days, a storm, and a clear day after it. A day is {} frames, so every=900 puts each tile a quarter-day apart. Sky starts holding {:.1} cell-equivalents",
                    WATERCYCLE_STORM.0,
                    WATERCYCLE_STORM.1,
                    pixel_physics::sim::field::DAY_NIGHT_PERIOD_FRAMES,
                    w.atmospheric_bank
                );
            } else {
                w.frame = STORMCYCLE_START;
                println!(
                    "stormcycle: seed {STORMCYCLE_SEED}, world frame {STORMCYCLE_START}; the front runs {}..{}, so a run of 8400 frames is dry, wet, dry. Sky starts holding {:.1} cell-equivalents",
                    STORMCYCLE_STORM.0, STORMCYCLE_STORM.1, w.atmospheric_bank
                );
            }
        }
        // Falling and spreading, rather than resting on the floor already:
        // the state the horizontal seam tearing shows up in.
        "fall" => {
            stone_floor(&mut w);
            for x in 20..250 {
                for y in 20..200 {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        // A column of water dropped onto a short unwalled shelf in open air:
        // the front spreads with *nothing under it* for most of its length,
        // then pours off both ends. Built for open bug #1 (whiskers), which
        // `fall` and `pour` no longer reproduce: this is the geometry that
        // still sheds a residual comb, and the one whose films are actually
        // partial-fill rather than full cells thrown sideways. Shared cell
        // for cell with `examples/film_probe.rs`'s scene of the same name --
        // the numbers and the picture have to come from the same world.
        "shelf" => {
            stone_floor(&mut w);
            for x in 180..332 {
                for y in 200..204 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for x in 236..276 {
                for y in 120..190 {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        // Stage 2 of the creature milestone: two synthetic trails on the
        // pheromone planes, over ordinary terrain, so the overlay ramps can
        // be judged **against a real signal**. An empty plane renders as a
        // flat ramp floor, which is correct and proves nothing -- the
        // failure this scene exists to catch is a trail that is present in
        // the data and unreadable on screen, which is exactly how the
        // canopy-density sheet came to read as blank.
        //
        // Look at it with `channel=pheromone_a` and `channel=pheromone_b`.
        "pheromone" => {
            stone_floor(&mut w);
            for x in 40..470 {
                for y in 200..floor_y {
                    w.set(x, y, Cell::new(material::SAND, 0));
                }
            }
            // Laid repeatedly, as ants would: a single deposit evaporates
            // before it spreads (measured -- see pheromone.rs's DIFFUSE).
            for i in 0..60u64 {
                for x in 60..450 {
                    w.deposit_pheromone(Channel::A, x, 190, DEPOSIT);
                    let y = 150 + (x - 60) / 12;
                    w.deposit_pheromone(Channel::B, x, y, DEPOSIT);
                }
                w.frame = i * 4;
                w.step_pheromones();
            }
        }
        // A scatter of separate grains falling into an open pool: the
        // splash.
        //
        // **`scene=blob` cannot show this and that is geometry, not
        // tuning.** A splash site is where a denser cell displaces near-full
        // liquid *with air directly above it* (`update::try_move`), and
        // under a 68-wide solid blob the cell above the displaced water is
        // always the next sand cell down. The handful of sites a blob does
        // produce are its own trailing edge, thrown into the middle of a
        // blob that fills the frame -- 37 droplets over the whole entry,
        // every one of them invisible behind the sand.
        //
        // Separate grains are the case the mechanic is for and the case a
        // player makes: each one arrives at open water on its own, and the
        // droplets it throws fly against open air. **One cell in twenty,
        // over a hundred rows**, and both numbers were turned down from a
        // first cut at one in five over sixty: at that density the falling
        // grains are a curtain, arrivals overlap, and a droplet is one blue
        // pixel in a snowstorm of yellow ones. The scene has to leave the
        // air *empty* between arrivals or it cannot show what it is for.
        "splash" => {
            stone_floor(&mut w);
            for y in 0..floor_y {
                w.set(120, y, Cell::new(material::STONE, 0).with_attached(true));
                w.set(392, y, Cell::new(material::STONE, 0).with_attached(true));
            }
            for x in 121..392 {
                for y in 200..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
            let mut grains = 0;
            for x in 160..355 {
                for y in 20..120 {
                    // A hash rather than an rng draw, so the scatter is a
                    // pure function of position and two runs of this scene
                    // are the same scene (`PLAN.md`: determinism).
                    if rng::jitter(x, y) < 0.05 {
                        w.set(x, y, Cell::new(material::SAND, (rng::jitter(y, x) * 255.0) as u8));
                        grains += 1;
                    }
                }
            }
            println!("splash: {grains} loose grains over an open pool 271 wide");
        }
        // **Rock into water, which nothing here dropped before.** The
        // splash scene above is loose grains, deliberately -- a splash
        // needs air above the water it displaces, and a solid slab never
        // leaves any. That makes it the wrong instrument for the owner's
        // report ("I don't see any splash... clumps of sand or rocks would
        // be better"), because the answer for a *clump* is the CA rule and
        // the answer for a *boulder* is `rigid::report_entry_splash`, and
        // only the second is a thing the engine can do at all.
        //
        // A slab of unattached stone held over an open pool by nothing.
        // Unattached, so the load model takes it on the first check; wide
        // and thick enough to clear `MIN_FRACTURE_CELLS` and produce real
        // bodies rather than grit; and dropped from `fall=` rows up
        // (default 90) so it arrives well over `SPLASH_MIN_ENTRY_SPEED`.
        "rockdrop" => {
            stone_floor(&mut w);
            for y in 0..floor_y {
                w.set(120, y, Cell::new(material::STONE, 0).with_attached(true));
                w.set(392, y, Cell::new(material::STONE, 0).with_attached(true));
            }
            for x in 121..392 {
                for y in 200..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
            let top = 200 - args.fall;
            let mut rock = 0;
            for x in 226..286 {
                for y in top..(top + 10) {
                    w.set(x, y, Cell::new(material::STONE, (rng::jitter(x, y) * 255.0) as u8));
                    rock += 1;
                }
            }
            // **The dark box this scene used to draw under the slab is
            // gone, and this note is kept because it was wrong in an
            // instructive way.** It said the box was "the scene, not a bug":
            // the slab is `Solid`, the surface freeze is once-only and per
            // column, so its sixty columns read as underground from row
            // `top` down, and nothing could be done about it from here. The
            // first two clauses were right and the conclusion was not --
            // the freeze being once-only was never the problem, asking it
            // *per column* was, and the answer is stored per cell now
            // (`Reports/dark-bands-diagnosis.md`). A dark band here again
            // is a regression, not the scene.
            //
            // Frame 0 is still the old picture, because the freeze happens
            // on the first `begin_step` and tile 0 draws before it: render
            // from `start=1` when the question is about the background.
            //
            // Nothing disturbs this scene, so the first check has to be
            // asked for -- the same way `capped` does it. Without this the
            // slab hangs there and the harness reports zero of everything,
            // which reads exactly like the splash being broken.
            //
            // And since `TIGHT` became the default `chain_reach`, asking
            // for the check is no longer enough: the failure also has to
            // be *licensed* by something disturbed nearby, or it is found
            // and declined. Measured when that landed -- 600 loose cells
            // still above row 195 and zero bodies in flight, i.e. exactly
            // the "reads like the splash being broken" outcome this
            // comment already warned about, arriving through a second
            // door.
            w.schedule_structural_check_around(256, top + 5);
            // Extent 30, which is the slab's own half-width, not 0.
            //
            // This is exactly what `Disturbance::extent` is for and it was
            // learned the hard way here first: a centre-only record with no
            // extent licenses `chain_reach` either side -- 32 cells at the
            // default -- and this slab is 60 wide, so its outer fourteen
            // columns went unlicensed and **231 cells of it hung in the air
            // after the rest had gone**. The first fix recorded per column
            // and let the coalescing collapse that back down, which worked
            // and was the wrong shape: the wound is one 60-wide slab, so
            // the honest statement is one record that says how wide it is.
            w.record_disturbance(256, top + 5, 30);
            println!("rockdrop: a {rock}-cell slab of unattached stone {} rows over an open pool 271 wide", args.fall);
        }
        // A dense blob dropped into a walled pool: the displacement striping.
        "blob" => {
            stone_floor(&mut w);
            for y in 0..floor_y {
                w.set(120, y, Cell::new(material::STONE, 0));
                w.set(392, y, Cell::new(material::STONE, 0));
            }
            for x in 121..392 {
                for y in 160..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
            w.paint_circle(256, 80, 34, material::SAND);
        }
        // Sand blobs dropped on a floor: the original chunk-seam cliffs.
        "sand" => {
            stone_floor(&mut w);
            w.paint_circle(120, 70, 40, material::SAND);
            w.paint_circle(250, 70, 40, material::SAND);
            w.paint_circle(380, 70, 24, material::SAND);
        }
        // M15 explosions. A settled sand pile with a stone slab buried in
        // it and open air above -- the geometry the owner's own live report
        // used ("two explosions in a sand pile"), plus enough solid
        // material for the crater edge and the fireball ring to read
        // against something that cannot avalanche. `explode=` fires into it.
        "boom" => {
            stone_floor(&mut w);
            for x in 100..412 {
                for y in 180..floor_y {
                    w.set(x, y, Cell::new(material::SAND, (rng::jitter(x, y) * 255.0) as u8));
                }
            }
            for x in 150..360 {
                for y in 230..244 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
        }
        // The same blast against `Solid` only, so nothing can slump in and
        // hide what the blast itself actually did.
        "boom_stone" => {
            stone_floor(&mut w);
            for x in 100..412 {
                for y in 180..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
        }
        // A deep, flat sand bed with nothing else in it -- for firing several
        // blasts at different *depths* below the free surface in one image.
        // The owner's own framing of the M15 complaint: material only blasts
        // around when the charge is near the edge, and "it just doesn't
        // happen if you're not really close."
        "sandbed" => {
            stone_floor(&mut w);
            for x in 20..492 {
                for y in 120..floor_y {
                    w.set(x, y, Cell::new(material::SAND, (rng::jitter(x, y) * 255.0) as u8));
                }
            }
        }
        // The same depth sweep in the other material the complaint named.
        // A liquid holds no angle of repose, so a crater here closes by
        // flowing rather than by avalanching.
        // A puddle and a lake side by side, the same depth, so the only
        // thing that differs is width -- `evaporation.rs`'s own paired
        // scene, so a sheet from this can be read against what the guards
        // measure rather than against a fresh scene nobody has a prior on.
        // Walled, because an open puddle spreads away across the floor
        // before the field registers it as a source at all, and the sheet
        // would then be a picture of spreading.
        //
        // Both bodies sit in *one* world here where the tests use one each,
        // and that is a real difference: the lake humidifies the puddle
        // (1.82 above the puddle against 1.45 for the same puddle alone),
        // so the puddle here dries somewhat slower than the guard's number.
        // Worth it -- one image showing both is the whole point.
        "evaporate" => {
            let floor = 160;
            for x in 0..WIDTH {
                for y in floor..(floor + 6) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            for (x0, width) in [(40, 6), (120, 240)] {
                for y in (floor - 4)..floor {
                    w.set(x0 - 1, y, Cell::new(material::STONE, 0));
                    w.set(x0 + width, y, Cell::new(material::STONE, 0));
                    for x in x0..(x0 + width) {
                        w.set(x, y, water_at(x, y));
                    }
                }
            }
        }
        "waterbed" => {
            stone_floor(&mut w);
            for x in 20..492 {
                for y in 120..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        // M16 plants. A single seed on a stone shelf with a puddle beside
        // it -- deliberately the same geometry as the committed live
        // verification shots (`docs/screenshots/tree-rewrite-live-
        // verification/`), so a sheet from this scene can be compared
        // directly against the artifact the owner's "still a tiny tree, one
        // cell thick, ~18 cells, no leaves, no roots" report was made about
        // rather than against a fresh scene nobody has a prior on.
        //
        // **The shelf height is load-bearing, not cosmetic.** `Germinate`'s
        // light gate reads `field_at(x, y - FIELD_SCALE).light`, and
        // `field.rs`'s `LIGHT_DECAY` puts the `0.1` crossing roughly 75
        // world cells below open sky. The ordinary `stone_floor` at
        // `HEIGHT - 8` is 300+ cells down and a seed there never germinates
        // at all -- which is exactly how the ported tree tests started
        // failing when they kept the old system's y=100-150 planting depth
        // (see `PLAN.md`'s tree-rewrite step 7 entry). `TREE_GROUND_Y` sits
        // comfortably inside the lit band with headroom, per this repo's
        // "set bars from measurement with headroom" convention.
        "tree" => {
            for x in 0..WIDTH {
                for y in TREE_GROUND_Y..(TREE_GROUND_Y + 6) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            w.plant_tree(200, TREE_GROUND_Y - 1);
            w.paint_circle(150, TREE_GROUND_Y - 4, 7, material::WATER);
        }
        // The same shelf, but a soil bed over a water table and several
        // seeds spaced across it. This is the scene the later phases are
        // aimed at -- roots growing into soil, soil moisture being drunk
        // down, canopies competing for light -- and today it should show
        // *none* of that, which is the point of shooting it now: it is the
        // before-picture for work that has not started.
        "forest" => {
            // `soil` has no `material::` constant of its own -- it was
            // appended to `EMBEDDED` deliberately (see that array's own
            // comment on why inserting rather than appending would
            // renumber the well-known ids), so it is looked up by name
            // like `wood` and `moss` already are elsewhere.
            let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
            for x in 0..WIDTH {
                for y in (TREE_GROUND_Y + 40)..(TREE_GROUND_Y + 46) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
                // Soil all the way down to the stone.
                //
                // An earlier version buried a band of free `water` in the
                // lower soil as a stand-in water table. That does not work
                // and the reason is worth keeping: `soil` is a `Powder` and
                // sinks through `Liquid`, so within a few hundred frames the
                // soil had swapped places with the water and the "table" was
                // a film lying on the *surface* — with every seed then
                // germinating onto water rather than soil, and no root ever
                // starting. Correct physics, useless scene, and it read as a
                // root bug.
                //
                // A real water table needs moisture held *inside* soil
                // cells, which is Decision 3 (§4a, per-cell fill in a
                // `Powder`'s own `aux`). Until that lands the honest scene
                // is dry soil, and the puddle below is on the surface where
                // it will stay put.
                // Starts at field capacity -- damp, the way real ground
                // between rain events is, and the state a root system
                // actually lives in. `Powder` aux is moisture now
                // (`material::SOIL_SATURATED`, where 0 means *dry*, the
                // opposite of a liquid's fill), and a scene of bone-dry
                // soil would sit below the wilting point where `Absorb`
                // correctly credits nothing at all.
                for y in TREE_GROUND_Y..(TREE_GROUND_Y + 40) {
                    w.set(x, y, Cell::new(soil, (rng::jitter(x, y) * 255.0) as u8).with_aux(material::SOIL_FIELD_CAPACITY));
                }
            }
            w.paint_circle(260, TREE_GROUND_Y - 3, 6, material::WATER);
            // Dropped from well above the ground on purpose: a seed is a
            // Powder, so this exercises the fall and landing rather than
            // pre-placing each seed on the surface.
            for x in [80, 200, 320, 440] {
                w.plant_tree(x, TREE_GROUND_Y - 25);
            }
        }
        // **A scene built for growing plants, rather than one built for
        // particle physics and reused.** `Reports/tree-architecture-
        // research.md` §6: every judgement about tree shape up to this
        // point was made in `forest`, which puts ground at y=40 in a
        // 320-tall world -- 40 rows of sky against 280 of dirt, because it
        // was laid out when depth was the interesting axis. Trees reached
        // that ceiling and could only spread sideways, and the resulting
        // silhouette was read as "canopies merge into a slab" and chased
        // as a plant bug for two sessions. Measured: at 40 rows of sky the
        // widest above-ground row is 56 cells; at 70 it is **7**.
        //
        // So this scene inverts the proportions -- a deep sky over a soil
        // bed just thick enough for a real root system -- and is the one
        // to judge plant *shape* in. `forest` is kept as it is: it is
        // still the right scene for root/soil work, and every earlier
        // sheet was shot in it.
        // Built from `common::PlantScene`, the same code `plant_probe`
        // uses -- see that module for why these two harnesses may not build
        // their own worlds any more.
        // `grove`, on the heterogeneous bed -- the same scene with three
        // conflicting tasks in it (moisture gradient, varying soil depth,
        // clumped founders). A separate scene name rather than a knob on
        // `grove`, so every stored `grove` sheet still means what it meant.
        "gradient" => {
            let base = common::PlantScene::varied();
            let plants = if args.plants > 0 { args.plants } else { base.trees };
            return common::PlantScene {
                species: args.species.clone(),
                trees: plants,
                soil_moisture: args.soil_moisture,
                soil_depth: args.soil_depth,
                start_frame: args.frame0,
                ..base
            }
            .build();
        }
        "grove" => {
            let base = common::PlantScene::default();
            let plants = if args.plants > 0 { args.plants } else { base.trees };
            return common::PlantScene {
                species: args.species.clone(),
                trees: plants,
                soil_moisture: args.soil_moisture,
                soil_depth: args.soil_depth,
                start_frame: args.frame0,
                ..base
            }
            .build();
        }
        // `grove`, plus a gnome who walks the length of it once the trees
        // are actually trees. The one question it exists to answer: does he
        // *get through*, or does he wedge against the first trunk the way
        // he did before living tissue stopped being a wall.
        //
        // Read the **distance** printed beside the tile, not the picture. A
        // gnome standing against a trunk and a gnome standing in one are
        // the same few pixels at contact-sheet zoom, and the difference
        // between them is the entire change.
        //
        // **It honours `plants=` and `frame0=`, and that is the guard's
        // point rather than a convenience.** A gnome case over a *grown*
        // stand is a guard over a procedural system, and `CLAUDE.md`'s
        // rule is that such a guard has to sweep the procedure or it is
        // blind by construction. `PlantScene` takes no seed, so the two
        // axes that actually redraw the stand are the tree count and the
        // start frame -- the latter because `weather::at` is a pure
        // function of `(seed, frame)`, so a different window grows the
        // stand under different rain and leaves the litter and spilled
        // soil lying differently. Sweep them and read the **min** across
        // runs, never one run: measured over six start frames the same
        // build spanned 47 to 357 cells travelled.
        "wood" => {
            let mut world = gnome_stand(args);
            world.player = Some(pixel_physics::sim::player::Player::at(12, 190));
            return world;
        }
        // The same grown stand, but he walks until he has hold of a tree
        // and then goes up it. Read the **climbed** counter beside the
        // tile, not the picture: a gnome at the top of a tree and a gnome
        // shoved up there by the depenetration pass are the same few pixels
        // at this zoom, and only a number separates them.
        "climb" => {
            let mut world = gnome_stand(args);
            world.player = Some(pixel_physics::sim::player::Player::at(12, 190));
            return world;
        }
        // Walk to a tree and shake it. Read the counters: a tree that shed
        // nothing and a shake that never fired are the same picture, and
        // `shake_shed` is graded by shade, so a healthy stand is *supposed*
        // to drop very little.
        "shake" => {
            let mut world = gnome_stand(args);
            world.player = Some(pixel_physics::sim::player::Player::at(12, 190));
            return world;
        }
        // The sandbox's *real* starting terrain, built by the same
        // `app::build_terrain` the running game calls -- not a replica, so
        // what this renders is what a player actually sees. Exists to answer
        // one question by eye: with structural checks computed at generation
        // rather than skipped, does any of it move? The three ledges float
        // with no in-plane path to bedrock and stand only because 6 cells is
        // thicker than stone's confinement diameter, which is exactly the
        // claim `Reports/worldgen-design.md` §6b says would collapse the
        // world if it were wrong.
        // **A colony you can actually look at**, which until now did not
        // exist anywhere. Everything about ants had been judged by counters
        // -- 402 genomes of statistics and not one picture -- which is the
        // exact inversion `CLAUDE.md` opens by warning against. An image
        // says *what and where*; the numbers already say how much.
        //
        // `genome=` picks who is being drawn, by the same label
        // `creature_space` prints, so a row in that sweep and a sheet here
        // are the same animal (they share `brain::random_genome`). The three
        // worth looking at, from the 400-genome run:
        //
        //   genome=r029   short-range forager, the best survivor found
        //   genome=r017   long-range directed commuter
        //   genome=r059   sessile grazer -- never moves a cell, still eats
        //   genome=authored   the hand-tuned ant, which r029 beats
        //   genome=zero   cannot move at all; the floor, and the control
        //
        // Wetland, deliberately: it is the only preset where moss both
        // exists at ant height and replenishes (86 -> 1,194 cells over a
        // run, against 10 -> 10 on "rolling"), and moss is what makes the
        // sessile strategy possible at all. On dry terrain r059 would just
        // look like a dead ant, and the sheet would be evidence of nothing.
        //
        // Look at it with `channel=pheromone_b` to see the food trail form,
        // and `channel=pheromone_a` for the nest scent.
        //
        // **Capture at noon, or the sheet is unreadable.** The day/night
        // oscillator has a period of 3600 frames and frame 0 is noon
        // (`field::DAY_NIGHT_PERIOD_FRAMES`), and this scene's 2,400-frame
        // warmup leaves capture starting two thirds through the cycle -- in
        // the dark. The first sheet rendered was six tiles of night. Use
        // `start=1200 every=3600` so every tile lands at exactly noon and
        // the tiles differ by behaviour rather than by time of day:
        //
        //   cargo run --release --example filmstrip -- scene=colony         //     genome=r029 start=1200 every=3600 count=6 cols=3 zoom=2
        // **Meat, at the size a corpse is actually seen.** A row of bodies
        // laid out from starved to killed-in-its-prime, plus the one a fire
        // left, so the question "can you tell rich meat from poor meat"
        // gets asked at play scale instead of from a palette listing.
        //
        // Worths are stamped directly rather than by starving real ants:
        // what is being judged is the *appearance ramp*, and driving it
        // through a colony would make the sheet a picture of which ants
        // happened to die well. `creature_dies` derives the same shade from
        // the same numbers -- see `a_corpse_is_worth_what_the_animal_was_
        // made_of` for the tie between them.
        "carrion" => {
            let corpse = w.materials.id_of("corpse").expect("corpse is compiled in");
            let soil = w.materials.id_of("soil").expect("soil is compiled in");
            let floor = 150;
            for x in 0..w.bounds().expect("bounded").max_x {
                for y in floor..(floor + 8) {
                    w.set(x, y, Cell::new(soil, 0).with_attached(true));
                }
            }
            // `ant.ron`: body_energy 120, start_energy 900, so a corpse runs
            // from 120 (starved, dead at exactly zero) to 1020 (killed with
            // a full bank). The shade ramp in `creature_dies` divides by
            // that same 1020.
            let full = 1020.0f32;
            let shades = w.materials.get(corpse).palette.len().max(1) as u32;
            for (i, worth) in [120u16, 320, 520, 760, 1020].into_iter().enumerate() {
                let shade = ((worth as f32 / full).clamp(0.0, 1.0) * (shades - 1) as f32).round() as u8;
                let x = 24 + i as i32 * 24;
                for dx in 0..2 {
                    w.set(x + dx, floor - 1, Cell::new(corpse, shade).with_aux(worth));
                }
            }
            // And the burnt one, which arrives with no stamp at all. Shade 0
            // and `aux` 0 is exactly what `fire.rs`'s burnout now writes for
            // a material whose shade is derived -- it used to draw at random,
            // which put a burnt ant at the bright end one time in five once
            // this ramp was wide enough to read. Priced by the material
            // fallback, so it belongs at the dark end beside the starved one.
            for dx in 0..2 {
                w.set(24 + 5 * 24 + dx, floor - 1, Cell::new(corpse, 0));
            }
        }
        "colony" => {
            let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
            if let Some(e) = err {
                panic!("worldgen presets unavailable: {e}");
            }
            let params = presets.get("wetland").expect("the wetland preset");
            pixel_physics::worldgen::generate(&mut w, pixel_physics::worldgen::Spec::Generated { params, seed: args.seed });

            let species = w.species.id_of("ant").expect("ant species");
            let genome = match args.genome.as_str() {
                "authored" => w.species.get(species).genome.clone(),
                "zero" => vec![0.0; pixel_physics::sim::brain::GENOME_LEN],
                label => {
                    let n: u64 = label.trim_start_matches('r').parse().unwrap_or_else(|_| {
                        panic!("genome={label:?}: expected \"authored\", \"zero\", or \"rNNN\"")
                    });
                    pixel_physics::sim::brain::random_genome(pixel_physics::sim::brain::sweep_genome_seed(n))
                }
            };
            w.species.set_genome(species, genome);

            // Let the trees put leaves out before the ants arrive; a
            // seedling is not a food source, and a scene whose food has not
            // grown yet measures the warmup rather than the colony.
            for _ in 0..2400 {
                pixel_physics::sim::parallel::step(&mut w);
                w.step_active_sites();
                w.step_fields();
            }
            // Found it on the ground, the same call the `Y` key makes, so
            // the sheet shows what a player actually gets.
            // **Find DRY land, not just "the first thing from the top".**
            // The obvious version put the colony at mid-width, where this
            // seed happens to have a lake, so the surface it found was
            // water: 48 ants were placed on a surface they cannot stand on
            // and the contact sheet was a picture of a lake. Wetland has
            // water in it by definition -- that is the point of it -- so the
            // scene has to say which surface it wants.
            use pixel_physics::sim::material::MaterialKind;
            // **Ask the engine where a colony can go; do not re-derive it.**
            // This scene used to carry its own `dry_surface` predicate, and
            // `World::found_colony` carried a different one -- which is
            // `Reports/open-bugs-handoff.md` §R2, and it cost this scene its
            // default seed for six days. `creature::colony_ant_site` is now
            // the single definition and both callers use it, so the scene
            // can no longer believe it chose ground that placement refuses.
            use pixel_physics::sim::creature::colony_ant_site;
            // Scored at the row the colony would actually be founded at, so
            // the estimate and the placement see the same cursor.
            let would_place = |w: &World, x: i32| -> i32 {
                let Some(cy) = colony_ant_site(w, x, 0) else { return 0 };
                (0..52).filter(|i| colony_ant_site(w, x - 102 + i * 4, cy - 2).is_some()).count() as i32
            };
            // Only where the colony's whole 204-cell span fits inside the
            // world: `found_colony` centres 52 ants at spacing 4, and
            // founding it near an edge silently drops every ant that lands
            // outside (16 of 52, the first time this scene ran).
            let half_span = 102;
            let (cx, cy) = (half_span..WIDTH - half_span)
                .filter_map(|x| colony_ant_site(&w, x, 0).map(|y| (x, y)))
                // Most dry ground within reach, ties broken toward the
                // middle of the map. A score rather than a hard window: on a
                // wetland seed there may be no unbroken 200-cell beach, and
                // demanding one made the scene panic rather than degrade.
                .max_by_key(|&(x, _)| (would_place(&w, x), -(x - WIDTH / 2).abs()))
                // **Say which seed, and say what was there instead.** This
                // `.expect` fired on the scene's own default seed for six
                // days, and its message ("some dry ground") sent the next
                // reader looking for a lake. Name the seed so the failure is
                // reproducible from the line, and census the band so the
                // reader learns whether the obstruction was water or wood.
                .unwrap_or_else(|| {
                    let (mut liquid, mut plant, mut empty) = (0, 0, 0);
                    for x in half_span..WIDTH - half_span {
                        match (0..HEIGHT).find(|&y| !matches!(w.materials.kind(w.get(x, y).material), MaterialKind::Empty | MaterialKind::Gas)) {
                            None => empty += 1,
                            Some(y) => match w.materials.kind(w.get(x, y).material) {
                                MaterialKind::Liquid => liquid += 1,
                                MaterialKind::Plant => plant += 1,
                                _ => {}
                            },
                        }
                    }
                    panic!(
                        "scene=colony seed={}: no dry ground in columns {half_span}..{}. \
                         Topmost cell over that band: {liquid} liquid, {plant} plant, {empty} empty. \
                         Try another seed=.",
                        args.seed,
                        WIDTH - half_span
                    )
                });
            // **Before placement, not after.** Measured after the call, this
            // counts only the sites the colony did *not* use -- every ant
            // makes its own site read as occupied -- so it came out lower
            // than `placed` on every seed. Arithmetically correct, and an
            // answer to a different question.
            let viable = would_place(&w, cx);
            let placed = w.found_colony(cx, cy - 2);
            assert!(placed > 0, "the colony scene placed no ants -- the scene is not showing what it claims to");
            // **Three numbers, because the two gaps have different causes.**
            // 52 -> viable is the scene losing sites to water or a canopy;
            // viable -> placed is `found_colony` disagreeing with the scene
            // about what counts as ground, which is the predicate mismatch
            // `open-bugs-handoff.md` §R2 flags. One number hides which.
            println!(
                "scene=colony genome={} seed={} : {placed} ants founded of {viable} viable sites of 52 asked, at x={cx}, surface y={cy}",
                args.genome, args.seed
            );
            println!("  suggested crop: crop={},{},240,110", cx - 120, cy - 70);
        }
        // **One verb, four bodies, four identical plinths.** The whole claim
        // of `Reports/creature-motion-design.md` §5 is that the *body*
        // decides what an impulse does, so a single hopping ant demonstrates
        // nothing -- what has to be on screen is several bodies given the
        // same verb and doing different things with it.
        //
        // Four lanes, each a shelf at the same height over the same drop,
        // each carrying one of the shipped body plans:
        //
        //   ant        Chain(2)   2 cells   the cheap generalist
        //   ant_long   Chain(6)   6 cells   shallower, and a plate when strung out
        //   ant_wide   5x2 rigid  9 cells   barely leaves the ground, then glides
        //   ant_block  3x3 rigid  9 cells   the same mass, and drops like a stone
        //
        // **The last two are the controlled pair and the reason the scene is
        // a cliff rather than flat ground.** Nine cells each at density 1.0,
        // so `LAUNCH_WORK` gives them the identical launch speed; everything
        // that separates them is drag, and drag only separates them over a
        // drop taller than their own 1.5-cell hop. On the flat they are
        // indistinguishable, which is correct and is not a picture.
        //
        // **The impulse is wired here, not in the species files.** Those four
        // are the appearance lane's candidates and their whole value is that
        // each is `ant.ron` with exactly one line changed (`body:`); adding a
        // second difference would make every appearance sheet ever taken
        // uncomparable. So the scene appends `(Bias, Impulse, w)` to each
        // one's authored wiring at build time, the same trick
        // `scene=colony genome=` uses. Nothing shipped hops by default.
        //
        // **Three knobs, and each exists because a measurement needed it.**
        //
        //   impulse=0   its own control: `ant.ron`'s wiring exactly, so the
        //               arm is the pre-verb engine and not a stand-in for it
        //   body=NAME   one lane only -- `report_colony`'s blocked fraction
        //               is world-summed, so a scene holding one chain and
        //               three rigid bodies reports a figure belonging to
        //               none of them
        //   shelf=ROW   trade drop height for zoom. The drag law separates
        //               two equal-mass bodies over a *long* drop, and a long
        //               drop is what forces the zoom down until a 9-cell
        //               creature is a smudge
        //
        // The whole thing, and the pair at a legible size:
        //
        //   cargo run --release --example filmstrip -- scene=hop \
        //     start=0 every=8 count=12 cols=4 zoom=2 crop=0,100,512,216
        //   cargo run --release --example filmstrip -- scene=hop shelf=232 \
        //     start=30 every=8 count=6 cols=3 zoom=6 crop=306,228,160,88
        "hop" => {
            use pixel_physics::sim::brain::{BrainInput, BrainOutput, Instinct};
            stone_floor(&mut w);
            // One shelf per lane. `attached`, like `stone_floor`, or a shelf
            // anchored only at its ends erodes inward from every free face
            // and the animals fall before they ever decide anything.
            let shelf_y = HOP_SHELF_Y;
            for &(x0, name) in HOP_LANES.iter() {
                if !args.hop_body.is_empty() && args.hop_body != name {
                    continue;
                }
                for x in x0..(x0 + HOP_SHELF_WIDTH) {
                    for y in shelf_y..(shelf_y + 5) {
                        w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                    }
                }
            }
            for &(x0, name) in HOP_LANES.iter() {
                if !args.hop_body.is_empty() && args.hop_body != name {
                    continue;
                }
                let Some(species) = w.species.id_of(name) else {
                    panic!("scene=hop: species {name:?} is not compiled in -- see organism.rs's SPECIES list");
                };
                let def = w.species.get(species).creature.as_ref().expect("a creature species").clone();
                // The authored wiring plus one connection, so the animal is
                // the shipped one in every other respect. `Bias` is always
                // 1.0, so a positive weight here is a standing intent to
                // jump whenever the move roll succeeds.
                let mut wiring: Vec<Instinct> = def.instincts.clone();
                // At weight 0 this is `ant.ron`'s wiring exactly: `squash(0)`
                // is 0, the gate never opens, and the arm is the pre-verb
                // engine rather than an approximation of it.
                if args.impulse != 0.0 {
                    wiring.push(Instinct(BrainInput::Bias, BrainOutput::Impulse, args.impulse));
                }
                let genome = pixel_physics::sim::brain::genome_from_wiring(&wiring, &def.hidden_wiring, &def.hidden_outputs, &def.recurrence);
                w.species.set_genome(species, genome);
                // At the right-hand end of its own shelf, facing east: a
                // couple of steps and it is at the edge.
                let x = x0 + HOP_SHELF_WIDTH - 4;
                if let Some(site) = pixel_physics::sim::creature::plant_creature_seed(&mut w, x, shelf_y - 1, name) {
                    w.schedule_active_site(site);
                } else {
                    panic!("scene=hop: {name:?} would not fit at ({x}, {})", shelf_y - 1);
                }
            }
            // **The harness echoes its own parameters.** `CLAUDE.md`'s
            // megastudy post-mortem: a knob nobody can see the value of is a
            // knob nobody can tell is disconnected, and a 3.5-hour study
            // shipped 24 logs of 3 populations because of exactly that.
            println!(
                "scene=hop: impulse={} body={} | one verb, shelf at y={shelf_y} over a {}-cell drop",
                args.impulse,
                if args.hop_body.is_empty() { "all four" } else { &args.hop_body },
                HEIGHT - FLOOR_THICKNESS - shelf_y
            );
            println!("  suggested crop: crop=0,{},512,{}", shelf_y - 12, HEIGHT - FLOOR_THICKNESS - shelf_y + 20);
        }
        "terrain" => {
            pixel_physics::app::build_terrain(&mut w);
        }
        // **One dig into a generated world.** Reported from play: "one crack
        // in the ground basically propagates throughout the whole world and
        // slowly breaks everything."
        //
        // Kept separate from `scene=worldgen` rather than added to it as a
        // flag: that scene is worldgen's own, it asserts that a generated
        // world arrives at rest and *stays* there, and a scene that
        // sometimes digs would make its zero-failure reading mean two
        // different things.
        //
        // The cut lands on the surface at mid-width, found by walking down
        // rather than assumed -- generated terrain puts the surface
        // wherever it likes, and a dig into open air would reproduce
        // nothing while looking like it had.
        // M9: the gnome on an obstacle course -- flat floor, a 2-cell curb
        // the step-up should climb without a jump, and a 10-cell platform
        // the scripted full jump (`gnome_script`) should clear. Judged on
        // the contact sheet: is the arc an arc, does the landing look like
        // a landing, does he climb the curb without stuttering.
        "gnome" => {
            stone_floor(&mut w);
            for x in 150..170 {
                for y in (floor_y - 2)..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for x in 280..420 {
                for y in (floor_y - 10)..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            w.player = Some(pixel_physics::sim::player::Player::at(40, floor_y - 4));
        }
        // M9 phase 2: the gnome tunnels into a cliff face, held-digging
        // while walking into his own bore. What this is read for, in
        // order: does a bore actually *open* (rubble and stone are near
        // the same grey, so a dig that only loosens looks identical to
        // one that never fired -- read the bite counter next to the
        // image), does the spoil come out of the mouth, and can he walk
        // over what he has thrown behind him. The massif is deep enough
        // that the tunnel never breaks through, so the whole run is the
        // confined case rather than the easy one.
        // `scene=smash` shares the bed for the same reason `chop` shares
        // `shake`'s: the tool is the only variable, so the two sheets are
        // a controlled pair. What differs is what the sheet is read for --
        // the tunnel asks whether a bore opens, this asks whether the face
        // *fails*, which is a thing the pick cannot cause at all.
        "tunnel" | "smash" => {
            stone_floor(&mut w);
            for x in 180..WIDTH {
                for y in (floor_y - 90)..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            w.player = Some(pixel_physics::sim::player::Player::at(150, floor_y - 4));
        }
        // M9's own acceptance line, verbatim: "player can be buried by a
        // sand dump and dig out". A column of sand drops on a standing
        // gnome, entombs him, and he digs his way clear -- the counters
        // report which tick he went under and which tick he came back,
        // because "buried" is a flag no picture can show.
        //
        // A *dump*, sized deliberately: 20x30 of sand, which settles to a
        // heap burying him about a dozen cells deep. Not a mountain --
        // the first version of this scene dropped 2240 cells and buried
        // him thirty deep, where no opening exists within any bounded
        // spoil throw and he stays under for good. That is the correct
        // outcome for being at the bottom of a hill (`BURIED_THROW` is
        // finite on purpose) but it is not the case M9 names, and a scene
        // that tests the unreachable one tells you nothing about the
        // reachable one.
        "bury" => {
            stone_floor(&mut w);
            for x in 246..266 {
                for y in 240..270 {
                    w.set(x, y, Cell::new(material::SAND, (x % 3) as u8));
                }
            }
            w.player = Some(pixel_physics::sim::player::Player::at(256, floor_y - 4));
        }
        // M9 phase 3: water. He is dropped into a walled pool from a
        // height and the script runs the three things swimming has to get
        // right in order -- he sinks under his own momentum, floats back
        // up rather than walking the bottom, is pulled under again by
        // held `S`, and finally jumps clear of the surface. The last of
        // those is the one that needed a mechanism of its own (the coyote
        // window while submerged); the rest is buoyancy and damping.
        "swim" => {
            stone_floor(&mut w);
            for y in 150..floor_y {
                for x in 180..190 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
                for x in 330..340 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for x in 190..330 {
                for y in 190..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
            w.player = Some(pixel_physics::sim::player::Player::at(260, 120));
        }
        // M9's other acceptance line: "stands on a tumbling rigid body".
        // The `undercut` recipe with a passenger -- a shelf cut off its
        // support, which the structural pass promotes to a chunk body and
        // drops. He is standing on it when it goes. Bodies live off-grid,
        // so before phase 3 a grid read could not see one and the shelf
        // fell straight through him; what this sheet is read for is
        // whether he rides it down and is still on top when it lands.
        "ride" => {
            stone_floor(&mut w);
            for y in 120..260 {
                for x in 0..90 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // A *short* shelf. The full 120-cell `undercut` span shatters
            // into twenty-odd bodies the moment it goes, which is correct
            // for that scene (it exists to show a big collapse) and
            // useless for this one: there is no raft to ride, only debris
            // to fall through. Sized down until it promotes as a couple
            // of coherent pieces.
            for y in 150..158 {
                for x in 90..186 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            for x in 92..184 {
                for y in 155..158 {
                    w.paint_capsule((x, y), (x, y), 0, material::EMPTY, 1.0);
                }
            }
            w.player = Some(pixel_physics::sim::player::Player::at(150, 143));
        }
        "worldcrack" => {
            let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
            if let Some(e) = err {
                panic!("{e}");
            }
            let name = if args.preset.is_empty() { presets.default_name() } else { args.preset.clone() };
            let Some(params) = presets.get(&name) else { panic!("unknown preset {name:?}") };
            pixel_physics::worldgen::generate(&mut w, pixel_physics::worldgen::Spec::Generated { params, seed: args.seed });
            let x = WIDTH / 2;
            let surface = (0..HEIGHT).find(|&y| w.get(x, y).material != material::EMPTY).expect("ground somewhere under mid-width");
            println!("worldcrack {name} seed {} -- cut at ({x}, {})", args.seed, surface + args.dig);
            // The verb matters more than the radius.  pulverizes a
            // core, loosens a chip zone *and* scores cracks out to three
            // times its radius -- and a crack severs a support path rather
            // than merely weakening it, so it manufactures pieces that have
            // to find a new way to the ground.  is the quiet cut. The
            // owner plays with the hammer, and nothing here covered it.
            // Can a tunnel be dug and stay open? The question the whole
            // milestone is about, and nothing here has ever asked it: every
            // scene cut *once*. A tunnel is the case where each bite stands
            // on the last one's spoil and under the last one's roof, so it
            // is the one that compounds.
            //
            // Driven in one burst rather than over time, deliberately: a
            // player digging at their own pace lets each bite settle before
            // the next, so this is the harsher reading of the same verb.
            if args.tunnel > 0 {
                let step = args.step.unwrap_or(args.dig).max(1);
                let depth = surface + args.depth.unwrap_or(args.dig * 3);
                for i in 0..args.tunnel {
                    pixel_physics::sim::rigid::mine(&mut w, x + i * step, depth, args.dig, args.dig_yield);
                }
            } else if args.strike > 0 {
                let force = args.strike as f32 * 0.9;
                pixel_physics::sim::rigid::strike(&mut w, x, surface + args.strike, args.strike, force);
            } else if args.dig > 0 {
                pixel_physics::sim::rigid::mine(&mut w, x, surface + args.dig, args.dig, args.dig_yield);
            }
            if args.relax {
                pixel_physics::sim::structural::compute_world_distances(&mut w);
            }
        }
        // The reference room `B` stamps, standing on the app's real terrain
        // rather than on a flat test floor. `scene=room` answers "does it
        // hold"; this answers "is it a sensible size", which is a different
        // question and needs the actual world in frame for scale. Render it
        // whole-world at zoom 1 -- the moment it is cropped or magnified the
        // only comparison it exists to make is gone.
        "refroom" => {
            let mut app = pixel_physics::app::App::new();
            app.stamp_reference_room(WIDTH / 2, 40);
            w = app.world;
        }
        // Generated terrain, the thing worldgen is judged on. Reads
        // `assets/worldgen.ron` rather than a replica of it, for the same
        // reason `terrain` calls the real `build_terrain`: a scene that
        // approximated the generator would stop being evidence about the
        // generator the first time either drifted.
        //
        // `seed=` and `preset=` pick the world; a run with `count=1` is a
        // single still, which is what the seed sweep in the worldgen plan
        // uses. Stepping it at all is also the at-rest check in visual form
        // — generated terrain that settles is terrain that moved.
        "worldgen" => {
            let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
            if let Some(e) = err {
                panic!("{e}");
            }
            let name = if args.preset.is_empty() { presets.default_name() } else { args.preset.clone() };
            let Some(params) = presets.get(&name) else { panic!("unknown preset {name:?}") };
            let report = pixel_physics::worldgen::generate_reported(
                &mut w,
                pixel_physics::worldgen::Spec::Generated { params, seed: args.seed },
            );
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            // Printed next to the image on purpose. A contact sheet cannot
            // show whether a feature pass ran -- a terrace and an overhang
            // read the same at this zoom -- so a pass reporting zero here is
            // the only way to catch one that silently never fires.
            println!("worldgen {name} seed {}", args.seed);
            for (pass, cells) in &report {
                println!("  {pass:<14} {cells:>7} cells");
            }
        }
        // The payoff mechanic, and the one M17 "was built for and has never
        // had a real test case" (`Reports/worldgen-design.md` §7): mine
        // upward into a ledge until the roof left above the excavation is
        // thinner than stone can hold, and watch it come down. The bite is
        // taken out of the ledge's underside through the ordinary eraser
        // brush, so this goes through the same reactive path a player does.
        // Undercut an attached cliff shelf: erase the material *under* one
        // end of it so the outer part is left hanging over open air. This is
        // the case the whole model exists for, and the one that produced
        // nothing before `detach_exposed_neighbours` -- attached rock
        // anchors outright, so carving it used to just delete cells while
        // everything around them stayed permanently held.
        "undercut" => {
            stone_floor(&mut w);
            for y in 120..260 {
                for x in 0..90 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // A shelf continuing out from the cliff face, also part of the
            // massif.
            for y in 150..162 {
                for x in 90..210 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            // Dig the shelf's support away from underneath, through the
            // ordinary eraser brush.
            for x in 92..208 {
                for y in 156..162 {
                    w.paint_capsule((x, y), (x, y), 0, material::EMPTY, 1.0);
                }
            }
        }
        // A solid attached cliff, struck once. Nothing here is structurally
        // unsound, so every piece that leaves is leaving because it was hit
        // -- which is the whole point of having a verb at all.
        "strike" => {
            stone_floor(&mut w);
            for y in 60..280 {
                for x in 140..380 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            pixel_physics::sim::rigid::strike(&mut w, 260, 150, 14, 12.0);
        }
        // Work one spot on an attached shelf with repeated blows, the way a
        // player would with `C`. Each hit drives the fissure deeper and cuts
        // what the rock around it can carry, so the shelf should give way
        // after several -- rather than needing to be chewed away cell by
        // cell, which is what erasing amounts to.
        "worked" => {
            stone_floor(&mut w);
            for y in 120..280 {
                for x in 0..90 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for y in 150..164 {
                for x in 90..250 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            // Six blows on the shelf where it leaves the cliff -- the most
            // stressed point of a cantilever, and where a person would aim.
            for _ in 0..6 {
                pixel_physics::sim::rigid::strike(&mut w, 100, 157, 7, 6.0);
            }
        }
        // The reported case: a tall thick player-built column with a cap
        // overhanging it on both sides. Built as foreground (no
        // `with_attached`), exactly as the stone brush lays it down, so this
        // is what a player actually gets. It used to tear its own cap off
        // and dissolve most of it to dust.
        "capped" => {
            stone_floor(&mut w);
            for y in 120..floor_y {
                for x in 226..286 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            for y in 90..126 {
                for x in 196..316 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            w.schedule_structural_check_around(200, 108);
            w.schedule_structural_check_around(312, 108);
            // Disturbances as well as checks, and here it is the *guard*
            // that needs them rather than the outcome. This case asserts
            // `max_failures=0` -- the thick column stands -- and at the
            // shipped `TIGHT` reach an undisturbed scene cannot fail
            // whatever the load model thinks, so without this it could
            // pass on the leash rather than on the model.
            //
            // **Measured, it does not: at `chain_reach=spread` this scene
            // still reports 0 overload and 0 unsupported failures**, so
            // the column was always being held up by the model. These are
            // here so it cannot acquire that dependency later, not to
            // repair one. They also make the scene what its comment above
            // already claims it is -- what the stone brush lays down --
            // since `World::paint_capsule` records a disturbance per
            // structural cell it writes.
            w.record_disturbance(200, 108, 0);
            w.record_disturbance(312, 108, 0);
        }
        "mine" => {
            pixel_physics::app::build_terrain(&mut w);
            // The 60..200 ledge spans y=200..206. Erase its lower rows
            // across most of its length, leaving a roof too thin to stand.
            for x in 70..190 {
                for y in 202..206 {
                    w.paint_capsule((x, y), (x, y), 0, material::EMPTY, 1.0);
                }
            }
        }
        // Acceptance case 4, and the owner's original case: a big overhang
        // joined to the cliff by a deliberately thin ligament. It must snap
        // at the *neck*, not at the tip and not at all.
        //
        // This is the shape the reach model could not get right in
        // principle rather than by tuning. The ligament sits at *low*
        // distance -- it is right next to solid rock -- so reach says it is
        // fine, while the tip, being far out, is the part that fails. That
        // is backwards: rock fails where the stress is highest, and the
        // stress is highest where the section is thinnest.
        //
        // No blow and no erasing: the geometry alone has to be enough, so
        // nothing here can be mistaken for the strike doing the work.
        "ligament" => {
            stone_floor(&mut w);
            for y in 100..280 {
                for x in 0..100 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // The neck: 4 cells deep, 10 long. Everything beyond it has to
            // carry its moment through these forty cells.
            for y in 150..154 {
                for x in 100..110 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // The overhang: 40 deep and 110 long, hung off that neck.
            for y in 130..170 {
                for x in 110..220 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            // One structural check at the neck, and a disturbance to go
            // with it.
            //
            // **This used to read "which is all a disturbance would do",
            // and that stopped being true.** It was written when
            // `chain_reach` defaulted to no limit, where the ring was
            // never consulted and recording into it was genuinely a no-op.
            // With `TIGHT` the default, a failure is refused unless
            // something near it reported itself -- so the scheduled check
            // ran, found the neck overloaded, and was declined: zero
            // overload failures on the case that exists to show the
            // owner's original ligament snapping. In play the neck is thin
            // because someone cut it thin, and cutting records; this scene
            // builds it thin instead, so it has to say so itself.
            w.schedule_structural_check_around(105, 152);
            // Extent 0: nothing was removed and nothing struck, so the
            // "wound" is the single cell the check is asked about.
            w.record_disturbance(105, 152, 0);
        }
        // What a player actually builds, painted through the ordinary
        // brush at the radius they use (R2, so 5 cells thick).
        //
        // Reconstructed from a playtest screenshot after the report "most
        // things are still falling apart": a column with two arms, a tall
        // hook, and a wide arch, all foreground, all standing on the floor.
        // The screenshot's stress view read almost entirely *green* -- so
        // whatever is bringing these down is not the torque criterion, and
        // guessing which of the two failure paths it is from a picture is
        // exactly what the failure counters exist to stop.
        "built" => {
            stone_floor(&mut w);
            let paint = |w: &mut World, a: (i32, i32), b: (i32, i32)| {
                w.paint_capsule(a, b, 2, material::STONE, 1.0);
            };
            // The "F": a column off the floor with two short arms.
            paint(&mut w, (60, floor_y - 1), (60, 120));
            paint(&mut w, (60, 150), (110, 150));
            paint(&mut w, (60, 190), (105, 190));
            // The hook: up, then a long arm back over open air.
            paint(&mut w, (300, floor_y - 1), (300, 70));
            paint(&mut w, (300, 70), (210, 60));
            // The arch: two feet and a span, the case the model has the
            // most trouble with because an arch carries its load in
            // compression along its curve and this model only knows about
            // bending moment.
            let arch: Vec<(i32, i32)> = (0..=20)
                .map(|i| {
                    let t = i as f32 / 20.0;
                    let angle = std::f32::consts::PI * t;
                    (360 + (angle.cos() * -110.0) as i32, floor_y - 1 - (angle.sin() * 120.0) as i32)
                })
                .collect();
            for pair in arch.windows(2) {
                paint(&mut w, pair[0], pair[1]);
            }
        }
        // **The unzip reproduction.** A hollow room built exactly the way
        // `Tool::Room` builds one, then one cut into a wall -- one click of
        // `D`, which is what the report was.
        //
        // Two things have to be faithful for this to reproduce, and both
        // are easy to get wrong:
        //
        // - The walls go down through `paint_capsule_as`, not `set`,
        //   because that is what marks them intact and reruns the scoped
        //   relaxation. A room laid down with `set` is unattached
        //   foreground and fails for an entirely different reason.
        // - The four walls are drawn as four *overlapping* capsule runs, so
        //   the corners are covered twice. Four independent segments leak
        //   at every corner, which structurally means the roof is not
        //   carried by the walls at all -- the scene would then be
        //   measuring a bug in itself.
        //
        // The cut is at mid-height on the left wall, where a doorway goes.
        //
        // **What this scene found.** `Tool::Room` sets wall thickness from
        // `brush_radius` and `App::mine` passes the *same* `brush_radius`
        // through as the cut radius. A capsule of radius r is `2r+1` thick
        // and a dig of radius r is `2r+1` across, so a cut into a wall
        // built by the room tool severs it completely, every time, at any
        // height, at any brush size. There is no ligament left because
        // there cannot be one. The roof then hangs off the far wall alone
        // and duly overloads -- correctly. Two verbs sharing one number
        // where the whole point is that one must be smaller than the
        // other.
        // `wall=` and `dig=` are separate knobs *because the app gives them
        // the same number* and that turns out to be the whole bug -- see
        // the note above. Being able to vary them independently here is how
        // the question "how much thicker than a cut does a wall have to be"
        // gets an answer instead of an argument.
        "room" => {
            stone_floor(&mut w);
            let (x0, y0) = (140, 150);
            let (x1, y1) = (x0 + args.span, floor_y - 1);
            for (a, b) in [((x0, y0), (x1, y0)), ((x0, y1), (x1, y1)), ((x0, y0), (x0, y1)), ((x1, y0), (x1, y1))] {
                w.paint_capsule_as(a, b, args.wall, material::STONE, 1.0);
            }
            // One click, at mid-height on the left wall. `dig=0` makes no
            // cut at all, which is the control: it says whether the room
            // even stands untouched, and that has to be established before
            // any number from a cut means anything.
            if args.dig > 0 {
                pixel_physics::sim::rigid::mine(&mut w, x0, (y0 + y1) / 2, args.dig, args.dig_yield);
            }
        }
        // A thin shelf cantilevered off a thick pillar, with the join then
        // cut so the shelf detaches whole.
        //
        // This is the case that actually produces an M8 chunk body, and the
        // contrast with `mine` is the point. A mined roof fails
        // *progressively* -- its cells sit at genuinely different distances
        // (17, 18, 19...) and cross their span on different ticks, so at the
        // instant the first one breaks its neighbours are still supported
        // and there is no region to promote. A *detached* region has no
        // anchor at all, so its cells climb in lockstep (the
        // count-to-infinity dynamic in `structural.rs`'s module doc) and
        // cross together, which is what gives `try_promote_failing_region` a
        // whole connected region to find at once.
        //
        // The shelf is deliberately 3 cells thick: thicker than stone's
        // confinement diameter and it would anchor itself and hang there
        // forever, which is documented, intended behaviour and not what this
        // scene is for.
        "snap" => {
            stone_floor(&mut w);
            // The pillar is a cliff: attached, so it anchors the shelf and
            // does not crumble under its own weight.
            for y in 120..200 {
                for x in 60..80 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // The shelf is *not* attached -- it is the thing under test, and
            // cutting it off its pillar should drop it.
            for y in 140..143 {
                for x in 80..112 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
            // Cut the shelf off its pillar, through the ordinary eraser
            // brush so this goes down the same reactive path a player does.
            for y in 140..143 {
                w.paint_capsule((80, y), (80, y), 0, material::EMPTY, 1.0);
            }
        }
        // A natural cave: a void inside *attached* rock, the situation every
        // mining blast actually happens in. `boom_stone` answers "what does a
        // blast do to a solid mass"; this answers the two questions that mass
        // cannot ask -- what happens at a cave *wall* (one free face beside
        // the charge) and under a cave *roof* (free face below, gravity
        // pointing into it). Place the charge with the ordinary `explode=`
        // arg: (186,240) is inside the left wall, (256,200) is inside the
        // roof. The printed "roofed void (cave volume)" line is the number to
        // read -- a blast that widens the cave grows it, one that caves the
        // roof in shrinks it.
        "cavern" => {
            stone_floor(&mut w);
            for x in 106..406 {
                for y in 130..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            // The cave: an ellipse, 120 wide by 56 tall, roof 82 cells thick.
            let (cx, cy, a, b) = (256i32, 240i32, 60f32, 28f32);
            for x in (cx - 60)..=(cx + 60) {
                for y in (cy - 28)..=(cy + 28) {
                    let (dx, dy) = ((x - cx) as f32 / a, (y - cy) as f32 / b);
                    if dx * dx + dy * dy <= 1.0 {
                        w.set(x, y, Cell::EMPTY);
                    }
                }
            }
            pixel_physics::sim::structural::compute_world_distances(&mut w);
        }
        // **The felling bed: one tree, alone, at a fixed x, with room to
        // fall.** The instrument `Reports/felling-blockers.md` §3 asks for
        // first, and `Reports/plant-project-review-2026-08-23.md` D1.
        //
        // `grove plants=1` grows the same tree and was the starting point.
        // It is not enough for this question for one reason: `PlantScene`
        // spaces its stand as `width / (trees + 1)`, so the trunk moves
        // every time `plants=` changes, and a `cut=`/`chop=` rectangle
        // aimed at the trunk in one run lands in open sky in the next --
        // which reads on a contact sheet as *the cut did nothing* rather
        // than as *the cut missed by thirty cells*. That is precisely the
        // confusion `fire_due_cuts` already prints a living-tissue count to
        // prevent, and a scene whose subject cannot move is the cheaper
        // half of the same fix.
        //
        // So this is `PlantScene` with `trees: 1` and nothing else changed:
        // the trunk stands at `FELL_TRUNK_X` in every run, at every
        // species, for as long as the bed is 512 wide. `plants=` is
        // deliberately ignored here (use `grove` for a stand) -- honouring
        // it would give the scene back the one property it exists to
        // remove. `species=`, `moisture=` and `frame0=` pass through.
        //
        // The bed itself is built by `common::PlantScene`, not here: the
        // two plant harnesses may not build their own worlds (see that
        // module for the drift this ended).
        // **`scene=chop` is the felling bed with a gnome in it**, not the
        // shake bed, and the first version got that wrong in a way the
        // counters caught. On `gnome_stand` he chopped twice, his own chips
        // buried him at tick 6107, and 212 further strokes swung at a pile
        // he was entombed in — `4 strokes (2 on living tissue)` climbing to
        // `214 strokes (2 on living tissue)` while `BURIED` never cleared.
        // A cutting verb wants firm ground and something with a bole; the
        // shake bed is soft forest floor and creeper stems.
        //
        // Sharing `fell`'s bed also makes the pair worth having: the same
        // tree, cut by hand there and cut by the gnome here.
        "fell" | "chop" => {
            let w = common::PlantScene {
                species: args.species.clone(),
                trees: 1,
                soil_moisture: args.soil_moisture,
                start_frame: args.frame0,
                ..common::PlantScene::default()
            }
            .build();
            // Asserted, not assumed. The whole value of this scene is that
            // a coordinate written into a `cut=` keeps working, and a
            // spacing change in `PlantScene` would break that silently:
            // the run would still produce a sheet, of an untouched tree,
            // which is indistinguishable from a support model that does
            // nothing.
            assert!(
                (0..HEIGHT).any(|y| w.get(FELL_TRUNK_X, y).organism_id() != 0),
                "scene=fell expects its seed at x={FELL_TRUNK_X}; PlantScene's spacing has moved"
            );
            let mut w = w;
            if args.scene == "chop" {
                // Beside the bole, not in it, and on the ground the tree is
                // rooted in. `Script::Chop` walks him the last few cells and
                // aims ahead of himself, so where he starts only has to be
                // near enough and clear.
                let ground = (0..HEIGHT)
                    .find(|&y| !w.is_empty(FELL_TRUNK_X - 20, y))
                    .unwrap_or(HEIGHT - 1);
                w.player = Some(pixel_physics::sim::player::Player::at(FELL_TRUNK_X - 20, ground - 8));
            }
            return w;
        }
        other => panic!(
            "unknown scene {other:?}; known: pour, fall, shelf, blob, sand, boom, boom_stone, sandbed, waterbed, tree, forest, grove, terrain, worldgen, mine, snap, undercut, strike, worked, capped, ligament, built, room, refroom, worldcrack, gnome, tunnel, smash, bury, swim, ride, cavern, wood, climb, shake, fell, chop"
        ),
    }
    w
}

struct Args {
    scene: String,
    /// `day=`/`weather=`/`growth=`/`creatures=`/`gnome=` — the world-speed
    /// knobs (`sim::clock`), each "N times slower than baseline".
    ///
    /// **Explicit here, and deliberately not read from `assets/clock.ron`.**
    /// The app loads that file and every harness leaves `World::new`'s
    /// baseline alone, which is what keeps several hundred guards and every
    /// stored contact sheet valid across a change to the shipped day length.
    /// The cost of that divergence is that a sheet rendered here is at
    /// whatever these say, so they are echoed on stdout beside the scene name
    /// — `CLAUDE.md`'s harness rule, after a 3.5-hour study turned out to be
    /// three populations wearing 24 logs.
    clock: pixel_physics::sim::clock::Clock,
    /// `seed=N` -- which generated world `scene=worldgen` builds.
    seed: u64,
    /// `yield=F` -- the gnome's `dig_yield`, for comparing the spoil
    /// modes the app cycles with `F2`. Whether a bore actually opens is
    /// decided entirely by this number (see `player::Tuning::dig_yield`),
    /// so a harness that could not vary it could not show the difference
    /// between "you cannot dig" and "rock simply goes".
    dig_yield: f32,
    /// `shoulder=N` -- the gnome's `shoulder_grains`, for sweeping how many
    /// loose grains above the wade line he pushes past. 0 is the old veto,
    /// under which one stray soil cell in a canopy was an impassable wall.
    shoulder_grains: u8,
    /// `digstyle=bore|free` -- which cut shape the pick uses, so the two can
    /// be rendered as a controlled pair on one scene. The app's own default is
    /// `bore`; `free` is what the pick did before it existed, and is still
    /// reachable in play on `4`.
    ///
    /// **Not `dig=` or `cut=`**, which `scene=room`'s cut radius and the
    /// crop rectangle have held all along -- see the parse arm for what
    /// claiming one of them twice cost.
    dig_style: pixel_physics::sim::player::DigStyle,
    /// `species=` -- which species `scene=grove` plants (tree, conifer,
    /// shrub). The grove is the shape harness, and Phase 2's whole point
    /// is that different species are different *shapes*.
    species: String,
    /// `moisture=N` -- how wet `scene=grove` starts, on `SOIL_SATURATED`'s
    /// scale. Field capacity by default; below `SOIL_WILTING_POINT` gives
    /// the dormancy arm, where seeds wait rather than germinate.
    soil_moisture: u16,
    /// `soil=N` -- how many rows of soil `scene=grove` beds the stand in,
    /// defaulting to `common::SOIL_DEPTH` (34).
    ///
    /// **`plant_probe` has had this knob all along and this file did not**,
    /// so a root comparison took its numbers at one depth and its picture at
    /// another. That gap produced a wrong published claim: at 34 rows the
    /// deep-rooting treatment's deepest individual measured exactly 34 --
    /// it was standing on the floor of the scene -- and its depth histogram,
    /// which is normalised to the soil column, read as bottom-heavy for that
    /// reason alone. Given 100 rows the same treatment reads shallow. The
    /// owner saw it before the harness did: *"Have you provided enough soil
    /// under the plant to really test differences."*
    ///
    /// `ground_y` is 200 in a 320-row world, so roughly 110 rows are
    /// available before the bed runs out of world.
    soil_depth: i32,
    /// `frame0=N` -- the frame the world starts on, which pins the weather
    /// (`weather::at` is pure in seed and frame). Prefer multiples of 3600:
    /// that pins the day phase, the sky and every organism's tick offset at
    /// once.
    frame0: u64,
    /// `plants=N` -- how many founders `scene=grove` plants, evenly
    /// spaced. Defaults to `PlantScene`'s own 8, which is tree spacing.
    ///
    /// Exists because a *ground layer* cannot be judged on tree spacing.
    /// Grass at 8 founders across 512 cells renders as four isolated
    /// sprigs per tile -- which says nothing about whether a sward reads
    /// as a surface layer, the actual question WP-B3's acceptance asks.
    /// That is this repo's own "a scene that contradicts the code will
    /// look like a bug in the code", in its cheaper form: a scene that
    /// cannot contain the artifact will look like the artifact is absent.
    plants: usize,
    /// `ignite=x,y,radius,frame` -- start a fire at a chosen frame, after
    /// the vegetation has had time to grow. Repeatable.
    ignitions: Vec<(i32, i32, i32, usize)>,
    /// `dry=aux,frame` -- reset every water-holding cell in the world to
    /// `aux` at `frame`, on `SOIL_SATURATED`'s scale.
    ///
    /// **A dry meadow cannot be grown, which is why this is a knob and not
    /// a scene parameter.** Measured on this branch: a sward started at
    /// soil 250 finishes 3,000 frames of growth with the ground at 289 and
    /// climbing, because unplanted soil has three moisture sources and one
    /// sink (`Reports/open-bugs-handoff.md` §F8, open). Every starting
    /// value from 250 to 620 therefore grows the *same* sward on damp
    /// ground, and a burn shot at any of them is a burn on damp ground.
    ///
    /// Resetting after growth and before the ignition gives **identical
    /// fuel and one variable**, which is the paired comparison the burn
    /// work needs -- the same reason `examples/fire_probe.rs` carries
    /// `burnmoisture=`. Leave a few hundred frames between this and
    /// `ignite=` for the moisture field to catch up with the ground.
    dries: Vec<(u16, usize)>,
    /// `preset=NAME` -- which entry of `assets/worldgen.ron` it uses. Empty
    /// means that file's own `default`.
    preset: String,
    /// `wall=N` / `dig=N` -- capsule radii for `scene=room`'s walls and for
    /// the cut made into them. Both default to 3, which is what the app
    /// itself does, because the app has only one number for both.
    wall: i32,
    dig: i32,
    /// Strike radius for scene=worldcrack. Zero means dig instead.
    strike: i32,
    /// Number of dig bites driven horizontally into the hill, as a tunnel.
    tunnel: i32,
    /// `relax=1` -- run a **converged** distance pass straight after the
    /// dig, instead of letting the scheduled relaxation reconverge over the
    /// following frames.
    ///
    /// The instrument for one specific question: is a failure real, or is
    /// it a cell judged mid-convergence against a stale distance? Those look
    /// identical in an image and `load.rs` has been bitten by the
    /// distinction before. Answered for the dig cascade: no, staleness is
    /// not the cause -- see `Reports/next-session-handoff.md` 1b.
    relax: bool,
    /// `span=N` -- how wide `scene=room` is drawn, outer edge to outer
    /// edge. The knob the whole "what can a player actually build" question
    /// turns on, and the reason it is a knob rather than a constant: a
    /// single width says the room stands or does not, and what is wanted is
    /// the *envelope*.
    span: i32,
    /// `fall=N` -- how many rows above the pool `scene=rockdrop` starts its
    /// slab. The lever on entry speed, which is what
    /// `rigid::SPLASH_MIN_ENTRY_SPEED` gates on.
    fall: i32,
    /// `pond=N` -- how wide `scene=coldsnap` cuts its pond, shore to shore.
    ///
    /// A knob rather than a constant for the same reason `span` is one: the
    /// structural question about a frozen sheet is not "does 60 cells
    /// hold" but *where the envelope is*, because the sheet's only anchor
    /// path is the shoreline and the answer is therefore a width. This is
    /// the argument ice.ron's `max_unsupported_span` was swept against.
    pond: i32,
    start: usize,
    every: usize,
    count: usize,
    /// `phase=noon` / `phase=off` -- force a contact sheet's samples onto a
    /// fixed point of the day/night cycle, or force that off. `None` decides
    /// it from the span; see `snap_to_noon`.
    phase: Option<bool>,
    cols: usize,
    zoom: i32,
    crop: Rect,
    parallel_driver: bool,
    /// `genome=` for `scene=colony`: `authored`, `zero`, or `rNNN` naming a
    /// genome from `creature_space`'s sweep by the label it printed.
    genome: String,
    /// `scene=hop`'s impulse weight. **A knob rather than a constant so the
    /// scene can run its own control**: at 0 nothing hops, and four bodies
    /// milling on four shelves is what the engine did before this verb
    /// existed. An A/B against that is the only way to say what the verb
    /// bought, and `CLAUDE.md` asks for the paired comparison rather than
    /// one run against a remembered impression.
    impulse: f32,
    /// `scene=hop`: run one lane only, by species name, or every lane when
    /// empty.
    ///
    /// **This is how the blocked-movement half of
    /// `creature-motion-design.md` §7 gets measured.** `report_colony`'s
    /// `blocked` fraction is summed over the whole world, so a scene holding
    /// a chain and three rigid bodies reports one number that belongs to
    /// none of them — the mean-over-events trap `CLAUDE.md` names, one level
    /// up. One lane at a time is what makes the figure attributable.
    hop_body: String,
    out: String,
    grain: GrainMode,
    /// `bubbles=` -- which of `render.rs`'s `BubbleMode` looks to draw
    /// boiling liquid with. `off` is the default and today's behaviour;
    /// the point of the argument is `scene=boil` side by side.
    bubbles: BubbleMode,
    /// `gas=` -- which of `render.rs`'s `GasMode` looks to draw gas with.
    /// `opaque` is the default and today's behaviour.
    gas: GasMode,
    /// `trees=weave|haze|front|behind` -- which `TreeDepth` the sheet is
    /// shot in. The whole value of a selector is being able to put its
    /// settings side by side, and a still image is the only way to compare
    /// two of them at once.
    tree_depth: TreeDepth,
    /// `channel=` -- render the sheet through one of `render.rs`'s debug
    /// overlays instead of ordinary material colour. The whole reason the
    /// plant work needs this harness: resource, canopy density and (later)
    /// vein conductance are per-cell scalars that decide plant shape and
    /// have **never been drawn**, which is how `tree-rewrite-design.md`
    /// §2b's self-avoidance mechanism shipped inert past two design
    /// reviews. A contact sheet in a channel shows both what the value is
    /// and how it evolves across the tiles, which is the question.
    organism_overlay: OrganismOverlay,
    field_overlay: FieldOverlay,
    /// `skylight=off|4|2|1` -- which sky-light mode to draw through, so the
    /// `9`/`F12` selector can be A/B'd on the structural scenes headlessly.
    sky_light: SkyLight,
    /// `daylight=<0.0..1.0>` -- draw every tile at one fixed hour instead
    /// of at whatever time of day the run happened to reach. `1.0` is noon,
    /// `0.0` the darkest the lighting term goes. Unset is the ordinary
    /// day/night cycle, so every sheet recorded before this existed still
    /// reproduces exactly.
    ///
    /// **Render-only, and not a cheat.** It reaches `Renderer::
    /// daylight_pin` and nothing else: the simulation's clock, the light
    /// field, plants and weather all still run on `world.frame`, so this
    /// changes what the sheet looks like and nothing about what happened
    /// in it. The nine-blast harness fires charges 400 frames apart into a
    /// 3,600-frame day, so its nine panels are nine different exposures --
    /// columns 3-6 come out at night and are genuinely hard to read -- and
    /// the tiles are being compared *to each other*. A variable that is not
    /// the one under test has to be held constant (`CLAUDE.md`: a channel
    /// that oscillates by design must be divided out of decisions).
    daylight: Option<f32>,
    /// `channel=stress` -- repaint the sheet with `load::evaluate`'s stress
    /// ratio, the same green-to-red ramp the app draws on `N`.
    ///
    /// Not one of `render.rs`'s overlays because it is not a stored channel:
    /// it is the *output* of the model under test, and it has to be computed
    /// here. It exists because the load-concentration defect is literally
    /// visible -- a one-pixel red line down an otherwise green wall -- and
    /// arguing about it from a mass probe takes an afternoon that one glance
    /// settles.
    ///
    /// **A full replace, not a blend**, per `CLAUDE.md`: the app blends at
    /// 0.55 into the cell's own colour, which is fine on a live screen you
    /// can toggle and unreadable at the zoom a contact sheet is read at --
    /// grey stone under a 45% green wash is grey stone.
    ///
    /// And **"not evaluated" gets its own colour** (dark blue) rather than
    /// falling through to the material. It is a third state, not a low
    /// stress, and conflating it with green is exactly how a model that
    /// never looks at 15 cells of a 17-cell wall reads as "the wall is
    /// fine".
    stress: bool,
    /// `channel=exposure` -- repaint the sheet with `weather::exposure`, the
    /// terrain-derived wind shelter every consumer of it will sample.
    ///
    /// Not one of `render.rs`'s overlays, for `stress`'s reason one step
    /// further: exposure is not a stored channel *and never will be*. It is
    /// a pure read over terrain, deliberately holding no per-tile state,
    /// because the version of this that kept state in the field was
    /// measured at a permanent 3.55 ms on every scene and reverted (see
    /// `weather::exposure`'s own doc). There is nothing to overlay; it has
    /// to be computed here.
    ///
    /// **A full replace on a fixed dark-to-bright ramp**, per `CLAUDE.md`,
    /// and not a blend into the cell's own colour: a magnitude-scaled blend
    /// was tried here once and produced a canopy-density sheet that read as
    /// blank, which would have sent a fix at working code.
    exposure: bool,
    /// `wind=` -- which way the wind is blowing for `channel=exposure`.
    ///
    /// Exposure is a function of terrain, position *and* wind direction:
    /// the lee of a ridge is the sheltered side only for as long as the
    /// wind holds. Defaults to what `weather::at` actually says at this
    /// world's seed and frame, so the sheet shows the real field rather
    /// than a hypothetical one, and the value used is printed under the
    /// tile either way.
    wind: Option<f32>,
    /// Write an animated GIF of every frame in the range instead of a grid.
    /// The grid is for *me* to read; motion is for a human to watch, and
    /// some of these artifacts only read correctly in motion.
    gif: bool,
    /// `explode=x,y,radius,strength,frame` -- fire one `explosion::trigger`
    /// at the given frame. Repeatable, for several blasts in one run.
    explosions: Vec<(i32, i32, i32, f32, usize)>,
    /// `blast=x,depth,radius,strength,frame` -- the same charge, placed
    /// `depth` cells below the **local solid surface** at column `x`.
    /// Repeatable. Held as `(x, depth, radius, strength, frame)`; the `y`
    /// is not known until it fires.
    ///
    /// **Why this exists next to `explode=` rather than replacing it.** A
    /// fixed `y` is a different situation on every seed -- open sky on one,
    /// bedrock on the next -- so a seed sweep over absolute coordinates
    /// measures the terrain and not the change. `CLAUDE.md`: a guard over a
    /// procedural system has to sweep the procedure. One `blast=` list is
    /// valid on every seed; one `explode=` list is valid on exactly one.
    ///
    /// Resolved inside `fire_due_explosions`, at the frame it fires, not at
    /// parse time -- so a later charge fired into the crater an earlier one
    /// left sees the crater rather than the surface that used to be there.
    ///
    /// `depth` is signed: negative is an airburst above the surface,
    /// `0` is the surface cell itself, positive is into the ground.
    blasts: Vec<(i32, i32, i32, f32, usize)>,
    /// `panels=W,H,age1[,age2,...]` -- write a **second** contact sheet:
    /// one column per charge fired, one row per age, each cell a `W`x`H`
    /// crop centred on that charge's own resolved site and captured
    /// `age` frames after **that charge's own detonation**.
    ///
    /// Per-charge age rather than absolute frame, and that is the whole
    /// point of the sheet: nine charges fired at nine different frames and
    /// sampled at one absolute frame are nine blasts at nine different
    /// points in their life, which is exactly the confound that makes "are
    /// these nine outcomes different?" unanswerable. Anchoring each column
    /// on its own bang makes a row a controlled comparison.
    ///
    /// Nothing is labelled on the image on purpose. The counters are on
    /// stdout; a label baked into a PNG is a second source of truth that
    /// goes stale.
    panels: Option<(i32, i32, Vec<usize>)>,
    /// `cut=x,y,w,h,frame` -- erase a rectangle at the given frame.
    ///
    /// **A surgical alternative to `explode`, and it exists because the
    /// blast was the wrong instrument for the question.** Asking "does a
    /// damaged tree respond" with `explode=224,175,6,2.0` vaporised the
    /// whole tree: a trunk here is two or three cells wide, so any blast
    /// big enough to reach across it is big enough to remove everything
    /// nearby, and what the sheet showed was a debris pile rather than a
    /// severed stem. A rectangle removes exactly what is named, delivers no
    /// impulse and no heat, and leaves the rest of the plant untouched --
    /// which is what isolates the *physiological* response from the
    /// mechanical one.
    cuts: Vec<(i32, i32, i32, i32, usize)>,
    /// `chop=x,y,radius,force,frame` -- swing `rigid::strike` there, the
    /// way the player's `C` key does. Repeatable.
    ///
    /// **The verb, where `cut=` above is the control.** A rectangle erases;
    /// a blow delivers a bite, a loosened ring, cracks, a pressure impulse
    /// and a recorded disturbance, and it is the only one of the two a
    /// player can actually perform. Keeping both is the paired comparison
    /// felling needs: if the crown comes down under `cut=` and not under
    /// `chop=`, the support model is fine and the verb is not reaching the
    /// tree -- which was exactly the state of things before D2.
    ///
    /// `radius` is the brush radius the blow is scaled off (see `strike`'s
    /// own doc on why it has a floor) and `force` is the impulse handed to
    /// the fracturer. `chop=256,192,4,6.0,6000` is an axe-sized bite at the
    /// foot of `scene=fell`'s trunk.
    chops: Vec<(i32, i32, i32, f32, usize)>,
    /// `fell=frame[,radius[,force]]` -- at that frame, chop through the
    /// subject's **own** thinnest bole row, wherever it currently is.
    ///
    /// `chop=` above needs three coordinates typed against a particular
    /// tree at a particular age, and a tree is not a fixed shape: the same
    /// individual's cheapest cut moved from `x 255..280` at frame 6,000 to
    /// `x 263..280` after three blows, and a different species or a longer
    /// run moves it further. A felling harness whose aim has to be
    /// re-derived by hand every time it is used is one that will quietly be
    /// used with a stale aim -- which produces a sheet of an untouched tree
    /// and reads as "the mechanism does nothing".
    ///
    /// So this asks `FellCensus` where the bole is and walks the blow
    /// across it, one bite per `radius`. Seed- and age-independent by
    /// construction, and the knob lane P's resprout work (D4/P5) wants: a
    /// cut it can fire at frame 10,000 without knowing the shape of what it
    /// is cutting.
    fell: Option<(usize, i32, f32)>,
    /// `depowder=frame` -- erase every `Powder`-kind cell in the world at
    /// the given frame, and say how many.
    ///
    /// **The paired control for the powder surcharge**, and it exists
    /// because neither `cut` nor `explode` can express it. The question R4
    /// has to answer is "did the roof come down because the muck on top of
    /// it now weighs something, or because the blast quietly took the
    /// shell's capacity away" -- and the only way to separate those is to
    /// run the identical blast and then take the muck away, leaving the
    /// cracked, detached shell exactly as the blast left it. `cut` erases a
    /// rectangle including the rock, which changes the thing under test;
    /// this erases only what is loose.
    depowder: Option<usize>,
    /// `poke=x,y,frame` -- schedule a structural check on that cell and its
    /// four neighbours at the given frame, changing nothing else.
    /// Repeatable.
    ///
    /// **The control for "was this cell ever asked?".** A cell the load
    /// model calls unsupported and that is nonetheless still standing has
    /// exactly two explanations -- no check was ever scheduled on it, or
    /// one was and something downstream declined -- and no readout can tell
    /// them apart, because both look like a standing cell with an
    /// UNSUPPORTED verdict beside it. Poking it schedules the check that
    /// may be missing and touches nothing else: if the cell then falls, it
    /// was never asked; if it stays, it was asked and refused, and the
    /// refusal is the bug. Distinct from `cut=` on purpose -- a cut removes
    /// material, which changes the very support question being asked.
    pokes: Vec<(i32, i32, usize)>,
    /// `load=x,y` -- print `sim::load::evaluate` at that cell for every
    /// tile. Repeatable. The structural counterpart of the `bodies` line:
    /// an image says a shelf is still up, and only a number says whether it
    /// is up with 3% of its capacity used or 97%.
    probes: Vec<(i32, i32)>,
    /// `repeat=N` -- run the whole scene N times and report the **minimum**
    /// worst-frame with the spread beside it, rather than one sample.
    ///
    /// This machine is contended enough that a single sample is not a
    /// measurement. Observed within one session: 18.0 ms twice running on
    /// `scene=terrain`, which schedules zero structural checks and cannot
    /// be doing the work that number would imply, and 40.5 / 55.6 ms on
    /// scenes that measured 14-19 ms moments later. Three separate
    /// near-misses where contention was almost read as a regression.
    ///
    /// The minimum is the right statistic, not the mean: contention can
    /// only ever make a frame *slower*, so the fastest observed run is the
    /// closest thing to the machine's actual cost. The spread is printed
    /// beside it so a sample that is all noise is visible as such.
    repeat: usize,
    /// `min_failing_cells=N` -- exit non-zero unless the run's structural
    /// failures took at least N cells between them.
    ///
    /// **The continuous counterpart of `min_overloaded`, and it exists
    /// because the count measured the wrong thing.** `roomcut`'s bar was
    /// "at least 5 overload events", which is a *granularity* reading
    /// dressed up as a collapse reading: making pieces bigger takes the
    /// same room apart in fewer, larger events and trips it. Measured
    /// across the quench-crust change, on the identical cut: 11 events
    /// carrying 2,398 cells became 4 events carrying 2,197, with total
    /// failing cells 2,742 against 2,713 -- the same room coming down, in
    /// coarser pieces, and the event count halved twice over.
    ///
    /// `CLAUDE.md`: "prefer a continuous quantity over a count of bad
    /// cells -- counts give knife-edge margins; sums separate cleanly."
    min_failing_cells: Option<u32>,
    /// `min_severed=N` -- exit non-zero unless the plant-support check
    /// broke at least N organism cells free
    /// (`FailureCounts::severed_organism_cells`).
    ///
    /// **Felling's own bar, and it has to be its own counter.** Nothing
    /// else in `FailureCounts` moves when a crown comes down: the
    /// organism path records neither `overloaded` nor `unsupported`, and
    /// `min_failing_cells` therefore reads zero through a run that
    /// dismantled an entire tree. It is a *cell* count rather than an
    /// event count for `min_failing_cells`'s own recorded reason -- counts
    /// give knife-edge margins, sums separate cleanly.
    min_severed: Option<u32>,
    /// `min_overloaded=N` / `max_failures=N` -- exit non-zero unless the
    /// run produced at least / at most that many structural failures. See
    /// `check_expectations`.
    min_overloaded: Option<u32>,
    max_failures: Option<u32>,
    /// `max_unconfined=N` -- like `max_failures`, but **confined crushes
    /// don't count**: the bound is on unsupported failures plus overloads
    /// whose region had a free face. For a scene asserting "nothing comes
    /// apart" over material that can legitimately crack in place --
    /// `scene=coldsnap`'s pond, which a hard front freezes solid to the
    /// basin floor now that stone conducts, whose confined interior then
    /// crush-fissures and thaws back to clean water -- `max_failures=0`
    /// gates the wrong mechanism: a fissure is not a collapse. Dismantling
    /// (the thing such a scene is actually about) still lands in this
    /// count, because a dismantled region by definition has a free face.
    max_unconfined: Option<u32>,
    /// `max_frame_ms=N` -- exit non-zero if the **minimum** worst-frame
    /// across `repeat` runs exceeds N.
    ///
    /// Checked against the minimum specifically, and that is what makes it
    /// safe to gate CI on. A single sample from a contended machine is not
    /// a measurement -- this session saw 18.0 ms twice running on a scene
    /// that schedules no structural work at all -- so a bar checked against
    /// one run, or against a mean, would be permanently flaky and would
    /// train everyone to ignore it. Contention can only make a frame
    /// slower, so the fastest of several runs is the closest thing to the
    /// machine's real cost, and a bar it still fails is a real regression.
    max_frame_ms: Option<f64>,
    /// `max_sites=N` -- exit non-zero if the structural scheduler still has
    /// more than N sites pending when the run ends.
    ///
    /// **The one counter that states `open-bugs-handoff.md` §S directly**,
    /// and the reason it is a *final* count rather than a peak. §S is
    /// "every destructive verb leaves the structural scheduler pinned at
    /// its cap for ever", so the refutation is not that the backlog stays
    /// small -- a real blow should spike it -- but that it **drains**.
    /// Measured on `scene=strike` 2026-08-27, sites at frames 2/62/122/182:
    /// **958, 968, 824, 289** with the shipped ground root and **958, 2747,
    /// 5034, 7145** with the pre-2026-08-27 flat `0`. One drains, one
    /// climbs monotonically, and only the final reading separates them by
    /// more than a factor of three.
    ///
    /// A counter rather than a wall clock, deliberately -- `CLAUDE.md`
    /// gates on counters because a bar over a clock is a flake generator,
    /// and the two cases in this file that do gate on `max_frame_ms` have
    /// flaked on a loaded box within this session.
    max_sites: Option<usize>,
    /// `min_bodies=N` -- exit non-zero unless at least N coherent chunk
    /// bodies were in flight at once at some point in the run.
    ///
    /// A different question from `min_overloaded`, and the `strike` scene
    /// is why both exist. That scene is about a *blow throwing pieces*,
    /// and the mechanism there is `rigid::strike`'s own fracture, not the
    /// load criterion -- so asserting overload failures tested something
    /// the scene is not about, and duly broke when an unrelated change to
    /// the fragment ladder shifted how many separate events the same
    /// material came away in. Peak concurrent bodies is the quantity that
    /// actually says "it threw pieces".
    min_bodies: Option<usize>,

    /// `ice=minCols,maxCells` -- exit non-zero unless at least `minCols` of
    /// the pond's columns are frozen at the surface **and** no more than
    /// `maxCells` of it is ice in total.
    ///
    /// **Both halves, because each alone passes the artifact the other
    /// catches.** Reported from play: *"it never really freezes"* -- which
    /// a coverage floor answers -- and the fix for it, left unbounded,
    /// turns a pond into a solid block, which only a ceiling on the ice
    /// itself sees. A cell count alone cannot tell a closed sheet from a
    /// churning slush of the same mass, and a coverage figure alone cannot
    /// tell a sheet from a pond frozen to its bed.
    ///
    /// Lives here rather than in a unit test because the constant it
    /// guards, `weather::SHEET_MAX_THICKNESS`, is invisible in a dry
    /// fixture: it limits how far the front's sweep reaches *through* what
    /// is already frozen, and it is a lying drift that spends that budget.
    /// Measured with the cap removed, this scene goes 570 cells of ice to
    /// **823**, and the water under it 507 to 158.
    ice: Option<(usize, i64)>,
    /// `min_travelled=N` -- exit non-zero unless the scripted gnome covered
    /// at least N cells after setting off.
    ///
    /// The gnome path had no gated case at all before this, which is how a
    /// character who could be walled in by a *tree* went unnoticed. Distance
    /// is the right quantity for the same reason `min_overloaded` is the
    /// right one for a collapse: "he is standing near a trunk" and "he is
    /// standing *in* one" are the same picture.
    min_travelled: Option<i32>,
    /// `loadmap=1` -- also report the single most-stressed cell in the
    /// world per tile. `CLAUDE.md`: sanity-check a new metric against a
    /// case you know is fine before trusting it about one you don't, and
    /// "nothing anywhere is over 1.0" on a scene that visibly stands is
    /// exactly that check.
    loadmap: bool,
    /// `min_cave=P` -- exit non-zero unless at least P percent of the
    /// roofed void present at the cut is still there at the end.
    ///
    /// The gate for "a cave can be dug and it does not collapse", which is
    /// the owner's own statement of what this has to do. A fraction rather
    /// than an absolute so one bar covers every bore size and length.
    min_cave: Option<i64>,
    /// `max_cave=P` -- exit non-zero unless the roofed void is down to at
    /// most P percent of what was there at the cut.
    ///
    /// **The other half of `min_cave`, and the pair is the point**: "a cave
    /// can be dug and it does not collapse" passes trivially by making rock
    /// invincible, which is how four earlier support models died. `min_cave`
    /// guards the deep bore; this guards the shallow one, which has to come
    /// down.
    ///
    /// It replaces `min_overloaded=50` on that case, and the reason is the
    /// same one that moved `roomcut` to `min_failing_cells` earlier the same
    /// session: an event count reads *which failure mode fired*, not whether
    /// the cave fell in. Measured across the grain-footing change, on the
    /// identical bore -- overload failures 65 (3,918 cells) became 7 (169),
    /// while the roofed void went 678 -> 69 before and 678 -> **64** after.
    /// The roof came down slightly *harder* and the bar called it a
    /// regression, because the failures had moved from `Overloaded` to
    /// `Unsupported`. The case's own comment already said the pair should be
    /// gated on roofed void; only this half was not.
    max_cave: Option<i64>,
    /// `max_rock_above=Y,N` -- exit non-zero unless at most N `Solid` cells
    /// are left strictly above row Y at the end of the run.
    ///
    /// **"Where did the rock end up", which is the only question a falling
    /// scene is really about, and the one no existing bar asked.** Written
    /// for `scene=rockdrop` after the owner reported "the boulder just
    /// stops and gets stuck in the middle of the water" and the boulder
    /// turned out never to have left the sky at all -- 522 of its 600 cells
    /// still airborne at frame 400, with **every one of the seventeen
    /// acceptance cases green**, because not one of them drops anything
    /// into water.
    ///
    /// Deliberately not an event count and not a census of the model's own
    /// verdict. `hanging:` and `afloat:` both read **zero** through that
    /// whole bug, correctly by their own definitions: the load model
    /// believed the slab was supported, so a readout that asks the model
    /// cannot see it. A row and a count ask the world.
    max_rock_above: Option<(i32, usize)>,
    /// `step=N` -- how far apart consecutive `tunnel=` bites are placed.
    /// Defaults to `dig`, i.e. each bite overlaps the last by half.
    ///
    /// It used to be `dig * 2 + 1`, and that was **not a tunnel**. A disc
    /// of radius r is `2r + 1` across on its centre line and narrower
    /// everywhere else, so centres spaced `2r + 1` apart leave solid rock
    /// standing between every pair of bites. Dumped, four bites came out
    /// as four separate chambers joined only near the floor, with 2-4 cell
    /// pillars between them -- a string of beads. It looked like a row of
    /// circles because it *was* a row of circles, and every measurement of
    /// "does a tunnel hold" was really measuring a row of small caverns
    /// separated by thin pillars, which is about the least representative
    /// geometry available.
    step: Option<i32>,
    /// `depth=N` -- how far below the surface a `worldcrack` tunnel is
    /// driven, in cells. Defaults to `dig * 3`.
    ///
    /// It exists because the default **couples two variables**, and that
    /// invalidated a measurement: comparing bore sizes at equal tunnel
    /// length showed a 13-cell gallery holding (14 cells of rock lost)
    /// where a 5-cell ant tunnel collapsed (1,105) -- the exact inverse of
    /// what the owner expects -- but the big bore was also being driven
    /// three times deeper, under three times the roof. A knob that moves
    /// something else along with it is not a knob. Set this to hold depth
    /// fixed while bore varies, which is what requirement 2 (collapse
    /// depending on height, not only span) has to be measured against.
    depth: Option<i32>,
    /// `dump=x,y,w,h` -- print the materials in that rectangle as ASCII,
    /// once per captured tile.
    ///
    /// For questions a contact sheet cannot answer however far it is
    /// zoomed. "Why will the gnome not walk into his own tunnel" turned
    /// into three wrong guesses about the geometry of the mouth read off a
    /// 20x magnification -- whether the floor was a step up or a drop,
    /// whether a lip of rock survived at the threshold -- when what was
    /// needed was the cells themselves. `.` is air, `#` solid, `o` powder,
    /// `~` liquid, `P` where the player is standing.
    dump: Option<Rect>,
    /// `max_lost=N` -- exit non-zero if the world ended with more than N
    /// cells fewer than it started with.
    ///
    /// **A failure count is not a damage count**, and this is the number
    /// that closes that gap. `FailureCounts` counts cells that *failed*,
    /// and a failed cell that became rubble is still standing there --
    /// `CLAUDE.md` records two digs whose event counts looked comparable
    /// removing 894 and 23,042 cells, with nothing in the engine able to
    /// tell them apart. This is the census it asks for: how much material
    /// the world actually holds, before and after.
    ///
    /// The baseline is taken **after** the scene is built, so a scene's
    /// own cut is not counted: this measures what the *simulation*
    /// subsequently ate, which is the quantity that separates 894 from
    /// 23,042. Deliberately so, and it is what makes the bar safe to gate
    /// on -- the bite is a constant that moves whenever `yield` is
    /// retuned, and a guard that moved with it would need re-baselining on
    /// every legitimate spoil change while staying just as blind to a
    /// cascade. A gnome scene is the exception by construction: he digs
    /// during the run, so his bites do count, and `world holds` on his own
    /// report line is the number to read there.
    max_lost: Option<i64>,
    /// `confine=0` -- turn off `World::crush_confined`, so a failing
    /// region displaces whether or not it has anywhere to go.
    ///
    /// The control that isolates the rule. A sweep only varies its knob,
    /// and anything that landed *with* a mechanism is constant across
    /// every data point -- which has already read as "the approach is
    /// wrong" here once when a rider was the whole effect. Running the
    /// same binary with the rule off is what makes a before/after a
    /// measurement rather than a memory of an earlier build.
    confine: bool,
    /// `arch=0` -- turn off `World::arch_relief`, so a roof carries the
    /// whole column above it again. The control for the arching change.
    arch: bool,
    /// `share=0` -- turn off `World::section_share`, so each cell is judged
    /// on its own load path again. The control for the load-concentration
    /// change, and the only way to see the one-pixel red line come back.
    share: bool,
    /// `chain_reach=N` -- how far from something actually disturbed a
    /// failure may happen, in cells. Also takes a `CHAIN_MODES` name --
    /// `tight`, `local`, `spread`, `none` -- which is the spelling to
    /// prefer, since the numbers move when the modes are retuned.
    ///
    /// **Unset means the shipped default**, which is `TIGHT` since the
    /// playtest and was "no limit" before it. That change matters to any
    /// scene asserting *nothing fails*: TIGHT only licenses a failure near
    /// something that reported itself disturbed, so a hand-placed scene
    /// that no verb touched passes on the leash rather than on the load
    /// model -- the vacuous guard `CLAUDE.md` warns about.
    /// `scripts/acceptance.sh` passes `chain_reach=spread` on exactly
    /// those cases and says so at each one.
    chain_reach: Option<i32>,
    /// `joints=<spacing>` -- override stone's `Material::joint_spacing`, the
    /// pitch of the joint fabric (`sim::fracture_field`), in cells.
    ///
    /// It exists because the alternative is a rebuild per sweep point: the
    /// `.ron` files are `include_str!`d into the binary, so editing
    /// `stone.ron` and re-running a prebuilt harness produces bit-identical
    /// "runs" -- which `CLAUDE.md` records as having produced three of them
    /// before anyone noticed the knob was not connected. With this, the
    /// density A/B is one binary and one flag, and *differing* output across
    /// settings is itself the proof the knob reaches the mechanism.
    ///
    /// `0` turns jointing off entirely, which is the control.
    joint_spacing: Option<f32>,
    /// `bands=<contrast>` -- override stone's
    /// `Material::joint_band_contrast`: how far the grain varies from place
    /// to place. `0` (the shipped default) is a uniform grain everywhere.
    ///
    /// The A/B for the owner's *"could the pattern of cracks be more
    /// heterogeneous"*, and it is a knob rather than a decision because the
    /// trade is a judgement: `0.4` gives a visibly varied web, narrows the
    /// promoted-cell spread (best case down a quarter, worst case up 7%),
    /// and costs 10-14 ms on the worst frame. See
    /// `sim::fracture_field::pitch_at` for the four-seed table.
    joint_bands: Option<f32>,
    /// `jreach=`, `jopen=`, `jdensity=` -- the three `explosion::Tuning`
    /// knobs of the same mechanism, for the same no-rebuild reason.
    joint_reach: Option<f32>,
    joint_open: Option<f32>,
    joint_density: Option<f32>,
    /// `charge=<cap|powder|dynamite|mining|demolition>` -- load a whole
    /// `explosion::Preset` before any of the individual overrides below are
    /// applied, so a sheet can compare the five charge *types* rather than
    /// five settings of one. `blast=` with a radius or strength of `0` then
    /// takes the charge's own, which is the only way to see a preset as
    /// itself: a `CAP` fired at `DEMOLITION`'s radius is neither.
    charge: Option<String>,
    /// `jwidth=` -- `explosion::Tuning::joint_seam_width`, the cap on the
    /// seam-aperture ladder. `1` is the uniform one-cell seam that shipped
    /// before the ladder, and is the control any before/after over the crack
    /// pattern's *weight* has to be run against.
    joint_seam_width: Option<u32>,
    /// `crack_rays=` -- the hybrid knob. `0` (the default) is pure fabric;
    /// 4-6 puts the old radial walker back on top of it for an A/B.
    crack_rays: Option<u32>,
    /// `smoke=<fraction>` -- override `explosion::Tuning::smoke_fraction`,
    /// how much of a cleared crater is backfilled with `SMOKE`.
    ///
    /// **`smoke=0` is the control for "do chunks fall into the hole".**
    /// `rigid::clear_or_displaceable` shoves `Powder` and `Liquid` aside and
    /// treats everything else as a real obstruction, `Gas` included -- so at
    /// the shipped `0.18` a promoted chunk falling into a fresh crater can be
    /// stopped dead by the blast's own smoke, stall for
    /// `STALL_FRAMES_BEFORE_SETTLING` frames and re-embed roughly where it
    /// started. Running the same charge at `0` and at the default is the
    /// paired comparison that says whether that is actually happening, and
    /// it costs one flag rather than a rebuild.
    smoke: Option<f32>,

}

fn parse() -> Args {
    let mut a = Args {
        scene: "pour".into(),
        dig_yield: pixel_physics::sim::player::Tuning::default().dig_yield,
        shoulder_grains: pixel_physics::sim::player::Tuning::default().shoulder_grains,
        dig_style: pixel_physics::sim::player::DigStyle::default(),
        seed: 1,
        species: "tree".into(),
        soil_moisture: pixel_physics::sim::material::SOIL_FIELD_CAPACITY,
        frame0: 0,
        // 0 means "leave `PlantScene`'s own default alone".
        plants: 0,
        soil_depth: common::SOIL_DEPTH,
        ignitions: Vec::new(),
        dries: Vec::new(),
        preset: String::new(),
        start: 100,
        every: 60,
        count: 6,
        cols: 3,
        zoom: 1,
        genome: String::from("authored"),
        impulse: HOP_IMPULSE_WEIGHT,
        hop_body: String::new(),
        crop: Rect::new(0, 0, WIDTH - 1, HEIGHT - 1),
        parallel_driver: true,
        out: std::env::temp_dir().join("filmstrip.png").display().to_string(),
        grain: GrainMode::Position,
        bubbles: BubbleMode::default(),
        gas: GasMode::default(),
        tree_depth: TreeDepth::default(),
        organism_overlay: OrganismOverlay::Off,
        field_overlay: FieldOverlay::Off,
        sky_light: SkyLight::default(),
        daylight: None,
        stress: false,
        exposure: false,
        wind: None,
        gif: false,
        explosions: Vec::new(),
        blasts: Vec::new(),
        panels: None,
        cuts: Vec::new(),
        chops: Vec::new(),
        fell: None,
        depowder: None,
        pokes: Vec::new(),
        probes: Vec::new(),
        loadmap: false,
        repeat: 1,
        min_failing_cells: None,
        min_severed: None,
        min_overloaded: None,
        max_unconfined: None,
        max_failures: None,
        max_frame_ms: None,
        max_sites: None,
        min_bodies: None,
        ice: None,
        min_travelled: None,
        max_lost: None,
        dump: None,
        depth: None,
        step: None,
        min_cave: None,
        max_cave: None,
        max_rock_above: None,
        confine: true,
        arch: true,
        share: true,
        chain_reach: None,
        joint_spacing: None,
        joint_bands: None,
        joint_reach: None,
        joint_open: None,
        joint_density: None,
        joint_seam_width: None,
        charge: None,
        crack_rays: None,
        smoke: None,
        wall: 3,
        dig: 3,
        strike: 0,
        tunnel: 0,
        relax: false,
        span: 200,
        phase: None,
        // A pond a player would recognise as a pond, and the width the
        // acceptance case in ice.ron's note is stated at.
        fall: 90,
        pond: 60,
        clock: pixel_physics::sim::clock::Clock::default(),
    };
    let mut named_gif = false;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "scene" => a.scene = v.into(),
            "seed" => a.seed = v.parse().expect("seed"),
            "yield" => a.dig_yield = v.parse().expect("yield"),
            "shoulder" => a.shoulder_grains = v.parse().expect("shoulder"),
            // `digstyle=`, because **both shorter names were already
            // taken**: `dig=` is `scene=room`'s cut radius and `cut=` is a
            // crop rectangle. `dig=` was tried first, and
            // `scene=room`'s cut radius, and adding a second arm for it
            // shadowed that one: `acceptance.sh` passes `dig=0` and `dig=4`,
            // which then panicked with "known: bore, free" and took **ten
            // acceptance cases** down with it. The compiler said so —
            // `unreachable_patterns` on the older arm — and a background
            // `--release --examples` whose tail was grepped rather than read
            // is how the warning went by. `cut=` also matches the HUD's own
            // wording for the key that swaps it.
            "digstyle" => {
                a.dig_style = match v {
                    "bore" => pixel_physics::sim::player::DigStyle::Bore,
                    "free" => pixel_physics::sim::player::DigStyle::Free,
                    other => panic!("digstyle={other:?}; known: bore, free"),
                }
            }
            "species" => a.species = v.into(),
            "moisture" => a.soil_moisture = v.parse().expect("moisture"),
            "frame0" => a.frame0 = v.parse().expect("frame0"),
            "plants" => a.plants = v.parse().expect("plants"),
            "soil" => a.soil_depth = v.parse().expect("soil=ROWS"),
            "ignite" => {
                let n: Vec<i64> = v.split(',').map(|s| s.parse().expect("ignite")).collect();
                assert_eq!(n.len(), 4, "ignite=x,y,radius,frame");
                a.ignitions.push((n[0] as i32, n[1] as i32, n[2] as i32, n[3] as usize));
            }
            "dry" => {
                let n: Vec<i64> = v.split(',').map(|s| s.parse().expect("dry")).collect();
                assert_eq!(n.len(), 2, "dry=aux,frame");
                a.dries.push((n[0] as u16, n[1] as usize));
            }
            "preset" => a.preset = v.into(),
            "start" => a.start = v.parse().expect("start"),
            "every" => a.every = v.parse().expect("every"),
            "count" => a.count = v.parse().expect("count"),
            "phase" => a.phase = Some(v == "noon"),
            "cols" => a.cols = v.parse().expect("cols"),
            "zoom" => a.zoom = v.parse().expect("zoom"),
            "genome" => a.genome = v.to_string(),
            "impulse" => a.impulse = v.parse().expect("impulse=WEIGHT"),
            "body" => a.hop_body = v.to_string(),
            "driver" => a.parallel_driver = v != "serial",
            "out" => {
                named_gif = v.ends_with(".gif");
                a.out = v.into();
            }
            "gif" => a.gif = v != "false",
            // `skylight=off|4|2|1` -- the `F12` selector, by block
            // size, which is the only thing that differs between the
            // propagated modes.
            "skylight" => {
                a.sky_light = match v {
                    "off" | "depth" => SkyLight::Depth,
                    "4" => SkyLight::Coarse4,
                    "2" => SkyLight::Coarse2,
                    "1" | "exact" => SkyLight::Exact,
                    other => panic!("unknown skylight {other:?} (off|4|2|1)"),
                }
            }
            "grain" => {
                a.grain = match v {
                    "position" => GrainMode::Position,
                    "cell" => GrainMode::Cell,
                    "muted" => GrainMode::Muted,
                    "animated" => GrainMode::Animated,
                    "motion" => GrainMode::Motion,
                    other => panic!("unknown grain {other:?}"),
                }
            }
            "gas" => {
                a.gas = match v {
                    "opaque" => GasMode::Opaque,
                    "translucent" => GasMode::Translucent,
                    "byfill" => GasMode::ByFill,
                    other => panic!("unknown gas {other:?}"),
                }
            }
            "bubbles" => {
                a.bubbles = match v {
                    "off" => BubbleMode::Off,
                    "rising" => BubbleMode::Rising,
                    "large" => BubbleMode::Large,
                    "columns" => BubbleMode::Columns,
                    "surface" => BubbleMode::Surface,
                    other => panic!("unknown bubbles {other:?}"),
                }
            }
            "trees" => {
                a.tree_depth = match v {
                    "weave" => TreeDepth::Weave,
                    "haze" => TreeDepth::Haze,
                    "front" => TreeDepth::Front,
                    "behind" => TreeDepth::Behind,
                    other => panic!("unknown trees {other:?}"),
                }
            }
            // One flag for both overlay families, resolved by name: they are
            // one question ("which channel am I looking at") from the
            // caller's side, and keeping them as two arguments would invite
            // setting both and getting a sheet that is hard to attribute.
            "channel" => match v {
                "off" => {}
                "celltype" => a.organism_overlay = OrganismOverlay::CellType,
                "resource" => a.organism_overlay = OrganismOverlay::Resource,
                "canopy" => a.organism_overlay = OrganismOverlay::CanopyDensity,
                "vein" => a.organism_overlay = OrganismOverlay::VeinConductance,
                "soil" => a.organism_overlay = OrganismOverlay::SoilMoisture,
                // S3 built `OrganismOverlay::FoodValue` and specified this
                // switch alongside it; only the render half landed, so the
                // one readout that can answer "where is the food" was
                // unreachable from the harness that judges by eye.
                "foodvalue" => a.organism_overlay = OrganismOverlay::FoodValue,
                // Landed *with* the overlay this time, not a stage later:
                // the readout above was unreachable from here for a whole
                // milestone because only the render half shipped, and an
                // unknown `channel=` value is silently ignored rather than
                // rejected -- so a sheet rendered with a typo'd channel
                // looks exactly like a mechanism that does nothing.
                "gutbias" => a.organism_overlay = OrganismOverlay::GutBias,
                "light" => a.field_overlay = FieldOverlay::Light,
                "moisture" => a.field_overlay = FieldOverlay::Moisture,
                "temperature" => a.field_overlay = FieldOverlay::Temperature,
                "pressure" => a.field_overlay = FieldOverlay::Pressure,
                "pheromone_a" => a.field_overlay = FieldOverlay::PheromoneA,
                "pheromone_b" => a.field_overlay = FieldOverlay::PheromoneB,
                "stress" => a.stress = true,
                "exposure" => a.exposure = true,
                other => panic!(
                    "unknown channel {other:?}; known: off, celltype, resource, canopy, vein, soil, foodvalue, light, moisture, temperature, pressure, pheromone_a, pheromone_b, stress, exposure"
                ),
            },
            "wind" => {
                let f: f32 = v.parse().expect("wind=<-1.0..1.0>");
                assert!((-1.0..=1.0).contains(&f), "wind=<-1.0..1.0>, got {f}");
                a.wind = Some(f);
            }
            "daylight" => {
                let f: f32 = v.parse().expect("daylight=<0.0..1.0>");
                assert!((0.0..=1.0).contains(&f), "daylight=<0.0..1.0>, got {f}");
                a.daylight = Some(f);
            }
            "repeat" => a.repeat = v.parse::<usize>().expect("repeat").max(1),
            "wall" => a.wall = v.parse().expect("wall"),
            "dig" => a.dig = v.parse().expect("dig"),
            "strike" => a.strike = v.parse().expect("strike"),
            "tunnel" => a.tunnel = v.parse().expect("tunnel"),
            "relax" => a.relax = v != "false",
            "span" => a.span = v.parse().expect("span"),
            "fall" => a.fall = v.parse().expect("fall"),
            "pond" => a.pond = v.parse().expect("pond"),
            "min_failing_cells" => a.min_failing_cells = Some(v.parse().expect("min_failing_cells")),
            "min_severed" => a.min_severed = Some(v.parse().expect("min_severed")),
            "min_overloaded" => a.min_overloaded = Some(v.parse().expect("min_overloaded")),
            "max_unconfined" => a.max_unconfined = Some(v.parse().expect("max_unconfined")),
            "max_failures" => a.max_failures = Some(v.parse().expect("max_failures")),
            "max_lost" => a.max_lost = Some(v.parse().expect("max_lost")),
            "depth" => a.depth = Some(v.parse().expect("depth")),
            "step" => a.step = Some(v.parse().expect("step")),
            "min_cave" => a.min_cave = Some(v.parse().expect("min_cave")),
            "max_cave" => a.max_cave = Some(v.parse().expect("max_cave")),
            "max_rock_above" => {
                let n: Vec<i64> = v.split(',').map(|s| s.parse().expect("max_rock_above")).collect();
                assert_eq!(n.len(), 2, "max_rock_above=row,count");
                a.max_rock_above = Some((n[0] as i32, n[1] as usize));
            }
            "dump" => {
                let n: Vec<i32> = v.split(',').map(|p| p.parse().expect("dump=x,y,w,h")).collect();
                assert_eq!(n.len(), 4, "dump=x,y,w,h");
                a.dump = Some(Rect::new(n[0], n[1], n[0] + n[2] - 1, n[1] + n[3] - 1));
            }
            "confine" => a.confine = v != "0" && v != "false",
            "arch" => a.arch = v != "0" && v != "false",
            "share" => a.share = v != "0" && v != "false",
            "chain_reach" => {
                a.chain_reach = Some(match v {
                    // Named, so a scene says which *mode* it wants rather
                    // than a number that moves when the modes are retuned.
                    "tight" | "local" | "spread" | "none" => {
                        pixel_physics::sim::structural::CHAIN_MODES
                            .iter()
                            .find(|m| m.name.eq_ignore_ascii_case(v))
                            .expect("chain_reach name must be a CHAIN_MODES entry")
                            .reach
                    }
                    _ => v.parse().expect("chain_reach"),
                })
            }
            "joints" => a.joint_spacing = Some(v.parse().expect("joints=<spacing in cells>")),
            "bands" => a.joint_bands = Some(v.parse().expect("bands=<grain contrast 0..0.9>")),
            "jreach" => a.joint_reach = Some(v.parse().expect("jreach")),
            "jopen" => a.joint_open = Some(v.parse().expect("jopen")),
            "jdensity" => a.joint_density = Some(v.parse().expect("jdensity")),
            "jwidth" => a.joint_seam_width = Some(v.parse().expect("jwidth=<max seam cells 1..4>")),
            "charge" => a.charge = Some(v.into()),
            "crack_rays" => a.crack_rays = Some(v.parse().expect("crack_rays")),
            "smoke" => a.smoke = Some(v.parse().expect("smoke=<fraction 0..1>")),
            "max_frame_ms" => a.max_frame_ms = Some(v.parse().expect("max_frame_ms")),
            "max_sites" => a.max_sites = Some(v.parse().expect("max_sites")),
            "min_bodies" => a.min_bodies = Some(v.parse().expect("min_bodies")),
            "ice" => {
                let (cols, cells) = v.split_once(',').expect("ice=minCols,maxCells");
                a.ice = Some((cols.parse().expect("ice minCols"), cells.parse().expect("ice maxCells")));
            }
            "min_travelled" => a.min_travelled = Some(v.parse().expect("min_travelled")),
            "loadmap" => a.loadmap = v != "false",
            "load" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("load")).collect();
                assert_eq!(n.len(), 2, "load=x,y");
                a.probes.push((n[0], n[1]));
            }
            "explode" => {
                let n: Vec<f32> = v.split(',').map(|s| s.parse().expect("explode")).collect();
                assert_eq!(n.len(), 5, "explode=x,y,radius,strength,frame");
                a.explosions.push((n[0] as i32, n[1] as i32, n[2] as i32, n[3], n[4] as usize));
            }
            // Parsed through `f32` like `explode=` above, which also gets
            // the sign of a negative `depth` (an airburst) for free.
            "blast" => {
                let n: Vec<f32> = v.split(',').map(|s| s.parse().expect("blast")).collect();
                assert_eq!(n.len(), 5, "blast=x,depth,radius,strength,frame");
                a.blasts.push((n[0] as i32, n[1] as i32, n[2] as i32, n[3], n[4] as usize));
            }
            "panels" => {
                let n: Vec<i64> = v.split(',').map(|s| s.parse().expect("panels")).collect();
                assert!(n.len() >= 3, "panels=W,H,age1[,age2,...]");
                assert!(n[0] > 0 && n[1] > 0, "panels=W,H,... needs a positive crop");
                a.panels = Some((n[0] as i32, n[1] as i32, n[2..].iter().map(|&f| f as usize).collect()));
            }
            "cut" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("cut")).collect();
                assert_eq!(n.len(), 5, "cut=x,y,w,h,frame");
                a.cuts.push((n[0], n[1], n[2], n[3], n[4] as usize));
            }
            "fell" => {
                let n: Vec<f32> = v.split(',').map(|s| s.parse().expect("fell")).collect();
                assert!((1..=3).contains(&n.len()), "fell=frame[,radius[,force]]");
                a.fell = Some((n[0] as usize, n.get(1).map_or(FELL_BITE_RADIUS, |&r| r as i32), n.get(2).copied().unwrap_or(FELL_BITE_FORCE)));
            }
            "chop" => {
                let n: Vec<f32> = v.split(',').map(|s| s.parse().expect("chop")).collect();
                assert_eq!(n.len(), 5, "chop=x,y,radius,force,frame");
                a.chops.push((n[0] as i32, n[1] as i32, n[2] as i32, n[3], n[4] as usize));
            }
            "depowder" => a.depowder = Some(v.parse().expect("depowder")),
            "poke" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("poke")).collect();
                assert_eq!(n.len(), 3, "poke=x,y,frame");
                a.pokes.push((n[0], n[1], n[2] as usize));
            }
            "crop" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("crop")).collect();
                assert_eq!(n.len(), 4, "crop=x,y,w,h");
                a.crop = Rect::new(n[0], n[1], n[0] + n[2] - 1, n[1] + n[3] - 1);
            }
            "day" | "weather" | "growth" | "creatures" | "gnome" => {
                let n: u32 = v.parse().unwrap_or_else(|_| panic!("{k}= wants a whole number, got {v:?}"));
                a.clock.set_rates(0, |c| match k {
                    "day" => c.day_minutes = n,
                    "weather" => c.weather_slowdown = n,
                    "growth" => c.growth_slowdown = n,
                    "creatures" => c.creature_slowdown = n,
                    _ => c.gnome_slowdown = n,
                });
                // A value out of range clamps silently, and a knob whose
                // stored value is not what was asked for is a knob nobody can
                // tell is disconnected -- the failure `Args::clock` records.
                //
                // Compares the *specific* knob rather than asking whether the
                // whole clock is still at baseline, which was the first
                // version and rejected `day=1` -- a perfectly legitimate way
                // to name the default explicitly, and exactly what a paired
                // comparison against a slowed day needs on its other arm.
                let stored = match k {
                    "day" => a.clock.day_minutes,
                    "weather" => a.clock.weather_slowdown,
                    "growth" => a.clock.growth_slowdown,
                    "creatures" => a.clock.creature_slowdown,
                    _ => a.clock.gnome_slowdown,
                };
                assert_eq!(
                    stored, n,
                    "{k}={v} was clamped to {stored}: the range is 1..={}",
                    pixel_physics::sim::clock::MAX_SLOWDOWN
                );
            }
            other => panic!("unknown argument {other:?}"),
        }
    }
    // **A file named `.gif` gets a GIF.**
    //
    // `gif` defaults off, so omitting it while asking for `out=clip.gif`
    // wrote a contact sheet -- a PNG -- into a file with a `.gif` name, and
    // nothing anywhere complained. It cost a review round: a 110-tile sheet
    // 1,264 by 13,392 went out as an animation and came back as *"this is
    // not an animated gif, it is a panel of a bunch of static images"*.
    //
    // Inferred rather than rejected, because the filename is an unambiguous
    // statement of intent and an error here would only be read after the
    // same mistake. Announced, so it is never a silent reinterpretation.
    if named_gif && !a.gif {
        println!("gif: out is named .gif, so writing an animation ({} frames at true speed)", a.count);
        a.gif = true;
    }
    a
}

/// Schedule structural checks at whatever positions `poke=` named, once
/// their frame arrives. See `Args::pokes` for what the experiment is.
///
/// A disturbance is recorded alongside, because `World::chain_reach` can
/// refuse a failure that is far from anything disturbed -- and a poke that
/// is silently vetoed by that rule would read as "asked and refused" when
/// it was really "asked with the licence withheld", which is the exact
/// confusion this arg exists to end.
fn fire_due_pokes(world: &mut World, pending: &mut Vec<(i32, i32, usize)>, now: usize) {
    let mut i = 0;
    while i < pending.len() {
        if pending[i].2 <= now {
            let (x, y, _) = pending.remove(i);
            // Extent 0, and that is the honest value rather than a
            // placeholder: a poke is a single cell, and `record_disturbance`
            // takes the extent precisely so a *volume* verb cannot quietly
            // record itself as a point. A point verb saying 0 is the arg
            // working as intended.
            world.record_disturbance(x, y, 0);
            world.schedule_structural_check_around(x, y);
            let cell = world.get(x, y);
            println!(
                "  poke: ({x}, {y}) at frame {now} -- {}, aux {}, attached {}",
                world.materials.get(cell.material).name,
                cell.aux(),
                cell.attached()
            );
        } else {
            i += 1;
        }
    }
}

/// Erase every scheduled cut whose frame has arrived, draining it so it
/// cannot fire twice -- same shape as `fire_due_explosions`, and called
/// from the same three places for the same reason.
///
/// Reports what it actually removed rather than what it was asked to
/// remove. "Did the cut land on the tree" is a counter question, not a
/// picture question: a rectangle a few cells off the trunk looks identical
/// on a contact sheet to one that severed it, and this branch has already
/// spent a session reading a collapse as a feature that had never once
/// executed.
fn fire_due_cuts(world: &mut World, pending: &mut Vec<(i32, i32, i32, i32, usize)>, now: usize) {
    let mut i = 0;
    while i < pending.len() {
        if pending[i].4 <= now {
            let (x, y, w, h, _) = pending.remove(i);
            let mut removed = 0;
            let mut organism_cells = 0;
            for cy in y..y + h {
                for cx in x..x + w {
                    let cell = world.get(cx, cy);
                    if cell.material == material::EMPTY {
                        continue;
                    }
                    removed += 1;
                    if cell.organism_id() != 0 {
                        organism_cells += 1;
                    }
                    world.set(cx, cy, Cell::EMPTY);
                }
            }
            println!("  cut: ({x}, {y}) {w}x{h} at frame {now} -- removed {removed} cells, {organism_cells} of them living tissue");
        } else {
            i += 1;
        }
    }
}

/// Keep the world free of loose material from `frame` onward -- erase every
/// `Powder`-kind cell, every frame, and say how many the first pass took.
///
/// **Continuous, not one-shot, and that was measured rather than assumed.**
/// A single sweep at the blast frame removed *zero* cells (the muck is still
/// in the particle system, not in the grid), one at frame 75 removed 74 and
/// one at frame 100 removed 123 -- the plug is still arriving, so any single
/// instant is an arbitrary fraction of it. What the control has to hold
/// still is "nothing ever stands on the shell", which is a standing state,
/// not an event (`CLAUDE.md`: measure the standing state, not the event
/// rate).
///
/// Reports the first pass's count, because "the control removed the plug" is
/// a counter question: a sheet of a cave with no rubble in it and a sheet of
/// a cave whose rubble had already poured away look identical.
///
/// The cave-volume and cells-lost lines are **not comparable across this
/// flag** -- vacuuming rubble empties cells the census would otherwise still
/// count. Compare a vacuumed run only against another vacuumed run.
fn fire_due_depowder(world: &mut World, pending: &mut Option<usize>, first: &mut bool, now: usize) {
    match *pending {
        Some(frame) if frame <= now => {}
        _ => return,
    }
    let mut removed = 0;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if world.materials.kind(world.get(x, y).material) == MaterialKind::Powder {
                world.set(x, y, Cell::EMPTY);
                // Taking a load off is a disturbance exactly as putting one
                // on is, so the rock underneath has to be asked the question
                // again -- the same fan-out `World::paint_capsule` already
                // does for an ordinary erase. Around the erased cell only:
                // a world-wide reschedule every frame would swamp the site
                // budget and measure the scheduler instead of the model.
                world.schedule_structural_check_around(x, y);
                removed += 1;
            }
        }
    }
    if *first {
        println!("  depowder: from frame {now} on, the world is kept clear of loose material -- first pass took {removed} cells");
        *first = false;
    }
}

/// One charge that has actually gone off, in fire order: where it landed,
/// how big it was and which frame it detonated on.
///
/// Recorded rather than read back off `Args`, because `blast=`'s site is
/// not known until it fires and because the per-charge counters and the
/// `panels=` sheet both have to anchor on the frame it *did* fire, not the
/// frame it was asked to. A charge scheduled past the end of a run never
/// appears here at all, which is the honest answer.
#[derive(Clone, Copy, Debug)]
struct FiredCharge {
    x: i32,
    y: i32,
    radius: i32,
    frame: usize,
}

/// The topmost **solid** cell in column `x`: the smallest `y` whose cell is
/// neither materially empty, nor `Gas`, nor `Liquid`.
///
/// `Liquid` is excluded deliberately rather than by oversight. A charge
/// under the sea should be measured from the **seabed**, so `blast=120,8`
/// reads as "8 cells into the seabed with water overhead" -- which is the
/// near-water case this harness exists to fire. Measuring from the water
/// surface instead would put the same argument at a different depth on
/// every seed, which is the exact failure `blast=` exists to remove.
///
/// The emptiness test is the raw material comparison, not
/// `Cell::is_empty()`: that one is managed-aware and a promoted liquid
/// body's container cells read as *not* empty while holding no material at
/// all (`CLAUDE.md` gotchas; `explosion.rs`'s `clear_annulus` carries the
/// same note).
///
/// Panics on a column with no solid cell in it. A charge that quietly
/// relocated itself to somewhere it could be placed would make the sheet
/// lie about what it fired, and a sheet that lies is worse than no sheet.
fn solid_surface_y(world: &World, x: i32) -> i32 {
    for y in 0..HEIGHT {
        let m = world.get(x, y).material;
        if m == material::EMPTY {
            continue;
        }
        match world.materials.kind(m) {
            MaterialKind::Gas | MaterialKind::Liquid => continue,
            _ => return y,
        }
    }
    panic!("blast= at column {x}: no solid surface in that column, so the charge would have fired into open sky");
}

/// Light a fire at a scheduled frame -- `ignite=x,y,radius,frame`.
///
/// Same shape as `fire_due_cuts` and `fire_due_explosions`, drained the same
/// way, and called from the same three places for the same reason: a scene
/// whose whole subject is a fire front has to start burning *after* the
/// vegetation has grown, not at build time when there is nothing to burn.
///
/// Reports the cell count it actually lit, not the radius it was asked for.
/// "Did the fire start in the grass" is a counter question: an ignition two
/// rows above a sward looks identical on a contact sheet to one inside it,
/// and it would smoulder out while reading as a fire that failed to spread.
/// Apply every scheduled `dry=` whose frame has arrived. Same drain-based
/// shape as `fire_due_ignitions` below and called from the same places,
/// for the same reason: this has to land *after* the vegetation has grown,
/// and growth is what makes the ground damp in the first place.
///
/// Reports the cell count it changed and what the ground was before, not
/// just what it was set to. "Did the reset reach the bed" is a counter
/// question -- a `dry=` aimed at a scene with no water-holding material in
/// it changes nothing and looks, on a contact sheet, exactly like a dry
/// meadow that refused to burn for some other reason.
fn fire_due_dries(world: &mut World, pending: &mut Vec<(u16, usize)>, now: usize) {
    let mut i = 0;
    while i < pending.len() {
        if pending[i].1 <= now {
            let (aux, _) = pending.remove(i);
            let Some(b) = world.bounds() else { continue };
            let (mut changed, mut before) = (0usize, 0u64);
            for y in b.min_y..=b.max_y {
                for x in b.min_x..=b.max_x {
                    let c = world.get(x, y);
                    if world.materials.get(c.material).water_capacity > 0 {
                        before += u64::from(pixel_physics::sim::update::soil_moisture(c));
                        changed += 1;
                        world.set(x, y, c.with_aux(aux));
                    }
                }
            }
            let mean = if changed == 0 { 0 } else { before / changed as u64 };
            println!("  dry: {changed} water-holding cells set to aux {aux} at frame {now} -- they averaged {mean} before");
        } else {
            i += 1;
        }
    }
}

fn fire_due_ignitions(world: &mut World, pending: &mut Vec<(i32, i32, i32, usize)>, now: usize) {
    let mut i = 0;
    while i < pending.len() {
        if pending[i].3 <= now {
            let (x, y, r, _) = pending.remove(i);
            let lit = (y - r..=y + r)
                .flat_map(|cy| (x - r..=x + r).map(move |cx| (cx, cy)))
                .filter(|&(cx, cy)| {
                    let d = (cx - x, cy - y);
                    d.0 * d.0 + d.1 * d.1 <= r * r && world.get(cx, cy).material != material::EMPTY
                })
                .count();
            world.ignite_circle(x, y, r);
            println!("  ignite: ({x}, {y}) r={r} at frame {now} -- lit {lit} cells");
        } else {
            i += 1;
        }
    }
}

/// Fire every scheduled explosion whose frame has arrived, removing it from
/// the pending list so it cannot fire twice. Draining rather than
/// index-matching makes this safe to call both inside the stepping loop and
/// immediately before a capture, which is what lets `frame=0` work at all
/// (with `start=0` the loop body never runs before the first tile).
fn fire_due_explosions(
    world: &mut World,
    particles: &mut ParticleSystem,
    blasts: &mut explosion::Blasts,
    pending: &mut Vec<(i32, i32, i32, f32, usize)>,
    pending_blasts: &mut Vec<(i32, i32, i32, f32, usize)>,
    fired: &mut Vec<FiredCharge>,
    now: usize,
) {
    // **Order within one frame: every `explode=` due now, then every
    // `blast=` due now.** Arbitrary, and written down *because* it is
    // arbitrary -- determinism is required here (`PLAN.md`), and an order
    // nobody wrote down is an order somebody will reverse while tidying.
    let mut i = 0;
    while i < pending.len() {
        if pending[i].4 <= now {
            let (x, y, r, strength, _) = pending.remove(i);
            println!("  boom: ({x}, {y}) r={r} strength={strength} at frame {now}");
            blasts.trigger_with(world, particles, x, y, r, strength);
            fired.push(FiredCharge { x, y, radius: r, frame: now });
        } else {
            i += 1;
        }
    }
    let mut i = 0;
    while i < pending_blasts.len() {
        if pending_blasts[i].4 <= now {
            let (x, depth, r, strength, _) = pending_blasts.remove(i);
            // Resolved *here*, one line before the charge goes off, which
            // is the whole reason `blast=` is a separate list rather than
            // being folded into `explosions` at parse time: the surface it
            // measures has to be the surface as it is now, craters and all.
            let surface = solid_surface_y(world, x);
            let y = surface + depth;
            // The `-- blast=` suffix says what the argument resolved to, so
            // the sheet can be read without re-deriving the terrain. It
            // appears only for `blast=`; `explode=`'s line above is
            // unchanged byte for byte, because recorded measurements in
            // `Reports/explosion-stone-review.md` parse out of it.
            // `0` means "whatever this charge is" -- see `Args::charge`.
            let r = if r > 0 { r } else { blasts.tuning.radius.max(1.0) as i32 };
            let strength = if strength > 0.0 { strength } else { blasts.tuning.strength };
            println!("  boom: ({x}, {y}) r={r} strength={strength} at frame {now} -- blast={x},{depth} (surface y={surface})");
            blasts.trigger_with(world, particles, x, y, r, strength);
            fired.push(FiredCharge { x, y, radius: r, frame: now });
        } else {
            i += 1;
        }
    }
}

/// One full frame, in `App::update`'s own phase order.
///
/// This harness originally ran the CA sweep and nothing else, which is fine
/// for liquids but makes an explosion unviewable: debris lives in
/// `ParticleSystem` and does not move without `particles.step`, and the
/// pressure/heat a blast writes never propagates without `step_fields`. The
/// added phases are no-ops or non-CA-affecting for the four pre-existing
/// scenes (nothing promotes a liquid body, nothing schedules an active site,
/// and no liquid rule reads the field), so they do not move the measurements
/// already recorded against those.
fn advance(
    world: &mut World,
    particles: &mut ParticleSystem,
    blasts: &mut explosion::Blasts,
    parallel_driver: bool,
    step_no: usize,
    gnome: &mut Gnome,
    per_charge_reports: bool,
) {
    if parallel_driver {
        parallel::step(world);
    } else {
        update::step(world);
    }
    world.step_liquid_bodies();
    // M8 chunk bodies, in `App::update`'s own slot. Without this a promoted
    // body is lifted out of the grid and then never moves -- a collapse
    // would render as material simply disappearing, which is exactly the
    // kind of thing this harness exists to catch by eye.
    pixel_physics::sim::rigid::step_chunk_bodies(world);
    // M9: the gnome, in his `App::update` slot too. Input is scripted --
    // run right, jump once a second -- because a filmstrip's job is to
    // show the arc, not to be played. A no-op for every scene that
    // summons no player.
    if world.player.is_some() {
        gnome.act(world, step_no);
    }
    // **`SCHED_BACKLOG=N` -- put N background sites in front of the
    // scheduler every frame, so a scene can be looked at in the state a
    // player who has been digging is actually in.**
    //
    // This is a *reproduction*, not a fabrication. Measured on the shipped
    // 8192x2560 world with the pick swung every 20 frames (`scale_probe
    // load=ants:64,mine:20`), the structural queue produces 5,558-9,080
    // sites a frame against the 2,000 `scheduler::MAX_SITES_PER_FRAME`
    // drains, pending climbs past 62,000, and the creature census goes 27
    // sites a frame -> 11 -> **0**. Nothing in `filmstrip` can reach that
    // state on its own: its worlds are 512x320 and none of its scenes digs
    // for two minutes. This knob supplies the backlog directly so the
    // *consequence* -- what a colony looks like when the queue is full --
    // can be judged by eye, which is the only way this project judges
    // anything.
    //
    // An env var rather than an argument because `advance` has three
    // callers and this is a debug knob, matching `SCHED_PASS`,
    // `PROBE_NO_LOAD` and `RECONVERGE_AT` in the engine.
    //
    // `StructuralCheck` on an empty cell returns immediately
    // (`structural::tick`'s first branch), so the sites cost queue depth
    // and almost no time -- which is the point: what is being demonstrated
    // is *contention for the budget*, not the cost of the work.
    {
        use std::sync::OnceLock;
        static BACKLOG: OnceLock<usize> = OnceLock::new();
        let n = *BACKLOG.get_or_init(|| std::env::var("SCHED_BACKLOG").ok().and_then(|v| v.parse().ok()).unwrap_or(0));
        if n > 0 {
            let due = world.frame;
            // Swept over the left of the world, and that is load-bearing:
            // `ActiveSite`'s `Ord` is `next_frame` then `x`, so a flood to
            // the *west* of whatever is being watched is what puts the
            // watched thing last. A flood to its east would be served
            // after it and demonstrate nothing.
            for i in 0..n {
                world.schedule_active_site(pixel_physics::sim::scheduler::ActiveSite {
                    x: (i % 300) as i32,
                    y: 8 + (i / 300) as i32,
                    kind: pixel_physics::sim::scheduler::ActiveKind::StructuralCheck,
                    next_frame: due,
                });
            }
        }
    }
    world.step_active_sites();
    // R5's report line: printed the frame a blast's last stage finishes,
    // not at the trigger frame (`fire_due_explosions`'s own `boom:` line is
    // that one) -- `cells_cleared`/`cells_held_by_containment` accumulate
    // across every stage, so the report is only complete once `blasts` goes
    // back to empty. Checked by transition (was active, now is not) rather
    // than printed unconditionally every frame `blasts` is non-empty, or
    // this would spam one line per stage instead of one per blast.
    let was_active = !blasts.is_empty();
    blasts.step(world, particles);
    if was_active && blasts.is_empty() {
        println!("  blast report: {}", blasts.last_blast_report());
    }
    // One line per blast that finished this frame, each naming its own
    // site. The line above cannot do this job for a run that fires more
    // than one charge: `last_blast_report` is a single slot, so two
    // overlapping blasts collapse into one line and eight of nine reports
    // are silently overwritten -- `CLAUDE.md`'s "did it fire at all", one
    // level up. The queue is drained every frame regardless of whether
    // anything is printed, so `Blasts::finished` never accumulates.
    //
    // **Gated, and the gate is not cosmetic.** With a single charge this
    // line says exactly what the line above it says, only with an `(x, y)`
    // in front -- and every measurement recorded in
    // `Reports/explosion-stone-review.md` §8-§13 was taken from a
    // single-charge run, so an unconditional second line would change the
    // stdout of every one of them for no information. See
    // `per_charge_reports` at its definition in `run_once`.
    for (bx, by, report) in blasts.drain_finished_reports() {
        if per_charge_reports {
            println!("  blast report ({bx}, {by}): {report}");
        }
    }
    // `App::update`'s slot exactly: splashes are debited from the pool and
    // thrown here, between the blast stage and the particle step. Without
    // this line the sweep reports splash sites every frame and nothing ever
    // takes one, so `scene=blob` would show none -- and it is the only
    // place the water actually leaves the pool, so a harness that omits it
    // loses nothing rather than quietly draining.
    pixel_physics::sim::particle::throw_splashes(world, particles);
    particles.step(world);
    world.step_fields();
}

/// The scripted gnome, and the tally of what his verbs actually did.
///
/// Scripted rather than played because a contact sheet is judged on the
/// shape of an arc or the opening of a bore, and a script that varied per
/// run would make two sheets incomparable. The counters are here for the
/// separate reason `CLAUDE.md` records: "did it fire at all needs a
/// counter". A bore full of loose rubble and a bore the dig never touched
/// are the same picture at the zoom these are read at, and `buried` is a
/// flag no picture shows at all.
struct Gnome {
    /// The tuning this run digs with — `Tuning::default` except for
    /// whatever `yield=` overrode.
    tuning: pixel_physics::sim::player::Tuning,
    /// Which script — set from the scene name, since the M9 scenes each
    /// exist to show a different verb.
    script: Script,
    /// Bites that actually landed (the cooldown swallows most frames).
    bites: usize,
    /// Where he was standing when he set off, so the sheet can report how
    /// far he actually got. See `Script::Wood`.
    start_x: Option<f32>,
    /// Whether `Script::Climb` has hold of something yet, and the height it
    /// had when it grabbed. The rise from there is the number the sheet is
    /// read for — see `Script::Climb`.
    grabbed: bool,
    grabbed_at: f32,
    highest: f32,
    shakes: usize,
    dislodged_by_shaking: usize,
    shed: usize,
    seeds: usize,
    shaken_cells: usize,
    shaken_shoot: u32,
    /// Loose cells shoved clear of a bore, summed over every bite.
    displaced: usize,
    dusted: usize,
    /// First tick he read as buried, and the first tick he was free
    /// again after that — the two numbers `scene=bury` exists to produce.
    went_under: Option<usize>,
    came_back: Option<usize>,
    /// Hammer blows that broke something, and how many cells they acted
    /// on. Counted separately from the swings that landed on air, which is
    /// the distinction `Smash::broken` exists to make: a blow at a face and
    /// a blow at nothing are the same picture.
    blows: usize,
    broken: usize,
    /// Axe strokes, the cells they chipped out of the world, and how many
    /// of them landed on something *living* — the last is what separates a
    /// tree being felled from an axe hitting the rock behind it.
    strokes: usize,
    chips: usize,
    living_strokes: usize,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum Script {
    /// `scene=gnome`: run right, jumping once a second.
    #[default]
    Course,
    /// `scene=tunnel`: walk right into the cliff, digging ahead of him.
    Tunnel,
    /// `scene=bury`: stand still until the sand lands, then dig out.
    Bury,
    /// `scene=swim`: sink, float, pull under with `S`, then jump clear.
    Swim,
    /// `scene=ride`: no input at all — the shelf under him gives way and
    /// the only question is whether he goes with it.
    Ride,
    /// `scene=wood`: stand still while the stand grows, then walk the
    /// length of it.
    Wood,
    /// `scene=climb`: walk until something is in reach, then go up it.
    Climb,
    /// `scene=shake`: walk until a tree is in reach, then keep shaking it.
    Shake,
    /// `scene=smash`: stand at a cliff face and hammer it. The scene the
    /// pick cannot answer — what a blow *damages* reaches well past what
    /// it removes, so what this is read for is whether the face fails
    /// rather than whether a hole appears.
    Smash,
    /// `scene=chop`: walk until a tree is in reach, then take the axe to
    /// it. Paired with `scene=shake` on purpose: same walk, same tree,
    /// same button, and the belt is the whole difference between a shower
    /// of leaves and a felled trunk.
    Chop,
}

/// How long `Script::Wood` waits before setting off.
///
/// A grove is planted as *seeds*, and the sheets that judge tree shape are
/// shot at `start=8000`. Walking into a plot of bare soil would answer
/// nothing, so he holds still until there is a wood to walk into.
const WOOD_WALK_FROM: usize = 6000;

/// How long `Script::Climb` walks before it starts reaching for a hold —
/// far enough to be standing in a tree rather than beside a stray twig.
/// See the arm in `act` for the run that made this necessary.
const CLIMB_WALK_TICKS: usize = 60;

impl Gnome {
    fn for_scene(scene: &str, dig_yield: f32, shoulder_grains: u8) -> Self {
        let script = match scene {
            "tunnel" => Script::Tunnel,
            "bury" => Script::Bury,
            "swim" => Script::Swim,
            "ride" => Script::Ride,
            "wood" => Script::Wood,
            "climb" => Script::Climb,
            "shake" => Script::Shake,
            "smash" => Script::Smash,
            "chop" => Script::Chop,
            _ => Script::Course,
        };
        Self {
            script,
            tuning: pixel_physics::sim::player::Tuning { dig_yield, shoulder_grains, ..Default::default() },
            bites: 0,
            start_x: None,
            grabbed: false,
            grabbed_at: 0.0,
            highest: 0.0,
            shakes: 0,
            dislodged_by_shaking: 0,
            shed: 0,
            seeds: 0,
            shaken_cells: 0,
            shaken_shoot: 0,
            displaced: 0,
            dusted: 0,
            went_under: None,
            came_back: None,
            blows: 0,
            broken: 0,
            strokes: 0,
            chips: 0,
            living_strokes: 0,
        }
    }

    /// One tick of scripted input, plus the dig the scripts that dig ask
    /// for. Ordered dig-then-step, matching `App`: the click is handled
    /// on the input event, the character steps afterwards, so the
    /// depenetration pass in `step` sees the cells the dig just freed and
    /// can stand him up in them on the same frame.
    fn act(&mut self, world: &mut World, step_no: usize) {
        use pixel_physics::sim::player::{self, PlayerInput};
        let tuning = self.tuning;
        let phase = step_no % 60;
        let input = match self.script {
            Script::Course => PlayerInput {
                right: true,
                jump_pressed: phase == 30,
                jump_held: (30..48).contains(&phase),
                ..Default::default()
            },
            // Dig standing still, then walk in — alternating, rather
            // than holding `right` the whole time as this first did.
            // Walking constantly into the face makes him climb his own
            // spoil, and the sheet then measures *ramp* rather than
            // *cave*: at `yield=1.0`, where by construction no volume
            // leaves and no cave can exist, he travelled furthest of any
            // setting purely by walking up rubble.
            Script::Tunnel => PlayerInput { right: step_no % 90 >= 60, ..Default::default() },
            Script::Bury => PlayerInput::default(),
            // Three acts, on a fixed clock so two sheets compare: fall in
            // and float (to 150), pull under (to 260), then stroke up and
            // jump out.
            Script::Swim => PlayerInput {
                down: (150..260).contains(&step_no),
                jump_held: step_no >= 260,
                jump_pressed: step_no >= 260,
                ..Default::default()
            },
            Script::Ride => PlayerInput::default(),
            Script::Wood => PlayerInput { right: step_no >= WOOD_WALK_FROM, ..Default::default() },
            // Walk until he has a handhold, then hold `W` and nothing
            // else. Holding a direction *while* climbing shimmies him
            // sideways out of the trunk, which is how you leave a tree and
            // is not what this scene is showing.
            //
            // **Walk a fixed distance first, then climb.** The first
            // version reached for the first handhold it met, which was a
            // creeping twig at ground level twelve cells from where he
            // spawned. He gripped it, rose, left it, launched, fell back
            // in, gripped again -- and the counter reported "climbed 30
            // cells" off a stack of grab-and-launch cycles at knee height,
            // with the trees still a hundred cells away. The number was
            // real and meant nothing, which is the exact trap `CLAUDE.md`
            // opens by warning about; the picture is what caught it.
            Script::Climb if self.grabbed => PlayerInput { grab: true, jump_held: true, ..Default::default() },
            // Same walk-first delay `Script::Climb` needed, and for the
            // same reason: the first thing in reach of the spawn point is a
            // creeping twig, not a tree.
            Script::Shake => PlayerInput {
                right: step_no >= WOOD_WALK_FROM && (step_no < WOOD_WALK_FROM + CLIMB_WALK_TICKS || !self.grabbed),
                ..Default::default()
            },
            // Walk in, then **alternate** blows and steps, exactly as
            // `Tunnel` does and for a related reason.
            //
            // Standing still was the first version and it stopped after two
            // blows: a hammer does not excavate, so the rock it breaks
            // stays as rubble at his feet, and `hammer_point`'s ray then
            // finds no `Hard` cell inside `hammer_reach` (12, deliberately
            // short -- a swung hammer is not a extended pick). The counter
            // is what said so, frozen at `2 blows landed` across all four
            // tiles while the picture showed a perfectly good fissure star.
            //
            // That is the scene being wrong rather than the tool: a player
            // hammering a cliff walks into the hole they are making. See
            // `CLAUDE.md`'s "a scene that contradicts the code will look
            // like a bug in the code".
            Script::Smash => PlayerInput { right: step_no < 120 || step_no % 90 >= 60, ..Default::default() },
            Script::Chop => PlayerInput {
                right: step_no >= WOOD_WALK_FROM && (step_no < WOOD_WALK_FROM + CLIMB_WALK_TICKS || !self.grabbed),
                ..Default::default()
            },
            Script::Climb => PlayerInput {
                right: step_no >= WOOD_WALK_FROM,
                // Reaching only starts once he is clear of the twig — walk
                // first, then walk *and* reach until something takes.
                // `grab` is the reach: climbing has its own key now, so
                // holding `W` alone takes hold of nothing.
                grab: step_no >= WOOD_WALK_FROM + CLIMB_WALK_TICKS,
                jump_held: step_no >= WOOD_WALK_FROM + CLIMB_WALK_TICKS,
                ..Default::default()
            },
        };
        if self.script == Script::Wood && step_no == WOOD_WALK_FROM {
            self.start_x = world.player.as_ref().map(|p| p.x);
        }
        if self.script == Script::Shake && step_no >= WOOD_WALK_FROM + CLIMB_WALK_TICKS {
            // **Pointed at, not merely toward.** This aimed at the far
            // right edge of the world, which worked while the shake walked
            // a ray out from the gnome and took the first living thing on
            // it -- and stopped working the moment it started taking what
            // the cursor is actually on. Zero shakes, zero shed, off a
            // stand full of trees.
            //
            // His own centre is the honest aim for this script: he walks
            // through trees, so when he is standing in one the cursor on
            // himself is a cursor on it, and when he is not, it is not and
            // he keeps walking. That is exactly what the scene is for.
            let target = world.player.as_ref().and_then(|p| player::shake_target(world, p, p.center(), &tuning));
            if let Some(at) = target {
                self.grabbed = true;
                let shaken = world.get(at.0, at.1).organism_id();
                self.shaken_shoot = world.organism(shaken).map(|o| o.shoot_cells).unwrap_or(0);
                if let Some(s) = player::shake(world, at, &tuning) {
                    self.shaken_cells = s.cells;
                    self.shakes += 1;
                    self.dislodged_by_shaking += s.dislodged;
                    self.shed += s.shed;
                    self.seeds += s.seeds;
                }
            }
        }
        // The hammer, once he has walked up to the face. Aimed straight
        // ahead at his own height, which `hammer_point` then clamps onto
        // the near face — the same shape `Script::Tunnel` uses for the
        // pick, and for the same reason its own comment gives.
        // **Hammer to break, pick to clear** — and the alternation is the
        // scene rather than a workaround, because it is the loop the belt
        // exists for.
        //
        // Hammering alone stalls, and the counter is what said so: frozen
        // at **3 blows landed** across four tiles, with the gnome pinned at
        // x=173. A blow does not excavate — `rigid::strike` breaks rock to
        // rubble in place — so after two or three the face has retreated
        // past `hammer_reach` and the rubble it made is a drift several
        // cells abreast, which `wade_rows`/`shoulder_grains` correctly
        // treat as a wall. He cannot reach the rock and cannot walk to it.
        //
        // That is not the hammer being broken; it is the hammer being a
        // *breaking* tool with no spoil model, which is the pick's job.
        // Alternating them is what a player does and what this scene has to
        // show, or the sheet measures a tool used wrongly.
        if self.script == Script::Smash && step_no >= 120 {
            let hammering = (step_no / 240).is_multiple_of(2);
            if let Some(p) = world.player.as_mut() {
                p.tool = if hammering { player::Tool::Hammer } else { player::Tool::Pick };
            }
            let (cx, cy) = world.player.as_ref().expect("scene summoned one").center();
            if hammering {
                if let Some(hit) = player::smash(world, (cx + 60, cy), &tuning) {
                    if hit.broken > 0 {
                        self.blows += 1;
                        self.broken += hit.broken;
                    }
                }
            } else if let Some(bite) = player::dig(world, (cx + 60, cy), &tuning) {
                self.bites += 1;
                self.displaced += bite.displaced;
                self.dusted += bite.dusted;
            }
        }
        // The axe, on `scene=shake`'s walk. `chop_point` snaps to the
        // tissue under the cursor, so aiming at his own centre is aiming at
        // whatever he is standing in — exactly the argument the shake makes
        // for the same aim.
        // **Aimed ahead of him, not at his own centre, and swung
        // unconditionally.**
        //
        // `scene=shake` aims at the gnome's centre, and that is right for
        // the shake, which leaves the tissue where it is. It is wrong for a
        // *cutting* verb, and the counter said so: **1 stroke** across four
        // tiles. The first chop takes a `chop_radius` hole out of the cells
        // at his centre, so the "is there living tissue at the cursor" test
        // that gated the swing stops being true — the aim ate itself.
        //
        // A player keeps the cursor on the trunk, not on their own belly.
        // Aiming a few cells ahead at chest height does that, and dropping
        // the gate lets `chop_point` answer the question it exists to
        // answer: living tissue first, then a creature, then whatever the
        // ray reaches. A swing that finds nothing is a swing at nothing,
        // which `Chop::living` reports and the tile prints.
        if self.script == Script::Chop && step_no >= WOOD_WALK_FROM + CLIMB_WALK_TICKS {
            if let Some(p) = world.player.as_mut() {
                p.tool = player::Tool::Axe;
            }
            let p = world.player.as_ref().expect("summoned");
            let (cx, cy) = p.center();
            let ahead = (cx + if p.facing_left { -4 } else { 4 }, cy - 3);
            self.grabbed = player::shake_target(world, p, ahead, &tuning).is_some();
            if let Some(cut) = player::chop(world, ahead, &tuning) {
                self.strokes += 1;
                self.chips += cut.chips;
                self.living_strokes += usize::from(cut.living);
            }
        }
        if self.script == Script::Climb {
            if let Some(p) = world.player.as_ref() {
                if p.climbing && !self.grabbed {
                    self.grabbed = true;
                    self.grabbed_at = p.y;
                    self.highest = p.y;
                }
                self.highest = self.highest.min(p.y);
            }
        }
        // Aim: straight ahead at his own height for the tunnel, and
        // anywhere at all while buried, since a buried bite auto-aims.
        let digging = match self.script {
            Script::Course | Script::Swim | Script::Ride | Script::Wood | Script::Climb => false,
            // Handled below rather than through the dig path: the same
            // left button, a different verb.
            Script::Shake | Script::Smash | Script::Chop => false,
            Script::Tunnel => true,
            Script::Bury => step_no > 90,
        };
        if digging {
            // Aimed past his reach, not at a fixed 20 cells ahead, which
            // is what this did first. The aim ray is clipped to
            // `dig_reach`, so a cursor closer than the working face makes
            // the bite land in the open bore behind it — the tunnel
            // stopped advancing once it grew past the aim point, and the
            // dust count collapsed to a handful of cells a bite while
            // looking like the mechanism had failed.
            let (cx, cy) = world.player.as_ref().expect("scene summoned one").center();
            let far = cx + tuning.dig_reach as i32 * 2;
            if let Some(bite) = player::dig(world, (far, cy), &tuning) {
                self.bites += 1;
                self.displaced += bite.displaced;
                self.dusted += bite.dusted;
            }
        }
        player::step(world, input, &tuning);
        let p = world.player.as_ref().expect("still summoned");
        if p.buried && self.went_under.is_none() {
            self.went_under = Some(step_no);
        }
        if !p.buried && self.went_under.is_some() && self.came_back.is_none() {
            self.came_back = Some(step_no);
        }
    }

    /// How far he has come since setting off, or 0 if he never did.
    fn travelled(&self, world: &World) -> i32 {
        match (self.start_x, world.player.as_ref()) {
            (Some(from), Some(p)) => (p.x - from) as i32,
            _ => 0,
        }
    }

    /// The line printed beside each tile. Empty for scenes with no gnome,
    /// so the sheets that predate M9 read exactly as they did.
    fn report(&self, world: &World) -> Option<String> {
        let p = world.player.as_ref()?;
        let mut s = format!(
            "    gnome: at ({:.0}, {:.0}), {}, bites {} ({} cells displaced)",
            p.x,
            p.y,
            if p.buried {
                "BURIED"
            } else if p.swimming {
                "swimming"
            } else if p.wading {
                "wading"
            } else if p.grounded {
                "grounded"
            } else {
                "airborne"
            },
            self.bites,
            self.displaced
        );
        s.push_str(&format!(", {} dusted", self.dusted));
        // **Only when the tool that produces them is in his hands.** A row
        // of zeroes on every gnome sheet is a row nobody reads, and these
        // are exactly the "did it fire at all" counters the picture cannot
        // supply — a hammered cliff face and an untouched one differ by a
        // shade of grey, and an axe notch is three pixels.
        if self.script == Script::Smash {
            // **Cracked cells beside broken ones, because the playtest
            // complaint was a *ratio*.** Reported of the first hammer:
            // "it mostly makes big strike lines instead of breaking rock
            // into pieces". Both halves of that are real quantities --
            // cells the blow scored a fissure through, against cells it
            // actually took apart -- and neither is legible on a contact
            // sheet, where a crack is a one-pixel line and fresh rubble is
            // a shade of grey. Read the two together or the sheet cannot
            // say whether a change moved the balance or just the total.
            let cracked = count_cracked(world);
            s.push_str(&format!(", {} blows landed ({} cells broken, {cracked} cells cracked)", self.blows, self.broken));
        }
        if self.script == Script::Chop {
            s.push_str(&format!(
                ", {} strokes ({} on living tissue, {} cells chipped)",
                self.strokes, self.living_strokes, self.chips
            ));
        }
        // How much material is left in the world at all. The one number
        // that says whether a bore can exist: `mine` conserves cells, so
        // without thinning this never moves and no cave is possible
        // however much rubble is thrown about.
        let held: usize = (0..HEIGHT)
            .map(|y| (0..WIDTH).filter(|&x| world.get(x, y).material != material::EMPTY).count())
            .sum();
        s.push_str(&format!(", world holds {held} cells"));
        if let Some(from) = self.start_x {
            s.push_str(&format!(", travelled {:.0} cells", p.x - from));
        }
        // How much of him a tree is actually covering this frame.
        //
        // The depth effect is invisible in a still unless the sheet happens
        // to catch a frame with real overlap, and hunting for one by eye is
        // exactly the "an image says what and where, only a number says how
        // much" trap. This says whether there was anything to see.
        let (px0, py0, px1, py1) = p.bounds();
        let covered = (py0..=py1)
            .flat_map(|y| (px0..=px1).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let c = world.get(x, y);
                c.organism_id() != 0 && world.materials.get(c.material).climbable
            })
            .count();
        s.push_str(&format!(", {covered}/{} cells behind foliage", (px1 - px0 + 1) * (py1 - py0 + 1)));
        if self.script == Script::Shake {
            // What is being shaken, as well as what came of it. A shake
            // that reaches only the trunk it was grabbed by and one that
            // reaches the crown look identical on a contact sheet, and the
            // first version of the cap made exactly that mistake.
            s.push_str(&format!(", shaking {} of a {}-shoot plant", self.shaken_cells, self.shaken_shoot));
            s.push_str(&format!(
                ", {} shakes: {} knocked loose, {} leaves down, {} sown",
                self.shakes, self.dislodged_by_shaking, self.shed, self.seeds
            ));
        }
        if self.script == Script::Climb {
            match self.grabbed {
                true => s.push_str(&format!(", climbed {:.0} cells (gripped at y={:.0})", self.grabbed_at - self.highest, self.grabbed_at)),
                false => s.push_str(", NEVER GRIPPED"),
            }
        }
        if let Some(under) = self.went_under {
            s.push_str(&format!(", went under at {under}"));
            match self.came_back {
                Some(back) => s.push_str(&format!(", dug out by {back} ({} ticks under)", back - under)),
                None => s.push_str(", still under"),
            }
        }
        Some(s)
    }
}

/// Print the materials in `args.dump` as ASCII. See `Args::dump`.
fn dump_materials(world: &World, args: &Args) {
    let Some(r) = args.dump else { return };
    let player = world.player.as_ref().map(|p| p.bounds());
    println!("    dump x {}..{} y {}..{}:", r.min_x, r.max_x, r.min_y, r.max_y);
    for y in r.min_y..=r.max_y {
        let mut line = String::new();
        for x in r.min_x..=r.max_x {
            if let Some((x0, y0, x1, y1)) = player {
                if x >= x0 && x <= x1 && y >= y0 && y <= y1 {
                    line.push('P');
                    continue;
                }
            }
            let cell = world.get(x, y);
            line.push(match world.materials.kind(cell.material) {
                _ if cell.material == material::EMPTY => '.',
                MaterialKind::Solid => '#',
                MaterialKind::Powder => 'o',
                MaterialKind::Liquid => '~',
                MaterialKind::Gas => ':',
                _ => '?',
            });
        }
        println!("    {y:>3} {line}");
    }
}

/// What `log_pieces` found: the orientation split, the mass held in pieces,
/// and every piece's `(cells, width, height)` largest first.
#[derive(Default)]
struct LogPieces {
    lying: usize,
    upright: usize,
    square: usize,
    cells_in_pieces: usize,
    sizes: Vec<(usize, i32, i32)>,
}

/// The settled `log` pieces, folded into 8-connected clusters: how many,
/// how big, and — the part nothing else here can say — **how many are lying
/// down rather than standing on end**.
///
/// # Why orientation, and why it needed its own instrument
///
/// The acceptance question for T1 is *"do you see logs lying on the
/// ground"*, and the owner's answer to the first card was *"it doesn't
/// obviously look like fallen logs"*. That has two completely different
/// causes and no counter in the repo could tell them apart: either the
/// pieces are not being made, or they are being made and are standing
/// upright. The second is a live complaint on a neighbouring board in those
/// words — *"the long skinny vertical pieces should fall over, instead of
/// all standing upright"* — so it is not a hypothetical.
///
/// A piece is called **lying** when its bounding box is wider than tall,
/// **upright** when taller than wide, and **square** otherwise. Deliberately
/// the bounding box and not a fitted axis: at this resolution a log is a few
/// cells thick and any moment-of-inertia fit on a 3-cell-wide blob is
/// measuring rounding. The question being asked is the coarse one a player
/// answers by eye.
///
/// Only pieces of `MIN_BODY_CELLS` or more are counted, because that is what
/// "a piece" means everywhere else in this pipeline — a 3-cell speck of log
/// is grit that happens to have landed as a body, and counting it as a
/// fallen log would flatter the number the card is about.
/// Cells carrying a scored fissure — the "strike lines" a hammer leaves.
///
/// Whole-world rather than a disc around the blow, deliberately: cracks
/// are the one damage channel that *accumulates* across blows
/// (`rigid::score_cracks` keys its ray directions on the site so repeats
/// drive the same fissures deeper), so a per-blow count would understate
/// exactly the thing the complaint was about.
fn count_cracked(world: &World) -> usize {
    (0..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| (x, y))).filter(|&(x, y)| world.get(x, y).cracked()).count()
}

fn log_pieces(world: &World) -> LogPieces {
    let Some(log) = world.materials.id_of("log") else { return LogPieces::default() };
    let mut seen: HashSet<(i32, i32)> = HashSet::new();
    let mut pieces: Vec<(usize, i32, i32)> = Vec::new();
    let (mut lying, mut upright, mut square, mut cells_in_pieces) = (0usize, 0usize, 0usize, 0usize);
    for y0 in 0..HEIGHT {
        for x0 in 0..WIDTH {
            if world.get(x0, y0).material != log || seen.contains(&(x0, y0)) {
                continue;
            }
            let mut stack = vec![(x0, y0)];
            seen.insert((x0, y0));
            let (mut lo_x, mut hi_x, mut lo_y, mut hi_y) = (x0, x0, y0, y0);
            let mut n = 0usize;
            while let Some((x, y)) = stack.pop() {
                n += 1;
                lo_x = lo_x.min(x);
                hi_x = hi_x.max(x);
                lo_y = lo_y.min(y);
                hi_y = hi_y.max(y);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (x + dx, y + dy);
                        if (dx, dy) == (0, 0) || !world.in_bounds(nx, ny) {
                            continue;
                        }
                        if world.get(nx, ny).material == log && seen.insert((nx, ny)) {
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            if n < pixel_physics::sim::rigid::MIN_BODY_CELLS {
                continue;
            }
            let (w, h) = (hi_x - lo_x + 1, hi_y - lo_y + 1);
            cells_in_pieces += n;
            match w.cmp(&h) {
                std::cmp::Ordering::Greater => lying += 1,
                std::cmp::Ordering::Less => upright += 1,
                std::cmp::Ordering::Equal => square += 1,
            }
            pieces.push((n, w, h));
        }
    }
    pieces.sort_unstable_by_key(|p| std::cmp::Reverse(p.0));
    LogPieces { lying, upright, square, cells_in_pieces, sizes: pieces }
}

/// The `hanging` census's cells, folded into 8-connected clusters, one
/// line each.
///
/// **A count answers "how much"; only this answers "what".** 47 hanging
/// cells is one raft the size of a dinner plate *or* 47 grains scattered
/// over a pond, and those are opposite bugs: the first is a piece the model
/// declines to drop, the second is cells it never joins into a piece at
/// all. The bare count read the same either way and the owner had to read
/// the ambiguity off the image instead.
///
/// The neighbour tally beside each cluster is the second half of the
/// question. Rock left in mid-*air* and rock left in mid-*water* look
/// identical in a count and are different failures — water is a medium the
/// buoyancy rule in `structural::region_has_free_face` has an opinion
/// about, and air is not — so what is around a cluster is printed rather
/// than inferred from the scene name.
///
/// 8-connected, matching the neighbourhood `load::detached_piece` floods
/// over: a cluster here is meant to be the piece the model would have
/// considered, so a diagonal join must not split it into two.
/// Of the columns that hold water at all, how many have ice at the top of
/// it — see the `ice:` readout.
///
/// **The topmost water-phase cell in the column, not the topmost cell**, so
/// a drift of snow lying on a sheet does not read as an unfrozen surface
/// and a sheet under one still counts. Snow is skipped rather than counted
/// either way: it falls *onto* the water and is not what freezing over
/// means.
fn frozen_surface(world: &World) -> (usize, usize, f64, usize) {
    use pixel_physics::sim::material;
    let (mut frozen, mut total) = (0usize, 0usize);
    let (mut thickness_sum, mut thickest) = (0usize, 0usize);
    for x in 0..WIDTH {
        let (mut top, mut depth) = (None, 0usize);
        // The unbroken run of ice from the first freezable cell down --
        // **the quantity Stefan's law is about**, and the one thing this
        // census could not say. Coverage answers "has it closed over"; a
        // sheet that has closed can still be one cell thick or nine, and
        // the whole shape of real ice growth is thickness against time
        // (it goes as the square root, because ice insulates the water
        // under it). Nothing printed it, so nothing could see the curve.
        let mut run = 0usize;
        let mut counting = false;
        for y in 0..HEIGHT {
            let cell = world.get(x, y);
            let m = world.materials.get(cell.material);
            let is_water = cell.material == material::WATER;
            // Ice, by the same identity test `weather::water_equivalents`
            // uses: a solid whose melting point is below ambient and which
            // melts back into water.
            let is_ice = m.kind == MaterialKind::Solid && m.melts_into == Some(material::WATER);
            if is_water || is_ice {
                depth += 1;
                // **A part-full cell is skipped, not counted as unfrozen.**
                // `fire.rs` refuses to freeze a liquid below its
                // `freeze_min_fill` on purpose -- water holds its fringe
                // liquid to keep the freeze/thaw loop conservative -- and
                // the top row of a settled pond is exactly that remainder.
                // Counting it made this metric read **0 of 60 columns
                // frozen on a pond with 900 cells of ice in it**, which
                // would have been read as the mechanism doing nothing.
                // `CLAUDE.md`: ask what a metric counts when nothing is
                // wrong.
                let freezable = is_ice || pixel_physics::sim::update::liquid_fill(cell) >= m.freeze_min_fill;
                if top.is_none() && freezable {
                    top = Some(is_ice);
                    counting = is_ice;
                }
                if counting {
                    if is_ice {
                        run += 1;
                    } else {
                        counting = false;
                    }
                }
            }
        }
        // **A pond, not a puddle.** Without this the denominator jumps the
        // moment a snowfall thaws: meltwater spreads a film over the whole
        // world and `scene=coldsnap` went from 60 columns to 465, so the
        // percentage stopped being about the pond it was asked about.
        if depth >= POND_MIN_DEPTH {
            total += 1;
            if top == Some(true) {
                frozen += 1;
                thickness_sum += run;
                thickest = thickest.max(run);
            }
        }
    }
    let mean = if frozen > 0 { thickness_sum as f64 / frozen as f64 } else { 0.0 };
    (frozen, total, mean, thickest)
}

/// How deep a column of water has to be before the `ice:` readout counts it
/// as part of a pond. Four: deeper than the film a thaw spreads over flat
/// ground, far shallower than anything anyone would call a pond.
const POND_MIN_DEPTH: usize = 4;

/// Largest cluster the hanging census asks cell by cell.
///
/// Censusing a cluster of `k` unattached cells costs `k^2`, because
/// `load::evaluate` floods the connected region for each one. 512 is set
/// from what the scenes actually contain: the largest hanging piece
/// `lavapour` has ever produced is 18 cells, and a whole `roomcut` collapse
/// is in the hundreds -- so nothing that has ever shown this artifact is
/// sampled, and `scene=capped`'s deliberate 15,840-cell monolith is.
const HANGING_CENSUS_FULL: usize = 512;

fn describe_hanging(world: &World, cells: &[(i32, i32)]) -> Vec<String> {
    use pixel_physics::sim::material;
    let set: HashSet<(i32, i32)> = cells.iter().copied().collect();
    let mut seen: HashSet<(i32, i32)> = HashSet::new();
    struct Cluster {
        size: usize,
        x0: i32,
        x1: i32,
        y0: i32,
        y1: i32,
        air: u32,
        water: u32,
        solid: u32,
    }
    let mut clusters: Vec<Cluster> = Vec::new();
    for &start in cells {
        if !seen.insert(start) {
            continue;
        }
        let mut stack = vec![start];
        let mut members = Vec::new();
        while let Some((x, y)) = stack.pop() {
            members.push((x, y));
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let n = (x + dx, y + dy);
                    if set.contains(&n) && seen.insert(n) {
                        stack.push(n);
                    }
                }
            }
        }
        let mut c = Cluster {
            size: members.len(),
            x0: i32::MAX,
            x1: i32::MIN,
            y0: i32::MAX,
            y1: i32::MIN,
            air: 0,
            water: 0,
            solid: 0,
        };
        for &(x, y) in &members {
            c.x0 = c.x0.min(x);
            c.x1 = c.x1.max(x);
            c.y0 = c.y0.min(y);
            c.y1 = c.y1.max(y);
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if set.contains(&(nx, ny)) || !world.in_bounds(nx, ny) {
                    continue;
                }
                let cell = world.get(nx, ny);
                if cell.material == material::EMPTY {
                    c.air += 1;
                } else if cell.material == material::WATER {
                    c.water += 1;
                } else {
                    match world.materials.kind(cell.material) {
                        MaterialKind::Solid | MaterialKind::Powder => c.solid += 1,
                        // Steam, lava and the rest: real, and not the
                        // distinction this readout exists to draw.
                        _ => {}
                    }
                }
            }
        }
        clusters.push(c);
    }
    clusters.sort_by_key(|c| std::cmp::Reverse(c.size));
    // Six is enough to see the shape of the distribution without burying
    // the tile it belongs to; the tail is summarised rather than dropped,
    // because "and 200 singletons" is itself the finding in the scattered
    // case.
    const SHOWN: usize = 6;
    let mut out = Vec::new();
    let singletons = clusters.iter().filter(|c| c.size == 1).count();
    out.push(format!(
        "in {} clusters, largest {}, {singletons} lone cells",
        clusters.len(),
        clusters.first().map_or(0, |c| c.size),
    ));
    for c in clusters.iter().take(SHOWN) {
        out.push(format!(
            "{:>4} cells at x {}..{} y {}..{}, touching {} air / {} water / {} solid",
            c.size, c.x0, c.x1, c.y0, c.y1, c.air, c.water, c.solid,
        ));
    }
    if clusters.len() > SHOWN {
        out.push(format!("... and {} smaller", clusters.len() - SHOWN));
    }
    out
}

/// Groups of unattached `Solid` touching no possible support, one line
/// each, largest first. See the call site for why this exists beside
/// `hanging` rather than instead of it.
fn describe_afloat(world: &World) -> Vec<String> {
    use pixel_physics::sim::material;
    let mut seen: HashSet<(i32, i32)> = HashSet::new();
    let mut out: Vec<(usize, String)> = Vec::new();
    for y0 in 0..HEIGHT {
        for x0 in 0..WIDTH {
            let cell = world.get(x0, y0);
            if cell.attached()
                || cell.organism_id() != 0
                || world.materials.kind(cell.material) != MaterialKind::Solid
                || seen.contains(&(x0, y0))
            {
                continue;
            }
            // 8-connected over unattached solids, the neighbourhood
            // `load::detached_piece` floods over, so a group here is the
            // piece the model would have considered.
            let mut stack = vec![(x0, y0)];
            seen.insert((x0, y0));
            let mut members = Vec::new();
            let mut liquid = 0u32;
            let mut supportable = false;
            while let Some((x, y)) = stack.pop() {
                members.push((x, y));
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (x + dx, y + dy);
                        if (dx, dy) == (0, 0) || !world.in_bounds(nx, ny) {
                            continue;
                        }
                        let n = world.get(nx, ny);
                        match world.materials.kind(n.material) {
                            // Anything that could bear weight, whether or
                            // not the model currently thinks it does.
                            MaterialKind::Powder => supportable = true,
                            MaterialKind::Solid if n.material == material::BEDROCK || n.attached() => {
                                supportable = true;
                            }
                            MaterialKind::Solid if n.organism_id() == 0 => {
                                if seen.insert((nx, ny)) {
                                    stack.push((nx, ny));
                                }
                            }
                            MaterialKind::Liquid => liquid += 1,
                            _ => {}
                        }
                    }
                }
            }
            if supportable || liquid == 0 {
                continue;
            }
            let (x1, y1) = (
                members.iter().map(|c| c.0).max().unwrap(),
                members.iter().map(|c| c.1).max().unwrap(),
            );
            let (xa, ya) = (
                members.iter().map(|c| c.0).min().unwrap(),
                members.iter().map(|c| c.1).min().unwrap(),
            );
            out.push((
                members.len(),
                format!("{:>4} cells at x {xa}..{x1} y {ya}..{y1}, {liquid} liquid faces", members.len()),
            ));
        }
    }
    out.sort_by_key(|o| std::cmp::Reverse(o.0));
    out.into_iter().map(|(_, line)| line).collect()
}

/// Repaint `frame` with the load model's own verdict, cell by cell.
///
/// Three states, deliberately, because the model has three: material this
/// model has an opinion about (the green-to-red ramp), material it declines
/// to evaluate (dark blue), and no material at all (near-black). The middle
/// one is not cosmetic -- `is_structurally_interesting` skips anything
/// buried, so the inside of a thick wall is never load-tested, and painting
/// it the same green as "evaluated and fine" would hide the very thing this
/// channel was added to look at.
///
/// One shared `Cache` for the whole screen, for the reason
/// `load::evaluate_with_cache` documents: per-cell caches make this
/// O(region x subtree) and it is drawn over a whole world.
fn paint_stress(world: &World, frame: &mut [u8]) {
    let mut cache = pixel_physics::sim::load::Cache::default();
    let mut budget = u32::MAX;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let colour = match pixel_physics::sim::load::evaluate_with_cache(world, x, y, &mut cache, &mut budget) {
                // The app's ramp exactly (`App::draw_stress_overlay`), so
                // what a sheet shows and what the owner sees on `N` are the
                // same picture -- at alpha 1.0 rather than 0.55.
                Some(l) => {
                    let ratio = l.stress().clamp(0.0, 1.0);
                    [(40.0 + 215.0 * ratio) as u8, (220.0 * (1.0 - ratio)) as u8, 60, 255]
                }
                None if world.get(x, y).material == material::EMPTY => [12, 12, 16, 255],
                None => [30, 40, 90, 255],
            };
            let dst = (((y * WIDTH) + x) * 4) as usize;
            frame[dst..dst + 4].copy_from_slice(&colour);
        }
    }
}

/// Repaint `frame` with `weather::exposure` for the *ground* of each column.
///
/// **A column wash, and the first version was not.** Exposure is defined at
/// any point, so the obvious sheet paints every cell with its own value --
/// and that sheet is worthless. A cell inside rock has every upwind surface
/// above it and reads ~0.0; a cell in the sky has every upwind surface below
/// it and reads ~1.0; so the picture comes out a straw sky over a navy solid,
/// split on the ground line, and the terrain-driven variation that the
/// channel exists to show is a one-pixel band nobody can see. Rendered, it
/// is exactly the "reads as blank" failure `CLAUDE.md` warns produces a fix
/// aimed at working code -- the numbers under that same sheet were healthy
/// (spread 0.941), which is the whole argument for printing them.
///
/// What is painted instead is `weather::ground_exposure` for the column,
/// down the full height: the quantity `weather::gust` actually samples, and
/// the one that answers "is this a sheltered place". The surface row is
/// marked in a fixed cyan that is nowhere on the ramp, so the terrain
/// profile can be read against the wash without tinting it.
///
/// A **full replace on a fixed dark-to-bright ramp**, per `CLAUDE.md`, at
/// alpha 1.0: deep navy is sheltered, mid slate is level open ground
/// (`weather::NEUTRAL_EXPOSURE`), pale straw is fully exposed. Fixed rather
/// than normalised to the frame's own range -- a per-frame rescale makes a
/// world with no relief look exactly like one with plenty, which is the
/// failure this channel exists to catch, and the `preset=flat` control
/// would stop being a control.
fn paint_exposure(world: &World, frame: &mut [u8], wind: f32) {
    for x in 0..WIDTH {
        let surface = (0..HEIGHT).find(|&y| world.get(x, y).material != material::EMPTY);
        let e = pixel_physics::sim::weather::ground_exposure(world, x, wind)
            .unwrap_or(pixel_physics::sim::weather::NEUTRAL_EXPOSURE);
        // Two straight segments through the neutral point, so the ramp stays
        // monotone in exposure while level ground still lands on a
        // recognisable colour rather than an arbitrary grey.
        let colour = if e <= 0.5 {
            let t = e / 0.5;
            [(14.0 + 76.0 * t) as u8, (16.0 + 84.0 * t) as u8, (34.0 + 78.0 * t) as u8, 255]
        } else {
            let t = (e - 0.5) / 0.5;
            [(90.0 + 155.0 * t) as u8, (100.0 + 140.0 * t) as u8, (112.0 + 63.0 * t) as u8, 255]
        };
        for y in 0..HEIGHT {
            let dst = (((y * WIDTH) + x) * 4) as usize;
            let px = if Some(y) == surface { [0, 200, 220, 255] } else { colour };
            frame[dst..dst + 4].copy_from_slice(&px);
        }
    }
}

/// Which wind `channel=exposure` was drawn for, and the census that says
/// whether the channel is doing anything at all.
///
/// Printed next to the sheet because a picture cannot report its own
/// inputs, and an exposure sheet drawn at `wind = 0.02` on a world with no
/// relief is indistinguishable from a broken one. The spread in particular
/// is the "did it fire at all" counter `CLAUDE.md` asks for: a channel that
/// is working on rolling terrain has a wide one, and a flat world must
/// report ~0.000 with a mean at `NEUTRAL_EXPOSURE`.
fn report_exposure(world: &World, wind: f32) {
    let (mut lo, mut hi, mut sum, mut n) = (f32::MAX, f32::MIN, 0.0f64, 0u32);
    let (mut lo_x, mut hi_x) = (0, 0);
    for x in 0..WIDTH {
        let Some(s) = (0..HEIGHT).find(|&y| world.get(x, y).material != material::EMPTY) else { continue };
        let v = pixel_physics::sim::weather::exposure(world, x, s, wind);
        if v < lo {
            lo = v;
            lo_x = x;
        }
        if v > hi {
            hi = v;
            hi_x = x;
        }
        sum += v as f64;
        n += 1;
    }
    if n == 0 {
        println!("    exposure: no ground anywhere in this world");
        return;
    }
    println!(
        "    exposure (wind {wind:+.2}): ground columns {n}, mean {:.3}, sheltered {:.3} at x={lo_x}, exposed {:.3} at x={hi_x}, spread {:.3}",
        sum / n as f64,
        lo,
        hi,
        hi - lo
    );
}

/// Print whatever structural probes were asked for. Separate from the tile
/// line above because these scan the world and the tile line does not — a
/// scene that isn't about structure should pay nothing for this existing.
fn report_loads(world: &World, args: &Args) {
    for &(x, y) in &args.probes {
        match pixel_physics::sim::load::evaluate(world, x, y) {
            Some(l) => println!(
                "    load ({x},{y}): mass {} torque {} capacity {} stress {:.2}{}{}",
                l.mass,
                l.torque,
                l.capacity,
                l.stress(),
                if l.supported { "" } else { " UNSUPPORTED" },
                if l.truncated { " TRUNCATED" } else { "" },
            ),
            // Says *which* of the reasons, because "nothing here" covers
            // both "solid rock that cannot fail" and "this cell is gone",
            // and confusing those wastes a session.
            None => {
                let cell = world.get(x, y);
                let name = &world.materials.get(cell.material).name;
                println!("    load ({x},{y}): not evaluated -- {name}, aux {}, attached {}", cell.aux(), cell.attached());
            }
        }
    }
    if !args.loadmap {
        return;
    }
    let mut worst: Option<((i32, i32), pixel_physics::sim::load::Load)> = None;
    // Shared cache, for the reason the hanging census records: a whole-world
    // sweep of the uncached form is quadratic in the size of the piece.
    let mut cache = pixel_physics::sim::load::Cache::default();
    let mut budget = u32::MAX;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let Some(l) = pixel_physics::sim::load::evaluate_with_cache(world, x, y, &mut cache, &mut budget) else { continue };
            if worst.is_none_or(|(_, w)| l.stress() > w.stress()) {
                worst = Some(((x, y), l));
            }
        }
    }
    match worst {
        Some(((x, y), l)) => println!(
            "    worst stress: {:.2} at ({x},{y}) -- mass {} torque {} capacity {}{}",
            l.stress(),
            l.mass,
            l.torque,
            l.capacity,
            if l.supported { "" } else { " UNSUPPORTED" }
        ),
        None => println!("    worst stress: nothing evaluable in the world"),
    }
}

/// Assert what the scene was supposed to do, and exit non-zero if it did
/// not. Returns whether everything asked for held.
///
/// # Why the mechanism is asserted and not just the outcome
///
/// Because "it still stands" is true of a structure that is standing and
/// equally true of one nothing ever looked at. That is not hypothetical:
/// `scene=capped` was recorded as passing its acceptance case -- "the
/// thick column still stands, worst stress 0.00" -- while the whole
/// 15,840-cell structure was frozen, every cell still at `aux 0`, and not
/// one of them had ever been load-evaluated. The assertion was true and
/// meant nothing, which is `CLAUDE.md`'s vacuous-test failure arriving in
/// the acceptance harness instead of the test suite.
///
/// So a scene that is supposed to collapse must show the *criterion
/// firing* (`min_overloaded`), not merely that material moved; and a scene
/// that is supposed to stand must show that nothing fired
/// (`max_failures`), which is only meaningful once the same binary has
/// demonstrated it can fire at all on the collapsing scenes.
fn check_expectations(world: &World, args: &Args, gnome: &Gnome, best_ms: f64, peaks: (usize, usize), cells_before: (i64, i64), cave_before: i64) -> bool {
    let (peak_bodies, peak_tissue) = peaks;
    let f = world.structural_failures;
    let mut ok = true;
    if let Some(pct) = args.min_cave {
        let now = roofed_void(world);
        let kept = if cave_before == 0 { 0 } else { now * 100 / cave_before };
        if kept < pct {
            println!("  FAIL: the cave did not survive -- {kept}% of its roofed void left ({now} of {cave_before} cells), wanted {pct}%");
            ok = false;
        }
    }
    if let Some((row, most)) = args.max_rock_above {
        let left = (0..row)
            .flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let cell = world.get(x, y);
                !cell.attached() && world.materials.kind(cell.material) == MaterialKind::Solid
            })
            .count();
        if left > most {
            println!("  FAIL: {left} loose solid cells are still above row {row}, wanted at most {most}");
            ok = false;
        }
    }
    if let Some(pct) = args.max_cave {
        let now = roofed_void(world);
        let kept = if cave_before == 0 { 100 } else { now * 100 / cave_before };
        if kept > pct {
            println!("  FAIL: the cave should have come down -- {kept}% of its roofed void left ({now} of {cave_before} cells), wanted at most {pct}%");
            ok = false;
        }
    }
    if let Some(max) = args.max_lost {
        let lost = (cells_before.0 + cells_before.1) - occupied(world);
        if lost > max {
            println!("  FAIL: expected the world to lose at most {max} cells after the cut, lost {lost}");
            ok = false;
        }
    }
    if let Some(limit) = args.max_frame_ms {
        if best_ms > limit {
            println!("  FAIL: worst frame {best_ms:.2} ms over the {limit:.1} ms budget (best of {} runs)", args.repeat);
            ok = false;
        }
    }
    if let Some(max) = args.max_sites {
        let pending = world.active_site_count();
        if pending > max {
            println!(
                "  FAIL: the structural scheduler still has {pending} sites pending at the end of the run, wanted at most {max} -- see open-bugs-handoff.md §S, a backlog that climbs instead of draining"
            );
            ok = false;
        }
    }
    if let Some(min) = args.min_bodies {
        if peak_bodies < min {
            println!("  FAIL: expected at least {min} chunk bodies in flight at once, peaked at {peak_bodies}");
            ok = false;
        }
    }
    if let Some((min_cols, max_cells)) = args.ice {
        let (frozen_cols, total_cols, _, _) = frozen_surface(world);
        let ice_cells = water_census(world).1;
        if frozen_cols < min_cols {
            println!("  FAIL: expected at least {min_cols} of {total_cols} water columns frozen at the surface, got {frozen_cols}");
            ok = false;
        }
        if ice_cells > max_cells {
            println!("  FAIL: expected at most {max_cells} cells of ice -- a sheet, not a solid pond -- got {ice_cells}");
            ok = false;
        }
    }
    if let Some(min) = args.min_travelled {
        let went = gnome.travelled(world);
        if went < min {
            println!("  FAIL: expected the gnome to cover at least {min} cells, he covered {went}");
            ok = false;
        }
    }
    if let Some(min) = args.min_failing_cells {
        let cells = f.overloaded_cells + f.unsupported_cells;
        if cells < min {
            println!("  FAIL: expected structural failures to take at least {min} cells, they took {cells}");
            ok = false;
        }
    }
    if let Some(min) = args.min_severed {
        if f.severed_organism_cells < min {
            println!(
                "  FAIL: expected the support check to sever at least {min} cells of living tissue, it severed {}",
                f.severed_organism_cells
            );
            ok = false;
        }
    }
    if let Some(min) = args.min_overloaded {
        if f.overloaded < min {
            println!("  FAIL: expected at least {min} overload failures, got {}", f.overloaded);
            ok = false;
        }
    }
    if let Some(max) = args.max_failures {
        let total = f.overloaded + f.unsupported;
        if total > max {
            println!("  FAIL: expected at most {max} structural failures, got {total}");
            ok = false;
        }
    }
    // Printed unconditionally, not only when `min_bodies` is set: "did any
    // piece actually come away" is the counter half of every destruction
    // sheet (`CLAUDE.md`, "did it fire at all" needs a counter), and a body
    // whose whole life falls between two tiles is invisible in the per-tile
    // lines above.
    println!("  peak chunk bodies in flight at once: {peak_bodies}");
    // Printed unconditionally beside it, and only ever non-zero on a scene
    // with an organism in it: this is felling's "did it fire" counter, and
    // the one the per-tile census cannot carry.
    println!("  peak cells of plant tissue riding in those bodies: {peak_tissue}");
    println!(
        "  of {} cells the support check severed, {} left as pieces",
        world.structural_failures.severed_organism_cells, world.structural_failures.severed_organism_pieces
    );
    if let Some(max) = args.max_unconfined {
        // `confined` counts failures of *either* mode whose region had no
        // free face (`structural.rs` records the confined-unsupported
        // "pocket wedged in its own hole" case too), so it subtracts from
        // the combined total, not from `overloaded` alone. Saturating
        // because the invariant confined <= total is structural.rs's, not
        // this harness's, to enforce.
        let unconfined = (f.overloaded + f.unsupported).saturating_sub(f.confined);
        if unconfined > max {
            println!(
                "  FAIL: expected at most {max} unconfined structural failures, got {unconfined} ({} overloaded of which {} confined, {} unsupported)",
                f.overloaded, f.confined, f.unsupported
            );
            ok = false;
        }
    }
    if ok
        && (args.max_cave.is_some()
            || args.max_rock_above.is_some()
            || args.min_overloaded.is_some()
            || args.min_failing_cells.is_some()
            || args.min_severed.is_some()
            || args.max_failures.is_some()
            || args.max_unconfined.is_some()
            || args.max_frame_ms.is_some()
            || args.max_sites.is_some()
            || args.min_bodies.is_some()
            || args.ice.is_some()
            || args.min_travelled.is_some())
    {
        println!("  OK: scene={} met its expectations", args.scene);
    }
    ok
}

fn main() {
    let args = parse();
    // **Echo the world clock, always.** `CLAUDE.md`'s harness rule: a knob
    // whose value you cannot see is a knob you cannot tell is disconnected,
    // and a contact sheet that does not name its own clock was rendered by a
    // binary that may never have had one. Printed even at the baseline, so a
    // sheet whose caption is missing this line is from an older build.
    let c = &args.clock;
    println!(
        "filmstrip: scene={} clock(day={} weather={} growth={} creatures={} gnome={})",
        args.scene, c.day_minutes, c.weather_slowdown, c.growth_slowdown, c.creature_slowdown, c.gnome_slowdown
    );
    // Repeated runs are for the *timing* only -- the image and the
    // expectations come from the last one, which is a full run like any
    // other. Deliberately re-simulated from scratch rather than reusing a
    // warm world, since a second pass over an already-settled scene
    // measures nothing.
    let mut samples: Vec<f64> = Vec::new();
    for _ in 1..args.repeat {
        samples.push(run_once(&args, false).0);
    }
    let (last_ms, world, gnome, peaks, cells_before, cave_before) = run_once(&args, true);
    samples.push(last_ms);
    let best = samples.iter().cloned().fold(f64::INFINITY, f64::min);
    if args.repeat > 1 {
        let worst = samples.iter().cloned().fold(0.0, f64::max);
        println!("worst frame over {} runs: {best:.2} ms (spread {best:.2}-{worst:.2})", args.repeat);
    }
    if !check_expectations(&world, &args, &gnome, best, peaks, cells_before, cave_before) {
        std::process::exit(1);
    }
}

/// Gutter between tiles, in pixels, and the mid-grey it is filled with --
/// so a tile that is legitimately all-black stays distinguishable from the
/// space between tiles. Shared by the main sheet and the `panels=` one,
/// because two sheets read side by side with different gutters read as two
/// different instruments.
const TILE_GAP: i32 = 2;
const GUTTER_GREY: u8 = 90;

/// Blit one `crop`-sized window of a rendered world frame into `dst` at
/// `(ox, oy)`, magnified `zoom`x with nearest-neighbour replication.
///
/// One copy, used by the contact sheet, the GIF branch and the `panels=`
/// sheet. It was three copies of the same nested loop; the third was the
/// one that would have drifted.
fn blit_tile(dst: &mut [u8], dst_w: i32, (ox, oy): (i32, i32), frame: &[u8], crop: Rect, zoom: i32) {
    for y in 0..crop.height() {
        for x in 0..crop.width() {
            let (sx, sy) = (crop.min_x + x, crop.min_y + y);
            if sx < 0 || sy < 0 || sx >= WIDTH || sy >= HEIGHT {
                continue;
            }
            let src = (((sy * WIDTH) + sx) * 4) as usize;
            for zy in 0..zoom {
                for zx in 0..zoom {
                    let (dx, dy) = (ox + x * zoom + zx, oy + y * zoom + zy);
                    let d = (((dy * dst_w) + dx) * 4) as usize;
                    dst[d..d + 4].copy_from_slice(&frame[src..src + 4]);
                }
            }
        }
    }
}

/// The `W`x`H` window centred on a charge, **shifted** inside the world
/// rather than shrunk when it would run off an edge: every tile of a grid
/// has to be the same size or the grid does not compose, so a charge near
/// the world edge gets an off-centre view and never a small one.
fn panel_crop(x: i32, y: i32, w: i32, h: i32) -> Rect {
    let min_x = (x - w / 2).clamp(0, (WIDTH - w).max(0));
    let min_y = (y - h / 2).clamp(0, (HEIGHT - h).max(0));
    Rect::new(min_x, min_y, min_x + w - 1, min_y + h - 1)
}

/// `out`'s path with `-panels` inserted before the extension.
fn panels_path(out: &str) -> String {
    match out.rfind('.') {
        Some(i) if !out[i..].contains('/') && !out[i..].contains('\\') => {
            format!("{}-panels{}", &out[..i], &out[i..])
        }
        _ => format!("{out}-panels"),
    }
}

/// The `panels=` sheet in progress: one column per charge fired, one row
/// per age, each cell a crop centred on that charge's own site and captured
/// that many frames after **that charge's own detonation**.
struct PanelSheet {
    /// Crop size in world cells.
    w: i32,
    h: i32,
    ages: Vec<usize>,
    cols: usize,
    zoom: i32,
    sheet: Vec<u8>,
    sheet_w: i32,
    sheet_h: i32,
    /// Which grid cells have already been filled. Needed because the outer
    /// capture loop revisits a tile boundary frame -- `fire_due_*` is called
    /// once at `step_no == target` and again at the top of the next inner
    /// loop -- so without this a cell on such a frame would be rendered and
    /// blitted twice.
    filled: Vec<bool>,
    /// Its own `Renderer` and frame buffer, deliberately **not** the main
    /// sheet's. `Renderer::draw` advances an internal frame counter that the
    /// animated grain modes read, so sharing one would make the main sheet a
    /// function of how many panel captures happened to fall between its
    /// tiles. The main sheet has to stay byte-identical to a run without
    /// `panels=`, and this is how that is guaranteed rather than argued.
    renderer: Renderer,
    frame: Vec<u8>,
}

impl PanelSheet {
    fn new(args: &Args, w: i32, h: i32, ages: Vec<usize>, cols: usize) -> Self {
        let (tile_w, tile_h) = (w * args.zoom, h * args.zoom);
        let rows = ages.len() as i32;
        let sheet_w = cols as i32 * tile_w + (cols as i32 - 1) * TILE_GAP;
        let sheet_h = rows * tile_h + (rows - 1) * TILE_GAP;
        let mut sheet = vec![GUTTER_GREY; (sheet_w * sheet_h * 4) as usize];
        for p in sheet.chunks_exact_mut(4) {
            p[3] = 255;
        }
        let mut renderer = Renderer::new();
        renderer.grain = args.grain;
        renderer.organism_overlay = args.organism_overlay;
        renderer.field_overlay = args.field_overlay;
        renderer.pinned_light = args.daylight.map(pixel_physics::sky::frame_for_daylight);
        Self {
            w,
            h,
            filled: vec![false; ages.len() * cols],
            ages,
            cols,
            zoom: args.zoom,
            sheet,
            sheet_w,
            sheet_h,
            renderer,
            frame: vec![0u8; (WIDTH * HEIGHT * 4) as usize],
        }
    }

    /// Fill every grid cell whose moment is this frame. Renders the world at
    /// most once per frame and only when something actually wants it.
    fn capture(&mut self, world: &World, particles: &ParticleSystem, fired: &[FiredCharge], step_no: usize) {
        let mut drawn = false;
        for (col, c) in fired.iter().enumerate().take(self.cols) {
            for row in 0..self.ages.len() {
                let idx = row * self.cols + col;
                if self.filled[idx] || c.frame + self.ages[row] != step_no {
                    continue;
                }
                if !drawn {
                    // `force_full`, for the same reason the main sheet uses
                    // it: this must draw the whole world regardless of what
                    // moved, or a tile inherits pixels from whichever frame
                    // last touched them. The empty `touched` set is not a
                    // shortcut -- `force_full` makes it unread.
                    self.renderer.draw(world, particles, &HashSet::new(), &mut self.frame, (WIDTH as u32, HEIGHT as u32), true);
                    drawn = true;
                }
                let (tile_w, tile_h) = (self.w * self.zoom, self.h * self.zoom);
                let origin = (col as i32 * (tile_w + TILE_GAP), row as i32 * (tile_h + TILE_GAP));
                blit_tile(&mut self.sheet, self.sheet_w, origin, &self.frame, panel_crop(c.x, c.y, self.w, self.h), self.zoom);
                self.filled[idx] = true;
            }
        }
    }
}

/// **What the animals actually did, printed beside the picture.**
///
/// `CLAUDE.md`'s house rule for the review queue is that the discrete event
/// count goes in the card's `meta`, because two very different mechanisms
/// look identical at the zoom a contact sheet is read at -- a collapse once
/// read as "chunks are working" came from a run whose body count was zero
/// throughout. This example rendered creature scenes for months while
/// printing no creature counter at all, so a colony card's `meta` had to be
/// copied from some *other* harness's run of a *different* world, which is
/// the same defect one level up.
///
/// **Called from the GIF branch too**, which is the one that matters: the
/// GIF branch is what produces the animation a review card is built from,
/// and it returns before the contact-sheet path. Its comment says it
/// "reports neither" timing nor body samples, and that stays true -- this
/// is not a measurement of the run, it is the label on the picture.
///
/// **Silent unless the scene actually has animals**, and `live_organism_count`
/// is not that test: it counts plants too, so gating on it printed
/// `colony: 10 live | moves 0` under a *tree* sheet -- a counter that reports
/// on something the scene does not contain is the same defect as a metric that
/// counts droplets and calls them films. Creature activity is the tell; a
/// colony scene that placed ants always moves, and one that placed none is
/// caught by the scene's own assertion rather than by this line.
fn report_colony(world: &World, render: bool) {
    // **`moves` alone is the wrong gate now, and `scene=hop` is why.** A
    // creature that only ever launches never walks, so a hop sheet with four
    // animals in mid-air printed nothing at all -- the picture with no
    // number beside it that `CLAUDE.md` opens by warning about. Any sign of
    // creature activity will do; what must not happen is a scene full of
    // animals reporting silence.
    if !render || (world.creature_stats.moves == 0 && world.creature_stats.impulses == 0) {
        return;
    }
    let st = world.creature_stats;
    let blocked_frac = if st.moves > 0 { st.moves_blocked as f64 / st.moves as f64 } else { 0.0 };
    println!(
        "  creatures: {} live organism(s) | moves {} blocked {} ({blocked_frac:.3}) falls {} | pickups {} drops {} deliveries {} deaths {}",
        world.live_organism_count(),
        st.moves,
        st.moves_blocked,
        st.falls,
        st.pickups,
        st.drops,
        st.deliveries,
        st.deaths
    );
    println!(
        "  creatures: forage trips {} (bar {}) deepest {} | reach {:?}",
        st.forage_trips,
        pixel_physics::sim::creature::FORAGE_TRIP_MIN,
        st.forage_depth_max,
        st.forage_reach
    );
    // **The verb's own counters, printed under every creature sheet.** A
    // creature arcing through the air and one falling off a ledge are the
    // same photograph; `impulses` is the only thing that says which, and
    // `refused` is its effect-side pair. Zero here under a hop sheet means
    // the picture is of something else.
    println!(
        "  creatures: impulses {} (refused {}) | airborne frames {} | flight moves {}",
        st.impulses, st.impulses_refused, st.flight_frames, st.flight_moves
    );
}

/// One full run. Returns its worst frame in ms, the finished world, the
/// peak concurrent body count and how much material the world held *before*
/// the first step. `render` is false for the extra timing samples, which do
/// not need an image and should not pay for one.
fn run_once(args: &Args, render: bool) -> (f64, World, Gnome, (usize, usize), (i64, i64), i64) {
    let mut world = build(args);
    // After `build`, which may construct the world several different ways --
    // one place to set it means a scene cannot silently opt out.
    world.clock = args.clock;
    // Censused before the first step and after the last, because a failure
    // count cannot answer "how much did this eat" -- see `Args::max_lost`.
    // Taken here rather than in `build` so it includes whatever the scene
    // cut on construction: the dig is part of what the run costs.
    let cells_before = census(&world);
    let cave_before = roofed_void(&world);
    // The whole material grid, not a total. `census` answers *how much* was
    // lost; this answers *where*, which is the containment question and the
    // one nothing in the engine could answer honestly -- see
    // `damage_radius`. One `Vec<MaterialId>` over a 512x320 world is 320 kB
    // and is taken once per run.
    let materials_before: Vec<material::MaterialId> = (0..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| (x, y))).map(|(x, y)| world.get(x, y).material).collect();
    let mut renderer = Renderer::new();
    renderer.grain = args.grain;
    renderer.bubbles = args.bubbles;
    renderer.gas = args.gas;
    renderer.tree_depth = args.tree_depth;
    renderer.organism_overlay = args.organism_overlay;
    renderer.field_overlay = args.field_overlay;
    renderer.sky_light = args.sky_light;
    renderer.pinned_light = args.daylight.map(pixel_physics::sky::frame_for_daylight);
    let mut particles = ParticleSystem::new();
    let mut pending = args.explosions.clone();
    let mut pending_blasts = args.blasts.clone();
    // Where each charge actually went off, in fire order. `blast=`'s site
    // is not known until it fires, so this cannot be read off `Args`.
    let mut fired: Vec<FiredCharge> = Vec::new();
    // Whether this run prints the **per-charge** report and census lines.
    //
    // Gated rather than unconditional, and not to flatter a diff: with a
    // single `explode=` charge every one of those lines is a restatement of
    // a line already printed (`blast report:` and the boxed
    // `cracked cells within 3x radius of ...` are both about that one
    // charge), and every measurement recorded in
    // `Reports/explosion-stone-review.md` §8-§13 was taken from exactly
    // such a run. Adding a duplicate line to all of them buys nothing and
    // changes every recorded baseline. The lines earn their place the
    // moment a run fires more than one charge, which is what they were
    // built for -- and `blast=` is never a single-charge idiom in practice,
    // so it opts in on its own.
    let per_charge_reports = !args.blasts.is_empty() || args.explosions.len() > 1;
    let mut pending_cuts = args.cuts.clone();
    let mut pending_chops = args.chops.clone();
    let mut pending_fell = args.fell;
    let mut pending_depowder = args.depowder;
    let mut depowder_first = true;
    let mut pending_pokes = args.pokes.clone();
    let mut pending_ignitions = args.ignitions.clone();
    let mut pending_dries = args.dries.clone();
    let mut blasts = explosion::Blasts::new();
    if let Some(v) = args.joint_reach {
        blasts.tuning.joint_reach = v;
    }
    if let Some(v) = args.joint_open {
        blasts.tuning.joint_open_fraction = v;
    }
    // **First**, so every override below still wins over it -- a preset is a
    // starting point, not a lock.
    if let Some(name) = args.charge.as_deref() {
        let want = name.trim().to_ascii_lowercase();
        let found = explosion::Preset::ALL.into_iter().find(|p| p.label().eq_ignore_ascii_case(&want));
        let Some(p) = found else {
            let names: Vec<&str> = explosion::Preset::ALL.iter().map(|p| p.label()).collect();
            panic!("unknown charge {name:?}; known: {}", names.join(", "));
        };
        blasts.tuning = p.tuning();
        println!("  charge: {} -- radius {} strength {}", p.label(), blasts.tuning.radius, blasts.tuning.strength);
    }
    if let Some(v) = args.joint_seam_width {
        blasts.tuning.joint_seam_width = v.max(1);
    }
    if let Some(v) = args.joint_density {
        blasts.tuning.joint_density = v;
    }
    if let Some(v) = args.crack_rays {
        blasts.tuning.crack_rays = v;
    }
    if let Some(v) = args.smoke {
        blasts.tuning.smoke_fraction = v;
    }
    let mut gnome = Gnome::for_scene(&args.scene, args.dig_yield, args.shoulder_grains);
    // Set on the character rather than passed to `dig`: the style is his
    // state, exactly as it is in the app, so the harness and the game reach
    // the mechanism through the same door.
    if let Some(p) = world.player.as_mut() {
        p.dig_style = args.dig_style;
    }
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];

    let (cw, ch) = (args.crop.width(), args.crop.height());
    let (tile_w, tile_h) = (cw * args.zoom, ch * args.zoom);
    let gap = TILE_GAP;
    let rows = args.count.div_ceil(args.cols) as i32;
    let sheet_w = args.cols as i32 * tile_w + (args.cols as i32 - 1) * gap;
    let sheet_h = rows * tile_h + (rows - 1) * gap;
    // Mid-grey gutters, so a tile that is legitimately all-black stays
    // distinguishable from the space between tiles.
    let mut sheet = vec![GUTTER_GREY; (sheet_w * sheet_h * 4) as usize];
    for p in sheet.chunks_exact_mut(4) {
        p[3] = 255;
    }

    // GIF branch: motion is for a human to watch, and several of these
    // artifacts (a fringe that regenerates every frame, water that reads as
    // frozen because its grain is nailed to the screen) simply do not survive
    // being sampled into stills. Consecutive frames, real playback speed, and
    // a NETSCAPE loop -- the same reasoning `main.rs`'s capture hook records.
    // `image::save_buffer` below picks its encoder from the file extension, so
    // `out=x.gif` without `gif=1` silently writes the whole contact sheet as a
    // ONE-FRAME gif. It is a valid file, it is named like an animation, and it
    // cannot move -- which is exactly how two review cards shipped as stills
    // while the agent reported posting an animation. Refuse the combination
    // rather than producing the thing nobody wanted.
    if !args.gif && args.out.to_ascii_lowercase().ends_with(".gif") {
        panic!(
            "out={} ends in .gif but gif=1 was not passed. The contact sheet \
             would be written as a single-frame gif that cannot animate. \
             Add gif=1 for an animation, or use a .png name for a sheet.",
            args.out
        );
    }

    if args.gif {
        let mut frames = Vec::with_capacity(args.count);
        let mut step_no = 0usize;
        // Local to this branch, which returns before the sheet path's own
        // pair is declared. See the sampling site inside the loop for why a
        // gif run has to keep them at all.
        let (mut peak_bodies, mut peak_tissue) = (0usize, 0usize);
        for i in 0..args.count {
            let target = args.start + i * args.every;
            while step_no < target {
                fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
                fire_due_cuts(&mut world, &mut pending_cuts, step_no);
                fire_due_chops(&mut world, &mut pending_chops, step_no);
                fire_due_fell(&mut world, &mut pending_fell, step_no);
                fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
                fire_due_pokes(&mut world, &mut pending_pokes, step_no);
                fire_due_dries(&mut world, &mut pending_dries, step_no);
                fire_due_ignitions(&mut world, &mut pending_ignitions, step_no);
                advance(&mut world, &mut particles, &mut blasts, args.parallel_driver, step_no, &mut gnome, per_charge_reports);
                // **A GIF cannot carry its own counts, so the run has to
                // print them.** This branch used to sample nothing and
                // return a hardcoded zero, so the summary line reported
                // `peak chunk bodies in flight at once: 0` on a run that in
                // fact peaked at 22 -- and the house rule that the discrete
                // event count goes in the review card's `meta` then needed
                // a whole second, non-gif run at the same span to source a
                // number the gif run already had.
                // `Reports/physical-trees-design-2026-08-23.md` §9.5.
                peak_bodies = peak_bodies.max(world.chunk_bodies.len());
                peak_tissue = peak_tissue.max(world.chunk_bodies.iter().flat_map(|b| b.cells.iter()).filter(|c| c.organism_id != 0).count());
                step_no += 1;
            }
            fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
            fire_due_cuts(&mut world, &mut pending_cuts, step_no);
            fire_due_chops(&mut world, &mut pending_chops, step_no);
            fire_due_fell(&mut world, &mut pending_fell, step_no);
            fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
            fire_due_pokes(&mut world, &mut pending_pokes, step_no);
            fire_due_dries(&mut world, &mut pending_dries, step_no);
                fire_due_ignitions(&mut world, &mut pending_ignitions, step_no);
            let touched: HashSet<_> = world.take_touched_chunks();
            renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH as u32, HEIGHT as u32), true);
            if args.stress {
                paint_stress(&world, &mut frame);
            }
            if args.exposure {
                let wind = args.wind.unwrap_or_else(|| pixel_physics::sim::weather::at(world.seed, world.frame).wind);
                paint_exposure(&world, &mut frame, wind);
            }

            let mut tile = vec![0u8; (tile_w * tile_h * 4) as usize];
            blit_tile(&mut tile, tile_w, (0, 0), &frame, args.crop, args.zoom);
            frames.push(tile);
        }

        // 60 ticks/second is the sandbox's own fixed simulation rate, so
        // `every` ticks between captures maps directly to elapsed time.
        let delay_ms = (args.every as u64 * 1000) / 60;
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms.max(16)));
        let file = std::fs::File::create(&args.out).expect("creating the gif");
        let mut encoder = image::codecs::gif::GifEncoder::new(file);
        encoder.set_repeat(image::codecs::gif::Repeat::Infinite).expect("gif repeat");
        for tile in frames {
            let buf = image::RgbaImage::from_raw(tile_w as u32, tile_h as u32, tile).expect("gif frame");
            encoder.encode_frame(image::Frame::from_parts(buf, 0, 0, delay)).expect("gif frame");
        }
        drop(encoder);
        println!("animated gif ({tile_w}x{tile_h}, {} frames): {}", args.count, args.out);
        // **Counts, printed here rather than left to a second run.** The
        // branch still does no per-frame *timing* -- it renders every
        // captured frame and plays at real speed, so its worst frame is a
        // measurement of this harness rather than of the engine, and
        // `repeat=`/`max_frame_ms` genuinely do not apply. Counts are a
        // different thing: they are the world's, not the harness's, and
        // withholding them is what made a gif card unable to say whether
        // the mechanism it was showing had fired at all.
        //
        // The world census, not the per-tile block: the tiles are frames of
        // an animation here and there is no "tile 3" to hang it under.
        let living: usize = (0..WIDTH)
            .flat_map(|x| (0..HEIGHT).map(move |y| (x, y)))
            .filter(|&(x, y)| world.get(x, y).organism_id() != 0)
            .count();
        if living > 0 {
            FellCensus::of(&world).print(&world);
        }
        let f = &world.structural_failures;
        let failed_cells = f.overloaded_cells + f.unsupported_cells;
        println!(
            "    crumbled to grit (region < MIN_FRACTURE_CELLS): {} regions, {} cells of {} failed ({:.0}%)",
            f.crumbled,
            f.crumbled_cells,
            failed_cells,
            if failed_cells == 0 { 0.0 } else { 100.0 * f.crumbled_cells as f64 / failed_cells as f64 }
        );
        println!("    what came off: {} cells as chunks, {} as dust", f.promoted_cells, f.shattered_cells);
        // The colony report is main's own half of the same argument -- a
        // gif run that cannot say what its ants did is the identical
        // footgun -- and both sides of this merge fixed the branch's
        // silence for a different counter. Both kept.
        report_colony(&world, render);
        return (0.0, world, gnome, (peak_bodies, peak_tissue), cells_before, cave_before);
    }

    // `panels=`: a second sheet, one column per charge and one row per age.
    // Built after the GIF branch has had its chance to return, because that
    // branch writes an animation rather than a grid and has no second sheet
    // to write.
    let charges = args.explosions.len() + args.blasts.len();
    let mut panels = match (&args.panels, render) {
        (Some((pw, ph, ages)), true) => {
            assert!(charges > 0, "panels= needs at least one explode= or blast= charge -- a column per charge is the whole grid");
            Some(PanelSheet::new(args, *pw, *ph, ages.clone(), charges))
        }
        _ => None,
    };
    // The last frame any panel wants a picture of. A charge fired at 3400
    // and sampled 900 frames later needs 4300, which the main sheet's own
    // `start`/`every`/`count` knows nothing about -- so the run has to be
    // extended to reach it. Computed for the timing-only repeats as well as
    // the rendered run, so `repeat=` compares runs of the same length.
    let panel_last_frame = args.panels.as_ref().map(|(_, _, ages)| {
        let last_age = ages.iter().copied().max().unwrap_or(0);
        let last_charge = args.explosions.iter().map(|c| c.4).chain(args.blasts.iter().map(|c| c.4)).max().unwrap_or(0);
        last_charge + last_age
    });
    // Contact sheets draw every tile under the same light -- see
    // `pin_sheet_light`. The GIF branch above deliberately does not: it plays
    // at real speed, so its day/night swing is the world's own.
    if pin_sheet_light(args) && renderer.pinned_light.is_none() {
        // Phase 0 of the cycle is noon -- `field::sun_rising` runs noon,
        // sunset, midnight, sunrise.
        //
        // **Only when nothing asked for a specific hour.** The merge put two
        // independently-built pins on one field -- this automatic one and
        // `daylight=`, which is set above -- and without the guard the
        // automatic one lands last and silently overrides the explicit
        // request, while the run still announces the daylight it was asked
        // for. A sheet that says it was drawn at dusk and was drawn at noon
        // is worse than an unpinned one.
        renderer.pinned_light = Some(0);
    }

    let mut captured = 0usize;
    let mut step_no = 0usize;
    // Worst *single* frame, not the mean, and reported per tile rather than
    // once at the end. `Reports/fracture-mechanics-design.md` §3.4 names the
    // cascade spike as the thing to watch -- each break changes loads and
    // triggers more breaks in the same frame -- and a mean over 250 frames
    // hides exactly that. Per tile, because it localizes the spike to a
    // phase of the scene instead of just proving one happened.
    let mut worst_ms = 0.0f64;
    let mut worst_draw_ms = 0.0f64;
    let mut worst_frame = 0usize;
    // Sampled every frame, not just at capture: a body's whole life can
    // fall between two tiles, and "bodies 0" in every tile of a scene that
    // visibly threw rock is exactly the confusion this harness exists to
    // prevent.
    let mut peak_bodies = 0usize;
    // **Cells of living tissue riding in bodies, at the busiest instant of
    // the run**, sampled every frame for exactly the reason `peak_bodies`
    // is: a felled crown's whole flight can fall between two tiles, and the
    // per-tile line then reads "bodies carrying plant material 0 of 0" on a
    // run where a tree came down in pieces -- which is indistinguishable
    // from the pre-T1 behaviour it is supposed to show is gone. The last
    // tile of `scene=fell` is *after* everything landed, so the honest zero
    // there says nothing at all about whether it fired.
    let mut peak_tissue = 0usize;
    // Peak speed reached by a body in each size class -- see the loop that
    // fills it, and `rigid::SINK_DRAG_COEFFICIENT` for the curve it is the
    // readout for.
    let mut peak_by_size = [0.0f32; SIZE_BUCKETS.len()];
    // **How fast the fastest piece ever went, and when the first and last
    // of them came to rest.** Reported from play as *"a first group of
    // chunks that drop too fast and then the rest that come together with
    // the grit later"* -- which is two quantities, a peak speed and an
    // arrival spread, and the per-tile sample cannot give either: it sees
    // whichever bodies happen to be alive at that instant.
    let mut peak_speed = 0.0f32;
    let (mut first_rest, mut last_rest) = (None::<usize>, 0usize);
    let mut was_flying = false;
    // The bank at the previous tile, so the census can print a *rate* beside
    // the standing total. `None` on the first tile, which prints +0.00 rather
    // than a delta against a number that does not exist.
    let mut last_bank: Option<f64> = None;
    // **The floor's depth, which is the quantity the abscission complaint
    // actually named** -- "creating a giant pile of soil". Soil has no exit
    // channel (§O), so every soil cell decay writes is still there, and the
    // rise since the first sample IS the manufactured floor. Counted as an
    // absolute, deliberately: the neighbouring census records that an
    // earlier *exposed*-soil version was confounded because a leafier world
    // covers more ground, moving the denominator with the treatment. A bare
    // count has no denominator to move.
    let mut first_soil: Option<usize> = None;
    // Cross-tile state for the ice churn readout -- see the `ice:` line.
    let mut last_ice: Option<(u32, u32, i64)> = None;
    while captured < args.count {
        let target = args.start + captured * args.every;
        while step_no < target {
            fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
            fire_due_cuts(&mut world, &mut pending_cuts, step_no);
            fire_due_chops(&mut world, &mut pending_chops, step_no);
            fire_due_fell(&mut world, &mut pending_fell, step_no);
            fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
            fire_due_pokes(&mut world, &mut pending_pokes, step_no);
            fire_due_dries(&mut world, &mut pending_dries, step_no);
                fire_due_ignitions(&mut world, &mut pending_ignitions, step_no);
            // Outside the timed region below on purpose: a panel capture is
            // harness cost, and folding it into the worst-frame number would
            // make the sheet's own instrument report the sheet.
            if let Some(p) = panels.as_mut() {
                p.capture(&world, &particles, &fired, step_no);
            }
            let began = std::time::Instant::now();
            advance(&mut world, &mut particles, &mut blasts, args.parallel_driver, step_no, &mut gnome, per_charge_reports);
            let ms = began.elapsed().as_secs_f64() * 1000.0;
            // Frame 0 is excluded, and not to flatter the number: every
            // scene spikes there, including `terrain`, which runs no
            // structural work at all. It is chunk and field allocation plus
            // first-touch page faults, paid once at startup, and leaving it
            // in made all seven scenes report the same ~70-110 ms and hid
            // the differences between them entirely.
            peak_bodies = peak_bodies.max(world.chunk_bodies.len());
            peak_tissue = peak_tissue.max(world.chunk_bodies.iter().flat_map(|b| b.cells.iter()).filter(|c| c.organism_id != 0).count());
            for b in &world.chunk_bodies {
                let speed = (b.vx * b.vx + b.vy * b.vy).sqrt();
                peak_speed = peak_speed.max(speed);
                // Bucketed by size and kept as a *peak*, because a terminal
                // velocity is a cap and only a body that reached its cap is
                // evidence about where the cap is. The per-tile `by size`
                // line samples an instant and is confounded by exactly the
                // bodies that matter most: a big piece already resting on
                // the pile reads slow, which inverts the ordering and makes
                // a working size term look backwards. Measured once as
                // "smallest 4 cells across at 0.95, largest 10 across at
                // 0.18" on a build whose terminal genuinely rose with size.
                if submerged(&world, b) {
                    let e = body_extent_of(b) as usize;
                    let bucket =
                        SIZE_BUCKETS.iter().position(|&hi| e <= hi).unwrap_or(SIZE_BUCKETS.len() - 1);
                    peak_by_size[bucket] = peak_by_size[bucket].max(speed);
                }
            }
            let flying = !world.chunk_bodies.is_empty();
            if was_flying && !flying {
                first_rest.get_or_insert(step_no);
                last_rest = step_no;
            }
            was_flying = flying;
            if ms > worst_ms && step_no > 0 {
                worst_ms = ms;
                worst_frame = step_no;
            }
            step_no += 1;
        }
        fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
        fire_due_cuts(&mut world, &mut pending_cuts, step_no);
        fire_due_chops(&mut world, &mut pending_chops, step_no);
        fire_due_fell(&mut world, &mut pending_fell, step_no);
        fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
        fire_due_pokes(&mut world, &mut pending_pokes, step_no);
        fire_due_dries(&mut world, &mut pending_dries, step_no);
                fire_due_ignitions(&mut world, &mut pending_ignitions, step_no);
        if let Some(p) = panels.as_mut() {
            p.capture(&world, &particles, &fired, step_no);
        }
        // `force_full`, not the dirty-rect path: this must draw the whole
        // world every time regardless of what moved, or a tile would inherit
        // pixels from whichever frame last touched them.
        let touched: HashSet<_> = world.take_touched_chunks();
        // **Timed separately from the sim, because `worst frame` above is
        // `advance` only.** A render-side look option -- `GrainMode`,
        // `BubbleMode`, `GasMode` -- costs nothing that number can see, and
        // `CLAUDE.md` asks what a visual change costs. Drawn with every
        // chunk forced dirty, so this is the full-screen worst case rather
        // than whatever the dirty-rect skip happened to leave.
        let drew = std::time::Instant::now();
        renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH as u32, HEIGHT as u32), true);
        worst_draw_ms = worst_draw_ms.max(drew.elapsed().as_secs_f64() * 1000.0);
        if args.stress {
            paint_stress(&world, &mut frame);
        }
        if args.exposure {
            let wind = args.wind.unwrap_or_else(|| pixel_physics::sim::weather::at(world.seed, world.frame).wind);
            paint_exposure(&world, &mut frame, wind);
        }

        let (gx, gy) = (captured as i32 % args.cols as i32, captured as i32 / args.cols as i32);
        blit_tile(&mut sheet, sheet_w, (gx * (tile_w + gap), gy * (tile_h + gap)), &frame, args.crop, args.zoom);
        // `bodies` reports M8 chunk bodies in flight. Worth printing rather
        // than inferring from the image: a coherent falling slab and a
        // tightly-packed scatter of loose grains look nearly identical at
        // the zoom levels these sheets are usually read at, so "did this
        // actually promote to a body" is a question the picture cannot
        // answer on its own.
        if !render {
            captured += 1;
            continue;
        }
        println!(
            "  tile {captured}: frame {target}, awake {}/{}, sites {}, particles {}, bodies {} ({} cells)",
            world.active_chunk_count(),
            world.chunk_count(),
            world.active_site_count(),
            particles.len(),
            world.chunk_bodies.len(),
            world.chunk_bodies.iter().map(|b| b.cells.len()).sum::<usize>(),
        );
        // **How fast the pieces are actually going**, which the count
        // above cannot say and which play asked about: *"it falls at
        // slightly odd rates -- a first group of chunks that drop too fast
        // and then the rest that come together with the grit later."*
        // Printed as a spread, because the complaint is about the spread:
        // one number would be the average of the two groups and describe
        // neither.
        if !world.chunk_bodies.is_empty() {
            let mut speeds: Vec<f32> =
                world.chunk_bodies.iter().map(|b| (b.vx * b.vx + b.vy * b.vy).sqrt()).collect();
            speeds.sort_by(|a, b| a.partial_cmp(b).expect("no NaN speeds"));
            println!(
                "    body speed: slowest {:.2}, median {:.2}, fastest {:.2} cells/frame",
                speeds[0],
                speeds[speeds.len() / 2],
                speeds[speeds.len() - 1]
            );
            // **The spread above cannot say whether size is what varies it**,
            // and that is the question a terminal velocity is about: a real
            // one goes as the square root of the body's size, so grit drifts
            // down while a boulder plummets. A flat clamp and a
            // size-dependent one produce the same "slowest/median/fastest"
            // line on a scene with a range of pieces in it. Only a *paired*
            // extreme -- the smallest piece against the largest, in the same
            // frame -- separates them, which is `CLAUDE.md`'s rule about
            // comparing two runs applied to two bodies.
            let mut by_size: Vec<(i32, f32)> = world
                .chunk_bodies
                .iter()
                .map(|b| (body_extent_of(b), (b.vx * b.vx + b.vy * b.vy).sqrt()))
                .collect();
            by_size.sort_by_key(|&(e, _)| e);
            let (small, fast) = (by_size[0], by_size[by_size.len() - 1]);
            println!(
                "    by size: smallest piece {} cells across at {:.2}, largest {} across at {:.2}",
                small.0, small.1, fast.0, fast.1
            );
        }
        // Which failure fired, cumulatively. An overloaded piece and a
        // piece that was never held look identical falling, so the image
        // cannot say which mechanism produced what is on screen -- and
        // those are the two halves of the model, with different causes and
        // different bugs. `CLAUDE.md`: print the count next to the image
        // and read both.
        if let Some(line) = gnome.report(&world) {
            println!("{line}");
        }
        let f = world.structural_failures;
        println!(
            "    failures: overloaded {} ({} cells), unsupported {} ({} cells)",
            f.overloaded, f.overloaded_cells, f.unsupported, f.unsupported_cells
        );
        // **Relabelled to what it actually holds.** It was printed as
        // "furthest a failure landed from its trigger", which is not what
        // `max_chain_reach` is: it is Manhattan `|failure.at - (x, y)|`,
        // the distance from the checked cell to the failing ancestor,
        // bounded by construction to `ROOTWARD_CHECK_STEPS` hops. Read as a
        // containment number it is worse than useless -- on a rolling-world
        // blast it reads 1 cell while damage is landing everywhere.
        println!("    furthest a failure's root was from the cell that was checked: {} cells", f.max_chain_reach);
        // ...and the number that *is* the containment measure, in the units
        // the `F9` setting is written in so the two can be read against each
        // other. The reach is named rather than printed raw: `i32::MAX` as a
        // number is unreadable, and a sheet that does not say which mode
        // produced it cannot be compared to the one beside it.
        // ...and the number that *was* meant to be the containment measure.
        // **It cannot report a containment failure, and it is kept only so
        // the two can be read side by side.** It is recorded exclusively at
        // sites downstream of `clip_region_to_licence`, and for any cell
        // that clip retains `within_disturbance` guarantees a live
        // disturbance within `chain_reach + extent` while
        // `distance_to_live_disturbance` takes the *min* over disturbances
        // of `distance - extent` -- so it is `<= chain_reach` by arithmetic.
        // A run reading exactly 48 at LOCAL and exactly 16 at TIGHT is a
        // saturated ceiling. See `damage_radius`, printed beneath it, which
        // reads none of that machinery.
        println!(
            "    furthest damage landed from a live disturbance: {} cells (chain_reach = {}) -- CEILING, see below",
            f.max_damage_reach,
            chain_reach_name(world.chain_reach)
        );
        let (blast_reach, blast_past) = damage_radius(&world, &materials_before, &fired);
        println!("    furthest cell this run actually changed, from the charge that made it: {blast_reach} cells ({blast_past} past that charge's own radius)");
        // R3a's "did it fire at all" counter. A failure too big for one
        // tick comes down over several, and the `bodies` line above shows
        // that as a *series* of bursts -- but a series of bursts is also
        // what several independent failures look like, and a contact sheet
        // cannot tell them apart at all. The count says which it was.
        println!("    of those, paced across ticks: {} slice(s), {} cells", f.staged_slices, f.staged_cells);
        // The only line in this block that counts a *displacement*. Every
        // number above it -- `failures`, the reach, the paced slices -- is
        // recorded at `structural.rs`'s `record` call, which runs before
        // the free-face test, the boundary erosion, the slicing and the
        // fracture. So "unsupported 400 (12,000 cells)" is entirely
        // consistent with not one cell having moved, and that is not a
        // hypothetical: it is the exact shape of the owner's "no pieces
        // move, ever, not even chunks well over 8 cells, and nothing turns
        // to dust either" against a harness reporting hundreds of
        // failures. The census line below closes the gap one step further
        // along for *material*; this closes it for *motion*, and the two
        // are different questions -- rubble standing where it fell is a
        // loss to neither.
        //
        // Both halves are printed, never one: their ratio is the block-
        // size distribution the ethos is about, and a run that promotes
        // nothing and a run that shatters nothing are two different bugs
        // that either number alone reads as the same.
        println!(
            "    of those, actually moved: {} bodies ({} cells promoted), {} cells shattered to rubble",
            f.promoted_bodies, f.promoted_cells, f.shattered_cells
        );
        // ...and the *shape* of that number, which the pair above cannot
        // give. See `FailureCounts::promoted_sizes`: a mean cannot tell a
        // run where everything came off the same size from one with a few
        // blocks, more cobbles and a lot of grit, and the second is what
        // the ethos asks for.
        let sz = f.promoted_sizes;
        println!(
            "    body sizes: <8:{} 8-15:{} 16-31:{} 32-63:{} 64-127:{} 128-255:{} 256+:{}",
            sz[0], sz[1], sz[2], sz[3], sz[4], sz[5], sz[6]
        );
        // The rotation fit probe, printed because it spent the whole life of
        // the mechanism answering "clear" unconditionally and nothing on a
        // contact sheet could show it (`open-bugs-handoff.md` bug K). A
        // refusal count of zero on a scene with walls in it is the tell that
        // it has gone vacuous again.
        println!(
            "    quarter turns: {} asked, {} refused by the fit probe ({}%)",
            f.rotations_asked,
            f.rotations_refused,
            (f.rotations_refused * 100).checked_div(f.rotations_asked).unwrap_or(0)
        );
        // The tipping test, on its own line for the reason
        // `FailureCounts::topples_asked` gives: it fires on the floor and
        // the line above fires in the air, and a reader who sees one number
        // move cannot tell which mechanism moved it. Read beside the
        // lying/upright split below -- a settled pile that is still standing
        // on end with `asked` at zero is a wiring question, and one with
        // `refused` high is a pile with no room in it.
        println!(
            "    of which landed out of balance and went over: {} asked, {} refused ({}%)",
            f.topples_asked,
            f.topples_refused,
            (f.topples_refused * 100).checked_div(f.topples_asked).unwrap_or(0)
        );
        // And how the pieces themselves came down, which is **not** the same
        // question as the `settled log pieces` line further down: that one
        // folds touching logs into one cluster and reports the *pile's*
        // shape. Asked of each body as it lands, so two logs that come to
        // rest against each other are still two readings.
        println!(
            "    how pieces came to rest: {} lying, {} upright, {} square (each body's own box, at the frame it landed)",
            f.settled_lying, f.settled_upright, f.settled_square
        );
        // The phase-change "did it fire at all" counters, cumulative --
        // same reasoning as the failure counts above: whether the plume on
        // screen is boiled steam or painted smoke is a question the image
        // cannot answer.
        let p = world.phase_changes;
        println!(
            "    phase changes: boiled {}, condensed {}, froze {}, melted {}, reacted {}",
            p.boiled, p.condensed, p.froze, p.melted, p.reacted
        );
        // **How often evaporation found the air already saturated**, which no
        // image can show: a surface that is drying slowly and one that is
        // switched off outright look the same in every frame, and the second
        // is a mechanism not running. `evaporation::DrynessCounts` has the
        // full story; the short version is that a block of saturated soil is
        // pinned at double the humidity that stops evaporation dead, and
        // `dryness` samples the block *above* a surface, so how much of the
        // world this reaches is a question about relief.
        //
        // Water and soil separately, because a wide calm lake reading zero is
        // the designed behaviour and damp ground reading zero may not be.
        let d = world.dryness_counts;
        if d.water_checks + d.soil_checks > 0 {
            let pct = |share: Option<f64>| share.map_or("-".to_string(), |s| format!("{:.0}%", s * 100.0));
            println!(
                "    evaporation becalmed: soil {}/{} ({}), water {}/{} ({})",
                d.soil_becalmed,
                d.soil_checks,
                pct(d.soil_becalmed_share()),
                d.water_becalmed,
                d.water_checks,
                pct(d.water_becalmed_share()),
            );
        }
        // Same reasoning one line up, for splashes: a droplet in flight is
        // one pixel and lands within a few frames, so `particles` on the
        // tile line above is very often 0 even on a frame that threw a
        // dozen. Cumulative, so it answers "has this ever fired".
        if world.splashes_thrown > 0 {
            println!("    splash droplets thrown: {}", world.splashes_thrown);
        }
        // A census to read against the event counts above -- `CLAUDE.md`'s
        // "a failure count is not a damage count" bites here too: `froze`
        // going flat can mean "all the lava finished" or "the remaining
        // lava stopped cooling", and only a standing count tells them
        // apart. `burning` likewise separates "the pond simmers off stored
        // heat" from "something at the shoreline is still on fire".
        let (mut molten, mut burning, mut hot, mut bubbling) = (0u32, 0u32, 0u32, 0u32);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let cell = world.get(x, y);
                let m = world.materials.get(cell.material);
                if m.intrinsic_temperature.is_finite() {
                    molten += 1;
                }
                if cell.is_burning() {
                    burning += 1;
                }
                if cell.temperature() >= 100 {
                    hot += 1;
                }
                // The population `render.rs`'s bubble effect can draw on --
                // a *liquid* cell over the bubbling threshold, which is a
                // different set from `hot` above (that one is mostly the
                // steam cloud). Printed because "the pool does not look
                // like it is bubbling" and "there is nothing hot enough in
                // the pool to bubble" are the same picture, and only a
                // count separates them.
                if m.kind == MaterialKind::Liquid && (cell.temperature() as f32) >= pixel_physics::render::BUBBLE_MIN_TEMPERATURE {
                    bubbling += 1;
                }
            }
        }
        // The bank rides on the unconditional standing line rather than on
        // the self-gating water line below, so **every** scene shows it. It
        // is the one half of the world's water that no image can contain:
        // water that has evaporated is not anywhere on screen, and a sheet
        // showing a pond that has visibly shrunk cannot say whether the
        // water went into the sky or out of the world. Printed next to a
        // census of what is standing, the pair is the conservation law.
        println!(
            "    standing: molten {molten}, burning {burning}, cells at >=100C {hot}, liquid hot enough to bubble {bubbling}, bank {:.1}",
            world.atmospheric_bank
        );
        // WHERE the heat is, not just how much -- open-bugs-handoff 0b's own
        // discriminator ("dump where the >=100C cells and the outstanding
        // steam actually sit"). Per-material counts with a bounding box per
        // material, because "592 hot cells" was compatible with a hot stone
        // core, a trapped steam pocket, and a simmering pool all at once,
        // and only the location separates them. Bounded work: one scan,
        // printed only when something is hot, so quiet scenes stay quiet.
        if hot > 0 {
            use std::collections::HashMap as Map;
            let mut by_material: Map<u16, (u32, i64, i32, i32, i32, i32)> = Map::new();
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    let cell = world.get(x, y);
                    if cell.temperature() >= 100 {
                        let e = by_material.entry(cell.material.0).or_insert((0, 0, i32::MAX, i32::MIN, i32::MAX, i32::MIN));
                        e.0 += 1;
                        e.1 += cell.temperature() as i64;
                        e.2 = e.2.min(x);
                        e.3 = e.3.max(x);
                        e.4 = e.4.min(y);
                        e.5 = e.5.max(y);
                    }
                }
            }
            let mut rows: Vec<_> = by_material.into_iter().collect();
            rows.sort_by_key(|(_, (n, ..))| std::cmp::Reverse(*n));
            for (id, (n, temp_sum, x0, x1, y0, y1)) in rows.into_iter().take(4) {
                let name = &world.materials.get(pixel_physics::sim::material::MaterialId(id)).name;
                println!(
                    "      >=100C in {name}: {n} cells, mean {}C, box x {x0}..{x1} y {y0}..{y1}",
                    temp_sum / n as i64
                );
            }
        }
        // **The plume, as a standing state rather than an event rate.**
        // `boiled`/`condensed` above are cumulative and they run at nearly
        // 1:1 on any scene with a live boil, which is exactly what a
        // healthy loop looks like *and* exactly what a plume that rains
        // straight back down looks like. The owner's report -- steam rises
        // a few cells and drops back as rain, fast enough to read as
        // bouncing -- is about neither count: it is about how far the
        // plume gets and how much *water* is standing in the air inside
        // it. `CLAUDE.md`'s "when the complaint is about something visible
        // and persistent, measure the standing state, not the event rate".
        //
        // *Airborne water* is a water cell with materially-empty space
        // directly below it. Sanity-checked against cases known to be fine
        // before it was trusted, per `CLAUDE.md`: a settled pond
        // (`scene=coldsnap`) reads 0, and `scene=fall` -- water genuinely
        // falling through air -- reads in the hundreds, which is the
        // metric working, not an artifact. On `scene=lavapour` nothing
        // pours *water*, so every airborne water cell there is condensate
        // that has turned around and is on its way back down.
        let (mut steam_cells, mut steam_top, mut steam_bottom) = (0u32, i32::MAX, i32::MIN);
        let mut submerged_steam = 0u32;
        let (mut airborne, mut air_top, mut air_bottom) = (0u32, i32::MAX, i32::MIN);
        let steam_id = world.materials.id_of("steam");
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let cell = world.get(x, y);
                if Some(cell.material) == steam_id {
                    steam_cells += 1;
                    steam_top = steam_top.min(y);
                    steam_bottom = steam_bottom.max(y);
                    // **A bubble is submerged steam**, and nothing counted
                    // it. Reported from play about a heat source under a
                    // pool: *"I see bubbles form at the bottom, rise to the
                    // top and pop"*. Whether the engine ever produces that
                    // -- gas inside the water, on its way up -- or only ever
                    // vents steam at the interface, is not visible in a
                    // contact sheet and is not what `steam_cells` counts: a
                    // plume standing over a pond and a pond full of rising
                    // bubbles give the same total.
                    if y > 0 && world.get(x, y - 1).material == material::WATER {
                        submerged_steam += 1;
                    }
                }
                if cell.material == material::WATER
                    && y + 1 < HEIGHT
                    && world.get(x, y + 1).material == material::EMPTY
                {
                    airborne += 1;
                    air_top = air_top.min(y);
                    air_bottom = air_bottom.max(y);
                }
            }
        }
        // **Rock standing on nothing and touching nothing.** The owner's
        // second report ("it seems to freeze in place... it should sink")
        // as a standing count, because the failure counters cannot answer
        // it: `unsupported` says how many cells the model *judged* had no
        // support, and the whole defect was cells it never judged at all,
        // or judged and then deliberately left alone as confined. Only a
        // census of what is still standing there separates those.
        //
        // Deliberately the strictest reading -- a `Solid` with no
        // `Solid`/`Plant` neighbour in any of the 8 directions and nothing
        // solid beneath it. That makes it blind to a *raft* (whose cells
        // hold each other's hands) and immune to false positives on
        // ordinary terrain, where it reads 0 on every scene that has no
        // artifact. A looser definition was not worth the argument about
        // what an overhang is.
        // The load model's **own** verdict, censused: an unattached `Solid`
        // whose support chain reaches no anchor, still standing there.
        //
        // Two cheaper definitions were tried and are worth recording,
        // because both are the same mistake in different clothes. "No solid
        // neighbour at all" is blind to a *raft*, whose cells hold each
        // other's hands, and read 9 against 14 across a change that visibly
        // cleared a mound off a pond. "Nothing solid in the column below"
        // is blind to anything over water, because the pond floor is in
        // that column. `load::evaluate` asks the question the artifact
        // actually is, and it is the same function the `load=` probe prints
        // -- so a number here and a probe there cannot disagree.
        //
        // Restricted to unattached cells, which is what bounds the cost:
        // terrain is attached and is the overwhelming majority, and an
        // attached cell braced by the massif is not what anyone means by
        // rock hanging in the air.
        //
        // **Reported as clusters, not as a bare count, because the count
        // cannot say what the artifact is.** 47 hanging cells is either one
        // raft the size of a dinner plate or 47 grains scattered over the
        // pond, and those are different bugs with different fixes -- the
        // first is a piece the model refuses to drop, the second is cells
        // it never groups in the first place. The owner read the same
        // ambiguity off the image ("attached rock stuck in the middle of
        // the water, unless you just didn\'t wait long enough") and a
        // number that cannot distinguish them cannot answer them either.
        //
        // **Clustered first, and every cell of a cluster asked unless the
        // cluster is enormous.** `load::evaluate` answers by flooding the
        // connected region a cell belongs to, so a cluster of `k` cells
        // costs `k^2` to census cell by cell. `scene=capped` stands
        // **15,840** unattached cells up on purpose and this pass never
        // finished a single tile -- which read in the acceptance log as the
        // suite stalling with no clue which case did it. A shared
        // `load::Cache` alone did not fix it.
        //
        // Asking only each cluster's *foot* does fix it and is **too
        // blind**, which is worth recording because it is the obvious
        // move: the model's verdict is not uniform over a piece (a cell
        // beyond `max_unsupported_span` from the anchor is unsupported
        // while the foot standing on it is not), so `scene=lavapour`'s
        // first tile read **19 against 49**. The census exists to catch
        // exactly the cells that reading throws away.
        //
        // So the cap bounds *work*, never whether the census happens
        // (`CLAUDE.md`): every cluster up to `HANGING_CENSUS_FULL` is
        // asked cell by cell, exactly as before, and a larger one is asked
        // at its lowest-then-leftmost cell and **says so in the output**.
        // Nothing in any scene that has ever shown this artifact comes
        // close to the cap -- the largest piece in `lavapour` is 18 cells.
        let unattached: HashSet<(i32, i32)> = (0..HEIGHT)
            .flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let cell = world.get(x, y);
                !cell.attached() && cell.organism_id() == 0 && world.materials.kind(cell.material) == MaterialKind::Solid
            })
            .collect();
        let mut hanging_cells: Vec<(i32, i32)> = Vec::new();
        let mut load_cache = pixel_physics::sim::load::Cache::default();
        let mut load_budget = u32::MAX;
        let mut seen: HashSet<(i32, i32)> = HashSet::new();
        let mut sampled_clusters: Vec<usize> = Vec::new();
        // Iterated over the sorted grid rather than the `HashSet`, so the
        // clusters (and therefore the printed order) do not depend on hash
        // iteration order.
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if !unattached.contains(&(x, y)) || !seen.insert((x, y)) {
                    continue;
                }
                let mut stack = vec![(x, y)];
                let mut members = Vec::new();
                while let Some((cx, cy)) = stack.pop() {
                    members.push((cx, cy));
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let n = (cx + dx, cy + dy);
                            if unattached.contains(&n) && seen.insert(n) {
                                stack.push(n);
                            }
                        }
                    }
                }
                if members.len() <= HANGING_CENSUS_FULL {
                    for &(mx, my) in &members {
                        if pixel_physics::sim::load::evaluate_with_cache(&world, mx, my, &mut load_cache, &mut load_budget)
                            .is_some_and(|l| !l.supported)
                        {
                            hanging_cells.push((mx, my));
                        }
                    }
                    continue;
                }
                sampled_clusters.push(members.len());
                let foot = members.iter().copied().min_by_key(|&(mx, my)| (-my, mx)).expect("a cluster has a cell");
                if pixel_physics::sim::load::evaluate_with_cache(&world, foot.0, foot.1, &mut load_cache, &mut load_budget)
                    .is_some_and(|l| !l.supported)
                {
                    hanging_cells.extend(members);
                }
            }
        }
        hanging_cells.sort_unstable();
        if !sampled_clusters.is_empty() {
            println!(
                "    hanging census: {} cluster(s) over {HANGING_CENSUS_FULL} cells asked at the foot only, sizes {sampled_clusters:?}",
                sampled_clusters.len()
            );
        }
        let hanging = hanging_cells.len() as u32;
        if hanging > 0 {
            println!("    hanging: {hanging} unattached solid cells the load model says reach no anchor");
            for line in describe_hanging(&world, &hanging_cells) {
                println!("      {line}");
            }
        }
        // **The same artifact asked of the world instead of the model, and
        // the pair is the point.** `hanging` above is the load model's own
        // verdict censused, which makes it blind in exactly one direction:
        // a piece the model *wrongly* calls supported does not appear, and
        // that is the failure mode with the worst consequences, because
        // nothing downstream will ever touch it either. Caught for real --
        // a stone raft sat on `scene=lavadrop`'s pond from frame 600 to the
        // end of the run, plainly visible in the contact sheet, with
        // `hanging` reading 0 the whole time.
        //
        // So this one consults no model at all: a group of unattached solid
        // cells, none of which touches anything that could hold it up
        // (bedrock, attached rock, or loose material), with liquid
        // somewhere around it. That is "rock floating in water" as the
        // owner says it, and no support rule can talk it out of the answer.
        let afloat = describe_afloat(&world);
        if !afloat.is_empty() {
            println!("    afloat: {} groups of unattached rock held up by nothing but liquid", afloat.len());
            for line in afloat.iter().take(6) {
                println!("      {line}");
            }
            if afloat.len() > 6 {
                println!("      ... and {} smaller", afloat.len() - 6);
            }
        }
        if steam_cells > 0 || airborne > 0 {
            let span = |n: u32, a: i32, b: i32| if n == 0 { "-".to_string() } else { format!("rows {a}..{b}") };
            println!(
                "    plume: steam {steam_cells} cells ({}, {submerged_steam} submerged), airborne water {airborne} cells ({})",
                span(steam_cells, steam_top, steam_bottom),
                span(airborne, air_top, air_bottom)
            );
        }
        // The water cycle's standing state, next to the counters above --
        // and the pair is the point. The counters say the mechanism fired;
        // this says what is *there*, which is the question a freeze-and-thaw
        // actually turns on: a run can report thousands of freezes and
        // melts and still have quietly lost half its pool. Self-gating, so
        // the structural scenes stay quiet.
        let (liquid, frozen, snowy) = water_census(&world);
        if liquid > 0.0 || frozen > 0 || snowy > 0 {
            println!(
                "    water: {liquid:.1} cell-equivalents liquid, {frozen} frozen, {snowy} as snow (total {:.1})",
                liquid + frozen as f64 + snowy as f64
            );
            // ...and the same water counted the *other* way, which is the
            // only way that closes. The line above counts a flake and a full
            // water cell alike as one cell, which is right for asking
            // whether a pond froze over and wrong for conservation, because
            // snow is 0.3 the density of water.
            // `weather::water_equivalents` converts every phase at what it
            // would come back as, so this figure plus the bank is the
            // quantity that must hold still across a whole storm-and-drought
            // cycle -- and a tile-by-tile printout is where a drift shows up
            // as a trend rather than as a single end-of-run number.
            // **How much of the water's *surface* is frozen, and how much
            // of the freezing was undone.**
            //
            // Reported from play: *"it never really freezes and has snow
            // accumulate on top. The pixels seem to be constantly shifting."*
            // Neither half of that is answerable from the counters above.
            // `frozen` is a cell count, and a pond can hold three hundred
            // ice cells forever as a **churning slush** that never closes
            // into a sheet -- measured on `scene=coldsnap`: 491 freezes and
            // 510 melts across 340 frames for a net of **minus nineteen**,
            // with the band stuck at a quarter of the pond the whole time.
            //
            // So two numbers, and they answer different questions:
            //
            // - **coverage**: of the columns that hold any water at all,
            //   how many have ice at the top of that water. That is the
            //   thing a player calls "frozen over".
            // - **churn**: freeze events since the last tile against the
            //   change in standing ice. A healthy front is near 1.0 --
            //   almost every freeze sticks. A slush is unbounded, because
            //   the numerator keeps counting and the denominator is zero.
            let (surface_frozen, surface_total, mean_thick, thickest) = frozen_surface(&world);
            let (froze, melted) = (world.phase_changes.froze, world.phase_changes.melted);
            let churn = match last_ice {
                Some((pf, pm, pi)) => {
                    let (df, dm, di) = (froze - pf, melted - pm, frozen - pi);
                    let ratio = if di > 0 { format!("{:.1}", df as f64 / di as f64) } else { "no net gain".to_string() };
                    format!(", since the last tile froze +{df}, melted +{dm}, standing {di:+} (churn {ratio})")
                }
                None => String::new(),
            };
            last_ice = Some((froze, melted, frozen));
            if surface_total > 0 {
                println!(
                    "    ice: {surface_frozen} of {surface_total} water columns frozen at the surface ({:.0}%){churn}",
                    100.0 * surface_frozen as f64 / surface_total as f64
                );
                if surface_frozen > 0 {
                    println!("    sheet: {mean_thick:.1} cells thick on average, {thickest} at the thickest");
                }
            }
            let standing = pixel_physics::sim::weather::water_equivalents(&world);
            println!(
                "    water + sky: {standing:.1} standing + {:.1} banked = {:.1} cell-equivalents",
                world.atmospheric_bank,
                standing + world.atmospheric_bank
            );
            // **How fast the sky filled or emptied since the last tile, next
            // to what time of day it is and whether anything is falling.**
            // The absolute figure above is the conservation reading; this is
            // the *rate*, and it is a different question that the same
            // number cannot answer by eye. `evaporation::warmth` made drying
            // diurnal, so what this column should show on a dry scene is a
            // credit that rises through the morning, peaks around noon and
            // goes nearly flat overnight -- and then goes hard negative for
            // as long as a front is spending it.
            //
            // Printed as a delta rather than left for the reader to subtract
            // because the credit over a quarter-day is a percent or two of
            // the standing total, and a trend that small is invisible in a
            // column of four-figure numbers. It is also the counter that
            // says the coupling *fired*: two very different rates look
            // identical in a picture of a pond (`CLAUDE.md`, "did it fire at
            // all needs a counter").
            let elevation = pixel_physics::sim::field::sun_elevation(world.frame);
            let phase = match (elevation, pixel_physics::sim::field::sun_rising(world.frame)) {
                (e, _) if e > 0.7 => "noon",
                (e, _) if e < -0.7 => "midnight",
                (_, true) => "sunrise",
                (_, false) => "sunset",
            };
            let sky = pixel_physics::sim::weather::at(world.seed, world.frame);
            // The window *ends* at this tile, so the phase named is the phase
            // it ended in. Said outright in the line rather than left to the
            // reader: "+4.54 at sunset" and "+4.54 since sunset" are opposite
            // readings of the same number, and on a scene whose whole point is
            // which half of the day dries faster, guessing wrong inverts the
            // result.
            print!(
                "    sky: {:+.2} banked over the {} frames ending at {phase} (sun {elevation:+.2})",
                world.atmospheric_bank - last_bank.unwrap_or(world.atmospheric_bank),
                args.every,
            );
            if sky.is_precipitating() {
                println!(", {:?} at intensity {:.2}", sky.kind, sky.intensity);
            } else {
                println!(", clear");
            }
            last_bank = Some(world.atmospheric_bank);
        }
        // **Which rate the world is rotting at, and how much of it qualifies.**
        //
        // The worldgen soil baseline moved ground that used to sit at `aux ==
        // 0` up to a climate value, and `field.rs` forces humidity from soil
        // wetness -- so the damp gate that used to trip only near water may
        // now trip nearly everywhere. Damp decay is 25x dry decay, and that
        // is invisible on a contact sheet: rotted litter and litter that
        // simply never landed are the same absence of pixels. Only the split
        // says which.
        //
        // The census is over the cells that can *actually* decay -- the ones
        // with a `decays_into` -- because that is precisely the population
        // the gate is sampled over, one sample per cell at its own position.
        //
        // An earlier version censused *exposed soil* instead and was
        // quietly confounded: a wetter world grows more plants, more plants
        // cover more ground, so the denominator moved with the treatment
        // (175 exposed cells down to 65 on the same preset) and the
        // percentage compared two different populations. Count the thing the
        // rule evaluates, not a proxy that the treatment also moves.
        let gate = pixel_physics::sim::decay::DECAY_MOISTURE_THRESHOLD;
        let (mut decayable, mut above) = (0usize, 0usize);
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                if world.materials.get(world.get(x, y).material).decays_into.is_none() {
                    continue;
                }
                decayable += 1;
                if world.field_at(x, y).moisture > gate {
                    above += 1;
                }
            }
        }
        let pct = if decayable == 0 { 0.0 } else { 100.0 * above as f32 / decayable as f32 };
        // Living tissue beside the dead, because the two move together and
        // the interesting failure is the ratio: a world with plenty of
        // standing litter and no plants left is a world that rotted faster
        // than it grew, which no single count can show.
        let living: usize = (0..WIDTH)
            .flat_map(|x| (0..HEIGHT).map(move |y| (x, y)))
            .filter(|&(x, y)| world.get(x, y).organism_id() != 0)
            .count();
        println!("    living plant tissue: {living} cells");
        // **The felling census, printed beside the tile it describes.**
        // `living` above is one number for the whole world and cannot say
        // whether a severed crown is still attached, still standing while
        // detached, or already coming apart -- three states that look
        // identical on a contact sheet and have three different causes.
        // Gated on there being tissue at all so the destruction scenes,
        // which have none, do not grow three blank lines.
        if living > 0 {
            FellCensus::of(&world).print(&world);
        }
        println!(
            "    decay: {} damp + {} dry = {} events; of {decayable} decayable cells standing, {above} are above the {gate} damp gate ({pct:.0}%)",
            world.decayed_damp,
            world.decayed_dry,
            world.decayed_damp + world.decayed_dry,
        );
        // **How those events resolved, which since `Material::decay_yield`
        // is no longer the same number as the event count.** Litter yields a
        // solid 5% of the time and leaves nothing the rest, so the line above
        // counts decays while this one counts what they produced. Printed
        // next to each other deliberately: read alone, either is a number
        // that looks like soil production and is not.
        println!(
            "    rot: {} left solid + {} left nothing ({:.0}% yield)",
            world.rotted_to_solid,
            world.rotted_to_nothing,
            if world.rotted_to_solid + world.rotted_to_nothing > 0 {
                100.0 * world.rotted_to_solid as f64 / (world.rotted_to_solid + world.rotted_to_nothing) as f64
            } else {
                0.0
            },
        );
        // The decay events above are downstream of leaf fall, and leaf fall
        // has three causes with separate knobs. The retune's lever question
        // -- "which pressure is filling the floor" -- needs the split.
        let soil_id = world.materials.id_of("soil");
        if let Some(soil_id) = soil_id {
            let mut soil_now = 0usize;
            for x in 0..WIDTH {
                for y in 0..HEIGHT {
                    if world.get(x, y).material == soil_id {
                        soil_now += 1;
                    }
                }
            }
            let base = *first_soil.get_or_insert(soil_now);
            // **Net, not manufactured, and the difference is not pedantry.**
            // The first version of this line called the rise "manufactured by
            // decay" and clamped the negative case to zero. Then the retuned
            // arm ran -171, -265, -201, -114 before ending at +120: soil is
            // leaving as well as arriving, so decay's writes are only the
            // gross inflow and this column has never been able to see them
            // separately. `world.decayed_damp + decayed_dry` on the line
            // above IS the gross inflow; read the two together and the
            // difference between them is whatever is consuming soil.
            //
            // **Corrected when `Material::decay_yield` landed: the gross
            // inflow is `rotted_to_solid`, NOT the decay total.** Most litter
            // decays now leave nothing behind, so the event count above
            // overstates soil production by ~20x on any wooded scene. The
            // sentence above was true when written and silently became a
            // different claim -- which is the trap `CLAUDE.md` names, arriving
            // here by the usual route of a number staying arithmetically
            // correct while the thing it counts moves out from under it.
            println!(
                "    floor: {soil_now} soil cells, {:+} net since the first sample",
                soil_now as i64 - base as i64,
            );
        }
        // **Leaf against wood, because that is what sets a silhouette.**
        // `Reports/plant-appearance-design.md` is the reason this is a
        // separate line from the living-tissue total: three architectural
        // levers all fired, all printed their counters, and the owner still
        // read the sheets as unchanged, because every species was ~90% wood
        // and ~5% leaf and a lever that relabels a cell cannot move a
        // silhouette that composition sets. Any question of the form "does
        // the crown read differently" needs this ratio beside the picture,
        // not the cell count.
        if let (Some(leaf_id), Some(wood_id)) =
            (world.materials.id_of("leaf"), world.materials.id_of("wood"))
        {
            let (mut leaf, mut wood) = (0usize, 0usize);
            for x in 0..WIDTH {
                for y in 0..HEIGHT {
                    let m = world.get(x, y).material;
                    if m == leaf_id {
                        leaf += 1;
                    } else if m == wood_id {
                        wood += 1;
                    }
                }
            }
            let total = (leaf + wood).max(1);
            println!(
                "    composition: {leaf} leaf + {wood} wood = {} ({:.1}% leaf)",
                leaf + wood,
                100.0 * leaf as f64 / total as f64,
            );
        }
        // **The standing organ census, which is a different question from the
        // organ counters and the one a picture can actually be checked
        // against.** `CLAUDE.md`: when the complaint is visible and
        // persistent, measure the standing state, not the event rate. A
        // stand can build a thousand organs and show none, because each one
        // ripened and let go inside the interval between two tiles -- which
        // is exactly what a first pass at the fruiting habit did, and the
        // sheet showed flowers and no fruit at all while `organs built` read
        // 1,126. This line is what says whether there is anything on the
        // plant to see.
        //
        // Windfall is on it too, and separately: a fallen fruit is on the
        // *ground*, so counting it with the fruit would let a floor of them
        // stand in for a crop.
        {
            let count = |name: &str| {
                world.materials.id_of(name).map_or(0usize, |id| {
                    (0..WIDTH).flat_map(|x| (0..HEIGHT).map(move |y| (x, y))).filter(|&(x, y)| world.get(x, y).material == id).count()
                })
            };
            let (flower, fruit, windfall) = (count("flower"), count("fruit"), count("windfall"));
            if flower + fruit + windfall + world.organs_built as usize > 0 {
                // **Where the windfall actually is, not where it is assumed
                // to be.** The first version of this line called every
                // windfall cell "on the ground" without checking, which is
                // exactly the unverified label this repo keeps paying for --
                // and it is a live question, because `main` added
                // `Material::falls_through_organisms` for litter lodging in
                // crowns and a dropped fruit takes the same path. Banded the
                // way `litter_probe` bands litter so the two are comparable:
                // a cell resting on plant tissue and a cell high above the
                // ground are different failures and a total hides both.
                let (mut lodged, mut high) = (0usize, 0usize);
                if let Some(id) = world.materials.id_of("windfall") {
                    let ground = common::PlantScene::default().ground_y;
                    for x in 0..WIDTH {
                        for y in 0..HEIGHT {
                            if world.get(x, y).material != id {
                                continue;
                            }
                            if world.get(x, y + 1).organism_id() != 0 {
                                lodged += 1;
                            }
                            if y + 16 < ground {
                                high += 1;
                            }
                        }
                    }
                }
                println!(
                    "    organs standing: {flower} flower + {fruit} fruit on the plant, \
                     {windfall} windfall ({lodged} resting on plant tissue, {high} more than 16 rows up)"
                );
                // The event counters beside the standing census, because they
                // are different questions and a card needs both: *built* says
                // the mechanism fired, *terminated* says determinacy fired
                // (a truss builds four organs off one terminated axis), and
                // *dropped* is the far side of the whole sequence -- organs
                // can be built in quantity and never once let go.
                println!(
                    "    organ events: {} built, {} axes terminated, {} fruit dropped",
                    world.organs_built, world.axes_terminated, world.fruit_dropped
                );
            }
        }
        println!(
            "    shed: {} shade + {} drought + {} stranded = {} leaves",
            world.shed_shade,
            world.shed_drought,
            world.shed_stranded,
            world.shed_shade + world.shed_drought + world.shed_stranded,
        );
        // **How much of the failure became grit rather than pieces.** The
        // mean region size cannot answer this and was misread as answering
        // it -- see `FailureCounts::crumbled`.
        let failed_cells = f.overloaded_cells + f.unsupported_cells;
        println!(
            "    crumbled to grit (region < MIN_FRACTURE_CELLS): {} regions, {} cells of {} failed ({:.0}%)",
            f.crumbled,
            f.crumbled_cells,
            failed_cells,
            if failed_cells == 0 { 0.0 } else { 100.0 * f.crumbled_cells as f64 / failed_cells as f64 }
        );
        // **Did the section rule fire at all**, printed next to the image
        // rather than inferred from it. A review of that rule caught the
        // report offering "identical to the cell on terrain" as evidence it
        // was safe there, which is equally what a rule that never ran
        // produces. `moved` is the number that answers it -- see
        // `load::ShareCounts`.
        let sh = world.load_cache.shares;
        println!(
            "    section share: {} columns, {} in a member, {} actually moved, {} too wide to tell",
            sh.columns, sh.members, sh.moved, sh.too_wide
        );
        // How much of the damage happened to rock with nowhere to go --
        // the mid-mountain collapse the owner reports as looking fake.
        // A picture cannot answer this: a collapse at a cliff edge and one
        // eighty cells inside a massif are the same grey rubble at the
        // zoom a contact sheet is read at.
        println!("    of those, confined (no free face anywhere): {} ({} cells), deepest {} cells from air, {} cells fissured", f.confined, f.confined_cells, f.deepest_confined, f.crushed_cells);
        // The complaint itself, counted rather than looked at -- see
        // `severed_islands`. Printed beside `confined` because the two are
        // the same story from opposite ends: `confined` counts failures the
        // model *judged* and refused to move, this counts rock that is cut
        // free and standing there whether or not anything ever judged it.
        let (pieces, islands, island_cells, largest_piece) = severed_islands(&world);
        println!("    rock the fissures actually cut loose: {pieces} piece(s), largest {largest_piece} cells -- of those, wedged with no free face: {islands} ({island_cells} cells)");
        // §S5: what the model says about that piece when it cannot starve.
        let charge = fired.last().map(|c| (c.x, c.y, c.radius * 4));
        let (n, failing, holds, deferred) = interrogate_largest_severed_piece(&world, charge);
        if n > 0 {
            println!("    that piece, re-asked with an unlimited budget: {n} cells -- {failing} failing, {holds} holds, {deferred} deferred");
            // If it holds on a budget it cannot exhaust, something in it is an
            // anchor. `load::is_anchor` is private, so this counts the three
            // ways it can be true from outside, which is enough to say which
            // one to go and read.
            let piece = largest_severed_piece_near(&world, charge);
            let (mut bedrock, mut on_powder, mut on_liquid) = (0, 0, 0);
            for &(x, y) in &piece {
                if [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|&(dx, dy)| world.get(x + dx, y + dy).material == material::BEDROCK)
                {
                    bedrock += 1;
                }
                match world.materials.kind(world.get(x, y + 1).material) {
                    MaterialKind::Powder => on_powder += 1,
                    MaterialKind::Liquid => on_liquid += 1,
                    _ => {}
                }
            }
            println!("      of those cells: {bedrock} touch bedrock, {on_powder} sit on powder, {on_liquid} sit on liquid");
            // **What is it, and where?** `severed_islands` counts any maximal
            // solid component the cracks left unconnected -- which includes
            // things no blast ever touched. Naming the material and the box
            // is what stops "the largest severed piece" being read as "rock
            // the blast cut loose" when it is neither.
            let mut kinds: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
            for &(x, y) in &piece {
                *kinds.entry(world.materials.get(world.get(x, y).material).name.clone()).or_default() += 1;
            }
            let (x0, x1) = (piece.iter().map(|c| c.0).min().unwrap_or(0), piece.iter().map(|c| c.0).max().unwrap_or(0));
            let (y0, y1) = (piece.iter().map(|c| c.1).min().unwrap_or(0), piece.iter().map(|c| c.1).max().unwrap_or(0));
            let listed: Vec<String> = kinds.iter().map(|(k, v)| format!("{k} {v}")).collect();
            println!("      it is: {} -- x {x0}..{x1}, y {y0}..{y1}", listed.join(", "));
        }
        // How much the world has actually *lost* since the cut was made,
        // which the failure counts above cannot say: a failed cell that
        // became rubble is still standing there. Printed per tile rather
        // than once at the end so the trajectory is visible -- a run that
        // has stopped eating and one that is still going look identical in
        // a single total. See `Args::max_lost`.
        println!("    roofed void (cave volume): {} cells, was {} at the cut", roofed_void(&world), cave_before);
        println!("    gas cells standing in the world: {}", smoke_census(&world));
        let (solid, powder) = census(&world);
        println!(
            "    cells lost since the cut: {} (rock {:+}, rubble {:+})",
            (cells_before.0 + cells_before.1) - (solid + powder),
            solid - cells_before.0,
            powder - cells_before.1
        );
        // R1's halo, censused rather than inferred from the image: a
        // fissure line a cell wide reads as noise at the zoom a contact
        // sheet is usually read at (baseline measured 47 nearly-invisible
        // cells on `boom_stone`, per `Reports/explosion-stone-review.md`
        // §1a), so "did the crack halo actually fire" needs the same
        // counter-next-to-the-image treatment as `bodies` above. Boxed
        // around the *last* `explode=` site specifically -- a scene may
        // schedule more than one, and the box only means anything relative
        // to a single blast's own radius.
        if let Some(&(ex, ey, er, ..)) = args.explosions.last() {
            println!("    cracked cells within 3x radius of ({ex}, {ey}): {}", cracked_census(&world, ex, ey, er));
            // The anti-"permanent sticker" counter. Scorched stone has no
            // conductivity, so nothing in `fire.rs` ever cools it (its
            // thermally-inert fast path returns before any decay) -- a hot
            // ring written by a blast used to be *permanent*, and an image
            // cannot tell a glow that is fading from one that is frozen:
            // both are orange in a still. Max and count together, because
            // they answer different halves: the max says how bright the
            // brightest cell still is, the count says how much of the ring
            // is still lit at all.
            let (hottest, lit) = heat_census(&world, ex, ey, er);
            println!("    hottest cell within 3x radius: {hottest} C, cells above ambient: {lit}");
        }
        // The census above is boxed around the **last** `explode=` site,
        // which is one charge of however many the run fired -- with nine
        // charges it measures one of them and says nothing about the other
        // eight. It stays exactly as it is because recorded baselines quote
        // it; these are the ones that answer the question the sheet is
        // actually about. See `per_charge_reports` for why they are gated.
        if per_charge_reports {
            for c in &fired {
                println!("    cracked cells within 3x radius of ({}, {}): {}", c.x, c.y, cracked_census(&world, c.x, c.y, c.radius));
            }
            // A whole-world scan, and worth saying what it costs because
            // `CLAUDE.md` is emphatic that harness cost is still cost: it is
            // 512x320 cell reads **per tile**, not per frame -- a handful of
            // scans across a run of thousands of frames, next to which it
            // does not register. It is here because the boxes above cannot
            // answer it: cracks spread, and a halo that has walked out of
            // every box reads as the crack mechanism switching off.
            println!("    cracked cells in the world: {}", cracked_world_census(&world));
        }
        // Pieces or grit. A region below `MIN_FRACTURE_CELLS` is not
        // fractured at all -- it falls through to per-cell conversion,
        // which *is* powder -- so a run whose failures average 1 or 2
        // cells has already decided to produce dust no matter what the
        // fragment ladder is set to. Printed next to the image because the
        // two are indistinguishable at the zoom a contact sheet is read
        // at.
        let events = f.overloaded + f.unsupported;
        if events > 0 {
            // The histogram, not just the mean and the max: those two are
            // satisfied by 570 single cells with one outlier and by a real
            // spread alike, and only one of those can ever produce a chunk.
            // `MIN_FRACTURE_CELLS` (6) and `MIN_BODY_CELLS` (8) are bucket
            // edges, so the floors that decide the outcome are visible as
            // floors in the readout.
            let edges = pixel_physics::sim::world::SIZE_BUCKETS;
            let hist: Vec<String> = f
                .size_buckets
                .iter()
                .enumerate()
                .filter(|&(_, &n)| n > 0)
                .map(|(i, n)| {
                    let hi = edges.get(i + 1).map_or(String::from("+"), |next| {
                        if next - edges[i] == 1 { String::new() } else { format!("-{}", next - 1) }
                    });
                    format!("{}{hi}:{n}", edges[i])
                })
                .collect();
            // The mass split, beside the region sizes -- see
            // `FailureCounts::promoted_cells`. A big region that fractures
            // into fragments below `MIN_BODY_CELLS` is dust on screen and
            // reads as a success in every other number here.
            let (chunks, dust) = (f.promoted_cells, f.shattered_cells);
            // `checked_div` rather than a `> 0` guard around a bare `/`:
            // identical behaviour -- it is None exactly when the divisor is
            // zero -- and it is the shape clippy's `manual_checked_ops` asks
            // for from 1.98 on.
            if let Some(pct) = (chunks * 100).checked_div(chunks + dust) {
                println!(
                    "    what came off: {chunks} cells as chunks, {dust} as dust ({pct}% chunk by mass)"
                );
            }
            println!(
                "    failing region size: mean {:.1} cells, largest {}, sizes [{}]",
                (f.overloaded_cells + f.unsupported_cells) as f64 / events as f64,
                f.largest_failure,
                hist.join(" ")
            );
        }
        if render {
            println!("    worst frame so far: {worst_ms:.2} ms (frame {worst_frame})");
            report_loads(&world, args);
            if args.exposure {
                report_exposure(&world, args.wind.unwrap_or_else(|| pixel_physics::sim::weather::at(world.seed, world.frame).wind));
            }
            dump_materials(&world, args);
        }
        captured += 1;
    }

    // Keep stepping to the last frame `panels=` wants a picture of. Only
    // entered when `panels=` is set, so a run without it ends exactly where
    // it used to -- which matters, because `check_expectations` and the
    // final census both read the world this leaves behind.
    if let Some(limit) = panel_last_frame {
        while step_no < limit {
            fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
            fire_due_cuts(&mut world, &mut pending_cuts, step_no);
            fire_due_chops(&mut world, &mut pending_chops, step_no);
            fire_due_fell(&mut world, &mut pending_fell, step_no);
            fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
            if let Some(p) = panels.as_mut() {
                p.capture(&world, &particles, &fired, step_no);
            }
            let began = std::time::Instant::now();
            advance(&mut world, &mut particles, &mut blasts, args.parallel_driver, step_no, &mut gnome, per_charge_reports);
            let ms = began.elapsed().as_secs_f64() * 1000.0;
            peak_bodies = peak_bodies.max(world.chunk_bodies.len());
            peak_tissue = peak_tissue.max(world.chunk_bodies.iter().flat_map(|b| b.cells.iter()).filter(|c| c.organism_id != 0).count());
            if ms > worst_ms && step_no > 0 {
                worst_ms = ms;
                worst_frame = step_no;
            }
            step_no += 1;
        }
        fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, &mut pending_blasts, &mut fired, step_no);
        fire_due_cuts(&mut world, &mut pending_cuts, step_no);
        fire_due_chops(&mut world, &mut pending_chops, step_no);
        fire_due_fell(&mut world, &mut pending_fell, step_no);
        fire_due_depowder(&mut world, &mut pending_depowder, &mut depowder_first, step_no);
        if let Some(p) = panels.as_mut() {
            p.capture(&world, &particles, &fired, step_no);
        }
        // The run went past its last tile, so the per-tile worst-frame line
        // above stopped before the end. Say what the whole run cost, or the
        // frames the extension paid for would be the only ones nobody timed.
        if render {
            println!("  panels: ran on to frame {step_no}; worst frame over the whole run {worst_ms:.2} ms (frame {worst_frame})");
        }
    }

    report_colony(&world, render);

    if render {
        image::save_buffer(&args.out, &sheet, sheet_w as u32, sheet_h as u32, image::ColorType::Rgba8)
            .expect("writing the contact sheet");
        println!("contact sheet ({sheet_w}x{sheet_h}, {} tiles): {}", args.count, args.out);
        // Said out loud, because a pinned exposure is invisible in the
        // image and a sheet nobody can reproduce is not evidence.
        if let Some(f) = args.daylight {
            println!("  drawn at a pinned daylight of {f} (render only -- the run itself was unaffected)");
        }
        if let Some(p) = panels {
            let path = panels_path(&args.out);
            image::save_buffer(&path, &p.sheet, p.sheet_w as u32, p.sheet_h as u32, image::ColorType::Rgba8)
                .expect("writing the panels sheet");
            println!("panels sheet ({}x{}, {} rows x {} cols): {}", p.sheet_w, p.sheet_h, p.ages.len(), p.cols, path);
            // A grid cell that never got a picture is a charge that never
            // fired, and a grey square is not a reading -- say so rather
            // than letting the gutter colour be mistaken for "nothing
            // happened there".
            let missing = p.filled.iter().filter(|f| !**f).count();
            if missing > 0 {
                println!("  panels: {missing} of {} cells never captured -- a charge scheduled past the end of the run", p.filled.len());
            }
        }
    }
    if peak_speed > 0.0 {
        println!(
            "fastest piece: {peak_speed:.2} cells/frame; everything came to rest between frames {} and {last_rest}",
            first_rest.map_or("-".to_string(), |f| f.to_string())
        );
        let by_size: Vec<String> = SIZE_BUCKETS
            .iter()
            .zip(peak_by_size)
            .filter(|(_, v)| *v > 0.0)
            .map(|(hi, v)| format!("<={hi}: {v:.2}"))
            .collect();
        if !by_size.is_empty() {
            println!("fastest submerged by size (cells across): {}", by_size.join(", "));
        }
    }
    println!("worst full-screen draw: {worst_draw_ms:.2} ms");
    // The sheet is written in the `if render` block above and nowhere else.
    // An unguarded second write stood here after the merge, so a rendered run
    // encoded and announced the same PNG twice, and a timing-only `repeat=`
    // pass -- which renders nothing worth keeping -- wrote and announced one
    // as well.
    (worst_ms, world, gnome, (peak_bodies, peak_tissue), cells_before, cave_before)
}

/// How much rock and rubble the world is holding: `Solid` and `Powder`
/// only.
///
/// **Liquids and gases are excluded, and that is not tidiness -- counting
/// them makes the metric lie.** The first version counted every non-empty
/// cell and reported `canyon` *gaining* 167 cells over a run in which
/// nothing failed at all. A `Liquid` cell holds continuous fill, so one
/// full cell spreading into two half-full ones is +1 occupancy at
/// unchanged volume, and a preset with standing water manufactures
/// occupancy all run just by settling. That is `CLAUDE.md`'s "ask what a
/// metric counts when nothing is wrong" catching a bad metric on the case
/// that is fine, before it was trusted about a case that is not.
///
/// `Solid` + `Powder` is also exactly the right *question*: destruction
/// turns rock into rubble and rubble into nothing, and both of those are
/// in this number while neither is in a failure count.
///
/// `Cell::is_empty()` is managed-aware -- a promoted liquid body's
/// container cells are materially empty and still read as not-empty -- so
/// this asks the material directly.
/// How much **roofed void** the world holds: empty cells with rock
/// somewhere above them in the same column.
///
/// This is the "is it still a cave" number, and it exists because nothing
/// measured that. `rock destroyed` cannot: a 2-cell roof and an 8-cell
/// roof both come down completely, but the thin one contains less rock, so
/// the *worse* outcome read as the smaller number. It conflates how badly
/// something failed with how much material happened to be in it.
///
/// A cave dies in exactly two ways and this catches both. Fill it in and
/// the empty cells stop being empty; drop its roof and they stop having
/// rock above them, becoming ordinary sky. Either way the count falls.
///
/// The column test is deliberately "any solid above", not "solid directly
/// overhead within N": a tunnel under a mountain and a tunnel under a
/// metre of crust are both caves, and picking a depth threshold would be
/// choosing which one counts.
fn roofed_void(world: &World) -> i64 {
    let mut n = 0i64;
    for x in 0..WIDTH {
        let mut roofed = false;
        for y in 0..HEIGHT {
            let m = world.get(x, y).material;
            if world.materials.kind(m) == MaterialKind::Solid {
                roofed = true;
            } else if roofed && m == material::EMPTY {
                n += 1;
            }
        }
    }
    n
}

/// Returned split into `(solid, powder)` because the two answer different
/// questions and the split is the whole reading of a destruction run.
/// Rock turning to rubble is **damage** and moves nothing out of the world;
/// rubble disappearing is **removal**. A single total cannot tell a slab
/// that shattered in place from one that was carried off, and those are
/// exactly the two things the confinement rule exists to separate.
fn census(world: &World) -> (i64, i64) {
    let (mut solid, mut powder) = (0i64, 0i64);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            match world.materials.kind(world.get(x, y).material) {
                MaterialKind::Solid => solid += 1,
                MaterialKind::Powder => powder += 1,
                _ => {}
            }
        }
    }
    (solid, powder)
}

/// The water cycle's own census: how much water there is, in whatever phase
/// it is currently in.
///
/// Measured as **fill, not occupancy**, per `CLAUDE.md`'s metric traps: a
/// `Liquid` cell holds a continuous 0..`LIQUID_FULL` amount and every
/// resting body wears a fringe of near-empty ones, so counting cells
/// overstates a spread-out pool against a settled one. Ice and snow are
/// counted as whole cells because they are whole cells -- a `Solid`'s `aux`
/// is an anchor distance and carries no fill, which is exactly why
/// `fire.rs` will only freeze a near-full cell (`FREEZE_MIN_FILL`).
///
/// Returned in cell-equivalents (fill / `LIQUID_FULL`) so the three phases
/// are on one scale and a freeze-and-thaw can be read as conservation:
/// water lost to ice at the freeze should come back at the thaw, short of
/// the partial fringe that never froze.
fn water_census(world: &World) -> (f64, i64, i64) {
    const LIQUID_FULL: f64 = 1000.0;
    let (mut liquid, mut frozen, mut snowy) = (0.0f64, 0i64, 0i64);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let cell = world.get(x, y);
            let m = world.materials.get(cell.material);
            // Keyed on the material's own phase-change fields, not on a
            // name: anything that freezes counts as liquid water, anything
            // that melts back into it counts as its solid phase. See the
            // same rule in `weather.rs`'s chill walk.
            match m.kind {
                MaterialKind::Liquid if m.cooling_point.is_finite() => {
                    // `aux == 0` on a Liquid means **full**, not empty.
                    liquid += if cell.aux() == 0 { LIQUID_FULL } else { cell.aux() as f64 } / LIQUID_FULL;
                }
                MaterialKind::Solid if m.melts_into.is_some() => frozen += 1,
                MaterialKind::Powder if m.melts_into.is_some() => snowy += 1,
                _ => {}
            }
        }
    }
    (liquid, frozen, snowy)
}

fn occupied(world: &World) -> i64 {
    let (solid, powder) = census(world);
    solid + powder
}

/// Cells with either crack bit set (`Cell::cracked`, the OR of
/// `crack_right`/`crack_down`) within a `3 * radius` box centred on a blast
/// site -- the census R1's report line needs `explosion.rs`'s own R5 doc:
/// "did the crack halo actually fire" is a counter question, the same way
/// "did the chunk-body mechanism fire" turned out to be earlier in this
/// project's history (`CLAUDE.md`, "did it fire at all" needs a counter).
/// `3x radius` rather than the crack halo's own `length` so the box stays
/// meaningful across a sweep of `crack_reach` without having to be
/// recomputed by hand each time.
/// The hottest cell in the same box, and how many cells in it are still
/// above ambient at all -- `(hottest, lit)`.
///
/// Companion to `cracked_census`, and it exists for the same "a counter,
/// not a picture" reason: whether a blast's glow is *going away* is a
/// trajectory, and one still frame of an orange ring looks identical
/// whether it is cooling or frozen forever.
fn heat_census(world: &World, cx: i32, cy: i32, radius: i32) -> (i16, u32) {
    let box_r = radius * 3;
    let (mut hottest, mut lit) = (pixel_physics::sim::cell::AMBIENT_TEMPERATURE, 0u32);
    for y in (cy - box_r)..=(cy + box_r) {
        for x in (cx - box_r)..=(cx + box_r) {
            let t = world.get(x, y).temperature();
            hottest = hottest.max(t);
            if t > pixel_physics::sim::cell::AMBIENT_TEMPERATURE {
                lit += 1;
            }
        }
    }
    (hottest, lit)
}

/// Every `Gas` cell standing in the world right now.
///
/// A *standing* count, not a count of dissipation events, and that is the
/// point: what the owner sees is a grey cap that is still there, and
/// `CLAUDE.md`'s own rule is that a complaint about something visible and
/// persistent is answered by the standing state rather than the event rate
/// (the film hunt learned that the expensive way). It is also the "did it
/// fire at all" counter for `Material::dissipation` — smoke thinning and
/// smoke drifting off the top of the crop look identical on a contact
/// sheet, and only a number distinguishes them.
///
/// Whole world rather than boxed around the blast, because smoke *leaves*:
/// a box would show the plume clearing when it had only walked out of the
/// box, which is exactly the wrong reading.
fn smoke_census(world: &World) -> u32 {
    let mut n = 0u32;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if world.materials.kind(world.get(x, y).material) == MaterialKind::Gas {
                n += 1;
            }
        }
    }
    n
}

/// Every cracked cell in the world, whichever blast scored it.
///
/// The companion the boxed census needs for the same reason `smoke_census`
/// is whole-world rather than boxed: a crack star that has grown past
/// `3 * radius` leaves the box, and a box that loses its subject reports
/// the mechanism switching off. With several charges it is also the only
/// figure that is not double-counted where two halos overlap.
fn cracked_world_census(world: &World) -> u32 {
    let mut n = 0u32;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if world.get(x, y).cracked() {
                n += 1;
            }
        }
    }
    n
}

/// **Containment, measured without asking the thing that enforces it.**
///
/// How far from the charge that made it the furthest cell of **rock that
/// stopped being rock** sits, over every charge fired in the run, in cells
/// past that charge's own radius. A cell is attributed to its nearest
/// charge, so two blasts do not blame each other.
///
/// # Why this exists, and what it replaces
///
/// `FailureCounts::max_damage_reach` cannot report a containment failure.
/// It is recorded only at sites downstream of `clip_region_to_licence`, and
/// for any cell that clip retains, `within_disturbance` guarantees some live
/// disturbance within `chain_reach + extent` while
/// `distance_to_live_disturbance` takes the **min** over disturbances of
/// `distance - extent`. So the recorded value is `<= chain_reach` by
/// arithmetic, at every site. A table reading "LOCAL 48, TIGHT 16, NONE 0"
/// against leashes of 48, 16 and 0 is a saturated ceiling, not a
/// measurement, and `CLAUDE.md` names the shape: *a debug readout must not
/// be a function of the thing it debugs.*
///
/// This reads none of that machinery -- not `chain_reach`, not
/// `World::disturbances`, not `within_disturbance`, not `licence_radius`,
/// and not the `chain_window` age test. It compares two material grids and
/// measures a distance. It can therefore return 200 at TIGHT, which is the
/// entire point of having it.
///
/// # Two numbers, and the difference matters
///
/// `(furthest, past_radius)`. The first is measured from the epicentre and
/// includes the crater the charge is *supposed* to make; the second nets
/// that off, and is the one to compare against a leash, since `chain_reach`
/// leashes the chain beyond the wound rather than the wound itself
/// (`structural::Disturbance`). Reported as `-1` when nothing changed at
/// all, never `0`: `CLAUDE.md` asks what a metric says when nothing is
/// wrong, and "perfectly contained" and "nothing happened" must not share a
/// reading.
fn damage_radius(world: &World, before: &[material::MaterialId], fired: &[FiredCharge]) -> (i32, i32) {
    if fired.is_empty() {
        return (-1, -1);
    }
    let (mut furthest, mut past) = (-1i32, -1i32);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let idx = (y * WIDTH + x) as usize;
            let was = before[idx];
            // **Rock that stopped being rock**, and nothing else. The first
            // draft of this counted any changed cell and read 296 on a
            // single charge -- almost all of it smoke drifting to the far
            // edge of the world and debris grains landing. Both are real
            // changes and neither is damage, and `CLAUDE.md` has the rule
            // this broke: the whisker hunt's metric counted every droplet
            // in the world because its definition was what falling water
            // looks like. So: the cell was `Solid` before, and is not the
            // same material now. Deposition is excluded on purpose -- a
            // grain landing on a hillside is the blast's litter, not its
            // reach.
            if world.materials.kind(was) != MaterialKind::Solid || world.get(x, y).material == was {
                continue;
            }
            // Nearest charge, so a cell that two blasts could both claim is
            // charged to the one it is actually near.
            let mut best = i32::MAX;
            let mut best_past = i32::MAX;
            for c in fired {
                let d = (x - c.x).abs().max((y - c.y).abs());
                if d < best {
                    best = d;
                    best_past = d - c.radius;
                }
            }
            furthest = furthest.max(best);
            past = past.max(best_past);
        }
    }
    (furthest, past)
}

/// **The owner's first complaint, as a number.**
///
/// > *"Chunks of rock that seem fully cracked all the way around stay put
/// > too often and don't fall into the leftover hole/crater."*
///
/// An **island** here is a maximal 4-connected component of body material
/// that the fissures have cut completely free -- every step out of it is
/// either a cracked edge into more rock, or the edge of the world. If any
/// step reaches air, gas or loose material, the piece has somewhere to go
/// and is not what he is reporting; it is excluded.
///
/// Returned as `(pieces, islands, cells, largest)` -- `pieces` is every
/// component the fissures genuinely cut loose from the massif, and `islands`
/// is the subset of those with no free face anywhere. The pair is the whole
/// point: if `pieces` is near zero the web on screen is not cutting anything
/// at all, and no amount of work on what a *severed* piece is allowed to do
/// will change what the player sees.
///
/// All are printed, never one:
/// a run with many one-cell chips stuck in the massif and one with a single
/// 200-cell slab hanging in a wall are different bugs that any single number
/// reads as the same, and `MIN_BODY_CELLS` is 8 -- an island under that was
/// never going to fly whatever the support model said.
///
/// **Why this is a counter and not a picture.** A contact sheet shows a web
/// of dark polygon outlines whether those polygons are about to come apart
/// or have been welded in place for two thousand frames, and at the zoom a
/// sheet is read at the two are indistinguishable. `CLAUDE.md`: *"did it fire
/// at all" needs a counter, not a picture.*
///
/// 4-connected, matching `load::is_supported` and `rigid::take_fragment` --
/// the two consumers that decide whether this piece is held and how it comes
/// apart. Asking in 8 would call a piece free that neither of them can
/// actually separate.
/// **§S5's decisive probe: ask the load model about the piece that is stuck,
/// with a budget it cannot run out of.**
///
/// `severed_islands` says a piece is cut free and standing. That is a fact
/// about the *world*, and it cannot say which of two very different things is
/// wrong: the model believes the piece is held up, or the model never
/// finished asking. Those call for opposite fixes and look identical on a
/// contact sheet and in every counter the scheduler prints -- measured, the
/// cap counters fire 477 times on an idle world with no blast in it at all,
/// so "which cap fired" cannot separate them either.
///
/// So: take the largest severed piece, and put its own cells to
/// `load::failing_along_support_chain` directly with a budget large enough
/// that no cap can bind. Whatever comes back is the model's *considered*
/// answer, with starvation removed as a variable.
///
/// - mostly `Failing` -> the model agrees the piece should come down, and
///   the only reason it is standing is that the real run never got that far.
/// - mostly `Holds` -> the model genuinely believes it is supported, the
///   budget story is wrong, and the support rule is what needs work.
///
/// Returns `(cells in the piece, failing, holds, deferred)`.
fn interrogate_largest_severed_piece(world: &World, near: Option<(i32, i32, i32)>) -> (usize, usize, usize, usize) {
    let piece = largest_severed_piece_near(world, near);
    let mut cache = pixel_physics::sim::load::Cache::default();
    // Two orders of magnitude past `MAX_LOAD_CELLS_PER_FRAME`, so exhausting
    // it would take a region far larger than `MAX_REGION_CELLS` admits.
    let mut budget = 100_000_000u32;
    let (mut failing, mut holds, mut deferred) = (0, 0, 0);
    for &(x, y) in &piece {
        match pixel_physics::sim::load::failing_along_support_chain(world, x, y, &mut cache, &mut budget) {
            pixel_physics::sim::load::ChainVerdict::Failing(_) => failing += 1,
            pixel_physics::sim::load::ChainVerdict::Holds => holds += 1,
            pixel_physics::sim::load::ChainVerdict::Deferred => deferred += 1,
        }
    }
    (piece.len(), failing, holds, deferred)
}

/// The cells of the largest piece the fissures have cut completely free --
/// `severed_islands`' flood, kept whole instead of counted. Same 4-connected
/// traversal and the same `edge_is_cracked` rule, for the reason that
/// function gives: asking in 8 would call a piece free that neither
/// `load::is_supported` nor `rigid::take_fragment` can actually separate.
/// The same flood, restricted to **rock a charge could actually have cut** --
/// and the reason this restriction has to exist is the whole of §S5's
/// correction.
///
/// `severed_islands` counts every maximal solid component the cracks left
/// unconnected. That set contains things no blast ever touched: on
/// `preset=terraced seed=7` the largest such piece is a **390-cell floating
/// ice sheet at x 23..108**, while the charge is at x=300 and the largest
/// piece of *stone* anywhere near it is 8 cells. The ice holds because ice
/// floats, which is right. Reported as "rock the fissures cut loose, largest
/// 376 cells, still standing", it read as a spectacular bug and was not one.
///
/// So: `near` filters to a box around the charge, and any material that
/// `floats` is excluded outright -- a floating sheet is supported by the
/// water under it and is never the thing this question is about.
fn largest_severed_piece_near(world: &World, near: Option<(i32, i32, i32)>) -> Vec<(i32, i32)> {
    use std::collections::VecDeque;
    let idx = |x: i32, y: i32| y as usize * WIDTH as usize + x as usize;
    let mut seen = vec![false; (WIDTH * HEIGHT) as usize];
    let solid = |x: i32, y: i32| {
        world.in_bounds(x, y) && matches!(world.materials.kind(world.get(x, y).material), MaterialKind::Solid)
    };
    const MAX_ISLAND: usize = 512;
    let mut best: Vec<(i32, i32)> = Vec::new();
    let admits = |piece: &[(i32, i32)]| -> bool {
        if piece.iter().any(|&(x, y)| world.materials.get(world.get(x, y).material).floats) {
            return false;
        }
        match near {
            None => true,
            Some((cx, cy, r)) => piece.iter().any(|&(x, y)| (x - cx).abs() <= r && (y - cy).abs() <= r),
        }
    };
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if seen[idx(x, y)] || !solid(x, y) {
                continue;
            }
            let mut queue = VecDeque::from([(x, y)]);
            seen[idx(x, y)] = true;
            let mut member = Vec::new();
            let mut overflowed = false;
            while let Some((cx, cy)) = queue.pop_front() {
                member.push((cx, cy));
                if member.len() > MAX_ISLAND {
                    overflowed = true;
                    break;
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if pixel_physics::sim::structural::edge_is_cracked(world, cx, cy, dx, dy) {
                        continue;
                    }
                    if !world.in_bounds(nx, ny) || !solid(nx, ny) {
                        continue;
                    }
                    if !seen[idx(nx, ny)] {
                        seen[idx(nx, ny)] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
            if overflowed {
                while let Some((cx, cy)) = queue.pop_front() {
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (cx + dx, cy + dy);
                        if pixel_physics::sim::structural::edge_is_cracked(world, cx, cy, dx, dy) || !solid(nx, ny) || seen[idx(nx, ny)] {
                            continue;
                        }
                        seen[idx(nx, ny)] = true;
                        queue.push_back((nx, ny));
                    }
                }
                continue;
            }
            if member.len() > best.len() && admits(&member) {
                best = member;
            }
        }
    }
    best
}

fn severed_islands(world: &World) -> (usize, usize, usize, usize) {
    use std::collections::VecDeque;
    let idx = |x: i32, y: i32| y as usize * WIDTH as usize + x as usize;
    let mut seen = vec![false; (WIDTH * HEIGHT) as usize];
    let solid = |x: i32, y: i32| {
        world.in_bounds(x, y) && matches!(world.materials.kind(world.get(x, y).material), MaterialKind::Solid)
    };
    // A component bigger than this is the massif, not a chunk. Bounding the
    // *walk* rather than declining to answer: the walk is abandoned and the
    // cells stay marked, so the hillside is paid for once instead of once
    // per cell in it.
    const MAX_ISLAND: usize = 512;
    let (mut pieces, mut islands, mut cells, mut largest) = (0usize, 0usize, 0usize, 0usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if seen[idx(x, y)] || !solid(x, y) {
                continue;
            }
            let mut queue = VecDeque::from([(x, y)]);
            seen[idx(x, y)] = true;
            let mut member = Vec::new();
            let mut enclosed = true;
            let mut overflowed = false;
            while let Some((cx, cy)) = queue.pop_front() {
                member.push((cx, cy));
                if member.len() > MAX_ISLAND {
                    overflowed = true;
                    break;
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if pixel_physics::sim::structural::edge_is_cracked(world, cx, cy, dx, dy) {
                        continue; // a joint: this is where the island ends
                    }
                    if !world.in_bounds(nx, ny) {
                        continue; // the world's edge holds it like rock does
                    }
                    if !solid(nx, ny) {
                        // Air, gas, powder or plant across an *uncracked*
                        // edge: this piece has an open side, so whatever is
                        // keeping it up, it is not being wedged.
                        enclosed = false;
                        continue;
                    }
                    if !seen[idx(nx, ny)] {
                        seen[idx(nx, ny)] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
            if overflowed {
                // Drain the rest so the massif is marked and not re-walked.
                while let Some((cx, cy)) = queue.pop_front() {
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (cx + dx, cy + dy);
                        if pixel_physics::sim::structural::edge_is_cracked(world, cx, cy, dx, dy) || !solid(nx, ny) || seen[idx(nx, ny)] {
                            continue;
                        }
                        seen[idx(nx, ny)] = true;
                        queue.push_back((nx, ny));
                    }
                }
                continue;
            }
            pieces += 1;
            largest = largest.max(member.len());
            if enclosed {
                islands += 1;
                cells += member.len();
            }
        }
    }
    (pieces, islands, cells, largest)
}

fn cracked_census(world: &World, cx: i32, cy: i32, radius: i32) -> u32 {
    let box_r = radius * 3;
    let mut n = 0u32;
    for y in (cy - box_r)..=(cy + box_r) {
        for x in (cx - box_r)..=(cx + box_r) {
            if world.get(x, y).cracked() {
                n += 1;
            }
        }
    }
    n
}

/// How far above its collar `FellCensus` looks for a row to cut.
///
/// A person felling a tree cuts it within reach of the ground, and the
/// number has to be bounded for a duller reason too: the thinnest row of
/// any tree is its topmost twig, so an unbounded search would report a
/// one-cell cut that severs nothing. 15 rows is about a gnome and a half
/// (`player::PLAYER_HEIGHT` is 14).
const BOLE_REACH: i32 = 15;

/// Where the felling bed puts its one trunk. See `scene=fell`.
///
/// `PlantScene` spaces a stand as `width / (trees + 1)`, so a single tree
/// in a 512-wide bed stands at 256 and stays there across species, seeds
/// and runs. Asserted rather than assumed by `FellCensus::of`, because a
/// documented coordinate that has quietly drifted is worse than none: a
/// `cut=` written against this number would then remove nothing and the
/// sheet would show an untouched tree, which is the exact failure the
/// scene exists to make impossible.
const FELL_TRUNK_X: i32 = 256;

/// **The felling census: what is still standing, what has come off, and
/// whether the mechanism that takes a severed crown down ever ran.**
///
/// `Reports/felling-blockers.md` §3 step 0 and review item D1. Three
/// separate questions, none of which a contact sheet can answer, and the
/// project has already been fooled by two of them:
///
/// - **Did the cut land?** `fire_due_cuts` prints what it removed, which
///   covers a `cut=`; a `chop=` goes through `rigid::strike` and can only
///   be read off the standing census either side of it.
/// - **Did the support pass notice?** A crown whose cells are all still
///   `support < u16::MAX` is a crown that is *genuinely still attached* by
///   some path -- a very different bug from one that is detached and
///   standing there anyway, and the two are the same picture.
/// - **Did anything then fire?** `FailureCounts::severed_organism_cells`,
///   which exists for this line. The first `scene=fell` run had a trunk
///   with 83 cells removed, a canopy that *grew* from 2,823 to 2,911 over
///   the next 210 frames, and zero in every counter the harness had.
///
/// Deadwood is counted beside the living tissue because the two are one
/// budget: a felled tree's mass has to turn up somewhere, and "standing
/// living tissue fell by 400" plus "deadwood rose by 12" is the sawdust
/// failure (`Reports/design-philosophy.md` §0a) stated numerically. It is
/// a `Powder`, so it also drains away downward -- read the fall, not the
/// level.
struct FellCensus {
    /// Live organism cells in the grid, and the shoot/root split. Roots are
    /// the tissue whose material `reinforces_powder` -- the same test
    /// `plant::is_structural_anchor` uses to decide what may anchor in
    /// soil, so the two halves of this line are the two halves of the
    /// support question.
    standing: usize,
    shoot: usize,
    root: usize,
    /// Live organisms with at least one cell in the grid.
    organisms: usize,
    /// Cells `plant::anchor_support` could not reach from any anchor:
    /// `OrganismCell::support == u16::MAX`. **Detached and still standing
    /// is the interesting state**, because it is the one the structural
    /// check is supposed to consume and the one a picture cannot show.
    detached: usize,
    /// The largest *finite* support distance in the world, against which
    /// `wood.ron`'s `max_cantilever_reach` of 96 can be read. A crown that
    /// is attached only by a path costing more than the span is about to
    /// come apart for the cantilever reason rather than the attachment one.
    furthest: u16,
    /// **Where to aim the axe**, for the largest live organism: the
    /// cheapest row to cut through within `BOLE_REACH` of its collar, as
    /// `(y, x_lo, x_hi, cells)`.
    ///
    /// This exists because the first `scene=fell` cut missed. A 16-wide
    /// rectangle centred on `FELL_TRUNK_X` looked like a felling cut and
    /// left the tree standing, and the census said why in one line where
    /// the sheet could not say it at all: the crown was **still attached**
    /// (one detached cell in the whole world, furthest finite distance 62),
    /// because `tree` at this age is not a pole with a crown on it -- it is
    /// a fan of stems spanning x 240..283 at the rows just above the
    /// ground, and the cut took the middle of it.
    ///
    /// So the number a felling harness has to print is not "where is the
    /// trunk" but "how wide is the narrowest thing an axe could sever", and
    /// that is a property of the individual, changes as it grows, and is
    /// invisible at contact-sheet zoom. Searched within reach of the collar
    /// rather than over the whole plant, because the thinnest row of *any*
    /// tree is the topmost twig and cutting it fells nothing.
    ///
    /// `None` when nothing has a collar yet (no shoot, or an organism that
    /// has not ticked).
    collar: Option<(i32, i32, i32, usize)>,
    /// Debris standing in the grid, one field per **tier**: `log` is the
    /// piece a fall leaves lying, `deadwood` the grit it is lying in, and
    /// `litter` the foliage scattered over both.
    ///
    /// Three numbers rather than one because the whole acceptance question
    /// for T1 is the *ratio* between them. One "debris" total cannot tell a
    /// felled tree from a cone of sawdust, which is precisely the reading
    /// that shipped: 2,745 cells of deadwood and 77 of litter, no log tier
    /// at all, and a picture that looked like a collapse.
    log: usize,
    deadwood: usize,
    litter: usize,
    /// Cells riding in `ChunkBody`s, and how many of them **left the grid as
    /// organism tissue** (`rigid::BodyCell::organism_id`).
    ///
    /// Keyed on the id and not on `MaterialKind::Plant`, and that is not
    /// interchangeable: a body's cells keep the material they took off with
    /// today, but the moment anything converts a piece in flight the kind
    /// test would silently report zero on a body that is entirely tree. The
    /// id is what the question is actually about.
    body_cells: usize,
    body_plant_cells: usize,
}

impl FellCensus {
    fn of(world: &World) -> Self {
        let deadwood_id = world.materials.id_of("deadwood");
        let litter_id = world.materials.id_of("litter");
        let log_id = world.materials.id_of("log");
        let (mut standing, mut shoot, mut root) = (0usize, 0usize, 0usize);
        let (mut detached, mut furthest) = (0usize, 0u16);
        let (mut deadwood, mut litter, mut log) = (0usize, 0usize, 0usize);
        let mut ids: HashSet<u16> = HashSet::new();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let cell = world.get(x, y);
                if Some(cell.material) == deadwood_id {
                    deadwood += 1;
                }
                if Some(cell.material) == litter_id {
                    litter += 1;
                }
                if Some(cell.material) == log_id {
                    log += 1;
                }
                if cell.organism_id() == 0 {
                    continue;
                }
                standing += 1;
                ids.insert(cell.organism_id());
                if world.materials.get(cell.material).reinforces_powder {
                    root += 1;
                } else {
                    shoot += 1;
                }
                // `support` is `0` for a cell the organism has not walked
                // yet, which reads as perfectly anchored. That is the
                // deferral `structural::organism_structural_tick` relies on
                // and it is honest here for the same reason: a cell with no
                // sidecar has not been asked, and counting it as detached
                // would report a severance that has not happened.
                let support = world.organism_cell(x, y).map_or(0, |c| c.support);
                if support == u16::MAX {
                    detached += 1;
                } else {
                    furthest = furthest.max(support);
                }
            }
        }
        // The largest organism, not "the only one": a felling bed grows
        // moss and drops seeds, so `scene=fell` reports two or three live
        // organisms within a few thousand frames and an "exactly one" test
        // silently gave up on the tree it was planted for.
        let subject = ids
            .iter()
            .copied()
            .filter_map(|id| Some((world.organism(id)?.cells.len(), id)))
            .max()
            .map(|(_, id)| id);
        let collar = subject.and_then(|id| {
            let state = world.organism(id)?;
            let collar_y = state.collar_y?;
            // Rows are scanned from the collar upward, and the *lowest*
            // qualifying row wins ties, because a cut low on the bole is
            // what fells a tree -- a tie broken the other way would report
            // a row near the top of the reach that severs a third of the
            // crown.
            (collar_y - BOLE_REACH..=collar_y)
                .filter_map(|y| {
                    let xs: Vec<i32> = state.cells.keys().filter(|&&(_, cy)| cy == y).map(|&(x, _)| x).collect();
                    Some((xs.len(), y, *xs.iter().min()?, *xs.iter().max()?))
                })
                .min_by_key(|&(n, y, _, _)| (n, std::cmp::Reverse(y)))
                .map(|(n, y, lo, hi)| (y, lo, hi, n))
        });
        Self {
            standing,
            shoot,
            root,
            organisms: ids.len(),
            detached,
            furthest,
            collar,
            log,
            deadwood,
            litter,
            body_cells: world.chunk_bodies.iter().map(|b| b.cells.len()).sum(),
            body_plant_cells: world.chunk_bodies.iter().flat_map(|b| b.cells.iter()).filter(|c| c.organism_id != 0).count(),
        }
    }

    fn print(&self, world: &World) {
        let aim = match self.collar {
            Some((y, lo, hi, n)) => format!("thinnest bole row y={y}: {n} cells spanning x {lo}..{hi}, so cut={lo},{y},{},1", hi - lo + 1),
            None => "no collar yet -- nothing has a bole to cut".to_string(),
        };
        println!("    felling: standing {} cells (shoot {}, root {}) in {} organism(s); {aim}", self.standing, self.shoot, self.root, self.organisms);
        println!(
            "      support: detached (unreached) {} cells, furthest finite {} of wood's 96; severed by the support check {} cells so far",
            self.detached, self.furthest, world.structural_failures.severed_organism_cells,
        );
        // **The three tiers, and the promoted share, on the two lines the
        // acceptance bar is read off.** An image says a tree came down; only
        // these say whether what came down was pieces. The share is printed
        // rather than left to be divided by eye because the pair it is taken
        // from is cumulative and world-wide, and a reader who divides the
        // wrong two numbers gets a plausible answer -- see
        // `FailureCounts::severed_organism_pieces`.
        let severed = world.structural_failures.severed_organism_cells;
        let pieces = world.structural_failures.severed_organism_pieces;
        let share = if severed == 0 { 0.0 } else { 100.0 * pieces as f64 / severed as f64 };
        println!(
            "      debris: log {} cells, deadwood {}, litter {}; bodies carrying plant material {} of {} body cells",
            self.log, self.deadwood, self.litter, self.body_plant_cells, self.body_cells,
        );
        println!("      of {severed} severed cells, {pieces} left as pieces ({share:.0}%); the rest converted where they stood");
        // **§1c, made visible rather than fixed.** A body cell with nowhere
        // to go is dropped, and a felled crown lands in a pile of its own
        // grit -- the exact configuration where the ring search comes back
        // empty. Printed here so the difference between "the fall turned to
        // dust" and "part of the fall was deleted at the moment it landed"
        // is a number on the sheet rather than a hypothesis about it.
        println!(
            "      {} of those pieces landed as {} cells of dead tissue (log + litter, cumulative over re-landings); {} cells were lost in settle (nowhere to place)",
            pieces, world.structural_failures.settled_tissue_cells, world.structural_failures.settle_lost_cells
        );
        // **The whole pile, by material, with no summarising.** The
        // three-material line above is a summary and a summary is what let
        // a claim of "refixed" go out over a picture that had barely
        // changed: the *fall* improved enormously and the settled
        // composition moved 617->631 log and 557->466 litter, which is
        // nothing, and nobody had put the two settled tables side by side.
        // Printed in full so the next reading cannot skip the check.
        //
        // Counted over the debris box only -- the columns the tree
        // occupied, from the crown down to the ground -- so the world's
        // terrain does not swamp the thing being asked about.
        {
            let mut tally: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
            let mut loose = 0usize;
            for y in 0..HEIGHT {
                for x in 200..340 {
                    let cell = world.get(x, y);
                    if cell.material == material::EMPTY || cell.attached() {
                        continue;
                    }
                    let m = world.materials.get(cell.material);
                    *tally.entry(m.name.clone()).or_default() += 1;
                    if matches!(m.kind, MaterialKind::Powder) {
                        loose += 1;
                    }
                }
            }
            let total: usize = tally.values().sum();
            let listed: Vec<String> = tally.iter().rev().map(|(n, c)| format!("{n} {c}")).collect();
            println!("      unattached debris in the fall box (x 200..340): {} cells -- {}", total, listed.join(", "));
            println!(
                "      of that, {loose} cells are a Powder kind ({:.0}% of the pile is loose grain by count)",
                if total == 0 { 0.0 } else { 100.0 * loose as f64 / total as f64 }
            );
        }
        // **Lying or standing, which is the acceptance question and not a
        // rephrasing of the counts above.** "It doesn't obviously look like
        // fallen logs" has two causes that every other number here reads
        // the same: no pieces, or pieces standing on end. See `log_pieces`.
        let p = log_pieces(world);
        let biggest: Vec<String> = p.sizes.iter().take(4).map(|&(c, w, h)| format!("{c} cells {w}x{h}")).collect();
        println!(
            "      settled log pieces (>= {} cells): {} holding {} cells -- {} lying, {} upright, {} square; largest [{}]",
            pixel_physics::sim::rigid::MIN_BODY_CELLS,
            p.sizes.len(),
            p.cells_in_pieces,
            p.lying,
            p.upright,
            p.square,
            biggest.join(", ")
        );
    }
}

/// Swing `rigid::strike` at every `chop=` whose frame has arrived --
/// **the verb, as opposed to the eraser**.
///
/// `cut=` was the starting point and it is deliberately not enough.
/// `Reports/design-philosophy.md` §0a's original sin is that destruction
/// could only be provoked by *erasing* support, "which delivers no load and
/// no impulse, so nothing ever failed from being struck": a rectangle
/// removes exactly what it names and asks the structural model a question
/// no player can ask. `strike` is the player's own `C` key -- it takes a
/// bite, loosens what is around it, scores cracks, shoves the air and
/// records a disturbance -- so a `chop=` is the thing that actually has to
/// work, and a `cut=` is the control that isolates the support model from
/// the verb.
///
/// Reports the living-tissue count either side of the blow, because that
/// is the only way to read whether it landed: `strike` returns nothing, and
/// a swing that missed the trunk by three cells and one that took half of
/// it out are the same picture at contact-sheet zoom.
fn fire_due_chops(world: &mut World, pending: &mut Vec<(i32, i32, i32, f32, usize)>, now: usize) {
    let mut i = 0;
    while i < pending.len() {
        if pending[i].4 <= now {
            let (x, y, radius, force, _) = pending.remove(i);
            let before = FellCensus::of(world);
            pixel_physics::sim::rigid::strike(world, x, y, radius, force);
            let after = FellCensus::of(world);
            println!(
                "  chop: ({x}, {y}) r{radius} force {force} at frame {now} -- living tissue {} -> {} ({} cells taken), deadwood {} -> {}",
                before.standing,
                after.standing,
                before.standing as i64 - after.standing as i64,
                before.deadwood,
                after.deadwood,
            );
        } else {
            i += 1;
        }
    }
}

/// The bite and the swing `fell=` uses when it is not told otherwise.
///
/// Radius 5 is `strike`'s own arithmetic at its most axe-like: a core of 1
/// and a chip of 3, so one bite pulverizes about five cells and loosens
/// twenty-four. Measured on `scene=fell` at frame 6,000, that is 34-37
/// cells of living tissue per blow against a bole 26 cells wide -- three
/// blows to sever it, which is about what swinging an axe at a tree ought
/// to cost. A radius that took it in one would make the verb a delete key
/// again (`Reports/design-philosophy.md` §0a).
const FELL_BITE_RADIUS: i32 = 5;

/// Force 6.0, the same order as `scene=worked`'s repeated blows. It sets
/// how hard what comes loose is thrown, not whether the cut lands.
const FELL_BITE_FORCE: f32 = 6.0;

/// Walk a blow across the subject's own bole once `fell=`'s frame arrives.
///
/// Aim is taken from `FellCensus` rather than from the arguments -- see
/// `Args::fell` for why a typed coordinate goes stale. Fires bites from the
/// left edge of the bole to its right, stepping by the bite radius so the
/// bites overlap rather than leaving uncut columns between them (the same
/// scalloping reason `mine_swept` sweeps a capsule instead of stamping
/// discs).
///
/// **Every blow lands in the same frame**, which is not how a player fells
/// a tree and is deliberate here: a harness that spread the cut over
/// several frames would let the support pass run mid-cut, so the run would
/// be measuring "what does a half-severed tree do" on a boundary nobody
/// chose. `chop=` is the arg for staging blows by hand.
///
/// Reports the whole cut as one line, including the count that says whether
/// it landed: living tissue either side of it.
fn fire_due_fell(world: &mut World, pending: &mut Option<(usize, i32, f32)>, now: usize) {
    let Some((frame, radius, force)) = *pending else { return };
    if frame > now {
        return;
    }
    *pending = None;
    let census = FellCensus::of(world);
    let Some((y, lo, hi, cells)) = census.collar else {
        println!("  fell: at frame {now} -- nothing has a bole to cut; the scene has no standing shoot");
        return;
    };
    let mut x = lo;
    let mut bites = 0;
    loop {
        pixel_physics::sim::rigid::strike(world, x, y, radius, force);
        bites += 1;
        if x >= hi {
            break;
        }
        x = (x + radius).min(hi);
    }
    let after = FellCensus::of(world);
    println!(
        "  fell: {bites} bite(s) of r{radius} across the bole at y={y}, x {lo}..{hi} ({cells} cells of tissue in that row) at frame {now} -- living tissue {} -> {}, deadwood {} -> {}",
        census.standing, after.standing, census.deadwood, after.deadwood,
    );
}
