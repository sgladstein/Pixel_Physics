# Three ways past the body stamp, priced

*Lane I of the creature program, 2026-08-30. A decision document: the three
routes named in `creature-reproduction-economics.md` §§3.1, 3.2 and 3.5, each
with a number against it, and a recommendation.*

**Read §1 and §5 if you read nothing else.** §1 is the correction that
changes which routes are still live; §5 is what to build.

---

## 0. The finding, in one paragraph

An ant cannot afford a child because a birth has to *buy the child's body* —
`body_energy` for each of its two cells, **960**, against a bank that stops
filling at **100**. Three ways past that were named and none was priced.
Priced, **the shipped budget has moved the goalposts since they were
written**: `creature-reproduction-economics.md` §3 does its arithmetic
against a satiety line of **450**, and E14 has since cut `start_energy` from
900 to 200, which puts that line at **100**. Two of the three routes were
judged on the old number. Re-priced against the tree that actually ships:
**route 3 (a specialised gut) does not close, and cannot** — it falls short
by 381, not by the 31 the earlier arithmetic reports. **Route 1 (born at one
cell) no longer closes on its own either**, though it did at the old budget.
**Route 2 (fission) closes, by the widest margin of the three, and is the
only one that closes without also forcing the diet gene.** The
recommendation is route 2, and §5 says what it costs.

---

## 1. The correction: the routes were priced against a budget that no longer ships

An ant's bank has a hard ceiling. It stops eating above
`hunger_fraction * start_energy` and carries the mouthful home instead, so
the most it can ever hold is **the satiety line plus one mouthful**:

```
ceiling  =  hunger_fraction * start_energy  +  Y
bar      =  max(reproduce_threshold, g + body_energy * cells + 1)
```

where `g` is what the newborn is *given* (`birth_grant`) and `Y` is the best
mouthful this gut can digest. That model is not new — Lane A verified it
against measurement, predicting 165/195/220/270/570 for a measured
`richest bank` of 164/175/219/260/567 — and this report uses it only to
place the routes, never as a verdict. §3 measures.

**What has changed is one operand.** `creature-reproduction-economics.md`
§3 writes the satiety line as **450**, i.e. `0.5 * 900`. Between that report
and this one, E14 shipped and cut `start_energy` to **200**. `ant.ron`
today:

| | |
|---|---|
| `start_energy` | **200** (was 900) |
| `hunger_fraction` | 0.5 → satiety line **100** (was 450) |
| `body_energy` x 2 cells | **960**, unchanged |
| `birth_grant` trait −0.2 → fraction 0.4 | g = **80** (was 360) |
| `reproduce_threshold` | 1100 |
| `birth_cost` | 80 + 960 = 1040, floored bar **1100** |

**Every route's headroom fell by 350 and its stamp did not move.** That is
the whole of this correction, and it is enough to reverse two verdicts.

### 1.1 `Y` is not what the material table says it is

The other operand is `Y`, and it has a trap in it that this lane's harness
exists to remove. `creature_probe` prices the best mouthful over **the whole
material table**, which contains a `flower` worth 1,440 and a `fruit` worth
960. Its own comment says a bound read that way "can rule out and can never
rule in". Route 3 is exactly the case where that matters, because a matched
herbivore gut collects a flower's whole 1,440 and would clear every bar in
this document on it.

**Measured, censusing the cells actually standing in the world** (18 seeds,
before and after 24,000 frames, `examples/stamp_probe.rs`):

| gut | best mouthful on the whole table | best mouthful **standing in this world** |
|---|---|---|
| 0.0 (shipped) | 360 | **120** — a leaf, at quarter value |
| −1.0 (matched herbivore) | 1,440 | **480** — a leaf, at full value |

**No `fruit`, `flower` or `windfall` cell exists in any of these worlds, at
frame 600 or at frame 24,000.** The standing food is `leaf`, `moss`,
`litter` and ant flesh, every one of them face value 480. So the route-3
escape hatch — reproduction riding a fruit crop, which §3.5 calls "already
in the engine with nothing to build" — **is not in the world the ants are
standing in**, and the 1,440 that makes route 3 look comfortable is a number
about the material registry rather than about this game.

This is the difference between a ceiling of 1,540 and a ceiling of 580 for
the same animal, and it is the single number that decides route 3.

### 1.2 The arithmetic, all six arms

Satiety line 100 unless stated. "Short by" is `bar − ceiling`; a negative
number is headroom.

| arm | what it models | stamp | g | bar | Y | ceiling | short by |
|---|---|---|---|---|---|---|---|
| **S** | the shipped ant, untouched | 960 | 80 | 1,100 | 120 | 220 | **+880** |
| **R3** | route 3 alone, best case (`gut=−1`, `g=0`) | 960 | 0 | 961 | 480 | 580 | **+381** |
| **R3b** | route 3 at the *pre-E14* budget (`start_energy=900`) | 960 | 0 | 961 | 480 | 930 | **+31** |
| **R1** | route 1's stamp (born at one cell → 480), `g=0` | 480 | 0 | 481 | 120 | 220 | **+261** |
| **R13** | routes 1 and 3 together, at the *shipped* grant | 480 | 80 | 561 | 480 | 580 | **−19** |
| **R2** | route 2's stamp (moved, not bought → 0), shipped grant | 0 | 80 | 81 | 120 | 220 | **−139** |

Three things to read off it before any measurement:

- **R3b is the brief's own number** — "clears the 961 bar to within 31" — and
  it is right *at a budget of 900*. At the shipped 200 the same route is
  short by 381, twelve times as far.
- **R1 was a route and E14 closed it.** §3.1's table gives route 1 a budget
  of "≤ 90" for `g` at the shipped gut; that is `570 − 480` at the old
  satiety line. At 100 the budget is **−260**: there is no grant, including
  nothing at all, that makes a one-cell newborn affordable to a neutral gut.
- **R13 clears by 19 out of 580 — three percent.** That is a knife-edge, and
  the ceiling model is known to be beatable: Lane A measured a bank of 616
  against a printed 540. R13 is therefore the one arm where the arithmetic
  genuinely cannot call it and the measurement has to.

---

## 2. What was measured, and what the proxies cannot see

**The two stamp routes need mechanism in `src/sim/creature.rs`, which this
lane does not own.** They are priced by proxy: `body_energy=` reproduces the
birth arithmetic each route implies, in-process, with no source change and no
asset edit. What each proxy does and does not cover, stated rather than
assumed:

- **Route 1's proxy is near-exact on the books.** The route halves the
  *cells* stamped at birth (2 → 1) at `body_energy` 480; the proxy halves
  `body_energy` (480 → 240) at 2 cells. The product — what a birth takes out
  of the world, 480 — is identical, and so is the total worth of the
  resulting animal's flesh. What it cannot see is the thing the route is
  really about: **a one-cell newborn's survival in the window before it grows
  its second cell**. The proxy's animals are born full-size. That question
  needs the mechanism and is named in §5 as the first thing to measure after
  building it.
- **Route 2's proxy is conservative.** Setting `body_energy=0` gives the
  stamp the route gives it, but the harness moves `ant` and `corpse`
  `food_energy` with it — deliberately, to hold the flesh-pricing invariant
  that closes the corpse pump (dead-ends §13l) rather than open a pump it
  would then measure. Real fission moves a cell that is still worth 480. So
  **the proxy deletes carrion from the menu**, which makes route 2 look
  *worse* than it is. A route that closes despite it, closes. It also cannot
  see what fission does to the *parent* — §5 names that too.
- **Route 3 needs no proxy at all.** `gut=` sets the diet gene directly and
  the harness reads it back off a live founder, so a run cannot silently
  measure the neutral gut. This is the route that needs no new mechanism, and
  it is measured as itself.

**The seeds are pre-registered on a criterion independent of every route.**
Placement is wildly seed-dependent — the harness reports how many of its 55
founders actually got onto the ground, and it ranges from **2** (seed 1) to
**51** (seed 21). A world with no ants in it reports `births 0` exactly like
an ant that cannot afford one. All 30 seeds were screened at `frames=1`,
before any arm ran, and the **18 that seat at least 30 founders** were taken:
2, 6, 8, 9, 10, 11, 12, 13, 14, 16, 18, 20, 21, 22, 23, 25, 26, 27. Six seeds
is not a sweep, and eighteen is what the house rule asks for.

**The positive control fires.** Lane A's gate control —
`start_energy=200 body_energy=20 threshold=241 hunger=0.9 terrain=world
frames=24000`, at seed 2711 — gives **births 8,982, live 3,283, generation
102, 19 lineages**. A null there would have voided everything below. It is a
control and not a proposal: `hunger=0.9` is in dead-end territory, and a
route that only worked there would not have worked.

