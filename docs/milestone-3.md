# Milestone 3 — Multiplayer

Record of the third build session: plan section 7, "Milestone 3: Multiplayer".
That milestone is 18 checkboxes, so it is being built in slices.

**Slice 1, done:** the `server/` crate and the room registry. Codes, the public
room list, teams, ready states, host transfer, and the start check — every rule
in plan section 3 that does not need a socket.

**Slice 2, done:** the socket and the lobby protocol. Axum on `/ws`, the lobby
messages of plan section 9, a `room_state` pushed to the room after every change,
and a lobby screen in `web/app/lobby/`.

**Slice 3, done:** the engine holds several players. Sides, per-player input,
lives, score and spawn point, a `Setup` that describes a mode, and the snapshot
layout that carries them. Single Player behaves exactly as it did.

**Slice 4, done:** the match loop. One `World` per room, ticked at 60 Hz, with
`input` in and `map` and `snapshot` out. A room that starts a match now plays it.

**Status:** code complete and verified by tests, clippy, the binary on a real
port, the whole lobby protocol over a real Cloudflare tunnel, and a full 1v1
match driven by two socket clients. **No browser has opened the lobby screen
yet**, and **no browser has played a match**: the match page that renders server
snapshots is the next slice's first job.

## The checklist, and where each item landed

Plan section 7 lists the Milestone 3 items. Slice 1 covers these:

| Plan item | Where |
|---|---|
| Create room (mode, friendly fire, public or private, optional password) | `server/src/rooms.rs` (`Registry::create`) |
| Public room list; join by code | `Registry::list_public`, `Registry::join` |
| Reject joins once a match has started | `Registry::try_join`, `Status` |
| Lobby with teams, ready states, host controls | `Room`, `Player`, `Registry::set_ready` |
| Start conditions: all players ready, at least the mode's minimum per side | `Room::can_start` |
| Team modes (1v1, 2v2, 3v3), uneven starts allowed | `Mode`, `Room::next_team` |
| Free For All and Co-op vs AI | `Mode`, `Room::next_team`, `Room::can_start` |
| Host passes to next player when host leaves; empty rooms close | `Registry::leave` |
| Basic structured logging (`tracing`) | `server/src/rooms.rs`, `server/src/main.rs` |

Slice 4 covers one more item from the plan's list: **server runs `game-core`
natively per room** — `server/src/matches.rs`.

Still open, in the plan's order: win conditions and rounds, lives and spectating,
prediction and interpolation, session tokens and reconnect, the client page that
renders a server match, and the two-browser check.

### Slice 2: the socket

Slice 2 adds no new row to that table, on purpose. Every item left needs a
running match, and the engine still holds one player. What slice 2 does is put
the lobby on a socket, so the rows above are reachable from a browser and the
match loop has somewhere to plug in.

The messages, and what happens to each:

| Plan section 9 message | Slice 2 |
|---|---|
| `create_room { mode, friendly_fire, public, password? }` | handled — plus `name`, see below |
| `join_room { code, password? }` | handled — plus `name` |
| `list_rooms { }` | handled |
| `set_ready { ready }` | handled |
| `leave_room { }` | handled |
| `start_match { }` | **added** — the plan needs it but does not list it |
| `reconnect { code, session_token }` | refused with an error; needs the match loop |
| `request_swap { }` | refused with an error; needs the swap feature |
| `input { move, fire, tick }` | refused with an error; needs the match loop |
| `session { token }` | **not sent** — a token is only useful with reconnect |
| `room_list { rooms }` | sent |
| `room_state { … }` | sent, without the plan's `teams` field (see below) |
| `error { message }` | sent |
| `map`, `snapshot`, `map_event`, `player_*`, `match_over` | not sent; all need the match loop |

Every message the plan lists but this slice cannot answer is **refused with an
error rather than ignored**, so a client is never left wondering why nothing
happened. Slice 4 changed three rows of this table:

| Plan section 9 message | Slice 4 |
|---|---|
| `input { move, fire, tick }` | handled — `move` and `fire` become the engine's input bits; `tick` is ignored |
| `map { tiles }` | sent when a match starts and on every stage change |
| `snapshot { … }` | sent at 20 Hz as the engine's flat state buffer, in a `data` field |
| `map_event`, `player_*`, `match_over` | still not sent; they need rounds and reconnect |

`reconnect` and `request_swap` are the two messages still refused.

## Layout added

```
server/
├─ Cargo.toml            # game-core by path, plus axum, tokio, serde, tracing
├─ src/
│  ├─ lib.rs             # the crate's front door
│  ├─ main.rs            # binds 127.0.0.1:8080 and serves the router
│  ├─ rooms.rs           # the room registry
│  ├─ matches.rs         # the match loop: one World per room, 60 Hz
│  └─ ws.rs              # the socket: protocol, hub, size and rate limits
└─ tests/
   ├─ rooms.rs           # one test per room rule
   └─ ws.rs              # the lobby, and a match, over a real socket

web/
├─ app/lobby/page.tsx    # the lobby screen
├─ game/lobby.ts         # the message shapes and where the server is
└─ scripts/check-match.mjs  # a real match driven by two socket clients
```

Plan section 13 draws `server/src/ws/` and `server/src/rooms/` as directories.
Slice 1 made `rooms.rs` a file and slice 2 follows it, so the two match. Split
them when either grows a second file.

## The mode table, as code

Plan section 3 gives the modes. This is the same table in `Mode`:

| Mode | Max players | Sides | AI |
|---|---|---|---|
| `OneVOne` | 2 | yes | no |
| `TwoVTwo` | 4 | yes | no |
| `ThreeVThree` | 6 | yes | no |
| `FreeForAll` | 6 | no | no |
| `Coop` | 2 | one side | yes |

## Decisions worth knowing

- **No Cargo workspace.** `server/` takes `game-core` as a path dependency. A
  workspace would move the build target directory and touch the wasm build
  script, which works today. Two crates and two lockfiles is the smaller change.
- **Codes come from `game-core`'s seeded RNG.** No new dependency, and a seeded
  registry replays the same codes, which is what makes the tests deterministic.
- **The code alphabet is 32 symbols.** `I` and `O` are dropped because they read
  as `1` and `0`. 32 symbols over 5 characters is about 33 million codes, and 32
  is a power of two so picking a symbol is one `below(32)`.
- **The host is `players[0]`.** The plan says the host passes to "the next player
  who joined", which is a removal and nothing else. `players` is private so the
  order cannot be broken from outside.
- **Joining fills the smaller side, ties to A.** That also enforces the per-side
  maximum: a side cannot pass half the mode's count, because the other side is
  filled first. `can_start` therefore checks only the per-side *minimum*.
- **A room password is held in memory in plain text and compared exactly.** It
  is a room password, not an account credential, and the client sends it in the
  clear on each join. It is never logged. See `docs/deferred.md`.
- **Slice 1 does not run `game-core`** (slice 4 does). The plan's first Milestone 3
  item is "server runs `game-core` natively per room", but the engine was still
  single-player then: one player tank plus AI. Slice 3 made it hold several
  players and slice 4 gave it a loop to run in. Up to slice 1, `game-core` was
  only a dependency for the RNG.

### Slice 2 decisions

- **The name travels with `create_room` and `join_room`.** Plan section 9's
  message list has no name in either, but section 3 says a player's name is their
  username or a generated guest name, and `web/game/guest.ts` already generates
  one and shows it on the menu. There are no accounts until Milestone 4, so the
  client is the only place that name exists. One field on two messages beats a
  handshake — and the name has to be remembered per connection anyway, because a
  disconnect has to know which player to remove.
- **`start_match` is added.** The plan's protocol has no start message, but
  section 3 says the host starts the match. Without it a lobby could never start.
- **The host check lives in the registry, not the socket.** `Registry::start` now
  takes the requester and answers `StartError::NotHost`. Section 3 makes "the
  host starts" a room rule, and the room rules are the registry's — which is
  also where the tests are. This closed a real gap: slice 1's `start` let anyone
  start anyone's match.
- **`room_state` has no `teams` field.** The plan lists `players` and `teams`
  separately. Every player already carries their own `team`, so `teams` would be
  the same fact twice. Dropping it also saves the client a copy of the mode
  table: it groups by whatever sides are present, which is right for team modes,
  Free For All and Co-op without knowing which is which.
- **No `session` message yet.** Plan section 9 sends `session { token }` once on
  join. A token is only useful with the 30-second reconnect, so sending one now
  would be a value nothing can use — that is still true with the match loop in.
  The client learns the socket is live from the browser's own `open` event
  instead.
- **The two locks are never held at the same time.** The registry and the client
  map are separate mutexes, and every access goes through a helper that takes and
  releases one. That removes the lock-order question, and nothing is held across
  an `.await`.
- **A leaver hears nothing back.** `leave_room` broadcasts to the room, and the
  leaver is no longer in it. The client clears its own state, which is right
  whether the leave worked or was refused.
- **Two rules from plan section 10 are per-connection, so they live in `ws.rs`.**
  A message size cap (8 KiB acted on, 64 KiB read as a backstop) and a
  messages-per-second cap (120, well above the 60 Hz input the match loop will
  send). Both are logged when they trip.
- **A disconnect is a leave.** The socket closing removes the player and
  broadcasts the new roster. The plan's 30-second grace period needs the session
  token and the match loop, so it waits.
- **The lobby screen decides nothing.** `web/app/lobby/page.tsx` sends messages
  and draws whatever `room_state` it was last given. The sides, the start rules
  and the host are all the server's.

### Slice 3: the engine holds several players

The engine had one player tank and one spawn point, both hard-wired. Everything
else in the plan's section 6 is written in the plural — 2v2, Co-op, "all of a
team's players are out of lives" — so this slice is the one that makes the
simulation able to represent those, and nothing more. No match loop, no rounds,
no timers.

**What changed in `game-core`:**

- **Sides exist.** `Tank` gained `team` (`TEAM_A`, `TEAM_B`), and `Bullet`
  gained the firing tank's `team`. `Bullet::same_side_as` is now a plain
  comparison, and `Bullet::is_player` is gone — an id is no longer a side. The
  base belongs to side A, so the rule "only a shot from the other side brings the
  base down" became `team != TEAM_A`, which is the same behaviour in Single
  Player and the right rule in a match.
- **`World` holds `MAX_PLAYERS = 6` player tanks.** Each slot carries its own
  run state in parallel arrays: `player_lives`, `player_scores`, `player_respawn`
  and `player_spawns`. `MAX_TANKS` is now `MAX_PLAYERS + MAX_ENEMIES` = 10, so
  the `all` and `rects` machinery is untouched apart from the offsets.
- **`step` takes `&[Input]`, one per player**, in slot order. A player with no
  entry gets no input. Only the stage-clear confirm is read from all of them:
  either player can leave a cleared stage.
- **`Setup` says what the engine needs to know about a mode**: how many players,
  whether the AI wave system runs, whether friendly fire is on, and which side
  each player is on. `World::new(seed)` is `Setup::single()`, so every existing
  caller is unchanged. Everything else in section 6 — rounds, best-of-3, the time
  limit, sudden death, spectating, scoring — is the match loop's, not the
  simulation's.
- **A kill is credited to the player who landed the last hit.** `Tank` gained
  `last_hit_by`, set where a bullet damages a tank. A tank nobody shot (a
  `DestroyAll`, or a test setting `hp` to 0) falls back to player 0, which is the
  only player in Single Player. The run ends when *every* player is out of lives.
- **A stage declares one `P` per player**, up to `MAX_PLAYERS`, in player order.
  All three stage files gained a second `P` to the right of the base, so Co-op
  has somewhere to put its second tank. A stage that declares fewer than the match
  has players puts the extras on the last declared spawn.
- **Enemy ids start at `MAX_PLAYERS`** and wrap back there, so an enemy id can
  never collide with a player's. A player's id is its slot.
- **`MAX_BULLETS` stays 12**: six fully-upgraded players, two each.

**The snapshot, and why it moved once.** The player's fields used to sit inline in
the header. They had to move into a record array to hold more than one player, so
the whole ABI moved in one commit rather than twice. The header is now 12 ints
(no `score`, no `lives`, plus `playerCount`), followed by a `SNAP_PLAYER` record
per player, then enemies, bullets and spawn points:

```
header   tick state stage enemiesLeft baseAlive playerCount enemyCount
         bulletCount spawnCount dropKind dropX dropY
player   x y dir alive hp upgrade shield speed frozen invuln team lives score
enemy    x y dir alive hp id kind carries
bullet   x y dir team owner
spawn    x y flash
```

`SNAP_TANKS_OFFSET` is renamed `SNAP_ENEMIES_OFFSET`, because the tanks are now
two sections and the old name would mislead. Three constants are **appended** to
`CONSTANTS` (`snapPlayer`, `snapPlayersOffset`, `maxPlayers`), keeping the
positional contract with `CONSTANTS_FIELDS` in `web/game/wasm.ts` and `FIELDS` in
`web/scripts/check-wasm.mjs`. The bullet record's `speed` — which nothing in the
client ever read — was replaced by `team`, so `SNAP_BULLET` stayed at 5.

**What the client does with it:** `readSnapshot` builds a `players` array, the
renderer draws all of them, and the HUD and the audio triggers read player 0. The
browser still runs exactly one player, because a match runs on the server; the
wasm `tc_step` still takes one input and passes it as a one-element slice.

### Slice 3 decisions

- **`team` went in now, even though no mode uses a second player side yet.** It
  is used immediately (the enemies are side B, and the base rule is written in
  terms of sides), and adding it later would have meant a second snapshot change.
  It costs 4 bytes of padding per tank.
- **`friendly_fire` and `waves` went in now** for the same reason: they are the
  two mode facts a match needs from the engine, and both are one byte. Free For
  All's "friendly fire always on" needs no flag at all — giving every player a
  side of their own makes `same_side_as` false by construction, which is the
  honest way to say it.
- **The `Tank` size budget went 28 → 32.** The side byte and the last-hit-by byte
  overflowed the 28-byte layout, so the struct picked up 4 bytes of padding. Ten
  tanks on a map is 40 bytes; packing flags into a bitfield to save it would cost
  far more in readability than it saves in a 1392-byte struct.
- **`World` went 1144 → 1392 bytes.** That is six player tanks and six sets of
  run state. One room owns one `World`.
- **The extra-life threshold is per player.** In Co-op each player earns their
  own extra lives off their own score, as the two-player arcade original does. A
  shared threshold would have fired once per 5000 points *total*, which is wrong.
- **Positions stay `i32`.** Only the new per-player counters are narrow. The
  movement and collision loop is the hot path and wasm has no 16-bit registers.
- **The stage-clear condition now also requires `waves`.** Without it a PvP world
  — no AI, so zero enemies from the first tick — would have cleared its stage
  instantly. This was caught by the new `a_match_without_the_ai_never_spawns_a_wave_or_clears_a_stage` test.
- **PvP still cannot be played.** It needs two bases per side and maps to put them
  on; the stage format has one `#`. Slice 4 gave it a loop to run in, so FFA — no
  base at all — is the mode that works first.

## Verification performed

```sh
cd game-core && cargo test     # 31 engine, 5 stage, 1 size test
cd game-core && cargo clippy --all-targets
cd server && cargo test        # 26 room tests, 10 unit tests, 8 socket tests
cd server && cargo clippy --all-targets
cd server && cargo run         # serves /ws and /health on 127.0.0.1:8080
cd web && npm run check-lobby  # drives the protocol against a running server
cd web && npm run check-match  # drives a whole match against a running server
cd web && npm run check        # the wasm ABI, headless
cd web && npm run check-audio  # the sound triggers, headless
cd web && npx tsc --noEmit
cd web && npm run build
```

**Slice 4's tests** are in three places. Four unit tests in `matches.rs` cover
`setup_for`: a 1v1 becomes two sides and no waves, Co-op becomes two players on
one side with waves, Free For All gives every player a side of its own (and none
of them the base's or the enemies'), and the room's friendly fire — forced on in
FFA — reaches the engine. One new room test covers `Registry::finish`, and the
socket test `a_started_match_sends_the_map_and_snapshots_and_moves_a_player`
drives a real 1v1 over a socket: a full `MAP_W * MAP_H` tile map with brick and a
base, snapshots of exactly `SNAP_LEN` ints to both players, and only the tank
whose player sent `input` moving. `check-match.mjs` is the same claim made from
outside the process.

**Slice 3's engine tests** (8 new, in `game-core/tests/engine.rs`): a Co-op pair
starting on two spawns and each driving on its own input, players being solid to
each other, a player out of lives staying down while the other plays on and the
run ending only when both are out, a kill being credited to the player who landed
the last hit, friendly fire on and off deciding whether a shot hurts its own
side, Free For All giving every player a side of its own, a match without the AI
never spawning a wave or clearing a stage, and player and enemy ids never
colliding over a 6000-tick run. Plus one new stage test: every stage can seat a
Co-op pair, on two different spawns.

**The strongest evidence that Single Player is untouched** is that
`npm run check` prints the same line it did before the change —
`score=2700 lives=1 stage=1 enemiesLeft=4 idleGameOver=30s statesSeen=0,2` — and
`npm run check-audio` prints the same line it did before. The scripted run scores
the same 2700 points over the same 6000 ticks, from the same seed.

- The 26 room tests are one per rule in plan section 3: code shape and
  uniqueness, a seeded registry replaying its codes, the host being a player in
  their own room, join by code and a wrong code, a password being required and an
  empty password counting as none, joins refused after the start, names unique
  inside a room and reusable across rooms, names trimmed and bounded, the room
  filling at the mode limit, sides filling smaller-first with ties to A, Co-op on
  one side, Free For All with no sides and friendly fire forced on, the host's
  friendly-fire choice kept in team modes, host transfer, an empty room closing,
  leaving a room you are not in, start needing every player ready, **only the
  host starting**, 1v1 needing one a side, team modes needing two a side, uneven
  teams starting, Free For All and Co-op needing two, starting a room that does
  not exist, a finished match putting the room back in the lobby, and the public
  list showing public rooms with in-progress ones marked and sorted.
- The 10 unit tests cover the wire names, the three messages the client sends as
  the plan writes them, the messages still to come reading as unsupported, the
  room state carrying each player's own side, the rate limiter, and the four
  `setup_for` cases above.
- The 8 socket tests in `tests/ws.rs` run the server in-process on a free port
  with real WebSocket clients: a full 1v1 from create to start, **the match that
  then ticks**, a room in progress refusing a late join, the public list showing a
  public room and hiding a private one, the host leaving and the room closing when
  the last player does, a disconnect removing the player, an unhandled message
  being refused, and one connection being in one room at a time.
- **The binary itself was run**, not just the library: `/health` answered `ok`,
  `npm run check-lobby` walked create → join → ready → start → leave against it
  and printed `lobby ok`, and `npm run check-match` then played a whole 1v1 and
  printed `input moved the host's tank: x 1536 -> 1572`.
- **The same lobby protocol ran over a real Cloudflare quick tunnel**, with
  `LOBBY_URL=wss://…/ws`, and passed the same way. That is the transport the
  match uses, proven before the match loop existed.

### What is not verified here

- **No browser has opened `/lobby`.** The page type-checks and builds, and the
  messages it sends are the ones `check-lobby` sends, but no button has been
  clicked and no roster has been seen on screen. That needs the user.
- **No browser has played a match.** The loop, the `map` and the snapshots are
  proven over a socket, but the page that renders a server snapshot does not exist
  yet, so nothing draws one.
- **Nobody has seen two tanks on screen at once.** The renderer draws every
  player in the snapshot, and both the Co-op engine tests and the socket match
  prove two tanks move independently, but the only run the browser starts is
  Single Player. Two tanks side by side on screen waits on the match page.
- **The match loop has never reached `GameOver` on the server.** A real 1v1 takes
  minutes to run out of lives, so no test or check drives one to the end; the
  `GameOver` exit and its final snapshot are reasoned about, not observed.
- **The second `P` on each stage is unchecked by eye.** The stage tests prove the
  two spawns are on empty tiles, distinct, and reachable from each other; whether
  the spot *looks* right next to the base is a matter for the eye.
- **The named tunnel's `/ws` path rule is untested.** It needs a domain, and
  there is not one. A quick tunnel was used instead, which needs no path rule at
  all. See `docs/sharing.md`.

## Deliberately not done yet

In `docs/deferred.md` under "Server", with the rest of the live list.

## Slice 4: the match loop

One `World` per room, ticked at 60 Hz, taking each player's input and pushing the
resulting state back out. `server/src/matches.rs` is the whole of it; the socket
layer gained `input`, `map` and `snapshot`, and nothing else moved.

**How it works.** `start_match` reads the room once — the roster's names in slot
order and the `Setup` the mode maps to — releases the registry lock, and spawns a
task. The task owns the `World` and loops on a 60 Hz `tokio::time::interval`,
sending the flat snapshot every third tick (20 Hz) and the map when `World::stage`
changes. Input arrives as a message on a channel the Hub holds per room; the loop
keeps each player's latest one and reuses it every tick, so a dropped or late
message is harmless (plan section 5, "Missing input").

**Where each decision the brief left open landed:**

- **One task per room**, not one loop over all rooms. A few rooms is nothing, and
  a room's failure cannot stall another. `MissedTickBehavior::Delay` on the
  ticker: a stalled server skips the backlog rather than running it as fast as it
  can, which would spiral instead of catch up.
- **20 Hz snapshots.** `SNAPSHOT_HZ` is 20 and `TICKS_PER_SNAPSHOT` is `60/20`, so
  the snapshot rate stays a divisor of the tick rate and the loop needs no second
  clock. The plan says 15; 20 is what the brief suggested and it is comfortably
  under the 120 messages/s cap.
- **`serde_json` on the flat `[i32; SNAP_LEN]` array**, in a `snapshot { data }`
  message. No binary frame and no named fields: the offsets are already fixed in
  `game-core`, and the loop should not reshape state it does not own. A binary
  frame is a later measurement, not a guess.
- **`input { move, fire, tick }`.** `move` is the direction bits and `fire` is
  separate, so the engine's input is `(move & MOVE_MASK) | (fire ? INPUT_FIRE :
  0)` — masked, not trusted (plan section 10, "Input validation"). `tick` is
  deliberately not even declared: the loop repeats the last input, so a tick
  number has nothing to decide, and serde drops a field it does not know.
- **`setup_for(room)` maps a mode to `Setup`:** `waves` is `Mode::has_ai()`
  (Co-op only), `friendly_fire` comes from the room, the player count from the
  roster, and `teams` from each `Player`'s own side. Free For All's
  `Team::None` becomes `2 + slot`, a side of its own above the two real sides, so
  no player shares the base's side or the enemies'.
- **The client's second path is not built here.** A match page still needs to
  render *server* snapshots with `render.ts`, and there is no browser to check it
  in. It is the first job of the next slice.

**The trap, and what was done about it.** `World.state == GameOver` says the run
is over but not *why*: the base falling and every player running out of lives both
land there, and in a no-waves match nothing else can happen. Rather than guess a
winner, the loop stops and marks the run over with one last snapshot — the client
sees `GameOver` and knows the run ended, without being told who won. Reading
`base_alive` and the lives to name a winner is the rounds slice's job, where a
round outcome is a real thing.

**One thing this slice had to fix that the brief did not mention: a room could
not be played twice.** A finished match left its room in `Playing`, so nobody
could join and no new match could start. `Registry::finish` puts it back in
`Waiting` and the Hub re-broadcasts the lobby state. Final scores and a rematch
are the rounds slice's.

**How it was verified without a browser.** `web/scripts/check-match.mjs` (also
`npm run check-match`): two WebSocket clients, a real room, a real match — the map
arrives full and with brick and a base, snapshots arrive to both players, and when
only the host sends `input`, the host's tank moves and the guest's does not. It
reads the snapshot offsets from the wasm, so no engine number is copied into the
script. That is the local stand-in for plan section 7's two-browser check, and it
is how the socket work was proven in slice 2. **Do not call the match done on the
strength of unit tests alone** — the two-browser check needs the user.

**Then, in order:** the client page that renders a server match (the brief's
decision 6), then rounds and win conditions (best-of-3, lives and scores reset
between rounds, the round count carries over), then PvP scoring (500 a kill, 1000
for a base, -250 for a team kill), then two bases and PvP maps, then spectating.
`docs/deferred.md` lists each one with what is missing.
