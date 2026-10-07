# WAY_FOOT 12-seed bar on top of the backfill (Nest race, 2026-10-07)

**Build:** `claude/nest-race-way-foot` 74a2f08a (binary dt-bf). **Env:** NEEDS_FIRST=on,backfill CARRY_HOME=on
DOOR_COLUMN=on LAY_BAR=body NEST_STORE=on,pick=20,jaws,sky,meal,smell=10, WAY_FOOT=off|on, RAYON_NUM_THREADS=1.
deeptrace `scenario=nest_goal founder=evolved ants=0 hungry=1 dig=1 mapevery=25000 shots=1`, 300k. hungry.csv reduced
on the fly to frame,id,hy,zone,outcome (`hred.csv.gz`). Seeds 1-4 reproduce the earlier bf90 runs exactly (same
ants/starved/bites), so shots and mapevery do not change the run.

## Heap 90 (seeds 1-12), measured

`one12.py`: ants at 300k; starved 20-300k; deep hungry falls (% of deep hungry decisions); deep starvers (hungry spell
began 10+ rows down, 60-300k, died starved within 200 frames of it); store bites; digs.

```
t90off-s1	ants 701	starved 646	fall 15.1%	deep-starvers 362	bites 28409	digs 40535
t90on-s1	ants 796	starved 24	fall 10.4%	deep-starvers 10	bites 52211	digs 73765
t90off-s2	ants 709	starved 729	fall 16.5%	deep-starvers 446	bites 27054	digs 33080
t90on-s2	ants 771	starved 9	fall 12.4%	deep-starvers 3	bites 51618	digs 65659
t90off-s3	ants 647	starved 1040	fall 10.4%	deep-starvers 624	bites 18450	digs 29969
t90on-s3	ants 748	starved 148	fall 10.8%	deep-starvers 77	bites 49902	digs 65230
t90off-s4	ants 722	starved 434	fall 18.8%	deep-starvers 295	bites 23947	digs 38745
t90on-s4	ants 736	starved 269	fall 11.8%	deep-starvers 179	bites 42988	digs 58019
t90off-s5	ants 675	starved 671	fall 15.4%	deep-starvers 404	bites 24762	digs 39954
t90on-s5	ants 832	starved 364	fall 9.8%	deep-starvers 234	bites 43959	digs 64096
t90off-s6	ants 694	starved 941	fall 12.6%	deep-starvers 574	bites 27750	digs 42424
t90on-s6	ants 668	starved 232	fall 11.0%	deep-starvers 111	bites 56433	digs 70722
t90off-s7	ants 737	starved 497	fall 16.3%	deep-starvers 291	bites 21559	digs 32839
t90on-s7	ants 872	starved 20	fall 11.6%	deep-starvers 14	bites 55089	digs 80756
t90off-s8	ants 649	starved 1086	fall 13.4%	deep-starvers 666	bites 21408	digs 30197
t90on-s8	ants 614	starved 276	fall 11.1%	deep-starvers 162	bites 37855	digs 46826
t90off-s9	ants 604	starved 714	fall 17.0%	deep-starvers 469	bites 27844	digs 38798
t90on-s9	ants 791	starved 4	fall 11.1%	deep-starvers 2	bites 55955	digs 81869
t90off-s10	ants 745	starved 279	fall 19.6%	deep-starvers 188	bites 25058	digs 37173
t90on-s10	ants 596	starved 50	fall 11.4%	deep-starvers 9	bites 41530	digs 64625
t90off-s11	ants 687	starved 573	fall 17.6%	deep-starvers 401	bites 26868	digs 35952
t90on-s11	ants 765	starved 297	fall 10.2%	deep-starvers 155	bites 39584	digs 56508
t90off-s12	ants 695	starved 663	fall 18.3%	deep-starvers 445	bites 16525	digs 30045
t90on-s12	ants 741	starved 57	fall 11.7%	deep-starvers 1	bites 51138	digs 69926
```

- **Starved:** lower on 12 of 12 seeds; 8,273 off against 1,750 on.
- **Deep starvers:** lower on 12 of 12; 5,165 against 957.
- **Falls:** lower on 11 of 12 (s3 level, 10.4 against 10.8%).
- **Colony at 300k:** bigger on 9 of 12. Smaller on s6 (668 against 694), s8 (614 against 649) and s10 (596 against 745).
  None of these is a die-off.
- **Die-offs:** none in either arm. The lowest count after 50k was 370 off and 444 on.
- **Second entrances:** none in any of the 24 runs at any 5k mark, 5k-300k (`second_way.py` every 5k, Deep trace's
  check). Positive control: the same check finds the openings in batch-1 runs wf90on-s3 (from 130k) and b90smell-s3
  (from 140k).
- **Pictures:** `pictures/heap90-sN.png`, nest every 50k, off above on.
- **Still untraced:** the residual nest starvers under `on` (s4 269, s5 364, s8 276, s11 297). Deep trace traced the
  s3/s4 ones in the 4-seed rerun to inedible store crumbs (NEST_STORE, not WAY_FOOT); the `edible` fix (b5922852) is
  being tested.
