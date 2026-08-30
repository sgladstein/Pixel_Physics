# Creature line — the S6 gate, run

**Successor to `creature-line-handoff-2026-08-29-evening.md`** (which is on
`claude/creature-motion-slowdown-nnhqy0` at `01e77b6`, unmerged). That file
opens with a STOP: a positive control it could not run because the perf lane
owned the box. This note is that control, run.

## 2026-08-29 — the gate passes, and the birth path is not the problem

**Ants can breed. The reproduction build is unblocked**, and what stops the
shipped ant is its economy, exactly as `creature-reproduction-economics.md`
argues — not a birth path that never fires.

The handoff's own line, unmodified:

```
creature_probe start_energy=200 body_energy=20 threshold=241 hunger=0.9 terrain=world frames=24000
```

```
reproduction: births 1875 denied-no-space 62098 refused-no-slot 0
              live 300 deepest generation 18 | lineages 6 top share 0.517
              richest bank 616 against a birth cost of 240
```

**births 1875**, not a marginal one or two. Bit-identical across two runs.
The colony reaches **generation 18** and collapses from its founders to **6
lineages with one holding 52%** — so there is differential lineage success to
select on, which is more than the gate asked. The limiter in this config is
**space, not energy**: 62,098 births denied for want of room against 1,875
granted.

**So the economics report's §§1–5 stand.** The handoff staked them on this
run — *"if it reports `births 0`, the economics report's §§1–5 are wrong"* —
and it reports the opposite.

### The gate is more fragile than its line admits — pin the seed

Re-run at `seed=7` and it reads `births 0 ... live 0 ... richest bank 0`.
That is **an empty world, not a failed birth**: `totals` shows `eats 0
pickups 0 digs 0 drops 0` and `deaths 5`. Nothing was alive to breed.

The gate line does not pin a seed, so it takes the default `0xa17`. Had it
been run at a seed like this one, it would have read `births 0` and voided a
correct report for a reason with no economics in it. **A `births 0` must be
read against `live` before it is read as a verdict.** (`seed=` parses decimal
only — `seed=0xbee` panics.)

## The printed ceiling is not a ceiling — measured, twice

Secondary, and it outlives the gate. `creature_probe` prints a bound and, on
the shipped ant, called it *"UNREACHABLE, and this is a proof"*. **The bound
is exceeded in both control runs:**

| run | printed ceiling | measured `richest bank` | excess |
|---|---|---|---|
| control as specified | 540 | **616** | +76 (14%) |
| same, `mutation_rate=0` | 540 | **561** | +21 (3.9%) |

Two channels widen the real bank past that arithmetic, and neither is in the
sum:

- **It prices the founder's gut, not the eater's.** The probe reads
  `def.traits[TRAIT_GUT_BIAS]` — a species constant — while `creature.rs:1583`
  digests with the organism's *own* `s.traits[TRAIT_GUT_BIAS]`, which is
  heritable and mutates by `trait_variance`. A matched gut pays `worth` where
  the neutral founder pays `worth/4`. Turning mutation off removes **55 of the
  76** excess, which is what names this channel rather than assuming it.
- **A corpse is worth more than its stamp.** `creature.rs:3028` writes
  `(body_energy * cells + leftover) / cells` — the dead animal's unspent bank
  rides into the meat — where the probe stamps its cell with `body_energy`
  alone.

**The residual is not fully explained.** With mutation off the bound is still
exceeded by 21, and I did not chase it to a mechanism. Recorded as measured
and open rather than tidied into the two channels above.

**What this does and does not cost.** It does not move the shipped ant's
verdict: bank 567 against a 1,860 bar is a 3.3x margin, and nothing dies in
that scene, so the leftover channel is dry. What it costs is the word *proof*.
`examples/creature_probe.rs` now says "strong evidence of unreachability, but
the bound is not a ceiling"; the arithmetic is deliberately unchanged, so every
ceiling figure already on record stays comparable.

This is the failure the probe's own comments boast of having fixed once
already — an earlier version priced the mouthful with `body_energy` and
overstated the ceiling 6x. Same shape, one layer down: **a readout computed
from a species constant, in a run whose whole purpose is that the species
evolves away from it.**

## Not done, and why

- **The render fix** (handoff §6.1) already has **PR #132** open, from
  21:30 — after the handoff was written. Not mine to duplicate. Note **PR
  #133** looks like a second fix of the same glow-splat cost; someone should
  check whether they collide.
- **The reproduction build** (§6.3) is now unblocked by this gate. Left for
  whoever picks it up.
- **The body-size pricing question** (§6.4) is an owner decision and stays one.
- **`open-bugs-handoff.md` §S** (§6.5) still needs an owner.
