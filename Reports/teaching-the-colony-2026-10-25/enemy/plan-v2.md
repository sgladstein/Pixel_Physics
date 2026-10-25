# The enemy seat demo: implementation plan, v2 (revised after independent review)

*2026-10-25, enemy seat thread. v1 was reviewed by an agent that did not write it (`plan-review.md`: ready with
changes, 9 must-fix, 10 should-fix). This version takes every must-fix; "What the review changed" at the end lists
each item and what was done. Built on `review-feasibility.md` (feasible as the Raider, not the Pit).*

## What the owner will play

`cargo run --release --bin lab -- demo=enemy` (add `oldant` to hunt the hand-built colony). The window opens at once
and the box visibly fast-forwards while the colony digs in ("THE COLONY IS DIGGING IN: 12,000 / 20,000"), then
drops to 1x with **you, a centipede** (the shipped nest raider: a 5-cell chain, jaw 0.5, plate 2.0, the trained raider
mind under you) on the surface, ringed, with every nest door marked.

- **Arrows** creep. Held back over your tail you turn round; held level into a wall you climb.
- **B** bites a foe touching your head. **G** cuts what a 0.5 jaw cuts and shoves through stored crumbs. **E** takes
  flesh or a corpse into your crop. **Q** puts down what your jaws or crop hold.
- **Tab** camera follow, **H** help, time dial on PageUp/PageDown (queen's launcher). **Y** season card. **R** (after
  death) a new centipede at the surface.

**The loop, as the engine really runs it:** find a door, go in, bite, eat, get out. **Wounds are one meter per body**:
every bite adds to it, and when it fills, the cell that took *that* bite comes off. The head is death; a neck bite
cuts off everything behind it, and the severed piece is meat the ants carry home. So the tactic is real and goes on
the help card: **back out tail-first**, so the bites land on your tail and cost one cell at a time. Your crop holds
three ant cells and digests on its own clock (measured in A0); Q spits a cell to bite again, and the spat flesh lies
where the ants can take it back. Your tank empties as you walk; empty, you starve.

**What it honestly is:** a colony that fights back, not one that learns you. The help card says so. Learning is the
owner's call on the thread card (Season 2, below).

## Part A: the Raider. Steps, each with its proof

### A0. Measurements before building (CPU, in parallel with A1-A2)

1. **The box survives.** `assets/lab_scenarios/demo_enemy.ron` (from `lab_mid_fruit`: steady income, two fruit
   heaps 40 columns from the door, mister off). Trained colony alive at 80,000 frames on at least 5 of 6 seeds with no
   raider (`labsurvive founders=std`), and with one raider that goes in (`spiderlab species=centipede ant=std
   spiders=1 far=0`, budding off by setting the raider's bar out of reach for the harness only). If the box needs
   the training bed's founding grant, `demo=enemy` applies it at load and the report says so. **Proof:** ants alive by
   seed at 20k/40k/80k, both arms, both brains.
2. **There is a fight (decides the opening brain).** Bites landed *on* a raider inside the nest per minute at 1x,
   trained vs hand-built, 6 paired seeds (`encounter_log` rows `ENCOUNTER_BITE` with `b` = the raider, while its head
   is below the surface). Plus which chain index each lost cell had. **Gate:** if the trained colony lands under ~1
   bite a minute on a raider in its nest, the demo opens on the hand-built colony (or asks at launch), and the owner
   is told why.
3. **The digest clock.** Frames for the centipede's crop to digest one ant cell at 1x (one crop, logged per tick).
   It sets the HUD's "gut" line and the help card's numbers.
4. **The alarm trace** (design step 0): alarm laid by a bite beside trained ants, each ant within 10 cells traced for
   300 frames (position, verb, alarm features), with a 20x dose as the positive control. Decides one HUD word only
   ("they come to the bite" or not), so it runs last.

### A1. Steer any body with a mind: PR #903 (built; review fixes folded in)

`steer::take_forced` / `steer::choose` / `steer::force` / `World::mind_force`. Fixes from the review, in #903:
`choose` takes the genome's verb switches (a raider is never made to Feed); a chain held back over its tail turns
round instead of curling over its neck; the memory update `remember_done` moved out of the scratch module; a
test that B actually bites through the mind (with a no-B control at 0 bites). Known and documented: a led body lays
the scent its mind chose (the emits are the mind's); the scratch decision logs record the mind's pick; `MindCounts`
include the player's decisions.

### A2. `demo=enemy` on the shared launcher (engine hooks named as such)

Depends on the queen launcher (`Lab::demo: Option<DemoSeat>`, `assets/lab_scenarios/demo_<seat>.ron`, the key
layer). Mine: `src/lab/demo/enemy.rs`, `assets/lab_scenarios/demo_enemy.ron`, and two small engine hooks:

- **`Steered::may_breed: bool` (default `true`)**, read at the top of `try_bud` beside `births_paused`
  (`creature.rs` ~5637). The demo marks the raider with `false`: the player does not bud (the reviewer's run: 2
  raiders became 72). Default `true` keeps the queen seat's queen laying. `Steered::colony` is left as `mark` sets
  it (v1's `u32::MAX` would have crashed `World::book`).
- **`Steered::last_act: Option<(Option<Verb>, Verb, bool)>`**: the steered mind body's verb asked, verb carried out,
  and whether the act succeeded, written after `act` in `mind::tick` (one write, under `forced.is_some()`). The HUD
  reads it (A3).
- **Warm start, visible:** the scenario lands the colony at frame 0; `enemy.rs` runs the dial at its top while
  `frame < 20,000`, with the progress line, then places the raider (`place_predator`, lab pool member 0, far 30
  columns from the nearest door), marks it (`may_breed = false`, input `Some`), and sets 1x.
- **Brain choice at launch**, not a live switch: `demo=enemy` is the trained colony (the lab's default), `demo=enemy
  oldant` the hand-built one; the HUD names it. (v1's live `U` wiped every mind, the player's included.)
- **Off is off:** with `demo=None` the box is hash-identical over 3,000 frames (test, as
  `a_steer_on_no_living_body_changes_nothing`); with `may_breed` true everywhere, a budding raider buds as today.

### A3. The HUD and the marks

- **HUD (lab text, top-left):** `RAIDER 5 CELLS  WOUND ##--  (THE NEXT BITE THAT FILLS IT TAKES THAT CELL)`,
  `GUT 2/3 -- DIGESTING, NEXT CELL IN 9 s`, `TANK ######----`, `JAWS: CRUMB (Q PUTS IT DOWN)` when holding one,
  `YOU: 3 KILLS, 2 WOUNDED   THEY: 7 BITES ON YOU, 1 CELL TAKEN`, and `COLONY 162 ANTS, 41 LOADS HOME SINCE YOU
  CAME` (raw counts; the price `kills per 1,000 loads` only once loads ≥ 200). Loads read
  `creature_stats.trip_deliveries` (never `deliveries`: it counts crumbs shuffled inside the nest, 2.4-5.8x).
- **Verb feedback from `last_act`, 2 s:** asked B, did nothing: `FULL -- SPIT (Q) OR WAIT` or `NOTHING IN REACH` or
  `PLATE TOO HARD`; asked G, walked: `NOTHING YOUR JAW CAN CUT`; asked E with a handling gut: `YOUR GUT SETS THE PACE`.
- **Marks (demo-only draw):** a bite pulse at the biter's head for every `ENCOUNTER_BITE` row involving the player
  (`encounter_log` on in demo mode, drained every frame, only the player's rows kept); a red ring at the player's
  head on the frame a new `KillRecord` with `detail.attacker == player` appears (records carry no position); a
  marker over every `nest_sites` door. Corpses and severed segments are the world's own.
- **End states:** the player dies: the season card, then R. The colony dies: the season card says so; R restarts the
  box from the same seed. Above the steering cap the raider's own mind hunts: the HUD says `INSTINCT HUNTS (DIAL
  ABOVE 4x)`. Resting to digest at 4x is allowed and removes most waiting.
- **Proofs:** (1) a screenshot of the real lab renderer, cutaway on, showing the centipede in the nest distinguishable
  from crumbs; (2) a GIF of the autopilot hunt (A5) with the HUD; (3) the HUD's kills, bites and loads equal to the
  run log's independent counts.

### A4. The season card (Y, or on death)

Time hunted, kills, wounds left, bites taken, cells lost (which index), loads home while you hunted, ants at start
and now, kills per 1,000 loads (when loads ≥ 200), and one plain sentence from the numbers. "Which lines you ate"
is cut (the 30 members are near-clones); it returns with Season 2, where a line means something. **Proof:** the
card's screenshot after an autopilot hunt, its numbers checked against the run log.

### A5. The autopilot (proofs only)

`examples/enemyhunt.rs`: loads `demo_enemy.ron` through `Lab`, drives the raider **through `Steered::press` and
`SteerInput`, the keys the owner uses**: walk to the nearest door, enter, G through crumbs (Q when walled with a
pellet), bite what touches the head, E/Q to keep biting, back out tail-first at wound ≥ 3/4 or 2 cells lost, rest
on the surface to digest. A stuck counter (frames without the head moving) and "reached the nest interior" per run.
Writes the GIF (`labgif`'s capture of `Lab::draw`) and an event log. Paired seeds, both brains.

### A6. Frame cost

The demo's cost is in the draw: `labdraw` with follow on and a ring live, plus `labperf` µs/tick, beside the
`ascii` worst frame (demo off, untouched). Quoted in the PR.

### A7. Showing the owner, and landing

An Artifact page (the review queue is retired): the autopilot GIF against both brains side by side, labelled
blind (A/B), with kills, bites taken, cell index lost and loads home under each; the HUD screenshot; the card. Then
the owner plays it. PR 2: the demo, the autopilot, `Reports/enemy-seat-demo-2026-10-25.md` (review, plans,
measurements, pictures), its `Reports/README.md` line. Merge desk merges.

## Part B: Season 2 (only if the owner picks it on the card; a separate project)

The review showed v1's gate would pass 14% of the time with no effect, bred against a raider that never enters the
nest, with no positive control, and needed 27-110 core-hours, not "overnight". Redesigned:

- **Lines:** two replicate lines per arm (raided, control), from the current Teacher-lane founder pool, using the
  std_pool case set including `+lab` (so the lines survive the box at all).
- **The bed's raider goes in:** the autopilot itself (a scripted predator breaks no colony rule) or the blank-minded
  `far=0` raider, not the lab raider pool, which keeps its distance from the nest.
- **The judge:** the autopilot hunt on 12 paired seeds per line. Its positive control first: it must rank the
  hand-built colony and the trained one apart on price. Its null: replicate against replicate within an arm.
- **Gate:** both raided replicates beat both controls on ≥10 of 12 seeds, with `trip_deliveries` and ants alive not
  lower (no hiders), and "autopilot reached the interior" on every judged seed.
- **Season 1 is the control line**, so the card compares like with like.
- **Compute:** time one round first; at the lone-predator scale ~27 core-hours (about 7 h wall on this 4-core box),
  ~110 at the 64-colony scale the last report asked for. Runs on GitHub Actions or a runner thread; the Teacher lane is
  told first, since it is ant training on their line.

## Rules check

One physics (no physics change). One brain design, no scripted colony answers (the autopilot drives only the
predator). Dig and forage all game (the colony's own; the price readout rewards foraging). Every verb has a visible
consequence (corpse, bite pulse, kill ring, severed segment, moved store, HUD line). Graded (a wound meter, cells
lost one at a time if you back out, a price not a win). Frame cost measured in the draw. Shipped lab unchanged: all
behind `demo=enemy`; the two engine hooks are off-is-off and hash-tested. Features default on: the demo is a mode,
not a feature of the lab; its pieces (steer-mind, `may_breed`) are inert unless a demo uses them.

## Shared pieces

Uses queen's launcher and key layer. Provides steer-mind (PR #903) to scout. Seat keys: Y, R. Season 2 could later
be the queen game's "switch sides" stock.

## Estimate

A: 2.5-3 days of agent work after the launcher lands (the autopilot is the largest piece). A0 runs alongside.

## What the review changed

| Review item | Done |
|---|---|
| M1 length is not health | HUD is a wound meter plus "that cell comes off"; tail-first tactic on the help card; cell index logged in A0.2 and A5 |
| M2 satiation is not a clock | No ceiling claimed; Q-spit kept as the engine's verb and named on the HUD; digest time measured (A0.3); E's handling-gut line |
| M3 feedback from fallthrough | `Steered::last_act` written after `act`; messages derived from it; `JAWS: CRUMB` state |
| M4 colony u32::MAX | Dropped; `Steered::may_breed` flag at `try_bud`, default true |
| M5 danger unmeasured | A0.2 bites per minute in the nest, both brains, gates the opening brain |
| M6 live U switch | Brain chosen at launch (`oldant`), named on HUD |
| M7 A0 depends on A5 | A0 uses `labsurvive`/`spiderlab far=0` with budding off; shipped `demo_enemy.ron`; 6 seeds to 80k |
| M8 alarm and kill position not drawn | Bite pulse from `encounter_log`; kill ring at the player's head on a new `KillRecord` |
| M9 Part B | Redesigned (replicates, +lab, a raider that goes in, positive control, null, 10/12, compute named); owner's call |
| S1 warm start blocks | Visible fast-forward with a progress line |
| S2 goal on screen | Door markers, player ring, raw counts until 200 loads, cutaway screenshot proof |
| S3 end states | Death, colony death, steering cap all defined |
| S4 bites taken | `encounter_log` drained per frame, player's rows only |
| S5 lines eaten | Cut from A4 |
| S6 frame cost | `labdraw` with follow and ring |
| S7 A1 code | (a) genome switches, (d) remember moved, (f) turn-round preference, (g) bite test: fixed in #903; (b) emits, (e) logs, (h) counts: documented; (c) redrawn memory: SCRATCH-only, noted |
| S8 autopilot | Through `Steered::press`; stuck counter; estimate raised |
| S9 owner's verdict | Blind A/B on an Artifact page (review queue retired), then the owner plays |
| S10 alarm trace order | Runs last in A0 |
