//! SV2 message builders for template distribution protocol
//!
//! This module provides functions to build `NewTemplate` and `SetNewPrevHash`
//! messages directly from node's `BlockTemplate`.

use crate::error::TemplateDataError;
use bitcoin::{
    Amount, Target, Transaction,
    block::Header,
    consensus::{deserialize, serialize},
    hashes::{Hash, HashEngine, sha256d},
};
use node::ipc::BlockTemplate;
use node::stratum::BlockSubmissionRequest;
use stratum_core::{
    binary_sv2::{B016M, B064K, B0255, Seq064K, Seq0255, U256},
    template_distribution_sv2::{
        NewTemplate, RequestTransactionDataSuccess, SetNewPrevHash, SubmitSolution,
    },
};
use tracing::{debug, error};

/// Build a NewTemplate message from block template data
///
/// # Arguments
/// * `template_id` - Unique identifier for this template
/// * `future_template` - Whether this is a future template (true) or current (false)
/// * `header` - Block header containing version, prev_hash, etc.
/// * `coinbase_tx` - The coinbase transaction
/// * `merkle_path` - Merkle path from coinbase to merkle root
/// * `txs` - Transaction data (can be empty for initial template)
pub fn build_new_template<'decoder>(
    template_id: u64,
    future_template: bool,
    header: &Header,
    coinbase_tx: &Transaction,
    merkle_path: &[Vec<u8>],
    txs: Seq064K<'decoder, B016M<'decoder>>,
) -> Result<NewTemplate<'static>, TemplateDataError> {
    let version = header
        .version
        .to_consensus()
        .try_into()
        .map_err(|_| TemplateDataError::InvalidBlockVersion)?;

    let coinbase_tx_version = coinbase_tx
        .version
        .0
        .try_into()
        .map_err(|_| TemplateDataError::InvalidCoinbaseTxVersion)?;

    // Get coinbase script sig (the entire scriptSig becomes coinbase_prefix in SV2)
    let coinbase_prefix: B0255 = coinbase_tx.input[0]
        .script_sig
        .to_bytes()
        .try_into()
        .map_err(|_| TemplateDataError::InvalidCoinbaseScriptSig)?;

    let coinbase_tx_input_sequence = coinbase_tx.input[0].sequence.to_consensus_u32();

    // Calculate total value of all coinbase outputs
    let coinbase_tx_value_remaining = coinbase_tx
        .output
        .iter()
        .map(|output| output.value.to_sat())
        .sum::<u64>();

    // Get empty (zero-value) coinbase outputs and serialize them
    let empty_outputs: Vec<_> = coinbase_tx
        .output
        .iter()
        .filter(|output| output.value == Amount::from_sat(0))
        .cloned()
        .collect();

    let mut serialized_outputs = Vec::new();
    for output in &empty_outputs {
        serialized_outputs.extend_from_slice(&serialize(output));
    }

    let coinbase_tx_outputs: B064K = serialized_outputs
        .try_into()
        .map_err(|_| TemplateDataError::CoinbaseOutputSerializationFailed)?;

    let coinbase_tx_locktime = coinbase_tx.lock_time.to_consensus_u32();

    // Convert merkle path to Seq0255<U256>
    let merkle_path_u256: Vec<U256<'_>> = merkle_path
        .iter()
        .map(|hash_bytes| {
            U256::try_from(hash_bytes.clone())
                .map_err(|_| TemplateDataError::MerklePathError("Failed to convert hash".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let merkle_path_seq = Seq0255::new(merkle_path_u256)
        .map_err(|_| TemplateDataError::MerklePathError("Failed to create sequence".into()))?;

    let new_template = NewTemplate {
        template_id,
        future_template,
        version,
        coinbase_tx_version,
        coinbase_prefix,
        coinbase_tx_input_sequence,
        coinbase_tx_value_remaining,
        coinbase_tx_outputs_count: empty_outputs.len() as u32,
        coinbase_tx_outputs,
        coinbase_tx_locktime,
        merkle_path: merkle_path_seq,
        txs,
    };

    Ok(new_template.into_static())
}

/// Build a SetNewPrevHash message from block header
///
/// # Arguments
/// * `template_id` - Template ID this prev_hash applies to
/// * `header` - Block header containing prev_hash, time, bits
pub fn build_set_new_prev_hash(template_id: u64, header: &Header) -> SetNewPrevHash<'static> {
    let prev_hash: U256<'static> = (*header.prev_blockhash.as_byte_array()).into();

    let target = Target::from(header.bits);
    let target_bytes: [u8; 32] = target.to_le_bytes();
    let target_u256: U256<'static> = U256::from(target_bytes);

    let set_new_prev_hash = SetNewPrevHash {
        template_id,
        prev_hash,
        header_timestamp: header.time,
        n_bits: header.bits.to_consensus(),
        target: target_u256,
    };

    set_new_prev_hash.into_static()
}

/// Build RequestTransactionDataSuccess message from block data
///
/// # Arguments
/// * `template_id` - Template ID this data applies to
/// * `block_data` - Serialized block data
pub fn build_request_transaction_data_success(
    template_id: u64,
    block_data: &[u8],
) -> Result<RequestTransactionDataSuccess<'static>, TemplateDataError> {
    let block: bitcoin::Block = deserialize(block_data)
        .map_err(|e| TemplateDataError::InvalidCoinbaseTx(format!("Block deserialize: {}", e)))?;

    let tx_data: Vec<B016M<'static>> = block
        .txdata
        .iter()
        .map(|tx| {
            serialize(tx)
                .try_into()
                .expect("tx data should always be valid for B016M")
        })
        .collect();

    let transaction_list =
        Seq064K::new(tx_data).expect("tx data should always be valid for Seq064K");

    let excess_data: B064K = vec![]
        .try_into()
        .expect("empty vec should always be valid for B064K");

    Ok(RequestTransactionDataSuccess {
        template_id,
        transaction_list,
        excess_data,
    }
    .into_static())
}

/// Create an empty transaction sequence for initial templates
pub fn empty_tx_sequence<'a>() -> Seq064K<'a, B016M<'a>> {
    Seq064K::new(vec![]).expect("empty vec should always be valid for Seq064K")
}

/// Build a NewTemplate message directly from node's BlockTemplate
///
/// # Arguments
/// * `template_id` - Unique identifier for this template
/// * `future_template` - Whether this is a future template
/// * `block_template` - The block template from node's IPC client
pub fn build_new_template_from_block_template(
    template_id: u64,
    future_template: bool,
    block_template: &BlockTemplate,
) -> Result<NewTemplate<'static>, TemplateDataError> {
    let components = &block_template.components;

    // Parse header from components
    let header: Header = deserialize(&components.header)
        .map_err(|e| TemplateDataError::InvalidCoinbaseTx(format!("Header deserialize: {}", e)))?;

    // Parse coinbase transaction
    let coinbase_tx: Transaction = deserialize(&components.coinbase_transaction).map_err(|e| {
        TemplateDataError::InvalidCoinbaseTx(format!("Coinbase deserialize: {}", e))
    })?;

    build_new_template(
        template_id,
        future_template,
        &header,
        &coinbase_tx,
        &components.coinbase_merkle_path,
        empty_tx_sequence(),
    )
}

/// Build a SetNewPrevHash message directly from node's BlockTemplate
///
/// # Arguments
/// * `template_id` - Template ID this prev_hash applies to
/// * `block_template` - The block template from node's IPC client
pub fn build_set_new_prev_hash_from_block_template(
    template_id: u64,
    block_template: &BlockTemplate,
) -> Result<SetNewPrevHash<'static>, TemplateDataError> {
    let header: Header = deserialize(&block_template.components.header)
        .map_err(|e| TemplateDataError::InvalidCoinbaseTx(format!("Header deserialize: {}", e)))?;

    Ok(build_set_new_prev_hash(template_id, &header))
}

/// Get the previous block hash as U256 from a BlockTemplate
pub fn get_prev_hash_from_block_template(
    block_template: &BlockTemplate,
) -> Result<U256<'static>, TemplateDataError> {
    let header: Header = deserialize(&block_template.components.header)
        .map_err(|e| TemplateDataError::InvalidCoinbaseTx(format!("Header deserialize: {}", e)))?;
    Ok((*header.prev_blockhash.as_byte_array()).into())
}

/// Build a BlockSubmissionRequest from an SV2 SubmitSolution message
///
/// This creates a submission request that can be sent through the block_submission channel
/// to ipc_block_listener, which will submit it via the IPC client.
///
/// # Arguments
/// * `template_id` - The template ID for this solution
/// * `block_template` - The block template used for this solution  
/// * `submit_solution` - The SV2 submit solution message
pub fn build_block_submission_request(
    template_id: u64,
    block_template: &BlockTemplate,
    submit_solution: &SubmitSolution<'static>,
) -> Result<BlockSubmissionRequest, TemplateDataError> {
    let components = &block_template.components;

    // Parse original header and coinbase
    let original_header: Header = deserialize(&components.header)
        .map_err(|e| TemplateDataError::InvalidCoinbaseTx(format!("Header deserialize: {}", e)))?;

    let original_coinbase_tx: Transaction =
        deserialize(&components.coinbase_transaction).map_err(|e| {
            TemplateDataError::InvalidCoinbaseTx(format!("Original coinbase deserialize: {}", e))
        })?;

    // Parse solution coinbase
    let solution_coinbase_tx_bytes: Vec<u8> = submit_solution.coinbase_tx.to_vec();
    let solution_coinbase_tx: Transaction =
        deserialize(&solution_coinbase_tx_bytes).map_err(|e| {
            error!("SubmitSolution.coinbase_tx is invalid: {}", e);
            TemplateDataError::InvalidCoinbaseTx(e.to_string())
        })?;

    // Validate solution coinbase against original
    if solution_coinbase_tx.version != original_coinbase_tx.version
        || solution_coinbase_tx.lock_time != original_coinbase_tx.lock_time
        || solution_coinbase_tx.input.len() != 1
        || solution_coinbase_tx.input[0].sequence != original_coinbase_tx.input[0].sequence
        || solution_coinbase_tx.input[0].witness != original_coinbase_tx.input[0].witness
        || solution_coinbase_tx.input[0].previous_output
            != original_coinbase_tx.input[0].previous_output
    {
        error!("Solution coinbase tx is not congruent with original coinbase tx");
        return Err(TemplateDataError::InvalidCoinbaseTx(
            "Coinbase tx mismatch".into(),
        ));
    }

    debug!(
        "Building submission: version={}, timestamp={}, nonce={}",
        submit_solution.version, submit_solution.header_timestamp, submit_solution.header_nonce
    );

    // Build the solution header
    let solution_header = bitcoin::block::Header {
        version: bitcoin::block::Version::from_consensus(submit_solution.version as i32),
        prev_blockhash: original_header.prev_blockhash,
        merkle_root: compute_merkle_root(&solution_coinbase_tx, &components.coinbase_merkle_path),
        time: submit_solution.header_timestamp,
        nonce: submit_solution.header_nonce,
        bits: original_header.bits,
    };

    Ok(BlockSubmissionRequest {
        template_id,
        header: solution_header,
        coinbase_transaction: solution_coinbase_tx,
    })
}

/// Compute merkle root from coinbase transaction and merkle path
fn compute_merkle_root(
    coinbase_tx: &Transaction,
    merkle_path: &[Vec<u8>],
) -> bitcoin::TxMerkleNode {
    let coinbase_txid = coinbase_tx.compute_txid();
    let mut current_hash = *coinbase_txid.as_byte_array();

    // Combine with each sibling hash in the merkle path
    for sibling_hash_bytes in merkle_path {
        let mut hasher = sha256d::Hash::engine();
        HashEngine::input(&mut hasher, &current_hash);
        HashEngine::input(&mut hasher, sibling_hash_bytes);
        current_hash = *sha256d::Hash::from_engine(hasher).as_byte_array();
    }

    sha256d::Hash::from_byte_array(current_hash).into()
}
