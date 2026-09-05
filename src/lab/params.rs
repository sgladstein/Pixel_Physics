//! **The numbers behind the verbs, reachable while the box is running.**
//!
//! Owner direction, 2026-08-30: *"Your goals are not tweaking and optimizing
//! evolution now. Give me the tools, data, access to necessary parameters that
//! need to be tweaked and I do that testing myself in the game. That is the
//! game."*
//!
//! So this file is **not** a balance pass and does not set a single default.
//! It is the list of values the bar's six verbs are made of, each with the
//! range it may be moved through, where its live value is read from, and where
//! a change is written back to. `ui` draws it; nothing here knows what a pixel
//! is.
//!
//! # One registry, not a second one
//!
//! Every entry is a [`crate::tunables::Tunable`] — the sandbox's own
//! `(category, name, value, min, max, step)` record, with its `display`,
//! `stepped` and clamping reused verbatim. What is new here is the other half
//! of a knob, which the sandbox does not have for any of these: a [`Knob`]
//! saying *where the number lives*, so that one `read` and one `write` cover a
//! material field, a creature block, a growth arm and the bed's build spec
//! without four parallel panels.
//!
//! Entries are tagged [`TunableGroup::Lab`], which is deliberately outside the
//! sandbox panel's own menu cycle — see that variant's doc.
//!
//! # What is left out, and why
//!
//! **A panel with four hundred rows is not access, it is a haystack.** Forty-
//! odd entries across four pages is what a player experimenting with a
//! biosphere reaches for; the rest of the engine's several hundred material
//! fields stay in the sandbox's `O` panel where a sweep belongs. Adding one is
//! two lines — a row in [`registry`] and an arm in [`write`] — and
//! `every_writable_parameter_actually_moves` fails if you forget the second.
//!
//! Three things are registered **read-only**, and that is a finding rather
//! than a gap: `light_weight`, `branch_chance` and `upward_weight` are
//! `ByOrder<f32>`, four values indexed by branch order, and the type's only
//! constructor from outside is `uniform`. Writing one from a single-number row
//! would flatten `tree.ron`'s authored `[0.15, 0.3, 0.5, 0.6]` ramp into four
//! copies of one number — a change to three values dressed up as a change to
//! one. They are shown with every tier so the player can *see* them, and the
//! panel says plainly that it cannot move them.

use crate::sim::material;
use crate::sim::organism::{self, Behavior, CellType, SpeciesId};
use crate::sim::world::{self, World};
use crate::tunables::{self, Tunable, TunableGroup};

use super::scene::LabBox;

/// Which page of the parameters panel an entry sits on.
///
/// **Four, and they are the four things in the box** rather than four
/// technical origins: the player is looking for "why won't my ants dig" and
/// not for "which struct is this on". `GROUND` mixes three materials, `PLANT`
/// and `ANTS` each mix a species' creature block with its growth arms, and
/// `BOX` mixes a material field (the lamp) with the bed's build spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// Soil, packed soil and water — what everything else is standing in.
    Ground,
    /// The plant the bar's species chip currently has armed.
    Plant,
    /// The colony species `found_colony` releases.
    Ants,
    /// The bed itself, and the lamps over it.
    Box,
    /// **How heredity itself works** — global to the world rather than to any
    /// one species, and its own page because there are now six of them and
    /// the plant page was mixing *this species' numbers* with *how breeding
    /// behaves for everything*. Two different questions, and the second is
    /// what the lab is for.
    Heredity,
}

/// In tab order. One list, so the tab strip, the key and the tests cannot
/// disagree about what pages exist — `ui::TOOLS`' reason.
pub const GROUPS: [Group; 5] = [Group::Ground, Group::Plant, Group::Heredity, Group::Ants, Group::Box];

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Ground => "GROUND",
            Group::Plant => "PLANT",
            Group::Ants => "ANTS",
            Group::Box => "BOX",
            Group::Heredity => "HEREDITY",
        }
    }

    /// What the page is for, shown on hover over its own tab.
    pub fn note(self) -> &'static str {
        match self {
            Group::Ground => "WHAT THE BED IS MADE OF. SOIL IS WHAT ROOTS GO INTO AND ANTS DIG THROUGH; PACKED SOIL IS WHAT AN ANT LEAVES BEHIND WHEN IT DIGS, AND IT IS THE ONLY REASON A TUNNEL STAYS OPEN. CHANGES HERE ARE FELT ON THE NEXT TICK.",
            Group::Plant => "THE PLANT THE SPECIES CHIP ON THE BAR HAS ARMED. THESE ARE THE SPECIES' OWN NUMBERS, NOT ONE INDIVIDUAL'S -- MOVING ONE CHANGES EVERY PLANT OF THAT SPECIES ALREADY STANDING, ON THE NEXT TICK, AS WELL AS EVERY SEED YOU PLANT AFTERWARDS.",
            Group::Ants => "THE COLONY SPECIES. SAME RULE AS THE PLANT PAGE: THESE ARE THE SPECIES' NUMBERS AND THEY REACH EVERY ANT ALIVE. AN INDIVIDUAL'S OWN INHERITED TRAITS ARE ON THE CELL PAGE -- CLICK AN ANT WITH THE LOOK TOOL.",
            Group::Box => "THE BED AND THE LAMPS OVER IT. THE LAMP IS LIVE. EVERYTHING ELSE HERE IS THE SPEC THE BOX IS BUILT FROM, SO IT TAKES EFFECT WHEN YOU REBUILD -- CHANGE IT, THEN PRESS REBUILD.",
            Group::Heredity => "HOW BREEDING BEHAVES, FOR EVERY PLANT IN THE BOX AT ONCE -- NOT ONE SPECIES' NUMBERS. THIS IS THE PAGE THE LAB IS ACTUALLY FOR. EVERYTHING HERE IS FELT AT THE NEXT SEED RATHER THAN THE NEXT TICK, SO GIVE IT A GENERATION BEFORE DECIDING IT DID NOTHING, AND NONE OF IT IS SAVED TO A SPECIES FILE -- IT LASTS THE SESSION.",
        }
    }
}

/// **Where a parameter's value lives**, and therefore how it is read, how it
/// is written, and whether it can be saved.
///
/// A knob rather than a `(category, name)` string pair, because five of these
/// six kinds resolve through a different registry and two of them need a cell
/// type named as well. `Tunable::category`/`name` are still carried for the
/// save path, which is a text edit keyed on the field's own name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Knob {
    /// A `Material` field on the named material — live on the next tick, and
    /// saved by the same targeted span edit the sandbox's panel uses.
    Material { material: &'static str, field: &'static str },
    /// A `CreatureDef` field on the named species.
    Creature { species: String, field: &'static str },
    /// One of `CreatureDef::traits` — the heritable body traits, which is an
    /// authored *ancestral* value: it seeds a newborn and does not reach an
    /// animal that is already alive.
    CreatureTrait { species: String, slot: usize },
    /// A scalar on the named species' `GrowingTip` `Grow` arm. The cell type
    /// is fixed here on purpose: every one of these names exists twice in a
    /// species file, once for the shoot and once for the root.
    Grow { species: String, field: &'static str },
    /// A scalar on the named species' `Reproduce` arm.
    Reproduce { species: String, field: &'static str },
    /// A `pub` field on `Species` itself.
    Species { species: String, field: &'static str },
    /// A field of the bed's build spec. **Takes effect on the next rebuild**,
    /// which every row of it says.
    Bed { field: &'static str },
    /// **How hard heredity drifts** — a global of the plant line, live on the
    /// next birth.
    ///
    /// Its own kind, and **deliberately not a `Species` field**, which is the
    /// correction this row cost. The natural move is a `#[serde(default)]`
    /// scalar on the species file, on `design-philosophy.md` §2a's rule that a
    /// constant a non-programmer might tune graduates to `.ron` immediately.
    /// `plant::fate_mutation_chance`'s own doc had already ruled against it:
    /// species reach the binary through `include_str!`, so editing a `.ron`
    /// and re-running a prebuilt harness gives **bit-identical runs**, and a
    /// sweep over a mutation rate is exactly that shape. A runtime cell has
    /// neither problem, and it is honest that these are one number for every
    /// plant in the world rather than a property of a species.
    Heredity { field: &'static str },
    /// **A rule of the simulation itself, on or off** — a `bool` on
    /// [`World`], live on the next tick.
    ///
    /// Its own kind rather than a `Knob::Material` with a 0/1 span, because
    /// what it switches is not any one material's number: turning plant
    /// collapse off holds wood, rootwood, leaf and everything else a species
    /// file may name, and it reaches a second rule in `structural.rs` that no
    /// material field appears in at all. A row here prints `ON`/`OFF` rather
    /// than `1.000`/`0.000`, which is [`Param::shown`]'s whole purpose.
    Rule { field: &'static str },
    /// Shown, and not changeable from here. The panel draws no `-`/`+` pair on
    /// one of these and [`write`] refuses it; see this module's own doc for
    /// the three that are like this and why.
    ReadOnly,
}

/// One registered parameter: the number, where it lives, and what it means.
pub struct Param {
    pub group: Group,
    pub knob: Knob,
    /// The value, its range and its step — the sandbox's own record type, so
    /// `display`, `stepped` and the clamp are the ones already in use.
    pub tunable: Tunable,
    /// One sentence, shown while the cursor is over the row. Carried on the
    /// entry rather than in a table keyed by name, for `ui::Row::note`'s
    /// reason: a table beside the thing it describes goes stale.
    pub note: String,
    /// What the row prints instead of `tunable.display()`, for an entry whose
    /// value is not one number. `None` for every ordinary row.
    pub shown: Option<String>,
}

impl Param {
    /// Whether the panel draws a `-`/`+` pair on this row.
    pub fn writable(&self) -> bool {
        self.knob != Knob::ReadOnly
    }

    /// What the row prints in its value column.
    ///
    /// **Three decimals only where three decimals mean something.**
    /// `Tunable::display` gives every float `{:.3}`, which is right for a
    /// panel of material constants in the 0..2 range and wrong here: this
    /// registry holds `10000.000` (a seed half-life in ticks) beside `0.015`
    /// (a mutation rate), and at 5x7 glyphs the first of those is nine
    /// characters that ran straight through the `-` face on the first contact
    /// sheet of this page. The precision follows the magnitude, so every value
    /// this registry can hold fits its column.
    pub fn display(&self) -> String {
        if let Some(shown) = &self.shown {
            return shown.clone();
        }
        if self.tunable.integral {
            return self.tunable.display();
        }
        let v = self.tunable.value;
        match v.abs() {
            a if a >= 1000.0 => format!("{v:.0}"),
            a if a >= 100.0 => format!("{v:.1}"),
            a if a >= 10.0 => format!("{v:.2}"),
            _ => format!("{v:.3}"),
        }
    }

    /// The range column: how far this row may be moved in either direction.
    ///
    /// **`TO` rather than a dash, and the numbers trimmed.** `0.00-80.00` is
    /// ten characters at 5x7 to say `0 TO 80`, and on the two rows whose floor
    /// is negative — a gut bias runs -1 to +1 — the dash form reads
    /// `-1.00-1.00`, which has three minus signs in it and means none of them.
    pub fn range(&self) -> String {
        if !self.writable() {
            return String::new();
        }
        format!("{} TO {}", self.trim(self.tunable.min), self.trim(self.tunable.max))
    }

    /// A bound with no trailing zeros: `0`, `0.5`, `60000`.
    fn trim(&self, v: f32) -> String {
        if self.tunable.integral {
            return format!("{}", v.round() as i64);
        }
        let s = format!("{v:.2}");
        let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.') } else { &s };
        if s.is_empty() || s == "-" { "0".to_string() } else { s.to_string() }
    }

    /// Where the value sits in its own range, `0.0..=1.0` — the fill the row
    /// draws under itself, so "how much headroom is left" is readable without
    /// reading either number.
    pub fn fraction(&self) -> f32 {
        let span = self.tunable.max - self.tunable.min;
        if span <= 0.0 {
            return 0.0;
        }
        ((self.tunable.value - self.tunable.min) / span).clamp(0.0, 1.0)
    }
}

/// **The span a row may be moved through, and how far one press moves it.**
///
/// Three numbers that always travel together, as one argument. Not merely to
/// get under a lint: a registration reads as a sentence — *this knob, on this
/// material, over this span* — and three loose floats in a row of nine
/// positional arguments is exactly where a max and a step get swapped.
#[derive(Clone, Copy)]
pub struct Span {
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

const fn span(min: f32, max: f32, step: f32) -> Span {
    Span { min, max, step }
}

/// How small and how large the box may be built, in cells per side.
///
/// **The ceiling is a memory decision, not a simulation one**, and it is the
/// owner's (2026-09-01). A chamber costs `12 B` per cell for the grid plus
/// `4 B` for the two pheromone planes, so the bill is `w * h * 16 B` — the
/// shipped 512x320 is ~2.5 MB and 4096x4096 is ~268 MB. Nothing stops a rack
/// of large chambers exhausting a machine, which is why the two size rows
/// print what the current setting actually costs rather than leaving the
/// player to do that arithmetic: the number is on screen before `REBUILD` is
/// pressed, which is the only moment it can still be reconsidered.
///
/// The floor is `SHELL`-driven: below about 128 the shell, the lamps and a
/// compartment span stop fitting alongside each other.
const MIN_BOX: i32 = 128;
const MAX_BOX: i32 = 4096;

/// A float row.
fn float(group: Group, knob: Knob, category: &str, name: &str, value: f32, s: Span, note: &str) -> Param {
    Param {
        group,
        knob,
        tunable: Tunable::float(TunableGroup::Lab, category, name, value, s.min, s.max, s.step),
        note: note.to_string(),
        shown: None,
    }
}

/// An integer row — the `.ron` field behind it is a `u8`/`u16`/`u32`/`i32`,
/// and `Tunable::integral` is what keeps a decimal point out of the file.
fn integer(group: Group, knob: Knob, category: &str, name: &str, value: f32, s: Span, note: &str) -> Param {
    Param {
        group,
        knob,
        tunable: Tunable::integer(TunableGroup::Lab, category, name, value, s.min, s.max, s.step),
        note: note.to_string(),
        shown: None,
    }
}

/// **An on/off row.** Stored as a 0-or-1 integer so the panel's existing
/// `-`/`+` pair moves it with no new control, and *shown* as `ON`/`OFF`
/// because `1.000` is not a word a player is looking for.
fn toggle(group: Group, knob: Knob, category: &str, name: &str, on: bool, note: &str) -> Param {
    let v = if on { 1.0 } else { 0.0 };
    Param {
        group,
        knob,
        tunable: Tunable::integer(TunableGroup::Lab, category, name, v, 0.0, 1.0, 1.0),
        note: note.to_string(),
        shown: Some(if on { "ON".to_string() } else { "OFF".to_string() }),
    }
}

/// A row that shows a value it cannot change.
fn read_only(group: Group, category: &str, name: &str, shown: String, note: &str) -> Param {
    Param {
        group,
        knob: Knob::ReadOnly,
        tunable: Tunable::float(TunableGroup::Lab, category, name, 0.0, 0.0, 0.0, 0.0),
        note: note.to_string(),
        shown: Some(shown),
    }
}

/// The species the `COLONY` tool releases. Named rather than discovered,
/// because `creature::found_colony` names it too — a panel that tuned a
/// different ant from the one the button places would be a knob that reads
/// correctly and reaches nothing.
pub const COLONY_SPECIES: &str = "ant";

/// **Every parameter the lab exposes, in page and row order.**
///
/// `plant` is the species the bar's chip has armed, so the plant page follows
/// the tool: the chip picks what you are about to put in the ground and this
/// page is that plant's numbers. `None` — an asset set with no plantable
/// species — leaves the page empty rather than guessing at one.
///
/// Rebuilt fresh on every draw, like `App::tunables_list`: a few dozen entries
/// off registries that are already in memory, against the alternative of a
/// retained list to keep in sync with a species reload.
pub fn registry(world: &World, spec: &LabBox, plant: Option<SpeciesId>) -> Vec<Param> {
    let mut out = Vec::new();
    ground(world, &mut out);
    // **First on the page, and outside the `if`.** Outside because it is not
    // the armed species' number -- it is a rule of the box that reaches every
    // plant in it, and a page that hid it when an asset set happens to have no
    // plantable species would hide the one control that says why the last
    // stand fell over. First because the plant page is sixteen rows against a
    // thirteen-row screen, so anything at the end of it is behind a press of
    // the pager, and this was rendered there before it was moved.
    plant_mechanics_rows(world, &mut out);
    if let Some(id) = plant {
        let name = world.species.get(id).name.clone();
        plant_rows(world, &name, &mut out);
    }
    ant_rows(world, &mut out);
    box_rows(world, spec, &mut out);
    out
}

fn material_value(world: &World, name: &str, field: &str) -> Option<f32> {
    let id = world.materials.id_of(name)?;
    let m = world.materials.get(id);
    Some(match field {
        "friction_angle" => m.friction_angle,
        "penetration_resistance" => m.penetration_resistance,
        "water_capacity" => m.water_capacity as f32,
        "density" => m.density,
        "flow_rate" => m.flow_rate as f32,
        "min_transfer" => m.min_transfer as f32,
        "glow" => m.glow,
        "food_energy" => m.food_energy,
        _ => return None,
    })
}

fn ground(world: &World, out: &mut Vec<Param>) {
    let mut mat = |group, material: &'static str, field: &'static str, s: Span, integral, note: &str| {
        let Some(value) = material_value(world, material, field) else { return };
        let knob = Knob::Material { material, field };
        out.push(if integral {
            integer(group, knob, material, field, value, s, note)
        } else {
            float(group, knob, material, field, value, s, note)
        });
    };
    mat(Group::Ground, "soil", "penetration_resistance", span(0.0, 3.0, 0.05), false,
        "HOW HARD THIS IS TO DIG. AN ANT GETS THROUGH A CELL ONLY IF ITS OWN DIG FORCE (ANTS PAGE) IS AT LEAST THIS. RAISE IT ABOVE THE ANT'S DIG FORCE AND THE COLONY CANNOT TUNNEL AT ALL; DROP IT AND EVERY ANT BECOMES A MINER.");
    mat(Group::Ground, "soil", "friction_angle", span(0.0, 89.0, 1.0), false,
        "THE ANGLE A PILE OF THIS HOLDS BEFORE IT SLUMPS, IN DEGREES. IT SHAPES SPOIL HEAPS AND BANK FACES. IT CANNOT HOLD A TUNNEL OPEN -- REPOSE ONLY EVER MAKES A PILE FLATTER, WHICH IS WHAT PACKED SOIL BELOW EXISTS FOR.");
    mat(Group::Ground, "soil", "water_capacity", span(0.0, 4000.0, 100.0), true,
        "HOW MUCH WATER ONE CELL OF SOIL CAN HOLD. HIGH SOIL DRINKS A FLOOD AND HOLDS IT FOR ROOTS; LOW SOIL LETS THE WATER RUN STRAIGHT THROUGH AND POOL ON THE FLOOR.");
    mat(Group::Ground, "soil", "density", span(0.1, 5.0, 0.1), false,
        "HOW HEAVY A CELL OF SOIL IS. IT DECIDES WHAT SINKS THROUGH WHAT: SOIL IS HEAVIER THAN WATER, WHICH IS WHY A SPADEFUL DROPPED IN A POOL GOES TO THE BOTTOM.");
    mat(Group::Ground, "packedsoil", "penetration_resistance", span(0.0, 3.0, 0.05), false,
        "THE LINING AN ANT LAYS AS IT DIGS. THIS IS WHAT KEEPS A TUNNEL FROM FILLING IN BEHIND IT. RAISE IT ABOVE THE ANT'S DIG FORCE AND A COLONY CANNOT RE-DIG ITS OWN GALLERIES.");
    mat(Group::Ground, "packedsoil", "friction_angle", span(0.0, 89.0, 1.0), false,
        "THE ANGLE PACKED SOIL HOLDS. IT IS AUTHORED STEEPER THAN LOOSE SOIL BECAUSE A WORKED WALL IS A WORKED WALL.");
    mat(Group::Ground, "water", "flow_rate", span(1.0, 1000.0, 25.0), true,
        "HOW MUCH OF A CELL'S FILL MAY MOVE TO A NEIGHBOUR IN ONE TICK, OUT OF 1000. LOW MAKES WATER CRAWL AND POOLS TAKE MINUTES TO LEVEL; HIGH MAKES IT RUN LIKE A FLASH FLOOD.");
    mat(Group::Ground, "water", "min_transfer", span(1.0, 400.0, 4.0), true,
        "THE SMALLEST TRANSFER WORTH DOING, OUT OF 1000. IT IS WHERE LEVELLING GIVES UP: HIGHER SETTLES A POOL SOONER AND LEAVES IT MORE VISIBLY TILTED, LOWER KEEPS SHUFFLING FILL LONG AFTER THE SURFACE LOOKS FLAT.");
    mat(Group::Ground, "water", "density", span(0.1, 5.0, 0.1), false,
        "HOW HEAVY WATER IS AGAINST EVERYTHING ELSE. IT IS THE REASON SOIL SINKS THROUGH A POOL AND OIL DOES NOT.");
}

/// Read one scalar off the shoot's `Grow` arm.
///
/// `GrowingTip` and nothing else: the same names are on `RootTip` with
/// deliberately different values, and a reader that took whichever came first
/// would report the root's while the writer moved the shoot's.
fn grow_value(world: &World, species: &str, field: &str) -> Option<f32> {
    let id = world.species.id_of(species)?;
    world.species.get(id).behaviors(CellType::GrowingTip).iter().find_map(|b| match b {
        Behavior::Grow { cost, continuation_weight, wind_weight, crowding_weight, max_active_tips, leaf_spread, .. } => Some(match field {
            "cost" => *cost,
            "continuation_weight" => *continuation_weight,
            "wind_weight" => *wind_weight,
            "crowding_weight" => *crowding_weight,
            "max_active_tips" => *max_active_tips as f32,
            "leaf_spread" => *leaf_spread,
            _ => return None,
        }),
        _ => None,
    })
}

fn grow_by_order(world: &World, species: &str, field: &str) -> Option<String> {
    let id = world.species.id_of(species)?;
    world.species.get(id).behaviors(CellType::GrowingTip).iter().find_map(|b| match b {
        Behavior::Grow { branch_chance, light_weight, upward_weight, .. } => {
            let pick = match field {
                "branch_chance" => branch_chance,
                "light_weight" => light_weight,
                "upward_weight" => upward_weight,
                _ => return None,
            };
            Some((0..organism::BRANCH_ORDERS).map(|o| format!("{:.2}", pick.at(o as u8))).collect::<Vec<_>>().join("/"))
        }
        _ => None,
    })
}

fn repro_value(world: &World, species: &str, field: &str) -> Option<f32> {
    let id = world.species.id_of(species)?;
    world.species.get(id).behaviors(CellType::MatureBody).iter().chain(world.species.get(id).behaviors(CellType::GrowingTip)).find_map(|b| match b {
        Behavior::Reproduce { seed_cost, reproductive_allocation, seed_maturity, seed_launch } => Some(match field {
            "seed_cost" => *seed_cost,
            "reproductive_allocation" => *reproductive_allocation,
            "seed_maturity" => *seed_maturity as f32,
            "seed_launch" => *seed_launch,
            _ => return None,
        }),
        _ => None,
    })
}

fn plant_rows(world: &World, species: &str, out: &mut Vec<Param>) {
    let g = Group::Plant;
    let sp = species.to_string();
    let mut grow = |field: &'static str, s: Span, integral, note: &str| {
        let Some(value) = grow_value(world, species, field) else { return };
        let knob = Knob::Grow { species: sp.clone(), field };
        out.push(if integral {
            integer(g, knob, species, field, value, s, note)
        } else {
            float(g, knob, species, field, value, s, note)
        });
    };
    grow("crowding_weight", span(0.0, 80.0, 1.0), false,
        "HOW STRONGLY A SHOOT TIP AVOIDS ITS OWN CROWD. HIGH MAKES A PLANT SPREAD AND OPEN OUT; ZERO LETS IT PILE INTO ITSELF. THIS IS THE SHOOT'S -- THE ROOT CARRIES ITS OWN, AUTHORED SEPARATELY AND NOT SHOWN HERE.");
    grow("cost", span(0.0, 2.0, 0.02), false,
        "WHAT ONE NEW SHOOT CELL COSTS IN CARBON. IT IS THE PRICE OF GROWING: RAISE IT AND A PLANT SPENDS ITS INCOME ON FEWER CELLS, WHICH SHOWS UP AS A SMALLER PLANT LONG BEFORE IT SHOWS UP AS A DEAD ONE.");
    grow("continuation_weight", span(0.0, 4.0, 0.05), false,
        "HOW MUCH A TIP PREFERS TO KEEP GOING THE WAY IT WAS ALREADY HEADED. HIGH GIVES STRAIGHT RUNS, LOW GIVES A WANDERING, TANGLED HABIT.");
    grow("wind_weight", span(0.0, 4.0, 0.05), false,
        "HOW MUCH THE AIR IN THE BOX PUSHES A GROWING TIP AROUND. IN A SEALED BED THERE IS LITTLE WIND, SO THIS DOES LESS HERE THAN IT DOES OUTDOORS.");
    grow("max_active_tips", span(1.0, 64.0, 1.0), true,
        "HOW MANY SHOOT TIPS MAY BE GROWING AT ONCE. IT IS THE CEILING ON HOW BUSHY A PLANT CAN GET, AND IT IS ALSO A FRAME-COST KNOB -- EVERY ACTIVE TIP IS WORK EVERY TICK.");
    grow("leaf_spread", span(0.0, 1.0, 0.05), false,
        "THE SHAPE OF A CLUMP OF LEAVES, NOT HOW MANY THERE ARE. EACH NODE GETS A FIXED NUMBER OF LEAF CELLS AND THEN GROWS THEM OUTWARD ONE AT A TIME; AT 0 -- WHERE EVERY SPECIES SHIPS -- EACH STEP GOES SOMEWHERE RANDOM, SO A CLUMP IS A BLOB WHOSE SHAPE IS PURE CHANCE. TURN IT UP AND THE CLUMP REACHES AWAY FROM THE STEM IN A LINE INSTEAD, WHICH MAKES FOLIAGE READ AS SPIKY RATHER THAN BUSHY. IT MOVES NO EXTRA LEAF -- THE SAME CELLS GO DOWN IN A DIFFERENT ARRANGEMENT. LASTS THE SESSION.");

    let mut repro = |field: &'static str, s: Span, integral, note: &str| {
        let Some(value) = repro_value(world, species, field) else { return };
        let knob = Knob::Reproduce { species: sp.clone(), field };
        out.push(if integral {
            integer(g, knob, species, field, value, s, note)
        } else {
            float(g, knob, species, field, value, s, note)
        });
    };
    repro("seed_maturity", span(0.0, 4000.0, 10.0), true,
        "HOW MUCH SHOOT A PLANT MUST HAVE GROWN BEFORE IT MAY SET A SEED AT ALL. IT IS THE SINGLE BIGGEST LEVER ON WHETHER A GENERATION EVER TURNS OVER: A HERB BREEDS AT 60 CELLS AND A TREE AT 600, AND A STAND THAT NEVER MOVES THE GERMINATED COUNT ON THE PLANTS PAGE IS USUALLY STUCK BEHIND THIS.");
    repro("seed_cost", span(0.0, 2.0, 0.02), false,
        "WHAT ONE SEED COSTS TO MAKE. CHEAP SEEDS MEAN MANY SMALL CHANCES, DEAR ONES MEAN FEW GOOD ONES.");
    repro("reproductive_allocation", span(0.0, 1.0, 0.02), false,
        "WHAT SHARE OF A MATURE PLANT'S INCOME GOES INTO SEED RATHER THAN INTO MORE PLANT. IT IS THE GROW-VERSUS-BREED DIAL.");
    repro("seed_launch", span(0.0, 40.0, 1.0), false,
        "HOW FAR THIS PLANT FLINGS A SEED SIDEWAYS, IN CELLS. AT 0 -- WHERE EVERY SPECIES SHIPS -- A SEED DROPS AT THE PLANT'S FEET AND THE ONLY THING THAT MOVES IT AFTERWARDS IS THE FALL, WHICH IS WORTH ABOUT TWO THIRDS OF A CELL SIDEWAYS: THAT IS WHY A STAND SITS IN CLUMPS UNDER THE PLANTS THAT MADE IT. TURN IT UP AND SEED IS THROWN, THOUGH NOT THROUGH ANYTHING -- IT STOPS AT THE FIRST THING IN THE WAY, SO A PLANT IN A CORNER STILL SOWS A CORNER. IT IS A DISTANCE AND NOT A DIRECTION, SO MOST SEED STILL LANDS NEAR HOME AND A FEW GO A LONG WAY. MEASURED ON HERB OVER THREE BEDS, A REACH OF 12 PUT 38% MORE PLANTS DOWN WELL AWAY FROM WHERE ANYTHING WAS PLANTED. LASTS THE SESSION.");

    if let Some(id) = world.species.id_of(species) {
        let s = world.species.get(id);
        out.push(float(g, Knob::Species { species: sp.clone(), field: "seed_half_life" }, species, "seed_half_life",
            s.seed_half_life, span(0.0, 60_000.0, 500.0),
            "HOW LONG A SEED KEEPS IN THE GROUND BEFORE IT IS GONE, IN TICKS. SIXTY TICKS IS ONE SIMULATED SECOND. A LONG HALF-LIFE BUILDS A SEED BANK THAT OUTLIVES A BAD SPELL; A SHORT ONE MEANS A GENERATION MISSED IS A GENERATION LOST."));
        out.push(float(g, Knob::Species { species: sp.clone(), field: "remains_half_life" }, species, "remains_half_life",
            s.remains_half_life, span(0.0, 60_000.0, 500.0),
            "HOW LONG A DEAD PLANT'S REMAINS STAND BEFORE THEY ROT AWAY, IN TICKS. THIS IS WHAT MAKES A CULL GRADED RATHER THAN A DELETION -- AND WHAT THE ROT FEEDS BACK INTO THE SOIL IS THE NEXT GENERATION'S FOOD."));
    }

    for field in ["light_weight", "branch_chance", "upward_weight"] {
        let Some(shown) = grow_by_order(world, species, field) else { continue };
        out.push(read_only(g, species, field, shown,
            "FOUR NUMBERS, ONE PER BRANCH ORDER -- TRUNK FIRST, FINEST TWIG LAST. SHOWN AND NOT CHANGEABLE FROM HERE: THIS PANEL HAS ONE NUMBER PER ROW, AND WRITING ONE WOULD SET ALL FOUR TIERS THE SAME AND QUIETLY DESTROY THE AUTHORED RAMP. EDIT THE SPECIES FILE AND PRESS F5 IN THE SANDBOX TO CHANGE IT."));
    }
}

/// **Whether a plant may be taken apart by the load it is carrying** — the
/// one row that is a rule rather than a number, and the reason it is on this
/// page rather than on `BOX`.
///
/// Owner request: *"create an option for me to turn off plant/tree collapse
/// due to mechanics/bending stress."* A player watching a tree come down does
/// not know, and should not have to know, that two separate rules can have
/// done it — `plant::break_under_load`'s stress snap and
/// `structural::organism_structural_tick`'s cantilever span — so this is one
/// switch over both. What it deliberately leaves on is detachment: a branch
/// you cut still falls.
///
/// **Its own category header**, so a page of one species' growth numbers does
/// not appear to have grown a row that applies to all of them.
fn plant_mechanics_rows(world: &World, out: &mut Vec<Param>) {
    out.push(toggle(
        Group::Plant,
        Knob::Rule { field: "plant_load_failure" },
        "plant mechanics",
        "collapse_under_load",
        world.plant_load_failure,
        "WHETHER A LIVING PLANT MAY BE PULLED APART BY MECHANICS. ON IS THE SHIPPED BEHAVIOUR: A STEM SNAPS WHERE THE BENDING STRESS BEATS THE WOOD, A LIMB REACHING FURTHER THAN IT CAN HOLD GIVES WAY, AND ONE THAT LOSES ITS FOOTING COMES DOWN WHOLE. OFF HOLDS EVERY LIVING PLANT IN THE BOX TOGETHER HOWEVER FAR IT LEANS AND WHATEVER IS DUG OUT FROM UNDER IT, WHICH IS WHAT YOU WANT WHILE YOU ARE LOOKING AT GROWTH RATHER THAN AT MECHANICS. DEAD WOOD STILL COMES APART EITHER WAY, SO CULLING AND ROT STILL CLEAR THE BOX. IT REACHES EVERY SPECIES, IT IS FELT ON THE NEXT TICK, AND IT LASTS THE SESSION.",
    ));
    out.push(toggle(
        Group::Plant,
        Knob::Rule { field: "plant_bending" },
        "plant mechanics",
        "bend_under_load",
        world.plant_bending,
        "WHETHER A PLANT LEANS. ON, A STEM BOWS UNDER WHAT IT CARRIES AND LIES OVER IN A GUST; OFF, IT STANDS WHERE IT GREW HOWEVER HARD THE WIND BLOWS. SEPARATE FROM COLLAPSE ABOVE BECAUSE THEY ARE DIFFERENT PROMISES -- THAT ONE IS WHETHER A PLANT CAN BE PULLED APART, THIS ONE IS WHETHER IT BENDS AT ALL. TURNING IT OFF IS ALSO THE CHEAPEST THING ON THIS PAGE: WITH BOTH THIS AND COLLAPSE OFF, NOTHING READS THE STRESS FIELD AND THE BOX STOPS BUILDING IT, WHICH IS ABOUT A QUARTER OF THE PER-PLANT WORK. IT REACHES EVERY SPECIES, IT IS FELT ON THE NEXT TICK, AND IT LASTS THE SESSION.",
    ));
    out.push(toggle(
        Group::Plant,
        Knob::Rule { field: "plant_size_cadence" },
        "plant mechanics",
        "big_plants_tick_slower",
        world.plant_size_cadence,
        "WHETHER A BIG PLANT RUNS ON A SLOWER CLOCK THAN A SEEDLING. OFF, EVERY PLANT TICKS AT THE SAME RATE WHATEVER ITS SIZE, WHICH IS THE SHIPPED BEHAVIOUR. ON, A PLANT WAITS LONGER BETWEEN TICKS THE BIGGER IT IS -- A SEEDLING EVERY TICK, A GROWN TREE EVERY FIFTH. THIS IS THE ONE DIAL ON THIS PAGE THAT BUYS REAL SPEED IN A FULL BOX, BECAUSE A HANDFUL OF LARGE TREES IS ALMOST ALL OF THE WORK. IT IS ALSO NOT FREE: THE TICK IS THE PLANT'S ECONOMY, SO A SLOWED TREE DOES NOT MERELY UPDATE LESS, IT LIVES SLOWER WHILE THE SEEDS AROUND IT DO NOT -- WHICH CHANGES WHO WINS. LASTS THE SESSION.",
    ));
    out.push(float(
        Group::Heredity,
        Knob::Heredity { field: "mutation_sigma" },
        "heredity",
        "genotype_drift",
        world.mutation_sigma,
        span(0.0, 0.5, 0.005),
        "HOW FAR ONE OF A PLANT'S TEN CONTINUOUS GENES MAY MOVE IN A SINGLE GENERATION. THIS IS THE MUTAGEN DIAL. AT 0 EVERY SEED IS A CLONE OF ITS PARENT ON THOSE TEN AXES, WHICH IS NOT A DISABLED FEATURE BUT THE CONTROL ARM: IT IS THE NULL A SELECTED RUN HAS TO BEAT. TURNED UP, LINEAGES WANDER FASTER AND SELECTION HAS MORE TO SORT -- AND MORE OF WHAT IT SORTS IS NOISE. IT REACHES EVERY PLANT IN THE BOX, IT IS FELT AT THE NEXT SEED RATHER THAN ON THE NEXT TICK, AND IT LASTS THE SESSION.",
    ));
    out.push(float(
        Group::Heredity,
        Knob::Heredity { field: "fate_mutation_chance" },
        "heredity",
        "fate_drift",
        world.fate_mutation_chance,
        span(0.0, 1.0, 0.01),
        "THE CHANCE A SEED IS BORN WITH ONE OF ITS PARENT'S FATE RULES CHANGED -- WHAT A CELL TURNS INTO WHEN ITS TIME COMES, WHICH IS THE PART OF A PLANT'S GENOME THAT DECIDES ITS SHAPE RATHER THAN ITS SIZE. THE COARSER OF THE THREE DIALS ON THIS PAGE: A CHANGED FATE IS A DIFFERENT ARCHITECTURE, WHERE THE DRIFT ABOVE IS THE SAME PLANT NUDGED. LASTS THE SESSION.",
    ));
    // **Shipped at 0, and the row is how it gets turned on.** See
    // `plant::PARAM_MUTATION_CHANCE`: the mechanism is complete and what is
    // unmeasured is the rate this world wants, so the honest place for it is
    // a dial the owner can move rather than a constant a session guessed.
    out.push(float(
        Group::Heredity,
        Knob::Heredity { field: "param_mutation_chance" },
        "heredity",
        "species_drift",
        world.param_mutation_chance,
        span(0.0, 1.0, 0.01),
        "THE CHANCE A SEED IS BORN HAVING LEFT ONE OF ITS SPECIES' OWN NUMBERS BEHIND. THE OTHER TWO DIALS MOVE A PLANT INSIDE THE BOX ITS SPECIES FILE DRAWS -- EVERY GENE THERE IS A MULTIPLIER ON AN AUTHORED VALUE, SO A SPECIES THAT SAYS ZERO STAYS AT ZERO FOR EVER, WHICH IS WHY NO TREE, CONIFER OR SHRUB CAN EVOLVE A BRANCHING ROOT SYSTEM AND NO HERB A BRANCHING SHOOT. THIS ONE REPLACES THE NUMBER INSTEAD OF SCALING IT, SO A LINEAGE CAN LEAVE THE BOX ALTOGETHER: NODES UNDERGROUND, A DIFFERENT LEAF SIZE, A CHEAPER SEED. SHIPPED AT 0 BECAUSE WHAT IT COSTS HAS NOT BEEN MEASURED OVER A LONG RUN -- SEVERAL OF THESE NUMBERS HAVE A BENEFIT AND NO PRICE, AND A FREE LEVER MADE HERITABLE MAKES EVERY PLANT THE SAME RATHER THAN DIFFERENT. LASTS THE SESSION.",
    ));
    out.push(float(
        Group::Heredity,
        Knob::Heredity { field: "param_mutation_sigma" },
        "heredity",
        "species_drift_step",
        world.param_mutation_sigma,
        span(0.0, 1.0, 0.01),
        "HOW FAR ONE OF THOSE NUMBERS MOVES WHEN IT MOVES, AS A FRACTION OF WHAT THAT NUMBER IS WORTH ACROSS EVERY SPECIES IN THE BOX. SMALL VALUES ARE A LINEAGE EDGING AWAY FROM ITS SPECIES; LARGE ONES ARE A LINEAGE THAT ARRIVES SOMEWHERE ELSE IN ONE GENERATION AND USUALLY DIES THERE. LASTS THE SESSION.",
    ));
    // **Shipped OFF, and this row is how it gets turned on.** See
    // `organism::DevelopmentalKey`: the mechanism is measured and which end
    // of it the box should live at is an eye question, so it belongs on a
    // dial the owner can move rather than in a constant a session chose.
    out.push(float(
        Group::Heredity,
        Knob::Heredity { field: "developmental_key" },
        "heredity",
        "shared_development",
        match world.developmental_key {
            crate::sim::organism::DevelopmentalKey::World => 0.0,
            crate::sim::organism::DevelopmentalKey::Plant { coarseness } => coarseness as f32 + 1.0,
        },
        span(0.0, 8.0, 1.0),
        "WHETHER TWO PLANTS OF THE SAME GENOME GROW THE SAME SHAPE. 0 IS THE SHIPPED BEHAVIOUR AND IT IS THE PROBLEM THIS DIAL EXISTS FOR: EVERY CELL OF EVERY PLANT ROLLS ITS OWN DICE OFF ITS POSITION IN THE WORLD, SO A PLANT ONE COLUMN OVER IS A DIFFERENT PLANT -- TWELVE IDENTICAL SEEDS IN IDENTICAL BEDS COME OUT BETWEEN 83 AND 181 CELLS AND BETWEEN 27 AND 63 ROWS TALL. THAT SPREAD IS BIGGER THAN ANYTHING THE GENES DO, WHICH MEANS SELECTION CANNOT SEE SHAPE AT ALL. 1 GIVES EACH LINE ONE INHERITED FORM: BROTHERS AND SISTERS GROW ALIKE AND WHAT IS LEFT BETWEEN THEM IS WHAT THE LIGHT AND THE WATER AND THE NEIGHBOURS DID, WHICH IS THE VARIATION WORTH KEEPING. 2 GIVES EVERY PLANT ITS OWN FORM BUT MAKES IT WHOLE INSTEAD OF A MOSAIC -- STILL ALL DIFFERENT, EACH ONE COHERENT. 3 AND UP MAKE PATCHES: PLANTS WITHIN THAT MANY COLUMNS GROW ALIKE. MOVING IT REACHES EVERY PLANT IN THE BOX AT ONCE, INCLUDING ONES ALREADY STANDING -- BUT A PLANT THAT HAS FINISHED GROWING WILL NOT RESHAPE ITSELF, SO GIVE IT A GENERATION BEFORE DECIDING IT DID NOTHING. LASTS THE SESSION.",
    ));
}

fn creature_value(world: &World, species: &str, field: &str) -> Option<f32> {
    let id = world.species.id_of(species)?;
    let def = world.species.get(id).creature.as_ref()?;
    Some(match field {
        "dig_force" => def.dig_force,
        "digest_rate" => def.digest_rate,
        "crop_capacity" => def.crop_capacity,
        "body_energy" => def.body_energy,
        "start_energy" => def.start_energy,
        "reproduce_threshold" => def.reproduce_threshold,
        "mutation_rate" => def.mutation_rate,
        "tick_interval" => def.tick_interval as f32,
        "idle_cost_per_cell" => def.idle_cost_per_cell,
        "move_cost_per_cell" => def.move_cost_per_cell,
        "dig_cost_in_moves" => def.dig_cost_in_moves,
        "emit_cost_in_moves" => def.emit_cost_in_moves,
        "spoil_weight_cells" => def.spoil_weight_cells,
        "curvature_fraction" => def.curvature_fraction,
        "exposure_cost_per_cell" => def.exposure_cost_per_cell,
        _ => return None,
    })
}

fn ant_rows(world: &World, out: &mut Vec<Param>) {
    let g = Group::Ants;
    let species = COLONY_SPECIES;
    let sp = species.to_string();
    let mut cr = |field: &'static str, s: Span, integral, note: &str| {
        let Some(value) = creature_value(world, species, field) else { return };
        let knob = Knob::Creature { species: sp.clone(), field };
        out.push(if integral {
            integer(g, knob, species, field, value, s, note)
        } else {
            float(g, knob, species, field, value, s, note)
        });
    };
    cr("dig_force", span(0.0, 3.0, 0.05), false,
        "HOW HARD AN ANT CAN DIG. IT IS COMPARED STRAIGHT AGAINST A MATERIAL'S PENETRATION RESISTANCE ON THE GROUND PAGE -- SOIL IS 0.80 AND PACKED SOIL IS 0.95, SO AN ANT BELOW 0.80 CANNOT TUNNEL AT ALL AND ONE BETWEEN THE TWO CAN DIG FRESH GROUND AND NOT RE-OPEN ITS OWN LINED GALLERIES.");
    cr("crop_capacity", span(0.0, 4000.0, 40.0), false,
        "HOW MUCH AN ANT CAN CARRY AT ONCE, IN THE SAME UNITS AS A LEAF'S WORTH ON THE GROUND PAGE. IT MUST HOLD AT LEAST TWO OR THREE WHOLE MOUTHFULS: FOOD ONLY LEAVES THE CROP A WHOLE CELL AT A TIME, SO AN ANT THAT CAN HOLD EXACTLY ONE LEAF DIGESTS BELOW A LEAF IMMEDIATELY AND CAN NEVER PUT ANYTHING DOWN AGAIN.");
    cr("digest_rate", span(0.0, 40.0, 0.25), false,
        "HOW FAST AN ANT TURNS WHAT IT IS CARRYING INTO ITSELF, PER STEP. THIS IS WHAT DECIDES WHETHER FOOD REACHES THE NEST: AN ANT DIGESTS AS IT WALKS, SO A HIGH RATE FEEDS THE ANT AND A LOW ONE FEEDS THE COLONY. ZERO MEANS IT NEVER DIGESTS WHAT IT CARRIES AND WILL STARVE WITH A FULL MOUTH.");
    cr("start_energy", span(0.0, 3000.0, 25.0), false,
        "WHAT AN ANT IS BORN WITH. IT IS THE WHOLE OF ITS RUNWAY: DIVIDE IT BY THE IDLE COST BELOW AND YOU HAVE HOW MANY TICKS IT LIVES DOING NOTHING.");
    cr("body_energy", span(0.0, 500.0, 5.0), false,
        "WHAT AN ANT'S BODY IS WORTH TO WHATEVER EATS IT. IT IS ALSO WHAT A CORPSE PUTS BACK INTO THE BOX, SO IT IS THE RETURN LEG OF THE ENERGY LEDGER.");
    cr("reproduce_threshold", span(0.0, 4000.0, 25.0), false,
        "HOW MUCH ENERGY AN ANT MUST HAVE BANKED BEFORE IT WILL BREED. ZERO TURNS BREEDING OFF ENTIRELY. THIS AND START ENERGY TOGETHER DECIDE WHETHER A COLONY EVER REACHES A SECOND GENERATION, WHICH THE ANTS PAGE'S TREND STRIP IS THE READOUT FOR.");
    cr("mutation_rate", span(0.0, 0.5, 0.005), false,
        "HOW MUCH A NEWBORN'S BRAIN DIFFERS FROM ITS PARENT'S. ZERO IS CLONING AND EVOLUTION CANNOT HAPPEN; HIGH IS A COLONY THAT NEVER KEEPS WHAT WORKED.");
    cr("tick_interval", span(1.0, 60.0, 1.0), true,
        "HOW MANY WORLD TICKS BETWEEN ONE ANT'S TURNS. IT IS HOW FAST THE ANIMAL LIVES -- AND IT IS A FRAME-COST KNOB IN THE OTHER DIRECTION, BECAUSE A LOWER NUMBER IS MORE THINKING PER SECOND FOR EVERY ANT IN THE BOX.");
    cr("idle_cost_per_cell", span(0.0, 2.0, 0.01), false,
        "WHAT IT COSTS AN ANT TO SIMPLY EXIST, PER CELL OF BODY, PER TURN. IT IS THE CLOCK ON EVERY ANIMAL IN THE BOX.");
    cr("move_cost_per_cell", span(0.0, 4.0, 0.02), false,
        "WHAT IT COSTS TO MOVE, PER CELL OF BODY. AGAINST THE IDLE COST IT IS THE PRICE OF LOOKING FOR FOOD VERSUS THE PRICE OF WAITING FOR IT.");
    cr("dig_cost_in_moves", span(0.0, 40.0, 0.5), false,
        "WHAT DIGGING ONE CELL COSTS, COUNTED IN STEPS -- AT 3 IT COSTS AN ANT THE SAME AS WALKING THREE CELLS. IT SHIPS AT ZERO, WHICH MEANS EXCAVATION IS FREE AND THE COLONY WILL DIG WHATEVER IT DIGS WITHOUT EVER PAYING FOR IT. THAT IS WHY A BED WITH ANTS IN IT ENDS UP AS ONE ENORMOUS HOLE: THE ANTS ARE BORN WANTING TO DIG AND NOTHING IN THE WORLD CAN TALK THEM OUT OF IT. TURN IT UP AND DIGGING BECOMES A CHOICE THEY CAN GET WRONG.");
    cr("emit_cost_in_moves", span(0.0, 40.0, 0.5), false,
        "WHAT LAYING A FULL-STRENGTH SCENT TRAIL COSTS, COUNTED IN STEPS. ALSO ZERO ON ARRIVAL, AND FOR THE SAME REASON IT MATTERS: A TRAIL THAT COSTS NOTHING IS ALWAYS WORTH LAYING, SO NOTHING SEPARATES AN ANT THAT MARKS A ROUTE FROM ONE THAT MARKS EVERYWHERE IT GOES.");
    cr("spoil_weight_cells", span(0.0, 8.0, 0.1), false,
        "WHAT A LUMP OF DUG EARTH WEIGHS WHILE AN ANT CARRIES IT, IN CELLS OF ITS OWN BODY. CARRYING FOOD HAS ALWAYS COST SOMETHING; CARRYING SPOIL HAS NOT, SO AN ANT COULD HAUL DIRT ONE HUNDRED AND SIXTY CELLS FOR FREE. AT 1 A PELLET IS AS HEAVY AS HALF THE ANT.");
    cr("curvature_fraction", span(0.0, 0.000001, 0.00000001), false,
        "WHAT IT COSTS AN ANT TO FEEL THE SHAPE OF THE GROUND UNDER IT, PER CELL OF GROUND IT FEELS, PER TURN. THE SENSE READS A SQUARE PATCH AROUND THE ANT, SO WIDENING IT COSTS FOUR TIMES AS MUCH FOR TWICE THE REACH -- WHICH IS WHY NO CAP IS NEEDED TO KEEP IT HONEST. IT IS SET TO THE SAME PRICE PER CELL AS EYESIGHT, BECAUSE LOOKING AT A CELL COSTS WHAT IT COSTS WHICHEVER SENSE DOES THE LOOKING; THE ANT'S PATCH IS SMALL, SO IT COMES TO A FORTIETH OF WHAT A FULL SWEEP OF EYESIGHT WOULD.");
    cr("exposure_cost_per_cell", span(0.0, 1.0, 0.01), false,
        "WHAT IT COSTS TO STAND IN THE OPEN, PER CELL OF BODY, PER TURN -- ON TOP OF THE IDLE COST ABOVE. AN ANT IS SHELTERED WHEN THERE IS GROUND OVER ITS HEAD, WHICH IS THE SAME TEST THE ANTS THEMSELVES USE FOR THE INSIDE OF A BURROW. IT SHIPS AT ZERO, AND AT ZERO A ROOFED CELL IS WORTH EXACTLY AS MUCH AS AN OPEN ONE -- WHICH IS WHY DIGGING A NEST HAS NEVER PAID. TURN IT UP AND BEING CAUGHT OUTSIDE COSTS SOMETHING. BE WARNED THAT ON ITS OWN IT IS MOSTLY A FLAT TAX ON BEING ALIVE: MEASURED, ANTS ARE IN THE OPEN TWO TICKS IN THREE AND A PRICE WORTH A FIFTH OF EVERYTHING THEY BURN STILL DID NOT MAKE DIGGING WORTH IT.");

    if let Some(id) = world.species.id_of(species) {
        if let Some(def) = world.species.get(id).creature.as_ref() {
            for (slot, name, note) in TRAIT_ROWS {
                out.push(float(g, Knob::CreatureTrait { species: sp.clone(), slot: *slot }, species, name,
                    def.traits[*slot], span(-1.0, 1.0, 0.05), note));
            }
        }
    }
}

/// **Every heritable trait slot, as a table rather than as a call each.**
///
/// Two of the four slots that existed before `pace` were unreachable from
/// this page -- `reproduce_at` and `sight_range` both landed with a
/// hand-written `out.push` for the two older slots beside them and no line
/// of their own, so the owner could not set them and could not see them.
/// That is not an oversight anybody repeats on purpose; it is what a
/// registration written one slot at a time does. Indexing
/// `CREATURE_TRAITS`' constants from one table means a new slot is a row
/// here or a compile error there, and `every_trait_slot_has_a_row` asserts
/// the count.
const TRAIT_ROWS: &[(usize, &str, &str)] = &[
    (organism::TRAIT_GUT_BIAS, "gut_bias",
        "WHERE THIS LINEAGE'S DIGESTION SITS BETWEEN PLANT MATTER (-1) AND FLESH (+1). IT IS HERITABLE, SO THIS ROW IS THE ANCESTRAL VALUE A NEWBORN STARTS FROM AND NOT WHAT ANY ANT ALIVE HAS -- CLICK ONE WITH THE LOOK TOOL TO SEE ITS OWN."),
    (organism::TRAIT_BIRTH_GRANT, "birth_grant",
        "HOW MUCH OF START ENERGY A NEWBORN IS ACTUALLY HANDED. HERITABLE, LIKE GUT BIAS, SO THIS IS THE ANCESTRAL VALUE. IT IS THE PARENT'S INVESTMENT PER OFFSPRING."),
    (organism::TRAIT_REPRODUCE_AT, "reproduce_at",
        "HOW RICH AN ANT WAITS TO BE BEFORE IT BREEDS, AS A MULTIPLIER ON THE REPRODUCE THRESHOLD ROW ABOVE: -1 IS THE EARLIEST THE ARITHMETIC ALLOWS AND +1 IS TWICE THE BAR. LOW IS VERY NEARLY A SUICIDE PACT -- A PARENT THAT BREEDS THE INSTANT IT CAN AFFORD TO IS LEFT STANDING ON ONE JOULE. HIGH IS FEWER CHILDREN AND A LONGER LIFE TO HAVE THEM IN."),
    (organism::TRAIT_SIGHT_RANGE, "sight_range",
        "HOW FAR THIS LINEAGE CAN SEE, SHIFTED FROM WHAT THE SPECIES WAS AUTHORED WITH: 0 IS THE AUTHORED EYE, +1 ADDS SIXTY-FOUR CELLS OF REACH AND -1 TAKES IT AWAY. IT IS ADDITIVE, SO A SPECIES BORN BLIND -- WHICH IS EVERY ONE OF THEM BUT THE BEETLE -- CAN EVOLVE EYES. SEEING IS CHARGED PER CELL LOOKED AT, SO A BIGGER EYE IS A REAL BILL AND NOT A FREE UPGRADE."),
    (organism::TRAIT_PACE, "pace",
        "HOW FAST THIS LINEAGE LIVES: +1 IS AN ANT THAT TAKES ITS TURN TWICE AS OFTEN AND -1 ONE THAT TAKES IT HALF AS OFTEN. IT MOVES THE TICK INTERVAL ROW ABOVE, AND IT IS THE ONE ROW ON THIS PAGE YOU CAN WATCH WITHOUT AN OVERLAY -- A QUICK ANT SCURRIES AND A SLOW ONE PLODS. IT IS NOT A FREE SPEED-UP: EVERY COST AN ANT PAYS IS CHARGED ONCE PER TURN, SO LIVING TWICE AS FAST BURNS TWICE AS FAST."),
];

fn box_rows(world: &World, spec: &LabBox, out: &mut Vec<Param>) {
    let g = Group::Box;
    if let Some(value) = material_value(world, "crystal", "glow") {
        out.push(float(g, Knob::Material { material: "crystal", field: "glow" }, "crystal", "glow", value, span(0.0, 4.0, 0.1),
            "HOW BRIGHT THE GROW LAMPS ARE. THE BED IS SEALED, SO THIS IS THE ONLY LIGHT IN IT AND IT IS THE WHOLE OF THE PLANTS' INCOME. 4.0 IS THE ENGINE'S CEILING; AT 0 EVERY PLANT IN THE BOX STARVES. IT IS FELT ON THE NEXT TICK -- TURN IT DOWN AND WATCH THE LIGHT OVERLAY."));
    }
    let mut bed = |field: &'static str, value: f32, s: Span, note: &str| {
        out.push(integer(g, Knob::Bed { field }, "the bed", field, value, s, note));
    };
    // **What the box the player is standing in costs, on the row that
    // changes it.** The note is built rather than a literal so the megabyte
    // figure is this box's and not an example's — `Param::note` is a
    // `String`, so this costs one allocation on a page that is rebuilt only
    // when it is open.
    let megabytes = |w: i32, h: i32| {
        let probe = LabBox { width: w, height: h, ..spec.clone() };
        crate::lab::batch::BatchSpec::world_bytes(&probe) as f32 / (1024.0 * 1024.0)
    };
    let here = megabytes(spec.width, spec.height);
    bed("width", spec.width as f32, span(MIN_BOX as f32, MAX_BOX as f32, 64.0),
        &format!("HOW MANY CELLS WIDE THE BOX IS BUILT. WIDTH ITSELF IS VERY NEARLY FREE -- AN EMPTY BOX MEASURED 0.001 MS PER TICK AT 256, 512 AND 1024 ALIKE, SO THE SLEEPING MACHINERY DOES NOT CARE HOW BIG THE ROOM IS. WHAT WIDTH COSTS IS WHAT GROWS IN IT: AT A FIXED NUMBER OF FOUNDERS A WIDER BOX SPACES THEM FURTHER APART, SO IT GROWS MORE PLANT AND COSTS MORE FRAME -- ABOUT 0.6 MS PER 1000 PLANT CELLS AT ANY WIDTH. IT IS ALSO THE MOST IMPORTANT KNOB FOR WHAT LIVES: ONE ANT COLONY LEAVES 1% OF THE STAND STANDING IN A 256-WIDE BED AND 98% IN A 1024-WIDE ONE, WHILE THE COLONY ITSELF STARVES DOWN TO 2 ANTS AT 1024 BECAUSE IT CANNOT REACH THE PLANTS. THIS BOX HOLDS {here:.1} MB IN MEMORY AND EVERY CHAMBER ON THE RACK HOLDS ITS OWN. TAKES EFFECT ON REBUILD."));
    bed("height", spec.height as f32, span(MIN_BOX as f32, MAX_BOX as f32, 64.0),
        &format!("HOW MANY CELLS TALL THE BOX IS BUILT: SOIL, THE AIR A PLANT STANDS UP IN, AND THE CEILING THE LAMPS HANG FROM. THE SOIL SURFACE MOVES WITH IT -- RAISE THE HEIGHT AND GROUND LEVEL RISES IN PROPORTION, SO THE BED KEEPS ITS SHAPE INSTEAD OF SITTING IN THE TOP QUARTER OF AN EMPTY BOX. SET GROUND LEVEL AFTERWARDS IF YOU WANT A DIFFERENT SPLIT. HEIGHT BUYS HEADROOM RATHER THAN ROOM: PLANTS RARELY CLEAR 200 CELLS AND ROOTS REACH ABOUT 13 ROWS ON THEIR OWN, SO PAST ABOUT 640 YOU ARE MOSTLY PAYING FOR EMPTY AIR. THIS BOX HOLDS {here:.1} MB IN MEMORY AND EVERY CHAMBER ON THE RACK HOLDS ITS OWN. TAKES EFFECT ON REBUILD."));
    bed("lamp_spacing", spec.lamp_spacing as f32, span(8.0, 512.0, 8.0),
        "HOW FAR APART THE LAMPS ARE, IN CELLS. CLOSER IS MORE OF THEM AND A MORE EVENLY LIT BED; THERE IS ALWAYS AT LEAST ONE PER COMPARTMENT, BECAUSE A WALLED-OFF DARK BED IS A SILENT WAY TO KILL A POPULATION. A LAMP'S POOL IS ABOUT 55 COLUMNS, SO PAST THAT THE BED IS LIT IN ISLANDS WITH DARK GROUND BETWEEN. NEARLY FREE -- EIGHT LAMPS COST WHAT ONE COSTS. TAKES EFFECT ON REBUILD.");
    bed("soil_depth", spec.soil_depth as f32, span(8.0, 240.0, 8.0),
        "HOW MANY ROWS OF SOIL THE BED HAS. DEEP SOIL IS ROOM FOR ROOTS AND FOR TUNNELS. THE MOST EXPENSIVE KNOB ON THIS PAGE: 40 ROWS TO 160 MEASURED 1.9X THE FRAME FOR AN IDENTICAL STAND, AND THE COST IS THE SOIL WATER CYCLE IN THE SWEEP RATHER THAN THE LIGHT. ROOTS ONLY REACH ABOUT 13 ROWS ON THEIR OWN -- DEEPER THAN THAT IS FOR THE ANTS TO DIG. TAKES EFFECT ON REBUILD.");
    bed("ground_y", spec.ground_y as f32, span(40.0, 300.0, 10.0),
        "WHICH SCREEN ROW THE SOIL SURFACE SITS AT. LOWER ON THE SCREEN IS A DEEPER BED WITH LESS AIR OVER IT; HIGHER LEAVES MORE ROOM FOR A PLANT TO STAND UP IN. MOVE IT WITH THE BOX HEIGHT, NOT ON ITS OWN: LEFT AT 160 IN A 640-ROW BOX THE SOIL SITS IN THE TOP QUARTER AND 390 ROWS ARE EMPTY VOID, WHICH LOOKS LIKE A BROKEN BED RATHER THAN A TALL ONE. TAKES EFFECT ON REBUILD.");
    bed("compartments", spec.compartments as f32, span(1.0, 8.0, 1.0),
        "HOW MANY SEALED WALLS FLOOR TO CEILING THE BED IS DIVIDED BY. THEY BUY EVOLUTIONARY ISOLATION -- SEPARATE POPULATIONS THAT CANNOT MIX, WHICH IS WHERE DIVERGENCE COMES FROM. THEY DO NOT BUY SPEED AT THIS BED SIZE: THE 7.6X ON RECORD WAS A 2048-WIDE BED WITH A FAN IN IT, AND AT 512 THE FRAME GOES 1.69 -> 1.41 -> 1.92 MS ACROSS 1, 4 AND 16 -- NOT MONOTONE. TAKES EFFECT ON REBUILD.");
    bed("founders", spec.founders as f32, span(0.0, 64.0, 1.0),
        "HOW MANY PLANTS THE BOX IS STOCKED WITH WHEN IT IS BUILT. THE BINARY OPENS AT ZERO ON PURPOSE -- THE BOX STARTS WITH NOTHING AND YOU STOCK IT -- SO RAISE THIS ONLY IF YOU WANT A REBUILD TO HAND YOU A STAND. TAKES EFFECT ON REBUILD.");
    bed("colonies", spec.colonies as f32, span(0.0, 8.0, 1.0),
        "HOW MANY ANT COLONIES A REBUILD RELEASES, ONE PER COMPARTMENT AT MOST. ONE COLONY IS NOT A GARNISH: IT DECIDES WHETHER THE BED LIVES, AND HOW MUCH ROOM THE BED HAS DECIDES WHICH WAY. MEASURED OVER SIX SEEDS, EACH PAIRED AGAINST THE SAME WORLD WITH NO COLONY: IN A 256-WIDE BOX ONE COLONY LEAVES 1% OF THE STAND STANDING AND 6 OF 6 SEEDS FALL, AT 512 IT LEAVES 41%, AND AT 1024 IT LEAVES 98% -- BUT THERE THE COLONY ITSELF DROPS TO 2 ANTS, BECAUSE IT CANNOT REACH THE PLANTS. A COLONY EATS ITS OWN NEIGHBOURHOOD, SO A NARROW BED FEEDS THE ANTS AND KILLS THE PLANTS AND A WIDE ONE DOES THE REVERSE. IF A STOCKED BED SIMPLY EMPTIES, THIS IS THE FIRST THING TO SUSPECT. TAKES EFFECT ON REBUILD.");
    bed("colony_ants", spec.colony_ants as f32, span(1.0, 120.0, 1.0),
        "HOW MANY ANIMALS EACH COLONY IS FOUNDED WITH. FIFTY-TWO BY DEFAULT, AND THAT NUMBER IS NOT ARBITRARY: BELOW ABOUT FIFTY A COLONY LOOKS BROKEN EVEN WHEN THE CODE IS RIGHT, WHICH IS WHY THE KEY PLACES A CROWD RATHER THAN AN ANT. IT WAS ALSO THE ONE STOCKING NUMBER YOU COULD NOT SET -- FOUNDERS SAYS HOW MANY PLANTS AND COLONIES SAYS HOW MANY NESTS, AND HOW BIG A POPULATION STARTS IS THE FIRST THING A SELECTION EXPERIMENT WANTS TO VARY. THE BAND WIDENS WITH THE COUNT, SO A SMALL COLONY IS SPARSE RATHER THAN CLIPPED. TAKES EFFECT ON REBUILD.");
    bed("predators", spec.predators as f32, span(0.0, 8.0, 1.0),
        "HOW MANY BEETLES A REBUILD RELEASES, SPREAD THE SAME WAY THE COLONIES ARE. ZERO BY DEFAULT, BECAUSE A PREDATOR IS THE ONE STOCKING CHOICE THAT CAN EMPTY A BOX. WHAT IT IS FOR IS A PAIR: TWO CHAMBERS ON THE SAME SEED, THIS AT 0 AND AT 4, IS AN EXPERIMENT -- ONE CHAMBER ON ITS OWN IS A STORY. IF A STOCKED BED SIMPLY EMPTIES, RAISE COMPARTMENTS BEFORE BLAMING THE BEETLE: A PREDATOR AND ITS PREY IN ONE SEALED BOX WITH NOWHERE TO HIDE GO EXTINCT IN THEORY AS WELL AS HERE. TAKES EFFECT ON REBUILD.");
    bed("seed", spec.seed as f32, span(0.0, 999.0, 1.0),
        "THE NUMBER THIS BOX IS BUILT FROM. THE SAME SEED AND THE SAME BUILD REBUILD THE SAME BOX EXACTLY, WHICH IS WHAT LETS YOU CHANGE ONE PARAMETER AND COMPARE TWO RUNS RATHER THAN TWO WORLDS. TAKES EFFECT ON REBUILD.");
}

/// **The world-level dials the parameters page exposes that are not a
/// material or species field** — the three [`Knob::Rule`] switches and the
/// five [`Knob::Heredity`] numbers, all plain fields on [`World`] with no
/// asset file of their own. Everything else the page can save round-trips
/// through `assets/materials` or `assets/species`; these had no file at all
/// until this type, which was the larger half of "There is no save"
/// (`Reports/lanes/evolution-lab-coordinator.md`, round three).
///
/// [`Self::from_world`] and [`Self::apply_to`] are the only place that knows
/// this field list, mirroring [`write`]'s own `Knob::Rule` and
/// `Knob::Heredity` arms deliberately — a dial added to the panel and not to
/// both is a reader with no writer on restart, which looks exactly like a
/// working save until the process restarts.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Dials {
    pub plant_load_failure: bool,
    pub plant_bending: bool,
    pub plant_size_cadence: bool,
    pub mutation_sigma: f32,
    pub fate_mutation_chance: f32,
    pub param_mutation_chance: f32,
    pub param_mutation_sigma: f32,
    /// Same encoding [`write`]'s `Knob::Heredity { field: "developmental_key" }`
    /// arm uses: `0` is [`organism::DevelopmentalKey::World`], `n > 0` is
    /// `DevelopmentalKey::Plant { coarseness: n - 1 }`.
    pub developmental_key: u32,
}

impl Dials {
    /// Gitignored, like the specimen shelf (`sim::specimen::SHELF_DIR`) and
    /// `LabBox::ASSET_PATH` beside it — this is which dials a player left a
    /// running box at, not authored content shared by both games.
    pub const ASSET_PATH: &'static str = "assets/lab_dials.ron";
    /// Environment override for [`ASSET_PATH`](Self::ASSET_PATH).
    pub const ASSET_PATH_ENV: &'static str = "PIXEL_PHYSICS_LAB_DIALS";

    fn state_path() -> std::path::PathBuf {
        std::env::var(Self::ASSET_PATH_ENV).map(std::path::PathBuf::from).unwrap_or_else(|_| Self::ASSET_PATH.into())
    }

    /// Read the current value of every dial off a live `World`.
    pub fn from_world(world: &World) -> Self {
        Self {
            plant_load_failure: world.plant_load_failure,
            plant_bending: world.plant_bending,
            plant_size_cadence: world.plant_size_cadence,
            mutation_sigma: world.mutation_sigma,
            fate_mutation_chance: world.fate_mutation_chance,
            param_mutation_chance: world.param_mutation_chance,
            param_mutation_sigma: world.param_mutation_sigma,
            developmental_key: match world.developmental_key {
                organism::DevelopmentalKey::World => 0,
                organism::DevelopmentalKey::Plant { coarseness } => coarseness + 1,
            },
        }
    }

    /// The saved dials, if the parameters page has ever saved any and the
    /// file still parses. `None` — absent or stale alike — means the caller
    /// keeps whatever `World::default()` already set, exactly as before this
    /// file existed.
    pub fn load_saved() -> Option<Self> {
        let text = std::fs::read_to_string(Self::state_path()).ok()?;
        ron::from_str(&text).ok()
    }

    /// Restore every dial onto a live `World` — the inverse of
    /// [`Self::from_world`], and the same field-by-field assignment
    /// [`write`]'s `Knob::Rule`/`Knob::Heredity` arms make, so a restored
    /// session cannot mean something a live edit could not also reach.
    pub fn apply_to(&self, world: &mut World) {
        world.plant_load_failure = self.plant_load_failure;
        world.plant_bending = self.plant_bending;
        world.plant_size_cadence = self.plant_size_cadence;
        world.mutation_sigma = self.mutation_sigma;
        world.fate_mutation_chance = self.fate_mutation_chance;
        world.param_mutation_chance = self.param_mutation_chance;
        world.param_mutation_sigma = self.param_mutation_sigma;
        world.developmental_key = if self.developmental_key == 0 {
            organism::DevelopmentalKey::World
        } else {
            organism::DevelopmentalKey::Plant { coarseness: self.developmental_key - 1 }
        };
        // Every plant already standing has to be re-folded under the
        // restored key, or the box runs two rules at once -- see
        // `Knob::Heredity`'s own write arm, which this mirrors exactly.
        world.refold_developmental_seeds();
    }

    /// Write every dial to [`ASSET_PATH`](Self::ASSET_PATH) whole, like
    /// `player::Tuning::save` — a generated file with no comments to lose.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::state_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let pretty = ron::ser::PrettyConfig::new().struct_names(false);
        let text = ron::ser::to_string_pretty(self, pretty).map_err(|e| e.to_string())?;
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// **Write `value` back to wherever the parameter lives.**
///
/// Every path here is a live store the tick reads through, so a change is felt
/// on the next tick — with the one documented exception of [`Knob::Bed`],
/// which is the spec a rebuild is made from and says so on every row.
///
/// Returns whether anything was written. A `false` is a knob with a reader and
/// no writer, which is the failure `CLAUDE.md` names as looking exactly like
/// working code — `every_writable_parameter_actually_moves` is the guard that
/// keeps this from happening quietly.
pub fn write(world: &mut World, spec: &mut LabBox, knob: &Knob, value: f32) -> bool {
    match knob {
        Knob::ReadOnly => false,
        Knob::Material { material, field } => {
            let Some(id) = world.materials.id_of(material) else { return false };
            let m = world.materials.get_mut(id);
            match *field {
                "friction_angle" => m.friction_angle = value,
                "penetration_resistance" => m.penetration_resistance = value,
                "water_capacity" => m.water_capacity = value.max(0.0).round() as u16,
                "density" => m.density = value,
                "flow_rate" => m.flow_rate = value.max(0.0).round() as u16,
                "min_transfer" => m.min_transfer = value.max(0.0).round() as u16,
                "glow" => m.glow = value,
                "food_energy" => m.food_energy = value,
                _ => return false,
            }
            true
        }
        Knob::Creature { species, field } => {
            let Some(id) = world.species.id_of(species) else { return false };
            // Cloned, edited and put back rather than borrowed mutably:
            // `set_creature` is the accessor that already exists for exactly
            // this, and a `CreatureDef` is a few dozen bytes plus a body plan.
            let Some(mut def) = world.species.get(id).creature.clone() else { return false };
            match *field {
                "dig_force" => def.dig_force = value,
                "digest_rate" => def.digest_rate = value,
                "crop_capacity" => def.crop_capacity = value,
                "body_energy" => def.body_energy = value,
                "start_energy" => def.start_energy = value,
                "reproduce_threshold" => def.reproduce_threshold = value,
                "mutation_rate" => def.mutation_rate = value,
                "tick_interval" => def.tick_interval = value.max(1.0).round() as u64,
                "idle_cost_per_cell" => def.idle_cost_per_cell = value,
                "move_cost_per_cell" => def.move_cost_per_cell = value,
                "dig_cost_in_moves" => def.dig_cost_in_moves = value,
                "emit_cost_in_moves" => def.emit_cost_in_moves = value,
                "spoil_weight_cells" => def.spoil_weight_cells = value,
                "curvature_fraction" => def.curvature_fraction = value,
                "exposure_cost_per_cell" => def.exposure_cost_per_cell = value,
                        _ => return false,
            }
            world.species.set_creature(id, def);
            true
        }
        Knob::CreatureTrait { species, slot } => {
            let Some(id) = world.species.id_of(species) else { return false };
            let Some(mut def) = world.species.get(id).creature.clone() else { return false };
            let Some(t) = def.traits.get_mut(*slot) else { return false };
            *t = value;
            world.species.set_creature(id, def);
            true
        }
        Knob::Grow { species, field } => {
            let Some(id) = world.species.id_of(species) else { return false };
            let mut wrote = false;
            for b in world.species.get_mut(id).behaviors_mut(CellType::GrowingTip) {
                if let Behavior::Grow { cost, continuation_weight, wind_weight, crowding_weight, max_active_tips, leaf_spread, .. } = b {
                    match *field {
                        "cost" => *cost = value,
                        "continuation_weight" => *continuation_weight = value,
                        "wind_weight" => *wind_weight = value,
                        "crowding_weight" => *crowding_weight = value,
                        "max_active_tips" => *max_active_tips = value.max(1.0).round() as u32,
                        "leaf_spread" => *leaf_spread = value.clamp(0.0, 1.0),
                        _ => continue,
                    }
                    wrote = true;
                }
            }
            wrote
        }
        Knob::Reproduce { species, field } => {
            let Some(id) = world.species.id_of(species) else { return false };
            let mut wrote = false;
            let sp = world.species.get_mut(id);
            for ct in [CellType::MatureBody, CellType::GrowingTip] {
                for b in sp.behaviors_mut(ct) {
                    if let Behavior::Reproduce { seed_cost, reproductive_allocation, seed_maturity, seed_launch } = b {
                        match *field {
                            "seed_cost" => *seed_cost = value,
                            "reproductive_allocation" => *reproductive_allocation = value,
                            "seed_maturity" => *seed_maturity = value.max(0.0).round() as u32,
                            "seed_launch" => *seed_launch = value.max(0.0),
                            _ => continue,
                        }
                        wrote = true;
                    }
                }
            }
            wrote
        }
        Knob::Species { species, field } => {
            let Some(id) = world.species.id_of(species) else { return false };
            let sp = world.species.get_mut(id);
            match *field {
                "seed_half_life" => sp.seed_half_life = value,
                "remains_half_life" => sp.remains_half_life = value,
                _ => return false,
            }
            true
        }
        Knob::Heredity { field } => {
            // **Handled ahead of the rate guard**, which bounds the four
            // drift dials to 0..=1. This row is a mode rather than a rate and
            // its span runs to 8, so the shared predicate would refuse every
            // setting above `INHERITED`.
            if *field == "developmental_key" {
                if !(0.0..=8.0).contains(&value) {
                    return false;
                }
                let n = value.round() as u32;
                world.developmental_key = if n == 0 {
                    crate::sim::organism::DevelopmentalKey::World
                } else {
                    crate::sim::organism::DevelopmentalKey::Plant { coarseness: n - 1 }
                };
                // **Every plant already standing is re-folded**, or the box
                // runs two rules at once: `dev_seed` is stamped at
                // germination, so without this a plant that came up before
                // the dial moved would keep the old coarseness and the dial
                // would be lying about what it did. See
                // `World::refold_developmental_seeds`.
                world.refold_developmental_seeds();
                return true;
            }
            if !crate::sim::plant::settable_rate(value) {
                return false;
            }
            match *field {
                "mutation_sigma" => world.mutation_sigma = value,
                "fate_mutation_chance" => world.fate_mutation_chance = value,
                "param_mutation_chance" => world.param_mutation_chance = value,
                "param_mutation_sigma" => world.param_mutation_sigma = value,
                // **Its own arm, because it is the one row here that is not a
                // rate.** `settable_rate` bounds the four above to 0..=1; this
                // is a mode, so it is checked against its own span and mapped
                // before the shared guard would reject anything above 1.
                _ => return false,
            }
            true
        }
        Knob::Rule { field } => {
            let on = value >= 0.5;
            match *field {
                "plant_load_failure" => world.plant_load_failure = on,
                "plant_bending" => world.plant_bending = on,
                "plant_size_cadence" => world.plant_size_cadence = on,
                _ => return false,
            }
            true
        }
        Knob::Bed { field } => write_bed(spec, field, value),
    }
}

/// **Write one field of the bed spec, by name.** The single definition of
/// which fields of a `LabBox` are settable and how.
///
/// Split out of [`write`] because `batch` sweeps these fields too, and it has
/// a spec in hand and no `World` — it is generating the specs a rebuild will
/// be made *from*. Two copies of this table would be two answers to "which
/// knobs can a sweep vary", and the one that drifted would be the one that
/// silently swept nothing.
///
/// Returns whether anything was written, so a caller naming a field that does
/// not exist finds out rather than sweeping a constant — the `include_str!`
/// failure this repo already has on record, where three "runs" came back
/// bit-identical because the knob was never connected.
pub fn write_bed(spec: &mut LabBox, field: &str, value: f32) -> bool {
    let v = value.max(0.0).round();
    match field {
        "width" => spec.width = (v as i32).clamp(MIN_BOX, MAX_BOX),
        // **`ground_y` rides the height, and that is the whole reason this
        // arm is not a one-line assignment.** `lab_resolution` records the
        // trap: left at 160 in a 640-row box the soil sits in the top
        // quarter and 390 rows are empty void, which reads as a broken bed
        // rather than a tall one — a scene error wearing a result, and the
        // owner would meet it the first time they raised this row. Scaling
        // the surface with the box keeps the proportions whatever the
        // height, and `ground_y` stays its own row for anyone who wants to
        // override it afterwards.
        //
        // Ratio rather than a remembered offset, so it is idempotent:
        // writing the same height twice is a multiply by one, which matters
        // because a sweep writes every setting into a fresh clone of the
        // same spec.
        "height" => {
            let (old, new) = (spec.height.max(1), (v as i32).clamp(MIN_BOX, MAX_BOX));
            spec.ground_y = (i64::from(spec.ground_y) * i64::from(new) / i64::from(old)) as i32;
            spec.height = new;
        }
        "lamp_spacing" => spec.lamp_spacing = v as i32,
        "soil_depth" => spec.soil_depth = v as i32,
        "ground_y" => spec.ground_y = v as i32,
        "compartments" => spec.compartments = v as usize,
        "founders" => spec.founders = v as usize,
        "colonies" => spec.colonies = v as usize,
        "colony_ants" => spec.colony_ants = v as i32,
        "predators" => spec.predators = v as usize,
        "seed" => spec.seed = v as u64,
        _ => return false,
    }
    true
}

/// Read one field of the bed spec by the same names [`write_bed`] accepts.
///
/// The pair matters for a sweep: a sweep row has to *print* the setting it
/// ran at, and reading it back through the same table is what makes the
/// printed value the one that was actually applied rather than the one that
/// was asked for.
pub fn read_bed(spec: &LabBox, field: &str) -> Option<f32> {
    Some(match field {
        "width" => spec.width as f32,
        "height" => spec.height as f32,
        "lamp_spacing" => spec.lamp_spacing as f32,
        "soil_depth" => spec.soil_depth as f32,
        "ground_y" => spec.ground_y as f32,
        "compartments" => spec.compartments as f32,
        "founders" => spec.founders as f32,
        "colonies" => spec.colonies as f32,
        "colony_ants" => spec.colony_ants as f32,
        "predators" => spec.predators as f32,
        "seed" => spec.seed as f32,
        _ => return None,
    })
}

/// Whether a change to this knob is only felt after `REBUILD`.
pub fn needs_rebuild(knob: &Knob) -> bool {
    matches!(knob, Knob::Bed { .. })
}

// ------------------------------------------------------------------ saving

/// **Write one parameter back to the asset file it came from, or say why
/// not.**
///
/// A targeted span edit through `tunables::write_field_value` — the sandbox's
/// own path, and emphatically not a `ron::ser` round trip, which would destroy
/// every comment in a file whose comments carry the reasoning. The edited text
/// is parsed back before anything touches disk, so a bad edit is reported
/// rather than written.
///
/// **Two refusals, and both are real rather than defensive padding.**
///
/// - A field that appears more than once in the file is **ambiguous**, and the
///   span edit takes the first match. `tree.ron` holds two `crowding_weight`
///   lines — the shoot's 30.0 and the root's deliberate 0.0 — and `CLAUDE.md`
///   records a whole sweep invalidated by a blind edit that dragged the second
///   along with the first. Saving the shoot's would silently rewrite the
///   root's.
/// - A field whose current text is not a bare number is a **tuple or a list**:
///   `traits: (0.0, -0.2)` and `light_weight: [0.15, 0.3, 0.5, 0.6]` both look
///   like ordinary fields to a span edit, which would replace up to the first
///   comma and leave a dangling `, -0.2)` behind.
///
/// [`Knob::Bed`], [`Knob::Rule`] and [`Knob::Heredity`] have no *per-field*
/// asset file — they live on [`LabBox`]/[`World`] alone — so all three save
/// differently from every other row here: not a span edit of the one field
/// that changed, but the whole of [`LabBox`] or [`Dials`] as it currently
/// stands, via [`LabBox::save`] or [`Dials::save`]. That is deliberate
/// rather than a shortcut — a partial save of "just this field" would still
/// need the rest of the spec/dials to write a valid file, and reading them
/// back off the live `world`/`spec` in hand is simpler than reconstructing
/// them from a lone `Param`. Every other row still saves *itself*, unchanged
/// from before these three could be saved at all.
pub fn save(param: &Param, world: &World, spec: &LabBox) -> Result<String, String> {
    match &param.knob {
        Knob::Bed { .. } => {
            spec.save()?;
            Ok(format!("SAVED {} = {}", param.tunable.name.to_uppercase(), param.tunable.display()))
        }
        Knob::Rule { .. } | Knob::Heredity { .. } => {
            Dials::from_world(world).save()?;
            Ok(format!("SAVED {} = {}", param.tunable.name.to_uppercase(), param.tunable.display()))
        }
        _ => {
            let (path, updated) = planned_edit(param)?;
            std::fs::write(&path, updated).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(format!(
                "SAVED {} {} = {}",
                path.file_name().map_or_else(|| "?".into(), |f| f.to_string_lossy().to_uppercase()),
                param.tunable.name.to_uppercase(),
                param.tunable.display()
            ))
        }
    }
}

/// **What [`save`] would do, without doing it** — the same checks, the same
/// text edit, the same parse, and no write.
///
/// Exists so a harness can report whether every registered row is savable
/// without editing the repository's own asset files, which is a check nobody
/// could run twice. It is the *same code path* rather than a second claim
/// about it, for the text-edited rows: [`save`] is this plus one
/// `fs::write`. [`Knob::Bed`], [`Knob::Rule`] and [`Knob::Heredity`] are
/// reported directly instead — a whole-struct save has nothing a dry run
/// could parse-and-discard the way a span edit does, so there is no shared
/// path to reuse for them.
pub fn save_check(param: &Param) -> String {
    match &param.knob {
        Knob::Bed { .. } => format!("would write {}", LabBox::ASSET_PATH),
        Knob::Rule { .. } | Knob::Heredity { .. } => format!("would write {}", Dials::ASSET_PATH),
        _ => match planned_edit(param) {
            Ok((path, _)) => format!("would write {}", path.display()),
            Err(e) => e,
        },
    }
}

/// The file this parameter would be written to, and the whole new text of it.
///
/// **Text-edited rows only.** [`Knob::Bed`], [`Knob::Rule`] and
/// [`Knob::Heredity`] never reach this function — [`save`] and
/// [`save_check`] both branch on those three before calling it — so the
/// three arms below exist only to keep the match exhaustive, and say where
/// the real path is rather than repeat one of the two callers' branches.
fn planned_edit(param: &Param) -> Result<(std::path::PathBuf, String), String> {
    let (dir, file) = match &param.knob {
        Knob::ReadOnly => return Err("this row cannot be changed, so there is nothing to save".into()),
        Knob::Bed { .. } => return Err("unreachable via save/save_check -- see LabBox::save".into()),
        Knob::Rule { .. } => return Err("unreachable via save/save_check -- see Dials::save".into()),
        Knob::Heredity { .. } => return Err("unreachable via save/save_check -- see Dials::save".into()),
        Knob::Material { material, .. } => (material::ASSET_DIR, material.to_string()),
        Knob::Creature { species, .. }
        | Knob::CreatureTrait { species, .. }
        | Knob::Grow { species, .. }
        | Knob::Reproduce { species, .. }
        | Knob::Species { species, .. } => (organism::ASSET_DIR, species.clone()),
    };
    let field = param.tunable.name.as_str();
    let path = std::path::Path::new(dir).join(format!("{file}.ron"));
    let source = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let n = occurrences(&source, field);
    if n > 1 {
        return Err(format!("'{field}' appears {n} times in {file}.ron -- saving it would edit the wrong one"));
    }
    // **A field absent from a species file is not a field to append.**
    // `write_field_value` inserts a missing key before the file's outermost
    // `)`, which is right for a material -- most material files only write
    // what differs from `Material`'s serde defaults, so absence is the normal
    // case there -- and wrong for a species twice over. A `CreatureDef` field
    // lives nested inside `creature: Some((...))`, so an append would land it
    // at `SpeciesDef` level where nothing reads it; and two rows here are not
    // fields at all, but elements of `traits: (0.0, -0.2)`. Both would have
    // written a file that parses, loads, and quietly ignores the edit --
    // reporting a save that did not happen, which is the one outcome worse
    // than refusing.
    if n == 0 && dir == organism::ASSET_DIR {
        return Err(format!("'{field}' is not a field of {file}.ron -- this row is session-only"));
    }
    if let Some(current) = field_text(&source, field) {
        if current.parse::<f64>().is_err() {
            return Err(format!("'{field}' is not a single number in {file}.ron -- edit the file by hand"));
        }
    }
    let updated = tunables::write_field_value(&source, field, param.tunable.value, param.tunable.integral)?;
    // Parsed before anything is written, never after: a file that does not
    // load is a file the next run of the game cannot start from.
    if dir == material::ASSET_DIR {
        ron::from_str::<material::MaterialDef>(&updated).map_err(|e| format!("edit would corrupt {file}.ron: {e}"))?;
    } else {
        ron::from_str::<organism::SpeciesDef>(&updated).map_err(|e| format!("edit would corrupt {file}.ron: {e}"))?;
    }
    Ok((path, updated))
}

/// How many times `field` appears as a whole-identifier key in `source`.
fn occurrences(source: &str, field: &str) -> usize {
    let bytes = source.as_bytes();
    let mut count = 0;
    let mut from = 0;
    while let Some(rel) = source[from..].find(field) {
        let start = from + rel;
        let end = start + field.len();
        from = end;
        let boundary = start == 0 || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
        let key = source[end..].trim_start().starts_with(':');
        if boundary && key {
            count += 1;
        }
    }
    count
}

/// The text of `field`'s value as the file currently holds it, up to the first
/// separator — the same span `tunables::write_field_value` would replace.
fn field_text(source: &str, field: &str) -> Option<String> {
    let at = source.find(&format!("{field}:")).or_else(|| source.find(&format!("{field} :")))?;
    let after = source[at..].find(':')? + at + 1;
    let rest = &source[after..];
    let end = rest.find([',', ')', '\n']).unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

// -------------------------------------------------------------- a specimen

/// **What one individual is**, as label/value/explanation triples.
///
/// The other half of "data". The parameters pages above are a *species*'
/// numbers and reach every one of its members; this is the one you clicked,
/// and everything here differs between two individuals of the same species —
/// which is the whole premise of a box you are running selection in. The
/// design guide's ruling about the planting tool applies for the same reason
/// it applied there: without it, breeding is a slot machine.
///
/// Two shapes, because the kingdoms carry different state and a row that read
/// `--` for every plant would be worse than no row: a creature has heritable
/// body traits and a foraging errand, a plant has a genotype, an allele set
/// and a carbon economy.
pub fn specimen_rows(world: &World, id: u16) -> Vec<(String, String, String)> {
    specimen_sections(world, id).into_iter().flat_map(|(_, _, rows)| rows).collect()
}

/// One specimen readout row: label, value, and the note that explains it.
pub type SpecimenRow = (String, String, String);

/// One named group of specimen rows: its heading, what the heading means, and
/// the rows under it.
pub type SpecimenSection = (&'static str, &'static str, Vec<SpecimenRow>);

/// What each group's heading means, as the page's hover note reads it.
const LIFE_NOTE: &str = "WHERE THIS INDIVIDUAL CAME FROM, HOW FAR DOWN THE LINE IT IS, AND THE DATED LINES IT PUT IN THE RUN LOG. NOTHING HERE CHANGES ONCE IT IS WRITTEN -- A FRAME NUMBER IS SETTLED THE MOMENT THE THING HAPPENS, WHICH IS WHAT SEPARATES THIS GROUP FROM STATE.";
const STATE_NOTE: &str = "HOW IT IS DOING RIGHT NOW. THIS IS THE GROUP THAT MOVES WHILE THE BOX RUNS, AND THE ONE TO HAVE OPEN IF YOU ARE WATCHING SOMETHING GET INTO TROUBLE.";
const WORDS_NOTE: &str = "THE SAME GENOME, IN SENTENCES. WHAT KIND OF THING THIS IS, RATHER THAN WHAT ITS NUMBERS ARE -- EVERY LINE HERE IS DERIVED FROM A ROW UNDER GENOME, AND HOVERING ONE SAYS WHICH.";
const GENOME_NOTE: &str = "WHAT IT WAS DEALT AND CANNOT CHANGE, DRAWN WHEN IT WAS BORN AND CARRIED FOR LIFE. TWO INDIVIDUALS OF ONE SPECIES DIFFER HERE AND NOWHERE ELSE AT BIRTH -- THIS IS WHAT A JAR ON THE SHELF KEEPS.";

/// **The same readout, in the three groups the cell page folds it into.**
///
/// `LIFE` is where this individual came from, `STATE` is how it is doing right
/// now, and `GENOME` is what it was dealt and cannot change. Every kingdom
/// gets all three, in that order, and a group is never empty — the page draws
/// one header per group and a missing one would make two species' pages
/// disagree about which header means what.
///
/// **The grouping is here rather than in the page** because it is a statement
/// about what the numbers *are*, not about how they are drawn: `STATE` is the
/// block a player watches change while the box runs, and that is the same fact
/// whether it is folded, scrolled or printed by a harness.
pub fn specimen_sections(world: &World, id: u16) -> Vec<SpecimenSection> {
    let Some(state) = world.organism_state(id) else { return Vec::new() };
    let species = world.species.get(state.species);
    let mut life: Vec<SpecimenRow> = Vec::new();
    let mut rows: Vec<SpecimenRow> = Vec::new();
    let mut genome: Vec<SpecimenRow> = Vec::new();
    let mut row = |label: &str, value: String, note: &str| life.push((label.into(), value, note.into()));

    row("GENERATION", state.generation.to_string(),
        "HOW MANY ANCESTORS BACK TO A FOUNDER. A FOUNDER IS 0. IF THIS NEVER LEAVES 0 OR 1, NOTHING IN THE BOX IS BREEDING, WHICH IS THE ONE THING A POPULATION COUNT CANNOT TELL YOU BY ITSELF.");
    row("LINEAGE", state.lineage.to_string(),
        "WHICH FOUNDING LINE THIS INDIVIDUAL COMES FROM. TWO ANIMALS WITH THE SAME LINEAGE SHARE AN ANCESTOR IN THIS BOX; TWO WITH DIFFERENT ONES DO NOT.");
    // **Three states, not two.** `inherited` alone would report a specimen
    // released off the shelf as either "born here" (it was not -- nothing in
    // the box bore it) or "founder" (true economically, and it hides the one
    // thing the player did on purpose). A release is its own origin and says
    // so, or the rack's whole point -- did the line I picked do better --
    // cannot be read off a cell.
    // **`BORN` is `LIFE`, `AGE` is `STATE`.** The frame it was allocated is
    // settled the moment the thing exists, which is what this group is for;
    // how long ago that was moves every tick, which is what the next one is.
    row("BORN", format!("FRAME {}", state.born_frame),
        "THE FRAME THIS INDIVIDUAL WAS ALLOCATED. WITH ITS ORGANISM NUMBER IT IS WHAT PINS IT: A SLOT IS HANDED OUT AGAIN AFTER SIXTEEN REUSES, SO THE NUMBER ALONE WOULD FOLLOW WHATEVER LANDED IN IT NEXT. THE FRAME DOES NOT COME BACK.");
    row("ORIGIN", if state.stocked { "RELEASED FROM A JAR".into() } else if state.inherited { "BORN HERE".into() } else { "FOUNDER".into() },
        "WHERE THIS INDIVIDUAL CAME FROM. BORN HERE MEANS THE BOX BRED IT. FOUNDER MEANS IT WAS PLACED OUT OF NOTHING. RELEASED FROM A JAR MEANS YOU PUT IT BACK OFF THE SHELF, CARRYING A GENOME YOU KEPT. A BOX WHERE NOTHING EVER SAYS BORN HERE IS A BOX THAT HAS NOT REPRODUCED YET.");

    // **The individual's own lines out of the run log, filed under `LIFE`.**
    //
    // They belong here by the group's own contract: a frame number is settled
    // the moment the thing happens and never moves again, which is exactly
    // what separates `LIFE` from `STATE`. They were briefly a fifth group of
    // their own and that is why they are not: one extra heading is 15px, the
    // ant's page had about that much slack, and
    // `the_cell_page_fits_on_the_screen_for_a_plant_and_for_an_ant` went red
    // saying the page now fitted only because rows had been **dropped** --
    // which is the one thing that guard exists to refuse. A group that costs
    // a heading to say five short lines is not worth a trimmed page.
    life.extend(story(world, id, state.born_frame));

    // A second closure over `rows`, so the borrow of `life` ends here. The
    // shadowing is deliberate: every `row(...)` below this line files into
    // `STATE` and every one above it into `LIFE`, which is a good deal harder
    // to get wrong than a group argument repeated on eighteen call sites.
    let mut row = |label: &str, value: String, note: &str| rows.push((label.into(), value, note.into()));

    // **No energy row here.** The cell block above already prints the
    // organism's whole-body energy, and the same figure twice under one
    // heading reads as two different quantities that happen to agree.
    if species.creature.is_some() {
        // **The inherited traits are `GENOME`, not `STATE`**, for the reason
        // the grouping exists: they are drawn at birth and carried for life,
        // so they belong with the plant's genotype draws rather than with the
        // numbers that move while you watch.
        //
        // **Off the same `TRAIT_ROWS` table the parameters page reads**, and
        // that is the whole repair: this block listed two of what were four
        // slots, so an owner clicking an animal to ask what it had inherited
        // was shown half its genome with nothing to say so. A slot is now
        // visible here the moment it has a row there.
        for (slot, name, _) in TRAIT_ROWS {
            genome.push((name.to_uppercase(), format!("{:+.3}", state.traits[*slot]),
                format!("THIS ANIMAL'S OWN {}, INHERITED WITH JITTER RATHER THAN THE SPECIES VALUE ON THE ANTS PAGE. COMPARE THE TWO AND YOU ARE LOOKING AT HOW FAR THIS LINEAGE HAS DRIFTED.", name.replace('_', " ").to_uppercase())));
        }
        // **`forage_max`, not `since_nest`, and the swap is the same one the
        // roster's `FAR` row made.** This row read `SINCE NEST` and told the
        // reader that *"a number that only ever climbs is an ant that is
        // lost"*. Both halves stopped being true: `since_nest` counts ticks
        // at a species' `tick_interval`, so its scale is a constant rather
        // than a distance, and it is incremented unconditionally, so an ant
        // standing *on* the nest accrues it -- 136 of 142 of its resets were
        // loitering. It is documented measurement-only for exactly that
        // reason, and a per-individual row is precisely the case its own doc
        // warns about. It also disagreed with the `STATE` column beside it,
        // which is the worse failure on a page whose job is to be read.
        row("RANGE", state.forage_max.to_string(),
            "HOW FAR THIS ONE HAS GOT FROM THE LAST PLACE IT TOUCHED HOME, IN CELLS, ON THE TRIP IT IS ON NOW. IT RE-ANCHORS EVERY TIME IT TOUCHES THE NEST, SO STANDING AT HOME CANNOT RUN IT UP -- WHICH IS THE FAULT IN THE TICK COUNTER THIS ROW USED TO SHOW. PAST 30 THE ANIMALS PAGE CALLS IT FAR.");
        row("CROP", match &state.crop {
                Some(c) => format!("{} x{}", world.materials.get(c.material).display.to_uppercase(), c.cells),
                None => "EMPTY".into(),
            },
            "WHAT IT IS CARRYING AND HOW MUCH OF IT IS LEFT. THE NUMBER FALLS AS IT WALKS -- AN ANT DIGESTS ITS LOAD ON THE WAY HOME, SO A LONG TRIP DELIVERS LESS THAN A SHORT ONE.");
        row("BODY", state.cells.len().to_string(),
            "HOW MANY CELLS THIS ANIMAL IS. EVERY PER-CELL COST ON THE ANTS PAGE IS MULTIPLIED BY THIS.");
        row("AGE", format!("{} TICKS", world.frame.saturating_sub(state.born_frame)),
            "HOW LONG THIS ONE HAS BEEN ALIVE, IN SIMULATED TICKS. AGAINST THE ANIMALS PAGE'S OWN TURNOVER IT SAYS WHETHER YOU ARE LOOKING AT A FOUNDER THAT HAS OUTLASTED EVERYTHING OR AT SOMETHING BORN THIS MINUTE.");
        row("YOUNG", state.life.offspring.to_string(),
            "HOW MANY THIS ONE HAS BUDDED. IT IS ITS FITNESS, IN THE ONLY SENSE THE BOX MEASURES FOR AN ANIMAL -- AND A COLONY WHERE NOBODY'S NUMBER EVER LEAVES ZERO IS A COLONY THAT IS NOT BREEDING, WHICH A HEADCOUNT ALONE CANNOT TELL YOU.");
        row("FED", state.life.bites.to_string(),
            "MOUTHFULS TAKEN INTO THE CROP OVER ITS WHOLE LIFE. IT COUNTS PICKING FOOD UP AND NOT DIGESTING IT, WHICH ARE DIFFERENT EVENTS -- AN ANIMAL WITH A HIGH COUNT AND NO DELIVERIES IS EATING EVERYTHING IT FINDS WHERE IT FINDS IT.");
        row("DELIVERED", state.life.deliveries.to_string(),
            "LOADS IT HAS BROUGHT HOME. AGAINST FED IT IS THE HALF OF THE FORAGING LOOP THAT CLOSES: PICKUPS WITHOUT DELIVERIES IS A COLONY THAT FEEDS ITSELF AND NEVER STOCKS THE NEST.");
        row("DUG", state.life.digs.to_string(),
            "CELLS IT HAS EXCAVATED IN ITS LIFE. THE GALLERIES IN THE BED ARE THE SUM OF THESE.");
        row("WALKED", format!("{} / {} BLOCKED", state.life.moves, state.life.moves_blocked),
            "STEPS TAKEN, AND STEPS IT TRIED AND COULD NOT MAKE. THE SECOND NUMBER IS NOT WASTE -- A COLONY SPENDS A THIRD OF ITS LIFE TURNING ON THE SPOT -- BUT AN ANIMAL WHOSE BLOCKED COUNT DWARFS ITS MOVES IS ONE WEDGED SOMEWHERE.");
        return vec![
            ("WORDS", WORDS_NOTE, words(world, id)),
            ("LIFE", LIFE_NOTE, life),
            ("STATE", STATE_NOTE, rows),
            ("GENOME", GENOME_NOTE, genome),
        ];
    }

    row("SHOOT", state.shoot_cells.to_string(),
        "HOW MUCH SHOOT IT HAS GROWN. THIS IS THE NUMBER THE PLANT PAGE'S SEED MATURITY IS COMPARED AGAINST -- BELOW THAT FENCE THIS PLANT CANNOT SET A SEED AT ALL, HOWEVER MUCH ENERGY IT HAS.");
    row("ROOT", state.root_cells.to_string(),
        "HOW MUCH ROOT IT HAS. AGAINST THE SHOOT COUNT IT IS THE ROOT-TO-SHOOT BALANCE, WHICH IS WHAT DECIDES WHETHER IT DIES OF THIRST OR OF SHADE.");
    // **A plant's clock starts at seed set, not at germination**, because
    // that is when `bear_seed_at` allocates its organism. So this includes
    // however long it lay in the seed bank, and the row says so rather than
    // printing a number that means something different for the two kingdoms.
    row("AGE", format!("{} TICKS", world.frame.saturating_sub(state.born_frame)),
        "HOW LONG SINCE THIS INDIVIDUAL WAS CREATED, IN SIMULATED TICKS. FOR A PLANT THE CLOCK STARTS WHEN ITS PARENT SET THE SEED AND NOT WHEN IT GERMINATED, SO A LONG-DORMANT SEED READS OLD ON ITS FIRST DAY ABOVE GROUND. AN ANIMAL'S CLOCK STARTS AT ITS BIRTH.");
    row("SEEDS SET", state.seeds_set.to_string(),
        "SEEDS THIS INDIVIDUAL HAS SET IN ITS LIFE. IT IS ITS FITNESS, IN THE ONLY SENSE THE BOX MEASURES.");
    row("ROOT IN SOIL", {
        let (contact, total) = (state.contact_root_cells, state.root_cells.max(1));
        format!("{contact} ({}%)", contact * 100 / total)
    },
        "HOW MUCH OF THE ROOT IS ACTUALLY TOUCHING SOIL, AND SO ACTUALLY DRINKING. A ROOT CELL WALLED IN ON EVERY SIDE BY THE PLANT'S OWN ROOTS BUYS NOTHING AND STILL COSTS FOOD TO KEEP, SO TWO PLANTS OF THE SAME ROOT MASS CAN DIFFER BY NEARLY TWO TO ONE IN HOW MUCH OF IT EARNS. A LOW PERCENTAGE IS A SOLID BALL OF ROOT, WHICH IS WASTE.");
    row("WATER", format!("{:.2}", state.water_status),
        "HOW WELL WATERED IT IS: 1.00 IS SATISFIED AND 0.00 IS TAKING UP NOTHING IT ASKED FOR. A PLANT SITTING LOW HERE IS A PLANT WHOSE ROOTS CANNOT REACH.");
    row("UPTAKE/DEMAND", format!("{:.2} / {:.2}", state.water_uptake, state.water_demand),
        "THE TWO NUMBERS BEHIND WATER, AND THEY SAY WHICH PROBLEM YOU HAVE. UPTAKE IS WHAT THE ROOTS BROUGHT IN; DEMAND IS WHAT THE LEAVES WANT. LOW UPTAKE AGAINST ORDINARY DEMAND IS A PLANT THAT CANNOT REACH WATER -- GIVE IT SOME. ORDINARY UPTAKE AGAINST HIGH DEMAND IS A PLANT CARRYING MORE LEAF THAN ITS ROOTS CAN SUPPLY, WHICH WATERING WILL NOT FIX FOR LONG.");
    row("INCOME/UPKEEP", format!("{:.2} / {:.2}", state.income, state.maintenance),
        "WHAT IT EARNED LAST TICK IN CARBON, AGAINST WHAT STANDING STILL COSTS IT. INCOME ABOVE UPKEEP IS A PLANT THAT CAN GROW; BELOW IT, IT IS EATING ITSELF. INCOME IS NIGHT-SCALED, SO A PLANT READS LOW AT NIGHT WITHOUT BEING IN TROUBLE -- COMPARE THE PAIR, NOT THE FIRST NUMBER.");
    row("UNPAID", format!("{:.2}", state.maintenance_unpaid),
        "THE PART OF THE UPKEEP BILL IT COULD NOT PAY. ZERO ON ANY PLANT IN SURPLUS, WHICH IS MOST OF THEM FOR MOST OF THEIR LIVES -- SO ANYTHING ABOVE ZERO HERE IS THE ALARM, AND IT IS A CONTINUOUS AMOUNT RATHER THAN A COUNT OF STARVING CELLS BECAUSE A COUNT GIVES KNIFE-EDGE MARGINS.");
    row("STARVING", if state.starving_ticks == 0 {
        "NO".to_string()
    } else {
        format!("{} / {}", state.starving_ticks, crate::sim::plant::STARVATION_DEATH_TICKS)
    },
        "HOW LONG IT HAS FAILED TO PAY, AGAINST HOW LONG IT GETS BEFORE IT DIES OF IT. IT RESETS THE MOMENT THE PLANT CAN PAY AGAIN, SO THIS COUNTS A SUSTAINED FAILURE RATHER THAN A BAD AFTERNOON. READ IT AS A CLOCK: THE SECOND NUMBER IS DEATH.");
    row("BREEDING FUND", format!("{:.2}", state.reproductive_budget),
        "SURPLUS BANKED TOWARD SEED, RATHER THAN SPENT ON GROWTH. IT ACCRUES AND IS CAPPED INSTEAD OF BEING SPENT-OR-LOST, BECAUSE SEED SET FIRES ON A CHANCE AND THE SURPLUS HAS TO STILL BE THERE WHEN THE ROLL LANDS. A PLANT PARKED AT ZERO HERE IS ALIVE AND NOT REPRODUCING, WHICH IN THIS BOX IS THE SAME AS NOT COUNTING.");
    row("SENESCENT", if state.senescent { "YES -- ROTTING".into() } else { "NO".into() },
        "WHETHER IT IS DEAD AND ON ITS WAY OUT. A CULLED PLANT SAYS YES AND KEEPS ITS CELLS UNTIL THEY ROT, WHICH IS WHY A CULL IS GRADED RATHER THAN A DELETION.");
    // ...and the borrow of `rows` ends here, for the same reason: from this
    // line on, everything is `GENOME`.
    genome.push(("ALLELES".into(), state.alleles.iter().map(|a| a.to_string()).collect::<Vec<_>>().join("/"),
        "THE SIX DISCRETE MORPHOLOGY GENES, IN ORDER: LEAF ECONOMY, BRANCH ANGLE, INTERNODE, SYMPODIAL, TROPISM, WOOD DENSITY. THEY ARE CATEGORICAL, NOT SCALAR -- TWO PLANTS THAT DIFFER HERE ARE DIFFERENT SHAPES, NOT THE SAME SHAPE AT DIFFERENT SIZES.".into()));

    // The continuous genome, **only where the species gives it a width**. A
    // slot whose variance is zero is a draw with no consumer for this species:
    // printing `x1.00` for it would be ten rows of noise around whichever two
    // actually vary, which is the haystack this panel is built not to be.
    for (slot, label) in GENOTYPE_SLOTS.iter().enumerate() {
        let Some(width) = genotype_width(world, state.species, slot) else { continue };
        if width <= 0.0 {
            continue;
        }
        let factor = (1.0 + state.genotype_draws[slot] * width).max(0.0);
        genome.push((
            (*label).to_string(),
            format!("X{factor:.2}"),
            format!("THIS INDIVIDUAL'S OWN MULTIPLIER ON ITS SPECIES' {label}, DRAWN WHEN IT GERMINATED AND CARRIED FOR LIFE. 1.00 IS THE SPECIES VALUE; ITS SPECIES ALLOWS UP TO {:.0}% EITHER WAY. THIS IS WHY TWO SEEDS OF ONE SPECIES DO NOT GROW INTO THE SAME PLANT.", width * 100.0),
        ));
    }
    vec![
        ("WORDS", WORDS_NOTE, words(world, id)),
        ("LIFE", LIFE_NOTE, life),
        ("STATE", STATE_NOTE, rows),
        ("GENOME", GENOME_NOTE, genome),
    ]
}

/// **One individual's own lines out of the run log**, newest first.
///
/// The counters under `STATE` say *how much*; this says *when*, which is the
/// one question a standing number cannot answer -- at 1024x a player crosses
/// tens of thousands of frames between two glances, and "fed 41 times" does
/// not say whether it started this minute or has been foraging all run.
///
/// **Bounded by construction rather than by a cap**, which is the distinction
/// `CLAUDE.md`'s size-cap rule turns on: each of the five kinds fires at most
/// once in a life, so this is at most five rows however long the individual
/// lives, and there is no budget whose exhaustion could turn into an answer.
/// What the log *does* drop is old lines wholesale, and the row below says so
/// -- an individual older than the log's window has a truncated story, and a
/// truncated story must not read as an uneventful one.
fn story(world: &World, id: u16, born_frame: u64) -> Vec<SpecimenRow> {
    let mut rows: Vec<SpecimenRow> = world
        .run_log
        .about(id, born_frame)
        .map(|e| {
            (
                format!("F{}", e.frame),
                match e.kind {
                    world::LogKind::Born => "BORN".to_string(),
                    world::LogKind::Died => organism::DEATH_CAUSE_LIST
                        .get(e.other as usize)
                        .map(|c| c.label().to_string())
                        .unwrap_or_else(|| "DIED".to_string()),
                    world::LogKind::FirstFeed => "FIRST FED".to_string(),
                    world::LogKind::FirstSeed => "FIRST SEED".to_string(),
                    world::LogKind::LineEnded => format!("LINE {} ENDED", e.other),
                },
                "A LINE THIS INDIVIDUAL PUT IN THE RUN LOG, AT THE SIMULATED FRAME IT HAPPENED ON. THE BOX PAGE'S WHAT HAPPENED LIST IS THE SAME LOG WITH EVERYBODY IN IT.".to_string(),
            )
        })
        .collect();
    if rows.is_empty() {
        rows.push((
            "NO LINES".into(),
            "--".into(),
            "NOTHING THIS INDIVIDUAL DID HAS REACHED THE RUN LOG. EITHER IT HAS NOT YET DONE ANYTHING NOTABLE, OR IT IS OLD ENOUGH THAT ITS LINES HAVE AGED OUT OF THE LOG -- THE WHAT HAPPENED PAGE SAYS HOW MANY HAVE BEEN LOST FOR GOOD.".into(),
        ));
    }
    rows
}

/// **The genome read back as sentences** -- `plainspeak::describe`, as
/// specimen rows.
///
/// A row with no value column, because a phrase *is* the value: the label
/// column is 150 px and a sentence in it with a number beside it would wrap
/// or truncate, and the number is already in `GENOME` two headings down. The
/// explanation carries the weight or the allele it came from, so hovering any
/// sentence says why it was said.
fn words(world: &World, id: u16) -> Vec<SpecimenRow> {
    crate::lab::plainspeak::describe(world, id)
        .into_iter()
        .flat_map(|p| {
            // **A backstop, not the mechanism.** `page_rect` sizes the page
            // to its widest row and then clamps it onto the screen, so a long
            // sentence does not wrap of its own accord -- it widens the whole
            // page and slides it left over whatever it was opened from. A
            // thirty-character phrase took the cell page to 250 px and hid
            // three of the roster's eight columns behind it.
            //
            // So the phrases are *written* to fit, and
            // `plainspeak::every_phrase_fits_the_column` holds them to it over
            // every genome and every allele rather than over the ones anybody
            // thought of. This wrap is what happens if one ever slips through:
            // two short rows rather than a page that eats its neighbour. The
            // continuation carries the same explanation, so hovering either
            // half says the same thing.
            let note = p.detail;
            crate::lab::ui::wrap_words(&p.text, crate::lab::plainspeak::PHRASE_COLUMNS)
                .into_iter()
                .map(move |line| (line, String::new(), note.clone()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The genome's slot map, as `organism::GENOTYPE_TRAITS`' own doc names it.
/// Positional forever, on the same terms the constant is.
const GENOTYPE_SLOTS: [&str; organism::GENOTYPE_TRAITS] = [
    "SHOOT BRANCHING",
    "ROOT BRANCHING",
    "SHOOT PLASTOCHRON",
    "TURGOR PER CELL",
    "PIPE RATIO",
    "ROOT TROPISM",
    "ROOT:SHOOT BIAS",
    "STOMATAL CLOSURE",
    "ROOT PENETRATION",
    "STRAIN RESPONSE",
];

/// How wide this species lets `slot` vary.
///
/// **Slots 1, 5 and 8 are read off the root's `Grow` and the rest off the
/// shoot's**, exactly as `organism::GENOTYPE_TRAITS`' slot map says — that
/// separation is what lets a root and a shoot diverge inside one individual,
/// and a reader that took whichever arm came first would report the wrong
/// width for three of ten rows.
pub fn genotype_variance_of(world: &World, species: SpeciesId, slot: usize) -> Option<f32> {
    genotype_width(world, species, slot)
}

fn genotype_width(world: &World, species: SpeciesId, slot: usize) -> Option<f32> {
    let cell_type = if matches!(slot, 1 | 5 | 8) { CellType::RootTip } else { CellType::GrowingTip };
    world.species.get(species).behaviors(cell_type).iter().find_map(|b| match b {
        Behavior::Grow { genotype_variance, .. } => genotype_variance.get(slot).copied(),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::scene::LabBox;

    fn bed() -> (World, LabBox) {
        let spec = LabBox::default();
        (spec.build(), spec)
    }

    /// A private state file for the whole test, same reasoning as
    /// `lab::tests`' `SHELF_LOCK`: [`Dials::ASSET_PATH_ENV`] and
    /// [`LabBox::ASSET_PATH_ENV`] both resolve through process-global state,
    /// and `cargo test` runs a binary's tests in parallel by default.
    static STATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// A scratch path under this process's own tempdir — pid-tagged because
    /// `/tmp` is shared between agents in this project's containers
    /// (`Reports/lanes/evolution-lab-coordinator.md`).
    fn scratch_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("pixel_physics_lab_{tag}_{}.ron", std::process::id()))
    }

    #[test]
    fn a_saved_dials_round_trips_and_a_missing_file_reports_none() {
        let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let path = scratch_path("dials_roundtrip");
        let _ = std::fs::remove_file(&path);
        std::env::set_var(Dials::ASSET_PATH_ENV, &path);

        assert!(Dials::load_saved().is_none(), "nothing saved yet");

        let (mut world, _) = bed();
        world.plant_load_failure = false;
        world.mutation_sigma = 0.25;
        world.developmental_key = organism::DevelopmentalKey::Plant { coarseness: 3 };
        Dials::from_world(&world).save().expect("save");

        let loaded = Dials::load_saved().expect("a just-saved file parses back");
        assert!(!loaded.plant_load_failure);
        assert_eq!(loaded.mutation_sigma, 0.25);
        // coarseness 3 -> n - 1 == 3 -> n == 4, `Self::from_world`'s own encoding.
        assert_eq!(loaded.developmental_key, 4);

        let mut fresh = bed().0;
        loaded.apply_to(&mut fresh);
        assert!(!fresh.plant_load_failure);
        assert_eq!(fresh.mutation_sigma, 0.25);
        assert_eq!(fresh.developmental_key, organism::DevelopmentalKey::Plant { coarseness: 3 });

        let _ = std::fs::remove_file(&path);
        std::env::remove_var(Dials::ASSET_PATH_ENV);
    }

    /// **`save` used to refuse every `Bed`/`Rule`/`Heredity` row outright** —
    /// "session-only ... it lasts the session" — which was the larger half
    /// of "There is no save"
    /// (`Reports/lanes/evolution-lab-coordinator.md`, round three). Put the
    /// fault back (route these three through `planned_edit` again, as
    /// `save` used to) and this goes red: `CLAUDE.md`'s guard rule, applied
    /// to a green whose meaning just changed.
    #[test]
    fn save_now_persists_bed_and_rule_rows_instead_of_refusing_them() {
        let _guard = STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let bed_path = scratch_path("bed_save");
        let dials_path = scratch_path("dials_save");
        let _ = std::fs::remove_file(&bed_path);
        let _ = std::fs::remove_file(&dials_path);
        std::env::set_var(LabBox::ASSET_PATH_ENV, &bed_path);
        std::env::set_var(Dials::ASSET_PATH_ENV, &dials_path);

        let (world, spec) = bed();
        let all = registry(&world, &spec, None);
        let width = all.iter().find(|p| matches!(p.knob, Knob::Bed { field: "width" })).expect("width is registered");
        let rule = all
            .iter()
            .find(|p| matches!(p.knob, Knob::Rule { field: "plant_load_failure" }))
            .expect("plant_load_failure is registered");

        assert!(save(width, &world, &spec).is_ok(), "a Bed row should now save");
        assert!(bed_path.exists(), "LabBox::save wrote a file");
        assert!(save(rule, &world, &spec).is_ok(), "a Rule row should now save");
        assert!(dials_path.exists(), "Dials::save wrote a file");

        let _ = std::fs::remove_file(&bed_path);
        let _ = std::fs::remove_file(&dials_path);
        std::env::remove_var(LabBox::ASSET_PATH_ENV);
        std::env::remove_var(Dials::ASSET_PATH_ENV);
    }

    /// **The economy rows carry live numbers, not zeroes** — the guard for
    /// the way this readout fails silently.
    ///
    /// `specimen_rows` reads six fields that no other page touches
    /// (`income`, `maintenance`, `maintenance_unpaid`, `water_uptake`,
    /// `water_demand`, `contact_root_cells`). A row wired to a field that
    /// is never written compiles, renders, and shows `0.00` for ever —
    /// which is indistinguishable from a plant that genuinely earns
    /// nothing, and is `CLAUDE.md`'s "a channel needs a writer and a
    /// reader, and the compiler checks neither" exactly.
    ///
    /// So this grows a real bed and asserts the pair that cannot both be
    /// zero on a living plant: it earned something, and it wants water.
    #[test]
    fn the_examine_page_economy_rows_are_not_all_zero() {
        let (mut world, _) = bed();
        let mut particles = crate::sim::particle::ParticleSystem::default();
        let mut blasts = crate::sim::explosion::Blasts::default();
        let tuning = crate::sim::player::Tuning::default();
        for _ in 0..3_000 {
            crate::sim::frame::step(&mut world, &mut particles, &mut blasts, crate::sim::player::PlayerInput::default(), &tuning);
        }
        // The biggest plant, so the assertion is about an established
        // individual rather than a seed that landed last tick.
        let plant = world
            .live_organism_ids()
            .into_iter()
            .filter(|id| {
                world
                    .organism(*id)
                    .is_some_and(|s| world.species.get(s.species).creature.is_none())
            })
            .max_by_key(|id| world.organism(*id).map_or(0, |s| s.cells.len()))
            .expect("the default bed grows plants");
        let state = world.organism(plant).expect("just found it");
        assert!(state.cells.len() > 5, "test setup: wanted an established plant, got {} cells", state.cells.len());

        let rows = specimen_rows(&world, plant);
        let find = |label: &str| {
            rows.iter()
                .find(|(l, _, _)| l == label)
                .unwrap_or_else(|| panic!("no `{label}` row -- the label was renamed and this guard silently stopped checking it"))
                .1
                .clone()
        };
        // Named rather than indexed: a row inserted above would shift an
        // index and the guard would quietly check the wrong line.
        let (income, upkeep, demand) = (state.income, state.maintenance, state.water_demand);
        println!(
            "biggest plant: {} cells, income {income:.3} upkeep {upkeep:.3} unpaid {:.3} uptake {:.3} demand {demand:.3} contact root {}/{}",
            state.cells.len(),
            state.maintenance_unpaid,
            state.water_uptake,
            state.contact_root_cells,
            state.root_cells
        );
        for label in ["ROOT IN SOIL", "UPTAKE/DEMAND", "INCOME/UPKEEP", "UNPAID", "STARVING", "BREEDING FUND"] {
            let v = find(label);
            assert!(!v.is_empty(), "`{label}` rendered an empty value");
        }
        // A standing plant costs something to keep and its leaves want
        // water; both being exactly zero means the fields are not written.
        assert!(upkeep > 0.0, "UPKEEP is {upkeep} on a {}-cell plant -- the maintenance field is not being written", state.cells.len());
        assert!(demand > 0.0, "DEMAND is {demand} on a {}-cell plant -- the water_demand field is not being written", state.cells.len());
    }

    /// Every plantable species, so the plant page's registrations are covered
    /// for every species the chip can arm rather than only for whichever one
    /// happens to be first.
    fn every_page(world: &World, spec: &LabBox) -> Vec<Param> {
        let mut out = registry(world, spec, None);
        for id in crate::lab::ui::plantable_species(world) {
            out.extend(registry(world, spec, Some(id)).into_iter().filter(|p| p.group == Group::Plant));
        }
        out
    }

    /// **The positive control for the whole panel.**
    ///
    /// A registered row whose `write` arm is missing draws, highlights, takes
    /// a click and does nothing — `CLAUDE.md`'s "a channel needs a writer and
    /// a reader", with the reader alive and the writer absent, which is the
    /// shape that reads as working code. `write`'s `_ => return false` floor
    /// makes that a silent no-op by construction, so this is the thing that
    /// stops it being silent.
    ///
    /// Watched failing rather than assumed green: deleting any one arm of
    /// `write` reds this immediately and names the row.
    /// **The developmental dial is live for plants already standing**, not
    /// only for ones that germinate after it moves.
    ///
    /// `dev_seed` is stamped once at germination from the coarseness in force
    /// then, which is right for a hot path read once per organism cell per
    /// tick and wrong for a control the owner turns mid-run. The fault this
    /// pins is specific and was really there: without
    /// `World::refold_developmental_seeds`, a plant that came up at setting 0
    /// and then saw the dial moved to 2 kept `dev_seed == lineage_seed` — the
    /// *coarseness 0* fold — so the box ran one rule for old plants and
    /// another for new ones and the dial reported a setting it was not
    /// applying.
    ///
    /// Written as three settings rather than two because the failure needs
    /// the pair to disagree: at coarseness 0 the fold is the identity, so a
    /// version that never re-folded would still pass a 0-versus-off check.
    #[test]
    fn moving_the_developmental_dial_reaches_plants_already_standing() {
        let (mut world, spec) = bed();
        let herb = world.species.id_of("herb").expect("herb is compiled in");
        let id = world.push_organism(herb).expect("a slot is free");
        crate::sim::plant::seed_genotype(&mut world, id, 40, 30);
        // Germinate it under the shipped key, which is the case that matters:
        // a box the owner has been watching before touching the dial.
        crate::sim::plant::stamp_origin(&mut world, id, 40, 30);
        let lineage = world.organism(id).expect("live").lineage_seed;
        assert_ne!(lineage, 0, "test setup: a founder must draw a lineage seed");

        let row = registry(&world, &spec, Some(herb))
            .into_iter()
            .find(|p| matches!(p.knob, Knob::Heredity { field: "developmental_key" }))
            .expect("the shared_development row is registered");

        let mut spec = spec;
        let mut folded = Vec::new();
        for setting in [1.0f32, 2.0, 3.0] {
            assert!(write(&mut world, &mut spec, &row.knob, setting), "setting {setting} must be writable");
            folded.push(world.organism(id).expect("live").dev_seed);
        }
        assert_eq!(folded[0], lineage, "setting 1 drops position, so the fold is the lineage seed itself");
        assert_ne!(folded[1], folded[0], "setting 2 folds the germination coordinate in and must differ");
        assert_ne!(folded[2], folded[1], "setting 3 bands at a different coarseness and must differ again");
    }

    #[test]
    fn every_writable_parameter_actually_moves() {
        let (mut world, mut spec) = bed();
        let all = every_page(&world, &spec);
        let mut checked = 0;
        for param in &all {
            if !param.writable() {
                continue;
            }
            // Away from whichever end it is already sitting at, so a knob
            // shipped at its own ceiling (`water.flow_rate` is 1000 of 1000)
            // is not read as stuck.
            let sign = if param.tunable.value >= param.tunable.max { -1 } else { 1 };
            let target = param.tunable.stepped(sign);
            assert!(
                write(&mut world, &mut spec, &param.knob, target),
                "{}.{} is registered and has no writer",
                param.tunable.category,
                param.tunable.name
            );
            let after = every_page(&world, &spec)
                .into_iter()
                .find(|p| p.tunable.category == param.tunable.category && p.tunable.name == param.tunable.name)
                .expect("the row is still registered after writing it");
            assert_ne!(
                after.tunable.value, param.tunable.value,
                "{}.{} accepted a write and did not change: {} -> {} (asked for {target})",
                param.tunable.category, param.tunable.name, param.tunable.value, after.tunable.value
            );
            checked += 1;
        }
        assert!(checked >= 30, "only {checked} writable rows -- the registry has lost most of itself");
    }

    /// A read-only row must be refused rather than quietly written somewhere.
    #[test]
    fn a_shown_only_row_is_refused() {
        let (mut world, mut spec) = bed();
        assert!(!write(&mut world, &mut spec, &Knob::ReadOnly, 1.0));
    }

    /// **Every character the panel draws has to exist in the font.**
    ///
    /// `hud::draw_text` renders anything outside its 5x7 set as a silent
    /// blank, and that trap has shipped three times here. This registry is the
    /// widest surface of prose in the lab -- forty notes, four page notes and
    /// every field name, several of which come out of an asset file rather
    /// than out of this source.
    #[test]
    fn every_word_the_panel_draws_is_drawable() {
        let (world, spec) = bed();
        let check = |what: &str, text: &str| {
            for c in text.chars() {
                assert!(hud_can_draw(c), "{what} contains {c:?}, which draws as a blank: {text}");
            }
        };
        for group in GROUPS {
            check("a page label", group.label());
            check("a page note", group.note());
        }
        for p in every_page(&world, &spec) {
            check("a row note", &p.note);
            check("a row value", &p.display());
            check("a row range", &p.range());
            // The name goes through the panel's own `_`-to-space pass, so it
            // is checked in the form it is actually drawn in.
            check("a row name", &p.tunable.name.replace('_', " "));
        }
    }

    fn hud_can_draw(c: char) -> bool {
        crate::hud::has_glyph(c)
    }

    /// **A panel with four hundred rows is not access, it is a haystack.**
    /// The ceiling is stated rather than implied so that adding a page's worth
    /// of rows is a decision somebody makes on purpose.
    #[test]
    fn no_page_is_longer_than_two_screens() {
        let (world, spec) = bed();
        for id in crate::lab::ui::plantable_species(&world).into_iter().map(Some).chain([None]) {
            let all = registry(&world, &spec, id);
            for group in GROUPS {
                let n = all.iter().filter(|p| p.group == group).count();
                assert!(n <= 20, "the {} page has {n} rows", group.label());
            }
        }
    }

    /// **The save path's two refusals, both with the case they are for.**
    ///
    /// Not an assertion that saving works — that would write into
    /// `assets/species/` from a test — but that the checks in front of the
    /// write fire on the files that need them and stay quiet on the ones that
    /// do not. `CLAUDE.md`: put the fault back and watch it go red. The fault
    /// here is already in the tree, which is the point: `tree.ron` really does
    /// hold two `crowding_weight` lines, the shoot's 30.0 and the root's
    /// deliberate 0.0, and a span edit takes the first.
    #[test]
    fn saving_refuses_a_field_it_could_edit_the_wrong_copy_of() {
        let (world, spec) = bed();
        let Some(tree) = world.species.id_of("tree") else { return };
        let plant: Vec<Param> = registry(&world, &spec, Some(tree))
            .into_iter()
            .filter(|p| p.group == Group::Plant)
            .collect();
        let find = |name: &str| plant.iter().find(|p| p.tunable.name == name);

        let crowding = find("crowding_weight").expect("the plant page registers crowding_weight");
        let refused = save_check(crowding);
        assert!(refused.contains("appears 2 times"), "crowding_weight was not refused: {refused}");

        // ...and the negative control, or the guard above would pass with the
        // save path refusing everything.
        let maturity = find("seed_maturity").expect("the plant page registers seed_maturity");
        let allowed = save_check(maturity);
        assert!(allowed.starts_with("would write"), "seed_maturity should be savable: {allowed}");
    }

    /// A row that is not a field of its own file — `gut_bias` is one element
    /// of `traits: (0.0, -0.2)` — must be refused rather than appended as a
    /// new key nothing reads. Silently writing a file that parses, loads and
    /// ignores the edit is the one outcome worse than refusing.
    #[test]
    fn saving_refuses_a_row_that_is_not_a_field() {
        let (world, spec) = bed();
        let ants: Vec<Param> = registry(&world, &spec, None).into_iter().filter(|p| p.group == Group::Ants).collect();
        let Some(gut) = ants.iter().find(|p| p.tunable.name == "gut_bias") else { return };
        let refused = save_check(gut);
        assert!(refused.contains("not a field"), "gut_bias was not refused: {refused}");
        let dig = ants.iter().find(|p| p.tunable.name == "dig_force").expect("the ants page registers dig_force");
        assert!(save_check(dig).starts_with("would write"), "dig_force should be savable");
    }

    /// **`occurrences` counts keys, not substrings.** `seed_cost` inside
    /// `seed_costs` is a different field, and a comment mentioning a field
    /// name is not a field. Getting this wrong in the permissive direction
    /// lets an ambiguous save through, which is the failure the whole check
    /// exists to stop.
    #[test]
    fn occurrences_counts_only_whole_keys() {
        assert_eq!(occurrences("(seed_cost: 1.0)", "seed_cost"), 1);
        assert_eq!(occurrences("(seed_costs: 1.0)", "seed_cost"), 0);
        assert_eq!(occurrences("(xseed_cost: 1.0)", "seed_cost"), 0);
        assert_eq!(occurrences("// seed_cost is the price\n(seed_cost: 1.0)", "seed_cost"), 1);
        assert_eq!(occurrences("(a: (cost: 1.0), b: (cost: 2.0))", "cost"), 2);
    }

    /// A tuple or a list must not be mistaken for a number, or the span edit
    /// replaces up to the first comma and leaves `, -0.2)` dangling.
    /// **Every heritable trait slot is reachable from this page.**
    ///
    /// The guard for the failure that prompted `TRAIT_ROWS`: `reproduce_at`
    /// and `sight_range` both shipped with the two older slots registered by
    /// hand beside them and no row of their own, so half the genome was
    /// unreachable and unviewable and nothing said so. A count is enough --
    /// the table is indexed by the `TRAIT_*` constants, so a row that exists
    /// cannot be pointing at the wrong slot without failing to compile.
    #[test]
    fn every_trait_slot_has_a_row() {
        assert_eq!(
            TRAIT_ROWS.len(),
            organism::CREATURE_TRAITS,
            "a heritable trait slot has no row on the parameters page, so the owner can neither set it nor see it -- \
             that is how two of the first four slots shipped unreachable"
        );
        let mut seen: Vec<usize> = TRAIT_ROWS.iter().map(|(slot, ..)| *slot).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), TRAIT_ROWS.len(), "two rows name the same slot, so some other slot has none");
    }

    #[test]
    fn field_text_reads_the_span_the_edit_would_replace() {
        assert_eq!(field_text("(dig_force: 0.85,)", "dig_force").as_deref(), Some("0.85"));
        assert_eq!(field_text("(traits: (0.0, -0.2),)", "traits").as_deref(), Some("(0.0"));
        assert!(field_text("(traits: (0.0, -0.2),)", "traits").unwrap().parse::<f64>().is_err());
        assert_eq!(field_text("(density: 1.3 // heavy\n)", "density").as_deref(), Some("1.3 // heavy"));
    }

    /// **A taller box keeps its proportions**, which is the one thing a size
    /// row can get wrong in a way that looks like a broken game.
    ///
    /// `lab_resolution` records the trap this guards: at `height=640` with
    /// `ground_y` left at 160 the soil sits in the top quarter and 390 rows
    /// are empty void. That is a scene error wearing a result, and now that
    /// height is a knob the player can turn it is one they would meet on
    /// their first press rather than one only a harness could reach.
    ///
    /// Three claims, because the first alone passes on a writer that simply
    /// pins `ground_y` to a constant fraction and forgets the box: the
    /// surface scales, the *soil* is still soil-depth thick under it rather
    /// than the whole box, and writing the same height twice is a no-op.
    #[test]
    fn raising_the_box_raises_the_ground_with_it() {
        let (_, mut spec) = bed();
        let (h0, g0) = (spec.height, spec.ground_y);
        assert!(write_bed(&mut spec, "height", (h0 * 2) as f32));
        assert_eq!(spec.height, h0 * 2);
        assert_eq!(
            spec.ground_y,
            g0 * 2,
            "the soil surface must ride the box: {h0} -> {} rows left ground at {}, which is the \
             top-quarter bed lab_resolution documents",
            h0 * 2,
            spec.ground_y
        );
        // Depth is its own knob and must NOT have been dragged along.
        assert_eq!(spec.soil_depth, bed().1.soil_depth, "height must not move the soil depth");

        // Idempotent: a sweep writes every setting into a fresh clone of one
        // spec, so a second write of the same value must not scale twice.
        let g = spec.ground_y;
        assert!(write_bed(&mut spec, "height", (h0 * 2) as f32));
        assert_eq!(spec.ground_y, g, "writing the same height twice scaled the ground twice");

        // And back down again returns where it started, so the row is not a
        // ratchet the player cannot undo.
        assert!(write_bed(&mut spec, "height", h0 as f32));
        assert_eq!((spec.height, spec.ground_y), (h0, g0));
    }

    /// The size rows are bounded, because the cost is memory and nothing in
    /// the engine would refuse a box too large to hold.
    #[test]
    fn the_box_size_is_clamped_at_both_ends() {
        let (_, mut spec) = bed();
        assert!(write_bed(&mut spec, "width", 1.0));
        assert_eq!(spec.width, MIN_BOX, "a tiny width must clamp, not build a box with no room in it");
        assert!(write_bed(&mut spec, "width", 999_999.0));
        assert_eq!(spec.width, MAX_BOX, "an unbounded width would let the rack exhaust the machine");
        assert_eq!(read_bed(&spec, "width"), Some(MAX_BOX as f32), "width must read back through the same table");
        assert_eq!(read_bed(&spec, "height"), Some(spec.height as f32));
    }

    /// The bed's rows change the spec and nothing else — a rebuild is what
    /// applies them, which is what every one of their notes says.
    #[test]
    fn a_bed_row_moves_the_spec_and_not_the_world() {
        let (mut world, mut spec) = bed();
        let before = world.frame;
        assert!(write(&mut world, &mut spec, &Knob::Bed { field: "soil_depth" }, 96.0));
        assert_eq!(spec.soil_depth, 96);
        assert_eq!(world.frame, before, "writing the spec must not touch the running world");
        assert!(needs_rebuild(&Knob::Bed { field: "soil_depth" }));
        assert!(!needs_rebuild(&Knob::Material { material: "soil", field: "density" }));
    }
}
