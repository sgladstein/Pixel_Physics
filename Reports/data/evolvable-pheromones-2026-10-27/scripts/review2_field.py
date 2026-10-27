import csv, glob, statistics as st, sys, os
root=sys.argv[1]
for arm in ['free_s1','deaf_s1']:
    for gl in [(0,10),(140,150)]:
        ff=[];full=[];alive=[];stored=[];zero=[]
        for g in range(*gl):
            for d in glob.glob(f'{root}/{arm}/g{g:03}/r*/'):
                try: R=list(csv.DictReader(open(d+'stock.csv')))
                except OSError: continue
                if not R: continue
                f=[int(r['field_food']) for r in R if int(r['frame'])>3000]
                ff.append(st.mean(f)); full.append(sum(x>=55 for x in f)/len(f)); zero.append(sum(x<=5 for x in f)/len(f))
                alive.append(st.median(int(r['alive']) for r in R)); stored.append(int(R[-1]['nest_food'])+int(R[-1]['door_food']))
        print(arm, gl, 'runs',len(ff),'mean field food %.1f'%st.median(ff),'share of time heap>=55: %.2f'%st.median(full),'share field<=5: %.2f'%st.median(zero),'median alive %.0f'%st.median(alive),'stored %.0f'%st.median(stored))
