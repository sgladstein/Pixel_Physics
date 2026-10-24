# Training compute audit: what the VM's CPU bought

*2026-10-24. Analysis only, no new runs. Asked by the owner: are we testing the
right number of colonies and seeds, and how often did running a control arm
beside a new idea actually change the decision? Interactive version with
charts and the full decision ledger: https://claude.ai/artifact/FXPApab9z3BFE36NJ6Wvwn .
Scripts and reduced tables: `/mnt/project-files/training-efficiency/`.*

## Data

Every batch in the training bucket with results (52 batches, 2026-10-21 to
2026-10-24), streamed from `runs/*/small/*.tgz` and reduced to each arm's
`colsel.csv`, `cg.csv` and round/garden timings (`done: … in Xs`), and for the
older in-run batches each seed's `nest.csv` and `EXTINCT` marker. 209,000
colony-rounds and 83,000 garden runs; the 28 colony-selection batches alone
are 1,114 CPU-hours (about 17 hours of the 64-core VM). Spot checks reproduce
the lane notes exactly (colsel15 control 1,461 / 9 / 19 / 13.5; roof arm
4,465 / 1 / 143 / 1). Not included: h1turn-1021 (no small archives) and
farfood2-1024 (running). Calls use a two-sided sign test on per-colony
differences at p < 0.05, as the lanes did.

## Findings

**Where the CPU went (colony selection).** Idea and exploration arms 56%,
common gardens 30%, fresh control arms 8%, exact reruns 5.5% (a floor). The
garden runs after every second round (`CG_EVERY=2`, 24 gardens in 48 rounds);
decisions were read from one or two of them.

**Exact reruns.** The simulator is deterministic per build, so an arm with the
same build, founder, settings and seeds reproduces an earlier arm byte for
byte. 7.1% of all colony-rounds were such repeats: cs23ctl = cs21shape,
bp1pile = brood1ctl, cs9hcs = cs8hcs, cs10std's first 32 rounds = cs9joincr,
cs4inh replayed cs3lexlesinh's 16 rounds, hA1 ran four times (ha1, ha1next,
body, shortr), the h1 kid3 control three times (c1 ctl, oldref, strict).

**A comparison is needed.** Judging an idea arm against its own first garden
("did the colonies get better?") matched the real idea-vs-control call on only
9 of 18 decision measures; 3 came out backwards (inheritance retry nest, the
home-eat crutch's crater, plants + breeding starved), 3 were false wins and 3
missed real wins. Selection on its own improves a control a great deal, so
improvement since round 0 does not credit the idea.

**But a fresh control rarely added information.** Of 49 A/B calls in the lane
notes, 16 used an earlier arm as the comparison. Of the 31 same-batch
controls: 6 were exact reruns, 13 confirmed effects large enough that an
earlier run had settled them, 7 found no effect, and 5 changed or protected
the call (selection works at all, h1turn; the distance ramp is the cause and
costs nest, farfood1; two harness bugs, colsel4 and colsel14; and the baseline
moving after the meter change, colsel15).

**Colonies.** Subsampling the 176 pair-measures that had a call at 64
colonies: 32 colonies keep 99% of large-effect calls (mean difference over 1
spread) and 85% of medium ones (0.5–1); 16 keep 92% and 52%. No subset at any
size reversed a call; fewer colonies only turn calls into "can't tell". After
48 rounds the same colony number in two arms is uncorrelated (median r ≈
-0.01), so pairing by seed buys no precision and an earlier control is as good
as a same-batch one. Caveat: the colonies are also the selection population,
so fewer colonies changes training as well as measurement.

**Rounds.** Rule changes (roof and room rules, the inheritance retry) show in
the first garden, after two rounds. Learned changes (the light dig head)
settled by round 23–31. Erosion (nest peaking at round 15–21 then shrinking)
only shows in the full 48. Over all 48-round pair-measures, the garden after
round 15 already shows the round-47 call 58% of the time and the opposite 8%;
past round 29 almost nothing reverses.

## Recommendations

1. **Garden every 8 rounds** (`CG_EVERY=8`: rounds 7, 15, 23, 31, 39, 47).
   About 22% of colony-selection CPU.
2. **A control is a cache entry, never a rerun.** Key it by the build's
   simulation code, founder pool, settings and seeds; if the bucket holds an
   arm with that key, compare against it. Resume shared prefixes instead of
   replaying them. About 6–8%.
3. **One control per ratchet turn**, shared by every idea tested in that turn.
   Ratchet when a batch clearly beats the line, so the control lasts longer.
4. **Screen, then confirm.** 32 colonies × 16 rounds (gardens at 7 and 15),
   about a sixth of a full arm, catches every large effect seen here; promote
   finalists and anything aimed at late-game erosion to 64 × 48. Test 32 vs 64
   colonies once before making it standard.
5. **Name the deciding measure before the batch.** With five measures at
   p < 0.05, an idea that does nothing shows a "significant" change on at
   least one about one time in five.
6. **Count extinct colonies in the garden** (alive 0, nest 0): gardens drop
   them, so moving-food arms with 47–56 colonies left had survivor-only
   medians.

Recommendations 1 and 2 together would have cut about 28% of CPU with no
change to any decision recorded here.
