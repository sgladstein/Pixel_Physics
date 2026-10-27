import csv, glob, statistics as st, sys
root=sys.argv[1]
def corr(a,b):
    ma,mb=st.mean(a),st.mean(b); sa=sum((x-ma)**2 for x in a)**.5; sb=sum((y-mb)**2 for y in b)**.5
    return sum((x-ma)*(y-mb) for x,y in zip(a,b))/(sa*sb) if sa and sb else float('nan')
for arm in ['free_s1','deaf_s1']:
    for gl in [(0,10),(140,150)]:
        H=[];S=[];A=[]
        for g in range(*gl):
            for d in glob.glob(f'{root}/{arm}/g{g:03}/r*/'):
                try:
                    N=list(csv.DictReader(open(d+'nest.csv'))); R=list(csv.DictReader(open(d+'stock.csv')))
                except OSError: continue
                if not N or not R: continue
                H.append(int(N[-1]['food_home'])); S.append(int(R[-1]['nest_food'])+int(R[-1]['door_food'])); A.append(int(R[-1]['alive']))
        print(arm,gl,'n',len(H),'food_home med %.0f'%st.median(H),'stored med %.0f'%st.median(S),'alive end %.0f'%st.median(A),'r(home,stored)=%.2f'%corr(H,S),'r(alive,stored)=%.2f'%corr(A,S), 'last nest.csv frame', N[-1]['frame'])
