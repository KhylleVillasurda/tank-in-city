/**
 * Canvas renderer.
 *
 * Colours, not sprites: original pixel art arrives with the credits work in
 * Milestone 5, and the engine is what Milestone 2 has to prove. An enemy's
 * colour tracks its remaining hit points, so armour damage is visible.
 */

import { TILE, type Constants, type Snapshot } from "./wasm";

/** Indexed by `Dir` in game-core/src/tank.rs: up, down, left, right. */
const DIRS = [
  [0, -1],
  [0, 1],
  [-1, 0],
  [1, 0],
];

const TILE_COLORS: Record<number, string> = {
  [TILE.BRICK]: "#a4522a",
  [TILE.STEEL]: "#9aa0a6",
  [TILE.WATER]: "#1d4ed8",
  [TILE.TREE]: "#166534",
  [TILE.BASE]: "#eab308",
};

/** Indexed by `Kind`: player, basic, fast, power, armour. */
const ENEMY_COLORS = ["#34d399", "#9ca3af", "#22d3ee", "#a78bfa", "#f87171"];

/** Armour changes colour with each hit taken (plan section 6). */
const ARMOUR_COLORS = ["#f87171", "#fb923c", "#facc15", "#e5e7eb"];

/** Indexed by team: side A, side B, then anything else. Side A is the player's
 * own colour in Single Player, where it is the only team on the map. */
const TEAM_COLORS = ["#34d399", "#f472b6", "#facc15"];

const BACKGROUND = "#0b0b0f";

/** Letter drawn on each power-up, from the `PowerUp` enum order. */
const POWERUP_LETTERS = ["1", "S", "F", "X", "U", ">", "W"];

export type Renderer = {
  draw(snapshot: Snapshot, tiles: Uint8Array): void;
};

export function createRenderer(
  canvas: HTMLCanvasElement,
  consts: Constants,
  scale = 2,
): Renderer {
  // A "2d" context is always available for a canvas element.
  const ctx = canvas.getContext("2d")!;

  const width = (consts.worldW / consts.subpixels) * scale;
  const height = (consts.worldH / consts.subpixels) * scale;
  canvas.width = width;
  canvas.height = height;

  const to = (v: number) => (v / consts.subpixels) * scale;

  function enemyColor(kind: number, hp: number): string {
    if (kind === consts.enemyArmor) {
      return ARMOUR_COLORS[Math.min(ARMOUR_COLORS.length - 1, Math.max(0, hp - 1))];
    }
    return ENEMY_COLORS[kind] ?? ENEMY_COLORS[1];
  }

  function drawTank(
    x: number,
    y: number,
    dir: number,
    color: string,
    options: { shield?: boolean; blink?: boolean } = {},
  ) {
    if (options.blink) return;

    const size = to(consts.tankSize);
    const px = to(x);
    const py = to(y);

    ctx.fillStyle = color;
    ctx.fillRect(px, py, size, size);
    ctx.strokeStyle = BACKGROUND;
    ctx.lineWidth = scale;
    ctx.strokeRect(px + scale / 2, py + scale / 2, size - scale, size - scale);

    const [dx, dy] = DIRS[dir] ?? DIRS[0];
    const cx = px + size / 2;
    const cy = py + size / 2;
    ctx.strokeStyle = color;
    ctx.lineWidth = scale * 2;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx + (dx * size) / 2, cy + (dy * size) / 2);
    ctx.stroke();

    if (options.shield) {
      ctx.strokeStyle = "#60a5fa";
      ctx.lineWidth = scale;
      ctx.strokeRect(px - scale, py - scale, size + scale * 2, size + scale * 2);
    }
  }

  function drawBadge(x: number, y: number, label: string, color: string) {
    const size = to(consts.tile);
    const px = to(x);
    const py = to(y);
    ctx.fillStyle = color;
    ctx.fillRect(px, py, size, size);
    ctx.fillStyle = BACKGROUND;
    ctx.font = `${Math.floor(size * 0.75)}px monospace`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(label, px + size / 2, py + size / 2 + 1);
  }

  function draw(snapshot: Snapshot, tiles: Uint8Array) {
    ctx.fillStyle = BACKGROUND;
    ctx.fillRect(0, 0, width, height);

    const tile = to(consts.tile);
    for (let ty = 0; ty < consts.mapH; ty++) {
      for (let tx = 0; tx < consts.mapW; tx++) {
        const color = TILE_COLORS[tiles[ty * consts.mapW + tx]];
        if (!color) continue;
        ctx.fillStyle = color;
        ctx.fillRect(to(tx * consts.tile), to(ty * consts.tile), tile + 1, tile + 1);
      }
    }

    // A spawn point flashes before its tank appears.
    for (const spawn of snapshot.spawns) {
      if (spawn.flash <= 0 || Math.floor(spawn.flash / 8) % 2 === 0) continue;
      drawBadge(spawn.x * consts.tile, spawn.y * consts.tile, "!", "#fbbf24");
    }

    if (snapshot.drop) {
      drawBadge(
        snapshot.drop.x,
        snapshot.drop.y,
        POWERUP_LETTERS[snapshot.drop.kind] ?? "?",
        "#fde047",
      );
    }

    for (const enemy of snapshot.enemies) {
      drawTank(enemy.x, enemy.y, enemy.dir, enemyColor(enemy.kind, enemy.hp), {
        shield: enemy.carries,
      });
    }

    for (const player of snapshot.players) {
      if (!player.alive) continue;
      drawTank(player.x, player.y, player.dir, TEAM_COLORS[player.team] ?? TEAM_COLORS[0], {
        shield: player.shield > 0,
        // Respawn invulnerability blinks; a shield is a steady outline.
        blink: player.invuln > 0 && Math.floor(player.invuln / 6) % 2 === 0,
      });
      if (player.frozen > 0) {
        ctx.fillStyle = "rgba(96,165,250,0.35)";
        ctx.fillRect(to(player.x), to(player.y), to(consts.tankSize), to(consts.tankSize));
      }
    }

    const shot = scale * 3;
    ctx.fillStyle = "#fafafa";
    for (const bullet of snapshot.bullets) {
      ctx.fillRect(to(bullet.x) - shot / 2, to(bullet.y) - shot / 2, shot, shot);
    }
  }

  return { draw };
}
