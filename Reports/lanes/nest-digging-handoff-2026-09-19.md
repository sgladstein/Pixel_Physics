# Handoff — the nest-digging line, 2026-09-19

*Written at the end of a long session, for whoever picks this up. The plan is
`Reports/nest-digging-plan-2026-09-19.md`; this is the state, the ledger of
what is proven against what is assumed, and what to do first.*

## Branch and state

Branch `claude/sweet-tesla-ommknn`. Working tree clean at time of writing.
**Nothing in the plan is built.** Everything landed is an instrument or a
default-off switch.

| landed | what it is |
|---|---|
| `examples/digbox.rs` | The bare box. Stone, soil, air; no food, plants, weather. Deterministic — two shipped runs byte-identical. `selftest` runs four arms, two of each polarity |
| `PIXEL_PHYSICS_NEST_SITE_ROWS` | `src/sim/creature.rs` — site-based `AtNest` with a row reach. Default off, **proven bit-exact** |
| `PIXEL_PHYSICS_CROWDING_LOCAL=near\|wide` | `src/sim/creature.rs` — `Crowding` as a *local* worker density instead of the colony scalar. Default off, **proven bit-exact** |
| `digbox gate=` | Re-centres the chamber gate via `ih_slot`, harness-side |
| `digbox curvdig=` | Wires `SurfaceCurvature` to `Dig` via `io_slot`, harness-side |

**Run it like this.** Anything else and the numbers are not comparable:

```
digbox ants=1200 rate=8 w=400 soil=80 frames=20000 stops=10000,20000 trace
RAYON_NUM_THREADS=1
```

## The one thing that will mislead you

**`roofed` undercounts the nest by about three times.** It counts materially
EMPTY cells and a gallery with an ant standing in it is not empty. Use
**`room total` = roofed + open + bodies**. Measured: 157 + 165 + **692** =
1,014 against 941 cells hauled above the original surface — conservation
closes to 8%, and fails by 619 cells on `roofed` alone.

`lab::census`, `burrow_probe` and `latecensus` **all have this blind spot**
and it is not fixed. That is Stage 0 and it blocks everything, because three
results currently filed as nulls were scored on the wrong number.

## Ledger: proven, validated, assumed

**Measured here, trust it:** the four-of-five constant senses; `MoistureGrad`
reading 0.000 at every wetness; the gate's 26x compression; `AtNest` spanning
2 rows; curvature→`Dig` at 2.3x with the sign reversing; conservation at
`spoil_dumped/digs` 0.986; digging 2.11x faster than 2026-09-11 at matched
colony size with identical standing void.

**Validated against the papers this session (PubMed full text or abstract):**

- **Chambers need contents — confirmed, and the mechanism is narrower than the
  summary.** Römer & Roces 2014, [DOI](https://doi.org/10.1371/journal.pone.0097872).
  Contents are an *aggregation cue*; aggregation raises local density; density
  does the widening. Scope: one species, lab, clay arenas.
- **Tunnels dig downward — confirmed. The ~40° repose angle — NOT confirmed.**
  [DOI](https://doi.org/10.1073/pnas.2102267118). Abstract only; full text
  unavailable. It also carries a mechanism nobody had: ants pick grains under
  **low stress**, and arching makes tunnel-surface grains low-stress
  automatically. **This engine already has a per-cell load model** — see the
  plan's Stage 1.
- **No dig-face pheromone — confirmed, narrowly.** **The author is Bruce (2015),
  not Pielström & Roces** — the companion report misattributes it.
  [DOI](https://doi.org/10.1016/j.beproc.2015.10.021). n = groups of **5**, and
  the authors scope it to the *digging face* only.

**Still `[search]`-grade, not validated:** collision-rate→rest and the
t^(−1/2) decay; encounter-with-the-face as the regulator; the pellet relay of
up to ~12 workers; fresh spoil attracting digging.

## How to read papers here

**The container's egress proxy returns 403 for every journal host** — PLOS,
PMC, Europe PMC, PNAS, arXiv — and that applies to `curl` *and* `WebFetch`
alike. Do not try to route around it; the proxy README says to report it.

**The MCP connectors are the way in**: `mcp__PubMed__*` (including
`get_full_text_article`), `mcp__Consensus__*`, `mcp__bioRxiv__*`. They are
server-side and unaffected. Workflow that worked: `search_articles` →
`convert_article_ids` (PMID→PMCID) → `get_full_text_article`. Not everything
has full text; the PNAS paper returned abstract only.

**Two cautions.** A previous lane spent a night producing search-summary-grade
findings because it could not open papers — that is better than nothing and
much worse than the paper. And a content-farm page asserting seasonal ant
"digging pheromone blends" that steer vertical versus horizontal tunnels is
**fabricated**; it is exactly the feature that was nearly built.

## What to do first

1. **Stage 0** — fix the ant-blind census in the shared code, then re-score the
   three `Crowding` interventions. They may not be nulls.
2. **Stage 1** — a dig-direction bias at the call site. Measure the plain
   downward bias against the **load-based rule** (dig the least-loaded cell);
   the second is closer to the paper and free in this engine.
3. Then curvature, then spoil attraction, then contents.

**Do not** build a digging pheromone, a CO₂ field, or `Persist` as the home of
tunnel advance. The plan's §3 says why for each.

## Owner input pending

**A paper on how nest entrances are built.** Both research reports named the
entrance as the one thing the literature would not answer — function
documented, origin not — and explicitly warned against inventing a convergence
rule. The owner has a paper and will supply it. **Read it before building
anything about the entrance**, and note that our "entrance" is currently a
46-column-wide strip with nothing converging on it, which may be quietly
invalidating every other measurement in this line.

## Open PRs

- **#472** — the two biology reports (digging signals, build plan). Open, docs
  only. Contains the misattribution corrected above.
- `claude/digbox-eight-days-ago` — the then-vs-now comparison, branch pushed.
- The `claude/digging-papers-check` lane was spawned to validate the papers and
  **stopped** once the egress block was proven; the connectors made it moot.
