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
//! At 1:1 a one-pixel-tall dark line across a 512-wide world is genuinely
//! easy to miss — that is exactly how the horizontal chunk-seam tearing went
//! unnoticed until it was pointed out. `crop` and `zoom` are the difference
//! between "I looked at it" and "I saw it".
//!
//! ```text
//! cargo run --release --example filmstrip -- scene=pour start=100 every=60 count=6
//! cargo run --release --example filmstrip -- scene=pour crop=160,180,120,80 zoom=4
//! ```

use std::collections::HashSet;

use pixel_physics::render::{BubbleMode, FieldOverlay, GasMode, GrainMode, OrganismOverlay, Renderer};
mod common;

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::pheromone::{Channel, DEPOSIT};
use pixel_physics::sim::world::World;
use pixel_physics::sim::rng;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::{explosion, material, parallel, update};

const WIDTH: i32 = 512;
const HEIGHT: i32 = 320;
const FLOOR_THICKNESS: i32 = 8;

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
const COLDSNAP_START: u64 = 24310;
/// The frame seed 2900's snow turns to rain -- the moment the cold stops.
/// Not read by the simulation (`weather::at` is the authority) but a run
/// whose tiles all sit before this is a run that never showed a thaw, and
/// that is worth being able to see in the log.
const COLDSNAP_SNOW_ENDS: u64 = 25010;
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
fn build(args: &Args) -> World {
    let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
    // Set before the scene is built, because several scenes cut into the
    // world during construction and the rule has to be in force for that
    // cut as much as for the run that follows it.
    w.crush_confined = args.confine;
    w.arch_relief = args.arch;
    if let Some(reach) = args.chain_reach {
        w.chain_reach = reach;
    }
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
            // **The dark box under the slab in every tile is the scene, not
            // a bug.** `World::freeze_sky_surface` records top-of-ground per
            // column on the first frame and the slab is `Solid`, so its
            // sixty columns are "underground" from row `top` down and
            // render unlit for the rest of the run. Nothing can be done
            // about it from here -- the freeze is deliberately once-only
            // (`open-bugs-handoff.md` §4b records four failed attempts to
            // infer the surface instead) -- so crop below `top` when the
            // picture is for judging.
            //
            // Nothing disturbs this scene, so the first check has to be
            // asked for -- the same way `capped` does it. Without this the
            // slab hangs there and the harness reports zero of everything,
            // which reads exactly like the splash being broken.
            w.schedule_structural_check_around(256, top + 5);
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
        "grove" => {
            return common::PlantScene::default().build();
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
            let dry_surface = |w: &World, x: i32| -> Option<i32> {
                let y = (0..HEIGHT).find(|&y| !matches!(w.materials.kind(w.get(x, y).material), MaterialKind::Empty | MaterialKind::Gas))?;
                matches!(w.materials.kind(w.get(x, y).material), MaterialKind::Solid | MaterialKind::Powder).then_some(y)
            };
            // Widest stretch of dry ground nearest mid-width, so the colony
            // has somewhere to walk rather than a two-cell island.
            // **Score the thing actually wanted: how many ants would land.**
            // Scoring "dry columns within 60 cells" instead was a proxy, and
            // it picked a plateau too narrow for the colony -- 31 of 52 ants
            // placed, the rest quietly dropped into a lake. Count the real
            // 52 placement sites.
            let would_place = |w: &World, x: i32| -> i32 {
                (0..52).filter(|i| dry_surface(w, x - 102 + i * 4).is_some()).count() as i32
            };
            // Only where the colony's whole 204-cell span fits inside the
            // world: `found_colony` centres 52 ants at spacing 4, and
            // founding it near an edge silently drops every ant that lands
            // outside (16 of 52, the first time this scene ran).
            let half_span = 102;
            let (cx, cy) = (half_span..WIDTH - half_span)
                .filter_map(|x| dry_surface(&w, x).map(|y| (x, y)))
                // Most dry ground within reach, ties broken toward the
                // middle of the map. A score rather than a hard window: on a
                // wetland seed there may be no unbroken 200-cell beach, and
                // demanding one made the scene panic rather than degrade.
                .max_by_key(|&(x, _)| (would_place(&w, x), -(x - WIDTH / 2).abs()))
                .expect("some dry ground");
            let placed = w.found_colony(cx, cy - 2);
            assert!(placed > 0, "the colony scene placed no ants -- the scene is not showing what it claims to");
            println!("scene=colony genome={} : {placed} ants founded at x={cx}, surface y={cy}", args.genome);
            println!("  suggested crop: crop={},{},240,110", cx - 120, cy - 70);
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
        "tunnel" => {
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
            // One structural check at the neck, which is all a disturbance
            // would do. Nothing is removed, nothing is struck.
            w.schedule_structural_check_around(105, 152);
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
        other => panic!(
            "unknown scene {other:?}; known: pour, fall, shelf, blob, sand, boom, boom_stone, sandbed, waterbed, tree, forest, grove, terrain, worldgen, mine, snap, undercut, strike, worked, capped, ligament, built, room, refroom, worldcrack, gnome, tunnel, bury, swim, ride"
        ),
    }
    w
}

struct Args {
    scene: String,
    /// `seed=N` -- which generated world `scene=worldgen` builds.
    seed: u64,
    /// `yield=F` -- the gnome's `dig_yield`, for comparing the spoil
    /// modes the app cycles with `F2`. Whether a bore actually opens is
    /// decided entirely by this number (see `player::Tuning::dig_yield`),
    /// so a harness that could not vary it could not show the difference
    /// between "you cannot dig" and "rock simply goes".
    dig_yield: f32,
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
    cols: usize,
    zoom: i32,
    crop: Rect,
    parallel_driver: bool,
    /// `genome=` for `scene=colony`: `authored`, `zero`, or `rNNN` naming a
    /// genome from `creature_space`'s sweep by the label it printed.
    genome: String,
    out: String,
    grain: GrainMode,
    /// `bubbles=` -- which of `render.rs`'s `BubbleMode` looks to draw
    /// boiling liquid with. `off` is the default and today's behaviour;
    /// the point of the argument is `scene=boil` side by side.
    bubbles: BubbleMode,
    /// `gas=` -- which of `render.rs`'s `GasMode` looks to draw gas with.
    /// `opaque` is the default and today's behaviour.
    gas: GasMode,
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
    /// Write an animated GIF of every frame in the range instead of a grid.
    /// The grid is for *me* to read; motion is for a human to watch, and
    /// some of these artifacts only read correctly in motion.
    gif: bool,
    /// `explode=x,y,radius,strength,frame` -- fire one `explosion::trigger`
    /// at the given frame. Repeatable, for several blasts in one run.
    explosions: Vec<(i32, i32, i32, f32, usize)>,
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
    /// `chain_reach=N` -- how far from something actually disturbed a
    /// failure may happen, in cells. Unset means no limit, the shipped
    /// behaviour. `0` is "only what you struck ever fails".
    chain_reach: Option<i32>,

}

fn parse() -> Args {
    let mut a = Args {
        scene: "pour".into(),
        dig_yield: pixel_physics::sim::player::Tuning::default().dig_yield,
        seed: 1,
        preset: String::new(),
        start: 100,
        every: 60,
        count: 6,
        cols: 3,
        zoom: 1,
        genome: String::from("authored"),
        crop: Rect::new(0, 0, WIDTH - 1, HEIGHT - 1),
        parallel_driver: true,
        out: std::env::temp_dir().join("filmstrip.png").display().to_string(),
        grain: GrainMode::Position,
        bubbles: BubbleMode::Off,
        gas: GasMode::Opaque,
        organism_overlay: OrganismOverlay::Off,
        field_overlay: FieldOverlay::Off,
        gif: false,
        explosions: Vec::new(),
        cuts: Vec::new(),
        pokes: Vec::new(),
        probes: Vec::new(),
        loadmap: false,
        repeat: 1,
        min_failing_cells: None,
        min_overloaded: None,
        max_unconfined: None,
        max_failures: None,
        max_frame_ms: None,
        min_bodies: None,
        max_lost: None,
        dump: None,
        depth: None,
        step: None,
        min_cave: None,
        confine: true,
        arch: true,
        chain_reach: None,
        wall: 3,
        dig: 3,
        strike: 0,
        tunnel: 0,
        relax: false,
        span: 200,
        // A pond a player would recognise as a pond, and the width the
        // acceptance case in ice.ron's note is stated at.
        fall: 90,
        pond: 60,
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "scene" => a.scene = v.into(),
            "seed" => a.seed = v.parse().expect("seed"),
            "yield" => a.dig_yield = v.parse().expect("yield"),
            "preset" => a.preset = v.into(),
            "start" => a.start = v.parse().expect("start"),
            "every" => a.every = v.parse().expect("every"),
            "count" => a.count = v.parse().expect("count"),
            "cols" => a.cols = v.parse().expect("cols"),
            "zoom" => a.zoom = v.parse().expect("zoom"),
            "genome" => a.genome = v.to_string(),
            "driver" => a.parallel_driver = v != "serial",
            "out" => a.out = v.into(),
            "gif" => a.gif = v != "false",
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
                    "columns" => BubbleMode::Columns,
                    "surface" => BubbleMode::Surface,
                    other => panic!("unknown bubbles {other:?}"),
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
                "light" => a.field_overlay = FieldOverlay::Light,
                "moisture" => a.field_overlay = FieldOverlay::Moisture,
                "temperature" => a.field_overlay = FieldOverlay::Temperature,
                "pressure" => a.field_overlay = FieldOverlay::Pressure,
                "pheromone_a" => a.field_overlay = FieldOverlay::PheromoneA,
                "pheromone_b" => a.field_overlay = FieldOverlay::PheromoneB,
                other => panic!(
                    "unknown channel {other:?}; known: off, celltype, resource, canopy, vein, soil, light, moisture, temperature, pressure, pheromone_a, pheromone_b"
                ),
            },
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
            "min_overloaded" => a.min_overloaded = Some(v.parse().expect("min_overloaded")),
            "max_unconfined" => a.max_unconfined = Some(v.parse().expect("max_unconfined")),
            "max_failures" => a.max_failures = Some(v.parse().expect("max_failures")),
            "max_lost" => a.max_lost = Some(v.parse().expect("max_lost")),
            "depth" => a.depth = Some(v.parse().expect("depth")),
            "step" => a.step = Some(v.parse().expect("step")),
            "min_cave" => a.min_cave = Some(v.parse().expect("min_cave")),
            "dump" => {
                let n: Vec<i32> = v.split(',').map(|p| p.parse().expect("dump=x,y,w,h")).collect();
                assert_eq!(n.len(), 4, "dump=x,y,w,h");
                a.dump = Some(Rect::new(n[0], n[1], n[0] + n[2] - 1, n[1] + n[3] - 1));
            }
            "confine" => a.confine = v != "0" && v != "false",
            "arch" => a.arch = v != "0" && v != "false",
            "chain_reach" => a.chain_reach = Some(v.parse().expect("chain_reach")),
            "max_frame_ms" => a.max_frame_ms = Some(v.parse().expect("max_frame_ms")),
            "min_bodies" => a.min_bodies = Some(v.parse().expect("min_bodies")),
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
            "cut" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("cut")).collect();
                assert_eq!(n.len(), 5, "cut=x,y,w,h,frame");
                a.cuts.push((n[0], n[1], n[2], n[3], n[4] as usize));
            }
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
            other => panic!("unknown argument {other:?}"),
        }
    }
    a
}

/// Fire every scheduled explosion whose frame has arrived, removing it from
/// the pending list so it cannot fire twice. Draining rather than
/// index-matching makes this safe to call both inside the stepping loop and
/// immediately before a capture, which is what lets `frame=0` work at all
/// (with `start=0` the loop body never runs before the first tile).
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
            world.record_disturbance(x, y);
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

fn fire_due_explosions(
    world: &mut World,
    particles: &mut ParticleSystem,
    blasts: &mut explosion::Blasts,
    pending: &mut Vec<(i32, i32, i32, f32, usize)>,
    now: usize,
) {
    let mut i = 0;
    while i < pending.len() {
        if pending[i].4 <= now {
            let (x, y, r, strength, _) = pending.remove(i);
            println!("  boom: ({x}, {y}) r={r} strength={strength} at frame {now}");
            blasts.trigger_with(world, particles, x, y, r, strength);
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
    world.step_active_sites();
    blasts.step(world, particles);
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
    /// Loose cells shoved clear of a bore, summed over every bite.
    displaced: usize,
    dusted: usize,
    /// First tick he read as buried, and the first tick he was free
    /// again after that — the two numbers `scene=bury` exists to produce.
    went_under: Option<usize>,
    came_back: Option<usize>,
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
}

impl Gnome {
    fn for_scene(scene: &str, dig_yield: f32) -> Self {
        let script = match scene {
            "tunnel" => Script::Tunnel,
            "bury" => Script::Bury,
            "swim" => Script::Swim,
            "ride" => Script::Ride,
            _ => Script::Course,
        };
        Self {
            script,
            tuning: pixel_physics::sim::player::Tuning { dig_yield, ..Default::default() },
            bites: 0,
            displaced: 0,
            dusted: 0,
            went_under: None,
            came_back: None,
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
        };
        // Aim: straight ahead at his own height for the tunnel, and
        // anywhere at all while buried, since a buried bite auto-aims.
        let digging = match self.script {
            Script::Course | Script::Swim | Script::Ride => false,
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
        // How much material is left in the world at all. The one number
        // that says whether a bore can exist: `mine` conserves cells, so
        // without thinning this never moves and no cave is possible
        // however much rubble is thrown about.
        let held: usize = (0..HEIGHT)
            .map(|y| (0..WIDTH).filter(|&x| world.get(x, y).material != material::EMPTY).count())
            .sum();
        s.push_str(&format!(", world holds {held} cells"));
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
    clusters.sort_by(|a, b| b.size.cmp(&a.size));
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
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.into_iter().map(|(_, line)| line).collect()
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
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let Some(l) = pixel_physics::sim::load::evaluate(world, x, y) else { continue };
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
fn check_expectations(world: &World, args: &Args, best_ms: f64, peak_bodies: usize, cells_before: (i64, i64), cave_before: i64) -> bool {
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
    if let Some(min) = args.min_bodies {
        if peak_bodies < min {
            println!("  FAIL: expected at least {min} chunk bodies in flight at once, peaked at {peak_bodies}");
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
        && (args.min_overloaded.is_some()
            || args.min_failing_cells.is_some()
            || args.max_failures.is_some()
            || args.max_unconfined.is_some()
            || args.max_frame_ms.is_some()
            || args.min_bodies.is_some())
    {
        println!("  OK: scene={} met its expectations", args.scene);
    }
    ok
}

fn main() {
    let args = parse();
    // Repeated runs are for the *timing* only -- the image and the
    // expectations come from the last one, which is a full run like any
    // other. Deliberately re-simulated from scratch rather than reusing a
    // warm world, since a second pass over an already-settled scene
    // measures nothing.
    let mut samples: Vec<f64> = Vec::new();
    for _ in 1..args.repeat {
        samples.push(run_once(&args, false).0);
    }
    let (last_ms, world, peak_bodies, cells_before, cave_before) = run_once(&args, true);
    samples.push(last_ms);
    let best = samples.iter().cloned().fold(f64::INFINITY, f64::min);
    if args.repeat > 1 {
        let worst = samples.iter().cloned().fold(0.0, f64::max);
        println!("worst frame over {} runs: {best:.2} ms (spread {best:.2}-{worst:.2})", args.repeat);
    }
    if !check_expectations(&world, &args, best, peak_bodies, cells_before, cave_before) {
        std::process::exit(1);
    }
}

/// One full run. Returns its worst frame in ms, the finished world, the
/// peak concurrent body count and how much material the world held *before*
/// the first step. `render` is false for the extra timing samples, which do
/// not need an image and should not pay for one.
fn run_once(args: &Args, render: bool) -> (f64, World, usize, (i64, i64), i64) {
    let mut world = build(args);
    // Censused before the first step and after the last, because a failure
    // count cannot answer "how much did this eat" -- see `Args::max_lost`.
    // Taken here rather than in `build` so it includes whatever the scene
    // cut on construction: the dig is part of what the run costs.
    let cells_before = census(&world);
    let cave_before = roofed_void(&world);
    let mut renderer = Renderer::new();
    renderer.grain = args.grain;
    renderer.bubbles = args.bubbles;
    renderer.gas = args.gas;
    renderer.organism_overlay = args.organism_overlay;
    renderer.field_overlay = args.field_overlay;
    let mut particles = ParticleSystem::new();
    let mut pending = args.explosions.clone();
    let mut pending_cuts = args.cuts.clone();
    let mut pending_pokes = args.pokes.clone();
    let mut blasts = explosion::Blasts::new();
    let mut gnome = Gnome::for_scene(&args.scene, args.dig_yield);
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];

    let (cw, ch) = (args.crop.width(), args.crop.height());
    let (tile_w, tile_h) = (cw * args.zoom, ch * args.zoom);
    let gap = 2i32;
    let rows = args.count.div_ceil(args.cols) as i32;
    let sheet_w = args.cols as i32 * tile_w + (args.cols as i32 - 1) * gap;
    let sheet_h = rows * tile_h + (rows - 1) * gap;
    // Mid-grey gutters, so a tile that is legitimately all-black stays
    // distinguishable from the space between tiles.
    let mut sheet = vec![90u8; (sheet_w * sheet_h * 4) as usize];
    for p in sheet.chunks_exact_mut(4) {
        p[3] = 255;
    }

    // GIF branch: motion is for a human to watch, and several of these
    // artifacts (a fringe that regenerates every frame, water that reads as
    // frozen because its grain is nailed to the screen) simply do not survive
    // being sampled into stills. Consecutive frames, real playback speed, and
    // a NETSCAPE loop -- the same reasoning `main.rs`'s capture hook records.
    if args.gif {
        let mut frames = Vec::with_capacity(args.count);
        let mut step_no = 0usize;
        for i in 0..args.count {
            let target = args.start + i * args.every;
            while step_no < target {
                fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, step_no);
                fire_due_cuts(&mut world, &mut pending_cuts, step_no);
                fire_due_pokes(&mut world, &mut pending_pokes, step_no);
                advance(&mut world, &mut particles, &mut blasts, args.parallel_driver, step_no, &mut gnome);
                step_no += 1;
            }
            fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, step_no);
            fire_due_cuts(&mut world, &mut pending_cuts, step_no);
            fire_due_pokes(&mut world, &mut pending_pokes, step_no);
            let touched: HashSet<_> = world.take_touched_chunks();
            renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH as u32, HEIGHT as u32), true);

            let mut tile = vec![0u8; (tile_w * tile_h * 4) as usize];
            for y in 0..ch {
                for x in 0..cw {
                    let (sx, sy) = (args.crop.min_x + x, args.crop.min_y + y);
                    if sx < 0 || sy < 0 || sx >= WIDTH || sy >= HEIGHT {
                        continue;
                    }
                    let src = (((sy * WIDTH) + sx) * 4) as usize;
                    for zy in 0..args.zoom {
                        for zx in 0..args.zoom {
                            let (dx, dy) = (x * args.zoom + zx, y * args.zoom + zy);
                            let dst = (((dy * tile_w) + dx) * 4) as usize;
                            tile[dst..dst + 4].copy_from_slice(&frame[src..src + 4]);
                        }
                    }
                }
            }
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
        // The gif branch is for watching motion, not for measuring; it has
        // no per-frame timing of its own and `repeat`/expectations do not
        // apply to it.
        // The gif branch is for watching motion, not measuring: no
        // per-frame timing and no body sampling, so it reports neither.
        return (0.0, world, 0, cells_before, cave_before);
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
    let mut worst_frame = 0usize;
    // Sampled every frame, not just at capture: a body's whole life can
    // fall between two tiles, and "bodies 0" in every tile of a scene that
    // visibly threw rock is exactly the confusion this harness exists to
    // prevent.
    let mut peak_bodies = 0usize;
    // The bank at the previous tile, so the census can print a *rate* beside
    // the standing total. `None` on the first tile, which prints +0.00 rather
    // than a delta against a number that does not exist.
    let mut last_bank: Option<f64> = None;
    while captured < args.count {
        let target = args.start + captured * args.every;
        while step_no < target {
            fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, step_no);
            fire_due_cuts(&mut world, &mut pending_cuts, step_no);
            fire_due_pokes(&mut world, &mut pending_pokes, step_no);
            let began = std::time::Instant::now();
            advance(&mut world, &mut particles, &mut blasts, args.parallel_driver, step_no, &mut gnome);
            let ms = began.elapsed().as_secs_f64() * 1000.0;
            // Frame 0 is excluded, and not to flatter the number: every
            // scene spikes there, including `terrain`, which runs no
            // structural work at all. It is chunk and field allocation plus
            // first-touch page faults, paid once at startup, and leaving it
            // in made all seven scenes report the same ~70-110 ms and hid
            // the differences between them entirely.
            peak_bodies = peak_bodies.max(world.chunk_bodies.len());
            if ms > worst_ms && step_no > 0 {
                worst_ms = ms;
                worst_frame = step_no;
            }
            step_no += 1;
        }
        fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, step_no);
        fire_due_cuts(&mut world, &mut pending_cuts, step_no);
        fire_due_pokes(&mut world, &mut pending_pokes, step_no);
        // `force_full`, not the dirty-rect path: this must draw the whole
        // world every time regardless of what moved, or a tile would inherit
        // pixels from whichever frame last touched them.
        let touched: HashSet<_> = world.take_touched_chunks();
        renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH as u32, HEIGHT as u32), true);

        let (gx, gy) = (captured as i32 % args.cols as i32, captured as i32 / args.cols as i32);
        let (ox, oy) = (gx * (tile_w + gap), gy * (tile_h + gap));
        for y in 0..ch {
            for x in 0..cw {
                let (sx, sy) = (args.crop.min_x + x, args.crop.min_y + y);
                if sx < 0 || sy < 0 || sx >= WIDTH || sy >= HEIGHT {
                    continue;
                }
                let src = (((sy * WIDTH) + sx) * 4) as usize;
                for zy in 0..args.zoom {
                    for zx in 0..args.zoom {
                        let (dx, dy) = (ox + x * args.zoom + zx, oy + y * args.zoom + zy);
                        let dst = (((dy * sheet_w) + dx) * 4) as usize;
                        sheet[dst..dst + 4].copy_from_slice(&frame[src..src + 4]);
                    }
                }
            }
        }
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
        // The phase-change "did it fire at all" counters, cumulative --
        // same reasoning as the failure counts above: whether the plume on
        // screen is boiled steam or painted smoke is a question the image
        // cannot answer.
        let p = world.phase_changes;
        println!(
            "    phase changes: boiled {}, condensed {}, froze {}, melted {}, reacted {}",
            p.boiled, p.condensed, p.froze, p.melted, p.reacted
        );
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
        let (mut airborne, mut air_top, mut air_bottom) = (0u32, i32::MAX, i32::MIN);
        let steam_id = world.materials.id_of("steam");
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let cell = world.get(x, y);
                if Some(cell.material) == steam_id {
                    steam_cells += 1;
                    steam_top = steam_top.min(y);
                    steam_bottom = steam_bottom.max(y);
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
        let mut hanging_cells: Vec<(i32, i32)> = Vec::new();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let cell = world.get(x, y);
                if cell.attached() || cell.organism_id() != 0 {
                    continue;
                }
                if world.materials.kind(cell.material) != MaterialKind::Solid {
                    continue;
                }
                if pixel_physics::sim::load::evaluate(&world, x, y).is_some_and(|l| !l.supported) {
                    hanging_cells.push((x, y));
                }
            }
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
                "    plume: steam {steam_cells} cells ({}), airborne water {airborne} cells ({})",
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
        println!("    furthest a failure landed from its trigger: {} cells", f.max_chain_reach);
        // How much of the damage happened to rock with nowhere to go --
        // the mid-mountain collapse the owner reports as looking fake.
        // A picture cannot answer this: a collapse at a cliff edge and one
        // eighty cells inside a massif are the same grey rubble at the
        // zoom a contact sheet is read at.
        println!("    of those, confined (no free face anywhere): {} ({} cells), deepest {} cells from air, {} cells fissured", f.confined, f.confined_cells, f.deepest_confined, f.crushed_cells);
        // How much the world has actually *lost* since the cut was made,
        // which the failure counts above cannot say: a failed cell that
        // became rubble is still standing there. Printed per tile rather
        // than once at the end so the trajectory is visible -- a run that
        // has stopped eating and one that is still going look identical in
        // a single total. See `Args::max_lost`.
        println!("    roofed void (cave volume): {} cells, was {} at the cut", roofed_void(&world), cave_before);
        let (solid, powder) = census(&world);
        println!(
            "    cells lost since the cut: {} (rock {:+}, rubble {:+})",
            (cells_before.0 + cells_before.1) - (solid + powder),
            solid - cells_before.0,
            powder - cells_before.1
        );
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
            dump_materials(&world, args);
        }
        captured += 1;
    }

    if render {
        // **The one number a contact sheet cannot give you.** A collapse
        // rendered as coherent falling slabs was once read as "chunks are
        // working" while the body count was zero for the whole run -- what
        // was on screen was loose rubble that happened to hold its shape,
        // and the two are indistinguishable at the zoom a sheet is read at
        // (`CLAUDE.md`: "did it fire at all" needs a counter). Sampled every
        // frame rather than per tile, because a body's whole life can fall
        // between two captures.
        println!("peak chunk bodies in flight at once: {peak_bodies}");
        image::save_buffer(&args.out, &sheet, sheet_w as u32, sheet_h as u32, image::ColorType::Rgba8)
            .expect("writing the contact sheet");
        println!("contact sheet ({sheet_w}x{sheet_h}, {} tiles): {}", args.count, args.out);
    }
    (worst_ms, world, peak_bodies, cells_before, cave_before)
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
