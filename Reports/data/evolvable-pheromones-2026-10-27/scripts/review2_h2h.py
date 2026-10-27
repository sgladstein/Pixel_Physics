import csv, glob, statistics as st, sys
for arm in sys.argv[2:]:
    H=[];S=[];F=[];A=[];full=[]
    for d in sorted(glob.glob(f'{sys.argv[1]}/{arm}/*/')):
        try:
            N=list(csv.DictReader(open(d+'nest.csv'))); R=list(csv.DictReader(open(d+'stock.csv')))
        except OSError: continue
        H.append(int(N[-1]['food_home'])); S.append(int(R[-1]['nest_food'])+int(R[-1]['door_food']))
        f=[int(r['field_food']) for r in R if int(r['frame'])>3000]; F.append(st.mean(f)); full.append(sum(x>=55 for x in f)/len(f)); A.append(int(R[-1]['alive']))
    print(f'{arm:8} n {len(H)} food_home {st.median(H):.0f} stored {st.median(S):.0f} field_food mean {st.median(F):.1f} heap-full share {st.median(full):.2f} alive {st.median(A):.0f}')
