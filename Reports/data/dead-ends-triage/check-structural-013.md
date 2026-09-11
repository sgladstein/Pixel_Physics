# Check: `structural:013` — the two materials still on the rejected model

**Verdict: EXPIRED, condition met, and the work is smaller than it looked —
exactly two materials, both `.ron`-only.** Verified 2026-09-11.

## The entry

`README.md` 'M17 status', on directional support cost:

> Charging a **flat cost of 1 per relaxation step regardless of support
> direction**: a 1-cell tower snapped at exactly the reach a 1-cell cantilever
> managed, contradicting rock being strong in compression and weak in
> bending/tension. Split into `support_cost_below`/`_beside`/`_above`
> (stone 0/1/3).
>
> *Re-test when:* All three **default to 1** (the behaviour they replaced), so an
> un-authored material **silently regresses to the rejected model** — new
> materials need real directional costs.

## The condition, checked

`src/sim/material.rs`: `fn default_support_cost() -> u16 { 1 }`, applied by
`#[serde(default)]` to all three fields. So the clause is exactly right: an
unauthored material is on the rejected flat 1/1/1.

**11 of 55 material files author the triple** — `stone`, `basalt`, `crystal`,
`flowstone`, `growlamp`, `ironstone`, `limestone`, `mudstone`, `sandstone`,
`spar`, and `ice` partially. The other 44 do not.

## But 44 is the wrong number, and this is the useful part

The entry is about **bending** — a tower against a cantilever — which is a
`Solid` question. Filtering the 44 unauthored materials to `kind: Solid` leaves
**exactly two**:

| material | kind | authored |
|---|---|---|
| `log` | `Solid` | **none — flat 1/1/1** |
| `nest` | `Solid` | **none — flat 1/1/1** |

Everything else unauthored (`deadwood`, `packedsoil`, `rubble`, `windfall`,
`soil`, `sand`, …) is a `Powder` and does not do the bending this rule governs.

**And the code independently names the same pair.** `src/sim/load.rs:2135`:
*"only two materials in the shipped set that reach it are `log` and …"* — the
bearing clamp. Two unrelated lines of evidence, the asset census and the load
model's own comment, converge on `log` and `nest`.

*(A first grep for these missed, because it searched for `"log"` as a string
literal and the source names it in backticks inside a comment. The register's
`docgrep` gotcha in a new costume — that was my error, not a bad citation.)*

## What to do, and what it buys

Author `support_cost_below`/`_beside`/`_above` for `log` and `nest`, shaped from
stone's 0/1/3. **`.ron` only, no code.**

Then, with both arms:

```
bash scripts/acceptance.sh
cargo run --release --example filmstrip -- scene=fell fell=7150 frames=8750
```

*Alive:* the standing-log fraction and `crumbled to grit` move. `load.rs:2128`
records the baseline — **1,191 cells of log delivered by `rigid::settle`, 431
still standing, 592 cells of deadwood** — and a real tension cost should change
*where* a landed log breaks, not only how much.
*Still dead:* acceptance green and those counts unchanged to the cell, meaning
the inert path never evaluates these two in bending and the gap is theoretical.
*Positive control, first:* set `log`'s `above` to an absurd 60 and confirm
landed logs shatter. If they do not, the material never reaches the directional
branch and the question is moot — the cheap guard against measuring a path that
does not execute, which is the trap `GROUND_ROOT` was caught in.

**What it buys**, in the world's terms: felling is a shipped verb whose stated
product is *pieces*. Today a felled log bridging a hollow is exactly as strong
hanging as standing, so it breaks by size rather than by how it landed. And a
burrow roof is in tension while its floor is in compression — at cost 1 they are
identical, which is why a nest collapses uniformly instead of dropping its
ceiling first. One verb, a wider distribution of outcomes.
