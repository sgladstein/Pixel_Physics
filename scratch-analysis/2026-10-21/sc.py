import sys,ast,statistics as st
def fl(f): return [ast.literal_eval(l[l.index("["):])[24] for l in open(f) if l.startswith("NOISE seed")]
def fe(f):
    v=[float(l.split()[3]) for l in open(f) if l.startswith("NOISE seed")]; return [min(v[i],v[i+1]) for i in range(0,len(v),2)]
mode=sys.argv[1]
if mode=="dig":
    a,b=fl(sys.argv[2]),fl(sys.argv[3]); d=[x-y for x,y in zip(a,b)]
    print(f"{len(a)} sealed 8-row worlds: {st.mean(a):.2f} full loads vs s161 {st.mean(b):.2f}, diff {st.mean(d):+.2f} se {st.stdev(d)/len(d)**.5:.2f}; at 10+: {sum(x>=10 for x in a)} of 24 (first 12: {sum(x>=10 for x in a[:12])})")
else:
    a,b=fe(sys.argv[2]),fe(sys.argv[3]); print(f"fetch, {len(a)} folded units on 96 fresh worlds: {st.mean(a):.2f} vs s91 {st.mean(b):.2f} = {st.mean(a)/st.mean(b):.2f} of s91 (keepref's own 22.015 -> {st.mean(a)/22.015:.2f})")
