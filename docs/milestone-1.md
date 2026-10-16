# Milestone 1 — vertical slice (Day 1)

Record of the first build session: plan section 7, "Day 1: setup and first
session", in its suggested order. One map, one player tank that moves and
shoots, bricks that break, one enemy that moves and shoots, on an HTML5 canvas
in the browser.

**Status:** code complete and verified by tests and a build. The Milestone 1
play check (does movement and shooting *feel* right) has since been played and
passed — including the one bug it caught, a turn that looked like diagonal
movement; see "What is not verified here".

> Superseded in parts by [Milestone 2](milestone-2.md): the snapshot layout is
> now fixed offsets, the constants block is 44 values, and the arena comes from
> `assets/stages/` instead of `Map::arena`. The Pause/`State` machine and the
> stage-clear flow also arrived there. Everything in this file about the
> fixed-point model, the timestep and `game-core` being headless still holds.

## The Day 1 order, and where each step landed

| Step | Where |
|---|---|
| 1. Scaffold the `game-core` crate | `game-core/` — `cdylib` + `rlib`, no network/DOM/database |
| 2. Map tiles and the grid | `game-core/src/map.rs` |
| 3. One player tank in integer steps | `game-core/src/tank.rs` |
| 4. Bullets | `game-core/src/bullet.rs` |
| 5. Collision (walls, bricks, tanks) | `tank.rs` (tanks vs map and each other), `bullet.rs` (bullets vs map) |
| 6. Compile the same crate to WASM | `game-core/src/wasm.rs`, `web/scripts/build-wasm.sh` |
| 7. Render map and tank on a canvas | `web/game/render.ts`, `web/components/GameCanvas.tsx` |
| 8. Play it, then tune feel | Tuning knobs below; **played — see "What is not verified here"** |

Enemy movement and shooting (the "one enemy" half of the milestone) sits in
`game-core/src/ai.rs`.

## Layout created

```
game-core/           Rust crate: all game rules (WASM + native)
  src/lib.rs         World, Input, the 60 Hz step loop, snapshot format
  src/map.rs         tile grid, arena layout, geometry constants
  src/tank.rs        tanks, driving, grid alignment, tank collisions
  src/bullet.rs      bullets, brick/steel destruction
  src/ai.rs          enemy direction and firing behaviour
  src/rng.rs         seeded xorshift64
  src/rules.rs       tuning defaults from plan section 6
  src/wasm.rs        C ABI over World, compiled only for wasm32
  tests/engine.rs    native integration tests
web/                 Next.js frontend
  app/               routes (/, /game)
  components/        GameCanvas
  game/              wasm.ts (ABI), input.ts, render.ts, engine.ts (loop)
  scripts/           build-wasm.sh, check-wasm.mjs
  public/            game_core.wasm (generated)
docs/                this folder
CREDITS.md           asset credits (plan section 12) — no assets yet
```

`assets/`, `server/` and the `powerups.rs` / `rules.rs` split-out arrive with
their milestones.

## How to run

Prerequisites: Rust with the wasm target, and Node.

```sh
rustup target add wasm32-unknown-unknown
cd web && npm install
npm run dev          # http://localhost:3000/game
```

`predev` and `prebuild` rebuild the wasm first, so the generated module can
never drift from the Rust source. `npm run wasm` does it on its own, and
`npm run check` runs the headless engine check.

Controls: arrows or WASD to drive, Space to fire, R to restart after a game
over.

## The rules that Day 1 had to get right first

**Fixed-point integers, not floats.** Every position, velocity and size is an
`i32` count of `1/16 px` (`map::SUBPIXELS`). A tile is `16 px * 16 = 256` units
and a tank is `2x2` tiles. Float math is not bit-identical between WASM and
native CPUs, and that divergence would desync client prediction from the server
(plan section 4).

**Fixed 60 Hz timestep.** `TICK_HZ = 60`, and `World::step` advances exactly one
step. `web/game/engine.ts` accumulates real time and calls `tc_step` in whole
steps, so rendering can run at any frame rate without changing the simulation.
A long stall (background tab) is clamped to 250 ms of catch-up.

**`game-core` stays headless.** No network, DOM, or database code. The only
browser-specific file is `wasm.rs`, which is compiled only for
`target_arch = "wasm32"` and is a thin wrapper over `World`.

**Seeded RNG.** Spawns and AI decisions come from `rng.rs`, so the same seed
plus the same inputs replays to the same state. `tests/engine.rs` and
`scripts/check-wasm.mjs` both assert this.

## Engine ABI

The browser does not use wasm-bindgen, so the build is plain
`cargo build --target wasm32-unknown-unknown --release` with no extra tooling.
`web/game/wasm.ts` calls these exports and reads linear memory directly.

| Export | Purpose |
|---|---|
| `tc_init(seed: u32)` | Start or restart a match |
| `tc_step(input: u32)` | Advance one 60 Hz step |
| `tc_snapshot_ptr() -> *const i32` | Render state, rewritten every step |
| `tc_map_ptr() -> *const u8` | Tile grid, `MAP_W * MAP_H` bytes |
| `tc_constants_ptr() -> *const i32` | Geometry, snapshot layout and input bits |

The constants block is 17 `i32`s in a fixed order, so no engine number is
duplicated in TypeScript. The order is listed in `game-core/src/wasm.rs` and in
`CONSTANTS_FIELDS` in `web/game/wasm.ts` — those two lists must stay in step.

Input bits: up `1`, down `2`, left `4`, right `8`, fire `16`.

Snapshot buffer, offsets in `i32`:

| Offset | Value |
|---|---|
| 0–2 | `tick`, `over`, `score` |
| 3–7 | player `x`, `y`, `dir`, `alive`, `hp` |
| 8, 9 | `enemy_count`, `bullet_count` |
| 10 … | `enemy_count` enemy records, stride 6: `x, y, dir, alive, hp, id` |
| then | `bullet_count` bullet records, stride 5: `x, y, dir, 1, owner` |

`x`/`y` are `1/16 px` units, `dir` is the `Dir` enum (up 0, down 1, left 2,
right 3). Enemy and bullet records start immediately after the previous block,
so a reader must take the counts from slots 8 and 9 before indexing.

## Tuning knobs (plan Day 1 step 8)

All values are starting defaults to tune. Units are `1/16 px` per tick unless
noted.

| Knob | File | Value | Effect |
|---|---|---|---|
| `PLAYER_SPEED` | `tank.rs` | 8 (0.5 px/tick, 30 px/s) | How fast the player drives |
| `ENEMY_SPEED` | `tank.rs` | 8 | Enemy drive speed |
| `BULLET_SPEED` | `bullet.rs` | 32 (2 px/tick, 120 px/s) | Bullet speed |
| `PLAYER_FIRE_COOLDOWN` | `lib.rs` | 20 ticks | Minimum gap between player shots |
| `TURN_TICKS` | `ai.rs` | 40–120 ticks | How long an enemy holds a direction |
| `FIRE_TICKS` | `ai.rs` | 40–120 ticks | Gap between enemy shots |
| `TANK_SIZE`, `TILE` | `map.rs` | 512, 256 | 2x2 tiles, 16 px tiles |
| `SUBPIXELS` | `map.rs` | 16 | Fixed-point resolution — do not lower |
| `MAX_ENEMIES` | `tank.rs` | 4 | Enemies on the map at once |
| `MAP_W`, `MAP_H` | `rules.rs` | 26, 26 | Arena size in tiles |

`SCORE_ENEMY_KILL` (100) in `rules.rs` is already the plan's Basic-enemy value,
ready for Milestone 2.

## Behaviour worth knowing

- **Solid tanks.** Tanks cannot overlap and cannot drive through brick, steel,
  water, or the base. The arena border is steel. Trees are decoration.
- **Grid alignment.** While driving, the perpendicular axis is snapped onto the
  nearest tile edge, so a tank slips into a gap instead of snagging on its
  corner. The snap is one step, on a tick of its own: a tank therefore changes
  position on exactly one axis per tick and never travels diagonally.
  (Milestone 2 corrected the original gradual nudge — it drifted the tank
  sideways for several ticks after every turn, which read as diagonal movement.)
  Aligning the axis being *driven* instead makes the tank oscillate; that was
  the bug that took the longest on Day 1. The full reasoning, the alternatives
  and how to inspect it are in [grid-alignment.md](grid-alignment.md).
- **Bricks break, steel does not.** A bullet destroys the one brick tile it
  hits; water lets bullets through; trees block nothing.
- **One bullet per tank.** Enforced per tank, not per side, using a stable
  `Tank::id` so the check survives the enemy list being compacted (ids are 0 for
  the player, 1+ for enemies).
- **Enemy respawn for tuning.** Milestone 1 spawns a single enemy, and a new one
  appears at a random top spawn point when the previous one dies, so the arena
  stays playable while feel is tuned. Real waves are Milestone 2.
- **Player death ends the run.** One life, no respawn yet. Lives and respawn are
  Milestone 2.

## Verification performed

```sh
cd game-core && cargo test                    # 4 integration tests
cd web && npm run check                       # headless wasm ABI check
cd web && npx tsc --noEmit && npm run build   # type check and production build
```

- `cargo test` covers: driving in integer steps and stopping flush on the tile
  grid at the arena border; tanks being solid against each other and against
  walls; bullets breaking bricks and dying on steel; and a 900-tick replay from
  a fixed seed and input script producing an identical snapshot every tick.
- `scripts/check-wasm.mjs` loads the built module in Node and checks the
  constants, the map buffer, the snapshot layout, the wall stop, and that a
  replay is deterministic while a different seed diverges.
- `next build` produces `/` and `/game`, and a running server returns
  `application/wasm` for `/game_core.wasm`.

### What is not verified here

There is no browser in this environment, so the actual Milestone 1 check —
"movement and shooting feel right after a few minutes of tuning" — had to wait
for a player. It has now been played: the first report was that turning after
driving up looked like diagonal movement, which was a real bug and is fixed
([grid-alignment.md](grid-alignment.md)), and the follow-up report was that the
movement and the enemy AI feel right — the latter reading as random and not
clever, the same as Battle City. The tuning knobs below are therefore unchanged.
That is one session's judgement; report a seed if a later run feels wrong.

## Deliberately not done yet

| Skipped | Arrives in |
|---|---|
| Enemy waves, all four enemy types, armour | Milestone 2 |
| Lives, respawn, invulnerability, score persistence | Milestone 2 |
| Base destruction and game over on base fall | Milestone 2 |
| Power-ups (`game-core/src/powerups.rs`) | Milestone 2 |
| Stage data files (the arena is hard-coded) | Milestone 2 |
| Title screen, shadcn/ui components, Tailwind UI beyond layout | Milestone 2 |
| Client-side prediction, interpolation, snapshots over the network | Milestone 3 |
| `server/` crate, rooms, `tracing` logs | Milestone 3 |
| Sprites and sound | Milestone 5 |

Tailwind is installed and used for the page layout, but shadcn/ui is not added
yet: there is no UI to put in it until the title screen exists.
