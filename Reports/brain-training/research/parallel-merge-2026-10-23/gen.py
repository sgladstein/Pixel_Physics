import sys
def load(p):
    lines=open(p).read().split('\n')
    n=int(lines[0].split()[1]); g=[0.0]*n
    for l in lines[1:]:
        if l.strip():
            i,v=l.split(); g[int(i)]=float(v)
    return lines[0],g
def save(p,head,g):
    with open(p,'w') as f:
        f.write(head+'\n')
        for i,v in enumerate(g):
            if v!=0.0: f.write(f'{i} {repr(float(v))}\n')
