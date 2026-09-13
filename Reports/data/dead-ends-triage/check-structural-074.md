# Check: `structural:074` — is the mid-crown landmine still armed?

**Verdict: the condition the entry names is met. The empirical half is not run,
because no instrument stages the situation.** 2026-09-11.

## The entry

`dead-ends.md:372`, on `src/sim/plant.rs` `shed_stranded_leaves`:

> Scheduling structural checks from leaf abscission was tried and measured at
> **26× destruction**: the organism support search is **hop-bounded**, so a
> mid-crown check reads everything past the span limit as unsupported and
> amputates the upper crown — **772 cells against 20,213** at the same shedding
> rate, the only difference being the check. It masqueraded as *"the shedding
> mechanism collapses the stand at any setting"* across a whole eight-setting
> sweep.
>
> *Re-test when:* **Holds until the organism support search anchors properly**;
> Phase 3 damage work will meet the same landmine — any mid-crown disturbance
> today over-amputates, and **Phase 3 damage results are contaminated by this
> until fixed**.

## The condition, checked in source

**The hop bound is gone, and attachment no longer asks a distance question at
all.** `plant.rs:7368`:

> *"**That hole does not exist here, because this number does not answer 'is it
> attached'.** Attachment is `support == u16::MAX`, a separate question decided
> by **whether the anchor walk reached the cell at all**."*

`SUPPORT_COST_STANDING` is `0`, so a whole trunk relaxes to zero and "should
stand to any height". There is **no `MAX_SUPPORT_HOPS`, no span limit, and no
hop-bound constant anywhere in `plant.rs`** — the search anchors by
*reachability*, which is what "anchors properly" asks for.

This is a stronger check than the one that caught me out on `creatures:039`:
there I verified a named artifact existed; here the *mechanism* that produced
the failure — reading "past the span limit" as unsupported — has no span limit
left to read past.

## Why the empirical half is not run

The entry's live claim is that **any mid-crown disturbance today
over-amputates**, and that Phase 3 damage results are contaminated until it is
fixed. That is testable and should be tested. It was not, because **nothing
stages a mid-crown disturbance**:

- `plant_severance` — the instrument a summary would reach for — cuts *"a band
  of the plant's own cells removed **just above the soil line**"*. A base cut is
  not a mid-crown check, and the whole finding turns on the difference.
- `crown_census` counts material by height; it disturbs nothing.
- Nothing else under `examples/` removes cells mid-crown.

**Three specifics in the proposed arm were also wrong**, and are recorded so the
next session does not re-derive them: `schedule_structural_check_around` **does
not exist** in `plant.rs`; `shed_stranded_leaves` has **four** live call sites
(`:10682`, `:10712`, `:11082`, `:11495`) rather than two; and the swap-one-
identifier arm therefore is not available as described.

## What to run, when someone builds it

The arm is: re-introduce a structural check scheduled from abscission, and
measure standing cells against an unmodified binary on the same seeds.

*Alive (landmine gone):* standing `cells` within the seed spread of the
unmodified binary — the amputation was a property of the retired hop bound, and
**Phase 3 damage measurements are uncontaminated**.
*Still dead (landmine armed):* `cells` collapses toward the recorded 26× ratio —
the bound survives the change in form, and **every Phase 3 damage number taken
since is void**.
*Positive control, first:* confirm the harness's amputation column moves at all
on the unmodified binary. A column that cannot move reports "no amputation" for
a probe that never looked — which is the failure this register records most
often.

The instrument is the missing piece, and it is small: `plant_severance` already
has the cut machinery and the arm structure; what it needs is a cut placed by
height fraction rather than at the soil line.
