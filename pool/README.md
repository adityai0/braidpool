
# SRI Pool

SRI Pool is designed to communicate with Downstream role (most typically a Translator Proxy or a Mining Proxy) running SV2 protocol to exploit features introduced by its sub-protocols.

The most typical high level configuration is:

```
<--- Most Downstream ----------------------------------------- Most Upstream --->

+---------------------------------------------------+  +------------------------+
|                     Mining Farm                   |  |      Remote Pool       |
|                                                   |  |                        |
|  +-------------------+     +------------------+   |  |   +-----------------+  |
|  | SV1 Mining Device | <-> | Translator Proxy | <------> | SV2 Pool Server |  |
|  +-------------------+     +------------------+   |  |   +-----------------+  |
|                                                   |  |                        |
+---------------------------------------------------+  +------------------------+

```

It can receive templates from two potential sources:
- Sv2 Template Provider: a separate Sv2 application running either locally or on a different machine, for which a (optionally encrypted) TCP connection will be established
- Bitcoin Core v30+: an officially released Bitcoin Core node running locally, on the same machine, for which a UNIX socket connection will be established

## Setup

### Configuration File

There are three example configuration files demonstrating different template provider setups:

1. `pool-config-hosted-sv2-tp-example.toml` - Connects to a community hosted Sv2 Template Provider server
2. `pool-config-local-sv2-tp-example.toml` - Connects to a locally running Sv2 Template Provider
3. `pool-config-bitcoin-core-ipc-example.toml` - Connects directly to a Bitcoin Core node via IPC

The configuration file contains the following information:

1. The SRI Pool information which includes the SRI Pool authority public key
   (`authority_public_key`), the SRI Pool authority secret key (`authority_secret_key`).
2. The address which it will use to listen to new connection from downstream roles (`listen_address`)
3. The coinbase reward script specified as a descriptor (`coinbase_reward_script`)
4. A string that serves as signature on the coinbase tx (`pool_signature`).
5. The `template_provider_type` section, which determines how the pool obtains block templates. There are two options:
   - `[template_provider_type.Sv2Tp]` - Connects to an SV2 Template Provider, with the following parameters:
     - `address` - The Template Provider's network address
     - `public_key` - (Optional) The TP's authority public key for connection verification
   - `[template_provider_type.BitcoinCoreIpc]` - Connects directly to Bitcoin Core via IPC, with the following parameters:
     - `unix_socket_path` - Path to the Bitcoin Core IPC UNIX socket
     - `fee_threshold` - Minimum fee threshold to trigger new templates

For connections with a Sv2 Template Provider, you may want to verify that your TP connection is authentic. You can get the `public_key` from the logs of your TP, for example:

```
# 2024-02-13T14:59:24Z Template Provider authority key: EguTM8URcZDQVeEBsM4B5vg9weqEUnufA8pm85fG4bZd
```

### Run

There are three example configuration files found in `pool-apps/pool/config-examples`:

1. `pool-config-hosted-sv2-tp-example.toml` - Connects to our community hosted Sv2 Template Provider server
2. `pool-config-local-sv2-tp-example.toml` - Runs with your local Sv2 Template Provider
3. `pool-config-bitcoin-core-ipc-example.toml` - Runs with direct Bitcoin Core IPC connection

Run the Pool (example using hosted Sv2 TP):

```bash
cd pool-apps/pool
cargo run -- -c config-examples/pool-config-hosted-sv2-tp-example.toml
``` 
