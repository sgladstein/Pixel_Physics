# The run-order: 6 of 24, and why the other 18 are closed

A deep read of the 24 candidates the screen labelled `EXPIRED` or `UNWIRED` —
the highest-prior set, where the author named a precondition and it appeared to
have arrived. **Every precondition was then verified in source**, which is the
step the screen could not take.

**Six survive. Fifteen were demoted. Two are riders costing almost nothing, and
one is a low-value re-baseline.**

That demotion rate is the finding, not a disappointment: a precondition named in
2026-08 vocabulary and matched by a later reader is a *word* match, and the
verification is what separates it from a condition actually met.

## Run, best first

| # | entry | why it ranks here | cost |
|---|---|---|---|
| 1 | **plants:124** — admit reserves into frontier allocation | the prerequisite shipped *and then some* (`q_peak` **and** `q_now`), while `plant.rs:7265` still says it "is not built yet" | hours, 1 small arm |
| 2 | **structural:074** — the 26× mid-crown amputation | criterion changed from `reach > span` to `torque > capacity`; the docs still assert the hop-bound in present tense | minutes, 1-identifier arm |
| 3 | **structural:013** — directional costs unauthored | verified gap: `log`, `deadwood`, `nest`, `packedsoil`, `rubble`, `windfall` all sit at the rejected flat 1/1/1 | minutes, `.ron` only |
| 4 | **plants:058** — re-derive the canopy-decay constant | storage is a plain `f32` now; the quantization floor that forced 0.5 is gone and the re-tune is recorded as owed, in two places | minutes + a sweep |
| 5 | **plants:044** — the light-optimal slab, re-priced by soil | nutrient now prices *construction*; `crowding_weight: 30.0` identical on all seven species is a counterweight signature | hours, metric to write |
| 6 | **other:080** (+ **plants:081** folded in) | headline condition unmet, but the free-lever list shrank when turgor got priced, so the census now says something an inventory cannot | ~1 hour, nothing to write |

Riders: **rendering:012** is a one-line README correction (the paragraph still
tells sessions live screenshotting is impossible; CLAUDE.md now documents it
working). **structural:039** is a genuine `relax=1` re-baseline the entry itself
demands — take it only if a structural session is already open.

## The top two, in full

### 1 — `plants:124`, admit reserves into the frontier

**The claim and its number.** Dividing stored carbon among tips fused the stand:
**38,605 cells against 1,723**. Held because stock grows with mass, so every
tip's share stays high for ever.

**What changed.** `OrganismCell::q_peak` ships as the monotone high-water mark
(`organism.rs:7654`) *and* `q_now`, the same basipetal sum before the `max`
(`:7671`) — the source names this pair as exactly what is wanted. Meanwhile
`plant.rs:7265` still reads *"That is Phase 3's monotone girth memory, which is
the prerequisite and is not built yet."* Stale. Nothing in `break_buds` reads
either field.

**The decisive check.** One arm first: `break_buds` has **no effect counter**,
so "did it fire" is currently unanswerable. Add `world.buds_flushed`, and
mobilise on the **deficit `q_peak − q_now`** — bounded, and identically zero for
a plant that never had foliage — never on `stock`.

```
cargo run --release --example plant_severance -- species=tree seeds=6 trees=4 \
  frames=40000 cut=12000 fine=200 finefor=4000 arms=control,sever_noload
```

*Alive:* `d_cells` climbs back toward control after the cut, `unreached`
non-zero at the cut, `buds_flushed > 0` after it. *Still dead:* `d_cells` flat
with `buds_flushed 0`, **or** cells overshoot control and `above_ground_width`
widens — the 1,723 → 38,605 fusion is back. *Positive control:* the undamaged
arm must read `q_peak − q_now ≈ 0` and flush nothing extra.

**What it buys.** The sharpest ethos payoff in the set. Cutting a tree is today
a verb with no consequence but removal — measured: **1,344 living cells removed,
and over the next 7,400 frames neither tree rebuilt a crown.** Thriving or gone,
nothing between. This gives the cut a middle: a tree that had a crown comes
back, slowly and differently shaped; one that never had a crown does not.

### 2 — `structural:074`, is the mid-crown landmine still armed?

**The claim and its number.** Scheduling structural checks from leaf abscission
measured **26× destruction — 772 cells standing against 20,213** at the same
shedding rate, the check being the only difference. It masqueraded as *"the
shedding mechanism collapses the stand at any setting"* across a whole
eight-setting sweep — which is this project's *"when every setting of a sweep
fails the same way, suspect the sweep"* in its original form.

**What changed.** `structural.rs:606`: the criterion is now *"torque > capacity,
not reach > span"*. `plant.rs:7320`: anchoring is `support == u16::MAX`, a
separate question from distance. But `shed_stranded_leaves`' own doc
(`plant.rs:11672`) and the note at `:10870` **both still assert the hop-bound in
the present tense**. That contradiction is the finding and it is cheap to settle.

**The decisive check.** One identifier at two call sites (`plant.rs:10613`,
`:10643`), two binaries:

```
cargo run --release --example plant_severance -- species=tree seeds=6 trees=4 \
  frames=30000 cut=12000 arms=control
```

*Alive:* standing `cells` within noise of the unmodified binary, `unreached`
near zero — the amputation was a property of the retired rule. *Still dead:*
`cells` collapses toward the recorded ratio — the bound survives the criterion
change, and **every Phase 3 damage number taken since is void**. *Positive
control:* run `arms=sever_noload` on the unmodified binary first and confirm
`unreached` moves at all.

**What it buys.** Whether damage to a plant can have a middle at all. Today a
rock through a canopy either does nothing or takes the whole upper crown; there
is no "a branch came off". And it is a prerequisite rather than a feature — it
says whether existing damage measurements can be trusted.

## The nine that were already resolved, in code, and never written back

This is the systematic finding. Nine of the fifteen demotions are **closable
today with a source citation rather than a measurement**, because the re-test
was done and recorded in a doc comment or an asset and never propagated to the
register:

| entry | where the answer already lives |
|---|---|
| `creatures:066` | `ant.ron` — the re-test ran, with numbers: peak bank **1,601** against a floor of 1,041, colony reaching **generation 13** |
| `structural:040` | `load.rs:2137` — the gate shipped in the exact form the clause prescribed |
| `structural:038` | `load.rs:240` — the divisor is *gone, by name*; `bearing_moment` replaced it |
| `structural:015` | `structural.rs:330` — condition met, defer kept deliberately for a different, still-valid reason |
| `plants:069` | `plant.rs:9123` and `:8940` — child provisioning shipped in two sites, better designed than the reverted version |
| `plants:071` | `plant.rs:10827` — self-pruning shipped as whole-plant, distal-first die-back |
| `plants:074` | `plant.rs:10648` — shade abscission is built, graded, and already reads noon-equivalent light |
| `plants:019` | `plant.rs:11948` — `cross_section_axis` landed **and** `pipe_ratio` was re-tuned past the reverted value |
| `plants:009` | the replacement shipped and was swept at landing, settling at `branch_priming: [3]` |

**The register's `Re-test when:` clauses are being resolved in doc comments and
never back-propagated.** One pass writing these nine resolutions into the
register would shrink the next screening set by more than a third — and it is
the same gap that `creatures:039` fell through from the other direction.

## The two most expensive mistakes this run-order avoids

**`field:003`** reads as met from any summary and is demonstrably unmet in
source: the clause's condition is *storage-form* — "retry if the field's tile
storage stops being a `HashMap<ChunkCoord, FieldTile>`" — and `fxhash.rs:114` is
`pub type ChunkMap<V> = FxHashMap<ChunkCoord, V>`. **Still a HashMap.** The
`ChunkGrid` dense rewrite was applied to `Chunk`, the cell grid, not to field
tiles. A multi-hour rebuild for an optimisation carrying a known gameplay
regression.

**`creatures:066`**'s answer is already sitting in `ant.ron` with the numbers
attached.
