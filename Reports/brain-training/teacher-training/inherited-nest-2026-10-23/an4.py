import sys, os, re, statistics as st
sys.argv=['x','/','0']; src=open(sys.argv[0] if False else '/tmp/claude-0/-home-user-Pixel-Physics/3a00f6df-217d-51cc-af7b-c964241c2ed1/scratchpad/an.py').read().split('root=sys.argv[1]')[0]
exec(src)
root='/tmp/claude-0/-home-user-Pixel-Physics/3a00f6df-217d-51cc-af7b-c964241c2ed1/scratchpad/sw4'
Ns=[1,10,15,20,30,50,80]; rows={}
for t in sorted(os.listdir(root)):
    for N in Ns:
        d=f'{root}/{t}/N{N}'
        m=re.search(r'dropped (\d+) cells in small pockets, linked (\d+)',open(f'{d}/log').read())
        k,j,*_=stats(f'{d}/f0',f'{d}/f0')
        rows[(t,N)]=(int(m.group(1)),int(m.group(2)),k,j)
q=lambda v:(st.median(v),sorted(v)[int(.9*len(v))-1],max(v))
print('N   linked(med,p90,max)  dropped(med,p90,max)  joined/carved med  maps with links  carved med')
for N in Ns:
    v=[rows[(t,N)] for t in sorted(os.listdir(root))]
    print(N, q([x[1] for x in v]), q([x[0] for x in v]), '%.2f'%st.median([x[3]/max(1,x[2]) for x in v]), sum(1 for x in v if x[1]>0), st.median([x[2] for x in v]))
with open(root+'.tsv','w') as f:
    f.write('map\t'+'\t'.join(f'link{N}\tdrop{N}' for N in Ns)+'\n')
    for t in sorted(os.listdir(root)): f.write(t+'\t'+'\t'.join(f'{rows[(t,N)][1]}\t{rows[(t,N)][0]}' for N in Ns)+'\n')
