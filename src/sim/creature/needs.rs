//! **The needs-and-jobs walk: a second way for an ant to decide.** Built
//! beside today's ant and off until a harness turns it on, so if it fails
//! nothing else is lost (owner, 2026-10-06). The design of record is the doc
//! "A needs-and-jobs ant: design for a second walk"
//! (<https://claude.ai/code/artifact/6f9b6730-a887-4697-8689-906afc9afb0a>).
//!
//! **The switch is [`World::needs`], set at a frame, never an environment
//! variable**: a variable is read once per process and so cannot hand a
//! running colony over at a frame, and a `Chooser` variant would enrol the
//! walk in the 69 `Chooser::` comparisons that already exist. `None` is
//! today's ant and every hook below is one `is_some` test.
//!
//! **A hook, not the split of `act` the design planned** (slice 0). The
//! engineering review was right that the brain's outputs reach about ten
//! places in the tick and that `act` interleaves its mechanics with the hand
//! rules that gate them. Splitting it would have moved code that seven live
//! branches were editing on the day this was built
//! (`branchcheck.sh --who-touched src/sim/creature.rs`, 2026-10-06), so the
//! walk instead **drives `act` itself**: it hands `act` the urges its drive
//! wants (1 for the verb it means, 0 for the ones it does not, the brain's
//! own for the verbs it does not own yet), and `act`'s mechanics and the
//! gates on where a load may lie or a cut may go run exactly as they do for
//! today's ant. That is the design's "haul and dig as jobs ... with their
//! own act gates" taken literally. What a need must never meet is a veto,
//! and the guard in this module's tests checks that by outcome. The three
//! things a need does that no urge can ask `act` for -- eat a load held in
//! the jaws, put soil down where it closes no way, and cut out of a sealed
//! pocket -- are done here, before `act`, with `act`'s own helpers.
//!
//! **The ten places the brain reaches, and what the walk does at each**
//! (review finding E1):
//!
//! | Site | Today | Under this walk |
//! |---|---|---|
//! | step or stay (`Move`) | the brain's roll | [`kinesis`] |
//! | forage drive and pace | lifts `Move` for a needed forager | kept for the jobs, overridden for needs and idle by [`kinesis`] |
//! | launch (`Impulse`) | the brain | unchanged: an ant authors no `Impulse` |
//! | tumble after a lost roll | the chooser pauses | unchanged: a lost roll is a pause |
//! | `act`'s six urges | the brain | [`act`]: the drive's verbs, the brain's for the rest |
//! | the step (persistence, turn, noise) | `chooser_step` | [`step`]: the brain's persistence, turn and noise, the drive's pull and terms |
//! | trail A and B | the brain | unchanged until the walk owns trail laying |
//! | synapse tax | the brain, which still runs | unchanged, so starvation compares like with like |
//! | trace row | `DecisionRow` | unchanged, plus the walk's own [`WalkRow`] |
//! | speculative brain | every ant, every tick | unchanged: the brain still runs |
//!
//! **Pass-through** ([`NeedsMode::Passthrough`]) runs every hook and hands
//! today's value straight back. It is the control for the hooks themselves:
//! a colony switched to it at any frame must stay identical to one that never
//! was, frame for frame, map for map and ledger row for ledger row. **Checked
//! 2026-10-06** (`deeptrace`, 60k frames, the switch at 20k): seeds 1 and 2
//! shipped, and seed 1 under `NEEDS_FIRST`, `CARRY_HOME` and `NEST_REST`,
//! byte-identical in every map, ledger, colony, stats, hunger and event row
//! to the build before the hooks, switch off and pass-through alike. The
//! positive control: a build nudging any one of the three live hooks (the
//! urges halved, the step chance cut by a tenth, one extra draw before the
//! step) differs at the first map after the switch and at none before.
//!
//! **The walk** ([`NeedsMode::Walk`], slice 1). Every decision, in order:
//!
//! 1. **The needs.** Hunger is a smooth ramp of what the ant has, its energy
//!    and the food in its crop, from 0 at [`HUNGER_ONSET`] of its budget to
//!    1 at [`HUNGER_FULL`]: no cliff, so a mildly hungry ant drifts out and a
//!    starving one runs. It wins over the ant's job when its urge passes the
//!    job's hold, which is the job's own patience under a ceiling
//!    ([`HOLD_CEILING`]) set below hunger's urge at 30% of the budget, so no
//!    job can outvote an ant that is starving. A hungry ant eats what it
//!    holds, puts soil down where it closes no way, goes to food it senses,
//!    goes out when inside, and searches when outside. **Escape** is hunger
//!    that has stopped getting anywhere: an ant inside whose way out has made
//!    no progress for [`ESCAPE_STALL`] decisions, pushing at soil, cuts it,
//!    and goes on cutting while its only progress is into its own cuts.
//! 2. **The job**, when no need wins: haul while a pellet is in the jaws,
//!    carry home while a trip's food is in the crop, walk back to the face
//!    while it remembers one, forage while its forage stimulus holds, and
//!    otherwise idle. Foraging is taken with probability `r^2 / (1 + r^2)`
//!    of the ratio of its stimulus to the ant's own threshold (Bonabeau,
//!    Theraulaz & Deneubourg 1996): a small floor so fed ants still scout,
//!    and a leaky count of the ant's meetings with foragers coming home with
//!    food, so ants near a busy door are the ones sent out; it is braked by
//!    food lying uneaten where the ant unloads (the owner's "more than enough
//!    food", 2026-10-06).
//! 3. **The step**, by the winning drive: the pull it aims at (food, the way
//!    out, home, the door for soil, the face) with today's patience, and
//!    today's route, scouting and door-reader terms for an ant going out.
//!    An idle ant inside has no pull at all: **the kinesis** decides where
//!    it stops, its chance of stepping falling as it nears its own preferred
//!    depth ([`PREF_DEPTH`]), so idle ants gather deep without being pulled
//!    there. An idle ant outside, or in the spoil mound, walks home.
//!
//! **Labelled stand-ins** (the design's *Locality* table): depth is rows
//! below the founding ground, and the way out inside is the nest's way
//! (`NestWay`) below that ground and the mound's way above it. Both are
//! registry maps no ant could sense. The local replacements -- depth below
//! the door vector's origin, the door vector and the walls -- come later;
//! the door vector is kept already, and used where neither way reaches.
//!
//! **The no-veto guard** is this module's tests: an ant at 30% of its budget
//! in each of six places, holding each of three loads, in either role, must
//! eat, reach the open air or dig if sealed, before it starves. It reads what
//! happened in the scene, never a weight, and today's ant fails 16 of its 36
//! rows (`the_guard_on_todays_ant`).
//!
//! **The fix round** ([`WalkParts`], 2026-10-07). Slice 1 failed its gate:
//! all four colonies collapsed late. Two faults were traced on seed 1 (the
//! project's `needs-ant/slice1/results.md`, cited here by step), one round of
//! fixes was proposed, and another lane reviewed it before any of it was
//! built. Each fix is a named part, off unless a harness names it, so a
//! failure can be pinned on one, and with every part off the walk is slice 1
//! exactly (checked: `deeptrace` seed 1, the switch at 50k, every map,
//! colony row and walk count identical to slice 1's to 55k).
//!
//! - **Only diggers dig** (`only_diggers`, `dig_job`). Slice 1 handed the
//!   brain's dig urge to every drive: 98% of the extra cuts came from ants
//!   not on the dig job (foraging 63%, walking home 23%, resting 12%), in
//!   the mound's tunnels they cut 4.5 times as often per decision as
//!   shipped ants, and the soil choked the way in (steps 1-5). `only_diggers`
//!   gives `act` a dig urge for the dig job alone; escape and the door cut
//!   make their own cuts. `dig_job` lets an ant below ground take the job on
//!   the design's own stimulus, soil ahead plus the other adults within
//!   [`DIG_REACH`] of its head ([`DIG_THRESHOLD`]), where slice 1 gave it
//!   only to an ant that remembered a face (1% of its cuts). It is its own
//!   part because crowding-driven digging is a new mechanism here, not a
//!   proven one: an older trial was a coin flip on rooms, but never had ants
//!   deep enough to act on (dead-ends 1878; check C2 inconclusive).
//! - **The door cut** (`clear`). With only diggers digging, nothing outside
//!   could cut back in through a shut door, and the trace had carriers
//!   circling the mound while it was shut (step 5). The review scoped the
//!   cut to the door ([`in_door_scope`]), since an unscoped one is Fault 1
//!   again: an empty ant walking home that has won the step roll
//!   [`CLEAR_STALL`] decisions running without getting nearer may cut the
//!   one soil cell toward the door, from the nearest point it has reached
//!   only. Near the door its home is the door itself ([`home_pull_target`]):
//!   the first test draft never stalled in 500 decisions, because an idle
//!   ant re-anchors its home at every step beside the nest and so is home
//!   anywhere on the mound. An ant with food in its crop never makes it: a
//!   pellet cut into its jaws could never be put down, since `act`'s drop
//!   branch returns first.
//! - **Foragers keep walking, and give up outside** (`pace`, `give_up`,
//!   `lay_home`). Late on, empty ants outside stepped on 12% of decisions
//!   against shipped's 49% (step 6): slice 1 left the step to the brain,
//!   which barely moves an empty ant below the egg bar (step 7), and the
//!   forage job never ended outside (step 8). `pace` floors an empty
//!   forager's step chance at [`P_HOME`], flat while the job holds, as the
//!   review asked; `give_up` ends the job outside once the scout's patience
//!   for home has run out (that patience falls only on a step, so a frozen
//!   ant never gave up); `lay_home` sends an empty forager rich enough to
//!   lay home on the shipped lay pull, where 81% of them were out on the
//!   forage drive (step 8).
//! - **Food keeps its purpose** (`meal`): a bite taken at home to eat is
//!   kept and digested, never put back down. 77 of the 78 home bites the
//!   walk's traced ants took went back down a median 15 frames later. The
//!   rule is `keeps_home_meal`, the shipped storeroom's own helper, with its
//!   line ("under its grant"), called rather than copied.
//! - **Escape counts only real stalls** (`won_stall`): the way out's stall
//!   rises only on a decision that won the step roll and got no nearer,
//!   never on one where the ant chose to stay; escape had fired on a
//!   resting ant at hunger 0.06. The review's correction: the proposal said
//!   "a step tried and refused", which never happens here (the step draws
//!   only from usable headings), so that count would never have risen and
//!   escape would never have fired.
//!
//! **Food sealed in soil** (`reach`, `eat_fade`; 2026-10-10, the project's
//! `needs-ant/resume-2026-10-10/trace-buried-food-2026-10-10.md`, reviewed
//! before it was built). On the flip world, steady food, seed 1, 57 of the
//! walk's 82 nest starvers (the shipped ant's: 1) spent their last 2,000
//! frames on the eat drive aimed at a crumb sealed in soil: the food sense
//! read through walls, hunger read it before "out", and the eat drive never
//! gave up. Its pull did fade, on the shared home patience, but every new
//! target and every excursion re-armed it, and on the eat drive the way
//! out's stall is zeroed each decision, so escape never had a turn. Two
//! crumbs held about 56 ants between them. Neither part is in
//! [`WalkParts::ALL`], which stays the fix round's, so `needsparts=all`
//! means what it meant on 10-07.
//!
//! - **The food sense stops at soil** (`reach`): a food cell counts only if
//!   its 8-ring touches a cell joined to the head through open cells inside
//!   the 13x13 box ([`reachable_from_head`]). The design's own C6 finding is
//!   that sealed soil passes no air; the sense was written as `larva_scent`
//!   is, through walls, "tolerable at 6 cells", and the trace measured it
//!   as not.
//! - **The eat drive gives up** (`eat_fade`): straight-line distance to the
//!   nearest food it senses, whatever cell, counting only decisions that won
//!   the step roll; at [`EAT_STALL`] such decisions with no progress, the
//!   whole food sense is off for [`EAT_REST`] frames, so hunger falls to
//!   "out", and escape fires on out's own stall.
//!
//! **A hunger bout ends with food** (`hunger_holds`, with its `placebo`;
//! 2026-10-10, `needs-ant/resume-2026-10-10/trace-job-hold-2026-10-10.md`,
//! reviewed before it was built). The job is chosen before need is tested,
//! and the forage roll does not ask about hunger, so a hungry ant heading out
//! could roll the forage job and take its hold (up to 0.8, which hunger
//! reaches only at about 70 J) -- out of need again with no meal. On seed 11
//! (`reach`, steady food) 58 of 69 nest starvers had done so in their last
//! 8,000 frames; so had 53% of the living deep ants, so the flip is a
//! chance, not a sentence, and the reviewer bounded what the part can save
//! at about 35 of the 69 (11 never flipped, 15-23 died walled in). With the
//! part on, need that has fired stays until hunger is back to 0: the bout
//! has no give-up of its own, its drives keep theirs, and the veto still
//! wins. The `placebo` draws the forage roll from another stream -- the
//! same rule, another realisation -- so every comparison carries the spread
//! a do-nothing change makes on the same seeds.
//!
//! **Measured and left off, 2026-10-11** (`hunger-holds-results-2026-10-10.md`).
//! Seed 11's nest starvers went 69 -> 2, but the placebo alone took them to
//! 18. On steady seeds 1-4 the part failed its declared fails: the colony's
//! low fell beyond the placebo's move on 3 of 4, mound starvers rose beyond
//! it on 3 of 4, and soil set down rose 2-10x. Starvers free to walk out rose
//! 3-5 -> 24-55. On seed 3, 46 of 75 died on Seek in a pile on the mound's
//! face, every one a spent scout (`scout_home`) pulled straight at its
//! anchor. `reach` has the same pile, and its spent Seek ants go there too
//! (28 episodes on seed 3), but all 28 were fed within 3,000 frames (median
//! 670), 24 of them in the pile by a nestmate. Why the bout stops that
//! feeding is not traced. A restart-the-scout fix (`seek_stall`) was reviewed
//! NO: it sends pile ants away from the food. *Re-test when:* the pile is
//! fed under the bout (trace the donors first), or deep starvers come back
//! as the main fault.
//!
//! **Walled in** (`sealed_out`, `eat_held`; 2026-10-11,
//! `needs-ant/resume-2026-10-10/trace-walled-in-2026-10-11.md`, two review
//! rounds). Under `reach`, 93 adults on steady seeds 1-4 starved in a soil
//! pocket the map's flood calls closed, against 32 shipped. Two faults: 41
//! held a store load, and eating needs an empty cell to set it down in a
//! pocket that is all body; 43 held a soil pellet, and `walled_in` (a perfect
//! shell) refused them because a nestmate, a crumb or the pellet itself broke
//! the shell. `sealed_out` asks the local question instead: still for
//! [`ESCAPE_STALL`] decisions without leaving a [`SEAL_ROAM`] box, the ant
//! floods the air round its head ([`seal_flood`], the food sense's 13x13
//! box); no open cell on the box's rim is sealed, and a sealed ant gets out
//! whatever its hunger or job ([`sealed_escape`]: the nearest cuttable
//! heading, soil packed behind or dropped in the pocket away from the cut,
//! a store load set down). `eat_held` lets a hungry sealed ant with nowhere
//! to set its load down eat it from its jaws ([`eat_from_jaws`], the crop
//! rule of a swallow, [`crop_take`]). Why "still" is a roam box and not
//! `moved`: an ant flips end for end every decision, so its head moves while
//! it goes nowhere, and the flood never fired.
//!
//! **Seeds 1-4, steady, 100-300k** (`results-sealed-4seed-2026-10-11.md`):
//! walled-in starvers 0/1/4/2 with both parts against 65-93 pooled in each of
//! `reach`, the two nudges and the placebo; colony mean 253 vs 248. Alone,
//! `sealed_out` leaves 49, 48 of them holding a load: the eat does the work.
//! About half of all sealed events end with no cut of the ant's own (a
//! nestmate or a slump opened it). The nest-shape lines (chambers, band
//! fill, ground dug, the way in) leave the baselines' range no more often
//! than a baseline leaves the other three's. Known gap: a pocket wider than
//! the 13x13 box reads open (three ants on seed 3).
//!
//! **Twelve steady seeds, and heap 30 seeds 1-4** (same file): walled-in
//! starvers 31 against 206-278 per baseline arm, fewer than `reach` on 12
//! of 12; on heap 30, 40 against 121. Mound starvers 33 against 168-336.
//! The seal floods cost 0.25% of a 1,250-ant frame (`needscost cg=1`). One
//! named fail line leans: ants holding soil with food in the crop, +6% over
//! the highest baseline (a fed ant that cut free keeps its pellet, and `act`
//! cannot set soil down while laden). Two late heap-30 die-offs follow a
//! colony overshoot the baselines also show; 16 of 932 starvers there had
//! been sealed in their last 3,000 frames. Both parts are on in the new
//! ant since PR 684 ([`WalkParts::NEW_ANT`], the switch `walled_in`).

use super::*;
use std::collections::HashMap;

/// **Which walk the ants use once [`World::needs`] is set.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeedsMode {
    /// Every hook runs and hands today's value back unchanged: the identity
    /// control for the hooks.
    Passthrough,
    /// The needs-and-jobs walk (slice 1).
    Walk,
}

impl NeedsMode {
    /// `passthrough` or `walk`, as a harness names it. Anything else is
    /// refused rather than read as a default, so a typo cannot run the
    /// control under another arm's label.
    pub fn parse(raw: &str) -> NeedsMode {
        match raw.trim() {
            "passthrough" => NeedsMode::Passthrough,
            "walk" => NeedsMode::Walk,
            other => panic!("needs={other:?}: the walk's modes are passthrough and walk"),
        }
    }
}

/// **The fix round's parts** (slice 1's one round of fixes, 2026-10-07):
/// each a named part, so a failure can be pinned on one. All off, which
/// [`NeedsWalk::new`] sets, is slice 1 exactly, so the gate round stays
/// reproducible. What each part answers is in the module doc's *The fix
/// round*.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkParts {
    /// Only the dig job digs at `act`'s urge: every other drive hands `act`
    /// no dig urge (escape and the door cut make their own cuts).
    pub only_diggers: bool,
    /// The dig job is taken on its own stimulus, soil at the face plus the
    /// bodies round the head, not only by an ant that remembers a face.
    pub dig_job: bool,
    /// Near the door, an idle ant walking home aims at the door itself
    /// rather than its last nest contact, and an empty one that has stopped
    /// getting nearer may cut the soil cell toward it, near the door only.
    pub clear: bool,
    /// An empty forager steps at least at [`P_HOME`] while it holds the job.
    pub pace: bool,
    /// The forage job ends outside when the scout's patience runs out.
    pub give_up: bool,
    /// An empty forager rich enough to lay walks home to lay.
    pub lay_home: bool,
    /// Food picked up at home to eat is kept and digested, never put back
    /// down (`keeps_home_meal`, the helper the shipped walk's storeroom shares).
    pub meal: bool,
    /// The way out's stall counts only decisions that won the step roll and
    /// got no nearer, never a decision the ant chose to stay.
    pub won_stall: bool,
    /// **Not the fix round's** (2026-10-10, never in [`WalkParts::ALL`]): the
    /// food sense counts only food whose 8-ring touches a cell joined to the
    /// head through open cells inside its box, so a crumb sealed in soil is
    /// not sensed.
    pub reach: bool,
    /// **Not the fix round's** (2026-10-10, never in [`WalkParts::ALL`]): the
    /// eat drive gives up after [`EAT_STALL`] won decisions that got no
    /// nearer the food it senses, and the food sense is off for
    /// [`EAT_REST`] frames.
    pub eat_fade: bool,
    /// **Not the fix round's** (2026-10-10, never in [`WalkParts::ALL`]): a
    /// hunger bout ends with food, not with a job. Once need has fired, the
    /// ant stays in need until its hunger is back to 0; a job taken or kept
    /// in the bout has its hold ignored.
    pub hunger_holds: bool,
    /// **The placebo** (2026-10-10, never in [`WalkParts::ALL`]): the forage
    /// roll is drawn from its own stream ([`RNG_SLOT_NEEDS_PLACEBO`]) -- the
    /// same rule, another realisation. It changes nothing a rule could be
    /// credited with, so its move against the same seeds is the spread any
    /// other part's effect must clear.
    pub placebo: bool,
    /// **Not the fix round's** (2026-10-11, never in [`WalkParts::ALL`]; on
    /// in [`WalkParts::NEW_ANT`]): an ant whose head has not left a
    /// [`SEAL_ROAM`] box for [`ESCAPE_STALL`] decisions floods the air round
    /// its head ([`seal_flood`]); sealed -- no way out of the food sense's
    /// box -- it escapes whatever its hunger or its job ([`sealed_escape`]).
    pub sealed_out: bool,
    /// **Not the fix round's** (2026-10-11, never in [`WalkParts::ALL`]; on
    /// in [`WalkParts::NEW_ANT`]): a hungry ant sealed in
    /// ([`WalkParts::sealed_out`]'s flood) with a store load and nowhere to
    /// put it down eats it from its jaws ([`eat_from_jaws`]). Needs
    /// `sealed_out`, which alone sets the flag it reads: [`WalkParts::parse`]
    /// refuses it without.
    pub eat_held: bool,
}

impl WalkParts {
    pub const NONE: WalkParts = WalkParts {
        only_diggers: false,
        dig_job: false,
        clear: false,
        pace: false,
        give_up: false,
        lay_home: false,
        meal: false,
        won_stall: false,
        reach: false,
        eat_fade: false,
        hunger_holds: false,
        placebo: false,
        sealed_out: false,
        eat_held: false,
    };
    /// **The fix round's eight**, which is what `needsparts=all` has meant
    /// since 10-07; the parts added after it ([`WalkParts::reach`],
    /// [`WalkParts::eat_fade`], [`WalkParts::hunger_holds`],
    /// [`WalkParts::placebo`]) are named one by one.
    pub const ALL: WalkParts = WalkParts {
        only_diggers: true,
        dig_job: true,
        clear: true,
        pace: true,
        give_up: true,
        lay_home: true,
        meal: true,
        won_stall: true,
        reach: false,
        eat_fade: false,
        hunger_holds: false,
        placebo: false,
        sealed_out: false,
        eat_held: false,
    };
    /// **The new ant as it stands** (2026-10-11): `reach` (accepted as the
    /// baseline, owner 2026-10-10) and the `walled_in` switch's two parts,
    /// [`WalkParts::sealed_out`] and [`WalkParts::eat_held`], on. What a
    /// harness hands the walk when it names no parts (`deeptrace`'s
    /// `needsparts=`, `needscost`'s `walk` arm); `none` is slice 1 exactly.
    /// [`NeedsWalk::new`] still starts from [`WalkParts::NONE`], so the
    /// tests' scenes stay slice 1 unless they name a part. Measured in
    /// `needs-ant/resume-2026-10-10/results-sealed-4seed-2026-10-11.md`.
    pub const NEW_ANT: WalkParts = WalkParts {
        reach: true,
        sealed_out: true,
        eat_held: true,
        ..WalkParts::NONE
    };
    const NAMES: [&'static str; 14] = [
        "only_diggers",
        "dig_job",
        "clear",
        "pace",
        "give_up",
        "lay_home",
        "meal",
        "won_stall",
        "reach",
        "eat_fade",
        "hunger_holds",
        "placebo",
        "sealed_out",
        "eat_held",
    ];

    /// `all`, `none`, `new_ant` ([`WalkParts::NEW_ANT`]), `walled_in` (its
    /// switch's two parts, `sealed_out` and `eat_held`), or a comma list of
    /// the part names, as a harness names them. Anything else is refused rather than read as a default, so a
    /// typo cannot run one arm under another's label.
    pub fn parse(raw: &str) -> WalkParts {
        let mut p = WalkParts::NONE;
        for part in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match part {
                "all" => p = WalkParts::ALL,
                "none" => p = WalkParts::NONE,
                "new_ant" => p = WalkParts::NEW_ANT,
                "walled_in" => {
                    p.sealed_out = true;
                    p.eat_held = true;
                }
                "only_diggers" => p.only_diggers = true,
                "dig_job" => p.dig_job = true,
                "clear" => p.clear = true,
                "pace" => p.pace = true,
                "give_up" => p.give_up = true,
                "lay_home" => p.lay_home = true,
                "meal" => p.meal = true,
                "won_stall" => p.won_stall = true,
                "reach" => p.reach = true,
                "eat_fade" => p.eat_fade = true,
                "hunger_holds" => p.hunger_holds = true,
                "placebo" => p.placebo = true,
                "sealed_out" => p.sealed_out = true,
                "eat_held" => p.eat_held = true,
                other => panic!(
                    "needsparts={raw:?}: {other:?} is not all, none, new_ant, walled_in or one of {}",
                    WalkParts::NAMES.join(", ")
                ),
            }
        }
        // `eat_held` eats only once `sealed_out`'s flood has found the ant
        // sealed in, so alone it would run as the parts without it under a
        // label that names it: refused, as a typo is (code review of PR 684).
        if p.eat_held && !p.sealed_out {
            panic!("needsparts={raw:?}: eat_held needs sealed_out (it eats only where sealed_out has found the ant sealed in)");
        }
        p
    }

    /// The parts that are on, comma-separated, or `none`.
    pub fn label(self) -> String {
        let on = [
            self.only_diggers,
            self.dig_job,
            self.clear,
            self.pace,
            self.give_up,
            self.lay_home,
            self.meal,
            self.won_stall,
            self.reach,
            self.eat_fade,
            self.hunger_holds,
            self.placebo,
            self.sealed_out,
            self.eat_held,
        ];
        let names: Vec<&str> = WalkParts::NAMES
            .iter()
            .zip(on)
            .filter(|&(_, on)| on)
            .map(|(n, _)| *n)
            .collect();
        if names.is_empty() {
            "none".to_string()
        } else {
            names.join(",")
        }
    }
}

/// **The walk and everything it remembers**, held on [`World::needs`] so
/// nothing of it lives on `OrganismState`: the shipped ant's state, and so
/// its random stream, cannot be touched by it.
#[derive(Clone, Debug)]
pub struct NeedsWalk {
    pub mode: NeedsMode,
    /// The frame the walk was switched on, for the switch-time burn-in.
    pub since: u64,
    /// Each ant's memory under the walk, made on its first decision.
    minds: HashMap<OrganismId, Mind>,
    /// When the memories of the dead were last let go.
    pruned_at: u64,
    /// **The trace**: every decision of about one ant in this many, picked
    /// by a hash of its id ([`NeedsWalk::traces`]), is kept in
    /// [`NeedsWalk::rows`] for a harness to drain. 0 keeps none.
    ///
    /// **Never by the id's remainder**: the shipped storeroom makes every ant
    /// whose id is a multiple of 4 a nest worker for life (`caste=4`), so the
    /// first form of this trace, ids that are multiples of 20, followed the
    /// nest-worker caste and nobody else -- all 26 ants of the first slice-1
    /// trace (2026-10-07), whose foragers were all nest workers.
    pub trace_every: u32,
    /// ...or exactly these ants, when not empty (a harness's `only=` list).
    pub trace_ids: std::collections::HashSet<OrganismId>,
    pub rows: Vec<WalkRow>,
    /// What the walk did, summed over every ant, for a harness to read.
    pub counts: WalkCounts,
    /// **The guard's positive control**: needs never win, so every ant does
    /// its job or idles whatever its hunger. Never set outside a test.
    pub veto_needs: bool,
    /// The fix round's parts that are on ([`WalkParts`]); none by default.
    pub parts: WalkParts,
    /// **What `reach` hid, ant by ant**, for a harness to drain when it asks
    /// (`Some`): every hungry decision on which [`WalkParts::reach`] hid all
    /// the food in the box, its frame, the ant, and whether any of that food
    /// was joined to the head by air within [`AIR_CHECK`] cells -- food a way
    /// out of the box would have reached. Reads the world only.
    pub hidden_log: Option<Vec<(u64, OrganismId, bool)>>,
    /// **Every sealed event, ant by ant** ([`WalkParts::sealed_out`]), for a
    /// harness to drain when it asks (`Some`): found sealed, and freed --
    /// the air round the head reaches the box's rim again, after a step
    /// ([`SEAL_FREED`]) or standing ([`SEAL_OPENED`]). A death is the
    /// harness's to join from its own log.
    pub sealed_log: Option<Vec<SealRow>>,
}

/// What a [`SealRow`] records: found sealed.
pub const SEAL_FOUND: u8 = 0;
/// ...freed after a step of its own.
pub const SEAL_FREED: u8 = 1;
/// ...found open again without a step (another ant, or the ground, opened it).
pub const SEAL_OPENED: u8 = 2;

/// **One sealed event's start or end** ([`NeedsWalk::sealed_log`]).
#[derive(Clone, Copy, Debug)]
pub struct SealRow {
    pub frame: u64,
    pub id: OrganismId,
    /// [`SEAL_FOUND`], [`SEAL_FREED`] or [`SEAL_OPENED`].
    pub event: u8,
    pub at: (i32, i32),
    /// Cells the air round the head reached (the pocket, to the box's size).
    pub cells: u16,
    /// What it held: 0 nothing, 1 soil, 2 a store load.
    pub hold: u8,
    pub energy: f32,
    pub crop: f32,
    /// Cuts it made sealed this event, to the end row.
    pub cuts: u16,
}

impl NeedsWalk {
    /// Drop one ant's memory, so it is walked afresh: the player's ant while
    /// she is steered (`creature_tick`), so a stall or a seal from before
    /// does not come back with her (code review of PR 684).
    pub fn forget(&mut self, organism: OrganismId) {
        self.minds.remove(&organism);
    }

    pub fn new(mode: NeedsMode, frame: u64) -> NeedsWalk {
        NeedsWalk {
            mode,
            since: frame,
            minds: HashMap::new(),
            pruned_at: frame,
            trace_every: 0,
            trace_ids: std::collections::HashSet::new(),
            rows: Vec::new(),
            counts: WalkCounts::default(),
            veto_needs: false,
            parts: WalkParts::NONE,
            hidden_log: None,
            sealed_log: None,
        }
    }

    /// **Whether this ant's decisions go in the trace**: one of
    /// [`NeedsWalk::trace_ids`] if any are named, else one ant in about
    /// [`NeedsWalk::trace_every`] by a hash of its id that no id-keyed rule
    /// (a caste, a founder share) lines up with.
    pub fn traces(&self, organism: OrganismId) -> bool {
        if !self.trace_ids.is_empty() {
            return self.trace_ids.contains(&organism);
        }
        let mixed = (organism as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32;
        self.trace_every > 0 && mixed.is_multiple_of(u64::from(self.trace_every))
    }

    /// How many ants the walk is remembering (live ones, to within
    /// [`PRUNE_EVERY`] frames).
    pub fn minds(&self) -> usize {
        self.minds.len()
    }

    /// The drive and job of every ant the walk remembers, for a harness's
    /// census of who is doing what.
    pub fn census(&self) -> impl Iterator<Item = (OrganismId, Drive, Job)> + '_ {
        self.minds.iter().map(|(&id, m)| (id, m.drive, m.job))
    }

    /// **One ant's walk as it last decided**, for a harness that writes it
    /// beside the ant's position: `None` for an ant that has not decided
    /// under the walk yet.
    pub fn view(&self, organism: OrganismId) -> Option<WalkView> {
        let m = self.minds.get(&organism)?;
        Some(WalkView {
            drive: m.drive,
            job: m.job,
            hunger: m.hunger,
            hold: m.hold,
            forage: m.forage,
            threshold: FORAGE_THRESHOLD * m.jitter,
            pref: m.pref,
            p_move: m.p_move,
            stall: m.stall,
            meet: m.meet,
            glut: m.glut,
            door: m.door,
        })
    }
}

/// **What [`NeedsWalk::view`] shows of one ant**: its last decision's drive,
/// job, hunger and hold, its forage stimulus against its own threshold, its
/// preferred depth, its step chance, how long its way out has stalled, the
/// meetings and glut its stimulus is made of, and its door vector's origin.
#[derive(Clone, Copy, Debug)]
pub struct WalkView {
    pub drive: Drive,
    pub job: Job,
    pub hunger: f32,
    pub hold: f32,
    pub forage: f32,
    pub threshold: f32,
    pub pref: f32,
    pub p_move: f32,
    pub stall: u16,
    pub meet: f32,
    pub glut: f32,
    pub door: Option<(i32, i32)>,
}

/// **What wins a decision**: a need, the job, or idling. Set before `act`
/// and read by the kinesis and the step after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Drive {
    /// Hungry, with food in reach: go to it and eat.
    Eat,
    /// Hungry inside, nothing in reach: out by the way out.
    Out,
    /// Hungry inside and the way out has stalled against soil: cut.
    Escape,
    /// Hungry outside, nothing in reach: search, as a hungry scout does.
    Seek,
    /// On the forage job, empty, going out.
    Forage,
    /// Food from a trip in the crop: carry it home.
    Carry,
    /// A pellet in the jaws: take it out.
    Haul,
    /// Its pellet down, walking back to the face it cut.
    Dig,
    /// Idle outside or in the spoil mound: walk home.
    Home,
    /// Idle inside: no pull; the kinesis decides where it stops.
    #[default]
    Rest,
}

impl Drive {
    pub const ALL: [Drive; DRIVES] = [
        Drive::Eat,
        Drive::Out,
        Drive::Escape,
        Drive::Seek,
        Drive::Forage,
        Drive::Carry,
        Drive::Haul,
        Drive::Dig,
        Drive::Home,
        Drive::Rest,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Drive::Eat => "eat",
            Drive::Out => "out",
            Drive::Escape => "escape",
            Drive::Seek => "seek",
            Drive::Forage => "forage",
            Drive::Carry => "carry",
            Drive::Haul => "haul",
            Drive::Dig => "dig",
            Drive::Home => "home",
            Drive::Rest => "rest",
        }
    }

    /// A need, as against a job or idling.
    pub fn is_need(self) -> bool {
        matches!(self, Drive::Eat | Drive::Out | Drive::Escape | Drive::Seek)
    }
}

pub const DRIVES: usize = 10;

/// **The ant's job**: what it does when no need wins. Kept across a need, so
/// an ant that went to eat goes back to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Job {
    #[default]
    Idle,
    Forage,
    Haul,
    Dig,
}

impl Job {
    pub fn label(self) -> &'static str {
        match self {
            Job::Idle => "idle",
            Job::Forage => "forage",
            Job::Haul => "haul",
            Job::Dig => "dig",
        }
    }
}

/// **What the walk did**, summed over every ant since the switch.
#[derive(Clone, Copy, Debug, Default)]
pub struct WalkCounts {
    /// Decisions won by each drive, in [`Drive::ALL`]'s order.
    pub decisions: [u64; DRIVES],
    /// Idle ants that took the forage job, and foragers home that quit it.
    pub took_forage: u64,
    pub quit_forage: u64,
    /// A need won over a job that was not idle.
    pub need_over_job: u64,
    /// Store loads a hungry ant put down to eat, and pellets put down.
    pub ate_held: u64,
    pub pellets_down: u64,
    /// Cuts made escaping, and pellets packed behind while escaping.
    pub escape_cuts: u64,
    pub escape_packs: u64,
    /// Foragers that unloaded beside food already lying uneaten.
    pub gluts: u64,
    /// **Cuts `act` made, by the drive that made them**, in
    /// [`Drive::ALL`]'s order: the share the dig job makes is the first check
    /// on [`WalkParts::only_diggers`]. Escape's and the door's own cuts are
    /// counted apart, in `escape_cuts` and `clear_cuts`.
    pub cuts: [u64; DRIVES],
    /// The fix round's, each zero with its part off: idle ants that took the
    /// dig job on its stimulus, dig jobs ended because the stimulus fell or
    /// because patience ran out with no cut ([`WalkParts::dig_job`]); cuts
    /// toward home at the door ([`WalkParts::clear`]); forage jobs ended
    /// outside when the scout gave up ([`WalkParts::give_up`]); a forager's
    /// decisions turned home to lay ([`WalkParts::lay_home`]); decisions a
    /// home meal was kept from a drop the brain asked for
    /// ([`WalkParts::meal`]); and way-out decisions the ant chose to stay,
    /// not counted as a stall ([`WalkParts::won_stall`]).
    pub dig_took: u64,
    pub dig_quit: u64,
    pub dig_tired: u64,
    pub clear_cuts: u64,
    pub gave_up: u64,
    pub lay_walks: u64,
    pub meals_kept: u64,
    pub stays_not_stalls: u64,
    /// **Food sealed in soil**, each zero with its part off: hungry decisions
    /// on which the food sense found food in its box but none it could reach
    /// ([`WalkParts::reach`]); eat drives given up, and those given up by an
    /// ant that had already given up once this hunger bout (the loop the
    /// review asked to see); hungry decisions the sense was off in the
    /// window; and windows ended early by a meal or by reaching the open air
    /// ([`WalkParts::eat_fade`]).
    pub reach_hidden: u64,
    pub eat_gave_up: u64,
    pub eat_gave_up_again: u64,
    pub eat_rest: u64,
    pub eat_rest_ate: u64,
    pub eat_rest_out: u64,
    /// ...and of `reach_hidden`, the decisions on which some of the hidden
    /// food was joined to the head by air within [`AIR_CHECK`] cells: hidden
    /// round the box's edge, not sealed.
    pub reach_hidden_air: u64,
    /// **The hunger bout** ([`WalkParts::hunger_holds`]), each zero with the
    /// part off: bouts started; bouts ended fed (hunger back to 0), inside
    /// and in the open air; bouts ended by death (counted when the dead
    /// ant's memory is pruned); and decisions in need only because the bout
    /// held, a job's hold ignored.
    pub bouts: u64,
    pub bouts_fed: u64,
    pub bouts_fed_outside: u64,
    pub bouts_died: u64,
    pub hold_ignored: u64,
    /// **Sealed in** ([`WalkParts::sealed_out`], [`WalkParts::eat_held`]),
    /// each zero with its parts off: floods run, events found sealed, events
    /// freed (by its own step or otherwise), escape cuts and packs made
    /// sealed, cuts turned off the way out's heading because that cell could
    /// not be cut, pellets and store loads put down in the pocket, cuts of a
    /// cell already cut this event, store loads eaten from the jaws, and
    /// eats refused (the crop holds another food).
    pub seal_floods: u64,
    pub sealed_found: u64,
    pub sealed_freed: u64,
    pub sealed_opened: u64,
    pub sealed_cuts: u64,
    pub sealed_packs: u64,
    pub seal_aim_turned: u64,
    pub sealed_drops: u64,
    pub sealed_load_down: u64,
    pub seal_recut: u64,
    pub ate_held_jaws: u64,
    pub eat_held_refused: u64,
    /// Decisions holding soil with food in the crop, whatever the parts:
    /// `act` cannot put that pellet down (its drop branch returns before its
    /// soil branch), so the ant hauls it until the crop drains -- the state
    /// an eat then a cut out leaves (the rev 2 review, §3).
    pub laden_soil: u64,
}

/// What a decision cut, for the trace ([`WalkRow::cut`]).
pub const CUT_NONE: u8 = 0;
/// `act`'s own dig.
pub const CUT_ACT: u8 = 1;
/// Escape's cut, and the pellet it packed behind instead.
pub const CUT_ESCAPE: u8 = 2;
pub const CUT_PACK: u8 = 3;
/// The door cut ([`WalkParts::clear`]).
pub const CUT_CLEAR: u8 = 4;

/// **One decision of a traced ant** ([`NeedsWalk::trace_every`]).
#[derive(Clone, Copy, Debug)]
pub struct WalkRow {
    pub frame: u64,
    pub id: OrganismId,
    /// The head when it decided, and after its step.
    pub at: (i32, i32),
    pub to: (i32, i32),
    pub drive: Drive,
    pub job: Job,
    pub energy: f32,
    pub crop: f32,
    pub hunger: f32,
    pub hold: f32,
    /// The forage stimulus and this ant's threshold for it.
    pub forage: f32,
    pub threshold: f32,
    /// Rows below the founding ground (the stand-in depth) and its own
    /// preferred depth.
    pub depth: i32,
    pub pref: f32,
    pub p_move: f32,
    pub moved: bool,
    pub stall: u16,
    /// Where the drive's pull aimed, if it had one.
    pub target: Option<(i32, i32)>,
    /// **The step roll was won**: the ant meant to step, whether or not it
    /// got anywhere. `moved` alone cannot tell a chosen stay from a step
    /// that went nowhere.
    pub won: bool,
    /// What this decision cut: [`CUT_NONE`], [`CUT_ACT`], [`CUT_ESCAPE`],
    /// [`CUT_PACK`] or [`CUT_CLEAR`].
    pub cut: u8,
    /// **The scout's excursion after the step** (`OrganismState::
    /// scout_patience`, `scout_home`, `scout_dark`, `scout_for`): read for
    /// the trace of hungry ants piling on the mound face under
    /// `hunger_holds` (2026-10-10), where Seek's home pull was the suspect.
    pub scout_patience: f32,
    pub scout_home: bool,
    pub scout_dark: bool,
    pub scout_for: (i32, i32),
    /// Why an escape did not cut ([`ESC_WHY_NAMES`]).
    pub esc_why: u8,
    /// What broke `walled_in` for a pellet holder (bits, [`walled_in_why`]).
    pub esc_nb: u8,
    /// Sealed in, by [`WalkParts::sealed_out`]'s last flood.
    pub sealed: bool,
}

/// **One ant's memory under the walk.** Set at its first decision from what
/// the shipped ant was doing ([`Mind::at_switch`], the switch-time table),
/// then kept here.
#[derive(Clone, Copy, Debug)]
struct Mind {
    job: Job,
    /// The frame the job was last taken or renewed.
    since: u64,
    /// Its own factor on every threshold, drawn once from its id.
    jitter: f32,
    /// Its preferred resting depth, rows below the founding ground.
    pref: f32,
    /// **The door vector's origin**: the last open-air cell its head stood
    /// in. `None` for an ant that has not been out since the switch.
    door: Option<(i32, i32)>,
    /// The way out's progress: which measure, the best it has reached, and
    /// how many decisions since it last got better.
    out_kind: u8,
    out_best: i32,
    stall: u16,
    /// Meetings with foragers home with food, leaking at [`MEET_TAU`].
    meet: f32,
    met_seen: u64,
    meet_at: u64,
    /// Food lying uneaten where it last unloaded, 1 fading at [`GLUT_TAU`].
    glut: f32,
    glut_at: u64,
    /// **The way home's progress** ([`WalkParts::clear`]): the pull's
    /// target it is measured to, the nearest it has come, and how many
    /// decisions that won the step roll have not come nearer.
    home_for: Option<(i32, i32)>,
    home_best: f32,
    home_stall: u16,
    /// No dig job taken on its stimulus before this frame: set when one ran
    /// out of patience ([`WalkParts::dig_job`]).
    dig_rest_until: u64,
    /// Gave up foraging outside and has not been home since
    /// ([`WalkParts::give_up`]): it takes no forage job until it is.
    gave_up: bool,
    /// **The eat drive's progress** ([`WalkParts::eat_fade`]): the nearest
    /// it has come to the food it senses, whatever cell, how many decisions
    /// that won the step roll have not come nearer, the frame before which
    /// its food sense is off, give-ups this hunger bout, and its energy plus
    /// crop at its last decision, whose rise is a meal.
    eat_best: f32,
    eat_stall: u16,
    eat_rest_until: u64,
    eat_bouts: u16,
    fuel: f32,
    /// In a hunger bout ([`WalkParts::hunger_holds`]): set when need fires,
    /// cleared when hunger is back to 0. False at the switch.
    bout: bool,
    /// This decision's: the drive, its hunger and hold, the forage
    /// stimulus, food it sensed, the step chance, what it cut.
    drive: Drive,
    hunger: f32,
    hold: f32,
    forage: f32,
    food: Option<(i32, i32)>,
    p_move: f32,
    cut: u8,
    /// Why an escape that fired did not cut ([`ESC_WHY_NAMES`]), for the
    /// trace; 0 when it cut or did not escape.
    esc_why: u8,
    /// What broke `walled_in` for a pellet holder ([`walled_in_why`]).
    esc_nb: u8,
    /// **Sealed in** ([`WalkParts::sealed_out`]): decisions in a row without
    /// a step; whether the last flood found no way out of the box; the cells
    /// cut sealed this event, the last of them, and how many.
    seal_stall: u16,
    seal_from: (i32, i32),
    sealed: bool,
    seal_cuts: [(i32, i32); 4],
    seal_cut_n: u16,
    last_cut: Option<(i32, i32)>,
    /// Where it stood when it decided, for the trace.
    at: (i32, i32),
}

impl Mind {
    /// **The switch-time table**: what each item of the walk's memory is set
    /// to on an ant's first decision under it.
    ///
    /// | Item | Set from |
    /// |---|---|
    /// | job | haul with a pellet in the jaws; forage with food in the crop, or empty out in the open having foraged before; dig with a face to go back to; else idle |
    /// | threshold jitter, preferred depth | drawn from its id, once |
    /// | door vector | its head, if it stands in the open; else blank |
    /// | way-out progress | blank |
    /// | meetings | 1 if it met a forager home with food within the return window, else 0 |
    /// | food lying where it unloads | 0 |
    fn at_switch(
        world: &World,
        organism: OrganismId,
        st: &crate::sim::organism::OrganismState,
        head: (i32, i32),
        outside: bool,
    ) -> Mind {
        let laden = st.crop.is_some_and(|c| c.worth() > 0.0);
        let job = if st.spoil.is_some_and(|s| !s.store) {
            Job::Haul
        } else if laden || (outside && st.foraged) {
            Job::Forage
        } else if st.dig_return.is_some() {
            Job::Dig
        } else {
            Job::Idle
        };
        let mut r = rng::stream(world.seed, organism as u64, 0, RNG_SLOT_NEEDS_JITTER);
        let jitter = (JITTER * (2.0 * r.unit_f32() - 1.0)).exp();
        let pref = PREF_DEPTH + PREF_SPREAD * (2.0 * r.unit_f32() - 1.0);
        let met = st.return_met > 0 && (world.frame.saturating_sub(st.return_met) as f32) < return_window();
        Mind {
            job,
            since: world.frame,
            jitter,
            pref,
            door: outside.then_some(head),
            out_kind: OUT_NONE,
            out_best: i32::MAX,
            stall: 0,
            meet: if met { 1.0 } else { 0.0 },
            met_seen: st.return_met,
            meet_at: world.frame,
            glut: 0.0,
            glut_at: world.frame,
            home_for: None,
            home_best: f32::INFINITY,
            home_stall: 0,
            dig_rest_until: 0,
            gave_up: false,
            eat_best: f32::INFINITY,
            eat_stall: 0,
            eat_rest_until: 0,
            eat_bouts: 0,
            fuel: st.energy + st.crop.map_or(0.0, |c| c.worth().max(0.0)),
            bout: false,
            drive: Drive::Rest,
            hunger: 0.0,
            hold: 0.0,
            forage: 0.0,
            food: None,
            p_move: f32::NAN,
            cut: CUT_NONE,
            esc_why: 0,
            esc_nb: 0,
            seal_stall: 0,
            seal_from: head,
            sealed: false,
            seal_cuts: [(i32::MIN, i32::MIN); 4],
            seal_cut_n: 0,
            last_cut: None,
            at: head,
        }
    }
}

/// The random stream for the walk's own rolls (taking a job), keyed on the
/// ant and the frame as the shipped streams are. Slots 0-11 are
/// `creature.rs`'s.
const RNG_SLOT_NEEDS: u64 = 12;
/// ...and the one each ant's jitter is drawn from, once, at frame 0.
const RNG_SLOT_NEEDS_JITTER: u64 = 13;
/// ...and the dig job's roll ([`WalkParts::dig_job`]), apart from the forage
/// roll's so that turning the part on leaves who takes the forage job as it
/// was.
const RNG_SLOT_NEEDS_DIG: u64 = 14;
/// ...and the forage roll's stand-in under [`WalkParts::placebo`]: the same
/// roll, another stream.
const RNG_SLOT_NEEDS_PLACEBO: u64 = 15;

/// **Hunger's ramp**: 0 at and above this share of the budget (200 J), 1 at
/// and below [`HUNGER_FULL`], smooth between. The design's starting values;
/// newborns hatch at about half the budget, so they start mildly hungry.
pub const HUNGER_ONSET: f32 = 0.6;
pub const HUNGER_FULL: f32 = 0.25;

/// **The most a job can hold an ant against hunger**: below hunger's urge at
/// 30% of the budget (0.94), so a starving ant always leaves its job. A job
/// holds an ant by its patience times this, so a job that is stuck holds it
/// by nothing.
pub const HOLD_CEILING: f32 = 0.8;

/// Decisions on the way out with no progress, pushing at soil, before the
/// ant cuts: about 180 frames at the ant's six-frame decision. A hungry ant
/// at 30% has about 1,400 frames to live.
pub const ESCAPE_STALL: u16 = 30;
/// How often a still ant floods again for the seal ([`WalkParts::sealed_out`]),
/// in decisions, after its first at [`ESCAPE_STALL`].
const SEAL_EVERY: u16 = 10;
/// ...and how far the head may wander, Chebyshev, without its count starting
/// again: a 7x7 box, the size of most of the traced pockets (87 of 93 held 39
/// cells or fewer).
const SEAL_ROAM: i32 = 3;
/// Where [`Mind::seal_stall`] steps back by [`SEAL_EVERY`] rather than
/// growing: far past any cadence's start, and below `u16::MAX`.
const SEAL_STALL_TOP: u16 = ESCAPE_STALL + 6_000 * SEAL_EVERY;

/// **The forage stimulus**: a floor a fed ant still feels, so fed scouts
/// leave a colony that has no returns to meet (the design's founding-bed
/// risk), and the leaky count of meetings with foragers home with food,
/// which leaks over the shipped return window. The floor takes an idle ant
/// at threshold 1 about once in 2,500 decisions (15,000 frames); one
/// meeting a window takes it about every other decision.
pub const FORAGE_FLOOR: f32 = 0.02;
pub const FORAGE_THRESHOLD: f32 = 1.0;
const MEET_TAU: f32 = 1_400.0;
/// A forager stays on its job at least this long after taking or renewing
/// it, so it is out of the door before it asks again.
const FORAGE_DWELL: u64 = 600;
/// **More than enough food** (the owner, 2026-10-06): a forager that
/// unloads with this many food cells already lying within 3 cells of its
/// head remembers a glut, which brakes its forage stimulus and fades over
/// [`GLUT_TAU`]. Real foragers regulate on their own crop's unloading, not on
/// a colony total (Greenwald, Baltiansky & Feinerman 2018, eLife 7:e31730).
const GLUT_FOOD: u32 = 6;
const GLUT_TAU: f32 = 6_000.0;
/// How hard a forager going out pushes out: today's `always` drive.
const FORAGE_WANT: f32 = 1.0;

/// Each ant's thresholds are scaled by `exp(JITTER * u)`, `u` uniform in
/// [-1, 1): a factor from 0.67 to 1.5. Mutation is off while measuring, so
/// without it every ant would be the founder's copy.
const JITTER: f32 = 0.405;

/// **The preferred resting depth**: rows below the founding ground, plus or
/// minus [`PREF_SPREAD`] by ant. "Deep" in the scorecard is more than 10.
pub const PREF_DEPTH: f32 = 14.0;
const PREF_SPREAD: f32 = 3.0;
/// **The kinesis**: an idle ant steps at [`P_SHALLOW`] when [`DEPTH_BAND`]
/// rows or more above its preferred depth, at [`P_DEEP`] at or below it, and
/// in between linearly. `P_DEEP` is a fed ant's step rate today (about 3% of
/// its decisions); `P_SHALLOW` is a guess the first trace checks.
const P_SHALLOW: f32 = 0.5;
const P_DEEP: f32 = 0.03;
const DEPTH_BAND: f32 = 6.0;
/// An idle ant outside or in the mound, walking home.
const P_HOME: f32 = 0.5;

/// The pull to food a hungry ant senses, twice the home pull (design).
const FOOD_GAIN: f32 = 2.0;
/// How far the food sense reaches, in cells (a 13x13 box, as `larva_scent`).
const FOOD_REACH: i32 = 6;
/// The food sense's box, a side.
const FOOD_BOX: usize = (2 * FOOD_REACH + 1) as usize;
/// **The eat drive gives up** ([`WalkParts::eat_fade`]) after this many
/// decisions that won the step roll and came no nearer the food it senses:
/// the way out's own stall, about 180 frames at the ant's six-frame decision.
const EAT_STALL: u16 = ESCAPE_STALL;
/// ...and its food sense is then off this many frames, unless a meal or the
/// open air ends it first: about 100 decisions, long enough for the way out
/// to stall and escape to fire (30 decisions) more than once, and under half
/// what an ant at 30% of its budget has to live (about 1,400 frames). As long
/// as the stall, the review's reading of the proposal's first form, the ant
/// would turn back to the eat drive just as escape could fire.
const EAT_REST: u64 = 600;
/// The pull out never falls below this share of the home pull, however
/// mildly hungry the ant.
const OUT_GAIN_MIN: f32 = 0.3;

/// **The dig job's stimulus and threshold** ([`WalkParts::dig_job`]): soil
/// at the face counts 1 and each other adult animal with a cell within
/// [`DIG_REACH`] of the head counts 1, against this threshold times the ant's
/// jitter, taken with probability `r^2 / (1 + r^2)` as the forage job is. At
/// 6 an ant alone at a wall takes the job on about one decision in 37, one
/// with three nestmates round it and no soil ahead on one in 5, and one with
/// both on one in 3. A first value, for the
/// fix round's smoke run to check against the shipped walk's cuts below the
/// ground line (199 over 60-100k on seed 1, `results.md` step 1).
pub const DIG_THRESHOLD: f32 = 6.0;
const DIG_REACH: i32 = 2;
/// A dig job taken on its stimulus is held at least this long, so the ant
/// can cut where it stands, and then while its stimulus holds half the
/// threshold (the forage job's hysteresis).
const DIG_DWELL: u64 = 60;
/// ...and ends, whatever its stimulus, after this long with no cut: the
/// design's "or patience runs out". It is not taken again for as long.
const DIG_PATIENCE: u64 = 1_200;

/// **The door cut** ([`WalkParts::clear`]): an empty ant walking home whose
/// last this many decisions that won the step roll brought it no nearer
/// home -- escape's own count -- may cut the soil cell toward home, if that
/// cell is within [`CLEAR_COLS`] columns of the founding cut's and from
/// [`CLEAR_ABOVE`] rows over the founding ground to [`CLEAR_BELOW`] under
/// it. Every one of seed 1's six door seals over 50-99k had a cell of its
/// thinnest wall in that box (3-17 rows over the ground, `results.md` step
/// 4); outside it, carriers circling on the mound would cut the mound's
/// walls apart, which is fault 1 again (Nest building's review, must-fix 2).
const CLEAR_STALL: u16 = 30;
/// Nearer by less than this is no progress (in cells, straight-line).
const HOME_PROGRESS: f32 = 0.1;
const CLEAR_COLS: i32 = 5;
const CLEAR_ABOVE: i32 = 24;
const CLEAR_BELOW: i32 = 3;

/// Memories of ants no longer alive are let go this often.
const PRUNE_EVERY: u64 = 5_000;

/// Which measure the way out's progress is on ([`Mind::out_kind`]).
const OUT_NONE: u8 = 0;
const OUT_NEST_WAY: u8 = 1;
const OUT_MOUND_WAY: u8 = 2;
const OUT_DOOR: u8 = 3;
const OUT_UP: u8 = 4;

/// **Hunger's urge** at `have` (energy plus crop, as a share of the budget).
pub fn hunger_urge(have: f32) -> f32 {
    let t = ((HUNGER_ONSET - have) / (HUNGER_ONSET - HUNGER_FULL)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mind_of(world: &World, organism: OrganismId) -> Option<Mind> {
    world.needs.as_ref()?.minds.get(&organism).copied()
}

fn keep_mind(world: &mut World, organism: OrganismId, mind: Mind) {
    if let Some(n) = world.needs.as_mut() {
        n.minds.insert(organism, mind);
    }
}

/// **Before the move: what the jaws and the mouth do.** Pass-through runs
/// `act` on the brain's urges, as today's ant does.
#[allow(clippy::too_many_arguments)]
pub(super) fn act(
    world: &mut World,
    x: i32,
    y: i32,
    organism: OrganismId,
    def: &CreatureDef,
    inputs: &[f32; brain::BRAIN_INPUTS],
    outputs: &[f32; brain::BRAIN_OUTPUTS],
    draw: &mut rng::Rng,
) -> Did {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => super::act(world, x, y, organism, def, outputs, draw),
        Some(NeedsMode::Walk) => walk_act(world, (x, y), organism, def, inputs, outputs, draw),
    }
}

/// **Step or stay**: the chance this decision steps at all. Pass-through
/// hands today's chance back.
pub(super) fn kinesis(
    world: &mut World,
    organism: OrganismId,
    _def: &CreatureDef,
    head: (i32, i32),
    _inputs: &[f32; brain::BRAIN_INPUTS],
    p_move: f32,
) -> f32 {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => p_move,
        Some(NeedsMode::Walk) => {
            let Some(mut mind) = mind_of(world, organism) else {
                return p_move;
            };
            let parts = world.needs.as_ref().map_or(WalkParts::NONE, |n| n.parts);
            let p = walk_kinesis(world, head, &mind, p_move, parts);
            mind.p_move = p;
            keep_mind(world, organism, mind);
            p
        }
    }
}

/// **The step**, once the roll said step. Pass-through takes today's step:
/// the chooser's, or `step_chain`'s with the chooser off.
#[allow(clippy::too_many_arguments)]
pub(super) fn step(
    world: &mut World,
    organism: OrganismId,
    heading: u8,
    outputs: &[f32; brain::BRAIN_OUTPUTS],
    def: &CreatureDef,
    draw: &mut rng::Rng,
    chooser: Chooser,
) -> bool {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => {
            if chooser == Chooser::Off {
                step_chain(world, organism, heading, outputs, def, draw)
            } else {
                chooser_step(world, organism, heading, outputs, def, draw, chooser)
            }
        }
        Some(NeedsMode::Walk) => walk_step(world, organism, heading, outputs, def, draw),
    }
}

/// **After the move**: what the walk remembers of this tick. Pass-through
/// remembers nothing. `won` is whether the step roll was won -- the ant
/// meant to step -- which a stall must tell apart from a chosen stay
/// ([`WalkParts::won_stall`]); a fall or a pack takes no roll and reads
/// false.
pub(super) fn after(world: &mut World, organism: OrganismId, _def: &CreatureDef, moved: bool, won: bool) {
    match world.needs.as_ref().map(|n| n.mode) {
        Some(NeedsMode::Passthrough) | None => {}
        Some(NeedsMode::Walk) => walk_after(world, organism, moved, won),
    }
}

// --- the walk: act ------------------------------------------------------

/// **The needs, the job, and what the jaws do**, for one decision.
fn walk_act(
    world: &mut World,
    head: (i32, i32),
    organism: OrganismId,
    def: &CreatureDef,
    inputs: &[f32; brain::BRAIN_INPUTS],
    outputs: &[f32; brain::BRAIN_OUTPUTS],
    draw: &mut rng::Rng,
) -> Did {
    use brain::BrainOutput as O;
    prune(world);
    let frame = world.frame;
    let Some(st) = world.organism(organism) else {
        return super::act(world, head.0, head.1, organism, def, outputs, draw);
    };
    let start = def.start_energy.max(1.0);
    let crop_worth = st.crop.map_or(0.0, |c| c.worth().max(0.0));
    let fuel = st.energy + crop_worth;
    let hunger = hunger_urge(fuel / start);
    let spoil = st.spoil;
    let laden = crop_worth > 0.0 && !carries_lunch(world, st);
    let trip = st.trip_load;
    let face = st.dig_return.is_some();
    let return_met = st.return_met;
    let (home_patience, scout_patience) = (st.home_patience, st.scout_patience);
    let surface = world
        .nearest_nest_site(head.0, head.1)
        .map(|i| world.nest_sites[i].surface);
    let below = surface.is_some_and(|s| head.1 > s);
    let covered = under_cover(world, head.0, head.1);
    let outside = !below && !covered;
    let at_home = inputs[brain::BrainInput::AtNest as usize] > 0.0;
    // The fix round's reads of the ant ([`WalkParts`]), taken while its
    // state is borrowed; each is false with its part off.
    let parts = world.needs.as_ref().map_or(WalkParts::NONE, |n| n.parts);
    let ready_lay = parts.lay_home && ready_to_lay(world, def, st);
    let keeps_meal = parts.meal && keeps_home_meal(st, def);
    // The scout's patience ran out on this excursion: its memory is for the
    // home it is out from now, not one it left before.
    let scout_spent = parts.give_up && st.scout_home && st.scout_for == home_target(world, st);
    let heading = st.heading;
    let mut mind = mind_of(world, organism).unwrap_or_else(|| Mind::at_switch(world, organism, st, head, outside));
    mind.at = head;
    mind.food = None;
    mind.hunger = hunger;
    mind.cut = CUT_NONE;
    mind.esc_why = 0;
    mind.esc_nb = 0;
    if at_home {
        mind.gave_up = false;
    }
    if crop_worth > 0.0 && spoil.is_some_and(|s| !s.store) {
        world.needs.as_mut().expect("walking").counts.laden_soil += 1;
    }

    // --- memory ---------------------------------------------------------
    if outside {
        mind.door = Some(head);
    }
    // **The eat drive's memory** ([`WalkParts::eat_fade`]): a meal (energy
    // and crop up since its last decision) or the open air starts its
    // progress again and ends a window its food sense was off in.
    if parts.eat_fade {
        let ate = fuel > mind.fuel;
        if ate || outside {
            if frame < mind.eat_rest_until {
                let c = &mut world.needs.as_mut().expect("walking").counts;
                if ate {
                    c.eat_rest_ate += 1;
                } else {
                    c.eat_rest_out += 1;
                }
                mind.eat_rest_until = 0;
            }
            mind.eat_best = f32::INFINITY;
            mind.eat_stall = 0;
        }
    }
    mind.fuel = fuel;
    let dt = frame.saturating_sub(mind.meet_at) as f32;
    mind.meet *= (-dt / MEET_TAU).exp();
    mind.meet_at = frame;
    if return_met > mind.met_seen {
        mind.meet += 1.0;
        mind.met_seen = return_met;
    }
    let dt = frame.saturating_sub(mind.glut_at) as f32;
    mind.glut *= (-dt / GLUT_TAU).exp();
    mind.glut_at = frame;
    let threshold = FORAGE_THRESHOLD * mind.jitter;
    mind.forage = (FORAGE_FLOOR + mind.meet) * (1.0 - mind.glut).max(0.0);

    // --- the job --------------------------------------------------------
    // Forced by what the ant holds or remembers first: a pellet is hauled,
    // a trip's food (or any food, outside the nest) carried home, a face
    // walked back to. Then foraging, by its stimulus against the threshold.
    let carrying_home = laden && (trip || !below);
    // **The dig job's stimulus** ([`WalkParts::dig_job`]), read only where
    // the job can be taken or held on it: inside the dug nest, empty.
    let can_dig = parts.dig_job && below && !laden && spoil.is_none();
    let dig_threshold = DIG_THRESHOLD * mind.jitter;
    let dig_stim = if can_dig {
        dig_stimulus(world, organism, def, head, heading)
    } else {
        0.0
    };
    let job = if spoil.is_some_and(|s| !s.store) {
        Job::Haul
    } else if carrying_home {
        Job::Forage
    } else if face && spoil.is_none() && !laden {
        Job::Dig
    } else {
        match mind.job {
            Job::Forage if at_home && frame.saturating_sub(mind.since) >= FORAGE_DWELL => {
                // Home and empty: stay on while the stimulus holds half the
                // threshold (the design's hysteresis), else stop.
                if mind.forage >= 0.5 * threshold {
                    mind.since = frame;
                    Job::Forage
                } else {
                    world.needs.as_mut().expect("walking").counts.quit_forage += 1;
                    Job::Idle
                }
            }
            // **The scout's patience has run out** ([`WalkParts::give_up`]):
            // the design's "or patience runs out", which slice 1 kept and
            // never read, so a forager that found nothing stayed out for
            // good (`results.md` step 8). It stops, walks home idle, and
            // takes the job again only from home.
            Job::Forage if scout_spent && outside && !laden => {
                mind.gave_up = true;
                world.needs.as_mut().expect("walking").counts.gave_up += 1;
                Job::Idle
            }
            Job::Forage => Job::Forage,
            // **A dig job taken on its stimulus, with no face yet**
            // ([`WalkParts::dig_job`]): held for its dwell, then while its
            // stimulus holds half the threshold, and ended by patience --
            // so long with no cut -- whatever the stimulus. An ant whose cut
            // left it a face is held by the face instead, above.
            Job::Dig if can_dig => {
                let on = frame.saturating_sub(mind.since);
                if on > DIG_PATIENCE {
                    mind.dig_rest_until = frame + DIG_PATIENCE;
                    world.needs.as_mut().expect("walking").counts.dig_tired += 1;
                    Job::Idle
                } else if on < DIG_DWELL || dig_stim >= 0.5 * dig_threshold {
                    Job::Dig
                } else {
                    world.needs.as_mut().expect("walking").counts.dig_quit += 1;
                    Job::Idle
                }
            }
            Job::Haul | Job::Dig | Job::Idle => {
                let r = mind.forage / threshold;
                let p = r * r / (1.0 + r * r);
                let dig_roll = || {
                    let r = dig_stim / dig_threshold;
                    rng::stream(world.seed, organism as u64, frame, RNG_SLOT_NEEDS_DIG).unit_f32()
                        < r * r / (1.0 + r * r)
                };
                let slot = if parts.placebo {
                    RNG_SLOT_NEEDS_PLACEBO
                } else {
                    RNG_SLOT_NEEDS
                };
                if !mind.gave_up && rng::stream(world.seed, organism as u64, frame, slot).unit_f32() < p {
                    mind.since = frame;
                    world.needs.as_mut().expect("walking").counts.took_forage += 1;
                    Job::Forage
                } else if can_dig && frame >= mind.dig_rest_until && dig_roll() {
                    mind.since = frame;
                    world.needs.as_mut().expect("walking").counts.dig_took += 1;
                    Job::Dig
                } else {
                    Job::Idle
                }
            }
        }
    };
    if job != mind.job && matches!(job, Job::Haul | Job::Dig) {
        mind.since = frame;
    }
    mind.job = job;
    // **The job's hold**: its own patience under the ceiling. Idle holds
    // nothing. A dig job on its stimulus has no pull to lose patience on, so
    // its patience is the time left before it runs out with no cut.
    mind.hold = HOLD_CEILING
        * match (job, carrying_home) {
            (Job::Idle, _) => 0.0,
            (Job::Forage, false) => scout_patience,
            (Job::Dig, _) if can_dig && !face => {
                1.0 - (frame.saturating_sub(mind.since) as f32 / DIG_PATIENCE as f32).clamp(0.0, 1.0)
            }
            _ => home_patience,
        };
    let veto = world.needs.as_ref().is_some_and(|n| n.veto_needs);
    let held_off = !veto && hunger > 0.0 && hunger > mind.hold;
    // **The hunger bout** ([`WalkParts::hunger_holds`]): need that has fired
    // stays until hunger is back to 0, whatever hold a job taken since
    // carries. No give-up of its own -- the drives inside it have theirs --
    // and the veto still wins.
    let need = if parts.hunger_holds {
        let c = &mut world.needs.as_mut().expect("walking").counts;
        if mind.bout && hunger <= 0.0 {
            mind.bout = false;
            c.bouts_fed += 1;
            c.bouts_fed_outside += u64::from(outside);
        } else if !mind.bout && held_off {
            mind.bout = true;
            c.bouts += 1;
        }
        let need = !veto && hunger > 0.0 && mind.bout;
        c.hold_ignored += u64::from(need && !held_off);
        need
    } else {
        held_off
    };
    // Out of hunger, the eat drive's progress and its give-ups this bout
    // start again ([`WalkParts::eat_fade`]); a window still running stays.
    if parts.eat_fade && !need {
        mind.eat_best = f32::INFINITY;
        mind.eat_stall = 0;
        mind.eat_bouts = 0;
    }
    if need && job != Job::Idle {
        world.needs.as_mut().expect("walking").counts.need_over_job += 1;
    }

    let mut urges = *outputs;
    // **Sealed in, it gets out whatever its job** ([`WalkParts::sealed_out`]):
    // a fed ant's escape. The hungry one's is in `hungry_act`, after it has
    // eaten what it can reach or hold. An eat takes no step, so it does not
    // start the seal's count again, and an ant fed by its own load cuts out
    // on its next decision rather than walking its carry job (the rev 2
    // review, §3).
    let sealed_did = if !need && parts.sealed_out && mind.sealed {
        sealed_escape(world, organism, def, head, &mut mind)
    } else {
        None
    };
    let did = if need {
        hungry_act(
            world, head, organism, def, &mut mind, below, covered, outside, &mut urges,
        )
    } else if sealed_did.is_some() {
        sealed_did
    } else {
        mind.drive = match job {
            Job::Haul => Drive::Haul,
            Job::Forage if laden => Drive::Carry,
            // **Rich enough to lay, it walks home to lay**
            // ([`WalkParts::lay_home`]), as the shipped ant's lay-home pull
            // takes it (`ready_to_lay`), instead of steering out.
            Job::Forage if ready_lay => {
                world.needs.as_mut().expect("walking").counts.lay_walks += 1;
                Drive::Home
            }
            Job::Forage => Drive::Forage,
            Job::Dig => Drive::Dig,
            Job::Idle if below => Drive::Rest,
            Job::Idle => Drive::Home,
        };
        if mind.drive == Drive::Haul {
            // **Put it down where `act` lets it lie**: inside, the keep rule
            // holds it until it is out.
            urges[O::DropSpoil as usize] = 1.0;
            urges[O::Dig as usize] = 0.0;
        }
        // **Only the dig job digs at `act`'s urge** ([`WalkParts::
        // only_diggers`]), as the design's act table has it. Slice 1 handed
        // the brain's own dig urge to every job and to idling, and foraging,
        // walking home and resting made 98% of the cuts the shipped walk did
        // not, in the mound's unprotected walls (`results.md` steps 1-2).
        if parts.only_diggers && mind.drive != Drive::Dig {
            urges[O::Dig as usize] = 0.0;
        }
        // **Food picked up to eat is kept** ([`WalkParts::meal`]): a bite at
        // home that met the walk's hunger, which counts the crop, ended the
        // need and handed `act` the brain's drop, which is on at the nest --
        // 77 of 78 traced bites at home put back down a median 15 frames
        // later (`results.md`).
        if keeps_meal && urges[O::Drop as usize] > 0.0 {
            urges[O::Drop as usize] = 0.0;
            world.needs.as_mut().expect("walking").counts.meals_kept += 1;
        }
        // **The door cut** ([`WalkParts::clear`]): walking home, stalled,
        // the soil cell toward home near the door is cut.
        if parts.clear
            && mind.drive == Drive::Home
            && mind.home_stall >= CLEAR_STALL
            && clear_cut(world, organism, def, head, &mind)
        {
            mind.cut = CUT_CLEAR;
            mind.home_stall = 0;
            world.needs.as_mut().expect("walking").counts.clear_cuts += 1;
            Some(Did {
                dug: 1,
                ..Did::default()
            })
        } else {
            None
        }
    };
    world.needs.as_mut().expect("walking").counts.decisions[mind.drive as usize] += 1;
    let did = match did {
        Some(did) => did,
        None => {
            let delivered = world.creature_stats.trip_deliveries;
            let did = super::act(world, head.0, head.1, organism, def, &urges, draw);
            if did.dug > 0 && !did.packed {
                world.needs.as_mut().expect("walking").counts.cuts[mind.drive as usize] += 1;
                mind.cut = CUT_ACT;
                // A cut renews the dig job's patience.
                if mind.drive == Drive::Dig {
                    mind.since = world.frame;
                }
            }
            // **Unloaded beside food already lying uneaten**: more than
            // enough food, remembered.
            if world.creature_stats.trip_deliveries > delivered {
                let (hx, hy) = world
                    .organism(organism)
                    .and_then(|s| s.chain.first().copied())
                    .unwrap_or(head);
                if food_lying(world, (hx, hy), 3) >= GLUT_FOOD {
                    mind.glut = 1.0;
                    mind.glut_at = world.frame;
                    world.needs.as_mut().expect("walking").counts.gluts += 1;
                }
            }
            did
        }
    };
    keep_mind(world, organism, mind);
    did
}

/// **A need has won.** Eat what it holds, put soil down where it closes no
/// way, and pick the drive: food in reach, the way out inside (cutting when
/// it has stalled against soil), or the search outside. Returns what it did
/// itself, or `None` with `urges` set for `act`.
#[allow(clippy::too_many_arguments)]
fn hungry_act(
    world: &mut World,
    head: (i32, i32),
    organism: OrganismId,
    def: &CreatureDef,
    mind: &mut Mind,
    below: bool,
    covered: bool,
    outside: bool,
    urges: &mut [f32; brain::BRAIN_OUTPUTS],
) -> Option<Did> {
    use brain::BrainOutput as O;
    let spoil = world.organism(organism).and_then(|s| s.spoil);
    let parts = world.needs.as_ref().map_or(WalkParts::NONE, |n| n.parts);
    let in_mound = !below && covered;
    urges[O::Feed as usize] = 1.0;
    urges[O::Drop as usize] = 0.0;
    urges[O::Dig as usize] = 0.0;
    // Outside, a pellet goes down where `act` lets it lie; inside it is put
    // down here or kept.
    urges[O::DropSpoil as usize] = if outside { 1.0 } else { 0.0 };
    // **Eats what it holds**: a store load is put down beside the head and
    // `act`'s mouth takes it this same tick. No doorway rule: food put down
    // and eaten at once blocks no way.
    if let Some(load) = spoil.filter(|s| s.store) {
        if let Some(p) = food_down_site(world, head) {
            world.set(p.0, p.1, load.cell);
            if let Some(s) = world.organism_mut(organism) {
                s.spoil = None;
                s.spoil_ring = None;
            }
            world.needs.as_mut().expect("walking").counts.ate_held += 1;
            mind.drive = Drive::Eat;
            mind.food = Some(p);
            return None;
        }
        // **Nowhere to put it down, sealed in: it eats it from its jaws**
        // ([`WalkParts::eat_held`]). 41 of the 93 walled-in starvers on
        // steady seeds 1-4 (`reach`) died holding a store load, 33 of them
        // fed when the soil closed (`trace-walled-in-2026-10-11.md` §1A).
        // Sealed only, so a carrier in a crowd at the door, whose air goes
        // somewhere, still waits to put it down.
        if parts.eat_held && mind.sealed {
            if eat_from_jaws(world, organism, def, head, load) {
                world.needs.as_mut().expect("walking").counts.ate_held_jaws += 1;
                mind.drive = Drive::Eat;
                return Some(Did::default());
            }
            world.needs.as_mut().expect("walking").counts.eat_held_refused += 1;
        }
    }
    let escaping = mind.stall >= ESCAPE_STALL && (below || in_mound);
    // **Lets go of soil safely**: where the cell left keeps the way open
    // (`need_drop_site`), and never in the mound's tunnels unless it is
    // shut in there.
    if let Some(load) = spoil.filter(|s| !s.store) {
        if !outside && (!in_mound || escaping) {
            if let Some((px, py)) = need_drop_site(world, head) {
                world.set(px, py, load.cell);
                if let Some(s) = world.organism_mut(organism) {
                    s.spoil = None;
                    s.spoil_ring = None;
                }
                world.creature_stats.spoil_dumped += 1;
                world.needs.as_mut().expect("walking").counts.pellets_down += 1;
            }
        }
    }
    // **The food sense**, off while a give-up's window runs
    // ([`WalkParts::eat_fade`]).
    if parts.eat_fade && world.frame < mind.eat_rest_until {
        world.needs.as_mut().expect("walking").counts.eat_rest += 1;
    } else {
        let (food, hidden) = sense_food(world, organism, def, head, parts.reach);
        if let Some(food) = food {
            mind.drive = Drive::Eat;
            mind.food = Some(food);
            return None;
        }
        if hidden {
            // **Sealed, or round the box's edge** (the results review's item
            // 4): the flood stops at the box, so food joined to the head by
            // a way that leaves it is hidden too. A wider flood counts which.
            let air = food_joined_by_air(world, organism, def, head);
            let frame = world.frame;
            let n = world.needs.as_mut().expect("walking");
            n.counts.reach_hidden += 1;
            n.counts.reach_hidden_air += u64::from(air);
            if let Some(log) = n.hidden_log.as_mut() {
                log.push((frame, organism, air));
            }
        }
    }
    // **Sealed in** ([`WalkParts::sealed_out`]): nothing to eat in reach, so
    // it gets out -- the same escape a fed ant makes, in place of the one
    // below, which waits for a perfect shell of soil.
    if parts.sealed_out && mind.sealed {
        if let Some(did) = sealed_escape(world, organism, def, head, mind) {
            return Some(did);
        }
    }
    if outside {
        mind.drive = Drive::Seek;
        return None;
    }
    mind.drive = Drive::Out;
    if !escaping {
        return None;
    }
    // **Escape**: the way out has stalled. Cut toward it, or up, where the
    // cell ahead is soil -- a crowd ahead is waited out, not dug round.
    let aim = escape_aim(world, organism, head, mind);
    let (dx, dy) = DIRS[aim as usize];
    let ahead = world.get(head.0 + dx, head.1 + dy);
    if in_a_doorway(world, head) || !jaw_can_cut(world, def, organism, ahead) {
        mind.esc_why = if in_a_doorway(world, head) { 1 } else { 2 };
        return None;
    }
    mind.drive = Drive::Escape;
    if let Some(s) = world.organism_mut(organism) {
        s.heading = aim;
    }
    match world.organism(organism).and_then(|s| s.spoil) {
        Some(load) if !load.store => {
            // Still holding soil: walled in, it packs the pellet behind as it
            // cuts on; otherwise it found nowhere to put it down and waits.
            let walled = walled_in(world, organism, head);
            if walled && pack_behind(world, organism, def, head, load, false, None) {
                world.needs.as_mut().expect("walking").counts.escape_packs += 1;
                mind.cut = CUT_PACK;
                return Some(Did {
                    dug: 1,
                    packed: true,
                    ..Did::default()
                });
            }
            mind.esc_why = if walled { 4 } else { 3 };
            if !walled {
                mind.esc_nb = walled_in_why(world, organism, head);
            }
            None
        }
        Some(_) => {
            mind.esc_why = 5;
            None
        }
        None => {
            if escape_cut(world, organism, def, head, aim) {
                world.needs.as_mut().expect("walking").counts.escape_cuts += 1;
                mind.cut = CUT_ESCAPE;
                Some(Did {
                    dug: 1,
                    ..Did::default()
                })
            } else {
                mind.esc_why = 6;
                None
            }
        }
    }
}

/// **What keeps a pellet holder from counting as walled in**, as bits, for
/// the trace (2026-10-11, the walled-in starvers' fault B): 1 its own body
/// carries a rider, 2 an empty cell beside the head, 4 another animal or
/// brood, 8 food, 16 anything else (liquid, gas). `creature::walled_in`'s
/// test read again; nothing is changed.
fn walled_in_why(world: &World, organism: OrganismId, head: (i32, i32)) -> u8 {
    let Some(st) = world.organism(organism) else { return 0 };
    let mut why = 0;
    if !st
        .chain
        .iter()
        .all(|&(x, y)| world.get(x, y).organism_id() == organism && world.riders_at(x, y).is_empty())
    {
        why |= 1;
    }
    for &(dx, dy) in NEIGHBOURS_8.iter() {
        let c = world.get(head.0 + dx, head.1 + dy);
        if c.organism_id() == organism {
            continue;
        }
        if c.organism_id() != 0 {
            why |= 4;
        } else if c.material == material::EMPTY {
            why |= 2;
        } else if food_value(world, c) > 0.0 {
            why |= 8;
        } else if !matches!(
            world.materials.kind(c.material),
            MaterialKind::Solid | MaterialKind::Powder
        ) {
            why |= 16;
        }
    }
    why
}

/// **Whether the air round the head goes anywhere** ([`WalkParts::sealed_out`]):
/// [`reachable_from_head`]'s flood -- empty cells and living animals pass,
/// soil, food, crumbs, liquid and plants do not -- and `Some(cells reached)`
/// when none of them is on the box's rim: no way out within [`FOOD_REACH`]
/// cells. **A stand-in, named as one**: a pocket wider than the box reads
/// open (4 of the 93 walled-in starvers were in 126-cell pockets), and water
/// or a plant across a real way reads as a wall, harmless on the dry beds
/// and to be checked before a misted or played one (the rev 2 review, §2).
/// Nestmates pass, so two ants sealed in one pocket are each sealed -- the
/// case that broke `walled_in` for 11 of 26 traced pellet holders.
fn seal_flood(world: &World, head: (i32, i32)) -> Option<u16> {
    let open = reachable_from_head(world, head);
    let last = FOOD_BOX - 1;
    let mut cells = 0u16;
    for (y, row) in open.iter().enumerate() {
        for (x, &o) in row.iter().enumerate() {
            if !o {
                continue;
            }
            if x == 0 || y == 0 || x == last || y == last {
                return None;
            }
            cells += 1;
        }
    }
    Some(cells)
}

/// **What an ant holds, for the seal's log**: 0 nothing, 1 soil, 2 a store load.
fn hold_of(world: &World, organism: OrganismId) -> u8 {
    world
        .organism(organism)
        .and_then(|s| s.spoil)
        .map_or(0, |l| 1 + u8::from(l.store))
}

/// One row of [`NeedsWalk::sealed_log`], when a harness asked for it.
fn seal_log(world: &mut World, organism: OrganismId, event: u8, at: (i32, i32), cells: u16, cuts: u16) {
    if world.needs.as_ref().is_none_or(|n| n.sealed_log.is_none()) {
        return;
    }
    let hold = hold_of(world, organism);
    let (energy, crop) = world
        .organism(organism)
        .map_or((f32::NAN, 0.0), |s| (s.energy, s.crop.map_or(0.0, |c| c.worth())));
    let frame = world.frame;
    if let Some(log) = world.needs.as_mut().and_then(|n| n.sealed_log.as_mut()) {
        log.push(SealRow {
            frame,
            id: organism,
            event,
            at,
            cells,
            hold,
            energy,
            crop,
            cuts,
        });
    }
}

/// **Where a sealed ant puts its load down in the pocket**
/// ([`WalkParts::sealed_out`]): an empty cell beside the head, ground under it
/// first, never in a doorway or the door column, never one whose filling
/// closes a way ([`closes_a_way`]), and never the cell it last cut or one of
/// the three ahead of `aim` -- filled, those are the cut it would make again,
/// and it would cut and refill one cell for ever (the rev 2 review, §2). Only
/// the drop's footing is waived.
fn pocket_drop_site(world: &World, head: (i32, i32), last_cut: Option<(i32, i32)>, aim: u8) -> Option<(i32, i32)> {
    let ahead: Vec<(i32, i32)> = [aim.wrapping_add(7) % 8, aim, (aim + 1) % 8]
        .iter()
        .map(|&d| {
            let (dx, dy) = DIRS[d as usize];
            (head.0 + dx, head.1 + dy)
        })
        .collect();
    let ok = |p: &(i32, i32)| {
        world.in_bounds(p.0, p.1)
            && world.get(p.0, p.1).material == material::EMPTY
            && Some(*p) != last_cut
            && !ahead.contains(p)
            && !in_door_column(world, *p)
            && !in_a_doorway(world, *p)
            && !closes_a_way(world, *p)
    };
    let around = NEIGHBOURS_8.iter().map(|&(dx, dy)| (head.0 + dx, head.1 + dy));
    around
        .clone()
        .filter(ok)
        .find(|&p| drop_footing(world, p))
        .or_else(|| around.clone().find(ok))
}

/// **A sealed ant gets out** ([`WalkParts::sealed_out`]), whatever its hunger
/// or job: the escape `hungry_act` makes, without the wait for a perfect
/// shell of soil (`walled_in`), which three ordinary things break -- a
/// nestmate in the pocket, a spare empty cell, crumbs beside the head (26
/// traced pellet holders, `trace-walled-in-2026-10-11.md` §1B).
///
/// - Its way out's heading ([`escape_aim`]), or, where that cell cannot be
///   cut (its own air, a nestmate, rock), the cuttable heading nearest it.
///   Before the rev 2 review this sat behind the jaw's check, which refused
///   exactly the spare-cell and rider cases.
/// - Soil in its jaws: packed behind as it cuts on ([`pack_behind`]); where
///   that fails, put down in the pocket ([`pocket_drop_site`]), and the next
///   decision cuts with empty jaws.
/// - A store load (fed, or `eat_held` off or refused): put down in the
///   pocket the same way, where it stays as food.
/// - Empty jaws: [`escape_cut`].
///
/// `None`, and nothing done, in a doorway, or with nothing cuttable and
/// nowhere to put a load down: the caller's drive stands.
fn sealed_escape(
    world: &mut World,
    organism: OrganismId,
    def: &CreatureDef,
    head: (i32, i32),
    mind: &mut Mind,
) -> Option<Did> {
    if in_a_doorway(world, head) {
        mind.esc_why = 1;
        return None;
    }
    let want = escape_aim(world, organism, head, mind);
    let cuttable = |d: u8| {
        let (dx, dy) = DIRS[d as usize];
        jaw_can_cut(world, def, organism, world.get(head.0 + dx, head.1 + dy))
    };
    let aim = [0u8, 1, 7, 2, 6, 3, 5, 4]
        .iter()
        .map(|&o| (want + o) % 8)
        .find(|&d| cuttable(d));
    let spoil = world.organism(organism).and_then(|s| s.spoil);
    if aim.is_some_and(|a| a != want) {
        world.needs.as_mut().expect("walking").counts.seal_aim_turned += 1;
    }
    if let Some(load) = spoil {
        if let Some(a) = aim.filter(|_| !load.store) {
            if let Some(s) = world.organism_mut(organism) {
                s.heading = a;
            }
            if pack_behind(world, organism, def, head, load, false, None) {
                let c = &mut world.needs.as_mut().expect("walking").counts;
                c.escape_packs += 1;
                c.sealed_packs += 1;
                mind.drive = Drive::Escape;
                mind.cut = CUT_PACK;
                mind.seal_cut_n = mind.seal_cut_n.saturating_add(1);
                return Some(Did {
                    dug: 1,
                    packed: true,
                    ..Did::default()
                });
            }
        }
        if let Some(p) = pocket_drop_site(world, head, mind.last_cut, aim.unwrap_or(want)) {
            world.set(p.0, p.1, load.cell);
            if let Some(s) = world.organism_mut(organism) {
                s.spoil = None;
                s.spoil_ring = None;
            }
            let c = &mut world.needs.as_mut().expect("walking").counts;
            if load.store {
                c.sealed_load_down += 1;
            } else {
                c.sealed_drops += 1;
            }
            mind.drive = Drive::Escape;
            return Some(Did::default());
        }
        mind.esc_why = if load.store { 5 } else { 4 };
        return None;
    }
    let Some(a) = aim else {
        mind.esc_why = 2;
        return None;
    };
    if let Some(s) = world.organism_mut(organism) {
        s.heading = a;
    }
    let (dx, dy) = DIRS[a as usize];
    let cell = (head.0 + dx, head.1 + dy);
    if !escape_cut(world, organism, def, head, a) {
        mind.esc_why = 6;
        return None;
    }
    let c = &mut world.needs.as_mut().expect("walking").counts;
    c.escape_cuts += 1;
    c.sealed_cuts += 1;
    if mind.seal_cuts.contains(&cell) {
        c.seal_recut += 1;
    }
    mind.seal_cuts[usize::from(mind.seal_cut_n % 4)] = cell;
    mind.seal_cut_n = mind.seal_cut_n.saturating_add(1);
    mind.last_cut = Some(cell);
    mind.drive = Drive::Escape;
    mind.cut = CUT_ESCAPE;
    Some(Did {
        dug: 1,
        ..Did::default()
    })
}

/// **A store load eaten from the jaws** ([`WalkParts::eat_held`]): into the
/// crop at face value through `act`'s own swallow ([`crop_take`]), nothing
/// booked until it digests, and only where `act`'s mouth would take it: the
/// crop empty or holding the same food. `false`, and nothing changed,
/// otherwise. `store_kept` needs no check: it keeps the store from the fed,
/// and this ant is hungry.
///
/// **And the mouth's bookkeeping after it**, which the first build skipped
/// (code review of PR 684): `pickups` (and `pickups_at_nest`), `eats` and
/// `best_bite`, `life.bites` and the first-feed line, the carry patience's
/// restart, and an empty crop's fresh trip marks. A store load is the
/// nest's own food, so it is booked as food taken at home wherever the
/// pocket is: a packed lunch (`lunch`), never a trip (`trip_load`) and
/// never foraging (`foraged`). Without it a meal eaten from the jaws kept
/// whatever `lunch` the last crop left, and `walk_act`'s `laden` and
/// `crop_to_feed` read the meal as a load to carry.
fn eat_from_jaws(world: &mut World, organism: OrganismId, def: &CreatureDef, head: (i32, i32), load: Spoil) -> bool {
    let food = load.cell.material;
    let crop = world.organism(organism).and_then(|s| s.crop);
    if crop.is_some_and(|c| c.material != food) {
        return false;
    }
    let worth = food_value(world, load.cell);
    note_food(world, organism, head, FOOD_SWALLOW, worth, true);
    crop_take(world, organism, food, load.cell.shade, worth, None);
    let at_home = food_at_home(world, organism, def, head.0, head.1);
    world.creature_stats.pickups += 1;
    if at_home {
        world.creature_stats.pickups_at_nest += 1;
    }
    let restart = carry_patience_of(world);
    let trip_reach = trip_reach_of(world).is_some();
    let empty = crop.is_none_or(|c| c.worth() <= 0.0);
    let first = if let Some(s) = world.organism_mut(organism) {
        s.spoil = None;
        s.spoil_ring = None;
        s.life.bites += 1;
        if restart {
            s.home_best = f32::INFINITY;
            s.home_away = 0;
            s.home_patience = 1.0;
        }
        if trip_reach && empty {
            s.trip_load = false;
            s.trip_src = 0;
        }
        if empty {
            s.trip_cells = 0;
        }
        s.lunch = s.lunch || empty;
        s.life.bites == 1
    } else {
        false
    };
    if first {
        if let Some((born_frame, species, lineage, generation)) = world
            .organism(organism)
            .map(|s| (s.born_frame, s.species, s.lineage, s.generation))
        {
            world.run_log.push(crate::sim::world::LogEvent {
                frame: world.frame,
                id: organism,
                born_frame,
                species,
                kind: crate::sim::world::LogKind::FirstFeed,
                other: 0,
                lineage,
                generation,
                detail: String::new(),
            });
        }
    }
    world.creature_stats.eats += 1;
    world.creature_stats.best_bite = world.creature_stats.best_bite.max(worth);
    true
}

/// **Why an escape did not cut**, by [`WalkRow::esc_why`]: the trace of the
/// walled-in starvers (2026-10-11), who escaped for hundreds of decisions
/// holding a load and never cut.
pub const ESC_WHY_NAMES: [&str; 7] = [
    "",
    "doorway",
    "jaw_cannot_cut",
    "pellet_not_walled_in",
    "pellet_pack_failed",
    "store_load",
    "cut_failed",
];

/// **Where a hungry ant puts a store load down to eat it**: an empty cell
/// beside the head, one with ground under it first.
fn food_down_site(world: &World, head: (i32, i32)) -> Option<(i32, i32)> {
    let empty = |p: &(i32, i32)| world.in_bounds(p.0, p.1) && world.get(p.0, p.1).material == material::EMPTY;
    let mut around = NEIGHBOURS_8.iter().map(|&(dx, dy)| (head.0 + dx, head.1 + dy));
    around
        .clone()
        .filter(empty)
        .find(|&p| drop_footing(world, p))
        .or_else(|| around.find(empty))
}

/// **The food sense**: the nearest cell this ant's mouth would take within
/// [`FOOD_REACH`], by Chebyshev distance and then scan order. Loose food and
/// plants; never a living animal, and never its own kind.
///
/// With `reach` ([`WalkParts::reach`]) a cell counts only if its 8-ring, the
/// mouth's ring, touches a cell [`reachable_from_head`]; the second value
/// says food was in the box and none of it could be reached. Without it, food
/// is sensed through soil, as slice 1 sensed it, and the second is false.
fn sense_food(
    world: &World,
    organism: OrganismId,
    def: &CreatureDef,
    head: (i32, i32),
    reach: bool,
) -> (Option<(i32, i32)>, bool) {
    let gut = gut_of(world, organism, def);
    let mut best: Option<(i32, (i32, i32))> = None;
    let mut open: Option<[[bool; FOOD_BOX]; FOOD_BOX]> = None;
    let mut hidden = false;
    for dy in -FOOD_REACH..=FOOD_REACH {
        for dx in -FOOD_REACH..=FOOD_REACH {
            let (x, y) = (head.0 + dx, head.1 + dy);
            if !world.in_bounds(x, y) {
                continue;
            }
            let c = world.get(x, y);
            if c.material == material::EMPTY
                || is_living_kin(world, c, gut)
                || (c.organism_id() != 0 && is_animal_cell(world, c))
            {
                continue;
            }
            if diet_yield(world, c, gut.bias) <= EAT_YIELD_THRESHOLD {
                continue;
            }
            let d = dx.abs().max(dy.abs());
            if best.is_none_or(|(b, _)| d < b) {
                if reach && !touches_open(open.get_or_insert_with(|| reachable_from_head(world, head)), dx, dy) {
                    hidden = true;
                    continue;
                }
                best = Some((d, (x, y)));
            }
        }
    }
    (best.map(|(_, p)| p), hidden && best.is_none())
}

/// **Where the air round the head reaches** ([`WalkParts::reach`]): the cells
/// of the food sense's box joined to the head, 8-way as the walk steps,
/// through open cells: empty, or a living animal's (ants and brood; a jam
/// clears, and the eat drive's give-up covers one that does not). Indexed
/// `[dy + FOOD_REACH][dx + FOOD_REACH]`.
///
/// **A stand-in, named as one**: the walk's own step predicate is body-aware
/// (`usable_headings`) and cannot be flooded, so this asks what passes air,
/// not what passes the ant's body; liquid and plant cells do not pass; and
/// the flood stops at the box's edge, so food reachable only by a way that
/// leaves the box is not sensed though air along that way would carry it.
/// Out of the world is never open (`OUT_OF_BOUNDS` reads as bedrock).
///
/// **The edge's share, measured** (the results review, 2026-10-10, steady
/// food, store on): of the decisions on which `reach` and slice 1 disagreed,
/// the food was joined to the head by air within a 61x61 box on 12% (seed
/// 1), 37% (seed 3) and about 51% (seed 11). Starvers it touched were near
/// zero (0 of 134, 0 of 135, 2 of 325). `reach_hidden_air` counts it per
/// run. Liquid and plant cells block the flood too: harmless on the dry
/// beds, to be checked before a misted or played one.
fn reachable_from_head(world: &World, head: (i32, i32)) -> [[bool; FOOD_BOX]; FOOD_BOX] {
    let r = FOOD_REACH;
    let passes = |x: i32, y: i32| {
        world.in_bounds(x, y) && {
            let c = world.get(x, y);
            c.material == material::EMPTY || (c.organism_id() != 0 && is_animal_cell(world, c))
        }
    };
    let mut open = [[false; FOOD_BOX]; FOOD_BOX];
    open[r as usize][r as usize] = true;
    let mut todo = vec![(0, 0)];
    while let Some((dx, dy)) = todo.pop() {
        for &(ex, ey) in NEIGHBOURS_8.iter() {
            let (nx, ny) = (dx + ex, dy + ey);
            if nx.abs() > r || ny.abs() > r {
                continue;
            }
            let cell = &mut open[(ny + r) as usize][(nx + r) as usize];
            if *cell || !passes(head.0 + nx, head.1 + ny) {
                continue;
            }
            *cell = true;
            todo.push((nx, ny));
        }
    }
    open
}

/// How far the check of what [`WalkParts::reach`] hid floods: a 61x61 box.
pub const AIR_CHECK: i32 = 30;

/// **Whether any food in the sense's box is joined to the head by air within
/// [`AIR_CHECK`] cells**: the same food the sense scans for, and the same
/// flood as [`reachable_from_head`], wider. For the counters only; it reads
/// the world and decides nothing.
fn food_joined_by_air(world: &World, organism: OrganismId, def: &CreatureDef, head: (i32, i32)) -> bool {
    let gut = gut_of(world, organism, def);
    let r = AIR_CHECK;
    let side = (2 * r + 1) as usize;
    let at = |dx: i32, dy: i32| (dy + r) as usize * side + (dx + r) as usize;
    let passes = |x: i32, y: i32| {
        world.in_bounds(x, y) && {
            let c = world.get(x, y);
            c.material == material::EMPTY || (c.organism_id() != 0 && is_animal_cell(world, c))
        }
    };
    let mut open = vec![false; side * side];
    open[at(0, 0)] = true;
    let mut todo = vec![(0, 0)];
    while let Some((dx, dy)) = todo.pop() {
        for &(ex, ey) in NEIGHBOURS_8.iter() {
            let (nx, ny) = (dx + ex, dy + ey);
            if nx.abs() > r || ny.abs() > r || open[at(nx, ny)] || !passes(head.0 + nx, head.1 + ny) {
                continue;
            }
            open[at(nx, ny)] = true;
            todo.push((nx, ny));
        }
    }
    for dy in -FOOD_REACH..=FOOD_REACH {
        for dx in -FOOD_REACH..=FOOD_REACH {
            let (x, y) = (head.0 + dx, head.1 + dy);
            if !world.in_bounds(x, y) {
                continue;
            }
            let c = world.get(x, y);
            if c.material == material::EMPTY
                || is_living_kin(world, c, gut)
                || (c.organism_id() != 0 && is_animal_cell(world, c))
                || diet_yield(world, c, gut.bias) <= EAT_YIELD_THRESHOLD
            {
                continue;
            }
            if NEIGHBOURS_8.iter().any(|&(ex, ey)| open[at(dx + ex, dy + ey)]) {
                return true;
            }
        }
    }
    false
}

/// Whether the cell at `(dx, dy)` from the head has an open cell of `open`
/// in its 8-ring: the mouth, standing there, could take it.
fn touches_open(open: &[[bool; FOOD_BOX]; FOOD_BOX], dx: i32, dy: i32) -> bool {
    let r = FOOD_REACH;
    NEIGHBOURS_8.iter().any(|&(ex, ey)| {
        let (nx, ny) = (dx + ex, dy + ey);
        nx.abs() <= r && ny.abs() <= r && open[(ny + r) as usize][(nx + r) as usize]
    })
}

/// Loose food cells within `reach` of `head`: what a forager unloading sees
/// already lying there.
fn food_lying(world: &World, head: (i32, i32), reach: i32) -> u32 {
    let mut n = 0;
    for y in head.1 - reach..=head.1 + reach {
        for x in head.0 - reach..=head.0 + reach {
            if world.in_bounds(x, y) {
                let c = world.get(x, y);
                n += u32::from(c.organism_id() == 0 && food_value(world, c) > 0.0);
            }
        }
    }
    n
}

/// **Where the way out aims from here**, and which measure says whether a
/// step got nearer: the nest's way below the founding ground, the mound's
/// way in the mound (both labelled stand-ins), else the door vector's
/// origin, else straight up.
fn out_target(world: &World, organism: OrganismId, head: (i32, i32), mind: &Mind) -> (u8, Option<(i32, i32)>) {
    if let Some(p) = way_out_from(world, organism, head) {
        return (OUT_NEST_WAY, Some(p));
    }
    if let Some(p) = mound_way_out(world, organism, head) {
        return (OUT_MOUND_WAY, Some(p));
    }
    match mind.door {
        Some(d) if d != head => (OUT_DOOR, Some(d)),
        _ => (OUT_UP, None),
    }
}

/// [`REST_LOOKAHEAD`] steps out along the mound's way (`MoundOut`'s map),
/// as `mound_out_pull` reads it, without its switch or its hunger gate.
fn mound_way_out(world: &World, organism: OrganismId, head: (i32, i32)) -> Option<(i32, i32)> {
    let site = world.nearest_nest_site(head.0, head.1)?;
    let way = world.mound_ways.iter().find(|w| w.site == site)?;
    step_down_way(|x, y| way.at(x, y), organism, head)
}

/// **How far out this ant is, on the measure `kind` names** -- smaller is
/// nearer the open air -- for the escape's stall.
fn out_score(world: &World, head: (i32, i32), kind: u8, mind: &Mind) -> i32 {
    let way = |ways: &[NestWay]| {
        world
            .nearest_nest_site(head.0, head.1)
            .and_then(|site| ways.iter().find(|w| w.site == site))
            .and_then(|w| w.at(head.0, head.1))
            .map_or(i32::MAX, i32::from)
    };
    match kind {
        OUT_NEST_WAY => way(&world.nest_ways),
        OUT_MOUND_WAY => way(&world.mound_ways),
        OUT_DOOR => mind
            .door
            .map_or(i32::MAX, |(dx, dy)| (head.0 - dx).abs().max((head.1 - dy).abs())),
        _ => head.1,
    }
}

/// **Which way an escaping ant cuts**: the heading nearest its way out's
/// target, or straight up with none.
fn escape_aim(world: &World, organism: OrganismId, head: (i32, i32), mind: &Mind) -> u8 {
    let (_, target) = out_target(world, organism, head, mind);
    let Some(target) = target else { return UP_DIR };
    heading_toward(head, target)
}

/// The heading whose step points most nearly from `from` to `to`; straight
/// up when they are the same cell.
fn heading_toward(from: (i32, i32), to: (i32, i32)) -> u8 {
    let (vx, vy) = ((to.0 - from.0) as f32, (to.1 - from.1) as f32);
    let mut best = (f32::NEG_INFINITY, UP_DIR);
    for d in 0..8u8 {
        let (dx, dy) = DIRS[d as usize];
        let c = (dx as f32 * vx + dy as f32 * vy) / DIR_LEN[(d & 1) as usize];
        if c > best.0 {
            best = (c, d);
        }
    }
    best.1
}

/// **An escape's cut**: `act`'s dig mechanics -- the cell becomes the pellet
/// in the jaws, tamped as spoil, the gallery lined -- without the dig's
/// vetoes (the heap cue, the face trip, the roof, the turn downward), which
/// are about where a nest may grow; this grows nothing, it gets an ant out.
fn escape_cut(world: &mut World, organism: OrganismId, def: &CreatureDef, head: (i32, i32), aim: u8) -> bool {
    let (dx, dy) = DIRS[aim as usize];
    let (tx, ty) = (head.0 + dx, head.1 + dy);
    let target = world.get(tx, ty);
    if !jaw_can_cut(world, def, organism, target) {
        return false;
    }
    let mut pellet = target;
    let ground = world.materials.get(target.material);
    let hauled = if crate::sim::update::spoil_footing() {
        ground.spoils_into.or(ground.packs_into)
    } else {
        ground.packs_into
    };
    if let Some(hauled) = hauled {
        pellet.material = hauled;
    }
    world.set(tx, ty, Cell::EMPTY);
    world.dug_cells.insert((tx, ty));
    if spoil_kept() {
        if let Some(s) = world.organism_mut(organism) {
            s.spoil = Some(Spoil {
                cell: pellet,
                store: false,
            });
        }
    }
    world.creature_stats.digs += 1;
    if let Some(s) = world.organism_mut(organism) {
        s.life.digs += 1;
    }
    line_burrow(world, tx, ty);
    true
}

/// **The door cut** ([`WalkParts::clear`]): the one cell an empty ant
/// walking home needs cut to get nearer home -- the cell its heading toward
/// home points at -- when that cell is soil near the door
/// ([`in_door_scope`]). [`escape_cut`]'s mechanics: the cell becomes the
/// pellet in its jaws, which the haul job then takes out. A blocked path,
/// not an urge: the caller asks only once the ant has stalled
/// ([`CLEAR_STALL`]).
///
/// **Empty jaws and an empty crop only.** `act` never lets a laden ant dig,
/// and a pellet in the jaws of an ant with food in its crop could not be put
/// down: `act`'s drop branch returns before its soil branch. So a carrier
/// waits at a shut door for an empty ant to open it.
fn clear_cut(world: &mut World, organism: OrganismId, def: &CreatureDef, head: (i32, i32), mind: &Mind) -> bool {
    let Some(st) = world.organism(organism) else {
        return false;
    };
    if st.spoil.is_some() || st.crop.is_some_and(|c| c.worth() > 0.0) {
        return false;
    }
    let Some(target) = mind.home_for.filter(|&t| t != head) else {
        return false;
    };
    // From where it came nearest, which is where the way is blocked: not
    // from wherever its wandering had taken it when the count ran out.
    if home_distance(head, target) > mind.home_best + HOME_PROGRESS {
        return false;
    }
    let aim = heading_toward(head, target);
    let (dx, dy) = DIRS[aim as usize];
    let cell = (head.0 + dx, head.1 + dy);
    if !in_door_scope(world, cell) || !jaw_can_cut(world, def, organism, world.get(cell.0, cell.1)) {
        return false;
    }
    if let Some(s) = world.organism_mut(organism) {
        s.heading = aim;
    }
    escape_cut(world, organism, def, head, aim)
}

/// **Near the door** ([`WalkParts::clear`]'s box): within [`CLEAR_COLS`]
/// columns of the nearest nest's founding cut (of the site's own column,
/// with no cut), and from [`CLEAR_ABOVE`] rows over the founding ground down
/// to [`CLEAR_BELOW`] rows under it, or to the cut's mouth if that is deeper.
/// It holds the way through the mound to the door and the door itself, and
/// none of the mound's flanks.
fn in_door_scope(world: &World, (x, y): (i32, i32)) -> bool {
    let Some(i) = world.nearest_nest_site(x, y) else {
        return false;
    };
    let site = &world.nest_sites[i];
    let (x0, x1, bottom) = match site.shaft {
        Some(c) => (c.x0, c.x1, (site.surface + CLEAR_BELOW).max(c.mouth_bottom)),
        None => (site.x, site.x, site.surface + CLEAR_BELOW),
    };
    (x0 - CLEAR_COLS..=x1 + CLEAR_COLS).contains(&x) && (site.surface - CLEAR_ABOVE..=bottom).contains(&y)
}

/// **Where an idle ant walking home aims** ([`Drive::Home`]): its home
/// vector's origin, the last nest contact (`home_target`), as slice 1 has it
/// -- except near the door under [`WalkParts::clear`], where it aims at the
/// door itself ([`door_cell`]).
///
/// **Why the door cut needs it.** The last nest contact is re-anchored at
/// every step that touches the nest, and the nest's reach covers the ground
/// round the door, so the anchor follows an idle ant along the surface: it is
/// home wherever it stands there and never tries to get in. Put down outside
/// a shut door in the guard's bed, an ant walking home re-anchored at every
/// step from (64, 39) to (59, 39) and back, 500 decisions and no stall, so
/// no cut (`the_door_cut_opens_a_shut_door`, first draft, 2026-10-07). The
/// design's idle ant walks home *and rests inside*; the door is a labelled
/// stand-in for the way in, as the nest's way is for the way out. A carrier
/// keeps the anchor: it delivers at the doorstep by design.
fn home_pull_target(world: &World, st: &crate::sim::organism::OrganismState, head: (i32, i32)) -> (i32, i32) {
    let near_door = world.needs.as_ref().is_some_and(|n| n.parts.clear) && in_door_scope(world, head);
    near_door
        .then(|| door_cell(world, head))
        .flatten()
        .unwrap_or_else(|| home_target(world, st))
}

/// Straight-line distance from `head` to `target`, in cells.
fn home_distance(head: (i32, i32), target: (i32, i32)) -> f32 {
    let (dx, dy) = ((head.0 - target.0) as f32, (head.1 - target.1) as f32);
    (dx * dx + dy * dy).sqrt()
}

/// **The door**, as [`home_pull_target`] aims at it: the founding cut's
/// first cell under its mouth, in its west column; with no cut, two rows
/// under the site's founding ground.
fn door_cell(world: &World, head: (i32, i32)) -> Option<(i32, i32)> {
    let site = &world.nest_sites[world.nearest_nest_site(head.0, head.1)?];
    Some(match site.shaft {
        Some(c) => (c.x0, c.mouth_bottom + 1),
        None => (site.x, site.surface + 2),
    })
}

/// **The dig job's stimulus** ([`WalkParts::dig_job`]; the design's *Jobs*
/// table): soil at its face -- 1 if its jaws could cut the cell its heading
/// points at -- plus the other adult animals with a cell within
/// [`DIG_REACH`] of its head, counted rather than divided by the open cells
/// round it, which would favour tunnel tips. It falls as the ants spread into
/// the room the digging makes.
fn dig_stimulus(world: &World, organism: OrganismId, def: &CreatureDef, head: (i32, i32), heading: u8) -> f32 {
    let (dx, dy) = DIRS[heading as usize % 8];
    let soil = jaw_can_cut(world, def, organism, world.get(head.0 + dx, head.1 + dy));
    f32::from(u8::from(soil)) + bodies_near(world, organism, head, DIG_REACH) as f32
}

/// Other adult animals with a cell within `reach` of `head`, each counted
/// once. Brood is not a body that crowds: a larva is carried, not met.
fn bodies_near(world: &World, organism: OrganismId, head: (i32, i32), reach: i32) -> u32 {
    let mut seen: Vec<OrganismId> = Vec::new();
    for y in head.1 - reach..=head.1 + reach {
        for x in head.0 - reach..=head.0 + reach {
            if !world.in_bounds(x, y) {
                continue;
            }
            let c = world.get(x, y);
            let id = c.organism_id();
            if id == 0 || id == organism || seen.contains(&id) || !is_animal_cell(world, c) {
                continue;
            }
            if world.organism(id).is_some_and(|s| s.brood.is_none()) {
                seen.push(id);
            }
        }
    }
    seen.len() as u32
}

/// Let go of the memories of ants no longer alive, every [`PRUNE_EVERY`]
/// frames.
fn prune(world: &mut World) {
    let frame = world.frame;
    let due = world.needs.as_ref().is_some_and(|n| frame >= n.pruned_at + PRUNE_EVERY);
    if !due {
        return;
    }
    let live: std::collections::HashSet<OrganismId> = world.live_organism_ids().into_iter().collect();
    if let Some(n) = world.needs.as_mut() {
        let died = n.minds.iter().filter(|&(id, m)| m.bout && !live.contains(id)).count() as u64;
        n.counts.bouts_died += died;
        n.minds.retain(|id, _| live.contains(id));
        n.pruned_at = frame;
    }
}

// --- the walk: the kinesis ----------------------------------------------

/// **Step or stay, by drive.** The jobs keep the brain's chance (and the
/// forage drive's pace); an idle ant inside steps less the nearer it is to
/// its own preferred depth; an idle ant outside walks home at
/// [`P_HOME`]; a need lifts the ant's chance toward 1 as its hunger rises.
///
/// **An empty forager walks at least at [`P_HOME`]** ([`WalkParts::pace`]):
/// the design lists "step or stay" as the walk's own, and slice 1 handed it
/// back to the brain, whose chance for a fed empty ant below the egg bar is
/// near zero in both walks -- the shipped ant outside moves because the
/// lay-home pull lifts it. Late on (seed 1, 80-100k) empty ants outside
/// stepped on 20-25% of their decisions against 41-51% shipped, and
/// forage-drive ants under 1,000 J on 5-9%; and scouting patience decays
/// only on a step, so a still forager never gives up (`results.md` steps
/// 6-7, the last by code-read). A flat floor while the job holds, not one
/// falling with the stimulus: the job's own end already stops them (Nest
/// building's review, Q2).
fn walk_kinesis(world: &World, head: (i32, i32), mind: &Mind, p_move: f32, parts: WalkParts) -> f32 {
    let depth = depth_of(world, head);
    let rest = match depth {
        Some(d) if d > 0 => {
            let t = ((d as f32 - (mind.pref - DEPTH_BAND)) / DEPTH_BAND).clamp(0.0, 1.0);
            P_SHALLOW + (P_DEEP - P_SHALLOW) * t
        }
        _ => P_HOME,
    };
    match mind.drive {
        Drive::Escape => 1.0,
        Drive::Eat => rest + (1.0 - rest) * mind.hunger.max(0.5),
        Drive::Out | Drive::Seek => rest + (1.0 - rest) * mind.hunger,
        Drive::Forage if parts.pace => p_move.max(P_HOME),
        Drive::Forage | Drive::Carry | Drive::Haul | Drive::Dig => p_move,
        Drive::Home => p_move.max(P_HOME),
        Drive::Rest => rest,
    }
}

/// **Rows below the founding ground** (the depth stand-in), or `None` with
/// no nest.
fn depth_of(world: &World, head: (i32, i32)) -> Option<i32> {
    let i = world.nearest_nest_site(head.0, head.1)?;
    Some(head.1 - world.nest_sites[i].surface)
}

// --- the walk: the step -------------------------------------------------

/// **The step, by the winning drive** -- `chooser_step`'s machinery (usable
/// headings, the crossing, the weighted pick, the commit, both patiences)
/// with the drive choosing the pull and which of today's terms count.
fn walk_step(
    world: &mut World,
    organism: OrganismId,
    heading: u8,
    outputs: &[f32; brain::BRAIN_OUTPUTS],
    def: &CreatureDef,
    draw: &mut rng::Rng,
) -> bool {
    let Some(mind) = mind_of(world, organism) else {
        return chooser_step(world, organism, heading, outputs, def, draw, Chooser::TrailAway);
    };
    let Some((chain, groups, fates)) = world
        .organism(organism)
        .map(|s| (s.chain.clone(), s.segment_groups.clone(), s.fates))
    else {
        note_outcome(world, DecisionOutcome::NoBody);
        return false;
    };
    let Some(&(hx, hy)) = chain.first() else {
        note_outcome(world, DecisionOutcome::NoBody);
        return false;
    };
    let usable = usable_headings(world, organism, def);
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
    let push = parting_enabled();
    let stacker = stacker_of(world, organism);
    let crossing = if crossing_enabled() && !usable.contains(&heading) {
        trunk_crossing(world, def, body, heading, push, stacker)
    } else {
        None
    };
    if usable.is_empty() && crossing.is_none() {
        return step_chain(world, organism, heading, outputs, def, draw);
    }
    let drive = mind.drive;
    // **The carry home fills up before it walks** (`CarryHome`'s `fill`,
    // part of the carry here whatever that switch says).
    if drive == Drive::Carry
        && fills_before_walking(
            world,
            organism,
            def,
            (hx, hy),
            outputs[brain::BrainOutput::Feed as usize],
        )
    {
        world.creature_stats.carry_fills += 1;
        note_outcome(world, DecisionOutcome::Filling);
        return false;
    }
    // **The trip back to the face ends**, as `chooser_step` ends it.
    if let Some(s) = world
        .organism(organism)
        .filter(|s| s.dig_return.is_some() && s.spoil.is_none())
    {
        let site = s.dig_return.expect("filtered above");
        if dig_trip_over(world, def, s, (hx, hy), site) {
            if let Some(s) = world.organism_mut(organism) {
                s.dig_return = None;
            }
        }
    }
    let st = world.organism(organism).expect("live: its chain was just read");
    let pull: Option<((i32, i32), f32)> = match drive {
        Drive::Eat => mind.food.map(|t| (t, FOOD_GAIN)),
        Drive::Out | Drive::Escape => {
            let (_, target) = out_target(world, organism, (hx, hy), &mind);
            let gain = def.home_bias.max(1.0) * mind.hunger.max(OUT_GAIN_MIN);
            Some((target.unwrap_or((hx, hy - COVER_REACH)), gain))
        }
        Drive::Carry => Some((home_target(world, st), def.home_bias.max(1.0))),
        Drive::Home => Some((home_pull_target(world, st, (hx, hy)), def.home_bias.max(1.0))),
        Drive::Haul => soil_way_pull(world, organism, def, st, (hx, hy))
            .or_else(|| spoil_haul_target(world, st, (hx, hy)))
            .map(|t| (t, spoil_haul().unwrap_or(1.0))),
        Drive::Dig => dig_return_target(world, def, st).map(|t| (t, spoil_haul().unwrap_or(1.0))),
        Drive::Seek | Drive::Forage | Drive::Rest => None,
    };
    // The pull's memory, kept exactly as `chooser_step` keeps it: started
    // again at every new target, cleared with no pull.
    let patience = {
        let state = world.organism_mut(organism).expect("live: its chain was just read");
        match pull {
            Some((target, _)) if state.home_best_for != target => {
                state.home_best_for = target;
                state.home_best = f32::INFINITY;
                state.home_away = 0;
                state.home_patience = 1.0;
                state.home_search_loops = 0;
            }
            Some(_) => {}
            None => {
                state.home_best = f32::INFINITY;
                state.home_away = 0;
                state.home_patience = 1.0;
                state.home_search_loops = 0;
            }
        }
        state.home_patience
    };
    let home_cos = |d: u8| -> Option<f32> {
        let ((ax, ay), _) = pull?;
        let (vx, vy) = ((ax - hx) as f32, (ay - hy) as f32);
        let len = (vx * vx + vy * vy).sqrt();
        if len < 1.0 {
            return None;
        }
        let (dx, dy) = DIRS[d as usize];
        Some((dx as f32 * vx + dy as f32 * vy) / (len * DIR_LEN[(d & 1) as usize]))
    };
    let persist = brain::unit_scale(outputs[brain::BrainOutput::Persist as usize], PERSIST_MAX);
    let turn = outputs[brain::BrainOutput::Turn as usize];
    let k = CHOICE_EXPLORATION_K * brain::unit_scale(outputs[brain::BrainOutput::Tumble as usize], 2.0);
    let walk = traits_of(world, organism, def);
    let trail_gain = TRAIL_GAIN * walk_gain(&walk, organism::TRAIT_TRAIL_HOLD);
    let away_gain = AWAY_GAIN * walk_gain(&walk, organism::TRAIT_ROUTE_AWAY);
    let decay = patience_decay_of(&walk);
    let gain = pull.map_or(0.0, |(_, g)| g * walk_gain(&walk, organism::TRAIT_HOME_PULL) * patience);
    let laden = world
        .organism(organism)
        .is_some_and(|s| s.crop.is_some_and(|c| c.worth() > 0.0) && !carries_lunch(world, s));
    let trip = world.organism(organism).is_some_and(|s| s.trip_load);
    // **Going out**: an empty forager, or a hungry ant outside, reads
    // today's away, scouting and door terms against home.
    let going_out = matches!(drive, Drive::Seek | Drive::Forage) && !laden;
    let away_from = if going_out {
        world.organism(organism).map(|s| home_target(world, s))
    } else {
        None
    };
    let want = match drive {
        Drive::Seek => mind.hunger,
        Drive::Forage => mind.hunger.max(FORAGE_WANT),
        _ => 0.0,
    };
    let scout_w = if away_from.is_some() {
        scout_of(world) * walk_gain(&walk, organism::TRAIT_SCOUT) * want
    } else {
        0.0
    };
    let away_home_cos = |d: u8| -> Option<f32> {
        let (ax, ay) = away_from?;
        let (vx, vy) = ((ax - hx) as f32, (ay - hy) as f32);
        let len = (vx * vx + vy * vy).sqrt();
        if len < 1.0 {
            return None;
        }
        let (dx, dy) = DIRS[d as usize];
        Some((dx as f32 * vx + dy as f32 * vy) / (len * DIR_LEN[(d & 1) as usize]))
    };
    // The excursion's memory, as `chooser_step` keeps it.
    let (scout_patience, scout_home, scout_dark) = match away_from {
        Some((ax, _)) if scout_w > 0.0 => {
            let st = world.organism_mut(organism).expect("live: its chain was just read");
            if st.scout_for != away_from.expect("matched Some") {
                st.scout_for = away_from.expect("matched Some");
                st.scout_best = (hx - ax).abs() as f32;
                st.scout_patience = 1.0;
                st.scout_home = false;
                st.scout_lit = false;
                st.scout_dark = false;
                st.scout_e0 = st.energy;
            }
            (st.scout_patience, st.scout_home, st.scout_dark)
        }
        _ => (1.0, false, false),
    };
    let door = if going_out && pull.is_none() && !scout_home && food_trail_of(world).read {
        walk_door_read(world, organism, def, (hx, hy), want)
            .map(|(side, f)| (side, f * walk_gain(&walk, organism::TRAIT_DOOR_READ)))
    } else {
        None
    };
    let door_walk = door
        .and_then(|_| door_site(world, hx, hy))
        .map(|i| world.nest_sites[i].surface - 1);
    let door_term = |d: u8| -> f32 {
        let (Some((side, f)), Some(walk)) = (door, door_walk) else {
            return 0.0;
        };
        let (dx, dy) = DIRS[d as usize];
        if dx == side && hy + dy <= walk {
            f
        } else {
            0.0
        }
    };
    let planes = trail_planes(&walk, laden);
    let route = |d: u8| trail_presence(world, (hx, hy), d, planes);
    let spent = scout_home && scout_dark && scout_w > 0.0 && food_trail_of(world).giveup;
    // **The carry home is held by a route's contrast, not its level**
    // (`CarryHome`'s `turn`, part of the carry here).
    let options: Vec<u8> = usable.iter().copied().chain(crossing.map(|_| heading)).collect();
    let route_floor = if drive == Drive::Carry && trip {
        let f = options.iter().map(|&d| route(d)).fold(f32::INFINITY, f32::min);
        if f.is_finite() {
            f
        } else {
            0.0
        }
    } else {
        0.0
    };
    let hold = |d: u8| {
        if spent {
            1.0
        } else {
            1.0 + trail_gain * (route(d) - route_floor).max(0.0)
        }
    };
    let scout_cos = |d: u8| -> Option<f32> {
        let (ax, _) = away_from?;
        let vx = ax - hx;
        if vx == 0 {
            return None;
        }
        let (dx, _) = DIRS[d as usize];
        Some((dx * vx.signum()) as f32 / DIR_LEN[(d & 1) as usize])
    };
    let score = |d: u8| -> f32 {
        let rel = (d + 8 - heading) % 8;
        let side = match rel {
            1..=3 => turn.max(0.0),
            5..=7 => (-turn).max(0.0),
            _ => 0.0,
        };
        persist * TURN_PREF[rel.min(8 - rel) as usize] * hold(d)
            + side
            + home_cos(d).map_or(0.0, |c| gain * c)
            + if spent {
                0.0
            } else {
                away_home_cos(d).map_or(0.0, |c| -away_gain * route(d) * c)
            }
            + if scout_w <= 0.0 {
                0.0
            } else if spent {
                away_home_cos(d).map_or(0.0, |c| scout_w * c)
            } else if scout_home {
                away_home_cos(d).map_or(0.0, |c| scout_w * (1.0 - route(d)) * c)
            } else {
                scout_cos(d).map_or(0.0, |c| -scout_w * scout_patience * (1.0 - route(d)) * c)
            }
            + door_term(d)
    };
    let scores: Vec<f32> = options.iter().map(|&d| score(d)).collect();
    let pick = choose_weighted(&scores, k, draw.unit_f32());
    let picked_route = route(options[pick]);
    if let Some((side, _)) = door {
        if pick < usable.len() && DIRS[options[pick] as usize].0 == side {
            world.creature_stats.door_followed += 1;
        }
    }
    world.creature_stats.carry_turns += u64::from(route_floor > 0.0);
    if pick == usable.len() {
        let (to, thickness) = crossing.expect("the last option is the crossing only when there is one");
        let due = world.frame + u64::from(thickness) * organism_tick_interval(world, organism, def);
        if let Some(state) = world.organism_mut(organism) {
            state.crossing = Some(organism::Crossing {
                to,
                heading,
                due,
                thickness,
            });
        }
        world.creature_stats.crossings += 1;
        note_outcome(world, DecisionOutcome::Crossing);
        return false;
    }
    commit_step(
        world,
        organism,
        def,
        body,
        &authored,
        &authored_widths,
        heading,
        options[pick],
        push,
    );

    // Did that step take the scout further out? `chooser_step`'s memory.
    let bound = food_trail_of(world).giveup;
    let noreturn = food_trail_of(world).noreturn;
    if let (Some((ax, _)), true) = (away_from, scout_w > 0.0 && !scout_home) {
        let state = world.organism_mut(organism).expect("live: it just stepped");
        let nx = state.chain.first().map_or(hx, |c| c.0);
        let level = (nx - ax).abs() as f32;
        if bound && picked_route >= HUNGRY_ROUTE && level > f32::from(FORAGE_TRIP_MIN) {
            state.scout_lit = true;
        }
        let on_trail = picked_route > 0.0;
        let stranded = noreturn && state.energy < state.scout_e0 - state.energy;
        let dark_past_a_trail = bound && state.scout_lit && !on_trail && !stranded;
        if level > state.scout_best + PATIENCE_PROGRESS && !dark_past_a_trail {
            state.scout_best = level;
            state.scout_patience = (state.scout_patience + PATIENCE_RECOVER).min(1.0);
        } else {
            state.scout_patience *= decay;
            if state.scout_patience < SCOUT_GIVE_UP {
                state.scout_home = true;
                state.scout_dark = bound && !on_trail;
            }
        }
    }
    // Did that step close on the pull's target? `chooser_step`'s memory.
    if let Some(((ax, ay), _)) = pull {
        let state = world.organism_mut(organism).expect("live: it just stepped");
        let (nx, ny) = state.chain.first().copied().unwrap_or((hx, hy));
        let (vx, vy) = ((ax - nx) as f32, (ay - ny) as f32);
        let dist = (vx * vx + vy * vy).sqrt();
        if dist < state.home_best - PATIENCE_PROGRESS {
            state.home_best = dist;
            state.home_best_at = (nx, ny);
            state.home_away = 0;
            state.home_patience = (state.home_patience + PATIENCE_RECOVER).min(1.0);
        } else {
            state.home_patience *= decay;
            let (bx, by) = state.home_best_at;
            let away = (nx - bx).abs().max((ny - by).abs()).clamp(0, u16::MAX as i32) as u16;
            state.home_away = state.home_away.max(away);
            if state.home_away >= EXCURSION_CELLS && away <= 1 {
                state.home_patience = 1.0;
                state.home_away = 0;
            }
        }
    }
    true
}

/// **The door reader** (`door_read`), on the walk's own want rather than
/// `outward_want`'s: an empty ant going out, in a door box, reads which side
/// the food trail is on. The same stale gate and the same reach-6 sensors.
fn walk_door_read(
    world: &World,
    organism: OrganismId,
    def: &CreatureDef,
    (hx, hy): (i32, i32),
    want: f32,
) -> Option<(i32, f32)> {
    let site = door_site(world, hx, hy)?;
    let st = world.organism(organism)?;
    if st.spoil.is_some() || want <= 0.0 {
        return None;
    }
    let window = match food_trail_of(world).window {
        w if w > 0.0 => w,
        _ => return_window(),
    };
    let (last, window) = if forage_drive_of(world).need == ForageNeed::Met {
        (
            return_met_frame(st),
            window * walk_gain(&traits_of(world, organism, def), organism::TRAIT_RETURN_MEMORY),
        )
    } else {
        (world.nest_last_return.get(site).copied().unwrap_or(0), window)
    };
    if last == 0 || world.frame.saturating_sub(last) as f32 > window {
        return None;
    }
    let so = def.sensor_offset;
    let b_e = f32::from(world.pheromone_at(Channel::B, hx + so, hy));
    let b_w = f32::from(world.pheromone_at(Channel::B, hx - so, hy));
    let g = (b_e - b_w) / (b_e + b_w + TRAIL_HALF);
    if g == 0.0 {
        return None;
    }
    Some((if g > 0.0 { 1 } else { -1 }, food_trail_of(world).gain * want * g.abs()))
}

// --- the walk: after ------------------------------------------------------

/// **After the step**: the way out's progress, which is what escape reads,
/// the way home's, which the door cut reads, and the trace row.
fn walk_after(world: &mut World, organism: OrganismId, moved: bool, won: bool) {
    let Some(mut mind) = mind_of(world, organism) else {
        return;
    };
    let parts = world.needs.as_ref().map_or(WalkParts::NONE, |n| n.parts);
    let head = world
        .organism(organism)
        .and_then(|s| s.chain.first().copied())
        .unwrap_or(mind.at);
    if matches!(mind.drive, Drive::Out | Drive::Escape) {
        let (kind, _) = out_target(world, organism, head, &mind);
        let score = out_score(world, head, kind, &mind);
        if kind != mind.out_kind {
            mind.out_kind = kind;
            mind.out_best = score;
            mind.stall = 0;
        } else if score < mind.out_best {
            mind.out_best = score;
            // **Ground it cut itself is not the way getting anywhere**: an
            // escaping ant that stepped into its own cut stays escaping, so
            // it puts the pellet down and cuts on. Reset here, it waited out
            // the whole stall again after every cut, and a 30%-budget ant in
            // the guard's mound pocket cut three cells in 530 frames and
            // starved with its jaws full, two rows short of the air (traced
            // 2026-10-07, `trace_one_guard_row`).
            if mind.drive != Drive::Escape {
                mind.stall = 0;
            }
        } else if won || !parts.won_stall {
            mind.stall = mind.stall.saturating_add(1);
        } else {
            // **A chosen stay is not a stall** ([`WalkParts::won_stall`]):
            // slice 1 counted every decision that got no nearer, so an ant
            // that mostly chose to stay -- the way out's chance is its
            // resting chance plus its hunger's share -- reached the stall on
            // standing still, and escape fired on a resting ant at hunger
            // 0.06. **Not "a refused step"**, the proposal's first form:
            // the walk offers only headings an ant can take, so a chosen step
            // is never refused (0 in about 147k shaft decisions, Nest
            // building's door-column traces), and a count of refusals would
            // never let escape fire at all (its review, must-fix 1). A won
            // roll that got no nearer -- nothing usable toward the way, or a
            // step that did not bring it closer -- still counts.
            world.needs.as_mut().expect("walking").counts.stays_not_stalls += 1;
        }
    } else {
        mind.out_kind = OUT_NONE;
        mind.out_best = i32::MAX;
        mind.stall = 0;
    }
    // **Sealed in** ([`WalkParts::sealed_out`]): the flood runs at the
    // [`ESCAPE_STALL`]th decision in a row without a step, then on every
    // [`SEAL_EVERY`]th, and after each step of an ant already sealed, to see
    // whether it got out. Capped so a resting crowd does not flood every
    // decision (the rev 2 review's change 5).
    if parts.sealed_out {
        // **Getting somewhere is leaving the neighbourhood, not `moved`**: an
        // ant in a pocket its own size turns end for end every decision,
        // which `moved` counts, so the first build's count never reached the
        // stall in the guard's fault-A row; and a traced pellet holder in a
        // 13-cell sponge pocket (steady seed 2, ant 4194677) walked round it
        // for 900 frames before its hunger's escape fired, then died packing
        // upward at 12 J (2026-10-11). The count starts again only when the
        // head is more than [`SEAL_ROAM`] cells from where it started.
        let roamed = (head.0 - mind.seal_from.0).abs().max((head.1 - mind.seal_from.1).abs()) > SEAL_ROAM;
        if roamed {
            mind.seal_from = head;
        }
        // **Wrapped, never saturated** (code review of PR 684): a `u16`
        // pinned at 65,535 is on neither cadence, so an ant still that long
        // -- a resting nest worker -- was never flooded again. Stepping
        // back by [`SEAL_EVERY`] past [`SEAL_STALL_TOP`] keeps both
        // cadences' phase.
        mind.seal_stall = if roamed {
            0
        } else if mind.seal_stall >= SEAL_STALL_TOP {
            mind.seal_stall + 1 - SEAL_EVERY
        } else {
            mind.seal_stall + 1
        };
        // Sealed, it floods again after each of its own cuts or steps, to see
        // whether it got out, as well as on the cadence.
        let acted = roamed || mind.cut != CUT_NONE;
        let due = if mind.sealed {
            acted || mind.seal_stall.is_multiple_of(SEAL_EVERY)
        } else {
            mind.seal_stall >= ESCAPE_STALL && (mind.seal_stall - ESCAPE_STALL).is_multiple_of(SEAL_EVERY)
        };
        if due {
            let found = seal_flood(world, head);
            let c = &mut world.needs.as_mut().expect("walking").counts;
            c.seal_floods += 1;
            match (mind.sealed, found) {
                (false, Some(cells)) => {
                    c.sealed_found += 1;
                    mind.sealed = true;
                    mind.seal_cut_n = 0;
                    mind.seal_cuts = [(i32::MIN, i32::MIN); 4];
                    mind.last_cut = None;
                    seal_log(world, organism, SEAL_FOUND, head, cells, 0);
                }
                (true, None) => {
                    if acted {
                        c.sealed_freed += 1;
                    } else {
                        c.sealed_opened += 1;
                    }
                    mind.sealed = false;
                    let event = if acted { SEAL_FREED } else { SEAL_OPENED };
                    seal_log(world, organism, event, head, 0, mind.seal_cut_n);
                }
                _ => {}
            }
        }
    }
    // **The eat drive's progress** ([`WalkParts::eat_fade`]): straight-line
    // distance to the food it sensed, whatever cell (the sense re-picks the
    // nearest every decision, so a per-target count would start again on the
    // next sealed crumb), counting only decisions that won the step roll, as
    // the way out's and the way home's do: a slow ant in a crowd is not
    // stalled by staying. An ant beside food that does not eat is: its mouth
    // had its turn. Kept across the drive's turns to "out", so an ant whose
    // box the crumb drifts in and out of still gives up; a meal, the open air
    // and the end of hunger start it again (`decide`).
    if parts.eat_fade && mind.drive == Drive::Eat {
        if let Some(food) = mind.food {
            let d = home_distance(head, food);
            if d < mind.eat_best - PATIENCE_PROGRESS {
                mind.eat_best = d;
                mind.eat_stall = 0;
            } else if won {
                mind.eat_stall = mind.eat_stall.saturating_add(1);
                if mind.eat_stall >= EAT_STALL {
                    mind.eat_rest_until = world.frame + EAT_REST;
                    mind.eat_best = f32::INFINITY;
                    mind.eat_stall = 0;
                    let c = &mut world.needs.as_mut().expect("walking").counts;
                    c.eat_gave_up += 1;
                    if mind.eat_bouts > 0 {
                        c.eat_gave_up_again += 1;
                    }
                    mind.eat_bouts = mind.eat_bouts.saturating_add(1);
                }
            }
        }
    }
    // **The way home's progress** ([`WalkParts::clear`]): straight-line
    // distance to the pull's own target, counting only decisions that won the
    // step roll, for the same reason. Within a cell of its target it has
    // arrived, not stalled. Straight-line, not steps: by steps every cell of
    // the ground over a door is as near the door as the one over its mouth,
    // and the first draft's stall ran out three columns off and cut a new
    // way in beside the seal (`the_door_cut_opens_a_shut_door`).
    match world
        .organism(organism)
        .filter(|_| parts.clear && mind.drive == Drive::Home)
    {
        Some(st) => {
            let target = home_pull_target(world, st, head);
            let d = home_distance(head, target);
            if mind.home_for != Some(target) || d < 1.5 {
                mind.home_for = Some(target);
                mind.home_best = d;
                mind.home_stall = 0;
            } else if d < mind.home_best - HOME_PROGRESS {
                mind.home_best = d;
                mind.home_stall = 0;
            } else if won {
                mind.home_stall = mind.home_stall.saturating_add(1);
            }
        }
        None => {
            mind.home_for = None;
            mind.home_best = f32::INFINITY;
            mind.home_stall = 0;
        }
    }
    if let Some(n) = world.needs.as_ref() {
        if n.traces(organism) {
            let st = world.organism(organism);
            let row = WalkRow {
                frame: world.frame,
                id: organism,
                at: mind.at,
                to: head,
                drive: mind.drive,
                job: mind.job,
                energy: st.map_or(f32::NAN, |s| s.energy),
                crop: st.and_then(|s| s.crop).map_or(0.0, |c| c.worth()),
                hunger: mind.hunger,
                hold: mind.hold,
                forage: mind.forage,
                threshold: FORAGE_THRESHOLD * mind.jitter,
                depth: depth_of(world, head).unwrap_or(i32::MIN),
                pref: mind.pref,
                p_move: mind.p_move,
                moved,
                stall: mind.stall,
                target: walk_target(world, organism, head, &mind),
                won,
                cut: mind.cut,
                scout_patience: st.map_or(f32::NAN, |s| s.scout_patience),
                scout_home: st.is_some_and(|s| s.scout_home),
                scout_dark: st.is_some_and(|s| s.scout_dark),
                scout_for: st.map_or((0, 0), |s| s.scout_for),
                esc_why: mind.esc_why,
                esc_nb: mind.esc_nb,
                sealed: mind.sealed,
            };
            world.needs.as_mut().expect("walking").rows.push(row);
        }
    }
    keep_mind(world, organism, mind);
}

/// Where the drive's pull aimed, for the trace: the same targets
/// `walk_step` pulls to, read again after the step.
fn walk_target(world: &World, organism: OrganismId, head: (i32, i32), mind: &Mind) -> Option<(i32, i32)> {
    let st = world.organism(organism)?;
    match mind.drive {
        Drive::Eat => mind.food,
        Drive::Out | Drive::Escape => out_target(world, organism, head, mind).1,
        Drive::Carry => Some(home_target(world, st)),
        Drive::Home => Some(home_pull_target(world, st, head)),
        Drive::Haul => world.organism(organism).and_then(|s| spoil_haul_target(world, s, head)),
        Drive::Dig => st.dig_return,
        Drive::Seek | Drive::Forage | Drive::Rest => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect;

    // --- the no-veto guard ----------------------------------------------------
    //
    // The design's *Measurement and acceptance* and the rule audit's §5 (2026-10-06):
    // an ant at 30% of its budget, in every place, holding every load, in either
    // role, must within a bounded number of frames have eaten, reached the open
    // air, or -- where nothing joins its pocket to the air -- be digging; and if it
    // let go of a load, the way out must still be open afterwards. **It asserts
    // what happened in the scene, never the sign of a weight**: the history's
    // vetoes were each a rule that let a weight stay positive while a gate
    // further down took the act away.

    /// The founding ground's top row: everything at or below it is ground.
    const GROUND: i32 = 40;

    /// How long a row has, in frames. An ant ticks about once in six frames
    /// and pays about a quarter of a joule a tick, so at 30% of the budget it
    /// has about 1,400 frames to live: a row that has met nothing by then has
    /// starved, and the bound only stops a row the ant survives.
    const BOUND: u64 = 3_000;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Place {
        /// Just inside the door, at the top of the founding shaft.
        Door,
        /// The founding chamber at the shaft's foot, 6-7 rows down.
        Chamber,
        /// A room 20-21 rows down, at the end of a shaft below the chamber.
        Deep,
        /// A tunnel through the spoil mound, open to the air at its west end.
        MoundTunnel,
        /// A closed pocket in the mound that nothing joins.
        MoundPocket,
        /// A closed pocket in the ground, 30 rows down, that nothing joins.
        Buried,
        /// On the open ground east of the door, short of the food heap: the
        /// food sense's check only (`the_food_sense_reaches_round_a_wall_and_not_through_soil`), never
        /// in [`Place::ALL`], since an ant there is out at its first frame.
        Surface,
    }

    impl Place {
        const ALL: [Place; 6] = [
            Place::Door,
            Place::Chamber,
            Place::Deep,
            Place::MoundTunnel,
            Place::MoundPocket,
            Place::Buried,
        ];

        /// Where the ant is put down.
        fn at(self) -> (i32, i32) {
            match self {
                Place::Door => (61, 41),
                Place::Chamber => (58, 47),
                Place::Deep => (60, 61),
                Place::MoundTunnel => (40, 37),
                Place::MoundPocket => (49, 33),
                Place::Buried => (21, 71),
                Place::Surface => (70, 39),
            }
        }

        /// Nothing joins it to the air, so digging is the need met.
        fn sealed(self) -> bool {
            matches!(self, Place::MoundPocket | Place::Buried)
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Load {
        Nothing,
        /// A pellet of dug soil in the jaws.
        Pellet,
        /// A cell of food picked up at home for the store, in the jaws.
        Food,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Role {
        Forager,
        /// Nest-bound for life, as one ant in four is today (`is_nest_bound`).
        NestWorker,
    }

    /// One row of the guard: what the ant did, and when.
    #[derive(Clone, Debug)]
    struct Row {
        place: Place,
        load: Load,
        role: Role,
        /// The first frame its crop held food or its energy rose.
        ate: Option<u64>,
        /// The first frame its head stood above the ground with nothing overhead.
        out: Option<u64>,
        /// The first frame it cut a cell, and how many it cut.
        dug: Option<u64>,
        digs: u64,
        died: Option<u64>,
        /// It let go of the load it started with.
        let_go: bool,
        /// An open way joined it to the open air at the start, and at the end.
        way_before: bool,
        way_after: bool,
        head: (i32, i32),
        /// Under the walk, what it did: decisions by drive and the rest, and
        /// every decision when the walk was built tracing.
        counts: Option<WalkCounts>,
        trace: Vec<WalkRow>,
    }

    /// Cells a sealed ant must cut for its digging to count: one cut is a
    /// roll that came up, three is an ant working at the wall.
    const DIGGING: u64 = 3;

    impl Row {
        /// **When its need was met**, if before it died. Eating always counts.
        /// The open air counts unless it holds food, which a starving ant eats
        /// rather than carries (the design's hunger: *eats what it holds*).
        /// Digging counts only where nothing joined it to the air.
        fn met(&self) -> Option<u64> {
            let out = self.out.filter(|_| self.load != Load::Food);
            let dug = self.dug.filter(|_| self.place.sealed() && self.digs >= DIGGING);
            [self.ate, out, dug]
                .into_iter()
                .flatten()
                .min()
                .filter(|&f| self.died.is_none_or(|d| f < d))
        }

        /// Met, and no load it let go of shut a way that was open.
        fn green(&self) -> bool {
            self.met().is_some() && !(self.let_go && self.way_before && !self.way_after)
        }
    }

    /// **The guard's bed**: `mound_world`'s ground and mound (`creature.rs`'s
    /// tests), with the rest of a nest under them and food on the surface.
    /// Loose soil from row 40 over a stone floor at 92; a nest site at column
    /// 60 whose founding cut is a shaft on columns 60-61 down to row 45 and a
    /// chamber on rows 46-47; a second shaft from the chamber's floor down to
    /// a room on rows 60-61; a closed pocket in the ground on rows 70-71; a
    /// mound of packed soil on columns 30-55 over rows 26-39 with a tunnel on
    /// rows 35-37 open to the air at column 29 and a closed pocket on rows
    /// 31-33; and a heap of player food 25 columns east of the door.
    ///
    /// **Every open cell in the ground is lined with packed soil**, as the
    /// founding cut lines its own and a digging ant lines the gallery it cuts
    /// (`soil.ron`'s `packs_into`): a hole in loose soil is gone in five frames
    /// (`packedsoil.ron`), and the first draft of this bed lost its shaft,
    /// chamber and the mound pocket's one-row roof that way, burying the ant
    /// it was testing. The mound is fourteen rows tall so that no roof over
    /// its holes is a single row: a packed cell with air beneath it holds on
    /// three contacts, and a one-row roof has two. **Every room is two rows
    /// tall**, as the founding chamber is: an ant on the floor of a taller one
    /// can step only along the floor until it finds a wall (its usable
    /// headings were east and west alone), so a shaft in the middle of a
    /// three-row room's ceiling is out of reach, and the first draft's deep
    /// room starved every empty ant in it for that reason, which is the
    /// room's shape and not a rule.
    fn guard_bed() -> World {
        let mut w = World::new(Rect::new(0, 0, 119, 99));
        w.brood = Some(false);
        w.bud_at_nest = Some(false);
        let soil = w.materials.id_of("soil").expect("soil material");
        let packed = w.materials.id_of("packedsoil").expect("packed soil material");
        let food = w.materials.id_of("provisions").expect("provisions material");
        for x in 0..=119 {
            for y in GROUND..=99 {
                let stone = y >= 92 || x == 0 || x == 119;
                w.set(
                    x,
                    y,
                    if stone {
                        Cell::new(material::STONE, 0)
                    } else {
                        Cell::new(soil, 0)
                    },
                );
            }
        }
        w.register_nest_site(60, 38, 2);
        let cut = crate::sim::world::ShaftFootprint {
            x0: 60,
            x1: 61,
            top: 40,
            bottom: 45,
            mouth_bottom: 41,
            chamber_x0: 56,
            chamber_x1: 65,
            chamber_top: 46,
            chamber_bottom: 47,
            side: None,
        };
        w.nest_sites[0].shaft = Some(cut);
        let open: Vec<(i32, i32)> = (40..=59)
            .flat_map(|y| [(60, y), (61, y)])
            .chain((46..=47).flat_map(|y| (56..=65).map(move |x| (x, y))))
            .chain((60..=61).flat_map(|y| (54..=67).map(move |x| (x, y))))
            .chain((70..=71).flat_map(|y| (18..=24).map(move |x| (x, y))))
            .collect();
        for &(x, y) in &open {
            w.set(x, y, Cell::EMPTY);
        }
        for &(x, y) in &open {
            for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                if w.get(x + dx, y + dy).material == soil {
                    w.set(x + dx, y + dy, Cell::new(packed, 0));
                }
            }
        }
        for y in 26..GROUND {
            for x in 30..=55 {
                let tunnel = (35..=37).contains(&y) && x <= 45;
                let pocket = (31..=33).contains(&y) && (46..=52).contains(&x);
                w.set(
                    x,
                    y,
                    if tunnel || pocket {
                        Cell::EMPTY
                    } else {
                        Cell::new(packed, 0)
                    },
                );
            }
        }
        for y in 37..GROUND {
            for x in 84..=90 {
                w.set(x, y, Cell::new(food, 0));
            }
        }
        w.step_nest_dug();
        step_nest_rest(&mut w);
        w
    }

    /// **The guard's bed stands with nobody in it.** The first draft's did not:
    /// its shaft, chamber and the mound pocket's roof fell in within five
    /// frames and buried the ant each row was testing, so 22 of 36 rows read
    /// red for the ground, not for any rule. Every cell the bed opens must
    /// still be open after `BOUND` frames, and every place must be where an
    /// ant can be put down. Watched red with the packed lining taken out.
    #[test]
    fn the_guard_bed_stands() {
        let mut w = guard_bed();
        let open: Vec<(i32, i32)> = (0..=119)
            .flat_map(|x| (GROUND..=91).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::EMPTY)
            .collect();
        let mound: Vec<(i32, i32)> = (30..=55)
            .flat_map(|x| (26..GROUND).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).material == material::EMPTY)
            .collect();
        // Shaft 40, chamber 16 beside it, deep room 28, buried pocket 14; the
        // mound's tunnel 48 and pocket 21.
        assert_eq!(
            (open.len(), mound.len()),
            (98, 69),
            "test setup: the bed did not open the cells it names"
        );
        for _ in 0..BOUND {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
        }
        let filled: Vec<(i32, i32)> = open
            .iter()
            .chain(&mound)
            .copied()
            .filter(|&(x, y)| w.get(x, y).material != material::EMPTY)
            .collect();
        assert!(
            filled.is_empty(),
            "{} of the bed's open cells filled in {BOUND} frames, first {:?}",
            filled.len(),
            filled.first()
        );
        for place in Place::ALL {
            let mut w = guard_bed();
            let (x, y) = place.at();
            assert!(
                plant_creature_seed(&mut w, x, y, "ant").is_some(),
                "no ant fits at {place:?} {:?}",
                place.at()
            );
            assert_eq!(
                open_to_the_air(&w, (x, y), w.get(x, y).organism_id()),
                !place.sealed(),
                "{place:?} is not as joined to the air as it is named"
            );
        }
    }

    /// One row: the bed, one ant at `place` at 30% of its budget holding
    /// `load` in `role`, run for [`BOUND`] frames under `walk`, with `setup`
    /// applied to the world first (a switch the arm runs under).
    fn guard_row(place: Place, load: Load, role: Role, walk: Option<NeedsMode>, setup: fn(&mut World)) -> Row {
        let mut w = guard_bed();
        setup(&mut w);
        let (x, y) = place.at();
        let site = plant_creature_seed(&mut w, x, y, "ant").expect("test setup: the ant does not fit");
        w.schedule_active_site(site);
        let a = w.get(x, y).organism_id();
        assert_ne!(a, 0, "test setup: no ant at {:?}", place.at());
        let soil = w.materials.id_of("soil").expect("soil material");
        let food = w.materials.id_of("provisions").expect("provisions material");
        let start = w
            .species
            .get(w.organism(a).expect("live").species)
            .creature
            .as_ref()
            .expect("a creature")
            .start_energy;
        let st = w.organism_mut(a).expect("live");
        st.energy = 0.3 * start;
        st.nest_bound_until = if role == Role::NestWorker { u64::MAX } else { 0 };
        st.spoil = match load {
            Load::Nothing => None,
            Load::Pellet => Some(crate::sim::organism::Spoil {
                cell: Cell::new(soil, 0),
                store: false,
            }),
            Load::Food => Some(crate::sim::organism::Spoil {
                cell: Cell::new(food, 0),
                store: true,
            }),
        };
        // A `setup` that built its own walk (the vetoed control) keeps it.
        if let Some(mode) = walk {
            w.needs = Some(Box::new(NeedsWalk::new(mode, w.frame)));
        }
        let digs = w.creature_stats.digs;
        let way_before = open_to_the_air(&w, (x, y), a);
        let mut row = Row {
            place,
            load,
            role,
            ate: None,
            out: None,
            dug: None,
            digs: 0,
            died: None,
            let_go: false,
            way_before,
            way_after: way_before,
            head: (x, y),
            counts: None,
            trace: Vec::new(),
        };
        let mut energy = 0.3 * start;
        for f in 0..BOUND {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
            let Some(st) = w.organism(a) else {
                row.died = Some(f);
                break;
            };
            row.head = st.chain[0];
            if row.ate.is_none() && (st.crop.is_some() || st.energy > energy + 1.0) {
                row.ate = Some(f);
            }
            energy = st.energy;
            let (hx, hy) = row.head;
            if row.out.is_none() && hy < GROUND && !under_cover(&w, hx, hy) {
                row.out = Some(f);
            }
            row.digs = w.creature_stats.digs - digs;
            if row.dug.is_none() && row.digs > 0 {
                row.dug = Some(f);
            }
            row.let_go |= load != Load::Nothing && st.spoil.is_none();
        }
        row.way_after = open_to_the_air(&w, row.head, a);
        row.counts = w.needs.as_ref().filter(|n| n.mode == NeedsMode::Walk).map(|n| n.counts);
        row.trace = w
            .needs
            .as_mut()
            .map(|n| std::mem::take(&mut n.rows))
            .unwrap_or_default();
        row
    }

    /// Whether a 4-connected walk over empty cells (and the ant's own body)
    /// joins `from` to a cell above the ground with nothing overhead.
    fn open_to_the_air(w: &World, from: (i32, i32), a: OrganismId) -> bool {
        let passable = |x: i32, y: i32| {
            (0..=119).contains(&x) && (0..=99).contains(&y) && {
                let c = w.get(x, y);
                c.material == material::EMPTY || c.organism_id() == a
            }
        };
        let mut seen = std::collections::HashSet::from([from]);
        let mut todo = vec![from];
        while let Some((x, y)) = todo.pop() {
            if y < GROUND && !under_cover(w, x, y) {
                return true;
            }
            for n in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if passable(n.0, n.1) && seen.insert(n) {
                    todo.push(n);
                }
            }
        }
        false
    }

    /// Every row of the guard under `walk`: six places, three loads, two roles.
    fn guard(walk: Option<NeedsMode>, setup: fn(&mut World)) -> Vec<Row> {
        Place::ALL
            .into_iter()
            .flat_map(|p| {
                [Load::Nothing, Load::Pellet, Load::Food]
                    .into_iter()
                    .map(move |l| (p, l))
            })
            .flat_map(|(p, l)| [Role::Forager, Role::NestWorker].into_iter().map(move |r| (p, l, r)))
            .map(|(p, l, r)| guard_row(p, l, r, walk, setup))
            .collect()
    }

    fn print_guard(rows: &[Row]) {
        let at = |f: Option<u64>| f.map_or("-".to_string(), |f| f.to_string());
        println!(
            "{:<12} {:<10} {:<11} {:>5} {:>5} {:>5} {:>4} {:>5} {:>7} {:>9} {:<10} verdict",
            "place", "load", "role", "ate", "out", "dug", "cuts", "died", "let go", "way b/a", "head"
        );
        for r in rows {
            println!(
                "{:<12} {:<10} {:<11} {:>5} {:>5} {:>5} {:>4} {:>5} {:>7} {:>9} {:<10} {}",
                format!("{:?}", r.place),
                format!("{:?}", r.load),
                format!("{:?}", r.role),
                at(r.ate),
                at(r.out),
                at(r.dug),
                r.digs,
                at(r.died),
                r.let_go,
                format!("{}/{}", u8::from(r.way_before), u8::from(r.way_after)),
                format!("{:?}", r.head),
                if r.green() { "green" } else { "RED" }
            );
            if let Some(c) = r.counts {
                let drives: Vec<String> = Drive::ALL
                    .iter()
                    .zip(c.decisions)
                    .filter(|&(_, n)| n > 0)
                    .map(|(d, n)| format!("{} {n}", d.label()))
                    .collect();
                println!(
                    "    walk: {} | ate held {} pellets down {} escape cuts {} packs {}",
                    drives.join(", "),
                    c.ate_held,
                    c.pellets_down,
                    c.escape_cuts,
                    c.escape_packs
                );
            }
        }
        println!(
            "{} of {} rows green",
            rows.iter().filter(|r| r.green()).count(),
            rows.len()
        );
    }

    /// **The guard on today's ant**, as a report: which places, loads and
    /// roles leave a hungry ant with no way to meet its need. Run with
    /// `cargo test --release --lib needs::tests::the_guard_on_todays_ant -- --ignored --nocapture`
    /// (about 2 s, debug or release).
    ///
    /// **Read 2026-10-06 on this branch (main 3ba1e7bd and switches that are
    /// off): 20 of 36 rows green, the same for foragers and nest workers but
    /// in two rows.** The
    /// 16 red, in four groups:
    /// - **Holding food, it starves** (door, chamber, deep room; 6 rows): it
    ///   puts its store load down, walks out and dies without eating.
    /// - **In the mound's tunnel, 11 cells from the air** (4 rows): nothing
    ///   pulls it out of the mound (`MOUND_OUT`'s `way` is off), and its walk
    ///   paces the tunnel's floor until it starves.
    /// - **In a pocket in the ground, lean, it never cuts** (4 rows): the lean
    ///   rule takes its dig roll, and `MOUND_OUT=dig` lets only the mound's
    ///   pockets off it.
    /// - **A forager in the mound's pocket stops after one cut** (2 rows); a
    ///   nest worker there cuts five.
    ///
    /// Under `NEEDS_FIRST` with every part on, 26 of 36: the buried pocket
    /// digs and the chamber's carrier eats its load
    /// ([`the_guard_under_needs_first`]).
    #[test]
    #[ignore = "a report, not a gate: prints the guard's table for today's ant"]
    fn the_guard_on_todays_ant() {
        print_guard(&guard(None, |_| {}));
    }

    /// **...and under `NEEDS_FIRST` with every part on**, the switch built to
    /// waive the blocks a hungry or shut-in ant meets: the rows it turns
    /// green are the guard seeing a block lifted.
    #[test]
    #[ignore = "a report, not a gate: prints the guard's table under NEEDS_FIRST"]
    fn the_guard_under_needs_first() {
        print_guard(&guard(None, |w| w.needs_first = Some(NeedsFirst::ON)));
    }

    /// **The guard under the walk: every row green**, the gate slice 1 has to
    /// pass before it runs a colony. `cargo test --release --lib
    /// needs::tests::the_guard_under_the_walk -- --nocapture` prints the table,
    /// with each row's decisions by drive under it.
    ///
    /// **Read 2026-10-07: 36 of 36** (today's ant 20, `NEEDS_FIRST` 26). Held
    /// food is eaten within 5 frames in every place; the deep room and the
    /// mound's tunnel walk out; both pockets cut. The mound's pocket gets out
    /// and eats (frames 545-647) only since the escape stopped waiting out
    /// its stall after every cut; before, it was green on three cuts and
    /// starved two rows short of the air. The buried pocket, 30 rows down at
    /// 60 J, still starves 10 rows short of the ground: a cut and a step cost
    /// it about 1.6 J, so the climb is past its budget whatever the rule.
    #[test]
    fn the_guard_under_the_walk() {
        let rows = guard(Some(NeedsMode::Walk), |_| {});
        print_guard(&rows);
        let red: Vec<String> = rows
            .iter()
            .filter(|r| !r.green())
            .map(|r| format!("{:?}/{:?}/{:?}", r.place, r.load, r.role))
            .collect();
        assert!(
            red.is_empty(),
            "{} of {} rows red under the walk: {red:?}",
            red.len(),
            rows.len()
        );
    }

    /// **...and its positive control: the same walk with its needs vetoed**,
    /// so every ant does its job or idles whatever its hunger. The guard must
    /// see it: rows go red that the walk turns green.
    #[test]
    fn the_guard_sees_a_vetoed_need() {
        let rows = guard(None, |w| {
            let mut n = NeedsWalk::new(NeedsMode::Walk, w.frame);
            n.veto_needs = true;
            w.needs = Some(Box::new(n));
        });
        print_guard(&rows);
        let red = rows.iter().filter(|r| !r.green()).count();
        assert!(
            red >= VETO_RED,
            "only {red} of {} rows red with the needs vetoed",
            rows.len()
        );
    }

    /// Rows the vetoed control must leave red. **Read 2026-10-07: 9 of 36**
    /// (holding food at the door, starving: 2; idle in the deep room, never
    /// leaving: 2; buried, never cutting: 4; a forager in the mound's pocket
    /// stopping after two cuts: 1), so the bar has headroom under that, not
    /// on it.
    const VETO_RED: usize = 5;

    /// **The guard under the walk with every part of the fix round on**
    /// ([`WalkParts::ALL`]): the fix round takes dig urges away from every
    /// drive but the dig job and changes what escape's stall counts, so the
    /// rows to watch are the two sealed pockets (Nest building's review,
    /// must-fix 1): escape must still fire there.
    #[test]
    fn the_guard_under_the_walk_with_every_fix() {
        let rows = guard(None, |w| {
            let mut n = NeedsWalk::new(NeedsMode::Walk, w.frame);
            n.parts = WalkParts::ALL;
            w.needs = Some(Box::new(n));
        });
        print_guard(&rows);
        let red: Vec<String> = rows
            .iter()
            .filter(|r| !r.green())
            .map(|r| format!("{:?}/{:?}/{:?}", r.place, r.load, r.role))
            .collect();
        assert!(
            red.is_empty(),
            "{} of {} rows red under the walk with every fix: {red:?}",
            red.len(),
            rows.len()
        );
    }

    // --- sealed in: `sealed_out` and `eat_held` (2026-10-11) -------------------
    //
    // The rev 2 review's guard rows (`review-eat-held-rev2-2026-10-11.md`): five
    // scenes a sealed ant must get out of, green as **its head reached the open
    // air before it died** -- not "ate" -- and three it must not cut in. Each
    // scene is built from the guard's bed with the ants put down first and every
    // empty cell of the scene's box but the ones named then filled with packed
    // soil, so the pocket is exactly what the row says. Run against `reach`, the
    // new ant's baseline, where each of the five is red first.

    /// One ant of a sealed row: where its head goes, what it holds, its energy
    /// as a share of its budget.
    #[derive(Clone, Copy, Debug)]
    struct SealAnt {
        at: (i32, i32),
        load: Load,
        energy: f32,
    }

    /// What a sealed row saw, per ant, and the walk's counters at the end.
    #[derive(Debug)]
    struct SealRun {
        /// The first frame each ant's head stood in the open air, and its death.
        out: Vec<Option<u64>>,
        died: Vec<Option<u64>>,
        /// Cells cut over the run (`creature_stats.digs`).
        digs: u64,
        counts: WalkCounts,
        /// Sealed decisions, and those whose escape's cell could not be cut.
        sealed_decisions: u64,
        jaw_refused: u64,
        /// How far the live identity (live energy against the ledger's
        /// expected total) moved over the run: a meal eaten from the jaws
        /// must be booked at digestion like any other (the review's change 3).
        ledger_drift: f64,
    }

    impl SealRun {
        /// Every ant reached the open air before it died.
        fn all_out(&self) -> bool {
            self.out
                .iter()
                .zip(&self.died)
                .all(|(o, d)| o.is_some_and(|o| d.is_none_or(|d| o < d)))
        }
    }

    /// The scene: the guard's bed, `ants` put down, then every empty cell in
    /// `fill` (inclusive box) that is not in `keep` filled with packed soil,
    /// `edit` applied, and the walk run for `frames` under `parts`.
    fn seal_row(
        ants: &[SealAnt],
        fill: ((i32, i32), (i32, i32)),
        keep: &[(i32, i32)],
        edit: fn(&mut World),
        parts: WalkParts,
        frames: u64,
    ) -> SealRun {
        let mut w = guard_bed();
        let soil = w.materials.id_of("soil").expect("soil material");
        let packed = w.materials.id_of("packedsoil").expect("packed soil material");
        let food = w.materials.id_of("provisions").expect("provisions material");
        let mut ids = Vec::new();
        let mut colony = None;
        for a in ants {
            // One colony, so nestmates do not fight (`STRANGER_ALARM`): the
            // first draft's second ant was killed by the first at frame 5.
            let site = plant_creature_seed_in(&mut w, a.at.0, a.at.1, "ant", colony)
                .expect("test setup: the ant does not fit");
            w.schedule_active_site(site);
            let id = w.get(a.at.0, a.at.1).organism_id();
            assert_ne!(id, 0, "test setup: no ant at {:?}", a.at);
            colony = colony.or(w.organism(id).map(|s| s.colony));
            let start = w
                .species
                .get(w.organism(id).expect("live").species)
                .creature
                .as_ref()
                .expect("a creature")
                .start_energy;
            let st = w.organism_mut(id).expect("live");
            st.energy = a.energy * start;
            st.spoil = match a.load {
                Load::Nothing => None,
                Load::Pellet => Some(crate::sim::organism::Spoil {
                    cell: Cell::new(soil, 0),
                    store: false,
                }),
                Load::Food => Some(crate::sim::organism::Spoil {
                    cell: Cell::new(food, 0),
                    store: true,
                }),
            };
            ids.push(id);
        }
        let ((x0, y0), (x1, y1)) = fill;
        for x in x0..=x1 {
            for y in y0..=y1 {
                if w.get(x, y).material == material::EMPTY && !keep.contains(&(x, y)) {
                    w.set(x, y, Cell::new(packed, 0));
                }
            }
        }
        edit(&mut w);
        let mut n = NeedsWalk::new(NeedsMode::Walk, w.frame);
        n.parts = parts;
        n.trace_ids = ids.iter().copied().collect();
        w.needs = Some(Box::new(n));
        let digs = w.creature_stats.digs;
        let gap = |w: &World| w.live_creature_energy() - w.energy_ledger.expected_live_total();
        let gap0 = gap(&w);
        let mut out = vec![None; ids.len()];
        let mut died = vec![None; ids.len()];
        let (mut sealed_decisions, mut jaw_refused) = (0, 0);
        for f in 0..frames {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
            for (i, &id) in ids.iter().enumerate() {
                match w.organism(id) {
                    None => {
                        died[i].get_or_insert(f);
                    }
                    Some(st) => {
                        let (hx, hy) = st.chain[0];
                        if out[i].is_none() && hy < GROUND && !under_cover(&w, hx, hy) {
                            out[i] = Some(f);
                        }
                    }
                }
            }
            if let Some(n) = w.needs.as_mut() {
                for r in n.rows.drain(..) {
                    if r.sealed {
                        sealed_decisions += 1;
                        jaw_refused += u64::from(r.esc_why == 2);
                    }
                }
            }
            if out.iter().all(Option::is_some) {
                break;
            }
        }
        SealRun {
            out,
            died,
            digs: w.creature_stats.digs - digs,
            counts: w.needs.as_ref().expect("walking").counts,
            sealed_decisions,
            jaw_refused,
            ledger_drift: gap(&w) - gap0,
        }
    }

    /// The new ant's baseline, and the same with both parts on.
    const SEAL_BASE: WalkParts = WalkParts {
        reach: true,
        ..WalkParts::NONE
    };
    const SEAL_ON: WalkParts = WalkParts {
        reach: true,
        sealed_out: true,
        eat_held: true,
        ..WalkParts::NONE
    };

    /// Frames a sealed row runs: long enough for a fed ant to go hungry and
    /// for a store load eaten from the jaws to digest.
    const SEAL_FRAMES: u64 = 8_000;

    /// The mound's pocket (rows 31-33, columns 46-52) and its box.
    const POCKET_BOX: ((i32, i32), (i32, i32)) = ((43, 27), (55, 39));

    /// The cells an ant put down at `at` stands in, read off a fresh bed: the
    /// pocket a row keeps open is its body plus what the row names.
    fn body_at(at: (i32, i32)) -> Vec<(i32, i32)> {
        let mut w = guard_bed();
        plant_creature_seed(&mut w, at.0, at.1, "ant").expect("test setup: the ant does not fit");
        let id = w.get(at.0, at.1).organism_id();
        w.organism(id).expect("live").chain.clone()
    }

    fn print_seal(name: &str, r: &SealRun) {
        let c = &r.counts;
        println!(
            "{name:<34} out {:?} died {:?} digs {} ledger {:+.4} | floods {} found {} freed {} opened {} cuts {} packs {} turned {} drops {} loads down {} recut {} ate jaws {} refused {} laden soil {} | sealed decisions {} jaw refused {}",
            r.out,
            r.died,
            r.digs,
            r.ledger_drift,
            c.seal_floods,
            c.sealed_found,
            c.sealed_freed,
            c.sealed_opened,
            c.sealed_cuts,
            c.sealed_packs,
            c.seal_aim_turned,
            c.sealed_drops,
            c.sealed_load_down,
            c.seal_recut,
            c.ate_held_jaws,
            c.eat_held_refused,
            c.laden_soil,
            r.sealed_decisions,
            r.jaw_refused
        );
    }

    /// The five scenes a sealed ant must get out of, by name.
    fn sealed_rows(parts: WalkParts) -> Vec<(&'static str, SealRun)> {
        let at = (49, 33);
        let body = body_at(at);
        // A pocket three cells longer than the ant, on one row: it turns and
        // shuffles in it and is never in a perfect shell (`walled_in`).
        let spare: Vec<(i32, i32)> = (47..=53).map(|x| (x, 33)).collect();
        // Two ants nose to tail in a pocket that holds just the two.
        let (a, b) = ((48, 33), (50, 33));
        let mut two = body_at(a);
        two.extend(body_at(b));
        // One spare cell on the head's side, which crumbs fill.
        let mut crumbed = body.clone();
        crumbed.push((body[0].0 + 1, body[0].1));
        let hungry = 0.3;
        let fed = 1.0;
        vec![
            (
                "1 hungry, store load, no free cell",
                seal_row(
                    &[SealAnt {
                        at,
                        load: Load::Food,
                        energy: hungry,
                    }],
                    POCKET_BOX,
                    &body,
                    |_| {},
                    parts,
                    SEAL_FRAMES,
                ),
            ),
            (
                "2 hungry, soil, pocket 7 long",
                seal_row(
                    &[SealAnt {
                        at,
                        load: Load::Pellet,
                        energy: hungry,
                    }],
                    POCKET_BOX,
                    &spare,
                    |_| {},
                    parts,
                    SEAL_FRAMES,
                ),
            ),
            (
                "3 two ants in one pocket, soil",
                seal_row(
                    &[
                        SealAnt {
                            at: a,
                            load: Load::Pellet,
                            energy: hungry,
                        },
                        SealAnt {
                            at: b,
                            load: Load::Pellet,
                            energy: hungry,
                        },
                    ],
                    POCKET_BOX,
                    &two,
                    |_| {},
                    parts,
                    SEAL_FRAMES,
                ),
            ),
            (
                "4 hungry, soil, crumbs beside head",
                seal_row(
                    &[SealAnt {
                        at,
                        load: Load::Pellet,
                        energy: hungry,
                    }],
                    POCKET_BOX,
                    &crumbed,
                    |w| {
                        let crumbs = w.materials.id_of("crumbs").expect("crumbs material");
                        let head = body_at((49, 33))[0];
                        w.set(head.0 + 1, head.1, Cell::new(crumbs, 0).with_aux(8));
                        assert!(
                            food_value(w, w.get(head.0 + 1, head.1)) > 0.0,
                            "test setup: the crumbs are worth nothing"
                        );
                    },
                    parts,
                    SEAL_FRAMES,
                ),
            ),
            (
                "6 just fed, soil, deep pocket",
                seal_row(
                    &[SealAnt {
                        at: (21, 71),
                        load: Load::Pellet,
                        energy: 0.7,
                    }],
                    ((0, 0), (-1, -1)),
                    &[],
                    |_| {},
                    parts,
                    SEAL_FRAMES,
                ),
            ),
            (
                "7 two just fed, soil, deep pocket",
                seal_row(
                    &[
                        SealAnt {
                            at: (19, 71),
                            load: Load::Pellet,
                            energy: 0.7,
                        },
                        SealAnt {
                            at: (23, 71),
                            load: Load::Pellet,
                            energy: 0.7,
                        },
                    ],
                    ((0, 0), (-1, -1)),
                    &[],
                    |_| {},
                    parts,
                    SEAL_FRAMES,
                ),
            ),
            (
                "8 just fed, empty, deep pocket",
                seal_row(
                    &[SealAnt {
                        at: (21, 71),
                        load: Load::Nothing,
                        energy: 0.7,
                    }],
                    ((0, 0), (-1, -1)),
                    &[],
                    |_| {},
                    parts,
                    SEAL_FRAMES,
                ),
            ),
            (
                "5 fed, store load, no free cell",
                seal_row(
                    &[SealAnt {
                        at,
                        load: Load::Food,
                        energy: fed,
                    }],
                    POCKET_BOX,
                    &body,
                    |_| {},
                    parts,
                    SEAL_FRAMES,
                ),
            ),
        ]
    }

    /// **The five sealed scenes, against the baseline and with both parts
    /// on**, as a report: `cargo test --release --lib
    /// needs::tests::the_sealed_rows -- --ignored --nocapture`.
    #[test]
    #[ignore = "a report: prints the sealed rows on reach and with sealed_out,eat_held"]
    fn the_sealed_rows() {
        for (arm, parts) in [("reach", SEAL_BASE), ("reach,sealed_out,eat_held", SEAL_ON)] {
            println!("== {arm}");
            for (name, r) in sealed_rows(parts) {
                print_seal(name, &r);
            }
        }
    }

    /// **Every sealed scene gets out with both parts on**, and the jaw's
    /// refusal is under a tenth of the sealed decisions (the rev 2 review's
    /// changes 1-2).
    #[test]
    fn sealed_ants_get_out() {
        let rows = sealed_rows(SEAL_ON);
        for (name, r) in &rows {
            print_seal(name, r);
        }
        for (name, r) in &rows {
            assert!(r.all_out(), "{name}: not out before it died: {r:?}");
            assert!(
                r.jaw_refused * 10 < r.sealed_decisions.max(1),
                "{name}: the jaw refused {} of {} sealed decisions",
                r.jaw_refused,
                r.sealed_decisions
            );
            assert!(
                r.counts.seal_recut <= 2,
                "{name}: {} cells cut again",
                r.counts.seal_recut
            );
            assert!(
                r.ledger_drift.abs() < 0.5,
                "{name}: the live identity moved {:.3} J",
                r.ledger_drift
            );
        }
    }

    /// **...and its positive control: on the baseline both store-load rows
    /// starve in their pockets.** The other rows get out on the baseline too
    /// -- the shipped pack and the hungry escape free a pellet holder from a
    /// synthetic pocket, which the colony's traced pockets did not
    /// (`trace-walled-in-2026-10-11.md` §1B; read 2026-10-11) -- so they are
    /// reported, not gated: the deep ones get out in 1,223-2,273 frames on
    /// the baseline against 371-407 with the parts, once hungry rather than
    /// while fed, and one of the two deep ants starved at frame 1,847 after
    /// reaching the air hungry. The same rows also prove `sealed_out` off
    /// floods nothing.
    #[test]
    fn sealed_rows_are_red_on_the_baseline() {
        let rows = sealed_rows(SEAL_BASE);
        for (name, r) in &rows {
            print_seal(name, r);
        }
        // With `sealed_out` off the walk never floods and reads no decision as
        // sealed, in any row: the part's whole cost and effect sit behind its
        // own flag (PR 684's code review, item 6).
        for (name, r) in &rows {
            assert_eq!(r.counts.seal_floods, 0, "{name}: flooded with sealed_out off");
            assert_eq!(r.counts.sealed_found, 0, "{name}: found sealed with sealed_out off");
            assert_eq!(r.sealed_decisions, 0, "{name}: a sealed decision with sealed_out off");
        }
        for (name, r) in rows
            .iter()
            .filter(|(n, _)| ["1 ", "5 "].iter().any(|p| n.starts_with(p)))
        {
            assert!(
                !r.all_out(),
                "{name}: out on the baseline, so the row does not test the part: {r:?}"
            );
        }
    }

    /// **Where a sealed ant must not cut**: a fed ant resting at the dead end
    /// of the mound's tunnel, whose way runs out of the flood's box; a fed ant
    /// in the chamber with four nestmates packed round it (ants stand in for
    /// the brood pillar: both pass the flood); and an ant shut in the door's
    /// shaft, which the doorway rule keeps from cutting. No sealed cut in any.
    #[test]
    fn sealed_out_leaves_open_and_doorway_ants_alone() {
        let fed = 1.0;
        let tunnel = seal_row(
            &[SealAnt {
                at: (44, 37),
                load: Load::Nothing,
                energy: fed,
            }],
            ((0, 0), (-1, -1)),
            &[],
            |_| {},
            SEAL_ON,
            3_000,
        );
        print_seal("7 dead end of the mound's tunnel", &tunnel);
        let crowd = seal_row(
            &[
                SealAnt {
                    at: (57, 47),
                    load: Load::Nothing,
                    energy: fed,
                },
                SealAnt {
                    at: (59, 47),
                    load: Load::Nothing,
                    energy: fed,
                },
                SealAnt {
                    at: (61, 47),
                    load: Load::Nothing,
                    energy: fed,
                },
                SealAnt {
                    at: (63, 47),
                    load: Load::Nothing,
                    energy: fed,
                },
                SealAnt {
                    at: (65, 47),
                    load: Load::Nothing,
                    energy: fed,
                },
            ],
            ((0, 0), (-1, -1)),
            &[],
            |_| {},
            SEAL_ON,
            3_000,
        );
        print_seal("8 crowded chamber", &crowd);
        let door = body_at((61, 41));
        let shut = seal_row(
            &[SealAnt {
                at: (61, 41),
                load: Load::Nothing,
                energy: fed,
            }],
            ((54, 35), (68, 47)),
            &door,
            |_| {},
            SEAL_ON,
            3_000,
        );
        print_seal("9 shut in the doorway", &shut);
        for (name, r) in [("tunnel", &tunnel), ("crowd", &crowd)] {
            assert_eq!(r.counts.sealed_found, 0, "{name}: read as sealed");
            assert_eq!(r.counts.sealed_cuts + r.counts.sealed_packs, 0, "{name}: cut sealed");
        }
        assert_eq!(
            shut.counts.sealed_cuts + shut.counts.sealed_packs,
            0,
            "doorway: cut sealed"
        );
    }

    /// The parts parse as a harness names them, label back the same way, and
    /// none is slice 1.
    #[test]
    fn walk_parts_parse_and_label() {
        assert_eq!(WalkParts::parse("all"), WalkParts::ALL);
        assert_eq!(WalkParts::parse("none"), WalkParts::NONE);
        assert_eq!(WalkParts::parse(""), WalkParts::NONE);
        assert_eq!(NeedsWalk::new(NeedsMode::Walk, 0).parts, WalkParts::NONE);
        let p = WalkParts::parse("pace, give_up");
        assert!(p.pace && p.give_up && !p.clear && !p.only_diggers);
        assert_eq!(p.label(), "pace,give_up");
        assert_eq!(WalkParts::parse(&WalkParts::ALL.label()), WalkParts::ALL);
        assert_eq!(WalkParts::NONE.label(), "none");
        // `all` is the fix round's eight; the parts after it are named alone.
        let all = WalkParts::parse("all");
        assert!(!all.reach && !all.eat_fade);
        let p = WalkParts::parse("all,reach,eat_fade");
        assert!(p.reach && p.eat_fade && p.won_stall);
        assert_eq!(WalkParts::parse(&p.label()), p);
        // The new ant: `reach` and the `walled_in` switch's two parts, and
        // nothing of the fix round's.
        assert_eq!(WalkParts::parse("new_ant"), WalkParts::NEW_ANT);
        assert_eq!(WalkParts::parse("reach,walled_in"), WalkParts::NEW_ANT);
        assert_eq!(WalkParts::NEW_ANT.label(), "reach,sealed_out,eat_held");
        assert_eq!(WalkParts::parse(&WalkParts::NEW_ANT.label()), WalkParts::NEW_ANT);
        let w = WalkParts::parse("walled_in");
        assert!(w.sealed_out && w.eat_held && !w.reach);
    }

    #[test]
    #[should_panic(expected = "is not all, none, new_ant, walled_in or one of")]
    fn walk_parts_refuse_an_unknown_name() {
        WalkParts::parse("pace,giveup");
    }

    /// **`eat_held` alone is refused at the parse** (PR 684's code review,
    /// item 2): it eats only where `sealed_out` has found the ant sealed in,
    /// so on its own it would be an arm that measures nothing.
    #[test]
    #[should_panic(expected = "eat_held needs sealed_out")]
    fn walk_parts_refuse_eat_held_alone() {
        WalkParts::parse("reach,eat_held");
    }

    /// **A crop holding another food refuses the jaws' load**, and leaves
    /// both the crop and the load as they were: one crop holds one material
    /// (`crop_take` would otherwise blend two foods' worth into one unit).
    #[test]
    fn eat_from_jaws_refuses_a_crop_of_another_food() {
        let mut w = guard_bed();
        let food = w.materials.id_of("provisions").expect("provisions material");
        let corpse = w.materials.id_of("corpse").expect("corpse material");
        let at = (49, 33);
        let site = plant_creature_seed_in(&mut w, at.0, at.1, "ant", None).expect("test setup: the ant does not fit");
        w.schedule_active_site(site);
        let id = w.get(at.0, at.1).organism_id();
        assert_ne!(id, 0, "test setup: no ant at {at:?}");
        let def = w
            .species
            .get(w.organism(id).expect("live").species)
            .creature
            .clone()
            .expect("a creature");
        let held = crate::sim::organism::Crop {
            material: corpse,
            shade: 0,
            unit: 10.0,
            cells: 1,
            digesting: 0.0,
            passenger: None,
        };
        let load = crate::sim::organism::Spoil {
            cell: Cell::new(food, 0),
            store: true,
        };
        {
            let st = w.organism_mut(id).expect("live");
            st.crop = Some(held);
            st.spoil = Some(load);
        }
        let (bites, eats) = (w.organism(id).expect("live").life.bites, w.creature_stats.eats);
        assert!(
            !eat_from_jaws(&mut w, id, &def, at, load),
            "ate provisions into a crop of corpse"
        );
        let st = w.organism(id).expect("live");
        let crop = st.crop.expect("the crop was emptied");
        assert_eq!((crop.material, crop.cells), (corpse, 1), "the crop changed");
        assert_eq!(st.spoil.map(|s| s.cell.material), Some(food), "the load was dropped");
        assert_eq!(st.life.bites, bites, "a bite was counted");
        assert_eq!(w.creature_stats.eats, eats, "an eat was counted");
    }

    /// **The steered ant gets none of the walk's hooks** under
    /// [`World::needs`] (PR 684's code review, item 6): marked after the
    /// walk has given her a mind, she loses it at her next decision and never
    /// gets another, while the colony around her keeps walking.
    #[test]
    fn the_steered_ant_is_not_walked() {
        use crate::lab::scenario::{Placement, Scenario};
        use crate::sim::steer::{mark, SteerInput};
        let mut sc = Scenario::load("steady_income").expect("steady_income ships");
        for e in &mut sc.timeline {
            if matches!(e.what, Placement::Colony { .. } | Placement::Heap { .. }) {
                e.at = 200;
            }
        }
        let mut lab = crate::lab::Lab::new(sc.bed.clone());
        lab.load_scenario(sc);
        for _ in 0..400 {
            lab.tick_for_harness();
        }
        let mut n = NeedsWalk::new(NeedsMode::Walk, lab.world.frame);
        n.parts = WalkParts::NEW_ANT;
        lab.world.needs = Some(Box::new(n));
        for _ in 0..300 {
            lab.tick_for_harness();
        }
        let walked = |w: &World| {
            w.needs
                .as_ref()
                .expect("walking")
                .minds
                .keys()
                .copied()
                .collect::<Vec<_>>()
        };
        let mut minds = walked(&lab.world);
        minds.sort_unstable();
        let id = *minds
            .iter()
            .find(|&&i| lab.world.organism(i).is_some())
            .expect("the walk gave nobody a mind");
        let held = SteerInput {
            dir: Some((1, 0)),
            ..Default::default()
        };
        assert!(mark(&mut lab.world, id, Some(held)));
        for _ in 0..600 {
            lab.tick_for_harness();
        }
        assert!(lab.world.organism(id).is_some(), "test setup: the steered ant died");
        assert_eq!(
            lab.world.steered.as_ref().map(|s| s.used),
            Some(held),
            "test setup: the steered ant never decided"
        );
        assert!(
            !walked(&lab.world).contains(&id),
            "the walk kept a mind for the steered ant"
        );
        assert!(!walked(&lab.world).is_empty(), "the colony stopped walking");
    }

    /// **The door cut's box**: the way through the mound over the door and
    /// the door itself, none of the mound's flanks and none of the nest.
    /// The guard's bed has its founding cut on columns 60-61 under ground
    /// row 40.
    #[test]
    fn the_door_box_holds_the_door_and_not_the_mound() {
        let w = guard_bed();
        let surface = w.nest_sites[0].surface;
        for (cell, inside) in [
            ((60, surface - 10), true),
            ((61, surface + 1), true),
            ((55, surface - 2), true),
            ((66, surface - 2), true),
            ((54, surface - 2), false),
            ((67, surface - 2), false),
            ((40, 37), false),
            ((60, 61), false),
            ((60, surface - CLEAR_ABOVE - 1), false),
        ] {
            assert_eq!(in_door_scope(&w, cell), inside, "{cell:?} (surface row {surface})");
        }
    }

    /// One ant, fed and empty, put down outside a nest whose door is shut:
    /// the bed's founding cut sealed across its top row, one cell thick as
    /// four of seed 1's six seals were (`results.md` step 4), the ant's home
    /// (its last nest contact) inside below the seal. Run under the walk with
    /// `parts` for [`BOUND`] frames; what it cut, and whether the door opened.
    /// A two-row seal takes two cuts and the haul trip between them, about
    /// 400 frames over the mound and back in the first draft's trace.
    fn shut_door_row(parts: WalkParts) -> (WalkCounts, bool, Vec<WalkRow>) {
        let mut w = guard_bed();
        let packed = w.materials.id_of("packedsoil").expect("packed soil material");
        for x in 60..=61 {
            w.set(x, 40, Cell::new(packed, 0));
        }
        let site = plant_creature_seed(&mut w, 64, 39, "ant").expect("test setup: the ant does not fit");
        w.schedule_active_site(site);
        let a = w.get(64, 39).organism_id();
        assert_ne!(a, 0, "test setup: no ant");
        w.organism_mut(a).expect("live").forage_anchor = (60, 44);
        let mut n = NeedsWalk::new(NeedsMode::Walk, w.frame);
        n.parts = parts;
        n.trace_every = 1;
        w.needs = Some(Box::new(n));
        let shut = |w: &World| (60..=61).all(|x| w.get(x, 40).material == packed);
        assert!(shut(&w), "test setup: the door is not shut");
        let mut rows = Vec::new();
        for _ in 0..BOUND {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
            rows.append(&mut w.needs.as_mut().expect("walking").rows);
            if w.organism(a).is_none() {
                break;
            }
        }
        (w.needs.as_ref().expect("walking").counts, !shut(&w), rows)
    }

    /// **The door cut opens a shut door, and nothing else does** with only
    /// the dig job digging: the ant walking home stalls on the seal and cuts
    /// toward home. Its control, the same walk without the cut, never cuts.
    #[test]
    fn the_door_cut_opens_a_shut_door() {
        let only = WalkParts {
            only_diggers: true,
            ..WalkParts::NONE
        };
        let (c, opened, rows) = shut_door_row(WalkParts { clear: true, ..only });
        if std::env::var("NEEDS_DEBUG").is_ok() {
            for r in &rows {
                println!(
                    "{} {:?}->{:?} {} {} won {} moved {} cut {} target {:?} p {:.2}",
                    r.frame,
                    r.at,
                    r.to,
                    r.drive.label(),
                    r.job.label(),
                    r.won,
                    r.moved,
                    r.cut,
                    r.target,
                    r.p_move
                );
            }
        }
        let home = rows.iter().filter(|r| r.drive == Drive::Home).count();
        assert!(
            c.clear_cuts >= 1 && opened,
            "door cuts {}, door opened {opened}, {home} of {} decisions walking home, cuts by drive {:?}",
            c.clear_cuts,
            rows.len(),
            c.cuts
        );
        assert!(
            rows.iter().filter(|r| r.cut == CUT_CLEAR).all(in_door_scope_of_row),
            "a door cut outside the door's box"
        );
        let (c, opened, rows) = shut_door_row(only);
        assert!(
            c.clear_cuts == 0 && c.cuts.iter().sum::<u64>() == 0 && !opened,
            "control: door cuts {}, cuts {:?}, door opened {opened} over {} decisions",
            c.clear_cuts,
            c.cuts,
            rows.len()
        );
    }

    /// A door cut row's ant stood next to the cell it cut, which was in the
    /// box; read back loosely, as "its head was within a cell of the box".
    fn in_door_scope_of_row(r: &WalkRow) -> bool {
        let w = guard_bed();
        NEIGHBOURS_8
            .iter()
            .any(|&(dx, dy)| in_door_scope(&w, (r.at.0 + dx, r.at.1 + dy)))
    }

    /// **The dig job can be taken on its stimulus at all**: fed, empty ants
    /// put down together in the bed's deep room take it and cut, with only the
    /// dig job digging; with the stimulus off nobody does. A counter, not a
    /// picture -- the fix round's dig job is a new mechanism, and "did it fire"
    /// is its first question.
    #[test]
    fn the_dig_job_is_taken_in_a_crowd() {
        let run = |parts: WalkParts| -> WalkCounts {
            let mut w = guard_bed();
            let mut ants = 0;
            for x in (54..=67).step_by(2) {
                for y in [60, 61] {
                    if w.get(x, y).material == material::EMPTY {
                        if let Some(site) = plant_creature_seed(&mut w, x, y, "ant") {
                            w.schedule_active_site(site);
                            ants += 1;
                        }
                    }
                }
            }
            assert!(ants >= 4, "test setup: only {ants} ants fit in the deep room");
            let mut n = NeedsWalk::new(NeedsMode::Walk, w.frame);
            n.parts = parts;
            w.needs = Some(Box::new(n));
            for _ in 0..BOUND {
                crate::sim::update::step(&mut w);
                w.step_active_sites();
            }
            w.needs.as_ref().expect("walking").counts
        };
        let only = WalkParts {
            only_diggers: true,
            ..WalkParts::NONE
        };
        let on = run(WalkParts { dig_job: true, ..only });
        let dig = Drive::Dig as usize;
        println!(
            "dig job on: took {}, quit {}, tired {}, cuts by drive {:?}",
            on.dig_took, on.dig_quit, on.dig_tired, on.cuts
        );
        assert!(
            on.dig_took > 0 && on.cuts[dig] > 0,
            "the dig job was taken {} times and cut {} cells",
            on.dig_took,
            on.cuts[dig]
        );
        assert_eq!(
            on.cuts.iter().sum::<u64>(),
            on.cuts[dig],
            "a drive other than the dig job cut with only_diggers on: {:?}",
            on.cuts
        );
        let off = run(only);
        assert!(
            off.dig_took == 0 && off.cuts.iter().sum::<u64>() == 0,
            "control: took {}, cuts {:?}",
            off.dig_took,
            off.cuts
        );
    }

    // --- food sealed in soil (`reach`, `eat_fade`, 2026-10-10) -------------

    /// An arm of a row: its name, and the setup it runs under.
    type Arm = (&'static str, fn(&mut World));

    /// The walk with `parts` on, for a row's setup.
    fn walk_with(w: &mut World, parts: &str) {
        let mut n = NeedsWalk::new(NeedsMode::Walk, w.frame);
        n.parts = WalkParts::parse(parts);
        w.needs = Some(Box::new(n));
    }

    /// **A crumb sealed in soil** in the deep shaft's east wall, two columns
    /// from the shaft and six rows above the deep room's floor, so that the
    /// room and the shaft below the chamber are all within the food sense's
    /// box: the trace's crumb at (250, 201), six columns from the flip
    /// world's shaft, held about 40 ants. **The first place tried, three rows
    /// under the room's floor, trapped nobody**: the ant on slice 1 paced the
    /// room on the eat drive, drifted up the shaft out of the box in 210
    /// frames, went out and ate, so the row could not be red for the fault.
    /// In the shaft's wall slice 1 presses into the wall beside it on 151
    /// eat decisions and starves at frame 905.
    const SEALED_CRUMB: (i32, i32) = (63, 55);

    fn bury_crumb(w: &mut World) {
        let food = w.materials.id_of("provisions").expect("provisions material");
        w.set(SEALED_CRUMB.0, SEALED_CRUMB.1, Cell::new(food, 0));
    }

    /// The deep room's rows, each role, with the crumb buried, under `setup`.
    fn sealed_crumb_rows(setup: fn(&mut World)) -> Vec<Row> {
        [Role::Forager, Role::NestWorker]
            .into_iter()
            .map(|role| guard_row(Place::Deep, Load::Nothing, role, None, setup))
            .collect()
    }

    /// **A hungry ant beside food sealed in soil must eat or reach the air**,
    /// the row the no-veto guard lacked: none of its scenes put food in sealed
    /// soil beside a hungry ant. **Watched red on slice 1** (the positive
    /// control, asserted: the ant aims at the crumb and starves), green with
    /// either part and both.
    #[test]
    fn a_crumb_sealed_in_soil_does_not_hold_a_hungry_ant() {
        let mut w = guard_bed();
        bury_crumb(&mut w);
        let (cx, cy) = SEALED_CRUMB;
        assert!(
            NEIGHBOURS_8
                .iter()
                .all(|&(dx, dy)| w.get(cx + dx, cy + dy).material != material::EMPTY),
            "test setup: the crumb is not sealed"
        );
        let (ax, ay) = Place::Deep.at();
        assert!(
            (cx - ax).abs().max((cy - ay).abs()) <= FOOD_REACH,
            "test setup: the crumb is out of the food sense's reach"
        );
        let arms: [Arm; 4] = [
            ("slice 1", |w| {
                bury_crumb(w);
                walk_with(w, "none");
            }),
            ("reach", |w| {
                bury_crumb(w);
                walk_with(w, "reach");
            }),
            ("eat_fade", |w| {
                bury_crumb(w);
                walk_with(w, "eat_fade");
            }),
            ("both", |w| {
                bury_crumb(w);
                walk_with(w, "reach,eat_fade");
            }),
        ];
        let mut wrong = Vec::new();
        for (name, setup) in arms {
            println!("--- {name}");
            let rows = sealed_crumb_rows(setup);
            print_guard(&rows);

            for r in &rows {
                if let Some(c) = r.counts {
                    println!(
                        "    reach hidden {} | eat gave up {} again {} | sense off {} ended by a meal {} by the air {}",
                        c.reach_hidden, c.eat_gave_up, c.eat_gave_up_again, c.eat_rest, c.eat_rest_ate, c.eat_rest_out
                    );
                }
                let want = name != "slice 1";
                if r.green() != want {
                    wrong.push(format!("{name}/{:?}: green {}", r.role, r.green()));
                }
            }
        }
        assert!(wrong.is_empty(), "rows not as expected: {wrong:?}");
    }

    /// **What `reach` senses** (the review's negative control, and a tight
    /// check on a deterministic function): on the open ground, food beyond a
    /// two-cell post of packed soil, where the straight line is soil and the
    /// way is air over the post, is sensed with `reach` as without it; the
    /// same crumb sealed in soil is sensed only without it, and `reach` says
    /// so. A first form of this check, a row that let the ant walk to the
    /// crumb, could not fail: with the sense blinded entirely, the ant outside
    /// walked onto the crumb on its search anyway (eaten at frame 41).
    #[test]
    fn the_food_sense_reaches_round_a_wall_and_not_through_soil() {
        let mut w = guard_bed();
        let packed = w.materials.id_of("packedsoil").expect("packed soil material");
        let food = w.materials.id_of("provisions").expect("provisions material");
        let (x, y) = Place::Surface.at();
        plant_creature_seed(&mut w, x, y, "ant").expect("test setup: the ant does not fit");
        let a = w.get(x, y).organism_id();
        let def = w
            .species
            .get(w.organism(a).expect("live").species)
            .creature
            .clone()
            .expect("a creature");
        let head = w.organism(a).expect("live").chain[0];
        w.set(head.0 + 3, head.1, Cell::new(packed, 0));
        w.set(head.0 + 3, head.1 - 1, Cell::new(packed, 0));
        w.set(head.0 + 5, head.1, Cell::new(food, 0));
        let round = (head.0 + 5, head.1);
        assert_eq!(sense_food(&w, a, &def, head, false), (Some(round), false));
        assert_eq!(
            sense_food(&w, a, &def, head, true),
            (Some(round), false),
            "food round a wall hidden"
        );
        // Bury it: soil over and round it, the post left standing.
        for (dx, dy) in NEIGHBOURS_8 {
            if w.get(round.0 + dx, round.1 + dy).material == material::EMPTY {
                w.set(round.0 + dx, round.1 + dy, Cell::new(packed, 0));
            }
        }
        assert_eq!(sense_food(&w, a, &def, head, false), (Some(round), false));
        assert_eq!(
            sense_food(&w, a, &def, head, true),
            (None, true),
            "food sealed in soil sensed"
        );
    }

    // **No crowd row for `eat_fade`** (the review asked for a reachable meal
    // approached slowly through a crowd, which the give-up must not drop).
    // Built three ways in the deep room and none could fail: with strangers
    // the ant was killed at frame 59; with fed nestmates it was fed by them
    // at frame 5 (an adult shares with any poorer nestmate beside it); with
    // nestmates as hungry as it, it ate at frame 305 under slice 1, under
    // `eat_fade`, and under `eat_fade` with the stall cut to 1 decision (gave
    // up twice, ate at the same frame: the mouth takes food it passes on any
    // drive). So it is judged in the colony instead: give-ups, repeat
    // give-ups per hunger bout, and windows ended by a meal (`walk_counts`).

    /// **The three walk guards with each new part on** (the review's ask):
    /// every row green under the walk and under every fix, and the vetoed
    /// control still red, with `reach` and with `eat_fade` -- and with the
    /// hunger bout (`hunger_holds`) and its `placebo` (2026-10-10).
    #[test]
    fn the_guards_hold_with_the_food_parts() {
        let arms: [Arm; 8] = [
            ("reach", |w| walk_with(w, "reach")),
            ("eat_fade", |w| walk_with(w, "eat_fade")),
            ("all,reach", |w| walk_with(w, "all,reach")),
            ("all,eat_fade", |w| walk_with(w, "all,eat_fade")),
            ("hunger_holds", |w| walk_with(w, "hunger_holds")),
            ("reach,hunger_holds", |w| walk_with(w, "reach,hunger_holds")),
            ("all,hunger_holds", |w| walk_with(w, "all,hunger_holds")),
            ("placebo", |w| walk_with(w, "placebo")),
        ];
        let mut red = Vec::new();
        for (name, setup) in arms {
            let rows = guard(None, setup);
            for r in rows.iter().filter(|r| !r.green()) {
                red.push(format!("{name}: {:?}/{:?}/{:?}", r.place, r.load, r.role));
            }
        }
        let vetoes: [Arm; 3] = [
            ("reach", |w| {
                walk_with(w, "reach");
                w.needs.as_mut().expect("walking").veto_needs = true;
            }),
            ("eat_fade", |w| {
                walk_with(w, "eat_fade");
                w.needs.as_mut().expect("walking").veto_needs = true;
            }),
            ("hunger_holds", |w| {
                walk_with(w, "hunger_holds");
                w.needs.as_mut().expect("walking").veto_needs = true;
            }),
        ];
        let mut blind = Vec::new();
        for (name, setup) in vetoes {
            let n = guard(None, setup).iter().filter(|r| !r.green()).count();
            if n < VETO_RED {
                blind.push(format!("{name}: {n} red"));
            }
        }
        assert!(red.is_empty(), "{} rows red with a food part: {red:?}", red.len());
        assert!(
            blind.is_empty(),
            "the vetoed control went blind with a food part: {blind:?}"
        );
    }

    /// One ant in the deep room under `parts`, at `share` of its budget, its
    /// walk made to hold the forage job from its first decision on, run for
    /// `frames`: decisions it spent on the forage drive while hungry, and
    /// decisions on a hunger drive.
    fn forager_held(parts: &str, share: f32, frames: u64) -> (u32, u32) {
        let mut w = guard_bed();
        walk_with(&mut w, parts);
        let (x, y) = Place::Deep.at();
        let site = plant_creature_seed(&mut w, x, y, "ant").expect("test setup: the ant does not fit");
        w.schedule_active_site(site);
        let a = w.get(x, y).organism_id();
        let start = w
            .species
            .get(w.organism(a).expect("live").species)
            .creature
            .as_ref()
            .expect("a creature")
            .start_energy;
        let st = w.organism_mut(a).expect("live");
        st.energy = share * start;
        st.nest_bound_until = 0;
        st.scout_patience = 1.0;
        let (mut on_job, mut on_need) = (0, 0);
        let mut seen = 0;
        for _ in 0..frames {
            crate::sim::update::step(&mut w);
            w.step_active_sites();
            if w.organism(a).is_none() {
                break;
            }
            let frame = w.frame;
            let n = w.needs.as_mut().expect("walking");
            let Some(m) = n.minds.get_mut(&a) else {
                continue;
            };
            // **The flip, made certain**: the forage job is what the roll
            // gives a hungry ant on about one decision in 2,500; here it has
            // it from the start, and keeps it, as a forager does.
            if m.job != Job::Forage {
                m.job = Job::Forage;
                m.since = frame;
            }
            let decided = n.counts.decisions.iter().sum::<u64>();
            if decided == seen {
                continue;
            }
            seen = decided;
            if m.drive == Drive::Forage && m.hunger > 0.0 {
                on_job += 1;
            }
            if matches!(m.drive, Drive::Eat | Drive::Out | Drive::Seek | Drive::Escape) {
                on_need += 1;
            }
        }
        (on_job, on_need)
    }

    /// **A hungry ant that takes the forage job stays hungry** (`hunger_holds`,
    /// the review's guard rows (a) and (b), 2026-10-10). An ant at half its
    /// budget (hunger about 0.2) in the deep room, holding the forage job:
    /// - **(a) watched red without the part, asserted**: the job's hold (up to
    ///   0.8) is over its hunger, so it forages hungry (97 decisions, 3 on a
    ///   hunger drive before it held the job); with the part it spends no
    ///   hungry decision on the forage drive (0, and 91 on hunger drives).
    /// - **(b) the negative control**: fed (hunger 0), the part changes
    ///   nothing -- it forages on the job, as it does without the part.
    #[test]
    fn a_hungry_forager_stays_hungry() {
        const FRAMES: u64 = 600;
        let (job_off, need_off) = forager_held("none", 0.5, FRAMES);
        let (job_on, need_on) = forager_held("hunger_holds", 0.5, FRAMES);
        println!("hungry: slice 1 forage {job_off} need {need_off}; hunger_holds forage {job_on} need {need_on}");
        assert!(
            job_off > 10 && need_off * 10 < job_off,
            "the positive control went blind: slice 1 forage {job_off} need {need_off}"
        );
        assert!(
            job_on == 0 && need_on > 10,
            "hunger_holds let a hungry ant forage: forage {job_on} need {need_on}"
        );
        let fed_off = forager_held("none", 1.0, FRAMES);
        let fed_on = forager_held("hunger_holds", 1.0, FRAMES);
        println!("fed: slice 1 {fed_off:?}, hunger_holds {fed_on:?}");
        assert_eq!(fed_off, fed_on, "hunger_holds changed a fed forager");
        assert_eq!(fed_on.1, 0, "a fed forager took a hunger drive");
    }

    /// **One row of the guard under the walk, every decision printed**: the
    /// per-ant trace for a row that needs understanding. Name the row in
    /// `NEEDS_GUARD_ROW` as `place,load,role` (`MoundPocket,Nothing,Forager`),
    /// and the fix round's parts, if any, in `NEEDS_PARTS` (`all`, or a comma
    /// list).
    #[test]
    #[ignore = "a tool, not a gate: prints one guard row's every decision"]
    fn trace_one_guard_row() {
        let want = std::env::var("NEEDS_GUARD_ROW").unwrap_or_else(|_| "MoundPocket,Nothing,Forager".to_string());
        let parts: Vec<&str> = want.split(',').map(str::trim).collect();
        let place = *Place::ALL
            .iter()
            .find(|p| format!("{p:?}") == parts[0])
            .expect("a place");
        let load = *[Load::Nothing, Load::Pellet, Load::Food]
            .iter()
            .find(|l| format!("{l:?}") == parts[1])
            .expect("a load");
        let role = *[Role::Forager, Role::NestWorker]
            .iter()
            .find(|r| format!("{r:?}") == parts[2])
            .expect("a role");
        let row = guard_row(place, load, role, None, |w| {
            let mut n = NeedsWalk::new(NeedsMode::Walk, w.frame);
            n.trace_every = 1;
            n.parts = std::env::var("NEEDS_PARTS").map_or(WalkParts::NONE, |p| WalkParts::parse(&p));
            w.needs = Some(Box::new(n));
        });
        print_guard(std::slice::from_ref(&row));
        println!(
            "frame  at        to        drive   job     energy  crop   hunger hold  depth p_move moved won cut stall target"
        );
        for t in &row.trace {
            println!(
                "{:>6} {:<9} {:<9} {:<7} {:<7} {:>6.1} {:>6.1} {:>6.3} {:>5.2} {:>5} {:>6.3} {:>5} {:>3} {:>3} {:>5} {:?}",
                t.frame,
                format!("{:?}", t.at),
                format!("{:?}", t.to),
                t.drive.label(),
                t.job.label(),
                t.energy,
                t.crop,
                t.hunger,
                t.hold,
                t.depth,
                t.p_move,
                t.moved,
                u8::from(t.won),
                t.cut,
                t.stall,
                t.target
            );
        }
    }
}
