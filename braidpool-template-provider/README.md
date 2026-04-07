# Braidpool Template Provider

A Rust library that provides block templates for Stratum V2 mining by wrapping Braidpool's `node` crate IPC client functionality. This is a drop-in replacement for `bitcoin-core-sv2` that uses Braidpool's node infrastructure.

## Overview

`braidpool_template_provider` acts as a Template Distribution Protocol server, converting block templates from Bitcoin Core (via node's IPC client) into SV2 `NewTemplate` and `SetNewPrevHash` messages.

## Features

- **Same Interface** as `bitcoin-core-sv2` for seamless integration
- **Wraps Node's IPC Client** - leverages existing Braidpool infrastructure
- **Full SV2 Support** - `NewTemplate`, `SetNewPrevHash`, `SubmitSolution`, `RequestTransactionData`
- **Chain Tip Detection** - automatically detects new blocks
- **Template Caching** - maintains template history for solution verification

## Usage

```rust
use braidpool_template_provider::{BraidpoolTemplateProvider, BraidpoolTemplateProviderConfig, CancellationToken};

let config = BraidpoolTemplateProviderConfig {
    unix_socket_path: "/path/to/bitcoin.sock".into(),
    fee_threshold: 1000,
    min_interval: 30,
    incoming_tdp_receiver,
    outgoing_tdp_sender,
    cancellation_token: CancellationToken::new(),
    network_type: "mainnet".to_string(),
};

let mut provider = BraidpoolTemplateProvider::new(config).await?;
provider.run().await;
```

## Note

This provider must be run within a `tokio::task::LocalSet` because the underlying node IPC client uses capnp-rpc which is not `Send`.

## License

MIT OR Apache-2.0
