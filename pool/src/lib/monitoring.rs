//! Monitoring integration for Pool
//!
//! This module implements the ClientsMonitoring trait on `ChannelManager`.
//! Pool only has clients (miners connecting to it), no upstream server.

use stratum_apps::monitoring::client::{
    ClientInfo, ClientsMonitoring, ExtendedChannelInfo, StandardChannelInfo,
};

use crate::{channel_manager::ChannelManager, downstream::Downstream};

/// Helper to convert a Downstream to ClientInfo
fn downstream_to_client_info(client: &Downstream) -> ClientInfo {
    let mut extended_channels = Vec::new();
    let mut standard_channels = Vec::new();

    client
        .downstream_data
        .safe_lock(|dd| {
            for (_channel_id, extended_channel) in dd.extended_channels.iter() {
                let channel_id = extended_channel.get_channel_id();
                let target = extended_channel.get_target();
                let requested_max_target = extended_channel.get_requested_max_target();
                let user_identity = extended_channel.get_user_identity();
                let share_accounting = extended_channel.get_share_accounting();

                extended_channels.push(ExtendedChannelInfo {
                    channel_id,
                    user_identity: user_identity.clone(),
                    nominal_hashrate: extended_channel.get_nominal_hashrate(),
                    target_hex: hex::encode(target.to_be_bytes()),
                    requested_max_target_hex: hex::encode(requested_max_target.to_be_bytes()),
                    extranonce_prefix_hex: hex::encode(extended_channel.get_extranonce_prefix()),
                    full_extranonce_size: extended_channel.get_full_extranonce_size(),
                    rollable_extranonce_size: extended_channel.get_rollable_extranonce_size(),
                    shares_per_minute: extended_channel.get_shares_per_minute(),
                    shares_accepted: share_accounting.get_shares_accepted(),
                    share_work_sum: share_accounting.get_share_work_sum(),
                    last_share_sequence_number: share_accounting.get_last_share_sequence_number(),
                    best_diff: share_accounting.get_best_diff(),
                    last_batch_accepted: share_accounting.get_last_batch_accepted(),
                    last_batch_work_sum: share_accounting.get_last_batch_work_sum(),
                    share_batch_size: share_accounting.get_share_batch_size(),
                });
            }

            for (_channel_id, standard_channel) in dd.standard_channels.iter() {
                let channel_id = standard_channel.get_channel_id();
                let target = standard_channel.get_target();
                let requested_max_target = standard_channel.get_requested_max_target();
                let user_identity = standard_channel.get_user_identity();
                let share_accounting = standard_channel.get_share_accounting();

                standard_channels.push(StandardChannelInfo {
                    channel_id,
                    user_identity: user_identity.clone(),
                    nominal_hashrate: standard_channel.get_nominal_hashrate(),
                    target_hex: hex::encode(target.to_be_bytes()),
                    requested_max_target_hex: hex::encode(requested_max_target.to_be_bytes()),
                    extranonce_prefix_hex: hex::encode(standard_channel.get_extranonce_prefix()),
                    shares_per_minute: standard_channel.get_shares_per_minute(),
                    shares_accepted: share_accounting.get_shares_accepted(),
                    share_work_sum: share_accounting.get_share_work_sum(),
                    last_share_sequence_number: share_accounting.get_last_share_sequence_number(),
                    best_diff: share_accounting.get_best_diff(),
                    last_batch_accepted: share_accounting.get_last_batch_accepted(),
                    last_batch_work_sum: share_accounting.get_last_batch_work_sum(),
                    share_batch_size: share_accounting.get_share_batch_size(),
                });
            }
        })
        .unwrap();

    ClientInfo {
        client_id: client.downstream_id,
        extended_channels,
        standard_channels,
    }
}

impl ClientsMonitoring for ChannelManager {
    fn get_clients(&self) -> Vec<ClientInfo> {
        let mut clients = Vec::new();

        self.channel_manager_data
            .safe_lock(|d| {
                for (_client_id, client) in d.downstream.iter() {
                    clients.push(downstream_to_client_info(client));
                }
            })
            .unwrap();

        clients
    }

    fn get_client_by_id(&self, client_id: usize) -> Option<ClientInfo> {
        self.channel_manager_data
            .safe_lock(|d| d.downstream.get(&client_id).map(downstream_to_client_info))
            .unwrap()
    }
}
