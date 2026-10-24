import sys
from PIL import Image, ImageDraw, ImageFont
S=sys.argv[1]; maps=sys.argv[3].split(',')
def load(d):
    L=open(f'{d}/nestmap.txt').read().split('\n'); h=L[0].split(' '); leg=h[2].split(',')
    return int(h[0]),int(h[1]),[[leg[ord(c)-65] if c!='?' else '?' for c in r] for r in L[1:] if r]
COL={'empty':(18,20,28),'soil':(110,80,55),'packedsoil':(70,48,32),'spoil':(140,110,70),'stone':(90,90,95),'nest':(200,170,60),'crumbs':(160,140,90),'?':(120,120,120)}
GND={'soil','packedsoil','spoil','stone'}
X0,X1,Y0,Y1,K=80,178,58,106,4
font=ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',13); big=ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf',14)
pw,ph=(X1-X0)*K,(Y1-Y0)*K
cols=['link off, frame 0','link on, frame 0 (green = link cells)','link on, frame 3000 (red emptied, blue refilled)']
W=8+3*(pw+8); H=40+len(maps)*(ph+50)
img=Image.new('RGB',(W,H),(245,245,240)); d=ImageDraw.Draw(img)
d.text((8,6),'inherit_link=1: each sealed room keeps a 2-wide run of last round\'s open cells to the door (never the surface row, top 6 rows avoided), lined. One old ant, no digging.',fill=(0,0,0),font=font)
def joined(g,x0,y0):
    pass
for i,m in enumerate(maps):
    oy=30+i*(ph+50)
    x0,y0,a=load(f'{S}/sw3/{m}/L0/f0'); _,_,b=load(f'{S}/sw3/{m}/L1/f0'); _,_,c=load(f'{S}/sw3/{m}/L1/f3000')
    stats=[]
    for j in range(3):
        ox=8+j*(pw+8); n1=n2=0
        for y in range(Y0,Y1):
            for x in range(X0,X1):
                pa,pb,pc=a[y-y0][x-x0],b[y-y0][x-x0],c[y-y0][x-x0]
                if j==0: col=COL.get(pa,(255,0,255))
                elif j==1:
                    col=COL.get(pb,(255,0,255))
                    if pb not in GND and pb!='?' and pa in GND: col=(60,200,90); n1+=1
                else:
                    col=COL.get(pc,(255,0,255))
                    if pb in GND and pc not in GND and pc!='?': col=(230,40,40); n1+=1
                    elif pb not in GND and pb!='?' and pc in GND: col=(60,140,255); n2+=1
                d.rectangle([ox+(x-X0)*K,oy+18+(y-Y0)*K,ox+(x-X0+1)*K-1,oy+18+(y-Y0+1)*K-1],fill=col)
        lab=[f'{m}: {cols[0]}',f'{cols[1]}: {n1} cells',f'{cols[2]}: {n1} / {n2}'][j]
        d.text((ox,oy),lab,fill=(0,0,0),font=font)
img.save(sys.argv[2])
