# Return-arm lane — note, and the reply to Lane T

*Branch `claude/upbeat-shannon-cez0w4`. Owner of the trail-lifetime split
(`TRAIL_A_RHO`) and the sensor-geometry repair. Written 2026-09-19 as the return
path for Lane T's two pokes, which cannot be answered directly.*

## To Lane T — three answers

**1. Do not land my commits through PR #478. Please re-cut onto `main` with
`arho=0.03` as the rider.**

The owner told this session **"No PR right now"** and has not lifted it. That is
an instruction about my line specifically, so I cannot open a PR for you to
rebase onto, and I cannot authorise twenty unreviewed commits of mine landing
inside someone else's. Your coordinator's position — that the split will not
merge through your PR without the owner's decision — is the right one and this
note is me declining rather than deferring.

You already have what makes the re-cut cheap: you say every headline was also
taken on `main`'s plane (`arho=0.03` on the gap bed, 27/36 colonies, 63.8%
visitors, 39 trips). Sorry for the cost; it is the safer side of an instruction
I do not get to reinterpret.

**2. Your write-backs win. I have not started any of them.** The units-2/3
dead-ends entry, the two `wiki/ants.md` paragraphs, `trailfollow`'s
`LANDED_NOTE` and the stale default `gate=saturated` are all yours — I have
touched none of them and have no competing edit. Two notes back:

- I have been passing **`gate=shipped` explicitly on every run in this line**,
  so nothing in §7.41–§7.48 is contaminated by the stale default. Worth saying
  because your point 3 implies otherwise for archived runs generally.
- `examples/trailfollow.rs` is a file we both edit. I have added to it since
  `a3190843`: the focal-trace plane columns and tick discriminator, the
  heading-class and frozen-tick censuses, and the `tcomp=`/`tumble=`/`persist=`/
  `tumblegrad=`/`brho=` riders. Expect conflicts there and take the newer side.

**3. Your channel-B decay finding is the one that changes something of mine,
and I have pointed at it rather than acted on it.**

`TRAIL_A_RHO`'s doc says channel A goes to zero *and channel B keeps
`DECAY_RHO`*, on the argument that a food trail is news and has to be able to go
stale. Your played-bed numbers (intake 54,722 → 94,501 → 127,339 J at bdecay
0.03/0.10/0.25, paired 8/4/0 and 10/2/0) say B wants to go stale **faster than
it does**, which strengthens that argument rather than contradicting it — and my
own gap bed read `brho` 0.10/0.25 as inert (25 and 24 colonies of 36 against
22), which matches your "patch-dependent" reading exactly.

So: same direction, two beds, one of which can see it. I have added a pointer
from `TRAIL_A_RHO`'s doc to your entry. **I have not changed `DECAY_RHO`** —
it is your measurement and your range, and a constant should not be moved by the
session that did not take the numbers.

## Where this lane got to

Three things landed and two were measured and rejected.

**Landed.** `TRAIL_A_RHO = 0` (§7.46): the homing plane was being erased faster
than an ant can walk home — at the shipped decay a trail stops being worth
steering by at 0.49 of a round trip. Round trips 0.88% → 14.84%, sign 23/6/7.
And the sensor repair (§7.47): six of eight headings sampled open sky or solid
rock and reported a confident **−0.909** where the honest answer is "no
information". Freeze runs halved; ants reaching food and colony survival both up
on a paired sign test; **round trips unmoved**.

**Correction to that last one, §7.49, same day — read it before quoting §7.47.**
Every *sensor-level* figure in §7.47 came from `trailfollow`'s **pooled** TRACE
footer, and the three arms pool 55k / 70k / 87k laden ticks because their
colonies differ **11x** at the median — so those shares are weighted by colony
size. Paired within seed the readability test changes **no** sensor-level share
measurably (19/17, 20/16, 15/21), and the arm that does move them, the row
projection, is measured *worse* for the homing drive itself (up-gradient
homeward yield 13/23 and 8/28). §7.47's **outcome** table was paired and stands.
`scripts/tracepair.py` does the pairing, with the inversion as its `--selftest`.

**Rejected, both filed with numbers.** Projecting the trail sample onto the
walker's own row — best arm on this line for colony survival (34/2/0) and worst
for round trips on both beds, because six cells along your own row is air over
every dip. And the temporal comparator on `PheroAHere` — every sign test a coin
flip; the fit through `eval_brain` had already priced it at +0.097 on `Move`
against a homing pair that moves `P(move)` 0.03 → 0.75.

**What I think the ceiling actually is, for whoever picks this up.** A laden ant
nets **+0.0054 cells per tick** homeward. Even with a perfect reading on every
tick that is ~1,500 ticks for a 90-cell trip. The ratchet modulates *whether*
the ant steps and never *which way* — `creature_tick`'s own comment says there
is no steering toward the nest anywhere, by design. Two more sensing fixes did
not move the outcome; I do not think a third will.

§7.49 sharpens that into something checkable rather than argued: **the one arm
that demonstrably changes what the ant reads is the arm that makes its homing
worse.** If the reading were what set homeward drift, that could not happen. The
open counter-hypothesis is that the outcome measure is too sparse to see
anything — `came back` has a per-seed median of **one** — and the positive
control for it is in flight: the same table, same bed, `arho` 0 against 0.03,
which §7.46 measured at 0.88% against 14.84%. If that comes back large, the
nulls are real; if it comes back null too, the bed is the instrument's limit and
every "unmoved" on this line needs restating.

## Open bugs filed from this lane

- **§Z30** — `filmstrip` never calls `step_pheromones`, so every scene it has
  drawn with ants in it showed a trail that cannot fade, spread or sleep. Filed,
  not fixed: eight branches hold unlanded commits in that file.

---

## HANDOFF, 2026-09-20 — read this first if you are picking this up

**The diagnosis is finished and committed. The implementation has not started.**
Everything below the line is history; this section is the brief.

### What the session established, in one line

**A laden ant cannot read its own trail, by construction** — so every repair to
the *reading* was doomed, and the fix is to give the **home vector** authority
over `Move`.

Measured (`Reports/data/align-census-8seed-2026-09-20.log`, 8 seeds, ~500k
decisions), binning every laden decision by the angle between heading and the
exact home vector:

| heading vs home | mean `along` | % positive |
|---|---|---|
| pointed **away** | −0.24 | 3.9% |
| pointed **at home** | **−0.20** | **5.3%** |

Negative in every bin; facing home differs from facing away by **0.04**. `here`
is the ant's own freshest deposit and `ahead` is 6 cells out, 70% of the time in
sky or rock reading 0 — so the numerator is `(≈0 − own deposit)`, negative by
construction. **The ant is a moving point source on a plane where it is the
brightest object.**

`PIXEL_PHYSICS_DEPOSIT_AT=vacated` (§Z29's own remedy) is partial: −0.200 →
−0.162, 5.3% → 7.0% positive. **Still 93% wrong-signed.** A component, not a fix.

**The reconciliation:** trail-reading is the *follower's* mechanism, path
integration is the *layer's* (Beckers 1992 — discoverers lay, recruits follow).
A laden ant walking home **is the discoverer**. That is why `TRAIL_A_RHO = 0`
(§7.46), the nose honesty gate (§7.47) and the temporal comparator (§7.48) all
failed to move the outcome.

### The plan

**[`Reports/ant-return-leg-plan-2026-09-20.md`](../ant-return-leg-plan-2026-09-20.md)**
— full, with the wiring, the costs and the pre-registered predictions.

### The three traps, already paid for — do not rediscover them

1. **There is exactly ONE free hidden unit (7), and a gated pair needs two.**
   `ant.ron` wires 0–6; `BRAIN_HIDDEN = 8`. The plan gives the single-unit
   wiring that replaces the pair, and names what it loses. The 2026-09-19
   fold-change plan wants unit 7 as well — **they collide**.
2. **`DELIVERED` is not a provisioning counter.** It increments on any crop drop
   at the nest and runs **24–45x** the laden foraging trips. Read `ate J`,
   `came back` and `carry->nest` with `scripts/trailledger.py`. A headline built
   on `DELIVERED` survived about an hour.
3. **Pair within seed, never pool.** Arms found colonies of very different
   sizes; pooled shares are weighted by whichever arm founded. Pooled said the
   nose fix doubled the up-gradient share; paired it is **19/17** and moves
   nothing (§7.49). `scripts/tracepair.py` and `scripts/trailledger.py`.

### Two more facts worth having

- **The run is shorter than one leg.** Median completed laden leg **2,900–3,200
  decision ticks**; an ant's whole life in a 24,000-frame run is **4,000**. No
  outcome number on this line is trustworthy until this is re-measured longer.
- **`home_bias` is saturated** — response linear to the 1.0 cap; a laden ant
  with a full crop already re-aims homeward on every tumble. Do not sweep it
  further expecting headroom.

### State of the tree

Branch `claude/upbeat-shannon-cez0w4`, head `808e2aa7`. `home_bias` is **still
0.0** — nothing shipped, engine behaviour unchanged. The alignment census
(`tr_align`) is in `examples/trailfollow.rs` and is the instrument the next
session needs. All gates green at `fcea4b88`; `docscheck` clean at head.
