mod args;
use stratum_apps::config_helpers::logging::init_logging;
pub use translator_sv2::{config, error, status, sv1, sv2, TranslatorSv2};

use crate::args::process_cli_args;

#[cfg(feature = "hotpath-alloc")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    inner_main().await;
}

#[cfg(not(feature = "hotpath-alloc"))]
#[tokio::main]
async fn main() {
    inner_main().await;
}

/// Entrypoint for the Translator binary.
///
/// Loads the configuration from TOML and initializes the main runtime
/// defined in `translator_sv2::TranslatorSv2`. Errors during startup are logged.
#[hotpath::main]
async fn inner_main() {
    let proxy_config = process_cli_args().unwrap_or_else(|e| {
        eprintln!("Translator proxy config error: {e}");
        std::process::exit(1);
    });

    init_logging(proxy_config.log_dir());

    TranslatorSv2::new(proxy_config).start().await;
}
