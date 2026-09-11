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

## Remaining work

Screening, validation and the write-up are **done** — see
[`dead-ends-revival-2026-09-11.md`](dead-ends-revival-2026-09-11.md). What is
left:

1. **Rank the 118 candidates into a run order.** `data/dead-ends-triage/
   candidates.tsv` has them with line numbers and the screener's reason. Each
   needs the original number, the defect in CLAUDE.md's vocabulary, the decisive
   check with **both** expected results, a cost tier, and what it buys in ethos
   terms. Start with the 21 `EXPIRED` — the author named the condition, so the
   hard part is done.
2. **Run the cheap tier.** Smaller than it looks: 7 candidates cite a live flag,
   and a live flag is not a runnable arm.
3. **Close the loop.** `creatures:039` and `hopper` are the same failure twice:
   a mechanism parked against a named condition, the condition met by another
   line, nothing connecting them. The register is grepped by area, so the line
   that meets a condition never reads the entry waiting on it. Nothing in the
   repo fixes this and the sweep does not either.

## What a later session must not re-derive

- **The rubric took three calibration rounds.** `data/dead-ends-triage/
  rubric.md` is current; `calibration.md` records why the first attempt was
  worthless — a keyword-built truth set, a tidy 43%, and the rater right on
  every disagreement. Validation is **inter-rater agreement** (38/42, κ = 0.82),
  never ground truth.
- **The false-negative rate is 3 of 40**, measured, in `adversarial.md`. Do not
  re-run it without a reason; do re-run it if the rubric changes.
- **`UNWIRED` is rare by construction.** Where the author noticed the world
  could not express the thing and wrote it into `Re-test when:`, Question 2
  claims it as `EXPIRED` first.
- **A live ablation flag is not a runnable arm** (`finding-live-flag.md`).
- **Screening costs ~1.7k tokens/entry skeleton-first, 2.5k reading source.**
  One screener at a time; four in parallel exhaust a window.
- **Entry ids are positional and the keys are not.** Join on `key`
  (section+address hash), never on `section:ordinal` — a merge shifted eight
  already-labelled ids once already.
