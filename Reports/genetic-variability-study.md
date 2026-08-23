# The genetic-variability megastudy: what it measured, and what it did not

Commissioned as 3 species x 8 world seeds x 16 plants x 45,000 frames — 128
individuals per species — to give trait→outcome regressions per species,
cross-species separation, establishment patterns, and the conifer lean's
seed-dependence. It ran ~3.5 hours detached and completed.

**Read §1 before using any number in it.**

## 1. It is three runs, repeated eight times

The 24 logs contain **three distinct contents**:

```
$ md5sum *.log | awk '{print $1}' | sort | uniq -c
      8 3ecb90f3d629331e699b00e782dbedc9      (conifer, all 8 seeds)
      8 6a166ddde659fc3f02114b3563a85989      (shrub,   all 8 seeds)
      8 898af2515ef7379471d5f88fe2cec821      (tree,    all 8 seeds)
```

`examples/plant_probe.rs` was last written at 02:54 and
`target/release/examples/plant_probe.exe` was built at 02:40, so the binary
the study ran **never had the `worldseed=` argument**. An unrecognised
argument is silently ignored, so all 24 runs used the default world seed.

Verified both directions rather than inferred: the stale binary produces
identical genotype draws for `worldseed=11` and `worldseed=999`; a fresh
build of the same source produces different ones.

**So n = 16 per species at one world seed, not 128 across eight.** This is
`CLAUDE.md`'s "editing a `.ron` does nothing until the next build — identical
output across settings is the tell" one level up, and the study is the exact
situation in which nobody is watching: launched detached, overnight, against
a prebuilt harness. `plant_probe` now echoes its own
`species/trees/frames/worldseed/width` as its first line, so a log that does
not name its seed was written by a binary that never had one.

### What survives and what does not

| question the study was for | status |
|---|---|
| trait → outcome within a species | **survives**, at n=16 — underpowered, see §2 |
| cross-species separation | **survives** — §3 |
| establishment failure | **survives** as a rate at one world — §4 |
| spread of outcomes *across worlds* | lost — no replication happened |
| establishment as a population statistic | lost — one world |
| does the conifer lean's side vary by seed | **lost, and it was the point** — the conifer runs were included specifically to answer this, and all eight are the same run |

## 2. Trait → outcome, n = 16 per species

Pearson r of each genotype draw against outcome. **Critical |r| for p=0.05 at
n=16 is ~0.50**, so only the bolded rows clear significance; the rest are
reported for sign agreement with the earlier 1,024-genome studies, not as
findings.

| trait | tree r(cells) / r(height) | shrub | conifer |
|---|---|---|---|
| branch | +0.06 / −0.07 | +0.18 / −0.22 | −0.22 / −0.38 |
| plastochron | +0.34 / +0.13 | +0.05 / +0.10 | −0.17 / −0.01 |
| **turgor** | **−0.55 / −0.76** | −0.44 / −0.15 | −0.44 / **−0.72** |
| **pipe** | **−0.57** / −0.27 | −0.26 / +0.06 | −0.46 / −0.29 |

**Turgor is the one solid result and it replicates.** r(height) = −0.76 for
tree and −0.72 for conifer, against −0.74/−0.75 measured in the two earlier
1,024-genome studies on `tree` alone. A higher `turgor_per_cell` draw is a
lower derived height ceiling, so the sign is the model's own arithmetic
coming back out of the data — which is the right kind of corroboration for a
study this size.

**Pipe is correctly signed everywhere** (a higher ratio demands more foliage
per cell of width, so stems stay thinner and the plant totals fewer cells)
and clears significance only for `tree`.

**Plastochron does not reproduce here**, and that is a power problem rather
than a contradiction: the 1,024-genome study made it the *strongest* trait
(3.9x quintile spread in size). At n=16 its r=+0.34 in `tree` has the same
sign and cannot be distinguished from zero.

`branch` is inconsistent in sign across species and should not be read at
all at this n.

**Do not re-run this at n=16.** The re-run wants the seed connected (which
now works) *and* the shape metrics of §5, and at 8 seeds it is the 128
individuals originally intended.

## 3. Cross-species separation — only the shrub separates

| species | height range | cells range |
|---|---|---|
| shrub | 48 – 83 | 1,570 – 4,491 |
| tree | 113 – 177 | 2,549 – 6,909 |
| conifer | 154 – 204 | 3,194 – 8,133 |

- **Shrub is cleanly disjoint from both** on height (83 vs 113). It is the
  only unambiguous separation in the study, and it comes from
  `turgor_source: 0.4` — a height budget, not an architecture.
- **Tree and conifer overlap on height across a 23-row band** (154–177).
- **Mass separates nothing**: all three ranges overlap heavily.

This is the quantitative core of the owner's playtest reading. The species
were expected to be "disjoint or nearly" on these distributions and two of
three are not — and the one that is, is separated by a scalar height budget
rather than by any of the architectural levers built for it.

Composition tells the same story more sharply (per-species means):

| | tree | shrub | conifer |
|---|---|---|---|
| leaves as % of cells | 4.8% | 6.2% | 3.2% |
| rows >1 cell wide | 66% | 61% | 64% |

Three species, one composition. See `plant-appearance-design.md`.

## 4. Establishment: a quarter to a third of plants carry no foliage at all

Individuals finishing the run with **zero `Leaf` cells**:

| species | leafless |
|---|---|
| tree | **5 / 16** |
| conifer | **4 / 16** |
| shrub | 0 / 16 |

These are not dead — they are standing wood with a live structure and no
canopy. At 45,000 frames that is a terminal state, not a transient.

The shrub's 0/16 against the other two is the informative contrast: it is the
species with the smallest height budget and the highest `branch_chance`, and
`tree.ron`'s own note predicts exactly this — *"a seedling branches to build
leaf area"*, so the species that branches earliest and stops climbing
soonest is the one that establishes. It also names the real fix, which the
data now supports: `branch_chance` should be high while juvenile and low once
established, which `ByOrder` cannot express because order is position in the
plant and not age.

## 5. The study could not have answered the question that prompted it

Every quantity the harness recorded was a **magnitude** — cells, leaves,
height, thickness. The complaint it was meant to inform was that three
species look like one plant at different sizes, and a study whose entire
instrument is size cannot address that: three species differing *only* in
scale score as three clearly different species on every column above.

Three descriptors have been added to `plant_probe` for the re-run, the first
two scale-free by construction:

- **crown profile** — foliage width in five height bands, top first, as a
  percentage of that plant's widest band. Measured on `tree`:
  `[100, 77, 36, 0, 0]` — all foliage in the upper 60%, none in the lower
  40%, the bare-bole broadleaf signature. A fir should read descending and a
  shrub flat; that this one already discriminates is the point.
- **foliage centre** — mean leaf height over the plant's own span, 0 at the
  collar, 100 at the apex. `tree` measures **84**.
- **foliage share** — leaves as a percentage of cells. `tree` measures
  **7%** at the standard 8-tree probe.

## 6. What the re-run should be

1. Rebuild first, and check the echoed parameter line in the first log
   before letting it run for hours.
2. 3 species x 8 seeds x 16 plants, as originally specified — now actually
   24 populations, 128 individuals per species.
3. Report the §5 descriptors beside the magnitudes, and gate the
   cross-species claim on **profile and foliage centre**, not on height.
4. Keep conifer in for the lean question, which is still open and still
   unanswered.

Not done here, and worth stating plainly: the appearance work in
`plant-appearance-design.md` changes foliage volume, so **the re-run should
happen after it lands**, not against these numbers.
