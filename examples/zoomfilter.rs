//! **What the widest zoom-out actually puts on screen, under each of the
//! three `ZoomOutFilter`s, in both games.**
//!
//! Owner, from play, 2026-09-12: *"when I zoom out all the way, instead of
//! looking crisp, it looks like pixels of plants and other foreground things
//! are disappearing."* They were: the view drew one cell in `stride²` and
//! discarded the rest, so a one-cell-wide stem had three chances in four of
//! falling between sampled columns.
//!
//! Nothing already in `examples/` could answer it. `viewshot` has a `stride=`
//! and renders the real viewport, but no filter selector and no census of what
//! reached the screen; `labzoom` is one row per *zoom step* and is about how
//! far out the view may go, which is the settled argument this deliberately
//! does not reopen. What this adds is the **pairing and the counts**: three
//! filters side by side on one world in one binary, with the number a picture
//! cannot carry printed under each tile.
//!
//! ```text
//! cargo run --release --example zoomfilter
//! cargo run --release --example zoomfilter -- game=lab frames=6000 out=lab.png
//! cargo run --release --example zoomfilter -- game=world seed=7 settle=400
//! cargo run --release --example zoomfilter -- reps=8          # the timing arms, paired
//! ```
//!
//! **The census is a frame difference, not a colour match.** A palette reaches
//! a pixel through the sky light, the depth grade and the grain, so a probe
//! that looked for "wood brown" would be a test of `cell_colour` wearing the
//! name of a test about sampling. Instead the same world is rendered twice per
//! filter — once as it is, once with every plant and creature cell erased —
//! and the pixels that differ are, by construction, the pixels on which
//! something living reached the screen. The control arm makes it a positive
//! control as well: `Stride`'s count is known to be non-zero, so a zero here
//! means the probe is blind rather than the filter perfect.
//!
//! **Timing is paired and inside one run.** Two byte-identical runs have
//! disagreed 2.42x on this box, so the three filters are interleaved
//! round-robin over `reps=` passes and reported as medians — never one arm's
//! run against another arm's. `ascii` remains the frame-cost number of record;
//! this times `Renderer::draw` alone, which is the only thing the filter
//! touches.

use pixel_physics::app::{HEIGHT, WIDTH, WORLD_HEIGHT, WORLD_WIDTH};
use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::render::{Renderer, ZoomOutFilter};
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::fxhash::ChunkSet;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

/// One filter's tile: its label and its pixels.
type Tile = (String, Vec<u8>);
/// One game's row of the sheet: which game, and one tile per filter.
type Row = (String, Vec<Tile>);

const GAP: u32 = 6;
const LABEL: i32 = 9;

const FILTERS: [(ZoomOutFilter, &str); 3] =
    [(ZoomOutFilter::Stride, "STRIDE"), (ZoomOutFilter::Coverage, "COVERAGE"), (ZoomOutFilter::Average, "AVERAGE")];

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// The kinds censused separately, because **a pooled number cannot see a
/// per-kind blow-up**. An ant is two cells and a stem is one, so the kind most
/// at risk of being *over*-drawn by a max filter is exactly the one a pooled
/// plant-plus-creature count buries.
const CENSUSED: [(MaterialKind, &str); 3] =
    [(MaterialKind::Plant, "plant"), (MaterialKind::Creature, "creature"), (MaterialKind::Liquid, "liquid")];

/// Every cell of `kinds` **inside `seen`** erased, and how many there were.
///
/// The control world for the census below. Erased rather than "a world grown
/// without plants", because a second growth run is a different world and the
/// difference would then include the terrain.
///
/// **`seen` is the viewport rectangle, and taking it is the whole correctness
/// of the ratios.** Counted over the whole world instead, the count and the
/// denominator are rectangles of different sizes: on the 8192x2560 outdoor
/// world that put **145,170 sea cells** against a 2,621,440-cell viewport and
/// reported the sea drawn at 0.16x its own area, which is not a rendering
/// finding at all, just two numbers about two different rectangles.
/// `CLAUDE.md`'s standing rule -- ask what your number counts when nothing is
/// wrong -- and the tell was the same one it names: a ratio nowhere near 1 on
/// a filter that cannot lose a body of water 400 cells across.
fn without_kinds(world: &World, seen: Rect, kinds: &[MaterialKind]) -> (World, usize) {
    let mut bare = world.clone();
    let b = world.bounds().expect("a built world has bounds").intersection(seen).expect("the viewport overlaps the world");
    let mut n = 0usize;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = world.get(x, y);
            if kinds.contains(&world.materials.get(c.material).kind) {
                bare.set(x, y, Cell::EMPTY);
                n += 1;
            }
        }
    }
    bare.end_step();
    (bare, n)
}

struct Shot {
    frame: Vec<u8>,
    /// Screen pixels on which a plant or creature cell reached the screen.
    live_px: usize,
    /// Screen **columns** holding at least one of those — the quantity the
    /// complaint is about. A stand reads as sparse because of the gaps
    /// *between* stems, which a pixel total cannot express.
    live_cols: usize,
    /// Median full-redraw `Renderer::draw` time over `reps` passes, ms.
    draw_ms: f64,
    /// Median `Renderer::draw` time on a **settled** world with nothing
    /// touched and `force_full` false — the dirty-rect skip's own case, and
    /// the one `CLAUDE.md` insists on because an animated grain once looked
    /// free in every moving scene and cost ~10 ms/frame settled.
    settled_ms: f64,
    /// Pixels that skip recomputed on the settled pass. A filter that defeats
    /// the skip shows up here as a non-zero, whatever its per-pixel cost.
    settled_px: usize,
    /// Per entry of `CENSUSED`: screen pixels on which that kind reached the
    /// screen. **Paired with the true area below, in both directions**, which
    /// is the half a survival count cannot see: a max filter's characteristic
    /// failure is not dropping thin things but *over*-drawing them, and a
    /// stand that is 10% plant by area rendering as a hedge would be a worse
    /// lie than the dropout it fixes.
    kind_px: Vec<usize>,
    /// Pixels differing from the `Stride` tile — **how much of the picture
    /// this filter actually changed**, which needs no control world and works
    /// on any scene, including one with nothing alive in it. The living-cell
    /// census above is the better number where there is life to count and is
    /// simply blind where there is not; this one never is.
    moved_px: usize,
}

/// One filter's tile, its census and its timing, against `bare` as the control.
fn shoot(
    world: &World,
    bare: &World,
    kind_controls: &[World],
    filter: ZoomOutFilter,
    stride: i32,
    camera: (i32, i32),
    reps: usize,
) -> Shot {
    let particles = ParticleSystem::new();
    let bounds = world.bounds();
    let render = |w: &World| {
        let mut r = Renderer::new();
        r.zoom_out_filter = filter;
        r.set_camera(camera.0, camera.1, (WIDTH, HEIGHT), bounds);
        r.zoom_out_stride = stride;
        let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        // A full redraw every time: the arms share no renderer state, and a
        // dirty-rect skip here would photograph whatever was in the buffer.
        r.draw(w, &particles, &ChunkSet::default(), &mut buf, (WIDTH, HEIGHT), true);
        buf
    };
    let frame = render(world);
    let blank = render(bare);
    // One control render per kind, differenced against the same tile: a pixel
    // that changes when only the plants are erased is a pixel a plant reached.
    let kind_px: Vec<usize> = kind_controls
        .iter()
        .map(|c| {
            let control = render(c);
            frame.chunks_exact(4).zip(control.chunks_exact(4)).filter(|(a, b)| a != b).count()
        })
        .collect();
    let w = WIDTH as usize;
    let mut live_px = 0usize;
    let mut cols = vec![false; w];
    for (i, (a, b)) in frame.chunks_exact(4).zip(blank.chunks_exact(4)).enumerate() {
        if a != b {
            live_px += 1;
            cols[i % w] = true;
        }
    }
    Shot {
        frame,
        live_px,
        live_cols: cols.iter().filter(|c| **c).count(),
        draw_ms: f64::NAN,
        settled_ms: f64::NAN,
        settled_px: usize::MAX,
        moved_px: 0,
        kind_px,
    }
    .timed(world, filter, stride, camera, bounds, reps)
}

impl Shot {
    fn timed(
        mut self,
        world: &World,
        filter: ZoomOutFilter,
        stride: i32,
        camera: (i32, i32),
        bounds: Option<Rect>,
        reps: usize,
    ) -> Self {
        if reps == 0 {
            return self;
        }
        let particles = ParticleSystem::new();
        let mut r = Renderer::new();
        r.zoom_out_filter = filter;
        r.set_camera(camera.0, camera.1, (WIDTH, HEIGHT), bounds);
        r.zoom_out_stride = stride;
        let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        let mut times: Vec<f64> = Vec::with_capacity(reps);
        for _ in 0..reps {
            let t = std::time::Instant::now();
            r.draw(world, &particles, &ChunkSet::default(), &mut buf, (WIDTH, HEIGHT), true);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(|a, b| a.partial_cmp(b).expect("no NaN from a clock"));
        self.draw_ms = times[times.len() / 2];
        // **The settled arm.** `force_full` false with nothing touched is the
        // steady state of a world nobody is disturbing, which is exactly where
        // the dirty-rect skip earns its keep -- and exactly where a render
        // change has been free in every moving scene and ruinous at rest
        // before. The loop above has already warmed the buffer, so the skip
        // has a valid frame to reuse.
        let mut settled: Vec<f64> = Vec::with_capacity(reps);
        let mut px = 0usize;
        for _ in 0..reps {
            let t = std::time::Instant::now();
            px = r.draw(world, &particles, &ChunkSet::default(), &mut buf, (WIDTH, HEIGHT), false);
            settled.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        settled.sort_by(|a, b| a.partial_cmp(b).expect("no NaN from a clock"));
        self.settled_ms = settled[settled.len() / 2];
        self.settled_px = px;
        self
    }
}

/// The lab bed. Bigger than the viewport on purpose: `max_zoom_out_stride`
/// derives the cap from the *box*, so the shipped 512x320 bed cannot zoom out
/// at all and a sheet of it would photograph stride 1 three times.
fn lab_world(frames: u64) -> World {
    let base = LabBox::default();
    let (width, height): (i32, i32) = (arg("width").unwrap_or(2048), arg("height").unwrap_or(1280));
    // The ground rides the height, as it does on the parameters page and in
    // `labzoom` -- at the shipped `ground_y` a 1280-row box puts its soil in
    // the top eighth and the sheet photographs a scene error rather than a bed.
    let ground_y = base.ground_y * height / base.height.max(1);
    let mut lab = Lab::new(LabBox {
        width,
        height,
        ground_y,
        founders: arg("founders").unwrap_or(48),
        colonies: arg("colonies").unwrap_or(4),
        compartments: arg("walls").unwrap_or(1),
        seed: arg("seed").unwrap_or(base.seed),
        ..base
    });
    let tuning = player::Tuning::default();
    for _ in 0..frames {
        pixel_physics::sim::frame::step(
            &mut lab.world,
            &mut lab.particles,
            &mut lab.blasts,
            player::PlayerInput::default(),
            &tuning,
        );
    }
    lab.world
}

/// The outdoor world, which is 8192x2560 and so reaches the stride cap on its
/// own. `src/render.rs` is shared by both games, so the dropout is there too
/// and has to be judged there too.
fn outdoor_world(settle: u64) -> World {
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    let name: String = arg("preset").unwrap_or_else(|| presets.default_name());
    let Some(params) = presets.get(&name) else { panic!("unknown worldgen preset {name:?}") };
    let seed: u32 = arg("seed").unwrap_or(1);
    let mut world = World::new(Rect::new(0, 0, WORLD_WIDTH as i32 - 1, WORLD_HEIGHT as i32 - 1));
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed: seed as u64 });
    let mut particles = ParticleSystem::new();
    let mut blasts = pixel_physics::sim::explosion::Blasts::new();
    let tuning = player::Tuning::default();
    for _ in 0..settle {
        pixel_physics::sim::frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
    }
    world
}

fn main() {
    let game: String = arg("game").unwrap_or_else(|| "both".to_string());
    let frames: u64 = arg("frames").unwrap_or(6_000);
    let settle: u64 = arg("settle").unwrap_or(60);
    let reps: usize = arg("reps").unwrap_or(5);
    let out: String = arg("out").unwrap_or_else(|| "zoomfilter.png".to_string());
    // **`crop=x,y,w,h` then `scale=N`, in that order, and both are for the
    // review card rather than for the measurement.** The counts above are
    // taken on the whole 512x320 tile whatever these say -- a crop that also
    // moved the census would make the headline a function of the framing.
    // The review skill's own rule: the stills the owner has been able to judge
    // are 700-950 px across, and one 190x130 card came back reported as
    // showing none of the change, because at that size there was nothing to
    // see. Crop tight, then magnify.
    //
    // **One crop per row, separated by `;`**, because the two games want
    // different framings and a sheet cannot carry two tile sizes: the lab
    // bed's plants sit either side of the soil line and the outdoor surface
    // sits near the top of its tile, so a single rectangle that frames one
    // frames rock or sky in the other. Widths and heights must agree across
    // rows; only the offsets may differ. A single crop applies to every row.
    let crops: Vec<(u32, u32, u32, u32)> = arg::<String>("crop")
        .map(|v| {
            v.split(';')
                .map(|one| {
                    let p: Vec<u32> = one.split(',').map(|n| n.trim().parse().expect("crop wants x,y,w,h")).collect();
                    assert_eq!(p.len(), 4, "crop wants exactly x,y,w,h, got {one:?}");
                    assert!(
                        p[0] + p[2] <= WIDTH && p[1] + p[3] <= HEIGHT,
                        "crop {one} does not fit the {WIDTH}x{HEIGHT} tile -- an out-of-bounds crop reaches a card as a blank pane"
                    );
                    (p[0], p[1], p[2], p[3])
                })
                .collect()
        })
        .unwrap_or_default();
    assert!(
        crops.windows(2).all(|w| w[0].2 == w[1].2 && w[0].3 == w[1].3),
        "every crop must be the same size -- one sheet cannot hold two tile sizes"
    );
    let crop = crops.first().copied();
    let scale: u32 = arg("scale").unwrap_or(1);
    println!("zoomfilter: game={game} frames={frames} settle={settle} reps={reps} crop={crop:?} scale={scale} out={out}");

    let mut rows: Vec<Row> = Vec::new();
    for which in ["lab", "world"] {
        if game != "both" && game != which {
            continue;
        }
        let world = if which == "lab" { lab_world(frames) } else { outdoor_world(settle) };
        let b = world.bounds().expect("a built world has bounds");
        let (ww, wh) = (b.max_x - b.min_x + 1, b.max_y - b.min_y + 1);
        // The widest zoom-out the view is allowed over this world, reached the
        // way the player reaches it -- through `zoom_within`, so the cap this
        // round must not change decides it rather than this harness.
        let mut probe = Renderer::new();
        for _ in 0..8 {
            probe.zoom_within(-1, (WIDTH, HEIGHT), world.bounds());
        }
        let stride = probe.zoom_out_stride;
        // **Aimed at the ground, not at the middle of the world.** Centring
        // on an 8192x2560 world puts the view 1,280 rows down, which is solid
        // rock -- a tile with nothing living in it, and a census that then
        // reads 0 for every filter and looks exactly like a fix that does
        // nothing. `CLAUDE.md`: a scene that contradicts the code looks like a
        // bug in the code. `aim=x` overrides the column.
        // `aim=life` hunts for the most vegetated screenful instead, which
        // the outdoor world needs: flora is sparse there (about 1,300 living
        // cells in 8192x2560), so the middle of the world is bare rock and a
        // census taken on it reads 0 for every filter -- indistinguishable
        // from a fix that does nothing.
        let aim_x: i32 = match arg::<String>("aim").as_deref() {
            Some("life") => {
                let span = WIDTH as i32 * stride;
                let mut best = (0i32, -1i64);
                let mut x = 0;
                while x < ww {
                    let mut n = 0i64;
                    for cx in x..(x + span).min(ww) {
                        for cy in b.min_y..=b.max_y {
                            if matches!(
                                world.materials.kind(world.get(cx, cy).material),
                                MaterialKind::Plant | MaterialKind::Creature
                            ) {
                                n += 1;
                            }
                        }
                    }
                    if n > best.1 {
                        best = (x + span / 2, n);
                    }
                    x += span / 2;
                }
                println!("    aim=life: {} living cells in the best screenful, centred at x={}", best.1, best.0);
                best.0
            }
            Some(v) => v.parse().expect("aim=<column> or aim=life"),
            None => ww / 2,
        };
        let surface = (0..wh)
            .find(|&y| {
                matches!(
                    world.materials.kind(world.get(aim_x, y).material),
                    MaterialKind::Solid | MaterialKind::Powder
                )
            })
            .unwrap_or(wh / 2);
        let camera = (aim_x - (WIDTH as i32 * stride) / 2, surface - (HEIGHT as i32 * stride) / 2);
        // **The world cells the viewport actually covers**, not the world's own
        // area: the camera clamps, so on a world smaller than the view's span
        // the two agree and on a larger one they do not. The ratios below are
        // meaningless if this is taken from the wrong rectangle.
        let mut aim = Renderer::new();
        aim.zoom_out_stride = stride;
        aim.set_camera(camera.0, camera.1, (WIDTH, HEIGHT), world.bounds());
        let (span_x, span_y) = aim.visible_span((WIDTH, HEIGHT));
        let inside_x = (aim.camera_x.max(b.min_x)..(aim.camera_x + span_x).min(b.max_x + 1)).len() as f64;
        let inside_y = (aim.camera_y.max(b.min_y)..(aim.camera_y + span_y).min(b.max_y + 1)).len() as f64;
        let viewport_cells = inside_x * inside_y;
        // **The census rectangle is the viewport, and it is built from the same
        // camera the tiles are rendered through** rather than from the world's
        // own bounds -- see `without_kinds` for the reading that cost.
        let seen = Rect::new(aim.camera_x, aim.camera_y, aim.camera_x + span_x - 1, aim.camera_y + span_y - 1);
        let (bare, live_cells) = without_kinds(&world, seen, &[MaterialKind::Plant, MaterialKind::Creature]);
        let kind_worlds: Vec<(World, usize)> =
            CENSUSED.iter().map(|(k, _)| without_kinds(&world, seen, &[*k])).collect();
        let kind_controls: Vec<World> = kind_worlds.iter().map(|(w, _)| w.clone()).collect();
        println!(
            "  {which}: world {ww}x{wh}  widest stride {stride} (view {}x{}, {viewport_cells:.0} world cells on screen)  living cells present {live_cells}",
            WIDTH as i32 * stride,
            HEIGHT as i32 * stride
        );
        assert!(
            stride > 1,
            "{which} reached only stride {stride} -- at stride 1 all three filters are the same picture and this sheet answers nothing"
        );
        let mut tiles: Vec<Tile> = Vec::new();
        let mut control: Option<Vec<u8>> = None;
        for (filter, name) in FILTERS {
            let mut shot = shoot(&world, &bare, &kind_controls, filter, stride, camera, reps);
            shot.moved_px = match &control {
                Some(c) => c.chunks_exact(4).zip(shot.frame.chunks_exact(4)).filter(|(a, b)| a != b).count(),
                None => 0,
            };
            if control.is_none() {
                control = Some(shot.frame.clone());
            }
            let label = format!(
                "{name} {stride}x  LIVE PX {}  COLS {}/{}  MOVED {}  \
                 FULL {:.2}ms  SETTLED {:.2}ms ({} px)",
                shot.live_px, shot.live_cols, WIDTH, shot.moved_px, shot.draw_ms, shot.settled_ms, shot.settled_px
            );
            println!("    {label}");
            // **The inverse artifact, and it is the half a survival count is
            // blind to.** A filter that takes the most salient of 16 cells
            // can only ever draw a kind on *more* pixels than its share of the
            // area, and a stand that is a tenth plant rendering as a hedge
            // would be a worse lie than the dropout. So every kind is reported
            // against the area it actually occupies in the viewport: 1.00x is
            // areally honest, below is dropping, above is exaggerating.
            for (i, (_, kname)) in CENSUSED.iter().enumerate() {
                let cells = kind_worlds[i].1 as f64;
                let true_px = cells / viewport_cells * (WIDTH * HEIGHT) as f64;
                let got = shot.kind_px[i];
                let ratio = if true_px > 0.0 { got as f64 / true_px } else { f64::NAN };
                println!(
                    "      {kname:<9} {cells:>7.0} cells -> {true_px:>8.1} px of true area, drawn on {got:>6} px = {ratio:>5.2}x"
                );
            }
            tiles.push((label, shot.frame));
        }
        rows.push((which.to_string(), tiles));
    }
    if rows.is_empty() {
        eprintln!("game={game} selected nothing -- try game=lab, game=world or game=both");
        std::process::exit(1);
    }

    // Crop and magnify every tile, if asked. Nearest-neighbour replicate, the
    // same thing the cells themselves are drawn with -- an interpolating
    // resize on a picture whose whole subject is which cell reached which
    // pixel would answer the question by blurring it.
    let (tile_w, tile_ph) = match crop {
        Some((_, _, w, h)) => (w * scale, h * scale),
        None => (WIDTH * scale, HEIGHT * scale),
    };
    for (row, (_, tiles)) in rows.iter_mut().enumerate() {
        for (_, buf) in tiles.iter_mut() {
            let (cx, cy, cw, ch) = crops.get(row).copied().or(crop).unwrap_or((0, 0, WIDTH, HEIGHT));
            let mut cut = vec![0u8; (tile_w * tile_ph * 4) as usize];
            for y in 0..tile_ph {
                for x in 0..tile_w {
                    let src = (((cy + y / scale) * WIDTH + cx + x / scale) * 4) as usize;
                    let dst = ((y * tile_w + x) * 4) as usize;
                    cut[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
                }
            }
            let _ = (cw, ch);
            *buf = cut;
        }
    }

    // One row per game, one column per filter, so a filter is read down and a
    // game across.
    let cols = FILTERS.len() as u32;
    let tile_h = tile_ph + LABEL as u32;
    let sheet_w = cols * tile_w + (cols + 1) * GAP;
    let sheet_h = rows.len() as u32 * tile_h + (rows.len() as u32 + 1) * GAP;
    let mut sheet = vec![0u8; (sheet_w * sheet_h * 4) as usize];
    for px in sheet.chunks_exact_mut(4) {
        px.copy_from_slice(&[16, 16, 20, 255]);
    }
    for (row, (_, tiles)) in rows.iter().enumerate() {
        for (col, (label, buf)) in tiles.iter().enumerate() {
            let x0 = GAP + col as u32 * (tile_w + GAP);
            let y0 = GAP + row as u32 * (tile_h + GAP);
            for y in 0..tile_ph {
                for x in 0..tile_w {
                    let src = ((y * tile_w + x) * 4) as usize;
                    let dst = (((y0 + y + LABEL as u32) * sheet_w + x0 + x) * 4) as usize;
                    sheet[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
                }
            }
            pixel_physics::hud::draw_text(
                &mut sheet,
                sheet_w,
                sheet_h,
                x0 as i32,
                y0 as i32,
                label,
                [230, 230, 240, 255],
            );
        }
    }
    image::save_buffer(&out, &sheet, sheet_w, sheet_h, image::ColorType::Rgba8).expect("writing the sheet");
    println!("  wrote {out} ({sheet_w}x{sheet_h})");

    // **One PNG per filter as well as the sheet**, because a review card wants
    // one file per item: several files in one item become a frame sequence,
    // and three filters in one image cannot be a blind choice. Each carries
    // every game's row for that filter, stacked, and carries **no label** --
    // a blind card whose panes are captioned with their own names is not
    // blind.
    let stem = out.strip_suffix(".png").unwrap_or(&out);
    for (col, (_, name)) in FILTERS.iter().enumerate() {
        let h = rows.len() as u32 * (tile_ph + GAP) + GAP;
        let w = tile_w + 2 * GAP;
        let mut pane = vec![0u8; (w * h * 4) as usize];
        for px in pane.chunks_exact_mut(4) {
            px.copy_from_slice(&[16, 16, 20, 255]);
        }
        for (row, (_, tiles)) in rows.iter().enumerate() {
            let buf = &tiles[col].1;
            let y0 = GAP + row as u32 * (tile_ph + GAP);
            for y in 0..tile_ph {
                for x in 0..tile_w {
                    let src = ((y * tile_w + x) * 4) as usize;
                    let dst = (((y0 + y) * w + GAP + x) * 4) as usize;
                    pane[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
                }
            }
        }
        let path = format!("{stem}-{}.png", name.to_lowercase());
        image::save_buffer(&path, &pane, w, h, image::ColorType::Rgba8).expect("writing a pane");
        println!("  wrote {path} ({w}x{h})");
    }
}
