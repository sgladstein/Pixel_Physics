# Lane O — one odour per nest (fission B1)

*Round twenty-nine. Design of record:
[`../evolution-lab-fission-design-2026-09-12.md`](../evolution-lab-fission-design-2026-09-12.md)
§1, §3, §5, §6, §7 B1. Owner ruling 2026-09-12: ship drift on.*

## What landed

A nest patch is a **place that holds an odour**. `World::nest_sites` carries
one `NestSite` per `paint_nest_patch` call; an ant whose `AtNest` input is 1.0
exchanges odour with the nearest one (`creature::blend_with_nest`), a
trophallaxis contact does the same between two ants, and each site's own
odour wanders once per 1,000 frames. `ant.ron` ships `scent_drift: 0.15`.

## What not to re-derive

**The odour half works, measured, and is strong.** At `scent_drift = 1.0` —
nearly seven times the shipped width, the whole population replaced every
generation, 500 generations — every ant stays within **0.123** of its nest's
odour against a tolerance radius of 1.0. With the blend removed the same bed
spreads to **3.36**, the whole axis. At the shipped 0.15 over ten generations
(a session) the cloud is **0.0153** and nothing mints. The derivation in §5
(`0.072 * |u|` after 25 contacts) predicted 0.125 and measured 0.1233; the
arithmetic is sound and does not need re-checking.

**`TRAIT_TOLERANCE` drifts at `scent_drift` too, and cohesion does not blend
it.** `SCENT_SIDE_SLOTS` is four slots — the three odour axes *and* tolerance
— so §1's claim that *no setting of `scent_drift` can make a cohered nest eat
itself* holds for the **odour** and not for the **allele that judges it**. At
drift 1.0 over 500 generations an individual's tolerance random-walks to the
bottom of its axis and it reads its own sisters as strangers: measured 911
ant-generations at a radius under 0.3, and three mints. This is not a defect
of cohesion and it is arguably selection working. It is out of scope to
"fix", and blending tolerance would erase the deliberate asymmetry
`TRAIT_TOLERANCE`'s own doc is built on (a tolerant ant walks up to an
intolerant one). **At the shipped dial and a session's depth it does not
arise at all.** Register it; do not tune it.

**§3's divergence arithmetic is a free-nest calculation, and the residents
damp it.** `gamma * s + beta * G` is conserved by the exchange, so a wander
step of `sigma` applied to the site alone relaxes to
`sigma * beta / (gamma * n + beta)` once `n` at-nest contacts have been paid.
Measured on a six-ant bed over 120 epochs: median gap **0.580** where §3 says
1.01. At the played bed's forty ants the factor is about a ninth, which is a
wander that does nothing at all — and **no setting of sigma repairs it**,
because the scale needed saturates the `[-1, 1]` allele axis and the walk
stops being diffusive.

**The repair that landed: the wander moves the gestalt, not the substrate.**
`World::carry_nest_wander` adds each site's step to every animal whose
nearest site it is, once per interval. Two cut-off nests then part at exactly
§3's `2*n*sigma^2` — median **1.19** over twelve seeds against a radius of
1.0, with a per-seed spread of **0.27 to 1.95**. That spread is why the guard
gates an order statistic. Do not "improve" this into a per-site-only wander;
it was measured and it does not work.

**One ant a thousand frames really does hold two nests together**, and by an
order of magnitude: every one of twelve seeds came back under **0.18** with a
single crossing against a cut-off median of 1.19, and family on all twelve.
Polydomy is the default outcome, as §2 predicted.

## What the owner's verdict added, 2026-09-12

Card `20260912T051541289Z-3b03d3`. His reading: *"These sound like two
different mechanisms and so i don't fully understand what is being shown in
the images"*, and the question that bounds this whole lane — *"where is the
nest/home defined as because I see ant populations moved from where they are
originally placed and just live in the plants where there is food."*

**He is right, on two beds in three, and it tracks §T2 exactly.**
`labstats` now prints a standing occupancy census every 10,000 frames. Played
bed, 120,000 frames, drift 0.15 with blending on:

| seed | deliveries | what the colony does | blends after frame 10,000 |
|---|---|---|---|
| 1 | 31 | leaves; centroid walks to 130 cells from the patch | **7**, of 2,677 |
| 2 | 9 | leaves; centroid 28–67 cells out | **8**, of 2,603 |
| 3 | 268 | **stays**; centroid 0.4–17 cells, 15–47% within 32 | **1,077**, arriving to the last window |

So **where the round trip closes the colony lives at home and cohesion is live
all run; where it does not, the colony goes to the food and cohesion covers the
founding cohort only.** Three seeds is a correlation, not a mechanism, and the
causal direction is untested. **This bounds B2 as hard as it bounds B1**: a
`Leave` verb gated on a crowded nest cannot fire on a bed where nobody is at
the nest.

**Time away does not make an ant a stranger in this model, and the owner asked
for a build in which it does.** His words: *"if he spends too much time away
from home then he turns enemy, but if he stays close to home then he keeps the
scent."* What is actually built: an ant's odour moves when it is **born**, and
coming home is what undoes it. Away from home it neither blends nor drifts, and
it still tracks its nest's wander because `carry_nest_wander` steps every animal
whose nearest site it is, at any distance. A lineage that never comes home
random-walks at `scent_drift` per generation, so at the shipped 0.15 it needs
**44 generations** to reach a tolerance radius against the 5–11 a session
reaches — the shape exists and is far too slow to see. At drift 1.0 it is about
one generation, which is why that arm has own-kills at all.

**The cheap repair, if the owner wants that shape**: gate `carry_nest_wander` on
proximity to the site, so an ant away falls behind the mound's wander with
*time*. Ants at home still track it, so the divergence guard is unaffected. At
σ = 0.065 a wholly absent ant would take 237 epochs (237,000 frames, two
sessions) to reach a radius, so σ would need re-deriving with it. **Not built —
it is a design change and the owner's answer decides it.**

**Three arms on seed 3 isolating one mechanism each** (40,000–100,000 frames,
`labgif`), which is what cards `20260912T073328717Z-172fa8` and
`20260912T074657942Z-a14201` show:

| arm | living ants | killed by own colony | mints | blends |
|---|---|---|---|---|
| drift 0, blending off | 86 | 0 | 0 | 0 |
| drift 1.0, blending off | **155** | **675** | 2 | 0 |
| drift 1.0, blending on | **32** | **181** | 2 | 50,484 |

Blending cuts own-killings by **73%**, and the colony that stops eating itself
is **a fifth the size**. On a bed that starves its colony (§Z6), 675 killings
are 675 meals. **Do not read the smaller colony as cohesion costing
population** without that term; it is the bed, and the same arm at the shipped
0.15 costs nothing at all on any seed.

**A number quoted from the wrong harness looks exactly like a result.** Card
172fa8 went out with living-ant counts carried over from `labstats` at a
different frame span — 133 and 62 against the true 86 and 155, wrong in both
directions and wrong about which colony was bigger. `labgif` now counts its own
living animals beside the kill tally. Corrected on the queue before the owner
read it; the card carries the correction.

## Environment notes

- **`cargo test --lib` will hit the stale-incremental link error** on this
  container (`rust-lld: error: undefined hidden symbol: anon.…`, referenced
  from `creature::act`). `rm -rf target/debug/incremental` clears it. It is
  `CLAUDE.md`'s documented gotcha and not a code error; it cost ten minutes
  here because the message names a real function.
- **A test bed needs `scheduler::step` beside `update::step`.** `update::step`
  begins and ends the frame and sweeps; it does **not** dispatch creature
  active sites, so a bed driven with `update::step` alone ticks no animals at
  all and every creature counter reads zero. That looks exactly like a dead
  mechanism. `App::update`'s order is `update::step` then `scheduler::step`.
- **`blend_a_lifetime`-style helpers must not filter on the colony label.** A
  `regroup_by_scent` mint *renames* the ants it splits off, so a label filter
  silently stops reaching exactly the animals a split has just made
  interesting — which reads as "cohesion failed" and is the helper failing.
  Measured: a cloud of 2.645 that was really 0.123.

## What B2 needs from here

`World::nest_sites` and `World::register_nest_site(x, y, half_width)` are what
a budded satellite pushes into. `register_nest_site` treats anything within
`half_width` of an existing centre as a repaint rather than a new nest, so
`bud_distance = 120` is comfortably outside it. `NestSite::seeded` is lazy on
purpose — the ground is painted before a founder exists — so a satellite whose
party carries the parent's odour needs no `bud_scent_offset` plumbing at all:
the first ant to stand on it seeds it.
