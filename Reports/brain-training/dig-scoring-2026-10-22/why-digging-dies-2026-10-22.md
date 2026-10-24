# Why selection breeds digging out, and a fix to test (dig-scoring thread, 2026-10-22)

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*): copied verbatim from the project shared folder `nest-score/digloss/`. Files it names are in [`digloss/`](digloss/) beside this report (the two GIFs stay in the shared folder, `nest-score/digloss/gif/`). **Read the 13:15Z REFUTED note and the 13:40Z mechanism in [`README.md`](README.md) first:** round length was later shown not to be the lever.

> **Update 13:15Z: the short-round fix did NOT hold at 64 seeds.** The Teacher lane ran shortr-1022 (hA1, 6x200k / 12x100k / 24x50k, equal frames) and its own common garden (`shortr-1022-commongarden.txt`, 200k frames, sel off). Cuts, median: 131 / 190 / 116 vs unselected hA1 201. Paired by seed, 50k vs 200k: higher 6, lower 6. Food is the same. The 8-seed c1 result below (5/5, p=0.06) was most likely luck. The trace (most generations are bred while nobody cuts) stands as a description, but it does not show that round length is the lever. Note that hA1 erodes far more slowly than c1: 131 cuts after 6x200k, where c1 reached 0 by r4.

**Question.** In the Teacher lane's 60-round run (L60c1, c1 founder, score selection, 64 seeds), digging dies by
round 6: nests shrink to the 26-cell stamp. Paying more per cut (w_dig x2, x4; digw-1022) only delays this. Why?

**Answer in one line.** Digging happens at the start of each round, while the fresh world has a nest to dig, but
births happen all round. So in a 200k-frame round, about three quarters of the ~10 generations are bred while almost
nobody digs. Those generations are picked on food alone, and food and digging trade off in the brain, so digging
erodes. Paying more per cut can't stop this, because the pay only works in the stretch where ants are cutting.
**Short rounds, the same frames split into 50k-frame fresh worlds, kept digging in a small local test.**

## The trace (L60c1, seeds 1-12, rounds 0-20; 41 colony-rounds with 100+ cuts)

Scripts: `trace60.py`, `matched60.py`, `pick60.py`, `gen60.py` here; data pulled with `pull60.py` from the bucket.

1. **Digging is inherited.** Parent to child cut rate (same round, both adults): r 0.35, slope 0.34, n=6,049.
2. **Parent choice favours cutters, not the reverse.** At each birth, the parent's cuts in the previous 20k frames
   are compared with those of the adults alive over the same frames. Parents cut 1.25x as much as those adults
   (median), and more in 34 of 41 rounds. They deliver 1.79x as much (40 of 41).
3. **The hand-over between rounds isn't the filter either.** At first, the ants alive at the end (the next round's
   founders) seemed to cut about 40% less (33 of 40 rounds). That was an artefact of time: cutting falls through the
   round, and the survivors' lives sit late in it. Matched to the same 10k-frame bins, survivors cut as much as the
   ants that left: observed/expected 0.96 vs 1.01 (lower in 19, higher in 16 of 35).
   **Kept here so nobody re-finds the unmatched number.**
4. **Yet the genomes lose digging.** The test: each round's final ants in a common garden, with selection off, the
   same 3 fresh worlds and 60k frames (`common.sh`):

   | seed | round 0 | later rounds |
   |---|---|---|
   | 5 | 174 cuts | r2 19, r4 0, r6 0, r10 0 |
   | 7 | 193 | r3 53, r6 0 |
   | 9 | 167 | r4 97, r10 3, r20 0 |
   | 10 | 170 | r4 97 |

   Deliveries rise as cuts fall (s5: 959 to 2,145; s9: 425 to 4,310). The loss is in the genomes, and it is fast.
5. **Where the generations fall.** Summed over the 41 rounds, cuts per 50k-frame quarter are 5,874 / 2,600 /
   1,879 / 1,155. Births per quarter are 1,180 / 1,218 / 1,274 / 1,192. Half the cutting happens in the first
   quarter, while the births are even. The colony at round end is generation 6-12.

Putting 2, 4 and 5 together: selection pushes digging up wherever ants cut, but most generations live where nobody
cuts. There an ant that *would* dig scores the same zero as one that wouldn't, while food selection keeps pulling
the brain toward foraging. This is a correlated response: top-quarter cutters deliver 3.3 vs 4.1 per 10k frames.
So any fixed dig weight only acts in the first quarter. That fits digw-1022: x4 delays the decline, it doesn't stop it.

## The fix this points to, tested small (local, c1, 8 seeds, score selection, L60c1's arguments)

The same 800k frames, run either as 4 rounds of 200k (as L60c1) or as 16 rounds of 50k. Each round is a fresh
world. Afterwards, the last round's ants go into the same common garden (`cg2.sh`, `jobs.txt`).

| | 4 x 200k | 16 x 50k |
|---|---|---|
| common-garden cuts (median of seeds; round-0 ants ~170-190) | **0** (s1 0, s2 6, s4 0, s5 0, s6 0, s7 53, s8 233; s3 extinct) | **154** (s1 0, s2 142, s3 207, s4 0, s5 196, s6 0, s7 166, s8 283) |
| common-garden deliveries (median) | 1,512 | 1,148 |
| births over the run (mean per seed) | 458 | 497 |
| in-run cuts per seed | 706 | 1,881 |
| in-run home food per seed (all rounds) | 10,522 | 20,008 |
| in-run starved per seed | 269 | 37 |

The short-round arm cut more on 5 of 8 seeds; 2 were tied at 0, and 1 long colony went extinct (sign test 5/5,
p=0.06). The number of generations is the same, so the difference is where they were bred. The in-run food and
starvation are **not** a fair comparison: a fresh world every 50k frames resets the store and ends die-offs early.
The fair food number is the common garden's, and there short rounds cost about a quarter of deliveries at 60k
frames. Some short seeds (1, 4, 6) still lost digging, so this slows the loss a great deal but does not stop it.

## What I'd suggest for the "keeps digging" floor (Teacher lane's call)

- **Shorter rounds (or a mix)**, so most generations are bred while there is ground to dig. Cheapest, no new code:
  `rounds.sh` with 50k frames. Check at 64 seeds against the 200k control, paired by seed, on hA1, with common-garden
  cuts as the dig number. The cost to watch: less selection on foraging in an old nest.
- **Or keep digging available all round**: give the colony a target that grows with its population (more rooms
  wanted as it grows), so late generations meet ground worth cutting. This is closer to a real colony, but it's a
  design change.
- **An adaptive weight on its own won't do it.** It can only act on ants that are cutting.

**A caveat on the cause.** L60c1's random-parent arm also lost digging. With no food selection, the drag can't be
food selection there, so the likelier driver is erosion: every birth carries a mutation, and a behaviour that isn't
selected in most generations decays. The fix is the same either way: digging has to be selected in most generations,
not just in the first quarter of a round. I haven't traced the random arm.

Not done: hA1 (the binary here predates the pack move), and any 64-seed run.

## Update 13:40Z: late-round ants don't stop digging, they bail their own spoil

The dig price barely moves over a round (L60c1, 41 digging rounds: 2.96 in the first 10k frames, 2.49 in the last), so
it isn't why new-ground cuts fall. What changes is what the digging is spent on. hA1L20, 8 colonies (seeds 1, 2, 4, 5,
7, 8, 9, 12), events within 30 columns of the door, per colony per 50k frames:

| round | | 0-50k | 50-100k | 100-150k | 150-200k |
|---|---|---|---|---|---|
| r0 | new-ground cuts | 146 | 66 | 55 | 49 |
| | re-cuts of loose spoil | 254 | 312 | 356 | 461 |
| | spoil let go inside | 43 | 92 | 134 | 206 |
| | pit at bin end | 43 | 69 | 97 | 109 |
| r3 | cuts / recuts / in / pit | 140/587/329/47 | 51/673/402/57 | 42/764/478/76 | 48/791/548/89 |
| r6 | cuts / recuts / in / pit | 106/469/192/32 | 34/701/320/57 | 36/811/416/64 | 33/658/372/80 |

L60c1 (c1) shows the same: new cuts 106 -> 17 per 25k, recuts 197 -> 307-341, spoil in 71 -> 170-193, pit 0 -> 72.
The crater is a treadmill: spoil falls back, ants carry it out again (`out` stays ~250-400 per 50k all round), and
that carry is what late generations are paid and bred on. New ground, i.e. rooms, is only cut in the first quarter.

Implication: the floor needs (1) the crater stopped (the refusal teacher being copied into the founder) and (2) pay for
new rooms (`nestscore::cell_values`, room credit to the original cell's cutter, PR #795). Script: `pullev.py` here.

## Update 14:20Z: spoil refusal, and 6 rounds with both refusals

`PP_SPOILVETO=R` (patch `spoilveto-vs-98ad9dc4.patch`) refuses letting go of spoil below the datum or within R
columns of the door. Over one round (hA1, 8 seeds, score selection), the last 50k frames per colony:
none / PP_RIMVETO=3 / both: re-cuts 446 / 437 / 177, spoil let go inside 203 / 221 / 1, home food 42 / 202 / 240,
starved per round 63 / 12 / 61. The spoil refusal alone starved ants (162). Script: `tmsum.py`.

6 rounds x 200k (8 seeds, `rounds.sh`, build acaac0f3), then the common garden with selection off and NO refusals
(200k frames, world seed*1000+99, the Teacher lane's protocol). The plain arm reproduces hA1L20 r5 exactly, which
checks determinism.
  cuts, plain: 108 337 113 26 152 134 136 (s6 extinct), median 134
  cuts, both:  201 (s2 extinct) 165 0 193 0 0 248, median 165
  paired: both higher on 4 seeds, lower on 2. Common-garden deliveries are higher with both on 4 of 6 seeds.
Inconclusive at 8 seeds, and bimodal: 3 of 7 'both' colonies lost digging completely, against none of 7 plain under 20 except
seed 4 (26). This needs 64 seeds before anyone builds on it.

## Update 15:00Z: the copied spoil refusal learned "never let go"; a positive drop teacher

The crater refusal lesson thread copied both refusals into hA1. The ants learned not to let go of spoil anywhere (far
drops 937 -> 583, crater 54, 8 of 32 collapsed). Its table: /mnt/project-files/crater-refusal/README.md.
`PP_SPOILDROP=D` (patch `spoilveto-spoildrop-vs-98ad9dc4.patch`, works with PP_SPOILVETO) adds the positive half: with
spoil in the jaws, above the datum, D+ columns from the door, the teacher picks a Drop when one is on offer. Acting,
one round, 8 seeds, all three on (RIMVETO=3, SPOILVETO=10, SPOILDROP=15) vs both refusals:
  outs let go 25+ columns out after 100k: 32% vs 21%; re-cuts late 140 vs 177; spoil inside 0 vs 1
  home food per 50k 574/329/212/164 vs 480/285/215/240; starved 38 vs 61; pit at the end 120 vs 74 (plain 111)
So the acting version is no worse for feeding and moves spoil further. Pit growth is unexplained. Copying is the refusal
lesson thread's call.
