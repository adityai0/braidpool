//! # Braidpool Template Provider
//!
//! A Rust library that converts node's BlockTemplates to SV2 TemplateDistribution messages.
//!
//! ## Design
//!
//! This follows the same pattern as `ipc_template_consumer` in node crate:
//! - Receives `Arc<BlockTemplate>` from a channel (populated by `ipc_block_listener`)
//! - Converts to SV2 `NewTemplate` and `SetNewPrevHash` messages
//! - Handles `SubmitSolution` by delegating back to node's IPC client

use async_channel::{Receiver, Sender};
use node::ipc::{BlockTemplate, SharedBitcoinClient};
use node::{MAX_CACHED_TEMPLATES, TemplateId, get_next_template_id};
use std::{collections::HashMap, sync::Arc};
use stratum_core::{
    binary_sv2::U256,
    parsers_sv2::TemplateDistribution,
    template_distribution_sv2::{
        RequestTransactionData, RequestTransactionDataError, SubmitSolution,
    },
};
pub use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

pub mod error;
pub mod sv2_messages;

pub use error::BraidpoolTemplateProviderError;

/// Consumes block templates from ipc_block_listener and converts to SV2 TemplateDistribution messages.
///
/// This follows the same pattern as `ipc_template_consumer` in node crate:
/// - Receives `Arc<BlockTemplate>` from channel (populated by `ipc_block_listener`)
/// - Converts to SV2 `NewTemplate` and `SetNewPrevHash` messages
/// - Handles incoming `SubmitSolution` by delegating to shared_client
///
/// # Parameters
///
/// * `template_rx` - Receiver for BlockTemplates from ipc_block_listener
/// * `sv2_outgoing_tx` - Sender for outgoing SV2 TemplateDistribution messages
/// * `sv2_incoming_rx` - Receiver for incoming SV2 messages (SubmitSolution, RequestTransactionData)
/// * `template_cache` - Shared template cache (same one used by ipc_block_listener)
/// * `shared_client` - Reference to SharedBitcoinClient for submitting solutions
/// * `cancellation_token` - Token for graceful shutdown
pub async fn sv2_template_consumer(
    mut template_rx: tokio::sync::mpsc::Receiver<Arc<BlockTemplate>>,
    sv2_outgoing_tx: Sender<TemplateDistribution<'static>>,
    sv2_incoming_rx: Receiver<TemplateDistribution<'static>>,
    template_cache: Arc<tokio::sync::Mutex<HashMap<TemplateId, Arc<BlockTemplate>>>>,
    shared_client: &SharedBitcoinClient,
    cancellation_token: CancellationToken,
) -> Result<(), BraidpoolTemplateProviderError> {
    info!("SV2 template consumer started");

    let mut current_prev_hash: Option<U256<'static>> = None;

    // Wait for first CoinbaseOutputConstraints before processing templates
    info!("Waiting for CoinbaseOutputConstraints from pool");
    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                warn!("Cancelled before receiving CoinbaseOutputConstraints");
                return Ok(());
            }
            Ok(message) = sv2_incoming_rx.recv() => {
                if let TemplateDistribution::CoinbaseOutputConstraints(constraints) = message {
                    info!(
                        max_size = constraints.coinbase_output_max_additional_size,
                        max_sigops = constraints.coinbase_output_max_additional_sigops,
                        "Received CoinbaseOutputConstraints"
                    );
                    break;
                }
            }
        }
    }

    // Main consumer loop
    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                info!("SV2 template consumer shutting down");
                break;
            }

            // Receive new template from ipc_block_listener
            Some(ipc_template) = template_rx.recv() => {
                // Validate template has processed_block_hex
                if ipc_template.processed_block_hex.as_ref().map(|h| h.is_empty()).unwrap_or(true) {
                    warn!("Skipping template with missing/empty processed_block_hex");
                    continue;
                }

                // Generate template_id (uses node's global counter)
                let template_id = get_next_template_id();

                // Cache the template
                {
                    let mut cache = template_cache.lock().await;
                    cache.insert(template_id, ipc_template.clone());

                    // Cleanup old templates if cache is full
                    if cache.len() > MAX_CACHED_TEMPLATES {
                        let mut ids: Vec<TemplateId> = cache.keys().copied().collect();
                        ids.sort_unstable();
                        let remove_count = cache.len() - MAX_CACHED_TEMPLATES;
                        for id in ids.iter().take(remove_count) {
                            cache.remove(id);
                            debug!(template_id = %id, "Removed old template from cache");
                        }
                    }
                }

                // Get prev_hash from template
                let new_prev_hash = match sv2_messages::get_prev_hash_from_block_template(&ipc_template) {
                    Ok(h) => h,
                    Err(e) => {
                        error!(error = %e, "Failed to get prev_hash from template");
                        continue;
                    }
                };

                // Check if chain tip changed
                let is_new_block = current_prev_hash
                    .as_ref()
                    .map(|h| *h != new_prev_hash)
                    .unwrap_or(true);

                // Build NewTemplate message
                let new_template_msg = match sv2_messages::build_new_template_from_block_template(
                    template_id,
                    is_new_block, // future_template = true if new block
                    &ipc_template,
                ) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!(error = %e, "Failed to build NewTemplate message");
                        continue;
                    }
                };

                // Send NewTemplate
                if let Err(e) = sv2_outgoing_tx
                    .send(TemplateDistribution::NewTemplate(new_template_msg))
                    .await
                {
                    error!(error = %e, "Failed to send NewTemplate");
                    continue;
                }
                debug!(template_id = %template_id, future = is_new_block, "Sent NewTemplate");

                // Build and send SetNewPrevHash
                let set_prev_hash_msg = match sv2_messages::build_set_new_prev_hash_from_block_template(
                    template_id,
                    &ipc_template,
                ) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!(error = %e, "Failed to build SetNewPrevHash message");
                        continue;
                    }
                };

                if let Err(e) = sv2_outgoing_tx
                    .send(TemplateDistribution::SetNewPrevHash(set_prev_hash_msg))
                    .await
                {
                    error!(error = %e, "Failed to send SetNewPrevHash");
                    continue;
                }
                debug!(template_id = %template_id, "Sent SetNewPrevHash");

                // Update current prev_hash
                current_prev_hash = Some(new_prev_hash);

                info!(template_id = %template_id, "New SV2 template distributed");
            }

            // Handle incoming SV2 messages
            Ok(message) = sv2_incoming_rx.recv() => {
                match message {
                    TemplateDistribution::CoinbaseOutputConstraints(constraints) => {
                        info!(
                            max_size = constraints.coinbase_output_max_additional_size,
                            "Updated CoinbaseOutputConstraints"
                        );
                    }

                    TemplateDistribution::RequestTransactionData(request) => {
                        debug!(template_id = %request.template_id, "RequestTransactionData");
                        handle_request_transaction_data(
                            request,
                            &template_cache,
                            &sv2_outgoing_tx,
                        ).await;
                    }

                    TemplateDistribution::SubmitSolution(solution) => {
                        info!(template_id = %solution.template_id, "SubmitSolution received");
                        handle_submit_solution(
                            solution,
                            &template_cache,
                            shared_client,
                        ).await;
                    }

                    _ => {
                        warn!("Unexpected SV2 message type");
                    }
                }
            }
        }
    }

    Ok(())
}

/// Handle RequestTransactionData message
async fn handle_request_transaction_data(
    request: RequestTransactionData,
    template_cache: &Arc<tokio::sync::Mutex<HashMap<TemplateId, Arc<BlockTemplate>>>>,
    sv2_outgoing_tx: &Sender<TemplateDistribution<'static>>,
) {
    let cache = template_cache.lock().await;

    if let Some(template) = cache.get(&request.template_id) {
        if let Some(block_hex) = &template.processed_block_hex {
            match sv2_messages::build_request_transaction_data_success(
                request.template_id,
                block_hex,
            ) {
                Ok(success_msg) => {
                    let _ = sv2_outgoing_tx
                        .send(TemplateDistribution::RequestTransactionDataSuccess(
                            success_msg,
                        ))
                        .await;
                    return;
                }
                Err(e) => error!(error = %e, "Failed to build RequestTransactionDataSuccess"),
            }
        }
    }

    // Send error response
    let error_msg = RequestTransactionDataError {
        template_id: request.template_id,
        error_code: vec![].try_into().expect("empty vec is valid"),
    };
    let _ = sv2_outgoing_tx
        .send(TemplateDistribution::RequestTransactionDataError(
            error_msg.into_static(),
        ))
        .await;
}

/// Handle SubmitSolution message
async fn handle_submit_solution(
    solution: SubmitSolution<'static>,
    template_cache: &Arc<tokio::sync::Mutex<HashMap<TemplateId, Arc<BlockTemplate>>>>,
    shared_client: &SharedBitcoinClient,
) {
    let cache = template_cache.lock().await;

    if let Some(template) = cache.get(&solution.template_id) {
        match sv2_messages::submit_mining_solution(
            solution.template_id,
            template.clone(),
            solution.clone(),
            shared_client,
        )
        .await
        {
            Ok(()) => info!(template_id = %solution.template_id, "Solution submitted successfully"),
            Err(e) => {
                error!(template_id = %solution.template_id, error = %e, "Solution submission failed")
            }
        }
    } else {
        error!(
            template_id = %solution.template_id,
            cache_size = cache.len(),
            "Template not found for SubmitSolution"
        );
    }
}
