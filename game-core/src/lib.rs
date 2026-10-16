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
use tank::{Dir, Kind, Tank, MAX_ENEMIES, MAX_PLAYERS, MAX_TANKS, TEAM_A};

pub use rules::{MAP_H, MAP_W, TICK_HZ};

/// Room for the base's brick ring: a 2x2 base has 12 neighbours.
const MAX_STEEL_TILES: usize = 16;
/// Enemy ids start here, so no enemy id can ever collide with a player's.
const FIRST_ENEMY_ID: u8 = MAX_PLAYERS as u8;
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
/// carries in Milestone 3 (plan section 9).
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
    pub base_alive: bool,
    pub map: Map,

    /// The players, slots `0..player_count`. Single Player and Co-op put every
    /// one of them on the same side.
    pub players: [Tank; MAX_PLAYERS],
    pub player_count: usize,
    /// Per player, indexed with `players`: what survives a respawn. Positions
    /// stay `i32` here for the same reason they do in `Tank`.
    pub player_lives: [u8; MAX_PLAYERS],
    pub player_scores: [i32; MAX_PLAYERS],
    /// Ticks until that player respawns; 0 while alive, or out of lives.
    pub player_respawn: [u8; MAX_PLAYERS],
    /// Tile indices, not pixels. One per player, from the stage's `P` markers.
    pub player_spawns: [(u8, u8); MAX_PLAYERS],

    /// Whether the AI wave system runs at all: Single Player and Co-op.
    pub waves: bool,
    /// Whether a shot can hurt its own side (plan section 6, Friendly fire).
    pub friendly_fire: bool,

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
    next_extra_life: [i32; MAX_PLAYERS],
    /// Tile indices, not pixels. Sixteen tiles to remember costs 32 bytes as
    /// `u8` pairs and 128 as `i32` pairs.
    steel_tiles: [(u8, u8); MAX_STEEL_TILES],
    steel_count: usize,
    /// Fixed-point world units, the centre of the base.
    base_center: (i32, i32),
}

/// What the engine needs to know about a mode. Everything else in plan
/// section 6 — rounds, best-of-3, the time limit, sudden death, spectating,
/// scoring — belongs to the match loop, not to the simulation.
#[derive(Clone, Copy)]
pub struct Setup {
    /// How many players. 1 is Single Player, 2 is Co-op or 1v1, up to 6.
    pub players: u8,
    /// Whether the AI wave system runs. Single Player and Co-op only.
    pub waves: bool,
    /// Whether a shot can hurt its own side. Free For All always says yes,
    /// which falls out of giving each player a side of their own.
    pub friendly_fire: bool,
    /// Which side each player is on, in slot order. Team modes share a side.
    pub teams: [u8; MAX_PLAYERS],
}

impl Setup {
    /// Single Player: one player on side A, facing the AI.
    pub fn single() -> Setup {
        Setup {
            players: 1,
            waves: true,
            friendly_fire: false,
            teams: [TEAM_A; MAX_PLAYERS],
        }
    }
}

impl World {
    /// A Single Player run: one player, facing the AI.
    pub fn new(seed: u32) -> World {
        World::with_setup(seed, Setup::single())
    }

    pub fn with_setup(seed: u32, setup: Setup) -> World {
        let player_count = (setup.players.max(1) as usize).min(MAX_PLAYERS);
        let mut world = World {
            tick: 0,
            state: State::Playing,
            stage: 1,
            base_alive: true,
            map: Map::empty(),
            players: [Tank::default(); MAX_PLAYERS],
            player_count,
            player_lives: [START_LIVES; MAX_PLAYERS],
            player_scores: [0; MAX_PLAYERS],
            player_respawn: [0; MAX_PLAYERS],
            player_spawns: [(0, 0); MAX_PLAYERS],
            waves: setup.waves,
            friendly_fire: setup.friendly_fire,
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
            next_id: FIRST_ENEMY_ID,
            next_extra_life: [EXTRA_LIFE_EVERY; MAX_PLAYERS],
            steel_tiles: [(0, 0); MAX_STEEL_TILES],
            steel_count: 0,
            base_center: (0, 0),
        };
        // The side has to be in place before the first spawn, because
        // `spawn_player` carries forward the team it finds on the slot.
        for i in 0..player_count {
            world.players[i].team = setup.teams[i];
        }
        world.load_stage(1);
        world
    }

    /// The whole run's score, which is what the Single Player HUD shows.
    pub fn score(&self) -> i32 {
        self.player_scores[..self.player_count].iter().sum()
    }

    /// Enemies left in the stage: not yet spawned plus still alive. At most 20
    /// plus the 4 that can be on the map, so a byte holds it.
    pub fn enemies_left(&self) -> u8 {
        self.to_spawn + self.enemy_count as u8
    }

    /// Advance the simulation by exactly one fixed step. `inputs` holds one
    /// entry per player, in slot order; a player with no entry gets no input.
    pub fn step(&mut self, inputs: &[Input]) {
        self.tick += 1;

        match self.state {
            State::GameOver => return,
            State::StageClear => {
                // The stage ends when a player says so, not on a timer: the
                // stage-clear sound runs 13.7 s and a 2 s auto-advance cut it
                // off. So the screen waits, and a player leaves when ready.
                if inputs.iter().any(|i| i.bits & INPUT_CONFIRM != 0) {
                    let next = self.stage % stage::COUNT as u32 + 1;
                    self.load_stage(next);
                }
                return;
            }
            State::Playing => {}
        }

        self.update_timers();
        self.update_spawns();
        for i in 0..self.player_count {
            if !self.players[i].alive && self.player_lives[i] > 0 && self.player_respawn[i] == 0 {
                self.try_respawn(i);
            }
        }

        let p = self.player_count;
        let n = self.enemy_count;
        let mut all = [Tank::default(); MAX_TANKS];
        all[..p].copy_from_slice(&self.players[..p]);
        all[p..p + n].copy_from_slice(&self.enemies[..n]);

        // Snapshot of everyone's box. Tanks move at most half a pixel per
        // tick, far too little to tunnel through a 32 px tank, so stale boxes
        // within one tick are fine.
        let mut rects = [(0, 0, 0, 0); MAX_TANKS];
        for (i, t) in all.iter().enumerate().take(p + n) {
            if t.alive {
                rects[i] = t.rect();
            }
        }

        // --- Players: each drives and fires on its own input ---
        for i in 0..p {
            if !all[i].can_act() {
                continue;
            }
            let input = inputs.get(i).copied().unwrap_or(Input::new(0));
            if let Some(dir) = input.dir() {
                let speed = all[i].drive_speed();
                all[i].drive(&self.map, &rects, i, dir, speed);
                rects[i] = all[i].rect();
            }
            // A tank with respawn invulnerability cannot fire (plan section 6).
            if all[i].invuln == 0
                && input.fires()
                && all[i].cooldown == 0
                && self.has_room_for_shot(&all[i])
            {
                self.add_bullet(Bullet::spawn(&all[i]));
                all[i].cooldown = PLAYER_FIRE_COOLDOWN;
            }
        }

        // --- Enemies ---
        for i in 0..n {
            let idx = p + i;
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

        // --- Bullets ---
        let mut i = 0;
        while i < self.bullet_count {
            let hit = self.bullets[i].advance(&mut self.map);
            let mut dead = hit != Hit::None;
            // The base belongs to side A, so only a shot from another side
            // brings it down. The player spawns on the base's row, so allowing
            // its own shot to do it ended the run in under a second from
            // standstill. The shot still dies here, and still breaks the ring
            // brick in front of the base.
            if hit == Hit::Base && self.bullets[i].team != TEAM_A {
                self.base_alive = false;
            }
            if !dead {
                let b = self.bullets[i];
                let friendly = self.friendly_fire;
                for (idx, target) in all.iter_mut().enumerate().take(p + n) {
                    // Never its own tank, even with friendly fire on.
                    if !target.alive || b.owner == target.id {
                        continue;
                    }
                    if !friendly && b.same_side_as(target) {
                        continue;
                    }
                    if b.hits_rect(rects[idx]) {
                        if !target.protected() {
                            // Saturating: `hp` is unsigned, and a tank that is
                            // already on zero must not wrap to 255.
                            target.hp = target.hp.saturating_sub(1);
                            target.last_hit_by = b.owner;
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
            let picker = (0..p + n).find(|idx| all[*idx].alive && tank_overlaps(rects[*idx], area));
            if let Some(owner) = picker {
                self.drop = None;
                self.apply_powerup(drop.kind, &mut all, p, n, owner);
            }
        }

        // --- Deaths ---
        for i in 0..n {
            let idx = p + i;
            if all[idx].alive && all[idx].hp == 0 {
                all[idx].alive = false;
                let points = all[idx].kind().score();
                self.credit_kill(all[idx].last_hit_by, points);
                if all[idx].carries {
                    self.drop = Some(Drop {
                        kind: PowerUp::drop_for(&mut self.rng),
                        x: all[idx].x.div_euclid(TILE) * TILE,
                        y: all[idx].y.div_euclid(TILE) * TILE,
                    });
                }
            }
        }
        for (i, tank) in all.iter_mut().enumerate().take(p) {
            if tank.alive && tank.hp == 0 {
                tank.alive = false;
                self.player_lives[i] = self.player_lives[i].saturating_sub(1);
                // Out of lives is out of the match: the tank stays down, and
                // never respawns. Watching the rest is the match loop's job.
                if self.player_lives[i] > 0 {
                    self.player_respawn[i] = RESPAWN_TICKS;
                }
            }
        }
        // The run ends when every player is out (plan section 6, elimination).
        if self.player_lives[..p].iter().all(|l| *l == 0) {
            self.state = State::GameOver;
        }

        // Extra lives at fixed score thresholds, per player (plan section 6).
        for i in 0..p {
            while self.player_scores[i] >= self.next_extra_life[i] {
                self.player_lives[i] += 1;
                self.next_extra_life[i] += EXTRA_LIFE_EVERY;
            }
        }

        self.players[..p].copy_from_slice(&all[..p]);
        let mut kept = 0;
        for i in 0..n {
            if all[p + i].alive {
                self.enemies[kept] = all[p + i];
                kept += 1;
            }
        }
        self.enemy_count = kept;

        // --- Stage and run outcomes ---
        if !self.base_alive {
            // Losing the base ends the run in Single Player and Co-op.
            self.state = State::GameOver;
        } else if self.waves
            && self.state == State::Playing
            && self.to_spawn == 0
            && self.enemy_count == 0
            && self.spawn_flash[..self.spawn_count].iter().all(|f| *f == 0)
        {
            self.state = State::StageClear;
        }
    }

    /// Credit a kill to the player who landed the last hit. An id at or above
    /// `player_count` is an enemy's, or a tank nobody shot; player 0 takes it,
    /// which is the only player in Single Player.
    fn credit_kill(&mut self, by: u8, points: i32) {
        let slot = if (by as usize) < self.player_count {
            by as usize
        } else {
            0
        };
        self.player_scores[slot] += points;
    }

    // --- stage setup ------------------------------------------------------

    fn load_stage(&mut self, number: u32) {
        let data = stage::load(number);
        self.stage = number;
        self.map = data.map();
        self.base_center = data.base_center;
        self.spawns = data.enemy_spawns;
        self.spawn_count = data.spawn_count;
        for i in 0..MAX_PLAYERS {
            self.player_spawns[i] = data.player_spawn(i);
        }

        self.spawn_flash = [0; MAX_SPAWNS];
        self.spawn_bonus = [false; MAX_SPAWNS];
        // A PvP match has no AI at all, so it never queues a wave.
        self.to_spawn = if self.waves { ENEMIES_PER_STAGE } else { 0 };
        self.enemy_count = 0;
        self.enemies = [Tank::default(); MAX_ENEMIES];
        self.bullet_count = 0;
        self.drop = None;
        self.steel_timer = 0;
        self.steel_count = 0;
        self.base_alive = true;
        self.state = State::Playing;

        // A new stage resets power-ups and upgrades, but not lives or score.
        for i in 0..self.player_count {
            self.spawn_player(i, true);
        }
    }

    fn spawn_player(&mut self, i: usize, fresh: bool) {
        let (tx, ty) = self.player_spawns[i];
        // The side outlives the tank, so it is carried across a respawn.
        let team = self.players[i].team;
        let mut tank = Tank::spawn(Kind::Player, tx, ty, Dir::Up, self.stage, i as u8);
        tank.team = team;
        if !fresh {
            tank.invuln = INVULN_TICKS;
        }
        self.players[i] = tank;
        self.player_respawn[i] = 0;
    }

    /// Respawn player `i` next to the base, unless someone is on the spot.
    fn try_respawn(&mut self, i: usize) {
        // Tile indices in the World, world units in the maths below.
        let (tx, ty) = self.player_spawns[i];
        let (tx, ty) = (tx as i32, ty as i32);
        let spot = (tx * TILE, ty * TILE, (tx + 2) * TILE, (ty + 2) * TILE);
        let taken = self.enemies[..self.enemy_count]
            .iter()
            .any(|e| tank_overlaps(e.rect(), spot))
            || self.players[..self.player_count]
                .iter()
                .enumerate()
                .any(|(j, p)| j != i && p.alive && tank_overlaps(p.rect(), spot));
        if taken {
            self.player_respawn[i] = 1;
            return;
        }
        self.spawn_player(i, false);
    }

    // --- timers -----------------------------------------------------------

    fn update_timers(&mut self) {
        for i in 0..self.player_count {
            self.players[i].tick_timers();
        }
        for i in 0..self.enemy_count {
            self.enemies[i].tick_timers();
        }
        for timer in self.player_respawn.iter_mut() {
            *timer = timer.saturating_sub(1);
        }
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
        if self.players[..self.player_count]
            .iter()
            .any(|p| p.alive && tank_overlaps(p.rect(), area))
        {
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
        // Wrap above the players' own ids, so the two ranges never meet.
        self.next_id = if self.next_id >= 250 {
            FIRST_ENEMY_ID
        } else {
            self.next_id + 1
        };
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
        p: usize,
        n: usize,
        owner: usize,
    ) {
        let by_player = owner < p;
        let owner_team = all[owner].team;
        match kind {
            // The player gains a life; an enemy survives one extra hit.
            PowerUp::ExtraLife => {
                if by_player {
                    self.player_lives[owner] += 1;
                } else {
                    all[owner].hp += 1;
                }
            }
            PowerUp::Shield => all[owner].shield = rules::SHIELD_TICKS,
            // Freezes the opposing side.
            PowerUp::Freeze => {
                for tank in all[..p + n].iter_mut() {
                    if tank.alive && tank.team != owner_team {
                        tank.frozen = rules::FREEZE_TICKS;
                    }
                }
            }
            // Destroys every opposing tank on the map. A deliberate big swing.
            PowerUp::DestroyAll => {
                for tank in all[..p + n].iter_mut() {
                    if tank.alive && tank.team != owner_team && !tank.protected() {
                        tank.hp = 0;
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

pub const SNAP_HEADER: usize = 12;
pub const SNAP_PLAYER: usize = 13;
pub const SNAP_ENEMY: usize = 8;
pub const SNAP_BULLET: usize = 5;
pub const SNAP_SPAWN: usize = 3;
pub const SNAP_PLAYERS_OFFSET: usize = SNAP_HEADER;
pub const SNAP_ENEMIES_OFFSET: usize = SNAP_PLAYERS_OFFSET + MAX_PLAYERS * SNAP_PLAYER;
pub const SNAP_BULLETS_OFFSET: usize = SNAP_ENEMIES_OFFSET + MAX_ENEMIES * SNAP_ENEMY;
pub const SNAP_SPAWNS_OFFSET: usize = SNAP_BULLETS_OFFSET + MAX_BULLETS * SNAP_BULLET;
pub const SNAP_LEN: usize = SNAP_SPAWNS_OFFSET + MAX_SPAWNS * SNAP_SPAWN;
/// `drop_kind` when nothing is lying on the map.
pub const SNAP_NO_DROP: i32 = -1;

impl World {
    pub fn write_snapshot(&self, out: &mut [i32]) {
        out[0] = self.tick as i32;
        out[1] = self.state as i32;
        out[2] = self.stage as i32;
        out[3] = self.enemies_left() as i32;
        out[4] = self.base_alive as i32;
        out[5] = self.player_count as i32;
        out[6] = self.enemy_count as i32;
        out[7] = self.bullet_count as i32;
        out[8] = self.spawn_count as i32;
        out[9] = self.drop.map_or(SNAP_NO_DROP, |d| d.kind as i32);
        out[10] = self.drop.map_or(0, |d| d.x);
        out[11] = self.drop.map_or(0, |d| d.y);

        let mut o = SNAP_PLAYERS_OFFSET;
        for (i, t) in self.players[..self.player_count].iter().enumerate() {
            out[o] = t.x;
            out[o + 1] = t.y;
            out[o + 2] = t.dir as i32;
            out[o + 3] = t.alive as i32;
            out[o + 4] = t.hp as i32;
            out[o + 5] = t.upgrade as i32;
            out[o + 6] = t.shield as i32;
            out[o + 7] = t.speed_boost as i32;
            out[o + 8] = t.frozen as i32;
            out[o + 9] = t.invuln as i32;
            out[o + 10] = t.team as i32;
            out[o + 11] = self.player_lives[i] as i32;
            out[o + 12] = self.player_scores[i];
            o += SNAP_PLAYER;
        }

        o = SNAP_ENEMIES_OFFSET;
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
            out[o + 3] = b.team as i32;
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
