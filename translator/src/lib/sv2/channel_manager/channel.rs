use async_channel::{Receiver, Sender};
use stratum_apps::stratum_core::parsers_sv2::Mining;
use tracing::debug;

use crate::status::Status;

#[derive(Clone, Debug)]
pub struct ChannelState {
    pub upstream_sender: Sender<Mining<'static>>,
    pub upstream_receiver: Receiver<Mining<'static>>,
    pub sv1_server_sender: Sender<Mining<'static>>,
    pub sv1_server_receiver: Receiver<Mining<'static>>,
    pub status_sender: Sender<Status>,
}

impl ChannelState {
    pub fn new(
        upstream_sender: Sender<Mining<'static>>,
        upstream_receiver: Receiver<Mining<'static>>,
        sv1_server_sender: Sender<Mining<'static>>,
        sv1_server_receiver: Receiver<Mining<'static>>,
        status_sender: Sender<Status>,
    ) -> Self {
        Self {
            upstream_sender,
            upstream_receiver,
            sv1_server_sender,
            sv1_server_receiver,
            status_sender,
        }
    }

    pub fn drop(&self) {
        debug!("Dropping channel manager channels");
        self.upstream_receiver.close();
        self.upstream_sender.close();
        self.sv1_server_receiver.close();
        self.sv1_server_sender.close();
    }
}
