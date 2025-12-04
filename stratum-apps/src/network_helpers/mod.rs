//! High-level networking utilities for SV2 connections
//!
//! This module provides connection management, encrypted streams, and protocol handling
//! for Stratum V2 applications. It includes support for:
//!
//! - Noise-encrypted connections ([`noise_connection`], [`noise_stream`])
//! - SV1 protocol connections ([`sv1_connection`]) - when `sv1` feature is enabled
//!
//! Originally from the `network_helpers_sv2` crate.

pub mod noise_connection;
pub mod noise_stream;

#[cfg(feature = "sv1")]
pub mod sv1_connection;

use async_channel::{RecvError, SendError};
use std::fmt;
use stratum_core::codec_sv2::Error as CodecError;

/// Networking errors that can occur in SV2 connections
#[derive(Debug)]
pub enum Error {
    /// Invalid handshake message received from remote peer
    HandshakeRemoteInvalidMessage,
    /// Error from the codec layer
    CodecError(CodecError),
    /// Error receiving from async channel
    RecvError,
    /// Error sending to async channel
    SendError,
    /// Socket was closed, likely by the peer
    SocketClosed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::HandshakeRemoteInvalidMessage => {
                write!(f, "Invalid handshake message received from remote peer")
            }

            Error::CodecError(e) => write!(f, "{}", e),

            Error::RecvError => write!(f, "Error receiving from async channel"),

            Error::SendError => write!(f, "Error sending to async channel"),

            Error::SocketClosed => write!(f, "Socket was closed (likely by the peer)"),
        }
    }
}

impl From<CodecError> for Error {
    fn from(e: CodecError) -> Self {
        Error::CodecError(e)
    }
}

impl From<RecvError> for Error {
    fn from(_: RecvError) -> Self {
        Error::RecvError
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self {
        Error::SendError
    }
}
