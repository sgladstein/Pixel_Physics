# Cheats already seen, and what catches each

Ladder research lane, 2026-10-17. For the brain lane (consumer) and anyone writing a grader in `src/train/`.

**Rule of thumb:** every score written for this rig so far was gamed within a few generations. Each line below is a hole that was found once. A grader for a new rung should be checked against every row here before it is trained on, because most of them are not about the rung they were found in.

Status words: **measured** = seen in a log; **closed** = the rig now blocks it; **open** = not blocked yet; **inferred** = reasoned, not run.

## Found by the brain rig (10-17)

| # | Cheat | Where seen | What it fakes | What catches it | Status |
|---|---|---|---|---|---|
| C1 | **Scatter**: break the food heap and spread it, so cells near home count without a delivery | forage, brainfit probe | "food at home" counted as cells in a window | count food delivered into the nest store (or eaten at the nest by a living ant), not cells lying in a column window; a picture | measured, partly closed (window is still a column band, ±12 of the nest) |
| C2 | **Air hops**: use `Impulse`/`Fly`, verbs meant for other species, to jump food home | forage probe | delivery without walking | ban verbs the body should not have (`NOT_ANT_VERBS`) | closed |
| C3 | **Colony shrink hidden by hatching**: founders die, brood hatches and refills the head count | forage, 12 to 8 | "colony alive" | count deaths of the *founding* ants, not the head count at the end | closed (rig-2026-10-17c, local commit 9057e718) |
| C4 | **One-way habit on mirrored layouts**: always go east; the mean over east+west layouts rewards a specialist | forage s1 (hold-out mean 14.0, sd 15.2; 0.0 on every west layout, all founders dead); trail one-way walker 0.28 | "reads the world" | score the **worse side** of each mirrored pair, not the mean; print per-side numbers. **Not enough on trail** (measured 15:05): the walls turn a straight walker round, so it scores 0.20-0.26 on the worse side; signed progress puts it at 0 (`trail-rung-controls.md`) | closed in plan (brain lane step 0); trail needs signed progress |
| C5 | **Roof heaped over a pit**: once drop sites are free, pile spoil over a dip and score "roofed room" without digging a nest | dig, review-rev2 N13 (inferred) | "roofed room" | count room only below the founding ground line and joined to the air; see stage1-redteam.md R2 for the remaining gap: a lid two or more rows thick, and a nest sealed at the mouth, still count under PR 708 (72 and 40 cells). The drop-in in `dropin-nest-room.md` closes both and costs honest digging nothing | closed by the drop-in once pasted |
| C6 | **Spoil haul leaks the home direction into blind runs**: the haul branch of the homeward re-roll ignores `home_bias` | dig blind control, review-pr707-recheck | "blind" control that is not blind | blind outside the brain too: every body-side read of home, trail or nest cut on blind layouts | closed (cut) |
| C7 | **Mixed training, clone test**: trained among strangers, judged among copies, which jam a one-lane floor | trail (0.12-0.18 as clones vs 0.82 when trained as clones) | a training score that does not predict the test | train and test the same unit; colony arenas always use clones | closed (`clones=1`) |
| C8 | **Score credits a meal of a dead nestmate**: `home` counts crop food digested at the nest, which includes corpses | forage, review-rev2 N10 | food brought home | exclude corpse meals; count food fed to larvae instead | open (step 0 plan) |
| C9 | **Brood raising is unpaid**: food spent on larvae is not "home", so a colony that raises no brood scores higher | forage, game ant 30.1 cells taken but 8.4 scored | a fair baseline | count food fed to larvae as delivered | open (step 0 plan) |
| C10 | **Drift on a dead input**: big weights on `PheroARise`, which reads 0 unless laden | trail champions (12.2 on it) | nothing, but it reads as "uses change over time" | connection cost; check a weight's input actually varies in the arena before reading meaning into it | open, harmless |
| C11 | **Energy top-up makes digging free**: dig arena refills every ant every 50 frames | dig | a dig habit that would starve a real colony | a later rung that charges for digging; stage 1 is fine as long as the claim is "where", not "how much" | open by design |

## Found by the stage 2 notes' review (10-17 ~16:30, `review-stage2-docs.md`)

| # | Cheat | Where seen | What it fakes | What catches it | Status |
|---|---|---|---|---|---|
| S1 | **Rest beats wandering on a signed score**: rest reads 0, every blind mover reads below 0 on the worse side | trail, PR 713 (blank -0.025, persist -0.016) | the first hill from blank is "move less" (E2 by construction) | fold each pair by its mean, keep the worse side as a gate; print the share of brains that mostly rest | measured, open |
| S2 | **Edge, not slope**: step only where there is scent; the bare end of the trail turns the ant back | trail (0.311 alone, 0.171 as 6, against the follower's 0.443 and 0.260) | "follows the gradient" | an edge-only control, or a faint trail far past the start so the edge is never near | measured, open |
| S3 | **Hatchlings' old age counted as founders kept** | scarce probe and the rig's `old_age()` (31 old-age deaths against 23 founders gone) | survival | count founder ids only | measured, open |
| S4 | **Free jaw work on the mind path**: the mind returns before the dig, gnaw and share billing | PR 713 (code) | a dig or feed habit with no price | bill jaw work in the mind's tick; a test that can go red | code, open |
| S5 | **A door flood through open cells reaches the sky** | dig rung 2 as first written | "dug at the nest" for any surface scrape | flood through dug cells only | inferred, fixed in the note |
| S6 | **Bell timing**: one snapshot at the last frame pays a colony that crashes just after it | scarce (cheap arm 12.4 at 20k → 15.1 at 24k while 18.9 a layout starved) | a healthy colony | the mean or minimum over the last quarter of the run | measured, open |
| S7 | **Founder premium**: one founder costs 1/12 of the score, and the mind can read age | scarce (inferred) | colony care | births per founder as a second number | inferred |

## Found earlier, by the 2026-09 evolution work (`Reports/creature-direction.md` §13, filed in `Reports/dead-ends.md` ~1030-1061)

These were found before this rig existed. They apply to every rung.

| # | Cheat or trap | Record | What catches it |
|---|---|---|---|
| E1 | **The metric measured where ants were placed**, not what they did: a provably immobile zero-genome scored on "range" and "left the nest" | dead-ends ~1032 | run the **zero genome** (does nothing) on every new grader; it must score near 0 |
| E2 | **Doing nothing wins**: in an arena where idling is safest, selection finds the ant that moves least | dead-ends ~1038 | readiness check: a hand-built brain that does the task must beat the zero genome by a wide margin before any training |
| E3 | **The horizon flatters a strategy**: at 1,000 ticks idling won; at 3,000 the sign flipped | dead-ends ~1059 | score at two horizons (the arena length and 2x); a winner that loses at 2x is a horizon artefact |
| E4 | **A per-seed ceiling in the denominator**: placement silently failed on some cells, so the best possible score was 0.44 on one seed | dead-ends ~1050 | arithmetic sanity check of the best and worst possible score per layout; print `started` |
| E5 | **A flat floor is degenerate**: one legal step, so behaviour is forced | dead-ends ~1036 | layouts that really differ in shape; check the zero genome and a random genome produce different traces |
| E6 | **Position-keyed randomness becomes an inherited trait** | dead-ends ~1061 | key every fitness-relevant draw on the organism, not the cell |
| E7 | **A route test measured the terrain**: only one route was walkable, so "chose the short route" was forced | dead-ends ~1017 | a control where the right answer is the other way (mirror, moved food) |

## Kinds of cheat, so new ones are recognised

Every row above is one of these. A new grader gets one control per kind.

1. **Counting a place, not an act** (C1, E1). The score reads where things lie, so moving them by any route counts. *Control:* count the event (delivered, dug, fed) from the far side, and run the zero genome.
2. **A side door in the body** (C2, C6). A verb or a body sense the brain should not have does the work. *Control:* bans, plus the blind run blind outside the brain.
3. **Paying for a number the colony can trade** (C3, C8, C9). One quantity is bought with another the score does not see. *Control:* a second number it cannot trade away, printed beside the score.
4. **Averaging over layouts that a specialist can split** (C4, E7). *Control:* score the worse side; mirror pairs; moved targets.
5. **The score is met by building instead of doing** (C5). *Control:* score **where** the roof is, not what it is made of: room counts only under a roof from the original bed, below the old ground line, and joined to the sky (`dropin-nest-room.md`). Packed pellets and cut lining are the same material, so a material test cannot do it. A short settle catches only one-row lids.
6. **Training and test disagree** (C7, E3). *Control:* same unit, same horizon family, two horizons.
7. **The arena does the work** (E2, E4, E5). *Control:* a hand-built brain that should pass and one that should fail, run before evolution.
