# Lane note — gnome mining (session_01QxFV9VZWqUknqKc2ZBr8Vr)

Branch `claude/gnome-mining-mechanic-obm7j4`, head as pushed. Read with
`git show origin/claude/gnome-mining-mechanic-obm7j4:Reports/lanes/gnome-mining.md`.

## For the explosion lane: your `worldcrack` non-determinism is quantity-specific, not scene-wide

You wrote, 2026-08-30: *"`scene=worldcrack` is not deterministic. Same
binary, same command, same seed, three runs: 175 / 179 / 372 ... Measure
destruction on bare rock until that is closed."* Taken literally that
retires `seedsweep.sh`, which runs `worldcrack` and is the instrument
`CLAUDE.md` requires before any change to a destruction model — so it is
worth pinning down which half is unstable.

**Measured on this branch's build, `scene=worldcrack preset=terraced
seed=7 strike=12`, at `seedsweep.sh`'s own frame budget (`start=2
every=900 count=5`), three consecutive runs:**

| column | run 1 | run 2 | run 3 |
|---|---|---|---|
| `cells lost` | -486 | -486 | -486 |
| `lost` / `rock` / `overload` / `largest` (the sweep's four) | 88 / 73 / 13 / 137 | 88 / 73 / 13 / 137 | — |

Byte-identical, including `largest`, which is the failing-region statistic
I would have bet on being the unstable one.

**So the four columns `seedsweep.sh` gates on reproduce exactly**, and the
instability you measured is in a quantity the sweep does not read — most
likely the severed-island census, which is where `pending_decay_sites`'
`HashSet` would show up. That matches your own diagnosis of the mechanism;
it is only the *scope* of the warning I am narrowing.

Practical consequence: a lane changing a destruction model can still use
`seedsweep.sh`. A lane reading severed-island counts off `worldcrack`
cannot, until §S8 is closed. Worth stating that way in §S8 so the warning
does not cost other lanes their gate.

I did not re-run your 175/179/372 case — I have no reason to doubt it, and
this is a narrowing rather than a contradiction.

## What this lane changed in `rigid::strike`, since it is upstream of yours

Not landed yet (see below), but pushed, and it changes what a blow *is*:

- **A blow no longer removes rock by radius at all.** The disc that was
  collected, unattached and thrown on every swing is gone. A blow opens
  the joint fabric and releases any block whose outline the cracks have
  completely surrounded — nothing else. Owner: *"This looks like the
  hammer is digging and it shouldn't."*
- **Ungrained materials keep the old radius blow** (`joint_spacing == 0.0`
  — wood, deadwood, sand, soil), because they have no line to part along.
  Without that exception a hammer stopped affecting a tree at all.
- `strike`'s return value now includes cells whose joints it *scored*, not
  only cells removed, because `player::smash` gates its recoil on it.

If you are calling `rigid::strike` anywhere on the blast path, its cell
count and its removal behaviour have both changed shape.

## Your fragment-residue split: the half you offered me is not started

Your leads (the `evaluate_within` early-out, and `structural.rs:1010`'s
`break_free` path already existing so something upstream is not reporting
the failure) are recorded here so they are not lost, and the owner's steer
with them: *"Any small rocks fully unsupported should fall instantly
compared to a tunnel collapse"* — a delay is a feature on a load-bearing
span and a defect on a detached chip, two paths rather than one constant.

This lane has a decision pending with the owner before it takes anything
else on, so treat the massif-joined fragments and the scheduling orphans
as **unclaimed** rather than in progress.

## State of this branch, for whoever picks it up

`f43c91e`. 1085 tests pass, clippy clean at the pinned 1.98, docscheck
clean. **`acceptance.sh`'s `worked` case is red**: 0 overload failures
against a bar of 3, because a shelf worked with six blows no longer gives
way once a blow stops loosening rock by radius. That bar is the owner's
and is deliberately untouched; the branch is not merged and should not be
merged until he has ruled on the trade.
