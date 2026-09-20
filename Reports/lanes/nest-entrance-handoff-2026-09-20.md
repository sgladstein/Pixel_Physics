# Handoff — the nest entrance, 2026-09-20

*The entrance is now **dug** at founding rather than painted. The code is in
and builds; the first numbers are in and are **not yet a result**. This note
is the state, what is proven against what is merely run, and what to do
first.*

Branch `claude/exciting-meitner-epyvsg`, off `main` at `be80ee94`.

## What this is, in one line

`World::paint_nest_patch` converts **53 columns of surface ground and
excavates zero cells** — this engine's founding gesture paints a strip
*across* where the biology's digs a shaft *down*. `PIXEL_PHYSICS_NEST_SHAFT=
<rows>` makes it dig one.

## Why it is a founding change and not a behaviour one

This is the reading the owner's entrance research settled, and it is the
part a later session is most likely to undo by accident.

**There is no convergence mechanism to build.** Both prior research reports
refused to invent one, correctly — the literature has the *function* of a
single entrance (it is an information hub; extra entrances measurably cost a
colony) and not its *origin*. The research resolves it differently: a nest
has one entrance because **a founding queen dug one hole**, and everything
grew outward from it. Roces 2012 has her controlling depth by self-motion and
elapsed time, with no external cue at all.

So nothing needs to converge if the nest *starts* as a point. That is a
founding fact, not an ant behaviour.

## What landed

`World::dig_founding_shaft`, called from `paint_nest_patch`, behind
`nest_shaft_rows()` (`PIXEL_PHYSICS_NEST_SHAFT`). **Unset is bit-exact**, so
every measurement taken on the painted door still stands.

| | from | value |
|---|---|---|
| shaft width | Gravish 2013, tunnel ≈ one body length; `ant.ron` `body: Chain(2)` | **2 cells** |
| chamber | harvester entrance chamber ~5 cm long, 2–3 cm down, tunnels converging on it; §2.5's 2–5 mm/cell | **10–25 cells long, 4–15 down** — sized off the shaft, not its own constant |
| depth | Roces 2012: idiothetic + temporal, no external cue | **a dial**, because there is no environmental quantity to read it off |

## The numbers so far, and why they are NOT a result

`digbox ants=300 rate=8 w=200 soil=60 frames=6000`, one seed:

| `NEST_SHAFT` | room bbox | `vert` | room total | digs |
|---|---|---|---|---|
| unset (painted) | 175w × 14h | 0.08 | 723 | 2,749 |
| 8 | 112w × 14h | 0.12 | 606 | 2,159 |
| 20 | 178w × 21h | 0.12 | 930 | 2,975 |
| 40 | 150w × 18h | 0.12 | 860 | 3,190 |

**Read as "the colony does not keep the hole" this would be a finding. Do not
read it that way yet.** Three things are missing and each could invert it:

1. **No positive control that the shaft was ever cut.** Nothing here censuses
   the world at frame 0. A 40-row shaft reading 18 rows of room at frame
   6,000 is equally consistent with "it filled in" and with "it was never
   dug" — `CLAUDE.md`'s standing rule, and this lane has already paid for
   ignoring it once.
2. **No picture.** The one time this line trusted a number over a render, it
   nearly reported a shaft that did not exist.
3. **One seed, one frame count.** Outcomes here are chaotic; 3 and 4 seeds
   have produced conclusions that 12 reversed past their own sign.

## What to do first

1. **Census at frame 0 and render it.** Confirm the shaft exists before
   interpreting anything about whether it survives. This is one `stops=0,...`
   run and one `out=`.
2. **Then the survival question**, ≥12 seeds, scored on `room WxH vert` from
   `SUMMARY` — **never** on the `trace` line's "spread over N columns × M
   rows", which is at-nest *ant* spread and follows its own dial by
   construction.
3. **If the shaft does not survive**, that is the concentration problem
   arriving at the entrance, and it is the same wall four other levers hit
   (`nest-shape-three-negatives-2026-09-19.md`). Do not tune the depth
   against it.

## Where it sits with the owner's ruling

`nest-design-2026-09-14.md` §13 already retires the paint — *"a nest is a
site and nothing else"* — and is not implemented. This is the constructive
half of the warning left on it: §13's prescribed replacement spells out the
*current* door dimensions (`COLONY_HALF_WIDTH`, `±2`), so building it
literally would re-enshrine a 46×2 door. A dug shaft is what should replace
the paint.

## Do not build

**An entrance-convergence rule.** Beyond the literature having no mechanism,
our geometry cannot pose the question: entrance count and the 15 cm
clustering live on the horizontal ground *surface*, which in a vertical
cross-section is a **line**. A six-entrance nest cut by our plane shows one.
Full reasoning in `nest-entrance-dimensions-2026-09-19.md`.

## The owner's open question, which outranks all of this

Review card `20260919T091527031Z-675ecd` is unanswered. Rendering what a
colony digs showed **638 cells piled above the old ground line against 137 of
void below it** — so "a shallow lens" may be the wrong complaint and "a heap
with a scrape under it" the right one. If the owner says the heap is what
bothers him, this entrance work is aimed at the wrong target and should stop
until that is settled.
