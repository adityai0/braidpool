//! CLI argument parsing for the Pool binary.
//!
//! Defines the `Args` struct and a function to process CLI arguments into a PoolConfig.

use clap::Parser;
use ext_config::{Config, File, FileFormat};
use node::error::KeyManagementError;
use node::key_management::load_or_generate_authority_keypair;
use pool_sv2::config::PoolConfig;
use std::path::PathBuf;

/// Holds the parsed CLI arguments for the Pool binary.
#[derive(Parser, Debug)]
#[command(author, version, about = "Pool CLI", long_about = None)]
pub struct Args {
    #[arg(
        short = 'c',
        long = "config",
        help = "Path to the TOML configuration file (optional). If not provided, default config with auto-generated keys will be used."
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

#[cfg_attr(not(test), hotpath::measure)]
/// Parses CLI arguments and loads the PoolConfig from the specified file.
pub fn process_cli_args() -> PoolConfig {
    let args = Args::parse();

    let mut config = match args.config_path {
        Some(config_path) => {
            let config_path_str = config_path.to_str().expect("Invalid config path");
            Config::builder()
                .add_source(File::new(config_path_str, FileFormat::Toml))
                .build()
                .and_then(|settings| settings.try_deserialize::<PoolConfig>())
                .expect("Failed to load or deserialize config")
        }
        None => {
            println!(
                "No config file provided, using default configuration with auto-generated keys"
            );
            create_default_config().expect("Failed to create default config")
        }
    };

    config.set_log_dir(args.log_file);

    // Override network if provided via CLI
    if let Some(network) = args.network {
        config.set_network(network);
    }

    config
}

/// Creates a default PoolConfig
fn create_default_config() -> Result<PoolConfig, KeyManagementError> {
    use pool_sv2::config::{AuthorityConfig, ConnectionConfig};
    use std::net::SocketAddr;
    use stratum_apps::{config_helpers::CoinbaseRewardScript, tp_type::TemplateProviderType};

    // Load or generate the authority keypair
    let (public_key, secret_key) = load_or_generate_authority_keypair()?;

    let listen_address: SocketAddr = "0.0.0.0:43333".parse().expect("Invalid default address");
    let cert_validity_sec = 3600;
    let pool_signature = "Stratum V2 SRI Pool".to_string();

    let connection_config =
        ConnectionConfig::new(listen_address, cert_validity_sec, pool_signature);
    let authority_config = AuthorityConfig::new(public_key, secret_key);

    let coinbase_reward_script =
        CoinbaseRewardScript::from_descriptor("addr(tc1qwjjhut55y70qv6k36et0kpe7vzh9kprjj9s5hk)")
            .expect("Invalid default coinbase reward script");

    let template_provider_type = TemplateProviderType::BitcoinCoreIpc {
        network: stratum_apps::tp_type::BitcoinNetwork::Cpunet,
        data_dir: None,
        fee_threshold: 100,
        min_interval: 5,
    };
    let shares_per_minute = 6.0;
    let share_batch_size = 10;
    let server_id = 1;
    let supported_extensions = vec![];
    let required_extensions = vec![];
    let network = "cpunet".to_string(); // Default network

    Ok(PoolConfig::new(
        connection_config,
        template_provider_type,
        authority_config,
        coinbase_reward_script,
        shares_per_minute,
        share_batch_size,
        server_id,
        supported_extensions,
        required_extensions,
        network,
    ))
}
