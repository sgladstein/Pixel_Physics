import csv,sys,glob,os
# For each crash (alive drops >=12 within 4k), who survives: old ants (in snapshot before) at home vs field, or newborns.
tot={'old_home':0,'old_field':0,'new':0,'died_home':0,'died_field':0}
for r in sorted(glob.glob(sys.argv[1]+'/*_[1-8]/r*')):
    try: rows=list(csv.DictReader(open(r+'/sel.csv')))
    except: continue
    last=-99999
    for i in range(2,len(rows)):
        f=int(rows[i]['frame']); a0=int(rows[i-2]['alive']); a=int(rows[i]['alive'])
        if a0-a>=12 and f-last>8000:
            last=f
            s0=(f-1000)//4000*4000; s1=s0+8000
            p0=r+f'/ants_f{s0:07d}.txt'; p1=r+f'/ants_f{s1:07d}.txt'
            if not(os.path.exists(p0) and os.path.exists(p1)): continue
            A={l.split()[0]:int(l.split()[1]) for l in open(p0)}
            B={l.split()[0] for l in open(p1)}
            for k,x in A.items():
                home=abs(x-128)<=12
                if k in B: tot['old_home' if home else 'old_field']+=1
                else: tot['died_home' if home else 'died_field']+=1
            tot['new']+=len(B-set(A))
print(sys.argv[1],tot)
