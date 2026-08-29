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

use crate::sim::clock::{Clock, SkyPin, MAX_SLOWDOWN};
use crate::sim::explosion::{Preset, Tuning as Explosion};
use crate::sim::material::{MaterialKind, MaterialRegistry};
use crate::sim::player::Tuning as Player;
use crate::sim::weather::{Pin as WeatherPin, Weather};

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

    /// **`World` first, and it is the menu the panel opens on.**
    ///
    /// The order was `Physics, Visual, Explosion, Player, World`, which put
    /// what the sky is doing four presses behind a hundred-odd rows of
    /// material fields — reasonable while this was a material panel and wrong
    /// once it grew a time-of-day and a weather control, which are the two
    /// things anybody opens it *to use* rather than to sweep. Everything else
    /// keeps its relative order; only `World` moved, and it moved to the
    /// front rather than being special-cased out of the cycle.
    pub fn next(self) -> Self {
        match self {
            TunableGroup::World => TunableGroup::Physics,
            TunableGroup::Physics => TunableGroup::Visual,
            TunableGroup::Visual => TunableGroup::Explosion,
            TunableGroup::Explosion => TunableGroup::Player,
            TunableGroup::Player => TunableGroup::World,
        }
    }

    /// **Kept an exact inverse of [`Self::next`], and guarded as one.** This
    /// read `Physics => Player` for as long as `World` existed: the world-speed
    /// menu was added to `next` and to `all`, and this direction was missed.
    /// Nothing caught it because `prev` had no caller -- an unused `pub fn`
    /// is not dead code to clippy -- so the bug sat waiting for whoever first
    /// wired a "previous menu" key, which is exactly the reader least able to
    /// tell a wrong answer from a right one. `next_and_prev_are_inverses`
    /// fails now if a new variant updates only one direction.
    ///
    /// `Shift+Tab` is that caller, wired with the panel's own redesign. The
    /// guard was written before the key existed and was already green when it
    /// arrived, which is the cheap half of `CLAUDE.md`'s put-the-fault-back
    /// rule: a guard written before the fix has already been watched failing.
    pub fn prev(self) -> Self {
        match self {
            TunableGroup::World => TunableGroup::Player,
            TunableGroup::Physics => TunableGroup::World,
            TunableGroup::Visual => TunableGroup::Physics,
            TunableGroup::Explosion => TunableGroup::Visual,
            TunableGroup::Player => TunableGroup::Explosion,
        }
    }

    /// **In cycle order, and that is a contract rather than a coincidence.**
    /// The panel draws this as a tab strip, so a list that disagreed with
    /// [`Self::next`] would show a row of tabs that `Tab` walks in some other
    /// order — asserted by `all_is_the_cycle_order`.
    pub fn all() -> [TunableGroup; 5] {
        [
            TunableGroup::World,
            TunableGroup::Physics,
            TunableGroup::Visual,
            TunableGroup::Explosion,
            TunableGroup::Player,
        ]
    }
}

/// The section headings the EXPLOSION menu is split into. Not materials —
/// see [`TunableGroup::Explosion`]'s own doc for why a category here is not a
/// `.ron` file name.
///
/// **These are the whole of the menu simplification, and they are load-bearing
/// rather than decorative.** Reported from play: *"there are so many different
/// options for changing explosions and I don't really know what each does and
/// it is too complicated."* The panel already draws a subheader wherever the
/// category changes (`App::draw_tunables_panel`), and every explosion row used
/// to carry the same one — so twenty-six numbers arrived as one undivided
/// scroll, in the order they happened to be written, with the charge-shaping
/// knobs interleaved with the debris ballistics.
///
/// So the ordering is the fix, and the order is by *what you would reach for
/// first*: the charge type, then how big the bang is, then the two things you
/// can see it do, then the numbers that only matter once you are tuning one of
/// those. Nothing was removed — a knob that is hard to find is a different
/// problem from a knob that should not exist, and `crack_rays` in particular
/// is a settled A/B (see its own field doc) that still has to stay reachable.
pub const EXPLOSION_CATEGORY: &str = "charge";
pub const EXPLOSION_SIZE_CATEGORY: &str = "the bang";
pub const EXPLOSION_CRACK_CATEGORY: &str = "cracks";
pub const EXPLOSION_DEBRIS_CATEGORY: &str = "rubble, smoke and fire";
pub const EXPLOSION_ADVANCED_CATEGORY: &str = "advanced";

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
    /// **A closed set of named states rather than a number.** `None` is the
    /// ordinary numeric entry; `Some(labels)` makes `value` an index into
    /// `labels`, which the panel renders in place of the figure and draws as
    /// discrete segments instead of a fill.
    ///
    /// Exists because the two controls the options menu most needed — what
    /// time of day it is, and what the weather is doing — are *modes*, and a
    /// slider is the wrong instrument for a mode twice over: `3.000` is not
    /// an answer to "what is the sky doing", and a value between two states
    /// is not a state. Everything else about a choice is a number, so it
    /// reuses the whole registry, adjust and clamp path rather than forking
    /// it; only the rendering and the wrap differ.
    ///
    /// **An index past the end is a real, reachable value and means "held at
    /// something unnamed"** — see `weather::Pin::of`. The panel draws it as
    /// `HELD`; `max` deliberately does *not* cover it, because it is a state
    /// to be read and left, never one to be selected into.
    pub options: Option<Vec<&'static str>>,
}

impl Tunable {
    /// A float-valued tunable — the common case.
    fn float(group: TunableGroup, category: &str, name: &str, value: f32, min: f32, max: f32, step: f32) -> Self {
        Self {
            group,
            category: category.into(),
            name: name.into(),
            value,
            min,
            max,
            step,
            integral: false,
            options: None,
        }
    }

    /// An integer-valued one, whose `.ron` field must never be written with
    /// a decimal point — see `integral`.
    fn integer(group: TunableGroup, category: &str, name: &str, value: f32, min: f32, max: f32, step: f32) -> Self {
        Self {
            group,
            category: category.into(),
            name: name.into(),
            value,
            min,
            max,
            step,
            integral: true,
            options: None,
        }
    }

    /// One of a closed set of named states — see [`Tunable::options`].
    ///
    /// `index` may legitimately be `options.len()`, meaning the live state is
    /// not one of the named ones; `max` stays at the last *named* index so
    /// that no adjustment can ever land back on it.
    fn choice(group: TunableGroup, category: &str, name: &str, index: usize, options: Vec<&'static str>) -> Self {
        let last = options.len().saturating_sub(1) as f32;
        Self {
            group,
            category: category.into(),
            name: name.into(),
            value: index as f32,
            min: 0.0,
            max: last,
            step: 1.0,
            integral: true,
            options: Some(options),
        }
    }

    /// What this entry reads as on screen: the chosen label for a choice,
    /// the number for everything else.
    ///
    /// **`HELD` is not a placeholder.** An index past the end of the list is
    /// the live state being something no menu entry names, which is
    /// reachable whenever anything but the panel writes the underlying value
    /// (a test, a harness, a hand-edited asset). Rendering it as the first or
    /// nearest entry instead would be a readout that disagrees with the
    /// world — `CLAUDE.md`'s "a knob whose value you cannot see is a knob you
    /// cannot tell is disconnected", one step worse because it would be
    /// confidently wrong rather than absent.
    pub fn display(&self) -> String {
        match &self.options {
            Some(options) => options.get(self.value.round().max(0.0) as usize).map_or("HELD", |s| s).to_string(),
            None if self.integral => format!("{}", self.value.round() as i64),
            None => format!("{:.3}", self.value),
        }
    }

    /// The value one press of the arrow keys moves this to.
    ///
    /// **Choices wrap; numbers clamp**, and the difference is the point. A
    /// slider that stops at its end tells you there is no more headroom,
    /// which is information; a mode list that stops at its end is just a
    /// list you have to walk back down. Wrapping makes any state reachable
    /// in whichever direction is shorter, which is what a `LEFT/RIGHT`
    /// selector on nine entries needs to be usable at all.
    ///
    /// From an unnamed held state (`value` past the end) *either* direction
    /// goes to entry 0 — the running state. That is the one thing anybody in
    /// that position wants, and it is reachable without knowing how they got
    /// there.
    pub fn stepped(&self, sign: i32) -> f32 {
        let Some(options) = &self.options else {
            return (self.value + sign as f32 * self.step).clamp(self.min, self.max);
        };
        let len = options.len() as i32;
        if len == 0 {
            return 0.0;
        }
        let index = self.value.round().max(0.0) as i32;
        if index >= len {
            return 0.0;
        }
        (index + sign).rem_euclid(len) as f32
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
    // The charge type first, and on its own, because it is the row that makes
    // the other twenty-six optional: it moves the whole tuning at once, so
    // "what can a blast be" is answerable by walking one row rather than by
    // guessing which number to sweep. `HELD` the moment anything else is
    // touched -- see `Preset::of`.
    let charge_labels: Vec<&'static str> = Preset::ALL.iter().map(|p| p.label()).collect();
    let charge = Preset::of(t)
        .and_then(|p| Preset::ALL.iter().position(|q| *q == p))
        .unwrap_or(charge_labels.len());
    let size = EXPLOSION_SIZE_CATEGORY;
    let crack = EXPLOSION_CRACK_CATEGORY;
    let debris = EXPLOSION_DEBRIS_CATEGORY;
    let adv = EXPLOSION_ADVANCED_CATEGORY;
    vec![
        Tunable::choice(g, c, "type", charge, charge_labels),
        // How big the bang is. Three numbers, and between them they set the
        // scale of everything below -- most of the rest are fractions *of*
        // these.
        Tunable::float(g, size, "radius", t.radius, 1.0, 80.0, 1.0),
        Tunable::float(g, size, "strength", t.strength, 10.0, 600.0, 10.0),
        Tunable::float(g, size, "duration", t.duration, 1.0, 40.0, 1.0),
        // The joint fabric (F) and the growth beat -- everything that decides
        // what the rock looks like afterwards, which is the half of an
        // explosion that is still there once the smoke clears.
        //
        // `joint_reach` is a multiple of `radius` like `crack_reach`; the
        // next two are 0..1 fractions. These are the density controls the
        // owner's verdict on the pattern lands on -- *"I like a little of it,
        // but there is too much"* -- so they are exactly the kind of
        // judged-by-eye question this panel exists for. A fourth control is
        // `stone.ron`'s `joint_spacing`, which is a material field and is
        // already listed by `from_materials`.
        Tunable::float(g, crack, "joint_reach", t.joint_reach, 0.0, 6.0, 0.1),
        Tunable::float(g, crack, "joint_density", t.joint_density, 0.0, 1.0, 0.05),
        Tunable::float(g, crack, "joint_open_fraction", t.joint_open_fraction, 0.0, 1.0, 0.05),
        // The aperture cap. `1` is the uniform one-cell seam that shipped
        // before the ladder existed, and is the A/B control rather than a
        // degenerate setting -- hence a range that starts there rather than
        // at 0, which would be "no seams at all" and is what
        // `joint_open_fraction` already says.
        Tunable::integer(g, crack, "joint_seam_width", t.joint_seam_width as f32, 1.0, 6.0, 1.0),
        // `crack_growth` and `crack_stagger` are the pair that decide whether
        // the fissures read as a thing that *happens* or as a graphic stamped
        // on the stone, which is a judged-in-the-hand question and therefore
        // exactly what this panel is for. Floored at 1, matching the clamp
        // where it is read: `0` freezes the star half-drawn, so it is not a
        // setting anyone can want. `crack_stagger` is in frames, so a
        // whole-frame step like `duration`'s rather than a fractional one.
        Tunable::integer(g, crack, "crack_growth", t.crack_growth as f32, 1.0, 20.0, 1.0),
        Tunable::float(g, crack, "crack_stagger", t.crack_stagger, 0.0, 40.0, 1.0),
        // A temperature, so the same 10-degree granularity the material
        // temperatures use. Capped well below `flash_temperature`'s 2000:
        // `render.rs`'s glow ramp is saturated by ~420 and stone cannot
        // ignite from it, so the range past that buys nothing visible.
        Tunable::float(g, crack, "crack_glow_temperature", t.crack_glow_temperature, 0.0, 1000.0, 10.0),
        // The collar the finished web calves off the rim -- the beat where
        // the cracks stop being a picture and pieces come away.
        Tunable::integer(g, crack, "calve_depth", t.calve_depth as f32, 0.0, 32.0, 1.0),
        // What flies, what is left burning, and what is left standing in the
        // air afterwards.
        Tunable::float(g, debris, "debris_fraction", t.debris_fraction, 0.0, 1.0, 0.05),
        Tunable::float(g, debris, "vaporize_fraction", t.vaporize_fraction, 0.0, 1.0, 0.02),
        Tunable::float(g, debris, "smoke_fraction", t.smoke_fraction, 0.0, 1.0, 0.02),
        Tunable::float(g, debris, "fireball_fraction", t.fireball_fraction, 0.0, 3.0, 0.1),
        Tunable::float(g, debris, "flash_temperature", t.flash_temperature, 0.0, 2000.0, 50.0),
        // A *per-frame* retention, so the interesting band is the top tenth
        // (0.94 fades in ~90 frames, 0.99 in ~550) and a 0.05 step would
        // step straight over it -- `swim_damp`'s 0.01 for the same reason.
        Tunable::float(g, debris, "afterglow_retention", t.afterglow_retention, 0.5, 1.0, 0.01),
        // Below here: the ballistics of a single grain, the confinement probe
        // (R2), and the superseded radial walker. All real, none of them the
        // first thing anybody wants, and `crack_rays` in particular defaults
        // to `0` because the fabric replaced the walker rather than joining
        // it -- 4-6 puts the old star back on top for an A/B.
        //
        // `containment_floor` is a multiple of `radius`, not a 0..1 fraction,
        // hence the wider range and coarser step.
        Tunable::float(g, adv, "containment_floor", t.containment_floor, 0.0, 5.0, 0.1),
        Tunable::float(g, adv, "confined_cavity_fraction", t.confined_cavity_fraction, 0.0, 1.0, 0.05),
        Tunable::float(g, adv, "shockwave_multiplier", t.shockwave_multiplier, 1.0, 4.0, 0.1),
        Tunable::float(g, adv, "pierce_divisor", t.pierce_divisor, 1.0, 200.0, 2.0),
        Tunable::float(g, adv, "speed_per_strength", t.speed_per_strength, 0.0, 0.5, 0.005),
        Tunable::float(g, adv, "debris_jitter", t.debris_jitter, 0.0, 2.0, 0.05),
        Tunable::float(g, adv, "heat_fraction", t.heat_fraction, 0.5, 20.0, 0.5),
        Tunable::integer(g, adv, "crack_rays", t.crack_rays as f32, 0.0, 48.0, 1.0),
        Tunable::float(g, adv, "crack_reach", t.crack_reach, 0.0, 6.0, 0.1),
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
        // Capped below `PLAYER_WIDTH` (7): at a full course across him a
        // drift would be walked through, which is the thing the wade line
        // exists to prevent. 0 is the old veto, kept reachable for A/B.
        Tunable::integer(g, c, "shoulder_grains", t.shoulder_grains as f32, 0.0, 6.0, 1.0),
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
        // Capped at the bore box's own short side (`PLAYER_WIDTH + 2` = 9):
        // past that a stroke is the whole box, which `bore_slice` clamps to
        // anyway, so a wider range would be knob travel that does nothing.
        Tunable::integer(g, c, "bore_bite", t.bore_bite as f32, 1.0, 9.0, 1.0),
        Tunable::integer(g, c, "hammer_reach", t.hammer_reach as f32, 2.0, 40.0, 1.0),
        // `rigid::MIN_STRIKE_RADIUS` is 6, so anything under it is the same
        // blow -- the low end is reachable on purpose, to make that floor
        // visible in the hand rather than only in a comment.
        Tunable::integer(g, c, "hammer_radius", t.hammer_radius as f32, 1.0, 16.0, 1.0),
        Tunable::float(g, c, "hammer_force", t.hammer_force, 0.0, 12.0, 0.25),
        Tunable::integer(g, c, "hammer_cooldown", t.hammer_cooldown as f32, 1.0, 60.0, 1.0),
        Tunable::float(g, c, "hammer_recoil", t.hammer_recoil, 0.0, 2.0, 0.05),
        Tunable::integer(g, c, "chop_reach", t.chop_reach as f32, 2.0, 40.0, 1.0),
        Tunable::integer(g, c, "chop_radius", t.chop_radius as f32, 1.0, 12.0, 1.0),
        Tunable::integer(g, c, "chop_cooldown", t.chop_cooldown as f32, 1.0, 40.0, 1.0),
        Tunable::float(g, c, "chop_yield", t.chop_yield, 0.0, 1.0, 0.05),
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
        "shoulder_grains" => t.shoulder_grains = value.clamp(0.0, 6.0).round() as u8,
        "wade_slowdown" => t.wade_slowdown = value,
        "buoyancy" => t.buoyancy = value,
        "swim_damp" => t.swim_damp = value,
        "stroke_impulse" => t.stroke_impulse = value,
        "stroke_cooldown" => t.stroke_cooldown = value.max(1.0).round() as u8,
        "mantle_reach" => t.mantle_reach = value.max(0.0).round() as u8,
        "shake_reach" => t.shake_reach = value.max(1.0).round() as u8,
        "bore_bite" => t.bore_bite = value.max(1.0).round() as u8,
        "hammer_reach" => t.hammer_reach = value.max(1.0).round() as u8,
        "hammer_radius" => t.hammer_radius = value.max(1.0).round() as u8,
        "hammer_force" => t.hammer_force = value.max(0.0),
        "hammer_cooldown" => t.hammer_cooldown = value.max(1.0).round() as u8,
        "hammer_recoil" => t.hammer_recoil = value.max(0.0),
        "chop_reach" => t.chop_reach = value.max(1.0).round() as u8,
        "chop_radius" => t.chop_radius = value.max(1.0).round() as u8,
        "chop_cooldown" => t.chop_cooldown = value.max(1.0).round() as u8,
        "chop_yield" => t.chop_yield = value.clamp(0.0, 1.0),
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

/// **The two mode rows the WORLD menu opens with** — what time it is, and
/// what the weather is doing.
///
/// Separate from [`from_clock`] rather than folded into it, and the split is
/// not cosmetic. Those five are *rates*: whole multiples of baseline, floored
/// at 1, capped at [`MAX_SLOWDOWN`], and three tests assert exactly that over
/// everything `from_clock` returns. These two are choices with none of those
/// properties, and widening those assertions to admit them would have thrown
/// away the guard rather than extended it.
///
/// They are listed first because they are the two controls the owner asked
/// for and the two a player reaches for; the rates are how fast the world
/// ages, which is a session setting you set once.
///
/// Takes the weather override rather than a `&World` so that the whole
/// registry stays buildable from plain values — `from_clock`'s own reason for
/// taking a `&Clock`.
pub fn from_pins(clock: &Clock, weather_override: Option<Weather>) -> Vec<Tunable> {
    let g = TunableGroup::World;
    let c = WORLD_CATEGORY;
    // `map_or(len, ..)` is the "held at something unnamed" index — see
    // `Tunable::options`. Reached whenever anything but this panel wrote the
    // underlying value.
    let sky_labels: Vec<&'static str> = SkyPin::ALL.iter().map(|p| p.label()).collect();
    let sky = SkyPin::of(clock.sky_hold)
        .and_then(|p| SkyPin::ALL.iter().position(|q| *q == p))
        .unwrap_or(sky_labels.len());
    let weather_labels: Vec<&'static str> = WeatherPin::ALL.iter().map(|p| p.label()).collect();
    let weather = WeatherPin::of(weather_override)
        .and_then(|p| WeatherPin::ALL.iter().position(|q| *q == p))
        .unwrap_or(weather_labels.len());
    vec![
        Tunable::choice(g, c, "time_of_day", sky, sky_labels),
        Tunable::choice(g, c, "weather", weather, weather_labels),
    ]
}

/// Which [`SkyPin`] a `time_of_day` row's value selects, and which
/// [`WeatherPin`] a `weather` row's does — [`from_pins`]' other half.
///
/// Returns the pin rather than applying it, because applying is not a field
/// write: both go through `World` seams that also have to wake the field
/// (`World::set_sky_hold`, `World::set_weather_pin`), and a function here
/// that took a `&mut Clock` could do only half of it. That asymmetry with
/// `apply_clock` is deliberate and is the reason this is named `select` and
/// not `apply`.
///
/// An out-of-range index cannot arrive from [`Tunable::stepped`], which sends
/// every adjustment to 0; it is defended against anyway because the value
/// also reaches here from a pinned readout (`App::adjust_pinned`) rebuilt
/// against a list that may have changed underneath it.
pub fn select_sky_pin(value: f32) -> SkyPin {
    SkyPin::ALL.get(value.round().max(0.0) as usize).copied().unwrap_or_default()
}

/// As [`select_sky_pin`], for the weather row.
pub fn select_weather_pin(value: f32) -> WeatherPin {
    WeatherPin::ALL.get(value.round().max(0.0) as usize).copied().unwrap_or_default()
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
        // **Not a field write** -- it replaces the whole struct. It stays
        // here rather than being special-cased in `App::apply_adjust` the way
        // the two WORLD pins are, because unlike those it needs nothing but
        // the tuning itself: no `World` seam to wake, no phase to re-anchor.
        // An index past the last preset is the `HELD` readout and is never
        // selectable (`Tunable::stepped` wraps within the named range), so a
        // stray value leaves the tuning alone instead of snapping it to a
        // charge the player did not choose.
        "type" => {
            if let Some(p) = select_charge_preset(value) {
                *t = p.tuning();
            }
        }
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
        // Floored at 1 for the reason `crack_growth` is: the panel is not the
        // only caller, and a 0 written in from anywhere turns every seam into
        // a score, which `joint_open_fraction` already expresses and which
        // reads here as the fabric having broken.
        "joint_seam_width" => t.joint_seam_width = value.max(1.0).round() as u32,
        _ => {}
    }
}

/// Which [`Preset`] a `type` row's value selects — `None` for an index that
/// names no charge, which is `HELD` and must not be selectable into.
///
/// Free-standing and `pub` for the same reason `select_sky_pin` is: the
/// mapping from a row index to a named state is the half of a choice row a
/// test can pin without building an `App`.
pub fn select_charge_preset(value: f32) -> Option<Preset> {
    Preset::ALL.get(value.max(0.0).round() as usize).copied()
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
    use crate::sim::weather::Precipitation;
    use crate::sim::material;

    /// **`next` and `prev` must be exact inverses, in both directions and
    /// over every variant.** The bug this replaces is the one a `match` over
    /// an enum invites: `World` was added, `next` and `all` were updated, and
    /// `prev` was left mapping `Physics` to `Player` -- correct before the
    /// variant existed, silently skipping it after. It survived because
    /// nothing calls `prev` yet, so no test and no lint had any reason to
    /// look at it.
    ///
    /// Asserted as a round trip rather than as a table of expected pairs: a
    /// table has to be edited when a variant is added, which is the very
    /// thing that went wrong, whereas a round trip over `all()` simply grows.
    #[test]
    fn next_and_prev_are_inverses() {
        for g in TunableGroup::all() {
            assert_eq!(g.next().prev(), g, "{:?}.next().prev() left the cycle", g);
            assert_eq!(g.prev().next(), g, "{:?}.prev().next() left the cycle", g);
        }
        // ...and `next` alone must visit every group, or a menu exists that
        // no amount of pressing the cycle key can reach. `WORLD` is the one
        // that matters today: every world-speed knob lives behind it.
        let mut seen = vec![TunableGroup::Physics];
        let mut g = TunableGroup::Physics;
        for _ in 1..TunableGroup::all().len() {
            g = g.next();
            assert!(!seen.contains(&g), "cycling revisited {:?} before covering every group", g);
            seen.push(g);
        }
        assert_eq!(g.next(), TunableGroup::Physics, "the cycle must close");
        for want in TunableGroup::all() {
            assert!(seen.contains(&want), "{:?} is unreachable by cycling the panel key", want);
        }
    }

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
            joint_seam_width,
        );
        let listed = from_explosion(&base);
        for field in fields {
            assert!(
                listed.iter().any(|t| t.name == field),
                "explosion.{field} exists on Tuning but has no panel entry -- it cannot be swept in play"
            );
        }
        // **Counted over the numeric rows only, and the choice rows are named
        // separately rather than excused.** The menu now carries one row that
        // is deliberately not a field -- `type`, which writes a whole
        // `Preset` -- and widening the count to "fields + 1" would have let a
        // second such row appear with nothing noticing. Both halves still
        // fail closed: a new field with no row, and a new row with no field.
        let (choices, numeric): (Vec<_>, Vec<_>) = listed.iter().partition(|t| t.options.is_some());
        assert_eq!(numeric.len(), fields.len(), "from_explosion lists a numeric entry that is not a field on Tuning");
        let choice_names: Vec<&str> = choices.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(choice_names, ["type"], "an unexpected choice row appeared in the EXPLOSION menu");

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
    /// **The tab strip and `Tab` must walk the same order.** The panel draws
    /// `all()` left to right and `next()` is what the key does, so a list
    /// that disagreed would show tabs in an order the key does not follow —
    /// which reads as the key being broken, not the list.
    #[test]
    fn all_is_the_cycle_order() {
        let all = TunableGroup::all();
        for w in all.windows(2) {
            assert_eq!(w[0].next(), w[1], "{} is drawn before {} but Tab does not go there", w[0].label(), w[1].label());
        }
        assert_eq!(all[all.len() - 1].next(), all[0], "the cycle must close");
    }

    /// **The two pin rows must round-trip through the panel**, the same
    /// contract `every_registered_world_knob_can_be_written_back` holds the
    /// rate knobs to — and it needs its own test because these do not go
    /// through `apply_clock` at all (they are not field writes; see
    /// `from_pins`).
    #[test]
    fn every_named_sky_and_weather_can_be_selected_and_reads_back() {
        for (i, pin) in SkyPin::ALL.iter().enumerate() {
            assert_eq!(select_sky_pin(i as f32), *pin, "row index {i} does not select {}", pin.label());
            let mut c = Clock::default();
            c.set_rates(0, |c| c.sky_hold = pin.hold());
            let row = from_pins(&c, None).into_iter().find(|t| t.name == "time_of_day").expect("listed");
            assert_eq!(row.display(), pin.label(), "a held {} does not read back as itself", pin.label());
        }
        for (i, pin) in WeatherPin::ALL.iter().enumerate() {
            assert_eq!(select_weather_pin(i as f32), *pin, "row index {i} does not select {}", pin.label());
            let row = from_pins(&Clock::default(), pin.weather())
                .into_iter()
                .find(|t| t.name == "weather")
                .expect("listed");
            assert_eq!(row.display(), pin.label(), "a pinned {} does not read back as itself", pin.label());
        }
    }

    /// **A state no menu entry names must read as `HELD`, and any adjustment
    /// from it must reach the running state.**
    ///
    /// The one case the round-trip above cannot see, and it is reachable:
    /// the sky hold is a `pub` field in an asset file and the weather
    /// override takes any `Weather`. Rendering an unnamed hold as one of the
    /// presets would be a readout that is confidently wrong about what the
    /// world is doing.
    #[test]
    fn an_unnamed_hold_reads_as_held_and_any_press_releases_it() {
        let mut c = Clock::default();
        c.set_rates(0, |c| c.sky_hold = Some(137));
        let odd = Weather { intensity: 0.3, kind: Precipitation::Rain, wind: 0.1, chill: 0.0 };
        let rows = from_pins(&c, Some(odd));
        for t in &rows {
            assert_eq!(t.display(), "HELD", "{} should read as HELD when nothing names it", t.name);
            // Either direction, because somebody in this state does not know
            // which way is "back".
            for sign in [-1, 1] {
                assert_eq!(t.stepped(sign), 0.0, "{} must release to the running state", t.name);
            }
        }
        assert_eq!(select_sky_pin(rows[0].stepped(1)), SkyPin::Live);
        assert_eq!(select_weather_pin(rows[1].stepped(1)), WeatherPin::Live);
    }

    /// **A choice wraps and a number clamps** — see `Tunable::stepped` for
    /// why those are different and both right. A choice that clamped would
    /// make the last entry of a nine-item list eight presses from the first.
    #[test]
    fn choices_wrap_where_numbers_clamp() {
        let rows = from_pins(&Clock::default(), None);
        let sky = rows.iter().find(|t| t.name == "time_of_day").expect("listed");
        assert_eq!(sky.value, 0.0, "a fresh clock is not held");
        assert_eq!(sky.stepped(-1), sky.max, "left from the first entry wraps to the last");
        let last = Tunable { value: sky.max, ..sky.clone() };
        assert_eq!(last.stepped(1), 0.0, "right from the last entry wraps to the first");

        // The numeric half, on a knob sitting at its own floor.
        let day = from_clock(&Clock::default()).into_iter().find(|t| t.name == "day_minutes").expect("listed");
        assert_eq!(day.value, day.min);
        assert_eq!(day.stepped(-1), day.min, "a number at its floor stays there");
        assert_eq!(day.stepped(1), day.min + day.step);
    }

    /// Every choice draws its label, and every number draws a figure — the
    /// property the panel's value column depends on, asserted over the whole
    /// registry rather than the two rows that motivated it.
    #[test]
    fn a_choice_displays_a_label_and_a_number_displays_a_number() {
        let mut all = from_materials(&MaterialRegistry::builtin());
        all.extend(from_explosion(&Explosion::default()));
        all.extend(from_player(&Player::default()));
        all.extend(from_clock(&Clock::default()));
        all.extend(from_pins(&Clock::default(), None));
        let mut choices = 0;
        for t in &all {
            let shown = t.display();
            match &t.options {
                Some(options) => {
                    choices += 1;
                    assert!(
                        options.contains(&shown.as_str()) || shown == "HELD",
                        "{} shows {shown}, which is not one of its options",
                        t.name
                    );
                }
                None => assert!(
                    shown.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-'),
                    "{} shows {shown}, which is not a number",
                    t.name
                ),
            }
        }
        // The two WORLD pins, and EXPLOSION's charge type. Counted rather
        // than listed on purpose: a *new* choice row is the thing this
        // catches, and it catches it by failing rather than by being widened
        // to admit whatever appeared.
        assert_eq!(choices, 3, "the registry should hold the two pin rows and the charge type as choices");
    }

}
