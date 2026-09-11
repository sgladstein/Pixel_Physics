# Triage rubric for `Reports/dead-ends.md`

## The question

For each entry: **was the *idea* wrong, or was the *test* wrong?**

A rejection is a measurement made at a moment. Three things can make it wrong
without the idea being wrong:

- the **instrument** was wrong (it counted the wrong thing, or could not have
  moved, or was noisier than the effect);
- the experiment was **confounded** (a rider landed with the mechanism, a
  neighbouring subsystem interfered, or the constants were calibrated against
  the old behaviour);
- the **condition expired** — the rejection was correct then and is not now.

You are NOT asked whether the idea is good. You are asked whether the recorded
evidence actually supports the rejection *today*.

## Labels — assign exactly one per entry

Decide in this order. The three questions are different questions, and a later
one cannot overrule an earlier one.

### Question 1 — what kind of thing is this? (answer first, it beats everything)

| label | assign when |
|---|---|
| `META` | **the thing being rejected *is* the instrument.** Reviving it would change a measurement, not the engine: a harness, a counter, a census predicate, a sweep method, a process. An entry that rejects an engine mechanism **on the strength of** a measurement is NOT `META`, however much of it is about the measurement — that is `SUSPECT-INSTRUMENT` or `CONFOUNDED`. A withdrawn *inference* with no mechanism attached IS `META`. |
| `UNBUILT` | **never built**, and rejected at design time on cost, scope or preference — no measurement was taken and none is owed. If it was built and measured, it is not `UNBUILT` however the entry frames the decision. If it was never built because it is *impossible*, that is `DEAD`, not `UNBUILT` — filing an impossibility as a backlog item manufactures work. |

`META` beats `UNBUILT`: a proposed-and-declined *process metric* is `META`.

### Question 2 — has this already been resolved since it was written?

The register is **annotated in place**. An entry's `Re-test when:` clause may
already record its own re-test, often in bold, often with the superseded clause
struck through. Read the whole clause before labelling — **the phrase "condition
met" alone tells you nothing about the outcome.**

| label | assign when |
|---|---|
| `LANDED` | the entry records its own revival — retried, it worked, and it shipped. No action but to move it out of the register. |
| `RE-TESTED` | the condition was met, the re-test ran, and the answer was **still no**. Not revivable: the evidence is now *stronger* than when it was written. |
| `EXPIRED` | the condition the author named has since been met, **and no re-test is recorded**. This is the highest-value class, because the author named the condition themselves and nobody has checked it. |

Watch for the third shape: a condition that was met and made the rejection
**stricter** rather than reopening it. That is `RE-TESTED`, not `EXPIRED`.

### Question 3 — if it is still open, does the recorded evidence support it?

| label | assign when |
|---|---|
| `UNWIRED` | **the mechanism was built and works, and nothing in the world ever asked it to run.** A specific nameable artifact is missing — a genome wire, a species using the verb, a material carrying the flag, a scene containing the situation — and adding it is a smaller job than rebuilding the mechanism. Assign only when **all four** hold: (1) the negative is a **null** (a zero, or no measurable change), not an observed bad behaviour; (2) you can **name the missing artifact**, "no species has a `Bias -> Impulse` wire", not "the world was not suitable"; (3) the entry does **not itself attribute the null to the world** — if it does it has already been triaged, and is `EXPIRED` where the author named the missing thing as a re-test condition, `DEAD` where that thing cannot exist in this game; (4) rule A holds — delete the null and **no reason to reject survives**. |
| `SUSPECT-INSTRUMENT` | the recorded failure mode is a named trap below, **or** the effect is inside the instrument's noise floor. |
| `CONFOUNDED` | a rider rode along with the mechanism; a neighbouring subsystem produced the failure; the mechanism was measured at constants calibrated for the behaviour it replaced; or it read a shared engine channel whose **resolution or update rate** could not carry the distinction it needed. |
| `COSTED` | measured end-to-end, paired, genuinely negative on cost — **and the entry names a condition under which the arithmetic changes.** Reopenable, but only by that condition, not by tuning. Do not bury these in `DEAD`. |
| `DEAD` | a **structural** reason: contradictory requirements, a counterexample, an arithmetic impossibility, or a strictly-better superseding mechanism. |

**The `UNWIRED` discriminator is one question: did the mechanism execute?** If
it ran and the number could not see the result, that is `SUSPECT-INSTRUMENT`. If
it ran and something present suppressed the result, `CONFOUNDED`. If it ran and
produced nothing worth having, `DEAD` or `COSTED`. If nothing was built,
`UNBUILT`. `UNWIRED` is only for **never executed, and you can name what would
have called it** — and your reason line must name that artifact, which turns the
label into a work item rather than an opinion.

Two cases that look like `UNWIRED` and are not: a mechanism that ran but never
met the population it was built for (a disjoint call set is a design fault in
the mechanism — `DEAD`), and a world in a *state* that hid the effect
(`SUSPECT-INSTRUMENT`).

**Expect `UNWIRED` to be rare, and do not force it.** It is not a category of
failure but a category of *unnoticed* failure — where the author noticed and
wrote the missing thing into `Re-test when:`, Question 2 has already claimed it
as `EXPIRED`. Three calibration rounds over 54 entries produced none. That is
the register working, not the label failing.

### The two rules that decide most hard cases

**A. Is the suspect number load-bearing?** `SUSPECT-INSTRUMENT` and `CONFOUNDED`
win only when the flawed number is what the rejection *rests on*. Test: **delete
the suspect number — does the entry still reject?** If yes, label it for the
reasoning that survives (`DEAD` or `COSTED`), not for the bad number beside it.

**B. Confident prose is not a structural reason.** This register is written in a
maximally confident house voice — "Never", "Unconditional", "Permanent", "Do not
write a fourth". That is style, not evidence. **Your reason line must name the
structure**: the contradiction, the counterexample, the arithmetic, or the
superseding mechanism. If you cannot name it, the label is not `DEAD`.

Expect roughly one entry in six to be `META`. If your `DEAD` rate is running
much above half, rule B is not being applied.

## The named traps (from CLAUDE.md's Method section)

Each of these has cost this project real time. If the entry's recorded reason
for failure matches one, that is `SUSPECT-INSTRUMENT`.

**Instrument counted the wrong thing**
- a mean over *events* is not a mean over the thing you care about
- liquids: cell *count* measures spreading, not volume; measure column volume
- dark or torn rows: measure *fill*, not occupancy
- excavation: standing void is not *dug* void (it reverses the sign)
- destruction: a *failure* count is not a *damage* count
- a standing-quantity census taken while the process replenishing it is running
- attributing *creation* when the artifact that persists came from elsewhere

**Instrument could not have moved (a vacuous null)**

> **These are traps only when the identity is UNEXPLAINED.** An entry that
> reaches its conclusion *through* a bit-identical or exactly-zero result which
> it explains mechanically — multiplication by zero, a clamp at the draw site,
> redundancy by construction — and that shows some instrument which *did* move,
> has produced a finding, not a defect. Labelling those `SUSPECT-INSTRUMENT` is
> the worst error available here: it sends someone to re-measure arithmetic.
>
> **This rule bears on `SUSPECT-INSTRUMENT` only. It must never be used to rule
> out `UNWIRED`.** The two are about different things: this asks whether the
> *instrument* could have moved, `UNWIRED` asks whether the *world* could
> have produced the effect for it to measure. A zero that is fully explained by
> "nothing in the world exercised the mechanism" is the `UNWIRED` case, not
> a counter-example to it — the exemplar's own zero was mechanically explained
> too, and the explanation was that no species carried the wire.

- byte-identical / bit-identical output across a change that must have moved
  something → the knob was never connected, or the binary was stale
- a counter that reads zero on every frame → the mechanism never fired
- an exactly-zero delta → the condition it keyed on was degenerate
- an unknown argument silently ignored by the harness
- `cargo build --release` does not rebuild examples; a stale binary prints
  plausible numbers with a newer mtime than its source
- a guard that is *blind*: green is its default state
- an override applied *after* the derived artifact is built — a genome compiled
  at load, a table baked at startup — changes nothing and reads as a dead lever

**Instrument was noisier than the effect** (measured floors)
- `labbatch`: seed alone moves the lab census **2.42–3.12×** with no true effect
- `selection_arena`: resolution floor **~9.3 share-points per seed**
- `labsoil`: **2.32× plant cells / 3.12× plants** over 12 seeds at fixed settings
- wall-clock timing on an unquiet box: two runs of a byte-identical binary
  disagreed **2.42×**; a counter downstream of `parallel.rs` swung 2.2× on load
- **six seeds is not a sweep**: 1.64× over the first six became 1.08× over the
  next twelve, pooling to a per-seed median of zero
- the tell when there is no control: **tidiness**. A clean first result here is
  evidence of an artifact, not of a strong effect.
- **most instruments have no published floor.** Then use the entry's own
  per-seed spread. If it publishes fewer than six seeds and no spread at all,
  treat a null as `SUSPECT-INSTRUMENT`.

**Census taken at the wrong moment**
- a cascade censused before it settles reads a *delay* as damage.
  `seedsweep.sh`'s default `FRAMES` stops at frame **1,202** — mid-collapse —
  and misses two of eight collapses outright. Read `rock`, not `cells lost`.
- sized *after* the system responded rather than at the event: one charge read
  **369** wrong cells at +5 frames and **67,100** at +1,300. The late number
  sizes the fix wrongly by two orders of magnitude.
- sampled at an arbitrary phase of a designed oscillator (day/night, water
  cycle, weather). `cells lost` rides the water cycle at **±1,700 cells** with
  no verb at all.

**Confounds**
- a rider that landed *with* the mechanism is constant across every arm of a
  sweep, so every setting fails the same way and the approach looks wrong
- a term in a weighted sum is not an independent knob: changing what one term
  can *express* reallocates the whole sum. A correct mechanism at inherited
  constants is a regression.
- a fix that changes what a number *means* requires re-deriving the constants
  that read it
- a scene that contradicts the code looks like a bug in the code
- chunk decomposition: artifacts aligned with the F1 grid are usually the sweep
  order, not the physics
- **RNG stream shift**: a draw consumed at the call site shifts every downstream
  stream, so an "off" arm is not a reproduction of the pre-change binary — read
  it against its own seed spread, not against the old number
- an isolated harness overstates what the app will see (−50% in `field_cost`,
  −27% in `scale_probe phases=1`, same change)
- **a cost that vanishes may be work that vanished** — a queue that goes quiet
  because the system stopped asking looks identical to one that converged

## What has changed since most entries were written

**97% of entries (747/773) predate 2026-09-06.** Weigh these against every
pre-cluster entry:

- **2026-09-06 landed six couplings in one day** — soil nutrient as per-cell
  data (`Chunk::nutrient_deficit`), plants held by roots not walls, tissue
  parting, colony groups, kin-as-scent-distance, and **the plasticity dial**.
  Every creature null measured before that day was measured at what is now the
  clonal control (plasticity 0).
- **The cost baseline was rebuilt**: `ChunkGrid`, `src/sim/fxhash.rs`, inline
  `FieldTile`, rayon min-job thresholds. Lab tick **2.4–5.7× faster,
  bit-identical**. Every "measured slower" verdict predates it.
- **Six new brain wires and three new verbs**: `PreyNear`/`PreyBearing`,
  `ThreatNear`/`ThreatBearing`, `KinNeed`; `Impulse`, `Attack`, `Share`. Plus
  the alarm plane, colony groups, per-colony breeder index.
- **Genomes persist** (`src/sim/specimen.rs`, `species_export.rs`) and the
  reachable set is a closed authored interval (`examples/genome_reach`).
- **Roots on by default** (root:shoot 7.5% → 22.4%); a root drinking free water
  counts as touching water; a cell that changes role changes tissue.
- **Canopy throughfall**: rain drips through a wood instead of standing in it.
- **The evolution lab exists** — a sealed box running plants and creatures
  together, with a scenario file and a headless rack of ~20 instruments.
- **The real app can be screenshotted headlessly.** CLAUDE.md previously said
  this was impossible; rejections whose stated reason was "cannot be judged
  without a display" are now checkable.

**Explicitly NOT met**, so do not claim these:
- the structural support field's *range* is still unbounded — no hierarchical
  potential exists; `structural-support-model.md` is still "design, nothing built"
- `load.rs:2137`'s bearing clamp is still gated on `parent.is_none()`
- `GRAIN_FOOTING_PROBE` is still `8`
- **chunk decomposition is unchanged** — the checkerboard, take/put ownership
  and deferred queues in `parallel.rs` are untouched. The ten `## parallelism`
  entries all still stand on their original conditions.

## The exemplar

The repo has already proved this failure once, and it is the shape to hunt:

> **the jump had worked since 2026-08-29 with no species wired to use it — 0
> launches in `forage_probe`, 275 with one `Bias -> Impulse` wire.**

The zero was the world, not the mechanism. A null measured in a world that could
not express the thing being tested is not a negative result.

## Output format — one line per entry, tab-separated

```
<id>	<LABEL>	<confidence 1-3>	<one-sentence reason, naming the trap or the changed condition>
```

- `<id>` is the bracketed id from the skeleton, e.g. `plants:042`.
- confidence: `3` you are sure, `2` likely, `1` a guess worth a second look.
  Suffix a `?` (e.g. `2?`) when **the rubric's rules decided it against your own
  reading** — those are exactly the entries a second pass should get.
- The reason must name **the specific trap or the specific changed condition**.
  "Might be worth revisiting" is not a reason and will be rejected.
- If the skeleton is not enough, open the full entry:
  `sed -n '<line>,+12p' Reports/dead-ends.md`. Do this for anything you would
  otherwise guess at — but you will not need it for most entries.

Do not editorialise. One line per entry, every entry, no preamble.
