//! Tank In City server.
//!
//! The game rules come from `game-core`, the same crate the browser runs, so the
//! server and the client agree on hits, scores and spawns by construction. This
//! crate holds the parts the browser must not have: room state, sessions, and
//! the authoritative tick loop (plan section 4, "Who does what").
//!
//! Milestone 3 slice 1 is the lobby: codes, the public list, teams, ready
//! states, and the start check. Slice 2 puts it on a socket: the lobby messages
//! of plan section 9, and a `room_state` pushed to the room after every change.
//! Slice 4 is the match loop: one `World` per room, ticked at 60 Hz, with
//! `input` in and `map` and `snapshot` out. Rounds and win conditions are next.

pub mod matches;
pub mod rooms;
pub mod ws;
