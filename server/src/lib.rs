//! Tank In City server.
//!
//! The game rules come from `game-core`, the same crate the browser runs, so the
//! server and the client agree on hits, scores and spawns by construction. This
//! crate holds the parts the browser must not have: room state, sessions, and
//! the authoritative tick loop (plan section 4, "Who does what").
//!
//! Milestone 3 slice 1 is the lobby: codes, the public list, teams, ready
//! states, and the start check. The socket and the match loop come next.

pub mod rooms;
