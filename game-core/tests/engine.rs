//! Engine tests. These run natively, which is also the "compiles to native
//! code" half of the Milestone 1 check.

use game_core::bullet::Bullet;
use game_core::map::{Map, Tile, TANK_SIZE, TILE};
use game_core::powerups::{Drop, PowerUp};
use game_core::rules::{EXTRA_LIFE_EVERY, START_LIVES};
use game_core::stage;
use game_core::tank::{overlaps, Dir, Kind, Tank, MAX_ARMOR_HP, MAX_PLAYERS, TEAM_A, TEAM_B};
use game_core::{
    Input, Setup, State, World, INPUT_CONFIRM, INPUT_FIRE, INPUT_LEFT, INPUT_RIGHT, INPUT_UP,
    SNAP_LEN,
};

fn snapshot(world: &World) -> [i32; SNAP_LEN] {
    let mut out = [0; SNAP_LEN];
    world.write_snapshot(&mut out);
    out
}

/// Scripted input for the determinism check.
fn script(tick: u32) -> u32 {
    match tick / 40 % 4 {
        0 => INPUT_UP | INPUT_FIRE,
        1 => INPUT_RIGHT | INPUT_FIRE,
        2 => INPUT_LEFT,
        _ => INPUT_LEFT | INPUT_FIRE,
    }
}

/// A world holding exactly one enemy, with the wave counter parked so the
/// stage cannot clear underneath the test.
fn world_with_enemy(kind: Kind) -> World {
    let mut world = World::new(7);
    world.enemies[0] = Tank::spawn(kind, 12, 1, Dir::Down, 1, 9);
    world.enemy_count = 1;
    world.to_spawn = 0;
    world
}

/// A match of `players`, all on side A, with the wave system parked so nothing
/// spawns and the stage cannot clear underneath the test.
fn match_of(players: u8, waves: bool, friendly_fire: bool) -> World {
    let mut setup = Setup::single();
    setup.players = players;
    setup.waves = waves;
    setup.friendly_fire = friendly_fire;
    let mut world = World::with_setup(7, setup);
    world.spawn_count = 0;
    world
}

fn coop() -> World {
    match_of(2, true, false)
}

/// Free For All: every player on a side of its own, which is what makes
/// friendly fire always on there.
fn free_for_all(players: u8) -> World {
    let mut setup = Setup::single();
    setup.players = players;
    setup.waves = false;
    for (i, team) in setup.teams.iter_mut().enumerate() {
        *team = i as u8;
    }
    let mut world = World::with_setup(7, setup);
    world.spawn_count = 0;
    world
}

#[test]
fn player_moves_in_integer_steps_and_stops_at_the_wall() {
    // Driven against the stage map directly, so a wandering enemy cannot
    // change where the wall is.
    let map = stage::load(1).map();
    let mut me = Tank::spawn(Kind::Player, 6, 23, Dir::Up, 1, 0);
    let start = me.x;
    assert_eq!(me.y, 23 * TILE, "spawn is on the tile grid");

    let rects = [me.rect()];
    for _ in 0..600 {
        me.drive(&map, &rects, 0, Dir::Left, me.drive_speed());
    }

    assert!(me.x < start, "the tank moved left");
    assert_eq!(me.x, TILE, "flush against the steel border");
    assert_eq!(me.x % TILE, 0, "still on the tile grid");
    assert_eq!(me.y % TILE, 0, "and still row-aligned");
}

#[test]
fn a_world_run_keeps_the_player_inside_the_arena() {
    let mut world = World::new(7);
    for _ in 0..600 {
        world.step(&[Input::new(INPUT_LEFT)]);
        assert!(world.players[0].x >= TILE, "inside the steel border");
        assert!(world.players[0].y >= 0);
    }
}

#[test]
fn a_tank_moves_on_one_axis_at_a_time() {
    // Regression: the turn-alignment step eased the off-axis coordinate into
    // place over several ticks, so the tank travelled diagonally for about a
    // fifth of a second after every direction change. Tanks move in their four
    // directions only.
    let map = stage::load(1).map();
    let mut me = Tank::spawn(Kind::Player, 6, 23, Dir::Up, 1, 0);
    let mut rects = [me.rect(); 1];

    let mut dir = Dir::Up;
    let mut turns = 0;
    let mut both = 0;
    let mut moved = 0;
    let mut prev = (me.x, me.y);
    for tick in 0..600 {
        let next = match tick / 100 % 4 {
            0 => Dir::Up,
            1 => Dir::Left,
            2 => Dir::Down,
            _ => Dir::Right,
        };
        if next != dir {
            dir = next;
            turns += 1;
        }
        me.drive(&map, &rects, 0, dir, me.drive_speed());
        rects[0] = me.rect();

        let moved_x = me.x != prev.0;
        let moved_y = me.y != prev.1;
        if moved_x && moved_y {
            both += 1;
        }
        if moved_x || moved_y {
            moved += 1;
        }
        prev = (me.x, me.y);
    }

    assert!(turns >= 5, "the run changed direction");
    assert!(moved > 300, "the tank was not simply wedged in a corner");
    assert_eq!(both, 0, "{both} ticks moved the tank on two axes at once");
}

#[test]
fn turning_after_driving_up_lines_up_in_one_tick() {
    let map = stage::load(1).map();
    let mut me = Tank::spawn(Kind::Player, 6, 23, Dir::Up, 1, 0);
    let mut rects = [me.rect(); 1];

    // An odd tick count leaves the tank between two tile edges.
    for _ in 0..7 {
        me.drive(&map, &rects, 0, Dir::Up, me.drive_speed());
        rects[0] = me.rect();
    }
    assert_ne!(me.y % TILE, 0, "deliberately off the tile grid");
    assert!((me.y - 23 * TILE).abs() <= TILE / 2);

    me.drive(&map, &rects, 0, Dir::Left, me.drive_speed());
    rects[0] = me.rect();
    assert_eq!(me.y % TILE, 0, "lined up on the first tick of the new direction");

    // From here the other axis must not move again, ever.
    let (aligned_y, x) = (me.y, me.x);
    for _ in 0..30 {
        me.drive(&map, &rects, 0, Dir::Left, me.drive_speed());
        rects[0] = me.rect();
        assert_eq!(me.y, aligned_y, "no drift on the axis we are not driving");
    }
    assert!(me.x < x, "and it actually travelled");
}

#[test]
fn tanks_cannot_overlap() {
    let map = stage::load(1).map();
    let me = Tank::spawn(Kind::Player, 6, 23, Dir::Up, 1, 0);
    let other = Tank::spawn(Kind::Basic, 6, 21, Dir::Up, 1, 1);
    let empty = [(0, 0, 0, 0); 2];

    assert!(
        !me.hits(&map, &empty, 0, me.x, me.y - 1),
        "clear one unit up"
    );
    let rects = [me.rect(), other.rect()];
    assert!(
        me.hits(&map, &rects, 0, me.x, me.y - TILE),
        "one tile up is free of walls, so this is the other tank blocking"
    );
    assert!(
        !me.hits(&map, &empty, 0, me.x, me.y - TILE),
        "without the other tank the move is allowed"
    );
}

#[test]
fn bullets_break_bricks_and_die_on_steel() {
    use game_core::bullet::Hit;

    let mut map = Map::empty();
    map.set(10, 10, Tile::Brick);
    map.set(12, 10, Tile::Steel);

    let mut brick = Bullet {
        x: 10 * TILE - 100,
        y: 10 * TILE + TILE / 2,
        dir: Dir::Right as u8,
        speed: 32,
        owner: 0,
        team: TEAM_A,
    };
    let mut hit = Hit::None;
    for _ in 0..20 {
        if hit != Hit::None {
            break;
        }
        hit = brick.advance(&mut map);
    }
    assert_eq!(hit, Hit::Brick);
    assert_eq!(map.get(10, 10), Tile::Empty, "the brick is gone");

    let mut steel = Bullet {
        x: 12 * TILE - 100,
        y: 10 * TILE + TILE / 2,
        dir: Dir::Right as u8,
        speed: 32,
        owner: 0,
        team: TEAM_A,
    };
    hit = Hit::None;
    for _ in 0..20 {
        if hit != Hit::None {
            break;
        }
        hit = steel.advance(&mut map);
    }
    assert_eq!(hit, Hit::Wall);
    assert_eq!(map.get(12, 10), Tile::Steel, "steel survives");
}

#[test]
fn a_run_starts_with_lives_score_and_a_wave() {
    let world = World::new(1);
    assert_eq!(world.player_lives[0], START_LIVES);
    assert_eq!(world.player_scores[0], 0);
    assert_eq!(world.stage, 1);
    assert_eq!(world.enemies_left(), 20, "20 enemies per stage");
    assert!(world.base_alive);
}

#[test]
fn at_most_four_enemies_are_on_the_map_at_once() {
    let mut world = World::new(3);
    let mut seen_any = false;
    for _ in 0..1800 {
        world.step(&[Input::new(0)]);
        assert!(world.enemy_count <= 4, "cap of 4 concurrent enemies");
        if world.enemy_count > 0 {
            seen_any = true;
        }
    }
    assert!(seen_any, "enemies spawned");
    assert!(world.enemies_left() <= 20);
}

#[test]
fn armour_takes_more_hits_in_later_stages_and_is_capped() {
    assert_eq!(Kind::Armor.hp(1), 4);
    assert!(Kind::Armor.hp(9) > Kind::Armor.hp(1), "scales with stage");
    assert_eq!(Kind::Armor.hp(99), MAX_ARMOR_HP, "capped");
    assert_eq!(Kind::Basic.hp(99), 1);
}

#[test]
fn destroying_an_enemy_scores_its_type() {
    for (kind, points) in [
        (Kind::Basic, 100),
        (Kind::Fast, 200),
        (Kind::Power, 300),
        (Kind::Armor, 400),
    ] {
        let mut world = world_with_enemy(kind);
        world.enemies[0].hp = 0;
        world.step(&[Input::new(0)]);
        assert_eq!(world.player_scores[0], points, "{kind:?} is worth {points}");
        assert_eq!(world.enemy_count, 0, "{kind:?} is gone");
    }
}

#[test]
fn an_extra_life_arrives_at_the_score_threshold() {
    let mut world = world_with_enemy(Kind::Basic);
    world.player_scores[0] = EXTRA_LIFE_EVERY - 50;
    world.enemies[0].hp = 0;
    world.step(&[Input::new(0)]);
    assert!(world.player_scores[0] >= EXTRA_LIFE_EVERY);
    assert_eq!(world.player_lives[0], START_LIVES + 1);
}

#[test]
fn losing_all_lives_ends_the_run() {
    let mut world = World::new(5);
    world.players[0].hp = 0;
    world.step(&[Input::new(0)]);
    assert_eq!(world.player_lives[0], START_LIVES - 1);
    assert_eq!(world.state, State::Playing, "still has lives");
    assert!(!world.players[0].alive);

    let mut world = World::new(5);
    world.player_lives[0] = 1;
    world.players[0].hp = 0;
    world.step(&[Input::new(0)]);
    assert_eq!(world.player_lives[0], 0);
    assert_eq!(world.state, State::GameOver);
}

#[test]
fn respawn_returns_the_player_with_invulnerability() {
    let mut world = World::new(5);
    world.player_lives[0] = 3;
    world.players[0].hp = 0;
    world.step(&[Input::new(0)]);
    for _ in 0..240 {
        world.step(&[Input::new(0)]);
    }
    assert!(world.players[0].alive, "came back");
    assert!(world.players[0].invuln > 0, "with respawn invulnerability");
    assert_eq!(world.players[0].upgrade, 0, "upgrades reset on death");
}

#[test]
fn losing_the_base_ends_the_run() {
    let mut world = World::new(2);
    // A shot already inside a base tile.
    world.bullets[0] = Bullet {
        x: 12 * TILE + 8,
        y: 23 * TILE + 8,
        dir: Dir::Right as u8,
        speed: 32,
        owner: 2,
        team: TEAM_B,
    };
    world.bullet_count = 1;
    world.step(&[Input::new(0)]);
    assert!(!world.base_alive);
    assert_eq!(world.state, State::GameOver);
}

#[test]
fn your_own_shot_cannot_destroy_your_base() {
    // The player spawns on the base's row, so firing along that row used to
    // level the base in 47 ticks from a standstill and end the run with all
    // three lives intact.
    let mut world = World::new(2444195321);
    let ring = |w: &World| {
        (9..=16)
            .flat_map(|tx| (20..=25).map(move |ty| (tx, ty)))
            .filter(|&(tx, ty)| {
                w.tile(tx, ty) == Tile::Brick as u8 && w.map.touches_base(tx, ty)
            })
            .count()
    };
    let before = ring(&world);

    for tick in 0..180 {
        world.step(&[Input::new(INPUT_RIGHT | INPUT_FIRE)]);
        assert!(world.base_alive, "the player shot its own base down on tick {tick}");
        assert_eq!(world.state, State::Playing);
    }
    assert!(ring(&world) < before, "the shots still broke a ring brick");
}

#[test]
fn clearing_the_wave_advances_to_the_next_stage() {
    let mut world = World::new(4);
    world.enemy_count = 0;
    world.to_spawn = 0;
    world.step(&[Input::new(0)]);
    assert_eq!(world.state, State::StageClear);

    // The stage-clear screen waits for the player rather than timing out, so
    // holding fire does not skip the sound and neither does waiting.
    for _ in 0..600 {
        world.step(&[Input::new(INPUT_FIRE)]);
        assert_eq!(world.state, State::StageClear, "fire alone must not advance");
    }

    world.step(&[Input::new(INPUT_CONFIRM)]);
    assert_eq!(world.state, State::Playing);
    assert_eq!(world.stage, 2, "the next stage loaded");
    assert_eq!(world.enemies_left(), 20);
    assert_eq!(world.player_lives[0], START_LIVES, "lives carry over");
}

#[test]
fn the_player_takes_a_power_up_lying_on_its_tile() {
    /// A power-up and the check that its effect landed on the world.
    type Case = (PowerUp, fn(&World) -> bool);
    let cases: Vec<Case> = vec![
        (PowerUp::Shield, |w| w.players[0].shield > 0),
        (PowerUp::Speed, |w| w.players[0].speed_boost > 0),
        (PowerUp::Upgrade, |w| w.players[0].upgrade == 1),
        (PowerUp::SteelWalls, |w| {
            w.steel_timer > 0 && w.tile(11, 23) == Tile::Steel as u8
        }),
    ];
    for (kind, check) in cases {
        let mut world = World::new(8);
        world.drop = Some(Drop {
            kind,
            x: world.players[0].x,
            y: world.players[0].y,
        });
        world.step(&[Input::new(0)]);
        assert!(world.drop.is_none(), "{kind:?} was collected");
        assert!(check(&world), "{kind:?} took effect");
    }
}

#[test]
fn an_enemy_takes_a_power_up_for_its_own_side() {
    let mut world = world_with_enemy(Kind::Basic);
    let (x, y) = (world.enemies[0].x, world.enemies[0].y);

    world.drop = Some(Drop {
        kind: PowerUp::Upgrade,
        x,
        y,
    });
    world.step(&[Input::new(0)]);
    assert!(world.drop.is_none());
    assert_eq!(world.enemies[0].upgrade, 1, "the enemy upgraded, not the player");
    assert_eq!(world.players[0].upgrade, 0);

    let mut world = world_with_enemy(Kind::Basic);
    let (x, y) = (world.enemies[0].x, world.enemies[0].y);
    world.drop = Some(Drop {
        kind: PowerUp::Freeze,
        x,
        y,
    });
    world.step(&[Input::new(0)]);
    assert!(world.players[0].frozen > 0, "Freeze hits the opposing side");
}

#[test]
fn steel_base_walls_revert_to_brick() {
    let mut world = World::new(11);
    world.drop = Some(Drop {
        kind: PowerUp::SteelWalls,
        x: world.players[0].x,
        y: world.players[0].y,
    });
    world.step(&[Input::new(0)]);
    assert_eq!(world.tile(11, 23), Tile::Steel as u8);

    for _ in 0..game_core::rules::STEEL_WALLS_TICKS {
        world.step(&[Input::new(0)]);
    }
    assert_eq!(world.tile(11, 23), Tile::Brick as u8, "back to brick");
}

#[test]
fn a_reinforced_base_is_not_destroyed_by_one_shot() {
    // Regression guard for the steel wall covering the base in the same tile
    // the bullet would otherwise reach.
    let mut world = World::new(12);
    world.drop = Some(Drop {
        kind: PowerUp::SteelWalls,
        x: world.players[0].x,
        y: world.players[0].y,
    });
    world.step(&[Input::new(0)]);
    world.bullets[0] = Bullet {
        x: 11 * TILE + TILE / 2,
        y: 23 * TILE + 8,
        dir: Dir::Right as u8,
        speed: 32,
        owner: 2,
        team: TEAM_B,
    };
    world.bullet_count = 1;
    world.step(&[Input::new(0)]);
    assert!(world.base_alive, "steel stopped it");
    assert_eq!(world.state, State::Playing);
}

#[test]
fn distinct_seeds_produce_distinct_runs() {
    // Regression: seeding as `seed | 1` made every even seed replay its odd
    // neighbour (2 and 3, 4 and 5, ...).
    let mut firsts = Vec::new();
    for seed in 0..64u32 {
        let mut rng = game_core::rng::Rng::new(seed);
        let first = (0..8).map(|_| rng.next_u32()).collect::<Vec<_>>();
        assert!(!firsts.contains(&first), "seed {seed} repeats another seed");
        firsts.push(first);
    }
}

#[test]
fn same_seed_and_inputs_replay_identically() {
    for seed in [1234, 7, 99] {
        let mut a = World::new(seed);
        let mut b = World::new(seed);
        for tick in 0..4000 {
            let input = Input::new(script(tick));
            a.step(&[input]);
            b.step(&[input]);
            assert_eq!(snapshot(&a), snapshot(&b), "seed {seed} diverged at tick {tick}");
        }
        assert!(a.tick > 0);
    }
}

#[test]
fn different_seeds_diverge() {
    let mut a = World::new(1234);
    let mut b = World::new(1235);
    let mut diverged = false;
    for tick in 0..4000 {
        let input = Input::new(script(tick));
        a.step(&[input]);
        b.step(&[input]);
        diverged |= snapshot(&a) != snapshot(&b);
    }
    assert!(diverged, "neighbouring seeds should not replay the same");
}

// --- Several players at once (Milestone 3) ---

#[test]
fn a_co_op_pair_starts_side_by_side_and_drives_on_its_own_input() {
    let mut world = coop();
    assert_eq!(world.player_count, 2);
    assert!(world.players[0].alive && world.players[1].alive, "both start");
    assert_eq!(world.players[0].team, world.players[1].team, "one side");
    assert_ne!(
        world.players[0].rect(),
        world.players[1].rect(),
        "the stage declares a spawn for each"
    );

    let (x0, x1) = (world.players[0].x, world.players[1].x);
    for _ in 0..30 {
        world.step(&[Input::new(INPUT_LEFT), Input::new(INPUT_RIGHT)]);
    }
    assert!(world.players[0].x < x0, "player 0 drove left");
    assert!(world.players[1].x > x1, "player 1 drove right");
}

#[test]
fn players_are_solid_to_each_other() {
    let mut world = coop();
    // Three tiles apart on the base's row, driving the right one into the
    // left one.
    world.players[1].x = world.players[0].x - 3 * TILE;
    world.players[1].y = world.players[0].y;
    for _ in 0..120 {
        world.step(&[Input::new(0), Input::new(INPUT_RIGHT)]);
    }
    assert_eq!(
        world.players[1].x + TANK_SIZE,
        world.players[0].x,
        "stopped flush against the other tank"
    );
    assert!(
        !overlaps(world.players[0].rect(), world.players[1].rect()),
        "and not inside it"
    );
}

#[test]
fn a_player_out_of_lives_stays_down_while_the_other_plays_on() {
    let mut world = coop();
    world.player_lives[1] = 1;
    world.players[1].hp = 0;
    world.step(&[Input::new(0), Input::new(0)]);
    assert_eq!(world.player_lives[1], 0, "player 1 is out");
    assert_eq!(world.state, State::Playing, "player 0 is still in");

    // Out of lives means no respawn, however long the run goes on.
    for _ in 0..400 {
        world.step(&[Input::new(0), Input::new(0)]);
    }
    assert!(!world.players[1].alive, "and stays down");

    world.player_lives[0] = 1;
    world.players[0].hp = 0;
    world.step(&[Input::new(0), Input::new(0)]);
    assert_eq!(world.state, State::GameOver, "both out ends the run");
}

#[test]
fn a_kill_is_credited_to_the_player_who_landed_the_last_hit() {
    let mut world = coop();
    world.enemies[0] = Tank::spawn(Kind::Basic, 12, 1, Dir::Down, 1, MAX_PLAYERS as u8);
    world.enemy_count = 1;
    world.enemies[0].hp = 0;
    world.enemies[0].last_hit_by = 1; // player 1's id
    world.step(&[Input::new(0), Input::new(0)]);

    assert_eq!(world.player_scores[1], Kind::Basic.score());
    assert_eq!(world.player_scores[0], 0, "player 0 gets nothing");
    assert_eq!(world.score(), Kind::Basic.score());
}

#[test]
fn friendly_fire_decides_whether_a_shot_hurts_its_own_side() {
    for (friendly_fire, hits) in [(false, false), (true, true)] {
        let mut world = match_of(2, false, friendly_fire);
        // Player 1 two tiles to the right, on the same side, and player 0
        // shoots at it.
        world.players[1].x = world.players[0].x + 2 * TILE;
        world.players[1].y = world.players[0].y;
        let before = world.players[1].hp;

        for _ in 0..40 {
            world.step(&[Input::new(INPUT_RIGHT | INPUT_FIRE), Input::new(0)]);
        }
        assert_eq!(
            world.players[1].hp < before,
            hits,
            "friendly fire {friendly_fire}: teammate hit = {hits}"
        );
    }
}

#[test]
fn free_for_all_gives_every_player_a_side_of_its_own() {
    let world = free_for_all(6);
    let mut teams: Vec<u8> = world.players[..world.player_count]
        .iter()
        .map(|p| p.team)
        .collect();
    teams.sort_unstable();
    teams.dedup();
    assert_eq!(teams.len(), 6, "no two players share a side");
}

#[test]
fn a_match_without_the_ai_never_spawns_a_wave_or_clears_a_stage() {
    let mut world = free_for_all(4);
    assert_eq!(world.to_spawn, 0);
    assert_eq!(world.enemies_left(), 0);
    for _ in 0..600 {
        world.step(&[Input::new(0); 4]);
    }
    assert_eq!(world.enemy_count, 0, "no AI at all");
    assert_eq!(world.state, State::Playing, "and no wave to clear");
}

#[test]
fn player_and_enemy_ids_never_collide() {
    let mut world = World::new(7);
    let mut enemies_seen = 0;
    for _ in 0..6000 {
        world.step(&[Input::new(INPUT_UP | INPUT_FIRE)]);
        for (i, p) in world.players[..world.player_count].iter().enumerate() {
            assert_eq!(p.id, i as u8, "a player's id is its slot");
        }
        for e in world.enemies[..world.enemy_count].iter() {
            assert!(
                e.id as usize >= MAX_PLAYERS,
                "enemy id {} is inside the player range",
                e.id
            );
            enemies_seen += 1;
        }
    }
    assert!(enemies_seen > 0, "the run actually spawned enemies");
}
