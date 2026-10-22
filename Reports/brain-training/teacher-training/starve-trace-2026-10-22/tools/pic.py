import json,glob,csv,collections,sys
sys.path.insert(0,'tools'); from rank import cause
import matplotlib; matplotlib.use('Agg'); import matplotlib.pyplot as plt
C={"1":"#2a78d6","2":"#eb6834","3":"#1baf7a","0":"#eda100","4":"#e87ba4","5":"#9a9993"}
names={"0":"starved at round start, holding a pellet","1":"roamed past the food, never came back","2":"colony famine at home, holding pellets","3":"alone at home, pellet it can't put down","4":"pellet, out in the food band","5":"other"}
fig=plt.figure(figsize=(12,7.5),facecolor="#fcfcfb")
ax=fig.add_axes([0.13,0.62,0.35,0.30]); ax.set_facecolor("#fcfcfb")
arms=[("hA","hA1 founder\n(unselected)"),("sel","round-6 ants\n(after selection)"),("old","old ant\n(same worlds)")]
for j,(a,lab) in enumerate(arms):
    c=collections.Counter()
    for f in glob.glob(f"l/{a}[0-9]*/dead.json"):
        for o in json.load(open(f)): c[cause(o)[0]]+=1
    if a=="old":
        c=collections.Counter({"5":sum(1 for k in range(1,17) for r in csv.DictReader(open(f"l/old{k}/events.csv")) if r['kind']=='death')})
    b=0
    for k in ["1","2","3","0","4","5"]:
        if c[k]: ax.barh(j,c[k],left=b,color=C[k],height=0.6,edgecolor="#fcfcfb",linewidth=2); b+=c[k]
    ax.text(b+1,j,f"{b}",va='center',color="#0b0b0b",fontsize=10)
ax.set_yticks(range(3)); ax.set_yticklabels([x[1] for x in arms],fontsize=9); ax.invert_yaxis()
ax.set_xlabel("ants starved, 16 colonies x 24 ants x 200k frames",color="#52514e",fontsize=9)
for s in ["top","right"]: ax.spines[s].set_visible(False)
ax.set_title("What killed them",loc='left',fontsize=12,color="#0b0b0b")
h=[plt.Rectangle((0,0),1,1,color=C[k]) for k in ["1","2","3","0","4","5"]]
ax.legend(h,[names[k] for k in ["1","2","3","0","4","5"]],fontsize=8,frameon=False,loc="upper left",bbox_to_anchor=(-0.2,-0.22),ncol=2)
# where they died
ax2=fig.add_axes([0.58,0.62,0.38,0.30]); ax2.set_facecolor("#fcfcfb")
xs=[]
for a in ["hA","sel"]:
    for f in glob.glob(f"l/{a}[0-9]*/dead.json"):
        for o in json.load(open(f)): xs.append(o['dx'])
ax2.axvspan(15,35,color="#f0efec"); ax2.axvspan(-35,-15,color="#f0efec")
ax2.hist(xs,bins=range(-128,129,8),color="#2a78d6",edgecolor="#fcfcfb",linewidth=2)
ax2.text(25,ax2.get_ylim()[1]*0.9,"food",ha='center',fontsize=8,color="#52514e"); ax2.text(-25,ax2.get_ylim()[1]*0.9,"food",ha='center',fontsize=8,color="#52514e")
ax2.set_xlabel("columns from the nest at death (box walls at about ±124)",color="#52514e",fontsize=9)
ax2.set_title("Where they died (both new-ant arms, 147 ants)",loc='left',fontsize=12)
for s in ["top","right"]: ax2.spines[s].set_visible(False)
# one roamer
R,I="l/hA1/",4
t=[(int(r['frame']),int(r['x'])-128,float(r['tank'])) for r in csv.DictReader(open(R+"trace.csv")) if int(r['id'])==I]
ax3=fig.add_axes([0.08,0.08,0.38,0.27]); ax4=fig.add_axes([0.58,0.08,0.38,0.27])
for a in (ax3,ax4):
    a.set_facecolor("#fcfcfb"); [a.spines[s].set_visible(False) for s in ["top","right"]]
ax3.axhspan(15,35,color="#f0efec"); ax3.axhspan(-35,-15,color="#f0efec")
ax3.plot([x[0]/1000 for x in t],[x[1] for x in t],color="#2a78d6",lw=1.5)
ax3.set_ylabel("columns from nest",fontsize=9,color="#52514e"); ax3.set_xlabel("frame (thousands)",fontsize=9,color="#52514e")
ax3.set_title("One ant (hA1 seed 1, ant 4): where it was",loc='left',fontsize=11)
ax4.plot([x[0]/1000 for x in t],[x[2] for x in t],color="#eb6834",lw=1.5)
ax4.set_ylabel("tanks",fontsize=9,color="#52514e"); ax4.set_xlabel("frame (thousands)",fontsize=9,color="#52514e")
ax4.set_title("...and its energy: leaves fat, walks to the wall, starves there",loc='left',fontsize=11)
fig.savefig("/mnt/project-files/starve-trace/why-ants-starve-2026-10-22.png",dpi=110)
