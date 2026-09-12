# Triaging `Reports/dead-ends.md` — handoff

**This plan is mirrored into the repo at `Reports/dead-ends-triage-handoff.md` so
a later session can pick it up. Keep the two in step; the repo copy is the one
that survives.**

## Context

`Reports/dead-ends.md` holds 773 entries recording approaches this project tried
and rejected. It stops sessions re-walking dead ends, and it works. It has no
mechanism for noticing when a rejection has gone **stale** — and 97% of entries
describe a world older than the 2026-09-06 coupling day.

708 entries carry a `*Re-test when:* <condition>` clause. CLAUDE.md's standing
convention is that *"a dead end whose condition has since changed is due for
re-testing, not permanently banned."* Nobody has ever swept those conditions.

The question, in the owner's words: how many of these were rejected because the
**idea** was wrong, rather than because the **test** was wrong, the experiment
was **confounded** (a creature test that failed because plants reacted), or the
**condition has since changed**.

## Landed so far — **screening is complete, and the tables are now true**

**State 2026-09-12: the review's Part A is done except the adjudication of its
blind re-rating; Part B shipped as `--touching`; Part C's instrument and both
its baselines are on `claude/plants-124-crown` (PR #361).** The 2026-09-11
figures below are kept in their own column, because every one of them moved.

All 806 register entries carry a verdict — 804 distinct keys, and the only two
that share are the genuine duplicate pairs the register deliberately carries.
**The claim that "twelve addresses are listed twice within a section and share
one" was wrong and cost 19 verdicts.** Of 12 shared keys over 33 rows, only 2
were duplicates; the other 10 groups were distinct dead ends filed under one
report-section address, and each group's single verdict stood for all of them.
`stable_key` now hashes the claim's first 160 characters as well.

| label | 09-11 | 09-12 | | label | 09-11 | 09-12 |
|---|--:|--:|---|---|--:|--:|
| DEAD | 495 | **526** | | UNBUILT | 41 | 42 |
| META | 93 | 97 | | COSTED | 26 | 30 |
| CONFOUNDED | 57 | 57 | | EXPIRED | 21 | 8 |
| SUSPECT-INSTRUMENT | 9 | 10 | | RE-TESTED | 8 | 13 |
| LANDED | 6 | 18 | | UNWIRED | 2 | 1 |

**106 rows are labelled candidates (13%), and a blind re-rating puts the number
that stand at about 37% of those — roughly 39.** That correction matters more
than the count: this page previously said the 15% was a *floor*. It is not a
floor, and it is a different set. See the revival report's error-rate section.

By section: plants 25, structural 22, other 15, creatures 13, liquids 9,
weather 8, destruction 6, field 5, powders 4, rendering 4, worldgen 4,
scheduler 2, character 1.

The creature slice carried the owner's own hypothesis — a creature result failing
because a neighbouring subsystem reacted — and confirmed it: **23 of 176 came
back `CONFOUNDED`**, the highest rate of any slice.

### Checks run so far

- **`creatures:039` — verified EXPIRED** (`check-creatures-039.md`). The entry
  parks the Jones/Physarum lateral pheromone sensors because both land in open
  air for a side-view walker (0.000 over a cell holding A=27) and names its own
  reopening condition: *"Correct for anything moving in open space (a flier, a
  swimmer)."* `flitter` now exists and is airborne; the slots survive at
  `brain.rs:515,517`; **no species carries a lateral weight** — ten `.ron` files
  match the names and every match is a comment. The condition arrived and nobody
  rewired the sensors.
- **`structural:005`'s named replacement** (`check-structural-005.md`). Two of
  its three claims hold at the shipped 8192×2560 over three seeds; **the packing
  claim fails** — max offset 258 against the 239 quoted, needing a ninth bit.
- **A live flag is not a runnable arm** (`finding-live-flag.md`). `GROUND_ROOT`
  is correctly wired to a branch its own scene never reaches.

## Remaining work

Screening, validation and the write-up are **done** — see
[`dead-ends-revival-2026-09-11.md`](dead-ends-revival-2026-09-11.md). What is
left:

1. **Finish the adjudication.** The blind re-rating disagreed with the screen on
   71 of 212 rows and every disagreement is meant to be settled by reading
   before a candidate is kept. Until that lands, `candidates.tsv`'s 106 rows are
   the screen's list, not the adjudicated one, and about two thirds of them are
   expected to close.
2. **Then rank what survives.** `data/dead-ends-triage/candidates.tsv` is now
   *generated* from `screened.tsv` rather than maintained, so it cannot drift
   again — it used to carry 61 `CONFOUNDED` against the screen's 59, with a
   withdrawn entry as its first row. Each survivor needs the original number,
   the defect in CLAUDE.md's vocabulary, the decisive check with **both**
   expected results, a cost tier, and what it buys in ethos terms.
3. **Run the cheap tier.** Smaller than it looks: 7 candidates cite a live flag,
   and a live flag is not a runnable arm.
4. **The loop is half closed.** `deadendindex.py --touching` asks the arrival
   question — does this branch's diff *add* an identifier some `Re-test when:`
   clause names — and `branchcheck.sh --brief` runs it at session start. Recall
   is 2 of 5 on replay, so **silence is not evidence**. The half that remains is
   the **68 entries with no clause at all**, 50 of which name a source file:
   those are invisible to any name-matching rule by construction, and giving
   them a clause is the only thing that closes it.

## What a later session must not re-derive

- **The rubric took three calibration rounds.** `data/dead-ends-triage/
  rubric.md` is current; `calibration.md` records why the first attempt was
  worthless — a keyword-built truth set, a tidy 43%, and the rater right on
  every disagreement. Validation is **inter-rater agreement** (38/42, κ = 0.82),
  never ground truth.
- **There are two error rates and the screen only measured one.** The false
  negatives are 3 of 40 in `adversarial.md`, replicated at 4 of 40 with a
  neutral prompt and pooling to 7 of 80 — corroborated, and six of the seven are
  *stale* rather than revivable. **The false positives are the larger error**:
  the candidate list re-rated blind against 106 `DEAD` controls comes back
  **37% precise**, with only 4 of 106 controls reopened. Do not quote the
  candidate count as if it were a work list.
- **Two rubric rules are under-applied by default and must be quoted into any
  screening prompt**: rule A (delete the suspect number — does the entry still
  reject?) and Question 1 (`META` beats everything). Both failures run in the
  same direction, keeping entries open: `CONFOUNDED` closed to `DEAD` 21 times
  in one blind batch, `SUSPECT-INSTRUMENT` re-read as `META` six times.
- **`UNWIRED` is rare by construction.** Where the author noticed the world
  could not express the thing and wrote it into `Re-test when:`, Question 2
  claims it as `EXPIRED` first.
- **A live ablation flag is not a runnable arm** (`finding-live-flag.md`).
- **Screening costs ~1.7k tokens/entry skeleton-first, 2.5k reading source.**
  One screener at a time; four in parallel exhaust a window.
- **Entry ids are positional and the keys are not.** Join on `key`, never on
  `section:ordinal` — a merge shifted eight already-labelled ids once, and the
  re-join found a ninth: one `screened.tsv` row was still labelled `other:107`
  for what is now `other:110`. **The key is section + address + the claim's
  first 160 characters**; section + address alone collides, which is how 19
  entries ended up carrying a neighbour's verdict.
- **`--check` is a real gate now and was not before.** It used to call
  `write_outputs()` before reading its own flag, so it rewrote both generated
  files on every run and compared only the `##` heading counts against a count
  it had just produced. In a depth-1 clone — which is every cloud session — it
  exited 0 while rewriting 1,585 lines with the boundary commit's dates, and CI
  ran that and passed. It now renders in memory, compares on the non-blame
  columns, refuses to *regenerate* when the clone is shallow, and fails when any
  entry has no verdict.
