# Keeper seat: adversarial review and feasibility (2026-10-25)

*Step 1 of the keeper thread's five (BOARD.md). The reviewer did not write
`designs/keeper.md` or the teaching plan. Read in full: `keeper.md`,
`teaching-plan.md`, `BOARD.md`. Skimmed: `critique.md`, `seats-overview.md`,
`seat-guest-critique.md`, `shared/*.md`. Checked against `main` at 9e32979e
(PR #897). No simulation was run: a cargo build was already using this machine.
The weight claims below come from reading `assets/minds/std_pool.txt` directly,
plus a Python copy of `mind::decide_planned`'s scorer used as a **sign check
at random contexts**. That check gives the direction of a response, not its
size, and it is not a substitute for a run.*

---

## Verdict

**Feasible with changes.** The keeper cannot be a *teacher* on today's ants.
The verbs the design marks "now, cost S" mostly rest on mechanisms that do not
exist on the trained ant the lab actually runs:
- lamps give no heat;
- the trained ant never moves brood;
- the colony cannot emigrate;
- digging cost does not depend on wetness;
- the shipped pool's last-food weights are zeroed.

But two of the lab's existing hand tools probably do work on today's ants (the
hand **Dig** tool and the **Alarm** puff), and so does the larder. Also, a
learned response can be added for the demo cheaply and honestly, through a
precedent the plan missed: an additive side head of the kind `DigHead` uses,
not a hand-seeded scorer weight. That is enough for a playable ten-minute
keeper slice of the kind plan §8 step 1 asks for. It is not enough for the
design's headline, "trust is a smell". That idea needs a new scent plane, a
training bed that contains a keeper, and the guest seat's scent ladder, so it
belongs after the demo.

---

## The four findings that change the plan

1. **The trail null was measured on a different ant from the one the lab now
   runs.** Slice 0 (2026-10-11) measured the hand-built brain (`ant.ron`)
   through `queenprobe`. The lab has opened on the trained mind since
   2026-10-24 (`src/bin/lab.rs:297-307`). Its trail response is unmeasured.
   For the old brain the cause was never "unknown": `open-bugs-handoff.md` §Z7
   (and `src/lab/ui.rs:842-855`) says the B-reading units sit saturated. Plan
   §2's row "cause unknown, must be traced" is stale on both counts. In the
   shipped pool, the mind's weights lean the right way on a B gradient: a step
   toward stronger B two cells on (`b_ahead`) scores higher in 372 of 400
   random contexts. Every member also *lays* B (9 live emit links each). So the
   trail is untested on today's ant, not refuted.
2. **"Body-level pull with a heritable gain, as the many `*_pull` terms in
   `chooser_step`"** (plan §7 item 3, from critique §3) **does not apply to
   mind ants.** A mind ant skips sense, brain, `act` and the walk
   (`creature.rs:7993-8022`), so no `chooser_step` pull ever reaches it. The
   mind's real precedent is **the dig side head**
   (`creature/mind.rs:5833-5856`): a small head whose output is added to each
   option's raw score before the sharpness squash, kept outside the genome and
   off unless set. Together with a heritable rider past `GENOME_LEN` (as the lay
   gene does, `mind.rs:1754`, `MindGenome::lay`), this is the cheapest honest
   day-one route.
3. **"Hand-seed the founder's weight" has no clean slot in the shipped pool.**
   All 16 scorer units have a non-zero output weight in all 30 members. I
   counted 0 free units per member. So seeding `f::FOOD_WAY` means adding it
   into a unit that already mixes about 25 other inputs through one squash, and
   that changes those inputs' effect too. The unit test that "proves" the
   memory works (`mind.rs:4306-4335`) uses a blank walk-only genome, not the
   pool.
4. **The keeper's best "works today" architect verb is the hand Dig tool, not
   the watering can.** A hand-dug tunnel is lined with packed soil
   (`creature.rs` `hand_dig_with`). The pool's strongest feature weight is on
   `f::LINED`: net output-weighted sum +155. A Dig option aimed at a lined wall
   scores higher in 393 of 400 contexts. So the trained ants very likely widen
   and extend a gallery the player sketches. Wetness has no comparable lever
   (see the claim table).

---

## Claim table

T = true, F = false, P = partly. Paths are under `src/` unless they start
`assets/`.

| # | Claim (where it is made) | Verdict | Evidence |
|---|---|---|---|
| 1 | The lab's ants are the trained mind (plan §2, implied throughout) | **T, and it matters** | `bin/lab.rs:297-307` sets `set_new_ant(true)` unless `oldant`. But `Lab::new` defaults to `new_ant: false` (`lab/mod.rs:657`), so **every harness that builds a `Lab` measures the old ant** unless it calls `set_new_ant(true)` |
| 2 | Weights fixed at birth; 12 memory units (plan §2) | T | `creature/mind.rs:61-63` |
| 3 | std_pool (layout 4) has every `FOOD_WAY` weight zeroed on load (plan §2, §7) | **T** | Header `pool 30 genome 3285 layout=4` (`assets/minds/std_pool.txt:5`); `lab/training.rs:85-107` calls `enter_last_food`, which zeroes slot 18 in every unit when the layout is below 5 (`mind.rs:539-552`). Before zeroing, all 30 members carry 6 links there (180 in total), net -19.7: drift that would act at random if kept. Members 1-29 are stored as deltas from member 0 (`training.rs:94-96`) |
| 4 | A hand-laid trail does not pull workers; the cause is unknown (plan §2, keeper §1 "W lever") | **P: stale** | Measured on the old brain (`Reports/play-as-ant-slice0-2026-10-11.md` §2, `queenprobe`). The old brain's cause is known (§Z7, `lab/ui.rs:842-855`). On the mind it is unmeasured. The mind reads B as strength plus two gained slopes (`creature/options.rs:1019-1024`, `SCENT_GAIN` 32 at `:335`). Net pool weights: `b` -49, `b_diff` -1.1, `b_ahead` +68. Sign check: climbing B ahead raises a Walk score in 372 of 400 contexts. `trailfollow`, the only trail instrument, drives `brain::eval_brain`, the old brain only (`examples/trailfollow.rs:1-60`) |
| 5 | The hand trail writes which plane the mind reads | T | `paint_scent` (`lab/mod.rs:2845-2869`) writes the armed channel, A by default and B on a second press, as a ramp from 0 at the press point to `DEPOSIT` 40 over 160 cells (`SCENT_TRAIL_RAMP`, `:4718`). The mind samples those same planes. **No plane mismatch.** The ramp rises *away* from where the drag starts, so dragging food→nest points the gradient the wrong way for an outbound ant |
| 6 | Watering steers digging; "wet soil is easier and preferred" (keeper B5, B8; plan §4c "now (to verify)") | **F as a mechanism; the ants' response is unknown** | A mind's cut costs the same at any wetness: `mind::dig` → `cut_cell` (`mind.rs:2867-2900`, `creature.rs:22494`). The moisture discount on burrowing is the worm's (`creature.rs:2095-2108`). `f::HARD` is the *material's* `penetration_resistance` (`options.rs:1030`), so wet soil does not read softer. `f::WET` is the cell's water level (`options.rs:1032-1040`); its net weight is -16.7, and sign check says wetter Dig targets score *lower* in 366 of 400 contexts. The old-brain moisture taxis was measured negative (`Reports/dead-ends.md:1165-1167`). There is real physics to show instead: water poured by the Water tool (`lab/mod.rs:2714`) soaks soil, and saturated soil slumps |
| 7 | The lab has a watering tool | P | `Tool::Water` paints liquid water at full (`lab/ui.rs:783-784`, `lab/mod.rs:2714`). It is a pour, not a wetness brush. Soil is painted at field capacity (`lab/mod.rs:2671-2681`) |
| 8 | "Drown a gallery and they abandon it" (keeper B5) | **F** | Ants do not drown: water is a floor (`creature.rs:31972-31990`, `WATER_FOOTING`). Nothing abandons a place |
| 9 | A lamp at a chosen depth draws the brood (keeper B8; plan §4c "now") | **F, three times** | (a) `growlamp` emits light (`beam` 2.4) and no heat (`assets/materials/growlamp.ron:34-37`). (b) Lamps sit only in the ceiling: `place_lamp(world, cx)` takes a column (`lab/scene.rs:1011-1022`). (c) No mind ant moves brood: Pick up is food only (`options.rs:86-89`), and the body's brood carry is **off by default** and reads pile and depth, not warmth (`sim/brood.rs:452-468`; dead-end at `dead-ends.md:1244`). Brood development reads no temperature (`brood.rs` has no temperature read). The pool *does* weigh `f::WARMTH` heavily (net +91; toward warmer in 373 of 400 contexts). Whether the training bed ever had a temperature gradient is unchecked; if it had none, that weight is drift, exactly as `FOOD_WAY`'s was |
| 10 | Eviction by light or drought relocates the colony (keeper B6; plan "new (emigration)") | **F, and not small** | No emigration exists. Home is one stored point, `forage_anchor` (`options.rs` `sensed_home`, `:449-459`), and nothing moves it on conditions |
| 11 | Feeding station; larder control (A1, A3, A6, "S") | **T for placing food; P for "traffic thickens"** | `Tool::Food` paints `provisions` that never rot (`lab/ui.rs:803-828`), so a station persists. A returning commute needs the zeroed memory or the colony's own B, which nobody has measured with a hand-placed station on minds |
| 12 | Sow seeds the ants carry (B4, "S, exists") | **P** | The mind's Pick up takes seeds with `PP_MIND_GRAZE`, which is on by default (`mind.rs:5150-5160`). A spared seed rides as the crop's passenger (`mind.rs:2798-2818`) and is planted where that food is put down (`mind.rs:2966-2967`). "Trails become hedgerows" needs put-downs along routes; mind ants put food down mostly at home. Unmeasured |
| 13 | Stone and sand landscaping (B7, "S") | **P** | Soil brush, Dig (lined standing tunnel, `lab/ui.rs:776-782`) and Wall (a full floor-to-ceiling column, `:800-802`). **There is no stone block brush and no sand brush** (`PLACEABLE`, `:958`) |
| 14 | Fling exists; carry an ant to food (C3, "S to M") | P | `Tool::Fling` is a ballistic hop (`lab/ui.rs:884-898`, `Lab::fling_at` at `lab/mod.rs:2963`). There is no pick-up-and-set-down. Carrying writes nothing a mind reads: `last_food` is set only by the ant's own pick-up away from home (`mind.rs` `pick_up`), and its weight is 0 anyway |
| 15 | Alarm drop (pressure, C2) | **T, probably works today** | `Tool::Alarm` drops at the strength a bite writes (`lab/ui.rs:870-883`). Pool: `alarm` -19.6 and `alarm_diff` -16.8 net. A step toward alarm scores lower in 382 of 400 contexts, and a Dig there in 375 of 400. This is a repel lever on today's weights. The design files it under "M (vibration sense)"; it is already an S |
| 16 | Cull, shelf, clone/mutate, lineage names (D4, D5) | T | `Tool::Cull` (`lab/ui.rs:771-773`); the shelf and Release (`:785-799`) |
| 17 | A steerable queen body exists (plan §2 "the queen seat's body is built") | **P: inert on minds** | `sim::steer`'s hooks sit in the old path (`steer.rs:1-60`). A mind ant returns at `creature.rs:8022`, before `steer::take_input` at `:8171`. Already written up as `shared/steer-mind.md` (enemy seat) |
| 18 | There is a gnome body for the keeper | **P** | `sim::player` is the outdoor gnome: 7x14 cells (`player.rs:44-45`), **a ghost that writes no cells** (`player.rs:4-15`), so ants cannot touch, smell or be blocked by him. The lab carries the field but never summons one (`lab/mod.rs:142-146`). His small 2x3 form is a held-world feature (`wiki/the-gnome.md`). A gnome in the box is cheap to draw and means nothing to the ants unless he writes a plane |
| 19 | Extra smell channels are cheap; "fix the palette at 3-4 and quote ascii" (plan §7.6, keeper notes) | P | Today there are 3 planes (`mind.rs:64-69`; `pheromone.rs`). The module asks to "resist a [new] one until a concrete consumer exists" (`pheromone.rs:31-36`). Settled cost is tiny (0.0014 ms for two planes against a 0.5 ms gate, `Reports/creature-evolution-plan.md` E13). The live cost is per awake tile; measure it with `pherocost`. **The bigger cost is the read:** each option samples 2 cells per channel (`options.rs:1019-1024`), and feature building is already a large part of the mind's frame (`mind.rs:355-364`: the scorer's sparse walk alone was 8%) |
| 20 | New smell inputs are "one feature on each option" (keeper B1, cost M) | **F on cost** | `FEATURES` = 45 is baked into every row of the genome (`options.rs:233`; `mind.rs:195-238`). One new feature moves `S_VERB` and everything after it, and also moves the riders (dig head, lay gene) past `GENOME_LEN`. The precedent is the PR 2 relayout (`mind.rs:239-511`, `pad`/`v2_slots`/`newly_live`), plus a layout mark (`mind.rs:513-526`) and a re-carried pool. Medium, not small. Reusing a dead slot avoids it, but slot 18 is taken by `FOOD_WAY`, slot 40 by the far sense, and `c::D_SCENT+2` by the far ramp (`options.rs:491`) |
| 21 | The link price will strip unused senses (plan §2, corrected) | T as corrected | `how-the-ant-works.md` §12 `LINK_PRICE` row; plan §2 |
| 22 | Trained ants bite strangers | T | `PIXEL_PHYSICS_MIND_ATTACK` on (`how-the-ant-works.md:1537`); switch 8 is 1.0 in all 30 members (pool read). Irrelevant to a ghost gnome |

---

## Verb triage

★ = would make a satisfying demo verb: an instant, graded, visible response.

### (a) Works on today's ants, with a visible and graded consequence (to confirm by a short live check)

- **★ Sketch a gallery (the hand Dig tool).** The player carves a lined
  standing tunnel. The trained ants are strongly drawn to cut lined walls
  (claim 4). Expected sight: the colony moves in and widens or extends the
  sketch within minutes. Graded by how much of the sketch is used and how far
  it is extended. This is the honest "watering can as architect" (B8): the
  architect's tool is the shovel, not the can. **Risk:** the same weight may
  make them tear the lining off, so the sketch dissolves rather than grows.
  Look first.
- **★ Puff of alarm (shepherd's pressure, C2).** The existing Alarm click
  repels both walking and digging (claim 15). A player can steer a column
  round a bed, or keep diggers off a patch. Graded by the dose (clicks) and
  the decay. **Caveat:** alarm is also what a bite cries, so heavy use may
  read as cruelty. It leaves no lasting lesson, which is honest on today's
  ants.
- **The larder (A1, A3, A6).** Food that never rots, placed anywhere. The
  colony's intake and growth respond (`lab/ui.rs:810-817` records thirteen
  generations with food against starvation without it). Graded, but slow, and
  "traffic to the station" is unmeasured on minds.
- **Rain drill (F2) and pouring water.** Physics answers: saturated soil
  slumps and galleries can fill in. The ants do not drown and do not answer
  it. It is a visible consequence of the keeper's action, not of the colony.
- **Cull and shelf (D4, D5).** These exist. They are G-lever moves, slow to
  read.
- **Sow seeds (B4).** Ants carry seeds and plant them where food is put
  down. It is real but slow, and its "hedgerow" shape is unmeasured.
- **The double bridge (F4).** It can be built with Dig, Soil and Wall.
  Whether minds converge on the shorter bridge depends on B-following, which
  is unmeasured (claim 4). A good diagnostic scene, not a demo verb yet.

### (b) Works only with new engine code that is small

- **★ Treat bead / reward on arrival (A2).** Click a bead on an ant: give it
  food into its crop, which is real, so a real reward, and set its
  `last_food` to that spot. That is about 30 lines in `Lab`. It is visible
  only together with (b2) below.
- **(b2) A learned-place read for mind ants, as a side head.** Add
  `gain × f::FOOD_WAY` to each Walk option's raw score before the squash, as
  `DigHead` adds to Dig (`mind.rs:5833-5841`). The gain is a rider past
  `GENOME_LEN` (precedent: the lay gene), set by hand for the demo, behind
  `PP_KEEPER_RECALL`, off elsewhere. About 80-150 lines plus a test that goes
  red with the term removed. This is what turns every place-verb (station,
  bead, carry) into one the ant answers.
- **Meals carry the place (plan §4d "Meals carry the smell", place version).**
  When a mind ant Feeds a nestmate (`mind.rs` Feed, `blend_with_kin` path),
  copy a share of the giver's `last_food` freshness to the receiver. About 30
  lines. This makes a lesson spread through the colony as a graded epidemic,
  which is the owner's "more than follow me".
- **★ Pick up and set down an ant (C3).** A hand verb with no ballistic
  launch, placing a body on a free footing (`place_creature` exists). About
  60 lines. It needs (b2) to mean anything, and it is an S on top of it.
- **Heat mat (replaces B8's lamp).** Formicarium keepers do use heat mats.
  `World::add_heat` exists (`sim/world.rs:10776`), and field temperature has
  no decay term (`field.rs:5344-5348`). Adults' Walk weights lean warm. The
  brood stays put (nobody carries it), so this is "workers gather on the
  warm patch", not "the nursery moves". About 40 lines plus a fixture to draw.
  Its response must be measured, because the weight may be drift (claim 9).
- **A stone brush.** The Wall tool's material, painted as a disc. About 20
  lines. Useful for courses (F4, training grounds).

### (c) Needs the memory layer proper (plan §7) or training

- **Trust is a smell (C1/B1) and manning (C5).** These need a gnome plane
  (frame cost), a learned valence per ant, and either a training bed with a
  keeper in it or the identity-free reader. They also overlap the guest's
  scent ladder (`shared/scent-ladder.md`). Not for the demo.
- **Plants as words (B1/B2 "perfumer's garden").** These need odour tags per
  species, which means more planes or a per-cell tag, plus the
  identity-free valence reader, the genome relayout (claim 20), and a
  re-trained pool.
- **Nursery imprinting and vintages (E2), dialects, haunted ground,
  superstition, biddability as a gene (B9).** All of these need valence
  memory, and B9 also needs selection on the gain, which takes generations.
- **Teach a teacher (E1).** Needs follow-the-leader as a mind option and a
  trained weight on it.
- **Pay the guard (B3/B6).** A nectary organ, plus defence by minds against
  beetles, which is untrained.

### (d) Not worth it (now or as designed)

- **Eviction by light or drought, emigration (B6).** No site choice exists,
  and building one (quorum, brood transport by minds) is a subsystem, not a
  verb.
- **Lamp at a depth draws the brood (B8 as written).** Wrong on all three
  counts (claim 9). Replace it with the heat mat.
- **"Drown a gallery and they abandon it."** Ants do not drown (claim 8).
- **The nuptial net (D3), garden commissions, seasons.** Outdoor fiction; the
  lab is a sealed box (critique §1).
- **Splits and requeening (D1, D2).** These are M-L and slow to read. They
  belong with the queen seat's "who lays" hook (`shared/who-lays.md`).

---

## The smallest honest demo

Launch: `cargo run --release --bin lab -- demo=keeper` (the queen seat's
launcher, `shared/launcher.md`). It loads `assets/lab_scenarios/demo_keeper.ron`:
- one founding colony in a 512-wide box, on the trained pool;
- food about 60 columns out, so foraging is visible and nobody starves;
- one lamp column dimmed for contrast.

The shipped lab is unchanged with no `demo=`.

### A. Cheapest: "the keeper's hand" (no fakes; about 400-700 lines)

What the player does in the first ten minutes:
1. **0-2 min.** Sketch a gallery with Dig from the founding shaft toward
   where they want the nest. Watch the colony move in. The HUD shows **cuts
   made in your sketch** and **cells added beyond it**.
2. **2-5 min.** Lay a feeding station with Food at the far side. Turn on the
   **traffic overlay**: a fading heat-ribbon of the steps ants took in the last
   N frames, drawn as a full replace on a fixed ramp (CLAUDE.md debug-readout
   rule). The HUD shows **trips to the station and deliveries home**.
3. **5-8 min.** Herd with alarm puffs: keep the column off a seedling you
   planted. The HUD shows **steps turned away within r of a puff**.
4. **8-10 min.** Pour water over a gallery and watch the soil slump. Or cull
   one ant and open its page.

New code:
- the scenario `.ron`;
- a keeper HUD with the counters (about 150 lines);
- the traffic overlay (about 150 lines, one u16 grid, drawn only when on, so
  the dirty-rect skip is untouched when it is off);
- a stone brush (about 20 lines);
- the `demo=keeper` key routing (about 60 lines).

What it proves: what the keeper can do *today*. What it does not prove:
teaching. **On its own this is the "spectator with a shovel" the critique
warns about.** It is the honest floor, and it needs a live check of verbs 1
and 3 before anything is built on them.

### B. Most convincing: "teach a forager" (one labelled fake; about 900-1,300 lines on top of A)

Add (b), (b2), the meals copy and the ant carry. Ten minutes:
1. **0-3 min.** As A.1 and A.2.
2. **3-6 min.** Click a treat bead on one ant at the far station. A mark
   appears on it. When it goes home and comes back out, it **walks back to the
   station**: a visible commute, with its trip count on its page. An unmarked
   sister wanders.
3. **6-8 min.** Pick up a second ant and set it down at the station. It eats
   there, and from then on it commutes too.
4. **8-10 min.** Watch the lesson spread. Ants fed mouth-to-mouth by a taught
   ant start heading out toward the station without having been there. The
   HUD shows a **"knows the station" share of the colony** over time, with the
   teacher named. It decays as the memory fades (4,000-frame half-life,
   `options.rs:592`) unless renewed. That is graded and decaying, and the
   station is a reason to come back.

**What is faked:** the listening gain is set by hand, not evolved. That is
exactly what plan §8 step 1 sanctions ("a *scripted* learned response
(hand-set biases on ants that were fed or treated)"). It is **acceptable**
on three conditions:
- (i) the HUD says "learning: scripted gain";
- (ii) one key toggles the gain to 0, the honest arm, so the owner sees what
  today's ants do with the same memory;
- (iii) it stays behind the demo flag, so no pool, no training bed and no
  shipped default sees it.

Everything else is real: the memory is the engine's own `last_food`, written
by the ant's own pick-up or by the player's bead, and the reward is food in
the crop.

**Why B over the design's own pick (trust is a smell):** B delivers in one
trip (about 1,000-2,000 frames), on one ant, with a mark on it. It uses no new
plane, no relayout and no training. It also answers "how is this more than
follow me". The player never leads: they plant a memory, and the colony
passes it on by feeding, or forgets it.

### C. If the owner wants the gnome on screen

Draw the hand as a small gnome figure at the cursor, or summon a gnome at
2x3 scale (the held-world small form) who walks the surface while the clicks
act at reach. It is cosmetic, because a ghost body is invisible to ants
(claim 18), and it costs about 100-200 lines. Do it only after B is judged.

---

## Risks that would make it feel inert, and the counter that catches each early

| Risk | How it would look | Counter (build it before the verb) |
|---|---|---|
| The sketch dissolves: lining is torn off and nothing is extended | The player's tunnel turns into rubble | Cells of the sketch still open at +2k, +5k and +10k frames; cells dug that touch the sketch, against a no-sketch paired seed |
| The ants ignore the sketch (it is not "home", and `f::CUT` reads 0 because hand cuts are not in `dug_cells`) | The colony digs its own shaft beside it | Share of all cuts within 2 cells of the sketch, paired against a no-sketch seed; and **look at it** |
| The side-head term never changes a pick (the gain is too small, or the job head never picks Walk where it matters) | Taught ants wander like the rest | **Decisions moved**: for marked ants, recompute the pick with the term at 0 and count the decisions where it differs. Report it beside the image. Sensitivity control: gain 0 must read exactly 0 |
| The term is too strong (Pikmin) | Taught ants pace a straight line to the station even when full or hungry for home | Share of a taught ant's trips that end at the station; check the target band (critique: about 60-80%), and confirm that hunger and home still win |
| A memory alone does not produce a commute, because `last_food` is overwritten at every pick-up, including food found on the way | The lesson "jumps" to wherever the ant last ate | Per taught ant: distance from `last_food` to the station over time |
| Meals do not spread it: Feed between adults is rare | The share stays at the 1-2 ants the player touched | Feeds per 1,000 frames between adults in the demo box (count it first, with `world.minds.counts`) and the copies made |
| The station is found anyway, so the taught and untaught arms look alike | No contrast for the eye | Station visits per ant, taught against untaught, paired within a seed |
| Alarm repels but also spooks the nest (stops digging and foraging) | The colony stalls after a few puffs | Cuts and deliveries per 1,000 frames in the 2k frames after a puff, against a paired seed with no puff |
| The demo measures the wrong ant | Everything reads like slice 0 | Every harness asserts `lab.new_ant` and prints `mind::has` for one ant, because `Lab::new` defaults to off (claim 1) |
| Frame cost | A slower worst frame | Run `examples/ascii.rs` (the worst-frame number CI already runs) with the demo flag on and off. The overlay and HUD draw only when on, and the side head runs only for marked ants |

---

## Must-fix list for the implementation plan

1. **Re-baseline the trail on the mind before saying anything about trails.**
   Port `queenprobe mode=trail`'s four arms to a `Lab` with
   `set_new_ant(true)`. Fix plan §2's row: the null was measured on the old
   brain, and its cause there is §Z7. Lay the trail nest→food, because the
   ramp rises away from the press point (claim 5).
2. **Delete or replace every "now" verb that rests on a false mechanism:**
   - lamp at a depth, lamp draws the brood, eviction or emigration, and
     drowning a gallery, all in the design;
   - the plan §4c rows "Warm the nursery: now" and "Watering can as
     architect: now (to verify)". Warmth: lamps give no heat and brood is not
     moved. Wetness: no dig-cost effect, and the trained weight leans *away*
     from wet.
   Replace them with the hand Dig sketch, the heat mat (as b) and the alarm
   puff.
3. **Choose the side-head route, not hand-seeding or `chooser_step` pulls**
   (findings 2 and 3). State the interface:
   - the term is added to raw before the sharpness squash;
   - it applies to Walk options only;
   - the gain is a rider past `GENOME_LEN`, carried by `train::save_genome`
     and `fit_head`'s rider logic (`lab/training.rs:176`);
   - it is off unless the flag is set.
   Write a test that goes red with the term removed.
4. **Every learned verb ships with its honest arm on a key** (gain 0), and
   the HUD says which arm is live. That is the condition that makes the §8
   fake acceptable.
5. **Counters before verbs:** decisions moved, station visits taught against
   untaught, the "knows the station" share, sketch cells used or lost, steps
   turned by alarm. Put each count beside its image on the Artifact page
   (BOARD step 5).
6. **Look first** (CLAUDE.md Method 1): one GIF of today's ants with a hand
   sketch, and one with three alarm puffs, *before* the plan is written
   around them. If the sketch dissolves, demo A loses its opening verb.
7. **Put the memory layer in `shared/`.** The keeper's demo needs only the
   *place* half (b2 plus the meals copy). Scout and mind want the same piece.
   Claim `shared/recall-head.md` (side head plus meal copy) for the keeper
   seat, or join whoever claims it first. Trust and smell valence stay
   post-demo and should graft onto `shared/scent-ladder.md` (guest), not a
   second scent model.
8. **Price any new plane before proposing one** with `pherocost` and
   `ascii.rs`. Note the option-read cost (claim 19). Prefer reusing alarm or B
   for a demo cue over a fourth plane.
9. **Run the dead-ends grep for each reused mechanism:**
   - `BROOD_CARRY` (`dead-ends.md:1244`);
   - moisture taxis (`:1165`);
   - `RECRUIT` (17 hits);
   - before shipping anything that sends ants out:
     `python3 scripts/deadendindex.py --touching`.
10. **Fit the fiction to the box:** a formicarium keeper with a test-tube
    colony. Drop garden commissions, seasons and nuptial flights from the demo
    scope (critique §1).
11. **Do not touch the shipped lab:** everything goes behind `demo=keeper` or
    `PP_KEEPER_*`. Harnesses build a `Lab` with `set_new_ant(true)` stated.
