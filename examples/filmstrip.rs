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

use pixel_physics::render::{GrainMode, Renderer};
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;
use pixel_physics::sim::rng;
use pixel_physics::sim::{explosion, material, parallel, update};

const WIDTH: i32 = 512;
const HEIGHT: i32 = 320;
const FLOOR_THICKNESS: i32 = 8;

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
        "waterbed" => {
            stone_floor(&mut w);
            for x in 20..492 {
                for y in 120..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
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
        "terrain" => {
            pixel_physics::app::build_terrain(&mut w);
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
                pixel_physics::sim::rigid::mine(&mut w, x0, (y0 + y1) / 2, args.dig);
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
            "unknown scene {other:?}; known: pour, fall, blob, sand, boom, boom_stone, sandbed, waterbed, terrain, mine, snap, undercut, strike, worked, capped, ligament, built, room, refroom"
        ),
    }
    w
}

struct Args {
    scene: String,
    /// `wall=N` / `dig=N` -- capsule radii for `scene=room`'s walls and for
    /// the cut made into them. Both default to 3, which is what the app
    /// itself does, because the app has only one number for both.
    wall: i32,
    dig: i32,
    /// `span=N` -- how wide `scene=room` is drawn, outer edge to outer
    /// edge. The knob the whole "what can a player actually build" question
    /// turns on, and the reason it is a knob rather than a constant: a
    /// single width says the room stands or does not, and what is wanted is
    /// the *envelope*.
    span: i32,
    start: usize,
    every: usize,
    count: usize,
    cols: usize,
    zoom: i32,
    crop: Rect,
    parallel_driver: bool,
    out: String,
    grain: GrainMode,
    /// Write an animated GIF of every frame in the range instead of a grid.
    /// The grid is for *me* to read; motion is for a human to watch, and
    /// some of these artifacts only read correctly in motion.
    gif: bool,
    /// `explode=x,y,radius,strength,frame` -- fire one `explosion::trigger`
    /// at the given frame. Repeatable, for several blasts in one run.
    explosions: Vec<(i32, i32, i32, f32, usize)>,
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
    /// `min_overloaded=N` / `max_failures=N` -- exit non-zero unless the
    /// run produced at least / at most that many structural failures. See
    /// `check_expectations`.
    min_overloaded: Option<u32>,
    max_failures: Option<u32>,
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
}

fn parse() -> Args {
    let mut a = Args {
        scene: "pour".into(),
        start: 100,
        every: 60,
        count: 6,
        cols: 3,
        zoom: 1,
        crop: Rect::new(0, 0, WIDTH - 1, HEIGHT - 1),
        parallel_driver: true,
        out: std::env::temp_dir().join("filmstrip.png").display().to_string(),
        grain: GrainMode::Position,
        gif: false,
        explosions: Vec::new(),
        probes: Vec::new(),
        loadmap: false,
        repeat: 1,
        min_overloaded: None,
        max_failures: None,
        max_frame_ms: None,
        min_bodies: None,
        wall: 3,
        dig: 3,
        span: 200,
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "scene" => a.scene = v.into(),
            "start" => a.start = v.parse().expect("start"),
            "every" => a.every = v.parse().expect("every"),
            "count" => a.count = v.parse().expect("count"),
            "cols" => a.cols = v.parse().expect("cols"),
            "zoom" => a.zoom = v.parse().expect("zoom"),
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
            "repeat" => a.repeat = v.parse::<usize>().expect("repeat").max(1),
            "wall" => a.wall = v.parse().expect("wall"),
            "dig" => a.dig = v.parse().expect("dig"),
            "span" => a.span = v.parse().expect("span"),
            "min_overloaded" => a.min_overloaded = Some(v.parse().expect("min_overloaded")),
            "max_failures" => a.max_failures = Some(v.parse().expect("max_failures")),
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
fn advance(world: &mut World, particles: &mut ParticleSystem, blasts: &mut explosion::Blasts, parallel_driver: bool) {
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
    world.step_active_sites();
    blasts.step(world, particles);
    particles.step(world);
    world.step_fields();
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
fn check_expectations(world: &World, args: &Args, best_ms: f64, peak_bodies: usize) -> bool {
    let f = world.structural_failures;
    let mut ok = true;
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
    if ok && (args.min_overloaded.is_some() || args.max_failures.is_some() || args.max_frame_ms.is_some() || args.min_bodies.is_some()) {
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
    let (last_ms, world, peak_bodies) = run_once(&args, true);
    samples.push(last_ms);
    let best = samples.iter().cloned().fold(f64::INFINITY, f64::min);
    if args.repeat > 1 {
        let worst = samples.iter().cloned().fold(0.0, f64::max);
        println!("worst frame over {} runs: {best:.2} ms (spread {best:.2}-{worst:.2})", args.repeat);
    }
    if !check_expectations(&world, &args, best, peak_bodies) {
        std::process::exit(1);
    }
}

/// One full run. Returns its worst frame in ms and the finished world.
/// `render` is false for the extra timing samples, which do not need an
/// image and should not pay for one.
fn run_once(args: &Args, render: bool) -> (f64, World, usize) {
    let mut world = build(args);
    let mut renderer = Renderer::new();
    renderer.grain = args.grain;
    let mut particles = ParticleSystem::new();
    let mut pending = args.explosions.clone();
    let mut blasts = explosion::Blasts::new();
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
                advance(&mut world, &mut particles, &mut blasts, args.parallel_driver);
                step_no += 1;
            }
            fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, step_no);
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
        return (0.0, world, 0);
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
    while captured < args.count {
        let target = args.start + captured * args.every;
        while step_no < target {
            fire_due_explosions(&mut world, &mut particles, &mut blasts, &mut pending, step_no);
            let began = std::time::Instant::now();
            advance(&mut world, &mut particles, &mut blasts, args.parallel_driver);
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
        let f = world.structural_failures;
        println!(
            "    failures: overloaded {} ({} cells), unsupported {} ({} cells)",
            f.overloaded, f.overloaded_cells, f.unsupported, f.unsupported_cells
        );
        println!("    furthest a failure landed from its trigger: {} cells", f.max_chain_reach);
        // Pieces or grit. A region below `MIN_FRACTURE_CELLS` is not
        // fractured at all -- it falls through to per-cell conversion,
        // which *is* powder -- so a run whose failures average 1 or 2
        // cells has already decided to produce dust no matter what the
        // fragment ladder is set to. Printed next to the image because the
        // two are indistinguishable at the zoom a contact sheet is read
        // at.
        let events = f.overloaded + f.unsupported;
        if events > 0 {
            println!(
                "    failing region size: mean {:.1} cells, largest {}",
                (f.overloaded_cells + f.unsupported_cells) as f64 / events as f64,
                f.largest_failure
            );
        }
        if render {
            println!("    worst frame so far: {worst_ms:.2} ms (frame {worst_frame})");
            report_loads(&world, args);
        }
        captured += 1;
    }

    if render {
        image::save_buffer(&args.out, &sheet, sheet_w as u32, sheet_h as u32, image::ColorType::Rgba8)
            .expect("writing the contact sheet");
        println!("contact sheet ({sheet_w}x{sheet_h}, {} tiles): {}", args.count, args.out);
    }
    (worst_ms, world, peak_bodies)
}
