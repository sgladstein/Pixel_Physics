# Queen seat: adversarial feasibility review (2026-10-25)

Reviewed: `seat-demos/designs/queen.md` (whole), `teaching-plan.md` §2-§7, against `main` 9e32979e plus the
parent's uncommitted demo edits. Line numbers are for `main` unless marked. Measurements were made with a scratch
harness built in a `git archive HEAD` copy in the scratchpad. No repo file was left changed.

## 1. Verdict: feasible with changes

**The queen seat can be a playable demo, but not the one the design describes.** It counts as "mostly works on today's
weights" several levers that are dead in the default lab. Provision, the walk genes, brood-following, the shelf, and
tandem memory all live on the hand-built brain or on zeroed weights. Trained ants never mutate in the box, so
"choosing who lays" can only re-sort 30 near-copies of one pool. On the other side, the slow clock is faster than the
design feared: one generation is about 10-12k frames (about 3 minutes at 1x, under a minute at 4x). Laying itself
(who lays, how fast, where) is a real, immediate and graded lever. A demo built on laying rate, choosing whose mind
each egg carries, an in-box mutation switch and feeding the larvae is buildable in days.

## 2. Mechanic by mechanic

"Today" means the default lab: `bin/lab.rs:296-307` gives every ant the trained pool (`std_pool.txt`, 30 members,
layout 4) at start.

| Mechanic | Engine support today (file:line) | Gap | Teaches on today's ants? |
|---|---|---|---|
| **Steerable queen body** | `sim::steer`; the hooks are on the hand-built path, `creature.rs:8171`. A mind ant returns at `creature.rs:7998-8022` (`mind::tick`) before that hook, so **the keys are ignored on a trained ant** | Hours. The parent's uncommitted edit routes a keyboard-held mind ant down the hand-built path | n/a (it is the body) |
| **Lay (L), and lay faster or slower** | `steered_outputs` sets `Lay` (`creature.rs:27740`). The lay bar is 1,100 J and an egg costs 120 J (`ant.ron:122-130`). The queen rule applies to her colony (`regime_of_colony`, `creature.rs:23039`) | None | **Yes, strongly.** Measured below: under the queen rule the colony's whole birth rate is one ant's laying |
| **Choose who lays / whose line an egg carries** | No verb. The queen rule is "first to the bar, then incumbency" (`creature.rs:22966-22978`). An egg inherits the layer's mind (`creature.rs:6087-6089`, `brood.rs:1595`, `mind.rs:1728-1750`) | Hours to a day. At lay time, give the egg a nominated worker's mind `Arc` instead of hers. No crossover is needed for a demo | **Only as frequency-sorting among ≤30 near-identical minds** (see §3.2) |
| Patrilines with crossover (A1/A4) | No crossover in the box. "A mind does not mutate at a birth in the box" (`lab/mod.rs:1026-1030`) | Multi-day (crossover exists only in training code) | Same limit as above until mutation is on |
| Wild clutch (A7) | The mutation operator exists, `training.rs:401` (`neutral_mutate`), but is never called at a box birth | Hours to call it on an egg's mind, plus a decision that overrides "minds evolve on the TRAINING page" | **This is the only source of new variation in a session** |
| Provision dial (A8/B5) | **Dead three ways.** The mind passes `provision = 0.0` (`mind.rs:2605`). `ant.ron` has no `plastic` block, so the developmental weights are 0 and `expressed_traits` returns early (`creature.rs:5225-5229`). And no trait is body size: a `Chain(2)` body is fixed (`organism.rs:7503-7928`) | Multi-day. Thread a player number into `made`, author developmental weights (a design choice), and render the result | **No.** "Big and small siblings" cannot appear |
| Walk-gene vocabulary (queen.md §1.3) | Mind ants skip the hand-built walk: "no sense, no brain.rs, no act, no walk" (`creature.rs:7993-7996`) | Not applicable | **No.** These genes are inert on trained ants |
| Feed a worker in the act (F) | `Share` is on F (`creature.rs:27738`). The target is whichever hungry nestmate the body finds, not one the player picks | Hours to target one ant; favour is new bookkeeping | **Inverse effect risk.** A fed trained ant is the "fed ants idle on the mound" failure, so the reward can sedate the act it rewards |
| Bite a worker (B) | `nearest_foe` skips kin (`creature.rs:10979-11012`), so **B on a nestmate does nothing** | About a day for a kin bite with knockback and energy loss. "Chastened" (veto an option for N frames) is another day | A visible impulse, but no lasting change: no learning |
| Carry brood / brood as gravity (C1) | No queen verb carries brood, and the jaw refuses brood (`brood.rs` header). `BROOD_CARRY` is an off-by-default teleport rule that moved 0-19 brood and gained nothing (`dead-ends.md` 1244). Brood is a powder and falls 38-40 rows under the door (`dead-ends.md` 1248). The mind's home is `forage_anchor` (`options.rs:458`), not brood, and brood is a target-cell feature only (`options.rs:151,201`) | Multi-day, with an unknown payoff | **Unmeasured. Probably little**: nothing in the mind is pulled toward a pile it is not touching |
| Feed the larvae ("what young are fed") | Works now. A larva eats food beside it (`brood.rs:976-990`), and nurse-by-touch runs on the larva's side whatever brain the nurse has (`brood.rs:991-993`, 1036ff). Q drops crop food | None | **Yes, but only speed.** It decides when, not what, the young become |
| Where she lays: warm, deep, by the door (C2) | Egg and pupa stages are fixed frame counts (`ant.ron:126-128`), not tied to temperature. Larvae starve where they fall | New (temperature-tied growth) | Placement changes survival, not quality |
| Presence and absence (B6/U6) | No queen field. The closest is graded suppression: the bar ×1.25 inside 24 cells of a breeder (`creature.rs:23059-23086`). That is about breeding, not foraging age, and it is off for her colony under the queen rule | About a day: switch her colony to graded while she is away | Real but small. The ×1.25 was tuned down from 6× because higher values stopped workers fetching |
| Tandem run (B1) | No follow state. `FOOD_WAY` is zeroed for every layout < 5 pool (`mind.rs:539-552`), and all four shipped pools are layout 4 | Research (memory layer, plan §7) | **No.** Its knowledge write is dead on arrival; only a scripted follow is possible |
| Deeds ledger / bless via the dream (A3/A5/U3) | Life counters are moves, bites, digs, deliveries, trip deliveries and offspring (`organism.rs:7219-7260`). No "larvae fed" or "beetles bitten". No per-act clips. The steer log says "no replay has been shown yet" (`steer.rs:118-125`) | Ledger: hours. A dream replay: multi-day (snapshot and deterministic re-simulation, or recorded clips) | Only as an interface onto the who-lays verb |
| Ancestor shelf as father store (A9) | `specimen.rs` stores no mind. A released jar gets a pool mind by `id % 30` from the 16-frame sweep (`lab/mod.rs:1089-1104`, `training.rs:392`) | About a day to save and restore the mind `Arc` | **No, today.** A shelved champion comes back as a random pool member |
| Egg-eating / policing (A6) | Brood carries no food value (`brood.rs` header) | Hours to a day | n/a |
| Cull (existing tool) | `Tool::Cull` (`ui.rs:771`, `lab/mod.rs:2878`) | None | It is a selection verb today, with the same limit of 30 minds |

**The parent's other finding is confirmed.** `mind.rs` and `options.rs` contain no reference to dig company,
`RECRUIT`, the forage drive, the nest store, larva scent or nurse-stay, so all of those hand-built rules are dead for
trained ants. Two brood-side rules still apply, because they run on the larva's tick: `brood::nurse` and larvae
eating food beside them. The new edit that makes a keyboard-held mind ant take the hand-built path (uncommitted,
`creature.rs` near 7998) needs two checks:

- while steered, her eggs must still inherit her mind, not lose it;
- the 16-frame sweep (`lab/mod.rs:1951`) must not replace her mind.

## 3. Must-fix: claims that are wrong on today's engine

1. **"A steerable queen body exists" (plan §2).** It exists only for the hand-built brain. In the default lab the
   keys were dead (`creature.rs:8022` returns before `take_input` at `:8171`). Slice 0's founding numbers (20 workers
   in 11,400 frames, scripted) were all hand-built ants (`queenprobe` never calls `set_new_ant`), so they do not
   transfer to the trained colony.
2. **"Selection, provision, brood placement and presence act on the existing 2,800 weights" (plan §3b, critique §2).**
   Selection can only re-weight existing minds:
   - Box births copy the mind exactly (`mind.rs:1728-1729`, `lab/mod.rs:1028-1030`).
   - The pool is 30 members. Members 2-30 differ from member 1 in 8-107 of 3,285 slots (`std_pool.txt`, delta-encoded).
   - Measured, played bed, seed 1, default graded rule: distinct minds among adults fell 30 → 13 in 48k frames by
     drift alone.
   - A player choosing layers can reach a one-member colony in about 2-3 generations. Whether that colony behaves any
     differently is **unknown and must be measured first**: a common-garden census of deliveries and digs per pool
     member.
   - If the members do not differ, the genetic lever shows nothing, and the demo needs either in-box mutation (A7)
     or a pool mixed from different training lines.
3. **"Provision: in the engine already, S-M" (A8; plan §4b "now").** Dead in three places (§2 table). Also strike
   "body-size spread", "raised rich: jaw +30%" and "dwarf economies": there is no size trait on a two-cell body.
4. **"The walk has named, inherited gains… a queen can select on it" (queen.md §1.3).** Trained ants never read
   them (`creature.rs:7993-7996`).
5. **"Carry the brood: now (to verify)" / "C1: carry exists, brood scent exists".** The queen has no carry-brood
   verb. The carry rule is a teleport, off by default, and measured as useless. Larva scent is read only by the
   hand-built walk. Brood is a powder that falls.
6. **"Bite: now (attack verb, not yet live)".** The attack verb is live (`options.rs:397-404`, on by default), but it
   cannot target kin (`creature.rs:10979ff`). Biting a worker is new code.
7. **"Feed in the act: now (energy)".** True mechanically, but F feeds whichever hungry nestmate it finds, not the
   ant the player is rewarding. A fed trained ant tends to stop working, so the "stroke" may train idling (as
   selection on favour) rather than reward work.
8. **"The ancestor shelf exists" (A9, "S").** It drops the mind (§2 table).
9. **The queen rule starves a trained colony unless the player lays.** Measured, played bed, seed 1, 48k frames:

   | Rule | Births | Adults | Eggs laid |
   |---|---|---|---|
   | Graded (default) | 349 | peak 142 at 30k, then 49 | — |
   | `PIXEL_PHYSICS_BREEDING=queen` | 31 | 41 → 27 | 31, then **zero after frame ~18k**, while one living breeder held the colony |

   With a player queen who never ages, she *is* that breeder. If she stops laying, the colony dies, which is
   binary-adjacent unless it is surfaced. A demo needs either automatic laying when she is idle, or the rule
   relaxed when she has not laid for N frames.
10. **"Make the generation time short: a few minutes at 1x" (§6) is already true, so the risk is mis-priced**
    (measured, graded rule):

    | Quantity | Measured |
    |---|---|
    | Bulk of generation 1 | by 12k frames |
    | Generation 2 appears | 18k |
    | Generation 3 appears | 24k |
    | Generation 4 appears | 36k |
    | Mean generation at 48k | about 2.6 |
    | Egg to adult (Little's law: about 30 standing brood ÷ about 0.008 births a frame) | about 3,800 frames |

    The lab runs 60 ticks a second at 1x (`time.rs:285`). Headless, one process sustained 550-900 ticks a second
    with 40-140 ants, with two processes sharing 4 cores. So a 10-20 minute session at 1x is 36k-72k frames, about
    3-5 generations; at the 4x steering cap (`demo/mod.rs` `STEER_CAP`, `time.rs` `AUTO_DISPLAY`) it is about 15-25.
    The real binding limit is colony lifespan, not generation time: the graded colony boom-busted 142 → 49 inside
    48k frames.

## 4. The smallest queen demo that is genuinely playable

Its core: **the queen as the colony's only womb, choosing each egg's mind and rate, and feeding the brood. All of it
reads on screen, and the loop is minutes long.**

1. **Body** (in progress): steer the queen, eat (E), lay (L), drop food by larvae (Q), dig (G). Start reserve 900 J,
   no beetles for the first cohort (slice 0 §5).
2. **Lay rate as the work signal (C3), which already works.** Under the queen rule every egg is hers. The visible
   consequences:
   - the brood pile grows;
   - nurse traffic at the pile (nurse-by-touch is larva-side, so it works on trained ants);
   - a births counter.

   Graded: lay more or fewer. Add an "auto-lay while idle" default so a player who forgets does not starve the
   colony (must-fix 9).
3. **"Whose line is this egg" (A2 minus crossover).** Click a worker on its page or in the deeds ledger and press
   `Y`: her next N eggs carry that worker's mind. This is one assignment at lay time.
   - Visible: tint adults by mind identity (an overlay keyed on the mind `Arc`, beside the existing
     `OrganismOverlay::Lineage`, `render.rs:1198-1232`) and show the mix as a bar. The colony visibly becomes
     uniform within about two cohorts.
   - Graded: N eggs, not "all".
4. **Deeds ledger** (A5): sort living workers by digs, deliveries and trip deliveries from `LifeCounters`, which is
   the attention fix for 150 ants. Show a line's share against its deliveries per ant, paired as "this cohort
   against last": the evidence that selection did something.
5. **Wild clutch** (A7): `P` lays one egg whose mind gets `neutral_mutate` at a stated width. It is the only source
   of new behaviour in a session, gambled egg by egg. Mark mutants with a fleck. This needs the owner to accept
   in-box mutation for the demo.
6. **Feed the young** (works now): carry crop food to a hungry larva, and it pupates sooner. Show a larva-hunger
   tint, because the existing feature is invisible.

**Cut from the demo:**

- provision and castes;
- tandem runs (research);
- the bite (keep it only as a knockback with no behavioural claim, or cut it);
- presence damping, brood carry and gravity, brood temperature;
- the dream replay (keep the ledger instead);
- patrilines with crossover;
- U1 scent language, U4 rival colony, U9 mutiny, the shelf as father store.

Each of these is either dead on trained ants or multi-day with an unknown payoff. **Run first, before any code
beyond the body:** the common-garden census of pool members (must-fix 2). If members do not differ by at least the
seed-to-seed spread on deliveries, then item 3 teaches nothing, and item 5 (or a mixed pool) becomes the core rather
than the garnish.

## 5. Risks

- **Selection with nothing to select.** 30 near-clones and no in-box mutation (above). This is the central risk to
  "choosing who lays produces visible change": it can produce visible *uniformity* with no visible *behaviour*
  change. That is the Creatures/Black & White "invisible teaching" failure, now in the genetic channel.
- **The colony dies under the player.** The trained colony boom-busts: peak 142 → 49 by 48k frames on graded, with
  160-167 starved in 36k frames (`labsurvive`, seeds 1-2). Under the queen rule it shrinks steadily when she does
  not lay. The player then cannot tell her selection from the bust (queen.md §7 "unattributable outcomes"). Show
  starvation in-world as graded (graves are already recorded) and pair cohorts.
- **Known trained-pool failures read as the player's fault.**
  - Far food: `std_pool.txt`'s header shows fruit at 200 columns leaving 9 and 37 alive.
  - Fed ants idle, so feeding as a reward sedates.
  - Dig company, `RECRUIT` and forage drive do not exist for mind ants, so no tuning there reaches them.
- **Frame cost.**
  - Measured headless: 550-900 ticks a second at 40-140 trained ants on 4 cores, against 240 needed for 4x. That
    leaves headroom on this box. The owner's machine is the ruler, and render cost is excluded.
  - New overlays (mind tint, larva hunger) are draw-only.
  - A presence field or a scent language would add a per-cell plane every frame. Price it in `examples/ascii.rs`
    before building it.
- **Binary death.** Four beetles kill a lone queen every time (slice 0 §4). She does not age, but starvation and a
  beetle still end the run. Keep beetles out until the colony exists, or make wounds rather than kills the queen's
  rule.
- **Concurrent edits.** The parent is changing `creature.rs` near 7998 and `steer.rs`. Any measurement of the demo
  must be re-taken on that build. The figures above are for clean `main` 9e32979e.
- **Dead-ends check.** Nothing in `dead-ends.md` matches queen, tandem, patriline or crossover. `BROOD_CARRY`,
  `EGG_DOOR`, `FEED_FIRST` and `RECRUIT` (all in the design's neighbourhood) are recorded there as off for measured
  reasons.
