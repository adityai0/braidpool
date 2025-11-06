use stratum_core::{
    buffer_sv2,
    codec_sv2::{StandardEitherFrame, StandardSv2Frame},
    framing_sv2::framing::Sv2Frame,
    parsers_sv2::AnyMessage,
};

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
pub type EitherFrame = StandardEitherFrame<Message>;
pub type SV2Frame = Sv2Frame<Message, buffer_sv2::Slice>;

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
