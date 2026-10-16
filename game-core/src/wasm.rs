//! The browser bridge: a small C ABI over `World`.
//!
//! Deliberately not wasm-bindgen. The renderer only ever needs a flat integer
//! snapshot and the tile grid, so plain exports plus a linear-memory read on
//! the JS side keep the build to `cargo build --target wasm32-unknown-unknown`
//! with no extra tooling.

use crate::bullet::MAX_BULLETS;
use crate::map::{Tile, SUBPIXELS, TANK_SIZE, TILE, WORLD_H, WORLD_W};
use crate::powerups::PowerUp;
use crate::stage::MAX_SPAWNS;
use crate::tank::{Kind, MAX_ENEMIES};
use crate::{
    Input, State, World, INPUT_CONFIRM, INPUT_DOWN, INPUT_FIRE, INPUT_LEFT, INPUT_RIGHT, INPUT_UP,
    MAP_H, MAP_W, SNAP_BULLETS_OFFSET, SNAP_BULLET, SNAP_ENEMY, SNAP_HEADER, SNAP_LEN,
    SNAP_NO_DROP, SNAP_SPAWNS_OFFSET, SNAP_SPAWN, SNAP_TANKS_OFFSET, TICK_HZ,
};

static mut WORLD: Option<World> = None;
static mut SNAPSHOT: [i32; SNAP_LEN] = [0; SNAP_LEN];

/// Everything the renderer needs, so no number is duplicated in JavaScript.
/// See `CONSTANTS_FIELDS` in web/game/wasm.ts for the same order in TS.
static CONSTANTS: [i32; 45] = [
    TILE,
    TANK_SIZE,
    WORLD_W,
    WORLD_H,
    MAP_W as i32,
    MAP_H as i32,
    SUBPIXELS,
    TICK_HZ as i32,
    SNAP_LEN as i32,
    SNAP_HEADER as i32,
    SNAP_ENEMY as i32,
    SNAP_BULLET as i32,
    SNAP_SPAWN as i32,
    SNAP_TANKS_OFFSET as i32,
    SNAP_BULLETS_OFFSET as i32,
    SNAP_SPAWNS_OFFSET as i32,
    SNAP_NO_DROP,
    MAX_ENEMIES as i32,
    MAX_BULLETS as i32,
    MAX_SPAWNS as i32,
    INPUT_UP as i32,
    INPUT_DOWN as i32,
    INPUT_LEFT as i32,
    INPUT_RIGHT as i32,
    INPUT_FIRE as i32,
    State::Playing as i32,
    State::StageClear as i32,
    State::GameOver as i32,
    PowerUp::ExtraLife as i32,
    PowerUp::Shield as i32,
    PowerUp::Freeze as i32,
    PowerUp::DestroyAll as i32,
    PowerUp::Upgrade as i32,
    PowerUp::Speed as i32,
    PowerUp::SteelWalls as i32,
    Kind::Basic as i32,
    Kind::Fast as i32,
    Kind::Power as i32,
    Kind::Armor as i32,
    Tile::Brick as i32,
    Tile::Steel as i32,
    Tile::Water as i32,
    Tile::Tree as i32,
    Tile::Base as i32,
    // Appended, not inserted: the JavaScript side reads this array by position
    // through a parallel name list, so new values go on the end.
    INPUT_CONFIRM as i32,
];

fn world() -> &'static mut World {
    unsafe {
        (*std::ptr::addr_of_mut!(WORLD))
            .as_mut()
            .expect("tc_init must be called before stepping")
    }
}

/// Start (or restart) a run from `seed`.
#[no_mangle]
pub extern "C" fn tc_init(seed: u32) {
    unsafe {
        *std::ptr::addr_of_mut!(WORLD) = Some(World::new(seed));
    }
    render_snapshot();
}

/// Advance exactly one 60 Hz simulation step.
#[no_mangle]
pub extern "C" fn tc_step(input: u32) {
    world().step(Input::new(input));
    render_snapshot();
}

#[no_mangle]
pub extern "C" fn tc_snapshot_ptr() -> *const i32 {
    std::ptr::addr_of!(SNAPSHOT) as *const i32
}

#[no_mangle]
pub extern "C" fn tc_map_ptr() -> *const u8 {
    world().map.tiles.as_ptr()
}

#[no_mangle]
pub extern "C" fn tc_constants_ptr() -> *const i32 {
    CONSTANTS.as_ptr()
}

fn render_snapshot() {
    let buf = unsafe { &mut *std::ptr::addr_of_mut!(SNAPSHOT) };
    world().write_snapshot(buf);
}
