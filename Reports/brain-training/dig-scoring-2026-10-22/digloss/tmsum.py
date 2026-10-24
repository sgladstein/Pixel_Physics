import csv, glob, sys, collections
for arm in sys.argv[1:]:
    c = collections.defaultdict(collections.Counter); ds = sorted(glob.glob(arm + '_[0-9]*'))
    for d in ds:
        for e in csv.DictReader(open(d + '/events.csv')):
            k = e['kind']; b = int(e['frame']) // 50000
            if k in ('dig', 'recut', 'out', 'in') and abs(int(e['x']) - 128) <= 30:
                c[b][k] += 1
                if k == 'out' and abs(int(e['x']) - 128) <= 5: c[b]['out<=5'] += 1
            elif k == 'deliver' and abs(int(e['x']) - 128) <= 12:
                c[b]['home'] += 1
            elif k == 'death' and e['v'] == '1':
                c[b]['starved'] += 1
        for r in csv.DictReader(open(d + '/nest.csv')):
            if int(r['frame']) % 50000 == 0:
                b = int(r['frame']) // 50000 - 1; c[b]['pit'] += int(r['pit']); c[b]['rooms'] += int(r['chambers']) + int(r['niches']); c[b]['dug'] += int(r['dug']); c[b]['nest'] += float(r['nest_score'])
    print(arm, len(ds))
    for b in range(4):
        print(f'  {b*50:3d}-{b*50+50}k ' + ' '.join(f'{k} {c[b][k]/len(ds):5.0f}' for k in ('dig', 'recut', 'out', 'out<=5', 'in', 'pit', 'dug', 'rooms', 'nest', 'home', 'starved')))
