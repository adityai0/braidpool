use stratum_apps::stratum_core::sv1_api::server_to_client;

#[derive(Debug)]
pub struct Sv1ServerData {}

/// Delimiter used to separate original job ID from keepalive mutation counter.
/// Format: `{original_job_id}#{counter}`
pub const KEEPALIVE_JOB_ID_DELIMITER: char = '#';

impl Sv1ServerData {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Sv1ServerData {
    fn default() -> Self {
        Self::new()
    }
}
