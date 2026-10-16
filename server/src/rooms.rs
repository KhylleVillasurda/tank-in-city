//! Rooms: codes, the public list, teams, and the lobby (plan section 3).
//!
//! Lobby state only. The socket, the match loop and the database are not here
//! yet; this is the part they get built on. Nothing in this module trusts a
//! client — it is the server's own bookkeeping, and every rule below is one the
//! plan states.

use std::collections::HashMap;

use game_core::rng::Rng;
use tracing::{info, warn};

/// Room codes are 5 characters (plan section 3, "Room rules").
pub const CODE_LEN: usize = 5;

/// The symbols a code is built from. `I` and `O` are missing because they read
/// as `1` and `0` in most fonts, which leaves exactly 32 symbols and makes a
/// code 32^5, about 33 million.
const CODE_ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// Most players any mode allows: 3v3.
pub const MAX_PLAYERS: usize = 6;

/// Longest name accepted. Long enough for a real username, short enough to fit
/// a lobby row.
pub const MAX_NAME_LEN: usize = 16;

/// A match mode (plan section 3, "Room modes").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Mode {
    OneVOne,
    TwoVTwo,
    ThreeVThree,
    FreeForAll,
    Coop,
}

impl Mode {
    pub const ALL: [Mode; 5] = [
        Mode::OneVOne,
        Mode::TwoVTwo,
        Mode::ThreeVThree,
        Mode::FreeForAll,
        Mode::Coop,
    ];

    /// Most players a match of this mode allows.
    pub fn max_players(self) -> u8 {
        match self {
            Mode::OneVOne => 2,
            Mode::TwoVTwo => 4,
            Mode::ThreeVThree => 6,
            Mode::FreeForAll => 6,
            Mode::Coop => 2,
        }
    }

    /// True when joining players are split into two sides. Free For All is
    /// every player for themselves, and Co-op puts both humans on one side.
    pub fn has_sides(self) -> bool {
        matches!(self, Mode::OneVOne | Mode::TwoVTwo | Mode::ThreeVThree)
    }

    /// True when the mode fields AI tanks. Co-op is the only one that does.
    pub fn has_ai(self) -> bool {
        matches!(self, Mode::Coop)
    }

    /// True when friendly fire is on whatever the host chose. Free For All is
    /// always on.
    pub fn friendly_fire_forced(self) -> bool {
        matches!(self, Mode::FreeForAll)
    }
}

/// Which side a player is on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Team {
    A,
    B,
    /// Free For All. Co-op is not `None`: both humans share side A.
    None,
}

/// Where a room is in its life. `Playing` is what blocks new joins.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Waiting,
    Playing,
}

/// One player in a lobby.
#[derive(Clone, Debug)]
pub struct Player {
    pub name: String,
    pub team: Team,
    pub ready: bool,
}

/// One room's lobby.
///
/// `players` is private and stays ordered: `players[0]` is the host, so the host
/// passing to "the next player who joined" is a removal and nothing else. An
/// empty room is removed from the registry, which is why `host` can never be
/// empty.
#[derive(Debug)]
pub struct Room {
    pub code: String,
    pub mode: Mode,
    /// Locked once the match starts (plan section 3, "Friendly fire").
    pub friendly_fire: bool,
    /// Public rooms appear in the room list. Private ones are joined by code.
    pub public: bool,
    pub status: Status,
    /// A room password. Held in memory and compared exactly; it is never logged.
    password: Option<String>,
    players: Vec<Player>,
}

impl Room {
    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// The host: whoever joined first, or took over when the previous host left.
    pub fn host(&self) -> &str {
        debug_assert!(
            !self.players.is_empty(),
            "an empty room should have been closed"
        );
        self.players.first().map(|p| p.name.as_str()).unwrap_or("")
    }

    pub fn password_required(&self) -> bool {
        self.password.is_some()
    }

    /// How many players are on a side.
    pub fn side_count(&self, team: Team) -> usize {
        self.players.iter().filter(|p| p.team == team).count()
    }

    /// The side a joining player goes to: the smaller one, ties to A (plan
    /// section 3, "Teams in the lobby"). Free For All has no sides; Co-op puts
    /// everyone on A.
    fn next_team(&self) -> Team {
        if !self.mode.has_sides() {
            return if self.mode.has_ai() { Team::A } else { Team::None };
        }
        if self.side_count(Team::A) <= self.side_count(Team::B) {
            Team::A
        } else {
            Team::B
        }
    }

    /// The plan's start conditions (section 3, "Start conditions"): enough on
    /// each side, and every player ready.
    ///
    /// The per-side *maximum* is not checked here because joining already
    /// enforces it: a side is filled to the smaller one first, so no side can
    /// pass half the mode's player count. That stops being obvious once the host
    /// can move players, which is when this needs the check too.
    pub fn can_start(&self) -> Result<(), StartError> {
        if self.mode.has_sides() {
            let need = if self.mode == Mode::OneVOne { 1 } else { 2 };
            for team in [Team::A, Team::B] {
                if self.side_count(team) < need {
                    return Err(StartError::SideTooSmall);
                }
            }
        } else if self.players.len() < 2 {
            return Err(StartError::TooFewPlayers);
        }
        if self.players.iter().any(|p| !p.ready) {
            return Err(StartError::NotAllReady);
        }
        Ok(())
    }
}

/// Why a join was refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JoinError {
    NoSuchRoom,
    WrongPassword,
    /// The match has started (plan section 3, "Room rules").
    InProgress,
    /// Someone in the room already has that name.
    NameTaken,
    RoomFull,
    /// Empty, or longer than [`MAX_NAME_LEN`].
    BadName,
    /// The player is not in that room, for `leave` and `set_ready`.
    NotInRoom,
}

/// Why a match cannot start.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StartError {
    NoSuchRoom,
    /// A side has fewer players than the mode needs.
    SideTooSmall,
    /// A non-team mode has fewer than two players.
    TooFewPlayers,
    NotAllReady,
}

/// One row of the public room list (plan section 3, "Room list").
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RoomSummary {
    pub code: String,
    pub host: String,
    pub mode: Mode,
    pub players: u8,
    pub status: Status,
}

/// Every room on the server, plus the code generator.
///
/// Seeded from `game-core`'s RNG so the same seed replays the same codes, which
/// is what makes this testable without touching the clock.
pub struct Registry {
    rooms: HashMap<String, Room>,
    rng: Rng,
}

impl Registry {
    pub fn new(seed: u32) -> Self {
        Self {
            rooms: HashMap::new(),
            rng: Rng::new(seed),
        }
    }

    pub fn room(&self, code: &str) -> Option<&Room> {
        self.rooms.get(code)
    }

    pub fn len(&self) -> usize {
        self.rooms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty()
    }

    /// A fresh code that no room is using.
    ///
    /// shortcut: retries until one is free. With 33 million codes and at most a
    /// few hundred rooms this will not spin; revisit if rooms ever number in the
    /// millions.
    fn fresh_code(&mut self) -> String {
        loop {
            let code: String = (0..CODE_LEN)
                .map(|_| CODE_ALPHABET[self.rng.below(32) as usize] as char)
                .collect();
            if !self.rooms.contains_key(&code) {
                return code;
            }
        }
    }

    /// Create a room and return its code (plan section 3, "Host options").
    ///
    /// The host is a player in their own room, not a separate thing, so they
    /// count toward the mode's player count and can be replaced by the host
    /// transfer.
    pub fn create(
        &mut self,
        host: &str,
        mode: Mode,
        friendly_fire: bool,
        public: bool,
        password: Option<String>,
    ) -> Result<String, JoinError> {
        let name = match clean_name(host) {
            Ok(name) => name,
            Err(e) => {
                warn!(host, reason = ?e, "room refused");
                return Err(e);
            }
        };
        // Free For All is always on, whatever the host asked for.
        let friendly_fire = friendly_fire || mode.friendly_fire_forced();
        let code = self.fresh_code();
        let mut room = Room {
            code: code.clone(),
            mode,
            friendly_fire,
            public,
            status: Status::Waiting,
            password: password.filter(|p| !p.is_empty()),
            players: Vec::new(),
        };
        room.players.push(Player {
            name,
            team: room.next_team(),
            ready: false,
        });
        info!(code = %code, mode = ?mode, public, "room created");
        self.rooms.insert(code.clone(), room);
        Ok(code)
    }

    /// Join a room by code (plan section 3, "Join Room").
    ///
    /// Returns the side the player was put on.
    pub fn join(
        &mut self,
        code: &str,
        name: &str,
        password: Option<&str>,
    ) -> Result<Team, JoinError> {
        match self.try_join(code, name, password) {
            Ok(team) => {
                info!(code, player = name, ?team, "player joined");
                Ok(team)
            }
            Err(e) => {
                warn!(code, player = name, reason = ?e, "join refused");
                Err(e)
            }
        }
    }

    fn try_join(
        &mut self,
        code: &str,
        name: &str,
        password: Option<&str>,
    ) -> Result<Team, JoinError> {
        let name = clean_name(name)?;
        let room = self.rooms.get_mut(code).ok_or(JoinError::NoSuchRoom)?;

        // "No joining after the match starts."
        if room.status != Status::Waiting {
            return Err(JoinError::InProgress);
        }
        if let Some(expected) = &room.password {
            if password != Some(expected.as_str()) {
                return Err(JoinError::WrongPassword);
            }
        }
        if room.players.iter().any(|p| p.name == name) {
            return Err(JoinError::NameTaken);
        }
        if room.players.len() as u8 >= room.mode.max_players() {
            return Err(JoinError::RoomFull);
        }
        let team = room.next_team();
        room.players.push(Player {
            name,
            team,
            ready: false,
        });
        Ok(team)
    }

    /// Remove a player. The host passes to the next player who joined, and an
    /// empty room closes (plan section 3, "Room rules").
    pub fn leave(&mut self, code: &str, name: &str) -> Result<(), JoinError> {
        let Some(room) = self.rooms.get_mut(code) else {
            warn!(code, player = name, "leave refused, no such room");
            return Err(JoinError::NoSuchRoom);
        };
        let Some(idx) = room.players.iter().position(|p| p.name == name) else {
            warn!(code, player = name, "leave refused, not in room");
            return Err(JoinError::NotInRoom);
        };
        room.players.remove(idx);
        if room.players.is_empty() {
            self.rooms.remove(code);
            info!(code, "room closed, empty");
        } else {
            info!(code, player = name, host = room.host(), "player left");
        }
        Ok(())
    }

    /// Mark a player ready or not. The host starts the match.
    pub fn set_ready(&mut self, code: &str, name: &str, ready: bool) -> Result<(), JoinError> {
        let Some(room) = self.rooms.get_mut(code) else {
            return Err(JoinError::NoSuchRoom);
        };
        let Some(player) = room.players.iter_mut().find(|p| p.name == name) else {
            return Err(JoinError::NotInRoom);
        };
        player.ready = ready;
        info!(code, player = name, ready, "ready changed");
        Ok(())
    }

    /// Start the match if the lobby allows it.
    pub fn start(&mut self, code: &str) -> Result<(), StartError> {
        match self.try_start(code) {
            Ok(()) => {
                info!(code, "match started");
                Ok(())
            }
            Err(e) => {
                warn!(code, reason = ?e, "start refused");
                Err(e)
            }
        }
    }

    fn try_start(&mut self, code: &str) -> Result<(), StartError> {
        let room = self.rooms.get_mut(code).ok_or(StartError::NoSuchRoom)?;
        room.can_start()?;
        room.status = Status::Playing;
        Ok(())
    }

    /// Public rooms, in-progress ones included and marked, as the plan asks.
    ///
    /// Sorted by code so the list does not jump around: a `HashMap` has no order
    /// of its own, and the list refreshes on a timer.
    pub fn list_public(&self) -> Vec<RoomSummary> {
        let mut out: Vec<RoomSummary> = self
            .rooms
            .values()
            .filter(|r| r.public)
            .map(|r| RoomSummary {
                code: r.code.clone(),
                host: r.host().to_string(),
                mode: r.mode,
                players: r.players.len() as u8,
                status: r.status,
            })
            .collect();
        out.sort_by(|a, b| a.code.cmp(&b.code));
        out
    }
}

/// Trim a name and reject the ones a lobby cannot show.
fn clean_name(name: &str) -> Result<String, JoinError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_LEN {
        return Err(JoinError::BadName);
    }
    Ok(name.to_string())
}
