# Handoff

Read this first. It is the state of the project at the end of the session that
added Cloudflare tunnel sharing, written for whoever picks it up next — human or
AI.

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
| 3 — rooms and netplay | **slice 1 done** (the lobby); the match loop and the socket are next |
| 4 — accounts and leaderboards | not started |

One mode exists (Single Player). The engine holds those rules directly rather
than behind a `Mode` enum; the seam is `World::new` plus `rules.rs`.

`server/` exists now. It is a second Rust crate beside `game-core`, with **no
Cargo workspace** — a path dependency, so the wasm build script is untouched.

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
9. `docs/milestone-3.md` — the server and the lobby rules, plus what slice 1 left
   out.
10. `docs/sharing.md` — how to put the game on a public link with cloudflared, so
    friends can open it without a deployment.

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
  tile indices (`steel_tiles` alone went 128 bytes to 32). `World` is
  **1,144 bytes, was 1,384**. Positions deliberately stay `i32` — see
  `docs/milestone-2.md`, "Why the numbers are the width they are".
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
- **`server/` is new: the room registry.** A second Rust crate (`game-core` by
  path, plus `tracing`). Codes, the public room list, teams, ready states, host
  transfer, and the start check — every rule in plan section 3 that needs no
  socket. 24 tests, one per rule. `docs/milestone-3.md` has the decisions and
  what slice 1 left out. **It does not run a match yet:** the engine is still
  single-player, so the match loop needs the engine to hold several players
  first.
- **Sharing over a Cloudflare tunnel.** `web/scripts/share.sh` (also `npm run
  share`) starts a quick tunnel to the web app and refuses to start if nothing
  answers on the port. `deploy/cloudflared.yml` is the named-tunnel config, with
  an ingress rule for the game server ready for slice 2. `docs/sharing.md`
  documents both. **Verified end to end:** the public link served the page, the
  wasm, the audio and the font, and **a WebSocket round-tripped through a quick
  tunnel** — which is the transport multiplayer will need, and it needs no
  account and no domain.

## How to verify

All of these must pass before a change is called done.

```sh
cd game-core && cargo test          # 28 tests: 23 engine, 1 sizes, 4 stage data
cd game-core && cargo clippy --all-targets   # must be silent
cd server && cargo test             # 24 tests: the room rules
cd server && cargo clippy --all-targets      # must be silent
cd web && npm run check             # headless wasm ABI and rules
cd web && npm run check-audio       # headless sound triggers
cd web && npx tsc --noEmit          # type check
cd web && npm run build             # production build
```

Then, because code changed:

```sh
graphify update .
```

Current state: 28 tests pass in `game-core` and 24 in `server`, no compiler
warnings and clippy is silent in both, `npm run check` reports
`wasm ABI ok: score=2700 lives=1 stage=1 enemiesLeft=4 idleGameOver=30s`,
`npm run check-audio` reports `audio ok`, `tsc` and `build` are clean, and the
graph was 683 nodes / 1013 edges / 44 communities at the last update.

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
  it and the symptom is a wrong constant, not an error. There are 45 today.
- **The snapshot is read as an `Int32Array`.** Changing a Rust field's width
  changes the ABI, not just the struct. See the layout table in
  `docs/milestone-2.md`.
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
- **The work is committed.** `main` (renamed from `master`) has five commits: the
  plan, then "Add Milestones 1 and 2: engine, single player, audio,
  presentation", then "Add Milestone 3 slice 1: the server crate and the room
  registry", then the clippy fixes, then the tunnel work (HEAD).
  `git log --oneline` has the hashes — this file deliberately does not, because
  amending a commit changes them. There is no remote yet.
  `graphify-out/`, `web/public/game_core.wasm` and the raw audio in `assets/` are
  generated or deliberately excluded and stay ignored.
- **There are two Rust crates and no Cargo workspace.** `game-core/` and
  `server/`, each with its own `Cargo.lock` and `target/`. `server/` reaches
  `game-core` by path. Adding a workspace would move the build target directory
  and touch `web/scripts/build-wasm.sh`, which works — do not add one casually.
- **`server/` does not run a match.** The engine is single-player, so a room
  reaches `Playing` and stops. Do not wire a `World` into a room before the
  engine can hold several players.

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

In the order that pays best:

1. **Milestone 3 slice 2: the WebSocket layer and the lobby protocol.** Axum
   plus `server/src/ws/`, speaking the messages in plan section 9 that the
   registry already answers: `create_room`, `join_room`, `list_rooms`,
   `set_ready`, `leave_room`, `room_state`, `room_list`, `error`. **No engine
   change**, so it cannot disturb the movement path — and it is the first slice
   the user could actually see in a browser.
2. **Milestone 3 slice 3: the engine holds several players.** `World` has one
   `player` tank and an `enemies` array today. PvP needs N player tanks and no
   AI (team modes never have AI), which is what the match loop waits on. This is
   the delicate slice: it changes the snapshot layout, so all three constant
   lists and `web/game/wasm.ts` move with it.
3. **The six missing sounds.** `docs/deferred.md` lists each one with the event
   that triggers it. Adding one is a file plus a line in `web/game/audio.ts`.
4. **The engine loop is still undecided** — `tank_moving_by_kenny.ogg` is wired,
   `tank_moving.ogg` is a different sound, and only an ear can choose.
5. **Sprites** (Milestone 5) would change the game's look more than anything
   else left on the list, but they are waiting on the user's art.
