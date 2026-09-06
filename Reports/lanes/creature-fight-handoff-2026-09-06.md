# Ant/beetle fighting dynamics — handoff, 2026-09-06

**Branch** `claude/creature-behavior-analysis-m28jo2`
**Status** the two blockers below are fixed on that branch; the *balance*
question is open and is what this note hands over.
**Guards** `a_chain_creature_bites_what_is_eating_its_back` (new, four arms),
`a_swarm_gets_through_what_one_mouth_cannot` (rewritten -- read its doc
comment before trusting anything it used to assert).

---

## The one thing to read first

**A lone ant takes a maximum-armour beetle apart, and did so before any of my
changes.** Measured in `a_swarm_gets_through_what_one_mouth_cannot`'s own
scene, with the whole-body scan held off — i.e. against the code as it stood
on `main` — one ant kills a beetle at `TRAIT_ARMOUR = 1.0` (the top of the
range) at **frame 101**. With the scan fixed it survives to **162**, which is
better and is still not "stronger than an ant".

That test was called `a_swarm_gets_through_what_one_mouth_cannot` and asserted
`swarm_gnaws > lone_gnaws` — a count of *bites*, 12 against 30, duly green —
while the outcome in its own title was false in both arms. **Its green was
evidence about bite counts, not about the ruling.** It now asserts
time-to-first-breach, which is 5–6x apart on both sides of the change and goes
red under attacker-banking; but the claim in the name is still not something
the engine can currently demonstrate.

**So the open question is: can any armour a lineage can actually reach make a
beetle survivable against one ant?** Today the answer looks like no, and the
arithmetic says why — see the cap below.

## The cap nobody has priced against

`armour_of` is `1/ratio_factor(t)` with `t` clamped to `[-1, 1]`, so the
multiplier is bounded in **[0.5, 2.0]**. The beetle material authors
`penetration_resistance: 0.8`, so a beetle's reachable armour is **0.4 to
1.6** against an ant's bite of **1.0**. At the very top, `damage = (1.0/1.6)^2
= 0.39` — about 2.6 bites per cell, on a four-cell body. That is the whole
budget an armoured lineage has, and it buys ~10 bites.

Three levers, none of them explored:
- the material's base `penetration_resistance` (currently **0.8**, and the
  activation to raise it is parked — see below);
- the trait clamp, which is `[-1, 1]` for every slot and is not obviously
  right for a slot whose ratio is the point;
- body size, which multiplies the bill without touching either.

## What is parked, and why

`assets/materials/beetle.ron` still ships `penetration_resistance: 0.8`,
deliberately. Raising it past the ant's 1.0 makes the beetle **inedible**, and
an inedible breeding beetle collapses the colony from 17–21 ants to 3–5 on
4/4 seeds. The comment in that file documents its own obsolescence: *"a food
priced above every shipped bite force is not armoured, it is inedible."* The
graded bite removed the binary, so the number can now move — but it has not
been swept, and the ecology result above is what it has to be swept against.

`Reports/selective-environments-2026-09-05.md` § *a predator you can eat is
not a predator* has the seed table.

## Two blockers I removed (both landed)

**1. A chain creature could not bite what was eating its back.**
`adjacent_food_counted` scanned the eight neighbours of the *head*. Measured
in a sealed chamber over 1,200 frames: the beetle's head was adjacent to the
ant **zero** times and the ant's head was adjacent to the beetle **sixty** — a
perfect asymmetry, and not because the beetle was idle, it roamed the whole
chamber with its energy barely moving. Every cell that was not the mouth was
pure liability, which inverts what body length and armour buy. It now scans
every body cell, deduplicated, and **past the mouth it reaches only for a
living non-self organism** — foraging is a mouth, fighting is not. Guard:
`a_chain_creature_bites_what_is_eating_its_back`, four arms.

The effect, on the swarm scene: a beetle at maximum armour survives a lone ant
to frame **162** where it used to fall at **101**. The outdoor game is
bit-identical — `ascii`'s ant scenes hold only ants, and a nestmate is kin, so
the extra reach never offers anything (deposition 1.03x on 744 drops from
7,276 laden ants, digit for digit the same as `main`).

**2. Gnawing was priced per unit of progress, so armour bought time and never
cost.** `dig_cost_in_moves * damage` made the total cost of breaching a cell
*constant at every armour value*: 1.50 J for a 50 J beetle cell whether the
plate took one bite or four hundred — a flat 33x profit no thickness could
dent. It is now charged per jaw closure: armour 1.6 costs an ant 3.9 J for
that cell, armour 8 costs 96 J, and somewhere between them gnawing stops
being worth doing.

## Two traps this cost me, for whoever measures next

- **`bites_refused` and `gnaws` are multiplied by body geometry if the scan
  is not deduplicated.** The first whole-body version carried a comment saying
  a cell adjacent to two body cells "is simply scored twice, which changes
  nothing". It changed `bites_refused` from 1 to 2 on an unchanged scene. The
  ring is now visited once per position.
- **Foraging reach and fighting reach are not the same reach, and widening
  one widens the other unless you stop it.** The first version of the scan
  fix let the whole body look for anything edible, so an ant picked loose soil
  up with its abdomen. `ascii`'s deposition-follows-moisture ratio fell from
  **1.03 to 0.82** against a 0.9 bar -- drops 744 -> 946, laden ants
  7,276 -> 9,374, i.e. more opportunistic grabs taken wherever the body
  brushed something and a weaker link between where an ant goes and where it
  puts things down. Past the mouth the body now reaches only for a living
  non-self organism. If you widen this again, that gate is the one that
  notices.
- **`gnaws` cannot compare two arms that both finish the job.** A beetle is
  finite, so the chewing it takes is finite: with the body scan, one ant and
  eight ants land **39 bites each, exactly**. An identical number across two
  arms that must differ is this repo's tidiness tell, and here it is not a bug
  — it is a conserved quantity. Use time-to-first-breach.

## Where things are

- `src/sim/creature.rs` — `adjacent_food_counted` (the scan), `Did::gnaws`
  (the price), `armour_of` / `armour_at` (the multiplier).
- `src/sim/organism.rs` — `TRAIT_ARMOUR = 9`, `armour_fraction`.
- `src/lab/params.rs` — the GENOME and COSTS pages; every scalar an ant is
  made of is reachable from the box, and there is a guard that says so.
- `examples/labstats.rs` — `beetlearmour=`, `antbite=`, `pace=` overrides.
