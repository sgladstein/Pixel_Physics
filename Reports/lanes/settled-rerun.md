# Lane: re-measure the selection arena at equilibrium

**For a session with a machine that will not restart mid-run.** Everything
here is compute, not judgement: run four commands, check one line, paste the
output back. No design decisions are delegated.

**Branch: `claude/plant-selection-teeth-test-q7w2ne`** (PR #134). Work on that
branch; do not open a new one.

---

## 1. What is wrong, in one paragraph

`Reports/plant-selection-teeth-2026-08-29.md` reports a selection ladder
measured over **20,000 frames**. A later 150,000-frame run showed the quantity
being measured — arm B's share of the bed — **does not settle until around
frame 50,000–75,000**. So every headline number in that report is a
mid-transient reading, not the equilibrium it is presented as. §0 of the report
already says so.

The direction is safe (18 of 18 seeds, and the ladder's ordering is a large
consistent signal). The **magnitudes** are provisional. This lane replaces them.

## 2. Build

```
git fetch origin
git checkout claude/plant-selection-teeth-test-q7w2ne
git pull
cargo build --release --examples
```

**`--examples` is not optional.** `cargo build --release` does not rebuild
them, and a stale binary prints plausible numbers from old code — see
`CLAUDE.md`'s `include_str!` gotcha, which has cost this project three
bit-identical "runs" of a knob that was never connected.

## 3. Run

Four arms. Parallel if you have the cores; they are independent.

```
A=./target/release/examples/selection_arena
$A arm=same mirror=off  seeds=18 founders=8 frames=90000 every=5000 > /tmp/control.log      2>&1 &
$A arm=nobranch         seeds=18 founders=8 frames=90000 every=5000 > /tmp/nobranch.log     2>&1 &
$A arm=early            seeds=18 founders=8 frames=90000 every=5000 > /tmp/early.log        2>&1 &
$A arm=norootbranch     seeds=18 founders=8 frames=90000 every=5000 > /tmp/norootbranch.log 2>&1 &
wait
tail -40 /tmp/control.log /tmp/nobranch.log /tmp/early.log /tmp/norootbranch.log
```

**If you only have time for two, run `control` and `nobranch`.** The control
licenses everything else, and `nobranch` is the load-bearing row — a plant that
grows, flowers and sets seed, and still loses.

`lethal` is deliberately not in the list: it reads 0.0% because such a plant
leaves no biomass by construction, and it settles instantly. Nothing to redo.

**Shorter options.** `seeds=12` is fine — the effects are large, so seeds
sharpen the magnitude rather than establish it. `seed0=N seeds=1` runs exactly
one seed, so the work can be chunked and resumed; the per-seed lines are the
durable record.

## 4. The one line that decides whether the run counts

Each log either does or does not contain:

```
*** N of M world-runs ended while arm B's share was STILL MOVING ***
```

- **Absent → the run is settled and usable.** This is the pass condition.
- **Present → 90,000 frames was not enough.** Re-run that arm at
  `frames=150000`. Do not report the numbers; they are the same defect this
  lane exists to fix.

Also check the banner line each run prints. If it does not say
`frames=90000`, the argument did not take and the binary is stale — an unknown
argument is silently ignored.

## 5. What to send back

The `tail -40` output is enough. The lines that matter, per arm:

```
pooled over N seeds ...
  B share of organisms  median ...
  B share of cells      median ...   quartiles ...
  seeds where B held LESS than half the biomass: X of N
  Wilcoxon signed-rank ... z=... p=...
```

Paste it verbatim. **Do not summarise, round, or drop the quartiles** — the
spread is load-bearing here: the control's own spread is what sets the
resolution floor, and the median alone has already been misleading once in
this line (it read 63.7% with no true effect present).

## 6. What to expect, so a surprise is recognisable

From the 20,000-frame (unsettled) run:

| arm | median B share | seeds B lost | p |
|---|---|---|---|
| control | 55.4% | 6/18 | 0.19 |
| `early` | 7.1% | 18/18 | 0.0002 |
| `nobranch` | 38.9% | 18/18 | 0.0002 |
| `norootbranch` | 49.7% | 10/18 | 0.86 |

The ordering should hold. The magnitudes may move — that is the point of the
exercise, so **a shift is a result, not an error**.

**Two things would be genuinely surprising and worth flagging loudly rather
than filing:**

- the **control** moving far from ~50% or becoming significant. It is two
  copies of one genome; if it separates, the harness is measuring its own
  asymmetry and every other arm is void.
- `norootbranch` becoming significant. It is currently the null that says root
  architecture is not under selection in this bed.

## 7. Do not

- **Do not change the harness to make a number come out.** If an arm looks
  wrong, say so; the last three "results" in this line were harness bugs
  (`lateral: None` is not "no lateral"; a herb shoot never places a lateral;
  a mirrored identical-arms control is an algebraic identity).
- **Do not quote the trajectory / selection-coefficient block** if it prints.
  It is implemented and known-broken: the generation axis saturates and the
  share equilibrates, so the slope means nothing. `Reports/plant-selection-
  teeth-2026-08-29.md` §5a has the full account.
- **Do not merge PR #134** on the strength of these numbers alone; the report
  text needs updating with them first.

## 8. Second job, if the machine is free — the distribution of fitness effects

**Only after §3.** This is the question the whole evolvability programme is
for, and it is now runnable: every arm above is a handicap authored by hand,
which shows the world *can* select and says nothing about whether the
mutations the engine actually makes are things it can select *on*.

`arm=mutantK` draws the K'th mutation from the **shipped** operator
(`FateGenome::mutate`, not a copy of it), keyed so mutation K is the same
mutation on every world seed.

```
A=./target/release/examples/selection_arena
for k in $(seq 0 23); do
  $A arm=mutant$k seeds=6 founders=8 frames=90000 every=5000
done > /tmp/mutants.log 2>&1
python3 scripts/dfe.py /tmp/mutants.log /tmp/control.log
```

24 mutations x 6 seeds is the useful minimum; more of either is better, and
the loop is resumable by editing the range. `/tmp/control.log` is the one §3
already produced — it sets the noise floor and cannot be skipped.

**Why this is answerable when a per-mutation verdict is not.** §3 of the
report puts the resolution floor at ~9.3 share-points per seed, so pinning one
mutation needs hundreds of worlds. But variance decomposes:

```
Var(observed across mutations) = Var(true effects) + Var(noise)
```

and the control measures `Var(noise)` directly, so the real spread follows by
subtraction even when no single mutation is resolvable. `scripts/dfe.py` does
that arithmetic and prints the reasoning.

- **Var(true) ~ 0** -> mutations are effectively neutral, selection has nothing
  to sort, and the bottleneck is the genotype->phenotype map, not the
  environment. Report it as an **upper bound**, never as proof of neutrality;
  the script prints the bound.
- **Var(true) > 0** -> heritable fitness variation exists and evolution can
  proceed.

**Expect a large silent fraction, and do not read it as neutrality.** A quick
8-mutation probe at one seed had five arms land on *identical* shares, which is
the signature of the genome changing and the plant not — the per-query
fallback absorbing the operator, exactly what
`plant-fate-operator-gate-2026-08-29.md` measured. `dfe.py` counts those
separately and refuses to pool them, because pooling drags the spread toward
the noise floor and reads as "mutations are neutral" when the truth is "those
mutations never happened, phenotypically". **The silent count is itself a
result** — it is the live-population version of the operator gate, and it bears
directly on the fallback fork the owner has not yet decided.

Send back the `dfe.py` output plus the tail of `/tmp/mutants.log`.

## 9. Context, if wanted

- `Reports/plant-selection-teeth-2026-08-29.md` — the report being corrected;
  §0 is the caveat, §2 the controls, §3 the power limits.
- `Reports/instruments.md` — the `selection_arena` row.
- `examples/selection_arena.rs` — the module doc explains the mirror, the
  lineage attribution and why the identical-arms control must be unmirrored.
