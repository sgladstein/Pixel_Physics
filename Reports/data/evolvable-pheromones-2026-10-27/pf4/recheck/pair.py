import json, sys, math, statistics as st
res = json.load(open(sys.argv[1]))
def sign(a, b):
    n = sum(1 for x, y in zip(a, b) if x != y); k = sum(1 for x, y in zip(a, b) if x > y)
    m = min(k, n - k); p = min(1.0, 2 * sum(math.comb(n, i) for i in range(m + 1)) / 2 ** n) if n else 1.0
    return k, n, p
def cmp(A, B, keys):
    sa, sb = res[A], res[B]
    seeds = sorted(set(sa) & set(sb))
    print(f'{A} vs {B} ({len(seeds)} paired worlds)')
    for k in keys:
        a = [sa[s][k] for s in seeds]; b = [sb[s][k] for s in seeds]
        hi, n, p = sign(a, b)
        print(f'   {k:10} {st.median(a):7.0f} vs {st.median(b):7.0f}   {A} higher {hi}/{n}  p={p:.4f}')
K = ['fresh', 'home', 'f_heap', 'heap_kj', 'f_near', 'kj', 'alive']
for a, b in [x.split(':') for x in sys.argv[2:]]:
    cmp(a, b, K)
