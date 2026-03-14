use std::{sync::Arc, thread::JoinHandle};

use async_channel::unbounded;
use bitcoin_core_sv2::CancellationToken;
use stratum_apps::{
    stratum_core::bitcoin::consensus::Encodable, task_manager::TaskManager,
    tp_type::TemplateProviderType, utils::types::GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS,
};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::{
    channel_manager::ChannelManager,
    config::PoolConfig,
    error::PoolErrorKind,
    status::State,
    template_receiver::{
        bitcoin_core::{connect_to_bitcoin_core, BitcoinCoreSv2Config},
        sv2_tp::Sv2Tp,
    },
};

pub mod braidpool;
pub mod channel_manager;
pub mod config;
pub mod downstream;
pub mod error;
mod io_task;
mod monitoring;
pub mod status;
pub mod template_receiver;
pub mod utils;

#[derive(Debug, Clone)]
pub struct PoolSv2 {
    config: PoolConfig,
    cancellation_token: CancellationToken,
}

#[cfg_attr(not(test), hotpath::measure_all)]
impl PoolSv2 {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Starts the Pool main loop.
    pub async fn start(&self) -> Result<(), PoolErrorKind> {
        let coinbase_outputs = vec![self.config.get_txout()];
        let mut encoded_outputs = vec![];

        coinbase_outputs
            .consensus_encode(&mut encoded_outputs)
            .expect("Invalid coinbase output in config");

        let cancellation_token = self.cancellation_token.clone();

        let task_manager = Arc::new(TaskManager::new());

        let (status_sender, status_receiver) = unbounded();

        let (channel_manager_to_downstream_sender, _channel_manager_to_downstream_receiver) =
            broadcast::channel(10);
        let (downstream_to_channel_manager_sender, downstream_to_channel_manager_receiver) =
            unbounded();

        let (channel_manager_to_tp_sender, channel_manager_to_tp_receiver) = unbounded();
        let (tp_to_channel_manager_sender, tp_to_channel_manager_receiver) = unbounded();

        debug!("Channels initialized.");

        let channel_manager = ChannelManager::new(
            self.config.clone(),
            channel_manager_to_tp_sender.clone(),
            tp_to_channel_manager_receiver,
            channel_manager_to_downstream_sender.clone(),
            downstream_to_channel_manager_receiver,
            encoded_outputs.clone(),
            self.config.network().to_owned(),
        )
        .await?;
        println!("Configuration received - {:?}", self.config);
        // Start monitoring server if configured
        if let Some(monitoring_addr) = self.config.monitoring_address() {
            info!(
                "Initializing monitoring server on http://{}",
                monitoring_addr
            );

            let monitoring_server = stratum_apps::monitoring::MonitoringServer::new(
                monitoring_addr,
                None, // Pool doesn't have channels opened with servers
                Some(Arc::new(channel_manager.clone())), // channels opened with clients
                std::time::Duration::from_secs(self.config.monitoring_cache_refresh_secs()),
            )
            .expect("Failed to initialize monitoring server");

            let cancellation_token_clone = cancellation_token.clone();
            let shutdown_signal = async move {
                cancellation_token_clone.cancelled().await;
            };

            task_manager.spawn(async move {
                if let Err(e) = monitoring_server.run(shutdown_signal).await {
                    error!("Monitoring server error: {}", e);
                }
            });
        }
        debug!(
            "The public key and private key being used for authority key pair - {} - {}",
            self.config.authority_public_key().0.to_string(),
            self.config.authority_secret_key().0.display_secret()
        );
        let channel_manager_clone = channel_manager.clone();
        let channel_manager_for_cleanup = channel_manager.clone();
        let mut bitcoin_core_sv2_join_handle: Option<JoinHandle<()>> = None;

        // Initialize Braidpool P2P if configured
        let mut braidpool_handle: Option<tokio::task::JoinHandle<()>> = None;
        let mut _braidpool_cmd_tx: Option<tokio::sync::mpsc::Sender<braidpool::SwarmCommand>> =
            None;
        let mut _braidpool_db_tx: Option<tokio::sync::mpsc::Sender<braidpool::BraidpoolDBTypes>> =
            None;
        let mut _braidpool_braid: Option<std::sync::Arc<tokio::sync::RwLock<braidpool::Braid>>> =
            None;

        if let Some(bind_addr) = self.config.braidpool_bind_addr() {
            info!("Initializing Braidpool P2P on {}", bind_addr);

            let braidpool_config = braidpool::BraidpoolConfig {
                bind_addr: bind_addr.to_string(),
                max_peers: self.config.braidpool_max_peers(),
                keystore_path: self
                    .config
                    .braidpool_keystore_path()
                    .map(|p| p.to_path_buf()),
                addnodes: self.config.braidpool_addnodes().to_vec(),
                network_name: self.config.network().to_string(),
            };

            let braidpool_p2p =
                braidpool::BraidpoolP2P::new(braidpool_config, cancellation_token.clone());
            //Spawning the p2p handlers/event_loop for handling braidpool p2p events
            match braidpool_p2p.spawn().await {
                Ok((handle, cmd_tx, db_tx, braid)) => {
                    braidpool_handle = Some(handle);
                    _braidpool_cmd_tx = Some(cmd_tx);
                    _braidpool_db_tx = Some(db_tx);
                    _braidpool_braid = Some(braid);
                    info!("Braidpool P2P initialized successfully");
                }
                Err(e) => {
                    error!("Failed to initialize Braidpool P2P: {}", e);
                }
            }
        }

        match self.config.template_provider_type().clone() {
            TemplateProviderType::Sv2Tp {
                address,
                public_key,
            } => {
                let sv2_tp = Sv2Tp::new(
                    address.clone(),
                    public_key,
                    channel_manager_to_tp_receiver,
                    tp_to_channel_manager_sender,
                    cancellation_token.clone(),
                    task_manager.clone(),
                )
                .await?;

                sv2_tp
                    .start(
                        address,
                        cancellation_token.clone(),
                        status_sender.clone(),
                        task_manager.clone(),
                    )
                    .await?;

                info!("Sv2 Template Provider setup done");
            }
            TemplateProviderType::BitcoinCoreIpc {
                network,
                data_dir,
                fee_threshold,
                min_interval,
            } => {
                let unix_socket_path =
                    stratum_apps::tp_type::resolve_ipc_socket_path(&network, data_dir)
                        .ok_or_else(|| PoolErrorKind::Configuration(
                            "Could not determine Bitcoin data directory. Please set data_dir in config.".to_string()
                        ))?;

                info!(
                    "Using Bitcoin Core IPC socket at: {}",
                    unix_socket_path.display()
                );

                // incoming and outgoing TDP channels from the perspective of BitcoinCoreSv2
                let incoming_tdp_receiver = channel_manager_to_tp_receiver.clone();
                let outgoing_tdp_sender = tp_to_channel_manager_sender.clone();

                let bitcoin_core_config = BitcoinCoreSv2Config {
                    unix_socket_path,
                    fee_threshold,
                    min_interval,
                    incoming_tdp_receiver,
                    outgoing_tdp_sender,
                    cancellation_token: CancellationToken::new(),
                    network_type: self.config.network().to_owned(),
                };

                bitcoin_core_sv2_join_handle = Some(
                    connect_to_bitcoin_core(
                        bitcoin_core_config,
                        cancellation_token.clone(),
                        task_manager.clone(),
                        status_sender.clone(),
                    )
                    .await,
                );
            }
        }

        channel_manager
            .start(
                cancellation_token.clone(),
                status_sender.clone(),
                task_manager.clone(),
                coinbase_outputs,
            )
            .await?;

        channel_manager_clone
            .start_downstream_server(
                *self.config.authority_public_key(),
                *self.config.authority_secret_key(),
                self.config.cert_validity_sec(),
                *self.config.listen_address(),
                task_manager.clone(),
                cancellation_token.clone(),
                status_sender,
                downstream_to_channel_manager_sender,
                channel_manager_to_downstream_sender,
            )
            .await?;

        info!("Spawning status listener task...");
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Ctrl+C received — initiating graceful shutdown...");
                    cancellation_token.cancel();
                    break;
                }
                message = status_receiver.recv() => {
                    if let Ok(status) = message {
                        match status.state {
                            State::DownstreamShutdown{downstream_id,..} => {
                                warn!("Downstream {downstream_id:?} disconnected — cleaning up channel manager.");
                                // Remove downstream from channel manager to prevent memory leak
                                if let Err(e) = channel_manager_for_cleanup.remove_downstream(downstream_id) {
                                    error!("Failed to remove downstream {downstream_id:?}: {e:?}");
                                    cancellation_token.cancel();
                                    break;
                                }
                            }
                            State::TemplateReceiverShutdown(_) => {
                                warn!("Template Receiver shutdown requested — initiating full shutdown.");
                                cancellation_token.cancel();
                                break;
                            }
                            State::ChannelManagerShutdown(_) => {
                                warn!("Channel Manager shutdown requested — initiating full shutdown.");
                                cancellation_token.cancel();
                                break;
                            }
                        }
                    }
                }
            }
        }

        if let Some(bitcoin_core_sv2_join_handle) = bitcoin_core_sv2_join_handle {
            info!("Waiting for BitcoinCoreSv2 dedicated thread to shutdown...");
            match bitcoin_core_sv2_join_handle.join() {
                Ok(_) => info!("BitcoinCoreSv2 dedicated thread shutdown complete."),
                Err(e) => error!("BitcoinCoreSv2 dedicated thread error: {e:?}"),
            }
        }

        // Wait for Braidpool P2P to shutdown
        if let Some(handle) = braidpool_handle {
            info!("Waiting for Braidpool P2P to shutdown...");
            match handle.await {
                Ok(_) => info!("Braidpool P2P shutdown complete."),
                Err(e) => error!("Braidpool P2P task error: {e:?}"),
            }
        }

        warn!(
            "Graceful shutdown: waiting {} seconds for tasks to finish",
            GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS
        );

        match tokio::time::timeout(
            std::time::Duration::from_secs(GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS),
            task_manager.join_all(),
        )
        .await
        {
            Ok(_) => {
                info!("All tasks joined cleanly");
            }
            Err(_) => {
                warn!(
                    "Tasks did not finish within {} seconds, aborting",
                    GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS
                );
                task_manager.abort_all().await;
                info!("Joining aborted tasks...");
                task_manager.join_all().await;
                warn!("Forced shutdown complete");
            }
        }
        info!("Pool shutdown complete.");
        Ok(())
    }
}

impl Drop for PoolSv2 {
    fn drop(&mut self) {
        info!("PoolSv2 dropped");
        self.cancellation_token.cancel();
    }
}
