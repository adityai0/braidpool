use stratum_core::{codec_sv2::StandardSv2Frame, parsers_sv2::AnyMessage};

pub type TemplateId = u64;
pub type UpstreamJobId = u32;
pub type JobId = u32;
pub type DownstreamId = usize;
pub type RequestId = u32;
pub type ChannelId = u32;
pub type Hashrate = f32;
pub type SharesPerMinute = f32;
pub type SharesBatchSize = usize;

pub type Message = AnyMessage<'static>;
pub type StdFrame = StandardSv2Frame<Message>;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct VardiffKey {
    pub downstream_id: DownstreamId,
    pub channel_id: ChannelId,
}

impl From<(DownstreamId, ChannelId)> for VardiffKey {
    fn from(value: (DownstreamId, ChannelId)) -> Self {
        VardiffKey {
            downstream_id: value.0,
            channel_id: value.1,
        }
    }
}
