import csv, matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from collections import defaultdict
import statistics as st
NEST=128
B,O,G,R='#2a6fdb','#d9731a','#8a8f98','#c0392b'
INK='#222'
plt.rcParams.update({'font.size':10,'axes.edgecolor':'#bbb','axes.labelcolor':INK,'xtick.color':INK,'ytick.color':INK,'axes.spines.top':False,'axes.spines.right':False})
fig,ax=plt.subplots(1,3,figsize=(15,4.6))
# (a) alive over time for 3 seeds
for s,c in zip((21,22,23),(B,O,G)):
    rows=list(csv.DictReader(open(f'/tmp/pt/s{s}/stock.csv')))
    ax[0].plot([int(r['frame'])/1000 for r in rows],[int(r['alive']) for r in rows],color=c,lw=1.6,label=f'seed {s}')
ax[0].set_title('Headcount over 100k frames',loc='left',fontsize=11)
ax[0].set_xlabel('frames (thousands)'); ax[0].set_ylabel('ants alive'); ax[0].legend(frameon=False)
ax[0].text(0.02,0.04,'dips = most of the colony starving within ~2k frames;\nseed 21 ends as one ant at 120 tanks',transform=ax[0].transAxes,color='#555',fontsize=9)
# (b) per ant: share of life at food zone vs mean tank
for s,c in zip((21,22,23),(B,O,G)):
    per=defaultdict(lambda:[0,0,0.0])
    for r in csv.DictReader(open(f'/tmp/pt/s{s}/trace.csv')):
        p=per[int(r['id'])]; p[0]+=1; p[1]+=14<=abs(int(r['x'])-NEST)<40; p[2]+=float(r['tank'])
    xs=[100*p[1]/p[0] for p in per.values() if p[0]>=100]; ys=[p[2]/p[0] for p in per.values() if p[0]>=100]
    ax[1].scatter(xs,ys,s=14,color=c,alpha=.6,edgecolor='none',label=f'seed {s}')
ax[1].set_yscale('log'); ax[1].set_title('Each ant: where it lives vs how fed it is',loc='left',fontsize=11)
ax[1].set_xlabel('% of life spent at the food heaps'); ax[1].set_ylabel('mean tank (tanks, log)')
ax[1].text(4,1.0,'never go out: 1-10 tanks',color=R,fontsize=9); ax[1].text(62,2.0,'heap residents:\n10-90+ tanks',color='#555',fontsize=9)
# (c) life of ants that starved, by origin
life=defaultdict(list)
for s in (21,22,23):
    kind={}
    for r in csv.DictReader(open(f'/tmp/pt/s{s}/births.csv')):
        if r['kind'] in ('birth','hall','refill'): kind[int(r['id'])]=r['kind']
    first={}
    for r in csv.DictReader(open(f'/tmp/pt/s{s}/trace.csv')): first.setdefault(int(r['id']),int(r['frame']))
    for l in open(f'/tmp/pt/s{s}/events.csv'):
        e=l.strip().split(',')
        if e[2]=='death' and e[5]=='1' and int(e[1]) in first:
            k=kind.get(int(e[1]),'founder'); k='refill/hall' if k in('hall','refill') else k
            life[k].append((int(e[0])-first[int(e[1])])/1000)
order=['founder','birth','refill/hall']; cols=[B,O,R]
data=[life[k] for k in order]
bp=ax[2].boxplot(data,orientation='horizontal',widths=.55,patch_artist=True,showfliers=False)
for p,c in zip(bp['boxes'],cols): p.set_facecolor(c); p.set_alpha(.55); p.set_edgecolor(c)
for m in bp['medians']: m.set_color(INK)
ax[2].set_yticklabels([f'{k}\n(n={len(life[k])} starved)' for k in order]); ax[2].set_xscale('log')
ax[2].set_xlabel('frames lived before starving (thousands, log)'); ax[2].set_title('How long the ants that starved lived',loc='left',fontsize=11)
for i,k in enumerate(order): ax[2].text(st.median(life[k])*1.05,i+1.33,f'median {st.median(life[k]):.1f}k',fontsize=9,color='#555')
fig.suptitle('h1 mix, in-run colony, 3 seeds x 100k frames (scratch harness, current compute branch)',x=0.01,ha='left',fontsize=10,color='#555')
plt.tight_layout(); plt.savefig('/tmp/pt/playtest-findings.png',dpi=130,facecolor='white')
