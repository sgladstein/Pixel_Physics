#!/usr/bin/env python3
"""build.py -- data.py's JSON into page.tmpl.html -> index.html."""
import json, os, sys
here = os.path.dirname(os.path.abspath(__file__))
d = json.load(open(os.path.join(here, 'data.json')))
arms = [['all', 'All four fixes'], ['p1', 'Fix 1 alone'], ['p2', 'Fix 2 alone'], ['p3', 'Fix 3 alone'], ['p4', 'Fix 4 alone'],
        ['slice1', 'No fixes (slice 1)'], ['stack', 'The stack'], ['noants', 'Shipped']]
meta = {
 'all': 'Every fix on: the round as reviewed.',
 'p1': 'Fix 1 alone: only ants on the dig job dig; the dig job starts on soil ahead plus a crowd; an ant shut out cuts back in through the door.',
 'p2': 'Fix 2 alone: an empty forager keeps walking; the forage job ends outside when patience runs out; a forager rich enough to lay walks home to lay.',
 'p3': 'Fix 3 alone: a hungry ant keeps the food it picked up at home to eat, instead of putting it back down.',
 'p4': 'Fix 4 alone: an ant starts cutting its way out only after real stalls, not while it rests.',
 'slice1': 'The new ant with none of the fixes (slice 1).',
 'stack': 'The shipped ant with the hunger-first, carry-home and nest-rest switches on.',
 'noants': 'The shipped game, drawn dashed under every other arm.',
}
verdict = sys.argv[1] if len(sys.argv) > 1 else ''
foot = ('Code: branch claude/project-thread-ns0j6p at 067dad72 (each fix a named part, all off by default). Instrument: '
        'examples/deeptrace.rs needs=walk needsat=50000 needsparts=..., RAYON_NUM_THREADS=1, so every run repeats exactly. '
        'Food carried in is trip deliveries (food brought in from outside), never forage_trips. Deep ants are read once '
        'every 1,000 frames; the table gives their mean over 100k-300k.')
html = open(os.path.join(here, 'page.tmpl.html')).read()
for k, v in (('__DATA__', d['data']), ('__GATE__', d['gate']), ('__ARMS__', arms), ('__META__', meta), ('__VERDICT__', verdict), ('__FOOT__', foot)):
    html = html.replace(k, json.dumps(v, separators=(',', ':'), ensure_ascii=False))
os.makedirs(os.path.join(here, 'pub'), exist_ok=True)
open(os.path.join(here, 'pub', 'index.html'), 'w').write(html)
print('ok', len(html))
