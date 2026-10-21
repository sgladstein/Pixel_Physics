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

## Where this leaves step 1

Against the design's bars: the best copy (go down only, 10.57) is in the "partial" band -- 35% of the reachable gain
(the strict floor-limited ceiling is 18.23), kept skills held. Not yet run: DAgger rounds on the go-down copy, the
self-imitation arm, the dig rule's failure traced to its cause, and a GIF. Open for the owner: whether to (a) build on
go down alone (DAgger, then a selection run from it), (b) keep working the dig and home rules into the copy (they
interfere in the shared scorer; a context sense or more capacity may be needed), or (c) treat walk home as a lesson for
selection rather than copying, since it costs fetch.
