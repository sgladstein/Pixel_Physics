# Rest with an end — round 33, lane F

**The findings are a report, not a note**:
[`../evolution-lab-rest-with-an-end-2026-09-13.md`](../evolution-lab-rest-with-an-end-2026-09-13.md).
What stays here is what another lane needs from this one.

**§Z13 is re-aimed, not closed, and the framing you may have inherited is
wrong.** *"Resting, not blocked"* is still true about the counters; the owner
overruled what it meant (2026-09-13): *"If they never ask to move that is
still stuck, it is just because the rest mechanism needs fixing."* His verdict
on the fixed arm, blind: ***"A looks way better"*** — decoded through
`blind_was: [1, 0]`, his A is the fix.

**Four things that will bite another lane:**

- **Every breeding number in the repo moves from birth 1.** A new brain input
  (`Stillness`) puts `live_slots` at **870**, so every species'
  `mutation_rate` is re-derived to `3.18 / 870 = 0.0036552` and
  `random_genome` draws shift. **`origin/main` is therefore not a valid
  control arm for anything behavioural on this branch** — use
  `wire=Stillness:Move:0.0`, which is today's behaviour inside one binary.
- **`labgif wire=` was a silent no-op until this landed** (§Z20). It wrote the
  genome before `load_scenario`, which rebuilds the world. Any arm you took
  with it is the control wearing a label — re-run it.
- **`Move`'s negative half is the homing mechanism**, not spare headroom. Any
  term you add to that output has to answer what the output's existing *low*
  values are already doing; a linear version of this fix took `deliveries`
  from 4,908 to 54 while the colony looked fine.
- **`labforage` gained three readouts**: `SUMMARY per_animal` (per-animal idle
  fractions, the two tails, and the latch test), `SUMMARY rest_bouts` and
  `SUMMARY standing_now`. Appended after `main`'s fields, never interleaved.

**Left open**: the ~40% drop in deliveries per ant-tick is real and is not
isolated from colony drift. The clean repair is a signal separating
*navigating* from *resting* as data rather than by timescale; nothing in the
brain says which an animal is doing.
