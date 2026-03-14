//! Defines the structure and parsing logic for command-line arguments.
//!
//! It provides the `Args` struct to hold parsed arguments,
//! and the `from_args` function to parse them from the command line.
use clap::Parser;
use ext_config::{Config, File, FileFormat};
use node::error::KeyManagementError;
use node::key_management::load_authority_public_key;
use std::path::PathBuf;
use tracing::{error, info};
use translator_sv2::{
    config::{DownstreamDifficultyConfig, TranslatorConfig, Upstream},
    error::TproxyErrorKind,
};

/// Holds the parsed CLI arguments.
#[derive(Parser, Debug)]
#[command(author, version, about = "Translator Proxy", long_about = None)]
pub struct Args {
    #[arg(
        short = 'c',
        long = "config",
        help = "Path to the TOML configuration file (optional). If not provided, default config will be used with auto-loaded authority key."
    )]
    pub config_path: Option<PathBuf>,
    #[arg(
        short = 'f',
        long = "log-file",
        help = "Path to the log file. If not set, logs will only be written to stdout."
    )]
    pub log_file: Option<PathBuf>,
    #[arg(
        short = 'n',
        long = "network",
        help = "Which network to use. Valid options are mainnet, testnet4, signet, cpunet (preferred)"
    )]
    pub network: Option<String>,
}

/// Process CLI args, if any.
#[allow(clippy::result_large_err)]
pub fn process_cli_args() -> Result<TranslatorConfig, TproxyErrorKind> {
    // Parse CLI arguments
    let args = Args::parse();

    let mut config = match args.config_path {
        Some(config_path) => {
            // Build configuration from the provided file path
            let config_path_str = config_path.to_str().ok_or_else(|| {
                error!("Invalid configuration path.");
                TproxyErrorKind::BadCliArgs
            })?;

            let settings = Config::builder()
                .add_source(File::new(config_path_str, FileFormat::Toml))
                .build()?;

            settings.try_deserialize::<TranslatorConfig>()?
        }
        None => {
            info!("No config file provided, using default configuration with auto-loaded authority key");
            create_default_config().map_err(|e| {
                error!("Failed to create default config: {}", e);
                TproxyErrorKind::BadCliArgs
            })?
        }
    };

    config.set_log_dir(args.log_file);

    // Override network if provided via CLI
    if let Some(network) = args.network {
        config.set_network(network);
    }

    Ok(config)
}

/// Creates a default TranslatorConfig by loading the authority public key
fn create_default_config() -> Result<TranslatorConfig, KeyManagementError> {
    // Load the authority public key from the Pool's key storage
    let authority_pubkey = load_authority_public_key()?;

    info!("Loaded authority public key from ~/.braidpool/authority.key");

    let upstream = Upstream::new("127.0.0.1".to_string(), 43333, authority_pubkey);

    let downstream_difficulty_config =
        DownstreamDifficultyConfig::new(10_000_000_000_000.0, 6.0, false, 60);

    Ok(TranslatorConfig::new(
        vec![upstream],
        "0.0.0.0".to_string(),
        34255,
        downstream_difficulty_config,
        2,
        2,
        4,
        "braidpool_miner".to_string(),
        true,
        vec![],
        vec![],
        "cpunet".to_string(), // Default network
    ))
}
