# Check: `plants:124` — the prerequisite shipped, and the code says nobody reads it

**Verdict: EXPIRED, verified in both existence and semantics. The strongest
candidate in the run-order.** 2026-09-12.

## The entry

`dead-ends.md`, on `plant.rs` `allocate_to_frontier` / `break_buds`:

> Dividing the plant's stored carbon (stock) among tips was tried: **stock grows
> with mass, so every tip's share stays high for ever**; the stand fused into one
> solid canopy, **38,605 cells against 1,723**. The quantity divided must be
> income (bounded by intercepted light).
>
> *Re-test when:* Stock may only enter the equation once a **monotone high-water
> memory (`q_peak` girth memory)** can distinguish a plant that lost foliage
> from one that never had any; until then mobilising reserves refuses
> fusion-free tuning.

## Verified

| claim | where | result |
|---|---|---|
| `q_peak` exists | `organism.rs:7708` | `pub q_peak: f32` — **shipped** |
| a companion distinguishing *now* from *ever* exists | `organism.rs:7730` | `pub q_now: f32`, *"the same basipetal sum as `q_peak`, **before the high-water `max`**"* — **shipped, and not asked for** |
| `break_buds` reads either | `plant.rs:9018` | **zero** occurrences of `q_peak` or `q_now` in the function body |
| the prerequisite is documented as missing | `plant.rs:7335` | *"monotone girth memory, which is the prerequisite and **is not built yet**"* — **stale**, 4,400 lines from the field that answers it |

**The semantics check out, not just the names** — which is the check that caught
me out on `creatures:039`. `q_now`'s own doc states the property the entry
needs: *"is this cell still carrying any living foliage? The peak says what it
once carried and, being monotone on purpose, keeps saying so for ever."*

**And the code names this exact gap.** `organism.rs:7727`:

> *"`plant::break_buds`' known defect (`q_peak` remembers, **nothing reads the
> difference**) wants exactly this pair as well."*

So: the prerequisite shipped, a *better* form of it shipped alongside
unrequested, the consumer was never wired, and the file that would wire it still
says the prerequisite does not exist.

## The check

One arm to write first. **`break_buds` has no effect counter**, so "did it fire"
is currently unanswerable — add `world.buds_flushed` beside the existing
`shed_shade`/`shed_drought`/`shed_stranded` counters in `world.rs`.

Mobilise on the **deficit `q_peak − q_now`**, never on `stock`. That is the
whole point: the deficit is *bounded*, and it is identically zero for a plant
that never had foliage — which is the distinction the entry demanded and the
reason the original fused. Dividing stock fused the stand because stock grows
without bound; the deficit cannot.

```
cargo build --release --examples        # set -o pipefail, read ${PIPESTATUS[0]}
cargo run --release --example plant_severance -- species=tree seeds=6 trees=4 \
  frames=40000 cut=12000 fine=200 finefor=4000 arms=control,sever_noload
```

*Alive:* `d_cells` climbs back toward control after the cut, `unreached`
non-zero at the cut, `buds_flushed > 0` in the post-cut window.
*Still dead:* `d_cells` flat with `buds_flushed 0` (reserves never mobilise),
**or** cells overshoot control and `above_ground_width` widens — the 1,723 →
38,605 fusion is back and the deficit key is no better than stock.
*Positive control:* the undamaged `control` arm must read `q_peak − q_now ≈ 0`
and flush nothing extra. `unreached` is the harness's own control on the cut — a
sever leaving it at zero severed nothing.

## What it buys

The sharpest ethos payoff in the set, and it is the ethos' own first law.
Cutting a tree today removes cells and nothing else: measured, **1,344 living
cells removed, and over the next 7,400 frames neither tree rebuilt a crown** —
they sat flat-topped with a faint greening at the cut face. Thriving or gone,
with nothing between.

This gives the cut a **middle**: a tree that had a crown comes back, slowly and
differently shaped, and one that never had a crown does not — which is exactly
the distinction `q_peak − q_now` encodes.
