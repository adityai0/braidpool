use pool_sv2::PoolSv2;
use stratum_apps::config_helpers::logging::init_logging;

use crate::args::process_cli_args;

mod args;

#[cfg(feature = "hotpath-alloc")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    _ = inner_main().await;
}

#[cfg(not(feature = "hotpath-alloc"))]
#[tokio::main]
async fn main() {
    _ = inner_main().await;
}

#[hotpath::main]
async fn inner_main() {
    let config = process_cli_args();
    init_logging(config.log_dir());
    if let Err(e) = PoolSv2::new(config).start().await {
        tracing::error!("Pool Error'ed out: {e}");
    };
}
