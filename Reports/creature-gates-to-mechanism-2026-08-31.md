# The authored gates come out, and four of the plan's own assumptions go with them

**Status:** built and landed 2026-08-31, PRs #190, #192, #194. Engine at
`main` carrying the rack (#193), the plant health overlay (#195) and the
`PLANT HEALTH` fix (#198).

The owner's objection, 2026-08-30, is the whole brief: *"this sounds like
we're forcing a system into creating behaviors that we want instead of
creating the most correct system and allowing behaviors to develop."* The
programme that answered it is `S0`–`S6` in the plan of record. This document
is not that plan restated — it is **what the plan turned out to be wrong
about**, which is the half no later session can reconstruct from the diff.

The governing line, adopted with the plan and still holding after everything
below: **the mechanism is code, the policy is genome.** Its two corollaries
did the actual work — *add senses and economies, never behaviours*, and *a
sense must not pre-categorise what it senses.*

---

## 1. What landed

Briefly, because the commits carry the detail.

| | |
|---|---|
| **S0** | Hauling and looking cost something. Load-dependent movement (`carried_cells`), a perception tax (`sight_fraction`). Three proposed genes stopped being ratchets. |
| **S1** | The crop. `hunger_fraction` and Gate 0's provisioning clause deleted; an animal takes what it finds and digests it as it walks. `BrainInput::Carrying` becomes crop *fill*, so the eat-vs-carry crossover lives in mutable bias weights. |
| **S2** | `Feed` against `Drop` through `choose_weighted`, so which verb fires is genome rather than statement order. |
| **S3** | A birth payable from food **within reach** — deliberately not from a nest, which would have hardcoded a colony. |
| **S4** | `TRAIT_REPRODUCE_AT`. `CREATURE_TRAITS` 2 → 3. |
| **S5a** | `predation_probe mode=range`. Measurement only. |

The lab bed also gained `LabBox::predators`, defaulting to 0.

---

## 2. Four things the plan asserted that measurement contradicted

**This is the section to read.** Each was written into the plan in good
faith, each was checked because the plan said to check it, and each was
wrong. Three of the four would have produced working code that expressed
nothing.

### 2a. `store_in_body` was specced as a gene and is redundant

The plan gave S4 two slots: `TRAIT_REPRODUCE_AT` and `TRAIT_STORE_IN_BODY`,
the latter to make the granary and replete forks *"corners of one space
rather than a design choice"*.

**S2 had already delivered exactly that**, and the plan could not see it
because S2 was specced before it was built. The verb choice is a
`choose_weighted` over two **brain output** weights, `Feed` and `Drop`. Those
are heritable, they mutate, and they are conditioned on everything the brain
senses — crop fill, food adjacency, both pheromone planes. They can express
*"put it down when laden and near home"* and *"never put it down"*, which
are the two forks by name.

A scalar trait beside them is a second knob on one quantity and a **strictly
weaker** one, because it is unconditioned. `CLAUDE.md`'s *when several knobs
move the same number, check what each one trades* settles it: this one trades
nothing the weight does not already trade. Slot dropped; `CREATURE_TRAITS`
went 2 → 3 rather than 2 → 4.

**The general shape, which is the reusable part:** a plan written across
several stages can specify in a late stage something an earlier stage
delivers by a different route. Before adding a lever, check whether the
stages already landed made one.

### 2b. `reproduce_at` does not need senescence to be two-sided

The plan: *"`reproduce_at` becomes two-sided only once S5e's age-linked
mortality lands, which is why that ordering is not optional."* The reasoning
was Cole's paradox — without an adult-survival term, waiting to breed is free
and the allele has one reachable end.

**Starvation already does that job.** `place_creature` tops a parent up to
`birth_cost + 1` and then charges `birth_cost`, so an animal that breeds the
instant it can afford to is left standing on **one joule** — about three
ticks of upkeep for the shipped ant. The low allele is very nearly a suicide
pact; the high one is a survival buffer bought by breeding less often. Both
ends are reachable today.

An age-linked hazard will *sharpen* that axis. It is not what makes it exist,
and S5e was not a precondition.

### 2c. The refuge already exists, and I nearly rebuilt it

Coming out of S5a's first result I recommended building encounter bias,
calling it *"the missing mechanism"*. It is not missing.
`a_wide_body_cannot_enter_a_one_cell_tunnel_that_a_chain_walks_through` says
so in as many words:

> *"The refuge, and there is no hiding code anywhere. An ant is a
> one-cell-wide following chain; a beetle is a 2x2 rigid block. A tunnel one
> cell tall admits the first and refuses the second, purely because a rigid
> body's passability check covers every cell of it."*

Sight is occluded by material on the same terms. What was missing was not
mechanism but a **gradient** — see §4.

**The cheap check that would have caught it earlier:** before building a
mechanism, grep the test names for the property. The test above is named for
the exact claim.

### 2d. A wrong-arity RON tuple is loud, not silent

The plan warned that widening `CREATURE_TRAITS` was dangerous because both
fields are `#[serde(default)]`, so *"a wrong-arity tuple **fails silently**
into the default"*, and specced guards against that.

Measured by narrowing `ant.ron` back to two: RON refuses a
present-but-malformed field outright, `SpeciesRegistry::builtin` panics, and
the message names the file position and both lengths —

```
317:36: Expected an array of length 3 but found 2 elements instead
```

`#[serde(default)]` fires only for a field that is **absent**. So the silent
case is a *misspelling*; widening a slot is the loud one. The guards were
re-aimed at the failure that can actually happen: a `.ron` updated to the
right arity with a value dropped or shifted one place in the edit, which
parses fine and quietly gives one gene another's setting.

---

## 3. The lab bed's colony crash is overgrazing, and there is a control

The sealed bed runs **52 ants → 12** over 24,000 frames. It was tempting to
read that as a mis-priced economy, and the costs were flagged as owed
re-derivation before S4. **They are not the lever**, and two independent
measurements say so.

**The arithmetic.** A two-cell ant pays, per decision: idle `0.05 × 2` =
0.10, move `0.125 × 2` = 0.25 (0.63 with a full 1,440 crop), synapse ~0.01.
Against `digest_rate 3.3 × quality` ≈ 2–3 while it has food. Income is 5–9×
outgo. The lab page states the same thing from the other side —
`ANT NEEDS 344 STEPS OF FEEDING PER CHILD`, which is 2,064 frames out of
24,000, a **9% duty cycle**.

**The control.** Eight founders are sown across the bed and the colony is
founded at the midpoint. At frame 3,000:

| founder column | 60 | 116 | 172 | 228 | *256 = nest* | 284 | 340 | 396 | 452 |
|---|---|---|---|---|---|---|---|---|---|
| cells, with ants | 99 | 69 | **dead** | **dead** | | **dead** | **dead** | 99 | 54 |
| cells, `colonies=0` | 105 | 69 | 58 | 89 | | 36 | 81 | 102 | 54 |

Monotone in distance from the nest, and **the four furthest are untouched to
the cell** (99 vs 105, 69 vs 69, 54 vs 54). Fifty-two ants against four
one-cell seedlings is not a contest; once those are gone the survivors are
~140 columns away, past what a laden ant carries. By 24,000 frames the box
holds 2,033 plant cells the colony cannot reach.

Reproduce: `labshot colonies=0 frames=3000` against the default, and read the
`founders (cells):` line.

**So the bed is answering a harder question than it looks like it is asking**,
and the remedy is spatial rather than economic. The knob exists —
`compartments`, and now `predators` beside it.

---

## 4. S5a: predation selects neither ranging nor sheltering

`predation_probe mode=range`, **twelve seeds**, 6,000 frames, sampled every
30, paired against the same seeds at `beetles=0`. `lift` = share of deaths in
a class ÷ share of ant-samples in it, so **1.00 is mortality flat in that
variable and selecting nothing**. 190 and 231 deaths; coverage against the
engine's independent death counter 0.99 and 1.00.

**Distance from the nest** — mortality is biased *toward home* in both arms:

| band | no predator | beetles |
|---|---|---|
| ≤0 (on the nest) | 2.15 | 1.77 |
| ≤8 | 1.73 | 1.97 |
| ≤24 | 1.23 | 1.37 |
| ≤56 | 0.63 | 0.78 |
| ≤120 | 0.58 | 0.50 |
| beyond 120 | 0.50 | 0.76 |

**Shelter** — two tests, because the loose one is not the one the engine
implements:

| | no predator | beetles | advantage |
|---|---|---|---|
| roofed vs open | 0.59 / 1.36 | 0.54 / 1.41 | 2.31× → 2.61× |
| **predator cannot fit** vs could reach | 0.43 / 1.22 | 0.56 / 1.18 | 2.84× → 2.11× |

**The two shelter tables disagree about the sign of the beetle term.** That
is the finding rather than a defect: at 231 deaths, predation's contribution
to either gradient is inside the noise, while the gradients themselves are
unmistakable and present with no predator in the world.

**What it means for the owner's hoped-for outcome** — *ants digging homes to
keep themselves safe.* Shelter pays enormously. Predators are not what makes
it pay. **A behaviour cannot be selected for as safety from predators while
predators do not measurably set its value**, however heritable the `Dig`
weight is. So the plan's ordering inverts: giving the beetle a breeding rate
first would have added mortality to a world where mortality selects nothing.

**Two caveats, both live.** `build_scene` places beetles at `40 + i*45` and
the nest band is columns 16–90, so **two of nine start inside it** — the
near-band lifts want a placement sweep before they carry weight; the far
bands are clean, since no beetle starts past 400. And "roofed" asks only
whether solid material stands over the head, so a canopy answers yes and a
2×2 beetle walks under it; that is why the predator-fit row exists beside it.

---

## 5. A method finding: four seeds lied, and lied *tidily*

This section is here because it nearly published a wrong headline, and
because the failure has a tell.

S5a was first run on four seeds and gave a clean, quotable result: the far
band read **0.47 without predators against 0.48 with them** — predation
adding exactly nothing — and the shelter tables agreed it took nothing away.
A whole conclusion was written on it and put in a PR.

Twelve seeds moved every beetle-term number, one past its own sign:

| | 4 seeds | 12 seeds |
|---|---|---|
| far band 120+ | 0.47 → 0.48 | 0.50 → **0.76** |
| roofed advantage | 2.73× / 2.56× | 2.31× / **2.61×** |
| tight advantage | 5.1× / 2.1× | 2.84× / 2.11× |

83 deaths against 231. The small sample was not *wrong*, it was
**unrepresentative** — and it was **tidier than the truth**, which is
`CLAUDE.md`'s named tell for an artifact when no control is to hand. The
claim it supported, *"predation selects nothing"*, was too strong and is
withdrawn; what survives is the weaker and better-supported statement in §4.

`CLAUDE.md` already says *six seeds is not a sweep*. This is that rule's
own case, measured. The doc comments in `predation_probe` now carry both
numbers and refuse anything under twelve seeds, so the next reader cannot
repeat the four-seed quote.

---

## 6. Corrections to earlier reports

**`larder-reachability-2026-08-30.md`.** Two of its findings are superseded
by mechanism rather than by re-measurement:

- *"`creature::try_bud` charges `state.energy` and there is no second term,
  so a granary of any size funds zero births."* **S3 added the second term.**
  A birth is payable from edible cells in the head's neighbourhood, charged
  through the ledger exactly as an eaten cell is. The blocking fact the
  report names is gone.
- *"the colony is the sink"* — measured as the colony taking a paired 10
  cells off a granary it did not build. **S2 found the cause and it is not a
  behaviour.** `act` checked the ingest branch before the drop branch, gated
  only on crop room, so an ant standing at its own nest re-took what it had
  just delivered — a consequence of statement order with no weight of any
  genome involved. The measurement was sound; its attribution was to a
  choice the animals were never making.
- The report exists to decide whether `store_in_body`'s granary end is
  reachable. **The gene was dropped** (§2a), so the question it answers is
  moot — but its §8a, the five-trees-under-a-moving-`main` study, is
  untouched by any of this and remains the reusable part.

**`creature-gate0-births-2026-08-30.md`.** Its shipped mechanism — *"an
animal short of a child's price now finishes the meal"* — **is the Gate 0
provisioning clause S1 deleted.** Its measurements stand as a record of what
that gate did; its mechanism is no longer in the engine. Its open bug (1,651
pickups, 4 deliveries, an empty larder) is superseded by §3 above, which
attributes the lab bed's failure to overgrazing rather than to the drop rule.

**The withdrawn `births-denied-no-space` finding**, recorded so it is not
re-found: it read 11,442 at 2-cell ant pitch and **29** at 4-cell, which
looks like a strong result and is a scene artifact of a probe packing 55
two-cell ants into each other's birth neighbourhoods. Separately, the
counter is incremented in `try_bud`, which runs **every creature tick**, so
it counts *calls* rather than distinct denied births: one boxed-in animal
retrying for 63 ticks reads 63. Both halves have to be known before the
number means anything.

---

## 7. What is owed, and what not to re-derive

**Do not re-derive:**

- the ant's metabolic costs against the crop economy — §3 has the arithmetic
  and two controls saying they are not the binding term;
- whether the refuge mechanism exists — §2c, it does, and it is tested;
- whether `store_in_body` needs a slot — §2a, it does not;
- whether `reproduce_at` needs senescence first — §2b, it does not.

**Owed, in order:**

1. **The beetle placement sweep** (§4's first caveat). It is the one
   confound left standing on the S5a numbers, and it is an argument change
   rather than a mechanism change.
2. **Is any ant ever in a space a predator could not enter?** The
   `predator_could_stand` row answers this per sample; what is not yet
   measured is whether ants *arrive* there by digging or merely by walking
   under terrain that was already there. That splits the outcome: if the
   refuge is never dug, the constraint is digging; if it is dug and predation
   still does not discriminate, the constraint is the beetle's hunting.
3. **S5c/S5d/S5e** — all three assume predation has teeth, and §4 says it
   does not yet. S5d should read
   `creature-vision-sizing-2026-08-30.md` rather than re-deriving what
   vision costs.

**The success criterion is still not met and should not be quietly
restated.** The plan's own bar is the lab bed's colony sustaining itself —
`live` non-zero and not falling at 24,000 frames, without hand-fed food. It
runs 52 → 12. What changed is that the reason is now known and is spatial
(§3), rather than unknown and presumed economic.
