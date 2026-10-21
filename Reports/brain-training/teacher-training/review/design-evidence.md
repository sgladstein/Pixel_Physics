# Review of `teacher-design-2026-10-21.md`: evidence and feasibility

*Adversarial reviewer, 2026-10-21. Branch `ccr-439ee31b-pn36yq` at 6aa3b4b4; scratch code in `/home/user/tp` (920daf21).
Every step-1 number was recomputed from the CSVs with `measurements/arms.py`. Sense claims were checked against the
writers in `options.rs` / `mind.rs`, against which links are live in the genomes, and against the solo copy log
`/home/user/c0/check.log` (s161 as loaded, 48 worlds, 112,130 non-settle decisions). Scripts:
`scratchpad/an/{feat,f2,links}.py`. No repo file was edited.*

## Verdict

**Every step-1 number is quoted correctly. The parts of the plan that are new rest on three wrong readings of the
code:**

1. **The soil planner's trial drop measures something the arena does not score.** The arena counts a full load where
   the pellet is *put down*, not where it ends up.
2. **Spoil does not slide.** So the trial drop would return the same value for every candidate.
3. **On the current 8-row step, s161 already carries 88% of its full-depth cuts out as full loads.** "Where the soil
   goes" can add at most about 0.8 full loads a world. Stage 1 cannot show a gain that small.

The nest planner's sense table overstates `HEADROOM` and `ROOM`/`CROWD`. Its progress tools are misnamed and partly
blind to the nest shapes it proposes.

## Claims checked

| # | Claim (doc §) | Source | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Teacher on s161 20.66 vs 6.8, "three times" (Answer) | step1-review table | correct | `ceiling-v2/dig_down_home.csv` 20.66±0.37; `none.csv` 6.80±0.39 (ratio 3.04) |
| 2 | Go down copy 6.42 → 10.57, +4.16±0.48 (Answer, §2) | step1-results | correct | `step1-runs/s161q.csv`, `only-down-soil.csv` via arms.py |
| 3 | Dig copied alone 3.35, 15 seeds under 3 (§2) | step1-results | correct (number) | `only-dig-soil.csv`: 3.35±0.19, 15/48 seeds (33/96 worlds) |
| 4 | "The ceiling test showed the [dig–descent] coupling first" (§2) | measurements §3 | **wrong** | In the ceiling, dig alone is **11.96 (+5.16), 1 seed under 3** (`ceiling-v2/dig.csv`). What stranded was dig + home without down. The ceiling shows dig and down are superadditive (16.94), not that dig alone collapses |
| 5 | "+4.7-load lesson someone already wrote" (§2) | measurements §3 | stale | +4.74 is the pre-memory-fix arm; memory-consistent is +5.16 |
| 6 | Probe passed all three rules at 94-100% held out (§3, §5.4) | measurements §2 | **misleading** | The three copied rules are D1b 0.999, D3 0.995 and D4 1.000. The 0.938 is D2 (put-down), which was not copied. D1b's accuracy sits on a 0.971 floor. Through the brain's own channel D1b reads 0.65/0.87 or 0.83/0.97 recall/precision; step1-review finding 12 required quoting that number |
| 7 | 706 → 522 live links when quieted | step1-results | correct | `down-s2.out` line 1; 727/543 nonzero including 21 header slots |
| 8 | s171 9.47, same ceiling, better kept skills (§7) | step1-results | correct | fetch 19.64 vs 17.11, trail 0.796 vs 0.611 |
| 9 | Go down "kept skills held" (§2) | step1-results | correct, loosely | fetch 17.11 vs 16.66; trail 0.611 vs 0.640 (−4.5%) |
| 10 | Revived links: slot 1498 and `DEPTH` 654…1239 | step1-results | correct | Layout: S_FEAT=583, so 654 = unit 1 + f::DEPTH(26). 1498 = JOB+34 (Walk's best-score read). The copy has 8 live f::DEPTH links, s161 has 0 (links.py) |
| 11 | 22 senses, 12 memories, 45 option features; 16 scorer and 8 hidden units (§3, §7) | mind.rs:61-68,126; options.rs:215 | correct | Only **8 of 16** scorer units are live in s161 (output weight ≠ 0), and 6 of 8 hidden units are read |
| 12 | `SPOIL` = placed spoil apart from native soil, `LINED` = lining (§3) | options.rs:144-148, 204-206, 538-541 | correct (option feature) | Written in `describe`. `c::SPOIL` (context) is a different sense: "carrying a pellet" (mind.rs:78, 1430) |
| 13 | `CUT` = the hole | options.rs:166-171, 550; `in_the_hole` :508 | correct | Written and read (4 live links) |
| 14 | `ROOF_DROP`, `ROOF_CUT` sense a roof-dropping cut | options.rs:415; creature.rs:22856; mind.rs:1412 | correct, narrower than stated | Covers the crumb and footing rules on **self-supporting** neighbours one cut ahead. It does not see loose soil sliding by repose (the lab's "loose rim" source, dead-ends:1222). Non-zero on 14,236 Dig options in the solo log |
| 15 | `HEADROOM` = "headroom in body heights", the body-size roof template (§3, §5.4) | options.rs:181, 582-585, 316-317 | **wrong as used** | It is the open cells above the *option's target*, up to 6, divided by 6. On Dig options: 0 for 71%, 1.0 (open sky) for 23%, an intermediate value for 6.5% (feat.py). For a wall or roof cut, the cell above the target is soil, so it reads 0 whatever the chamber's height. No context sense gives headroom over the ant: `ROOFED` is binary, any ground within 20 rows (creature.rs:12369) |
| 16 | `DEPTH`, `HOME`, `HOME_DIST`, `UP` | options.rs:576-577, 636; mind.rs:1440, 1465 | correct | `DEPTH` = squash(rows/16): 64 rows reads 0.80 and 48 rows 0.75, so spacing at depth is compressed |
| 17 | `CROWD` reads 0 solo (§6.3); `CROWD`/`ROOM` only in a crowd (§5.4) | mind.rs:1452-1464; options.rs:610-626 | **half right** | `c::CROWD` is 0 on all 112,130 solo decisions. **`f::CROWD` is never 0**: it reads 0.125 or 0.25, the ant's own body inside the target's 3×3. **`f::ROOM` takes 32 distinct values solo**: it measures openness (open cells over 24, divided by 1 + bodies) |
| 18 | `LAST_FOOD`, `BUILDING` reserved and missing | mind.rs:818 `UNWRITTEN`; options.rs:160, 198 | correct | Never non-zero in the log. But s161 still carries **3 live links on f::BUILDING**: billed every tick, reading 0 |
| 19 | `STAND`/`CLIMB` for walkable paths | options.rs:385-387, 562 | correct | Written per Walk option |
| 20 | `rooms_of`: room cell = 7 of 9 open, 4-connected rooms, steps between rooms | nestcensus.rs:195-314, 64-66 | correct, with limits | Steps are counted from the biggest room only, capped at 60 (beyond that a room reads 999, "unreached"); rooms under 30 cells are dropped; animal cells count as open. The pure function works on any grid. `nest_row` needs a `LabBox`, which the soil arena has (`founding_cut`, challenges.rs) |
| 21 | `World::in_room`, `in_chamber`, `room_per_ant` (§5.2) | world.rs:1438, 1478, 1623 | **misattributed** | These are `SideRoom::in_room` and `ShaftFootprint::in_chamber`, rectangle tests of the *founding* cut, which cannot measure dug progress. `room_per_ant` is `NestRoom`'s (roofed void per ant, kept by `step_nest_room` only with `room_gate` and nest sites, world.rs:9338) |
| 22 | World is Clone | world.rs:3930 | correct | `#[derive(Clone)] pub struct World` |
| 23 | Cells can be stepped without minds | update.rs:73; challenges.rs:26-33; world.rs:7585 | correct | `update::step(w)` alone runs the cell sweep (plus weather, springs, soil water). Creatures run in `step_active_sites`. The teacher hook takes `&World` (tp mind.rs:3663), so a clone is available there |
| 24 | ~58 µs a frame | search design §3 | unverifiable | Derived from "0.7 CPU-s a world", with no log cited. It includes minds. Clone cost is unmeasured (the search doc says so), so "~20 ms upper bound" is not an upper bound |
| 25 | Loaded decisions ≈ a third of ~2,700; ~3 drop options a decision (§4.3) | — | wrong, and harmless | 2,336 non-settle decisions a world (2,700 counts the 381 settle decisions, which are never labelled); **42% loaded**; **0.41 Drop options per loaded decision** (19,550 over 47,600). The cost is about 7× lower than estimated |
| 26 | S3: a full load is put down ≥3 Chebyshev from every hole cell at the bell, **after a 2,000-frame settle** (§4.1) | challenges.rs:1437-1440, 1660-1690, 1752-1755, 1967-1990 | **wrong in the part that matters** | `full_loads` is decided from the **put-down coordinates** (`dump_log`) against the hole as of the bell, and also needs the cut ≥ shaft depth and a lift ≥ shaft rows. The settle changes only `refill_pellet`/`refill_fall`, which cost −4 each in `score`, never `full_loads` |
| 27 | The grains rest at ~45° (1:1 slide); inner toe at r_c − h (§4.2, §5.1) | spoil.ron:59-79; update.rs:827-848, 1113-1285 | **wrong** | A placed pellet is `spoil`: `self_supporting`, `needs_footing`. It never slides while it has footing (`return wet_changed`). Put-down requires footing on 2 of the 3 cells below (creature.rs:18171-18186), so a 2-wide column can grow straight up. Bed soil is 33°, not 45° |
| 28 | Dead-ends record: SPOIL_RING/CREST/DOOR_COLLAR, MOUND_DIG, SOLID_MOUND, FORAGER_NODIG, NEST_HOME=mound, 268-275 → 134-198 (§4.4) | dead-ends.md:1199, 1222, 1233, 1234 | correct | The re-test condition is quoted correctly |
| 29 | Fill by cause "pellet slid, rim fell, put back" is already printed (§4.4) | challenges.rs:1712-1726 | roughly | `refill_pellet` (put back) and `refill_fall`. `fall_putdown` is a proximity count (a moved pellet within 2 cells), not a trace |
| 30 | 2,400 J budget (§8) | measurements header | correct for these runs | The code default `SOIL_BUDGET` is 1,200 (challenges.rs:1455); the runs pass `budget=2400` |
| 31 | Step 0 "go-down copy's seed-to-seed spread" not done (§10) | `/home/user/seeds/e{1,2,3}.csv` (05:21, one minute after the doc's commit) | now done | Copier seeds give 10.57 / 10.60 / **9.80** (+4.16 / +4.19 / +3.39). The copy is not luck, and the spread is about 0.8 |

## Findings

### BLOCKER 1. The soil planner's trial drop has no effect on the score it is judged by

§4.3 values a drop as "worth nothing if it slides inside the zone or into the hole, one full load if it comes to rest
on the plan". The arena does not work that way. `soil_trip_at` (tp challenges.rs:1660-1690) credits the full load at
the put-down cell (`away(at)` on the `dump_log` coordinates). A pellet that later slides into the hole keeps its full
load and costs −4 of `score` through `refill_*`.

Stage 1 of the gauntlet (§9) passes on full loads. So the planner's one piece of "privileged computation at its
plainest" cannot move the number that gates it. The only privileged fact that does bear on S3 is **future cuts**: a
pellet put down 3 cells from today's hole is scored against the hole at the bell. That fact comes from the nest plan,
not from physics.

**Fix:** value a put-down by S3 eligibility against the *planned* hole: today's hole plus the dig plan's cells. That is
a pure geometric test with no clone needed. Value slides in the units `score` actually charges (refill, −4 per hole
cell), as a separate term. Or, if the owner wants "where the soil ends up" to count, change S3 to read the pellet's
resting cell after the settle. That is a scorer change for the ladder lane, with its own control.

### BLOCKER 2. On the 8-row step, the soil planner cannot show a gain

From the solo log's `E` rows, s161 makes 7.10 full cuts a world and carries 6.27 of them out as full loads (88%). Of
all 1,042 put-downs, 86% are already "away".

A perfect soil planner acting alone on s161 can therefore add at most about **+0.83 full loads a world**. A seed-
clustered difference here has s.e. ~0.45-0.55 (arms.py on these arms), so the §9 bar ("> 2 s.e. above the brain") is
out of reach even for a perfect planner. The limit at 8 rows is full cuts, which is dig/down's domain. The doc's own
Answer says as much ("what limits us is copying").

**Fix:** run stage 1 for the soil planner on top of the dig + down teacher, or on the go-down copy, where full cuts
rise (16.94 with dig + down), and read its share of full cuts carried out. Or score it on `refill_*` and `score`.
Either way, print full_cuts, full_loads and their ratio for the brain first. If the soil planner stays first in the
order (§10), say that it is first for the owner's mound picture, not for full loads.

### MAJOR 3. The trial drop would read the same value for every option

Because of claim 27: a pellet put down with footing is stable spoil. Within a 300-frame cells-only trial it moves only
if something undermines it, and in a clone with frozen minds nothing digs. So the trial would value nearly every Drop
option identically. Under the regularised target, a constant term gives every option the same advantage, so it
abstains everywhere. The term would sit on top of whatever the carry path does and contribute nothing, while costing a
clone per option.

This is CLAUDE.md's "ask what your number counts when nothing is wrong". The 45° ring geometry in §4.2, and the
renders' "flat ring about 8 cells tall reaching 34-86 cells", inherit the same wrong angle.

**Fix:** before building it, count from existing runs the pellets that moved (the `rolled` set inside `soil_trip_at`,
which is never printed) and what moved them. Add a positive control: a put-down on a cell the plan will undermine must
read as sliding. Redo the ring geometry with spoil's real rule (it stands; the 2-of-3 footing spreads it) and soil's
33°.

### MAJOR 4. The "Yes" rows of the §5.4 sense table are not supported

- **"Widen sideways, not up → `HEADROOM`, `ROOFED`: Yes."** The roof height over the ant is not on any Dig option
  (claim 15), and `ROOFED` is a binary 20-row look. The only reading of roof height is `HEADROOM` on *Walk* options.
  The scorer scores each option alone, and the job stage sees one best score per verb. So no path inside the brain
  combines a Walk option's headroom with a Dig option's direction (`UP`). The wide probe (mean/max over options)
  would pass this; the brain's channel likely would not. That is measurement 2's D1b trap again (wide 0.98 recall,
  narrow 0.65-0.83).
- **"Stop widening → only in a crowd (`CROWD`, `ROOM`)."** `f::ROOM` varies solo (32 values) and `f::CROWD` reads the
  ant's own body (claim 17). So the "taught solo vs crowd" boundary in §5.4/§6.3 is not where the doc puts it, and
  §6.3's protect list ("`CROWD`, `SELF`, `AGE`") misses that `AGE` is a run clock solo, free for selection to give a
  solo meaning.
- **"Dig down here → probe 94-100%".** See claim 6.

**Fix:** make the copyability gate (§3, §9 stage 2) the **narrow-channel probe**: features reach the job stage only
through each verb's best score, as in measurement 2's second table. Keep the wide probe as the upper bound. Mark
chamber-roof height as "missing: a context headroom-over-head sense" (way out 4). Khuong's template is exactly that
local sense.

### MAJOR 5. The nest progress tools cannot show the nest the planner builds

- `World::in_room`/`in_chamber` are rectangle tests of the founding cut (claim 21). Drop them.
- `rooms_of` joins chambers through any passage 3 or more wide (inside a 4-wide tube every cell has 9 of 9 open; the
  test `a_hole_with_no_waist_is_one_room` shows the effect). So the owner's 4-cell blueprint passages read as **one
  room**. Rooms more than 60 steps from the biggest read 999 ("unreached"). That is a cap producing an answer, which
  CLAUDE.md rules a bug. A 64-row box can exceed it.
- "Blueprint match, roofed void only" credits **zero** for the shaft, which is open to the sky by definition, but the
  dig order's first priority is "the shaft before the chamber". At frame 0 the match is already positive (the
  founding 2×8 shaft and 9×2 chamber, `check.log` G row) unless the frame-0 value is subtracted. Ant bodies count as
  open cells in `nest_row`.

**Fix:** score progress as frame-0-subtracted open cells inside the blueprint (shaft included) minus those outside,
below the datum. Report chambers by matching blueprint chambers, not with `rooms_of`. Add a positive control (carve
the blueprint by hand: the match must read 1) and a negative one (the founding world: it must read 0).

### MAJOR 6. "Dig: never copy it alone" rests on a misread of the ceiling

Claim 4. In the ceiling, the rule acting alone gains +5.16. The *copy* of it collapses (3.35), and step1-results lists
"the dig rule's failure traced to its cause" as not yet run. So the coupling the doc cites is a property of the copy,
with no traced mechanism, not of the rule.

Relatedly, the go-down copy (+4.16 on the quieted baseline) gains more than the go-down rule acting alone (+3.41,
v1 ceiling, unquieted baseline; not strictly comparable). The GIF caption says the copy's cuts concentrated at the
foot. The 8 revived `DEPTH` links feed every verb's scorer. So "go down copies cleanly" may include an unlabelled dig
lesson.

**Fix:** trace the dig copy's collapse (the funnel skill) before deciding the order in §2. Report the go-down copy's
per-rule agreement on its own runs: does its foot-dig rate move?

### MAJOR 7. §7's capacity lever is mislabelled as a brain change

s161 uses 8 of its 16 scorer units. The other 8 have output weight 0, and the copier zeroes their inputs and does not
revive them. Spare capacity exists inside the current genome. It costs energy per link revived (step1-review finding
2: fully live, the ant runs dry near frame 9,300), but it needs no brain-lane change. Listing capacity last, as a brain
change, skips a cheaper test.

**Fix:** add "revive one dead scorer unit for the lesson, with energy at the bell reported" before "capacity" in the
§7 table.

### MINOR

- **The shuffled-plan control in stage 1 (§9) cannot fail cleanly.** A shuffled ring still pays a full load at any
  away site. Use the step-1 drop rule (D2, "nearest clear site", −0.97 alone) as the control the planner must beat.
- **Cost (§4.3, §10).** Clone cost is unmeasured, so "upper bound ~20 ms" is not one. The decision counts are
  overstated (claim 25). "A trial drop per decision is milliseconds" in §10 contradicts the doc's own 20 ms × 3. Bench
  the clone first, as the search doc already says.
- **§6.3's "anything that never matters is pruned".** `zero_dead` (mind.rs:827-842) prunes unread units and unwritten
  inputs, not constant inputs. Its own text leaves "inputs constant only in today's arenas" alone. And s161 still
  carries 3 live f::BUILDING links: billed every tick, reading 0.
- **§4.4 "the brain already senses [rim falls]".** True for the soil arena's crumb-rule shoulder. Not for loose-soil
  slides, the lab's dominant source.
- **§4.4's solo label "mound cuts: no"** is the `MOUND_DIG` rule that starved colonies (268-275 → 134-198). If it
  copies solo, it will spread into the colony tier the way walk-home spread to foragers. Abstain on mound cuts solo
  instead of labelling them "no".
- **Body scale.** The ant is `Chain(2)` (ant.ron:27) and the doc's 2-cell body length is right. `REACH`'s comment
  ("two body heights of a three-cell ant", options.rs:316) is stale. `HEADROOM` is in sixths of 6 cells, not body
  heights.
- **Step 0 (§10)** can be marked done for the seed spread: 10.57 / 10.60 / 9.80.
