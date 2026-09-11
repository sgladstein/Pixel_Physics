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

## Landed so far — **screening is complete**

All 779 register rows now carry a verdict (758 distinct dead ends; twelve
addresses are listed twice within a section and share one).

| label | n | | label | n |
|---|--:|---|---|--:|
| DEAD | 495 | | UNBUILT | 41 |
| META | 93 | | COSTED | 26 |
| CONFOUNDED | 57 | | EXPIRED | 21 |
| SUSPECT-INSTRUMENT | 9 | | RE-TESTED | 8 |
| LANDED | 6 | | UNWIRED | 2 |

**118 revival candidates (15%).** The register is right about roughly two thirds
of what it holds, and the value of the sweep is in locating the rest precisely.

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

## Remaining work, in order — push after every step

### 1. Finish screening — DONE

**structural is complete (90/90)**: 58 DEAD, 13 CONFOUNDED, 4 EXPIRED, 4 COSTED,
3 UNBUILT, 3 META, **3 LANDED**, 1 UNWIRED, 1 RE-TESTED. The screener died on a
window boundary before reaching destruction or powders; the rows survived
because it was appending as it went, which is why that rule is in the brief.

Left: destruction 99, powders 18, rendering 42, worldgen 23, scheduler 13,
parallelism 10, plus two stragglers in `other`. Then
rendering + worldgen + scheduler + parallelism (88), plus two stragglers in
`other`.

**One Sonnet screener at a time, skeleton-first.** Measured cost: **~1.7k tokens
per entry** when the screener works from the skeleton with a capped lookup
budget, against **2.5k** when it reads source — and the skeleton-first run
opened the register **four times in 176 entries** with no loss of quality. Four
parallel screeners exhaust a 5-hour window before finishing; three runs died
that way.

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
