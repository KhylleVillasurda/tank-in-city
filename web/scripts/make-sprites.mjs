// Draws the coin each theme spins: one strip of eight 16x16 frames.
//
// A sheet per theme, because the coin is made of the theme's own colours, and a
// palette that changed without its sheet changing would put colours on screen
// that the palette does not contain — the one thing this UI promises it never
// does. check-pixel re-renders every sheet and compares it with the committed
// one, so that cannot happen quietly.
//
// The art is arithmetic rather than files: a disc squeezed by the spin, lit from
// a turning angle, shaded from the theme's own ramp. A PNG writer is twenty lines
// of zlib, so there is nothing to install and no build step.
//
//   node scripts/make-sprites.mjs      # rewrites public/sprites/coin-*.png

import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { rampOf, readThemes, rgbOf } from "./palettes.mjs";

export const SIZE = 16; // one frame
export const FRAMES = 8; // one whole spin
const MID = (SIZE - 1) / 2;
const RADIUS = 6.4; // leaves a pixel of margin for the outline

const SHEETS = join(dirname(fileURLToPath(import.meta.url)), "..", "public", "sprites");

/** Where a theme's sheet lives, both on disk and on the site. */
export const sheetPath = (id) => join(SHEETS, `coin-${id}.png`);
export const sheetUrl = (id) => `/sprites/coin-${id}.png`;

/**
 * The coin in one pose: `shade(rank)` picks a colour off the theme's ramp, 0 for
 * the darkest and 1 for the brightest.
 */
function pose(turn, shade) {
  // Never narrower than a pixel: edge-on is a line, not nothing.
  const flat = Math.max(Math.abs(Math.cos(turn)), 1 / RADIUS);
  const lit = [Math.cos(turn - Math.PI / 3), -0.75]; // where the light sits
  const pixels = [];
  for (let y = 0; y < SIZE; y++) {
    for (let x = 0; x < SIZE; x++) {
      const dx = (x - MID) / (RADIUS * flat);
      const dy = (y - MID) / RADIUS;
      const d = Math.hypot(dx, dy);
      const face = dx * lit[0] + dy * lit[1] > 0; // this pixel faces the light
      if (d > 1.18) pixels.push(null); // outside the coin
      else if (d > 1) pixels.push(shade(0)); // outline
      else if (d > 0.78) pixels.push(shade(face ? 0.95 : 0.6)); // rim
      else if (d > 0.42) pixels.push(shade(face ? 0.8 : 0.45)); // face
      else pixels.push(shade(face ? 0.65 : 0.3)); // centre
    }
  }
  return pixels;
}

/** The whole strip as colours, frame after frame: a hex, or null for clear. */
export function sheetPixels(ramp) {
  const shade = (rank) => ramp[Math.round(rank * (ramp.length - 1))];
  const pixels = [];
  for (let frame = 0; frame < FRAMES; frame++) {
    pixels.push(...pose((frame / FRAMES) * Math.PI, shade));
  }
  return pixels;
}

// --- just enough PNG to write a sprite sheet -------------------------------

const CRC = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c;
  }
  return table;
})();

function crc32(buf) {
  let c = -1;
  for (const byte of buf) c = CRC[(c ^ byte) & 255] ^ (c >>> 8);
  return (c ^ -1) >>> 0;
}

function chunk(type, data) {
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const head = Buffer.alloc(4);
  head.writeUInt32BE(data.length);
  const tail = Buffer.alloc(4);
  tail.writeUInt32BE(crc32(body));
  return Buffer.concat([head, body, tail]);
}

/** 8-bit RGBA, one IDAT, filter 0 on every row. */
function png(width, height, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bits a channel
  ihdr[9] = 6; // truecolour with alpha
  const stride = width * 4;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** The strip as PNG bytes, frames side by side. */
export function renderSheet(ramp) {
  const width = SIZE * FRAMES;
  const rgba = Buffer.alloc(width * SIZE * 4);
  sheetPixels(ramp).forEach((hex, i) => {
    const frame = (i / (SIZE * SIZE)) | 0;
    const x = (i % SIZE) + frame * SIZE;
    const y = ((i / SIZE) | 0) % SIZE;
    const [r, g, b] = hex ? rgbOf(hex) : [0, 0, 0];
    const at = (y * width + x) * 4;
    rgba[at] = r;
    rgba[at + 1] = g;
    rgba[at + 2] = b;
    rgba[at + 3] = hex ? 255 : 0;
  });
  return png(width, SIZE, rgba);
}

function main() {
  const themes = readThemes();
  mkdirSync(SHEETS, { recursive: true });
  for (const theme of themes) {
    // The sheet a theme asks for is the sheet this writes; a rename here without
    // the one in the CSS would leave a panel with no coin at all.
    if (theme.sheet !== sheetUrl(theme.id)) {
      throw new Error(`${theme.file} asks for ${theme.sheet}, not ${sheetUrl(theme.id)}`);
    }
    writeFileSync(sheetPath(theme.id), renderSheet(rampOf(theme.roles)));
    console.log(`wrote ${sheetPath(theme.id)} — ${FRAMES} frames of ${SIZE}x${SIZE}`);
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main();
