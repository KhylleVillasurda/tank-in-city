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
- **More 8-bit dressing, if wanted.** Today: scanlines, a blinking `> insert
  coin` prompt, a four-step entrance, and Press Start 2P on every screen. Obvious
  next candidates are a stepped typewriter reveal on the title, a CRT flicker on
  stage clear, and a real attract loop behind the prompt — the words are arcade,
  but nothing plays behind them yet. The rule to keep: snap between states with
  `steps()`, never ease — smooth motion is the one thing an 8-bit machine could
  not do.
- **`shadcn/ui`** is still not installed, and now definitely will not be. The
  lobby's dropdown and checkboxes are site components instead — `Select` and
  `Check` in `web/components/`. The checkbox is a real `<input>` with
  `appearance-none`, so the browser keeps the label, the space bar and the form
  value and only the square is replaced. The dropdown could not be done that way:
  an OS-drawn `<select>` popup cannot be styled at all, so `Select` re-implements
  what a native one gives away — listbox roles, arrows, Enter, Escape, and a
  click outside to dismiss. Add a third one only when a third control needs it.
- **No favicon.** Every page 404s on `/favicon.ico`: the tab shows the default
  glyph and the console logs an error. Needs an icon — an 8x8 tank in the game's
  palette as `app/icon.svg` would match the rest of the dressing.
- **Small text is off the face's 8px grid.** `text-xs` (12px) and `text-sm` (14px)
  are not multiples of 8, so the small type is a little soft next to the crisp
  32px title. 16px would be crisp but ~30% wider and would move every layout, so
  it waits for the user's verdict after their play test rather than being guessed.

## Engine

- **Narrowing positions to `i16`.** The counters and tile indices are done (see
  `docs/milestone-2.md`). `Tank.x/y`, `Bullet.x/y`, `Drop.x/y` and `base_center`
  stay `i32`: they fit in `i16`, but they are what the movement and collision path
  does arithmetic on in the hottest loop in the engine, and wasm has no 16-bit
  registers. Worth revisiting only if the per-room memory in Milestone 3 turns out
  to actually bite — measure the room count first.
- **Two bases, and maps to put them on.** `Setup` can put players on two sides
  and the base rule already reads sides, but a stage has one `#` and one base is
  side A's. Team PvP needs a base per side, a stage format that can declare two,
  and at least five maps (plan section 6). FFA needs no base at all, which
  already works: a map with no `#` has nothing to lose.
- **Match scoring.** The engine credits an enemy kill to the player who landed
  the last hit and nothing else. Plan section 6 also wants 500 for a PvP kill,
  1000 for destroying a base, and **-250 for a team kill, with the victim
  counting a death and the team kill not counting as a kill**. None of that is
  implemented; a player death currently scores nobody. The match loop needs
  per-player kill and death counts, which means the engine has to report them.
- **Spectating.** Plan section 6: an eliminated player watches the rest,
  switching between remaining players' views. Today an out-of-lives player's tank
  simply stays down and the run continues without them. The match loop owns this.
- **An input-replay harness.** The engine is deterministic from `(seed, inputs)`
  and `?seed=` already replays a run, so a recorded input log could replay a run
  *with* your driving, which is what a bug report actually needs. Nobody has
  missed it yet.
- **A per-stage difficulty ramp.** Enemy kinds are picked by a weight curve in
  `tank.rs`; every stage has the same 20 enemies otherwise. Left out because
  difficulty is closed by play test, not because it is hard.

## Server

Milestone 3 slices 1, 2, 3 and 4 (the lobby, the socket under it, the engine
holding several players, and the match loop) are in. These are what they left out
on purpose.

- **The match page.** `/game` runs the wasm engine locally and the lobby screen is
  on a socket, but nothing renders a *server* match. A page that takes the flat
  `snapshot` from the socket and draws it with `render.ts` (which already takes a
  plain `Snapshot` object) is the first job of the next slice. Until it exists,
  the only way to play a match is two socket clients — `npm run check-match`.
- **Rounds.** Plan section 6: best-of-3, a round ends when a base falls or a side
  is out of lives, lives and scores reset between rounds, the round count carries
  over. None of it exists. The loop stops the moment `World::state` becomes
  `GameOver`, which is the end of the whole run.
- **`match_over`, and what a finished match owes its players.** The loop ends and
  puts the room back in the lobby (`Registry::finish`), and the last snapshot
  carries `GameOver`, so a client knows the run ended. It is not told *who won*:
  `GameOver` fires for the base falling and for every player running out of lives
  alike, and the engine does not say which. Naming a winner, and sending
  `match_over { winner, scores }`, belongs with rounds.
- **Every message that still needs a running match.** `reconnect` and
  `request_swap` are refused with an error rather than ignored. `map_event`,
  `player_disconnected`, `player_reconnected` and `player_eliminated` are not sent
  at all.
- **A player who leaves mid-match leaves the room, not the match.** The socket's
  `leave_room` removes them from the room, so the lobby roster shrinks, but the
  running `World` keeps their tank where it was and it stops receiving input — it
  just idles. That is the plan's "disconnected or quitting players keep the
  contribution they had when they left", minus the contribution, and it is what
  spectating and elimination will replace.
- **Kicking.** Plan section 10 gives the host a kick before the match starts. It
  is not in the protocol and not built.
- **A name filter.** Plan section 10 asks for "a basic filter for obvious slurs"
  on names. Today a name is only checked for being non-empty and 16 characters or
  fewer, so anyone can put anything in a lobby. Needs a word list, and a decision
  about which languages.
- **Where the client finds the socket.** A quick tunnel hands out a new address
  every run, so it cannot be hard-coded; and the game page is `https`, so the
  socket has to arrive as `wss://` through a tunnel of its own rather than a plain
  `ws://` (the browser blocks that as mixed content). Today that is the
  `?server=` parameter on the lobby URL, in `docs/sharing.md`. A named tunnel puts
  both on one hostname and needs no parameter at all.
- **The room code is quoted in some log lines and bare in others.** `rooms.rs`
  writes `code=%code` when it creates a room and a bare `code` field everywhere
  else, and the two render differently — `code=VDS97` against `code="VDS97"` — so
  grepping one code does not give every line for it. Plan section 14 asks for
  exactly that. Make them all `%code`.
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
| Rooms, hosting, joining, the match loop, server authority | Milestone 3 (the lobby, its socket, several players in the engine, and the match loop are done; the match page, rounds and prediction are not; see "Server" above) |
| Accounts, email verification, profiles, friends, chat, spectating | Milestone 4 |
| Local and online leaderboards, persistent scores | Milestone 4 |
| Sprites, the credits page | Milestone 5 |
| Five stage maps (three exist) | Milestone 5 |
| Co-op and PvP modes | after Single Player is solid (plan §5) |
