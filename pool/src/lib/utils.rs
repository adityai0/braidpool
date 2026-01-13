use std::net::SocketAddr;
use stratum_apps::{
    stratum_core::{
        binary_sv2::Str0255,
        common_messages_sv2::{Protocol, SetupConnection},
        mining_sv2::CloseChannel,
    },
    utils::types::{ChannelId, DownstreamId},
};

use crate::error::PoolErrorKind;

/// Represents a message that can trigger shutdown of various system components.
#[derive(Debug, Clone)]
pub enum ShutdownMessage {
    /// Shutdown all components immediately
    ShutdownAll,
    /// Shutdown all downstream connections
    DownstreamShutdownAll,
    /// Shutdown a specific downstream connection by ID
    DownstreamShutdown(DownstreamId),
}

/// Constructs a `SetupConnection` message for the mining protocol.
#[allow(clippy::result_large_err)]
pub fn get_setup_connection_message(
    min_version: u16,
    max_version: u16,
) -> Result<SetupConnection<'static>, PoolErrorKind> {
    let endpoint_host = "0.0.0.0".to_string().into_bytes().try_into()?;
    let vendor = String::new().try_into()?;
    let hardware_version = String::new().try_into()?;
    let firmware = String::new().try_into()?;
    let device_id = String::new().try_into()?;
    let flags = 0b0000_0000_0000_0000_0000_0000_0000_0110;
    Ok(SetupConnection {
        protocol: Protocol::MiningProtocol,
        min_version,
        max_version,
        flags,
        endpoint_host,
        endpoint_port: 50,
        vendor,
        hardware_version,
        firmware,
        device_id,
    })
}

/// Constructs a `SetupConnection` message for the Template Provider (TP).
#[allow(clippy::result_large_err)]
pub fn get_setup_connection_message_tp(
    address: SocketAddr,
) -> Result<SetupConnection<'static>, PoolErrorKind> {
    let endpoint_host = address.ip().to_string().into_bytes().try_into()?;
    let vendor = String::new().try_into()?;
    let hardware_version = String::new().try_into()?;
    let firmware = String::new().try_into()?;
    let device_id = String::new().try_into()?;
    Ok(SetupConnection {
        protocol: Protocol::TemplateDistributionProtocol,
        min_version: 2,
        max_version: 2,
        flags: 0b0000_0000_0000_0000_0000_0000_0000_0000,
        endpoint_host,
        endpoint_port: address.port(),
        vendor,
        hardware_version,
        firmware,
        device_id,
    })
}

/// Creates a [`CloseChannel`] message for the given channel ID and reason.
///
/// The `msg` is converted into a [`Str0255`] reason code.  
/// If conversion fails, this function will panic.
pub(crate) fn create_close_channel_msg(channel_id: ChannelId, msg: &str) -> CloseChannel<'_> {
    CloseChannel {
        channel_id,
        reason_code: Str0255::try_from(msg.to_string()).expect("Could not convert message."),
    }
}
