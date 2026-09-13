# The first real playtest log — 560,000 frames, five colonies, and where the time goes

*2026-09-13. The owner played a session on the trunk the morning #374 landed
and handed the chronicle over. It is the first log anyone here has had that
was produced by a person playing rather than by a harness, and it is the first
carrying the load columns. The file is committed beside this report at
[`data/playtest-2026-09-13-herb_longant-s1-560k.txt`](data/playtest-2026-09-13-herb_longant-s1-560k.txt).*

**Read §1 before anything else in round 32.** Performance is the owner's
stated first priority and this log sizes it.

## 0. What the session was

`herb_longant`, seed 1, **560,000 frames in 84 minutes of wall clock**, dial at
1024X for all but ~1,500 frames near the end. Box height was raised to 512
during setup. One dial off shipped: `PLANT_LOAD_FAILURE false`.

Five long-ant colonies existed. Four ended, one was alive at the close with
**1,753 ants**; the peak standing population was **2,982**. `LONGANT 3b` split
off `LONGANT 3` at frame 143,000 — colony fission fired once, unprompted.
Lineages reached **72 generations** (`HAIL`, peak 1,104 living) and **51**
(`SCREE`, peak 1,393).

The owner's own summary, which the numbers below bear out: *"3 large colonies
of long ants, 2 died (at least one i think because they ate/damaged all the
plants), but the trees in the area recovered after they died and filled the
area over their old colony (good). There is one colony still thriving."*

## 1. Where the time goes: about two microseconds per ant per tick

**Every figure here comes from the `wall` column** — a unix timestamp written
beside each census row — divided by the frames between rows. It is the only
clock in the file that does not depend on an engine counter, and it is what
the owner actually experiences.

**The tell first, because it decides which statistic is honest.** Adjacent
samples at the *same* ant count differ by **12–14x**, and that spread is as
large below 800 ants as above it. He was using the machine; this is
`CLAUDE.md`'s *a timing number is only as trustworthy as the box was quiet*,
and it means **no single interval means anything** and the spike samples are
not evidence of engine stalls. The defensible estimator under contention is
the **lower envelope** — the minimum cost in each band, which is the closest
thing to an uncontended run.

Fitted on that envelope, over eight octiles of ant count:

| mean ants | measured µs/tick (min) | active sites |
|---|---|---|
| 39 | 1,000 | 1,608 |
| 189 | 1,200 | 1,306 |
| 319 | 1,100 | 843 |
| 441 | 1,300 | 1,254 |
| 802 | 2,000 | 1,444 |
| 1,138 | 3,700 | 2,176 |
| 1,531 | 3,500 | 2,800 |
| 2,473 | 6,300 | 3,588 |

**cost ≈ 1.0 ms/tick + ~2.1 µs per ant per tick.** It holds at both ends
without tuning: 39 ants measures 1,000 µs against 1,087 predicted, and 2,473
ants measures 6,300 against 6,515. So:

| ants | µs/tick | ticks/s | ants as share of the tick |
|---|---|---|---|
| 0 | ~1,000 | ~1,000 | 0% |
| 1,000 | ~3,100 | ~320 | 68% |
| 2,000 | ~5,200 | ~190 | 81% |
| 3,000 | ~7,300 | ~137 | 86% |

**At the population he plays at, the creatures are the frame.** Everything
else together is about a millisecond.

**This is not suspiciously tidy, and the reason matters**: linear in creature
count is the *expected* shape for a per-creature brain-and-move pass, not a
surprising one, and the residuals are messy — the model under-predicts the low
end by a third. What would have been suspicious is a clean curve of some
shape nobody predicted.

### It is not the cell sweep, and it is not the renderer

Two alternative explanations, both refuted by the same file:

- **The sweep.** Over the span where cost rose **6.3x**, `active sites` rose
  **2.2x** (1,608 → 3,588) and `awake chunks` **4.3x** (12 → 52). Worse for
  the hypothesis than the ratios: at 319 mean ants the sites column is at its
  **lowest of the whole session** (843) while cost is flat at 1,100 µs. Cost
  and sites are not even monotone together.
- **The renderer.** `draws skipped` went **2 → 10 across all 560,000 frames**.
  Ten. The loop is not dropping frames to buy ticks, so it is sim-bound, and
  the sim's own budget is being spent on creatures.

### What this log cannot answer, and what to do instead

**It cannot localise the cost inside the creature pass**, and no amount of
re-reading it will. The clock is contended and the file has no per-phase
breakdown — deliberately, since `#374` refused to put stopwatches in the live
loop. **The next step is to replay this bed headlessly on a quiet box** with
`RAYON_NUM_THREADS` pinned and `scale_probe phases=`, which is the instrument
that already exists for exactly this. The chronicle's job was to say *what bed
to build*; it did that and the numbers above are the target to beat.

## 2. Five chronicle columns are dead, and four of them are the nest

Across **all 56 census samples**, each of these is a single constant:

| column | value in all 56 samples | what it is supposed to say |
|---|---|---|
| `roofed` | **0** | empty cells with ground over them — *a nest exists* |
| `pit` | **0** | standing void that is not roofed |
| `pack<` | **0** | worked soil below the surface — the gallery lining |
| `mnd` | **48** | mound height in rows — reads 48 even at 0 mound cells |
| nest band | **`0/0`** | the band every band-scoped column is measured over |

**`roofed` reads zero for a session with 2,982 ants and 356,688 digs.** It is
the column `CLAUDE.md`'s own metric-trap section names as *the* one to read for
excavation — *"what a player calls a nest is roofed void"* — and it is
answering nothing. `mnd` reporting **48 rows high at zero mound cells** is a
number that cannot move, which is the same rule from the other side.

The likely common cause is the last row: the nest band is `0/0`, so anything
scoped to it is measuring an empty set. That is one fix, not five.

> **Corrected 2026-09-13 (round 32 Lane B, PR #387).** It is **two** fixes,
> not one, and they are independent — the band reads `0/0` on a bed whose
> spec and world agree perfectly, and the four footprint columns read their
> constants on a bed that has a band. `lab::census` took "the original
> surface" from `LabBox::ground_y`, which describes the bed that will be
> built on the *next* REBUILD; the owner raised the box height during setup,
> `ground_y` rode the height by design, and the census spent the session
> measuring a datum 96 rows down in the stone base. Separately,
> `census::nest_columns` built the band from the bed spec and the scenario,
> and his five colonies were founded by hand (`FOUNDERS 0  COLONIES 0`).
> **And `pack^` was not working either** — see the correction under §4.
> Full account: [`evolution-lab-census-datum-2026-09-13.md`](evolution-lab-census-datum-2026-09-13.md).

**Until this is repaired the chronicle cannot say anything about the nest**,
which is most of what the owner wants to know from it. `pack^` (mound cells
above the surface) *does* work and is the only structural column that does.

> **Withdrawn 2026-09-13 (PR #387): `pack^` does not work either.** Under
> the same datum drift it counted worked soil in rows `[surface+48,
> surface+96)` — the bottom half of the soil bed, the deep gallery lining —
> as mound standing above the surface. It is *differently* wrong rather than
> dead, which is why it looked alive.

## 3. The land does recover, and it takes about 320,000 frames

This **refutes** the premise round 32's brief was originally given — *"the
abandoned nest never regrows"* — which came from a shorter playtest look.

| frame | bare ground outside the nest | plants | ants |
|---|---|---|---|
| 40,000 | **2%** (19/1024) | 277 | 37 |
| 130,000 | 33% (335/1024) | 45 | 410 |
| 180,000 | **56%** (574/1024) | 24 | 296 |
| 250,000 | 38% (393/1024) | 73 | 590 |
| 450,000 | 22% (229/1024) | 178 | 1,016 |
| 500,000 | **6%** (59/1024) | **409** | 2,037 |
| 560,000 | 6% (61/1024) | 264 | 1,753 |

The colony strips the bed to **56% bare** and it comes back to **6%**, with
the plant count ending **higher than its original peak** (409 against 277).
Recovery from the trough took roughly **320,000 frames** — long enough that
every previous look at this question was taken before it happened.

**So the remaining question is not "does it recover" but "should it take that
long"**, which is a judgement for the owner and not a bug to fix blind.

## 4. The anthill does not exist until frame 360,000

> **This whole section is withdrawn, 2026-09-13 (PR #387).** It rests
> entirely on `pack^`, which the §2 correction shows was measuring the deep
> gallery lining rather than the mound. The 8,352 cells are a real count of
> worked soil; what they are not is a count of mound, so neither the
> 360,000-frame onset below nor its consequence — that a nest-structure card
> must be taken at 400,000+ frames rather than 150,000 — follows from this
> log. **Both need re-taking on a chronicle written after #387.** The
> paragraph is left standing rather than deleted because the owner's §Z18
> verdict it reinterprets is real and the re-take has to start somewhere.

`pack^` — mound cells standing above the surface — is **exactly zero for the
first 350,000 frames**, then 33 at 360,000, and climbs to **8,352** by 560,000.

**This closes round 31's largest finding.** The owner's verdict on Lane B's
§Z18 card was *"none of this reads as an ant hill... just herbs growing in
dirt"*; that card was rendered at **150,000 frames**, which is 210,000 frames
before the first mound cell exists. The bed was not wrong and the ant count
(74) was not the whole story — **the card was taken before the structure it
was meant to show had been built**. Any future card of nest structure must be
at **400,000+ frames**, and that is now a measured floor rather than a guess.

## 5. Deaths, and the run log

**Old age beats starvation in every colony**, consistent with round 31's
lifespan re-take (#376, age 49.4% of deaths):

| colony | old age | starved | outcome |
|---|---|---|---|
| LONGANT 1 | 4,246 | 3,389 | **1,753 alive** |
| LONGANT 3b | 2,392 | 1,733 | ended |
| LONGANT 2 | 1,521 | 819 | ended (1 starved aloft) |
| LONGANT 3 | 75 | 26 | ended |

Exactly **one** death in 560,000 frames is attributed to another animal
(`1 KILLED BY LONGANT 1`), which matches round 30's finding that fighting is
not a real death channel on this bed.

**`LOG DROPPED 79,642`.** The narrative log kept **664 of 15,905** births.
The two-ring split that shipped this morning (#374) stopped a big colony's
churn evicting *line* events, and it did work — 64 line-ended and 47 milestone
events survived — but the individual ring is being overrun by two orders of
magnitude at this population. The cap needs re-deriving against a real colony
rather than against a harness.

> **Done 2026-09-13 (round 32 Lane B), with one correction to the sentence
> above.** *"At this population"* attributes the overrun to the colony, and
> at most half of it is: the ring took **81,690** events, of which at most
> ~46,300 can be animal (15,905 births + 14,203 deaths + `FirstFeed`, which
> fires once per animal), so **~35,400 are the plant stand** — a plant
> pushes `Born` at germination and `Died` when freed, exactly as an ant
> does. The cap moved 2048 → 8192; the load-bearing fix is that `COUNTS:`
> no longer censuses the ring at all. See
> [`evolution-lab-chronicle-counts-2026-09-13.md`](evolution-lab-chronicle-counts-2026-09-13.md).

## 6. What the dial says, and why `debt` is useless here

The dial was at **1024X**, asking for **6,144 ticks per drawn frame**. The box
delivered **95** at frame 10,000 — with **zero ants in the world**. So the
requested rate was never once achievable, `debt` sits pinned near 12,300 for
the entire session, and **the achieved-against-requested ratio carries no
information** in this log: it is saturated from the first sample.

**Read absolute throughput, not the ratio and not `debt`**, whenever the dial
is above what the box can do. The columns that earn their place here are
`wall`, `ants`, `awake`, `sites` and `skip`.
