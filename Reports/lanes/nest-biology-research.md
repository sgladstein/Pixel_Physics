# Lane note — nest biology research

*Branch `claude/nest-biology-research`, docs only throughout: no `src/`,
`assets/`, `examples/` or test change, because `claude/laughing-davinci-f6lu1r`
and later `claude/sweet-tesla-ommknn` were live in `src/sim/creature.rs` and
`assets/species/*.ron` the whole time.*

**Everything this lane found is in its three reports. This note keeps only
what is addressed to another lane** — per `docscheck`'s own cap rule.

## What it produced

| report | holds | state |
|---|---|---|
| [`nest-biology-2026-09-19.md`](../nest-biology-2026-09-19.md) | The four commissioned areas (chamber architecture and depth, granaries, digging regulation, microclimate); **§10** the owner's ruling that the nest has no purpose here; **§11** eggs, and the correction to §10.4's pricing | **Merged, PR #468** |
| [`nest-biology-digging-signals-2026-09-19.md`](../nest-biology-digging-signals-2026-09-19.md) | The coordinator's seven questions on what ants dig by; **§0a** two citations that did not survive validation | PR #472 |
| [`nest-build-plan-2026-09-19.md`](../nest-build-plan-2026-09-19.md) | The staged build; **§0a** partly superseded and defers | PR #472 |

## For whoever picks the nest build up

**[`Reports/nest-digging-plan-2026-09-19.md`](../nest-digging-plan-2026-09-19.md)
is the plan** — it landed on `main` 2026-09-19, after mine was written. Mine defers to it in four places and its Stage 0 is blocking —
`roofed` undercounts the nest ~3x because a gallery with an ant in it is not
materially `EMPTY`, and `burrow_probe`, `latecensus` and `lab::census` all
share the blind spot. Read `nest-build-plan` §0a for the full comparison.

**Four findings of mine stay additive** (grepped their plan; zero hits each):
a dug void **already stays open** via `line_burrow` → `self_supporting`
`packedsoil`, so `burrow_probe`'s "gallery gone in 5 frames" is a *hand-carved*
void and that gate is already passed; **contents exist without eggs** via the
`live_seed` guard; the brain-input pricing (24 live slots, `mutation_rate`
re-derived in every species file, baselines void) as the argument *for* their
call-site route; and the landing discipline.

**Two citations of mine are wrong and are marked in place**: the
no-dig-face-pheromone author is **Bruce (2015)**, not Pielström & Roces; and
the **~40° repose angle is unvalidated** (abstract only — only the downward
direction is confirmed).

## Two method lessons this lane paid for

1. **A doc comment's summary of what a channel measures is a claim to check,
   not a measurement to cite.** I reported `(MoistureGrad, Dig, -0.55)` as a
   depth weight with the wrong sign, from the function's prose. It returns
   `sqrt(gx²+gy²)` — unsigned — so the term is **inert**, not inverted. This
   repo's doc comments being unusually good is what made the check feel
   unnecessary.
2. **A file-ownership check is only as current as the whole of its output.** I
   ran `--who-touched src/sim/creature.rs` and read only the LANDED half. The
   branch half named `claude/sweet-tesla-ommknn`, and I wrote a competing plan
   without reading it.

## A broken check this lane ran for hours — worth not repeating

**`git merge-tree <base> <a> <b>` (the legacy three-argument form) prefixes
its conflict markers with `+`**, because its output is diff-shaped:
`+<<<<<<< .our`, plus a `changed in both` header per file. So the obvious
pre-merge check —

```
git merge-tree $(git merge-base A B) A B | grep -c '^<<<<<<<'
```

— **returns 0 on a merge that genuinely conflicts.** I quoted "0 conflict
hunks" from it on six consecutive check-ins across seven hours; the real
merge of `main` into this branch conflicted in `Reports/README.md` the
moment it was attempted. Drop the `^` anchor, or use `git merge-tree
--write-tree` (the modern form, which exits non-zero on conflict).

**It is this repository's own worst-recurring failure wearing a new
costume** — a number that is arithmetically correct (no line *does* begin
with `<<<<<<<`) and answers a different question from the one asked. And the
tell CLAUDE.md names was right there: it was **tidy**, returning a clean 0
every single time. `scripts/branchcheck.sh` does not use `merge-tree`, so
its `BxF` numbers are unaffected; the blast radius was only my own check.

**A second instance the same hour, and this one is a rule about scope**: I
ran `docscheck` on a *conflicted* working tree and it said **clean**. It
does not parse for conflict markers, so its verdict on a half-merged tree is
meaningless. Resolve first, then check.

## State

**Read the head off PR #472**, not from here — it has gone stale four times.
A lane does not merge its own PR here (`CLAUDE.md`), and a woken lane has no
messaging tools, so this note is the whole return path. **Nothing is waiting
on this lane.**
