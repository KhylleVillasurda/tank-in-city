//! Enemy behaviour (plan section 6, Enemy behavior).
//!
//! Moves in straight lines, changes direction at intervals or when blocked,
//! fires at intervals, and sometimes heads for the base.

use crate::rng::Rng;
use crate::tank::{Dir, Tank};

/// Ticks between direction changes. Tuning knob.
pub const TURN_TICKS: (u32, u32) = (40, 120);
/// Ticks between AI fire decisions. Tuning knob.
pub const FIRE_TICKS: (u32, u32) = (90, 260);
/// Chance, in percent, of turning toward the base instead of picking at
/// random. Tuning knob.
pub const HEAD_FOR_BASE_CHANCE: u32 = 10;
/// Chance, in percent, of changing direction at all when the timer runs out.
pub const TURN_CHANCE: u32 = 40;

/// Advance one enemy's AI by one tick. Returns true when it wants to fire.
pub fn update(tank: &mut Tank, rng: &mut Rng, base: (i32, i32)) -> bool {
    if tank.blocked {
        // A wall: turn into one of the three directions that is not the one
        // that just failed, rather than re-rolling the same one.
        tank.dir = ((tank.dir as usize + 1 + rng.below(3) as usize) % 4) as u8;
        tank.turn_timer = rng.range(TURN_TICKS.0, TURN_TICKS.1) as u8;
    } else if tank.turn_timer == 0 {
        if rng.below(100) < HEAD_FOR_BASE_CHANCE {
            tank.dir = Dir::toward(tank.center(), base) as u8;
        } else if rng.below(100) < TURN_CHANCE {
            tank.dir = Dir::ALL[rng.below(4) as usize] as u8;
        }
        tank.turn_timer = rng.range(TURN_TICKS.0, TURN_TICKS.1) as u8;
    } else {
        tank.turn_timer -= 1;
    }

    if tank.fire_timer == 0 {
        tank.fire_timer = rng.range(FIRE_TICKS.0, FIRE_TICKS.1) as u16;
        true
    } else {
        tank.fire_timer -= 1;
        false
    }
}
