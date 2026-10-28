# Rewrite a copy log in which a mind played (not shadowed) so each decision is labelled with the mind's own pick:
# field 15 (`carry`) becomes `1|pick` (fields: tag,Q,frame,id,x,y,settle,pick,...). settle=2 context rows untouched.
import sys
src, dst = sys.argv[1], sys.argv[2]
n = 0
with open(src) as fi, open(dst, 'w') as fo:
    for line in fi:
        if ',Q,' in line[:24]:
            f = line.rstrip('\n').split(',')
            if f[1] == 'Q' and f[6] != '2' and int(f[7]) >= 0 and len(f) > 15:
                f[15] = '1|' + f[7]
                line = ','.join(f) + '\n'
                n += 1
        fo.write(line)
print(dst, n)
