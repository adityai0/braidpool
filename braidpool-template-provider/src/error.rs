//! Error types for braidpool-template-provider

use std::fmt;

/// Errors that can occur in the braidpool template provider
#[derive(Debug)]
pub enum BraidpoolTemplateProviderError {
    /// Failed to connect to IPC socket
    ConnectionFailed(String),
    /// IPC communication error
    IpcError(String),
    /// Invalid block template data
    InvalidTemplateData(String),
    /// Invalid coinbase transaction
    InvalidCoinbaseTx(String),
    /// Invalid block version
    InvalidBlockVersion,
    /// Invalid coinbase transaction version
    InvalidCoinbaseTxVersion,
    /// Invalid coinbase script sig
    InvalidCoinbaseScriptSig,
    /// Failed to serialize coinbase outputs
    CoinbaseOutputSerializationFailed,
    /// Failed to deserialize block
    BlockDeserializationFailed(String),
    /// Failed to submit solution
    FailedToSubmitSolution(String),
    /// Template not found
    TemplateNotFound(u64),
    /// Channel send error
    ChannelSendError(String),
    /// Channel receive error
    ChannelRecvError(String),
    /// Node is in initial block download
    NodeInIbd,
    /// Merkle path conversion error
    MerklePathError(String),
}

impl fmt::Display for BraidpoolTemplateProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            Self::IpcError(msg) => write!(f, "IPC error: {}", msg),
            Self::InvalidTemplateData(msg) => write!(f, "Invalid template data: {}", msg),
            Self::InvalidCoinbaseTx(msg) => write!(f, "Invalid coinbase tx: {}", msg),
            Self::InvalidBlockVersion => write!(f, "Invalid block version"),
            Self::InvalidCoinbaseTxVersion => write!(f, "Invalid coinbase tx version"),
            Self::InvalidCoinbaseScriptSig => write!(f, "Invalid coinbase script sig"),
            Self::CoinbaseOutputSerializationFailed => {
                write!(f, "Coinbase output serialization failed")
            }
            Self::BlockDeserializationFailed(msg) => {
                write!(f, "Block deserialization failed: {}", msg)
            }
            Self::FailedToSubmitSolution(msg) => write!(f, "Failed to submit solution: {}", msg),
            Self::TemplateNotFound(id) => write!(f, "Template not found: {}", id),
            Self::ChannelSendError(msg) => write!(f, "Channel send error: {}", msg),
            Self::ChannelRecvError(msg) => write!(f, "Channel receive error: {}", msg),
            Self::NodeInIbd => write!(f, "Node is in initial block download"),
            Self::MerklePathError(msg) => write!(f, "Merkle path error: {}", msg),
        }
    }
}

impl std::error::Error for BraidpoolTemplateProviderError {}

/// Errors specific to template data operations
#[derive(Debug)]
pub enum TemplateDataError {
    /// Invalid block version
    InvalidBlockVersion,
    /// Invalid coinbase transaction version
    InvalidCoinbaseTxVersion,
    /// Invalid coinbase script sig
    InvalidCoinbaseScriptSig,
    /// Failed to serialize coinbase outputs
    CoinbaseOutputSerializationFailed,
    /// Invalid coinbase transaction
    InvalidCoinbaseTx(String),
    /// Merkle path conversion error
    MerklePathError(String),
}

impl fmt::Display for TemplateDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBlockVersion => write!(f, "Invalid block version"),
            Self::InvalidCoinbaseTxVersion => write!(f, "Invalid coinbase tx version"),
            Self::InvalidCoinbaseScriptSig => write!(f, "Invalid coinbase script sig"),
            Self::CoinbaseOutputSerializationFailed => {
                write!(f, "Coinbase output serialization failed")
            }
            Self::InvalidCoinbaseTx(msg) => write!(f, "Invalid coinbase tx: {}", msg),
            Self::MerklePathError(msg) => write!(f, "Merkle path error: {}", msg),
        }
    }
}

impl std::error::Error for TemplateDataError {}
