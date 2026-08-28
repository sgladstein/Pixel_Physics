# Why every change costs a global retune (2026-08-27)

**Status: method finding, written from a live instance.** The owner named a
pattern he has been living with:

> "I feel like we get stuck in this loop where we have a bunch of features to
> build. we build one and it changes things and then we have to spend a ton of
> time tweaking everything and maybe dropping the feature because it made
> things worse. but that could have been temporary and we never get around to
> building the next feature."

The pattern is real, it has a diagnosable cause, and the cause was measured
the same day. This is not about plants; the plant line is where it was
noticed.

## 1. The instance

`plant-phototropism-lateral-2026-08-27.md`: a repair that `dead-ends.md`
itself prescribes was built, proved correct by a control that fails on the
old code, and withdrawn — because it took a stand's reproduction to **zero**.

**Nothing in that sentence is a defect.** The test was right: it failed on an
*anti-vacuity* guard, i.e. it announced that the scenario it needs in order
to mean anything had stopped occurring. The implementation was right: it did
what the record prescribed and its guard proved it non-blind.

**What was wrong was a constant.** `light_weight` at `[0.15, 0.3, 0.5, 0.6]`
is a sensible number for a lever whose codomain is `{(0,-1), (0,0)}`. It is a
nonsense number for a lever that can point anywhere. One shared scoring
budget got reallocated, and a stand that used to climb started spreading.

## 2. The cause: free parameters, cross-calibrated

Measured the same day
(`plant-heritability-survey-design-2026-08-27.md` §4a): **most of the largest
phenotype levers have no counterweight** — `turgor_source`, `turgor_yield`,
`plastochron`, `heading_inertia`, the juvenile trio, `seed_maturity`, the
`rate` scalar, both half-lives, and `crowding_weight`, whose counterweight was
*deliberately removed*.

**A system whose parameters carry no costs has no equilibrium to return to.**
Every constant is implicitly calibrated against every other constant's
*current* behaviour — including the broken parts. So any change reallocates a
fixed budget and everything moves at once. Three independent instances are
already on the record: `pipe_ratio` calibrated against a broken `thicken()`
flood fill; `light_weight` calibrated against a lever that could only say
"up"; and a granular capacity divisor that existed only to cancel an eager
rooting rule, where *"every attempt to tune it made some case worse because it
was holding a different mistake in place."*

**So costs are not a feature competing with other features. They are what
makes features composable.** In a priced system a new mechanism is *absorbed*
— the economy pushes back and settles somewhere new. In a free system every
new mechanism requires a global retune, and the price per increment never
comes down. That is the loop, stated mechanically.

## 3. The bias that makes it worse

In a system with chaotic dynamics most structural changes make things worse
**before** the constants around them are re-derived. Judging a change on its
immediate effect therefore **systematically rejects exactly the changes that
need a retune** — which is most of the important ones — and systematically
keeps the changes that happen to fit the current tuning.

That is a ratchet toward a local optimum. **And in this project the local
optimum is one the owner has already rejected**, three times, in the same
words: the plants differ only in size and colour. Work spent making a new
feature compatible with the current constants is work spent defending that.

## 4. What to do differently

1. **Guard properties, not the stand.** A test should assert *plants
   reproduce*, *nothing explodes*, *frame cost is bounded*. A fingerprint over
   emergent behaviour is a ratchet against change. (`CLAUDE.md` already says
   "assert the property, not two instants fitted to one trajectory" — this is
   the same rule applied to guards over a whole world rather than one tree.)
2. **Batch by shared tuning surface, not by feature.** `phototropism`,
   `light_weight`, `upward_weight` and `crowding_weight` divide **one** score
   (`plant.rs`'s `preference` sum), so they are one change and landing them
   separately guarantees churn. `phototropism` and `seed_half_life` share
   nothing and must never be batched. This is neither "slow and iterative" nor
   "big bang": the unit of work is *a budget*, not *a feature*.
3. **Price before you build.** Adding organs to a free-parameter system means
   retuning everything afterwards. Adding them to a priced system means the
   economy absorbs them. This is the concrete argument for doing the
   counterweight work *before* the organ/clade machinery, not alongside it.
4. **Budget the retune as part of the change.** "Is this worth building?" has
   to mean "worth it *including* the retune". If the retune is unaffordable,
   the change has not been scoped, it has been started.

## 5. The session's own error, as the worked example

Rule 4 is written from failing it. This session proposed the phototropism
repair as a *"small, no-regrets fix"* — and had, in its own hands, an
inventory stating `light_weight` was authored at 0.15-0.6. **It quoted that
number and still called the change small.** It was never a bug fix; it was a
tuning change wearing one, and rule 4 applied honestly would have said: do
not start this without the budget to re-derive five species' weights.

The withdrawal was correct. The scoping was not.

## 6. Proposed, not made: a `CLAUDE.md` rule

This is deliberately a report rather than an edit to `CLAUDE.md`, which is
loaded before every session and belongs to every line, not to this one. The
candidate rule, for the owner to accept or reject:

> **A change that reallocates a shared budget is a tuning change, not a fix,
> however small its diff.** Before starting one, name every constant
> calibrated against the current behaviour of the thing being changed, and
> budget re-deriving them as part of the work. A correct mechanism at
> inherited constants is a regression.

It meets `CLAUDE.md`'s own addition test — it is universal (any subsystem with
a weighted score or an economy), the failure is expensive, and nothing else
catches it: every gate in the repo passed the phototropism change except one
anti-vacuity guard that fired for an unrelated reason and caught it by luck.
