#!/usr/bin/env python3
"""Learning curves for training-rig runs (`examples/train.rs`).

    python3 scripts/traincurves.py RUN_DIR [RUN_DIR ...] [--out curves.svg] [--title T] [--ref 0.22 "hand-wired"]

Reads each run's `progress.csv` and `holdout.csv` and draws one chart:

* thin line: the training best per generation (selected on, so optimistic);
* thick line with dots: the champion on hold-out layouts (the honest curve);
* dashed: the champion with its senses cut (if it rises too, what was learned
  is a habit, not a response to the world);
* dotted grey: the start genome as control;
* `--ref`: a horizontal line for a reference score (a hand-built ant).

**Plateau** is read off the hold-out curve, not the training one: the training
best is the maximum of a noisy population and keeps creeping up on luck alone.
A run is called plateaued when its last hold-out score is within 5% of its
score one checkpoint earlier, and the script prints the gain per checkpoint
so a reader can judge for themselves. No dependencies beyond the standard
library: the chart is hand-written SVG, and a PNG is made beside it when
`rsvg-convert` or a headless Chromium is available (`to_png`).
"""
import csv
import os
import sys

COLOURS = ["#2b6cb0", "#c05621", "#2f855a", "#9b2c2c", "#6b46c1", "#b7791f"]


def read(run):
    prog = list(csv.DictReader(open(os.path.join(run, "progress.csv"))))
    hold = list(csv.DictReader(open(os.path.join(run, "holdout.csv")))) if os.path.exists(os.path.join(run, "holdout.csv")) else []
    best = [(int(r["gen"]), float(r["best"])) for r in prog]
    control = [(int(r["gen"]), float(r["control"])) for r in prog]
    champ = [(int(r["gen"]), float(r["fit"])) for r in hold if r["who"] == "champion"]
    blind = [(int(r["gen"]), float(r["fit"])) for r in hold if r["who"] == "blinded"]
    start = [(int(r["gen"]), float(r["fit"])) for r in hold if r["who"] == "start"]
    return dict(best=best, control=control, champ=champ, blind=blind, start=start)


def plateau(champ):
    if len(champ) < 2:
        return "too few hold-outs to say"
    steps = []
    for (g0, a), (g1, b) in zip(champ, champ[1:]):
        steps.append(f"gen {g0}->{g1}: {a:.3f}->{b:.3f}")
    a, b = champ[-2][1], champ[-1][1]
    flat = b <= a * 1.05 + 1e-9
    return ("PLATEAU" if flat else "still rising") + " (" + "; ".join(steps) + ")"


def to_png(svg, w, h):
    """A PNG beside the SVG, because a file preview shows an SVG as its
    source text. `rsvg-convert` if installed, else a headless Chromium
    (`TRAINCURVES_CHROME`, or the one this repo's cloud box ships under
    /opt/pw-browsers). Returns the PNG path, or None if neither is there."""
    import glob
    import shutil
    import subprocess
    png = os.path.splitext(svg)[0] + ".png"
    if shutil.which("rsvg-convert"):
        if subprocess.run(["rsvg-convert", "-o", png, svg], capture_output=True).returncode == 0:
            return png
    chromes = [os.environ.get("TRAINCURVES_CHROME", "")] + sorted(glob.glob("/opt/pw-browsers/chromium_headless_shell-*/chrome-linux/headless_shell"))
    for c in chromes:
        if c and os.path.exists(c):
            r = subprocess.run([c, "--no-sandbox", "--hide-scrollbars", f"--window-size={w},{h}", f"--screenshot={os.path.abspath(png)}", "file://" + os.path.abspath(svg)], capture_output=True, timeout=120)
            if r.returncode == 0 and os.path.exists(png):
                return png
    return None


def main():
    args = sys.argv[1:]
    out = "curves.svg"
    title = None
    refs = []
    runs = []
    i = 0
    while i < len(args):
        if args[i] == "--out":
            out = args[i + 1]
            i += 2
        elif args[i] == "--title":
            title = args[i + 1]
            i += 2
        elif args[i] == "--ref":
            refs.append((float(args[i + 1]), args[i + 2]))
            i += 3
        else:
            runs.append(args[i])
            i += 1
    if not runs:
        sys.exit(__doc__)
    data = [(os.path.basename(os.path.normpath(r)), read(r)) for r in runs]
    gmax = max(max(g for g, _ in d["best"]) for _, d in data)
    ymax = max([v for _, d in data for k in ("best", "champ", "blind") for _, v in d[k]] + [v for v, _ in refs] + [1e-6]) * 1.08

    W, H, L, R, T, B = 760, 420, 60, 170, 30, 40
    pw, ph = W - L - R, H - T - B
    x = lambda g: L + pw * g / max(gmax, 1)
    y = lambda v: T + ph * (1 - v / ymax)
    s = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" font-family="sans-serif" font-size="12">',
         f'<rect width="{W}" height="{H}" fill="white"/>']
    for k in range(6):
        v = ymax * k / 5
        s.append(f'<line x1="{L}" x2="{L + pw}" y1="{y(v):.1f}" y2="{y(v):.1f}" stroke="#e2e8f0"/>')
        s.append(f'<text x="{L - 6}" y="{y(v) + 4:.1f}" text-anchor="end" fill="#4a5568">{v:.2f}</text>')
    for k in range(0, gmax + 1, max(1, gmax // 8)):
        s.append(f'<text x="{x(k):.1f}" y="{T + ph + 16}" text-anchor="middle" fill="#4a5568">{k}</text>')
    s.append(f'<text x="{L + pw / 2}" y="{H - 6}" text-anchor="middle" fill="#2d3748">generation</text>')
    s.append(f'<text x="14" y="{T + ph / 2}" text-anchor="middle" fill="#2d3748" transform="rotate(-90 14 {T + ph / 2})">score</text>')
    if title is None:
        title = "Learning curves: " + ", ".join(n for n, _ in data)
    s.append(f'<text x="{L}" y="18" fill="#1a202c" font-size="14" font-weight="bold">{title}</text>')

    def poly(pts, colour, width=1.0, dash=None, dots=False):
        if not pts:
            return
        d = " ".join(f"{x(g):.1f},{y(v):.1f}" for g, v in pts)
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        s.append(f'<polyline points="{d}" fill="none" stroke="{colour}" stroke-width="{width}"{extra}/>')
        if dots:
            for g, v in pts:
                s.append(f'<circle cx="{x(g):.1f}" cy="{y(v):.1f}" r="3.5" fill="{colour}"/>')

    for v, label in refs:
        s.append(f'<line x1="{L}" x2="{L + pw}" y1="{y(v):.1f}" y2="{y(v):.1f}" stroke="#1a202c" stroke-dasharray="8 4"/>')
        s.append(f'<text x="{L + pw + 6}" y="{y(v) + 4:.1f}" fill="#1a202c">{label}</text>')
    ly = T + 10
    for n, (name, d) in enumerate(data):
        c = COLOURS[n % len(COLOURS)]
        poly(d["best"], c, 1.0)
        poly(d["champ"], c, 2.5, dots=True)
        poly(d["blind"], c, 1.2, dash="4 3")
        poly(d["control"], "#a0aec0", 1.0, dash="1 3")
        s.append(f'<rect x="{L + pw + 6}" y="{ly + 30 * n + 12}" width="10" height="10" fill="{c}"/>')
        s.append(f'<text x="{L + pw + 20}" y="{ly + 30 * n + 21}" fill="#2d3748">{name}</text>')
    key = ["thin: training best", "thick+dots: hold-out", "dashed: senses cut", "dotted grey: start"]
    for k, t in enumerate(key):
        s.append(f'<text x="{L + pw + 6}" y="{H - B - 60 + 15 * k}" fill="#4a5568" font-size="10">{t}</text>')
    s.append("</svg>")
    open(out, "w").write("\n".join(s))
    print(f"wrote {out}")
    png = to_png(out, W, H)
    if png:
        print(f"wrote {png}")
    for name, d in data:
        print(f"{name}: {plateau(d['champ'])}")


if __name__ == "__main__":
    main()
