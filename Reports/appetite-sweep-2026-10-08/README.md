# Steady food: the appetite dial does not steady the colony; `edible` does, and the stack starves ants on the surface (2026-10-08)

**Step A of the playtest plan**, on renewable food (the owner, 2026-10-08:
dying on finite food is expected; the test is renewable food), at a colony of
a few hundred. Measured unless marked *inferred*. 4 seeds each; per the
owner's rule, more seeds only where results are unclear.

## Setup

- `steady_income.ron` at **40 cells of provisions per 1,000 frames** (38,400 J),
  30 columns east of the door: the owner's playtest switches hold 190-280 ants
  by 130k, main's defaults about 300.
- `deeptrace` built from main plus the appetite-dial row (now on main via PR
  661), `food=0` (no harness top-up), `hungry=1`, 200k frames, mutation off,
  seeds 1-4. Scripts: [`run4.sh`](run4.sh), [`sweep.sh`](sweep.sh).
- Environments: **play** (the owner's 2026-10-07 playtest switch line, no
  `edible`), **playedible** (the same plus `NEST_STORE` `edible`), **main**
  (no switches: main's shipped game).

## 1. The appetite dial (`digest_hunger_weight`): not clear, so not a default

| env | weight | colony min / mean after 100k | adults starved | larvae starved | eggs |
|---|---|---|---|---|---|
| play | 0 | 106-224 / 219-268 | 154-812 | 17-315 | 1,866-2,219 |
| play | 0.25 | 112-217 / 207-255 | 283-653 | 2-119 | 1,812-2,068 |
| play | 0.5 | **18**-167 / 141-247 | 306-596 | 4-48 | 1,408-1,872 |
| play | 0.75 | 30-145 / 103-229 | 98-153 | 0-8 | 814-1,213 |
| main | 0 | 213-263 / 267-295 | 53-98 | 179-247 | 1,895-1,931 |
| main | 0.25 | 235-266 / 274-294 | 47-148 | 215-219 | 1,928-1,955 |
| main | 0.5 | 204-255 / 266-280 | 58-94 | 152-180 | 1,734-1,798 |
| main | 0.75 | 162-197 / 222-247 | **202-267** | 102-117 | 1,505-1,638 |

(main weights 0.25-0.75 were seeds 1-3 at writing; seed 4, added after, falls inside every range but one: at 0.25 its minimum is 175 and adults starved 65, at 0.75 adults starved 336.)

- **Consistent:** more weight means fewer eggs and fewer larvae starved, on
  every seed in both environments.
- **Not consistent:** the colony's swing and adult starvation. On `play`, 0.5
  dropped one seed to 18 ants; on `main`, 0.75 tripled adult starvation.
- **Verdict: stays a lab knob at 0** (on the ANTS page since PR 661). It trades
  eggs for fewer starving larvae, but it is not what makes the colony steady.
  The 4-seed result at the smaller income (no near-collapse at 0.5) did not
  replicate at 40 cells.

## 2. What does make it steady: `edible`

The `play` colony's dips (to 106-109 ants on seeds 3 and 4) were **mass
starvation inside the nest**: 242 of 243 ants starved in the worst 10k-frame
pulse were in the dug nest, and 814 of the `play` arm's starvers over 4 seeds
died there, against 16 on `main`. That is the store crumb trap
(`Reports/handoff/2026-10-07/residual-nest-deaths-deep-trace-2026-10-07.md`):
the nest store reads "can feed" on crumbs too small to eat and the hungry
stay beside it. The owner's playtest ran without the fix.

| env | seed | colony min / mean / max | starved (in the nest) | larvae starved |
|---|---|---|---|---|
| play | 1 | 213 / 261 / 301 | 197 (0) | 315 |
| play | 2 | 224 / 268 / 303 | 155 (1) | 116 |
| play | 3 | **109** / 243 / 301 | 411 (**222**) | 222 |
| play | 4 | **106** / 219 / 304 | 827 (**591**) | 17 |
| playedible | 1 | 221 / 269 / 350 | 424 (1) | 142 |
| playedible | 2 | 225 / 268 / 304 | 387 (9) | 121 |
| playedible | 3 | 244 / 272 / 293 | 191 (3) | 6 |
| playedible | 4 | 243 / 275 / 304 | 198 (1) | 131 |

**With `edible`, deaths in the nest fall to 1-9 per run and no seed dips
below 221.** This is the clearest result of the night, and it is consistent
on every seed that had the pulses. It adds weight to Deep trace's "yes to the
fix" (`second-key-verdict`), on renewable food.

## 3. What is left: the stack starves ants on the surface

Starved by zone, 4 seeds summed: `play` nest 814, surface 669; `playedible`
surface **1,087**, nest 14; `main` surface 150, mound tunnels 109, nest 16.

- With the trap gone, the stack still loses about 7x main's surface starvers.
- They are not lost foragers who never found food: 845 of 1,087 had been at
  the heap, 664 spent most of their hungry spell east (the heap's side), and
  they die at a median age of 15,310 frames (main's few surface starvers die
  at a median 3,659).
- *Inferred, untraced:* something in the stack makes ants that reach the food
  still starve near it. Candidates: `NEST_STORE`'s fetchers emptying the heap
  into the nest (`pick=20`), `CARRY_HOME`'s crop fill, `LAY_BAR=body` turning
  surplus into eggs, `NEEDS_FIRST`. **Next: a leave-one-out on `steady_income`**
  (`playedible` minus each switch, 4 seeds), which also answers part of the
  stack's default-flip question (plan step D).

## Floor check (nest goal, heap 90, endless food)

The dial must not shrink colonies that live on endless food. `nest_goal`,
heap 90 columns from the door with the harness top-up, weights 0 and 0.5,
4 seeds, 200k:

| env | weight | colony min / mean / max after 100k | adults starved | eggs |
|---|---|---|---|---|
| play | 0 | 629-686 / 673-724 / 717-790 | **21-29** | 3,889-3,940 |
| play | 0.5 | 626-801 / 754-934 / 867-1,108 | **275-575** | 4,143-4,538 |
| main | 0 | 121-176 / 252-333 / 376-473 | 57-148 | 1,823-2,045 |
| main | 0.5 | 68-221 / 259-334 / 362-444 | 97-119 | 1,644-1,937 |

- **On the playtest switches, 0.5 starves 10-20x more adults on every seed**
  (275-575 against 21-29) while the colony grows larger (mean +10% to +29%).
  The food is endless, so this is not the income: *inferred*, ants holding
  their surplus as crop cargo instead of in their bodies run lean between
  meals. Not traced.
- On main the dial is mixed: no consistent direction on any column.
- So the floor check adds a cost on top of section 1's verdict: the dial
  stays at 0.
