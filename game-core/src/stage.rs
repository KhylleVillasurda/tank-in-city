//! Stage data: ASCII maps loaded from `assets/stages/`.
//!
//! The crate never touches the filesystem, so the maps are baked in with
//! `include_str!` and parsed at load time. That works identically on WASM and
//! native, and keeps a stage a plain text file anyone can edit.
//!
//! Legend:
//! ```text
//! .  empty      B  brick     S  steel     W  water    T  tree
//! #  base       P  player spawn (empty tile)        E  enemy spawn (empty tile)
//! ```
//!
//! A stage declares one `P` per player it can hold, in player order, up to
//! `MAX_PLAYERS`. The first is the Single Player spawn; Co-op uses the second.

use crate::map::{Map, Tile};
use crate::rules::{MAP_H, MAP_W};
use crate::tank::MAX_PLAYERS;

pub const MAX_SPAWNS: usize = 3;

/// One parsed stage.
pub struct Stage {
    pub tiles: [u8; MAP_W * MAP_H],
    /// Tile indices, not pixels: a 26x26 map needs 5 bits, so a byte each is
    /// already generous. Player order; the first is always present.
    pub player_spawns: [(u8, u8); MAX_PLAYERS],
    pub player_spawn_count: usize,
    pub enemy_spawns: [(u8, u8); MAX_SPAWNS],
    pub spawn_count: usize,
    /// Centre of the base in fixed-point world units, which is what bullets and
    /// the AI aim at — not a tile index.
    pub base_center: (i32, i32),
}

/// Fallback used if a file is missing a spawn marker, so a typo degrades the
/// stage instead of aborting the WASM module. `tests/stages.rs` is what keeps
/// the real files honest.
const FALLBACK_PLAYER_SPAWN: (u8, u8) = (6, (MAP_H - 3) as u8);

static SOURCES: [&str; 3] = [
    include_str!("../../assets/stages/stage1.txt"),
    include_str!("../../assets/stages/stage2.txt"),
    include_str!("../../assets/stages/stage3.txt"),
];

pub const COUNT: usize = 3;

/// Stage numbers are 1-based and wrap, so the endless run cycles the maps.
pub fn load(number: u32) -> Stage {
    let index = (number.max(1) - 1) as usize % COUNT;
    parse(SOURCES[index])
}

pub fn parse(text: &str) -> Stage {
    let mut tiles = [Tile::Empty as u8; MAP_W * MAP_H];
    let mut player_spawns = [(0, 0); MAX_PLAYERS];
    let mut player_spawn_count = 0;
    let mut enemy_spawns = [(0, 0); MAX_SPAWNS];
    let mut spawn_count = 0;

    for (ty, line) in text.lines().filter(|l| !l.trim().is_empty()).take(MAP_H).enumerate() {
        for (tx, ch) in line.trim_end().chars().take(MAP_W).enumerate() {
            let tile = match ch {
                'B' => Tile::Brick,
                'S' => Tile::Steel,
                'W' => Tile::Water,
                'T' => Tile::Tree,
                '#' => Tile::Base,
                _ => Tile::Empty,
            };
            tiles[ty * MAP_W + tx] = tile as u8;
            match ch {
                'P' if player_spawn_count < MAX_PLAYERS => {
                    player_spawns[player_spawn_count] = (tx as u8, ty as u8);
                    player_spawn_count += 1;
                }
                'E' if spawn_count < MAX_SPAWNS => {
                    enemy_spawns[spawn_count] = (tx as u8, ty as u8);
                    spawn_count += 1;
                }
                _ => {}
            }
        }
    }

    // A stage with no `P` at all still has to run: the first slot falls back,
    // and the count stays at 1 so `player_spawn` always has an answer.
    if player_spawn_count == 0 {
        player_spawns[0] = FALLBACK_PLAYER_SPAWN;
        player_spawn_count = 1;
    }

    let spawn_count = spawn_count.max(1);
    Stage {
        tiles,
        player_spawns,
        player_spawn_count,
        enemy_spawns,
        spawn_count,
        base_center: base_center(&tiles),
    }
}

fn base_center(tiles: &[u8; MAP_W * MAP_H]) -> (i32, i32) {
    let tile = crate::map::TILE;
    let mut sum = (0i64, 0i64);
    let mut count = 0i64;
    for (i, t) in tiles.iter().enumerate() {
        if *t == Tile::Base as u8 {
            sum.0 += (i % MAP_W) as i64;
            sum.1 += (i / MAP_W) as i64;
            count += 1;
        }
    }
    // Centre of the base in fixed-point world units, so bullets and AI have
    // something to aim at.
    if count == 0 {
        // This used to return tile indices while the path below returns world
        // units, so a stage with no base sent every enemy toward the top-left
        // corner. Same units either way now.
        return (
            MAP_W as i32 / 2 * tile + tile / 2,
            (MAP_H as i32 - 3) * tile + tile / 2,
        );
    }
    (
        (sum.0 / count) as i32 * tile + tile / 2,
        (sum.1 / count) as i32 * tile + tile / 2,
    )
}

impl Stage {
    pub fn map(&self) -> Map {
        Map { tiles: self.tiles }
    }

    /// Where player `i` starts. A stage that declares fewer `P`s than the match
    /// has players puts the extras on the last declared spawn.
    pub fn player_spawn(&self, i: usize) -> (u8, u8) {
        let last = self.player_spawn_count.saturating_sub(1);
        self.player_spawns[i.min(last)]
    }
}
