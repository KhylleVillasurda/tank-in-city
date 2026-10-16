# Sprites

The drawing sizes for the artwork, derived from the engine's own constants so the
art and the rules cannot disagree. Read this before drawing anything; no code
guesses a size, and neither should you.

Verified against the source on 2026-10-16. If a constant below changes, this
table is stale — the numbers come from `game-core/src/map.rs`,
`game-core/src/bullet.rs` and `game-core/src/rules.rs`, and `subpixels`, `tile`
and `tankSize` are all exported to JavaScript in the constants block.

## The rule

**A tile is 16 px.** Every sprite size is a multiple of 8, and almost everything
is a multiple of 16.

**One art pixel = one world pixel.** Draw at 1x. The renderer already multiplies
by an internal `scale` of 2 (an 832x832 canvas for a 416x416 arena), so a 16 px
tile lands as 32 screen px on its own.

That 1x rule is not a style preference, it is what keeps the art consistent with
itself: one art pixel ends up exactly the 2 screen px the renderer already uses
for every outline. Draw 32 px tiles instead and your outlines come out at half
the weight of the HUD and the shield ring around them.

Display art only at **whole** scales — 2x, 3x, 4x — with `image-rendering:
pixelated`, which is already set in `web/app/globals.css`. A 1.5x scale is what
makes pixel art look broken.

## Sizes

| Sprite | Size | Multiples of the tile | Where the number comes from |
|---|---|---|---|
| brick, steel, water, tree | **16x16** | 1x1 | one tile |
| tank | **32x32** | 2x2 | `TANK_SIZE = 2 * TILE` |
| base | **32x32** | 2x2 | the base occupies cols 12–13, rows 23–24 |
| bullet | **4x4** | — | `BULLET_RADIUS = 2` px, so 4 px across |
| power-up | **16x16** | 1x1 | a drop sits on one tile |
| shield ring | **32x32** | 2x2 | tank-sized |
| spawn flash | **32x32** | 2x2 | tank-sized |
| explosion | **32x32** | 2x2 | tank-sized, so it covers what it killed |

Reference: the whole arena is 26x26 tiles, so 416x416 px, drawn into an 832x832
canvas.

## Tanks

Tanks are the one asset with real depth to them.

**32x32, four directions per tank.** Four enemy kinds (**Basic, Fast, Power,
Armour**) means **16 frames**, plus the player's tank.

Two details that are easy to get wrong:

- **Bricks need no damage stages.** The engine removes a whole 16 px brick tile in
  one hit — the original game's quarter-tile damage is not in our rules. One
  brick sprite is the complete set.
- **Armour changes colour as it loses hit points.** Either draw four armour
  frames, or keep that one tinted in code the way it is now. Decide before
  drawing, because it changes the frame count.

Animated sets: water wants **2 frames**. Everything else is fine at 1.

## Art rules

1. No anti-aliasing and no soft edges — hard pixels only.
2. 1 px outlines in the background colour, matching how tanks are drawn today.
3. Keep to a small fixed palette, 8–16 colours, or it stops reading as 8-bit.
4. Transparent PNGs.
5. Original art only, or CC0 or free-licensed work, with a row in `CREDITS.md`
   per plan §12. Plan §1 forbids ripping anything from the original game.

## Where the files go

The same pattern the sound already uses: masters live in `assets/sprites/`, and
the copies the browser fetches are served from `web/public/sprites/`.

## What the code change touches

Not just the art files. `web/game/render.ts` draws tanks and tiles with
`fillRect`, so `drawTank` and the tile loop become `drawImage`, plus
`ctx.imageSmoothingEnabled = false`. Sprites also arrive asynchronously, so the
renderer needs a ready-gate before its first frame or the opening frames draw
blank. Enemy type is currently shown by colour; with sprites it becomes shape, so
that choice in `enemyColor` goes away.
