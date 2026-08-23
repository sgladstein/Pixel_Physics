//! A generic live-tunables registry — §10 of the plan's UI-improvement
//! pass, built on §9's text primitive. Not a bespoke panel per subsystem:
//! any already data-driven value registers as one `Tunable`
//! `(category, name, value, min, max, step)`, and the panel (`app.rs`'s
//! `draw_tunables_panel`) is generic over whatever's currently registered.
//!
//! **Scope of this pass.** Materials are the only registrant — every
//! finite `f32` field on `Material` (`density`, `friction_angle`,
//! `flammability`, `ignition_temperature`, `burn_temperature`,
//! `heat_conductivity`, `melting_point`, `boiling_point`), plus engine
//! constants that are not material fields at all (see `TunableGroup::
//! Explosion`).
//!
//! **Integer fields.** These used to be excluded on the reasoning that the
//! save path always writes a float literal and building the distinction in
//! for fields nothing exercised would be scope creep. That reasoning was
//! wrong in a way that cost a real bug rather than just an absent feature:
//! `min_transfer` *was* registered anyway (liquids only), so saving it wrote
//! `min_transfer: 60.0` into a `u16` and RON rejected the file with
//! "Expected comma" — reported from live play. `Tunable::integral` now
//! carries the distinction explicitly and `format_value` honours it, so
//! integer fields are registerable and `flow_rate` is registered too.
//! A field left at its "never" sentinel (`f32::INFINITY`
//! — `ignition_temperature`, `burn_temperature`, `melting_point`,
//! `boiling_point`, all default to this per `material.rs`'s own doc) is
//! also skipped: dragging a slider "up" from infinity has no sensible
//! starting point, and registering it would either silently clamp to a
//! misleadingly-finite value or need its own special-cased UI.

use crate::sim::clock::{Clock, MAX_SLOWDOWN};
use crate::sim::explosion::Tuning as Explosion;
use crate::sim::material::{MaterialKind, MaterialRegistry};
use crate::sim::player::Tuning as Player;

/// Which menu the tunables panel shows an entry in.
///
/// The panel lists one group at a time. With a dozen-odd materials and ten
/// fields each it was a single scroll of well over a hundred rows, and the
/// two or three entries anyone is actually iterating on were buried in it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TunableGroup {
    /// How the material behaves — density, friction, combustion, flow.
    Physics,
    /// How it is drawn. Changes nothing in the simulation, which is the
    /// property that makes the split worth having and is asserted in tests.
    Visual,
    /// Explosions. The first group whose entries are **not material fields**
    /// — they live on `sim::explosion::Tuning`, an engine struct — which is
    /// why `Tunable::category` is no longer necessarily a material name and
    /// why the save path has to branch on the group rather than assuming
    /// every entry has a `.ron` file named after its category.
    Explosion,
    /// The character (M9). Engine-struct entries like `Explosion`'s, from
    /// `sim::player::Tuning` — how the gnome runs, jumps and falls, all
    /// judged in the hand and therefore all sweepable live.
    Player,
    /// **World time** (`sim::clock`) — how fast the day, the weather, growth
    /// and creatures run, each independently, and none of them the physics
    /// clock. Here rather than in a key binding because every key in the app
    /// is already bound, and because this panel's pin (`Enter`, then the
    /// arrows with the panel closed) is exactly the right shape for a knob
    /// whose effect is judged by watching the world rather than by reading a
    /// number.
    World,
}

impl TunableGroup {
    pub fn label(self) -> &'static str {
        match self {
            TunableGroup::Physics => "PHYSICS",
            TunableGroup::Visual => "VISUAL",
            TunableGroup::Explosion => "EXPLOSION",
            TunableGroup::Player => "PLAYER",
            TunableGroup::World => "WORLD",
        }
    }

    pub fn next(self) -> Self {
        match self {
            TunableGroup::Physics => TunableGroup::Visual,
            TunableGroup::Visual => TunableGroup::Explosion,
            TunableGroup::Explosion => TunableGroup::Player,
            TunableGroup::Player => TunableGroup::World,
            TunableGroup::World => TunableGroup::Physics,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            TunableGroup::Physics => TunableGroup::Player,
            TunableGroup::Visual => TunableGroup::Physics,
            TunableGroup::Explosion => TunableGroup::Visual,
            TunableGroup::Player => TunableGroup::Explosion,
            TunableGroup::World => TunableGroup::Player,
        }
    }

    pub fn all() -> [TunableGroup; 5] {
        [
            TunableGroup::Physics,
            TunableGroup::Visual,
            TunableGroup::Explosion,
            TunableGroup::Player,
            TunableGroup::World,
        ]
    }
}

/// The one category name every [`TunableGroup::Explosion`] entry uses.
/// Not a material — see that variant's own doc.
pub const EXPLOSION_CATEGORY: &str = "explosion";

/// Likewise for [`TunableGroup::Player`].
pub const PLAYER_CATEGORY: &str = "player";

/// Likewise for [`TunableGroup::World`].
pub const WORLD_CATEGORY: &str = "world";

/// One live-adjustable value. `value` is a live snapshot at the moment
/// the registry was built, not a handle back into the registry it came
/// from — `App` re-derives the list fresh whenever the panel is open
/// (materials are cheap to enumerate; a few dozen entries), so there is
/// no separate synchronization path to keep a stale snapshot updated.
#[derive(Clone)]
pub struct Tunable {
    pub group: TunableGroup,
    /// Which material this belongs to — also the `.ron` file's own base
    /// name, by the convention every shipped material file already
    /// follows (`sand.ron` defines `name: "sand"`).
    pub category: String,
    /// The exact field name as it appears in the `.ron` file — reused
    /// verbatim as the search key for the targeted-edit save path, so a
    /// typo here would silently fail to save rather than corrupt the
    /// wrong field.
    pub name: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    /// Whether the underlying `.ron` field is an integer type rather than a
    /// float. Every [`Tunable`] carries its value as `f32` for one uniform
    /// adjust/display path, but the *file* is typed, and RON will not accept
    /// a decimal where a `u8`/`u16` is declared.
    ///
    /// Getting this wrong is not a cosmetic problem, and it shipped: the
    /// save path formatted every value through `format_value`, which always
    /// emits a decimal point, so saving `water.min_transfer` (a `u16`) wrote
    /// `min_transfer: 60.0` and `ron::from_str` rejected it with
    /// **"Expected comma"** — RON reads the `60`, then finds `.0` where it
    /// wanted the next field. That is exactly the error the panel's own
    /// footer was reporting, on the one liquid field the file comments
    /// specifically invite people to sweep live. Reported from live play,
    /// reproduced directly against `water.ron` before this field existed.
    pub integral: bool,
}

impl Tunable {
    /// A float-valued tunable — the common case.
    fn float(group: TunableGroup, category: &str, name: &str, value: f32, min: f32, max: f32, step: f32) -> Self {
        Self { group, category: category.into(), name: name.into(), value, min, max, step, integral: false }
    }

    /// An integer-valued one, whose `.ron` field must never be written with
    /// a decimal point — see `integral`.
    fn integer(group: TunableGroup, category: &str, name: &str, value: f32, min: f32, max: f32, step: f32) -> Self {
        Self { group, category: category.into(), name: name.into(), value, min, max, step, integral: true }
    }
}

/// Every material field worth exposing, in a stable order (the order the
/// panel lists them in, category by category).
pub fn from_materials(materials: &MaterialRegistry) -> Vec<Tunable> {
    let mut out = Vec::new();
    // `paintable()`'s own filter (skip `Empty`/`Bedrock`, the two engine-
    // internal ids with nothing a content author would ever tune) is
    // reused here for the identical reason it exists there.
    for id in materials.paintable() {
        let m = materials.get(id);
        let category = m.name.clone();
        let phys = TunableGroup::Physics;
        out.push(Tunable::float(phys, &category, "density", m.density, 0.0, 5.0, 0.1));
        out.push(Tunable::float(phys, &category, "friction_angle", m.friction_angle, 1.0, 89.0, 1.0));
        out.push(Tunable::float(phys, &category, "flammability", m.flammability, 0.0, 1.0, 0.05));
        // Liquids only -- a dead band means nothing to a powder or a gas, and
        // an entry per material that ignores it is just noise in the panel.
        if m.kind == MaterialKind::Liquid {
            out.push(Tunable::float(TunableGroup::Visual, &category, "fill_dimming", m.fill_dimming, 0.0, 1.0, 0.05));
            // `u16` in the file -- see `Tunable::integral`.
            out.push(Tunable::integer(phys, &category, "min_transfer", m.min_transfer as f32, 0.0, 400.0, 4.0));
            out.push(Tunable::integer(phys, &category, "flow_rate", m.flow_rate as f32, 0.0, 1000.0, 25.0));
        }
        // Gases only, and the same reasoning the liquid-only block above
        // carries: a dissipation chance means nothing to a powder or a
        // solid. Here specifically because "how long should smoke hang
        // around" is a look judgement and not a derivable number -- the
        // arithmetic on `MaterialDef::dissipation` narrows the range to
        // about 2-4 seconds of half-life and cannot pick inside it, which
        // is exactly the case `CLAUDE.md` says to answer with a live knob
        // rather than an argument. Fine step: the whole usable range is
        // under a hundredth.
        if m.kind == MaterialKind::Gas {
            out.push(Tunable::float(phys, &category, "dissipation", m.dissipation, 0.0, 0.05, 0.001));
        }
        out.push(Tunable::float(phys, &category, "heat_conductivity", m.heat_conductivity, 0.0, 1.0, 0.05));
        for (field, value) in [
            ("ignition_temperature", m.ignition_temperature),
            ("burn_temperature", m.burn_temperature),
            ("melting_point", m.melting_point),
            ("boiling_point", m.boiling_point),
        ] {
            if value.is_finite() {
                out.push(Tunable::float(phys, &category, field, value, 0.0, 2000.0, 10.0));
            }
        }
        // Structural integrity (`structural.rs`), `Solid` only -- a
        // confinement radius or a compression weight means nothing to a
        // powder or a gas, and the panel is long enough already. A material
        // sitting at the "never" span sentinel is skipped for exactly the
        // reason an infinite `ignition_temperature` is: there is no sensible
        // value to drag a slider up from. All `u16` in the file, so all
        // `integer` -- see `Tunable::integral` for why writing `2.0` into one
        // is a save-breaking error rather than a cosmetic one.
        if m.kind == MaterialKind::Solid && m.max_unsupported_span != u16::MAX {
            out.push(Tunable::integer(phys, &category, "max_unsupported_span", m.max_unsupported_span as f32, 0.0, 64.0, 1.0));
            out.push(Tunable::integer(phys, &category, "support_cost_below", m.support_cost_below as f32, 0.0, 8.0, 1.0));
            // Floored at 1, matching `Material::from`'s own clamp: all three
            // at 0 would let a distance propagate forever without growing,
            // silently disabling spans rather than tuning them.
            out.push(Tunable::integer(phys, &category, "support_cost_beside", m.support_cost_beside as f32, 1.0, 8.0, 1.0));
            out.push(Tunable::integer(phys, &category, "support_cost_above", m.support_cost_above as f32, 1.0, 8.0, 1.0));
        }
    }
    out
}

/// Every explosion parameter, in the order the panel lists them — roughly
/// "what does the blast look like" first, then the finer physics.
///
/// Each field's own reasoning lives on `sim::explosion::Tuning`; the ranges
/// here are what makes sense to sweep with a key held down, not hard limits.
///
/// "Every" is enforced, not aspirational: the crack, confinement and
/// afterglow fields were added to `Tuning` across two passes and none of
/// them reached this list, so the panel showed a blast's older half while
/// the knobs that decide how the rock actually comes apart were reachable
/// only by editing `assets/explosion.ron` and restarting.
/// `every_explosion_tunable_can_actually_be_written_back` now destructures
/// `Tuning` exhaustively and will not compile past a new field.
pub fn from_explosion(t: &Explosion) -> Vec<Tunable> {
    let g = TunableGroup::Explosion;
    let c = EXPLOSION_CATEGORY;
    vec![
        Tunable::float(g, c, "radius", t.radius, 1.0, 80.0, 1.0),
        Tunable::float(g, c, "strength", t.strength, 10.0, 600.0, 10.0),
        Tunable::float(g, c, "duration", t.duration, 1.0, 40.0, 1.0),
        Tunable::float(g, c, "debris_fraction", t.debris_fraction, 0.0, 1.0, 0.05),
        Tunable::float(g, c, "smoke_fraction", t.smoke_fraction, 0.0, 1.0, 0.02),
        Tunable::float(g, c, "flash_temperature", t.flash_temperature, 0.0, 2000.0, 50.0),
        Tunable::float(g, c, "fireball_fraction", t.fireball_fraction, 0.0, 3.0, 0.1),
        Tunable::float(g, c, "vaporize_fraction", t.vaporize_fraction, 0.0, 1.0, 0.02),
        Tunable::float(g, c, "shockwave_multiplier", t.shockwave_multiplier, 1.0, 4.0, 0.1),
        Tunable::float(g, c, "pierce_divisor", t.pierce_divisor, 1.0, 200.0, 2.0),
        Tunable::float(g, c, "speed_per_strength", t.speed_per_strength, 0.0, 0.5, 0.005),
        Tunable::float(g, c, "debris_jitter", t.debris_jitter, 0.0, 2.0, 0.05),
        Tunable::float(g, c, "heat_fraction", t.heat_fraction, 0.5, 20.0, 0.5),
        // The crack star (R1). `crack_growth` and `crack_stagger` are the
        // pair that decide whether the fissures read as a thing that
        // *happens* or as a graphic stamped on the stone, which is a
        // judged-in-the-hand question and therefore exactly what this panel
        // is for. The counts (`crack_rays`, `crack_growth`, and
        // `calve_depth` below) are `u32` on `Tuning`, so they register as
        // `integer` -- see `Tunable::integral`.
        Tunable::integer(g, c, "crack_rays", t.crack_rays as f32, 0.0, 48.0, 1.0),
        Tunable::float(g, c, "crack_reach", t.crack_reach, 0.0, 6.0, 0.1),
        // Floored at 1, matching the clamp where it is read: `0` freezes the
        // star half-drawn, so it is not a setting anyone can want.
        Tunable::integer(g, c, "crack_growth", t.crack_growth as f32, 1.0, 20.0, 1.0),
        // In frames, so a whole-frame step like `duration`'s rather than a
        // fractional one.
        Tunable::float(g, c, "crack_stagger", t.crack_stagger, 0.0, 40.0, 1.0),
        // A temperature, so the same 10-degree granularity the material
        // temperatures use. Capped well below `flash_temperature`'s 2000:
        // `render.rs`'s glow ramp is saturated by ~420 and stone cannot
        // ignite from it, so the range past that buys nothing visible.
        Tunable::float(g, c, "crack_glow_temperature", t.crack_glow_temperature, 0.0, 1000.0, 10.0),
        // Confinement (R2), and the collar the finished star calves off the
        // rim. `containment_floor` is a multiple of `radius`, not a 0..1
        // fraction, hence the wider range and coarser step.
        Tunable::float(g, c, "containment_floor", t.containment_floor, 0.0, 5.0, 0.1),
        Tunable::float(g, c, "confined_cavity_fraction", t.confined_cavity_fraction, 0.0, 1.0, 0.05),
        Tunable::integer(g, c, "calve_depth", t.calve_depth as f32, 0.0, 32.0, 1.0),
        // A *per-frame* retention, so the interesting band is the top tenth
        // (0.94 fades in ~90 frames, 0.99 in ~550) and a 0.05 step would
        // step straight over it -- `swim_damp`'s 0.01 for the same reason.
        Tunable::float(g, c, "afterglow_retention", t.afterglow_retention, 0.5, 1.0, 0.01),
        // The joint fabric (F). `joint_reach` is a multiple of `radius`
        // like `crack_reach`, and the other two are 0..1 fractions.
        //
        // These three are the density controls the owner's verdict on the
        // pattern lands on -- *"I like a little of it, but there is too
        // much"* -- so they are exactly the kind of judged-by-eye question
        // this panel exists for. The fourth control is
        // `stone.ron`'s `joint_spacing`, which is a material field and is
        // already listed by `from_materials`.
        Tunable::float(g, c, "joint_reach", t.joint_reach, 0.0, 6.0, 0.1),
        Tunable::float(g, c, "joint_open_fraction", t.joint_open_fraction, 0.0, 1.0, 0.05),
        Tunable::float(g, c, "joint_density", t.joint_density, 0.0, 1.0, 0.05),
    ]
}

/// Every character-feel parameter, roughly running first, then jumping,
/// then the forgiveness windows. Same matched-pair contract with
/// `apply_player` that `from_explosion`/`apply_explosion` document.
pub fn from_player(t: &Player) -> Vec<Tunable> {
    let g = TunableGroup::Player;
    let c = PLAYER_CATEGORY;
    vec![
        Tunable::float(g, c, "run_accel", t.run_accel, 0.02, 0.5, 0.01),
        Tunable::float(g, c, "run_max", t.run_max, 0.4, 2.5, 0.1),
        Tunable::float(g, c, "ground_decel", t.ground_decel, 0.02, 1.0, 0.02),
        Tunable::float(g, c, "air_control", t.air_control, 0.0, 1.0, 0.05),
        Tunable::float(g, c, "jump_impulse", t.jump_impulse, 1.0, 3.0, 0.05),
        Tunable::float(g, c, "gravity", t.gravity, 0.05, 0.4, 0.01),
        Tunable::float(g, c, "fall_clamp", t.fall_clamp, 1.0, 6.0, 0.25),
        Tunable::integer(g, c, "coyote_frames", t.coyote_frames as f32, 0.0, 15.0, 1.0),
        Tunable::integer(g, c, "jump_buffer_frames", t.jump_buffer_frames as f32, 0.0, 10.0, 1.0),
        Tunable::integer(g, c, "step_up", t.step_up as f32, 0.0, 8.0, 1.0),
        Tunable::integer(g, c, "dig_reach", t.dig_reach as f32, 2.0, 40.0, 1.0),
        Tunable::integer(g, c, "dig_radius", t.dig_radius as f32, 1.0, 12.0, 1.0),
        Tunable::integer(g, c, "dig_cooldown", t.dig_cooldown as f32, 1.0, 30.0, 1.0),
        Tunable::integer(g, c, "wade_rows", t.wade_rows as f32, 0.0, 5.0, 1.0),
        Tunable::float(g, c, "wade_slowdown", t.wade_slowdown, 0.1, 1.0, 0.05),
        // Negative is the useful half of this range: he floats. Positive
        // values are left reachable on purpose so the panel can answer
        // "what if he sank" without a rebuild.
        Tunable::float(g, c, "buoyancy", t.buoyancy, -1.0, 0.5, 0.05),
        Tunable::float(g, c, "swim_damp", t.swim_damp, 0.5, 1.0, 0.01),
        Tunable::float(g, c, "stroke_impulse", t.stroke_impulse, 0.1, 2.5, 0.05),
        Tunable::integer(g, c, "stroke_cooldown", t.stroke_cooldown as f32, 1.0, 40.0, 1.0),
        Tunable::integer(g, c, "mantle_reach", t.mantle_reach as f32, 0.0, 8.0, 1.0),
        Tunable::integer(g, c, "shake_reach", t.shake_reach as f32, 2.0, 40.0, 1.0),
        Tunable::float(g, c, "shake_shed", t.shake_shed, 0.0, 1.0, 0.05),
        Tunable::float(g, c, "shake_seed", t.shake_seed, 0.0, 1.0, 0.05),
        Tunable::float(g, c, "climb_speed", t.climb_speed, 0.2, 2.0, 0.05),
        Tunable::float(g, c, "surface_hop", t.surface_hop, 0.0, 1.5, 0.05),
        Tunable::float(g, c, "dig_yield", t.dig_yield, 0.0, 1.0, 0.05),
    ]
}

/// Apply one adjusted player value back onto the live tuning struct —
/// `from_player`'s other half; keep the two name lists together.
pub fn apply_player(t: &mut Player, name: &str, value: f32) {
    match name {
        "run_accel" => t.run_accel = value,
        "run_max" => t.run_max = value,
        "ground_decel" => t.ground_decel = value,
        "air_control" => t.air_control = value,
        "jump_impulse" => t.jump_impulse = value,
        "gravity" => t.gravity = value,
        "fall_clamp" => t.fall_clamp = value,
        "coyote_frames" => t.coyote_frames = value.max(0.0).round() as u8,
        "jump_buffer_frames" => t.jump_buffer_frames = value.max(0.0).round() as u8,
        "step_up" => t.step_up = value.max(0.0).round() as u8,
        "dig_reach" => t.dig_reach = value.max(0.0).round() as u8,
        "dig_radius" => t.dig_radius = value.max(1.0).round() as u8,
        // Floored at 1: a zero cooldown is a bite every frame, which is
        // not "fast digging" but a bore that opens faster than the eye
        // can read and a frame cost 8x what was measured.
        "dig_cooldown" => t.dig_cooldown = value.max(1.0).round() as u8,
        "wade_rows" => t.wade_rows = value.max(0.0).round() as u8,
        "wade_slowdown" => t.wade_slowdown = value,
        "buoyancy" => t.buoyancy = value,
        "swim_damp" => t.swim_damp = value,
        "stroke_impulse" => t.stroke_impulse = value,
        "stroke_cooldown" => t.stroke_cooldown = value.max(1.0).round() as u8,
        "mantle_reach" => t.mantle_reach = value.max(0.0).round() as u8,
        "shake_reach" => t.shake_reach = value.max(1.0).round() as u8,
        "shake_shed" => t.shake_shed = value,
        "shake_seed" => t.shake_seed = value,
        "climb_speed" => t.climb_speed = value,
        "surface_hop" => t.surface_hop = value,
        "dig_yield" => t.dig_yield = value.clamp(0.0, 1.0),
        _ => {}
    }
}

/// Every world-time knob (`sim::clock`), in the order the panel lists them:
/// the two the owner asked about first, then the rest.
///
/// **All integers, all "N times slower than baseline", all stepping by 1.**
/// The panel's arrow keys do not auto-repeat (`main.rs` filters
/// `event.repeat`), so a fine float step across a wide span would take
/// hundreds of presses; one press being one meaningful unit is what makes
/// this sweepable by hand at all. `day_minutes` is named in minutes rather
/// than as a bare multiplier because minutes is the quantity anyone actually
/// thinks in — the two are the same number only because the baseline day
/// happens to be one minute long.
///
/// Ranges run to [`MAX_SLOWDOWN`], which is set from what breaks rather than
/// from an aspiration — see that constant.
pub fn from_clock(c: &Clock) -> Vec<Tunable> {
    let g = TunableGroup::World;
    let cat = WORLD_CATEGORY;
    let max = MAX_SLOWDOWN as f32;
    vec![
        Tunable::integer(g, cat, "day_minutes", c.day_minutes as f32, 1.0, max, 1.0),
        Tunable::integer(g, cat, "growth_slowdown", c.growth_slowdown as f32, 1.0, max, 1.0),
        Tunable::integer(g, cat, "weather_slowdown", c.weather_slowdown as f32, 1.0, max, 1.0),
        Tunable::integer(g, cat, "creature_slowdown", c.creature_slowdown as f32, 1.0, max, 1.0),
        Tunable::integer(g, cat, "gnome_slowdown", c.gnome_slowdown as f32, 1.0, max, 1.0),
    ]
}

/// Apply one adjusted world-time value back onto the live clock.
///
/// **Goes through `Clock::set_rates`, never straight at the field**, and that
/// is the whole reason this takes a `frame`. The phase clocks are derived by
/// dividing elapsed frames by the rate, so writing a rate on its own
/// reinterprets the entire history at the new one: dragging the day length
/// from 1 to 4 an hour in would move the sun to wherever that hour lands
/// under the new divisor. Re-anchoring first pins the current sky and weather
/// frames, so the value at this instant is unchanged and only the slope
/// moves — which is the behaviour a live slider has to have.
///
/// Matched-pair contract with `from_clock`, the same one
/// `from_explosion`/`apply_explosion` document and
/// `every_registered_world_knob_can_be_written_back` asserts.
pub fn apply_clock(c: &mut Clock, frame: u64, name: &str, value: f32) {
    let v = value.max(1.0).round() as u32;
    c.set_rates(frame, |c| match name {
        "day_minutes" => c.day_minutes = v,
        "weather_slowdown" => c.weather_slowdown = v,
        "growth_slowdown" => c.growth_slowdown = v,
        "creature_slowdown" => c.creature_slowdown = v,
        "gnome_slowdown" => c.gnome_slowdown = v,
        // `from_clock` is the only source of entries, so every name it can
        // produce is handled above. A defensive floor against the two lists
        // drifting, not a reachable case -- and the test named above is what
        // keeps it unreachable.
        _ => {}
    });
}

/// Apply one adjusted explosion value back onto the live tuning struct.
///
/// Kept next to `from_explosion` on purpose: the two are a matched pair of
/// name lists, and splitting them across modules is how the field name in
/// one drifts from the field name in the other. The same argument applies to
/// `app.rs`'s material dispatch, which has lived with that risk since M-UI;
/// this at least keeps the new half honest.
pub fn apply_explosion(t: &mut Explosion, name: &str, value: f32) {
    match name {
        "radius" => t.radius = value,
        "strength" => t.strength = value,
        "duration" => t.duration = value,
        "debris_fraction" => t.debris_fraction = value,
        "smoke_fraction" => t.smoke_fraction = value,
        "flash_temperature" => t.flash_temperature = value,
        "fireball_fraction" => t.fireball_fraction = value,
        "vaporize_fraction" => t.vaporize_fraction = value,
        "shockwave_multiplier" => t.shockwave_multiplier = value,
        "pierce_divisor" => t.pierce_divisor = value,
        "speed_per_strength" => t.speed_per_strength = value,
        "debris_jitter" => t.debris_jitter = value,
        "heat_fraction" => t.heat_fraction = value,
        "crack_rays" => t.crack_rays = value.max(0.0).round() as u32,
        "crack_reach" => t.crack_reach = value,
        // Floored at 1 here as well as in the registered range: the panel
        // is not the only caller, and a 0 written in from anywhere freezes
        // the star half-drawn.
        "crack_growth" => t.crack_growth = value.max(1.0).round() as u32,
        "crack_stagger" => t.crack_stagger = value,
        "crack_glow_temperature" => t.crack_glow_temperature = value,
        "containment_floor" => t.containment_floor = value,
        "confined_cavity_fraction" => t.confined_cavity_fraction = value,
        "calve_depth" => t.calve_depth = value.max(0.0).round() as u32,
        "afterglow_retention" => t.afterglow_retention = value,
        "joint_reach" => t.joint_reach = value,
        "joint_open_fraction" => t.joint_open_fraction = value,
        "joint_density" => t.joint_density = value,
        _ => {}
    }
}

/// Where a saved edit's target `.ron` file lives, by the same
/// name-matches-filename convention `from_materials` already relies on.
pub fn material_file_path(dir: impl AsRef<std::path::Path>, material_name: &str) -> std::path::PathBuf {
    dir.as_ref().join(format!("{material_name}.ron"))
}

/// Rewrite exactly `field`'s own value span in `source` to `new_value`,
/// leaving every other byte — comments included — untouched. Not a
/// `ron::ser` round-trip: that would silently destroy every comment in the
/// file, and material files' comments carry real reasoning (`oil.ron`'s
/// own header, for one). Distinct from a substring match inside a longer
/// field name (matching `temperature` inside `burn_temperature` would
/// corrupt the wrong field).
///
/// Most material files only write the handful of fields that differ from
/// `Material`'s `serde` defaults (`stone.ron` never mentions
/// `heat_conductivity`, for one) — so `field` genuinely absent from the
/// text is the *common* case for a registered [`Tunable`], not a typo, and
/// live-verifying this against real asset files (rather than only the
/// hand-built strings in this module's own tests) is what caught it.
/// When `field` isn't found as an existing key, this appends
/// `field: new_value,` on its own line just before the file's own closing
/// `)` (the outermost one — every shipped material file is a single
/// top-level struct, so its last `)` is unambiguous) rather than erroring.
///
/// `integral` selects the number format — see [`Tunable::integral`] for why
/// a decimal point in the wrong place is a save-breaking error rather than a
/// cosmetic one.
pub fn write_field_value(source: &str, field: &str, new_value: f32, integral: bool) -> Result<String, String> {
    match find_field_value_span(source, field) {
        Some(span) => {
            let mut out = String::with_capacity(source.len());
            out.push_str(&source[..span.start]);
            out.push_str(&format_value(new_value, integral));
            out.push_str(&source[span.end..]);
            Ok(out)
        }
        None => {
            let close = source.rfind(')').ok_or_else(|| format!("field '{field}' not found, and no closing ')' to insert it before"))?;
            // The struct's last existing field may or may not already end
            // in a trailing comma (RON allows either) -- insert one
            // ourselves when it's missing, or `field: value` would run
            // straight into whatever came before it with no separator.
            // `last_significant_char` (not a raw `.trim_end().chars().last()`)
            // matters here: a review found that if the struct's last line
            // before `)` is a bare trailing comment, the naive version reads
            // the comment's own last letter as "the last real character",
            // decides no comma is needed, and inserts one anyway right after
            // the comment -- for the case where a comma genuinely *was*
            // needed, that stray comma lands outside the comment (harmless),
            // but the reverse mistake (skipping a needed comma because the
            // comment happened to end past where a comma already was) would
            // produce RON `ron::from_str` rejects before ever writing to
            // disk. Either way, reading through comments is the correct fix.
            let needs_comma = !matches!(last_significant_char(&source[..close]), Some(',') | Some('(') | None);
            let mut out = String::with_capacity(source.len() + field.len() + 16);
            out.push_str(&source[..close]);
            if needs_comma {
                out.push(',');
            }
            out.push_str(&format!("\n    {field}: {},\n", format_value(new_value, integral)));
            out.push_str(&source[close..]);
            Ok(out)
        }
    }
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// The last non-whitespace character in `text`, ignoring `//` line
/// comments entirely — so a trailing comment-only line (or a value with a
/// trailing inline comment) doesn't masquerade as real struct content.
/// Scans lines from the end, stripping each line's own comment before
/// checking whether anything real is left on it.
fn last_significant_char(text: &str) -> Option<char> {
    for line in text.lines().rev() {
        let code = line.split("//").next().unwrap_or("");
        if let Some(c) = code.trim_end().chars().last() {
            return Some(c);
        }
    }
    None
}

/// Byte range of `field`'s value literal within `source` — from the first
/// non-whitespace character after its `:` to the first `,`, `)`, newline,
/// or `//` line comment after that (whichever comes first; trailing
/// whitespace trimmed off the end). `field` must appear as a whole
/// identifier (not preceded or followed by another identifier character)
/// immediately followed by `:` (whitespace allowed in between, matching how
/// `ron`'s own parser is whitespace-insensitive there).
///
/// The `//` case matters: an independent review confirmed that without it,
/// a field written as `density: 1.0 // heavy` has its trailing comment
/// silently folded into the matched span and deleted on save — the
/// resulting text is still valid RON (so `save_tunable`'s parse check
/// doesn't catch it), it just permanently loses the comment, which is
/// exactly what this module's targeted-edit approach exists to avoid.
fn find_field_value_span(source: &str, field: &str) -> Option<std::ops::Range<usize>> {
    let bytes = source.as_bytes();
    let mut search_from = 0;
    while let Some(rel) = source[search_from..].find(field) {
        let start = search_from + rel;
        let end = start + field.len();
        search_from = end;
        let prev_is_boundary = start == 0 || !is_ident_char(bytes[start - 1]);
        let next_is_boundary = end == bytes.len() || !is_ident_char(bytes[end]);
        if !prev_is_boundary || !next_is_boundary {
            continue;
        }
        let after = &source[end..];
        let after_trimmed = after.trim_start();
        if !after_trimmed.starts_with(':') {
            continue;
        }
        let colon_pos = end + (after.len() - after_trimmed.len());
        let value_region = &source[colon_pos + 1..];
        let value_start = colon_pos + 1 + (value_region.len() - value_region.trim_start().len());
        let value_rest = &source[value_start..];
        let delim = value_rest.find([',', ')', '\n']).unwrap_or(value_rest.len());
        let comment = value_rest.find("//").unwrap_or(value_rest.len());
        let raw_len = delim.min(comment);
        let trimmed_len = value_rest[..raw_len].trim_end().len();
        return Some(value_start..value_start + trimmed_len);
    }
    None
}

/// A RON float literal for `v` — always at least one digit after the
/// decimal point (`45` alone is a valid RON float too, but every shipped
/// material file already writes whole numbers as `45.0`, and matching
/// that existing style matters more here than shaving two bytes).
/// Rounded to 4 decimal places and trailing zeros stripped, so a step of
/// `0.1` repeatedly applied doesn't accumulate `f32` noise into the file
/// as `45.09999998`.
fn format_value(v: f32, integral: bool) -> String {
    if integral {
        // No decimal point at all: the field is a `u8`/`u16` in the file and
        // RON rejects `60.0` there with "Expected comma". Rounded rather
        // than truncated so a value nudged to 59.9999 by repeated `step`
        // addition saves as 60, not 59.
        return format!("{}", v.round() as i64);
    }
    let s = format!("{v:.4}");
    let trimmed = s.trim_end_matches('0');
    let trimmed = trimmed.trim_end_matches('.');
    if trimmed.contains('.') {
        trimmed.to_string()
    } else {
        format!("{trimmed}.0")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::material;

    #[test]
    fn every_group_is_reachable_and_visual_changes_nothing_in_the_simulation() {
        let registry = MaterialRegistry::builtin();
        // Every registrant, not just materials. `Explosion`'s entries come
        // from `from_explosion` — an engine struct rather than a material —
        // and this assertion is about the *panel* having something to show
        // in every menu, so it has to see everything the panel does. Keeping
        // it material-only would have silently passed while the new menu was
        // empty, which is precisely what it exists to catch.
        let mut all = from_materials(&registry);
        all.extend(from_explosion(&Explosion::default()));
        all.extend(from_player(&Player::default()));
        all.extend(from_clock(&Clock::default()));
        for group in TunableGroup::all() {
            assert!(
                all.iter().any(|t| t.group == group),
                "the {} menu is empty, so nothing would ever show in it",
                group.label()
            );
        }
        // The split is only worth having if `Visual` really is inert, so
        // that is asserted rather than left as an intention: adding a field
        // here forces a decision about which side it belongs on.
        for t in all.iter().filter(|t| t.group == TunableGroup::Visual) {
            assert_eq!(t.name, "fill_dimming", "unexpected entry {} in the VISUAL menu", t.name);
        }
    }

    /// Every world-time knob must round-trip: listed by `from_clock`,
    /// written back by `apply_clock`, and *observable* afterwards.
    ///
    /// The observability half is the point. `apply_clock` writes through
    /// `Clock::set_rates`, which clamps — so a typo'd name would silently
    /// write nothing while the row still displayed and still appeared to
    /// adjust, the exact failure the explosion pair's own doc describes. This
    /// checks the value actually moved, and that the exhaustive destructuring
    /// below refuses to compile past a new field.
    #[test]
    fn every_registered_world_knob_can_be_written_back() {
        let base = Clock::default();
        // The *exhaustiveness* half of this contract lives in `clock.rs`'s
        // own tests (`every_settable_knob_is_registered_in_the_panel`),
        // because the anchor fields are private to that module and a
        // destructure here cannot name them -- which is the right way round:
        // they are running state, not settings, and the panel has no business
        // seeing them.
        for t in from_clock(&base) {
            let mut c = Clock::default();
            apply_clock(&mut c, 0, &t.name, 3.0);
            let listed = from_clock(&c).into_iter().find(|x| x.name == t.name).expect("still listed");
            assert_eq!(listed.value, 3.0, "{} listed but not written back by apply_clock", t.name);
            // ...and nothing else moved with it.
            for other in from_clock(&c).iter().filter(|x| x.name != t.name) {
                assert_eq!(other.value, 1.0, "adjusting {} also moved {}", t.name, other.name);
            }
        }
    }

    /// Every knob is an integer stepping by 1, and the reason is a UI
    /// constraint rather than a preference — see `from_clock`.
    #[test]
    fn every_world_knob_is_an_integer_stepping_by_one() {
        for t in from_clock(&Clock::default()) {
            assert!(t.integral, "{} must be integral: the clock knobs are whole multiples", t.name);
            assert_eq!(t.step, 1.0, "{} must step by 1: the panel's arrows do not auto-repeat", t.name);
            assert_eq!(t.min, 1.0, "{} must floor at 1: 0 would divide the world by zero", t.name);
            assert_eq!(t.max, MAX_SLOWDOWN as f32, "{} must not exceed the measured cap", t.name);
        }
    }

    /// The panel's two halves of the explosion story have to agree: every
    /// entry `from_explosion` lists must be one `apply_explosion` can
    /// actually write back. They are a hand-maintained pair of name lists,
    /// and a typo in either would silently produce a row that displays fine
    /// and does nothing when adjusted — the exact failure mode `app.rs`'s
    /// material dispatch has always been exposed to and nothing checks.
    ///
    /// It also checks the *other* direction, which the write-back loop
    /// alone cannot see: a field added to `Explosion` and never registered
    /// produces no row at all, so there is nothing for the loop to iterate
    /// over and it stays green while the new knob is unreachable in play.
    /// That is not hypothetical — the whole crack/confinement/afterglow
    /// half of `Tuning` sat unexposed for two passes. The list below is
    /// destructured out of `Explosion` exhaustively, so adding a field
    /// stops this test *compiling* until it is named here, and the
    /// assertion then forces it to be wired rather than merely named.
    #[test]
    fn every_explosion_tunable_can_actually_be_written_back() {
        let base = Explosion::default();

        macro_rules! explosion_fields {
            ($($f:ident),* $(,)?) => {{
                // Binds nothing (`$f: _`) and omits `..` on purpose: the
                // compiler's exhaustiveness check is the point of the
                // pattern, and the array is the same list as strings.
                let Explosion { $($f: _),* } = base;
                [$(stringify!($f)),*]
            }};
        }
        let fields = explosion_fields!(
            radius,
            strength,
            duration,
            vaporize_fraction,
            debris_fraction,
            shockwave_multiplier,
            fireball_fraction,
            flash_temperature,
            smoke_fraction,
            heat_fraction,
            speed_per_strength,
            debris_jitter,
            crack_rays,
            crack_reach,
            crack_growth,
            crack_stagger,
            containment_floor,
            confined_cavity_fraction,
            calve_depth,
            afterglow_retention,
            crack_glow_temperature,
            pierce_divisor,
            joint_reach,
            joint_open_fraction,
            joint_density,
        );
        let listed = from_explosion(&base);
        for field in fields {
            assert!(
                listed.iter().any(|t| t.name == field),
                "explosion.{field} exists on Tuning but has no panel entry -- it cannot be swept in play"
            );
        }
        assert_eq!(listed.len(), fields.len(), "from_explosion lists an entry that is not a field on Tuning");

        for t in from_explosion(&base) {
            let mut probe = base;
            // A value guaranteed different from the current one, inside the
            // registered range, so "nothing changed" can only mean the write
            // did not land.
            let target = if (t.value - t.max).abs() > f32::EPSILON { t.max } else { t.min };
            apply_explosion(&mut probe, &t.name, target);
            assert_ne!(probe, base, "adjusting explosion.{} changed nothing -- name mismatch?", t.name);
            let written = from_explosion(&probe).into_iter().find(|x| x.name == t.name).expect("entry still listed");
            assert_eq!(written.value, target, "explosion.{} wrote to the wrong field", t.name);
        }
    }

    /// Same two-hand-maintained-lists contract as the explosion test
    /// above, same failure mode guarded: a row that displays fine and
    /// does nothing when adjusted.
    #[test]
    fn every_player_tunable_can_actually_be_written_back() {
        let base = Player::default();
        for t in from_player(&base) {
            let mut probe = base;
            let target = if (t.value - t.max).abs() > f32::EPSILON { t.max } else { t.min };
            apply_player(&mut probe, &t.name, target);
            assert_ne!(probe, base, "adjusting player.{} changed nothing -- name mismatch?", t.name);
            let written = from_player(&probe).into_iter().find(|x| x.name == t.name).expect("entry still listed");
            assert_eq!(written.value, target, "player.{} wrote to the wrong field", t.name);
        }
    }

    #[test]
    fn from_materials_registers_finite_fields_and_skips_never_sentinels() {
        let reg = MaterialRegistry::builtin();
        let tunables = from_materials(&reg);
        let sand: Vec<_> = tunables.iter().filter(|t| t.category == "sand").collect();
        assert!(sand.iter().any(|t| t.name == "friction_angle"));
        assert!(sand.iter().any(|t| t.name == "density"));
        // Sand has no ignition_temperature set (stays at the "never"
        // sentinel) -- must not appear at all, not appear clamped to 2000.
        assert!(!sand.iter().any(|t| t.name == "ignition_temperature"), "an unset (infinite) field should not be registered");

        // Oil actually sets a real burn_temperature (900C) -- unlike
        // ignition_temperature, which oil's own file explicitly leaves at
        // the "never" default (catches only from an adjacent flame, not
        // spontaneously; see oil.ron's own comment).
        let oil: Vec<_> = tunables.iter().filter(|t| t.category == "oil").collect();
        assert!(oil.iter().any(|t| t.name == "burn_temperature"), "oil sets a real burn_temperature and should be tunable");
        assert!(!oil.iter().any(|t| t.name == "ignition_temperature"), "oil leaves ignition_temperature unset and it should not be registered");
        let _ = material::SAND; // keep the import meaningful if the above ever changes
    }

    #[test]
    fn write_field_value_replaces_only_the_named_field() {
        let source = "(name: \"sand\", kind: Powder, density: 1.6, friction_angle: 34.0, colors: [(1,2,3)])";
        let updated = write_field_value(source, "friction_angle", 40.0, false).unwrap();
        assert!(updated.contains("friction_angle: 40.0"));
        assert!(updated.contains("density: 1.6"), "an unrelated field must be untouched: {updated}");
        assert_eq!(updated.len(), source.len() + "40.0".len() - "34.0".len());
    }

    #[test]
    fn write_field_value_does_not_match_a_field_name_that_is_a_substring_of_another() {
        let source = "(name: \"oil\", kind: Liquid, burn_temperature: 900.0, temperature: 20.0)";
        // A field literally named "temperature" exists too, distinct from
        // "burn_temperature" -- searching for "temperature" must not stop
        // at the substring inside "burn_temperature".
        let updated = write_field_value(source, "temperature", 25.0, false).unwrap();
        assert!(updated.contains("burn_temperature: 900.0"), "burn_temperature must be untouched: {updated}");
        assert!(updated.contains("temperature: 25.0"), "the standalone temperature field should have been updated: {updated}");
    }

    #[test]
    fn write_field_value_preserves_comments_elsewhere_in_the_file() {
        let source = "// a real reason this material exists\n(name: \"oil\", kind: Liquid, density: 0.8)";
        let updated = write_field_value(source, "density", 0.9, false).unwrap();
        assert!(updated.starts_with("// a real reason this material exists\n"), "the comment must survive verbatim: {updated}");
    }

    #[test]
    fn write_field_value_does_not_swallow_a_trailing_inline_comment_on_the_edited_line() {
        // A field written as `density: 1.0 // heavy` is natural RON style
        // that no shipped file happens to use today, but the original
        // value-span search stopped only at `,`/`)`/newline -- swallowing
        // the comment into the matched span and silently deleting it on
        // save, without the parse-before-write safety net catching it
        // (the result is still valid RON, just missing the comment).
        let source = "(name: \"oil\", kind: Liquid, density: 1.0 // heavy\n)";
        let updated = write_field_value(source, "density", 0.9, false).unwrap();
        assert!(updated.contains("// heavy"), "the inline comment must survive: {updated}");
        assert!(updated.contains("density: 0.9"), "{updated}");
    }

    #[test]
    fn write_field_value_insert_path_reads_through_a_trailing_comment_before_the_closing_paren() {
        // A comment-only line as the struct's last content before `)`
        // must not be mistaken for "the last real character was a letter,
        // so a comma is needed" -- `last_significant_char` has to see past
        // it to the real last field (which already ends in a comma here).
        let source = "(name: \"stone\", kind: Solid, density: 2.5, colors: [(1,2,3)],\n// a trailing note\n)";
        let updated = write_field_value(source, "heat_conductivity", 0.4, false).unwrap();
        assert!(updated.contains("// a trailing note"), "the comment must survive: {updated}");
        ron::from_str::<material::MaterialDef>(&updated).expect("must still parse -- no stray or missing comma: {updated}");
    }

    #[test]
    fn write_field_value_appends_a_field_missing_from_the_file_before_the_closing_paren() {
        // The common case: `heat_conductivity` (say) is a real, finite
        // field on `Material` -- registered as a `Tunable` -- but this
        // particular file never wrote it, relying on the struct default
        // instead. Saving an adjustment must still work, not error.
        let source = "(name: \"stone\", kind: Solid, density: 2.5, colors: [(128, 128, 132)], max_unsupported_span: 3)";
        let updated = write_field_value(source, "heat_conductivity", 0.4, false).unwrap();
        assert!(updated.contains("heat_conductivity: 0.4"), "{updated}");
        assert!(updated.contains("density: 2.5"), "existing fields must be untouched: {updated}");
        assert!(updated.trim_end().ends_with(')'), "must still end with the closing paren: {updated}");
        ron::from_str::<material::MaterialDef>(&updated).expect("appended field should still parse");
    }

    #[test]
    fn write_field_value_errors_only_when_there_is_no_closing_paren_at_all() {
        assert!(write_field_value("not a ron struct", "density", 1.0, false).is_err());
    }

    #[test]
    fn a_written_field_value_still_parses_as_valid_ron() {
        // The actual safety net the save path relies on: this doesn't
        // enforce parseability itself (that's `App::save_tunable`'s job,
        // reading the result back through `ron::from_str` before ever
        // writing to disk) -- this test just confirms the common case
        // produces valid RON, so a regression here would be caught before
        // it ever reached that safety net silently passing malformed text.
        let source = std::fs::read_to_string("assets/materials/sand.ron").expect("sand.ron should exist in the crate root");
        let updated = write_field_value(&source, "friction_angle", 37.5, false).unwrap();
        ron::from_str::<material::MaterialDef>(&updated).expect("edited sand.ron should still parse");
    }

    #[test]
    fn format_value_strips_trailing_zeros_but_keeps_a_decimal_point() {
        assert_eq!(format_value(45.0, true), "45");
        assert_eq!(format_value(59.9999, true), "60", "an integral value must round, not truncate");
        assert_eq!(format_value(45.0, false), "45.0");
        assert_eq!(format_value(12.345, false), "12.345");
        assert_eq!(format_value(0.1, false), "0.1");
        assert_eq!(format_value(-5.5, false), "-5.5");
    }

    /// Every registered tunable must survive the panel's actual save path
    /// against its own real asset file.
    ///
    /// The bug this exists for shipped and was reported from live play:
    /// `min_transfer` is a `u16`, the save path wrote `min_transfer: 60.0`,
    /// and `ron::from_str` rejected the result with "Expected comma" —
    /// which the panel then displayed in its footer. No existing test caught
    /// it, because every save test used a hand-built source string with only
    /// float fields in it, and none of them re-parsed the result as a real
    /// `MaterialDef`. Both of those gaps are closed here: real files, real
    /// round trip, every registered entry rather than a chosen few.
    ///
    /// Confirmed to fail before `Tunable::integral` existed — on exactly
    /// the two `u16` liquid fields and nothing else.
    #[test]
    fn every_registered_tunable_saves_to_a_file_that_still_parses() {
        let registry = MaterialRegistry::builtin();
        for t in from_materials(&registry) {
            let path = material_file_path(material::ASSET_DIR, &t.category);
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue; // a material with no file of its own is not this test's problem
            };
            // Mid-range rather than the current value, so the write is a
            // real change and an integral field genuinely has to round.
            let probe = (t.min + t.max) / 2.0;
            let updated = write_field_value(&source, &t.name, probe, t.integral)
                .unwrap_or_else(|e| panic!("{}.{} failed to write: {e}", t.category, t.name));
            if let Err(e) = ron::from_str::<material::MaterialDef>(&updated) {
                panic!(
                    "{}.{} = {probe} produced a file that no longer parses: {e}\n\
                     wrote: {}",
                    t.category,
                    t.name,
                    updated.lines().find(|l| l.contains(&t.name)).unwrap_or("?").trim()
                );
            }
        }
    }
}
