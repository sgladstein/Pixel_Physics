# Scout plan v1: independent review

*2026-10-25. Reviewer did not write the plan or the design. I read `plan-v1.md`, `review-step1-feasibility.md`,
`BOARD.md`, every file in `shared/` and every status file, skimmed `designs/seat-scout.md`, and checked the plan
against `main` at `9e32979e`. No repo file was changed.*

*"Probe" = a scratch copy of the repo with a ~30-line forced-pick slot after `mind::tick`'s `PP_TRAIL_ACT` block
(the shape the plan assumes for `World::mind_force`); crate in `scratchpad/followprobe/`. Each trial: `steady_income`,
colony at frame 200, trained minds, a leader forced east every decision (the player holding Right), the nearest
surface nestmate following; 12 pairs over seeds 1-4, flat and over a 5x20 stone step, 1,800 frames each.*

## Verdict

Buildable, with seven blocking fixes before step 1. The forced pick works: a trained follower forced to "walk the
option nearest the leader" stays in contact on open ground and over a step. But the friction as tuned is a
one-second metronome with a pass/fail outcome, and the pupils the accept rule selects starve on a 200-column trip.

## Ranked findings

### 1. BLOCKING: the follow rule leaves the 3-4 cell band unforced, so her own mind walks her away

§3 step 2 forces a step toward the leader only when `gap <= contact` (2). Between 3 and the break gap (4) nothing
is forced, so the follower's own mind decides. The trained pool has no reason to follow, so she wanders off, and
the gap only grows.

*Probe, seed 2 pair 2, no pauses at all.* The rule as written broke the pair in 130 frames, with the gap
histogram stuck at 3+. The same pair with the force kept on up to the break gap held contact on 96% of frames over
250 cells. The other pairs started at gap 1-2 and never left the band, which is why this hides.

*Fix.* While the pair is `Following`, force toward the leader at any gap up to the break gap. Also make her **Rest at
gap <= 1**. In the probe the follower's head sat inside the leader's body cells (gap 0, stacking) on about 30% of
frames (e.g. 535 of 1,800), and at the lab's zoom two overlapping ants read as one ant. A pupil one cell behind her
leader is the picture of a tandem; one on top of her is not.

### 2. BLOCKING: the pauses make leading a 1-second stutter with a binary outcome

At m = 12 steps and the measured 5 frames per step, she pauses every 60 frames (1 second at 1x) for 40 frames. A
200-column lead is about 16 stops.

On expiry she resumes only if the gap is <= 3, so walking on through any pause loses her. The probe shows the
outcome is all or nothing:

| Leader behaviour | Pairs | Result |
|---|---|---|
| Walks on through pauses | 12/12 | Contact broken within 125-185 frames (at the first pause) |
| Stops during each pause | 12/12 | 100% contact, about 210 cells in 1,800 frames |
| No pauses at all | 12/12 | 96-100% contact, about 250 cells (to the wall) |

No partial failures, no near misses, no choice: the whole skill is "stop when the pip goes amber", sixteen times
a run. That is a nuisance, not a rhythm, and it fails the graded-outcome rule.

*Fix.* Make the stops situational and fewer, and give the player something to read and decide:

- Pause at landmarks (a change of slope, a tunnel mouth, the patch's edge, a junction), plus a random
  geometric clock with a mean of about 40-60 steps.
- Pause 60-90 frames, scaled by pace.
- Give a tell before each stop: she drops a cell behind for 10-15 frames.
- On expiry, resume at gap <= contact + 2.
- Grade the break: a lost pupil who is re-tapped within the search time resumes with "lessons" kept, and one who
  is not keeps a fraction of them. "Learning the way" can then be a count of landmarks she paused at, shown on
  Sniff. That gives "broken runs still help" a mechanism instead of a chance.

Tune m and k by play, and post a blind A/B of two settings early.

### 3. BLOCKING: the economy kills the pupils the accept rule picks, and the formula has no unit

The bank is uncapped (`start_energy` 200 J; the probe saw up to 3,866 J), so `energy_frac` is undefined. Two
readings give different games:

| Reading | 200 J | Typical warmed-up colony |
|---|---|---|
| `energy / start_energy` | p = 0.05 (refuses) | p = 0.05 for nearly every ant |
| The mind's own `c::ENERGY = e/(1+e)` (`mind.rs:73`) | p = 0.5 | p = 0.05-0.14 |

The colony numbers come from the probe at 15k frames (shipped mutation on, seeds 1-3): the bank was p10 22-286 J,
p50 116-1,039 J, p90 300-1,282 J. Survivors of the crash are rich.

The deeper problem is cost. A mind ant pays a brain bill of `synapse_fraction (2.2e-6) x start_energy x live links`
every tick (`mind.rs:2248-2258`). That is about 1 J per 5-frame tick, so:

- **A pupil.** `Reports/instruments.md` (labsurvive row) has it as "with no food a tank lasts the shipped pool
  ~1,000 frames", and the pool "starved to the last ant" with food at 100-240 columns. The probe agrees. One ant
  walked 221 cells for 236 J. One follower starting at 281 J died after 1,445 frames of forced walking (cause not traced). A
  200-column lead at 5 frames per cell is 1,000 frames, about one tank. A sister hungry enough to accept (under
  100 J on either reading) dies before the heap, and one at 200 J arrives empty.
- **The player.** Her body is a mind ant too, so she burns about 12 J a second at 1x: about 36 tanks over a
  10-minute session. The plan never mentions her own hunger clock.

*Fix:*

- Define `energy_frac` as `c::ENERGY`, the mind-visible number.
- Choose the far distance by measurement in step 0. The probe and labsurvive data point to 100-130 columns: past
  where unled ants go, and inside a led ant's round trip.
- Gate acceptance on her surviving the trip, *or* let the player feed her first with F: "share a taste" becomes
  the recruiting verb, and a starving pupil becomes a decision instead of a silent death.
- Put both banks on the HUD, and "pupils starved while led" in the meta.

### 4. BLOCKING: `extra_walls` is a full-height wall, and a step slows nobody anyway

`LabBox::extra_walls` is a list of columns. `paint_wall` fills each one with stone from the ceiling to the bed
bottom (`src/lab/scene.rs:775-780`) and treats it as a compartment partition (`:750-764`). §2 0a's "raised step of
`extra_walls` stone" would seal the far heap into its own box.

*Fix:* use `Placement::Fill { material: "stone", ... }` (`scenario.rs:127`; "stone" exists).

Even as a Fill, the step does not do what the plan says it does.

- **Followers.** In the probe, all 12 no-pause pairs crossed a 5-high step behind a climbing leader at 100%
  contact and reached the far wall. The step slows the follower exactly as much as the leader.
- **The player.** The step is a trap for her. Holding Right at a vertical face stands still: the mind's Walk
  options are the 8 neighbours whose landing the body fits (`options.rs:772-813`), the straight-up option is at
  90 degrees, and the steer rule takes only options within 90 degrees (`steer.rs:239`, `cos > 1e-6`). The player
  must press Up or Up+Right.

Drop the step's claim, or build an obstacle that differs between leader and follower (a gap she must be waited
at, which fits finding 2's landmark pauses). Put "Up climbs" on the help card, or ask the enemy seat to fall back
to the 90-degree option when nothing nearer is legal.

### 5. BLOCKING: the cascade trigger counts the wrong thing, so *c* is mislabelled by up to 3x

§3 fires the cascade when "her crop/spoil drops inside the nest radius", with chance *c* per delivery. That
condition has two problems:

- `spoil` is soil in the jaws, so digging would trigger cascades.
- A mind puts its crop down one cell per `put_down` (`mind.rs:2949-3013`), and the crop holds 5,760 J
  (`ant.ron:455`), which is 6 cells of provisions at 960 J each. So "c = 0.25 per delivery" is up to
  1 - 0.75^6 = 82% per trip, and the "half-c" run is 54%. The card would print 0.25 for an 82% mechanism.

The per-ant signal exists, and minds do deliver: 70-85% of their put-downs land at the nest (probe counts,
1.5k-15k frames). Key the cascade on the first delivery of a trip, which is the `returned = take(trip_load)` flag
at `mind.rs:2995-2998`, or on the `life.trip_deliveries` delta. Require her `last_food` to be inside the patch, so
a trickle trip does not count. Report *c* per trip.

### 6. BLOCKING: an "expanding square spiral" search does not exist in a side-view world

The lab is a vertical slice with gravity. An ant on the surface can only go left, go right, climb, or dig. The
search in §3 step 4, and the faint search square in §4's overlays, would try to walk into the air and the soil, and
would draw a box there.

*Fix:* search as an expanding left-right sweep along whatever she stands on (the surface or the tunnel), about
360 frames, and draw it as a bracket on the ground.

Related, and **untested**: following the straight-line direction to the leader will stall at tunnel corners and at
the nest door. That brings back the "terrain accidents the player cannot read" the step-1 review warned of (H6). A
breadcrumb follow is the robust version: step toward the oldest of the leader's recorded head cells within reach.
Probe the nest exit before choosing.

### 7. BLOCKING: the blind is not blind, and arm B is not given equal terms

**The HUD and help card break the blind.**

- The HUD prints `c=0.25`, and arm C prints `c=0`.
- The help card prints the knob struct.
- B has no tandem counters, and pressing Y in B does nothing.

The owner will name B within seconds (fine, but say so) and C from the HUD. A real blind exists only between A
and C, and their difference shows only after a taught sister's first delivery, minutes in.

*Fix.* Hide *c* and the knobs in sealed mode. Give B the same key: Y in B "calls", so the tapped sister and every
eligible idle sister within 6 cells fall in. The hands are then the same.

**B is not fair in scale or survival.** "Every idle nestmate within 6 cells" at the nest is 20-40 ants with no
cap. Under finding 3 they all survive, because B has no hunger gate and full ants follow. A's pupils are hungry
ones who starve. B's reward is then 6-20x A's on the same HUD. B also has no release, so a column follows the
player into the nest forever.

*Fix.* Give B the same eligibility rule, the same cap (6), the same arrival release, and a timeout.

### 8. IMPROVE (close to blocking): pace mismatch between player and pupil

Every probe pair walked at a 5-frame interval. That is because the harnesses' measuring default is mutation off,
and the founders are identical. With the shipped mutation on, the 15k-frame colony had intervals 4-7 (seed 1: 15 of
43 ants not at 5; `ratio_factor`, `creature.rs:5241-5260`).

The player body is "the youngest adult nearest the nest", so its pace is random. At contact 2:

- A 4-frame player leaving a 6-frame pupil gains 5 cells a second, so she is lost in under a second on open ground.
- A slow player is followed perfectly.

The plan's flat-ground test ("a leader walking at the follower's pace") will pass while the game breaks.

*Fix.* While leading, the player steps no faster than her pupil. Real tandem leaders slow down, and this makes
pace legible ("she is slow", on Sniff), so "Pick a pupil" becomes a real choice. Choose the player body at the
colony's modal interval. Run `scoutdemo` with mutation on.

### 9. IMPROVE: the nest-bound filter puts a caste into the player's hands

Tap candidates are "not nest-bound (`nest_bound_until`)". For a mind ant that stamp gates only nursing and what
counts as home (`mind.rs:1642-1650`); minds walk anywhere. Filtering on it means one sister in four, chosen by id,
can never be tapped. That is the hand-made caste the owner ruled out, made visible as a key that never works on
her. *Fix:* drop the filter.

### 10. IMPROVE: dependencies, and what to build first

None of the three pieces is on `origin` yet: there are no branches for enemy `1ekeut`, queen `epio3i` or keeper
`20o5zg` after `git fetch`. Enemy's status says the slot is "built, tests running".

- **Mind-steer.** The plan's stopgap (strip the player's mind) covers only the *player* body. Every *follower* is a
  mind ant, so tandem cannot run at all without `mind_force`. The probe shows the slot is about 30 lines, and it
  worked on the first try. If enemy's PR is not up within a day, land a minimal slot as its own PR, in the agreed
  `ForcedPick` shape and offered to enemy, rather than keeping a branch-only shim. A shim that never lands means no
  demo.
- **The launcher.** As specified, it does two things and nothing else. The scout also needs:
  - `arm=`/`c=` passed through, or env vars;
  - a draw hook *inside* `Lab::draw`, or `labgif`-style GIFs will not show the overlays;
  - the tandem tick per simulated tick in `tick_world` (`lab/mod.rs:4875`), so the binary and the harness run the
    same sequence and the force is not stale at 4x.

  Ask queen for all three now.
- **Recall.** Correctly optional, but without it "pupils on a 2nd trip" is a dead 0 on the HUD. Hide it until
  recall lands.

*Build order that guarantees a demo:* scenario and census; the minimal slot; `tandem.rs` with findings 1-6 fixed;
the headless `scoutdemo` and a GIF (the first Artifact-page result, needing no launcher, keys or recall); keys and
HUD on the launcher; recall last.

### 11. IMPROVE: step 0c needs a teleport that does not exist

"Teleport 10 hungry trained ants onto the far heap": there is no public move-an-organism call (grep `teleport`,
`relocate`: none). `Placement::Animal` makes a nestless animal, not a colony member. *Fix:* walk them there with the
forced slot (the probe does exactly this), or budget writing a chain move.

### 12. IMPROVE: the off-is-off proof, and what it cannot see

- The test the plan copies (`steer.rs:281`) builds `Lab::new` without `set_new_ant(true)`, which is the
  **hand-built** ant. It would never reach `mind::tick`, where `mind_force` lives. Run it on minds, and keep the
  live-pair positive control.
- "Hashes identical to main" cannot be tested from the branch. Compare `None` against `Some(empty)` in one build,
  as `steer.rs` does.
- The hash cannot see key routing with `demo` unset, chronicle lines, cost, or the new `demo_scout.ron` appearing
  on every player's SCENARIO page (`ui.rs:3816`, `Scenario::list`) and in the scenario-wide tests
  (`scenario.rs:647`, `:688`).
- `examples/ascii.rs` is the outdoor game, so its worst frame cannot move. Quote the lab's own tick cost with demo
  on and off on the same state instead, plus the overlay's draw cost, which repaints every frame.

Missing tests: a leader faster than her pupil (8); `MindCounts::failed` stays 0 under forced picks; the cascade
fires per trip; B's cap and release; same seed and scripted input give the same hash (pairing depends on it).

### 13. IMPROVE: smaller points

- **Step 0a will likely take the most iteration.** At 15k frames the probe had 6-43 ants alive over 3 seeds.
  `Lab::grant_ants` (`lab/mod.rs:1071`), which is labsurvive's `grant=3`, is a ready lever and is demo-only code.
- **The digs-per-arm metric has no power.** The whole colony dug 43-60 cells in 13.5k frames, so dig counts are
  near zero in every arm. Count digs and foraging trips per ant-minute instead, or drop the claim.
- **Forced picks should update the mind's memory** via `scratch_log::remember`, as the `PP_OLDLAB_ACT` override
  does (`mind.rs:2377-2386`).
- **Tap draws** must use `rng::stream(seed, id, frame, slot)` for determinism.
- **The time dial can be driven.** `Lab::time` is public, and `TimeControl::set_preset` exists (`time.rs:832`).
  Trigger the hand-over on the first tick at or past 15,000, not on equality.

## The seven questions, mapped

1. Mechanisms: the forced Walk works (picks apply at her own decisions, every 5 frames; the Walk set is the 8
   neighbours the body fits, FALL steps included, plus Rest). A forced Rest stood the leader still (falls still come
   first, `mind.rs:2302`). Fill, a trickle and a refilled heap are expressible; `extra_walls` is not (4). The dial
   can be driven. A per-trip delivery signal exists (5).
2. Knobs: contact, break and arrival radius are fine; pauses (2), search (6) and accept (3) are not.
3. A/B/C: A and B differ in three ways at once (one at a time against a crowd, friction, consent), so "B wins"
   cannot say which one lost; B is unequal and unblindable (7). Arm 1 is also the owner's tutorial: give him a
   practice run, rotate the order, and write down the question he answers ("which would you play again tomorrow?").
4. Owner rules: the nest-bound caste (9) and graded outcomes (2) are breached; the proofs need fixing (12).
5. Dependencies: (10). 6. Underestimates: the scripted player (pathing, climbing, eating to stay alive), 0c (11),
   0a, the launcher additions, the side-view search. Budget 5-6 days, not 4. 7. Below.

## The single most likely way this ends dull

**It ends up an escort chore with an invisible payoff.** The player holds Right, stops sixteen times on cue, and
watches a pupil arrive near empty or die. The only reward is a counter that grows slowly, because without recall
nothing commutes, and *c* fires per cell. Every probe number points this way:

- with no pauses, there is nothing to do;
- with pauses, there is one reflex;
- the trip costs a whole tank;
- the "graded" outcomes collapse to pass/fail.

*Change the plan in three ways:*

- **Make the payoff visible within seconds of arrival.** She eats, turns, and carries a crop home along the route
  you showed her, drawn as a fading line. Show the colony's food income from the patch as a bar, so one success
  moves something on screen.
- **Make the friction situational and decided.** Landmark pauses and the pupil's own pace replace the clock.
  The hunger budget makes "feed her first or pick a stronger sister" the choice, which puts F to work as the
  recruiting verb.
- **Get a rough playable A against B in front of the owner after step 1**, about day 2, before building the HUD,
  overlays and sealed arms. The owner's hands are the only instrument that can answer this question, and the
  current plan reads them once, on the last day.
