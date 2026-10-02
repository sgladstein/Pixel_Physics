# Nest lane: handoff (2026-10-02, 22:05 UTC)

For a fresh Claude with no context. Repo `sgladstein/Pixel_Physics`, main
`0738a8ca`. Read `CLAUDE.md` first; it is binding.

## What the lane owns

The ant colony's nest: where "home" is, digging and nest shape, what the
caste of nest workers does, and where food is kept inside the nest.
- **Not owned:** laying, brood stages, `brood.rs`, `kin_deficit` and `nurse`
  (breeding lane); the forage throttle and drives (foraging lane).
- Eat-on-arrival belongs to the breeding lane's open "fed ants store" card.
  Don't change it.

## State

**Merged and on by default:**
- PR 539: nest roof, `DIG_ROOF=6`.
- PR 541: kin footing, `PIXEL_PHYSICS_KIN_FOOTING`. Ants grip a nestmate
  standing on ground.
- Earlier: the door, the dug shaft, the walked spoil cycle, stacking at 4.

**PR 547 MERGED** as main `e8adc960` (22:25, piling on; the storeroom card was still unanswered). **Open: PR 549**, a docs-only dead-ends index fix on the same branch restarted from e8adc960. PR 547 was: branch
`claude/nest-piles-va71j6`, head `886949fc`, main `0738a8ca` merged in. It
ships two defaults:
1. **Dug home** (`NestHome::Dug`). Home is every open cell a flood fill
   reaches from the door or the founding cut, within 60x60, recomputed every
   256 frames by `World::step_nest_dug`. `PIXEL_PHYSICS_NEST_HOME=material`
   restores the old door strip.
2. **Food piling replaces the drawn side storeroom.**
   `Storeroom::SHIPPED = on,caste=4,workerhome,pile`. This uses Deneubourg's
   pick-up and drop rule (`pile_pick_p`, `pile_drop_p`, `pile_food_share`).
   `PIXEL_PHYSICS_STOREROOM=on,caste=4,workerhome,side,keep` restores the
   room.

Scottt has an unanswered decision card in the nest thread on piling
(recommended "Piling"). If he picks "Keep storeroom", flip `SHIPPED` back
before merge. Results are in `Reports/nest-one-entrance-2026-09-29.md` §28.

**Off-by-default switches on the branch:**
- `PIXEL_PHYSICS_NEST_LEASH=deep|off`. `deep` pulls nest workers to the
  chamber floor and holds patience; `off` removes the caste tether.
- digbox `cutscsv=PATH`: one row per cut.
- digbox `decisions_nest`: logs the decisions of nest-bound ants that hold
  nothing.

**Withdrawn:** `BrainInput::BroodBearing`, a hungry-larva bearing wired to
`Turn`. On the food box it crashed colonies: alive 283 → 189, fall from peak
11% → 49%. The cause was not traced. The code is at commits `05f43224` and
`c6b81778`; there is a `dead-ends.md` entry. Re-adding an input re-derives
every species' `mutation_rate` (3.18 / live slots).

**Everything is pushed.** Branch `claude/nest-piles-va71j6`, head `886949fc`
(= PR 547), working tree clean. The tether switch is on the head; the larva
cue is in that branch's history at `05f43224` / `c6b81778` (removed again in
`16202e48`). Run and summary scripts from the last sweeps are in
`/mnt/project-files/nest/scripts/` (`rung.sh`/`runh.sh` launch 12-seed arms
with pinned binaries; `summg.py` and `stab.py` summarise digbox output: fruit,
births, fall from peak, nest-worker places). Their scratchpad paths need
editing before reuse. **Runs in flight:** none. The numbers are in §28.

## Next steps, in order

1. **PR 549** (dead-ends index) to green; the merge desk merges, lanes never do. If Scottt answers the storeroom card "Keep storeroom", set `Storeroom::SHIPPED` back to `on,caste=4,workerhome,side,keep` in a new PR.
2. **Lab box:** colonies die under "lay only at the nest" (PR 546).
   - The dug home does not fix it: rich ants stay out on the surface.
   - The suggested fix, "an ant ready to lay walks home", went to the
     breeding lane, which owns it.
   - Lab nests are small because lab colonies are small. Digging scales with
     crowding, and lab digging matches the food box at the same colony size.
3. **One big room, not many chambers.** Under the dug home, cuts rise 2-4x,
   93-96% of them inside home. The ants at each cut stay dense (median 6-10
   within 3 cells). Toffin 2009: density at the dig edge decides round
   versus branched. Local crowding (`CROWDING_LOCAL=near|wide`) still gave
   one room. The plan is roles by age replacing the 1-in-4 caste, as an
   evolvable brain weight.
4. **Nest workers stay out of the nest.** On the food box only 14-24% are in
   the mouth or underground.
   - Cause, traced on individuals: the door pull at the crowded surface spot
     runs out of patience, and fed ants step about 3% of the time, so they
     drift.
   - Scottt rejected "pull to a spot".
     Real nest workers stay because their work and their rest are inside.
     Build work cues the brain reads, not tethers.
   - No-tether plus rest-inside moved workers underground (14% → 22%) but
     colonies swung harder (fall 37% against 11%).
5. **Retire the caste** once roles by age measure as working (the plan
   Scottt agreed at 14:26).

## Scottt's rulings that bind the lane

- **Goal (2026-10-02):** a stable colony that breeds less, builds a food
  supply and survives long term. Judge by fall from peak, colonies lost and
  standing food, not by colony size or fruit.
- **No drawn rooms or castes.** Structure emerges from ant rules, and rules
  should be evolvable (brain weights or genome, not constants).
- **No guessing biology.** Cite the repo reports
  (`Reports/nest-biology-2026-09-19.md`) and real sources.
- **Defaults:** new features default on unless measured harmful.
- **Threads:** never start a new thread without his explicit yes.
- **Lane mechanics:**
  - Run fmt, clippy and tests only for the touched area, then open a draft PR.
  - Hand the PR to the coordinator as soon as the work is done; the merge
    desk merges.
  - Stamp every number with its main commit.

## Pitfalls already paid for

**Counters that mislead:**
- digbox "in the storeroom" and "stored food" counts were mostly surface
  crumbs; food underground is only 2-5 cells.
- TRIPS "inside" includes the mound's overhangs.
- JAM "standing facing an animal" counts frames between decisions.

**How to run and read runs:**
- Anything that changes where ants walk must be run on a food box:
  `digbox hungry gap=90 w=260 soil=60 nulls=0 pile food=400 refill=400 ants=40 seed=N frames=...`.
- Also run the lab box: `labforage frames=120000 seed=N`.
- Copy binaries to a pinned path before a long sweep. A rebuild mid-sweep
  silently switches the arms.
- Trace individuals before naming a cause. My first cause for wandering
  nest workers ("hungry ones forage") was wrong: 80% were fed.

**Harness traps:**
- Wait on processes with `pgrep -x`, never `-f`.
- Seed the dug-home fill from the founding cut as well as the door. Loose
  items must count as open, or a crumb on the mouth cuts home to 6 cells.

## Key files

**Code:**
- `src/sim/creature.rs`: `NestHome`, `adjacent_nest`, `nest_within_reach`,
  `Storeroom`, the pile functions, `home_pull`, `rest_pull`,
  `chooser_step`.
- `src/sim/world.rs`: `step_nest_dug`, `step_nest_room`.
- `examples/digbox.rs`: the food box and its census lines PILES, HOME,
  FUNNEL, LEDGER.
- `examples/labnest.rs`: lab nest census.

**Reports and docs:**
- `Reports/nest-one-entrance-2026-09-29.md`: §20 rest, §26 grip, §27
  piling, §28 home and piling.
- `Reports/nest-biology-2026-09-19.md`.
- `Reports/how-the-ant-works.md`: update it whenever a mechanism changes.
- `wiki/ants.md`.
- Lane note: `Reports/lanes/nest-mouth.md`.

**Project files:**
- `/mnt/project-files/nest/in-the-nest-audit-2026-10-02.md`: every system
  that asks "in the nest", with verdicts.
- `/mnt/project-files/nest/brief-2026-10-02.md`.

## Tools and methods (what differs from the repo's standard rules)

### Test beds, exact commands

Build first, then copy the binary to a pinned path. A rebuild mid-sweep
silently switches every later arm onto the new code.

```
cargo build --release --example digbox --example labforage
cp target/release/examples/{digbox,labforage} $PIN/
```

The machine has 4 cores; run 4 at once. Gate on
`pgrep -xc digbox` + `pgrep -xc labforage`, never `pgrep -f`.

| bed | command | time per run | what it answers |
|---|---|---|---|
| **food box** (the main bed) | `digbox hungry gap=90 w=260 soil=60 nulls=0 pile food=400 refill=400 ants=40 seed=N frames=144000` | ~6 min | Foraging, eating, breeding and digging together. 12 seeds is the standard sample |
| **long runs** | the food box at `frames=240000` | ~10 min | Stability: fall from peak, colonies lost, end size. The owner's goal is judged here |
| **lab box** (the second game) | `labforage frames=120000 seed=N` | ~6-8 min | The real lab. Read the `SUMMARY` line: `born`, `alive`, `intake`, `nest_visits`, `buds_held_for_nest`; also the `BROOD` line. **Lab numbers from before 0738a8ca are not comparable** (laying only at the nest changed the box) |
| **dig box, fed** | `digbox ants=40 frames=24000 seed=N` (or `ants=200`) | ~2 min | Digging alone: no food, every ant topped up |
| **lab nest census** | `labnest frames=24000 seeds=3 founders=8` | ~5 min | Lab nest size over time (`roofed`, `digs`, ants) |

- `BUD_SITE=nest` (lay only at the nest) is the default on main since PR 546.
  The food box used to set it by environment; that is no longer needed.
- A change to where ants walk must also be run on the food box (owner
  rule). Check the lab box before telling Scottt a fix works.
- 12 seeds per arm, compared within seed. Quote medians and "higher on N of
  12". Stamp every number with its main commit.

### digbox instruments: what each line really counts

- **Population table** (`frame ants digs roofed open ...`): one row every
  12k frames.
  - `ants`: live adults.
  - `digs`: engine digs, cumulative.
  - `roofed`: empty cells with ground above them.
- **`FOOD frame=F cells placed N`**: fruit cells placed by refill,
  cumulative. This is the "fruit taken" number.
- **`hatched (births) N`**: adults born.
- **`PILE frame=F`**: where ants and nest workers stand, in four places: on
  the mound over the mouth, in the mouth, underground, out on the surface.
  Also food clumps (8-connected groups; groups of 3+ within 30 columns of the
  door), and `pile_left`.
- **`HOME frame=F`**: the size of the dug home.
- **LARDER line** (`in the storeroom A, elsewhere below the old ground line
  B, above it outside the pile C`):
  - **A** is the drawn room only; it goes with the room.
  - **C** is mostly crumbs strewn on the surface. **Do not call A+B+C
    "stored food"**: real underground food is B, 1-5 cells.
- **TRIPS "inside"** includes the mound's own overhangs, so most "inside"
  drops are on the mound.
- **JAM "standing facing an animal"** counts every frame between decisions,
  and an ant decides about every 7 frames. It is not evidence of traffic.
- **FUNNEL / LEDGER / REFILL / FILL / WIDTH / CRATER**: the dig ledger
  (where cuts land, what refilled dug cells).

**Tracing options.** Trace individuals before naming any cause; these give a
row per individual.
- `tripcsv=PATH`: one row per carrier per frame.
- `decisions=PATH`: one row per decision, with inputs, `p_move`, patience,
  outcome and anchor. Add `decisions_nest` to log nest-bound ants that hold
  nothing instead of carriers.
- `cutscsv=PATH`: one row per cut, with open neighbours, whether it is in
  home, animals within 3, and nest worker or not.
- `antscsv=` and `stops=` exist too.
- `out=x.png` renders the box. Post pictures; don't describe them.

**Visual review:** `python3 scripts/review.py` (see
`.claude/skills/review/SKILL.md`). Put the event count in `meta`.

### Switches (environment, `PIXEL_PHYSICS_` prefix) and their defaults on PR 547's head

| switch | default | notes |
|---|---|---|
| `NEST_HOME` | `dug` | `material`/`off` = painted door strip; `shaft`, `mouth` older forms |
| `STOREROOM` | `on,caste=4,workerhome,pile` | Parts: `on` (carry), `caste=4` (1 in 4 ants is a nest worker, by id, for life), `workerhome` (the founding cut is home to nest workers), `pile`, `side` (the drawn side room), `keep` (a fed ant won't eat stored food), `nestbound=<frames>[/k]` (age roles), `harvest`. `off` = no storeroom |
| `DIG_ROOF` | 6 (PR 539) | Rows of roof kept over rooms |
| `KIN_FOOTING` | on (PR 541) | `off` disables gripping a nestmate |
| `NEST_LEASH` | unset (tether to the door spot) | `deep` = pull to the chamber floor, never give up; `off` = no tether |
| `NEST_REST` | off | `workers`, `on`, `all`: idle ants rest inside. Measured in §20 |
| `CROWDING_LOCAL` | unset (colony-wide) | `near`, `wide`: local crowding for the dig gate. Did not change one big room |
| `BUD_SITE` | nest (PR 546) | Breeding lane's |
| `BREEDING_MAX` | 1.25 (PR 545) | Graded fertility; breeding lane's |

### Habits Scottt corrected (a fresh lane would repeat these)

- **Answering biology from memory.** Check
  `Reports/nest-biology-2026-09-19.md`, `Reports/ant-breeding-plan-2026-09-29.md`
  (biology table near line 465) and
  `Reports/ant-sim-literature-review-external-2026-09-19.md`, then search the
  web, and cite. He rejected "as far as I know".
- **Drawn structures or fixed pulls.** "Why is there behavior just getting
  pulled to a spot?" Structure must come from ant rules, and rules should be
  brain or genome weights so species can evolve them.
- **Mislabelling a counter.** "Stored food" was crumbs, and "at the nest"
  was the door strip. Name what a counter counts before quoting it.
- **Naming a cause from an aggregate.** The first cause given for wandering
  nest workers was wrong; the decision trace showed 80% were fed.
- **Running the full test suite before a PR.** Run fmt, clippy and
  touched-area tests, open a draft, keep working. CI runs on drafts. Hand
  the PR to the coordinator immediately, not after green. **Lanes never
  merge**; the merge desk does.
- **Asking before flipping a measured-neutral-or-better default.** Don't;
  "You can turn anything on by default that you want" (2026-10-01). If the
  permission check blocks it, quote that and redo the edit.
- **Starting a new thread without his explicit yes.** Never. "Up to you"
  counts as yes; a question does not.
- **Talking to Scottt.** Lead with what the change does in world terms, then
  where it sits in the arc, then the mechanism. Keep it short; he reads
  cold.

### Team memory this lane relies on (substance copied)

- **Goal:** small, stable colonies. Judge by long-run survival and food
  stock, not size or fruit.
- **Lane split:** the nest lane does not build breeding rules (laying,
  brood, fertility, `kin_deficit`, `nurse`).
- **Defaults:** features default on unless a measured harm, which is stated
  in the PR.
- **Merge desk:** CI takes ~18 min.
  - The desk merges one PR at a time, with a merge commit (not squash), and
    merges main into a branch, never rebases.
  - If a lane pushes after hand-off, it must warn the desk.
  - A lane saying "handed to the desk" doesn't mean the desk got it; check.
- **Kin footing history:** without the 60-tick grip limit, resting ants
  formed a mat over the door and one colony fell to 3.

### Scripts (also in /mnt/project-files/nest/scripts/; copied here in case that folder doesn't carry over)

`run_arms.sh`: N seeds by M arms, 4 at a time, from a pinned binary. Edit
`S`, `env_for` and the command line.

```bash
S=/path/to/scratch; cd /path/to/Pixel_Physics
busy() { echo $(( $(pgrep -xc digbox) + $(pgrep -xc labforage) )); }
env_for() { case $1 in
  base) echo "";;
  dug) echo "PIXEL_PHYSICS_NEST_HOME=dug";;
  material) echo "PIXEL_PHYSICS_NEST_HOME=material";;
esac; }
mkdir -p $S/out
for s in 1 2 3 4 5 6 7 8 9 10 11 12; do for arm in material dug; do
  while [ $(busy) -ge 4 ]; do sleep 5; done
  ( env $(env_for $arm) $S/bin/digbox hungry gap=90 w=260 soil=60 nulls=0 pile food=400 refill=400 ants=40 seed=$s frames=144000 > $S/out/box-$arm-s$s.txt 2>&1 ) &
  sleep 1
done; done; wait
```

`summg.py DIR arm1 arm2 ...` reads `DIR/box-ARM-sN.txt` at 144k. It prints
fruit, births, alive, dead (<10), fall from peak, nest-worker places
(summed over 48k, 96k and 144k) and underground food.

```python
import re,statistics as st,sys,os
d=sys.argv[1]; arms=sys.argv[2:]
pat=r'on the mound over the mouth (\d+) \(nest workers (\d+).*?in the mouth (\d+) \(nest workers (\d+).*?underground (\d+) \(nest workers (\d+).*?out on the surface (\d+) \(nest workers (\d+)'
for a in arms:
    P=[];B=[];A=[];F=[];W=[0,0,0,0];U=[];n=0
    for s in range(1,13):
        f=f'{d}/box-{a}-s{s}.txt'
        if not os.path.exists(f): continue
        t=open(f).read()
        if '144000' not in t or not re.search(r'^PILE frame=144000',t,re.M): continue
        n+=1
        P.append(int(re.findall(r'FOOD frame=144000 cells placed (\d+)',t)[0]))
        B.append(int(re.findall(r'hatched \(births\) (\d+)',t)[-1]))
        rows=[(int(fr),int(x)) for fr,x in re.findall(r'^\s+(\d+000)\s+(\d+)\s',t,re.M)]
        ants=[x for fr,x in rows if fr>=24000]; A.append(ants[-1])
        pk=max(ants); tr=min(ants[ants.index(pk):]); F.append((pk-tr)/pk if pk else 0)
        for fr in ['48000','96000','144000']:
            m=re.search(r'^PILE frame='+fr+r' .*',t,re.M)
            g=[int(x) for x in re.search(pat,m.group(0)).groups()]
            for i in range(4): W[i]+=g[2*i+1]
        food=re.findall(r'elsewhere below the old ground line (\d+)',t); U.append(int(food[-1]) if food else -1)
    if not n: continue
    tw=sum(W) or 1
    print(f'{a:5} n={n} fruit {st.median(P):.0f} births {st.median(B):.0f} alive {st.median(A):.0f} dead(<10) {sum(x<10 for x in A)} fall {st.median(F):.2f} | nest workers mound {100*W[0]//tw}% mouth {100*W[1]//tw}% under {100*W[2]//tw}% surface {100*W[3]//tw}% | food underground {st.median(U)}')
```

`stab.py DIR arm1 ...` is for 240k long runs named `DIR/ARM-sN.txt`. It
prints peak, end, fall from peak, dead (<10) and standing food.

```python
import re,statistics as st,sys
def one(path):
    t=open(path).read()
    rows=[(int(f),int(a)) for f,a in re.findall(r'^\s+(\d+000)\s+(\d+)\s',t,re.M)]
    ants=[a for f,a in rows if f>=24000]
    peak=max(ants); end=ants[-1]
    trough=min(ants[ants.index(peak):])
    food=[int(b)+int(c)+int(a0) for a0,b,c in re.findall(r'in the storeroom (\d+), elsewhere below the old ground line (\d+), above it outside the pile (\d+)',t)][1:]
    return peak,end,trough,st.mean(food) if food else 0
for d,arms in [(sys.argv[1],sys.argv[2:])]:
    for a in arms:
        R=[one(f'{d}/{a}-s{s}.txt') for s in range(1,13)]
        drop=[ (p-tr)/p for p,e,tr,f in R]
        print(a,'peak',st.median(r[0] for r in R),'end',st.median(r[1] for r in R),'fall from peak',round(st.median(drop),2),'dead/near(<10)',sum(r[1]<10 for r in R),'standing food mean',round(st.median(r[3] for r in R),1))
```

Lab-box summary, inline:

```python
import re,statistics as st
for a in ['material','dug']:
    rows=[]
    for s in range(1,13):
        m=re.search(r'^SUMMARY .*',open(f'hlab/{a}-s{s}.txt').read(),re.M).group(0)
        g=lambda k:int(re.search(rf' {k}=(\d+)',m).group(1))
        rows.append((g('born'),g('alive'),g('intake')))
    print(a,'born',st.median(r[0] for r in rows),'alive',st.median(r[1] for r in rows),'dead boxes',sum(r[1]==0 for r in rows))
```
