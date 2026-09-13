# Check: `structural:005`'s named replacement, at shipped world size

**Entry** (`dead-ends.md:238`): clamping the support field at a horizon and
calling the saturated value "no known path" — rejected, because `load.rs` reads
the field as a strict *order* and a clamp creates plateaus in which a cell has
no strictly-lower neighbour left. **That rejection stands and is not in
question.**

What is in question is the alternative its own `Re-test when:` clause names,
which has never been built. The clause makes three checkable claims about the
hierarchical `(chunk-component level, distance-to-portal)` potential:

> …is never clamped, **keeps a strict order everywhere** (`held->unheld` 0 on
> every world measured), **agrees with the exact field on reachability exactly**
> (0 either way), and **packs into the existing `u16`** (max level 39, max
> offset 239).

## Run

```
./target/release/examples/support_census size=8192x2560 seeds=1,3,7
```

**53 s total**, three seeds at the shipped 8192×2560. `support_census` is
read-only — it builds candidate fields beside the real one and never writes.
Its own controls passed in a prior run at 2048×640: the flat-zero arm differs
98.02% (sensitive) and the identity arm differs 0.00% (specific).

| | seed 1 | seed 3 | seed 7 | claim |
|---|---|---|---|---|
| `held->unheld` | **0** | **0** | **0** | 0 — **holds** |
| `unheld->held` | **0** | **0** | **0** | — |
| reachability disagreement, either direction | **0** | **0** | **0** | 0 — **holds** |
| max level | 42 | 39 | 39 | 39 |
| **max offset** | 252 | **258** | 252 | **239 — fails** |
| unreachable nodes | 10 | 8 | **75** | not stated |
| distance values differing from exact | 50.99% | 56.29% | 57.64% | not stated |
| `support_count` moved | 48.88% | 53.93% | 55.34% | not stated |

## Reading

**Two of the three claims hold at the shipped world size, and they are the two
that decide whether the replacement is safe.** Nothing the exact field calls
held becomes unheld, and reachability agrees exactly, on all three seeds — which
is the property the clamp was rejected for destroying.

**The packing claim does not hold.** Max offset is **258** against the quoted
239, which needs a **ninth bit**. Level 42 needs six, so the pair still fits a
`u16` (15 bits) — but the margin quoted in the register is not the margin three
seeds give, and it was quoted as a settled property. This is the register's own
*"six seeds is not a sweep"* rule biting a claim inside the register.

Two quantities the clause does not mention and a builder would need:
**unreachable nodes** (10 / 8 / **75** — seed 7 is an order out), and that
roughly **half of all `support_count` values move**. Held/unheld order surviving
is not the same as the load DAG being unchanged, and `load.rs`'s `dependants`
and `support_parent` both read that order.

## Status

Not a revival of `structural:005` — that entry is correctly `DEAD` as stated.
This is a **correction to the claim standing in its re-test clause**, and it is
the cheapest kind of finding this triage can produce: 53 seconds against a
sentence that a future builder would have taken on faith and sized a `u16`
layout around.
