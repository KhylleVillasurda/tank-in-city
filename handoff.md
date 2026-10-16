# Handoff

Read this first. It is the state of the project at the end of the session that
added sound and the 8-bit presentation, written for whoever picks it up next —
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
| 3 — rooms and netplay | not started |
| 4 — accounts and leaderboards | not started |

One mode exists (Single Player). The engine holds those rules directly rather
than behind a `Mode` enum; the seam is `World::new` plus `rules.rs`.

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

## How to verify

All five must pass before a change is called done.

```sh
cd game-core && cargo test          # 28 tests: 23 engine, 1 sizes, 4 stage data
cd web && npm run check             # headless wasm ABI and rules
cd web && npm run check-audio       # headless sound triggers
cd web && npx tsc --noEmit          # type check
cd web && npm run build             # production build
```

Then, because code changed:

```sh
graphify update .
```

Current state: 28 tests pass, no compiler warnings, `npm run check` reports
`wasm ABI ok: score=2700 lives=1 stage=1 enemiesLeft=4 idleGameOver=30s`,
`npm run check-audio` reports `audio ok`, `tsc` and `build` are clean, and the
graph was 554 nodes / 816 edges / 26 communities at the last update.

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
- **Everything is uncommitted.** `HEAD` is `27d319c` ("Add Tank In City project
  plan") and every source file is untracked. `graphify-out/` and
  `web/public/game_core.wasm` are generated and ignored.

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

1. **Restart the dev server, then look at the title screen.** The last session's
   presentation work (font, scanlines, animations) is verified in a clean build
   but the user's running server needs a restart to show it. Confirm with the
   user that it looks right.
2. **The six missing sounds.** `docs/deferred.md` lists each one with the event
   that triggers it. Adding one is a file plus a line in `web/game/audio.ts`.
3. **The engine loop is still undecided** — `tank_moving_by_kenny.ogg` is wired,
   `tank_moving.ogg` is a different sound, and only an ear can choose.
4. **Sprites** (Milestone 5) would change the game's look more than anything
   else left on the list.
5. **Commit.** Everything is untracked and has been for two sessions. That is
   the largest single risk in this repository.
