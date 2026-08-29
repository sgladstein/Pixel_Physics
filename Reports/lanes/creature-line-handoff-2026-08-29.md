# Creature line — handoff, 2026-08-29

**Written by the session that did the work** (`session_01Kj1RL25qeNucfTuHoqirbE`),
because the things a fresh session cannot reconstruct from the commits are:
which of my claims were **wrong**, which numbers are **stale**, and what is
**authorised** rather than merely discussed.

**Read in this order:** this file, then `Reports/creature-motion-design.md`
(the live design, §6 is the owner's calls), then
`Reports/creature-evolution-plan.md` §0 decision log for E9/E10.

---

## 1. STOP — verify this before anything else

**Updated after the first full suite came back.** `cargo test --lib` on the
re-lay: **1004 passed, 3 failed.** All three are now fixed, and one of them
was not a stale literal:

| failed | why | fixed |
|---|---|---|
| `the_block_layout_exactly_fills_the_genome` | pins the block boundaries to literals — fired by design | re-derived: 4096 / 8192 / 8256 / 12,352 |
| `the_genome_manifest_is_pinned` | fired by design; its comment demands the commit say every genome now means something else | 2,369,832,241 -> 1,235,247,055 |
| `the_active_count_matches_a_hand_counted_sparse_genome` | **a latent bug the re-lay exposed** — see below | rewritten through the named accessors |

**The third one is worth understanding before you touch `brain.rs`.** It wrote
genome weights with its own stride arithmetic (`IO_END + input * BRAIN_HIDDEN
+ 1`), which does **not** match how `eval_brain` indexes that block
(`hidden * INPUT_SLOTS + input`). It was wrong before this change and passed
only because the old constants made the wrong arithmetic land on a live slot
by coincidence. `brain.rs`'s accessors exist for exactly this and say so:
*"every caller outside `eval_brain`'s inner loops goes through these, so a
future re-lay is one edit rather than a hunt through hand-written index
expressions — which is how the 6 -> 9 growth got as far as it did."* That test
was the one caller that did not. **If you add any code that indexes a genome,
use `io_slot` / `ih_slot` / `hh_slot` / `ho_slot`.**

**Still to verify before opening a PR** — the full suite has not been re-run
green end to end since these fixes, so run on that branch:

```
cargo test --lib
cargo clippy --all-targets -- -D warnings          # and on 1.98.0, see CLAUDE.md
bash scripts/docscheck.sh                          # was clean at commit time
```

**And one check that is not in the gates and matters more than they do:**
`a655a8e` re-lays the brain genome. Authored species files store *named*
wiring lists rather than slot indices, so `genome_from_wiring` should
re-expand them into the new layout transparently — **but that is an
assumption, not a measurement.** Confirm it:

```
# build ascii from the commit BEFORE a655a8e, keep the binary, then from a655a8e
# run both, diff the counters. They must be byte-identical.
```

If they are not identical, the re-lay changed behaviour and `a655a8e` should
come out. **I did not get to this check.** It is the single highest-value
thing on this list.

---

## 2. What landed today (all merged to `main`)

| PR | what |
|---|---|
| #103 | Colony scene fixed — no panic at its default seed, no ants placed on water, placement follows terrain |
| #106 | An evolved creature can be exported as a species `.ron` the game loads |
| #107 | Creature appearance measured: extent is the only lever, palette exhausted |
| #109 | The creature record re-baselined on today's `main` |
| #110 | Owner decisions E9 (water) and E10 (bodies) recorded |

Three lanes (A appearance, B export, C re-baseline) ran and are **finished and
idle**. Do not re-poke them.

---

## 3. Authorised vs discussed — do not confuse these

**Authorised by the owner, explicitly:**

- **E9** — water response is a heritable trait; drown/float/swim all reachable;
  the float limit is **physical** (mass vs displacement), not genetic.
- **E10** — the ant keeps `Chain(2)`; heritable body comes later via **chain
  length**, one integer.
- **One impulse verb**, and hold slot 12 unnamed with a stated condition.
- **The reserve at 64/64/64** — the subject of `a655a8e`.
- The general standing authorisations in `CLAUDE.md`: open PRs, merge your own
  when CI is green.

**Discussed and NOT authorised — do not start these:**

- The **impulse verb itself**. The design is agreed; nobody said build it.
- The **horizon change** (dropping `start_energy` so ants can die). I offered
  it twice; the owner never answered. **It is not authorised.**
- **S6 reproduction**, `shade-by-cell-type`, heritable chain length, predation.
- **Call 4 of the motion report is still open**: is "creatures can cross gaps"
  wanted at all, given that the current refusal is deliberate and cost two
  attempts? Ask before building the impulse verb.

---

## 4. Things I asserted that were WRONG — do not inherit them

I was wrong repeatedly today in one consistent way, and a fresh session
reading only my confident prose would inherit it.

| I claimed | truth |
|---|---|
| Enlarging the reserve costs "31% more search space" | **0%.** `is_live_slot` gates on the *live* counts; `live_slots()` is 268 either way |
| A verb costs "32 slots, 11%" | **20 live slots, +7.5%** — I used reserved width against `GENOME_LEN`, both wrong denominators |
| The big genome "falls out of L1" | `eval_brain` walks 14 live rows: **1.3 KB vs 3.5 KB**. Both fine |
| Birth cost is "20x and bites at S6" | **~0.5 s across an overnight run.** Negligible |
| §R2's ants-on-water was the colony scene's problem | It was the **canopy**: 217 of 308 columns had a leaf on top on seed 1 |
| "The world feeds a colony for free" | **Overturned by lane C.** It is the *horizon* — the run ends at 22% of an idle lifetime. Food supplies 2.9% of the colony's energy |

**The pattern, which is the useful part: every one was a *proportion* I never
converted into an absolute.** A proportion of a tiny number is tiny. If you
find yourself about to say "N% worse" or "20x", compute the absolute before
you say it.

**Two of my own guards were also defective and are worth knowing about:**

- A water guard that **passed with the fault put back** — blind, not weak. Its
  bed's canopy sat above the cursor row so the scan never met a leaf.
- A `viable` count computed **after** placement, so every ant made its own site
  read as occupied and the number came out below `placed`.

---

## 5. Stale numbers — do not quote these

- **`Reports/creature-motion-design.md` §7's falls-per-move baseline** (11,031
  moves / 1,629 falls = 14.8%) **predates `d007c156` and `4c95233`.** The
  report says so at the number. **Re-take it before gating anything on it.**
- Lane C's economy sweep is a `ba6fc98` reading; `main` moved under it. Flagged
  in its own report.
- Anything in the creature record quoted from before 2026-08-29 was measured
  on a pre-worldgen-change world. Lane C re-baselined the standing guards;
  everything else is suspect.

---

## 6. Where the work goes next, in order

Nothing below is authorised. This is the shape, not a work order.

1. **Ask call 4** (gaps wanted at all?) before touching locomotion.
2. **The impulse verb** — one output, per `creature-motion-design.md`. The
   ballistic physics already exists in `rigid.rs`; the body decides what the
   impulse does via drag and density. **Ships with §7's guards, especially
   falls-per-move**, because two previous attempts put falls at 59–80% of all
   moves and that is the failure this re-opens.
3. **Water trait (E9)** — puts the first carrion in the world, which is what
   the diet gene measured as missing.
4. **The horizon** — the thing that makes anything die, hence the only source
   of selection pressure. **Needs the owner's word.**
5. **S6** — heredity is one line (`OrganismState::genome` is already
   per-individual); the week is the machinery around it. Five hard constraints
   are listed in `creature-review-2026-08.md` §T4.

**Predation is blocked on perception, not hunting** — beetles measured
bit-identical at `beetles=0` and `beetles=9`. The cheap early move is the
parked probe: wire the beetle to the ants' trail channel, one `.ron` edit plus
a rebuild.

---

## 7. Open review cards

None. Both creature cards were answered and are recorded as E9/E10. Run
`python3 scripts/review.py inbox` at session start regardless — the queue is
shared and the sync is a git branch, so a stale local copy reads `open` for a
card that was answered (**this happened to me today**; `review.py sync` first).

---

## 8. Environment notes that cost me time

- **No `/usr/bin/time` on this container.** Use `date +%s.%N` arithmetic.
- **`frame_profile` has no creature scene** — timing a creature change with it
  measures nothing. Use `forage_probe` (creature-only, echoes its parameters).
- A background job can report **exit 0 with every output line an error**. Read
  the output, never the status.
- `cargo test --lib` takes ~4 minutes here after a `brain.rs` edit, because it
  rebuilds everything.
