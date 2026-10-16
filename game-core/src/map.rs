//! The tile grid: Day 1 step 2 of the plan.
//!
//! Positions are fixed-point integers, never floats (plan section 4). One
//! pixel is `SUBPIXELS` units and one tile is `TILE` units, so every
//! coordinate in the engine is an `i32` count of 1/16 px.

use crate::rules::{MAP_H, MAP_W};

/// Fixed-point units per pixel.
pub const SUBPIXELS: i32 = 16;
pub const TILE_PX: i32 = 16;
/// Tile edge in fixed-point units.
pub const TILE: i32 = TILE_PX * SUBPIXELS;
/// Tanks are 2x2 tiles, as in the original.
pub const TANK_SIZE: i32 = 2 * TILE;
pub const WORLD_W: i32 = MAP_W as i32 * TILE;
pub const WORLD_H: i32 = MAP_H as i32 * TILE;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Tile {
    Empty = 0,
    Brick = 1,
    Steel = 2,
    Water = 3,
    Tree = 4,
    Base = 5,
}

impl Tile {
    pub fn from_u8(v: u8) -> Tile {
        match v {
            1 => Tile::Brick,
            2 => Tile::Steel,
            3 => Tile::Water,
            4 => Tile::Tree,
            5 => Tile::Base,
            _ => Tile::Empty,
        }
    }

    /// Tanks cannot drive through these. Water blocks tanks but not bullets.
    pub fn blocks_tank(self) -> bool {
        matches!(
            self,
            Tile::Brick | Tile::Steel | Tile::Water | Tile::Base
        )
    }

    /// Bullets die here. Trees are decoration; water lets bullets through.
    pub fn blocks_bullet(self) -> bool {
        matches!(self, Tile::Brick | Tile::Steel | Tile::Base)
    }
}

pub struct Map {
    pub tiles: [u8; MAP_W * MAP_H],
}

impl Map {
    pub fn get(&self, tx: i32, ty: i32) -> Tile {
        if tx < 0 || ty < 0 || tx >= MAP_W as i32 || ty >= MAP_H as i32 {
            // Outside the arena is solid rock, so nothing escapes the border.
            return Tile::Steel;
        }
        Tile::from_u8(self.tiles[ty as usize * MAP_W + tx as usize])
    }

    pub fn set(&mut self, tx: i32, ty: i32, tile: Tile) {
        if tx < 0 || ty < 0 || tx >= MAP_W as i32 || ty >= MAP_H as i32 {
            return;
        }
        self.tiles[ty as usize * MAP_W + tx as usize] = tile as u8;
    }

    pub fn blocks_tank(&self, tx: i32, ty: i32) -> bool {
        self.get(tx, ty).blocks_tank()
    }

    /// Where the base tiles are, for the AI and for the steel-wall power-up.
    pub fn touches_base(&self, tx: i32, ty: i32) -> bool {
        for dy in -1..=1 {
            for dx in -1..=1 {
                if self.get(tx + dx, ty + dy) == Tile::Base {
                    return true;
                }
            }
        }
        false
    }

    pub fn empty() -> Map {
        Map {
            tiles: [Tile::Empty as u8; MAP_W * MAP_H],
        }
    }
}
