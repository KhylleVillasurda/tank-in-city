# Milestone 3 — Multiplayer

Record of the third build session: plan section 7, "Milestone 3: Multiplayer".
That milestone is 18 checkboxes, so it is being built in slices.

**Slice 1, done:** the `server/` crate and the room registry. Codes, the public
room list, teams, ready states, host transfer, and the start check — every rule
in plan section 3 that does not need a socket.

**Slice 2, done:** the socket and the lobby protocol. Axum on `/ws`, the lobby
messages of plan section 9, a `room_state` pushed to the room after every change,
and a lobby screen in `web/app/lobby/`.

**Status:** code complete and verified by tests, clippy, the binary on a real
port, and the whole lobby protocol over a real Cloudflare tunnel. **No browser
has opened the lobby screen yet**, and **no match runs**: a room reaches
"playing" and stops there.

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

Still open, in the plan's order: the match loop, win conditions, rounds, lives
and spectating, prediction and interpolation, session tokens and reconnect, and
the two-browser check.

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
happened.

## Layout added

```
server/
├─ Cargo.toml            # game-core by path, plus axum, tokio, serde, tracing
├─ src/
│  ├─ lib.rs             # the crate's front door
│  ├─ main.rs            # binds 127.0.0.1:8080 and serves the router
│  ├─ rooms.rs           # the room registry
│  └─ ws.rs              # the socket: protocol, hub, size and rate limits
└─ tests/
   ├─ rooms.rs           # one test per room rule
   └─ ws.rs              # the lobby over a real socket

web/
├─ app/lobby/page.tsx    # the lobby screen
└─ game/lobby.ts         # the message shapes and where the server is
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
- **Slice 1 does not run `game-core`.** The plan's first Milestone 3 item is
  "server runs `game-core` natively per room", but the engine is still
  single-player: one player tank plus AI. A room cannot run a match until the
  engine holds several players, so the match loop is the next slice and the
  registry is the one before it. `game-core` is already a dependency, used for
  the RNG.

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
  join. A token is only useful with the 30-second reconnect, which needs the
  match loop, so sending one now would be a value nothing can use. The client
  learns the socket is live from the browser's own `open` event instead.
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

## Verification performed

```sh
cd server && cargo test        # 25 room tests, 5 unit tests, 7 socket tests
cd server && cargo clippy --all-targets
cd server && cargo run         # serves /ws and /health on 127.0.0.1:8080
cd web && npm run check-lobby  # drives the protocol against a running server
cd web && npx tsc --noEmit
cd web && npm run build
```

- The 25 room tests are one per rule in plan section 3: code shape and
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
  not exist, and the public list showing public rooms with in-progress ones
  marked and sorted.
- The 5 unit tests in `ws.rs` cover the wire names, the two messages the client
  sends as the plan writes them, the unhandled messages reading as unsupported,
  the room state carrying each player's own side, and the rate limiter.
- The 7 socket tests in `tests/ws.rs` run the server in-process on a free port
  with real WebSocket clients: a full 1v1 from create to start, a room in
  progress refusing a late join, the public list showing a public room and hiding
  a private one, the host leaving and the room closing when the last player does,
  a disconnect removing the player, an unhandled message being refused, and one
  connection being in one room at a time.
- **The binary itself was run**, not just the library: `/health` answered `ok`,
  and `npm run check-lobby` walked create → join → ready → start → leave against
  it and printed `lobby ok`.
- **The same protocol ran over a real Cloudflare quick tunnel**, with
  `LOBBY_URL=wss://…/ws`, and passed the same way. That is the transport
  multiplayer will use, proven before the match loop exists.

### What is not verified here

- **No browser has opened `/lobby`.** The page type-checks and builds, and the
  messages it sends are the ones `check-lobby` sends, but no button has been
  clicked and no roster has been seen on screen. That needs the user.
- **No match.** A room reaches `Playing` and stops there; no `World` is created
  and no ticks run.
- **The named tunnel's `/ws` path rule is untested.** It needs a domain, and
  there is not one. A quick tunnel was used instead, which needs no path rule at
  all. See `docs/sharing.md`.

## Deliberately not done yet

In `docs/deferred.md` under "Server", with the rest of the live list.
