# Guest seat demo: implementation plan v1 ("the beggar")

*2026-10-25, guest seat thread. Follows `review-feasibility.md` (verdict: feasible with changes, the beggar only).
Branch `claude/project-thread-rdj55n`. To be reviewed by a fresh agent, then revised, then built.*

## What the owner gets

`cargo run --release --bin lab -- demo=guest` opens a lab box with a grown, trained colony and drops **you, a
rove beetle the size of a worker**, on the soil ten cells from the door. You smell wrong. Every ant that touches you
decides what you are, and shows it on its body: walks on, stops and antennates you (a `?` over it), flares (a `!`
and a puff of alarm), or nips you (red flash; four nips kill). Rub workers and lie in the nest to take the colony's
smell, beg nurses for food in the brood pile, and keep your disguise topped up, because it fades. Eat a larva when
you are desperate: it fills you at once and sets off the alarm around you. A ledger counts what the colony gave and
what it cost, and grades your life: cuckoo, thief or predator.

**Where it sits.** The guest is the seat where the colony learns *how to treat you*, read off ant bodies, with no
trust meter. It is also the first piece of "ants answer by smell", which the keeper seat wants later for the gnome
(keeper as symbiont). This plan is the demo; whether any of it ships in the plain lab is the owner's call.

## Ground rules

- **Off is off.** Nothing changes unless `World::visitors` is `Some`, which only `demo=guest` sets. Guarded by a
  hash test (same shape as `steer.rs`'s `a_steer_on_no_living_body_changes_nothing`) and by `deeptrace` byte-identity
  against `origin/main` before the PR.
- **One physics.** The guest walks, falls, digs and starves by the ant's own body rules; only the colony's
  *reaction* to it is new.
- **Brain-agnostic.** The rule runs before `mind::tick` (`creature.rs:8022`), so it serves trained ants (the lab
  default) and the hand-built ant (`oldant`) alike. The demo opens on trained ants.
- **No species-gate change.** `is_living_kin_id` and `kin_crosses_kinds` are untouched.

## Shared pieces and who builds them

| Piece | Owner | What I use |
|---|---|---|
| Launcher (`demo=<seat>`, `Lab::demo`, `assets/lab_scenarios/demo_<seat>.ron`) | queen | `DemoSeat::Guest`, `demo_guest.ron`, `src/lab/demo/guest.rs` |
| Body key layer (arrows steer; seat keys Y U P J K R; H help; Tab follow) | queen (`steered-ant.md`) | the layer, for my rove body; `steer::mark` |
| Scent ladder (`World::visitors`, rungs, gates) | **guest (me)**, `shared/scent-ladder.md` | built here |
| Steering a hand-built-brain body | exists (`sim::steer` hooks) | add `SteerInput::jump` |

If queen's launcher has not landed when step 5 starts, I build a minimal `demo=guest` read in `src/lab/demo/guest.rs`
behind the same names and rebase onto theirs; no seat waits on another.

## The steps, each with its proof

### Step 1: the rove beetle (body only)

- `assets/species/rove.ron` + `assets/materials/rove.ron`, appended to both `EMBEDDED` lists. Body `Chain(2)` like
  the ant, the ant's walk constants, `tick_interval: 6`, dark glossy beetle colours, scent authored far from the
  ant's founding point (e.g. `(0.8, 0.8, 0.8)` as the beetle has), tolerance irrelevant. Minimal instincts (Bias→Move
  only). Gut that cannot use the lab's food (so E does nothing; the guest eats only what ants give it and larvae).
  `start_energy` sized so it starves in ~3 minutes of 1x play without feeding (the pressure of the loop).
- Not in the lab's animal chip (as the centipede is held off), not given a mind (`give_new_ant_minds` touches
  species `ant` only).
- **Proof:** run `tests/*.rs` that iterate species and materials (`cargo test --release --test <each>`); probe
  `guestprobe body=rove`: steered with the arrow held, the rove walks through the nest's galleries (head moves >= 40
  cells in 600 frames inside the nest, printed path), and with the ladder off it is killed by trained ants exactly
  as the beetle was (positive control that the ladder will have something to move).

### Step 2: the ladder core (`src/sim/guest.rs`, the shared piece)

- `World::visitors: Option<Visitors>`; `Visitors { list: Vec<Visitor>, marks: FxHashMap<OrganismId, Mark>,
  counts: VisitCounts, events: Vec<VisitEvent>, params: LadderParams }`. A `Visitor` is an organism id (the API
  leaves room for a point visitor for the keeper later; not built now).
- **Contact check**, one call before the mind return in `creature_tick`, only when `visitors` is `Some` and the
  ticking animal is a colony ant (species `ant`): is any visitor cell in the ant's head 8-neighbourhood? (Visitor
  cells are a handful; compare against each visitor's chain. No world scan.)
- **The roll** on contact, at most once per ant per `LADDER_COOLDOWN` (default 30 frames) per visitor:
  - `g = |scent(visitor) - scent(ant)| / tolerance_radius(ant)` (the host judges; `scent_of`, `tolerance_radius`).
  - effective `g' = g + 0.5 * alarm_here/ALARM_MAX + pick(ant) + 0.3 * hunger(ant) + beg_penalty`, where
    `pick(ant)` is a stable per-id offset in [-0.2, +0.2] (from `rng::stream(seed, id, 0, SLOT)`), so some ants are
    reliably pickier: dice, not a gene, and said so on the help card.
  - rung probabilities from `g'` through four logistic steps with floors (no rung certain): ignore, antennate,
    display, nip. Starting centres 0.5 / 0.9 / 1.3, slope 6, floor 0.03; tuned in step 6 by playing.
- **Rung effects**, on the ant's tick, then the ant's decision is spent (early return with its usual reschedule
  and its idle bill charged as a Rest would be):
  - *antennate*: the ant turns to face the visitor and holds for 3 decisions (`hold_until`), mark `?`.
  - *display*: as antennate, plus an alarm deposit at the visitor (the ant's own `cry_alarm` path), mark `!`.
  - *nip*: `gnawed += NIP` on the visitor (0.25, so four nips take a cell and the head is death, the existing
    wound/heal path), alarm cried, mark red. Counted.
  - *ignore*: nothing, the mind decides as usual (no early return).
- **Gates**: in `options.rs` the Attack option is not offered when the target is a registered visitor; in the
  hand-built ant's fight (`nearest_foe`/its attack in `act`) a registered visitor is not a foe. One `Option` read
  each when off.
- Read-outs: `marks` (rung, frame) per ant for drawing; `counts` (contacts, rolls, per rung, feeds, J fed, larvae
  eaten, nips taken, deaths); `events` capped ring for the ledger.
- **Proofs:**
  1. Off-is-off hash test (visitors `None` vs a `Some` with an empty list vs a list naming a dead id: identical;
     positive control: a live visitor differs).
  2. Unit tests: rung odds monotone in `g`; no rung reaches 1 or 0; the host's tolerance (not the visitor's) decides
     (swap tolerances, answer changes only with the host's); the mind is never offered Attack on a visitor and is
     offered it on a non-visitor stranger (positive control); each test watched red with its line removed.
  3. Probe `guestprobe ladder=1`, standing rove at the door, 6 seeds x far/near scent: **near scent lives longer
     and takes fewer nips than far**, with antennations and displays both non-zero (the middle exists). Print the
     per-rung counts; the far arm must not be immortal and the near arm must not be untouched (graded, not binary).

### Step 3: the guest's verbs (in `sim/guest.rs`, guest-only calls)

- **Rub** (key R, held, beside an ant): every guest decision with R held and an ant adjacent, both scents step
  `RUB_BETA` (0.04) toward each other (the symmetric `blend_with_kin` arithmetic, its own copy so the blend's
  callers are untouched). The rubbed ant gets a rung roll at half the usual odds (being groomed is calming), so
  rubbing a hostile ant can still get you nipped.
- **Soak** (key P, held, standing on the nest patch or beside brood): the guest's scent steps `SOAK_BETA` (0.02) a
  decision toward the nearest seeded nest site's odour; the nest takes back `gamma` (0.002). Host-judged gate: if
  the nest's colony would refuse you (`g` vs the mean tolerance > 2.0), soak does nothing and says so.
- **Fade**: every guest decision its scent relaxes `FADE` (0.004) toward its authored odour. So a disguise is upkeep:
  about a minute of 1x play from "family" back to "stranger" if you stop.
- **Beg** (key Y, beside an adult ant): roll acceptance `p = clamp(1.2 - g', 0, 1)`; on accept the ant gives
  `SHARE_FRACTION` of what it can spare (crop first, as `crop_feed` does; else bank above `start_energy`), booked
  `SharedOut` on its colony; the guest's energy rises; a one-cell droplet flash between them; mark `+`. On refuse,
  the ant rolls the ladder with `beg_penalty` 0.3 (begging from a doubtful ant is conspicuous). One beg per ant per
  cooldown.
- **Eat a larva** (key B, beside brood): the guest takes one adjacent brood item (kills it to a corpse cell through
  the normal death path, gains its energy), and a large alarm deposit around it, which raises every nearby roll.
  Counted as `larvae_eaten`. Adults are never targets (the guest is not a predator of workers).
- **Jump** (key J): `SteerInput::jump` sets `Impulse` in `steered_outputs` (the ant's shipped ballistic jump), so
  you can spring clear of a nip.
- Hide needs no code (body physics).
- **Proofs:** unit tests for each verb's arithmetic (rub moves both, soak refuses past the gate, fade returns,
  beg books the ledger and closes the energy account, larva eaten leaves a corpse and alarm), each watched red. A
  scripted-player probe (`guestprobe script=1`, a fixed key script: rub three workers, soak 600 frames, beg in the
  pile) on 6 seeds: **the scripted guest's mean `g` falls, its nips per contact fall, and it is fed at least once on
  most seeds**, against the standing guest. A second arm, script with larva-eating, shows the alarm and nip cost.

### Step 4: the demo scenario

- `assets/lab_scenarios/demo_guest.ron`: the played bed's box and colony, trained ants, **pre-grown**: the
  scenario fast-forwards to a grown nest before the player sees it (reuse `PIXEL_PHYSICS_LAB_TICKS`-style warmup,
  ~30,000 frames, behind a "growing the colony" line), then places the rove 10 cells from the door and marks it.
- If warmup takes too long on a laptop (measure: target < 15 s), fall back to a smaller box or fewer frames.
- **Proof:** launch headless with the screenshot hook; the frame shows a grown nest with brood and the rove on the
  soil.

### Step 5: the demo seat (`src/lab/demo/guest.rs`)

- Keys through queen's body key layer: arrows move; R rub, P soak, Y beg, B eat a larva, J jump; H help card; Tab
  follow-cam (the lab's existing pin/follow).
- **On the rove**: a ring whose colour runs red → amber → green with the guest's `g` against the colony (mean over
  the last few contacts, or against the nest odour when nobody is near). A full replace on a fixed ramp (CLAUDE.md's
  debug-readout rule), drawn in `Ui::draw_world_marks` via the seat's draw hook.
- **On each ant**: its last rung for ~1 s: `?` antennate (yellow), `!` display (orange), red flash on a nip, `+` on a
  feed. 3-pixel glyphs above the head, full replace.
- **Ledger panel** (seat HUD, lab plain style): energy bar, lives, fed (times, J), nips taken, antennations,
  displays, larvae eaten, the colony's brood and adults now vs when you arrived, and a running grade: **cuckoo**
  (fed, brood held), **thief** (fed, brood thinned), **predator** (larvae eaten). On death a life summary line, then
  the rove respawns at the drop point after 3 s (no lineage in the demo).
- Speed: the guest is played at 1x-2x; above the steering cap it stands (input kept `Some`).
- **Proof:** the screenshot and click hooks; a filmstrip/labgif GIF of a scripted session (rub → soak → beg → nip
  → jump → larva → alarm) with the counts in the caption.

### Step 6: tune by playing, then measure

- Tune the ladder centres and the rates so that: a fresh guest at the door is nipped within ~10-20 s of contact;
  three rubs plus a minute of soaking make most contacts antennations; a nurse in the pile accepts a beg more often
  than not once disguised; stopping upkeep returns you to "stranger" in about a minute. Graded every step, never a
  switch.
- **Measure**: scripted probe over 12 seeds, before/after tuning; the per-rung distribution for a disguised versus a
  raw guest (both must have a middle). Frame cost: `examples/ascii.rs` worst frame and `labperf` on the demo box with
  and without the guest (the ladder runs only on contact; expect < 1%).
- **Proof:** the numbers in the report; a GIF of a real played session on the Artifact page.

### Step 7: land it

- Report `Reports/guest-seat-demo-2026-10-25.md` (what was built, the probe numbers, what the owner should try,
  what is hand-made), its line in `Reports/README.md`, the review and plan beside it.
- Artifact page: GIFs (raw guest nipped; disguised guest fed; larva and alarm), the ledger, the counts.
- PR to the merge desk; `deeptrace` off-is-off identity attached; tell the keeper thread the ladder's interface is
  on main.
- Project memory: the guest seat's state and the step-0 facts.

## Risks

- **Contacts may be rare or overwhelming.** 160 ants in a small nest touch a standing guest 20-600 times per 1,000
  frames (probe `touch`). The cooldown per ant bounds rolls; the floors keep it graded. Tune in step 6.
- **The ant's held decisions cost the colony work.** Antennating ants stop working. With one guest this is a few ants
  at a time; measured as part of step 6 (colony deliveries with and without the guest).
- **The guest may not fit where it wants to go** (crowded galleries). It can dig (G) like an ant.
- **The mind may route around the rule**, e.g. a mind Feed offered to a visitor of another colony is impossible
  today (Feed needs the same colony), so all feeding is the beg rule's: by design.
- **Respawn and warmup times** decide whether the demo feels snappy; measured in steps 4 and 5.

## What this plan does not do

Steal, go limp and be carried, lay an egg, play a lineage, the herd, the arms race, the memory layer, the keeper as
symbiont (post-demo, the ladder's API leaves room), any change to how ants judge each other.
