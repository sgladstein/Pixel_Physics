# Second opinion on the load-concentration change

Review of branch `load-share` at `43a57b7` and its write-up
`Reports/load-concentration-review.md` (`b4fb357`), by the
structural/destruction session that owns `load.rs`'s neighbours.

**Everything below was re-run against `load-share`'s own binary**, not taken
from the report. Where a claim reproduced, it says so; where it did not, the
numbers are here.

---

## 1. Verified, exactly

| claim | reproduced |
|---|---|
| `room wall=8 dig=0`, outer/inner face mass `157` / `2956` -> `2956` / `2956` | **yes**, to the cell |
| `scripts/acceptance.sh` 16/16 | **yes** |
| `caveshallow` failing-region mean `10.0` -> `4.1`, largest `146` -> `118`, `rock -64` either way | **yes**, to the cell |

The distribution fix works and the behavioural payoff is the right one: a
notched face that carried *nothing at all* now carries its share. That is
what section 2d asked for.

## 2. The best parts

- **Section 7** -- finding that a column's strength is quadratic in width
  where physics says linear, *building* the correction, measuring it
  (`worked` 7 -> 918 overload failures), withdrawing it, and writing up the
  four things a replacement must clear. The first of those four is the
  sharp one: the shelf case and the column case share `base`, so no single
  multiplier can recalibrate.
- **Section 5** -- "a cascade censused before it settles reads a *delay* as
  damage", with the same binary and seeds giving opposite conclusions at
  frame 202 against frame 1,500. That is a new measurement trap and belongs
  in `CLAUDE.md`, not only in this report.
- The guard table naming what each deliberate break fails, **including the
  row where nothing fails**, and the `a_run_with_no_ends...` note about a
  first version that probed a cell whose 40-window never reached the load.

---

## 3. The objection that should gate the merge

### `is_member` does not merely have a cliff at 40 -- sharing never fires on terrain

`is_member` asks whether the cells past each end of the *section* are air,
and `section_cells` is capped at `MAX_SECTION` (40). Natural rock is
hundreds of cells wide, so its window never reaches air, so `is_member` is
false, so **the rule cannot apply to a massif at all**.

The report's own results are consistent with this and read differently once
it is assumed: `dig=6` bit-identical, `terrain` and all five `crack` cases
identical to the cell. Those look like "the change is safe on terrain"; they
may instead mean "the change never ran on terrain".

`CLAUDE.md` states the rule twice, having paid for it twice: *"A size cap
must bound work, never gate whether something happens... Any `if too_big {
return }` is a claim that the largest cases deserve the least behaviour --
check that is what you meant."* Section 6b half-acknowledges this and rates
it third; it is first.

It may well be *right* that sharing applies to built structures and not to
massifs. If so it should be stated as data or as an explicit width test,
not arrive as a side effect of a work cap that exists for a different
reason.

**What would settle it, and it is one counter:** print how many times the
share actually fired, and run a terrain scene. `CLAUDE.md`: "did it fire at
all" needs a counter, not a picture -- and a report of "identical to the
cell" is exactly what a mechanism that never executed produces.

---

## 4. The objection that needs the owner, not a number

### The dust threshold, and `caveshallow`

`MIN_FRACTURE_CELLS` is **6**. A region below it is not fractured at all --
it falls through to per-cell conversion, which *is* powder. `caveshallow`'s
mean failing region moves `10.0` -> **`4.1`**, so the typical event now
lands below the threshold, for an identical material outcome.

That is not the "graded beats binary" trade the report frames it as. It is
the **"everything turns to dust"** failure this project has already
rewritten twice, arriving in a new place. Section 6a ranks it first, which
is right; it should also gate the merge rather than being a follow-up,
because no number settles it -- it needs to be watched in motion.

The genuinely interesting thread is that `roomcut` moves the *opposite* way
(mean 27.6 -> 64.2, i.e. bigger pieces). Same change, opposite granularity.
Understanding why those two differ is worth more than either number.

---

## 5. The frame-cost figures do not reproduce

The report gives `worldcrack strike=12` 11.97 -> 21.67 ms and calls it "the
honest cost and it is not small". Re-run on `load-share`'s own binary:

| measurement | `share=0` | `share=1` |
|---|---|---|
| the report's exact command, `repeat=5` | 30.39 ms | **25.21 ms** |
| same scene, preset and seed pinned, `repeat=3` | 17.87 ms | **15.32 ms** |

`share=1` measured **faster both times**, and the spreads (30-75 ms on one
scene) are far larger than the effect being claimed. So the cost is
unsupported in *either* direction rather than disproved.

This matters beyond the headline: section 6c's case for keeping
`flows_down` rests partly on it costing 33% of a frame, and that number is
equally unreliable.

This machine has been too noisy for frame numbers all session -- one
unchanged scene gave 20.5-50.9 ms across three runs. Re-measure when it is
quiet, and prefer `repeat=5` or more.

---

## 6. Smaller points

- **Drop `flows_down` for now.** It survives on max-but-not-p90, no
  unit-testable geometry distinguishes it (the report says so), and its
  cost is unmeasured. Section 6c is nearly an argument for cutting it
  already. Reinstate it when a second settled sweep at other seeds
  separates it.
- **`vertical_path` as a redundant early-out is fine** and honestly stated.
  No objection.
- **The `capacity` -> `capacity_within` split needs the second pair of eyes
  it asks for.** "Behaviour is intended to be unchanged" is the exact shape
  of the bug `CLAUDE.md` records about commit messages not being evidence.
  A test asserting the two paths agree across a spread of cells closes it
  cheaply.
- **Promote the section 2a finding out of "what is not covered".** `wall=3
  span=200` now loses 48 cells rather than 1,064 and is identical with the
  share on and off, so concentration was *not* its root cause. That retires
  a suspicion the handoff had recorded, and checking it was the right
  instinct.

---

## 7. Recommendation

**Close, but not yet.** The distribution fix itself is correct, well guarded
and well evidenced.

Three things first, in order:

1. **The terrain counter.** Print how often the share fires; run a terrain
   scene; decide whether the `MAX_SECTION` coupling is intended and state it
   as such if it is.
2. **The `caveshallow` granularity, in motion, in front of the owner.** A
   mean of 4.1 against a fracture threshold of 6 is the dust failure until
   somebody watching it says otherwise.
3. **Re-measure frame cost on a quiet machine**, and drop `flows_down`
   unless it earns its place under that measurement.

*Written against `load-share` `43a57b7`, from `master` `bef468a`.*
