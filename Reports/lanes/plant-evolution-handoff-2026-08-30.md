# Plant evolution: handoff, 2026-08-30

**Read the files, then decide for yourself.** What follows is state and
opinion, not a work order. The suggestions in §5 are one session's view formed
while tired of its own assumptions; several of this session's best findings
came from *doubting* the plan in front of it. If the code says something else,
believe the code.

---

## 1. Where the line actually stands

Four things are now measured rather than assumed:

- **The genome mutates at the rate the constant says.** 0.982% pooled against
  a nominal 1.000%, 96% of draws applying (`plant-mutation-counted-at-source-
  2026-08-29.md`). The 2.6x "drift shortfall" that was open was a **model
  error**, not a mechanism one.
- **The production rule is heritable and drifts** — 20 coexisting rule tables
  in one herb stand.
- **The world selects between living plants.** `nobranch` loses 11 points of
  the bed on 18 of 18 seeds, p=0.0002, against a control at p=0.19
  (`plant-selection-teeth-2026-08-29.md`). This answers the owner's
  false-negative worry: the environment is **not** inert.
- **Selection here is equilibrium-seeking, not directional.** Arms settle at a
  stable ratio instead of one displacing the other. `nobranch` sits at ~39%,
  `early` at ~7%; neither heads for zero.

So variation, heritability and differential fitness are all present. What is
**not** established is whether the mutations the *engine* makes land where
selection can see them — every arm to date is a handicap authored by hand.

## 2. What is broken or provisional, and must not be quoted

- **The teeth report's magnitudes are mid-transient.** Arms ran 20,000 frames;
  the share does not settle until ~50,000–75,000. §0 of that report says so.
  The *direction* is safe; the numbers are not final. `Reports/lanes/
  settled-rerun.md` is the job that fixes it and was handed to a local machine.
- **The frequency-trajectory / selection-coefficient readout is
  known-broken.** Two independent reasons: the generation axis saturates (mean
  generation is taken over *living* organisms, so it equilibrates at ~2.9 and
  is not a clock), and the share equilibrates too, so there is no signal to
  integrate. §5a has the account. The harness detects both and warns. **Do not
  quote its `s`.** A struck-through section records the obvious fix
  (a cumulative clock) and why it does not work — do not rebuild it.
- **One live bug, left deliberately**: the endpoint/trajectory cross-check
  composes medians taken independently across seeds, and medians do not
  compose. Recorded in its commit. Fixing a check before the thing it checks
  is validated is the wrong order — if you revive the trajectory work, fix it
  then.

## 3. Open, waiting on the owner

**The per-query fallback fork** — review card `20260829T204941423Z-880e13`,
still unanswered. `plant::fate_for` consults genome → species → builtin *per
query*, so a mutation that vacates a slot is backfilled and three of four
operators are near-inert. `FateLookup` (default-off) exists to render the
alternatives. Measurement cut it from three options to two: `NoSpecies` is
byte-identical to `Full` at a 10x mutation rate, so `builtin_fate` is the real
absorber. **Do not change the shipped behaviour without the owner's answer.**

## 4. Instruments you now have

- `selection_arena` — two genomes competing in one bed, attributed by lineage,
  mirrored assignment, order statistic over seeds. `arm=` is a ladder
  (`same|lethal|early|nobranch|norootbranch|mutantK`). `dump=1` prints the raw
  trajectory. `seed0=` makes one seed independently runnable.
- `scripts/dfe.py` — variance decomposition for the distribution of fitness
  effects. Validated both directions on data in hand.
- `OrganismState::lineage` — now claimed by plants (it existed and no plant
  ever entered it, so every plant read 0). This is what makes a lineage
  census possible at all; `plant-rule-drift-observed` §5 wanted it.
- Counters `fate_mutation_rolls` / `_fired` / `_applied` on `World`.

## 5. What I would do next — and why you should check it rather than take it

**a. Collect the settled re-run and the DFE**, both handed to a local machine
via `Reports/lanes/settled-rerun.md`. The DFE is the payoff of the whole line:
it asks whether real mutations produce fitness variation the world can act on.
Expect a large *silent* fraction — a probe had 5 of 8 mutations land on
identical shares — and `dfe.py` counts those separately rather than pooling
them as neutral, which would read as "mutations don't matter".

**b. If the DFE returns Var(true) ≈ 0**, the bottleneck is the
genotype→phenotype map, not the environment, and the line's next work is
composition/organs/morphology rather than evolution machinery. Treat that as an
**upper bound**, never as proof of neutrality.

**c. Make water actually scarce.** `norootbranch` is a null (roots not under
selection) and the varied bed does *not* change it — my prediction, refuted and
recorded. The evidence points at the binding resource: herb is carbon-limited
(leaf construction refuses 45–48% of wanted cells), so shoot architecture pays
and root architecture does not. A bed at the wilting point, or a drought cycle,
is the untested prediction. **Varying a resource is not making it scarce.**

**Unowned and independent**: `open-bugs-handoff.md` §1n — grass sets zero seeds
on main, a shipped species that cannot reproduce, never bisected. It does not
unblock evolution (grass's fate table is byte-identical to tree's) but it is a
real bug.

## 6. Method notes that cost this session real time

- **Look at raw output before summarising it.** This session's three best
  findings — the establishment hump, the saturated axis, the five identical
  mutant shares — were all invisible in correctly-computed summary statistics.
  Two wrong claims were made to the owner before anyone plotted a curve.
- **A control that cannot fail is not a control.** The mirrored identical-arms
  control returns exactly 50.0% as an algebraic identity. Run it unmirrored.
- **Tidiness is the tell.** Every wrong number here was clean: exactly 50.0%,
  exactly 0.0%, five identical shares. Outcomes in this engine are chaotic.
- **A genome that changed is not a plant that changed.** Three arms were
  vacuous before they were real, including the discovery that `lateral: None`
  is not "no lateral" (`plant.rs`'s Grow arm falls back to the cell's own type)
  and that a herb shoot never places a lateral at all while its root does.
- **This container restarts** (observed uptime: 3 minutes with a 12-seed pair
  in flight). Long runs need `seed0=` chunking or a local machine.
