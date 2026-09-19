# Lane T — the trail line's rejections, re-evaluated (ant-survey round)

*Session `session_018P1mVfE1HKA1uDesCHWw3Y`, coordinator
`session_01HTNLNphUPgpg5GqCwCQvmW`. Review and test only, by the owner's
instruction mid-round: the trail lane (`session_01X2rqLneJDzw21CcqoFD7aE`,
on `claude/upbeat-shannon-cez0w4`) builds; this lane measures and writes
back. Full report:
[`ant-survey-trail-reevaluation-2026-09-19.md`](../ant-survey-trail-reevaluation-2026-09-19.md).*

## 2026-09-19

**Branch:** `claude/ant-survey-trail`, cut from `claude/upbeat-shannon-cez0w4`
at `a3190843` (the trail-lifetime split, `TRAIL_A_RHO 0`), `origin/main`
(`404bfe2f`, PR #475) merged in. **The split is carried by this PR**; the
trail lane was told by poke (15:30Z) and can veto it in its note.
`main`'s plane is one rider away (`arho=0.03`) and every headline was also
taken there. Head SHA and PR number: at the bottom, updated as they move.

**What was found, in one line each** (36 seeds, gap 90, paired within seed;
tables in the report):

- **Odometer on `EmitB`: inert at both reader gains, scored on recruitment.**
  Third re-run; the first two never varied the reader. Written back.
- **The food-trail reader's de-saturation (`ac02ac03`, 09-18) is the whole
  hand-trail effect** — 22 colonies of 36 against 4 with the old deaf pair,
  52% of ants at the food against 5%. It overturned a dead end and the
  record was never told: `dead-ends.md`, `wiki/ants.md` (two paragraphs)
  and `trailfollow`'s note all still said deaf-on-purpose, and PR #475
  quoted them. Corrected here.
- **The colony's own channel B is a net cost**: ants that lay none keep 29
  colonies alive against 22 and close 107 trips against 65. `self` ≡ `mute`
  in every configuration. A `(Crowding, EmitB, −4.5)` survival gain (31
  colonies) is the same cost effect, shown by that control.
- **Channel B's decay band (0.10, 0.25) is inert** on the gap bed; the
  played-bed half (`labforage bdecay=`) is in the report §5.
- **Trophallaxis** on the played bed, 12 seeds, 150,000 frames: report §6.
- **Laden right-of-way**: not built (this lane tests). The diff exists at
  `8071d002`, withdrawn at `a1d1730a`; the scoring run is in report §7 and
  was offered to the trail lane.

**→ coordinator.** Three things are yours to decide, none of them this
lane's: whether the lifetime split lands through this PR or the trail
lane's; whether `trailfollow`'s default `gate=` should stop being the stale
`saturated` (it is a harness default, one line, and every run that omits
`gate=` is a three-change comparison); and the standing fact that the
colony cannot lay a trail worth following and pays to lay one — the
survey's core recommendation presupposes the opposite, and no candidate in
this round moves it.

**→ Lane N.** Nothing here touches the dig or drop verbs, `ant.ron` or
`world.rs`. `who-touched` on `creature.rs` at 14:50Z: your branch was not
on the remote yet; `nest-biology-research` and `upbeat-shannon` were the
only live holders.

**Traps met** (report §8): `trailfollow`'s default gate saturates the homing
pair too; `gate=b2` is a no-op since the 18th and panics rather than lying;
`pkill -f` on a pattern matching the calling shell.

Head: *(updated on push)*. PR: *(opened when the queues finish)*.
