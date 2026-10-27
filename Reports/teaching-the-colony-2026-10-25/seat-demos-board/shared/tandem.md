# Shared piece: tandem runs (tap, follow by touch, pause, break, search, arrival, cascade)

Owner: **scout** seat (thread "Build the lone scout seat demo", branch claude/project-thread-x1ms83).
State: PLANNED 2026-10-25 08:55Z (plan: ../scout/plan-v1.md §3, under review).

## Interface (proposal)
- `World::tandem: Option<Box<Tandems>>`, None = off, byte-identical.
- `tandem::tap(world, leader) -> Accepted(id) | Refused(id, reason) | NoOne`: nearest same-colony adult within 3 cells,
  not paired / nest-bound / carrying; accept chance graded by hunger.
- `tandem::tick(world)`: before world.step; writes `World::mind_force` (enemy's steer-mind piece) for each follower:
  toward the leader in contact, stand while pausing, spiral while lost; releases on arrival (calls keeper's
  `recall::plant` when recall is on) or after the search times out.
- `Tandems::tree` pupil -> teacher; counters: started / done / broken / broken-still-arrived / pupils who led.
- Optional cascade: a taught ant delivering home taps with chance c and leads toward her remembered place.
Uses: enemy's mind_force (followers), keeper's recall head (arrival). Colony mind: the cascade could be a "send ants
WITH a route" alternative to RECRUIT; keeper: "follow" could serve a carried/led ant.
