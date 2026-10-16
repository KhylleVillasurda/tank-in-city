//! The lobby over a real socket: plan section 9's messages against plan
//! section 3's rules.
//!
//! The server runs in-process on a free port and the clients are real
//! WebSocket connections, so this covers the upgrade, the JSON on the wire, and
//! the broadcasts — everything the browser client does. The rules themselves are
//! covered once in `tests/rooms.rs`; these tests check the wiring.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use server::ws::{self, Hub};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// How long to wait for messages that should already be on their way. The
/// server is in-process, so this only has to cover a task hop.
const SETTLE: Duration = Duration::from_millis(50);

/// Start the server on a free port and return its socket URL.
async fn start() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = ws::router(Hub::new(7));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("ws://{addr}/ws")
}

struct Client {
    socket: Socket,
}

impl Client {
    async fn connect(url: &str) -> Self {
        let (socket, _) = connect_async(url).await.unwrap();
        Self { socket }
    }

    /// Send exactly what a browser client would send.
    async fn send(&mut self, text: &str) {
        self.socket
            .send(Message::Text(text.to_string().into()))
            .await
            .unwrap();
    }

    async fn read(&mut self) -> Value {
        loop {
            let msg = self.socket.next().await.expect("socket closed").unwrap();
            if let Message::Text(text) = msg {
                return serde_json::from_str(text.as_str()).expect("the server sent invalid JSON");
            }
        }
    }

    /// The state after an action: wait for the first message, then take the last
    /// one that arrives with it. An action can push more than one message, and
    /// which of them arrives last is not something to depend on by hand.
    async fn settle(&mut self) -> Value {
        let mut last = self.read().await;
        while let Ok(msg) = tokio::time::timeout(SETTLE, self.read()).await {
            last = msg;
        }
        last
    }
}

const CREATE_1V1: &str =
    r#"{"type":"create_room","name":"ann","mode":"1v1","friendly_fire":false,"public":true}"#;

/// A started 1v1: two players, both ready, host started. Returns the URL, the
/// room code, and both clients.
async fn started_1v1() -> (String, String, Client, Client) {
    let url = start().await;
    let mut host = Client::connect(&url).await;
    let mut guest = Client::connect(&url).await;

    host.send(CREATE_1V1).await;
    let code = host.settle().await["code"].as_str().unwrap().to_string();

    guest
        .send(&format!(
            r#"{{"type":"join_room","name":"bo","code":"{code}"}}"#
        ))
        .await;
    guest.settle().await;
    host.settle().await;

    for client in [&mut host, &mut guest] {
        client.send(r#"{"type":"set_ready","ready":true}"#).await;
    }
    host.settle().await;
    guest.settle().await;

    host.send(r#"{"type":"start_match"}"#).await;
    assert_eq!(host.settle().await["status"], "playing");
    assert_eq!(guest.settle().await["status"], "playing");

    (url, code, host, guest)
}

#[tokio::test]
async fn two_players_lobby_and_start_a_1v1() {
    let url = start().await;
    let mut host = Client::connect(&url).await;
    let mut guest = Client::connect(&url).await;

    // The host creates the room and is put in it.
    host.send(CREATE_1V1).await;
    let state = host.settle().await;
    assert_eq!(state["type"], "room_state");
    assert_eq!(state["host"], "ann");
    assert_eq!(state["status"], "waiting");
    assert_eq!(state["friendly_fire"], false);
    assert_eq!(state["players"][0]["name"], "ann");
    assert_eq!(state["players"][0]["team"], "a");
    assert_eq!(state["players"][0]["ready"], false);
    let code = state["code"].as_str().unwrap().to_string();
    assert_eq!(code.chars().count(), 5, "{code}");

    // The guest joins, and both of them see the new roster without asking.
    guest
        .send(&format!(
            r#"{{"type":"join_room","name":"bo","code":"{code}"}}"#
        ))
        .await;
    let joined = guest.settle().await;
    assert_eq!(joined["players"][1]["name"], "bo");
    assert_eq!(joined["players"][1]["team"], "b", "the smaller side first");
    let seen = host.settle().await;
    assert_eq!(seen["players"].as_array().unwrap().len(), 2);

    // Ready states travel to the whole room.
    host.send(r#"{"type":"set_ready","ready":true}"#).await;
    assert_eq!(host.settle().await["players"][0]["ready"], true);
    assert_eq!(guest.settle().await["players"][0]["ready"], true);

    guest.send(r#"{"type":"set_ready","ready":true}"#).await;
    host.settle().await;
    assert_eq!(guest.settle().await["players"][1]["ready"], true);

    // Only the host starts.
    guest.send(r#"{"type":"start_match"}"#).await;
    let refused = guest.settle().await;
    assert_eq!(refused["type"], "error");
    assert_eq!(refused["message"], "only the host can start the match");

    host.send(r#"{"type":"start_match"}"#).await;
    assert_eq!(host.settle().await["status"], "playing");
    assert_eq!(guest.settle().await["status"], "playing");
}

#[tokio::test]
async fn a_room_in_progress_refuses_new_players() {
    let (url, code, _host, _guest) = started_1v1().await;

    let mut late = Client::connect(&url).await;
    late.send(&format!(
        r#"{{"type":"join_room","name":"cy","code":"{code}"}}"#
    ))
    .await;
    let refused = late.settle().await;
    assert_eq!(refused["type"], "error");
    assert_eq!(refused["message"], "that match has already started");
}

#[tokio::test]
async fn the_public_list_shows_a_public_room_and_hides_a_private_one() {
    let url = start().await;
    let mut host = Client::connect(&url).await;

    host.send(
        r#"{"type":"create_room","name":"ann","mode":"coop","friendly_fire":false,"public":true}"#,
    )
    .await;
    host.settle().await;
    host.send(
        r#"{"type":"create_room","name":"ann","mode":"coop","friendly_fire":false,"public":false}"#,
    )
    .await;
    // Refused: this connection is already in a room, which is its own test.
    assert_eq!(host.settle().await["type"], "error");

    host.send(r#"{"type":"list_rooms"}"#).await;
    let list = host.settle().await;
    assert_eq!(list["type"], "room_list");
    let rooms = list["rooms"].as_array().unwrap();
    assert_eq!(rooms.len(), 1, "the private room is not listed");
    assert_eq!(rooms[0]["host"], "ann");
    assert_eq!(rooms[0]["mode"], "coop");
    assert_eq!(rooms[0]["players"], 1);
    assert_eq!(rooms[0]["status"], "waiting");
}

#[tokio::test]
async fn leaving_hands_the_room_to_the_next_player() {
    let url = start().await;
    let mut host = Client::connect(&url).await;
    let mut guest = Client::connect(&url).await;

    host.send(CREATE_1V1).await;
    let code = host.settle().await["code"].as_str().unwrap().to_string();
    guest
        .send(&format!(
            r#"{{"type":"join_room","name":"bo","code":"{code}"}}"#
        ))
        .await;
    guest.settle().await;
    host.settle().await;

    host.send(r#"{"type":"leave_room"}"#).await;
    // The leaver hears nothing back: it is no longer in the room. The client
    // clears its own state, because that is right whatever the server says.
    let after = guest.settle().await;
    assert_eq!(after["host"], "bo", "the next player is the host");
    assert_eq!(after["players"].as_array().unwrap().len(), 1);

    // The last one out closes the room.
    guest.send(r#"{"type":"leave_room"}"#).await;
    guest.send(r#"{"type":"list_rooms"}"#).await;
    assert_eq!(guest.settle().await["rooms"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn a_disconnect_takes_the_player_out_of_the_room() {
    let url = start().await;
    let mut host = Client::connect(&url).await;
    let guest = Client::connect(&url).await;

    host.send(CREATE_1V1).await;
    let code = host.settle().await["code"].as_str().unwrap().to_string();
    let mut guest = guest;
    guest
        .send(&format!(
            r#"{{"type":"join_room","name":"bo","code":"{code}"}}"#
        ))
        .await;
    guest.settle().await;
    host.settle().await;

    drop(guest);

    let after = host.settle().await;
    assert_eq!(after["players"].as_array().unwrap().len(), 1);
    assert_eq!(after["host"], "ann", "the host is still here");
}

#[tokio::test]
async fn messages_this_slice_does_not_answer_are_refused_not_ignored() {
    let url = start().await;
    let mut client = Client::connect(&url).await;

    client
        .send(r#"{"type":"input","move":1,"fire":false,"tick":1}"#)
        .await;
    let refused = client.settle().await;
    assert_eq!(refused["type"], "error");
    assert!(
        refused["message"].as_str().unwrap().contains("lobby only"),
        "{refused}"
    );

    client.send(r#"{"type":"not_a_message"}"#).await;
    assert_eq!(client.settle().await["type"], "error");
}

#[tokio::test]
async fn one_connection_is_in_one_room_at_a_time() {
    let url = start().await;
    let mut client = Client::connect(&url).await;

    client.send(CREATE_1V1).await;
    client.settle().await;

    client.send(CREATE_1V1).await;
    let refused = client.settle().await;
    assert_eq!(refused["type"], "error");
    assert_eq!(refused["message"], "leave your room before creating another");
}
