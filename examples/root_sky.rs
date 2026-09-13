//! **Do roots climb out of the ground?** — §W6's remaining question, asked
//! of the scene the owner actually reported it in.
//!
//! The owner, by eye on a `grove` sheet: *"there is an issue where the roots
//! are growing into the tree/sky."* `open-bugs-handoff.md` §W6 traced the
//! chain and fixed the last link — a `RootTip` may now take an `EMPTY` cell
//! only if ground touches it — but the *sighting* was never confirmed gone,
//! and a unit scene cannot confirm it: a root cannot enter `Liquid`, so the
//! climb path has to be **air** while the attractor sits above it, and
//! standing water falls into that path unless something props it up, which
//! then blocks the climb. Only real weather over open terrain has both.
//!
//! ```text
//! cargo run --release --example root_sky -- seeds=6 frames=24000
//! PIXEL_PHYSICS_ROOT_SUBSTRATE=off cargo run --release --example root_sky -- seeds=6 frames=24000
//! ```
//!
//! # What it counts, and why not the obvious thing
//!
//! **Root cells with open sky above them.** Walk up from the cell; if
//! nothing that is not living tissue is met before the top of the world, the
//! root is out of the ground.
//!
//! The obvious metric — *a root cell with no soil touching it* — is wrong,
//! and wrong in a way that looks right. It was tried first and reported **23
//! of 55 root cells "standing in open air"** in a plain walled bed with
//! nothing wrong with it: a root ball dense enough that every neighbour of an
//! interior cell is its own tissue, and the soil it grew through was
//! displaced on the way in. That number reached a code comment before the
//! control caught it. Sky-above cannot make that mistake, because a buried
//! root ball has ground over it.
//!
//! # Paired on the ablation, not across a rebuild
//!
//! `PIXEL_PHYSICS_ROOT_SUBSTRATE=off` restores the pre-fix rule, so both
//! arms come from **one binary** on the same seeds. This line has twice been
//! caught comparing across a rebuild — once when a merge moved the baseline
//! 4,449 -> 2,751 cells with the mechanism still off — and a switch is the
//! only thing that removes the confound rather than measuring around it.

mod common;

use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::organism::{self, CellType};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}="))?.parse().ok())
}

/// Is there open sky above `(x, y)`? Living tissue does not roof anything —
/// a root under its own trunk is still out of the ground.
fn under_open_sky(world: &World, x: i32, y: i32) -> bool {
    let top = world.bounds().map_or(0, |b| b.min_y);
    let mut cy = y - 1;
    while cy >= top {
        let cell = world.get(x, cy);
        if cell.organism_id() == 0
            && matches!(world.materials.kind(cell.material), MaterialKind::Solid | MaterialKind::Powder)
        {
            return false;
        }
        cy -= 1;
    }
    true
}

/// The terrain surface in column `x`: the topmost cell that is ground
/// rather than sky or living tissue. `None` if the column is all air.
///
/// **This is the census the organism walk could not do.** Walking
/// `state.cells` asks "does a *live plant* hold root tissue up here", and
/// the pale cream in the owner's sheet is partly tissue no live plant
/// claims any more -- shed, orphaned, still standing, still rendered. A
/// grid pass keyed on the *material* sees it; a membership pass cannot.
fn surface_y(world: &World, x: i32) -> Option<i32> {
    let b = world.bounds()?;
    for y in b.min_y..=b.max_y {
        let cell = world.get(x, y);
        if cell.organism_id() == 0
            && matches!(world.materials.kind(cell.material), MaterialKind::Solid | MaterialKind::Powder)
        {
            return Some(y);
        }
    }
    None
}

fn main() {
    let seeds: u64 = arg("seeds").unwrap_or(6);
    let seed0: u64 = arg("seed0").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(24_000);
    let plants: usize = arg("plants").unwrap_or(common::PlantScene::default().trees);

    // Echo the parameters, and the arm. A knob nobody can see the value of
    // is a knob nobody can tell is disconnected -- and this one is an
    // environment variable, which is the easiest kind to run without.
    let rule = !matches!(std::env::var("PIXEL_PHYSICS_ROOT_SUBSTRATE").as_deref(), Ok("off"));
    let card = arg::<u64>("defaultseed").unwrap_or(0) == 1;
    println!(
        "root_sky: {} frames={frames} plants={plants} arm={}",
        if card {
            "seed=CARD (PlantScene default -- the world review card 168b0f was rendered from)".to_string()
        } else {
            format!("seeds={seed0}..{}", seed0 + seeds - 1)
        },
        if rule { "roots need substrate (shipped)" } else { "OFF -- pre-fix behaviour" }
    );

    // **The owner's own sheet, reproducible.** Card 168b0f was
    // `filmstrip scene=grove start=24000` with no `seed=`, and `PlantScene`
    // documents `None` as "leaves `World::new`'s own seed alone, so every
    // stored sheet keeps meaning exactly what it meant". A numbered sweep is
    // therefore a *different world* from the one the complaint came from --
    // which matters here, because the chain W6 describes needs rain, and
    // rain is a pure function of `(seed, frame)`. `defaultseed=1` runs the
    // card's world and nothing else.
    let arms: Vec<Option<u64>> =
        if card { vec![None] } else { (0..seeds).map(|s| Some(seed0 + s)).collect() };

    let (mut tot_roots, mut tot_sky, mut worst) = (0usize, 0usize, 0usize);
    let (mut tot_above, mut worst_rise_all) = (0usize, 0i32);
    let (mut tot_grid, mut tot_grid_above, mut tot_grid_orphan, mut worst_grid_rise) =
        (0usize, 0usize, 0usize, 0i32);
    let mut tot_by_type = [0usize; 7];
    let mut tot_sod = 0usize;
    let mut tot_grid_sky = 0usize;
    for arm in arms {
        let label = arm.map_or("card".to_string(), |s| s.to_string());
        let mut world = common::PlantScene { trees: plants, seed: arm, ..Default::default() }.build();
        // The shipped driver, not a hand-rolled loop: weather, the field and
        // the plants all have to run or the rain this question is about
        // never falls.
        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::new();
        let tuning = player::Tuning::default();
        for _ in 0..frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }

        let (mut roots, mut sky) = (0usize, 0usize);
        let mut highest: Option<(i32, i32)> = None;
        // **The other reading of the complaint.** "Growing into the
        // tree/sky" can mean root tissue *in the air*, which `sky` counts,
        // or root tissue *inside the trunk* -- root-type cells well above
        // the plant's own collar, where shoot tissue belongs. They are
        // different defects and they look alike in a sentence, which is
        // exactly the ambiguity `CLAUDE.md` says to resolve by measuring
        // both rather than by picking one and building on it.
        let (mut above_collar, mut worst_rise) = (0usize, 0i32);
        // Positions, not just a count: a number says whether it happened and
        // only a coordinate says where to point a camera. `CLAUDE.md` --
        // an image tells you what and where, a metric how much.
        let mut intruders: Vec<(i32, i32, i32)> = Vec::new();
        for id in world.live_organism_ids() {
            let Some(state) = world.organism(id) else { continue };
            let collar = state.collar_y;
            let cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
            for (cx, cy) in cells {
                let cell = world.get(cx, cy);
                if cell.organism_id() != id {
                    continue;
                }
                let is_root = matches!(organism::cell_type(cell.aux()), Some(CellType::RootTip))
                    || world.materials.get(cell.material).reinforces_powder;
                if !is_root {
                    continue;
                }
                roots += 1;
                if let Some(collar) = collar {
                    // Above the collar by more than a couple of cells: a
                    // root mat straddles the collar row normally, so a small
                    // rise is the anatomy, not the defect.
                    let rise = collar - cy;
                    if rise > 2 {
                        above_collar += 1;
                        worst_rise = worst_rise.max(rise);
                        intruders.push((rise, cx, cy));
                    }
                }
                if under_open_sky(&world, cx, cy) {
                    sky += 1;
                    if highest.is_none_or(|(_, hy)| cy < hy) {
                        highest = Some((cx, cy));
                    }
                }
            }
        }
        // **The grid census, and the reason there is one.** Everything above
        // walks `state.cells` -- it asks whether a *live plant* is holding
        // root tissue up in the air. Measured on the owner's own sheet
        // (card 168b0f, `scene=grove start=24000`), 16% of the root-coloured
        // pixels sit ABOVE the soil line, as high as 80 cells up, while this
        // probe's organism walk reported 14 cells across six seeds. Both
        // numbers are arithmetically right; they count different things,
        // and the membership walk cannot see tissue no live organism claims.
        // So census the grid by MATERIAL, and say which half it is.
        //
        // Root tissue is `reinforces_powder` on a `Plant` -- `grassroot`
        // carries the same flag and is a `Powder`, i.e. sod, not a root.
        let (mut grid_roots, mut grid_above, mut grid_orphan, mut grid_worst) =
            (0usize, 0usize, 0usize, 0i32);
        // [RootTip, MatureBody, GrowingTip, Leaf, DormantBud, other]
        // [RootTip, MatureBody, GrowingTip, Leaf, DormantBud, other-owned, UNOWNED]
        let mut grid_by_type = [0usize; 7];
        let mut grid_sod = 0usize;
        let (mut shoot_above, mut shoot_above_rootlike) = (0usize, 0usize);
        let mut grid_sky = 0usize;
        let mut grid_hits: Vec<(i32, i32, i32, bool)> = Vec::new();
        // Resolved once per world, not per cell: `id_of` is a string hash.
        let sod = world.materials.id_of("grassroot");
        if let Some(b) = world.bounds() {
            for x in b.min_x..=b.max_x {
                let surf = surface_y(&world, x);
                for y in b.min_y..=b.max_y {
                    let cell = world.get(x, y);
                    // **All living tissue above the soil line, whatever it is
                    // made of** -- and this census exists because the owner
                    // asked the question the root-material count cannot
                    // answer.
                    //
                    // The tissue-role fix does not refuse the conversion a
                    // plant evolved, and does not stop the shoot growing: it
                    // only changes what that shoot is *made of*. So
                    // "root-material cells above the soil line: 761 -> 13" is
                    // consistent with two very different worlds -- the shoot
                    // still standing there in wood, or the shoot never having
                    // grown at all. Both give the same number, and the
                    // below-ground control does not separate them because it
                    // looks the wrong way.
                    //
                    // This counts every organism-owned cell above the surface,
                    // by material class rather than by role. If the fix works
                    // the way it is claimed to, WOOD above ground rises by
                    // about what root material lost, and the total barely
                    // moves. `CLAUDE.md`: *look again after the fix, for what
                    // you did not measure.*
                    if cell.organism_id() != 0 {
                        if let Some(surf) = surf {
                            if surf - y > 2 {
                                shoot_above += 1;
                                if world.materials.get(cell.material).reinforces_powder {
                                    shoot_above_rootlike += 1;
                                }
                            }
                        }
                    }
                    if !world.materials.get(cell.material).reinforces_powder
                        || !matches!(world.materials.kind(cell.material), MaterialKind::Plant)
                    {
                        continue;
                    }
                    // **Which root material, and is this even a plant's
                    // cell?** Two traps, both nearly published as a result.
                    //
                    // `grassroot` is also `kind: Plant` and also
                    // `reinforces_powder` -- it is what soil *becomes* when
                    // grass roots it, so those cells carry
                    // `organism_id == 0`. On a cell no organism owns, `aux`
                    // is **moisture**, not a packed cell type, so decoding it
                    // manufactures cell types out of soil wetness. Split the
                    // materials, and decode `aux` only where an organism
                    // actually owns the cell.
                    let owned = cell.organism_id() != 0;
                    if Some(cell.material) == sod {
                        grid_sod += 1;
                        continue;
                    }
                    grid_roots += 1;
                    let Some(surf) = surf else { continue };
                    let rise = surf - y;
                    if rise > 2 {
                        grid_above += 1;
                        grid_worst = grid_worst.max(rise);
                        let live = cell.organism_id() != 0
                            && world.organism(cell.organism_id()).is_some();
                        if !live {
                            grid_orphan += 1;
                        }
                        // **The discriminator.** Root *material* above the
                        // soil line has two very different causes and they
                        // are indistinguishable in a count:
                        //
                        // - a `RootTip`/`MatureBody` still doing root work,
                        //   left standing because the SOIL went (erosion,
                        //   a collapse, a dig) -- the root did not move;
                        // - a `GrowingTip`/`Leaf`/`DormantBud`, i.e. SHOOT
                        //   tissue wearing root material, which is the
                        //   propagate-from-parent rule carrying rootwood up
                        //   out of the ground. `tissue_appearance` only
                        //   overrides material for organs (`Flower`,
                        //   `Fruit`); every other type inherits, and its own
                        //   doc says inheriting "is precisely how a flower
                        //   ends up brown".
                        //
                        // The second is the owner's pale-cream plant. The
                        // first is a different bug (see W4). Counting them
                        // together is `CLAUDE.md`'s *ask what your number
                        // counts*, so do not.
                        match if owned { organism::cell_type(cell.aux()) } else { None } {
                            Some(CellType::RootTip) => grid_by_type[0] += 1,
                            Some(CellType::MatureBody) => grid_by_type[1] += 1,
                            Some(CellType::GrowingTip) => grid_by_type[2] += 1,
                            Some(CellType::Leaf) => grid_by_type[3] += 1,
                            Some(CellType::DormantBud) => grid_by_type[4] += 1,
                            None if !owned => grid_by_type[6] += 1,
                            _ => grid_by_type[5] += 1,
                        }
                        // Open sky, or a void under a roof: a root standing
                        // in a dug-out pocket is not "in the sky", and the
                        // fix for it is not the same fix.
                        if under_open_sky(&world, x, y) {
                            grid_sky += 1;
                        }
                        grid_hits.push((rise, x, y, live));
                    }
                }
            }
        }
        println!(
            "           ALL living tissue above the soil line: {shoot_above} cells, of which {shoot_above_rootlike} are root material \
({} are not) -- the count that says whether the shoot is still THERE, not merely no longer pale",
            shoot_above - shoot_above_rootlike
        );
        let gpct = if grid_roots > 0 { 100.0 * grid_above as f64 / grid_roots as f64 } else { 0.0 };
        println!(
            "           GRID: {grid_roots:>6} root-tissue cells, {grid_above:>5} above the soil line ({gpct:>5.1}%), \
{grid_orphan} owned by no live plant, {grid_sky} under open sky, worst rise {grid_worst}"
        );
        println!(
            "           of those, by type: RootTip {} | MatureBody {} (AMBIGUOUS -- shared by root and shoot) | \
unambiguously SHOOT tissue in root material: tip {} + leaf {} + bud {} = {} | other-owned {} | \
UNOWNED (no organism -- aux is not a cell type here) {}   [grassroot sod skipped: {}]",
            grid_by_type[0],
            grid_by_type[1],
            grid_by_type[2],
            grid_by_type[3],
            grid_by_type[4],
            grid_by_type[2] + grid_by_type[3] + grid_by_type[4],
            grid_by_type[5],
            grid_by_type[6],
            grid_sod,
        );
        if !grid_hits.is_empty() {
            grid_hits.sort_unstable_by_key(|a| std::cmp::Reverse(a.0));
            println!(
                "           highest (rise, x, y, live): {:?}",
                &grid_hits[..grid_hits.len().min(6)]
            );
        }
        for (t, n) in tot_by_type.iter_mut().zip(grid_by_type) {
            *t += n;
        }
        tot_grid_sky += grid_sky;
        tot_sod += grid_sod;
        tot_grid += grid_roots;
        tot_grid_above += grid_above;
        tot_grid_orphan += grid_orphan;
        worst_grid_rise = worst_grid_rise.max(grid_worst);
        let pct = if roots > 0 { 100.0 * sky as f64 / roots as f64 } else { 0.0 };
        println!(
            "  seed {label:>4}: {roots:>6} root cells, {sky:>5} under open sky ({pct:>5.1}%)  highest {highest:?}  \
| {above_collar:>4} inside the shoot, worst {worst_rise:>3} cells above the collar"
        );
        if !intruders.is_empty() {
            intruders.sort_unstable_by_key(|a| std::cmp::Reverse(a.0));
            println!("           root tissue inside the shoot at (rise, x, y): {:?}", &intruders[..intruders.len().min(6)]);
        }
        tot_above += above_collar;
        worst_rise_all = worst_rise_all.max(worst_rise);
        tot_roots += roots;
        tot_sky += sky;
        worst = worst.max(sky);
    }
    let pct = if tot_roots > 0 { 100.0 * tot_sky as f64 / tot_roots as f64 } else { 0.0 };
    println!(
        "\n  TOTAL {tot_roots} root cells, {tot_sky} under open sky ({pct:.2}%), worst single seed {worst}\n\
  TOTAL {tot_above} root cells inside the shoot (>2 above the collar), worst rise {worst_rise_all} cells"
    );
    let gpct = if tot_grid > 0 { 100.0 * tot_grid_above as f64 / tot_grid as f64 } else { 0.0 };
    println!(
        "  GRID  {tot_grid} root-tissue cells, {tot_grid_above} above the soil line ({gpct:.2}%), \
{tot_grid_orphan} owned by no live plant, {tot_grid_sky} under open sky, worst rise {worst_grid_rise} cells"
    );
    println!(
        "  GRID  by type: RootTip {} | MatureBody {} (AMBIGUOUS -- shared by root and shoot, so this is NOT \
a count of root work) | unambiguously SHOOT tissue in root material {} (tip {} + leaf {} + bud {}) | \
other-owned {} | UNOWNED {}   [grassroot sod skipped: {}]",
        tot_by_type[0],
        tot_by_type[1],
        tot_by_type[2] + tot_by_type[3] + tot_by_type[4],
        tot_by_type[2],
        tot_by_type[3],
        tot_by_type[4],
        tot_by_type[5],
        tot_by_type[6],
        tot_sod,
    );
}
