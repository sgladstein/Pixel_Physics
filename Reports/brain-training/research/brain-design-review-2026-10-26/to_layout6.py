# to_layout6.py IN OUT : a genome from this review's scratch build -- layout 5 (core 2797, dig head 482, lay gate 6 =
# 3285 slots) with homefit's scratch gate rows appended past it (3285 + 16 x 35) -- rewritten in layout 6, where the
# gate rows sit right after the core (2797..3357) and the riders follow. A file of plain layout 5 needs nothing: every
# reader gains the gates at zero (`mind::enter_gate`). Use on unit8_gated_fit.raw.txt, or on a pool member written by
# mkarm.py after appending gate_GI.txt's rows at 3285 + their index.
import sys
CORE, OLD, GATES = 2797, 3285, 16 * 35
src, out = sys.argv[1], sys.argv[2]
lines = [l.split() for l in open(src).read().splitlines()[1:] if l.strip()]
def move(i):
    if i < CORE: return i
    if i < OLD: return i + GATES          # dig head and lay gate move along by the gate block
    return i - OLD + CORE                 # the scratch gate rows land in the gate block
n = OLD + GATES
w = sorted((move(int(i)), v) for i, v in lines)
open(out, "w").write(f"genome {n} layout=6\n" + "".join(f"{i} {v}\n" for i, v in w))
print(out, len(w), "weights; gate weights", sum(1 for i, _ in w if CORE <= i < CORE + GATES))
