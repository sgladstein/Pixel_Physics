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

**Post-merge check, and it is the round's own rule paying off.** #371's CI ran
against `16bab295`; #370 landed at `047df5c6` while it was still running, so
**the merged result is a combination CI never tested** — the conflict-free
merge again, one layer up. Verified by hand on `a9c571fd`: `docscheck` clean,
`clippy --all-targets --release --locked -D warnings` clean, `cargo test --lib
--release` **1,664 passed / 0 failed / 70 ignored**, identical to the
pre-merge baseline.

