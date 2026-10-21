# Step 1, interim: the copier is built and validated; what copying the teacher does so far

*2026-10-21, by the teacher-approach session. Status: **interim** -- the copier (design v2,
[`step1-copy-design-2026-10-21.md`](step1-copy-design-2026-10-21.md)) is built and passes its controls; round 0 and one
DAgger round have run on the teacher arm; the self-imitation arm's labels are built and checked but it has not run.
Everything local, no VMs. Code: `measurements/copier/` (scratch, never for main); configs and per-world results:
`measurements/step1-runs/`.*

## The answer, first

**The copier works: it can move s161 most of the way to a reachable target's behaviour.** Copying the teacher works
**for one rule -- "go down" -- which on its own takes s161 from 6.42 to 10.57 full loads a world (+4.16 +- 0.48)
with fetch and trail intact.** The other two rules do not copy cleanly: copied alone, "dig only at the foot" collapses
the ant (3.35), and "walk home" breaks fetch (an empty forager learns to head home instead of out to the food). Copied
together, the lessons interfere inside the brain's one shared scorer; the best combined copy so far is 8.64 with fetch
and trail badly hurt. The best copy, go-down only, sits in the design's "partial" band (35% of the reachable gain;
the bar is 50%).

## The copier and its checks

Built per design v2: a full-precision log of every option of every decision with the teacher's state labels
(`copylog`, `PP_COPY_LOG`, `PP_TEACH_LABEL`), a replay of the brain's real `decide` on logged states for any genome
(`copyreplay`), and the copier itself (`copier.py`: numpy + autograd; the ceiling policy as the copy target; KL on
every decision; live links plus an allow-list; one-step memory path; per-group weights; held-out split by world).

| Check | Result |
|---|---|
| Forward pass vs Rust `decide`, s161 and a perturbed genome (26,401 decisions) | max error 2.6e-5 (float32 rounding); PASS |
| One-step memory path vs logged memory (112,082 pairs) | max error 4.4e-6; PASS |
| Genome round trip through the real loader | identical full loads on all 48 worlds |
| `copylog` vs `reliab`; log on vs off | identical full loads, world by world |
| Label-only teacher vs acting teacher (8 worlds) | vetoes 210 = 210; forces 63 = 63 |
| Gradients vs finite differences | agree to 4 decimals on each output's largest slots |
| Self-imitation pellet rebuild vs the scorer | 326 = 326 full loads |

**The positive and return-to-start controls failed first, for a reason worth keeping.** Copying s162's policy into
s161 matched it 0.91-0.97 on held-out decisions and scored 1.39 in the world (s162: 7.50): s162's choices depend on its
*own* memory, and the copy keeps s161's memory weights (frozen by design), so it fed s162's rules with memory written
in s161's code. The perturbed start of the return control had jittered the frozen memory weights too. With targets the
copier can actually reach (changes to the trainable blocks only):

| Control | Start | Target | Copy | Share of the gap closed |
|---|---|---|---|---|
| Positive: s161 -> pS22 (s161, trainable weights jittered by 0.1) | 6.42 | 8.55 | **8.19** | 0.83 |
| Return: pS21 -> s161 | 3.45 | 6.42 | **6.08** | 0.89 |
| Shuffled teacher labels (within decision / within world) | 6.42 | -- | 0.55 | no gain (destroys) |

So the copier moves this brain's *behaviour*, not just its probabilities on logged states, from one round of data.
The teacher's policy is built from s161's own probabilities, so it speaks s161's memory code.

## Copying the teacher

48 seeds = 96 worlds, 8 rows, 12,000 frames, standard errors clustered by seed; fetch and trail on 48 fixed worlds
each (s91 24.23 / 0.864; s161 16.66 / 0.640 = 0.69 / 0.74 of s91). Baseline: **s161 with its unread units quieted,
6.42** (the copier zeroes the inputs of units nothing reads: 706 -> 522 live links; fetch and trail unchanged).

| Copy | Soil full loads | vs 6.42 | Fetch | Trail |
|---|---|---|---|---|
| all three rules, equal weights (round 0) | 5.85 | -0.56 | **1.05** | 0.674 |
| all three, anchors x5, must-not-dig x10 (round 0b) | 8.64 | +2.22 +- 0.43 | 8.97 | 0.369 |
| all three, DAgger round 1, kept skills x10 | 6.34 | -0.07 | 9.76 | 0.630 |
| **go down only** (round-0b weights) | **10.57** | **+4.16 +- 0.48** | **17.11** | 0.611 |
| walk home only | 7.03 | +0.61 | **11.46** | 0.632 |
| dig only | **3.35** (15 seeds under 3) | -3.06 | 18.08 | 0.630 |
| dig + go down | 9.32 | +2.91 +- 0.47 | 11.50 | 0.602 |

What the traces show:

- **Walk home generalises past where it was taught.** Round 0's copy carried loads on the surface 47% of the time
  (s161: 17%) and put them down 1% of the time (s161: 6%): "head for the mouth" had spread from empty ants to loaded
  ones, which then never reached a spot clear of the hole. The same spread breaks fetch: an empty forager walks home.
  The anchors (decisions where the target is s161 itself, and fetch/trail decisions) hold it back only partly, even at
  10x; the copies that leave out walk home keep fetch.
- **Dig copies only half.** The copy digs at the foot 80-91% of the time (s161: 24%), but its wall-dig rate barely
  moves (P(Dig) with no foot cell in reach 0.012 -> 0.004-0.010), and copied alone it collapses the ant -- the same
  coupling the ceiling test found (dig needs descent beside it), now in the copy.
- **Go down copies cleanly**, probably because the step options carry `DEPTH` directly (probe: 1.000), so the lesson
  sits in a few scorer weights that fetch and trail do not lean on.
- **The lessons interfere.** Dig + go down (9.32) is below go down alone (10.57) and costs fetch, though neither does
  alone: one shared scorer serves every verb and arena.

## Against selection, on the same worlds

The brain lane reproduced the ceiling (s161 6.80 alone, 20.66 with the teacher) and reported that 100 generations of
selection at 8 rows from s161 (s171, s172; about 8 VM-hours, ~$3.50) reach 9.47 and 9.50 alone, with the teacher's
ceiling unchanged at ~20.5. On this report's worlds:

| Brain | Soil full loads | Fetch (share of s91) | Trail (share of s91) | Cost |
|---|---|---|---|---|
| s161, quieted | 6.42 +- 0.28 | 16.66 (0.69) | 0.640 (0.74) | -- |
| s171, 100 generations of selection | 9.47 +- 0.41 | 19.64 (0.81) | 0.796 (0.92) | ~8 VM-hours |
| s161 + copied "go down" | **10.57 +- 0.39** | 17.11 (0.71) | 0.611 (0.71) | ~1 local CPU-hour |

Copying edges selection on the dig score (+1.1, about two standard errors); selection holds kept skills better (its
runs had the kept-skill floor). They look complementary: copy a lesson in, then select with the floor from the copy --
which needs the copy's revived links protected (slot 1498 and the `DEPTH` links 654, 699, 789, 834, 924, 1059, 1149,
1239; the brain lane has offered a named-slot protect list). The brain lane also suggests copying into s171, whose
ceiling is the same.

## Since the interim: is the copy repeatable, does DAgger help, and is s171 a better start?

**The go-down copy is a repeatable result, not a lucky one.** The same copy retrained with two more seeds (different
batches and initial order, same data and settings), judged on the same 96 worlds:

| Go-down copy | Full loads | vs 6.42 | Fetch (s161q 16.66) | Fetch, paired vs s161q |
|---|---|---|---|---|
| seed 1 (the one above) | 10.57 ± 0.39 | +4.16 ± 0.48 | 17.11 | +0.46 ± 1.13 |
| seed 2 | 10.60 ± 0.36 | +4.19 ± 0.46 | 14.75 | −1.90 ± 1.09 |
| seed 3 | 9.80 ± 0.34 | +3.39 ± 0.45 | 15.42 | −1.24 ± 0.91 |

All three revive the same nine links (slot 1498 and the eight `DEPTH` links). The soil gain spreads by about 0.8
between copies; the fetch cost spreads more (+0.5 to −1.9). No single copy's fetch change is outside about two
standard errors, but two of three are below the start, so **"kept skills intact" should read "fetch roughly held, at
a cost of up to ~2 trips on some copies"**. The fetch errors are not seed-clustered (48 fixed worlds), so they are a
little optimistic.

**One DAgger round on the go-down copy made it worse**: relabelling the copy's own states and copying again from s161
on the pooled data gave 9.50 full loads (against 10.57), fetch 15.32, trail 0.598. One round is not a verdict on DAgger,
but it is no reason to spend more rounds on this lesson before the others work.

**s171 has the same headroom as s161, rule by rule.** s171 (100 generations of selection at 8 rows; 9.47 alone) with
each rule acting alone on the same 96 worlds:

| Teacher acting | s171: full loads | s171: gain | s161: gain (measurements, 6.80 alone) |
|---|---|---|---|
| none | 9.47 | -- | -- |
| dig at the foot | 15.22 | +5.75 | +4.74 |
| go down | 12.11 | +2.65 | +3.41 |
| walk home | 14.86 | +5.40 | +1.56 |
| dig + go down | 16.23 | +6.76 | +10.11 |
| all three | 20.50 | +11.03 | +13.74 (20.54) |

The all-rules ceiling is the same as s161's (20.50 against 20.54-20.66), so **selection raised the floor without
raising the ceiling**. Rule by rule the headroom moved: go down is worth less on s171 (+2.65 against +3.41; selection
already taught some of it), and **walk home is worth three times as much** (+5.40 against +1.56), so on s171 it no
longer needs go-down beside it to pay. **So copying go-down into s171 would buy less than it bought s161**; the bigger
prizes on s171 are dig and walk home, the two lessons that did not copy cleanly into s161. Whether they copy into s171
is the next test worth running, with the context probe of the teacher design (section 7) first, since walk home broke
fetch by spreading to foragers.

## The copy handed to the brain lane: go-down into s171

For the brain lane's copy-then-select test (owner-approved): go-down copied into s171, fetch and trail anchored, the
`f::ROOF_DROP` links frozen (the brain lane's request: load-bearing for s171). Soil on my 96 worlds: **9.47 -> 11.64
(+2.17 +- 0.55)**, about 80% of go-down's acting gain on s171 (+2.65). Genome:
`measurements/step1-runs/s171-copy/s171-down.txt`; revived links 699, 789, 834, 924, 1059, 1149, 1239 (`f::DEPTH`
into live units) and 1498.

**Fetch: first reported as meeting the floor, and it does not.** On my 48 fetch worlds it read 20.08 against s171's
19.64. The brain lane scored it with the trainer's own harness (`train brain=mind challenge=fetch dist=10,30
budget=1200 holdout=96 noise=1 score=`), 96 fresh worlds, each mirrored pair folded to its worse side as the trainer's
floor is applied -- and reproduced here:

| | Fetch mean (96) | Worse side (48 pairs) | Soil, brain lane's 96 fresh worlds |
|---|---|---|---|
| s171 | 20.57 | 19.34 +- 0.57 | 10.90 |
| copy handed over | 19.37 | 17.62 +- 0.80 | 11.58 (+0.69 +- 0.53) |
| 4x-anchor copy | 18.94 | 16.99 +- 0.94 | -- |

The trainer's floor is 0.8 x keepref's calibration score (22.015) = 17.61, so the copy sits on it, which is not
"meets". Two lessons kept: **judge a kept skill with the harness that enforces it**, folded the way it folds; and on
these worlds the soil gain is about half what mine showed (+0.69 against +2.17), so a 96-world set is not one number
either. **The fix (copy v2, `s171-down-v2.txt`).** Heavier fetch anchors alone made it worse (worse side 15.76 at 10x, 17.51
at 20x): the loss was a few total failures -- the copy's forager, empty and inside the nest's chamber below ground,
kept taking the deepest step (go-down's trigger) and never came out (step 0.5 §6). What fixed it: those look-alike
fetch decisions (an empty ant below ground) as their own anchor group at 20x, drawn from s171's fetch runs on 80
worlds and the copy's own, **plus one revived scorer unit**; `f::ROOF_DROP` frozen and never seeded.

| s171 copy | Soil, my 96 worlds (s171 9.47) | Fetch worse side, trainer's harness (s171 19.34; floor 17.61) | Fetch worlds under 5 (of 96) | Trail worse side (s171 0.745) |
|---|---|---|---|---|
| v1, handed over first | 11.64 (+2.17 +- 0.55) | 17.62 +- 0.80 | 5 | -- |
| look-alike anchor only | 10.36 (+0.90 +- 0.58) | 18.59 +- 0.72 | 3 | -- |
| **v2: look-alike anchor + one unit** | **10.71 (+1.24 +- 0.51)** | **19.83 +- 0.26** | **0** | **0.731 +- 0.016** |

v2 holds fetch better than s171 itself: same 96 layouts, mean 20.86 against 20.57, and on s171's own two failure
worlds it scores 20.4 and 18.1 (s171: 1.7 and 3.6) -- the anchor taught "climb out of the nest" more firmly than s171
had it. The price is about half the soil gain. Its 83 revived links (the 8 go-down links, unit 0's inputs, verb row and
output) are in `s171-down-v2-revived.txt`.

## Where this leaves step 1

Against the design's bars: the best copy (go down only, 10.57; repeated at 10.60 and 9.80) is in the "partial" band --
35% of the reachable gain (the strict floor-limited ceiling is 18.23), fetch roughly held. GIF:
`measurements/s161-vs-copy-down-world2.gif` (sealed world 2: s161's scattered wall cuts against the copy's cuts
concentrated at the foot). One DAgger round on it did not help (9.50). Not yet run: the self-imitation arm, the dig
rule's failure traced to its cause, and copying into s171.

What comes next is in [`teacher-design-2026-10-21.md`](teacher-design-2026-10-21.md) (under review): keep these three
rules as the 8-row teacher and the examiner of later ones; replace walk home with a carry path that only labels loaded
ants; copy dig only after go-down, with a regularised target; and test a context probe (can the ant tell an empty
digger from an empty forager?) before copying a lesson that could spread. On s171 the larger prizes are dig and walk
home, not go down.
