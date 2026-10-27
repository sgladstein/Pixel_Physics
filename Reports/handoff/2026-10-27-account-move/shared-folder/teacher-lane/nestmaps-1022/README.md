# End-of-round nest maps, 2026-10-22 (Teacher lane)

- `cs3lex/`, `cs3lexles/`, `cs3lexlesinh/` — colsel3-1022 (colony selection on cuts that stay nest), round 15, one folder per colony (64 each).
  `cs3lexlesinh` inherits its nest round to round, so its r15 map is 16 rounds of digging; the other two start each round in a fresh world (one 200k round).
- `film-5099-best/`, `film-5099-old/` — one 200k common-garden run each in world 5099: a typical cs3lexlesinh colony (r15 pool, selection off) and the old ant. These two have `events.csv` (frame,id,kind,x,y,v,cutter; kind dig = cut of original ground).
- `nestmap.txt`: header `x0 y0 legend` (legend = comma-separated material names; cell char = chr(65 + index); `?` = organism/food/creature), then one row per y, one char per x. Window 100 wide, 40 tall around the door (NEST = x 128).
- `surface.txt`: the datum (original ground row) per column, 256 lines.
- `nest.csv`: the nest score's own line per snapshot (dug, chambers, niches, tunnel_cells, pit, ...).
- `nestcount.py`: the Teacher lane's counter (nest / crater / spoil / kept cuts) — `count(dir)`.
