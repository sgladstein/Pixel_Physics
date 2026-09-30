# Week review: the ant line, PRs #485-#518 (2026-09-29)

**Status: review, read-only to the engine; pinned at `2274e347` (`main`, 2026-09-29; #519, #520 and #522 merged after the pin and are not covered, §12).** An adversarial read of
the thirty-four PRs the owner's own ant sessions merged between 22 and 29 September, every one on green CI
and none with a human reviewer. It found no blocker in what the week changed (a panic older than the week, fixed after the pin, is in §12). It found that the week's defaults, taken together, are a
large net gain on seeds nobody tuned on, and where the evidence, the guards and the records behind them are
thinner than they read. Lane notes: [lanes/foraging-loop.md](lanes/foraging-loop.md),
[lanes/nest-mouth.md](lanes/nest-mouth.md). Method and controls: §11. Nothing was posted on the merged PRs.

## 0. In brief

**What the week did.** Thirty-four PRs (#485-#518, 22-29 September) on three ant lines, about ten thousand lines of
engine and asset churn: the walk, the crop and scouting (a heading chooser, a doubled crop, food half as heavy, a
hungry ant that scouts); the foraging loop (the forage drive, a packed lunch, the birth price, the trip reach); and the
nest (a dug founding shaft, the heap cue, dig-down, and the door and storeroom the owner called "the full granary",
with the one-entrance work built and held off). Fourteen of the PRs changed what every ant does by default. Every one
was merged by its own session on green CI, with no human reviewer, so this is the first time anyone but the authors
has read them.

**Verdict: no blocker, and the stack works: the walk is what lets a colony live at all, and the other thirteen defaults add a large gain on top of it on the bed, on evidence that is thinner than it reads.**

- **On the colony bed, on seeds nobody tuned on** (24 frozen in advance, 72 more as a replication), the ant with every
  shipped switch put back dies out on every seed (477 of the 480 founders starve, 3 die of old age; 14 cells of food are taken in all). The shipped ant, on the same 24 seeds, takes
  5,513 cells of food, breeds 181 and ends with 442 ants. Under the game's own breeding rules the same swap turns a colony that dies (58 born)
  into one that fills the world (63,642 born).
- **That pair is decided by the walk alone**, so the review also ran the walk left on with the other thirteen defaults put back:
  the thirteen still add 2.6x the food and 4.5x the births on the 24 frozen seeds (2.8x and 5.4x on the 72; food up on 24 of 24 seeds and on 72 of 72).
- **In the lab box** (the lanes' own pre-ship instrument), the walk-on pair reads as a trade, not a gain: starvation per ant-frame falls 78% (paired interval 63% to 87%; lower on 11 of 12 seeds) and boxes that die
  out go from 3 of 12 to 0, while births (x0.82, interval x0.43 to x1.56), intake, ant-frames lived and peak colony do not separate at 12 seeds:
  a steadier colony, not demonstrably a smaller one, and the sign gate scores it only through starvation.
- **Hygiene**: of the four switch-off PRs tried, the three that could be isolated reproduce their parents when off; runs are deterministic;
  the heap cue, dig-down and "spoil stays spoil" each still cut entrances at HEAD.
- **Frame cost** may rise a little with the colony (+1% per tick at 0 ants, +2% at 200, +5% at 600; the per-ant cost +12%; not established: 5 of 6 pairs), most likely from the shipped walk.

**Where the risk is** (ranked in §1):

1. **The evidence used to ship.** The lab's pre-ship gate is a sign test that cannot see a 15-25% loss (it fires 5-46% of the time), so
   "no gate fired" is being written as "neutral"; the colony bed cannot see the birth price at all; and three defaults (dig-down, the storeroom,
   the heap cue) cost the bed something at HEAD that the stack result hides, with a fourth (drop-through-bodies) leaning the same way.
2. **The nest.** The granary (#513) undid the week's own one-entrance nest: entrances 3 -> 6, "more nest-like than random digging" 20 of 24 seeds ->
   0, and a nest that now leans west on **24 of 24** seeds because the storeroom is always cut on the west. #513's body says most of this and
   the owner ruled the trade; the titles of #507, #508 and #518 and the nest lane note still say one entrance and "stops leaning west".
3. **The guards.** The walk every ant ships with has no unit-level trace or ledger guard, and 12 of the 14 shipped-on defaults (`CHOOSER`, `CARRY_PATIENCE`, `FORAGE_DRIVE`, `TRIP_REACH`, `PACKED_LUNCH`, `BIRTH_PRICE`, `SPOIL_CUE`, `DIG_DOWN`, `SPOIL_PACKS`, `NEST_DOOR`, `STOREROOM`, `DROP_REACH`) can be put back to their old values without one of the 1,947 tests going red.
4. **Small things with evidence**: a colony founded over a corpse on the surface erases it (#518 says it does not); the nest scorer scores an
   arm that stops the digging as `0/0`; putting `NEST_SHAFT` back on its own leaves a nest that cannot dig; four switch parsers ignore a typo without a word.

**What I would do first**, cheapest first: (a) re-scope the four stale headlines (§7); (b) print the paired interval beside every gate row in
`labpair.py`, so "not significant" arrives with what it could not exclude; (c) one test per shipped default that goes red when it is switched
back (12 have none); (d) make `nestscore.py` count an unscored seed as worse, not absent; (e) ask the owner whether "it is okay if nest
building **temporarily** hurts colony numbers" covers a standing default (#508, #513), and whether the lab's steadier colony (births unresolved) is the
colony they want.

## 1. Findings, ranked

Severity scale: **blocker** (wrong behaviour, a crash or a determinism break on a default path) /
**unsupported ship** (a default flipped on evidence that does not carry it) / **false or stale record** /
**blind guard** / **nit**. CONFIRMED means the coordinator reproduced it or put the two lines side by side;
PLAUSIBLE means it was read, not run. Where a reader agent found something the coordinator did not
re-check, it is marked *(reader)*. Nothing was posted on the merged PRs.

**No blocker was found.** No PR changes a default in a way the review could show to be a net harm on the
bed or in the lab box (three carry a measurable cost on the bed against a gain elsewhere or an owner ruling, and a fourth leans the same way: W10), no run diverged between two
executions of the same binary, and every PR that could be isolated reproduced its parent when its switch was off. The review
looked at what the week's PRs changed and ran the game-default regime; it did not look for crashes elsewhere, and one older panic,
fixed by #519 after the pin, is noted in §12. The
findings are about *evidence and records*, and about a handful of guards and edges.

| # | Finding | Severity | Status | PRs |
|---|---|---|---|---|
| W1 | **"No gate fired" is being read as "neutral", and the gate cannot see the harms it is read against.** Sign test at n = 24 flags a true 15% loss 5-15% of the time; #510's births drop (x0.62) scored p = 0.064; the bed cannot see #510 at all (24 of 24 seeds identical). The four wrong-way births estimates sit beside a steadier lab colony: with the walk on, the other thirteen give births x0.82 (95% interval x0.43 to x1.56, 12 fresh seeds, unresolved), starvation per ant-frame -78% (interval -63% to -87%) and die-outs 3 -> 0 of 12; the wiki's "far fewer starve, none of a dozen test boxes died out" is the part the pair supports | unsupported ship (process) | CONFIRMED | #509, #510, #513, #515, #516 |
| W2 | **The nest still leans west on every seed, because the storeroom is always cut on the west; #518's title says the lean stopped.** 24 of 24 seeds west, median 11 columns; `STOREROOM=off` centres it (10 / 12 / 2) | false headline + shipped bias | CONFIRMED | #513, #518 |
| W3 | **The granary undid the week's one-entrance nest, and the titles and docs of #507 and #508 still say one entrance.** Entrances 3 -> 6, "more nest-like than random" 20 of 24 -> 0 of 24; put the granary back at HEAD and the 3-entrance nest returns (2.5 entrances, 20 of 24 nest-like, cells dug 242 against 600). Disclosed once in #513, owner-ruled, not carried into titles or docs | false or stale record | CONFIRMED | #507, #508, #513 |
| W4 | **The shipped walk has no unit-level guard, and 12 of the 14 shipped-on defaults are pinned by no test at all.** Six tests are pinned to the old walk (five in `creature.rs`, one in `specimen.rs`); with the five in `creature.rs` unpinned, four go red, all four tuned to the old walk (two by mechanism, two by a scene number) | blind guard | CONFIRMED | #487, #502, #506, #508, #512 |
| W5 | **A colony founded over a corpse lying on the surface erases it**; #518's fix and the wiki say it is left where it lies. Reproduced by a scratch test | small bug + false record | CONFIRMED (run) | #518, #493 |
| W6 | **The nest scorer drops the seeds where an arm stopped the digging**, so the worst outcome reads `0/0`. Shown live: `NEST_SHAFT=off` digs nothing on 24 of 24 seeds and `nestscore.py --base` prints `mouths 0/0, reach 0/0, ...` with no warning | blind guard (instrument) | CONFIRMED (run) | #498 (origin), all nest PRs |
| W7 | **Putting one switch back does not give the ant before it, and mistyped switches fail open.** `NEST_SHAFT=off` with the door on leaves a nest that cannot dig; four parsers ignore a typo silently; `DIG_DOWN`'s qualifier typo selects the arm kept off for a measured harm, and a test pins that | hazard | CONFIRMED | #507, #508, #485, #487, #494, #503 |
| W8 | **Stale and inconsistent records**: a doc that says the shipped cue is off, two `World` field docs that give the old defaults, a lane note quoting an older ant, a store-lunch hold whose stated reason contradicts the written gate, a dated bullet naming the wrong value, a paraphrase that drops "temporarily" (§7) | false or stale record | CONFIRMED | #490, #503, #507, #508, #514, #515 |
| W9 | **Frame cost**: **a small per-ant increase, likely but not established: +1% per tick at 0 ants, +2% at 200, +5% at 600** on the lab-box replay (`antcost`, six alternating pairs; HEAD slower in 4, 5 and 5 of the six, sign p 0.22 at 200 and 600; the fitted per-ant cost 3.89 -> 4.36 µs, +12%, the six HEAD slopes ranking above the six baseline slopes at rank-test p about 0.04, exploratory). The likeliest cause is the shipped walk itself (the baseline walked the old way), which the review did not isolate; nothing measured suggests a per-cell cost on the sweep (§8). | (cost) | CONFIRMED (measured); attribution not isolated | #487, #513 |
| W10 | **Three shipped defaults cost the bed something at HEAD, and a fourth leans the same way; the evidence they shipped on predates the ships that changed the ant.** Putting one back, on 72 fresh seeds (125-196) that were not used to pick the arm: dig-down (#508): food +8%, births +28%, starved -24%, ants at the end +12% (sign p 0.003-0.046; its gain is in the nest, 9 entrances against 6 without it, not on the bed; the 24 frozen seeds leaned the other way on births, -17%, 6 up and 15 down, p 0.078); the storeroom (#513): starved -65% (p < 0.001), nest food -48%, ants at the end +9% (p 0.18; +13%, p 0.011 on the 24 frozen), births and food unchanged; the heap cue (#507): without it births +29% (p 0.008) but starved +86% (p < 0.001), ants at the end unchanged, so a trade; drop-through-bodies (#485, owner: "on, because the loop improves"): food +5.5%, births +14%, ants at the end +4% (p 0.08-0.22), food eaten +4% (p 0.044), the same direction on the 24 frozen. Each was stated or owner-ruled at ship, or is small; **none is a demonstrated net harm**, and the drop is the one to re-check | unsupported ship (mild, exploratory) | CONFIRMED (measured), exploratory | #485, #507, #508, #513 |
| W11 | **Small edges** (nits): `scout_home` outlives a give-up on 11 of 130 departures after one (8%); `lift_out` returns "no site" at its 4,096-cell cap; a storeroom near the west edge is clipped; the drop search is unbounded and west-first (owner-ruled); the default shaft is half a column east; three of #517's fixes have no test; one uncached environment read | nit | mixed | #485, #492, #493, #517 |

## 2. Does the stack hold on seeds nobody tuned on?

**Yes: the review's main worry did not come true. But the tidiest result below is decided by one switch, so the
evidence for the other thirteen is laid out separately.**
Every PR was measured against the `main` before it, on colony-bed seeds 1-24 that every lane reused, so a
default that "wins" could be a winner on the seeds it was chosen on, and twelve small wins measured one at a time do
not add up to a measurement of the whole. So the review froze **fresh seeds 101-124** as the primary set (with
125-196 as an independent replication) *before* any run, and ran a ladder of the milestone commits, an arm that is
HEAD with every shipped switch put back to its "before" value (*all-off*), an arm that is HEAD with the walk left on and
the other thirteen put back (*walk on*), and each switch alone (§10).

**Table L1. The ladder, 24 frozen fresh seeds (101-124), gap 90, 24,000 frames. Cell = sum over seeds (median per seed).**

| arm | n | food taken (cells) | J at nest | born | starved | old age | ants at end | food eaten (J) | note |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `bb65d507` anchor (before #485) | 24 | n/p | n/p | n/p | n/p | n/p | n/p | 13,180 (0) | old harness: only food eaten is printed |
| all shipped switches at their before-values (HEAD) | 24 | 14 (0) | 953 (0) | 0 (0) | 477 (20) | 3 (0) | 0 (0) | 2,984 (0) | decided by the walk alone: `CHOOSER=off` on its own reads the same (§10) |
| **walk on, the other thirteen switches at their before-values (HEAD)** | 24 | 2,130 (86) | 85,742 (3,224) | 40 (2) | 254 (10) | 53 (2) | 213 (8) | 422,948 (17,224) | the mediating arm: what the rest of the stack buys once the walk is on |
| `6ea225f5` #485 | 24 | 22 (0) | n/p | 0 (0) | 479 (20) | 1 (0) | 0 (0) | 3,970 (0) |  |
| `c889fa59` #504 | 24 | 2,813 (120) | 163,409 (7,060) | 40 (2) | 176 (6) | 73 (3) | 271 (12) | 540,134 (23,960) |  |
| `72383e39` #513 | 24 | 5,569 (238) | 263,227 (10,944) | 207 (8) | 113 (4) | 93 (4) | 481 (21) | 1,034,499 (43,457) |  |
| `2274e347` HEAD | 24 | 5,513 (234) | 259,881 (11,116) | 181 (8) | 116 (4) | 103 (4) | 442 (18) | 1,012,983 (41,510) |  |

**Table L2. Same arms, all 96 fresh seeds (101-196).**

| arm | n | food taken (cells) | J at nest | born | starved | old age | ants at end | food eaten (J) | note |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| all shipped switches at their before-values (HEAD) | 96 | 73 (0) | 4,532 (0) | 0 (0) | 1,902 (20) | 14 (0) | 4 (0) | 15,842 (0) | decided by the walk alone: `CHOOSER=off` on its own reads the same (§10) |
| **walk on, the other thirteen switches at their before-values (HEAD)** | 96 | 8,107 (85) | 333,830 (3,446) | 129 (1) | 1,091 (11) | 202 (2) | 756 (8) | 1,569,839 (16,654) | the mediating arm: what the rest of the stack buys once the walk is on |
| `6ea225f5` #485 | 96 | 78 (0) | n/p | 0 (0) | 1,908 (20) | 11 (0) | 1 (0) | 14,840 (0) |  |
| `c889fa59` #504 | 96 | 11,749 (122) | 638,200 (6,456) | 185 (2) | 698 (6) | 311 (3) | 1,096 (12) | 2,233,883 (24,146) |  |
| `72383e39` #513 | 96 | 22,125 (235) | 1,034,408 (11,143) | 717 (7) | 378 (2) | 403 (4) | 1,856 (20) | 4,065,752 (42,570) |  |
| `2274e347` HEAD | 96 | 22,188 (232) | 1,041,422 (11,246) | 663 (6) | 332 (3) | 419 (4) | 1,832 (19) | 4,054,567 (41,438) |  |

**Table L3. Paired steps on the 24 frozen seeds.**

| step | metric | base sum -> new sum | up/dn/tie by seed | sign p |
|---|---|---|---|---:|
| #485 -> #504 | food taken (cells) | 22 -> 2,813 | 24/0/0 | 0.000 |
| #485 -> #504 | born | 0 -> 40 | 20/0/4 | 0.000 |
| #485 -> #504 | starved | 479 -> 176 | 0/24/0 | 0.000 |
| #485 -> #504 | old age | 1 -> 73 | 24/0/0 | 0.000 |
| #485 -> #504 | ants at end | 0 -> 271 | 24/0/0 | 0.000 |
| #485 -> #504 | food eaten (J) | 3,970 -> 540,134 | 24/0/0 | 0.000 |
| #504 -> #513 | food taken (cells) | 2,813 -> 5,569 | 23/1/0 | 0.000 |
| #504 -> #513 | J at nest | 163,409 -> 263,227 | 20/4/0 | 0.002 |
| #504 -> #513 | born | 40 -> 207 | 24/0/0 | 0.000 |
| #504 -> #513 | starved | 176 -> 113 | 6/17/1 | 0.035 |
| #504 -> #513 | old age | 73 -> 93 | 13/4/7 | 0.049 |
| #504 -> #513 | ants at end | 271 -> 481 | 21/2/1 | 0.000 |
| #504 -> #513 | food eaten (J) | 540,134 -> 1,034,499 | 23/1/0 | 0.000 |
| #513 -> HEAD (#515-#518) | food taken (cells) | 5,569 -> 5,513 | 10/14/0 | 0.541 |
| #513 -> HEAD (#515-#518) | J at nest | 263,227 -> 259,881 | 13/11/0 | 0.839 |
| #513 -> HEAD (#515-#518) | born | 207 -> 181 | 8/12/4 | 0.503 |
| #513 -> HEAD (#515-#518) | starved | 113 -> 116 | 12/10/2 | 0.832 |
| #513 -> HEAD (#515-#518) | old age | 93 -> 103 | 12/6/6 | 0.238 |
| #513 -> HEAD (#515-#518) | ants at end | 481 -> 442 | 7/15/2 | 0.134 |
| #513 -> HEAD (#515-#518) | food eaten (J) | 1,034,499 -> 1,012,983 | 11/13/0 | 0.839 |
| all-off -> HEAD (the walk) | food taken (cells) | 14 -> 5,513 | 24/0/0 | 0.000 |
| all-off -> HEAD (the walk) | J at nest | 953 -> 259,881 | 24/0/0 | 0.000 |
| all-off -> HEAD (the walk) | born | 0 -> 181 | 23/0/1 | 0.000 |
| all-off -> HEAD (the walk) | starved | 477 -> 116 | 0/24/0 | 0.000 |
| all-off -> HEAD (the walk) | old age | 3 -> 103 | 24/0/0 | 0.000 |
| all-off -> HEAD (the walk) | ants at end | 0 -> 442 | 24/0/0 | 0.000 |
| all-off -> HEAD (the walk) | food eaten (J) | 2,984 -> 1,012,983 | 24/0/0 | 0.000 |
| all-off -> walk on (`CHOOSER` alone) | food taken (cells) | 14 -> 2,130 | 24/0/0 | 0.000 |
| all-off -> walk on (`CHOOSER` alone) | J at nest | 953 -> 85,742 | 24/0/0 | 0.000 |
| all-off -> walk on (`CHOOSER` alone) | born | 0 -> 40 | 18/0/6 | 0.000 |
| all-off -> walk on (`CHOOSER` alone) | starved | 477 -> 254 | 0/24/0 | 0.000 |
| all-off -> walk on (`CHOOSER` alone) | old age | 3 -> 53 | 20/1/3 | 0.000 |
| all-off -> walk on (`CHOOSER` alone) | ants at end | 0 -> 213 | 24/0/0 | 0.000 |
| all-off -> walk on (`CHOOSER` alone) | food eaten (J) | 2,984 -> 422,948 | 24/0/0 | 0.000 |
| walk on -> HEAD (the other thirteen) | food taken (cells) | 2,130 -> 5,513 | 24/0/0 | 0.000 |
| walk on -> HEAD (the other thirteen) | J at nest | 85,742 -> 259,881 | 24/0/0 | 0.000 |
| walk on -> HEAD (the other thirteen) | born | 40 -> 181 | 21/1/2 | 0.000 |
| walk on -> HEAD (the other thirteen) | starved | 254 -> 116 | 3/20/1 | 0.000 |
| walk on -> HEAD (the other thirteen) | old age | 53 -> 103 | 20/1/3 | 0.000 |
| walk on -> HEAD (the other thirteen) | ants at end | 213 -> 442 | 22/2/0 | 0.000 |
| walk on -> HEAD (the other thirteen) | food eaten (J) | 422,948 -> 1,012,983 | 24/0/0 | 0.000 |

**Table L4. Gap 140 (the far pile), 24 frozen seeds.**

| arm | n | food taken (cells) | J at nest | born | starved | old age | ants at end | food eaten (J) | note |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `bb65d507` anchor | 24 | n/p | n/p | n/p | n/p | n/p | n/p | 1,354 (0) | old harness: only food eaten |
| all shipped switches at before-values (HEAD) | 24 | 0 (0) | 0 (0) | 0 (0) | 479 (20) | 1 (0) | 0 (0) | 0 (0) |  |
| `2274e347` HEAD | 24 | 4,192 (184) | 194,654 (8,338) | 67 (2) | 135 (5) | 90 (4) | 322 (14) | 755,656 (32,216) |  |

**Table L5. The game's own defaults (no bed environment: unrestricted breeding), gap 90, 24 frozen seeds.**

| arm | n | food taken (cells) | J at nest | born | starved | old age | ants at end | food eaten (J) | note |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| all shipped switches at before-values (HEAD) | 24 | 430 (1) | 2,115 (0) | 58 (0) | 476 (20) | 6 (0) | 56 (0) | 112,086 (228) |  |
| `2274e347` HEAD | 24 | 180,483 (7,419) | 33,865 (1,346) | 63,642 (2,328) | 4,418 (64) | 1,801 (83) | 33,542 (1,642) | 104,592,744 (4,162,171) |  |

**Table L6. Game defaults, all-off -> HEAD, paired.**

| step | metric | base sum -> new sum | up/dn/tie by seed | sign p |
|---|---|---|---|---:|
| all-off -> HEAD, game defaults | food taken (cells) | 430 -> 180,483 | 24/0/0 | 0.000 |
| all-off -> HEAD, game defaults | J at nest | 2,115 -> 33,865 | 23/1/0 | 0.000 |
| all-off -> HEAD, game defaults | born | 58 -> 63,642 | 24/0/0 | 0.000 |
| all-off -> HEAD, game defaults | starved | 476 -> 4,418 | 18/6/0 | 0.023 |
| all-off -> HEAD, game defaults | old age | 6 -> 1,801 | 24/0/0 | 0.000 |
| all-off -> HEAD, game defaults | ants at end | 56 -> 33,542 | 24/0/0 | 0.000 |
| all-off -> HEAD, game defaults | food eaten (J) | 112,086 -> 104,592,744 | 24/0/0 | 0.000 |

**Table L7. Gap 140, all-off -> HEAD, paired.**

| step | metric | base sum -> new sum | up/dn/tie by seed | sign p |
|---|---|---|---|---:|
| all-off -> HEAD, gap 140 | food taken (cells) | 0 -> 4,192 | 24/0/0 | 0.000 |
| all-off -> HEAD, gap 140 | J at nest | 0 -> 194,654 | 24/0/0 | 0.000 |
| all-off -> HEAD, gap 140 | born | 0 -> 67 | 20/0/4 | 0.000 |
| all-off -> HEAD, gap 140 | starved | 479 -> 135 | 0/24/0 | 0.000 |
| all-off -> HEAD, gap 140 | old age | 1 -> 90 | 22/0/2 | 0.000 |
| all-off -> HEAD, gap 140 | ants at end | 0 -> 322 | 24/0/0 | 0.000 |
| all-off -> HEAD, gap 140 | food eaten (J) | 0 -> 755,656 | 24/0/0 | 0.000 |

What the tables say, in the bed's own terms:

- **All-off against HEAD measures the walk, and only the walk.** With every shipped switch put back, the founders
  die out on every one of the 24 seeds (477 of 480 starve, 3 die of old age, 0 are born, 14 cells of food are taken
  in all); at HEAD the same seeds take 5,513 cells, breed 181 and end with 442 ants, better on 24 of 24 seeds on most
  measures. `CHOOSER=off` **on its own** (the other thirteen shipped) reads the same in kind: no ants left and no
  births on 24 of 24 seeds (§10), so this 24-of-24 pair is guaranteed once the walk is on and says nothing about the
  other thirteen. The game's own breeding rules (Table L5/L6) and the far pile (L4) tell the same story for the same
  reason.
- **With the walk on, the other thirteen are a large gain of their own.** The walk-on arm (Table L1, bold row) takes
  2,130 cells, breeds 40, starves 254 and ends with 213 ants: adding the walk first, it brings about two fifths of the food gain
  (14 -> 2,130 of 14 -> 5,513) and halves the starving (with the walk off the thirteen leave the colony dead too, §10, so the split depends on the order). Putting the thirteen on top of it (= HEAD) takes food to 5,513 on
  24 of 24 seeds, births 40 -> 181 (21 of 24), starved 254 -> 116 (fewer on 20 of 24), ants at the end 213 -> 442 (22 of 24) and
  food eaten 24 of 24 (Table L3, last two steps). On the 72 independent seeds (125-196) the same step reads food taken 5,977 -> 16,675 (higher on 72 of 72 seeds), births 89 -> 482 (65 of 72), starved 837 -> 216 (fewer on 71 of 72) and ants at the end 543 -> 1,390 (71 of 72); the walk alone, all-off against walk-on, reads 59 -> 5,977 cells and 4 -> 543 ants.
- **The ladder is the other evidence for the rest, and the gain arrives in two places.** #485 -> #504 (the walk, crop,
  scouting and the forage drive: #486-#504) takes starved from 479 to 176 and births from 0 to 40, and #504 -> #513
  (the nest's shaft, heap cue and dig-down, the packed lunch, the birth price and the granary: #505-#514) takes births
  from 40 to 207 and starved from 176 to 113 (births 24 of 24 up).
- **No step after #513 separates on the bed.** #515-#518 (the returns drive, trip reach, the walked entrance held
  off, the west-lean fix; #514 merged just before #513) leave every metric inside the noise at 24 seeds (births
  207 -> 181, 8 up, 12 down, sign p 0.50; ants at the end 481 -> 442, 7/15, p 0.13; starved 113 -> 116, 12/10) and at 96
  (births 717 -> 663, 36 up, 50 down, 10 tied, p 0.16; ants at the end 1,856 -> 1,832, 36/49, p 0.19). That is a
  statement about what these tests can see: a loss of 10-20% on births is below it (§4), so it is not evidence that
  nothing moved.

**Checks on the fresh-seed result.** (1) The anchor `bb65d507` is an older binary run with no shipped switch in its
environment at all; its harness prints only food eaten (and it predates `BUD_SITE`, so its buds follow the old
rule), and it too eats almost nothing: 13,180 J over the 24 seeds (median 0) against 2,984 for all-off and 1,012,983
at HEAD (at 140 cells, 2,038 J over 96 seeds against 228 all-off), so the old ant's failure to feed itself on this bed
is not something the all-off arm's environment made up. (2) The arm's header echoes ten of the fourteen before-values it was
given (`SCOUT=0`, `BIRTH_PRICE=face`, `TRIP_REACH=None`, `NEST_DOOR=off`, ...; the bed harness never prints `CHOOSER`,
`SPOIL_CUE`, `DIG_DOWN` or `SPOIL_PACKS`), all fourteen came from one list under `env -i`, stderr is empty, and every
one-switch arm but `BIRTH_PRICE` (which the bed cannot see, §4) differs from HEAD on the bed (§10). (3) **The lanes' seeds may have been a
little kind.** Their baseline row (build before #517, seeds 1-24, 90 cells) reads starved 82, born 179, and #518's
body gives HEAD on those seeds as starved 57, born 188; HEAD on the review's four 24-seed blocks of fresh seeds reads
starved 116 / 70 / 84 / 62 and born 181 / 151 / 191 / 140 (the frozen primary block is the worst of the four on
starvation), and at 140 cells the fresh seeds run somewhat worse than the lanes' row (starved 130 and born 75 per 24
seeds against their 110 and 87). That is the direction a mild selection of seeds 1-24 would leave, and it shows in one
place: **#518's bed line ("starved 82 -> 57, fewer on 16 of 24") does not reproduce on fresh seeds**: the merge
before it against HEAD reads starved 115 -> 116 (11 more, 10 fewer, 3 tied), born 174 -> 181. It is small against the
stack's effect on the same seeds (starved 477 -> 116) and does not touch the conclusion.

**What this does and does not clear.** It clears "the week's defaults, together, are a net gain on the colony bed
and in the game's own regime". It does not clear each default separately (the ablations in §10 show three that cost
something at HEAD and a fourth that leans the same way, W10), it says nothing about the lab box (§3), and it is a statement about food and colony numbers,
not about how the nest looks (§5).

## 3. The lab box: does the stack hold in the lanes' own pre-ship instrument?

Four ships in a row carried a births estimate on the wrong side of zero that the sign gate scored
"not significant": #510 (17 seeds down of 24, p 0.064), #513 (12 seeds: 9 down, p 0.146), #515 (16 of 24,
p 0.152) and #516 (bed, 72 seeds: 33 down of 55, p 0.177). Taken one at a time each stayed under
the bar. Taken together they are the shape a hidden cumulative loss would have, and the lab box
(`labforage scenario=played_bed`, 120,000 frames) is the instrument the lanes named as the
pre-ship check, so the review ran the stack there on 12 fresh seeds (101-112), both arms of a seed
run together: **HEAD against HEAD with every shipped switch put back** (all-off), and **HEAD against
HEAD with the walk left on and the other thirteen put back** (walk on). The second pair is the one that
says anything about the thirteen: with the walk off the colony is weak on this box as it is on the bed
(peak 71 ants), so all-off against HEAD measures the walk (§2).

**All-off against HEAD** (`off` -> `head`, 12 seeds):

```
labpair: off -> head
keyed on (arm, seed): off 12 logs -> 12 seeds, head 12 logs -> 12 seeds; paired 12: [101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112]
  run: scenario=played_bed frames=120000 sample=900 | table rows per log 134..134
  header settings that differ between the arms: BIRTH_PRICE face->guaranteed, CARRY_PATIENCE off->pickup, FORAGE_DRIVE off->unset, NEST_DOOR off->shipped, PACKED_LUNCH off->on, SCOUT 0->shipped, STOREROOM off->on,workerhome,side,keep,caste=4, TRIP_REACH None->Some(16)

GATE -- pre-ship regression check. median off -> head; seeds where head is higher/lower; sign p
  births                                         150.5 ->       266.5    9/3   p 0.146   worse on 3/12 (lower is harm)
  food eaten, J (intake)                       379,878 ->     937,492   12/0   p 0.000   worse on 0/12 (lower is harm)
  ant-frames lived, millions                       5.8 ->         8.8    8/4   p 0.388   worse on 4/12 (lower is harm)
  died of old age                                109.5 ->       153.5    8/4   p 0.388   worse on 4/12 (lower is harm)
  starved per million ant-frames                   6.9 ->         3.8    4/8   p 0.388   worse on 4/12 (higher is harm)
    starved, raw (scales with ant-frames)         37.0 ->        39.5    7/5   p 0.774   context, not gated

CRASH TIMING -- not a gate: the box grazes out, so these say when the crash landed, not harm
  died out (alive=0 at the end)                      0 ->           0   boxes of 12
  under 10 at the end (incl. died out)               2 ->           0   boxes of 12
  alive at the end                                32.5 ->       141.0   11/1   p 0.006
  peak ants                                       71.0 ->       141.0   12/0   p 0.000
  frame of the peak                             51,750 ->     117,900   11/1   p 0.006
  fell below a quarter of peak (boxes)               4 ->           0   median frame 82,350 -> n/a; at/after 100,000: 1 -> 0
```

**And with the walk left on** (`x13`: the other thirteen switches back, 12 seeds):

```
labpair: x13 -> head
keyed on (arm, seed): x13 12 logs -> 12 seeds, head 12 logs -> 12 seeds; paired 12: [101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112]
  run: scenario=played_bed frames=120000 sample=900 | table rows per log 134..134
  header settings that differ between the arms: BIRTH_PRICE face->guaranteed, CARRY_PATIENCE off->pickup, FORAGE_DRIVE off->unset, NEST_DOOR off->shipped, PACKED_LUNCH off->on, SCOUT 0->shipped, STOREROOM off->on,workerhome,side,keep,caste=4, TRIP_REACH None->Some(16)

GATE -- pre-ship regression check. median x13 -> head; seeds where head is higher/lower; sign p
  births                                         460.0 ->       266.5    5/7   p 0.774   worse on 7/12 (lower is harm)
  food eaten, J (intake)                     1,147,194 ->     937,492    6/6   p 1.000   worse on 6/12 (lower is harm)
  ant-frames lived, millions                      10.1 ->         8.8    7/5   p 0.774   worse on 5/12 (lower is harm)
  died of old age                                148.0 ->       153.5    7/5   p 0.774   worse on 5/12 (lower is harm)
  starved per million ant-frames                  23.9 ->         3.8    1/11  p 0.006   worse on 1/12 (higher is harm)
    starved, raw (scales with ant-frames)        231.5 ->        39.5    1/11  p 0.006   context, not gated

CRASH TIMING -- not a gate: the box grazes out, so these say when the crash landed, not harm
  died out (alive=0 at the end)                      3 ->           0   boxes of 12
  under 10 at the end (incl. died out)               3 ->           0   boxes of 12
  alive at the end                               110.5 ->       141.0    9/3   p 0.146
  peak ants                                      237.5 ->       141.0    6/6   p 1.000
  frame of the peak                            102,150 ->     117,900    7/3   p 0.344
  fell below a quarter of peak (boxes)               5 ->           0   median frame 83,700 -> n/a; at/after 100,000: 2 -> 0
```

Both blocks stop before `labpair.py`'s closing "NOT GATED -- kept for the record" section. Medians, all-off -> HEAD (seeds where HEAD is higher): raw deliveries 1,323 -> 6,773 (12 of 12), pickups at home 1,444 -> 3,866 (11 of 12), deepest breeding generation 7 -> 15 (11 of 12). Walk-on -> HEAD: deliveries 5,968 -> 6,773 (6 of 12), pickups at home 5,641 -> 3,866 (3 of 12), deepest breeding generation 23 -> 15 (3 of 12). Net food into home is marked an overcount there and is left out.

**What the pairs say.** Against all-off the stack is a gain on everything the box reads (births 150 -> 266, up on 9 of 12; food eaten up on 12 of 12; peak
colony 71 -> 141; ants alive at the end 32 -> 141, 11 of 12), but that pair is decided by the walk (§2). **With the walk left on, the picture
is a trade and the births estimate is unresolved.** Births go 460 -> 266 in medians, but paired by seed the geometric-mean ratio is **x0.82 with a 95%
interval of x0.43 to x1.56** (5 seeds up, 7 down, sign p 0.77), and food eaten x0.97 (interval x0.58 to x1.60): a cumulative births loss of up
to about half is not excluded at 12 seeds, and neither is no loss. What is clear is the other side of the trade: **starvation per million ant-frames falls to about a fifth (paired ratio x0.22, interval x0.13 to x0.37; the medians go from 23.9 to 3.8), lower on 11 of 12 seeds (sign p 0.006), and boxes that die out go from 3 of 12 to 0**. What is not shown is that the colony is smaller: the peak colony is x0.87 (interval x0.51 to x1.48, higher at HEAD on 6 seeds and lower on 6), ant-frames lived x1.02 (x0.63 to x1.66), and the box ends with more ants on 9 of 12 seeds. The medians (peak 237 -> 141, births 460 -> 266) suggest a smaller colony and the paired ratios do not confirm it; the one ungated measure that leans that way is the deepest breeding generation (23 -> 15, lower on 9 of 12, p 0.15). The wiki says that "a colony now stays smaller and steadier: it raises half as many young, but far fewer starve, none of a dozen test boxes died out". "Half as many young" is the ratio of the medians (x0.58); paired by seed the estimate is x0.82 and its interval spans x0.43 to x1.56, so the pair neither confirms nor excludes it. The starvation half of that sentence is what the pair supports, and the gate scores the trade only through its starvation rate. Whether the owner wants that trade in the lab box is a judgment about how it should feel, which this review leaves to them.

What the review cannot say: **what each step costs at HEAD.** Of the four births estimates, one (#510) is a
demonstrated loss (paired log-t p about 0.006) and the other three are unresolved estimates on the wrong side of zero;
the pairs above cannot separate them, and the sign gate cannot either (§4). Two
facts bound the reading: the `played_bed` scenario itself changed inside #515 (the owner's test-bed review dropped the
`Clear` before the colony lands and added one tree), so lab baselines quoted before and after #515 are different boxes; and the
lab header echoes only eight of the fourteen switches (`BIRTH_PRICE`, `CARRY_PATIENCE`, `FORAGE_DRIVE`, `NEST_DOOR`,
`PACKED_LUNCH`, `SCOUT`, `STOREROOM`, `TRIP_REACH`), so for the walk, the shaft, the cue, dig-down, spoil packing and the
drop the arm is known by its effect, not by an echo.

## 4. Do the instruments measure what they are quoted for?

**The lanes' arithmetic is sound.** Every claim number that could be recomputed from
the per-seed logs committed under `Reports/data/` reproduced exactly: seven
metrics across #509, #510, #515 and #516, 24 paired seeds each, no unfinished runs
(reader A3's script over the committed logs, re-run by the coordinator with an identical output; `labpair`, `nestscore`,
`antloop` and `decisioncensus` also pass their own `--selftest`). What is wrong is
what the numbers are asked to carry.

**The lab's pre-ship gate cannot see the harms it is quoted against** (W1). It is an
exact two-sided sign test over 24 seeds: `p < 0.05` needs at least 18 seeds on one
side (17 to 7 gives 0.064). A sign test throws the size of each seed's difference
away, and these logs are noisy: the standard deviation of a paired log-ratio is
0.47-0.72 for food eaten and ant-frames, 0.7-1.1 for births and 0.6-1.3 for the
starved rate. At those spreads the gate flags a true 15% loss 5-15% of the time and a
true 25% loss 12-46% of the time, and it needs a true loss of 34% (spread 0.5) to 56%
(spread 1.0) to be 80% likely to fire at all (reader A3's simulation over the committed
logs; the coordinator's independent arithmetic on the exact sign-test distribution gives
the same picture to within simulation error). **So "no gate fires" is what the lanes' own rule
prescribes, and it is not evidence of neutrality**, yet the word is used that way
(#509: "neutral"). The worked case is #510: births per lab seed x0.62
(95% interval x0.45-x0.86, paired t on the logs -3.0, p about 0.006) against a gate
reading of 17 seeds down, 7 up, p = 0.064. The PR states the births drop, so it is not
hidden; it is the clearest instance of a real ~38% effect that the gate scored
"not significant". The paired intervals for the other shipped "no gate" arms are wide:
#515 births -46% to +23%, #509 births -13% to +43% and starved -13% to +80%, and
#516's first build's starved rate +1% to +63% (sign test 16 to 8, p = 0.15).

**And the bed cannot see the birth price at all.** With `BIRTH_PRICE=face` at HEAD the
bed is *identical on every panel metric* on 24 of 24 seeds (0 up, 0 down, 24 tied on each; the
arm's header echoes `BIRTH_PRICE=face`, and the BIRTHS line matches stop for stop on seed 101, the one compared), because
the bed has no plants, so no seed for the price to discount. "The bed decides"
therefore decides nothing for #510: its only evidence is the lab's births drop.

**The same split was read two ways** (Table T1). Store lunch was
held off at 17 to 7, p = 0.064 ("starved 137 -> 263, died out 1 -> 5"), while #510 shipped
at exactly that split. By the lane note's own rule neither reason for the hold counts:
raw starved is "context, not gated" (ant-frames rose 10.5 M -> 13.1 M, 18 to 6), and
"died out" is "crash timing, not harm". The hold itself stands: #515 re-held it on the bed
(starved 75 -> 215, 20 to 4, p = 0.0015). It is the recorded *reason* that does not
follow the recorded rule. Low severity; the direction of the mistake is the safe one.

**Table T1. The threshold, as applied** (sign p is exact and two-sided; "rule" is the foraging
lane note: only an opposite sign at p < 0.05 counts against a change).

| PR | evidence | split | n | p | decision | consistent with the rule? |
|---|---|---|---:|---:|---|---|
| #508 | bed starved 83 -> 152 | 18 worse / 6 | 24 | 0.023 | shipped, owner ruling on nest steps | yes, harm stated |
| #509 | lab: 13/11, 13/11, 11/13, 14/10; died out 1 -> 3 | even | 24 | >= 0.54 | shipped "neutral" | yes; "neutral" overreads it |
| #510 | lab births 608 -> 408 | 17 worse / 7 | 24 | 0.064 | shipped | yes; the effect is real (log-t p 0.006) |
| #513 | lab, 12 seeds: births, alive, ant-frames fewer on 9 | 9 / 3 | 12 | 0.146 | shipped, owner "Full granary" | ran 12 seeds, where 9/3 cannot reach 0.05 |
| #513 | bed, 80 founders: starved 848 -> 1,102 | 20 worse / 4 | 24 | 0.0015 | shipped, stated | yes |
| #514 | lab store lunch: raw starved 137 -> 263, died out 1 -> 5 | 17 worse / 7 | 24 | 0.064 | **held off** | **no**: gated metrics improved 18/6 (p 0.02) |
| #515 | lab births 481 -> 428 | 16 worse / 8 | 24 | 0.152 | shipped | yes |
| #515 | bed, store lunch: starved 75 -> 215 | 20 worse / 4 | 24 | 0.0015 | held off | yes |
| #516 | bed births 562 -> 484, 72 seeds | 33 worse / 22 | 55 | 0.177 | shipped | yes |

**The nest scorer has three holes that all point the same way** (W6). `scripts/nestscore.py`
`paired()` (lines 314-336) (a) skips any seed whose colony is empty at the stop, and
`digbox` prints `(fewer than 10 dug cells: not scored)` for exactly the seeds where an arm
stopped the digging, so an arm that halts digging **loses** those seeds instead of scoring
worse on them (`digbox.rs:1006-1009`, `nestscore.py:132-142`); (b) with `--base` and no
`--stop` it pairs at the latest frame *any* run reached, so an arm whose runs end earlier
(or are still being written) reads `0/0` on every metric with no warning
(`nestscore.py:412-418`); (c) it prints up/down counts without `n` or the skip count. It
also scores entrances as `|mouths - 1|`, the one-entrance PRs' own target, and pools seven
metrics of different kinds (shares and absolute rows) so that digging more can win on
volume. `#508`'s headline ("more nest-like than random digging on 22 of 24 against 12")
counts seeds whose panel median against the walkers null is at least 0.9, over a per-seed
varying subset of the seven metrics; **no nest-lane `digbox` log is committed**, so it
could not be recomputed without mouths and roofed from the record (the review re-ran the
nest instrument at HEAD instead: §5). Positive control not present: the selftest checks
parsing and duplicate-key refusal only.

**Three scripts have no selftest at all**: `antidle.py`, `drivefade.py` and `tripsrc.py`
crash on `--selftest` (`ZeroDivisionError`, `IndexError`, `IndexError`), and #516's
"returns booked away from the pile 629 -> 20" rests on them. `drivefade.py` keys its parse
on the seed alone (the gap comes from nowhere) and `tripsrc.py` pools every CSV in a
directory with no key and no cardinality print, the shape CLAUDE.md warns about; the traces
were not committed, so it cannot be shown that any pooling happened.

**Provenance of the lab arms** (W8, records). #509's treatment logs (`m4on`) echo three header
keys that its baseline logs (`m2off`) do not (`COLONY_SPACING`, `STACK_DEPTH`, `BUD_SITE`; re-checked by the coordinator
on the committed archive), so they came from two different `labforage` binaries although the header says "one binary per
comparison"; the numbers themselves reproduce. And #515's shipped-drive lab arm (`ret`) and #516's "off = main's ant" arm are the
same configuration by their own records: **23 of 24 seeds are byte-identical on the
`p_move_hist` summary and seed 9 differs** (n 1,110,952 against 1,110,856; the coordinator
re-ran this comparison on the committed archives). Either #515's committed logs came from
an intermediate build or `TRIP_REACH` unset is not bit-exact with `main`'s ant on one seed
in 24. Cause not established (a run of seed 9 at `ac8644fe` against `59338afd` with
`TRIP_REACH=off` would settle it).

## 5. The nest: which way it leans, and where the week left it

The nest lane judges a step on the dig box's own measures (the lane note's gloss of the owner's
2026-09-27 "it is okay if nest building temporarily hurts colony numbers"), so this section reads `digbox` (40 ants, `energy=1000`, no food, 24,000
frames, the lane's own setting) on the same fresh seeds 101-124, at HEAD, under `env -i`.

**It still leans west, on every seed, and the storeroom is all of it** (W2). "West of the door" is
`digbox`'s signed centre of the middle half of the dug cells, `p50x`, negative = west.

| Arm (HEAD, 40 ants, 24 seeds) | west / east / centred | median centre | sign p (west vs east) |
|---|---:|---:|---:|
| shipped default | **24 / 0 / 0** | **-11 columns** | 1e-7 |
| `STOREROOM=off` | 10 / 12 / 2 | +1 | 0.83 |
| paired, storeroom off against on | 23 of 24 seeds move east | +9.5 columns | |

So the instrument reads a centred nest when nothing biases it (the negative control) and
-11 columns when the storeroom does. At 200 ants the default is 17 west, 6 east, 1 centred
(median -4, p 0.035). #518's own paired numbers replicate on fresh seeds (22 of 24 west at 40
ants there, 15 of 24 at 200); what does not hold is its title. **Why:** the storeroom's side is a
constant, `let side = storeroom_of(self).side.then_some(if off > 0 { 1 } else { -1 })`
(`creature.rs:5123`) with `off` the shaft offset, which is 0 by default, so every shipped
nest cuts its storeroom to the west. The stated reason (the comment above it, `creature.rs:5120-5122`: "west of one under it -- away from the
food on the colony bed, which lies east") is a property of the colony bed. #518 removed the half turn's fixed side because "a fixed side is
still a bias" and then kept this one. Consequences: any world with food to the west has its
storeroom cut *toward* it, and a colony founded within about ten columns of the west edge gets its room
clipped (`x - 11 .. x - 5` at the default sizes; writes off the world are no-ops, and the room is
still recorded if one passage cell was cut; read, not run).
Smaller constant-side ties in the same code (reader A2's reading, spot-checked against `NEIGHBOURS_8`'s order): the default two-wide shaft spans columns `x..x+1`, half a
column *east* of the door's centre; `storeroom_drop_site` takes the first lowest neighbour in
`NEIGHBOURS_8` order (SW before S before SE); `food_drop_site`'s search runs NW, N, NE, W before E;
`lift_out` (off) and `store_post_site` (off) prefer the west. The class is the one
`.claude/rules/src-sim-cells.md` records as hit four times and never once caught by a test;
this week's first instance, `turn_toward`'s half turn, shipped in #508 with every up-facing digger
turning west and was caught by the foraging lane's review, not by a test (§11 control).

**Along the week, the lane's own instrument says the nest got less nest-like**
(`digbox`, each rung's own binary, so comparable on trend, not to the digit).

**Table N1. The nest lane's instrument along the week** (`digbox`, 40 ants, no food, 24,000 frames, fresh seeds 101-124; each rung is its own commit's binary, so comparable on trend, not to the digit: the #513-to-HEAD step also crosses #517's harness edit, which moved digbox's digs on its own, §8).

| arm | n | entrances, median (mean) | cells dug, median | centre: west / east / centred (median) | nest-like vs random walkers, all metrics | ... without entrances and roofed share | note |
|---|---:|---:|---:|---|---:|---:|---|
| `ffbac01e` #506 (before the shaft and cue ship) | 24 | 12.0 (12.3) | 1352 | 13 / 10 / 1 (-1) | 0 of 24 | 0 of 24 |  |
| `7933010e` #512 (after #507, #508) | 24 | 3.0 (3.0) | 248 | 16 / 6 / 2 (-1) | 20 of 24 | 17 of 24 | the one-entrance nest |
| `72383e39` #513 (granary on) | 24 | 6.0 (5.9) | 436 | 24 / 0 / 0 (-12) | 0 of 24 | 0 of 24 |  |
| `2274e347` HEAD | 24 | 6.0 (5.6) | 600 | 24 / 0 / 0 (-11) | 0 of 24 | 0 of 24 |  |

The granary (#513) is where it turns: on the review's seeds entrances go 3 to 6 and "more
nest-like than random digging" 20 of 24 to 0 of 24. **#513's body says the same on the lane's seeds**
("7 entrances against 2", "0 of 24 against 21") and the owner ruled "Full granary on my default", so
this is a disclosed, ruled trade of nest shape for a colony that eats and breeds (§2: births 40 to 207). The finding is
about what did not follow: #507's "One entrance by default" and #508's "One entrance for the whole run"
stand as titles, the dig-down doc still quotes "2 against 6", and the ship-on evidence for #507 and #508
was measured on a nest that no longer exists. 

**The mechanisms themselves are intact at HEAD.** Putting one nest switch back (Table N2) shows each doing what it
was shipped for: without the heap cue the colony opens 2 more entrances (8 against 6; more on 19 of 24 seeds, fewer
on 1, p < 0.001) and digs 42% more; without dig-down, 3 more (9 against 6; 21 of 24 seeds) and digs *less* (444 cells
against 600, on every seed); with spoil packing back, 2.5 more (21 of 24). The door and the storeroom are the ones that
matter for the granary's cost: **either half alone brings the entrances back** (door alone 7, storeroom alone 6, both 6),
and only with both put back does the 3-entrance nest return (2.5 entrances, fewer on 22 of 24 seeds, p < 0.001). **The
instrument's headline survives the review**: in that granary-off nest 20 of 24 seeds read more nest-like than random
digging, and still 18 of 24 with the entrance count and the roofed share taken out of the panel (17 of 24 at #512), so #508's
"22 of 24" was not the scorer rewarding its own target. Its evidence is simply for a nest the granary has since replaced.
All six nest switches back at once gives 18.5 entrances and no nest-like seed (the #506 rung, on the old harness, read 12 entrances).

**Table N2. HEAD with one nest switch put back to its before-value** (same box, same 24 seeds; entrance counts paired against the HEAD row).

| arm | n | entrances, median (mean) | cells dug, median | centre: west / east / centred (median) | nest-like vs random walkers, all metrics | ... without entrances and roofed share | note |
|---|---:|---:|---:|---|---:|---:|---|
| HEAD, as shipped | 24 | 6.0 (5.6) | 600 | 24 / 0 / 0 (-11) | 0 of 24 | 0 of 24 |  |
| `SPOIL_CUE=off` | 24 | 8.0 (8.0) | 852 | 22 / 2 / 0 (-7) | 0 of 24 | 0 of 24 |  |
| `DIG_DOWN=off` | 24 | 9.0 (9.0) | 444 | 21 / 3 / 0 (-6) | 0 of 24 | 0 of 24 |  |
| `SPOIL_PACKS=on` (spoil packs, the before) | 24 | 8.5 (9.0) | 655 | 22 / 2 / 0 (-8) | 0 of 24 | 0 of 24 |  |
| `NEST_DOOR=off` (the 53-column strip) | 24 | 6.0 (6.0) | 548 | 20 / 4 / 0 (-6) | 8 of 24 | 2 of 24 |  |
| `STOREROOM=off` | 24 | 7.0 (6.7) | 626 | 10 / 12 / 2 (+1) | 0 of 24 | 0 of 24 | the only arm that centres the nest |
| door **and** storeroom off (the granary put back) | 24 | 2.5 (2.5) | 242 | 13 / 10 / 1 (-1) | 20 of 24 | 18 of 24 | reproduces #512's nest |
| `NEST_SHAFT=off` (door still on) | 24 | n/a | 0 | 0 / 0 / 24 (+0) | 0 of 0 | 0 of 0 | digs nothing: see §8 |
| all six nest switches off | 24 | 18.5 (18.7) | 1272 | 12 / 12 / 0 (+1) | 0 of 24 | 0 of 24 | no seed nest-like |

## 6. Do the guards guard?

A green suite is evidence about the tests, not the code, until a test has been watched going red for
the fault it is named for. CI is green on `main` after all 34 merges, so the question here is
narrower and cheaper to state than "do the tests pass": **if someone put a shipped default back to its
old value, would any test say so?** The review answers it without editing a line of source. It builds
the test binary once, and for each of the 14 shipped switches runs the tests with that switch at its
before-value (one process per value, since the switches are read once per process), against a control
at the default environment that must be all green: it was (68 of 68 new tests, 1,947 of 1,947 in the
whole lib suite).

| switch (put back to its before-value) | red among the 68 new tests | red in the whole lib suite (beyond the default run) | reading |
|---|---|---|---|
| `CHOOSER` | 0 | 0 | **unpinned: nothing goes red** |
| `SCOUT` | 1: `a_declined_load_books_nothing_and_counts_once` | skipped: a new test already goes red | pinned (a new test) |
| `CARRY_PATIENCE` | 0 | 0 | **unpinned: nothing goes red** |
| `FORAGE_DRIVE` | 0 | 0 | **unpinned: nothing goes red** |
| `TRIP_REACH` | 0 | 0 | **unpinned: nothing goes red** |
| `PACKED_LUNCH` | 0 | 0 | **unpinned: nothing goes red** |
| `BIRTH_PRICE` | 0 | 0 | **unpinned: nothing goes red** |
| `NEST_SHAFT` | 1: `a_founding_digs_with_the_founders_own_jaw` | skipped: a new test already goes red | pinned (a new test) |
| `SPOIL_CUE` | 0 | 0 | **unpinned: nothing goes red** |
| `DIG_DOWN` | 0 | 0 | **unpinned: nothing goes red** |
| `SPOIL_PACKS` | 0 | 0 | **unpinned: nothing goes red** |
| `NEST_DOOR` | 0 | 0 | **unpinned: nothing goes red** |
| `STOREROOM` | 0 | 0 | **unpinned: nothing goes red** |
| `DROP_REACH` | 0 | 0 | **unpinned: nothing goes red** |

**How to read a green.** Most of the week's new tests set a `World` field for the mechanism they name
(`w.chooser`, `w.nest_shaft`, `w.spoil_cue`, `w.storeroom`, `w.nest_door`, `w.dig_down`, ...), which
overrides the environment, so an environment flip cannot reach them *by construction*. 31 of the 68
pin a field in their body; the other 37 either take the arm as an argument (the parser tests,
`spoil_cue_factor(..., cue)`, `trip_reach_bite(..., on)`) or read the process default, and the flip run
says which: two of them went red (`a_declined_load_books_nothing_and_counts_once` under the `SCOUT` flip,
`a_founding_digs_with_the_founders_own_jaw` under the `NEST_SHAFT` flip). Pinned and argument-taking tests
are good unit tests, and they are also why "the new tests stay green when the mechanism is flipped" is the
expected result and not by itself a finding. The finding is narrower: the shipped defaults themselves. Putting each of `CHOOSER`, `CARRY_PATIENCE`, `FORAGE_DRIVE`, `TRIP_REACH`, `PACKED_LUNCH`, `BIRTH_PRICE`, `SPOIL_CUE`, `DIG_DOWN`, `SPOIL_PACKS`, `NEST_DOOR`, `STOREROOM`, `DROP_REACH` back to its before-value turns no test red, in the 68 new tests or in the whole 1,947-test suite; `SCOUT`, `NEST_SHAFT` are each pinned by one new test. Every test that touches these mechanisms pins its own arm, so the suite checks that each arm works and never checks which arm ships. Positive control: with the five pins off, four old-walk tests fail at the default environment and pass under `PIXEL_PHYSICS_CHOOSER=off`, so an environment flip does reach the world and would have turned a test red had one asserted the shipped walk. That each of the other flip values took effect in the test process is shown indirectly, because the runner does not echo the environment: the same strings, read by the same parsers, change the bed's outcome for 13 of the 14 arms (§10), and the lab's header echoes `BIRTH_PRICE=face`, the one arm the bed cannot see.

**The shipped walk is the clearest case, and it needed no flip to see.** #487 made the trail-away
walk the default and pinned every guard that needed the old walk's mechanics to `Chooser::Off`: five in
`creature.rs` and one in `specimen.rs`. The review took the five in `creature.rs` off (a scratch build, never
committed; the `specimen.rs` one was not tested): of the 2,037 lib tests in that build (the suite's 2,035 and two scratch tests of the review's own), 1,944 passed, 88 were ignored, and the 5 that
failed were the four unpinned tests plus the review's own deliberate one. All four are tuned to the old walk and correctly pinned, two by
mechanism: `every_traced_decision_agrees_with_the_counters_and_the_positions` ("a tumble roll is taken exactly
when the move roll fails") and `the_homeward_re_roll_aims_along_a_known_floor...` ("0 tumbles asked and 0
fired": the shipped walk has no tumble to re-roll, so its own vacuity guard fires); and two by a measured scene
number that moved, `a_maximally_armoured_ant_is_graded_only_when_the_reach_allows_it` ("3 of 6 survived against a
measured 2") and `a_lone_grazer_cannot_farm_a_moss_lawn_forever` (37 mouthfuls against 31).
`every_lifetime_counter_closes_against_its_world_total` passes unpinned. So this is **a coverage gap, not
a defect**: the walk every ant ships with has **no unit-level test that its decision trace agrees with
its counters and positions**, and the comments say `trailfollow decisioncsv` does that job, which CI
does not run. It is also why the old walk's mechanics are still tested at all: only by pinning.

Other guard facts, each read from the diffs and checked against the flip run:

- **The heap-cue tests cannot tell the shipped ant from the ant before it.** `the_heap_cue_*`
  (#506, #512) call `spoil_cue_factor(&w, from, to, 2, cue)` with the cue passed in, so they test the
  rule as a function; none of the 68 tests applies the cue *through the ant*, and the
  `SPOIL_CUE=off` flip turned none of them red.
- **#517 fixes three walk faults with no test** (ledger `tests_added = 0`); the walked cycle it
  fixes is off by default.
- **`#508`'s dig-down gate** (enclosed-only), the one thing separating the shipped arm from the arm with
  the measured starvation harm, had no guard until #518's `the_dig_down_turn_is_an_enclosed_diggers_alone`
  (which the lane watched go red with the gate removed).
- **The tests that assert exact numbers were moved twice in the week, and both moves are explained**:
  `two_colony_bed` 85/115 -> 90/110 (#485: a positive control went red when the bed moved) and a crop weight `1.5 -> (2 + food_weight)/2` (#490). Of 729
  assertion-message literals in `creature.rs` at the baseline, three no longer appear at HEAD, all
  edits (one vacuity bar for the shipped door lowered from more than 4 to at least 3 painted cells; two
  renames). Nothing was deleted outright.
- **Best-guarded PR of the week: #516**: eleven tests, nine of them trip tests pinned through a helper that
  takes the arm, so they can tell shipped from off. #510's `a_birth_never_overdraws_its_parent_on_seeds` pins both arms
  with a face-value control.

## 7. Do the records say what shipped?

Two thirds of what a later session reads as "the bar" is prose, so a claim that
outruns its table matters more here than in most repositories. Every row below but (i) was read
against HEAD by the coordinator (CONFIRMED = the two lines were put side by side).

| # | The record says | HEAD says | Where | Severity |
|---|---|---|---|---|
| a | **"The nest stops leaning west"** (#518's title; the nest lane note's card line "the nest no longer leans west"; the wiki says "the galleries fan down both sides of the door; the storeroom still lies to the west", accurate about the storeroom and generous about "both sides") | west of the door on **24 of 24** seeds at 40 ants, median 11 columns (§5); #518's own body says 22 of 24 (median 8) and 15 of 24 at 200 ants, and that "the lean that is left is the storeroom" | `Reports/lanes/nest-mouth.md:182`, `wiki/ants.md:21-25`, `claims/518` | **false headline** (body honest; title and the lane note's card line not) |
| b | **"One entrance by default"** (#507), **"One entrance for the whole run"** (#508); the dig-down doc's "openings 2 against 6" | median **6 entrances** at 40 ants at HEAD (min 2, max 7, 24 seeds), and 0 of 24 seeds "more nest-like than random digging" (was 22 of 24 at #508): the granary (#513) undid it, and #513's body says so ("7 entrances against 2", "0 of 24 against 21"); the two titles and the dig-down/cue docs were not re-scoped | `creature.rs` dig-down doc, PR titles | **stale record** (disclosed once, not carried) |
| c | the nest lane note's live question: "43 and 70 cells against the lift's 207.5 and 613.5", "births lower (160 against 223)" | those are #517's own numbers for an older ant; after #516 and #518 the default reads 260 / 583.5 cells dug and the bed's births 179-188 | `Reports/lanes/nest-mouth.md` (Live question) | stale numbers |
| d | `SPOIL_CUE` doc: "Unset or `off`, nothing is read and no draw is taken, so the ant is bit-exact" | unset and `on` both parse to the shipped cue (`"" \| "on" => Some(SPOIL_CUE_SHIPPED)`, `creature.rs:10622`) | `creature.rs:10522-10524` | stale doc: a reader who leaves it unset "for a control" runs the cue |
| e | `World::chooser`: "`None` follows the environment, which is off unless set"; `World::scout`: "which is 0 (no pull) unless set" | since #487 and #494 the defaults are the trail-away walk and gain 2.0 | `world.rs:3655-3656`, `world.rs:3701-3703` | stale docs: a guard author reading them takes `None` for the old ant |
| f | README dated bullet pairs `PIXEL_PHYSICS_FORAGE_DRIVE=always` with `ForageDrive::SHIPPED` | `SHIPPED` is `Returns` (`creature.rs:15074`); `ALWAYS` is the form that shipped 2026-09-27 to 09-29 | `README.md:8522` | nit (dated log bullet) |
| g | foraging lane note: store lunch sends "32% of ant-time west against 14%" (§22t) | §22t says "41% of ant-time is west of the nest against 13%" | `Reports/lanes/foraging-loop.md:104` vs `Reports/ant-scenes-2026-09-23.md:2751` | nit (sizes the lane's top open problem from a figure its own report does not contain) |
| h | the owner's words, recorded: "It is okay if nest building **temporarily** hurts colony numbers" | every paraphrase drops "temporarily" ("colony numbers do not block a nest step"; `SPOIL_PACKS` comment); #508 then ships a standing default with bed starved 83 -> 152 and born 59 -> 30 | `Reports/lanes/nest-mouth.md:31`, `creature.rs` `SPOIL_PACKS` doc | worth an owner question: does the ruling cover a standing default? |
| i | #490 records the owner's "Let's test it out" as a default-on ruling; the 09-25 to 09-29 rulings (scouting card 09-26, granary card 09-29 have no response in the queue) cannot be checked outside the sessions' own notes; the owner's 2026-09-29 ruling is to answer in chat, not the queue (recorded by #522, after the pin), so the queue's silence is not evidence that a ruling was not given | `review.py inbox` and `origin/review-queue` hold 493 responses, the newest ant one 2026-09-24 (reader A4; not re-counted) | `claims/490`, review queue | PLAUSIBLE: owner to confirm wording |
| j | #508: "six wider refusals were measured and every one lost the nest" | the table lists five plus the shipped row; two of the five ran 12 seeds and do not separate (p 0.15-0.73); every wider refusal starved *fewer* on the bed (98-135 against 143), stated in the table, absent from the headline | `claims/508` | nit |
| k | `spoil_switches_line()` (the harness header) prints `dig_down_bias()`, the environment's value | a world-field arm (`World::dig_down`, set by one test) would print the environment's arm; no example sets it | `creature.rs:10719-10729` | nit (the stale-label trap its own doc says it exists to prevent) |

**Verified clean here:** the wiki's own "What is not right yet: the door is not a roof" paragraph (`wiki/ants.md:845-853`) states the granary's entrance problem plainly, and its "one entrance" lines are dated history; the 16-row switch table of `Reports/how-the-ant-works.md` §12
matches the code defaults row for row (reader A4, checked against `defaults.tsv`);
bug entries Z33, Z35 and Z36 match their PRs and Z34 is still open, as #494 says
(its starved numbers stand alone); `dead-ends.md` entries the PRs cite exist;
`wiki/ants.md`'s freshness note is a real date; `docscheck.sh` is clean and
`bugindex.py --check` current at HEAD; **16 new reports, all indexed** (plus one data note under `Reports/data/`).

## 8. Switch hygiene, determinism and cost

**"Unset reproduces the parent" held on every PR that could be isolated, three of four tried, on a small sample.**
For the four "(switch, off)" PRs the review built each parent and merge commit, ran them
at the default environment under `env -i` on colony-bed seeds 101-104 (24,000 frames) and in
`digbox` (40 ants, seed 101), and diffed the run signatures, **with a positive control**: the
merge with its own switch turned on must differ from the parent, or the comparison cannot tell.
The four controls all differed, as they had to; identity held for the three PRs that could be isolated (#506, #514, #517).

| PR | switch | bed (4 seeds) | digbox (1 seed) | note |
|---|---|---|---|---|
| #506 | `SPOIL_CUE` | identical | identical (gains ledger lines only) | |
| #514 | `HAUL_BITE`, `STORE_LUNCH` | identical but for one new counter on the BIRTHS line | identical | |
| #517 | `SPOIL_OUT` | identical | *differed*, then identical | the first digbox comparison said "differs on 7 of 7 runs" (digs +17 to +83%): that was #517's own harness edit (founders now homed at the door, disclosed in its body), not the engine: the #517 engine built with the *parent's* `digbox.rs` gives byte-identical output on 5 of 5 runs (40 ants seeds 1, 2, 3, 101; 200 ants seed 1), so **digbox numbers before and after #517 are not comparable** |
| #512 | `STOREROOM` | not testable | not testable | its final tree also ships the heap-cue fix ON (intended), so parent-vs-merge cannot isolate the off switch; its "line for line" proof was taken on earlier branch commits |

**Determinism holds.** The same HEAD binary twice, and `RAYON_NUM_THREADS` 1 against 4, give
identical bed logs (four seeds); the digbox repeats and 1-against-4 agree as well; and two independent digbox runs of the
default at 40 ants, taken an hour apart for different purposes, agree on 24 of 24 seeds byte for byte apart from timing lines.

**The switch parsers fail open, silently in four cases** (`defaults.tsv`, each read from
the parser). A mistyped or misremembered spelling does not turn the arm off; it runs the
shipped one, and in an A/B the arm and its control are then the same run wearing two labels.

| Switch | What a typo does | Warning |
|---|---|---|
| `CHOOSER` | any unrecognised value is the shipped walk | none |
| `SCOUT` | any non-number reads as gain 2.0, so `SCOUT=off` leaves scouting **on** (the docs say `0`) | none |
| `SPOIL_PACKS` | only the exact string `on` enables packing | none |
| `DROP_REACH` | only the exact string `adjacent` restores the old drop | none |
| `CARRY_PATIENCE`, `FORAGE_DRIVE`, `TRIP_REACH`, `PACKED_LUNCH`, `BIRTH_PRICE`, `NEST_SHAFT`, `SPOIL_CUE`, `NEST_DOOR` | read as shipped | `eprintln!` only |
| **`DIG_DOWN`** | a typo in the *qualifier* (`1.0,enclsoed`) selects **turn anywhere**, the arm the doc keeps off for a measured harm (bed starved 83 -> 295), and a test **pins that fallback** (`creature.rs:37731`) although the parser's own rule is "unreadable reads as shipped" | `eprintln!` only |

(One naming trap in the same family: `HAUL_BITE=off` *enables* its new rule, since the switch names the ant before it; the review's all-off arm leaves it unset for that reason.)

The harnesses echo most resolved values in their header (`trailfollow` ten of the fourteen switches,
`labforage` eight, `digbox` its nest switches), which is what made the review's arms checkable; for the rest the check
was that each one-switch arm differs from HEAD. Low severity on its own; it is the reason every review arm ran under
`env -i` with an allow-list.

**Randomness.** This week's one new stream, `RNG_SLOT_HALF_TURN = 9`, is unique among the
slot constants and keyed `(seed, organism, frame)`, so it moves no other draw; end to end the
coin is fair (with the storeroom off the nest centres: 10 west, 12 east, 2 centred, p 0.83,
§5). `RNG_SLOT_OLD_AGE` and `RNG_SLOT_NEST_SCENT` are both `8`, as #518 notes; that is identical at the
baseline and the keys differ, so it is a pre-existing nit and not a finding against these PRs.

**Environment reads.** 28 `env::var` sites were added to `src/` this week; **27 sit inside a
`OnceLock`**. The one that does not is `plant::guaranteed_bite_fraction`
(`plant.rs:2647`, `PIXEL_PHYSICS_SEED_CARGO`): it is reached only for a bare seed cell
within reach of a parent that is deciding whether to bud, so it costs almost nothing, but it is
the one read that takes the environment lock at play time.

**Frame cost** (a hard constraint here). The week's new per-frame work is `World::step_nest_need`, which under the
shipped `returns` drive is O(1) per frame (it resizes on site growth, clears, returns; reader A1), and the
nest-room census (`step_nest_room`, about 150,000 world reads every 256 frames) is baseline code that #493 only re-documented,
so its spike is unchanged. New per-event work: `surface_curvature` on each dig roll of an enclosed digger
not already facing down (up to 4 `traits_of` lookups a roll), a second `provisions_in_reach` scan per bud try,
and `trip_source()` per non-home bite (O(nest sites), feeding counters only); `lift_out`'s 4,096-cell search is off by default.
None is on a per-cell path of the sweep. What *is* new per ant per tick is the shipped walk (a weighted choice over
every usable heading, the trail read, the fall check) in place of the old step-and-tumble the baseline ant walked.

**Measured:** `antcost` (the lab box replayed with a standing ant count, arms round-robin inside one run, minimum
over reps), the baseline `bb65d507` against HEAD, **six alternating pairs** (runs 1-3 at 0/200/600/~1,200 ants, 4 reps;
runs 4-6 at 0/200/600, 6 reps, the 1,200 arm dropped because stocking reaches different populations), each binary
run from its own commit's checkout, `RAYON_NUM_THREADS=4`, with every other review job paused (SIGSTOP) for both
windows; the review's own table scripts ran during HEAD's run 2, which holds the 5,019 µs arm.

| binary | run | 0 ants | 200 ants | ~600 ants | ~1,200 requested | fit: empty bed + per ant |
|---|---:|---:|---:|---:|---:|---|
| baseline `bb65d507` | 1 | 2,204 (0) | 2,892 (202) | 4,396 (599) | 6,144 (1078) | 2,183 + 3.67 µs |
| baseline `bb65d507` | 2 | 1,917 (0) | 2,866 (202) | 4,464 (594) | 5,806 (960) | 1,992 + 4.03 µs |
| baseline `bb65d507` | 3 | 2,007 (0) | 2,932 (202) | 4,406 (598) | 5,902 (960) | 2,050 + 4.00 µs |
| baseline `bb65d507` | 4 | 1,914 (0) | 2,840 (202) | 4,275 (598) | - | 1,969 + 3.90 µs |
| baseline `bb65d507` | 5 | 1,774 (0) | 2,799 (202) | 3,964 (590) | - | 1,890 + 3.62 µs |
| baseline `bb65d507` | 6 | 2,119 (0) | 2,889 (202) | 4,428 (596) | - | 2,114 + 3.88 µs |
| HEAD `2274e347` | 1 | 2,009 (0) | 2,910 (200) | 4,299 (600) | 5,231 (876) | 2,089 + 3.63 µs |
| HEAD `2274e347` | 2 | 2,020 (0) | 2,849 (200) | 5,019 (594) | 5,636 (876) | 2,071 + 4.33 µs |
| HEAD `2274e347` | 3 | 2,163 (0) | 2,956 (200) | 4,703 (600) | 6,164 (876) | 2,092 + 4.54 µs |
| HEAD `2274e347` | 4 | 1,955 (0) | 3,018 (200) | 4,548 (600) | - | 2,039 + 4.25 µs |
| HEAD `2274e347` | 5 | 1,832 (0) | 2,914 (200) | 4,510 (602) | - | 1,914 + 4.38 µs |
| HEAD `2274e347` | 6 | 1,959 (0) | 3,267 (200) | 5,022 (587) | - | 2,071 + 5.12 µs |
| baseline median | | **1,962** (n=6) | **2,877** (n=6) | **4,401** (n=6) | **5,902** (n=3) | 2,021 + 3.89 µs |
| HEAD median | | **1,984** (n=6) | **2,935** (n=6) | **4,626** (n=6) | **5,636** (n=3) | 2,071 + 4.36 µs |

(µs per tick, minimum over reps of 300 frames: 4 reps in runs 1-3, 6 reps in runs 4-6, which skip the 1,200 arm; the measured standing ant count in brackets)

| ants | baseline median | HEAD median | HEAD / baseline (medians) | pairs with HEAD slower |
|---:|---:|---:|---:|---:|
| 0 | 1,962 | 1,984 | 1.011 | 4 of 6 |
| 200 | 2,877 | 2,935 | 1.020 | 5 of 6 |
| 600 | 4,401 | 4,626 | 1.051 | 5 of 6 |

The fitted cost per tick is `empty bed + per-ant x ants`: intercept 2,021 (baseline) against 2,071 µs (HEAD), **slope 3.89
against 4.36 µs per ant per tick (+12%)**, with five of HEAD's six slopes above the baseline's highest (4.03). The `min/median` over reps reads 0.76-0.98
across arms (1.00 is perfectly quiet), so single arms are noisy by up to a quarter and the medians over six pairs carry the
reading. So: **a per-ant cost increase of a few percent (5% at 600 ants, 12% on the slope) is likely but not established: HEAD is slower in 5 of 6 pairs at 200 and at 600 ants (sign p 0.22 each), the ranges overlap, and the six HEAD slopes rank above the six baseline slopes at rank-test p about 0.04 (exploratory; the highest, run 6, has no recorded disturbance while run 2 has one). It is consistent with
the shipped walk doing more per ant than the old one, and was not attributed** (a `CHOOSER=off` arm of `antcost` would
say how much of it is the walk). It is not a per-cell sweep cost: at 0 ants the two are within about 1% (+1.1%, HEAD slower in 4 of 6 pairs). What was not measured:
the worst frame at 512x320 outdoors with a large colony (the world will grow), and `examples/ascii`'s CI numbers; the review
demoted them for `antcost`, whose axis is the standing ant count.

## 9. All 34 PRs

Verdicts: **clean** (nothing found), **note** (something worth knowing, no action needed), **finding** (one of W1-W11 applies). `W` numbers are §1's.

| PR | line | what | src + asset lines changed | tests | verdict | notes |
|---|---|---|---:|---:|---|---|
| [#485](https://github.com/sgladstein/Pixel_Physics/pull/485) | walk | walk reference, trace, scenes, the chooser (off); Z33 crumbs fix and drop-through-bodies **on** | 2,836 | 16 | note | owner-ruled drop stopgap searches without a depth cap, west first (W11); W10: at HEAD putting the old adjacent drop back leans better (72 fresh seeds: food +5.5%, births +14%, ants at the end +4%, p 0.08-0.22; food eaten +4%, p 0.044); 16 tests |
| [#486](https://github.com/sgladstein/Pixel_Physics/pull/486) | walk | no hand-laid trail; measurement and harness riders | 0 | 0 | clean | measurement only |
| [#487](https://github.com/sgladstein/Pixel_Physics/pull/487) | walk | the trail-away walk **on**; a hungry ant keeps what it holds | 154 | 1 | finding | W4: five tests (six with `specimen.rs`) pinned to the old walk, none guards the shipped one; the switch the whole bed gain rests on (putting it back kills the colony on 24 of 24 seeds) |
| [#488](https://github.com/sgladstein/Pixel_Physics/pull/488) | walk | `antloop.py`: the loop ant by ant | 0 | 0 | clean | tool; selftest passes |
| [#489](https://github.com/sgladstein/Pixel_Physics/pull/489) | walk | why foragers starve: a full crop is 3x the ant (off) | 82 | 1 | clean | numbers reproduce |
| [#490](https://github.com/sgladstein/Pixel_Physics/pull/490) | walk | doubled crop, food half as heavy **on** (assets) | 83 | 0 | note | W8: "Let's test it out" is recorded as a default-on ruling; not checkable from the review queue |
| [#491](https://github.com/sgladstein/Pixel_Physics/pull/491) | walk | nest-door test: a painted door buried (off) | 165 | 1 | clean | measured |
| [#492](https://github.com/sgladstein/Pixel_Physics/pull/492) | walk | scouting switch | 223 | 1 | note | W11: `scout_home` outlives a give-up on 11 of 130 departures after a give-up, 8% (four bed seeds, decision trace) |
| [#493](https://github.com/sgladstein/Pixel_Physics/pull/493) | nest | nest mouth: the dug founding shaft (off) | 608 | 5 | note | W2/W11: default shaft half a column east; W5 (surface corpse) lives in its founding paint |
| [#494](https://github.com/sgladstein/Pixel_Physics/pull/494) | walk | scouting **on**; the come-home rule held off | 353 | 2 | note | harm stated; supported at HEAD (putting it back: starved +122%, 21 of 24 seeds); the `SCOUT=off` typo leaves it on (W7) |
| [#495](https://github.com/sgladstein/Pixel_Physics/pull/495) | walk | foraging-loop handoff note | 0 | 0 | clean | docs |
| [#496](https://github.com/sgladstein/Pixel_Physics/pull/496) | loop | the forage drive (off) | 610 | 3 | clean | no defect found; its later form shipped in #515 |
| [#497](https://github.com/sgladstein/Pixel_Physics/pull/497) | nest | nest lane docs and file ownership | 17 | 0 | clean | docs |
| [#498](https://github.com/sgladstein/Pixel_Physics/pull/498) | nest | scoreboard and funnel (harness) | 0 | 0 | finding | W6: the nest scorer's holes originate here |
| [#499](https://github.com/sgladstein/Pixel_Physics/pull/499) | nest | refill from own bank; dig-down off; a claim withdrawn | 15 | 0 | clean | a claim withdrawn in place: good practice |
| [#500](https://github.com/sgladstein/Pixel_Physics/pull/500) | loop | forage drive corrected on main; `,fed` tested | 88 | 1 | note | the correction was a provenance error (logs from an intermediate build), caught by the lane itself |
| [#501](https://github.com/sgladstein/Pixel_Physics/pull/501) | nest | refill finding corrected | 0 | 0 | clean | correction in place |
| [#502](https://github.com/sgladstein/Pixel_Physics/pull/502) | nest | heaps refill holes: pellets set on ants (two switches, off) | 166 | 2 | note | W4: its tests read the cached environment and cannot take both arms |
| [#503](https://github.com/sgladstein/Pixel_Physics/pull/503) | nest | spoil stays spoil **on** | 27 | 0 | note | owner-ruled, harm stated (3 of 12 colonies lost); W8: "temporarily" dropped in paraphrase; at HEAD not separable (all effects under 10%) |
| [#504](https://github.com/sgladstein/Pixel_Physics/pull/504) | loop | forage drive and carry patience **on**; Z35 | 216 | 2 | note | W1: 12-seed lab check; both supported at HEAD (drive off: births -78%; carry patience off: food eaten -11%, p 0.02) |
| [#505](https://github.com/sgladstein/Pixel_Physics/pull/505) | nest | two weights on the dig | 37 | 0 | clean |  |
| [#506](https://github.com/sgladstein/Pixel_Physics/pull/506) | nest | one entrance: the heap decides where ground opens (off) | 250 | 2 | note | identity CONFIRMED (bed and digbox, control differs); foraging lane's R1/R2 review fixed two defects |
| [#507](https://github.com/sgladstein/Pixel_Physics/pull/507) | nest | the founding shaft and the heap cue **on**; two founding bugs | 331 | 2 | finding | W3: title "One entrance by default" against 6 entrances at HEAD; W8: `SPOIL_CUE` doc still says unset is off; the shaft is supported at HEAD (off: births -92%) but see W7; W10: the cue costs births at HEAD (72 fresh seeds: +29% without it) while cutting starvation |
| [#508](https://github.com/sgladstein/Pixel_Physics/pull/508) | nest | dig-down **on** for an enclosed digger | 345 | 1 | finding | W3, W7 (a typo picks the harmful arm and a test pins it), W4 (gate unguarded until #518), W8; bed starved 83 -> 152 and born 59 -> 30 were stated; still cuts entrances at HEAD (9 -> 6); W10: costs the bed at HEAD (72 fresh seeds, without it: food +8%, births +28%, starved -24%, ants at the end +12%) |
| [#509](https://github.com/sgladstein/Pixel_Physics/pull/509) | loop | packed lunch **on**; food off the pile +44% | 292 | 2 | finding | W1: lab arms from two binaries, "neutral" overreads a p-value; at HEAD the effect holds and is smaller (putting it back: food taken -20%, births -66%) |
| [#510](https://github.com/sgladstein/Pixel_Physics/pull/510) | loop | birth price **on** (Z36) | 178 | 1 | finding | W1: the bed cannot see it (24 of 24 identical), the lab shows births x0.62 at p 0.064; stated in the body; fixes a real overdraw |
| [#511](https://github.com/sgladstein/Pixel_Physics/pull/511) | loop | why so little food builds up at the nest (evaluation) | 0 | 0 | clean | no behaviour change |
| [#512](https://github.com/sgladstein/Pixel_Physics/pull/512) | nest | storeroom (off), castes, the heap cue lets a room deepen **on** | 816 | 2 | note | not testable by parent-vs-merge (ships the cue fix on); storeroom side is W2; its drop site takes SW before S before SE (W11) |
| [#513](https://github.com/sgladstein/Pixel_Physics/pull/513) | nest | the full granary **on**: door, storeroom, nest workers | 618 | 5 | finding | W2 (west side), W3 (entrances 3 -> 6, nest-like 20 -> 0 of 24, disclosed), W1 (12-seed lab check); the bed gain is real (granary off: births -38%); W10: the storeroom half costs the bed (without it starved -65% on 72 fresh seeds, +13% ants at the end on the 24 frozen, p 0.011; nest food halves) |
| [#514](https://github.com/sgladstein/Pixel_Physics/pull/514) | loop | store lunch held off; economy control | 169 | 1 | finding | W1/W8: the hold's stated reason contradicts the lane's written gate (17 to 7 at p 0.064 shipped elsewhere); identity CONFIRMED |
| [#515](https://github.com/sgladstein/Pixel_Physics/pull/515) | loop | the returns drive ships; store lunch stays off; a test-bed review | 308 | 2 | finding | births 16 of 24 lower (p 0.15) shipped by the rule; W8: 32% vs 41%; lab arms differ from #516's "off" on seed 9 |
| [#516](https://github.com/sgladstein/Pixel_Physics/pull/516) | loop | trip reach **on** | 495 | 11 | note | births lean lower, unresolved (bed 72 seeds -14%); no effect separable at HEAD; eleven tests: the best-guarded PR of the week |
| [#517](https://github.com/sgladstein/Pixel_Physics/pull/517) | nest | one entrance round the door: `SPOIL_OUT` and friends (off) | 435 | 0 | note | identity CONFIRMED for the engine (a harness edit moved digbox numbers); W4 (three fixes, no tests), W11 (lift cap) |
| [#518](https://github.com/sgladstein/Pixel_Physics/pull/518) | nest | the nest stops leaning west (R3) | 245 | 4 | finding | W2 (title against 24 of 24 west), W5 (surface corpse), W8; the coin is fair end to end |

## 10. The default-flip ledger

What each shipped default buys on the colony bed at HEAD, measured by putting that one switch back to its before-value on the 24 frozen fresh seeds (bed 20 founders, gap 90, 24,000 frames; exploratory, 14 arms x 4 measures, no correction for multiplicity), with a replication on 72 independent seeds (125-196) for the five arms where putting the switch back looked neutral or better on the bed at 24 seeds (picked after looking at the 24; spoil packing looked neutral too and was not replicated), and whether any test goes red when it is flipped (§6).

| switch | flipped by | unset now | putting it back, 24 frozen seeds: food taken | born | ants at end | starved | replication on 72 independent seeds (125-196): food taken / born / ants at end / starved | tests red when flipped: new (68) / whole suite |
|---|---|---|---|---|---|---|---|---|
| `CHOOSER` | #487 | trail-away walk | **-99% (0/24)** | **-100% (0/23)** | **-100% (0/24)** | **+309% (24/0)** | - | 0 / 0 |
| `SCOUT` | #494 | gain 2.0 | **-27% (4/20)** | -16% (8/13) | **-30% (5/18)** | **+122% (21/3)** | - | 1 / not run |
| `CARRY_PATIENCE` | #504 | on | -10% (7/17) | -25% (7/16) | -12% (7/15) | +20% (9/6) | - | 0 / 0 |
| `FORAGE_DRIVE` | #504, #515 | returns | **-40% (1/23)** | **-78% (1/22)** | **-36% (3/21)** | +28% (14/5) | - | 0 / 0 |
| `TRIP_REACH` | #516 | on (16) | +1% (9/9) | +7% (10/6) | +5% (11/6) | -7% (3/7) | +2% (33/27) / +2% (27/23) / +1% (28/25) / +2% (11/11) (n=72) | 0 / 0 |
| `PACKED_LUNCH` | #509 | on | **-20% (4/20)** | **-66% (2/21)** | **-17% (6/18)** | -26% (9/12) | - | 0 / 0 |
| `BIRTH_PRICE` | #510 | guaranteed | +0% (0/0) | +0% (0/0) | +0% (0/0) | +0% (0/0) | - | 0 / 0 |
| `NEST_SHAFT` | #507 | 6 rows | **-41% (2/22)** | **-92% (1/23)** | -16% (7/16) | **-85% (2/19)** | - | 1 / not run |
| `SPOIL_CUE` | #507 | on | +6% (13/11) | +58% (15/7) | +21% (14/7) | +21% (10/13) | -4% (32/40) / **+29% (43/21)** / -0% (32/37) / **+86% (50/16)** (n=72) | 0 / 0 |
| `DIG_DOWN` | #508 | enclosed digger | +1% (13/11) | -17% (6/15) | +1% (11/10) | -30% (9/14) | **+8% (49/23)** / **+28% (41/24)** / **+12% (47/22)** / **-24% (22/40)** (n=72) | 0 / 0 |
| `SPOIL_PACKS` | #503 | off (spoil stays spoil) | -7% (11/13) | -7% (9/13) | -1% (9/13) | +4% (10/10) | - | 0 / 0 |
| `NEST_DOOR` | #513 | 5-column door | **-23% (2/22)** | -26% (8/14) | **-14% (4/17)** | +20% (15/6) | - | 0 / 0 |
| `STOREROOM` | #513 | side room, castes, keep | -3% (13/11) | -12% (12/12) | **+13% (18/5)** | **-74% (2/18)** | -1% (31/41) / +0% (32/33) / +9% (39/27) / **-65% (10/48)** (n=72) | 0 / 0 |
| `DROP_REACH` | #485 | drop through bodies | +8% (15/9) | +35% (13/6) | +19% (15/6) | -11% (7/11) | +6% (44/28) / +14% (38/26) / +4% (39/28) / +6% (24/32) (n=72) | 0 / 0 |

(cell = change of the ablated arm's sum against HEAD's, then seeds where the arm is higher / lower; **bold** = exact two-sided sign p < 0.05. For food taken, born and ants at end a negative number means putting the switch back makes that measure worse, i.e. the shipped default helps; for starved a positive number is the worse one.)

**Reading the ledger** (fourteen defaults, no correction for multiplicity, so read the bold cells as leads, not as a family-wise result):

- **Clearly earning their place on the bed** (putting them back costs food, births or ants at the end at p < 0.05 on the frozen seeds):
  the walk (`CHOOSER`: everything collapses), the forage drive, the packed lunch, the founding shaft, scouting and the door;
  carry patience weakly (food eaten p 0.02, births 7 up and 16 down, p 0.09). The shaft's number carries a caveat: putting `NEST_SHAFT` back on
  its own also switches off what hangs from it (the storeroom is cut off the shaft and the door has no mouth), so it measures the
  shaft and its dependants (§8, W7).
- **Null on the bed**: trip reach (72 independent seeds: every measure within 2%, p >= 0.5) and spoil packing (24 seeds only: every effect under 10%, none separating).
- **Invisible to the bed**: the birth price (24 of 24 seeds identical: the bed has no seeds), so its evidence is the lab's alone (W1).
- **Costing the bed something at HEAD** (W10): dig-down, the storeroom and the heap cue, and, weakly, drop-through-bodies (food eaten +4%, p 0.044; its other measures do not separate). Three of the four are
  nest steps under the owner's ruling that colony numbers do not block a nest step (dig-down's gain is in the nest: 9 entrances against 6 without it); the drop is the one whose ruling ("on, because the loop
  improves") rests on evidence from before the walk and the granary existed.
- **Pinned by no test at all**, in the whole 1,947-test suite: `CHOOSER`, `CARRY_PATIENCE`, `FORAGE_DRIVE`, `TRIP_REACH`, `PACKED_LUNCH`, `BIRTH_PRICE`, `SPOIL_CUE`, `DIG_DOWN`, `SPOIL_PACKS`, `NEST_DOOR`, `STOREROOM`, `DROP_REACH` (§6, last column).

## 11. Controls: what was run to check the review itself, and what each showed

A review that only reports what it found cannot say what it would have missed. Each
instrument here was given a case whose answer was known, and each *mistake in the review's
own instruments* that the controls caught is listed too, because the same kinds of mistake are
what this report is about.

| Instrument | Control | Result |
|---|---|---|
| Reader agents (4, briefed from raw diffs and PR claims as hypotheses, not the lanes' narrative) | each reader's first item was written before opening anything else: A1 and A2 read #508's diff alone (the foraging lane's review, R3 findings 2 and 3, found its west-leaning half turn and its unguarded enclosed-only gate after the merge); A3 read `nestscore.py`'s `paired()` cold; A4 read #518's body against its title | **passed** for the two known-defect controls: A1 and A2 both re-found the west-leaning half turn from the diff alone (A2 also the untested gate; A1 did not list it). A3's and A4's first items had no owner-found defect behind them, so they say nothing about those readers' sensitivity; both did find what the coordinator already knew (the `abs(mouths - 1)` entrance score and the silent skip of empty seeds; #518's title against its own body). A reader that had missed its known answer would have had its "nothing found" discounted |
| Switch-off identity | the merge with its own switch ON must differ from its parent | the merge with its switch on differed from its parent in all four; identity held for the three that could be isolated |
| Ladder / ablation arms | the all-off arm's header must echo the before-values; the anchor (an old binary, no environment) must agree with it; arms must show a difference before being compared | the header echoed ten of the fourteen before-values (the rest are not printed by the bed harness; those arms were checked by their effect); the anchor also eats almost nothing (13,180 J against 2,984 all-off, against 1,012,983 at HEAD; its harness prints only food eaten); one arm (`BIRTH_PRICE`) tied to the last digit, and was *explained* rather than dropped: the bed has no seed for the price to touch |
| Nest symmetry probe | a run with nothing biasing the nest must read centred; the biased run must read off-centre | storeroom off: 10 west, 12 east, 2 centred (p 0.83); storeroom on: 24 of 24 west, median -11 columns |
| Guard flip-sensitivity | the default-environment run must be all green; a flip must be able to turn something red | 68 of 68 new tests and **1,947 of 1,947** lib tests green at default; the `SCOUT` flip turned `a_declined_load_books_nothing_and_counts_once` red and the `NEST_SHAFT` flip turned `a_founding_digs_with_the_founders_own_jaw` red |
| The lab pre-ship gate | inject a 15% harm at the spread the committed logs actually have and see whether the gate fires | it fires 5-15% of the time: the gate is blind to a harm of that size, which is finding W1 |
| Statistics parse | print the key's cardinality and `n` before any aggregate | done for every arm; two parse faults were caught this way (below) |
| This report's draft | a skeptic agent (told to find defects only, and given the logs, the generators and the source) read the assembled draft against them | 29 defects: 8 overreaches (a "real" births drop shown for only one PR, a header check that covered ten of fourteen switches, a replication pooled with the seeds that prompted it, ...), 7 inconsistencies between sections, 6 wrong numbers, line citations or quotations, 3 omissions (one changed a conclusion: the tidy all-off pair is decided by the walk alone, fixed by a walk-on arm and by rewriting §2 and §3), and 5 nits; all applied. A second skeptic read the revised draft and found 13 more (five inconsistencies, one of them a sentence left over from the earlier text; four overreaches; one omission; three nits; none changed a table number): the lab paragraph still endorsed the wiki's "half as many young" and called the colony smaller, starvation was quoted as a median ratio where births were paired, and the replication criterion was misdescribed; all applied. Neither could check what it could not run (below) |

**Mistakes the controls caught in the review's own work** (all fixed before any number
was used; none of them changed a conclusion):

- a parser that attached each seed's detail block to the *previous* seed's table row (the
  harness prints the block before its row): every seed was shifted by one until checked
  against the raw log (seed 101: 149 cells, 3 born, 6 starved);
- a regular expression for the nest's signed centre (`p50x=+7`) that dropped every east seed,
  which read as "10 west, 0 east" until the raw line was compared (true: 10 west, 12 east,
  2 centred);
- a **contaminated test binary**: a script copied over the clean snapshot with the patched
  experimental build, and the default-environment control (which showed two unexpected
  failures) is what exposed it; every flip result from that run was discarded and the whole
  set redone on a clean HEAD build;
- a **wrong working directory**: 20 tests read `assets/` from disk, so a full-suite run from
  the wrong directory failed exactly those 20 at default environment, which the same control
  caught; the first attempt was discarded;
- a comparison of each commit's *own* `digbox` that said #517's engine changed the nest on 7
  of 7 runs, when it was #517's harness edit (§8);
- **the draft itself**: between the skeptic's read and the final text the review's own summary of "the stack works"
  was found to rest on a pair decided by one switch (above), a replication pooled with the seeds that prompted it
  (moved to the 72 independent seeds), and a wiki paragraph quoted only up to the clause that made it accurate;
- a `pkill`-style pattern that matched the reviewer's own shell and killed it, and an arm that
  was launched twice (two "done" lines); its logs were checked for holes (none: the runs are
  deterministic, so identical bytes at identical offsets).

## 12. What was not checked, and what this review cannot say

- **PRs merged after `2274e347` are not reviewed.** At the time of writing there are three: **#520** ("Trail instruments", the foraging
  lane, merged 2026-09-29 23:11Z: +216 lines in `creature.rs`, `examples/trailfollow.rs`, three scripts and lane-note edits); **#522** (the nest
  lane, merged 2026-09-30 00:33Z: an engine fix in `close_or_hand_over` that is byte-identical at stacking cap 1, the carry switched on but inert
  without the walked cycle, `digbox` fed by default, the nest lane note rewritten); and **#519** (an ant breeding plan, merged 2026-09-30 18:31Z,
  with one engine fix, below). Every `file:line` in this report is at `2274e347`, and the files these touched (`creature.rs`, `world.rs`,
  `foraging-loop.md`, `nest-mouth.md`, `how-the-ant-works.md`) have shifted since. **#484** and the three open PRs (#471, #476, #478), stale
  branches, and prose quality are out of scope. Four things in the later PRs bear on this report:
  - **#519 fixes a panic this review did not find**, and "no blocker" (§1) is not a claim that the pinned commit cannot panic: the review looked
    at what the week's PRs changed, not for crashes elsewhere. `World::denied_seen` was a fixed `[u64; 64]` (`world.rs:3940` at the pin, indexed at
    `7579`), so a refused birth by an organism in slot 4,096 or above indexed out of bounds; #519 makes it a growing `Vec`. The code is older than
    the week. #519 reads the path from the allocator (the held world's grown start makes 4,093 organisms) and did not observe it in a played
    session. None of this review's runs panicked, and the largest colony alive at the end of a game-default run was 2,202 ants (seed 115).
  - **#522 rewrote the nest lane note**: §7 row (c) (its stale live-question numbers) and the lane-note half of row (a) (the "no longer leans west"
    card line) cite the pinned version and no longer hold on `main`; row (h)'s quotation is unchanged.
  - **#522 makes `digbox` fed by default** (`hungry` restores the old box), which is why §13 is written for the pinned commit.
  - **The owner now answers in chat, not the review queue** (2026-09-29: "I cannot review the queue, post questions/images in this chat",
    recorded by #522), which is why the queue holds no later ant answers (row i and the rulings bullet below).
- **Only 4 of 34 PRs were tried for "unset reproduces the parent" (3 could be isolated)**, on four bed seeds and one
  `digbox` seed each (§8): the ones that say so and could be built. A sample, not a proof.
- **Mutation testing was not done.** The flip run says whether a default is pinned; it does not say whether
  a pinning test would catch a subtly wrong *mechanism*.
- **Reader agents read code, they did not run it**; every finding marked CONFIRMED was re-read or re-run by
  the coordinator, findings marked *(reader)* or PLAUSIBLE were not. No skeptic stage was run on the findings
  themselves beyond the coordinator's re-runs; two separate skeptic passes did read this report's drafts against
  the logs and the source, and their findings were applied (§11). The second did not re-check §4, §5, §7, the nest tables or the assertion-literal and field-pin counts, which only the first read, and could not confirm from the logs that each flip value took effect in the test process (the runner swallows a passing test's warnings; §6 gives the indirect evidence).
- **The colony bed is one food pile on bare soil with 20 founders**; the lab box is one scenario. Neither
  contains water, a predator, a second species' food, or a player. The stack holds up on both, with the caveats above, and says
  nothing about play.
- **The lab pair has 12 seeds.** At n = 12 the sign test needs 10 to 2 for p < 0.05, so births (9 to 3) is
  unresolved by design, and the effect sizes are the informative part.
- **The nest scorer's null models were used as shipped** (walkers, eden, uniform, rows); the review did not
  audit them beyond noting that the door and shaft are masked for the colony and every null alike
  (`digbox.rs` `portal`, reader A3).
- **Frame cost was measured with `antcost`**, the lab-box replay, not with `examples/ascii` (which CI runs)
  and not at 512x320 outdoors with a large colony, which is where the world will grow.
- **Rulings dated 09-25 to 09-29 were checked against the lanes' own notes only**, not against the owner's
  review queue, which holds no response after 2026-09-24 for the ant line (reader A4, not re-counted). Since 2026-09-29 the owner answers in chat, not the queue
  (the lane note, as rewritten by #522), so the queue cannot confirm them.
- **Shallow clone (301 commits, grafted roots)**: `git log -S`, blame and every `branchcheck` "DATA" verdict
  were avoided; per-PR diffs are tree and merge-commit diffs.
- **Per-step costs at HEAD in the lab** (for example what the birth price costs in the lab at HEAD) were
  not measured except where §3 says so; the bed cannot see several of them (§4).

## 13. Reproduce

The review's scripts and logs are in the session's scratch area, not in the repository (this PR adds
only this report and its index line). Everything below is a command against a clean checkout of
`2274e347` plus the switch environment the row names, run under `env -i` with `RAYON_NUM_THREADS=1`
unless stated. The harnesses moved after the pin (`trailfollow` by #520, `digbox` by #522: on `main` the dig box is fed by default, so add `hungry` to
get the no-food box used here); these commands are for the pinned commit.

| What | Command (from the repo root) |
|---|---|
| The ladder and the ablations | `env -i PATH=/usr/bin:/bin HOME=$HOME RAYON_NUM_THREADS=1 PIXEL_PHYSICS_COLONY_SPACING=2 PIXEL_PHYSICS_STACK_DEPTH=4 PIXEL_PHYSICS_BUD_SITE=nest <ARM ENV> ./target/release/examples/trailfollow mode=gap gate=shipped frames=24000 ants=20 near=10 food=400 refill=400 arms=self gaps=90 seeds=8 seed0=101` (three blocks: 101, 109, 117), where `<ARM ENV>` is empty for HEAD and the switch at its before-value for an ablation (`PIXEL_PHYSICS_SCOUT=0`, `..._BIRTH_PRICE=face`, `..._NEST_DOOR=off`, ...); the all-off arm sets all fourteen |
| The game's own regime | the same with no `COLONY_SPACING`, `STACK_DEPTH` or `BUD_SITE` |
| The nest lane's instrument | `env -i ... ./target/release/examples/digbox ants=40 energy=1000 frames=24000 seed=<101..124>` and `python3 scripts/nestscore.py <dir> --stop 24000 --base <arm>`; the signed centre is `p50x` on the last `SUMMARY digs=` line |
| Nest lean, paired | the default against `PIXEL_PHYSICS_STOREROOM=off`, `p50x` per seed |
| The lab stack pair | from the repo root: `env -i ... RAYON_NUM_THREADS=1 [<all fourteen before-values>] ./target/release/examples/labforage scenario=played_bed frames=120000 seed=<101..112>`, then `python3 scripts/labpair.py <dir> off head` |
| Guard flips | `cargo test --release --lib --no-run`, then from the repo root `env -i ... PIXEL_PHYSICS_<SWITCH>=<before-value> <test binary> --test-threads=3`, one process per value (a default-environment run first: it must be all green) |
| Un-pinned walk | remove the five `w.chooser = Some(Chooser::Off);` pins in `creature.rs` (the sixth is in `specimen.rs`, untested) and run the lib tests |
| Surface corpse (W5) | a scratch test beside `the_founding_cut_leaves_a_corpse_where_it_lies` (`creature.rs:22325`) with two changes: the corpse (`aux` 300) goes on the first ground row, `(60, 40)` (what `colony_surface(&w, 60, 39)` returns), instead of `(60, 42)`, and the whole `w.found_colony_of(60, 39, "ant", 4)` runs instead of the test-only `cut_founding_shaft`. Then `w.get(60, 40)` reads `(MaterialId(0), 0)` where the corpse was; the same test with the corpse at `(60, 42)` keeps it (checked again at the end of the review) |
| `nestscore` hole (W6) | `PIXEL_PHYSICS_NEST_SHAFT=off` digbox logs as arm `D_SHAFT`, then `python3 scripts/nestscore.py <dir> --stop 24000 --arms D_DEFAULT,D_SHAFT --base D_DEFAULT` |
| Gate power | `python3 scripts/labpair.py`'s `sign_p` over an injected log-ratio shift with the committed spreads (reader A3's simulation reproduced by the coordinator's exact-binomial arithmetic) |
| Cost | runs 1-3: `antcost ants=0,200,600,1200 frames=300 reps=4`; runs 4-6: `antcost ants=0,200,600 frames=300 reps=6`; both with `RAYON_NUM_THREADS=4` under `env -i`, each binary run from its own commit's checkout (the lab box reads `assets/` at run time), whole runs alternating baseline, HEAD, baseline, HEAD, ..., with every other job SIGSTOPped for the window |
| Docs | `bash scripts/docscheck.sh`, `python3 scripts/bugindex.py --check` (both clean at HEAD) |
