# NEST_STORE `edible`, first test (Nest race, 2026-10-07 ~22:40 UTC), for Deep trace's check

Build `claude/nest-race-way-foot` b5922852 (dt-ed). Base = the 12-seed bar's on arm (t90on, dt-bf 74a2f08a; dt-ed with
`edible` unset is byte-identical to dt-bf, seed 1 to 40k). Arm = base env + `NEST_STORE=...,smell=10,edible`. Heap 90,
seeds 1-4, 300k, mapevery=25000 shots=1, hungry reduced to hred. No foodevery (no store census in these runs).

## Measured, base -> edible
    t90on-s1	ants 796	starved 24	fall 10.4%	deep-starvers 10	bites 52211	digs 73765
    ed90ed-s1	ants 729	starved 7	fall 13.0%	deep-starvers 0	bites 50345	digs 69072
    t90on-s2	ants 771	starved 9	fall 12.4%	deep-starvers 3	bites 51618	digs 65659
    ed90ed-s2	ants 782	starved 47	fall 13.4%	deep-starvers 2	bites 56259	digs 83250
    t90on-s3	ants 748	starved 148	fall 10.8%	deep-starvers 77	bites 49902	digs 65230
    ed90ed-s3	ants 778	starved 2	fall 11.7%	deep-starvers 1	bites 59522	digs 87774
    t90on-s4	ants 736	starved 269	fall 11.8%	deep-starvers 179	bites 42988	digs 58019
    ed90ed-s4	ants 727	starved 15	fall 13.1%	deep-starvers 1	bites 61545	digs 90621

- **The traced problem:** deep starvers 10/3/77/179 -> 0/2/1/1. Nest-zone starved deaths 20-300k are 1-3 per seed under
  `edible` (ledger zone_end).
- **Starved 20-300k:** 24/9/148/269 -> 7/47/2/15. Seed 2 rises (9 -> 47): 44 of them died in mound tunnels at 200-300k.
  **Untraced.** Possibly the release Deep trace warned of (hungry ants let out once the store reads empty), but that is
  not checked.
- **Store food at 300k:** 167/112/118/35 -> 264/127/201/175 cells. Store bites 43-52k -> 50-62k. Digs up 6-56%.
- **Colony at 300k:** 796/771/748/736 -> 729/782/778/727 (2 up, 2 down). Lowest after 50k: 447-565, so no die-off.
- **Falls** are slightly higher, 10.4-12.4% -> 11.7-13.4%.
- Pictures: `pictures/edible-vs-base-heap90-sN.png`.

**Verdict (Nest race, for Deep trace's check):** it fixes the traced problem. Late nest starving on s3/s4 is gone, and
deep starvers are 0-2 on all 4 seeds. The open item is seed 2's 44 mound-tunnel deaths at 200-300k, which must be traced
before any default-on. Not default-on yet: 4 seeds at heap 90 only, and it is on the NEEDS_FIRST stack.
