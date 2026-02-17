//! Ping protocol event handlers.

use std::time::Duration;

use libp2p::PeerId;
use tracing::{info, warn};

use crate::peer_manager::PeerManager;

/// Handles successful ping responses.
pub fn handle_ping_success(peer: PeerId, latency: Duration, peer_manager: &mut PeerManager) {
    info!(
        peer = %peer,
        latency_ms = %latency.as_millis(),
        "Ping"
    );
    peer_manager.update_latency(&peer, latency);
}

/// Handles ping failures.
pub fn handle_ping_failure(peer: PeerId, error: libp2p::ping::Failure) {
    warn!(
        peer = %peer,
        error = %error,
        "Ping failed"
    );
}
