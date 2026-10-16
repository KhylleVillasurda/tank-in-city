//! The Tank In City server.
//!
//! Listens on `/ws` for the protocol of plan section 9 and on `/health` so a
//! tunnel can be checked with one request. A room that starts a match gets one
//! `World` per room, ticked at 60 Hz by the loop in `matches.rs`.

use server::rooms::Mode;
use server::ws::{self, Hub};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Lifecycle and errors only, never per-tick state or secrets (plan section
    // 7, Milestone 3). The default filter is INFO, which is exactly that.
    tracing_subscriber::fmt().init();

    let port = port();
    let app = ws::router(Hub::new(seed()));

    // Local only. A Cloudflare tunnel reaches it from this machine, and nothing
    // else needs to; see docs/sharing.md.
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    info!(port, modes = Mode::ALL.len(), "tank-in-city server up");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    info!("tank-in-city server stopped");
    Ok(())
}

/// The port to listen on. `deploy/cloudflared.yml` expects 8080.
fn port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080)
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
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
