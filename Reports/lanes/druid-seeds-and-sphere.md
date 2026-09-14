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

### The brief's trace was right, and it is three asset lines

`assets/worldgen.ron`, the `druid` preset: `moss_density`, `tree_density`,
`grass_density` all to `0.0`. `passes::life_scatter` early-outs on all three
at zero, which is the path `arid` and `flat` already take.

**Measured rather than assumed**, paired, one binary and two copies of the
file (`pass_ablation seeds=1 preset=druid`):

| | before | after |
|---|---|---|
| `life_scatter` | **993** | **0** |
| `soil_blanket` | 1,239,668 | 1,239,668 |
| `ponds` | 94,493 | 94,493 |
| `soil_moisture` | 1,223,852 | 1,223,852 |
| `moisture_init` | 71,324 | 71,324 |

The before-arm is the positive control: the instrument reads a known non-zero
case at 993 and the same case at 0 with only the densities changed. Every
other pass is byte-identical across the arms, so nothing was displaced into a
different pass.

`Start::Bare` never grows (`grow = 0`), and nothing else in the generator
plants, so that is the whole of it. `Start::Bare`'s doc said "plus
`life_scatter`'s single seed cell per plant" and has been corrected.

### The registry gotcha did not bite, and here is the evidence

`tests/worldgen.rs` sweeps `presets()` and one guard asserts not one cell
moves in the 120 frames after generation. Removing falling seed powder should
make that guard *happier*, and it does — full `cargo test --release` green
(numbers in the PR body). `bash scripts/worldgencheck.sh` green.

The half of that guard worth naming: `life_scatter` still writes on other
presets, so the every-pass-writes-somewhere half of `worldgencheck` is
unaffected. A change that zeroed it on *every* preset would have failed there,
which is the check doing its job.

---

## Item 4, second half — the seed economy, and why this shape

The brief is right that the supply was unlimited and that no decrement existed
anywhere in the repo. The design decision was where seeds come *back* from.
This is the part that is genuinely the owner's call; what is here is the
simplest defensible version, and the assumption is stated so it can be
overturned cheaply.

**What is built.** A pouch per kind (`Druid::seeds`, index-parallel to
`seed_kinds`): `SEED_START` 8 of each at the start, `SEED_CAP` 24. Sowing
spends one, and only on a seed that actually goes in the ground. An empty
pouch refuses and says so. The pouch refills from **mature plants standing in
the circle she carries** — `SEED_PER_PLANT_SECOND` 0.005 per plant per second,
credited to the plant's own kind, gated at `SEED_FROM_CELLS` 24 cells.

**Three things that shape rested on, all of them arguable:**

1. **Per kind, not one number.** "Eight grass and no oak" is a state the game
   can be in, which is what makes cycling the seed kind a decision rather than
   a preference. A single counter is a smaller change and a duller one.

2. **The *carried* circle, not a standing one.** This is the load-bearing
   choice. Gathering is presence — the same thing the carried circle already
   is — so the way to be paid in seed is to walk your own wood. Crediting
   standing quickenings instead would mean a player who places a circle over a
   wood and leaves gets paid for ever, which is the unlimited supply wearing a
   delay. It also means switching the sphere off stops the pouch filling, for
   free, out of the same `Option` — the two items interlock rather than sitting
   beside each other.

3. **A cell-count maturity bar rather than the species' own `seed_maturity`.**
   That fence is a plant's business and moves with the genome; this is the
   player's question, which is "does this look like a tree yet". 24 cells
   against a grown tree's 31–153.

**What I considered and did not build.** Buying seeds with power — rejected,
and the brief is right to warn against it: power is the animal economy and
`Setting::Unlimited` already means power, so a seed priced in power would make
the two indistinguishable the first time someone pressed `U`. Picking up loose
seed cells off the ground — more satisfying, and it is what the world actually
produces once plants reproduce, but a loose seed cell is a `Seed`-typed cell
belonging to a live organism, so harvesting one means organism surgery on the
plant line's reproduction path, and it would let her hoover up the
regeneration she is there to encourage. Left for a later session with a clear
run at it; noted here rather than in `dead-ends.md` because neither was built.

**What I would ask the owner**, if a message could reach him: *is the refill
meant to exist at all, or should a run be bounded by what she sets out with?*
Everything above assumes it should — a supply that can only ever go down makes
the outcome binary (you have seeds or the game is over), which is the failure
law 1 names. But that is an inference from the ethos, not from his words.

### Numbers that are first guesses

`SEED_START` 8, `SEED_CAP` 24, `SEED_FROM_CELLS` 24, `SEED_PER_PLANT_SECOND`
0.005. Every one is a first guess, and each says so at its definition — the
same footing as the rest of this economy, whose own doc says the numbers are
"wrong in the way a first guess is wrong". At 0.005 a wood of twenty mature
plants under her feet pays a seed every ten seconds.

**The one I would sweep first is `SEED_FROM_CELLS`**, because it is the only
one that decides whether the mechanic has a *middle*: too low and every sprout
pays, which is the unlimited supply again; too high and nothing ever pays and
the pouch is a countdown.

---

## Item 5 — the sphere's off switch

### The brief's warning was correct and I can confirm the arithmetic

The carried circle at its base radius costs exactly zero, and there is a guard
saying so. Nothing here is a saving unless the player has widened it with `]`,
and then it is the widening that stops being billed. I did not touch
`carried_cost` and its guard is untouched and still green.

So what is built is the thing the words say: a way to make the world **hold
still** where she stands.

### Why it is a flag and not a radius

`World::carried_radius`'s doc refuses to let a dial reach off by accident, and
that comment is load-bearing — `frame.rs:101` reads `0` as the default size,
so a dial turned to zero gives a *default-sized* circle, and even a genuinely
zero radius would still run time for the cell underfoot because
`Quickening::contains` is `<=`. Both of those are asserted directly in
`the_carried_circle_turns_off_by_the_flag_and_not_by_the_dial`, so a later
session that "simplifies" either will be told.

`World::carried_off` is therefore its own word. Default `false`, so the
sandbox and the lab see no change at all.

### It is a trade, not a free button

Off, the colony under her feet stores no charge, a seed she has sown does not
germinate, the wood she is in stops growing, and her pouch stops filling. Every
one of those falls out of `World::time_runs_at` and needed no code — the same
way the rule that a colony must be founded inside running time does.

### The delivery, per law 2

The world going still around her is the visible consequence: with
`world.carried` at `None` the held look reclaims the ground she is standing on,
so the bubble does not dim — it disappears. The HUD says `YOUR CIRCLE OFF` in
warning colour, which is the half that stops an empty power bar reading as a
fault.

`PIXEL_PHYSICS_DRUID_CIRCLE=off` is a **control arm, not a setting**: the key
is Lane A's file and a headless capture cannot press one, so without it the
two arms could only be compared across a rebuild. One binary, one switch.

---

## Guards, and the two that were blind

Both new guards were written after the code, so neither gets `CLAUDE.md`'s
already-watched-it-go-red exemption. Putting the faults back found the first
version of the pouch guard **blind twice**:

| fault put back | first version | now |
|---|---|---|
| decrement deleted | **passed** | fails |
| empty-pouch refusal deleted | fails | fails |
| credit the selected kind, not the plant's | **passed** | fails |
| drop the carried-circle gate | **passed** | fails |
| drop the maturity bar | **passed** | fails |
| `frame.rs` ignores `carried_off` | fails | fails |

Two distinct causes, both worth knowing:

1. **The guard never reached a successful sowing.** Its world had no ground,
   so `plant_seed` returned `false` from its first line — every assertion was
   about the refusal path and the spend was never executed. This is
   `CLAUDE.md`'s *a scene that contradicts the code will look like a bug in
   the code*, arriving as a false pass rather than a false failure.
   `bare_for_test` now lays a floor and reloads the registries.

   A second scene error under it: with the player standing still, the *second*
   sowing is refused for "the cell is not empty" — a seed already lying there
   — so a guard that did not move him between sowings would have been
   measuring the ground.

2. **The crediting rule was inline in an organism walk that needs a grown
   world**, so the one claim a player would notice going wrong was unreachable
   by any test that runs in under a minute. It is now `seed_credit`, a function
   over plain values, and its guard opens with the positive control (a grown
   tree in the circle pays into the *tree* pouch) before asserting any of its
   four `None`s.

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

**Head SHA:** see the last line of `PR_BODY_LANE_B.md`, which is written
against the final commit.
