use crate::key_utils::Secp256k1PublicKey;
use std::path::PathBuf;

/// Which type of Template Provider will be used,
/// along with the relevant config parameters for each.
#[derive(Clone, Debug, serde::Deserialize)]
pub enum TemplateProviderType {
    Sv2Tp {
        address: String,
        public_key: Option<Secp256k1PublicKey>,
    },
    BitcoinCoreIpc {
        unix_socket_path: PathBuf,
        fee_threshold: u64,
        min_interval: u8,
    },
}
