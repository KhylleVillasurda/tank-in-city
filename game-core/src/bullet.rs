//! Bullets.
//!
//! A bullet is a point plus a small radius. It moves in whole fixed-point
//! steps, so it can never tunnel through a 16 px tile or a 32 px tank.

use crate::map::{Map, Tile, TILE, WORLD_H, WORLD_W};
use crate::tank::{Dir, Rect, Tank};

/// A tank may have only 1 bullet in flight, 2 once fully upgraded, so this
/// is enough for the player plus four upgraded enemies.
pub const MAX_BULLETS: usize = 12;
/// Half-width of the bullet's collision box, in fixed-point units.
pub const BULLET_RADIUS: i32 = 2 * crate::map::SUBPIXELS;

/// What stopped a bullet this tick.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hit {
    /// Still flying.
    None,
    /// Steel, water, trees, the arena edge, or another bullet's wall.
    Wall,
    /// A brick, which is now gone.
    Brick,
    /// The base. The caller decides what that means.
    Base,
}

#[derive(Clone, Copy, Default)]
pub struct Bullet {
    /// Centre of the bullet.
    pub x: i32,
    pub y: i32,
    pub dir: u8,
    /// Fixed-point units per tick, from the firing tank's type and upgrades.
    pub speed: i32,
    /// `Tank::id` of the tank that fired it, so it cannot hit its owner.
    pub owner: u8,
}

impl Bullet {
    pub fn spawn(t: &Tank) -> Bullet {
        let (cx, cy) = t.center();
        let (dx, dy) = t.dir().delta();
        let reach = crate::map::TANK_SIZE / 2;
        Bullet {
            x: cx + dx * reach,
            y: cy + dy * reach,
            dir: t.dir as u8,
            speed: t.bullet_speed(),
            owner: t.id,
        }
    }

    /// Move one tick and report what it ran into.
    pub fn advance(&mut self, map: &mut Map) -> Hit {
        let (dx, dy) = Dir::from_u8(self.dir).delta();
        for _ in 0..self.speed {
            self.x += dx;
            self.y += dy;
            if self.x < 0 || self.y < 0 || self.x >= WORLD_W || self.y >= WORLD_H {
                return Hit::Wall;
            }
            let (tx, ty) = (self.x.div_euclid(TILE), self.y.div_euclid(TILE));
            match map.get(tx, ty) {
                Tile::Brick => {
                    map.set(tx, ty, Tile::Empty);
                    return Hit::Brick;
                }
                Tile::Base => return Hit::Base,
                t if t.blocks_bullet() => return Hit::Wall,
                _ => {}
            }
        }
        Hit::None
    }

    pub fn hits_rect(&self, r: Rect) -> bool {
        self.x + BULLET_RADIUS > r.0
            && self.x - BULLET_RADIUS < r.2
            && self.y + BULLET_RADIUS > r.1
            && self.y - BULLET_RADIUS < r.3
    }

    /// True when the player fired it. 0 is the player, 1+ are enemies.
    pub fn is_player(&self) -> bool {
        self.owner == 0
    }

    /// True when `tank` is on the bullet's own side. 0 is the player, anything
    /// else is an enemy.
    pub fn same_side_as(&self, tank: &Tank) -> bool {
        (self.owner == 0) == (tank.id == 0)
    }
}
