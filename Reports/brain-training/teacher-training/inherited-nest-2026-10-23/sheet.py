import sys
from PIL import Image, ImageDraw, ImageFont
S=sys.argv[1]
def load(d):
    L=open(f'{d}/nestmap.txt').read().split('\n'); h=L[0].split(' '); leg=h[2].split(',')
    return int(h[0]),int(h[1]),[[leg[ord(c)-65] if c!='?' else '?' for c in r] for r in L[1:] if r]
COL={'empty':(18,20,28),'soil':(110,80,55),'packedsoil':(70,48,32),'spoil':(140,110,70),'stone':(90,90,95),'nest':(200,170,60),'crumbs':(160,140,90),'?':(120,120,120)}
X0,X1,Y0,Y1,K=98,162,58,102,5
GND={'soil','packedsoil','spoil','stone'}
fr=[0,10,20,50,100,400]
try: font=ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',13); big=ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf',16)
except: font=big=ImageFont.load_default()
pw,ph=(X1-X0)*K,(Y1-Y0)*K
W=len(fr)*(pw+8)+8+pw+8; H=2*(ph+60)+70
img=Image.new('RGB',(W,H),(245,245,240)); d=ImageDraw.Draw(img)
d.text((8,6),'Inherited nest (cs3lexlesinh c5 map), one old ant, no digging. Red = ground that emptied since frame 0, blue = carved hole that refilled.',fill=(0,0,0),font=font)
for row,(L,name) in enumerate([('0','Today: carved into loose soil'),('1','Fix: carved cells lined like an ant cut')]):
    oy=30+row*(ph+60)
    d.text((8,oy),name,fill=(0,0,0),font=big)
    x0,y0,a=load(f'{S}/fc{L}/f0')
    for i,f in enumerate(fr):
        _,_,b=load(f'{S}/fc{L}/f{f}')
        ox=8+i*(pw+8); emp=ref=0
        for y in range(Y0,Y1):
            for x in range(X0,X1):
                p=a[y-y0][x-x0]; q=b[y-y0][x-x0]
                c=COL.get(q,(255,0,255))
                if p in GND and q not in GND and q!='?': c=(230,40,40); emp+=1
                elif p not in GND and p!='?' and q in GND: c=(60,140,255); ref+=1
                d.rectangle([ox+(x-X0)*K,oy+22+(y-Y0)*K,ox+(x-X0+1)*K-1,oy+22+(y-Y0+1)*K-1],fill=c)
        d.text((ox,oy+24+ph),f'frame {f}: emptied {emp}, refilled {ref}',fill=(0,0,0),font=font)
    g=Image.open(f'{S}/film{L}/film/f0000400.png')
    # film crop starts at x=NEST-80=48, y=30
    g=g.crop((X0-48,Y0-30,X1-48,Y1-30)).resize((pw,ph),Image.NEAREST)
    ox=8+len(fr)*(pw+8); img.paste(g,(ox,oy+22)); d.text((ox,oy+24+ph),'game render, frame 400',fill=(0,0,0),font=font)
img.save(sys.argv[2])
