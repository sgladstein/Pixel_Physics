# Lane B — the druid's seeds, and an off switch for her sphere

Branch `claude/druid-seeds-and-sphere`, cut from `91567399` (PR #408's head).
Owner playtest items 4 and 5, 2026-09-14.

What a diff cannot show is below. The mechanics themselves are documented at
their definitions; this is the reasoning, the things that turned out not to be
true, and the one line Lane A needs.

---

## FOR LANE A — the exact call

```rust
/// Returns whether the circle is now *on*.
pub fn toggle_carried_circle(&mut self) -> bool
```

on `pixel_physics::druid::Druid`. It speaks for itself (`Druid::note`), sets
`world.carried` to `None` within the frame the key was pressed in, and clears
`last_wake` on the way back on so the ground underfoot wakes immediately
instead of waiting until she has walked `CARRY_WAKE_STEP` away from it.

For a button's lit/unlit state:

```rust
pub fn carried_off(&self) -> bool
```

Nothing else is needed. The HUD already says `YOUR CIRCLE OFF` in `WARN`
colour when it is off — that is in `Readout`, not in `Interface::draw`, so it
is not in your region.

**Suggested key.** `[` and `]` are the size dial and must stay as they are;
`\` sits beside them on every keyboard layout the legend assumes and is
unbound. But it is your call — the legend guard
(`the_legend_names_every_key_the_binary_binds`) is in your file and will tell
you if the legend and the binding disagree.

**And one more line in `src/bin/druid.rs` that is not mine to write.** `Z`/`V`
write `Druid::speed` directly, with no `Druid` method in between, so the note
the dial raises is entirely yours. Lane E's finding is that the dial
multiplies **grazing** as well as cost — ants eating a wood inside a
quickening, 228 plant cells at speed 1 against **2,217** at speed 8 — and
nothing on screen says so. `Druid::speed`'s doc now carries the measurement;
the sentence the player needs is still missing. Either bind it to a method
here and I will write it, or say it in the dial's own note.

**My footprint in `src/druid/hud.rs`**, all in one commit (`8c980361`) and all
inside `:185-258`, which is ~140 lines clear of `Interface::draw`:

- two fields on `Readout` — `carried_off: bool`, `seeds: u32`;
- the `YOUR CIRCLE` line gains a branch (`OFF` as a word, `WARN` colour);
- the `SEED` line gains the pouch count and goes `WARN` at zero;
- the two `Readout` literals in that file's own tests gain the two fields, and
  the coverage sweep's tuple gains two columns so both branches of both new
  lines are actually formatted.

That is two fields where the licence said one. The second is unavoidable:
`carried_off` cannot be inferred from `carried_radius` (the radius keeps its
value while the circle is off, and the engine reads a zero radius as the
default size anyway), and the seed count cannot be inferred from `sown`.

`Druid::charged_animals` is still there — delete it as planned, I have not
touched it.

---

## Item 4 — no seeds in the generated world

The brief's trace was right in every particular, and the first half really is
three zeroed densities in `assets/worldgen.ron`. The paired ablation table
(`life_scatter` 993 cells → 0, every other pass byte-identical) is in the PR
body. Two things about it that are not:

- **The before-arm is the positive control**, which is the only reason the 0
  means anything: the same instrument reads a known non-zero case at 993 with
  nothing changed but the densities.
- **The registry gotcha did not bite, and the reason is worth keeping.**
  Removing falling seed powder makes `tests/worldgen.rs`'s not-one-cell-moves
  guard *happier*, not angrier. And `life_scatter` still writes on other
  presets, so `worldgencheck`'s every-pass-writes-somewhere half is
  unaffected — a change that zeroed it on *every* preset would have failed
  there, which is that check doing its job.

---

## Item 4, second half — the seed economy, and why this shape

The brief is right that the supply was unlimited and that no decrement existed
anywhere in the repo. The design decision was where seeds come *back* from —
genuinely the owner's call, so what is here is the simplest defensible version
with the assumption stated. **What is built** is in the PR body and in
`README.md`'s `Held world status`; what belongs here is why this shape rather
than another.

**Three choices it rested on, all arguable:**

1. **Per kind, not one number.** "Eight grass and no oak" is a state the game
   can be in, which is what makes cycling the seed kind a decision rather than
   a preference. A single counter is a smaller change and a duller one.

2. **The *carried* circle, not a standing one.** The load-bearing one.
   Gathering is presence — the same thing the carried circle already is — so
   the way to be paid in seed is to walk your own wood. Crediting standing
   quickenings would pay a player who drops a circle over a wood and leaves,
   which is the unlimited supply wearing a delay. It also means switching the
   sphere off stops the pouch filling, for free, out of the same `Option` —
   the two items interlock rather than sitting beside each other.

3. **A cell-count maturity bar rather than the species' own `seed_maturity`.**
   That fence is a plant's business and moves with the genome; this is the
   player's question, which is "does this look like a tree yet". 24 cells
   against a grown tree's 31–153.

**Two shapes considered and not built**, neither reaching `dead-ends.md`
because neither was built. Seeds priced in power: `Setting::Unlimited` already
means power, so the two would become indistinguishable the first time anyone
pressed `U`. Picking loose seed cells up off the ground: more satisfying, but a
loose seed cell belongs to a live organism, so harvesting one is surgery on the
plant line's reproduction path — and it would let her hoover up the
regeneration she is there to encourage. Worth a later session.

**The open question for the owner** — the coordinator says it has been carried
to him verbatim, so this is the record rather than the ask: *is the refill
meant to exist at all, or should a run be bounded by what she sets out with?*
Everything here assumes it should, on law 1 — a supply that can only go down
makes the outcome binary — which is an inference from the ethos and not from
his words.

The four numbers are first guesses and say so at their definitions. **Sweep
`SEED_FROM_CELLS` first**: it alone decides whether the mechanic has a
*middle*. Too low and every sprout pays, which is the unlimited supply again;
too high and nothing pays and the pouch is a countdown.

### What happens near the organism ceiling — §Z21

Lane C's finding, routed here by the coordinator: 12 bits of slot index means
**4,095 organisms**, and a *grown* start already arrives at 4,093 of them.

**The supply is designed against that ceiling rather than ignoring it.** Two
properties, both deliberate: the pouch is finite and capped, so free sowing
cannot run the table up on its own; and a refusal at the ceiling is now **said
out loud and does not cost a seed**. `plant_tree_species` answers a bare
`false` for both "this cell is occupied" and "the engine is out of slots", so
`plant_seed` samples `World::organisms_refused` across the call and
distinguishes them — *"the world is full - nothing can be born until something
dies"* rather than *"no room here"*, which would send a player looking for
better ground that does not exist. A refusal nobody can see is the shape of
bug this game keeps filing.

---

## Item 5 — the sphere's off switch

The reasoning is in the PR body and in `README.md`'s `Held world status`. What
belongs here is the three things a later session would otherwise re-derive:

1. **The brief's warning was right and I confirmed it.** The carried circle at
   its base radius costs exactly zero. `carried_cost` is untouched and its
   guard still green. Nothing about the off switch is a power saving unless
   the player has widened the circle with `]`.
2. **It is a flag and not a radius**, because `carried_radius`'s doc refuses
   to let a dial reach off by accident, `frame.rs` reads `0` as the default
   size, and `Quickening::contains` is `<=` so even a real zero would run time
   for the cell underfoot. All three are asserted directly in
   `the_carried_circle_turns_off_by_the_flag_and_not_by_the_dial`, so a later
   "simplification" of any of them gets told rather than discovered.
3. **Off is a trade.** The colony stops storing charge, sown seed stops
   germinating, the wood stops growing, the pouch stops filling — all of it out
   of `World::time_runs_at`, none of it new code.

`PIXEL_PHYSICS_DRUID_CIRCLE=off` is a **control arm, not a setting**: the key
is Lane A's file and a headless capture cannot press one, so without it the
two arms could only be compared across a rebuild.

---

## Guards — the six-arm sensitivity table is in the PR body

What belongs here is the two *causes*, because both generalise past this lane
and neither is visible in a diff.

1. **The pouch guard never reached a successful sowing.** Its world had no
   ground, so `plant_seed` returned `false` from its first line — every
   assertion was about the refusal path and the spend was never executed.
   `CLAUDE.md`'s *a scene that contradicts the code will look like a bug in
   the code*, arriving as a false **pass** rather than a false failure, which
   is the direction that rule is not usually read in. A second scene error sat
   under it: standing still, the second sowing is refused for *"the cell is not
   empty"*, so a guard that did not move him between sowings would have been
   measuring the ground.

2. **The crediting rule was inline in an organism walk that needs a grown
   world**, so the one claim a player would notice going wrong was unreachable
   by any test that runs in under a minute. Pulled out as `seed_credit`, a
   function over plain values, with a guard that opens on the positive control
   before asserting any of its four `None`s.

---

## Anything in the brief that turned out wrong

Nothing material. Every address the coordinator gave was correct, including
the three asset lines, the `passes.rs:4819-4821` early-out, the `frame.rs`
clamp, and the claim that no seed decrement exists anywhere in the repo.

One correction of emphasis, not of fact: the brief says the sphere's base cost
"is already exactly zero", and it is — but it is worth saying that the reason
an off switch is still worth building is *not* power at all. It is that the
held world had no way to hold still while she was in it. Reporting this to the
owner as "your premise was wrong, here is a switch anyway" would be true and
useless; the switch is the answer to what he asked for, and the arithmetic is a
footnote.

---

## Review cards posted (fire-and-forget, not waited on)

Real captures of the shipped binary through `xvfb-run` + lavapipe, not
`filmstrip` — the app's own screenshot hook puts the HUD in the picture, which
is where half of each mechanic lives.

| card | asks |
|---|---|
| `20260914T044639296Z-dd68c2` | does the bare land read as somewhere worth planting, or as an empty map |
| `20260914T044811000Z-26d062` | does the sphere switched off read as the world holding still, or just as the ring being hidden (**blind A/B**) |

**Collect with `python3 scripts/review.py inbox`.** The second is blinded, so
translate his prose through `blind_was` before believing it. Its context says
outright that the sphere already cost nothing, because he will see
`OUT 0.0/S` on both panes and an unexplained card would read as a broken
mechanic.

Two environment facts that cost twenty minutes here went into `CLAUDE.md`'s
headless-screenshot block rather than this note, because they are not about
this lane: the druid binary writes a *differently named* file, and it does not
exit after the shutter.

---

**Head SHA:** see the last line of `PR_BODY_LANE_B.md`, which is written
against the final commit.
