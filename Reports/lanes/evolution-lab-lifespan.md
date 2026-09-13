# Lane: ants die of age

Design of record:
[`../evolution-lab-late-game-design-2026-09-12.md`](../evolution-lab-late-game-design-2026-09-12.md)
§1.2 and §2 brief 2. Shipped on, authored per species, **not heritable**.

**Everything measured on this line before 2026-09-12 was measured through the
seed cull** (#366: an ant that ate a seed was overwritten by the seed, two
fifths of colony deaths). The re-take is round 31, lane A:
[`../evolution-lab-lifespan-rederived-2026-09-13.md`](../evolution-lab-lifespan-rederived-2026-09-13.md),
raw digest of all 75 runs in
[`../data/evolution-lab-round-31-lifespan-sweep.txt`](../data/evolution-lab-round-31-lifespan-sweep.txt).
**Read that before quoting any number from this note's history.**

## Where the constant stands, 2026-09-13

**`life_half_life: 40000` is unchanged, and is now measured rather than
inherited.** Twelve seeds, five arms, `latecensus scenario=played_bed
frames=200000`, `RAYON_NUM_THREADS=1`, all arms out of one binary:

- **40,000 is the floor.** Halving to 20,000 is the only setting in the sweep
  that kills colonies — **four of twelve seeds at zero ants by 200,000
  frames**, against twelve of twelve alive at 20,000's neighbours *and at
  immortal*. The pre-fix sweep read halving as "flattened no further" and
  shipped 40,000 on that; post-fix it is plainly worse, so the floor is real.
- **Doubling to 80,000 buys nothing a paired test can see.** Ants 5 of 12,
  plants 6 of 12, bank 8 of 12, all p >= 0.39. Its low tail is kinder (ants at
  200,000: p10 45 / min 27 against 40,000's p10 6 / min 1) and **the tail and
  the paired test disagree, so the paired test wins** — five seeds run the
  other way, which is what a two-seed tail difference looks like at n=12.

**The claim this lane was built on is withdrawn and not restored.** Against an
*immortal* colony the lifespan no longer changes the population at all: ants
7 of 12, plants 5 of 12, seed bank 6 of 12 at 200,000 frames, three coin
flips. The 6.6x runaway bound (3,182 → 483) was already withdrawn when scent
drift moved the baseline; this sweep says the remaining effect was the cull.
**A lane arriving from the pre-fix record will expect the lifespan to be a
population brake. On this trunk it is not one.**

**What it does buy is the death.** Age is **55% of colony deaths at the
median** at 120,000 frames and outnumbers hunger on nine of twelve seeds, and
the ageing colony starves less on ten of twelve (median −26 deaths, p=0.071).
Every one of those was a starvation before this shipped.

## Two things this lane's numbers now overturn elsewhere

- **§Z6 does not reproduce as written.** *Every shipped bed starves its colony
  inside one play session* was two of nine runs alive; the shipped arm at
  200,000 frames is **26 of 27 across all six shipped beds**. It should be
  rewritten, not closed — the bar is at 300,000 frames on two `labstats` beds
  this sweep does not run, and three of the 27 end in single figures.
- **The dig gate's justification has moved.** #359's *5 beds of 12 against 0*
  is gone (12 of 12 alive in both arms), and its digging counter does not
  reproduce at 200,000 frames on this trunk (5 of 12, p=1.00, against 11 of 12
  at 1.90x). What survives is the bed: **plants standing higher with the gate
  on, 10 of 12 seeds, p=0.039** — and only in the second half of a session,
  not at 120,000 frames.

## What not to re-derive

- **The hazard interval had to become an argument, and it is settled.**
  `plant::old_age_chance` baked `ORGANISM_TICK_INTERVAL` (45) into a per-tick
  chance; an ant rolls every 6. Called unchanged it would have put an ant's
  median at **T/2.7** *and made `pace` a lifespan gene* — a heritable trait
  silently deciding how long its lineage lives. The repair is
  `plant::old_age_chance_over(age, T, interval)`, the plant's own function a
  wrapper on it, and `the_hazard_is_a_property_of_the_half_life_not_the_interval`
  asserting the three survivorship numbers at intervals 6, 12 and 45. The
  chance is per *frame* and the interval multiplies it, so **any new caller at
  a new cadence must pass its own** — the *individual's*
  (`organism_tick_interval`), which already reads `pace`.
- **Survival at `2.5T` is 1.31%, not 0.4%.** `exp(-ln2 · 2.5²)`; `0.4%` is
  `2^-8`, the survival at `2.83T`. Corrected in `plant.rs`, in the two test
  comments and (2026-09-12) in design §1.2 line 182. No bar depended on it.
- **`life_half_life` is `u32` frames on `CreatureDef`, 0 = immortal.** A
  species file that does not name it is bit-identical; that is why the default
  is 0.
- **Not heritable, and that is a ruling** (design §4: priced before inherited).
- **`DeathCause::OldAge` is appended to `DEATH_CAUSE_LIST`, not inserted.**
  `World::deaths_by_cause` and `GroupDeaths::by_cause` are positional arrays.
- **The dial is on the ANTS page under `tick_interval`**, not GENOME — GENOME
  is "what a lineage inherits" and this is not.
- **`lifespan=` is accepted by `latecensus`, `labstats`, `labforage` and
  `labgif`**, all four echoing the value whether or not it was passed. The dig
  gate has no argument at all — it is `PIXEL_PHYSICS_LAB_ROOM=off`, read once
  per process by `creature::room_gate_default` — and since 2026-09-13
  `latecensus` echoes it on the parameter block for that reason.
- **`labgif` defaults to `rain=steady` and `latecensus` has no rain at all, so
  a card paired against a census must pass `rain=off`.** Cost an afternoon's
  render: at the default the seed-1 colony peaks at 42 ants and is extinct by
  frame 250,000, where the census on the same scenario, seed and lifespan has
  it at 2,013 and climbing. It was not showing a weaker boom; it was showing a
  different world.
- **The cohort test has to zero eight charges, not two.** Idle and move are
  the obvious ones; brain, eye, ground sense, jaw and shell are each levied
  per tick as a fraction of `start_energy`, and leaving them on puts
  starvation deaths in the column a survival curve reads.
- **A baseline control shorter than the mechanism's onset proves nothing.**
  The scent-drift merge left three of six paired runs **byte-identical** and
  changed the other three completely — inert on small colonies, decisive on
  large ones — and the changed arms were identical to 100,000 frames. The
  60,000-frame determinism control could not have caught it.

## The controls this line runs, quoted

- **Positive control** — 36 founders at `life_half_life: 6000`, `latecensus
  frames=24000 sample=1500`: 35 of 36 alive at `T/4` (97.2%, model 96%), 7 at
  `T` with 17 dead of age (47%, model 50%), 1 at `2.5T`. **The fault put back**
  is the paired `lifespan=0` arm: `OLDAGE` 0 at all seventeen stops.
- **Round 31's positive control is #366's own published pair.** The fix moved
  ants alive at 120,000 frames from 0 / 8 / 29 to **52 / 10 / 135** on seeds
  1–3; the shipped arm of this sweep reads 52 / 10 / 135 exactly.
- **`ascii` is green and NOT byte-identical**, and only **two** scenes changed
  behaviour: `ants: the foraging loop` — 22 → 15 creatures at 12,000 frames,
  deaths 7 → 9, births 11 → 6 — and `ants: deposition follows the moisture
  gradient`, deaths 52 → 53. Expected age deaths among ~20 ants are ~1.2 and 2
  are seen; the lost births are the knock-on of the lost workers.

## The superseded sweeps

The `f3acaf76` and `c7ee0f40` tables that stood here until 2026-09-13, and the
`labstats` paired arms beside them, were all taken through the seed cull. They
are in this file's history and in
[`../evolution-lab-round-29-2026-09-12.md`](../evolution-lab-round-29-2026-09-12.md);
**do not quote them.** Two of their conclusions are already withdrawn in this
note above, and the third — every colony at every setting extinct by 500,000
frames — is simply unknown, because round 31 stopped at 200,000.

Card `20260912T092226496Z-07d3ab` (blind, board `lab`) pairs the two whole
sessions as 41-frame sequences and is also pre-fix. Collect with
`review.py inbox`.
