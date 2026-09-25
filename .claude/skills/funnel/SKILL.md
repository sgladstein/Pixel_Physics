---
name: funnel
description: Find out why a per-individual system is not working — a foraging loop, a plant's growth, a rigid body's breakage — by counting every individual through the stages of the thing they are meant to do, then reading the brains of the ones that got stuck. Use when a mechanism "does not work" and you do not yet know WHERE it fails, when a rate or a count moved and you need to know whether it moved for the right reason, or when you are about to quote an improvement as a ratio. Also use before accepting that a fix "didn't work".
---

# The funnel, then the individuals

Two instruments, in this order. Owner's instruction, 2026-09-20, after a
session spent three times learning it the expensive way.

## Why this exists

**An aggregate cannot carry a reason.** A rate says *how often*; it cannot say
*because the gate it needed was shut*. So every question of the form "why is
this not working" is answered by tracing individuals — but you cannot trace
every individual usefully until you know *which* ones to trace. The funnel
tells you where they are lost; the trace tells you why.

Measured cost of skipping straight to a population statistic, in one session
on one question: **three splits, all arithmetically correct, all invalid** —
one underpowered by an order of magnitude, one whose grouping variable the
engine reset mid-run, one whose denominator counted a different *kind* of
event on each side and came out **37x backwards**. One per-tick trace of one
animal answered it immediately.

## 1. The funnel

**Count individuals through the stages of the loop, not events.** For the ant
line that is: lived → reached the food → picked it up out there → turned for
home with it → got back → put it down → went out again → reached the food a
second time.

Four rules, each from a failure already paid for here:

- **Per individual, at its high-water mark, monotone.** `stage =
  stage.max(n)`, so an ant is booked once at the furthest point it ever
  reached. Every rate in this repo that divided events by events has been
  wrong at least once, because an individual that relapses gets counted twice
  and the two sides of the ratio count different kinds of event.
- **Counts AND two percentage columns: `of prev` and `of all`.** *Of prev* is
  where they are lost. *Of all* is whether the colony is doing anything.
  Owner, 2026-09-20: *"Something that doubles from two ants to four sometimes
  looks really good, but actually still 90% of the ants aren't doing
  anything."*
- **Gate each stage on the state that makes it real, not on the event that
  looks like it.** A pickup beside the nest is not the start of a commute:
  measured, **1,998 of 2,132 laden legs were 12-frame pickups and putdowns at
  the comb**, and an ungated census of them read 37x backwards. A crop
  emptying is not a delivery unless it empties at the nest — since digestion
  pays out continuously, a crop that empties in the field is the animal
  *eating its cargo*.
- **Read the funnel before quoting any ratio.** A change that moved `drops`
  by +64% at p 0.0066 turned out to have moved real commutes **down**, 129 to
  109, while doorstep churn went up. The tell was that `carry->nest` did not
  move with it.

`examples/trailfollow.rs` has a worked one: `FUNNEL`, `Track::stage`, and the
`THE LOOP, ANT BY ANT` block. Copy its shape rather than its stages.

**For the ant's foraging loop it is one command.** Run the colony bed with
`decisioncsv dtag=<tag>` and keep the log, then
`python3 scripts/antloop.py <dir of CSVs> --tag <tag> --log <run.log>`. It prints
the funnel (reach -> pick up -> home holding it -> put down, then the 2nd..10th
loop), loops per ant, broken loops (ate it on the way / ate it at home), **who
starved** bucketed by how far they got and where they died, a **time budget**
(on the nest standing, exploring, up a wall, digging, carrying), and the
**economy** (burn against intake, who ate the food, how much stands on the
nest). It reconciles its starved count with the harness's `DEATHS BY CAUSE`
per run and prints any run that disagrees. Owner, 2026-09-25, of the first
hand-built version: *"a great analysis. We should do this more often."* So run
it on every bed change to the ant, before and after, not only when something
looks wrong.

## 2. The individuals

**Once the funnel names the stage where they are lost, trace every individual
that reached it** — not one focal animal. One animal is an anecdote until the
population trace agrees with it.

- **Put the inputs and the chosen output in the row**, not just the position.
  An animal walking confidently to the wrong place and one that will not steer
  at all produce identical position rows.
- **Pair every "it fired" counter with an effect counter from the far side of
  the call.** `tumbles_homeward` once read 1,150 of 15,291 where every one was
  a correct aim at a *wrong target*: both counters reported a working
  mechanism.
- **A row per tick, and check the decomposition reproduces the engine's own
  number.** `trailfollow`'s trace asserts `move_terms` rebuilds what
  `eval_brain` computed; without that, every conclusion is about arithmetic
  the harness invented.
- **Trace to an OUTCOME, and bucket by it.** "Completed" against "died"
  against "gave up" is where the answer is. Pooling them describes no
  individual that exists.

## 3. Before you believe the numbers

- **Key the parse on every dimension the run sweeps.** `trailfollow` sweeps
  three commute distances; a parse keyed on `(seed, arm)` silently pools
  three experiments, last write wins. Print the key's cardinality and check it
  against what the run varied.
- **Wait for the writer to exit** — `pgrep -x <exe>`, never `-f`, which
  matches the waiting shell's own command line and hangs for ever.
- **Pair within individual or within seed. Never pool across arms** whose
  populations differ in size, because the weights are then the thing under
  test.
- **Put the fault back.** A guard's green is evidence only if it can go red.
  Three ledger guards in this repo stayed green through a deliberate
  double-counting fault; the one written to see it went red in 0.09 s.

## 4. And do not throw a fix away because it did not fix everything

Owner, 2026-09-20: *"This is a complex multi stage issue and too often we fix
one problem but it doesn't solve everything so then we throw it out."*

Worked case from that day: a homing sensor measured as a **null** on the loop
— laps 91 → 95, every sign test a coin flip — and was nearly filed as a dead
end. On a repaired food economy the same wire, unchanged, took closed laps
**88 → 144, 19 seeds up of 21, p 0.0002**. It was never inert; it was gated
behind a different failure. **When a mechanism is sound but the outcome does
not move, the question is which OTHER stage of the funnel is capping it** —
so read the funnel again with the fix on, and look at where the ants now
stop.
