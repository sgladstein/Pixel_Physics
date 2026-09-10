# The breeding clock: how many evolutionary generations a session actually gets

*The eusociality lane's first deliverable, and it is a measurement rather than
a feature. The owner's standing question is whether the lab can be made
eusocial — one breeder, sterile workers, castes that behave differently —
without the engine ever learning what a queen is. The answer turns on a
number nobody had: how fast the evolutionary clock ticks under each candidate
breeding rule. Status: **measurement of record for the breeding clock; the
box is not committed to any regime until the owner has seen this**.*

## 0. What was asked, and what binds

Owner rulings carried into this lane, all 2026-09-09:

- **A queen is three authored values over mechanisms that exist** — a
  per-species founding rule, a founder who rests because she is full, sterile
  workers through the caste channel — **never a type the engine knows.**
- **Castes must actually behave differently**, not only breed differently.
  `BrainInput::Made` is a live input, so `(Made, verb)` weights are the
  mechanism: one genome, expressed differently by caste.
- **Rest is the absence of a reason**, not the presence of a full stomach.
- **Fertility is graded**, so a queenless colony's workers resume breeding.

And the sequencing, which is the part this report exists to honour: *first the
measurement, then the build.* Nothing here ships a queen.

## 1. The three regimes are one rule at three settings

The three arms the owner named — individual budding, queen-only breeding, and
graded suppression by proximity to a breeder — turned out not to be three
mechanisms. They are one:

> **An animal's breeding bar is scaled by its proximity to a breeder, where a
> breeder is any animal that has already produced a child.**

- `individual` — the knob at zero. No suppression, no scan, today's world.
- `queen` — colony-wide. While the colony holds another living breeder,
  nobody else buds.
- `graded` — the bar rises smoothly toward a breeder and is exactly 1.0 at
  the radius and beyond.

Three things fall out rather than being authored, which is why this shape was
taken over the alternatives:

**No queen type exists.** There is a distance and a knob. The owner's ruling
is satisfied structurally rather than by discipline.

**The rule bootstraps.** At founding nobody has children, so nobody is
suppressed; the first animal to reach its bar breeds, and *by breeding*
becomes the breeder the rest of the colony reads. The colony finds its queen
instead of being issued one.

**A queenless colony resuming is the same code path as the opening state.**
The owner's grading ruling needs no special case: when no breeder is in range
the factor is exactly 1.0, and that is the same arithmetic that runs on frame
one.

It also avoids inventing a number. The obvious alternative — "a queen
produces workers, and a fraction of her children are new queens" — needs that
fraction authored, and the fraction would then be the thing setting the
generation clock this report is trying to measure.

## 2. Two traps the design is built around

Both would have produced a number that was arithmetically correct and about
the wrong thing, which `CLAUDE.md` names as the worst-recurring failure here.

**The floor.** `try_bud` computes `bar = threshold.max(cost + 1.0)`.
Suppression applied to `threshold` alone is defeated by that floor: a heavily
suppressed worker still breeds the moment it can afford `cost + 1`. The arm
would have read as doing nothing — and "the queen arm does nothing" is a
publishable-looking finding. Suppression multiplies the composed bar, after
the max.

**The counter.** `World::deepest_animal_generation` is a max over every child
ever born. Under queen-only breeding it counts sterile workers that are
genetic dead ends, and reads one step deeper than the chain a genome actually
travels. So `deepest_breeder_generation` was added beside it: the deepest
generation of an animal that has *itself* reproduced. Both are printed, and
**the two diverging is the tell** that an arm is manufacturing dead ends
rather than deepening a line. Under individual budding they run exactly one
apart, every seed, which is the invariant that says the pair is wired right.

## 3. The bed had to be built before anything could be measured on it

Every headless lab figure before this was taken on the harness default: eight
herb founders of one species. The owner does not play that box. Asked for the
mix they actually plant, they said grass, herb and shrub.

**A tree was in the first reading of that answer, and was measured out of it.**
Rendered before anything was run on it, one tree founder becomes 79 plants and
shades the bench to 0.008 of lamp light by frame 30,000:

| bed, 120,000 frames, no ants | light at the bench, 6k -> 120k | plants at 120k |
|---|---|---|
| grass | 0.562 -> 0.562 | 917 |
| herb | 0.320 -> 0.240 | 614 |
| shrub | 0.256 -> 0.110 | 414 |
| one tree | 0.156 -> **0.008** | 79 |

A long run on a bed with a tree in it measures the canopy closing. Put to the
owner as review card `20260909T232116894Z-2b3661`.

**And a grown bed physically refuses a colony.** `found_colony_of` needs bare
footing, and by frame 6,000 the vegetation and its seed litter have the
surface. Founders seated, of the 52 asked for:

| bed at frame 6,000 | founders seated |
|---|---|
| bare bed, no plants at all | 52 |
| 22 plants spread evenly | **8** |
| 13 plants spread evenly | 23 |
| 13 plants, none in the nest band | 33 |

A colony of eight reads in every downstream number as a breeding rule that
never fires. The founder count is the only line in the output that says
otherwise — `CLAUDE.md`'s *a scene that contradicts the code will look like a
bug in the code*. The played bed therefore keeps a bare band at columns
210-310, which is what a player does by eye anyway.

Seating still varies hard with the seed — 33, 13 and 18 on seeds 1, 2 and 3 —
because where the seed litter falls is where the nest cannot go. That spread
is a real property of the played bed and belongs beside any figure taken on it.

## 4. Two harness bugs, both caught by the same tell

**`seed=` was being dropped in scenario mode.** `Scenario::build` reads
`self.bed`, so a seed applied to the local spec reached the census and nothing
else, and every run used the file's pinned seed. Three seeds returned a
*byte-identical* sample row. That is `CLAUDE.md`'s "identical output across a
change that must have moved something", and it was one command away from
turning an eighteen-run sweep into three runs reported six times.

**Nest columns came back empty for a scenario.** A scenario sets
`colonies: 0` and founds on its timeline, so `colony_columns()` is empty and
the whole `d<16 / d<48 / d<128 / far` split would have collapsed into `far` —
a census still printing four columns and meaning none of them.

## 5. The measurement

**Eighteen runs: three regimes x six seeds, the played bed, 120,000 frames,
colony founded at 6,000.** `RAYON_NUM_THREADS` pinned at 4, because every
headline here is a counter and `CLAUDE.md` records a pure count swinging
610 to 278 between an idle box and a loaded one.

**Taken on `main` as of 2026-09-10.** An earlier identical sweep was run and
then thrown away: the merge of `main` brought PR #291's composed move row
onto the ant, which changes foraging and therefore breeding, and a figure
taken on a tree nobody else has is not a figure. Both sweeps agree on every
conclusion below; the individual arm's median is 13.5 in each.

| regime | generations, median | range | breeder chain, median | births | alive at 120k | deaths |
|---|---|---|---|---|---|---|
| individual budding | **13.5** | 8-36 | 12.5 | 273 | 146 | 146 |
| graded suppression | **8.5** | 5-22 | 7.5 | 162 | 141 | **36** |
| queen-only | **1.0** | 1-2 | **0.0** | 11 | 27 | 2 |

Per seed (1-6), deepest generation reached:

```
individual    8  13  13  14  36  17
graded       22   5  13   9   8   6
queen         1   1   1   2   1   1
```

### What it says

**Queen-only is a thirteen-fold collapse of the evolutionary clock, and it is
not close.** Five seeds of six reach exactly one generation; none reaches
three. The breeder chain — the depth a genome actually travels — has a median
of **zero**, meaning that on five seeds of six *no animal born in the box ever
reproduced*. The colony is the queen and her first brood, and that is where it
stops. Reproduction concentrated in one individual makes worker mutations dead
ends, exactly as predicted, and the prediction understated it.

The owner's target of 60-70 generations in a session is unreachable under
queen-only by three orders of magnitude. At individual budding's 13.5 per
120,000 frames it needs about 530,000 frames; under queen-only it needs
roughly seven million.

**Graded suppression costs about a third of the clock and buys stability
rather than population.** 8.5 against 13.5 is a real cost, larger than the
earlier sweep suggested and worth stating plainly. What it buys is not more
animals — 141 alive against 146 is a wash — but **a quarter of the deaths**,
36 against 146. Individual budding is boom-and-bust: seed 5 ran 1,824 births
and 1,684 deaths to stand 154 animals at the end, and the earlier sweep's
seed 1 ate the bed from 4,303 edible cells down to 171 and then starved.
Graded reaches the same standing population without the churn.

**So the choice between individual and graded is a real trade, not a free
win**, and it is the owner's to make: a third of the evolutionary clock, in
exchange for a colony that stops eating itself. What the measurement does
settle is that queen-only is off the table as the box's rule.

### What it does not say

**This does not measure whether castes behave differently.** It measures the
clock only. `Made` is a live brain input and `(Made, verb)` weights are the
mechanism for caste-conditional behaviour, but no species authors a
`Provision` weight yet, so every child in these runs is `made = 0.0` and no
caste exists in any arm. That is the next build, and its instrument is
per-caste verb counters, not this table.

**Nor does it measure colony-against-colony selection**, which is the
condition under which castes are adaptive at all. `scent_spread` ships at
zero, so two colonies are never strangers; these runs hold one colony. Under
queen-only the colony is the unit of selection and a generation is a colony
founding a colony — which cannot happen in a box with one nest and no
dispersal. **The queen number above is therefore the clock for a queen regime
as the box can express it today, not for eusociality as it would work with
competing colonies.** It is still the number that decides whether to commit
the box to queen-only now, and the answer is no.

### The anomaly is CLOSED: the scan was blind to recycled slots

**Cause found and fixed, 2026-09-10.** This report first said that under
`queen` the living-breeder count reached 2 on two seeds where a colony-wide
rule permits one, ruled out colony splitting, a second birth path and a
recycled `children` count, and left the live hypothesis as a non-finite
bank slipping through the infinite bar. **That hypothesis was wrong.** The
cause was in the lookup itself, and it was found by measuring the
per-colony breeder index against the scan it replaces.

**An organism id is `(generation << 12) | slot_index`.** Both scan loops
iterated `1..=slots`, which are bare slot indices and so decode as
generation 0 every time. When a slot is freed and reused its generation is
bumped, so its occupant's real id is 4096 or higher and the loop never
visits it. **The breeding rule was blind to every animal in a recycled
slot** — a queen in a reused slot was invisible to
`colony_has_other_breeder`, so a second animal bred.

It fits every fact the earlier investigation had: the count rose tens of
thousands of frames in rather than at founding, because slots are only
recycled once animals start dying. And it is the same class as the
`0..slots` off-by-one fixed while the regime was built — that corrected the
*range* and left the *encoding* wrong, both from reconstructing an id by
hand instead of asking the allocator.

**Confirmed by re-running the two seeds that leaked**, played bed, 120,000
frames:

| seed | before | after |
|---|---|---|
| 2 | breeders **2** | breeders **1**, born 16, alive 26 |
| 4 | breeders **2** | breeders **1**, born 18, alive 33 |

**The headline is unchanged.** `gen` and `bgen` are identical on both seeds
before and after, so the thirteen-fold collapse stands — and it is now a
tight figure rather than the lower bound this report had to settle for.

**What found it was the instrument, not the reading**, and the general form
is worth keeping. The old path and its replacement were run as two arms of
one binary over a long run on a deliberately crowded bed; their sample
tables were identical for 26,100 frames and then separated by a single
birth. Nothing shorter would have shown it — at 12,000 frames the arms
agree exactly. **An old path raced against its replacement inside one run
is a correctness check no unit test here matched**, because a single
differing decision cascades into a different world by the next frame and is
therefore impossible to miss.

### The second finding is fixed too: `graded` now scales

The lookup used to scan every organism slot — plants included, about 955 on
the played bed at frame 6,000 — once per tick for every animal that could
afford a child. The "rare tick" argument keeping it off the hot path was
made on a starving bed; at the thousand ants the owner already plays at,
that is hundreds of animals against roughly two thousand slots every tick.

`World::colony_breeders` is now a per-colony **candidate list**, never a
source of truth. The invariant is one-directional — *every living breeder is
in its colony's list, and entries that are not breeders may also be in it* —
so false positives are skipped on read and false negatives cannot happen,
because `children` is incremented in exactly one place and that place
pushes. **Nothing was added to any death path**, which is what keeps the
fragile half of a cache out of it: the original doc argued against a cache
on the grounds that a stale "yes" would lock a colony with no breeder alive
to unlock it, and that argument is respected rather than overridden.

**Measured, both arms from one binary on `scenario=crowded_bench`** — one
big colony among thirty plants, built to be the pessimal case for an index
rather than a flattering one, since many small colonies would give the
index almost nothing to walk:

| frames | breeders by then | organisms visited, scan | index | ratio |
|---|---|---|---|---|
| 12,000 | 12 | 1,866,041 | 55,868 | **33x** |
| 40,000 | 42 | 117,739,271 | 10,039,361 | **11.7x** |

The ratio falls as the colony accumulates breeders, which is the honest
scaling story and the reason the bed is crowded on purpose. Under
`individual` the counter reads **zero** — the structural proof that the
common path never looks at all.

**The counter is what makes that claim checkable.** `breeder_scan_visits`
counts candidates on *both* arms, so this is a ratio rather than two
numbers, and a zero on the index arm would mean the lookups had stopped
happening rather than got cheaper — `CLAUDE.md`'s "a cost that vanishes may
be work that vanished". A guard asserts the non-zero half explicitly.

### Is the queen a dead end, or did it fail on implementation?

**The owner's question, 2026-09-10, and the answer is that the collapse is
structural — a better implementation makes it worse, not better.**

Under a colony-wide queen rule, a second breeder can only appear when the
sole breeder dies. So **the evolutionary clock IS the queen replacement
rate**, by construction. The data agrees: every run that reached generation
2 did so by exactly one succession, and the runs that never lost a breeder
never left generation 1.

That is what makes it a dead end rather than a tuning problem. Every
improvement on the list — defining need against the breeding bar so workers
feed the queen preferentially, provisioning her a larger reserve, giving her
workers to defend her — **makes the queen live longer, and a queen who lives
longer is a clock that ticks more slowly.** There is no setting of a
well-implemented queen that runs faster in this box.

**What the box is missing is not a better queen. It is colony-level
reproduction.** In nature a queen-only lineage advances when a daughter
queen *leaves and founds a new colony*; an evolutionary generation is a
colony founding a colony. This box has one nest, no dispersal, and
`scent_spread` at zero so colonies are never strangers — so the only channel
by which the lineage can advance is the sole breeder dying. That is the
whole finding.

**The upside if it were built is castes, not the clock.** Queen-only makes
the colony the unit of selection, which is the only condition under which
sterile castes are adaptive — and castes are the actual goal. But even a
working dispersal-based version would tick *slower* than individual budding,
because a colony has to bank a surplus before it can export a founder.

**So: likely dead, and not worth a playtest now.** A queen-only box is
twenty-five ants with one of them breeding and nothing evolving; there is
nothing in it for a playtest to judge that this table has not already said.
The cheaper bet is that `graded` gets some of the same colony-level
selection — breeding concentrates near breeders without being exclusive — at
a third of the clock rather than all of it, and that is untested and worth
testing once castes exist to select on.

**The condition this rejection depends on**, recorded so it can be re-opened
rather than re-derived: queen-only becomes worth measuring again the moment
the box has **dispersal and colony competition**. If those get built for
other reasons — the rack, predators, two colonies in one bed — re-run this
sweep before assuming the answer still holds.

## 6. What to build, in this order

1. **The owner's ruling on the trade, 2026-09-10: graded.** Put to them
   directly rather than through the review queue, on their own standing
   instruction that a question needing no visual is asked in the session
   ("if it does not require a visual, just ask questions here") — so this
   line is where the ruling lives, because the queue does not hold it.
   Their words: *"I lean graded suppression, but I want to question if the
   queen is dead end or it failed because of the implementation or
   environment."* That second half is answered in §5 above, and the answer
   is that the collapse is structural.

   Graded costs a third of the evolutionary clock and returns a colony that
   does not eat itself. Two conditions before it ships as the default, and
   neither is optional: **the breeder lookup has to scale** (§5's second
   finding — it currently scans every organism in the world), and
   `GRADED_MAX_SUPPRESSION` wants a sweep, being a provisional 6.0 nothing
   has measured.
2. **Resolve the extra-breeder anomaly** before any of it is trusted further.
3. **The caste channel, so sterility is provisioned rather than imposed.**
   `Provision` is a live output nobody wires and `Made` a live input nobody
   reads; a `(Made, verb)` weight is what makes a caste behave differently
   rather than only breed differently. The principle check the owner asked
   for — a line that zeroes the sterility weight reverts to budding workers
   — is already satisfied by `graded`'s own shape.
4. **Need defined so feeding the breeder pays the forager.** The extension
   point is documented beside `kin_deficit` in `creature.rs`: a breeder near
   its breeding bar with a thin bank should read as needier than a worker on
   the same joules, as a separate term or a weighting by fertility, never as
   a change to the plain energy fraction.
5. **Scent spread above zero, so colonies compete.** Without it there is no
   pressure that pays for a caste, and step 3 has nothing to select on.
6. **Per-caste verb counters** — attacks, deliveries, shares, by caste
   bucket — as the instrument, before any claim that castes differ.

## 7. Reproducing this

```
for arm in individual queen graded; do for s in 1 2 3 4 5 6; do
  RAYON_NUM_THREADS=4 PIXEL_PHYSICS_BREEDING=$arm \
    ./target/release/examples/labforage scenario=played_bed frames=120000 seed=$s
done; done
```

`PIXEL_PHYSICS_BREEDING` = `individual` (default) | `queen` | `graded`;
`PIXEL_PHYSICS_BREEDING_RADIUS` (default 24) is read by `graded` only. The
`brdr`, `gen` and `bgen` columns are in every sample row and in `SUMMARY`.
One run is about five minutes.
