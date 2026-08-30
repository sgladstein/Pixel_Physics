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
    /// Widest single row of each formation -- the "how thick is it" number
    /// the owner asked for (*"they should have a taper and be thicker"*),
    /// and one this instrument simply did not have until round 6. The pass
    /// reported its own *drawn* base width for a while instead, which
    /// flattered it: it recorded the width the draw intended, not the width
    /// that reached the world.
    widths: Vec<i32>,
    /// **Mean width -- body cells divided by height -- as a hundredth, so the
    /// integer `stat` printer can carry it.** `widths` above is the *widest
    /// single row*, and that is not the quantity the owner's *"they are all 1
    /// pixel thick"* is about: a trunk one cell wide for ninety rows with a
    /// twelve-cell foot in one row scores a base width of 12 and reads as a
    /// wire. The eye integrates over the whole silhouette, so the ruler has
    /// to as well.
    ///
    /// The failure is CLAUDE.md's *ask what your number counts when nothing
    /// is wrong*, in its widest-row costume: `base width` is arithmetically
    /// correct and cannot distinguish a cone from a wire with a lump on the
    /// end, which is precisely the pair the complaint separates.
    mean_widths: Vec<i32>,
    /// Bodies attached at both ceiling and floor -- true columns. Legal
    /// since Phase 0 made formations walk-through, so this is a feature
    /// count, not a defect count.
    columns: usize,
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
            // `span=1` mode's own arguments; see `span_sweep`.
            "span" | "widths" | "pillars" | "frames" | "roomh" | "leash" | "hit" | "lid" => {}
            _ => panic!("unknown argument {arg:?}"),
        }
    }

    // The span sweep is its own mode: it does not census the caves the
    // generator wrote, it cuts a room of a *chosen* width into real rock and
    // asks whether the roof stays up. See `span_sweep`.
    if std::env::args().any(|a| a == "span=1") {
        span_sweep(seeds, &only);
        return;
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
            stat("  formation base width", forms.widths.clone());
            // Printed next to the base width on purpose: the gap between the
            // two *is* the finding. A cone has a mean width near half its
            // base; a wire with a foot has a mean width near 100 (one cell)
            // whatever its base says.
            stat("  formation mean width x100", forms.mean_widths.clone());
            let wires = forms.mean_widths.iter().filter(|&&m| m < 200).count();
            println!(
                "  wire-thin formations (mean width < 2 cells): {} of {} ({:.0}%)",
                wires,
                forms.mean_widths.len(),
                100.0 * wires as f32 / forms.mean_widths.len().max(1) as f32
            );
            // **Split by height, because the aggregate hides the complaint.**
            // A quarter of formations being hairlines is survivable if they
            // are the small ones; it is the whole complaint if they are the
            // tall ones, which are what a silhouette is made of. The two
            // medians are printed side by side so the reader can see which
            // case this is without taking it on trust.
            let med_of = |mut v: Vec<i32>| {
                if v.is_empty() {
                    return -1;
                }
                v.sort_unstable();
                v[v.len() / 2]
            };
            let (mut tall_w, mut short_w) = (Vec::new(), Vec::new());
            let h_med = med_of(forms.heights.clone());
            for (h, m) in forms.heights.iter().zip(forms.mean_widths.iter()) {
                if *h > h_med { tall_w.push(*m) } else { short_w.push(*m) }
            }
            println!(
                "  mean width x100 by height: taller-than-median {} (n={}), rest {} (n={})",
                med_of(tall_w.clone()),
                tall_w.len(),
                med_of(short_w.clone()),
                short_w.len()
            );
            // **Tall *and* thin, counted on its own**, because neither of the
            // two lines above finds it. 25% hairlines is survivable if they
            // are the soda-straw fringe; the taller half being wider on
            // average says they mostly are. What ruins a photograph is the
            // formation that crosses the whole frame at one cell -- more than
            // three gnome-heights of hairline -- and that is a tail, so only
            // a count of the tail can see it. Median and mean both hide it by
            // construction.
            let eyesores = forms
                .heights
                .iter()
                .zip(forms.mean_widths.iter())
                .filter(|(h, m)| **h >= 42 && **m < 200)
                .count();
            let tall = forms.heights.iter().filter(|&&h| h >= 42).count();
            println!(
                "  hairlines over 3 gnome-heights tall: {eyesores} \
                 ({:.0}% of the {tall} formations that tall)",
                100.0 * eyesores as f32 / tall.max(1) as f32
            );
            println!("  true columns (floor to ceiling): {}", forms.columns);
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
/// The shallowest row this census will look at, **read off the world rather
/// than assumed to be half its height**.
///
/// It was `WORLD_HEIGHT / 2`, which was right when the world was
/// 2048x640 and is **1,280 rows down** at 8192x2560 -- below most of the
/// depth band `vaults` places into, which starts at `surface + 200`. This is
/// the identical defect the design lane repaired in `viewshot vault=1`
/// (`Reports/cave-redesign-2026-08-29.md` §3.5), where it printed `NO VAULT
/// in this world` on a seed whose own pass counter said `systems 1` --
/// found there and not looked for here, because the same instrument file
/// carries both readings and only one of them was under suspicion.
///
/// **Per column, not one row for the whole world**, and the first version was
/// one row: at 8192 columns the ground line moves hundreds of rows, so a
/// single cut-off taken from the *shallowest* column leaves every deeper
/// column's sky inside the window. Measured that way the census reported 25
/// "systems" in two seeds with a median span down of **17 cells** -- valley
/// air, counted as caves.
///
/// The margin below each column's own ground does two jobs: it keeps the sky
/// out of the flood, which matters now that a cave can have a **mouth** and
/// would otherwise flood-fill the whole atmosphere as one component; and it
/// keeps out brow and overhang air, which is not a cave (`brows` reaches 20
/// cells, so 60 clears it with room to spare).
fn cave_top(world: &World) -> Vec<i32> {
    let (w, h) = (pixel_physics::app::WORLD_WIDTH as i32, pixel_physics::app::WORLD_HEIGHT as i32);
    (0..w)
        .map(|x| {
            let ground = (0..h).find(|&y| world.get(x, y).material != material::EMPTY).unwrap_or(h);
            (ground + 60).min(h - 1)
        })
        .collect()
}

fn deep_area(world: &World) -> (usize, usize) {
    let (mut void, mut rock) = (0usize, 0usize);
    let tops = cave_top(world);
    for y in 0..pixel_physics::app::WORLD_HEIGHT as i32 {
        for x in 0..pixel_physics::app::WORLD_WIDTH as i32 {
            if y < tops[x as usize] {
                continue;
            }
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
    let tops = cave_top(world);
    let top = *tops.iter().min().unwrap_or(&0);
    let idx = |x: i32, y: i32| ((y - top) * w + x) as usize;
    let mut seen = vec![false; ((h - top) * w) as usize];
    let mut out = Vec::new();

    // Void here means "not solid and not loose fill" — air, and also the
    // standing water a flooded chamber holds, because a flooded passage is
    // still a passage and measuring it as stone would say a water-filled
    // system does not exist.
    let is_void = |x: i32, y: i32| {
        if y < tops[x as usize] {
            return false; // above this column's own ground: sky, not cave
        }
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
            // cave -- the pass's own `MIN_SYSTEM_CELLS` is 80.
            if cells.len() < 80 {
                continue;
            }
            // **And it has to reach the depth band, or it is not a cave.**
            // Widening the window from `WORLD_HEIGHT / 2` to each column's own
            // ground line let the census see caves that were previously
            // invisible -- and also let it see every overhang, brow-shadow and
            // valley pocket in the world. Measured before this line: 64
            // components over 16 arid seeds, **4.0 per world**, against one or
            // two systems the pass actually placed. `vault_min_depth` is 200,
            // so a cave by construction has a cell at least that far under its
            // own column's ground; nothing near the surface does.
            if !cells.iter().any(|&(x, y)| y >= tops[x as usize] + 140) {
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
///
/// **Rewritten in round 6, because the old test could not see a wide
/// formation at all.** It walked a column and required void on *both*
/// immediate flanks at every row -- a "free-standing" test, which is a
/// reasonable definition of a formation while every formation is one cell
/// wide, and is exactly wrong once they taper. A cone's own base rows have
/// a neighbour on one side, so they failed the test, and the measured
/// height truncated to whatever stuck out above the widest part. Landing
/// the cone drove measured height p90 from 18-19 down to 4-6 **while the
/// formations got bigger**, and an earlier version of the pass had already
/// been shaped around that -- its flare deliberately confined to the
/// bottom fifth of each trunk to keep this number readable. Shaping the
/// rock to please the ruler; the ruler was wrong.
///
/// Formations are their own materials since Phase 0 (`flowstone`, `spar`),
/// so the honest definition is available directly and needs no shape
/// heuristic at all: a formation is a **connected body of formation
/// material**. Flood it 8-connected -- the neighbourhood the writer uses --
/// and its height is its bounding box, its base width the widest row it
/// has. `stone` is deliberately *not* included: the ceiling guard's teeth
/// and the fused column are massif rock, and lumping wall in with
/// decoration is how this instrument lied the first time.
fn measure_formations(world: &World, cells: &[(i32, i32)], forms: &mut Formations) {
    let flowstone = world.materials.id_of("flowstone");
    let spar = world.materials.id_of("spar");
    let is_form_mat = |m| Some(m) == flowstone || Some(m) == spar;
    let (w, h) = (pixel_physics::app::WORLD_WIDTH as i32, pixel_physics::app::WORLD_HEIGHT as i32);
    let bbox = (
        cells.iter().map(|c| c.0).min().unwrap() - 1,
        cells.iter().map(|c| c.0).max().unwrap() + 1,
        cells.iter().map(|c| c.1).min().unwrap() - 1,
        cells.iter().map(|c| c.1).max().unwrap() + 1,
    );
    let void_here: std::collections::HashSet<(i32, i32)> = cells.iter().copied().collect();
    let mut seen: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
    let mut tips_down: Vec<(i32, i32, i32)> = Vec::new(); // (x, tip row, height)
    let mut tips_up: Vec<(i32, i32, i32)> = Vec::new();
    for x in bbox.0.max(0)..=bbox.1.min(w - 1) {
        for y in bbox.2.max(0)..=bbox.3.min(h - 1) {
            if seen.contains(&(x, y)) || !is_form_mat(world.get(x, y).material) {
                continue;
            }
            // 8-connected, because `carve_cave_void` writes a cone whose
            // outermost column can meet the trunk only at a corner.
            let mut stack = vec![(x, y)];
            let mut body: Vec<(i32, i32)> = Vec::new();
            seen.insert((x, y));
            while let Some((px, py)) = stack.pop() {
                body.push((px, py));
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (px + dx, py + dy);
                        if nx < 0 || ny < 0 || nx >= w || ny >= h || seen.contains(&(nx, ny)) {
                            continue;
                        }
                        if is_form_mat(world.get(nx, ny).material) {
                            seen.insert((nx, ny));
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            let (top, bot) = (
                body.iter().map(|c| c.1).min().unwrap(),
                body.iter().map(|c| c.1).max().unwrap(),
            );
            // Widest single row, which is what "how thick is it at the
            // base" means to the eye. Counted as occupied cells in the row
            // rather than the row's span, so a body with a gap in it is not
            // credited with the gap.
            let mut per_row = std::collections::HashMap::new();
            for &(_, cy) in &body {
                *per_row.entry(cy).or_insert(0i32) += 1;
            }
            let base_width = *per_row.values().max().unwrap();
            forms.count += 1;
            forms.cells += body.len();
            forms.heights.push(bot - top + 1);
            forms.widths.push(base_width);
            forms.mean_widths.push((100 * body.len() as i32) / (bot - top + 1).max(1));
            if body.iter().any(|&(cx, cy)| Some(world.get(cx, cy).material) == spar) {
                forms.crystal += 1;
            }
            // Hanging (rock above its top) vs standing (rock below its
            // base). A body attached at both ends is a true column and
            // counts as neither -- it is not a tip.
            let cx = body.iter().map(|c| c.0).sum::<i32>() / body.len() as i32;
            let hanging = !void_here.contains(&(cx, top - 1));
            let standing = !void_here.contains(&(cx, bot + 1));
            let height = bot - top + 1;
            match (hanging, standing) {
                (true, false) => tips_down.push((cx, bot, height)),
                (false, true) => tips_up.push((cx, top, height)),
                (true, true) => forms.columns += 1,
                (false, false) => {}
            }
            // At a waterline: any liquid within one cell of the body.
            if body.iter().any(|&(fx, fy)| {
                (-1..=1).any(|dx| {
                    (-1..=1).any(|dy| {
                        let (nx, ny) = (fx + dx, fy + dy);
                        nx >= 0
                            && nx < w
                            && ny >= 0
                            && ny < h
                            && world.materials.kind(world.get(nx, ny).material) == MaterialKind::Liquid
                    })
                })
            }) {
                forms.at_waterline += 1;
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

// ---------------------------------------------------------------------------
// `span=1`: how wide a room can be before its roof comes down
// ---------------------------------------------------------------------------

/// **Does a room of width `W` stand in real rock at cave depth, and what does
/// it take to hold one up?**
///
/// Written for `Reports/worldgen-caves-rebuilt-2026-08-29.md`. The cave
/// redesign asks for rooms 3-7x the 145-cell one the generator shipped, and
/// `Reports/cave-redesign-2026-08-29.md` §10 names this as the largest open
/// risk in the whole programme and says to measure it *before* the size is
/// chosen. Nothing in `Reports/instruments.md` answered it: `arch_probe`
/// sweeps a hand-built pier-and-lintel scene in a 200-row test world, and
/// `support_census` reads the distance field without ever cutting a hole.
///
/// # The two answers, and why quoting one of them alone is wrong
///
/// `World::chain_reach` is a **policy layered over** the load model: at the
/// shipped TIGHT setting a failing region is clipped to what sits near a live
/// disturbance, so an *undisturbed* generated cave is never licensed to fail
/// however wide its roof is. That is the honest answer to "does the world
/// stand at genesis", and it is not the answer to "what happens when the
/// player digs in here". So every width is run in three arms:
///
/// | arm | what it asks |
/// |---|---|
/// | `quiet` | shipped leash, nothing disturbed — does the room survive genesis |
/// | `pick` | shipped leash, one pick swing into the ceiling — what a player sets off |
/// | `model` | leash off (`without_chain_limit`) — what the load model believes, i.e. the bound |
///
/// # Controls, both mandatory
///
/// `W = 0` carves nothing and must lose **zero** rock in every arm: a
/// non-zero reading there means the census is counting the sweep's own
/// weathering rather than the collapse. And the widest arm must lose a lot
/// under `model`: a sweep that reads zero at every width has measured nothing
/// (`CLAUDE.md`, *ask what your number counts when nothing is wrong* — and
/// its other half, that it must move when something is).
///
/// ```text
/// cargo run --release --example cave_probe -- span=1 seeds=3 preset=rolling
/// cargo run --release --example cave_probe -- span=1 widths=0,128,512,1024 pillars=0,224
/// ```
fn span_sweep(seeds: u64, only: &str) {
    let arg = |k: &str| std::env::args().find_map(|a| a.strip_prefix(k).map(|v| v.to_string()));
    let list = |k: &str, d: &str| -> Vec<i32> {
        arg(k).unwrap_or_else(|| d.to_string()).split(',').map(|v| v.parse().expect(k)).collect()
    };
    let widths = list("widths=", "0,64,128,256,512,1024");
    let pillars = list("pillars=", "0");
    let frames: usize = arg("frames=").map(|v| v.parse().expect("frames=N")).unwrap_or(400);
    let room_h: i32 = arg("roomh=").map(|v| v.parse().expect("roomh=N")).unwrap_or(160);
    // **The positive control.** `lid=T` cuts a second void above the room,
    // leaving the roof as a genuine slab `T` cells thick spanning the whole
    // width -- the case whose answer is *known* to be non-zero. A sweep whose
    // widest room loses a few hundred cells is only believable if the
    // instrument can also report a real collapse, and this is the arm that
    // proves it can (`CLAUDE.md`: run the case whose answer you know is
    // non-zero and check the instrument reports it).
    let lid: i32 = arg("lid=").map(|v| v.parse().expect("lid=N")).unwrap_or(0);

    let (presets, err) = worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    // Echoes its own parameters -- a log that does not name its sweep was
    // written by a binary that may not have had one (`CLAUDE.md`).
    println!(
        "cave span sweep: widths {widths:?} pillars {pillars:?} room_h {room_h} frames {frames} \
         lid {lid} seeds {seeds} world {}x{}",
        pixel_physics::app::WORLD_WIDTH,
        pixel_physics::app::WORLD_HEIGHT
    );
    println!("  gnome is {PLAYER_W}x{PLAYER_H}; stone spans {} attached", stone_span());
    println!();

    for name in presets.cycle_order() {
        if !only.is_empty() && name != only {
            continue;
        }
        let Some(params) = presets.get(&name) else { continue };
        println!("### {name}");
        println!(
            "  {:>6} {:>7} | {:>8} {:>8} {:>8} | {:>7} {:>7}",
            "width", "pillars", "quiet", "pick", "model", "fail_q", "fail_m"
        );
        for &w in &widths {
            for &p in &pillars {
                if w == 0 && p != pillars[0] {
                    continue; // the null control needs running once, not once per pillar arm
                }
                let (mut quiet, mut pick, mut model) = (0i64, 0i64, 0i64);
                let (mut fq, mut fm) = (0u64, 0u64);
                for seed in 1..=seeds {
                    let (a, faq) = one_room(params, seed, w, p, room_h, frames, lid, Arm::Quiet);
                    let (b, _) = one_room(params, seed, w, p, room_h, frames, lid, Arm::Pick);
                    let (c, fam) = one_room(params, seed, w, p, room_h, frames, lid, Arm::Model);
                    quiet += a;
                    pick += b;
                    model += c;
                    fq += faq;
                    fm += fam;
                }
                let n = seeds as i64;
                println!(
                    "  {w:>6} {p:>7} | {:>8} {:>8} {:>8} | {fq:>7} {fm:>7}",
                    quiet / n,
                    pick / n,
                    model / n
                );
            }
        }
        println!();
    }
}

/// The gnome's box, from `player.rs`, so the room heights below are quoted in
/// something the owner can see.
const PLAYER_W: i32 = 7;
const PLAYER_H: i32 = 14;

fn stone_span() -> i32 {
    let w = World::new(Rect::new(0, 0, 1, 1));
    let m = w.materials.get(material::STONE);
    m.max_unsupported_span as i32 * m.attached_span_bonus as i32
}

#[derive(Clone, Copy, PartialEq)]
enum Arm {
    /// Shipped leash, undisturbed.
    Quiet,
    /// Shipped leash, one pick swing into the middle of the ceiling.
    Pick,
    /// Leash off: what the load model believes with no policy over it.
    Model,
}

/// Cut one room and report `(rock cells lost around it, structural failures)`.
///
/// The world is rebuilt per arm rather than cloned, which costs a few seconds
/// and buys that no arm can inherit another's damage.
#[allow(clippy::too_many_arguments)]
fn one_room(
    params: &pixel_physics::worldgen::WorldgenParams,
    seed: u64,
    room_w: i32,
    pillar_pitch: i32,
    room_h: i32,
    frames: usize,
    lid: i32,
    arm: Arm,
) -> (i64, u64) {
    use pixel_physics::sim::cell::Cell;
    use pixel_physics::sim::{parallel, rigid, structural};

    let bounds = Rect::new(
        0,
        0,
        pixel_physics::app::WORLD_WIDTH as i32 - 1,
        pixel_physics::app::WORLD_HEIGHT as i32 - 1,
    );
    let mut world = World::new(bounds);
    worldgen::generate(&mut world, worldgen::Spec::Generated { params, seed });
    if arm == Arm::Model {
        world = world.without_chain_limit();
    }

    // The site: the middle of the world, in the middle of the depth band the
    // cave pass places into, so the rock around it is ordinary massif rather
    // than anything near a surface or the bedrock floor.
    let w = pixel_physics::app::WORLD_WIDTH as i32;
    let cx = w / 2;
    let (top, bottom) = cave_band(&world, params, seed, cx);
    let cy = (top + bottom) / 2;

    // Census box: the room plus a generous apron, so a cascade running out
    // past the room's own walls is still counted.
    let apron = 400;
    let box_x = (cx - room_w / 2 - apron, cx + room_w / 2 + apron);
    let box_y = (cy - room_h / 2 - apron, cy + room_h / 2 + apron);

    if room_w > 0 {
        // Pillars are left standing, not added: a pillar here is rock the cut
        // did not take, which is what a collapse survivor is.
        let x0 = cx - room_w / 2;
        for y in cy - room_h / 2..=cy + room_h / 2 {
            for x in x0..=cx + room_w / 2 {
                if pillar_pitch > 0 && (x - x0).rem_euclid(pillar_pitch) < PILLAR_W {
                    continue;
                }
                if world.in_bounds(x, y) {
                    world.set(x, y, Cell::EMPTY);
                }
            }
        }
    }
    if lid > 0 && room_w > 0 {
        // The control: take the rock above the roof away too, so what is left
        // spanning the room is a slab and not a massif.
        let x0 = cx - room_w / 2;
        for y in cy - room_h / 2 - lid - 200..=cy - room_h / 2 - lid - 1 {
            for x in x0..=cx + room_w / 2 {
                if pillar_pitch > 0 && (x - x0).rem_euclid(pillar_pitch) < PILLAR_W {
                    continue;
                }
                if world.in_bounds(x, y) {
                    world.set(x, y, Cell::EMPTY);
                }
            }
        }
    }
    // The field has to be re-solved after the cut, exactly as generation
    // solves it after the passes: without this every roof cell keeps the
    // distance it had while the rock under it was still there.
    structural::compute_world_distances(&mut world);

    if arm == Arm::Model && room_w > 0 {
        // **Hand-cut geometry reaches the structural heap through nothing.**
        // `arch_probe` carries the same pair for the same reason: a scene
        // nothing ever asks about stands because it was never questioned,
        // which is `CLAUDE.md`'s vacuous-test failure in scene form. The
        // first run of this sweep did not have these two lines and reported
        // **zero rock lost at every width including 2048** -- a clean,
        // arithmetically-correct null about a question nobody had asked.
        for y in cy - room_h / 2 - 40..=cy + room_h / 2 + 40 {
            for x in cx - room_w / 2 - 40..=cx + room_w / 2 + 40 {
                if world.in_bounds(x, y) && world.materials.get(world.get(x, y).material).rock {
                    world.schedule_structural_check(x, y);
                }
            }
        }
        world.record_disturbance(cx, cy, room_w / 2 + 40);
    }

    let before = rock_in(&world, box_x, box_y);
    if arm == Arm::Pick && room_w > 0 {
        // One swing into the middle of the ceiling -- the verb, at the
        // shipped brush radius. `mine` reports what it loosened; the census
        // below is what actually left.
        rigid::mine(&mut world, cx, cy - room_h / 2 - 1, 6, 1.0);
    }
    for _ in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        rigid::step_chunk_bodies(&mut world);
    }
    let after = rock_in(&world, box_x, box_y);
    (
        before as i64 - after as i64,
        u64::from(world.structural_failures.unsupported) + u64::from(world.structural_failures.overloaded),
    )
}

/// How wide a pillar the sweep leaves standing. Two gnomes across: narrower
/// and it is a formation rather than a support, and the question here is
/// about support.
const PILLAR_W: i32 = 14;

/// The depth band `vaults` places into, read from the same plan the pass
/// reads so the probe cannot drift from it.
fn cave_band(
    world: &World,
    params: &pixel_physics::worldgen::WorldgenParams,
    seed: u64,
    x: i32,
) -> (i32, i32) {
    let bounds = world.bounds().expect("bounded");
    let soil = world.materials.get(world.materials.id_of("soil").expect("soil")).friction_angle.to_radians().tan();
    let sand = world.materials.get(world.materials.id_of("sand").expect("sand")).friction_angle.to_radians().tan();
    let terrain = pixel_physics::worldgen::column::Terrain::new(
        seed,
        params,
        bounds.max_x + 1,
        bounds.max_y + 1,
        soil,
        sand,
    );
    let plan = terrain.plan(x);
    (plan.surface_y + params.vault_min_depth, plan.bedrock_top_y - params.vault_bedrock_margin)
}

/// Intact rock cells inside a box. **A census, not a failure count** --
/// `CLAUDE.md`'s metric trap: a failed cell that became rubble is still
/// standing there, and two digs whose event counts looked comparable removed
/// 894 and 23,042 cells.
fn rock_in(world: &World, xs: (i32, i32), ys: (i32, i32)) -> usize {
    let mut n = 0;
    for y in ys.0..=ys.1 {
        for x in xs.0..=xs.1 {
            if world.in_bounds(x, y) && world.materials.get(world.get(x, y).material).rock {
                n += 1;
            }
        }
    }
    n
}
