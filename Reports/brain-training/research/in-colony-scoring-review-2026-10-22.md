# In-colony scoring review: what the ant score pays, and what selecting colonies does

*Written 2026-10-22 by the "In-colony scoring review" lane, at Scotttt's request. Status: code read plus two small local
measurements (12 colonies each, 4 cores), not a 64-seed batch. Live copy, kept in step with this file on the day:
<https://claude.ai/artifact/S2h242Q6ebWYKjywSEBsUt>. Handed to the Teacher lane, which owns the ant and the harness.*

**The answer.** The in-run score pays single ants for acts. The two things the owner wants, a large multi-chamber nest
dug all game and a colony that never runs dry, are colony results that no single ant owns, so no setting of the per-ant
weights can buy them. That fits the record: about a dozen score-weight arms, none adopted. Measured in one 12-colony
round, what picks parents is 87% food, 10% meals at home and 3% digging, and the three need-driven prices barely move.
The recommendation is to stop tuning the per-ant ledger and add a second level of selection, comparing the colonies of
a batch on what each achieved and refounding the weak ones from the strong ones' brains. A 12-colony test of that
tripled food brought home and cut starvation by two thirds against today's training, **but digging went to zero**,
because the colony score used let a colony that never digs win. Digging has to be a gate on the colony score, not a
term in a sum.

**Code read:** `examples/inrun.rs` on the compute branch `claude/project-thread-8y5ak0` at ee38ae5b (never for `main`),
standard hA1 job arguments (`scratch/inrun/jobs/ha1next.txt`, arm hA1c). **Not repeated here:** the literature on in-run
selection (`in-run-selection-method-2026-10-21.md`, beside this file), and the Fable review of the training approach
(<https://claude.ai/artifact/AYMLfoEd4jvMiJK2g2RgG9>; hand-off in the project files at
`colony-training-review/for-teacher-lane-2026-10-22.md`).

## 1. What the score pays today

Per event, one food cell brought home is worth about five to ten cuts of new ground.

| Paid act | Points per event | Paid to | What moves the price (measured) |
| --- | --- | --- | --- |
| Food cell put down within 12 columns of the door | 7 x tank price (1 to 5) x depth (1 to 3); half paid on the way home | the carrier | tank price rises as banks drain; measured 1.0 almost all round |
| Cut of original ground within 20 columns | 0.5 x dig price (1 to 3) | the cutter | meant to fall as open cells per ant pass 25; measured 2.9 early, 2.1 at the end |
| Spoil let go above the ground line | clear price (1 to 3) x distance ramp (nothing inside 15 columns, full at 25) | the carrier, plus 0.5 x dig price to the original cutter | rises with loose spoil in the nest; at its cap of 3 from 75,000 frames on |
| Meal at home after a stretch of work | 2 | the eater | none |
| Leaving home while below 2 tanks | 10, at most once per 3,000 frames | the leaver | none |
| Food bitten in the field | minus 0.05 | the eater | none |

**How parents are picked.** Each ant's points per 1,000 frames of life are compared with the adults' median and divided
by a noise estimate from the ant's own squared points (the z score). Every adult is replaced about once per 30,000
frames; only ants above the median breed, the best get up to three children, no founder line may pass a quarter of the
colony, and a parent retires after four children. A child is a mutated copy on three full tanks. Ants starved in the
last 30,000 frames may still be picked at half score when the colony is short. The top 30 by score found the next
200,000-frame round in a fresh world.

**Never paid:** the general nest score (`lab::nestscore`, printed to `nest.csv` every 25,000 frames), food left in the
store, ants alive, and whether anyone starved.

## 2. What is wrong with it

1. **It selects single ants for colony results.** A dug room and a full store help every ant; the ledger can only pay
   the act, to whoever did it. Real workers never breed; the colony is what is selected. The record fits: dig weight x2
   and x4 only delayed the loss of digging (digw-1022), room credit paid almost nothing because the cutters were dead
   before the room existed, and starved-never-parent, hungry pay x3 and the tank price each left starvation where it
   was (starve-1022: 5.6 and 6.5 starved of 24 against 4.9).
2. **In practice the ledger is a food score.** Over 1,302 births in 12 colonies (one standard round, hA1), what
   separated parents from the other adults (parent's rate minus the adults' mean, by part) was 87% food delivered,
   10% meals at home, 1% hungry departures and 3% everything to do with digging. Digging earns about 0.15 points per
   1,000 frames against 1.6 for food, and diggers barely differ from non-diggers in it, so selection cannot see it.
3. **The prices do not do what they say.** The dig price does not collapse (2.9 to 2.1), so it is not why digging
   fades; this review first guessed it was, and the measurement says no. The clear price pins at its cap by 75,000
   frames, but clearing is 1% of what picks parents, so it neither pays the spoil treadmill nor stops it. The tank food
   price sits at 1.0. All three adaptive prices are inert; late-round bailing is unselected behaviour, not paid
   behaviour.
4. **Births ignore the store (minor).** Three tanks are 600 energy, under one provisions cell (960), so ~220 births a
   colony-round are ~150 cells against ~3,400 brought home. The real cost is that births run on the score's schedule
   whatever the store holds. It is also a rule the game does not have.
5. **The parent rank punishes lumpy earners.** z divides by the ant's own squared points, so one big delivery counts
   against a forager. In the 12-colony round, z picked 1,022 parents and plain score would have picked the same ant 271
   times (27%); the research lane found 44 of 133. `rank=shrunk` is built but off in the standard job.
6. **Training and judging measure different things.** Arms are scored by in-run food and judged by the common garden,
   so arms win in training and lose in the garden (veto-1022: more food, 88 cuts against 130).
7. **Late generations have nothing to dig** (Fable review): half of all cuts fall in a round's first quarter at a fixed
   headcount, so most births are judged on food alone and digging erodes under mutation.

## 3. What to change

None of these is a new weight.

| # | Change | Fixes | Cost | How to judge |
| --- | --- | --- | --- | --- |
| 1 | **Select colonies, not only ants.** After every round judge each colony on its result; refound the bottom half from the top half's brains; keep ant-level selection inside, frozen. Digging must be a gate or its own lexicase case, and the raw nest score cannot be used while a no-dig colony outscores every crater-digger (§4). | 1, 6, 7 | one wait for all colonies at each round end | common garden, 32+ seeds: cuts, starved, home food, nest score |
| 2 | **Do not build adaptive ant-level weights.** The need-driven prices are already inert, and weight cannot buy a job ants barely differ in. Put need where the ant senses it (hunger, a crowded nest) and where the colony is judged. | 2, 3 | none | already judged: x2/x4 dig weight only delayed the loss |
| 3 | **Rank parents by shrunk rate, not z** (`rank=shrunk`). | 5 | none | parents agreeing with plain score; home food paired |
| 4 | **Births cost the store** (low priority). | 4 | more early extinctions | starved and extinct, paired by seed |

Colony selection is also the natural home for the owner's pool-of-brains ruling (2026-10-22): the product becomes the
best colonies' surviving mixture, and the number of colony lines alive at round 20 measures diversity. Fable's lexicase
and crossover switches fit inside it.

## 4. Colony-selection test

**Comparing whole colonies is a far stronger lever than anything in the ant score, and it bought exactly what it was
scored on.** 12 colonies of hA1, 6 rounds of 200,000 frames, standard ant score inside each. After each round the colony
arm ranked colonies by a rank-sum of four results (deliveries within 8 columns, new-ground cuts within 20 columns in the
round's second half, starved, final nest score) and refounded the bottom 6 from the top 6's carried 30 (best donates to
worst; extinct ranks last). Both arms' final carries were then run in the same 12 fixed fresh worlds (seed
`c*1000+99`), selection off, 200,000 frames, with unselected hA1 mutants and the old ant in the same worlds.

| Arm | Food home (median) | New ground cut (median) | Colonies still digging (20+ cuts) | Starved of 30 (mean) | Nest score (median) | Colonies alive after 6 rounds |
| --- | --- | --- | --- | --- | --- | --- |
| Colony selection | 12,324 | 0 | 0 of 12 | 1.7 | 1 | 12 |
| Plain (today's training) | 4,131 | 132 | 9 of 10 | 5.5 | -90 | 10 |
| hA1 unselected | 270 | 201 | 12 of 12 | 3.9 | -119 | - |
| Old ant | 652 | 298 | 12 of 12 | 0.0 | 143 | - |

Paired by world, colony vs plain: more food in 9 of 10, fewer starved in 7 (3 more), fewer cuts in 9 of 9. In training,
round 5 totals over 12 colonies were 173,251 home vs 46,867, 34 starved vs 339, and 28 late cuts vs 439.

**Why digging lost.** Every colony that digs today makes a crater, so its nest score is negative (-90). A colony that
never digs scores about +1. Food and starvation also favour colonies where every ant forages, so three of the four
measures pointed away from digging and the cuts rank was outvoted. The old ant shows a digging colony can score well
(+143, a real chamber).

**Next (the Teacher lane's):** colony selection with digging as a gate (no colony without a chamber, or without N
second-half new-ground cuts, can rank in the top half) or as a lexicase case over colonies, 64 colonies on the VM.

**Caveats:** 12 colonies and one seed set. The plain arm matches the Teacher lane's 64-seed common garden (about 3,900
home, about 5 starved, about 130 cuts), a sanity check on the setup.

## 5. Changing the score over time

The owner asked how much the score should change as the ants improve (food first, then one chamber, then a
multi-chamber nest), and how to handle cheats. Scoring easy behaviour first is sound and standard. The risk is in how a
rung is retired, and §4 shows it: when the weight left digging, digging went to zero.

- **Benefits:** something to select on early, and steps small enough for mutation to find.
- **Risks:** a skill that stops being paid erodes; each new measure gets gamed (surface-soil farming, the crater, the
  no-dig nest score in §4); scores across a change cannot be compared; weights get moved by feel, not readiness.

What to do instead of moving weights:

1. **A met rung becomes a gate, not a smaller weight.** A colony that stops feeding itself or stops digging cannot
   rank in the top half; the newest rung orders the colonies that pass.
2. **Promote on readiness, not time:** add the next rung when most colonies reliably pass the current one.
3. **Put harder goals into the world as needs:** a growing colony that must make room, cold nights outside.
4. **Keep a judge that is never trained on:** fixed worlds, the old ant as the bar, pictures. When a cheat appears, fix
   the measure, add a guard for it, and log the change at a round boundary.

## Files

In `scoring-review/` beside this file: `colsel.py` (the round driver, plain or colony mode), `cg.py` (the fixed-world
judge), `picks.py` (selection differential by part), and `pick-log-vs-ee38ae5b.patch` (a read-only per-part credit log
and `pick.csv` for `examples/inrun.rs` on the compute branch). Raw results are in the project files under
`scoring-review/`.
