use core::fmt;

use braidpool_common::cpunet;
use miniscript::bitcoin::{address, hex};

/// Error enum
#[derive(Debug)]
pub enum Error {
    /// Error parsing a Bitcoin address
    Address(address::ParseError),
    /// Error parsing a raw descriptor as hex.
    Hex(hex::HexToBytesError),
    /// Invalid `output_script_value` for script type. It must be a valid public key/script
    InvalidOutputScript,
    /// Unknown script type in config
    UnknownOutputScriptType,
    /// Error from the `miniscript` crate.
    Miniscript(miniscript::Error),
    /// Parsing error for cpunet address
    ParseCpunetError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            Address(ref e) => write!(f, "Bitcoin address: {e}"),
            Hex(ref e) => write!(f, "Decoding hex-formatted script: {e}"),
            UnknownOutputScriptType => write!(f, "Unknown script type in config"),
            InvalidOutputScript => write!(f, "Invalid output_script_value for your script type. It must be a valid public key/script"),
            Miniscript(ref e) => write!(f, "Miniscript: {e}"),
            ParseCpunetError(ref error)=>write!(f,"Cpunet parse error - {error}")
        }
    }
}

impl From<address::ParseError> for Error {
    fn from(e: address::ParseError) -> Self {
        Error::Address(e)
    }
}

impl From<hex::HexToBytesError> for Error {
    fn from(e: hex::HexToBytesError) -> Self {
        Error::Hex(e)
    }
}

impl From<miniscript::Error> for Error {
    fn from(e: miniscript::Error) -> Self {
        Error::Miniscript(e)
    }
}
impl From<cpunet::CpunetAddressError> for Error {
    fn from(value: cpunet::CpunetAddressError) -> Self {
        match value {
            cpunet::CpunetAddressError::Bech32(e) => Self::ParseCpunetError(e.0.to_string()),
            cpunet::CpunetAddressError::InvalidProgram(e) => Self::ParseCpunetError(e),
            cpunet::CpunetAddressError::InvalidWitnessVersion(e) => {
                Self::ParseCpunetError(format!("Invalid witness verison received - {e}"))
            }
            cpunet::CpunetAddressError::WrongNetwork { expected, found } => Self::ParseCpunetError(
                format!("Invalid network recieved expected - {expected} found - {found} "),
            ),
        }
    }
}
