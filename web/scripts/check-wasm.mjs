// Self-check for the wasm ABI: run the engine headless, no browser.
// Run: npm run check
import { readFile } from "node:fs/promises";

const bytes = await readFile(new URL("../public/game_core.wasm", import.meta.url));
const { instance } = await WebAssembly.instantiate(bytes, {});
const x = instance.exports;

// Same order as CONSTANTS_FIELDS in game/wasm.ts.
const FIELDS = [
  "tile", "tankSize", "worldW", "worldH", "mapW", "mapH", "subpixels", "tickHz",
  "snapLen", "snapHeader", "snapEnemy", "snapBullet", "snapSpawn",
  "snapEnemiesOffset", "snapBulletsOffset", "snapSpawnsOffset", "snapNoDrop",
  "maxEnemies", "maxBullets", "maxSpawns",
  "inputUp", "inputDown", "inputLeft", "inputRight", "inputFire",
  "statePlaying", "stateStageClear", "stateGameOver",
  "extraLife", "shield", "freeze", "destroyAll", "upgrade", "speed", "steelWalls",
  "basic", "fast", "power", "armor",
  "tileBrick", "tileSteel", "tileWater", "tileTree", "tileBase", "inputConfirm",
  "snapPlayer", "snapPlayersOffset", "maxPlayers",
];
const raw = new Int32Array(x.memory.buffer, x.tc_constants_ptr(), FIELDS.length);
const c = Object.fromEntries(FIELDS.map((f, i) => [f, raw[i]]));

// --- Geometry and layout ---
assert(c.tickHz === 60, "fixed 60 Hz timestep");
assert(c.tile === 16 * c.subpixels, "tile is 16 px");
assert(c.tankSize === 2 * c.tile, "tank is 2x2 tiles");
assert(c.worldW === c.mapW * c.tile && c.worldH === c.mapH * c.tile, "world size");
assert(c.snapPlayersOffset === c.snapHeader, "player records follow the header");
assert(
  c.snapEnemiesOffset === c.snapPlayersOffset + c.maxPlayers * c.snapPlayer,
  "enemies follow players",
);
assert(
  c.snapBulletsOffset === c.snapEnemiesOffset + c.maxEnemies * c.snapEnemy,
  "bullets follow enemies",
);
assert(
  c.snapSpawnsOffset === c.snapBulletsOffset + c.maxBullets * c.snapBullet,
  "spawn points follow bullets",
);
assert(
  c.snapLen === c.snapSpawnsOffset + c.maxSpawns * c.snapSpawn,
  "snapshot buffer is exactly the four sections",
);
assert(c.snapNoDrop === -1, "no-drop sentinel");
assert(c.statePlaying === 0 && c.stateStageClear === 1 && c.stateGameOver === 2, "states");
assert(c.maxPlayers === 6, "3v3 is the widest room (plan section 3)");

const snapshot = () => new Int32Array(x.memory.buffer, x.tc_snapshot_ptr(), c.snapLen);
const tiles = () => new Uint8Array(x.memory.buffer, x.tc_map_ptr(), c.mapW * c.mapH);

const H = {
  tick: 0, state: 1, stage: 2, enemiesLeft: 3, baseAlive: 4, players: 5,
  enemies: 6, bullets: 7, spawns: 8, dropKind: 9,
};

/** Player 0's record. Single Player is the only run the browser starts. */
const P = {
  x: c.snapPlayersOffset,
  y: c.snapPlayersOffset + 1,
  dir: c.snapPlayersOffset + 2,
  alive: c.snapPlayersOffset + 3,
  hp: c.snapPlayersOffset + 4,
  team: c.snapPlayersOffset + 10,
  lives: c.snapPlayersOffset + 11,
  score: c.snapPlayersOffset + 12,
};

function checkInvariants(s, where) {
  assert(s[H.state] >= 0 && s[H.state] <= 2, `${where}: known state`);
  assert(s[H.players] >= 1 && s[H.players] <= c.maxPlayers, `${where}: players`);
  assert(s[P.lives] >= 0, `${where}: lives`);
  assert(s[P.score] >= 0, `${where}: score`);
  assert(s[H.stage] >= 1, `${where}: stage`);
  assert(s[H.enemiesLeft] >= 0 && s[H.enemiesLeft] <= 20, `${where}: enemies left`);
  assert(s[H.enemies] <= c.maxEnemies, `${where}: concurrent enemies`);
  assert(s[H.bullets] <= c.maxBullets, `${where}: concurrent bullets`);
  assert(s[P.x] >= c.tile && s[P.x] + c.tankSize <= c.worldW, `${where}: player x`);
  assert(s[P.y] >= 0 && s[P.y] + c.tankSize <= c.worldH, `${where}: player y`);
  assert(s[H.dropKind] >= c.snapNoDrop && s[H.dropKind] <= 6, `${where}: drop kind`);
}

function run(seed, ticks) {
  x.tc_init(seed);
  const states = new Set();
  for (let i = 0; i < ticks; i++) {
    x.tc_step(i % 160 < 40 ? c.inputUp | c.inputFire
      : i % 160 < 80 ? c.inputRight | c.inputFire
      : i % 160 < 120 ? c.inputLeft
      : c.inputLeft | c.inputFire);
    if (i % 250 === 0) {
      const s = snapshot();
      checkInvariants(s, `seed ${seed} tick ${i}`);
      states.add(s[H.state]);
    }
  }
  return { snap: Array.from(snapshot()), states };
}

// --- The stage-clear screen waits for the player ---
// The screen cannot be reached from here without clearing 20 enemies, which no
// scripted run does. What is checked is the bit itself: it is exported, it is
// distinct from fire, and sending it while playing changes nothing.
assert(c.inputConfirm === 32, "confirm is its own bit");
assert(
  c.inputConfirm !== c.inputFire &&
    (c.inputConfirm & c.inputFire) === 0 &&
    (c.inputConfirm & c.inputUp) === 0,
  "confirm does not collide with another bit",
);
x.tc_init(7);
const beforeConfirm = Array.from(snapshot());
for (let i = 0; i < 30; i++) x.tc_step(c.inputConfirm);
const afterConfirm = snapshot();
// The player must not move, shoot or lose anything. The clock, the spawn
// flashes and the wave count do move on their own, so those are left out.
for (const [name, i] of Object.entries({
  state: H.state, stage: H.stage,
  score: P.score, lives: P.lives, x: P.x, y: P.y, dir: P.dir, alive: P.alive, hp: P.hp,
  bullets: H.bullets,
})) {
  assert(afterConfirm[i] === beforeConfirm[i], `confirm during play left ${name} alone`);
}
assert(afterConfirm[H.state] === c.statePlaying, "still playing");

// --- A fresh run ---
x.tc_init(1);
const start = snapshot();
assert(start[H.tick] === 0, "tick starts at zero");
assert(start[H.players] === 1, "single player is one player");
assert(start[P.lives] === 3, "three lives");
assert(start[P.score] === 0, "no score yet");
assert(start[P.team] === 0, "the player is on side A");
assert(start[H.stage] === 1, "stage one");
assert(start[H.enemiesLeft] === 20, "twenty enemies in a stage");
assert(start[H.baseAlive] === 1, "base standing");
assert(start[H.enemies] === 0, "no enemies until the first spawn flash");
assert(start[H.spawns] === 3, "three enemy spawn points on the stage");
assert(tiles().length === c.mapW * c.mapH, "map buffer");
assert(tiles().some((t) => t === c.tileSteel), "stage has steel");
assert(tiles().some((t) => t === c.tileBase), "stage has a base");
assert(
  tiles().filter((t) => t === c.tileBase).length === 4,
  "the base is 2x2 tiles",
);

// --- A long run: the wave system has to hold up ---
let scored = 0;
let long = null;
for (const seed of [1, 2, 3, 4]) {
  const result = run(seed, 6000);
  if (result.snap[P.score] > 0) scored++;
  long ??= result;
}
assert(scored > 0, "enemies were destroyed and scored");
assert(long.snap[H.tick] === 6000, "the run advanced");

// --- Lose conditions end the run on their own ---
// An idle player is shot to pieces: lives or the base have to run out.
x.tc_init(3);
let over = 0;
for (let i = 1; i <= 60 * 300; i++) {
  x.tc_step(0);
  if (snapshot()[H.state] === c.stateGameOver) {
    over = i;
    break;
  }
}
assert(over > 0, "an idle player eventually loses the run");

let stages = new Set();
for (const seed of [1, 2, 3, 4]) {
  const { states } = run(seed, 6000);
  for (const s of states) stages.add(s);
}
assert(stages.has(c.statePlaying) || stages.has(c.stateGameOver), "reachable states");

// --- Determinism (plan section 4) ---
const a = JSON.stringify(run(9, 3000).snap);
const b = JSON.stringify(run(9, 3000).snap);
const d = JSON.stringify(run(10, 3000).snap);
assert(a === b, "replay is deterministic");
assert(a !== d, "different seeds diverge");

console.log(
  "wasm ABI ok: score=%d lives=%d stage=%d enemiesLeft=%d idleGameOver=%ds statesSeen=%s",
  long.snap[P.score], long.snap[P.lives], long.snap[H.stage],
  long.snap[H.enemiesLeft], Math.round(over / 60), [...stages].join(","),
);

function assert(ok, what) {
  if (!ok) {
    console.error(`FAIL: ${what}`);
    process.exit(1);
  }
}
