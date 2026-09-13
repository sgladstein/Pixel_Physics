# The evolution lab, round thirty-two: the brief

**Handed over by round 31's coordinator, 2026-09-13.** Round 31's record is
[`evolution-lab-round-31-2026-09-13.md`](evolution-lab-round-31-2026-09-13.md).
**Read that record's *"nobody can reproduce the owner's bed"* section before
anything else here** — it is why this brief is ordered the way it is, and it
invalidates the premise of at least three open register sections.

**A `lab-coordinator` skill now exists** (`.claude/skills/lab-coordinator/SKILL.md`,
landed 2026-09-13 by another session). Read it for the spawn/poke/close
mechanics; this brief carries only what is specific to round 32.

**Round 32 is a performance round.** The owner, 2026-09-13: *"The biggest
issue is the performance after the creature numbers get high and that is our
#1 priority by far."* The tasks below are in his order, not round 31's — the
bed-building task that used to be first is gone, because **he has since played
a 560,000-frame session and handed the chronicle over**, and it is committed
at `Reports/data/playtest-2026-09-13-herb_longant-s1-560k.txt` with its
analysis in
[`evolution-lab-playtest-2026-09-13.md`](evolution-lab-playtest-2026-09-13.md).
Read that report before task 1.

**Task order is his, not a queue.** Task 1 (performance) is *"our #1 priority
by far"* in his words. **Task 6 (zoom-out) he has separately called a
priority** — round 31 filed it as unscheduled and he corrected that, so it is a
lane, not a note. Task 2 is small, gates every future log he sends, and should
land early and on its own. Tasks 3, 4 and 5 are real but wait.

**Trunk state this brief was written against.** Round 31 landed **#370, #371,
#372, #373, #374, #375, #376, #379** (the dug pellet needs a footing) and
**#380** (the idle-anim clock). So on
today's `main`: the chronicle records player actions and load and autosaves;
the MENU rows draw as buttons; the ant lifespan and every played-bed number
have been re-taken post-cull; and `spoil` is a distinct material from
`packedsoil`. **Two numbers to re-take rather than inherit** — the suite was
1,664 / 0 / 70 at `047df5c6` and has grown since, and any played-bed figure in
a report dated before 2026-09-13 was measured through the seed cull.

## Task 1 — the late-game performance deep-dive. This is the round.

**The owner's words, 2026-09-13: *"The biggest issue is the performance after
the creature numbers get high and that is our #1 priority by far."* Everything
below task 2 is secondary to this.**

**It is already sized, and you do not have to guess at a bed.** He played a
560,000-frame session and handed the chronicle over; the analysis is
[`evolution-lab-playtest-2026-09-13.md`](evolution-lab-playtest-2026-09-13.md)
and the raw file is committed at
`Reports/data/playtest-2026-09-13-herb_longant-s1-560k.txt`. **Read that report
before opening the log.** What it establishes:

- **cost ≈ 1.0 ms/tick + ~2.1 µs per ant per tick**, fitted on the lower
  envelope of his own wall clock. At 3,000 ants the creatures are **86% of the
  frame** and everything else together is about a millisecond.
- **It is not the cell sweep.** Cost rose 6.3x while `active sites` rose 2.2x,
  and at one sample sites are at their session **minimum** while cost is flat.
- **It is not the renderer.** `draws skipped` is **2 → 10 across all 560,000
  frames**.
- **`debt` and achieved-against-requested carry no information in that log** —
  the dial asked 6,144 ticks/frame and the box did 95 with *zero ants*, so both
  are saturated from the first sample. Read absolute throughput.

**So the target is the per-creature pass and the number to beat is 2.1 µs per
ant per tick.** Halving it roughly doubles the population he can play at.

**What the log cannot do, and do not try to make it**: localise the cost inside
that pass. Adjacent samples at equal ant count differ **12–14x** because he was
using the machine — `CLAUDE.md`'s *a timing number is only as trustworthy as
the box was quiet*, and it means no single interval is evidence of anything.
**Replay his bed headlessly on a quiet box**: `RAYON_NUM_THREADS` pinned,
`scale_probe phases=` for the breakdown, arms compared inside one run. His
scenario is `herb_longant` seed 1 with the box height raised to 512 and
`PLANT_LOAD_FAILURE false`; the log's own header states all of it.

**Quote the whole-frame figure, never a sub-phase.** This repo has a measured
case of a change that removed 91% of a phase's work, moved every per-pass
timing, and made the frame **slower** — the cost relocated to cold misses. And
`ascii` cannot answer anything Lab-side: it drives `sim::frame::step` on a bare
`World` and never reaches `Lab::tick`.

**The perf line's handed-forward list** is the ~21% in the kernel and rayon,
then the moisture pass, then the pheromone `roundf` (which is *not*
behaviour-free) — but that list predates this measurement and **the log says
the creature pass dominates**, so treat the list as second.

**Model: Opus, not Fable.** The difficulty is measurement discipline, not
reasoning depth, and this repo's history of timing numbers that were correct
and about the wrong thing is long.

## Task 2 — the chronicle cannot describe a nest, and five columns say so

**Five census columns are each a single constant across all 56 samples of a
560,000-frame session with 2,982 ants and 356,688 digs:**

| column | value in all 56 samples | what it should say |
|---|---|---|
| `roofed` | **0** | empty cells with ground over them — *a nest exists* |
| `pit` | **0** | standing void that is not roofed |
| `pack<` | **0** | worked soil below the surface — the gallery lining |
| `mnd` | **48** | mound height in rows — reads 48 at **zero** mound cells |
| nest band | **`0/0`** | the band every band-scoped column is measured over |

`roofed` is the column `CLAUDE.md` names as *the* one to read for excavation —
*"what a player calls a nest is roofed void"* — and it is answering nothing.
**The likely common cause is the last row**: the nest band is empty, so
everything scoped to it measures an empty set. That is probably one fix, not
five. `pack^` works and is the only structural column that does.

**Watch each one go red before you trust it**: construct a world with a known
roofed chamber and confirm the repaired column reports it, then delete the
chamber and confirm it drops. A column that reads 0 everywhere passes any test
that only checks it does not crash.

### And what the next log should carry that this one did not

**The owner asked directly what would have helped.** Answering the round's
first question took a hand-fitted lower envelope over 56 wall-clock differences
because the file could not do it itself. These are in priority order and the
first three would each have changed an answer:

1. **Tick-time statistics measured inside the engine — min, median and max over
   each census window.** This is the big one. The contention problem
   (adjacent samples differing 12–14x) had to be worked around by fitting a
   lower envelope by hand; **a per-window minimum is exactly that statistic,
   and the engine can measure it properly.** It costs one comparison per tick.
2. **Creature *cells*, not just creature count, and active against resting.**
   `ants` is a head count, and a long-ant is multi-cell while §Z12 says most of
   a long-run pile is one-cell ants — so **the 2.1 µs "per ant" may be
   averaging two very different animals**. And a resting ant is nearly free
   (§Z13 says they rest a great deal), so **cost per *active* creature could be
   several times the headline**. Either would move the target this round is
   aimed at, which makes them the two most valuable columns missing.
3. **A coarse phase split sampled at census cadence** — creature pass, cell
   sweep, field, render. #374 was right to keep stopwatches out of the live
   loop, but **once per 10,000 frames is free**, and it would turn task 1's
   central claim from an inference (cost tracks ants, not sites) into a
   measurement.
4. **Millisecond wall clock.** `wall` is whole seconds, so the early intervals
   (11–13 s) carry ~8% quantisation. Free to fix.
5. **Size the individual ring against a real colony.** This log dropped
   **79,642** events and kept 664 of 15,905 births. The two-ring split worked —
   every line event survived — but the individual ring is overrun by two orders
   of magnitude at 3,000 ants.

**Do not add per-phase timing to the live loop** and do not let this grow into
a profiler. Everything above is a read or a counter at census cadence.

This whole task is small and it gates every future log he sends, so do it early
and land it on its own rather than behind task 1.

## Task 3 — recovery is slow, not absent. The premise changed; read this first.

**This task was written on a premise the owner's own 560,000-frame log
refutes, and it is left here re-aimed rather than deleted.** The premise was
his earlier report — *the abandoned nest never regrows, though a hand-planted
herb there grows fine*. At session length the bed does come back:

| frame | bare ground outside the nest | plants |
|---|---|---|
| 40,000 | **2%** | 277 |
| 180,000 | **56%** | 24 |
| 500,000 | **6%** | **409** |

It strips to 56% bare and returns to 6%, ending with **more** plants than its
original peak. **Recovery from the trough takes about 320,000 frames** — long
enough that every earlier look at this question was taken before it happened,
which is exactly how it read as "never".

**So the question is no longer "does it recover" but "should it take that
long", and that is the owner's call, not a bug to fix blind.** Put it to him
through the coordinator before building anything. What follows is the measured
cause list if he says it is too slow — do not act on it otherwise.
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

**One thing already moves this number, and #376 measured it: the dig gate.**
Its whole justification was re-derived this round from *the colony lives* —
which is now 12 of 12 in both arms and says nothing — to **the bed stays
green**: plants standing higher with the gate on, **10 of 12 seeds, median
+24, p = 0.039**, and only in the *second half* of a session. That is a
land-recovery result wearing a colony-behaviour label, and it is the closest
thing to a baseline this task has. Two consequences. **Use its method** —
paired arms on the same seeds, a sign test over twelve, read at 200,000 frames
rather than at 120,000, where the same comparison is 7 of 12 and p = 0.77.
And **whatever you change here, re-run the gate arm with it**: the gate and
the four causes above all act on the same standing-plant count, so a fix
measured against a trunk with the gate on is not measuring itself alone.

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
not moved in 50,000 frames.

**One hard floor for any card of nest structure, now measured rather than
guessed: 400,000+ frames.** The playtest log shows mound cells at **exactly
zero for the first 350,000 frames**, then 33 at 360,000, reaching 8,352 by
560,000. Round 31's §Z18 card drew *"none of this reads as an ant hill"* at
**150,000 frames** — 210,000 frames before the first mound cell existed. The
bed was not the whole problem; the card was taken before the thing it was
meant to show had been built. Round 31's second card is 6,000 ticks and posted;
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

## Task 6 — zoom out should use the screen's pixels. The owner calls this a priority.

**Round 31's coordinator filed this as "not scheduled" because he asked a
question rather than asked for work. That was wrong and he has said so.** It
is a lane.

> *"I don't understand why the zoom out has to be complicated... why my screen
> resolution can solve all of the pixels, why cannot there just be more pixels
> when you zoom out?"*

**He is right, and the constraint is not the monitor.** The renderer draws into
a fixed **512x320 back-buffer** (`WIDTH`/`HEIGHT`, `src/app.rs:202`) which
`Pixels::new(WIDTH, HEIGHT, surface)` upscales to the window, so the screen's
real pixels *magnify* that image rather than carrying more of it. At
`MAX_ZOOM_OUT_STRIDE = 4` the view spans **2048x1280 cells through a 512x320
buffer** — sixteen world cells per buffer pixel, and something must be
discarded. That is the whole of §Z11: `Stride` drew the block's top-left cell
and dropped the other fifteen, so a one-cell-wide stem had three chances in
four of vanishing. The salience rule that closed §Z11 is a good answer to
*"which of sixteen cells wins"*; **this removes the question instead.**

**The shape to build: grow the buffer only when zoomed out.** At the widest
stride allocate 2048x1280 rather than 512x320; every cell gets a pixel, nothing
is discarded, and it is still inside an ordinary monitor. **Normal zoom is
untouched and pays nothing** — which matters, because at one cell per *physical*
pixel the world reads sharp and small rather than chunky, and chunky is the
house style. Only the zoom-out path should change.

**The cost is the whole objection and it must be measured, not argued.** The
renderer works per buffer pixel: 512x320 is 164k, 2048x1280 is 2.6M — 16x. A
zoomed-out view is mostly settled terrain, which is exactly where the
dirty-rect skip does its best work, **so the true cost may be far below the
pixel ratio — but that is a hypothesis.** `CLAUDE.md` requires the whole-frame
figure. `ascii` cannot answer it (headless, no render); use the real app's
capture path or `scale_probe`, and quote worst-frame *and* mean so the ratio
pins it.

**Acceptance is judge-by-eye and the card is the deliverable**: widest zoom-out,
before and after, same scene and seed, **with the count of one-cell-wide
features present in the world and the count actually drawn in the card's
`meta`** — that number is what says thin things stopped vanishing, and a
picture alone cannot.

**For the lab this is nearly free**: `MIN_BOX`/`MAX_BOX` are 128–4096
(`src/lab/params.rs:330`) and played boxes are far below the top, so a
2048x1280 buffer shows a typical box whole at one cell per pixel with no
downsampling anywhere. **The outdoor world is 8192x2560**, so it still needs
stride past a point — this shrinks the problem there rather than removing it.

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
