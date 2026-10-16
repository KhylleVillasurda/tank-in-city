//! The WebSocket layer: the protocol of plan section 9.
//!
//! One connection is one player. This module is a thin translator: it reads a
//! message, calls the room registry or the match loop, and pushes the result to
//! everyone in that room. Every rule stays in `rooms.rs`, so there is no lobby
//! logic here to drift away from the tests there.
//!
//! Slice 4 answers the lobby (`create_room`, `join_room`, `list_rooms`,
//! `set_ready`, `start_match`, `leave_room`) and the match: `start_match` spawns
//! the loop in `matches.rs`, `input` is the other half of it, and `map` and
//! `snapshot` go back out. `reconnect` and `request_swap` are still refused with
//! an error rather than ignored, so a client is never left guessing.
//!
//! Two rules from plan section 10 live here because they are per-connection: a
//! message size cap, and a messages-per-second cap.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use game_core::INPUT_FIRE;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{info, warn};

use crate::matches::{MatchInput, MOVE_MASK};
use crate::rooms::{JoinError, Mode, Player, Registry, Room, StartError, Status};

/// Largest client message this layer will act on (plan section 10, "Message
/// size limit"). A lobby message is a few hundred bytes.
pub const MAX_MESSAGE_BYTES: usize = 8 * 1024;

/// Largest message the socket will even read. Above [`MAX_MESSAGE_BYTES`] on
/// purpose: this one is the backstop against a memory bomb, and the check above
/// is the one that answers with a readable error.
const SOCKET_MESSAGE_LIMIT: usize = 64 * 1024;

/// Messages per second, per connection (plan section 10, "Rate limiting").
///
/// Well above the 60 Hz input rate the match loop will send (plan section 5),
/// so it only ever catches a flood.
const MESSAGES_PER_SECOND: u32 = 120;

/// Client to server, plan section 9.
///
/// `name` rides along with `create_room` and `join_room` because the plan's own
/// section 3 says a player's name is their username or a generated guest name,
/// and `web/game/guest.ts` already generates one. The plan's message list omits
/// it; see `docs/milestone-3.md`.
///
/// `start_match` is not in the plan's list either, and it has to exist: section 3
/// says the host starts the match.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    CreateRoom {
        name: String,
        mode: Mode,
        friendly_fire: bool,
        public: bool,
        #[serde(default)]
        password: Option<String>,
    },
    JoinRoom {
        name: String,
        code: String,
        #[serde(default)]
        password: Option<String>,
    },
    ListRooms,
    SetReady {
        ready: bool,
    },
    StartMatch,
    LeaveRoom,
    /// The plan's `input { move, fire, tick }`. `tick` is deliberately not
    /// declared: the loop repeats each player's latest input every tick, so a
    /// tick number has nothing to decide (plan section 5, "Missing input"), and
    /// serde drops a field it does not know. See `docs/milestone-3.md`.
    Input {
        /// The direction bits: up, down, left, right. `fire` is separate.
        #[serde(default)]
        r#move: u32,
        #[serde(default)]
        fire: bool,
    },
    /// The messages this slice still does not answer: `reconnect` and
    /// `request_swap`. One arm instead of two dead ones, and it keeps a typo
    /// from looking like a handled message.
    #[serde(other)]
    Unsupported,
}

/// Server to client, plan section 9.
///
/// `session { token }` is not here yet: a session token is only useful with the
/// 30-second reconnect, which needs player identity across a disconnect. The
/// client knows the socket is live from the browser's own open event.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    RoomList {
        rooms: Vec<crate::rooms::RoomSummary>,
    },
    RoomState(RoomState),
    /// The full tile map, when a match starts and at the start of each stage
    /// (plan section 5, "Map transfer"). Row-major, `MAP_H` rows of `MAP_W`.
    Map {
        tiles: Vec<u8>,
    },
    /// The flat `[i32; SNAP_LEN]` state buffer from `World::write_snapshot`, at
    /// `SNAPSHOT_HZ`. The fixed offsets are in `game-core`, so the client needs
    /// no dynamic arithmetic; see the layout table in `docs/milestone-3.md`.
    Snapshot {
        data: Vec<i32>,
    },
    Error {
        message: String,
    },
}

/// A room as the lobby sees it (plan section 9, `room_state`).
///
/// The plan also lists a `teams` field. It is left out on purpose: every player
/// carries their own `team`, so the sides are already in `players`, and a second
/// copy of the same fact is a second thing that can be wrong. It also saves the
/// client a copy of the mode table — it groups by whatever sides are present,
/// which is correct for team modes, Free For All and Co-op alike.
#[derive(Debug, Clone, Serialize)]
pub struct RoomState {
    pub code: String,
    pub mode: Mode,
    pub friendly_fire: bool,
    pub host: String,
    pub status: Status,
    pub players: Vec<Player>,
}

impl From<&Room> for RoomState {
    fn from(room: &Room) -> Self {
        Self {
            code: room.code.clone(),
            mode: room.mode,
            friendly_fire: room.friendly_fire,
            host: room.host().to_string(),
            status: room.status,
            players: room.players().to_vec(),
        }
    }
}

/// Where one connection sits: the name the registry knows it by, and its room.
///
/// One field rather than two `Option`s so "has a name but no room" is not a
/// state that can exist.
#[derive(Clone)]
struct Seat {
    name: String,
    code: String,
}

/// One open socket.
struct Client {
    seat: Option<Seat>,
    tx: UnboundedSender<ServerMessage>,
}

/// Every room, every open socket, and every running match.
///
/// The locks are never held at the same time — see [`Hub::registry`] — so there
/// is no lock order to get wrong.
pub struct Hub {
    rooms: Mutex<Registry>,
    clients: Mutex<HashMap<u64, Client>>,
    /// One sender per room running a match, so an `input` knows where to go.
    /// A finished match leaves its entry behind; the next `start_match` for that
    /// code overwrites it, and a send into a dead one is dropped.
    matches: Mutex<HashMap<String, UnboundedSender<MatchInput>>>,
    next_id: AtomicU64,
    /// Seeds for `World::with_setup`. Room codes come from the registry's own
    /// RNG; matches only have to differ from one another.
    next_seed: AtomicU32,
}

impl Hub {
    pub fn new(seed: u32) -> Arc<Self> {
        Arc::new(Self {
            rooms: Mutex::new(Registry::new(seed)),
            clients: Mutex::new(HashMap::new()),
            matches: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            next_seed: AtomicU32::new(seed),
        })
    }

    /// Run something against the registry. The lock is taken and released in
    /// here, so a caller cannot accidentally hold it across a second lock or an
    /// `.await`.
    fn registry<T>(&self, f: impl FnOnce(&mut Registry) -> T) -> T {
        f(&mut self.rooms.lock().expect("registry lock"))
    }

    fn clients<T>(&self, f: impl FnOnce(&mut HashMap<u64, Client>) -> T) -> T {
        f(&mut self.clients.lock().expect("client lock"))
    }

    fn matches<T>(
        &self,
        f: impl FnOnce(&mut HashMap<String, UnboundedSender<MatchInput>>) -> T,
    ) -> T {
        f(&mut self.matches.lock().expect("match lock"))
    }

    /// Register a socket and hand back its id and its outbox.
    fn connect(&self) -> (u64, UnboundedReceiver<ServerMessage>) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = unbounded_channel();
        self.clients(|c| c.insert(id, Client { seat: None, tx }));
        (id, rx)
    }

    /// A closed socket leaves its room, and the room hears about it.
    fn disconnect(&self, id: u64) {
        let client = self.clients(|c| c.remove(&id));
        let Some(Seat { name, code }) = client.and_then(|c| c.seat) else {
            return;
        };
        let left = self.registry(|r| r.leave(&code, &name));
        if left.is_ok() {
            info!(code, player = %name, "player left on disconnect");
            self.broadcast_room(&code);
        }
    }

    fn seat_of(&self, id: u64) -> Option<Seat> {
        self.clients(|c| c.get(&id).and_then(|client| client.seat.clone()))
    }

    fn seat(&self, id: u64, name: &str, code: &str) {
        self.clients(|c| {
            if let Some(client) = c.get_mut(&id) {
                client.seat = Some(Seat {
                    name: name.to_string(),
                    code: code.to_string(),
                });
            }
        });
    }

    fn unseat(&self, id: u64) {
        self.clients(|c| {
            if let Some(client) = c.get_mut(&id) {
                client.seat = None;
            }
        });
    }

    fn send_to(&self, id: u64, msg: ServerMessage) {
        self.clients(|c| {
            if let Some(client) = c.get(&id) {
                let _ = client.tx.send(msg);
            }
        });
    }

    fn refuse(&self, id: u64, message: &str) {
        self.send_to(
            id,
            ServerMessage::Error {
                message: message.to_string(),
            },
        );
    }

    fn room_state(&self, code: &str) -> Option<RoomState> {
        self.registry(|r| r.room(code).map(RoomState::from))
    }

    /// One message to everyone in a room.
    pub(crate) fn broadcast(&self, code: &str, msg: ServerMessage) {
        self.clients(|c| {
            for client in c.values() {
                if client.seat.as_ref().is_some_and(|s| s.code == code) {
                    let _ = client.tx.send(msg.clone());
                }
            }
        });
    }

    /// Send the room's current state to everyone in it. Called after anything
    /// that changes a lobby, so no client has to poll.
    fn broadcast_room(&self, code: &str) {
        if let Some(state) = self.room_state(code) {
            self.broadcast(code, ServerMessage::RoomState(state));
        }
    }

    /// Whether a room still exists. The match loop asks, so a room everyone left
    /// does not keep a `World` ticking for nobody.
    pub(crate) fn room_exists(&self, code: &str) -> bool {
        self.registry(|r| r.room(code).is_some())
    }

    /// A match has finished: the room goes back to the lobby so another can
    /// start. False when the room is already gone.
    pub(crate) fn finish_match(&self, code: &str) -> bool {
        let ended = self.registry(|r| r.finish(code));
        if ended {
            info!(code, "room back in the lobby");
            self.broadcast_room(code);
        }
        ended
    }

    /// Spawn a room's match loop.
    ///
    /// Everything the loop needs is read while the registry lock is held, then
    /// the lock is released before the task is spawned.
    fn start_match(self: &Arc<Self>, code: &str) {
        // Read the mode and the roster in one visit, so the lock is released
        // before anything is spawned.
        let prepared = self.registry(|r| {
            r.room(code).map(|room| {
                (
                    crate::matches::setup_for(room),
                    room.players()
                        .iter()
                        .map(|p| p.name.clone())
                        .collect::<Vec<_>>(),
                )
            })
        });
        let Some((setup, roster)) = prepared else {
            return;
        };

        let (tx, rx) = unbounded_channel();
        self.matches(|m| {
            // A previous sender for this code belongs to a finished match. It is
            // dropped here, which is how a stale loop is told to stop.
            m.insert(code.to_string(), tx);
        });

        let seed = self.next_seed.fetch_add(1, Ordering::Relaxed);
        tokio::spawn(crate::matches::run(
            Arc::clone(self),
            code.to_string(),
            seed,
            rx,
            setup,
            roster,
        ));
    }

    /// Answer one message. Deliberately synchronous: nothing here waits, so no
    /// lock can be held across an `.await`.
    fn handle(self: &Arc<Self>, id: u64, msg: ClientMessage) {
        match msg {
            ClientMessage::ListRooms => {
                let rooms = self.registry(|r| r.list_public());
                self.send_to(id, ServerMessage::RoomList { rooms });
            }
            ClientMessage::CreateRoom {
                name,
                mode,
                friendly_fire,
                public,
                password,
            } => {
                if self.seat_of(id).is_some() {
                    return self.refuse(id, "leave your room before creating another");
                }
                let created = self.registry(|r| r.create(&name, mode, friendly_fire, public, password));
                match created {
                    Ok(code) => {
                        self.seat(id, &name, &code);
                        self.broadcast_room(&code);
                    }
                    Err(e) => self.refuse(id, join_error(e)),
                }
            }
            ClientMessage::JoinRoom {
                name,
                code,
                password,
            } => {
                if self.seat_of(id).is_some() {
                    return self.refuse(id, "leave your room before joining another");
                }
                let joined = self.registry(|r| r.join(&code, &name, password.as_deref()));
                match joined {
                    Ok(_team) => {
                        self.seat(id, &name, &code);
                        self.broadcast_room(&code);
                    }
                    Err(e) => self.refuse(id, join_error(e)),
                }
            }
            ClientMessage::SetReady { ready } => {
                let Some(seat) = self.seat_of(id) else {
                    return self.refuse(id, "join a room first");
                };
                let done = self.registry(|r| r.set_ready(&seat.code, &seat.name, ready));
                match done {
                    Ok(()) => self.broadcast_room(&seat.code),
                    Err(e) => self.refuse(id, join_error(e)),
                }
            }
            ClientMessage::StartMatch => {
                let Some(seat) = self.seat_of(id) else {
                    return self.refuse(id, "join a room first");
                };
                // The host check is the registry's, not this layer's.
                let done = self.registry(|r| r.start(&seat.code, &seat.name));
                match done {
                    Ok(()) => {
                        self.broadcast_room(&seat.code);
                        self.start_match(&seat.code);
                    }
                    Err(e) => self.refuse(id, start_error(e)),
                }
            }
            ClientMessage::Input { r#move, fire } => {
                let Some(seat) = self.seat_of(id) else {
                    return self.refuse(id, "join a room first");
                };
                // Plan section 10: the server decides, so unknown bits are
                // dropped rather than trusted. An input while no match is
                // running is dropped too: a client sends one per tick, and an
                // error for each would be noise, not information.
                let bits = (r#move & MOVE_MASK) | if fire { INPUT_FIRE } else { 0 };
                self.matches(|m| {
                    if let Some(tx) = m.get(&seat.code) {
                        let _ = tx.send(MatchInput {
                            name: seat.name.clone(),
                            bits,
                        });
                    }
                });
            }
            ClientMessage::LeaveRoom => {
                let Some(seat) = self.seat_of(id) else {
                    return self.refuse(id, "you are not in a room");
                };
                let done = self.registry(|r| r.leave(&seat.code, &seat.name));
                self.unseat(id);
                match done {
                    Ok(()) => self.broadcast_room(&seat.code),
                    Err(e) => self.refuse(id, join_error(e)),
                }
            }
            ClientMessage::Unsupported => self.refuse(
                id,
                "that message is not handled yet: reconnect and swapping come later",
            ),
        }
    }
}

/// The server's routes. `/ws` is the game socket; `/health` is there so a
/// tunnel can be checked with one `curl`.
pub fn router(hub: Arc<Hub>) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(upgrade))
        .with_state(hub)
}

async fn upgrade(ws: WebSocketUpgrade, State(hub): State<Arc<Hub>>) -> impl IntoResponse {
    ws.max_message_size(SOCKET_MESSAGE_LIMIT)
        .on_upgrade(move |socket| serve(hub, socket))
}

/// One connection, start to finish.
async fn serve(hub: Arc<Hub>, mut socket: WebSocket) {
    let (id, mut outbox) = hub.connect();
    info!(connection = id, "socket opened");
    let mut limiter = Limiter::new();

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let text = match incoming {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Close(_))) | None => break,
                    // Binary, ping and pong carry no lobby message.
                    Some(Ok(_)) => continue,
                    Some(Err(e)) => {
                        warn!(connection = id, error = %e, "socket error");
                        break;
                    }
                };
                if text.len() > MAX_MESSAGE_BYTES {
                    warn!(connection = id, bytes = text.len(), "oversized message");
                    hub.refuse(id, "that message is too large");
                    continue;
                }
                if !limiter.allow(Instant::now()) {
                    warn!(connection = id, "rate limit tripped");
                    hub.refuse(id, "too many messages; slow down");
                    break;
                }
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(msg) => hub.handle(id, msg),
                    Err(e) => {
                        warn!(connection = id, error = %e, "unreadable message");
                        hub.refuse(id, "could not read that message");
                    }
                }
            }
            outgoing = outbox.recv() => {
                let Some(msg) = outgoing else { break };
                if socket.send(Message::Text(to_json(&msg).into())).await.is_err() {
                    break;
                }
            }
        }
    }

    hub.disconnect(id);
    info!(connection = id, "socket closed");
}

/// Encode one message. These types are all plain data, so this cannot fail;
/// the fallback exists so a bug here is a bad message, not a panic.
fn to_json(msg: &ServerMessage) -> String {
    serde_json::to_string(msg)
        .unwrap_or_else(|_| r#"{"type":"error","message":"the server could not encode that"}"#.into())
}

fn join_error(e: JoinError) -> &'static str {
    match e {
        JoinError::NoSuchRoom => "no room with that code",
        JoinError::WrongPassword => "wrong password",
        JoinError::InProgress => "that match has already started",
        JoinError::NameTaken => "someone in that room already has your name",
        JoinError::RoomFull => "that room is full",
        JoinError::BadName => "that name is empty or too long",
        JoinError::NotInRoom => "you are not in that room",
    }
}

fn start_error(e: StartError) -> &'static str {
    match e {
        StartError::NoSuchRoom => "no room with that code",
        StartError::NotHost => "only the host can start the match",
        StartError::SideTooSmall => "each side needs enough players",
        StartError::TooFewPlayers => "this mode needs at least two players",
        StartError::NotAllReady => "every player has to be ready",
    }
}

/// A one-second window of message counts.
struct Limiter {
    window: Instant,
    count: u32,
}

impl Limiter {
    fn new() -> Self {
        Self {
            window: Instant::now(),
            count: 0,
        }
    }

    /// False once this connection has gone over the cap for the current second.
    /// `now` is an argument so a test can move the clock without sleeping.
    fn allow(&mut self, now: Instant) -> bool {
        if now.duration_since(self.window) >= Duration::from_secs(1) {
            self.window = now;
            self.count = 0;
        }
        self.count += 1;
        self.count <= MESSAGES_PER_SECOND
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::Team;

    #[test]
    fn the_wire_names_are_the_ones_the_plan_uses() {
        assert_eq!(serde_json::to_string(&Mode::OneVOne).unwrap(), "\"1v1\"");
        assert_eq!(serde_json::to_string(&Mode::FreeForAll).unwrap(), "\"free_for_all\"");
        assert_eq!(serde_json::to_string(&Mode::Coop).unwrap(), "\"coop\"");
        assert_eq!(serde_json::to_string(&Status::Waiting).unwrap(), "\"waiting\"");
        assert_eq!(serde_json::to_string(&Team::None).unwrap(), "\"none\"");
    }

    #[test]
    fn a_create_room_message_reads_as_the_plan_writes_it() {
        let text = r#"{"type":"create_room","name":"ann","mode":"free_for_all",
            "friendly_fire":false,"public":true}"#;
        match serde_json::from_str::<ClientMessage>(text).unwrap() {
            ClientMessage::CreateRoom {
                name,
                mode,
                friendly_fire,
                public,
                password,
            } => {
                assert_eq!(name, "ann");
                assert_eq!(mode, Mode::FreeForAll);
                assert!(!friendly_fire);
                assert!(public);
                assert!(password.is_none(), "an omitted password is no password");
            }
            other => panic!("read as {other:?}"),
        }
    }

    #[test]
    fn an_input_message_reads_as_the_plan_writes_it() {
        let text = r#"{"type":"input","move":8,"fire":true,"tick":3}"#;
        let parsed = serde_json::from_str::<ClientMessage>(text).unwrap();
        assert!(
            matches!(parsed, ClientMessage::Input { r#move: 8, fire: true }),
            "the plan's move/fire shape must read, and its tick field is ignored"
        );
    }

    #[test]
    fn the_messages_still_to_come_read_as_unsupported() {
        for text in [
            r#"{"type":"reconnect","code":"ABCDE","session_token":"x"}"#,
            r#"{"type":"request_swap"}"#,
            r#"{"type":"nonsense"}"#,
        ] {
            let parsed = serde_json::from_str::<ClientMessage>(text).unwrap();
            assert!(matches!(parsed, ClientMessage::Unsupported), "{text}");
        }
    }

    #[test]
    fn room_state_carries_each_players_own_side() {
        let mut reg = Registry::new(7);
        let code = reg.create("ann", Mode::TwoVTwo, true, true, None).unwrap();
        reg.join(&code, "bo", None).unwrap();

        let state = RoomState::from(reg.room(&code).unwrap());
        assert_eq!(state.code, code);
        assert_eq!(state.host, "ann");
        assert_eq!(state.status, Status::Waiting);
        assert_eq!(state.players.len(), 2);
        assert_eq!(state.players[0].team, Team::A);
        assert_eq!(state.players[1].team, Team::B);
    }

    #[test]
    fn the_rate_limit_allows_a_burst_and_then_stops() {
        let mut limiter = Limiter::new();
        let start = Instant::now();
        for n in 1..=MESSAGES_PER_SECOND {
            assert!(limiter.allow(start), "message {n} is inside the cap");
        }
        assert!(!limiter.allow(start), "one over the cap is refused");
        assert!(
            limiter.allow(start + Duration::from_secs(1)),
            "a fresh second starts over"
        );
    }
}
