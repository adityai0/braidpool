//! ## Translator Sv2
//!
//! Provides the core logic and main struct (`TranslatorSv2`) for running a
//! Stratum V1 to Stratum V2 translation proxy.
//!
//! This module orchestrates the interaction between downstream SV1 miners and upstream SV2
//! applications (proxies or pool servers).
//!
//! The central component is the `TranslatorSv2` struct, which encapsulates the state and
//! provides the `start` method as the main entry point for running the translator service.
//! It relies on several sub-modules (`config`, `downstream_sv1`, `upstream_sv2`, `proxy`, `status`,
//! etc.) for specialized functionalities.
#![allow(clippy::module_inception)]
use async_channel::{unbounded, Receiver, Sender};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use stratum_apps::{
    key_utils::Secp256k1PublicKey, stratum_core::parsers_sv2::Mining, task_manager::TaskManager,
};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

pub use stratum_apps::stratum_core::sv1_api::server_to_client;

use config::TranslatorConfig;

use crate::{
    error::TproxyError,
    status::{State, Status},
    sv1::sv1_server::sv1_server::Sv1Server,
    sv2::{channel_manager::ChannelMode, ChannelManager, Upstream},
    utils::ShutdownMessage,
};

pub mod config;
pub mod error;
mod io_task;
pub mod status;
pub mod sv1;
pub mod sv2;
pub mod utils;

/// The main struct that manages the SV1/SV2 translator.
#[derive(Clone, Debug)]
pub struct TranslatorSv2 {
    config: TranslatorConfig,
}

impl TranslatorSv2 {
    /// Creates a new `TranslatorSv2`.
    ///
    /// Initializes the translator with the given configuration and sets up
    /// the reconnect wait time.
    pub fn new(config: TranslatorConfig) -> Self {
        Self { config }
    }

    /// Starts the translator.
    ///
    /// This method starts the main event loop, which handles connections,
    /// protocol translation, job management, and status reporting.
    pub async fn start(self) {
        info!("Starting Translator Proxy...");

        let (notify_shutdown, _) = tokio::sync::broadcast::channel::<ShutdownMessage>(1);
        let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel::<()>(1);
        let task_manager = Arc::new(TaskManager::new());

        let (status_sender, status_receiver) = async_channel::unbounded::<Status>();

        let (channel_manager_to_upstream_sender, channel_manager_to_upstream_receiver) =
            unbounded();
        let (upstream_to_channel_manager_sender, upstream_to_channel_manager_receiver) =
            unbounded();
        let (channel_manager_to_sv1_server_sender, channel_manager_to_sv1_server_receiver) =
            unbounded();
        let (sv1_server_to_channel_manager_sender, sv1_server_to_channel_manager_receiver) =
            unbounded();

        debug!("Channels initialized.");

        let mut upstream_addresses = self
            .config
            .upstreams
            .iter()
            .map(|upstream| {
                let upstream_addr =
                    SocketAddr::new(upstream.address.parse().unwrap(), upstream.port);
                (upstream_addr, upstream.authority_pubkey, false)
            })
            .collect::<Vec<_>>();

        if let Err(e) = self
            .initialize_upstream(
                &mut upstream_addresses,
                channel_manager_to_upstream_receiver.clone(),
                upstream_to_channel_manager_sender.clone(),
                notify_shutdown.clone(),
                status_sender.clone(),
                shutdown_complete_tx.clone(),
                task_manager.clone(),
            )
            .await
        {
            error!("Failed to initialize any upstream connection: {e:?}");
            return;
        }

        let channel_manager = Arc::new(ChannelManager::new(
            channel_manager_to_upstream_sender,
            upstream_to_channel_manager_receiver,
            channel_manager_to_sv1_server_sender.clone(),
            sv1_server_to_channel_manager_receiver,
            status_sender.clone(),
            if self.config.aggregate_channels {
                ChannelMode::Aggregated
            } else {
                ChannelMode::NonAggregated
            },
        ));

        let downstream_addr = SocketAddr::new(
            self.config.downstream_address.parse().unwrap(),
            self.config.downstream_port,
        );

        let sv1_server = Arc::new(Sv1Server::new(
            downstream_addr,
            channel_manager_to_sv1_server_receiver,
            sv1_server_to_channel_manager_sender,
            self.config.clone(),
        ));

        ChannelManager::run_channel_manager_tasks(
            channel_manager.clone(),
            notify_shutdown.clone(),
            shutdown_complete_tx.clone(),
            status_sender.clone(),
            task_manager.clone(),
        )
        .await;

        if let Err(e) = Sv1Server::start(
            sv1_server,
            notify_shutdown.clone(),
            shutdown_complete_tx.clone(),
            status_sender.clone(),
            task_manager.clone(),
        )
        .await
        {
            error!("SV1 server startup failed: {e:?}");
            return;
        }

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Ctrl+C received — initiating graceful shutdown...");
                    let _ = notify_shutdown.send(ShutdownMessage::ShutdownAll);
                    break;
                }
                message = status_receiver.recv() => {
                    if let Ok(status) = message {
                        match status.state {
                            State::DownstreamShutdown{downstream_id,..} => {
                                warn!("Downstream {downstream_id:?} disconnected — notifying SV1 server.");
                                let _ = notify_shutdown.send(ShutdownMessage::DownstreamShutdown(downstream_id));
                            }
                            State::Sv1ServerShutdown(_) => {
                                warn!("SV1 Server shutdown requested — initiating full shutdown.");
                                let _ = notify_shutdown.send(ShutdownMessage::ShutdownAll);
                                break;
                            }
                            State::ChannelManagerShutdown(_) => {
                                warn!("Channel Manager shutdown requested — initiating full shutdown.");
                                let _ = notify_shutdown.send(ShutdownMessage::ShutdownAll);
                                break;
                            }
                            State::UpstreamShutdown(msg) => {
                                warn!("Upstream connection dropped: {msg:?} — attempting reconnection...");
                                _ = notify_shutdown.send(ShutdownMessage::DownstreamShutdownAll);

                                if let Err(e) = self.initialize_upstream(
                                    &mut upstream_addresses,
                                    channel_manager_to_upstream_receiver.clone(),
                                    upstream_to_channel_manager_sender.clone(),
                                    notify_shutdown.clone(),
                                    status_sender.clone(),
                                    shutdown_complete_tx.clone(),
                                    task_manager.clone()
                                ).await {
                                    error!("Couldn't perform fallback, shutting system down: {e:?}");
                                    let _ = notify_shutdown.send(ShutdownMessage::ShutdownAll);
                                    break;
                                } else {
                                    info!("Upstream restarted successfully.");
                                    // Reset channel manager state and shutdown downstreams in one message
                                    let _ = notify_shutdown.send(ShutdownMessage::UpstreamFallback);
                                }
                            }
                        }
                    }
                }
            }
        }

        drop(shutdown_complete_tx);
        info!("Waiting for shutdown completion signals from subsystems...");
        let shutdown_timeout = tokio::time::Duration::from_secs(5);
        tokio::select! {
            _ = shutdown_complete_rx.recv() => {
                info!("All subsystems reported shutdown complete.");
            }
            _ = tokio::time::sleep(shutdown_timeout) => {
                warn!("Graceful shutdown timed out after {shutdown_timeout:?} — forcing shutdown.");
                task_manager.abort_all().await;
            }
        }
        info!("Joining remaining tasks...");
        task_manager.join_all().await;
        info!("TranslatorSv2 shutdown complete.");
    }

    /// Initializes the upstream connection list, handling retries, fallbacks, and flagging.
    ///
    /// Upstreams are tried sequentially, each receiving a fixed number of retries before we
    /// advance to the next entry. This ensures we exhaust every healthy upstream before shutting
    /// the translator down.
    ///
    /// The boolean flag in the `(SocketAddr, Secp256k1PublicKey, bool)` tuple acts as the
    /// upstream's state machine: `false` means "never tried", while `true` means "already
    /// connected or marked as malicious". Once an upstream is flagged we skip it on future loops
    /// to avoid hammering known-bad endpoints during failover.
    #[allow(clippy::too_many_arguments)]
    pub async fn initialize_upstream(
        &self,
        upstreams: &mut [(SocketAddr, Secp256k1PublicKey, bool)],
        channel_manager_to_upstream_receiver: Receiver<Mining<'static>>,
        upstream_to_channel_manager_sender: Sender<Mining<'static>>,
        notify_shutdown: broadcast::Sender<ShutdownMessage>,
        status_sender: Sender<Status>,
        shutdown_complete_tx: mpsc::Sender<()>,
        task_manager: Arc<TaskManager>,
    ) -> Result<(), TproxyError> {
        const MAX_RETRIES: usize = 3;
        let upstream_len = upstreams.len();
        for (i, upstream_addr) in upstreams.iter_mut().enumerate() {
            info!(
                "Trying upstream {} of {}: {:?}",
                i + 1,
                upstream_len,
                upstream_addr
            );

            // Skip upstreams already marked as malicious. We’ve previously failed or
            // blacklisted them, so no need to warn or attempt reconnecting again.
            if upstream_addr.2 {
                debug!(
                    "Upstream previously marked as malicious, skipping initial attempt warnings."
                );
                continue;
            }

            for attempt in 1..=MAX_RETRIES {
                info!("Connection attempt {}/{}...", attempt, MAX_RETRIES);
                tokio::time::sleep(Duration::from_secs(1)).await;

                match try_initialize_upstream(
                    upstream_addr,
                    upstream_to_channel_manager_sender.clone(),
                    channel_manager_to_upstream_receiver.clone(),
                    notify_shutdown.clone(),
                    status_sender.clone(),
                    shutdown_complete_tx.clone(),
                    task_manager.clone(),
                )
                .await
                {
                    Ok(pair) => {
                        upstream_addr.2 = true;
                        return Ok(pair);
                    }
                    Err(e) => {
                        warn!(
                            "Attempt {}/{} failed for {:?}: {:?}",
                            attempt, MAX_RETRIES, upstream_addr, e
                        );
                        if attempt == MAX_RETRIES {
                            warn!(
                                "Max retries reached for {:?}, moving to next upstream",
                                upstream_addr
                            );
                        }
                    }
                }
            }
            upstream_addr.2 = true;
        }

        tracing::error!("All upstreams failed after {} retries each", MAX_RETRIES);
        Err(TproxyError::Shutdown)
    }
}

// Attempts to initialize a single upstream.
async fn try_initialize_upstream(
    upstream_addr: &(SocketAddr, Secp256k1PublicKey, bool),
    upstream_to_channel_manager_sender: Sender<Mining<'static>>,
    channel_manager_to_upstream_receiver: Receiver<Mining<'static>>,
    notify_shutdown: broadcast::Sender<ShutdownMessage>,
    status_sender: Sender<Status>,
    shutdown_complete_tx: mpsc::Sender<()>,
    task_manager: Arc<TaskManager>,
) -> Result<(), TproxyError> {
    let upstream = Upstream::new(
        upstream_addr,
        upstream_to_channel_manager_sender,
        channel_manager_to_upstream_receiver,
        notify_shutdown.clone(),
        shutdown_complete_tx.clone(),
        task_manager.clone(),
    )
    .await?;

    upstream
        .start(
            notify_shutdown,
            shutdown_complete_tx,
            status_sender,
            task_manager,
        )
        .await?;
    Ok(())
}
