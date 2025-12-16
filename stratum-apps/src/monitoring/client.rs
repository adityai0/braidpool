//! Client monitoring types
//!
//! These types are for monitoring **clients** (downstream connections).
//! Each client can have multiple channels opened with the app.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Information about an extended channel
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExtendedChannelInfo {
    pub channel_id: u32,
    pub user_identity: String,
    pub nominal_hashrate: f32,
    pub target_hex: String,
    pub requested_max_target_hex: String,
    pub extranonce_prefix_hex: String,
    pub full_extranonce_size: usize,
    pub rollable_extranonce_size: u16,
    pub shares_per_minute: f32,
    pub shares_accepted: u32,
    pub share_work_sum: f64,
    pub last_share_sequence_number: u32,
    pub best_diff: f64,
    pub last_batch_accepted: u32,
    pub last_batch_work_sum: f64,
    pub share_batch_size: usize,
}

/// Information about a standard channel
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StandardChannelInfo {
    pub channel_id: u32,
    pub user_identity: String,
    pub nominal_hashrate: f32,
    pub target_hex: String,
    pub requested_max_target_hex: String,
    pub extranonce_prefix_hex: String,
    pub shares_per_minute: f32,
    pub shares_accepted: u32,
    pub share_work_sum: f64,
    pub last_share_sequence_number: u32,
    pub best_diff: f64,
    pub last_batch_accepted: u32,
    pub last_batch_work_sum: f64,
    pub share_batch_size: usize,
}

/// Information about a single client (downstream connection)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClientInfo {
    pub client_id: usize,
    pub extended_channels: Vec<ExtendedChannelInfo>,
    pub standard_channels: Vec<StandardChannelInfo>,
}

impl ClientInfo {
    /// Get total number of channels for this client
    pub fn total_channels(&self) -> usize {
        self.extended_channels.len() + self.standard_channels.len()
    }

    /// Get total hashrate for this client
    pub fn total_hashrate(&self) -> f32 {
        self.extended_channels
            .iter()
            .map(|c| c.nominal_hashrate)
            .sum::<f32>()
            + self
                .standard_channels
                .iter()
                .map(|c| c.nominal_hashrate)
                .sum::<f32>()
    }
}

/// Aggregate information about all clients
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClientsSummary {
    pub total_clients: usize,
    pub total_channels: usize,
    pub extended_channels: usize,
    pub standard_channels: usize,
    pub total_hashrate: f32,
}

/// Trait for monitoring clients (downstream connections)
pub trait ClientsMonitoring: Send + Sync {
    /// Get all clients with their channels
    fn get_clients(&self) -> Vec<ClientInfo>;

    /// Get summary of all clients
    fn get_clients_summary(&self) -> ClientsSummary {
        let clients = self.get_clients();
        let extended: usize = clients.iter().map(|c| c.extended_channels.len()).sum();
        let standard: usize = clients.iter().map(|c| c.standard_channels.len()).sum();

        ClientsSummary {
            total_clients: clients.len(),
            total_channels: extended + standard,
            extended_channels: extended,
            standard_channels: standard,
            total_hashrate: clients.iter().map(|c| c.total_hashrate()).sum(),
        }
    }
}
