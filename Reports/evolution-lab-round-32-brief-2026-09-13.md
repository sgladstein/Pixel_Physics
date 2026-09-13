# The evolution lab, round thirty-two: the brief

**Handed over by round 31's coordinator, 2026-09-13.** Round 31's record is
[`evolution-lab-round-31-2026-09-13.md`](evolution-lab-round-31-2026-09-13.md).
**Read that record's *"nobody can reproduce the owner's bed"* section before
anything else here** — it is why this brief is ordered the way it is, and it
invalidates the premise of at least three open register sections.

**A `lab-coordinator` skill now exists** (`.claude/skills/lab-coordinator/SKILL.md`,
landed 2026-09-13 by another session). Read it for the spawn/poke/close
mechanics; this brief carries only what is specific to round 32.

**Trunk state this brief was written against.** Round 31 landed **#370, #371,
#372, #373, #374, #375, #376**, with **#379** (the dug pellet needs a footing)
and **#380** (the idle-anim clock) reviewed and closing behind them. So on
today's `main`: the chronicle records player actions and load and autosaves;
the MENU rows draw as buttons; the ant lifespan and every played-bed number
have been re-taken post-cull; and `spoil` is a distinct material from
`packedsoil`. **Two numbers to re-take rather than inherit** — the suite was
1,664 / 0 / 70 at `047df5c6` and has grown since, and any played-bed figure in
a report dated before 2026-09-13 was measured through the seed cull.

## Task 1 — build a bed that looks like the owner's game

**Everything else in this brief is downstream of this, and most of the open
register is untrustworthy until it exists.**

Round 31 established, by measurement and then by the owner's own eye, that
**no bed this project can currently generate resembles the one he plays.**

| | ants |
|---|---|
| the owner's ordinary late game | **1000+ long ants** |
| `played_bed`, shipped settings, 200,000 frames | median **111**, max **408** |
| the largest single run in a 75-run sweep | **1,067**, once, `life_half_life: 80000` seed 8 |

His verdict on the most developed nest round 31 could render — `played_bed`
seed 3, 150,000 frames, 74 ants — was *"None of this reads as an ant hill
though it just looks like herbs growing in dirt."*

**The input you will have that round 31 did not: a real chronicle file.**
#374 gave the chronicle player actions, autosave, and nine load/cost columns
(wall clock, awake chunks, active sites, achieved against requested ticks,
`sim_debt`, speed multiple, display rate, draws skipped). The owner has agreed
to play a session to the point where it hurts and hand the file over. **Ask
the coordinator for it; do not post a card asking him** (see the standing
rulings below).

From that file, build a scenario that reaches his scale, and **prove it does**
before anything is measured on it. Then re-open, on that bed and not before:
§Z18 (floating debris), §Z13 (resting reads as stuck), and the stripped-ground
complaint in task 3.

## Task 2 — the late-game performance deep-dive

**The owner asked for this directly and it is the reason task 1 exists.** He
reports the game struggling with 1000+ long ants.

**Do not start it on a 400-ant bed.** With task 1's bed and his chronicle:

- read `sim_debt`, achieved-against-requested and `draws_skipped` out of his
  own session first — that says whether he is sim-bound or render-bound, and
  it is the fork everything else hangs on;
- **read `speed_multiple` beside every one of those** — the same achieved rate
  means opposite things at 1X and at the top of the ladder;
- the perf line's handed-forward list is the **~21% in the kernel and rayon**,
  then the moisture pass, then the pheromone `roundf` (which is *not*
  behaviour-free);
- per-phase timing belongs in `scale_probe phases=`, headless, **never in the
  live loop** — round 31 deliberately kept stopwatches out of `Lab::tick`.

**Model: Opus, not Fable.** See the rulings below — the difficulty here is
measurement discipline, not reasoning depth, and this repo's history of
timing numbers that were correct and about the wrong thing is long.

## Task 3 — land recovery behind a colony

The owner's playtest report, 2026-09-13: ants find a herb patch, feed, breed,
dig, **eat it clean**, and move on; the abandoned nest never regrows, though a
hand-planted herb there grows fine.

**Eating a patch clean and moving on is foraging, not a bug. The bug is that
the land does not recover behind them** — and that is a plant problem.
`Reports/plant-reseeding-2026-09-03.md` already measured four causes ahead of
dispersal, which is only a 1.4x effect:

- the germination gate opens on **two materials in the whole set** (three now
  — `spoil` declares `water_capacity: 1000`, checked);
- the grow lamps leave **32-column dead bands**;
- the colony is a **seed predator**, cutting the stand **2.8x**;
- and the largest single sink is seeds stuck **on the parent plant** — 183 of
  332 standing seeds, because a seed does not fall through branches while a
  windfall does, an inconsistency that was never designed.

**Treat these as one problem.** Fixing dispersal alone buys 1.4x against a
2.8x predation loss.

## Task 4 — §Z13: two of three explanations are closed, the third is untouched

**Do not build a fourth idle animation. Round 31 answered why the first three
read as failures and it was not the look.** `IDLE_ANIM_DELAY` was counted in
`Renderer::frame` — draw calls — not world ticks, so on the posted card's own
`every=10` sampling a 60-frame delay was 600 world ticks and **only 8 of 22
full-length long ants ever animated in the window the owner judged**. Fixed to
`World::frame` (#380): **21 of 22**. The undercount was live in the real game
too, since `App::update`'s catch-up loop runs several ticks per draw.

So of the three explanations — never fired, too faint, genuinely stuck — the
first is closed and was most of it, and the second is now testable for the
first time on a card where the mechanism actually runs. **The third is
untouched and is the one that matters**, because if those ants are genuinely
stuck then §Z13's "look problem, not a walk bug" is wrong and the walk is. It
**has its marker coordinates, and round 31 said otherwise and was wrong.**
Round 31's own idle card (`20260913T034419970Z-34d562`) is archived carrying no
stored response at all, so nothing is retrievable from it. But **round 29's
card `20260912T045951545Z-6931d4` is the same complaint on the same animals,
and `get` returns its three markers in full**, normalised to the image:

| x | y | the owner's note |
|---|---|---|
| 0.9078 | 0.6414 | *"This the most prominent thing that shows no movement in both images"* |
| 0.6347 | 0.5325 | *"also no movement"* |
| 0.3773 | 0.6172 | *"no movement"* |

Map those back through that card's own crop and zoom to world cells, find the
animals standing there, and **check them from the inside**: `moves`,
`moves_blocked`, `traffic_deferred`, and `HeadBlock`/`head_block`'s
open-heading count in `src/sim/creature.rs`. A resting ant and a blocked one
are trivially separable by counter and not at all separable by eye.

**Read cards with `get <id>`, never off `inbox`.** `inbox` is a filtered view,
not a listing: measured 2026-09-13, all three cards named above are absent from
it entirely while `get` returns them, and off `inbox` the round-29 card reads
as having no annotations. Round 31 concluded *"annotations do not survive the
queue"* from exactly that mistake, and told a lane the check was impossible.

**And a resting ant's idle streaks run 33–83 stops of 900 frames**, so a card
must be long enough to show both the animation cycling and an animal that has
not moved in 50,000 frames. Round 31's second card is 6,000 ticks and posted;
collect its verdict before shooting another.

## Task 5 — the spoil teleport, and PR #221, which must not go invisible twice

**`SPOIL_LIFT = 160` is live on today's trunk and nothing this round removed
it.** `src/sim/creature.rs`, in the dig-drop path: when no 8-neighbour will
hold a pellet, the drop scans **straight up as far as 160 rows** for the first
cell that is empty with two of three filled beneath, **with no check that a
path exists**. The ant never climbs. The pellet is teleported, and it stays.
Measured over four seeds on PR #221's own instrument: tallest standing pellet
**+52 / +67 / +99 / +94** with a tree in the box against **+4 / +3 / +2 / +2**
without.

**#379 mitigates this and does not fix it.** A pellet is now `spoil`, which
needs ground under it, so a pellet teleported onto plant tissue falls — which
kills the lattice bootstrap, where each pellet satisfied "two of three beneath"
against the previous one. But **the teleport itself is untouched**: a pellet
that lands on real ground 160 rows up is still a cell that crossed the box
without anything carrying it. Round 31 declined to widen #379 into this, which
was right; it is its own task.

**PR #221 (`claude/creature-plant-pathfinding-rjzkqe`) has been open since
2026-09-03 and still carries work that is not on the trunk**:
`examples/spoil_destination.rs`, with a **no-tree positive control**, and
`CreatureStats::spoil_lifted` / `spoil_lift_max` — the split `spoil_dumped`
cannot make, because it sums both placement branches. Round 31's Lane B read
it and measured **44% of pellets going through the up-column branch**, but
shipped none of the counters. **Land the instrument and the counters first**,
then decide the mechanism with them running.

**Why it went invisible, which is the part to carry forward.** Its register
section was filed as **§Z4, a letter already used and closed on `main`**, so
it is not in the register anyone reads — and `branchcheck --prs` lists its
branch as *having a PR*, which reads as owned rather than stalled. Round 30's
rule was *the PR list is not the work list*; this is the other half: **the PR
list is not the landed list either**, and a ten-day-old open PR can hold the
exact instrument a new lane is being told to build from scratch. It is now
ten days old and it just survived a round that read it and still did not land
it. **Either land it this round or close it and file the mechanism as its own
register section under a letter `bugindex.py --branches` says is free.**

## Standing rulings this round paid for

- **The review queue is for visual evaluations only.** Owner, 2026-09-13:
  *"General questions or requests should be sent to the coordinating agent to
  tell me."* A lane wanting a file, an answer or a decision routes it through
  the coordinator.
- **Fable 5.1 is `$10`/`$50` per MTok against Opus 5's `$5`/`$25`** — twice
  Opus, the most expensive tier, not the cheapest. Round 31's brief said the
  opposite; it was retracted in #372. `create_session` takes no effort
  parameter, so for a lane **the model is the whole dial**.
- **A conservation failure is a question about the ruler before it is a
  question about the engine.** Adding a material silently changes every census
  that enumerates materials by name — including ones inside tests.
- **The dangerous merge is the conflict-free one, and CI cannot see it.** Two
  PRs measured against different trunks are a combination nothing tested.
  Round 31 merged three lane branches into a scratch branch and ran the suite
  there; it found a red test no CI had reported. Applied twice more before
  merging, both clean.
- **Grep `examples/` too, and grep the *pairs*.** The material census above
  broke **five** sites; the lane found three, CI found the other two, and both
  misses were in `examples/` — where every measurement in this repo comes from
  and where nothing prompts you to look. The sites that matter are the ones
  naming a material set on **both sides of an identity**: a census that names
  it once is merely wrong, one that names it twice still balances for the old
  world and breaks silently for the new.
- **Ask "did it fire at all" of a negative verdict, not only of a harness.**
  `CLAUDE.md` already says to print the discrete event count beside the image;
  task 4 above is the other half — a card whose mechanism did not run and a
  card whose mechanism does not work are the same picture and the same
  verdict.
- **`review.py inbox` is a filtered view, not a listing of the queue, and
  reading a verdict off it gives wrong answers.** Measured 2026-09-13: three
  cards, two posted that day, are absent from `inbox` entirely while
  `get <id>` returns them in full — and off `inbox` a card carrying three
  marker annotations reads as having none. **`get <id>` is the only
  authoritative read.** Round 31 got this wrong in the other direction first
  and published "annotations do not survive the queue", which is false.
- **A card can be archived carrying no stored response at all** even after the
  owner answers it — round 31's idle card is. When that happens the verdict he
  relayed in chat is the only copy, so write it into the register rather than
  pointing at the card.
- **The coordinator note is 12,859 B against its 12,000 B advisory cap**, up
  from 11,727, because round 31 produced six binding findings and five
  compression passes could not reach the cap without dropping a ruling.
  **Archive rounds twenty-nine and thirty into
  `evolution-lab-rounds-archive.md` at this round's close** — that is the
  structural fix, and prose-golf is not.
