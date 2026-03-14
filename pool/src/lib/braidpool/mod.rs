//! Braidpool P2P integration for the Pool crate.
//!
//! This module provides P2P networking integration for the pool, enabling
//! bead synchronization, peer discovery, and consensus participation.
//!
//! The P2P layer is spawned as a separate async task similar to how it's
//! done in the node crate's main.rs.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use libp2p::{floodsub, identity::Keypair};
use node::{
    behaviour::BRAIDPOOL_TOPIC,
    db::db_handlers::{fetch_beads_in_batch, DBHandler},
    ibd_manager::{IBDManager, IBD_TRIGGER_AFTER},
    peer_manager::PeerManager,
    swarm::{
        bootstrap::{dial_additional_nodes, setup_and_bootstrap},
        builder::{build_swarm, parse_bind_address, SwarmConfig},
        config::DEFAULT_P2P_PORT,
    },
    SwarmHandler,
};
use tokio::sync::{mpsc, RwLock};
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

pub use node::braid::Braid;
pub use node::db::BraidpoolDBTypes;
pub use node::swarm::p2p_event_loop;
pub use node::swarm::SwarmContext;
pub use node::utils::compute_block_hash;
pub use node::SwarmCommand;

#[derive(Debug, Clone)]
pub struct BraidpoolConfig {
    pub bind_addr: String,
    pub max_peers: usize,
    pub keystore_path: Option<std::path::PathBuf>,
    pub addnodes: Vec<String>,
    pub network_name: String,
}

impl Default for BraidpoolConfig {
    fn default() -> Self {
        Self {
            bind_addr: format!("0.0.0.0:{}", DEFAULT_P2P_PORT),
            max_peers: 8,
            keystore_path: None,
            addnodes: Vec::new(),
            network_name: "mainnet".to_string(),
        }
    }
}

pub struct BraidpoolP2P {
    config: BraidpoolConfig,
    braid: Arc<RwLock<Braid>>,
    ibd_spinlock: Arc<AtomicBool>,
    cancellation_token: CancellationToken,
    network_name: String,
}

impl BraidpoolP2P {
    pub fn new(config: BraidpoolConfig, cancellation_token: CancellationToken) -> Self {
        let network_name = config.network_name.clone();
        Self {
            config,
            braid: Arc::new(RwLock::new(Braid::new(Vec::new(), network_name.clone()))),
            ibd_spinlock: Arc::new(AtomicBool::new(true)),
            cancellation_token,
            network_name,
        }
    }
    #[inline]
    pub fn braid(&self) -> Arc<RwLock<Braid>> {
        self.braid.clone()
    }
    #[inline]
    pub fn is_in_ibd(&self) -> bool {
        self.ibd_spinlock.load(Ordering::SeqCst)
    }

    pub async fn spawn(
        self,
    ) -> Result<
        (
            tokio::task::JoinHandle<()>,
            mpsc::Sender<SwarmCommand>,
            mpsc::Sender<BraidpoolDBTypes>,
            Arc<RwLock<Braid>>,
        ),
        BraidpoolError,
    > {
        let (mut ibd_manager, ibd_command_tx) = IBDManager::new();
        let _ibd_handler = tokio::spawn(async move {
            ibd_manager.run_ibd_handler().await;
        });
        let (mut db_handler, db_tx) = DBHandler::new(self.network_name.clone())
            .await
            .map_err(|e| BraidpoolError::Database(format!("{:?}", e)))?;

        let db_connection_pool = db_handler.db_connection_pool.clone();
        // Load beads from database before spawning other tasks
        let braid_ref = self.braid.clone();
        let initial_bead_fetch_handle = tokio::spawn(async move {
            let mut guard = braid_ref.write().await;
            let fetched_beads = fetch_beads_in_batch(db_connection_pool, 1000).await?;
            for bead in &fetched_beads {
                let status = guard.extend(bead);
                debug!(hash = ?bead.block_header.block_hash(), status = ?status, "Bead inserted");
            }
            info!(beads = fetched_beads.len(), "Beads loaded from DB");
            Ok::<(), node::error::DBErrors>(())
        });

        match initial_bead_fetch_handle.await {
            Ok(Ok(())) => info!("Initial bead fetch completed"),
            Ok(Err(e)) => {
                error!(error = ?e, "Failed to fetch beads from DB");
                return Err(BraidpoolError::Database(format!(
                    "Database bead fetch failed: {:?}",
                    e
                )));
            }
            Err(e) => {
                error!(error = ?e, "Initial bead fetch task panicked");
                return Err(BraidpoolError::Database(format!(
                    "Initial bead fetch task panicked: {}",
                    e
                )));
            }
        }

        // Start DB query handler
        tokio::spawn(async move {
            let _res = db_handler.insert_query_handler().await;
        });

        let (swarm_handler, swarm_command_receiver) =
            SwarmHandler::new(Arc::clone(&self.braid), db_tx.clone());
        let swarm_command_sender = swarm_handler.command_sender.clone();

        let keypair = if let Some(keystore_path) = &self.config.keystore_path {
            load_or_generate_keypair(keystore_path)?
        } else {
            info!("Generating ephemeral P2P keypair");
            Keypair::generate_ed25519()
        };

        let bind_addr = parse_bind_address(&self.config.bind_addr, DEFAULT_P2P_PORT)
            .map_err(|e| BraidpoolError::Config(format!("Invalid bind address: {}", e)))?;

        let swarm_config = SwarmConfig::new(keypair, bind_addr);
        let listen_addr = swarm_config
            .to_multiaddr()
            .map_err(|e| BraidpoolError::Config(format!("Invalid multiaddr: {}", e)))?;
        debug!("Braidpool config- {:?}", swarm_config);
        let mut swarm = build_swarm(swarm_config)
            .await
            .map_err(|e| BraidpoolError::Swarm(format!("{}", e)))?;

        let topic = floodsub::Topic::new(BRAIDPOOL_TOPIC);
        setup_and_bootstrap(&mut swarm, listen_addr, topic)
            .map_err(|e| BraidpoolError::Swarm(format!("{}", e)))?;

        if !self.config.addnodes.is_empty() {
            dial_additional_nodes(&mut swarm, &self.config.addnodes);
        }
        let peer_manager = PeerManager::new(self.config.max_peers);
        let ctx = SwarmContext::new(
            self.braid.clone(),
            db_tx.clone(),
            ibd_command_tx,
            self.ibd_spinlock.clone(),
            peer_manager,
            swarm_command_sender.clone(),
            self.network_name,
        );

        let braid_ref = self.braid.clone();
        let cancellation_token = self.cancellation_token.clone();
        //Trigger task for initiating IBD
        let swarm_command_tx = swarm_command_sender.clone();
        let _ibd_trigger_handler = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(IBD_TRIGGER_AFTER)).await;
            if let Err(e) = swarm_command_tx.send(SwarmCommand::InitiateIBD).await {
                error!(error = ?e, "Failed to initiate IBD");
            } else {
                info!("IBD trigger sent");
            }
        });
        let handle = tokio::spawn(async move {
            tokio::select! {
                _ = p2p_event_loop::run(ctx, swarm, swarm_command_receiver) => {
                    info!("P2P event loop completed");
                }
                _ = cancellation_token.cancelled() => {
                    info!("P2P cancelled via token");
                }
            }
        });

        info!("Braidpool P2P started");
        Ok((handle, swarm_command_sender, db_tx, braid_ref))
    }
}

fn load_or_generate_keypair(keystore_path: &std::path::Path) -> Result<Keypair, BraidpoolError> {
    use std::fs;

    match fs::read(keystore_path) {
        Ok(bytes) => {
            info!(path = %keystore_path.display(), "Loading keypair from keystore");
            Keypair::from_protobuf_encoding(&bytes)
                .map_err(|e| BraidpoolError::Keystore(format!("Failed to decode keypair: {}", e)))
        }
        Err(_) => {
            info!(path = %keystore_path.display(), "Generating new keypair");
            let keypair = Keypair::generate_ed25519();
            let bytes = keypair.to_protobuf_encoding().map_err(|e| {
                BraidpoolError::Keystore(format!("Failed to encode keypair: {}", e))
            })?;

            if let Some(parent) = keystore_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    BraidpoolError::Keystore(format!("Failed to create keystore dir: {}", e))
                })?;
            }

            fs::write(keystore_path, bytes).map_err(|e| {
                BraidpoolError::Keystore(format!("Failed to write keystore: {}", e))
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(keystore_path)
                    .map_err(|e| BraidpoolError::Keystore(format!("Failed to get perms: {}", e)))?
                    .permissions();
                perms.set_mode(0o400);
                fs::set_permissions(keystore_path, perms)
                    .map_err(|e| BraidpoolError::Keystore(format!("Failed to set perms: {}", e)))?;
            }

            Ok(keypair)
        }
    }
}

#[derive(Debug)]
pub enum BraidpoolError {
    Config(String),
    Database(String),
    Keystore(String),
    Swarm(String),
}

impl std::fmt::Display for BraidpoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BraidpoolError::Config(msg) => write!(f, "Config error: {}", msg),
            BraidpoolError::Database(msg) => write!(f, "Database error: {}", msg),
            BraidpoolError::Keystore(msg) => write!(f, "Keystore error: {}", msg),
            BraidpoolError::Swarm(msg) => write!(f, "Swarm error: {}", msg),
        }
    }
}

impl std::error::Error for BraidpoolError {}
