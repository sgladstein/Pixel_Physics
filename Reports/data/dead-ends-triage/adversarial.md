# The false-negative rate: 3 of 40

**The only number that says whether this triage is trustworthy.** Measured
2026-09-11.

## Method

40 entries sampled at random (seed 4242) from the 495 labelled `DEAD`, given in
full to a fresh Opus agent **with the labels stripped**, asked to make the
strongest honest case for reopening each and then say whether that case stands.
It was told to expect most to hold, and that a run reopening half of them would
not be rigorous.

It reopened **3 of 40 — 7.5%**.

Extrapolated over 495 `DEAD` entries that is roughly **37 further candidates**,
taking the true total from the screen's 118 to about **155 of 779, near 20%**.
The screen is therefore somewhat tight rather than loose, which is the safer
direction for a register whose job is preventing wasted sessions — but it is not
negligible, and this file is the reason a reader knows the 15% was a floor.

## The three, each verified in source

**`creatures:013` — the entry prescribes code that no longer exists.** It
records replacing "eat if energy below full" with "a `hunger_fraction` (0.5)
branch between eat and carry", and calls that unconditional. **`hunger_fraction`
has zero live references in `src/`** — every one of the five hits is a comment
or test prose. Commit `e8314d16` deleted it, and `src/sim/creature.rs:5531` now
reads *"Nothing here decides between eating and carrying, because there is no
such decision any more… An animal now takes what it finds and digests it as it
walks."* The shipped model is closer to the option the entry rejected than to
the one it prescribes. The agent's reading of *why* is the useful part: without
a crop that digests over time, eating **is** destruction of the load, so the
failure belonged to instant conversion rather than to the gate value.

**`plants:142` — measured entirely inside a condition its own source comment
says has been removed, and half of it has since been readopted.**
`src/sim/plant.rs:11931` is more specific than the register: both probes
*"**under**-read inside a blob, because a blob is porous… The per-stem run is
the quantity the pipe model actually names, and it is viable now in a way it was
not before."* `fn stem_run(.., axis)` ships at `plant.rs:11843` and is the
perpendicular axis walk. So one of the two rejected probes is the current
mechanism; the other was beaten 31,591 to 12,039 under blob conditions that no
longer obtain.

**`rendering:014` — the reopen condition it names has become cheap.** The
rejection is a risk estimate ("no reference font to check against") and its own
condition is *"re-open only with a verification method per glyph"* — which is
now one contact sheet and a `review.py` card, this project's standard workflow
and not available when the entry was written. The consumer set also grew: a
decision taken for a handful of debug lines now caps the whole lab UI's
typography.

## What the pass also confirms

37 of 40 hold, and the agent flagged several as unusually well-instrumented —
`structural:005` carries a size trend across four horizons and three world
sizes; `structural:009` inverted its own guard and its settling census together
and **disclaims its own 5.7 ms gap as machine noise rather than quoting it**;
`other:056` was measured against a working positive control. It spot-checked
four numeric claims in source (`surface_hop` 0.75 / `PLAYER_HEIGHT` 14,
`particle::MAX_SPEED_PER_AXIS` 8.0, `ORGANISM_TICK_INTERVAL` 45, and `pixels`
0.17.2's private surface and unconditional full upload) and **all four verified
as stated**.

That is the finding underneath the headline: this register is mostly accurate,
and where it is wrong it is usually wrong because the world moved, not because
the reasoning was poor.
