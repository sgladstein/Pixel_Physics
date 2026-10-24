# Body genes in the training bed (body-genes thread, 2026-10-22)

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*): copied verbatim from the project shared folder `body-genes/`. The thread is closed; `results.md` and the four bed-switch patches (against compute commit 98ad9dc4, apply all four) are in [`body-genes-2026-10-22/`](body-genes-2026-10-22/). The engine gene itself is on `main` (PR #827, 2a67f6d).

Owner, 13:34Z: "The body should be able to mutate too." This folder is the hand-off to the Teacher lane.

## The switch

Branch `claude/project-thread-fnf43q` = compute branch `claude/project-thread-8y5ak0` at 98ad9dc4 plus two commits:

- `db1cc976` (engine, the only part meant for main, after the owner sees numbers): the appetite gate is a body gene,
  `organism::TRAIT_APPETITE` (slot 27; the trait vector grows 27 -> 28). The gate's weight is
  `digest_hunger_weight + allele`, clamped 0..1. Width 0 on every species, so nothing drifts in the game.
  `creature::mutate_traits` is the body half of the game's birth (split out of `mutate_newborn`, same draws in the same
  order) and `creature::inherit_body` lays a parent's body on a child the harness placed.
- `a4a59972` (scratch, examples/inrun.rs): `body=LIST`. Off by default.

`body=default` opens: **appetite, digest_rate, crop_capacity, gut_bias** (pace left it after the pilot: see [`results.md`](body-genes-2026-10-22/results.md)) (widths: the bed species' own, 0.15 where
it ships 0). Any slot can be listed by its lab name, `name:width` to override, e.g. `body=appetite:0.3,pace`.
**With appetite open, pass `digest_bar=3`** (the bed's breeding bar is 1e30, so without it the gate reads every ant as
hungry and does nothing; the run prints a warning).

A child takes its parent's open genes and mutates them on its own birth stream; closed genes stay as the founding body
and take no draw. Bodies carry across rounds: `final/ID.body.txt` sits beside `ID.raw.txt`, and a `.raw.txt` founder
picks up its sibling body file. `body.csv` in each round logs every ant's open genes (founders at frame 0, then each
child as born, with its parent).

## What was checked (hA1, the ha1next arguments)

- **Off is byte-identical** to the compute branch at 98ad9dc4: seeds 1 and 2 at 200k frames, events/births/sel/stock
  identical with the gate off, and with `hungergate=1 digest_bar=3` (which now reads through the new gene function).
- **17 of the old 27 genes are dead for a mind ant.** Opened at width 0.5 they moved by up to 0.58, and events, births
  and sel stayed byte-identical to body off over 200k frames on two seeds (229 and 308 births). They are:
  birth_grant, reproduce_at (the bed breeds by the harness, not the body); sight_range, curvature_radius (a mind casts
  no rays, and is not billed for them); the six walk gains, four trail-plane weights and forage memory, and both door
  genes (read only by the game ant's own walk and act, which a mind replaces).
  So the worry that "behaviour genes overlap the brain" does not arise for the new ant: they do not reach it at all.
- **Each live gene changes the run** when opened alone (positive control): appetite (with digest_bar=3), digest_rate,
  crop_capacity, gut_bias, pace, armour, dig_force all change events.csv within 40k frames.
- Round carry-over: all 24 round-1 founders' bodies match a round-0 final body.

## Why these five, and not armour or dig_force

- **armour** is billed every tick and nothing bites an ant in this bed, so selection can only push it down. That would
  breed soft ants for a game where fights exist. Closed.
- **dig_force**: the ant's jaw is 1.0 and packed soil / spoil is 0.95, so one mutation step down (up to 0.15) leaves an
  ant that cannot cut spoil, and the jaw is billed every tick. Selection is already breeding digging out of the bed
  (dig lane), and a cheap jaw would lock that in. Closed. (Side finding: the lab game mutates this gene at 0.15 today.)
- **pace** was open in the pilot and climbed fast (+0.2 to +0.6..0.8 in 6 rounds); a quicker ant takes more turns per frame (~1.5x slower frames), so it left body=default. `body=...,pace` still opens it.
- **gut_bias** is open: the bed has one food (provisions), so it will move toward whatever suits provisions.

## The batch (`scratch/inrun/jobs/body1022.txt`, 320 jobs, hA1 64 seeds x 6 rounds x 200k)

| arm | args on top of ha1next's control |
|---|---|
| hA1c | none (same-batch control) |
| bodyD | `body=default digest_bar=3` (gate gene starts off, may evolve on) |
| bodyG | `body=appetite digest_bar=3` (the gate gene alone) |
| hg | `hungergate=1 digest_bar=3` (species gate fixed on: the ha1next arm) |
| hgbodyD | `hungergate=1 digest_bar=3 body=default` (gate starts full, may evolve down) |

`python3 scratch/cloud/inrun_cloud.py start body-1022 jobs=scratch/inrun/jobs/body1022.txt --ref claude/project-thread-fnf43q --type c4d-highcpu-64 --hours 5 --disk 200`
Judge with evreduce.py + pairfood.py as before. `scratch/inrun/bodydrift.py DIR` prints where selection took each open
gene per arm and round (works on a `pull` of the small copies). `scratch/inrun/localreduce.py` makes evreduce's rows
from a local batch.

Status and results: see `results.md` in this folder when it exists.

## From the teacher lane, 14:25Z
The 64-core quota is one VM for the whole account (us-central1 preemptible 64; other regions 0-24, global 12). My needdrop-1022 (owner-approved test, ~12 min) has it now; I delete it the moment it finishes. Then launch body-1022 yourself, it is yours. I will judge it with the common garden (cgtools in my scratchpad) if you leave results.md.

## Patches (patches/, against compute branch 98ad9dc4)

Apply **all four** with `git am` (0002 and 0004, the bed switch, need 0001's `creature::inherit_body`). 0001 and 0003
are the engine change, which is also going to main by a merge-desk PR; once main is merged into the compute branch
they fold in. Branch claude/project-thread-fnf43q no longer holds the bed switch: it is main + the engine gene.
