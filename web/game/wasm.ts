/**
 * The game-core WASM ABI. Mirrors game-core/src/wasm.rs and nothing else.
 *
 * game-core is built with plain `cargo build --target wasm32-unknown-unknown`
 * (see scripts/build-wasm.sh), so there is no wasm-bindgen glue: we call the
 * exported functions and read the engine's linear memory directly.
 */

export type Core = {
  memory: WebAssembly.Memory;
  tc_init(seed: number): void;
  tc_step(input: number): void;
  tc_snapshot_ptr(): number;
  tc_map_ptr(): number;
  tc_constants_ptr(): number;
};

/** Field order of the constants block in game-core/src/wasm.rs. */
const CONSTANTS_FIELDS = [
  "tile",
  "tankSize",
  "worldW",
  "worldH",
  "mapW",
  "mapH",
  "subpixels",
  "tickHz",
  "snapLen",
  "snapHeader",
  "snapEnemy",
  "snapBullet",
  "snapSpawn",
  "snapTanksOffset",
  "snapBulletsOffset",
  "snapSpawnsOffset",
  "snapNoDrop",
  "maxEnemies",
  "maxBullets",
  "maxSpawns",
  "inputUp",
  "inputDown",
  "inputLeft",
  "inputRight",
  "inputFire",
  "statePlaying",
  "stateStageClear",
  "stateGameOver",
  "powerupExtraLife",
  "powerupShield",
  "powerupFreeze",
  "powerupDestroyAll",
  "powerupUpgrade",
  "powerupSpeed",
  "powerupSteelWalls",
  "enemyBasic",
  "enemyFast",
  "enemyPower",
  "enemyArmor",
  "tileBrick",
  "tileSteel",
  "tileWater",
  "tileTree",
  "tileBase",
  // Appended to match the Rust array; see CONSTANTS in game-core/src/wasm.rs.
  "inputConfirm",
] as const;

export type Constants = Record<(typeof CONSTANTS_FIELDS)[number], number>;

export type EnemyView = {
  x: number;
  y: number;
  dir: number;
  hp: number;
  id: number;
  kind: number;
  carries: boolean;
};

export type BulletView = { x: number; y: number; dir: number; owner: number };
export type SpawnView = { x: number; y: number; flash: number };

export type PlayerView = {
  x: number;
  y: number;
  dir: number;
  alive: boolean;
  hp: number;
  upgrade: number;
  shield: number;
  speed: number;
  frozen: number;
  invuln: number;
};

export type Snapshot = {
  tick: number;
  state: number;
  score: number;
  lives: number;
  stage: number;
  enemiesLeft: number;
  player: PlayerView;
  baseAlive: boolean;
  enemyCount: number;
  bulletCount: number;
  spawnCount: number;
  drop: { kind: number; x: number; y: number } | null;
  enemies: EnemyView[];
  bullets: BulletView[];
  spawns: SpawnView[];
};

export async function loadCore(url = "/game_core.wasm"): Promise<{
  core: Core;
  consts: Constants;
}> {
  const bytes = await fetch(url).then((r) => {
    if (!r.ok) throw new Error(`${url}: ${r.status}`);
    return r.arrayBuffer();
  });
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const core = instance.exports as unknown as Core;

  const raw = new Int32Array(
    core.memory.buffer,
    core.tc_constants_ptr(),
    CONSTANTS_FIELDS.length,
  );
  const consts = Object.fromEntries(
    CONSTANTS_FIELDS.map((name, i) => [name, raw[i]]),
  ) as Constants;

  return { core, consts };
}

// Views over linear memory. Cached, and rebuilt if the engine ever grows its
// memory (which detaches the old views).
let cache: { buffer: ArrayBuffer; snap: Int32Array; tiles: Uint8Array } | null = null;

function views(core: Core, consts: Constants) {
  if (!cache || cache.buffer !== core.memory.buffer) {
    cache = {
      buffer: core.memory.buffer,
      snap: new Int32Array(core.memory.buffer, core.tc_snapshot_ptr(), consts.snapLen),
      tiles: new Uint8Array(core.memory.buffer, core.tc_map_ptr(), consts.mapW * consts.mapH),
    };
  }
  return cache;
}

export function readSnapshot(core: Core, consts: Constants): Snapshot {
  const { snap } = views(core, consts);
  const enemyCount = snap[17];
  const bulletCount = snap[18];
  const spawnCount = snap[19];
  const dropKind = snap[20];

  const enemies: EnemyView[] = [];
  for (let i = 0; i < enemyCount; i++) {
    const o = consts.snapTanksOffset + i * consts.snapEnemy;
    enemies.push({
      x: snap[o],
      y: snap[o + 1],
      dir: snap[o + 2],
      hp: snap[o + 4],
      id: snap[o + 5],
      kind: snap[o + 6],
      carries: snap[o + 7] === 1,
    });
  }

  const bullets: BulletView[] = [];
  for (let i = 0; i < bulletCount; i++) {
    const o = consts.snapBulletsOffset + i * consts.snapBullet;
    bullets.push({ x: snap[o], y: snap[o + 1], dir: snap[o + 2], owner: snap[o + 4] });
  }

  const spawns: SpawnView[] = [];
  for (let i = 0; i < spawnCount; i++) {
    const o = consts.snapSpawnsOffset + i * consts.snapSpawn;
    spawns.push({ x: snap[o], y: snap[o + 1], flash: snap[o + 2] });
  }

  return {
    tick: snap[0],
    state: snap[1],
    score: snap[2],
    lives: snap[3],
    stage: snap[4],
    enemiesLeft: snap[5],
    player: {
      x: snap[6],
      y: snap[7],
      dir: snap[8],
      alive: snap[9] === 1,
      hp: snap[10],
      upgrade: snap[11],
      shield: snap[12],
      speed: snap[13],
      frozen: snap[14],
      invuln: snap[15],
    },
    baseAlive: snap[16] === 1,
    enemyCount,
    bulletCount,
    spawnCount,
    drop: dropKind === consts.snapNoDrop ? null : { kind: dropKind, x: snap[21], y: snap[22] },
    enemies,
    bullets,
    spawns,
  };
}

export function readTiles(core: Core, consts: Constants): Uint8Array {
  return views(core, consts).tiles;
}

/// Tile values; mirrors the `Tile` enum in game-core/src/map.rs.
export const TILE = {
  EMPTY: 0,
  BRICK: 1,
  STEEL: 2,
  WATER: 3,
  TREE: 4,
  BASE: 5,
} as const;
