# The evolution lab, round thirty-one — 2026-09-13

**Coordinator's record. IN PROGRESS — the lane sections are written as they
report; do not cite an empty one as a null.** Opened against `main` at
`16bab295`, with [`evolution-lab-round-31-brief-2026-09-13.md`](evolution-lab-round-31-brief-2026-09-13.md)
as the task list. Round 30's account is
[`evolution-lab-round-30-2026-09-12.md`](evolution-lab-round-30-2026-09-12.md).

## The lanes

Five, one per task, each a cloud session on its own container. Model chosen by
the round-31 brief's rule — Opus where a wrong number compounds, Sonnet for a
bounded build with a clear acceptance test.

| lane | task | model | branch |
|---|---|---|---|
| A | T1 re-derive the ant lifespan and every played-bed number | opus | `claude/lab-lifespan-rederive-r31` |
| B | T4 floating debris, §Z18 | opus | `claude/lab-floating-spoil-r31` |
| C | T3 the MENU page reads as a list | sonnet | `claude/lab-menu-buttons-r31` |
| D | T2 the chronicle carries the playtest | sonnet | `claude/lab-chronicle-playtest-r31` |
| E | T5 §Z13 resting reads as stuck | sonnet | `claude/lab-resting-reads-r31` |

## What the PR list was hiding, and it was the round's best find

**The brief told Lane B to build a standing census of unsupported worked
ground, because §Z18 says nothing in the harness counts it. That was true of
`main` and false of the repository.**

`PR #221` — `claude/creature-plant-pathfinding-rjzkqe`, **open since
2026-09-03, never merged** — already carries it: `examples/spoil_destination.rs`
measuring tallest standing pellet with a **no-tree positive control**, and
`CreatureStats::spoil_lifted` / `spoil_lift_max`, the split that `spoil_dumped`
cannot make because it sums both placement branches. Each verified absent from
`16bab295` before Lane B was redirected onto it.

**It also names a mechanism the register does not**, and it is live on the
trunk today. `src/sim/creature.rs:6601`:

```rust
.or_else(|| (1..=SPOIL_LIFT).map(|dy| (x, y - dy)).find(|&(px, py)| open(px, py)))
```

with `const SPOIL_LIFT: i32 = 160` at `:6778`. When no 8-neighbour will hold a
pellet, the drop scans **straight up as far as 160 rows** for the first cell
that is empty with two of three filled beneath and `SPOIL_HEADROOM` clear
above — **with no check that a path exists**. The ant never climbs; the pellet
is teleported, and being `packedsoil` it is `self_supporting`, so it stays.
Measured there over four seeds: tallest standing pellet **+52 / +67 / +99 /
+94** with a tree against **+4 / +3 / +2 / +2** without.

Note what that does to §Z18's lattice specifically: **a lattice satisfies "two
of three beneath filled" for itself**, so each new pellet teleports onto the
top of the previous one and the lattice bootstraps upward with no ant ever
walking it.

**Why it was invisible, and the lesson.** Its register section was filed as
**§Z4, a letter already used and closed on `main`**. So the PR is not in the
bug register anyone reads, and `branchcheck --prs` lists its branch as having
a PR — which reads as *owned*, not as *stalled*. **A branch having a PR is not
evidence anyone is reading it.** Round 30's rule was *the PR list is not the
work list*; this is the other half — the PR list is also not the *landed*
list, and a ten-day-old open PR can hold the exact instrument a new lane is
being told to build.

## A clean merge that broke the build — PR #371

Round 31's brief warns that *the dangerous merge is the one with no conflicts*.
This is a worked instance, produced while recovering a second invisible branch.

`Reports/plant-engine-rethink-2026-09-03.md` §6.13 on `main` claimed four-fifths
of cloned plants sit in a tight band. **Its own author withdrew that on
2026-09-04** — the table measured `tree` **seedlings** at 6,000 frames, a fifth
of a generation, where a whip of ~190 cells has nothing built to differ with.
The same twelve columns at 20,000 frames: **3 of 12** within ±15% against 28 of
33, CV 0.286 → **0.374**, largest/smallest 2.7x → **22x**. The withdrawal sat on
`claude/plant-engine-rethink-brief-rk7lq5` for **eight days with no pull
request** while the trunk asserted the retracted claim.

`git merge-tree --write-tree` reported the merge clean. `Reports/instruments.md`
had **24 commits of drift** on `main` and merged correctly anyway, because the
branch's edit was confined to its own `clone_identity` row. What broke was a
file whose lines neither side touched:

```
error[E0308]: expected `&HashSet<ChunkCoord, BuildHasherDefault<FxHasher>>`
                 found `&HashSet<_, RandomState>`
```

`renderer.draw` takes `&ChunkSet`, which `main` has since aliased to
`FxHashSet<ChunkCoord>` (`src/sim/fxhash.rs:115`); the stale example still
passed a `std::collections::HashSet`. **Git merges per file and per line; a
type alias is neither.** Fixed to the idiom `examples/clone_variance.rs:209`
already uses — the sibling harness from the same report, current only because
it landed. `clippy --all-targets --release --locked -D warnings`: **101 → 0**.

## The brief's own model guidance was wrong, and it was corrected mid-round

`PR #372`, opened by the round-30 coordinator an hour after #370 landed,
retracts the model-choice section it had just written. **Fable 5.1 is
`$10.00` / `$50.00` per MTok against Opus 5's `$5.00` / `$25.00` — twice
Opus on both, the most expensive tier available here, not "three to five
times cheaper".** Sonnet 5 is `$2.00` / `$10.00`, Haiku 4.5 `$1.00` / `$5.00`.

**Verified independently against the API reference before acting on it**, per
the brief's own instruction to check a claim rather than take it. The rates
match, and so does the documented order: start on Opus 5 and escalate *up* to
Fable only for long-horizon work where Opus at higher effort has measured
short — the opposite direction from the retracted bullet. Confirmed too that
**`create_session` accepts `model` and no effort or thinking parameter**, so
for a lane the model is the whole dial and the choice carries more weight here
than generic guidance assumes.

**What went wrong is this repo's own recurring failure, one more time**: the
section inferred a *rate* from one round's *invoice*. Two Fable lanes cost $22
and $15 against an Opus lane at $105 — but those were short design lanes that
read little, while the Opus lane ran a twelve-seed paired sweep. **The invoice
measured run length, not model rate.** Arithmetically correct, and about a
different question.

**Round 31 was unaffected**: A and B on Opus, C, D and E on Sonnet, no Fable
lane. The Sonnet choices are now justified by rate — 2.5x under Opus for a
bounded build with a clear acceptance test — rather than by an invoice.

## The combined-merge probe, and what it caught

**Round 30's rule is that the dangerous merge is the conflict-free one. Round
31 acted on it *before* merging rather than after, and it paid immediately.**

With three lane branches green-or-nearly and each measured against a trunk
that lacked the other two, the coordinator merged all three into a scratch
branch off `main` and ran the suite there. **Zero conflicts** — `src/lab/ui.rs`
and `src/sim/creature.rs` both auto-merged — `clippy --all-targets` clean, and
then:

```
sim::creature::tests::digging_moves_the_ground_rather_than_eating_it
ground cells 259 -> 207 over 491 digs and 490 dumps
  left: 207   right: 262
```

**Isolated to Lane B alone** by re-running B against `main` on its own, so it
was not an interaction — it was a lane's own regression that **no CI had
reported, because that lane had not opened its PR yet.** The probe was the only
thing in the round positioned to see it.

**And the failure meant the opposite of what it said.** The test's `ground`
closure counts exactly two materials in both halves — `standing` over rows
0..=199 (`m == soil || m == packed`) and `held`, the spoil in an ant's
mandibles (`sp.cell.material == soil || sp.cell.material == packed`). Lane B's
entire approach was to introduce a **third**, `spoil`. So the 55 missing cells
are cells that are now the new material: the bed is conserved and the ruler
shrank. Read as written — *digging is still eating the bed* — it condemns a
correct change.

**That test had already survived this exact trap once, and says so in its own
comment**: *"Fixing the census rather than the engine, because the engine was
right: nothing was created, the two columns were adding up different things"*
(2026-09-05, when `held` counted **any** spoil rather than only ground spoil).
Lane B is the other direction of the identical mistake — and it had already
remembered to teach `lab::census` and `soilfork` about `spoil`, missing only
this test's own private census.

**The transferable part.** Adding a material silently changes the meaning of
every census that enumerates materials by name, and those censuses do not
announce themselves — one was inside a test. So: **when a change introduces a
new material, grep every site that enumerates the old ones as a category, and
treat a conservation failure as a question about the ruler before treating it
as a question about the engine.** Both readings are consistent with the number;
they prescribe opposite work.

**And then it happened three more times on the same branch, which is what
makes it a rule rather than an anecdote.** Lane B fixed the three sites it
could find — `lab::census`, `soilfork`, and the unit test above — opened #379,
and CI went red on `cargo run --example ascii`:

```
the bank is not conserved: 3600 -> 3519 standing, 0 in mandibles,
0 lost with their carriers, 2 rotted back from carrion
  left: 3519   right: 3602
```

The coordinator reproduced it in a worktree with the census broken into its
parts, rather than reasoning about it: **soil 3,415, packed 104, spoil 83,
summing to 3,602 — the expected total exactly.** Not approximately, not within
a tolerance: the conservation identity balances to the cell the moment the
third form of the ground is counted. That is a positive control as well as a
diagnosis, because a partial explanation could not produce an exact balance.
The 83 missing cells were pellets lying on the surface in plain sight. Lane B
reached the identical diagnosis independently, to the same 83 cells, and found
a **fifth** site the coordinator had not checked (`burrow_probe`).

**So five censuses, four of them private to one file, and the lane's own
enumeration found three.** The residue matters more than the count: the two it
missed were both in `examples/`, which is where every measurement in this repo
comes from and where nothing prompts you to look. **The grep has to be over
`examples/` too, and the sites to grep for are the *pairs* — every place an
identity names a set of materials on both sides of an equals sign.** A census
that names the set once is merely wrong; one that names it twice is wrong in a
way that still balances for the old world and breaks silently for the new.

## The lifespan constant survives, and everything that justified it is gone

**Lane A's re-take (#376) is the round's largest quantitative result, and it
overturns the register's own headline.** Everything anyone knew about how long
an ant lives was measured while #366's seed-eats-ant cull was running. 75 runs
— twelve seeds, five arms out of one binary, plus the other five shipped beds
at seeds 1–3.

- **§Z6 — "every shipped bed starves its colony inside one play session" — is
  overturned as written.** 26 of 27 shipped-bed runs hold a live colony at
  200,000 frames, against the register's 2 of 9. Left OPEN on a narrower
  claim, and correctly so: it is written at 300,000 frames, it names two beds
  not re-run, and three of the 27 end at 1, 2 and 33 ants — a colony too small
  to be one.
- **`life_half_life: 40000` stays, on entirely new evidence.** Against an
  *immortal* colony the constant no longer moves the population at all — ants
  7 of 12, plants 5 of 12, bank 6 of 12, every p ≥ 0.39. What justifies it is
  a **floor**: halving to 20,000 is the only setting in the sweep that kills
  colonies, 4 of 12 at zero by 200,000 frames. Doubling buys nothing paired.
- **Removing death makes the colony hungrier, not larger**: starvation pooled
  over twelve seeds goes 2,575 → 4,067, **+58%**, with the population flat.
- **The dig gate keeps its place on a different number.** #359 shipped it on
  *alive on 5 beds of 12 against 0*; post-fix that is 12 of 12 in both arms —
  the difference had been between two colonies that were both being culled.
  What earns it now is the bed: plants standing higher with the gate on, 10 of
  12 seeds, median +24, **p = 0.039**, and only in the second half of a
  session.

**Two coordinator readings were wrong and the lane corrected both**, which is
worth recording because the corrections were right:

- The coordinator's order statistics were computed by linear interpolation and
  the lane's by nearest rank — 8.9 / 111.0 / 286.8 against 6 / 117 / 290 for
  the same twelve seeds. **They reconcile exactly once the convention is
  stated**, and no answer turned on it. Both columns are now in the report.
- **The `deliveries` cliff is real and is not the lifespan.** The coordinator
  offered seed 1's 37-against-5,932 as a lifespan effect isolated by a paired
  control. The value arms refute the causal half: same seed, 20,000 delivers
  2,466 and 80,000 delivers 4,792, both mortal, everything else identical.
  Over twelve seeds the shipped arm is the *best* deliverer. It is a bed
  property worth a lane and it is not evidence about the constant.

## "Did it fire at all" applies to the review card, not only to the harness

**Lane E's is the round's cheapest finding and its most reusable.** Three idle
animations were built as a runtime selector, posted as a card, and the owner
answered:

> *"The creatures that I think look stuck are stuck in all of them. Although
> this is a very short gif to have to judge this on."*

Three readings were available — the animation is too subtle, the animation
never ran, or the ants are genuinely stuck. **The lane checked the second
first, because it needs no markers and no further owner time**, and it was the
answer: `IDLE_ANIM_DELAY` was counted in `Renderer::frame`, which is **draw
calls**, not world ticks. Every headless capture in this repo draws on a
*sample* of ticks (`labgif`'s `every=`), so on the posted card's own `every=10`
a 60-frame delay was **600 world ticks** — 40% of the 1,500-tick window gone
before any animal could be flagged. Counted directly: **8 of 22 full-length
long ants ever animated in the window the owner was judging.** After the fix,
**21 of 22**.

**This was live in the real game, not only in the capture tool**: `App::
update`'s catch-up loop can run several world ticks per draw, so the same
undercount reached the player.

**The rule `CLAUDE.md` already has is "when a change adds a discrete *this
happened* event, print the count next to the image and read both". This is its
missing half: the count belongs next to the image *when the verdict comes back
negative*, too.** A card whose mechanism did not run and a card whose mechanism
does not work are the same picture and the same verdict, and the round would
otherwise have spent a fourth candidate on a look that was never the problem.
**The cheap check is not "is the effect too subtle" but "did it reach the
animals in this frame, and how many".**

**The coordinator's review finding on it:** the fix is correct and unguarded.
Every idle-anim test in `src/render.rs` is an `#[ignore]`d `probe_*` — the
thirteen cost rows and the new firing probe — so nothing in CI fails if
`world.frame` goes back to `self.frame`. A guard has to advance the tick count
across **fewer draw calls than ticks**; one that draws once per tick cannot
tell the two counters apart and would be blind, which is this file's standing
rule about a guard that cannot fail for the fault it is named for.

**And the owner's three markers never arrived.** He placed them on the card and
both cards read `annotations: []` from the shared queue. So the third reading —
that the specific ants he pointed at are genuinely stuck, with `moves_blocked`
climbing — could not be checked at all this round, and is filed in §Z13 as the
open question with `HeadBlock`/`moves_blocked` named as what settles it. **A
verdict's free-text comment survives the queue and its annotations did not**,
which is a fact about the instrument worth knowing before anyone designs
another card around marker placement.

## The round's largest finding: nobody can reproduce the owner's bed

**Three separate visual complaints were investigated this round on a bed that
does not contain any of them, and nobody noticed until the owner said so.**

Lane B's §Z18 card rendered `played_bed` seed 3 at 150,000 frames — **74
ants**, described in the lane's own commit as *"the most developed nest of the
four beds measured"*. The owner's verdict:

> *"Everything looks normal is all these pictures. None of this reads as an
> ant hill though it just looks like herbs growing in dirt which is fine"*

**He plays sessions with 1000+ long ants.** Seventy-four ants in a herb patch
is not the scene his complaint is about. So every number in §Z18 — 19 cells,
90 cells, the 22-cell repair, the twelve-seed coin flip — is arithmetic about
a bed that does not show the phenomenon. The repair is real and the
measurements are honest; the *scene* was never checked against the complaint.

`CLAUDE.md` has this rule already — *when a mechanism appears inert, check the
scene still contains the situation you think it does* — and it was written for
a mechanism looking dead. **This is its other half: a scene can also fail to
contain a defect you are trying to remove**, and then a fix that does nothing
and a fix that works are indistinguishable, both reading as "looks normal".

**The scale gap is now measured rather than suspected.** Lane A's 75 runs put
the shipped bed's maximum at **408 ants** at 200,000 frames (median 111), and
the whole sweep's maximum at **1,067**, reached once, on `life_half_life:
80000` seed 8. The owner's ordinary play is at or above the top of everything
this project can currently generate.

**What this invalidates, and what it does not.** It does not invalidate the
pellet repair, `hangcensus`, its six controls, or the tint pass — the tint
pass is the round's most transferable instrument, being the only one that can
tell worked ground from plant *on sight* rather than by counting. What it
invalidates is any claim that those numbers describe what the owner sees. §Z18
now records the bed as **unreproduced**, in those words.

**The consequence for the programme is larger than §Z18.** The floating
debris, the stripped ground that never recovers, and the resting ants that
read as stuck are all reports from a bed nobody here has built. Until one
exists, a rendered card is a picture of a different world and a null result
from it means nothing. **The chronicle work (#374) is therefore the round's
critical path rather than a convenience**: one played session gives the scale,
the species mix, the horizon and the player's own actions, which is what a
reproducing bed has to be built from.

## Numbers this round established for its own use

- **Post-merge baseline**, `main` at `047df5c6`: `cargo test --lib --release`
  = **1,664 passed / 0 failed / 70 ignored**, 205 s. `CLAUDE.md` still quotes
  1,324 from 2026-09-02; the suite has grown by 340 and nothing regressed.
- **Next free bug identifiers**, over 72 refs carrying the register: **§Z19**
  and **§W8**. `--branches`, never `--check`.
- **`examples/latecensus.rs` does not parse `ants_at=`.** That argument belongs
  to `labforage` and `labshot`, and an unknown argument here is silently
  ignored. Its real surface is `scenario=`, `seed=` (a turbofish `arg::<u64>`,
  which a grep for `arg("seed")` misses), `frames=`, `sample=`, `no_colony=`
  (the paired unfed control) and **`control=selftest`**, a positive control
  already written. `latecensus scenario=played_bed frames=N` grows the bed, so
  it is a played bed by construction and needs no `ants_at=`.

## Landed this round

| PR | what |
|---|---|
| #370 | round 31's brief, two stale handoff items, first model-choice guidance |
| #371 | the withdrawn clone "tight band" finding, recovered from a branch with no PR |
| #372 | the brief's own model-pricing table, corrected against the API reference |
| #373 | Lane C — the MENU page's rows draw as buttons |
| #374 | Lane D — the chronicle records what the player did, and whether the box was slow |
| #375 | Lane E — three idle animations behind one runtime selector |
| #376 | Lane A — the ant lifespan and every played-bed number, re-taken after the seed cull |

**Still open at the time of writing**, both reviewed and both close:

| PR | what | state |
|---|---|---|
| #379 | Lane B — a dug pellet is a wall only while something is under it | census fix pushed after CI caught two more blind sites; CI re-running |
| #380 | Lane E — the idle-anim clock counts ticks, not draw calls | correct; sent back for a guard and a main merge |

**#374's merge was verified before #376's was allowed to follow it.** #374
reshaped `src/lab/census.rs` and #376 edits `examples/latecensus.rs`, which
reads it — a file pair no CI had ever seen together, since each PR was measured
against a trunk lacking the other. Probed in a worktree off the merged `main`:
zero conflicts, `cargo build --release --examples` clean, `docscheck` clean,
`bugindex --check` current. That probe is the round's third application of the
same rule and the second time it was run *before* a merge rather than after.

**Post-merge check, and it is the round's own rule paying off.** #371's CI ran
against `16bab295`; #370 landed at `047df5c6` while it was still running, so
**the merged result is a combination CI never tested** — the conflict-free
merge again, one layer up. Verified by hand on `a9c571fd`: `docscheck` clean,
`clippy --all-targets --release --locked -D warnings` clean, `cargo test --lib
--release` **1,664 passed / 0 failed / 70 ignored**, identical to the
pre-merge baseline.

