# The birth grant, the starvation cut, and the term neither of them touches

**Status: built and landed.** `birth_grant` is a heritable slot,
`start_energy` is cut, `deaths` is a live counter for the first time, and
the shipped ant still does not breed — **which is arithmetic rather than a
shortfall, and is the finding this report exists for.**

> **§6 is superseded, and §2's arithmetic is not.** Later on 2026-08-30 the
> shipped ant bred, and none of the three routes §6 lists was needed:
> `Reports/creature-gate0-births-2026-08-30.md`. Everything §2 derives is
> confirmed exactly — the roof really is `hunger_fraction * start_energy` plus
> **one** mouthful — but the "one" turned out to be a property of the *feeding
> rule* rather than of the animal. An ant short of a child's price now keeps
> eating, out on the route when the mouthful pays for the child alone and at
> the nest for as long as the larder lasts, so the roof is the colony's stores
> instead. Read §2 as the measurement it is and §6 as the conclusion that was
> drawn from it before anyone questioned the rule underneath.

Lane A of the 2026-08-30 creature program. Files:
`src/sim/organism.rs`, `src/sim/creature.rs`, `assets/species/*.ron`,
`examples/creature_probe.rs`, `wiki/ants.md`.

---

## 0. Findings, in the order they change a decision

1. **`birth_grant` + E14 cannot make the shipped ant breed, and no setting
   of either closes it.** The lane brief describes them as "two authorised,
   unbuilt things [that] close that gap". They do not. The binding term is
   the **body stamp** — `body_energy * cells` = 960 for a two-cell ant — and
   it is invariant to both changes. Measured, not argued: §2.
2. **Cutting `start_energy` makes reproduction *harder*, not easier**, and
   the direction is the opposite of the brief's premise. The bank ceiling is
   `hunger_fraction * start_energy + one mouthful`, so it falls with the
   budget while the 960 stamp does not move at all. Bank-over-bar goes
   **0.30 at 900 → 0.21 at 200 → 0.16 at 90**. E14 is still worth doing, for
   an entirely different reason (finding 4).
3. **`ant.ron`'s own comment had this backwards** and has been corrected. It
   read: *"it is why cutting the grant to shorten the horizon does not also
   make breeding harder."* It does make it harder, by lowering the ceiling.
   The half it got right — "the stamp is the expensive half, and it does not
   move when the grant does" — is the whole finding, one step short.
4. **What E14 actually buys is a colony that finds a level instead of
   running down.** The headline everyone including this report first wrote
   is that `deaths` "read 0 everywhere and is now live". **That is wrong,
   and the truth is better** — see §4a. The pre-cut ant does die; it dies at
   36,000 frames and later, past any horizon a harness runs, and it then
   keeps dying. Post-cut the colony loses 16 of 45 inside the first 12,000
   frames and then very nearly stops. E14 converts an unbounded terminal
   run-down into an **early cull that stabilises**, and it is the first real
   carrion in the world at a horizon anyone will see it — §2.5's reason the
   diet curve has one hump.
5. **`birth_grant` is real, connected, and free for anything that does not
   breed.** It moves the shipped ant's birth cost 1,860 → 1,040, and on a
   config where births *are* reachable the cost tracks the allele exactly
   (60 / 120 / 180 / 240 at grants 0.1 / 0.4 / 0.7 / 1.0). A species that
   does not reproduce is **byte-identical** across the change: §4, control C.
6. **Closing the gap needs the stamp term.** Two candidates, both already
   written up and neither authorised here: a child **born at one cell** that
   grows into its plan (economics §3.1 — halves the stamp to 480 and opens a
   ≤ 90 budget for the endowment at the shipped gut), or **fission**, where
   the stamp is *moved* rather than bought (§3.2 — stamp zero). A
   sufficiently specialised gut is the third route and the only one needing
   no new mechanism: at `gut_bias = -1.0` a leaf pays 480 instead of 120.

---

## 1. What was built

**`TRAIT_BIRTH_GRANT`, slot 1 of `CREATURE_TRAITS`** (`organism.rs`). Where
on the shared `-1..=1` axis a lineage provisions its young, read as a
fraction of `start_energy` by `creature::grant_fraction`. Ancestral `-0.2`
on the ant line — a **0.4 grant** — per the economics report's §5.1
recommendation of 0.3–0.5, with `trait_variance` 0.15 in line with the gut's.

The point is that `start_energy` was doing two jobs: the founder's budget
*and* every bud's endowment. §2.6's anti-freeloading rule — *the bar must
exceed what a newborn is given* — was only expressible as "must exceed
`start_energy`" because those were the same number by accident. They are not
the same number now.

Three details worth keeping:

* **The grant is read off the *parent*, not the child.** The provisioning
  decision belongs to the animal paying for it, which is the biology
  (Smith–Fretwell) and also what the existing draw order already did:
  `try_bud` mutates the child's copy *after* placement, so the child's own
  allele governs what it will give to *its* children.
* **`reproduce_at`'s no-suicide guarantee is kept total under a heritable
  grant.** The species floor is computed from the *ancestral* allele; an
  individual whose grant has drifted upward owes more than that, so
  `try_bud` takes `threshold.max(cost + 1.0)` against its own cost. Without
  that, a drifted parent is charged past its bank and the birth kills it —
  exactly what the floor exists to make impossible.
* **The serde default is `+1.0`, not `0.0`.** Zero on this axis is a *half*
  grant, so a plain `Default` would have silently halved the endowment of
  every species that authors no `traits` line, on the day the slot landed.

**E14, the horizon cut** (`ant.ron`). `start_energy: 900 → 200`, and the
number is derived rather than chosen: an idle life is
`start_energy / idle_cost` ticks of `tick_interval` frames, so 200 gives
`200 / 0.10 * 6` = **12,000 frames, exactly one scene horizon**. At 900 it
was 54,000 — a scene saw 22% of a lifetime, and foraging perfectly and never
eating at all came out the same inside any run a harness could afford.

---

## 2. The arithmetic, and why the package cannot close it

An ant banks food only while `energy < hunger_fraction * start_energy`;
above that line it carries the mouthful home instead of eating it. So the
most it can ever hold is the hunger line plus one mouthful:

```
ceiling   =  hunger_fraction * start_energy  +  Y
bar       =  birth_cost + 1  =  grant + body_energy * cells + 1
```

`Y` is what one cell yields **to this gut**: `diet_yield`'s matched filter
pays `food_value * (1 - |gut_bias - food_class| / 2)^2`, and the shipped ant
is neutral, so a 480 leaf pays it **120**.

**The ceiling model is not assumed — it was measured across the cut**, and
this is the positive control on the model itself. Predicted against
`richest bank`, `creature_probe terrain=world frames=24000`:

| `start_energy` | predicted `0.5·S + 120` | measured `richest bank` |
|---|---|---|
| 900 | 570 | **567** |
| 450 | 345 | **344** |
| 300 | 270 | **260** |
| 200 | 220 | **219** |
| 90 | 165 | **164** |

Now put the bar beside it. `body_energy * cells` = 480 × 2 = **960**, and
nothing in this package moves that term:

| `start_energy` | ceiling | bar at grant 0.4 | bank / bar |
|---|---|---|---|
| 900 | 567 | 1,321 | 0.43 |
| 200 | 219 | 1,041 | **0.21** |
| 90 | 164 | 997 | **0.16** |

**Even at a grant of zero the bar is 961, and the ceiling at the shipped
budget is 567.** The endowment could be free — the child conjured out of
nothing — and the ant still could not pay. Cutting the budget lowers the
ceiling by `0.5 ΔS` and the bar by only `0.4 ΔS`, so every cut makes the
ratio worse. That is finding 2, and it is why the two changes are worth
doing and are not a fix.

**What a closure would require**, holding `hunger_fraction` at 0.5 (raising
it is a recorded dead end, deliveries 1,733 → 3) and `body_energy` at 480
(pinned to the flesh pricing invariant against the `ant`, `chitin_*` and
`corpse` materials, and not changeable from a species file):

```
grant + 960 + 1  ≤  0.5 * S + Y
```

At `Y = 120` and `grant = 0` this needs **S ≥ 1,682** — nearly double the
budget E14 was authorised to *cut*. The two authorised levers pull the wrong
way on the one term that binds.

---

## 3. Which constants were re-derived, and which were deliberately not

E14 is a shared-budget reallocation, so every constant calibrated against
the old budget is named here with its verdict rather than inherited.

| constant | value | rescales with `start_energy`? | verdict |
|---|---|---|---|
| `synapse_fraction` | 2.222e-6 | **yes, by construction** | **left alone, correctly.** It is authored as a fraction *of this budget* precisely so a cut needs no correction — that is what §13j bought, after an absolute 0.002 spent 80% of a 90-point life on thinking and invalidated a three-knob sweep. Now 4.44e-4 per synapse per tick; every ratio unchanged. |
| `hunger_fraction` | 0.5 | yes | left alone. A fraction, so it rescales; and raising it is a recorded dead end. |
| `idle_cost` | 0.10 | no | **left alone deliberately.** It is the horizon knob's *other operand* — re-deriving it would put the 54,000-frame lifetime back and undo E14. |
| `move_cost` | 0.25 | no | left alone, and its 2.5:1 ratio to idle is the researched worm figure, which the cut preserves. |
| `body_energy` | 480 | no | **cannot be changed from this file.** It is bound to the flesh pricing invariant — `ant`, `chitin_*` and `corpse` all carry `food_energy: 480` against it — and breaking that re-opens §13l's corpse pump. This is the term that binds, and it is out of this lane's reach. |
| `reproduce_threshold` | 2000 → **1100** | no | **re-derived.** 2000 described itself as clearing `start_energy` "by more than a factor of two"; at a budget of 200 that would have been a factor of ten and meant nothing. The birth cost is now 80 + 960 = 1,040, so `reproduce_at` floors at 1,041; 1100 sits just above the floor, so the authored number is a statement rather than a value the floor silently replaces. |

**Do not tune `reproduce_threshold` to "fix" this.** `reproduce_at` floors
it at `birth_cost + 1` whatever the file says, so an edit downward does
nothing at all — and reads exactly like the change having been made.

**Two test constants were also calibrated against an immortal ant**, and
both went red on the cut. Neither was re-fitted; both were replaced by the
property they were standing in for, so the next retune does not take them
red again:

* `the_standing_meat_never_exceeds_what_was_put_into_it` took its
  ledger baseline at a hardcoded 10,000 frames because "the first death is
  around frame 12,000". The cut moved the first death to ~4,250 and put 9
  deaths inside the baseline window. It now **searches** for the last sample
  before any death (found: 4,000 frames) and asserts it had a window at all.
* `a_lone_grazer_cannot_farm_a_moss_lawn_forever` ran both arms for a literal
  60,000 frames — about 1.1 idle lifetimes at 900, and **five** at 200 — so
  its *control* arm starved and the setup assertion reported "this scene
  cannot feed anything" about a scene that was fine. The horizon is now
  computed from the animal's own budget as 1.1 idle lifetimes.

---

## 4. The controls, all four run

**Control A — the model is sensitive at all.** Economics §7's E1, the arm
that says §§1–5 are void if it comes back zero:

```
creature_probe start_energy=200 body_energy=20 threshold=241 hunger=0.9 terrain=world frames=24000
  -> births 1875, deaths 1620, richest bank 616 against a birth cost of 240
```

The ceiling model permits it and it happens. A null would have voided the
arithmetic in §2; it is not a null.

**Control B — `deaths` is live inside a usable horizon, and it is the cut
that put it there.** The shipped colony, before and after, then with the
budget restored, all at 24,000 frames:

| | `deaths` | `live` | `births` | `richest bank` | bar |
|---|---|---|---|---|---|
| before (900) | **0** | 45 | 0 | 567 | 1,860 |
| after (200) | **17** | 28 | 0 | 219 | 1,040 |
| after, budget restored to 900 | **0** | 45 | 0 | 567 | 1,320 |

The counter goes non-zero with the cut and **back to exactly zero** with it
undone — at this horizon, which §4a is about. And across the cut the colony is thinned at every setting and wiped
out at none — deaths 26 / 28 / 17 / 11 / 7 leaving 19 / 17 / 28 / 34 / 38 of
45 founders alive at budgets 90 / 150 / 200 / 300 / 450 — which is the graded
outcome the house ethos asks for rather than a binary.

**Control C — `birth_grant` is free for anything that does not breed.** The
restored-budget row above is **byte-identical to the pre-change baseline on
every counter** (moves 51,578, blocked 5,020, falls 10,125, eats 200,
pickups 4,992, digs 65, drops 4,919, deliveries 1,874, nest-visits 16,468).
So the entire behavioural delta on the shipped ant is E14's cut, and the new
slot costs a non-reproducing species nothing.

**Control D — `birth_grant` moves the outcome where births are reachable.**
Same gate-control scene, sweeping the allele:

| grant | birth cost | births | deaths | deepest generation |
|---|---|---|---|---|
| 0.1 | **60** | 3,906 | 2,736 | 48 |
| 0.4 | **120** | 3,930 | 3,725 | 65 |
| 0.7 | **180** | 7,176 | 6,811 | 104 |
| 1.0 | **240** | 5,517 | 5,090 | 77 |

The cost tracks the allele exactly. **Read the births column as sensitivity
and nothing more**: `denied-no-space` runs 159k–563k across these rows, so
this scene is space-limited rather than energy-limited, and `hunger=0.9` is
deliberately in dead-end territory — economics §7 flags this arm as a
control, not a proposal. It says the gene is connected and moves the world
hard. It does not say which direction selection favours, and no run here
answers that.

### 4a. The correction: "deaths read 0 everywhere" is false, and the real result is better

**Every earlier statement of E14's justification — the decision record's, the
lane brief's, and this report's own first draft — says `deaths` reads 0
everywhere.** It does not. It reads 0 at the horizons a harness runs, which
is a different and much narrower claim, and the difference was found only
because a `filmstrip` render for the review card came back with **11 deaths
on the pre-change binary** where the story said there should be none.

Deaths against horizon, `creature_probe terrain=world`, both budgets:

| frames | at 900 (before) | at 200 (after) |
|---|---|---|
| 12,000 | **0** | **16** |
| 24,000 | **0** | 17 |
| 36,000 | 14 | 20 |
| 54,000 | 34 | **22** |

Read the columns, not the first row. **The pre-cut colony does not fail to
die — it dies late and then keeps dying**, 0 → 14 → 34 and climbing, because
every ant is slowly spending down a grant it can never fully replace inside
its satiety ceiling. **The post-cut colony takes its losses at once and then
very nearly stops**: 16 by frame 12,000, and only 6 more over the next
42,000. By 54,000 frames the *cut* colony has lost **fewer** ants than the
uncut one.

That is a better outcome than the one E14 was authorised on, and a different
one. It is not "ants can now starve" — they always could. It is that
mortality now selects: the ants that die are the ones that failed to feed
themselves early, the survivors are the ones that did, and the population
settles at a level the world supports instead of drifting to zero on a
timescale nobody watches. **A graded outcome with a middle**, which is what
the house ethos asks for and what the "0 everywhere" framing could not have
predicted.

**The methodological lesson is this file's own rule arriving again**: ask
what a counter reads when nothing is wrong. `deaths 0` was measured at one
horizon, believed as a property of the engine, written into a decision
record, and repeated by two documents and this report — and one render at a
longer horizon overturned it.

**And the guards were checked for blindness, not just for green.** The
grazer horizon shrank 5x, which is exactly the shape that makes a guard stop
being able to fail, so the fault it is named for was put back: moss
`food_energy` × 40 takes it red at **21.033** against the 1.0 pump line. It
is not blind at the shorter horizon.

---

## 5. Frame cost at a breeding population

Creatures were measured free at 55 ants, and a breeding population is not 55.

**All three rows on one binary**, which the fourth row is the reason for:

| scene | peak population | mean frame |
|---|---|---|
| control scene (budget restored to 900) | 45 | 2.98 ms |
| shipped colony, after the cut | 45 | 3.17 ms |
| gate control, breeding | **1,781** | **4.30 ms** |
| *the same control scene on the pre-change binary* | *45* | *2.61 ms* |

So a breeding population of 1,781 costs **+1.3 ms** over the 45-ant colony
on the same build — real, and not near a budget.

**That fourth row is why the first three had to be re-measured rather than
compared against this morning's numbers.** It is the *byte-identical* scene
— every counter matches the pre-change run exactly, per control C — and it
reads **14% apart** across the two binaries. Nothing in the simulation
changed; the machine did. Had the 2.61 been carried forward as the baseline,
the cut would have "cost" 22% and the whole comparison would have been
measuring the box.

**And quote the mean, not the worst.** The worst frame reads 65.6 ms and is
not pinned by any aggregate — mean × frames is 103,296 ms against it, so it
is an order statistic over many similar frames rather than one rare
expensive event, which makes it noise wearing a number.

---

## 5a. One thing this change does not deliver: you cannot see it

**The ethos bar is that an event with no visible consequence is not
finished**, and by that bar E14 is not finished. Twelve ants starve in the
review card's run and the food census shows live ants standing next to the
bodies and eating them — and **the corpses are not findable in the picture.**

The mechanism is a collision of two things that are each individually right.
A corpse is shaded by its worth, dark for a starved one and pale for a fresh
kill, which is correct and useful. But a *starvation* death is by
construction the poorest corpse there is, so it renders at the darkest end of
the ramp — against ants, which are dark, on soil, which is dark. The one
death the world can now produce for itself is the one death it draws least
legibly.

`render.rs` is not this lane's file and this is not a bug in it; it is an
interaction nobody could have seen before there were any starvation deaths to
look at. It went to the owner's review queue
(`20260830T031945607Z-7e0999`, board `creatures`) as a question rather than a
guess, because "make starved corpses lighter" trades directly against the
worth ramp meaning what it says.

**The owner answered, and the answer is sharper than the question.** In full:

> *"visually, I cannot tell anything from these. ants are mostly visible with
> there motion"*

So the finding is confirmed — and the **mechanism** is not the one this
section first proposed. The problem is not primarily that a starved corpse is
dark against dark soil. It is that **an ant is legible by moving, and dying is
exactly the thing that stops it moving.** A corpse is invisible *because* it
has stopped being the thing that made an ant findable in the first place.

Two consequences, and the second is the one that changes what a fix has to be:

* **A still contact sheet cannot answer any creature-legibility question**,
  and this report's first card was the wrong instrument rather than a
  well-posed question with a bad answer. Motion is the channel a creature is
  read on, so a GIF is the minimum, and even that cannot help *here* —
  see below. Recorded in `Reports/instruments.md`, because it generalises
  past this lane to anything that renders creatures for judgement.
* **A corpse cannot be made findable by motion, by definition**, so no
  camera, horizon or sampling rate fixes this — it can only be fixed by
  making a corpse differ in *colour* from a live ant. That is a real
  argument for a render change and it is not this lane's to make: it
  trades against the worth ramp, which is `render.rs`'s and the owner's.
  **What is settled is that the ramp alone cannot carry it**, because the
  poorest corpse in the world is the one this feature produces and the ramp
  draws it at the end nearest the animal it came from.

---

## 6. What is still owed, and to whom

The gap is the stamp. Three routes, in ascending cost, none of them
authorised here:

1. **Specialise the gut.** No new mechanism at all: at `gut_bias = -1.0` a
   leaf pays 480 instead of 120, and `0.5·900 + 480 = 930` clears the 961
   bar to within 31 — a knife-edge, but the only route that is purely a
   change to authored data. This is why E5 already asks S6's ancestor to be
   "a new solitary-grazer ancestor" rather than the ant, and §2 is the
   arithmetic reason that instruction was right.
2. **Born at one cell, growing into the body plan** (economics §3.1). Halves
   the stamp to 480 and leaves a ≤ 90 budget for the endowment at the
   shipped gut. Needs a growth step that charges `body_energy` at the moment
   it appends; the ledger owes nothing new. Costs appearance — a one-cell
   creature is at the bottom of the findable range.
3. **Fission** (§3.2). Stamp *moved* rather than bought, so the stamp term
   goes to zero and the bar is the grant alone. Cheapest arithmetically,
   same growth prerequisite.

**`birth_grant` is a prerequisite for all three and is now in.** It is also
the gene whose trade-off only exists because of E14: without starvation, a
low grant is strictly better and the allele goes to its floor on the first
generation. The two changes really were one package — just not the package
that makes the shipped ant breed.

**One prediction worth using as the guard when selection is measured.**
Smith–Fretwell says the optimum offspring size is *independent of parental
resources*, so enriching the world should move the **number** of births and
leave the mean `birth_grant` alone. A run in which both move together says
the model as built is not the model as described.

---

## 7. Provenance

Everything above is `creature_probe` on `terrain=world frames=24000` at the
default seed unless stated, measured 2026-08-30 on one machine in one
session, with the release examples rebuilt between every setting — species
files are `include_str!`'d and a stale binary produces bit-identical "runs".

**Re-taken after the plant lane's tree-breaking work landed on `main`
underneath this branch**, because a world where trees now come apart is a
different world for an ant to forage in, and a number measured on a tree
nobody else has does not transfer. It transferred:

| | before the merge | after |
|---|---|---|
| shipped ant: `deaths` | 17 | **17** |
| shipped ant: `live` | 28 | **28** |
| shipped ant: `richest bank` | 219 | 217 |
| birth cost | 1,040 | **1,040** |
| control (budget 900): `deaths` | 0 | **0** |
| control: `richest bank` | 567 | 568 |

**The world did change** — `moves` went 31,035 → 29,328 and `deliveries`
1,991 → 2,156, so the ants are living somewhere measurably different — and
every number this report rests on is unmoved or moved by two points. That is
a stronger staleness check than a repeat on one tree would have been.

Reads on: `Reports/creature-reproduction-economics.md` §§1, 3.1, 3.6, 5.1
(the arithmetic this confirms and, in one place, corrects the direction of);
`Reports/creature-evolution-plan.md` §0 E12/E14 and §2.6.
