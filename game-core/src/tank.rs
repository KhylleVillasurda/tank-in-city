//! Tanks, tank types, and tank collisions.

use crate::map::{Map, TANK_SIZE, TILE, WORLD_H, WORLD_W};
use crate::rng::Rng;

/// Up to 4 enemies on the map at once (plan section 6, Enemy waves).
pub const MAX_ENEMIES: usize = 4;
/// The player plus the enemies.
pub const MAX_TANKS: usize = 1 + MAX_ENEMIES;
/// Armour tops out here however deep the run goes.
pub const MAX_ARMOR_HP: u8 = 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Dir {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }

    pub fn from_u8(v: u8) -> Dir {
        match v {
            1 => Dir::Down,
            2 => Dir::Left,
            3 => Dir::Right,
            _ => Dir::Up,
        }
    }

    /// The direction that gets `from` closer to `to` on the dominant axis.
    pub fn toward(from: (i32, i32), to: (i32, i32)) -> Dir {
        if (to.0 - from.0).abs() > (to.1 - from.1).abs() {
            if to.0 < from.0 {
                Dir::Left
            } else {
                Dir::Right
            }
        } else if to.1 < from.1 {
            Dir::Up
        } else {
            Dir::Down
        }
    }
}

/// Tank types (plan section 6, Enemy types). Speeds are fixed-point units per
/// tick: 16 units is one pixel, so 8 is 0.5 px/tick or 30 px/s.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Kind {
    Player = 0,
    /// Slow, 1 hit.
    Basic = 1,
    /// Quick, 1 hit.
    Fast = 2,
    /// Fast bullets, 1 hit.
    Power = 3,
    /// Takes several hits, more in later stages.
    Armor = 4,
}

impl Kind {
    pub fn from_u8(v: u8) -> Kind {
        match v {
            1 => Kind::Basic,
            2 => Kind::Fast,
            3 => Kind::Power,
            4 => Kind::Armor,
            _ => Kind::Player,
        }
    }

    pub fn is_enemy(self) -> bool {
        self != Kind::Player
    }

    pub fn speed(self) -> i32 {
        match self {
            Kind::Player => 12,
            Kind::Basic => 8,
            Kind::Fast => 16,
            Kind::Power => 10,
            Kind::Armor => 8,
        }
    }

    pub fn bullet_speed(self) -> i32 {
        match self {
            Kind::Power => 48,
            _ => 32,
        }
    }

    /// Points for destroying one, per plan section 6, Scoring.
    pub fn score(self) -> i32 {
        match self {
            Kind::Basic => 100,
            Kind::Fast => 200,
            Kind::Power => 300,
            Kind::Armor => 400,
            Kind::Player => 0,
        }
    }

    /// Armour takes 4 hits by default, more in later stages, and no more
    /// than `MAX_ARMOR_HP`.
    pub fn hp(self, stage: u32) -> u8 {
        match self {
            Kind::Armor => (4 + stage.saturating_sub(1) / 2).min(MAX_ARMOR_HP as u32) as u8,
            _ => 1,
        }
    }

    /// Difficulty rises with the stage number: fewer Basics, more Fast,
    /// Power and Armour tanks. Weights to tune.
    pub fn pick(rng: &mut Rng, stage: u32) -> Kind {
        let weights = [4, 1 + stage / 2, 1 + stage / 3, stage / 2];
        let mut roll = rng.below(weights.iter().sum());
        for (i, weight) in weights.iter().enumerate() {
            if roll < *weight {
                return Kind::from_u8(i as u8 + 1);
            }
            roll -= weight;
        }
        Kind::Basic
    }
}

/// Axis-aligned box, `(x0, y0, x1, y1)` in fixed-point units.
pub type Rect = (i32, i32, i32, i32);

/// Tick counters carry the narrowest type that holds their longest value, taken
/// from `rules.rs`: anything bounded by 255 is a `u8`, and the power-up
/// durations, which reach 900, are `u16`. Positions stay `i32` on purpose — the
/// collision path does arithmetic on them in the hottest loop in the engine, and
/// wasm has no 16-bit registers, so narrowing those would add sign-extension to
/// save bytes nobody is short of.
#[derive(Clone, Copy)]
pub struct Tank {
    /// Stable id, so a bullet in flight still knows its owner after the
    /// enemy list is compacted. 0 is always the player.
    pub id: u8,
    pub kind: u8,
    pub x: i32,
    pub y: i32,
    pub dir: u8,
    pub alive: bool,
    /// Hits left. Armour tops out at `MAX_ARMOR_HP`, everything else takes 1.
    pub hp: u8,
    /// Ticks until this tank may fire again.
    pub cooldown: u16,
    /// Set by `drive`; the AI uses it to pick a new direction when stuck.
    pub blocked: bool,
    /// AI: ticks until it changes direction (unused by the player).
    pub turn_timer: u8,
    /// AI: ticks until it tries to fire (unused by the player).
    pub fire_timer: u16,
    /// 0 = plain, 1 = faster bullets, 2 = also a second shot in flight.
    pub upgrade: u8,
    /// Power-up shield. Blocks damage, still allowed to fire.
    pub shield: u16,
    /// Respawn invulnerability. Blocks damage, and blocks firing.
    pub invuln: u8,
    pub speed_boost: u16,
    /// Frozen by the opposing side's Freeze power-up.
    pub frozen: u16,
    /// A bonus enemy: flashes, and drops a power-up when destroyed.
    pub carries: bool,
}

impl Default for Tank {
    fn default() -> Self {
        Tank {
            id: 0,
            kind: Kind::Player as u8,
            x: 0,
            y: 0,
            dir: Dir::Up as u8,
            alive: false,
            hp: 1,
            cooldown: 0,
            blocked: false,
            turn_timer: 0,
            fire_timer: 0,
            upgrade: 0,
            shield: 0,
            invuln: 0,
            speed_boost: 0,
            frozen: 0,
            carries: false,
        }
    }
}

impl Tank {
    pub fn spawn(kind: Kind, tx: u8, ty: u8, dir: Dir, stage: u32, id: u8) -> Tank {
        Tank {
            id,
            kind: kind as u8,
            x: tx as i32 * TILE,
            y: ty as i32 * TILE,
            dir: dir as u8,
            alive: true,
            hp: kind.hp(stage),
            ..Tank::default()
        }
    }

    pub fn kind(&self) -> Kind {
        Kind::from_u8(self.kind)
    }

    pub fn dir(&self) -> Dir {
        Dir::from_u8(self.dir)
    }

    pub fn center(&self) -> (i32, i32) {
        (self.x + TANK_SIZE / 2, self.y + TANK_SIZE / 2)
    }

    pub fn rect(&self) -> Rect {
        (self.x, self.y, self.x + TANK_SIZE, self.y + TANK_SIZE)
    }

    /// A fully upgraded tank may have 2 bullets in flight, otherwise 1.
    pub fn bullet_limit(&self) -> usize {
        if self.upgrade >= 2 {
            2
        } else {
            1
        }
    }

    pub fn bullet_speed(&self) -> i32 {
        self.kind().bullet_speed() + if self.upgrade >= 1 { 16 } else { 0 }
    }

    pub fn drive_speed(&self) -> i32 {
        let base = self.kind().speed();
        if self.speed_boost > 0 {
            base * 3 / 2
        } else {
            base
        }
    }

    /// Shield or respawn invulnerability: damage does nothing.
    pub fn protected(&self) -> bool {
        self.shield > 0 || self.invuln > 0
    }

    /// Alive and not frozen.
    pub fn can_act(&self) -> bool {
        self.alive && self.frozen == 0
    }

    pub fn tick_timers(&mut self) {
        self.cooldown = self.cooldown.saturating_sub(1);
        self.shield = self.shield.saturating_sub(1);
        self.invuln = self.invuln.saturating_sub(1);
        self.speed_boost = self.speed_boost.saturating_sub(1);
        self.frozen = self.frozen.saturating_sub(1);
    }

    /// Drive `speed` fixed-point units in `dir`. Tanks are solid, so a tank
    /// that runs into a wall or another tank stops flush against it.
    pub fn drive(&mut self, map: &Map, rects: &[Rect], me: usize, dir: Dir, speed: i32) {
        self.dir = dir as u8;
        let (dx, dy) = dir.delta();

        // Line up on the axis we are not driving on, so the tank slips into
        // gaps instead of snagging on their corners. Done in a single step and
        // on its own tick: easing it over several ticks moved the tank on both
        // axes at once, which read as diagonal movement, and tanks only ever
        // move in the four directions.
        if self.snap_to_grid(map, rects, me, dx != 0) {
            self.blocked = false;
            return;
        }

        let mut blocked = false;
        for _ in 0..speed {
            let (nx, ny) = (self.x + dx, self.y + dy);
            if self.hits(map, rects, me, nx, ny) {
                blocked = true;
                break;
            }
            self.x = nx;
            self.y = ny;
        }
        self.blocked = blocked;
    }

    /// Move the off-axis coordinate onto the nearest tile edge, in one step.
    /// Returns true if it moved, which means the caller should not also travel
    /// this tick. `axis_y` is true when driving horizontally.
    fn snap_to_grid(&mut self, map: &Map, rects: &[Rect], me: usize, axis_y: bool) -> bool {
        let cur = if axis_y { self.y } else { self.x };
        let target = (cur + TILE / 2).div_euclid(TILE) * TILE;
        if target == cur {
            return false;
        }
        let (nx, ny) = if axis_y {
            (self.x, target)
        } else {
            (target, self.y)
        };
        if self.hits(map, rects, me, nx, ny) {
            return false;
        }
        self.x = nx;
        self.y = ny;
        true
    }

    /// Would a tank at `(nx, ny)` overlap a solid tile or another tank?
    pub fn hits(&self, map: &Map, rects: &[Rect], me: usize, nx: i32, ny: i32) -> bool {
        if nx < 0 || ny < 0 || nx + TANK_SIZE > WORLD_W || ny + TANK_SIZE > WORLD_H {
            return true;
        }
        let tx0 = nx.div_euclid(TILE);
        let ty0 = ny.div_euclid(TILE);
        let tx1 = (nx + TANK_SIZE - 1).div_euclid(TILE);
        let ty1 = (ny + TANK_SIZE - 1).div_euclid(TILE);
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                if map.blocks_tank(tx, ty) {
                    return true;
                }
            }
        }
        let r = (nx, ny, nx + TANK_SIZE, ny + TANK_SIZE);
        for (i, other) in rects.iter().enumerate() {
            if i != me && overlaps(r, *other) {
                return true;
            }
        }
        false
    }
}

pub fn overlaps(a: Rect, b: Rect) -> bool {
    a.0 < b.2 && a.2 > b.0 && a.1 < b.3 && a.3 > b.1
}
