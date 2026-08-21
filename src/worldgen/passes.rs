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

/// Tones per palette family. `Cell::shade` is `family * TONES + tone`.
///
/// Four because that is what every shipped ramp already had: `strata_shade`
/// draws four bed tones, `soil_shade` walks a four-step profile, and
/// `loose_shade` picks one of four. Families are appended to the material
/// palettes in blocks of this size, so family 0 is byte-for-byte the palette
/// each file shipped with (`assets/materials/stone.ron` carries the note).
const TONES: u8 = 4;

/// The reference family: no regional shift. What the brush lays down, and
/// what every material with a single family has.
const FAMILY_NEUTRAL: u8 = 0;
/// Wet country: darker, richer, blue-shifted.
const FAMILY_WET: u8 = 1;
/// Dry country: warmer and paler.
const FAMILY_DRY: u8 = 2;
/// Resistant beds: pale, low-saturation cap-rock. **Stone only** — soil and
/// sand ship three families, and resistance is a property of rock.
const FAMILY_RESISTANT: u8 = 3;

/// Wavelength of the slow field that displaces the family probabilities, in
/// cells, applied on both axes.
///
/// Chosen at the scale of the artifact it breaks up rather than of the
/// terrain: the piers were tens of columns wide and ran the full height of a
/// cliff, so a field that turns over every ~48 cells puts several reversals
/// down any face tall enough to have read as a pier. Much shorter and the
/// displacement stops being a facies boundary and starts being a second
/// stipple on top of the first; much longer and a whole cliff sits under one
/// sign of the bias, which is the pier again with a different threshold.
const FAMILY_FIELD_WAVELENGTH: f32 = 48.0;

/// Gravel's second family: the buried read, for lenses sealed in the rock.
///
/// Not a region family — it is a *context* family, and that distinction is
/// what let one palette serve two masters instead of trading them off.
/// Gravel is read against two completely different backgrounds: scree
/// against sky and rock face, and a lens against solid stone. Its shipped
/// greys are within a few points of stone's, which is right for scree and
/// makes a buried lens invisible. Recolouring the whole material to fix the
/// lens would have made every talus apron read as something other than
/// broken rock.
const BURIED_FAMILY: u8 = 1;

/// Which palette family a cell at `(x, y)` takes, from the blended region
/// `Character` at `x`.
///
/// This is the whole of "regions become biomes, step 1", and it costs
/// nothing per frame: the family is folded into `Cell::shade` once at
/// genesis and `render.rs` reads that byte, it never recomputes a colour.
/// Keying a tint to a live field read instead is the precedent that forces
/// full redraws forever (`Reports/world-review-2026-08.md` §7.21).
///
/// **A cumulative selection over one noise draw, not three independent
/// coin flips.** One draw means the three probabilities partition cleanly —
/// a cell cannot come out both dry and resistant — and it means the
/// transition between two regions is a *dither* rather than a ruled line.
/// That matters more than it sounds: aridity varies smoothly across a
/// region boundary, so a hard threshold would put a perfectly straight
/// vertical colour seam through solid rock, which is the single most
/// artificial thing a layered generator can draw (the soil/stone contact in
/// `soil_blanket` is dithered for exactly this reason and says so).
///
/// **The per-cell dither is not enough on its own, and round 1 shipped
/// without the other half.** The draw `u` varies per cell, but the
/// *thresholds* it is compared against came from `character(x)` alone — so
/// the probability of a family was constant down an entire column, and a
/// transition zone came out as a full-height vertical pier of stipple whose
/// density never changed with depth. On canyon's jagged terrain the merge
/// review read exactly that: columns of grey standing inside warm country at
/// x ~ 290-320, 620-660 and 880-990 on seed 7. A stipple can only hide a
/// boundary it is allowed to move; this one was pinned to a column.
///
/// So the probability is displaced by a slow 2-D field
/// ([`FAMILY_FIELD_WAVELENGTH`]) before the draw is compared against it. The
/// family boundary then wanders up and down through the rock over tens of
/// rows, which is what a facies change actually does, and no column is a
/// pier because no column has a constant threshold any more.
///
/// The character is read at `x` only. `y` enters through the dither draw and
/// through that field, so a family boundary is ragged in both directions
/// rather than being a column of one colour beside a column of another.
fn palette_family(ctx: &Ctx, x: i32, y: i32, cap_rock: bool) -> u8 {
    // A preset that asked for no regional variation gets none, including no
    // palette shift. `flat` is the structural test bed and its whole point
    // is that nothing about it wanders; it is also the control render the
    // destruction workstream compares against, so leaving it byte-identical
    // is worth more than colouring it.
    if ctx.terrain.params.region_variation <= 0.0 {
        return FAMILY_NEUTRAL;
    }
    let ch = ctx.terrain.character(x);
    let u = noise::unit(ctx.terrain.seed, Purpose::Palette, x, y);
    // The slow displacement, centred on zero so it pushes a threshold both
    // ways: a field that only ever *added* probability would widen every
    // family into its neighbour rather than making the boundary meander.
    let bias = ctx.terrain.params.palette_field
        * (noise::fbm_2d(
            ctx.terrain.seed,
            Purpose::PaletteField,
            x as f32 / FAMILY_FIELD_WAVELENGTH,
            y as f32 / FAMILY_FIELD_WAVELENGTH,
            2,
        ) * 2.0
            - 1.0);

    // Resistance first, and only for rock. Thresholds sit above the neutral
    // 1.0 that `Character::neutral` hands out, so an unremarkable region
    // stays grey and only a genuinely resistant draw bleaches.
    let mut floor = 0.0;
    if cap_rock {
        let resistant = (noise::smoothstep(1.25, 1.80, ch.resistance) + bias).clamp(0.0, 1.0);
        if u < resistant {
            return FAMILY_RESISTANT;
        }
        floor = resistant;
    }
    // Dry and wet are mutually exclusive by construction -- the two ramps do
    // not overlap -- so their probabilities can share the remaining room
    // without either stealing from the other.
    //
    // The aridity ramps are wider than round 1 shipped them (0.50..0.78 and
    // 0.10..0.34). A narrow ramp means a region is nearly all-or-nothing dry,
    // so the dither band -- the only place the two families interleave at all
    // -- is a few columns wide and the rest is solid blocks of one family.
    // Widening the ramp is what gives the 2-D field room to work: a broad
    // band of genuinely mixed probability is what a slow displacement can
    // make wander.
    let dry = (noise::smoothstep(0.42, 0.86, ch.aridity) + bias).clamp(0.0, 1.0);
    if u < floor + dry * (1.0 - floor) {
        return FAMILY_DRY;
    }
    let wet = (1.0 - noise::smoothstep(0.06, 0.42, ch.aridity) + bias).clamp(0.0, 1.0);
    if u < floor + (dry + wet) * (1.0 - floor) {
        return FAMILY_WET;
    }
    FAMILY_NEUTRAL
}

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
    let tone = if noise::unit(ctx.terrain.seed, Purpose::Shade, x, y) < 0.12 {
        (base + 1).min(TONES - 1)
    } else {
        base
    };
    // The region's family, applied *after* the tone. The band index still
    // decides which of the four tones a bed takes, so a bed remains
    // followable from one cliff face to the next -- what the region changes
    // is which four colours those tones name. Family, not texture.
    palette_family(ctx, x, y, true) * TONES + tone
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
    let tone = if f < 0.3 {
        2
    } else if f < 0.6 {
        0
    } else if f < 0.85 {
        1
    } else {
        3
    };
    // Aridity shifts the whole profile -- dark rich loam in a basin, pale
    // dusty soil on a dry hillside -- while the 2/0/1/3 walk down the ramp
    // stays put, so a cut bank still reads as topsoil over subsoil wherever
    // it is. No cap-rock family: resistance is a property of the rock.
    palette_family(ctx, x, y, false) * TONES + tone
}

/// A varied shade for loose material, matching what the brush lays down.
///
/// Not optional decoration: `render.rs`'s per-cell grain mode keys entirely
/// off this byte, so material created with a uniform shade renders visibly
/// flat under it (`examples/filmstrip.rs` documents the same trap for water).
fn loose_shade(ctx: &Ctx, purpose: Purpose, x: i32, y: i32) -> u8 {
    ((noise::unit(ctx.terrain.seed, purpose, x, y) * TONES as f32) as u8).min(TONES - 1)
}

/// [`loose_shade`] plus the region's palette family.
///
/// Split from `loose_shade` rather than folded into it because the two have
/// different subjects. Surface cover belongs to the country it is lying on;
/// a lens sealed inside the massif and the scree at a cliff foot do not, and
/// gravel has one family anyway. Adding the family here rather than at the
/// four call sites keeps `loose_shade` meaning exactly what it used to.
fn cover_shade(ctx: &Ctx, purpose: Purpose, x: i32, y: i32) -> u8 {
    palette_family(ctx, x, y, false) * TONES + loose_shade(ctx, purpose, x, y)
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
        //
        // Read off the *plans* (the eroded surface), not `Terrain::slope`/
        // `elev` (the pre-erosion curve): round-4 task 4. `surface_y` is a
        // central difference away from a slope the same way `elev` is --
        // the two differ only by the sign flip `elev = datum - surface_y`,
        // so `|Δsurface_y| == |Δelev|` and the slope reads identically. At
        // `world_age == 0` `surface_y` is `round(datum - elev)`, so this is
        // the same quantity `Terrain::slope`/`elev` gave before, rounding
        // aside; at `world_age > 0` it is the *true* eroded surface, which
        // is the whole point -- a valley the erosion pass just filled with
        // sediment should read as a valley floor here too.
        let plan_surface_y = |x: i32| ctx.plans[x.clamp(0, ctx.terrain.w - 1) as usize].surface_y;
        let plan_slope = ((plan_surface_y(x + 1) - plan_surface_y(x - 1)) as f32 / 2.0).abs();
        let plan_elev = ctx.terrain.datum() - c.surface_y as f32;
        let is_valley_floor = plan_slope < 0.1 && plan_elev < -0.45 * p.relief_amplitude;
        // Dry country carries sand instead of soil, all the way down rather
        // than as a skin over it: a desert whose dunes sit on a soil profile
        // reads as a costume over the same world, which is exactly the
        // sameness this regional layer exists to break.
        let sandy = ctx.terrain.is_sandy(x);
        // Talus reads as gravel, at the *top* of the profile rather than the
        // bottom: rockfall lands on the blanket, it does not seep under it.
        // `plan_from` already folded this depth into `soil_depth` (deposits
        // deepen the blanket, never mint elevation -- `column.rs`'s note on
        // `extra_cover`), so this is purely a recolouring of cover cells
        // that were already going to be placed; zero new placement, and the
        // dithered soil/stone contact at the bottom is untouched by it.
        let talus_cells = if ctx.deposits.talus[x as usize] >= 1.0 {
            (ctx.deposits.talus[x as usize].round() as i32).min(c.soil_depth)
        } else {
            0
        };
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
            let (m, shade) = if depth < talus_cells {
                // Rockfall, not the native profile: gravel takes its buried
                // family so it reads as broken rock rather than as scree
                // lying on open ground -- same reasoning as a sealed lens
                // (`pockets`) and the vug floor (`cave_system`), both of
                // which draw buried gravel the same way.
                ctx.talus_recolored.set(ctx.talus_recolored.get() + 1);
                (ctx.gravel, BURIED_FAMILY * TONES + loose_shade(ctx, Purpose::Dither, x, y))
            } else if noise::unit(ctx.terrain.seed, Purpose::Dither, x, y) < stony * 0.85 {
                (ctx.gravel, loose_shade(ctx, Purpose::Dither, x, y))
            } else if sandy || (is_valley_floor && y < top + 2) {
                (ctx.sand, cover_shade(ctx, Purpose::Shade, x, y))
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

/// Longest run the talus pass will walk looking for the foot of a fall.
const MAX_FALL: i32 = 120;

/// A drop of at least this many cells counts as a cliff for the brow and
/// talus passes. Below it the "face" is a slope, and hanging a lip off it
/// would read as a mistake rather than as an overhang.
const CLIFF_DROP: i32 = 6;

/// Columns the near scale looks over. A terrace riser is a single-column
/// jump and a snap-driven face resolves within a handful of columns.
const RUN_NEAR: i32 = 4;

/// The escarpment scale, and the drop that qualifies at it.
///
/// **This is the fix for the brows/talus blindness.** The near scale asks
/// for six cells within four columns -- a slope of 1.5 -- and a regional
/// escarpment does not have that anywhere along it: it spends tens of
/// columns falling a hundred cells, which is a slope near 1. So the biggest
/// faces in the world, the ones most obviously wanting a lip and an apron,
/// were the ones the detector was most certain to miss. `brows` wrote 34
/// cells and `talus` 148 in a 1.3M-cell world.
///
/// Not simply a proportional bar at the longer run: `CLIFF_DROP * RUN_FAR /
/// RUN_NEAR` is 30, i.e. the same slope of 1.5, which would find exactly
/// nothing extra for exactly the same reason. The far scale exists to catch
/// a *gentler* face that is nonetheless very tall, so its bar is a slope of
/// 1.0. A face qualifying at either scale qualifies.
const RUN_FAR: i32 = 20;
const CLIFF_DROP_FAR: i32 = 20;

/// Cap on how far a brow reaches out, and on how tall a talus heap starts.
///
/// Bounds the work without gating whether it happens -- the landmine that
/// has been written twice in this engine already (`if too_big { return }`
/// claims the largest cases deserve the least behaviour). A brow is still
/// hung on every qualifying face however deep the drop; only its reach
/// saturates. They also make each pass's column margin a *number* rather
/// than an unbounded claim, which is what the pass table has to declare.
const MAX_BROW_REACH: i32 = 20;
const MAX_TALUS_PEAK: i32 = 30;

/// Cliff edges as `(edge_x, direction, drop)`, where `direction` is +1 when
/// the ground falls away to the right and -1 when it falls to the left.
fn cliff_edges(plans: &[ColumnPlan], w: i32) -> Vec<(i32, i32, i32)> {
    // Measured over a run, not between neighbours.
    //
    // Adjacent columns were enough when the macro shape was one smooth wave
    // with terrace snaps cut into it: a snap is a single-column jump. Once
    // the skyline came from regional escarpments, the steep ground was spread
    // over several columns and no pair of neighbours differed by six -- so
    // `talus` wrote nothing at all in any world, and scree and overhangs
    // silently left the game. The test that counts per-pass output is what
    // said so; every render still looked like a plausible world.
    //
    // Two runs now, for the same reason read one scale further out. See
    // `RUN_FAR`.
    let mut edges = Vec::new();
    let at = |x: i32| plans[x.clamp(0, w - 1) as usize].surface_y;
    for x in 0..w {
        let here = at(x);
        for dir in [1, -1] {
            // The edge is the last high column, so the neighbour on the
            // falling side must not be higher.
            if at(x + dir) < here {
                continue;
            }
            // The deepest fall found at either scale. Taking the max rather
            // than the first match matters: a face that qualifies at both
            // must be sized by the *escarpment* it is part of, not by the
            // first four columns of it.
            let near = (1..=RUN_NEAR).map(|d| at(x + dir * d)).max().unwrap_or(here) - here;
            let far = (1..=RUN_FAR).map(|d| at(x + dir * d)).max().unwrap_or(here) - here;
            let drop = if far >= CLIFF_DROP_FAR {
                far.max(near)
            } else if near >= CLIFF_DROP {
                near
            } else {
                continue;
            };
            edges.push((x, dir, drop));
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
        // Only hang a lip from bare rock. The origin's own topmost cell is
        // what every written cell ultimately has to trace an attached path
        // back through, and a soil-covered origin's topmost cell is loose
        // Powder -- not part of the attached network at all, whatever sits
        // under it. A lip hung there is structurally disconnected from the
        // massif however solid it looks, which the far escarpment scale's
        // "gentler but tall" detection (`RUN_FAR`/`CLIFF_DROP_FAR`) makes
        // reachable: a long, gradual slope can qualify as an edge at many
        // consecutive columns without any one of them being steep enough to
        // clear `plan_from`'s soil cutoff. Round-4 finding R4-3 has the
        // repro this was found from (erosion turning per-preset ages on
        // made these gentle escarpments common enough to hit reliably).
        if ctx.plans[x as usize].soil_depth > 0 {
            continue;
        }
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
            .min(drop - 1)
            .min(MAX_BROW_REACH);
        // Thickness scales with the face too, which it did not before: a lip
        // reaching twenty cells out of a two-cell slab reads as a diving
        // board rather than as rock. Extended rather than replaced -- the
        // reach term above already half-did this and is left as it was.
        let thick = 2
            + (noise::unit(ctx.terrain.seed, Purpose::Pocket, x, dir * 13) * 2.0) as i32
            + (drop / 22).min(3);
        for row in 0..thick {
            let y = top + row;
            // Never below the local water table. A lip that dips underwater
            // can end up with `ponds` filling both above and below it --
            // the same round-4 finding R4-3 as the soil-origin gate above,
            // a different way to reach it: the origin was bare rock, but
            // the *lip* still hung over a hollow that turned out to be
            // flooded on both sides once ponds ran. `ponds` only knows how
            // to fill a hollow that reaches the open surface (`vaults`
            // handles the sealed-chamber case, deliberately, with its own
            // equator rule); a rock shelf straddling the table is not that.
            if y >= ctx.plans[x as usize].table_y {
                break;
            }
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
    let p = ctx.terrain.params;
    if p.talus_max_height <= 0.0 {
        return 0;
    }
    let w = ctx.terrain.w;
    let ground = |x: i32| ctx.plans[x.clamp(0, w - 1) as usize].surface_y;

    // Heap height per column, before it is made to stand up.
    let mut heap = vec![0i32; w as usize];
    for (x, dir, _) in cliff_edges(&ctx.plans, w) {
        // Walk to the foot of the fall. Scree collects at the bottom of a
        // slope, and the column beside a cliff edge is the top of one — a
        // distinction that did not exist while every cliff was a
        // single-column terrace snap, and that made this pass write nothing
        // at all once escarpments fell over tens of columns.
        let mut foot = x + dir;
        let mut guard = 0;
        while guard < MAX_FALL {
            let next = foot + dir;
            if next < 0 || next >= w || ground(next) <= ground(foot) {
                break;
            }
            foot = next;
            guard += 1;
        }
        if foot < 0 || foot >= w || foot == x {
            continue;
        }
        let fall = ground(foot) - ground(x);
        // Apron volume follows the face that shed it. `talus_max_height`
        // alone is 8 to 18, so every cliff past about thirty cells got the
        // same heap however tall it was -- a hundred-cell escarpment and a
        // terrace riser shedding identically, which is most of why scree
        // never read as a consequence of anything. Still capped, and still
        // never more than half the fall, so an apron cannot bury its own
        // cliff.
        let peak = (p.talus_max_height as i32 + fall / 5).min(fall / 2).min(MAX_TALUS_PEAK);
        if peak <= 0 {
            continue;
        }
        // Deepest against the slope, thinning away from it. Overlapping
        // aprons take the larger rather than summing, so two cliffs sharing a
        // basin do not stack into a mound taller than either.
        for step in 0..(peak * 2) {
            let tx = foot + dir * step;
            if tx < 0 || tx >= w {
                break;
            }
            let h = peak - step / 2;
            if h <= 0 {
                break;
            }
            heap[tx as usize] = heap[tx as usize].max(h);
        }
    }

    // **Make the heap stand up, rather than trusting the geometry that drew
    // it.** This pass had four separate at-rest failures, each a different
    // exposed free face — a wedge inheriting the ground's slope, a toe cut
    // off into a vertical face, an apron standing proud of gently rising
    // ground — and each fix produced the next. Four failures of the same kind
    // is the signal `CLAUDE.md` names: the approach was wrong, not the
    // tuning. Deriving a stable shape from cliff geometry means enumerating
    // every way terrain can undercut it, and the enumeration was never done.
    //
    // So the shape is no longer derived. The apron's *top surface* is taken
    // as a profile and clamped, by the same two-sweep repose taper the soil
    // blanket uses, until no part of it is steeper than gravel stands at.
    // Whatever the geometry above proposes, what gets written is a surface
    // that cannot avalanche — a property of the sweep rather than of the
    // case analysis.
    let step = (p.soil_slope_cutoff * ctx.gravel_tan).max(0.05);
    // Work in +up elevation so "steeper" is one comparison, not two.
    let mut top: Vec<f32> = (0..w).map(|x| -(ground(x) as f32) + heap[x as usize] as f32).collect();
    for x in 1..top.len() {
        top[x] = top[x].min(top[x - 1] + step);
    }
    for x in (0..top.len().saturating_sub(1)).rev() {
        top[x] = top[x].min(top[x + 1] + step);
    }

    let mut n = 0;
    for x in 0..w {
        let height = (top[x as usize] + ground(x) as f32).floor() as i32;
        if height <= 0 {
            continue;
        }
        let g = ground(x);
        for y in (g - height).max(0)..g {
            // Open air only. The apron heaps against the face and on top of
            // the ground; it never eats into either.
            if world.get(x, y).material != material::EMPTY {
                continue;
            }
            world.set(x, y, Cell::new(ctx.gravel, loose_shade(ctx, Purpose::Pocket, x, y)));
            n += 1;
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
/// The largest half-extent a chamber may reach, in cells, on either axis.
///
/// A cap that bounds *work*, never whether the pass runs -- the landmine
/// CLAUDE.md records twice, from `rigid::fracture` declining to break any
/// region larger than its body-size cap and so dissolving the biggest
/// collapses into dust. A chamber too large for this is drawn smaller; a
/// massif too thin for the depth band simply has no chamber, and the counter
/// says so.
const MAX_VAULT_EXTENT: i32 = 30;

/// How thick the solid stone rind around a chamber must be, in cells.
///
/// Two rather than `pockets`' one, and the difference is the whole at-rest
/// argument. A lens is solid powder: a one-cell rind is enough to guarantee
/// nothing is flush with a free face. A chamber is *hollow*, so its roof is
/// an unsupported span and its floor is loose material over a void -- a
/// single stray cell of air on the far side of a one-cell rind would put a
/// hole through into whatever is next door and let the floor run out of it.
const VAULT_RIND: i32 = 2;

/// Half-extents of a cave system's envelope, in cells. ~180x70, per the
/// round-3 spec; constants tuned by eye against the ASCII probe.
const CAVE_HALF_W: i32 = 90;
const CAVE_HALF_H: i32 = 35;
/// Envelope-local grid dimensions for the collect phase.
const CAVE_GRID_W: i32 = 2 * CAVE_HALF_W + 1;
const CAVE_GRID_H: i32 = 2 * CAVE_HALF_H + 1;

/// Size of one Worley lattice cell, in *sheared* cave-space cells.
///
/// Together with [`CAVE_SQUASH`] this sets how many lattice cells the
/// envelope holds: `(2 * CAVE_HALF_W / CAVE_CELL)` across and
/// `(2 * CAVE_HALF_H * CAVE_SQUASH / CAVE_CELL)` down. Round 3 shipped 52,
/// which gives ~3.5 x 2.7 = 9 -- and round 5 measured what that actually
/// means: the whole envelope is one open lens with the ceiling guard's
/// stone teeth in it read as pillars, median open column 30 in a 69-tall
/// box (`Reports/cave-beauty-review-2026-08.md`'s measured addendum). At 9
/// lattice cells there is no anatomy to have. 22.0 gives ~8.2 x 3.8 = 31
/// cells -- `cave_probe field=` at this value measures open column med 4,
/// p95 11-12, max 20-28 over three field seeds, against the old field's
/// med 30 (see the round-5 task file's table); the built world has to be
/// re-measured because the ceiling guard and gravel floors are downstream
/// of the raw field and can reshape it.
const CAVE_CELL: f32 = 22.0;

/// Vertical compression applied before the field is sampled, so everything
/// the threshold carves -- chambers and passages both -- comes out wider
/// than tall, lying along the bedding rather than cutting across it.
///
/// Round 3 shipped 2.0; round 5 drops it to 1.2, landed together with
/// [`CAVE_CELL`] and [`CAVE_THRESHOLD`] because the three only mean
/// anything as a set (`Reports/worldgen-implementation-tasks-round5-2026-08.md`
/// task 2). This is the anisotropy of the *lattice*, not of the bedding
/// dip -- that is `strata_offset`'s shear, applied separately below, and
/// unaffected by this constant; a strip has to confirm by eye that systems
/// still lie along the visible dip after the drop, and if they stop, that
/// is a finding, not a reason to put the squash back.
const CAVE_SQUASH: f32 = 1.2;

/// Cells over which the threshold fades to nothing at the envelope edge,
/// per axis. Without the fade, a passage crossing the boundary is sawn off
/// into a dead-plumb face the full height of the envelope -- the round-2
/// scan-cap lesson arriving at envelope scale, seen in the first ASCII dump
/// as a 70-row straight wall at the bbox edge. Fading the threshold pinches
/// every void shut before it reaches the wall, so the system ends in
/// naturally narrowing passages instead of a cut. The vertical fade is half
/// the horizontal one for the same reason [`CAVE_SQUASH`] is 2: a fade that
/// reads as the same *shape* on both axes has to match the anisotropy.
const CAVE_EDGE_FADE_X: f32 = 14.0;
const CAVE_EDGE_FADE_Y: f32 = 7.0;

/// The one threshold: a cell is void where `F2 - F1 < CAVE_THRESHOLD`.
///
/// `F2 - F1` is zero along Worley-cell boundaries and grows toward centres,
/// so a low threshold carves the boundary network -- passages -- and the
/// junctions where boundaries meet open into wider bulges -- chambers. The
/// spec sketched a second sub-threshold (`t_chamber`, carving discs around
/// the *centres*), and that sketch does not survive the geometry: a disc
/// around a feature point never touches the boundary web (at radius 0.3 of
/// a unit lattice the gap to the `F2 - F1 < 0.12` strip is ~0.14 of solid
/// stone), so every chamber it adds is a sealed satellite the component
/// keep below then throws away. One threshold on one field is also exactly
/// what the research names (`Reports/worldgen-design.md` §7). See the
/// round-3 finding.
///
/// Round 3 shipped 0.34, which at the round-3 [`CAVE_CELL`] opened ~53% of
/// a 9-lattice-cell envelope -- one flooded room, not a network. Round 5
/// drops it to 0.09 alongside the smaller lattice cell above; the two
/// changes are not independent; see [`CAVE_CELL`] for the measured field
/// numbers this pair produces.
const CAVE_THRESHOLD: f32 = 0.09;

/// Longest horizontal run of void with stone directly above it that a
/// system may keep, in cells -- the roof-span bound the round-2 arithmetic
/// cleared for chambers. Runs longer than this get a stone tooth dropped
/// from their middle ([`carve_cave_void`]) until every span complies.
const MAX_CEILING_SPAN: i32 = 36;

/// A kept component smaller than this is not a system: a sliver of passage
/// with no chamber is a dig reward of nothing. Rejected wholesale, same as
/// a failed seal.
const MIN_SYSTEM_CELLS: usize = 80;

/// Chance a placement draw is a geode vug rather than a cave system. The
/// vug stays as the rare jewel variant; the cave is the main event.
const VUG_CHANCE: f32 = 0.25;

/// Speleothem placement, all tuned against the ASCII probe:
/// per-candidate-column chance at full ceiling height (the smoothstep over
/// the open span makes tall chambers -- drip height -- much denser than low
/// passages), the crystal minority, and how often a formation is a paired
/// stalactite-over-stalagmite almost meeting -- the postcard shot.
const SPELEO_DENSITY: f32 = 0.30;
const SPELEO_CRYSTAL: f32 = 0.15;
const SPELEO_PAIR: f32 = 0.25;

/// Round-5 task 4b: the fixed `SPELEO_SPACING = 4` this replaces enforced
/// *even* spacing everywhere, which is precisely the "reads as a comb"
/// artefact the beauty review named -- the opposite of drip concentration.
/// The minimum gap between candidate columns is now driven by
/// [`noise::value_1d`] on `Purpose::Drip`, a low-frequency field sampled
/// every [`DRIP_SCALE`] cells: in a wet stretch the gap shrinks to
/// [`SPELEO_SPACING_MIN`], letting formations bunch; in a dry one it grows
/// to [`SPELEO_SPACING_MAX`], leaving the stretch close to bare. The floor
/// stays a hard 2 regardless of how wet a stretch reads -- two formations
/// one column apart can still read as merged into a single wall (measured:
/// dropping it to 1 *reduced* the counted total, because the 2-wide
/// secondary taper then reached into the one clear column between
/// neighbours and merged them into a shape with no free-standing face at
/// all), which is the "keep a minimum of 1-2 columns" the task asks for.
/// The same reasoning gates the secondary taper off entirely below a gap
/// of 4: it is a cosmetic width variation, not worth the merge risk in a
/// tight cluster.
///
/// **`SPELEO_SPACING_MAX` measured down from a literal reading, not
/// guessed.** The bar is 60 free-standing formations/system (from 17);
/// `cave_probe`'s own silhouette test only counts a column as one formation
/// if *both* neighbours are void, so a cluster has to stay just loose
/// enough to keep each member individually free-standing while packing as
/// many in as it can. Un-throttled dry stretches (`SPELEO_SPACING_MAX`
/// near the envelope's own width) left most of a system's length
/// contributing nothing, measuring 27-38/system; loosening the dry ceiling
/// to 14 -- still an order of magnitude sparser than the wet floor of 2 --
/// raised it to 35-45/system without moving the height bars (p50 stays at
/// the task-4a ceiling of 3, p90 18-19, well inside both). 60 was not
/// reached; see the round-5 task file's finding for the honest number and
/// what was tried.
const SPELEO_SPACING_MIN: i32 = 2;
const SPELEO_SPACING_MAX: i32 = 14;
const DRIP_SCALE: f32 = 40.0;

/// What the vault pass did, beyond the cell count the pass table carries.
///
/// The pass-table row says *whether* it fired; these say *what* it made, and
/// they are printed beside the table (see [`vaults`]) because a cave is the
/// extreme case of "a picture cannot show whether the thing you built is
/// what produced it" -- the whole feature is invisible by design until dug
/// into, and a render of a world with a dead decoration stage looks exactly
/// like a render of a working one.
#[derive(Default)]
struct VaultReport {
    cells: usize,
    systems: usize,
    chambers: usize,
    passage_cells: usize,
    speleothem_cells: usize,
    water_cells: usize,
}

/// Sealed cave systems and geode vugs, buried far below the surface: the
/// found-a-secret moment.
///
/// **Genesis-only and zero standing cost.** Nothing here runs per frame; a
/// system is written once, and concealment comes free from the viewport
/// rather than from any render work -- at 200 rows below the surface it is
/// simply never on screen until someone digs to it.
///
/// Grown from [`pockets`]'s collect-then-verify-seal skeleton, which round 1
/// already generalised to rotated shapes. The contract is that one, kept
/// exactly: **every cell of the envelope -- void, lining and a rind of
/// solid rock around the whole of it -- is checked to be stone before a
/// single cell is written.** Approximating that per cell is what leaves
/// loose material outcropping on a free face, and here it would be worse
/// than for a lens, because a chamber that clips an existing void spills its
/// floor into that void on frame one and the world is no longer at rest.
///
/// Two shapes, because one shape repeated is a feature of the generator
/// rather than a discovery:
///
///   * a **cave system** -- a Worley `F2 - F1` field thresholded inside a
///     bounded envelope and sheared along the local bedding, so what comes
///     out is chambers linked by passages that follow the visible strata:
///     the anatomy real caves have, from one field and one threshold. See
///     [`carve_cave_void`].
///   * a **geode vug** -- a single ellipse with a crystal lining grown
///     inward from its wall, which is the one bright thing in the deep
///     massif.
pub fn vaults(ctx: &Ctx, world: &mut World) -> usize {
    let p = ctx.terrain.params;
    if p.vault_density <= 0.0 {
        return 0;
    }
    let seed = ctx.terrain.seed;
    let (w, h) = (ctx.terrain.w, ctx.terrain.h);
    // The count for the whole world, not per region: a vault is meant to be
    // rare enough that finding one is an event, and `pockets`' per-region
    // draw would put one in every 64-cell block.
    let whole = p.vault_density.floor() as i32;
    let extra = i32::from(noise::unit(seed, Purpose::Vault, -1, -1) < p.vault_density.fract());
    let mut written = 0;
    let mut report = VaultReport::default();
    for k in 0..whole + extra {
        // Position from its own draw. Rejected rather than nudged when the
        // massif there is too thin -- moving a rejected chamber to wherever
        // it would fit is how a "rare secret" ends up in the same kind of
        // place in every world.
        let cx = (noise::unit(seed, Purpose::Vault, k, 0) * w as f32) as i32;
        if cx < 0 || cx >= w {
            continue;
        }
        let plan = ctx.plans[cx as usize];
        let top = plan.surface_y + p.vault_min_depth;
        let bottom = plan.bedrock_top_y - p.vault_bedrock_margin;

        // Which shape. A draw per system, on its own coordinate so the
        // choice is not correlated with where it landed.
        let vug = noise::unit(seed, Purpose::Vault, k, 2) < VUG_CHANCE;

        if !vug {
            // The envelope must sit entirely inside the depth band and, with
            // its rind, inside the world. Rejected rather than nudged when it
            // cannot -- moving a rejected system to wherever it fits is how a
            // "rare secret" ends up in the same kind of place in every world.
            // The band being too shallow is the intended outcome in a small
            // world, not a failure to handle: such a world has no system and
            // the pass counter reports zero rather than the pass quietly
            // relaxing its own depth rule to produce one.
            let lo = top + CAVE_HALF_H + VAULT_RIND;
            let hi = bottom - CAVE_HALF_H - VAULT_RIND;
            if hi < lo || cx - CAVE_HALF_W - VAULT_RIND < 0 || cx + CAVE_HALF_W + VAULT_RIND >= w {
                continue;
            }
            let cy = lo + (noise::unit(seed, Purpose::Vault, k, 1) * (hi - lo + 1) as f32) as i32;
            let r = cave_system(ctx, world, k, cx, cy);
            written += r.cells;
            report.cells += r.cells;
            report.systems += r.systems;
            report.chambers += r.chambers;
            report.passage_cells += r.passage_cells;
            report.speleothem_cells += r.speleothem_cells;
            report.water_cells += r.water_cells;
            continue;
        }

        // The band is empty in a shallow world -- same rule as the system's,
        // at the vug's own smaller size.
        if bottom - top < 2 * MAX_VAULT_EXTENT {
            continue;
        }
        let span = (bottom - top) as f32;
        let cy = top + (noise::unit(seed, Purpose::Vault, k, 1) * span) as i32;

        // The vug is a single ellipse. (The multi-lobe grotto this code once
        // unioned into lumpy caverns is superseded by the cave system above;
        // the one-entry lobe list survives because the shape test and the cap
        // below are written against it and verified in that form.)
        let a = 8.0 + noise::unit(seed, Purpose::Vault, k * 17, 4) * 12.0;
        let b = 6.0 + noise::unit(seed, Purpose::Vault, k * 17, 5) * 6.0;
        // **The cap is applied to the lobe, not to the scan box**, and
        // the difference is a correctness bug rather than a preference.
        // Capping the scan instead was the first version: a lobe reaching
        // past the cap had its far end simply never visited, so those
        // cells were neither written *nor seal-checked* -- the chamber
        // came out with a flat sawn-off face, and the guarantee that the
        // whole envelope is stone quietly stopped covering all of it.
        // Shrinking the lobe keeps the shape whole and the check total,
        // which is the landmine CLAUDE.md states twice: a size cap must
        // bound work, never gate whether something happens.
        let limit = (MAX_VAULT_EXTENT - VAULT_RIND - 1) as f32;
        let lobes: Vec<(f32, f32, f32, f32)> =
            vec![(0.0, 0.0, a.min(limit).max(2.0), b.min(limit).max(2.0))];
        let inside = |dx: f32, dy: f32, grow: f32| {
            lobes.iter().any(|&(ox, oy, a, b)| {
                ((dx - ox) / (a + grow)).powi(2) + ((dy - oy) / (b + grow)).powi(2) <= 1.0
            })
        };
        // The true bounding box of the union plus the rind. Not capped here
        // -- the lobes are already inside the cap by construction above, so
        // this scan is guaranteed to cover the whole envelope, which is what
        // makes the seal check total.
        let ext = lobes
            .iter()
            .map(|&(ox, oy, a, b)| ((ox.abs() + a).ceil() as i32).max((oy.abs() + b).ceil() as i32))
            .max()
            .unwrap_or(0)
            + VAULT_RIND
            + 1;

        // Collect first, write only if the whole envelope is solid stone.
        let mut hollow: Vec<(i32, i32)> = Vec::new();
        let mut lining: Vec<(i32, i32)> = Vec::new();
        let mut sealed = true;
        'envelope: for dy in -ext..=ext {
            for dx in -ext..=ext {
                let (px, py) = (cx + dx, cy + dy);
                let (fx, fy) = (dx as f32, dy as f32);
                // Outside even the rind: not part of this chamber at all.
                if !inside(fx, fy, VAULT_RIND as f32) {
                    continue;
                }
                if px < 0 || px >= w || py < 0 || py >= h {
                    sealed = false;
                    break 'envelope;
                }
                if world.get(px, py).material != ctx.stone {
                    sealed = false;
                    break 'envelope;
                }
                if inside(fx, fy, 0.0) {
                    // A vug's lining is the outermost ring of the chamber:
                    // inside the wall, but not inside a shape shrunk by the
                    // lining thickness. Grown *inward* rather than outward so
                    // the rind check above still covers it. Thickness one to
                    // three cells, per cell -- at the round-2 one-to-two the
                    // rim read as a perfect ring, which is a generator's
                    // shape, not a geode's.
                    let thickness = 1.0 + 2.0 * noise::unit(seed, Purpose::Vault, px, py);
                    if vug && !inside(fx, fy, -thickness) {
                        lining.push((px, py));
                    } else {
                        hollow.push((px, py));
                    }
                }
            }
        }
        if !sealed || hollow.is_empty() {
            continue;
        }

        // **The floor is filled flat, and "flat" is doing structural work
        // rather than aesthetic work.** A floor that followed the chamber's
        // curve would be loose gravel lying on a bowl -- a slope at every
        // cell, most of them past repose -- and it would run on frame one,
        // which is the whole at-rest guarantee gone. Filling every hollow
        // cell from a chosen row downward makes the gravel's top surface a
        // horizontal line by construction, and everything under it is packed
        // solid against stone. Nothing needs clamping afterwards because
        // nothing is ever placed on a slope.
        //
        // The first version wrote gravel only at the single lowest row, which
        // is what "the lowest row, filled flat" literally says and is not a
        // floor at all: the chamber's curved bottom stayed bare stone and the
        // gravel was a two-cell strip at the very bottom of the bowl.
        let floor_y = hollow.iter().map(|&(_, y)| y).max().unwrap_or(cy);
        let ceiling_y = hollow.iter().map(|&(_, y)| y).min().unwrap_or(cy);
        let thickness = 2 + (noise::unit(seed, Purpose::Vault, k, 8) * 3.0) as i32;
        // Never fill the chamber solid: leave at least two rows of head-room,
        // or a small grotto becomes a lump of buried gravel with no void in
        // it and there is nothing to find.
        let floor_top = (floor_y - thickness + 1).max(ceiling_y + 2);

        // Standing water when the chamber floor sits below the local water
        // table, written exactly as `ponds` writes it: level, and full --
        // `Cell::new` leaves `aux == 0`, which on a `Liquid` means **full**,
        // the opposite of the `Powder` convention (CLAUDE.md's two-conventions
        // gotcha; writing a literal 0 fill here would be correct by accident
        // rather than on purpose, so it is stated).
        //
        // The surface is the table clipped into the chamber. At the shipped
        // `vault_min_depth` the table is always far above the chamber, so the
        // clip binds and the chamber floods to its ceiling -- see the round-2
        // finding, which measures how often that is and why it is flagged
        // rather than tuned away here.
        let table = ctx.plans[cx as usize].table_y;
        let flooded = floor_top > table;
        // **The water surface has to be a row wide enough to be a surface**,
        // and this is an at-rest requirement rather than a cosmetic one.
        //
        // A chamber's topmost hollow row is often one or two cells -- the
        // apex of an ellipse, narrowed further by a vug's lining -- and
        // filling it puts a one-cell column of water standing above the body
        // it sits on. That is a head difference, and the liquid solver does
        // exactly what it should with one: the cell drains into the wider
        // water beneath it and comes out empty. Measured, not reasoned about:
        // `rolling` seed 1 lost precisely one cell at (70, 257), the single
        // hollow cell on row 257 of a chamber whose next row down is fourteen
        // wide.
        //
        // So the surface walks down to the first row that is at least
        // `MIN_SURFACE_WIDTH` wide, and whatever is above it stays air -- a
        // pocket of trapped gas under the roof, which is both what a sealed
        // flooded void actually contains and a better thing to break into
        // than a solid block of water.
        // The rule that works is not a width bar but a *shape* one: the
        // surface must be the chamber's **widest** row, so that no row above
        // the waterline is narrower than the water under it. A chamber is an
        // ellipse, or a union of them, so filling to any row above the
        // equator makes a flask -- a narrow neck standing on a wide body --
        // and the solver drains the neck. Filling to the equator makes a
        // bowl, where every row below the surface is narrower than it, which
        // is the shape a pond has and the shape that holds.
        //
        // A bar of "at least N cells wide" was tried first and is recorded
        // here as wrong rather than untried: at N = 5 `rolling` seed 4 still
        // lost six cells off a six-wide row standing over a much wider one.
        // The quantity that matters was never the absolute width.
        let mut widths: std::collections::BTreeMap<i32, i32> = Default::default();
        for &(_, y) in &hollow {
            *widths.entry(y).or_default() += 1;
        }
        let equator = widths.iter().max_by_key(|&(_, &n)| n).map(|(&y, _)| y).unwrap_or(ceiling_y);
        let water_surface = table.max(equator);

        for &(px, py) in &lining {
            world.set(px, py, Cell::new(ctx.crystal, loose_shade(ctx, Purpose::Vault, px, py)));
            written += 1;
        }
        report.systems += 1;
        report.chambers += 1;
        for &(px, py) in &hollow {
            let cell = if py >= floor_top {
                // Buried gravel's family, for the same reason a sealed lens
                // takes it: this is read against solid stone and nothing else.
                Cell::new(ctx.gravel, BURIED_FAMILY * TONES + loose_shade(ctx, Purpose::Vault, px, py))
            } else if flooded && py >= water_surface {
                report.water_cells += 1;
                Cell::new(ctx.water, loose_shade(ctx, Purpose::Shade, px, py))
            } else {
                Cell::EMPTY
            };
            world.set(px, py, cell);
            written += 1;
        }
        report.cells += hollow.len() + lining.len();
    }
    // The counters next to the picture, printed only when something placed:
    // the pass-table row alone cannot say whether the anatomy stages fired,
    // and a cave is invisible in any render until someone digs to it. The
    // format deliberately does not match the table's `name N cells` rows, so
    // the sweep's parser never mistakes it for one.
    if report.systems > 0 {
        println!(
            "  vaults detail: systems {} chambers {} passages {} speleothems {} water {}",
            report.systems,
            report.chambers,
            report.passage_cells,
            report.speleothem_cells,
            report.water_cells
        );
    }
    written
}

/// Envelope-local index into a cave grid. `(0, 0)` is the placement draw.
fn cave_idx(dx: i32, dy: i32) -> usize {
    ((dy + CAVE_HALF_H) * CAVE_GRID_W + dx + CAVE_HALF_W) as usize
}

/// Whether the cave plan leaves something solid at `(dx, dy)`: undisturbed
/// rock (anything outside the kept void, including outside the envelope),
/// or planned floor gravel. What the floor verifier leans on.
fn planned_solid(void: &[bool], floor: &[Option<(i32, i32, i32)>], dx: i32, dy: i32) -> bool {
    if dx.abs() > CAVE_HALF_W || dy.abs() > CAVE_HALF_H {
        return true;
    }
    if !void[cave_idx(dx, dy)] {
        return true;
    }
    matches!(floor[(dx + CAVE_HALF_W) as usize], Some((_, b, h)) if h > 0 && dy > b - h)
}

/// Keep only the connected component containing the seed point -- the void
/// cell nearest the envelope centre, ties broken in raster order -- over a
/// 4-neighbour flood. A disconnected satellite chamber is a second system
/// nobody can reach from the first, so it goes back to being stone.
fn keep_seed_component(void: &mut [bool]) {
    let mut seed = None;
    let mut best = i64::MAX;
    for dy in -CAVE_HALF_H..=CAVE_HALF_H {
        for dx in -CAVE_HALF_W..=CAVE_HALF_W {
            if void[cave_idx(dx, dy)] {
                let d = (dx as i64).pow(2) + (dy as i64).pow(2);
                if d < best {
                    best = d;
                    seed = Some((dx, dy));
                }
            }
        }
    }
    let Some(seed) = seed else { return };
    let mut kept = vec![false; void.len()];
    kept[cave_idx(seed.0, seed.1)] = true;
    let mut stack = vec![seed];
    while let Some((x, y)) = stack.pop() {
        for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
            if nx.abs() <= CAVE_HALF_W
                && ny.abs() <= CAVE_HALF_H
                && void[cave_idx(nx, ny)]
                && !kept[cave_idx(nx, ny)]
            {
                kept[cave_idx(nx, ny)] = true;
                stack.push((nx, ny));
            }
        }
    }
    void.copy_from_slice(&kept);
}

/// The first ceiling run longer than [`MAX_CEILING_SPAN`], as
/// `(row, start, len)`, topmost-leftmost first so the guard is
/// deterministic. A ceiling run is a maximal horizontal run of void cells
/// each with non-void directly above -- which, once the seal has passed, is
/// stone: the unsupported roof span the guard bounds.
fn first_long_ceiling_run(void: &[bool]) -> Option<(i32, i32, i32)> {
    for dy in -CAVE_HALF_H..=CAVE_HALF_H {
        let mut run = 0;
        for dx in -CAVE_HALF_W..=CAVE_HALF_W + 1 {
            let ceiling = dx <= CAVE_HALF_W
                && void[cave_idx(dx, dy)]
                && (dy == -CAVE_HALF_H || !void[cave_idx(dx, dy - 1)]);
            if ceiling {
                run += 1;
            } else {
                if run > MAX_CEILING_SPAN {
                    return Some((dy, dx - run, run));
                }
                run = 0;
            }
        }
    }
    None
}

/// Collect a cave system's void as a boolean grid over the envelope.
///
/// Worley `F2 - F1` under [`CAVE_THRESHOLD`], evaluated in a frame sheared
/// onto the local bedding and squashed by [`CAVE_SQUASH`]; then three
/// repairs alternate to a fixpoint (round-5 task 1): the seed component is
/// kept, the ceiling-span guard drops a stone tooth into any roof run
/// longer than [`MAX_CEILING_SPAN`], and [`erode_breaches`] retracts the
/// void from anything that is not stone. Returns `None` when what survives
/// is too small to be a system.
///
/// **Why erosion joins this loop rather than running once before or after
/// it.** The old seal check scanned the *whole* dilated envelope in one go
/// and rejected the entire system for a single non-stone cell anywhere in
/// it -- measured (see the round-5 task file's addendum): every rejection
/// across canyon/rolling/wetland was one stray `sand` or `gravel` cell from
/// a `pockets` lens, and the bigger the system the likelier one fell inside
/// its rind. That is the size-cap landmine in a new costume: a check meant
/// to guarantee "nothing loose sits flush with a free face" was instead
/// gating *whether a system exists at all*. Retracting only the void near
/// the breach keeps the guarantee -- by construction, not by inference --
/// while losing only the few cells actually threatened.
///
/// The three repairs cannot run in a fixed sequence because each can create
/// work for another: the ceiling guard's tooth leaves whatever the world
/// already had at that cell un-carved, and if that happens to be a stray
/// grain rather than stone, erosion has a new breach source to react to;
/// erosion shrinking the void can sever a passage and orphan a component,
/// which only the seed-component keep notices; and either can shorten or
/// lengthen a roof run enough to cross [`MAX_CEILING_SPAN`]. Looping until
/// none of the three changes anything terminates because each one that
/// fires removes at least one void cell from a finite grid.
fn carve_cave_void(ctx: &Ctx, world: &World, k: i32, cx: i32, cy: i32) -> Option<Vec<bool>> {
    // A per-system field seed derived from the placement stream: two systems
    // in one world must not share a Worley lattice, or a pair placed near
    // each other would carve correlated shapes.
    let sys = noise::hash(ctx.terrain.seed, Purpose::Vault, k, 97);
    let mut void = vec![false; (CAVE_GRID_W * CAVE_GRID_H) as usize];
    // Bedding shear: the field's vertical coordinate is measured against the
    // local strata surface -- the same `y + strata_offset(x)` locus the shade
    // pass bands the rock with, the benches snap to and the lenses lie in --
    // so the system elongates along the visible dip, tilt and fold included,
    // and a cave reads as geology rather than as noise. Fourth consumer of
    // that one pure function; it has to agree with the other three.
    let base_off = ctx.terrain.strata_offset(cx);
    for dy in -CAVE_HALF_H..=CAVE_HALF_H {
        for dx in -CAVE_HALF_W..=CAVE_HALF_W {
            let v = (dy as f32 + ctx.terrain.strata_offset(cx + dx) - base_off) * CAVE_SQUASH;
            let (f1, f2) =
                noise::worley_f2_f1(sys, Purpose::Cave, dx as f32 / CAVE_CELL, v / CAVE_CELL);
            // Threshold fades to zero at the envelope edge -- see
            // `CAVE_EDGE_FADE_X` for the sawn-off face it removes.
            let fade = ((CAVE_HALF_W - dx.abs()) as f32 / CAVE_EDGE_FADE_X)
                .min((CAVE_HALF_H - dy.abs()) as f32 / CAVE_EDGE_FADE_Y)
                .clamp(0.0, 1.0);
            if f2 - f1 < CAVE_THRESHOLD * fade {
                void[cave_idx(dx, dy)] = true;
            }
        }
    }

    settle_cave_void(ctx, world, cx, cy, &mut void);

    // Round-5 task 3: one monumental chamber, grown around the point of
    // greatest clearance in the settled void, then the whole settle runs
    // again -- growth can only ever breach or over-lengthen a span, never
    // disconnect (it is a pure union), but it can do either of the first
    // two, and re-settling is what turns "grew into a lens" or "grew a roof
    // too wide" back into a system that still satisfies every earlier
    // guarantee.
    let (requested, added) = grow_monumental_chamber(ctx, k, cx, &mut void);
    if requested > 0 {
        settle_cave_void(ctx, world, cx, cy, &mut void);
        // Reported unconditionally, including the zero case: a chamber
        // eaten down to nothing by a nearby breach is exactly the "size cap
        // gates whether it happens" landmine in a new shape if it goes
        // unremarked (CLAUDE.md, and task 1/3's own text).
        let survived = added.iter().filter(|&&idx| void[idx]).count();
        println!("  chamber: requested {requested} cells, {survived} survived the re-settle");
    }

    let count = void.iter().filter(|&&v| v).count();
    (count >= MIN_SYSTEM_CELLS).then_some(void)
}

/// Component keep, ceiling guard and breach erosion, alternated to a
/// fixpoint (round-5 task 1's doc comment on [`carve_cave_void`] has the
/// full reasoning for why none of the three can run only once or only in a
/// fixed order). Factored out because round-5 task 3 has to run it twice:
/// once on the raw carved field, and again after the monumental chamber's
/// growth, which can breach or over-lengthen a span but never disconnect.
fn settle_cave_void(ctx: &Ctx, world: &World, cx: i32, cy: i32, void: &mut [bool]) {
    loop {
        keep_seed_component(void);
        let ceiling = first_long_ceiling_run(void);
        if let Some((y, x0, len)) = ceiling {
            // A stone tooth hung from the run's middle: three rows deep,
            // tapering 5-3-1 wide, so the splitter reads as rock coming down
            // from the roof rather than as a one-cell film. It splits the
            // span into two runs of at most half the original.
            let mx = x0 + len / 2;
            for j in 0..3 {
                let half = 2 - j;
                for x in mx - half..=mx + half {
                    if x.abs() <= CAVE_HALF_W && (y + j).abs() <= CAVE_HALF_H {
                        void[cave_idx(x, y + j)] = false;
                    }
                }
            }
        }
        let eroded = erode_breaches(ctx, world, cx, cy, void);
        if ceiling.is_none() && !eroded {
            break;
        }
    }
}

/// Round-5 task 3: dilate the void's point of greatest clearance into one
/// monumental chamber -- criterion 3's "rooms with necks" and criterion 1's
/// "one monumental anchor formation" both need one conspicuously larger
/// space per system, and the shipped anatomy (task 2) has none.
///
/// The point chosen is a void cell with the greatest Chebyshev distance to
/// the nearest non-void cell, computed by an exact two-pass chessboard
/// distance transform (chessboard distance is separable, unlike Euclidean,
/// so two raster sweeps suffice) over the *settled* void -- the deepest
/// interior point of whatever the system already carved, which is where a
/// real cavern's biggest room tends to sit: as far as possible from every
/// wall the passage network has already found. Among cells within 1 of that
/// maximum, the one with the most room to grow into wins; see the
/// selection code below for why that tie-break is load-bearing rather than
/// cosmetic.
///
/// Half-extents are a per-system draw on [`Purpose::CaveChamber`] (12-24
/// vertical, 1.4x that horizontal), **capped to the room the envelope has
/// left from that centre, never to zero** -- the cap CLAUDE.md asks for
/// twice: it bounds the ellipse, it never gates whether one grows. Returns
/// `(requested, added)`: how many *new* cells the ellipse tried to add
/// (for a printed report -- a chamber that survives re-settling at 0 of a
/// nonzero request is exactly the silently-skipped case that report exists
/// to catch) and the indices added, so the caller can measure how many
/// survive the following settle.
fn grow_monumental_chamber(ctx: &Ctx, k: i32, cx: i32, void: &mut [bool]) -> (usize, Vec<usize>) {
    let n = (CAVE_GRID_W * CAVE_GRID_H) as usize;
    let mut dist = vec![0i32; n];
    for dy in -CAVE_HALF_H..=CAVE_HALF_H {
        for dx in -CAVE_HALF_W..=CAVE_HALF_W {
            let idx = cave_idx(dx, dy);
            dist[idx] = if void[idx] { i32::MAX / 4 } else { 0 };
        }
    }
    let get = |dist: &[i32], dx: i32, dy: i32| {
        if dx.abs() > CAVE_HALF_W || dy.abs() > CAVE_HALF_H { 0 } else { dist[cave_idx(dx, dy)] }
    };
    // Forward pass (top-left to bottom-right), then backward -- together
    // exact for Chebyshev distance, where a single pass is not.
    for dy in -CAVE_HALF_H..=CAVE_HALF_H {
        for dx in -CAVE_HALF_W..=CAVE_HALF_W {
            let idx = cave_idx(dx, dy);
            if dist[idx] == 0 {
                continue;
            }
            let mut d = dist[idx];
            for (ox, oy) in [(-1, 0), (0, -1), (-1, -1), (1, -1)] {
                d = d.min(get(&dist, dx + ox, dy + oy) + 1);
            }
            dist[idx] = d;
        }
    }
    for dy in (-CAVE_HALF_H..=CAVE_HALF_H).rev() {
        for dx in (-CAVE_HALF_W..=CAVE_HALF_W).rev() {
            let idx = cave_idx(dx, dy);
            if dist[idx] == 0 {
                continue;
            }
            let mut d = dist[idx];
            for (ox, oy) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
                d = d.min(get(&dist, dx + ox, dy + oy) + 1);
            }
            dist[idx] = d;
        }
    }

    // The greatest clearance value anywhere in the void, first.
    let mut max_clear = 0i32;
    for dy in -CAVE_HALF_H..=CAVE_HALF_H {
        for dx in -CAVE_HALF_W..=CAVE_HALF_W {
            let idx = cave_idx(dx, dy);
            if void[idx] {
                max_clear = max_clear.max(dist[idx]);
            }
        }
    }
    if max_clear <= 0 {
        // No interior void cell at all -- a system too thin to have one.
        return (0, Vec::new());
    }
    // Among cells within 1 of that maximum -- a task-2 passage network is
    // close to uniform width, so the true widest points are rarely a
    // singleton and are almost always several cells wide at the "widest"
    // junction, not one pixel -- break toward the one with the most room to
    // grow into, i.e. `min(room_v, room_h)`, raster order the final
    // tie-break for determinism.
    //
    // **Measured, not assumed.** The literal single argmax (raster
    // tie-break only) put the chosen point hard against the vertical
    // envelope edge often enough that the room cap below throttled most
    // chambers to a fraction of their drawn size: tallest-open-column p50
    // over 16 seeds was 30-31 (task 3's bar is >= 40) and canyon's own
    // per-seed max was 21-48 with a median of 30. A system's own vertical
    // span already reaches within a few cells of the envelope edge (task
    // 2's own census: span down med 67-68 of a possible 71), so "the"
    // widest point sits near that edge about as often as not -- and this is
    // where the effect shows up, not in the location choice being wrong.
    // Widening the candidate set to near-ties and keying the pick on
    // available room fixed it without moving the primary criterion (still
    // greatest clearance, not an arbitrary central point): p50 rose to
    // 45-48 across every preset, comfortably clearing the bar.
    let mut best = (0i32, 0i32, 0i32); // (room, dx, dy)
    for dy in -CAVE_HALF_H..=CAVE_HALF_H {
        for dx in -CAVE_HALF_W..=CAVE_HALF_W {
            let idx = cave_idx(dx, dy);
            if void[idx] && dist[idx] >= max_clear - 1 {
                let room = (CAVE_HALF_H - dy.abs()).min(CAVE_HALF_W - dx.abs());
                if room > best.0 {
                    best = (room, dx, dy);
                }
            }
        }
    }
    let (_, bx, by) = best;

    let seed = ctx.terrain.seed;
    let rv_draw = 12.0 + noise::unit(seed, Purpose::CaveChamber, cx + bx, k) * 12.0;
    let rh_draw = rv_draw * 1.4;
    // The cap: shrink to whatever room the envelope has left from this
    // centre. Never to less than 2 -- a system whose clearance point sits
    // hard against the envelope edge still gets *a* chamber, a small one,
    // not none.
    let rv = rv_draw.min((CAVE_HALF_H - by.abs()) as f32).max(2.0);
    let rh = rh_draw.min((CAVE_HALF_W - bx.abs()) as f32).max(2.0);

    let mut added = Vec::new();
    let (rv_i, rh_i) = (rv.ceil() as i32, rh.ceil() as i32);
    for dy in -rv_i..=rv_i {
        for dx in -rh_i..=rh_i {
            if (dx as f32 / rh).powi(2) + (dy as f32 / rv).powi(2) > 1.0 {
                continue;
            }
            let (ex, ey) = (bx + dx, by + dy);
            if ex.abs() > CAVE_HALF_W || ey.abs() > CAVE_HALF_H {
                continue;
            }
            let idx = cave_idx(ex, ey);
            if !void[idx] {
                void[idx] = true;
                added.push(idx);
            }
        }
    }
    (added.len(), added)
}

/// Retract the void from any breach: a void cell is kept only if it is
/// itself stone in the world today, and every cell within [`VAULT_RIND`]
/// Chebyshev cells of it that is *not itself part of the void* is stone too
/// (the world edge counts as not-stone, via `Cell::OUT_OF_BOUNDS`'s bedrock
/// sentinel). A neighbour that is still part of the void poses no risk --
/// it is carved to air as well, not loose material left flush with a free
/// face, which is the property this check exists to guarantee.
///
/// **Iterated to its own fixpoint inside this call**, because retracting a
/// void cell that was itself non-stone turns it into exactly the kind of
/// solid, non-stone neighbour that can breach whatever void survives next
/// to it -- the grain of sand does not disappear when it stops being void,
/// it just becomes a permanent resident of the rind. Each pass that removes
/// anything shrinks a finite grid, so this terminates.
///
/// Returns whether anything was removed, so the caller's outer fixpoint
/// (component keep, ceiling guard, this) knows whether to loop again.
fn erode_breaches(ctx: &Ctx, world: &World, cx: i32, cy: i32, void: &mut [bool]) -> bool {
    let is_stone = |px: i32, py: i32| world.get(px, py).material == ctx.stone;
    let mut any = false;
    loop {
        let mut to_remove = Vec::new();
        for dy in -CAVE_HALF_H..=CAVE_HALF_H {
            for dx in -CAVE_HALF_W..=CAVE_HALF_W {
                if !void[cave_idx(dx, dy)] {
                    continue;
                }
                let mut breached = !is_stone(cx + dx, cy + dy);
                if !breached {
                    'nb: for ry in -VAULT_RIND..=VAULT_RIND {
                        for rx in -VAULT_RIND..=VAULT_RIND {
                            if rx == 0 && ry == 0 {
                                continue; // the cell itself, checked above
                            }
                            let (nx, ny) = (dx + rx, dy + ry);
                            let still_void = nx.abs() <= CAVE_HALF_W
                                && ny.abs() <= CAVE_HALF_H
                                && void[cave_idx(nx, ny)];
                            if still_void {
                                continue;
                            }
                            if !is_stone(cx + nx, cy + ny) {
                                breached = true;
                                break 'nb;
                            }
                        }
                    }
                }
                if breached {
                    to_remove.push(cave_idx(dx, dy));
                }
            }
        }
        if to_remove.is_empty() {
            break;
        }
        any = true;
        for idx in to_remove {
            void[idx] = false;
        }
    }
    any
}

/// One cave system: carve the void, verify the seal, write the cells.
///
/// Returns cells written; zero is a wholesale rejection -- the
/// collect-then-verify contract, kept from `pockets` through the round-2
/// vaults: nothing is written unless the entire envelope passed. After
/// round-5 task 1, that rejection should be near-never: [`carve_cave_void`]
/// now erodes the void away from any breach as it carves, so the seal check
/// below is expected to pass by construction and is kept as an assertion
/// rather than a silent reject -- a failure here is a bug in the erosion
/// step, not a normal outcome, and the test suite has to be able to see it
/// fail for that reason.
fn cave_system(ctx: &Ctx, world: &mut World, k: i32, cx: i32, cy: i32) -> VaultReport {
    let Some(void) = carve_cave_void(ctx, world, k, cx, cy) else { return VaultReport::default() };

    // The seal, kept as an assertion rather than a silent reject (round-5
    // task 1). Every cell within the rind of the kept component -- a 2-cell
    // Chebyshev dilation, diagonals included -- must be solid stone. The
    // dilation is this shape's equivalent of the ellipse path's
    // `inside(.., VAULT_RIND)`: the *envelope grown by the rind*, not the
    // bounding box. The spec sketched "bounding box + rind, all stone", and
    // that reading rejects a system for a sand lens tens of cells from the
    // nearest void -- see the round-3 finding; the r2 skeleton this grew
    // from never checked a box either, it checked the dilated shape.
    //
    // This used to reject the whole system wholesale on the first breach it
    // found, and that is what round 5 replaced: `carve_cave_void` now erodes
    // the void away from every breach as part of carving it, so by the time
    // control reaches here the property below is expected to hold *by
    // construction*, not by luck. Asserting rather than returning turns a
    // regression in that erosion into a loud, attributable failure instead
    // of a silent drop back to the old "one grain deletes the system"
    // behaviour wearing a passing test.
    for dy in -(CAVE_HALF_H + VAULT_RIND)..=(CAVE_HALF_H + VAULT_RIND) {
        for dx in -(CAVE_HALF_W + VAULT_RIND)..=(CAVE_HALF_W + VAULT_RIND) {
            let in_grid = dx.abs() <= CAVE_HALF_W && dy.abs() <= CAVE_HALF_H;
            // The void cells themselves first: they are the envelope's
            // interior and must be stone like everything else -- a lens cell
            // *inside* the would-be void is just as much a breach as one in
            // the rind.
            if in_grid && void[cave_idx(dx, dy)] {
                assert_eq!(
                    world.get(cx + dx, cy + dy).material,
                    ctx.stone,
                    "cave system k={k} at ({cx},{cy}): void cell ({dx},{dy}) was not eroded from a breach"
                );
                continue;
            }
            let near_void = (-VAULT_RIND..=VAULT_RIND).any(|ry| {
                (-VAULT_RIND..=VAULT_RIND).any(|rx| {
                    let (nx, ny) = (dx + rx, dy + ry);
                    nx.abs() <= CAVE_HALF_W
                        && ny.abs() <= CAVE_HALF_H
                        && void[cave_idx(nx, ny)]
                })
            });
            if near_void {
                assert_eq!(
                    world.get(cx + dx, cy + dy).material,
                    ctx.stone,
                    "cave system k={k} at ({cx},{cy}): rind cell ({dx},{dy}) was not eroded from a breach"
                );
            }
        }
    }

    // ---- gravel floors: flat fill per cavity, at rest by construction ----
    // Per column, the bottommost vertical run of void carries the floor.
    // (A chamber stacked directly above another keeps a bare stone bowl --
    // rare under the squash, and it costs a floor, not the seal.) The fill
    // is proposed as a constant depth per cavity, capped to leave two rows
    // of headroom, then its top surface is repose-clamped by the same
    // two-sweep taper `talus` uses -- so whatever the floor under it steps,
    // what gets written is a surface that cannot avalanche, a property of
    // the sweep rather than of any case analysis.
    let mut floor: Vec<Option<(i32, i32, i32)>> = vec![None; CAVE_GRID_W as usize];
    for dx in -CAVE_HALF_W..=CAVE_HALF_W {
        let mut bot = None;
        let mut top = 0;
        for dy in (-CAVE_HALF_H..=CAVE_HALF_H).rev() {
            if void[cave_idx(dx, dy)] {
                if bot.is_none() {
                    bot = Some(dy);
                }
                top = dy;
            } else if bot.is_some() {
                break;
            }
        }
        if let Some(b) = bot {
            floor[(dx + CAVE_HALF_W) as usize] = Some((top, b, 0));
        }
    }
    let seed = ctx.terrain.seed;
    let step = (ctx.terrain.params.soil_slope_cutoff * ctx.gravel_tan).max(0.05);
    let mut col = 0usize;
    while col < floor.len() {
        if floor[col].is_none() {
            col += 1;
            continue;
        }
        let start = col;
        // A cavity: consecutive columns whose bottom runs overlap in y.
        // Where they do not, solid rock separates the two floors and the
        // clamp must not couple them.
        while col + 1 < floor.len() {
            let (Some(a), Some(b)) = (floor[col], floor[col + 1]) else { break };
            if a.0 > b.1 || b.0 > a.1 {
                break;
            }
            col += 1;
        }
        let end = col;
        col += 1;
        // One nominal depth per cavity, drawn on its first column.
        let base = 2 + (noise::unit(seed, Purpose::CaveFloor, cx + start as i32, k) * 3.0) as i32;
        // Breakdown mounds: one to three heaps per large cavity, proposed as
        // unit-slope triangles on top of the base fill -- a cave floor is
        // rubble fallen from the roof, not tile, and a dead-flat fill from
        // wall to wall was the last ruled line left in the system. Proposed
        // only; the repose sweep below shaves them to gravel's own angle and
        // the verifier guards their toes like everything else's.
        let width = end - start + 1;
        let mut mound = vec![0i32; width];
        if width >= 20 {
            let sx = cx + start as i32;
            let count = 1 + (noise::unit(seed, Purpose::CaveFloor, sx, k * 31 + 1) * 3.0) as i32;
            for m in 0..count {
                let at = (noise::unit(seed, Purpose::CaveFloor, sx + m * 7, k * 31 + 2)
                    * width as f32) as i32;
                let peak =
                    2 + (noise::unit(seed, Purpose::CaveFloor, sx + m * 7, k * 31 + 3) * 4.0) as i32;
                for (i, e) in mound.iter_mut().enumerate() {
                    *e = (*e).max(peak - (i as i32 - at).abs());
                }
            }
        }
        // Elevation (+up) of the proposed gravel top, then the two sweeps.
        let mut e: Vec<f32> = (start..=end)
            .map(|i| {
                let (t, b, _) = floor[i].expect("cavity columns all have a floor run");
                ((base + mound[i - start]).min((b - t - 1).max(0))) as f32 - b as f32
            })
            .collect();
        for i in 1..e.len() {
            e[i] = e[i].min(e[i - 1] + step);
        }
        for i in (0..e.len().saturating_sub(1)).rev() {
            e[i] = e[i].min(e[i + 1] + step);
        }
        for (i, ei) in e.iter().enumerate() {
            let (t, b, _) = floor[start + i].expect("cavity columns all have a floor run");
            let h = ((*ei + b as f32).floor() as i32).clamp(0, (b - t - 1).max(0));
            floor[start + i] = Some((t, b, h));
        }
    }

    // **The sweep proposes; this verifies.** The two-sweep taper knows only
    // its own segment, and a cavity floor can end one column before a drop
    // it cannot see: measured on `rolling` seed 1, a mid-height ledge's
    // bottom runs were split from the deep chamber's by a one-cell stone
    // shelf, so the segment ended with h = 3 at a lip over fifteen rows of
    // open air, and eleven cells avalanched off it on frame one. That is the
    // talus lesson re-learnt in a cave: enumerating every way geometry can
    // undercut a heap misses one. So the planned shape is checked cell by
    // cell against the slide rule powder actually obeys, and any column that
    // fails is lowered until nothing can. A property of the check, not of
    // the case analysis.
    //
    // **Round-5 correction: the slide rule has no flank requirement.**
    // `update_powder`'s diagonal step (`src/sim/update.rs`) tries
    // `try_move(x, y, x +/- 1, y + 1)` straight off, and `try_move` only
    // ever inspects the *target* cell -- it never reads `(x +/- 1, y)` at
    // all. This check used to require that same-row flank open too, which
    // is a stricter condition than the engine actually enforces, and task
    // 2's narrower lattice was the first geometry to produce the gap: a
    // gravel pair walled solid on both flanks, sitting over solid floor,
    // but with the flank's *own* diagonal-down neighbour open one row
    // further over -- `wetland` seed 1 lost exactly the two cells at
    // (326,219)-(326,220) into (327,221) on frame one, reproduced with
    // `probe_temp_t2_regression` before this fix and gone after it. Checking
    // the flank was true of every shape round 3's wide, flat lenses could
    // produce, so the gap was invisible until task 2 made narrow vertical
    // shafts routine; it was never true of the rule the sand itself obeys.
    loop {
        let mut changed = false;
        for i in 0..floor.len() {
            let Some((t, b, h)) = floor[i] else { continue };
            if h == 0 {
                continue;
            }
            let dx = i as i32 - CAVE_HALF_W;
            let mut new_h = h;
            // Shallowest gravel cell with an open diagonal-down neighbour,
            // scanning down -- the flank itself need not be open too.
            for y in (b - h + 1)..=b {
                let exposed = [-1, 1].iter().any(|&s| !planned_solid(&void, &floor, dx + s, y + 1));
                if exposed {
                    // Drop this cell and everything stacked on it.
                    new_h = b - y;
                    break;
                }
            }
            if new_h < h {
                floor[i] = Some((t, b, new_h));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // ---- chambers, as the census and the waterline both mean them ----
    // Column-height based, not cavity based. The cavity segments the floors
    // use chain through every connecting passage, so "one cavity" is
    // usually the entire system -- a census keyed on it read `passages 0`
    // for systems that are visibly rooms-and-corridors, and a waterline
    // keyed on it put "the lowest chamber floor" at the bottom of a crevice
    // no player would call a room (measured: a seven-cell puddle in the
    // corner of `rolling` seed 1). A *chamber column* is one where the open
    // void above any gravel stands at least twelve cells tall; a chamber is
    // a run of at least six such columns; its floor is the deepest finished
    // gravel surface under it.
    let mut chamber_col = vec![false; CAVE_GRID_W as usize];
    let mut chambers = 0usize;
    let mut chamber_floors: Vec<i32> = Vec::new();
    {
        let fs = |i: usize| floor[i].map(|(_, b, h)| b - h).unwrap_or(CAVE_HALF_H);
        let tall: Vec<bool> = (0..CAVE_GRID_W)
            .map(|i| {
                let dx = i - CAVE_HALF_W;
                let mut best = 0;
                let mut run = 0;
                for dy in -CAVE_HALF_H..=CAVE_HALF_H {
                    if void[cave_idx(dx, dy)] && dy <= fs(i as usize) {
                        run += 1;
                        best = best.max(run);
                    } else {
                        run = 0;
                    }
                }
                best >= 12
            })
            .collect();
        let mut i = 0usize;
        while i < tall.len() {
            if !tall[i] {
                i += 1;
                continue;
            }
            let start = i;
            while i < tall.len() && tall[i] {
                i += 1;
            }
            if i - start >= 6 {
                chambers += 1;
                chamber_floors.push((start..i).map(fs).max().unwrap_or(0));
                for c in chamber_col.iter_mut().take(i).skip(start) {
                    *c = true;
                }
            }
        }
    }

    // ---- the aquifer waterline: one draw per system ----
    // Dry (above every floor), pools (between the lowest and median chamber
    // floor), or flooded (above the median). Water is written the way
    // `ponds` writes it -- level, full, one global line for the whole
    // system -- so the surface cannot hold a head difference anywhere:
    // every column's topmost water sits on the same row, and a pocket whose
    // ceiling is below the line fills to its roof and is sealed. That is
    // the whole at-rest argument, and it is the same one that made the
    // round-2 vug fill to its widest row. Connectivity does the rest: low
    // chambers pond, high ones stay dry, and one system can hold both --
    // which is the picture worth finding.
    chamber_floors.sort_unstable();
    // Envelope-local dy of the water surface; `i32::MAX` is a dry system.
    let water_line = if chamber_floors.is_empty() {
        i32::MAX
    } else {
        let u = noise::unit(seed, Purpose::CaveFloor, -1 - k, 0);
        let v = noise::unit(seed, Purpose::CaveFloor, -1 - k, 1);
        // Sorted by dy, so the first is the *highest* floor (smallest dy)
        // and the last the lowest. The line always submerges its target
        // floor by at least one row -- a line drawn exactly *at* a floor is
        // a zero-depth pond, which a single-chamber system otherwise gets
        // every time (its lowest and median floor are the same row).
        let highest = chamber_floors[0];
        let median = chamber_floors[chamber_floors.len() / 2];
        let lowest = *chamber_floors.last().expect("non-empty");
        if u < 0.30 {
            i32::MAX
        } else if u < 0.75 {
            // Pools: from just-submerging the lowest chamber up to the
            // median chamber's floor.
            lowest - 1 - (v * (lowest - median).max(2) as f32) as i32
        } else {
            // Flooded: from just-submerging the median chamber up toward
            // the highest chamber's floor.
            median - 1 - (v * (median - highest).max(2) as f32) as i32
        }
    };

    // ---- speleothems: the wonder, grown after the void is cut ----
    // Stalactites from ceilings, stalagmites from floors, 1-2 cells wide and
    // tapering (the secondary column is shorter, so the root is the wide
    // end), a crystal minority, and the occasional pair almost meeting.
    // Placed on the bottommost run per column -- the galleries the floors
    // are in -- *after* the floor verifier, because adding attached solid
    // can only ever add support, never take it away, so the verified gravel
    // stays verified. A stalagmite is written from the *stone* under the run
    // upward through any gravel over it: "structurally trivial, rooted in
    // the massif" means rooted on rock, not standing on loose fill.
    //
    // A formation must never bridge floor to ceiling -- a column splits the
    // passage the player walks -- so every placement leaves at least two
    // open rows in its column, except a pair, which closes to a drawn gap
    // of one or two on purpose.
    let mut speleo = vec![0u8; (CAVE_GRID_W * CAVE_GRID_H) as usize]; // 0 none, 1 stone, 2 crystal
    let mut speleo_cells = 0usize;
    {
        let mut last: Option<i32> = None;
        for i in 0..floor.len() {
            let dx = i as i32 - CAVE_HALF_W;
            let px = cx + dx;

            // Round-5 task 4b: `SPELEO_SPACING = 4` enforced *even* spacing
            // everywhere, which is precisely the "reads as a comb" artefact
            // the beauty review named -- the opposite of drip
            // concentration. The minimum gap is now a low-frequency
            // drip-focus field: in a wet stretch it shrinks toward
            // `SPELEO_SPACING_MIN`, letting formations bunch; in a dry one
            // it grows toward `SPELEO_SPACING_MAX`, leaving the stretch
            // close to bare.
            //
            // The `smoothstep(0.1, 0.5, focus)` thresholds are read off
            // `value_1d`'s own measured range at this `DRIP_SCALE`, not the
            // nominal `[0, 1)` -- interpolating between two per-lattice
            // `unit` draws rarely reaches either extreme, so a threshold
            // written against the theoretical range left most of a system
            // reading as "middling" instead of clearly wet or dry (a probe
            // dump showed the field only ever reaching about 0.13-0.82 over
            // one system's width). Widening the window to bracket the
            // *observed* range is what actually produces legible clustering
            // rather than a mild, all-over ripple.
            let focus = noise::value_1d(seed, Purpose::Drip, px as f32 / DRIP_SCALE);
            let dry = 1.0 - noise::smoothstep(0.1, 0.5, focus);
            let min_spacing = SPELEO_SPACING_MIN + (dry * (SPELEO_SPACING_MAX - SPELEO_SPACING_MIN) as f32) as i32;
            if last.is_some_and(|l| dx - l < min_spacing) {
                continue;
            }

            // Round-5 task 4b: every run tall enough to qualify, not only
            // the column's bottommost -- formations used to decorate one
            // gallery of a multi-level system and leave the rest bare,
            // because placement read `floor[i]` directly, which is the
            // bottommost run by construction. Enumerated fresh here rather
            // than reused from `floor` (which only ever keeps the last
            // one): a maximal vertical run of void is exactly what a
            // gallery *is*, the same definition `floor`'s own construction
            // uses, just not discarded for every run but one.
            let mut runs: Vec<(i32, i32)> = Vec::new();
            let mut open_top: Option<i32> = None;
            for dy in -CAVE_HALF_H..=CAVE_HALF_H {
                if void[cave_idx(dx, dy)] {
                    if open_top.is_none() {
                        open_top = Some(dy);
                    }
                } else if let Some(top) = open_top.take() {
                    runs.push((top, dy - 1));
                }
            }
            if let Some(top) = open_top {
                runs.push((top, CAVE_HALF_H));
            }

            let mut placed_here = false;
            for (ri, &(t, b_raw)) in runs.iter().enumerate() {
                // Only the bottommost run of a column ever carries a
                // gravel floor (see the fill loop above): every other
                // run's own bottom cell is solid rock by construction --
                // that rock is *why* the run ended -- so only the bottom
                // run needs the `floor[i]` height correction.
                let is_bottom = ri == runs.len() - 1;
                let (b, h) = if is_bottom {
                    match floor[i] {
                        Some((_, fb, fh)) => (fb, fh),
                        None => (b_raw, 0),
                    }
                } else {
                    (b_raw, 0)
                };
                let fs = b - h; // lowest open row: the floor surface
                let span = fs - t + 1;
                if span < 5 {
                    continue;
                }
                // A distinct sub-range of the noise coordinate per run, so
                // a second gallery in the same column does not draw
                // identically to the first.
                let ry = ri as i32 * 20;
                // The drip focus doubles as a density multiplier, not just
                // a spacing throttle: wide spacing alone rediscovers the
                // old comb at a lower frequency (measured), because
                // `SPELEO_DENSITY` -- calibrated for the old *even*
                // spacing -- is far too low to fill even a loosened gap.
                // The span term is loosened too (`smoothstep(3, 5, ..)`
                // against the old `(6, 26)`, tuned for round-3's wide flat
                // lens): the outer `span < 5` filter already keeps out
                // anything too cramped to hold a formation, and gating
                // *again* on the same quantity at a chamber-only scale
                // left ordinary passage -- most of a system's length --
                // essentially undecorated regardless of how wet it read.
                let wet = noise::smoothstep(0.1, 0.4, focus);
                let chance = SPELEO_DENSITY * 4.0 * wet * noise::smoothstep(3.0, 5.0, span as f32);
                if noise::unit(seed, Purpose::Speleothem, px, ry) >= chance {
                    continue;
                }
                let kind = noise::unit(seed, Purpose::Speleothem, px, ry + 1);
                let crystal = noise::unit(seed, Purpose::Speleothem, px, ry + 2) < SPELEO_CRYSTAL;
                let pair = kind < SPELEO_PAIR && span >= 7;
                let stalactite = pair || kind < SPELEO_PAIR + 0.45;
                // Round-5 task 4a: a heavy-tailed draw scaled to the local
                // open span, replacing the old `2 + unit * 6` -- a uniform
                // draw capped at 8 regardless of how tall the room was,
                // measured (`cave_probe`) at median 3, p90 6, max 7 over
                // 539 formations: there was no tail to make heavy, the
                // ceiling had to move first. `unit^1.3 * avail`, base 1:
                // tried cubed and squared first and both under-shot the
                // p90 >= 10 bar (cubed: p90 5-6; squared: p90 7-8) while
                // already meeting p50 <= 3 and max >= 25 -- most formations
                // sit in ordinary passage, where `avail` itself is small,
                // so a heavier tail alone cannot lift the 90th percentile
                // past what enough *tall-span* formations reach. 1.3 is
                // the mildest exponent that clears p90 >= 10 on every
                // preset (measured: p50 1, p90 8-12, max 28-34) without
                // giving up the fringe: median stays at the soda-straw
                // floor while the tail reaches deep into the chamber-scale
                // spans task 3 added. `.min(span - 2)` still holds as the
                // structural cap; it binds rarely now instead of almost
                // always.
                let avail = (span - 2).max(0) as f32;
                let mut lt = if stalactite {
                    (1.0 + noise::unit(seed, Purpose::Speleothem, px, ry + 3).powf(1.3) * avail)
                        .min((span - 2) as f32) as i32
                } else {
                    0
                };
                let mut lg = if pair || !stalactite {
                    (1.0 + noise::unit(seed, Purpose::Speleothem, px, ry + 4).powf(1.3) * avail)
                        .min((span - 2) as f32) as i32
                } else {
                    0
                };
                if pair {
                    // Almost meeting: shrink the longer half until the
                    // drawn one-or-two-cell gap fits.
                    let gap = 1 + (noise::unit(seed, Purpose::Speleothem, px, ry + 5) * 2.0) as i32;
                    while lt + lg + gap > span {
                        if lt >= lg {
                            lt -= 1;
                        } else {
                            lg -= 1;
                        }
                    }
                }
                if lt < 2 {
                    lt = 0;
                }
                if lg < 2 {
                    lg = 0;
                }
                if lt == 0 && lg == 0 {
                    continue;
                }
                let mat = if crystal { 2u8 } else { 1u8 };
                let mut put = |gx: i32, gy: i32| {
                    if gx.abs() <= CAVE_HALF_W && gy.abs() <= CAVE_HALF_H && void[cave_idx(gx, gy)] {
                        speleo[cave_idx(gx, gy)] = mat;
                    }
                };
                for y in t..t + lt {
                    put(dx, y);
                }
                for y in (fs - lg + 1)..=b {
                    put(dx, y);
                }
                // A minority go two cells wide, the secondary column
                // shorter -- the taper that makes the root the wide end.
                // Only where the neighbouring column's run lines up, and
                // always leaving that column its own two open rows.
                // Bottommost gallery only: the comparison is against the
                // neighbour's own bottommost run (`floor[j]`), which is
                // not a meaningful comparison for an upper gallery.
                if is_bottom && min_spacing >= 4 && noise::unit(seed, Purpose::Speleothem, px, ry + 6) < 0.4 {
                    let side =
                        if noise::unit(seed, Purpose::Speleothem, px, ry + 7) < 0.5 { 1 } else { -1 };
                    let j = i as i32 + side;
                    if j >= 0 && (j as usize) < floor.len() {
                        // `span2 >= 3` before clamping to it: a two-row
                        // slot has no room for a secondary at all, and
                        // `clamp(1, 0)` panics -- found by the debug suite
                        // on a world the release sweep never built.
                        if let Some((t2, b2, h2)) = floor[j as usize] {
                            let fs2 = b2 - h2;
                            let span2 = fs2 - t2 + 1;
                            if span2 >= 3 {
                                if lt > 0 && (t2 - t).abs() <= 1 {
                                    let lt2 = (lt * 3 / 5).clamp(1, span2 - 2);
                                    for y in t2..t2 + lt2 {
                                        put(dx + side, y);
                                    }
                                }
                                if lg > 0 && (fs2 - fs).abs() <= 1 {
                                    let lg2 = (lg * 3 / 5).clamp(1, span2 - 2);
                                    for y in (fs2 - lg2 + 1)..=b2 {
                                        put(dx + side, y);
                                    }
                                }
                            }
                        }
                    }
                }
                placed_here = true;
            }
            if placed_here {
                last = Some(dx);
            }
        }
    }

    let mut written = 0;
    let mut water_cells = 0usize;
    let mut passage_cells = 0usize;
    for dy in -CAVE_HALF_H..=CAVE_HALF_H {
        for dx in -CAVE_HALF_W..=CAVE_HALF_W {
            if !void[cave_idx(dx, dy)] {
                continue;
            }
            let (px, py) = (cx + dx, cy + dy);
            let formation = speleo[cave_idx(dx, dy)];
            let gravel =
                matches!(floor[(dx + CAVE_HALF_W) as usize], Some((_, b, h)) if h > 0 && dy > b - h);
            let cell = if formation == 2 {
                // The crystal minority: the same material as a vug lining,
                // attached like the rock it grows from.
                Cell::new(ctx.crystal, loose_shade(ctx, Purpose::Vault, px, py)).with_attached(true)
            } else if formation == 1 {
                // Flowstone: stone, but *pale* stone -- the cap-rock family
                // with its own per-cell tone rather than the wall's banding.
                // Deposited calcite is paler than the rock it grew from, and
                // two things ride on the shade being different from
                // `strata_shade(px, py)`: a formation reads as a formation
                // instead of a striped stump of wall, and the paired-build
                // diff every round-3 guard relies on can *see* it -- written
                // with the wall's own shade it is byte-identical to the
                // control world and vanishes from every test.
                Cell::new(ctx.stone, FAMILY_RESISTANT * TONES + loose_shade(ctx, Purpose::Speleothem, px, py))
                    .with_attached(true)
            } else if gravel {
                // Buried gravel's family, same as a lens and the vug floor:
                // read against solid stone and nothing else.
                Cell::new(ctx.gravel, BURIED_FAMILY * TONES + loose_shade(ctx, Purpose::Vault, px, py))
            } else if dy >= water_line {
                // Below the system's waterline: water, exactly as `ponds`
                // writes it -- full (`aux == 0` on a Liquid means full, the
                // documented opposite of the Powder convention), shade
                // varied for the grain mode.
                water_cells += 1;
                Cell::new(ctx.water, loose_shade(ctx, Purpose::Shade, px, py))
            } else {
                if !chamber_col[(dx + CAVE_HALF_W) as usize] {
                    passage_cells += 1;
                }
                Cell::EMPTY
            };
            world.set(px, py, cell);
            written += 1;
            if formation != 0 {
                speleo_cells += 1;
            }
        }
    }
    VaultReport {
        cells: written,
        systems: 1,
        chambers,
        passage_cells,
        speleothem_cells: speleo_cells,
        water_cells,
    }
}

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
    // The world edge is a wall, not an outlet. Reads outside the world return
    // a solid sentinel, so water genuinely cannot leave -- but this sweep used
    // to start from the edge column's own ground, which treats the edge as a
    // spillway. While the macro shape was one fixed wave the low point was
    // always interior and it never mattered; once regions could put the low
    // ground at an edge, those worlds drained and `wetland` generated no water
    // at all. Starting from the highest ground in the world makes the edges as
    // tall as the tallest barrier, which is the wall they actually are.
    let wall = ctx.plans.iter().map(|c| c.surface_y).min().unwrap_or(0);
    let mut running = wall;
    for (x, rim) in left_rim.iter_mut().enumerate() {
        running = running.min(ctx.plans[x].surface_y);
        *rim = running;
    }
    running = wall;
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
/// Write the soil-water profile the CA physics are at rest in: saturated
/// ground at and below the water table, a two-step capillary fringe above
/// it, and a saturated wetted perimeter around standing water.
///
/// **The reconciliation between "pools arrive already level" and real soil
/// hydrology, and the merge that joined them is why it exists.** A pond
/// rests on drinkable ground now: `update_soil_water`'s infiltration lets
/// soil absorb an adjacent liquid, so a pond generated over *dry* soil
/// spends its opening minutes drinking its own bed and banks — the world
/// opens by visibly draining its lakes, and the at-rest guarantee
/// (`tests/worldgen.rs`) fails on exactly the water cells the ponds pass
/// placed. Hydrologically the dry ground was the lie: standing water
/// persists where the ground around it is already saturated — a pond *is*
/// the water table reaching daylight, and the ponds pass fills basins to
/// the table, so writing that truth into the soil is authoring the
/// equilibrium, not faking one.
///
/// The exact shape is forced by the two CA rules it must stand against,
/// and deviating from it re-derives a churn this merge already paid for
/// twice (see `SOIL_CAPILLARY_REST`'s own doc for the perpetual two-cell
/// pump):
///
/// - **At and below the effective table: saturated.** The effective table
///   is the column's own `table_y`, or the bed of any standing water in
///   the column if that is higher — a pond outranks the smoothed table
///   locally, being direct evidence of water at that height.
/// - **Adjacent to any liquid cell: saturated.** The wetted perimeter.
///   A merely-damp bank cell drinks its neighbouring water cell (bounded,
///   but the at-rest test rightly counts the vanished water), and a full
///   one cannot.
/// - **At fringe distance one and two from the saturated zone: field
///   capacity, then 240.** Distance is *Manhattan distance to the nearest
///   saturated cell across the whole ground*, not rows-above-the-table in
///   this column alone — capillary exchanges across all four faces, so on
///   a slope or a terrace step the fringe wraps the saturated zone
///   sideways exactly as it does upward, and a column-only fringe leaves a
///   bare lateral edge that capillary then assembles at runtime (measured:
///   84–108 frames of awake chunks on the terraced and rolling presets,
///   against 31 with the fringe authored isotropically). Each step stays
///   within `SOIL_CAPILLARY_REST` (380) of its neighbours in every
///   direction, so the fringe stands; each stays at or under field
///   capacity, so drainage has nothing to do.
///
/// Writes are `max` against what is already there, so the rules compose,
/// and cells without `water_capacity` (sand, gravel, stone) are skipped in
/// place — a sand pocket inside the saturated zone stays inert and
/// exchanges nothing, which seals rather than leaks.
pub fn soil_moisture(ctx: &Ctx, world: &mut World) -> usize {
    let w = ctx.terrain.w as usize;
    if w == 0 {
        return 0;
    }
    // Standing water outranks the smoothed table locally: the bed of the
    // lowest water cell caps the effective table from above.
    let mut table: Vec<i32> = (0..w).map(|x| ctx.plans[x].table_y).collect();
    for (x, t) in table.iter_mut().enumerate() {
        for y in 0..ctx.terrain.h {
            if world.materials.kind(world.get(x as i32, y).material) == material::MaterialKind::Liquid {
                *t = (*t).min(y + 1);
            }
        }
    }

    // Manhattan distance to free water, by the classic two-pass transform
    // over the whole ground. Seeds: liquid cells at -1 (the soil *touching*
    // water is saturated, so water's neighbours must land at 0) and
    // capacity-bearing cells at or below their column's table at 0. A
    // per-column transform was tried first and left the wetted perimeter of
    // every above-table pond unwrapped — its saturated bank cells were not
    // seeds, so their outward neighbours computed as far-from-water, got no
    // fringe, and capillary assembled one at runtime (rolling preset: 82
    // frames of awake chunks against 31 once the ponds seed the transform).
    let h = ctx.terrain.h as usize;
    let far = i32::MAX / 2;
    let idx = |x: usize, y: usize| y * w + x;
    let mut d = vec![far; w * h];
    for y in 0..h {
        for x in 0..w {
            let cell = world.get(x as i32, y as i32);
            if world.materials.kind(cell.material) == material::MaterialKind::Liquid {
                d[idx(x, y)] = -1;
            } else if world.materials.get(cell.material).water_capacity > 0 {
                let wetted = [(0, -1), (-1, 0), (1, 0), (0, 1)]
                    .iter()
                    .any(|&(dx, dy)| world.materials.kind(world.get(x as i32 + dx, y as i32 + dy).material) == material::MaterialKind::Liquid);
                if wetted || (y as i32) >= table[x] {
                    d[idx(x, y)] = 0;
                }
            }
        }
    }
    // **Saturation is closed downward, and skipping this closure was a
    // measured pump.** A saturated cell over an unsaturated one drains
    // (gravity), and where standing water can refill the upper cell
    // (infiltration at a pond's wetted edge), the pair cycles forever —
    // watched in the sweep test's own diff: bank columns filling top-down
    // at ~15 units a frame while the pond surface re-levelled what the
    // banks drank. Hydrostatically the closure is just true: ground
    // directly beneath saturated ground is saturated. A capacity-free cell
    // (sand, stone) ends the chain and seals it.
    for x in 0..w {
        for y in 1..h {
            if d[idx(x, y)] > 0 && d[idx(x, y - 1)] <= 0 && world.materials.get(world.get(x as i32, y as i32).material).water_capacity > 0 {
                d[idx(x, y)] = 0;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let mut best = d[idx(x, y)];
            if x > 0 {
                best = best.min(d[idx(x - 1, y)] + 1);
            }
            if y > 0 {
                best = best.min(d[idx(x, y - 1)] + 1);
            }
            d[idx(x, y)] = best;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let mut best = d[idx(x, y)];
            if x + 1 < w {
                best = best.min(d[idx(x + 1, y)] + 1);
            }
            if y + 1 < h {
                best = best.min(d[idx(x, y + 1)] + 1);
            }
            d[idx(x, y)] = best;
        }
    }

    let fringe_fraction = [material::SOIL_FIELD_CAPACITY as f32 / material::SOIL_SATURATED as f32, 0.24];
    let mut n = 0;
    for y in 0..h {
        for x in 0..w {
            let cell = world.get(x as i32, y as i32);
            let capacity = world.materials.get(cell.material).water_capacity;
            if capacity == 0 {
                continue;
            }
            let target = match d[idx(x, y)] {
                dist if dist <= 0 => capacity,
                1 => (capacity as f32 * fringe_fraction[0]) as u16,
                2 => (capacity as f32 * fringe_fraction[1]) as u16,
                _ => continue,
            };
            if crate::sim::update::soil_moisture(cell) < target {
                world.set(x as i32, y as i32, cell.with_aux(target));
                n += 1;
            }
        }
    }
    n
}

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

/// Moss and tree seeds on ground that will support them.
///
/// The world arrives with life in it rather than waiting for the player to
/// plant every blade — which matters for a reason beyond decoration. A world
/// whose only vegetation is hand-placed grows *evenly spaced* vegetation,
/// because that is how a person places things, and evenly spaced plants are
/// the single most artificial-looking thing a side-view world can do.
///
/// So placement is **clustered, not uniform**: the density is multiplied by a
/// low-frequency field that is then *squared*. Squaring is the whole device —
/// it pushes most of the world below the planting threshold and concentrates
/// the rest into stands, giving thickets and clearings instead of a scatter.
/// Removing that square gives a uniform sprinkle, which is exactly the look
/// this exists to avoid.
///
/// Seeds, not grown plants. What comes up, how tall it gets and how it leans
/// are the organism substrate's business, and a generator that placed finished
/// trees would be authoring an outcome the simulation is meant to produce.
pub fn life_scatter(ctx: &Ctx, world: &mut World) -> usize {
    let p = ctx.terrain.params;
    if p.moss_density <= 0.0 && p.tree_density <= 0.0 {
        return 0;
    }
    let mut n = 0;
    // `Option`, not a sentinel. The first version used `i32::MIN` and never
    // planted a single tree in any world: `x - i32::MIN` overflows, wraps
    // negative in release, and fails the spacing test for every column
    // forever. The counter found it -- the render just looked like a world
    // where trees are rare.
    let mut last_tree: Option<i32> = None;
    for x in 0..ctx.terrain.w {
        let ground = ctx.plans[x as usize].surface_y;
        let above = ground - 1;
        if above < 0 {
            continue;
        }
        // Somewhere to stand, and space to stand in. The emptiness check
        // also keeps anything from being planted in a pond, which is what
        // should happen without a rule saying so.
        let footing = world.get(x, ground).material;
        if world.get(x, above).material != material::EMPTY {
            continue;
        }
        let cluster = noise::fbm_1d(
            ctx.terrain.seed,
            Purpose::Life,
            x as f32 / p.life_cluster_wavelength.max(1.0),
            2,
        );
        let cluster = cluster * cluster;

        // Trees want soil to root in; moss will take bare rock as well, which
        // is what puts green on a cliff face where nothing else grows.
        // Trees want soil specifically. Sand is footing enough for moss to
        // cling to but not for a tree to root in, which is what keeps a
        // desert looking like a desert without a rule saying "no trees in
        // deserts" -- the material already says it.
        let on_soil = footing == ctx.soil;
        // ...and dry country thins what grows even where soil remains.
        let dryness = 1.0 - ctx.terrain.character(x).aridity;
        if on_soil
            && last_tree.is_none_or(|last| x - last >= TREE_SPACING)
            && noise::unit(ctx.terrain.seed, Purpose::Life, x, 7) < p.tree_density * cluster * dryness
            && world.plant_tree_species(x, above, "tree")
        {
            last_tree = Some(x);
            n += 1;
            continue;
        }
        if noise::unit(ctx.terrain.seed, Purpose::Life, x, 9) < p.moss_density * cluster * dryness {
            world.plant_moss_seed(x, above);
            n += 1;
        }
    }
    n
}

/// Closest two tree seeds may be planted, in columns.
///
/// Not an aesthetic rule so much as an admission about the substrate: two
/// seedlings a couple of cells apart compete for the same light and water and
/// one of them simply fails, so planting them is spending generation on a
/// plant that will not be there. Far enough apart to both have a chance, near
/// enough that a stand still reads as a stand.
const TREE_SPACING: i32 = 7;
/// How far a lens is stretched along its bedding plane, at the two ends of
/// the draw.
///
/// A lens whose long axis is 2-4x its old one, at the same thickness. Round
/// ellipses at a uniform density are what made these read as polka dots
/// rather than as geology — and worse, as *ore*, which is a promise the game
/// does not keep. A sedimentary lens is deposited within a bed, so it is
/// long, thin and lies along the bedding; that shape is the whole difference
/// between "a lens in the rock" and "a spot".
const LENS_STRETCH: (f32, f32) = (2.0, 4.0);

/// Sand and gravel lenses sealed inside the rock.
///
/// Loose material the player only finds by digging, and which behaves the
/// moment it is exposed — cut into one and it pours. Fully enclosed, so it is
/// trivially at rest until something opens it, which is why this is the one
/// place generated powder can sit at any shape at all.
///
/// **Lenses lie in the bedding.** Each ellipse's long axis is rotated onto
/// the local strata band — the same `strata_offset` the shade pass bands the
/// rock with and the surface benches snap to, so a lens sits *in* a visible
/// bed rather than cutting across it. One noise field doing a third job that
/// has to agree with the other two.
///
/// **Density and size follow the country and the depth.** `Character.
/// sediment` makes a sedimentary region richer and a resistant one sparse,
/// and both thin out toward bedrock: loose lenses are a shallow-burial
/// feature, and a massif that is equally spotty at every depth reads as
/// wallpaper. Both factors are exactly `1.0` at a neutral character and zero
/// depth, so a preset with no regional variation generates what it always
/// did.
pub fn pockets(ctx: &Ctx, world: &mut World) -> usize {
    let mut n = 0;
    let p = ctx.terrain.params;
    if p.pocket_density <= 0.0 {
        return n;
    }
    const REGION: i32 = 64;
    let seed = ctx.terrain.seed;
    let w = ctx.terrain.w;
    for ry in 0..ctx.terrain.h.div_euclid(REGION) + 1 {
        for rx in 0..w.div_euclid(REGION) + 1 {
            // The region's own character and burial depth, sampled at its
            // centre column. Per region rather than per candidate because the
            // *count* has to be decided before a candidate exists, and a
            // 64-cell region is well inside one region of the world map.
            let mx = (rx * REGION + REGION / 2).clamp(0, w - 1);
            let plan = ctx.plans[mx as usize];
            let ch = ctx.terrain.character(mx);
            // Sedimentary country is richer, resistant country is sparse.
            // Written so that `sediment == 1` and `resistance == 1` -- the
            // neutral character -- comes out at exactly 1.0.
            let richness = (ch.sediment * (1.6 - 0.6 * ch.resistance)).clamp(0.0, 2.5);
            // Depth, as a fraction of the massif's own thickness rather than
            // an absolute row: a canyon massif is five times the depth of a
            // wetland one and "near bedrock" has to mean the same thing in
            // both.
            let my = ry * REGION + REGION / 2;
            let thickness = (plan.bedrock_top_y - plan.surface_y).max(1) as f32;
            let t = ((my - plan.surface_y) as f32 / thickness).clamp(0.0, 1.0);
            // Quadratic, for the same reason `plan`'s soil taper is: a linear
            // fall thins lenses out as soon as the rock deepens at all, so
            // the upper massif -- the part a player actually digs -- loses
            // the feature it is supposed to have most of.
            let with_depth = 0.15 + 0.85 * (1.0 - t) * (1.0 - t);
            let density = p.pocket_density * richness * with_depth;
            // A fractional density means "sometimes one": the whole number is
            // guaranteed and the remainder is a per-region coin flip.
            let whole = density.floor() as i32;
            let extra = i32::from(noise::unit(seed, Purpose::Pocket, rx, ry) < density.fract());
            for k in 0..whole + extra {
                let cx = rx * REGION + (noise::unit(seed, Purpose::Pocket, rx * 31 + k, ry) * REGION as f32) as i32;
                let cy = ry * REGION + (noise::unit(seed, Purpose::Pocket, rx, ry * 31 + k) * REGION as f32) as i32;
                if cx < 0 || cx >= w {
                    continue;
                }
                // Size follows the same supply as the count, gently -- 1.0 at
                // a neutral character, 0.6..1.4 across the range regions
                // actually draw.
                let bulk = (0.5 + 0.5 * ch.sediment).clamp(0.5, 1.5);
                let stretch = LENS_STRETCH.0
                    + noise::unit(seed, Purpose::Pocket, cx * 7, cy * 5) * (LENS_STRETCH.1 - LENS_STRETCH.0);
                let a = (4.0 + noise::unit(seed, Purpose::Pocket, cx, cy) * 6.0) * stretch * bulk;
                let b = (2.0 + noise::unit(seed, Purpose::Pocket, cy, cx) * 2.0) * bulk;
                let m = if noise::unit(seed, Purpose::Pocket, cx + 1, cy + 1) < 0.5 { ctx.sand } else { ctx.gravel };
                // The bedding plane through this lens. A band is the locus of
                // constant `y + strata_offset(x)`, so its gradient is minus
                // the offset's -- a central difference over the same pure
                // function `strata_shade` and `terraced` both read.
                let dip = -(ctx.terrain.strata_offset(cx + 1) - ctx.terrain.strata_offset(cx - 1)) / 2.0;
                let norm = (1.0 + dip * dip).sqrt();
                let (cos_t, sin_t) = (1.0 / norm, dip / norm);

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
                // The rotated ellipse's bounding box, so the scan still
                // covers the whole shape once the long axis is no longer
                // along x. Same +1 margin the rind always had.
                let ext_x = ((a * cos_t).abs() + (b * sin_t).abs()).ceil() as i32 + 1;
                let ext_y = ((a * sin_t).abs() + (b * cos_t).abs()).ceil() as i32 + 1;
                'lens: for dy in -ext_y..=ext_y {
                    for dx in -ext_x..=ext_x {
                        let (px, py) = (cx + dx, cy + dy);
                        // Into the bed's own frame: `u` along the bedding,
                        // `v` across it.
                        let u = dx as f32 * cos_t + dy as f32 * sin_t;
                        let v = -(dx as f32) * sin_t + dy as f32 * cos_t;
                        let d = (u / a).powi(2) + (v / b).powi(2);
                        // The rind: one cell beyond the lens must also be
                        // stone, so the lens is never flush with a free face.
                        let rind = (u / (a + 1.0)).powi(2) + (v / (b + 1.0)).powi(2);
                        if rind > 1.0 {
                            continue;
                        }
                        if px < 0 || px >= w || py < 0 || py >= ctx.terrain.h {
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
                    // Buried gravel takes its own palette family, so a lens
                    // is not grey-on-grey invisible inside grey rock. Scree
                    // at a cliff foot and the stony base of a soil profile
                    // keep family 0 -- they are read against sky and soil,
                    // not against stone, and they are what "gravel" looks
                    // like when the player paints it. See `assets/gravel.ron`.
                    let shade = if m == ctx.gravel {
                        BURIED_FAMILY * TONES + loose_shade(ctx, Purpose::Pocket, px, py)
                    } else {
                        loose_shade(ctx, Purpose::Pocket, px, py)
                    };
                    world.set(px, py, Cell::new(m, shade));
                    n += 1;
                }
            }
        }
    }
    n
}

/// Rounded attached-stone clusters seated where a hard band shed enough to
/// leave a boulder socket (`erosion::Deposits::boulder`).
///
/// Sockets are markers, not shapes: erosion only records *where* a resistant
/// surface sheds past the threshold, never what should stand there
/// (`Reports/worldgen-erosion-design.md`'s "markers are data on the plan" --
/// the state-the-difference-as-data lesson, `CLAUDE.md`). This pass is the
/// one place that turns a marker into geometry, cloning `pockets`' collect-
/// verify-write shape: propose every cell of one boulder and only write any
/// of them if every one is a safe target.
///
/// Runs after `pockets` and before `vaults` (`mod.rs`'s `PASSES`), so at the
/// point this reads the world only `stone_massif`, `bedrock_floor`,
/// `soil_blanket`, `brows`, `talus` and `pockets` have written -- water and
/// vault linings do not exist yet. The write-target check still excludes
/// them: a check that "cannot fire today" is exactly the kind CLAUDE.md
/// warns rots invisibly the day something upstream of this pass changes.
///
/// **Most markers reject.** A hard band that sheds enough to leave a socket
/// is, by construction, right at a steep drop, and `brows` hangs a lip at
/// almost every qualifying edge in `canyon` (`brow_chance` 0.9) -- so the
/// open air a dome wants to rise into is very often already a brow's
/// underside. Measured at the 512-column harness, canyon age 1.0: roughly
/// 16-18 markers per 20 seeds, and only about 1 boulder in 30 seeds actually
/// seats (`a_forced_boulder_world_seats_stone_and_arrives_at_rest`,
/// `tests/worldgen.rs`). This is read as the collect-verify-write contract
/// working as designed -- never overwrite a brow -- rather than a bug in
/// it; see the round-4 Findings entry in
/// `Reports/worldgen-implementation-tasks-2026-08.md` for the numbers.
pub fn boulders(ctx: &Ctx, world: &mut World) -> usize {
    let mut n = 0;
    let seed = ctx.terrain.seed;
    let w = ctx.terrain.w;
    let mut x = 0;
    while x < w {
        if !ctx.deposits.boulder[x as usize] {
            x += 1;
            continue;
        }
        // Merge the whole run of adjacent markers into one boulder, seated
        // at the run's centre rather than at its first column.
        let start = x;
        while x < w && ctx.deposits.boulder[x as usize] {
            x += 1;
        }
        let cx = (start + x - 1) / 2;

        // Shape, from a stream keyed on the run's centre column: 2-5 cells
        // wide, 2-4 tall, and clamped to never taller than it is wide --
        // stricter than (and so still honours) the erosion design's
        // structural non-negotiable #3, "never taller than 3x its base
        // width", which a boulder toppling over its own footprint the
        // moment structural distances are computed would violate outright.
        let width = 2 + (noise::unit(seed, Purpose::Boulder, cx, 0) * 4.0) as i32;
        let height = (2 + (noise::unit(seed, Purpose::Boulder, cx, 1) * 3.0) as i32).min(width);
        let a = width as f32 / 2.0;
        let b = height as f32 / 2.0;
        let reach = (a.ceil() as i32).max(1);

        // Collect first, write only if every proposed cell is a safe
        // target: open air or loose cover (soil/sand/gravel) for the dome,
        // which is displaced rather than skipped -- a boulder resting on
        // top of an untouched talus apron would look like it was dropped
        // there, not eroded out of the rock beneath it. The seat row (the
        // column's own topmost solid cell) additionally accepts bare stone
        // without writing it: "sits on rock" needs no displacement, only
        // contact. Anything else -- the permanent massif reached early,
        // bedrock, water, a vault lining -- rejects the whole boulder,
        // never just the one cell, matching `pockets`' all-or-nothing seal.
        let mut cells: Vec<(i32, i32)> = Vec::new();
        let mut sealed = true;
        'run: for dx in -reach..=reach {
            if (dx as f32 / a).abs() > 1.0 {
                continue;
            }
            let lx = cx + dx;
            if lx < 0 || lx >= w {
                sealed = false;
                break;
            }
            let extent = (b * (1.0 - (dx as f32 / a).powi(2)).max(0.0).sqrt()).round() as i32;
            let dome = extent.max(1);
            let ground_y = ctx.plans[lx as usize].surface_y;
            for row in 1..=dome {
                let py = ground_y - row;
                if py < 0 {
                    sealed = false;
                    break 'run;
                }
                let mat = world.get(lx, py).material;
                if mat == material::EMPTY || mat == ctx.soil || mat == ctx.sand || mat == ctx.gravel {
                    cells.push((lx, py));
                } else {
                    sealed = false;
                    break 'run;
                }
            }
            let seat = world.get(lx, ground_y).material;
            if seat == ctx.soil || seat == ctx.sand || seat == ctx.gravel {
                cells.push((lx, ground_y));
            } else if seat != ctx.stone {
                sealed = false;
                break 'run;
            }
        }
        if !sealed {
            continue;
        }
        ctx.boulders_seated.set(ctx.boulders_seated.get() + 1);
        for (px, py) in cells {
            // The pale cap-rock family, unconditionally: a boulder is a
            // hard-band survivor by construction (`erosion.rs`'s
            // `BOULDER_HARDNESS` gate on which shed counts toward a
            // socket), so it should read as the resistant stone it came
            // from rather than take the region's ordinary shade -- the same
            // reasoning the speleothem stalactites use for themselves.
            world.set(
                px,
                py,
                Cell::new(ctx.stone, FAMILY_RESISTANT * TONES + loose_shade(ctx, Purpose::Boulder, px, py))
                    .with_attached(true),
            );
            n += 1;
        }
    }
    n
}
