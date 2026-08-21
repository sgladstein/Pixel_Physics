//! What a generated world's caves actually *are*, as numbers, over a seed sweep.
//!
//! The companion instrument to `viewshot vault=1` renders, and deliberately
//! second to them: the render answers *what and where* — "it reads as a flat
//! smear rather than a room" — and this answers *how much*, which a render at
//! contact-sheet zoom cannot. Both were needed to reach the round-5 verdict:
//! the pictures said the chambers are not rooms, and only the census said the
//! whole system spans 4% of the world's width and its tallest open column is
//! 24 cells in a 410-row massif. A "cave beauty" argument conducted on either
//! one alone has already been run here and came out generous.
//!
//! Every quantity is measured over a *sweep*, and reported as an order
//! statistic (median / p90 / max over seeds), never a single seed —
//! CLAUDE.md's rule for anything guarding procedural content, and doubly so
//! here because cave placement is a noise draw whose per-seed outcome
//! reshuffles on any legitimate change.
//!
//! ```text
//! cargo run --release --example cave_probe                       # 16 seeds, every preset
//! cargo run --release --example cave_probe -- seeds=32 preset=canyon
//! cargo run --release --example cave_probe -- verbose=1          # per-system lines
//! ```

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::world::World;
use pixel_physics::worldgen;

/// One connected void network found under the massif, with the shape
/// quantities the beauty criteria are actually about.
#[derive(Debug, Clone)]
struct System {
    cells: usize,
    w: i32,
    h: i32,
    /// Tallest uninterrupted vertical run of void anywhere in the system —
    /// the "is any part of this a *room*" number. A passage lying along the
    /// bedding scores its own thickness here however long it is.
    tallest: i32,
    /// Height of the open column at each void column, as a distribution:
    /// the contrast between chamber and passage is `p95 / median`, which is
    /// the compression-and-release the cave photographs are built on.
    col_median: i32,
    col_p95: i32,
    /// Widest uninterrupted horizontal run — the ceiling-span bound's own
    /// quantity, kept because it is what `MAX_CEILING_SPAN` gates.
    widest: i32,
    /// Percent of this system's void the player can actually reach.
    ///
    /// **The question every other number here fails to ask.** Contrast,
    /// chamber height and formation counts all improved in round 5 while the
    /// owner's reaction to the render was "it doesn't look like I could even
    /// enter it" — because the median open column went to 4-5 cells and
    /// `PLAYER_WIDTH x PLAYER_HEIGHT` is **7 x 14**, with crouch marked
    /// "Reserved ... (phase 3)" and unimplemented. A cave tuned for
    /// compression-and-release on a per-column statistic can be a beautiful
    /// plan and a solid wall to the character walking it.
    ///
    /// Computed as a morphological opening: keep every position where the
    /// whole 7x14 box is void, flood those, then measure the void within one
    /// box of a kept position. Reported per system so a network of tight
    /// tubes joining two big rooms scores low even though both rooms are
    /// enormous.
    reachable_pct: i32,
    /// Percent of the void inside the **largest single connected** walkable
    /// region, and how many disjoint walkable regions the system has.
    ///
    /// `reachable_pct` alone cannot answer the owner's question. A system of
    /// three fine chambers strung on tubes four cells across scores well on
    /// it -- every chamber is walkable -- and is three separate caves to a
    /// player who cannot get from one to the next. What "can I enter it and
    /// walk it" means is *one* walkable region covering most of the void, so
    /// that is what these measure: 8-connected flood over the positions
    /// where the whole 7x14 box fits, then the void within one box of the
    /// biggest such region.
    walk_largest_pct: i32,
    walk_regions: i32,
}

/// Formations, measured as what the eye reads: a silhouette's height.
#[derive(Default)]
struct Formations {
    count: usize,
    cells: usize,
    crystal: usize,
    heights: Vec<i32>,
    /// Formations whose foot is within one cell of standing water.
    at_waterline: usize,
    /// Stalactite/stalagmite pairs whose tips end within 3 cells of each
    /// other, and the tallest such pair's combined height.
    near_pairs: usize,
    tallest_pair: i32,
}

fn main() {
    let mut seeds: u64 = 16;
    let mut only: String = String::new();
    let mut verbose = false;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "seeds" => seeds = v.parse().expect("seeds=N"),
            "preset" => only = v.to_string(),
            "verbose" => verbose = v != "0",
            // Handled before the sweep loop; accepted here so the arg parser
            // does not reject its own mode.
            "field" | "t" | "t3" | "halfw" | "halfh" | "cell" | "squash" | "fseed" => {}
            _ => panic!("unknown argument {arg:?}"),
        }
    }

    // The field dump is its own mode: it builds no world at all, because
    // the question it answers is about the *rule*, not about any seed.
    if let Some(t) = std::env::args().find_map(|a| a.strip_prefix("field=").map(|v| v.to_string())) {
        let _ = t;
        let get = |k: &str, d: f32| {
            std::env::args()
                .find_map(|a| a.strip_prefix(k).map(|v| v.parse().expect(k)))
                .unwrap_or(d)
        };
        dump_field(
            get("t=", 0.34),
            get("t3=", 0.55),
            get("halfw=", 90.0) as i32,
            get("halfh=", 35.0) as i32,
            get("cell=", 52.0),
            get("squash=", 2.0),
            get("fseed=", 12345.0) as u64,
        );
        return;
    }

    let (presets, err) = worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }

    println!(
        "cave census: {seeds} seeds x {} presets, world {}x{}",
        presets.cycle_order().len(),
        pixel_physics::app::WORLD_WIDTH,
        pixel_physics::app::WORLD_HEIGHT
    );
    println!();

    for name in presets.cycle_order() {
        if !only.is_empty() && name != only {
            continue;
        }
        let Some(params) = presets.get(&name) else { continue };
        let mut systems: Vec<System> = Vec::new();
        let mut forms = Formations::default();
        let mut worlds_with_none = 0usize;
        let mut void_total = 0usize;
        let mut rock_total = 0usize;
        let mut vug_list: Vec<System> = Vec::new();

        for seed in 1..=seeds {
            let bounds = Rect::new(
                0,
                0,
                pixel_physics::app::WORLD_WIDTH as i32 - 1,
                pixel_physics::app::WORLD_HEIGHT as i32 - 1,
            );
            let mut world = World::new(bounds);
            worldgen::generate(&mut world, worldgen::Spec::Generated { params, seed });

            let found = census(&world, &mut forms, &mut vug_list);
            if found.is_empty() {
                worlds_with_none += 1;
            }
            if verbose {
                for s in &found {
                    println!(
                        "  {name} s{seed}: {:>5} cells  {:>3}x{:<3}  tallest {:>3}  cols {:>2}/{:<3}  widest {:>3}",
                        s.cells, s.w, s.h, s.tallest, s.col_median, s.col_p95, s.widest
                    );
                }
            }
            systems.extend(found);
            let (v, r) = deep_area(&world);
            void_total += v;
            rock_total += r;
        }

        println!("### {name}");
        if systems.is_empty() {
            println!("  no systems in any of {seeds} seeds");
            println!();
            continue;
        }
        println!(
            "  geode vugs: {} over {seeds} seeds",
            vug_list.len()
        );
        println!(
            "  systems: {} over {seeds} seeds ({:.1}/world; {worlds_with_none} worlds with none)",
            systems.len(),
            systems.len() as f32 / seeds as f32
        );
        stat("  void cells/system  ", systems.iter().map(|s| s.cells as i32).collect());
        stat("  span across (cells)", systems.iter().map(|s| s.w).collect());
        stat("  span down (cells)  ", systems.iter().map(|s| s.h).collect());
        stat("  tallest open column", systems.iter().map(|s| s.tallest).collect());
        stat("  median open column ", systems.iter().map(|s| s.col_median).collect());
        stat("  p95 open column    ", systems.iter().map(|s| s.col_p95).collect());
        stat("  widest ceiling span", systems.iter().map(|s| s.widest).collect());
        stat("  reachable by player %", systems.iter().map(|s| s.reachable_pct).collect());
        stat("  largest walkable  %", systems.iter().map(|s| s.walk_largest_pct).collect());
        stat("  walkable regions   ", systems.iter().map(|s| s.walk_regions).collect());
        // The compression/release ratio, per system: what a photograph of a
        // cave is composed on. 1.0 is a tube of constant bore.
        let contrast: Vec<i32> = systems
            .iter()
            .map(|s| 100 * s.col_p95 / s.col_median.max(1))
            .collect();
        stat("  contrast p95/med x100", contrast);
        println!(
            "  void is {:.3}% of the deep massif ({void_total} of {} cells)",
            100.0 * void_total as f32 / (void_total + rock_total).max(1) as f32,
            void_total + rock_total
        );
        println!(
            "  formations: {} ({} cells, {} crystal, {:.1}/system)",
            forms.count,
            forms.cells,
            forms.crystal,
            forms.count as f32 / systems.len() as f32
        );
        if !forms.heights.is_empty() {
            stat("  formation height   ", forms.heights.clone());
        }
        println!(
            "  near-pairs: {} (tallest combined {} cells); formations at a waterline: {}",
            forms.near_pairs, forms.tallest_pair, forms.at_waterline
        );
        println!();
        forms = Formations::default();
    }
}

/// Void, deep enough to be cave rather than an open surface hollow. The
/// depth band matches the pass's own `vault_min_depth` intent rather than
/// reading it, because what is being measured is "what is down there", not
/// "did the knob take".
fn deep_y() -> i32 {
    pixel_physics::app::WORLD_HEIGHT as i32 / 2
}

fn deep_area(world: &World) -> (usize, usize) {
    let (mut void, mut rock) = (0usize, 0usize);
    for y in deep_y()..pixel_physics::app::WORLD_HEIGHT as i32 {
        for x in 0..pixel_physics::app::WORLD_WIDTH as i32 {
            let c = world.get(x, y);
            if c.material == material::EMPTY {
                void += 1;
            } else if world.materials.kind(c.material) == MaterialKind::Solid {
                rock += 1;
            }
        }
    }
    (void, rock)
}

/// Flood-fill every deep void component and measure it. 8-connected,
/// because that is the neighbourhood the carve writes at and a 4-connected
/// read of an 8-connected writer sees fragments (CLAUDE.md).
fn census(world: &World, forms: &mut Formations, vugs: &mut Vec<System>) -> Vec<System> {
    let (w, h) = (pixel_physics::app::WORLD_WIDTH as i32, pixel_physics::app::WORLD_HEIGHT as i32);
    let top = deep_y();
    let idx = |x: i32, y: i32| ((y - top) * w + x) as usize;
    let mut seen = vec![false; ((h - top) * w) as usize];
    let mut out = Vec::new();

    // Void here means "not solid and not loose fill" — air, and also the
    // standing water a flooded chamber holds, because a flooded passage is
    // still a passage and measuring it as stone would say a water-filled
    // system does not exist.
    let is_void = |x: i32, y: i32| {
        let c = world.get(x, y);
        c.material == material::EMPTY || world.materials.kind(c.material) == MaterialKind::Liquid
    };

    for y0 in top..h {
        for x0 in 0..w {
            if seen[idx(x0, y0)] || !is_void(x0, y0) {
                continue;
            }
            let mut stack = vec![(x0, y0)];
            let mut cells: Vec<(i32, i32)> = Vec::new();
            seen[idx(x0, y0)] = true;
            while let Some((x, y)) = stack.pop() {
                cells.push((x, y));
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || nx >= w || ny < top || ny >= h {
                            continue;
                        }
                        if seen[idx(nx, ny)] || !is_void(nx, ny) {
                            continue;
                        }
                        seen[idx(nx, ny)] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            // A component this small is a pocket edge or a stray cell, not a
            // cave — the pass's own `MIN_SYSTEM_CELLS` is 80.
            if cells.len() < 80 {
                continue;
            }
            let sh = shape(world, &cells);
            // A geode vug is a single ellipse at most ~40 cells across; a
            // cave system's envelope is 180. Counting the two together
            // averages a jewel and a gallery into a number describing
            // neither -- which is how "median span 37" came out of a sweep
            // whose systems are all 179 wide.
            if sh.w < 50 {
                vugs.push(sh);
            } else {
                measure_formations(world, &cells, forms);
                out.push(sh);
            }
        }
    }
    out
}

fn shape(world: &World, cells: &[(i32, i32)]) -> System {
    let (x0, x1) = (cells.iter().map(|c| c.0).min().unwrap(), cells.iter().map(|c| c.0).max().unwrap());
    let (y0, y1) = (cells.iter().map(|c| c.1).min().unwrap(), cells.iter().map(|c| c.1).max().unwrap());
    let (cw, ch) = ((x1 - x0 + 1) as usize, (y1 - y0 + 1) as usize);
    let mut grid = vec![false; cw * ch];
    for &(x, y) in cells {
        grid[((y - y0) as usize) * cw + (x - x0) as usize] = true;
    }
    // Per-column tallest open run, which is the quantity a player reads as
    // "how tall is the room here". A column crossed by two separate galleries
    // scores the taller one, not their sum.
    let mut col_runs: Vec<i32> = Vec::new();
    let mut tallest = 0;
    for cx in 0..cw {
        let (mut run, mut best) = (0, 0);
        for cy in 0..ch {
            if grid[cy * cw + cx] {
                run += 1;
                best = best.max(run);
            } else {
                run = 0;
            }
        }
        if best > 0 {
            col_runs.push(best);
            tallest = tallest.max(best);
        }
    }
    let mut widest = 0;
    for cy in 0..ch {
        let mut run = 0;
        for cx in 0..cw {
            if grid[cy * cw + cx] {
                run += 1;
                widest = widest.max(run);
            } else {
                run = 0;
            }
        }
    }
    // --- how much of this can the player stand in and walk through ---
    let (pw, ph) = (
        pixel_physics::sim::player::PLAYER_WIDTH as usize,
        pixel_physics::sim::player::PLAYER_HEIGHT as usize,
    );
    // **Passable is not the same as void, and conflating them made this
    // number wrong.** `grid` holds the carved void. The player also walks
    // straight through anything whose material carries `Material::scenery`
    // -- `flowstone` and `spar`, i.e. every speleothem -- so a formation
    // standing in a passage is not an obstruction to him even though it is
    // solid rock to the CA.
    //
    // Measuring the box against `grid` alone therefore counted a decorated
    // cave as impassable: with the shipped lattice, zeroing speleothem
    // density alone moved this metric 0-8% -> 32-42% with the void shape
    // completely unchanged, which is the control that proved formations,
    // not passage geometry, were what the old number was reading. Found by
    // the round-6 cave track (finding A1-1); the bug is this file's, and it
    // arrived when `Material::scenery` shipped and only the *material
    // names* here were updated.
    let passable = |wx: i32, wy: i32| -> bool {
        let cxi = (wx - x0) as usize;
        let cyi = (wy - y0) as usize;
        grid[cyi * cw + cxi] || world.materials.get(world.get(wx, wy).material).scenery
    };
    let mut fits = vec![false; cw * ch];
    if cw >= pw && ch >= ph {
        for cy in 0..=(ch - ph) {
            for cx in 0..=(cw - pw) {
                if (0..ph).all(|dy| (0..pw).all(|dx| passable(x0 + (cx + dx) as i32, y0 + (cy + dy) as i32))) {
                    fits[cy * cw + cx] = true;
                }
            }
        }
    }
    // Every void cell the player's box covers from some standable position.
    let mut reached = vec![false; cw * ch];
    for cy in 0..ch {
        for cx in 0..cw {
            if !fits[cy * cw + cx] {
                continue;
            }
            for dy in 0..ph {
                for dx in 0..pw {
                    reached[(cy + dy) * cw + cx + dx] = true;
                }
            }
        }
    }
    // --- and is it *one* cave, or several he cannot travel between? ---
    //
    // Flood the fit positions 8-connected. Two fit positions adjacent in the
    // grid mean the box slides between them, so a component is a region he
    // can walk without leaving the box's freedom -- the traversal question,
    // asked of the same box that answers the occupancy one.
    let mut region = vec![u32::MAX; cw * ch];
    let mut regions: Vec<Vec<usize>> = Vec::new();
    for start in 0..cw * ch {
        if !fits[start] || region[start] != u32::MAX {
            continue;
        }
        let id = regions.len() as u32;
        let mut stack = vec![start];
        let mut members = Vec::new();
        region[start] = id;
        while let Some(i) = stack.pop() {
            members.push(i);
            let (cx, cy) = ((i % cw) as i32, (i / cw) as i32);
            for dy in -1..=1i32 {
                for dx in -1..=1i32 {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx < 0 || ny < 0 || nx >= cw as i32 || ny >= ch as i32 {
                        continue;
                    }
                    let n = ny as usize * cw + nx as usize;
                    if fits[n] && region[n] == u32::MAX {
                        region[n] = id;
                        stack.push(n);
                    }
                }
            }
        }
        regions.push(members);
    }
    let mut walk_largest = 0usize;
    for members in &regions {
        let mut seen = vec![false; cw * ch];
        let mut n = 0;
        for &i in members {
            let (cx, cy) = (i % cw, i / cw);
            for dy in 0..ph {
                for dx in 0..pw {
                    let j = (cy + dy) * cw + cx + dx;
                    if grid[j] && !seen[j] {
                        seen[j] = true;
                        n += 1;
                    }
                }
            }
        }
        walk_largest = walk_largest.max(n);
    }
    // Numerator is reached **void**, not reached cells: now that the box may
    // sit over scenery, `reached` covers cells that are not in `cells` at all,
    // and dividing those by the void count would report more than 100% of a
    // cave as walkable. The question is still "how much of this cave's open
    // space can he get into".
    let reachable = (0..cw * ch).filter(|&i| reached[i] && grid[i]).count();
    let reachable_pct = (100 * reachable / cells.len().max(1)) as i32;

    col_runs.sort_unstable();
    let pick = |q: f32| col_runs[((col_runs.len() as f32 - 1.0) * q) as usize];
    System {
        cells: cells.len(),
        w: (x1 - x0 + 1),
        h: (y1 - y0 + 1),
        tallest,
        col_median: pick(0.5),
        col_p95: pick(0.95),
        widest,
        reachable_pct,
        walk_largest_pct: (100 * walk_largest / cells.len().max(1)) as i32,
        walk_regions: regions.len() as i32,
    }
}

/// Speleothems, read back off the world rather than counted at write time:
/// what matters is the silhouette a player sees, and a formation buried in
/// its own breakdown floor is not one however many cells the pass wrote.
fn measure_formations(world: &World, cells: &[(i32, i32)], forms: &mut Formations) {
    // **Formations are their own materials now, and this instrument has to
    // know.** They used to be written as `stone`/`crystal` with a distinct
    // palette shade; they are `flowstone`/`spar` since the scenery split. A
    // probe still looking for the old pair would report **zero formations**
    // on a world full of them -- the ruler changing meaning under the
    // measurement, which is exactly the failure this file exists to catch.
    // `stone` stays in the list because the ceiling guard's teeth and the
    // fused columns are massif stone, and both are formations to the eye.
    let stone = world.materials.id_of("stone");
    let crystal = world.materials.id_of("flowstone");
    let spar = world.materials.id_of("spar");
    let (w, h) = (pixel_physics::app::WORLD_WIDTH as i32, pixel_physics::app::WORLD_HEIGHT as i32);
    let bbox = (
        cells.iter().map(|c| c.0).min().unwrap(),
        cells.iter().map(|c| c.0).max().unwrap(),
        cells.iter().map(|c| c.1).min().unwrap(),
        cells.iter().map(|c| c.1).max().unwrap(),
    );
    let void_here: std::collections::HashSet<(i32, i32)> = cells.iter().copied().collect();
    // A formation *cell*: rock or crystal that is free-standing at that row —
    // void immediately on both sides. Applying the left/right test per row,
    // rather than once at a seed cell and then walking the column, is the
    // whole difference between measuring a stalagmite and measuring the
    // massif: the first version of this probe walked out of the cave into
    // solid rock and reported formations 437 cells tall inside a 70-cell
    // system. A number that cannot fit inside the thing it describes is the
    // tell.
    let is_form = |x: i32, y: i32| {
        let m = world.get(x, y).material;
        (Some(m) == stone || Some(m) == crystal || Some(m) == spar)
            && !void_here.contains(&(x, y))
            && void_here.contains(&(x - 1, y))
            && void_here.contains(&(x + 1, y))
    };
    // A formation is a column of stone/crystal one-or-two cells wide with
    // void on both sides — the silhouette test, walked from its attachment.
    let mut counted: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
    let mut tips_down: Vec<(i32, i32, i32)> = Vec::new(); // stalactite tip (x, y, height)
    let mut tips_up: Vec<(i32, i32, i32)> = Vec::new();
    for x in bbox.0..=bbox.1 {
        for y in bbox.2..=bbox.3 {
            if !is_form(x, y) || counted.contains(&(x, y)) {
                continue;
            }
            // Walk up and down while every row stays free-standing.
            let mut top = y;
            while top > 0 && is_form(x, top - 1) {
                top -= 1;
            }
            let mut bot = y;
            while bot + 1 < h && is_form(x, bot + 1) {
                bot += 1;
            }
            let height = bot - top + 1;
            for yy in top..=bot {
                counted.insert((x, yy));
            }
            forms.count += 1;
            forms.cells += height as usize;
            forms.heights.push(height);
            if Some(world.get(x, (top + bot) / 2).material) == spar {
                forms.crystal += 1;
            }
            // Hanging (attached above, free below) vs standing.
            let hanging = top > 0 && !void_here.contains(&(x, top - 1));
            if hanging {
                tips_down.push((x, bot, height));
            } else {
                tips_up.push((x, top, height));
            }
            // At a waterline: any liquid within one cell of the foot. A
            // hanging formation's foot is its tip, a standing one's is its
            // base -- both are `bot`, which is why there is no branch here.
            let foot = bot;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let (nx, ny) = (x + dx, foot + dy);
                    if nx < 0 || nx >= w || ny < 0 || ny >= h {
                        continue;
                    }
                    if world.materials.kind(world.get(nx, ny).material) == MaterialKind::Liquid {
                        forms.at_waterline += 1;
                        break;
                    }
                }
            }
        }
    }
    for &(dx, dy, dh) in &tips_down {
        for &(ux, uy, uh) in &tips_up {
            if (dx - ux).abs() <= 1 && uy > dy && uy - dy <= 3 {
                forms.near_pairs += 1;
                forms.tallest_pair = forms.tallest_pair.max(dh + uh);
            }
        }
    }
}

fn stat(label: &str, mut v: Vec<i32>) {
    if v.is_empty() {
        println!("{label}: -");
        return;
    }
    v.sort_unstable();
    let pick = |q: f32| v[((v.len() as f32 - 1.0) * q) as usize];
    println!(
        "{label}: med {:>5}  p90 {:>5}  max {:>5}  (n={})",
        pick(0.5),
        pick(0.9),
        v[v.len() - 1],
        v.len()
    );
}

/// ASCII dump of the cave field itself, for judging a *threshold rule*
/// before any of it is built.
///
/// Round 3 rejected a second sub-threshold on the grounds that a disc drawn
/// around a Worley feature *point* never touches the boundary web, so every
/// chamber it adds is a sealed satellite — correct, and it does not
/// generalise to the rule this dumps. `F3 - F1` is small exactly at lattice
/// *vertices*, where three cells meet, and a vertex lies **on** the web by
/// construction (`F2 - F1` is zero there). So a second threshold on `F3 - F1`
/// widens the web at its junctions and cannot disconnect. This dump is how
/// that claim was checked before it was specced, rather than after.
///
/// ```text
/// cargo run --release --example cave_probe -- field=1 t=0.34 t3=0.55
/// ```
fn dump_field(t: f32, t3: f32, half_w: i32, half_h: i32, cell: f32, squash: f32, seed: u64) {
    use pixel_physics::worldgen::noise::Purpose;

    let (mut web, mut junction, mut both) = (0usize, 0usize, 0usize);
    let mut rows: Vec<String> = Vec::new();
    for dy in -half_h..=half_h {
        let mut row = String::new();
        for dx in -half_w..=half_w {
            let v = dy as f32 * squash;
            let (f1, f2, f3) = worley3(seed, Purpose::Cave, dx as f32 / cell, v / cell);
            let fade = ((half_w - dx.abs()) as f32 / 14.0)
                .min((half_h - dy.abs()) as f32 / 7.0)
                .clamp(0.0, 1.0);
            let passage = f2 - f1 < t * fade;
            let chamber = f3 - f1 < t3 * fade;
            if passage {
                web += 1;
            }
            if chamber {
                junction += 1;
            }
            if passage && chamber {
                both += 1;
            }
            row.push(match (passage, chamber) {
                (true, true) => '#',  // web widened at a junction
                (true, false) => '.', // passage
                (false, true) => '?', // chamber not on the web -- the failure mode
                _ => ' ',
            });
        }
        rows.push(row);
    }
    for r in &rows {
        println!("{r}");
    }
    println!();
    println!(
        "envelope {}x{} at cell {cell} squash {squash} = {:.1} x {:.1} lattice cells ({:.0} in all)",
        2 * half_w + 1,
        2 * half_h + 1,
        2.0 * half_w as f32 / cell,
        2.0 * half_h as f32 * squash / cell,
        (2.0 * half_w as f32 / cell) * (2.0 * half_h as f32 * squash / cell)
    );
    println!("t={t} t3={t3}: web {web} cells ({:.0}% of envelope), junction {junction}, both {both}",
        100.0 * web as f32 / ((2 * half_w + 1) * (2 * half_h + 1)) as f32);
    // The honest connectivity test. "Is this junction cell on the web" is
    // the wrong question and its answer misleads: a bulge cell one step off
    // the `F2 - F1` strip is still part of the same room, reached through
    // its neighbours. What decides whether the second threshold adds rooms
    // or sealed satellites is whether the **union** stays one component --
    // so flood the union and report the largest component's share, which is
    // what `keep_seed_component` will actually keep.
    let w = (2 * half_w + 1) as usize;
    let h = (2 * half_h + 1) as usize;
    let open: Vec<bool> = rows
        .iter()
        .flat_map(|r| r.chars().map(|c| c != ' ').collect::<Vec<_>>())
        .collect();
    let mut seen = vec![false; w * h];
    let (mut biggest, mut parts) = (0usize, 0usize);
    for start in 0..w * h {
        if seen[start] || !open[start] {
            continue;
        }
        parts += 1;
        let mut stack = vec![start];
        seen[start] = true;
        let mut n = 0usize;
        while let Some(i) = stack.pop() {
            n += 1;
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if open[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        biggest = biggest.max(n);
    }
    let total = open.iter().filter(|&&o| o).count();
    println!(
        "  union: {total} open cells in {parts} components; largest keeps {biggest} ({:.0}%). \
         The rest is what `keep_seed_component` throws away.",
        100.0 * biggest as f32 / total.max(1) as f32
    );
    // What the eye reads as compression and release: the per-column tallest
    // open run, exactly as the world census measures it, so a field
    // experiment and a built world are comparable numbers.
    let mut runs: Vec<i32> = Vec::new();
    for x in 0..w {
        let (mut run, mut best) = (0, 0);
        for y in 0..h {
            if open[y * w + x] {
                run += 1;
                best = best.max(run);
            } else {
                run = 0;
            }
        }
        if best > 0 {
            runs.push(best);
        }
    }
    runs.sort_unstable();
    if !runs.is_empty() {
        let q = |f: f32| runs[((runs.len() as f32 - 1.0) * f) as usize];
        println!(
            "  open column: med {} p95 {} max {} -> contrast p95/med {:.1}x",
            q(0.5),
            q(0.95),
            runs[runs.len() - 1],
            q(0.95) as f32 / q(0.5).max(1) as f32
        );
    }
}

/// `worley_f2_f1` extended to the third-nearest feature point. Kept local to
/// the probe: whether this belongs in `noise.rs` is the round-5 spec's
/// question, and a probe that needs a library change before it can answer it
/// is not a probe.
fn worley3(seed: u64, purpose: pixel_physics::worldgen::noise::Purpose, x: f32, y: f32) -> (f32, f32, f32) {
    use pixel_physics::worldgen::noise::hash;
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let (mut f1, mut f2, mut f3) = (f32::MAX, f32::MAX, f32::MAX);
    for j in -1..=1 {
        for i in -1..=1 {
            let (cx, cy) = (x0 + i, y0 + j);
            let h = hash(seed, purpose, cx, cy);
            let fx = (h >> 40) as f32 / (1u64 << 24) as f32;
            let fy = ((h >> 16) & 0x00FF_FFFF) as f32 / (1u64 << 24) as f32;
            let (dx, dy) = (cx as f32 + fx - x, cy as f32 + fy - y);
            let d = (dx * dx + dy * dy).sqrt();
            if d < f1 {
                f3 = f2;
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f3 = f2;
                f2 = d;
            } else if d < f3 {
                f3 = d;
            }
        }
    }
    (f1, f2, f3)
}
