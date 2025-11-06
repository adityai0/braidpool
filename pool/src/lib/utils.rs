use std::{net::SocketAddr, sync::Arc};

use async_channel::{Receiver, Sender};
use stratum_apps::{
    network_helpers::noise_stream::{NoiseTcpReadHalf, NoiseTcpWriteHalf},
    stratum_core::{
        common_messages_sv2::{Protocol, SetupConnection},
        framing_sv2::framing::Frame,
    },
    task_manager::TaskManager,
    utils::types::{DownstreamId, Message, SV2Frame},
};
use tokio::sync::broadcast;
use tracing::{error, trace, warn, Instrument};

use crate::{
    error::PoolResult,
    status::{StatusSender, StatusType},
};

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
) -> PoolResult<SetupConnection<'static>> {
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
pub fn get_setup_connection_message_tp(address: SocketAddr) -> SetupConnection<'static> {
    let endpoint_host = address.ip().to_string().into_bytes().try_into().unwrap();
    let vendor = String::new().try_into().unwrap();
    let hardware_version = String::new().try_into().unwrap();
    let firmware = String::new().try_into().unwrap();
    let device_id = String::new().try_into().unwrap();
    SetupConnection {
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
    }
}

/// Spawns async reader and writer tasks for handling framed I/O with shutdown support.
#[track_caller]
#[allow(clippy::too_many_arguments)]
pub fn spawn_io_tasks(
    task_manager: Arc<TaskManager>,
    mut reader: NoiseTcpReadHalf<Message>,
    mut writer: NoiseTcpWriteHalf<Message>,
    outbound_rx: Receiver<SV2Frame>,
    inbound_tx: Sender<SV2Frame>,
    notify_shutdown: broadcast::Sender<ShutdownMessage>,
    status_sender: StatusSender,
) {
    let caller = std::panic::Location::caller();
    let inbound_tx_clone = inbound_tx.clone();
    let outbound_rx_clone = outbound_rx.clone();
    {
        let mut shutdown_rx = notify_shutdown.subscribe();
        let status_sender = status_sender.clone();
        let status_type: StatusType = StatusType::from(&status_sender);

        task_manager.spawn(async move {
            trace!("Reader task started");
            loop {
                tokio::select! {
                    message = shutdown_rx.recv() => {
                        match message {
                            Ok(ShutdownMessage::ShutdownAll) => {
                                trace!("Received global shutdown");
                                inbound_tx.close();
                                break;
                            }
                            Ok(ShutdownMessage::DownstreamShutdown(down_id))  if matches!(status_type, StatusType::Downstream(id) if id == down_id) => {
                                trace!(down_id, "Received downstream shutdown");
                                if status_type != StatusType::TemplateReceiver {
                                    inbound_tx.close();
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    res = reader.read_frame() => {
                        match res {
                            Ok(frame) => {
                                match frame {
                                    Frame::HandShake(frame) => {
                                        error!(?frame, "Received handshake frame");
                                        drop(frame);
                                        break;
                                    },
                                    Frame::Sv2(sv2_frame) => {
                                        trace!("Received inbound frame");
                                        if let Err(e) = inbound_tx.send(sv2_frame).await {
                                            inbound_tx.close();
                                            error!(error=?e, "Failed to forward inbound frame");
                                            break;
                                        }
                                    },
                                }
                            }
                            Err(e) => {
                                error!(error=?e, "Reader error");
                                inbound_tx.close();
                                break;
                            }
                        }
                    }
                }
            }
            inbound_tx.close();
            outbound_rx_clone.close();
            drop(inbound_tx);
            drop(outbound_rx_clone);
            warn!("Reader task exited.");
        }.instrument(tracing::trace_span!(
            "reader_task",
            spawned_at = %format!("{}:{}", caller.file(), caller.line())
        )));
    }

    {
        let mut shutdown_rx = notify_shutdown.subscribe();
        let status_type: StatusType = StatusType::from(&status_sender);

        task_manager.spawn(async move {
            trace!("Writer task started");
            loop {
                tokio::select! {
                    message = shutdown_rx.recv() => {
                        match message {
                            Ok(ShutdownMessage::ShutdownAll) => {
                                trace!("Received global shutdown");
                                outbound_rx.close();
                                break;
                            }
                            Ok(ShutdownMessage::DownstreamShutdown(down_id))  if matches!(status_type, StatusType::Downstream(id) if id == down_id) => {
                                trace!(down_id, "Received downstream shutdown");
                                if status_type != StatusType::TemplateReceiver {
                                    outbound_rx.close();
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    res = outbound_rx.recv() => {
                        match res {
                            Ok(frame) => {
                                trace!("Sending outbound frame");
                                if let Err(e) = writer.write_frame(frame.into()).await {
                                    error!(error=?e, "Writer error");
                                    outbound_rx.close();
                                    break;
                                }
                            }
                            Err(_) => {
                                outbound_rx.close();
                                warn!("Outbound channel closed");
                                break;
                            }
                        }
                    }
                }
            }
            outbound_rx.close();
            inbound_tx_clone.close();
            drop(outbound_rx);
            drop(inbound_tx_clone);
            warn!("Writer task exited.");
        }.instrument(tracing::trace_span!(
            "writer_task",
            spawned_at = %format!("{}:{}", caller.file(), caller.line())
        )));
    }
}
