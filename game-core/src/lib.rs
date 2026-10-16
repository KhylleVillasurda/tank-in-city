//! Tank In City game rules.
//!
//! This crate is the whole game: rules, map, tanks, bullets, collisions.
//! It has no network, DOM, or database code (plan section 4), so it compiles
//! both to WebAssembly for the browser and to native code for the server.
//!
//! Everything runs on a fixed 60 Hz timestep with integer fixed-point
//! positions (plan Day 1, "Get these right from the very first commit").
//!
//! Milestone 2 scope: the Single Player endless arcade — enemy waves and
//! types, lives and respawn, the base, scoring, power-ups, and stage data.

pub mod ai;
pub mod bullet;
pub mod map;
pub mod powerups;
pub mod rng;
pub mod rules;
pub mod stage;
pub mod tank;

#[cfg(target_arch = "wasm32")]
mod wasm;

use ai::{FIRE_TICKS, TURN_TICKS};
use bullet::{Bullet, Hit, MAX_BULLETS};
use map::{Map, Tile, TILE};
use powerups::{Drop, PowerUp};
use rng::Rng;
use rules::{
    ENEMIES_PER_STAGE, EXTRA_LIFE_EVERY, INVULN_TICKS, RESPAWN_TICKS, SPAWN_FLASH_TICKS,
    START_LIVES, STEEL_WALLS_TICKS,
};
use stage::MAX_SPAWNS;
use tank::{Dir, Kind, Tank, MAX_ENEMIES, MAX_TANKS};

pub use rules::{MAP_H, MAP_W, TICK_HZ};

/// Room for the base's brick ring: a 2x2 base has 12 neighbours.
const MAX_STEEL_TILES: usize = 16;
/// Minimum gap between the player's shots. The in-flight limit does most of
/// the work; this stops key-mashing from feeling different.
pub const PLAYER_FIRE_COOLDOWN: u16 = 20;

pub const INPUT_UP: u32 = 1;
pub const INPUT_DOWN: u32 = 2;
pub const INPUT_LEFT: u32 = 4;
pub const INPUT_RIGHT: u32 = 8;
pub const INPUT_FIRE: u32 = 16;
/// Asks to move on from a stage-clear screen. Sent on the *press* of a key or a
/// click, once, so a player still holding fire does not skip the sound. The
/// engine ignores it outside `StageClear`.
pub const INPUT_CONFIRM: u32 = 32;

/// Where a run is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum State {
    Playing = 0,
    StageClear = 1,
    GameOver = 2,
}

/// One tick of player intent. The same shape the WebSocket `input` message
/// will carry in Milestone 3 (plan section 9).
#[derive(Clone, Copy)]
pub struct Input {
    pub bits: u32,
}

impl Input {
    pub fn new(bits: u32) -> Input {
        Input { bits }
    }

    /// The direction to drive, if any. Simultaneous keys resolve in the order
    /// up, down, left, right. The browser sends only the most recent key.
    pub fn dir(self) -> Option<Dir> {
        [
            (INPUT_UP, Dir::Up),
            (INPUT_DOWN, Dir::Down),
            (INPUT_LEFT, Dir::Left),
            (INPUT_RIGHT, Dir::Right),
        ]
        .into_iter()
        .find(|(bit, _)| self.bits & bit != 0)
        .map(|(_, dir)| dir)
    }

    pub fn fires(self) -> bool {
        self.bits & INPUT_FIRE != 0
    }
}

pub struct World {
    pub tick: u32,
    pub state: State,
    /// Stage numbers are 1-based, matching the stage files.
    pub stage: u32,
    pub score: i32,
    pub lives: u8,
    pub base_alive: bool,
    pub map: Map,
    pub player: Tank,
    /// Tile indices, not pixels.
    pub player_spawn: (u8, u8),
    pub respawn_timer: u8,

    pub enemies: [Tank; MAX_ENEMIES],
    pub enemy_count: usize,
    pub bullets: [Bullet; MAX_BULLETS],
    pub bullet_count: usize,

    /// Enemies in this stage that have not spawned yet.
    pub to_spawn: u8,
    /// Tile indices, not pixels.
    pub spawns: [(u8, u8); MAX_SPAWNS],
    pub spawn_count: usize,
    pub spawn_flash: [u8; MAX_SPAWNS],
    pub spawn_bonus: [bool; MAX_SPAWNS],

    pub drop: Option<Drop>,
    pub steel_timer: u16,

    pub rng: Rng,
    next_id: u8,
    next_extra_life: i32,
    /// Tile indices, not pixels. Sixteen tiles to remember costs 32 bytes as
    /// `u8` pairs and 128 as `i32` pairs.
    steel_tiles: [(u8, u8); MAX_STEEL_TILES],
    steel_count: usize,
    /// Fixed-point world units, the centre of the base.
    base_center: (i32, i32),
}

impl World {
    pub fn new(seed: u32) -> World {
        let mut world = World {
            tick: 0,
            state: State::Playing,
            stage: 1,
            score: 0,
            lives: START_LIVES,
            base_alive: true,
            map: Map::empty(),
            player: Tank::default(),
            player_spawn: (0, 0),
            respawn_timer: 0,
            enemies: [Tank::default(); MAX_ENEMIES],
            enemy_count: 0,
            bullets: [Bullet::default(); MAX_BULLETS],
            bullet_count: 0,
            to_spawn: ENEMIES_PER_STAGE,
            spawns: [(0, 0); MAX_SPAWNS],
            spawn_count: 0,
            spawn_flash: [0; MAX_SPAWNS],
            spawn_bonus: [false; MAX_SPAWNS],
            drop: None,
            steel_timer: 0,
            rng: Rng::new(seed),
            next_id: 1,
            next_extra_life: EXTRA_LIFE_EVERY,
            steel_tiles: [(0, 0); MAX_STEEL_TILES],
            steel_count: 0,
            base_center: (0, 0),
        };
        world.load_stage(1);
        world
    }

    /// Enemies left in the stage: not yet spawned plus still alive. At most 20
    /// plus the 4 that can be on the map, so a byte holds it.
    pub fn enemies_left(&self) -> u8 {
        self.to_spawn + self.enemy_count as u8
    }

    /// Advance the simulation by exactly one fixed step.
    pub fn step(&mut self, input: Input) {
        self.tick += 1;

        match self.state {
            State::GameOver => return,
            State::StageClear => {
                // The stage ends when the player says so, not on a timer: the
                // stage-clear sound runs 13.7 s and a 2 s auto-advance cut it
                // off. So the screen waits, and the player leaves when ready.
                if input.bits & INPUT_CONFIRM != 0 {
                    let next = self.stage % stage::COUNT as u32 + 1;
                    self.load_stage(next);
                }
                return;
            }
            State::Playing => {}
        }

        self.update_timers();
        self.update_spawns();
        if !self.player.alive && self.respawn_timer == 0 {
            self.try_respawn();
        }

        let n = self.enemy_count;
        let mut all = [Tank::default(); MAX_TANKS];
        all[0] = self.player;
        all[1..1 + n].copy_from_slice(&self.enemies[..n]);

        // Snapshot of everyone's box. Tanks move at most half a pixel per
        // tick, far too little to tunnel through a 32 px tank, so stale boxes
        // within one tick are fine.
        let mut rects = [(0, 0, 0, 0); MAX_TANKS];
        for (i, t) in all.iter().enumerate().take(1 + n) {
            if t.alive {
                rects[i] = t.rect();
            }
        }

        // --- Player ---
        if all[0].can_act() {
            if let Some(dir) = input.dir() {
                let speed = all[0].drive_speed();
                all[0].drive(&self.map, &rects, 0, dir, speed);
                rects[0] = all[0].rect();
            }
        }

        // --- Enemies ---
        for i in 0..n {
            let idx = 1 + i;
            if !all[idx].can_act() {
                continue;
            }
            let wants_fire = ai::update(&mut all[idx], &mut self.rng, self.base_center);
            let dir = all[idx].dir();
            let speed = all[idx].drive_speed();
            all[idx].drive(&self.map, &rects, idx, dir, speed);
            rects[idx] = all[idx].rect();
            if wants_fire && all[idx].cooldown == 0 && self.has_room_for_shot(&all[idx]) {
                self.add_bullet(Bullet::spawn(&all[idx]));
                all[idx].cooldown = rules::ENEMY_FIRE_COOLDOWN;
            }
        }

        // --- Firing ---
        if all[0].can_act()
            && all[0].invuln == 0
            && input.fires()
            && all[0].cooldown == 0
            && self.has_room_for_shot(&all[0])
        {
            self.add_bullet(Bullet::spawn(&all[0]));
            all[0].cooldown = PLAYER_FIRE_COOLDOWN;
        }

        // --- Bullets ---
        let mut i = 0;
        while i < self.bullet_count {
            let hit = self.bullets[i].advance(&mut self.map);
            let mut dead = hit != Hit::None;
            // Only an enemy shot brings the base down. The player spawns on the
            // base's row, so allowing its own shot to do it ended the run in
            // under a second from standstill. The shot still dies here, and
            // still breaks the ring brick in front of the base.
            if hit == Hit::Base && !self.bullets[i].is_player() {
                self.base_alive = false;
            }
            if !dead {
                let b = self.bullets[i];
                for (idx, target) in all.iter_mut().enumerate().take(1 + n) {
                    if !target.alive || b.same_side_as(target) {
                        continue;
                    }
                    if b.hits_rect(rects[idx]) {
                        if !target.protected() {
                            // Saturating: `hp` is unsigned, and a tank that is
                            // already on zero must not wrap to 255.
                            target.hp = target.hp.saturating_sub(1);
                        }
                        dead = true;
                        break;
                    }
                }
            }
            if dead {
                self.bullet_count -= 1;
                self.bullets[i] = self.bullets[self.bullet_count];
            } else {
                i += 1;
            }
        }

        // --- Power-up pickup: tanks drive over the drop ---
        if let Some(drop) = self.drop {
            let area = drop.rect();
            let picker = (0..=n).find(|idx| all[*idx].alive && tank_overlaps(rects[*idx], area));
            if let Some(owner) = picker {
                self.drop = None;
                self.apply_powerup(drop.kind, &mut all, n, owner);
            }
        }

        // --- Deaths ---
        for i in 0..n {
            let idx = 1 + i;
            if all[idx].alive && all[idx].hp == 0 {
                all[idx].alive = false;
                self.score += all[idx].kind().score();
                if all[idx].carries {
                    self.drop = Some(Drop {
                        kind: PowerUp::drop_for(&mut self.rng),
                        x: all[idx].x.div_euclid(TILE) * TILE,
                        y: all[idx].y.div_euclid(TILE) * TILE,
                    });
                }
            }
        }
        if all[0].alive && all[0].hp == 0 {
            all[0].alive = false;
            self.lives = self.lives.saturating_sub(1);
            if self.lives == 0 {
                self.state = State::GameOver;
            } else {
                self.respawn_timer = RESPAWN_TICKS;
            }
        }

        // Extra lives at fixed score thresholds (plan section 6).
        while self.score >= self.next_extra_life {
            self.lives += 1;
            self.next_extra_life += EXTRA_LIFE_EVERY;
        }

        self.player = all[0];
        let mut kept = 0;
        for i in 0..n {
            if all[1 + i].alive {
                self.enemies[kept] = all[1 + i];
                kept += 1;
            }
        }
        self.enemy_count = kept;

        // --- Stage and run outcomes ---
        if !self.base_alive {
            // In Single Player, losing the base ends the run.
            self.state = State::GameOver;
        } else if self.state == State::Playing
            && self.to_spawn == 0
            && self.enemy_count == 0
            && self.spawn_flash[..self.spawn_count].iter().all(|f| *f == 0)
        {
            self.state = State::StageClear;
        }
    }

    // --- stage setup ------------------------------------------------------

    fn load_stage(&mut self, number: u32) {
        let data = stage::load(number);
        self.stage = number;
        self.map = data.map();
        self.base_center = data.base_center;
        self.player_spawn = data.player_spawn;
        self.spawns = data.enemy_spawns;
        self.spawn_count = data.spawn_count;

        self.spawn_flash = [0; MAX_SPAWNS];
        self.spawn_bonus = [false; MAX_SPAWNS];
        self.to_spawn = ENEMIES_PER_STAGE;
        self.enemy_count = 0;
        self.enemies = [Tank::default(); MAX_ENEMIES];
        self.bullet_count = 0;
        self.drop = None;
        self.steel_timer = 0;
        self.steel_count = 0;
        self.base_alive = true;
        self.state = State::Playing;

        // A new stage resets power-ups and upgrades, but not lives or score.
        self.spawn_player(true);
    }

    fn spawn_player(&mut self, fresh: bool) {
        let (tx, ty) = self.player_spawn;
        let mut tank = Tank::spawn(Kind::Player, tx, ty, Dir::Up, self.stage, 0);
        if !fresh {
            tank.invuln = INVULN_TICKS;
        }
        self.player = tank;
        self.respawn_timer = 0;
    }

    /// Respawn next to the base, unless an enemy is sitting on the spot.
    fn try_respawn(&mut self) {
        // Tile indices in the World, world units in the maths below.
        let (tx, ty) = self.player_spawn;
        let (tx, ty) = (tx as i32, ty as i32);
        let spot = (tx * TILE, ty * TILE, (tx + 2) * TILE, (ty + 2) * TILE);
        if self.enemies[..self.enemy_count]
            .iter()
            .any(|e| tank_overlaps(e.rect(), spot))
        {
            self.respawn_timer = 1;
            return;
        }
        self.spawn_player(false);
    }

    // --- timers -----------------------------------------------------------

    fn update_timers(&mut self) {
        self.player.tick_timers();
        for i in 0..self.enemy_count {
            self.enemies[i].tick_timers();
        }
        self.respawn_timer = self.respawn_timer.saturating_sub(1);
        if self.steel_timer > 0 {
            self.steel_timer -= 1;
            if self.steel_timer == 0 {
                self.revert_steel_walls();
            }
        }
    }

    // --- waves ------------------------------------------------------------

    fn update_spawns(&mut self) {
        for slot in 0..self.spawn_count {
            if self.spawn_flash[slot] == 0 {
                continue;
            }
            self.spawn_flash[slot] -= 1;
            if self.spawn_flash[slot] == 0 {
                self.place_enemy(slot);
            }
        }

        // Start flashes while there is room for them among the live tanks.
        while self.to_spawn > 0 {
            let flashing = self.spawn_flash[..self.spawn_count]
                .iter()
                .filter(|f| **f > 0)
                .count();
            if self.enemy_count + flashing >= MAX_ENEMIES {
                break;
            }
            let Some(slot) = self.free_spawn_slot() else {
                break;
            };
            let number = ENEMIES_PER_STAGE - self.to_spawn + 1;
            self.spawn_bonus[slot] = rules::BONUS_ENEMIES.contains(&number);
            self.spawn_flash[slot] = SPAWN_FLASH_TICKS;
            self.to_spawn -= 1;
        }
    }

    fn free_spawn_slot(&self) -> Option<usize> {
        (0..self.spawn_count).find(|slot| {
            self.spawn_flash[*slot] == 0 && !self.spawn_point_blocked(*slot)
        })
    }

    fn spawn_point_blocked(&self, slot: usize) -> bool {
        // Tile indices in the World, world units in the maths below.
        let (tx, ty) = self.spawns[slot];
        let (tx, ty) = (tx as i32, ty as i32);
        let area = (tx * TILE, ty * TILE, (tx + 2) * TILE, (ty + 2) * TILE);
        if self.player.alive && tank_overlaps(self.player.rect(), area) {
            return true;
        }
        self.enemies[..self.enemy_count]
            .iter()
            .any(|e| tank_overlaps(e.rect(), area))
    }

    fn place_enemy(&mut self, slot: usize) {
        if self.enemy_count >= MAX_ENEMIES {
            self.requeue(slot);
            return;
        }
        // The player may have driven onto the point while it flashed.
        if self.spawn_point_blocked(slot) {
            self.requeue(slot);
            return;
        }

        let (tx, ty) = self.spawns[slot];
        let kind = Kind::pick(&mut self.rng, self.stage);
        let id = self.next_id;
        self.next_id = self.next_id % 250 + 1;
        let mut enemy = Tank::spawn(kind, tx, ty, Dir::Down, self.stage, id);
        enemy.carries = self.spawn_bonus[slot];
        enemy.turn_timer = self.rng.range(TURN_TICKS.0, TURN_TICKS.1) as u8;
        enemy.fire_timer = self.rng.range(FIRE_TICKS.0, FIRE_TICKS.1) as u16;
        self.spawn_bonus[slot] = false;
        self.enemies[self.enemy_count] = enemy;
        self.enemy_count += 1;
    }

    /// Give up on a spawn that could not happen, so the stage can still clear.
    fn requeue(&mut self, slot: usize) {
        self.spawn_flash[slot] = 0;
        self.spawn_bonus[slot] = false;
        self.to_spawn += 1;
    }

    // --- power-ups --------------------------------------------------------

    fn apply_powerup(
        &mut self,
        kind: PowerUp,
        all: &mut [Tank; MAX_TANKS],
        n: usize,
        owner: usize,
    ) {
        let by_player = owner == 0;
        match kind {
            // The player gains a life; an enemy survives one extra hit.
            PowerUp::ExtraLife => {
                if by_player {
                    self.lives += 1;
                } else {
                    all[owner].hp += 1;
                }
            }
            PowerUp::Shield => all[owner].shield = rules::SHIELD_TICKS,
            // Freezes the opposing side.
            PowerUp::Freeze => {
                if by_player {
                    for enemy in all[1..=n].iter_mut() {
                        if enemy.alive {
                            enemy.frozen = rules::FREEZE_TICKS;
                        }
                    }
                } else {
                    all[0].frozen = rules::FREEZE_TICKS;
                }
            }
            // Destroys every opposing tank on the map. A deliberate big swing.
            PowerUp::DestroyAll => {
                let targets: &mut [Tank] = if by_player { &mut all[1..=n] } else { &mut all[..1] };
                for target in targets {
                    if target.alive && !target.protected() {
                        target.hp = 0;
                    }
                }
            }
            PowerUp::Upgrade => all[owner].upgrade = (all[owner].upgrade + 1).min(2),
            PowerUp::Speed => all[owner].speed_boost = rules::SPEED_TICKS,
            // Reinforces the collector's own base. In Single Player the enemy
            // has no base, so for an enemy this just consumes the pickup.
            PowerUp::SteelWalls => {
                if by_player {
                    self.reinforce_base_walls();
                }
            }
        }
    }

    fn reinforce_base_walls(&mut self) {
        if self.steel_timer == 0 {
            self.steel_count = 0;
        }
        for ty in 0..MAP_H as i32 {
            for tx in 0..MAP_W as i32 {
                if self.map.get(tx, ty) != Tile::Brick || !self.map.touches_base(tx, ty) {
                    continue;
                }
                self.map.set(tx, ty, Tile::Steel);
                if self.steel_count < MAX_STEEL_TILES {
                    self.steel_tiles[self.steel_count] = (tx as u8, ty as u8);
                    self.steel_count += 1;
                }
            }
        }
        self.steel_timer = STEEL_WALLS_TICKS;
    }

    fn revert_steel_walls(&mut self) {
        for i in 0..self.steel_count {
            let (tx, ty) = self.steel_tiles[i];
            let (tx, ty) = (tx as i32, ty as i32);
            if self.map.get(tx, ty) == Tile::Steel {
                self.map.set(tx, ty, Tile::Brick);
            }
        }
        self.steel_count = 0;
    }

    // --- books ------------------------------------------------------------

    fn has_room_for_shot(&self, tank: &Tank) -> bool {
        self.bullets[..self.bullet_count]
            .iter()
            .filter(|b| b.owner == tank.id)
            .count()
            < tank.bullet_limit()
    }

    fn add_bullet(&mut self, b: Bullet) {
        if self.bullet_count < MAX_BULLETS {
            self.bullets[self.bullet_count] = b;
            self.bullet_count += 1;
        }
    }

    pub fn tile(&self, tx: i32, ty: i32) -> u8 {
        self.map.get(tx, ty) as u8
    }
}

fn tank_overlaps(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 < b.2 && a.2 > b.0 && a.1 < b.3 && a.3 > b.1
}

// --- Browser bridge -------------------------------------------------------
// Flat integer buffer the canvas renderer reads. Kept out of the engine above
// so the same rules stay usable from native code. Sections sit at fixed
// offsets, so the renderer needs no dynamic arithmetic; the counts in the
// header say how many records are live.

pub const SNAP_HEADER: usize = 23;
pub const SNAP_ENEMY: usize = 8;
pub const SNAP_BULLET: usize = 5;
pub const SNAP_SPAWN: usize = 3;
pub const SNAP_TANKS_OFFSET: usize = SNAP_HEADER;
pub const SNAP_BULLETS_OFFSET: usize = SNAP_TANKS_OFFSET + MAX_ENEMIES * SNAP_ENEMY;
pub const SNAP_SPAWNS_OFFSET: usize = SNAP_BULLETS_OFFSET + MAX_BULLETS * SNAP_BULLET;
pub const SNAP_LEN: usize = SNAP_SPAWNS_OFFSET + MAX_SPAWNS * SNAP_SPAWN;
/// `drop_kind` when nothing is lying on the map.
pub const SNAP_NO_DROP: i32 = -1;

impl World {
    pub fn write_snapshot(&self, out: &mut [i32]) {
        out[0] = self.tick as i32;
        out[1] = self.state as i32;
        out[2] = self.score;
        out[3] = self.lives as i32;
        out[4] = self.stage as i32;
        out[5] = self.enemies_left() as i32;
        out[6] = self.player.x;
        out[7] = self.player.y;
        out[8] = self.player.dir as i32;
        out[9] = self.player.alive as i32;
        out[10] = self.player.hp as i32;
        out[11] = self.player.upgrade as i32;
        out[12] = self.player.shield as i32;
        out[13] = self.player.speed_boost as i32;
        out[14] = self.player.frozen as i32;
        out[15] = self.player.invuln as i32;
        out[16] = self.base_alive as i32;
        out[17] = self.enemy_count as i32;
        out[18] = self.bullet_count as i32;
        out[19] = self.spawn_count as i32;
        out[20] = self.drop.map_or(SNAP_NO_DROP, |d| d.kind as i32);
        out[21] = self.drop.map_or(0, |d| d.x);
        out[22] = self.drop.map_or(0, |d| d.y);

        let mut o = SNAP_TANKS_OFFSET;
        for t in self.enemies[..self.enemy_count].iter() {
            out[o] = t.x;
            out[o + 1] = t.y;
            out[o + 2] = t.dir as i32;
            out[o + 3] = t.alive as i32;
            out[o + 4] = t.hp as i32;
            out[o + 5] = t.id as i32;
            out[o + 6] = t.kind as i32;
            out[o + 7] = t.carries as i32;
            o += SNAP_ENEMY;
        }

        o = SNAP_BULLETS_OFFSET;
        for b in self.bullets[..self.bullet_count].iter() {
            out[o] = b.x;
            out[o + 1] = b.y;
            out[o + 2] = b.dir as i32;
            out[o + 3] = b.speed;
            out[o + 4] = b.owner as i32;
            o += SNAP_BULLET;
        }

        o = SNAP_SPAWNS_OFFSET;
        for slot in 0..self.spawn_count {
            out[o] = self.spawns[slot].0 as i32;
            out[o + 1] = self.spawns[slot].1 as i32;
            out[o + 2] = self.spawn_flash[slot] as i32;
            o += SNAP_SPAWN;
        }
    }
}
