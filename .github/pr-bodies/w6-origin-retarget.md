**A plant *evolved* its root tips into shoot tips, and the shoot it then grows
is made of root wood — that is the owner's pale-cream plant, and it is not a
bug in the material rule.** This is the origin of §W6, traced rather than
inferred, and it settles why the two substrate rules in the previous change
measured as a null.

Bug register: [`Reports/open-bugs-handoff.md`](../Reports/open-bugs-handoff.md) §W6.

## The trace

`PIXEL_PHYSICS_ROOT_TRACE=1` (`plant.rs::trace_root_material`) prints every
moment root material lands on an unambiguously *shoot* cell type —
`GrowingTip`, `Leaf` or `DormantBud`. `MatureBody` is excluded deliberately:
it is the one type genuinely shared by root and shoot, so a rootwood
`MatureBody` is settled root tissue and not an anomaly.

The census that preceded this counts a **standing** population, and
`CLAUDE.md`'s rule is that a standing artifact and the rate that creates it
are different questions. On the card's world, 5,000 frames:

| transition | count | |
|---|---|---|
| **`relabel-after-grow: RootTip -> GrowingTip`** | **22** | **the source** |
| `grow-child: GrowingTip -> GrowingTip` | 26 | downstream |
| `relabel-after-grow: GrowingTip -> DormantBud` | 10 | downstream |
| `grow-lateral: GrowingTip -> GrowingTip` | 2 | downstream |

The first event is at **(102, 200), frame 3540** — on the soil line — and
every later one climbs from it: y 199, 198, 197, 196, 195, 194, 193 by frame
4154. One cell converts, and an ordinary shoot grows up out of it.

## Why that transition cannot come from the assets

Every shipped species declares its root's grow rule as `(when: Grew, becomes:
MatureBody, child: Some(RootTip), lateral: Some(RootTip))`, and
`builtin_fate`'s frontier arm independently gives `(RootTip, Grew) ->
MatureBody`. Neither can produce a `GrowingTip`.

The only mechanism in the engine that can is **`organism::FateOp::Retarget`**
— 60% of fate mutations — which picks a random rule, a random slot
(`becomes`, `child` or `lateral`) and a random `PLANT_CELL_TYPES` entry. A
lineage retargeted its root's `Grew` rule from `MatureBody` to `GrowingTip`.

**Nothing about that is wrong.** It is a legitimate evolutionary move and a
real plant behaviour — root-borne suckers. What is wrong is only that the new
shoot keeps root material, because material propagates from the parent and
every descendant inherits it.

## What this explains

- **Why both substrate rules are a null.** The tissue standing in the sky is
  *shoot* tissue, and shoots belong in the air. `growable` and `thicken`
  refuse *root* growth with no ground against it; this was never root growth.
  The gates are correct and simply do not describe this population —
  `CLAUDE.md`'s *a change that moves nothing*, in the form where the condition
  is sound and the population is not the one it names.
- **Why it is so seed-dependent.** The mutation has to arise and survive. Six
  `grove` seeds had none; the card's default-seed world had one.
- **Why `World::root_shoots_launched` read zero** while an earlier session
  counted seven above-ground clumps and recorded the contradiction in
  `examples/genome_reach.rs`'s own doc. That counter fires on `cell_type ==
  RootTip && lateral_type not in {RootTip, MatureBody}` — it watches the
  **`lateral`** slot. This mutation hit the **`becomes`** slot, so the counter
  was blind by construction. Another *ask what your number counts*, and it
  closes that open question.

## What ships

- **`trace_root_material`**, off unless `PIXEL_PHYSICS_ROOT_TRACE=1`, read
  through a `OnceLock` bool checked before any material lookup, so the shipped
  path costs one relaxed load. Capped at 200 lines by an `AtomicUsize` so a
  24,000-frame run stays readable. Deliberately **not** a `World` field:
  `world.rs` is the second most collided file in this repo and this is
  scaffolding, not a shipped counter.
- **`thicken` is not traced, and that is a conclusion rather than an
  omission** — it only ever writes `MatureBody`, so it can spread root
  material but cannot be the site that first puts it on shoot tissue. A call
  there would be dead code that reads like coverage.
- **§W6 rewritten** around the origin, with the fix now settled for a stronger
  reason: the genome may legally retarget any cell type to any other, so there
  is no version of lineage-propagated material that stays correct. Material
  must follow the cell's **role** — and the fix has to cover the *relabel*
  sites, not only the creation sites, because this conversion happens by
  relabel.

## Gates

`cargo test --lib` · `cargo clippy --all-targets --release --locked -- -D
warnings` · `scripts/docscheck.sh` — all clean.
