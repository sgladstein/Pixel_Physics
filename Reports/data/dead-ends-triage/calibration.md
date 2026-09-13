# Calibration result: the validation set was the broken instrument

## What was run

42 entries from `dead-ends.md`, given in full, classified blind against
`RUBRIC.md` by an Opus agent. Expected answers were built mechanically:

- `EXPIRED` (4) — entries carrying the file's own `CONDITION MET` marker
- `SUSPECT-INSTRUMENT` (14) — entries whose prose matches `vacuous|byte-identical|
  counter reads zero|exactly-zero|moved nothing`
- `DEAD` (16) — `PERMANENT`-family entries matching `contradictor|counterexample|
  impossib|mutually exclusive|by construction|strictly better|superseded`
- `META` (8) — entries whose address begins `examples/`, `scripts/` or `tests/`

**Exact agreement: 18/42 = 43%.**

## Why that number means nothing about the rubric

Four disagreements were adjudicated by reading the entries. **The rater was
right in all four and the expected answer was wrong in all four.**

| entry | expected | rater | who was right |
|---|---|---|---|
| `destruction:010` | SUSPECT-INSTRUMENT | DEAD | **rater** — "byte-identical" is present, but a new counter fired in the same run (44 pieces cut loose, largest 292). The instrument was demonstrably live; the hypothesis was falsified. |
| `other:077` | SUSPECT-INSTRUMENT | DEAD | **rater** — the byte-identical arm is an *explained arithmetic consequence*: the form is `base*(1+draw*variance)`, so zero times any genome is zero. Widening the variance is provably vacuous, which is the finding, not a defect in it. |
| `rendering:004` | META | DEAD | **rater** — the address is `examples/subpixel.rs`, but the entry rejects a *rendering mechanism* (normalising a reconstruction gradient into a unit surface normal). `subpixel.rs` is a testbed where mechanisms live, not a test of one. |
| `structural:073` | DEAD | META | **rater** — the entry withdraws an *inference from an A/B instrument* ("delete a layer, nothing changes, so the layer was not doing the work"), corrected by a direct count of 1,305 fallback saves. What was rejected is a way of reasoning, not a mechanism. |

The two ground-truth heuristics failed in opposite directions and for the same
reason: **a keyword is not a claim about meaning.** `byte-identical` appears
both in "the instrument could not have moved" and in "the arithmetic says this
must be identical, and that is the result". An `examples/` address appears both
on "our harness was wrong" and on "the mechanism we prototyped there is wrong".

The one stratum with real ground truth — the file's own `CONDITION MET` marker
— the rater handled correctly where it was checkable. `scheduler:006` is marked
**CONDITION MET AND THE ANSWER IS NO**: the condition was met, the re-test was
run, and it failed. The expected answer said `EXPIRED` (i.e. still to be
checked); the rater said `DEAD` and named the 2026-09-05 re-test. The rater is
right, and the mechanical rule that produced the expectation cannot see the
difference between a condition that is *open* and one that has been *closed by
measurement*.

## The rule this is an instance of

`CLAUDE.md`: *ask what your number counts when nothing is wrong.* The 43% is
arithmetically correct and answers a different question than the one asked — it
measures agreement between the rater and a keyword matcher, and the keyword
matcher is not an authority on anything. It was also **tidy**: a clean 43% with
a legible-looking confusion matrix, which is the documented tell.

The rule extends here in a way worth carrying: **a ground truth built by the
same kind of process the classifier is replacing cannot validate that
classifier.** A semantic judgement needs semantic ground truth — read by hand,
or the artifact's own declaration — and there is no cheap mechanical substitute.

## What replaced it

Ground truth is expensive and mostly unavailable, so the check that *is*
affordable measures the property the fan-out actually depends on: **inter-rater
agreement.** Two Opus raters, the same 42 entries, the same rubric,
independently. High agreement says the rubric is specified tightly enough that
four screeners will not each invent their own taxonomy; low agreement says the
label boundaries need work before 773 entries are pushed through them. It needs
no ground truth, which is the point.
