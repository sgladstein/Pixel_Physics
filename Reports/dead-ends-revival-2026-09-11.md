# Which dead ends are still dead — a full sweep of the register

**Status: complete for screening and validation; the run-order is the open half.
2026-09-11.**

`Reports/dead-ends.md` stops sessions re-walking approaches that failed. It has
no mechanism for noticing when a rejection has gone **stale**, and 97% of its
entries describe a world older than the 2026-09-06 coupling day. 708 of them
name a re-test condition in their own `Re-test when:` clause and **nobody had
ever swept those conditions** — exactly three carried the file's own
`CONDITION MET` marker.

This is that sweep. Every one of the 779 entries now carries a verdict, and the
sweep's own false-negative rate is measured rather than asserted.

## The answer

| | |
|---|--:|
| `DEAD` — structural: a contradiction, a counterexample, arithmetic, or a strictly better replacement | **495** |
| `META` — rejects a harness or a process, not an engine mechanism | 93 |
| `CONFOUNDED` | 57 |
| `UNBUILT` — argued and declined, never measured | 41 |
| `COSTED` | 26 |
| `EXPIRED` | 21 |
| `SUSPECT-INSTRUMENT` | 9 |
| `RE-TESTED` — condition met, retried, still no | 8 |
| `LANDED` — retried, worked, shipped | 6 |
| `UNWIRED` | 2 |

**118 revival candidates, 15%** — and the adversarial pass below puts the true
figure nearer **20%**. **The register is right about roughly two thirds of what
it holds.** That is the headline, and it is worth stating plainly: this is a
well-kept record, and the value of a sweep is locating the minority precisely
rather than casting doubt on the whole.

## The largest actionable class is the owner's own hypothesis

`CONFOUNDED` — 57 entries — is where a mechanism was judged on a failure that a
*neighbouring* subsystem produced. The creature slice alone returned 23 of 176,
the highest rate of any slice.

The shape recurs in a way worth naming, because the fix is one change serving
several entries:

- **Two separate entries** concluded tree wind-lean does not work. `field.rs`
  clamps pressure but not velocity, so an explosion shockwave dominated the
  growth term in both. One neighbouring defect, two buried mechanics.
- **`creatures:049`** — the ants' "am I in the open" distinction failed because
  `LightHere` resolves per *field block*, not per cell. The sensor could not
  carry the distinction; the idea was never tested.
- **`creatures:022`** — `synapse_cost` was held fixed in absolute terms while
  `start_energy` was cut tenfold, silently reallocating its weight. This is
  CLAUDE.md's own *"a correct mechanism at inherited constants is a
  regression"*, inside the register.
- **`plants:015`** — "these species all look the same" was judged while every
  species was ~90% wood drawing from one four-brown palette. The lever was fine;
  texture and colour set the silhouette.

`COSTED` — 26 entries — is the same story for performance. Every "measured
slower" verdict in the register predates `ChunkGrid`, `fxhash.rs`, inlined
`FieldTile` and the rayon job-threshold work, which together made the lab tick
**2.4–5.7× faster, bit-identical**. Several cite precisely the hash-probe cell
lookups that no longer exist.

## The sweep's own error rate: 3 of 40

40 entries sampled at random from the 495 `DEAD`, labels stripped, handed to a
fresh rater asked for the strongest honest case to reopen each. **It reopened 3
— 7.5%**, which over 495 implies about 37 more candidates and a true total near
**155 of 779**. Full account in
[`data/dead-ends-triage/adversarial.md`](data/dead-ends-triage/adversarial.md).

The screen is therefore tight rather than loose — the safer direction for a
register whose job is preventing wasted sessions — and the 15% should be read as
a floor. All three reopens were verified in source:

- **`creatures:013` prescribes deleted code.** `hunger_fraction` has **zero**
  live references; all five hits are comments. `creature.rs:5531` now reads
  *"there is no such decision any more"*.
- **`plants:142`** was measured entirely inside a condition its own source
  comment says is gone, and `stem_run` — one of the two probes it rejected — is
  the shipped mechanism.
- **`rendering:014`**'s named reopen condition, per-glyph verification, is now
  one contact sheet and a `review.py` card.

## The case I got wrong, and what it cost

`creatures:039` was written up here as the clean verified revival and **it is
not one**. The correction is worth more than the claim was.

The entry parks the Jones/Physarum lateral pheromone sensors because both land
in open air for a surface walker in a side-view world — **0.000 over a cell
holding A=27** — and names its own reopening condition: *"correct for anything
moving in open space (a flier, a swimmer)."*

Three things checked out. `flitter` exists and is airborne. The slots survive at
`brain.rs:515,517`, and `creature.rs:3691` fills `inputs[lateral_slot] = r - l`
every tick. No species carries a genome weight on either slot, so the value is
computed and discarded — "kept but unwired", exactly as written.

**The fourth check was never asked, and it overturns the other three.**
`flitter` has no pheromone economy at all: counting real wires rather than
comments, **`flitter` 0 against `ant` 4**. It neither lays a trail nor follows
one, and its own file records the strip as deliberate. Wiring a lateral
*pheromone* sensor onto the one species that moves in open space would sense a
plane that species never writes to and never reads.

The entry asked for "anything moving in open space" as a **proxy** for *a
creature whose lateral offsets are not in dead air*. `flitter` satisfies the
proxy and not the thing it stood for. That is this project's own *"a scene that
contradicts the code will look like a bug in the code"* one level up: the
precondition was checked against the **existence** of a flier rather than
against whether the flier contains the situation under test.

**The general form is the useful part, and it applies to every `EXPIRED` in this
report**: a re-test clause names a condition in the vocabulary available when it
was written, and a later reader matches the words. Verifying that the named
artifact exists is not verifying that the condition is met. Each of the
remaining candidates needs the second check, and a deep read of the 24
highest-prior candidates demoted **15 of them** on exactly this basis — nine
because the work had already been done and recorded in a doc comment or an asset
rather than back in the register.

## Two things measured on the way

**A live ablation flag is not a runnable arm.**
[`finding-live-flag.md`](data/dead-ends-triage/finding-live-flag.md). `GROUND_ROOT`
is the most-cited live flag and is correctly wired — to a branch its own scene
never reaches: paired arms came back byte-identical while `SCHED_PASS=1`
reported `grounded 0 (flat 0)` every frame. An archived arm needs three steps,
the first two seconds each: does the scene contain the situation, does the
switched path execute, and only then do the arms differ.

**A claim standing in a re-test clause can be wrong.**
[`check-structural-005.md`](data/dead-ends-triage/check-structural-005.md).
`structural:005` points at a hierarchical potential and quotes it as packing
into a `u16` at "max offset 239". At the shipped 8192×2560 over three seeds it
is **258** — a ninth bit. Two of its three claims hold; 53 seconds found the one
that does not, in a sentence a future builder would have sized a layout around.

## Method, and what it cost

The rubric took **three calibration rounds**, recorded in
[`calibration.md`](data/dead-ends-triage/calibration.md) because the failures
generalise:

- **The first calibration set was itself the broken instrument.** Expected
  answers were built by keyword — `byte-identical` → instrument fault, an
  `examples/` address → meta — and produced a tidy-looking **43%** agreement. On
  every disagreement adjudicated by reading, **the rater was right and the
  expectation was wrong**. A keyword is not a claim about meaning: the same
  phrase appears in "the instrument could not have moved" and in "the arithmetic
  says this must be identical, and that is the finding". **A ground truth built
  by the same kind of process the classifier replaces cannot validate it.**
- Replaced by **inter-rater agreement**, which needs no ground truth: two
  independent raters, 42 entries, **38 agreed, Cohen's κ = 0.82**. The rubric
  was *reliable*; its label set was *invalid*.
- Rounds two and three added `LANDED` / `RE-TESTED` / `COSTED` — the register
  annotates completed re-tests in place, so "condition met" alone does not say
  whether the answer was yes, no, or unknown — and fixed `UNWIRED`, which never
  fired because its name **collided with the register's own idiom** ("cannot
  express" is used here to mean a mechanism lacks representational power, the
  exact inverse) and because a counter-trap written to stop one label
  over-firing was suppressing it.

Screening cost **~1.7k tokens per entry** working from a generated skeleton with
a capped lookup budget, against **2.5k** when a screener read source; the
skeleton-first run opened the register **four times in 176 entries** with no
loss of quality. Four parallel screeners exhaust a session window before
finishing — three runs died that way.

## What is open

1. **Rank the 118 candidates into a run order.** Each needs the original number,
   the defect named in CLAUDE.md's vocabulary, the decisive check with *both*
   expected results, a cost tier, and what it buys in ethos terms.
2. **Run the cheap tier.** Smaller than it looks: only 7 candidates cite a live
   flag, and a live flag is not an arm.
3. **The loop that is not closed.** `creatures:039` and `hopper` are the same
   failure twice, and nothing makes a met condition findable by the line that
   met it. `deadendindex.py --check` is now gated by `docscheck`, which fixes
   drift in the section counts but not this.

Machine-readable results: [`data/dead-ends-triage/`](data/dead-ends-triage/) —
`screened.tsv` (779 verdicts), `candidates.tsv` (the 118), `rubric.md`, and the
checks above. Regenerate the index with `python3 scripts/deadendindex.py`.
