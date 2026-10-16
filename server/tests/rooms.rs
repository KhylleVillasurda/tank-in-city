//! The room rules from plan section 3, one test each.
//!
//! These are the server's own rules, so they are tested against the registry
//! directly — no socket and no browser needed.

use std::collections::HashSet;

use server::rooms::{
    JoinError, Mode, Registry, StartError, Status, Team, CODE_LEN, MAX_NAME_LEN,
};

fn registry() -> Registry {
    Registry::new(7)
}

/// A room of `names`, with `names[0]` as host, everyone ready.
fn lobby(mode: Mode, names: &[&str]) -> (Registry, String) {
    let mut reg = registry();
    let code = reg.create(names[0], mode, true, true, None).unwrap();
    for name in &names[1..] {
        reg.join(&code, name, None).unwrap();
    }
    for name in names {
        reg.set_ready(&code, name, true).unwrap();
    }
    (reg, code)
}

#[test]
fn codes_are_five_readable_characters_and_unique() {
    let mut reg = registry();
    let mut seen = HashSet::new();
    for i in 0..200 {
        let code = reg
            .create(&format!("p{i}"), Mode::OneVOne, true, true, None)
            .unwrap();
        assert_eq!(code.chars().count(), CODE_LEN, "{code} is the wrong length");
        assert!(
            code.chars().all(|c| c.is_ascii_alphanumeric()),
            "{code} has a symbol outside the alphabet"
        );
        assert!(
            !code.contains('I') && !code.contains('O'),
            "{code} contains a character that reads as 1 or 0"
        );
        assert!(seen.insert(code.clone()), "{code} was handed out twice");
    }
}

#[test]
fn the_same_seed_makes_the_same_codes() {
    let codes = |seed| {
        let mut reg = Registry::new(seed);
        (0..5)
            .map(|i| {
                reg.create(&format!("p{i}"), Mode::OneVOne, true, true, None)
                    .unwrap()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(codes(3), codes(3));
    assert_ne!(codes(3), codes(4));
}

#[test]
fn the_host_is_a_player_in_their_own_room() {
    let mut reg = registry();
    let code = reg.create("ann", Mode::OneVOne, true, true, None).unwrap();
    let room = reg.room(&code).unwrap();
    assert_eq!(room.host(), "ann");
    assert_eq!(room.player_count(), 1);
    assert!(!room.players()[0].ready, "a fresh player is not ready");
    assert_eq!(room.status, Status::Waiting);
}

#[test]
fn join_needs_a_real_code() {
    let mut reg = registry();
    let code = reg.create("ann", Mode::OneVOne, true, true, None).unwrap();
    assert_eq!(reg.join(&code, "bo", None), Ok(Team::B));
    assert_eq!(reg.join("ZZZZZ", "cy", None), Err(JoinError::NoSuchRoom));
}

#[test]
fn a_password_is_required_when_one_is_set() {
    let mut reg = registry();
    let code = reg
        .create("ann", Mode::OneVOne, true, false, Some("open-sesame".into()))
        .unwrap();
    assert!(reg.room(&code).unwrap().password_required());
    assert_eq!(reg.join(&code, "bo", None), Err(JoinError::WrongPassword));
    assert_eq!(
        reg.join(&code, "bo", Some("nope")),
        Err(JoinError::WrongPassword)
    );
    assert_eq!(reg.join(&code, "bo", Some("open-sesame")), Ok(Team::B));
}

#[test]
fn an_empty_password_is_no_password() {
    let mut reg = registry();
    let code = reg
        .create("ann", Mode::OneVOne, true, false, Some(String::new()))
        .unwrap();
    assert!(!reg.room(&code).unwrap().password_required());
    assert_eq!(reg.join(&code, "bo", None), Ok(Team::B));
}

#[test]
fn joins_are_rejected_once_the_match_has_started() {
    let (mut reg, code) = lobby(Mode::FreeForAll, &["ann", "bo"]);
    assert_eq!(reg.start(&code, "ann"), Ok(()));
    assert_eq!(reg.room(&code).unwrap().status, Status::Playing);
    assert_eq!(reg.join(&code, "cy", None), Err(JoinError::InProgress));
}

#[test]
fn names_are_unique_in_a_room() {
    let mut reg = registry();
    let code = reg.create("ann", Mode::ThreeVThree, true, true, None).unwrap();
    assert_eq!(reg.join(&code, "bo", None), Ok(Team::B));
    assert_eq!(reg.join(&code, "bo", None), Err(JoinError::NameTaken));
}

#[test]
fn the_same_name_can_be_in_two_rooms() {
    let mut reg = registry();
    let a = reg.create("ann", Mode::OneVOne, true, true, None).unwrap();
    let b = reg.create("ann", Mode::OneVOne, true, true, None).unwrap();
    assert_ne!(a, b);
    assert_eq!(reg.join(&b, "bo", None), Ok(Team::B));
}

#[test]
fn names_are_trimmed_and_bounded() {
    let mut reg = registry();
    let code = reg.create("  ann  ", Mode::OneVOne, true, true, None).unwrap();
    assert_eq!(reg.room(&code).unwrap().host(), "ann");

    assert_eq!(
        reg.create("   ", Mode::OneVOne, true, true, None),
        Err(JoinError::BadName)
    );
    let too_long = "x".repeat(MAX_NAME_LEN + 1);
    assert_eq!(
        reg.create(&too_long, Mode::OneVOne, true, true, None),
        Err(JoinError::BadName)
    );
    let longest = "y".repeat(MAX_NAME_LEN);
    assert!(reg.create(&longest, Mode::OneVOne, true, true, None).is_ok());
}

#[test]
fn a_room_fills_at_the_mode_limit() {
    let mut reg = registry();
    let code = reg.create("p0", Mode::TwoVTwo, true, true, None).unwrap();
    for i in 1..4 {
        reg.join(&code, &format!("p{i}"), None).unwrap();
    }
    assert_eq!(reg.join(&code, "p4", None), Err(JoinError::RoomFull));
}

#[test]
fn players_fill_the_smaller_side_and_ties_go_to_a() {
    let mut reg = registry();
    let code = reg.create("p0", Mode::ThreeVThree, true, true, None).unwrap();
    assert_eq!(reg.join(&code, "p1", None), Ok(Team::B));
    assert_eq!(reg.join(&code, "p2", None), Ok(Team::A));
    assert_eq!(reg.join(&code, "p3", None), Ok(Team::B));
    assert_eq!(reg.join(&code, "p4", None), Ok(Team::A));

    let room = reg.room(&code).unwrap();
    assert_eq!(room.side_count(Team::A), 3);
    assert_eq!(room.side_count(Team::B), 2);
}

#[test]
fn coop_puts_both_players_on_one_side() {
    let mut reg = registry();
    let code = reg.create("ann", Mode::Coop, false, true, None).unwrap();
    assert_eq!(reg.join(&code, "bo", None), Ok(Team::A));

    let room = reg.room(&code).unwrap();
    assert_eq!(room.side_count(Team::A), 2);
    assert_eq!(room.side_count(Team::B), 0);
    assert!(Mode::Coop.has_ai());
    assert!(!Mode::OneVOne.has_ai());
}

#[test]
fn free_for_all_has_no_sides_and_friendly_fire_is_forced_on() {
    let mut reg = registry();
    // Asked for off, and still on.
    let code = reg
        .create("ann", Mode::FreeForAll, false, true, None)
        .unwrap();
    assert!(reg.room(&code).unwrap().friendly_fire);
    assert_eq!(reg.join(&code, "bo", None), Ok(Team::None));
    assert_eq!(reg.room(&code).unwrap().side_count(Team::None), 2);
}

#[test]
fn the_hosts_friendly_fire_choice_is_kept_in_team_modes() {
    let mut reg = registry();
    let off = reg.create("ann", Mode::OneVOne, false, true, None).unwrap();
    let on = reg.create("bo", Mode::OneVOne, true, true, None).unwrap();
    assert!(!reg.room(&off).unwrap().friendly_fire);
    assert!(reg.room(&on).unwrap().friendly_fire);
}

#[test]
fn the_host_passes_to_the_next_player_and_an_empty_room_closes() {
    let mut reg = registry();
    let code = reg.create("ann", Mode::ThreeVThree, true, true, None).unwrap();
    reg.join(&code, "bo", None).unwrap();
    reg.join(&code, "cy", None).unwrap();

    reg.leave(&code, "ann").unwrap();
    assert_eq!(reg.room(&code).unwrap().host(), "bo");
    assert_eq!(reg.room(&code).unwrap().player_count(), 2);

    reg.leave(&code, "bo").unwrap();
    reg.leave(&code, "cy").unwrap();
    assert!(reg.room(&code).is_none(), "the room should have closed");
    assert!(reg.is_empty());
}

#[test]
fn leaving_a_room_you_are_not_in_is_refused() {
    let mut reg = registry();
    let code = reg.create("ann", Mode::OneVOne, true, true, None).unwrap();
    assert_eq!(reg.leave(&code, "bo"), Err(JoinError::NotInRoom));
    assert_eq!(reg.leave("ZZZZZ", "ann"), Err(JoinError::NoSuchRoom));
    assert_eq!(reg.set_ready(&code, "bo", true), Err(JoinError::NotInRoom));
    assert_eq!(
        reg.set_ready("ZZZZZ", "ann", true),
        Err(JoinError::NoSuchRoom)
    );
}

#[test]
fn start_needs_every_player_ready() {
    let mut reg = registry();
    let code = reg.create("ann", Mode::OneVOne, true, true, None).unwrap();
    reg.join(&code, "bo", None).unwrap();

    reg.set_ready(&code, "ann", true).unwrap();
    assert_eq!(reg.start(&code, "ann"), Err(StartError::NotAllReady));

    reg.set_ready(&code, "bo", true).unwrap();
    assert_eq!(reg.start(&code, "ann"), Ok(()));
}

#[test]
fn only_the_host_can_start() {
    let (mut reg, code) = lobby(Mode::FreeForAll, &["ann", "bo"]);
    assert_eq!(reg.start(&code, "bo"), Err(StartError::NotHost));
    assert_eq!(reg.room(&code).unwrap().status, Status::Waiting);
    assert_eq!(reg.start(&code, "ann"), Ok(()));
}

#[test]
fn one_v_one_needs_one_a_side() {
    let (reg, code) = lobby(Mode::OneVOne, &["ann", "bo"]);
    assert_eq!(reg.room(&code).unwrap().can_start(), Ok(()));

    let mut reg = registry();
    let code = reg.create("ann", Mode::OneVOne, true, true, None).unwrap();
    reg.set_ready(&code, "ann", true).unwrap();
    assert_eq!(
        reg.room(&code).unwrap().can_start(),
        Err(StartError::SideTooSmall),
        "one player alone cannot start a 1v1"
    );
}

#[test]
fn team_modes_need_two_a_side() {
    let (reg, code) = lobby(Mode::ThreeVThree, &["ann", "bo"]);
    assert_eq!(
        reg.room(&code).unwrap().can_start(),
        Err(StartError::SideTooSmall)
    );
}

#[test]
fn uneven_teams_can_start() {
    let (reg, code) = lobby(Mode::ThreeVThree, &["a", "b", "c", "d", "e"]);
    let room = reg.room(&code).unwrap();
    assert_eq!(room.side_count(Team::A), 3);
    assert_eq!(room.side_count(Team::B), 2);
    assert_eq!(room.can_start(), Ok(()));
}

#[test]
fn free_for_all_and_coop_need_two_players() {
    for mode in [Mode::FreeForAll, Mode::Coop] {
        let mut reg = registry();
        let code = reg.create("ann", mode, true, true, None).unwrap();
        reg.set_ready(&code, "ann", true).unwrap();
        assert_eq!(
            reg.room(&code).unwrap().can_start(),
            Err(StartError::TooFewPlayers),
            "{mode:?} must not start with one player"
        );

        reg.join(&code, "bo", None).unwrap();
        reg.set_ready(&code, "bo", true).unwrap();
        assert_eq!(reg.start(&code, "ann"), Ok(()), "{mode:?} should start with two");
    }
}

#[test]
fn starting_a_room_that_does_not_exist_is_refused() {
    let mut reg = registry();
    assert_eq!(reg.start("ZZZZZ", "ann"), Err(StartError::NoSuchRoom));
}

#[test]
fn finishing_a_match_puts_the_room_back_in_the_lobby() {
    let (mut reg, code) = lobby(Mode::FreeForAll, &["ann", "bo"]);
    reg.start(&code, "ann").unwrap();
    assert_eq!(reg.room(&code).unwrap().status, Status::Playing);

    assert!(reg.finish(&code), "the room was there to finish");
    assert_eq!(reg.room(&code).unwrap().status, Status::Waiting);
    assert_eq!(reg.join(&code, "cy", None), Ok(Team::None), "joinable again");

    // A room everyone left is gone, and finishing it is not an error.
    assert!(!reg.finish("ZZZZZ"));
}

#[test]
fn the_public_list_shows_public_rooms_and_marks_in_progress() {
    let mut reg = registry();
    let open = reg
        .create("ann", Mode::FreeForAll, true, true, None)
        .unwrap();
    reg.create("bo", Mode::FreeForAll, true, false, None).unwrap();
    let playing = reg
        .create("cy", Mode::FreeForAll, true, true, None)
        .unwrap();

    assert_eq!(reg.list_public().len(), 2, "the private room is not listed");

    reg.join(&playing, "de", None).unwrap();
    reg.set_ready(&playing, "cy", true).unwrap();
    reg.set_ready(&playing, "de", true).unwrap();
    reg.start(&playing, "cy").unwrap();

    let list = reg.list_public();
    assert_eq!(list.len(), 2, "a room in progress stays listed");
    let row = |code: &str| list.iter().find(|r| r.code == code).unwrap().clone();
    assert_eq!(row(&open).status, Status::Waiting);
    assert_eq!(row(&playing).status, Status::Playing);
    assert_eq!(row(&playing).players, 2);
    assert_eq!(row(&playing).host, "cy");
    assert_eq!(row(&playing).mode, Mode::FreeForAll);

    // Sorted by code, so the list does not jump around as it refreshes.
    let codes: Vec<_> = list.iter().map(|r| r.code.clone()).collect();
    let mut sorted = codes.clone();
    sorted.sort();
    assert_eq!(codes, sorted);
}
