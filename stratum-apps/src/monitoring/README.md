# Monitoring Module

HTTP JSON API and Prometheus metrics for SV2 applications.

## API Endpoints

All channel-related endpoints support pagination via `?offset=N&limit=M` query params.

| Endpoint | Description |
|----------|-------------|
| `/swagger-ui` | Swagger UI (interactive API docs) |
| `/api-docs/openapi.json` | OpenAPI specification |
| `/api/v1/health` | Health check |
| `/api/v1/global` | Global statistics |
| `/api/v1/server` | Server with channels (paginated) |
| `/api/v1/clients` | All Sv2 clients with channels (paginated) |
| `/api/v1/clients/{id}` | Single Sv2 client with channels (paginated) |
| `/api/v1/sv1/clients` | Sv1 clients (Translator Proxy only, paginated) |
| `/api/v1/sv1/clients/{id}` | Single Sv1 client by ID (Translator Proxy only) |
| `/metrics` | Prometheus metrics |

## Traits

Applications implement these traits on their data structures:

- `ServerMonitoring` - For upstream connection info
- `ClientsMonitoring` - For downstream client info  
- `Sv1ClientsMonitoring` - For Sv1 clients (Translator Proxy only)

## Usage

```rust
use stratum_apps::monitoring::MonitoringServer;

let server = MonitoringServer::new(
    "0.0.0.0:9090".parse()?,
    Some(Arc::new(channel_manager.clone())), // server monitoring
    Some(Arc::new(channel_manager.clone())), // clients monitoring
)?;

// For Translator, add SV1 monitoring
let server = server.with_sv1_monitoring(Arc::new(sv1_server.clone()))?;

// Run with shutdown signal
server.run(shutdown_signal).await?;
```

## Prometheus Metrics

**System:**
- `sv2_uptime_seconds` - Server uptime

**Server:**
- `sv2_server_channels_total` - Total server channels
- `sv2_server_channels_extended` - Extended server channels
- `sv2_server_channels_standard` - Standard server channels
- `sv2_server_hashrate_total` - Total server hashrate
- `sv2_server_channel_hashrate{channel_id, user_identity}` - Per-channel hashrate
- `sv2_server_shares_accepted_total{channel_id, user_identity}` - Per-channel shares

**Clients:**
- `sv2_clients_total` - Connected client count
- `sv2_client_channels_total` - Total client channels
- `sv2_client_channels_extended` - Extended client channels
- `sv2_client_channels_standard` - Standard client channels
- `sv2_client_hashrate_total` - Total client hashrate
- `sv2_client_channel_hashrate{client_id, channel_id, user_identity}` - Per-channel hashrate
- `sv2_client_shares_accepted_total{client_id, channel_id, user_identity}` - Per-channel shares
- `sv2_client_channel_shares_per_minute{client_id, channel_id, user_identity}` - Per-channel share rate

**Sv1 (Translator Proxy only):**
- `sv1_clients_total` - Sv1 client count
- `sv1_hashrate_total` - Sv1 total hashrate
