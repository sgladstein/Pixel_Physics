"""Part 1 picture: where s161 gen 336's digging time and pellets go (24 sealed worlds, 4-row shaft)."""
import sys, collections, statistics as st
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

LOG, OUT = sys.argv[1], sys.argv[2]
src = open(LOG.rsplit("/trace/", 1)[0] + "/pellets.py").read().split("allrec = []")[0].replace('print(f"worlds parsed', "#")
sys.argv = [sys.argv[0], LOG]
exec(src)

INK, INK2, MUTED, GRID, SURF = "#1f1f1e", "#55544f", "#8a8984", "#e6e5e1", "#fcfcfb"
GOOD, WASTE, LOST, OTHER = "#2a78d6", "#eb6834", "#55544f", "#c3c2b7"

recs = [x for w in worlds for x in fates(w)]
fig, ax = plt.subplots(2, 2, figsize=(13, 9))
fig.patch.set_facecolor(SURF)
for a in ax.flat:
    a.set_facecolor(SURF); a.grid(True, axis="y", color=GRID, lw=0.8); a.set_axisbelow(True)
    for sp in ("top", "right"): a.spines[sp].set_visible(False)
    for sp in ("left", "bottom"): a.spines[sp].set_color(MUTED)
    a.tick_params(colors=INK2, labelsize=9)

# A: trips by what was cut and what became of it
a = ax[0, 0]
cls = ["foot", "wall", "rim", "recut"]
lab = {"foot": "shaft's foot\n(4+ rows: pays)", "wall": "shaft wall\n(1-3 rows: no pay)", "rim": "top row\n(no pay)", "recut": "re-cut a pellet\n(no pay)"}
full = [sum(1 for x in recs if x["cls"] == c and x["fate"] == "FULL LOAD") / 24 for c in cls]
outn = [sum(1 for x in recs if x["cls"] == c and x["fate"] == "away, short lift") / 24 for c in cls]
lost = [sum(1 for x in recs if x["cls"] == c and x["fate"] not in ("FULL LOAD", "away, short lift")) / 24 for c in cls]
xs = range(len(cls))
a.bar(xs, full, color=GOOD, width=0.6, label="full load (counts)")
a.bar(xs, outn, bottom=full, color=WASTE, width=0.6, label="carried out, pays nothing")
a.bar(xs, lost, bottom=[f + o for f, o in zip(full, outn)], color=LOST, width=0.6, label="put back near the hole, or still held")
for i in xs:
    a.text(i, full[i] + outn[i] + lost[i] + 0.12, f"{full[i] + outn[i] + lost[i]:.1f}", ha="center", fontsize=9, color=INK)
a.set_xticks(list(xs)); a.set_xticklabels([lab[c] for c in cls], fontsize=8.5)
a.set_ylabel("trips per world", color=INK)
a.set_title("A. Each cut starts a trip. 8.0 a world start at the foot (7.2 become full loads);\n"
            "4.8 more are cut where nothing pays and are carried out all the same.", loc="left", fontsize=10, color=INK)
a.legend(frameon=False, fontsize=8.5, loc="upper right")

# B: where the time goes
a = ax[0, 1]
bud = collections.Counter()
for w in worlds:
    for m in w["M"]:
        if m["f"] > FRAMES: continue
        c = m["ctx"]
        bud[("carrying" if c["spoil"] > 0.5 else "empty") + (", below ground" if c["depth"] > 0.01 else ", on the surface")] += 1
order = ["empty, below ground", "carrying, below ground", "carrying, on the surface", "empty, on the surface"]
tot = sum(bud.values())
vals = [bud[k] / tot for k in order]
cols = [GOOD, GOOD, OTHER, WASTE]
a.barh(range(4), vals, color=cols, height=0.6)
for i, v in enumerate(vals):
    a.text(v + 0.01, i, f"{v:.0%}", va="center", fontsize=9, color=INK)
a.set_yticks(range(4)); a.set_yticklabels(order, fontsize=9)
a.set_xlim(0, 0.62); a.grid(True, axis="x", color=GRID); a.grid(False, axis="y")
a.set_xlabel("share of all decisions in the 6,000 frames", color=INK)
a.set_title("B. Where the time goes: only 28% below ground. Half of all decisions\nare an empty ant walking on the surface.", loc="left", fontsize=10, color=INK)

# C: how far loads are carried
a = ax[1, 0]
dist = [abs(x["put"]["x"] - x["cut"]["x"]) for x in recs if x["fate"] == "FULL LOAD"]
c = collections.Counter(dist)
a.bar(sorted(c), [c[k] / 24 for k in sorted(c)], color=GOOD, width=0.8)
a.axvline(4.5, color=INK, lw=1.2, ls="--")
a.text(4.7, max(c.values()) / 24 * 0.95, "about as far as the rule needs\n(3 cells clear of the hole)", fontsize=8.5, color=INK, va="top")
a.axvline(st.median(dist), color=WASTE, lw=1.5)
a.text(st.median(dist) + 0.2, max(c.values()) / 24 * 0.6, f"median {st.median(dist):.0f}", fontsize=8.5, color=INK)
a.set_xlabel("columns between the cut and where the load is put down", color=INK)
a.set_ylabel("full loads per world", color=INK)
a.set_title("C. Loads are carried about 3x farther than the rule needs.", loc="left", fontsize=10, color=INK)

# D: how far the empty ant wanders
a = ax[1, 1]
emp = collections.Counter(); car = collections.Counter()
for w in worlds:
    cx = st.median([cc["x"] for cc in w["C"] if cc["d"] >= SHAFT] or [128])
    for m in w["M"]:
        if m["f"] > FRAMES or m["ctx"]["depth"] > 0.01: continue
        d = min(int(abs(m["x"] - cx)) // 5 * 5, 30)
        (car if m["ctx"]["spoil"] > 0.5 else emp)[d] += 1
bins = list(range(0, 35, 5))
te, tc = sum(emp.values()), sum(car.values())
a.bar([b - 1.1 for b in bins], [car[b] / tc for b in bins], width=2.1, color=OTHER, label="carrying")
a.bar([b + 1.1 for b in bins], [emp[b] / te for b in bins], width=2.1, color=WASTE, label="empty")
a.set_xticks(bins); a.set_xticklabels([f"{b}-{b + 4}" if b < 30 else "30+" for b in bins], fontsize=8.5)
a.set_xlabel("columns from the shaft (surface time only)", color=INK)
a.set_ylabel("share of that state's surface time", color=INK)
a.legend(frameon=False, fontsize=8.5)
a.set_title("D. After putting a load down the empty ant keeps going: 45% of its\nsurface time is 10+ columns out, 21% is 15+.", loc="left", fontsize=10, color=INK)

fig.suptitle("Part 1: where s161 gen 336's time and soil go (24 sealed worlds, 4-row shaft, 6,000 frames; every pellet traced)",
             x=0.01, ha="left", fontsize=12.5, color=INK, fontweight="bold")
fig.tight_layout(rect=(0, 0, 1, 0.96))
fig.savefig(OUT, dpi=110, facecolor=SURF)
print("wrote", OUT)
