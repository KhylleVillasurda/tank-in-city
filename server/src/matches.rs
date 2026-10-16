//! The match loop: one `World` per room, ticked at 60 Hz (plan section 5).
//!
//! Slice 4 of Milestone 3. A room that reaches `Playing` gets one of these,
//! spawned by `start_match`. It owns the `World`, so the registry stays lobby
//! bookkeeping and the socket layer stays a translator; this is the only module
//! that runs the simulation.
//!
//! What goes out: the full tile map when the match starts and on every stage
//! change, then the flat `[i32; SNAP_LEN]` snapshot at [`SNAPSHOT_HZ`]. The plan
//! says 15 Hz and asks the client to interpolate; this slice starts at 20.
//!
//! What this slice deliberately leaves alone: rounds, win conditions, scoring
//! and `match_over`. `World::state == GameOver` says the run is over but not
//! *why* — the base falling and every player running out of lives both land
//! there — so the loop stops instead of guessing an outcome. That decision is
//! the rounds slice's; `docs/deferred.md` lists the rest.

use std::sync::Arc;
use std::time::Duration;

use game_core::tank::{MAX_PLAYERS, TEAM_A, TEAM_B};
use game_core::{
    Input, Setup, State, World, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_UP, SNAP_LEN, TICK_HZ,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::MissedTickBehavior;
use tracing::info;

use crate::rooms::{Room, Team};
use crate::ws::{Hub, ServerMessage};

/// Snapshots per second. 191 ints per snapshot at 60 Hz is more than a browser
/// needs, and the client interpolates between them anyway (plan section 5).
pub const SNAPSHOT_HZ: u32 = 20;

/// 60 Hz ticks between snapshots, so the snapshot rate stays a divisor of the
/// simulation rate and the loop needs no clock of its own.
const TICKS_PER_SNAPSHOT: u32 = TICK_HZ / SNAPSHOT_HZ;

/// The bits a client's `move` may set: the four directions. Anything else is
/// masked off, not trusted (plan section 10, "Input validation").
pub const MOVE_MASK: u32 = INPUT_UP | INPUT_DOWN | INPUT_LEFT | INPUT_RIGHT;

/// One player's latest intent, on its way from the socket to the loop.
pub struct MatchInput {
    /// The player's name, because the slot is the match's to decide: a player
    /// leaving mid-match must not renumber anyone in the running `World`.
    pub name: String,
    pub bits: u32,
}

/// The engine facts a room's mode maps to.
///
/// `Setup` carries four things and nothing else; everything else in plan
/// section 6 (rounds, timers, match scoring) is this module's, not the engine's.
pub fn setup_for(room: &Room) -> Setup {
    let mut teams = [TEAM_A; MAX_PLAYERS];
    for (slot, player) in room.players().iter().enumerate() {
        teams[slot] = match player.team {
            Team::A => TEAM_A,
            Team::B => TEAM_B,
            // Free For All: each player gets a side of its own. Kept above the
            // two real sides so no player shares the base's side or the
            // enemies', which is what makes friendly fire always on there.
            Team::None => (2 + slot) as u8,
        };
    }
    Setup {
        players: room.player_count().min(MAX_PLAYERS) as u8,
        waves: room.mode.has_ai(),
        friendly_fire: room.friendly_fire,
        teams,
    }
}

/// Run one room's match until the run ends or the room is gone.
///
/// `inputs` carries each player's latest intent; the loop repeats it every tick,
/// so a dropped or late message is harmless (plan section 5, "Missing input").
pub async fn run(
    hub: Arc<Hub>,
    code: String,
    seed: u32,
    mut inputs: UnboundedReceiver<MatchInput>,
    setup: Setup,
    roster: Vec<String>,
) {
    let mut world = World::with_setup(seed, setup);
    let mut latest = [Input::new(0); MAX_PLAYERS];
    let mut snapshot = vec![0i32; SNAP_LEN];
    let mut stage = world.stage;

    hub.broadcast(
        &code,
        ServerMessage::Map {
            tiles: world.map.tiles.to_vec(),
        },
    );
    info!(code, players = roster.len(), "match loop started");

    // The tick period itself, not 1000/60 ms: that rounds to 16 and runs fast.
    let mut ticker = tokio::time::interval(Duration::from_nanos(1_000_000_000 / TICK_HZ as u64));
    // A stalled server skips the backlog rather than running it as fast as it
    // can, which would spiral instead of catching up.
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let mut since_snapshot = 0;
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                world.step(&latest);
                if world.stage != stage {
                    stage = world.stage;
                    hub.broadcast(&code, ServerMessage::Map { tiles: world.map.tiles.to_vec() });
                }

                let over = world.state == State::GameOver;
                since_snapshot += 1;
                if since_snapshot >= TICKS_PER_SNAPSHOT || over {
                    since_snapshot = 0;
                    if !hub.room_exists(&code) {
                        break;
                    }
                    world.write_snapshot(&mut snapshot);
                    hub.broadcast(&code, ServerMessage::Snapshot { data: snapshot.clone() });
                }
                // The last snapshot carries GameOver, so a client sees the run
                // end rather than waiting for one that never comes.
                if over {
                    break;
                }
            }
            input = inputs.recv() => {
                // The Hub holds the only sender, so a closed channel means this
                // match was replaced or the server is shutting down.
                let Some(input) = input else { break };
                if let Some(slot) = roster.iter().position(|name| *name == input.name) {
                    latest[slot] = Input::new(input.bits);
                }
            }
        }
    }

    // The room goes back to the lobby so another match can start. Post-match
    // scores, rounds and best-of-3 are the rounds slice's.
    if hub.finish_match(&code) {
        info!(code, tick = world.tick, "match ended");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::{Mode, Registry};

    /// A room of `names`, with `names[0]` as host.
    fn room(mode: Mode, names: &[&str]) -> (Registry, String) {
        let mut reg = Registry::new(7);
        let code = reg.create(names[0], mode, true, true, None).unwrap();
        for name in &names[1..] {
            reg.join(&code, name, None).unwrap();
        }
        (reg, code)
    }

    #[test]
    fn a_1v1_becomes_two_sides_and_no_waves() {
        let (reg, code) = room(Mode::OneVOne, &["ann", "bo"]);
        let setup = setup_for(reg.room(&code).unwrap());
        assert_eq!(setup.players, 2);
        assert!(!setup.waves);
        assert_eq!(&setup.teams[..2], &[TEAM_A, TEAM_B]);
    }

    #[test]
    fn coop_becomes_two_players_on_one_side_with_waves() {
        let (reg, code) = room(Mode::Coop, &["ann", "bo"]);
        let setup = setup_for(reg.room(&code).unwrap());
        assert_eq!(setup.players, 2);
        assert!(setup.waves, "co-op is the mode with AI waves");
        assert_eq!(&setup.teams[..2], &[TEAM_A, TEAM_A]);
    }

    #[test]
    fn free_for_all_gives_every_player_its_own_side() {
        let (reg, code) = room(Mode::FreeForAll, &["ann", "bo", "cy"]);
        let setup = setup_for(reg.room(&code).unwrap());
        assert_eq!(setup.players, 3);
        assert!(!setup.waves);
        // Distinct, and neither the base's side nor the enemies'.
        assert_eq!(&setup.teams[..3], &[2, 3, 4]);
    }

    #[test]
    fn the_rooms_friendly_fire_reaches_the_engine() {
        let mut reg = Registry::new(7);
        let off = reg.create("ann", Mode::OneVOne, false, true, None).unwrap();
        assert!(!setup_for(reg.room(&off).unwrap()).friendly_fire);
        let on = reg.create("bo", Mode::OneVOne, true, true, None).unwrap();
        assert!(setup_for(reg.room(&on).unwrap()).friendly_fire);
        // Free For All forces it on whatever the host asked for.
        let ffa = reg
            .create("cy", Mode::FreeForAll, false, true, None)
            .unwrap();
        assert!(setup_for(reg.room(&ffa).unwrap()).friendly_fire);
    }
}
