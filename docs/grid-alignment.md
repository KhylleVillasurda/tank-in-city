# Grid alignment, and why turning nudges the tank

A design note, written after a player reported "if I ran up then switched to side
the tank seemingly goes a bit down too". The behaviour is deliberate, and the
code that produces it is easy to get wrong in a way that looks like a different
bug — that mistake was already made once. This file is here so the next person
to touch it does not have to re-derive it.

**Short version.** A tank is exactly 2 tiles wide and a doorway is exactly 2
tiles, so a tank only passes through one when it sits exactly on the tile grid.
Turning from anywhere else therefore corrects the off-axis coordinate by up to
half a tile. That correction is one frame, one step, and it is the only time a
tank changes an axis it is not driving on.

See also [Milestone 2](milestone-2.md) — "Behaviour worth knowing" for the rule,
and its verification section for the bug report.

## The constraint

Units are 1/16 px, so 16 units = 1 px.

| | units | px |
|---|---|---|
| `TILE` (one brick) | 256 | 16 |
| `TANK_SIZE` (a tank, 2x2 tiles) | 512 | 32 |
| a 2-tile doorway | 512 | 32 |
| half a tile | 128 | 8 |

The tank and the gap are the same size. The tank fits only at offset 0 within
the gap; half a tile out of line and it is 128 units too wide for the hole.

That kills the obvious idea of aligning to a *finer* grid. A half-tile grid
would halve the correction (4 px instead of 8 px) but would let a tank rest 128
units out of line, where it can never enter the doorway it is aiming at. The
tile grid is the only grid that works, so the correction can be up to
`TILE / 2`.

Consequence: a tank sitting between two tile edges must be corrected *somehow*
once it moves along the other axis. The only choices are when and how fast.

## The rule as built — `game-core/src/tank.rs`

- `drive()` calls `snap_to_grid()` for the off-axis before travelling.
- The snap is **one step**, not eased, and when it moves, `drive()` returns
  without travelling that tick.
- Invariant: **a tank's position changes on at most one axis per tick.** So
  movement is only ever up, down, left or right.
- The target is the **nearest** tile edge, so the correction is at most
  `TILE / 2` = 128 units = 8 px — a quarter of the tank.
- A snap that would collide is skipped, and travel proceeds normally. A tank
  wedged in a corner therefore cannot deadlock.

```rust
if self.snap_to_grid(map, rects, me, dx != 0) {
    self.blocked = false;
    return;                  // this tick lined up; do not also travel
}
```

## The bug this replaced

The first version nudged the off-axis at up to `speed` units **per tick while
still driving**. The tank therefore moved on both axes for the ~7 ticks it took
to cover the 84-unit correction, which read as diagonal movement — exactly what
the player reported, alongside the "goes a bit down" half.

**Never ease this.** One step, on a tick of its own, is the whole fix. Two tests
hold it: `a_tank_moves_on_one_axis_at_a_time` counts ticks where both axes moved
and requires zero, and `turning_after_driving_up_lines_up_in_one_tick` requires
the snap to happen on the first tick of the new direction and never again.

## Why nearest, and not "round forward"

Rounding in the direction of travel (or away from it) permits a correction of up
to a full tile, 256 units — 16 px, twice as visible. Nearest is the
minimum-magnitude choice, which is why it is the one used.

The price: nearest can move the tank *backwards*. Drive up, stop between edges,
turn left, and the tank may drop up to 8 px — it is undoing part of the up-move.
This is the "a bit down" the player noticed, now bounded to 8 px in a single
frame instead of a drift.

## Still open: the 8 px click

Deliberately unchanged, pending a report on how it feels to play. Options, in
increasing order of how much they change:

1. **Accept it** (current). The genre ratio is the same: the original corrects up
   to 4 px on a 16 px tank, also instantaneously — the same quarter of the tank.
2. **Settle on stop.** When no direction is held, snap both axes. Removes the
   click from "stop, then turn", which is the common case. Costs the tank
   visibly sliding up to 8 px on its own when you release the key, and a direct
   up-to-left without releasing still clicks.
3. **Snap only when blocked.** Moves the correction to the doorway. The hop
   reads worse there (a sideways jump as you hit a wall), and gap entry can fail
   outright when the snap is blocked.
4. **Finer grid.** Not an option — see the constraint above.

If it is revisited, the only lines that matter are the snap target in
`Tank::snap_to_grid` and `TILE` in `map.rs`.

## Looking at it without a browser

The rule is visible in the built wasm, tick by tick. From `web/`:

```bash
node -e '
import("node:fs/promises").then(async (fs) => {
  const { instance } = await WebAssembly.instantiate(
    await fs.readFile("public/game_core.wasm"), {});
  const x = instance.exports;
  const raw = new Int32Array(x.memory.buffer, x.tc_constants_ptr(), 44);
  const s = () => new Int32Array(x.memory.buffer, x.tc_snapshot_ptr(), raw[8]);
  const row = (l) => console.log(l.padEnd(8), "x=" + s()[6], "y=" + s()[7], "y%256=" + (s()[7] % 256));
  x.tc_init(2444195321);
  for (let i = 0; i < 7; i++) x.tc_step(raw[20]);          // up
  row("up x7");
  for (let i = 0; i < 3; i++) { x.tc_step(raw[22]); row("left"); }
});'
```

`y` moves on exactly one tick (the first `left`), then stays put while `x` walks
left. Before the fix, `y` closed by 12 every tick alongside `x` — diagonal.
