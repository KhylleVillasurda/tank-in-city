//! Stage data has to be structurally sound or the game is unplayable, and a
//! hand-drawn ASCII map is exactly the kind of thing that rots silently.

use game_core::map::Tile;
use game_core::stage::{self, Stage};
use game_core::{MAP_H, MAP_W};

const SOLID: [Tile; 4] = [Tile::Brick, Tile::Steel, Tile::Water, Tile::Base];

/// Every spawn marker a stage declares: the players' first, then the enemies'.
fn all_spawns(s: &Stage) -> Vec<(u8, u8)> {
    s.player_spawns[..s.player_spawn_count]
        .iter()
        .copied()
        .chain(s.enemy_spawns[..s.spawn_count].iter().copied())
        .collect()
}

fn stage_texts() -> Vec<(&'static str, Stage)> {
    (1..=stage::COUNT as u32)
        .map(|n| {
            let s = stage::load(n);
            (Box::leak(format!("stage{n}").into_boxed_str()) as &'static str, s)
        })
        .collect()
}

#[test]
fn every_stage_is_well_formed_and_connected() {
    for (name, s) in stage_texts() {
        // The border must be sealed.
        for x in 0..MAP_W as i32 {
            assert!(s.map().blocks_tank(x, 0), "{name}: top border hole at {x}");
            assert!(
                s.map().blocks_tank(x, MAP_H as i32 - 1),
                "{name}: bottom border hole at {x}"
            );
        }

        // A base to defend.
        let base = (0..MAP_W * MAP_H)
            .filter(|i| s.tiles[*i] == Tile::Base as u8)
            .count();
        assert!(base > 0, "{name}: no base");

        // Spawnpoints clear enough for a 2x2 tank.
        assert!(s.spawn_count >= 1, "{name}: no enemy spawns");
        for (tx, ty) in all_spawns(&s) {
            // Tile indices are `u8` in the World; the arithmetic here wants `i32`.
            let (tx, ty) = (tx as i32, ty as i32);
            for dy in 0..2 {
                for dx in 0..2 {
                    assert!(
                        !SOLID.contains(&s.map().get(tx + dx, ty + dy)),
                        "{name}: spawn at ({tx}, {ty}) is blocked"
                    );
                }
            }
        }

        // Every enemy spawn must be reachable from the player spawn. Tanks
        // are 2x2, so the walkable nodes are the 2x2 blocks that are free.
        let free: Vec<(i32, i32)> = (0..MAP_H as i32 - 1)
            .flat_map(|y| (0..MAP_W as i32 - 1).map(move |x| (x, y)))
            .filter(|(x, y)| {
                (0..2).all(|a| (0..2).all(|b| !SOLID.contains(&s.map().get(x + a, y + b))))
            })
            .collect();
        let start = (s.player_spawn(0).0 as i32, s.player_spawn(0).1 as i32);
        assert!(free.contains(&start), "{name}: player spawn unreachable");

        let mut seen = vec![start];
        let mut stack = vec![start];
        while let Some((x, y)) = stack.pop() {
            for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if free.contains(&(nx, ny)) && !seen.contains(&(nx, ny)) {
                    seen.push((nx, ny));
                    stack.push((nx, ny));
                }
            }
        }
        // Co-op puts two players on the map at once, so every player spawn has
        // to be somewhere a tank can reach, not just the first.
        for i in 0..s.player_spawn_count {
            let spawn = s.player_spawn(i);
            assert!(
                seen.contains(&(spawn.0 as i32, spawn.1 as i32)),
                "{name}: player spawn {i} at {spawn:?} is cut off"
            );
        }
        for spawn in &s.enemy_spawns[..s.spawn_count] {
            let spawn = (spawn.0 as i32, spawn.1 as i32);
            assert!(
                seen.contains(&spawn),
                "{name}: enemy spawn {spawn:?} cannot reach the player"
            );
        }
    }
}

#[test]
fn stage_numbers_wrap_around() {
    let first = stage::load(1);
    let wrapped = stage::load(stage::COUNT as u32 + 1);
    assert_eq!(first.tiles, wrapped.tiles, "the endless run cycles the maps");
    assert_eq!(first.player_spawns, wrapped.player_spawns);
}

#[test]
fn stage_zero_is_treated_as_the_first_stage() {
    assert_eq!(stage::load(0).tiles, stage::load(1).tiles);
}

#[test]
fn every_stage_can_seat_a_co_op_pair() {
    for (name, s) in stage_texts() {
        assert!(
            s.player_spawn_count >= 2,
            "{name}: Co-op needs a spawn for the second player"
        );
        assert_ne!(
            s.player_spawn(0),
            s.player_spawn(1),
            "{name}: the two players would start inside each other"
        );
    }
}

#[test]
fn spawn_markers_land_on_empty_tiles() {
    for (name, s) in stage_texts() {
        for (tx, ty) in all_spawns(&s) {
            assert!(
                (tx as usize) < MAP_W && (ty as usize) < MAP_H,
                "{name}: spawn marker at ({tx}, {ty}) is off the map"
            );
            assert_eq!(
                s.tiles[ty as usize * MAP_W + tx as usize],
                Tile::Empty as u8,
                "{name}: spawn marker should be a clear tile"
            );
        }
    }
}
