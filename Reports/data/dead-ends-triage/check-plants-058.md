# Check: `plants:058` — both gates are met, and one of them is met by another entry

**Verdict: EXPIRED. The stated condition holds, and the *second* gate the code
adds on top of it has also since been cleared.** 2026-09-12.

## The entry

> Packing canopy density into **4 bits of `Cell::aux`** forced tuning a
> behaviour constant around **storage**: decay rates smaller than the
> quantization half-step (≈0.133) got quantization-locked, so
> `CANOPY_DENSITY_DECAY_PER_TICK` had to be raised to **0.5** for the mechanism
> to work at all — *the tail wagging the dog*.
>
> *Re-test when:* The 4-bit packing was removed by the sidecar migration, so the
> 0.5 setting's rationale is gone — the decay constant is re-derivable once
> storage is a plain float.

## The stated condition: met

`organism.rs:7623` — `pub canopy_density: f32`. A plain float, on the sidecar
struct, no packing. `plant.rs:521` still reads
`const CANOPY_DENSITY_DECAY_PER_TICK: f32 = 0.5` — the value the quantization
forced, with the quantization gone.

## The second gate, which the entry does not mention

The constant's own doc adds a *different* reason for leaving it alone, and a
reader who stopped at the entry would miss it:

> *"So this constant is left alone, and so is `pipe_ratio`. **The real fix is the
> `thicken()` change `PLAN.md` already lists as known-open.** … the constants are
> deliberately not re-tuned here — `plant-substrate-v2-design.md` §10 forbids
> `.ron` edits at this step precisely so the economy pass tunes once, against the
> final transport mechanism, rather than twice."*

So there were two gates: the packing (the entry's) and the `thicken()` fix
(the code's). **Both are now clear**, and the evidence comes from the entry next
door:

- **The `thicken()` fix landed** — `cross_section_axis` in `plant.rs`, verified
  while checking `plants:019`.
- **The deferral has already lapsed in practice.** The same doc says *"and so is
  `pipe_ratio`"* — but `pipe_ratio` **has** since been re-tuned, to 5.5 on
  conifer/creeper/shrub/tree, 14.0 on herb, 16.0 on scrambler, against the 6.0
  that was withdrawn. Half of the "leave both alone" policy was acted on and the
  other half was not.

That is the useful part: the two entries interlock, and checking one answered
the other's hidden gate. Neither could be settled from its own text.

## The check

An env override is the arm, and `plant.rs` already uses that pattern 46 times.

```
cargo build --release --examples
for d in 0.5 0.25 0.12 0.06; do
  PIXEL_PHYSICS_CANOPY_DECAY=$d cargo run --release --example plant_probe -- \
    species=tree trees=12 frames=30000 worldseed=1
done
```

*Alive:* crown occupancy and crowding's effective reach move monotonically with
the setting — self-avoidance is a live lever again and 0.5 is not its best value.
*Still dead:* the four runs differ by less than the **31-to-153-cell** spread
this repo records for twelve identical trees from one genome — the rate stopped
mattering once the floor was removed, and 0.5 can be documented as arbitrary
rather than re-derived.
*Fired-confirmation:* `plant_probe` echoes its own parameters, so the header must
name the rate — a log that does not name it was written by a binary that never
had it, which is the trap that produced eight byte-identical logs per species in
the 3.5-hour megastudy. And the `0.5` arm must reproduce the current binary
bit-for-bit; if it does not, the override is not reaching the constant.

## Standing

Lower value than `plants:124` or `structural:013` — this re-derives a constant
rather than adding a middle to a binary or giving a verb a consequence, and
**exactness is explicitly not a goal here**. Worth doing as the cheap half of the
crowding question (`plants:044`), which touches the same signal, rather than on
its own.
