# Which dead ends are still dead — a full sweep of the register

**Status: complete for screening and validation; the run-order is the open half.
2026-09-11.**

`Reports/dead-ends.md` stops sessions re-walking approaches that failed. It has
no mechanism for noticing when a rejection has gone **stale**, and 97% of its
entries describe a world older than the 2026-09-06 coupling day. 708 of them
name a re-test condition in their own `Re-test when:` clause and **nobody had
ever swept those conditions** — exactly three carried the file's own
`CONDITION MET` marker.

This is that sweep. Every one of the 807 entries now carries a verdict, and
**both** of the sweep's error rates are measured rather than asserted — the
false negatives it misses in `DEAD`, and, since the 2026-09-12 review, the
false positives in its own candidate list.

## The answer

**Revised 2026-09-12**, after the re-key found 19 entries carrying a
neighbour's verdict, the 19 `main` added were screened, and the relabels of the
review's §2c and §4 were applied. The 2026-09-11 column is kept so the movement
is visible.

| | 09-11 | 09-12 |
|---|--:|--:|
| `DEAD` — structural: a contradiction, a counterexample, arithmetic, or a strictly better replacement | 495 | **563** |
| `META` — rejects a harness or a process, not an engine mechanism | 93 | **109** |
| `UNBUILT` — argued and declined, never measured | 41 | 43 |
| `LANDED` — retried, worked, shipped | 6 | 23 |
| `COSTED` | 26 | 21 |
| `CONFOUNDED` | 57 | **19** |
| `RE-TESTED` — condition met, retried, still no | 8 | 18 |
| `EXPIRED` | 21 | 7 |
| `SUSPECT-INSTRUMENT` | 9 | **1** |
| `UNWIRED` | 2 | 1 |
| | **758** | **805** |

**The 09-11 column is the distribution recoverable from `screened.tsv` at that
day's commit, not the table this page first published**, which was hand-built at
a slightly different moment and disagreed with its own data in five of ten rows
(it read `DEAD` 497, `EXPIRED` 12, `RE-TESTED` 11, `LANDED` 13,
`SUSPECT-INSTRUMENT` 10). Both columns now come from the file, and the 09-12 one
is what `deadendindex.py --check` gates.

**49 revival candidates, 6% — not the 118 this page first reported.** Every
labelled candidate was re-rated blind and every disagreement settled by reading
source; the account is below. Two classes carried almost all of the error:
`CONFOUNDED` fell 57 → 19 and `SUSPECT-INSTRUMENT` 10 → 1, the latter almost
entirely into `META`, because the thing being rejected *was* the instrument.

**The register is right about far more of what it holds than the first pass
credited** — and that is the finding, not a disappointment. A register whose job
is stopping wasted sessions is supposed to be mostly right, and the value of a
sweep is locating the minority precisely rather than casting doubt on the whole.

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
  *(`creatures:022` stood here as the showcase example and has been removed,
  2026-09-12: it records that the confounded sweep was "invalidated and
  restarted", which is a process already handled. Question 1 of the rubric
  claims it as `META` before `CONFOUNDED` can — the first sign that this class
  was over-assigned.)*
- **`plants:015`** — "these species all look the same" was judged while every
  species was ~90% wood drawing from one four-brown palette. The lever was fine;
  texture and colour set the silhouette.

`COSTED` — 26 entries — is the same story for performance. Every "measured
slower" verdict in the register predates `ChunkGrid`, `fxhash.rs`, inlined
`FieldTile` and the rayon job-threshold work, which together made the lab tick
**2.4–5.7× faster, bit-identical**. Several cite precisely the hash-probe cell
lookups that no longer exist.

## The sweep's two error rates, and only one of them was measured here

### The false negatives: 3 of 40, and it replicates

40 entries sampled at random from the 495 `DEAD`, labels stripped, handed to a
fresh rater asked for the strongest honest case to reopen each. **It reopened 3
— 7.5%.** Full account in
[`data/dead-ends-triage/adversarial.md`](data/dead-ends-triage/adversarial.md).

**The 2026-09-12 review drew a second sample of 40 with a neutral prompt and
reopened 4**, pooling to **7 of 80, 8.75%** — so this figure is corroborated and
the anchoring in the first prompt ("expect most to hold") did not move it. Note
what the seven are, because it changes what they are worth: **six are stale
rather than revivable** — the world moved and the entry was resolved or
superseded elsewhere without a write-back — and only `destruction:034` is a
condition met with the re-test still owed.

### The false positives: this page did not measure them, and they are the larger error

**The screen was never tested on its own candidates, and blind they mostly do
not stand.** All 106 candidate rows were re-rated blind against the same rubric,
labels stripped, blended with 106 `DEAD` controls so the base rate told the
rater nothing, with the two rules a first review found under-applied quoted in
the prompt — rule A (delete the suspect number; does the entry still reject?)
and Question 1 (`META` beats everything).

| | n | |
|---|--:|---|
| candidates confirmed blind | 39 of 106 | **precision 37%** |
| `DEAD` controls reopened | 4 of 106 | 3.8%, so the rater is not simply closing everything |

The controls are what make the 37% mean anything: a rater that closed everything
would have reopened none of the 106 `DEAD`.

**Then every disagreement was settled by reading.** The two readings differed on
71 of the 212 rows, and a third pass adjudicated each against source: **63 went
to the blind reading, 7 to the screen, and 1 to neither** (`Blast::calve`'s
release bound, which is `COSTED` — its cost figure is independent of the
explained null, and `default_joint_density` is still `0.9`, which is the
reopening condition the rubric says not to bury in `DEAD`).

**Candidates: 106 → 49**, from the 118 first published: the nine write-backs
took it to 111, the relabels to 101, the orphan screen put 5 back, and
adjudication took the 106 that went into the blind pass down to 49. Two classes
carry almost the whole error, and both are
failures of the same two rules. `CONFOUNDED` fell 57 → 19 — rule A, delete the
suspect number and the entry still rejects. `SUSPECT-INSTRUMENT` fell 10 → 1,
almost all of it into `META` — Question 1, the thing being rejected *was* the
instrument, so reviving it would change a measurement rather than the engine.

**So the reading this page originally gave — "the screen is tight rather than
loose, and the 15% is a floor" — is withdrawn, and the correction is not a
tightening of that number but a different number and a different set.** The real
candidate list is **49**, and the ~40 entries the `DEAD` side hides are mostly
the write-back kind rather than the revivable kind: six of the seven pooled
reopens are stale, not revivable.

The `DEAD` side reopens were verified in source:

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

## Landed since: the nine, and the gap they expose

The deep read of the 24 highest-prior candidates demoted 15, and **nine of those
were closable with a source citation rather than a measurement** — the re-test
had been done and recorded in a doc comment or an asset, and never propagated
back. All nine are now annotated in the register with `CONDITION MET`, each
verified in source first. (Every line number the deep read reported had shifted
with a merge, so each was re-located by content — trusting them would have
written nine citations pointing at the wrong code.)

`plants:019` is the one worth noticing: it carries **no `Re-test when:` clause
at all**, which is why nothing ever asked whether its condition had arrived.
**68 entries are in that position** (re-counted 2026-09-12 on the regenerated
index; this page and the PR body both said 71 and the tables said 65) — they
cannot go stale *visibly*, because they never stated what would make them
stale. **50 of the 68 name a source file or an asset**, which is the set where
writing a clause would actually pay: `deadendindex.py --touching` can only see
an entry whose clause names the arriving identifier, so these 50 are invisible
to it by construction and `plants:019` is the control that proves it.

That is the structural gap this sweep found and did not close: **the register is
read by area and resolved in code.** `creatures:039` fell through it one way — a
condition met by another line's work, unnoticed — and these nine fell through it
the other, resolved and never written back.

## What is open

1. **Rank the remaining candidates into a run order.** Each needs the original number,
   the defect named in CLAUDE.md's vocabulary, the decisive check with *both*
   expected results, a cost tier, and what it buys in ethos terms.
2. **Run the cheap tier.** Smaller than it looks: only 7 candidates cite a live
   flag, and a live flag is not an arm.
3. **The loop, half closed 2026-09-12.** `creatures:039` and `hopper` are the
   same failure twice, and nothing made a met condition findable by the line
   that met it. `deadendindex.py --touching` now asks the *arrival* question —
   does this branch's diff add, to a tree that did not have it, an identifier
   some `Re-test when:` clause names — and `branchcheck.sh --brief` runs it at
   session start. **Recall is 2 of 5 on replay and silence is not evidence**:
   it cannot see an entry with no clause (the 68 above), nor a condition met by
   a value changing rather than a name arriving, which is how `field:027` and
   `destruction:034` both slipped past it. Specificity is why it ships anyway —
   0, 0, 0, 0, 1 hits over five unrelated merged PRs. The looser form that
   catches all five controls scores 43 hits against one true positive on a
   plant-line commit and is recorded as a dead end.

Machine-readable results: [`data/dead-ends-triage/`](data/dead-ends-triage/) —
`screened.tsv` (one verdict per distinct entry, the adjudicated ones marked in
their reason line), `candidates.tsv` (the 49, regenerated from `screened.tsv`
rather than maintained), `rubric.md`, and the
checks above. Regenerate the index with `python3 scripts/deadendindex.py`.
