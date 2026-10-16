# Milestone 2 — Single Player

Record of the second build session: plan section 5, "Milestone 2: Single
Player". The endless arcade run — enemy waves and all four enemy types, lives
and respawn, the base, scoring, power-ups, stage data, and the title screen.

**Status:** code complete and verified by tests and a build. Both halves of the
Milestone 2 play check are now done: the seeded-replay half is automated and
passing, and the *played* half — "a run can be played until you run out of
lives" — was played by a person, who reported that it feels fine and that the
enemy AI reads as intended. See "What is not verified here" for what that
report does and does not cover.

## The checklist, and where each item landed

| Plan item | Where |
|---|---|
| Full enemy waves and all four enemy types | `lib.rs` (`update_spawns`, `place_enemy`), `tank.rs` (`Kind`) |
| Base with brick walls, game over when it falls | `stage.rs` (`#` tile), `bullet.rs` (`Hit::Base`), `lib.rs` |
| Lives, score, stage counter | `lib.rs`, `rules.rs` |
| Endless run (score + highest stage, no final stage) | `lib.rs` (`next_stage` wraps the maps); shown on the game-over overlay |
| Power-ups (Single Player column) | `powerups.rs`, `World::apply_powerup` |
| Stage data loading | `stage.rs`, `assets/stages/stage{1,2,3}.txt` |
| Pause, game over, next stage | `engine.ts` (pause), `GameCanvas.tsx` (overlays), `lib.rs` (`State`) |
| Title screen (log in, sign up, play as guest) | `web/app/page.tsx`, `web/app/menu/page.tsx`, `web/game/guest.ts` |
| **Check:** a run is played until lives run out, seeded replay reproduces it | Replay: automated and passing. Played: **needs a browser** |

## Layout added

```
assets/stages/         stage1.txt, stage2.txt, stage3.txt (plain text maps)
game-core/src/
  stage.rs             ASCII stage parsing, spawn points, base centre
  powerups.rs          the PowerUp enum and a drop on the map
  (lib.rs, tank.rs, bullet.rs, ai.rs, rules.rs rewritten for the above)
game-core/tests/
  engine.rs            rules, waves, lives, power-ups, determinism
  stages.rs            every stage is well formed and connected
web/
  app/page.tsx         title screen
  app/menu/page.tsx    main menu (guest)
  game/guest.ts        generated guest name
```

`server/` is still absent: rooms are Milestone 3. There is no `web/app/play`
route any more; the game is reached through the menu.

## Stage data

`assets/stages/*.txt` is a 26x26 grid, one character per tile, baked into the
binary with `include_str!` so the crate still never touches a filesystem (it
must run on WASM). Editing a stage needs no rebuild of anything but `game-core`.

```
.  empty      B  brick     S  steel     W  water    T  tree
#  base       P  player spawn (clear tile)       E  enemy spawn (clear tile)
```

`P` and `E` mark the **top-left of a 2x2 tank**, so the three tiles right of
and below a marker must also be clear. `tests/stages.rs` enforces that, plus a
sealed border, a base, and — the one that matters — that a 2x2 tank can drive
from the player spawn to every enemy spawn. A hand-drawn maze rots silently
otherwise.

Stage numbers are 1-based and wrap, so the endless run cycles 1, 2, 3, 1, … Plan
section 11 asks for at least five maps before shipping; three is enough to prove
the loading and cycling path.

The generator used to draw these files was a throwaway script and is not in the
repo. Edit the text files directly.

## Rules implemented (plan section 6)

**Waves.** 20 enemies per stage, at most 4 alive at once, from 3 spawn points
that flash for 1 s before a tank appears. Enemy type is drawn from a weighted
table that shifts with the stage number, so later stages have more Fast and
Power tanks and, from stage 3 on, Armour. All of it from the seeded RNG.

**Enemy types.**

| Type | Speed | Bullet | Hits | Score |
|---|---|---|---|---|
| Basic | 8 (slow) | 32 | 1 | 100 |
| Fast | 16 (quick) | 32 | 1 | 200 |
| Power | 10 | 48 (fast) | 1 | 300 |
| Armour | 8 | 32 | 4, +1 every 2 stages, cap 10 | 400 |

**Lives.** Start with 3. A destroyed *player* tank respawns next to the base
after 3 s with 2 s of invulnerability, and a tank with respawn invulnerability
cannot fire (defense only). Enemies never respawn — a dead enemy just counts
against the wave, which is why a stage can be cleared. One extra life per 5000
points.

**Base.** Losing it ends the run, the same as losing the last life. Only an
enemy shot can do it — your own shots die on it and break its ring, but cannot
end the run. Its brick ring can be reinforced by the Steel Walls power-up.

There is exactly **one** base, and it is the player's. **The AI never owns a
base** in Single Player — that is the plan's own wording: "In Single Player and
Co-op the enemy has no base, so it simply consumes the pickup and robs the
player of it" (§Power-ups). So AI tanks only ever *attack* a base; the
`HEAD_FOR_BASE_CHANCE` knob is about them heading for yours, never about
defending one of their own. A side owning a base to defend is a *team-mode* idea
("each team defends exactly one base", §Base), and the plan gives those modes no
AI tanks at all. If a future mode ever puts an AI side on a team with a base,
three things have to become per-side: the stage format, the `Tile::Base` lookup
in `stage::base_center`, and the single `World::base_alive` flag.

**Power-ups.** A bonus enemy — the 4th, 11th and 18th spawned in a stage —
flashes and drops one when destroyed, at its own tile, anywhere on the map. At
most one lies on the map at a time.

| Power-up | Player takes it | An enemy takes it |
|---|---|---|
| Extra life | +1 life | survives one extra hit |
| Shield | invulnerable 8 s, can still fire | same |
| Freeze | enemies frozen 8 s | the player is frozen 8 s |
| Destroy all | every enemy on the map dies (and scores) | the player dies |
| Upgrade | faster bullets, then a second shot in flight | same |
| Speed | +50% speed for 10 s | same |
| Steel walls | the base's brick ring becomes steel for 15 s | nothing (no base to defend) |

Shield and respawn invulnerability both stop damage; only respawn
invulnerability blocks firing. Upgrades last until the tank is destroyed.

## Browser ABI

Unchanged in shape: no wasm-bindgen, four exports, direct linear-memory reads.

| Export | Purpose |
|---|---|
| `tc_init(seed: u32)` | Start or restart a run |
| `tc_step(input: u32)` | Advance one 60 Hz step |
| `tc_snapshot_ptr() -> *const i32` | Render state, rewritten every step |
| `tc_map_ptr() -> *const u8` | Tile grid, `MAP_W * MAP_H` bytes |
| `tc_constants_ptr() -> *const i32` | 44 values: geometry, layout, states, tile and type ids |

`CONSTANTS_FIELDS` in `web/game/wasm.ts` lists the same 44 in the same order and
must stay in step with `CONSTANTS` in `wasm.rs`. `npm run check` asserts the
invariants between them (section offsets, sentinels, enum values).

**Snapshot layout is now fixed offsets**, not packed blocks — the sections are
sized to the engine's maximums, so a reader needs no dynamic arithmetic:

| Offset | Value |
|---|---|
| 0–5 | `tick`, `state`, `score`, `lives`, `stage`, `enemies_left` |
| 6–15 | player `x, y, dir, alive, hp, upgrade, shield, speed, frozen, invuln` |
| 16–19 | `base_alive`, `enemy_count`, `bullet_count`, `spawn_count` |
| 20–22 | drop `kind` (`-1` = none), `x`, `y` |
| 23 … | 4 enemy records, stride 8: `x, y, dir, alive, hp, id, kind, carries` |
| 54 … | 12 bullet records, stride 5: `x, y, dir, speed, owner` |
| 114 … | 3 spawn records, stride 3: `tile x, tile y, flash ticks` |
| — | `SNAP_LEN` = 124 |

`state` is `0` playing, `1` stage clear, `2` game over. `enemies_left` counts
unspawned plus alive, so the HUD shows what the original shows.

## Tuning knobs (all values are starting defaults to tune)

| Knob | File | Value | Effect |
|---|---|---|---|
| `Kind::Player::speed` | `tank.rs` | 12 (0.75 px/tick, 45 px/s) | Player drive speed |
| `Kind::Basic/Fast/Power/Armor::speed` | `tank.rs` | 8 / 16 / 10 / 8 | Enemy drive speeds |
| `Kind::Power::bullet_speed` | `tank.rs` | 48 (others 32) | Power-tank bullets |
| `Kind::pick` weights | `tank.rs` | `[4, 1+s/2, 1+s/3, s/2]` | Difficulty curve across stages |
| `Kind::Armor::hp` | `tank.rs` | `4 + (stage-1)/2`, cap 10 | How much armour soaks |
| `PLAYER_FIRE_COOLDOWN` | `lib.rs` | 20 ticks | Minimum gap between player shots |
| `TURN_TICKS` | `ai.rs` | 40–120 ticks | How long an enemy holds a direction |
| `FIRE_TICKS` | `ai.rs` | 90–260 ticks | Gap between enemy shots |
| `ENEMY_FIRE_COOLDOWN` | `rules.rs` | 40 ticks | Hard floor on that gap |
| `TURN_CHANCE` | `ai.rs` | 40% | Chance of a new random direction |
| `HEAD_FOR_BASE_CHANCE` | `ai.rs` | 10% | Chance of aiming at the base |
| `START_LIVES` | `rules.rs` | 3 | Lives in a run |
| `EXTRA_LIFE_EVERY` | `rules.rs` | 5000 points | Extra-life threshold |
| `RESPAWN_TICKS`, `INVULN_TICKS` | `rules.rs` | 180, 120 | Death to respawn, then grace |
| `ENEMIES_PER_STAGE` | `rules.rs` | 20 | Length of a stage |
| `SPAWN_FLASH_TICKS` | `rules.rs` | 60 | Warning before a tank appears |
| `BONUS_ENEMIES` | `rules.rs` | `[4, 11, 18]` | Which enemies carry a power-up |
| `SHIELD/FREEZE/SPEED/STEEL_WALLS_TICKS` | `rules.rs` | 480/480/600/900 | Power-up durations |
| `MAX_ENEMIES` | `tank.rs` | 4 | Enemies on the map at once |
| `max_bullets` | `tank.rs` | 1, 2 when upgraded | Shots in flight per tank |

The AI is **deliberately dim**, and that is the target rather than a shortfall:
an enemy picks a direction at random, holds it for 40–120 ticks, and only
sometimes aims at the base or at you. It does not plan, does not dodge, and does
not coordinate with the other three. A player who compared it to the original —
"random, they do not have that much intelligence, same as Battle City" — was
describing the intent. Anything smarter belongs behind a per-stage or per-mode
switch, not in the default.

## Reproducing a run

The engine is deterministic from `(seed, inputs)`, so a play session can be
replayed exactly. The game-over overlay prints the run's seed and links to
`/game?seed=<that seed>`; loading that URL starts the identical run. A report
like "seed 12345, died at 40 s on stage 1" is therefore something anyone can
reproduce, and it is what a difficulty report should contain.

`?seed=` takes digits only; anything else is treated as no seed and the run is
random. `?seed=0` is a real seed, not "unset". Values above `u32` wrap.

Pressing R starts a *new* random run, so the displayed seed changes with it.

## Behaviour worth knowing

- **Four directions, one axis per tick.** A tank's position changes on at most
  one axis per tick, so movement is only ever up, down, left or right. When the
  direction changes while the tank sits between two tile edges, that tick is
  spent snapping onto the nearest edge instead of travelling. The snap is at
  most half a tile and happens once, in one frame. It has to exist: a tank only
  fits through a 2-tile gap when it is aligned to the tile grid, and without it
  you would have to stop on exactly the right pixel to go through a doorway.
  Full reasoning in [grid-alignment.md](grid-alignment.md).
- **Whole-tile bricks.** Each brick tile takes one bullet. The original's
  bricks are quarter tiles, so one tile here is four times as brittle. The stage
  files compensate by giving the base a second brick layer above it; without
  that an untouched player lost the base in about 14 seconds.
- **A bullet reports what stopped it** (`Hit::None/Wall/Brick/Base`), which is
  how the base's destruction is noticed without re-scanning the map.
- **Only an enemy shot destroys the base.** The player spawns on the base's row,
  and while its own shots counted, holding RIGHT + FIRE from the spawn point
  levelled the base in 47 ticks — the whole run lost from a standstill. A player
  bullet that reaches the base still dies there and still breaks the ring brick
  in front of it; it just cannot end the run. Enemy shots end it as before.
- **Enemies fire only when they have a shot slot free**, counted per stable
  `Tank::id`, so compaction of the enemy list cannot leak a free shot.
- **A spawn that is cancelled** — the player is parked on the point when the
  flash expires — goes back into the wave rather than being lost, so a stage can
  always be cleared.
- **The stage-clear screen waits for the player.** Clearing the last enemy stops
  the simulation in `State::StageClear`, and it stays there until a confirm
  arrives: Enter, or a click on the canvas. Nothing advances on a timer. Fire is
  deliberately *not* a confirm — a player still mashing the fire key when the
  last enemy dies would skip the screen and the sound with it — and the engine
  ignores the confirm bit outside `StageClear`. This replaced a fixed 2 s pause,
  which cut off a 13.7 s stage-clear sound six times over and never gave the
  player time to read the score. It is an input-driven screen now, not a timed
  one. Only this screen behaves that way; game over still ends the run outright.
- **Trees and water do not hide anything yet**; they are tiles that block tanks
  (water) or nothing at all (trees) exactly as on Day 1. Trees blocking
  line-of-sight is a rendering concern, and rendering is still rectangles.

## Why the numbers are the width they are

The rule is right-size each field, not "make everything `u8`": give a field the
narrowest type that holds its longest value, and only where that is a real win.
This was first written up as *deferred*, with the reasoning below, and then
carried out anyway once the rule was made explicit. Both the reasoning and the
result are kept here, because the reasoning is what decides where the line falls.

**What was narrowed.** Tick counters, which are the bulk of it, and tile indices,
which were the worst offender:

| Field | Was | Now | Longest value it must hold |
|---|---|---|---|
| `Tank.hp` | `i32` | `u8` | 10 (`MAX_ARMOR_HP`) |
| `Tank.cooldown` | `u32` | `u16` | 260 (`FIRE_TICKS`) |
| `Tank.turn_timer` | `u32` | `u8` | 120 (`TURN_TICKS`) |
| `Tank.fire_timer` | `u32` | `u16` | 260 (`FIRE_TICKS`) |
| `Tank.shield` | `u32` | `u16` | 480 (`SHIELD_TICKS`) |
| `Tank.invuln` | `u32` | `u8` | 120 (`INVULN_TICKS`) |
| `Tank.speed_boost` | `u32` | `u16` | 600 (`SPEED_TICKS`) |
| `Tank.frozen` | `u32` | `u16` | 480 (`FREEZE_TICKS`) |
| `World.lives` | `u32` | `u8` | 3 to start |
| `World.to_spawn` | `u32` | `u8` | 20 (`ENEMIES_PER_STAGE`) |
| `World.respawn_timer` | `u32` | `u8` | 180 (`RESPAWN_TICKS`) |
| `World.steel_timer` | `u32` | `u16` | 900 (`STEEL_WALLS_TICKS`) |
| `World.spawn_flash` | `[u32; 3]` | `[u8; 3]` | 60 (`SPAWN_FLASH_TICKS`) |
| `steel_tiles`, `spawns`, `player_spawn` | `(i32, i32)` pairs | `(u8, u8)` pairs | 25 — a tile index in a 26 × 26 map |

`steel_tiles` alone was 128 bytes to remember 16 tile positions that need five
bits each. As `u8` pairs it is 32.

The `rules.rs` constants carry the same widths as the fields they feed, so the
assignments need no casts. `TICK_HZ` stays `u32` because everything else is
derived from it.

**What was deliberately left wide.** Positions and the things that scale without
a bound:

- `Tank.x/y`, `Bullet.x/y`, `Drop.x/y`, `base_center` stay `i32`. They fit in
  `i16` (a 26-tile world is 6,656 fixed-point units) but they are what the
  collision and movement path does arithmetic on, in the hottest loop in the
  engine — up to 24 single-unit steps per tank per tick, each with a
  `div_euclid`. WebAssembly has no 8- or 16-bit registers, only `i32`/`i64`, so a
  narrower field is a load-mask-store on every access. Narrowing the coldest
  fields saves bytes; narrowing the hottest would spend speed to save bytes
  nobody is short of.
- `tick` (`u32`) wraps after 2.2 years at 60 Hz, and `score` (`i32`) and `stage`
  (`u32`) have no bound worth betting on in an endless run.
- `Input.bits` stays `u32` to match the C ABI parameter. It is never stored, so
  narrowing it would buy nothing.

**The result.**

| Thing | Size |
|---|---|
| The whole `World` — one instance, a single static | **1,144 bytes** (was 1,384) |
| One `Tank` | 28 bytes (was 48) |
| The tile map inside it | 676 bytes, already `u8` at 26 × 26 |
| The snapshot handed to JavaScript every frame | 496 bytes, `[i32; 124]` |
| WebAssembly linear memory, as instantiated | 1,114,112 bytes (17 pages) |

240 bytes off, 17%. Still 0.1% of the module's reserved memory, and the honest
framing has not changed: this is hygiene, not a performance fix. It matters
because Milestone 3 runs one `World` per room and the room count is the
multiplier — and because a field that says `u8` documents its own range.

`tests/sizes.rs` pins the sizes so a field cannot quietly widen back. Raise the
number deliberately, in the same commit as the feature that needs the room.

**A bug this found.** `stage::base_center` had a fallback for a stage with no
base that returned *tile indices* while the normal path returned *world units*,
so a baseless stage would have pointed every enemy at the top-left corner. Dormant
(the three real stages all have a base) and fixed while passing through.

**And a lint that proves the point.** Narrowing `hp` to `u8` made the two
`hp <= 0` checks in `lib.rs` clippy errors: `absurd_extreme_comparisons` is
deny-by-default, because on an unsigned type `<= 0` is just `== 0` with a case
that can never happen. They are `hp == 0` now. `cargo clippy --all-targets` is
silent, and is part of the check.

## Verification performed

```sh
cd game-core && cargo test                    # 28 tests: 23 engine, 1 sizes, 4 stage data
cd game-core && cargo clippy --all-targets    # silent
cd web && npm run check                       # headless wasm ABI + rules check
cd web && npm run check-audio                 # headless sound-trigger check
cd web && npx tsc --noEmit && npm run build   # type check and production build
```

- `cargo test` covers: the wave cap of 4, armour hit points scaling with the
  stage, score per enemy type, the extra-life threshold, losing all lives ending
  the run, respawn with invulnerability and reset upgrades, the base being lost
  to a shot and surviving one behind steel walls, clearing a wave advancing the
  stage with lives carried over, and that the stage-clear screen waits for a
  confirm instead of a timer (600 ticks of holding fire must not advance it),
  every power-up's effect for the player *and*
  for an enemy, steel walls reverting to brick, bullet-versus-brick and
  bullet-versus-steel, tank solidity, that a tank moves on one axis at a time,
  that a turn lines the tank up in exactly one tick, that your own shot cannot
  destroy your own base, and two determinism tests
  (same seed reproduces, neighbouring seeds must not).
- `scripts/check-wasm.mjs` loads the built module in Node, checks the 45
  constants and their invariants, the map buffer, a fresh run's starting
  position, that the confirm bit is its own bit and does nothing during play,
  four 6000-tick runs against the layout's invariants, that an idle
  player eventually loses the run, and determinism.
- The RNG seeding bug this found: `Rng::new` was `seed | 1`, so seeds 2 and 3
  (and 4 and 5, …) replayed *identically*. Found by two seeds in a scripted run
  producing byte-identical results. Fixed with a SplitMix64 scramble; there is a
  regression test over the first 64 seeds.
- The movement bug a player reported: turning after driving up made the tank
  drift, which read as diagonal movement. The turn-alignment step eased the
  off-axis coordinate into place at up to `speed` units *per tick while still
  driving*, so for roughly a fifth of a second after every direction change the
  tank moved on both axes. It is now a single step on a tick of its own, and
  `a_tank_moves_on_one_axis_at_a_time` counts ticks where both axes moved —
  it must be zero.
- A throwaway **harness** bot in `/tmp` (not committed) played the game end to
  end. It is a script standing in for a human player — it is *not* the enemy AI
  tanks, and it does not have a base of its own; it defends the player's, the way
  a person would. Worst seed survives about 16 s, best about 98 s, scoring up to
  1900. That is a floor on run length, not a difficulty measurement: it never
  dodges, so it eats every bullet. It confirms the waves, damage, deaths and
  respawn all run under real play rather than only in unit tests. Two of its
  twelve seeds end at 16 s with all three lives intact, which is the base being
  shelled by an enemy, not the player dying.
- The self-destruct footgun, found by driving the built wasm directly rather
  than by playing: the player stands at spawn, holds RIGHT + FIRE, and the base
  is gone 47 ticks later. Only enemy shots destroy the base now, and
  `your_own_shot_cannot_destroy_your_base` holds the spawn-row case down.
- `next build` produces `/`, `/menu` and `/game`, and a running server returns
  `application/wasm` for `/game_core.wasm`.

### What is not verified here

There is still no browser *in this environment* — every frontend check here is
static (`tsc`, `build`, `curl`). What a player has since done closes most of that
gap:

1. **Feel — played once, and it passed.** A player played a run and reported it
   "feels fine", with the enemy AI reading as intended (random and not clever,
   like Battle City). Both play checks — Milestone 1's movement feel and this
   milestone's "can a run be played until you run out of lives" — are therefore
   closed, and the tuning below is left as it is. That is one session's
   judgement, not a survey: if a later run feels wrong, report its seed (see
   "Reproducing a run") and it replays exactly. First knobs to move are
   `FIRE_TICKS`, `HEAD_FOR_BASE_CHANCE` and the player speed.
2. **The overlay, HUD and canvas have now drawn frames.** A player read a seed
   (`2444195321`) off the game-over overlay, which exercises the whole chain:
   canvas, HUD, overlay, and the `?seed=` plumbing that carries the seed out to
   the page. What is still unclicked is the replay half — nobody has followed
   `/game?seed=<n>` and confirmed it starts the same run. That claim rests on the
   headless determinism check, not on a browser.
3. **Nobody has seen the stage-clear screen in a browser.** No run on this
   record cleared all 20 enemies of a stage — the player's run and every
   scripted run ended earlier. The new "click, or press Enter, to carry on"
   screen is therefore verified in the engine (`cargo test` holds fire down for
   600 ticks and it does not advance; one confirm bit then loads stage 2) and the
   bit is verified to reach the built wasm (`npm run check` does nothing during
   play with it). What is unverified is the part a person sees: the wording, and
   that clicking the canvas actually reaches the engine through
   `Game.clicked()`. Reaching stage 2 is the first time this screen appears, so
   the first person to clear a stage is the first to test it.

## Deliberately not done yet

Kept in one place so it cannot drift: **[deferred.md](deferred.md)**. That list
holds the sound files still missing, the undecided engine loop, the mix numbers
that are still guesses, the `Mode` enum and the state packing that both wait on
Milestone 3, and everything the later milestones own.
