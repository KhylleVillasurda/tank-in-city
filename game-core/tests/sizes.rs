//! Guards the widths of the engine's state.
//!
//! Every field carries the narrowest type that holds its longest value, and this
//! is what keeps it that way: a field cannot quietly widen back to `u32` without
//! failing here.
//!
//! These are not the smallest sizes the types could ever have. They are the
//! sizes the engine has now, pinned deliberately. When a real feature needs more
//! room, raise the number in the same commit and say why in the message.

use std::mem::size_of;

use game_core::bullet::Bullet;
use game_core::map::Map;
use game_core::powerups::Drop;
use game_core::tank::Tank;
use game_core::World;

/// One whole run of game state. It was 1384 bytes before the widths were
/// tightened; most of the saving is the `Tank` counters and the 16 remembered
/// steel tiles. Milestone 3's several-players change put a `Tank` and its
/// run-long state on every one of the six player slots, which is most of the
/// 1144 -> 1392. A room owns one `World`.
const WORLD_BUDGET: usize = 1392;

#[test]
fn state_stays_narrow() {
    let actual = size_of::<World>();
    assert!(
        actual <= WORLD_BUDGET,
        "World grew to {actual} bytes, budget is {WORLD_BUDGET}. Did a field widen?"
    );
    // 28 -> 32: the side byte and the last-hit-by byte overflowed the 28-byte
    // layout, so the struct picked up 4 bytes of padding. Six player tanks and
    // four enemies is ten of them.
    assert!(size_of::<Tank>() <= 32, "Tank grew to {}", size_of::<Tank>());
    assert!(
        size_of::<Bullet>() <= 16,
        "Bullet grew to {}",
        size_of::<Bullet>()
    );
    assert!(size_of::<Map>() <= 676, "Map grew to {}", size_of::<Map>());
    assert!(size_of::<Drop>() <= 12, "Drop grew to {}", size_of::<Drop>());
}
