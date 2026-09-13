# The chronicle's counts were a census of its own ring

*2026-09-13. Round 32, Lane B, second item. Closes
[`evolution-lab-playtest-2026-09-13.md`](evolution-lab-playtest-2026-09-13.md)
§5 — `LOG DROPPED 79,642`, and a narrative log that kept 664 of 15,905
births.*

## The finding in one line

**The cap was the symptom; the defect is that `COUNTS:` counted the ring.**
`chronicle_text` tallied `RunLog::recent()`, so a ring that is *full* prints
as a *complete* count — `BORN 664` for a bed that had had 15,905 animal
births in it. No cap fixes that short of holding the whole session, which
this report prices and declines. A 72-byte cumulative tally fixes it at any
cap.

**The lab already knew the rule.** The LOG page's own `OLDER` row says it in
as many words: *"nothing in the lab is ever counted off this page — the
counts come from each individual's own totals, which are never trimmed."*
The chronicle was the one place that did it anyway.

## What is actually in the ring, which is not what §5 assumed

§5 says the ring *"is being overrun by two orders of magnitude at this
population"*, which attributes it to the ant colony. At most half of it is,
and this follows from the owner's own log with no new run:

| | |
|---|---|
| individual events pushed | **81,690** (79,642 dropped + a ring full at 2,048) |
| animal births | 15,905 |
| animal deaths | 14,203 |
| `FirstFeed`, at most once per animal (`state.life.bites == 1`) | ≤ ~16,200 |
| **therefore animal-attributable, at most** | **~46,300** |
| **therefore the plant stand, at least** | **~35,400 — 43% of the ring** |

The line ring is not under pressure at all: it held 242 of its 2,048, so all
79,642 drops are individuals.

**A plant pushes `Born` when it germinates and `Died` when it is freed,
exactly as an ant does**, and the census columns that look like they would
show it do not — `born`/`died` there are `creature_stats`, animals only, and
the screen labels them `ANIMALS BORN / DIED`. So founding fewer colonies
would not have saved the ring; the bed's own turnover is the other half.

**Measured directly rather than inferred, on a run of the repaired
binary.** `chronicle frames=60000 colonies=1 founders=8`:

```
counts: BORN 667, DIED 2900, FIRST FED 53, FIRST SEED 65, LINE ENDED 50, LINE MILESTONE 2
        | line events 52 vs individual events 3685 | lineages claimed 60 | log dropped 0
```

against a census row reading **11 animal births and 58 animal deaths** at the
same frame. So **656 of the 667 births and ~2,840 of the 2,900 deaths are
plants**, on a bed holding five ants. The upper bound from the owner's log
says *at least* 43%; on this bed it is 97%. Deaths exceed births because a
dormant seed is an organism that can die without ever germinating, and
germination is what pushes `Born` — the plant line's own rule, and visible
here for the first time because the counts are no longer clipped.

Two things that reconcile in that line and are worth reading as checks:
`667 + 2900 + 53 + 65 = 3685`, the individual total, and `50 + 2 = 52`, the
line total. And `log dropped 0` at 3,685 individual events is the cap change
itself — **the old 2048 would have dropped 1,637 of them on this short
run**.

## Why the cap is not sized to hold a session

A session-sized ring is 131,072 (2^17 > 81,690) and costs twice:

- **7.3 MB per world.** `size_of::<LogEvent>()` is 56 bytes, one ring per
  chamber on the rack, and `RunLog` is `Clone`.
- **~0.95 ms of every painted frame the LOG page is open.** `Ui::log_rows`
  collects the whole merged ring into a `Vec` and then scans it again for its
  `OLDER` row. Measured paired and alternating inside one process, four ring
  lengths, forty rounds each:

| ring | per paint | Vec |
|---|---|---|
| 2,048 | 0.012 ms | 16 KB |
| 8,192 | 0.041 ms | 64 KB |
| 32,768 | 0.209 ms | 256 KB |
| **131,072** | **0.945 ms** | 1,024 KB |

Linear, as the code says it must be. Six percent of a 16.7 ms frame to
back-fill a page that shows fourteen rows is not a trade this engine makes,
and it buys nothing the tally does not already give.

**The first version of that measurement was vacuous and said so.** All four
arms reported 0.005 ms and 16 KB, because `RunLog::push` clamps at
`RUN_LOG_CAP` and every arm was therefore the same 2,048-event ring —
`CLAUDE.md`'s *identical output across a change that must have moved
something*. The numbers above are from a build whose cap exceeds every arm.

## What the old cap was derived against

2048, from *"roughly 640 notable events per 90,000 frames of the shipped bed
... several sessions of headroom even on a colony far bigger than the shipped
one"*. **The shipped bed is not a colony**: measured here, one colony on the
default 512-wide box is down to **5 ants by 60,000 frames**, so that estimate
was taken on a population that had already collapsed. The owner's session
pushed forty times the cap.

**8192**, then — four times the depth for 459 KB and 0.041 ms. At the owner's
own event rate (81,690 / 560,000 = 0.146 per frame) that is ~56,000 frames of
scrollback against the old ~14,000: minutes of his wall clock rather than
seconds, and still free. What the ring is left holding is scrollback and one
pinned individual's timeline; the counts no longer depend on it.

## The controls

Three, each watched going red against its own restored defect:

| fault restored | what goes red |
|---|---|
| `COUNTS:` tallying `recent()` again | the chronicle control, printing `BORN 8192 … LOG DROPPED 8209` — the owner's `BORN 664 … LOG DROPPED 79642` in miniature |
| the tally clamped at `RUN_LOG_CAP` | the `RunLog` control, 8,192 against 24,576 |
| `pushed_by_kind` emitting zeros | the omission control, on `FIRST SEED 0` |

The `RunLog` control asserts **both** halves on one log: that `pushed` names
every event, and that counting the ring gives a *different* answer on the
same data — so it cannot pass with the fix reverted, and cannot pass
vacuously on a run that never overflowed. It also checks each kind's slot
separately (a single total would hide a kind wired to the wrong index) and
closes the identity `sum(pushed) == len + dropped`.

## Left alone, deliberately

`LINES ENDED n` beside the counts is still `ended_lines(world).len()` — a
count off the line ring. That is correct for what it labels (how many legend
paragraphs the chronicle carries below it), and the line ring is at 12% of
its cap. If it ever fills, that number and the exact `LINE ENDED n` in
`COUNTS:` will diverge, and the divergence is the useful signal.
