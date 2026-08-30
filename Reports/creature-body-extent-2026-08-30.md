# The body was free, and no body but the shipped one survives

Lane D of the creature program. Two questions were handed to it: make extent
a working lever (E10), and make a bigger body *cost* something, because the
lane brief had spotted in the code that it did not.

Both were built. The second one changed what the first one means.

## 0. Findings, in the order they change a decision

1. **E10's premise is false, and it was false in one line.** E10 authorises
   chain *length* as the cheap route to a visible creature on the stated
   grounds that *"per-cell metabolic cost already prices a longer body, so no
   cost system is owed."* Nothing in `creature.rs`'s cost path read
   `chain.len()`. Measured by putting the old behaviour back into this
   change's own guard: a two-cell ant and a six-cell ant both paid
   **0.1116 energy per tick — a difference of exactly zero**. A longer body
   was strictly free: 3x the ink, *less* blocked movement, an identical bill.
2. **It is priced now, and the shipped ant did not move.** `idle_cost` and
   `move_cost` are per body cell, charged against the live chain. `ascii` is
   **byte-identical on every counter across 31 scenes** — including a
   27-creature colony that runs 11,102 moves, 57 eats and 4 deaths, so the
   path is live rather than untouched.
3. **And it turns out not to be the thing standing in extent's way.** At the
   shipped seed and horizon, **no chain longer than two cells leaves a living
   colony** — at three, four, six or nine cells, and *at the bill that
   shipped yesterday as much as at the new one*. The paired control is §4b:
   `body=3` and `body=6` reach `live 0` with the pricing reverted to its old
   flat total. It reproduces on a flat slab, so it is not terrain.
4. **So the extent lever cannot be handed to the owner as a choice yet.**
   The lane brief asked for a runtime selector and a blind A/B so the owner
   could pick the shipped body. Offering that choice today would be offering
   a choice between one animal that lives and four that go extinct, which is
   not the question the appearance report was asking. §6 says what is owed
   before the card is worth posting.
5. **Shade-by-cell-type is built and is free.** It is the appearance report's
   *"smallest change that opens any of this"*, it costs no per-cell state,
   and it is off by default so no shipped animal moved.

## 1. What was built

| | |
|---|---|
| `organism.rs` | `idle_cost` → `idle_cost_per_cell`, `move_cost` → `move_cost_per_cell`. `ShadeRule` (`Random` default, `Countershade`) on `CreatureDef` |
| `creature.rs` | `live_body_cells`, charged at all four cost sites; `shades_by_luma` and `body_shade` at the hatch seam |
| `examples/creature_probe.rs` | `body=`, `idle=`, `move=` — the instrument that made the control in §4b possible |
| `assets/species/*.ron` | six species re-derived, table in §3 |

**The rename is deliberate and is not tidiness.** Re-deriving the numbers
alone would leave every species file reading `0.05` where it read `0.10`,
with nothing on the page to say the two are the same animal — and the next
person to author a species would write the whole-animal cost into a per-cell
field. `CLAUDE.md`'s rule is that when a fix changes what a number *means*,
re-deriving what reads it is part of the fix; a name is the cheapest way to
stop the meaning being lost.

**Charged against the live chain, not `BodyPlan`.** An animal that has lost
cells is a smaller animal and burns less. That fell out of reading the right
quantity rather than being designed, and it has its own guard.

## 2. The premise, and the measurement that falsifies it

`spent` was `def.idle_cost + synapse_tax`, and `spent += def.move_cost` on a
step. Both are constants of the *species*, not of the body. So:

> A longer body put more on screen, was blocked **2-6%** of its moves against
> **25-43%** for a rigid body of the same size, and cost the same to run.

An unpriced lever ratchets to its maximum and expresses nothing — the
degenerate codomain that took plant reproduction to zero
(`why-changes-cost-so-much-2026-08-27.md`). Nothing had ratcheted only
because body length is not heritable yet; E10 wants it to be.

**The number that says it was really zero.** `a_longer_body_costs_
proportionally_more_to_run` is a paired 2-vs-6-cell comparison holding
everything else fixed. With the pre-change flat charge injected back in it
reports `0.11155701 -> 0.11155701 (difference 0, expected 0.2)`. That is the
guard proving it can fail, and the old behaviour quantified in one line.

## 3. Which constants were re-derived, and which were deliberately not

This is a shared-budget reallocation on top of the one #142 just did, so
every constant is named with a verdict, including the untouched ones.

| constant | before | after | verdict |
|---|---|---|---|
| `idle_cost` → `idle_cost_per_cell` | 0.10 per animal | **0.05 per cell** | **re-derived and renamed.** 0.10 / 2 cells. At the shipped ant, `0.05 x 2 = 0.10` exactly, so E14's horizon is untouched. |
| `move_cost` → `move_cost_per_cell` | 0.25 per animal | **0.125 per cell** | **re-derived and renamed.** 0.25 / 2. The **2.5:1 idle:move ratio** is the researched worm figure that `ant.ron` defends and E14 preserves; dividing both operands by the same 2 cannot move a ratio, so it survives. |
| `start_energy` | 200 | 200 | **left alone, and this is the load-bearing choice.** #142 bound the bank ceiling to it (`hunger_fraction * start_energy + one mouthful`) hours earlier; scaling it with the body would re-open that arithmetic the same night. **The consequence is real and is not hidden**: with the tank flat and the burn per cell, an *n*-cell animal's starvation horizon is `1/n` of a two-cell one. §6 argues that is the wrong model and says what the right one costs. |
| `body_energy` | 480 | 480 | **cannot move from a species file.** `ant`, `chitin_*` and `corpse` all carry `food_energy: 480` against it and breaking that re-opens the corpse pump. Out of this lane's reach, exactly as #142 found. |
| `synapse_fraction` | 2.222e-6 | unchanged | **left alone, and it was pre-paid for this.** `organism.rs`'s own doc says it had to become a fraction *before body size is heritable (S8), because `start_energy` becomes a function of the body then and an absolute tax would quietly re-price thinking for every size.* That work is already done. |
| `hunger_fraction` | 0.5 | unchanged | left alone. A fraction of `start_energy`, which does not scale with the body, so nothing about it changed meaning. |
| `reproduce_threshold` | 1100 | unchanged | left alone, but **its floor moved**: `birth_cost` is `birth_grant + body_energy * cells`, so the bar goes 1,040 at two cells to **2,960 at six**. `reproduce_at` floors the authored value at `birth_cost + 1`, so the authored 1,100 is silently replaced for any body above two cells. Not re-derived because #142's dead-end entry is exact: tuning it downward does nothing. |
| `beetle` `idle_cost`/`move_cost` | 0.15 / 0.4 | 0.0375 / 0.10 per cell | **total deliberately preserved** — 4 cells restores 0.15 and 0.40. A beetle is a different species, not an ant at another size, so its *total* is what carries meaning. This leaves it cheaper per cell than an ant (0.0375 against 0.05), which is **inherited from the two authored totals rather than chosen**; it happens to point the way real metabolism scales, and should not be read as a model. |
| `ant_long` / `ant_block` / `ant_wide` / `chitin_pale` | 0.10 / 0.25 | **ant's rate, 0.05 / 0.125** | **re-derived, and this is where the change bites.** These are the appearance report's own arms, and they carried the *identical* whole-animal cost as the two-cell ant at 6 and 9 cells. They now pay **3.0x** and **4.5x**. Every appearance figure in that report was measured on bodies that were free. |
| `grazer_horizon()` test constant | `start_energy / idle_cost` | `start_energy / (idle_cost_per_cell * cells)` | **re-derived in form, unchanged in value** at the shipped ant. Written out rather than folded so that it is visible it is now a function of the body. |
| `creature_space.rs` swept `move_cost` | 0.08 | 0.04 | **a unit change, not a retune.** The harness runs a two-cell ant, so half the old whole-animal figure is the same bill. §13k's mapping is in the old units and its numbers must be doubled to be read here — noted at the line. |

## 4. The controls

### 4a. The shipped ant did not move, and the path was live

`ascii`, changed binary against a baseline built from this branch's merge
commit **in the same session on the same machine**: **zero non-timing
differences across 31 scenes, 0 skipped.** The colony scene inside it runs 27
creatures, 11,102 moves, 2,330 pickups, 57 eats and 4 deaths, so this is an
identity result over thousands of executions of the changed lines rather than
a scene that never reached them.

### 4b. The multi-cell collapse is **not** this change

The control that matters, and the one that overturned this lane's working
assumption. `creature_probe terrain=world seed=0xA17 frames=12000`, and the
`idle=`/`move=` knobs exist precisely so the same body can be run against the
bill it used to pay — otherwise `body=6` confounds two changes at once, which
is `CLAUDE.md`'s rule about a rider travelling with the mechanism.

| arm | peak pop | moves | deliveries | deaths | **live @12k** |
|---|---|---|---|---|---|
| `body=2` (shipped, byte-identical) | 45 | 20,351 | 733 | 16 | **29** |
| `body=3`, **old flat bill** | 34 | 14,413 | 652 | 12 | **0** |
| `body=3`, priced | 34 | 10,083 | 339 | 18 | **0** |
| `body=6`, **old flat bill** | 29 | 13,661 | 485 | 6 | **0** |
| `body=6`, priced | 29 | 5,917 | 352 | 17 | **0** |
| `body=4`, priced | 30 | 8,840 | 624 | 15 | **0** |
| `body=9`, priced | 26 | 5,547 | 457 | 15 | **0** |

Read the paired rows. **Peak population is identical within each body size
across both bills**, so placement is the same and the arms are properly
paired — and both reach `live 0`. The pricing is real and bites hard (at six
cells it takes moves 13,661 → 5,917, a 57% cut, and deaths 6 → 17), but it is
**not the difference between a living colony and a dead one.** That was
already decided before this change existed.

**And it is not the terrain.** On the hand-built flat slab, `body=2` gives
peak 55 / live 24 and `body=3` gives peak 28 / **live 0**. A flat floor is
the easiest ground a chain could be asked to stand on.

**Two distinct pre-existing effects, neither of them priced:**

- **Placement roughly halves at three cells**, 45 → 34 on the world and
  55 → 28 on a slab. A `Chain(n)` is laid out as *n* cells in a straight
  horizontal line, every one of which must be empty at the hatch.
- **The colony is then consumed.** At `body=3` on the old bill, peak 34 and
  deaths 12 leave 22 animals unaccounted for by the death counter. Ants read
  each other as food — a frame-0 dump shows `food in reach: ant 480 ant 480
  ant 480 ant 480`, and ant flesh at 480 is the richest food in the world.
  **This is stated as the leading hypothesis and not as a result**: it has a
  mechanism and a frame-0 observation behind it, and no isolating control.
  Filed rather than concluded.

## 5. Shade by cell type

The appearance report's §7 seam: the `CellType` was computed at the hatch and
thrown away as far as appearance goes, while the shade beside it was an
independent random draw.

**It is assigned once and stays correct for ever, at zero per-step cost.**
`relocate_chain` moves whole `Cell` values rather than rebuilding them (P-1,
so burn state rides along), and position `to[i]` always receives the cell
from `from[i]`. For a chain that means index 0 is always the head cell; for a
rigid body the facing mirror flips `dx` and never `dy`. **So a cell's index —
and its height within the body — is invariant under motion**, which is what
makes a hatch-time assignment legitimate rather than something that would
smear after one step.

`Countershade` gives the head the palest entry, then grades:

- a body with **vertical extent** grades top-to-bottom, pale above and dark
  below — countershading, the only thing that holds contrast against both the
  sky an animal is silhouetted on and the ground it tunnels in;
- a **`Chain` has no underside at all**, being one cell thick, so the grade
  runs head-to-tail instead. That is not a fallback for its own sake: E10's
  own note is that a long chain reads as a *worm*, and what makes it read as
  an animal is a visible front and a segmented flank rather than one flat
  smear.

**The palette is not widened**, which is `dead-ends.md`'s `log.ron` entry: a
wide spread makes a field of cells draw randomly across a range, which is
speckle, and speckle destroys the one thing a small body has. Both rules
choose from the palette the species already has; `Countershade` differs from
`Random` only in *which* existing entry each cell gets.

Default is `Random`, so no shipped animal moved — E10 is the owner's standing
decision that the shipped ant does not change without a verdict, and a guard
asserts `ant.ron` still carries `Random` and two cells so that changing it is
a deliberate act rather than a merge accident.

### 5a. What it looks like, measured rather than asserted

Rendered through `creature_look mode=live` on the `flat` preset, 600 frames,
arms that are the **same simulation** — 15 ants placed, 135 body cells on
screen, 9.00 cells per creature on both — so only the paint differs:

| nine-cell body | ink per creature |
|---|---|
| `ant_block`, random shade | 1,295 |
| `ant_block_shaded`, countershaded | **1,270** |

**That is slightly *less* ink, and it is expected rather than a regression**:
countershading deliberately puts the pale cells on top, where they contrast
less against a bright sky. So this is **not** a contrast win and is not
claimed as one — §3 of the appearance report already established that no flat
value wins against both backgrounds, and the claim here is about *anatomy*,
which is not what `ink` measures.

**Whether it reads as an anatomy is on the owner's queue**, blind, as card
`20260830T063631048Z-ae976a` (board `creatures`). Posted rather than asserted
because "this looks better" is exactly the claim this repo has been wrong
about three times.

**On a chain it is barely visible, and that is worth recording as a limit.**
The same pair built at `Chain(6)` renders as two near-identical rows of dark
bars: a single-file chain silhouetted against sky is a solid dark shape
whatever its cells do internally, and the ant palette spans only 24 luma units
by design. So the head-to-tail grade is real, unit-tested and correct, and it
does **not** on its own answer E10's "a long chain reads as a worm" — it needs
either a body with an underside or a horizon where the chain is seen against
ground rather than sky. Not a reason to widen the palette; that is a recorded
dead end.

## 5b. Two things measured elsewhere that change what this is worth

Both arrived from the coordinator after the package was built, and both
qualify it rather than contradict it. Neither was measured here; the numbers
are theirs.

### The body is a motion lever too, and §5 of the motion report already priced it

`creature-motion-design.md` §5, *"What the one verb buys, per body"*: launch
speed is `sqrt(2W/m)` off the **cell count** and descent is a drag law off the
**bounding box**, so `ant_wide` and `ant_block` — same mass, 5x2 against 3x3 —
launch identically and fall **2.3x** apart. Its closing line is the one that
matters here: *"a longer chain is a different animal in motion, not just a
longer one on screen."*

**And that table's cost column was already claiming this change.** The long
chain's entry reads *"what it pays: more metabolism per tick"* — which was
**not true when it was written**, for the reason §2 gives. So this change does
not merely price a body; it makes `creature-motion-design.md` §5 honest. That
is a second document carrying E10's false premise, and it is now correct
without being edited.

**Note the interaction with the launch verb, which this change touches.** The
impulse cost is `move_cost_per_cell * body_cells * LAUNCH_COST_IN_MOVES`, so
launching now scales with the body as walking does. At the shipped two-cell
ant it is arithmetically identical (0.125 x 2 = the old 0.25); above two cells
a hop costs proportionally more, which is the intended direction — a heavier
animal should pay more to leave the ground.

### The falls-per-move gate, run because of that message

`forage_probe gate=1 seeds=12 frames=12000 spacing=4` — the guard E11's
authorisation was made conditional on, and **it was missing from this lane's
gate list until the coordinator named it**. Filed as a process note as much as
a result: the change touches the launch verb's cost, so the gate was in scope
and I had not run it.

```
GATE PASS: worst seed falls/move 0.270 <= 0.40 over 12 seeds at 12000 frames
             min   median   max
falls/mv   0.207    0.214  0.270
```

Against the re-taken baseline the coordinator quotes — 0.208 / 0.225 / 0.334
— this run sits at or just under it on all three order statistics. **No
improvement is claimed from that**: `main` moved between the two readings
(#142, #145, #149 all landed in between), so it is not a paired comparison,
and the statistic is known not to settle across frame budgets. The claim is
only the one the gate makes: the shipped ant is comfortably inside the bar.

**Two limits on what this gate covers here.** It runs the *shipped* two-cell
ant — `forage_probe` takes `frames`, `seeds`, `climb`, `spacing` and `gate`,
and has **no body knob** — so it says nothing about falls at three, six or
nine cells. Since §4b finds no multi-cell colony survives anyway, adding one
would measure the fall rate of a population that is dying for other reasons;
the honest order is §R3 first, then a body knob on this harness, then the
gate per size. That is recorded in §6 rather than done.

### Motion removes the size axis, for the ants that are moving

Lane H (`creature-motion-decoys-2026-08-30.md`, PR #150) measured whether
motion changes `decoys` — the number `creature-appearance-design.md`'s
"extent must roughly quadruple" rests on. In ordinary weather a body that
**moves** has **0-2** competitors at *every* size from 1 to 16 cells, against
**141** at two cells and **15** at nine for a body that is **still**. A
walking two-cell ant is already better off than a stationary sixteen-cell one.

**It qualifies rather than demolishes**, and the split is what matters here:
**22-42% of ants never moved once** over a ~384-frame horizon, so for a
*resting* ant the static ladder stands unchanged and extent is still the only
lever. For a walking one, **nine cells buys nothing measurable** — while
costing 4.5x the body energy per hatch, about a third of the placement sites,
and an 8-10x blocked-movement rate.

**So the price this change sets is what decides whether extent is ever worth
choosing**, which makes the pricing work matter more rather than less. A
longer body being free is exactly the defect it was; what has changed is that
the benefit it was free in exchange for is now known to be small for any
animal that walks.

### What a bigger body does to the birth arithmetic

Lane A measured the **960-point body stamp** as the term blocking a breeding
colony. That term is `body_energy * cells`, so it scales with the body — and
the bank ceiling it is measured against does **not**, because the ceiling is
`hunger_fraction * start_energy + one mouthful` and `start_energy` is flat
(§3). Putting the two together:

| body | birth cost | bank ceiling | bank / bar |
|---|---|---|---|
| 2 cells — ships today | 1,040 | 460 | **0.44** |
| 6 cells | 2,960 | 460 | **0.16** |
| 9 cells | **4,400** | 460 | **0.10** |

The 1,040 and 2,960 are `creature_probe`'s own printed figures; 4,400 follows
from the same `birth_grant + body_energy * cells`.

**A bigger body therefore moves a colony strictly further from breeding, and
by a factor, not a margin.** A nine-cell ant is about **4.2x** further from
affording a child than the two-cell one — on top of §4b's finding that it does
not survive to try. This is not a new mechanism, it is Lane A's arithmetic
evaluated at a body size nobody had evaluated it at, and it is one more reason
the body-size card is held rather than posted.

## 6. What is owed before the owner should be asked to choose a body

The lane brief asked for a runtime selector and a blind A/B. **Both are held,
and the reason is §4b rather than the cost of building them.**

The appearance report's recommendation — *"ship a body of about nine cells
and keep the dark palette"* — rests on decoy counts measured over 600-frame
renders, which are sound. What §4b adds is that the same nine-cell body does
not hold a colony over 12,000 frames, and did not before this change either.
A blind A/B today would ask the owner to choose between one body that works
and four that go extinct, and would get an answer to a question nobody meant
to ask.

Three things are owed, in this order:

1. **Root-cause the multi-cell collapse.** Filed in
   `open-bugs-handoff.md`. The cannibalism hypothesis is cheap to test: make
   ant flesh inedible to ants and re-run `body=3`.
2. **Decide whether the tank scales with the body.** With `start_energy`
   flat, an *n*-cell animal's horizon is `1/n`. That is almost certainly
   wrong as biology — a bigger animal carries a bigger reserve — and it is
   the sharpest term this change introduces. Scaling it re-opens #142's bank
   ceiling arithmetic, which is why it was not done here rather than because
   it is not wanted. **A sublinear (Kleiber, `M^0.75`) burn is the other
   candidate**, and it has a property linear pricing does not: bigger animals
   become more famine-robust while still needing more mouthfuls per unit
   time, which is a graded trade with a middle rather than a ratchet in
   either direction. Neither was measured here and neither should be adopted
   on this paragraph.
3. **A `body=` knob on `forage_probe`, then the gate per body size.** The
   falls-per-move gate is the condition E11 travels with, and a bigger body
   changes the fall law (launch off cell count, descent off bounding box). It
   cannot usefully be read per size until §R3 is closed, because it would be
   measuring the fall rate of a population dying for unrelated reasons.
4. **Then the selector and the card.** The keyboard is genuinely full —
   `main.rs` says so at the `Comma` binding, *"every letter and digit is
   already bound and F9-F12 are owned by macOS"* — so the home for it is the
   tunables panel, which is where `TunableGroup::World` went for exactly this
   reason and says so in its own doc.

**Nothing here says an ant should stay two cells.** It says that the cheap
route E10 authorised does not yet produce an animal that lives, and that the
reason is upstream of both the pricing and the palette.

### The lever this report names and does not build

Lane H's closing point, recorded here because it is the first answer anyone
has had to the half of E5 that `creature-appearance-design.md` §7 answered
with a flat no.

**How often an animal moves is brain-side.** §7's finding is that the two
things deciding whether a creature is worth looking at — extent and palette —
are exactly the two an individual cannot own: `individual_as_species` copies
the parent's body verbatim, and the palette is one material keyed by species
name. `genome` and `traits` are all an individual owns. But if a *moving*
body has 0-2 decoys at every size while a still one has 141, then **how much
an animal moves is an appearance-adjacent property — and it is exactly the
kind of thing the genome already reaches.**

**Unmeasured and unauthorised, and deliberately not built here.** It is named
so the next session does not have to rediscover it, and so that E5's "how do
we do that by evolution rather than direct design" has one candidate answer on
the record instead of none.

### And one thing that is now owed to `creature-appearance-design.md` itself

Its §5 ledger note — *"4.5x the `body_energy` the ledger stamps at every
hatch — a real change to the colony's economy, not only its picture"* — is
correct and was written before anything could price the running cost. The
figures in its §2 and §5 tables were all measured on bodies that were free to
run. Nothing in that report needs retracting: `decoys`, `ink` and the blocked
rates are properties of a render and a walk, not of a metabolism. But its
recommendation — *"ship a body of about nine cells"* — now has three costs
attached that it could not have quoted: 4.5x the running bill, 4.2x further
from breeding (§5b), and a colony that does not survive 12,000 frames at any
bill (§4b).

## 7. Provenance

Everything measured on this branch, on one machine, in one session.
`creature_probe terrain=world seed=0xA17 frames=12000`, and the slab arms on
its hand-built floor. The baseline `ascii` binary was built from this
branch's own merge commit rather than carried from earlier — #142 found the
byte-identical control scene reading 14% apart across two binaries, so a
baseline from another build would not have been comparable.

**One horizon, one seed.** The extinction results are 12,000 frames at seed
`0xA17`; §4b's claim is that the *paired* arms agree, which is robust to the
seed in a way an absolute survival number is not. A seed sweep is owed before
any bar is set on these numbers, and none is set here.
