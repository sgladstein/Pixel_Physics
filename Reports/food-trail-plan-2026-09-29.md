# The food trail: lay, read and give up, built together, instruments first

*Plan of record, 2026-09-29, agreed with the owner (rulings in Context).
`engine`. The foraging-loop lane (`Reports/lanes/foraging-loop.md`) owns it.
Written after §22v of `ant-scenes-2026-09-23.md` measured that the colony's
own trail is a net cost today. Progress is recorded in that report's §23,
not here; this document is edited only when the plan changes.*

## Context

The owner wants pheromone trails to do the work. In biology they are the main mechanism, other people's simulations work with them, and they are the flexible, stigmergic, non-hardcoded answer that fits the project's goal.

After weeks of work the trail still does not help. §22v (2026-09-29, `Reports/ant-scenes-2026-09-23.md`) measured that today the colony's own trail is a net cost. On the colony bed at gap 90 over 24 seeds, muting trail B changes:
- food taken: 6,062 → 7,512 (higher on 21 of 24 seeds);
- born: 188 → 282;
- starved: 57 → 29.

**Why earlier attempts failed.** The working recipe has three parts that only work together:
1. **Lay** a food trail that is strongest near the food.
2. **Read** it by turning toward the stronger side.
3. **Give up** on a trail that leads nowhere, using a walker that can hold a line.

This project built each part on a different ant and judged each alone:
- **Lay** (the food-charged odometer, `dead-ends.md:1954`) was tested only on the old walk, which could follow nothing. It came out "tested and inert", with the re-test condition "when the outbound leg works". That condition is met today: 82% of ants reach food.
- **Read** (an uphill reader) ran against a constant-rate trail, whose slope points home, so it walked ants home. The chooser was then built to read presence only.
- **Give up** exists for scouts, but is switched off on a trail. The away term has no patience, and the give-up pull home is scaled by `1 − presence`.

**Owner rulings (2026-09-29):**
- The lay rule is an engine switch for the experiments, moved into the genome later if it wins.
- Only hungry or drive-motivated ants follow.
- Ship the full recipe if it beats today's ant without harm, even if it only ties "trail off", and keep working on the gap.
- On the two-pile bed, the old pile is cleared at each swap.

**Intended outcome:**
- Instruments to see the trail over time, replay it exactly, and compare it with the profile we expect.
- The full recipe built behind one switch.
- A test bed where a trail can prove its worth.
- A decision rule registered in advance.

## What exists today, and what we must build to see the trail

| Need | Exists | Gap (built in Stage 0) |
|---|---|---|
| **B over time along the route** | `trailfollow btrail btrailevery=N` writes BTRAIL rows, one row at y=surface from nest to pile. `scripts/btrailchart.py` draws a heatmap, and `--stats` gives lit cells, worst gap and % bridged. Archived: `Reports/data/btrail-4seed-gap90-2026-09-21.log` | `arm=` prints the gate name, not the arm. `hand=` is taken from `stop=` only. The row misses west of the door (where 13.9% of B is laid), past the pile, the band of walking rows, and the shaft. Channel A only via the coarse APROF rows |
| **Slope: does B rise toward food?** | `own trail:` line: an end-of-run `along`, plus `b_profile`, a 5-point run mean. Today it reads `[11736,2525,2420,2398,589]`, tallest at the nest | No time series of slope; no rising-share statistic at the consumer's threshold and reach 2 |
| **Measured vs expected** | Nothing compares them. Plane-only models exist: `onetrail mode=timing`, `pherolife`. Emission curves only: `brain.rs:2145 what_a_food_odometer_emits` | The whole thing: a shadow plane that replays the logged deposits bit-for-bit (positive control), counterfactual planes under candidate lay rules on today's real paths, and a design-level expected profile |
| **Do ants climb it?** | `decisioncsv` logs `chosen_route` (presence of the picked heading only, saturating); the `/home/user/runs/trailq/` scripts see B only through it | Raw B for all 8 headings (near and far cells), per-heading scores, the amount of B actually laid, `since_trip` |
| **By eye** | `filmstrip channel=pheromone_b` works only in filmstrip's own scenes. The lab overlay (`render.rs:8655`) scales by value/65535, so one deposit draws faint | A `gifoverlay=b` in trailfollow with a fixed log scale, painted under `gifants` |

## Standing rules for every stage

- **Identity.** With the switch unset and no new flags, logs and decision CSVs are byte-identical to the parent binary. Setup: the standard bed (`RAYON_NUM_THREADS=1 PIXEL_PHYSICS_COLONY_SPACING=2 PIXEL_PHYSICS_STACK_DEPTH=4 PIXEL_PHYSICS_BUD_SITE=nest`, `trailfollow mode=gap gate=shipped frames=24000 ants=20 near=10 food=400 refill=400`, arms self/mute/hand, gaps 90 and 140, seeds 1-8, `decisioncsv`), compared with `cmp`.
- **Determinism.** No new RNG draws (`choose_weighted` keeps its single draw, creature.rs:15929) and no transcendentals in the sim. One seed at `RAYON_NUM_THREADS=4` equals the 1-thread run.
- **Frame cost.** All new work is per decision, never per cell. Check the worst frame from `cargo run --release --example ascii` and trailfollow's wall time, off against on; the budget is ≤ 2%.
- **Measurement hygiene.**
  - Every swept dimension is echoed in the log header and keyed in the parse (`FOOD_TRAIL`, arm, gap, seed, piles); print the key cardinality.
  - Predictions are registered in the report before each comparison.
  - Pairs are within seed, 24 seeds.
  - Each stage runs one frozen binary (`/home/user/runs/tf-<sha>`).
- **Before editing.** Run `bash scripts/branchcheck.sh --who-touched src/sim/creature.rs`. `Reports/how-the-ant-works.md` is updated in the same commit as any mechanism it describes. Every new guard is watched red under its own fault.

## Stage 0: instruments (behaviour-neutral) — PR-A

**0a. Engine: write-only state and trace fields.**
- `OrganismState::since_trip: u16`, beside `trip_src` (organism.rs:6224). Initialised to 0 (world.rs:7126), reset to 0 wherever `trip_load = true` (creature.rs:12497), incremented with `saturating_add(1)` beside `since_nest` (creature.rs:~6765).
- `DecisionScratch` and `DecisionRow` (creature.rs:631-832) gain these fields, filled only when `world.decision_log.is_some()`:
  - from the deposit block (6578-6677): `emit_a_laid` and `emit_b_laid` (the amounts actually deposited), `emit_b_brain`, and `deposit_at`;
  - from `chooser_step` (15726-15996): `b_near[8]` and `b_far[8]` (raw B at head+d and head+2d, the cells `trail_presence` reads), `b_six[8]` (via `trail_sample_point`, 11415), the options mask, `score[8]` and `k`.
- Heading order follows `DIRS` (creature.rs:79).

**0b. `examples/trailfollow.rs`.**
1. **BTRAIL fixes** (2730-2756):
   - `arm=` prints the real arm and `gate=` the gate; add `ch=A|B`, `kind=live|replay|cf`, `rule=`, `ft=`, `piles=`, `y=`;
   - one row per y over surface−3..=surface+1;
   - x widened from west of the door to past the pile;
   - `ATRAIL` rows, and `BSHAFT` column maxima down the shaft to the storeroom.
2. **`dwide`** appends the 0a columns to the decision CSV. It is flag-gated, so the default CSV stays byte-identical. Header at 4576, writer at 2641-2703.
3. **`shadow`**: a replay plane.
   - Clone `w.pheromones` at frame 0 (`Pheromones` is `Clone`, pheromone.rs:851).
   - Each frame, mirror the harness's own paints (`lay` 507, `lay_home` 356, `lay_flat` 398, refactored to take a sink), deposit the drained rows' `emit_*_laid` at `deposit_at`, and `step()` with the live interval.
   - At every sample, assert replay == live over every cell of A and B.
   - `shadowfault=n` drops every n-th deposit and must trip the assertion (watched red).
4. **`cf=<rules>`**: counterfactual planes on the same paths, emitted as `BTRAIL kind=cf` rows.
   - `brain`: must equal replay exactly.
   - `gate`: trip_load && crop>0.
   - `odoT` (T ∈ {16, 24, 32, 48}): gate × T/(T+since_trip).
   - `stepT`: a distance odometer.
   - `sqW_T`: a preview of the genome-side squash form.
5. **`gifoverlay=b|cf:<rule>`**: a harness post-pass after `renderer.draw` (~3148). Full-replace pixels at fixed log brightness `ln(1+v/256)/ln(1+4·DEPOSIT/256)`, with ants painted over them (`gifants`).
6. Put arm, ft and dtag in the ladencsv filename (4458). Echo `FOOD_TRAIL`, `shadow`, `cf` and `dwide` in the header (5011).

**0c. `examples/onetrail.rs` `mode=stream`**, the design-level expected profile. Generalise `timing_mode` (396-473):
- a stream of returning ants at a traced pace and interval, with `lay=const|odo t=`, `dwell=` (shuffling at the door) and `reverse=on`;
- the real `Pheromones::step`;
- output `BTRAIL kind=design` rows plus rising share, log slope, pile/door ratio, and the door read at reach 2 and 6.

**0d. Scripts.**
- **`scripts/btrailchart.py`**:
  - select by kind, rule, ch and y; key cardinality;
  - `--stats` gains the OLS slope of log(1+B) toward food, the rising share of adjacent lit pairs (≥ TRAIL_HALF/4), continuity at `--reach 2`, and the pile/door ratio;
  - `--expect` draws live | expected | diff panels on a fixed scale;
  - `--door` gives the E−W gradient at the door over time;
  - `--selftest` on a synthetic ramp, a nest peak and noise.
- **`scripts/trailclimb.py`** (new):
  - Recompute P(pick) from the logged scores. The check is that realised equals expected within 99%, per leg; this is the positive control that the trace matches the draw.
  - Climb readouts: share taking the highest-B option (realised against a no-follow counterfactual) and uphill-step share.
  - Split by leg (fed-driven, hungry, lunch), zone (west, door, road, pile, shaft) and arm, paired by seed.
  - A door-departure table: gradient at departure against the side taken and the age of the last return.
- **`scripts/trailpace.py`** (new): design-model parameters from the trace (trip age at first drop, laden speed, return interval).

**0e. Tests** in creature.rs, each watched red:
- `the_decision_trace_changes_nothing_it_watches` (27392) stays green, plus a vacuity check. Red: inject a deposit in the traced branch.
- `since_trip_counts_from_the_last_trip_pickup`, using the `trip_reach_bite_primed` helpers (28318/28325). Red: delete the reset.
- `a_replayed_plane_equals_the_live_one`. Red: skip laden rows.

**Gate G0 (registered in §23a before the cf readouts open):**
- G0.1: identity.
- G0.2: replay ≡ live for self, mute and hand at 90 and 140; `shadowfault` trips.
- G0.3: the new surface rows reproduce `b_profile` exactly.
- G0.4: trace self-consistency.
- G0.5: `cf=brain` ≡ replay.
- Predictions P0.4-P0.10:
  - today's self profile falls toward food (rising share < 0.5 on ≥ 7/8 seeds); hand rises (> 0.9, 8/8);
  - `cf=gate` cuts west-of-door laying by 90-97% and door B by ≥ 50%, but alone does not make it rise;
  - `cf=odo32` rises (share ≥ 0.6 on ≥ 6/8 at 90, ≥ 5/8 at 140);
  - the door E−W gradient exceeds 0.1 on ≥ 50% of fed departures at reach 2, ≥ 65% at reach 6, against ≤ 35% today;
  - door B falls with the age of the last return ("carries news"), against today's flat 0.99-1.00.

**T criterion (registered):** the largest T ∈ {16, 24, 32, 48} such that P0.6 holds at both gaps and a single deposit at the p90 trip age still clears `TRAIL_HALF` (1,024 raw). The prior is T = 32. Time-based wins unless `stepT` clearly beats it on the door gradient.

## Stage 1: LAY — PR-C, commit 1 (switch off by default)

**Switch.** `PIXEL_PHYSICS_FOOD_TRAIL`, parsed as `FoodTrail { lay, read, giveup, t, gain, reach, follow_all }`. It follows the `parse_forage_drive` parts template (creature.rs:14957-14979) and the `trip_reach` shape (15266-15291).
- Spellings: `""`/`off` = OFF (until Stage 6); `on` = lay, read, giveup; parts `lay`, `read`, `giveup`, `t=`, `gain=`, `reach=2|6`, `follow=all`.
- Anything unknown is reported and read as unset. `read` without `lay` is accepted only as a labelled diagnostic arm.
- Add `World::food_trail` (world.rs fields 3654-3741, init 6087-6104) and `food_trail_of(world)`.
- `off` reads nothing, writes nothing, and takes no draw.
- Echo it in trailfollow (5011) and labforage (1561).

**Rule** (creature.rs:6639). It multiplies the genome's EmitB, never replaces it, so the `mute` arm (trailfollow.rs:1255/1414, which zeroes the genome's EmitB) stays a control:

`emit_b = emit_b_brain × (lay ? [trip_load && crop>0] × T'/(T' + since_trip) : 1)`, with T' = T × `cell_scale`.

Consequences:
- The emit cost prices what is actually laid.
- A returner stops laying at its first drop, when `trip_load` clears (12726-12729).
- Lunch carriers, store holders and nest workers lay nothing.

**Tests:**
- `parse_food_trail_reads_its_spellings`, using template 28285.
- `only_a_trip_load_lays_food_scent_and_it_is_strongest_at_the_pickup`: rho and diffuse set to 0; a lunch carrier and a non-trip pickup lay nothing.
- `mute_stays_a_control_under_the_lay_rule`. Red: replace instead of multiply.
- `food_trail_off_is_the_ant_before_it`, using template 28795.

**Predictions:**
- P1.1: the live profile matches `cf=odoT` (r ≥ 0.8 on ≥ 6/8).
- P1.2: laying west of the door falls ≥ 90%.
- P1.3: door B at fed departures falls from 0.99 to ≤ 0.8 and tracks return age.
- P1.4: the east share of departures is unchanged (lay alone gives no bearing).
- P1.5: given-up west ants are home in ≤ 800 frames (today 1,392).
- P1.6: food taken beats off on ≥ 16/24 and ties mute.
- P1.7: emit energy falls ≤ 50%.

**Gate:** P1.1-P1.3 hold, with no harm against off. If P1.1 fails, stop and trace.

## Stage 2: READ — PR-C, commit 2

**Additive term** in the chooser score (15906-15924). It applies when `read && reads_trail && !laden && no pull && no spoil && !nest-bound`, and it must work at the door, where `away_from` is None.
- `b(d) = max(B[head+d], B[head+2d])` as raw values; never "here" (Z29). With reach=6, also B at `trail_sample_point` gated by `trail_could_be_here`.
- Level options only (vertical headings get no term: the shaft, and the level-cosine lesson).
- `grad(d) = (b(d) − mean_{o≠d} b(o)) / (b(d) + mean + TRAIL_HALF)`.
- `follow(d) = FOLLOW_GAIN(3) × want × fpat × grad(d)`, where `want = max(hunger, forage_drive_level)` (the owner ruled: hungry or driven ants only) and `fpat = 1` until Stage 3.
- It is additive because `TURN_PREF[4] = 0`. Worked example at the door, facing west, with B 5000 to the east against 1000: P(east) ≈ 0.81 per decision.
- `b[8]` is computed once per decision, and only when read is on.

**Tests:**
- `at_the_door_an_empty_driven_ant_turns_toward_the_stronger_food_trail`, from template 27905, anchor reset each frame, plus a mirror scene. Red: gain 0.
- `the_reader_climbs_a_ramp_either_way_round`.
- `a_fed_resting_ant_is_not_recruited`. Red: want = 1.
- `the_reader_changes_nothing_on_a_muted_plane` (byte-identical).
- A unit test of `grad`.

**Predictions:**
- P2.1: at the door with |grad| ≥ 0.2, picks of the higher-B heading run ≥ 0.25 above the no-follow expectation.
- P2.2: the fed east share rises to ≥ 0.80.
- P2.3: fed departures reaching the pile rise to ≥ 0.60.
- P2.4: lay+read beats mute on taken (≥ 15/24) at 90, and is not lower at 140.
- P2.5: entries into the shaft rise ≤ 10%.
- P2.6: `read` without lay is harmful (the negative control).
- P2.7: mute + full ≡ mute.

**Gate:** P2.1, P2.2 and P2.7 hold, with no harm to starvation.

## Stage 3: GIVE UP — PR-C, commit 3

Under `giveup`:
- The away term (15916) and the follow term are multiplied by `scout_patience`, and set to 0 once `scout_home` is set.
- The given-up home pull (15920) drops its `(1 − route(d))` factor.
- The patience update (15958-15971) is unchanged.
- A separate trail patience for fed, non-driven ants is added only if the trace shows them stuck on B.

**Tests:**
- `a_follower_on_a_dead_end_trail_gives_up_and_comes_home`, from template 27967. Red: remove fpat.
- `after_giving_up_a_trail_does_not_turn_an_ant_back_out`. Red: restore `(1 − route)`.

**Predictions:**
- P3.1: given-up ants reach home within 1.2 × mute's time.
- P3.2: their outward step share is ≤ 0.15.
- P3.3: wall deaths do not rise.
- P3.4: the full recipe is not lower than lay+read.

## Stage 4: the two-pile alternating bed — PR-B (trailfollow and scripts only; after PR-A)

**Arguments:**
- `pile2=west alt=<frames>`, default 6000;
- `altclear=on` (the owner's choice): at a swap the old pile's cells are removed and booked as `withdrawn`;
- `start=seed`: odd seeds start east, even west, to balance the 0.67-0.70 east bias.

**Geometry:** `nest_x = max(half_band, gap+60)`, piles at `nest_x ± gap`, walls 60 cells past each pile. The founding guard (1792-1803) is extended.

**Single-pile sites that become lists:**
- pile slots 1949, `place_food` 1962-1980, refill 2707-2709;
- `at_food` 3867, `first_arrival` 3891, funnel 3928-3957, bands 3888/4008, `near_ticks` 4015;
- `b_prof` 2711 and BTRAIL;
- `scripts/antloop.py:71-72` and `antidle.py:22`, which read the piles and the schedule from the header echo.

**Arms:**
- Refuse `hand`, `hmute`, `homeA`, `flatN` and `flatF` with `pile2`: they paint nest → east only, and a painted rise flatters a climber.
- Add `oracle`: a ramp rising toward the currently stocked pile, repainted at each swap. It is a labelled positive control that the bed can reward a trail, never a shipping comparison.

**Readouts:** a SWAP row per swap giving D1 (first trip pickup at the new pile), T50 (recovery to half the previous phase's rate), stale visits, and last stale visit. Defaults: gap 90, `alt=6000`, `frames=36000`.

**Gate:**
- P4.1: identity with `pile2` unset.
- P4.2: `oracle` beats mute on taken (≥ 18/24). If it does not, fix the bed, not the trail.
- P4.3: per-side balance within 20%.

## Stage 5: measurement and the decision rule

**Arms:**

| Arm | Setting |
|---|---|
| off | unset |
| M | mute, the bar |
| L | lay |
| LG | lay + giveup |
| LR | lay + read |
| F | lay + read + giveup |
| D | read without lay, diagnostic, 8 seeds |
| I | M + F, 4 seeds, must be ≡ M |

**Beds:**

| Bed | Setting |
|---|---|
| B1 | gap 90 |
| B2 | gap 140 |
| B3 | 80 founders |
| B4 | pulsed pile (`food=30 refill=6000`) at 90 and 140 |
| B5 | two-pile, plus `oracle` |
| B6 | lab box (`labforage scenario=played_bed`, 24 seeds, rain, repo root, no bed env), read with `scripts/labpair.py`; winner against off only, via a cloud lane (`lab-coordinator` skill, model set explicitly) |

**Traced subsets:** `decisioncsv dwide shadow cf=odoT`, 8 seeds, on B1, B2 and B5 for off, M, L, LR and F.

**Reading order:**
1. Lead with food taken.
2. Then born, starved, nest food, bodies.
3. B5: per-phase taken, D1, T50, stale visits.
4. Door: east share and reach.
5. Funnel: `scripts/antloop.py --vs`.
6. Five traced individuals per arm: turned at the door, went west, gave up.

**B5 predictions (F against M):**
- P5.1: D1 not faster, median ≤ +500 frames.
- P5.2: T50 shorter on ≥ 15/24.
- P5.3: taken higher on ≥ 15/24.
- P5.4: last stale visit ≤ 1,500 frames after the swap in ≥ 75% of swaps.

**Decision rule (owner-approved: ship if better than today without harm):** set `FOOD_TRAIL=on` (F) as the default if all of these hold:
- (i) F beats off on taken at 90 (≥ 16/24), is not lower at 140, and starved is not higher (p < 0.1);
- (iii) on B5, `oracle` beats M, and F beats M on taken or T50 (≥ 15/24);
- (iv) B3 and B4 are not worse than off at p < 0.05;
- (v) no gate in the B6 lab pair is worse than off at p < 0.05.

On (ii) — F against M on B1 and B2:
- If F ties or beats M, report that.
- If F loses to M, ship anyway (the owner's ruling), and file the gap as the next problem.

If F fails and a subset (L or LG) passes, ship the subset and trace the failing part. If nothing beats off, do not ship: keep the instruments, and write dead-end entries with the numbers.

**Compute.** Run locally from one frozen binary: a jobs file through `xargs -P 4 -L 1`, `RAYON_NUM_THREADS=1`, traced jobs first.
- The untraced matrix takes under about 1 hour; the traced subsets about 40 minutes.
- Analysis agents are read-only (profile against expected, door and climb, funnel and individuals, two-pile phases), followed by 2-3 refuters per headline claim.
- Use a Workflow for the analysis fan-out (ultracode is on). Run `scripts/agentmeter.py` afterwards.

## Stage 6: ship, docs, follow-up — PR-D, then PR-E

**Flip the default** if the rule passes. Identity check: `FOOD_TRAIL=off` ≡ the pre-flip main.

**Docs, updated in the same PR:**
- `Reports/how-the-ant-works.md`:
  - §6d: the follow term and patience;
  - §7: the lay rule, and fix the stale "only the forward difference, only into Move" at 661-664;
  - §10: B is laid only on a trip load;
  - §12: the switch;
  - §15: the new columns;
  - bump the "Verified against" line.
- Stale comments:
  - `ant.ron` 1370-1387 and 2135-2144;
  - organism.rs 6211-6219 (`trip_load` now has a second reader);
  - brain.rs 977;
  - the `AWAY_GAIN` doc (creature.rs:14803-14809): read needs lay.
- `wiki/ants.md` (with its freshness note), README Forage status, the lane note.
- `dead-ends.md` write-backs at 1954 (odometer re-test result) and 1192 (give-up now applied on a trail).
- `python3 scripts/deadendindex.py --touching`, then `bash scripts/docscheck.sh`.

**Review card:**
- a GIF pair of the door with `gifoverlay=b gifants=1`, off against F, same seed;
- live / expected / diff heatmaps;
- one traced ant.

**PR-E (owner-chosen follow-up):** move the lay rule into the genome.
- Add `BrainInput::TripFresh`, sensed as `trip_load && crop>0 ? T/(T+since_trip) : 0`, and wire `(TripFresh, EmitB, w)` in place of `(CarryingFood, EmitB, 2.5)`, with w from the `cf sqW_T` plane.
- Re-pin: live slots 942 → 966 (brain.rs:2459-2528), the manifest pin, and `mutation_rate` in all six species files.
- Equivalence check: the live profile and a B1 pair against the engine-side run.

## PR sequence

1. **PR-A:** instruments (Stage 0) and §23a.
2. **PR-B:** the two-pile bed (Stage 4) and §23b.
3. **PR-C:** the switch, off. Stages 1-3 as three commits, §23c-e. Land lay alone first if the branch drifts (branchcheck BxF > 300).
4. **PR-D:** the default flip and docs.
5. **PR-E:** genome-side.

Each PR: full `cargo test --release`, `cargo clippy --all-targets --release --locked -- -D warnings`, docscheck, the identity check, a squash merge on green, and restart the branch from main.

## Before starting: answer the nest lane

Their poke of 20:58 goes in the lane note (`Reports/lanes/foraging-loop.md`) plus a one-shot poke.
1. **STACK_DEPTH default 4:** no objection. The bed already runs 4, so the bed baseline does not move; the lab and game baselines do. Their conditions (a 24-seed lab pair with `labpair.py`, and a bed pair) are right. The lane's "one bed pair at `STACK_DEPTH=1`" rule becomes moot once the game default is 4.
2. **The carry (SPOIL_RING under SPOIL_OUT, off):** nothing needed.
3. **Resting ants living inside the nest:**
   - No objection in principle; make it a joint switch measured on this bed (taken, starved, fed departures).
   - It interacts with this plan's door reader, because ants would leave from deeper inside, and with the anchor re-anchoring on nest contact.
   - Ask them to keep `trip_load` and door geometry semantics, and to poke before it lands.

## Risks, and the instrument that catches each

- **The door stays unreadable** (5 columns wide, delivery lands in it): P0.7/P0.8 on the cf plane before any reader exists; `reach=6` as a fallback.
- **Traffic or door dwell beats the timing gradient:** P0.5/P0.6, the design model's `dwell`, and `stepT` as a comparison.
- **Leash to a stale trail:** D1 and stale visits in B5, and the give-up stage.
- **Recruiting resting ants:** gated by `want`; readout is time out for fed, non-driven ants.
- **The shaft pulls followers:** no term on vertical headings; BSHAFT; P2.5.
- **Mute stops being a control:** the multiply rule, its test, and M+F ≡ M.
- **The lay gate leaks with `TRIP_REACH=off`:** documented, and tests run with the reach on.
- **Emit-cost confound:** `emit_energy` reported per arm.
- **cf is open-loop:** used for lay only; read is judged against the design model.
- **Keep B's decay:** rho 0 cut reach to a third, so `brho` is not touched.

## Verification (end to end)

- **Per stage:**
  - the identity `cmp` on logs and CSVs;
  - replay ≡ live, with `shadowfault` tripping;
  - the trace self-consistency check in `trailclimb.py`;
  - every new test watched red then green;
  - full `cargo test --release`, the clippy line, the `ascii` worst frame, docscheck.
- **Stage 5:** the registered predictions scored in §23; paired 24-seed tables per bed; funnel and five traced individuals per arm; the lab pair with `labpair.py`; a review card for the owner (a door GIF with the B overlay, off against F, and the live/expected/diff heatmaps).

## Critical files

- `src/sim/creature.rs`:
  - deposit 6578-6677;
  - trace 631-832 and 6700-6765;
  - trip pickup and booking 12455-12499 and 12726-12729;
  - `trail_presence` 15628; `chooser_step` 15726-15996;
  - switch templates 14957-14979 and 15266-15291;
  - tests 27392, 27905, 27967, 28285, 28318, 28795.
- `src/sim/organism.rs` 6211-6256.
- `src/sim/world.rs` 3654-3741, 6087-6104, 7106-7133.
- `examples/trailfollow.rs`:
  - `run` 1353; mute 1255/1414; nest 1763-1803; piles 1949-1980;
  - loop, drain and BTRAIL 2577-2756;
  - single-pile sites 3867-4015; header 4576; echo 5011; arms 5181.
- `examples/onetrail.rs` 396-503.
- `scripts/btrailchart.py`; new `scripts/trailclimb.py` and `scripts/trailpace.py`.
- Reused: `pheromone.rs` public API (`Pheromones::new`/`clone`/`deposit`/`step`, `set_channel_rho`/`diffuse`); `trail_sample_point`/`trail_could_be_here` (creature.rs:11415); the `trip_reach_bite_primed` test helpers; `scripts/antloop.py`, `labpair.py`, `drivefade.py`; the `/home/user/runs/trailq/` departure scripts (port what is needed into `trailclimb.py`).
