"""state.txt's `genome top` block (index, f32 hex) -> a genome file, header copied from a reference genome file."""
import sys, struct, re
state, ref, out = sys.argv[1:4]
L = open(state).read().splitlines(); i = L.index(next(l for l in L if l.startswith("genome top "))) + 1
rows = []
while i < len(L) and re.fullmatch(r"\d+ [0-9a-f]{8}", L[i]):
    k, h = L[i].split(); rows.append((int(k), struct.unpack(">f", bytes.fromhex(h))[0])); i += 1
hdr = open(ref).readline()
open(out, "w").write(hdr + "".join(f"{k} {v!r}\n" for k, v in rows))
print(out, len(rows), "weights")
