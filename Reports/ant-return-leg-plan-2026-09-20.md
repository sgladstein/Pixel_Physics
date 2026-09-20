# The returning ant cannot read its own trail — and never could

*2026-09-20. Supersedes the 2026-09-19 plan and this session's two earlier
drafts. The measurement below was the fork all three were arguing past.*

## Context — the test that settles it

Three candidate fixes were on the table and all three rested on the same
untested assumption: that when a laden ant points at home, its trail sensor
says so. **It does not.**

New census in `examples/trailfollow.rs` (`tr_align`), binning every laden
decision by the angle between the ant's heading and its exact home vector.
8 seeds, ~500,000 decisions, both arms:

| heading vs home | mean `along` | % positive |
|---|---|---|
| pointed **away** (−1.0..−0.6) | −0.24 | 3.9% |
| pointed **at home** (+0.6..+1.0) | **−0.20** | **5.3%** |

**The reading is negative in every alignment bin, and facing home differs from
facing away by 0.04.** The trail carries essentially no information about which
way home is.

### Why, and it is structural rather than a bug to tune

`along = (ahead − here) / (ahead + here + SCALE)`. For the ant that laid the
trail:

- **`here` is its own freshest deposit.** Channel A is written at the head cell
  on every successful move, so the cell underfoot is the brightest thing in the
  neighbourhood (`open-bugs-handoff.md` §Z29, filed this session).
- **`ahead` is six cells out**, and measured, **70% of laden ticks have that
  sample in open sky or solid rock**, which reads 0 after the §7.47 honesty
  gate. Of the 30% that land on a surface, mean `along` is still −0.14.

So the numerator is `(≈0 − own deposit)` — **negative by construction**. The ant
is a moving point source on a plane where it is the brightest object. It cannot
see a gradient because it *is* the gradient.

**§Z29's own remedy is not enough.** `PIXEL_PHYSICS_DEPOSIT_AT=vacated` moves
pointed-at-home from −0.200 to −0.162 and 5.3% → 7.0% positive. Directionally
right, and still **93% wrong-signed**. Keep it as a component; do not expect it
to carry the fix.

### What this reconciles

The literature already separated these roles and we conflated them.
**Beckers 1992: discoverers lay, recruits follow.** Trail-reading is the
*follower's* mechanism. Path integration is the *layer's*. A laden ant walking
home **is the discoverer** — and we have been asking it to navigate by reading
the trail it is in the act of creating.

That is why every repair to the *reading* has failed to move the outcome:
`TRAIL_A_RHO = 0` (§7.46), the nose honesty gate (§7.47), and the temporal
comparator into `Move` (§7.48, a measured coin flip). They improved a channel
that is structurally uninformative for this animal.

## Work

### Step 1 — Give the home vector authority over `Move`

> **BUILT 2026-09-20, AND THE WIRING BELOW IS WRONG — read this box before the
> section.** The goal is right and shipped; the *circuit* specified here does
> not work and the reason is arithmetic rather than tuning. `squash` is
> `x / (1 + |x|)`, so a shut `(Bias, 7, -45)` unit reads **-0.978, not 0** —
> the gated pair is neutral when shut only because it has two units at -0.978
> and subtracts them. **The mirror is the neutraliser, not decoration.**
> Measured through `eval_brain` (`brain.rs`'s ignored
> `what_the_home_wire_emits`,
> [`Reports/data/home-wire-response-curve-2026-09-20.log`](data/home-wire-response-curve-2026-09-20.log)),
> the single-unit form puts **-2.439 on every EMPTY ant's `Move`** against a
> walking ant's +0.25 — `P(move)` clamped to 0, a colony that never forages —
> and **+0.833 on a laden ant across the bearing**, which is a *laden ants move
> more* lever that would have lifted every alignment bin together and made the
> result unattributable.
>
> **What shipped instead:** the gate moved upstream of `squash` into the
> sensor — `creature::sense` reads `BrainInput::HomeAligned` as 0.0 for an
> empty ant — and the wire is a direct `(HomeAligned, Move, 3.0)` instinct with
> **no hidden unit at all**: 0.000 empty, 0.000 across, ±2.143 on the bearing.
> What that costs is the gate as a *gene*: selection can move the gain but not
> the threshold. What it buys, beyond working, is that **unit 7 stays free**,
> so trap 1 below — the collision with the 2026-09-19 fold-change plan — does
> not happen. Full entry in `Reports/dead-ends.md` (`other:133`).
>
> Everything else in this section stands: the quantity, the re-pin list, the
> speculation contract and the pre-registered predictions are unchanged.


`home_weighted_pick` aims the body from the exact home vector; `P(move)` is set
by `PheroAAlong` and nothing else knows where home is. So an ant pointed at its
own nest computes `P(move) ≈ 0.01`. **That is the fix, and after the test above
it is no longer a workaround — it is the only signal this animal has that
actually knows the answer.**

The quantity exists already: `home_weighted_pick` computes the dot product at
`creature.rs:12283`. Expose it as `BrainInput::HomeAligned` — cos of the angle
between heading and home vector, `−1..+1`, zero with no anchor or standing on
it — and wire it in `ant.ron` through **a gated pair exactly like `PheroAAlong`
today**: two hidden units, opposite signs, both gated on `CarryingFood`, into
`Move` at ±w. Proven circuit shape, nothing invented.

**There is only ONE free hidden unit, and a gated pair needs two.** Verified
2026-09-20: `ant.ron` wires units **0–6** (0/1 the channel-A pair, 2/3 the
channel-B pair, 4 the `AtNest` odometer, 5/6 the `Crowding`→`Dig` pair) and
`BRAIN_HIDDEN = 8`. **Unit 7 is the only one free, and the 2026-09-19
fold-change plan wants it too.** So the pair form is not available and the
wiring is the single-unit version of the same shape:

```
(Bias,         7, -45.0)     gate shut unless carrying
(CarryingFood, 7, +45.5)     opens at fill >= 0.989, exactly h0's threshold
(HomeAligned,  7,   +w)      sweep w; h0 uses 6.0 for PheroAAlong
(7, Move, +w2)               sweep w2; h0/h1 use +-2.5
```

That is `h0`'s form exactly, minus its mirror. **What it loses is the
symmetric swing** — `2.5·(h0 − h1)` is near-linear through zero, whereas one
squashed unit saturates asymmetrically around its bias. Probably sufficient and
**must be checked**: sweep `w`/`w2` and read the response curve through
`brain.rs`'s ignored `what_an_odometer_emits` before trusting a bed result.

**Cost, honestly:** `BRAIN_INPUTS` 32 → 33, `mutation_rate` re-derived in six
species files (`3.18 / live_slots`, and `live_slots` moves), and **two pinned
constants re-pinned** — `the_live_slot_count_is_pinned` (currently 918) and
`the_genome_manifest_is_pinned` (currently 460_972_744). Stored genome baselines
are voided. World-hash gates (`ascii`, `acceptance.sh`, `tests/determinism.rs`)
move by construction: re-baseline, do not widen.

**One contract to check, latent rather than live.** `sense` would gain a read of
`forage_anchor` — *organism* state, which `sense_read_rects` does not declare,
because it declares world-cell rects. `forage_anchor` is rewritten on nest
contact (`creature.rs:10703`), so a speculated sense could in principle go
stale. `ParMode` ships **off** (`par_mode_default`), so this is latent — but run
**`ParMode::Verify` on a colony bed**, which is the only thing that proves
`sense` and `sense_read_rects` agree.

**Pre-registered predictions.** `P(move)` in the pointed-at-home bin rises from
0.078 toward the up-gradient figure of 0.59; the laden leg falls from its
2,900-tick median; and the alignment census stops being flat — the
pointed-at-home row must separate from the pointed-away row. **If `P(move)`
rises and the leg does not shorten**, the ant is moving without moving home and
the step choice (Step 4) is next.

### Step 2 — Keep `DEPOSIT_AT=vacated` as a component, measured on its own

It is a real improvement to the sensor (5.3% → 7.0% positive when pointed home)
and it is nearly free. Sweep it **with and without Step 1** so its contribution
is attributable rather than absorbed.

### Step 3 — Run length, which no outcome number survives without

The median completed laden leg is **2,900–3,200 decision ticks**; an ant's whole
life in a 24,000-frame run is **4,000 ticks**. One leg is 73–79% of an ant's
existence before it has found anything. Re-run at `frames=100000`.

**Do this after Step 1**, because Step 1 changes the leg length and sizing the
harness against the old figure would bake in the wrong budget.

### Step 4 — Fold-change detection, moved to where it belongs

The adaptive-guard work (2026-09-19 plan: unit 7 holds the recent average
concentration and normalises in place of the constant `SCALE`; Lazova et al.
fold-change detection; Segall/Block/Berg temporal comparison) is **correct and
still wanted** — but it is the **follower's** fix, not the layer's. Its payoff
is in **recruitment**, where a second ant reads a trail it did not lay and the
200x concentration range across the route is exactly the problem fold-change
exists to solve.

Scheduled there deliberately. Applied to the returning ant today it would
amplify a reading whose *sign* is wrong 94% of the time, and **no normalisation
fixes a sign** — the guard is in the denominator and is always positive.

### Step 5 — §R4 and the step choice, if Step 1 is not enough

`Turn` is structurally inert on flat ground (filed, reproduced) and the home
vector reaches `step_chain` only as a measurement counter. If Step 1 raises
`P(move)` without shortening the leg, direction has to reach the step choice
too — added to the candidate scoring directly, never through `Turn`, and
**additive on top of footing so it can never overcome its absence** (§R4's own
warning: `falls 16,451 against moves 22,138`).

## Rejected, with reasons

- **A longer `sensor_offset`.** Plausibility gate: no lever ships that gives a
  2-cell animal a 10-body-length reach. A bounded 6 → 8/10 nudge may be tested;
  nothing beyond.
- **The temporal comparator into `Move`.** Built and measured this session
  (§7.48): every sign test a coin flip.
- **More `home_bias`.** Saturated — response linear to the 1.0 cap, and a laden
  ant with a full crop already re-aims homeward on every tumble.

## Acceptance criteria

- **Outcome bar:** return leg and food reaching the nest. **Intake and founding
  are recorded, not gated** — owner's ruling; the granary comes after and
  settles them.
- **Mechanism bar, which the outcome cannot substitute for:** the alignment
  census must separate. Pointed-at-home must read meaningfully better than
  pointed-away; today the gap is 0.04.
- **Plausibility gate**, as above.
- **Judge by eye** — a review card before it is called done, with the
  round-trip count in `meta`.

## Verification

1. `cargo build --release --examples` with `set -o pipefail` **first**.
2. **Positive control every run:** homeward tumble % reads `0.00` on the
   baseline arm and rises monotonically (held 36/0/0 across nine settings).
3. Paired within seed via `scripts/trailledger.py`. **Never `DELIVERED`** — it
   counts any crop drop at the nest and runs 24–45x the laden foraging trips.
4. Full gates: `cargo test` (not `--lib`), `clippy`, `ascii` worst-frame,
   `acceptance.sh`, `worldgencheck.sh`, `docscheck.sh`.
5. `python3 scripts/deadendindex.py --touching`; §Z29's write-back is part of
   this work, and it should record that `vacated` is partial rather than a fix.
