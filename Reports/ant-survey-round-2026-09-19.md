# The ant-survey follow-up round — what three lanes overturned, and what waits on the owner

*Round record, 2026-09-19, written by the coordinator
(`session_01HTNLNphUPgpg5GqCwCQvmW`) at the close. Brief:
[`ant-survey-followup-brief-2026-09-19.md`](ant-survey-followup-brief-2026-09-19.md).
The round was the owner's instruction after reading
[`ant-sim-research-review-2026-09-19.md`](ant-sim-research-review-2026-09-19.md):
one lane to build the review's performance proposals, two to re-evaluate the
survey-backed mechanisms this repo had rejected, under **"don't trust past
results."** Docs only.*

## 0. What the round overturned, in one line each

1. **The record was a day stale on the one thing the whole trail line turns
   on.** The food-trail reader was de-saturated on 2026-09-18 (`ac02ac03`:
   units 2/3's gate `Bias 45 / CarryingFood −75` → `0.5 / −45.5`), and Lane T
   measured that change as **the whole of the hand-trail effect** — 22
   colonies of 36 alive against 4 with the old deaf pair, 52% of ants at the
   food against 5%. `dead-ends.md`, `wiki/ants.md`, `trailfollow`'s note and
   the coordinator's review all still said *deaf on purpose*. Corrected on
   PR #478. (Lane T, Fable.)
2. **The nest's lever was in the tree, switched off, swept in the wrong
   axis.** `PIXEL_PHYSICS_NEST_SITE_ROWS=40` gives room **1.44x on 12 of 12**
   seeds and height-over-width **0.11 → 0.20**, taller on 11 of 12, with
   shafts under the chamber; only its sibling `_COLS` had ever been scored.
   The three `Crowding` nulls hold, and hold *harder*, because the local
   reading desaturates the input and the nest still does not move. Merged as
   PR #477. (Lane N, Opus.)
3. **The field's response to a walking colony is real, and no wake rule can
   reach it.** The write-seam skip is bit-identical and ships as a switch;
   the solve set does not move by one tile (65.4/tick either way) because the
   lab bed already solves half the world for plants and sky and every ant
   stands inside that halo. What ants add is CA block rescans, 85% of them
   earned by digging, dropping and disturbing soil water. Merged as PR #479.
   (Lane P, Opus.)

**And one rejection fell.** Channel B fading faster (rho 0.25, in the
literature band the register had rejected on `u8` planes) is inert where a
trail is hand-laid to one pile and **positive on the played bed with a
reader that can hear it**: intake better on 10 seeds of 12 (median 54,722 →
127,339 J), unvisited larder lower on 11 of 12. A dial's range, 0.03–0.25,
under *expose, not tune*. (Lane T.)

## 1. The lanes

| lane | session | model | branch → PR | state at close | cost |
|---|---|---|---|---|---|
| N, nest | `session_01UXdunmxDzJjPeuzNmBUK1R` | Opus 5 | `claude/ant-survey-nest` → **#477, merged** `5f9cf93a` | done; blind card `20260919T151821144Z-da82d9` unanswered | $46.8 |
| P, performance | `session_01NNfMrfBtARTHKwoZxidxRo` | Opus 5 | `claude/ant-survey-perf` → **#479, merged** `40d9d65a` | done; kept running past its brief and was stopped | $69.5 |
| T, trail | `session_018P1mVfE1HKA1uDesCHWw3Y` | Fable 5.1 | `claude/ant-survey-trail` → **#478, open, green** | done; **waits on the owner** (§4) | $137.1 |

Costs are the sessions' own `cost_usd` at 21:32Z; the coordinator's is not
in them. Both Opus lanes and the Fable lane produced a report the round could
use; the Fable lane cost 2–3x either Opus lane, and about half of that was
run length after its brief was delivered (§5).

## 2. What each lane found, beyond §0

**Lane T** — `Reports/ant-survey-trail-reevaluation-2026-09-19.md`, on PR #478's
branch only until it lands (`git show origin/claude/ant-survey-trail:Reports/ant-survey-trail-reevaluation-2026-09-19.md`). Five candidates at 36 seeds, paired within seed, on the bed with a
return leg (cut from `claude/upbeat-shannon-cez0w4`, see §4): the
food-charged `EmitB` odometer is **inert at both reader gains** — the two
earlier tests had each varied only one factor; `(Crowding, EmitB, −w)` is null
at 2.35 and its survival gain at 4.5 is the emission cost falling, shown by a
no-channel-B control; the no-entry mark was **not built** because `self ≡
mute` — there is no recruitment to a stale patch for it to correct;
trophallaxis on the played bed **delays and steepens** the founding cliff
rather than flattening it (nine seeds of twelve behind by frame 9,000);
laden right-of-way was scored but not built (this lane tests). Two standing
facts: **the colony's own channel B is a net cost** on the return-leg bed (29
colonies alive laying none against 22), and **no candidate produces a colony
that can lay a trail worth following**, which the survey's core
recommendation presupposes. Also: `trailfollow`'s default `gate=` is the
stale `saturated` preset, so any archived run without an explicit gate is a
three-change comparison.

**Lane N** — [`nest-rejections-rescored-2026-09-19.md`](nest-rejections-rescored-2026-09-19.md).
Two of the three suspected faults were real (the census undercounted the nest
threefold; every published arm was one run in a seed-free box) and fixing
them changed no verdict. The `LightHere` spoil-drop gate fails **harder** with
the per-cell in-the-open reading its entry asked for (0.49x room, better on 0
of 12); Khuong's pellet rule was **declined before building** — one drop in
ninety ever has a marked and an unmarked candidate, and `line_burrow` relabels
two thirds of the mound into gallery lining after it lands; the downward dig
bias at the *turn* buys volume (1.51x) and not shape. Four default-off
switches, `digbox seed=`, and an `iqr` column because a bounding box is a max
statistic that calls scattering the bigger nest.

**Lane P** — [`ant-field-wake-2026-09-19.md`](ant-field-wake-2026-09-19.md)
and [`gpu-field-design-2026-09-19.md`](gpu-field-design-2026-09-19.md). The
per-phase stopwatch ships gated (`PIXEL_PHYSICS_PHASE_CLOCK=1`), free when off,
and already reads **field 20–21%, sweep 56–60%** of a lab tick — the owner's
bed is his to take. `FIELD_CREATURE_WAKE=blocked` is bit-identical and buys
~0.002 ms; `=0` is *not* bit-identical because the momentum passes run over
the solve set, and the air in a sealed box is substantially the ants stirring
it. The GPU field's condition is not met by ~1.6x and the note argues the
share was never the deciding number: the readback is per tick and the
sleeping-tile economy is what a per-pixel device cannot express. **Bug §Z31**
filed: `field::step`'s carry decision keys on `is_settled()` on a premise
that fails, and moves the field hash on an ant-free bed.

## 3. What the lanes corrected in the coordinator's own review

Recorded because the review is on `main` and will be read.

| review said | lane found | fixed where |
|---|---|---|
| §2.3: the food route is *"left near-deaf on purpose"*; §3 items 2–3 aim at §Z7 as open | de-saturated on the 18th; the reader is the whole hand-trail effect | T; the review's §2.3 row and §3 items carry a dated outcome line from this PR |
| §8.2: *"a creature cell neither blocks nor sources anything the field solves"* | true for five arrays, and `glow`/`beam` **are** read from a creature cell, answering zero only because no creature material sets them | P, §1.2 — the switch is keyed on the emission |
| §8.3: awake chunks "27.9–31.1" at three populations | one population wearing three labels (the stocking loop saturates); the spread is rep selection | P; the review's §8 already says the stocking caps, the range was misread |
| §8.3: field 0.090 → 0.188 ms for 52 ants | 0.081 → 0.117 on the same box the same day: same direction, 1.44x not 2.09x | P; do not quote the multiplier unmeasured |
| §3 item 6: Khuong's rule as a cheap candidate | one drop in ninety has a choice; the eraser is `line_burrow` | N, declined with reopening conditions in `dead-ends.md` |
| the nest plan's §1: *"interventions on whether cannot produce a shape"* (quoted approvingly) | a scalar cannot; one applied over a region inherits the region's | N |

## 4. What waits on the owner

1. **The lifetime split.** Lane T branched from `claude/upbeat-shannon-cez0w4`
   (the owner's live trail session, `session_01X2rqLneJDzw21CcqoFD7aE`) because
   it is the only bed with a return leg, so **PR #478 carries that branch's
   twenty commits** (`TRAIL_A_RHO 0`). Land the split through #478, or
   through the trail session's own PR and let #478 shrink to Lane T's docs,
   write-backs and harness riders. The coordinator will not merge it without
   that word.
2. **The nest site reach.** The blind A/B card is the first half of the
   decision whether `NEST_SITE_ROWS=40` ships as a value. Lane N's note says
   why it should and why it will not say so on its own.
3. **Channel B's fade as a dial** (0.03–0.25), per *expose, not tune*.
4. **`home_bias`** is still at 0.0 with no dial and no wiki line (the review's
   own finding; untouched by this round).

## 5. What the round cost to run, and what it should not cost again

- **The account's five-hour usage limit stopped all three lanes at ~16:20Z**
  with Lane P's two hours of work unpushed in its container. A poke at 20:14Z,
  after the reset, resumed both P and T and each pushed within twenty minutes.
  **Brief every lane to push before its first sweep, not after its last.**
- **Two lanes kept running after their deliverable landed** — P onto a
  follow-on nobody asked for, on a branch that had already merged; T past its
  five candidates. Both were interrupted at 21:33Z and told to save unpushed
  work on a fresh branch. *The lane that wrecks a budget is the long one.*
  Put a stop condition in the brief: **open the PR and end the turn.**
- **Pokes do not strip the GitHub tools after all** — both lanes opened their
  own PRs after being poked, against what the brief assumed. The skill's
  line should say *a trigger-fired fresh session* has none; a poked child of
  a session that had them keeps them.
- **`deadendindex.py` refuses a shallow clone.** Resolving a generated-index
  conflict needs `git fetch --unshallow` first (five seconds here).
- **A `mergeable_state` read is stale within a minute.** #477 read `clean`
  and was refused for a conflict sixty seconds later because #472 landed in
  between; the fix was one additive index hunk. Merge the base in before
  reading the state, not after.

## 6. Model tally, for the running record

| lane | model | checked | verdict |
|---|---|---|---|
| T | Fable 5.1 | `ac02ac03`'s diff read by the coordinator: the gate moved exactly as claimed | **right, and found the error nobody else had** — the record a day stale on the reader; cost 2–3x the Opus lanes, half of it after delivery |
| P | Opus 5 | its `glow`/`beam` correction read against `rebuild_blocked`'s doc; its stocking-cap trap reproduced the coordinator's own run | right, and corrected the coordinator twice |
| N | Opus 5 | PR merged on CI; numbers not independently re-run | plausible and internally controlled (twelve paired seeds, a guard red both ways); the card is the check |

Running tally with the lab's: Fable lanes 2 for 2 on finding an error in
what they were handed; Opus lanes 2 for 2 here. Nothing in this round
separates the models on *correctness*; what separated them was **cost after
delivery**, which is a stop-condition problem and not a model one.

## 7. For the next coordinator

Read this file, then §4. Nothing here starts until the owner has ruled on
item 1. When he has: land #478 (or its remainder), run `docscheck`, and
archive the three lane sessions. The next brief writes itself from §4 and
from the two standing facts in §2 — the colony cannot lay a trail worth
following, and the nest has no purpose yet — which are the two things every
candidate in this round ran into.
