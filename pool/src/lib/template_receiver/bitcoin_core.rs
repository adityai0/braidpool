use crate::{
    error::PoolError,
    status::{handle_error, State, Status, StatusSender},
    utils::ShutdownMessage,
};
use async_channel::{Receiver, Sender};
use bitcoin_core_sv2::{BitcoinCoreSv2, CancellationToken};
use std::{path::PathBuf, sync::Arc, thread::JoinHandle};
use stratum_apps::{
    stratum_core::{
        bitcoin::{
            absolute::LockTime, transaction::Version, OutPoint, ScriptBuf, Sequence, Transaction,
            TxIn, TxOut, Witness,
        },
        parsers_sv2::TemplateDistribution,
        template_distribution_sv2::CoinbaseOutputConstraints,
    },
    task_manager::TaskManager,
};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct BitcoinCoreSv2Config {
    pub unix_socket_path: PathBuf,
    pub fee_threshold: u64,
    pub min_interval: u8,
    pub incoming_tdp_receiver: Receiver<TemplateDistribution<'static>>,
    pub outgoing_tdp_sender: Sender<TemplateDistribution<'static>>,
    pub cancellation_token: CancellationToken,
}

#[allow(clippy::too_many_arguments)]
pub async fn connect_to_bitcoin_core(
    bitcoin_core_config: BitcoinCoreSv2Config,
    incoming_tdp_sender: Sender<TemplateDistribution<'static>>,
    notify_shutdown: broadcast::Sender<ShutdownMessage>,
    task_manager: Arc<TaskManager>,
    status_sender: Sender<Status>,
    coinbase_outputs: Vec<TxOut>,
) -> JoinHandle<()> {
    let mut shutdown_rx = notify_shutdown.subscribe();
    let cancellation_token_clone = bitcoin_core_config.cancellation_token.clone();
    let status_sender_clone = status_sender.clone();

    // spawn a task to handle shutdown signals and cancellation token activations
    task_manager.spawn(async move {
        loop {
            tokio::select! {
                message = shutdown_rx.recv() => {
                    if let Ok(ShutdownMessage::ShutdownAll) = message {
                        cancellation_token_clone.cancel();
                        break;
                    }
                }
                _ = cancellation_token_clone.cancelled() => {
                    // turn status_sender into a StatusSender::TemplateReceiver
                    let status_sender = StatusSender::TemplateReceiver(status_sender_clone);

                    handle_error(
                        &status_sender,
                        PoolError::BitcoinCoreSv2CancellationTokenActivated,
                    )
                    .await;
                    break;
                }
            }
        }
    });

    let status_sender_clone = status_sender.clone();

    // spawn a dedicated thread to run the BitcoinCoreSv2 instance
    // because we're limited to tokio::task::LocalSet due to the use of `capnp` clients on
    // `bitcoin-core-sv2`, which are not `Send`
    let join_handle = std::thread::spawn(move || {
        // we need a dedicated runtime so we can spawn an async task inside the LocalSet
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("Failed to create Tokio runtime: {:?}", e);

                // we can't use handle_error here because we're not in a async context yet
                let _ = status_sender_clone.send_blocking(Status {
                    state: State::TemplateReceiverShutdown(
                        PoolError::FailedToCreateBitcoinCoreTokioRuntime,
                    ),
                });
                return;
            }
        };
        let tokio_local_set = tokio::task::LocalSet::new();

        tokio_local_set.block_on(&rt, async move {
            // create a new BitcoinCoreSv2 instance
            let mut sv2_bitcoin_core = match BitcoinCoreSv2::new(
                &bitcoin_core_config.unix_socket_path,
                bitcoin_core_config.fee_threshold,
                bitcoin_core_config.min_interval,
                bitcoin_core_config.incoming_tdp_receiver,
                bitcoin_core_config.outgoing_tdp_sender,
                bitcoin_core_config.cancellation_token.clone(),
            )
            .await
            {
                Ok(sv2_bitcoin_core) => sv2_bitcoin_core,
                Err(e) => {
                    tracing::error!("Failed to create BitcoinCoreToSv2: {:?}", e);
                    bitcoin_core_config.cancellation_token.cancel();
                    return;
                }
            };

            // run the BitcoinCoreSv2 instance, which will block until the cancellation token is
            // activated
            sv2_bitcoin_core.run().await;
        });
    });

    // calculate the max coinbase output size for CoinbaseOutputConstraints
    let max_size: u32 = coinbase_outputs.iter().map(|o| o.size() as u32).sum();
    tracing::debug!(
        max_size,
        outputs_count = coinbase_outputs.len(),
        "Calculated max coinbase output size"
    );

    // this is used to calculate the sigops of the coinbase outputs
    // for CoinbaseOutputConstraints
    let dummy_coinbase = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::from(vec![vec![0; 32]]),
        }],
        output: coinbase_outputs,
    };

    let max_sigops = dummy_coinbase.total_sigop_cost(|_| None) as u16;
    tracing::debug!(max_sigops, "Calculated max sigops for coinbase");

    let coinbase_output_constraints = CoinbaseOutputConstraints {
        coinbase_output_max_additional_size: max_size,
        coinbase_output_max_additional_sigops: max_sigops,
    };

    // turn status_sender into a StatusSender::TemplateReceiver
    let status_sender = StatusSender::TemplateReceiver(status_sender);

    // send the CoinbaseOutputConstraints message to the spawned BitcoinCoreSv2 instance
    match incoming_tdp_sender
        .send(TemplateDistribution::CoinbaseOutputConstraints(
            coinbase_output_constraints,
        ))
        .await
    {
        Ok(_) => {}
        Err(e) => {
            tracing::error!(
                "Failed to send CoinbaseOutputConstraints message to channel manager: {:?}",
                e
            );
            handle_error(
                &status_sender,
                PoolError::FailedToSendCoinbaseOutputConstraints,
            )
            .await;
        }
    };

    join_handle
}
