# Hand-written digger (nest lane, 2026-10-29)

Owner's question (2026-10-29): could hand-written brain weights make the colony dig a better nest? `make.py` writes an
8-unit dig head by hand into every std_pool founder and leaves the rest of the ant alone. No training is involved.

    python3 scratch/nest/handdig/make.py scratch/inrun/genomes/std_pool_raw OUT thr=0.2 out=3 door=8   # arm H3
    BIN=target/release/examples/inrun bash scratch/nest/poolrounds.sh RUN SEED 4 200000 OUT nest_r=0
    # deeper dirt: add SETX=PP_BED_SOIL=60 (src/train/challenges.rs inrun::bed_soil, default 40)

What the units do: cut along the room's floor, cut down rather than up, push away from the door, turn down when far
out, and two "when" units that make an ant less likely to take the dig job in a wide room or right by the door.

## Results, 8 worlds (61-68), against the trained ant on the same build

| | Trained ant | H3 |
|---|---|---|
| Passage share, round 1 | 3-15% | 16-55% (8 of 8 higher, p 0.008) |
| Passage share, round 4, shallow dirt | 6-28% | 31-58% (7 of 7 living pairs) |
| Colony lines dead by round 4, shallow (40 rows) | 0 of 8 | 1 of 8 |
| Colony lines dead by round 4, deep (60 rows) | 1 of 8 | 4 of 8 |

The pictures agree with the passage share this time: branching tunnels and small rooms, not the crumb-packed shaft
that fooled the score in `../roomcopy/`. With deeper dirt the trained ant still digs one cone. The deep-dirt crashes
come late in a round, when foragers stop bringing food home and births stop. The dead ants are found against the bed's
end walls on the surface, not underground. Worlds 69-76 are being run to settle the survival cost.
