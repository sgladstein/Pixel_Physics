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

## Landed so far

| commit | what |
|---|---|
| `a09227ca` | `scripts/deadendindex.py` + index TSV + skeleton |
| `3459b009` | the live-flag-is-not-a-live-arm finding |
| `88fd2fd0` | 306 screened entries, rubric, calibration record, two checks |

**306 of 773 screened.** DEAD 194 (63%), META 55, CONFOUNDED 16, COSTED 13,
EXPIRED 12, UNBUILT 11, SUSPECT-INSTRUMENT 2, RE-TESTED 2, UNWIRED 1.
**44 revival candidates (14%).** The register is mostly right; the value is in
locating the minority precisely.

Slice b (plants + field, 201) is complete. Slice d (other, rendering, worldgen,
scheduler, parallelism) has 105 of 193.

## Remaining work, in order — push after every step

### 1. Mirror this plan into the repo, and merge `main`

`branchcheck` reports **4 ahead / 84 behind, BxF 672** — past the 300 bar where
merges get expensive. Merge `main`, run `bash scripts/docscheck.sh`
unconditionally after it (CLAUDE.md), regenerate the index, push.

### 2. Screen slice c — creatures, character, liquids, weather (172 entries)

The owner's own hypothesis lives here, 92% of creature entries predate the
2026-09-06 cluster, and it is where `UNWIRED` is most likely real.

**One Sonnet agent, skeleton-first.** Slice b cost **253k tokens for 101 entries
(~2.5k each)** because screeners opened full entries and cross-checked source;
three runs died on rate limits. So: work from `SLICE_c.md`, open the register
**only** for entries tagged `ALREADY-CONDITION-MET` or where the clause is
genuinely ambiguous, cap source lookups, and **append every ~25 rows to the
output file** so a window boundary costs the tail and not the run.

Push the partial TSV after it lands, even if short.

### 3. Rank the 44+ candidates into a run order

Each needs: the original claim and its number; the defect named in CLAUDE.md's
vocabulary; **the decisive check** — exact command plus what says alive and what
says still dead; cost tier; and what it buys in ethos terms.

### 4. Run the cheap tier, then write the report

`Reports/dead-ends-revival-2026-09-11.md` with its line in `Reports/README.md`
in the same commit. Annotate `dead-ends.md` **only for entries a check actually
settled**, in the register's own house style — strike the superseded clause, add
`CONDITION MET <date>:`. `CONDITION MET AND THE ANSWER IS NO` is a first-class
outcome and the file already carries one. Fix the seven stale section counts.
Wire `deadendindex.py --check` into `scripts/docscheck.sh` beside the
`addrcheck.py` block at line 517.

## What a later session must not re-derive

- **The rubric took three calibration rounds.** `Reports/data/dead-ends-triage/
  rubric.md` is the current one; `calibration.md` records why the first attempt
  was worthless — expected answers built by keyword, a tidy-looking 43%
  agreement, and on every disagreement adjudicated by reading, the rater was
  right and the expectation wrong. Validation is **inter-rater agreement**
  (38/42, κ = 0.82), not ground truth.
- **`UNWIRED` is rare by construction, not broken.** Where the author noticed
  the world couldn't express the thing and wrote it into `Re-test when:`,
  Question 2 has already claimed it as `EXPIRED`.
- **A live ablation flag is not a runnable arm.** `GROUND_ROOT` is correctly
  wired to a branch its scene never reaches. An archived arm needs three steps,
  the first two seconds each: does the scene contain the situation, does the
  switched path execute, and only then do the arms differ.
- **Don't run four screeners at once.** One slice at a time.

## Verification

- `python3 scripts/deadendindex.py --check` — 773 entries, addresses resolve.
- Every screened entry carries exactly one label; per-slice DEAD rates compared
  (a slice far off the others is a rater problem, not a subsystem fact).
- Phase 4's adversarial pass states a **false-negative rate as a number**;
  without it the triage is unverified.
- Before landing: `cargo test --lib`, `cargo clippy --all-targets --release
  --locked -- -D warnings`, `bash scripts/docscheck.sh`.
