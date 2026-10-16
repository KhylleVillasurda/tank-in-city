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
  "snapEnemiesOffset",
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
  "snapPlayer",
  "snapPlayersOffset",
  "maxPlayers",
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

export type BulletView = { x: number; y: number; dir: number; team: number; owner: number };
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
  /** Which side this player is on. Team modes share one; FFA gives each its own. */
  team: number;
  lives: number;
  score: number;
};

export type Snapshot = {
  tick: number;
  state: number;
  stage: number;
  enemiesLeft: number;
  baseAlive: boolean;
  playerCount: number;
  enemyCount: number;
  bulletCount: number;
  spawnCount: number;
  drop: { kind: number; x: number; y: number } | null;
  players: PlayerView[];
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
  const playerCount = snap[5];
  const enemyCount = snap[6];
  const bulletCount = snap[7];
  const spawnCount = snap[8];
  const dropKind = snap[9];

  const players: PlayerView[] = [];
  for (let i = 0; i < playerCount; i++) {
    const o = consts.snapPlayersOffset + i * consts.snapPlayer;
    players.push({
      x: snap[o],
      y: snap[o + 1],
      dir: snap[o + 2],
      alive: snap[o + 3] === 1,
      hp: snap[o + 4],
      upgrade: snap[o + 5],
      shield: snap[o + 6],
      speed: snap[o + 7],
      frozen: snap[o + 8],
      invuln: snap[o + 9],
      team: snap[o + 10],
      lives: snap[o + 11],
      score: snap[o + 12],
    });
  }

  const enemies: EnemyView[] = [];
  for (let i = 0; i < enemyCount; i++) {
    const o = consts.snapEnemiesOffset + i * consts.snapEnemy;
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
    bullets.push({
      x: snap[o],
      y: snap[o + 1],
      dir: snap[o + 2],
      team: snap[o + 3],
      owner: snap[o + 4],
    });
  }

  const spawns: SpawnView[] = [];
  for (let i = 0; i < spawnCount; i++) {
    const o = consts.snapSpawnsOffset + i * consts.snapSpawn;
    spawns.push({ x: snap[o], y: snap[o + 1], flash: snap[o + 2] });
  }

  return {
    tick: snap[0],
    state: snap[1],
    stage: snap[2],
    enemiesLeft: snap[3],
    baseAlive: snap[4] === 1,
    playerCount,
    enemyCount,
    bulletCount,
    spawnCount,
    drop: dropKind === consts.snapNoDrop ? null : { kind: dropKind, x: snap[10], y: snap[11] },
    players,
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
