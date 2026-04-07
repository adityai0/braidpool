use clap::Parser;
use std::path::PathBuf;

/// Default data directory path for Linux
#[cfg(target_os = "linux")]
const DEFAULT_DATADIR: &str = "~/.braidpool/";

/// Default data directory path for macOS
#[cfg(target_os = "macos")]
const DEFAULT_DATADIR: &str = "~/Library/Application Support/braidpool/";

/// Default Bitcoin RPC cookie path for Linux
#[cfg(target_os = "linux")]
const DEFAULT_RPC_COOKIE: &str = "~/.bitcoin/.cookie";

/// Default Bitcoin RPC cookie path for macOS
#[cfg(target_os = "macos")]
const DEFAULT_RPC_COOKIE: &str = "~/Library/Application Support/Bitcoin/.cookie";

/// Default IPC socket path for Linux (Unix domain socket)
#[cfg(target_os = "linux")]
const DEFAULT_IPC_SOCKET: &str = "/tmp/bitcoin-cpunet.sock";

/// Default IPC socket path for macOS (Unix domain socket)
#[cfg(target_os = "macos")]
const DEFAULT_IPC_SOCKET: &str = "/tmp/bitcoin-cpunet.sock";

#[derive(Parser, Debug, Clone)]
#[command(name = "braid", about = "Braidpool Node CLI")]
pub struct Cli {
    /// Braid data directory
    #[arg(long, default_value = DEFAULT_DATADIR)]
    pub datadir: PathBuf,

    /// Bind to a given address and always listen on it
    #[arg(long, default_value = "0.0.0.0:6680")]
    pub bind: String,

    /// Add a node to connect to and attempt to keep the connection open. This option can be
    /// specified multiple times
    #[arg(long)]
    pub addnode: Option<Vec<String>>,

    /// Connect to this bitcoin node
    #[arg(long, default_value = "0.0.0.0")]
    pub bitcoin: String,

    /// Use this port for bitcoin RPC
    #[arg(long, default_value = "8332")]
    pub rpcport: u16,

    /// Use this username for bitcoin RPC
    #[arg(long)]
    pub rpcuser: Option<String>,

    /// Use this password for bitcoin RPC
    #[arg(long)]
    pub rpcpass: Option<String>,

    /// Which network to use. Valid options are mainnet, testnet4, signet, cpunet (preferred)
    #[arg(long, default_value = "main")]
    pub network: Option<String>,

    /// Use this cookie file for bitcoin RPC
    #[arg(long, default_value = DEFAULT_RPC_COOKIE)]
    pub rpccookie: Option<String>,

    /// Path to Bitcoin Core IPC socket (Unix domain socket on Linux/macOS, named pipe on Windows)
    #[arg(long, default_value = DEFAULT_IPC_SOCKET)]
    pub ipc_socket: String,
}
