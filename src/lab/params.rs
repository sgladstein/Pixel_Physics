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
use crate::sim::world::World;
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
}

/// In tab order. One list, so the tab strip, the key and the tests cannot
/// disagree about what pages exist — `ui::TOOLS`' reason.
pub const GROUPS: [Group; 4] = [Group::Ground, Group::Plant, Group::Ants, Group::Box];

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Ground => "GROUND",
            Group::Plant => "PLANT",
            Group::Ants => "ANTS",
            Group::Box => "BOX",
        }
    }

    /// What the page is for, shown on hover over its own tab.
    pub fn note(self) -> &'static str {
        match self {
            Group::Ground => "WHAT THE BED IS MADE OF. SOIL IS WHAT ROOTS GO INTO AND ANTS DIG THROUGH; PACKED SOIL IS WHAT AN ANT LEAVES BEHIND WHEN IT DIGS, AND IT IS THE ONLY REASON A TUNNEL STAYS OPEN. CHANGES HERE ARE FELT ON THE NEXT TICK.",
            Group::Plant => "THE PLANT THE SPECIES CHIP ON THE BAR HAS ARMED. THESE ARE THE SPECIES' OWN NUMBERS, NOT ONE INDIVIDUAL'S -- MOVING ONE CHANGES EVERY PLANT OF THAT SPECIES ALREADY STANDING, ON THE NEXT TICK, AS WELL AS EVERY SEED YOU PLANT AFTERWARDS.",
            Group::Ants => "THE COLONY SPECIES. SAME RULE AS THE PLANT PAGE: THESE ARE THE SPECIES' NUMBERS AND THEY REACH EVERY ANT ALIVE. AN INDIVIDUAL'S OWN INHERITED TRAITS ARE ON THE CELL PAGE -- CLICK AN ANT WITH THE LOOK TOOL.",
            Group::Box => "THE BED AND THE LAMPS OVER IT. THE LAMP IS LIVE. EVERYTHING ELSE HERE IS THE SPEC THE BOX IS BUILT FROM, SO IT TAKES EFFECT WHEN YOU REBUILD -- CHANGE IT, THEN PRESS REBUILD.",
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
        Behavior::Grow { cost, continuation_weight, wind_weight, crowding_weight, max_active_tips, .. } => Some(match field {
            "cost" => *cost,
            "continuation_weight" => *continuation_weight,
            "wind_weight" => *wind_weight,
            "crowding_weight" => *crowding_weight,
            "max_active_tips" => *max_active_tips as f32,
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
        Behavior::Reproduce { seed_cost, reproductive_allocation, seed_maturity } => Some(match field {
            "seed_cost" => *seed_cost,
            "reproductive_allocation" => *reproductive_allocation,
            "seed_maturity" => *seed_maturity as f32,
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

fn creature_value(world: &World, species: &str, field: &str) -> Option<f32> {
    let id = world.species.id_of(species)?;
    let def = world.species.get(id).creature.as_ref()?;
    Some(match field {
        "dig_force" => def.dig_force,
        "hunger_fraction" => def.hunger_fraction,
        "body_energy" => def.body_energy,
        "start_energy" => def.start_energy,
        "reproduce_threshold" => def.reproduce_threshold,
        "mutation_rate" => def.mutation_rate,
        "tick_interval" => def.tick_interval as f32,
        "idle_cost_per_cell" => def.idle_cost_per_cell,
        "move_cost_per_cell" => def.move_cost_per_cell,
        "nest_memory" => def.nest_memory as f32,
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
    cr("hunger_fraction", span(0.0, 1.0, 0.02), false,
        "HOW EMPTY AN ANT HAS TO GET BEFORE IT GOES LOOKING FOR FOOD, AS A SHARE OF ITS FULL BELLY. LOW MEANS A COLONY THAT ONLY FORAGES IN A CRISIS; HIGH MEANS ONE THAT IS ALWAYS OUT.");
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
    cr("nest_memory", span(0.0, 4000.0, 50.0), true,
        "HOW LONG AN ANT REMEMBERS WHERE THE NEST WAS. IT IS WHAT LETS A FORAGER COME BACK RATHER THAN WANDER.");

    if let Some(id) = world.species.id_of(species) {
        if let Some(def) = world.species.get(id).creature.as_ref() {
            out.push(float(g, Knob::CreatureTrait { species: sp.clone(), slot: organism::TRAIT_GUT_BIAS }, species, "gut_bias",
                def.traits[organism::TRAIT_GUT_BIAS], span(-1.0, 1.0, 0.05),
                "WHERE THIS LINEAGE'S DIGESTION SITS BETWEEN PLANT MATTER (-1) AND FLESH (+1). IT IS HERITABLE, SO THIS ROW IS THE ANCESTRAL VALUE A NEWBORN STARTS FROM AND NOT WHAT ANY ANT ALIVE HAS -- CLICK ONE WITH THE LOOK TOOL TO SEE ITS OWN."));
            out.push(float(g, Knob::CreatureTrait { species: sp, slot: organism::TRAIT_BIRTH_GRANT }, species, "birth_grant",
                def.traits[organism::TRAIT_BIRTH_GRANT], span(-1.0, 1.0, 0.05),
                "HOW MUCH OF START ENERGY A NEWBORN IS ACTUALLY HANDED. HERITABLE, LIKE GUT BIAS, SO THIS IS THE ANCESTRAL VALUE. IT IS THE PARENT'S INVESTMENT PER OFFSPRING."));
        }
    }
}

fn box_rows(world: &World, spec: &LabBox, out: &mut Vec<Param>) {
    let g = Group::Box;
    if let Some(value) = material_value(world, "crystal", "glow") {
        out.push(float(g, Knob::Material { material: "crystal", field: "glow" }, "crystal", "glow", value, span(0.0, 4.0, 0.1),
            "HOW BRIGHT THE GROW LAMPS ARE. THE BED IS SEALED, SO THIS IS THE ONLY LIGHT IN IT AND IT IS THE WHOLE OF THE PLANTS' INCOME. 4.0 IS THE ENGINE'S CEILING; AT 0 EVERY PLANT IN THE BOX STARVES. IT IS FELT ON THE NEXT TICK -- TURN IT DOWN AND WATCH THE LIGHT OVERLAY."));
    }
    let mut bed = |field: &'static str, value: f32, s: Span, note: &str| {
        out.push(integer(g, Knob::Bed { field }, "the bed", field, value, s, note));
    };
    bed("lamp_spacing", spec.lamp_spacing as f32, span(8.0, 512.0, 8.0),
        "HOW FAR APART THE LAMPS ARE, IN CELLS. CLOSER IS MORE OF THEM AND A MORE EVENLY LIT BED; THERE IS ALWAYS AT LEAST ONE PER COMPARTMENT, BECAUSE A WALLED-OFF DARK BED IS A SILENT WAY TO KILL A POPULATION. TAKES EFFECT ON REBUILD.");
    bed("soil_depth", spec.soil_depth as f32, span(8.0, 240.0, 8.0),
        "HOW MANY ROWS OF SOIL THE BED HAS. DEEP SOIL IS ROOM FOR ROOTS AND FOR TUNNELS, AND IT IS PAID FOR IN FRAME TIME -- 40 ROWS TO 240 COSTS ABOUT TWICE THE FRAME. TAKES EFFECT ON REBUILD.");
    bed("ground_y", spec.ground_y as f32, span(40.0, 300.0, 10.0),
        "WHICH SCREEN ROW THE SOIL SURFACE SITS AT. LOWER ON THE SCREEN IS A DEEPER BED WITH LESS AIR OVER IT; HIGHER LEAVES MORE ROOM FOR A PLANT TO STAND UP IN. TAKES EFFECT ON REBUILD.");
    bed("compartments", spec.compartments as f32, span(1.0, 8.0, 1.0),
        "HOW MANY SEALED WALLS FLOOR TO CEILING THE BED IS DIVIDED BY. THEY BUY EVOLUTIONARY ISOLATION -- SEPARATE POPULATIONS THAT CANNOT MIX -- AND THEY ALSO BUY SPEED. TAKES EFFECT ON REBUILD.");
    bed("founders", spec.founders as f32, span(0.0, 64.0, 1.0),
        "HOW MANY PLANTS THE BOX IS STOCKED WITH WHEN IT IS BUILT. THE BINARY OPENS AT ZERO ON PURPOSE -- THE BOX STARTS WITH NOTHING AND YOU STOCK IT -- SO RAISE THIS ONLY IF YOU WANT A REBUILD TO HAND YOU A STAND. TAKES EFFECT ON REBUILD.");
    bed("colonies", spec.colonies as f32, span(0.0, 8.0, 1.0),
        "HOW MANY ANT COLONIES A REBUILD RELEASES, ONE PER COMPARTMENT AT MOST. TAKES EFFECT ON REBUILD.");
    bed("seed", spec.seed as f32, span(0.0, 999.0, 1.0),
        "THE NUMBER THIS BOX IS BUILT FROM. THE SAME SEED AND THE SAME BUILD REBUILD THE SAME BOX EXACTLY, WHICH IS WHAT LETS YOU CHANGE ONE PARAMETER AND COMPARE TWO RUNS RATHER THAN TWO WORLDS. TAKES EFFECT ON REBUILD.");
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
                "hunger_fraction" => def.hunger_fraction = value,
                "body_energy" => def.body_energy = value,
                "start_energy" => def.start_energy = value,
                "reproduce_threshold" => def.reproduce_threshold = value,
                "mutation_rate" => def.mutation_rate = value,
                "tick_interval" => def.tick_interval = value.max(1.0).round() as u64,
                "idle_cost_per_cell" => def.idle_cost_per_cell = value,
                "move_cost_per_cell" => def.move_cost_per_cell = value,
                "nest_memory" => def.nest_memory = value.max(0.0).round() as u16,
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
                if let Behavior::Grow { cost, continuation_weight, wind_weight, crowding_weight, max_active_tips, .. } = b {
                    match *field {
                        "cost" => *cost = value,
                        "continuation_weight" => *continuation_weight = value,
                        "wind_weight" => *wind_weight = value,
                        "crowding_weight" => *crowding_weight = value,
                        "max_active_tips" => *max_active_tips = value.max(1.0).round() as u32,
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
                    if let Behavior::Reproduce { seed_cost, reproductive_allocation, seed_maturity } = b {
                        match *field {
                            "seed_cost" => *seed_cost = value,
                            "reproductive_allocation" => *reproductive_allocation = value,
                            "seed_maturity" => *seed_maturity = value.max(0.0).round() as u32,
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
        Knob::Bed { field } => {
            let v = value.max(0.0).round();
            match *field {
                "lamp_spacing" => spec.lamp_spacing = v as i32,
                "soil_depth" => spec.soil_depth = v as i32,
                "ground_y" => spec.ground_y = v as i32,
                "compartments" => spec.compartments = v as usize,
                "founders" => spec.founders = v as usize,
                "colonies" => spec.colonies = v as usize,
                "seed" => spec.seed = v as u64,
                _ => return false,
            }
            true
        }
    }
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
/// [`Knob::Bed`] has no asset file at all — it is the running spec — and says
/// so rather than reporting a save that did not happen, which is
/// `App::save_tunable`'s rule for the weather pin.
pub fn save(param: &Param) -> Result<String, String> {
    let (path, updated) = planned_edit(param)?;
    std::fs::write(&path, updated).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(format!(
        "SAVED {} {} = {}",
        path.file_name().map_or_else(|| "?".into(), |f| f.to_string_lossy().to_uppercase()),
        param.tunable.name.to_uppercase(),
        param.tunable.display()
    ))
}

/// **What [`save`] would do, without doing it** — the same checks, the same
/// text edit, the same parse, and no write.
///
/// Exists so a harness can report whether every registered row is savable
/// without editing the repository's own asset files, which is a check nobody
/// could run twice. It is the *same code path* rather than a second claim
/// about it: [`save`] is this plus one `fs::write`.
pub fn save_check(param: &Param) -> String {
    match planned_edit(param) {
        Ok((path, _)) => format!("would write {}", path.display()),
        Err(e) => e,
    }
}

/// The file this parameter would be written to, and the whole new text of it.
fn planned_edit(param: &Param) -> Result<(std::path::PathBuf, String), String> {
    let (dir, file) = match &param.knob {
        Knob::ReadOnly => return Err("this row cannot be changed, so there is nothing to save".into()),
        Knob::Bed { .. } => {
            return Err("the bed's spec is session-only -- it has no file. rebuild to apply it".into())
        }
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
    let Some(state) = world.organism_state(id) else { return Vec::new() };
    let species = world.species.get(state.species);
    let mut rows: Vec<(String, String, String)> = Vec::new();
    let mut row = |label: &str, value: String, note: &str| rows.push((label.into(), value, note.into()));

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
    row("ORIGIN", if state.stocked { "RELEASED FROM A JAR".into() } else if state.inherited { "BORN HERE".into() } else { "FOUNDER".into() },
        "WHERE THIS INDIVIDUAL CAME FROM. BORN HERE MEANS THE BOX BRED IT. FOUNDER MEANS IT WAS PLACED OUT OF NOTHING. RELEASED FROM A JAR MEANS YOU PUT IT BACK OFF THE SHELF, CARRYING A GENOME YOU KEPT. A BOX WHERE NOTHING EVER SAYS BORN HERE IS A BOX THAT HAS NOT REPRODUCED YET.");

    // **No energy row here.** The cell block above already prints the
    // organism's whole-body energy, and the same figure twice under one
    // heading reads as two different quantities that happen to agree.
    if species.creature.is_some() {
        row("GUT BIAS", format!("{:+.3}", state.traits[organism::TRAIT_GUT_BIAS]),
            "THIS ANIMAL'S OWN DIET, -1 PLANT MATTER TO +1 FLESH. IT IS INHERITED WITH JITTER, SO IT IS NOT THE SPECIES VALUE ON THE ANTS PAGE -- COMPARE THE TWO AND YOU ARE LOOKING AT ONE GENERATION OF DRIFT.");
        row("BIRTH GRANT", format!("{:+.3}", state.traits[organism::TRAIT_BIRTH_GRANT]),
            "HOW MUCH THIS ONE WOULD HAND A NEWBORN, AS ITS OWN INHERITED VALUE RATHER THAN THE SPECIES'.");
        row("SINCE NEST", state.since_nest.to_string(),
            "TICKS SINCE IT LAST TOUCHED THE NEST. IT CLIMBS WHILE A FORAGER IS OUT AND RESETS WHEN IT GETS HOME, SO A NUMBER THAT ONLY EVER CLIMBS IS AN ANT THAT IS LOST.");
        row("CARRYING", match &state.carrying {
                Some(c) => world.materials.get(c.material).display.to_uppercase(),
                None => "NOTHING".into(),
            },
            "WHAT IT HAS IN ITS JAWS. AN ANT ON ITS WAY HOME IS CARRYING SOMETHING; ONE ON ITS WAY OUT IS NOT.");
        row("BODY", state.cells.len().to_string(),
            "HOW MANY CELLS THIS ANIMAL IS. EVERY PER-CELL COST ON THE ANTS PAGE IS MULTIPLIED BY THIS.");
        return rows;
    }

    row("SHOOT", state.shoot_cells.to_string(),
        "HOW MUCH SHOOT IT HAS GROWN. THIS IS THE NUMBER THE PLANT PAGE'S SEED MATURITY IS COMPARED AGAINST -- BELOW THAT FENCE THIS PLANT CANNOT SET A SEED AT ALL, HOWEVER MUCH ENERGY IT HAS.");
    row("ROOT", state.root_cells.to_string(),
        "HOW MUCH ROOT IT HAS. AGAINST THE SHOOT COUNT IT IS THE ROOT-TO-SHOOT BALANCE, WHICH IS WHAT DECIDES WHETHER IT DIES OF THIRST OR OF SHADE.");
    row("SEEDS SET", state.seeds_set.to_string(),
        "SEEDS THIS INDIVIDUAL HAS SET IN ITS LIFE. IT IS ITS FITNESS, IN THE ONLY SENSE THE BOX MEASURES.");
    row("WATER", format!("{:.2}", state.water_status),
        "HOW WELL WATERED IT IS: 1.00 IS SATISFIED AND 0.00 IS TAKING UP NOTHING IT ASKED FOR. A PLANT SITTING LOW HERE IS A PLANT WHOSE ROOTS CANNOT REACH.");
    row("SENESCENT", if state.senescent { "YES -- ROTTING".into() } else { "NO".into() },
        "WHETHER IT IS DEAD AND ON ITS WAY OUT. A CULLED PLANT SAYS YES AND KEEPS ITS CELLS UNTIL THEY ROT, WHICH IS WHY A CULL IS GRADED RATHER THAN A DELETION.");
    row("ALLELES", state.alleles.iter().map(|a| a.to_string()).collect::<Vec<_>>().join("/"),
        "THE SIX DISCRETE MORPHOLOGY GENES, IN ORDER: LEAF ECONOMY, BRANCH ANGLE, INTERNODE, SYMPODIAL, TROPISM, WOOD DENSITY. THEY ARE CATEGORICAL, NOT SCALAR -- TWO PLANTS THAT DIFFER HERE ARE DIFFERENT SHAPES, NOT THE SAME SHAPE AT DIFFERENT SIZES.");

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
        rows.push((
            (*label).to_string(),
            format!("X{factor:.2}"),
            format!("THIS INDIVIDUAL'S OWN MULTIPLIER ON ITS SPECIES' {label}, DRAWN WHEN IT GERMINATED AND CARRIED FOR LIFE. 1.00 IS THE SPECIES VALUE; ITS SPECIES ALLOWS UP TO {:.0}% EITHER WAY. THIS IS WHY TWO SEEDS OF ONE SPECIES DO NOT GROW INTO THE SAME PLANT.", width * 100.0),
        ));
    }
    rows
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
    #[test]
    fn field_text_reads_the_span_the_edit_would_replace() {
        assert_eq!(field_text("(dig_force: 0.85,)", "dig_force").as_deref(), Some("0.85"));
        assert_eq!(field_text("(traits: (0.0, -0.2),)", "traits").as_deref(), Some("(0.0"));
        assert!(field_text("(traits: (0.0, -0.2),)", "traits").unwrap().parse::<f64>().is_err());
        assert_eq!(field_text("(density: 1.3 // heavy\n)", "density").as_deref(), Some("1.3 // heavy"));
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
