# Sounding the rock, and ore in the pillar that holds the roof

**Status: design, nothing built.** The second half needs no new simulation at
all — only worldgen placement and a reason to care.

Two ideas in one report because neither is a game on its own and together they
are a loop: **sounding** is how the player reads the structure, and **ore in
pillars** is the reason they have to.

---

## Part 1 — Sounding: the model knows the keystone, the player cannot ask

### What is already computed and discarded

`load::failing_along_support_chain` walks a cell's ancestors and returns
`Failure`, whose position field carries a comment that says the whole thing:

> The position is not the cell that was checked — it is the ancestor the chain
> walk found to be over its limit, which may be many cells away. Carried out so
> the impulse, and therefore what the eye reads as the origin of the collapse,
> lands on the joint that gave way.

**The engine already identifies the keystone.** It uses that knowledge to aim
an impulse and then discards it. The player has no way to ask "what is holding
this up" short of hitting it and finding out.

### The design: make the hammer's light swing a question

`rigid::strike` is the hammer. A *light* swing — modifier held, or a separate
bind — does no damage and instead reports:

- **the load ratio at the struck cell**, as `design-load-telegraph.md`'s dust
  and creak, in a one-off burst rather than continuously;
- **the load path**, briefly: walk `support_parent` from the struck cell and
  flash that chain. This is `chain_reaches_anchor` rendered, and it is a
  complete answer to "what is holding this up" in one keypress;
- **whether it arrives at all** — a chain that reaches an anchor and one that
  does not are different pictures.

The walk is already bounded (`ROOTWARD_CHECK_STEPS` = 48, `MAX_SUPPORT_WALK` =
512), costs one evaluation, and happens on a keypress rather than per frame.

### Why this is not a debug overlay

A stress heatmap answers "where is the model worried" for the whole screen at
once, and `CLAUDE.md` records a corrected overlay still being misread. Sounding
answers **one question the player asked, at a place they chose**, and it costs
them an action to ask. That is the difference between a readout and a verb.

### The trap this must avoid

`rigid::is_tool_target` takes `Solid | Plant` and refuses bedrock, and the §S
measurement burned a whole run on a probe that swung at soil and reported 200
cuts and **0 cells removed**. A sounding verb that silently answers about the
wrong cell is the same failure wearing a UI. **It must report what it actually
struck** — material and position — not just a number.

---

## Part 2 — Ore in load-bearing rock

### The oldest dilemma in mining games, and here it is simply true

Every mining game gestures at "don't dig out the support". None of them
simulate it, so it is either a scripted event or a lie. This engine computes
it every frame: a pillar's cells are on the load path of everything above
them, `dependants` says so, and removing them re-routes load into whatever is
left.

**So the feature is not a mechanic. It is a placement decision.** Put the
valuable seam where the load is.

### What already exists

`assets/materials/` carries `crystal`, `spar` and `shard` — Solids that read
as valuable and are currently cave decoration. Nothing new has to be invented
to try this; `crystal` in a pillar is a complete prototype.

### The design

- Worldgen places a seam that **runs through** structural rock rather than
  beside it, so extracting it and keeping the roof are in tension.
- The player's tools already do the rest: mine it and the load re-routes; the
  roof either finds another path or does not.
- Sounding (Part 1) is what turns this from a coin flip into a decision. A
  player who can ask *"is this column carrying anything"* is playing;
  one who cannot is gambling.

### The graded outcome, because a binary one is the failure mode

`CLAUDE.md`'s first law again. "Take the ore and the roof falls" is a binary,
and a binary reads as a trap. The middle already exists in the model and only
has to be exposed:

| how much you take | what the model already does |
|---|---|
| a little | load re-routes through the remaining section; `capacity` falls with section depth *squared*, so it gets measurably closer to its limit |
| more | the telegraph starts: dust, creak, the ratio climbing |
| too much | `failing_along_support_chain` finds the ancestor over its limit and the region comes away |

So a careful player takes most of the seam and leaves a rib. A greedy one gets
buried. **Neither outcome is authored** — both fall out of `capacity`.

### The long tail nobody is using

`Cell::attached` is lost by destruction and **never regained** — the module
doc is explicit that attachment is only ever lost. So worked rock is
permanently weaker than virgin rock, for the rest of the run. That means:

- **your own old workings are the hazard.** Most games make explored space
  safe; this engine makes it structurally scarred forever, and nothing
  currently draws attention to it.
- a gallery you stripped an hour ago is a genuinely worse place to be than one
  you have not touched, with no bookkeeping needed to make it so.

That is an unusual survival loop and it is *already in the data*.

## What it costs

- **Sounding**: a bind, a chain walk on keypress, and a way to draw the flash.
  No new per-frame work.
- **Ore in pillars**: a worldgen placement rule and, if the existing materials
  are not right, one `.ron`. No simulation change.

## The falsifying experiments

| question | how | what falsifies it |
|---|---|---|
| **is the keystone stable enough to be worth pointing at?** | log `Failure::at` across `acceptance.sh`'s collapse cases; check the same cell is named on repeated runs of the same scene | the position jittering run to run — then it is not a keystone, it is whichever cell the walk reached first, and pointing at it teaches the player something false |
| **is the middle actually reachable?** | take a pillar and remove cells one at a time, plotting `torque/capacity` against cells removed | a step function — if the ratio sits flat and then the roof goes, there is no graded outcome to expose and this needs `capacity` work first |
| **does a seam through a pillar produce a real decision?** | place a seam, then measure the largest fraction extractable before failure, over a **seed sweep** at the order statistic | "you can take 99%" or "you can take 5%" — either makes the decision fake |

**Run the second one first.** It is a loop over `load::evaluate` on a
hand-built pillar and it decides whether this is a design or a trapdoor.
`CLAUDE.md` is explicit that a mechanic which is either thriving or gone has
the same defect the rubble did.

## The judge-by-eye question

**Does a sounded chain read as information or as noise?** A flash along 40
cells of load path is a lot of screen. Blind A/B through `scripts/review.py` —
the same structure sounded three ways (path only, ratio only, both) — and ask
which one answers "can I dig here". The instruments index's own warning
applies: a debug channel that is a function of the thing it debugs is
worthless, so the flash must be a full replace on a fixed ramp, not a blend
into the rock's own colour.
