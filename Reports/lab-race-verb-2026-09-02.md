# A race between two jars: `creature_arena` as a lab verb

**Status:** design, not built. 2026-09-02. Scoped to **the lab**.

**The owner's question, which is the brief:** *"Is this something that would be
useful to implement into the evolution lab? Would it be fun for the users to
play around with, or is it more of a development tool?"*

**The answer: both, and they are two different things sharing one engine.**
`examples/creature_arena.rs` is an epistemics instrument and should stay one.
The game feature is a **tournament between saved specimens**, which is a
different surface over the same machinery.

---

## 1. Why this is a game feature and not only a tool

**It closes the loop the specimen shelf left open.** You can already jar a
creature's genetics, clone it, and mutate it by broods. You cannot find out
whether the mutant is any good. That is a collection with no game in it — and
the rack's own next-verb note says `CROSS` (breed two jars) is missing. Racing
is `CROSS`'s sibling: breed them, then **find out**. It is what gives every jar
on the shelf a reason to exist.

**It is what the owner already asked the lab to be:** *"Give me the tools, data,
access to the parameters that need to be tweaked and I do that testing myself in
the game. That is the game."*

**And it is fast enough to watch, which was the surprise.** A valid run is
24,000 frames (§3.2). The dial runs 12 ticks per displayed frame at 60 Hz
(`M* = 12·D/60`), so that is **2,000 displayed frames — about 33 seconds**. A
match, not a batch job. Anyone assuming this is an overnight harness has not
done the arithmetic.

---

## 2. What already exists, so this is small

| piece | where |
|---|---|
| the whole race engine — arms, mirroring, lineage attribution, the horizon guard | `examples/creature_arena.rs` |
| putting a specific genome into a live organism | `World::set_organism_genome` (`world.rs:3176`) |
| jars, the rack page, and its buttons | `src/sim/specimen.rs`, `src/lab/ui.rs` |
| lab-only material recolour, as a precedent for per-arm tint | `lab/mod.rs`'s `earth_toned_nest` |

Nothing here needs new simulation. It is a surface over validated machinery.

---

## 3. The four integrity requirements

**These are not polish. Three of the four are the difference between a game that
tells the truth and one that lies to the player**, and each is measured rather
than argued.

### 3.1 The mirror runs automatically and invisibly

A colony is founded left-to-right along the ground, so a founder at the end of
the row is not interchangeable with one in the middle — fewer neighbours,
different ground, different distance to the nest patch. The harness runs every
scene **twice** with the arm assignment mirrored (ABAB…, then BABA…) and pools
the pair, which cancels position exactly rather than approximately.

**The player should never see this. It must never be optional.** Measured:
`arm=same mirror=off` — the same genome against itself, position confound left
in — reads **42.9%–70.0%**, two seeds below 50, three above, one tied. That
spread is what the mirror removes, and without it the winner is decided by where
the game happened to stand the founders.

### 3.2 The horizon is enforced, not warned

**This is the one that would actively mislead a player.** At `labbatch`'s
9,000-frame default, the `lethal` arm — a brain with **every weight zeroed**,
an animal that never moves, eats, digs or breeds — read **65.8% of the animals
on 4 seeds of 4**, 52 of 52 alive at generation 0. It beat the authored ant.

It had won nothing. A founder's grant is
`start_energy / (idle_cost_per_cell × cells) × tick_interval` = `200 / (0.05 ×
2) × 6` = **12,000 frames**. Inside a shorter window, not spending is strictly
better and starving is not yet possible. At 24,000 frames the same arms read
**0.0% on 4 of 4**.

The harness computes this from the species file and prints which side of it the
run is on. **A printed caution is right for a developer and useless for a
player**: the lab must extend the run to clear the grant, or refuse to show a
verdict. A player who races at 9,000 frames concludes their worst creature is
their best.

### 3.3 One race is a coin flip, so a race is a series

`labbatch` puts the **world seed alone** at **2.42×–3.12×** across the lab census
with no true effect present. A single match is a sample from that spread wearing
a verdict.

**This looks like it makes the feature worse and it makes it better.** The
honest readout is not *"A beat B"*, it is *"A beat B on 5 of 7"* — which is a
**best-of-N**, a structure players already understand and enjoy. The statistical
requirement maps onto a better game shape than the dishonest version, not a
worse one.

### 3.4 The arms must be visually distinguishable

Two colonies of one species mix within a few hundred frames — which is also why
attribution is by `OrganismState::lineage` and never by position (§4). A player
watching needs to see whose ants are whose. Per-arm tint, on the precedent of
`earth_toned_nest`, which already recolours a material for the lab alone without
touching the sandbox.

---

## 4. Why attribution is by lineage, and why the plant harness could not be copied

`selection_arena` classifies plants by position, because two plants stay where
they were planted. **Animals move.** And genome cannot be the label either,
because it mutates at every birth, so classifying by it loses the descendants.

`OrganismState::lineage` is carried through `Origin::Bud` unchanged, which is
what makes an arm's share countable **even when both arms are the same genome** —
precisely the control that has to work for anything else to mean anything.

---

## 5. The design that falls out

**Where it lives: the rack page, not the tool bar.**

The bar is **full at seven, measured rather than assumed** — `TOOLS` is
`[Look, Plant, Colony, Cull, Soil, Water, Wall]`, and
`the_bar_fits_the_screen_and_no_two_widgets_overlap` refuses an eighth exactly
as it refused a ninth before `KEEP` and `FREE` were dropped. Its own doc warns:
*"run the fit guard before assuming the next lab control has anywhere to live,
and expect it to say no."*

**But two precedents already solve this.** `WALL` shipped *"reachable only by
its key,"* with whether it earned a cell left to the owner rather than forced —
because *"squeezing a control in is how the overlapping columns on the rack page
happened."* And `Tool::Release` ("PLACE") has **no button at all**; its key is
bound directly in `bin/lab.rs`.

So: **RACE is a rack-page action over two jars, plus a key.** That is where
jar actions already live — `KEEP` and `FREE` became rack buttons on 2026-09-01
— and it needs no bar cell.

**The readout is two lines**, and the second one matters as much as the first:

- **who won, and how many of N** — the direction statistic, not a difference of
  means.
- **did each side breed at all** — the harness's `ever` column, distinct lineages
  ever seen per arm. It separates *"your creature lost"* from *"your creature
  never bred,"* which is the difference between a close race and a total
  failure, and a player wants to know which they got.

**One opponent from the dev ladder translates and the rest do not.** `arm=` is a
ladder — `same`, `lethal`, `nofeed`, `notrail`, `random` — built to report
*where* discrimination stops. A player does not want to race a deliberately
lobotomised ant. **`random` does translate**: *"is my creature better than
noise?"* is a legible benchmark and a natural difficulty floor.

**Caveat on shipping `random` as a player-facing benchmark:** its evidence is
currently the authored ant ahead on **5 of 6 seeds**, median 27.3%, one seed at
63.6%. `CLAUDE.md` says flatly that **six seeds is not a sweep** — that rule's
own case went 1.64× over six and 1.08× over the next twelve, pooling to a median
of zero. **Re-run `arm=random` at twelve seeds before it becomes the thing a
player measures themselves against.** The `lethal` result (0.0% on 4 of 4) is
unambiguous enough not to need it.

---

## 6. Two things to decide deliberately rather than discover

**It edges into deferred territory.** The design guide's **Gate 5 — the score
and the economy — is explicitly not being built yet**. A race produces a
verdict, and a verdict is a score. The distinction that probably keeps it on the
right side of the line: it scores a **matchup**, not the player. Worth ruling on
before building rather than after.

**How many seeds is a race?** Seven at ~33 s each is about four minutes of
watching, which is a lot for one answer; three is ninety seconds and is a much
weaker claim. Options, in the order I would try them: run the series **headless
and fast** and show only the last match live; or run three by default with a
"best of seven" the player can ask for. **This is a playtest question, not an
arithmetic one** — it wants the review queue, not a calculation.

---

## 7. What would say it worked, and what would say it did not

**Worked:** a player races two jars they made, gets a verdict they trust, and
goes back to the rack to breed a better one. The loop closing is the whole
point — the shelf stops being a museum.

**Did not:** the verdicts feel arbitrary. That would mean the series is too short
for the noise floor (§3.3), and the fix is more seeds rather than a different
readout. **Do not fix it by hiding the variance** — a verdict the player cannot
trust is worse than a slow one, and this is exactly the failure the mirror and
the horizon guard exist to prevent.

**The guard that must exist before it ships to a player:** race a jar against
**itself**. It must come back at the null, over the same number of seeds a real
race uses. If a creature beats itself, the feature is lying, and every verdict
it has ever shown is void.
