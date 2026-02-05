/// Internal data structure for the ChannelManager.
///
/// This struct maintains all the state needed for SV2 channel management,
/// including pending channel requests, active channels, and mode-specific
/// data structures like extranonce factories for aggregated mode.
#[derive(Debug, Clone, Default)]
pub struct ChannelManagerData {
    // Extranonce prefix factory for allocating unique prefixes in aggregated mode
    // pub extranonce_prefix_factory: Option<ExtendedExtranonce>,
}

#[cfg_attr(not(test), hotpath::measure_all)]
impl ChannelManagerData {
    /// Creates a new ChannelManagerData instance.
    ///
    /// # Arguments
    /// * `mode` - The operational mode (Aggregated or NonAggregated)
    /// * `supported_extensions` - Extensions that the translator supports
    /// * `required_extensions` - Extensions that the translator requires
    ///
    /// # Returns
    /// A new ChannelManagerData instance with empty state
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets all channel state for upstream reconnection.
    ///
    /// This method clears all existing channel state that becomes invalid
    /// when the upstream connection is lost and reestablished. It preserves
    /// the operational mode and extension configuration but clears:
    /// - All pending channel requests
    /// - All active extended channels
    /// - The upstream extended channel
    /// - The extranonce prefix factory
    /// - Negotiated extensions (will be renegotiated with new connection)
    ///
    /// This ensures that new channels will be properly opened with the
    /// newly connected upstream server.
    pub fn reset_for_upstream_reconnection(&mut self) {
        // self.extranonce_prefix_factory = None;
        // Note: we intentionally preserve `mode`, `supported_extensions`, and `required_extensions`
        // as they are configuration settings
    }
}
