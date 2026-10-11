"""pop.py RUN... : per 10k frames -- ants, brood, and what happened in the window (starved, old age,
eggs, births, larvae starved, eats, deliveries, trips). stats.csv counters are cumulative; deltas here."""
import sys, csv
COLS = ["died_starved", "died_old_age", "died_killed", "eggs_laid", "births", "larvae_starved", "eats",
        "deliveries", "trip_deliveries", "forage_trips", "digs"]
for run in sys.argv[1:]:
    rows = {int(r["frame"]): r for r in csv.DictReader(open(f"{run}/stats.csv"))}
    print(f"== {run}")
    print(f"{'frame':>7} {'ants':>5} {'brood':>5} " + " ".join(f"{c[:9]:>9}" for c in COLS))
    prev = None
    for f in range(10000, 300001, 10000):
        r = rows.get(f)
        if r is None:
            continue
        if prev is not None:
            d = [float(r[c]) - float(prev[c]) for c in COLS]
            print(f"{f:>7} {r['ants']:>5} {r['brood']:>5} " + " ".join(f"{x:>9.0f}" for x in d))
        prev = r
