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

/// Wavelength of the field a palette-family threshold is compared against,
/// in cells -- the scale at which the boundary between two rock countries
/// wanders. See `palette_family`, and open bug 0b for the per-cell white
/// noise it replaced.
///
/// Chosen by eye from a three-point sweep on the same crop, because "does
/// this read as a rock boundary" is not a question a number answers:
///
/// - **14** -- the boundary has so much detail it reads as camouflage
///   blotches rather than as two countries meeting.
/// - **40** -- a coastline, with bays and peninsulas. Shipped.
/// - **96** -- clean and sweeping, but a bare curve; the geology goes out
///   of it.
///
/// Deliberately much shorter than [`FAMILY_FIELD_WAVELENGTH`]. The two do
/// different jobs and must not converge: the slow one displaces the
/// *threshold*, this one is what the threshold is compared against, and if
/// they moved together the comparison would be degenerate.
const FAMILY_DITHER_WAVELENGTH: f32 = 40.0;

/// How far the fBm draw is stretched about its centre before it is compared
/// against a family threshold -- see `palette_family`. Set so the field
/// covers the full 0..1 the thresholds were tuned against.
const FAMILY_DITHER_CONTRAST: f32 = 3.2;

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
    palette_family_for(ctx, ctx.terrain.character(x), x, y, cap_rock)
}

/// [`palette_family`] with the column's `Character` already in hand.
///
/// `Terrain::character` is a per-*column* sample, so calling it once per cell
/// down a run is pure repetition. Split out rather than inlined into the
/// caller because the family rule is subtle enough that it must exist in
/// exactly one place; `column_shade_matches_the_per_cell_version` pins the
/// two entry points together.
fn palette_family_for(ctx: &Ctx, ch: crate::worldgen::region::Character, x: i32, y: i32, cap_rock: bool) -> u8 {
    // A preset that asked for no regional variation gets none, including no
    // palette shift. `flat` is the structural test bed and its whole point
    // is that nothing about it wanders; it is also the control render the
    // destruction workstream compares against, so leaving it byte-identical
    // is worth more than colouring it.
    if ctx.terrain.params.region_variation <= 0.0 {
        return FAMILY_NEUTRAL;
    }
    // **A *field*, not a per-cell coin** (open bug 0b).
    //
    // This was `noise::unit(.., x, y)` -- an independent draw per cell.
    // Wherever the family probability is mid-range, which is most of the
    // world *by design* (the aridity ramps were widened deliberately to make
    // it so), that is a per-cell Bernoulli dither between families differing
    // by ~40 brightness points *and* a large hue shift: neutral grey
    // `128,128,132` against warm sandstone `168,146,112`. At play scale it
    // reads as television static, and it was measured as the majority of the
    // deep massif's speckle -- which is why cutting `render.rs`'s grain to
    // zero at depth barely moved the picture.
    //
    // The *intent* is right and is kept: a meandering boundary between
    // differently-coloured countries. The meander was in the wrong place.
    // Sampling fBm on the same `Purpose::Palette` stream -- same stream,
    // different function, so no discriminant is claimed -- puts it in the
    // field, and a country's interior comes out solid instead of sprayed.
    // Stretched, because swapping the *distribution* changes what every
    // threshold below means. `noise::unit` is uniform on 0..1; a normalised
    // three-octave fBm piles up around the middle -- sampled across a world
    // it spans roughly 0.30..0.60. Thresholds tuned to catch the tails of a
    // uniform draw therefore stop firing at all: measured, `wetland` seed 1
    // came out with **every rock cell in a single family**, caught by
    // `a_varied_world_uses_more_than_one_rock_family`. Fixing a mechanism
    // and leaving the constants that read it un-re-derived is its own
    // recurring bug here; the stretch is that re-derivation, and it is one
    // line rather than four retuned ramps.
    let u = ((noise::fbm_2d(
        ctx.terrain.seed,
        Purpose::Palette,
        x as f32 / FAMILY_DITHER_WAVELENGTH,
        y as f32 / FAMILY_DITHER_WAVELENGTH,
        3,
    ) - 0.45)
        * FAMILY_DITHER_CONTRAST
        + 0.5)
        .clamp(0.0, 1.0);
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
///
/// `pub(crate)` so `residual.rs` can paint a tor or stack with the same
/// banding the massif around it carries, rather than inventing a second
/// shading rule that would disagree with it at the seam.
/// The per-column invariants of [`strata_shade`], computed once.
///
/// **Built because `stone_massif` is the most expensive pass in the
/// generator by a wide margin** -- 4302 ms of a 6.2 s pass table at
/// 8192x2560, writing 19.7 M cells at ~209 ns each. Almost none of that is
/// the write. It is `strata_shade`, which per cell evaluates
/// `strata_offset` (two fBm octaves, a function of `x` alone),
/// `Terrain::character` (a per-column sample), a band tone draw (a function
/// of the band index, so constant for `strata_thickness` consecutive rows),
/// and `palette_family`'s two 2-D fBm samples.
///
/// Only the last of those genuinely varies per cell. This holds the rest:
/// the column offset and character once, and the band draw memoised as the
/// walk crosses into a new band. Output is identical by construction, and
/// `column_shade_matches_the_per_cell_version` asserts it cell for cell.
pub(crate) struct ColumnShade {
    offset: f32,
    thickness: f32,
    character: crate::worldgen::region::Character,
    band: i32,
    base: u8,
}

impl ColumnShade {
    pub(crate) fn new(ctx: &Ctx, x: i32) -> Self {
        ColumnShade {
            offset: ctx.terrain.strata_offset(x),
            thickness: ctx.terrain.params.strata_thickness.max(1.0),
            character: ctx.terrain.character(x),
            // No band sampled yet. `i32::MIN` cannot collide with a real
            // band index for any world this engine can address, so the first
            // `shade` call always takes the miss path.
            band: i32::MIN,
            base: 0,
        }
    }

    pub(crate) fn shade(&mut self, ctx: &Ctx, x: i32, y: i32) -> u8 {
        let band = ((y as f32 + self.offset) / self.thickness).floor() as i32;
        if band != self.band {
            self.band = band;
            let r = noise::unit(ctx.terrain.seed, Purpose::Strata, band, 0);
            self.base = if r < 0.30 {
                3
            } else if r < 0.55 {
                1
            } else if r < 0.80 {
                0
            } else {
                2
            };
        }
        let tone = if noise::unit(ctx.terrain.seed, Purpose::Shade, x, y) < 0.12 {
            (self.base + 1).min(TONES - 1)
        } else {
            self.base
        };
        palette_family_for(ctx, self.character, x, y, true) * TONES + tone
    }
}

pub(crate) fn strata_shade(ctx: &Ctx, x: i32, y: i32) -> u8 {
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
        // `fill_run` rather than `set` per cell: this pass writes essentially
        // the whole world (19.7 M cells at 8192x2560) and the chunk, the
        // material and its sweep properties are all constant down a run. See
        // `World::fill_run`, which exists for this measurement.
        let mut shade = ColumnShade::new(ctx, x);
        n += world.fill_run(x, (c.surface_y + c.soil_depth).max(0), c.bedrock_top_y - 1, ctx.stone, |y| {
            Cell::new(ctx.stone, shade.shade(ctx, x, y)).with_attached(true)
        });
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
        n += world.fill_run(x, c.bedrock_top_y, ctx.terrain.h - 1, material::BEDROCK, |_| {
            Cell::new(material::BEDROCK, 0).with_attached(true)
        });
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

/// Columns of context `brows` reads beyond the ones it writes: the far
/// detection run to find the face, then the reach it hangs the lip out over.
///
/// **An expression, not a number, and that is the point.** Every margin in
/// this table has now been silently wrong at least once -- `talus` declared 3
/// while walking 120, `vaults` declared 96 while reaching 202 -- and each
/// time the number was correct on the day it was written and had no way to
/// stay correct when the constant behind it moved. A margin is the contract a
/// per-chunk generator will plan against, so an understated one is a promise
/// to produce different cells at a chunk edge, and nothing checks it at
/// runtime: `pass_summary()`'s only consumer reads the GLOBAL list, not the
/// numbers. `every_local_pass_declares_the_margin_it_reaches` in
/// `tests/worldgen.rs` is the check; writing the derivation is what stops it
/// having anything to catch.
pub const BROWS_MARGIN: i32 = RUN_FAR + MAX_BROW_REACH;

/// Columns of context `talus` reads: the far detection run, the walk down to
/// the foot of the fall, and the apron laid out either side of it.
///
/// The apron term is `2 * MAX_TALUS_PEAK` because the heap runs out at a
/// slope of about a half (`for step in 0..peak * 2` at the write site), so a
/// heap starting at the cap reaches twice the cap in columns.
pub const TALUS_MARGIN: i32 = RUN_FAR + MAX_FALL + 2 * MAX_TALUS_PEAK;

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
///
/// **4x for the 4x world** (Phase 2), with the vug's own semi-axes and its
/// crystal lining thickness moved with it. Scaling the cap alone would have
/// done nothing -- the draw never reached 30 -- and scaling the draw without
/// the lining would have left the one bright thing in the deep massif rimmed
/// by a hairline: a 1-3 cell ring is a rim on a 16-cell ellipse and a scratch
/// on a 64-cell one, which is round 2's *"reads as a generator's shape, not a
/// geode's"* finding arriving from the other direction.
const MAX_VAULT_EXTENT: i32 = 120;

/// How thick the solid stone rind around a chamber must be, in cells.
///
/// Two rather than `pockets`' one, and the difference is the whole at-rest
/// argument. A lens is solid powder: a one-cell rind is enough to guarantee
/// nothing is flush with a free face. A chamber is *hollow*, so its roof is
/// an unsupported span and its floor is loose material over a void -- a
/// single stray cell of air on the far side of a one-cell rind would put a
/// hole through into whatever is next door and let the floor run out of it.
pub const VAULT_RIND: i32 = 2;

/// Half-extents of a cave system's envelope, in cells -- **drawn per
/// system since round 6's A2**, not fixed.
///
/// Round 3 shipped a single ~180x70 envelope for every system in every
/// world, tuned by eye against the ASCII probe. The owner's ask: *"Caves
/// should be bigger or at least the upper limit should be bigger, then
/// should have a variety of sizes."* Both halves matter and the second is
/// the harder one -- an envelope that is always maximal is the same
/// failure as one that is always 180 wide, just louder.
///
/// So the draw is heavy-tailed and weighted small: most systems stay near
/// the round-3 size, which is what a world full of them should look like,
/// and the tail reaches [`MAX_CAVE_HALF_W`]x[`MAX_CAVE_HALF_H`]. The
/// exponent is the knob; see [`CaveEnv::draw`].
/// The half-width every lattice constant in this file was tuned against,
/// and the reference `CaveEnv::cell` scales from. Round 3's fixed envelope.
///
/// **Deliberately not scaled with the world, and that is the whole
/// mechanism.** `CaveEnv::cell` is `CAVE_CELL * half_w / ROUND_3_HALF_W`, so
/// this and [`CAVE_CELL`] are the *denominator* every cave-space length is
/// expressed against. Scaling them alongside the envelope would leave every
/// ratio unchanged and produce a bigger box with the same furniture in it --
/// which is precisely what A2 measured and rejected: with the lattice cell
/// held fixed, span across reached its target and largest-walkable fell 38%
/// -> 23%, because the extra area went into finer structure the player
/// cannot occupy. Leaving the reference alone is what makes a 4x envelope a
/// 4x *cave*.
const ROUND_3_HALF_W: i32 = 90;

/// Half-extents of a cave system's envelope, **4x round 7's, for the 4x
/// world** (`Reports/world-scale-handoff.md`, Phase 2).
///
/// The owner's rejection of round 6 was that features have no room to have a
/// shape: *"You cannot create good looking crystals or stalagmites and
/// stalactites that are only 1-2 pixels wide."* The world grew to make room;
/// this is a cave growing into it. Everything the envelope is the
/// denominator of follows for free -- the lattice cell, the edge fades,
/// `min_system_cells`, the monumental chamber's `chamber_scale` -- because
/// each of those is already a ratio against [`ROUND_3_HALF_W`] rather than
/// an absolute size. That was A2's design and it is what makes this a
/// four-line change rather than a re-tune.
///
/// **What does not follow, and is Phase 3's business, not a bug here:**
/// [`MAX_CEILING_SPAN`] is a roof-*structure* bound (how far stone spans
/// unsupported), not a cave-size one, so a 4x system gets roughly 4x the
/// stone teeth dropped into its roof. The handoff predicts this in as many
/// words -- *"expect Phase 2 alone to look worse"* -- because the honeycomb
/// gets larger rather than better until the shape work lands.
const MIN_CAVE_HALF_W: i32 = 220;
const MIN_CAVE_HALF_H: i32 = 88;
/// The upper limit, and the number `vaults`' declared column margin has to
/// cover. **Raising either of these without raising `Pass::margin` in
/// `worldgen/mod.rs` breaks the streaming contract silently** -- nothing
/// checks it at runtime, because `pass_summary()`'s only consumer looks at
/// the GLOBAL list and not at the numbers. `a_cave_cannot_reach_past_its_
/// declared_margin` in `tests/worldgen.rs` is what catches it instead, and
/// the margin is now *derived* from these ([`VAULTS_MARGIN`]) so the two
/// cannot drift apart again.
pub const MAX_CAVE_HALF_W: i32 = 800;
const MAX_CAVE_HALF_H: i32 = 320;

/// Columns of context the `vaults` pass reads beyond the ones it writes.
///
/// Derived here rather than written as a literal in the pass table, because
/// a literal is what let it be silently wrong before: round 6's A2 raised
/// [`MAX_CAVE_HALF_W`] from 90 to 200 and the declared margin stayed at 96,
/// a promise to produce different cells at a chunk edge that nothing checked
/// at runtime. The geode vug's [`MAX_VAULT_EXTENT`] plus its rind sits well
/// inside this.
pub const VAULTS_MARGIN: i32 = MAX_CAVE_HALF_W + VAULT_RIND;

/// One system's envelope: its half-extents, and the local grid arithmetic
/// that used to be `const`.
///
/// Passed by value (it is two `i32`s) rather than threaded as two
/// arguments, so the grid arithmetic stays in one place and a function
/// cannot accidentally index one system's array with another's stride --
/// which is the whole failure mode of turning a compile-time grid into a
/// runtime one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CaveEnv {
    half_w: i32,
    half_h: i32,
}

impl CaveEnv {
    /// The heavy-tailed size draw, weighted small.
    ///
    /// `u^EXP` with `EXP > 1` pushes the mass toward the minimum: at 3.0
    /// the median system sits at about 1/8 of the way up the range and
    /// only the top few percent of draws get near the cap.
    fn draw(seed: u64, k: i32, cx: i32) -> Self {
        const EXP: f32 = 3.0;
        let u = noise::unit(seed, Purpose::CaveSize, k, cx).powf(EXP);
        let half_w = MIN_CAVE_HALF_W + (u * (MAX_CAVE_HALF_W - MIN_CAVE_HALF_W) as f32) as i32;
        // **Height draws separately, and that is the whole of *"longer"*.**
        // Both extents used to come from this one `u`, and because
        // 220/88, 580/232 and 800/320 are all 2.5, the ratio cancelled for
        // every value of it: **every cave in the game was exactly 2.5:1**,
        // large ones included. A size draw cannot produce a long cave if it
        // only produces bigger copies of one shape. The old doc justified
        // the coupling as "a system that is 400 wide and 44 tall is a crack,
        // not a cave" -- true, and an argument for an aspect *bound*, which
        // is below, not for one fixed ratio.
        //
        // Its own stream (`CaveVariety`, reserved and until now unused for
        // exactly this) rather than a second sample of `CaveSize`, so the
        // aspect is not correlated with the size -- otherwise every long
        // cave would also be a big one and the variety is half illusory.
        let v = noise::unit(seed, Purpose::CaveVariety, k, cx);
        let half_h = MIN_CAVE_HALF_H + (v * (MAX_CAVE_HALF_H - MIN_CAVE_HALF_H) as f32) as i32;
        // Clamped to an aspect band rather than left free: one end is the
        // crack the old doc warned about, the other is a shaft, which is a
        // different feature and not what a dissolution trunk is. The band is
        // wide enough that the draw does real work -- 1.5:1 through 8:1,
        // against the single 2.5:1 it replaces.
        let half_h = half_h
            .min((half_w as f32 / MIN_ASPECT) as i32)
            .max((half_w as f32 / MAX_ASPECT) as i32)
            .clamp(MIN_CAVE_HALF_H, MAX_CAVE_HALF_H);
        CaveEnv { half_w, half_h }
    }

    #[inline]
    fn grid_w(self) -> i32 {
        2 * self.half_w + 1
    }

    #[inline]
    fn grid_h(self) -> i32 {
        2 * self.half_h + 1
    }

    #[inline]
    fn area(self) -> usize {
        (self.grid_w() * self.grid_h()) as usize
    }

    #[inline]
    fn idx(self, dx: i32, dy: i32) -> usize {
        ((dy + self.half_h) * self.grid_w() + dx + self.half_w) as usize
    }

    /// Smallest void component this envelope will call a system.
    ///
    /// Was a flat `MIN_SYSTEM_CELLS = 80`, which is 0.62% of the round-3
    /// grid and would be 0.12% of a maximal one -- at which point it has
    /// stopped meaning "is this a system at all" and started meaning
    /// "is this more than nothing". Scaled to the envelope so it keeps its
    /// original meaning at every size.
    #[inline]
    fn min_system_cells(self) -> usize {
        (self.area() / 160).max(MIN_SYSTEM_CELLS)
    }
}





/// Aspect band for [`CaveEnv::draw`], width over height. Replaces the single
/// 2.5:1 that fell out of drawing both extents from one sample.
const MIN_ASPECT: f32 = 1.5;
const MAX_ASPECT: f32 = 8.0;

/// Dissolution trunk constants (Phase 3). Every length is a *fraction* of
/// the envelope, not an absolute, so a bigger cave is a bigger cave rather
/// than a zoom of a small one -- which is the failure `CaveEnv::cell` was
/// built to avoid and the reason round 6's A2 rejected scaling the lattice.
///
/// Radius of the conduit between rooms, in cells. Absolute rather than
/// fractional: a *passage* is a passage at any system size -- what scales is
/// how far it runs and how big the rooms on it are. 9 puts the gnome (7x14)
/// through it with the section aspect below.
const TRUNK_RADIUS: f32 = 9.0;

/// How far a room opens, as a fraction of the envelope half-height. This is
/// what makes chamber-to-passage contrast a *designed* quantity rather than
/// an accident of where Worley cells happened to meet.
const ROOM_GAIN: f32 = 1.60;

/// Wavelength of the room field, in cells. Long, so rooms are beads on a
/// conduit rather than a corrugation. Irregular by construction (`fbm`, not
/// a sine): a regular spacing reads as graph paper, which this repo has
/// already paid for once with a crack lattice on a fixed 6x6 grid.
const ROOM_SCALE: f32 = 170.0;

/// How much of the room field opens at all. Above it the trunk swells;
/// below it stays a passage. Raising it makes rooms rarer and larger.
const ROOM_THRESHOLD: f32 = 0.22;

/// Meander of the conduit, as a fraction of the envelope half-height, and
/// its wavelength in cells. Small and slow: this is a river's wander, not
/// noise on a line.
const TRUNK_WANDER: f32 = 0.28;
const TRUNK_WANDER_SCALE: f32 = 420.0;

/// Roughness on the radius, and its wavelength. **Load-bearing beyond
/// looks:** a constant radius gives a flat roof, and `MAX_CEILING_SPAN`
/// drops a stone tooth into any roof run past 36 columns. The Worley web
/// never triggered that guard (measured: zero teeth, every system, every
/// seed), so a smooth conduit would be the first thing in the game to wake
/// it, and it would saw the trunk into segments. The wavelength is
/// deliberately well under 36.
const ROUGH_GAIN: f32 = 0.45;
const ROUGH_SCALE: f32 = 23.0;

/// Section aspect: how much wider than tall the conduit is. Carries the job
/// `CAVE_SQUASH` did for the field -- everything lies along the bedding
/// rather than cutting across it.
const SECTION_ASPECT: f32 = 1.6;


/// Fraction of the half-height the conduit must stay clear of, top and
/// bottom.
///
/// **The old field faded its threshold on *both* axes and the first version
/// of this carve kept only the horizontal half.** The consequence is not
/// cosmetic: a room can swell until it touches the envelope's top, which sits
/// `VAULT_RIND` (2 cells) below the depth band's ceiling, so all that is left
/// over the void is two cells of stone. At `vault_min_depth: 40` -- the
/// forced-vault tests' setting, caves within 42 rows of the surface -- that
/// is two cells of rock between a chamber and the soil.
///
/// **Added on reasoning, not on a failure, and the difference is worth
/// recording.** It went in while chasing a red
/// `a_forced_vault_world_is_sealed_and_arrives_at_rest`, and that test turns
/// out to fail identically without this carve: the same 8 cells, the same
/// preset and seed, on a worktree build of the previous commit. So this
/// fixed nothing that was measured to be broken. It is kept because the
/// asymmetry with the field it replaces is real and the argument below is
/// the field's own -- but nobody has yet produced the artifact it prevents.
///
/// The horizontal fade below has the same justification the removed
/// `CAVE_EDGE_FADE_Y` did, one axis over: without it a passage crossing the
/// boundary is sawn off into a dead-plumb face, the round-2 scan-cap lesson
/// at envelope scale.
const VERT_MARGIN: f32 = 0.14;

/// Fraction of the half-width over which the conduit pinches shut at the
/// envelope wall. Without it a passage crossing the boundary is sawn off
/// into a dead-plumb face -- the round-2 scan-cap lesson at envelope scale.
const EDGE_FADE: f32 = 0.22;

/// How much of the envelope's height the trunk may sit in, and how many
/// samples pick its bed. Dissolution follows the rock that gives, so the
/// softest band sampled wins; sampling rather than solving keeps it a pure
/// function of position with no search.
const BED_BAND: f32 = 0.55;
const BED_SAMPLES: usize = 9;

/// Feeders per system, and how far each runs as a fraction of the envelope
/// half-height. A trunk with nothing off it is a corridor; these are what
/// make it a system, and they are most of the void (see the arithmetic in
/// `carve_cave_void`).
const TRIB_MIN: i32 = 4;
const TRIB_MAX: i32 = 9;
const TRIB_LEN_MIN: f32 = 0.35;
const TRIB_LEN_MAX: f32 = 1.05;

/// A feeder's radius as a fraction of the trunk's, and how sharply it tapers
/// to nothing. Narrower than the trunk, because that is the direction water
/// gathers: many small feeders into one conduit, never the reverse.
const TRIB_RADIUS: f32 = 0.72;
const TRIB_TAPER: f32 = 0.55;

/// How strongly a feeder follows the local dip sideways as it descends, and
/// how much it meanders on top of that. Dip-following is what stops a feeder
/// reading as a drilled shaft.
const TRIB_DIP: f32 = 2.2;
const TRIB_WANDER: f32 = 0.30;

/// Wavelengths of meander over a feeder's whole length. Under one, so a
/// branch bends once rather than corrugating -- see `trib_wander`.
const TRIB_WANDER_WAVES: f32 = 0.9;

/// How far the monumental chamber's wall wanders from its nominal radius,
/// as a fraction of it, and how many lobes it wanders in.
///
/// The chamber was a literal rasterised ellipse until the owner named it:
/// *"That overall cave shape here is bad though. It looks like a perfect
/// oval, not natural"* (card `20260823T103359957Z-2eaf50`). The precedent
/// for the fix is `lens_roughness`, which did the same thing to the sand
/// lenses -- *"Much better"* (card `20260822T084149897Z-f5e022`).
///
/// `LOBES` is the radius of the circle traced through the noise lattice by
/// the unit direction, so the count of features around the wall is roughly
/// its circumference: 1.6 gives ~10. Fewer reads as a lumpy egg, many more
/// reads as a crinkled edge on a shape that is still an oval underneath.
const CHAMBER_WALL_AMP: f32 = 0.22;
const CHAMBER_WALL_LOBES: f32 = 1.6;

/// Longest horizontal run of void with stone directly above it that a
/// system may keep, in cells -- the roof-span bound the round-2 arithmetic
/// cleared for chambers. Runs longer than this get a stone tooth dropped
/// from their middle ([`carve_cave_void`]) until every span complies.
const MAX_CEILING_SPAN: i32 = 36;

/// A kept component smaller than this is not a system: a sliver of passage
/// with no chamber is a dig reward of nothing. Rejected wholesale, same as
/// a failed seal.
///
/// An *area*, not a length, and already envelope-relative in use --
/// `CaveEnv::min_system_cells` is `(area / 160).max(MIN_SYSTEM_CELLS)` -- so
/// the 4x envelope carries it without this number moving.
const MIN_SYSTEM_CELLS: usize = 80;

/// Columns a cavity needs before its floor gets breakdown mounds, **4x for
/// the 4x world** (Phase 2). See the mound block in `carve_cave_void` for
/// why this could not stay at 20.
const MOUND_MIN_WIDTH: usize = 80;

/// Chance a placement draw is a geode vug rather than a cave system. The
/// vug stays as the rare jewel variant; the cave is the main event.
const VUG_CHANCE: f32 = 0.25;

/// Speleothem placement, all tuned against the ASCII probe:
/// per-candidate-column chance at full ceiling height (the smoothstep over
/// the open span makes tall chambers -- drip height -- much denser than low
/// passages), the crystal minority, and how often a formation is a paired
/// stalactite-over-stalagmite.
///
/// **Round 6's A3 roughly halves this from round 5's 0.30**, landed
/// together with the spacing and width constants below because the fix the
/// owner asked for -- *"fewer... but thicker"* -- is a budget reallocation,
/// not a density dial alone: round 5 measured 29-40/system by spending
/// everything on count; A3 spends less on count (target 12-20/system) so
/// there is width budget for the taper below without exceeding it.
const SPELEO_DENSITY: f32 = 0.19;
const SPELEO_CRYSTAL: f32 = 0.15;
/// Chance a placement is a stalactite-over-stalagmite pair rather than a
/// single stalactite or stalagmite. Round 5 forced the pair's two halves to
/// stop just short of touching (`SPELEO_PAIR`'s "almost meeting" gap-shrink,
/// removed in round 6's A3 below); this constant, which only decides *how
/// often* a placement is a pair at all, is unchanged.
const SPELEO_PAIR: f32 = 0.25;

/// Round-5 task 4b: the fixed `SPELEO_SPACING = 4` this replaces enforced
/// *even* spacing everywhere, which is precisely the "reads as a comb"
/// artefact the beauty review named -- the opposite of drip concentration.
/// The minimum gap between candidate columns is driven by
/// [`noise::value_1d`] on `Purpose::Drip`, a low-frequency field sampled
/// every [`DRIP_SCALE`] cells: in a wet stretch the gap shrinks to
/// [`SPELEO_SPACING_MIN`], letting formations bunch; in a dry one it grows
/// to [`SPELEO_SPACING_MAX`], leaving the stretch close to bare.
///
/// **Round 6's A3 raises both floors.** Round 5's floor was a hard 2,
/// tuned so a 1-2 wide secondary taper could not reach across the gap and
/// merge two neighbours into one wall. A3's base width
/// ([`SPELEO_WIDTH_MIN`]..[`SPELEO_WIDTH_MAX`]) is a real cone reaching up
/// to 4 cells either side of its own centre, so the same merge risk now
/// needs a wider floor to guarantee against -- every footprint's half-width
/// is clamped to `(min_spacing - 1) / 2` at the write site
/// (`carve_cave_void`'s speleothem block) precisely so two neighbouring
/// footprints can never touch, by construction, rather than by a
/// fully-covered-run check after the fact (round 5's approach, removed:
/// see the block's own comment for why it no longer has a job). Raising
/// the floor from 2 to 6 and the ceiling from 14 to 26 is also most of how
/// A3 buys back "fewer": count, spacing and width are the same knob spent
/// three ways, and this task spends more of it on spacing/width than round
/// 5 did, per the owner's explicit instruction to spend the budget on size
/// rather than count now that formations are scenery and cost nothing in
/// walkability.
/// **4x for the 4x world** (Phase 2). Spacing, width and count are the same
/// budget spent three ways, and the envelope's own 4x growth already spends
/// the count side of it: a system four times wider offers four times the
/// candidate columns at the same spacing, so holding these fixed would have
/// bought back exactly the *"way too many, way too thin"* distribution A3
/// was undoing, at four times the scale. Growing the gaps with the cave
/// keeps formations-per-chamber where round 6 left it and leaves the width
/// budget below free to actually be spent.
const SPELEO_SPACING_MIN: i32 = 36;
const SPELEO_SPACING_MAX: i32 = 112;
const DRIP_SCALE: f32 = 160.0;

/// Base width of a formation's cone, in cells, drawn per placement --
/// round 6's A3, replacing round 5's "a minority go two cells wide, the
/// secondary column shorter", which the owner read correctly as one step,
/// not a taper (*"they are all 1 pixel thick. They should have a taper and
/// be thicker"*). Every formation with a bottommost run draws a width in
/// this range (not a minority chance -- "thicker" means most of them, not
/// some of them) and gets a real cone: height falls off linearly with
/// distance from centre, reaching the drawn base width only at the very
/// foot and narrowing to the ordinary 1-wide trunk at the tip. Clamped
/// against [`SPELEO_SPACING_MIN`]/`MAX` at the write site so no footprint
/// can ever reach a neighbour's.
/// How far a cone column may look for its own ceiling or floor before
/// giving up, in rows. A ceiling that drops more than this across a single
/// column is not the same ceiling, and a cone that followed it there would
/// be drawing into a different cavity rather than thickening this
/// formation's root.
const CONE_ANCHOR_SEARCH: i32 = 3;

/// **The owner's complaint, in one pair of numbers, 4x'd for the 4x world**
/// (Phase 2). Round 6's A3 got these from 1-2 cells to 3-8 and the round-7
/// census still measured median base width **3** across 16 seeds and every
/// preset (`examples/cave_probe.rs`) -- which is what *"you cannot create
/// good looking crystals or stalagmites and stalactites that are only 1-2
/// pixels wide"* is about. Three cells has no silhouette, no taper and no
/// interior at any zoom; twelve has all three.
///
/// The overlap proof at the write site is unchanged and still by
/// construction, not by inspection: every footprint's half-width is clamped
/// to `(min_spacing - 1) / 2`, so two neighbouring cones cannot touch. With
/// [`SPELEO_SPACING_MIN`] at 36 that clamp is 17 and this ceiling's own half
/// is 16, so the width draw is what binds -- the same ordering round 6 had
/// (floor 9 clamping to 4 against a half-ceiling of 4), which is what keeps
/// the taper reachable rather than permanently clipped.
const SPELEO_WIDTH_MIN: i32 = 12;
const SPELEO_WIDTH_MAX: i32 = 32;

/// Round-5 task 5: how many cells below the waterline a column's floor may
/// sit and still be a candidate -- too far below and a stalagmite would
/// have to be implausibly tall to break the surface at all. The chance a
/// candidate actually places one, and its crystal minority raised well
/// past the ordinary `SPELEO_CRYSTAL`, because this is specifically the
/// shot criterion 5 asks for: a lit formation breaking still water.
///
/// `WATERLINE_CHANCE` is 1.0 -- every eligible column places one -- and
/// that is load-bearing, not generous: measured (`wetland`, 8 seeds), the
/// span requirement below the reach check is what actually gates this,
/// dropping 4-39 reach-eligible columns per system to 0-6 that also have
/// the headroom to clear the surface. Most of a system's length is
/// ordinary task-2 passage (span 5-8), and a floor several cells under the
/// table needs more room than that to break through -- so there is
/// nothing left to spend a chance draw on rejecting. See the round-5
/// finding for the bar this did and did not reach.
const WATERLINE_FLOOR_REACH: i32 = 16;
const WATERLINE_CHANCE: f32 = 1.0;
const WATERLINE_CRYSTAL: f32 = 0.5;

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
    /// Every formation's drawn base width, in cells (round 6, A3).
    ///
    /// `cave_probe` has no width instrument -- its formation test only ever
    /// recognises a column with void on *both* immediate flanks, which a
    /// wide cone's own base rows fail by construction (see finding A3-1) --
    /// so this is the only place the "median base width >= 3, range 3-8"
    /// bar can be read from directly, off the true values this pass wrote
    /// rather than a re-derivation through a ruler blind to the quantity.
    formation_widths: Vec<i32>,
    /// Wall time of the whole `vaults` pass, in milliseconds (round 6, A0).
    ///
    /// Nothing measured this before: the pass table's cell count says
    /// whether something placed, never what it cost, and the cave path is
    /// the one pass in this file whose cost is **quadratic in envelope
    /// area** (the ceiling-settle fixpoint re-floods the whole envelope
    /// once per tooth dropped, and a wide roof run needs several). A2's
    /// planned size growth is a live threat to the ~800ms regen budget and
    /// there was no number to size it against -- this is that number,
    /// printed unconditionally whenever the pass actually iterates, so a
    /// world where every draw is rejected still reports what the rejections
    /// cost.
    build_ms: f64,
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
    let t0 = std::time::Instant::now();
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
        // Drawn before the depth-band test, which needs `half_h`, and keyed
        // on the placement rather than on `cy` -- `cy` is chosen *from* the
        // band that `half_h` defines, so keying the size on it would be
        // circular.
        let env = CaveEnv::draw(seed, k, cx);

        if !vug {
            // The envelope must sit entirely inside the depth band and, with
            // its rind, inside the world. Rejected rather than nudged when it
            // cannot -- moving a rejected system to wherever it fits is how a
            // "rare secret" ends up in the same kind of place in every world.
            // The band being too shallow is the intended outcome in a small
            // world, not a failure to handle: such a world has no system and
            // the pass counter reports zero rather than the pass quietly
            // relaxing its own depth rule to produce one.
            // **Shrunk to fit, not rejected for being big.** The draw is
            // heavy-tailed, so a large envelope near a world edge or in a
            // shallow depth band would simply be lost -- and measured at
            // half-width 200 that is a rejection rate near 20% of draws
            // against 9% before, which would eat round 5's presence win
            // whole. Clamping the *size* is not the same as moving the
            // system: the comment above forbids relocating a rejected
            // placement, because that puts every world's secret in the same
            // kind of place. A cave that is smaller because there is less
            // room for it is the opposite -- it stays exactly where the
            // draw put it.
            //
            // Rejected only when even the minimum envelope does not fit,
            // which is the round-3 behaviour for a world too shallow to
            // hold a cave at all.
            let room_w = cx.min(w - 1 - cx) - VAULT_RIND;
            let room_h = (bottom - top) / 2 - VAULT_RIND;
            let env = CaveEnv {
                half_w: env.half_w.min(room_w),
                half_h: env.half_h.min(room_h),
            };
            if env.half_w < MIN_CAVE_HALF_W || env.half_h < MIN_CAVE_HALF_H {
                continue;
            }
            let lo = top + env.half_h + VAULT_RIND;
            let hi = bottom - env.half_h - VAULT_RIND;
            if hi < lo {
                continue;
            }
            let cy = lo + (noise::unit(seed, Purpose::Vault, k, 1) * (hi - lo + 1) as f32) as i32;
            let r = cave_system(ctx, env, world, k, cx, cy);
            written += r.cells;
            report.cells += r.cells;
            report.systems += r.systems;
            report.chambers += r.chambers;
            report.passage_cells += r.passage_cells;
            report.speleothem_cells += r.speleothem_cells;
            report.water_cells += r.water_cells;
            report.formation_widths.extend(r.formation_widths);
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
        let a = 32.0 + noise::unit(seed, Purpose::Vault, k * 17, 4) * 48.0;
        let b = 24.0 + noise::unit(seed, Purpose::Vault, k * 17, 5) * 24.0;
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
                    let thickness = 4.0 + 8.0 * noise::unit(seed, Purpose::Vault, px, py);
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
        let thickness = 8 + (noise::unit(seed, Purpose::Vault, k, 8) * 12.0) as i32;
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
    // The counters next to the picture, printed whenever the pass actually
    // iterated: the pass-table row alone cannot say whether the anatomy
    // stages fired, and a cave is invisible in any render until someone digs
    // to it. The format deliberately does not match the table's `name N
    // cells` rows, so the sweep's parser never mistakes it for one.
    //
    // Timing is printed even when `report.systems == 0` (round 6, A0): a
    // world where every draw got rejected at the seal check still paid the
    // full envelope scan for each one, and that cost was previously
    // invisible -- the old gate (`report.systems > 0`) hid exactly the
    // rejection-heavy worlds this number exists to catch.
    report.build_ms = t0.elapsed().as_secs_f64() * 1000.0;
    if whole + extra > 0 {
        // Formation base widths (round 6, A3): median and range, read off
        // this pass's own writes rather than `cave_probe` -- see
        // `VaultReport::formation_widths`'s own comment for why the probe
        // cannot see this quantity at all.
        let (w_med, w_lo, w_hi) = if report.formation_widths.is_empty() {
            (0, 0, 0)
        } else {
            let mut ws = report.formation_widths.clone();
            ws.sort_unstable();
            (ws[ws.len() / 2], ws[0], ws[ws.len() - 1])
        };
        println!(
            "  vaults detail: systems {} chambers {} passages {} speleothems {} water {} \
             | formations {} base-width med {} range {}-{} | ceiling teeth {} | {:.1}ms",
            report.systems,
            report.chambers,
            report.passage_cells,
            report.speleothem_cells,
            report.water_cells,
            report.formation_widths.len(),
            w_med,
            w_lo,
            w_hi,
            CEILING_TEETH.with(|t| t.get()),
            report.build_ms
        );
    }
    written
}

/// Whether the cave plan leaves something solid at `(dx, dy)`: undisturbed
/// rock (anything outside the kept void, including outside the envelope),
/// or planned floor gravel. What the floor verifier leans on.
fn planned_solid(env: CaveEnv, void: &[bool], floor: &[Option<(i32, i32, i32)>], dx: i32, dy: i32) -> bool {
    if dx.abs() > env.half_w || dy.abs() > env.half_h {
        return true;
    }
    if !void[env.idx(dx, dy)] {
        return true;
    }
    matches!(floor[(dx + env.half_w) as usize], Some((_, b, h)) if h > 0 && dy > b - h)
}

/// Keep only the connected component containing the seed point -- the void
/// cell nearest the envelope centre, ties broken in raster order -- over a
/// 4-neighbour flood. A disconnected satellite chamber is a second system
/// nobody can reach from the first, so it goes back to being stone.
fn keep_seed_component(env: CaveEnv, void: &mut [bool]) {
    let mut seed = None;
    let mut best = i64::MAX;
    for dy in -env.half_h..=env.half_h {
        for dx in -env.half_w..=env.half_w {
            if void[env.idx(dx, dy)] {
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
    kept[env.idx(seed.0, seed.1)] = true;
    let mut stack = vec![seed];
    while let Some((x, y)) = stack.pop() {
        for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
            if nx.abs() <= env.half_w
                && ny.abs() <= env.half_h
                && void[env.idx(nx, ny)]
                && !kept[env.idx(nx, ny)]
            {
                kept[env.idx(nx, ny)] = true;
                stack.push((nx, ny));
            }
        }
    }
    void.copy_from_slice(&kept);
}

/// Every ceiling run longer than [`MAX_CEILING_SPAN`], as `(row, start,
/// len)`, topmost-leftmost first so the guard is deterministic. A ceiling
/// run is a maximal horizontal run of void cells each with non-void
/// directly above -- which, once the seal has passed, is stone: the
/// unsupported roof span the guard bounds.
///
/// **Returns every violation in one scan, not just the first** (round 6,
/// A0). The settle loop below used to call this once per tooth and
/// re-flood the whole envelope (`keep_seed_component`) between every single
/// one -- O(area) per tooth, and a system with a 176-cell widest span
/// (measured, `cave_probe`) needs about five. Collecting every run first and
/// dropping all their teeth before the next flood collapses that to one
/// flood per *round*, and a round only repeats at all when a dropped tooth
/// severs a passage and exposes a new violation on the far side of the cut
/// -- rare, and never more than a handful of rounds in the measured sweep.
/// Two long runs on the same row are never within tooth-width (5) of each
/// other's midpoints in practice (each is >36 cells and the gap between
/// them is itself non-void), so applying every tooth from one scan before
/// rescanning does not clip a compliant neighbour.
fn all_long_ceiling_runs(env: CaveEnv, void: &[bool]) -> Vec<(i32, i32, i32)> {
    let mut runs = Vec::new();
    for dy in -env.half_h..=env.half_h {
        let mut run = 0;
        for dx in -env.half_w..=env.half_w + 1 {
            let ceiling = dx <= env.half_w
                && void[env.idx(dx, dy)]
                && (dy == -env.half_h || !void[env.idx(dx, dy - 1)]);
            if ceiling {
                run += 1;
            } else {
                if run > MAX_CEILING_SPAN {
                    runs.push((dy, dx - run, run));
                }
                run = 0;
            }
        }
    }
    runs
}

thread_local! {
    /// Teeth the ceiling guard actually dropped, tallied for `vaults detail`.
    ///
    /// **Because a bound that never binds and a bound doing its job are the same
    /// silence.** Measured 2026-08-23: it is **zero**, on every system of every
    /// canyon seed tried — [`MAX_CEILING_SPAN`] has never fired in the shipped
    /// configuration. A "roof run" needs void with stone *directly* above it for
    /// 36 consecutive columns, i.e. a **flat** ceiling, and neither a Worley
    /// boundary web nor a rasterised ellipse produces one: on a curved roof each
    /// row contributes only the few columns where the curve sits at that exact
    /// height. Disabling the guard entirely (span 100000) moved the census by
    /// nothing — walkable regions max 92 -> 90, every other column identical.
    ///
    /// **This matters for Phase 3 rather than being a curiosity.** Dissolution
    /// carves passages *along* bedding, which is precisely a long horizontal run
    /// at constant height with stone above — so a guard that is dormant today
    /// would begin firing hard the moment that lands, and would saw a trunk
    /// passage into segments. Read this counter beside any cave-shape change.
    ///
    /// `cave_probe`'s "widest ceiling span" column cannot answer this: it
    /// measures the widest void run in *any* row, with no condition on what is
    /// above it. Two different quantities wearing one name.
    static CEILING_TEETH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
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
/// One column of the conduit: an elliptical section, wider than tall.
///
/// Swept along x by its own half-width, so consecutive steps overlap and the
/// conduit comes out continuous rather than as a row of discs -- which is
/// what a naive per-step stamp gives on any path that is not axis-aligned,
/// and it reads as a string of beads.
///
/// Wider than tall carries the job [`CAVE_SQUASH`] did for the old field:
/// everything lies along the bedding instead of cutting across it.
fn stamp_section(env: CaveEnv, void: &mut [bool], x: f32, ty: f32, r: f32) {
    let rv = r;
    let rh = r * SECTION_ASPECT;
    let lo = (ty - rv).floor() as i32;
    let hi = (ty + rv).ceil() as i32;
    for dy in lo..=hi {
        if dy.abs() > env.half_h {
            continue;
        }
        let dyf = dy as f32 - ty;
        if (dyf / rv).abs() >= 1.0 {
            continue;
        }
        let span = rh * (1.0 - (dyf / rv).powi(2)).sqrt();
        let x0 = (x - span).floor() as i32;
        let x1 = (x + span).ceil() as i32;
        for ex in x0..=x1 {
            if ex.abs() <= env.half_w {
                void[env.idx(ex, dy)] = true;
            }
        }
    }
}

/// A feeder's sideways meander, as a pure function of its own index and how
/// far along it is -- so two feeders of one system wander differently and
/// nothing depends on iteration order.
///
/// **The frequency is the whole of whether this reads as a passage or as a
/// lightning bolt.** At `f * 6.0` the first render came out as sharp
/// switchbacks: several wavelengths over a branch a hundred cells long, with
/// a lateral swing of tens of cells between steps one cell apart. A feeder
/// meanders; it does not corrugate.
fn trib_wander(sys: u64, k: i32, b: i32, f: f32) -> f32 {
    noise::fbm_1d_c(sys, Purpose::CaveVariety, f * TRIB_WANDER_WAVES + (k * 31 + b * 7) as f32, 2)
}

/// Stamp the conduit along the segment between two centres.
///
/// **Not the endpoints, the segment.** Stamping only at sampled centres
/// leaves gaps the moment consecutive samples are further apart than a
/// section is wide -- which happens on any path that turns, and comes out as
/// a string of beads or a torn passage. Half-cell steps are below the
/// smallest section this ever draws, so the sweep is continuous by
/// construction rather than by the sampling happening to be fine enough.
fn stamp_run(env: CaveEnv, void: &mut [bool], from: (f32, f32), to: (f32, f32), r: f32) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let steps = ((dx * dx + dy * dy).sqrt() * 2.0).ceil().max(1.0);
    for i in 0..=(steps as i32) {
        let t = i as f32 / steps;
        stamp_section(env, void, from.0 + dx * t, from.1 + dy * t, r);
    }
}

fn carve_cave_void(ctx: &Ctx, env: CaveEnv, world: &World, k: i32, cx: i32, cy: i32) -> Option<Vec<bool>> {
    // A per-system field seed derived from the placement stream: two systems
    // in one world must not share a Worley lattice, or a pair placed near
    // each other would carve correlated shapes.
    // A per-system field seed derived from the placement stream: two systems
    // in one world must not share a field, or a pair placed near each other
    // would carve correlated shapes.
    let sys = noise::hash(ctx.terrain.seed, Purpose::Vault, k, 97);
    let mut void = vec![false; env.area()];

    // The bedding locus this system is measured against -- the same
    // `y + strata_offset(x)` the shade pass bands the rock with, the benches
    // snap to and the lenses lie in. Fifth consumer of that one pure
    // function; it has to agree with the other four.
    let base_off = ctx.terrain.strata_offset(cx);
    let hardness = ctx.terrain.hardness_field();

    // ---- the trunk: one conduit along one bed, opening into rooms ----
    //
    // Stamped as a swept disc whose centre follows the bedding and whose
    // radius varies continuously. Three things fall out of that shape which
    // the Worley web could not give:
    //
    // - **It is a chain, not a net.** Rooms are beads on one conduit, so a
    //   player walks from one to the next instead of choosing between
    //   ninety-two pockets. That is the owner's *"chains of caves"*.
    // - **Its length is its own parameter.** A trunk is as long as the
    //   envelope, so *"longer"* is now a size draw rather than a shape
    //   accident (see `CaveEnv::draw`).
    // - **No straight segments.** `F2 - F1` is zero along Worley cell
    //   boundaries, which are perpendicular bisectors -- straight by
    //   construction, meeting at 120-degree junctions, and read as cracked
    //   mud rather than as rock. *"The voroni patter is too much."*
    //
    // **The radius must never hold still, and that is structural as well as
    // aesthetic.** A constant-radius tube along a constant bed has a flat
    // roof, and `MAX_CEILING_SPAN` drops a stone tooth into any roof run
    // past 36 columns. That guard has never once fired against the Worley
    // web (measured: zero teeth, every system, every seed -- a curved roof
    // never gives 36 columns at one height), so a flat-roofed conduit would
    // wake it up and saw the trunk into segments, which is the one thing a
    // chain cannot survive. The `fbm` on the radius keeps every roof run
    // short by construction. Read `ceiling teeth` in `vaults detail`
    // alongside any change here.
    let half_w = env.half_w as f32;
    let half_h = env.half_h as f32;

    // Where the trunk sits in the envelope, and which bed it dissolves.
    // Biased toward a *soft* band: dissolution follows the rock that gives.
    // Sampled at the system's own centre, so one bed is chosen for the whole
    // conduit rather than the trunk wandering between beds as it goes.
    let bed_pick = {
        let mut best = (f32::MAX, 0.0f32);
        for i in 0..BED_SAMPLES {
            let f = (i as f32 / (BED_SAMPLES - 1) as f32) * 2.0 - 1.0;
            let e = ctx.terrain.datum() - (cy as f32 + f * half_h * BED_BAND);
            let h = hardness.at(cx, e);
            if h < best.0 {
                best = (h, f);
            }
        }
        best.1 * half_h * BED_BAND
    };

    // **The trunk's centreline, named once, because a feeder that does not
    // start exactly on it is thrown away.** `keep_seed_component` keeps only
    // the component containing the envelope centre, so a branch seeded at
    // the bed while the trunk had wandered off it is a disconnected
    // satellite and goes back to being stone -- silently, and looking
    // exactly like "feeders add no void". Measured that way: adding feeders
    // moved void 0.121% -> 0.138% and *raised* `walk_regions` max from 3 to
    // 10, which is the signature of fragments being culled rather than
    // branches being joined.
    let trunk_y = |x: f32| {
        // The conduit lies *in* its bed, so it rises and falls with the dip,
        // tilt and fold included -- the same shear the old field applied,
        // kept, because it is what makes a cave read as geology.
        let bed = base_off - ctx.terrain.strata_offset(cx + x as i32);
        // A slow wander so the conduit is not a ruled line. Low frequency
        // and small amplitude: this is meander, not noise.
        let wander = noise::fbm_1d_c(sys, Purpose::Cave, x / TRUNK_WANDER_SCALE, 2)
            * half_h
            * TRUNK_WANDER;
        bed_pick + bed + wander
    };

    for dx in -env.half_w..=env.half_w {
        let x = dx as f32;
        let ty = trunk_y(x);

        // Radius: a narrow conduit that opens into rooms. The room field is
        // a slower `fbm` than the roughness one, so rooms are spaced along
        // the trunk at irregular intervals rather than periodically -- a
        // regular spacing reads as graph paper, which this repo has already
        // paid for once with a crack lattice.
        let room = noise::fbm_1d_c(sys, Purpose::CaveVariety, x / ROOM_SCALE, 2);
        let open = ((room - ROOM_THRESHOLD) / (1.0 - ROOM_THRESHOLD)).clamp(0.0, 1.0);
        let rough = noise::fbm_1d_c(sys, Purpose::Cave, x / ROUGH_SCALE + 41.0, 3);
        let r = (TRUNK_RADIUS + open * open * half_h * ROOM_GAIN) * (1.0 + rough * ROUGH_GAIN);

        // Pinch shut before the envelope wall, or a passage crossing the
        // boundary is sawn off into a dead-plumb face the full height of the
        // envelope -- the round-2 scan-cap lesson at envelope scale, and the
        // reason the old field faded its threshold at the edge.
        let fade = ((half_w - x.abs()) / (half_w * EDGE_FADE)).clamp(0.0, 1.0);
        // Horizontal pinch, then the vertical clamp: the section may not
        // reach within `VERT_MARGIN` of the envelope's top or bottom, or the
        // rind is the only rock left over the void. See `VERT_MARGIN`.
        let r = (r * fade).min(((1.0 - VERT_MARGIN) * half_h - ty.abs()).max(0.0));
        if r < 1.0 {
            continue;
        }

        stamp_section(env, &mut void, x, ty, r);
    }

    // ---- tributaries: feeders into the trunk, down the dip ----
    //
    // **The trunk alone is not enough cave, and that is arithmetic rather
    // than tuning.** A conduit is a one-dimensional structure through a
    // two-dimensional envelope, so it fills a far smaller fraction of it
    // than a space-filling threshold field did: measured, trunk-only put
    // void at **0.121%** of the deep massif against the honeycomb's 0.563%,
    // which is the same "a 4x world with round-7 caves has essentially no
    // caves in it" failure §4 of the Phase 2 report measures, arrived at
    // from the other direction. Widening the trunk to close that gap would
    // buy one enormous tube; feeders buy the same area as *cave system*.
    //
    // They also answer what a trunk cannot: a single conduit is a corridor,
    // and what makes a cave feel explorable is somewhere to turn off. Each
    // starts at a point on the trunk and runs down the dip -- the direction
    // water actually moves -- with its own wander and a radius that tapers
    // to nothing at the far end, so it reads as a feeder narrowing rather
    // than a tube stopping.
    let branches = TRIB_MIN
        + (noise::unit(sys, Purpose::CaveVariety, k, 7) * (TRIB_MAX - TRIB_MIN + 1) as f32) as i32;
    for b in 0..branches {
        // Where it leaves the trunk, and how far it runs. Both drawn on the
        // branch index so two systems' feeders are uncorrelated.
        let at = (noise::unit(sys, Purpose::CaveVariety, k, 100 + b) * 2.0 - 1.0) * half_w * 0.8;
        let len = (TRIB_LEN_MIN
            + noise::unit(sys, Purpose::CaveVariety, k, 200 + b) * (TRIB_LEN_MAX - TRIB_LEN_MIN))
            * half_h;
        // Up or down from the trunk, and the dip it follows sideways.
        let down = if noise::unit(sys, Purpose::CaveVariety, k, 300 + b) < 0.5 { -1.0 } else { 1.0 };
        let dip = (ctx.terrain.strata_offset(cx + at as i32 + 8)
            - ctx.terrain.strata_offset(cx + at as i32 - 8))
            / 16.0;

        let steps = len.ceil() as i32;
        let mut prev: Option<(f32, f32)> = None;
        for i in 0..=steps {
            let f = i as f32 / steps.max(1) as f32;
            // Along the dip as it descends, so a feeder cuts across bedding
            // at the shallow angle water does, not at right angles.
            let x = at + f * len * (dip * TRIB_DIP + trib_wander(sys, k, b, f) * TRIB_WANDER);
            // Anchored on the trunk at its own start, then descending from
            // there -- not on the bed, which the trunk has usually wandered
            // off by the time it reaches this column.
            let ty = trunk_y(at) + down * f * len;
            // Tapers to nothing: a feeder narrows into the rock rather than
            // ending in a flat wall, which is the same reason the trunk
            // pinches shut at the envelope edge.
            let r = TRUNK_RADIUS * TRIB_RADIUS * (1.0 - f).powf(TRIB_TAPER)
                * (1.0 + noise::fbm_1d_c(sys, Purpose::Cave, f * len / ROUGH_SCALE + 91.0, 3) * ROUGH_GAIN);
            let fade = ((half_w - x.abs()) / (half_w * EDGE_FADE)).clamp(0.0, 1.0);
            let r = (r * fade).min(((1.0 - VERT_MARGIN) * half_h - ty.abs()).max(0.0));
            if r < 1.0 || ty.abs() > half_h {
                prev = None;
                continue;
            }
            if let Some(p) = prev {
                stamp_run(env, &mut void, p, (x, ty), r);
            } else {
                stamp_section(env, &mut void, x, ty, r);
            }
            prev = Some((x, ty));
        }
    }

    let mut teeth = settle_cave_void(ctx, env, world, cx, cy, &mut void);

    // Round-5 task 3: one monumental chamber, grown around the point of
    // greatest clearance in the settled void, then the whole settle runs
    // again -- growth can only ever breach or over-lengthen a span, never
    // disconnect (it is a pure union), but it can do either of the first
    // two, and re-settling is what turns "grew into a lens" or "grew a roof
    // too wide" back into a system that still satisfies every earlier
    // guarantee.
    let (requested, added) = grow_monumental_chamber(ctx, env, k, cx, &mut void);
    if requested > 0 {
        teeth += settle_cave_void(ctx, env, world, cx, cy, &mut void);
        // Reported unconditionally, including the zero case: a chamber
        // eaten down to nothing by a nearby breach is exactly the "size cap
        // gates whether it happens" landmine in a new shape if it goes
        // unremarked (CLAUDE.md, and task 1/3's own text).
        let survived = added.iter().filter(|&&idx| void[idx]).count();
        println!("  chamber: requested {requested} cells, {survived} survived the re-settle");
    }

    CEILING_TEETH.with(|t| t.set(t.get() + teeth));

    let count = void.iter().filter(|&&v| v).count();
    // Scaled to the envelope (round 6, A2): a flat 80 was 0.62% of the
    // round-3 grid and would be 0.12% of a maximal one, at which point it
    // stops asking "is this a system" and starts asking "is this more than
    // nothing".
    (count >= env.min_system_cells()).then_some(void)
}

/// Component keep, ceiling guard and breach erosion, alternated to a
/// fixpoint (round-5 task 1's doc comment on [`carve_cave_void`] has the
/// full reasoning for why none of the three can run only once or only in a
/// fixed order). Factored out because round-5 task 3 has to run it twice:
/// once on the raw carved field, and again after the monumental chamber's
/// growth, which can breach or over-lengthen a span but never disconnect.
fn settle_cave_void(ctx: &Ctx, env: CaveEnv, world: &World, cx: i32, cy: i32, void: &mut [bool]) -> usize {
    let mut teeth = 0usize;
    loop {
        keep_seed_component(env, void);
        // Every violation this round, not just the first (round-6 A0). The
        // O(area) `keep_seed_component` flood used to run once per *tooth*;
        // it now runs once per *round*, and a round only repeats when a
        // dropped tooth severs a passage and exposes a new violation beyond
        // the cut. Each round still removes at least one void cell, so this
        // terminates on the same argument as before.
        let runs = all_long_ceiling_runs(env, void);
        teeth += runs.len();
        for &(y, x0, len) in &runs {
            // A stone tooth hung from the run's middle: three rows deep,
            // tapering 5-3-1 wide, so the splitter reads as rock coming down
            // from the roof rather than as a one-cell film. It splits the
            // span into two runs of at most half the original.
            let mx = x0 + len / 2;
            for j in 0..3 {
                let half = 2 - j;
                for x in mx - half..=mx + half {
                    if x.abs() <= env.half_w && (y + j).abs() <= env.half_h {
                        void[env.idx(x, y + j)] = false;
                    }
                }
            }
        }
        let eroded = erode_breaches(ctx, env, world, cx, cy, void);
        if runs.is_empty() && !eroded {
            break;
        }
    }
    teeth
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
fn grow_monumental_chamber(ctx: &Ctx, env: CaveEnv, k: i32, cx: i32, void: &mut [bool]) -> (usize, Vec<usize>) {
    let n = (env.grid_w() * env.grid_h()) as usize;
    let mut dist = vec![0i32; n];
    for dy in -env.half_h..=env.half_h {
        for dx in -env.half_w..=env.half_w {
            let idx = env.idx(dx, dy);
            dist[idx] = if void[idx] { i32::MAX / 4 } else { 0 };
        }
    }
    let get = |dist: &[i32], dx: i32, dy: i32| {
        if dx.abs() > env.half_w || dy.abs() > env.half_h { 0 } else { dist[env.idx(dx, dy)] }
    };
    // Forward pass (top-left to bottom-right), then backward -- together
    // exact for Chebyshev distance, where a single pass is not.
    for dy in -env.half_h..=env.half_h {
        for dx in -env.half_w..=env.half_w {
            let idx = env.idx(dx, dy);
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
    for dy in (-env.half_h..=env.half_h).rev() {
        for dx in (-env.half_w..=env.half_w).rev() {
            let idx = env.idx(dx, dy);
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
    for dy in -env.half_h..=env.half_h {
        for dx in -env.half_w..=env.half_w {
            let idx = env.idx(dx, dy);
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
    for dy in -env.half_h..=env.half_h {
        for dx in -env.half_w..=env.half_w {
            let idx = env.idx(dx, dy);
            if void[idx] && dist[idx] >= max_clear - 1 {
                let room = (env.half_h - dy.abs()).min(env.half_w - dx.abs());
                if room > best.0 {
                    best = (room, dx, dy);
                }
            }
        }
    }
    let (_, bx, by) = best;

    let seed = ctx.terrain.seed;
    // Scaled with the envelope, for the same reason `CaveEnv::cell` is
    // (round 6, A2): a fixed 12-24 half-height is a *room* in a round-3
    // envelope and an alcove in a maximal one, so held constant it makes
    // large systems read as all passage and no release. Measured on canyon
    // over 16 seeds, chamber fixed while the lattice scaled: contrast
    // p95/median 4.55x against the 5.0x bar, recovered by this line.
    let chamber_scale = env.half_w as f32 / ROUND_3_HALF_W as f32;
    let rv_draw =
        (12.0 + noise::unit(seed, Purpose::CaveChamber, cx + bx, k) * 12.0) * chamber_scale;
    let rh_draw = rv_draw * 1.4;
    // The cap: shrink to whatever room the envelope has left from this
    // centre. Never to less than 2 -- a system whose clearance point sits
    // hard against the envelope edge still gets *a* chamber, a small one,
    // not none.
    let rv = rv_draw.min((env.half_h - by.abs()) as f32).max(2.0);
    let rh = rh_draw.min((env.half_w - bx.abs()) as f32).max(2.0);

    let mut added = Vec::new();
    // Bounds widened by the wall's own amplitude, or every bulge would be
    // clipped flat against the old `r <= 1` box and the roughening would show
    // up only as bites taken *out* of the oval -- which reads worse than the
    // oval did.
    let (rv_i, rh_i) =
        ((rv * (1.0 + CHAMBER_WALL_AMP)).ceil() as i32, (rh * (1.0 + CHAMBER_WALL_AMP)).ceil() as i32);
    for dy in -rv_i..=rv_i {
        for dx in -rh_i..=rh_i {
            // **Not an ellipse any more.** This was
            // `(dx/rh)^2 + (dy/rv)^2 > 1.0` -- a literal rasterised oval,
            // unioned on top of whatever the carve had produced. Phase 3
            // replaced the *carve* with dissolution and never touched this,
            // so the biggest room in every system stayed a drawn primitive;
            // shown one, the owner: *"That overall cave shape here is bad
            // though. It looks like a perfect oval, not natural"* (card
            // `20260823T103359957Z-2eaf50`).
            //
            // The radius is perturbed instead, by an fBm sampled on the
            // **unit direction** rather than on the cell. Sampling the cell
            // would vary the threshold *along* a ray as well as across it,
            // which punches stone islands into the middle of the room; on the
            // direction it is constant along every ray, so the outline is
            // irregular and the interior stays solid void. That also keeps
            // the shape star-shaped about its centre, hence radially
            // connected -- which is what lets the re-settle below stay sound,
            // since growth must never disconnect (it is a pure union).
            let (nx, ny) = (dx as f32 / rh, dy as f32 / rv);
            let r = (nx * nx + ny * ny).sqrt();
            let limit = if r < 1e-3 {
                // The centre has no direction to sample. Always inside.
                1.0
            } else {
                let (ux, uy) = (nx / r, ny / r);
                let w = noise::fbm_2d(
                    seed,
                    Purpose::CaveWall,
                    ux * CHAMBER_WALL_LOBES,
                    uy * CHAMBER_WALL_LOBES,
                    2,
                ) * 2.0
                    - 1.0;
                1.0 + CHAMBER_WALL_AMP * w
            };
            if r > limit {
                continue;
            }
            let (ex, ey) = (bx + dx, by + dy);
            if ex.abs() > env.half_w || ey.abs() > env.half_h {
                continue;
            }
            let idx = env.idx(ex, ey);
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
fn erode_breaches(ctx: &Ctx, env: CaveEnv, world: &World, cx: i32, cy: i32, void: &mut [bool]) -> bool {
    let is_stone = |px: i32, py: i32| world.get(px, py).material == ctx.stone;
    let mut any = false;
    loop {
        let mut to_remove = Vec::new();
        for dy in -env.half_h..=env.half_h {
            for dx in -env.half_w..=env.half_w {
                if !void[env.idx(dx, dy)] {
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
                            let still_void = nx.abs() <= env.half_w
                                && ny.abs() <= env.half_h
                                && void[env.idx(nx, ny)];
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
                    to_remove.push(env.idx(dx, dy));
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
fn cave_system(ctx: &Ctx, env: CaveEnv, world: &mut World, k: i32, cx: i32, cy: i32) -> VaultReport {
    let Some(void) = carve_cave_void(ctx, env, world, k, cx, cy) else { return VaultReport::default() };

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
    for dy in -(env.half_h + VAULT_RIND)..=(env.half_h + VAULT_RIND) {
        for dx in -(env.half_w + VAULT_RIND)..=(env.half_w + VAULT_RIND) {
            let in_grid = dx.abs() <= env.half_w && dy.abs() <= env.half_h;
            // The void cells themselves first: they are the envelope's
            // interior and must be stone like everything else -- a lens cell
            // *inside* the would-be void is just as much a breach as one in
            // the rind.
            if in_grid && void[env.idx(dx, dy)] {
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
                    nx.abs() <= env.half_w
                        && ny.abs() <= env.half_h
                        && void[env.idx(nx, ny)]
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
    let mut floor: Vec<Option<(i32, i32, i32)>> = vec![None; env.grid_w() as usize];
    for dx in -env.half_w..=env.half_w {
        let mut bot = None;
        let mut top = 0;
        for dy in (-env.half_h..=env.half_h).rev() {
            if void[env.idx(dx, dy)] {
                if bot.is_none() {
                    bot = Some(dy);
                }
                top = dy;
            } else if bot.is_some() {
                break;
            }
        }
        if let Some(b) = bot {
            floor[(dx + env.half_w) as usize] = Some((top, b, 0));
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
        //
        // Depth, mound size and the "large cavity" bar are all **4x for the
        // 4x world** (Phase 2), and they had to move together with the
        // envelope rather than be left alone. A cavity in a 4x system is four
        // times wider, so a bar of 20 columns is met by every cavity there is
        // and one to three mounds five rows tall become a fine stipple along
        // a floor four times longer -- the count knob left pointing the wrong
        // way, which is the same trade the speleothem constants above spell
        // out. `MOUND_MIN_WIDTH` scaled with the cavity keeps "one to three
        // heaps per large cavity" meaning what it says.
        let base = 8 + (noise::unit(seed, Purpose::CaveFloor, cx + start as i32, k) * 12.0) as i32;
        // Breakdown mounds: one to three heaps per large cavity, proposed as
        // unit-slope triangles on top of the base fill -- a cave floor is
        // rubble fallen from the roof, not tile, and a dead-flat fill from
        // wall to wall was the last ruled line left in the system. Proposed
        // only; the repose sweep below shaves them to gravel's own angle and
        // the verifier guards their toes like everything else's.
        let width = end - start + 1;
        let mut mound = vec![0i32; width];
        if width >= MOUND_MIN_WIDTH {
            let sx = cx + start as i32;
            let count = 1 + (noise::unit(seed, Purpose::CaveFloor, sx, k * 31 + 1) * 3.0) as i32;
            for m in 0..count {
                let at = (noise::unit(seed, Purpose::CaveFloor, sx + m * 7, k * 31 + 2)
                    * width as f32) as i32;
                let peak =
                    8 + (noise::unit(seed, Purpose::CaveFloor, sx + m * 7, k * 31 + 3) * 16.0) as i32;
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
            let dx = i as i32 - env.half_w;
            let mut new_h = h;
            // Shallowest gravel cell with an open diagonal-down neighbour,
            // scanning down -- the flank itself need not be open too.
            for y in (b - h + 1)..=b {
                let exposed = [-1, 1].iter().any(|&s| !planned_solid(env, &void, &floor, dx + s, y + 1));
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
    let mut chamber_col = vec![false; env.grid_w() as usize];
    let mut chambers = 0usize;
    let mut chamber_floors: Vec<i32> = Vec::new();
    // Round-5 task 4c needs the run bounds themselves, not only which
    // columns are inside one -- it fuses a column in the *largest* chamber
    // run, and "largest" is not recoverable from `chamber_col` alone.
    let mut chamber_runs: Vec<(usize, usize)> = Vec::new();
    {
        let fs = |i: usize| floor[i].map(|(_, b, h)| b - h).unwrap_or(env.half_h);
        let tall: Vec<bool> = (0..env.grid_w())
            .map(|i| {
                let dx = i - env.half_w;
                let mut best = 0;
                let mut run = 0;
                for dy in -env.half_h..=env.half_h {
                    if void[env.idx(dx, dy)] && dy <= fs(i as usize) {
                        run += 1;
                        best = best.max(run);
                    } else {
                        run = 0;
                    }
                }
                // 4x for the 4x world (Phase 2): "tall enough to read as a
                // chamber rather than a passage" is a claim about the
                // cavity, and cavities in a 4x envelope are 4x taller, so
                // 12 rows had stopped separating the two.
                best >= 48
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
            // Likewise 4x: six columns of tall void is a junction in a 4x
            // system, not a room.
            if i - start >= 24 {
                chambers += 1;
                chamber_floors.push((start..i).map(fs).max().unwrap_or(0));
                for c in chamber_col.iter_mut().take(i).skip(start) {
                    *c = true;
                }
                chamber_runs.push((start, i));
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
    // Stalactites from ceilings, stalagmites from floors, a crystal
    // minority, and the occasional pair.
    //
    // **Round 6's A3 rebuild.** Owner, verbatim, on round 5's output: *"they
    // are all 1 pixel thick. They should have a taper and be thicker but
    // fewer of them"*, and the cave is *"totally full of stuff"*. Round 5's
    // one concession to width -- a minority of formations grew a single
    // secondary column, shorter, beside the trunk -- was read correctly as
    // a rectangle with one step, not a taper, and was deliberately not
    // relaxed further in Phase 0 until this task landed the taper that
    // makes doing so safe. Below: every formation with a bottommost run
    // draws a real base width ([`SPELEO_WIDTH_MIN`]..[`SPELEO_WIDTH_MAX`])
    // and a continuous cone -- not a chance of one extra column -- and
    // [`SPELEO_DENSITY`]/[`SPELEO_SPACING_MIN`]/`MAX` are retuned together
    // so the count drops enough to spend on that width. Placed on the
    // bottommost run per column -- the galleries the floors are in --
    // *after* the floor verifier, because adding attached solid can only
    // ever add support, never take it away, so the verified gravel stays
    // verified. A stalagmite is written from the *stone* under the run
    // upward through any gravel over it: "structurally trivial, rooted in
    // the massif" means rooted on rock, not standing on loose fill.
    //
    // **A formation may now bridge floor to ceiling.** The old rule --
    // never bridge, because a column splits the passage the player walks
    // -- predates Phase 0's `Material::scenery`: `flowstone`/`spar` are
    // scenery now, the player walks through one, and a column blocks
    // nothing regardless of where it stands. `SPELEO_PAIR`'s "almost
    // meeting" gap-shrink existed only to enforce the old rule for a pair,
    // and is gone below; a pair now closes all the way when the heavy-tailed
    // draw takes it there, which is the one shape a cave photograph is
    // built on. The only remaining constraint on any half's length is
    // physical -- it cannot exceed the open span it is drawn into.
    let mut speleo = vec![0u8; env.area()]; // 0 none, 1 stone, 2 crystal
    let mut speleo_cells = 0usize;
    // Base-width census for `VaultReport` below. `cave_probe` has no width
    // instrument of its own -- its formation silhouette test only ever
    // recognises a column with void on *both* immediate flanks, so a wide
    // cone's base rows are invisible to it by construction (see the
    // A3-1 finding) -- so this pass grades its own homework instead of
    // asking a ruler that cannot see the quantity.
    let mut widths: Vec<i32> = Vec::new();
    {
        let mut last: Option<i32> = None;
        // Indexed rather than iterated: `floor[i]` is only one of the things
        // this body needs `i` for -- `dx` and `px` are derived from it, and
        // the neighbour lookups that once justified the index are gone (the
        // cone draws against `void` now). Clippy's `enumerate()` suggestion
        // would bind an item nothing reads.
        #[allow(clippy::needless_range_loop)]
        for i in 0..floor.len() {
            let dx = i as i32 - env.half_w;
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
            for dy in -env.half_h..=env.half_h {
                if void[env.idx(dx, dy)] {
                    if open_top.is_none() {
                        open_top = Some(dy);
                    }
                } else if let Some(top) = open_top.take() {
                    runs.push((top, dy - 1));
                }
            }
            if let Some(top) = open_top {
                runs.push((top, env.half_h));
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
                // **4x for the 4x world** (Phase 2), and the reason is the
                // cone rather than the count. A formation's trunk length is
                // drawn from this span (`lt` below) while its base width is
                // drawn from `SPELEO_WIDTH_*`, which Phase 2 took to 12-32
                // -- so a 5-row cavity would now hold a 32-cell-wide, 3-row
                // pancake. Every gate in this block that reads `span` moves
                // with the envelope for the same reason.
                if span < 20 {
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
                let chance = SPELEO_DENSITY * 4.0 * wet * noise::smoothstep(12.0, 20.0, span as f32);
                if noise::unit(seed, Purpose::Speleothem, px, ry) >= chance {
                    continue;
                }
                let kind = noise::unit(seed, Purpose::Speleothem, px, ry + 1);
                let crystal = noise::unit(seed, Purpose::Speleothem, px, ry + 2) < SPELEO_CRYSTAL;
                let pair = kind < SPELEO_PAIR && span >= 28;
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
                    // Round 6's A3: no forced gap any more (see this
                    // block's own header comment) -- shrink only as far as
                    // physically necessary to fit inside the open span.
                    while lt + lg > span {
                        if lt >= lg {
                            lt -= 1;
                        } else {
                            lg -= 1;
                        }
                    }
                }
                // A trunk shorter than this is dropped rather than drawn: at
                // the 4x base width a two-row stub is a wide flat lump, not
                // a formation. 4x round 7's 2, for the same reason the span
                // gate above moved.
                if lt < 8 {
                    lt = 0;
                }
                if lg < 8 {
                    lg = 0;
                }
                if lt == 0 && lg == 0 {
                    continue;
                }
                let mat = if crystal { 2u8 } else { 1u8 };
                let mut put = |gx: i32, gy: i32| {
                    if gx.abs() <= env.half_w && gy.abs() <= env.half_h && void[env.idx(gx, gy)] {
                        speleo[env.idx(gx, gy)] = mat;
                    }
                };
                for y in t..t + lt {
                    put(dx, y);
                }
                for y in (fs - lg + 1)..=b {
                    put(dx, y);
                }
                // ---- the cone: a real taper, not one stepped secondary ----
                // Round 6's A3. Every formation with a bottommost run draws
                // a base width -- not a minority chance of one extra column
                // -- and every offset out from the centre gets a height
                // scaled down linearly with distance: `height(o) =
                // height(0) * (1 - |o| / reach)`, a true cone in cross-
                // section that only reaches its full drawn width at the
                // very foot and narrows continuously to the ordinary
                // 1-wide trunk at the tip, rather than round 5's single
                // step at a fixed fraction of the height.
                //
                // The half-width is clamped to `(min_spacing - 1) / 2`
                // *before* it is used for anything -- this is what
                // guarantees two neighbouring footprints can never touch,
                // by construction, so there is no "two independent writers
                // jointly seal a shared neighbour" case left to catch after
                // the fact (round 5's fully-covered-run reopen, removed
                // below: see that block's own comment for why). At
                // `SPELEO_SPACING_MIN` (9) the clamp is already `(9-1)/2 =
                // 4`, `SPELEO_WIDTH_MAX`'s own half -- so this cap is
                // saturated across the whole spacing range, not adaptive
                // within it, and exists purely as the overlap proof: two
                // centres are guaranteed >= 9 apart and no footprint can
                // reach further than 4, so `9 > 4 + 4` holds everywhere.
                // Kept as `(min_spacing - 1) / 2` rather than a bare
                // constant so the proof stays visible at the call site and
                // survives a future change to either spacing bound without
                // silently losing the guarantee.
                //
                // **Every gallery, not just the bottommost one.** Round 5
                // confined the secondary column to `is_bottom` because its
                // alignment test looked the neighbour up in `floor[]`, which
                // holds each column's *bottommost* run and says nothing
                // about an upper one. A3 inherited the gate along with the
                // lookup. Neither survives drawing against the void
                // directly (see the cone's own comment below), and the gate
                // was excluding the formations that matter most: probing one
                // wetland system, only 7 of ~20 placements were `is_bottom`,
                // and every trunk over 8 cells long -- the ones that carry
                // the silhouette -- was in the excluded class, left one
                // pixel wide.
                {
                    let half_cap = ((min_spacing - 1) / 2).clamp(1, SPELEO_WIDTH_MAX / 2);
                    let bw = (SPELEO_WIDTH_MIN
                        + (noise::unit(seed, Purpose::Speleothem, px, ry + 6).powf(1.5)
                            * (SPELEO_WIDTH_MAX - SPELEO_WIDTH_MIN) as f32) as i32)
                        .min(2 * half_cap + 1);
                    let half_l = (bw / 2).min(half_cap);
                    let half_r = (bw - 1 - bw / 2).min(half_cap);
                    widths.push(half_l + half_r + 1);
                    let reach = half_l.max(half_r) as f32 + 1.0;
                    // **The cone runs the full length of the trunk, and it
                    // is drawn against the void, not against a neighbour's
                    // bookkeeping.**
                    //
                    // A3 as first written capped the flare to 22% of trunk
                    // height and only ran it for `is_bottom` placements,
                    // looking each neighbour column up in `floor[]` -- the
                    // *bottommost* run -- and requiring its ceiling to sit
                    // within one cell of this one. Three gates, and between
                    // them the mechanism almost never fired: probing every
                    // placement in one wetland system, only 7 of ~20 were
                    // `is_bottom` at all, and the tall visible ones (trunks
                    // of 16, 14 and 8 cells) were every one of them in the
                    // excluded class. Taking the flare from 22% to 100% of
                    // trunk height moved the whole system by **28 cells**,
                    // which is the tell: the cone was being gated off, not
                    // scaled down. Meanwhile `widths` recorded the *drawn*
                    // base width, so `vaults detail` reported a median of 5
                    // for formations that were one pixel wide on screen --
                    // a ruler measuring the intention instead of the
                    // artifact.
                    //
                    // None of those gates has a reason left. `put` already
                    // refuses any cell that is not void, so drawing an
                    // offset column straight down from the trunk's own
                    // ceiling row and letting it clip against the rock
                    // *is* the neighbour-alignment rule, exactly, for every
                    // gallery rather than the bottom one -- with the break
                    // below stopping the run at the first solid row so a
                    // cone can never jump a rock band into the cavity
                    // beneath. The old comment argued the flare should stay
                    // short to keep `cave_probe`'s formation-height stat
                    // readable, since a cone's flanking rows are invisible
                    // to a silhouette test that wants void on both sides.
                    // That is shaping the rock to please the ruler, which
                    // is the wrong way round; the height stat is fixed
                    // separately and the measured p90 drop it predicts is
                    // an artifact of the probe, not of the cave.
                    let cone = |l: f32, frac: f32| ((l * frac) as i32).max(1);
                    for o in -half_l..=half_r {
                        if o == 0 {
                            continue;
                        }
                        let frac = 1.0 - (o.unsigned_abs() as f32) / reach;
                        if frac <= 0.0 {
                            continue;
                        }
                        let solid = |gx: i32, gy: i32| {
                            gx.abs() > env.half_w || gy.abs() > env.half_h || !void[env.idx(gx, gy)]
                        };
                        // **Each offset starts at its own ceiling, not at
                        // the trunk's.** Starting every column of the cone
                        // at the trunk's own top row `t` reproduced the
                        // alignment gate it replaced, in a quieter costume:
                        // a real ceiling slopes, so one column over the rock
                        // often sits a row lower, `solid` was true at `t`,
                        // and the run broke before writing a cell. Measured
                        // at the render: the flare appeared on the two or
                        // three formations whose ceiling happened to be
                        // level and on none of the rest.
                        //
                        // So walk down (up) a few rows to find where this
                        // column's own rock actually is, and hang from
                        // there. The search is bounded by the slope a
                        // ceiling can plausibly have across one column;
                        // finding nothing means this column is not part of
                        // the same cavity and the offset is skipped.
                        let anchor = |from: i32, step: i32| -> Option<i32> {
                            (0..=CONE_ANCHOR_SEARCH)
                                .map(|d| from + d * step)
                                .find(|&y| !solid(dx + o, y))
                        };
                        if lt > 0 {
                            if let Some(y0) = anchor(t, 1) {
                                for y in y0..y0 + cone(lt as f32, frac) {
                                    if solid(dx + o, y) {
                                        break;
                                    }
                                    put(dx + o, y);
                                }
                            }
                        }
                        if lg > 0 {
                            if let Some(y0) = anchor(b, -1) {
                                for y in ((y0 - cone(lg as f32, frac) + 1)..=y0).rev() {
                                    if solid(dx + o, y) {
                                        break;
                                    }
                                    put(dx + o, y);
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

    // ---- round-5 task 5: waterline formations ----
    // Criterion 5's readable half: a formation standing *in* the pool,
    // breaking the surface, with its crystal minority raised so its glow
    // -- already spilling across water by construction -- has something
    // lit to spill onto. Today's ordinary placement does not target this
    // at all: a stalagmite is drawn for height independent of where the
    // waterline sits, so it either stands high and dry above a flooded
    // floor or, when it does reach the water, is drawn no more often and
    // no more crystalline than any other column -- measured at 0-9
    // formations at a waterline over 16 *seeds*, not per flooded system.
    //
    // A second, targeted pass over columns the main one already
    // considered: only where the floor sits a handful of cells *below*
    // the waterline (so a stalagmite reaching up from it can actually
    // break the surface rather than standing fully submerged with
    // nothing showing, or fully dry with nothing to break), sized to
    // guarantee it clears the surface by at least one cell rather than
    // leaving that to the ordinary heavy-tailed draw, which has no reason
    // to know where the water is. `speleo[idx] == 0` guards every write
    // so this never overwrites a formation the main pass already placed.
    if water_line != i32::MAX {
        let mut last_wl: Option<i32> = None;
        for (i, slot) in floor.iter().enumerate() {
            let Some((t, b, h)) = *slot else { continue };
            let dx = i as i32 - env.half_w;
            let fs = b - h;
            if fs < water_line || fs - water_line > WATERLINE_FLOOR_REACH {
                continue;
            }
            // 4x for the 4x world (Phase 2): two columns apart was a real
            // gap when a formation was 3 cells wide and is an overlap now
            // that it is 12-32. The main pass's own overlap proof is
            // `SPELEO_SPACING_MIN`-derived; this is the same guarantee for
            // the targeted pass, which does not go through it.
            if last_wl.is_some_and(|l| dx - l < SPELEO_WIDTH_MAX) {
                continue;
            }
            let span = fs - t + 1;
            // Enough room to root on rock below and still break the
            // surface by at least one cell. This is the bar most
            // candidates miss (measured, `wetland` s1-8: of 4-39 columns
            // whose floor is within reach, only 0-6 have enough span to
            // actually clear it) -- most of a system's length is ordinary
            // passage at the task-2 lattice scale, span 5-8, and a floor
            // several cells under the table needs more headroom than that
            // to break the surface at all. See the round-5 finding.
            let need = fs - water_line + 1;
            if span < need + 1 {
                continue;
            }
            let px = cx + dx;
            if noise::unit(seed, Purpose::Speleothem, px, 200) >= WATERLINE_CHANCE {
                continue;
            }
            let lg = need.min(span - 2).max(2);
            let crystal = noise::unit(seed, Purpose::Speleothem, px, 201) < WATERLINE_CRYSTAL;
            let mat = if crystal { 2u8 } else { 1u8 };
            for y in (fs - lg + 1)..=b {
                let idx = env.idx(dx, y);
                if void[idx] && speleo[idx] == 0 {
                    speleo[idx] = mat;
                }
            }
            last_wl = Some(dx);
        }
    }

    // **Round 5's "verify, then repair" fully-covered-run reopen is gone.**
    // It existed because two *different* primary formations could each
    // reach into the same shared neighbour from opposite sides -- each
    // leaving its own share of that neighbour's run clear, and still
    // between them covering the whole thing once unioned -- and no single
    // arithmetic clamp covered every combination of independent writers, so
    // it checked the written state directly and reopened the middle cell
    // of any run that came out fully solid. Round 6's A3 removes the
    // scenario it guarded against instead of re-deriving the check for it:
    // every footprint's half-width is now clamped to guarantee it can never
    // reach a neighbouring footprint at all (see the cone's own comment
    // above), so there is no "two independent writers jointly seal a
    // shared column" case left to happen, deliberately or otherwise. Kept
    // reopening anyway, this check would have undone A3's whole point --
    // it cannot tell an accidental seal from a true column doing exactly
    // what the owner asked for.
    //
    // ---- round-5 task 4c: one fused column, in a chamber only ----
    // A *guaranteed* bridged column, off-centre in the system's largest
    // chamber: criterion 2's money shot (a stalactite and stalagmite grown
    // into one another) and criterion 1's monumental anchor, drawn every
    // time rather than left to the ordinary mechanism's own chance. Round
    // 6's A3 makes an ordinary pair bridging elsewhere a legitimate,
    // occasional outcome too (see this block's own header comment) -- this
    // is no longer the *only* place it can happen, but it is still the one
    // guaranteed to. Placed off the run's own centre line -- a third or
    // two-thirds of the way across, drawn per system -- so it does not
    // stand in the one spot
    // a player crossing the chamber would walk through anyway.
    //
    // Rooted exactly like every other formation here: written from the
    // stone under the floor upward, through any gravel, to the stone
    // ceiling above -- "structurally trivial, rooted in the massif" means
    // rooted on rock at both ends, and a column spanning solid-to-solid is
    // *more* attached support than a stalactite or stalagmite alone, never
    // less, so it cannot be less safe than what the seal already allows.
    if let Some(&(start, end)) = chamber_runs.iter().max_by_key(|&&(s, e)| e - s) {
        let width = end - start;
        // A third in from whichever side the draw picks -- off-centre by
        // construction, never the run's own middle column.
        let side = noise::unit(seed, Purpose::Speleothem, cx, -3) < 0.5;
        let frac = 0.28 + noise::unit(seed, Purpose::Speleothem, cx, -4) * 0.12;
        let offset = ((width as f32 * frac) as usize).clamp(1, width.saturating_sub(1).max(1));
        let i = if side { start + offset } else { end - 1 - offset };
        if let Some((t, b, _)) = floor[i] {
            let dx = i as i32 - env.half_w;
            let crystal = noise::unit(seed, Purpose::Speleothem, cx, -5) < SPELEO_CRYSTAL;
            let mat = if crystal { 2u8 } else { 1u8 };
            for y in t..=b {
                if void[env.idx(dx, y)] {
                    speleo[env.idx(dx, y)] = mat;
                }
            }
        }
    }

    let mut written = 0;
    let mut water_cells = 0usize;
    let mut passage_cells = 0usize;
    for dy in -env.half_h..=env.half_h {
        for dx in -env.half_w..=env.half_w {
            if !void[env.idx(dx, dy)] {
                continue;
            }
            let (px, py) = (cx + dx, cy + dy);
            let formation = speleo[env.idx(dx, dy)];
            let gravel =
                matches!(floor[(dx + env.half_w) as usize], Some((_, b, h)) if h > 0 && dy > b - h);
            let cell = if formation == 2 {
                // The crystal minority: the same material as a vug lining,
                // attached like the rock it grows from.
                Cell::new(ctx.spar, loose_shade(ctx, Purpose::Vault, px, py)).with_attached(true)
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
                Cell::new(ctx.flowstone, loose_shade(ctx, Purpose::Speleothem, px, py)).with_attached(true)
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
                if !chamber_col[(dx + env.half_w) as usize] {
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
        formation_widths: widths,
        // Set by the caller (`vaults`), which times the whole pass rather
        // than any one system.
        build_ms: 0.0,
    }
}

/// Minimum columns between two springs. A world that spent its whole budget
/// on one escarpment would read as a leak, not as a country with rivers in
/// it -- and every column along an escarpment is its own `cliff_edges`
/// candidate, so without this the budget goes to six adjacent faces.
const SPRING_SPACING: i32 = 900;

/// How far downhill the pass walks looking for the basin its fall feeds.
///
/// This is the *drain*'s reach, and the drain is the whole point of placing
/// one: without a sink the pool rises until it drowns its own outlet and the
/// river stops being a river. `PLAN.md` asks for "a real source ... and a
/// real sink"; this is how far the sink is allowed to be from the source.
///
/// Bounded rather than global, and the bound is what keeps this a
/// finite-margin pass. `viewshot`'s hand-placed spring drains at the *world's*
/// lowest column, which is a global read and also does not work: in `ascii`'s
/// river-cost scene the drain sits 2030 columns from the outlet and the
/// ledger reports `drained 0` after 1400 frames -- the water never gets
/// there. Placed within reach instead, the same mechanism drains 83% of what
/// it emits.
const SPRING_DRAIN_REACH: i32 = 150;

/// The source basin cut into the shelf behind the lip: how wide it may run,
/// the least shelf that will take one, and how deep the bowl goes at its
/// middle.
///
/// Set from a census over six seeds and three presets. The shelf -- ground
/// behind the rim standing at or above the lip -- runs a median 107 columns on
/// `canyon` and 120 on `rolling` and `terraced` (the probe's search cap, so
/// those are floors), and 73-94% of rims carry at least 16 columns of it, so
/// the minimum below is comfortable rather than binding. The depth is set
/// against the gnome, who is 14 rows tall: a 12-deep pool reads as water he
/// could stand in up to the chest, which is a tarn rather than a puddle.
const SPRING_BASIN_W: i32 = 40;
const SPRING_BASIN_MIN_W: i32 = 12;
const SPRING_BASIN_DEPTH: i32 = 12;

/// How far above the lip the basin's ground may stand. Small on purpose: see
/// the shelf comment in `springs` for the trench this stops.
const SPRING_BASIN_RIM: i32 = 8;

/// Columns of untouched ground the basin keeps either side of itself.
const SPRING_BASIN_CLEARANCE: i32 = 3;

/// How far above the lip -- and so above the pool's own surface, which the lip
/// pins -- the outlet sits. Clear of the water for good, for the two reasons
/// in the seating comment in `springs`: a drowned outlet stops emitting, and a
/// partly-wet one stalls without even reporting a throttle.
const SPRING_SOURCE_LIFT: i32 = 8;

/// How deep the pool at the foot of a fall stands before it starts draining
/// away, in cells. The drain sits this far above the basin floor, and the
/// pool self-levels there -- see the seating comment in `springs`.
///
/// Sized to be seen rather than to be right: `render.rs` dims a liquid by
/// fill and `ponds` already refuses pools too shallow to read as water at
/// all, so a plunge pool that is technically present and two cells deep buys
/// nothing the owner asked for.
pub const SPRING_POOL_DEPTH: i32 = 10;

/// How far a spring stays clear of either world edge.
const SPRING_EDGE_MARGIN: i32 = 64;

/// Air below the outlet needed before a face counts as a fall rather than a
/// damp patch. Cheap insurance against the failure mode `World::add_spring`
/// cannot catch: it validates nothing about position, so an outlet inside
/// ground is not an error -- it is permanently `walled`, emits zero forever,
/// and shows only as a rising `throttled` count.
const SPRING_MIN_AIR: i32 = 12;

/// Columns beyond the ones it decides for that `springs` reads.
///
/// `RUN_FAR` for the same cliff detection `brows` and `talus` do, `MAX_FALL`
/// down to the foot of the face, then `SPRING_DRAIN_REACH` along the falling
/// side for the basin -- the deepest of the three, and they compose rather
/// than overlap, so the sum is the honest bound. `MAX_SPAN` covers the
/// emission columns themselves hanging off the rim.
pub const SPRINGS_MARGIN: i32 =
    RUN_FAR + MAX_FALL + SPRING_DRAIN_REACH + crate::sim::spring::MAX_SPAN;

/// Topmost occupied cell of a column in the built world, or `h` if the column
/// is empty all the way down. The plan's `surface_y` is not this: `talus`,
/// `brows`, `residuals` and `ponds` all write after it.
fn world_top(world: &World, x: i32, h: i32) -> i32 {
    (0..h).find(|&ty| world.get(x, ty).material != material::EMPTY).unwrap_or(h)
}

/// Where a fall's water ends up, as drain positions: the plunge pool at its
/// foot, and the next low ground its overflow runs on to.
///
/// **Public, and shared with the harnesses, because getting this wrong is
/// silent.** A drain in the wrong column is not an error anywhere -- the
/// ledger just reports `drained 0` and the scene fills like a bath while
/// still printing a "standing" frame cost. Both harnesses had their own copy
/// of the rule and both had the same bug: `viewshot spring=` and `ascii`'s
/// river-cost scene each drained at the *world's* lowest column, which in
/// that scene lands 2030 columns from the outlet. One rule, one place.
///
/// Nested reaches, because "where the water ends up" is not one place: a fall
/// makes a plunge pool at its foot, and what that pool overflows runs on to
/// the next low ground. Which of the two a given world's water actually
/// settles in depends on terrain nobody is going to simulate here. Draining
/// both is cheap -- a drain only ever removes work, so there is no budget on
/// them the way there is on springs -- and it is the difference between a
/// river and a rising bath. Seed 7 emitted 4.2M fill units into a
/// single-reach drain and returned `drained 0`.
///
/// The height is [`SPRING_POOL_DEPTH`] above the basin floor, **not on it**.
/// The drain's *height* is what decides whether a pool stands, and the rate
/// is not: `spring::step` takes only from a drain cell that currently holds
/// a liquid, so a drain above the waterline is inert. Nothing leaves until
/// the pool has risen to it, and then it takes at most `DRAIN_FILL` per
/// frame -- which equals `EMIT_FILL`, so one drain balances one emission
/// column and the pool settles *at the drain's height* with the throughput
/// passing through. The first version seated drains one cell above the floor,
/// which is this with the pool depth set to zero: it took the water as fast
/// as it landed and nothing ever stood at the bottom. The owner, shown it:
/// *"it looks like it comes from nowhere and goes nowhere ... Ideally it
/// should also end in a pool."*
pub fn spring_drains(world: &World, from_x: i32, dir: i32, span: i32, w: i32, h: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    let mut seen: Vec<i32> = Vec::new();
    for reach in [MAX_FALL / 3, SPRING_DRAIN_REACH] {
        let mut basin = from_x;
        for d in 1..=reach {
            let x = (from_x + dir * d).clamp(0, w - 1);
            if world_top(world, x, h) > world_top(world, basin, h) {
                basin = x;
            }
        }
        if seen.contains(&basin) {
            continue;
        }
        seen.push(basin);
        let floor = world_top(world, basin, h);
        for d in 0..span {
            let dx = (basin + d - span / 2).clamp(0, w - 1);
            // Follow the basin's own floor where it is lower than the centre
            // column's, so a drain never ends up buried in a bank that
            // happens to be higher than where the pool bottoms out.
            let dy = world_top(world, dx, h).min(floor) - SPRING_POOL_DEPTH;
            if dy >= 0 {
                out.push((dx, dy));
            }
        }
    }
    out
}

/// Springs where the water table daylights on a cliff face, each with a drain
/// in the basin its fall feeds.
///
/// **The pass owns all geometric validity.** `World::add_spring` validates
/// nothing about position (`world.rs`) -- its only rejections are a span
/// outside `1..=MAX_SPAN` and the summed-span budget, and
/// `add_spring(-9999, -9999, 1)` returns `true`. An outlet seated in rock is
/// therefore not an error anywhere downstream; it is a spring that emits
/// nothing for the life of the world and reports it only as a climbing
/// `throttled` count. So every candidate is checked against the *finished*
/// world before it is registered, and a failure moves to the next candidate
/// rather than aborting.
///
/// Runs after `ponds`, because the drowned-spring throttle reads the standing
/// pool level and `ponds` writes into EMPTY only; and before `soil_moisture`,
/// because that pass builds its saturated zone from the liquid cells actually
/// present and its own doc warns that a pool over dry soil "spends its
/// opening minutes drinking its own bed and banks".
///
/// Returns 0 cells always: it registers emitters, it does not write terrain.
/// `every_pass_writes_something` asserts that explicitly rather than skipping
/// the pass, for the reason `vaults` and `boulders` get the same treatment --
/// an exclusion that stops being true should fail loudly.
pub fn springs(ctx: &Ctx, world: &mut World) -> usize {
    let p = ctx.terrain.params;
    let mut budget = p.spring_flow.round() as i32;
    if budget <= 0 {
        return 0;
    }
    let (w, h) = (ctx.terrain.w, ctx.terrain.h);
    let plans = &ctx.plans;
    let seed = ctx.terrain.seed;

    // **Scan from a seed-dependent origin, wrapping.** `cliff_edges` returns
    // candidates in x order, and taking the first that qualifies put every
    // world's waterfall in its first thousand columns -- measured across six
    // canyon seeds at x = 1, 392, 409, 505, 873 and 1035, in a world 8192
    // wide, with one of them literally against the world edge. Rotating the
    // scan costs nothing, stays a pure function of the seed, and does not
    // spend any of the scarce candidates the way a sparse acceptance draw
    // does (that was tried: it cut placement from 1.0 springs per world to
    // 0.2, because after the real gates a world offers only a handful).
    let origin = (noise::unit(seed, Purpose::Spring, 0, 0) * w as f32) as i32;
    let mut candidates = cliff_edges(plans, w);
    let split = candidates.partition_point(|&(x, _, _)| x < origin);
    candidates.rotate_left(split);

    let mut placed_at: Vec<i32> = Vec::new();
    // Why candidates are refused, printed under `SPRING_DEBUG=1`. Kept
    // because placement here is a chain of gates over the *built* world, and
    // "0 springs placed" is the same output for six different causes -- three
    // successive models were told apart only by these counts. `probe_p1_
    // where_can_a_spring_go` is the probe that reads them.
    let (mut n_cand, mut n_spacing, mut n_soil) = (0usize, 0usize, 0usize);
    let (mut n_table, mut n_edge, mut n_blocked) = (0usize, 0usize, 0usize);
    let (mut n_shelf, mut filled) = (0usize, 0usize);
    let mut n_placed = 0usize;
    for (rim, dir, _drop) in candidates {
        n_cand += 1;
        if budget <= 0 {
            break;
        }
        if placed_at.iter().any(|&px| (rim - px).abs() < SPRING_SPACING) {
            n_spacing += 1;
            continue;
        }
        let plan = plans[rim as usize];
        // A loose Powder top is not rock for water to weep from -- the same
        // test `brows` makes, and for the same reason.
        if plan.soil_depth > 0 {
            n_soil += 1;
            continue;
        }
        // The dry-preset gate, the same one `moisture_init` uses: `arid` and
        // `flat` put the table past the world floor, so no face intersects it
        // and the pass falls out for free rather than by a special case.
        if plan.table_y >= h {
            n_table += 1;
            continue;
        }
        // The table is a gate on *whether*, not on where -- see the seating
        // comment below for why these are perched springs.
        //
        // An earlier version also required the table to lie between the rim's
        // ground and the foot of the face, the literal reading of "the aquifer
        // daylights here". Dropped, because it is the wrong question once the
        // outlet is perched, and it was doing real damage: it rejected 65-92%
        // of every preset's candidates and **all** of `canyon`'s, which ships
        // `table_offset: 70` and so keeps its table below even its valley
        // floors. Canyon is the preset with the best waterfall faces in the
        // game; a gate that switches it off entirely was measuring the wrong
        // thing.
        if plan.table_y <= plan.surface_y {
            n_table += 1;
            continue;
        }
        let span = budget.min(crate::sim::spring::MAX_SPAN);
        // **The source pool is cut, not found.**
        //
        // Looking for one is a recorded dead end (`Reports/dead-ends.md`): for
        // a basin to spill over *this* cliff, this cliff's lip has to be the
        // basin's lowest exit, and requiring that placed zero springs across
        // four presets and six seeds. It is not a tuning failure -- a cliff
        // edge is a local high point, so the ground behind it rises.
        //
        // That same fact is what makes cutting one work. The ground behind the
        // lip standing *at or above* it is exactly the back wall a pool needs,
        // so a basin cut into that shelf is closed by construction and spills
        // forward over the cliff rather than running away inland. Measured
        // over six seeds: the shelf runs a median 107 columns on `canyon` and
        // 120 on `rolling` and `terraced` (the probe's search cap, so those
        // are floors), and 73-94% of rims carry at least 16 columns of it.
        let lip = plans[rim as usize].surface_y;
        let back = -dir;
        // The shelf: ground standing at or above the lip. Carve the whole of
        // it up to `SPRING_BASIN_W`.
        //
        // No back wall is required, and requiring one placed **nothing** on
        // any preset: the ground behind a rim is *flat at the lip*, not
        // rising, so a column standing strictly above the lip within a
        // basin's width essentially never occurs. It is not needed either --
        // water filled to the lip has two ways out, the cliff at distance 0
        // and the far end of the shelf a hundred columns away, and it reaches
        // the cliff first. That is the whole mechanism.
        // **Level ground, not merely ground above the lip.** The shelf test was
        // `surface_y <= lip`, which admits ground *well* above it -- and since
        // the cut clears each column from the sky down to the bowl floor, a
        // basin sited on rising ground is a sheer trench gouged through a
        // hillside. Shown one, the owner: *"a weird cut through a sharp piece
        // of stone"*. So the ground has to be within `SPRING_BASIN_RIM` rows
        // of the lip for its whole width: the cut is then shallow everywhere
        // and reads as a hollow in flat ground rather than as an excavation.
        let mut shelf = 0;
        while shelf < SPRING_BASIN_W && {
            let g = plans[(rim + back * (shelf + 1)).clamp(0, w - 1) as usize].surface_y;
            g <= lip && g >= lip - SPRING_BASIN_RIM
        } {
            shelf += 1;
        }
        if shelf < SPRING_BASIN_MIN_W {
            n_shelf += 1;
            continue;
        }
        let (near, far) = (rim + back, rim + back * shelf);
        let (bx0, bx1) = (near.min(far), near.max(far));
        if bx0 < SPRING_EDGE_MARGIN || bx1 >= w - SPRING_EDGE_MARGIN {
            n_edge += 1;
            continue;
        }

        // The bowl floor: deepest in the middle, tapering to nothing at both
        // ends, with a little wobble so it is a tarn and not a quarry. Held as
        // a closure over the plan so the volume can be inspected before a
        // single cell is removed.
        let floor_at = |x: i32| -> i32 {
            let t = (x - bx0) as f32 / ((bx1 - bx0).max(1)) as f32;
            let bowl = 1.0 - (2.0 * t - 1.0).powi(2);
            let wobble = noise::value_1d(seed, Purpose::Spring, x as f32 / 9.0) - 0.5;
            lip + ((SPRING_BASIN_DEPTH as f32) * bowl + wobble * 3.0).round().max(1.0) as i32
        };

        // **Refuse anything that is not ordinary ground**, before removing a
        // cell. The rest of this file keeps a never-overwrite-a-sealed-feature
        // contract and this is that contract: crystal, flowstone, spar and
        // standing water all mean some other pass has already authored here.
        // It is also what keeps the vault seal's `assert_eq!` out of play --
        // chambers sit `vault_min_depth` (200) rows down and this cuts at the
        // surface, so the two should never meet, and this check is what makes
        // that a guarantee rather than an expectation.
        let ordinary = |m: material::MaterialId| {
            m == material::EMPTY || m == ctx.stone || m == ctx.soil || m == ctx.sand || m == ctx.gravel
        };
        // The flanks are checked too, not just the carve volume. A basin cut
        // hard against an existing pond merges with it, and two pools at
        // different levels touching is a head difference: the water flows,
        // the world is not at rest, and `every_pool_has_a_level_surface`
        // reports a surface that "steps from 176 to 163 between x 406 and
        // 407" -- which is exactly what it caught here.
        let clean = (bx0 - SPRING_BASIN_CLEARANCE..=bx1 + SPRING_BASIN_CLEARANCE).all(|bx| {
            let bx = bx.clamp(0, w - 1);
            let in_carve = bx >= bx0 && bx <= bx1;
            if in_carve {
                return plans[bx as usize].soil_depth == 0
                    && (0..=floor_at(bx)).all(|by| by >= h || ordinary(world.get(bx, by).material));
            }
            // On the flanks, only water matters, and only water near this
            // pool's own level -- a pond far below in the same canyon cannot
            // merge with it, but one within a bowl-depth of the lip can. The
            // first version checked a fixed depth from the lip and bottomed
            // out one row short of the pond that actually merged.
            let band = SPRING_BASIN_DEPTH + SPRING_BASIN_CLEARANCE;
            ((lip - band).max(0)..=(lip + band).min(h - 1))
                .all(|by| world.get(bx, by).material != ctx.water)
        });
        if !clean {
            n_blocked += 1;
            continue;
        }

        // Cut it. Every column is cleared from the top of the world down to
        // its bowl floor, so nothing is ever left overhanging -- material only
        // comes off from above, which is what makes the carve structurally
        // safe by construction rather than by a check afterwards.
        for bx in bx0..=bx1 {
            let floor = floor_at(bx);
            for by in 0..floor {
                if world.get(bx, by).material != material::EMPTY {
                    world.set(bx, by, Cell::EMPTY);
                }
            }
            // Then fill to the lip. `ponds`' convention verbatim: `aux` is
            // left alone because on a `Liquid` `aux == 0` means **full**, and
            // writing a literal here is the documented way to manufacture
            // water out of nothing.
            for by in lip..floor {
                world.set(bx, by, Cell::new(ctx.water, loose_shade(ctx, Purpose::Shade, bx, by)));
                filled += 1;
            }
        }

        // **The outlet sits clear above the pool it feeds.** Two mechanisms
        // measured on the way here, both recorded in `dead-ends.md`: a spring
        // emitting into the pool it is filling switches *itself* off, because
        // `spring::step` counts an outlet drowned at `THROTTLE_FILL` (90% of a
        // cell); and an outlet holding *partly* filled water neither emits nor
        // counts as throttled, so it stalls silently while the ledger still
        // looks healthy. Above the lip the outlet stays in air for good,
        // because the pool cannot rise past the lip -- it spills there
        // instead, which is the whole mechanism.
        let y = lip - SPRING_SOURCE_LIFT;
        let mid = (bx0 + bx1) / 2;
        let x0 = (mid - span / 2).clamp(0, w - span);
        if y < 0 || y >= h {
            n_edge += 1;
            continue;
        }
        let outlet_clear = (0..span).all(|d| world.get(x0 + d, y).material == material::EMPTY);
        if !outlet_clear {
            n_blocked += 1;
            continue;
        }
        // And a real drop on the other side for the overflow to fall down.
        if (1..=RUN_FAR)
            .map(|d| world_top(world, (rim + dir * d).clamp(0, w - 1), h))
            .max()
            .unwrap_or(lip)
            < lip + SPRING_MIN_AIR
        {
            n_blocked += 1;
            continue;
        }
        let ex = (rim + dir).clamp(0, w - 1);
        if !world.add_spring(x0, y, span) {
            // The engine's own budget refused it. Nothing further will fit
            // either, since every remaining span is at least this wide.
            break;
        }

        // **The sink, in the plunge pool -- not at the world's low point.**
        // The rule, and the long account of how each half of it was got
        // wrong first, live in `spring_drains` -- shared with the harnesses,
        // which each had their own broken copy of it.
        for (dx, dy) in spring_drains(world, ex, dir, span, w, h) {
            world.add_drain(dx, dy);
        }
        budget -= span;
        placed_at.push(rim);
        n_placed += 1;
    }
    if std::env::var("SPRING_DEBUG").is_ok() {
        println!(
            "  springs: {n_cand} cliff candidates -> refused {n_spacing} too close, {n_soil} soil-topped, \
             {n_table} no groundwater, {n_shelf} no shelf to cut into, {n_edge} at the world edge, \
             {n_blocked} blocked; PLACED {n_placed} ({filled} cells of source pool)"
        );
    }
    filled
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
    // **A one-byte classification mirror, because this pass reads the world
    // more times than any other and writes almost nothing.**
    //
    // Measured at 8192x2560: 1876 ms to write 37 k cells -- 30% of the whole
    // pass table for 0.2% of its output. The cost was never the writing. The
    // scans below asked `World::get` per cell, and one of them four more
    // times for the neighbours, so a 21 M-cell world paid ~105 M
    // bounds-checked `HashMap<ChunkCoord, Chunk>` lookups with a material
    // registry lookup on each.
    //
    // Everything they ask is a property of the *material*: is it a liquid,
    // and does it hold water. Two bits, resolved once per cell by walking
    // chunks directly -- the same shape `structural::compute_world_distances`
    // uses, and for the same reason.
    const IS_LIQUID: u8 = 1;
    const HOLDS_WATER: u8 = 2;
    let hh = ctx.terrain.h as usize;
    let cls = |x: usize, y: usize| y * w + x;
    let mut class = vec![0u8; w * hh];
    for chunk in world.chunks() {
        let (ox, oy) = chunk.coord.origin();
        for ly in 0..crate::sim::chunk::CHUNK_SIZE {
            for lx in 0..crate::sim::chunk::CHUNK_SIZE {
                let (x, y) = (ox + lx, oy + ly);
                if x < 0 || y < 0 || x >= ctx.terrain.w || y >= ctx.terrain.h {
                    continue;
                }
                let m = chunk.get_world(x, y).material;
                let mut bits = 0;
                if world.materials.kind(m) == material::MaterialKind::Liquid {
                    bits |= IS_LIQUID;
                }
                if world.materials.get(m).water_capacity > 0 {
                    bits |= HOLDS_WATER;
                }
                class[cls(x as usize, y as usize)] = bits;
            }
        }
    }

    // Standing water outranks the smoothed table locally: the bed of the
    // lowest water cell caps the effective table from above.
    let mut table: Vec<i32> = (0..w).map(|x| ctx.plans[x].table_y).collect();
    for (x, t) in table.iter_mut().enumerate() {
        for y in 0..ctx.terrain.h {
            if class[cls(x, y as usize)] & IS_LIQUID != 0 {
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
    // Out of bounds is not liquid, which is what `World::get` answered before
    // (`Cell::OUT_OF_BOUNDS` is bedrock); this keeps that reading rather than
    // letting an edge cell wrap to the far side of the world.
    let liquid_at = |x: i64, y: i64| {
        x >= 0 && y >= 0 && x < w as i64 && y < hh as i64 && class[cls(x as usize, y as usize)] & IS_LIQUID != 0
    };
    for y in 0..h {
        for x in 0..w {
            let bits = class[cls(x, y)];
            if bits & IS_LIQUID != 0 {
                d[idx(x, y)] = -1;
            } else if bits & HOLDS_WATER != 0 {
                let wetted = [(0, -1), (-1, 0), (1, 0), (0, 1)]
                    .iter()
                    .any(|&(dx, dy)| liquid_at(x as i64 + dx, y as i64 + dy));
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
            if d[idx(x, y)] > 0 && d[idx(x, y - 1)] <= 0 && class[cls(x, y)] & HOLDS_WATER != 0 {
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
            // Gate on the mirror before touching the world: only ~0.2% of
            // cells hold water, and this loop used to pay a `World::get` and
            // a registry lookup for every one of the other 99.8%.
            if class[cls(x, y)] & HOLDS_WATER == 0 {
                continue;
            }
            let cell = world.get(x as i32, y as i32);
            let capacity = world.materials.get(cell.material).water_capacity;
            if capacity == 0 {
                continue;
            }
            // **The climate baseline is a FLOOR under every arm, not a
            // replacement for the far one**, and getting that wrong was a
            // real defect caught in review before it landed.
            //
            // The arms below are a *capillary fringe*: wettest in the water,
            // drying with distance. Written as a fourth arm, the baseline
            // competed with them instead of underwriting them -- and since
            // the `match` takes the first matching arm, a cell two cells
            // from a pond got the fringe's 240 while a cell fifty columns
            // away got the climate's 570. **A dry ring around every pond,
            // with damper ground beyond it**: non-monotonic and backwards.
            // On `rolling` that is a tree at the water's edge reading 0.14
            // plant-available against 0.65 out in the field.
            //
            // As a floor it composes correctly, and gives the intended
            // behaviour for free: in wet country the profile flattens
            // (1000, 620, 570, 570) because damp ground is damp everywhere,
            // and in dry country the fringe still reads (1000, 620, 240,
            // 50). The gradient stays visible exactly where the climate is
            // dry, which is where it means something.
            let baseline = {
                let wet = 1.0 - ctx.terrain.character(x as i32).aridity;
                let want = material::SOIL_FIELD_CAPACITY as f32 * wet;
                (capacity as f32 * (want / material::SOIL_SATURATED as f32)) as u16
            };
            let target = match d[idx(x, y)] {
                dist if dist <= 0 => capacity,
                1 => (capacity as f32 * fringe_fraction[0]) as u16,
                2 => (capacity as f32 * fringe_fraction[1]) as u16,
                // Beyond the fringe the climate is all there is.
                _ => 0,
            }
            .max(baseline);
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

/// How far a lens's outline departs from the ellipse it is built on, as a
/// fraction of its own radius: a few low harmonics that make it lumpy, and a
/// finer field that roughens the edge cell by cell.
///
/// **The owner's words: *"The ovals of sand throughout the stone looks bad
/// and should be fixed. It should be a more natural shape than perfect
/// ovals."*** An exact rotated ellipse is a *drawn primitive*, and the round-6
/// review rejected the drawn things by name while leaving alone the ones that
/// come out of a process. At the sizes these draw -- a semi-major axis of 8
/// to 60 cells -- the long arc is smooth over tens of cells, and nothing else
/// in the rock has an edge like that.
///
/// Three harmonics rather than one, at 3/5/8, because a single sine is an egg
/// and two is a peanut; odd, non-multiple frequencies stop the outline
/// closing back on any symmetry. Phases are drawn per lens.
///
/// The lobe term is a fraction of *radius*, so it is worth 2-17 cells along
/// the long axis and well under a cell across the short one -- which is the
/// right asymmetry: a lens is 2-12 cells thick, so there is no room for
/// shape across it, and the long smooth arc is the part that reads as drawn.
/// Where the two combine to pull the boundary inside its own centre the lens
/// simply pinches out, which is what a real lens does at its ends.
const LENS_LOBE: f32 = 0.30;
const LENS_GRAIN: f32 = 0.12;
/// Cells per cycle of the edge-grain field.
const LENS_GRAIN_SCALE: f32 = 7.0;

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
    // Clamped at the read rather than trusted from the file: a negative
    // roughness would invert the bound the scan box and the early-out are
    // both derived from, and a lens could then be written outside the region
    // that was seal-checked.
    let rough = p.lens_roughness.max(0.0);
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
                // Grown by the most the outline can bulge, or the scan would
                // clip a lobe -- and a clipped lobe is not merely a smaller
                // lens: its cells would be written without ever being
                // seal-checked, which is the sawn-off-face bug the vault
                // pass records under its own cap.
                let bulge = 1.0 + rough * (LENS_LOBE + LENS_GRAIN);
                let (ea, eb) = ((a + 1.0) * bulge, (b + 1.0) * bulge);
                let ext_x = ((ea * cos_t).abs() + (eb * sin_t).abs()).ceil() as i32 + 1;
                let ext_y = ((ea * sin_t).abs() + (eb * cos_t).abs()).ceil() as i32 + 1;
                // The lens's own outline, drawn once per lens rather than per
                // cell: three seeded phases for the lobes.
                let phase = |k: i32| {
                    noise::unit(seed, Purpose::PocketEdge, cx.wrapping_mul(13).wrapping_add(k), cy.wrapping_mul(7))
                        * std::f32::consts::TAU
                };
                let (p1, p2, p3) = (phase(0), phase(1), phase(2));
                'lens: for dy in -ext_y..=ext_y {
                    for dx in -ext_x..=ext_x {
                        let (px, py) = (cx + dx, cy + dy);
                        // Into the bed's own frame: `u` along the bedding,
                        // `v` across it.
                        let u = dx as f32 * cos_t + dy as f32 * sin_t;
                        let v = -(dx as f32) * sin_t + dy as f32 * cos_t;
                        // The outline, in the ellipse's own normalised frame:
                        // radius 1.0 is the ellipse, and `wobble` moves it.
                        // **The same `wobble` is applied to the lens and to
                        // its rind**, and the rind is the same shape built on
                        // the grown axes -- which is what keeps the seal
                        // sound. For any cell `d_out <= d_in` because
                        // `a + 1 > a`, so every cell inside the lens is
                        // inside the rind by construction, whatever the
                        // outline does. Perturbing the two independently
                        // would let a lobe push the lens through its own
                        // rind and outcrop on a free face, which is the one
                        // failure this pass exists to prevent.
                        // **Reject on the cheap bound before paying for the
                        // outline.** The scan box is 1.42x the ellipse in
                        // each axis now, so most of what it visits is outside
                        // the shape entirely -- and the outline costs an
                        // `atan2`, three `sin`s and a two-octave fBm per
                        // cell. Computing it for cells that cannot possibly
                        // be inside took the pass from 45.9 ms to 244.4 ms.
                        //
                        // Safe because `wobble` is bounded by construction:
                        // the harmonics are normalised to +-1 and scaled by
                        // `LENS_LOBE`, the grain to +-1 by `LENS_GRAIN`, so
                        // nothing can be inside a radius past
                        // `1 + LENS_LOBE + LENS_GRAIN`. A cell rejected here
                        // would have been rejected by the full test.
                        let (nu, nv) = (u / a, v / b);
                        let outer = ((u / (a + 1.0)).powi(2) + (v / (b + 1.0)).powi(2)).sqrt();
                        if outer > 1.0 + rough * (LENS_LOBE + LENS_GRAIN) {
                            continue;
                        }
                        let theta = nv.atan2(nu);
                        let lobe = (0.60 * (3.0 * theta + p1).sin()
                            + 0.30 * (5.0 * theta + p2).sin()
                            + 0.25 * (8.0 * theta + p3).sin())
                            / 1.15;
                        let grain = noise::value_2d(
                            seed,
                            Purpose::PocketEdge,
                            (cx + dx) as f32 / LENS_GRAIN_SCALE,
                            (cy + dy) as f32 / LENS_GRAIN_SCALE,
                        ) - 0.5;
                        let wobble = rough * (LENS_LOBE * lobe + LENS_GRAIN * 2.0 * grain);
                        let d = (nu * nu + nv * nv).sqrt();
                        // The rind: one cell beyond the lens must also be
                        // stone, so the lens is never flush with a free face.
                        if outer > 1.0 + wobble {
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
                        if d <= 1.0 + wobble {
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

/// How far a boulder's socket is allowed to dig through loose cover
/// looking for the real massif before giving up on the whole boulder. A
/// safety bound on the walk, not a behaviour knob -- see `residual.rs`'s
/// constant of the same name and purpose, added for the identical reason.
const MAX_SOCKET_DEPTH: i32 = 80;

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

        // Shape, from a stream keyed on the run's centre column.
        //
        // Round 6 Track B, B3: re-derived from the *real* 3x rule after
        // measuring what round 4 actually shipped
        // (`Reports/worldgen-erosion-design.md`'s 2026-08-20 addendum).
        // Three shrinks had compounded, none of them structural: the design
        // says only `height <= 3x base width`; round 4's task file read
        // that as "2-5 wide, 2-4 tall"; and this clamped tighter still with
        // `height.min(width)`, a 1x ratio where 3x was allowed -- a 12x8
        // boulder is 0.67x and satisfies the real rule with room to spare.
        // Width widened from 2-5 to 3-13 so the 3x ceiling has a base worth
        // multiplying; height now drawn independently up to that ceiling
        // rather than clamped to width. Both draws are skewed toward their
        // own top half (`sqrt`, not a straight `unit`): a marker is a
        // steep-drop site by construction, so the tallest draws are also
        // the ones least likely to find enough open air to seat in --
        // measured directly (`BOULDER_PROBE=1`, reverted before this
        // commit): a uniform draw put visible-height p50 at 4 against a
        // bar of 6, because narrow/short attempts seat far more often than
        // wide/tall ones and dominate the successfully-seated population.
        // Skewing the draw upward is what a uniform draw cannot do on its
        // own reach; the skew is on the *attempt*, not an override of which
        // ones survive collect-verify-write.
        let width = 3 + (noise::unit(seed, Purpose::Boulder, cx, 0).sqrt() * 10.0) as i32;
        let max_height = ((width as f32) * 3.0).round() as i32;
        let height = 2 + (noise::unit(seed, Purpose::Boulder, cx, 1).sqrt() * (max_height - 2).max(1) as f32) as i32;
        let a = width as f32 / 2.0;
        // `b` is the *visible standing height* directly, not halved. Round
        // 4's dome was a full-height ellipse with only the rows *above*
        // `ground_y` written, so a "4 tall" boulder rose `round(4/2)` = 2
        // rows -- arithmetic, not a design choice, and the addendum's
        // measured consequence: seating in 3 of 24 worlds, visible height
        // 1-2 against a 14-cell player. Using `height` as the semi-axis
        // outright means the drawn number is the number a player sees.
        let b = height as f32;
        let reach = (a.ceil() as i32).max(1);
        // How many further rows of *already-bare* rock get recoloured
        // once the socket reaches it, purely for the visual blend -- "a
        // boulder reads as embedded in the rock it seated in, not merely
        // touching it". Proportional to the visible height, floored at 1.
        // This is decoration, not the connectivity fix below: recolouring
        // stone that was already attached and already reachable cannot
        // itself create or break a path to an anchor.
        let visual_blend = ((b * 0.3).ceil() as i32).max(1);

        // Collect first, write only if every proposed cell is a safe
        // target: open air or loose cover (soil/sand/gravel) for the dome,
        // which is displaced rather than skipped -- a boulder resting on
        // top of an untouched talus apron would look like it was dropped
        // there, not eroded out of the rock beneath it. Anything else --
        // the permanent massif reached early, bedrock, water, a vault
        // lining -- rejects the whole boulder, never just the one column,
        // matching `pockets`' all-or-nothing seal.
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
            // Socketed, not perched: dig down through loose cover until
            // this column threads real rock, not a fixed fraction of the
            // dome's own height. A fixed depth was the first version, and
            // it could leave a boulder floating -- on a soil blanket
            // deeper than the fixed socket, the newly-attached seat layer
            // has no *relaxable* path down through the (non-solid) soil to
            // the massif underneath, so it reads solid while never
            // actually reaching an anchor
            // (`Reports/worldgen-implementation-tasks-round6-formations.md`'s
            // B2 finding measured the identical bug in `residual.rs` --
            // same fix, same reasoning, here before it could recur).
            let mut py = ground_y;
            loop {
                let mat = world.get(lx, py).material;
                if mat == ctx.stone {
                    break;
                }
                if mat != ctx.soil && mat != ctx.sand && mat != ctx.gravel {
                    sealed = false;
                    break 'run;
                }
                cells.push((lx, py));
                py += 1;
                if py - ground_y > MAX_SOCKET_DEPTH {
                    sealed = false;
                    break 'run;
                }
            }
            // Past the real rock: a few more rows purely for the visual
            // blend, already-attached and already-reachable so recolouring
            // them cannot change what is or is not anchored.
            for row in 0..visual_blend {
                let bpy = py + row;
                if world.get(lx, bpy).material == ctx.stone {
                    cells.push((lx, bpy));
                } else {
                    break;
                }
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
