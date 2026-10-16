# Handoff

Read this first. It is the state of the project at the end of the session that
taught the engine to hold several players, written for whoever picks it up next —
human or AI.

Everything here was checked, not remembered. Where a fact lives in another file,
this points at it instead of copying it, so the two cannot drift apart.

## Where the project stands

`Tank In City`, built to `plan.md`. **The plan carries everything** — scope,
rules, milestones, repo layout. Read it before proposing anything.

| Milestone | State |
|---|---|
| 1 — engine and the first playable arena | done, play-tested, closed |
| 2 — Single Player | done, play-tested, closed |
| 5 — sound | **pulled forward and working**, six of twelve files still missing |
| 5 — sprites and the credits page | not started |
| 3 — rooms and netplay | **slices 1, 2 and 3 done** (the lobby, the socket under it, and the engine holding several players); the match loop is next |
| 4 — accounts and leaderboards | not started |

One mode exists (Single Player). What the engine needs to know about any other
mode travels in a `Setup` struct — player count, whether the AI waves run,
whether friendly fire is on, and which side each player is on. Everything else
in plan section 6 (rounds, timers, spectating, match scoring) is the match loop's.

`server/` exists now and listens. It is a second Rust crate beside `game-core`,
with **no Cargo workspace** — a path dependency, so the wasm build script is
untouched. `cd server && cargo run` serves the lobby socket on `/ws` and a
`/health` check on `127.0.0.1:8080`. Nothing runs a match yet.

## Read these, in this order

1. `plan.md` — the source of truth.
2. `docs/deferred.md` — **the live to-do list.** One page, meant to shrink.
3. `docs/milestone-2.md` — the current engine record: rules, tuning knobs, the
   snapshot layout, and the measurements behind the type sizes.
4. `docs/audio.md` — the sound design, the files, and how each sound is
   triggered.
5. `CREDITS.md` — every asset, its author, its source and its licence.
6. `docs/grid-alignment.md` — why turning nudges a tank. Read before touching
   movement.
7. `AGENTS.md` — use the graphify MCP; do not read twenty files to find one
   function.
8. `docs/sprites.md` — the drawing sizes for the artwork. Read before drawing or
   placing any sprite.
9. `docs/milestone-3.md` — the server, the lobby rules, the socket, the engine's
   several-players change, and what each slice left out.
10. `docs/sharing.md` — how to put the game on a public link with cloudflared, so
    friends can open it without a deployment. Read it before testing with
    anyone: it says what friends can and cannot do today.

## What the last session changed

Do not redo these; build on them.

- **Sound, end to end.** `web/game/audio.ts` is new. There is **no engine
  change** behind it: it diffs consecutive snapshots. Called from the frame
  callback in `web/game/engine.ts`, next to the renderer.
- **The stage-clear screen is no longer timed.** It waits for the player to
  press Enter or click, so the 13.7 s jingle is never cut off. That *is* an
  engine change: `INPUT_CONFIRM` (bit `32`) is a new input bit, and
  `STAGE_CLEAR_TICKS` is gone.
- **The audio files were converted.** WAVs to Ogg Vorbis, music to 96 kbps:
  5.9 MB to 1.8 MB. The raw downloads stayed in `assets/` and are gitignored on
  purpose (two of the three licences forbid redistributing the unmodified file).
- **Levels were fixed.** `shoot` was 14 dB below everything else. Peaks now sit
  together.
- **A pixel font and three animations.** Press Start 2P, vendored in
  `web/app/fonts/` with its OFL licence. Scanlines, a blinking prompt, and a
  four-step entrance — all snapping with `steps()`, never easing.
- **The state types were narrowed.** Every field now carries the narrowest type
  that holds its longest value: the `Tank` counters, the `World` counters, and the
  tile indices (`steel_tiles` alone went 128 bytes to 32). `World` went from
  **1,384 bytes to 1,144** at the time; the several-players change has since
  taken it to **1,392**, for the reason in `tests/sizes.rs`. Positions
  deliberately stay `i32` — see `docs/milestone-2.md`, "Why the numbers are the
  width they are".
- **`tests/sizes.rs` is new** and pins those sizes, so a field cannot widen back
  by accident. Raise the budget deliberately when a feature needs the room.
- **A latent bug fixed:** `stage::base_center`'s no-base fallback returned tile
  indices while the real path returned world units.
- **`cargo clippy` is clean in both crates, and is now part of the check.**
  Narrowing `hp` to `u8` turned two `hp <= 0` comparisons into clippy errors
  (`absurd_extreme_comparisons` is deny-by-default), now `hp == 0`. Two smaller
  lints were pre-existing and are also gone: a redundant `as u8` cast in
  `bullet.rs`, an unnecessary `let` in `ai.rs`, and a complex type in
  `tests/engine.rs`.
- **`server/` is new: the room registry, and now the socket on top of it.** A
  second Rust crate (`game-core` by path, plus `axum`, `tokio`, `serde`,
  `tracing`). Codes, the public room list, teams, ready states, host transfer,
  and the start check — every rule in plan section 3 that needs no socket. 25
  tests, one per rule. `docs/milestone-3.md` has the decisions and what each
  slice left out. **It does not run a match yet:** nothing constructs a `World`
  for a room.
- **Sharing over a Cloudflare tunnel.** `web/scripts/share.sh` (also `npm run
  share`) starts a quick tunnel to the web app and refuses to start if nothing
  answers on the port. `deploy/cloudflared.yml` is the named-tunnel config: it
  sends `/ws` to the game server and everything else to the web app, **on one
  hostname**, so the lobby needs no configuration and cannot hit mixed content.
  `docs/sharing.md` documents both.
- **The lobby is on a socket.** `server/src/ws.rs` is new: Axum on `/ws` plus a
  `/health`, the plan section 9 lobby messages, and a `room_state` pushed to the
  room after every change. `server/src/main.rs` now binds `127.0.0.1:8080`
  instead of logging one line and exiting. Two rules from plan section 10 live
  there because they are per-connection: a message size cap and a rate cap.
- **`Registry::start` now takes the requester.** Slice 1 let anyone start
  anyone's match. Section 3 says the host starts it, so that rule moved into the
  registry, where the other room rules and their tests are.
- **The lobby screen exists.** `web/app/lobby/page.tsx` and `web/game/lobby.ts`,
  reached from the menu. It decides nothing: it sends messages and draws the last
  `room_state` it was given.
- **`web/scripts/check-lobby.mjs`** (also `npm run check-lobby`) drives the whole
  protocol against a running server, which is how the socket gets checked without
  a browser. `LOBBY_URL` points it at a tunnel.
- **Verified over a real tunnel:** the whole lobby — create, join, ready, start,
  leave — ran against a Cloudflare quick tunnel over `wss://`. That is the
  transport multiplayer will use, proven before the match loop exists.
- **The engine holds several players.** `World` has `players: [Tank; 6]` with
  per-player `player_lives`, `player_scores`, `player_respawn` and
  `player_spawns`, and `step` takes `&[Input]`, one per player. `MAX_TANKS` went
  5 to 10. `Setup` describes a mode; `World::new(seed)` is `Setup::single()`, so
  every old caller still works.
- **Sides exist.** `Tank` and `Bullet` gained `team` (`TEAM_A` / `TEAM_B`), and
  `Bullet::is_player` is gone. The enemies are side B, the base is side A, and
  `same_side_as` is a plain comparison. Free For All's "friendly fire always on"
  falls out of giving every player a side of its own.
- **The snapshot moved once, on purpose.** The player's fields left the header
  for a `SNAP_PLAYER` record array, so the header is 12 ints now, not 23.
  `SNAP_TANKS_OFFSET` is renamed `SNAP_ENEMIES_OFFSET`. Three constants were
  **appended**. The bullet record's `speed` — which nothing read — became `team`.
- **A kill is credited to the player who landed the last hit.** `Tank` gained
  `last_hit_by`. The run ends when *every* player is out of lives, not the first.
- **Each stage file gained a second `P`**, so Co-op has two spawns. The parser
  collects up to `MAX_PLAYERS` of them.
- **The `Tank` and `World` size budgets were raised**, to 32 and 1392 bytes. Both
  have the reason written next to the number in `game-core/tests/sizes.rs`.
- **Single Player is provably unchanged:** `npm run check` and
  `npm run check-audio` print exactly what they printed before the change.

## How to verify

All of these must pass before a change is called done.

```sh
cd game-core && cargo test          # 37 tests: 31 engine, 1 sizes, 5 stage data
cd game-core && cargo clippy --all-targets   # must be silent
cd server && cargo test             # 37 tests: 25 room rules, 5 socket units, 7 socket
cd server && cargo clippy --all-targets      # must be silent
cd web && npm run check             # headless wasm ABI and rules
cd web && npm run check-audio       # headless sound triggers
cd web && npx tsc --noEmit          # type check
cd web && npm run build             # production build
```

Then, with the server up in another terminal (`cd server && cargo run`), because
the socket cannot be checked any other way:

```sh
cd web && npm run check-lobby       # drives the whole lobby protocol
```

Then, because code changed:

```sh
graphify update .
```

Current state: 37 tests pass in `game-core` and 37 in `server`, no compiler
warnings and clippy is silent in both, `npm run check` reports
`wasm ABI ok: score=2700 lives=1 stage=1 enemiesLeft=4 idleGameOver=30s`,
`npm run check-audio` reports `audio ok`, `npm run check-lobby` reports
`lobby ok`, `tsc` and `build` are clean, and the graph was 847 nodes / 1351 edges
/ 56 communities at the last update.

`score=2700 lives=1` in that line is worth knowing: it is the *same* number the
check printed before the several-players change, from the same seed over the same
6000 ticks. It is the cheapest proof that Single Player still behaves exactly as
it did.

## Traps that will cost you an hour

- **The graphify MCP used to answer from the wrong repository — fixed, and
  verified after a restart.** The cause was a hardcoded `--graph` path to another
  project in `~/.config/opencode/opencode.jsonc`, which pinned that repo as the
  default for every workspace. Removing the flag (2026-10-16) makes the default
  `graphify-out/graph.json` beside whichever project the server was started in.
  Checked in three workspaces and each one returned its own node count, so **no
  `project_path` is needed any more.** Two things that still bite: a project with
  no graph returns a plain "no graph" error rather than a wrong answer, so run
  `graphify update .` first; and the query tool's argument is `question`, not
  `query`.
- **Three lists must stay in step, by position.** `CONSTANTS` in
  `game-core/src/wasm.rs` (with a hard-coded length), `CONSTANTS_FIELDS` in
  `web/game/wasm.ts`, and `FIELDS` in `web/scripts/check-wasm.mjs`. They are read
  by index, so **append, never insert** — inserting renumbers everything after
  it and the symptom is a wrong constant, not an error. There are 48 today.
- **The snapshot header's field order is a fourth positional list**, written out
  three times: `write_snapshot` in `game-core/src/lib.rs`, `readSnapshot` in
  `web/game/wasm.ts`, and the `H` map in `web/scripts/check-wasm.mjs`. It is 12
  ints — `tick state stage enemiesLeft baseAlive playerCount enemyCount
  bulletCount spawnCount dropKind dropX dropY`. The record arrays after it are
  reached through the offset constants, so only the header is hand-indexed.
- **The snapshot is read as an `Int32Array`.** Changing a Rust field's width
  changes the ABI, not just the struct. See the layout table in
  `docs/milestone-3.md` for the current one, and `docs/milestone-2.md` for the
  type-size reasoning.
- **There is no browser or speaker in this environment.** Frontend and audio
  claims here can only be static (`tsc`, `build`, `curl`, the two check
  scripts). Only the user's play reports are human evidence, so ask for one and
  give them a `?seed=` to reproduce with.
- **Do not delete `web/.next` while the user's `next dev` is running.** It
  desyncs that server's cache: the last session did it, and the root layout went
  stale while pages updated, which looks exactly like a broken change. If it
  happens, tell the user to restart `npm run dev`.
- **`web/game/audio.ts` must stay strippable by node.** `scripts/check-audio.mjs`
  imports it directly, and node's built-in TypeScript stripping rejects
  constructor parameter properties, so `Sounds` writes its fields out longhand on
  purpose. Do not "tidy" it back.
- **Audio cannot be judged here.** Verify a trigger with `npm run check-audio`;
  never claim a sound "sounds right".
- **`web/AGENTS.md` is generated by `next dev`, and is committed on purpose.**
  Next 16 writes its own agent-rules block there on every dev run. Deleting it
  only brings it back as an uncommitted change. Leave it.
- **The work is committed.** `main` (renamed from `master`) has six commits: the
  plan, then "Add Milestones 1 and 2: engine, single player, audio,
  presentation", then "Add Milestone 3 slice 1: the server crate and the room
  registry", then the clippy fixes, then the tunnel work, then the lobby socket
  (HEAD). `git log --oneline` has the hashes — this file deliberately does not,
  because amending a commit changes them. There is no remote yet.
  `graphify-out/`, `web/public/game_core.wasm` and the raw audio in `assets/` are
  generated or deliberately excluded and stay ignored.
- **There are two Rust crates and no Cargo workspace.** `game-core/` and
  `server/`, each with its own `Cargo.lock` and `target/`. `server/` reaches
  `game-core` by path. Adding a workspace would move the build target directory
  and touch `web/scripts/build-wasm.sh`, which works — do not add one casually.
- **`server/` does not run a match.** Nothing constructs a `World` for a room, so
  a room reaches `Playing` and stops. The engine *can* hold a match now
  (`World::with_setup` plus `step(&inputs)`), so the missing piece is the loop
  itself, not the simulation.
- **`World::step` takes `&[Input]`, one per player, in slot order.** There is no
  single-input overload. `tc_step` in `wasm.rs` is the one place that still takes
  one input, because the browser only runs Single Player; it wraps it in a
  one-element slice. A match on the server calls `step` directly.
- **A `World` has no mode.** `Setup` carries the four facts the engine needs
  (player count, waves, friendly fire, sides) and nothing else. Do not grow it
  into the mode table — rounds, timers and match scoring are the match loop's, and
  `server/src/rooms.rs` already owns the mode table.
- **`npm run check-lobby` is the one check that needs something running.** Start
  `cd server && cargo run` first. It fails with `could not reach …` when the
  server is down, and that is the check working, not a bug in it.
- **The lobby over a quick tunnel needs two tunnels and a `?server=`.** A quick
  tunnel forwards one port, so the socket needs its own, and its address changes
  every run, so it cannot be baked into the build. `docs/sharing.md` has the
  exact commands; a named tunnel avoids both, because
  `deploy/cloudflared.yml` routes `/ws` on the one hostname.
- **`web/app/lobby/page.tsx` must not read `location` while rendering.** Next
  prerenders it, where `location` does not exist. That is why `serverUrl()` is
  called inside an effect. Reading it at the top of the component breaks the
  build, not just the page.

## Constraints to keep

- **Fixed-point integer maths, a fixed 60 Hz timestep.** `i32` with `SUBPIXELS =
  16` (1/16 px). A tile is 256 units (16 px); a tank is 2x2 tiles (512 units).
- **Give each field the narrowest type that holds its longest value.** That is
  right-sizing, not "make everything `u8`" — `u8`, `u16` or `u32` as the range
  says. `AGENTS.md` states it. Positions are the one deliberate exception, for
  the reason in `docs/milestone-2.md`, and `tests/sizes.rs` is what holds the
  line.
- **`game-core` never touches the network, the DOM, or audio.** It is the
  headless rules engine. Sound and rendering live in `web/` for that reason.
- **No wasm-bindgen.** Plain `cargo build --target wasm32-unknown-unknown`
  plus a hand-written C ABI and direct memory reads from JavaScript.
- **No asset may be ripped from the original Battle City** (plan §1). Not a
  sprite, not a sound, not a "soundfont".
- **No engine number is duplicated in TypeScript.** They are exported from wasm
  as the constants block and read through `CONSTANTS_FIELDS`.
- **Document as you go.** The user asks for this explicitly, so the next visit is
  quick. New design decision → a short note in `docs/`; new asset → a row in
  `CREDITS.md`; new deferred work → `docs/deferred.md`.

## The user

Plain words, short sentences, no jargon, no undefined metaphor. They have said
this directly: tone the jargon down. Use bold and short lists so a long answer is
scannable. Answer the question that was asked first, then the detail.

## Waiting on the user

Three things are blocked on the user, not on code. Do not present them as new
discoveries, and do not re-ask them every turn.

| Waiting on | The blocker | Raise it when |
|---|---|---|
| **Sprite art** | The user is sourcing pixel art. The sizes are already fixed in `docs/sprites.md`, so no decision is needed — only the files. | Work touches rendering, layout or visual polish; a milestone ends; or the user asks what comes next. **Once per session at most.** |
| Six sound files | `docs/deferred.md` lists each one with the event that triggers it. | Work touches sound. |
| The engine loop | `tank_moving_by_kenny.ogg` is wired; `tank_moving.ogg` is a different sound. | The user is play-testing — only their ear can decide. |

## If you are picking up work now

**Start here: `docs/milestone-3.md`, "Slice 4: the match loop — the next
session".** It is a short brief — what the loop has to do, where it plugs in, the
six decisions to settle with a recommendation for each, the one trap in it, and
how to verify it without a browser. Everything it needs already exists.

In the order that pays best:

1. **The match loop.** This is the last thing between the lobby and the
   two-browser check in plan section 7, and the only thing left that makes the
   game multiplayer. One `World` per room (`World::with_setup`), a 60 Hz tick, the
   plan section 9 `input` and `snapshot` messages, and the `map` message on a
   stage change. The socket, the rooms, the lobby screen and an engine that holds
   six players are all in place and waiting for it.
2. **Rounds and win conditions**, right behind the loop: best-of-3, a round ends
   when a base falls or a side is out of lives, lives and scores reset between
   rounds. `docs/deferred.md` under "Engine" and "Server" lists exactly what is
   missing, including the PvP scoring the engine does not do yet.
3. **Two bases and PvP maps.** Team PvP needs a base per side and at least five
   maps to cycle (plan section 6). The stage format has one `#` today. FFA needs
   no base at all, so it can be played before this.
4. **The six missing sounds.** `docs/deferred.md` lists each one with the event
   that triggers it. Adding one is a file plus a line in `web/game/audio.ts`.
5. **The engine loop is still undecided** — `tank_moving_by_kenny.ogg` is wired,
   `tank_moving.ogg` is a different sound, and only an ear can choose.

**Sprites** (Milestone 5) would change the game's look more than anything else
left on the list, but they are waiting on the user's art.

**One thing to ask the user early**, because it blocks the last step of the match
loop: the two-browser check. It needs two browsers, a tunnel and the user's
hands. Ask before reaching it, not after.
