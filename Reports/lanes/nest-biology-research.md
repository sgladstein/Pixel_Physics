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

**`Reports/nest-digging-plan-2026-09-19.md` on `claude/sweet-tesla-ommknn` is
the plan.** Mine defers to it in four places and its Stage 0 is blocking —
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

## State

**Read the head off PR #472**, not from here — it has gone stale four times.
A lane does not merge its own PR here (`CLAUDE.md`), and a woken lane has no
messaging tools, so this note is the whole return path. **Nothing is waiting
on this lane.**
