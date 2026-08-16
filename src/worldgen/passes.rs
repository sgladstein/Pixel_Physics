//! The **realise** half of worldgen: named passes that write cells, each
//! reading only its own column plus a declared margin either side.
//!
//! Split from `column.rs` on the decision/realisation line every shipped
//! generator uses (`Reports/prior-art-worldgen-slicing.md` §6.6: Terraria's
//! ~110 passes, Dwarf Fortress's 18, Minecraft's decision/realisation split
//! with a declared task margin). Nobody ships strict no-neighbour worldgen,
//! so the honest design is not to pretend passes are independent but to say
//! *how far* each one reaches — which is what `Pass::margin` in the parent
//! module records, and what a later per-chunk generator will use to decide
//! how many columns either side of a chunk it must plan before it can fill
//! that chunk.
//!
//! Order matters: later passes overwrite earlier ones, and several of them
//! deliberately only write into cells that are still empty.

use super::column::ColumnPlan;
use super::noise::{self, Purpose};
use super::Ctx;
use crate::sim::material;
use crate::sim::world::World;
use crate::sim::Cell;

/// Sedimentary banding, written into the shade byte.
///
/// The whole visual argument for cut rock. Stone's four shades cost nothing
/// at runtime — the byte is written once here and read by the renderer — but
/// arranged as tilted, gently folded bands they turn every cliff face, mine
/// shaft and (later) cave wall into readable geology instead of flat grey.
/// The legacy terrain used `x % 4`, which produces vertical pinstripes: the
/// one arrangement that reads as a rendering bug rather than as rock.
fn strata_shade(ctx: &Ctx, x: i32, y: i32) -> u8 {
    let p = ctx.terrain.params;
    let thickness = p.strata_thickness.max(1.0);
    // The same displacement the surface benches are snapped to, so a terrace
    // edge always sits on a band boundary that is visible in the rock below
    // it (`column::terraced`).
    let band_y = y as f32 + ctx.terrain.strata_offset(x);
    let band = (band_y / thickness).floor() as i32;
    // Each band draws its own tone, rather than the sequence cycling.
    //
    // A fixed four-tone cycle was the first version, and at any distance it
    // reads as wallpaper: the eye picks up the repeat immediately and the
    // rock stops looking deposited and starts looking tiled. Real
    // stratigraphy is mostly a run of similar beds with the occasional
    // markedly lighter or darker marker bed in it, and no period at all. The
    // weights below say exactly that — about five in six bands sit in the two
    // middle tones, and the rest are markers.
    //
    // Keyed on the band index alone, so a bed holds its tone across the whole
    // world: a layer you can follow from one cliff face to another is the
    // entire point of drawing them.
    //
    // The four tones are spread roughly evenly rather than concentrated in
    // the middle two. Stone's palette spans only 110..138 in brightness, so
    // there is not enough range to spend most of it on near-identical greys —
    // the first version of this weighting put 86% of bands into tones eight
    // apart and the banding simply vanished from the render, which is a
    // reminder that "subtle" and "invisible" are one tuning step apart when
    // the palette is this tight. Irregular run lengths, not low contrast, are
    // what stop it reading as wallpaper.
    let r = noise::unit(ctx.terrain.seed, Purpose::Strata, band, 0);
    let base = if r < 0.30 {
        3
    } else if r < 0.55 {
        1
    } else if r < 0.80 {
        0
    } else {
        2
    };
    // A minority of cells jump a tone, so the bands have grain inside them
    // rather than reading as flat ribbons.
    if noise::unit(ctx.terrain.seed, Purpose::Shade, x, y) < 0.12 {
        (base + 1).min(3)
    } else {
        base
    }
}

/// Shade for a soil cell, darkening toward the surface.
///
/// A soil profile is legible in cross-section and nowhere else, which makes
/// it exactly the kind of detail this view exists to show: dark organic
/// topsoil over paler mineral subsoil, the thing you see in the side of every
/// ditch. Drawn with the same four palette entries the brush uses, so it
/// costs nothing — soil's palette happens to run from `2` (darkest) through
/// `0` and `1` to `3` (lightest), which is the ramp used here.
///
/// `depth` is cells below the top of the blanket.
fn soil_shade(ctx: &Ctx, x: i32, y: i32, depth: i32, total: i32) -> u8 {
    // Jitter first, so the horizons are ragged rather than ruled.
    let jitter = noise::unit(ctx.terrain.seed, Purpose::Shade, x, y);
    let f = if total <= 1 { 0.0 } else { depth as f32 / (total - 1) as f32 };
    let f = (f + (jitter - 0.5) * 0.35).clamp(0.0, 1.0);
    if f < 0.3 {
        2
    } else if f < 0.6 {
        0
    } else if f < 0.85 {
        1
    } else {
        3
    }
}

/// A varied shade for loose material, matching what the brush lays down.
///
/// Not optional decoration: `render.rs`'s per-cell grain mode keys entirely
/// off this byte, so material created with a uniform shade renders visibly
/// flat under it (`examples/filmstrip.rs` documents the same trap for water).
fn loose_shade(ctx: &Ctx, purpose: Purpose, x: i32, y: i32) -> u8 {
    ((noise::unit(ctx.terrain.seed, purpose, x, y) * 4.0) as u8).min(3)
}

/// Everything from the soil line down to bedrock.
///
/// Attached solid, like the legacy terrain and for the same reason: attached
/// cells are structural anchors (`sim/cell.rs`), so the massif holds itself
/// up by construction and can carry overhangs, terraces and (later) cave
/// roofs without any of it being at risk of collapsing the moment the world
/// loads. It is also the at-rest guarantee for all of the rock: a `Solid` has
/// no movement rule at all, so it cannot settle however steep the face is.
pub fn stone_massif(ctx: &Ctx, world: &mut World) -> usize {
    let mut n = 0;
    for x in 0..ctx.terrain.w {
        let c = ctx.plans[x as usize];
        for y in (c.surface_y + c.soil_depth).max(0)..c.bedrock_top_y {
            world.set(x, y, Cell::new(ctx.stone, strata_shade(ctx, x, y)).with_attached(true));
            n += 1;
        }
    }
    n
}

/// The world floor: literal bedrock, the anchor every structural check
/// terminates at. Its top edge undulates because nothing in this world is
/// allowed to be a ruler line except a water surface.
pub fn bedrock_floor(ctx: &Ctx, world: &mut World) -> usize {
    let mut n = 0;
    for x in 0..ctx.terrain.w {
        let c = ctx.plans[x as usize];
        for y in c.bedrock_top_y..ctx.terrain.h {
            world.set(x, y, Cell::new(material::BEDROCK, 0).with_attached(true));
            n += 1;
        }
    }
    n
}

/// Soil over the rock, thinning to nothing on steep ground.
///
/// Real `Powder`, not attached — this is material the player can dig, and
/// that plants will root in. It stays put because of *where* it is placed,
/// never because it is exempted from the rules: `column::plan` already
/// refused to put soil on anything steeper than its angle of repose.
pub fn soil_blanket(ctx: &Ctx, world: &mut World) -> usize {
    let mut n = 0;
    let p = ctx.terrain.params;
    for x in 0..ctx.terrain.w {
        let c = ctx.plans[x as usize];
        if c.soil_depth <= 0 {
            continue;
        }
        let top = c.surface_y.max(0);
        // Never past the bedrock: this pass runs after `bedrock_floor`, so an
        // unclamped blanket on a deep-soil preset would replace the world's
        // anchor with powder and drop the whole massif.
        let bottom = (c.surface_y + c.soil_depth).min(c.bedrock_top_y);
        // Flat low ground collects washed sediment. Judged from the column's
        // own elevation rather than from the world's lowest point, so the
        // pass stays local to its declared margin and survives the move to
        // per-chunk generation unchanged.
        let is_valley_floor =
            ctx.terrain.slope(x) < 0.1 && ctx.terrain.elev(x) < -0.45 * p.relief_amplitude;
        for y in top..bottom {
            // A dithered band where soil meets stone, rather than a clean
            // horizon. Two materials meeting on an exact line is the single
            // most artificial-looking thing a layered generator can do.
            let depth = y - top;
            // A gradational contact rather than a horizon.
            //
            // Soil does not stop and rock start at a line; the bottom of a
            // profile is stones in earth, getting stonier down. Drawn as a
            // clean boundary — which is what a two-row dither at a flat 25%
            // amounted to — the eye reads it as two materials stacked by a
            // program, because that is what it is. The odds ramp from nothing
            // four cells up to near-certain at the base, so the transition
            // has depth to it and no two columns break at the same row.
            let into_contact = (bottom - y).max(0);
            let stony = if into_contact <= CONTACT_DEPTH {
                1.0 - into_contact as f32 / (CONTACT_DEPTH + 1) as f32
            } else {
                0.0
            };
            let (m, shade) = if noise::unit(ctx.terrain.seed, Purpose::Dither, x, y) < stony * 0.85 {
                (ctx.gravel, loose_shade(ctx, Purpose::Dither, x, y))
            } else if is_valley_floor && y < top + 2 {
                (ctx.sand, loose_shade(ctx, Purpose::Shade, x, y))
            } else {
                (ctx.soil, soil_shade(ctx, x, y, depth, bottom - top))
            };
            world.set(x, y, Cell::new(m, shade));
            n += 1;
        }
    }
    n
}

/// How many cells of the soil profile's base grade into stony ground. Not a
/// tunable: it is a property of what a soil profile looks like, not of a
/// particular world's style (`Reports/design-philosophy.md` §2a).
const CONTACT_DEPTH: i32 = 4;

/// A drop of at least this many cells counts as a cliff for the brow and
/// talus passes. Below it the "face" is a slope, and hanging a lip off it
/// would read as a mistake rather than as an overhang.
const CLIFF_DROP: i32 = 6;

/// Cliff edges as `(edge_x, direction, drop)`, where `direction` is +1 when
/// the ground falls away to the right and -1 when it falls to the left.
fn cliff_edges(plans: &[ColumnPlan], w: i32) -> Vec<(i32, i32, i32)> {
    let mut edges = Vec::new();
    for x in 0..w {
        let here = plans[x as usize].surface_y;
        if x + 1 < w {
            let drop = plans[(x + 1) as usize].surface_y - here;
            if drop >= CLIFF_DROP {
                edges.push((x, 1, drop));
            }
        }
        if x > 0 {
            let drop = plans[(x - 1) as usize].surface_y - here;
            if drop >= CLIFF_DROP {
                edges.push((x, -1, drop));
            }
        }
    }
    edges
}

/// Overhanging lips at the top of cliffs.
///
/// The uniqueness deliverable of this pass list. A heightfield alone can only
/// produce a function of x — no overhang, no undercut, nothing to stand under
/// — and a world made only of graph-of-a-function terrain is exactly what
/// every side-view generator in this genre looks like. A brow is attached
/// solid, so it stands until something hits it, at which point the load model
/// already in the engine decides what happens. Neither Terraria nor Noita
/// generates terrain that participates in a structural model, because neither
/// has one.
pub fn brows(ctx: &Ctx, world: &mut World) -> usize {
    let mut n = 0;
    let p = ctx.terrain.params;
    if p.brow_chance <= 0.0 {
        return n;
    }
    for (x, dir, drop) in cliff_edges(&ctx.plans, ctx.terrain.w) {
        if noise::unit(ctx.terrain.seed, Purpose::Pocket, x, dir) >= p.brow_chance {
            continue;
        }
        let top = ctx.plans[x as usize].surface_y;
        // Scaled to the face it hangs off. A fixed three-to-five cells was
        // the first version, and at the handful of genuine cliffs a world
        // contains it produced a lip too small to read as an overhang at all
        // — the pass ran and the picture could not tell. Never longer than
        // the drop is deep, though: a lip that outreaches its own face reads
        // as a shelf floating in the air.
        let reach = (3 + (noise::unit(ctx.terrain.seed, Purpose::Pocket, x, dir * 7) * 3.0) as i32 + drop / 4)
            .min(drop - 1);
        let thick = 2 + (noise::unit(ctx.terrain.seed, Purpose::Pocket, x, dir * 13) * 2.0) as i32;
        for row in 0..thick {
            let y = top + row;
            // Tapered underside, so the lip is a wedge rather than a slab.
            for step in 1..=(reach - row).max(0) {
                let lx = x + dir * step;
                if lx < 0 || lx >= ctx.terrain.w {
                    break;
                }
                // Only into open air: a brow must never overwrite the ground
                // it is hanging over.
                if world.get(lx, y).material != material::EMPTY {
                    break;
                }
                world.set(lx, y, Cell::new(ctx.stone, strata_shade(ctx, lx, y)).with_attached(true));
                n += 1;
            }
        }
    }
    n
}

/// Gravel aprons heaped at the foot of cliffs.
///
/// Bare vertical rock meeting flat ground reads as computer graphics; real
/// faces shed, and the shed material piles against them. It is also the
/// first loose material the player meets — a cliff with a scree slope under
/// it is something to dig into, not just something to look at.
///
/// The wedge recedes one cell of height per two cells out, a slope of 0.5
/// against gravel's 45° repose, so it is comfortably at rest by construction.
pub fn talus(ctx: &Ctx, world: &mut World) -> usize {
    let mut n = 0;
    let p = ctx.terrain.params;
    if p.talus_max_height <= 0.0 {
        return n;
    }
    for (x, dir, drop) in cliff_edges(&ctx.plans, ctx.terrain.w) {
        let peak = (p.talus_max_height as i32).min(drop / 2);
        if peak <= 0 {
            continue;
        }
        // The apron is planned in full and validated before a single cell is
        // written, because the only shape that stays put is one whose toe
        // tapers to nothing.
        //
        // Two earlier versions avalanched, both for the same underlying
        // reason and neither visible in a render. Following each column's own
        // ground gave the wedge the ground's slope on top of its own, coming
        // out steeper than gravel's repose. Cutting the wedge short where the
        // footing got steep replaced that with a five-cell vertical face at
        // the toe, which simply moved the free surface rather than removing
        // it. The honest answer is that below a *continuously* steep face —
        // a canyon wall — there is nowhere for scree to accumulate at all,
        // and the pass should decline rather than approximate.
        let foot = x + dir;
        if foot < 0 || foot >= ctx.terrain.w {
            continue;
        }
        let base_y = ctx.plans[foot as usize].surface_y;
        // The apron's top surface: one cell of fall per two cells out, a
        // slope of 0.5 against gravel's 45° repose.
        let top_at = |step: i32| base_y - (peak - step / 2);
        let mut plan: Vec<(i32, i32, i32)> = Vec::new(); // (tx, top_y, ground)
        let mut prev_ground = base_y;
        let mut tapered = false;
        for step in 0..=(peak * 2) {
            let tx = x + dir * (step + 1);
            if tx < 0 || tx >= ctx.terrain.w {
                break;
            }
            let ground = ctx.plans[tx as usize].surface_y;
            // A further drop inside the footprint would leave the apron
            // hanging over it.
            if (ground - prev_ground).abs() > 1 {
                break;
            }
            prev_ground = ground;
            let top_y = top_at(step);
            if top_y >= ground {
                // The apron's surface has met the ground: it has thinned to
                // nothing of its own accord, which is the only ending that
                // leaves no free face.
                tapered = true;
                break;
            }
            plan.push((tx, top_y, ground));
        }
        // No taper means the ground fell away as fast as the apron did and
        // the two never met. Scree there would end in a cliff of its own.
        if !tapered {
            continue;
        }
        for (tx, top_y, ground) in plan {
            for y in top_y.max(0)..ground {
                // Open air only. The apron heaps *against* the face and on
                // top of the ground; it never eats into either, so every
                // cell it writes is resting on something solid.
                if world.get(tx, y).material != material::EMPTY {
                    continue;
                }
                world.set(tx, y, Cell::new(ctx.gravel, loose_shade(ctx, Purpose::Pocket, tx, y)));
                n += 1;
            }
        }
    }
    n
}

/// Standing water in every hollow the table reaches.
///
/// The only pass so far that has to see the whole world, and it is worth
/// being precise about why: whether water stands at a column depends on the
/// height of the *lowest rim* enclosing it, which can be any distance away.
/// That is the classic trapped-water scan, and it is exactly the reasoning
/// the coarse `(x, z)` map exists to take over (design doc §5) — until then
/// this pass is honest debt rather than a solved problem, which is what its
/// `GLOBAL` margin records.
///
/// Water is *born at its own level*, full, and flat. That is not an
/// optimisation, it is the whole difference between a world that opens
/// settled and one the player watches slosh for a minute: a flat, full pool
/// is already at equilibrium, so no levelling transfer fires and the chunks
/// sleep on the first sweep.
pub fn ponds(ctx: &Ctx, world: &mut World) -> usize {
    let mut n = 0;
    let w = ctx.terrain.w as usize;
    if w == 0 {
        return n;
    }
    // Rim heights sweeping in from each edge. Remember y-down: a *smaller*
    // y is a taller barrier, so the running extreme is a minimum.
    let mut left_rim = vec![i32::MAX; w];
    let mut right_rim = vec![i32::MAX; w];
    let mut running = i32::MAX;
    for (x, rim) in left_rim.iter_mut().enumerate() {
        running = running.min(ctx.plans[x].surface_y);
        *rim = running;
    }
    running = i32::MAX;
    for x in (0..w).rev() {
        running = running.min(ctx.plans[x].surface_y);
        right_rim[x] = running;
    }

    // Plan first, write second: the minimum-size rule below has to be able to
    // reject a pool *before* any of it exists, and a film deleted after the
    // fact would leave the chunks it touched awake for nothing.
    //
    // **Basins are grouped by topography, not by where water ends up.** That
    // distinction is the whole correctness of this pass and it cost a
    // debugging session to find. Grouping contiguous *wet* columns splits a
    // basin in two wherever a submerged ridge pokes up between two deeper
    // parts: each half then gets its own level, and since a hollow's two
    // halves generally have different water tables, the two levels differ —
    // so the first sweep flowed one into the other and 686 cells of water
    // redistributed themselves. On screen that is a world that opens by
    // visibly draining its own lake.
    //
    // A column is in a basin when its spill level is genuinely below its
    // ground, and a submerged ridge satisfies that too, so grouping this way
    // keeps the whole hollow together.
    let spill: Vec<i32> = (0..w).map(|x| left_rim[x].max(right_rim[x])).collect();

    let min_depth = ctx.terrain.params.pond_min_depth.max(0.0) as i32;
    let min_width = ctx.terrain.params.pond_min_width.max(0.0) as i32;
    let mut level = vec![i32::MAX; w];
    let mut x = 0usize;
    while x < w {
        if spill[x] >= ctx.plans[x].surface_y {
            x += 1;
            continue;
        }
        let start = x;
        // One level for the whole basin. A free surface is level; a sloped
        // one is a head difference, and head differences flow.
        //
        // The level is the lower in elevation — the larger y — of the two
        // constraints: the most restrictive rim anywhere on the basin, and
        // the highest the table reaches inside it. Groundwater fills a hollow
        // to the top of its table and no further than the rim.
        let mut rim = i32::MIN;
        let mut table_top = i32::MAX;
        while x < w && spill[x] < ctx.plans[x].surface_y {
            rim = rim.max(spill[x]);
            table_top = table_top.min(ctx.plans[x].table_y);
            x += 1;
        }
        let pool = rim.max(table_top);

        // Reject pools too small to read as water. A one-cell film renders as
        // a black line rather than as a pool, because `render.rs` dims a
        // liquid toward black by fill -- so the thin ones do not look like
        // shallow water, they look like a bug.
        let deepest = (start..x).map(|i| ctx.plans[i].surface_y - pool).max().unwrap_or(0);
        let wet_columns = (start..x).filter(|&i| pool < ctx.plans[i].surface_y).count() as i32;
        if wet_columns < min_width || deepest < min_depth {
            continue;
        }
        for (i, slot) in level.iter_mut().enumerate().take(x).skip(start) {
            if pool < ctx.plans[i].surface_y {
                *slot = pool;
            }
        }
    }

    for (x, &pool) in level.iter().enumerate() {
        if pool == i32::MAX {
            continue;
        }
        let plan = ctx.plans[x];
        for y in pool.max(0)..plan.surface_y {
            if world.get(x as i32, y).material != material::EMPTY {
                continue;
            }
            // `aux` is left alone deliberately: on a `Liquid` cell `aux == 0`
            // means **full**, and writing a literal fill here would be the
            // documented way to manufacture a full cell out of nothing. The
            // shade is varied because uniform-shade water renders visibly
            // flat under the per-cell grain mode.
            world.set(x as i32, y, Cell::new(ctx.water, loose_shade(ctx, Purpose::Shade, x as i32, y)));
            n += 1;
        }
    }
    n
}

/// The saturated zone: a moisture floor under the water table, ramping up
/// through the capillary fringe above it.
///
/// **No liquid cells are placed inside the ground, ever.** A cell holds one
/// material and there is no porosity, so saturated rock is rock whose field
/// says it is wet — which is also why the underground cannot "fill with
/// water" however high the table is set. The fear that a water table floods
/// the world is answered by this pass's existence: the aquifer is a number
/// per field block, and the only liquid anywhere is standing in open hollows.
pub fn moisture_init(ctx: &Ctx, world: &mut World) -> usize {
    let mut n = 0;
    let fringe = ctx.terrain.params.capillary_fringe.max(0.0);
    // One write per field block rather than per cell: the field grid is
    // 1/FIELD_SCALE resolution and writing every cell would do the same work
    // sixty-four times.
    let step = crate::sim::field::FIELD_SCALE;
    let mut y = 0;
    while y < ctx.terrain.h {
        let mut x = 0;
        while x < ctx.terrain.w {
            let plan = ctx.plans[x as usize];
            let floor = if y >= plan.table_y {
                1.0
            } else if fringe > 0.0 && (plan.table_y - y) as f32 <= fringe {
                // Linear ramp up to saturation: the capillary fringe, where
                // ground is damp above the table because water wicks up into
                // it. This is the band roots should be able to find without
                // growing into standing water and drowning.
                let above = (plan.table_y - y) as f32;
                0.15 + (1.0 - 0.15) * (1.0 - above / fringe)
            } else if plan.table_y < ctx.terrain.h
                && y >= plan.surface_y
                && plan.soil_depth > 0
                && y < plan.surface_y + plan.soil_depth
            {
                // Soil holds a little damp of its own, but only in a world
                // that has groundwater to hold. Gated on the table being
                // inside the world so that a preset which switches water off
                // switches *all* of it off — `arid` reporting a few hundred
                // damp cells would make the pivot lever a half-measure, and
                // the whole point of that lever is that it is total.
                0.15
            } else {
                0.0
            };
            if floor > 0.0 {
                world.set_field_moisture_floor(x, y, floor);
                n += 1;
            }
            x += step;
        }
        y += step;
    }
    n
}

/// Sand and gravel lenses sealed inside the rock.
///
/// Loose material the player only finds by digging, and which behaves the
/// moment it is exposed — cut into one and it pours. Fully enclosed, so it is
/// trivially at rest until something opens it, which is why this is the one
/// place generated powder can sit at any shape at all.
pub fn pockets(ctx: &Ctx, world: &mut World) -> usize {
    let mut n = 0;
    let p = ctx.terrain.params;
    if p.pocket_density <= 0.0 {
        return n;
    }
    const REGION: i32 = 64;
    let seed = ctx.terrain.seed;
    for ry in 0..ctx.terrain.h.div_euclid(REGION) + 1 {
        for rx in 0..ctx.terrain.w.div_euclid(REGION) + 1 {
            // A fractional density means "sometimes one": the whole number is
            // guaranteed and the remainder is a per-region coin flip.
            let whole = p.pocket_density.floor() as i32;
            let extra = i32::from(noise::unit(seed, Purpose::Pocket, rx, ry) < p.pocket_density.fract());
            for k in 0..whole + extra {
                let cx = rx * REGION + (noise::unit(seed, Purpose::Pocket, rx * 31 + k, ry) * REGION as f32) as i32;
                let cy = ry * REGION + (noise::unit(seed, Purpose::Pocket, rx, ry * 31 + k) * REGION as f32) as i32;
                if cx < 0 || cx >= ctx.terrain.w {
                    continue;
                }
                let a = 4.0 + noise::unit(seed, Purpose::Pocket, cx, cy) * 6.0;
                let b = 2.0 + noise::unit(seed, Purpose::Pocket, cy, cx) * 2.0;
                let m = if noise::unit(seed, Purpose::Pocket, cx + 1, cy + 1) < 0.5 { ctx.sand } else { ctx.gravel };

                // Collect first, write only if the whole lens plus a one-cell
                // rind is solid stone.
                //
                // Skipping non-stone cells individually was the first version
                // and it is not the same thing: a lens clipping the surface
                // simply lost the cells that stuck out and kept the rest,
                // leaving loose powder outcropping on a slope, which promptly
                // ran. "Sealed" is the property the whole pass relies on for
                // its cells to be at rest, so it has to be checked before any
                // of them is written, not approximated per cell.
                let mut lens = Vec::new();
                let mut sealed = true;
                'lens: for dy in -(b as i32) - 1..=(b as i32) + 1 {
                    for dx in -(a as i32) - 1..=(a as i32) + 1 {
                        let (px, py) = (cx + dx, cy + dy);
                        let d = (dx as f32 / a).powi(2) + (dy as f32 / b).powi(2);
                        // The rind: one cell beyond the lens must also be
                        // stone, so the lens is never flush with a free face.
                        let rind = (dx.abs() as f32 / (a + 1.0)).powi(2) + (dy.abs() as f32 / (b + 1.0)).powi(2);
                        if rind > 1.0 {
                            continue;
                        }
                        if px < 0 || px >= ctx.terrain.w || py < 0 || py >= ctx.terrain.h {
                            sealed = false;
                            break 'lens;
                        }
                        if world.get(px, py).material != ctx.stone {
                            sealed = false;
                            break 'lens;
                        }
                        if d <= 1.0 {
                            lens.push((px, py));
                        }
                    }
                }
                if !sealed {
                    continue;
                }
                for (px, py) in lens {
                    world.set(px, py, Cell::new(m, loose_shade(ctx, Purpose::Pocket, px, py)));
                    n += 1;
                }
            }
        }
    }
    n
}
