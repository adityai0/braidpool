use async_channel::{Receiver, Sender};
use stratum_apps::{
    stratum_core::sv1_api::json_rpc,
    utils::types::{ChannelId, DownstreamId},
};
use tokio::sync::broadcast;
use tracing::debug;

#[derive(Clone, Debug)]
pub struct DownstreamChannelState {
    pub downstream_sv1_sender: Sender<json_rpc::Message>,
    pub downstream_sv1_receiver: Receiver<json_rpc::Message>,
    pub sv1_server_sender: Sender<(DownstreamId, json_rpc::Message)>,
    pub sv1_server_broadcast:
        broadcast::Sender<(ChannelId, Option<DownstreamId>, json_rpc::Message)>, /* channel_id, optional downstream_id, message */
}

#[cfg_attr(not(test), hotpath::measure_all)]
impl DownstreamChannelState {
    pub fn new(
        downstream_sv1_sender: Sender<json_rpc::Message>,
        downstream_sv1_receiver: Receiver<json_rpc::Message>,
        sv1_server_sender: Sender<(DownstreamId, json_rpc::Message)>,
        sv1_server_broadcast: broadcast::Sender<(
            ChannelId,
            Option<DownstreamId>,
            json_rpc::Message,
        )>,
    ) -> Self {
        Self {
            downstream_sv1_receiver,
            downstream_sv1_sender,
            sv1_server_broadcast,
            sv1_server_sender,
        }
    }

    pub fn drop(&self) {
        debug!("Dropping downstream channel state");
        self.downstream_sv1_receiver.close();
        self.downstream_sv1_sender.close();
    }
}
