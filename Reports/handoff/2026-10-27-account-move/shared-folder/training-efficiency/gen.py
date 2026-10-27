import json, collections, html
from ledger import L
E = lambda s: html.escape(str(s))
needn = json.load(open("needn.json"))["bybin"]
noise = json.load(open("noise.json"))
alone = json.load(open("alone.json"))
early = {(e["batch"], e["idea"], e["metric"]): e for e in json.load(open("early.json"))}

# ---------- chart 1: CPU split stacked bar ----------
parts = [("Idea and exploration arms", 628, "s1"), ("Common gardens (every 2nd round)", 333, "s2"), ("Fresh control arms", 92, "s3"), ("Exact reruns of an earlier arm", 61, "s4")]
T = sum(p[1] for p in parts)
W = 720; x = 0; segs = []; labels = []
for name, v, cls in parts:
    w = W * v / T
    segs.append(f'<rect class="{cls} mark" x="{x+1:.1f}" y="0" width="{max(w-2,1):.1f}" height="34" rx="4" data-tip="{E(name)}: {v} CPU-hours ({v/T:.0%})"/>')
    x += w
legend1 = "".join(f'<li><span class="sw {c}"></span>{E(n)} <b>{v} h · {v/T:.0%}</b></li>' for n, v, c in parts)
chart1 = f'<svg viewBox="0 0 {W} 34" class="bar" role="img" aria-label="CPU split">{"".join(segs)}</svg><ul class="legend">{legend1}</ul>'

# ---------- chart 2: calls kept vs colonies ----------
bins = [("large (>1 sd)", "Large effect (more than 1 spread)", "s1"), ("medium (0.5-1 sd)", "Medium (0.5 to 1 spread)", "s2"), ("small (<0.5 sd)", "Small (under 0.5 spread)", "s3")]
Ns = [4, 6, 8, 10, 12, 16, 20, 24, 32, 40, 48, 56]
cw, ch, ml, mr, mt, mb = 640, 270, 44, 150, 12, 46
X = lambda n: ml + (n - 4) / (64 - 4) * (cw - ml - mr)
Y = lambda v: mt + (1 - v) * (ch - mt - mb)
g = []
for t in (0, .25, .5, .75, 1):
    g.append(f'<line class="grid" x1="{ml}" x2="{cw-mr}" y1="{Y(t):.1f}" y2="{Y(t):.1f}"/><text class="tick" x="{ml-8}" y="{Y(t)+4:.1f}" text-anchor="end">{int(t*100)}%</text>')
for n in (8, 16, 24, 32, 48, 64):
    g.append(f'<text class="tick" x="{X(n):.1f}" y="{Y(0)+18:.0f}" text-anchor="middle">{n}</text>')
g.append(f'<line class="axis" x1="{ml}" x2="{cw-mr}" y1="{Y(0):.1f}" y2="{Y(0):.1f}"/>')
labs = []
g.append(f'<line class="ref" x1="{X(32):.1f}" x2="{X(32):.1f}" y1="{mt}" y2="{Y(0):.1f}"/>')
for key, name, cls in bins:
    d = needn[key]
    pts = [(n, d[str(n)][0] / d[str(n)][2]) for n in Ns if str(n) in d]
    pts.append((64, 1.0))
    path = " ".join(f"{'M' if i == 0 else 'L'}{X(n):.1f},{Y(v):.1f}" for i, (n, v) in enumerate(pts))
    g.append(f'<path class="line {cls}" d="{path}"/>')
    for n, v in pts[:-1]:
        g.append(f'<circle class="dot {cls}" cx="{X(n):.1f}" cy="{Y(v):.1f}" r="4" data-tip="{E(name)}: with {n} colonies, {v:.0%} of the 64-colony calls still come out the same"/>')
    labs.append([Y(pts[-2][1]) + 4, name.split(" (")[0]])
labs.sort()
for i in range(1, len(labs)):
    labs[i][0] = max(labs[i][0], labs[i-1][0] + 15)
for y, t in labs:
    g.append(f'<text class="dl" x="{cw-mr+8}" y="{y:.1f}">{E(t)}</text>')
g.append(f'<text class="tick" x="{(ml+cw-mr)/2:.0f}" y="{ch-1}" text-anchor="middle">colonies per arm (subsampled from the 64 that ran)</text>')
chart2 = f'<svg viewBox="0 0 {cw} {ch}" class="chart" role="img" aria-label="Calls kept versus colonies">{"".join(g)}</svg>'
leg2 = "".join(f'<li><span class="sw {c}"></span>{E(n)}</li>' for k, n, c in bins)

# ---------- chart 3: call stability by garden round (48-round runs) ----------
by = collections.defaultdict(lambda: [0, 0, 0])
for r in noise["rounds"]:
    if r["final"] == 0 or r["final_round"] != 47: continue
    by[r["round"]][0 if r["call"] == r["final"] else (1 if r["call"] == 0 else 2)] += 1
rs = sorted(by)
X3 = lambda r: ml + r / 47 * (cw - ml - mr)
g = []
for t in (0, .25, .5, .75, 1):
    g.append(f'<line class="grid" x1="{ml}" x2="{cw-mr}" y1="{Y(t):.1f}" y2="{Y(t):.1f}"/><text class="tick" x="{ml-8}" y="{Y(t)+4:.1f}" text-anchor="end">{int(t*100)}%</text>')
for r in (1, 7, 15, 23, 31, 39, 47):
    g.append(f'<text class="tick" x="{X3(r):.1f}" y="{Y(0)+18:.0f}" text-anchor="middle">{r}</text>')
g.append(f'<line class="axis" x1="{ml}" x2="{cw-mr}" y1="{Y(0):.1f}" y2="{Y(0):.1f}"/>')
for idx, name, cls in ((0, "Same call as round 47", "s1"), (2, "Opposite call", "s2")):
    pts = [(r, by[r][idx] / sum(by[r])) for r in rs]
    g.append(f'<path class="line {cls}" d="{" ".join(("M" if i == 0 else "L") + f"{X3(r):.1f},{Y(v):.1f}" for i, (r, v) in enumerate(pts))}"/>')
    for r, v in pts:
        g.append(f'<circle class="dot {cls}" cx="{X3(r):.1f}" cy="{Y(v):.1f}" r="4" data-tip="Garden after round {r}: {v:.0%} {name.lower()}"/>')
    g.append(f'<text class="dl" x="{cw-mr+8}" y="{Y(pts[-2][1])+4:.1f}">{name}</text>')
g.append(f'<text class="tick" x="{(ml+cw-mr)/2:.0f}" y="{ch-1}" text-anchor="middle">garden round</text>')
chart3 = f'<svg viewBox="0 0 {cw} {ch}" class="chart" role="img" aria-label="Call stability by round">{"".join(g)}</svg>'
leg3 = '<li><span class="sw s1"></span>Same call as at round 47</li><li><span class="sw s2"></span>Opposite call</li>'

# ---------- alone table ----------
sym = {1: '<span class="c up">better</span>', -1: '<span class="c dn">worse</span>', 0: '<span class="c ns">no call</span>'}
NAME = {"cs3lexlesinh": "lesson + inherit nest", "cs8hcs": "hcs founder (vs hA1)", "cs9joincr": "cut_join + crater", "cs15roof": "roof rule", "cs15both": "roof + room rules",
        "cs20light": "light dig head", "cs23inh": "inheritance retry", "brood1crutch": "home-eat crutch", "bp1": "plants + breeding", "stdoff": "rules off from round 0", "farramp": "distance ramp"}
arows = []
for a in alone:
    agree = a["alone"] == a["vs_ctl"]
    kind = "agree" if agree else ("backwards" if a["alone"] == -a["vs_ctl"] and a["vs_ctl"] != 0 else ("missed" if a["alone"] == 0 else "overcalled"))
    e = early.get((a["batch"], a["idea"], a["metric"]))
    arows.append(f'<tr class="{kind}"><td>{E(NAME.get(a["idea"], a["idea"]))}<div class="sub">{E(a["batch"])}</div></td><td>{E(a["metric"])}</td>'
                 f'<td class="num">{a["med_idea_r1"]:.0f} → {a["med_idea"]:.0f}</td><td class="num">{a["med_ctl_r1"]:.0f} → {a["med_ctl"]:.0f}</td>'
                 f'<td>{sym[a["vs_ctl"]]}</td><td>{sym[a["alone"]]}</td><td><span class="tag t-{kind}">{ {"agree":"same","backwards":"backwards","missed":"missed it","overcalled":"false win"}[kind]}</span></td>'
                 f'<td class="num">{("r" + str(e["stable_from"])) if e else ""}</td></tr>')
akinds = collections.Counter(r.split('"')[1] for r in arows)

# ---------- ledger ----------
VAL = {"clear": ("Large effect", "History would have given the same call"), "dup": ("Exact rerun", "Control reproduced an earlier arm byte for byte"),
       "ns": ("No effect", "Control showed the idea did nothing measurable"), "selection": ("Changed the call", "Showed what selection alone does"),
       "moved": ("Changed the call", "Baseline had moved since the last run"), "void": ("Caught a bug", "Control's numbers exposed a harness bug"), "na": ("Exploration", "No comparison")}
CTLN = {"same": "Same batch", "hist": "Earlier arm reused", "none": "None"}
lrows = "".join(f'<tr data-v="{E(v)}" data-c="{E(c)}"><td class="num">{E(era)}</td><td><b>{E(b)}</b><div class="sub">{E(q)}</div></td><td>{E(CTLN[c])}</td><td>{E(o)}</td>'
                f'<td><span class="tag v-{E(v)}">{E(VAL[v][0])}</span></td><td class="note">{E(n)}</td></tr>' for era, b, q, c, o, v, n in L)
cnt_same = collections.Counter(v for _, _, _, c, _, v, _ in L if c == "same")
cnt_hist = collections.Counter(v for _, _, _, c, _, v, _ in L if c == "hist")
D = dict(chart1=chart1, chart2=chart2, leg2=leg2, chart3=chart3, leg3=leg3, arows="".join(arows), lrows=lrows,
         n_same=sum(cnt_same.values()), same_dup=cnt_same["dup"], same_clear=cnt_same["clear"], same_ns=cnt_same["ns"],
         same_changed=cnt_same["selection"] + cnt_same["moved"] + cnt_same["void"], n_hist=sum(cnt_hist.values()), n_all=len(L),
         a_total=len(alone), a_agree=akinds["agree"], a_back=akinds["backwards"], a_over=akinds["overcalled"], a_miss=akinds["missed"])
tpl = open("page.tpl.html").read()
for k, v in D.items(): tpl = tpl.replace("{{" + k + "}}", str(v))
assert "{{" not in tpl, tpl[tpl.index("{{"):tpl.index("{{")+40]
open("/mnt/project-files/training-efficiency/training-compute-review.html", "w").write(tpl)
print(D["n_same"], cnt_same, cnt_hist, akinds)
