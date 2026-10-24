import matplotlib; matplotlib.use("Agg")
import matplotlib.pyplot as plt
ink, ink2, grid, s1, s2 = "#0b0b0b", "#52514e", "#e4e3df", "#2a78d6", "#eb6834"
plt.rcParams.update({"font.size": 11, "axes.edgecolor": grid, "axes.labelcolor": ink2, "xtick.color": ink2, "ytick.color": ink2})
fig, (a, b) = plt.subplots(1, 2, figsize=(12, 4.6), facecolor="#fcfcfb", gridspec_kw=dict(width_ratios=[1.25, 1]))
for ax in (a, b):
    ax.set_facecolor("#fcfcfb"); ax.spines[["top", "right"]].set_visible(False); ax.grid(axis="y", color=grid, lw=0.8); ax.set_axisbelow(True)
rows = list(range(8)); sc = [24, 10, -4, 23, 22, 125, 208, 238]
a.plot(rows, sc, color=s1, lw=2, marker="o", ms=8, mec="#fcfcfb", mew=2)
a.axhline(0, color=ink2, lw=0.8)
a.axvspan(0.5, 4.5, color=s2, alpha=0.08, lw=0)
a.text(2.5, 175, "rows 1-4: no reward\n(rows 1-2 charged as cracks)", ha="center", color=ink2, fontsize=10)
a.text(5.15, 112, "first pay: a 7x5\nbox fits inside", color=ink2, fontsize=10, va="top")
a.set_xlabel("rows of a 28-wide side chamber dug so far (floor first, rising)")
a.set_ylabel("nest score (well-built cells)")
a.set_title("Digging a chamber the way an ant would earns nothing until row 5", loc="left", color=ink, fontsize=12)
names = ["harvester stack\n(owner's pick)", "corridor maze,\nno rooms"]
asis, low = [760, 23], [837, 1238]
y = [1, 0]; h = 0.36
b.barh([v + h/2 for v in y], asis, height=h - 0.03, color=s1, label="score as built (7x5 core)")
b.barh([v - h/2 for v in y], low, height=h - 0.03, color=s2, label="if 3-tall rooms count (7x3 core)")
for v, x1, x2 in zip(y, asis, low):
    b.text(x1 + 20, v + h/2, f"{x1}", va="center", color=ink, fontsize=10)
    b.text(x2 + 20, v - h/2, f"{x2}", va="center", color=ink, fontsize=10)
b.set_yticks(y, names); b.set_xlim(0, 1500); b.grid(axis="y", visible=False); b.grid(axis="x", color=grid, lw=0.8)
b.set_xlabel("nest score")
b.set_title("Paying low rooms makes a maze the best nest", loc="left", color=ink, fontsize=12)
b.legend(frameon=False, loc="upper center", bbox_to_anchor=(0.5, -0.18), ncol=2, fontsize=9.5)
fig.tight_layout(); fig.savefig("/mnt/project-files/nest-score/nest-score-gradient.png", dpi=130)
