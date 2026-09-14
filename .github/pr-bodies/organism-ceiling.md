**The world can hold a million living things instead of 4,095, and when it
refuses a birth it says so.** The held world could not found a colony: `C`
is the only verb in that game that makes an animal, and on the grown and
dead starts it placed 2 animals over nine stands while the bar said
*"nothing founded - no ground here"*. The ground was fine. The world was
out of identities, and nothing above the allocator could tell you that.

Play-facing: the druid's founding message now names which of three
refusals fired. Engine-facing: the organism ceiling moves off 4,095, which
every readout in the repo had spelled out by hand.

Closes [`Reports/open-bugs-handoff.md`](../Reports/open-bugs-handoff.md)
§Z21. README's *M16 status* and `PLAN.md` both gained the new numbers.

## The ceiling, 4,095 → 1,048,575

`Cell::organism_id` was a `u16` split 12 bits slot index / 4 bits
generation. It is a `u32` split **20 / 12**, so `Cell` goes 12 → 16 bytes.

**The generation half was widened deliberately rather than the index taking
everything.** The wrap that bounds stale-handle aliasing goes from **16
reuses to 4,096**, so the widening makes that safer rather than trading it
away. That matters because stale handles are not hypothetical here:
detached tissue keeps its `organism_id`, so litter naming a dead organism
lies around any grown world.

The free alternative — re-splitting the old `u16` as 13/3 for 8,191 slots —
was refused for exactly that reason and is recorded in
[`Reports/dead-ends.md`](../Reports/dead-ends.md) with its re-test
condition. It halves the aliasing margin to buy a ceiling the druid grow
phase reaches after two more passes at 4,093 organisms each.

## What it costs, measured rather than argued

Paired alternating runs of two fixed binaries, three rounds, `frame_profile`
on the shipped 8192x2560 world, settled block:

| | before | after |
|---|---|---|
| cell grid at 8192x2560 | 240 MiB | **320 MiB** (+33%) |
| `ca_sweep` p50 median | 5.51 ms | 5.71 ms (+4%) |
| whole frame, mean | 19.05 ms | **18.96 ms — unmoved** |

The memory is the real price and the frame does not see it: the frame is
47% draw and 22% fields, so 0.2 ms on the sweep sits inside run-to-run
spread.

**`examples/ascii`'s worst frame could not separate the two arms at all** —
the 16-byte build won 3 of 6 pairs. That is `CLAUDE.md`'s
order-statistic-is-noise case, so the mean and p50 are what is quoted here
and the worst frame is not.

## Where the three games actually sit

Measured for this change, because the ceiling was being argued about
without anyone having read it:

| | slots used | refused |
|---|---|---|
| outdoor at generation, 8192x2560, 4 seeds | 100–333 / 4,095 | 0 |
| outdoor, 2048x640, grown 20,000 frames | **882** / 4,095, still climbing | 0 |
| lab herb at 45,000 frames | 1,812–2,503 | 0 |
| druid `Start::Grown` | **4,095 / 4,095** | 26 |

The second row is new: §Z21 explicitly flagged "a played world grows into
the ceiling" as unmeasured, and it does — eightfold over 20,000 frames and
not levelling off.

## The message

Both founding paths read `World::organisms_refused` either side of the
founding and route through one `refusal_note(stations_offered, no_slots)`,
so the two verbs cannot drift apart:

- *"no room for another living thing - the world is full"* — the allocator
  turned the births away
- *"no ground here - stand on something solid"* — no station was offered
- *"no room - the ground here is full. try open ground"* — stations were
  offered and none took

Slots is named first because it is the only one of the three the player
cannot act on by moving, which is what both other wordings tell them to do.

## Two latent bugs that fell out of the widening

`src/lab/mod.rs` and `examples/creature_probe.rs` censused organisms by
scanning `1..4096u16` by hand. That hardcoded the old ceiling **and was
already wrong before it moved**: a bare slot index carries generation 0, so
`World::organism` resolved it only while the slot had never been reused,
and every organism in a recycled slot was invisible to both censuses. Both
now iterate `World::live_organism_ids`.

The lab's slot row also coloured amber at a spelled `>= 4000` — 0.4% of the
new ceiling — and named `4095` in its help text. Both now read
`World::organism_slot_high_water`.

## Guards, each watched going red under the fault it names

- `the_organism_ceiling_is_past_the_old_four_thousand_and_ninety_five` and
  `a_handle_round_trips_above_the_old_twelve_bit_index` both fail with the
  split put back to 12/4.
- `a_world_out_of_slots_says_so_instead_of_blaming_the_ground` and
  `the_two_ground_refusals_are_unchanged_and_distinct` both fail with the
  slot arm removed.

The ceiling guard asserts **distinctness of handles**, not a success count:
§F4's original bug was not a refusal but silent aliasing, and a count would
pass straight through it. The slot-exhaustion guard **fills every slot for
real** (~0.6 s, measured) rather than setting a flag, because the claim
being guarded is that the counter moves when the world is full — and it
carries the quiet-when-nothing-is-wrong control beside it.

Two superseded tests were updated rather than deleted: both asserted the
16-reuse cycle, and the property they guard is unchanged at 4,096.

## One visible consequence worth knowing about

The renderer hashes the handle to decide which trees draw in front of the
gnome. A generation-0 slot encodes to the same number under both splits, so
**a freshly generated world is pixel-identical**. An organism in a *reused*
slot shifts (generation 3, slot 1234: 13,522 → 3,146,962), so a minority of
trees in a long-played world will flip which side of him they draw on.
Deterministic and stable within a build, and the property the guard asserts
— both answers occur, and a given tree keeps its answer — is untouched.

## Not fixed here

The druid grow phase still produces ~4,093 small organisms and a senescent
plant still holds its slot in a held world where nothing rots. The ceiling
is simply far enough away that neither is a failure now. §Z21's other two
candidates (fewer, larger organisms; freeing a senescent plant's slot on
`Start::Dead`) stay open as economy questions rather than as bugs.

## Gates

`cargo test` 1,808 lib + 44 worldgen + 3 determinism + 10 bin, 0 failed.
`cargo clippy --all-targets --release --locked -- -D warnings` clean.
`scripts/acceptance.sh` all cases met. `examples/ascii` 31 scenes, 0
skipped. `scripts/docscheck.sh` clean.

`OrganismId` is a type alias rather than a newtype, so the next widening is
one line instead of another sweep of 88 signatures.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_016eoZhh9rh5TySDGhePp26r
