//! Numbers the plan leaves as "starting defaults to tune" (plan section 6).
//!
//! Each constant carries the narrowest type that holds it, and the struct field
//! it feeds matches. Tick counts that fit in a byte are `u8`; the power-up
//! durations need more than 255 and are `u16`. `TICK_HZ` stays `u32` because it
//! is the base rate everything else is derived from.

/// Fixed simulation rate. The engine advances by exactly this many steps per
/// second, independent of the renderer's frame rate.
pub const TICK_HZ: u32 = 60;

/// Arena size in tiles. Classic Battle City proportions.
pub const MAP_W: usize = 26;
pub const MAP_H: usize = 26;

// --- Single Player, endless arcade (plan section 6) ---

/// Lives the run starts with.
pub const START_LIVES: u8 = 3;
/// One extra life every this many points. Tuning knob.
pub const EXTRA_LIFE_EVERY: i32 = 5_000;

/// A destroyed tank comes back after this long.
pub const RESPAWN_TICKS: u8 = (3 * TICK_HZ) as u8;
/// Respawn invulnerability. A tank this fresh cannot fire (defense only).
pub const INVULN_TICKS: u8 = (2 * TICK_HZ) as u8;

/// Enemies in one stage.
pub const ENEMIES_PER_STAGE: u8 = 20;
/// How long a spawn point flashes before its tank appears.
pub const SPAWN_FLASH_TICKS: u8 = TICK_HZ as u8;
/// The 4th, 11th and 18th enemies of a stage carry a power-up.
pub const BONUS_ENEMIES: [u8; 3] = [4, 11, 18];

// --- Power-up durations (plan section 6, Power-ups) ---

pub const SHIELD_TICKS: u16 = (8 * TICK_HZ) as u16;
pub const FREEZE_TICKS: u16 = (8 * TICK_HZ) as u16;
pub const SPEED_TICKS: u16 = (10 * TICK_HZ) as u16;
pub const STEEL_WALLS_TICKS: u16 = (15 * TICK_HZ) as u16;

/// Longest gap between enemy shots.
pub const ENEMY_FIRE_COOLDOWN: u16 = 40;
