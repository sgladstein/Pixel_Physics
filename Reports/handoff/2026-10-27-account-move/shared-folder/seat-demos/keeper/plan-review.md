# Keeper plan v1: independent review (2026-10-25)

*Step 3 of five (BOARD.md). The reviewer did not write the plan, the feasibility review or `keeperlook`. Read in full:
`plan-v1.md`, `review-feasibility.md`, `BOARD.md`, `examples/keeperlook.rs`, `look/*.txt`. Skimmed: `shared/*.md`,
`scout/plan-v1.md`, `designs/keeper.md` §7. Checked against `main` 9e32979e **plus the uncommitted work already in this
worktree** (`src/sim/creature/recall.rs`, and edits to `mind.rs`, `world.rs` and `creature.rs`): the plan's author has
started step 1, and the code already differs from the plan. Mind line numbers below are for the working tree, which is main
plus 18 lines. Re-runs used the existing `target/release/examples/keeperlook`, built at 08:41, before `recall.rs` existed.
I did not rebuild, because another session is editing this tree.*

## Verdict

**Build with changes.** The engine pieces are sound in outline, and the recall code already in the tree is better than the
plan text. But four things need fixing before anything else is built:
- the step-0 headline finding is an instrument artifact;
- step 0's own data contradicts the demo's premise;
- Carry as written would leave an orphan ant that never ticks again;
- the guard that is meant to prove "off is byte-identical" cannot fail.

---

## Must-fix

**M1. "The hand trail evaporates in ~100 ticks" is a probe artifact. Do not file it as a bug.** In the v2 runs the far heap
was 8 rows tall (136 cells, which is 8 × 17). `keeperlook` re-reads the far-end probe at `surface(far_x+6) - 1` every
sample (`keeperlook.rs:234`), and the powder heap slumps, so the probe moves off the cell the trail was laid on. Re-run on
seed 1, same binary:
- `heap=8`: far end 9,216 → **42** → 184 at +0/+100/+300. This reproduces the plan's number.
- `heap=4` (the source default today): the same column reads 9,491 → **6,788** → 5,819.
- The trail middle, on flat ground and on neither heap, reads 4,800 → 3,708 → 2,027.

That middle curve is exactly 0.97 per pass at one pass every 12 frames, from these sources:
- `PHEROMONE_INTERVAL` at `pheromone.rs:110`;
- `LAB_B_RHO = DECAY_RHO = 0.03` at `:204` and `:1058`;
- the lab's narrow blend, `LAB_B_DIFFUSE = 0.05` (`:1065`), applied at `lab/scene.rs:1346-1351`.

So an unreinforced hand trail decays by a factor of e in about 33 passes (~400 frames) and reaches 10% at about 900 frames.
That is short against a ~2,200-frame round trip, which `pheromone.rs:96-109` already documents ("a trail here is a live map
of where ants are, not a memory"). It is not "gone before an ant can walk it", and it is not new.

To do:
- remove plan §1 point 2;
- remove the step-8 `open-bugs-handoff.md` filing and the "finding for every seat" line;
- fix `keeperlook` so it samples the laid cell, and so it echoes `heap`, `every`, `nest` and `crop`. It silently takes
  arguments it never prints, which is the trap CLAUDE.md warns about.

The trail arm's null result (far food 14 vs 0 and 4 vs 12) still stands, but its cause is open again.

**M2. Step 0 refutes the demo's premise.** Plan §0 says untaught ants "never find" a station 200 columns out, because
trained colonies "do not wander past about 50 columns". In every v2 control seed, the untaught colony ate the heap **150
columns out** to zero:

| `v2-none` seed | far heap | B at far end (colony-laid, untaught) |
|---|---|---|
| s1 | 133 → 0 by +9,000 | up to 16,866 |
| s2 | 136 → 0 by +12,000 | up to 13,944 |
| s3 | 136 → 0 by +9,000 | up to 15,394 |

The colony crash at +12k to +18k comes *after* the far heap runs out, so the colony was living on it. My `heap=4` re-run
also prints 1 to 4 ants whose fresh last food is the far heap within 300 frames.

The "50 columns" figure traces to `labsurvive` (`Reports/instruments.md:229`): the pool *lived* with food 10-50 columns out
and *starved* at 100-240. That is about survival with no near food, not about range.

To do:
- **Run step 4's reach check first**, before the gain sweep, in the real demo box with its larder.
- Run it **with gain G on and nobody taught**, because the tilt acts on every ant with any memory (M4).
- Use at least 12 seeds and gate the *max*. A 12k-frame `keeperlook` run takes about 1 s here, so 3 seeds saves nothing.
- Expect the station to be found, and plan the contrast as *graded*: frames to the first station delivery, and station
  deliveries per 1,000 frames, taught against untaught on paired seeds. Do not plan it as 0 against more than 0. That
  is also the honest version: the keeper speeds discovery up rather than making it possible.

**M3. §3.1 describes a different mechanism from the code. Fix the plan and the shared interface, not the code.**
- **The plan:** add `gain * FOOD_WAY` to Walk's raw score before the squash (`mind.rs:2094-2098`). That would also raise
  `best[Walk]` (`:2100`), which feeds the job head (`:2122`, `row[CONTEXT+MEMORY] * best[v]`) and the memory update
  through `all_best`/`all_mean` (`:2194`). So it would change *whether* ants walk, and their recurrent memory, as well as
  which way they walk.
- **The code** (`mind.rs:2179-2186`, `recall::tilt`): it multiplies each Walk target's pick weight by `exp(gain * FOOD_WAY)`
  *after* the verb is drawn. It never changes the verb.
- The code is the better design. The dig head's own note (`mind.rs:2165-2169`) records that a head added before the squash
  "could only flatten it further". The plan cites that same head as its precedent for adding before the squash.

To do:
- Update §3.1 and `shared/recall-head.md`, which still says "added to raw score… before the squash", and tell scout. Its
  step 0d gain values assume the interface as written.
- **State the consequence.** The tilt's ceiling is the share of an unladen ant's decisions that are Walk. Measure that share
  (on the surface and in the nest) before sweeping the gain.
- Make the fallback, before any forced pick, a *job-stage* term on Walk scaled by the best `FOOD_WAY`. The precedent is the
  dig head's "when" read-out at `mind.rs:2134-2137`.

**M4. The tilt acts on the whole colony, not on "taught ants only", and it pulls hungry ants out.** `recall::tilt_for`
returns the gain for *any* ant with any `last_food` and an empty crop and jaws. Ants that found the near food by themselves
get pulled back to it too. Four consequences:
- **The `K` key changes the whole colony's foraging.** "Unmarked sister wanders" (review B.2) will not be what is on screen.
- **The frame-cost line in §6 is wrong.**
- **Step 2's "laden and hungry ants still go home" contradicts the code.** Nothing gates on hunger, so a hungry unladen ant
  at home is pulled 200 columns out.

That last one is the RECRUIT harm: never-foraged ants sent out starved, 442 against 298 and 323 (`seats-overview.md:58`).
Its sibling is `hungry_out_pull` (`dead-ends.md:1265`), where pulling lean ants out crashed colonies.

To do:
- Decide whether the gain is colony-wide (a "listening" sense, which is defensible) or applies to taught and copied ants only
  (a provenance gate).
- Say which in the HUD.
- Measure starvation and digs per 1,000 frames for **all** ants, paired by seed, gain G against 0.

**M5. Carry as specified orphans or kills the ant.**
- (a) **"Clear them, and rewrite"**: clearing an organism's cells drops them from its list at the `World::set` seam, and an
  organism with no cells is freed (`release_if_bodyless`, `creature.rs:1638`). The plan's "drawn under the cursor while
  held" must not hold the body cleared across a frame. Do the whole move on release.
- (b) **The scheduler keys the tick on the head's position** (`creature.rs:7887-7911`). A body moved outside its own tick
  fires its next site at the *old* cell, fails the ownership test, calls `reconcile_chain` and returns **no site**: "not
  dead, not scheduled, just an orphan standing in the world forever". A frozen ant never starves, so it reads as healthy.
  After moving, call `world.schedule_active_site` at the new head.
- (c) **There is an existing helper:** `relocate_chain` (`creature.rs:32151`). For pure translations it takes `&[]` groups
  and is exact; flight (`:29502`) and the fall use it, together with `translated_if_free` (`:28642`) and
  `body_is_supported` (`:28651`). It also handles the rider index, whose two out-of-sync ownership records have already
  cost an afternoon (`dead-ends.md:2083`). Wrap these in one public `creature::set_down(world, id, dx, dy) -> bool`.
- **Refuse a carry** when the ant has `flight` set, or carries or is a rider.
- **On a carry**, drop or keep `spoil`, and clear `dig_return`, `home_best_*` and `scout_for`. Keep `forage_anchor`: the ant
  needs its way home.
- **Tests:**
  - a carried ant's tick count keeps rising and its energy keeps falling for 1,000 frames;
  - the cell-ownership invariant holds;
  - delete the reschedule line and watch the tick test go red.

**M6. Guard (a), "off is byte-identical over 3,000 frames of `nest_goal`", cannot fail.**
- `nest_goal` lands its colony at **frame 6,000** (`assets/lab_scenarios/nest_goal.ron`, timeline). Before that no ant
  exists and no mind decides.
- `Lab::new` opens on the old ant unless `set_new_ant(true)` is called.
- A test inside one binary can only compare that binary with itself.

To do:
- (i) Compare `lab_cost`'s `world hash` on main and on the branch with no flag: two commits that differ only by the change.
- (ii) In-binary, on a *landed* new-ant colony, after at least 3,000 frames of decisions: `None` must equal
  `Some(gain 0, copy 0)`, and must also equal `Some(gain 0, copy 0.5)`.
- (iii) Positive control: `Some(gain 1)` must differ.

(ii) holds today only because the lab freezes mutation (`mind.rs:1728-1729`) and `last_food` has one reader (the
`FOOD_WAY` feature, `options.rs:693`), whose weights are zeroed. Write that down as the reason it holds.

**M7. Aged copies are clamped to frame 0, so early copies are fresher than designed.**
- `share_on_feed` writes `at = frame.saturating_sub(age)`, with `age = T/given - T`.
- At T = 4,000, a second-hand copy needs `age` 12,000 and a third-hand one 28,000.
- The colony lands at 6,000, so for most of a ten-minute demo these ages saturate to frame 0. A third-hand copy at frame
  15,000 reads 0.21, not 0.125.
- The fresher copy then passes itself on, so the generations blur. This is exactly the graded spread the copy exists for.

Fix: store the strength separately (in provenance, applied in `tilt` and `knows`), or keep `at` as a signed frame.
Test the copy at frame 8,000, not only late in a run.

**M8. "Knows the station" needs a freshness threshold, and the fade is not a half-life.**
- Freshness is `T/(T+age)` (`options.rs:693`): ½ at 4,000 frames, ⅓ at 8,000, ¼ at 12,000. It is hyperbolic, not "half its
  strength every 4,000 frames" (§0).
- `FOOD_WAY` multiplies it by `ln(1 + d/40)` (`options.rs:610-613`, `:1136`), which is 1.79 at 200 columns. A 30,000-frame-old
  memory still pulls at 0.21 × G.
- `recall::knows` returns any freshness, so the HUD share as specified only ever rises. A memory is lost only when it is
  overwritten.

To do:
- Define "knows" as freshness ≥ φ, for example 0.25, and print φ.
- In step 3, run until 30,000 frames after the last tending, so the "drifts back" claim is measured, not assumed.

**M9. The larder is not "at home", so it overwrites the lesson.**
- `food_at_home` means beside the dug nest or inside its shaft cut (`creature.rs:4528`, `:19287-19312`).
- A heap "beside the nest" on the surface, like `nest_goal`'s at x=286, 30 columns from the door at 256, writes `last_food`
  on every pick-up (`mind.rs:2843-2846`). So a taught ant that eats at the larder forgets the station at once. That is a
  binary outcome, which CLAUDE.md law 1 forbids.

To do:
- Put the topped-up larder **inside** the nest, as a ration below the colony's need, so foraging pressure stays ("ants dig
  and forage all game").
- Or keep it outside and make "the larder pulls them back" a measured, labelled part of the fade.
- Fix the §6 row either way.

**M10. Treat must not call `crop_take` raw.**
- It is private, and it does not guard (`creature.rs:14676`): it merges any material into the crop and keeps the old
  material, with no capacity check.
- `pick_up`'s guards are at `mind.rs:2803-2807` (wrong material, `organism_crop_capacity`).
- Write a public wrapper with those guards.
- Book the injected joules on a colony account, because a treat creates food.
- Refuse to plant the memory at home: a treat inside nest reach would teach "here is the nest".

---

## Should-improve

1. **Re-run step 0 from one binary and record its SHA.**
   - The saved files come from at least two engines. Seed 1's nest is at column 215 in v1 and 253 in v2, the same seed
     and scenario.
   - The current source prints "adult feeds" and "knows" lines that no saved file has. So the plan's "70 feeds per 1,000"
     has no raw output behind it. My re-run reads 48 per 1,000 over frames 6k-12k and 163 per 1,000 just after 12k:
     consistent in size, so the claim is plausible.
   - The header "0 ANTS NOW THINK WITH THE TRAINED MINDS" is printed before the colony lands. The count line after it is
     the real assertion.
2. **Alarm: measure while the puff is alive.**
   - `ALARM_RHO` 0.35 empties the plane in about 144 frames, by design (`pheromone.rs:324-336`). So "a flash" is right,
     and it is the intended behaviour, not a finding.
   - The 9,000-frame total integrates mostly chaos after the paths diverge.
   - Re-run, first 300 frames, ant-ticks near the puff, none against alarm:

     | seed | none | alarm | change |
     |---|---|---|---|
     | 1 | 1,543 | 1,182 | −23% |
     | 2 | 803 | 727 | −9% |
     | 3 | 755 | 701 | −7% |

     That is a small, graded repel, not "the opposite of repel".
   - The 17×17 box at `nest_x+15` sits on the nest door, so entrance traffic swamps it.
   - The v1 seed-2 byte-identical result was a puff with no ant in reach while it lived: the scene did not test the verb.
     (The current geometry is not identical.)
   - Fix: aim the puff at walking ants, and count only during the live window.
3. **Replace the "decisions moved" argmax.** The pick is a weighted draw (`choose_weighted`, `creature.rs:495`), so an argmax
   recomputation measures nothing. Keep the total-variation `mass_moved` the code already has. Add **picks flipped**: run
   `choose_weighted` with and without the tilt on the *same* `draw`. That is exact, cheap, and 0 at gain 0.
4. **Allocation churn.** Each tilted Walk decision allocates three `Vec`s (`fw` at `mind.rs:2182`, and two inside
   `tilt`'s `mass`). Under M4 that is most decisions. The same function already moved its lists to the stack for this
   reason (`mind.rs:2152-2156`). Use `[f32; STACK_OPTS]`.
5. **Provenance can lie.** It is never pruned on death and never invalidated when the ant's own `pick_up` overwrites
   `last_food`, so the ant page can say "taught by your hand" about a memory of the larder. Store it with the `(place, at)`
   it describes, and check that it still matches.
6. **Laden test.** The code's `crop`/`spoil` test is right, and `trip_load` is wrong:
   - `trip_load` is cleared only by a put-down at the nest (`mind.rs:3009`), and a quarter of loads are digested on the way
     home (`dead-ends.md:1247`);
   - a treated ant never sets it.

   But `crop` is also the gut, so "unladen" means "nothing being digested". Report the share of decisions that were tilted.
   For the same reason, "deliveries home from the station" may read low even when the ants commute.
7. **`world.recall` is lost on rebuild.** `Lab::reset` builds a new `World` (`lab/mod.rs:800-803`), which reads only the
   environment. Re-apply the demo's `Recall` after every reset and scenario load, and test it.
8. **Key collisions.** Y, U, J, K and H are all bound in `bin/lab.rs:734-794`: K Wall, J Alarm, U Lamp, H creature colour,
   Y life marks. Demo mode shadows the Alarm and Lamp keys that §0 says "stay available". Settle precedence with the
   launcher.
9. **Drop the 1024 box unless step 4 needs it.**
   - The viewport is `app::WIDTH`, 512 (`lab/mod.rs:32`, `:65`). A 512 box with the nest off-centre fits the nest and a
     200-column station on one screen. A 1024 box needs zoom-out or panning, and doubles the sweep.
   - Width up to 4,096 is supported (`params.rs:340`), so 1024 is possible, just not free.
10. **Build order and pacing.**
    - Step 2 uses Carry before any step builds it. Sweep the gain with `plant` first, and give Carry its own step with the
      M5 tests.
    - A `nest_goal`-based box has no ants until frame 6,000, so land the colony at frame 0.
    - State the time dial: ten minutes at 1x is ~36,000 frames, so a few round trips of 200 columns per taught ant at best.
11. **Use `lab/trips.rs` for legibility.** It draws a pinned ant's distance-from-nest sawtooth, which is exactly the picture
    of "she commutes".

---

## Answers

**1. Step 0's conclusions.**
- **Trail:** wrong as stated (M1). The far-end probe re-reads `surface()` on a slumping 8-row heap. On flat ground the trail
  decays at the lab's ρ = 0.03 per 12-frame pass. The lab's B diffusion is 0.05, so diffusion is not what empties it.
- **Alarm is a flash:** right, and by design (should-improve 2).
- **Counters:**
  - `creature_stats.digs` is not blind. Mind digs go through `cut_cell`, which books both `digs` and `dug_cells`
    (`creature.rs:22495`, `:22584`).
  - The near-puff box is effectively blind: it sits on the nest door and integrates 9,000 frames over a ~150-frame stimulus.
  - "Cut beside sketch" counts any emptied band cell, including slump into the sketch. Its control band overlaps the nest's
    own early digging (11 to 13 cuts by +100 in `none`).
  - `nest_x` (the median head column) is reasonable.
  - "far food" is honest: a ±12 window around the heap only.
- **Seed-2 alarm byte-identical:** a scene where no ant was in reach during the puff's ~150-frame life. That is a tidiness
  tell, not a response.

**2. Recall head.**
- **Where the score is built:** the raw score is built at `mind.rs:2094-2097` and squashed at `:2098`.
- **Is there a separate verb head:** yes. The job/verb head (`:2112-2150`) draws the verb from `best[v]`, context, memory,
  hidden units and the pull. Only then are that verb's targets drawn (`:2171-2187`). The tilt in the tree acts at the target
  stage, so it changes *which* Walk, never *whether* to walk (M3).
- **Is `trip_load` the right laden test:** no (should-improve 6). The code's `crop`/`spoil` test is right.
- **Is `FOOD_WAY` computed at no extra cost:** yes. It is computed for every option whenever `last_food` is on (the default)
  and the ant has a memory (`options.rs:1128-1137`). The scorer's sparse plan skips its zeroed weights, so the feature costs
  nothing extra; the tilt's own cost is the allocations (should-improve 4).
- **Is "decisions moved" cheap:** yes, as picks flipped under the same draw (should-improve 3). An argmax is the wrong
  quantity.
- **Would the byte-identical test catch a leak:** not as specified (M6).

**3. Meal copy.** `mind::feed`'s adult branch (`mind.rs:3105-3112`) is the right hook. It is the downhill energy share,
after `blend_with_kin`; the larva path has already returned. Ageing by pushing the frame back is valid in principle and
breaks in practice through `saturating_sub` (M7).

**4. Carry.** Not safe as written; see M5 for the helper and the stale state.

**5. Treat.** `crop_take` is private and unguarded (M10). `plant` writes `last_food` directly, not through `pick_up`, which
is acceptable if labelled. The crop then holds food, so the treated ant walks home untilted first, then commutes.

**6. Wide box.** Supported, but expensive and not needed (should-improve 9). The "~50 columns" claim is inconsistent with
step 0: the 150-column heap was emptied in 3 of 3 control seeds, and the colony laid its own B to it (M2).

**7. Larder, registry, HUD, launcher.**
- **Larder:** wrong about "at home" (M9).
- **Registry:** fine. Define "visit" as the head within r.
- **HUD and overlays:** cheap, provided the traffic grid accumulates every frame (one pass over the ants).
- **Launcher:** the env-gate stopgap is fine; settle the key collisions (should-improve 8).

**8. Is it satisfying and legible?** Partly.
- **Strong:** the meal-copy line from giver to receiver at the moment of a feed, and a pinned never-been-there ant whose
  trips strip starts to saw out to the station. That is visibly not "follow me": the player never leads, and the knowledge
  moves by mouth.
- **Weak:**
  - Carry-and-commute alone *is* "walk them to food" by another name. Its only distinctive part is the copy.
  - As specified, the contrast rests on untaught ants never reaching the station, which step 0 says is false (M2).
  - The honest arm changes the whole colony (M4).
- **The stronger demo the plan missed** uses step 0's two real positives:
  1. **Opening: the architect.** The sketched gallery is adopted on today's ants with no fake. Room ant-ticks were 345,
     3,855 and 0 against 0; cuts beside the sketch 36 vs 32, 32 vs 5, 40 vs 15. It deserves the first two minutes, not a
     line in the HUD.
  2. **Then the station, honest.** The colony finds food 150 columns out and lays its own B trail to it. The traffic
     overlay shows the trail forming, with no scripting.
  3. **Then the keeper teaches.** Carry and Treat make discovery *sooner*. The copy spreads it, and it fades. The HUD
     reads "first delivery at frame X (taught) against Y (untaught, paired seed)".

  Every beat in that arc is graded, and the one scripted gain is confined to the last third.

**9. Dead ends.**
- **No entries:** `last_food`, `FOOD_WAY`, recall, trophallaxis, meal copy, teleport.
- **Relevant:**
  - RECRUIT: the harm figure (442 vs 298) is in `seats-overview.md:58`. Its `dead-ends.md` hits are about laden carriers:
    `:1247` (a quarter of loads digested before home) and `:1207` (gut and hunger).
  - `:1265` `hungry_out_pull`: pulling lean ants out crashed colonies. This applies to M4.
  - `:2083`: cell ownership kept in two records. This applies to M5.
- **`scripts/deadendindex.py --touching`:** silent, but nothing is committed yet, so the silence is not evidence.
