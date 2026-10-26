# mkarm.py FIT OUTDIR : colony 13's members (c13/, split.py of assets/minds/std_pool.txt, feature 18 zeroed; layout 5)
# written in layout 6 with scorer unit 8 -- its context, feature and verb rows, output weight and context gate row --
# replaced by FIT's (a layout-6 genome, e.g. unit8_gated_fit.raw.txt). Run the pool with inrun founders= as usual.
# (During the review the gate rows were read from a side file, PP_GATE_FILE, in a scratch build; since layout 6 they
# live in the genome, and to_layout6.py converts a file from that build.)
import sys, glob, os
fit, out = sys.argv[1], sys.argv[2]
U = 8; CORE, OLD, GATES = 2797, 3285, 16 * 35
rows = set(range(23 + 35 * U, 23 + 35 * U + 35)) | set(range(583 + 45 * U, 583 + 45 * U + 45)) \
     | set(range(1303 + 9 * U, 1303 + 9 * U + 9)) | {1447 + U} | set(range(CORE + 35 * U, CORE + 35 * U + 35))
head = open(fit).readline()
assert "layout=6" in head, f"{fit}: not a layout-6 genome (convert it with to_layout6.py)"
f = {int(i): w for i, w in (l.split() for l in open(fit).read().splitlines()[1:] if l.strip())}
os.makedirs(out, exist_ok=True)
for p in sorted(glob.glob('c13/m*.raw.txt')):
    L = open(p).read().splitlines()
    assert L[0].split()[1] == str(OLD), f"{p}: expected a layout-5 member of {OLD} slots"
    keep = {}
    for l in L[1:]:
        i, w = l.split(); i = int(i)
        i = i if i < CORE else i + GATES          # riders move along by the gate block
        if i not in rows: keep[i] = w
    keep.update({i: w for i, w in f.items() if i in rows})
    open(os.path.join(out, os.path.basename(p)), 'w').write(
        f"genome {OLD + GATES} layout=6\n" + "".join(f"{i} {keep[i]}\n" for i in sorted(keep)))
print(out, 'unit', U, 'from', fit)
