import sys
for path in sys.argv[1:]:
    n=sat=satnest=nest=0; satb=0; hd_sat=[]
    wopt=flat=0; wopt_sat=flat_sat=0
    for l in open(path):
        if l.startswith('M,'):
            p=l.split(','); c=[float(x) for x in p[10].split()]
            n+=1; a=c[4]; at=c[3]>0
            nest+=at
            if a>=0.9999: sat+=1; satnest+=at; h=c[12]; hd_sat.append(32*h/(1-h))
            if c[5]>=0.9999: satb+=1
            cura=a
        elif l.startswith('O,'):
            for op in l.rstrip().split(',',5)[5].split(';'):
                if '|' not in op: continue
                head,nz=op.split('|')
                if head.split()[0]!='0': continue  # walk
                d=dict(kv.split(':') for kv in nz.split())
                wopt+=1; z=float(d.get('10','0'))==0
                flat+=z
                if cura>=0.9999: wopt_sat+=1; flat_sat+=z
    hd_sat.sort()
    print(path.split('/')[-1], f"decisions {n}: scent A at the ceiling {sat/n*100:.1f}% (at nest {satnest}/{sat}), B at ceiling {satb/n*100:.1f}%; home dist of saturated decisions p50={hd_sat[len(hd_sat)//2] if hd_sat else 0:.0f} p90={hd_sat[int(.9*len(hd_sat))] if hd_sat else 0:.0f}")
    print(f"   walk options with A-difference exactly 0: {flat/max(wopt,1)*100:.1f}% overall, {flat_sat/max(wopt_sat,1)*100:.1f}% when standing at the ceiling")
