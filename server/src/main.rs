//! The Tank In City server.
//!
//! Nothing listens on a socket yet. This boots the logging and the room
//! registry, which is the seam the WebSocket layer plugs into (plan section 5).

use server::rooms::{Mode, Registry};
use tracing::info;

fn main() {
    // Lifecycle and errors only, never per-tick state or secrets (plan section
    // 7, Milestone 3). The default filter is INFO, which is exactly that.
    tracing_subscriber::fmt().init();

    let rooms = Registry::new(seed());
    info!(
        rooms = rooms.len(),
        modes = Mode::ALL.len(),
        "tank-in-city server up"
    );
}

/// A seed for the room codes.
///
/// Not a secret, and not required to be one: the codes are not secrets either,
/// and a private room carries a password on top. It only has to differ between
/// runs, so the clock is enough and no dependency is needed.
fn seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() ^ d.as_secs() as u32)
        .unwrap_or(1)
}
