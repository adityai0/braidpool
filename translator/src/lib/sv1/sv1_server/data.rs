use crate::sv1::downstream::downstream::Downstream;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc, RwLock},
};
use stratum_apps::{
    stratum_core::{
        bitcoin::Target, channels_sv2::vardiff::classic::VardiffState, mining_sv2::SetNewPrevHash,
        sv1_api::server_to_client,
    },
    utils::types::{ChannelId, DownstreamId, Hashrate},
};

#[derive(Debug, Clone)]
pub struct PendingTargetUpdate {
    pub downstream_id: DownstreamId,
    pub new_target: Target,
    pub new_hashrate: Hashrate,
}

#[derive(Debug)]
pub struct Sv1ServerData {
    pub downstreams: HashMap<DownstreamId, Arc<Downstream>>,
    pub vardiff: HashMap<DownstreamId, Arc<RwLock<VardiffState>>>,
    pub prevhash: Option<SetNewPrevHash<'static>>,
    pub downstream_id_factory: AtomicUsize,
    /// Job storage for aggregated mode - all Sv1 downstreams share the same jobs
    pub aggregated_valid_jobs: Option<Vec<server_to_client::Notify<'static>>>,
    /// Job storage for non-aggregated mode - each Sv1 downstream has its own jobs
    pub non_aggregated_valid_jobs:
        Option<HashMap<ChannelId, Vec<server_to_client::Notify<'static>>>>,
    /// Tracks pending target updates that are waiting for SetTarget response from upstream
    pub pending_target_updates: Vec<PendingTargetUpdate>,
    /// The initial target used when opening channels - used when no downstreams remain
    pub initial_target: Option<Target>,
}

impl Sv1ServerData {
    pub fn new(aggregate_channels: bool) -> Self {
        Self {
            downstreams: HashMap::new(),
            vardiff: HashMap::new(),
            prevhash: None,
            downstream_id_factory: AtomicUsize::new(0),
            aggregated_valid_jobs: aggregate_channels.then(Vec::new),
            non_aggregated_valid_jobs: (!aggregate_channels).then(HashMap::new),
            pending_target_updates: Vec::new(),
            initial_target: None,
        }
    }
}
