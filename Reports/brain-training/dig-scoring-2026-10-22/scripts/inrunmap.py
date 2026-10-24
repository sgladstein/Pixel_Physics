"""Score an in-run colony's map with the niche prototype and print what the score charges."""
import sys, os
sys.argv_saved = sys.argv; sys.argv = ["x"]
exec(open(os.path.join(os.path.dirname(__file__), "climb2.py")).read().split("spec = json.load")[0])
sys.argv = sys.argv_saved
def load(run, frame):
    surf = [int(l) for l in open(f"{run}/surface.txt")]
    with open(f"{run}/map_f{frame:07d}.txt") as fh:
        x0, y0, w, h = map(int, fh.readline().split())
        rows = [ln.rstrip("\n") for ln in fh][:h]
    m = np.array([[c in ".af" for c in (r + " " * w)[:w]] for r in rows])
    ground = np.array([surf[x0 + i] - y0 for i in range(w)])
    return m, ground, x0, y0
if __name__ == "__main__":
    run, frame, door = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    m, ground, x0, y0 = load(run, frame)
    s, parts = ns.score(m, ground, door - x0)
    tot, parts2, nm, cr = score(m, ground, door - x0, verbose='parts')
    print(f"{run} f{frame}: proto {s['score']} niches {tot} dug {s['dug']} cracks {s['cracks']} pit {s['pit']} stray {s['stray']} chambers {s['chambers']} tunnel {s['tunnel_credit']}")
