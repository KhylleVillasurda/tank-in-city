# Handoff

Read this first. It is the state of the project at the end of the session that
taught the server to run a match, written for whoever picks it up next — human or
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
| 3 — rooms and netplay | **slices 1, 2, 3 and 4 done** (the lobby, the socket under it, the engine holding several players, and the match loop); the match page a browser can watch is next |
| 4 — accounts and leaderboards | not started |

One mode is playable in a browser (Single Player). What the engine needs to know
about any other mode travels in a `Setup` struct — player count, whether the AI
waves run, whether friendly fire is on, and which side each player is on; the
mode table is the server's, and `matches::setup_for` is the one place the two are
translated. A server room can run a Co-op, FFA or team match today. Everything
else in plan section 6 (rounds, timers, spectating, match scoring) is still to
come, and is the next slice's.

`server/` exists now and listens. It is a second Rust crate beside `game-core`,
with **no Cargo workspace** — a path dependency, so the wasm build script is
untouched. `cd server && cargo run` serves the lobby socket on `/ws` and a
`/health` check on `127.0.0.1:8080`. **It runs a match now:** `start_match`
spawns a task per room with one `World`, ticked at 60 Hz, taking `input` and
sending `map` and `snapshot`. Nothing renders it in a browser yet; two socket
clients can, and `npm run check-match` is one.

A pixel UI kit lives at `/ui`: a panel, a spinning coin and an insert-coin bar, in
three swappable palettes, with its own checks (`docs/pixel-ui.md`). No game screen
uses it yet and no page links to it.

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
10. `docs/running.md` — **one page: build, run, check, share.** Read it first if
    you need to start anything; it is the short version of the two below.
11. `docs/sharing.md` — how to put the game on a public link with cloudflared, so
    friends can open it without a deployment. Read it before testing with
    anyone: it says what friends can and cannot do today.
12. `docs/pixel-ui.md` — the pixel UI on `/ui`: the eight role names, the three
    palettes, the animation budget, and the measured result of the palette and
    stepping checks.

## What this session changed

Do not redo these; build on them.

- **The palette is a role scheme with three themes behind it.** `panel.css` names
  eight roles and no colours at all; `web/components/pixel/*.css` is one file per
  theme filling them in. **Eulbink** (Inkpendude, 7 colours) is the default,
  **Ammo-8** (Ammo, 8) is the one the brief came with, **Cryptic Ocean**
  (Klafooty, 6) is the third. A fourth palette is a CSS file, one line in the
  switch, and `npm run sprites` — no component and no rule changes.
- **The default answers to a bare `.pixel` as well as to its own name.** So a page
  with the panel on it is themed before any script runs and the stored choice only
  catches up: no flash of an unthemed panel. Exactly one theme file may carry that
  selector, and the check fails if two do.
- **The coin sheets are generated, not drawn.** `npm run sprites` writes one
  8-frame 16x16 sheet per theme, shaded from that theme's own colours, using a
  twenty-line PNG writer and no dependencies. `npm run check-pixel` re-renders
  every sheet and fails if the committed PNG has drifted from its palette — that
  is the guard on the "only palette colours" rule. It also fails on a colour that
  is in no palette, an easing curve, a frame held longer than 150ms, a theme
  missing a role, and two default themes. **All of those guards were tested by
  breaking them on purpose**, not assumed to work.
- **The pixels were checked in a real browser, not by reading the CSS.** With
  every animation paused at a chosen millisecond: four times inside one coin frame
  are pixel-identical, the eight frames differ by 608 to 1664 pixels, and sixteen
  element screenshots across the three themes contain no colour outside the
  palette. The browser-decoded sheets too. Numbers and method: `docs/pixel-ui.md`.
- **One measured deviation, and it is text.** 0.98% of a full-page screenshot is
  off-palette and all of it is subpixel anti-aliasing on glyph edges; the art,
  surfaces, borders, bevels and shadows measure clean. Nothing in CSS turns it off
  on Linux — `-webkit-font-smoothing: none`, an exact line-height, whole-pixel
  positions and `transform: translateZ(0)` each changed the count by **exactly
  nothing**. `docs/pixel-ui.md` ends with the three ways to trade it.
- **The scanline overlay stands down where the panel is.**
  `body:has(.pixel)::after` in `panel.css`: the site-wide overlay paints white at
  3%, which is not a colour in any palette. Worth remembering before putting a
  palette-exact surface anywhere else.

## What the last session changed

Do not redo these; build on them.

- **The match loop exists.** `server/src/matches.rs` is new: one `tokio` task per
  room, one `World::with_setup` per match, a 60 Hz `interval`, and a snapshot
  every third tick (20 Hz). `Hub::start_match` spawns it when `Registry::start`
  succeeds. The socket gained `input` in and `map` + `snapshot` out; `reconnect`
  and `request_swap` are the only refusals left.
- **The wire is the engine's flat buffer.** `snapshot { data: [i32; SNAP_LEN] }`,
  `serde_json`, offsets unchanged. `map { tiles: [u8; MAP_W*MAP_H] }` goes out
  when the match starts and when `World::stage` changes — which, with no waves,
  is never, so the map's stage-change path is only reachable in Co-op.
- **`input` is `{ move, fire }`**, and `tick` is deliberately not declared:
  `move` is masked to the four direction bits and `fire` adds `INPUT_FIRE`, so
  `(move & MOVE_MASK) | fire`. The loop keeps each player's latest input and
  reuses it, so a tick number has nothing to decide.
- **A finished match puts the room back in the lobby.** `Registry::finish` sets
  `Status::Waiting`; without it a played room could never be joined or started
  again. The loop's last snapshot carries `GameOver`. It does **not** name a
  winner: `GameOver` fires for the base falling and for everyone running out of
  lives alike, and the engine does not say which. That is the rounds slice's.
- **`check-match.mjs`** (`npm run check-match`) is new: two WebSocket clients, a
  real 1v1, asserting the map, the snapshots, and that only the player who sent
  `input` moves. It reads the snapshot offsets from the wasm, so no engine number
  is copied into the script.
- **`tests/ws.rs` had to stop waiting for silence.** `settle()` drained messages
  until a 50 ms gap, which never comes once snapshots stream at 20 Hz. The test
  client now has `next(type)` and `until(type, ok)` with a pending queue, and
  `settle()` is only used before a match starts. Any new socket test after
  `start_match` must use the same two.
- **`CONSTANTS_FIELDS` in `web/game/wasm.ts` is exported now**, so
  `check-match.mjs` reads the constants list from the one source instead of
  copying it a fourth time.
- **The stage-clear jingle plays alone, and is cut when the player carries on.**
  It used to run over the looping music. `Sounds::setMusicSilenced` ramps the
  music gain to zero while state is `stageClear` and back afterwards — a ramp on
  the running loop, not a stop and restart, so the track resumes where it left
  off instead of at bar one. The music also now starts from whichever finishes
  last, the unlock or the decode, rather than a frame later. The jingle itself
  keeps a handle on its source (`playJingle`/`stopJingle`) so leaving the screen
  stops 13.7 seconds of victory music rather than playing it over the next stage.
  `npm run check-audio` asserts all of it.
- **The lobby's dropdown and checkboxes are site components.**
  `web/components/Select.tsx` and `web/components/Check.tsx`. The native
  `<select>` popup is OS-drawn and cannot be styled, so `Select` re-implements
  the listbox keyboard behaviour; `Check` is a real `<input>` with
  `appearance-none`, so only the square changes. Text inputs and every button
  got a site focus ring (`focus-visible:outline-emerald-500`, and a border colour
  on `FIELD` in `web/app/lobby/page.tsx`) instead of the browser's default one.
- **The player's music switch is remembered, and outranks the jingle.** A button
  in the HUD on `/game` (`Music on` / `Music off`), stored in `localStorage` as
  `tank-in-city.music`. `Sounds::setMusicOff` sets one of two flags and
  `applyMusicGain` turns them into a gain ramp, so a stage clear ending cannot
  un-mute what the player muted. `check-audio.mjs` asserts that interaction.
- **Lives are boxes, not a number.** `Lives` in `web/components/GameCanvas.tsx`
  draws one box per remaining life (3 at the start, from `START_LIVES`), the way
  Battle City did. The count is unchanged — it is the same `players[0].lives`
  the HUD printed — and the boxes carry one `aria-label` so a screen reader hears
  "3 lives left" instead of a row of empty spans.
- **The user's tunnel is now dashboard-managed, not config-file.** Their
  `cloudflared.service` runs with the token in the systemd unit, so
  `~/.cloudflared/` is empty and `deploy/cloudflared.yml` is **not in use** —
  do not edit it expecting the live tunnel to change. Its routes are **Networking
  → Tunnels → Routes → Add route → Published application**: `/ws` →
  `http://localhost:8080` **added first**, then the bare hostname →
  `http://localhost:3000`. `docs/sharing.md` has both flows.
- **The frontend was finally looked at in a real browser** (Playwright MCP; recipe
  in "How to verify", setup in "Traps"). Every page reports
  `font-family: pixel, …` with the face `loaded` — the first real proof the font
  fix works, after two rounds of static checking — and the HUD, menu and lobby have
  no clipped or overflowing text. The lobby's hand-rolled `Select` opens on click,
  moves with ArrowDown and picks with Enter, with `aria-expanded` and
  `aria-activedescendant` correct at each step. Tab reaches *Play as guest* with a
  visible 2px ring and Enter follows it, so the "press start" confusion is gone in
  the keyboard path too. The music switch flips `Music on`/`Music off` and writes
  `tank-in-city.music`, and `/game` logs no errors and draws. One defect it found
  is fixed: under the wider pixel face the *Create an account* row wrapped its
  label mid-phrase, so `Disabled` in `web/app/page.tsx` now keeps the label whole
  and drops the note onto its own line. Left over: `/favicon.ico` 404s on every
  page (`docs/deferred.md`).

## What the session before changed

Do not redo these either.

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
  four-step entrance — all snapping with `steps()`, never easing. The prompt
  reads `> insert coin`; it used to say "press start", which promises that Enter
  starts the game, and nothing did (the button below does).
- **The pixel font was never actually rendering.** The file was fine; the wiring
  was not. `next/font` sets `--font-pixel-face` on `<body>`, while Tailwind's
  `@theme` emitted `--font-pixel: var(--font-pixel-face), ...` on `:root`, one
  level above where that name exists — so the utility went invalid at
  computed-value time and every pixel-font element quietly kept the body font.
  There is no `@theme` entry for the face at all now: `globals.css` puts the
  stack on `body`, the element next/font sets the variable on. Still checkable
  without a browser — the built CSS must read
  `body{font-family:var(--font-pixel-face), ui-monospace, ...}`.
- **The 8-bit face is the site's one face.** One rule in `globals.css` now covers
  the HUD, both menus, the overlays and the lobby, and the per-element
  `font-pixel` classes are gone with it. Two consequences worth keeping:
  **never bold anything** — Press Start 2P has one weight, so `<b>` and
  `font-bold` only blur the pixels, and emphasis is a colour (five `<b>`s became
  `<span>`s for this) — and **canvas text cannot take a class**, so `render.ts`
  reads the face off `document.body` with `getComputedStyle` for the spawn and
  power-up letters it draws inside the canvas.
- **The font file was swapped for the full TTF the user supplied**
  (`web/app/fonts/press-start-2p.ttf`, fontspace v2.0, 82 KB), replacing the
  Google Fonts Latin subset (v3.0, 12 KB). Same typeface; the swap costs 70 KB
  and buys the non-Latin glyphs. Its own OFL text was copied to
  `web/app/fonts/OFL.txt` (that build reserves the name "Press Start", not "Press
  Start 2P") and the `CREDITS.md` row now points at fontspace. The old file is
  recoverable from commit `98e89af` if the size matters later.
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
  slice left out. **Then it did not run a match yet:** nothing constructed a
  `World` for a room. The match loop above is what changed that.
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
cd server && cargo test             # 44 tests: 26 room rules, 10 unit, 8 socket
cd server && cargo clippy --all-targets      # must be silent
cd web && npm run check             # headless wasm ABI and rules
cd web && npm run check-audio       # headless sound triggers
cd web && npm run check-pixel       # palette roles, stepped animation, sprite sheets
cd web && npx tsc --noEmit          # type check
cd web && npm run build             # production build
```

Then, with the server up in another terminal (`cd server && cargo run`), because
the socket cannot be checked any other way:

```sh
cd web && npm run check-lobby       # drives the whole lobby protocol
cd web && npm run check-match       # plays a whole 1v1 over the socket
```

Then, when the change is visual (font, layout, focus, a form control), look at it
in a real browser through the Playwright MCP server: `cd web && npm run build &&
npx next start -p 3009`, then navigate to `http://localhost:3009/`, evaluate for
computed styles and overflow, and screenshot into `.playwright-mcp/`. The window
opens on the user's screen, and it plays sound through their speakers — mute
before driving `/game`.

Then, because code changed:

```sh
graphify update .
```

Current state: 37 tests pass in `game-core` and 44 in `server`, no compiler
warnings and clippy is silent in both, `npm run check` reports
`wasm ABI ok: score=2700 lives=1 stage=1 enemiesLeft=4 idleGameOver=30s
statesSeen=0,2`, `npm run check-audio` reports `audio ok`, `npm run check-lobby`
reports `lobby ok`, `npm run check-match` reports
`input moved the host's tank: x 1536 -> 1572`, `npm run check-pixel` reports
`pixel ui ok: 3 themes (eulbink by default), 3 stepped animations, 3 sprite
sheets`, `tsc` and `build` are clean, and the graph is 981 nodes / 1638 edges / 70
communities.

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
- **There IS a browser now — a frontend claim is checkable, so check it.** The
  Playwright MCP server (added 2026-10-17) is in
  `~/.config/opencode/opencode.jsonc`. It needs `--executable-path` pointing at
  `~/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome`: its default is the
  `chrome` **channel** (system Chrome, not installed) and `--browser` has no
  `chromium` choice. Start `next start -p 3009` and drive that: computed styles,
  `document.fonts`, layout overflow, console errors, clicks and key presses all
  work, and a screenshot comes back as an image you can look at. Screenshots only
  land inside the workspace (`.playwright-mcp/`, gitignored). Still **no speaker**:
  the sound the browser makes comes out of the *user's* speakers, so mute before
  driving the game (`localStorage["tank-in-city.music"] = "off"`, then load) or ask
  their ear.
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
- **Do not reach for a native form control.** An OS-drawn `<select>` popup cannot
  be styled, so the site's dropdown is `web/components/Select.tsx`; checkboxes
  are `web/components/Check.tsx`. A `<select>` that reappears in a page is a
  regression, not a shortcut.
- **`web/AGENTS.md` is generated by `next dev`, and is committed on purpose.**
  Next 16 writes its own agent-rules block there on every dev run. Deleting it
  only brings it back as an uncommitted change. Leave it.
- **Everything up to the slice 4 brief is committed; slice 4 is not.** `main`
  (renamed from `master`) ends with the lobby socket, then the brief for the match
  loop, then a graph-numbers refresh (HEAD). The match loop (slice 4) is
  **uncommitted working tree** — commit it before starting the next slice.
  `git log --oneline` has the hashes — this file deliberately does not, because
  amending a commit changes them. There is no remote yet.
  `graphify-out/`, `web/public/game_core.wasm` and the raw audio in `assets/` are
  generated or deliberately excluded and stay ignored.
- **There are two Rust crates and no Cargo workspace.** `game-core/` and
  `server/`, each with its own `Cargo.lock` and `target/`. `server/` reaches
  `game-core` by path. Adding a workspace would move the build target directory
  and touch `web/scripts/build-wasm.sh`, which works — do not add one casually.
- **`server/` runs a match now, and the loop is one task per room.**
  `Hub::start_match` reads the room under the registry lock, releases it, and
  spawns `matches::run` with the roster and the `Setup`. The task owns the
  `World`; the Hub only keeps a channel so an `input` can reach it. Two traps
  live here: an `input` for a player not in the roster is dropped (the roster is
  captured at start, so a mid-match leaver does not renumber anyone), and a
  finished match leaves its sender in `Hub::matches` — the next `start_match` for
  that code overwrites it, and a send into a dead one is ignored on purpose.
- **`settle()` is useless once a match is running.** It waits for a 50 ms gap in
  messages, and snapshots arrive every 50 ms, so it may never return. Use
  `Client::next(type)` / `until(type, ok)` in `tests/ws.rs` after `start_match`.
- **A finished match must not leave the room unusable.** A room that played a
  match used to stay `Playing` forever, so nobody could join it and no new match
  could start: `Registry::finish` now puts it back in `Waiting` and the Hub
  re-broadcasts the lobby state. If you add rounds, that is where post-match
  belongs.
- **`GameOver` does not name a winner.** It fires for the base falling and for
  every player running out of lives alike. The loop stops and sends one last
  snapshot; anything that says *who won* has to read `base_alive` and the lives,
  or the engine has to start reporting a reason.
- **`World::step` takes `&[Input]`, one per player, in slot order.** There is no
  single-input overload. `tc_step` in `wasm.rs` is the one place that still takes
  one input, because the browser only runs Single Player; it wraps it in a
  one-element slice. A match on the server calls `step` directly.
- **A `World` has no mode.** `Setup` carries the four facts the engine needs
  (player count, waves, friendly fire, sides) and nothing else. Do not grow it
  into the mode table — rounds, timers and match scoring belong to the match loop
  and the server, and `server/src/rooms.rs` already owns the mode table.
  `matches::setup_for` is the one place the two are translated.
- **`npm run check-lobby` and `npm run check-match` are the two checks that need
  something running.** Start `cd server && cargo run` first. They fail with
  `could not reach …` when the server is down, and that is the check working, not
  a bug in it. `check-match` also needs `npm run wasm` (it reads the snapshot
  offsets from the wasm) — `npm run check` builds it, and so does `prebuild`.
- **The lobby over a quick tunnel needs two tunnels and a `?server=`.** A quick
  tunnel forwards one port, so the socket needs its own, and its address changes
  every run, so it cannot be baked into the build. `docs/sharing.md` has the
  exact commands; a named tunnel avoids both, because `/ws` is routed on the one
  hostname — in `deploy/cloudflared.yml`, or as the first published-application
  route in the dashboard.
- **Route order is what makes one hostname work.** Whether in the file or the
  dashboard list, `/ws` has to be matched before the hostname-wide rule, or the
  socket request lands on Next.js and 404s. This is a silent failure: the page
  loads perfectly and only the lobby says "could not reach the game server".
- **There is no free tunnel hostname. The user has no domain (as of this
  session).** The free tier covers the tunnel, not the name: a published
  application's hostname is `<subdomain>.<domain-you-own>`, and theirs saved as
  the bare label `tank_in_city` (visible in `journalctl -u cloudflared`, which is
  how to read what Cloudflare actually pushed down). So the **quick tunnel is the
  live path** and the named tunnel + its two routes sit idle until a domain
  exists — do not treat the named tunnel as broken, and do not fill in
  `deploy/cloudflared.yml` (their tunnel is token-based, `/etc/cloudflared/token`).
- **A quick tunnel's link is not resolvable for ~30 seconds.** cloudflared prints
  the address before the DNS record exists, and the first lookup gets `NXDOMAIN`,
  which the local resolver then remembers — three separate attempts here died on
  that, and one looked like a broken tunnel when it was only a poisoned cache.
  `dig @1.1.1.1 <host>` answers while the local resolver does not. Verified after
  the wait: the game page served **200** and the whole lobby protocol ran over
  `wss://` on a real quick tunnel (`npm run check-lobby` → `lobby ok`).
- **Playing locally needs no tunnel and no domain.** `http://localhost:3000/lobby`
  is served over plain `http`, so `serverUrl()` returns `ws://localhost:8080/ws`:
  two tabs, two guests, a real room. That is the fastest way to exercise the lobby
  and, once the match page exists, a 1v1.

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
| **Whether `/ui` joins the game** | The kit and the game are two looks: `/ui` is palette-exact in three palettes, the game and lobby are still emerald. Nothing links to `/ui` yet. | The user asks about the UI, or work touches the title screen or the lobby. |
| **Text edges against the palette rule** | 0.98% of a `/ui` screenshot is anti-aliased text rather than palette colour. Accepting it is the current answer; `docs/pixel-ui.md` lists the alternatives. | The user says the rule is absolute, or asks about the fringe colours. |

## If you are picking up work now

**Start here: `docs/milestone-3.md`, "Slice 4: the match loop".** It records what
the loop does, where each of the brief's six decisions landed, the `GameOver` trap
and how it was handled, and how it is checked without a browser. The loop is in;
what it leaves is the next slice.

In the order that pays best:

1. **The match page.** This is the last thing between the lobby and the
   two-browser check in plan section 7, and the only thing left that makes the
   game watchable. `/game` runs the wasm engine locally; a match needs a page that
   renders a *server* `snapshot` instead. `render.ts` already takes a plain
   `Snapshot` object, so the renderer is reused — the work is one page, a socket
   that holds open after `start_match`, and turning the flat `data` array back
   into a `Snapshot` with the same offsets `readSnapshot` uses. **A browser does
   exist here** — the Playwright MCP server drives one (see "Traps"), so this can
   be built, driven and screenshotted in this environment; the user still has to
   judge how it feels to play.
2. **Rounds and win conditions**, right behind it: best-of-3, a round ends when a
   base falls or a side is out of lives, lives and scores reset between rounds,
   and a winner is named — which needs the `GameOver` reason the engine does not
   report yet. `docs/deferred.md` under "Engine" and "Server" lists exactly what is
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

**One thing to ask the user early**, because it blocks the two-browser check: it
needs two browsers, a tunnel and the user's hands. Ask before reaching it, not
after.
