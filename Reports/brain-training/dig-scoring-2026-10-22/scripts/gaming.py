import sys; sys.argv=["x"]
exec(open("climb2.py").read().split("spec = json.load")[0])
G, X0 = 64, 128
def base(depth):
    m = np.zeros((128, 256), bool); m[:G] = True; m[G:G + depth, X0 - 2:X0 + 2] = True; return m
def comb(tall, gap, length, both=True, depth=56):
    m = base(depth); y = G + 4
    while y + tall <= G + depth:
        m[y:y + tall, X0 + 2:X0 + 2 + length] = True
        if both: m[y:y + tall, X0 - 2 - length:X0 - 2] = True
        y += tall + gap
    return m
for tall, gap, L in [(1, 1, 30), (2, 1, 30), (2, 2, 30), (3, 1, 30), (4, 1, 30), (4, 3, 30), (2, 1, 60)]:
    m = comb(tall, gap, L)
    print(f"comb tall {tall} wall {gap} long {L}: dug {int((m[G:]).sum())}  as built {ns.score(m, G, X0)[0]['score']:7.1f}  with niches {score(m, G, X0):7.1f}")
