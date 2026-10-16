# Deferred

Things that are known, wanted, and not done yet. This is the **live** list: it is
meant to be edited and to shrink. `plan.md` carries the schedule; this file
carries the loose ends that are too small to belong to a milestone and too easy
to forget.

Nothing here is a defect. Everything here is a choice that was made knowingly.

## Sound

Half of the twelve sounds have no file yet. The engine side is finished and
`npm run check-audio` covers it, so adding one of these is a file plus one line.

| Sound | Plays when | Suggested file |
|---|---|---|
| brick hit | a bullet dies on a brick — no kill, no score | `hit-brick.ogg` |
| steel hit | a bullet dies on steel | `hit-steel.ogg` |
| power-up appeared | the drop's kind changes from `-1` (currently silent) | `powerup-appear.ogg` |
| power-up taken | the drop's kind goes back to `-1` while playing (currently silent) | `powerup-pickup.ogg` |
| run over | state becomes 2 (currently silent) | `game-over.ogg` |
| menu press | a button on the title or menu is used | `menu-select.ogg` |

Two notes on the first two. Telling a brick hit from a steel hit means re-reading
the tile map under the bullet, which the snapshot diff deliberately avoids today
(docs/audio.md, "known limits"). And the explosion already covers enemy killed,
you killed and base lost, and the shot already pitches down for enemies, so those
three fifths of the list are done by reuse rather than by new files.

### Undecided, and only an ear can decide

- **Which engine loop.** `tank_moving_by_kenny.ogg` (4 s) is wired, chosen because
  Kenney is CC0 with no extraction clause. `tank_moving.ogg` (0.42 s) is a
  *different sound*, not a duplicate. Swapping `engine:` in `web/game/audio.ts` is
  one line.
- **-ish numbers**, both guesses made without speakers: `MUSIC_GAIN` (now 0.6,
  raised from 0.45 after a play report) and `ENGINE_GAIN` (0.3). Also the balance
  between `shoot.ogg` and `explosion.ogg`, which are both at full peak and may
  want a few dB of daylight between them.

## Presentation

- **Sprites.** Tanks, bricks, steel, water, trees, bullets and the base are still
  drawn as rectangles. Milestone 5. Plan §1: original art only, nothing ripped
  from the original game. **The sizes are already decided and written down in
  `docs/sprites.md`** — draw to that spec so nothing has to guess.
- **The in-game credits page.** Plan §12 wants one, reading from `CREDITS.md`.
  Milestone 5.
- **More 8-bit dressing, if wanted.** Today: scanlines, a blinking prompt, a
  four-step entrance, and Press Start 2P on headings and buttons. Obvious next
  candidates are a stepped typewriter reveal on the title, a CRT flicker on
  stage clear, and an arcade "insert coin" attract loop. The rule to keep: snap
  between states with `steps()`, never ease — smooth motion is the one thing an
  8-bit machine could not do.
- **`shadcn/ui`** is still not installed. The title screen and menu are three
  buttons and a heading; a component library would be more code than the thing it
  replaces.

## Engine

- **Narrowing positions to `i16`.** The counters and tile indices are done (see
  `docs/milestone-2.md`). `Tank.x/y`, `Bullet.x/y`, `Drop.x/y` and `base_center`
  stay `i32`: they fit in `i16`, but they are what the movement and collision path
  does arithmetic on in the hottest loop in the engine, and wasm has no 16-bit
  registers. Worth revisiting only if the per-room memory in Milestone 3 turns out
  to actually bite — measure the room count first.
- **A `Mode` enum.** One mode exists, so the enum would be a variant with no
  second case. The seam is `World::new` plus `rules.rs`. Milestone 3.
- **An input-replay harness.** The engine is deterministic from `(seed, inputs)`
  and `?seed=` already replays a run, so a recorded input log could replay a run
  *with* your driving, which is what a bug report actually needs. Nobody has
  missed it yet.
- **A per-stage difficulty ramp.** Enemy kinds are picked by a weight curve in
  `tank.rs`; every stage has the same 20 enemies otherwise. Left out because
  difficulty is closed by play test, not because it is hard.

## Server

Milestone 3 slice 1 (the lobby) is in. These are what it left out on purpose.

- **The match loop.** A room reaches `Playing` and stops. The engine is still
  single-player — one player tank plus AI — so a room cannot run a match until
  `game-core` holds several players. That engine change is the next slice, and
  the match loop follows it. This is why slice 1 has no `World` in it.
- **The socket.** No HTTP, no WebSocket, no Axum yet. The registry is a plain
  library, so the protocol layer plugs into it without rework.
- **Where the client finds the socket.** A quick tunnel hands out a new address
  every run, so it cannot be hard-coded; and the game page is `https`, so the
  socket has to arrive as `wss://` through a tunnel of its own rather than a plain
  `ws://` (the browser blocks that as mixed content). `docs/sharing.md`, "When
  multiplayer arrives", lists the options. A WebSocket was tested through a quick
  tunnel and it works, so this is a wiring decision, not a blocker.
- **Session tokens and reconnect.** Plan section 5 issues a token on join and
  allows a 30-second reconnect. The token is only useful with reconnect, and
  reconnect needs the match loop, so both wait. `Player` has no identity across
  a disconnect yet.
- **Swap requests and host-move.** The plan has players requesting a swap that
  another player accepts, and the host moving players directly. Both change team
  assignment, which is the one thing that would make the per-side maximum in
  `Room::can_start` a real check again — today joining fills the smaller side
  first, so no side can exceed its share. Add the check with the feature.
- **The room password is plain text in memory.** It is a lobby password, not an
  account credential, and the client sends it in the clear on every join, so
  hashing it would protect nothing that is not already exposed. Revisit when
  Milestone 4 brings real accounts and a reason to hold a secret.
- **A room code is guessable.** 33 million codes, and a private room also has a
  password. Fine for a game lobby; not an access token.

## Later milestones

Scheduled in `plan.md`, listed here only so this file is the one place to look.

| Skipped | Arrives in |
|---|---|
| Rooms, hosting, joining, the match loop, client prediction, server authority | Milestone 3 (the lobby is done; see "Server" above) |
| Accounts, email verification, profiles, friends, chat, spectating | Milestone 4 |
| Local and online leaderboards, persistent scores | Milestone 4 |
| Sprites, the credits page | Milestone 5 |
| Five stage maps (three exist) | Milestone 5 |
| Co-op and PvP modes | after Single Player is solid (plan §5) |
