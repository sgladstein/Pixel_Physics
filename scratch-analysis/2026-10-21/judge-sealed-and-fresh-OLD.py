"""SCRATCH: judge racing A/B arms. usage: python3 -I judge.py DIR name [name...]
Per brain: full loads on 24 sealed and 96 fresh 8-row worlds (12,000 frames), s.e. per mirrored pair; fetch and trail
(worse side of each pair, as trained) as a share of s91 on the same 96 worlds; against s171 (the start), unpaired."""
import sys, ast, statistics as st

D = sys.argv[1]


def rows(f, col=None):
    out = []
    for l in open(f"{D}/{f}"):
        if l.startswith("NOISE seed"):
            out.append(ast.literal_eval(l[l.index("["):])[24] if col == "fl" else float(l.split()[3]))
    return out


def pairs(v):
    return [(v[2 * i] + v[2 * i + 1]) / 2 for i in range(len(v) // 2)]


def ms(v):
    p = pairs(v)
    return st.fmean(p), st.stdev(p) / len(p) ** 0.5


def worse(v):
    return st.fmean(min(v[2 * i], v[2 * i + 1]) for i in range(len(v) // 2))


s91f, s91t = worse(rows("s91-fetch.txt")), worse(rows("s91-trail.txt"))
base = {k: ms(rows(f"s171-{k}.txt", "fl")) for k in ("sealed", "fresh")}
print(f"{'brain':7s} {'sealed 24':>16s} {'vs s171':>15s} {'fresh 96':>16s} {'vs s171':>15s} {'fetch/s91':>9s} {'trail/s91':>9s}")
for n in ["s171"] + sys.argv[2:]:
    line = f"{n:7s}"
    for k in ("sealed", "fresh"):
        m, se = ms(rows(f"{n}-{k}.txt", "fl"))
        line += f" {m:8.2f} +- {se:4.2f}"
        line += f" {m - base[k][0]:+7.2f} +- {(se ** 2 + base[k][1] ** 2) ** 0.5:4.2f}" if n != "s171" else f" {'':15s}"
    line += f" {worse(rows(f'{n}-fetch.txt')) / s91f:9.2f} {worse(rows(f'{n}-trail.txt')) / s91t:9.2f}"
    print(line)
