# colsel6-1022 (Teacher lane, 2026-10-22 20:05Z)

64 colonies x 32 rounds, hA1 founder, lexicase over colonies on (home8, -starved, kept late cuts, nest) + extras, protected
lesson c1:0.1, nest-only inheritance. Common garden per 24 ants (fresh world c*1000+99, 200k, sel off), median over colonies.
Control = colsel4 cs4inh (same arm, no extra cases). Old ant: food 716, starved 0, nest 302, kept late 46, crater 4, spoil 0, spoil_in 0, shape 20 (12 worlds).

| arm, round 31 | food | starved | nest | kept late | crater | spoil left | spoil let go below datum | shape |
|---|---|---|---|---|---|---|---|---|
| control (cs4inh) | 1,582 | 0 | 147 | 34 | 100 | 188 | 1,030 | - |
| +spoil | 1,502 | 0 | 160 | 54 | 150 | 18 | 280 | 6 |
| +spoil+crater | 2,133 | 2 | 114 | 32 | 80 | 16 | 93 | 3 |
| +spoil+crater+shape | 2,315 | 3 | 40 | 6 | 49 | 7 | 88 | 2 |

Paired vs control by colony at r31: +spoil spoil left lower 63/0, spoil_in lower 60/3, nest 31 up/32 down, crater WORSE 49/13.
+spoil+crater: crater better 37/25, nest worse 44/18, starved worse 40/10. +shape: nest worse 54/8, shape median 2 every round (max 25.5).

Reading: the spoil case stops the treadmill (control's nest falls 257 -> 147 after r21; +spoil holds ~150-160 to r31).
Crater and shape as extra colony cases did not move their own number much and diluted the dig cases (each case leads
lexicase 1/N of the time): selection has no variation in room-making to pick from. Shape needs a behaviour source first.
