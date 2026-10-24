import sys, os, numpy as np
sys.argv=["x"]; exec(open("inrunmap.py").read().split('if __name__')[0])
import matplotlib; matplotlib.use("Agg"); import matplotlib.pyplot as plt
R=sys_R="/tmp/claude-0/-home-user-Pixel-Physics/d8079a03-3861-5d88-a8a6-9ab879927dc0/scratchpad/inrun/crowd/"
panels=[("n1_s6",12000,"One ant, 12k frames\n(the length of its training trial)"),("n1_s6",48000,"Same ant, 48k frames"),("n1_s6",200000,"Same ant, 200k frames"),("n30_s4",200000,"30 ants, 200k frames")]
C=dict(soil=(0.45,0.34,0.25),sky=(0.86,0.90,0.95),pit=(0.85,0.22,0.18),nest=(0.16,0.42,0.75),spoil=(0.78,0.62,0.40),ant=(0.1,0.1,0.1),food=(0.35,0.65,0.3))
fig,axs=plt.subplots(1,4,figsize=(16,4.6),facecolor="#fcfcfb")
for ax,(run,fr,title) in zip(axs,panels):
    m,ground,x0,y0=load(R+run,fr)
    raw=open(f"{R}{run}/map_f{fr:07d}.txt").read().split("\n")[1:1+m.shape[0]]
    h,w=m.shape; img=np.zeros((h,w,3))
    for xx in range(w):
        sky=True
        for yy in range(h):
            ch=(raw[yy]+" "*w)[xx]; below=yy>=ground[xx]
            if not m[yy,xx]: sky=False
            if ch=="#": img[yy,xx]=C["soil"] if below else C["spoil"]
            elif ch=="f": img[yy,xx]=C["food"]
            elif ch=="a": img[yy,xx]=C["ant"]
            else: img[yy,xx]=(C["pit"] if sky else C["nest"]) if below else C["sky"]
    s,_=ns.score(m,ground,128-x0); tot,*_=score(m,ground,128-x0,verbose='parts')
    g=int(ground[128-x0])
    ax.imshow(img[g-12:g+16,128-x0-30:128-x0+30],interpolation="nearest"); ax.set_xticks([]); ax.set_yticks([])
    for sp in ax.spines.values(): sp.set_visible(False)
    ax.set_title(title,loc="left",fontsize=11)
    ax.text(0,1.0,f"open pit {s['pit']} cells · roofed nest {s['dug']-s['pit']} · nest score {tot:.0f}",transform=ax.transAxes,fontsize=9,va="bottom",color="#52514e") if False else ax.set_xlabel(f"open pit {s['pit']} cells, nest score {tot:.0f}",fontsize=10,color="#333")
import matplotlib.patches as mp
fig.legend(handles=[mp.Patch(color=C[k],label=l) for k,l in (("pit","dug, open to the sky (crater)"),("nest","dug, under a roof"),("spoil","spoil heaped above ground"),("soil","untouched ground"))],loc="lower center",ncol=4,frameon=False,fontsize=10)
fig.suptitle("Trained diggers dig one shaft, then spend the rest of their lives widening its mouth into a crater",x=0.01,ha="left",fontsize=13)
fig.tight_layout(rect=(0,0.07,1,0.94)); fig.savefig(sys_out:="/mnt/project-files/nest-score/crater-2026-10-21.png",dpi=110)
print(sys_out)
