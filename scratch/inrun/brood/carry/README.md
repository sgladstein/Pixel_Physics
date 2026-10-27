# Late colsel20 carries (Teacher lane, 2026-10-23)

Arm cs20light (hcs + dh7 evolvable dig head, PP_DIGLIGHT=1, body rules weaned off by round 32), round 47.

Per colony:
- `pool/` = the round-47 final pool: index.csv (id, family, score) + one `ID.raw.txt` genome each.
  Founders list as the garden uses it: top 30 by score, 30 slots shared round-robin
  (scratch: /mnt/project-files/teacher-lane/tools/cg15.sh builds `founders=FILE@family.id:N,...`).
- `nestmap.txt`, `surface.txt`, `stock.csv` = a rules-off fresh-world garden (seed c*1000+99, dist=25, 200k frames)
  ending with the food-packed bowl (gardens-colsel20.png).
- `selection-round-nestmap.txt` = the colony's own round-47 selection world map.

Env for these genomes: PP_HOMEVEC=40:30,PP_DIGHEAD=scratch/inrun/genomes/dh7.head,PP_DIGHEAD_GATE=1,PP_DIGHEAD_EVOLVE=1,PP_DIGLIGHT=1
(no PP_BODYRULES = rules off). Args: colsel20.txt line 1 minus `colsel 30 MODE 64 48` and `wean=`.
