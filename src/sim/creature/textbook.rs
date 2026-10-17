//! **The textbook ant: a third way for an ant to decide, from cited rules
//! only.** Built beside today's ant and the needs walk, and off until a
//! harness sets `World::textbook`. The design of record is
//! `/mnt/project-files/emergence-sandbox/textbook-ant/design-textbook-ant-2026-10-15.md`
//! (rev 2, after an adversarial review beside it), and the owner's calls
//! were "Design doc first", then "Build, pillars first" (2026-10-15).
//!
//! **What it is for.** The textbook tests (Khuong pillars, the double
//! bridge, sorting) failed on both brained ants, and in the pillar box the
//! game ant's keep rules held 99.6% of its carriers before the drop roll was
//! reached (`/mnt/project-files/emergence-sandbox/pillars/README.md`). A
//! reference ant with only the textbook's rules, in the same engine and the
//! same body, says which failures are the engine's and which are our rules':
//! where it passes and the game ant fails, adding one game rule to it at a
//! time names the rule that blocks.
//!
//! **Slice A, soil (the pillar test), is all that is built.** Three rules,
//! each a named part with its parameter in [`TextbookCfg`]:
//! - **walk**: a correlated random walk over the body's usable headings,
//!   the same walk the level-1 walkers take (`examples/pillarbox`): keep the
//!   heading, turn one point either way with `turn`, and take the first
//!   usable heading nearest it. Modelling standard; not checked.
//! - **pick** (Khuong et al. 2016, PNAS 113:1303, *Lasius niger*, SI): an
//!   empty ant cuts a cell from the layer beneath its head, floor or built
//!   soil alike, with `1 - exp(-eta_p,1 dt / n)`, `n` the ground round its
//!   head: lone pellets are taken and piles kept, which remodels the build
//!   ([`cut_cell`], gated by [`jaw_can_cut`] only).
//! - **put down** (the paper's Eq. 1): a laden ant takes, at random, an open
//!   cell beside its head that the engine will hold and that shuts no way
//!   ([`closes_a_way`]), and puts the pellet there with
//!   `1 - exp(-(eta_d,0 + b_d n) dt exp(-tau_m / tau))`, `tau_m` the time
//!   since the latest deposit round it. A side deposit (no ground beneath,
//!   held only by `update::spoil_corbel`) is further weighted by the body
//!   template `F(h)`, `h` the open height beneath it: the paper's ants put
//!   material on a pillar's side only from about a body length up, which is
//!   what turns pillars into caps, arches and a roof. **Where it can stand
//!   is engine-driven**: without `SPOIL_CORBEL` a pellet needs ground
//!   straight beneath (`needs_footing`) and no cap can stand at all.
//!
//! **What the tick still does for it, and what it replaces** (the design's
//! §4 table). The brain still runs, as it does under the needs walk, so the
//! synapse tax is paid like for like and the speculation pre-pass is
//! untouched; its outputs are ignored except where [`outputs_for`] zeroes
//! them (trail emission, the launch, laying and provisioning: births wait,
//! and slice A lays no trail). `act`, the move roll and the walk are
//! replaced; the nest-scent blend, metabolism, digestion and death are not.
//! Falling is checked every decision, as under the chooser.
//!
//! **Its draws are its own** (`RNG_SLOT_TEXTBOOK`), so nothing it does
//! moves another ant's stream, and with `World::textbook` unset no line
//! here runs.
use super::*;

/// The textbook ant's own random stream; see [`super::RNG_SLOT_MOVE`].
const RNG_SLOT_TEXTBOOK: u64 = 17;

/// **Slice A's parameters: Khuong et al. 2016's fitted model** (PNAS
/// 113:1303, Eq. 1 and SI; *Lasius niger*), each the authored constant a gene
/// will later scale (design §6). The paper's rates are per second for an ant
/// that steps half a millimetre a second; ours steps one cell, a quarter of
/// its 4 mm body (the body is two cells), so one decision stands for `dt`
/// = 4 s. **That scaling is ours**, as is everything marked so below.
///
/// **Rev 1 of this slice is superseded** (2026-10-15): it took level 1's
/// walker rule (`p0 + (1 - p0) m/(m+K)`, p0 0.002, no pick-up from built
/// soil, no body template) and built one-wide needles 15-40 body lengths
/// tall, which its census scored as pillars. Checked against the paper the
/// owner asked for, the pillars are a body length wide, stop rising at about
/// a body length and cap into arches (`pillars/README.md`, "Checked against
/// the paper").
#[derive(Clone, Copy, Debug)]
pub struct TextbookCfg {
    /// The chance a decision is a step rather than a pause (ours).
    pub p_move: f32,
    /// The chance a step first turns one point, either way at even odds
    /// (ours: the level-1 walkers' one in eight).
    pub turn: f32,
    /// Seconds one decision stands for (ours: 4, see above).
    pub dt: f32,
    /// Deposition rate with nothing round the ant, per second: eta_d,0 0.025.
    pub drop_base: f32,
    /// Deposition rate added per pellet round the ant, per second: b_d 0.11.
    pub drop_per: f32,
    /// Pick-up rate over the pellets round the ant, per second: eta_p,1 0.029.
    pub pick: f32,
    /// The pheromone's mean lifetime, in frames (the paper's 1/eta_m: 20 min,
    /// 1,800 frames at six frames a decision and `dt` 4 s). 0 is no pheromone
    /// (the paper: "no structure can be built"); infinite never fades.
    pub tau: f32,
    /// The body-size template, in cells of open height under a side
    /// deposit: the paper's lowest side deposit is 4.02 mm against a 4.1 mm
    /// body, so a side deposit is half-weighted at `template` cells (one body
    /// length, 2) and full one cell higher (ours: a linear ramp for the
    /// fitted CDF of Fig. 3C).
    pub template: f32,
    /// The material the floor is made of (picked from, as built soil is).
    pub source: material::MaterialId,
    /// Whether bedrock counts in `n` (ours, off by default). The paper's `n`
    /// is "the number of particles present in V26" -- building material --
    /// and its experiments counted "previously deposited pellets". Rev 2
    /// counted all ground, so an ant on the box's bare stone floor or its
    /// walls read three pellets where there were none, and dropped there
    /// at four times the base rate: the bed's own floor pulled the build
    /// wide. Off counts only what the ant could pick (the source and spoil).
    pub n_rock: bool,
    /// Where an empty ant picks from (ours, on by default): any loose pellet
    /// round the head with open sky above it, or, off, only the three cells
    /// under the head -- the paper's "bottom layer of V26" read literally.
    /// Measured on the bare rules in a side view (`pillars/bare-model`, cell
    /// half a body, 47 ants over 388 mm, 20 min pheromone, 3 seeds, 8 days):
    /// picking round the head keeps a field of ~20 pillars 0.6-0.8 body wide
    /// and 2.2-3.9 body lengths apart (the paper's 2.4) for all eight days;
    /// picking only from beneath leaves pillar flanks uneaten, builds 15-18
    /// heaps 2.4-3 bodies wide and 4-5 apart, and freezes by 48 h with every
    /// carrier laden and no fresh mark left to drop by.
    pub pick_around: bool,
    /// Whether the base deposition rate applies without scent (ours, on by
    /// default). Eq. 1 read literally, `(eta_d,0 + b_d n) exp(-tau_m eta_m)`,
    /// has no drops at all where no deposit is fresh. With no scent at all
    /// every ant then picks one pellet and carries it to the end (pillar box,
    /// tau 0: 80 picks, 0 drops), and a carrier far from any fresh deposit is
    /// stuck the same way. The paper's own no-scent result is that "the
    /// material ... is just remodeled", which needs drops. On, the scent
    /// scales the pile term only: `eta_d,0 + b_d n exp(-tau_m eta_m)`.
    pub base_always: bool,
    /// Whether a pellet may be laid against the box (ours, on by default):
    /// off, a site with stone (ground that is not building material) beside
    /// or above it is no site -- stone beneath, the floor, still is. The
    /// paper's lattice ants "are randomly reoriented in a direction facing
    /// away from" its boundaries, and its arena is a Petri dish whose walls
    /// nobody reports being built on, so nothing there is laid against a
    /// wall. Ours walk the box's walls and lid, and under `base_always` and
    /// `SPOIL_CORBEL` they lined them: pillar box, 80 ants, tau 0, 520k
    /// frames, the floor's soil ended as a ramp 76 cells up the left wall.
    pub build_on_box: bool,
    /// **Slice B, the bridge: foraging on a trail** (`None` is slice A's
    /// soil verbs). See [`ForageCfg`].
    pub forage: Option<ForageCfg>,
    /// **Slice C, brood sorting** (`None` is slice A's or B's verbs). See
    /// [`SortCfg`]. Takes precedence over `forage`.
    pub sort: Option<SortCfg>,
    /// **Layer 1 of the rebuild: dig and forage in one run** (`None` keeps
    /// the slices apart). Read only together with `forage`. See [`NestCfg`].
    pub nest: Option<NestCfg>,
}

/// **Slice C's parameters: brood sorting by size** (Franks & Sendova-Franks
/// 1992, *Leptothorax unifasciatus*: one cluster, the smallest brood in the
/// centre, larger brood further out). An empty ant picks up an `item` cell
/// it touches with `1 - score`, and a laden one puts it down on an open cell
/// beside its head, with ground beneath, with `score`. Here `score` is Don &
/// Amos's annular placement score (arXiv nlin/0507045, after Wilson et al.
/// 2004's robots): the sum of `penalty` per item within `penalty_m * size`
/// and `bonus` per item within `bonus_m * size`, clamped to 0..1. A bigger
/// item needs more clear room, so it lies further out.
///
/// An item's size is its cell's shade (0, 1, 2) looked up in `sizes`, times
/// `scale`. That is an inert stand-in: the game's brood has no size yet.
/// "Within" is centre-to-centre on the grid (theirs is edge to edge).
///
/// Chosen on the walker box (`examples/broodsort`, 16 seeds, `scale` 1.5,
/// `bonus_m` 3; the paper's own constants are `scale` 1, `bonus_m` 4).
/// **It does not yet pass the paper's bar** (2026-10-15): with the order
/// ratios required inside the published span (medium 1.2-2.0, large
/// 1.7-3.2) it is 3/16 top-down, 6/16 in side view, and 3/16 on this ant
/// (`examples/antsort`). It had been called 13/16 on a bar that only asked
/// whether large items lie further out than chance; the owner's read of
/// the picture -- medium mixed into the small heap, large strung out as
/// singles to the walls -- is what the stricter bar measures. Every item is
/// one cell, so a medium item's room (0.9 cell) rounds to none and a large
/// one's to a single empty cell; at `scale` 2 or more the large items find
/// no site and stay in the jaws.
#[derive(Clone, Copy, Debug)]
pub struct SortCfg {
    pub item: material::MaterialId,
    pub sizes: [f32; 3],
    pub scale: f32,
    pub penalty: f32,
    pub bonus: f32,
    pub penalty_m: f32,
    pub bonus_m: f32,
    /// The control: every score is this constant (`broodsort`'s `off` is
    /// 0.1), so ants carry brood at random with the same verbs.
    pub flat: Option<f32>,
}

impl SortCfg {
    /// Don & Amos's Experiment 1 at the walker box's grid scale.
    pub fn room(item: material::MaterialId) -> SortCfg {
        SortCfg {
            item,
            sizes: [0.5, 1.5, 3.0],
            scale: 1.5,
            penalty: -50.0,
            bonus: 0.1,
            penalty_m: 0.4,
            bonus_m: 3.0,
            flat: None,
        }
    }

    /// The placement score for an item of `shade` at `(x, y)`, not counting
    /// `skip` (the item itself, when it is being picked up).
    pub fn score(&self, world: &World, shade: u8, (x, y): (i32, i32), skip: (i32, i32)) -> f32 {
        if let Some(f) = self.flat {
            return f;
        }
        let s = self.sizes[usize::from(shade.min(2))] * self.scale;
        let (near, far) = (self.penalty_m * s, self.bonus_m * s);
        let r = far.ceil() as i32;
        let mut score = 0.0f32;
        for dy in -r..=r {
            for dx in -r..=r {
                let c = (x + dx, y + dy);
                if (dx, dy) == (0, 0) || c == skip || !world.in_bounds(c.0, c.1) {
                    continue;
                }
                let cell = world.get(c.0, c.1);
                if cell.material != self.item || cell.organism_id() != 0 {
                    continue;
                }
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                if d <= near {
                    score += self.penalty;
                } else if d <= far {
                    score += self.bonus;
                }
            }
        }
        score.clamp(0.0, 1.0)
    }
}

/// **Slice B's parameters: a forager on a food trail** (design rev 2,
/// slice B; the double bridge of Goss et al. 1989 and Deneubourg et al.
/// 1990, *Linepithema humile*). An empty ant picks up a cell of `food` it
/// touches; a laden one puts it down within `home_r` of `home`. Each step
/// is a heading drawn in proportion to
/// `persistence x exp(vector x cos(heading, target)) x (k + C)^n`:
/// - **persistence** by how far the heading turns from the body's facing
///   (ours, the walkers' table in `examples/bridgebox.rs`);
/// - **the vector**: the home vector for a laden ant (Mueller & Wehner
///   1988, error 0, a free positive control) and the feeder vector for an
///   empty one (path integration's memory of the food site, Collett et
///   al.). Both point at a harness-given site: a labelled stand-in for a
///   remembered route, not a sensed one;
/// - **the trail**: Deneubourg's choice `(k + C)^n` on the food trail
///   (channel B) one cell ahead, `C` and `k` in units of `SCALE`: Fitted
///   on these bridges at k = 20 passages and n = 2, so `k = 20 x lay`.
///   Applying a fork rule to every heading at every step is a modelling
///   choice.
///
/// A step lays `lay` units of B where it lands: always for the Argentine
/// ant of the bridge papers (`lay_out`), only when laden otherwise (Beckers
/// et al. 1992, *L. niger*). The engine plane holds 255 units a cell, so a
/// unit is one `SCALE`: at the shipped `DEPOSIT` (40 units) a cell is full
/// after six passages and `(k + C)^2` can never exceed 1.7x. A trail that
/// hits the cap reads the same on both branches: `examples/antbridge` runs
/// `lay=0.1 k=2` with almost no fade (the papers ignore evaporation), which
/// keeps a busy branch under it.
///
/// **Measured on Goss's bridge** (`antbridge`, 2026-10-15, 2-cell
/// tunnels, 8 seeds a cell, both mirrors): equal branches, scent off,
/// 0.48-0.53; scent on, one branch over 80% on 5/8 (60 ants) and 6/8 (150),
/// either branch winning. r = 2: the short branch is the majority on 13/16
/// and 14/16, over 80% on 9/16 and 11/16 (Goss: 14/14 and 11/14). Short
/// branch added later: the colony stays on the long one 16/16 at both
/// sizes (Goss: 16/18). In 3-cell tunnels a colony settles on whichever
/// surface of the neck carries the trail -- the floor, approached first
/// from the chamber below -- and the floor leads to the lower branch: 0/8
/// with the short branch above.
#[derive(Clone, Copy, Debug)]
pub struct ForageCfg {
    pub food: material::MaterialId,
    pub home: (i32, i32),
    pub home_r: i32,
    /// Where the home vector points (a laden ant), and the feeder vector
    /// (an empty one). Separate from `home`, the drop zone, so a harness can
    /// aim both at the bridge's own level: aimed at a chamber floor below
    /// the branches, both vectors favoured the lower branch at every fork.
    pub aim_home: (i32, i32),
    pub feeder: (i32, i32),
    /// The home vector's weight (a laden ant), and the feeder vector's (an
    /// empty one). Separate because food can move: a remembered feeder
    /// that is now empty pins an empty ant in front of it, where the home
    /// vector is never wrong (Dussutour et al. 2009's moved food,
    /// `antbridge feeders=2`).
    pub vector: f32,
    pub vector_out: f32,
    /// Both vectors pull along the row only (towards the target's column,
    /// never up or down). With two feeder rooms one above the other any
    /// aimed height sits nearer one room's food: aimed between them the
    /// feeder vector pulled ants onto the upper room's crumbs and off the
    /// lower's, and the colony settled on the lower room's branch on 0 of 4
    /// runs; and the home vector, aimed above the lower room's door, pinned
    /// laden ants in the corner over it (38 laden ants in the room at once,
    /// trail laid both ways) (`antbridge feeders=2`, 2026-10-16).
    pub out_flat: bool,
    pub k: f32,
    pub n: i32,
    pub lay: f32,
    pub lay_out: bool,
    /// With `lay_out`, an empty ant lays `out_frac` of `lay`. Dussutour et
    /// al. 2009 (*Pheidole megacephala*) found two trails: a weak, long-lived
    /// one laid by every ant and a strong, short-lived one laid by fed ants.
    /// One plane holds both here, as a weaker outbound deposit. 1: the
    /// Argentine ant's equal deposits both ways.
    pub out_frac: f32,
    /// Whether the trail is read at all (`false` is the muted arm: laid,
    /// never followed).
    pub read: bool,
    /// **Giving up**: an empty ant that has taken this many decisions since
    /// it left home without finding food turns for home on the home vector,
    /// empty, and starts again from there (0: never). The dead-end rule of
    /// Robinson et al. 2008's trail model (*J. Theor. Biol.* 255:250, an
    /// ant at an unrewarded branch's end turns back); a remembered feeder
    /// that is now empty otherwise pins an ant in front of it for good
    /// (`antbridge feeders=2`, 2026-10-16: the empty room filled with ants).
    pub give_up: u32,
    /// Where giving up counts: only decisions within `search_r` of `feeder`
    /// (0: everywhere since home). Counting the whole trip made ants on a
    /// long, busy branch give up on food that was there, and their no-entry
    /// mark then drove the colony off it (Goss's late-opened short branch
    /// taken 8/8, 2026-10-16); an ant searches where it remembers the food
    /// and turns back from there (Wehner's desert ants; Robinson's dead end).
    pub search_r: i32,
    /// ...and only while no loose `food` lies within `sense` cells of the
    /// head (0: food is not looked for). An ant held back from food by the
    /// crowd round it has not found the site empty: on Goss's bridge, with
    /// the feeder always stocked, 1,959 marking steps were laid in one run
    /// by ants that gave up in the crowd, and the short branch's win fell
    /// from 80% of traffic on 5/8 runs to 1/8 (2026-10-16). A modelling
    /// choice (a forager smells food it cannot yet reach).
    pub sense: i32,
    /// **Crowding lays less** (Czaczkes, Grueter & Ratnieks 2013, *J. R.
    /// Soc. Interface* 10:20121009, *Lasius niger*): after 10 encounters over
    /// 20 cm a forager was 45% less likely to deposit, and from the least to
    /// the most crowded trail 5.6 times fewer ants deposited. Czaczkes 2014
    /// (*J. Theor. Biol.* 360:172) found this is what lets a colony refocus
    /// when food quality switches (97% of simulated colonies, against none
    /// without it). A step lays with chance `max(1 / 5.6, exp(-crowd x
    /// met))`, `met` the other ants beside the head summed over recent steps
    /// (a running sum fading by `1 / CROWD_WINDOW` a step). `crowd = 0.06`
    /// gives the paper's 45% at 10. 0: off, every step lays.
    pub crowd: f32,
    /// **The no-entry mark** (Robinson, Jackson, Holcombe & Ratnieks 2005,
    /// *Nature* 438:442, *Monomorium pharaonis*): a repellent laid on an
    /// unrewarding branch, read by others as "not this way". An ant that has
    /// given up ([`ForageCfg::give_up`]) lays `no_entry` deposits of it on
    /// each step home; a step's weight is multiplied by `exp(-repel x R)`,
    /// `R` the mark ahead in deposits. Robinson et al. 2009 (*Insectes Soc.*
    /// 56:256) found its effect starts about twice the trail's and lasts
    /// about 2.4 times as long (78 min against 33). **It rides channel A**,
    /// which nothing else lays in `antbridge`'s box: a labelled stand-in. A
    /// game version would need a plane of its own. 0: off.
    pub no_entry: f32,
    pub repel: f32,
    /// Lay the mark only at a junction, on the cells the body has just come
    /// through (Robinson et al. 2005 found it concentrated just past the
    /// bifurcation, on the unrewarding branch). Laid on every step home, a
    /// mark strong enough to turn ants also lay on the stem both branches
    /// share and shut the nest in: traffic fell to 250-500 crossings
    /// (`repel=10`, 2026-10-16). A junction is a head with three or more
    /// separate runs of open cells on the ring two cells out.
    pub mark_fork: bool,
    /// Lay the mark on every step home **until** the first junction, then
    /// stop for the trip: the whole unrewarding branch is marked, the stem
    /// is not. At the junction alone ([`ForageCfg::mark_fork`]) the colony
    /// locked onto the dead branch at low traffic (switch 3/8 and 1/8);
    /// laid all the way home, a colony of 150 ended the run with every ant
    /// in the nest (2026-10-16, laid both ways, seed 1 at 60,000 frames).
    pub mark_to_fork: bool,
    /// **The exploration scent** (Dussutour, Nicolis, Shephard, Beekman &
    /// Sumpter 2009, *J. Exp. Biol.* 212:2337, *Pheidole megacephala*): a
    /// second, longer-lived trail laid by **every** ant (λE = 0.04/min,
    /// against the food trail's λF = 0.07), weakly attractive (f(E) = 1 +
    /// E), and laid less where the food trail is strong ("exploitation
    /// locally suppresses exploration", their Eqn 10: E is made at
    /// `a(1 - cF)`). It leaves the unfed branch marked and the fed one
    /// bare, which is what the paper says makes a colony let go of moved
    /// food quickly and come back faster still (20.8 then 12.2 min).
    /// Their Discussion: this species "might not need to use a negative
    /// pheromone to mark an empty branch". A step lays `explore` deposits
    /// on channel A with chance `max(0, 1 - explore_c x F / k)`, `F` the
    /// food trail underfoot; a step's weight is multiplied by `1 + E /
    /// explore_k`, `E` the scent ahead. **It rides channel A**, so it
    /// cannot run with [`ForageCfg::no_entry`]. 0: off.
    pub explore: f32,
    pub explore_c: f32,
    /// 0: the exploration scent is not read at all, only laid.
    pub explore_k: f32,
    /// **A marked branch is not marked again**: the lay chance is also
    /// multiplied by `max(0, 1 - E / explore_max)`, `E` the exploration
    /// scent underfoot in deposits. In the paper's Eqn 10 a branch makes
    /// exploration scent at a fixed rate `a(1 - cF)`, however many ants
    /// walk it; laid per step it is a second trail, with its own feedback.
    /// Uncapped, it locked a colony onto a branch the food had left: after
    /// the food went back to the first branch, the second's exploration
    /// scent grew to 10-15 deposits from the traffic still on it and held
    /// the colony near half and half for 40 min (`antbridge`, 2026-10-16).
    /// A modelling choice that stands in for the paper's fixed rate, not a
    /// measured behaviour. 0: no cap.
    pub explore_max: f32,
    /// **Exploration speeds recruitment** (the paper's other coupling, Eqn
    /// 10: food trail is made at `phi_F E_i`, in proportion to the
    /// exploration scent on that branch; they cite the faster recruitment
    /// to a feeder at the end of an explored branch, their Fig. 4). A fed
    /// ant's deposit is multiplied by `1 + explore_lay x E`, `E` the
    /// exploration scent underfoot in deposits. 0: off.
    pub explore_lay: f32,
    /// Read the food trail as `k^n + C^n` (Dussutour 2009's choice, their
    /// Eqns 10-11, after Beckers et al. 1992) instead of Deneubourg's `(k +
    /// C)^n`. The two agree where `C` is far above `k`; below it the first
    /// is flat (a faint trail reads as none), the second is not.
    pub kn: bool,
}

/// **Layer 1 of the rebuild: one colony that digs and forages** (owner,
/// 2026-10-16: "rebuild ant", in layers, in today's world; the design is
/// `/mnt/project-files/emergence-sandbox/textbook-ant/layer1-dig-and-forage-2026-10-16.md`).
/// With `nest` and `forage` both set, [`act`] routes each decision by what
/// the ant carries and its role:
/// - **carrying food**: home on the home vector, laying the food trail, and
///   put it down at home (slice B's verbs);
/// - **carrying soil**: under a roof, walk up and keep it; out from under,
///   put it down by Khuong's Eq. 1 (slice A's verb), so spoil heaps outside;
/// - **an empty forager**: walk to the food it last found (its own memory,
///   not a feeder the harness names), or explore on the trail, and take
///   food it touches;
/// - **an empty digger**: cut soil touching its head, with a chance that
///   rises with the nestmates round it ([`NestCfg::dig_crowd`]).
///
/// Roles are fixed by ant id ([`NestCfg::forager_frac`]): a labelled stand-in
/// for layer 2's hunger.
#[derive(Clone, Copy, Debug)]
pub struct NestCfg {
    /// The share of ants that forage; the rest dig. Chosen by ant id, so a
    /// role never changes (ours: a stand-in until hunger decides it).
    pub forager_frac: f32,
    /// A digger's chance to cut a soil cell touching its head, per decision.
    pub dig: f32,
    /// **Digging needs company** (Rasse & Deneubourg 2001, *Insectes Soc.*
    /// 48:9, *Lasius niger*: excavation rate rises with the ants at work and
    /// nest volume settles in proportion to the colony; Buhl et al. 2004,
    /// *Naturwissenschaften* 91:602, *Messor sancta*): the dig chance is
    /// multiplied by `min(1, m / dig_crowd)`, `m` the other ants within two
    /// cells of the head. Digging then concentrates where ants are and stops
    /// where a grown nest spreads them thin. 0: off, company is not read
    /// (the control).
    pub dig_crowd: f32,
    /// **Under a roof**: ground within this many cells straight above the
    /// head (ours: a stand-in for the dark of the nest).
    pub roof: i32,
    /// How hard an ant that must leave the nest (a soil carrier, an empty
    /// forager) pulls upward while under a roof, as `ForageCfg::vector` does.
    pub up: f32,
    /// How hard an empty digger under a roof pulls downward (ours: the nest
    /// is dug down).
    pub down: f32,
    /// How hard a digger out on the surface pulls towards home.
    pub home: f32,
    /// **The door stays clear**: no spoil is put down within this many
    /// columns of home (ours: real spoil is carried a little way from the
    /// entrance, which is what rings a door with a crater; put down at the
    /// rim, diggers took it straight back, 2026-10-16 first run).
    pub clear_r: i32,
    /// **Backing out of a jam** (Aguilar et al. 2018, *Science* 361:672,
    /// fire ants: diggers that meet a clog at the face reverse rather than
    /// push in, and most of the digging is done by a few). An empty digger
    /// with at least `jam` other ants in the six cells ahead of it pulls up
    /// and out instead of down. 0: off.
    pub jam: u32,
    /// **The store is not forage**: a forager takes no food within this
    /// many cells of home (ours: in the first run foragers took food back
    /// off the pile they had just delivered, 2,024 picks against 169 cells
    /// taken from the heap).
    pub store_r: i32,
    /// Put spoil down as loose soil, which pours into a cone, instead of
    /// spoil, which stands wherever it has ground beneath (ours). As spoil,
    /// carriers stacked it into needles up to 68 cells tall (2026-10-16).
    pub loose: bool,
    /// Dig only the cell straight ahead (ours), not one point either side.
    pub straight: bool,
    /// Dig only from inside ([`inside`]): never cut the open surface.
    pub inside_only: bool,
    /// **Most diggers wait** (Aguilar et al. 2018, *Science* 361:672: in
    /// fire-ant excavation a few workers do most of the digging and the
    /// rest hold back, which keeps the tunnels from clogging): an empty
    /// digger outside the nest with at least `wait` other ants in the six
    /// cells ahead of it does not step, with chance `wait_p` a decision.
    /// Always (`wait_p` 1), a colony planted in a clump waited on itself
    /// for good: 6 cuts in 40k frames (2026-10-17). 0: off.
    pub wait: u32,
    pub wait_p: f32,
    /// **Digging feeds on itself** (Toffin et al. 2009, *PNAS* 106:18616,
    /// *Messor sancta*: excavation concentrates at the tips of growing
    /// tunnels, where workers are; Buhl et al. 2004): a cut made within
    /// `dig_fresh` frames is a fresh cut. A digger's chance to cut a face
    /// with no fresh cut within two cells is multiplied by `dig_new` (1:
    /// off), rising to the full chance with two or more, and an empty
    /// digger inside pulls with `dig_pull` towards the fresh cuts within
    /// eight cells.
    pub dig_new: f32,
    pub dig_fresh: u64,
    pub dig_pull: f32,
}

/// Whether a textbook ant is a forager under [`NestCfg::forager_frac`]: a
/// hash of its id, so the split holds for the run and is the same across
/// arms that plant the same ants.
fn forager(organism: OrganismId, nc: &NestCfg) -> bool {
    let h = (organism as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40;
    (h as f32 / (1u64 << 24) as f32) < nc.forager_frac
}

/// Whether `(x, y)` is under a roof: ground within `roof` cells straight
/// above ([`NestCfg::roof`]).
fn roofed(world: &World, roof: i32, x: i32, y: i32) -> bool {
    (1..=roof).any(|d| world.in_bounds(x, y - d) && ground(world, x, y - d))
}

/// **Inside**: under a roof, or with walls close on both sides (within two
/// cells). Looking only straight up, an open shaft read as outside and the
/// colony's spoil was put down in it (world review's prosecution,
/// 2026-10-16).
fn inside(world: &World, roof: i32, x: i32, y: i32) -> bool {
    roofed(world, roof, x, y) || walled(world, 2, x, y)
}

/// Fresh cuts ([`NestCfg::dig_fresh`]) within `r` cells of `(x, y)`, and
/// their summed offset from it.
fn fresh_cuts(world: &World, nc: &NestCfg, (x, y): (i32, i32), r: i32) -> (u32, (i32, i32)) {
    let Some(t) = world.textbook.as_ref() else {
        return (0, (0, 0));
    };
    let mut n = 0;
    let mut sum = (0, 0);
    for dy in -r..=r {
        for dx in -r..=r {
            if t.cuts
                .get(&(x + dx, y + dy))
                .is_some_and(|&at| world.frame.saturating_sub(at) < nc.dig_fresh)
            {
                n += 1;
                sum = (sum.0 + dx, sum.1 + dy);
            }
        }
    }
    (n, sum)
}

/// Other ants in the six cells ahead of a head facing `facing` (one and two
/// cells out, straight and one point either way), by distinct id.
fn ants_ahead(world: &World, organism: OrganismId, (x, y): (i32, i32), facing: usize) -> u32 {
    let mut seen: Vec<OrganismId> = [facing, (facing + 1) % 8, (facing + 7) % 8]
        .iter()
        .flat_map(|&d| (1..=2).map(move |k| (x + DIRS[d].0 * k, y + DIRS[d].1 * k)))
        .filter(|&(px, py)| world.in_bounds(px, py))
        .map(|(px, py)| world.get(px, py).organism_id())
        .filter(|&o| o != 0 && o != organism)
        .collect();
    seen.sort_unstable();
    seen.dedup();
    seen.len() as u32
}

/// Whether `(x, y)` has walls on both sides: ground within `reach` cells
/// on the head's row, left and right.
fn walled(world: &World, reach: i32, x: i32, y: i32) -> bool {
    let side = |s: i32| (1..=reach).any(|d| world.in_bounds(x + s * d, y) && ground(world, x + s * d, y));
    side(-1) && side(1)
}

/// Whether `(x, y)` is down a hole: on both sides, ground at or above the
/// head's row within `reach` cells across and up. A carrier there keeps its
/// pellet and climbs. Put down at a crater's sides, the spoil fell back
/// into the pit it came from (first runs, 2026-10-16: 4,600 cuts, ~130
/// cells dug, the crater's size steady from 20k frames on).
fn in_hole(world: &World, reach: i32, x: i32, y: i32) -> bool {
    let side = |s: i32| {
        (1..=reach)
            .any(|d| (0..=reach).any(|up| world.in_bounds(x + s * d, y - up) && ground(world, x + s * d, y - up)))
    };
    side(-1) && side(1)
}

/// Steps over which [`ForageCfg::crowd`] counts encounters: about the
/// paper's 20 cm for a 3-4 mm ant at one cell a body length, a labelled
/// guess at our grain.
pub const CROWD_WINDOW: f32 = 60.0;

/// What the rules did, for a harness's census. Nothing reads these back.
#[derive(Clone, Copy, Debug, Default)]
pub struct TextbookCounts {
    /// Decisions taken by a textbook ant.
    pub decisions: u64,
    /// Cells cut from the source.
    pub picks: u64,
    /// Laden decisions with no site beside the head.
    pub no_site: u64,
    /// Put-down rolls taken, and won.
    pub rolls: u64,
    pub drops: u64,
    /// The pheromone factor `exp(-tau_m / tau)` at the rolls, summed.
    pub mark_sum: f64,
    /// Rolls at a side site (a deposit with no ground beneath, held by
    /// `SPOIL_CORBEL`), and how many of those were won.
    pub side_rolls: u64,
    pub side_drops: u64,
    /// Steps taken, and decisions with no usable heading.
    pub steps: u64,
    pub stuck: u64,
    /// Slice B: food picked up at the feeder, and put down at home.
    pub food_picks: u64,
    pub delivered: u64,
    /// [`ForageCfg::crowd`]: laying steps rolled, and their encounter sums.
    pub crowd_rolls: u64,
    pub met_sum: f64,
    /// [`ForageCfg::no_entry`]: steps that laid the mark.
    pub no_entry_steps: u64,
    /// [`ForageCfg::explore`]: steps that laid the exploration scent.
    pub explore_steps: u64,
    /// [`NestCfg`]: soil cells cut by diggers, and dig rolls taken.
    pub digs: u64,
    pub dig_rolls: u64,
    /// [`NestCfg::wait`]: decisions a digger held back from a crowd.
    pub waits: u64,
}

/// One row of [`Textbook::nest_log`]: `(frame, ant, x, y, material, put)`.
pub type NestLogRow = (u64, OrganismId, i32, i32, material::MaterialId, bool);

/// **`World::textbook`**: which species decides by it, its parameters and
/// its counts. Set by a harness at a frame, never from the environment.
#[derive(Clone, Debug)]
pub struct Textbook {
    pub species: crate::sim::organism::SpeciesId,
    pub cfg: TextbookCfg,
    pub counts: TextbookCounts,
    /// Slice B's per-ant trip: decisions since home while empty, whether
    /// the ant has given up ([`ForageCfg::give_up`]), and whether it has
    /// marked a junction on the way home ([`ForageCfg::mark_fork`]). Read and
    /// written by key only, never iterated, so its order cannot reach a
    /// result.
    pub trips: std::collections::HashMap<OrganismId, (u32, bool, bool)>,
    /// [`ForageCfg::crowd`]'s running encounter sum per ant, and the ants
    /// beside its head last step (an encounter is an ant newly beside it, so
    /// a file of ants walking nose to tail is not one encounter a step), by
    /// key only.
    pub met: std::collections::HashMap<OrganismId, (f32, Vec<OrganismId>)>,
    /// [`NestCfg`]: where each forager last took food, its own memory (the
    /// feeder vector Wehner's desert ants carry), forgotten when it gives
    /// up there. By key only.
    pub food_at: std::collections::HashMap<OrganismId, (i32, i32)>,
    /// [`NestCfg`]'s cuts and put-downs, for a harness's trace: `(frame,
    /// ant, x, y, material, put)`, `put` false for a cut. `None` records
    /// nothing; read by nothing in the simulation.
    pub nest_log: Option<Vec<NestLogRow>>,
    /// [`NestCfg`]'s per-decision trace of the ants in `trace_ids`: one
    /// line each, what the digger saw and did. Read by nothing in the
    /// simulation.
    pub trace_ids: Vec<OrganismId>,
    pub trace: Vec<String>,
    /// [`NestCfg::dig_new`]: the frame of each cell's latest cut, by key.
    pub cuts: crate::sim::fxhash::PosMap<u64>,
}

impl Textbook {
    pub fn new(species: crate::sim::organism::SpeciesId, cfg: TextbookCfg) -> Textbook {
        Textbook {
            species,
            cfg,
            counts: TextbookCounts::default(),
            trips: std::collections::HashMap::new(),
            met: std::collections::HashMap::new(),
            food_at: std::collections::HashMap::new(),
            nest_log: None,
            trace_ids: Vec::new(),
            trace: Vec::new(),
            cuts: Default::default(),
        }
    }
}

/// Whether `organism` decides by the textbook rules this tick.
pub(super) fn decides(world: &World, organism: OrganismId) -> bool {
    match (world.textbook.as_ref(), world.organism(organism)) {
        (Some(t), Some(s)) => s.species == t.species,
        _ => false,
    }
}

/// The brain's outputs as the rest of the tick will read them: the ones the
/// textbook ant replaces with nothing are zeroed. `EmitA`/`EmitB` (slice A
/// lays no trail), `Impulse` (it walks), and `Lay`/`Provision` (births are
/// the harness's to pause; a zero `Lay` holds them, `try_bud`).
pub(super) fn outputs_for(mut outputs: [f32; brain::BRAIN_OUTPUTS]) -> [f32; brain::BRAIN_OUTPUTS] {
    use brain::BrainOutput as O;
    for o in [O::EmitA, O::EmitB, O::Impulse, O::Lay, O::Provision] {
        outputs[o as usize] = 0.0;
    }
    outputs
}

/// The textbook ant's draw for this tick.
pub(super) fn draw_for(world: &World, organism: OrganismId) -> rng::Rng {
    rng::stream(world.seed, organism as u64, world.frame, RNG_SLOT_TEXTBOOK)
}

/// The chance this decision is a step.
pub(super) fn p_move(world: &World) -> f32 {
    world.textbook.as_ref().map_or(0.0, |t| t.cfg.p_move)
}

/// **The pheromone at `p`**: `exp(-tau_m / tau)`, `tau_m` the frames since
/// the latest deposit in the 3x3 round it whose cell still stands (not empty,
/// not an animal) -- Eq. 1's `exp(-tau_m * eta_m)`, read on the ant's
/// neighbourhood as the paper's `V26` is. 0 at `tau` 0 or with no deposit
/// round it; 1 for any deposit at `tau` infinite.
pub(crate) fn mark_round(world: &World, tau: f32, (x, y): (i32, i32)) -> f32 {
    if tau <= 0.0 || world.build_marks.is_empty() {
        return 0.0;
    }
    let mut latest: Option<u64> = None;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let c = (x + dx, y + dy);
            let Some(&f) = world.build_marks.get(&c) else { continue };
            let cell = world.get(c.0, c.1);
            if cell.material == material::EMPTY || cell.organism_id() != 0 {
                continue;
            }
            latest = Some(latest.map_or(f, |l| l.max(f)));
        }
    }
    match latest {
        None => 0.0,
        Some(_) if tau.is_infinite() => 1.0,
        Some(f) => (-(world.frame.saturating_sub(f) as f32) / tau).exp(),
    }
}

/// Ground a pellet can rest on or be held by: not empty, not an animal, a
/// `Powder` or `Solid` (`is_footing`'s test).
fn ground(world: &World, x: i32, y: i32) -> bool {
    is_footing(world, x, y)
}

/// The pellets round the head: building material in its eight neighbours
/// (the paper's `n`, over `V26`) -- the source and spoil, or all ground
/// under [`TextbookCfg::n_rock`].
fn pellets_round(world: &World, cfg: &TextbookCfg, (x, y): (i32, i32)) -> u32 {
    let spoil = world.materials.id_of("spoil");
    DIRS.iter()
        .filter(|&&(dx, dy)| {
            let (px, py) = (x + dx, y + dy);
            ground(world, px, py)
                && (cfg.n_rock || {
                    let m = world.get(px, py).material;
                    m == cfg.source || Some(m) == spoil
                })
        })
        .count() as u32
}

/// Open cells straight beneath `(x, y)` before ground, up to 8: the height a
/// side deposit would stand over.
fn open_below(world: &World, x: i32, y: i32) -> u32 {
    (1..=8).take_while(|&d| !ground(world, x, y + d)).count() as u32
}

/// **Where a pellet can go**: an open cell beside the head that shares a
/// face with ground (the paper's rule) and that the engine will hold --
/// ground straight beneath, or, under `SPOIL_CORBEL`, a side neighbour that
/// is ground with ground beneath it -- and that shuts no way
/// ([`closes_a_way`]). Returns the site and whether it is a side deposit.
fn sites(world: &World, cfg: &TextbookCfg, (x, y): (i32, i32)) -> Vec<((i32, i32), bool)> {
    let corbel = crate::sim::update::spoil_corbel();
    let spoil = world.materials.id_of("spoil");
    let stone = |px: i32, py: i32| {
        ground(world, px, py) && {
            let m = world.get(px, py).material;
            m != cfg.source && Some(m) != spoil
        }
    };
    DIRS.iter()
        .map(|&(dx, dy)| (x + dx, y + dy))
        .filter(|&(px, py)| world.in_bounds(px, py) && world.is_empty(px, py))
        .filter_map(|(px, py)| {
            let footed = ground(world, px, py + 1);
            let held = corbel
                && [-1, 1]
                    .iter()
                    .any(|&sx| ground(world, px + sx, py) && ground(world, px + sx, py + 1));
            (footed || held).then_some(((px, py), !footed))
        })
        .filter(|&((px, py), _)| cfg.build_on_box || !(stone(px - 1, py) || stone(px + 1, py) || stone(px, py - 1)))
        .filter(|&(p, _)| !closes_a_way(world, p))
        .collect()
}

/// **The verbs, in place of `act`: Khuong's Eq. 1.** An empty ant picks a
/// cell from the layer beneath it (the paper's "bottom layer of V26") with
/// `1 - exp(-eta_p,1 dt / n)`: lone pellets go, piles stay. A laden ant takes
/// a site at random and puts the pellet there with
/// `F(h) (1 - exp(-(eta_d,0 + b_d n) dt exp(-tau_m / tau)))`, `F` the body
/// template for a side deposit (1 on ground). Returns what the tick charges
/// for (a cut is a dig).
pub(super) fn act(world: &mut World, organism: OrganismId, def: &CreatureDef, draw: &mut rng::Rng) -> Did {
    let Some(cfg) = world.textbook.as_ref().map(|t| t.cfg) else {
        return Did::default();
    };
    let Some((head, laden)) = world.organism(organism).map(|s| (s.chain.first().copied(), s.spoil)) else {
        return Did::default();
    };
    let Some((x, y)) = head else { return Did::default() };
    count(world, |c| c.decisions += 1);
    if let Some(sc) = cfg.sort {
        sort_act(world, organism, &sc, (x, y), laden.map(|l| l.cell), draw);
        return Did::default();
    }
    if let (Some(nc), Some(fc)) = (cfg.nest, cfg.forage) {
        return nest_act(
            world,
            organism,
            def,
            &cfg,
            &nc,
            &fc,
            (x, y),
            laden.map(|l| l.cell),
            draw,
        );
    }
    if let Some(fc) = cfg.forage {
        forage_act(world, organism, &fc, (x, y), laden.map(|l| l.cell), draw);
        return Did::default();
    }
    let n = pellets_round(world, &cfg, (x, y));
    match laden {
        None => {
            let spoil = world.materials.id_of("spoil");
            let reach: Vec<(i32, i32)> = if cfg.pick_around {
                // up is -y: a pellet with no ground on top (an ant standing on it does not count)
                DIRS.iter()
                    .map(|&(dx, dy)| (x + dx, y + dy))
                    .filter(|&(px, py)| world.in_bounds(px, py - 1) && !ground(world, px, py - 1))
                    .collect()
            } else {
                (-1..=1).map(|dx| (x + dx, y + 1)).collect()
            };
            let src: Vec<(i32, i32)> = reach
                .into_iter()
                .filter(|&(px, py)| {
                    let c = world.get(px, py);
                    world.in_bounds(px, py)
                        && (c.material == cfg.source || Some(c.material) == spoil)
                        && c.organism_id() == 0
                        && jaw_can_cut(world, def, organism, c)
                })
                .collect();
            let p = 1.0 - (-cfg.pick * cfg.dt / n.max(1) as f32).exp();
            if src.is_empty() || draw.unit_f32() >= p {
                return Did::default();
            }
            let at = src[draw.below(src.len() as u32) as usize];
            cut_cell(world, organism, at, true);
            count(world, |c| c.picks += 1);
            Did {
                dug: 1,
                ..Did::default()
            }
        }
        Some(load) => {
            put_down(world, organism, &cfg, (x, y), load.cell, n, draw);
            Did::default()
        }
    }
}

/// **Khuong's Eq. 1, the put-down**: a laden ant takes a site at random
/// ([`sites`]) and puts its pellet there with `F(h) (1 - exp(-(eta_d,0 +
/// b_d n) dt exp(-tau_m / tau)))`. `n` is [`pellets_round`] the head.
fn put_down(
    world: &mut World,
    organism: OrganismId,
    cfg: &TextbookCfg,
    (x, y): (i32, i32),
    cell: Cell,
    n: u32,
    draw: &mut rng::Rng,
) {
    let options = sites(world, cfg, (x, y));
    if options.is_empty() {
        count(world, |c| c.no_site += 1);
        return;
    }
    let (at, side) = options[draw.below(options.len() as u32) as usize];
    let fresh = mark_round(world, cfg.tau, (x, y));
    let template = if side {
        (open_below(world, at.0, at.1) as f32 - cfg.template + 0.5).clamp(0.0, 1.0)
    } else {
        1.0
    };
    // **`base_always`**: the scent scales the pile term only, so a
    // carrier still drops at `drop_base` where nothing is fresh.
    let rate = if cfg.base_always {
        cfg.drop_base + cfg.drop_per * n as f32 * fresh
    } else {
        (cfg.drop_base + cfg.drop_per * n as f32) * fresh
    };
    let chance = template * (1.0 - (-rate * cfg.dt).exp());
    let won = draw.unit_f32() < chance;
    count(world, |c| {
        c.rolls += 1;
        c.mark_sum += f64::from(fresh);
        c.drops += u64::from(won);
        c.side_rolls += u64::from(side);
        c.side_drops += u64::from(side && won);
    });
    if won {
        place_pellet(world, organism, at, cell);
        world.creature_stats.spoil_dumped += 1;
        let frame = world.frame;
        if let Some(log) = world.textbook.as_mut().and_then(|t| t.nest_log.as_mut()) {
            log.push((frame, organism, at.0, at.1, cell.material, true));
        }
    }
}

/// **Layer 1's verbs** ([`NestCfg`]): routed by what the ant carries and
/// its role. Returns what the tick charges for (a cut is a dig).
#[allow(clippy::too_many_arguments)]
fn nest_act(
    world: &mut World,
    organism: OrganismId,
    def: &CreatureDef,
    cfg: &TextbookCfg,
    nc: &NestCfg,
    fc: &ForageCfg,
    (x, y): (i32, i32),
    load: Option<Cell>,
    draw: &mut rng::Rng,
) -> Did {
    match load {
        Some(cell) if cell.material == fc.food => {
            forage_act(world, organism, fc, (x, y), Some(cell), draw);
            Did::default()
        }
        Some(mut cell) => {
            // **Put down as spoil**, which stands wherever it has ground
            // beneath (`needs_footing`). The cut's own pellet is packed
            // soil, which crumbles to loose soil with fewer than three
            // ground neighbours: a lone pellet on the rim poured back into
            // the crater, and the nest region's ground stayed level while
            // 1,883 cells were cut (2026-10-16). The game ant gets the same
            // under `SPOIL_FOOTING`.
            if let Some(m) = world.materials.id_of(if nc.loose { "soil" } else { "spoil" }) {
                cell.material = m;
            }
            // Spoil goes out: nothing is put down under a roof, nor by
            // the door.
            if !roofed(world, nc.roof, x, y) && !in_hole(world, nc.roof, x, y) && (x - fc.home.0).abs() > nc.clear_r {
                let n = pellets_round(world, cfg, (x, y));
                put_down(world, organism, cfg, (x, y), cell, n, draw);
            }
            Did::default()
        }
        None if forager(organism, nc) => {
            // The feeder vector is the ant's own memory; with none, it
            // explores and never gives up.
            let memory = world.textbook.as_ref().and_then(|t| t.food_at.get(&organism).copied());
            let mut f = *fc;
            match memory {
                Some(m) => f.feeder = m,
                None => f.give_up = 0,
            }
            f.home_r = f.home_r.max(nc.store_r);
            let before = world.organism(organism).is_some_and(|s| s.spoil.is_some());
            forage_act(world, organism, &f, (x, y), None, draw);
            let took = !before && world.organism(organism).is_some_and(|s| s.spoil.is_some());
            if let Some(t) = world.textbook.as_mut() {
                if took {
                    t.food_at.insert(organism, (x, y));
                } else if t.trips.get(&organism).is_some_and(|t| t.1) {
                    t.food_at.remove(&organism);
                }
            }
            Did::default()
        }
        None => {
            // Any ground the jaw can cut, lining included (the cut below
            // lines the walls round it, and a digger held to the bed's own
            // soil could never widen a lined tunnel), but not spoil: put
            // down outside, it is the mound, and diggers working the door
            // took it straight back (first run, 2026-10-16). Nor any cell
            // an ant put down (`build_marks`, Khuong's marked pellets): a
            // put-down pellet keeps the cut cell's material, soil or lining,
            // and diggers dug the mound back as fast as it was built (10k
            // frames: 609 cuts, 624 put-downs, ground in the nest region
            // +15 cells). Only while the mark is fresh (`tau`): marks are
            // never cleared, so a cell where a pellet once lay stayed
            // uncuttable for good, whatever filled it later.
            // **It digs ahead** (the cell it faces, or one point either
            // side), never round it: a crowd at the door that cut every cell
            // it touched dug one wide crater, and the whole colony stood
            // in it (first run, 2026-10-16). Ants in a tunnel face along
            // it, so the face is the tunnel's end.
            let spoil = world.materials.id_of("spoil");
            // Spoil inside is in the way, and is cleared: kept off-limits
            // everywhere, the spoil that fell into the shaft floored it
            // with a layer no digger could cut (2026-10-17).
            let here_inside = inside(world, nc.roof, x, y);
            let ahead = world
                .organism(organism)
                .and_then(|s| facing_of(&s.chain))
                .map_or(0, |(_, _, f)| f);
            let dirs: &[usize] = if nc.straight {
                &[ahead]
            } else {
                &[ahead, (ahead + 1) % 8, (ahead + 7) % 8]
            };
            let face: Vec<(i32, i32)> = dirs
                .iter()
                .map(|&d| DIRS[d])
                .map(|(dx, dy)| (x + dx, y + dy))
                .filter(|&(px, py)| {
                    world.in_bounds(px, py) && {
                        let c = world.get(px, py);
                        c.organism_id() == 0
                            && c.material != fc.food
                            && (here_inside || Some(c.material) != spoil)
                            && !world
                                .build_marks
                                .get(&(px, py))
                                .is_some_and(|&at| ((world.frame - at) as f32) < cfg.tau)
                            && jaw_can_cut(world, def, organism, c)
                    }
                })
                .collect();
            // **Digging is done inside.** Cutting from the open surface
            // by the door, the colony dug an open pit rather than a nest:
            // 24% of the spoil was put back down in it (open to the sky, so
            // "outside"), 218 of 563 cut cells were cut again, and the pit
            // held at ~160 cells (pellet funnel, seed 1, 40k frames,
            // 2026-10-17). The founding shaft is the first inside.
            let traced = world.textbook.as_ref().is_some_and(|t| t.trace_ids.contains(&organism));
            if traced {
                let ahead_ants = ants_ahead(world, organism, (x, y), ahead);
                let what: String = [ahead, (ahead + 1) % 8, (ahead + 7) % 8]
                    .iter()
                    .map(|&d| {
                        let (px, py) = (x + DIRS[d].0, y + DIRS[d].1);
                        let c = world.get(px, py);
                        if c.organism_id() != 0 {
                            'a'
                        } else if c.material == material::EMPTY {
                            '.'
                        } else {
                            world.materials.get(c.material).name.chars().next().unwrap_or('?')
                        }
                    })
                    .collect();
                let line = format!(
                    "{},{organism},{x},{y},{},{ahead},{what},{},{ahead_ants},{}",
                    world.frame,
                    u8::from(here_inside),
                    face.len(),
                    u8::from(roofed(world, nc.roof, x, y))
                );
                if let Some(t) = world.textbook.as_mut() {
                    t.trace.push(line);
                }
            }
            // **Walled in, it cuts its way out**, inside or not (the game
            // ant's escape cut): spoil put down round ants on the surface
            // lined into a shell, and 6 of 14 traced diggers stood in it
            // for the whole run, outside and so forbidden to cut (2026-10-17).
            let sealed = usable_headings(world, organism, def).is_empty();
            if face.is_empty() || (nc.inside_only && !here_inside && !sealed) {
                return Did::default();
            }
            let company = if nc.dig_crowd > 0.0 {
                let mut near: Vec<OrganismId> = (-2..=2)
                    .flat_map(|dy| (-2..=2).map(move |dx| (x + dx, y + dy)))
                    .filter(|&(px, py)| world.in_bounds(px, py))
                    .map(|(px, py)| world.get(px, py).organism_id())
                    .filter(|&o| o != 0 && o != organism)
                    .collect();
                near.sort_unstable();
                near.dedup();
                (near.len() as f32 / nc.dig_crowd).min(1.0)
            } else {
                1.0
            };
            count(world, |c| c.dig_rolls += 1);
            let at = face[draw.below(face.len() as u32) as usize];
            let fed = if nc.dig_new < 1.0 {
                let (n, _) = fresh_cuts(world, nc, at, 2);
                nc.dig_new + (1.0 - nc.dig_new) * (n as f32 / 2.0).min(1.0)
            } else {
                1.0
            };
            if draw.unit_f32() >= nc.dig * company * fed {
                return Did::default();
            }
            let (frame, cut) = (world.frame, world.get(at.0, at.1).material);
            if let Some(log) = world.textbook.as_mut().and_then(|t| t.nest_log.as_mut()) {
                log.push((frame, organism, at.0, at.1, cut, false));
            }
            cut_cell(world, organism, at, true);
            if let Some(t) = world.textbook.as_mut() {
                t.cuts.insert(at, frame);
            }
            // **The cut lines its walls**, as the game ant's and the needs
            // walk's do (`line_burrow`): a tunnel cut without it falls in
            // behind the digger, unlined soil being measured to come down
            // by frame 5 (world review, `world-review/medium.md`, 2026-10-16).
            line_burrow(world, at.0, at.1);
            count(world, |c| c.digs += 1);
            Did {
                dug: 1,
                ..Did::default()
            }
        }
    }
}

/// **The walk, in place of the move**: level 1's walker on the ant's body.
/// Turn one point with `turn`, then take the first usable heading in the
/// order straight, one point either way, two, three, and back.
pub(super) fn step(
    world: &mut World,
    organism: OrganismId,
    heading: u8,
    def: &CreatureDef,
    draw: &mut rng::Rng,
) -> bool {
    let turn = world.textbook.as_ref().map_or(0.0, |t| t.cfg.turn);
    let Some((chain, groups, fates)) = world
        .organism(organism)
        .map(|s| (s.chain.clone(), s.segment_groups.clone(), s.fates))
    else {
        note_outcome(world, DecisionOutcome::NoBody);
        return false;
    };
    if chain.is_empty() {
        note_outcome(world, DecisionOutcome::NoBody);
        return false;
    }
    let usable = usable_headings(world, organism, def);
    let forage = world.textbook.as_ref().and_then(|t| t.cfg.forage);
    let laden = world.organism(organism).is_some_and(|s| s.spoil.is_some());
    let gave_up = world
        .textbook
        .as_ref()
        .and_then(|t| t.trips.get(&organism))
        .is_some_and(|t| t.1);
    let nest = world.textbook.as_ref().and_then(|t| t.cfg.nest);
    let load = world.organism(organism).and_then(|s| s.spoil.map(|l| l.cell));
    // Under layer 1 only food is a laden trip: a soil carrier lays no trail.
    let laden = match (nest, forage) {
        (Some(_), Some(fc)) => load.is_some_and(|c| c.material == fc.food),
        _ => laden,
    };
    if let (Some(nc), Some(_)) = (nest, forage) {
        if nc.wait > 0 && load.is_none() && !forager(organism, &nc) {
            if let Some((hx, hy, facing)) = facing_of(&chain) {
                if !inside(world, nc.roof, hx, hy)
                    && ants_ahead(world, organism, (hx, hy), facing) >= nc.wait
                    && draw.unit_f32() < nc.wait_p
                {
                    count(world, |c| c.waits += 1);
                    note_outcome(world, DecisionOutcome::RollFailedIdle);
                    return false;
                }
            }
        }
    }
    let chosen = match (nest, forage) {
        (Some(nc), Some(fc)) => Some(nest_heading(
            world, organism, &nc, &fc, &chain, &usable, load, gave_up, draw,
        )),
        (_, Some(fc)) => Some(forage_heading(world, &fc, &chain, &usable, laden || gave_up, draw)),
        _ => None,
    };
    let heading = match chosen {
        Some(choice) => match choice {
            Some(d) => d,
            None => {
                count(world, |c| c.stuck += 1);
                note_outcome(world, DecisionOutcome::RollFailedIdle);
                return false;
            }
        },
        None => heading,
    };
    let want = if forage.is_none() && draw.unit_f32() < turn {
        (heading + if draw.flip() { 1 } else { 7 }) % 8
    } else {
        heading
    };
    // **Under slice C a blocked ant's turn order is drawn each step.** A
    // fixed order (clockwise first) gives every ant on a wall the same
    // handedness, and in the sorting walker box it carried every seed's
    // brood into one corner (`examples/broodsort`, 8/8). Slices A and B keep
    // the fixed order so their recorded numbers reproduce; the bridge runs
    // every condition mirrored, which cancels it there.
    let sorting = world.textbook.as_ref().is_some_and(|t| t.cfg.sort.is_some());
    let order: [u8; 8] = if sorting && draw.flip() {
        [0, 7, 1, 6, 2, 5, 3, 4]
    } else {
        [0, 1, 7, 2, 6, 3, 5, 4]
    };
    let Some(to) = order.iter().map(|&o| (want + o) % 8).find(|d| usable.contains(d)) else {
        count(world, |c| c.stuck += 1);
        note_outcome(world, DecisionOutcome::RollFailedIdle);
        return false;
    };
    let authored = segment_authored(def, fates);
    let authored_widths: Vec<u8> = authored
        .iter()
        .map(|s| if s.lateral.is_some() { 2 } else { 1 })
        .collect();
    let body = BodyShape {
        chain: &chain,
        groups: &groups,
        authored: &authored_widths,
    };
    commit_step(
        world,
        organism,
        def,
        body,
        &authored,
        &authored_widths,
        heading,
        to,
        parting_enabled(),
    );
    count(world, |c| c.steps += 1);
    if let Some(fc) = forage.filter(|fc| gave_up && !laden && fc.no_entry > 0.0) {
        let body = world.organism(organism).map(|s| s.chain.clone()).unwrap_or_default();
        let amount = (fc.no_entry * f32::from(pheromone::SCALE)) as pheromone::Scent;
        match body.first() {
            Some(&(hx, hy)) if fc.mark_fork => {
                // Once a trip, at the first junction reached: the body is
                // then still on the branch. Marking at every step near the
                // junction put the mark in the neck the nest shares, and
                // the colony stayed in the nest (2026-10-16, seed 1: the
                // nest walls lined with ants, 472 crossings).
                let first = world
                    .textbook
                    .as_ref()
                    .and_then(|t| t.trips.get(&organism))
                    .is_some_and(|t| !t.2);
                if first && junction(world, hx, hy) {
                    for &(bx, by) in &body[1..] {
                        world.deposit_pheromone(Channel::A, bx, by, amount);
                    }
                    if let Some(t) = world.textbook.as_mut().and_then(|t| t.trips.get_mut(&organism)) {
                        t.2 = true;
                    }
                    count(world, |c| c.no_entry_steps += 1);
                }
            }
            Some(&(hx, hy)) => {
                let done = fc.mark_to_fork
                    && world
                        .textbook
                        .as_ref()
                        .and_then(|t| t.trips.get(&organism))
                        .is_some_and(|t| t.2);
                if !done {
                    if fc.mark_to_fork && junction(world, hx, hy) {
                        if let Some(t) = world.textbook.as_mut().and_then(|t| t.trips.get_mut(&organism)) {
                            t.2 = true;
                        }
                    } else {
                        world.deposit_pheromone(Channel::A, hx, hy, amount);
                        count(world, |c| c.no_entry_steps += 1);
                    }
                }
            }
            None => {}
        }
    }
    if let Some(fc) = forage.filter(|fc| laden || fc.lay_out) {
        let head = world.organism(organism).and_then(|s| s.chain.first().copied());
        let lays = match head {
            Some((hx, hy)) if fc.crowd > 0.0 => {
                let mut others: Vec<OrganismId> = DIRS
                    .iter()
                    .filter(|&&(dx, dy)| world.in_bounds(hx + dx, hy + dy))
                    .map(|&(dx, dy)| world.get(hx + dx, hy + dy).organism_id())
                    .filter(|&o| o != 0 && o != organism)
                    .collect();
                others.sort_unstable();
                others.dedup();
                let met = world.textbook.as_mut().map_or(0.0, |t| {
                    let m = t.met.entry(organism).or_insert((0.0, Vec::new()));
                    let new = others.iter().filter(|o| m.1.binary_search(o).is_err()).count();
                    m.0 = m.0 * (1.0 - 1.0 / CROWD_WINDOW) + new as f32;
                    m.1 = others;
                    t.counts.crowd_rolls += 1;
                    t.counts.met_sum += f64::from(m.0);
                    m.0
                });
                draw.unit_f32() < (-fc.crowd * met).exp().max(1.0 / 5.6)
            }
            _ => true,
        };
        if let Some((hx, hy)) = head.filter(|_| lays) {
            let amount = if laden { fc.lay } else { fc.lay * fc.out_frac };
            let amount = if laden && fc.explore_lay > 0.0 {
                amount
                    * (1.0
                        + fc.explore_lay * f32::from(world.pheromone_at(Channel::A, hx, hy))
                            / f32::from(pheromone::SCALE))
            } else {
                amount
            };
            world.deposit_pheromone(
                Channel::B,
                hx,
                hy,
                (amount * f32::from(pheromone::SCALE)) as pheromone::Scent,
            );
        }
    }
    if let Some(fc) = forage.filter(|fc| fc.explore > 0.0) {
        // Every ant, either way, laden or not: the paper's E is laid by
        // all ants, and held back only by the food trail underfoot.
        if let Some((hx, hy)) = world.organism(organism).and_then(|s| s.chain.first().copied()) {
            let f = f32::from(world.pheromone_at(Channel::B, hx, hy)) / f32::from(pheromone::SCALE);
            let e = f32::from(world.pheromone_at(Channel::A, hx, hy)) / f32::from(pheromone::SCALE);
            let room = if fc.explore_max > 0.0 {
                (1.0 - e / fc.explore_max).max(0.0)
            } else {
                1.0
            };
            let chance = (1.0 - fc.explore_c * f / fc.k.max(1e-6)) * room;
            if chance > 0.0 && draw.unit_f32() < chance {
                world.deposit_pheromone(
                    Channel::A,
                    hx,
                    hy,
                    (fc.explore * f32::from(pheromone::SCALE)) as pheromone::Scent,
                );
                count(world, |c| c.explore_steps += 1);
            }
        }
    }
    true
}

/// **Slice B's verbs**: an empty ant takes a cell of food it touches; a
/// laden one near home puts its load on an open cell beside the head with
/// ground beneath it.
/// **Slice C's verbs** ([`SortCfg`]): an empty ant picks up an item it
/// touches with `1 - score` there; a laden one puts its item on an open cell
/// beside its head, with ground beneath, with `score` there.
fn sort_act(
    world: &mut World,
    organism: OrganismId,
    sc: &SortCfg,
    (x, y): (i32, i32),
    load: Option<Cell>,
    draw: &mut rng::Rng,
) {
    match load {
        None => {
            let items: Vec<(i32, i32)> = DIRS
                .iter()
                .map(|&(dx, dy)| (x + dx, y + dy))
                .filter(|&(px, py)| {
                    world.in_bounds(px, py) && {
                        let c = world.get(px, py);
                        c.material == sc.item && c.organism_id() == 0
                    }
                })
                .collect();
            if items.is_empty() {
                return;
            }
            let at = items[draw.below(items.len() as u32) as usize];
            let shade = world.get(at.0, at.1).shade;
            if draw.unit_f32() < 1.0 - sc.score(world, shade, at, at) {
                cut_cell(world, organism, at, false);
                count(world, |c| c.picks += 1);
            }
        }
        Some(cell) => {
            let open: Vec<(i32, i32)> = DIRS
                .iter()
                .map(|&(dx, dy)| (x + dx, y + dy))
                .filter(|&(px, py)| world.in_bounds(px, py) && world.is_empty(px, py) && ground(world, px, py + 1))
                .collect();
            if open.is_empty() {
                count(world, |c| c.no_site += 1);
                return;
            }
            let at = open[draw.below(open.len() as u32) as usize];
            let won = draw.unit_f32() < sc.score(world, cell.shade, at, at);
            count(world, |c| {
                c.rolls += 1;
                c.drops += u64::from(won);
            });
            if won {
                place_pellet(world, organism, at, cell);
            }
        }
    }
}

fn forage_act(
    world: &mut World,
    organism: OrganismId,
    fc: &ForageCfg,
    (x, y): (i32, i32),
    load: Option<Cell>,
    draw: &mut rng::Rng,
) {
    let home = (x - fc.home.0).pow(2) + (y - fc.home.1).pow(2) <= fc.home_r.pow(2);
    if fc.give_up > 0 {
        let food_near = fc.sense > 0
            && (-fc.sense..=fc.sense).any(|dy| {
                (-fc.sense..=fc.sense).any(|dx| {
                    world.in_bounds(x + dx, y + dy) && {
                        let c = world.get(x + dx, y + dy);
                        c.material == fc.food && c.organism_id() == 0
                    }
                })
            });
        if let Some(t) = world.textbook.as_mut() {
            if home || load.is_some() {
                t.trips.remove(&organism);
            } else {
                let near =
                    fc.search_r == 0 || (x - fc.feeder.0).pow(2) + (y - fc.feeder.1).pow(2) <= fc.search_r.pow(2);
                let trip = t.trips.entry(organism).or_insert((0, false, false));
                trip.0 += u32::from(near && !food_near);
                trip.1 |= trip.0 >= fc.give_up;
            }
        }
    }
    match load {
        None if !home => {
            let food: Vec<(i32, i32)> = DIRS
                .iter()
                .map(|&(dx, dy)| (x + dx, y + dy))
                .filter(|&(px, py)| {
                    world.in_bounds(px, py) && {
                        let c = world.get(px, py);
                        c.material == fc.food && c.organism_id() == 0
                    }
                })
                .collect();
            if !food.is_empty() {
                let at = food[draw.below(food.len() as u32) as usize];
                cut_cell(world, organism, at, false);
                count(world, |c| c.food_picks += 1);
            }
        }
        Some(cell) if home => {
            let open: Vec<(i32, i32)> = DIRS
                .iter()
                .map(|&(dx, dy)| (x + dx, y + dy))
                .filter(|&(px, py)| world.in_bounds(px, py) && world.is_empty(px, py) && ground(world, px, py + 1))
                .collect();
            if !open.is_empty() {
                let at = open[draw.below(open.len() as u32) as usize];
                place_pellet(world, organism, at, cell);
                count(world, |c| c.delivered += 1);
            }
        }
        _ => {}
    }
}

/// **Slice B's step**: a usable heading drawn in proportion to
/// persistence x vector x trail ([`ForageCfg`]). `None` with no usable
/// heading.
fn forage_heading(
    world: &World,
    fc: &ForageCfg,
    chain: &[(i32, i32)],
    usable: &[u8],
    laden: bool,
    draw: &mut rng::Rng,
) -> Option<u8> {
    let (hx, hy, facing) = facing_of(chain)?;
    let (tx, ty) = if laden { fc.aim_home } else { fc.feeder };
    let pull = if laden { fc.vector } else { fc.vector_out };
    let ty = if fc.out_flat { hy } else { ty };
    let (vx, vy) = ((tx - hx) as f32, (ty - hy) as f32);
    heading_by(world, fc, (hx, hy), facing, usable, (vx, vy), pull, fc.read, draw)
}

/// The facing of a body: from its second cell to its head, as an index
/// into `DIRS` (0 for a one-cell body).
fn facing_of(chain: &[(i32, i32)]) -> Option<(i32, i32, usize)> {
    let &(hx, hy) = chain.first()?;
    let facing = chain
        .get(1)
        .and_then(|&(bx, by)| {
            let d = ((hx - bx).signum(), (hy - by).signum());
            DIRS.iter().position(|&v| v == d)
        })
        .unwrap_or(0);
    Some((hx, hy, facing))
}

/// **Layer 1's step** ([`NestCfg`]): slice B's weighting, aimed by what the
/// ant carries and its role. Out of the nest (a soil carrier, a forager
/// under a roof) it pulls up; a digger under a roof pulls down, and out on
/// the surface pulls home; a food carrier and a forager that gave up pull
/// home; a forager pulls to the food it remembers. Only foragers and food
/// carriers read the food trail.
#[allow(clippy::too_many_arguments)]
fn nest_heading(
    world: &World,
    organism: OrganismId,
    nc: &NestCfg,
    fc: &ForageCfg,
    chain: &[(i32, i32)],
    usable: &[u8],
    load: Option<Cell>,
    gave_up: bool,
    draw: &mut rng::Rng,
) -> Option<u8> {
    let (hx, hy, facing) = facing_of(chain)?;
    let under = inside(world, nc.roof, hx, hy);
    let toward = |(tx, ty): (i32, i32)| ((tx - hx) as f32, (ty - hy) as f32);
    let food = load.is_some_and(|c| c.material == fc.food);
    let forages = forager(organism, nc);
    let ((vx, vy), pull) = match load {
        Some(_) if food => (toward(fc.aim_home), fc.vector),
        Some(_) if under || in_hole(world, nc.roof, hx, hy) => ((0.0, -1.0), nc.up),
        Some(_) => ((0.0, 0.0), 0.0),
        None if forages && gave_up => (toward(fc.aim_home), fc.vector),
        None if forages && under => ((0.0, -1.0), nc.up),
        None if forages => match world.textbook.as_ref().and_then(|t| t.food_at.get(&organism)) {
            Some(&m) => (toward(m), fc.vector_out),
            None => ((0.0, 0.0), 0.0),
        },
        None => {
            // A digger inside works down; outside it walks to the door
            // itself. Pulled down anywhere near the door, diggers stood on
            // the mound over it pressing into spoil they may not cut: at
            // 30k frames 19 of 37 were above the old ground line, 3 below
            // 12 rows (2026-10-16).
            if nc.jam > 0 && under && ants_ahead(world, organism, (hx, hy), facing) >= nc.jam {
                ((0.0, -1.0), nc.up)
            } else if under && nc.dig_pull > 0.0 && fresh_cuts(world, nc, (hx, hy), 8).0 > 0 {
                let (_, (sx, sy)) = fresh_cuts(world, nc, (hx, hy), 8);
                ((sx as f32, sy as f32), nc.dig_pull)
            } else if under || (hx - fc.home.0).pow(2) + (hy - fc.home.1).pow(2) <= (2 * fc.home_r).pow(2) {
                // At the door it goes down too: with `inside_only` it cannot
                // cut the surface there, and without the pull, diggers that
                // reached the door point milled on the surface (a 20-ant
                // colony had 0-3 underground; 85% of a 125-ant colony's
                // digger decisions were outside, within 4 columns of the
                // door, facing other ants: decision trace, 2026-10-17).
                ((0.0, 1.0), nc.down)
            } else {
                (toward(fc.aim_home), nc.home)
            }
        }
    };
    let read = fc.read && (food || forages);
    heading_by(world, fc, (hx, hy), facing, usable, (vx, vy), pull, read, draw)
}

/// **The weighted step**: a usable heading drawn in proportion to
/// persistence x `exp(pull cos)` x trail x mark, `cos` against `(vx, vy)`
/// (a zero vector with zero pull is no pull). `None` with no usable heading.
#[allow(clippy::too_many_arguments)]
fn heading_by(
    world: &World,
    fc: &ForageCfg,
    (hx, hy): (i32, i32),
    facing: usize,
    usable: &[u8],
    (vx, vy): (f32, f32),
    pull: f32,
    read: bool,
    draw: &mut rng::Rng,
) -> Option<u8> {
    let vlen = (vx * vx + vy * vy).sqrt().max(1e-6);
    let mut weights = [0f32; 8];
    let mut total = 0.0;
    for &d in usable {
        let (dx, dy) = DIRS[d as usize];
        let turn = (d as usize + 8 - facing) % 8;
        let persist = [1.0, 0.5, 0.15, 0.03, 0.01, 0.03, 0.15, 0.5][turn];
        let dl = ((dx * dx + dy * dy) as f32).sqrt();
        let cos = (dx as f32 * vx + dy as f32 * vy) / (dl * vlen);
        let c = if read {
            f32::from(world.pheromone_at(Channel::B, hx + dx, hy + dy)) / f32::from(pheromone::SCALE)
        } else {
            0.0
        };
        let mark = if read && fc.no_entry > 0.0 {
            (-fc.repel * f32::from(world.pheromone_at(Channel::A, hx + dx, hy + dy)) / f32::from(pheromone::SCALE))
                .exp()
        } else if read && fc.explore > 0.0 && fc.explore_k > 0.0 {
            1.0 + f32::from(world.pheromone_at(Channel::A, hx + dx, hy + dy))
                / f32::from(pheromone::SCALE)
                / fc.explore_k
        } else {
            1.0
        };
        let trail = if fc.kn {
            fc.k.powi(fc.n) + c.powi(fc.n)
        } else {
            (fc.k + c).powi(fc.n)
        };
        let w = persist * (pull * cos).exp() * trail * mark;
        weights[d as usize] = w;
        total += w;
    }
    if total <= 0.0 {
        return None;
    }
    let mut r = draw.unit_f32() * total;
    for &d in usable {
        let w = weights[d as usize];
        if r < w {
            return Some(d);
        }
        r -= w;
    }
    usable.last().copied()
}

/// Whether `(x, y)` is a junction: three or more separate runs of open
/// cells on the ring of 16 cells two out ([`ForageCfg::mark_fork`]). A
/// straight tunnel shows two runs, a corner two, a T or a Y three.
fn junction(world: &World, x: i32, y: i32) -> bool {
    let mut ring = Vec::with_capacity(16);
    for dx in -2..=2 {
        ring.push((dx, -2));
    }
    for dy in -1..=2 {
        ring.push((2, dy));
    }
    for dx in (-2..=1).rev() {
        ring.push((dx, 2));
    }
    for dy in (-1..=1).rev() {
        ring.push((-2, dy));
    }
    let open: Vec<bool> = ring
        .iter()
        .map(|&(dx, dy)| world.in_bounds(x + dx, y + dy) && !solid_for_ring(world, x + dx, y + dy))
        .collect();
    let runs = (0..open.len())
        .filter(|&i| open[i] && !open[(i + open.len() - 1) % open.len()])
        .count();
    runs >= 3
}

/// A ring cell blocks a junction run when it is terrain: anything but air
/// or an ant.
fn solid_for_ring(world: &World, x: i32, y: i32) -> bool {
    !world.is_empty(x, y) && world.get(x, y).organism_id() == 0
}

fn count(world: &mut World, f: impl FnOnce(&mut TextbookCounts)) {
    if let Some(t) = world.textbook.as_mut() {
        f(&mut t.counts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Rect;

    /// A stone box with a soil floor and `n` textbook ants on it, as
    /// `examples/pillarbox` builds it at a tenth the size.
    fn bed(n: usize, tau: f32) -> World {
        let (w, h) = (60, 40);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        let stone = world.materials.id_of("stone").expect("stone");
        let soil = world.materials.id_of("soil").expect("soil");
        for y in 0..h {
            for x in 0..w {
                if x < 2 || y < 2 || x >= w - 2 || y >= h - 2 {
                    world.set(x, y, Cell::new(stone, 0));
                } else if y >= h - 10 {
                    world.set(x, y, Cell::new(soil, 0));
                }
            }
        }
        for _ in 0..100 {
            crate::sim::update::step(&mut world);
        }
        world.births_paused = true;
        let id = world.species.id_of("ant").expect("ant ships");
        for i in 0..n {
            let x = 6 + (i as i32 * 47) % (w - 12);
            if let Some(site) = plant_creature_seed(&mut world, x, h - 11, "ant") {
                world.schedule_active_site(site);
            }
        }
        let cfg = TextbookCfg {
            p_move: 1.0,
            turn: 0.125,
            dt: 4.0,
            drop_base: 0.025,
            drop_per: 0.11,
            pick: 0.029,
            tau,
            template: 2.0,
            source: soil,
            n_rock: false,
            pick_around: true,
            base_always: true,
            build_on_box: false,
            forage: None,
            sort: None,
            nest: None,
        };
        // The layer is laid fresh at the start (the harness's bootstrap:
        // with no deposit anywhere, Eq. 1 never puts a pellet down).
        for x in 2..w - 2 {
            world.build_marks.insert((x, h - 10), 0);
        }
        world.textbook = Some(Box::new(Textbook::new(id, cfg)));
        world
    }

    fn run(world: &mut World, frames: u64) {
        for _ in 0..frames {
            crate::sim::update::step(world);
            world.step_active_sites();
        }
    }

    /// **The three verbs fire on the game's body**: the ants step, cut the
    /// source floor and put pellets down where they stand (`spoil` left in
    /// the world). The control for "it did nothing":
    /// every count starts at zero.
    #[test]
    fn textbook_ants_walk_pick_and_put_down() {
        let mut w = bed(6, 300.0);
        assert_eq!(w.textbook.as_ref().expect("set").counts.decisions, 0);
        run(&mut w, 6_000);
        let c = w.textbook.as_ref().expect("set").counts;
        assert!(c.decisions > 1_000 && c.steps > 500, "the ants did not walk: {c:?}");
        assert!(c.picks > 0 && c.drops > 0, "no pick or no put-down: {c:?}");
        let spoil = w.materials.id_of("spoil");
        let (wd, ht) = (60, 40);
        let standing = (0..ht)
            .flat_map(|y| (0..wd).map(move |x| (x, y)))
            .filter(|&(x, y)| Some(w.get(x, y).material) == spoil)
            .count();
        assert!(standing > 0, "nothing put down stands: {c:?}");
    }

    /// **The pheromone reads the latest deposit still standing, and fades**:
    /// a mark on a cell that is now empty counts nothing; the latest of two
    /// wins; one `tau` old reads e^-1; `tau` 0 is no pheromone and infinite
    /// reads 1.
    #[test]
    fn mark_round_reads_the_latest_standing_deposit() {
        let mut w = World::new(Rect::new(0, 0, 19, 19));
        let spoil = w.materials.id_of("spoil").expect("spoil");
        w.set(5, 5, Cell::new(spoil, 0));
        w.set(4, 5, Cell::new(spoil, 0));
        w.build_marks.insert((5, 5), 0);
        w.build_marks.insert((4, 5), 100);
        w.build_marks.insert((6, 5), 250);
        w.frame = 400;
        let m = mark_round(&w, 300.0, (5, 5));
        assert!(
            (m - (-1.0f32).exp()).abs() < 1e-4,
            "the latest standing deposit is one tau old; read {m}"
        );
        assert_eq!(mark_round(&w, 0.0, (5, 5)), 0.0);
        assert_eq!(mark_round(&w, f32::INFINITY, (5, 5)), 1.0);
        assert_eq!(mark_round(&w, 300.0, (12, 12)), 0.0);
    }

    /// The outputs the textbook ant replaces read zero; the rest pass.
    #[test]
    fn outputs_for_zeroes_only_what_it_replaces() {
        use brain::BrainOutput as O;
        let out = outputs_for([0.7; brain::BRAIN_OUTPUTS]);
        for o in [O::EmitA, O::EmitB, O::Impulse, O::Lay, O::Provision] {
            assert_eq!(out[o as usize], 0.0);
        }
        assert_eq!(out[O::Move as usize], 0.7);
    }
}
