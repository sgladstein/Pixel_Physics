"""SCRATCH (teacher lane): film/*.png from inrun film=N -> brightened, captioned frames for ffmpeg."""
import sys, csv, glob, re, bisect
from PIL import Image, ImageDraw, ImageEnhance, ImageFont
run, out, title = sys.argv[1], sys.argv[2], sys.argv[3]
S = 6; Y0, Y1 = 16, 72
stock = [(int(r['frame']), r) for r in csv.DictReader(open(run + '/stock.csv'))]
sf = [f for f, _ in stock]
ev = [(int(r['frame']), r['kind']) for r in csv.DictReader(open(run + '/births.csv'))]
try:
    font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 18)
except OSError:
    font = ImageFont.load_default()
files = sorted(glob.glob(run + '/film/f*.png'))
for k, p in enumerate(files):
    fr = int(re.search(r'f(\d+)', p).group(1))
    im = Image.open(p).convert('RGB').crop((0, Y0, 161, Y1))
    im = ImageEnhance.Brightness(im).enhance(1.7)
    im = im.resize((im.width * S, im.height * S), Image.NEAREST)
    canvas = Image.new('RGB', (im.width, im.height + 56), (18, 18, 22))
    canvas.paste(im, (0, 56))
    i = bisect.bisect_right(sf, fr) - 1
    st = stock[i][1] if i >= 0 else None
    born = sum(1 for f, kd in ev if f <= fr and kd in ('birth', 'refill', 'hall'))
    culled = sum(1 for f, kd in ev if f <= fr and kd in ('cull', 'retire'))
    d = ImageDraw.Draw(canvas)
    d.text((10, 5), f'{title}   frame {fr:,}', fill=(235, 235, 235), font=font)
    if st:
        d.text((10, 29), f'ants {st["alive"]}   mean tank {float(st["tank_mean"]):.1f}   food stored in nest {st["nest_food"]}   born {born}   culled or retired {culled}', fill=(200, 200, 200), font=font)
    canvas.save(f'{out}/v{k:05d}.png')
