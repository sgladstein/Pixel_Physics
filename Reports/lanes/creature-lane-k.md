# Lane K — §R3 diagnosis

**Branch** `claude/creature-lane-k-r3-diagnosis`
**Head SHA** _(filled at the end of this note)_
**Report** `Reports/creature-chain-head-loss-2026-08-30.md`
**Status** done. Diagnosis complete, §R3 rewritten in place, no `src/` change.

---

## The answer, in one sentence

**§R3 is neither cannibalism nor the hatch-site predicate — the colony above
two cells never dies at all.** A `Chain(n >= 3)` loses its `CellType::Head`
marking to a self-overwrite in `relocate_chain`, so every instrument that
finds an ant by looking for a head reports an empty world over a living,
feeding, delivering population. **The extent lever is recoverable.**

## What another lane must know

- **`src/sim/creature.rs` and `brain.rs` are untouched.** Everything went
  through `examples/creature_probe.rs`, which is not Lane J's. **The bug is
  in `creature.rs` and the fix is not mine to make** — see below for exactly
  where.
- **The fix, for whoever owns it.** `body_after_step` builds a chain's next
  body as `[head, chain[0], ..., chain[n-2]]`. When the head steps into a
  cell its own body occupies, that list holds **one position twice**, and
  `relocate_chain` writes the carried cells in order, so the trailing
  Segment lands on top of the Head. `Chain(2)` is immune because its two
  positions are always distinct. `reconcile_chain` cannot see it:
  `chain.first()` is unchanged and every entry is still owned, so it books
  no death, no injury and no `meat_lost`.
- **Do not re-derive the pricing, and do not chase cannibalism.** Both are
  ruled out by measurement — lane D ruled out the bill, and this lane ruled
  out cannibalism with a positive control (`eatskin=on` moves `meat_lost`
  0 -> 40,320; `kinfood=off` puts it back to 0; the shipped arm is
  byte-identical to `kinfood=off`).
- **`creature_probe` gained five knobs and four readouts**, all defaulting
  to shipped behaviour, so every number already filed still reproduces:
  `kinfood=off`, `eatskin=on`, `pitch=N`, hex `seed=`, and unconditional
  printing of the energy-ledger accounts, `bodies ever built`, the
  head-cell/registry population gap, chain integrity and the standing
  cell-type histogram. `Reports/instruments.md`'s row is updated.
- **`built - deaths = registry` is a checkable identity** and it held on
  **every one of the sweep's runs, 0 exceptions**. It is the cheapest way to
  tell a real death from a bookkeeping hole in any population run; reach for
  it before believing any extinction.
- **The founder pitch is a confound in both `creature_probe` scenes.** They
  plant at `x = base + i * 2`, calibrated to the shipped two-cell ant, so a
  three-cell body overlaps its neighbour. Anyone measuring a multi-cell body
  in this harness must set `pitch=` or their placement number is about the
  scene.

## What I did not do, and why

- **No fix.** Out of scope by the brief, and `creature.rs` is another lane's.
- **No review card.** The brief asked for one if an arm produced something
  visible. Neither does, and the reason is the finding itself: the bug is a
  **label**, so a `Chain(3)` colony looks identical on screen whether or not
  its heads exist — which is exactly why this went unnoticed for so long.
  A card would have asked the owner to judge two pictures that are the same
  picture. The cannibalism arm (`eatskin=on`) *is* dramatic, but rendering
  it needs a `body=`/`eatskin=` knob in `examples/filmstrip.rs`, a
  99-landing contested file another lane had just landed in, and the brief
  was explicit about not letting this grow.
- **The world-terrain half of the placement drop is unexplained.** On the
  slab it is the founder pitch (28 bodies at `pitch=2`, 46 at `pitch=4`). On
  the generated world it is not (34 -> 35), and I did not chase it.
- **`start_energy` is still flat while burn is per cell**, so an *n*-cell
  animal keeps a `1/n` starvation horizon. Lane D flagged it and left it
  deliberately; nothing here touches it, and it is a real remaining cost of
  a longer body.

## Gates

`cargo +1.98.0 clippy --all-targets -- -D warnings` clean on the CI
toolchain · `docscheck` clean · `bugindex --check` index current and
identifiers unique · `cargo test --lib` (see the PR for the count).
