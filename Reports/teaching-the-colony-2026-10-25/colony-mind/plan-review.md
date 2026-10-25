# Colony mind plan v1: independent review

*2026-10-25, a fresh reviewer that did not write the plan. Read against branch `claude/project-thread-6xokl9` at
09:00-09:40Z (the tree was moving under me: `scale_pheromone`, a `hush` arm, `demo_mind.ron` and `scripts/mindstake.py`
appeared while I read). No cargo; I ran the already-built `target/release/examples/mindprobe` (~45 runs) and one scratch
scenario through `PIXEL_PHYSICS_LAB_SCENARIOS`. Raw outputs in my scratchpad, not kept.*

## Verdict in plain words

**Build with changes, and change the order before anything else.** The plan's anchor claim, "hold the mouse on the
door and more food comes home (+19% to +43%)", is not supported: the counter it was measured on mostly counts food
being picked up and put back down inside the nest, and during the swell fewer ants are out, not more. Closing the
stomach does what it says to sharing, but my first check of it as a famine lever showed no consistent effect on
survival. So the two things the demo stands on, "swell feeds the colony" and "the stake can be steered", are both
unproven. Both can be measured with the harness that already exists, in about an hour, before any UI is built. If they
pass, the build plan is mostly sound. If they fail, the plan's own stop rule applies, and it should apply now.

---

## Must fix

### 1. Swell's payoff is measured on a counter that counts re-handling, not food coming home

`MindCounts::delivered` goes up for **every crop cell put down beside the nest** (`src/sim/creature/mind.rs:3016`,
`at_nest` from `nest_within_reach`, `src/sim/creature.rs:19286`), including food an ant lifted inside the nest and set
straight back. This exact trap is already pinned in the repo: `creature.rs:46019-46030` ("the deep trace's top forager
by `deliveries` had 291, of which 16 were food it bit at the pile and 241 food picked up inside the nest and put
straight back"), and it is why `CreatureStats::trip_deliveries` exists.

Evidence that step 0's swell number is re-handling, from runs of the same binary:

| Check | Result |
|---|---|
| Twin's `delivered` over 30,000 frames after the press point (seeds 1-4) | 5,965 / 6,990 / 6,279 / 2,859 cells, against an income of 1,200 (plus at most ~1,040 of backlog). 2.4x to 5.8x everything that ever arrived |
| `trailnear` hold=1000 r=15, first 1,000 frames, seeds 2/5/8 | `delivered` 93→193, 178→283, 179→459. Only 40 cells arrive per 1,000 frames, so +100 to +280 cannot be new food |
| Ants out of the nest during that hold (mean, paired) | **−8, −6.5, −12.5** on the same seeds. Swell keeps ants at the door; it does not send them out |
| Paired headcount 10,000 frames later, 8 seeds | −90 to +22, no sign |

What step 0 actually shows is that **pressing B onto laden ants at the door makes the colony handle food more there**.
That might still look like something (a busy door), but nobody has looked at it, and it is not "more food home".

**Fix:** re-run step 0's swell rows with `creature_stats.trip_deliveries` (food bitten on a trip, put down at home)
added to the `S` line, plus a positive control that the twin's trip count never exceeds income plus backlog. Use that
number for the HUD's DELIVERED/MIN. If trip deliveries do not move, swell has no payoff in a fed colony, and the
anchor verb goes. Hush, measured on `delivered`, will "work" for the same wrong reason, so it needs the same counter.

### 2. The voice is refilled by the same counter, so swell would pay for itself

The plan refills the voice "+1 per cell delivered to the nest". If that is `delivered`, a held swell raises its own
refill by +100 to +280 per 1,000 frames while costing about 0.02 × (laden ants in the brush) × 1,000, roughly 200. The
voice cost, which the plan names as the thing that stops "swell always" from being the one recipe, would be close to
free or negative. Refill from trip deliveries (or colony energy gained), and check the meter actually drains under a
held swell in the step 4 run.

### 3. Measure the stake before building any UI (move step 6 to step 1)

The plan builds launcher, verbs, voice, HUD and favour (steps 1-5) before asking whether any of it matters (step 6).
`mindprobe` already drives every arm, and a famine bed loads from a scratch directory, so the stake is a harness
question that needs nothing from the launcher. The author appears to have started this at 09:15Z (`demo_mind.ron`,
`scripts/mindstake.py`, a `stomachfam` arm); good, and it should become the formal gate.

My first cross-check (scratch copy of `steady_income` with income stopped 20,000 of every 35,000 frames, equivalent to
`demo_mind.ron`; `arm=stomach` held over the nest, r=30, for the whole first famine, 26,000-41,000; 8 paired seeds):

| | Twin | Stomach closed in famine |
|---|---|---|
| Adult feeds during the famine | 1,481-9,103 | 0-153 (sharing stopped) |
| Headcount at famine end (41k), per seed | 32, 104, 145, 0, 39, 1, 5, 4 | 70, 87, 78, 7, 43, 0, 8, 5 |
| Headcount at 56k | 71, 40, 104, 0, 0, 48, 0, 0 | 84, 0, 31, 55, 0, 1, 0, 0 |
| Extinct by 56k | 4 of 8 | 4 of 8 |

Up on 5 seeds and down on 3 at famine end; mixed at 56k. That is the chaos, not a lever. One crude policy on 8 seeds
is not a verdict, but it is the opposite of evidence for the demo's question.

Two facts about the stake that matter for legibility too:
- **Deaths do not line up with the famine.** Colonies often grow through the famine on stored food (seed 2: 44→104,
  seed 3: 39→145) and crash 5,000-20,000 frames after income returns (seed 5: 143 at famine end → 0 by 46k; seed 2:
  156 → 9). The steady-income twin crashes too (seed 3 302→60, seed 4 109→32, no famine at all). The bust is the
  colony overshooting, and a player cannot read "the famine killed them" or "my hand saved them" from that.
- **The stop rule needs numbers.** "Beyond the seed spread" is undefined against a headcount that swings 0-300.
  Suggest: a policy passes if, paired on 16 seeds, it changes adult-days lived (mean adult headcount) in the same
  direction on at least 12, and the extinction count by at least 3. Score the quantity, not pass/fail.

Also: `mindprobe`'s `alive` counts brood (`ants()`, `examples/mindprobe.rs:53-64`, filters species only), and a twin
went 1 → 48 ants in 15,000 frames (seed 6). Count adults and brood separately in the stake table.

### 4. Every key and both mouse buttons the plan picks are already bound in the lab

All 26 letters are bound in `src/bin/lab.rs`. **C** places a colony (`:702`), **F** cycles the display floor (`:866`),
**H** cycles creature colour (`:788`; the launcher also wants H for its help card), **right mouse** erases
(`:1001`), **left mouse** runs the current tool (`:988`). `[`/`]` is already the lab's brush size (`:771`,
`Lab::dial_step`), which is fine to reuse. The plan has to say how the demo takes input: the natural route is a
**demo tool in the existing tool palette** (left-drag = swell, shift = hush, a second tool for the stomach disc), so
the lab's own bindings stay intact and the tool chip names what the mouse does. Agree this with the queen thread's
launcher, since six seats will hit the same problem.

### 5. Cut favour from v1

It depends on `who_lays`, which has no plan yet (`status/queen.md`: launcher and who-lays still proposed), and the
plan already concedes "line share will move, behaviour will not". A verb whose only consequence is a fleck of colour
on eggs of near-identical ants fails the repo's second law ("there must be a verb, and it must deliver something").
It is the queen's verb in the arc anyway. Dropping it removes step 5 and the only cross-seat dependency besides the
launcher.

### 6. The first minute opens on a crashing colony, and the cycle is ten minutes long

On all 8 seeds the colony falls from 52 to 4-11 ants within 2,000 frames of landing, and is 9-42 ants at 15,000. A
35,000-frame cycle is about 10 minutes at 1x (60 ticks per second). The plan needs a script for the first 60 seconds:
which seed (pick one, measured to open well), how long the warm-up (long enough to be past the landing crash and on
the way up), what speed the box starts at (likely 4-8x so the first famine arrives in two or three minutes), and the
three lines on screen that teach the keys.

---

## Improvements

- **The hunger tint already exists.** `CreatureColour::Energy` (`src/render.rs:1298-1301`, label at `:1334`) colours
  each ant on a fixed dark-to-bright ramp, which is exactly the plan's spec, and it is drawn in the same per-cell
  path as the other creature colour modes, so it costs nothing extra. Set it on in the demo (or let H cycle to it)
  instead of building an overlay. That removes a day of work and an edit to `render.rs`, a contested file.
- **Look before you measure.** The out-of-nest drop says swell's on-screen picture is ants loitering at the door.
  Render it (the step 2 GIF) as the first thing in step 2, before the harness.
- **Hush was built ahead of its gate.** `scale_pheromone` (`pheromone.rs:669-681`, `world.rs:7827-7832`) is already in
  the tree; the plan says it lands "only if step 2 keeps hush". Fine to keep as a scratch arm, but it should not land
  unless hush passes on trip deliveries. The 9-cell scrub in the new `hush` arm is right: minds lay after acting, at
  the new head (`mind.rs:2577`, `:3224`), and the plane diffuses between ticks, so one cell would miss most of it.
- **"SHARES/MIN in discs" reads ~0 by construction**, because the hook removes the option. What the player wants to
  see is *shares refused*: count suppressed Feed offers in `stomach_closed` (one counter in `MindCounts`), or show the
  hunger spread inside the disc against outside.
- **A NO HAND ghost is the single biggest legibility win, not a v2 extra.** With a 0-300 swing and deaths that lag the
  famine, a player cannot credit their hand without the counterfactual. If the stake passes, measure the cost of a
  second 512x320 world at the demo's speed (`labperf`) before deferring it; the world is small today.
- **The timeline cannot repeat a cycle** (`src/lab/scenario.rs:100-111`: `at`, `every`, `until`). Writing it longhand
  works, but `demo_mind.ron`'s last feast ends at 410,000 with `horizon: 600000`, so after 410k the box starves for
  good. Either extend to the horizon or add a `period` field (a small, testable change to `due`).
- **Step 4's screenshot proof cannot hold or drag**: `PIXEL_PHYSICS_LAB_CLICK` plays single clicks. Script the held
  swell and the disc through an env hook or a harness-driven render.
- **Chaos floor.** Using `echo` as the floor is acceptable only because echo measured null; a floor should come from
  a perturbation that is known to mean nothing and does diverge (say one full `DEPOSIT` of B far from any ant, or a
  reseeded RNG slot). Quote the floor per metric, and once the metric is trip deliveries, re-derive it.
- **Swell's speed dependence.** The 4 s ramp is in game time, so at 8x a one-second click is a full swell. Sample
  the mouse per displayed frame and apply per tick (the precedent is `training.before_step`, `src/lab/mod.rs:1929`),
  and state the ramp in real time if the demo runs fast.
- **Frame cost.** Swell and hush walk every organism each tick (`ants()` allocates a Vec per tick in the harness);
  in the demo, walk the minds' own list and skip when the mouse is up. The tint is free (above). Cost is otherwise
  not a concern here.

## Is it worth building?

As written, the player holds a mouse button over a door and watches a number that does not mean what the label
says, against a stake whose ups and downs come from the colony itself, about ten minutes per cycle. That is not yet
satisfying: the verb has no proven delivery, and the outcome cannot be attributed to the player.

The idea underneath is good. A hand on the colony's own signals, graded rather than an order, is exactly the repo's
ethos, and closing the stomach has a real, graded, visible effect on who is fat and who is lean. **The single change
that would most improve it:** make the consequence attributable. Measure swell on food actually brought home, and if
any policy moves the stake, show it against the NO HAND ghost. If no policy moves the stake, the plan's own fallback
is right (fold the verbs into the queen game as late tools), and a raider as the stake (enemy seat) is the better
next try than a longer famine, because a raid gives a death the player can see happen and a sharp moment to act in.

## Suggested order

1. Add `trip_deliveries` and an adult/brood split to `mindprobe`'s `S` line; re-run swell and hush (16 seeds).
2. Stake gate on `demo_mind`: none / swell when trips are high / stomach in famine / both, 16 seeds, numeric rule.
3. Only if 1 and 2 pass: launcher stub, demo tools (no key collisions), swell/hush, stomach + `CreatureColour::Energy`,
   voice refilled by trip deliveries, HUD, first-60-seconds script, ghost twin if affordable, frame cost, land.
