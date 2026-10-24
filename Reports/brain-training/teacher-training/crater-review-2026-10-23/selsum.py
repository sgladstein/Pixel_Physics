import csv,sys,statistics as st
for arm in sys.argv[1:]:
    r=list(csv.DictReader(open(arm+'/cg.csv')))
    for R in sorted(set(int(x['round']) for x in r)):
        q=[x for x in r if int(x['round'])==R]; f=lambda k: st.median(float(x[k]) for x in q)
        print(arm.rsplit('/',1)[1],'garden after round',R,'n',len(q),'food',f('home8'),'starved',f('starved'),'cuts',f('cuts'),'nest',f('nest'),'crater',f('crater'),'kept late cuts',f('nestcuts_late'),'rooms',f('shape'))
