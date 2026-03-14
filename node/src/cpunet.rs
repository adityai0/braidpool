//! Cpunet implementation
//!
//! This module re-exports cpunet functionality from the `braidpool_common` crate.
//! All cpunet-specific types and functions are now shared across the workspace.

// Re-export everything from braidpool_common::cpunet
pub use braidpool_common::cpunet::*;

// Also re-export at module level for compatibility
pub use braidpool_common::{
    compute_block_hash, Cpunet, CpunetAddressError, CpunetParams, ParseCpunetError,
};
