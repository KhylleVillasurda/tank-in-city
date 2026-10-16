// Checks what the pixel panel promises, with no browser in the loop.
//
// Two rules, from the brief: nothing on screen may be a colour outside the
// theme's palette, and every animation must be a frame animation of 4 to 12
// frames at 80 to 150ms a frame. The rules live in prose at the top of
// components/pixel/panel.css; this is the same thing a machine can read.
//
// The browser half of the story — that the pixels really come out palette-exact,
// and that frames step rather than interpolate — is a Playwright pass, since
// nothing here can see a screenshot. See docs/pixel-ui.md for that recipe. What
// this catches is drift: a colour typed into the wrong file, an easing curve, a
// sprite sheet that no longer matches its palette, a theme missing a role.
//
//   cd web && npm run check-pixel

import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { THEMES_DIR, rampOf, readThemes } from "./palettes.mjs";
import { renderSheet, sheetPath, sheetPixels, sheetUrl } from "./make-sprites.mjs";

const WEB = join(dirname(fileURLToPath(import.meta.url)), "..");

/** Files that must not name a colour of their own — the panel draws in roles. */
const ROLE_ONLY = [
  "components/pixel/panel.css",
  "components/PixelPanel.tsx",
  "components/PixelTheme.tsx",
  "app/ui/page.tsx",
];

/** Declarations only: a comment may talk about blur or a hex. */
const code = (text) =>
  text.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "");

function check(ok, what) {
  if (!ok) throw new Error(what);
}

try {
  const themes = readThemes();
  check(themes.length > 0, `no theme files in ${THEMES_DIR}`);
  const panel = code(readFileSync(join(THEMES_DIR, "panel.css"), "utf8"));

  // A theme fills every role, and no two themes disagree about the roles there are.
  const roles = Object.keys(themes[0].roles).sort();
  check(roles.length > 0, `${themes[0].file} fills no roles`);
  for (const theme of themes) {
    const filled = Object.keys(theme.roles).sort();
    check(
      filled.join() === roles.join(),
      `${theme.file} fills ${filled.join(", ")}, not ${roles.join(", ")}`,
    );
    check(rampOf(theme.roles).length >= 4, `${theme.file} has too few colours to shade a coin`);
  }

  // The panel asks for roles that exist. `sheet` is the one non-colour role.
  const asked = new Set([...panel.matchAll(/var\(--pixel-([a-z-]+)\)/g)].map(([, role]) => role));
  for (const role of asked) {
    check(role === "sheet" || roles.includes(role), `panel.css asks for --pixel-${role}, which no theme fills`);
  }

  // Exactly one theme is the default, the one a bare `.pixel` gets.
  const defaults = themes.filter((theme) => theme.fallsBack);
  check(
    defaults.length === 1,
    `${defaults.length} themes answer to the bare .pixel selector, and one should`,
  );

  // Nothing outside a theme names a colour, and nothing blends two of them.
  const palette = new Set(themes.flatMap((theme) => Object.values(theme.roles)));
  for (const file of ROLE_ONLY) {
    const text = code(readFileSync(join(WEB, file), "utf8"));
    for (const [hex] of text.matchAll(/#[0-9a-f]{3,8}\b/gi)) {
      check(palette.has(hex.toLowerCase()), `${file} names ${hex}, which is in no palette`);
    }
    const blend = text.match(/\b(?:rgba|hsla?)\(|gradient|\bblur\b|mix-blend|opacity\s*:|filter\s*:/i);
    check(!blend, `${file} uses ${blend?.[0]}, which is not a palette colour`);
  }

  // Every animation is frames: steps(), 4 to 12 of them, 80 to 150ms each.
  const keyframes = new Set([...panel.matchAll(/@keyframes\s+([\w-]+)/g)].map(([, name]) => name));
  const frames = [...panel.matchAll(/animation:\s*([^;]+);/g)]
    .map(([, value]) => value.trim())
    .filter((value) => value !== "none"); // reduced motion
  check(frames.length > 0, "panel.css runs no animation");
  for (const animation of frames) {
    const name = animation.match(/^[-\w]+/)?.[0];
    const duration = animation.match(/([\d.]+)(ms|s)\b/);
    const steps = animation.match(/steps\((\d+)/);
    check(keyframes.has(name), `animation ${name} has no @keyframes`);
    check(steps, `animation ${name} does not use steps()`);
    check(
      !/cubic-bezier|\bease\b|ease-in|ease-out|\blinear\b/.test(animation),
      `animation ${name} eases instead of stepping`,
    );
    const count = Number(steps?.[1]);
    const held = (Number(duration?.[1]) * (duration?.[2] === "s" ? 1000 : 1)) / count;
    check(count >= 4 && count <= 12, `${name} runs ${count} frames, and 4 to 12 are allowed`);
    check(held >= 80 && held <= 150, `${name} holds a frame ${held}ms, and 80 to 150 are allowed`);
  }

  // Each sheet is the one its theme asks for, painted from that theme's ramp,
  // and hand-drawn edits to a committed PNG are drift, not art.
  for (const theme of themes) {
    check(
      theme.sheet === sheetUrl(theme.id),
      `${theme.file} asks for ${theme.sheet}, not ${sheetUrl(theme.id)}`,
    );
    const ramp = rampOf(theme.roles);
    const stray = [...new Set(sheetPixels(ramp).filter(Boolean))].filter(
      (hex) => !Object.values(theme.roles).includes(hex),
    );
    check(stray.length === 0, `${theme.id}'s coin uses ${stray.join(", ")}, which is not in its palette`);
    check(existsSync(sheetPath(theme.id)), `${sheetUrl(theme.id)} is missing — run npm run sprites`);
    check(
      readFileSync(sheetPath(theme.id)).equals(renderSheet(ramp)),
      `${sheetUrl(theme.id)} has drifted from its palette — run npm run sprites`,
    );
  }

  console.log(
    `pixel ui ok: ${themes.length} themes (${defaults[0].id} by default), ` +
      `${frames.length} stepped animations, ${themes.length} sprite sheets`,
  );
} catch (error) {
  console.error(`pixel ui check failed: ${error.message}`);
  process.exitCode = 1;
}
