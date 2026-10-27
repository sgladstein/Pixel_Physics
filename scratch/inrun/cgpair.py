"""SCRATCH (selection lane, 2026-10-27): one arm's common garden against a control's, paired by colony.

Colony c's garden world is the same in every arm (colsel.py seeds it by colony and round), so arms pair by colony.
A colony missing from a garden died out in training (colsel skips it, `colsel-judging` memory): it counts as alive 0
and nest 0, and as the WORSE side of its pair on every measure, starvation included -- never as a colony that starved
none. Prints, per measure, the arm's and the control's medians (missing counted as above), how many pairs the arm is
better / worse / tied, and the two-sided sign test; `k=` sets the Bonferroni bar (0.05 / k) for the verdict column.

usage: cgpair.py CONTROL_cg.csv ARM_cg.csv [ARM_cg.csv ...] round=47 [k=2] [ncol=32]
"""
import csv, statistics as st, sys
from math import comb

HIGHER = {'starved': -1, 'alive': 1, 'nest': 1, 'home8': 1, 'crater': -1, 'scale': 1, 'rooms': 1, 'late_cuts': 1}


def signp(w, l):
    n, k = w + l, min(w, l)
    return 1.0 if n == 0 else min(1.0, 2 * sum(comb(n, i) for i in range(k + 1)) / 2 ** n)


def garden(path, rnd):
    return {int(r['colony']): r for r in csv.DictReader(open(path)) if int(r['round']) == rnd}


def main(a):
    opt = dict(x.split('=', 1) for x in a if '=' in x)
    files = [x for x in a if '=' not in x]
    rnd, k, ncol = int(opt.get('round', 47)), int(opt.get('k', len(files) - 1)), int(opt.get('ncol', 32))
    ctl = garden(files[0], rnd)
    print(f'garden round {rnd}, {ncol} colonies; control {files[0]} has {len(ctl)}; Bonferroni bar p < {0.05 / k:.3g} (k={k})')
    for f in files[1:]:
        arm = garden(f, rnd)
        print(f'\n{f}: {len(arm)} colonies in the garden')
        for m, s in HIGHER.items():
            better = worse = tie = 0
            va, vc = [], []
            for c in range(1, ncol + 1):
                x, y = arm.get(c), ctl.get(c)
                xv = float(x[m]) if x else None
                yv = float(y[m]) if y else None
                fill = lambda v: v if v is not None else (0.0 if m in ('alive', 'nest', 'home8', 'scale', 'rooms', 'late_cuts') else float('nan'))
                va.append(fill(xv)); vc.append(fill(yv))
                if xv is None and yv is None:
                    tie += 1
                elif xv is None:
                    worse += 1
                elif yv is None:
                    better += 1
                elif s * (xv - yv) > 0:
                    better += 1
                elif s * (xv - yv) < 0:
                    worse += 1
                else:
                    tie += 1
            med = lambda v: st.median([x for x in v if x == x]) if any(x == x for x in v) else float('nan')
            p = signp(better, worse)
            print(f'   {m:10s} arm {med(va):8.1f}  control {med(vc):8.1f}  better {better:2d} worse {worse:2d} tied {tie:2d}  '
                  f'p = {p:.3g}{"  *" if p < 0.05 / k else ""}')


if __name__ == '__main__':
    main(sys.argv[1:])
