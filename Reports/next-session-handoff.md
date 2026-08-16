# Next session: fix the unzip, then re-judge everything else

**Written to be picked up cold.** State, the one live bug, the exact loop
for working it, what has already been tried and must not be retried, and
what comes after.

**Read first:** `CLAUDE.md`, then `Reports/building-rethink.md` §3a and §6
(the live edge), then this. `Reports/destruction-plan.md` holds the wider
backlog and the "Pending owner verification" list.

---

## 0. Where things stand

`master`, pushed, 406 tests, clippy clean, six acceptance cases gating in
CI via `scripts/acceptance.sh`.

The destruction model is **right and working**. Over this session it went
from reach-vs-span to torque-vs-capacity, gained section failure, load
flow over parallel supports, crack-driven detachment, a stress view (`N`),
a rectangle/room/line build tool (`Z`), a precise dig verb (`D`), and an
acceptance suite that asserts mechanisms rather than outcomes.

The owner's verdict after the intact reframe was **"big improvement"** —
building now stands, which was the headline complaint. Then one dig
unzipped a room into dust, which is where it sits now.

### The one live bug

> One click of `D` collapses an entire room, and it comes down as a
> uniform field of grit rather than as pieces.

**These are one failure.** A cascade that advances a cell at a time
necessarily produces dust: each step hands `rigid::fracture` a region
below `MIN_FRACTURE_CELLS` (6), so it is not fractured at all — it falls
through to per-cell `break_free`, and per-cell conversion *is* powder.

Corollary worth holding onto: **do not tune the fragment ladder to fix the
dust.** The ladder never receives a region worth splitting. Fix the
propagation and the pieces get large for free.

---

## 1. The fix, in the order to try it

### 1a. Prime suspect: the propagation front

`load::is_structurally_interesting` treats an intact cell as evaluable
when it is **adjacent to empty**:

```rust
NEIGHBOURS_4.iter().any(|&(dx, dy)| {
    edge_is_cracked(world, x, y, dx, dy)
        || world.get(x + dx, y + dy).material == material::EMPTY
})
```

So removing material makes its neighbours evaluable; if they fail they are
removed, which makes *their* neighbours evaluable. A self-propagating
front, bounded by nothing to do with how much damage was actually done.

**Why this only became fatal now.** Under the old model, built material
was unattached and already evaluated everywhere, so the front had no fresh
territory to advance into. Making material intact did not create the
front — it gave it somewhere to go. This is a regression introduced by the
reframe, not a pre-existing bug that surfaced.

**The change:** an intact cell should be evaluable because it is
*damaged*, not because it is next to a hole. Drop the empty-adjacency
clause and keep the crack clause.

**⚠️ This is exemption-shaped, and §3a is the warning.** It must be checked
against the case it could break: a structure that is genuinely unsound
must still fall. Specifically —

- `scene=ligament` **must still snap at the neck.** It is the canary: it
  fails from geometry alone with nothing touching it, so if adjacency is
  the only thing making its neck evaluable, this change silences it.
  Acceptance will catch this; if it does, that is the signal 1a is wrong
  in this form, and 1b is the next move rather than a smaller version
  of 1a.
- A blob painted in mid-air must still come down
  (`an_unsupported_foreground_blob_does_not_hang_in_mid_air`). That one
  goes through `!supported`, not the interest predicate, so it should be
  unaffected — confirm rather than assume.

### 1b. The detach footprint

`mine` and `strike` call `structural::detach_exposed_neighbours` for
**every cell they remove**, at radius `DETACH_DEPTH` (3). A radius-4 dig
therefore strips protection from a band roughly 13 cells wide. On a wall
thinner than that, one click un-protects the whole section — which is the
unzip arriving by a second route even if 1a fixes the first.

That radius was chosen when `attached` meant "part of the background
massif" and detaching widely was nearly free. It now decides **how much of
a structure loses its multiplier per click**, which is a completely
different job for the same number.

Note the tension before changing it: `DETACH_DEPTH` is 3 rather than 1
because at 1 it "stripped a single-cell skin off the dig face, so
everything that subsequently broke away was a one-cell sheet". Under the
current model the piece's thickness comes from the section, not from the
loosened band, so that rationale may no longer hold — check whether it
does before treating 3 as load-bearing.

### 1c. Region size at failure

Even a correct cascade should hand `fracture` something worth splitting.
`filmstrip` already prints `overloaded N (M cells)`; **`M/N` is the number
that says whether pieces or grit are coming out**, and nothing currently
reports it directly. Add the mean, or better the max, so a run can be read
at a glance.

Target to beat, measured on `scene=worked` at its first capture after D1
landed: 102 bodies / 1,888 cells, i.e. ~18 cells per body. A room coming
apart should be in that territory. If it is at 1–2, the front is still
running.

---

## 2. The measurement loop

Fast, and it is the whole loop — do not reach for anything else:

```
cargo build --release --example filmstrip
bash scripts/acceptance.sh                     # six cases, mechanism-asserting
target/release/examples/filmstrip.exe scene=built \
    start=2 every=40 count=6 crop=0,40,512,280 zoom=2 \
    out=target/filmstrips/built.png
```

`scene=built` already exists — it reconstructs the player's shapes
(a column with arms, a hook, an arch) through the real brush. **It does
not yet contain a room with a door cut in it, and it should**: add one
that uses `Tool::Room`'s geometry and then `rigid::mine` at a wall, which
is the exact reproduction of the report.

Read `failures: overloaded N (M cells), unsupported N (M cells)` next to
the image every time. `CLAUDE.md`: a coherent slab and a scatter of grains
are indistinguishable in a contact sheet.

**Timings:** always `repeat=3`, always the minimum. This machine produced
18.0 ms twice running on a scene that schedules zero structural work, and
40.5/55.6 ms on scenes that measure 14–19 ms moments later. Three
near-misses this session where contention was almost read as a regression.

**Images:** write to `target/filmstrips/` (gitignored) and link them with
relative markdown paths — the owner's client does not render file-send
cards.

---

## 3. What has been tried and must not be retried

- **Dividing torque by the section.** Fixed a beefy block, broke
  `scene=undercut`. Peak bending stress in a section of depth D is `M/D²`,
  which the model already has right — capacity carries the `D²`, torque
  the `M`. Dividing again gives `M/D³`. It also double-counts, because a
  shelf's rows already chain independently.
- **Intact as an *exemption*.** Broke `scene=ligament`, which fails from
  geometry alone. And the owner's own objection is the real argument: a
  structure standing only by exemption has no answer the moment anything
  asks, so one chip levels a castle. It must be a multiplier.
- **Raising `max_unsupported_span` to hold player spans.** 16→40 with
  `attached_span_bonus` 12→2 holds terrain capacity constant (1536→1600)
  and does make built spans stand — and stops `undercut` spalling
  entirely, because an undercut shelf spalls *precisely because* the
  loosened rock has become foreground.
- **Scheduling the parent on settle** (to fix load ordering). Every
  settling cell raised a check on its parent, which raised one on its
  parent, faster than the queue drained: 26 pending sites climbing to
  4,064 with frame cost tracking it from 2.5 ms to 3,160 ms. The bounded
  in-tick chain walk replaced it.
- **Four support models** (confinement, thickness, attachment-as-anchor,
  reach) — see `Reports/load-model-handoff.md` §6.

---

## 4. After the unzip is fixed

In order, and **re-judge each rather than inheriting its justification**:

1. **Tumbling.** Nothing suggests it is broken — `ChunkBody::spin` and the
   promotion nudge both work. It has had nothing to tumble while regions
   are one cell. The owner wants "things tilted and fell over more as
   large pieces"; check whether that is already true once pieces exist,
   before touching `SPIN_PER_SPEED`.
2. **Playtest.** `Reports/destruction-plan.md`'s "Pending owner
   verification" list, plus the two questions this session added: does
   stone's 6-rung fragment ladder read right, and is the build envelope
   satisfying.
3. **E1 (push damage outward from the break).** Its headline
   justification has already been delivered by other means — the
   concentration defect was fixed by load flowing over every support, and
   the cost bound by `ROOTWARD_CHECK_STEPS` 128→48 plus the per-frame
   budget. **Re-derive whether it is still worth it** rather than
   inheriting the plan's argument for it.
4. **F3** (replay a playtest report from a world dump) — still the biggest
   gap in the loop. Every report this session had to be reconstructed into
   a scene by hand, and at least one reconstruction was wrong.
5. **C2** (mortar as a material) and doorway/window cuts on the room tool.

### Known defects not yet confirmed

- **`GRANULAR_CAPACITY_DIVISOR` may be dead code.** Flagged by a
  concurrent review: `evaluate_within` early-returns on `is_anchor`, which
  includes `rests_on_ground`, so a cell resting on powder may still be
  fully exempt from the torque test — contrary to what the B1/B2 commit
  claims. **Not verified by me.** Check before reasoning about
  debris-propped structures.
- **`filmstrip` never renders inside its timed loop**, so every worst-frame
  number in this repo's history excludes drawing. The owner found a render
  regression the harness structurally could not see. Timing a render pass
  is the highest-value item left in Phase F.

---

## 5. Repo gotchas this session paid for

- **The app locks its exe.** `cargo build` fails with "Access is denied"
  while it runs; `cargo test` and building `--example filmstrip` still
  work.
- **The tree is worked concurrently.** A full `cargo test` failed twice
  mid-run from another session editing `src/sim/load.rs` under it, and
  passed on a re-run. Stage explicit paths, never `git add -A`, and check
  `git status` before committing — `Reports/worldgen-design.md` and
  `Reports/prior-art-worldgen-slicing.md` are someone else's work in
  progress.
- **Frame 0 is not a measurement.** Every scene spikes there, `terrain`
  included, and it schedules no structural work at all. `filmstrip`
  excludes it deliberately.
- **A guard test must be seen to fail.** The acceptance harness was
  verified in both directions (demanding `capped` collapse exits 1;
  demanding `worked` stand reports "expected at most 0 structural
  failures, got 15") before being trusted.
