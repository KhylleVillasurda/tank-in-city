// Reads the theme files the pixel panel is styled by, so the tools and the
// browser cannot disagree about what a palette is.
//
// A theme file is plain CSS and nothing more: its roles are its
// `--pixel-<role>: <hex>` lines, its sheet is its `--pixel-sheet` url, and its
// file name is its id — the same id the page sets in `data-theme` and the same id
// its sprite sheet is named after. Adding a theme therefore needs no entry
// anywhere, in here or in the panel.

import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const THEMES_DIR = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "components",
  "pixel",
);

/** Every theme beside panel.css, as `{ id, file, css, roles, sheet, fallsBack }`. */
export function readThemes() {
  return readdirSync(THEMES_DIR)
    .filter((name) => name.endsWith(".css") && name !== "panel.css")
    .map((file) => {
      const css = readFileSync(join(THEMES_DIR, file), "utf8");
      const roles = {};
      for (const [, role, hex] of css.matchAll(
        /--pixel-([a-z-]+):\s*(#[0-9a-f]{6})\s*;/gi,
      )) {
        roles[role] = hex.toLowerCase();
      }
      return {
        id: file.slice(0, -".css".length),
        file,
        css,
        roles,
        sheet: css.match(/--pixel-sheet:\s*url\("([^"]+)"\)/)?.[1],
        // The default theme is the one that also answers to the bare `.pixel`.
        fallsBack: /(^|\n)\.pixel\s*,/.test(css),
      };
    })
    .sort((a, b) => a.id.localeCompare(b.id));
}

/** A theme's colours, darkest first: the ramp a sprite is shaded along. */
export function rampOf(roles) {
  return [...new Set(Object.values(roles))].sort((a, b) => luminance(a) - luminance(b));
}

export const rgbOf = (hex) =>
  [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16));

/** sRGB weights — enough to put eight colours in order, not a colour space. */
function luminance(hex) {
  const [r, g, b] = rgbOf(hex);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
