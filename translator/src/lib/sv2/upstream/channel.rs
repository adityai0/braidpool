use async_channel::{Receiver, Sender};
use stratum_apps::{stratum_core::parsers_sv2::Mining, utils::types::SV2Frame};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct UpstreamChannelState {
    /// Receiver for the SV2 Upstream role
    pub upstream_receiver: Receiver<SV2Frame>,
    /// Sender for the SV2 Upstream role
    pub upstream_sender: Sender<SV2Frame>,
    /// Sender for the ChannelManager thread
    pub channel_manager_sender: Sender<Mining<'static>>,
    /// Receiver for the ChannelManager thread
    pub channel_manager_receiver: Receiver<Mining<'static>>,
}

impl UpstreamChannelState {
    pub fn new(
        channel_manager_sender: Sender<Mining<'static>>,
        channel_manager_receiver: Receiver<Mining<'static>>,
        upstream_receiver: Receiver<SV2Frame>,
        upstream_sender: Sender<SV2Frame>,
    ) -> Self {
        Self {
            channel_manager_sender,
            channel_manager_receiver,
            upstream_receiver,
            upstream_sender,
        }
    }

    pub fn drop(&self) {
        debug!("Closing all upstream channels");
        self.upstream_receiver.close();
        self.upstream_receiver.close();
    }
}
