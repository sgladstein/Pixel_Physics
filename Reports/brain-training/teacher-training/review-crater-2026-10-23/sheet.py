"""Contact sheet: rows = arms (label=glob), first N runs by seed; cyan = nest, pink = crater (fixed meter: crumbs do not
cut a room off, and a roofed pocket is never crater)."""
import sys,glob,os
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0,'/home/user/cloud/scratch/inrun')
import nestfix, roomshape
from PIL import Image, ImageDraw, ImageFont
out=sys.argv[1]; N=int(os.environ.get('N','6')); rows=[a.split('=',1) for a in sys.argv[2:]]
COL={'soil':(122,92,62),'packedsoil':(92,68,46),'spoil':(226,150,70),'nest':(150,90,170),'stone':(120,120,125),'crumbs':(240,210,80),'corpse':(200,40,40)}
PX=4; font=ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',13); big=ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf',16)
def sets(d):
    # re-run nestfix's flood to get the cell sets
    src=open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'nestfix.py')).read().replace("    return dict(nest=len(nest)","    SETS.update(nest=nest,roofed=roofed,below=below,e=e)\n    return dict(nest=len(nest)")
    SETS={}; g={'SETS':SETS,'__name__':'x'}; exec(src,g); r=g['count'](d); return SETS,r
def tile(d):
    t=open(d+'/nestmap.txt').read().split('\n'); h=t[0].split(' '); x0,y0,leg=int(h[0]),int(h[1]),h[2].split(',')
    g=[r for r in t[1:] if r]; surf=[int(l) for l in open(d+'/surface.txt') if l.strip()]
    S,r=sets(d); door=128-x0; a,b=door-20,door+20
    top=max(0,surf[128]-y0-12); g=g[top:top+52]
    im=Image.new('RGB',((b-a)*PX,len(g)*PX)); dr=ImageDraw.Draw(im)
    for yy,row in enumerate(g):
        y=yy+top
        for x in range(a,b):
            ch=row[x]
            if ch=='?': c=(90,200,90)
            else:
                m=leg[ord(ch)-65]
                if m!='empty': c=COL.get(m,(60,60,60))
                elif (x,y) in S['nest']: c=(40,210,230)
                elif S['below'](x,y) and not S['roofed'](x,y): c=(230,60,150)
                elif S['below'](x,y): c=(24,20,18)
                else: c=(52,60,78)
            dr.rectangle([(x-a)*PX,yy*PX,(x-a+1)*PX-1,(yy+1)*PX-1],fill=c)
    return im,r
W=40*PX; H=52*PX; CAP=34
sheet=Image.new('RGB',(170+N*(W+6),len(rows)*(H+CAP+10)+40),(14,14,18)); D=ImageDraw.Draw(sheet)
for i,(label,pat) in enumerate(rows):
    ds=sorted([d for d in glob.glob(pat) if os.path.exists(d+'/nestmap.txt')],key=lambda d:int(d.rstrip('/').rsplit('_',1)[1]))[:N]
    y=10+i*(H+CAP+10)
    for j,w in enumerate(label.split('|')): D.text((8,y+40+j*20),w,fill=(235,235,235),font=big if j==0 else font)
    for k,d in enumerate(ds):
        x=170+k*(W+6); t,r=tile(d); sheet.paste(t,(x,y))
        D.text((x,y+H+2),f"nest {r['nest']}  crater {r['crater']}",fill=(220,220,220),font=font)
        try: rs=roomshape.score(d)
        except Exception: rs=float('nan')
        D.text((x,y+H+17),f"rooms {rs:.0f}",fill=(220,220,220),font=font)
ly=sheet.height-26; lx=8
for name,c in [('nest (roofed, joined to door)',(40,210,230)),('crater (open to sky)',(230,60,150)),('open, roofed, cut off',(24,20,18)),('sky',(52,60,78)),('soil',(122,92,62)),('packed soil',(92,68,46)),('spoil',(226,150,70)),('stored food',(240,210,80)),('ants / plants',(90,200,90))]:
    D.rectangle([lx,ly,lx+14,ly+14],fill=c); D.text((lx+20,ly),name,fill=(220,220,220),font=font); lx+=30+int(D.textlength(name,font=font))
sheet.save(out); print(out,sheet.size)
