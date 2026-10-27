# A memory that writes and then holds: the write gate prototype (2026-10-27)

**Status:** prototype, off by default (`PP_WRITE=on` makes it trainable), and a measurement. The owner chose
"prototype it, off by default" on 2026-10-27 15:05Z after
[the latch landscape](latch-landscape-2026-10-27.md) found no step-by-step path to the hand-laid "home had food"
memory. Code: brain layout 8 (`mind::WRITE`). Data and harness: [`write-gate-2026-10-27/`](write-gate-2026-10-27/).

## Answer

**Rebuilt on a write gate, the same memory works as well, survives being damaged, and has a slope.** The
flip-flop version needed every piece at full strength. Half strength did as badly as no memory at all, and two
of its broken versions lost 13 and 14 of 16 colonies. The write-gated version keeps working at half strength. No
knock-out loses more than 2 colonies. Its simplest form, a single weight that says "write at the nest", already
starves fewer ants than no memory in 13 of 15 worlds. That is the step-by-step path the flip-flop lacked. The
last step is still a large jump, from about 250 starved to 40, and it needs two more weights to line up. Whether
selection actually makes that jump is the next test, which needs a selection run with `PP_WRITE=on`.

## The mechanism

Until layout 8, a memory unit had one rate gene, `r`, and every decision it moved to `r·old + (1-r)·new`. One
number set both how fast it wrote and how long it held. A unit could not write in the few decisions an ant
spends at the nest and then hold that fact through a trip. The only way to keep a fact was a flip-flop: a unit
feeding itself back at a weight past 1, balanced against set and reset inputs.

Layout 8 gives each of the 12 memory units a **write gate**. This is a row of 44 weights over the context, the
memory before, what each verb could reach, and a bias. Its sum `y` sets how far the unit moves this decision:
`z = clamp((1 - r) + y, 0, 1)`, then `m' = m + z·(new - m)`. With every write weight at zero, `z` is the old
`1 - r` and the old formula runs as before. So every saved pool remembers bit for bit as it did (see checks
below). With the rate gene at its top and one weight on `at_nest`, a unit writes at the nest and holds away from
it.

## The memory rebuilt on one unit

Every arm starts from `scratch/inrun/genomes/latch_pool.txt` (30 members, layout 7). Memory unit 9, the
flip-flop, is cleared and rebuilt as follows (`mkwrite.py`). Units 8 and 11 are left as they were. Unit 8's gate
still reads unit 9 at `+3`, bias `-2.74`.

- **Value:** `squash(60·reach[PickUp] - 10)`. This is about +0.83 to +0.98 when there is something to pick up,
  and -0.91 when there is nothing.
- **Hold:** rate gene 0.495, which gives rate 0.995.
- **Write:** `y = 96·at_nest + 84·reach[PickUp] - 30·energy - 84`. The gate opens at the nest in two cases:
  when there is food to pick up, or when the ant is hungry (energy below about 0.37, the old reset's threshold)
  and there is none. It stays shut away from the nest, and at the nest when the ant is fed and there is nothing
  to take. Without the energy term, it writes "empty" at the door on every trip. That is the Teacher lane's v1
  failure.

Each arm ran 16 worlds on the Teacher lane's standard setting (`wl.sh` in the landscape folder; seeds
`s*1000+7`, far moving food, founding grant, teacher off, selection off, 200k frames).

| arm | what changed | lost (of 16) | starved, median of the living | lower than full |
|---|---|---|---|---|
| **wfull** | nothing | 0 | **40** | -- |
| w_half | every write weight halved | 0 | 49.5 | 4/16 (4 ties) |
| w_lowrate | rate 0.5, not 0.995 | 1 | 36 | 8/16 (2 ties) |
| w_nohunger | no energy term: writes at every nest visit | 0 | 239 | 1/16 |
| w_nopick | no PickUp term: writes only when hungry at the nest | 2 | 252.5 | 1/16 |
| w_atnest | one weight: write at the nest, hold away | 0 | 253.5 | 2/16 |
| w_nowrite | no write gate: a slow average at rate 0.995 | 0 | 320.5 | 0/16 |
| w_noat | no at_nest term: never opens | 0 | 284.5 | 0/16 |
| w_novalue | the value never says "food" | 0 | 284.5 | 0/16 |
| w_noread | the come-home gate does not read the unit | 0 | 284.5 | 0/16 |

The flip-flop with every piece, in the same batch on this build: starved 55, 50, 42 and 103 on the first four
worlds. Those are the same four numbers as on main at layout 7.

The last three arms give identical numbers. In all three the unit never opens unit 8's gate, so they are the
same run. Any value they would hold is not read.

## Is there a slope?

These are paired sign tests over the 16 worlds, two-sided. Ties are dropped. Eleven tests were run, so the
Bonferroni bar is p < 0.0045.

| comparison | fewer starved in | p |
|---|---|---|
| w_atnest vs no read (one write weight, against no memory) | 13/15 | 0.007 |
| w_nopick vs no read | 13/16 | 0.021 |
| w_nohunger vs no read | 12/16 | 0.077 |
| w_atnest vs w_nowrite (the gate's one weight, against none) | 13/16 | 0.021 |
| w_nohunger vs w_atnest (adding the PickUp term) | 10/16 | 0.45 |
| wfull vs w_atnest | 14/16 | 0.004 |
| wfull vs w_nohunger | 15/16 | 0.0005 |
| wfull vs w_nopick | 15/16 | 0.0005 |
| w_half vs wfull | 4/12 | 0.39 |
| w_lowrate vs wfull | 8/14 | 0.79 |
| w_nowrite vs no read | 5/15 | 0.30 |

- **The first step pays, but only modestly.** One weight that writes at the nest cuts starvation from about 285
  to about 254 per world, in 13 of 15 worlds (p = 0.007). That does not pass the Bonferroni bar on its own. With
  the two-weight versions also lower than no memory (13/16 and 12/16), the direction holds up. Under the
  flip-flop, every partial version starved 280 to 309, the same band as no memory, and two were lethal.
- **The middle step is flat.** Adding the PickUp term to the one-weight gate changes nothing measurable (10/16,
  p = 0.45). Mutation would have to drift across this step, not climb it.
- **The last step is the big one.** Going from either two-term version to the full gate cuts starvation from
  about 245 to 40, in 15 of 16 worlds. That passes the bar. It needs the energy weight and the bias to land
  together. They have a wide band to land in, though: halving the whole row costs nothing measurable (4 lower
  of 12, p = 0.39).
- **It forgives damage.** Half strength works. The rate gene hardly matters, because a shut gate clamps `z` to
  0 whatever the rate (w_lowrate, 8/14, p = 0.79). The worst knock-out loses 2 colonies, not 14. Breaking the
  flip-flop's set input was lethal because an off memory unit reads -1 and tipped the balance (landscape report).
  Here nothing is balanced.

**Caveat on scope.** Every arm keeps the parts that are not the memory itself: unit 9's value reading
`reach[PickUp]`, and unit 8's gate reading unit 9. Selection would also have to find those. This measures the
write gate's own slope, not the whole path from a colony with no habit at all. That is the selection run's job.

## Checks

- Every pool saved before layout 8 gains the rows at zero and remembers bit for bit. The shipped-pool guard
  (std_pool, spider, centipede; 300 decisions each) keeps its hash. The layout-7 flip-flop arms give the same
  results here as on main. The played lab box gives the same RESULT line (153 alive, 0 starved, 257
  deliveries).
- Frame cost: none measurable. The played lab box with the shipped pool ran 33.0 / 32.8 s on main and
  32.9 / 31.5 s here.
- New unit test `a_write_gate_holds_a_memory_until_it_opens`. A gate opened only at the nest writes there in
  one decision and holds bit-exact away from it. The same unit without the gate barely writes, and drifts.

## Next

1. **A selection run from a pool with no habit, `PP_WRITE=on` against off.** This is the Teacher lane's
   machinery. The test is whether come-home-when-home-had-food appears, judged by the traced starvation of far
   colonies, not by score alone. A pass is the case for turning `PP_WRITE` on by default.
2. Whether the trainable write rows' link cost (528 more slots, billed like any link) slows other learning.
   Watch the standard cases in the same run.
