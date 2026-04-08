use crate::{
    error::{self, PoolError, PoolErrorKind},
    status::{handle_error, State, Status, StatusSender},
};
use async_channel::{Receiver, Sender};
use braidpool_template_provider::{sv2_template_consumer, CancellationToken};
use node::ipc::{ipc_block_listener, BlockTemplate};
use node::TemplateId;
use std::{collections::HashMap, path::PathBuf, sync::Arc, thread::JoinHandle};
use stratum_apps::{stratum_core::parsers_sv2::TemplateDistribution, task_manager::TaskManager};

#[derive(Clone)]
pub struct BitcoinCoreSv2Config {
    pub unix_socket_path: PathBuf,
    pub fee_threshold: u64,
    pub min_interval: u8,
    pub incoming_tdp_receiver: Receiver<TemplateDistribution<'static>>,
    pub outgoing_tdp_sender: Sender<TemplateDistribution<'static>>,
    pub cancellation_token: CancellationToken,
    pub network_type: String,
}

#[cfg_attr(not(test), hotpath::measure)]
pub async fn connect_to_bitcoin_core(
    bitcoin_core_config: BitcoinCoreSv2Config,
    cancellation_token: CancellationToken,
    task_manager: Arc<TaskManager>,
    status_sender: Sender<Status>,
) -> JoinHandle<()> {
    let bitcoin_core_canc_token = bitcoin_core_config.cancellation_token.clone();
    let status_sender_clone = status_sender.clone();

    // spawn a task to handle shutdown signals and cancellation token activations
    task_manager.spawn(async move {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                bitcoin_core_canc_token.cancel();
            }
            _ = bitcoin_core_canc_token.cancelled() => {
                // turn status_sender into a StatusSender::TemplateReceiver
                let status_sender = StatusSender::TemplateReceiver(status_sender_clone);

                handle_error::<error::TemplateProvider>(
                    &status_sender,
                    PoolError::shutdown(PoolErrorKind::BitcoinCoreSv2CancellationTokenActivated),
                )
                .await;
            }
        }
    });

    let status_sender_clone = status_sender.clone();

    // Spawn a dedicated thread because capnp clients are not Send
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("Failed to create Tokio runtime: {:?}", e);
                let _ = status_sender_clone.send_blocking(Status {
                    state: State::TemplateReceiverShutdown(
                        PoolErrorKind::FailedToCreateBitcoinCoreTokioRuntime,
                    ),
                });
                return;
            }
        };
        let tokio_local_set = tokio::task::LocalSet::new();

        tokio_local_set.block_on(&rt, async move {
            let ipc_socket_path = bitcoin_core_config
                .unix_socket_path
                .to_string_lossy()
                .to_string();
            let network_name = bitcoin_core_config.network_type.clone();
            let canc_token = bitcoin_core_config.cancellation_token.clone();

            // Create template channel (ipc_block_listener -> sv2_template_consumer)
            let (template_tx, template_rx) = tokio::sync::mpsc::channel::<Arc<BlockTemplate>>(10);

            // Create template cache (shared between ipc_block_listener and sv2_template_consumer)
            let template_cache: Arc<tokio::sync::Mutex<HashMap<TemplateId, Arc<BlockTemplate>>>> =
                Arc::new(tokio::sync::Mutex::new(HashMap::new()));

            // Create block submission channel
            // sv2_template_consumer sends BlockSubmissionRequest -> ipc_block_listener submits via IPC
            let (block_submission_tx, block_submission_rx) = tokio::sync::mpsc::unbounded_channel();

            // Spawn ipc_block_listener (fetches templates from Bitcoin Core)
            let listener_task = tokio::task::spawn_local({
                let socket = ipc_socket_path.clone();
                let tx = template_tx;
                let cache = template_cache.clone();
                let network = network_name.clone();
                async move {
                    match ipc_block_listener(socket, tx, cache, block_submission_rx, network).await
                    {
                        Ok(_) => tracing::info!("IPC block listener exited"),
                        Err(e) => tracing::error!("IPC block listener error: {:?}", e),
                    }
                }
            });

            // Spawn sv2_template_consumer (converts templates to SV2 messages)
            let consumer_task = tokio::task::spawn_local({
                let cache = template_cache.clone();
                let outgoing_tx = bitcoin_core_config.outgoing_tdp_sender;
                let incoming_rx = bitcoin_core_config.incoming_tdp_receiver;
                let token = canc_token.clone();
                async move {
                    if let Err(e) = sv2_template_consumer(
                        template_rx,
                        outgoing_tx,
                        incoming_rx,
                        cache,
                        block_submission_tx,
                        token,
                    )
                    .await
                    {
                        tracing::error!("SV2 template consumer error: {:?}", e);
                    }
                }
            });

            // Wait for either task to complete or cancellation
            tokio::select! {
                _ = listener_task => tracing::info!("IPC listener completed"),
                _ = consumer_task => tracing::info!("SV2 consumer completed"),
                _ = canc_token.cancelled() => tracing::info!("Template provider cancelled"),
            }
        });
    })
}
