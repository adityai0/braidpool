//These implementations must be defined under lib.rs as they are required for intergration tests
use crate::db::db_handlers::prepare_bead_tuple_data;
use bitcoin::{
    absolute::Time,
    consensus::encode::{deserialize, Decodable, Encodable, Error as ConsensusError},
    io::{Read, Write},
    BlockHash, CompactTarget, Txid,
};
use futures::lock::Mutex;
use num::ToPrimitive;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::UNIX_EPOCH,
};
use stratum_apps::key_utils::{Secp256k1PublicKey, Secp256k1SecretKey};
use stratum_apps::secp256k1::{All, Keypair, Message, Secp256k1};
use tokio::sync::mpsc::{self, Receiver, Sender};
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

/// Maximum number of block templates to retain in the in-memory cache.
///
/// This constant limits how many recent block templates, fetched via IPC from Bitcoin node,
/// are kept available for downstream miners. When the cache exceeds this size, the oldest
/// templates are evicted to make room for new ones. This helps prevent unbounded memory
/// growth and ensures efficient resource usage.
pub const MAX_CACHED_TEMPLATES: usize = 90;

use crate::{
    bead::Bead,
    braid::{AddBeadStatus, Braid},
    committed_metadata::{CommittedMetadata, TimeVec, TxIdVec},
    db::BraidpoolDBTypes,
    error::{IPCtemplateError, StratumErrors},
    stratum::{BlockTemplate, NotifyCmd},
    uncommitted_metadata::UnCommittedMetadata,
};
use std::error::Error;
#[macro_use]
pub mod macros;
pub mod bead;
pub mod behaviour;
pub mod braid;
pub mod cli;
pub mod committed_metadata;
pub mod config;
pub mod cpunet;
pub mod db;
pub mod error;
pub mod ibd_manager;
pub mod ipc;
pub mod key_management;
pub mod peer_manager;
pub mod rpc_server;
pub mod stratum;
pub mod swarm;
pub mod template_creator;
pub mod uncommitted_metadata;
pub mod utils;
use std::sync::atomic::{AtomicU64, Ordering};

//Including the capnp modules after building while compiling the workspace.package
pub mod proxy_capnp {
    include!(concat!(env!("OUT_DIR"), "/proxy_capnp.rs"));
}
pub mod mining_capnp {
    include!(concat!(env!("OUT_DIR"), "/mining_capnp.rs"));
}
pub mod echo_capnp {
    include!(concat!(env!("OUT_DIR"), "/echo_capnp.rs"));
}
pub mod common_capnp {
    include!(concat!(env!("OUT_DIR"), "/common_capnp.rs"));
}
pub mod init_capnp {
    include!(concat!(env!("OUT_DIR"), "/init_capnp.rs"));
}

/// Unique identifier assigned to each block template.
pub type TemplateId = u64;

/// Global template ID counter that persists across the application lifetime
static GLOBAL_TEMPLATE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Get the next unique template ID (increments on each call)
pub fn get_next_template_id() -> TemplateId {
    GLOBAL_TEMPLATE_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// **Length of the extranonce prefix (in bytes).**
///
/// In Stratum mining, the extranonce is split into two parts:
/// `EXTRANONCE1` (prefix) and `EXTRANONCE2` (suffix).
///
/// This constant defines the size of `EXTRANONCE1` as **4 bytes**.
/// Typically assigned by the mining pool to uniquely identify a miner generated randomly or can be done via the peer_addr hash.
pub const EXTRANONCE1_SIZE: usize = 4;

/// **Length of the extranonce suffix (in bytes).**
///
///These are the rollable bits defined under the extanonce,along with nonce and Version which can be worked upon to produce suitable valid share
/// being submitted by the miner via `mining.submit` .
pub const EXTRANONCE2_SIZE: usize = 4;
/// **Separator between `EXTRANONCE1` and `EXTRANONCE2`.**
///
/// This is an array of bytes used to clearly delimit the two extranonce parts.
/// In this testing configuration, the separator length equals
/// `EXTRANONCE1_SIZE + EXTRANONCE2_SIZE` (8 bytes total),
/// and is filled with the byte value `1u8` for simplicity.
/// can be changed accordingly as per discussion .
pub const EXTRANONCE_SEPARATOR: [u8; EXTRANONCE1_SIZE + EXTRANONCE2_SIZE] =
    [1u8; EXTRANONCE1_SIZE + EXTRANONCE2_SIZE];
/// Consumes block templates received via an IPC channel, updates shared state,
/// and notifies all connected consumers.
///
/// # Parameters
///
/// * `template_rx` - An asynchronous mpsc receiver providing block templates.
///   Each message is a tuple:
///     - `Vec<u8>`: Raw serialized block data.
///     - `Vec<Vec<u8>>`: Merkle branch data for the coinbase transaction.
/// * `notifier_tx` - An asynchronous mpsc sender used to notify all connected
///   components when a new block template is available.
/// * `latest_template_arc` - A thread-safe, mutable reference to the shared
///   [`BlockTemplate`] state, wrapped in an [`Arc`] and [`Mutex`].
/// * `latest_template_merkle_branch_arc` - A thread-safe, mutable reference to the
///   latest Merkle branch data for the coinbase transaction, wrapped in an [`Arc`] and [`Mutex`].
///
/// # Returns
///
/// * `Ok(())` - When the consumer loop completes without errors.
/// * `Err(IPCtemplateError)` - If an unrecoverable IPC template handling error occurs.
pub async fn ipc_template_consumer(
    mut template_rx: mpsc::Receiver<Arc<crate::ipc::client::BlockTemplate>>,
    notifier_tx: mpsc::Sender<NotifyCmd>,
    latest_template_arc: &mut Arc<Mutex<BlockTemplate>>,
    latest_template_merkle_branch_arc: &mut Arc<Mutex<Vec<Vec<u8>>>>,
    template_cache: Arc<
        tokio::sync::Mutex<HashMap<TemplateId, Arc<crate::ipc::client::BlockTemplate>>>,
    >,
    latest_template_id: Arc<Mutex<TemplateId>>,
) -> Result<(), IPCtemplateError> {
    while let Some(ipc_template) = template_rx.recv().await {
        let template_bytes = match &ipc_template.processed_block_hex {
            Some(processed_hex) if !processed_hex.is_empty() => processed_hex,
            _ => {
                warn!(
                    field = "processed_block_hex",
                    "Skipping invalid template - hex payload missing"
                );
                continue;
            }
        };

        if template_bytes.len() > 0 {
            // Generate new template_id for every template
            let template_id = get_next_template_id();
            {
                let mut latest_id = latest_template_id.lock().await;
                *latest_id = template_id;
            }

            // Cache the IPC template with this new ID
            {
                let mut cache = template_cache.lock().await;
                cache.insert(template_id, ipc_template.clone());

                // Cleanup old templates
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

            let candidate_block: Result<
                bitcoin::blockdata::block::Block,
                bitcoin::consensus::encode::Error,
            > = deserialize(&template_bytes);

            let merkle_branch_coinbase = ipc_template.components.coinbase_merkle_path.clone();
            let (template_header, template_transactions) = match candidate_block {
                Ok(template) => (template.header, template.txdata),
                Err(_error) => {
                    return Err(IPCtemplateError::TemplateConsumeError);
                }
            };
            let _coinbase_transaction = template_transactions.get(0);

            debug!(template_id = %template_id, template_header = ?template_header, "New block template");
            let template: BlockTemplate = BlockTemplate {
                version: template_header.version,
                previousblockhash: template_header.prev_blockhash,
                transactions: template_transactions.clone(),
                curtime: template_header.time,
                bits: template_header.bits,
                ..Default::default()
            };

            let mut latest_template = latest_template_arc.lock().await;
            latest_template.version = template.version;
            latest_template.rules = template.rules.clone();
            latest_template.vbavailable = template.vbavailable.clone();
            latest_template.vbrequired = template.vbrequired;
            latest_template.previousblockhash = template.previousblockhash.clone();
            latest_template.transactions = template.transactions.clone();
            latest_template.coinbaseaux = template.coinbaseaux.clone();
            latest_template.coinbasevalue = template.coinbasevalue;
            latest_template.longpollid = template.longpollid.clone();
            latest_template.target = template.target.clone();
            latest_template.mintime = template.mintime;
            latest_template.mutable = template.mutable.clone();
            latest_template.noncerange = template.noncerange.clone();
            latest_template.sigoplimit = template.sigoplimit;
            latest_template.sizelimit = template.sizelimit;
            latest_template.weightlimit = template.weightlimit;
            latest_template.curtime = template.curtime;
            latest_template.bits = template.bits;
            latest_template.height = template.height;
            latest_template.default_witness_commitment =
                template.default_witness_commitment.clone();
            let mut latest_template_merkle_branch = latest_template_merkle_branch_arc.lock().await;
            latest_template_merkle_branch.clear();
            for branch in merkle_branch_coinbase.iter() {
                latest_template_merkle_branch.push(branch.clone());
            }
            info!(
                template_id = %template_id,
                tx_count = %template_transactions.len(),
                "New block template"
            );

            let notification_sent_or_not = notifier_tx
                .send(NotifyCmd::SendToAll {
                    template: template,
                    merkle_branch_coinbase,
                    template_id,
                })
                .await;
            match notification_sent_or_not {
                Ok(_) => {
                    debug!(template_id = %template_id, "Template sent to notifier");
                }
                Err(error) => {
                    error!(error = ?error, "Failed to send template notification");
                }
            }
        } else {
            warn!(size_bytes = 0, expected_min = 80, "IPC template too short");
        }
    }

    Ok(())
}
pub enum SwarmCommand {
    PropagateValidBead { bead_bytes: Vec<u8> },
    //Initiate IBD after waiting for connection_mapping to be populated via peer discovery
    InitiateIBD,
}
pub fn compute_sha256_hash(bytes: &[u8]) -> [u8; 32] {
    let mut sha_256_engine = Sha256::new();
    sha_256_engine.update(bytes);
    let tag_bytes = sha_256_engine.finalize();
    tag_bytes.try_into().unwrap()
}
#[derive(Debug, Serialize, Deserialize, Clone)]
// Message to be signed for signature generation via secret_key
pub struct SignatureCommitment {
    pub extra_nonce_1: Vec<u8>,
    pub extra_nonce_2: Vec<u8>,
    pub broadcast_timestamp: Time,
}

impl Encodable for SignatureCommitment {
    fn consensus_encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, bitcoin::io::Error> {
        let mut len = 0;
        len += self.extra_nonce_1.consensus_encode(w)?;
        len += self.extra_nonce_2.consensus_encode(w)?;
        len += self
            .broadcast_timestamp
            .to_consensus_u32()
            .consensus_encode(w)?;
        Ok(len)
    }
}

impl Decodable for SignatureCommitment {
    fn consensus_decode<R: Read + ?Sized>(r: &mut R) -> Result<Self, ConsensusError> {
        let extra_nonce_1 = Vec::consensus_decode(r)?;
        let extra_nonce_2 = Vec::consensus_decode(r)?;
        let broadcast_timestamp = Time::from_consensus(u32::consensus_decode(r)?)
            .map_err(|_| ConsensusError::ParseFailed("invalid timestamp"))?;

        Ok(SignatureCommitment {
            extra_nonce_1,
            extra_nonce_2,
            broadcast_timestamp,
        })
    }
}

pub struct SwarmHandler {
    pub command_sender: Sender<SwarmCommand>,
    braid_arc: Arc<tokio::sync::RwLock<Braid>>,
    db_command_sender: tokio::sync::mpsc::Sender<BraidpoolDBTypes>,
    secret_key_auth: Secp256k1SecretKey,
    public_key_auth: Secp256k1PublicKey,
}
impl SwarmHandler {
    pub fn new(
        braid_arc: Arc<tokio::sync::RwLock<Braid>>,
        db_command_sender: tokio::sync::mpsc::Sender<BraidpoolDBTypes>,
        secret_key_auth: Secp256k1SecretKey,
        public_key_auth: Secp256k1PublicKey,
    ) -> (Self, Receiver<SwarmCommand>) {
        let (swarm_stratum_bridge_tx, swarm_stratum_bridge_rx) =
            mpsc::channel::<SwarmCommand>(1024);
        (
            Self {
                command_sender: swarm_stratum_bridge_tx,
                braid_arc: Arc::clone(&braid_arc),
                db_command_sender,
                public_key_auth: public_key_auth,
                secret_key_auth: secret_key_auth,
            },
            swarm_stratum_bridge_rx,
        )
    }
    pub async fn propagate_valid_bead(
        &mut self,
        candidate_block: bitcoin::Block,
        extranonce_2_raw_value: Vec<u8>,
        downstream_client_ip: &str,
        job_sent_timestamp: u32,
        downstream_payout_addr: &str,
        //TODO: Will be used as seperate entity after altering `uncommitted_metadata`
        extranonce_1_raw_value: Vec<u8>,
    ) -> Result<(), StratumErrors> {
        let candidate_block_header = candidate_block.header;
        let candidate_block_transactions = candidate_block.txdata;
        let ids: Vec<Txid> = candidate_block_transactions
            .iter()
            .map(|tx| tx.compute_txid())
            .collect();
        let transaction_ids: Vec<Txid> = Vec::from(ids);
        debug!("Broadcasting bead via floodsub");
        let mut time_hash_set = TimeVec(Vec::new());
        let mut parent_hash_set: HashSet<BlockHash> = HashSet::new();
        let mut braid_data = self.braid_arc.write().await;
        let tips_index = &braid_data.tips;
        //Committing parents data in bead
        for tip_bead in tips_index {
            let current_tip_bead = braid_data.beads.get(*tip_bead).unwrap();
            parent_hash_set.insert(braid_data.compute_bead_hash(current_tip_bead));
            time_hash_set
                .0
                .push(current_tip_bead.committed_metadata.start_timestamp);
        }
        debug!(tip_indices = ?tips_index, tip_hashes = ?parent_hash_set,
            "Tips before extending the Braid");
        //TODO:This will be replaced via the allotted `WeakShareDifficulty` after Difficulty adjustment
        let weak_target = CompactTarget::from_unprefixed_hex("1d00ffff").unwrap();
        //Mindiff
        let min_target = CompactTarget::from_unprefixed_hex("1d00ffff").unwrap();
        //Job sent time before downstream starts mining
        let job_notification_time_val =
            bitcoin::blockdata::locktime::absolute::Time::from_consensus(job_sent_timestamp)
                .unwrap();
        let candidate_block_bead_committed_metadata = CommittedMetadata {
            comm_pub_key: self.public_key_auth.0,
            transaction_ids: TxIdVec(transaction_ids),
            parents: parent_hash_set,
            parent_bead_timestamps: time_hash_set,
            payout_address: downstream_payout_addr.to_string(),
            start_timestamp: job_notification_time_val,
            min_target: min_target,
            weak_target: weak_target,
            miner_ip: downstream_client_ip.to_string(),
        };
        //Current UNIX timestamp during broadcast of bead
        let current_system_time = std::time::SystemTime::now();
        let duration_since_epoch = match current_system_time.duration_since(UNIX_EPOCH) {
            Ok(duration) => duration,
            Err(error) => {
                return Err(StratumErrors::ErrorFetchingCurrentUNIXTimestamp {
                    error: error.to_string(),
                })
            }
        };

        let unix_timestamp = duration_since_epoch.as_secs().to_u32().unwrap();

        let msg_commitment = SignatureCommitment {
            broadcast_timestamp: bitcoin::absolute::Time::from_consensus(unix_timestamp).unwrap(),
            extra_nonce_1: extranonce_1_raw_value.clone(),
            extra_nonce_2: extranonce_2_raw_value.clone(),
        };
        let msg_bytes = bitcoin::consensus::serialize(&msg_commitment);
        // Signature formation for adding inside `UnCommittedMetadata`
        let engine = Secp256k1::<All>::gen_new();
        let msg = Message::from_digest(compute_sha256_hash(&msg_bytes));
        let sign = engine.sign_schnorr(
            &msg,
            &Keypair::from_secret_key(&engine, &self.secret_key_auth.0),
        );
        let candidate_block_bead_uncommitted_metadata = UnCommittedMetadata {
            broadcast_timestamp: bitcoin::absolute::Time::from_consensus(unix_timestamp).unwrap(),
            extra_nonce_1: extranonce_1_raw_value,
            extra_nonce_2: extranonce_2_raw_value,
            signature: sign,
        };
        let weak_share = Bead {
            committed_metadata: candidate_block_bead_committed_metadata,
            block_header: candidate_block_header,
            uncommitted_metadata: candidate_block_bead_uncommitted_metadata,
        };
        let status = braid_data.extend(&weak_share);
        match status {
            AddBeadStatus::BeadAdded => {
                let new_tips: Vec<_> = braid_data.tips.iter().map(|&idx| idx).collect();
                info!(
                    hash = %braid_data.compute_bead_hash(&weak_share),
                    new_tips = ?new_tips,
                    "Braid extended successfully"
                );
                //Considering the index of the beads in braid will be same as the (insertion ids-1)
                let bead_id = braid_data
                    .bead_index_mapping
                    .get(&braid_data.compute_bead_hash(&weak_share))
                    .unwrap();
                let (txs_json, relative_json, parent_timestamp_json) = prepare_bead_tuple_data(
                    &braid_data.beads,
                    &braid_data.bead_index_mapping,
                    &weak_share,
                    &braid_data.network_name,
                )
                .unwrap();
                let _db_insertion_command = match self
                    .db_command_sender
                    .send(BraidpoolDBTypes::InsertTupleTypes {
                        query: db::InsertTupleTypes::InsertBeadSequentially {
                            bead_to_insert: weak_share.clone(),
                            txs_json: txs_json,
                            relative_json: relative_json,
                            parent_timestamp_json: parent_timestamp_json,
                            bead_id: *bead_id,
                        },
                    })
                    .await
                {
                    Ok(_) => {
                        debug!(
                            hash = %braid_data.compute_bead_hash(&weak_share),
                            "InsertBeadSequentially sent to DB thread"
                        );
                    }
                    Err(error) => {
                        error!(error = ?error, "Database insertion command failed");
                    }
                };
                let serialized_weak_share_bytes = bitcoin::consensus::serialize(&weak_share);
                //After validation of the candidate block constructed by the downstream node sending it to swarm for further propogation
                match self
                    .command_sender
                    .send(SwarmCommand::PropagateValidBead {
                        bead_bytes: serialized_weak_share_bytes,
                    })
                    .await
                {
                    Ok(_) => {
                        info!(
                            hash = %braid_data.compute_bead_hash(&weak_share),
                            "Bead sent to swarm"
                        );
                    }
                    Err(e) => {
                        error!(
                            hash = %braid_data.compute_bead_hash(&weak_share),
                            error = %e,
                            "Failed to send candidate block to swarm"
                        );
                        return Err(StratumErrors::CandidateBlockNotSent {
                            error: e.to_string(),
                        });
                    }
                };
            }
            _ => {
                warn!(status = ?status, hash = %braid_data.compute_bead_hash(&weak_share),
                    "Failed to extend Braid")
            }
        }
        Ok(())
    }
}

pub fn setup_tracing() -> Result<(), Box<dyn Error>> {
    // Create a filter that uses RUST_LOG environment variable if set,
    // otherwise falls back to reasonable defaults
    let filter = if std::env::var("RUST_LOG").is_ok() {
        // If RUST_LOG is set, use it exactly as provided
        tracing_subscriber::EnvFilter::from_default_env()
    } else {
        // If no RUST_LOG is set, use sensible defaults
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("node=info".parse()?)
            .add_directive("libp2p=info".parse()?)
    };

    // Enable file and line number logging when RUST_LOG includes debug or trace
    let show_location = std::env::var("RUST_LOG")
        .map(|v| v.contains("debug") || v.contains("trace"))
        .unwrap_or(false);

    // Build and initialize a `tracing` subscriber with the specified filter, colors, and target/module prefixes
    // The .init() method automatically calls LogTracer::init() when the tracing-log feature is enabled,
    // which intercepts log:: calls from dependencies (like libp2p, sqlx) and forwards them to tracing
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_target(true) // Show the target/module (e.g., "libp2p::kad", "node::main")
        .with_thread_ids(false) // Set to true if you want thread IDs
        .with_thread_names(false) // Set to true if you want thread names
        .with_file(show_location) // Show file names when RUST_LOG=debug or RUST_LOG=trace
        .with_line_number(show_location) // Show line numbers when RUST_LOG=debug or RUST_LOG=trace
        .with_ansi(true) // Enable ANSI colors
        .compact() // Use a more compact format that works well with colors
        .init();

    Ok(())
}
