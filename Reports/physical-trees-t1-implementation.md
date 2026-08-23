# T1 — the severed piece comes down as pieces

**Status: shipped.** The build half of
`physical-trees-design-2026-08-23.md` §8's first stage. Read that report for
*why* this shape; this is what was built, what it measured, and the three
things it found that the design could not have known.

**Freshness: 2026-08-23**, measured on `claude/t1-fell-as-pieces` against
`main` at `7cd1357`, every figure re-measured in one session on one machine.

---

## 1. The headline, on one cut

`filmstrip scene=fell fell=7150`, same seed, same tree, the cut severing the
same **2,648** cells on both arms:

| | main | T1 |
|---|---|---|
| promoted as pieces | **45 cells (1.7%)** | **1,160 cells (44%)** |
| bodies | 4 | **25** |
| body sizes | 8-15:4 | 8-15:28, 16-31:8, 32-63:2, 64-127:1, 128-255:1, **256+:2** |
| became `deadwood` grains | 2,745 | 396 |
| became `litter` | 0 | ~1,500 |
| standing as `log` at frame 8,750 | — (no such material) | **624** |
| peak plant cells riding in bodies | **0** | **1,211** |
| `crumbled to grit` | 3 regions, 5 cells | 176 regions, 283 cells |

**The §8 bar was ≥50% of severed mass promoted and this lands at 44%.** The
gap is not the mechanism and the number that shows it is the composition:
this individual is 2,773 shoot cells of which roughly **1,500 are foliage**,
and foliage goes to litter by design (§5.3). Of the *woody* 1,276 cells,
**1,160 promote — 91%**. The design's prototype measured 58% on a stand it
also measured as 60% wood (14,939 wood / 10,136 leaf, §3.2), which is the
same efficiency on a wood-heavier tree. Recorded rather than relabelled, per
`CLAUDE.md`: the bar was set from one individual and `scene=fell` grows a
different one now that `main` has moved.

**The end state is a lumpy pile with log-coloured masses in it, not a trunk
lying on the ground**, and that is the honest reading of the acceptance
still. The fall is straight down — segmenting and shear velocity are T2 —
so the crown lands where it stood. Posted for the owner's judgement on board
`felling` rather than argued here.

## 2. What shipped

1. **`MaterialDef::fragment_floor`** beside `fragment_rungs`: which rung the
   ladder *starts* at. Default 1 — the exponent that shipped before the
   field existed — so every other `.ron` draws exactly what it drew.
   `wood`/`rootwood`/`log` set 5 → {32, 64, 128, 256, 400}. `MAX_BODY_CELLS`
   untouched (§4.3).
2. **`MaterialDef::woody`**: organism structural tissue floods at eight,
   because `Grow` places at eight, and severs as pieces rather than scatter.
   Rock stays 4-connected and
   `diagonal_only_contact_does_not_connect_two_components` still guards why.
   A diagonal step is refused only when *both* L-routes round it are cracked,
   so a cracked bole still shapes its own pieces.
3. **Three tiers.** `log` (new, `Solid`) is the piece that lies where it
   fell; `deadwood` stays the grit; foliage goes down `breaks_into` to
   `litter` — which is the switch-over `litter.ron`'s own closing note was
   waiting for.
4. **`BodyCell::organism_id`**, two bytes into existing padding. `settle`
   writes a landed limb as `MaterialDef::severs_into` dead tissue instead of
   live `wood`; the census counts plant mass in flight by id rather than by
   kind; `promote` declines to schedule a structural check around tissue
   that is already leaving.
5. **`structural::detached_organism_piece`** — the 8-connected,
   same-organism, *and detached* run, taken in one call, sorted, never
   iterating a `HashSet` (§2a). `over_span` still snaps one cell at a time,
   and the outboard tissue arrives here as a piece on the next
   `anchor_support` pass; the graded chain falls out of that rather than
   being authored.
6. **§9.2**, fixed rather than filed: `schedule_organism_neighbours` walks
   eight, guarded by `a_diagonally_attached_twig_is_rescheduled_by_its_
   neighbour`, which fails against `NEIGHBOURS_4`.
7. **§9.5**, fixed: `filmstrip gif=1` prints its census and its real peak
   counts. It reported `peak chunk bodies in flight at once: 0` on a run
   that peaked at 25.

## 3. Three things the design could not have known

**3a. A fallen log anchored the tree it fell off.**
`plant::is_structural_anchor` treats any `Solid` neighbour as ground — true
of every solid in the world until `log` existed. A chip the axe knocks off a
bole lands beside the stump, and `scripts/acceptance.sh`'s `fell` case went
from **2,360 cells severed to 0**: six bites through the bole, the chips
flying exactly as intended, and the crown standing there perfectly
supported by them. Fixed as data (`MaterialDef::anchors_organisms`, default
true, false on `log`) rather than inferred from shape, because terrain and a
fallen log are both immovable solid matter and no geometry tells them apart.
`cell.attached()` was the tempting shortcut and is a different claim — it
would also stop a tree anchoring on rock a player stacked.

**3b. The structural opt-out held against bending and not against bearing.**
`load::capacity_within` returns `i64::MAX` for `max_unsupported_span ==
u16::MAX` and says in as many words that such a material "does not
participate in the structural system at all" — and `evaluate_within` then
clamped that to `bearing_moment`, because `i64::MAX.min(x)` is `x`. So the
only failure mode that applied to an opted-out material was the one it never
opted out of. Half of every fall's landed pieces were crushed back into
deadwood: **1,191 cells of log delivered by `settle`, 431 standing** at
frame 8,750, with 592 cells of deadwood that had no other source. Guarded on
the opt-out itself; the only two materials in the shipped set that reach it
are `log` and `nest`.

**3c. A piece tier needs a *narrow* palette, and every other debris material
wants a wide one.** `deadwood` runs (64,43,26)–(96,66,40) and `litter`
(132,96,44)–(182,146,78); a field of cells drawing randomly across that
range is speckle, which is exactly right for grains. `log` shipped first
with a 44-unit spread copied from that convention and was invisible inside
the grit it landed in — the shape of a fragment is the thing the eye has to
find in a pile, and a random draw across a wide range destroys it. Narrowed
to 18 units and shifted to a grey timber, well off `litter`'s gold and
`deadwood`'s dark brown. Judged by eye at play zoom on the settled pile,
which is the only way it could have been judged.

## 4. Costs, re-measured in the same session

**Measured twice, because `main` moved 50 commits mid-session** and
`CLAUDE.md` is right that a baseline taken against a tree nobody else has
does not transfer. The first pass isolates *this change* (`7cd1357` against
`7cd1357` + T1); the second is the PR as it stands (`00d1551` against the
merged branch). Both are reported.

### 4a. On the merged tree, against `main` at `00d1551`

- **`ascii`'s organism scene: mean 4.492 ms against `main`'s 4.634, worst
  66.1 against 69.1**, 12,000 frames. The branch measures marginally
  *faster*, which is noise in the direction that flatters it and is reported
  as noise. No scene in `ascii` reports a FAIL on either arm.
- **`seedsweep.sh`: order statistics identical.** `cells lost` max 324, p90
  200, median 0, total 1,285 on both arms; `rock destroyed` total 1,100, max
  837, p90 28, median 0 on both. 23 of the 24 rows byte-identical; `rolling
  3` moves 11 cells on a run that gains ~780.
- **`acceptance.sh` green, all cases.** `cargo test --release --locked --
  --skip root_and_shoot_branching_read_different_slots` green throughout,
  including `tests/worldgen.rs`, whose two water-at-rest failures `main`
  fixed while this branch was open.
- **Both drivers agree** on the felling scene: `driver=serial` reports 2,645
  cells severed / 1,157 as pieces (44%) / 630 log standing, `driver=parallel`
  2,648 / 1,160 (44%) / 617, peak 20 bodies either way.

### 4b. Isolating the change, against `main` at `7cd1357`

- **`ascii`'s organism scene: mean 4.603 ms against `main`'s 4.509, worst
  57.7 against 58.2**, 12,000 frames. Inside the spread the design report
  records for that scene (49.7 / 55.5 / 63.3 across three sessions on
  unchanged code). No scene in `ascii` reports a FAIL on either arm.
- **`seedsweep.sh` (default `dig=6`, 6 presets x 4 seeds): the gated order
  statistics are exactly equal.** `cells lost` max 324, p90 202, median 0,
  total 1,289 on both; `rock destroyed` max 837, p90 28, median 0, total
  1,100 on both. **23 of the 24 rows are byte-identical**; `rolling 3` moves
  38 cells on a run that *gains* 800-odd (its failure count is unchanged).
  That row is a vegetated world and the mover is leaf debris being `litter`
  rather than `deadwood`. Exactly-equal was the prediction, because the
  ladder is material-gated, and the one row that moved is content rather
  than model.
- **`acceptance.sh` green**, including `fell` (2,355 cells severed against
  main's 2,360 on the same case).
- **`ascii`'s ant colony does move**, and it is attributed rather than
  hidden: 76 → 73 live organisms, deliveries 643 → 188, forage trips 88 →
  110, all well inside the harness's own bars. Leaf debris is `litter` now,
  and `litter` carries `food_energy: 480` where `deadwood` carries none — so
  a stand's shed foliage became food. That is the ecology `litter.ron` was
  written for; it is a real behaviour change and it is not free.

## 5. Filed, not fixed

**`settle` drops a cell with nowhere to go, and a felled crown lands in a
pile of its own grit** — which is exactly the configuration where
`nearest_free`'s rings come back empty. **188 of 1,160** promoted cells on
this cut. `open-bugs-handoff.md` §1c owns it; T1 makes it a per-run number
(`FailureCounts::settle_lost_cells`) instead of a remembered one, per the
brief.

**`load::grain_is_footing` returns `cell.attached()` when its probe hits body
material**, which means a grain resting on a *settled* piece is not a
footing, so the piece resting on that grain reads as unsupported. In a deep
pile of alternating log and grit — which is what a fall makes — the upper
pieces fail. Standing measurement after 3b: 318 unsupported failures / 1,307
cells on the settled scene, and `log` 624 standing of 1,191 delivered. The
right fix asks whether the body-material cell under the grain *reaches an
anchor*, not whether it is attached, and that needs `chain_reaches_anchor`'s
memo threaded into a predicate that has none — a load-model change, out of
T1's surface. Filed in `open-bugs-handoff.md`.

**Foliage riding with the branch it hangs on** is the one lever left that
would move both the promoted share and the look, and §5.3 rules it out
("scatter, not a slab"). It is worth an A/B before T2 rather than an
argument: a fallen branch with its leaves still on reads as a fallen branch,
and a bare stick beside a leaf carpet does not. Not built here, because the
report is the plan of record and this contradicts it.
