# The evolution lab, round thirty-two: the performance round

*2026-09-13. Handed over by round 31's coordinator with the owner's priority
stated in his own words and already sized by measurement, which is the first
time a round has opened that way.*

> *"The biggest issue is the performance after the creature numbers get high
> and that is our #1 priority by far."* — the owner, 2026-09-13

Brief: [`evolution-lab-round-32-brief-2026-09-13.md`](evolution-lab-round-32-brief-2026-09-13.md).
Measurement of record: [`evolution-lab-playtest-2026-09-13.md`](evolution-lab-playtest-2026-09-13.md),
over his own 560,000-frame session.

## What the round was aimed at

**cost ≈ 1.0 ms/tick + ~2.1 µs per ant per tick.** At 3,000 ants the creatures
are **86% of the frame** and everything else together is about a millisecond.
Not the cell sweep, not the renderer — both refuted from the same log. **The
number to beat was 2.1 µs per ant per tick**, because halving it roughly
doubles the population he can play at.

## The lanes

| lane | job | model |
|---|---|---|
| A | the late-game creature cost — reproduce 2.1 µs headless, localise it inside `creature::tick`, cut it | Opus 5 |
| B | the five dead census columns, which gate every future log he sends | Opus 5 |
| C | the spoil teleport (PR #221, open since 2026-09-03) and whether §Z13's idle ants are resting or stuck | Opus 5 |

Three lanes rather than round 31's five, on the owner's conserve-tokens ruling
and with the account's seven-day limit at `allowed_warning`.

## What the coordinator checked before briefing anyone

**The round's own premise, refit from the raw log.** Three lanes were about to
be pointed at `2.1 µs per ant per tick`, so it was worth one command to see
whether that number survives being re-derived by someone who did not write it.
It mostly does — and the way it fails is the useful part. The full working is
[§1a of the playtest report](evolution-lab-playtest-2026-09-13.md); the short
form:

- **The ~1.0 ms floor is measured, not fitted.** Frames 10,000 / 20,000 /
  30,000 carry `ants 0` at 1,100 and 1,000 µs/tick. A least-squares line
  through the envelope puts the intercept anywhere between **234 and 930 µs**
  depending only on the number of bands — so the one quantity a regression is
  worst at here is the one that did not need regressing.
- **The slope is two slopes.** Below ~450 ants an ant costs **≈ 0.27 µs**;
  above ~600 ants it costs **≈ 2.6–2.9 µs**. The single 2.1 figure is an
  average across a bend and describes neither regime.
- **The slope estimate is otherwise robust** — 4/6/8/10/12-band envelope least
  squares, Theil–Sen over band medians, and the chord between extreme bands all
  land in **2.1–2.8**, centred near 2.4. §1's headline is the right order and
  the right target.

**Why this was worth a message rather than a footnote.** A lane building its
positive control at 100–500 ants would have measured 0.3 µs/ant, disagreed with
its own brief by sevenfold, and gone looking for a harness bug that does not
exist — `CLAUDE.md`'s *a scene that contradicts the code will look like a bug in
the code*, arriving through the brief rather than through the scene. Lane A was
told before it built anything.

**And the knee is worth more than the coefficient.** A constant per-ant cost
says *shave the brain*. A bend says something changes **character** with
population, and it sits almost exactly where the owner stops being able to
play. **The confound is stated rather than resolved**: in his log ant count
rises with session age, so only a controlled headless sweep — population set
rather than grown — separates the two, which is the first thing Lane A was
asked to build.

## Verdicts collected, which existed only in the queue

Round 31 left two answered cards nobody had written down. `CLAUDE.md` warns
that a card can be archived carrying no stored response, so a verdict that
lives only in the queue is a verdict that is about to be lost.

- **§Z13, `20260913T052938222Z-c7bc21`** — three idle-animation candidates,
  posted *after* #380 fixed the clock so the mechanism genuinely ran. **The
  owner's entire answer: _"Are they resting or stuck?"_** He picked no
  candidate and did not say it still looks wrong; **he asked the question back**.
  That closes the look explanation as the thing to work on and makes the
  measurement the deliverable — Lane C's job 2, now with the owner's own words
  behind it. **No fourth animation.**
- **§Z18, `20260913T044639577Z-4e0ecb`** — was the floating material dug soil
  or plant tissue, with every dug cell painted orange. **The owner's entire
  answer: _"Everything looks normal in all of these pictures."_** A null on the
  rendered bed, and a standing warning against using "does the bed look wrong"
  as the acceptance test for the spoil-teleport work.

Both carry a free-text comment and `annotations: []`. **Round 31's published
claim that annotations do not survive the queue is false** — round-29 card
`20260912T045951545Z-6931d4` still returns its three markers in full through
`get`. These two simply have none.

## Open, and the owner's to decide

**Is ~320,000 frames too long for the land to come back?** The premise task 3
was written on — *the abandoned nest never regrows* — is refuted by his own
log: bare ground outside the nest goes 2% → 56% → 6% and the plant count ends
at **409 against an original peak of 277**. It recovers, and it takes about
320,000 frames to do it, which is long enough that every earlier look landed
before it happened. Whether that is too slow is a judgement, not a bug, and
nothing should be built against it until he says.

## What landed

(filled in as the round runs)

## What the round overturned

(filled in as the round runs)
