# Keeper seat: implementation plan to a playable demo (v2, revised, 2026-10-25)

*Step 4 of five (BOARD.md). v1 is [`plan-v1.md`](plan-v1.md). Its independent review is
[`plan-review.md`](plan-review.md), with the verdict "build with changes". Every must-fix and should-improve
item is answered in §9. The recall head (§3.1) is already built to this plan on branch
`claude/project-thread-20o5zg`, with its tests (§3.1 proof).*

## 0. What the owner will play

`cargo run --release --bin lab -- demo=keeper` opens a 512-wide formicarium box. The nest is off-centre (about
column 110) and its larder ration is inside the nest. A **far station** sits about 280 columns out on the
surface. The demo has three beats over ten minutes, and each one is graded.

**1. The architect (no fakes).** The player sketches a gallery with the Dig brush, and the colony moves in.

Step 0 found this on today's trained ants with nothing scripted:
- a lined, hand-dug gallery is kept, not torn down;
- the colony slowly digs into it and around it;
- ants occupy the sketched room.

The HUD counts the cells cut beside your sketch and the ant-time inside it, against what the same colony did
beside an unsketched patch.

**2. The colony finds the far food on its own (no fakes).** Untaught trained colonies did reach and empty a
heap 150 columns out in every step-0 control seed, laying their own B trail to it. The traffic overlay shows that
road forming. The HUD shows the frame of the first station delivery.

**3. The keeper teaches (one labelled fake).** The player can carry an ant to the station, or treat an ant
there.
- A taught ant remembers the place. When unladen, it walks toward it more often: its trips strip saws out to
  the station.
- When it feeds a nestmate mouth to mouth, the nestmate learns the place second-hand at half strength. A thin
  line is drawn from giver to receiver.
- Second-hand ants that were never there head out.
- The "knows the station" share rises. It then fades if you stop.

The contrast is **graded and paired**: the station is found sooner and worked harder with the lesson than
without it, on the same seed. It is never "0 against something".

The `` ` `` key switches the scripted listening gain to 0, the honest arm. The HUD always names which arm is
live.

## 1. Step 0, corrected (what today's trained ants do with the keeper's hand)

**Instrument:** `examples/keeperlook.rs`.
- Box: `nest_goal` with `set_new_ant(true)`, asserted by a mind count.
- Warm-up: 12k frames.
- Pairing: each arm is paired with `arm=none` on the same seed.
- Output: it prints the end-of-window world hash.

Raw output is in `look/`. The re-run for the report is made from one binary, with its SHA (§6 step 0).

| Verb | Finding |
|---|---|
| **Hand trail (B)** | ~~gone in ~100 ticks~~ **(v1 error: the probe sat on a slumping food heap.)** On flat ground it decays at the lab's ρ 0.03 per 12-frame pass: a third remains at about 400 frames and 10% at about 900. That is short against a round trip, and already documented (`pheromone.rs:96-109`). The trail arm's null (far food left 14 vs 0 and 4 vs 12) is real, and its cause is open. **Not a demo verb.** |
| **Alarm puff** | Fades in about 150 frames, by design (`ALARM_RHO`). Inside that window, ant-ticks near the puff fell 23%, 9% and 7% against none (3 seeds, reviewer re-run). **A small, graded repel.** It stays a hand tool and is not a headline. |
| **Hand-dug gallery** | Kept: 261 of 265, 227 of 271 and 249 of 267 cells still open at +18k frames. Cut within 3 cells of it, against the control band: 36 vs 32, 32 vs 5, 40 vs 15. Ant-ticks in its room: 345, 3,855 and 0, against 0 in every control. **The opening beat.** The re-run will separate slumping from digging in the "cut beside" count. |
| **Far food untaught** | A heap 150 columns out was emptied in 3 of 3 seeds, with the colony's own B at the far end reaching up to 16,866. |
| **Survival** | `nest_goal` colonies collapse by about frame 30k once the food is gone (2 to 10 ants left). The demo box must hold a ration. |
| **Meals** | Adult-to-adult feeds run at about 48 to 163 per 1,000 frames (`minds.counts.fed_adult`). |

## 2. Scope

**In:**
- `demo=keeper`, through the queen's launcher (`shared/launcher.md`), with an env stopgap if that is late;
- `demo_keeper.ron`;
- the recall head (shared; scout consumes `plant_with` and `knows`);
- `creature::set_down` (carry);
- `creature::treat` (treat);
- the station registry;
- the inside-nest ration;
- the overlays and HUD;
- the paired harness and the GIFs for the Artifact page;
- the report.

**Out, post-demo:**
- trust as a smell and the gnome as a visitor (`shared/scent-ladder.md`);
- plants as words;
- nursery imprinting;
- a heritable listening gain;
- a longer-lived player trail (an owner design question);
- the gnome on screen.

**Never:**
- any change to the lab with no `demo=`. It must stay byte-identical, checked across commits in §6 step 1;
- any change to training or to `std_pool`.

## 3. Engine pieces

### 3.1 The recall head: `src/sim/creature/recall.rs` (built)

- **`World::recall: Option<Recall { gain, copy, counts, provenance }>`.** `None` is one `Option` read per
  decision. It is set only by a demo, or by `PIXEL_PHYSICS_RECALL_GAIN=g[,copy]`.
- **The tilt.** It is applied *after the verb is drawn*, in `decide_planned`'s target stage, where the dig head
  re-ranks cuts:
  - if the verb is Walk, each Walk target's pick weight is multiplied by `exp(gain × strength × FOOD_WAY)`;
  - it changes which way an ant walks, never whether it walks. The job stage, `best[]` and the recurrent memory
    are untouched;
  - it never excludes a heading;
  - the draw is consumed exactly once either way;
  - it uses stack buffers only.
- **Who it applies to.** Only ants whose memory was **taught or copied**: the provenance gate (review M4).
  - Ants that found food themselves are untouched, so the honest-arm key changes only the lesson's tree, not
    the colony.
  - Laden ants are skipped (any crop or spoil). The crop is also the gut, so "unladen" means nothing is being
    digested.
  - The share of decisions tilted is reported.
- **`Provenance { generation, giver, frame, place, strength }`.**
  - It describes one memory. When the ant's own pick-up writes somewhere else (more than 6 cells away), the
    record no longer matches, is ignored, and is pruned.
  - Strength is stored apart from age (review M7: early copies saturated at frame 0).
- **`plant_with(world, ant, cell, strength)` and `plant`.** Scout asked for this: a broken tandem run plants a
  weaker memory.
- **`knows(world, ant, place, r) -> Option<f32>`** returns strength × freshness. Freshness is `T/(T+age)`:
  ½ at 4k frames, ⅓ at 8k, ¼ at 12k. That is hyperbolic, **not a half-life**. The HUD counts "knows" at a
  printed threshold φ = 0.25.
- **The meal copy.** It hooks `mind::feed`'s adult branch, after `blend_with_kin`.
  - Only a taught or copied memory spreads.
  - It spreads at `copy` × the giver's strength × freshness.
  - It is written fresh, with that strength.
  - It replaces what the taker holds only if stronger. An untaught taker's own memory counts at its freshness:
    first-hand beats hearsay.
- **Counters:**
  - `tilted`;
  - `mass_moved`, the total variation under `choose_weighted`'s own `(floor+w)²` rule;
  - `picks_flipped`, the same draw with and without the tilt. Both are exactly 0 at gain 0;
  - `plants`, `copies`, and `copies_by_generation`.
- **Proof, run 2026-10-25:**
  - `the_recall_head_walks_a_trained_ant_toward_a_remembered_place`: std_pool member 0, memory planted 60
    columns east. Furthest reach in 1,500 frames: off 2, gain 0 2, gain 2 12, gain 4 37, **gain 6 124**,
    gain 8 113. Picks flipped 45 of 73 tilted decisions at gain 6.
  - `the_recall_head_at_gain_zero_is_todays_ant_exactly`: a per-frame fingerprint of every animal. Off equals
    gain 0 equals gain 0 with copy 0.5. Gain 2 differs (the positive control).
  - `a_fed_nestmate_takes_the_place_at_reduced_freshness`: second-hand reads 0.5, third-hand 0.25, staler
    news never overwrites, and generation is 2.
  - The tilt unit test.
  - **With the tilt disabled in `decide_planned`, both mind tests go red.** That was checked by putting the
    fault back.
  - Clippy is clean.
- **Why gain 0 is today's ant exactly:** the lab freezes mutation, and `last_food`'s only reader is the
  `FOOD_WAY` feature, whose pool weights are zeroed on load. The guard test says so, and goes red if either
  changes.

### 3.2 Carry: `creature::set_down(world, id, to) -> Result<(), &'static str>` (review M5)

- Wrap `translated_if_free` (`creature.rs:28642`), `body_is_supported` (`:28651`) and `relocate_chain`
  (`:32151`), with no groups for a pure translation. The flight and fall code use the same three.
- Then **`world.schedule_active_site` at the new head.** Without it, the scheduler fires at the old cell, fails
  the ownership test, and leaves an orphan that never ticks again (`creature.rs:7887-7911`).
- Search outward up to 8 cells from `to` for a free, supported placement.
- Refuse while the ant is in flight, carries a rider, or is a rider.
- Clear `dig_return`, `home_best_*` and `scout_for`. Keep `forage_anchor`, so it still knows home. Drop
  `spoil` where it stood.
- **The whole move happens on release.** The body is never cleared across a frame. While the button is held,
  only a ghost outline follows the cursor.
- **No memory write.** The carried ant finds the station's food itself, and its own `pick_up` writes
  `last_food`. Carry then registers provenance (generation 0, strength 1) at the station, so the gate admits it.
- **Tests:**
  - a carried ant's tick count keeps rising and its energy keeps falling over 1,000 frames;
  - cell ownership holds (every chain cell carries its id, and no stray cell keeps it);
  - delete the reschedule line and the tick test goes red.

### 3.3 Treat: `creature::treat(world, id, worth) -> bool` (review M10)

- A public wrapper with `pick_up`'s guards (`mind.rs:2803-2807`): the crop's material and
  `organism_crop_capacity`. It refuses if the crop is full or holds another food.
- The injected joules are booked to a keeper account (`Account::Keeper`, or the nearest existing one,
  labelled), because a treat creates food.
- `plant` at the ant's head, unless that is within nest reach (`food_at_home`): **a treat at home teaches
  "here is the nest" and is refused**, with a message.
- A gold pip rises off the ant.

### 3.4 Station, ration and box (`demo_keeper.ron` plus `src/lab/demo/keeper.rs`)

- **Box:** 512 × 320, soil as `nest_goal`, colony landed at **frame 0** (review: `nest_goal` has none until
  6,000), founded at about column 110.
- **Station:** a `provisions` heap at about column 390, topped up so it never runs dry. Visits are counted as the
  head within r = 10. Station bites are food cells taken there per 1,000 frames.
- **Ration (review M9):** a small `provisions` cache **inside the first chamber** once dug, which is at home, so
  pick-ups there never overwrite a lesson. It is topped up to a level below the colony's need, so foraging
  pressure stays. The level is set in step 3. It is labelled on the HUD: "LARDER KEPT (RATION)".
- **Further Food strokes** more than 40 columns from the nest register as extra stations.

## 4. Lab side

- **Tools on the bar,** like every lab verb, shown only in demo mode: CARRY (press on an ant, release where to
  set it down) and TREAT (click an ant). Every letter key is already bound (`bin/lab.rs`), so the demo's two
  keys are unbound ones:
  - `` ` ``: gain G ↔ 0, labelled "LISTENING: SCRIPTED GAIN G" or "LISTENING: OFF (TODAY'S ANTS)";
  - `\`: overlay cycle off → knows → traffic.

  The queen launcher's seat keys collide with lab bindings (review should-improve 8). That is reported to the
  queen thread; the keeper uses none of them.
- **Overlays.** Each is a full replace on a fixed ramp, drawn only when on.
  - **Knows:** a ring on each taught or copied ant, coloured by generation, with strength × freshness as
    brightness. A fading line from each copied ant to its giver for 30 seconds.
  - **Traffic:** a u16 heat ribbon of ant steps over 2,000 frames, accumulated in one pass over the ants per
    frame.
- **HUD line:**
  - knows N/M (φ 0.25), with a 60-second sparkline;
  - station bites per 1,000 frames;
  - first station delivery: frame X;
  - picks flipped in the last 1,000 frames;
  - carried and treated;
  - copies by generation;
  - cut beside your sketch and ant-time in it;
  - the ration line.
- **Pin and trips.** Pinning an ant opens `lab/trips.rs`'s distance-from-nest sawtooth, the picture of "she
  commutes". The ant page gets one sentence from provenance, for example "Remembers the east station (your
  hand, 3 trips since)" or "Heard of the east station from Bramble 412 (second-hand, half strength)".
- **Rebuild safety.** `Lab::reset` builds a new `World`, so the demo re-applies its `Recall` after every reset
  or scenario load. There is a test for this.

## 5. Gain and copy choice: measured, not assumed

- **The tilt's ceiling is the share of an unladen taught ant's decisions that are Walk.** Measure it first,
  on the surface and in the nest.
- Then sweep gain G ∈ {2, 4, 6, 8}.
- **Pick G as the smallest gain** that meets all of these on 12 seeds, gated on the worst seed:
  - a carried ant makes at least 2 station round trips in 12k frames;
  - taught ants do not starve more than untaught ones;
  - digs per 1,000 frames for all ants stay within the seed spread of gain 0.
- Then sweep copy ∈ {0.25, 0.5, 0.75}.
- **Pick the copy that gives a graded share curve:**
  - the peak share is 20 to 60%;
  - the share decays below half its peak within 30k frames of the last tending. This is measured, not assumed.

## 6. Build order, each with its proof

| # | Step | Proof |
|---|---|---|
| 0 | Re-run step 0 from one binary at one SHA, 12 seeds: none, sketch, alarm (counted in its live window, aimed at walking ants), trail (probe on the laid cell). Fix `keeperlook` to echo every argument | One table, with the SHA; the GIFs |
| 1 | **Recall head PR** (built): `recall.rs` plus the three hooks plus the tests. **A cross-commit check**: `keeperlook arm=none` world hash on main against the branch, same harness source, 3 seeds, must be identical. Then a PR to the merge desk, with the SHA posted in `shared/recall-head.md` for scout | Tests green, and red with the term removed (done). The hashes printed in the PR body |
| 2 | `set_down` plus `treat` plus their tests | The tick and ownership tests, red with the reschedule removed |
| 3 | `demo_keeper.ron`, ration, station. **Reach check first:** gain G on, nobody taught, 12 seeds × 30k frames | Colony alive (at least 15 ants at 30k on the worst seed). First-station-delivery frame distribution. Station bites per 1,000 frames |
| 4 | The §5 sweeps, through `keeperlook arm=carry` (carry 3 ants at frame 3k), paired against untaught | Tables by gain and by copy. G and copy chosen with numbers |
| 5 | Lab verbs, keys, overlays, HUD, page sentence, re-apply on reset | `uishot`/`labgif` stills. A `PIXEL_PHYSICS_LAB_CLICK` scripted carry |
| 6 | The paired film: the same seed through a scripted 10-minute session (sketch at 0, carry 2 and treat 1 at about 3 minutes), gain G against gain 0, side by side, counts in the captions | The Artifact page: the beat-1 GIF (sketch vs none), beat 2 (the honest road forming), beat 3 (G vs 0), the counts table, and what is scripted |
| 7 | Frame cost: `ascii.rs` and `labperf`, demo off against main, demo on against off | Worst frame quoted. Off unchanged |
| 8 | Report `Reports/keeper-seat-demo-2026-10-25.md` and its `Reports/README.md` line; `instruments.md` line for `keeperlook`; `how-we-test.md` if a measuring rule came out of this (candidate: "a probe re-reading `surface()` on a powder heap drifts"); `docscheck` | `docscheck.sh` green |

Time dial: ten minutes at 1x is about 36,000 frames. A 280-column round trip is about 2,000 to 2,500 frames, so a
taught ant can make a handful of trips in the demo. The HUD's speed dial lets the owner fast-forward the fade.

## 7. Risks and counters

| Risk | Counter |
|---|---|
| Unladen taught ants rarely pick Walk, so the tilt has little to act on | The Walk share is measured first (§5). The fallback is a job-stage term on Walk scaled by the best `FOOD_WAY` (the dig head's "when" read-out precedent, `mind.rs:2134-2137`), labelled. Before a forced pick, ask the owner |
| Pikmin: taught ants pace a line | Taught ants' share of unladen walks heading out, against a target band of 60 to 80%. Digs per taught ant; starvation among taught ants |
| The lesson is overwritten by food found on the way | The ration is at home. A per-taught-ant memory-to-station distance is tracked over time. Provenance pruning makes it visible as "forgot" |
| The copy saturates, or never spreads | The §5 copy sweep |
| Untaught ants are already as fast as taught ones | That is the honest result, and it gets reported. Then move the station further, or put a ridge of hand-dug soil in the way, and say so |
| Measuring the old ant | `keeperlook` asserts the mind count |
| Frame cost | §6 step 7. The tilt is a handful of `exp` calls per taught ant's Walk decision, on the stack |

## 8. Collaboration

- **scout:**
  - consumes `plant_with` and `knows`;
  - shares the generation counter;
  - "carry" (mine) and "lead by touch" (theirs) teach the same lesson through the same memory and copy;
  - one demo with two hands is possible, to be decided after both demos are judged;
  - told about the as-built interface and the step-0 correction.
- **queen:**
  - the launcher;
  - the key-collision note.
- **guest:**
  - the trust idea, post-demo;
  - its scent ladder.
- **colony mind:**
  - the copy rule may serve "habits";
  - use `trip_deliveries`, never `deliveries` (its finding).
- **Teacher lane** (at landing):
  - nothing here touches training;
  - the recall head is a candidate trained sense later (a heritable gain);
  - `Lab::new` defaults to the old ant.

## 9. What the review changed

| Review item | Change |
|---|---|
| M1: the trail probe artifact | Struck from §1. No bug filed. `keeperlook` gets fixed in step 0 |
| M2: the station is found untaught | Beat 2 makes that a feature. The contrast is graded and paired. The reach check runs first, on 12 seeds |
| M3: the plan described add-before-squash | §3.1 now describes the built post-draw tilt. The shared file and scout were updated. The Walk share is measured first |
| M4: the tilt acts colony-wide and on the hungry | Provenance gate. The starvation and digs gates in §5 cover all ants |
| M5: carry would orphan the ant | `set_down` via `relocate_chain` plus a reschedule, with tests |
| M6: the off-guard cannot fail | A per-frame fingerprint on landed mind ants (off = 0 = 0 with copy), a positive control, and a cross-commit hash |
| M7: copies clamped at frame 0 | Strength is stored separately |
| M8: knows needs a threshold; the fade is not a half-life | φ = 0.25, printed. The fade is described correctly. The decay is measured over 30k frames |
| M9: the larder overwrites the lesson | The ration sits inside the nest |
| M10: treat used `crop_take` raw | A guarded `treat` wrapper. Joules booked. Refused at home |
| S1–S11 | One-binary re-run (S1). Alarm in its live window (S2). `picks_flipped` (S3). Stack buffers (S4). Provenance matches the memory (S5). The crop/spoil laden test kept (S6). Re-apply on reset (S7). Unbound keys (S8). The 512 box (S9). Carry has its own step, and the colony lands at frame 0 (S10). `trips.rs` (S11) |
