# Lateral phototropism: the prescribed fix works, and it stops trees breeding (2026-08-27)

**Status: finding, change NOT landed.** The repair `Reports/dead-ends.md`
prescribes was built, proved correct by a non-blind guard, and **withdrawn
before landing** because it suppressed reproduction outright. The working
patch is preserved verbatim in
`Reports/data/phototropism-2026-08-27/PROPOSED-*.rs.txt`.

## 1. What was wrong

`organism::phototropism_dir` compared light at `(x, y)` against `(x, y-4)`
and returned `(0,-1)` or `(0,0)`. **Its entire codomain contained no vector
with a horizontal component**, so it was collinear with `upward_weight` and
no scene — a slope, a clearing, a neighbour's shadow — could make it respond.
`dead-ends.md` records the consequence (`light_weight` measured **inert
across 1,024 genomes**, *by construction*) and its re-test line prescribes
the repair:

> *"the probe is reshaped to a real gradient (the `moisture_pull` form:
> sample ±offset in x and y, return normalised gradient)"*

## 2. What was built

Exactly that, copying `moisture_pull` (`organism.rs:3455`) rather than
re-deriving it: sample `±LIGHT_SENSOR_OFFSET` (4.0, matching
`MOISTURE_SENSOR_OFFSET`; `FIELD_SCALE` is 8 so ±4 spans a block) in x and y
through `field_at_bilinear`, return the normalised gradient. It stays a
*difference* of two readings, so the day/night phase still cancels — the
property `ambient_light_above`'s doc says is load-bearing.

**The guard is not blind, and this was checked rather than assumed.** The new
test asserts a real rightward component when light sits to the side. Run
against the **old** implementation with the **new** test in place, it fails
with `got (0, -1)` — the old codomain. Against the new one it passes,
returning `(0.5547, -0.8321)`: a genuine diagonal.

> A first attempt at that control was **malformed and looked green**:
> `git stash` reverted the test along with the implementation, so what passed
> was the *old* test. `CLAUDE.md`'s "a green suite does not prove a test ran",
> hit live. The valid control reverts only `organism.rs`.

## 3. Why it was withdrawn

`cargo test --lib`: **945 passed, 1 failed** —
`expressing_the_appended_genome_slot_changes_no_plant`, and it failed on its
**anti-vacuity guard**, not its identity assertion:

```
the stand must breed within the budget or this guard never exercises
`set_seed`; got 4 owners
```

`FOUNDERS` is 4, so the stand produced **no offspring at all**. That guard
passes on `main`, so the change took reproduction from working to zero in
that scene.

**The cause is not the gradient; it is that `light_weight` was calibrated
against a lever that could only ever say "up".** Authored values are
`tree [0.15, 0.3, 0.5, 0.6]`, `shrub [0.3, 0.45]`, `conifer [0.1..0.2]`,
`creeper [0.05, 0.05]`, `grass [0.1, 0.1]`. Against the old codomain those
weights only ever reinforced `upward_weight`. Give them a real 2D direction
and up to 0.6 of a tip's scoring budget starts steering sideways — the plants
spread instead of climbing, never reach `seed_maturity` (600 shoot cells for
tree), and never breed.

This is `CLAUDE.md`'s *"fixing a bug often exposes a constant that was
compensating for it"*, in its sharper form: the constant was not compensating,
it was **free**, and the fix gives it teeth. **Re-deriving those weights is
part of the fix, not scope creep** — and that is a tuning pass over 5 species
x up to 4 orders, gated on a seed sweep, which is more than the session that
found it had left.

## 4. What is banked

- A **baseline sweep** is committed: `tree`, 6 world seeds, 8 plants, 30,000
  frames, on `main` as of this date, in
  `Reports/data/phototropism-2026-08-27/seed*.log`. Six distinct md5s. The
  after-arm can be compared against it directly rather than re-measured.
  Baseline shape: crown profile median ~`[73-76, 98-99, 88-93, 80-82, 50-55]`,
  foliage centre 56-57, foliage share 45%.
- The **working patch**, both files, verbatim.
- The **non-blind guard**, in the patch, with its control recipe written down.

## 5. What the next session should do

1. Re-apply the patch from `PROPOSED-*.rs.txt`.
2. **Re-derive `light_weight` per species**, starting far below the authored
   values — the old numbers are calibrated against a dead lever and should be
   treated as meaningless rather than as a starting point. `0.0` everywhere
   reproduces today's *lateral* behaviour but not today's plants, since the
   old lever did duplicate `upward_weight` when lit from above; that
   duplication is what the new weights have to replace.
3. Gate on `expressing_the_appended_genome_slot_changes_no_plant` breeding
   again, then on the committed baseline sweep, then on a blind card — a
   plant that leans toward a gap is exactly a judge-by-eye change.

**Do not land it without step 2.** A correct mechanism at inherited weights
is a stand that does not reproduce.
