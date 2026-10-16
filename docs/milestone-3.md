# Milestone 3 — Multiplayer

Record of the third build session: plan section 7, "Milestone 3: Multiplayer".
That milestone is 18 checkboxes, so it is being built in slices.

**Slice 1, done:** the `server/` crate and the room registry. Codes, the public
room list, teams, ready states, host transfer, and the start check — every rule
in plan section 3 that does not need a socket.

**Status:** code complete and verified by tests, clippy, and a boot of the
binary. Nothing listens on a socket yet, and no browser has talked to it.

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

## Layout added

```
server/
├─ Cargo.toml            # game-core by path, tracing, tracing-subscriber
├─ src/
│  ├─ lib.rs             # the crate's front door
│  ├─ main.rs            # boots logging and the registry
│  └─ rooms.rs           # the room registry
└─ tests/rooms.rs        # one test per room rule
```

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

## Verification performed

```sh
cd server && cargo test        # 24 tests
cd server && cargo clippy --all-targets
cd server && cargo run         # boots, logs one line, exits
```

- The 24 tests are one per rule in plan section 3: code shape and uniqueness, a
  seeded registry replaying its codes, the host being a player in their own room,
  join by code and a wrong code, a password being required and an empty password
  counting as none, joins refused after the start, names unique inside a room and
  reusable across rooms, names trimmed and bounded, the room filling at the mode
  limit, sides filling smaller-first with ties to A, Co-op on one side, Free For
  All with no sides and friendly fire forced on, the host's friendly-fire choice
  kept in team modes, host transfer, an empty room closing, leaving a room you
  are not in, start needing every player ready, 1v1 needing one a side, team
  modes needing two a side, uneven teams starting, Free For All and Co-op needing
  two, starting a room that does not exist, and the public list showing public
  rooms with in-progress ones marked and sorted.

### What is not verified here

- **No socket, so no client.** Nothing has connected to this. The registry is
  tested directly.
- **No match.** A room reaches `Playing` and stops there; no `World` is created
  and no ticks run.
- **Nobody has seen a lobby screen**, because there is not one yet.

## Deliberately not done yet

In `docs/deferred.md` under "Server", with the rest of the live list.
