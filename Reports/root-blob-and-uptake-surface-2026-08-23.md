# The root blob, and whether pricing soil contact would stop it

**Status: measurement, no mechanism built.** A follow-up to
`world-flora-sowing-2026-08-23.md` §6, written to size a lever the owner
proposed before a package is spent building it (`CLAUDE.md`: check that a
planned step can demonstrate itself, before promising it will). `Absorb`
lives in `plant.rs` and belongs to the plant-core lane; nothing here changes
root code.

## 1. What prompted it

The B1 line ended with two root treatments converging: the slot-5 tropism
axis peaks at 25,200 frames and is gone by 43,200, and both mature systems
render as a dense mass rather than a shape. Posted, and the owner's
direction back:

> *"We should evaluate both. Not sure which makes sense to do first. There
> should be a disadvantage for growing a big blob of roots that fully fills
> in all space. If the root cell isnt touching soil it cannot benefit the
> plant and has a cost or maybe it or maybe there are costs and benefits but
> it only grows bloby in certain environments but in most it doesn't. As
> usual we don't want to force the roots to grow a certain way but set up a
> system that leads to interesting and heterogenous results, not every plant
> root eventually grows into the same blob that is worst case"*

That is a proposal about **uptake surface**: a root cell walled in by its own
siblings shares no face with soil, so it can absorb nothing while still
costing carbon. If the interior share grows with the mass, the blob is
self-limiting with no rule saying so — which is exactly the "system, not
preset" constraint `root-morphology-findings.md` records.

## 2. The measurement

`examples/root_contact.rs`. Grove scene, 8 tree founders, 100-row bed, the
fraction of root cells sharing a **face** with soil — a face because an
exchange crosses one, which is why `diffuse_resource` is 4-neighbour while
growth places at 8. The eight-neighbour figure is printed beside it so the
choice is visible rather than assumed.

| frames | root cells | touching soil (4-nbr) | (8-nbr) | walled in |
|---|---|---|---|---|
| 10,800 | 2,545 | 66.7% | 85.9% | 33.1% |
| 25,200 | 4,248 | 63.0% | 82.0% | 36.1% |
| 43,200 | 9,854 | 65.9% | 83.1% | 33.3% |

## 3. Two findings, and the second changes the proposal

**The lever is not a rounding error.** A third of every root system is walled
in by its own siblings at every age measured. Those cells can absorb nothing
and cost carbon to build. Pricing them is charging for something real.

**But the interior share does not rise with the mass**, and that is the part
that matters. Root cells nearly quadruple between the first and last row
while the interior holds at about one third. A filled disc would go the other
way — its interior share rises with radius — so what reads as a blob at
render zoom is a **dense branching mat that keeps a constant
surface-to-volume ratio at every scale**, not a solid body.

So the cost as specified is a **flat ~33% tax on root mass**. It makes roots
cheaper to run in aggregate; it does *not* make large ones disproportionately
expensive, and on its own it will not stop the mass growing. The mechanism
the owner reached for is sound about what is wasteful and does not, by
itself, deliver the brake he wants from it.

**Anything meant to stop the blob has to be scale-dependent**, and two
candidates already exist in the engine rather than needing invention: a cap
tied to the canopy the roots feed (the pipe model already relates
cross-section to foliage above), or crowding among a plant's own root tips —
the shoot has `crowding_weight` doing exactly this job and the root block
deliberately sets it to `0.0`. Neither is proposed here, and neither is
measured.

## 4. The finding that supports the owner's real goal

Per plant at 43,200 — same genome, same scene, same carbon:

| root cells | touching soil |
|---|---|
| 638 | 51.3% |
| 691 | 61.8% |
| 697 | 57.5% |
| 859 | **79.2%** |
| 1,427 | 65.9% |
| 1,731 | 61.1% |
| 1,861 | 75.0% |
| 1,950 | 64.8% |

**The heterogeneity the owner wants already exists — it is just not priced.**
One individual runs at 79% contact and another at 51% for comparable mass.
Today nothing rewards the efficient one, so the spread is invisible and
cannot be selected on. Pricing contact would convert an existing 51-79%
spread into a fitness difference **without forcing any shape**, which is the
"set up a system that leads to interesting and heterogeneous results" the
constraint asks for.

That is the strongest argument for doing it, and it is a different argument
from the one the proposal was made on.

## 5. The question this report was posted with is still open, and the card is why

Posted as a card asking: *take the cost anyway for the 51-79% spread it
exposes, or does the brake have to come from somewhere else?* It came back
answered with **a pane selection and no comment** — `choice: 1`, "slot 5
high (gain 0.84) at 43,200".

**That is not an answer to the question, and the fault is the card's.** It
was posted as a `before_after`, whose affordance is *pick a pane*, while the
question in its text was an either/or about a mechanism. The two panes were
illustrations of one phenomenon, not options. So the only structured reply
available was to choose a picture, for a question that was not about
pictures — `CLAUDE.md`'s "ask one answerable question", failed by choosing a
card kind that could not carry it.

**Do not read `choice: 1` as a decision.** The cost-versus-brake question is
untouched, and the owner's own standing position on it is the earlier
verdict: *"We should evaluate both. Not sure which makes sense to do
first."*

The lesson generalises past this card: a *design* question with no visual
component does not belong in the review queue at all, whose job is judgement
by eye. This one belonged in the PR and the report, where it now is.

## 6. What this does not answer

- **Whether a priced interior changes the grown shape at all.** A flat tax
  might simply shrink every root system by a third and leave the form
  identical. That needs the mechanism built and a paired render.
- **Whether "only blobby in certain environments" falls out.** The owner's
  second clause — costs and benefits that make the blob rational in some
  soils and not others — is a moisture-dependent version of the same idea
  and is not measured here at all.
- **The censoring caveat from `world-flora-sowing-2026-08-23.md` §6 still
  stands**: the deep arm's deepest individual sits on the 100-row bed floor
  from 25,200 on, so these systems are partly bed-shaped. A 200-row bed
  needs a `height=` knob the harness does not have.
