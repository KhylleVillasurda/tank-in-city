//! Power-ups (plan section 6).
//!
//! An enemy that drives over a drop takes it and applies it to its own side,
//! which is why the effects live on `World` rather than being hard-wired to
//! the player. The table in the plan is what `World::apply_powerup` implements.

use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum PowerUp {
    ExtraLife = 0,
    Shield = 1,
    Freeze = 2,
    DestroyAll = 3,
    Upgrade = 4,
    Speed = 5,
    SteelWalls = 6,
}

impl PowerUp {
    pub const ALL: [PowerUp; 7] = [
        PowerUp::ExtraLife,
        PowerUp::Shield,
        PowerUp::Freeze,
        PowerUp::DestroyAll,
        PowerUp::Upgrade,
        PowerUp::Speed,
        PowerUp::SteelWalls,
    ];

    pub fn from_u8(v: u8) -> PowerUp {
        match v {
            1 => PowerUp::Shield,
            2 => PowerUp::Freeze,
            3 => PowerUp::DestroyAll,
            4 => PowerUp::Upgrade,
            5 => PowerUp::Speed,
            6 => PowerUp::SteelWalls,
            _ => PowerUp::ExtraLife,
        }
    }

    /// Uniform for now. Every power-up in the Single Player column is in the
    /// pool, so weighting is a tuning knob rather than a mode difference.
    pub fn drop_for(rng: &mut Rng) -> PowerUp {
        PowerUp::ALL[rng.below(PowerUp::ALL.len() as u32) as usize]
    }
}

/// A drop lying on the map. At most one at a time (plan section 6).
#[derive(Clone, Copy)]
pub struct Drop {
    pub kind: PowerUp,
    pub x: i32,
    pub y: i32,
}

impl Drop {
    pub fn rect(&self) -> (i32, i32, i32, i32) {
        let size = crate::map::TILE;
        (self.x, self.y, self.x + size, self.y + size)
    }
}
