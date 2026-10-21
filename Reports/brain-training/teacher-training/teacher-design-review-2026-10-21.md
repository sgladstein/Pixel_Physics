# Teacher design: the review's disposition

*2026-10-21, by the teacher-approach session. Three adversarial reviews of
[`teacher-design-2026-10-21.md`](teacher-design-2026-10-21.md) v1 (commit 6aa3b4b4), each by an independent agent with
its own lens. The reviews are kept verbatim in `review/`: `design-method.md` (imitation learning, planning, the
literature), `design-biology.md` (biology and the game, against the dead-ends record), `design-evidence.md` (every
number and sense claim checked against the code and the step-1 CSVs, with its scripts' results). v2 of the design
answers them. Literature verification levels are the reviewers' own: nothing in the method review is full-text (the
Consensus quota was spent and arXiv was refused); the biology review read Gautrais 2014 in full and checked six
abstracts.*

## What changed the plan

**All three reviews, independently, took the first step away from the soil planner**, for three different reasons:

- **Method**: the doc named the student as the bottleneck and then scheduled three days of planner building before any
  experiment on it. Two DAgger rounds had made copies worse; the dig collapse was untraced.
- **Evidence**: at 8 rows a perfect soil planner adds at most ~0.8 full loads (s161 already carries 88% of its full
  cuts out, 86% of put-downs already clear), and its trial drop cannot move `full_loads`, which the arena credits at
  the put-down cell.
- **Biology**: placed spoil never slides in this engine, so the trial drop returns a constant; and a full ring is, in
  2D, a wall across the road to the food.

So v2 opens with **step 0.5** (why copying fails, offline, with a go/no-go written in advance), then the **search pilot
on dig-or-not**, then planners -- nest first, soil for the mound only.

## Findings and dispositions

| # | Review | Finding | Disposition |
|---|---|---|---|
| 1 | Method B1 | The order of work skips the bottleneck it names | **Accepted.** Step 0.5 (design §4) with a go/no-go |
| 2 | Method B2 | One lesson form would have weakened go-down: at any tau in the grid a +0.02-load step gets its odds ×1.07-1.95, a tenth of the push that worked | **Accepted.** Lesson form per decision type; hard targets for steps unless a retrofit reproduces go-down (§1, §4 item 4) |
| 3 | Method B3 | Walk home was dropped, not replaced: it teaches the *empty* return trip (+5.40 on s171), and the nest planner's empty-ant step labels bring back the look-alike that broke fetch | **Accepted.** Walk home kept; the context probe gates it (§2, §3) |
| 4 | Evidence B1 | The trial drop cannot move the score that gates it | **Accepted.** Put-downs valued against the *planned* hole, by geometry (§5) |
| 5 | Evidence B2 | At 8 rows the soil planner cannot show a gain | **Accepted.** It is for the mound, scored on the mound, after the nest planner (§5, §10) |
| 6 | Biology B1, Evidence M3 | Placed spoil never slides; the 45° geometry and the trial drop fail; loose soil runs at 33° | **Accepted.** Trial *cut* instead (does a cut unfoot spoil that slumps into the hole?); 33° geometry; the owner is asked whether fresh pellets should settle (§5) |
| 7 | Biology B2 | A ring is a wall in 2D; the record shows the mound arm worst at door use (30 of 72 against 54) and refusing mound cuts starved colonies | **Accepted as a fork for the owner**, who called a ring ideal: paired ring/crescent render with the foragers' route (§5). Mound cuts abstained solo, never labelled "no" |
| 8 | Method M1 | Tau limits movement on labelled states; spread is on look-alikes, which step 1's 10× anchors already pulled on | **Accepted.** The tau curve is plotted against step 1's loss-weight curve; if they coincide, tau buys nothing against spread (§4 item 4). Citation corrected to the MPO/AWR family |
| 9 | Method M2 | The exchange rate is biased four ways (whose rate, per what, frames or joules, nearest-site potential used alone) | **Accepted.** Current student's rate, per round, stated per frame of the whole run (per joule where energy binds); a rollout check per planner (§1, §9 stage 1b) |
| 10 | Method M3, Evidence M4 | The probe evidence was misread: the 94-100% was all-features; the brain-channel probe (0.65/0.87, AUC 0.613) predicted the dig failure | **Accepted.** Three probes with numeric bars, the brain channel the gate (§3) |
| 11 | Method M4 | A stronger teacher can make a worse student; planners should adapt to the student (GPS, PLATO, A2D, DPI) | **Accepted.** Student-aware tie-breaking in every planner (§1) |
| 12 | Method M5 | "Way out 3" is the hand teacher again and makes the gate circular | **Accepted.** The local rule *is* the teacher; the planner earns its place only on the disagreement set, measured and probed alone; the owner told it narrows the nest to local-rule shapes (§3) |
| 13 | Method M6 | Central assignment is privileged information; local students fail to imitate central planners in dense settings; "no ant can tell" contradicts Buhl 2005; outcome scoring unpriced | **Accepted.** Assignment pre-gate on 12-ant logs; counterfactual search as fallback; local sizing rule first; teacher-free 2/4-ant runs first; priced (§7) |
| 14 | Method M7 | DAgger anchors the copy to s161 where it learned to differ | **Accepted as a hypothesis**, tested in step 0.5 item 5; p is the current student (§1) |
| 15 | Method M8 | No stop conditions; controls that cannot fail; stage 1 bar should be non-inferiority | **Accepted.** Abandon-if column, known-negative/positive plan controls, non-inferiority bar, numeric bars (§9, §11) |
| 16 | Evidence M4 | `HEADROOM` is not a roof template; `f::CROWD` and `f::ROOM` vary solo | **Accepted.** Sense table corrected; headroom over the head listed as a candidate sense (§3, §6.2) |
| 17 | Evidence M5, Biology M4 | `in_room`/`in_chamber` are founding-cut rectangles; `rooms_of` merges chambers through 4-wide passages and cannot see a 2-tall chamber | **Accepted, and built**: the general nest score uses a 7×5 chamber core (§6.1) |
| 18 | Evidence M6 | "Never copy dig alone" misreads the ceiling (acting alone +5.16, one seed under 3) | **Accepted.** Claim withdrawn; dig is next after step 0.5 (§2) |
| 19 | Evidence M7 | Capacity is not only a brain change: 8 of 16 scorer units are dead in s161 | **Accepted** (§3) |
| 20 | Biology M1 | §4.4 missed five records: the far ring, the mound arm's door use, slope at the drop, the building mark, a room-height cap, crowding-driven rooms | **Accepted.** Each named with its re-test condition as a pre-check (§3, §5) |
| 21 | Biology M2 | The "2D lab nests" are horizontal discs, not sections | **Accepted.** Relabelled plan-view; used for colony laws; round-room blueprint demoted (§6.3) |
| 22 | Biology M3 | The blueprints are nests of colonies 10-1,000× larger | **Accepted.** Profile scales by colony size: 12 ants, shaft + 2-3 chambers (§6.3) |
| 23 | Biology M4 | True-scale chambers read as cracks; keep 5-6 cells | **Superseded by the owner's ruling: 7 cells** (from three rendered sizes) |
| 24 | Biology M5 | "About 8 cells tall" was the renderer's cap; "wider than any real disc" was wrong | **Accepted.** Corrected; budget stated as nest size + backfill + remainder (§5) |
| 25 | Biology M6 | Senses: probe slope in the brain's form first; body pitch better grounded; building scent only if it decays; `LAST_FOOD` is a memory; encounter rate for crowding | **Accepted** (§3) |
| 26 | Biology M7 | Khuong's template is about building roofs, not digging | **Accepted.** Tschinkel for the shape; Khuong as an analogy; the `FLAT_ROOM` record noted (§3) |
| 27 | Biology S1-S6 | A lumpy distributed heap, a preferred side per ant, visible drops, event counts in `meta`, door-keeping as a verb, staged growth, tunnels in the mound are real | **Accepted** where they fit (§5, §6.2, §9 stage 0); door-keeping noted for the soil planner |
| 28 | Biology recommendation | The staircase is the best blueprint for realism and learnability | **Not taken: the owner picked the harvester.** The score and generator are profile-based, so the staircase stays one profile away |
| 29 | Method suggestions | Search pilot before the soil planner; Option Iteration / Distral if capacity; Teacher-Student Curriculum Learning for lesson order; Eysenbach 2022 for self-imitation; teacher-free colony runs first | **Accepted**: search moved ahead (§8, §10), colony runs first (§7); the rest noted for when their condition arises |
| 30 | Minor, all three | Stale numbers (kept skills, DAgger, s171 go-down), the 300-frame trial against the 2,000 settle, cost estimates (~7× high, clone unmeasured), T1 amendment belongs in the guidelines, colony search cost not linear, `c::SPOIL` vs `f::SPOIL`, `DEPTH` compression | **Accepted**; T1 amended in `guidelines-2026-10-20.md` |

## What the reviews said to keep

The four-part teacher with the rules as the degenerate planner and the examiner; acting before copying for every
planner; "the planner proposes, the search disposes"; the honest prediction that the soil planner does not fix
spill-back; "never forbid mound cuts by position"; new senses only through the brain lane with links at zero; the
owner's eye on the plan; abstention as "the brain itself"; varying the world before the brain for division of labour;
and the context probe -- "the best idea in the doc for the failure that actually happened".
